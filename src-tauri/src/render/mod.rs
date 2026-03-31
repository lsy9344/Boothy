use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    },
    session::{session_manifest::SessionCaptureRecord, session_paths::SessionPaths},
};

const PINNED_DARKTABLE_VERSION: &str = "5.4.1";
const MAX_IN_FLIGHT_RENDER_JOBS: usize = 2;
const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(45);
const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
const PREVIEW_MAX_WIDTH_PX: u32 = 1280;
const PREVIEW_MAX_HEIGHT_PX: u32 = 1280;

static RENDER_QUEUE_DEPTH: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderIntent {
    Preview,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCaptureAsset {
    pub asset_path: String,
    pub ready_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderWorkerError {
    pub reason_code: &'static str,
    pub customer_message: String,
    pub operator_detail: String,
}

pub fn render_capture_asset_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    intent: RenderIntent,
) -> Result<RenderedCaptureAsset, RenderWorkerError> {
    let _queue_guard = acquire_render_queue_slot()?;
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let preset_id =
        capture
            .active_preset_id
            .as_deref()
            .ok_or_else(|| RenderWorkerError {
                reason_code: "missing-preset-binding",
                customer_message: safe_render_failure_message(intent),
                operator_detail:
                    "capture record에 activePresetId가 없어 published bundle을 고정할 수 없어요."
                        .into(),
            })?;
    let bundle = find_published_preset_runtime_bundle(
        &catalog_root,
        preset_id,
        &capture.active_preset_version,
    )
    .ok_or_else(|| RenderWorkerError {
        reason_code: "bundle-resolution-failed",
        customer_message: safe_render_failure_message(intent),
        operator_detail: format!(
            "capture-bound bundle을 찾지 못했어요: presetId={preset_id}, publishedVersion={}",
            capture.active_preset_version
        ),
    })?;

    if bundle.darktable_version != PINNED_DARKTABLE_VERSION {
        return Err(RenderWorkerError {
            reason_code: "darktable-version-mismatch",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "pinned darktable version과 bundle metadata가 다릅니다: expected={PINNED_DARKTABLE_VERSION}, actual={}",
                bundle.darktable_version
            ),
        });
    }

    let paths = SessionPaths::new(base_dir, session_id);
    let output_path = match intent {
        RenderIntent::Preview => paths
            .renders_previews_dir
            .join(format!("{}.jpg", capture.capture_id)),
        RenderIntent::Final => paths
            .renders_finals_dir
            .join(format!("{}.jpg", capture.capture_id)),
    };
    let output_root = match intent {
        RenderIntent::Preview => &paths.renders_previews_dir,
        RenderIntent::Final => &paths.renders_finals_dir,
    };

    fs::create_dir_all(output_root).map_err(|error| RenderWorkerError {
        reason_code: "render-output-dir-unavailable",
        customer_message: safe_render_failure_message(intent),
        operator_detail: format!("render output directory를 준비하지 못했어요: {error}"),
    })?;

    if !output_path.starts_with(output_root) {
        return Err(RenderWorkerError {
            reason_code: "invalid-output-path",
            customer_message: safe_render_failure_message(intent),
            operator_detail: "render output path가 현재 세션 범위를 벗어났어요.".into(),
        });
    }

    let invocation = build_darktable_invocation(
        base_dir,
        &bundle.darktable_version,
        &bundle.xmp_template_path,
        &capture.raw.asset_path,
        &output_path,
        intent,
    );
    log::info!(
        "render_job_started session={} capture_id={} stage={} binary={} source={} detail={}",
        session_id,
        capture.capture_id,
        render_stage_label(intent),
        invocation.binary,
        invocation.binary_source,
        render_invocation_detail(intent)
    );
    let render_started = Instant::now();
    let invocation_result = run_darktable_invocation(&invocation, intent)?;
    validate_render_output(&output_path, intent)?;
    let render_elapsed_ms = render_started.elapsed().as_millis();

    let ready_at_ms = current_time_ms().map_err(|error| RenderWorkerError {
        reason_code: "render-clock-unavailable",
        customer_message: safe_render_failure_message(intent),
        operator_detail: error,
    })?;

    append_render_event(
        &paths,
        &capture.capture_id,
        intent,
        "ready",
        Some(match intent {
            RenderIntent::Preview => "preview-ready",
            RenderIntent::Final => "final-ready",
        }),
        Some(&format!(
            "presetId={};publishedVersion={};binary={};source={};elapsedMs={};detail={};args={};status={}",
            bundle.preset_id,
            bundle.published_version,
            invocation.binary,
            invocation.binary_source,
            render_elapsed_ms,
            render_invocation_detail(intent),
            invocation.arguments.join(" "),
            invocation_result.exit_code
        )),
    );

    Ok(RenderedCaptureAsset {
        asset_path: output_path.to_string_lossy().into_owned(),
        ready_at_ms,
    })
}

