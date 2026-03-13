use std::{fs, path::Path};

use boothy_lib::{
    db::{migrations::apply_pending_migrations, sqlite::open_operational_log_connection},
    diagnostics::lifecycle_log::insert_lifecycle_event,
    session::{
        session_manifest::SessionActivePresetSelection,
        session_repository::{append_session_capture, provision_session, SessionProvisionRequest},
    },
};
use rusqlite::OptionalExtension;
use tempfile::tempdir;

#[test]
fn same_day_duplicate_sessions_suffix_names_and_create_reserved_artifacts() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");

    let first = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "홍길동 오후 세션".into(),
            created_at: "2026-03-08T00:00:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("first session should provision");

    let second = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "홍길동 오후 세션".into(),
            created_at: "2026-03-08T00:05:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("duplicate session should suffix");

    assert_eq!(first.session_name, "홍길동 오후 세션");
    assert_eq!(second.session_name, "홍길동 오후 세션_2");
    assert!(first.session_dir.ends_with(Path::new("2026-03-08").join("홍길동 오후 세션")));
    assert!(second.session_dir.ends_with(Path::new("2026-03-08").join("홍길동 오후 세션_2")));
    assert!(first.manifest_path.ends_with(Path::new("session.json")));
    assert!(first.events_path.ends_with(Path::new("events.ndjson")));
    assert!(first.export_status_path.ends_with(Path::new("export-status.json")));
    assert!(first.processed_dir.ends_with(Path::new("processed")));
    assert!(first.manifest_path.exists());
    assert!(first.events_path.exists());
    assert!(first.export_status_path.exists());
}

#[test]
fn session_started_lifecycle_payload_keeps_only_minimum_customer_identifiers() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");
    let db_path = temp_dir.path().join("operational-log.sqlite3");
    let mut connection = open_operational_log_connection(&db_path).expect("database should open");
    apply_pending_migrations(&mut connection).expect("migrations should apply");

    let session = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "홍길동 오후 세션".into(),
            created_at: "2026-03-08T00:00:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("session should provision");

    insert_lifecycle_event(
        &connection,
        &session.as_lifecycle_event("branch-unconfigured"),
    )
    .expect("session_created event should insert");

    let payload_json: String = connection
        .query_row(
            "SELECT payload_json FROM session_events WHERE session_id = ?1",
            [session.session_id.as_str()],
            |row| row.get(0),
        )
        .expect("payload_json should exist");

    let payload: serde_json::Value = serde_json::from_str(&payload_json).expect("payload should parse");

    assert!(payload.get("reservationName").is_none());
    assert!(payload.get("phoneLast4").is_none());
    assert!(payload.get("fullPhoneNumber").is_none());
    assert!(payload.get("reservationSource").is_none());

    let event_type: Option<String> = connection
        .query_row(
            "SELECT event_type FROM session_events WHERE session_id = ?1",
            [session.session_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .expect("event_type should query");

    assert_eq!(event_type.as_deref(), Some("session_created"));
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&session.manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should parse");

    assert!(manifest.get("phoneLast4").is_none());
}

#[test]
fn provision_session_preserves_coupon_extended_timing_inputs_in_the_manifest() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");

    let session = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "홍길동 오후 세션".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: Some("2026-03-08T09:00:00.000Z".into()),
            session_type: Some("couponExtended".into()),
        },
    )
    .expect("session should provision");

    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&session.manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should parse");

    assert_eq!(manifest["timing"]["reservationStartAt"], "2026-03-08T09:00:00.000Z");
    assert_eq!(manifest["timing"]["actualShootEndAt"], "2026-03-08T10:40:00.000Z");
    assert_eq!(manifest["timing"]["sessionType"], "couponExtended");
}

#[test]
fn provision_session_sanitizes_windows_unsafe_session_names_before_creating_the_session_folder() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");

    let session = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "  Kim:Family?  ".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("invalid Windows path characters should be normalized");

    assert_eq!(session.session_name, "Kim_Family");
    assert!(session
        .session_dir
        .ends_with(Path::new("2026-03-08").join("Kim_Family")));
}

#[test]
fn provision_session_avoids_reserved_windows_device_names() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");

    let session = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "CON".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("reserved device names should be rewritten");

    assert_eq!(session.session_name, "CON_session");
    assert!(session
        .session_dir
        .ends_with(Path::new("2026-03-08").join("CON_session")));
}

#[test]
fn append_session_capture_records_the_canonical_original_asset_under_originals() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");

    let session = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "홍길동 오후 세션".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("session should provision");

    fs::create_dir_all(session.session_dir.join("originals")).expect("originals directory should exist");
    fs::write(
        session.session_dir.join("originals/capture-001.nef"),
        b"sidecar-original-bytes",
    )
    .expect("original asset should exist");
    fs::write(
        session.processed_dir.join("capture-001.png"),
        b"sidecar-processed-bytes",
    )
    .expect("processed asset should exist");

    let persisted_capture = append_session_capture(
        &session.manifest_path,
        &session.session_id,
        SessionActivePresetSelection {
            preset_id: "background-pink".into(),
            display_name: "배경지 - 핑크".into(),
        },
        "capture-001",
        "originals/capture-001.nef",
        "capture-001.png",
        "2026-03-08T09:08:00.000Z",
    )
    .expect("capture should persist");

    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&session.manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should parse");

    assert_eq!(persisted_capture.capture_id, "capture-001");
    assert_eq!(manifest["captures"][0]["originalFileName"], "originals/capture-001.nef");
    assert!(session
        .session_dir
        .join("originals/capture-001.nef")
        .exists());
    assert!(
        fs::read(session.processed_dir.join("capture-001.png"))
            .expect("processed capture should be readable")
            == b"sidecar-processed-bytes"
    );
}

#[test]
fn append_session_capture_preserves_sidecar_written_assets_instead_of_overwriting_them() {
    let temp_dir = tempdir().expect("temporary directory should exist");
    let session_root = temp_dir.path().join("sessions");

    let session = provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: "홍길동 오후 세션".into(),
            created_at: "2026-03-08T09:07:00.000Z".into(),
            operational_date: "2026-03-08".into(),
            reservation_start_at: None,
            session_type: None,
        },
    )
    .expect("session should provision");

    fs::create_dir_all(session.session_dir.join("originals")).expect("originals directory should exist");
    fs::write(
        session.session_dir.join("originals/capture-001.nef"),
        b"real-raw-bytes-from-sidecar",
    )
    .expect("original asset should exist");
    fs::write(
        session.processed_dir.join("capture-001.png"),
        b"real-preview-bytes-from-sidecar",
    )
    .expect("processed asset should exist");

    append_session_capture(
        &session.manifest_path,
        &session.session_id,
        SessionActivePresetSelection {
            preset_id: "background-pink".into(),
            display_name: "배경지 - 핑크".into(),
        },
        "capture-001",
        "originals/capture-001.nef",
        "capture-001.png",
        "2026-03-08T09:08:00.000Z",
    )
    .expect("capture should persist");

    assert_eq!(
        fs::read(session.session_dir.join("originals/capture-001.nef"))
            .expect("original asset should remain readable"),
        b"real-raw-bytes-from-sidecar"
    );
    assert_eq!(
        fs::read(session.processed_dir.join("capture-001.png"))
            .expect("processed asset should remain readable"),
        b"real-preview-bytes-from-sidecar"
    );
}
