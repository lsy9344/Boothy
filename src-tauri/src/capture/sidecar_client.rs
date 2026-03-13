use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
};

use serde_json::Value;

use crate::contracts::{
    dto::{
        CameraReadinessPayload, CameraReadinessRequest, CameraStatusChangedEvent,
        CameraStatusSnapshot, CaptureCommandRequest, CaptureProgressEvent, NormalizedErrorEnvelope,
        SidecarCaptureRequest, SidecarCaptureRequestPayload, SidecarCaptureSuccessResponse,
        SidecarErrorResponse, SidecarSuccessResponse,
    },
    schema_version::PROTOCOL_SCHEMA_VERSION,
};

pub use crate::contracts::dto::MockScenario;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarReadinessOutcome {
    pub progress_events: Vec<CameraStatusChangedEvent>,
    pub success: Option<SidecarSuccessResponse>,
    pub error: Option<NormalizedErrorEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarCaptureOutcome {
    pub progress_events: Vec<CaptureProgressEvent>,
    pub success: Option<SidecarCaptureSuccessResponse>,
    pub error: Option<NormalizedErrorEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarReadinessWatchMessage {
    Success(CameraStatusSnapshot),
    Error(NormalizedErrorEnvelope),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarClientConfig {
    pub node_executable: String,
    pub mock_sidecar_path: PathBuf,
}

impl Default for SidecarClientConfig {
    fn default() -> Self {
        Self {
            node_executable: "node".into(),
            mock_sidecar_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("sidecar")
                .join("mock")
                .join("mock-camera-sidecar.mjs"),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SidecarReadinessWatchRequest {
    schema_version: String,
    request_id: String,
    correlation_id: String,
    method: String,
    session_id: Option<String>,
    payload: CameraReadinessPayload,
}

pub struct MockReadinessWatch {
    child: Child,
    stdout: BufReader<ChildStdout>,
}

pub fn run_mock_readiness_sidecar(
    config: &SidecarClientConfig,
    request: &CameraReadinessRequest,
) -> Result<SidecarReadinessOutcome, String> {
    let stdout = run_mock_sidecar(config, request, "readiness request")?;
    let mut progress_events = Vec::new();
    let mut success = None;
    let mut error = None;

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let message: Value =
            serde_json::from_str(line).map_err(|parse_error| format!("invalid sidecar JSON: {parse_error}"))?;

        if message
            .get("schemaVersion")
            .and_then(Value::as_str)
            != Some(PROTOCOL_SCHEMA_VERSION)
        {
            return Err(format!("unexpected sidecar schemaVersion: {message}"));
        }

        if message.get("event").and_then(Value::as_str) == Some("camera.statusChanged") {
            let event: CameraStatusChangedEvent = serde_json::from_value(message)
                .map_err(|parse_error| format!("invalid sidecar progress event: {parse_error}"))?;
            progress_events.push(event);
            continue;
        }

        match message.get("ok").and_then(Value::as_bool) {
            Some(true) => {
                success = Some(
                    serde_json::from_value(message)
                        .map_err(|parse_error| format!("invalid sidecar success response: {parse_error}"))?,
                );
            }
            Some(false) => {
                let response: SidecarErrorResponse = serde_json::from_value(message)
                    .map_err(|parse_error| format!("invalid sidecar error response: {parse_error}"))?;
                error = Some(response.error);
            }
            _ => return Err(format!("unexpected sidecar response: {line}")),
        }
    }

    Ok(SidecarReadinessOutcome {
        progress_events,
        success,
        error,
    })
}

pub fn watch_mock_readiness_sidecar(
    config: &SidecarClientConfig,
    request: &CameraReadinessRequest,
) -> Result<MockReadinessWatch, String> {
    let mut child = spawn_mock_sidecar(
        config,
        &SidecarReadinessWatchRequest {
            schema_version: request.schema_version.clone(),
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            method: "camera.watchReadiness".into(),
            session_id: request.session_id.clone(),
            payload: request.payload.clone(),
        },
        "readiness watch request",
    )?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "mock camera sidecar stdout is unavailable".to_string())?;

    Ok(MockReadinessWatch {
        child,
        stdout: BufReader::new(stdout),
    })
}

pub fn run_mock_capture_sidecar(
    config: &SidecarClientConfig,
    request: &CaptureCommandRequest,
    capture_id: &str,
    original_file_name: &str,
    processed_file_name: &str,
    original_output_path: &Path,
    processed_output_path: &Path,
) -> Result<SidecarCaptureOutcome, String> {
    let stdout = run_mock_sidecar(
        config,
        &SidecarCaptureRequest::new(
            request,
            capture_id,
            original_file_name,
            processed_file_name,
            original_output_path,
            processed_output_path,
        ),
        "capture request",
    )?;
    let mut progress_events = Vec::new();
    let mut success = None;
    let mut error = None;

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let message: Value =
            serde_json::from_str(line).map_err(|parse_error| format!("invalid sidecar JSON: {parse_error}"))?;

        if message.get("event").and_then(Value::as_str) == Some("capture.progress") {
            if message
                .get("schemaVersion")
                .and_then(Value::as_str)
                != Some(PROTOCOL_SCHEMA_VERSION)
            {
                return Err(format!("unexpected capture progress schemaVersion: {message}"));
            }

            let event: CaptureProgressEvent = serde_json::from_value(message)
                .map_err(|parse_error| format!("invalid capture progress event: {parse_error}"))?;
            progress_events.push(event);
            continue;
        }

        match message.get("ok").and_then(Value::as_bool) {
            Some(true) => {
                success = Some(
                    serde_json::from_value(message)
                        .map_err(|parse_error| format!("invalid capture success response: {parse_error}"))?,
                );
            }
            Some(false) => {
                let response: SidecarErrorResponse = serde_json::from_value(message)
                    .map_err(|parse_error| format!("invalid sidecar error response: {parse_error}"))?;
                error = Some(response.error);
            }
            _ => return Err(format!("unexpected sidecar response: {line}")),
        }
    }

    Ok(SidecarCaptureOutcome {
        progress_events,
        success,
        error,
    })
}

impl SidecarCaptureRequest {
    fn new(
        request: &CaptureCommandRequest,
        capture_id: &str,
        original_file_name: &str,
        processed_file_name: &str,
        original_output_path: &Path,
        processed_output_path: &Path,
    ) -> Self {
        Self {
            schema_version: request.schema_version.clone(),
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            method: request.method.clone(),
            session_id: request.session_id.clone(),
            payload: SidecarCaptureRequestPayload {
                active_preset: request.payload.active_preset.clone(),
                capture_id: capture_id.into(),
                original_file_name: original_file_name.into(),
                processed_file_name: processed_file_name.into(),
                original_output_path: original_output_path.to_string_lossy().replace('\\', "/"),
                processed_output_path: processed_output_path.to_string_lossy().replace('\\', "/"),
            },
        }
    }
}

fn run_mock_sidecar<T: serde::Serialize>(
    config: &SidecarClientConfig,
    request: &T,
    request_name: &str,
) -> Result<String, String> {
    let child = spawn_mock_sidecar(config, request, request_name)?;

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for mock camera sidecar: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("mock camera sidecar exited with status {}", output.status)
        } else {
            stderr
        });
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("mock camera sidecar stdout was not valid UTF-8: {error}"))
}

