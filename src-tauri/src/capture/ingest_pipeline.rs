use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::{
        sidecar_client::{CompletedCaptureFastPreview, FastPreviewReadyUpdate},
        CAPTURE_PIPELINE_LOCK, IN_FLIGHT_CAPTURE_SESSIONS,
    },
    contracts::dto::{CaptureRequestInputDto, HostErrorEnvelope},
    render::{
        dedicated_renderer::{
            preview_promotion_snapshot_paths_in_dir,
            resolve_preview_renderer_route_snapshot_in_dir,
        },
        enqueue_resident_preview_render_in_dir, is_valid_render_preview_asset,
        log_render_failure_in_dir, log_render_ready_in_dir, log_render_start_in_dir,
        promote_preview_render_output, render_capture_asset_in_dir,
        render_preview_asset_to_path_in_dir, RenderIntent,
    },
    session::{
        session_manifest::{
            current_timestamp, ActivePresetBinding, CaptureTimingMetrics, FinalCaptureAsset,
            PreviewCaptureAsset, PreviewRendererRouteSnapshot,
            PreviewRendererWarmStateSnapshot, RawCaptureAsset, SessionCaptureRecord,
            SessionManifest, CAPTURE_BUDGET_MS, PREVIEW_BUDGET_MS,
            SESSION_CAPTURE_SCHEMA_VERSION,
        },
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
    },
    timing::{
        append_session_timing_event_in_dir, sync_session_timing_in_dir, SessionTimingEventInput,
    },
};

const FAST_PREVIEW_ALLOWED_EXTENSIONS: [&str; 2] = ["jpg", "jpeg"];
// Latest booth evidence shows the helper's usable same-capture preview often
// lands ~0.6-0.7s after RAW persistence. Keep the wait under 1s so fallback
// still stays bounded, but leave enough budget to actually catch that path.
const HELPER_FAST_PREVIEW_WAIT_MS: u64 = 900;
const HELPER_FAST_PREVIEW_POLL_MS: u64 = 40;
// Speculative render is only worth adopting when it is already about to finish.
// Give the lighter raster lane enough room to close before we spend another
// darktable process on the same capture.
const SPECULATIVE_PREVIEW_WAIT_MS: u64 = 1800;
const SPECULATIVE_PREVIEW_POLL_MS: u64 = 40;
// Once the same-capture image is already visible, the booth should fall through
// to the fallback close quickly instead of draining indefinitely.
const SPECULATIVE_PREVIEW_DRAIN_WAIT_MS: u64 = 400;
const SPECULATIVE_PREVIEW_DRAIN_POLL_MS: u64 = 80;
// If a speculative close is still actively rendering after the initial wait,
// prefer joining that in-flight work over starting a second preview render that
// competes for the same preview worker/runtime. Recent booth logs show the
// slower same-capture raster close can still arrive just under 6s after the
// first thumbnail becomes visible, so keep the join window wide enough to let
// that correct close win before we fall back to a competing render.
const SPECULATIVE_PREVIEW_JOIN_WAIT_MS: u64 = 5000;
const SPECULATIVE_PREVIEW_JOIN_POLL_MS: u64 = 80;
const PREVIEW_REFINEMENT_IDLE_WAIT_MS: u64 = 5000;
const PREVIEW_REFINEMENT_IDLE_POLL_MS: u64 = 80;

struct FastPreviewPromotionResult {
    asset_path: String,
    visible_at_ms: Option<u64>,
}

struct PreparedSpeculativePreviewSource {
    asset_path: PathBuf,
    cleanup_path: Option<PathBuf>,
}

pub fn persist_capture_in_dir(
    base_dir: &Path,
    input: &CaptureRequestInputDto,
    capture_id: String,
    request_id: String,
    raw_asset_path: String,
    fast_preview: Option<CompletedCaptureFastPreview>,
    pre_promoted_fast_preview: Option<&FastPreviewReadyUpdate>,
    acknowledged_at_ms: u64,
    persisted_at_ms: u64,
) -> Result<
    (
        SessionManifest,
        SessionCaptureRecord,
        Option<FastPreviewReadyUpdate>,
    ),
    HostErrorEnvelope,
> {
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    manifest =
        sync_session_timing_in_dir(base_dir, &paths.manifest_path, manifest, SystemTime::now())?;
    let active_preset = manifest.active_preset.clone().ok_or_else(|| {
        HostErrorEnvelope::preset_not_available("촬영 전에 룩을 다시 골라 주세요.")
    })?;

    let mut capture = build_saved_capture_record(
        &manifest,
        &active_preset,
        capture_id,
        request_id,
        raw_asset_path,
        acknowledged_at_ms,
        persisted_at_ms,
    );
    let promoted_fast_preview = fast_preview.as_ref().and_then(|handoff| {
        promote_fast_preview_asset(
            &paths,
            &capture.capture_id,
            &capture.request_id,
            Some(capture.raw.asset_path.as_str()),
            handoff,
        )
    });

    if let Some(ref promoted_fast_preview) = promoted_fast_preview {
        capture.preview.asset_path = Some(promoted_fast_preview.asset_path.clone());
        capture.timing.fast_preview_visible_at_ms = promoted_fast_preview.visible_at_ms;
    } else if let Some(pre_promoted_fast_preview) = pre_promoted_fast_preview {
        capture.preview.asset_path = Some(pre_promoted_fast_preview.asset_path.clone());
        capture.timing.fast_preview_visible_at_ms = Some(pre_promoted_fast_preview.visible_at_ms);
    } else if let Some(seed_result) =
        seed_pending_preview_asset_path(&paths, &capture.capture_id, &capture.request_id)
    {
        // If helper handoff metadata is missing or invalid but the same-capture
        // preview file already exists on disk, keep the fast-path alive.
        capture.preview.asset_path = Some(seed_result.asset_path);
        capture.timing.fast_preview_visible_at_ms = seed_result.visible_at_ms;
    }

    if let Err(error) = write_capture_time_preview_promotion_snapshots_in_dir(
        base_dir,
        &capture.session_id,
        &capture.capture_id,
    ) {
        log::warn!(
            "capture-time preview snapshot failed session={} capture_id={} code={} message={}",
            capture.session_id,
            capture.capture_id,
            error.code,
            error.message
        );
    }

    let fast_preview_update =
        promoted_fast_preview
            .as_ref()
            .map(|promoted| FastPreviewReadyUpdate {
                request_id: capture.request_id.clone(),
                capture_id: capture.capture_id.clone(),
                asset_path: promoted.asset_path.clone(),
                kind: fast_preview
                    .as_ref()
                    .and_then(|preview| preview.kind.clone()),
                visible_at_ms: promoted.visible_at_ms.unwrap_or(persisted_at_ms),
            });

    manifest.captures.push(capture.clone());
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = "preview-waiting".into();

    write_session_manifest(&paths.manifest_path, &manifest)?;

    if let Some(first_visible_asset_path) = capture.preview.asset_path.as_deref() {
        let route_snapshot = resolve_preview_renderer_route_snapshot_in_dir(
            base_dir,
            &active_preset.preset_id,
            &active_preset.published_version,
        );
        if should_skip_speculative_preview_render_for_route(
            &route_snapshot,
            manifest.active_preview_renderer_warm_state.as_ref(),
            &active_preset,
        ) {
            let warm_state = manifest
                .active_preview_renderer_warm_state
                .as_ref()
                .map(|snapshot| snapshot.state.as_str())
                .unwrap_or("none");
            let detail = format!(
                "reason=dedicated-renderer-warm-route;route={};routeStage={};warmState={}",
                route_snapshot.route, route_snapshot.route_stage, warm_state
            );
            let _ = append_session_timing_event_in_dir(
                base_dir,
                SessionTimingEventInput {
                    session_id: &manifest.session_id,
                    event: "speculative-preview-skipped",
                    capture_id: Some(&capture.capture_id),
                    request_id: Some(&capture.request_id),
                    detail: Some(&detail),
                },
            );
        } else {
            start_speculative_preview_render_in_dir(
                base_dir,
                &manifest.session_id,
                &capture.request_id,
                &capture.capture_id,
                &active_preset.preset_id,
                &active_preset.published_version,
                first_visible_asset_path,
            );
        }
    }

    Ok((manifest, capture, fast_preview_update))
}

