use std::{
    collections::VecDeque,
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{LazyLock, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::json;

use crate::{
    capture::sidecar_client::{
        bundled_helper_dir, CanonHelperStatusMessage, CAMERA_HELPER_STATUS_FILE_NAME,
        CANON_HELPER_STATUS_SCHEMA_VERSION,
    },
    session::{
        session_manifest::{current_timestamp, rfc3339_to_unix_seconds},
        session_paths::SessionPaths,
    },
};

static HELPER_PROCESS: LazyLock<Mutex<Option<TrackedHelperProcess>>> =
    LazyLock::new(|| Mutex::new(None));
const HELPER_POLL_INTERVAL_MS: &str = "250";
const HELPER_STATUS_INTERVAL_MS: &str = "250";
const HELPER_STARTUP_PROBE_DELAY_MS: Duration = Duration::from_millis(200);
const HELPER_CONNECT_STALL_RESTART_AFTER_SECONDS: u64 = 20;
const HELPER_CAPTURE_IN_FLIGHT_STALL_RESTART_AFTER_SECONDS: u64 = 45;
const HELPER_STARTUP_RESTART_WINDOW: Duration = Duration::from_secs(20);
const HELPER_STARTUP_RESTART_LIMIT: usize = 1;
const HELPER_STARTUP_REPEATED_STATUS_FAST_FAIL_AFTER: Duration = Duration::from_secs(3);
const HELPER_STARTUP_REPEATED_STATUS_FAST_FAIL_DELTA: u64 = 10;
const HELPER_STATUS_MAX_AGE_SECONDS: u64 = 5;
const CANON_SDK_ROOT_ENV: &str = "BOOTHY_CANON_SDK_ROOT";

enum HelperLaunchTarget {
    Executable(PathBuf),
    DotnetProject {
        project_path: PathBuf,
        sdk_root: Option<PathBuf>,
    },
}

struct TrackedHelperProcess {
    session_id: String,
    child: Child,
    startup_restart_attempts: VecDeque<SystemTime>,
    startup_phase_tracker: StartupPhaseTracker,
}

#[derive(Default, Clone)]
struct StartupPhaseTracker {
    detail_code: Option<String>,
    started_at: Option<SystemTime>,
    starting_sequence: Option<u64>,
}

struct HelperLaunchFailure {
    detail_code: &'static str,
}

pub fn try_ensure_helper_running(base_dir: &Path, session_id: &str) {
    if let Err(error) = ensure_helper_running(base_dir, session_id) {
        let _ = write_supervisor_failure_status(base_dir, session_id, error.detail_code);
    }
}

pub fn shutdown_helper_process() {
    let Ok(mut guard) = HELPER_PROCESS.lock() else {
        return;
    };

    if let Some(mut tracked) = guard.take() {
        terminate_child(&mut tracked.child);
    }
}

fn ensure_helper_running(base_dir: &Path, session_id: &str) -> Result<(), HelperLaunchFailure> {
    let helper_launch_target = resolve_helper_launch_target().ok_or(HelperLaunchFailure {
        detail_code: "helper-binary-missing",
    })?;
    let mut guard = HELPER_PROCESS.lock().map_err(|_| HelperLaunchFailure {
        detail_code: "helper-supervisor-unavailable",
    })?;
    let (mut startup_restart_attempts, mut startup_phase_tracker) = match guard.as_ref() {
        Some(tracked) if tracked.session_id == session_id => (
            tracked.startup_restart_attempts.clone(),
            tracked.startup_phase_tracker.clone(),
        ),
        _ => (VecDeque::new(), StartupPhaseTracker::default()),
    };

    if let Some(tracked) = guard.as_mut() {
        if tracked.session_id == session_id {
            match tracked.child.try_wait() {
                Ok(None) => {
                    let status = read_helper_status(base_dir, session_id);
                    let restart_detail_code = status
                        .as_ref()
                        .and_then(helper_status_restart_detail_code)
                        .or_else(|| {
                            status.as_ref().and_then(|status| {
                                startup_phase_stall_detail_code(
                                    &mut startup_phase_tracker,
                                    status,
                                    SystemTime::now(),
                                )
                            })
                        });
                    if let Some(restart_detail_code) = restart_detail_code {
                        let restart_observed_at = SystemTime::now();
                        if !helper_startup_restart_allowed(
                            &startup_restart_attempts,
                            restart_observed_at,
                        ) {
                            let mut tracked = guard.take().expect("tracked helper should exist");
                            terminate_child(&mut tracked.child);
                            return Err(HelperLaunchFailure {
                                detail_code: restart_detail_code,
                            });
                        }
                        record_helper_startup_restart(
                            &mut startup_restart_attempts,
                            restart_observed_at,
                        );
                        let mut tracked = guard.take().expect("tracked helper should exist");
                        terminate_child(&mut tracked.child);
                    } else {
                        tracked.startup_phase_tracker = startup_phase_tracker;
                        return Ok(());
                    }
                }
                Ok(Some(_)) | Err(_) => {
                    let restart_observed_at = SystemTime::now();
                    let status = read_helper_status(base_dir, session_id);
                    let restart_detail_code = startup_phase_exit_restart_detail_code(
                        &mut startup_restart_attempts,
                        &startup_phase_tracker,
                        status.as_ref(),
                        restart_observed_at,
                    );
                    let mut tracked = guard.take().expect("tracked helper should exist");
                    terminate_child(&mut tracked.child);
                    if let Some(restart_detail_code) = restart_detail_code {
                        return Err(HelperLaunchFailure {
                            detail_code: restart_detail_code,
                        });
                    }
                }
            }
        } else {
            let mut tracked = guard.take().expect("tracked helper should exist");
            terminate_child(&mut tracked.child);
        }
    }

    terminate_stale_helper_processes(&helper_launch_target, base_dir);
    write_starting_helper_status(base_dir, session_id).map_err(|_| HelperLaunchFailure {
        detail_code: "helper-launch-failed",
    })?;

    let child = spawn_compatible_helper_process(&helper_launch_target, base_dir, session_id)
        .map_err(|_| HelperLaunchFailure {
            detail_code: "helper-launch-failed",
        })?;

    *guard = Some(TrackedHelperProcess {
        session_id: session_id.into(),
        child,
        startup_restart_attempts,
        startup_phase_tracker: StartupPhaseTracker::default(),
    });

    Ok(())
}

fn resolve_helper_launch_target() -> Option<HelperLaunchTarget> {
    let helper_dir = resolve_helper_dir();
    let current_exe_dir = env::current_exe()
        .ok()
        .and_then(|current_exe| current_exe.parent().map(Path::to_path_buf));
    let env_override = env::var_os("BOOTHY_CANON_HELPER_EXE").map(PathBuf::from);

    resolve_helper_launch_target_from(
        &helper_dir,
        current_exe_dir.as_deref(),
        dotnet_available(),
        env_override.as_deref(),
    )
}

fn resolve_helper_launch_target_from(
    helper_dir: &Path,
    current_exe_dir: Option<&Path>,
    dotnet_is_available: bool,
    env_override: Option<&Path>,
) -> Option<HelperLaunchTarget> {
    let mut candidates = Vec::new();

    if let Some(path) = env_override {
        candidates.push(path.to_path_buf());
    }

    let helper_project_path = helper_dir.join("src/CanonHelper/CanonHelper.csproj");
    let can_launch_dotnet_project = helper_project_path.is_file() && dotnet_is_available;

    if can_launch_dotnet_project {
        return Some(HelperLaunchTarget::DotnetProject {
            project_path: helper_project_path,
            sdk_root: resolve_canon_sdk_root(&helper_dir),
        });
    }

    candidates.push(helper_dir.join("canon-helper.exe"));
    candidates.push(
        helper_dir.join("src/CanonHelper/bin/Release/net8.0/win-x64/publish/canon-helper.exe"),
    );
    candidates.push(helper_dir.join("src/CanonHelper/bin/Debug/net8.0/canon-helper.exe"));

    if let Some(current_dir) = current_exe_dir {
        candidates.push(current_dir.join("canon-helper.exe"));
        candidates.push(current_dir.join("canon-helper/canon-helper.exe"));
        candidates.push(current_dir.join("sidecar/canon-helper/canon-helper.exe"));
    }

    if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
        return Some(HelperLaunchTarget::Executable(path));
    }

    None
}

fn resolve_helper_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .join(bundled_helper_dir())
}

