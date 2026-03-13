use boothy_lib::{
    capture::sidecar_client::{
        run_mock_capture_sidecar, run_mock_readiness_sidecar, watch_mock_readiness_sidecar,
        SidecarClientConfig, SidecarReadinessWatchMessage,
    },
    contracts::{
        dto::{
            CameraReadinessPayload, CameraReadinessRequest, CaptureActivePresetDto,
            CaptureCommandPayload, CaptureCommandRequest, MockScenario,
        },
        schema_version::{CONTRACT_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION},
    },
};
use tempfile::tempdir;

#[test]
fn mock_sidecar_emits_degraded_readiness_protocol_messages() {
    let outcome = run_mock_readiness_sidecar(
        &SidecarClientConfig::default(),
        &CameraReadinessRequest {
            schema_version: PROTOCOL_SCHEMA_VERSION.into(),
            request_id: "req-ready-003".into(),
            correlation_id: "corr-session-003".into(),
            method: "camera.checkReadiness".into(),
            session_id: Some("session-003".into()),
            payload: CameraReadinessPayload {
                desired_camera_id: None,
                mock_scenario: Some(MockScenario::ReadinessDegraded),
            },
        },
    )
    .expect("mock sidecar should respond");

    assert_eq!(outcome.progress_events.len(), 1);
    assert_eq!(
        outcome
            .success
            .expect("success response should exist")
            .status
            .readiness,
        boothy_lib::contracts::dto::CameraReadiness::Degraded
    );
}

#[test]
fn mock_sidecar_can_stream_readiness_updates_for_native_watchers() {
    let mut watch = watch_mock_readiness_sidecar(
        &SidecarClientConfig::default(),
        &CameraReadinessRequest {
            schema_version: PROTOCOL_SCHEMA_VERSION.into(),
            request_id: "req-watch-001".into(),
            correlation_id: "corr-watch-001".into(),
            method: "camera.checkReadiness".into(),
            session_id: Some("session-watch-001".into()),
            payload: CameraReadinessPayload {
                desired_camera_id: None,
                mock_scenario: None,
            },
        },
    )
    .expect("mock watch sidecar should start");

    let first = watch
        .next_message()
        .expect("watch stream should yield a message")
        .expect("watch stream should not terminate immediately");
    let second = watch
        .next_message()
        .expect("watch stream should keep yielding messages")
        .expect("watch stream should remain active");

    match first {
        SidecarReadinessWatchMessage::Success(status) => {
            assert_eq!(status.readiness, boothy_lib::contracts::dto::CameraReadiness::Ready);
        }
        other => panic!("expected a readiness success message, got {other:?}"),
    }

    match second {
        SidecarReadinessWatchMessage::Success(status) => {
            assert_eq!(status.readiness, boothy_lib::contracts::dto::CameraReadiness::Ready);
        }
        other => panic!("expected a second readiness success message, got {other:?}"),
    }

    watch.stop().expect("watch stream should stop cleanly");
}

#[test]
fn mock_sidecar_emits_capture_progress_before_returning_a_capture_result() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_dir = temp_dir.path().join("session-003");
    let processed_dir = session_dir.join("processed");
    let original_output_path = session_dir.join("originals/capture-003.nef");
    let processed_output_path = processed_dir.join("capture-003.png");

    let outcome = run_mock_capture_sidecar(
        &SidecarClientConfig::default(),
        &CaptureCommandRequest {
            schema_version: PROTOCOL_SCHEMA_VERSION.into(),
            request_id: "req-capture-003".into(),
            correlation_id: "corr-session-003".into(),
            method: "camera.capture".into(),
            session_id: "session-003".into(),
            payload: CaptureCommandPayload {
                active_preset: CaptureActivePresetDto {
                    preset_id: "background-pink".into(),
                    label: "배경지 - 핑크".into(),
                },
            },
        },
        "capture-003",
        "originals/capture-003.nef",
        "capture-003.png",
        &original_output_path,
        &processed_output_path,
    )
    .expect("mock capture sidecar should respond");

    assert_eq!(outcome.progress_events.len(), 2);
    assert_eq!(
        outcome.progress_events[0].payload.stage,
        boothy_lib::contracts::dto::CaptureProgressStage::CaptureStarted
    );
    assert_eq!(
        outcome.progress_events[1].payload.stage,
        boothy_lib::contracts::dto::CaptureProgressStage::CaptureCompleted
    );
    let success = outcome.success.expect("capture success response should exist");
    assert_eq!(success.schema_version, CONTRACT_SCHEMA_VERSION);
    assert_eq!(success.session_id, "session-003");
    assert_eq!(success.original_file_name, "originals/capture-003.nef");
    assert_eq!(success.processed_file_name, "capture-003.png");
    assert!(success.manifest_path.ends_with("/session.json"));
}
