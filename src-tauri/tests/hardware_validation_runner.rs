use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Once,
    },
    thread,
    time::{Duration, SystemTime},
};

use boothy_lib::{
    automation::hardware_validation::{
        run_hardware_validation_in_dir, AppLaunchMode, HardwareValidationRunInput,
    },
    capture::sidecar_client::{
        read_capture_request_messages, CAMERA_HELPER_STATUS_FILE_NAME,
        CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION, CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        CANON_HELPER_STATUS_SCHEMA_VERSION,
    },
    preset::preset_catalog::resolve_published_preset_catalog_dir,
    session::{
        session_manifest::{current_timestamp, SessionManifest},
        session_paths::SessionPaths,
    },
};

static FAKE_DARKTABLE_SETUP: Once = Once::new();

#[test]
fn hardware_validation_runner_records_a_full_five_capture_pass_with_artifacts() {
    ensure_fake_darktable_cli();
    let base_dir = temp_test_dir("runner-pass");
    let output_dir = base_dir.join("validation-output");
    create_published_bundle(
        &resolve_published_preset_catalog_dir(&base_dir),
        "preset_new-draft-2",
        "2026.04.10",
        "look2",
    );

    let helper_base_dir = base_dir.clone();
    let helper_thread = thread::spawn(move || simulate_capture_helper(&helper_base_dir, 5));

    let result = run_hardware_validation_in_dir(
        &base_dir,
        &output_dir,
        HardwareValidationRunInput {
            prompt: "Kim validation 4821".into(),
            preset_query: "look2".into(),
            capture_count: 5,
            app_launch_mode: AppLaunchMode::Skip,
            phone_last_four: None,
        },
    )
    .expect("runner should finish");

    helper_thread
        .join()
        .expect("helper thread should complete for pass run");

    assert_eq!(result.status, "passed");
    assert_eq!(result.capture_count, 5);
    assert!(result.run_dir.is_dir());

    let summary_path = result.run_dir.join("run-summary.json");
    let steps_path = result.run_dir.join("run-steps.jsonl");
    let artifacts_path = result.run_dir.join("artifacts-index.json");

    assert!(summary_path.is_file());
    assert!(steps_path.is_file());
    assert!(artifacts_path.is_file());

    let summary: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&summary_path).expect("summary should exist"))
            .expect("summary should deserialize");
    assert_eq!(summary["status"], "passed");
    assert_eq!(summary["capturesPassed"], 5);
    assert_eq!(summary["capturesRequested"], 5);
    assert_eq!(summary["preset"]["displayName"], "look2");

    let steps = fs::read_to_string(&steps_path).expect("step log should exist");
    assert!(steps.contains("\"eventType\":\"session-started\""));
    assert!(steps.contains("\"eventType\":\"preset-selected\""));
    assert_eq!(
        steps
            .matches("\"eventType\":\"capture-cycle-passed\"")
            .count(),
        5
    );
}

#[test]
fn hardware_validation_runner_writes_failure_report_when_look2_preset_is_missing() {
    ensure_fake_darktable_cli();
    let base_dir = temp_test_dir("runner-missing-preset");
    let output_dir = base_dir.join("validation-output");

    let result = run_hardware_validation_in_dir(
        &base_dir,
        &output_dir,
        HardwareValidationRunInput {
            prompt: "Preset missing case".into(),
            preset_query: "look2".into(),
            capture_count: 5,
            app_launch_mode: AppLaunchMode::Skip,
            phone_last_four: Some("4821".into()),
        },
    )
    .expect("runner should emit a failed result instead of crashing");

    assert_eq!(result.status, "failed");
    assert!(result.failure_report_path.is_some());

    let failure_report_path = result
        .failure_report_path
        .clone()
        .expect("failed run should include a failure report");
    let failure_report =
        fs::read_to_string(&failure_report_path).expect("failure report should exist");
    assert!(failure_report.contains("preset-not-found"));
    assert!(failure_report.contains("look2"));

    let summary: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(result.run_dir.join("run-summary.json")).expect("summary should exist"),
    )
    .expect("summary should deserialize");
    assert_eq!(summary["status"], "failed");
    assert_eq!(summary["failure"]["code"], "preset-not-found");
}