pub fn promote_pending_fast_preview_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    fast_preview_asset_path: &str,
    kind: Option<&str>,
) -> Option<FastPreviewReadyUpdate> {
    let paths = SessionPaths::try_new(base_dir, session_id).ok()?;
    let promoted = promote_fast_preview_asset(
        &paths,
        capture_id,
        request_id,
        None,
        &CompletedCaptureFastPreview {
            asset_path: fast_preview_asset_path.to_string(),
            kind: kind.map(str::to_string),
        },
    )?;

    Some(FastPreviewReadyUpdate {
        request_id: request_id.to_string(),
        capture_id: capture_id.to_string(),
        asset_path: promoted.asset_path,
        kind: kind.map(str::to_string),
        visible_at_ms: promoted.visible_at_ms.or_else(|| current_time_ms().ok())?,
    })
}

pub fn start_speculative_preview_render_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    fast_preview_asset_path: &str,
) {
    let paths = match SessionPaths::try_new(base_dir, session_id) {
        Ok(paths) => paths,
        Err(_) => return,
    };
    let source_path = match fs::canonicalize(fast_preview_asset_path) {
        Ok(path) => path,
        Err(_) => return,
    };

    if !is_session_scoped_asset_path(&paths, &source_path)
        || !is_valid_render_preview_asset(&source_path)
    {
        return;
    }

    let prepared_source =
        match prepare_speculative_preview_source_path(&paths, capture_id, request_id, &source_path)
        {
            Some(value) => value,
            None => return,
        };

    let speculative_output_path = speculative_preview_output_path(&paths, capture_id);
    let speculative_lock_path = speculative_preview_lock_path(&paths, capture_id, request_id);
    let speculative_detail_path = speculative_preview_detail_path(&paths, capture_id, request_id);

    if is_valid_render_preview_asset(&speculative_output_path) || speculative_lock_path.exists() {
        return;
    }

    if let Some(parent) = speculative_output_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    if fs::write(&speculative_lock_path, request_id).is_err() {
        return;
    }

    if let Err(error) = enqueue_resident_preview_render_in_dir(
        base_dir,
        session_id,
        request_id,
        capture_id,
        preset_id,
        preset_version,
        &prepared_source.asset_path,
        prepared_source.cleanup_path.as_deref(),
        &speculative_output_path,
        &speculative_detail_path,
        &speculative_lock_path,
    ) {
        log::warn!(
            "resident_first_visible_render_enqueue_failed session={} capture_id={} request_id={} reason_code={} detail={}",
            session_id,
            capture_id,
            request_id,
            error.reason_code,
            error.operator_detail
        );
        log_render_failure_in_dir(
            base_dir,
            session_id,
            capture_id,
            Some(request_id),
            RenderIntent::Preview,
            error.reason_code,
        );
        spawn_one_shot_speculative_preview_render_in_dir(
            base_dir.to_path_buf(),
            session_id.to_string(),
            request_id.to_string(),
            capture_id.to_string(),
            preset_id.to_string(),
            preset_version.to_string(),
            prepared_source.asset_path,
            prepared_source.cleanup_path,
            speculative_output_path,
            speculative_detail_path,
            speculative_lock_path,
        );
    }
}

fn spawn_one_shot_speculative_preview_render_in_dir(
    base_dir: PathBuf,
    session_id: String,
    request_id: String,
    capture_id: String,
    preset_id: String,
    preset_version: String,
    source_path: PathBuf,
    source_cleanup_path: Option<PathBuf>,
    speculative_output_path: PathBuf,
    speculative_detail_path: PathBuf,
    speculative_lock_path: PathBuf,
) {
    thread::spawn(move || {
        wait_for_capture_pipeline_idle(&base_dir);

        log_render_start_in_dir(
            &base_dir,
            &session_id,
            &capture_id,
            &request_id,
            RenderIntent::Preview,
        );

        let render_result = render_preview_asset_to_path_in_dir(
            &base_dir,
            &session_id,
            &request_id,
            &capture_id,
            &preset_id,
            &preset_version,
            &source_path,
            &speculative_output_path,
        );

        match render_result {
            Ok(prepared_render) => {
                let _ = fs::write(&speculative_detail_path, prepared_render.detail);
            }
            Err(error) => {
                log::warn!(
                    "speculative_preview_render_failed session={} capture_id={} request_id={} reason_code={} detail={}",
                    session_id,
                    capture_id,
                    request_id,
                    error.reason_code,
                    error.operator_detail
                );
                log_render_failure_in_dir(
                    &base_dir,
                    &session_id,
                    &capture_id,
                    Some(&request_id),
                    RenderIntent::Preview,
                    error.reason_code,
                );
                let _ = fs::remove_file(&speculative_output_path);
                let _ = fs::remove_file(&speculative_detail_path);
            }
        }

        if let Some(source_cleanup_path) = source_cleanup_path.as_ref() {
            let _ = fs::remove_file(source_cleanup_path);
        }
        let _ = fs::remove_file(&speculative_lock_path);
    });
}

