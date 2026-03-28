use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{LazyLock, Mutex},
    time::SystemTime,
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
    let helper_executable = resolve_helper_executable().ok_or(HelperLaunchFailure {
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

    let child = spawn_helper_process(&helper_executable, base_dir, session_id).map_err(|_| {
        HelperLaunchFailure {
            detail_code: "helper-launch-failed",
        }
    })?;
    let _ = clear_helper_status_file(base_dir, session_id);

    *guard = Some(TrackedHelperProcess {
        session_id: session_id.into(),
        child,
    });

    Ok(())
}

fn resolve_helper_executable() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = env::var_os("BOOTHY_CANON_HELPER_EXE").map(PathBuf::from) {
        candidates.push(path);
    }

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let helper_dir = project_root.join(bundled_helper_dir());

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

    candidates.into_iter().find(|path| path.is_file())
}

fn spawn_helper_process(
    helper_executable: &Path,
    base_dir: &Path,
    session_id: &str,
) -> Result<Child, std::io::Error> {
    let mut command = Command::new(helper_executable);
    command
        .arg("--runtime-root")
        .arg(base_dir)
        .arg("--session-id")
        .arg(session_id)
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