#[test]
fn hardware_validation_runner_captures_failure_diagnostics_for_readiness_timeouts() {
    ensure_fake_darktable_cli();
    let base_dir = temp_test_dir("runner-readiness-timeout");
    let output_dir = base_dir.join("validation-output");
    create_published_bundle(
        &resolve_published_preset_catalog_dir(&base_dir),
        "preset_new-draft-2",
        "2026.04.10",
        "look2",
    );

    let helper_base_dir = base_dir.clone();
    let stop_helper = Arc::new(AtomicBool::new(false));
    let stop_signal = stop_helper.clone();
    let helper_thread =
        thread::spawn(move || simulate_stuck_connecting_helper(&helper_base_dir, stop_signal));

    let result = run_hardware_validation_in_dir(
        &base_dir,
        &output_dir,
        HardwareValidationRunInput {
            prompt: "Kim4821".into(),
            preset_query: "look2".into(),
            capture_count: 1,
            app_launch_mode: AppLaunchMode::Skip,
            phone_last_four: None,
        },
    )
    .expect("runner should emit a failed result instead of crashing");

    stop_helper.store(true, Ordering::Relaxed);
    helper_thread
        .join()
        .expect("helper thread should complete for timeout run");

    assert_eq!(result.status, "failed");

    let failure_diagnostics_path = result.run_dir.join("failure-diagnostics.json");
    assert!(failure_diagnostics_path.is_file());

    let failure_diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&failure_diagnostics_path).expect("failure diagnostics should exist"),
    )
    .expect("failure diagnostics should deserialize");
    assert_eq!(
        failure_diagnostics["helperStatus"]["detailCode"],
        "session-opening"
    );
    assert_eq!(
        failure_diagnostics["helperStatus"]["cameraModel"],
        "Canon EOS 700D"
    );
    assert!(failure_diagnostics["startupLogTail"]
        .as_array()
        .is_some_and(|lines| !lines.is_empty()));

    let artifacts: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(result.run_dir.join("artifacts-index.json"))
            .expect("artifacts index should exist"),
    )
    .expect("artifacts index should deserialize");
    assert!(artifacts["helperStartupLogPath"].is_string());
    assert!(artifacts["failureDiagnosticsPath"].is_string());

    let failure_report = fs::read_to_string(
        result
            .failure_report_path
            .expect("failed run should include a failure report"),
    )
    .expect("failure report should exist");
    assert!(failure_report.contains("Last readiness snapshot"));
    assert!(failure_report.contains("session-opening"));
    assert!(failure_report.contains("Canon EOS 700D"));
}