fn dotnet_available() -> bool {
    Command::new("dotnet")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn resolve_canon_sdk_root(helper_dir: &Path) -> Option<PathBuf> {
    if let Some(path) = env::var_os(CANON_SDK_ROOT_ENV).map(PathBuf::from) {
        if is_valid_canon_sdk_root(&path) {
            return Some(path);
        }
    }

    let vendor_root = helper_dir.join("vendor/canon-edsdk");
    if is_valid_canon_sdk_root(&vendor_root) {
        return Some(vendor_root);
    }

    if let Some(path) = read_canon_sdk_root_from_vendor_readme(helper_dir) {
        return Some(path);
    }

    find_canon_sdk_root_in_known_workspace()
}

fn read_canon_sdk_root_from_vendor_readme(helper_dir: &Path) -> Option<PathBuf> {
    let readme = fs::read_to_string(helper_dir.join("vendor/README.md")).ok()?;

    readme.lines().find_map(|line| {
        let candidate = line
            .trim()
            .strip_prefix("- Source folder selected: `")?
            .strip_suffix('`')?;
        let path = PathBuf::from(candidate);

        if is_valid_canon_sdk_root(&path) {
            Some(path)
        } else {
            None
        }
    })
}

fn find_canon_sdk_root_in_known_workspace() -> Option<PathBuf> {
    let sdk_workspace = PathBuf::from(r"C:\Code\cannon_sdk");
    let mut candidates = fs::read_dir(sdk_workspace)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() || !is_valid_canon_sdk_root(&path) {
                return None;
            }

            let modified_at = entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            Some((modified_at, path))
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|(modified_at, _)| *modified_at);
    candidates.pop().map(|(_, path)| path)
}

fn is_valid_canon_sdk_root(path: &Path) -> bool {
    path.join("Windows/Sample/CSharp/CameraControl/CameraControl/EDSDK.cs")
        .is_file()
        && path.join("Windows/EDSDK_64/Dll/EDSDK.dll").is_file()
}

