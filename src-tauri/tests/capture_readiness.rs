use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    capture::{
        ingest_pipeline::{complete_preview_render_in_dir, mark_preview_render_failed_in_dir},
        normalized_state::{
            delete_capture_in_dir, get_capture_readiness_in_dir, request_capture_in_dir,
        },
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureReadinessInputDto, CaptureRequestInputDto,
        SessionStartInputDto,
    },
    preset::preset_catalog::resolve_published_preset_catalog_dir,
    session::{
        session_manifest::{current_timestamp, SessionManifest},
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-capture-{test_name}-{stamp}"))
}

#[test]
fn readiness_requires_an_active_preset_before_capture_is_allowed() {
    let base_dir = unique_test_root("preset-required");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.primary_action, "choose-preset");
    assert_eq!(readiness.reason_code, "preset-missing");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_returns_ready_once_session_and_preset_are_valid() {
    let base_dir = unique_test_root("ready");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve");

    assert_eq!(readiness.customer_state, "Ready");
    assert!(readiness.can_capture);
    assert_eq!(readiness.primary_action, "capture");
    assert_eq!(readiness.reason_code, "ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn unknown_lifecycle_stages_do_not_fall_through_to_ready() {
    let base_dir = unique_test_root("unknown-stage");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    update_stage(&base_dir, &session.session_id, "unexpected-stage");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "camera-preparing");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance() {
    let base_dir = unique_test_root("blocked");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    update_stage(&base_dir, &session.session_id, "helper-preparing");

    let helper_readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("helper preparing readiness should resolve");

    assert_eq!(helper_readiness.customer_state, "Preparing");
    assert!(!helper_readiness.can_capture);
    assert_eq!(helper_readiness.reason_code, "helper-preparing");

    update_stage(&base_dir, &session.session_id, "phone-required");

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id,
        },
    )
    .expect_err("phone required should block capture");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .expect("blocked capture should include readiness")
            .reason_code,
        "phone-required",
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn invalidated_active_preset_binding_returns_to_preset_missing() {
    let base_dir = unique_test_root("stale-preset");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    fs::remove_dir_all(catalog_root.join("preset_soft-glow").join("2026.03.20"))
        .expect("published bundle should be removable");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should still resolve");

    assert_eq!(readiness.reason_code, "preset-missing");
    assert!(!readiness.can_capture);

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id,
        },
    )
    .expect_err("stale preset binding should block capture");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .expect("blocked capture should include readiness")
            .reason_code,
        "preset-missing",
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_persists_raw_before_preview_waiting_and_only_exposes_preview_after_ready() {
    let base_dir = unique_test_root("capture-preview-flow");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture_result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should be accepted after raw persistence");

    assert_eq!(capture_result.status, "capture-saved");
    assert_eq!(capture_result.readiness.surface_state, "captureSaved");
    assert!(std::path::Path::new(&capture_result.capture.raw.asset_path).is_file());
    assert!(capture_result.capture.preview.asset_path.is_none());
    assert_eq!(capture_result.capture.render_status, "previewWaiting");
    assert_eq!(capture_result.capture.timing.capture_budget_ms, 1_000);
    assert_eq!(capture_result.capture.timing.preview_budget_ms, 5_000);

    let waiting = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("preview waiting should resolve");

    assert_eq!(waiting.customer_state, "Preview Waiting");
    assert_eq!(waiting.reason_code, "preview-waiting");
    assert_eq!(waiting.surface_state, "previewWaiting");
    assert!(waiting
        .latest_capture
        .as_ref()
        .expect("waiting state should retain latest capture")
        .preview
        .asset_path
        .is_none());

    let ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect("preview render should complete");

    assert!(std::path::Path::new(
        ready_capture
            .preview
            .asset_path
            .as_deref()
            .expect("preview path should exist after completion"),
    )
    .is_file());

    let ready = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("preview ready state should resolve");

    assert_eq!(ready.surface_state, "previewReady");
    assert_eq!(ready.reason_code, "ready");
    assert!(ready.can_capture);
    assert_eq!(
        ready
            .latest_capture
            .as_ref()
            .and_then(|capture| capture.preview.asset_path.as_ref())
            .map(|path| path.ends_with(".jpg")),
        Some(true),
    );
    assert_eq!(
        ready
            .latest_capture
            .as_ref()
            .expect("ready capture should be returned")
            .timing
            .preview_budget_state,
        "withinBudget",
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_blocks_a_follow_up_capture_until_the_latest_preview_is_ready() {
    let base_dir = unique_test_root("capture-follow-up-blocked");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("first capture should save");

    let follow_up_error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect_err("follow-up capture should be blocked while preview is waiting");

    assert_eq!(follow_up_error.code, "capture-not-ready");
    assert_eq!(
        follow_up_error
            .readiness
            .expect("blocked follow-up should include readiness")
            .reason_code,
        "preview-waiting",
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn switching_active_preset_only_changes_future_capture_bindings() {
    let base_dir = unique_test_root("capture-preset-switch-forward-only");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_named_published_bundle(&catalog_root, "preset_soft-glow", "Soft Glow", "2026.03.20");
    create_named_published_bundle(&catalog_root, "preset_mono-pop", "Mono Pop", "2026.03.21");

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("first preset should become active");

    let first_capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("first capture should save");
    let first_ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first preview should complete");

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_mono-pop".into(),
            published_version: "2026.03.21".into(),
        },
    )
    .expect("second preset should become active");

    let second_capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("second capture should save");

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    assert_eq!(
        manifest
            .active_preset
            .as_ref()
            .expect("active preset should still exist")
            .preset_id,
        "preset_mono-pop",
    );
    assert_eq!(manifest.captures.len(), 2);
    assert_eq!(
        manifest.captures[0].active_preset_id.as_deref(),
        Some("preset_soft-glow")
    );
    assert_eq!(manifest.captures[0].active_preset_version, "2026.03.20");
    assert_eq!(
        manifest.captures[0].active_preset_display_name.as_deref(),
        Some("Soft Glow")
    );
    assert_eq!(
        manifest.captures[1].active_preset_id.as_deref(),
        Some("preset_mono-pop")
    );
    assert_eq!(manifest.captures[1].active_preset_version, "2026.03.21");
    assert_eq!(
        manifest.captures[1].active_preset_display_name.as_deref(),
        Some("Mono Pop")
    );
    assert_eq!(
        manifest.captures[0].raw.asset_path,
        first_capture.capture.raw.asset_path,
    );
    assert_eq!(
        manifest.captures[0].preview.asset_path,
        first_ready_capture.preview.asset_path,
    );
    assert_eq!(
        manifest.captures[1].capture_id,
        second_capture.capture.capture_id,
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_render_failure_escalates_to_a_phone_required_boundary() {
    let base_dir = unique_test_root("preview-render-failure");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture_result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");

    mark_preview_render_failed_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect("preview failure boundary should persist");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve after failure");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn completed_post_end_uses_handoff_guidance_when_final_handoff_metadata_is_present() {
    let base_dir = unique_test_root("handoff-ready-guidance");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");
    complete_preview_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("preview should complete");
    mark_capture_final_ready(&base_dir, &session.session_id, &capture.capture.capture_id);
    write_handoff_guidance(
        &base_dir,
        &session.session_id,
        serde_json::json!({
            "approvedRecipientLabel": "Front Desk",
            "primaryActionLabel": "안내된 직원에게 이름을 말씀해 주세요.",
            "showBoothAlias": true
        }),
    );
    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("handoff-ready guidance should resolve");

    assert_eq!(readiness.reason_code, "completed");
    let post_end = readiness.post_end.expect("completed guidance should exist");
    assert_eq!(post_end.state(), "completed");
    assert_eq!(post_end.completion_variant(), Some("handoff-ready"));

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "completed");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn handoff_ready_completion_falls_back_to_local_deliverable_without_destination_metadata() {
    let base_dir = unique_test_root("handoff-ready-fallback");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");
    complete_preview_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("preview should complete");
    mark_capture_final_ready(&base_dir, &session.session_id, &capture.capture.capture_id);
    write_handoff_guidance(
        &base_dir,
        &session.session_id,
        serde_json::json!({
            "primaryActionLabel": "안내를 확인해 주세요."
        }),
    );
    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("handoff-ready completion should resolve with a safe fallback");

    let post_end = readiness.post_end.expect("completed guidance should exist");
    assert_eq!(post_end.state(), "completed");
    assert_eq!(
        post_end.completion_variant(),
        Some("local-deliverable-ready")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn delete_capture_removes_only_the_selected_current_session_artifacts() {
    let base_dir = unique_test_root("delete-current-session-capture");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let first_capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("first capture should save");
    let first_ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first preview should complete");

    let second_capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("second capture should save");
    let second_ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second preview should complete");

    let delete_result = delete_capture_in_dir(
        &base_dir,
        CaptureDeleteInputDto {
            session_id: session.session_id.clone(),
            capture_id: second_capture.capture.capture_id.clone(),
        },
    )
    .expect("selected capture should be deleted");

    assert_eq!(delete_result.status, "capture-deleted");
    assert_eq!(delete_result.manifest.captures.len(), 1);
    assert_eq!(
        delete_result.manifest.captures[0].capture_id,
        first_capture.capture.capture_id,
    );
    assert_eq!(delete_result.readiness.reason_code, "ready");
    assert!(std::path::Path::new(&first_capture.capture.raw.asset_path).is_file());
    assert!(std::path::Path::new(
        &first_ready_capture
            .preview
            .asset_path
            .clone()
            .expect("first preview path should exist")
    )
    .is_file());
    assert!(!std::path::Path::new(&second_capture.capture.raw.asset_path).exists());
    assert!(!std::path::Path::new(
        &second_ready_capture
            .preview
            .asset_path
            .clone()
            .expect("second preview path should exist")
    )
    .exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn delete_capture_blocks_preview_waiting_targets_and_preserves_files() {
    let base_dir = unique_test_root("delete-preview-waiting-blocked");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");

    let error = delete_capture_in_dir(
        &base_dir,
        CaptureDeleteInputDto {
            session_id: session.session_id.clone(),
            capture_id: capture.capture.capture_id.clone(),
        },
    )
    .expect_err("preview waiting capture should be blocked");

    assert_eq!(error.code, "capture-delete-blocked");
    assert_eq!(
        error
            .readiness
            .expect("blocked delete should keep readiness")
            .reason_code,
        "preview-waiting",
    );
    assert!(std::path::Path::new(&capture.capture.raw.asset_path).is_file());

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should still resolve");

    assert_eq!(readiness.reason_code, "preview-waiting");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn delete_capture_preserves_manifest_when_asset_cleanup_cannot_be_staged() {
    let base_dir = unique_test_root("delete-stage-failure");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");
    complete_preview_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("preview should complete");

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    manifest.captures[0].raw.asset_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .to_string_lossy()
        .into_owned();

    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let error = delete_capture_in_dir(
        &base_dir,
        CaptureDeleteInputDto {
            session_id: session.session_id.clone(),
            capture_id: capture.capture.capture_id.clone(),
        },
    )
    .expect_err("directory-backed asset should fail staging");

    assert_eq!(error.code, "session-persistence-failed");

    let persisted_manifest_bytes =
        fs::read_to_string(&manifest_path).expect("manifest should still be readable");
    let persisted_manifest: SessionManifest =
        serde_json::from_str(&persisted_manifest_bytes).expect("manifest should deserialize");

    assert_eq!(persisted_manifest.captures.len(), 1);
    assert_eq!(
        persisted_manifest.captures[0].capture_id,
        capture.capture.capture_id,
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn warning_window_projects_warning_readiness_and_persists_a_warning_audit_log() {
    let base_dir = unique_test_root("timing-warning");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-10),
        &timestamp_offset(60),
        "active",
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("warning readiness should resolve");

    assert_eq!(readiness.reason_code, "warning");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "warning");
    assert_eq!(
        manifest
            .timing
            .as_ref()
            .and_then(|timing| timing.warning_triggered_at.as_ref())
            .map(|value| !value.is_empty()),
        Some(true)
    );

    let log_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("timing-events.log");
    let log = fs::read_to_string(log_path).expect("warning timing log should exist");

    assert!(log.contains("event=warning"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn exact_end_blocks_capture_and_reserves_the_extension_audit_hook_in_logs() {
    let base_dir = unique_test_root("timing-ended");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");
    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save before exact end");

    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("exact-end readiness should project explicit export-waiting");

    assert_eq!(readiness.reason_code, "export-waiting");
    assert!(!readiness.can_capture);
    assert_eq!(
        readiness.post_end.as_ref().map(|post_end| post_end.state()),
        Some("export-waiting")
    );
    assert_eq!(
        readiness
            .timing
            .as_ref()
            .map(|timing| timing.phase.as_str()),
        Some("ended")
    );
    assert_eq!(
        readiness
            .latest_capture
            .as_ref()
            .map(|latest_capture| latest_capture.capture_id.as_str()),
        Some(capture.capture.capture_id.as_str())
    );

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect_err("exact end should block capture");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .as_ref()
            .map(|readiness| readiness.reason_code.as_str()),
        Some("export-waiting")
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "export-waiting");
    assert_eq!(
        manifest.post_end.as_ref().map(|post_end| post_end.state()),
        Some("export-waiting")
    );
    assert_eq!(
        manifest
            .timing
            .as_ref()
            .map(|timing| timing.capture_allowed),
        Some(false)
    );

    let log_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("timing-events.log");
    let log = fs::read_to_string(log_path).expect("ended timing log should exist");

    assert!(log.contains("event=ended"));
    assert!(log.contains("event=extension-hook-reserved"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn ended_readiness_does_not_fall_back_to_preset_missing_after_end() {
    let base_dir = unique_test_root("timing-ended-with-missing-preset");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");
    request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save before exact end");

    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );
    fs::remove_dir_all(&catalog_root).expect("preset catalog should be removable");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("ended readiness should still resolve");

    assert_eq!(readiness.reason_code, "export-waiting");
    assert_ne!(readiness.reason_code, "preset-missing");
    assert_eq!(
        readiness.post_end.as_ref().map(|post_end| post_end.state()),
        Some("export-waiting")
    );
    assert_eq!(
        readiness
            .timing
            .as_ref()
            .map(|timing| timing.phase.as_str()),
        Some("ended")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_ready_capture_promotes_to_completed_local_deliverable_ready_after_exact_end() {
    let base_dir = unique_test_root("timing-completed-local");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");

    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let ended_readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("exact-end readiness should remain blocked before preview finishes");

    assert_eq!(ended_readiness.reason_code, "export-waiting");
    assert_eq!(
        ended_readiness
            .post_end
            .as_ref()
            .map(|post_end| post_end.state()),
        Some("export-waiting")
    );

    complete_preview_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("preview should complete");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("completed post-end readiness should resolve");

    assert_eq!(readiness.reason_code, "completed");
    assert_eq!(
        readiness
            .post_end
            .as_ref()
            .and_then(|post_end| post_end.completion_variant()),
        Some("local-deliverable-ready")
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "completed");
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.post_end_state.as_str()),
        Some("local-deliverable-ready")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn post_end_render_failure_preserves_saved_capture_assets() {
    let base_dir = unique_test_root("timing-phone-required-preserves-assets");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("capture should save");
    let raw_asset_path = capture.capture.raw.asset_path.clone();

    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    mark_preview_render_failed_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("post-end render failure should be recorded");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("phone-required post-end readiness should resolve");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(std::path::Path::new(&raw_asset_path).is_file());

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.capture_id.as_str()),
        Some(capture.capture.capture_id.as_str())
    );
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.post_end_state.as_str()),
        Some("postEndPending")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_ready_capture_remains_scoped_to_its_own_session() {
    let base_dir = unique_test_root("capture-session-isolation");
    let first_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("first session should be created");
    let second_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Lee".into(),
            phone_last_four: "1234".into(),
        },
    )
    .expect("second session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);

    for session_id in [&first_session.session_id, &second_session.session_id] {
        select_active_preset_in_dir(
            &base_dir,
            boothy_lib::contracts::dto::PresetSelectionInputDto {
                session_id: session_id.to_string(),
                preset_id: "preset_soft-glow".into(),
                published_version: "2026.03.20".into(),
            },
        )
        .expect("preset should become active");
    }

    let first_capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: first_session.session_id.clone(),
        },
    )
    .expect("first capture should save");
    let second_capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: second_session.session_id.clone(),
        },
    )
    .expect("second capture should save");

    complete_preview_render_in_dir(
        &base_dir,
        &first_session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first preview should complete");
    complete_preview_render_in_dir(
        &base_dir,
        &second_session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second preview should complete");

    let first_ready = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: first_session.session_id.clone(),
        },
    )
    .expect("first readiness should resolve");
    let second_ready = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: second_session.session_id.clone(),
        },
    )
    .expect("second readiness should resolve");

    assert_eq!(
        first_ready
            .latest_capture
            .as_ref()
            .expect("first latest capture should exist")
            .session_id,
        first_session.session_id,
    );
    assert_eq!(
        second_ready
            .latest_capture
            .as_ref()
            .expect("second latest capture should exist")
            .session_id,
        second_session.session_id,
    );
    assert_ne!(
        first_ready
            .latest_capture
            .as_ref()
            .expect("first capture should exist")
            .capture_id,
        second_ready
            .latest_capture
            .as_ref()
            .expect("second capture should exist")
            .capture_id,
    );

    let _ = fs::remove_dir_all(base_dir);
}

fn mark_capture_final_ready(base_dir: &PathBuf, session_id: &str, capture_id: &str) {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");
    let capture = manifest
        .captures
        .iter_mut()
        .find(|value| value.capture_id == capture_id)
        .expect("capture should exist");

    capture.render_status = "finalReady".into();
    capture.post_end_state = "handoffReady".into();
    capture.final_asset.asset_path = Some(
        SessionPaths::new(base_dir, session_id)
            .renders_finals_dir
            .join(format!("{capture_id}.jpg"))
            .to_string_lossy()
            .into_owned(),
    );
    capture.final_asset.ready_at_ms = Some(capture.raw.persisted_at_ms + 100);

    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");
}

fn write_handoff_guidance(base_dir: &PathBuf, session_id: &str, payload: serde_json::Value) {
    let guidance_path = SessionPaths::new(base_dir, session_id)
        .handoff_dir
        .join("customer-guidance.json");
    fs::create_dir_all(
        guidance_path
            .parent()
            .expect("handoff guidance should have a parent directory"),
    )
    .expect("handoff dir should exist");
    fs::write(
        guidance_path,
        serde_json::to_vec_pretty(&payload).expect("handoff payload should serialize"),
    )
    .expect("handoff guidance should be writable");
}

fn update_stage(base_dir: &PathBuf, session_id: &str, stage: &str) {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    manifest.lifecycle.stage = stage.into();

    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");
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

fn read_manifest(base_dir: &PathBuf, session_id: &str) -> SessionManifest {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(manifest_path).expect("manifest should be readable");

    serde_json::from_str(&manifest_bytes).expect("manifest should deserialize")
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

fn create_published_bundle(catalog_root: &PathBuf) {
    create_named_published_bundle(catalog_root, "preset_soft-glow", "Soft Glow", "2026.03.20");
}

fn create_named_published_bundle(
    catalog_root: &PathBuf,
    preset_id: &str,
    display_name: &str,
    published_version: &str,
) {
    let bundle_dir = catalog_root.join(preset_id).join(published_version);
    fs::create_dir_all(&bundle_dir).expect("bundle directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should exist");

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
        "altText": format!("{display_name} sample portrait"),
      }
    });

    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&bundle).expect("bundle should serialize"),
    )
    .expect("bundle should be writable");
}
