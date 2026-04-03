use std::{
    fs,
    path::PathBuf,
    sync::Once,
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    capture::{
        normalized_state::get_capture_readiness_in_dir,
        sidecar_client::{
            read_capture_request_messages, CAMERA_HELPER_EVENTS_FILE_NAME,
            CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION, CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        },
    },
    commands::runtime_commands::capability_snapshot_for_profile,
    contracts::dto::{
        CaptureReadinessInputDto, OperatorRecoveryActionInputDto, SessionStartInputDto,
    },
    diagnostics::recovery::{
        execute_operator_recovery_action_in_dir, load_operator_recovery_summary_in_dir,
    },
    session::{
        session_manifest::{current_timestamp, SessionManifest},
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

fn unique_test_root(test_name: &str) -> PathBuf {
    ensure_fake_darktable_cli();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-operator-recovery-{test_name}-{stamp}"))
}

static FAKE_DARKTABLE_SETUP: Once = Once::new();

fn ensure_fake_darktable_cli() {
    FAKE_DARKTABLE_SETUP.call_once(|| {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", script_path);
    });
}

#[test]
fn operator_recovery_summary_exposes_preview_render_actions_only() {
    let base_dir = unique_test_root("preview-actions");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir);

    let summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("recovery summary should load");

    assert_eq!(summary.session_id.as_deref(), Some(session_id.as_str()));
    assert_eq!(
        summary.blocked_category.as_deref(),
        Some("preview-or-render")
    );
    assert_eq!(
        summary.allowed_actions,
        vec![
            "retry".to_string(),
            "approved-boundary-restart".to_string(),
            "route-phone-required".to_string()
        ]
    );
    assert!(!summary
        .diagnostics_summary
        .as_ref()
        .expect("diagnostics summary should exist")
        .detail
        .contains("captures/originals"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_recovery_retry_moves_preview_render_blockage_to_ready_state() {
    let base_dir = unique_test_root("preview-retry");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir);

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorRecoveryActionInputDto {
            session_id: session_id.clone(),
            action: "retry".into(),
        },
    )
    .expect("retry should succeed");

    assert_eq!(result.status, "applied");
    assert_eq!(result.next_state.customer_state, "Ready");
    assert_eq!(result.next_state.reason_code, "ready");
    assert_eq!(result.summary.blocked_category, None);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.clone(),
        },
    )
    .expect("readiness should refresh");
    let preview_path = readiness
        .latest_capture
        .as_ref()
        .and_then(|capture| capture.preview.asset_path.as_ref())
        .expect("retry should produce a booth-safe preview asset");

    assert!(std::path::Path::new(preview_path).is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_recovery_time_extension_reopens_an_ended_session_within_policy_bounds() {
    let base_dir = unique_test_root("time-extension");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir);

    update_timing(
        &base_dir,
        &session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("timing summary should load");

    assert_eq!(
        summary.blocked_category.as_deref(),
        Some("timing-or-post-end")
    );
    assert!(summary
        .allowed_actions
        .contains(&"approved-time-extension".to_string()));

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorRecoveryActionInputDto {
            session_id: session_id.clone(),
            action: "approved-time-extension".into(),
        },
    )
    .expect("time extension should succeed");

    assert_eq!(result.status, "applied");
    assert_eq!(result.next_state.post_end_state, None);
    assert_eq!(result.summary.post_end_state, None);
    assert!(matches!(
        result.next_state.reason_code.as_str(),
        "preview-waiting" | "ready"
    ));

    let manifest = read_manifest(&base_dir, &session_id);
    let timing = manifest.timing.expect("timing should remain available");

    assert_eq!(timing.approved_extension_minutes, 5);
    assert!(timing.approved_extension_audit_ref.is_some());
    assert_eq!(manifest.post_end, None);
    assert_ne!(manifest.lifecycle.stage, "export-waiting");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_recovery_rejects_time_extension_when_the_session_still_cannot_reopen() {
    let base_dir = unique_test_root("time-extension-too-late");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir);

    update_timing(
        &base_dir,
        &session_id,
        &timestamp_offset(-900),
        &timestamp_offset(-301),
        "ended",
    );

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorRecoveryActionInputDto {
            session_id: session_id.clone(),
            action: "approved-time-extension".into(),
        },
    )
    .expect("late time extension should return a typed rejection");

    assert_eq!(result.status, "rejected");
    assert_eq!(
        result.rejection_reason.as_deref(),
        Some("recovery-unavailable")
    );

    let manifest = read_manifest(&base_dir, &session_id);
    let timing = manifest.timing.expect("timing should remain available");

    assert_eq!(timing.approved_extension_minutes, 0);
    assert_eq!(timing.ended_triggered_at.is_some(), true);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_recovery_rejection_keeps_the_actual_booth_reason_for_capture_blocked_sessions() {
    let base_dir = unique_test_root("capture-blocked-rejection");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_unconfigured_session(&base_dir);

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorRecoveryActionInputDto {
            session_id: session_id.clone(),
            action: "approved-boundary-restart".into(),
        },
    )
    .expect("capture-blocked restart should return a typed rejection");

    assert_eq!(result.status, "rejected");
    assert_eq!(
        result.rejection_reason.as_deref(),
        Some("recovery-unavailable")
    );
    assert_eq!(result.next_state.customer_state, "Preparing");
    assert_eq!(result.next_state.reason_code, "preset-missing");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_recovery_routes_to_phone_required_without_damaging_session_assets() {
    let base_dir = unique_test_root("route-phone-required");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir);
    let manifest_before = read_manifest(&base_dir, &session_id);
    let raw_asset_path = manifest_before
        .captures
        .last()
        .map(|capture| capture.raw.asset_path.clone())
        .expect("capture should exist");

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorRecoveryActionInputDto {
            session_id: session_id.clone(),
            action: "route-phone-required".into(),
        },
    )
    .expect("phone required routing should succeed");

    assert_eq!(result.status, "applied");
    assert_eq!(result.next_state.customer_state, "Phone Required");
    assert_eq!(result.next_state.reason_code, "phone-required");
    assert_eq!(
        result.summary.post_end_state.as_deref(),
        Some("phone-required")
    );

    let manifest_after = read_manifest(&base_dir, &session_id);

    assert_eq!(manifest_after.lifecycle.stage, "phone-required");
    assert_eq!(
        manifest_after
            .post_end
            .as_ref()
            .map(|post_end| post_end.state()),
        Some("phone-required")
    );
    assert!(std::path::Path::new(&raw_asset_path).is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_recovery_rejects_foreign_session_actions_without_mutating_the_current_session() {
    let base_dir = unique_test_root("foreign-session");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let older_session_id = create_preview_waiting_session(&base_dir);
    std::thread::sleep(std::time::Duration::from_millis(20));
    let current_session_id = create_preview_waiting_session(&base_dir);
    let current_manifest_before = read_manifest(&base_dir, &current_session_id);

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorRecoveryActionInputDto {
            session_id: older_session_id.clone(),
            action: "retry".into(),
        },
    )
    .expect("foreign session requests should be rejected as typed results");

    assert_eq!(result.status, "rejected");
    assert_eq!(result.rejection_reason.as_deref(), Some("session-mismatch"));
    assert_eq!(
        result.summary.session_id.as_deref(),
        Some(current_session_id.as_str())
    );

    let current_manifest_after = read_manifest(&base_dir, &current_session_id);

    assert_eq!(
        current_manifest_before.updated_at,
        current_manifest_after.updated_at
    );

    let _ = fs::remove_dir_all(base_dir);
}