fn spawn_compatible_helper_process(
    helper_launch_target: &HelperLaunchTarget,
    base_dir: &Path,
    session_id: &str,
) -> Result<Child, std::io::Error> {
    let mut child = spawn_helper_process(helper_launch_target, base_dir, session_id, true)?;

    if helper_exited_immediately(&mut child)? {
        child = spawn_helper_process(helper_launch_target, base_dir, session_id, false)?;

        if helper_exited_immediately(&mut child)? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "helper exited during startup",
            ));
        }
    }

    Ok(child)
}

fn helper_exited_immediately(child: &mut Child) -> Result<bool, std::io::Error> {
    thread::sleep(HELPER_STARTUP_PROBE_DELAY_MS);

    Ok(child.try_wait()?.is_some())
}

fn spawn_helper_process(
    helper_launch_target: &HelperLaunchTarget,
    base_dir: &Path,
    session_id: &str,
    use_fast_status_args: bool,
) -> Result<Child, std::io::Error> {
    let mut command = match helper_launch_target {
        HelperLaunchTarget::Executable(helper_executable) => Command::new(helper_executable),
        HelperLaunchTarget::DotnetProject {
            project_path,
            sdk_root,
        } => {
            let mut dotnet = Command::new("dotnet");
            dotnet
                .arg("run")
                .arg("--project")
                .arg(project_path)
                .arg("--no-launch-profile")
                .arg("--");

            if let Some(path) = sdk_root {
                dotnet.env(CANON_SDK_ROOT_ENV, path);
            }

            dotnet
        }
    };

    command
        .arg("--runtime-root")
        .arg(base_dir)
        .arg("--session-id")
        .arg(session_id)
        .arg("--parent-pid")
        .arg(std::process::id().to_string());

    if use_fast_status_args {
        command
            .arg("--poll-interval-ms")
            .arg(HELPER_POLL_INTERVAL_MS)
            .arg("--status-interval-ms")
            .arg(HELPER_STATUS_INTERVAL_MS);
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(0x08000000);
    }

    command.spawn()
}

#[cfg(windows)]
fn terminate_stale_helper_processes(helper_launch_target: &HelperLaunchTarget, base_dir: &Path) {
    let script = build_stale_helper_cleanup_script(helper_launch_target, base_dir);
    if script.is_empty() {
        return;
    }

    let _ = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    thread::sleep(HELPER_STARTUP_PROBE_DELAY_MS);
}

#[cfg(not(windows))]
fn terminate_stale_helper_processes(_helper_launch_target: &HelperLaunchTarget, _base_dir: &Path) {}

#[cfg(windows)]
fn build_stale_helper_cleanup_script(
    helper_launch_target: &HelperLaunchTarget,
    base_dir: &Path,
) -> String {
    let runtime_root = powershell_single_quote_literal(&base_dir.to_string_lossy());

    match helper_launch_target {
        HelperLaunchTarget::Executable(helper_executable) => {
            let helper_path = powershell_single_quote_literal(&helper_executable.to_string_lossy());

            format!(
                "$runtimeRoot = '{runtime_root}'\n\
$helperPath = '{helper_path}'\n\
Get-CimInstance Win32_Process |\n\
  Where-Object {{\n\
    $_.CommandLine -like '*--runtime-root*' -and\n\
    $_.CommandLine -like ('*' + $runtimeRoot + '*') -and\n\
    ($_.ExecutablePath -eq $helperPath -or $_.Name -eq 'canon-helper.exe')\n\
  }} |\n\
  ForEach-Object {{\n\
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue\n\
  }}\n"
            )
        }
        HelperLaunchTarget::DotnetProject { project_path, .. } => {
            let project_path = powershell_single_quote_literal(&project_path.to_string_lossy());

            format!(
                "$runtimeRoot = '{runtime_root}'\n\
$projectPath = '{project_path}'\n\
Get-CimInstance Win32_Process |\n\
  Where-Object {{\n\
    $_.Name -eq 'dotnet.exe' -and\n\
    $_.CommandLine -like '*--runtime-root*' -and\n\
    $_.CommandLine -like ('*' + $runtimeRoot + '*') -and\n\
    $_.CommandLine -like ('*' + $projectPath + '*')\n\
  }} |\n\
  ForEach-Object {{\n\
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue\n\
  }}\n"
            )
        }
    }
}

#[cfg(windows)]
fn powershell_single_quote_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn write_supervisor_failure_status(
    base_dir: &Path,
    session_id: &str,
    detail_code: &str,
) -> Result<(), std::io::Error> {
    let paths = SessionPaths::try_new(base_dir, session_id)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.message))?;
    let observed_at = current_timestamp(SystemTime::now())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.message))?;
    let payload = json!({
        "schemaVersion": CANON_HELPER_STATUS_SCHEMA_VERSION,
        "type": "camera-status",
        "sessionId": session_id,
        "sequence": 0,
        "observedAt": observed_at,
        "cameraState": "error",
        "helperState": "error",
        "detailCode": detail_code
    });

    fs::create_dir_all(&paths.diagnostics_dir)?;
    fs::write(
        paths.diagnostics_dir.join(CAMERA_HELPER_STATUS_FILE_NAME),
        serde_json::to_vec_pretty(&payload)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?,
    )?;

    Ok(())
}

