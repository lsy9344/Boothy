use boothy_lib::{
    session::manifest::SessionManifest,
    session::{
        manifest::{create_session_manifest, SessionManifestDraft},
        session_paths::resolve_session_paths,
    },
    contracts::schema_version::MANIFEST_SCHEMA_VERSION,
};

#[test]
fn manifest_round_trips_with_versioned_camel_case_fields() {
    let paths = resolve_session_paths("C:/Boothy/Sessions", "session-001");
    let manifest = create_session_manifest(SessionManifestDraft {
        session_id: "session-001".into(),
        session_name: "Kim Family".into(),
        operational_date: "2026-03-08".into(),
        created_at: "2026-03-08T09:00:00.000Z".into(),
        reservation_start_at: "2026-03-08T09:00:00.000Z".into(),
        session_type: "standard".into(),
        capture_revision: 0,
        active_preset_name: None,
        active_preset: None,
        latest_capture_id: None,
        captures: vec![],
        paths,
    })
    .expect("manifest should build");

    let value = serde_json::to_value(&manifest).expect("manifest should serialize");

    assert_eq!(value["schemaVersion"], MANIFEST_SCHEMA_VERSION);
    assert_eq!(value["processedDir"], "C:/Boothy/Sessions/session-001/processed");
    assert_eq!(value["cameraState"]["connectionState"], "offline");
    assert_eq!(value["timing"]["reservationStartAt"], "2026-03-08T09:00:00.000Z");
    assert_eq!(value["timing"]["actualShootEndAt"], "2026-03-08T09:50:00.000Z");
    assert_eq!(value["timing"]["sessionType"], "standard");
    assert_eq!(value["timing"]["operatorExtensionCount"], 0);
    assert_eq!(value["timing"]["lastTimingUpdateAt"], "2026-03-08T09:00:00.000Z");
    assert_eq!(value["exportState"]["status"], "notStarted");
    assert_eq!(value["captureRevision"], 0);
    assert_eq!(value["latestCaptureId"], serde_json::Value::Null);
    assert_eq!(value["activePresetName"], serde_json::Value::Null);
    assert_eq!(value["activePreset"], serde_json::Value::Null);
    assert_eq!(value["captures"], serde_json::json!([]));

    let round_trip: SessionManifest =
        serde_json::from_value(value).expect("manifest should deserialize");
    assert_eq!(round_trip, manifest);
}
