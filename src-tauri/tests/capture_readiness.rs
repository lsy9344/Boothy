use std::{
    fs,
    path::PathBuf,
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    capture::{
        ingest_pipeline::{complete_preview_render_in_dir, mark_preview_render_failed_in_dir},
        normalized_state::{
            delete_capture_in_dir, get_capture_readiness_in_dir, request_capture_in_dir,
        },
        sidecar_client::{
            read_capture_request_messages, CanonHelperCaptureRequestMessage,
            CAMERA_HELPER_EVENTS_FILE_NAME, CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
            CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        },
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureReadinessInputDto, CaptureRequestInputDto,
        CaptureRequestResultDto, SessionStartInputDto,
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
fn readiness_recovers_from_a_manifest_backup_left_during_an_atomic_swap_gap() {
    let base_dir = unique_test_root("manifest-backup-gap");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let backup_path = manifest_path.with_extension("json.bak");

    fs::rename(&manifest_path, &backup_path).expect("manifest should move to backup");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should recover from the backup manifest");

    assert_eq!(readiness.session_id, session.session_id);
    assert_eq!(readiness.reason_code, "preset-missing");
    assert!(manifest_path.is_file());
    assert!(!backup_path.exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_stays_blocked_until_live_helper_truth_is_fresh() {
    let base_dir = unique_test_root("missing-helper-truth");
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

    assert_eq!(readiness.customer_state, "Preparing");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.primary_action, "wait");
    assert_eq!(readiness.reason_code, "camera-preparing");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("blocked readiness should still expose live capture truth");
    assert_eq!(live_truth.source, "canon-helper-sidecar");
    assert_eq!(live_truth.freshness, "missing");
    assert_eq!(live_truth.camera_state, "unknown");
    assert_eq!(live_truth.helper_state, "unknown");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_returns_ready_once_session_preset_and_live_helper_truth_are_valid() {
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
    write_ready_helper_status(&base_dir, &session.session_id);

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
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("ready readiness should include fresh live capture truth");
    assert_eq!(live_truth.source, "canon-helper-sidecar");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.session_match, "matched");
    assert_eq!(live_truth.camera_state, "ready");
    assert_eq!(live_truth.helper_state, "healthy");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_accepts_utf8_bom_prefixed_helper_status_files() {
    let base_dir = unique_test_root("ready-bom-status");
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
    write_ready_helper_status_with_utf8_bom(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve from a helper status file with a UTF-8 BOM");

    assert_eq!(readiness.customer_state, "Ready");
    assert!(readiness.can_capture);
    assert_eq!(readiness.reason_code, "ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_surfaces_camera_power_guidance_when_helper_reports_disconnected() {
    let base_dir = unique_test_root("camera-disconnected-guidance");
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
    write_helper_status(
        &base_dir,
        &session.session_id,
        "disconnected",
        "healthy",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert_eq!(readiness.reason_code, "camera-preparing");
    assert_eq!(readiness.customer_message, "카메라 전원을 확인하고 있어요.");
    assert_eq!(
        readiness.support_message,
        "카메라를 켜고 연결이 안정되면 바로 촬영할 수 있어요."
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_surfaces_camera_connecting_guidance_when_helper_detects_powered_device() {
    let base_dir = unique_test_root("camera-connecting-guidance");
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
    write_helper_status(
        &base_dir,
        &session.session_id,
        "connecting",
        "healthy",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert_eq!(readiness.reason_code, "camera-preparing");
    assert_eq!(
        readiness.customer_message,
        "카메라를 확인했고 연결을 마무리하고 있어요."
    );
    assert_eq!(readiness.support_message, "잠시만 기다려 주세요.");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_stays_blocked_when_helper_truth_is_absent_even_in_runtime_dir() {
    let local_app_data_root = unique_test_root("runtime-probe-localappdata");
    let base_dir = local_app_data_root.join("com.tauri.dev").join("dabi_shoot");
    let _env_guard = scoped_env_vars(vec![(
        "LOCALAPPDATA",
        Some(std::ffi::OsString::from(local_app_data_root.clone())),
    )]);
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
    .expect("runtime readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "camera-preparing");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("missing helper truth should still expose live capture truth");
    assert_eq!(live_truth.source, "canon-helper-sidecar");
    assert_eq!(live_truth.freshness, "missing");

    let _ = fs::remove_dir_all(local_app_data_root);
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
    write_ready_helper_status(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);

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
    assert_eq!(
        fs::read(
            ready_capture
                .preview
                .asset_path
                .as_deref()
                .expect("preview path should exist after completion"),
        )
        .expect("preview file should be readable"),
        b"helper-raw",
    );

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
fn capture_flow_prefers_a_sidecar_generated_preview_for_raw_only_captures() {
    let base_dir = unique_test_root("capture-preview-sidecar-generated");
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let capture_id = capture_result.capture.capture_id.clone();
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let manifest_path = session_paths.manifest_path.clone();
    let raw_cr3_path = session_paths
        .captures_originals_dir
        .join(format!("{capture_id}.cr3"));
    fs::write(&raw_cr3_path, b"helper-raw-cr3").expect("cr3 raw should be writable");

    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.captures[0].raw.asset_path = raw_cr3_path.to_string_lossy().into_owned();
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    fs::create_dir_all(&session_paths.renders_previews_dir)
        .expect("preview directory should exist");
    let sidecar_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{capture_id}.jpg"));
    fs::write(&sidecar_preview_path, b"sidecar-preview-jpg")
        .expect("sidecar preview should be writable");

    let ready_capture = complete_preview_render_in_dir(&base_dir, &session.session_id, &capture_id)
        .expect("preview render should complete with sidecar preview");

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(sidecar_preview_path.to_string_lossy().as_ref()),
    );
    assert_eq!(
        fs::read(&sidecar_preview_path).expect("preview file should be readable"),
        b"sidecar-preview-jpg",
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_repairs_a_placeholder_svg_preview_when_a_raster_sidecar_exists() {
    let base_dir = unique_test_root("capture-preview-repair-from-sidecar");
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect("preview render should complete");
    let capture_id = ready_capture.capture_id.clone();
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let svg_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{capture_id}.svg"));
    let jpg_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{capture_id}.jpg"));
    fs::write(&svg_preview_path, b"<svg></svg>").expect("svg placeholder should be writable");
    fs::write(&jpg_preview_path, b"rendered-jpg").expect("jpg preview should be writable");

    let manifest_path = session_paths.manifest_path.clone();
    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.captures[0].preview.asset_path = Some(svg_preview_path.to_string_lossy().into_owned());
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should repair the preview asset path");

    assert_eq!(
        readiness
            .latest_capture
            .as_ref()
            .and_then(|capture| capture.preview.asset_path.as_ref()),
        Some(&jpg_preview_path.to_string_lossy().into_owned()),
    );

    let repaired_manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should still be readable"),
    )
    .expect("manifest should deserialize after repair");
    assert_eq!(
        repaired_manifest.captures[0].preview.asset_path.as_deref(),
        Some(jpg_preview_path.to_string_lossy().as_ref()),
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_waits_briefly_for_a_delayed_sidecar_preview_before_falling_back() {
    let base_dir = unique_test_root("capture-preview-delayed-sidecar");
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let capture_id = capture_result.capture.capture_id.clone();
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let manifest_path = session_paths.manifest_path.clone();
    let raw_cr3_path = session_paths
        .captures_originals_dir
        .join(format!("{capture_id}.cr3"));
    fs::write(&raw_cr3_path, b"helper-raw-cr3").expect("cr3 raw should be writable");

    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.captures[0].raw.asset_path = raw_cr3_path.to_string_lossy().into_owned();
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    fs::create_dir_all(&session_paths.renders_previews_dir)
        .expect("preview directory should exist");
    let sidecar_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{capture_id}.jpg"));
    let delayed_preview_path = sidecar_preview_path.clone();

    let preview_writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        fs::write(&delayed_preview_path, b"delayed-sidecar-preview")
            .expect("delayed sidecar preview should be writable");
    });

    let ready_capture = complete_preview_render_in_dir(&base_dir, &session.session_id, &capture_id)
        .expect("preview render should wait for the delayed sidecar preview");

    preview_writer
        .join()
        .expect("delayed preview writer should complete");

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(sidecar_preview_path.to_string_lossy().as_ref()),
    );
    assert_eq!(
        fs::read(&sidecar_preview_path).expect("preview file should be readable"),
        b"delayed-sidecar-preview",
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
    write_ready_helper_status(&base_dir, &session.session_id);

    request_capture_with_helper_success(&base_dir, &session.session_id);

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
fn capture_flow_times_out_when_helper_accepts_but_no_file_arrives() {
    let base_dir = unique_test_root("capture-timeout-no-file");
    write_capture_timeout_override(&base_dir, 75);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
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
    });

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect_err("accepted without a file should time out");

    helper_thread
        .join()
        .expect("helper timeout thread should complete");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .expect("timeout should include readiness")
            .reason_code,
        "phone-required",
    );
    let manifest = read_manifest(&base_dir, &session.session_id);
    assert!(manifest.captures.is_empty());
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("follow-up readiness should resolve");
    assert_eq!(readiness.reason_code, "phone-required");
    assert_eq!(readiness.customer_state, "Phone Required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_rejects_file_arrivals_from_the_wrong_session() {
    let base_dir = unique_test_root("capture-wrong-session");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let foreign_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Lee".into(),
            phone_last_four: "1234".into(),
        },
    )
    .expect("foreign session should be created");
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let wrong_session_id = foreign_session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_wrong_session.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("wrong-session raw path should have a parent"),
        )
        .expect("raw directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("wrong-session raw should exist");

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
              "sessionId": wrong_session_id,
              "requestId": request.request_id,
              "captureId": "capture_wrong_session",
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect_err("wrong-session file arrival should be rejected");

    helper_thread
        .join()
        .expect("wrong-session helper thread should complete");

    assert_eq!(error.code, "capture-not-ready");
    assert!(read_manifest(&base_dir, &session.session_id)
        .captures
        .is_empty());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_rejects_file_arrivals_for_the_wrong_request() {
    let base_dir = unique_test_root("capture-wrong-request");
    write_capture_timeout_override(&base_dir, 100);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_wrong_request.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("wrong-request raw path should have a parent"),
        )
        .expect("raw directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("wrong-request raw should exist");

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
              "requestId": format!("{}_stale", request.request_id),
              "captureId": "capture_wrong_request",
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect_err("wrong-request file arrival should not create success");

    helper_thread
        .join()
        .expect("wrong-request helper thread should complete");

    assert_eq!(error.code, "capture-not-ready");
    assert!(read_manifest(&base_dir, &session.session_id)
        .captures
        .is_empty());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_ignores_duplicate_file_arrivals_after_persisting_once() {
    let base_dir = unique_test_root("capture-duplicate-arrival");
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        let capture_id = "capture_duplicate".to_string();
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("duplicate raw path should have a parent"),
        )
        .expect("raw directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("duplicate raw should exist");

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
        for _ in 0..2 {
            append_helper_event(
                &helper_base_dir,
                &helper_session_id,
                serde_json::json!({
                  "schemaVersion": CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
                  "type": "file-arrived",
                  "sessionId": request.session_id,
                  "requestId": request.request_id,
                  "captureId": capture_id,
                  "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
                  "rawPath": raw_path.to_string_lossy().into_owned(),
                }),
            );
        }
    });

    let capture = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("duplicate arrivals should still save once");

    helper_thread
        .join()
        .expect("duplicate helper thread should complete");

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(capture.capture.capture_id, "capture_duplicate");
    assert_eq!(manifest.captures.len(), 1);
    assert_eq!(manifest.captures[0].capture_id, "capture_duplicate");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_blocks_a_second_capture_while_the_first_round_trip_is_in_flight() {
    let base_dir = unique_test_root("capture-in-flight-second-block");
    write_capture_timeout_override(&base_dir, 1000);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let request_base_dir = base_dir.clone();
    let request_session_id = session.session_id.clone();
    let capture_thread = thread::spawn(move || {
        request_capture_in_dir(
            &request_base_dir,
            CaptureRequestInputDto {
                session_id: request_session_id,
            },
        )
    });

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        thread::sleep(Duration::from_millis(100));
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_in_flight.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("in-flight raw path should have a parent"),
        )
        .expect("raw directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("in-flight raw should exist");

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
              "captureId": "capture_in_flight",
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    wait_for_any_capture_request(&base_dir, &session.session_id);

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect_err("second capture should be blocked during the first round trip");

    helper_thread
        .join()
        .expect("in-flight helper thread should complete");
    let first_capture = capture_thread
        .join()
        .expect("first capture thread should complete")
        .expect("first capture should still succeed");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .expect("in-flight block should include readiness")
            .reason_code,
        "camera-preparing",
    );
    assert_eq!(first_capture.capture.capture_id, "capture_in_flight");

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let first_ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first preview should complete");

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
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
    write_ready_helper_status(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);
    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);
    request_capture_with_helper_success(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);

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
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
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
        write_ready_helper_status(&base_dir, session_id);
    }

    let first_capture = request_capture_with_helper_success(&base_dir, &first_session.session_id);
    let second_capture = request_capture_with_helper_success(&base_dir, &second_session.session_id);

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

fn request_capture_with_helper_success(
    base_dir: &PathBuf,
    session_id: &str,
) -> CaptureRequestResultDto {
    let readiness = get_capture_readiness_in_dir(
        base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.into(),
        },
    )
    .expect("pre-capture readiness should resolve");
    assert!(
        readiness.can_capture,
        "helper success precondition expected ready readiness, got {}",
        readiness.reason_code
    );
    let existing_request_count = capture_request_count(base_dir, session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session_id.to_string();

    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");

        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": helper_session_id,
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
              "captureId": capture_id,
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    let result = request_capture_in_dir(
        base_dir,
        CaptureRequestInputDto {
            session_id: session_id.into(),
        },
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    result
}

fn wait_for_latest_capture_request(
    base_dir: &PathBuf,
    session_id: &str,
    existing_request_count: usize,
) -> CanonHelperCaptureRequestMessage {
    for _ in 0..200 {
        let requests = read_capture_request_messages(base_dir, session_id)
            .expect("capture request log should be readable");

        if requests.len() > existing_request_count {
            if let Some(request) = requests.last() {
                return request.clone();
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    panic!("capture request should have been written")
}

fn capture_request_count(base_dir: &PathBuf, session_id: &str) -> usize {
    read_capture_request_messages(base_dir, session_id)
        .expect("capture request log should be readable")
        .len()
}

fn wait_for_any_capture_request(
    base_dir: &PathBuf,
    session_id: &str,
) -> CanonHelperCaptureRequestMessage {
    for _ in 0..200 {
        let requests = read_capture_request_messages(base_dir, session_id)
            .expect("capture request log should be readable");

        if let Some(request) = requests.last() {
            return request.clone();
        }

        thread::sleep(Duration::from_millis(10));
    }

    panic!("capture request should have been written")
}

fn append_helper_event(base_dir: &PathBuf, session_id: &str, event: serde_json::Value) {
    let event_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_EVENTS_FILE_NAME);
    fs::create_dir_all(
        event_path
            .parent()
            .expect("helper event path should have a parent directory"),
    )
    .expect("helper event directory should exist");

    let serialized_event = serde_json::to_string(&event).expect("helper event should serialize");
    let existing = fs::read_to_string(&event_path).unwrap_or_default();
    let next_contents = if existing.trim().is_empty() {
        format!("{serialized_event}\n")
    } else {
        format!("{existing}{serialized_event}\n")
    };

    fs::write(event_path, next_contents).expect("helper event log should be writable");
}

fn write_capture_timeout_override(base_dir: &PathBuf, timeout_ms: u64) {
    fs::create_dir_all(base_dir).expect("capture timeout root should exist");
    fs::write(
        base_dir.join(".camera-helper-capture-timeout-ms"),
        timeout_ms.to_string(),
    )
    .expect("capture timeout override should be writable");
}

fn write_ready_helper_status(base_dir: &PathBuf, session_id: &str) {
    write_helper_status(
        base_dir,
        session_id,
        "ready",
        "healthy",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
    );
}

fn write_ready_helper_status_with_utf8_bom(base_dir: &PathBuf, session_id: &str) {
    let status_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("camera-helper-status.json");
    let payload = serde_json::to_string(&serde_json::json!({
      "schemaVersion": "canon-helper-status/v1",
      "type": "camera-status",
      "sessionId": session_id,
      "sequence": 1,
      "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
      "cameraState": "ready",
      "helperState": "healthy"
    }))
    .expect("helper status should serialize");

    fs::create_dir_all(
        status_path
            .parent()
            .expect("helper status should have a diagnostics directory"),
    )
    .expect("diagnostics directory should exist");
    fs::write(status_path, format!("\u{feff}{payload}")).expect("helper status should be writable");
}

fn write_helper_status(
    base_dir: &PathBuf,
    session_id: &str,
    camera_state: &str,
    helper_state: &str,
    observed_at: &str,
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
          "helperState": helper_state
        }))
        .expect("helper status should serialize"),
    )
    .expect("helper status should be writable");
}

struct ScopedEnvVarGuard {
    original_values: Vec<(String, Option<std::ffi::OsString>)>,
}

impl Drop for ScopedEnvVarGuard {
    fn drop(&mut self) {
        for (key, original_value) in self.original_values.drain(..).rev() {
            match original_value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn scoped_env_vars(values: Vec<(&str, Option<std::ffi::OsString>)>) -> ScopedEnvVarGuard {
    let mut original_values = Vec::new();

    for (key, next_value) in values {
        original_values.push((key.to_string(), std::env::var_os(key)));
        match next_value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }

    ScopedEnvVarGuard { original_values }
}