pub fn complete_preview_render_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    wait_for_speculative_preview_completion_in_dir(base_dir, &paths, capture_id)?;

    let mut capture_snapshot = loop {
        let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
            HostErrorEnvelope::persistence(
                "프리뷰 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.",
            )
        })?;
        let mut manifest = read_session_manifest(&paths.manifest_path)?;
        let capture_index = manifest
            .captures
            .iter()
            .position(|capture| capture.capture_id == capture_id)
            .ok_or_else(|| {
                HostErrorEnvelope::session_not_found("방금 저장된 촬영 기록을 찾지 못했어요.")
            })?;

        if manifest.captures[capture_index]
            .preview
            .asset_path
            .is_some()
            && matches!(
                manifest.captures[capture_index].render_status.as_str(),
                "previewReady" | "finalReady"
            )
        {
            return Ok(manifest.captures[capture_index].clone());
        }

        if let Some(speculative_capture) = try_complete_speculative_preview_render_in_dir(
            base_dir,
            &paths,
            &mut manifest,
            capture_index,
        )? {
            return Ok(speculative_capture);
        }

        let promoted_late_fast_preview =
            sync_helper_fast_preview_before_render(&paths, &mut manifest, capture_index)?;
        if promoted_late_fast_preview {
            let capture = manifest
                .captures
                .get(capture_index)
                .cloned()
                .expect("capture index already resolved");
            if let (Some(preset_id), Some(first_visible_asset_path)) = (
                capture.active_preset_id.as_deref(),
                capture.preview.asset_path.as_deref(),
            ) {
                start_speculative_preview_render_in_dir(
                    base_dir,
                    session_id,
                    &capture.request_id,
                    capture_id,
                    preset_id,
                    &capture.active_preset_version,
                    first_visible_asset_path,
                );
                let request_id = capture.request_id.clone();
                drop(_pipeline_guard);
                wait_for_speculative_preview_completion_for_request_in_dir(
                    base_dir,
                    &paths,
                    capture_id,
                    &request_id,
                );
                continue;
            }
            break capture;
        }

        break manifest.captures[capture_index].clone();
    };

    wait_for_active_speculative_preview_close_before_direct_render_in_dir(
        &paths,
        capture_id,
        &capture_snapshot.request_id,
    );

    {
        let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
            HostErrorEnvelope::persistence(
                "프리뷰 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.",
            )
        })?;
        let mut manifest = read_session_manifest(&paths.manifest_path)?;
        let capture_index = manifest
            .captures
            .iter()
            .position(|capture| capture.capture_id == capture_id)
            .ok_or_else(|| {
                HostErrorEnvelope::session_not_found("방금 저장된 촬영 기록을 찾지 못했어요.")
            })?;

        if manifest.captures[capture_index]
            .preview
            .asset_path
            .is_some()
            && matches!(
                manifest.captures[capture_index].render_status.as_str(),
                "previewReady" | "finalReady"
            )
        {
            return Ok(manifest.captures[capture_index].clone());
        }

        if let Some(speculative_capture) = try_complete_speculative_preview_render_in_dir(
            base_dir,
            &paths,
            &mut manifest,
            capture_index,
        )? {
            return Ok(speculative_capture);
        }

        capture_snapshot = manifest.captures[capture_index].clone();
    }

    log_render_start_in_dir(
        base_dir,
        session_id,
        capture_id,
        &capture_snapshot.request_id,
        RenderIntent::Preview,
    );
    let rendered_preview = match render_capture_asset_in_dir(
        base_dir,
        session_id,
        &capture_snapshot,
        RenderIntent::Preview,
    ) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "capture_preview_render_failed session={} capture_id={} reason_code={} detail={}",
                session_id,
                capture_id,
                error.reason_code,
                error.operator_detail
            );
            log_render_failure_in_dir(
                base_dir,
                session_id,
                capture_id,
                Some(&capture_snapshot.request_id),
                RenderIntent::Preview,
                error.reason_code,
            );
            return Err(HostErrorEnvelope::persistence(error.customer_message));
        }
    };

    finish_preview_render_in_dir(base_dir, &paths, session_id, capture_id, rendered_preview)
}

fn wait_for_speculative_preview_completion_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    capture_id: &str,
) -> Result<(), HostErrorEnvelope> {
    let manifest = read_session_manifest(&paths.manifest_path)?;
    let Some(capture) = manifest
        .captures
        .iter()
        .find(|capture| capture.capture_id == capture_id)
    else {
        return Ok(());
    };

    wait_for_speculative_preview_completion_for_request_in_dir(
        base_dir,
        paths,
        capture_id,
        &capture.request_id,
    );
    Ok(())
}

fn wait_for_speculative_preview_completion_for_request_in_dir(
    _base_dir: &Path,
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
) {
    let speculative_output_path = speculative_preview_output_path(paths, capture_id);
    let speculative_lock_path = speculative_preview_lock_path(paths, capture_id, request_id);

    if !speculative_lock_path.exists() {
        return;
    }

    let wait_cycles = (SPECULATIVE_PREVIEW_WAIT_MS / SPECULATIVE_PREVIEW_POLL_MS).max(1);
    for attempt in 0..=wait_cycles {
        if is_valid_render_preview_asset(&speculative_output_path)
            || !speculative_lock_path.exists()
        {
            return;
        }

        if attempt < wait_cycles {
            thread::sleep(Duration::from_millis(SPECULATIVE_PREVIEW_POLL_MS));
        }
    }

    let drain_cycles =
        (SPECULATIVE_PREVIEW_DRAIN_WAIT_MS / SPECULATIVE_PREVIEW_DRAIN_POLL_MS).max(1);
    for attempt in 0..=drain_cycles {
        if is_valid_render_preview_asset(&speculative_output_path)
            || !speculative_lock_path.exists()
        {
            return;
        }

        if attempt < drain_cycles {
            thread::sleep(Duration::from_millis(SPECULATIVE_PREVIEW_DRAIN_POLL_MS));
        }
    }

    if speculative_lock_path.exists() {
        log::info!(
            "speculative_preview_wait_budget_exhausted session={} capture_id={} request_id={} wait_ms={}",
            paths
                .session_root
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default(),
            capture_id,
            request_id,
            SPECULATIVE_PREVIEW_WAIT_MS + SPECULATIVE_PREVIEW_DRAIN_WAIT_MS
        );
    }
}

fn wait_for_active_speculative_preview_close_before_direct_render_in_dir(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
) {
    let speculative_output_path = speculative_preview_output_path(paths, capture_id);
    let speculative_lock_path = speculative_preview_lock_path(paths, capture_id, request_id);

    if !speculative_lock_path.exists() {
        return;
    }

    let wait_cycles = (SPECULATIVE_PREVIEW_JOIN_WAIT_MS / SPECULATIVE_PREVIEW_JOIN_POLL_MS).max(1);
    for attempt in 0..=wait_cycles {
        if is_valid_render_preview_asset(&speculative_output_path)
            || !speculative_lock_path.exists()
        {
            return;
        }

        if attempt < wait_cycles {
            thread::sleep(Duration::from_millis(SPECULATIVE_PREVIEW_JOIN_POLL_MS));
        }
    }

    if speculative_lock_path.exists() {
        log::info!(
            "speculative_preview_join_wait_exhausted session={} capture_id={} request_id={} wait_ms={}",
            paths
                .session_root
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default(),
            capture_id,
            request_id,
            SPECULATIVE_PREVIEW_JOIN_WAIT_MS
        );
    }
}

