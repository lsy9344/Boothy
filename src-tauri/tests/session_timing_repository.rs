use boothy_lib::session::session_repository::{
    extend_session_timing,
    get_session_timing,
    initialize_session_timing,
    provision_session,
    SessionProvisionRequest,
};
use tempfile::tempdir;

#[test]
fn timing_state_persists_and_reloads_without_shape_drift() {
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

    let initialized = initialize_session_timing(
        &session.manifest_path,
        &session.session_id,
        "2026-03-08T09:00:00.000Z",
        "couponExtended",
        "2026-03-08T09:07:00.000Z",
    )
    .expect("timing should initialize");

    assert_eq!(
        initialized.timing.actual_shoot_end_at,
        "2026-03-08T10:40:00.000Z"
    );
    assert_eq!(initialized.timing.session_type, "couponExtended");
    assert_eq!(initialized.timing.operator_extension_count, 0);

    let reloaded = get_session_timing(&session.manifest_path, &session.session_id)
        .expect("timing should reload from the manifest");

    assert_eq!(reloaded.timing, initialized.timing);

    let extended = extend_session_timing(
        &session.manifest_path,
        &session.session_id,
        "2026-03-08T09:30:00.000Z",
    )
    .expect("timing should extend");

    assert_eq!(
        extended.timing.actual_shoot_end_at,
        "2026-03-08T11:40:00.000Z"
    );
    assert_eq!(extended.timing.operator_extension_count, 1);

    let reloaded_after_extension = get_session_timing(&session.manifest_path, &session.session_id)
        .expect("extended timing should reload from the manifest");

    assert_eq!(reloaded_after_extension.timing, extended.timing);
}
