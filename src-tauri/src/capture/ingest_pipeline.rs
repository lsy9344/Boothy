use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::{sidecar_client::CompletedCaptureFastPreview, CAPTURE_PIPELINE_LOCK},
    contracts::dto::{CaptureRequestInputDto, HostErrorEnvelope},
    render::{
        is_valid_render_preview_asset, log_render_failure_in_dir, log_render_start_in_dir,
        render_capture_asset_in_dir, RenderIntent,
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
    timing::sync_session_timing_in_dir,
};

const FAST_PREVIEW_ALLOWED_EXTENSIONS: [&str; 2] = ["jpg", "jpeg"];

struct FastPreviewPromotionResult {
    asset_path: String,
    visible_at_ms: Option<u64>,
}

pub fn persist_capture_in_dir(
    base_dir: &Path,
    input: &CaptureRequestInputDto,
    capture_id: String,
    request_id: String,
    raw_asset_path: String,
    fast_preview: Option<CompletedCaptureFastPreview>,
    acknowledged_at_ms: u64,
    persisted_at_ms: u64,
) -> Result<(SessionManifest, SessionCaptureRecord), HostErrorEnvelope> {
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
    if let Some(promoted_fast_preview) = fast_preview.as_ref().and_then(|handoff| {
        promote_fast_preview_asset(
            &paths,
            &capture.capture_id,
            &capture.raw.asset_path,
            handoff,
        )
    }) {
        capture.preview.asset_path = Some(promoted_fast_preview.asset_path);
        capture.timing.fast_preview_visible_at_ms = promoted_fast_preview.visible_at_ms;
    } else if fast_preview.is_none() {
        if let Some(seed_result) = seed_pending_preview_asset_path(&paths, &capture.capture_id) {
            capture.preview.asset_path = Some(seed_result.asset_path);
            capture.timing.fast_preview_visible_at_ms = seed_result.visible_at_ms;
        }
    }

    manifest.captures.push(capture.clone());
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = "preview-waiting".into();

    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok((manifest, capture))
}

pub fn complete_preview_render_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
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

    let capture_snapshot = manifest.captures[capture_index].clone();
    log_render_start_in_dir(base_dir, session_id, capture_id, RenderIntent::Preview);
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
                RenderIntent::Preview,
                error.reason_code,
            );
            return Err(HostErrorEnvelope::persistence(error.customer_message));
        }
    };

    let preview_visible_at_ms = rendered_preview.ready_at_ms;
    let capture = {
        let capture = manifest
            .captures
            .get_mut(capture_index)
            .expect("capture index already resolved");

        capture.preview.asset_path = Some(rendered_preview.asset_path);
        capture.preview.ready_at_ms = Some(preview_visible_at_ms);
        capture.render_status = "previewReady".into();
        capture.timing.preview_visible_at_ms = Some(preview_visible_at_ms);
        capture.timing.xmp_preview_ready_at_ms = Some(preview_visible_at_ms);
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
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(&manifest);
    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok(capture)
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
    log_render_start_in_dir(base_dir, session_id, capture_id, RenderIntent::Final);
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
    raw_asset_path: &str,
    handoff: &CompletedCaptureFastPreview,
) -> Option<FastPreviewPromotionResult> {
    log_fast_preview_event(
        paths,
        capture_id,
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
        "fast-preview-promoted",
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
    raw_asset_path: &str,
    candidate_asset_path: &str,
) -> Result<PathBuf, &'static str> {
    let candidate_path = PathBuf::from(candidate_asset_path);
    if !candidate_path.is_absolute() {
        return Err("not-absolute");
    }

    if !is_session_scoped_asset_path(paths, &candidate_path) {
        return Err("unscoped");
    }

    let extension = candidate_path
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

    let stem = candidate_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("missing-filename")?;
    let normalized_candidate = normalize_path(&candidate_path);
    let handoff_fast_preview_root = normalize_path(&paths.handoff_dir.join("fast-preview"))
        .trim_end_matches('/')
        .to_string();
    let canonical_preview_root = normalize_path(&paths.renders_previews_dir)
        .trim_end_matches('/')
        .to_string();
    let in_handoff_root =
        normalized_candidate.starts_with(&(handoff_fast_preview_root.clone() + "/"));
    let in_canonical_preview_root =
        normalized_candidate.starts_with(&(canonical_preview_root.clone() + "/"));

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

    let preview_metadata = fs::metadata(&candidate_path).map_err(|_| "missing")?;
    if !preview_metadata.is_file() {
        return Err("missing");
    }
    if preview_metadata.len() == 0 {
        return Err("empty");
    }
    if !is_valid_render_preview_asset(&candidate_path) {
        return Err("invalid-raster");
    }

    let raw_metadata = fs::metadata(raw_asset_path).map_err(|_| "raw-missing")?;
    let preview_modified = preview_metadata.modified().ok().and_then(system_time_to_ms);
    let raw_modified = raw_metadata.modified().ok().and_then(system_time_to_ms);
    if let (Some(preview_modified), Some(raw_modified)) = (preview_modified, raw_modified) {
        if preview_modified < raw_modified {
            return Err("stale");
        }
    }

    Ok(candidate_path)
}

fn is_session_scoped_asset_path(paths: &SessionPaths, candidate_path: &Path) -> bool {
    let normalized_candidate = normalize_path(candidate_path);
    let normalized_session_root = format!(
        "{}/",
        paths
            .session_root
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase()
    );

    if normalized_candidate.starts_with("//") {
        return false;
    }

    if normalized_candidate
        .split('/')
        .any(|segment| segment == "..")
    {
        return false;
    }

    normalized_candidate.starts_with(&normalized_session_root)
}

fn log_fast_preview_event(
    paths: &SessionPaths,
    capture_id: &str,
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
        "{occurred_at}\tsession={}\tcapture={capture_id}\tevent={event}\tkind={kind}\tdetail={detail}",
        paths
            .session_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    );
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
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

fn current_time_ms() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

fn system_time_to_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}