fn try_complete_speculative_preview_render_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    manifest: &mut SessionManifest,
    capture_index: usize,
) -> Result<Option<SessionCaptureRecord>, HostErrorEnvelope> {
    let Some(capture_snapshot) = manifest.captures.get(capture_index).cloned() else {
        return Ok(None);
    };

    let speculative_output_path =
        speculative_preview_output_path(paths, &capture_snapshot.capture_id);
    let speculative_lock_path = speculative_preview_lock_path(
        paths,
        &capture_snapshot.capture_id,
        &capture_snapshot.request_id,
    );

    if !speculative_lock_path.exists() && !is_valid_render_preview_asset(&speculative_output_path) {
        return Ok(None);
    }

    if is_valid_render_preview_asset(&speculative_output_path) {
        let canonical_preview_path = paths
            .renders_previews_dir
            .join(format!("{}.jpg", capture_snapshot.capture_id));
        if let Err(error) =
            promote_preview_render_output(&speculative_output_path, &canonical_preview_path)
        {
            log::warn!(
                "speculative_preview_promote_failed session={} capture_id={} request_id={} reason_code={} detail={}",
                capture_snapshot.session_id,
                capture_snapshot.capture_id,
                capture_snapshot.request_id,
                error.reason_code,
                error.operator_detail
            );
            let _ = fs::remove_file(&speculative_output_path);
            let _ = fs::remove_file(speculative_preview_detail_path(
                paths,
                &capture_snapshot.capture_id,
                &capture_snapshot.request_id,
            ));
            return Ok(None);
        }

        let preview_visible_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                HostErrorEnvelope::persistence(
                    "프리뷰 시각을 기록하지 못했어요. 잠시 후 다시 시도해 주세요.",
                )
            })?
            .as_millis() as u64;
        let render_detail = fs::read_to_string(speculative_preview_detail_path(
            paths,
            &capture_snapshot.capture_id,
            &capture_snapshot.request_id,
        ))
        .unwrap_or_else(|_| {
            "presetId=unknown;publishedVersion=unknown;binary=darktable-cli;source=unknown;elapsedMs=unknown;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=unknown;status=unknown"
                .into()
        });
        log_render_ready_in_dir(
            base_dir,
            &capture_snapshot.session_id,
            &capture_snapshot.capture_id,
            &capture_snapshot.request_id,
            RenderIntent::Preview,
            &render_detail,
        );

        let capture = {
            let capture = manifest
                .captures
                .get_mut(capture_index)
                .expect("capture index already resolved");

            let budget_state = if preview_visible_at_ms
                .saturating_sub(capture.timing.capture_acknowledged_at_ms)
                <= PREVIEW_BUDGET_MS
            {
                "withinBudget"
            } else {
                "exceededBudget"
            };

            capture.preview.asset_path =
                Some(canonical_preview_path.to_string_lossy().into_owned());
            capture.preview.ready_at_ms = Some(preview_visible_at_ms);
            capture.render_status = "previewReady".into();
            capture.timing.preview_visible_at_ms = Some(preview_visible_at_ms);
            capture.timing.xmp_preview_ready_at_ms = Some(preview_visible_at_ms);
            capture.timing.fast_preview_visible_at_ms = Some(
                capture
                    .timing
                    .fast_preview_visible_at_ms
                    .unwrap_or(preview_visible_at_ms),
            );
            capture.timing.preview_budget_state = budget_state.into();

            capture.clone()
        };

        manifest.updated_at = current_timestamp(SystemTime::now())?;
        manifest.lifecycle.stage = derive_capture_lifecycle_stage(manifest);
        write_session_manifest(&paths.manifest_path, manifest)?;
        append_capture_preview_ready_event(
            base_dir,
            &capture_snapshot.session_id,
            &capture,
            preview_visible_at_ms,
        );

        let _ = fs::remove_file(speculative_preview_detail_path(
            paths,
            &capture_snapshot.capture_id,
            &capture_snapshot.request_id,
        ));

        return Ok(Some(capture));
    }

    Ok(None)
}

fn wait_for_capture_pipeline_idle(base_dir: &Path) {
    let wait_cycles = (PREVIEW_REFINEMENT_IDLE_WAIT_MS / PREVIEW_REFINEMENT_IDLE_POLL_MS).max(1);

    for _ in 0..=wait_cycles {
        if !has_in_flight_capture_for_runtime(base_dir) {
            return;
        }

        thread::sleep(Duration::from_millis(PREVIEW_REFINEMENT_IDLE_POLL_MS));
    }
}

fn has_in_flight_capture_for_runtime(base_dir: &Path) -> bool {
    let runtime_key = base_dir.to_string_lossy().into_owned();

    IN_FLIGHT_CAPTURE_SESSIONS
        .lock()
        .map(|sessions| sessions.contains_key(&runtime_key))
        .unwrap_or(false)
}

pub(crate) fn finish_preview_render_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    session_id: &str,
    capture_id: &str,
    rendered_preview: crate::render::RenderedCaptureAsset,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("프리뷰 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let capture_index = manifest
        .captures
        .iter()
        .position(|capture| capture.capture_id == capture_id)
        .ok_or_else(|| {
            HostErrorEnvelope::session_not_found("방금 저장된 촬영 기록을 찾지 못했어요.")
        })?;
    let preview_ready_at_ms = rendered_preview.ready_at_ms;
    let capture = {
        let capture = manifest
            .captures
            .get_mut(capture_index)
            .expect("capture index already resolved");
        let first_truth_close = capture.timing.xmp_preview_ready_at_ms.is_none();

        capture.preview.asset_path = Some(rendered_preview.asset_path);
        capture.preview.ready_at_ms = Some(preview_ready_at_ms);
        if capture.render_status != "finalReady" {
            capture.render_status = "previewReady".into();
        }
        if first_truth_close {
            capture.timing.preview_visible_at_ms = Some(preview_ready_at_ms);
            capture.timing.xmp_preview_ready_at_ms = Some(preview_ready_at_ms);
            capture.timing.preview_budget_state = if preview_ready_at_ms
                .saturating_sub(capture.timing.capture_acknowledged_at_ms)
                <= PREVIEW_BUDGET_MS
            {
                "withinBudget".into()
            } else {
                "exceededBudget".into()
            };
        } else {
            capture.timing.preview_visible_at_ms = capture
                .timing
                .preview_visible_at_ms
                .or(capture.preview.ready_at_ms);
            capture.timing.xmp_preview_ready_at_ms = capture
                .timing
                .xmp_preview_ready_at_ms
                .or(capture.preview.ready_at_ms);
        }

        (capture.clone(), first_truth_close)
    };
    let (capture, first_truth_close) = capture;

    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(&manifest);
    write_session_manifest(&paths.manifest_path, &manifest)?;
    if first_truth_close {
        append_capture_preview_ready_event(base_dir, session_id, &capture, preview_ready_at_ms);
    }

    Ok(capture)
}

fn sync_helper_fast_preview_before_render(
    paths: &SessionPaths,
    manifest: &mut SessionManifest,
    capture_index: usize,
) -> Result<bool, HostErrorEnvelope> {
    let Some(capture) = manifest.captures.get_mut(capture_index) else {
        return Ok(false);
    };

    if capture.preview.asset_path.is_some() {
        return Ok(false);
    }

    let wait_cycles = (HELPER_FAST_PREVIEW_WAIT_MS / HELPER_FAST_PREVIEW_POLL_MS).max(1);
    for _ in 0..=wait_cycles {
        if let Some(promoted_fast_preview) =
            seed_pending_preview_asset_path(paths, &capture.capture_id, &capture.request_id)
        {
            capture.preview.asset_path = Some(promoted_fast_preview.asset_path);
            if capture.timing.fast_preview_visible_at_ms.is_none() {
                capture.timing.fast_preview_visible_at_ms = promoted_fast_preview.visible_at_ms;
            }
            manifest.updated_at = current_timestamp(SystemTime::now())?;
            write_session_manifest(&paths.manifest_path, manifest)?;
            return Ok(true);
        }

        thread::sleep(Duration::from_millis(HELPER_FAST_PREVIEW_POLL_MS));
    }

    log::info!(
        "helper_fast_preview_wait_budget_exhausted session={} capture_id={} request_id={} wait_ms={}",
        capture.session_id,
        capture.capture_id,
        capture.request_id,
        HELPER_FAST_PREVIEW_WAIT_MS
    );

    Ok(false)
}

