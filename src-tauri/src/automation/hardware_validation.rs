use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::json;

use crate::{
    capture::{
        helper_supervisor::try_ensure_helper_running,
        ingest_pipeline::{complete_preview_render_in_dir, mark_final_render_failed_in_dir},
        normalized_state::{get_capture_readiness_in_dir, request_capture_in_dir},
        sidecar_client::{
            read_latest_helper_error_message, read_latest_status_message, CanonHelperErrorMessage,
            CanonHelperStatusMessage, CAMERA_HELPER_EVENTS_FILE_NAME,
            CAMERA_HELPER_REQUESTS_FILE_NAME, CAMERA_HELPER_STATUS_FILE_NAME,
        },
    },
    contracts::dto::{
        CapabilitySnapshotDto, CaptureReadinessDto, CaptureReadinessInputDto,
        CaptureRequestInputDto, HostErrorEnvelope, LoadPresetCatalogInputDto,
        OperatorSessionSummaryDto, PresetSelectionInputDto, PublishedPresetSummaryDto,
        SessionStartInputDto,
    },
    diagnostics::{
        audit_log::load_operator_audit_history_in_dir, load_operator_session_summary_in_dir,
    },
    preset::preset_catalog::load_preset_catalog_in_dir,
    render::{schedule_preview_renderer_warmup_in_dir, wait_for_preview_renderer_warmup_to_settle},
    session::{
        session_manifest::{current_timestamp, normalize_customer_name, SessionCaptureRecord},
        session_paths::SessionPaths,
        session_repository::{
            read_session_manifest, resolve_app_session_base_dir, select_active_preset_in_dir,
            start_session_in_dir,
        },
    },
};

const PREVIEW_RUNTIME_WARMUP_SETTLE_TIMEOUT_MS: u64 = 20_000;
const HOST_OWNED_RESERVE_INPUT_SETTLE_TIMEOUT_MS: u64 = 3_000;
const HOST_OWNED_RESERVE_INPUT_POLL_INTERVAL_MS: u64 = 50;
const POST_END_WAIT_POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLaunchMode {
    Skip,
    LaunchSiblingExe,
}

#[derive(Debug, Clone)]
pub struct HardwareValidationRunInput {
    pub prompt: String,
    pub preset_query: String,
    pub capture_count: u32,
    pub app_launch_mode: AppLaunchMode,
    pub phone_last_four: Option<String>,
    pub post_end_wait_timeout_ms: Option<u64>,
    pub validate_render_failure_isolation: bool,
}

#[derive(Debug, Clone)]
pub struct HardwareValidationRunResult {
    pub status: String,
    pub capture_count: u32,
    pub run_dir: PathBuf,
    pub summary_path: PathBuf,
    pub steps_path: PathBuf,
    pub artifacts_index_path: PathBuf,
    pub failure_report_path: Option<PathBuf>,
    pub session_id: Option<String>,
}

#[derive(Debug)]
pub struct HardwareValidationRunnerError {
    message: String,
}

impl HardwareValidationRunnerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HardwareValidationRunnerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HardwareValidationRunnerError {}

impl From<std::io::Error> for HardwareValidationRunnerError {
    fn from(value: std::io::Error) -> Self {
        Self::new(format!("hardware validation I/O failed: {value}"))
    }
}

impl From<serde_json::Error> for HardwareValidationRunnerError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(format!("hardware validation JSON failed: {value}"))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailureDiagnostic {
    code: String,
    problem: String,
    suspected_cause: String,
    debug_hints: Vec<String>,
}

