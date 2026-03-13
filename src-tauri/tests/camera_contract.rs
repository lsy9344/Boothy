use boothy_lib::{
    capture::camera_host::{
        build_capture_confidence_snapshot, CameraHost, CameraHostConfig, VecProgressSink,
    },
    contracts::{
        dto::{
            CameraReadinessPayload, CameraReadinessRequest, CaptureActivePresetDto,
            SidecarCaptureRequest, SidecarCaptureRequestPayload, SidecarCaptureSuccessResponse,
        },
        schema_version::{ERROR_ENVELOPE_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION},
    },
    session::manifest::{
        CameraState, ExportState, SessionActivePresetSelection, SessionManifest, SessionTiming,
    },
};
use std::fs;
use tempfile::tempdir;

#[test]
fn readiness_flow_returns_contract_success_and_forwards_progress_events() {
    let host = CameraHost::new(CameraHostConfig::default());
    let mut sink = VecProgressSink::default();

    let result = host.run_readiness_flow(
        CameraReadinessRequest {
            schema_version: PROTOCOL_SCHEMA_VERSION.into(),
            request_id: "req-ready-001".into(),
            correlation_id: "corr-session-001".into(),
            method: "camera.checkReadiness".into(),
            session_id: Some("session-001".into()),
            payload: CameraReadinessPayload {
                desired_camera_id: Some("canon-eos-r100".into()),
                mock_scenario: None,
            },
        },
        &mut sink,
    );

    assert!(result.ok);
    assert_eq!(result.status.readiness, boothy_lib::contracts::dto::CameraReadiness::Ready);
    assert_eq!(sink.events.len(), 1);
    assert_eq!(sink.events[0].event, "camera.statusChanged");
}

#[test]
fn readiness_flow_returns_normalized_error_envelope_on_sidecar_error() {
    let host = CameraHost::new(CameraHostConfig::default());
    let mut sink = VecProgressSink::default();

    let result = host.run_readiness_flow(
        CameraReadinessRequest {
            schema_version: PROTOCOL_SCHEMA_VERSION.into(),
            request_id: "req-ready-002".into(),
            correlation_id: "corr-session-002".into(),
            method: "camera.checkReadiness".into(),
            session_id: Some("session-002".into()),
            payload: CameraReadinessPayload {
                desired_camera_id: None,
                mock_scenario: Some(boothy_lib::contracts::dto::MockScenario::NormalizedError),
            },
        },
        &mut sink,
    );

    assert!(!result.ok);
    let error = result.error.expect("normalized error envelope should be present");
    assert_eq!(error.schema_version, ERROR_ENVELOPE_SCHEMA_VERSION);
    assert_eq!(error.code, "camera.reconnecting");
    assert!(sink.events.is_empty());
}

#[test]
fn capture_confidence_snapshot_uses_manifest_timing_and_active_preset() {
    let snapshot = build_capture_confidence_snapshot(
        &SessionManifest {
            schema_version: 1,
            session_id: "2026-03-08:홍길동1234".into(),
            session_name: "홍길동1234".into(),
            operational_date: "2026-03-08".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            session_dir: "C:/Boothy/Sessions/2026-03-08/홍길동1234".into(),
            manifest_path: "C:/Boothy/Sessions/2026-03-08/홍길동1234/session.json".into(),
            events_path: "C:/Boothy/Sessions/2026-03-08/홍길동1234/events.ndjson".into(),
            export_status_path: "C:/Boothy/Sessions/2026-03-08/홍길동1234/export-status.json".into(),
            processed_dir: "C:/Boothy/Sessions/2026-03-08/홍길동1234/processed".into(),
            capture_revision: 0,
            latest_capture_id: None,
            active_preset_name: Some("배경지 - 핑크".into()),
            active_preset: Some(SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            }),
            captures: vec![],
            camera_state: CameraState {
                connection_state: "offline".into(),
            },
            timing: SessionTiming {
                reservation_start_at: "2026-03-08T09:00:00.000Z".into(),
                actual_shoot_end_at: "2026-03-08T10:40:00.000Z".into(),
                session_type: "couponExtended".into(),
                operator_extension_count: 0,
                last_timing_update_at: "2026-03-08T09:07:00.000Z".into(),
            },
            export_state: ExportState {
                status: "notStarted".into(),
            },
        },
        "2026-03-08T09:10:00.000Z",
    );

    assert_eq!(snapshot.shoot_ends_at, "2026-03-08T10:40:00.000Z");
    assert_eq!(snapshot.active_preset.preset_id, "background-pink");
    assert_eq!(snapshot.active_preset.label, "배경지 - 핑크");
}