pub fn complete_final_render_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence(
            "최종 결과 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.",
        )
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let capture_index = manifest
        .captures
        .iter()
        .position(|capture| capture.capture_id == capture_id)
        .ok_or_else(|| {
            HostErrorEnvelope::session_not_found("방금 저장된 촬영 기록을 찾지 못했어요.")
        })?;

    if manifest.captures[capture_index]
        .final_asset
        .asset_path
        .is_some()
        && manifest.captures[capture_index].render_status == "finalReady"
    {
        return Ok(manifest.captures[capture_index].clone());
    }

    if manifest.captures[capture_index]
        .preview
        .asset_path
        .is_none()
    {
        return Err(HostErrorEnvelope::persistence(
            "최종 결과를 만들기 전에 확인용 사진이 먼저 준비되어야 해요.",
        ));
    }

    let capture_snapshot = manifest.captures[capture_index].clone();
    log_render_start_in_dir(
        base_dir,
        session_id,
        capture_id,
        &capture_snapshot.request_id,
        RenderIntent::Final,
    );
    let rendered_final = match render_capture_asset_in_dir(
        base_dir,
        session_id,
        &capture_snapshot,
        RenderIntent::Final,
    ) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "capture_final_render_failed session={} capture_id={} reason_code={} detail={}",
                session_id,
                capture_id,
                error.reason_code,
                error.operator_detail
            );
            log_render_failure_in_dir(
                base_dir,
                session_id,
                capture_id,
                Some(&capture_snapshot.request_id),
                RenderIntent::Final,
                error.reason_code,
            );
            return Err(HostErrorEnvelope::persistence(error.customer_message));
        }
    };

    let capture = {
        let capture = manifest
            .captures
            .get_mut(capture_index)
            .expect("capture index already resolved");

        capture.final_asset.asset_path = Some(rendered_final.asset_path);
        capture.final_asset.ready_at_ms = Some(rendered_final.ready_at_ms);
        capture.render_status = "finalReady".into();
        capture.post_end_state = "handoffReady".into();

        capture.clone()
    };

    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = "capture-ready".into();
    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok(capture)
}

pub fn mark_preview_render_failed_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    mark_render_failed_in_dir(base_dir, session_id, capture_id, RenderIntent::Preview)
}

pub fn mark_final_render_failed_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    mark_render_failed_in_dir(base_dir, session_id, capture_id, RenderIntent::Final)
}

fn mark_render_failed_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    intent: RenderIntent,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("렌더 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let Some(capture_index) = manifest
        .captures
        .iter()
        .position(|capture| capture.capture_id == capture_id)
    else {
        return Ok(manifest);
    };

    let Some(latest_capture) = manifest.captures.last() else {
        return Ok(manifest);
    };

    if latest_capture.capture_id != capture_id {
        return Ok(manifest);
    }

    if matches!(intent, RenderIntent::Preview)
        && manifest.captures[capture_index]
            .preview
            .asset_path
            .is_some()
    {
        return Ok(manifest);
    }

    manifest.captures[capture_index].render_status = "renderFailed".into();
    manifest.captures[capture_index].timing.preview_budget_state = "exceededBudget".into();
    manifest.captures[capture_index].post_end_state = "postEndPending".into();
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = "phone-required".into();
    log_render_failure_in_dir(
        base_dir,
        session_id,
        capture_id,
        Some(&manifest.captures[capture_index].request_id),
        intent,
        match intent {
            RenderIntent::Preview => "preview-render-failed",
            RenderIntent::Final => "final-render-failed",
        },
    );

    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok(manifest)
}

fn build_saved_capture_record(
    manifest: &SessionManifest,
    active_preset: &ActivePresetBinding,
    capture_id: String,
    request_id: String,
    raw_asset_path: String,
    acknowledged_at_ms: u64,
    persisted_at_ms: u64,
) -> SessionCaptureRecord {
    SessionCaptureRecord {
        schema_version: SESSION_CAPTURE_SCHEMA_VERSION.into(),
        session_id: manifest.session_id.clone(),
        booth_alias: manifest.booth_alias.clone(),
        active_preset_id: Some(active_preset.preset_id.clone()),
        active_preset_version: active_preset.published_version.clone(),
        active_preset_display_name: manifest.active_preset_display_name.clone(),
        preview_renderer_route: manifest.active_preview_renderer_route.clone(),
        capture_id,
        request_id,
        raw: RawCaptureAsset {
            asset_path: raw_asset_path,
            persisted_at_ms,
        },
        preview: PreviewCaptureAsset {
            asset_path: None,
            enqueued_at_ms: Some(persisted_at_ms),
            ready_at_ms: None,
        },
        final_asset: FinalCaptureAsset {
            asset_path: None,
            ready_at_ms: None,
        },
        render_status: "previewWaiting".into(),
        post_end_state: "activeSession".into(),
        timing: CaptureTimingMetrics {
            capture_acknowledged_at_ms: acknowledged_at_ms,
            preview_visible_at_ms: None,
            fast_preview_visible_at_ms: None,
            xmp_preview_ready_at_ms: None,
            capture_budget_ms: CAPTURE_BUDGET_MS,
            preview_budget_ms: PREVIEW_BUDGET_MS,
            preview_budget_state: "pending".into(),
        },
    }
}

fn write_capture_time_preview_promotion_snapshots_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<(), HostErrorEnvelope> {
    let (route_policy_snapshot_path, catalog_state_snapshot_path) =
        preview_promotion_snapshot_paths_in_dir(base_dir, session_id, capture_id)?;
    copy_snapshot_artifact(
        &base_dir
            .join("branch-config")
            .join("preview-renderer-policy.json"),
        &route_policy_snapshot_path,
    )?;
    write_capture_time_catalog_snapshot_in_dir(base_dir, session_id, &catalog_state_snapshot_path)?;

    Ok(())
}

fn write_capture_time_catalog_snapshot_in_dir(
    base_dir: &Path,
    session_id: &str,
    destination: &Path,
) -> Result<(), HostErrorEnvelope> {
    let manifest_path = SessionPaths::try_new(base_dir, session_id)?.manifest_path;
    let manifest = read_session_manifest(&manifest_path)?;

    if let (Some(catalog_revision), Some(catalog_snapshot)) = (
        manifest.catalog_revision,
        manifest.catalog_snapshot.as_ref(),
    ) {
        let parent = destination.parent().ok_or_else(|| {
            HostErrorEnvelope::persistence("snapshot artifact 경로를 준비하지 못했어요.")
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "snapshot artifact 경로를 만들지 못했어요: {error}"
            ))
        })?;
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "preset-catalog-state/v1",
            "catalogRevision": catalog_revision,
            "updatedAt": manifest.updated_at,
            "livePresets": catalog_snapshot,
        }))
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "capture 시점 catalog snapshot을 직렬화하지 못했어요: {error}"
            ))
        })?;
        fs::write(destination, bytes).map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "snapshot artifact를 저장하지 못했어요: {error}"
            ))
        })?;
        return Ok(());
    }

    copy_snapshot_artifact(
        &base_dir.join("preset-catalog").join("catalog-state.json"),
        destination,
    )
}

