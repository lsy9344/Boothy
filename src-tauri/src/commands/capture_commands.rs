use std::{thread, time::Duration};

use tauri::{Emitter, Manager};

use crate::{
    capture::{
        helper_supervisor::try_ensure_helper_running,
        ingest_pipeline::{complete_preview_render_in_dir, mark_preview_render_failed_in_dir},
        normalized_state::{
            delete_capture_in_dir, get_capture_readiness_in_dir,
            request_capture_in_dir_with_fast_preview,
        },
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureDeleteResultDto, CaptureFastPreviewUpdateDto,
        CaptureReadinessDto, CaptureReadinessInputDto, CaptureReadinessUpdateDto,
        CaptureRequestInputDto, CaptureRequestResultDto, HostErrorEnvelope,
    },
    session::session_repository::resolve_app_session_base_dir,
};

const CAPTURE_READINESS_UPDATE_EVENT: &str = "capture-readiness-update";
const CAPTURE_FAST_PREVIEW_UPDATE_EVENT: &str = "capture-fast-preview-update";

#[tauri::command]
pub fn get_capture_readiness(
    app: tauri::AppHandle,
    input: CaptureReadinessInputDto,
) -> Result<CaptureReadinessDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    try_ensure_helper_running(&base_dir, &input.session_id);
    let session_id = input.session_id.clone();

    match get_capture_readiness_in_dir(&base_dir, input) {
        Ok(readiness) => {
            let live_truth_summary = readiness
                .live_capture_truth
                .as_ref()
                .map(|truth| {
                    format!(
                        "{}:{}:{}:{}",
                        truth.freshness,
                        truth.session_match,
                        truth.camera_state,
                        truth.helper_state
                    )
                })
                .unwrap_or_else(|| "none".into());
            log::info!(
                "capture_readiness session={} customer_state={} reason_code={} can_capture={} live_truth={}",
                session_id,
                readiness.customer_state,
                readiness.reason_code,
                readiness.can_capture,
                live_truth_summary
            );
            Ok(readiness)
        }
        Err(error) => {
            log::warn!(
                "capture_readiness_failed session={} code={} message={}",
                session_id,
                error.code,
                error.message
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub fn delete_capture(
    app: tauri::AppHandle,
    input: CaptureDeleteInputDto,
) -> Result<CaptureDeleteResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);

    delete_capture_in_dir(&base_dir, input)
}

#[tauri::command]
pub fn request_capture(
    app: tauri::AppHandle,
    input: CaptureRequestInputDto,
) -> Result<CaptureRequestResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    try_ensure_helper_running(&base_dir, &input.session_id);
    let session_id = input.session_id.clone();
    let preview_session_id = session_id.clone();
    let preview_app = app.clone();
    let result = match request_capture_in_dir_with_fast_preview(&base_dir, input, move |update| {
        let _ = preview_app.emit(
            CAPTURE_FAST_PREVIEW_UPDATE_EVENT,
            CaptureFastPreviewUpdateDto::new(
                preview_session_id.clone(),
                update.request_id,
                update.capture_id,
                update.asset_path,
                update.visible_at_ms,
                update.kind,
            ),
        );
    }) {
        Ok(result) => {
            log::info!(
                "capture_request_saved session={} capture_id={} request_id={} readiness={}",
                session_id,
                result.capture.capture_id,
                result.capture.request_id,
                result.readiness.reason_code
            );
            result
        }
        Err(error) => {
            log::warn!(
                "capture_request_failed session={} code={} message={}",
                session_id,
                error.code,
                error.message
            );
            return Err(error);
        }
    };
    let preview_base_dir = base_dir.clone();
    let preview_session_id = result.session_id.clone();
    let preview_capture_id = result.capture.capture_id.clone();
    let preview_app = app.clone();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(120));
        let readiness = match complete_preview_render_in_dir(
            &preview_base_dir,
            &preview_session_id,
            &preview_capture_id,
        ) {
            Ok(capture) => {
                let preview_elapsed_ms = capture
                    .timing
                    .preview_visible_at_ms
                    .unwrap_or(capture.raw.persisted_at_ms)
                    .saturating_sub(capture.timing.capture_acknowledged_at_ms);
                log::info!(
                    "capture_preview_ready session={} capture_id={} elapsed_ms={} budget_state={}",
                    preview_session_id,
                    preview_capture_id,
                    preview_elapsed_ms,
                    capture.timing.preview_budget_state
                );
                read_current_capture_readiness(&preview_base_dir, &preview_session_id)
                    .unwrap_or_else(|| {
                        CaptureReadinessDto::preview_ready(preview_session_id.clone(), capture)
                    })
            }
            Err(_) => {
                log::warn!(
                    "capture_preview_failed session={} capture_id={}",
                    preview_session_id,
                    preview_capture_id
                );
                let _ = mark_preview_render_failed_in_dir(
                    &preview_base_dir,
                    &preview_session_id,
                    &preview_capture_id,
                );
                read_current_capture_readiness(&preview_base_dir, &preview_session_id)
                    .unwrap_or_else(|| {
                        CaptureReadinessDto::phone_required(preview_session_id.clone())
                    })
            }
        };

        let _ = preview_app.emit(
            CAPTURE_READINESS_UPDATE_EVENT,
            CaptureReadinessUpdateDto::new(preview_session_id, readiness),
        );
    });

    Ok(result)
}

fn read_current_capture_readiness(
    base_dir: &std::path::Path,
    session_id: &str,
) -> Option<CaptureReadinessDto> {
    get_capture_readiness_in_dir(
        base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.into(),
        },
    )
    .ok()
}