fn simulate_capture_helper(base_dir: &Path, expected_requests: usize) {
    let session_id = wait_for_session_id(base_dir);
    let stop_heartbeat = Arc::new(AtomicBool::new(false));
    let heartbeat_base_dir = base_dir.to_path_buf();
    let heartbeat_session_id = session_id.clone();
    let heartbeat_stop = stop_heartbeat.clone();
    let heartbeat_thread = thread::spawn(move || {
        while !heartbeat_stop.load(Ordering::Relaxed) {
            write_ready_helper_status(&heartbeat_base_dir, &heartbeat_session_id);
            thread::sleep(Duration::from_millis(500));
        }
    });

    let mut handled_requests = 0;

    while handled_requests < expected_requests {
        let requests = read_capture_request_messages(base_dir, &session_id)
            .expect("request log should be readable");

        if requests.len() <= handled_requests {
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        let request = requests[handled_requests].clone();
        handled_requests += 1;

        let raw_path = SessionPaths::new(base_dir, &session_id)
            .captures_originals_dir
            .join(format!("capture_{handled_requests}.jpg"));
        fs::create_dir_all(raw_path.parent().expect("raw path should have a parent"))
            .expect("raw dir should exist");
        fs::write(&raw_path, format!("helper-raw-{handled_requests}"))
            .expect("raw file should exist");

        append_helper_event(
            base_dir,
            &session_id,
            serde_json::json!({
                "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
                "type": "capture-accepted",
                "sessionId": request.session_id,
                "requestId": request.request_id,
            }),
        );
        append_helper_event(
            base_dir,
            &session_id,
            serde_json::json!({
                "schemaVersion": CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
                "type": "file-arrived",
                "sessionId": request.session_id,
                "requestId": request.request_id,
                "captureId": format!("capture_{handled_requests:02}"),
                "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
                "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    }

    stop_heartbeat.store(true, Ordering::Relaxed);
    heartbeat_thread
        .join()
        .expect("heartbeat thread should stop cleanly");
}

fn simulate_stuck_connecting_helper(base_dir: &Path, stop_signal: Arc<AtomicBool>) {
    let session_id = wait_for_session_id(base_dir);
    let diagnostics_dir = SessionPaths::new(base_dir, &session_id).diagnostics_dir;
    fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir should exist");

    let startup_log_path = diagnostics_dir.join("camera-helper-startup.log");
    fs::write(
        &startup_log_path,
        [
            "2026-04-21T00:48:15Z\tsequence=1\tcameraState=connecting\thelperState=starting\tdetailCode=sdk-initializing\tcameraModel=\n",
            "2026-04-21T00:48:16Z\tsequence=2\tcameraState=connecting\thelperState=healthy\tdetailCode=windows-device-detected\tcameraModel=Canon EOS 700D\n",
            "2026-04-21T00:48:17Z\tsequence=3\tcameraState=connecting\thelperState=connecting\tdetailCode=session-opening\tcameraModel=Canon EOS 700D\n",
        ]
        .join(""),
    )
    .expect("startup log should be writable");

    let mut sequence = 1u64;
    while !stop_signal.load(Ordering::Relaxed) {
        write_helper_status(base_dir, &session_id, sequence, "connecting", "connecting");
        sequence += 1;
        thread::sleep(Duration::from_millis(200));
    }
}

fn wait_for_session_id(base_dir: &Path) -> String {
    for _ in 0..300 {
        let sessions_root = base_dir.join("sessions");
        if let Ok(entries) = fs::read_dir(&sessions_root) {
            let session_ids = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .filter_map(|path| {
                    path.file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                })
                .filter(|name| name.starts_with("session_"))
                .collect::<Vec<_>>();

            if let Some(session_id) = session_ids.first() {
                return session_id.clone();
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    panic!("runner should create a session before capture starts")
}

fn append_helper_event(base_dir: &Path, session_id: &str, event: serde_json::Value) {
    let event_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("camera-helper-events.jsonl");
    fs::create_dir_all(
        event_path
            .parent()
            .expect("helper event path should have a parent"),
    )
    .expect("helper event dir should exist");

    let mut contents = String::new();
    if event_path.exists() {
        contents = fs::read_to_string(&event_path).expect("event log should be readable");
    }

    contents.push_str(&serde_json::to_string(&event).expect("event should serialize"));
    contents.push('\n');
    fs::write(event_path, contents).expect("event log should be writable");
}

fn write_ready_helper_status(base_dir: &Path, session_id: &str) {
    write_helper_status(base_dir, session_id, 1, "ready", "healthy");
}

fn write_helper_status(
    base_dir: &Path,
    session_id: &str,
    sequence: u64,
    camera_state: &str,
    helper_state: &str,
) {
    let status_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_STATUS_FILE_NAME);
    fs::create_dir_all(
        status_path
            .parent()
            .expect("status path should have a parent"),
    )
    .expect("status dir should exist");

    fs::write(
        status_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": CANON_HELPER_STATUS_SCHEMA_VERSION,
            "type": "camera-status",
            "sessionId": session_id,
            "sequence": sequence,
            "observedAt": current_timestamp(SystemTime::now()).expect("status timestamp should serialize"),
            "cameraState": camera_state,
            "helperState": helper_state,
            "cameraModel": "Canon EOS 700D",
            "detailCode": if camera_state == "ready" { "session-opened" } else { "session-opening" }
        }))
        .expect("status should serialize"),
    )
    .expect("status file should be writable");
}

fn create_published_bundle(
    catalog_root: &Path,
    preset_id: &str,
    published_version: &str,
    display_name: &str,
) {
    let bundle_dir = catalog_root.join(preset_id).join(published_version);
    fs::create_dir_all(bundle_dir.join("xmp")).expect("bundle dir should exist");
    fs::write(bundle_dir.join("preview.svg"), "<svg></svg>").expect("preview should exist");
    fs::write(bundle_dir.join("xmp").join("template.xmp"), "<xmp />")
        .expect("template should exist");
    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "published-preset-bundle/v1",
            "presetId": preset_id,
            "displayName": display_name,
            "publishedVersion": published_version,
            "lifecycleStatus": "published",
            "boothStatus": "booth-safe",
            "darktableVersion": "5.4.1",
            "xmpTemplatePath": "xmp/template.xmp",
            "previewProfile": {
                "profileId": "preview-jpeg",
                "displayName": "Booth Preview JPEG",
                "outputColorSpace": "sRGB"
            },
            "finalProfile": {
                "profileId": "final-jpeg",
                "displayName": "Booth Final JPEG",
                "outputColorSpace": "sRGB"
            },
            "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.svg",
                "altText": format!("{display_name} preview")
            }
        }))
        .expect("bundle should serialize"),
    )
    .expect("bundle should exist");
}

fn temp_test_dir(label: &str) -> PathBuf {
    let base_dir = std::env::temp_dir().join(format!(
        "boothy-hardware-validation-{label}-{}",
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).expect("base dir should be creatable");
    base_dir
}

fn ensure_fake_darktable_cli() {
    FAKE_DARKTABLE_SETUP.call_once(|| {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", script_path);
    });
}

fn _read_manifest(base_dir: &Path, session_id: &str) -> SessionManifest {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(manifest_path).expect("manifest should be readable");

    serde_json::from_str(&manifest_bytes).expect("manifest should deserialize")
}
