use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{LazyLock, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use serde_json::json;

use crate::{
    capture::sidecar_client::{
        bundled_helper_dir, CAMERA_HELPER_STATUS_FILE_NAME, CANON_HELPER_STATUS_SCHEMA_VERSION,
    },
    session::{session_manifest::current_timestamp, session_paths::SessionPaths},
};

static HELPER_PROCESS: LazyLock<Mutex<Option<TrackedHelperProcess>>> =
    LazyLock::new(|| Mutex::new(None));
const HELPER_POLL_INTERVAL_MS: &str = "250";
const HELPER_STATUS_INTERVAL_MS: &str = "250";
const HELPER_STARTUP_PROBE_DELAY_MS: Duration = Duration::from_millis(200);
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

    if let Some(tracked) = guard.as_mut() {
        if tracked.session_id == session_id {
            match tracked.child.try_wait() {
                Ok(None) => return Ok(()),
                Ok(Some(_)) | Err(_) => {
                    let mut tracked = guard.take().expect("tracked helper should exist");
                    terminate_child(&mut tracked.child);
                }
            }
        } else {
            let mut tracked = guard.take().expect("tracked helper should exist");
            terminate_child(&mut tracked.child);
        }
    }

    let child = spawn_compatible_helper_process(&helper_launch_target, base_dir, session_id)
        .map_err(|_| HelperLaunchFailure {
            detail_code: "helper-launch-failed",
        })?;
    let _ = clear_helper_status_file(base_dir, session_id);

    *guard = Some(TrackedHelperProcess {
        session_id: session_id.into(),
        child,
    });

    Ok(())
}

fn resolve_helper_launch_target() -> Option<HelperLaunchTarget> {
    let mut candidates = Vec::new();

    if let Some(path) = env::var_os("BOOTHY_CANON_HELPER_EXE").map(PathBuf::from) {
        candidates.push(path);
    }

    let helper_dir = resolve_helper_dir();
    let helper_project_path = helper_dir.join("src/CanonHelper/CanonHelper.csproj");
    let can_launch_dotnet_project = helper_project_path.is_file() && dotnet_available();

    if cfg!(debug_assertions) && can_launch_dotnet_project {
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

    if let Ok(current_exe) = env::current_exe() {
        if let Some(current_dir) = current_exe.parent() {
            candidates.push(current_dir.join("canon-helper.exe"));
            candidates.push(current_dir.join("canon-helper/canon-helper.exe"));
            candidates.push(current_dir.join("sidecar/canon-helper/canon-helper.exe"));
        }
    }

    if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
        return Some(HelperLaunchTarget::Executable(path));
    }

    if can_launch_dotnet_project {
        return Some(HelperLaunchTarget::DotnetProject {
            project_path: helper_project_path,
            sdk_root: resolve_canon_sdk_root(&helper_dir),
        });
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
        .arg(session_id);

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

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn clear_helper_status_file(base_dir: &Path, session_id: &str) -> Result<(), std::io::Error> {
    let status_path = SessionPaths::try_new(base_dir, session_id)
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_STATUS_FILE_NAME))
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.message))?;

    if status_path.exists() {
        fs::remove_file(status_path)?;
    }

    Ok(())
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