fn read_helper_status(base_dir: &Path, session_id: &str) -> Option<CanonHelperStatusMessage> {
    let status_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_STATUS_FILE_NAME);
    let Ok(bytes) = fs::read(status_path) else {
        return None;
    };
    serde_json::from_slice::<CanonHelperStatusMessage>(&bytes).ok()
}

#[cfg(test)]
fn helper_status_requests_restart(base_dir: &Path, session_id: &str) -> bool {
    read_helper_status(base_dir, session_id)
        .as_ref()
        .and_then(helper_status_restart_detail_code)
        .is_some()
}

fn helper_status_restart_detail_code(status: &CanonHelperStatusMessage) -> Option<&'static str> {
    if status.camera_state == "capturing"
        && status.helper_state == "healthy"
        && status.detail_code.as_deref() == Some("capture-in-flight")
        && helper_status_age_seconds(status)
            .is_some_and(|age| age >= HELPER_CAPTURE_IN_FLIGHT_STALL_RESTART_AFTER_SECONDS)
    {
        return Some("capture-in-flight-timeout");
    }

    if status.camera_state == "error" && status.helper_state == "error" {
        return match status.detail_code.as_deref() {
            Some("camera-connect-timeout") => Some("camera-connect-timeout"),
            Some("sdk-init-timeout") => Some("sdk-init-timeout"),
            Some("session-open-timeout") => Some("session-open-timeout"),
            _ => None,
        };
    }

    None
}

fn helper_status_age_seconds(status: &CanonHelperStatusMessage) -> Option<u64> {
    let observed_at_seconds = rfc3339_to_unix_seconds(&status.observed_at).ok()?;
    let now_duration = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;

    Some(now_duration.as_secs().saturating_sub(observed_at_seconds))
}

fn startup_phase_stall_detail_code(
    tracker: &mut StartupPhaseTracker,
    status: &CanonHelperStatusMessage,
    now: SystemTime,
) -> Option<&'static str> {
    if let Some(detail_code) = startup_phase_detail_code(status) {
        tracker.detail_code = Some(detail_code.into());
        if tracker.started_at.is_none() {
            tracker.started_at = Some(now);
            tracker.starting_sequence = status.sequence;
        }
    } else if !startup_phase_bridge_status(status) {
        tracker.detail_code = None;
        tracker.started_at = None;
        tracker.starting_sequence = None;
        return None;
    }

    let Some(started_at) = tracker.started_at else {
        return None;
    };

    let elapsed = now.duration_since(started_at).unwrap_or_default();
    if startup_phase_repeated_status_fast_fail(tracker, status, elapsed) {
        return Some(startup_phase_timeout_detail_code(
            tracker.detail_code.as_deref(),
        ));
    }

    if elapsed.as_secs() < HELPER_CONNECT_STALL_RESTART_AFTER_SECONDS {
        return None;
    }

    Some(startup_phase_timeout_detail_code(
        tracker.detail_code.as_deref(),
    ))
}

fn startup_phase_detail_code(status: &CanonHelperStatusMessage) -> Option<&str> {
    match (
        status.camera_state.as_str(),
        status.helper_state.as_str(),
        status.detail_code.as_deref(),
    ) {
        ("connecting", "starting", Some("helper-starting" | "sdk-initializing" | "scanning")) => {
            status.detail_code.as_deref()
        }
        ("connecting", "connecting", Some("session-opening")) => Some("session-opening"),
        _ => None,
    }
}

fn startup_phase_bridge_status(status: &CanonHelperStatusMessage) -> bool {
    matches!(
        (
            status.camera_state.as_str(),
            status.helper_state.as_str(),
            status.detail_code.as_deref(),
        ),
        ("connecting", "healthy", Some("windows-device-detected"))
    )
}

fn startup_phase_repeated_status_fast_fail(
    tracker: &StartupPhaseTracker,
    status: &CanonHelperStatusMessage,
    elapsed: Duration,
) -> bool {
    if elapsed < HELPER_STARTUP_REPEATED_STATUS_FAST_FAIL_AFTER {
        return false;
    }

    let Some(starting_sequence) = tracker.starting_sequence else {
        return false;
    };
    let Some(current_sequence) = status.sequence else {
        return false;
    };

    current_sequence.saturating_sub(starting_sequence)
        >= HELPER_STARTUP_REPEATED_STATUS_FAST_FAIL_DELTA
}

fn startup_phase_exit_restart_detail_code(
    restart_attempts: &mut VecDeque<SystemTime>,
    tracker: &StartupPhaseTracker,
    status: Option<&CanonHelperStatusMessage>,
    now: SystemTime,
) -> Option<&'static str> {
    if !startup_phase_exit_requires_restart(tracker, status) {
        return None;
    }

    if helper_startup_restart_allowed(restart_attempts, now) {
        record_helper_startup_restart(restart_attempts, now);
        return None;
    }

    Some(startup_phase_timeout_detail_code(
        tracker
            .detail_code
            .as_deref()
            .or_else(|| status.and_then(startup_phase_detail_code)),
    ))
}