fn copy_snapshot_artifact(source: &Path, destination: &Path) -> Result<(), HostErrorEnvelope> {
    let parent = destination.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("snapshot artifact 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        HostErrorEnvelope::persistence(format!("snapshot artifact 경로를 만들지 못했어요: {error}"))
    })?;
    let bytes = fs::read(source).map_err(|error| {
        HostErrorEnvelope::persistence(format!("snapshot source를 읽지 못했어요: {error}"))
    })?;
    fs::write(destination, bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("snapshot artifact를 저장하지 못했어요: {error}"))
    })?;

    Ok(())
}

fn derive_capture_lifecycle_stage(manifest: &SessionManifest) -> String {
    match manifest.captures.last() {
        Some(capture)
            if capture.render_status == "previewWaiting"
                || capture.render_status == "captureSaved" =>
        {
            "preview-waiting".into()
        }
        Some(capture) if capture.render_status == "renderFailed" => "phone-required".into(),
        Some(_) => "capture-ready".into(),
        None if manifest.active_preset.is_some() => "capture-ready".into(),
        None => "session-started".into(),
    }
}

fn seed_pending_preview_asset_path(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
) -> Option<FastPreviewPromotionResult> {
    let preferred_extensions = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
    let Some(preview_path) = preferred_extensions
        .iter()
        .map(|extension| {
            paths
                .renders_previews_dir
                .join(format!("{}.{}", capture_id, extension))
        })
        .find(|path| is_valid_render_preview_asset(path))
    else {
        return None;
    };

    let asset_path = preview_path.to_string_lossy().into_owned();
    log_fast_preview_event(
        paths,
        capture_id,
        request_id,
        "fast-preview-promoted",
        Some("legacy-canonical-scan"),
        Some(&format!("assetPath={asset_path}")),
    );

    Some(FastPreviewPromotionResult {
        asset_path,
        visible_at_ms: current_time_ms().ok(),
    })
}

fn promote_fast_preview_asset(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
    raw_asset_path: Option<&str>,
    handoff: &CompletedCaptureFastPreview,
) -> Option<FastPreviewPromotionResult> {
    log_fast_preview_event(
        paths,
        capture_id,
        request_id,
        "fast-preview-promote-start",
        handoff.kind.as_deref(),
        Some(&format!("assetPath={}", handoff.asset_path)),
    );

    let candidate_path = match validate_fast_preview_candidate(
        paths,
        capture_id,
        raw_asset_path,
        &handoff.asset_path,
    ) {
        Ok(path) => path,
        Err(reason) => {
            log_fast_preview_event(
                paths,
                capture_id,
                request_id,
                "fast-preview-invalid",
                handoff.kind.as_deref(),
                Some(&format!("reason={reason};assetPath={}", handoff.asset_path)),
            );
            return None;
        }
    };

    let canonical_path = paths.renders_previews_dir.join(format!("{capture_id}.jpg"));
    if let Some(parent) = canonical_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            log_fast_preview_event(
                paths,
                capture_id,
                request_id,
                "fast-preview-invalid",
                handoff.kind.as_deref(),
                Some(&format!(
                    "reason=promote-dir-unavailable;assetPath={};error={error}",
                    handoff.asset_path
                )),
            );
            return None;
        }
    }

    let normalized_candidate = normalize_path(&candidate_path);
    let normalized_canonical = normalize_path(&canonical_path);
    let mut backup_path = None;
    if normalized_candidate != normalized_canonical {
        if canonical_path.exists() {
            let candidate_backup_path =
                build_fast_preview_backup_path(&canonical_path, handoff.kind.as_deref());
            if let Err(error) = fs::rename(&canonical_path, &candidate_backup_path) {
                log_fast_preview_event(
                    paths,
                    capture_id,
                    request_id,
                    "fast-preview-invalid",
                    handoff.kind.as_deref(),
                    Some(&format!(
                        "reason=promote-overwrite-failed;assetPath={};canonicalPath={};error={error}",
                        handoff.asset_path,
                        canonical_path.to_string_lossy()
                    )),
                );
                return None;
            }

            backup_path = Some(candidate_backup_path);
        }

        if let Err(error) = fs::copy(&candidate_path, &canonical_path) {
            if let Some(backup_path) = backup_path.as_ref() {
                let _ = restore_fast_preview_backup(backup_path, &canonical_path);
            }
            log_fast_preview_event(
                paths,
                capture_id,
                request_id,
                "fast-preview-invalid",
                handoff.kind.as_deref(),
                Some(&format!(
                    "reason=promote-copy-failed;assetPath={};canonicalPath={};error={error}",
                    handoff.asset_path,
                    canonical_path.to_string_lossy()
                )),
            );
            return None;
        }
    }

    if !is_valid_render_preview_asset(&canonical_path) {
        let _ = fs::remove_file(&canonical_path);
        if let Some(backup_path) = backup_path.as_ref() {
            let _ = restore_fast_preview_backup(backup_path, &canonical_path);
        }
        log_fast_preview_event(
            paths,
            capture_id,
            request_id,
            "fast-preview-invalid",
            handoff.kind.as_deref(),
            Some(&format!(
                "reason=promote-output-invalid;assetPath={};canonicalPath={}",
                handoff.asset_path,
                canonical_path.to_string_lossy()
            )),
        );
        return None;
    }

    if let Some(backup_path) = backup_path.as_ref() {
        let _ = fs::remove_file(backup_path);
    }

    let asset_path = canonical_path.to_string_lossy().into_owned();
    log_fast_preview_event(
        paths,
        capture_id,
        request_id,
        "fast-preview-promoted",
        handoff.kind.as_deref(),
        Some(&format!("assetPath={asset_path}")),
    );
    log_fast_preview_event(
        paths,
        capture_id,
        request_id,
        "fast-preview-visible",
        handoff.kind.as_deref(),
        Some(&format!("assetPath={asset_path}")),
    );

    Some(FastPreviewPromotionResult {
        asset_path,
        visible_at_ms: current_time_ms().ok(),
    })
}

fn validate_fast_preview_candidate(
    paths: &SessionPaths,
    capture_id: &str,
    raw_asset_path: Option<&str>,
    candidate_asset_path: &str,
) -> Result<PathBuf, &'static str> {
    let candidate_path = PathBuf::from(candidate_asset_path);
    if !candidate_path.is_absolute() {
        return Err("not-absolute");
    }

    let canonical_candidate_path = fs::canonicalize(&candidate_path).map_err(|_| "missing")?;
    if !is_session_scoped_asset_path(paths, &canonical_candidate_path) {
        return Err("unscoped");
    }

    let extension = canonical_candidate_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or("missing-extension")?;
    if !FAST_PREVIEW_ALLOWED_EXTENSIONS
        .iter()
        .any(|allowed| *allowed == extension)
    {
        return Err("unsupported-extension");
    }

    let stem = canonical_candidate_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("missing-filename")?;
    let normalized_candidate = normalize_path(&canonical_candidate_path);
    let handoff_fast_preview_root =
        canonicalize_existing_root(&paths.handoff_dir.join("fast-preview"));
    let canonical_preview_root = canonicalize_existing_root(&paths.renders_previews_dir);
    let in_handoff_root = handoff_fast_preview_root
        .as_deref()
        .map(|root| {
            normalized_candidate == root
                || normalized_candidate.starts_with(&(root.to_string() + "/"))
        })
        .unwrap_or(false);
    let in_canonical_preview_root = canonical_preview_root
        .as_deref()
        .map(|root| {
            normalized_candidate == root
                || normalized_candidate.starts_with(&(root.to_string() + "/"))
        })
        .unwrap_or(false);

    if !in_handoff_root && !in_canonical_preview_root {
        return Err("disallowed-directory");
    }

    let matches_capture = if in_canonical_preview_root {
        stem == capture_id
    } else {
        stem == capture_id
            || stem.starts_with(&format!("{capture_id}."))
            || stem.starts_with(&format!("{capture_id}_"))
    };

    if !matches_capture {
        return Err("wrong-capture");
    }

    let preview_metadata = fs::metadata(&canonical_candidate_path).map_err(|_| "missing")?;
    if !preview_metadata.is_file() {
        return Err("missing");
    }
    if preview_metadata.len() == 0 {
        return Err("empty");
    }
    if !is_valid_render_preview_asset(&canonical_candidate_path) {
        return Err("invalid-raster");
    }

    if let Some(raw_asset_path) = raw_asset_path {
        let raw_metadata = fs::metadata(raw_asset_path).map_err(|_| "raw-missing")?;
        let preview_modified = preview_metadata.modified().ok().and_then(system_time_to_ms);
        let raw_modified = raw_metadata.modified().ok().and_then(system_time_to_ms);
        if let (Some(preview_modified), Some(raw_modified)) = (preview_modified, raw_modified) {
            if preview_modified < raw_modified {
                return Err("stale");
            }
        }
    }

    Ok(canonical_candidate_path)
}

