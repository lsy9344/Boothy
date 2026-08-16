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
    viewer::readiness_gate::{apply_viewer_gate, CAPTURE_REASON_VIEWER_PREPARING},
};

const CAPTURE_READINESS_UPDATE_EVENT: &str = "capture-readiness-update";
const CAPTURE_FAST_PREVIEW_UPDATE_EVENT: &str = "capture-fast-preview-update";
const PREVIEW_REFINEMENT_WAIT_MS: u64 = 2000;
const PREVIEW_REFINEMENT_POLL_MS: u64 = 40;

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

    // Story 7.1: 관람 창이 사라졌다면 여기서 복구한다. booth가 readiness를 주기적으로 조회하므로
    // 창을 잃어도 자동으로 다시 열리고, 그 전까지 촬영은 `viewer-preparing`으로 정직하게 막힌다.
    // 이 호출은 main thread에서도 안전하다. 생성은 background 스레드로 넘어가고 즉시 반환된다.
    crate::commands::viewer_commands::ensure_viewer_window_state(&app);

    match get_capture_readiness_in_dir(&base_dir, input)
        .map(|readiness| gate_readiness_on_viewer(&app, readiness, &session_id))
    {
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
    // Story 7.2: 삭제 뒤에는 manifest에서 request 상관관계를 알 수 없으므로 먼저 읽어 둔다.
    let deleted_request_id =
        find_capture_request_id(&base_dir, &input.session_id, &input.capture_id);
    let session_id = input.session_id.clone();

    let result = delete_capture_in_dir(&base_dir, input)?;

    if let Some(request_id) = deleted_request_id {
        crate::commands::display_commands::forget_display_request(&app, &session_id, &request_id)?;
    }

    Ok(result)
}

/// 삭제 대상 capture의 `requestId`를 미리 찾는다. 표시 generation은 request 단위로 보관된다.
fn find_capture_request_id(
    base_dir: &std::path::Path,
    session_id: &str,
    capture_id: &str,
) -> Option<String> {
    let paths = crate::session::session_paths::SessionPaths::try_new(base_dir, session_id).ok()?;

    crate::session::session_repository::read_session_manifest(&paths.manifest_path)
        .ok()?
        .captures
        .iter()
        .find(|capture| capture.capture_id == capture_id)
        .map(|capture| capture.request_id.clone())
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

    // Story 7.1: 관람 창이 현재 세션에 준비되기 전에는 촬영을 시작하지 않는다.
    let readiness =
        read_current_capture_readiness(&app, &base_dir, &session_id).ok_or_else(|| {
            log::warn!("capture_blocked_readiness_unavailable session={session_id}");
            HostErrorEnvelope::capture_not_ready(
                "촬영 준비 상태를 다시 확인하고 있어요.",
                CaptureReadinessDto::viewer_preparing(session_id.clone()),
            )
        })?;

    if !readiness.can_capture && readiness.reason_code == CAPTURE_REASON_VIEWER_PREPARING {
        log::warn!("capture_blocked_viewer_not_ready session={session_id}");

        return Err(HostErrorEnvelope::capture_not_ready(
            "화면을 준비하고 있어요.",
            readiness,
        ));
    }

    // Story 7.4: **촬영 수락 시점의** viewer 세대를 여기서 읽는다. RAW가 도착한 뒤에 읽으면
    // 그 사이 관람 창이 재생성돼도 알아채지 못해, 죽은 세대의 자산이 화면에 오른다.
    let request_viewer_epoch =
        crate::commands::display_commands::current_viewer_epoch_for_request(&app);

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
    // Story 7.2: 계측 lane. 기본은 off이며 켜져 있을 때만 request-scoped sample generation을
    // 게시한다. 전부 background 스레드에서 수행하므로 이 command의 반환을 지연시키지 않는다.
    crate::commands::display_commands::spawn_sample_lane_publication(
        &app,
        result.session_id.clone(),
        result.capture.request_id.clone(),
        Some(result.capture.capture_id.clone()),
    );

    // Story 7.4: display-fit preset proxy lane. HV-15 Go 이후 기본은 on이며, RAW와 preset 결속이 모두
    // 확정된 지금이 유일하게 정확한 착수 지점이다. 전부 background 스레드에서 수행하므로
    // 이 command의 반환을 지연시키지 않는다.
    crate::commands::display_commands::spawn_display_proxy_publication(
        &app,
        result.session_id.clone(),
        result.capture.request_id.clone(),
        result.capture.capture_id.clone(),
        result.capture.active_preset_id.clone(),
        result.capture.active_preset_version.clone(),
        std::path::PathBuf::from(&result.capture.raw.asset_path),
        request_viewer_epoch,
    );

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
                    log::info!(
                        "capture_preview_ready session={} capture_id={} elapsed_ms={} budget_state={}",
                        preview_session_id,
                        preview_capture_id,
                        preview_elapsed_ms,
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
                let readiness = read_current_capture_readiness(
                    &preview_app,
                    &preview_base_dir,
                    &preview_session_id,
                )
                .unwrap_or_else(|| CaptureReadinessDto::phone_required(preview_session_id.clone()));
                let _ = preview_app.emit(
                    CAPTURE_READINESS_UPDATE_EVENT,
                    CaptureReadinessUpdateDto::new(preview_session_id, readiness),
                );
                return;
            }
        };
        let readiness =
            read_current_capture_readiness(&preview_app, &preview_base_dir, &preview_session_id)
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

        if initial_capture.timing.xmp_preview_ready_at_ms.is_none() {
            emit_refined_preview_readiness_when_available(
                &preview_app,
                &preview_base_dir,
                &preview_session_id,
                &preview_capture_id,
                initial_capture
                    .timing
                    .fast_preview_visible_at_ms
                    .unwrap_or(initial_capture.raw.persisted_at_ms),
            );
        }
    });

    Ok(result)
}