pub fn log_render_failure_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    intent: RenderIntent,
    reason_code: &str,
) {
    let paths = SessionPaths::new(base_dir, session_id);
    append_render_event(
        &paths,
        capture_id,
        intent,
        "failed",
        Some(reason_code),
        None,
    );
}

pub fn is_valid_render_preview_asset(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => has_jpeg_signature(path),
        Some("png") => has_png_signature(path),
        Some("webp") | Some("gif") | Some("bmp") => true,
        _ => false,
    }
}

fn safe_render_failure_message(intent: RenderIntent) -> String {
    match intent {
        RenderIntent::Preview => {
            "확인용 사진을 아직 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into()
        }
        RenderIntent::Final => {
            "결과 사진을 아직 마무리하지 못했어요. 가까운 직원에게 알려 주세요.".into()
        }
    }
}

fn render_stage_label(intent: RenderIntent) -> &'static str {
    match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
    }
}

fn render_invocation_detail(intent: RenderIntent) -> String {
    match intent {
        RenderIntent::Preview => {
            format!("widthCap={PREVIEW_MAX_WIDTH_PX};heightCap={PREVIEW_MAX_HEIGHT_PX};hq=false")
        }
        RenderIntent::Final => "widthCap=full;heightCap=full;hq=true".into(),
    }
}

fn acquire_render_queue_slot() -> Result<RenderQueueGuard, RenderWorkerError> {
    let mut depth = RENDER_QUEUE_DEPTH.lock().map_err(|_| RenderWorkerError {
        reason_code: "render-queue-unavailable",
        customer_message: "결과 사진을 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into(),
        operator_detail: "render queue mutex를 잠그지 못했어요.".into(),
    })?;

    if *depth >= MAX_IN_FLIGHT_RENDER_JOBS {
        return Err(RenderWorkerError {
            reason_code: "render-queue-saturated",
            customer_message: "결과 사진을 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into(),
            operator_detail: format!(
                "bounded render queue가 가득 찼어요. inFlight={}, max={MAX_IN_FLIGHT_RENDER_JOBS}",
                *depth
            ),
        });
    }

    *depth += 1;

    Ok(RenderQueueGuard {})
}

fn current_time_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "render worker가 시스템 시계를 읽지 못했어요.".to_string())?
        .as_millis() as u64)
}

fn append_render_event(
    paths: &SessionPaths,
    capture_id: &str,
    intent: RenderIntent,
    event: &str,
    reason_code: Option<&str>,
    detail: Option<&str>,
) {
    let occurred_at = match crate::session::session_manifest::current_timestamp(SystemTime::now()) {
        Ok(value) => value,
        Err(_) => return,
    };
    let _ = fs::create_dir_all(&paths.diagnostics_dir);
    let log_path = paths.diagnostics_dir.join("timing-events.log");
    let mut file = match OpenOptions::new().create(true).append(true).open(log_path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let stage = match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
    };
    let reason_code = reason_code.unwrap_or("none");
    let detail = detail.unwrap_or("none");
    let _ = writeln!(
        file,
        "{occurred_at}\tsession={}\tcapture={capture_id}\tevent=render-{event}\tstage={stage}\treason={reason_code}\tdetail={detail}",
        paths
            .session_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    );
}

struct RenderQueueGuard {}

impl Drop for RenderQueueGuard {
    fn drop(&mut self) {
        if let Ok(mut depth) = RENDER_QUEUE_DEPTH.lock() {
            *depth = depth.saturating_sub(1);
        }
    }
}

struct DarktableInvocation {
    binary: String,
    binary_source: &'static str,
    arguments: Vec<String>,
    working_directory: PathBuf,
}

struct DarktableInvocationResult {
    exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DarktableBinaryResolution {
    binary: String,
    source: &'static str,
}

fn build_darktable_invocation(
    base_dir: &Path,
    _darktable_version: &str,
    xmp_template_path: &Path,
    raw_asset_path: &str,
    output_path: &Path,
    intent: RenderIntent,
) -> DarktableInvocation {
    let mode = match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
    };
    let worker_root = base_dir.join(".boothy-darktable").join(mode);
    let configdir = worker_root.join("config");
    let library = worker_root.join("library.db");
    let hq_flag = match intent {
        RenderIntent::Preview => "false",
        RenderIntent::Final => "true",
    };
    let binary_resolution = resolve_darktable_cli_binary();
    let mut arguments = vec![
        raw_asset_path.replace('\\', "/"),
        xmp_template_path.to_string_lossy().replace('\\', "/"),
        output_path.to_string_lossy().replace('\\', "/"),
        "--hq".into(),
        hq_flag.into(),
    ];