#[derive(Debug, Clone)]
struct RunFailure {
    diagnostic: FailureDiagnostic,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedPresetRecord {
    preset_id: String,
    display_name: String,
    published_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunSummary {
    schema_version: &'static str,
    status: String,
    prompt: String,
    session_id: Option<String>,
    booth_alias: Option<String>,
    captures_requested: u32,
    captures_passed: u32,
    preset: Option<ResolvedPresetRecord>,
    app_launch_mode: String,
    app_launched: bool,
    started_at: String,
    completed_at: String,
    failure: Option<FailureDiagnostic>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StepLogEntry {
    schema_version: &'static str,
    occurred_at: String,
    event_type: String,
    status: String,
    session_id: Option<String>,
    capture_index: Option<u32>,
    capture_id: Option<String>,
    request_id: Option<String>,
    message: String,
    detail: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactIndex {
    schema_version: &'static str,
    run_dir: String,
    session_id: Option<String>,
    session_manifest_path: Option<String>,
    captures_originals_dir: Option<String>,
    renders_previews_dir: Option<String>,
    diagnostics_dir: Option<String>,
    timing_events_log_path: Option<String>,
    operator_audit_log_path: Option<String>,
    helper_status_path: Option<String>,
    helper_startup_log_path: Option<String>,
    helper_requests_path: Option<String>,
    helper_events_path: Option<String>,
    failure_diagnostics_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailureDiagnosticsSnapshot {
    schema_version: &'static str,
    session_id: String,
    collected_at: String,
    last_readiness: Option<CaptureReadinessDto>,
    readiness_error: Option<String>,
    helper_status: Option<CanonHelperStatusMessage>,
    helper_status_error: Option<String>,
    helper_error: Option<CanonHelperErrorMessage>,
    helper_error_read_error: Option<String>,
    operator_session_summary: Option<OperatorSessionSummaryDto>,
    operator_session_summary_error: Option<String>,
    startup_log_tail: Vec<String>,
    helper_requests_tail: Vec<String>,
    helper_events_tail: Vec<String>,
    timing_events_tail: Vec<String>,
}

struct RunContext {
    run_dir: PathBuf,
    summary_path: PathBuf,
    steps_path: PathBuf,
    artifacts_index_path: PathBuf,
    failure_report_path: PathBuf,
    failure_diagnostics_path: PathBuf,
    started_at: String,
    prompt: String,
    preset_query: String,
    requested_capture_count: u32,
    app_launch_mode: String,
    app_launched: bool,
    captures_passed: u32,
    first_capture_id: Option<String>,
    latest_capture_id: Option<String>,
    session_id: Option<String>,
    booth_alias: Option<String>,
    preset: Option<ResolvedPresetRecord>,
}

impl RunContext {
    fn new(
        output_root: &Path,
        prompt: &str,
        preset_query: &str,
        requested_capture_count: u32,
        app_launch_mode: &AppLaunchMode,
    ) -> Result<Self, HardwareValidationRunnerError> {
        fs::create_dir_all(output_root)?;
        let started_at = current_timestamp(SystemTime::now())
            .map_err(|error| HardwareValidationRunnerError::new(error.message))?;
        let run_stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let run_dir = output_root.join(format!("hardware-validation-run-{run_stamp}"));
        fs::create_dir_all(&run_dir)?;

        Ok(Self {
            summary_path: run_dir.join("run-summary.json"),
            steps_path: run_dir.join("run-steps.jsonl"),
            artifacts_index_path: run_dir.join("artifacts-index.json"),
            failure_report_path: run_dir.join("failure-report.md"),
            failure_diagnostics_path: run_dir.join("failure-diagnostics.json"),
            run_dir,
            started_at,
            prompt: prompt.to_string(),
            preset_query: preset_query.to_string(),
            requested_capture_count,
            app_launch_mode: match app_launch_mode {
                AppLaunchMode::Skip => "skip".into(),
                AppLaunchMode::LaunchSiblingExe => "launch-sibling-exe".into(),
            },
            app_launched: false,
            captures_passed: 0,
            first_capture_id: None,
            latest_capture_id: None,
            session_id: None,
            booth_alias: None,
            preset: None,
        })
    }

    fn append_step(
        &self,
        event_type: &str,
        status: &str,
        message: &str,
        capture_index: Option<u32>,
        capture_id: Option<&str>,
        request_id: Option<&str>,
        detail: serde_json::Value,
    ) -> Result<(), HardwareValidationRunnerError> {
        let occurred_at = current_timestamp(SystemTime::now())
            .map_err(|error| HardwareValidationRunnerError::new(error.message))?;
        let entry = StepLogEntry {
            schema_version: "hardware-validation-step/v1",
            occurred_at,
            event_type: event_type.into(),
            status: status.into(),
            session_id: self.session_id.clone(),
            capture_index,
            capture_id: capture_id.map(str::to_string),
            request_id: request_id.map(str::to_string),
            message: message.into(),
            detail,
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.steps_path)?;
        let line = serde_json::to_string(&entry)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;

        Ok(())
    }
}

pub fn run_hardware_validation_in_dir(
    base_dir: &Path,
    output_root: &Path,
    input: HardwareValidationRunInput,
) -> Result<HardwareValidationRunResult, HardwareValidationRunnerError> {
    let mut context = RunContext::new(
        output_root,
        &input.prompt,
        &input.preset_query,
        input.capture_count.max(1),
        &input.app_launch_mode,
    )?;
    context.append_step(
        "run-started",
        "running",
        "Hardware validation run started.",
        None,
        None,
        None,
        json!({
            "prompt": input.prompt,
            "presetQuery": input.preset_query,
            "captureCount": input.capture_count.max(1),
            "validateRenderFailureIsolation": input.validate_render_failure_isolation,
        }),
    )?;

    let mut launched_process =
        launch_app_if_needed(base_dir, &input.app_launch_mode, &mut context)?;
    let run_outcome = execute_validation_run(base_dir, &mut context, &input);
    if let Some(child) = launched_process.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }

    finalize_run(base_dir, context, run_outcome)
}

fn execute_validation_run(
    base_dir: &Path,
    context: &mut RunContext,
    input: &HardwareValidationRunInput,
) -> Result<(), RunFailure> {
    let session_input = resolve_session_start_input(&input.prompt, input.phone_last_four.clone());
    let session = start_session_in_dir(base_dir, session_input.clone()).map_err(|error| {
        run_failure(
            "session-start-failed",
            format!("세션을 시작하지 못했어요: {}", error.message),
            "세션 입력이 유효하지 않거나 세션 루트를 만들지 못했습니다.",
            vec![
                "run-summary.json의 failure 필드와 step log를 먼저 확인하세요.",
                "session root 권한과 prompt 파싱 결과를 점검하세요.",
            ],
        )
    })?;
    context.session_id = Some(session.session_id.clone());
    context.booth_alias = Some(session.booth_alias.clone());
    context
        .append_step(
            "session-started",
            "passed",
            "Validation session started.",
            None,
            None,
            None,
            json!({
                "sessionId": session.session_id,
                "boothAlias": session.booth_alias,
                "name": session_input.name,
                "phoneLastFour": session_input.phone_last_four,
            }),
        )
        .map_err(internal_run_failure)?;

    let preset_catalog = load_preset_catalog_in_dir(
        base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .map_err(|error| {
        run_failure(
            "preset-catalog-unavailable",
            format!("프리셋 카탈로그를 읽지 못했어요: {}", error.message),
            "세션이 고정된 catalog snapshot을 만들지 못했거나 published bundle을 읽지 못했습니다.",
            vec![
                "preset-catalog/published 아래 bundle.json 존재 여부를 확인하세요.",
                "세션 manifest의 catalogSnapshot이 비어 있지 않은지 확인하세요.",
            ],
        )
    })?;
    let preset = resolve_requested_preset(&preset_catalog.presets, &input.preset_query)
        .ok_or_else(|| {
            run_failure(
                "preset-not-found",
                format!(
                    "요청한 프리셋 `{}`을 현재 세션 카탈로그에서 찾지 못했어요.",
                    input.preset_query
                ),
                "look2가 현재 published catalog snapshot에 없거나 이름/식별자가 다릅니다.",
                vec![
                    "현재 세션 catalogSnapshot과 preset-catalog/published 내용을 비교하세요.",
                    "displayName과 presetId 둘 다 `look2`로 찾히는지 확인하세요.",
                ],
            )
        })?;
    context.preset = Some(ResolvedPresetRecord {
        preset_id: preset.preset_id.clone(),
        display_name: preset.display_name.clone(),
        published_version: preset.published_version.clone(),
    });
    let selection = select_active_preset_in_dir(
        base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: preset.preset_id.clone(),
            published_version: preset.published_version.clone(),
        },
    )
    .map_err(|error| {
        run_failure(
            "preset-select-failed",
            format!("프리셋을 선택하지 못했어요: {}", error.message),
            "세션 catalog snapshot과 selected preset binding이 맞지 않거나 manifest 갱신에 실패했습니다.",
            vec![
                "preset selection 결과와 session.json의 activePreset을 비교하세요.",
                "operator audit에 preset 관련 실패가 남았는지 확인하세요.",
            ],
        )
    })?;
    context
        .append_step(
            "preset-selected",
            "passed",
            "Requested preset selected for the validation session.",
            None,
            None,
            None,
            json!({
                "sessionId": selection.session_id,
                "presetId": preset.preset_id,
                "displayName": preset.display_name,
                "publishedVersion": preset.published_version,
            }),
        )
        .map_err(internal_run_failure)?;

    schedule_preview_renderer_warmup_in_dir(
        base_dir,
        &session.session_id,
        &selection.active_preset.preset_id,
        &selection.active_preset.published_version,
    );
    let warmup_settled = wait_for_preview_renderer_warmup_to_settle(
        &session.session_id,
        &selection.active_preset.preset_id,
        &selection.active_preset.published_version,
        Duration::from_millis(PREVIEW_RUNTIME_WARMUP_SETTLE_TIMEOUT_MS),
    );
    let warmup_step_status = preview_runtime_warmup_step_status(warmup_settled);
    context
        .append_step(
            "preview-runtime-warmed",
            warmup_step_status,
            if warmup_settled {
                "Preview runtime warm-up completed before the first validation capture."
            } else {
                "Preview runtime warm-up did not complete before the first validation capture."
            },
            None,
            None,
            None,
            json!({
                "sessionId": session.session_id,
                "presetId": selection.active_preset.preset_id,
                "publishedVersion": selection.active_preset.published_version,
                "warmupSettled": warmup_settled,
                "timeoutMs": PREVIEW_RUNTIME_WARMUP_SETTLE_TIMEOUT_MS,
            }),
        )
        .map_err(internal_run_failure)?;
    if !warmup_settled {
        return Err(run_failure(
            "preview-runtime-warmup-failed",
            "preview runtime warm-up이 첫 촬영 전에 완료되지 않았어요.",
            "검증 세션이 cold preview path로 시작했거나 warm-up render가 실패했습니다.",
            vec![
                "run-steps.jsonl의 preview-runtime-warmed detail을 확인하세요.",
                "preview warm-up stderr log와 render queue 상태를 확인하세요.",
            ],
        ));
    }

    for capture_index in 1..=input.capture_count.max(1) {
        let ready_readiness = wait_for_ready_capture_gate(base_dir, &session.session_id)?;
        context
            .append_step(
                "capture-cycle-started",
                "running",
                "Capture cycle started.",
                Some(capture_index),
                None,
                None,
                json!({
                    "captureIndex": capture_index,
                    "reasonCode": ready_readiness.reason_code,
                    "customerState": ready_readiness.customer_state,
                }),
            )
            .map_err(internal_run_failure)?;

        let capture_result = request_capture_in_dir(
            base_dir,
            CaptureRequestInputDto {
                session_id: session.session_id.clone(),
                request_id: None,
            },
        )
        .map_err(|error| map_capture_failure(&error, capture_index))?;
        context
            .append_step(
                "capture-saved",
                "passed",
                "Capture was accepted and saved to the session root.",
                Some(capture_index),
                Some(&capture_result.capture.capture_id),
                Some(&capture_result.capture.request_id),
                json!({
                    "captureId": capture_result.capture.capture_id,
                    "requestId": capture_result.capture.request_id,
                    "rawPath": capture_result.capture.raw.asset_path,
                    "renderStatus": capture_result.capture.render_status,
                }),
            )
            .map_err(internal_run_failure)?;

        let initial_reserve_input = wait_for_host_owned_reserve_input_evidence(
            base_dir,
            &session.session_id,
            &capture_result.capture.capture_id,
            &capture_result.capture.request_id,
            Duration::from_millis(HOST_OWNED_RESERVE_INPUT_SETTLE_TIMEOUT_MS),
        )
        .map_err(internal_run_failure)?;

        let preview_capture = complete_preview_render_in_dir(
            base_dir,
            &session.session_id,
            &capture_result.capture.capture_id,
        )
        .map_err(|error| {
            run_failure(
                "preview-render-failed",
                format!(
                    "capture {} preview render를 닫지 못했어요: {}",
                    capture_index, error.message
                ),
                "darktable preview render가 실패했거나 preview close 근거가 부족합니다.",
                vec![
                    "timing-events.log의 preview/final 이벤트를 확인하세요.",
                    "fake/real darktable 실행 경로와 preset bundle metadata를 점검하세요.",
                ],
            )
        })?;
        let reserve_input = if initial_reserve_input.satisfies_host_owned_boundary() {
            initial_reserve_input
        } else {
            let mut refreshed_reserve_input = read_host_owned_reserve_input_evidence(
                base_dir,
                &session.session_id,
                &capture_result.capture.capture_id,
                &capture_result.capture.request_id,
            )
            .map_err(internal_run_failure)?;
            refreshed_reserve_input.preserve_pre_settle_evidence(&initial_reserve_input);
            refreshed_reserve_input.wait_elapsed_ms = initial_reserve_input.wait_elapsed_ms;
            refreshed_reserve_input.wait_timed_out = initial_reserve_input.wait_timed_out;
            refreshed_reserve_input
        };
        let reserve_input_ready = reserve_input.satisfies_host_owned_boundary();
        context
            .append_step(
                "host-owned-reserve-input",
                if reserve_input_ready {
                    "passed"
                } else {
                    "failed"
                },
                if reserve_input_ready {
                    "Host-owned preset-applied fast preview handoff was observed."
                } else {
                    "Host-owned preset-applied fast preview handoff was not observed."
                },
                Some(capture_index),
                Some(&capture_result.capture.capture_id),
                Some(&capture_result.capture.request_id),
                reserve_input.to_step_detail(),
            )
            .map_err(internal_run_failure)?;
        if !reserve_input_ready {
            context
                .append_step(
                    "capture-preview-settled-after-no-go",
                    "passed",
                    "Saved capture preview was settled before returning the No-Go result.",
                    Some(capture_index),
                    Some(&preview_capture.capture_id),
                    Some(&preview_capture.request_id),
                    json!({
                        "captureId": preview_capture.capture_id,
                        "requestId": preview_capture.request_id,
                        "previewPath": preview_capture.preview.asset_path,
                        "previewKind": preview_capture.preview.kind,
                        "renderStatus": preview_capture.render_status,
                    }),
                )
                .map_err(internal_run_failure)?;
            return Err(host_owned_reserve_unavailable_failure(
                capture_index,
                &reserve_input,
            ));
        }
        let readiness = get_capture_readiness_in_dir(
            base_dir,
            CaptureReadinessInputDto {
                session_id: session.session_id.clone(),
            },
        )
        .map_err(|error| {
            run_failure(
                "readiness-refresh-failed",
                format!("촬영 후 readiness를 새로 읽지 못했어요: {}", error.message),
                "캡처 후 manifest 또는 diagnostics sync가 실패했습니다.",
                vec![
                    "session.json과 helper diagnostics를 함께 확인하세요.",
                    "현재 세션의 capture binding이 유지되는지 확인하세요.",
                ],
            )
        })?;

        if !matches!(
            preview_capture.render_status.as_str(),
            "previewReady" | "finalReady"
        ) {
            return Err(run_failure(
                "preview-not-ready",
                format!(
                    "capture {} preview가 준비 상태로 닫히지 않았어요.",
                    capture_index
                ),
                "preview 파일이 생성되지 않았거나 render status가 previewReady/finalReady로 전환되지 않았습니다.",
                vec![
                    "renders/previews 경로에 실제 파일이 있는지 확인하세요.",
                    "timing-events.log의 request-capture -> file-arrived -> preview ready 순서를 확인하세요.",
                ],
            ));
        }
        let preview_route_detail = read_latest_preview_route_detail(
            base_dir,
            &session.session_id,
            &preview_capture.capture_id,
            &preview_capture.request_id,
        )
        .map_err(internal_run_failure)?;
        validate_preview_truth_gate(
            &preview_capture,
            capture_index,
            preview_route_detail.as_deref(),
        )?;

        context.captures_passed += 1;
        if context.first_capture_id.is_none() {
            context.first_capture_id = Some(preview_capture.capture_id.clone());
        }
        context.latest_capture_id = Some(preview_capture.capture_id.clone());
        context
            .append_step(
                "capture-preview-ready",
                "passed",
                "Preview was rendered and became available.",
                Some(capture_index),
                Some(&preview_capture.capture_id),
                Some(&preview_capture.request_id),
                json!({
                    "captureId": preview_capture.capture_id,
                    "requestId": preview_capture.request_id,
                    "previewPath": preview_capture.preview.asset_path,
                    "renderStatus": preview_capture.render_status,
                    "customerState": readiness.customer_state,
                    "reasonCode": readiness.reason_code,
                }),
            )
            .map_err(internal_run_failure)?;
        context
            .append_step(
                "capture-cycle-passed",
                "passed",
                "Capture cycle completed with preview-ready evidence.",
                Some(capture_index),
                Some(&preview_capture.capture_id),
                Some(&preview_capture.request_id),
                json!({
                    "captureId": preview_capture.capture_id,
                    "requestId": preview_capture.request_id,
                    "previewPath": preview_capture.preview.asset_path,
                }),
            )
            .map_err(internal_run_failure)?;
    }

    if input.validate_render_failure_isolation {
        let timeout_ms = input.post_end_wait_timeout_ms.ok_or_else(|| {
            run_failure(
                "post-end-wait-required",
                "렌더 실패 격리 검증에는 종료 후 상태 대기 시간이 필요해요.",
                "HV-11은 Phone Required post-end truth까지 확인해야 하므로 timeout을 지정해야 합니다.",
                vec![
                    "hardware-validation-runner.ps1에 -PostEndTimeoutSeconds 값을 지정하세요.",
                    "1분 검증에서는 보통 120초면 충분합니다.",
                ],
            )
        })?;
        induce_final_render_failure_isolation(base_dir, context, &session.session_id)?;
        let Some(capture_id) = context.first_capture_id.clone() else {
            return Err(run_failure(
                "render-failure-target-missing",
                "렌더 실패를 만들 검증 촬영을 찾지 못했어요.",
                "HV-11은 최소 한 장의 preview-ready 촬영물에 실패 상태를 만들어야 합니다.",
                vec!["run-steps.jsonl에서 capture-preview-ready 단계가 있는지 확인하세요."],
            ));
        };
        wait_for_post_end_state(
            base_dir,
            context,
            &session.session_id,
            timeout_ms,
            PostEndWaitExpectation::RenderFailureIsolation {
                capture_id: capture_id.as_str(),
            },
        )?;
    } else if let Some(timeout_ms) = input.post_end_wait_timeout_ms {
        wait_for_post_end_state(
            base_dir,
            context,
            &session.session_id,
            timeout_ms,
            PostEndWaitExpectation::ExportReady,
        )?;
    }

    Ok(())
}

fn induce_final_render_failure_isolation(
    base_dir: &Path,
    context: &mut RunContext,
    session_id: &str,
) -> Result<(), RunFailure> {
    let Some(capture_id) = context.first_capture_id.clone() else {
        return Err(run_failure(
            "render-failure-target-missing",
            "렌더 실패를 만들 검증 촬영을 찾지 못했어요.",
            "HV-11은 preview-ready 촬영물에 실패 상태를 만들어야 합니다.",
            vec!["run-steps.jsonl에서 capture-preview-ready 단계가 있는지 확인하세요."],
        ));
    };

    if context.latest_capture_id.as_deref() == Some(capture_id.as_str()) {
        return Err(run_failure(
            "render-failure-target-needs-non-latest",
            "렌더 실패 격리 검증에는 최소 2장의 촬영이 필요해요.",
            "최신 촬영 실패는 active session recovery 대상이 될 수 있어, HV-11은 안정적인 non-latest final render failure로 검증합니다.",
            vec![
                "hardware-validation-runner.ps1의 -CaptureCount 값을 2 이상으로 지정하세요.",
                "기본 5컷 검증에서는 첫 번째 촬영을 실패 격리 대상으로 사용합니다.",
            ],
        ));
    }

    let manifest =
        mark_final_render_failed_in_dir(base_dir, session_id, &capture_id).map_err(|error| {
            run_failure(
                "render-failure-induction-failed",
                format!(
                    "HV-11용 final render failure를 만들지 못했어요: {}",
                    error.message
                ),
                "검증 대상 capture가 manifest에 없거나 render failure 상태를 기록하지 못했습니다.",
                vec![
                    "run-steps.jsonl에서 첫 capture-preview-ready의 captureId를 확인하세요.",
                    "session.json에 해당 capture가 남아 있는지 확인하세요.",
                ],
            )
        })?;

    let capture = manifest
        .captures
        .iter()
        .find(|capture| capture.capture_id == capture_id)
        .ok_or_else(|| {
            run_failure(
                "render-failure-target-missing",
                "렌더 실패를 기록한 촬영물이 session.json에서 사라졌어요.",
                "HV-11은 실패 촬영과 원본 증적이 세션에 보존되어야 합니다.",
                vec!["session.json의 captures 배열과 originals 폴더를 확인하세요."],
            )
        })?;
    let raw_preserved = Path::new(&capture.raw.asset_path).is_file();
    let preview_preserved = capture
        .preview
        .asset_path
        .as_deref()
        .map(Path::new)
        .is_some_and(Path::is_file);

    if capture.render_status != "renderFailed" || !raw_preserved || !preview_preserved {
        return Err(run_failure(
            "render-failure-evidence-incomplete",
            "렌더 실패 격리 증적이 충분하지 않아요.",
            "실패 상태, RAW 원본, 확인용 preview가 모두 남아 있어야 고객 화면을 안전하게 보호했다고 볼 수 있습니다.",
            vec![
                "session.json의 renderStatus와 raw.assetPath를 확인하세요.",
                "captures/originals와 renders/previews 파일 존재 여부를 확인하세요.",
            ],
        ));
    }

    context
        .append_step(
            "final-render-failure-induced",
            "passed",
            "Final render failure was induced for HV-11 isolation validation.",
            None,
            Some(&capture_id),
            Some(&capture.request_id),
            json!({
                "sessionId": session_id,
                "captureId": capture.capture_id,
                "requestId": capture.request_id,
                "renderStatus": capture.render_status,
                "lifecycleStage": manifest.lifecycle.stage,
                "rawPath": capture.raw.asset_path,
                "previewPath": capture.preview.asset_path,
                "rawPreserved": raw_preserved,
                "previewPreserved": preview_preserved,
            }),
        )
        .map_err(internal_run_failure)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PostEndWaitExpectation<'a> {
    ExportReady,
    RenderFailureIsolation { capture_id: &'a str },
}

fn wait_for_post_end_state(
    base_dir: &Path,
    context: &mut RunContext,
    session_id: &str,
    timeout_ms: u64,
    expectation: PostEndWaitExpectation<'_>,
) -> Result<(), RunFailure> {
    let started_at = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let mut last_readiness: Option<CaptureReadinessDto> = None;

    while started_at.elapsed() <= timeout {
        let readiness = get_capture_readiness_in_dir(
            base_dir,
            CaptureReadinessInputDto {
                session_id: session_id.into(),
            },
        )
        .map_err(|error| {
            run_failure(
                "post-end-readiness-refresh-failed",
                format!("종료 후 readiness를 새로 읽지 못했어요: {}", error.message),
                "post-end truth 확인 중 manifest 또는 diagnostics sync가 실패했습니다.",
                vec![
                    "session.json의 timing/postEnd 값을 확인하세요.",
                    "timing-events.log에 ended/post-end 이벤트가 남았는지 확인하세요.",
                ],
            )
        })?;

        if readiness.post_end.is_some() {
            match expectation {
                PostEndWaitExpectation::ExportReady => {
                    if matches!(
                        readiness.reason_code.as_str(),
                        "export-waiting" | "completed"
                    ) {
                        context
                            .append_step(
                                "post-end-state-ready",
                                "passed",
                                "Post-end state was observed after the validation captures.",
                                None,
                                None,
                                None,
                                json!({
                                    "sessionId": session_id,
                                    "reasonCode": readiness.reason_code,
                                    "customerState": readiness.customer_state,
                                    "canCapture": readiness.can_capture,
                                    "waitElapsedMs": started_at.elapsed().as_millis(),
                                }),
                            )
                            .map_err(internal_run_failure)?;
                        return Ok(());
                    }

                    if readiness.reason_code == "phone-required" {
                        return Err(run_failure(
                            "post-end-phone-required",
                            "종료 후 상태가 직원 확인 필요로 전환됐어요.",
                            "post-end truth는 생성됐지만 결과 준비 성공 상태가 아니라 보호 상태입니다.",
                            vec![
                                "session.json의 postEnd와 capture renderStatus를 확인하세요.",
                                "final render failure 또는 missing asset 근거를 확인하세요.",
                            ],
                        ));
                    }
                }
                PostEndWaitExpectation::RenderFailureIsolation { capture_id } => {
                    if matches!(
                        readiness.reason_code.as_str(),
                        "export-waiting" | "completed"
                    ) {
                        return Err(run_failure(
                            "render-failure-isolation-not-protected",
                            "렌더 실패가 종료 후 완료/결과 대기 상태처럼 보였어요.",
                            "HV-11은 실패 촬영을 성공처럼 승격하지 않고 직원 확인 필요 상태로 격리해야 합니다.",
                            vec![
                                "session.json의 failed capture renderStatus를 확인하세요.",
                                "postEnd.state가 completed/export-waiting으로 잘못 닫혔는지 확인하세요.",
                            ],
                        ));
                    }

                    if readiness.reason_code == "phone-required" {
                        let detail = render_failure_isolation_detail(
                            base_dir, session_id, capture_id, &readiness,
                        )
                        .map_err(internal_run_failure)?;
                        context
                            .append_step(
                                "post-end-render-failure-isolated",
                                "passed",
                                "Post-end render failure stayed isolated behind staff help guidance.",
                                None,
                                Some(capture_id),
                                None,
                                detail,
                            )
                            .map_err(internal_run_failure)?;
                        return Ok(());
                    }
                }
            }
        }

        last_readiness = Some(readiness);
        thread::sleep(Duration::from_millis(POST_END_WAIT_POLL_INTERVAL_MS));
    }

    let last_reason_code = last_readiness
        .as_ref()
        .map(|readiness| readiness.reason_code.as_str())
        .unwrap_or("unknown");
    let last_customer_state = last_readiness
        .as_ref()
        .map(|readiness| readiness.customer_state.as_str())
        .unwrap_or("unknown");

    Err(run_failure(
        "post-end-readiness-timeout",
        format!(
            "종료 후 postEnd truth가 제한 시간 안에 보이지 않았어요. 마지막 상태는 `{last_reason_code}` / `{last_customer_state}`입니다."
        ),
        "세션 종료 시각에 도달하지 않았거나 post-end evaluator가 durable truth를 기록하지 못했습니다.",
        vec![
            "session.json의 timing.adjustedEndAt, timing.phase, postEnd를 확인하세요.",
            "timing-events.log의 ended/post-end 이벤트 순서를 확인하세요.",
        ],
    ))
}

fn render_failure_isolation_detail(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    readiness: &CaptureReadinessDto,
) -> Result<serde_json::Value, HardwareValidationRunnerError> {
    let paths = SessionPaths::try_new(base_dir, session_id).map_err(|error| {
        HardwareValidationRunnerError::new(format!(
            "render failure isolation path resolution failed: {}",
            error.message
        ))
    })?;
    let manifest = read_session_manifest(&paths.manifest_path).map_err(|error| {
        HardwareValidationRunnerError::new(format!(
            "render failure isolation manifest read failed: {}",
            error.message
        ))
    })?;
    let capture = manifest
        .captures
        .iter()
        .find(|capture| capture.capture_id == capture_id)
        .ok_or_else(|| {
            HardwareValidationRunnerError::new(format!(
                "render failure target capture was missing: {capture_id}"
            ))
        })?;

    let raw_preserved = Path::new(&capture.raw.asset_path).is_file();
    let preview_preserved = capture
        .preview
        .asset_path
        .as_deref()
        .map(Path::new)
        .is_some_and(Path::is_file);
    let final_ready = capture
        .final_asset
        .asset_path
        .as_deref()
        .map(Path::new)
        .is_some_and(Path::is_file)
        && capture.final_asset.ready_at_ms.is_some();

    if capture.render_status != "renderFailed"
        || !raw_preserved
        || !preview_preserved
        || final_ready
        || readiness.can_capture
        || manifest.lifecycle.stage != "phone-required"
    {
        return Err(HardwareValidationRunnerError::new(format!(
            "render failure isolation evidence incomplete: renderStatus={}, rawPreserved={}, previewPreserved={}, finalReady={}, canCapture={}, lifecycleStage={}",
            capture.render_status,
            raw_preserved,
            preview_preserved,
            final_ready,
            readiness.can_capture,
            manifest.lifecycle.stage
        )));
    }

    Ok(json!({
        "sessionId": session_id,
        "captureId": capture.capture_id,
        "requestId": capture.request_id,
        "reasonCode": readiness.reason_code,
        "customerState": readiness.customer_state,
        "canCapture": readiness.can_capture,
        "primaryAction": readiness.primary_action,
        "customerMessage": readiness.customer_message,
        "supportMessage": readiness.support_message,
        "lifecycleStage": manifest.lifecycle.stage,
        "postEndState": manifest.post_end.as_ref().map(|post_end| post_end.state()),
        "renderStatus": capture.render_status,
        "rawPath": capture.raw.asset_path,
        "previewPath": capture.preview.asset_path,
        "rawPreserved": raw_preserved,
        "previewPreserved": preview_preserved,
        "finalReady": final_ready,
    }))
}

#[derive(Debug, Clone, Default)]
struct HostOwnedReserveInputEvidence {
    event_count: usize,
    file_arrived_fast_preview_kind: Option<String>,
    file_arrived_fast_preview_path: Option<String>,
    latest_fast_preview_kind: Option<String>,
    latest_fast_preview_path: Option<String>,
    latest_fast_preview_failure_kind: Option<String>,
    latest_fast_preview_failure_code: Option<String>,
    latest_preview_route_detail: Option<String>,
    latest_speculative_preview_detail: Option<String>,
    speculative_preview_output_ready: bool,
    speculative_preview_lock_present: bool,
    wait_elapsed_ms: u128,
    wait_timed_out: bool,
}

impl HostOwnedReserveInputEvidence {
    fn satisfies_host_owned_boundary(&self) -> bool {
        self.has_host_owned_preview_route_evidence()
    }

    fn has_host_owned_preview_route_evidence(&self) -> bool {
        self.latest_preview_route_detail
            .as_deref()
            .or(self.latest_speculative_preview_detail.as_deref())
            .is_some_and(preview_route_satisfies_host_owned_boundary)
    }

    fn preserve_pre_settle_evidence(&mut self, original: &Self) {
        if self.latest_speculative_preview_detail.is_none() {
            self.latest_speculative_preview_detail =
                original.latest_speculative_preview_detail.clone();
        }
        self.speculative_preview_output_ready |= original.speculative_preview_output_ready;
        self.speculative_preview_lock_present |= original.speculative_preview_lock_present;
    }

    fn observed_summary(&self) -> String {
        format!(
            "fileArrivedKind={};latestFastPreviewKind={};latestFailureKind={};latestFailureCode={};latestPreviewRoute={};latestSpeculativeRoute={};speculativeOutputReady={};speculativeLockPresent={};eventCount={}",
            self.file_arrived_fast_preview_kind
                .as_deref()
                .unwrap_or("none"),
            self.latest_fast_preview_kind
                .as_deref()
                .unwrap_or("none"),
            self.latest_fast_preview_failure_kind
                .as_deref()
                .unwrap_or("none"),
            self.latest_fast_preview_failure_code
                .as_deref()
                .unwrap_or("none"),
            self.latest_preview_route_detail
                .as_deref()
                .map(summarize_preview_route_detail)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none".into()),
            self.latest_speculative_preview_detail
                .as_deref()
                .map(summarize_preview_route_detail)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none".into()),
            self.speculative_preview_output_ready,
            self.speculative_preview_lock_present,
            self.event_count
        )
    }

    fn to_step_detail(&self) -> serde_json::Value {
        json!({
            "satisfiesHostOwnedBoundary": self.satisfies_host_owned_boundary(),
            "eventCount": self.event_count,
            "fileArrivedFastPreviewKind": self.file_arrived_fast_preview_kind,
            "fileArrivedFastPreviewPath": self.file_arrived_fast_preview_path,
            "latestFastPreviewKind": self.latest_fast_preview_kind,
            "latestFastPreviewPath": self.latest_fast_preview_path,
            "latestFastPreviewFailureKind": self.latest_fast_preview_failure_kind,
            "latestFastPreviewFailureCode": self.latest_fast_preview_failure_code,
            "latestPreviewRouteDetail": self.latest_preview_route_detail,
            "latestSpeculativePreviewDetail": self.latest_speculative_preview_detail,
            "speculativePreviewOutputReady": self.speculative_preview_output_ready,
            "speculativePreviewLockPresent": self.speculative_preview_lock_present,
            "waitElapsedMs": self.wait_elapsed_ms,
            "waitTimedOut": self.wait_timed_out,
        })
    }
}

fn wait_for_host_owned_reserve_input_evidence(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
    timeout: Duration,
) -> Result<HostOwnedReserveInputEvidence, HardwareValidationRunnerError> {
    let started_at = Instant::now();

    loop {
        let mut latest_evidence =
            read_host_owned_reserve_input_evidence(base_dir, session_id, capture_id, request_id)?;
        latest_evidence.wait_elapsed_ms = started_at.elapsed().as_millis();

        if latest_evidence.satisfies_host_owned_boundary() {
            return Ok(latest_evidence);
        }

        if started_at.elapsed() >= timeout {
            latest_evidence.wait_elapsed_ms = started_at.elapsed().as_millis();
            latest_evidence.wait_timed_out = true;
            return Ok(latest_evidence);
        }

        thread::sleep(Duration::from_millis(
            HOST_OWNED_RESERVE_INPUT_POLL_INTERVAL_MS,
        ));
    }
}

fn read_host_owned_reserve_input_evidence(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
) -> Result<HostOwnedReserveInputEvidence, HardwareValidationRunnerError> {
    let events_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_EVENTS_FILE_NAME);

    let mut evidence = HostOwnedReserveInputEvidence::default();
    if events_path.exists() {
        for line in fs::read_to_string(events_path)?.lines() {
            let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if json_string_field(&event, "sessionId").as_deref() != Some(session_id)
                || json_string_field(&event, "requestId").as_deref() != Some(request_id)
            {
                continue;
            }
            let event_capture_id = json_string_field(&event, "captureId");
            if event_capture_id
                .as_deref()
                .is_some_and(|value| value != capture_id)
            {
                continue;
            }

            let Some(event_type) = json_string_field(&event, "type") else {
                continue;
            };
            match event_type.as_str() {
                "file-arrived" => {
                    evidence.event_count += 1;
                    evidence.file_arrived_fast_preview_kind =
                        json_string_field(&event, "fastPreviewKind");
                    evidence.file_arrived_fast_preview_path =
                        json_string_field(&event, "fastPreviewPath");
                }
                "fast-preview-ready" => {
                    evidence.event_count += 1;
                    evidence.latest_fast_preview_kind =
                        json_string_field(&event, "fastPreviewKind");
                    evidence.latest_fast_preview_path =
                        json_string_field(&event, "fastPreviewPath");
                }
                "fast-preview-failed" => {
                    evidence.event_count += 1;
                    evidence.latest_fast_preview_failure_kind =
                        json_string_field(&event, "fastPreviewKind");
                    evidence.latest_fast_preview_failure_code =
                        json_string_field(&event, "detailCode");
                }
                _ => {}
            }
        }
    }

    evidence.latest_preview_route_detail =
        read_latest_preview_route_detail(base_dir, session_id, capture_id, request_id)?;
    let paths = SessionPaths::new(base_dir, session_id);
    let speculative_output_path = paths
        .renders_previews_dir
        .join(format!("{capture_id}.preview-speculative.jpg"));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{capture_id}.{request_id}.preview-speculative.detail"
    ));
    let speculative_lock_path = paths.renders_previews_dir.join(format!(
        "{capture_id}.{request_id}.preview-speculative.lock"
    ));
    evidence.speculative_preview_output_ready =
        speculative_output_path.is_file() && speculative_detail_path.is_file();
    evidence.speculative_preview_lock_present = speculative_lock_path.exists();
    if speculative_detail_path.is_file() {
        evidence.latest_speculative_preview_detail = Some(normalize_preview_route_truth_detail(
            fs::read_to_string(speculative_detail_path)?.trim(),
        ))
        .filter(|value| !value.is_empty());
    }

    Ok(evidence)
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn host_owned_reserve_unavailable_failure(
    capture_index: u32,
    evidence: &HostOwnedReserveInputEvidence,
) -> RunFailure {
    run_failure(
        "preview-host-owned-reserve-unavailable",
        format!(
            "capture {capture_index} host-owned preset-applied fast preview handoff가 없어요: {}",
            evidence.observed_summary()
        ),
        "helper가 공식 reserve path 입력인 preset-applied fast-preview handoff를 제공하지 않아 validation이 darktable fallback으로 넘어가기 전에 중단했습니다.",
        vec![
            "camera-helper-events.jsonl에서 file-arrived fastPreviewKind와 fast-preview-ready fastPreviewKind를 확인하세요.",
            "run-steps.jsonl의 latestSpeculativePreviewDetail이 darktable 경로인지 확인하세요.",
            "windows-shell-thumbnail/raw-fallback-preview는 first-visible comparison evidence일 뿐 official host-owned Go 근거가 아닙니다.",
        ],
    )
}

fn validate_preview_truth_gate(
    capture: &SessionCaptureRecord,
    capture_index: u32,
    preview_route_detail: Option<&str>,
) -> Result<(), RunFailure> {
    let preview_kind = capture.preview.kind.as_deref().unwrap_or("unknown");
    if preview_kind != "preset-applied-preview" {
        return Err(preview_truth_gate_failure(
            format!(
                "capture {capture_index} preview kind가 `preset-applied-preview`가 아니라 `{preview_kind}`로 닫혔어요."
            ),
        ));
    }

    let Some(preset_applied_visible_at_ms) = capture.timing.xmp_preview_ready_at_ms else {
        return Err(preview_truth_gate_failure(format!(
            "capture {capture_index}에 preset-applied visible 시각이 기록되지 않았어요."
        )));
    };
    let Some(first_visible_at_ms) = capture.timing.fast_preview_visible_at_ms else {
        return Err(preview_truth_gate_failure(format!(
            "capture {capture_index}에 first-visible 기준 시각이 기록되지 않았어요."
        )));
    };

    if preset_applied_visible_at_ms < first_visible_at_ms {
        return Err(preview_truth_gate_failure(format!(
            "capture {capture_index}의 preset-applied visible 시각이 first-visible보다 앞서는 역전 evidence예요."
        )));
    }

    let official_gate_elapsed_ms = preset_applied_visible_at_ms.saturating_sub(first_visible_at_ms);
    match preview_route_detail {
        Some(detail) if preview_route_satisfies_host_owned_boundary(detail) => {}
        Some(detail) => {
            return Err(preview_route_owner_gate_failure(format!(
                "capture {capture_index} preview route가 host-owned reserve path가 아니에요: {}",
                summarize_preview_route_detail(detail)
            )));
        }
        None => {
            return Err(preview_route_owner_gate_failure(format!(
                "capture {capture_index}에 preview-render-ready route owner evidence가 없어요."
            )));
        }
    }

    if official_gate_elapsed_ms > 3_000 {
        return Err(preview_truth_gate_failure(
            format!(
                "capture {capture_index} official gate가 {official_gate_elapsed_ms}ms로 3000ms를 넘었어요."
            ),
        ));
    }

    Ok(())
}

fn read_latest_preview_route_detail(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
) -> Result<Option<String>, HardwareValidationRunnerError> {
    let log_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("timing-events.log");
    if !log_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(log_path)?;
    Ok(contents
        .lines()
        .filter(|line| {
            line.contains("event=preview-render-ready")
                && line.contains(&format!("capture={capture_id}"))
                && line.contains(&format!("request={request_id}"))
        })
        .filter_map(extract_timing_detail)
        .last()
        .map(str::to_string))
}

fn extract_timing_detail(line: &str) -> Option<&str> {
    line.split_once("\tdetail=").map(|(_, detail)| detail)
}

fn preview_route_satisfies_host_owned_boundary(detail: &str) -> bool {
    detail.contains("binary=fast-preview-handoff")
        && detail.contains("source=fast-preview-handoff")
        && detail.contains("engineSource=host-owned-native")
        && detail.contains("inputSourceAsset=raw-original")
        && detail.contains("sourceAsset=preset-applied-preview")
        && detail.contains("truthOwner=display-sized-preset-applied")
        && detail.contains("truthProfile=original-full-preset")
        && !detail.contains("truthBlocker=")
        && !preview_route_uses_self_labeled_resident_darktable_engine(detail)
        && !preview_route_uses_operation_derived_raster_approximation(detail)
}

fn preview_route_uses_self_labeled_resident_darktable_engine(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    if !normalized.contains("enginemode=resident-full-preset") {
        return false;
    }

    detail.split(';').any(|part| {
        let normalized_part = part.to_ascii_lowercase();
        (normalized_part.starts_with("enginebinary=") && normalized_part.contains("darktable-cli"))
            || (normalized_part.starts_with("enginesource=")
                && normalized_part.contains("program-files-bin"))
            || (normalized_part.starts_with("engineadaptersource=")
                && normalized_part.contains("program-files-bin"))
    })
}

fn preview_route_uses_operation_derived_raster_approximation(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    normalized.contains("profile=operation-derived")
        || normalized.contains("inputsourceasset=fast-preview-raster")
}

fn normalize_preview_route_truth_detail(detail: &str) -> String {
    let with_truth_owner = if detail.contains("truthOwner=") {
        detail.to_string()
    } else {
        format!("{detail};truthOwner=display-sized-preset-applied")
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

fn summarize_preview_route_detail(detail: &str) -> String {
    let interesting_keys = [
        "binary=",
        "source=",
        "sourceAsset=",
        "inputSourceAsset=",
        "truthOwner=",
        "truthProfile=",
        "truthBlocker=",
        "requiredInputSourceAsset=",
        "elapsedMs=",
        "engineBinary=",
        "engineSource=",
        "engineMode=",
        "engineAdapter=",
        "engineAdapterSource=",
    ];
    detail
        .split(';')
        .filter(|part| interesting_keys.iter().any(|key| part.starts_with(key)))
        .collect::<Vec<_>>()
        .join(";")
}

fn preview_route_owner_gate_failure(problem: String) -> RunFailure {
    run_failure(
        "preview-route-owner-gate-failed",
        problem,
        "공식 hardware Go는 raw-original full-preset route evidence로만 닫혀야 합니다.",
        vec![
            "timing-events.log의 preview-render-ready detail에서 binary/source가 fast-preview-handoff인지 확인하세요.",
            "metadata-only preview, fast-preview-raster, operation-derived profile, self-labeled resident route는 official Go로 승격하지 마세요.",
        ],
    )
}

fn preview_truth_gate_failure(problem: String) -> RunFailure {
    run_failure(
        "preview-truth-gate-failed",
        problem,
        "preview가 preset-applied truthful close로 닫히지 않았거나 official gate를 넘었습니다.",
        vec![
            "session.json의 latest capture preview.kind와 timing.xmpPreviewReadyAtMs를 확인하세요.",
            "timing-events.log의 capture_preview_ready detail에서 originalVisibleToPresetAppliedVisibleMs를 확인하세요.",
        ],
    )
}

fn preview_runtime_warmup_step_status(warmup_settled: bool) -> &'static str {
    if warmup_settled {
        "passed"
    } else {
        "failed"
    }
}

fn finalize_run(
    base_dir: &Path,
    context: RunContext,
    run_outcome: Result<(), RunFailure>,
) -> Result<HardwareValidationRunResult, HardwareValidationRunnerError> {
    let mut failure_diagnostics = None;
    let (status, failure) = match run_outcome {
        Ok(()) => ("passed".to_string(), None),
        Err(failure) => {
            if let Some(snapshot) = collect_failure_diagnostics(base_dir, &context)? {
                fs::write(
                    &context.failure_diagnostics_path,
                    serde_json::to_vec_pretty(&snapshot)?,
                )?;
                context.append_step(
                    "failure-diagnostics-captured",
                    "passed",
                    "Failure diagnostics snapshot was captured.",
                    None,
                    None,
                    None,
                    json!({
                        "failureDiagnosticsPath": context.failure_diagnostics_path.to_string_lossy(),
                        "readinessReasonCode": snapshot.last_readiness.as_ref().map(|readiness| readiness.reason_code.clone()),
                        "helperDetailCode": snapshot.helper_status.as_ref().and_then(|status| status.detail_code.clone()),
                        "cameraConnectionState": snapshot
                            .operator_session_summary
                            .as_ref()
                            .map(|summary| summary.camera_connection.state.clone()),
                    }),
                )?;
                failure_diagnostics = Some(snapshot);
            }
            context.append_step(
                "run-failed",
                "failed",
                "Hardware validation failed.",
                None,
                None,
                None,
                json!({
                    "code": failure.diagnostic.code,
                    "problem": failure.diagnostic.problem,
                }),
            )?;
            ("failed".to_string(), Some(failure.diagnostic))
        }
    };
    let completed_at = current_timestamp(SystemTime::now())
        .map_err(|error| HardwareValidationRunnerError::new(error.message))?;
    let summary = RunSummary {
        schema_version: "hardware-validation-summary/v1",
        status: status.clone(),
        prompt: context.prompt.clone(),
        session_id: context.session_id.clone(),
        booth_alias: context.booth_alias.clone(),
        captures_requested: context.requested_capture_count,
        captures_passed: context.captures_passed,
        preset: context.preset.clone(),
        app_launch_mode: context.app_launch_mode.clone(),
        app_launched: context.app_launched,
        started_at: context.started_at.clone(),
        completed_at,
        failure: failure.clone(),
    };
    fs::write(&context.summary_path, serde_json::to_vec_pretty(&summary)?)?;

    let artifacts_index = build_artifacts_index(base_dir, &context)?;
    fs::write(
        &context.artifacts_index_path,
        serde_json::to_vec_pretty(&artifacts_index)?,
    )?;

    let failure_report_path = if let Some(failure) = failure {
        fs::write(
            &context.failure_report_path,
            build_failure_report_markdown(
                &context,
                &failure,
                &artifacts_index,
                failure_diagnostics.as_ref(),
            ),
        )?;
        Some(context.failure_report_path.clone())
    } else {
        None
    };

    context.append_step(
        "run-completed",
        if status == "passed" {
            "passed"
        } else {
            "failed"
        },
        "Hardware validation run completed.",
        None,
        None,
        None,
        json!({
            "status": status,
            "capturesPassed": context.captures_passed,
            "capturesRequested": context.requested_capture_count,
        }),
    )?;

    Ok(HardwareValidationRunResult {
        status,
        capture_count: context.requested_capture_count,
        run_dir: context.run_dir,
        summary_path: context.summary_path,
        steps_path: context.steps_path,
        artifacts_index_path: context.artifacts_index_path,
        failure_report_path,
        session_id: context.session_id,
    })
}

fn build_artifacts_index(
    base_dir: &Path,
    context: &RunContext,
) -> Result<ArtifactIndex, HardwareValidationRunnerError> {
    let session_paths = context
        .session_id
        .as_deref()
        .map(|session_id| SessionPaths::new(base_dir, session_id));
    let session_manifest_path = session_paths
        .as_ref()
        .map(|paths| paths.manifest_path.to_string_lossy().into_owned())
        .filter(|path| Path::new(path).exists());
    let captures_originals_dir = session_paths
        .as_ref()
        .map(|paths| paths.captures_originals_dir.to_string_lossy().into_owned())
        .filter(|path| Path::new(path).exists());
    let renders_previews_dir = session_paths
        .as_ref()
        .map(|paths| paths.renders_previews_dir.to_string_lossy().into_owned())
        .filter(|path| Path::new(path).exists());
    let diagnostics_dir = session_paths
        .as_ref()
        .map(|paths| paths.diagnostics_dir.to_string_lossy().into_owned())
        .filter(|path| Path::new(path).exists());
    let timing_events_log_path = session_paths
        .as_ref()
        .map(|paths| paths.diagnostics_dir.join("timing-events.log"))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned());
    let helper_status_path = session_paths
        .as_ref()
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_STATUS_FILE_NAME))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned());
    let helper_startup_log_path = session_paths
        .as_ref()
        .map(|paths| paths.diagnostics_dir.join("camera-helper-startup.log"))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned());
    let helper_requests_path = session_paths
        .as_ref()
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_REQUESTS_FILE_NAME))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned());
    let helper_events_path = session_paths
        .as_ref()
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_EVENTS_FILE_NAME))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned());
    let operator_audit_log_path = base_dir
        .join("diagnostics")
        .join("operator-audit-log.json")
        .to_string_lossy()
        .into_owned();
    let operator_audit_log_path = Path::new(&operator_audit_log_path)
        .exists()
        .then_some(operator_audit_log_path);

    if operator_audit_log_path.is_some() && context.session_id.is_some() {
        let _ = load_operator_audit_history_in_dir(
            base_dir,
            &crate::contracts::dto::CapabilitySnapshotDto {
                is_admin_authenticated: true,
                allowed_surfaces: vec!["operator".into()],
                authenticated_actor_id: None,
                authenticated_actor_label: None,
            },
            crate::contracts::dto::OperatorAuditQueryFilterDto {
                session_id: context.session_id.clone(),
                event_categories: Vec::new(),
                limit: Some(20),
            },
        );
    }

    Ok(ArtifactIndex {
        schema_version: "hardware-validation-artifacts/v1",
        run_dir: context.run_dir.to_string_lossy().into_owned(),
        session_id: context.session_id.clone(),
        session_manifest_path,
        captures_originals_dir,
        renders_previews_dir,
        diagnostics_dir,
        timing_events_log_path,
        operator_audit_log_path,
        helper_status_path,
        helper_startup_log_path,
        helper_requests_path,
        helper_events_path,
        failure_diagnostics_path: context.failure_diagnostics_path.exists().then(|| {
            context
                .failure_diagnostics_path
                .to_string_lossy()
                .into_owned()
        }),
    })
}

