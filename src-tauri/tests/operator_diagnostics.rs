use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    commands::runtime_commands::capability_snapshot_for_profile,
    diagnostics::{ensure_operator_window_label, load_operator_session_summary_in_dir},
    session::{
        session_manifest::{
            current_timestamp, ActivePresetBinding, CaptureTimingMetrics, CompletedPostEnd,
            ExportWaitingPostEnd, FinalCaptureAsset, PreviewCaptureAsset, RawCaptureAsset,
            SessionCaptureRecord, SessionCustomer, SessionLifecycle, SessionManifest,
            SessionPostEnd, SESSION_CAPTURE_SCHEMA_VERSION, SESSION_MANIFEST_SCHEMA_VERSION,
            SESSION_POST_END_COMPLETED, SESSION_POST_END_EXPORT_WAITING,
        },
        session_paths::SessionPaths,
    },
};

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-operator-diagnostics-{test_name}-{stamp}"))
}

#[test]
fn operator_diagnostics_returns_a_safe_no_session_summary() {
    let base_dir = unique_test_root("no-session");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("authorized operator should receive a no-session summary");

    assert_eq!(summary.state, "no-session");
    assert_eq!(summary.blocked_state_category, "not-blocked");
    assert_eq!(summary.session_id, None);
    assert_eq!(summary.camera_connection.state, "disconnected");
    assert_eq!(summary.capture_boundary.status, "clear");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_classifies_capture_blocked_sessions() {
    let base_dir = unique_test_root("capture-blocked");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "camera-preparing".into(),
        },
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_helper_status(
        &base_dir,
        session_id,
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        "disconnected",
        "healthy",
        Some("camera-not-found"),
    );

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("capture-blocked summary should load");

    assert_eq!(summary.state, "session-loaded");
    assert_eq!(summary.blocked_state_category, "capture-blocked");
    assert_eq!(summary.camera_connection.state, "disconnected");
    assert_eq!(summary.capture_boundary.status, "blocked");
    assert_eq!(
        summary
            .recent_failure
            .as_ref()
            .map(|failure| failure.title.as_str()),
        Some("활성 preset이 아직 선택되지 않았어요.")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_classifies_preview_render_blocked_sessions() {
    let base_dir = unique_test_root("preview-render-blocked");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1n";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "preview-waiting".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        captures: vec![preview_waiting_capture(session_id)],
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_ready_helper_status(&base_dir, session_id);

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("preview/render-blocked summary should load");

    assert_eq!(summary.blocked_state_category, "preview-render-blocked");
    assert_eq!(summary.camera_connection.state, "connected");
    assert_eq!(summary.preview_render_boundary.status, "blocked");
    assert_eq!(
        summary.active_preset_display_name.as_deref(),
        Some("Soft Glow")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_classifies_post_end_blocked_sessions() {
    let base_dir = unique_test_root("post-end-blocked");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1o";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "export-waiting".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        captures: vec![preview_waiting_capture(session_id)],
        post_end: Some(SessionPostEnd::ExportWaiting(ExportWaitingPostEnd {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            evaluated_at: "2026-03-26T00:12:00Z".into(),
        })),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("post-end summary should load");

    assert_eq!(summary.blocked_state_category, "timing-post-end-blocked");
    assert_eq!(summary.completion_boundary.status, "blocked");
    assert_eq!(summary.post_end_state.as_deref(), Some("export-waiting"));
    assert_eq!(
        summary
            .recent_failure
            .as_ref()
            .map(|failure| failure.title.as_str()),
        Some("종료 후 완료 판정이 아직 보류돼 있어요.")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_reuses_live_capture_truth_from_capture_readiness() {
    let base_dir = unique_test_root("live-capture-truth");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1v";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "capture-ready".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_ready_helper_status(&base_dir, session_id);

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should reuse live capture truth");
    let live_truth = summary
        .live_capture_truth
        .as_ref()
        .expect("operator summary should expose live capture truth");

    assert_eq!(live_truth.source, "canon-helper-sidecar");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.session_match, "matched");
    assert_eq!(live_truth.camera_state, "ready");
    assert_eq!(live_truth.helper_state, "healthy");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_projects_connecting_camera_connection_state() {
    let base_dir = unique_test_root("camera-connection-connecting");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1w";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "helper-preparing".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_helper_status(
        &base_dir,
        session_id,
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        "connecting",
        "starting",
        Some("session-opening"),
    );

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("connecting summary should load");

    assert_eq!(summary.camera_connection.state, "connecting");
    assert_eq!(
        summary.camera_connection.title,
        "카메라 연결을 확인하는 중이에요."
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_projects_disconnected_camera_connection_state_from_detail_code() {
    let base_dir = unique_test_root("camera-connection-disconnected-detail-code");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1d";
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "camera-preparing".into(),
        },
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_helper_status(
        &base_dir,
        session_id,
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        "connecting",
        "healthy",
        Some("unsupported-camera"),
    );

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("disconnected summary should load");
    let live_truth = summary
        .live_capture_truth
        .as_ref()
        .expect("live capture truth should be present");

    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.session_match, "matched");
    assert_eq!(
        live_truth.detail_code.as_deref(),
        Some("unsupported-camera")
    );
    assert_eq!(summary.camera_connection.state, "disconnected");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_projects_recovery_required_camera_connection_state_from_detail_code() {
    let base_dir = unique_test_root("camera-connection-recovery-detail-code");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1e";
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "helper-preparing".into(),
        },
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_helper_status(
        &base_dir,
        session_id,
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        "connecting",
        "healthy",
        Some("sdk-init-failed"),
    );

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("recovery summary should load");
    let live_truth = summary
        .live_capture_truth
        .as_ref()
        .expect("live capture truth should be present");

    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.session_match, "matched");
    assert_eq!(live_truth.detail_code.as_deref(), Some("sdk-init-failed"));
    assert_eq!(summary.camera_connection.state, "recovery-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_projects_recovery_required_camera_connection_state() {
    let base_dir = unique_test_root("camera-connection-recovery-required");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1x";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "capture-ready".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_helper_status(
        &base_dir,
        session_id,
        &current_timestamp(
            SystemTime::now()
                .checked_sub(Duration::from_secs(60))
                .expect("stale helper timestamp should be earlier than now"),
        )
        .expect("stale helper timestamp should serialize"),
        "ready",
        "healthy",
        Some("camera-ready"),
    );

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("recovery-required summary should load");

    assert_eq!(summary.camera_connection.state, "recovery-required");
    assert_eq!(summary.blocked_state_category, "capture-blocked");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_describes_startup_connect_failures_without_claiming_capture_saved() {
    let base_dir = unique_test_root("startup-connect-timeout-diagnostics");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1y";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "phone-required".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);
    write_helper_status(
        &base_dir,
        session_id,
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        "error",
        "error",
        Some("session-open-timeout"),
    );

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("startup timeout summary should load");

    assert_eq!(summary.camera_connection.state, "recovery-required");
    assert_eq!(
        summary
            .recent_failure
            .as_ref()
            .map(|failure| failure.title.as_str()),
        Some("카메라 연결 시작이 실패했어요.")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_camera_connection_observed_at_uses_only_live_capture_truth() {
    let base_dir = unique_test_root("camera-connection-observed-at-source");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1f";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "export-waiting".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        captures: vec![preview_waiting_capture(session_id)],
        post_end: Some(SessionPostEnd::ExportWaiting(ExportWaitingPostEnd {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            evaluated_at: "2026-03-26T00:12:00Z".into(),
        })),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);

    let paths = SessionPaths::new(&base_dir, session_id);
    fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics directory should exist");
    fs::write(
        paths.diagnostics_dir.join("timing-events.log"),
        "2026-03-26T00:12:00Z\tsession=session_01hs6n1r8b8zc5v4ey2x7b9g1f\tevent=post-end-evaluated\tstate=export-waiting",
    )
    .expect("diagnostics log should write");

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("summary should load");

    assert_eq!(summary.camera_connection.state, "recovery-required");
    assert_eq!(summary.camera_connection.observed_at, None);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_rejects_falling_back_to_an_older_session_when_the_latest_manifest_is_invalid(
) {
    let base_dir = unique_test_root("latest-invalid-manifest");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let older_session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1q";
    let newer_session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1r";

    write_manifest(&base_dir, &base_manifest(older_session_id));
    thread::sleep(Duration::from_millis(20));

    let newer_paths = SessionPaths::new(&base_dir, newer_session_id);
    fs::create_dir_all(&newer_paths.session_root).expect("newer session directory should exist");
    fs::write(&newer_paths.manifest_path, "{ not-json").expect("invalid manifest should write");

    let error = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect_err("a newer invalid manifest should block stale fallback");

    assert_eq!(error.code, "session-persistence-failed");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_returns_no_session_when_the_latest_session_is_already_completed() {
    let base_dir = unique_test_root("completed-session");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1s";
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "completed".into(),
        },
        post_end: Some(SessionPostEnd::Completed(CompletedPostEnd {
            state: SESSION_POST_END_COMPLETED.into(),
            evaluated_at: "2026-03-26T00:12:00Z".into(),
            completion_variant: "handoff-ready".into(),
            approved_recipient_label: Some("Front Desk".into()),
            next_location_label: None,
            primary_action_label: "직원 안내를 따라 이동해 주세요.".into(),
            support_action_label: None,
            show_booth_alias: true,
            handoff: None,
        })),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("completed sessions should collapse to a safe no-session summary");

    assert_eq!(summary.state, "no-session");
    assert_eq!(summary.session_id, None);
    assert_eq!(summary.blocked_state_category, "not-blocked");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_redacts_malformed_diagnostics_logs() {
    let base_dir = unique_test_root("malformed-diagnostics");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1p";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "preview-waiting".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        captures: vec![preview_waiting_capture(session_id)],
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);

    let paths = SessionPaths::new(&base_dir, session_id);
    fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics directory should exist");
    fs::write(
        paths.diagnostics_dir.join("timing-events.log"),
        "C:\\render-worker\\stderr.log\tsession=session_01hs6n1r8b8zc5v4ey2x7b9g1p\tevent=render-failed",
    )
    .expect("malformed diagnostics log should write");

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("summary should still load when diagnostics log is malformed");
    let recent_failure = summary
        .recent_failure
        .expect("malformed diagnostics should surface a safe recent failure summary");

    assert_eq!(summary.blocked_state_category, "preview-render-blocked");
    assert!(recent_failure.title.contains("복원하지 못했어요"));
    assert!(recent_failure.detail.contains("로그 형식이 올바르지 않아"));
    assert!(!recent_failure
        .detail
        .contains("C:\\render-worker\\stderr.log"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_rejects_non_operator_window_labels() {
    let error = ensure_operator_window_label("booth-window")
        .expect_err("operator diagnostics should reject non-operator windows");

    assert_eq!(error.code, "capability-denied");
}

#[test]
fn operator_diagnostics_projects_post_end_without_persisting_a_read_side_effect() {
    let base_dir = unique_test_root("projection-no-side-effect");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1t";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "ended".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        timing: Some(boothy_lib::session::session_manifest::SessionTiming {
            schema_version: "session-timing/v1".into(),
            session_id: session_id.into(),
            adjusted_end_at: "2026-03-26T00:01:00Z".into(),
            warning_at: "2026-03-26T00:00:30Z".into(),
            phase: "ended".into(),
            capture_allowed: false,
            approved_extension_minutes: 0,
            approved_extension_audit_ref: None,
            warning_triggered_at: Some("2026-03-26T00:00:30Z".into()),
            ended_triggered_at: Some("2026-03-26T00:01:00Z".into()),
        }),
        captures: vec![preview_waiting_capture(session_id)],
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);

    let paths = SessionPaths::new(&base_dir, session_id);
    let before = fs::read_to_string(&paths.manifest_path).expect("manifest should exist");

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should project post-end state without failing");

    let after = fs::read_to_string(&paths.manifest_path).expect("manifest should still exist");

    assert_eq!(summary.blocked_state_category, "timing-post-end-blocked");
    assert_eq!(before, after);
    assert!(!paths.diagnostics_dir.join("timing-events.log").is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_diagnostics_recent_failure_uses_specific_context_instead_of_boundary_copy() {
    let base_dir = unique_test_root("recent-failure-context");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1u";
    create_published_bundle(&base_dir, "preset_soft-glow", "2026.03.26", "Soft Glow");
    let manifest = SessionManifest {
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "export-waiting".into(),
        },
        active_preset: Some(ActivePresetBinding {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        }),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_display_name: Some("Soft Glow".into()),
        captures: vec![preview_waiting_capture(session_id)],
        post_end: Some(SessionPostEnd::ExportWaiting(ExportWaitingPostEnd {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            evaluated_at: "2026-03-26T00:12:00Z".into(),
        })),
        ..base_manifest(session_id)
    };

    write_manifest(&base_dir, &manifest);

    let paths = SessionPaths::new(&base_dir, session_id);
    fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics directory should exist");
    fs::write(
        paths.diagnostics_dir.join("timing-events.log"),
        "2026-03-26T00:12:00Z\tsession=session_01hs6n1r8b8zc5v4ey2x7b9g1u\tevent=post-end-evaluated\tstate=export-waiting",
    )
    .expect("diagnostics log should write");

    let summary = load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("summary should load");
    let recent_failure = summary.recent_failure.expect("recent failure should exist");

    assert_eq!(
        recent_failure.observed_at.as_deref(),
        Some("2026-03-26T00:12:00Z")
    );
    assert_eq!(
        recent_failure.title,
        "종료 후 완료 판정이 아직 보류돼 있어요."
    );
    assert_ne!(recent_failure.title, summary.completion_boundary.title);

    let _ = fs::remove_dir_all(base_dir);
}

fn base_manifest(session_id: &str) -> SessionManifest {
    SessionManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION.into(),
        session_id: session_id.into(),
        booth_alias: "Kim 4821".into(),
        customer: SessionCustomer {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
        created_at: "2026-03-26T00:00:00Z".into(),
        updated_at: "2026-03-26T00:00:10Z".into(),
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "session-started".into(),
        },
        catalog_revision: None,
        catalog_snapshot: None,
        active_preset: None,
        active_preset_id: None,
        active_preset_display_name: None,
        timing: None,
        captures: Vec::new(),
        post_end: None,
    }
}

fn preview_waiting_capture(session_id: &str) -> SessionCaptureRecord {
    SessionCaptureRecord {
        schema_version: SESSION_CAPTURE_SCHEMA_VERSION.into(),
        session_id: session_id.into(),
        booth_alias: "Kim 4821".into(),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_version: "2026.03.26".into(),
        active_preset_display_name: Some("Soft Glow".into()),
        capture_id: "capture_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
        request_id: "request_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
        raw: RawCaptureAsset {
            asset_path: format!("C:/boothy/sessions/{session_id}/captures/originals/capture.jpg"),
            persisted_at_ms: 100,
        },
        preview: PreviewCaptureAsset {
            asset_path: None,
            enqueued_at_ms: Some(100),
            ready_at_ms: None,
            kind: None,
        },
        final_asset: FinalCaptureAsset {
            asset_path: None,
            ready_at_ms: None,
        },
        render_status: "previewWaiting".into(),
        post_end_state: "activeSession".into(),
        timing: CaptureTimingMetrics {
            capture_acknowledged_at_ms: 100,
            preview_visible_at_ms: None,
            fast_preview_visible_at_ms: None,
            xmp_preview_ready_at_ms: None,
            capture_budget_ms: 1_000,
            preview_budget_ms: 5_000,
            preview_budget_state: "pending".into(),
        },
    }
}

fn write_manifest(base_dir: &Path, manifest: &SessionManifest) {
    let paths = SessionPaths::new(base_dir, &manifest.session_id);

    fs::create_dir_all(&paths.session_root).expect("session directory should exist");
    fs::create_dir_all(&paths.captures_originals_dir).expect("capture directory should exist");
    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    fs::create_dir_all(&paths.renders_finals_dir).expect("final directory should exist");
    fs::create_dir_all(&paths.handoff_dir).expect("handoff directory should exist");
    fs::write(
        &paths.manifest_path,
        serde_json::to_vec_pretty(manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");
}

fn create_published_bundle(
    base_dir: &Path,
    preset_id: &str,
    published_version: &str,
    display_name: &str,
) {
    let bundle_dir = base_dir
        .join("preset-catalog")
        .join("published")
        .join(preset_id)
        .join(published_version);

    fs::create_dir_all(&bundle_dir).expect("bundle directory should exist");
    fs::create_dir_all(bundle_dir.join("xmp")).expect("xmp directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), "preview").expect("preview should write");
    fs::write(
        bundle_dir.join("xmp").join("template.xmp"),
        format!(
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
                "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">",
                "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">",
                "<rdf:Description xmlns:darktable=\"http://darktable.sf.net/\">",
                "<darktable:history><rdf:Seq><rdf:li><darktable:module>{preset_id}</darktable:module></rdf:li></rdf:Seq></darktable:history>",
                "</rdf:Description></rdf:RDF></x:xmpmeta>"
            ),
            preset_id = preset_id
        ),
    )
    .expect("xmp template should write");
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
                "profileId": format!("{preset_id}-preview"),
                "displayName": format!("{display_name} Preview"),
                "outputColorSpace": "sRGB",
            },
            "finalProfile": {
                "profileId": format!("{preset_id}-final"),
                "displayName": format!("{display_name} Final"),
                "outputColorSpace": "sRGB",
            },
            "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.jpg",
                "altText": format!("{display_name} preview"),
            }
        }))
        .expect("bundle should serialize"),
    )
    .expect("bundle should write");
}

fn write_ready_helper_status(base_dir: &Path, session_id: &str) {
    write_helper_status(
        base_dir,
        session_id,
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        "ready",
        "healthy",
        Some("camera-ready"),
    );
}

fn write_helper_status(
    base_dir: &Path,
    session_id: &str,
    observed_at: &str,
    camera_state: &str,
    helper_state: &str,
    detail_code: Option<&str>,
) {
    let status_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("camera-helper-status.json");
    fs::create_dir_all(
        status_path
            .parent()
            .expect("helper status should have a diagnostics directory"),
    )
    .expect("diagnostics directory should exist");
    fs::write(
        status_path,
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "canon-helper-status/v1",
          "sessionId": session_id,
          "sequence": 1,
          "observedAt": observed_at,
          "cameraState": camera_state,
          "helperState": helper_state,
          "detailCode": detail_code
        }))
        .expect("helper status should serialize"),
    )
    .expect("helper status should be writable");
}
