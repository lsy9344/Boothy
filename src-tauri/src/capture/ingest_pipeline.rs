use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::CAPTURE_PIPELINE_LOCK,
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

const SIDECAR_PREVIEW_DISCOVERY_TIMEOUT_MS: u64 = 900;
const SIDECAR_PREVIEW_DISCOVERY_POLL_INTERVAL_MS: u64 = 75;

pub fn persist_capture_in_dir(
    base_dir: &Path,
    input: &CaptureRequestInputDto,
    capture_id: String,
    request_id: String,
    raw_asset_path: String,
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

    let capture = build_saved_capture_record(
        &manifest,
        &active_preset,
        capture_id,
        request_id,
        raw_asset_path,
        acknowledged_at_ms,
        persisted_at_ms,
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

    let preview_path = materialize_preview_asset(&paths, &manifest.captures[capture_index])
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
            capture_budget_ms: CAPTURE_BUDGET_MS,
            preview_budget_ms: PREVIEW_BUDGET_MS,
            preview_budget_state: "pending".into(),
        },
    }
}

fn materialize_preview_asset(
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
) -> Result<PathBuf, std::io::Error> {
    let raw_path = Path::new(&capture.raw.asset_path);
    let raw_extension = raw_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());

    let preview_extension = match raw_extension.as_deref() {
        Some("jpg" | "jpeg") => "jpg",
        Some("png") => "png",
        Some("webp") => "webp",
        Some("gif") => "gif",
        Some("bmp") => "bmp",
        Some("svg") => "svg",
        _ => "svg",
    };
    let preview_path = paths
        .renders_previews_dir
        .join(format!("{}.{}", capture.capture_id, preview_extension));

    if matches!(
        raw_extension.as_deref(),
        Some("jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "svg")
    ) {
        fs::copy(raw_path, &preview_path)?;
    } else if let Some(existing_preview_path) =
        find_existing_sidecar_preview_asset(paths, &capture.capture_id)
    {
        return Ok(existing_preview_path);
    } else if let Some(delayed_preview_path) =
        wait_for_existing_sidecar_preview_asset(paths, &capture.capture_id)
    {
        return Ok(delayed_preview_path);
    } else {
        fs::write(
            &preview_path,
            build_preview_fallback_svg_bytes(&capture.capture_id),
        )?;
    }

    Ok(preview_path)
}

fn find_existing_sidecar_preview_asset(paths: &SessionPaths, capture_id: &str) -> Option<PathBuf> {
    ["jpg", "jpeg", "png", "webp", "gif", "bmp", "svg"]
        .iter()
        .map(|extension| {
            paths
                .renders_previews_dir
                .join(format!("{capture_id}.{extension}"))
        })
        .find(|path| path.is_file())
}

fn wait_for_existing_sidecar_preview_asset(
    paths: &SessionPaths,
    capture_id: &str,
) -> Option<PathBuf> {
    let poll_interval = Duration::from_millis(SIDECAR_PREVIEW_DISCOVERY_POLL_INTERVAL_MS);
    let max_attempts = SIDECAR_PREVIEW_DISCOVERY_TIMEOUT_MS
        .div_ceil(SIDECAR_PREVIEW_DISCOVERY_POLL_INTERVAL_MS)
        .max(1);

    for _ in 0..max_attempts {
        if let Some(preview_path) = find_existing_sidecar_preview_asset(paths, capture_id) {
            return Some(preview_path);
        }

        thread::sleep(poll_interval);
    }

    find_existing_sidecar_preview_asset(paths, capture_id)
}

fn build_preview_fallback_svg_bytes(capture_id: &str) -> Vec<u8> {
    format!(
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800" viewBox="0 0 1200 800">"##,
            r##"<rect width="1200" height="800" fill="#101820"/>"##,
            r##"<rect x="80" y="80" width="1040" height="640" rx="28" fill="#1f3140" stroke="#89b8d8" stroke-width="4"/>"##,
            r##"<text x="600" y="360" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="52" fill="#f6fbff">"##,
            r#"Preview unavailable"#,
            r#"</text>"#,
            r##"<text x="600" y="430" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="28" fill="#c7d9e6">"##,
            r#"capture: {capture_id}"#,
            r#"</text>"#,
            r#"</svg>"#,
        ),
        capture_id = capture_id,
    )
    .into_bytes()
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