    if matches!(intent, RenderIntent::Preview) {
        arguments.push("--width".into());
        arguments.push(PREVIEW_MAX_WIDTH_PX.to_string());
        arguments.push("--height".into());
        arguments.push(PREVIEW_MAX_HEIGHT_PX.to_string());
    }

    arguments.extend([
        "--core".into(),
        "--configdir".into(),
        configdir.to_string_lossy().replace('\\', "/"),
        "--library".into(),
        library.to_string_lossy().replace('\\', "/"),
    ]);

    DarktableInvocation {
        binary: binary_resolution.binary,
        binary_source: binary_resolution.source,
        arguments,
        working_directory: base_dir.to_path_buf(),
    }
}

fn resolve_darktable_cli_binary() -> DarktableBinaryResolution {
    let env_override = std::env::var(DARKTABLE_CLI_BIN_ENV).ok();
    let candidates = darktable_cli_binary_candidates();
    resolve_darktable_cli_binary_with_candidates(env_override.as_deref(), &candidates)
}

fn resolve_darktable_cli_binary_with_candidates(
    env_override: Option<&str>,
    candidates: &[(&'static str, PathBuf)],
) -> DarktableBinaryResolution {
    if let Some(value) = env_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return DarktableBinaryResolution {
            binary: value.to_string(),
            source: "env-override",
        };
    }

    for (source, candidate) in candidates {
        if candidate.is_file() {
            return DarktableBinaryResolution {
                binary: candidate.to_string_lossy().into_owned(),
                source,
            };
        }
    }

    DarktableBinaryResolution {
        binary: "darktable-cli".into(),
        source: "path",
    }
}

fn darktable_cli_binary_candidates() -> Vec<(&'static str, PathBuf)> {
    let mut candidates = Vec::new();

    if !cfg!(windows) {
        return candidates;
    }

    push_darktable_cli_candidate(
        &mut candidates,
        "program-files-bin",
        std::env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|root| root.join("darktable").join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "program-w6432-bin",
        std::env::var_os("ProgramW6432")
            .map(PathBuf::from)
            .map(|root| root.join("darktable").join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "localappdata-programs-bin",
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| {
                root.join("Programs")
                    .join("darktable")
                    .join("bin")
                    .join("darktable-cli.exe")
            }),
    );

    candidates
}

fn push_darktable_cli_candidate(
    candidates: &mut Vec<(&'static str, PathBuf)>,
    source: &'static str,
    candidate: Option<PathBuf>,
) {
    let Some(candidate) = candidate else {
        return;
    };

    if candidates
        .iter()
        .any(|(_, existing)| *existing == candidate)
    {
        return;
    }

    candidates.push((source, candidate));
}

fn run_darktable_invocation(
    invocation: &DarktableInvocation,
    intent: RenderIntent,
) -> Result<DarktableInvocationResult, RenderWorkerError> {
    let mut child = Command::new(&invocation.binary)
        .args(&invocation.arguments)
        .current_dir(&invocation.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let reason_code = if error.kind() == std::io::ErrorKind::NotFound {
                "render-cli-missing"
            } else {
                "render-process-launch-failed"
            };

            RenderWorkerError {
                reason_code,
                customer_message: safe_render_failure_message(intent),
                operator_detail: format!(
                    "darktable-cli를 시작하지 못했어요: binary={} source={} error={error}",
                    invocation.binary, invocation.binary_source
                ),
            }
        })?;

    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| RenderWorkerError {
                        reason_code: "render-process-wait-failed",
                        customer_message: safe_render_failure_message(intent),
                        operator_detail: format!(
                            "render 프로세스 출력을 회수하지 못했어요: {error}"
                        ),
                    })?;

                if status.success() {
                    return Ok(DarktableInvocationResult {
                        exit_code: status.code().unwrap_or(0),
                    });
                }

                return Err(RenderWorkerError {
                    reason_code: "render-process-failed",
                    customer_message: safe_render_failure_message(intent),
                    operator_detail: format!(
                        "darktable-cli가 실패했어요: exitCode={} stderr={}",
                        status.code().unwrap_or(-1),
                        sanitize_process_output(&output.stderr)
                    ),
                });
            }
            Ok(None) => {
                if started_at.elapsed() >= DEFAULT_RENDER_TIMEOUT {
                    let _ = child.kill();
                    let output = child.wait_with_output().ok();
                    return Err(RenderWorkerError {
                        reason_code: "render-process-timeout",
                        customer_message: safe_render_failure_message(intent),
                        operator_detail: format!(
                            "darktable-cli가 제한 시간 안에 끝나지 않았어요: timeoutMs={} stderr={}",
                            DEFAULT_RENDER_TIMEOUT.as_millis(),
                            output
                                .as_ref()
                                .map(|value| sanitize_process_output(&value.stderr))
                                .unwrap_or_else(|| "none".into())
                        ),
                    });
                }

                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                let _ = child.kill();
                return Err(RenderWorkerError {
                    reason_code: "render-process-state-unavailable",
                    customer_message: safe_render_failure_message(intent),
                    operator_detail: format!("render 프로세스 상태를 확인하지 못했어요: {error}"),
                });
            }
        }
    }
}