fn build_failure_report_markdown(
    context: &RunContext,
    failure: &FailureDiagnostic,
    artifacts: &ArtifactIndex,
    failure_diagnostics: Option<&FailureDiagnosticsSnapshot>,
) -> String {
    let mut report = String::new();
    report.push_str("# Hardware Validation Failure Report\n\n");
    report.push_str(&format!("- Prompt: `{}`\n", context.prompt));
    report.push_str(&format!("- Preset query: `{}`\n", context.preset_query));
    if let Some(session_id) = context.session_id.as_deref() {
        report.push_str(&format!("- Session ID: `{session_id}`\n"));
    }
    report.push_str(&format!("- Failure code: `{}`\n", failure.code));
    report.push_str(&format!("- Problem: {}\n", failure.problem));
    report.push_str(&format!("- Suspected cause: {}\n", failure.suspected_cause));
    report.push_str("- Debug hints:\n");
    for hint in &failure.debug_hints {
        report.push_str(&format!("  - {hint}\n"));
    }
    if let Some(snapshot) = failure_diagnostics {
        report.push_str("\n## Diagnostic Snapshot\n");
        if let Some(readiness) = snapshot.last_readiness.as_ref() {
            report.push_str(&format!(
                "- Last readiness snapshot: reasonCode=`{}`, customerState=`{}`, canCapture=`{}`\n",
                readiness.reason_code, readiness.customer_state, readiness.can_capture
            ));
        } else if let Some(error) = snapshot.readiness_error.as_deref() {
            report.push_str(&format!(
                "- Last readiness snapshot: unavailable ({error})\n"
            ));
        }
        if let Some(status) = snapshot.helper_status.as_ref() {
            report.push_str(&format!(
                "- Helper status: cameraState=`{}`, helperState=`{}`, detailCode=`{}`, cameraModel=`{}`, observedAt=`{}`\n",
                status.camera_state,
                status.helper_state,
                status.detail_code.as_deref().unwrap_or("unknown"),
                status.camera_model.as_deref().unwrap_or("unknown"),
                status.observed_at,
            ));
        } else if let Some(error) = snapshot.helper_status_error.as_deref() {
            report.push_str(&format!("- Helper status: unavailable ({error})\n"));
        }
        if let Some(helper_error) = snapshot.helper_error.as_ref() {
            report.push_str(&format!(
                "- Helper error: detailCode=`{}`, message=`{}`\n",
                helper_error.detail_code,
                helper_error.message.as_deref().unwrap_or("unknown"),
            ));
        }
        if let Some(summary) = snapshot.operator_session_summary.as_ref() {
            report.push_str(&format!(
                "- Operator summary: blockedStateCategory=`{}`, lifecycleStage=`{}`, cameraConnection=`{}`\n",
                summary.blocked_state_category,
                summary.lifecycle_stage.as_deref().unwrap_or("unknown"),
                summary.camera_connection.state,
            ));
        }
        if !snapshot.startup_log_tail.is_empty() {
            report.push_str("- Startup log tail:\n");
            for line in snapshot.startup_log_tail.iter().take(8) {
                report.push_str(&format!("  - {line}\n"));
            }
        }
    }
    report.push_str("\n## Artifacts\n");
    if let Some(path) = artifacts.session_manifest_path.as_deref() {
        report.push_str(&format!("- session.json: `{path}`\n"));
    }
    if let Some(path) = artifacts.timing_events_log_path.as_deref() {
        report.push_str(&format!("- timing-events.log: `{path}`\n"));
    }
    if let Some(path) = artifacts.operator_audit_log_path.as_deref() {
        report.push_str(&format!("- operator-audit-log.json: `{path}`\n"));
    }
    if let Some(path) = artifacts.helper_status_path.as_deref() {
        report.push_str(&format!("- camera-helper-status.json: `{path}`\n"));
    }
    if let Some(path) = artifacts.helper_startup_log_path.as_deref() {
        report.push_str(&format!("- camera-helper-startup.log: `{path}`\n"));
    }
    if let Some(path) = artifacts.helper_requests_path.as_deref() {
        report.push_str(&format!("- camera-helper-requests.jsonl: `{path}`\n"));
    }
    if let Some(path) = artifacts.helper_events_path.as_deref() {
        report.push_str(&format!("- camera-helper-events.jsonl: `{path}`\n"));
    }
    if let Some(path) = artifacts.failure_diagnostics_path.as_deref() {
        report.push_str(&format!("- failure-diagnostics.json: `{path}`\n"));
    }

    report
}

