use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Barrier, Once},
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
        CaptureReadinessInputDto, DraftNoisePolicyDto, DraftPresetEditPayloadDto,
        DraftPresetPreviewReferenceDto, DraftRenderProfileDto, OperatorAuditQueryFilterDto,
        PublishValidatedPresetInputDto, RollbackPresetCatalogInputDto, SessionStartInputDto,
        ValidateDraftPresetInputDto,
    },
    diagnostics::{
        audit_log::{
            append_operator_audit_record, load_operator_audit_history_in_dir,
            OperatorAuditRecordInput,
        },
        recovery::execute_operator_recovery_action_in_dir,
    },
    preset::{
        authoring_pipeline::{
            create_draft_preset_in_dir, publish_validated_preset_in_dir,
            resolve_draft_authoring_root, validate_draft_preset_in_dir,
        },
        preset_catalog::resolve_published_preset_catalog_dir,
        preset_catalog_state::rollback_preset_catalog_in_dir,
    },
    session::{
        session_manifest::{current_timestamp, SessionManifest},
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

static FAKE_DARKTABLE_SETUP: Once = Once::new();

fn setup_fake_darktable() {
    FAKE_DARKTABLE_SETUP.call_once(|| {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", script_path);
    });
}

fn unique_test_root(test_name: &str) -> PathBuf {
    setup_fake_darktable();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-operator-audit-{test_name}-{stamp}"))
}

#[test]
fn operator_audit_records_lifecycle_timing_post_end_and_operator_intervention_history() {
    let base_dir = unique_test_root("runtime-history");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir, "2026.03.26");

    update_timing(
        &base_dir,
        &session_id,
        &timestamp_offset(-30),
        &timestamp_offset(60),
        "active",
    );
    let _ = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.clone(),
        },
    )
    .expect("warning sync should succeed");

    update_timing(
        &base_dir,
        &session_id,
        &timestamp_offset(-90),
        &timestamp_offset(-1),
        "warning",
    );
    mark_latest_capture_render_failed(&base_dir, &session_id);
    let _ = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.clone(),
        },
    )
    .expect("ended sync should succeed");

    let _ = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        boothy_lib::contracts::dto::OperatorRecoveryActionInputDto {
            session_id: session_id.clone(),
            action: "route-phone-required".into(),
        },
    )
    .expect("operator intervention should succeed");

    let history = load_operator_audit_history_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorAuditQueryFilterDto {
            session_id: Some(session_id.clone()),
            event_categories: Vec::new(),
            limit: Some(20),
        },
    )
    .expect("audit history should load");

    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "session-started"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "warning-triggered"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "session-ended"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "post-end-phone-required"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "route-phone-required"));
    assert!(history.summary.critical_failure_events >= 1);
    assert!(history.summary.operator_intervention_events >= 1);
    assert!(history
        .events
        .iter()
        .all(|entry| entry.event_id.len() <= 64));

    let manifest = read_manifest(&base_dir, &session_id);
    let raw_asset_path = manifest
        .captures
        .last()
        .map(|capture| capture.raw.asset_path.clone())
        .expect("capture should still exist");
    assert!(Path::new(&raw_asset_path).is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_audit_records_publication_rejection_success_and_catalog_rollback_without_touching_active_session_truth(
) {
    let base_dir = unique_test_root("publication-history");
    let authoring_capability = capability_snapshot_for_profile("authoring-enabled", true);
    let operator_capability = capability_snapshot_for_profile("operator-enabled", true);
    let session_id = create_preview_waiting_session(&base_dir, "2026.03.26");

    create_draft_preset_in_dir(
        &base_dir,
        &authoring_capability,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    create_published_bundle(
        &resolve_published_preset_catalog_dir(&base_dir),
        "preset_soft-glow-draft",
        "2026.03.26",
        "Soft Glow Draft",
    );
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation = validate_draft_preset_in_dir(
        &base_dir,
        &authoring_capability,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should succeed");

    let _ = publish_validated_preset_in_dir(
        &base_dir,
        &authoring_capability,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation.draft.draft_version,
            validation_checked_at: validation.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect("duplicate version should return a typed rejection");

    let _ = publish_validated_preset_in_dir(
        &base_dir,
        &authoring_capability,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation.draft.draft_version,
            validation_checked_at: validation.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.27".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: Some("현재 세션 유지".into()),
        },
    )
    .expect("publication should succeed");

    let rollback = rollback_preset_catalog_in_dir(
        &base_dir,
        &authoring_capability,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            target_published_version: "2026.03.26".into(),
            expected_catalog_revision: 2,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect("rollback should succeed");

    let history = load_operator_audit_history_in_dir(
        &base_dir,
        &operator_capability,
        OperatorAuditQueryFilterDto {
            session_id: None,
            event_categories: vec!["publication-recovery".into()],
            limit: Some(20),
        },
    )
    .expect("publication audit history should load");

    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "publication-rejected"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "publication-approved"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "publication-published"));
    assert!(history
        .events
        .iter()
        .any(|entry| entry.event_type == "catalog-rollback"));
    assert!(history.summary.publication_recovery_events >= 4);

    let manifest = read_manifest(&base_dir, &session_id);
    assert_eq!(
        manifest
            .active_preset
            .as_ref()
            .map(|preset| preset.published_version.as_str()),
        Some("2026.03.26")
    );

    let publication_history_path = base_dir
        .join("preset-authoring")
        .join("publication-audit")
        .join("preset_soft-glow-draft.json");
    fs::write(&publication_history_path, "{ malformed legacy json")
        .expect("legacy publication artifact should be corrupted");

    let after_corruption = load_operator_audit_history_in_dir(
        &base_dir,
        &operator_capability,
        OperatorAuditQueryFilterDto {
            session_id: None,
            event_categories: vec!["publication-recovery".into()],
            limit: Some(20),
        },
    )
    .expect("central audit history should ignore malformed legacy artifacts");
    assert!(after_corruption.summary.publication_recovery_events >= 4);

    match rollback {
        boothy_lib::contracts::dto::RollbackPresetCatalogResultDto::RolledBack {
            summary, ..
        } => {
            assert_eq!(summary.live_published_version, "2026.03.26");
        }
        boothy_lib::contracts::dto::RollbackPresetCatalogResultDto::Rejected { .. } => {
            panic!("expected rollback success")
        }
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_audit_write_failures_do_not_break_current_booth_flow() {
    let base_dir = unique_test_root("audit-write-failure");
    fs::create_dir_all(&base_dir).expect("base dir should exist");
    fs::write(base_dir.join("diagnostics"), "blocked")
        .expect("root diagnostics path should block central audit store writes");

    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session start should not fail when audit storage is unavailable");

    create_published_bundle(
        &resolve_published_preset_catalog_dir(&base_dir),
        "preset_soft-glow",
        "2026.03.26",
        "Soft Glow",
    );
    let selection = select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.26".into(),
        },
    )
    .expect("preset selection should still succeed");

    assert_eq!(selection.session_id, session.session_id);
    assert!(SessionPaths::new(&base_dir, &session.session_id)
        .manifest_path
        .is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_audit_does_not_record_session_mismatch_rejections_on_foreign_sessions() {
    let base_dir = unique_test_root("session-mismatch-audit");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let older_session_id = create_preview_waiting_session(&base_dir, "2026.03.26");
    std::thread::sleep(std::time::Duration::from_millis(20));
    let current_session_id = create_preview_waiting_session(&base_dir, "2026.03.26");

    let result = execute_operator_recovery_action_in_dir(
        &base_dir,
        &capability_snapshot,
        boothy_lib::contracts::dto::OperatorRecoveryActionInputDto {
            session_id: older_session_id.clone(),
            action: "retry".into(),
        },
    )
    .expect("foreign session request should return a typed rejection");

    assert_eq!(result.status, "rejected");
    assert_eq!(result.rejection_reason.as_deref(), Some("session-mismatch"));
    assert_eq!(
        result.summary.session_id.as_deref(),
        Some(current_session_id.as_str())
    );

    let older_history = load_operator_audit_history_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorAuditQueryFilterDto {
            session_id: Some(older_session_id.clone()),
            event_categories: vec!["operator-intervention".into()],
            limit: Some(20),
        },
    )
    .expect("foreign session history should load");

    assert!(older_history
        .events
        .iter()
        .all(|entry| { entry.reason_code.as_deref() != Some("session-mismatch") }));

    let current_history = load_operator_audit_history_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorAuditQueryFilterDto {
            session_id: Some(current_session_id),
            event_categories: vec!["operator-intervention".into()],
            limit: Some(20),
        },
    )
    .expect("current session history should load");

    assert!(current_history
        .events
        .iter()
        .all(|entry| { entry.reason_code.as_deref() != Some("session-mismatch") }));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_audit_store_preserves_parallel_appends() {
    let base_dir = unique_test_root("parallel-appends");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let barrier = Arc::new(Barrier::new(8));
    let mut handles = Vec::new();

    for index in 0..8 {
        let base_dir = base_dir.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();

            append_operator_audit_record(
                &base_dir,
                OperatorAuditRecordInput {
                    occurred_at: "2026-03-27T00:10:00Z".into(),
                    session_id: Some(format!("session_{index:026x}")),
                    event_category: "operator-intervention",
                    event_type: "retry",
                    summary: format!("parallel audit {index}"),
                    detail: "parallel write regression guard".into(),
                    actor_id: None,
                    source: "operator-console",
                    capture_id: None,
                    preset_id: None,
                    published_version: None,
                    reason_code: None,
                },
            )
        }));
    }

    for handle in handles {
        handle
            .join()
            .expect("parallel append thread should join")
            .expect("parallel append should succeed");
    }

    let history = load_operator_audit_history_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorAuditQueryFilterDto {
            session_id: None,
            event_categories: vec!["operator-intervention".into()],
            limit: Some(20),
        },
    )
    .expect("parallel audit history should load");

    let unique_event_ids = history
        .events
        .iter()
        .map(|entry| entry.event_id.clone())
        .collect::<HashSet<_>>();

    assert_eq!(history.summary.total_events, 8);
    assert_eq!(history.events.len(), 8);
    assert_eq!(unique_event_ids.len(), 8);
    assert!(history
        .events
        .iter()
        .all(|entry| entry.event_id.len() <= 64));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_audit_history_reads_backup_store_during_atomic_swap_gap() {
    let base_dir = unique_test_root("backup-read");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let diagnostics_dir = base_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir should exist");
    fs::write(
        diagnostics_dir.join("operator-audit-log.json.bak"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "operator-audit-store/v1",
          "entries": [{
            "schemaVersion": "operator-audit-entry/v1",
            "eventId": "audit-20260327T001000-0000000a-retry-session",
            "occurredAt": "2026-03-27T00:10:00Z",
            "sessionId": "session_00000000000000000000000000",
            "eventCategory": "operator-intervention",
            "eventType": "retry",
            "summary": "backup audit entry",
            "detail": "reader should fall back to backup during atomic swap",
            "actorId": null,
            "source": "operator-console",
            "captureId": null,
            "presetId": null,
            "publishedVersion": null,
            "reasonCode": null
          }]
        }))
        .expect("backup store should serialize"),
    )
    .expect("backup store should write");

    let history = load_operator_audit_history_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorAuditQueryFilterDto {
            session_id: None,
            event_categories: vec!["operator-intervention".into()],
            limit: Some(10),
        },
    )
    .expect("history should load from backup");

    assert_eq!(history.events.len(), 1);
    assert_eq!(history.summary.total_events, 1);
    assert_eq!(history.events[0].summary, "backup audit entry");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn operator_audit_recovers_from_stale_lock_files() {
    let base_dir = unique_test_root("stale-lock");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let diagnostics_dir = base_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir should exist");
    let lock_path = diagnostics_dir.join("operator-audit-log.lock");
    fs::write(&lock_path, "stale lock").expect("lock file should write");
    std::thread::sleep(std::time::Duration::from_millis(1700));

    append_operator_audit_record(
        &base_dir,
        OperatorAuditRecordInput {
            occurred_at: "2026-03-27T00:10:00Z".into(),
            session_id: Some("session_00000000000000000000000000".into()),
            event_category: "operator-intervention",
            event_type: "retry",
            summary: "stale lock recovered".into(),
            detail: "stale audit lock should not block future writes".into(),
            actor_id: None,
            source: "operator-console",
            capture_id: None,
            preset_id: None,
            published_version: None,
            reason_code: None,
        },
    )
    .expect("stale lock should be cleared before append");

    let history = load_operator_audit_history_in_dir(
        &base_dir,
        &capability_snapshot,
        OperatorAuditQueryFilterDto {
            session_id: None,
            event_categories: vec!["operator-intervention".into()],
            limit: Some(10),
        },
    )
    .expect("history should load after stale lock recovery");

    assert!(!lock_path.exists());
    assert_eq!(history.events.len(), 1);
    assert_eq!(history.events[0].summary, "stale lock recovered");

    let _ = fs::remove_dir_all(base_dir);
}