fn startup_phase_exit_requires_restart(
    tracker: &StartupPhaseTracker,
    status: Option<&CanonHelperStatusMessage>,
) -> bool {
    tracker.started_at.is_some()
        || status.is_some_and(|status| {
            startup_phase_detail_code(status).is_some() || startup_phase_bridge_status(status)
        })
}

fn startup_phase_timeout_detail_code(detail_code: Option<&str>) -> &'static str {
    match detail_code {
        Some("session-opening") => "session-open-timeout",
        Some(_) => "sdk-init-timeout",
        None => "camera-connect-timeout",
    }
}

fn helper_startup_restart_allowed(
    restart_attempts: &VecDeque<SystemTime>,
    now: SystemTime,
) -> bool {
    recent_helper_startup_restart_count(restart_attempts, now) < HELPER_STARTUP_RESTART_LIMIT
}

fn record_helper_startup_restart(restart_attempts: &mut VecDeque<SystemTime>, now: SystemTime) {
    prune_expired_helper_startup_restarts(restart_attempts, now);
    restart_attempts.push_back(now);
}

fn recent_helper_startup_restart_count(
    restart_attempts: &VecDeque<SystemTime>,
    now: SystemTime,
) -> usize {
    restart_attempts
        .iter()
        .filter(|attempted_at| helper_startup_restart_is_within_window(**attempted_at, now))
        .count()
}

fn prune_expired_helper_startup_restarts(
    restart_attempts: &mut VecDeque<SystemTime>,
    now: SystemTime,
) {
    while let Some(oldest_attempt) = restart_attempts.front().copied() {
        if helper_startup_restart_is_within_window(oldest_attempt, now) {
            break;
        }

        restart_attempts.pop_front();
    }
}

fn helper_startup_restart_is_within_window(attempted_at: SystemTime, now: SystemTime) -> bool {
    now.duration_since(attempted_at)
        .map(|elapsed| elapsed <= HELPER_STARTUP_RESTART_WINDOW)
        .unwrap_or(true)
}

fn write_starting_helper_status(base_dir: &Path, session_id: &str) -> Result<(), std::io::Error> {
    if read_helper_status(base_dir, session_id)
        .as_ref()
        .is_some_and(should_preserve_existing_helper_status)
    {
        return Ok(());
    }

    let paths = SessionPaths::try_new(base_dir, session_id)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.message))?;
    let observed_at = current_timestamp(SystemTime::now())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.message))?;
    let payload = json!({
        "schemaVersion": CANON_HELPER_STATUS_SCHEMA_VERSION,
        "type": "camera-status",
        "sessionId": session_id,
        "sequence": 0,
        "observedAt": observed_at,
        "cameraState": "connecting",
        "helperState": "starting",
        "detailCode": "helper-starting"
    });

    fs::create_dir_all(&paths.diagnostics_dir)?;
    fs::write(
        paths.diagnostics_dir.join(CAMERA_HELPER_STATUS_FILE_NAME),
        serde_json::to_vec_pretty(&payload)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?,
    )?;

    Ok(())
}

fn should_preserve_existing_helper_status(status: &CanonHelperStatusMessage) -> bool {
    helper_status_restart_detail_code(status).is_some()
        || (startup_phase_status(status) && !helper_status_is_fresh(status))
}

fn startup_phase_status(status: &CanonHelperStatusMessage) -> bool {
    startup_phase_detail_code(status).is_some() || startup_phase_bridge_status(status)
}