fn collect_failure_diagnostics(
    base_dir: &Path,
    context: &RunContext,
) -> Result<Option<FailureDiagnosticsSnapshot>, HardwareValidationRunnerError> {
    let Some(session_id) = context.session_id.as_deref() else {
        return Ok(None);
    };
    let collected_at = current_timestamp(SystemTime::now())
        .map_err(|error| HardwareValidationRunnerError::new(error.message))?;
    let session_paths = SessionPaths::new(base_dir, session_id);
    let (last_readiness, readiness_error) = match get_capture_readiness_in_dir(
        base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.into(),
        },
    ) {
        Ok(readiness) => (Some(readiness), None),
        Err(error) => (None, Some(error.message)),
    };
    let (helper_status, helper_status_error) =
        match read_latest_status_message(base_dir, session_id) {
            Ok(message) => (message, None),
            Err(error) => (None, Some(format!("{error:?}"))),
        };
    let (helper_error, helper_error_read_error) =
        match read_latest_helper_error_message(base_dir, session_id) {
            Ok(message) => (message, None),
            Err(error) => (None, Some(format!("{error:?}"))),
        };
    let capability_snapshot = CapabilitySnapshotDto {
        is_admin_authenticated: true,
        allowed_surfaces: vec!["operator".into()],
        authenticated_actor_id: None,
        authenticated_actor_label: None,
    };
    let (operator_session_summary, operator_session_summary_error) =
        match load_operator_session_summary_in_dir(base_dir, &capability_snapshot) {
            Ok(summary) => (Some(summary), None),
            Err(error) => (None, Some(error.message)),
        };

    Ok(Some(FailureDiagnosticsSnapshot {
        schema_version: "hardware-validation-failure-diagnostics/v1",
        session_id: session_id.to_string(),
        collected_at,
        last_readiness,
        readiness_error,
        helper_status,
        helper_status_error,
        helper_error,
        helper_error_read_error,
        operator_session_summary,
        operator_session_summary_error,
        startup_log_tail: read_text_log_tail(
            &session_paths
                .diagnostics_dir
                .join("camera-helper-startup.log"),
            20,
        )?,
        helper_requests_tail: read_text_log_tail(
            &session_paths
                .diagnostics_dir
                .join(CAMERA_HELPER_REQUESTS_FILE_NAME),
            10,
        )?,
        helper_events_tail: read_text_log_tail(
            &session_paths
                .diagnostics_dir
                .join(CAMERA_HELPER_EVENTS_FILE_NAME),
            10,
        )?,
        timing_events_tail: read_text_log_tail(
            &session_paths.diagnostics_dir.join("timing-events.log"),
            20,
        )?,
    }))
}

