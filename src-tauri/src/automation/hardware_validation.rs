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
        ingest_pipeline::complete_preview_render_in_dir,
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
    session::{
        session_manifest::{current_timestamp, normalize_customer_name},
        session_paths::SessionPaths,
        session_repository::{
            resolve_app_session_base_dir, select_active_preset_in_dir, start_session_in_dir,
        },
    },
};

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

        context.captures_passed += 1;
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

    Ok(())
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
    let timeout = Duration::from_secs(8);
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
