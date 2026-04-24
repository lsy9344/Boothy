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
        enqueue_resident_preview_render_in_dir, is_valid_render_preview_asset,
        log_render_failure_in_dir, log_render_ready_in_dir, log_render_start_in_dir,
        promote_preview_render_output, render_capture_asset_from_raw_in_dir,
        render_capture_asset_in_dir, render_preview_asset_to_path_in_dir, RenderIntent,
    },
    session::{
        session_manifest::{
            current_timestamp, ActivePresetBinding, CaptureTimingMetrics, FinalCaptureAsset,
            PreviewCaptureAsset, RawCaptureAsset, SessionCaptureRecord, SessionManifest,
            CAPTURE_BUDGET_MS, PREVIEW_BUDGET_MS, SESSION_CAPTURE_SCHEMA_VERSION,
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
// keep serializing behind that in-flight work for the full renderer timeout.
// Recent first-shot field evidence showed that opening a second darktable lane
// around the 7s mark can crash both renders and force `existing-preview-fallback`
// instead of a truthful close.
const SPECULATIVE_PREVIEW_JOIN_WAIT_MS: u64 = 45000;
const SPECULATIVE_PREVIEW_JOIN_POLL_MS: u64 = 80;
const PREVIEW_REFINEMENT_IDLE_WAIT_MS: u64 = 5000;
const PREVIEW_REFINEMENT_IDLE_POLL_MS: u64 = 80;
const LEGACY_CANONICAL_SCAN_FAST_PREVIEW_KIND: &str = "legacy-canonical-scan";
const TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND: &str = "preset-applied-preview";

#[derive(Debug, Clone)]
struct FastPreviewPromotionResult {
    asset_path: String,
    kind: Option<String>,
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
    early_fast_preview_update: Option<&FastPreviewReadyUpdate>,
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
    let reused_early_fast_preview = reuse_early_fast_preview_update(
        &paths,
        &capture.capture_id,
        &capture.request_id,
        early_fast_preview_update,
    );
    let selected_fast_preview = select_saved_fast_preview_baseline(
        promoted_fast_preview.clone(),
        reused_early_fast_preview.clone(),
    );
    let seeded_fast_preview = if selected_fast_preview.is_none() {
        seed_pending_preview_asset_path(&paths, &capture.capture_id, &capture.request_id)
    } else {
        None
    };
    let first_visible_at_ms = saved_fast_preview_visible_at_ms(
        selected_fast_preview.as_ref(),
        reused_early_fast_preview.as_ref(),
        seeded_fast_preview.as_ref(),
    );

    if let Some(ref selected_fast_preview) = selected_fast_preview {
        capture.preview.asset_path = Some(selected_fast_preview.asset_path.clone());
        capture.preview.kind = selected_fast_preview.kind.clone();
    } else if let Some(ref seed_result) = seeded_fast_preview {
        // If helper handoff metadata is missing or invalid but the same-capture
        // preview file already exists on disk, keep the fast-path alive.
        capture.preview.asset_path = Some(seed_result.asset_path.clone());
        capture.preview.kind = seed_result.kind.clone();
    }
    capture.timing.fast_preview_visible_at_ms = first_visible_at_ms;

    let promoted_fast_preview_kind = selected_fast_preview
        .as_ref()
        .or(seeded_fast_preview.as_ref())
        .and_then(|promoted| promoted.kind.as_deref());
    let truthful_fast_preview_ready_at_ms = selected_fast_preview
        .as_ref()
        .or(seeded_fast_preview.as_ref())
        .and_then(|promoted| promoted.visible_at_ms)
        .or(Some(persisted_at_ms))
        .filter(|_| is_truthful_fast_preview_kind(promoted_fast_preview_kind));
    let truthful_fast_preview_closed = truthful_fast_preview_ready_at_ms
        .map(|ready_at_ms| close_preview_truth(&mut capture, ready_at_ms))
        .unwrap_or(false);

    let fast_preview_update = promoted_fast_preview.as_ref().and_then(|promoted| {
        should_emit_promoted_fast_preview_after_persist(selected_fast_preview.as_ref(), promoted)
            .then(|| {
                promoted_fast_preview_ready_update(
                    &capture.request_id,
                    &capture.capture_id,
                    promoted,
                    persisted_at_ms,
                )
            })
    });

    manifest.captures.push(capture.clone());
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(&manifest);

    write_session_manifest(&paths.manifest_path, &manifest)?;

    if truthful_fast_preview_closed {
        let preview_ready_at_ms = capture
            .preview
            .ready_at_ms
            .expect("truthful fast preview close should set ready timestamp");
        log_render_ready_in_dir(
            base_dir,
            &manifest.session_id,
            &capture.capture_id,
            &capture.request_id,
            RenderIntent::Preview,
            &truthful_fast_preview_render_detail(
                &active_preset.preset_id,
                &active_preset.published_version,
                preview_ready_at_ms,
                capture.timing.capture_acknowledged_at_ms,
            ),
        );
        append_capture_preview_ready_event(
            base_dir,
            &manifest.session_id,
            &capture,
            preview_ready_at_ms,
        );
    }

    if capture.preview.ready_at_ms.is_none() {
        if let Some(first_visible_asset_path) = capture.preview.asset_path.as_deref() {
            let fast_preview_kind = promoted_fast_preview
                .as_ref()
                .or(seeded_fast_preview.as_ref())
                .and_then(|promoted| promoted.kind.as_deref());
            if should_start_speculative_preview_render(fast_preview_kind) {
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

pub fn reconcile_saved_capture_fast_preview_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    update: &FastPreviewReadyUpdate,
) -> Result<Option<SessionCaptureRecord>, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let Some(capture_index) = manifest.captures.iter_mut().position(|capture| {
        capture.capture_id == capture_id && capture.request_id == update.request_id
    }) else {
        return Ok(None);
    };
    let capture = manifest
        .captures
        .get_mut(capture_index)
        .expect("capture index should stay valid");

    if capture.preview.ready_at_ms.is_some()
        || is_truthful_fast_preview_kind(capture.preview.kind.as_deref())
    {
        return Ok(Some(capture.clone()));
    }

    capture.preview.asset_path = Some(update.asset_path.clone());
    capture.preview.kind = update.kind.clone();
    if capture.timing.fast_preview_visible_at_ms.is_none() {
        capture.timing.fast_preview_visible_at_ms = Some(update.visible_at_ms);
    }
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    let capture_snapshot = capture.clone();
    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok(Some(capture_snapshot))
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

        if let Some(existing_capture) = try_close_existing_preview_without_render_in_manifest(
            base_dir,
            &paths,
            &mut manifest,
            capture_index,
        )? {
            return Ok(existing_capture);
        }

        let promoted_late_fast_preview_kind =
            sync_helper_fast_preview_before_render(base_dir, &paths, &mut manifest, capture_index)?;
        if promoted_late_fast_preview_kind.is_some() {
            let capture = manifest
                .captures
                .get(capture_index)
                .cloned()
                .expect("capture index already resolved");
            if capture.preview.ready_at_ms.is_some() {
                return Ok(capture);
            }
            if let (Some(preset_id), Some(first_visible_asset_path)) = (
                capture.active_preset_id.as_deref(),
                capture.preview.asset_path.as_deref(),
            ) {
                if should_start_speculative_preview_render(
                    promoted_late_fast_preview_kind.as_deref(),
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
                }
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

        if let Some(existing_capture) = try_close_existing_preview_without_render_in_manifest(
            base_dir,
            &paths,
            &mut manifest,
            capture_index,
        )? {
            return Ok(existing_capture);
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
    let rendered_preview = match render_capture_asset_from_raw_with_queue_retry_in_dir(
        base_dir,
        session_id,
        &capture_snapshot,
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
            if let Some(fallback_capture) = try_close_existing_preview_after_render_failure_in_dir(
                base_dir, &paths, capture_id,
            )? {
                return Ok(fallback_capture);
            }
            return Err(HostErrorEnvelope::persistence(error.customer_message));
        }
    };

    finish_preview_render_in_dir(base_dir, &paths, session_id, capture_id, rendered_preview)
}

fn render_capture_asset_from_raw_with_queue_retry_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
) -> Result<crate::render::RenderedCaptureAsset, crate::render::RenderWorkerError> {
    let wait_cycles = (PREVIEW_REFINEMENT_IDLE_WAIT_MS / PREVIEW_REFINEMENT_IDLE_POLL_MS).max(1);
    let mut last_error = None;

    for attempt in 0..=wait_cycles {
        match render_capture_asset_from_raw_in_dir(
            base_dir,
            session_id,
            capture,
            RenderIntent::Preview,
        ) {
            Ok(rendered_preview) => return Ok(rendered_preview),
            Err(error)
                if error.reason_code == "render-queue-saturated" && attempt < wait_cycles =>
            {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(PREVIEW_REFINEMENT_IDLE_POLL_MS));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.expect("queue retry should preserve the last saturation error"))
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
    let speculative_detail_path = speculative_preview_detail_path(paths, capture_id, request_id);
    let speculative_lock_path = speculative_preview_lock_path(paths, capture_id, request_id);

    if !speculative_lock_path.exists() {
        return;
    }

    let wait_cycles = (SPECULATIVE_PREVIEW_WAIT_MS / SPECULATIVE_PREVIEW_POLL_MS).max(1);
    for attempt in 0..=wait_cycles {
        if speculative_preview_output_is_ready_to_promote(
            &speculative_output_path,
            &speculative_detail_path,
            &speculative_lock_path,
        ) {
            return;
        }

        if attempt < wait_cycles {
            thread::sleep(Duration::from_millis(SPECULATIVE_PREVIEW_POLL_MS));
        }
    }

    let drain_cycles =
        (SPECULATIVE_PREVIEW_DRAIN_WAIT_MS / SPECULATIVE_PREVIEW_DRAIN_POLL_MS).max(1);
    for attempt in 0..=drain_cycles {
        if speculative_preview_output_is_ready_to_promote(
            &speculative_output_path,
            &speculative_detail_path,
            &speculative_lock_path,
        ) {
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
    let speculative_detail_path = speculative_preview_detail_path(paths, capture_id, request_id);
    let speculative_lock_path = speculative_preview_lock_path(paths, capture_id, request_id);

    if !speculative_lock_path.exists() {
        return;
    }

    let wait_cycles = (SPECULATIVE_PREVIEW_JOIN_WAIT_MS / SPECULATIVE_PREVIEW_JOIN_POLL_MS).max(1);
    for attempt in 0..=wait_cycles {
        if speculative_preview_output_is_ready_to_promote(
            &speculative_output_path,
            &speculative_detail_path,
            &speculative_lock_path,
        ) {
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
    let speculative_detail_path = speculative_preview_detail_path(
        paths,
        &capture_snapshot.capture_id,
        &capture_snapshot.request_id,
    );
    let speculative_lock_path = speculative_preview_lock_path(
        paths,
        &capture_snapshot.capture_id,
        &capture_snapshot.request_id,
    );

    if !speculative_lock_path.exists() && !is_valid_render_preview_asset(&speculative_output_path) {
        return Ok(None);
    }

    if speculative_preview_output_is_ready_to_promote(
        &speculative_output_path,
        &speculative_detail_path,
        &speculative_lock_path,
    ) {
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
        let render_detail = fs::read_to_string(&speculative_detail_path)
        .unwrap_or_else(|_| {
            "presetId=unknown;publishedVersion=unknown;binary=darktable-cli;source=unknown;elapsedMs=unknown;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=unknown;status=unknown"
                .into()
        });
        let truthful_render_detail = normalize_preset_applied_render_detail(&render_detail);
        log_render_ready_in_dir(
            base_dir,
            &capture_snapshot.session_id,
            &capture_snapshot.capture_id,
            &capture_snapshot.request_id,
            RenderIntent::Preview,
            &truthful_render_detail,
        );

        let capture = {
            let capture = manifest
                .captures
                .get_mut(capture_index)
                .expect("capture index already resolved");

            capture.preview.asset_path =
                Some(canonical_preview_path.to_string_lossy().into_owned());
            capture.preview.ready_at_ms = Some(preview_visible_at_ms);
            capture.preview.kind = Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into());
            capture.render_status = "previewReady".into();
            capture.timing.preview_visible_at_ms = Some(preview_visible_at_ms);
            capture.timing.xmp_preview_ready_at_ms = Some(preview_visible_at_ms);
            capture.timing.fast_preview_visible_at_ms = Some(
                capture
                    .timing
                    .fast_preview_visible_at_ms
                    .unwrap_or(preview_visible_at_ms),
            );
            capture.timing.preview_budget_state = if preview_visible_at_ms
                .saturating_sub(capture.timing.capture_acknowledged_at_ms)
                <= PREVIEW_BUDGET_MS
            {
                "withinBudget".into()
            } else {
                "exceededBudget".into()
            };

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

        let _ = fs::remove_file(&speculative_detail_path);

        return Ok(Some(capture));
    }

    Ok(None)
}

fn speculative_preview_output_is_ready_to_promote(
    speculative_output_path: &Path,
    speculative_detail_path: &Path,
    speculative_lock_path: &Path,
) -> bool {
    is_valid_render_preview_asset(speculative_output_path)
        && (!speculative_lock_path.exists() || speculative_detail_path.is_file())
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

fn finish_preview_render_in_dir(
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
        capture.preview.kind = rendered_preview
            .preview_kind
            .or_else(|| capture.preview.kind.clone())
            .or_else(|| Some("raw-original".into()));
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

fn should_close_existing_preview_without_render(capture: &SessionCaptureRecord) -> bool {
    capture.preview.ready_at_ms.is_none()
        && capture
            .preview
            .asset_path
            .as_deref()
            .map(Path::new)
            .map(is_valid_render_preview_asset)
            .unwrap_or(false)
        && is_truthful_fast_preview_kind(capture.preview.kind.as_deref())
}

fn has_displayable_existing_preview(paths: &SessionPaths, capture: &SessionCaptureRecord) -> bool {
    capture
        .preview
        .asset_path
        .as_deref()
        .map(Path::new)
        .filter(|asset_path| is_valid_render_preview_asset(asset_path))
        .and_then(|asset_path| {
            is_session_scoped_asset_path(paths, asset_path)
                .then(|| asset_path.to_string_lossy().into_owned())
        })
        .is_some()
}

fn existing_preview_render_failure_fallback_detail(
    capture: &SessionCaptureRecord,
    preview_ready_at_ms: u64,
) -> String {
    format!(
        "presetId={};publishedVersion={};binary=existing-preview-fallback;source=existing-preview-fallback;elapsedMs={};detail=widthCap=display;heightCap=display;hq=false;sourceAsset={};truthOwner=existing-preview-fallback;args=none;status=render-failed-fallback",
        capture.active_preset_id.as_deref().unwrap_or("unknown"),
        capture.active_preset_version,
        preview_ready_at_ms.saturating_sub(capture.timing.capture_acknowledged_at_ms),
        capture
            .preview
            .kind
            .as_deref()
            .unwrap_or(LEGACY_CANONICAL_SCAN_FAST_PREVIEW_KIND)
    )
}

fn try_close_existing_preview_after_render_failure_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    capture_id: &str,
) -> Result<Option<SessionCaptureRecord>, HostErrorEnvelope> {
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("프리뷰 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let Some(capture_index) = manifest
        .captures
        .iter()
        .position(|capture| capture.capture_id == capture_id)
    else {
        return Ok(None);
    };
    let Some(capture) = manifest.captures.get(capture_index) else {
        return Ok(None);
    };
    if capture.preview.ready_at_ms.is_some() || !has_displayable_existing_preview(paths, capture) {
        return Ok(None);
    }

    let preview_ready_at_ms = capture
        .timing
        .fast_preview_visible_at_ms
        .or(Some(capture.raw.persisted_at_ms))
        .or_else(|| current_time_ms().ok())
        .ok_or_else(|| {
            HostErrorEnvelope::persistence(
                "프리뷰 시각을 기록하지 못했어요. 잠시 후 다시 시도해 주세요.",
            )
        })?;
    let capture = {
        let capture = manifest
            .captures
            .get_mut(capture_index)
            .expect("capture index already resolved");
        capture.preview.ready_at_ms = Some(preview_ready_at_ms);
        capture.render_status = "previewReady".into();
        capture.timing.preview_visible_at_ms = capture
            .timing
            .preview_visible_at_ms
            .or(Some(preview_ready_at_ms));
        capture.timing.preview_budget_state = if preview_ready_at_ms
            .saturating_sub(capture.timing.capture_acknowledged_at_ms)
            <= PREVIEW_BUDGET_MS
        {
            "withinBudget".into()
        } else {
            "exceededBudget".into()
        };
        capture.clone()
    };

    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(&manifest);
    write_session_manifest(&paths.manifest_path, &manifest)?;
    log_render_ready_in_dir(
        base_dir,
        &capture.session_id,
        &capture.capture_id,
        &capture.request_id,
        RenderIntent::Preview,
        &existing_preview_render_failure_fallback_detail(&capture, preview_ready_at_ms),
    );
    append_capture_preview_ready_event(
        base_dir,
        &capture.session_id,
        &capture,
        preview_ready_at_ms,
    );

    Ok(Some(capture))
}

fn close_truthful_preview_in_manifest(
    base_dir: &Path,
    paths: &SessionPaths,
    manifest: &mut SessionManifest,
    capture_index: usize,
    promoted_fast_preview: FastPreviewPromotionResult,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    let preview_ready_at_ms =
        promoted_fast_preview
            .visible_at_ms
            .unwrap_or(current_time_ms().map_err(|_| {
                HostErrorEnvelope::persistence(
                    "프리뷰 시각을 기록하지 못했어요. 잠시 후 다시 시도해 주세요.",
                )
            })?);
    let (capture_snapshot, first_truth_close) = {
        let capture = manifest
            .captures
            .get_mut(capture_index)
            .expect("capture index already resolved");
        capture.preview.asset_path = Some(promoted_fast_preview.asset_path);
        capture.preview.kind = promoted_fast_preview
            .kind
            .clone()
            .or_else(|| Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into()));
        if capture.timing.fast_preview_visible_at_ms.is_none() {
            capture.timing.fast_preview_visible_at_ms = promoted_fast_preview.visible_at_ms;
        }
        let first_truth_close = close_preview_truth(capture, preview_ready_at_ms);
        (capture.clone(), first_truth_close)
    };
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(manifest);
    write_session_manifest(&paths.manifest_path, manifest)?;
    if first_truth_close {
        log_render_ready_in_dir(
            base_dir,
            &capture_snapshot.session_id,
            &capture_snapshot.capture_id,
            &capture_snapshot.request_id,
            RenderIntent::Preview,
            &truthful_fast_preview_render_detail(
                capture_snapshot
                    .active_preset_id
                    .as_deref()
                    .unwrap_or("unknown"),
                &capture_snapshot.active_preset_version,
                preview_ready_at_ms,
                capture_snapshot.timing.capture_acknowledged_at_ms,
            ),
        );
        append_capture_preview_ready_event(
            base_dir,
            &capture_snapshot.session_id,
            &capture_snapshot,
            preview_ready_at_ms,
        );
    }

    Ok(capture_snapshot)
}

fn try_close_existing_preview_without_render_in_manifest(
    base_dir: &Path,
    paths: &SessionPaths,
    manifest: &mut SessionManifest,
    capture_index: usize,
) -> Result<Option<SessionCaptureRecord>, HostErrorEnvelope> {
    let Some(capture) = manifest.captures.get(capture_index) else {
        return Ok(None);
    };
    if !should_close_existing_preview_without_render(capture) {
        return Ok(None);
    }

    let promoted_fast_preview = FastPreviewPromotionResult {
        asset_path: capture
            .preview
            .asset_path
            .clone()
            .expect("existing preview close requires an asset path"),
        kind: Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into()),
        visible_at_ms: capture
            .timing
            .fast_preview_visible_at_ms
            .or(capture.preview.ready_at_ms)
            .or(Some(capture.raw.persisted_at_ms)),
    };

    close_truthful_preview_in_manifest(
        base_dir,
        paths,
        manifest,
        capture_index,
        promoted_fast_preview,
    )
    .map(Some)
}

fn sync_helper_fast_preview_before_render(
    base_dir: &Path,
    paths: &SessionPaths,
    manifest: &mut SessionManifest,
    capture_index: usize,
) -> Result<Option<String>, HostErrorEnvelope> {
    let Some(capture) = manifest.captures.get_mut(capture_index) else {
        return Ok(None);
    };

    if capture.preview.ready_at_ms.is_some() {
        return Ok(None);
    }

    if let Some(promoted_fast_preview) =
        seed_truthful_fast_preview_asset_path(paths, &capture.capture_id, &capture.request_id)
    {
        let preview_kind = promoted_fast_preview.kind.clone();
        let _ = close_truthful_preview_in_manifest(
            base_dir,
            paths,
            manifest,
            capture_index,
            promoted_fast_preview,
        )?;
        return Ok(preview_kind);
    }

    if capture.preview.asset_path.is_some() {
        return Ok(None);
    }

    let wait_cycles = (HELPER_FAST_PREVIEW_WAIT_MS / HELPER_FAST_PREVIEW_POLL_MS).max(1);
    for _ in 0..=wait_cycles {
        if let Some(promoted_fast_preview) =
            seed_pending_preview_asset_path(paths, &capture.capture_id, &capture.request_id)
        {
            let preview_kind = promoted_fast_preview.kind.clone();
            if is_truthful_fast_preview_kind(preview_kind.as_deref()) {
                let _ = close_truthful_preview_in_manifest(
                    base_dir,
                    paths,
                    manifest,
                    capture_index,
                    promoted_fast_preview,
                )?;
            } else {
                capture.preview.asset_path = Some(promoted_fast_preview.asset_path);
                capture.preview.kind = promoted_fast_preview.kind.clone();
                if capture.timing.fast_preview_visible_at_ms.is_none() {
                    capture.timing.fast_preview_visible_at_ms = promoted_fast_preview.visible_at_ms;
                }
                manifest.updated_at = current_timestamp(SystemTime::now())?;
                write_session_manifest(&paths.manifest_path, manifest)?;
            }
            return Ok(preview_kind);
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

    Ok(None)
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
            kind: None,
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
    if let Some(promoted_truthful_preview) =
        seed_truthful_fast_preview_asset_path(paths, capture_id, request_id)
    {
        return Some(promoted_truthful_preview);
    }

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
        Some(LEGACY_CANONICAL_SCAN_FAST_PREVIEW_KIND),
        Some(&format!("assetPath={asset_path}")),
    );

    Some(FastPreviewPromotionResult {
        asset_path,
        kind: Some(LEGACY_CANONICAL_SCAN_FAST_PREVIEW_KIND.into()),
        visible_at_ms: current_time_ms().ok(),
    })
}

fn reuse_early_fast_preview_update(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
    update: Option<&FastPreviewReadyUpdate>,
) -> Option<FastPreviewPromotionResult> {
    let update = update?;
    if update.capture_id != capture_id || update.request_id != request_id {
        return None;
    }

    let candidate_path = PathBuf::from(&update.asset_path);
    if !candidate_path.is_file()
        || !is_session_scoped_asset_path(paths, &candidate_path)
        || !is_valid_render_preview_asset(&candidate_path)
    {
        return None;
    }

    Some(FastPreviewPromotionResult {
        asset_path: update.asset_path.clone(),
        kind: update.kind.clone(),
        visible_at_ms: Some(update.visible_at_ms),
    })
}

fn select_saved_fast_preview_baseline(
    promoted_fast_preview: Option<FastPreviewPromotionResult>,
    reused_early_fast_preview: Option<FastPreviewPromotionResult>,
) -> Option<FastPreviewPromotionResult> {
    match (reused_early_fast_preview, promoted_fast_preview) {
        (Some(early_fast_preview), Some(promoted_fast_preview)) => {
            if should_promoted_fast_preview_override_saved_baseline(
                &early_fast_preview,
                &promoted_fast_preview,
            ) {
                Some(promoted_fast_preview)
            } else {
                Some(early_fast_preview)
            }
        }
        (Some(early_fast_preview), None) => Some(early_fast_preview),
        (None, Some(promoted_fast_preview)) => Some(promoted_fast_preview),
        (None, None) => None,
    }
}

fn should_promoted_fast_preview_override_saved_baseline(
    current: &FastPreviewPromotionResult,
    candidate: &FastPreviewPromotionResult,
) -> bool {
    if current.asset_path == candidate.asset_path && current.kind == candidate.kind {
        return false;
    }

    match (
        is_truthful_fast_preview_kind(current.kind.as_deref()),
        is_truthful_fast_preview_kind(candidate.kind.as_deref()),
    ) {
        (false, true) => true,
        (true, false) => false,
        (false, false) => false,
        _ => {
            candidate.visible_at_ms > current.visible_at_ms
                || (candidate.visible_at_ms == current.visible_at_ms
                    && candidate.asset_path != current.asset_path)
        }
    }
}

fn should_emit_promoted_fast_preview_after_persist(
    selected_fast_preview: Option<&FastPreviewPromotionResult>,
    promoted_fast_preview: &FastPreviewPromotionResult,
) -> bool {
    selected_fast_preview.is_some_and(|selected_fast_preview| {
        selected_fast_preview.asset_path == promoted_fast_preview.asset_path
            && selected_fast_preview.kind == promoted_fast_preview.kind
            && selected_fast_preview.visible_at_ms == promoted_fast_preview.visible_at_ms
    })
}

fn promoted_fast_preview_ready_update(
    request_id: &str,
    capture_id: &str,
    promoted_fast_preview: &FastPreviewPromotionResult,
    persisted_at_ms: u64,
) -> FastPreviewReadyUpdate {
    FastPreviewReadyUpdate {
        request_id: request_id.to_string(),
        capture_id: capture_id.to_string(),
        asset_path: promoted_fast_preview.asset_path.clone(),
        kind: promoted_fast_preview.kind.clone(),
        visible_at_ms: promoted_fast_preview
            .visible_at_ms
            .unwrap_or(persisted_at_ms),
    }
}

fn saved_fast_preview_visible_at_ms(
    selected_fast_preview: Option<&FastPreviewPromotionResult>,
    reused_early_fast_preview: Option<&FastPreviewPromotionResult>,
    seeded_fast_preview: Option<&FastPreviewPromotionResult>,
) -> Option<u64> {
    reused_early_fast_preview
        .and_then(|fast_preview| fast_preview.visible_at_ms)
        .or_else(|| selected_fast_preview.and_then(|fast_preview| fast_preview.visible_at_ms))
        .or_else(|| seeded_fast_preview.and_then(|fast_preview| fast_preview.visible_at_ms))
}

fn seed_truthful_fast_preview_asset_path(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: &str,
) -> Option<FastPreviewPromotionResult> {
    let preferred_extensions = ["jpg", "jpeg"];
    let truthful_handoff_root = paths.handoff_dir.join("fast-preview");
    let preview_path = preferred_extensions.iter().find_map(|extension| {
        let candidate_path = truthful_handoff_root.join(format!(
            "{capture_id}.{TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND}.{extension}"
        ));
        candidate_path.is_file().then_some(candidate_path)
    })?;

    promote_fast_preview_asset(
        paths,
        capture_id,
        request_id,
        None,
        &CompletedCaptureFastPreview {
            asset_path: preview_path.to_string_lossy().into_owned(),
            kind: Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into()),
        },
    )
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
    let resolved_kind = resolve_fast_preview_kind(handoff.kind.as_deref(), &candidate_path);

    let canonical_path = paths.renders_previews_dir.join(format!("{capture_id}.jpg"));
    if let Some(parent) = canonical_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            log_fast_preview_event(
                paths,
                capture_id,
                request_id,
                "fast-preview-invalid",
                resolved_kind.as_deref(),
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
                build_fast_preview_backup_path(&canonical_path, resolved_kind.as_deref());
            if let Err(error) = fs::rename(&canonical_path, &candidate_backup_path) {
                log_fast_preview_event(
                    paths,
                    capture_id,
                    request_id,
                    "fast-preview-invalid",
                    resolved_kind.as_deref(),
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
                resolved_kind.as_deref(),
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
            resolved_kind.as_deref(),
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
        resolved_kind.as_deref(),
        Some(&format!("assetPath={asset_path}")),
    );
    log_fast_preview_event(
        paths,
        capture_id,
        request_id,
        "fast-preview-visible",
        resolved_kind.as_deref(),
        Some(&format!("assetPath={asset_path}")),
    );

    Some(FastPreviewPromotionResult {
        asset_path,
        kind: resolved_kind,
        visible_at_ms: current_time_ms().ok(),
    })
}

pub(crate) fn should_start_speculative_preview_render(fast_preview_kind: Option<&str>) -> bool {
    !matches!(
        fast_preview_kind,
        Some("camera-thumbnail" | TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND)
    )
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
    if stage_speculative_preview_source(source_path, &staged_source_path).is_err() {
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

fn stage_speculative_preview_source(
    source_path: &Path,
    staged_source_path: &Path,
) -> std::io::Result<()> {
    match fs::hard_link(source_path, staged_source_path) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source_path, staged_source_path)?;
            Ok(())
        }
    }
}

fn current_time_ms() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

fn resolve_fast_preview_kind(kind: Option<&str>, candidate_path: &Path) -> Option<String> {
    kind.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            candidate_path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    value.contains(&format!(".{TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND}."))
                })
                .map(|_| TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.to_string())
        })
}

fn is_truthful_fast_preview_kind(kind: Option<&str>) -> bool {
    matches!(kind, Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND))
}

fn close_preview_truth(capture: &mut SessionCaptureRecord, preview_ready_at_ms: u64) -> bool {
    let first_truth_close = capture.timing.xmp_preview_ready_at_ms.is_none();

    capture.preview.ready_at_ms = Some(preview_ready_at_ms);
    capture.render_status = "previewReady".into();

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

    first_truth_close
}

fn truthful_fast_preview_render_detail(
    preset_id: &str,
    preset_version: &str,
    preview_ready_at_ms: u64,
    capture_acknowledged_at_ms: u64,
) -> String {
    format!(
        "presetId={};publishedVersion={};binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs={};detail=widthCap=display;heightCap=display;hq=false;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;args=none;status=ready",
        preset_id,
        preset_version,
        preview_ready_at_ms.saturating_sub(capture_acknowledged_at_ms)
    )
}

fn normalize_preset_applied_render_detail(render_detail: &str) -> String {
    let with_truth_owner = if render_detail.contains("truthOwner=") {
        render_detail.to_string()
    } else {
        format!("{render_detail};truthOwner=display-sized-preset-applied")
    };

    if with_truth_owner.contains("sourceAsset=preset-applied-preview") {
        return with_truth_owner;
    }

    if with_truth_owner.contains("sourceAsset=fast-preview-raster") {
        return with_truth_owner.replace(
            "sourceAsset=fast-preview-raster",
            "inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview",
        );
    }

    format!("{with_truth_owner};sourceAsset=preset-applied-preview")
}

fn append_capture_preview_ready_event(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    preview_ready_at_ms: u64,
) {
    let total_elapsed_ms =
        preview_ready_at_ms.saturating_sub(capture.timing.capture_acknowledged_at_ms);
    let detail = if let Some(first_visible_at_ms) = capture.timing.fast_preview_visible_at_ms {
        format!(
            "elapsedMs={};originalVisibleToPresetAppliedVisibleMs={};firstVisibleAtMs={};presetAppliedVisibleAtMs={};budgetState={};renderStatus={}",
            total_elapsed_ms,
            preview_ready_at_ms.saturating_sub(first_visible_at_ms),
            first_visible_at_ms,
            preview_ready_at_ms,
            capture.timing.preview_budget_state,
            capture.render_status
        )
    } else {
        format!(
            "elapsedMs={};originalVisibleToPresetAppliedVisibleMs=unavailable;budgetState={};renderStatus={}",
            total_elapsed_ms,
            capture.timing.preview_budget_state,
            capture.render_status
        )
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    static SPECULATIVE_WAIT_TEST_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

    #[test]
    fn truthful_saved_preview_keeps_the_earlier_first_visible_timestamp() {
        let early_first_visible = FastPreviewPromotionResult {
            asset_path: "C:/preview/first-visible.jpg".into(),
            kind: Some("windows-shell-thumbnail".into()),
            visible_at_ms: Some(1_000),
        };
        let truthful_preview = FastPreviewPromotionResult {
            asset_path: "C:/preview/preset-applied.jpg".into(),
            kind: Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into()),
            visible_at_ms: Some(1_800),
        };

        let selected_fast_preview = select_saved_fast_preview_baseline(
            Some(truthful_preview.clone()),
            Some(early_first_visible.clone()),
        )
        .expect("a saved fast preview should be selected");

        assert_eq!(
            selected_fast_preview.kind.as_deref(),
            Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND),
            "truthful saved preview should still win preview ownership"
        );
        assert_eq!(
            saved_fast_preview_visible_at_ms(
                Some(&selected_fast_preview),
                Some(&early_first_visible),
                None,
            ),
            Some(1_000),
            "first-visible timing must remain anchored to the earliest approved preview"
        );
    }

    #[test]
    fn truthful_saved_preview_is_not_downgraded_by_later_non_truthful_metadata() {
        let truthful_preview = FastPreviewPromotionResult {
            asset_path: "C:/preview/preset-applied.jpg".into(),
            kind: Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into()),
            visible_at_ms: Some(1_000),
        };
        let later_non_truthful = FastPreviewPromotionResult {
            asset_path: "C:/preview/later-camera-thumbnail.jpg".into(),
            kind: Some("camera-thumbnail".into()),
            visible_at_ms: Some(1_600),
        };

        assert!(
            !should_promoted_fast_preview_override_saved_baseline(
                &truthful_preview,
                &later_non_truthful,
            ),
            "a saved truthful preview must keep ownership even if later non-truthful metadata arrives"
        );
    }

    #[test]
    fn promoted_fast_preview_update_uses_the_resolved_promoted_kind() {
        let promoted_fast_preview = FastPreviewPromotionResult {
            asset_path: "C:/preview/capture_01.preset-applied-preview.jpg".into(),
            kind: Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND.into()),
            visible_at_ms: None,
        };

        let update = promoted_fast_preview_ready_update(
            "request_0001",
            "capture_0001",
            &promoted_fast_preview,
            1_500,
        );

        assert_eq!(update.request_id, "request_0001");
        assert_eq!(update.capture_id, "capture_0001");
        assert_eq!(
            update.kind.as_deref(),
            Some(TRUTHFUL_PRESET_APPLIED_FAST_PREVIEW_KIND),
            "emitted readiness update should inherit the resolved truthful owner"
        );
        assert_eq!(update.visible_at_ms, 1_500);
    }

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

        assert!(
            waited_for < Duration::from_millis(2400),
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

        assert!(
            waited_for < Duration::from_millis(2400),
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
}