fn create_preview_waiting_session(base_dir: &Path, published_version: &str) -> String {
    let session = start_session_in_dir(
        base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");

    create_published_bundle(
        &resolve_published_preset_catalog_dir(base_dir),
        "preset_soft-glow",
        published_version,
        "Soft Glow",
    );
    select_active_preset_in_dir(
        base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: published_version.into(),
        },
    )
    .expect("preset should become active");
    write_ready_helper_status(base_dir, &session.session_id);

    let _ = request_capture_with_helper_success(base_dir, &session.session_id);

    session.session_id
}

fn read_manifest(base_dir: &Path, session_id: &str) -> SessionManifest {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(manifest_path).expect("manifest should be readable");

    serde_json::from_str(&manifest_bytes).expect("manifest should deserialize")
}

fn request_capture_with_helper_success(
    base_dir: &Path,
    session_id: &str,
) -> boothy_lib::contracts::dto::CaptureRequestResultDto {
    let helper_base_dir = base_dir.to_path_buf();
    let helper_session_id = session_id.to_string();

    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("operator_audit_capture.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("operator audit raw path should have a parent"),
        )
        .expect("operator audit raw dir should exist");
        fs::write(&raw_path, b"helper-raw").expect("operator audit raw should exist");

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
              "captureId": "capture_operator_audit",
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
        .expect("operator audit helper thread should complete");

    result
}