fn create_preview_waiting_session(base_dir: &PathBuf) -> String {
    let session = start_session_in_dir(
        base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = base_dir.join("preset-catalog").join("published");

    create_named_published_bundle(&catalog_root, "preset_soft-glow", "Soft Glow", "2026.03.26");

    select_active_preset_in_dir(
        base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        },
    )
    .expect("preset should become active");
    write_ready_helper_status(base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(base_dir, &session.session_id);

    let raw_asset_path = capture_result.capture.raw.asset_path;

    assert!(std::path::Path::new(&raw_asset_path).is_file());

    session.session_id
}

fn create_unconfigured_session(base_dir: &PathBuf) -> String {
    start_session_in_dir(
        base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created")
    .session_id
}

fn read_manifest(base_dir: &PathBuf, session_id: &str) -> SessionManifest {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(manifest_path).expect("manifest should be readable");

    serde_json::from_str(&manifest_bytes).expect("manifest should deserialize")
}

fn request_capture_with_helper_success(
    base_dir: &PathBuf,
    session_id: &str,
) -> boothy_lib::contracts::dto::CaptureRequestResultDto {
    let helper_base_dir = base_dir.clone();
    let helper_session_id = session_id.to_string();

    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("operator_recovery_capture.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("operator recovery raw path should have a parent"),
        )
        .expect("operator recovery raw dir should exist");
        fs::write(&raw_path, b"helper-raw").expect("operator recovery raw should exist");

        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": request.session_id,
              "requestId": request.request_id,
            }),
        );
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
              "type": "file-arrived",
              "sessionId": request.session_id,
              "requestId": request.request_id,
              "captureId": "capture_operator_recovery",
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    let result = boothy_lib::capture::normalized_state::request_capture_in_dir(
        base_dir,
        boothy_lib::contracts::dto::CaptureRequestInputDto {
            session_id: session_id.into(),
            request_id: None,
        },
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("operator recovery helper thread should complete");

    result
}