fn read_text_log_tail(
    path: &Path,
    max_lines: usize,
) -> Result<Vec<String>, HardwareValidationRunnerError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)?;
    let lines = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].to_vec())
}

fn launch_app_if_needed(
    base_dir: &Path,
    app_launch_mode: &AppLaunchMode,
    context: &mut RunContext,
) -> Result<Option<Child>, HardwareValidationRunnerError> {
    match app_launch_mode {
        AppLaunchMode::Skip => {
            context.append_step(
                "app-launch-skipped",
                "passed",
                "App launch was skipped for this run.",
                None,
                None,
                None,
                json!({
                    "baseDir": base_dir.to_string_lossy(),
                }),
            )?;
            Ok(None)
        }
        AppLaunchMode::LaunchSiblingExe => {
            let current_exe = std::env::current_exe()?;
            let sibling_name = if cfg!(windows) {
                "boothy.exe"
            } else {
                "boothy"
            };
            let sibling_exe = current_exe
                .parent()
                .map(|path| path.join(sibling_name))
                .ok_or_else(|| {
                    HardwareValidationRunnerError::new("runner executable parent is missing")
                })?;
            if !sibling_exe.exists() {
                return Err(HardwareValidationRunnerError::new(format!(
                    "boothy executable was not found next to the runner: {}",
                    sibling_exe.display()
                )));
            }

            let child = Command::new(&sibling_exe)
                .env("BOOTHY_RUNTIME_PROFILE", "operator-enabled")
                .env("BOOTHY_ADMIN_AUTHENTICATED", "true")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            context.app_launched = true;
            context.append_step(
                "app-launched",
                "passed",
                "Boothy app launched for hardware validation.",
                None,
                None,
                None,
                json!({
                    "path": sibling_exe.to_string_lossy(),
                }),
            )?;
            thread::sleep(Duration::from_secs(3));
            Ok(Some(child))
        }
    }
}

