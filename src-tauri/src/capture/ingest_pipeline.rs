use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::{normalized_state::normalize_capture_readiness, CAPTURE_PIPELINE_LOCK},
    contracts::dto::{CaptureRequestInputDto, HostErrorEnvelope},
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

static CAPTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn persist_capture_in_dir(
    base_dir: &Path,
    input: &CaptureRequestInputDto,
) -> Result<(SessionManifest, SessionCaptureRecord), HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    manifest =
        sync_session_timing_in_dir(base_dir, &paths.manifest_path, manifest, SystemTime::now())?;
    let readiness = normalize_capture_readiness(base_dir, &manifest);

    if !readiness.can_capture {
        return Err(HostErrorEnvelope::capture_not_ready(
            "지금은 촬영할 수 없어요.",
            readiness,
        ));
    }

    let active_preset = manifest.active_preset.clone().ok_or_else(|| {
        HostErrorEnvelope::preset_not_available("촬영 전에 룩을 다시 골라 주세요.")
    })?;
    let capture_id = generate_capture_id("capture");
    let request_id = generate_capture_id("request");
    let raw_path = paths
        .captures_originals_dir
        .join(format!("{capture_id}.jpg"));

    fs::create_dir_all(&paths.captures_originals_dir).map_err(map_fs_error)?;
    fs::write(
        &raw_path,
        build_placeholder_asset_bytes(&manifest, &capture_id, "raw"),
    )
    .map_err(map_fs_error)?;
    let acknowledged_at_ms = current_time_ms()?;

    let capture = build_saved_capture_record(
        &manifest,
        &active_preset,
        capture_id,
        request_id,
        raw_path.to_string_lossy().into_owned(),
        acknowledged_at_ms,
    );

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
    {
        return Ok(manifest.captures[capture_index].clone());
    }

    fs::create_dir_all(&paths.renders_previews_dir).map_err(map_fs_error)?;

    let preview_path = paths.renders_previews_dir.join(format!("{capture_id}.jpg"));

    fs::write(
        &preview_path,
        build_placeholder_asset_bytes(&manifest, capture_id, "preview"),
    )
    .map_err(map_fs_error)?;

    let preview_visible_at_ms = current_time_ms()?;
    let capture = {
        let capture = manifest
            .captures
            .get_mut(capture_index)
            .expect("capture index already resolved");

        capture.preview.asset_path = Some(preview_path.to_string_lossy().into_owned());
        capture.preview.ready_at_ms = Some(preview_visible_at_ms);
        capture.render_status = "previewReady".into();
        capture.timing.preview_visible_at_ms = Some(preview_visible_at_ms);
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
    manifest.lifecycle.stage = match manifest.captures.last() {
        Some(latest_capture)
            if latest_capture.render_status == "previewWaiting"
                || latest_capture.render_status == "captureSaved" =>
        {
            "preview-waiting".into()
        }
        _ => "capture-ready".into(),
    };

    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok(capture)
}

pub fn mark_preview_render_failed_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("프리뷰 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
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

    if latest_capture.capture_id != capture_id
        || manifest.captures[capture_index]
            .preview
            .asset_path
            .is_some()
    {
        return Ok(manifest);
    }

    manifest.captures[capture_index].render_status = "renderFailed".into();
    manifest.captures[capture_index].timing.preview_budget_state = "exceededBudget".into();
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    manifest.lifecycle.stage = "phone-required".into();

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
            persisted_at_ms: acknowledged_at_ms,
        },
        preview: PreviewCaptureAsset {
            asset_path: None,
            enqueued_at_ms: Some(acknowledged_at_ms),
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
            capture_budget_ms: CAPTURE_BUDGET_MS,
            preview_budget_ms: PREVIEW_BUDGET_MS,
            preview_budget_state: "pending".into(),
        },
    }
}

fn build_placeholder_asset_bytes(
    manifest: &SessionManifest,
    capture_id: &str,
    asset_kind: &str,
) -> Vec<u8> {
    format!(
        "boothy-{asset_kind}\nsession={}\nboothAlias={}\ncapture={capture_id}\n",
        manifest.session_id, manifest.booth_alias
    )
    .into_bytes()
}

fn generate_capture_id(prefix: &str) -> String {
    let unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = CAPTURE_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let value = unix_nanos ^ (counter << 16);

    format!("{prefix}_{value:026x}")
}

fn current_time_ms() -> Result<u64, HostErrorEnvelope> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            HostErrorEnvelope::persistence("시스템 시계를 확인할 수 없어 촬영을 저장하지 못했어요.")
        })?
        .as_millis() as u64)
}

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("세션 파일을 만들지 못했어요: {error}"))
}