fn wait_for_latest_capture_request(
    base_dir: &PathBuf,
    session_id: &str,
) -> boothy_lib::capture::sidecar_client::CanonHelperCaptureRequestMessage {
    for _ in 0..200 {
        let requests = read_capture_request_messages(base_dir, session_id)
            .expect("operator recovery request log should be readable");

        if let Some(request) = requests.last() {
            return request.clone();
        }

        thread::sleep(Duration::from_millis(10));
    }

    panic!("operator recovery request should have been written")
}

fn append_helper_event(base_dir: &PathBuf, session_id: &str, event: serde_json::Value) {
    let event_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_EVENTS_FILE_NAME);
    fs::create_dir_all(
        event_path
            .parent()
            .expect("operator recovery event path should have a parent"),
    )
    .expect("operator recovery event dir should exist");

    let serialized_event =
        serde_json::to_string(&event).expect("operator recovery event should serialize");
    let existing = fs::read_to_string(&event_path).unwrap_or_default();
    let next_contents = if existing.trim().is_empty() {
        format!("{serialized_event}\n")
    } else {
        format!("{existing}{serialized_event}\n")
    };

    fs::write(event_path, next_contents).expect("operator recovery event log should be writable");
}

fn update_timing(
    base_dir: &PathBuf,
    session_id: &str,
    warning_at: &str,
    adjusted_end_at: &str,
    phase: &str,
) {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    let timing = manifest
        .timing
        .as_mut()
        .expect("session timing should exist");
    timing.warning_at = warning_at.into();
    timing.adjusted_end_at = adjusted_end_at.into();
    timing.phase = phase.into();
    timing.capture_allowed = phase != "ended";
    timing.warning_triggered_at = None;
    timing.ended_triggered_at = None;

    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");
}

fn timestamp_offset(offset_seconds: i64) -> String {
    let now = SystemTime::now();
    let shifted = if offset_seconds >= 0 {
        now.checked_add(std::time::Duration::from_secs(offset_seconds as u64))
            .expect("future timestamp should be valid")
    } else {
        now.checked_sub(std::time::Duration::from_secs(
            offset_seconds.unsigned_abs(),
        ))
        .expect("past timestamp should be valid")
    };

    current_timestamp(shifted).expect("shifted timestamp should serialize")
}

fn create_named_published_bundle(
    catalog_root: &PathBuf,
    preset_id: &str,
    display_name: &str,
    published_version: &str,
) {
    let bundle_dir = catalog_root.join(preset_id).join(published_version);
    fs::create_dir_all(&bundle_dir).expect("bundle directory should exist");
    fs::create_dir_all(bundle_dir.join("xmp")).expect("xmp directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should exist");
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
    .expect("xmp template should exist");

    let bundle = serde_json::json!({
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
        "altText": format!("{display_name} sample portrait"),
      }
    });

    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&bundle).expect("bundle should serialize"),
    )
    .expect("bundle should be writable");
}

fn write_ready_helper_status(base_dir: &PathBuf, session_id: &str) {
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
          "observedAt": current_timestamp(SystemTime::now())
            .expect("helper timestamp should serialize"),
          "cameraState": "ready",
          "helperState": "healthy"
        }))
        .expect("helper status should serialize"),
    )
    .expect("helper status should be writable");
}