fn resolve_session_start_input(
    prompt: &str,
    phone_last_four: Option<String>,
) -> SessionStartInputDto {
    let mut tokens = prompt
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let phone_last_four = phone_last_four.unwrap_or_else(|| {
        if let Some(index) = tokens.iter().rposition(|token| {
            token.len() == 4 && token.chars().all(|value| value.is_ascii_digit())
        }) {
            return tokens.remove(index);
        }

        tokens
            .iter()
            .rposition(|token| {
                token.len() > 4
                    && token
                        .chars()
                        .rev()
                        .take(4)
                        .all(|value| value.is_ascii_digit())
                    && token
                        .chars()
                        .take(token.chars().count().saturating_sub(4))
                        .any(|value| !value.is_ascii_digit())
            })
            .map(|index| {
                let token = tokens.remove(index);
                let (name_part, last_four) = token.split_at(token.len() - 4);
                let name_part = name_part.trim_end_matches(['-', '_']);
                if !name_part.is_empty() {
                    tokens.insert(index, name_part.to_string());
                }
                last_four.to_string()
            })
            .unwrap_or_else(|| "0000".into())
    });
    let normalized_name = normalize_customer_name(&tokens.join(" "));
    let name = if normalized_name.is_empty() {
        "Hardware Validation".into()
    } else {
        normalized_name
    };

    SessionStartInputDto {
        name,
        phone_last_four,
    }
}

fn resolve_requested_preset(
    presets: &[PublishedPresetSummaryDto],
    preset_query: &str,
) -> Option<PublishedPresetSummaryDto> {
    let normalized_query = preset_query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return None;
    }

    presets
        .iter()
        .find(|preset| preset.preset_id.eq_ignore_ascii_case(&normalized_query))
        .cloned()
        .or_else(|| {
            presets
                .iter()
                .find(|preset| {
                    preset
                        .display_name
                        .trim()
                        .eq_ignore_ascii_case(&normalized_query)
                })
                .cloned()
        })
}

fn wait_for_ready_capture_gate(
    base_dir: &Path,
    session_id: &str,
) -> Result<crate::contracts::dto::CaptureReadinessDto, RunFailure> {
    let timeout = ready_capture_gate_timeout();
    let helper_bootstrap_after = Duration::from_secs(1);
    let start = Instant::now();
    let mut last_readiness = None;
    let mut helper_bootstrap_requested = false;

    while start.elapsed() <= timeout {
        let readiness = get_capture_readiness_in_dir(
            base_dir,
            CaptureReadinessInputDto {
                session_id: session_id.into(),
            },
        )
        .map_err(|error| {
            run_failure(
                "readiness-refresh-failed",
                format!("촬영 준비 상태를 읽지 못했어요: {}", error.message),
                "readiness 확인 단계에서 manifest 또는 helper status sync가 실패했습니다.",
                vec![
                    "camera-helper status와 session.json 최신 시각을 비교하세요.",
                    "operator audit에 readiness 관련 오류가 있는지 확인하세요.",
                ],
            )
        })?;

        if readiness_satisfies_capture_gate(&readiness) {
            return Ok(readiness);
        }

        if !helper_bootstrap_requested
            && start.elapsed() >= helper_bootstrap_after
            && readiness_missing_helper_status(&readiness)
        {
            try_ensure_helper_running(base_dir, session_id);
            helper_bootstrap_requested = true;
        }

        last_readiness = Some(readiness);
        thread::sleep(Duration::from_millis(250));
    }

    let last_reason_code = last_readiness
        .as_ref()
        .map(|readiness| readiness.reason_code.clone())
        .unwrap_or_else(|| "unknown".into());
    let last_customer_state = last_readiness
        .as_ref()
        .map(|readiness| readiness.customer_state.clone())
        .unwrap_or_else(|| "unknown".into());

    Err(run_failure(
        "capture-readiness-timeout",
        "다음 촬영 전에 Ready 상태로 복귀하지 않았어요.".to_string(),
        format!(
            "camera/helper status가 freshness를 회복하지 못했거나 마지막 readiness가 `{last_reason_code}` / `{last_customer_state}`로 남았습니다."
        ),
        vec![
            "camera-helper-status.json의 observedAt freshness를 확인하세요.",
            "timing-events.log에서 이전 capture close 이후 helper status가 다시 갱신됐는지 확인하세요.",
        ],
    ))
}

fn ready_capture_gate_timeout() -> Duration {
    Duration::from_secs(15)
}

fn readiness_missing_helper_status(readiness: &crate::contracts::dto::CaptureReadinessDto) -> bool {
    readiness.reason_code == "camera-preparing"
        && readiness.live_capture_truth.as_ref().is_some_and(|truth| {
            truth.freshness == "missing"
                && truth.session_match == "unknown"
                && truth.camera_state == "unknown"
                && truth.helper_state == "unknown"
        })
}