fn spawn_mock_sidecar<T: serde::Serialize>(
    config: &SidecarClientConfig,
    request: &T,
    request_name: &str,
) -> Result<Child, String> {
    let mut child = Command::new(&config.node_executable)
        .arg(&config.mock_sidecar_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to launch mock camera sidecar: {error}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "mock camera sidecar stdin is unavailable".to_string())?;
        let request_line =
            serde_json::to_string(request).map_err(|error| format!("invalid camera request: {error}"))?;
        stdin
            .write_all(format!("{request_line}\n").as_bytes())
            .map_err(|error| format!("failed to write {request_name} to sidecar: {error}"))?;
    }

    Ok(child)
}

impl MockReadinessWatch {
    pub fn next_message(&mut self) -> Result<Option<SidecarReadinessWatchMessage>, String> {
        loop {
            let mut line = String::new();
            let bytes_read = self
                .stdout
                .read_line(&mut line)
                .map_err(|error| format!("failed to read mock readiness watch output: {error}"))?;

            if bytes_read == 0 {
                return Ok(None);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let message: Value = serde_json::from_str(trimmed)
                .map_err(|parse_error| format!("invalid sidecar JSON: {parse_error}"))?;

            if message.get("event").and_then(Value::as_str) == Some("camera.statusChanged") {
                continue;
            }

            if message
                .get("schemaVersion")
                .and_then(Value::as_str)
                != Some(PROTOCOL_SCHEMA_VERSION)
            {
                return Err(format!("unexpected sidecar schemaVersion: {message}"));
            }

            return match message.get("ok").and_then(Value::as_bool) {
                Some(true) => {
                    let response: SidecarSuccessResponse = serde_json::from_value(message)
                        .map_err(|parse_error| format!("invalid sidecar success response: {parse_error}"))?;
                    Ok(Some(SidecarReadinessWatchMessage::Success(response.status)))
                }
                Some(false) => {
                    let response: SidecarErrorResponse = serde_json::from_value(message)
                        .map_err(|parse_error| format!("invalid sidecar error response: {parse_error}"))?;
                    Ok(Some(SidecarReadinessWatchMessage::Error(response.error)))
                }
                _ => Err(format!("unexpected sidecar response: {trimmed}")),
            };
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.child
            .kill()
            .map_err(|error| format!("failed to stop mock readiness watch: {error}"))?;
        self.child
            .wait()
            .map_err(|error| format!("failed to wait for mock readiness watch shutdown: {error}"))?;
        Ok(())
    }
}