fn wait_for_latest_capture_request(
    base_dir: &PathBuf,
    session_id: &str,
) -> boothy_lib::capture::sidecar_client::CanonHelperCaptureRequestMessage {
    for _ in 0..200 {
        let requests = read_capture_request_messages(base_dir, session_id)
            .expect("operator audit request log should be readable");

        if let Some(request) = requests.last() {
            return request.clone();
        }

        thread::sleep(Duration::from_millis(10));
    }

    panic!("operator audit request should have been written")
}

fn append_helper_event(base_dir: &PathBuf, session_id: &str, event: serde_json::Value) {
    let event_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_EVENTS_FILE_NAME);
    fs::create_dir_all(
        event_path
            .parent()
            .expect("operator audit event path should have a parent"),
    )
    .expect("operator audit event dir should exist");

    let serialized_event =
        serde_json::to_string(&event).expect("operator audit event should serialize");
    let existing = fs::read_to_string(&event_path).unwrap_or_default();
    let next_contents = if existing.trim().is_empty() {
        format!("{serialized_event}\n")
    } else {
        format!("{existing}{serialized_event}\n")
    };

    fs::write(event_path, next_contents).expect("operator audit event log should be writable");
}

fn update_timing(
    base_dir: &Path,
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

fn mark_latest_capture_render_failed(base_dir: &Path, session_id: &str) {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    let latest_capture = manifest.captures.last_mut().expect("capture should exist");
    latest_capture.render_status = "renderFailed".into();
    latest_capture.post_end_state = "postEndPending".into();

    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");
}

fn sample_draft_payload(preset_id: &str, display_name: &str) -> DraftPresetEditPayloadDto {
    DraftPresetEditPayloadDto {
        preset_id: preset_id.into(),
        display_name: display_name.into(),
        lifecycle_state: "draft".into(),
        darktable_version: "5.4.1".into(),
        darktable_project_path: "darktable/soft-glow.dtpreset".into(),
        xmp_template_path: "xmp/soft-glow.xmp".into(),
        preview_profile: render_profile("preview-standard", "Preview Standard"),
        final_profile: render_profile("final-standard", "Final Standard"),
        noise_policy: DraftNoisePolicyDto {
            policy_id: "balanced-noise".into(),
            display_name: "Balanced Noise".into(),
            reduction_mode: "balanced".into(),
        },
        preview: DraftPresetPreviewReferenceDto {
            asset_path: "previews/soft-glow.jpg".into(),
            alt_text: "Soft Glow draft portrait".into(),
        },
        sample_cut: DraftPresetPreviewReferenceDto {
            asset_path: "samples/soft-glow-cut.jpg".into(),
            alt_text: "Soft Glow sample cut".into(),
        },
        description: Some("부드러운 피부톤 baseline".into()),
        notes: Some("승인 전 내부 검토용".into()),
    }
}

fn render_profile(profile_id: &str, display_name: &str) -> DraftRenderProfileDto {
    DraftRenderProfileDto {
        profile_id: profile_id.into(),
        display_name: display_name.into(),
        output_color_space: "sRGB".into(),
    }
}

fn scaffold_valid_draft_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        "<darktable><history><item operation=\"exposure\"></item></history></darktable>",
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
}

fn create_published_bundle(
    catalog_root: &Path,
    preset_id: &str,
    published_version: &str,
    display_name: &str,
) {
    let bundle_dir = catalog_root.join(preset_id).join(published_version);
    fs::create_dir_all(&bundle_dir).expect("bundle directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), "preview").expect("preview should write");
    let bundle = serde_json::json!({
      "schemaVersion": "published-preset-bundle/v1",
      "presetId": preset_id,
      "displayName": display_name,
      "publishedVersion": published_version,
      "lifecycleStatus": "published",
      "boothStatus": "booth-safe",
      "preview": {
        "kind": "preview-tile",
        "assetPath": "preview.jpg",
        "altText": format!("{display_name} preview"),
      }
    });
    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&bundle).expect("bundle should serialize"),
    )
    .expect("bundle should write");
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

fn write_ready_helper_status(base_dir: &Path, session_id: &str) {
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