fn readiness_satisfies_capture_gate(
    readiness: &crate::contracts::dto::CaptureReadinessDto,
) -> bool {
    readiness.can_capture
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_capture_gate_accepts_warning_when_capture_is_allowed() {
        let readiness = CaptureReadinessDto::warning("session_000000000000000000000001", None);

        assert!(readiness_satisfies_capture_gate(&readiness));
    }

    #[test]
    fn post_end_wait_accepts_export_waiting_truth() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-post-end-wait-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let output_dir = base_dir.join("validation-output");
        let session = start_session_in_dir(
            &base_dir,
            SessionStartInputDto {
                name: "Kim".into(),
                phone_last_four: "4821".into(),
            },
        )
        .expect("session should start");
        let paths = SessionPaths::new(&base_dir, &session.session_id);
        let mut manifest =
            crate::session::session_repository::read_session_manifest(&paths.manifest_path)
                .expect("manifest should read");
        let timing = manifest.timing.as_mut().expect("timing should exist");
        timing.adjusted_end_at = "2000-01-01T00:00:00Z".into();
        timing.warning_at = "1999-12-31T23:55:00Z".into();
        crate::session::session_repository::write_session_manifest(&paths.manifest_path, &manifest)
            .expect("manifest should be writable");

        let mut context = RunContext::new(&output_dir, "Kim4821", "look2", 1, &AppLaunchMode::Skip)
            .expect("run context should be created");
        context.session_id = Some(session.session_id.clone());

        wait_for_post_end_state(
            &base_dir,
            &mut context,
            &session.session_id,
            1_000,
            PostEndWaitExpectation::ExportReady,
        )
        .expect("post-end wait should observe export waiting");

        let steps = fs::read_to_string(context.steps_path).expect("step log should exist");
        assert!(steps.contains("\"eventType\":\"post-end-state-ready\""));
        assert!(steps.contains("\"reasonCode\":\"export-waiting\""));

        let manifest =
            crate::session::session_repository::read_session_manifest(&paths.manifest_path)
                .expect("manifest should read after wait");
        assert_eq!(
            manifest.post_end.as_ref().map(|post_end| post_end.state()),
            Some("export-waiting")
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn post_end_wait_accepts_render_failure_isolation_truth() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-post-end-render-failure-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let output_dir = base_dir.join("validation-output");
        let session = start_session_in_dir(
            &base_dir,
            SessionStartInputDto {
                name: "Kim".into(),
                phone_last_four: "4821".into(),
            },
        )
        .expect("session should start");
        let paths = SessionPaths::new(&base_dir, &session.session_id);
        fs::create_dir_all(&paths.captures_originals_dir).expect("originals dir should exist");
        fs::create_dir_all(&paths.renders_previews_dir).expect("previews dir should exist");

        let first_raw = paths.captures_originals_dir.join("capture_01.CR2");
        let second_raw = paths.captures_originals_dir.join("capture_02.CR2");
        let first_preview = paths.renders_previews_dir.join("capture_01.jpg");
        let second_preview = paths.renders_previews_dir.join("capture_02.jpg");
        fs::write(&first_raw, b"raw-1").expect("first raw should exist");
        fs::write(&second_raw, b"raw-2").expect("second raw should exist");
        fs::write(&first_preview, b"preview-1").expect("first preview should exist");
        fs::write(&second_preview, b"preview-2").expect("second preview should exist");

        let mut first_capture =
            truth_gate_capture("preset-applied-preview", Some(1_000), Some(1_500));
        first_capture.session_id = session.session_id.clone();
        first_capture.booth_alias = session.booth_alias.clone();
        first_capture.capture_id = "capture_01".into();
        first_capture.request_id = "request_01".into();
        first_capture.raw.asset_path = first_raw.to_string_lossy().into_owned();
        first_capture.preview.asset_path = Some(first_preview.to_string_lossy().into_owned());
        let mut second_capture = first_capture.clone();
        second_capture.capture_id = "capture_02".into();
        second_capture.request_id = "request_02".into();
        second_capture.raw.asset_path = second_raw.to_string_lossy().into_owned();
        second_capture.preview.asset_path = Some(second_preview.to_string_lossy().into_owned());

        let mut manifest =
            crate::session::session_repository::read_session_manifest(&paths.manifest_path)
                .expect("manifest should read");
        let timing = manifest.timing.as_mut().expect("timing should exist");
        timing.adjusted_end_at = "2000-01-01T00:00:00Z".into();
        timing.warning_at = "1999-12-31T23:55:00Z".into();
        manifest.captures = vec![first_capture, second_capture];
        crate::session::session_repository::write_session_manifest(&paths.manifest_path, &manifest)
            .expect("manifest should be writable");

        crate::capture::ingest_pipeline::mark_final_render_failed_in_dir(
            &base_dir,
            &session.session_id,
            "capture_01",
        )
        .expect("final render failure should be marked");

        let mut context = RunContext::new(&output_dir, "Kim4821", "look2", 2, &AppLaunchMode::Skip)
            .expect("run context should be created");
        context.session_id = Some(session.session_id.clone());

        wait_for_post_end_state(
            &base_dir,
            &mut context,
            &session.session_id,
            1_000,
            PostEndWaitExpectation::RenderFailureIsolation {
                capture_id: "capture_01",
            },
        )
        .expect("post-end wait should observe phone-required render failure isolation");

        let steps = fs::read_to_string(context.steps_path).expect("step log should exist");
        assert!(steps.contains("\"eventType\":\"post-end-render-failure-isolated\""));
        assert!(steps.contains("\"reasonCode\":\"phone-required\""));

        let manifest =
            crate::session::session_repository::read_session_manifest(&paths.manifest_path)
                .expect("manifest should read after wait");
        let failed_capture = manifest
            .captures
            .iter()
            .find(|capture| capture.capture_id == "capture_01")
            .expect("failed capture should remain");
        assert_eq!(failed_capture.render_status, "renderFailed");
        assert_eq!(manifest.lifecycle.stage, "phone-required");
        assert_eq!(
            manifest.post_end.as_ref().map(|post_end| post_end.state()),
            Some("phone-required")
        );
        assert!(first_raw.is_file());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn preview_truth_gate_rejects_inverted_timing_evidence() {
        let capture = truth_gate_capture("preset-applied-preview", Some(2_000), Some(1_500));

        let failure = validate_preview_truth_gate(&capture, 1, None)
            .expect_err("inverted timing evidence should fail the official gate");

        assert_eq!(failure.diagnostic.code, "preview-truth-gate-failed");
        assert!(failure.diagnostic.problem.contains("역전"));
    }

    #[test]
    fn preview_runtime_warmup_step_status_marks_unsettled_as_failed() {
        assert_eq!(preview_runtime_warmup_step_status(true), "passed");
        assert_eq!(preview_runtime_warmup_step_status(false), "failed");
    }

    #[test]
    fn host_owned_reserve_input_waits_past_early_non_host_preview() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-host-owned-reserve-wait-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let session_id = "session_000000000000000000000001";
        let capture_id = "capture_01";
        let request_id = "request_01";
        let events_path = SessionPaths::new(&base_dir, session_id)
            .diagnostics_dir
            .join(CAMERA_HELPER_EVENTS_FILE_NAME);
        fs::create_dir_all(
            events_path
                .parent()
                .expect("events path should have a parent"),
        )
        .expect("diagnostics dir should be writable");
        append_helper_event_for_wait_test(
            &events_path,
            json!({
                "schemaVersion": "canon-helper-file-arrived/v1",
                "type": "file-arrived",
                "sessionId": session_id,
                "requestId": request_id,
                "captureId": capture_id,
                "rawPath": "C:/capture_01.CR2",
                "fastPreviewPath": null,
                "fastPreviewKind": null,
            }),
        );
        append_helper_event_for_wait_test(
            &events_path,
            json!({
                "schemaVersion": "canon-helper-fast-preview-ready/v1",
                "type": "fast-preview-ready",
                "sessionId": session_id,
                "requestId": request_id,
                "captureId": capture_id,
                "fastPreviewPath": "C:/capture_01.shell.jpg",
                "fastPreviewKind": "windows-shell-thumbnail",
            }),
        );

        let delayed_events_path = events_path.clone();
        let delayed_timing_path = SessionPaths::new(&base_dir, session_id)
            .diagnostics_dir
            .join("timing-events.log");
        let delayed_session_id = session_id.to_string();
        let delayed_request_id = request_id.to_string();
        let delayed_capture_id = capture_id.to_string();
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            append_helper_event_for_wait_test(
                &delayed_events_path,
                json!({
                    "schemaVersion": "canon-helper-fast-preview-ready/v1",
                    "type": "fast-preview-ready",
                    "sessionId": delayed_session_id,
                    "requestId": delayed_request_id,
                    "captureId": delayed_capture_id,
                    "fastPreviewPath": "C:/capture_01.preset-applied-preview.jpg",
                    "fastPreviewKind": "preset-applied-preview",
                }),
            );
            fs::write(
                &delayed_timing_path,
                format!(
                    "2026-04-27T00:00:00Z\tsession={delayed_session_id}\tcapture={delayed_capture_id}\trequest={delayed_request_id}\tevent=preview-render-ready\tstage=preview\treason=render-ready\tdetail=presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=900;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=none;status=ready\n"
                ),
            )
            .expect("truthful route timing evidence should be writable");
        });

        let evidence = wait_for_host_owned_reserve_input_evidence(
            &base_dir,
            session_id,
            capture_id,
            request_id,
            Duration::from_millis(1_000),
        )
        .expect("wait evidence should be readable");
        writer.join().expect("delayed event writer should complete");

        assert!(evidence.satisfies_host_owned_boundary());
        assert_eq!(
            evidence.latest_fast_preview_kind.as_deref(),
            Some("preset-applied-preview")
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn host_owned_reserve_input_rejects_preset_applied_kind_without_truth_route_evidence() {
        let evidence = HostOwnedReserveInputEvidence {
            latest_fast_preview_kind: Some("preset-applied-preview".into()),
            latest_fast_preview_path: Some("C:/capture_01.preset-applied-preview.jpg".into()),
            ..Default::default()
        };

        assert!(
            !evidence.satisfies_host_owned_boundary(),
            "preset-applied kind alone is not enough; official reserve input needs raw-original original/full-preset route evidence"
        );
    }

    #[test]
    fn host_owned_reserve_input_accepts_host_route_timing_evidence_without_helper_handoff() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-host-owned-reserve-route-evidence-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let session_id = "session_000000000000000000000001";
        let capture_id = "capture_01";
        let request_id = "request_01";
        let paths = SessionPaths::new(&base_dir, session_id);
        fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics dir should be writable");
        fs::write(
            paths.diagnostics_dir.join("timing-events.log"),
            format!(
                "2026-04-27T00:00:00Z\tsession={session_id}\tcapture={capture_id}\trequest={request_id}\tevent=preview-render-ready\tstage=preview\treason=render-ready\tdetail=presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=900;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=none;status=ready\n"
            ),
        )
        .expect("timing events should be writable");

        let evidence =
            read_host_owned_reserve_input_evidence(&base_dir, session_id, capture_id, request_id)
                .expect("evidence should be readable");

        assert!(
            evidence.satisfies_host_owned_boundary(),
            "host-owned route evidence should satisfy the reserve boundary even without helper metadata"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn host_owned_reserve_input_records_speculative_preview_route_evidence() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-host-owned-reserve-speculative-evidence-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let session_id = "session_000000000000000000000001";
        let capture_id = "capture_01";
        let request_id = "request_01";
        let paths = SessionPaths::new(&base_dir, session_id);
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should be writable");
        fs::write(
            paths
                .renders_previews_dir
                .join("capture_01.preview-speculative.jpg"),
            [0xFF, 0xD8, 0xFF, 0xD9],
        )
        .expect("speculative preview output should be writable");
        fs::write(
            paths
                .renders_previews_dir
                .join("capture_01.request_01.preview-speculative.detail"),
            "presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=900;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=none;status=ready",
        )
        .expect("speculative detail should be writable");

        let evidence =
            read_host_owned_reserve_input_evidence(&base_dir, session_id, capture_id, request_id)
                .expect("evidence should be readable");

        assert!(evidence.speculative_preview_output_ready);
        assert!(
            evidence.satisfies_host_owned_boundary(),
            "speculative fast-preview-handoff route evidence should satisfy the reserve boundary"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn no_go_failure_evidence_preserves_pre_settle_speculative_detail_when_settle_cleans_it_up() {
        let original = HostOwnedReserveInputEvidence {
            latest_speculative_preview_detail: Some(
                normalize_preview_route_truth_detail("presetId=preset_test;publishedVersion=2026.04.10;binary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;source=program-files-bin;elapsedMs=3011;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=none;status=0"),
            ),
            speculative_preview_output_ready: true,
            speculative_preview_lock_present: true,
            wait_elapsed_ms: 3_023,
            wait_timed_out: true,
            ..Default::default()
        };
        let mut refreshed = HostOwnedReserveInputEvidence {
            latest_preview_route_detail: Some(
                "presetId=preset_test;publishedVersion=2026.04.10;binary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;source=program-files-bin;elapsedMs=3011;detail=widthCap=256;heightCap=256;hq=false;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;args=none;status=0".into(),
            ),
            ..Default::default()
        };

        refreshed.preserve_pre_settle_evidence(&original);

        let summary = refreshed.observed_summary();
        assert!(
            summary.contains("latestSpeculativeRoute=binary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;source=program-files-bin;"),
            "final No-Go summary should retain the speculative route that existed before settle cleanup: {summary}"
        );
        assert!(summary.contains("inputSourceAsset=fast-preview-raster"));
        assert!(summary.contains("sourceAsset=preset-applied-preview"));
        assert!(summary.contains("truthOwner=display-sized-preset-applied"));
        assert!(summary.contains("elapsedMs=3011"));
        assert!(
            summary.contains("speculativeOutputReady=true"),
            "final No-Go summary should retain pre-settle speculative output readiness: {summary}"
        );
        assert!(
            summary.contains("speculativeLockPresent=true"),
            "final No-Go summary should retain pre-settle speculative lock state: {summary}"
        );
    }

    #[test]
    fn host_owned_reserve_input_normalizes_speculative_darktable_route_for_failure_readout() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-host-owned-reserve-normalized-speculative-evidence-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let session_id = "session_000000000000000000000001";
        let capture_id = "capture_01";
        let request_id = "request_01";
        let paths = SessionPaths::new(&base_dir, session_id);
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should be writable");
        fs::write(
            paths
                .renders_previews_dir
                .join("capture_01.preview-speculative.jpg"),
            [0xFF, 0xD8, 0xFF, 0xD9],
        )
        .expect("speculative preview output should be writable");
        fs::write(
            paths
                .renders_previews_dir
                .join("capture_01.request_01.preview-speculative.detail"),
            "presetId=preset_test;publishedVersion=2026.04.10;binary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;source=program-files-bin;elapsedMs=3008;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=none;status=0",
        )
        .expect("speculative detail should be writable");

        let evidence =
            read_host_owned_reserve_input_evidence(&base_dir, session_id, capture_id, request_id)
                .expect("evidence should be readable");
        let detail = evidence
            .latest_speculative_preview_detail
            .as_deref()
            .expect("speculative detail should be recorded");

        assert!(detail.contains("inputSourceAsset=fast-preview-raster"));
        assert!(detail.contains("sourceAsset=preset-applied-preview"));
        assert!(detail.contains("truthOwner=display-sized-preset-applied"));
        assert!(
            !evidence.satisfies_host_owned_boundary(),
            "darktable speculative routes remain comparison evidence after normalization"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn preview_truth_gate_rejects_darktable_route_even_inside_latency_budget() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));
        let failure = validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;source=program-files-bin;elapsedMs=950;detail=widthCap=256;heightCap=256;hq=false;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied"),
        )
        .expect_err("darktable preview route must not satisfy the official host-owned boundary");

        assert_eq!(failure.diagnostic.code, "preview-route-owner-gate-failed");
        assert!(failure
            .diagnostic
            .problem
            .contains("host-owned reserve path"));
    }

    #[test]
    fn preview_truth_gate_rejects_darktable_backed_fast_preview_handoff() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));
        let failure = validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=950;detail=widthCap=256;heightCap=256;hq=false;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;engineBinary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;engineSource=program-files-bin;args=none;status=0"),
        )
        .expect_err("darktable-backed handoff must remain comparison evidence");

        assert_eq!(failure.diagnostic.code, "preview-route-owner-gate-failed");
    }

    #[test]
    fn preview_truth_gate_rejects_operation_derived_raster_handoff() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));
        let failure = validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=950;detail=widthCap=256;heightCap=256;hq=false;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/preview-source.jpg output=C:/preview.jpg profile=operation-derived;status=0"),
        )
        .expect_err("operation-derived raster handoff is not full original preset truth");

        assert_eq!(failure.diagnostic.code, "preview-route-owner-gate-failed");
    }

    #[test]
    fn preview_truth_gate_rejects_fast_preview_handoff_without_original_full_preset_profile() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));
        let failure = validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=950;detail=widthCap=display;heightCap=display;hq=false;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=none;status=ready"),
        )
        .expect_err("host-owned handoff without original/full-preset proof is not official truth");

        assert_eq!(failure.diagnostic.code, "preview-route-owner-gate-failed");
    }

    #[test]
    fn preview_truth_gate_rejects_truth_blocked_full_preset_labels() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));
        let failure = validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=1000;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;truthBlocker=renderer-proof-missing;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=none;status=metadata-only"),
        )
        .expect_err("truth-blocked labels must not satisfy the official hardware gate");

        assert_eq!(failure.diagnostic.code, "preview-route-owner-gate-failed");
    }

    #[test]
    fn preview_truth_gate_accepts_fast_preview_handoff_route_inside_latency_budget() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));

        validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=1000;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=none;status=ready"),
        )
        .expect("fast preview handoff route should satisfy the official host-owned boundary");
    }

    #[test]
    fn preview_truth_gate_accepts_explicit_per_capture_darktable_full_preset_route() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));

        validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=1000;detail=widthCap=384;heightCap=384;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineMode=per-capture-cli;engineAdapter=darktable-compatible;engineAdapterSource=program-files-bin;engineBinary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;engineSource=host-owned-native;args=none;status=0"),
        )
        .expect("explicit per-capture full-preset route should satisfy the current product boundary");
    }

    #[test]
    fn preview_truth_gate_rejects_self_labeled_resident_darktable_compatible_route() {
        let capture = truth_gate_capture("preset-applied-preview", Some(1_000), Some(2_000));

        let failure = validate_preview_truth_gate(
            &capture,
            1,
            Some("presetId=preset_test;publishedVersion=2026.04.10;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=1000;detail=widthCap=384;heightCap=384;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineMode=resident-full-preset;engineAdapter=darktable-compatible;engineAdapterSource=program-files-bin;engineBinary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe;engineSource=host-owned-native;args=none;status=0"),
        )
        .expect_err("self-labeled resident darktable-cli route must not satisfy the official host-owned boundary");

        assert_eq!(failure.diagnostic.code, "preview-route-owner-gate-failed");
    }

    #[test]
    fn readiness_wait_budget_covers_runtime_reconnect_headroom() {
        assert!(
            ready_capture_gate_timeout() >= Duration::from_secs(15),
            "validation readiness timeout should not be shorter than helper reconnect recovery"
        );
    }

    fn append_helper_event_for_wait_test(path: &Path, event: serde_json::Value) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("event log should be writable");
        writeln!(
            file,
            "{}",
            serde_json::to_string(&event).expect("event should serialize")
        )
        .expect("event line should be writable");
    }

    fn truth_gate_capture(
        preview_kind: &str,
        first_visible_at_ms: Option<u64>,
        preset_applied_visible_at_ms: Option<u64>,
    ) -> SessionCaptureRecord {
        SessionCaptureRecord {
            schema_version: "session-capture/v1".into(),
            session_id: "session_000000000000000000000001".into(),
            booth_alias: "Kim 4821".into(),
            active_preset_id: Some("preset_test".into()),
            active_preset_version: "2026.04.10".into(),
            active_preset_display_name: Some("look2".into()),
            capture_id: "capture_01".into(),
            request_id: "request_01".into(),
            raw: crate::session::session_manifest::RawCaptureAsset {
                asset_path: "captures/originals/capture_01.CR2".into(),
                persisted_at_ms: 1_000,
            },
            preview: crate::session::session_manifest::PreviewCaptureAsset {
                asset_path: Some("renders/previews/capture_01.jpg".into()),
                enqueued_at_ms: Some(1_000),
                ready_at_ms: preset_applied_visible_at_ms,
                kind: Some(preview_kind.into()),
            },
            final_asset: crate::session::session_manifest::FinalCaptureAsset {
                asset_path: None,
                ready_at_ms: None,
            },
            render_status: "previewReady".into(),
            post_end_state: "activeSession".into(),
            timing: crate::session::session_manifest::CaptureTimingMetrics {
                capture_acknowledged_at_ms: 900,
                preview_visible_at_ms: preset_applied_visible_at_ms,
                fast_preview_visible_at_ms: first_visible_at_ms,
                xmp_preview_ready_at_ms: preset_applied_visible_at_ms,
                capture_budget_ms: 5_000,
                preview_budget_ms: 15_000,
                preview_budget_state: "withinBudget".into(),
            },
        }
    }
}