fn helper_status_is_fresh(status: &CanonHelperStatusMessage) -> bool {
    let Ok(observed_at_seconds) = rfc3339_to_unix_seconds(&status.observed_at) else {
        return false;
    };
    let Ok(now_duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return false;
    };

    now_duration.as_secs().saturating_sub(observed_at_seconds) <= HELPER_STATUS_MAX_AGE_SECONDS
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn executable_cleanup_script_targets_runtime_root_and_helper_path() {
        let script = build_stale_helper_cleanup_script(
            &HelperLaunchTarget::Executable(PathBuf::from(
                r"C:\Code\Project\Boothy\sidecar\canon-helper\canon-helper.exe",
            )),
            Path::new(r"C:\Users\KimYS\Pictures\dabi_shoot"),
        );

        assert!(script.contains("$runtimeRoot = 'C:\\Users\\KimYS\\Pictures\\dabi_shoot'"));
        assert!(script.contains(
            "$helperPath = 'C:\\Code\\Project\\Boothy\\sidecar\\canon-helper\\canon-helper.exe'"
        ));
        assert!(script.contains("$_.ExecutablePath -eq $helperPath"));
        assert!(script.contains("$_.CommandLine -like '*--runtime-root*'"));
    }

    #[test]
    fn dotnet_cleanup_script_targets_helper_project_processes() {
        let script = build_stale_helper_cleanup_script(
            &HelperLaunchTarget::DotnetProject {
                project_path: PathBuf::from(
                    r"C:\Code\Project\Boothy\sidecar\canon-helper\src\CanonHelper\CanonHelper.csproj",
                ),
                sdk_root: None,
            },
            Path::new(r"C:\Users\KimYS\Pictures\dabi_shoot"),
        );

        assert!(script.contains("$_.Name -eq 'dotnet.exe'"));
        assert!(
            script.contains(
                "$projectPath = 'C:\\Code\\Project\\Boothy\\sidecar\\canon-helper\\src\\CanonHelper\\CanonHelper.csproj'"
            )
        );
        assert!(script.contains("$_.CommandLine -like ('*' + $projectPath + '*')"));
    }

    #[test]
    fn powershell_literals_escape_single_quotes() {
        assert_eq!(
            powershell_single_quote_literal("C:\\Users\\Kim'YS"),
            "C:\\Users\\Kim''YS"
        );
    }

    #[test]
    fn debug_helper_source_is_preferred_over_stale_local_executable() {
        let helper_dir = std::env::temp_dir().join(format!(
            "boothy-helper-launch-target-{}",
            std::process::id()
        ));
        let project_path = helper_dir.join("src/CanonHelper/CanonHelper.csproj");
        let helper_exe_path = helper_dir.join("src/CanonHelper/bin/Debug/net8.0/canon-helper.exe");
        let _ = fs::remove_dir_all(&helper_dir);
        fs::create_dir_all(
            project_path
                .parent()
                .expect("helper project path should have a parent"),
        )
        .expect("helper project directory should exist");
        fs::create_dir_all(
            helper_exe_path
                .parent()
                .expect("helper exe path should have a parent"),
        )
        .expect("helper exe directory should exist");
        fs::write(&project_path, "<Project />").expect("helper project should be writable");
        fs::write(&helper_exe_path, b"stub").expect("helper exe should be writable");

        let target = resolve_helper_launch_target_from(&helper_dir, None, true, None)
            .expect("helper launch target should resolve");

        match target {
            HelperLaunchTarget::Executable(path) => {
                panic!("stale helper executable should not win over current source: {path:?}")
            }
            HelperLaunchTarget::DotnetProject {
                project_path: resolved_project_path,
                ..
            } => assert_eq!(resolved_project_path, project_path),
        }

        let _ = fs::remove_dir_all(&helper_dir);
    }

    #[test]
    fn starting_helper_status_is_written_before_live_status_arrives() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-helper-supervisor-status-{}",
            std::process::id()
        ));
        let session_id = "session_00000000000000000000000001";
        let status_path = SessionPaths::try_new(&base_dir, session_id)
            .expect("session paths should resolve")
            .diagnostics_dir
            .join(CAMERA_HELPER_STATUS_FILE_NAME);
        let _ = fs::remove_dir_all(&base_dir);

        write_starting_helper_status(&base_dir, session_id)
            .expect("starting helper status should be writable");

        let payload: serde_json::Value = serde_json::from_slice(
            &fs::read(&status_path).expect("starting helper status should exist"),
        )
        .expect("starting helper status should deserialize");

        assert_eq!(payload["cameraState"], "connecting");
        assert_eq!(payload["helperState"], "starting");
        assert_eq!(payload["detailCode"], "helper-starting");

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn starting_helper_status_does_not_overwrite_a_stale_startup_failure_status() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-helper-supervisor-preserve-stale-startup-{}",
            std::process::id()
        ));
        let session_id = "session_00000000000000000000000009";
        let status_path = SessionPaths::try_new(&base_dir, session_id)
            .expect("session paths should resolve")
            .diagnostics_dir
            .join(CAMERA_HELPER_STATUS_FILE_NAME);
        let _ = fs::remove_dir_all(&base_dir);
        fs::create_dir_all(
            status_path
                .parent()
                .expect("status path should have a parent"),
        )
        .expect("diagnostics directory should exist");

        let stale_status = serde_json::json!({
            "schemaVersion": CANON_HELPER_STATUS_SCHEMA_VERSION,
            "type": "camera-status",
            "sessionId": session_id,
            "sequence": 2,
            "observedAt": current_timestamp(
                SystemTime::now()
                    .checked_sub(Duration::from_secs(30))
                    .expect("stale startup timestamp should compute"),
            )
            .expect("stale startup timestamp should serialize"),
            "cameraState": "connecting",
            "helperState": "starting",
            "cameraModel": serde_json::Value::Null,
            "requestId": serde_json::Value::Null,
            "detailCode": "sdk-initializing"
        });
        fs::write(
            &status_path,
            serde_json::to_vec_pretty(&stale_status).expect("status should serialize"),
        )
        .expect("stale status should be writable");

        write_starting_helper_status(&base_dir, session_id)
            .expect("starting helper status should not fail");

        let payload: serde_json::Value =
            serde_json::from_slice(&fs::read(&status_path).expect("status should exist"))
                .expect("status should deserialize");

        assert_eq!(payload["detailCode"], "sdk-initializing");
        assert_eq!(payload["helperState"], "starting");

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn starting_helper_status_does_not_overwrite_a_recorded_startup_timeout() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-helper-supervisor-preserve-timeout-{}",
            std::process::id()
        ));
        let session_id = "session_00000000000000000000000010";
        let status_path = SessionPaths::try_new(&base_dir, session_id)
            .expect("session paths should resolve")
            .diagnostics_dir
            .join(CAMERA_HELPER_STATUS_FILE_NAME);
        let _ = fs::remove_dir_all(&base_dir);

        write_supervisor_failure_status(&base_dir, session_id, "sdk-init-timeout")
            .expect("timeout status should be writable");

        write_starting_helper_status(&base_dir, session_id)
            .expect("starting helper status should not fail");

        let payload: serde_json::Value =
            serde_json::from_slice(&fs::read(&status_path).expect("status should exist"))
                .expect("status should deserialize");

        assert_eq!(payload["detailCode"], "sdk-init-timeout");
        assert_eq!(payload["cameraState"], "error");
        assert_eq!(payload["helperState"], "error");

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn connect_timeout_status_requests_a_helper_restart() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-helper-supervisor-restart-{}",
            std::process::id()
        ));
        let session_id = "session_00000000000000000000000002";
        let _ = fs::remove_dir_all(&base_dir);

        write_supervisor_failure_status(&base_dir, session_id, "camera-connect-timeout")
            .expect("connect-timeout status should be writable");

        assert!(helper_status_requests_restart(&base_dir, session_id));

        write_supervisor_failure_status(&base_dir, session_id, "sdk-init-timeout")
            .expect("sdk-init-timeout status should be writable");

        assert!(helper_status_requests_restart(&base_dir, session_id));

        write_supervisor_failure_status(&base_dir, session_id, "session-open-timeout")
            .expect("session-open-timeout status should be writable");

        assert!(helper_status_requests_restart(&base_dir, session_id));

        write_supervisor_failure_status(&base_dir, session_id, "session-open-failed")
            .expect("session-open-failed status should be writable");

        assert!(!helper_status_requests_restart(&base_dir, session_id));

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn stale_capture_in_flight_status_requests_a_helper_restart() {
        let session_id = "session_00000000000000000000000002";
        let stale_status = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: session_id.into(),
            sequence: Some(24),
            observed_at: current_timestamp(
                SystemTime::now()
                    .checked_sub(Duration::from_secs(
                        HELPER_CAPTURE_IN_FLIGHT_STALL_RESTART_AFTER_SECONDS + 1,
                    ))
                    .expect("stale capture-in-flight timestamp should compute"),
            )
            .expect("helper timestamp should serialize"),
            camera_state: "capturing".into(),
            helper_state: "healthy".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: Some("request_stalled".into()),
            detail_code: Some("capture-in-flight".into()),
        };

        assert_eq!(
            helper_status_restart_detail_code(&stale_status),
            Some("capture-in-flight-timeout")
        );

        let fresh_status = CanonHelperStatusMessage {
            observed_at: current_timestamp(SystemTime::now())
                .expect("fresh helper timestamp should serialize"),
            ..stale_status
        };

        assert_eq!(helper_status_restart_detail_code(&fresh_status), None);
    }

    #[test]
    fn prolonged_session_opening_status_escalates_via_tracker_even_while_status_is_fresh() {
        let mut tracker = StartupPhaseTracker::default();
        let now = SystemTime::now();
        let status = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000003".into(),
            sequence: Some(5),
            observed_at: current_timestamp(now).expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "connecting".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("session-opening".into()),
        };

        assert_eq!(
            startup_phase_stall_detail_code(&mut tracker, &status, now),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &status,
                now.checked_add(Duration::from_secs(
                    HELPER_CONNECT_STALL_RESTART_AFTER_SECONDS + 1
                ))
                .expect("stall timestamp should compute"),
            ),
            Some("session-open-timeout")
        );
    }

    #[test]
    fn windows_device_detected_between_session_opening_updates_does_not_reset_stall_tracking() {
        let mut tracker = StartupPhaseTracker::default();
        let now = SystemTime::now();
        let session_opening = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000004".into(),
            sequence: Some(10),
            observed_at: current_timestamp(now).expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "connecting".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("session-opening".into()),
        };
        let windows_device_detected = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000004".into(),
            sequence: Some(11),
            observed_at: current_timestamp(
                now.checked_add(Duration::from_secs(10))
                    .expect("bridge timestamp should compute"),
            )
            .expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "healthy".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("windows-device-detected".into()),
        };

        assert_eq!(
            startup_phase_stall_detail_code(&mut tracker, &session_opening, now),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &windows_device_detected,
                now.checked_add(Duration::from_secs(10))
                    .expect("bridge timestamp should compute"),
            ),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &session_opening,
                now.checked_add(Duration::from_secs(
                    HELPER_CONNECT_STALL_RESTART_AFTER_SECONDS + 1
                ))
                .expect("stall timestamp should compute"),
            ),
            Some("session-open-timeout")
        );
    }

    #[test]
    fn alternating_startup_phases_keep_the_original_stall_budget() {
        let mut tracker = StartupPhaseTracker::default();
        let now = SystemTime::now();
        let sdk_initializing = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000005".into(),
            sequence: Some(20),
            observed_at: current_timestamp(now).expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "starting".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("sdk-initializing".into()),
        };
        let windows_device_detected = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000005".into(),
            sequence: Some(21),
            observed_at: current_timestamp(
                now.checked_add(Duration::from_secs(8))
                    .expect("bridge timestamp should compute"),
            )
            .expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "healthy".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("windows-device-detected".into()),
        };
        let session_opening = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000005".into(),
            sequence: Some(22),
            observed_at: current_timestamp(
                now.checked_add(Duration::from_secs(
                    HELPER_CONNECT_STALL_RESTART_AFTER_SECONDS + 1,
                ))
                .expect("stall timestamp should compute"),
            )
            .expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "connecting".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("session-opening".into()),
        };

        assert_eq!(
            startup_phase_stall_detail_code(&mut tracker, &sdk_initializing, now),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &windows_device_detected,
                now.checked_add(Duration::from_secs(8))
                    .expect("bridge timestamp should compute"),
            ),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &session_opening,
                now.checked_add(Duration::from_secs(
                    HELPER_CONNECT_STALL_RESTART_AFTER_SECONDS + 1,
                ))
                .expect("stall timestamp should compute"),
            ),
            Some("session-open-timeout")
        );
    }

    #[test]
    fn repeated_startup_status_updates_fast_fail_before_the_twenty_second_budget() {
        let mut tracker = StartupPhaseTracker::default();
        let now = SystemTime::now();
        let session_opening = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000006".into(),
            sequence: Some(1),
            observed_at: current_timestamp(now).expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "connecting".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("session-opening".into()),
        };
        let repeated_bridge = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000006".into(),
            sequence: Some(15),
            observed_at: current_timestamp(
                now.checked_add(Duration::from_secs(4))
                    .expect("fast-fail timestamp should compute"),
            )
            .expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "healthy".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("windows-device-detected".into()),
        };

        assert_eq!(
            startup_phase_stall_detail_code(&mut tracker, &session_opening, now),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &repeated_bridge,
                now.checked_add(Duration::from_secs(4))
                    .expect("fast-fail timestamp should compute"),
            ),
            Some("session-open-timeout")
        );
    }

    #[test]
    fn dense_startup_burst_fast_fails_after_ten_repeated_transitions() {
        let mut tracker = StartupPhaseTracker::default();
        let now = SystemTime::now();
        let sdk_initializing = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000008".into(),
            sequence: Some(1),
            observed_at: current_timestamp(now).expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "starting".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("sdk-initializing".into()),
        };
        let repeated_bridge = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000008".into(),
            sequence: Some(11),
            observed_at: current_timestamp(
                now.checked_add(Duration::from_secs(3))
                    .expect("fast-fail timestamp should compute"),
            )
            .expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "healthy".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("windows-device-detected".into()),
        };

        assert_eq!(
            startup_phase_stall_detail_code(&mut tracker, &sdk_initializing, now),
            None
        );
        assert_eq!(
            startup_phase_stall_detail_code(
                &mut tracker,
                &repeated_bridge,
                now.checked_add(Duration::from_secs(3))
                    .expect("fast-fail timestamp should compute"),
            ),
            Some("sdk-init-timeout")
        );
    }

    #[test]
    fn startup_restart_budget_allows_the_first_retry() {
        let restart_attempts = std::collections::VecDeque::new();

        assert!(helper_startup_restart_allowed(
            &restart_attempts,
            SystemTime::now()
        ));
    }

    #[test]
    fn startup_restart_budget_blocks_repeated_retries_inside_the_budget_window() {
        let now = SystemTime::now();
        let restart_attempts = std::collections::VecDeque::from([now]);

        assert!(!helper_startup_restart_allowed(&restart_attempts, now));
    }

    #[test]
    fn startup_restart_budget_reopens_after_the_window_expires() {
        let now = SystemTime::now();
        let restart_attempts = std::collections::VecDeque::from([now
            .checked_sub(HELPER_STARTUP_RESTART_WINDOW + Duration::from_secs(1))
            .expect("restart attempt timestamp should compute")]);

        assert!(helper_startup_restart_allowed(&restart_attempts, now));
    }

    #[test]
    fn startup_exit_during_windows_device_detected_consumes_restart_budget_then_escalates() {
        let mut restart_attempts = std::collections::VecDeque::new();
        let now = SystemTime::now();
        let tracker = StartupPhaseTracker {
            detail_code: Some("session-opening".into()),
            started_at: Some(
                now.checked_sub(Duration::from_secs(5))
                    .expect("startup timestamp should compute"),
            ),
            starting_sequence: Some(1),
        };
        let status = CanonHelperStatusMessage {
            schema_version: CANON_HELPER_STATUS_SCHEMA_VERSION.into(),
            message_type: Some("camera-status".into()),
            session_id: "session_00000000000000000000000007".into(),
            sequence: Some(21),
            observed_at: current_timestamp(now).expect("helper timestamp should serialize"),
            camera_state: "connecting".into(),
            helper_state: "healthy".into(),
            camera_model: Some("Canon EOS 700D".into()),
            request_id: None,
            detail_code: Some("windows-device-detected".into()),
        };

        assert_eq!(
            startup_phase_exit_restart_detail_code(
                &mut restart_attempts,
                &tracker,
                Some(&status),
                now,
            ),
            None
        );
        assert_eq!(restart_attempts.len(), 1);
        assert_eq!(
            startup_phase_exit_restart_detail_code(
                &mut restart_attempts,
                &tracker,
                Some(&status),
                now.checked_add(Duration::from_secs(5))
                    .expect("restart timestamp should compute"),
            ),
            Some("session-open-timeout")
        );
    }
}
