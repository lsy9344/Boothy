use std::{thread, time::Duration};

use tauri::{Emitter, Manager};

use crate::{
    capture::{
        ingest_pipeline::{complete_preview_render_in_dir, mark_preview_render_failed_in_dir},
        normalized_state::{
            delete_capture_in_dir, get_capture_readiness_in_dir, normalize_capture_readiness,
            request_capture_in_dir,
        },
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureDeleteResultDto, CaptureReadinessDto,
        CaptureReadinessInputDto, CaptureReadinessUpdateDto, CaptureRequestInputDto,
        CaptureRequestResultDto, HostErrorEnvelope,
    },
    session::{
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, resolve_app_session_base_dir},
    },
};

const CAPTURE_READINESS_UPDATE_EVENT: &str = "capture-readiness-update";

#[tauri::command]
pub fn get_capture_readiness(
    app: tauri::AppHandle,
    input: CaptureReadinessInputDto,
) -> Result<CaptureReadinessDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);

    get_capture_readiness_in_dir(&base_dir, input)
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
    let result = request_capture_in_dir(&base_dir, input)?;
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
            Ok(capture) => CaptureReadinessDto::preview_ready(preview_session_id.clone(), capture),
            Err(_) => {
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
    let manifest_path = SessionPaths::try_new(base_dir, session_id)
        .ok()?
        .manifest_path;
    let manifest = read_session_manifest(&manifest_path).ok()?;

    Some(normalize_capture_readiness(base_dir, &manifest))
}