fn is_session_scoped_asset_path(paths: &SessionPaths, candidate_path: &Path) -> bool {
    let normalized_candidate = normalize_path(candidate_path);
    let Some(normalized_session_root) = canonicalize_existing_root(&paths.session_root) else {
        return false;
    };

    normalized_candidate == normalized_session_root
        || normalized_candidate.starts_with(&(normalized_session_root + "/"))
}

fn log_fast_preview_event(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
    event: &str,
    kind: Option<&str>,
    detail: Option<&str>,
) {
    let occurred_at = match current_timestamp(SystemTime::now()) {
        Ok(value) => value,
        Err(_) => return,
    };
    let _ = fs::create_dir_all(&paths.diagnostics_dir);
    let log_path = paths.diagnostics_dir.join("timing-events.log");
    let mut file = match OpenOptions::new().create(true).append(true).open(log_path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let kind = kind.unwrap_or("none");
    let detail = detail.unwrap_or("none");
    let _ = writeln!(
        file,
        "{occurred_at}\tsession={}\tcapture={capture_id}\trequest={request_id}\tevent={event}\tkind={kind}\tdetail={detail}",
        paths
            .session_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    );
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_lowercase()
        .trim_start_matches("//?/")
        .to_string()
}

fn canonicalize_existing_root(path: &Path) -> Option<String> {
    fs::canonicalize(path)
        .ok()
        .map(|resolved| normalize_path(&resolved).trim_end_matches('/').to_string())
}

fn build_fast_preview_backup_path(canonical_path: &Path, kind: Option<&str>) -> PathBuf {
    let parent = canonical_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = canonical_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("preview");
    let extension = canonical_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("jpg");
    let suffix = kind
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.replace(['\\', '/', ' ', ':', ';'], "-"))
        .unwrap_or_else(|| "backup".into());

    parent.join(format!("{stem}.{suffix}.backup.{extension}"))
}

fn restore_fast_preview_backup(
    backup_path: &Path,
    canonical_path: &Path,
) -> Result<(), std::io::Error> {
    if canonical_path.exists() {
        fs::remove_file(canonical_path)?;
    }

    fs::rename(backup_path, canonical_path)
}

fn speculative_preview_output_path(paths: &SessionPaths, capture_id: &str) -> PathBuf {
    paths
        .renders_previews_dir
        .join(format!("{capture_id}.preview-speculative.jpg"))
}

fn speculative_preview_lock_path(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
) -> PathBuf {
    paths.renders_previews_dir.join(format!(
        "{capture_id}.{request_id}.preview-speculative.lock"
    ))
}

fn speculative_preview_detail_path(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
) -> PathBuf {
    paths.renders_previews_dir.join(format!(
        "{capture_id}.{request_id}.preview-speculative.detail"
    ))
}

fn speculative_preview_source_path(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
    extension: &str,
) -> PathBuf {
    paths.renders_previews_dir.join(format!(
        "{capture_id}.{request_id}.preview-speculative-source.{extension}"
    ))
}

fn prepare_speculative_preview_source_path(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
    source_path: &Path,
) -> Option<PreparedSpeculativePreviewSource> {
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| FAST_PREVIEW_ALLOWED_EXTENSIONS.contains(&value.as_str()))
        .unwrap_or_else(|| "jpg".into());
    let staged_source_path =
        speculative_preview_source_path(paths, capture_id, request_id, extension.as_str());
    if let Some(parent) = staged_source_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return None;
        }
    }

    let _ = fs::remove_file(&staged_source_path);
    if fs::copy(source_path, &staged_source_path).is_err() {
        return None;
    }

    if !is_valid_render_preview_asset(&staged_source_path) {
        let _ = fs::remove_file(&staged_source_path);
        return None;
    }

    Some(PreparedSpeculativePreviewSource {
        asset_path: staged_source_path.clone(),
        cleanup_path: Some(staged_source_path),
    })
}

fn current_time_ms() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

fn append_capture_preview_ready_event(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    preview_ready_at_ms: u64,
) {
    let detail = format!(
        "elapsedMs={};budgetState={};renderStatus={}",
        preview_ready_at_ms.saturating_sub(capture.timing.capture_acknowledged_at_ms),
        capture.timing.preview_budget_state,
        capture.render_status
    );
    let _ = append_session_timing_event_in_dir(
        base_dir,
        SessionTimingEventInput {
            session_id,
            event: "capture_preview_ready",
            capture_id: Some(&capture.capture_id),
            request_id: Some(&capture.request_id),
            detail: Some(&detail),
        },
    );
}

fn system_time_to_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn should_skip_speculative_preview_render_for_route(
    route_snapshot: &PreviewRendererRouteSnapshot,
    warm_state_snapshot: Option<&PreviewRendererWarmStateSnapshot>,
    active_preset: &ActivePresetBinding,
) -> bool {
    let uses_legacy_dedicated_route = route_snapshot.route == "local-renderer-sidecar"
        && route_snapshot.implementation_track.as_deref() != Some("actual-primary-lane");

    if !uses_legacy_dedicated_route
        || !matches!(route_snapshot.route_stage.as_str(), "canary" | "default")
        || route_snapshot.fallback_reason_code.is_some()
    {
        return false;
    }

    let Some(warm_state_snapshot) = warm_state_snapshot else {
        return false;
    };

    warm_state_snapshot.preset_id == active_preset.preset_id
        && warm_state_snapshot.published_version == active_preset.published_version
        && matches!(warm_state_snapshot.state.as_str(), "warm-ready" | "warm-hit")
}

#[cfg(test)]
mod tests {
    use super::*;

