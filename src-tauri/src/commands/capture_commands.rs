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
        PresetSelectionInputDto,
    },
    render::{
        prime_preview_worker_runtime_in_dir, schedule_preview_renderer_warmup_in_dir,
        wait_for_preview_renderer_warmup_to_settle,
    },
    session::session_repository::resolve_app_session_base_dir,
};

const CAPTURE_READINESS_UPDATE_EVENT: &str = "capture-readiness-update";
const CAPTURE_FAST_PREVIEW_UPDATE_EVENT: &str = "capture-fast-preview-update";
const PREVIEW_REFINEMENT_WAIT_MS: u64 = 8_000;
const PREVIEW_REFINEMENT_POLL_MS: u64 = 40;
const PREVIEW_RUNTIME_PRIME_SETTLE_TIMEOUT_MS: u64 = 20_000;

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
        let initial_capture = match complete_preview_render_in_dir(
            &preview_base_dir,
            &preview_session_id,
            &preview_capture_id,
        ) {
            Ok(capture) => {
                if let Some(preview_ready_at_ms) = capture.timing.xmp_preview_ready_at_ms {
                    let preview_elapsed_ms = preview_ready_at_ms
                        .saturating_sub(capture.timing.capture_acknowledged_at_ms);
                    let official_gate_elapsed_ms =
                        capture
                            .timing
                            .fast_preview_visible_at_ms
                            .map(|first_visible_at_ms| {
                                preview_ready_at_ms.saturating_sub(first_visible_at_ms)
                            });
                    log::info!(
                        "capture_preview_ready session={} capture_id={} elapsed_ms={} original_visible_to_preset_applied_visible_ms={} budget_state={}",
                        preview_session_id,
                        preview_capture_id,
                        preview_elapsed_ms,
                        official_gate_elapsed_ms
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unavailable".into()),
                        capture.timing.preview_budget_state
                    );
                } else {
                    let first_visible_at_ms = capture
                        .timing
                        .fast_preview_visible_at_ms
                        .unwrap_or(capture.raw.persisted_at_ms);
                    let first_visible_elapsed_ms = first_visible_at_ms
                        .saturating_sub(capture.timing.capture_acknowledged_at_ms);
                    log::info!(
                        "capture_first_visible_pending session={} capture_id={} elapsed_ms={} render_status={}",
                        preview_session_id,
                        preview_capture_id,
                        first_visible_elapsed_ms,
                        capture.render_status
                    );
                }
                capture
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
                let readiness =
                    read_current_capture_readiness(&preview_base_dir, &preview_session_id)
                        .unwrap_or_else(|| {
                            CaptureReadinessDto::phone_required(preview_session_id.clone())
                        });
                let _ = preview_app.emit(
                    CAPTURE_READINESS_UPDATE_EVENT,
                    CaptureReadinessUpdateDto::new(preview_session_id, readiness),
                );
                return;
            }
        };
        let readiness = read_current_capture_readiness(&preview_base_dir, &preview_session_id)
            .unwrap_or_else(|| {
                if initial_capture.timing.xmp_preview_ready_at_ms.is_some() {
                    CaptureReadinessDto::preview_ready(
                        preview_session_id.clone(),
                        initial_capture.clone(),
                    )
                } else {
                    CaptureReadinessDto::preview_waiting(
                        preview_session_id.clone(),
                        Some(initial_capture.clone()),
                    )
                }
            });

        let _ = preview_app.emit(
            CAPTURE_READINESS_UPDATE_EVENT,
            CaptureReadinessUpdateDto::new(preview_session_id.clone(), readiness),
        );

        if initial_capture.preview.ready_at_ms.is_none() {
            emit_refined_preview_readiness_when_available(
                &preview_app,
                &preview_base_dir,
                &preview_session_id,
                &preview_capture_id,
                initial_capture
                    .timing
                    .fast_preview_visible_at_ms
                    .unwrap_or(initial_capture.raw.persisted_at_ms),
                initial_capture.preview.ready_at_ms,
            );
        }
    });

    Ok(result)
}

#[tauri::command]
pub fn prime_preview_runtime(
    app: tauri::AppHandle,
    input: PresetSelectionInputDto,
) -> Result<(), HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);

    prime_preview_worker_runtime_in_dir(&base_dir, &input.session_id);
    schedule_preview_renderer_warmup_in_dir(
        &base_dir,
        &input.session_id,
        &input.preset_id,
        &input.published_version,
    );
    let warmup_settled = wait_for_preview_renderer_warmup_to_settle(
        &input.session_id,
        &input.preset_id,
        &input.published_version,
        Duration::from_millis(PREVIEW_RUNTIME_PRIME_SETTLE_TIMEOUT_MS),
    );
    if !warmup_settled {
        log::warn!(
            "preview_runtime_prime_wait_timed_out session={} preset_id={} published_version={} timeout_ms={}",
            input.session_id,
            input.preset_id,
            input.published_version,
            PREVIEW_RUNTIME_PRIME_SETTLE_TIMEOUT_MS
        );
    }
    log::info!(
        "preview_runtime_primed session={} preset_id={} published_version={} warmup_settled={}",
        input.session_id,
        input.preset_id,
        input.published_version,
        warmup_settled
    );

    Ok(())
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

fn emit_refined_preview_readiness_when_available(
    app: &tauri::AppHandle,
    base_dir: &std::path::Path,
    session_id: &str,
    capture_id: &str,
    first_visible_at_ms: u64,
    baseline_ready_at_ms: Option<u64>,
) {
    let wait_cycles = (PREVIEW_REFINEMENT_WAIT_MS / PREVIEW_REFINEMENT_POLL_MS).max(1);

    for _ in 0..=wait_cycles {
        let Some(readiness) = read_current_capture_readiness(base_dir, session_id) else {
            thread::sleep(Duration::from_millis(PREVIEW_REFINEMENT_POLL_MS));
            continue;
        };

        let refinement_ready = readiness
            .latest_capture
            .as_ref()
            .filter(|capture| capture.capture_id == capture_id);
        let refined_ready_at_ms = refinement_ready
            .and_then(|capture| capture.preview.ready_at_ms)
            .filter(|ready_at_ms| {
                baseline_ready_at_ms.map_or(*ready_at_ms >= first_visible_at_ms, |baseline| {
                    *ready_at_ms > baseline
                })
            });

        if let Some(refined_ready_at_ms) = refined_ready_at_ms {
            let waited_from_ms = baseline_ready_at_ms.unwrap_or(first_visible_at_ms);
            let refined_elapsed_ms = refined_ready_at_ms.saturating_sub(waited_from_ms);
            log::info!(
                "capture_preview_refinement_ready session={} capture_id={} refinement_elapsed_ms={}",
                session_id,
                capture_id,
                refined_elapsed_ms
            );
            let _ = app.emit(
                CAPTURE_READINESS_UPDATE_EVENT,
                CaptureReadinessUpdateDto::new(session_id.to_string(), readiness),
            );
            return;
        }

        thread::sleep(Duration::from_millis(PREVIEW_REFINEMENT_POLL_MS));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refinement_wait_budget_covers_idle_and_queue_window() {
        assert!(
            PREVIEW_REFINEMENT_WAIT_MS >= 8_000,
            "refinement watcher should outlive the render idle and queue wait windows"
        );
    }
}