fn validate_render_output(
    output_path: &Path,
    intent: RenderIntent,
) -> Result<(), RenderWorkerError> {
    if !output_path.is_file() {
        return Err(RenderWorkerError {
            reason_code: "render-output-missing",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "render 출력 파일이 존재하지 않아요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    let metadata = fs::metadata(output_path).map_err(|error| RenderWorkerError {
        reason_code: "render-output-unreadable",
        customer_message: safe_render_failure_message(intent),
        operator_detail: format!("render 출력 파일 metadata를 읽지 못했어요: {error}"),
    })?;

    if metadata.len() == 0 {
        return Err(RenderWorkerError {
            reason_code: "render-output-empty",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "render 출력 파일이 비어 있어요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    if !is_valid_render_preview_asset(output_path) {
        return Err(RenderWorkerError {
            reason_code: "render-output-invalid",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "render 출력 파일이 유효한 raster 형식이 아니에요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    Ok(())
}

fn has_jpeg_signature(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };

    bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF
}

fn has_png_signature(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };

    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

fn sanitize_process_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();

    if trimmed.is_empty() {
        "none".into()
    } else {
        trimmed.replace('\n', " ").replace('\r', " ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "boothy-render-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn darktable_cli_resolution_prefers_env_override() {
        let resolution =
            resolve_darktable_cli_binary_with_candidates(Some("C:/custom/darktable-cli.exe"), &[]);

        assert_eq!(resolution.binary, "C:/custom/darktable-cli.exe");
        assert_eq!(resolution.source, "env-override");
    }

    #[test]
    fn darktable_cli_resolution_uses_existing_known_install_path() {
        let temp_dir = unique_temp_dir("known-install");
        let candidate = temp_dir
            .join("darktable")
            .join("bin")
            .join("darktable-cli.exe");
        fs::create_dir_all(
            candidate
                .parent()
                .expect("candidate should have a parent directory"),
        )
        .expect("candidate parent directory should be creatable");
        fs::write(&candidate, "cli").expect("candidate binary should be writable");

        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[("program-files-bin", candidate.clone())],
        );

        assert_eq!(resolution.binary, candidate.to_string_lossy().as_ref());
        assert_eq!(resolution.source, "program-files-bin");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn darktable_cli_resolution_falls_back_to_path_when_no_known_binary_exists() {
        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[(
                "program-files-bin",
                PathBuf::from("C:/missing/darktable-cli.exe"),
            )],
        );

        assert_eq!(resolution.binary, "darktable-cli");
        assert_eq!(resolution.source, "path");
    }

    #[test]
    fn preview_invocation_uses_display_sized_render_arguments() {
        let temp_dir = unique_temp_dir("preview-invocation");
        let invocation = build_darktable_invocation(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("preview.xmp"),
            "C:/captures/originals/capture.cr2",
            &temp_dir
                .join("renders")
                .join("previews")
                .join("capture.jpg"),
            RenderIntent::Preview,
        );

        assert!(invocation.arguments.contains(&"--width".to_string()));
        assert!(invocation
            .arguments
            .contains(&PREVIEW_MAX_WIDTH_PX.to_string()));
        assert!(invocation.arguments.contains(&"--height".to_string()));
        assert!(invocation
            .arguments
            .contains(&PREVIEW_MAX_HEIGHT_PX.to_string()));
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| { pair[0] == "--hq" && pair[1] == "false" }));
    }

    #[test]
    fn final_invocation_keeps_full_resolution_render_arguments() {
        let temp_dir = unique_temp_dir("final-invocation");
        let invocation = build_darktable_invocation(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("final.xmp"),
            "C:/captures/originals/capture.cr2",
            &temp_dir.join("renders").join("finals").join("capture.jpg"),
            RenderIntent::Final,
        );

        assert!(!invocation.arguments.contains(&"--width".to_string()));
        assert!(!invocation.arguments.contains(&"--height".to_string()));
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| { pair[0] == "--hq" && pair[1] == "true" }));
    }
}