    static SPECULATIVE_WAIT_TEST_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));
    const SPECULATIVE_WAIT_ASSERTION_SLACK_MS: u64 = 400;

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "boothy-ingest-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be available")
                .as_nanos()
        ))
    }

    #[test]
    fn speculative_preview_wait_budget_stays_bounded_even_while_another_capture_is_in_flight() {
        let _guard = SPECULATIVE_WAIT_TEST_MUTEX
            .lock()
            .expect("test mutex should lock");
        let base_dir = unique_temp_dir("speculative-preview-wait");
        let session_id = "session_000000000000000000000001";
        let paths = SessionPaths::new(&base_dir, session_id);
        let capture_id = "capture_test";
        let request_id = "request_test";
        let runtime_key = base_dir.to_string_lossy().into_owned();
        let lock_path = speculative_preview_lock_path(&paths, capture_id, request_id);

        fs::create_dir_all(&paths.renders_previews_dir)
            .expect("preview directory should be created");
        fs::write(&lock_path, request_id).expect("speculative lock should be written");
        IN_FLIGHT_CAPTURE_SESSIONS
            .lock()
            .expect("in-flight capture sessions should lock")
            .insert(runtime_key.clone(), session_id.to_string());

        let wait_started = std::time::Instant::now();
        wait_for_speculative_preview_completion_for_request_in_dir(
            &base_dir, &paths, capture_id, request_id,
        );
        let waited_for = wait_started.elapsed();
        let expected_upper_bound_ms = SPECULATIVE_PREVIEW_WAIT_MS
            + SPECULATIVE_PREVIEW_DRAIN_WAIT_MS
            + SPECULATIVE_WAIT_ASSERTION_SLACK_MS;

        assert!(
            waited_for < Duration::from_millis(expected_upper_bound_ms),
            "speculative wait should fall through quickly even if another capture is in flight, actual wait: {waited_for:?}"
        );

        IN_FLIGHT_CAPTURE_SESSIONS
            .lock()
            .expect("in-flight capture sessions should lock")
            .remove(&runtime_key);
        let _ = fs::remove_file(lock_path);
        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn speculative_preview_wait_budget_stays_bounded_without_in_flight_capture() {
        let _guard = SPECULATIVE_WAIT_TEST_MUTEX
            .lock()
            .expect("test mutex should lock");
        let base_dir = unique_temp_dir("speculative-preview-fast-fallback");
        let session_id = "session_000000000000000000000001";
        let paths = SessionPaths::new(&base_dir, session_id);
        let capture_id = "capture_test";
        let request_id = "request_test";
        let lock_path = speculative_preview_lock_path(&paths, capture_id, request_id);

        fs::create_dir_all(&paths.renders_previews_dir)
            .expect("preview directory should be created");
        fs::write(&lock_path, request_id).expect("speculative lock should be written");

        let wait_started = std::time::Instant::now();
        wait_for_speculative_preview_completion_for_request_in_dir(
            &base_dir, &paths, capture_id, request_id,
        );
        let waited_for = wait_started.elapsed();
        let expected_upper_bound_ms = SPECULATIVE_PREVIEW_WAIT_MS
            + SPECULATIVE_PREVIEW_DRAIN_WAIT_MS
            + SPECULATIVE_WAIT_ASSERTION_SLACK_MS;

        assert!(
            waited_for < Duration::from_millis(expected_upper_bound_ms),
            "speculative wait should fall through quickly when no capture is in flight, actual wait: {waited_for:?}"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn speculative_preview_source_is_staged_to_a_stable_copy() {
        let _guard = SPECULATIVE_WAIT_TEST_MUTEX
            .lock()
            .expect("test mutex should lock");
        let base_dir = unique_temp_dir("speculative-preview-source-copy");
        let session_id = "session_000000000000000000000001";
        let paths = SessionPaths::new(&base_dir, session_id);
        let source_path = paths.renders_previews_dir.join("capture_test.jpg");

        fs::create_dir_all(&paths.renders_previews_dir)
            .expect("preview directory should be created");
        fs::write(&source_path, [0xFF, 0xD8, 0xFF, 0xD9])
            .expect("source preview should be writable");

        let prepared = prepare_speculative_preview_source_path(
            &paths,
            "capture_test",
            "request_test",
            &source_path,
        )
        .expect("speculative preview source should be staged");

        assert_ne!(
            prepared.asset_path, source_path,
            "speculative render should not read the mutable canonical preview in place"
        );
        assert!(
            prepared
                .asset_path
                .ends_with("capture_test.request_test.preview-speculative-source.jpg"),
            "staged source path should be request-scoped for cleanup and race isolation"
        );
        assert!(
            is_valid_render_preview_asset(&prepared.asset_path),
            "staged source should stay displayable"
        );

        fs::remove_file(&source_path).expect("source preview should be removable");
        assert!(
            prepared.asset_path.exists(),
            "staged source should survive canonical preview replacement"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn keeps_speculative_preview_when_actual_primary_lane_is_warm_and_active() {
        let route_snapshot = PreviewRendererRouteSnapshot {
            route: "local-renderer-sidecar".into(),
            route_stage: "canary".into(),
            fallback_reason_code: None,
            implementation_track: Some("actual-primary-lane".into()),
        };
        let warm_state_snapshot = PreviewRendererWarmStateSnapshot {
            preset_id: "preset_test-look".into(),
            published_version: "2026.03.31".into(),
            state: "warm-hit".into(),
            observed_at: "2026-04-14T06:30:00Z".into(),
            diagnostics_detail_path: None,
        };
        let active_preset = ActivePresetBinding {
            preset_id: "preset_test-look".into(),
            published_version: "2026.03.31".into(),
        };

        assert!(!should_skip_speculative_preview_render_for_route(
            &route_snapshot,
            Some(&warm_state_snapshot),
            &active_preset,
        ));
    }

    #[test]
    fn skips_speculative_preview_only_for_legacy_dedicated_route_when_warm_and_active() {
        let route_snapshot = PreviewRendererRouteSnapshot {
            route: "local-renderer-sidecar".into(),
            route_stage: "canary".into(),
            fallback_reason_code: None,
            implementation_track: None,
        };
        let warm_state_snapshot = PreviewRendererWarmStateSnapshot {
            preset_id: "preset_test-look".into(),
            published_version: "2026.03.31".into(),
            state: "warm-hit".into(),
            observed_at: "2026-04-14T06:30:00Z".into(),
            diagnostics_detail_path: None,
        };
        let active_preset = ActivePresetBinding {
            preset_id: "preset_test-look".into(),
            published_version: "2026.03.31".into(),
        };

        assert!(should_skip_speculative_preview_render_for_route(
            &route_snapshot,
            Some(&warm_state_snapshot),
            &active_preset,
        ));
    }

    #[test]
    fn keeps_speculative_preview_when_warm_state_is_for_another_preset() {
        let route_snapshot = PreviewRendererRouteSnapshot {
            route: "local-renderer-sidecar".into(),
            route_stage: "canary".into(),
            fallback_reason_code: None,
            implementation_track: Some("actual-primary-lane".into()),
        };
        let warm_state_snapshot = PreviewRendererWarmStateSnapshot {
            preset_id: "preset_other".into(),
            published_version: "2026.03.31".into(),
            state: "warm-hit".into(),
            observed_at: "2026-04-14T06:30:00Z".into(),
            diagnostics_detail_path: None,
        };
        let active_preset = ActivePresetBinding {
            preset_id: "preset_test-look".into(),
            published_version: "2026.03.31".into(),
        };

        assert!(!should_skip_speculative_preview_render_for_route(
            &route_snapshot,
            Some(&warm_state_snapshot),
            &active_preset,
        ));
    }
}