/// Story 7.1: viewer가 준비되지 않았으면 촬영 가능 상태를 내린다 (downgrade 전용).
fn gate_readiness_on_viewer(
    app: &tauri::AppHandle,
    readiness: CaptureReadinessDto,
    session_id: &str,
) -> CaptureReadinessDto {
    let snapshot = {
        let handle = app.state::<crate::viewer::ViewerStateHandle>();
        let state = handle.0.lock().expect("viewer state lock poisoned");

        state.snapshot_with_clock(
            crate::viewer::current_epoch_ms(),
            crate::viewer::current_monotonic_ms(),
        )
    };

    apply_viewer_gate(readiness, &snapshot, session_id)
}

fn read_current_capture_readiness(
    app: &tauri::AppHandle,
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
    .map(|readiness| gate_readiness_on_viewer(app, readiness, session_id))
}

fn emit_refined_preview_readiness_when_available(
    app: &tauri::AppHandle,
    base_dir: &std::path::Path,
    session_id: &str,
    capture_id: &str,
    first_visible_at_ms: u64,
) {
    let wait_cycles = (PREVIEW_REFINEMENT_WAIT_MS / PREVIEW_REFINEMENT_POLL_MS).max(1);

    for _ in 0..=wait_cycles {
        let Some(readiness) = read_current_capture_readiness(app, base_dir, session_id) else {
            thread::sleep(Duration::from_millis(PREVIEW_REFINEMENT_POLL_MS));
            continue;
        };

        let refinement_ready = readiness
            .latest_capture
            .as_ref()
            .filter(|capture| capture.capture_id == capture_id)
            .and_then(|capture| capture.timing.xmp_preview_ready_at_ms)
            .filter(|ready_at_ms| *ready_at_ms >= first_visible_at_ms);

        if let Some(refined_ready_at_ms) = refinement_ready {
            let refined_elapsed_ms = refined_ready_at_ms.saturating_sub(first_visible_at_ms);
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