fn map_capture_failure(error: &HostErrorEnvelope, capture_index: u32) -> RunFailure {
    let code = if error.code == "capture-not-ready" {
        error
            .readiness
            .as_ref()
            .map(|readiness| readiness.reason_code.clone())
            .unwrap_or_else(|| error.code.clone())
    } else {
        error.code.clone()
    };
    run_failure(
        "capture-request-failed",
        format!("capture {} 요청이 실패했어요: {}", capture_index, error.message),
        format!("capture 단계에서 host가 `{code}` 상태로 요청을 거절했거나 helper round trip을 닫지 못했습니다."),
        vec![
            "camera-helper status/request/event 로그를 먼저 확인하세요.",
            "session.json의 latest capture와 readiness.reasonCode를 비교하세요.",
        ],
    )
}

fn run_failure(
    code: impl Into<String>,
    problem: impl Into<String>,
    suspected_cause: impl Into<String>,
    debug_hints: Vec<&str>,
) -> RunFailure {
    RunFailure {
        diagnostic: FailureDiagnostic {
            code: code.into(),
            problem: problem.into(),
            suspected_cause: suspected_cause.into(),
            debug_hints: debug_hints.into_iter().map(str::to_string).collect(),
        },
    }
}

fn internal_run_failure(error: HardwareValidationRunnerError) -> RunFailure {
    run_failure(
        "runner-internal-error",
        error.to_string(),
        "러너가 자신의 로그 또는 산출물 파일을 쓰는 중에 실패했습니다.",
        vec![
            "run directory 쓰기 권한과 디스크 상태를 확인하세요.",
            "run-steps.jsonl과 run-summary.json 생성 시점을 비교하세요.",
        ],
    )
}

pub fn default_runtime_base_dir() -> PathBuf {
    resolve_app_session_base_dir(std::env::temp_dir())
}
