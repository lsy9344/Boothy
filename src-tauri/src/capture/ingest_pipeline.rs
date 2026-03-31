use std::{path::Path, time::SystemTime};

use crate::{
    capture::CAPTURE_PIPELINE_LOCK,
    contracts::dto::{CaptureRequestInputDto, HostErrorEnvelope},
    render::{log_render_failure_in_dir, render_capture_asset_in_dir, RenderIntent},
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
        && matches!(
            manifest.captures[capture_index].render_status.as_str(),
            "previewReady" | "finalReady"
        )
    {
        return Ok(manifest.captures[capture_index].clone());
    }

    let capture_snapshot = manifest.captures[capture_index].clone();
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