#[test]
fn capture_confidence_snapshot_returns_the_latest_session_photo_when_a_processed_asset_exists() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_dir = temp_dir.path().join("2026-03-08").join("홍길동1234");
    let processed_dir = session_dir.join("processed");
    fs::create_dir_all(&processed_dir).expect("processed directory should exist");
    fs::write(processed_dir.join("capture-001.png"), b"fake-image").expect("capture should be written");

    let snapshot = build_capture_confidence_snapshot(
        &SessionManifest {
            schema_version: 1,
            session_id: "2026-03-08:홍길동1234".into(),
            session_name: "홍길동1234".into(),
            operational_date: "2026-03-08".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            session_dir: session_dir.to_string_lossy().replace('\\', "/"),
            manifest_path: session_dir.join("session.json").to_string_lossy().replace('\\', "/"),
            events_path: session_dir.join("events.ndjson").to_string_lossy().replace('\\', "/"),
            export_status_path: session_dir
                .join("export-status.json")
                .to_string_lossy()
                .replace('\\', "/"),
            processed_dir: processed_dir.to_string_lossy().replace('\\', "/"),
            capture_revision: 7,
            latest_capture_id: Some("capture-001".into()),
            active_preset_name: Some("배경지 - 핑크".into()),
            active_preset: Some(SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            }),
            captures: vec![boothy_lib::session::manifest::ManifestCaptureRecord {
                capture_id: "capture-001".into(),
                original_file_name: "originals/capture-001.nef".into(),
                processed_file_name: "capture-001.png".into(),
                captured_at: "2026-03-08T09:09:00.000Z".into(),
            }],
            camera_state: CameraState {
                connection_state: "offline".into(),
            },
            timing: SessionTiming {
                reservation_start_at: "2026-03-08T09:00:00.000Z".into(),
                actual_shoot_end_at: "2026-03-08T10:40:00.000Z".into(),
                session_type: "couponExtended".into(),
                operator_extension_count: 0,
                last_timing_update_at: "2026-03-08T09:07:00.000Z".into(),
            },
            export_state: ExportState {
                status: "notStarted".into(),
            },
        },
        "2026-03-08T09:10:00.000Z",
    );

    assert_eq!(snapshot.revision, 7);
    match snapshot.latest_photo {
        boothy_lib::contracts::dto::LatestPhotoState::Ready { photo } => {
            assert_eq!(photo.session_id, "2026-03-08:홍길동1234");
            assert_eq!(photo.capture_id, "capture-001");
            assert_eq!(photo.sequence, 1);
            assert!(photo.asset_url.ends_with("/processed/capture-001.png"));
        }
        other => panic!("expected ready latest photo, got {other:?}"),
    }
}

#[test]
fn sidecar_capture_request_contract_round_trips_the_helper_payload_shape() {
    let request = SidecarCaptureRequest {
        schema_version: PROTOCOL_SCHEMA_VERSION.into(),
        request_id: "req-capture-001".into(),
        correlation_id: "corr-session-001".into(),
        method: "camera.capture".into(),
        session_id: "session-001".into(),
        payload: SidecarCaptureRequestPayload {
            active_preset: CaptureActivePresetDto {
                preset_id: "background-pink".into(),
                label: "배경지 - 핑크".into(),
            },
            capture_id: "capture-001".into(),
            original_file_name: "originals/capture-001.nef".into(),
            processed_file_name: "capture-001.png".into(),
            original_output_path:
                "C:/Boothy/Sessions/2026-03-08/홍길동1234/originals/capture-001.nef".into(),
            processed_output_path:
                "C:/Boothy/Sessions/2026-03-08/홍길동1234/processed/capture-001.png".into(),
        },
    };

    let value = serde_json::to_value(&request).expect("sidecar request should serialize");
    let parsed: SidecarCaptureRequest =
        serde_json::from_value(value.clone()).expect("sidecar request should deserialize");

    assert_eq!(parsed, request);
    assert_eq!(value["payload"]["captureId"], "capture-001");
}

#[test]
fn sidecar_capture_success_contract_round_trips_the_helper_success_shape() {
    let response = SidecarCaptureSuccessResponse {
        schema_version: "boothy.camera.contract.v1".into(),
        request_id: "req-capture-001".into(),
        correlation_id: "corr-session-001".into(),
        ok: true,
        session_id: "session-001".into(),
        capture_id: "capture-001".into(),
        original_file_name: "originals/capture-001.nef".into(),
        processed_file_name: "capture-001.png".into(),
        captured_at: "2026-03-08T10:00:08.000Z".into(),
        manifest_path: "C:/Boothy/Sessions/2026-03-08/홍길동1234/session.json".into(),
    };

    let value = serde_json::to_value(&response).expect("sidecar response should serialize");
    let parsed: SidecarCaptureSuccessResponse =
        serde_json::from_value(value.clone()).expect("sidecar response should deserialize");

    assert_eq!(parsed, response);
    assert_eq!(value["manifestPath"], "C:/Boothy/Sessions/2026-03-08/홍길동1234/session.json");
}
