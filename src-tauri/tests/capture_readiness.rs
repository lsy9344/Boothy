use std::{
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex, Once},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    capture::{
        ingest_pipeline::{complete_preview_render_in_dir, mark_preview_render_failed_in_dir},
        normalized_state::{
            delete_capture_in_dir, get_capture_readiness_in_dir, request_capture_in_dir,
            request_capture_in_dir_with_fast_preview,
        },
        sidecar_client::{
            read_capture_request_messages, write_capture_request_message,
            CanonHelperCaptureRequestMessage, CAMERA_HELPER_EVENTS_FILE_NAME,
            CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
            CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION, CANON_HELPER_ERROR_SCHEMA_VERSION,
            CANON_HELPER_FAST_PREVIEW_READY_SCHEMA_VERSION,
            CANON_HELPER_FAST_THUMBNAIL_ATTEMPTED_SCHEMA_VERSION,
            CANON_HELPER_FAST_THUMBNAIL_FAILED_SCHEMA_VERSION,
            CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        },
    },
    commands::runtime_commands::{
        append_capture_client_timing_event_in_dir, CaptureClientDebugLogInputDto,
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureReadinessInputDto, CaptureRequestInputDto,
        CaptureRequestResultDto, SessionStartInputDto,
    },
    preset::default_catalog::ensure_default_preset_catalog_in_dir,
    preset::preset_catalog::resolve_published_preset_catalog_dir,
    render::render_preview_asset_to_path_in_dir,
    session::{
        session_manifest::{current_timestamp, CompletedPostEnd, SessionManifest, SessionPostEnd},
        session_paths::SessionPaths,
        session_repository::{
            select_active_preset_in_dir, set_manifest_write_retryable_failures_for_tests,
            start_session_in_dir,
        },
    },
};

static FAKE_DARKTABLE_SETUP: Once = Once::new();
static SPECULATIVE_PREVIEW_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn unique_test_root(test_name: &str) -> PathBuf {
    ensure_fake_darktable_cli();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-capture-{test_name}-{stamp}"))
}

fn ensure_fake_darktable_cli() {
    FAKE_DARKTABLE_SETUP.call_once(|| {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", script_path);
    });
}

fn assert_valid_jpeg(path: &str) {
    let bytes = fs::read(path).expect("render output should be readable");
    assert!(bytes.len() >= 4, "jpeg output should not be empty");
    assert_eq!(bytes[0], 0xFF, "jpeg should start with SOI");
    assert_eq!(bytes[1], 0xD8, "jpeg should start with SOI");
    assert_eq!(bytes[2], 0xFF, "jpeg should start with marker");
}

fn write_test_jpeg(path: &std::path::Path) {
    fs::write(path, [0xFF, 0xD8, 0xFF, 0xD9]).expect("jpeg fixture should be writable");
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
            request_id: None,
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
            request_id: None,
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
    assert_valid_jpeg(
        ready_capture
            .preview
            .asset_path
            .as_deref()
            .expect("preview path should exist after completion"),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

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
    let ready_capture = ready
        .latest_capture
        .as_ref()
        .expect("ready capture should be returned");
    let expected_preview_budget_state = if ready_capture
        .timing
        .preview_visible_at_ms
        .expect("ready capture should stamp preview visibility timing")
        .saturating_sub(ready_capture.timing.capture_acknowledged_at_ms)
        <= ready_capture.timing.preview_budget_ms
    {
        "withinBudget"
    } else {
        "exceededBudget"
    };
    assert_eq!(
        ready_capture.timing.preview_budget_state,
        expected_preview_budget_state,
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_ignores_sidecar_preview_placeholders_and_uses_bundle_render_output() {
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
        .expect("preview render should complete from the published bundle");

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(sidecar_preview_path.to_string_lossy().as_ref()),
    );
    assert_valid_jpeg(sidecar_preview_path.to_string_lossy().as_ref());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_does_not_repair_a_placeholder_svg_preview_when_a_raster_sidecar_exists() {
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
    write_test_jpeg(&jpg_preview_path);

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
    .expect("readiness should stay readable without repairing the preview asset path");

    assert_eq!(
        readiness
            .latest_capture
            .as_ref()
            .and_then(|capture| capture.preview.asset_path.as_ref()),
        Some(&svg_preview_path.to_string_lossy().into_owned()),
    );

    let repaired_manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should still be readable"),
    )
    .expect("manifest should deserialize after readiness lookup");
    assert_eq!(
        repaired_manifest.captures[0].preview.asset_path.as_deref(),
        Some(svg_preview_path.to_string_lossy().as_ref()),
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_render_rejects_invalid_non_raster_output() {
    let base_dir = unique_test_root("capture-preview-invalid-output");
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
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let forced_raw_path = session_paths
        .captures_originals_dir
        .join("force-invalid-output.jpg");
    fs::write(&forced_raw_path, b"raw-marker").expect("raw marker should be writable");

    let manifest_path = session_paths.manifest_path.clone();
    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.captures[0].raw.asset_path = forced_raw_path.to_string_lossy().into_owned();
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let error = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect_err("invalid render output should not be accepted");

    assert_eq!(error.code, "session-persistence-failed");
    assert!(!SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture_result.capture.capture_id))
        .is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_saved_keeps_a_fast_same_capture_thumbnail_visible_while_preview_render_is_still_pending()
{
    let base_dir = unique_test_root("capture-saved-fast-preview");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
        let raw_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        let preview_path = session_paths
            .renders_previews_dir
            .join(format!("{capture_id}.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::create_dir_all(
            preview_path
                .parent()
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&preview_path);

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
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    let preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(result.capture.preview.ready_at_ms, None);
    assert_eq!(
        result.capture.preview.asset_path.as_deref(),
        Some(preview_path.to_string_lossy().as_ref()),
    );
    assert_valid_jpeg(preview_path.to_string_lossy().as_ref());

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    assert_eq!(
        manifest.captures[0].preview.asset_path.as_deref(),
        Some(preview_path.to_string_lossy().as_ref()),
    );
    assert_eq!(manifest.captures[0].preview.ready_at_ms, None);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it() {
    let base_dir = unique_test_root("helper-fast-preview-handoff");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
        let raw_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        let fast_preview_path = session_paths
            .handoff_dir
            .join("fast-preview")
            .join(format!("{capture_id}.camera-thumbnail.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::create_dir_all(
            fast_preview_path
                .parent()
                .expect("fast preview path should have a parent directory"),
        )
        .expect("fast preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&fast_preview_path);

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
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&fast_preview_path),
            Some("camera-thumbnail"),
        );
    });

    let result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(result.capture.preview.ready_at_ms, None);
    assert_eq!(
        result.capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref()),
    );
    assert!(result.capture.timing.fast_preview_visible_at_ms.is_some());
    assert_eq!(result.capture.timing.xmp_preview_ready_at_ms, None);
    assert_valid_jpeg(canonical_preview_path.to_string_lossy().as_ref());

    let initial_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect(
                "preview render should replace the pending fast preview on the same canonical path",
            );

    assert_eq!(
        initial_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref()),
    );
    assert!(initial_capture.timing.fast_preview_visible_at_ms.is_some());

    let preview_ready = if initial_capture.timing.xmp_preview_ready_at_ms.is_some() {
        assert_eq!(initial_capture.render_status, "previewReady");
        assert_eq!(
            initial_capture.preview.ready_at_ms,
            initial_capture.timing.preview_visible_at_ms,
        );
        assert_eq!(
            initial_capture.preview.ready_at_ms,
            initial_capture.timing.xmp_preview_ready_at_ms,
        );
        initial_capture
    } else {
        assert_eq!(initial_capture.render_status, "previewWaiting");
        assert_eq!(initial_capture.preview.ready_at_ms, None);

        let mut ready = None;
        for _ in 0..40 {
            let manifest = read_manifest(&base_dir, &session.session_id);
            let latest_capture = manifest
                .captures
                .iter()
                .find(|capture| capture.capture_id == result.capture.capture_id)
                .cloned()
                .expect("capture should stay in manifest");

            if latest_capture.timing.xmp_preview_ready_at_ms.is_some() {
                ready = Some(latest_capture);
                break;
            }

            thread::sleep(Duration::from_millis(50));
        }

        let refined_capture = ready.expect("raw refinement should eventually finish");
        assert_eq!(refined_capture.render_status, "previewReady");
        assert_eq!(
            refined_capture.preview.asset_path.as_deref(),
            Some(canonical_preview_path.to_string_lossy().as_ref()),
        );
        assert_eq!(
            refined_capture.preview.ready_at_ms,
            refined_capture.timing.xmp_preview_ready_at_ms
        );
        refined_capture
    };

    append_capture_client_timing_event_in_dir(
        &base_dir,
        &CaptureClientDebugLogInputDto {
            label: "recent-session-visible".into(),
            session_id: Some(session.session_id.clone()),
            runtime_mode: Some("tauri".into()),
            customer_state: None,
            reason_code: None,
            can_capture: None,
            message: Some(
                format!(
                    "captureId={};requestId={};previewKind=preset-applied-preview;surface=recent-session;uiLagMs=23;readyAtMs={};latest=true",
                    preview_ready.capture_id,
                    preview_ready.request_id,
                    preview_ready
                        .timing
                        .xmp_preview_ready_at_ms
                        .expect("preview ready timing should be present"),
                ),
            ),
        },
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=request-capture"));
    assert!(timing_events.contains("event=file-arrived"));
    assert!(timing_events.contains("event=fast-preview-promote-start"));
    assert!(timing_events.contains("event=fast-preview-promoted"));
    assert!(timing_events.contains("event=fast-preview-visible"));
    assert!(timing_events.contains("event=preview-render-start"));
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("event=capture_preview_ready"));
    assert!(timing_events.contains("event=recent-session-visible"));
    assert!(timing_events.contains(&format!("request={}", result.capture.request_id)));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn client_recent_session_visibility_events_are_mirrored_into_session_timing_logs() {
    let base_dir = unique_test_root("recent-session-visible-timing-log");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");

    append_capture_client_timing_event_in_dir(
        &base_dir,
        &CaptureClientDebugLogInputDto {
            label: "recent-session-visible".into(),
            session_id: Some(session.session_id.clone()),
            runtime_mode: Some("tauri".into()),
            customer_state: None,
            reason_code: None,
            can_capture: None,
            message: Some(
                "captureId=capture_recent_01;requestId=request_recent_01;previewKind=preset-applied-preview;surface=recent-session;uiLagMs=23;readyAtMs=123;latest=true"
                    .into(),
            ),
        },
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");

    assert!(timing_events.contains("event=recent-session-visible"));
    assert!(timing_events.contains("capture=capture_recent_01"));
    assert!(timing_events.contains("request=request_recent_01"));
    assert!(timing_events.contains("previewKind=preset-applied-preview"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn client_button_pressed_events_are_mirrored_into_session_timing_logs() {
    let base_dir = unique_test_root("button-pressed-timing-log");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");

    append_capture_client_timing_event_in_dir(
        &base_dir,
        &CaptureClientDebugLogInputDto {
            label: "button-pressed".into(),
            session_id: Some(session.session_id.clone()),
            runtime_mode: Some("tauri".into()),
            customer_state: None,
            reason_code: None,
            can_capture: None,
            message: Some("requestId=request_button_01".into()),
        },
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");

    assert!(timing_events.contains("event=button-pressed"));
    assert!(timing_events.contains("request=request_button_01"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn fast_preview_updates_are_emitted_from_the_canonical_preview_path_before_capture_save_closes() {
    let base_dir = unique_test_root("fast-preview-updates");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
        let raw_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        let fast_preview_path = session_paths
            .handoff_dir
            .join("fast-preview")
            .join(format!("{capture_id}.camera-thumbnail.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::create_dir_all(
            fast_preview_path
                .parent()
                .expect("fast preview path should have a parent directory"),
        )
        .expect("fast preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&fast_preview_path);

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
        append_fast_preview_ready_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &fast_preview_path,
            Some("camera-thumbnail"),
        );
        thread::sleep(Duration::from_millis(40));
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&fast_preview_path),
            Some("camera-thumbnail"),
        );
    });

    let mut fast_preview_updates = Vec::new();
    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |update| {
            fast_preview_updates.push(update);
        },
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    assert_eq!(fast_preview_updates.len(), 1);
    assert_eq!(
        fast_preview_updates[0].request_id,
        result.capture.request_id
    );
    assert_eq!(
        fast_preview_updates[0].capture_id,
        result.capture.capture_id
    );
    assert_eq!(
        fast_preview_updates[0].kind.as_deref(),
        Some("camera-thumbnail")
    );
    assert_eq!(
        fast_preview_updates[0].asset_path,
        canonical_preview_path.to_string_lossy(),
        "fast preview update should point at the canonical preview path"
    );
    assert!(
        fast_preview_updates[0].visible_at_ms <= result.capture.raw.persisted_at_ms,
        "fast preview should surface before raw persistence closes the request"
    );
    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(result.readiness.customer_state, "Preview Waiting");
    assert_eq!(
        result.capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_render_can_finish_from_fast_preview_before_raw_handoff_closes() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("speculative-preview-render");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
        let raw_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        let fast_preview_path = session_paths
            .handoff_dir
            .join("fast-preview")
            .join(format!("{capture_id}.camera-thumbnail.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::create_dir_all(
            fast_preview_path
                .parent()
                .expect("fast preview path should have a parent directory"),
        )
        .expect("fast preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&fast_preview_path);

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
        append_fast_preview_ready_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &fast_preview_path,
            Some("camera-thumbnail"),
        );
        thread::sleep(Duration::from_millis(200));
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&fast_preview_path),
            Some("camera-thumbnail"),
        );
    });

    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |_| {},
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    let initial_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("speculative preview render should promote after capture save");

    let preview_ready = if initial_capture.timing.xmp_preview_ready_at_ms.is_some() {
        assert_eq!(initial_capture.render_status, "previewReady");
        assert_eq!(
            initial_capture.preview.ready_at_ms,
            initial_capture.timing.preview_visible_at_ms
        );
        initial_capture.clone()
    } else {
        assert_eq!(initial_capture.render_status, "previewWaiting");
        assert_eq!(initial_capture.preview.ready_at_ms, None);

        let manifest_after_refinement = {
            let mut refined_manifest = None;
            for _ in 0..40 {
                let manifest = read_manifest(&base_dir, &session.session_id);
                let latest_capture = manifest
                    .captures
                    .iter()
                    .find(|capture| capture.capture_id == result.capture.capture_id)
                    .cloned()
                    .expect("capture should stay in manifest");

                if latest_capture.timing.xmp_preview_ready_at_ms.is_some() {
                    refined_manifest = Some(manifest);
                    break;
                }

                thread::sleep(Duration::from_millis(50));
            }

            refined_manifest.expect("raw refinement should eventually finish")
        };
        let refined_capture = manifest_after_refinement
            .captures
            .iter()
            .find(|capture| capture.capture_id == result.capture.capture_id)
            .expect("refined capture should exist");
        assert_eq!(refined_capture.render_status, "previewReady");
        assert!(
            refined_capture
                .timing
                .preview_visible_at_ms
                .expect("refined preview visibility should exist")
                >= initial_capture
                    .timing
                    .fast_preview_visible_at_ms
                    .expect("first-visible timing should exist")
        );
        assert!(
            refined_capture
                .timing
                .xmp_preview_ready_at_ms
                .expect("refinement timestamp should exist")
                >= initial_capture
                    .timing
                    .fast_preview_visible_at_ms
                    .expect("first-visible timing should exist")
        );
        assert!(
            refined_capture
                .preview
                .ready_at_ms
                .expect("refined preview timestamp should exist")
                >= initial_capture
                    .timing
                    .fast_preview_visible_at_ms
                    .expect("first-visible timing should exist")
        );
        refined_capture.clone()
    };

    if initial_capture.timing.xmp_preview_ready_at_ms.is_some() {
        assert_eq!(
            preview_ready.preview.ready_at_ms,
            preview_ready.timing.xmp_preview_ready_at_ms
        );
    }

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=preview-render-start"));
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("sourceAsset=fast-preview-raster"));
    assert!(timing_events.contains(&format!("request={}", result.capture.request_id)));
    assert!(
        !timing_events.contains("reason=first-capture-cold-start"),
        "the first capture should not stay on the cold-start skip path once a same-capture preview is already available"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn invalid_fast_preview_ready_events_are_discarded_without_breaking_capture_success() {
    let base_dir = unique_test_root("invalid-fast-preview-ready-event");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
        let raw_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        let invalid_fast_preview_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.camera-thumbnail.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&invalid_fast_preview_path);

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
        append_fast_preview_ready_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &invalid_fast_preview_path,
            Some("camera-thumbnail"),
        );
        thread::sleep(Duration::from_millis(40));
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            None,
            None,
        );
    });

    let mut fast_preview_updates = Vec::new();
    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |update| {
            fast_preview_updates.push(update);
        },
    )
    .expect("invalid fast-preview-ready should not fail the RAW capture round trip");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    assert!(fast_preview_updates.is_empty());
    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(result.capture.preview.asset_path, None);
    assert_eq!(result.capture.preview.ready_at_ms, None);
    assert!(std::path::Path::new(&result.capture.raw.asset_path).is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_promotes_a_late_canonical_fast_preview_without_marking_preview_ready() {
    let base_dir = unique_test_root("late-canonical-fast-preview");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));
    fs::create_dir_all(
        canonical_preview_path
            .parent()
            .expect("preview path should have a parent directory"),
    )
    .expect("preview directory should exist");
    write_test_jpeg(&canonical_preview_path);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should absorb the late canonical preview");

    let latest_capture = readiness
        .latest_capture
        .expect("late canonical preview should attach to the latest capture");
    assert_eq!(readiness.customer_state, "Ready");
    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);
    assert_eq!(latest_capture.render_status, "previewWaiting");
    assert_eq!(latest_capture.preview.ready_at_ms, None);
    assert_eq!(
        latest_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert!(
        latest_capture.timing.fast_preview_visible_at_ms.is_some(),
        "late canonical preview should populate fast-preview timing"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_reuses_a_late_first_capture_fast_preview_for_the_truthful_close() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("late-canonical-fast-preview-resident");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));
    fs::create_dir_all(
        canonical_preview_path
            .parent()
            .expect("preview path should have a parent directory"),
    )
    .expect("preview directory should exist");
    write_test_jpeg(&canonical_preview_path);

    let initial_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("late same-capture preview should still close truthfully on the first capture");

    assert!(initial_capture.timing.fast_preview_visible_at_ms.is_some());
    assert_eq!(
        initial_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(initial_capture.render_status, "previewReady");
    assert_eq!(
        initial_capture.preview.ready_at_ms,
        initial_capture.timing.xmp_preview_ready_at_ms
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=fast-preview-promoted"));
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("sourceAsset=fast-preview-raster"));
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        1,
        "the first capture should keep a single truthful close even when the fast preview arrives late"
    );
    assert!(
        !timing_events.contains("event=preview-render-queue-saturated"),
        "the first capture should not hit preview queue saturation while adopting the late same-capture close"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn deleting_the_first_capture_does_not_reclassify_a_later_capture_as_cold_start() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("late-fast-preview-after-first-delete");
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

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first capture should close truthfully");
    delete_capture_in_dir(
        &base_dir,
        CaptureDeleteInputDto {
            session_id: session.session_id.clone(),
            capture_id: first_capture.capture.capture_id.clone(),
        },
    )
    .expect("the first capture should be deletable after closing");

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", second_capture.capture.capture_id));
    fs::create_dir_all(
        canonical_preview_path
            .parent()
            .expect("preview path should have a parent directory"),
    )
    .expect("preview directory should exist");
    write_test_jpeg(&canonical_preview_path);

    let refined_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("later capture should still use the warmed path after the first capture was deleted");

    assert_eq!(refined_capture.render_status, "previewReady");
    assert!(refined_capture.timing.fast_preview_visible_at_ms.is_some());

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert_eq!(
        timing_events.matches("reason=first-capture-cold-start").count(),
        1,
        "deleting the original first capture should not make later captures look like cold-start work"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_treats_a_finished_speculative_preview_as_preview_ready() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("finished-speculative-preview-ready");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let canonical_preview_path = paths
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));
    let speculative_output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));

    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    write_test_jpeg(&speculative_output_path);
    fs::write(
        &speculative_detail_path,
        "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fake-darktable-cli;source=test;elapsedMs=120;detail=widthCap=384;heightCap=384;hq=false;sourceAsset=fast-preview-raster;args=fake;status=0",
    )
    .expect("speculative render detail should be writable");

    let initial_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("finished speculative output should close the preview immediately");

    assert_eq!(initial_capture.render_status, "previewReady");
    assert_eq!(
        initial_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        initial_capture.preview.ready_at_ms,
        initial_capture.timing.preview_visible_at_ms
    );
    assert_eq!(
        initial_capture.preview.ready_at_ms,
        initial_capture.timing.xmp_preview_ready_at_ms
    );
    assert!(
        initial_capture.timing.fast_preview_visible_at_ms.is_some(),
        "speculative close should preserve first-visible timing"
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("sourceAsset=fast-preview-raster"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn speculative_preview_render_stays_on_darktable_even_when_truthful_close_canary_matches() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("speculative-preview-stays-darktable");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "accept");
    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path =
        seed_pending_canonical_preview(&base_dir, &session.session_id, &capture.capture.capture_id);
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_output_path = session_paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        capture.capture.capture_id
    ));
    let prepared = render_preview_asset_to_path_in_dir(
        &base_dir,
        &session.session_id,
        &capture.capture.request_id,
        &capture.capture.capture_id,
        "preset_soft-glow",
        "2026.03.20",
        &canonical_preview_path,
        &speculative_output_path,
    )
    .expect("speculative preview should stay on the approved darktable baseline");

    assert!(speculative_output_path.exists());
    assert!(
        prepared.detail.contains("selectedRoute=darktable"),
        "speculative preview should not inherit the truthful close canary route"
    );
    assert!(
        prepared
            .detail
            .contains("selectedPolicyReason=speculative-baseline"),
        "speculative preview should explain why it stayed on darktable"
    );
    assert!(
        prepared.detail.contains("closeOwnerRoute=darktable"),
        "speculative preview should keep the approved baseline as its close owner"
    );
    assert!(
        !prepared
            .detail
            .contains("routeFallbackReasonCode=local-renderer-sidecar-error"),
        "speculative preview should not invoke the local renderer and then fall back"
    );
    let local_renderer_diagnostics_dir = session_paths.diagnostics_dir.join("local-renderer");
    assert!(
        fs::read_dir(&local_renderer_diagnostics_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true),
        "speculative preview should not create local renderer request/response diagnostics"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_a_finished_speculative_preview_pending_until_explicit_retry() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("readiness-promotes-finished-speculative-preview");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let canonical_preview_path =
        seed_pending_canonical_preview(&base_dir, &session.session_id, &result.capture.capture_id);
    let speculative_output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));

    write_test_jpeg(&speculative_output_path);
    fs::write(
        &speculative_detail_path,
        "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fake-darktable-cli;source=test;elapsedMs=2200;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=fake;status=0",
    )
    .expect("speculative render detail should be writable");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should not promote a finished speculative close by itself");

    let latest_capture = readiness
        .latest_capture
        .expect("latest capture should stay attached");
    assert_eq!(readiness.reason_code, "ready");
    assert_eq!(latest_capture.render_status, "previewWaiting");
    assert_eq!(
        latest_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(latest_capture.preview.ready_at_ms, None);

    let timing_events =
        fs::read_to_string(paths.diagnostics_dir.join("timing-events.log")).unwrap_or_default();
    assert!(!timing_events.contains("event=preview-render-ready"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_waits_for_a_healthy_speculative_close_before_raw_fallback() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("wait-for-speculative-close");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        result.capture.capture_id, result.capture.request_id
    ));
    let output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));
    let canonical_preview_path = paths
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    fs::write(&result.capture.raw.asset_path, b"force-process-fail")
        .expect("raw fallback marker should be writable");
    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    fs::write(&lock_path, &result.capture.request_id).expect("speculative lock should be writable");

    let delayed_output_path = output_path.clone();
    let delayed_detail_path = detail_path.clone();
    let delayed_lock_path = lock_path.clone();
    let delayed_writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(1500));
        write_test_jpeg(&delayed_output_path);
        fs::write(
            &delayed_detail_path,
            "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fake-darktable-cli;source=test;elapsedMs=1500;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=fake;status=0",
        )
        .expect("speculative detail should be writable");
        fs::remove_file(&delayed_lock_path).expect("speculative lock should be removable");
    });

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("healthy speculative close should win before raw fallback");

    delayed_writer
        .join()
        .expect("delayed speculative writer should complete");

    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        completed_capture.preview.ready_at_ms,
        completed_capture.timing.xmp_preview_ready_at_ms
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("sourceAsset=fast-preview-raster"));
    assert!(
        !timing_events.contains("sourceAsset=raw-original"),
        "raw fallback should not win when speculative close arrives inside the healthy wait window"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_does_not_start_a_duplicate_render_while_speculative_close_is_active() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("no-duplicate-render-while-speculative-active");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        result.capture.capture_id, result.capture.request_id
    ));
    let output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));
    let canonical_preview_path = paths
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    write_test_jpeg(&canonical_preview_path);
    fs::write(&lock_path, &result.capture.request_id).expect("speculative lock should be writable");

    let delayed_output_path = output_path.clone();
    let delayed_detail_path = detail_path.clone();
    let delayed_lock_path = lock_path.clone();
    let delayed_writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(4300));
        write_test_jpeg(&delayed_output_path);
        fs::write(
            &delayed_detail_path,
            "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fake-darktable-cli;source=test;elapsedMs=4300;detail=widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster;args=fake;status=0",
        )
        .expect("speculative detail should be writable");
        fs::remove_file(&delayed_lock_path).expect("speculative lock should be removable");
    });

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("in-flight speculative close should finish before a duplicate render starts");

    delayed_writer
        .join()
        .expect("delayed speculative writer should complete");

    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        completed_capture.preview.ready_at_ms,
        completed_capture.timing.xmp_preview_ready_at_ms
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        0,
        "the booth should keep waiting for the externally active speculative close instead of starting its own duplicate preview render"
    );
    assert!(timing_events.contains("sourceAsset=fast-preview-raster"));
    assert!(
        !timing_events.contains("sourceAsset=raw-original"),
        "duplicate raw fallback should stay out of the way while the same capture close is still active"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn speculative_preview_wait_does_not_hold_the_capture_pipeline_lock() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("speculative-preview-wait-nonblocking");
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

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        first_capture.capture.capture_id, first_capture.capture.request_id
    ));
    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    fs::write(&speculative_lock_path, &first_capture.capture.request_id)
        .expect("speculative lock should be writable");

    let preview_base_dir = base_dir.clone();
    let preview_session_id = session.session_id.clone();
    let preview_capture_id = first_capture.capture.capture_id.clone();
    let preview_thread = thread::spawn(move || {
        complete_preview_render_in_dir(&preview_base_dir, &preview_session_id, &preview_capture_id)
    });

    thread::sleep(Duration::from_millis(80));

    let lock_contender_started = Instant::now();
    mark_preview_render_failed_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("lock contender should complete even while speculative wait is pending");
    let lock_contender_elapsed = lock_contender_started.elapsed();

    assert!(
        lock_contender_elapsed < Duration::from_millis(800),
        "speculative preview wait should not hold the capture pipeline lock: {lock_contender_elapsed:?}"
    );

    fs::remove_file(&speculative_lock_path).expect("speculative lock should be removable");

    preview_thread
        .join()
        .expect("preview completion thread should join")
        .expect("preview completion should fall back to the normal render path");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_allows_next_capture_once_same_capture_fast_preview_is_visible() {
    let base_dir = unique_test_root("late-canonical-fast-preview-capture-ready");
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));
    fs::create_dir_all(
        canonical_preview_path
            .parent()
            .expect("preview path should have a parent directory"),
    )
    .expect("preview directory should exist");
    write_test_jpeg(&canonical_preview_path);
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should allow capture once same-capture fast preview is visible");

    let latest_capture = readiness
        .latest_capture
        .expect("latest capture should stay attached");
    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);
    assert_eq!(latest_capture.render_status, "previewWaiting");
    assert_eq!(latest_capture.preview.ready_at_ms, None);
    assert_eq!(
        latest_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert!(
        latest_capture.timing.fast_preview_visible_at_ms.is_some(),
        "fast-preview timing should remain populated"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn client_supplied_request_id_is_preserved_for_the_helper_capture_round_trip() {
    let base_dir = unique_test_root("client-request-id");
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

    let client_request_id = "request_client_supplied";
    let capture_result = request_capture_with_helper_success_for_request_id(
        &base_dir,
        &session.session_id,
        Some(client_request_id),
    );
    let requests = read_capture_request_messages(&base_dir, &session.session_id)
        .expect("capture request log should be readable");

    assert_eq!(capture_result.capture.request_id, client_request_id);
    assert_eq!(
        requests.last().map(|request| request.request_id.as_str()),
        Some(client_request_id),
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn duplicate_client_supplied_request_id_is_rejected_before_helper_consumption() {
    let base_dir = unique_test_root("duplicate-client-request-id");
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

    let duplicate_request_id = "request_client_supplied";
    write_capture_request_message(
        &base_dir,
        &CanonHelperCaptureRequestMessage {
            schema_version: CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION.into(),
            message_type: "request-capture".into(),
            session_id: session.session_id.clone(),
            request_id: duplicate_request_id.into(),
            requested_at: current_timestamp(SystemTime::now())
                .expect("request timestamp should serialize"),
            active_preset_id: "preset_soft-glow".into(),
            active_preset_version: "2026.03.20".into(),
        },
    )
    .expect("prior request should be writable");

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: Some(duplicate_request_id.into()),
        },
    )
    .expect_err("duplicate request ids should be rejected before helper consumption");

    let requests = read_capture_request_messages(&base_dir, &session.session_id)
        .expect("capture request log should be readable");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].request_id, duplicate_request_id);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn fast_thumbnail_attempted_and_failed_events_do_not_break_capture_success() {
    let base_dir = unique_test_root("fast-thumbnail-attempted-failed");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
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
        append_fast_thumbnail_attempted_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            Some("camera-thumbnail"),
        );
        append_fast_thumbnail_failed_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            "fast-thumbnail-download-failed",
            Some("camera-thumbnail"),
        );
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            None,
            None,
        );
    });

    let mut fast_preview_updates = Vec::new();
    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |update| {
            fast_preview_updates.push(update);
        },
    )
    .expect("capture should save after diagnostic-only thumbnail events");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    assert!(fast_preview_updates.is_empty());
    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.capture.render_status, "previewWaiting");
    assert!(std::path::Path::new(&result.capture.raw.asset_path).is_file());

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=capture-accepted"));
    assert!(timing_events.contains("event=fast-thumbnail-attempted"));
    assert!(timing_events.contains("event=fast-thumbnail-failed"));
    assert!(timing_events.contains("fastPreviewKind=camera-thumbnail"));
    assert!(timing_events.contains("detailCode=fast-thumbnail-download-failed"));
    assert!(timing_events.contains(&format!("request={}", result.capture.request_id)));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn foreign_session_thumbnail_telemetry_is_ignored_without_breaking_capture_success() {
    let base_dir = unique_test_root("foreign-session-thumbnail-telemetry");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
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
              "schemaVersion": CANON_HELPER_FAST_THUMBNAIL_ATTEMPTED_SCHEMA_VERSION,
              "type": "fast-thumbnail-attempted",
              "sessionId": "session_foreign",
              "requestId": request.request_id,
              "captureId": capture_id,
              "observedAt": current_timestamp(SystemTime::now()).expect("timestamp should serialize"),
              "fastPreviewKind": "camera-thumbnail",
            }),
        );
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_FAST_THUMBNAIL_FAILED_SCHEMA_VERSION,
              "type": "fast-thumbnail-failed",
              "sessionId": "session_foreign",
              "requestId": request.request_id,
              "captureId": capture_id,
              "observedAt": current_timestamp(SystemTime::now()).expect("timestamp should serialize"),
              "detailCode": "fast-thumbnail-download-failed",
              "fastPreviewKind": "camera-thumbnail",
            }),
        );
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            None,
            None,
        );
    });

    let result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect("foreign-session thumbnail telemetry should be ignored");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.capture.render_status, "previewWaiting");
    assert!(std::path::Path::new(&result.capture.raw.asset_path).is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn invalid_fast_preview_handoffs_are_discarded_without_breaking_capture_success() {
    for scenario in [
        "wrong-session",
        "wrong-capture",
        "wrong-directory",
        "stale",
        "corrupted",
    ] {
        let base_dir = unique_test_root(&format!("invalid-fast-preview-{scenario}"));
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
        let existing_request_count = capture_request_count(&base_dir, &session.session_id);

        let helper_base_dir = base_dir.clone();
        let helper_session_id = session.session_id.clone();
        let helper_scenario = scenario.to_string();
        let helper_thread = thread::spawn(move || {
            let request = wait_for_latest_capture_request(
                &helper_base_dir,
                &helper_session_id,
                existing_request_count,
            );
            let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
            let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
            let raw_path = session_paths
                .captures_originals_dir
                .join(format!("{capture_id}.jpg"));

            fs::create_dir_all(
                raw_path
                    .parent()
                    .expect("raw capture path should have a parent directory"),
            )
            .expect("raw capture directory should exist");

            let fast_preview_path = match helper_scenario.as_str() {
                "wrong-session" => {
                    let foreign_paths =
                        SessionPaths::new(&helper_base_dir, "session_foreign_preview");
                    let path = foreign_paths
                        .handoff_dir
                        .join("fast-preview")
                        .join(format!("{capture_id}.jpg"));
                    fs::create_dir_all(
                        path.parent()
                            .expect("foreign fast preview should have a parent directory"),
                    )
                    .expect("foreign fast preview directory should exist");
                    write_test_jpeg(&path);
                    path
                }
                "wrong-capture" => {
                    let path = session_paths
                        .handoff_dir
                        .join("fast-preview")
                        .join("capture_someone_else.fast-preview.jpg");
                    fs::create_dir_all(
                        path.parent()
                            .expect("wrong-capture preview should have a parent directory"),
                    )
                    .expect("wrong-capture preview directory should exist");
                    write_test_jpeg(&path);
                    path
                }
                "wrong-directory" => {
                    let path = session_paths
                        .captures_originals_dir
                        .join(format!("{capture_id}.camera-thumbnail.jpg"));
                    fs::create_dir_all(
                        path.parent()
                            .expect("wrong-directory preview should have a parent directory"),
                    )
                    .expect("wrong-directory preview directory should exist");
                    write_test_jpeg(&path);
                    path
                }
                "stale" => {
                    let path = session_paths
                        .handoff_dir
                        .join("fast-preview")
                        .join(format!("{capture_id}.stale.jpg"));
                    fs::create_dir_all(
                        path.parent()
                            .expect("stale preview should have a parent directory"),
                    )
                    .expect("stale preview directory should exist");
                    write_test_jpeg(&path);
                    thread::sleep(Duration::from_millis(30));
                    path
                }
                "corrupted" => {
                    let path = session_paths
                        .handoff_dir
                        .join("fast-preview")
                        .join(format!("{capture_id}.broken.jpg"));
                    fs::create_dir_all(
                        path.parent()
                            .expect("corrupted preview should have a parent directory"),
                    )
                    .expect("corrupted preview directory should exist");
                    fs::write(&path, b"not-a-jpeg").expect("corrupted preview should be writable");
                    path
                }
                _ => unreachable!("unsupported invalid fast preview scenario"),
            };

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
            append_file_arrived_event(
                &helper_base_dir,
                &helper_session_id,
                &request,
                &capture_id,
                &raw_path,
                Some(&fast_preview_path),
                Some("camera-thumbnail"),
            );
        });

        let result = request_capture_in_dir(
            &base_dir,
            CaptureRequestInputDto {
                session_id: session.session_id.clone(),
                request_id: None,
            },
        )
        .expect("capture success should stay intact even when the fast preview handoff is invalid");

        helper_thread
            .join()
            .expect("helper capture thread should complete");

        assert_eq!(
            result.capture.render_status, "previewWaiting",
            "scenario={scenario}"
        );
        assert_eq!(
            result.capture.preview.asset_path, None,
            "scenario={scenario}"
        );
        assert_eq!(
            result.capture.preview.ready_at_ms, None,
            "scenario={scenario}"
        );
        assert_eq!(
            result.capture.timing.fast_preview_visible_at_ms, None,
            "scenario={scenario}"
        );

        let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
            .renders_previews_dir
            .join(format!("{}.jpg", result.capture.capture_id));
        assert!(
            !canonical_preview_path.is_file(),
            "invalid fast preview should not create the canonical preview asset: scenario={scenario}"
        );

        let timing_events = fs::read_to_string(
            SessionPaths::new(&base_dir, &session.session_id)
                .diagnostics_dir
                .join("timing-events.log"),
        )
        .expect("timing events should be readable");
        assert!(
            timing_events.contains("event=fast-preview-invalid"),
            "invalid handoff should be logged: scenario={scenario}"
        );

        let _ = fs::remove_dir_all(base_dir);
    }
}

#[test]
fn canonical_same_capture_preview_is_still_seeded_when_fast_preview_handoff_is_invalid() {
    let base_dir = unique_test_root("invalid-fast-preview-seeds-canonical-preview");
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
    let existing_request_count = capture_request_count(&base_dir, &session.session_id);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(
            &helper_base_dir,
            &helper_session_id,
            existing_request_count,
        );
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let session_paths = SessionPaths::new(&helper_base_dir, &helper_session_id);
        let raw_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));
        let invalid_fast_preview_path = session_paths
            .captures_originals_dir
            .join(format!("{capture_id}.camera-thumbnail.jpg"));
        let canonical_preview_path = session_paths
            .renders_previews_dir
            .join(format!("{capture_id}.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::create_dir_all(
            canonical_preview_path
                .parent()
                .expect("canonical preview path should have a parent directory"),
        )
        .expect("canonical preview directory should exist");

        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&invalid_fast_preview_path);
        write_test_jpeg(&canonical_preview_path);

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
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&invalid_fast_preview_path),
            Some("camera-thumbnail"),
        );
    });

    let result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect("capture should keep the canonical preview fast path alive");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(result.capture.preview.ready_at_ms, None);
    assert_eq!(
        result.capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref()),
    );
    assert!(
        result.capture.timing.fast_preview_visible_at_ms.is_some(),
        "canonical preview scan should still record first-visible timing"
    );
    assert_valid_jpeg(canonical_preview_path.to_string_lossy().as_ref());

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=fast-preview-invalid"));
    assert!(timing_events.contains("event=fast-preview-promoted"));
    assert!(timing_events.contains("kind=legacy-canonical-scan"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_does_not_rerender_a_legacy_text_jpg_preview_by_itself() {
    let base_dir = unique_test_root("capture-preview-repair-from-invalid-jpg");
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
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let preview_path = session_paths
        .renders_previews_dir
        .join(format!("{}.jpg", capture_result.capture.capture_id));
    fs::create_dir_all(&session_paths.renders_previews_dir)
        .expect("preview directory should exist");
    fs::write(&preview_path, b"not-a-real-jpeg").expect("legacy invalid preview should exist");

    let manifest_path = session_paths.manifest_path.clone();
    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.captures[0].preview.asset_path = Some(preview_path.to_string_lossy().into_owned());
    manifest.captures[0].preview.ready_at_ms = Some(1234);
    manifest.captures[0].render_status = "previewReady".into();
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
    .expect("readiness lookup should not rerender an invalid preview");

    let repaired_preview_path = readiness
        .latest_capture
        .as_ref()
        .and_then(|capture| capture.preview.asset_path.as_deref())
        .expect("preview path should stay attached");
    assert_eq!(
        repaired_preview_path,
        preview_path.to_string_lossy().as_ref()
    );
    assert_eq!(
        fs::read(repaired_preview_path).expect("preview bytes should stay readable"),
        b"not-a-real-jpeg"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_the_canonical_preview_path_even_when_newer_suffix_previews_exist() {
    let base_dir = unique_test_root("capture-preview-keep-canonical-path");
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
    let ready_capture = complete_preview_render_in_dir(&base_dir, &session.session_id, &capture_id)
        .expect("preview render should complete");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let canonical_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{capture_id}.jpg"));
    let suffixed_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{capture_id}_2.jpg"));

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );

    thread::sleep(Duration::from_millis(30));
    write_test_jpeg(&suffixed_preview_path);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should preserve the canonical preview path");

    assert_eq!(
        readiness
            .latest_capture
            .as_ref()
            .and_then(|capture| capture.preview.asset_path.as_deref()),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );

    let manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&session_paths.manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    assert_eq!(
        manifest.captures[0].preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_legacy_published_bundle_without_render_profiles_still_prepares_preview() {
    let base_dir = unique_test_root("capture-legacy-published-bundle");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let bundle_dir = resolve_published_preset_catalog_dir(&base_dir)
        .join("preset_test-look")
        .join("2026.03.31");

    fs::create_dir_all(bundle_dir.join("xmp")).expect("xmp directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should exist");
    fs::write(
        bundle_dir.join("xmp").join("test-look.xmp"),
        b"<?xml version=\"1.0\"?><xmp/>",
    )
    .expect("xmp template should exist");
    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "published-preset-bundle/v1",
          "presetId": "preset_test-look",
          "displayName": "Test Look",
          "publishedVersion": "2026.03.31",
          "lifecycleStatus": "published",
          "boothStatus": "booth-safe",
          "darktableVersion": "5.4.1",
          "darktableProjectPath": "darktable/test-look.dtpreset",
          "xmpTemplatePath": "xmp/test-look.xmp",
          "preview": {
            "kind": "preview-tile",
            "assetPath": "preview.jpg",
            "altText": "Test Look preview"
          }
        }))
        .expect("legacy bundle should serialize"),
    )
    .expect("legacy bundle should be writable");

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_test-look".into(),
            published_version: "2026.03.31".into(),
        },
    )
    .expect("legacy preset should become active");
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let preview_ready =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
            .expect("legacy published bundle should still render preview");

    assert_eq!(preview_ready.render_status, "previewReady");
    assert!(preview_ready.preview.asset_path.is_some());
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("legacy published bundle readiness should resolve");

    assert_eq!(readiness.surface_state, "previewReady");
    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_finishes_without_waiting_for_a_delayed_sidecar_placeholder() {
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
        .expect("preview render should not depend on a delayed sidecar placeholder");

    assert_valid_jpeg(sidecar_preview_path.to_string_lossy().as_ref());

    preview_writer
        .join()
        .expect("delayed preview writer should complete");

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(sidecar_preview_path.to_string_lossy().as_ref()),
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
            request_id: None,
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
            request_id: None,
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
fn capture_flow_keeps_session_retryable_when_focus_is_not_locked() {
    let base_dir = unique_test_root("capture-focus-not-locked-retryable");
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
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
              "type": "helper-error",
              "sessionId": request.session_id,
              "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
              "detailCode": "capture-focus-not-locked",
              "message": "카메라가 초점을 아직 잡지 못했어요.",
            }),
        );
        write_ready_helper_status(&helper_base_dir, &helper_session_id);
    });

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect_err("focus-not-locked should return retryable readiness");

    helper_thread
        .join()
        .expect("helper retryable focus thread should complete");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .expect("retryable focus failure should include readiness")
            .reason_code,
        "capture-retry-required",
    );
    let manifest = read_manifest(&base_dir, &session.session_id);
    assert!(manifest.captures.is_empty());
    assert_eq!(manifest.lifecycle.stage, "preset-selected");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("follow-up readiness should resolve");
    assert_eq!(readiness.reason_code, "ready");
    assert_eq!(readiness.customer_state, "Ready");

    let retry_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    assert_eq!(retry_capture.status, "capture-saved");
    assert_eq!(retry_capture.readiness.reason_code, "preview-waiting");
    assert_eq!(
        read_manifest(&base_dir, &session.session_id).captures.len(),
        1
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_after_retryable_focus_failure_until_operator_retry() {
    let base_dir = unique_test_root("retryable-focus-failure-unlocks");
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
    update_stage(&base_dir, &session.session_id, "phone-required");
    append_helper_event(
        &base_dir,
        &session.session_id,
        serde_json::json!({
          "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
          "type": "helper-error",
          "sessionId": session.session_id,
          "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
          "detailCode": "capture-trigger-failed",
          "message": "셔터 명령을 보낼 수 없었어요: 0x00008d01",
        }),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("retryable focus failure should remain blocked until an explicit retry");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_after_capture_download_timeout_until_explicit_recovery() {
    let base_dir = unique_test_root("capture-download-timeout-unlocks");
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
    update_stage(&base_dir, &session.session_id, "phone-required");
    append_helper_event(
        &base_dir,
        &session.session_id,
        serde_json::json!({
          "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
          "type": "helper-error",
          "sessionId": session.session_id,
          "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
          "detailCode": "capture-download-timeout",
          "message": "RAW handoff를 기다리다 시간이 초과되었어요.",
        }),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness lookup should not auto-recover a capture download timeout");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_after_capture_transfer_start_timeout_until_explicit_recovery() {
    let base_dir = unique_test_root("capture-transfer-start-timeout-unlocks");
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
    update_stage(&base_dir, &session.session_id, "phone-required");
    append_helper_event(
        &base_dir,
        &session.session_id,
        serde_json::json!({
          "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
          "type": "helper-error",
          "sessionId": session.session_id,
          "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
          "detailCode": "capture-transfer-start-timeout",
          "message": "촬영은 수락됐지만 RAW transfer 시작 신호가 오지 않았어요.",
        }),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness lookup should not auto-recover a transfer-start timeout");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_when_retryable_helper_error_is_older_than_the_blocked_stage() {
    let base_dir = unique_test_root("capture-trigger-stale-retry-does-not-unlock");
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
    append_helper_event(
        &base_dir,
        &session.session_id,
        serde_json::json!({
          "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
          "type": "helper-error",
          "sessionId": session.session_id,
          "observedAt": "2026-03-20T00:00:00Z",
          "detailCode": "capture-trigger-failed",
          "message": "이전 세션의 초점 실패 이벤트예요: 0x00008d01",
        }),
    );
    update_stage(&base_dir, &session.session_id, "phone-required");
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("stale retryable helper error should not clear phone-required");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_retries_manifest_persist_after_a_transient_write_conflict() {
    let base_dir = unique_test_root("capture-persist-retries");
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

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    set_manifest_write_retryable_failures_for_tests(&manifest_path, 2);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);

    set_manifest_write_retryable_failures_for_tests(&manifest_path, 0);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.captures.len(), 1);
    assert_eq!(manifest.lifecycle.stage, "preview-waiting");
    assert_eq!(
        manifest.captures[0].capture_id,
        capture_result.capture.capture_id,
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_escalates_when_manifest_persist_never_recovers() {
    let base_dir = unique_test_root("capture-persist-hard-failure");
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

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    set_manifest_write_retryable_failures_for_tests(&manifest_path, 32);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
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
              "captureId": capture_id,
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    let error = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect_err("capture persist should fail when manifest write never recovers");

    helper_thread
        .join()
        .expect("helper success thread should complete");
    set_manifest_write_retryable_failures_for_tests(&manifest_path, 0);

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .as_ref()
            .expect("persist failure should still expose a blocked readiness")
            .reason_code,
        "phone-required",
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert!(manifest.captures.is_empty());
    assert_eq!(manifest.lifecycle.stage, "preset-selected");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("persist failure should keep later readiness blocked");
    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

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
            request_id: None,
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
            request_id: None,
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
            request_id: None,
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
                request_id: None,
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
            request_id: None,
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
fn preview_render_stays_bound_to_the_capture_version_after_active_preset_changes() {
    let base_dir = unique_test_root("capture-preset-version-drift-guard");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_named_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "Soft Glow V1",
        "2026.03.20",
    );

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("first preset version should become active");
    write_ready_helper_status(&base_dir, &session.session_id);

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);

    create_named_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "Soft Glow V2",
        "2026.03.21",
    );
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id)
        .manifest_path
        .clone();
    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.active_preset = Some(boothy_lib::session::session_manifest::ActivePresetBinding {
        preset_id: "preset_soft-glow".into(),
        published_version: "2026.03.21".into(),
    });
    manifest.active_preset_id = Some("preset_soft-glow".into());
    manifest.active_preset_display_name = Some("Soft Glow V2".into());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let first_ready_capture = complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("preview render should remain capture-bound");

    assert_valid_jpeg(
        first_ready_capture
            .preview
            .asset_path
            .as_deref()
            .expect("preview path should exist"),
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("publishedVersion=2026.03.20"));
    assert!(!timing_events.contains("publishedVersion=2026.03.21"));

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
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let forced_raw_path = session_paths
        .captures_originals_dir
        .join("force-invalid-output.jpg");
    fs::write(&forced_raw_path, b"raw-marker").expect("raw marker should be writable");

    let manifest_path = session_paths.manifest_path.clone();
    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.captures[0].raw.asset_path = forced_raw_path.to_string_lossy().into_owned();
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    mark_preview_render_failed_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect("preview failure boundary should persist");
    write_ready_helper_status(&base_dir, &session.session_id);

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
fn handoff_ready_completion_preserves_variant_with_generic_destination_without_destination_metadata(
) {
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
            "approvedRecipientLabel": "   ",
            "nextLocationLabel": "",
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
    assert_eq!(post_end.completion_variant(), Some("handoff-ready"));
    match post_end {
        SessionPostEnd::Completed(value) => {
            assert_eq!(value.next_location_label.as_deref(), Some("안내된 곳"));
            assert_eq!(value.primary_action_label, "안내를 확인해 주세요.");
        }
        _ => panic!("expected completed post-end"),
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn invalid_existing_handoff_ready_record_is_rebuilt_with_safe_destination_guidance() {
    let base_dir = unique_test_root("handoff-ready-invalid-existing");
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
    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.lifecycle.stage = "completed".into();
    manifest.post_end = Some(SessionPostEnd::Completed(CompletedPostEnd {
        state: "completed".into(),
        evaluated_at: timestamp_offset(-5),
        completion_variant: "handoff-ready".into(),
        approved_recipient_label: None,
        next_location_label: None,
        primary_action_label: "안내를 확인해 주세요.".into(),
        support_action_label: None,
        show_booth_alias: false,
        handoff: None,
    }));
    fs::write(
        SessionPaths::new(&base_dir, &session.session_id).manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("handoff-ready completion should be repaired safely");

    let post_end = readiness.post_end.expect("completed guidance should exist");
    assert_eq!(post_end.completion_variant(), Some("handoff-ready"));
    match post_end {
        SessionPostEnd::Completed(value) => {
            assert_eq!(value.next_location_label.as_deref(), Some("안내된 곳"));
            assert_eq!(value.primary_action_label, "안내된 곳으로 이동해 주세요.");
        }
        _ => panic!("expected completed post-end"),
    }

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
    write_ready_helper_status(&base_dir, &session.session_id);

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
            request_id: None,
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
fn ended_preview_ready_capture_waits_for_final_render_and_promotes_to_handoff_ready() {
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
        Some("handoff-ready")
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "completed");
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.post_end_state.as_str()),
        Some("handoff-ready")
    );
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.render_status.as_str()),
        Some("finalReady")
    );
    assert!(manifest
        .captures
        .last()
        .and_then(|latest_capture| latest_capture.final_asset.asset_path.as_deref())
        .map(std::path::Path::new)
        .map(|path| path.is_file())
        .unwrap_or(false));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_does_not_recover_render_failure_just_because_the_bundle_is_upgraded() {
    let base_dir = unique_test_root("capture-legacy-default-bundle-recovery");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let legacy_bundle_dir = resolve_published_preset_catalog_dir(&base_dir)
        .join("preset_daylight")
        .join("2026.03.27");
    fs::create_dir_all(&legacy_bundle_dir).expect("legacy bundle directory should exist");
    fs::write(legacy_bundle_dir.join("preview.svg"), "<svg/>").expect("preview should exist");
    fs::write(
        legacy_bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "published-preset-bundle/v1",
          "presetId": "preset_daylight",
          "displayName": "Daylight",
          "publishedVersion": "2026.03.27",
          "lifecycleStatus": "published",
          "boothStatus": "booth-safe",
          "preview": {
            "kind": "preview-tile",
            "assetPath": "preview.svg",
            "altText": "Daylight preview"
          }
        }))
        .expect("legacy bundle should serialize"),
    )
    .expect("legacy bundle should be writable");

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_daylight".into(),
            published_version: "2026.03.27".into(),
        },
    )
    .expect("legacy daylight preset should become active");
    write_ready_helper_status(&base_dir, &session.session_id);

    let capture_result = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect_err("legacy bundle should fail preview render before upgrade");
    mark_preview_render_failed_in_dir(
        &base_dir,
        &session.session_id,
        &capture_result.capture.capture_id,
    )
    .expect("render failure should be recorded");

    ensure_default_preset_catalog_in_dir(&base_dir)
        .expect("legacy default bundle should be upgraded");
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness lookup should not auto-recover a render failure");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|capture| capture.render_status.as_str()),
        Some("renderFailed")
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

#[test]
fn complete_preview_render_accepts_a_valid_local_renderer_candidate_and_keeps_the_canonical_slot() {
    let base_dir = unique_test_root("local-renderer-accept");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let canonical_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    assert_eq!(result.capture.timing.xmp_preview_ready_at_ms, None);

    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("valid local renderer candidate should close the preview");

    assert_valid_jpeg(canonical_preview_path.to_string_lossy().as_ref());
    assert_eq!(ready_capture.render_status, "previewReady");
    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert!(ready_capture.preview.ready_at_ms.is_some());
    assert!(ready_capture.timing.xmp_preview_ready_at_ms.is_some());

    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=renderer-route-selected"));
    assert!(timing_events.contains("reason=local-renderer-sidecar"));
    assert!(timing_events.contains("event=renderer-close-owner"));
    assert!(timing_events.contains("detail=route=local-renderer-sidecar"));
    assert!(timing_events.contains("fidelityDetail=deltaE=0.4"));
    assert!(
        !timing_events.contains("event=renderer-route-fallback"),
        "valid canary candidate should not fall back"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn forced_fallback_policy_skips_the_sidecar_and_keeps_darktable_as_close_owner() {
    let base_dir = unique_test_root("local-renderer-forced-fallback");
    write_preset_scoped_preview_render_route_policy(&base_dir, true);
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

    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("forced fallback should keep preview close healthy");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=renderer-route-selected"));
    assert!(timing_events.contains("reason=darktable"));
    assert!(timing_events.contains("policyReason=forced-fallback"));
    assert!(timing_events.contains("fallbackReason=manual-disable"));
    assert!(timing_events.contains("event=renderer-close-owner"));
    assert!(timing_events.contains("detail=route=darktable"));
    assert!(timing_events.contains("fidelityVerdict=approved-baseline"));
    assert!(timing_events.contains("fidelityDetail=engine=darktable-cli,comparison=baseline-owner"));
    assert!(
        !timing_events.contains("event=renderer-route-fallback"),
        "forced fallback should bypass the sidecar instead of attempting and failing"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn branch_scoped_policy_selects_the_local_renderer_without_an_env_override() {
    let base_dir = unique_test_root("local-renderer-branch-scope");
    let _branch_env_guard = scoped_env_vars(vec![("BOOTHY_BRANCH_ID", None)]);
    write_branch_rollout_state(&base_dir, "gangnam-01");
    write_branch_scoped_preview_render_route_policy(&base_dir, "gangnam-01");
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

    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("branch-scoped booth policy should still select the local renderer");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=renderer-route-selected"));
    assert!(timing_events.contains("reason=local-renderer-sidecar"));
    assert!(timing_events.contains("policyReason=canary-match"));
    assert!(timing_events.contains("event=renderer-close-owner"));
    assert!(timing_events.contains("detail=route=local-renderer-sidecar"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn invalid_local_renderer_candidate_falls_back_to_darktable_without_false_ready() {
    let base_dir = unique_test_root("local-renderer-fallback");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "wrong-session");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    assert_eq!(result.capture.timing.xmp_preview_ready_at_ms, None);

    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("invalid canary candidate should fall back to darktable");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let canonical_preview_path = session_paths
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));

    assert_valid_jpeg(canonical_preview_path.to_string_lossy().as_ref());
    assert_eq!(ready_capture.render_status, "previewReady");
    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );

    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=renderer-route-selected"));
    assert!(timing_events.contains("reason=local-renderer-sidecar"));
    assert!(timing_events.contains("event=renderer-route-fallback"));
    assert!(timing_events.contains("reason=local-renderer-session-mismatch"));
    assert!(timing_events.contains("event=renderer-close-owner"));
    assert!(timing_events.contains("detail=route=darktable"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn wrong_capture_and_wrong_preset_candidates_fall_back_without_false_ready() {
    for (mode, reason_code) in [
        ("wrong-capture", "local-renderer-capture-mismatch"),
        (
            "wrong-preset-version",
            "local-renderer-preset-version-mismatch",
        ),
    ] {
        let base_dir = unique_test_root(&format!("local-renderer-{mode}"));
        write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

        write_fake_local_renderer_sidecar(&base_dir, mode);

        let result = request_capture_with_helper_success(&base_dir, &session.session_id);
        let ready_capture = complete_preview_render_in_dir(
            &base_dir,
            &session.session_id,
            &result.capture.capture_id,
        )
        .expect("invalid canary candidate should fall back to darktable");

        let session_paths = SessionPaths::new(&base_dir, &session.session_id);
        let canonical_preview_path = session_paths
            .renders_previews_dir
            .join(format!("{}.jpg", result.capture.capture_id));
        let timing_events =
            fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
                .expect("timing events should be readable");

        assert_valid_jpeg(canonical_preview_path.to_string_lossy().as_ref());
        assert_eq!(ready_capture.render_status, "previewReady");
        assert!(timing_events.contains("event=renderer-route-fallback"));
        assert!(timing_events.contains(&format!("reason={reason_code}")));
        assert!(timing_events.contains("detail=route=darktable"));

        let _ = fs::remove_dir_all(base_dir);
    }
}

#[test]
fn local_renderer_timeout_falls_back_to_darktable() {
    let base_dir = unique_test_root("local-renderer-timeout");
    let _timeout_guard = scoped_env_vars(vec![(
        "BOOTHY_LOCAL_RENDERER_TIMEOUT_MS",
        Some(std::ffi::OsString::from("50")),
    )]);
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "timeout");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("timed out sidecar should fall back to darktable");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=renderer-route-fallback"));
    assert!(timing_events.contains("reason=local-renderer-timeout"));
    assert!(timing_events.contains("detail=route=darktable"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn malformed_stale_and_duplicate_local_renderer_candidates_fall_back_without_false_ready() {
    for (mode, reason_code) in [
        ("malformed", "local-renderer-malformed-response"),
        ("stale", "local-renderer-stale-output"),
        ("duplicate", "local-renderer-duplicate-completion"),
    ] {
        let base_dir = unique_test_root(&format!("local-renderer-{mode}-fallback"));
        write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

        write_fake_local_renderer_sidecar(&base_dir, mode);

        let result = request_capture_with_helper_success(&base_dir, &session.session_id);
        let ready_capture = complete_preview_render_in_dir(
            &base_dir,
            &session.session_id,
            &result.capture.capture_id,
        )
        .expect("invalid canary candidate should fall back to darktable");

        let session_paths = SessionPaths::new(&base_dir, &session.session_id);
        let timing_events =
            fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
                .expect("timing events should be readable");

        assert_eq!(ready_capture.render_status, "previewReady");
        assert!(timing_events.contains("event=renderer-route-fallback"));
        assert!(timing_events.contains(&format!("reason={reason_code}")));
        assert!(timing_events.contains("detail=route=darktable"));

        let _ = fs::remove_dir_all(base_dir);
    }
}

#[test]
fn local_renderer_error_envelope_is_recorded_before_fallback() {
    let base_dir = unique_test_root("local-renderer-error-envelope");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "error-envelope");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("sidecar error envelope should still fall back to darktable");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=renderer-route-fallback"));
    assert!(timing_events.contains("reason=local-renderer-sidecar-error"));
    assert!(timing_events.contains("reasonDetail=local renderer sidecar가 오류 envelope를 반환했어요: darktable bridge failed inside the sidecar"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn noisy_local_renderer_stderr_does_not_force_a_false_timeout() {
    let base_dir = unique_test_root("local-renderer-noisy-stderr");
    let _timeout_guard = scoped_env_vars(vec![(
        "BOOTHY_LOCAL_RENDERER_TIMEOUT_MS",
        Some(std::ffi::OsString::from("1500")),
    )]);
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "noisy-stderr");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("noisy stderr alone should not force a timeout fallback");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=renderer-close-owner"));
    assert!(timing_events.contains("detail=route=local-renderer-sidecar"));
    assert!(
        !timing_events.contains("reason=local-renderer-timeout"),
        "stderr noise alone should not trigger timeout fallback"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn local_renderer_failure_forces_darktable_for_the_rest_of_the_session() {
    let base_dir = unique_test_root("local-renderer-session-health-fallback");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "error-envelope");

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first capture should still recover with darktable fallback");

    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second capture should close without retrying the unhealthy sidecar");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    let first_capture_section = timing_events
        .lines()
        .filter(|line| line.contains(&format!("capture={}", first_capture.capture.capture_id)))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(first_capture_section.contains("event=renderer-route-fallback"));
    assert!(first_capture_section.contains("reason=local-renderer-sidecar-error"));

    let second_capture_section = timing_events
        .lines()
        .filter(|line| line.contains(&format!("capture={}", second_capture.capture.capture_id)))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(second_capture_section.contains("event=renderer-route-selected"));
    assert!(second_capture_section.contains("reason=darktable"));
    assert!(second_capture_section.contains("policyReason=forced-fallback"));
    assert!(second_capture_section.contains("fallbackReason=session-sidecar-health-check-failed"));
    assert!(
        !second_capture_section.contains("event=renderer-route-fallback"),
        "once the session marks the sidecar unhealthy, later captures should bypass it instead of retrying and failing again"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn speculative_preview_baseline_does_not_force_darktable_for_later_truthful_close_canary() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("local-renderer-speculative-session-health");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "error-envelope");

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = seed_pending_canonical_preview(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    );
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_output_path = session_paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        first_capture.capture.capture_id
    ));
    let prepared = render_preview_asset_to_path_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.request_id,
        &first_capture.capture.capture_id,
        "preset_soft-glow",
        "2026.03.20",
        &canonical_preview_path,
        &speculative_output_path,
    )
    .expect("speculative preview should still recover with the direct baseline path");

    assert!(
        prepared.detail.contains("selectedRoute=darktable"),
        "speculative preview should stay on the approved darktable baseline"
    );
    assert!(
        prepared
            .detail
            .contains("selectedPolicyReason=speculative-baseline"),
        "speculative preview should document the bounded-scope baseline decision"
    );
    assert!(
        prepared.detail.contains("closeOwnerRoute=darktable"),
        "speculative fallback should still produce a valid direct-render candidate"
    );
    assert!(
        !prepared
            .detail
            .contains("routeFallbackReasonCode=local-renderer-sidecar-error"),
        "speculative preview should not try the local renderer route anymore"
    );

    let locked_policy_path = session_paths
        .diagnostics_dir
        .join("preview-renderer-policy.lock.json");
    let locked_policy_before_second_capture =
        fs::read_to_string(&locked_policy_path).expect("locked policy should be readable");
    assert!(
        !locked_policy_before_second_capture.contains("session-sidecar-health-check-failed"),
        "a speculative-only sidecar miss should not quarantine the whole session"
    );

    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("later captures should still be allowed to retry the local renderer");

    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    let second_capture_section = timing_events
        .lines()
        .filter(|line| line.contains(&format!("capture={}", second_capture.capture.capture_id)))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(second_capture_section.contains("event=renderer-route-selected"));
    assert!(second_capture_section.contains("reason=local-renderer-sidecar"));
    assert!(
        !second_capture_section.contains("policyReason=forced-fallback"),
        "speculative misses should not silently force later captures back to darktable"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_policy_is_locked_from_session_start_through_the_active_session() {
    let base_dir = unique_test_root("local-renderer-session-policy-lock");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let locked_policy_path = session_paths
        .diagnostics_dir
        .join("preview-renderer-policy.lock.json");

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

    assert!(
        locked_policy_path.is_file(),
        "policy lock should exist before the first capture"
    );
    let locked_policy =
        fs::read_to_string(&locked_policy_path).expect("locked policy should be readable");
    assert!(locked_policy.contains("\"presetId\": \"preset_soft-glow\""));

    write_preset_scoped_preview_render_route_policy(&base_dir, true);
    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let first_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first capture should close with the canary route");

    write_preview_render_route_policy(&base_dir, &session.session_id, true);

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("mid-session policy edits should not flip the selected route");

    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    let first_capture_section = timing_events
        .lines()
        .filter(|line| line.contains(&format!("capture={}", first_capture.capture.capture_id)))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(first_capture_section.contains("event=renderer-route-selected"));
    assert!(first_capture_section.contains("reason=local-renderer-sidecar"));
    assert!(
        !first_capture_section.contains("policyReason=forced-fallback"),
        "session-start lock should ignore policy edits made before the first capture closes"
    );

    let second_capture_section = timing_events
        .lines()
        .filter(|line| line.contains(&format!("capture={}", second_capture.capture.capture_id)))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(second_capture_section.contains("event=renderer-route-selected"));
    assert!(second_capture_section.contains("reason=local-renderer-sidecar"));
    assert!(
        !second_capture_section.contains("policyReason=forced-fallback"),
        "active-session policy lock should ignore later branch policy edits"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn local_renderer_truthful_close_reuses_an_existing_canonical_preview_slot() {
    let base_dir = unique_test_root("local-renderer-same-slot");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "accept");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path =
        seed_pending_canonical_preview(&base_dir, &session.session_id, &result.capture.capture_id);
    let before_bytes = fs::read(&canonical_preview_path).expect("pending preview should exist");

    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("truthful close should replace the pending preview in place");
    let after_bytes = fs::read(&canonical_preview_path).expect("rendered preview should exist");
    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(ready_capture.render_status, "previewReady");
    assert_ne!(
        before_bytes, after_bytes,
        "truthful close should replace the existing slot instead of leaving the pending bytes"
    );
    assert_eq!(
        ready_capture.timing.preset_applied_delta_ms,
        Some(
            ready_capture
                .timing
                .preview_visible_at_ms
                .expect("truthful close should stamp preview visible timing")
                .saturating_sub(
                    ready_capture
                        .timing
                        .fast_preview_visible_at_ms
                        .expect("same-capture first-visible timing should be preserved"),
                ),
        ),
        "same session evidence should keep the truthful-close delta alongside preview timings"
    );
    assert!(
        timing_events.contains("sourceAsset=fast-preview-raster"),
        "accepted local renderer close should preserve the actual fast-preview source in diagnostics"
    );
    assert!(
        timing_events.contains("presetAppliedDeltaMs="),
        "capture preview ready evidence should include the truthful-close delta for canary comparison"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn local_renderer_fallback_reuses_an_existing_canonical_preview_slot() {
    let base_dir = unique_test_root("local-renderer-fallback-same-slot");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
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

    write_fake_local_renderer_sidecar(&base_dir, "error-envelope");

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path =
        seed_pending_canonical_preview(&base_dir, &session.session_id, &result.capture.capture_id);
    let before_bytes = fs::read(&canonical_preview_path).expect("pending preview should exist");

    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("fallback close should still replace the pending preview in place");
    let after_bytes = fs::read(&canonical_preview_path).expect("rendered preview should exist");

    assert_eq!(
        ready_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(ready_capture.render_status, "previewReady");
    assert_ne!(
        before_bytes, after_bytes,
        "darktable fallback should reuse the canonical slot instead of abandoning the pending preview"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn real_local_renderer_sidecar_rejects_an_unpinned_darktable_binary() {
    let base_dir = unique_test_root("local-renderer-version-mismatch");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
    let mismatched_darktable = write_fake_darktable_binary_with_version(&base_dir, "5.5.0");
    let real_sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .join("sidecar")
        .join("local-renderer")
        .join("local-renderer-sidecar.cmd");
    let _env_guard = scoped_env_vars(vec![
        (
            "BOOTHY_LOCAL_RENDERER_BIN",
            Some(real_sidecar.into_os_string()),
        ),
        (
            "BOOTHY_DARKTABLE_CLI_BIN",
            Some(mismatched_darktable.into_os_string()),
        ),
    ]);
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("version mismatch should fall back to darktable");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("reason=local-renderer-sidecar-error"));
    assert!(timing_events.contains("requested=5.4.1 actual=5.5.0"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn real_local_renderer_sidecar_accepts_patch_skew_within_the_same_darktable_minor() {
    let base_dir = unique_test_root("local-renderer-version-patch-skew");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
    let compatible_patch_darktable = write_fake_darktable_binary_with_version(&base_dir, "5.4.0");
    let real_sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .join("sidecar")
        .join("local-renderer")
        .join("local-renderer-sidecar.cmd");
    let _env_guard = scoped_env_vars(vec![
        (
            "BOOTHY_LOCAL_RENDERER_BIN",
            Some(real_sidecar.into_os_string()),
        ),
        (
            "BOOTHY_DARKTABLE_CLI_BIN",
            Some(compatible_patch_darktable.into_os_string()),
        ),
    ]);
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("same-minor darktable patch skew should still let the local renderer close");

    let session_paths = SessionPaths::new(&base_dir, &session.session_id);
    let timing_events = fs::read_to_string(session_paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=renderer-close-owner"));
    assert!(timing_events.contains("detail=route=local-renderer-sidecar"));
    assert!(
        !timing_events.contains("requested=5.4.1 actual=5.4.0"),
        "same-minor patch skew should not force a version-mismatch fallback"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn real_local_renderer_sidecar_reuses_a_runtime_scoped_darktable_version_cache() {
    let base_dir = unique_test_root("local-renderer-version-cache");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
    let (counting_darktable, version_counter_path) =
        write_counting_fake_darktable_binary_with_version(&base_dir, "5.4.1");
    let real_sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .join("sidecar")
        .join("local-renderer")
        .join("local-renderer-sidecar.cmd");
    let _env_guard = scoped_env_vars(vec![
        (
            "BOOTHY_LOCAL_RENDERER_BIN",
            Some(real_sidecar.into_os_string()),
        ),
        (
            "BOOTHY_DARKTABLE_CLI_BIN",
            Some(counting_darktable.into_os_string()),
        ),
    ]);
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root);

    for customer_name in ["Kim", "Lee"] {
        let session = start_session_in_dir(
            &base_dir,
            SessionStartInputDto {
                name: customer_name.into(),
                phone_last_four: "4821".into(),
            },
        )
        .expect("session should be created");
        select_active_preset_in_dir(
            &base_dir,
            boothy_lib::contracts::dto::PresetSelectionInputDto {
                session_id: session.session_id.clone(),
                preset_id: "preset_soft-glow".into(),
                published_version: "2026.03.20".into(),
            },
        )
        .expect("preset should become active");

        let result = request_capture_with_helper_success(&base_dir, &session.session_id);
        let ready_capture = complete_preview_render_in_dir(
            &base_dir,
            &session.session_id,
            &result.capture.capture_id,
        )
        .expect("local renderer should close the preview");
        assert_eq!(ready_capture.render_status, "previewReady");
    }

    let version_probe_count = fs::read_to_string(&version_counter_path)
        .expect("version counter should be readable")
        .lines()
        .count();
    let cache_path = base_dir
        .join(".boothy-local-renderer")
        .join("preview")
        .join("darktable-version-cache.json");
    let cache_contents = fs::read_to_string(&cache_path).expect("version cache should exist");

    assert_eq!(
        version_probe_count, 1,
        "runtime-scoped sidecar cache should avoid probing the darktable version on every capture"
    );
    assert!(
        cache_contents.contains("\"version\":  \"5.4.1\"")
            || cache_contents.contains("\"version\":\"5.4.1\""),
        "cached version metadata should be written once the sidecar verifies the binary"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn real_local_renderer_sidecar_disables_opencl_for_preview_bridge() {
    let base_dir = unique_test_root("local-renderer-disable-opencl");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
    let (argument_logging_darktable, arguments_log_path) =
        write_argument_logging_fake_darktable_binary_with_version(&base_dir, "5.4.1");
    let real_sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .join("sidecar")
        .join("local-renderer")
        .join("local-renderer-sidecar.cmd");
    let _env_guard = scoped_env_vars(vec![
        (
            "BOOTHY_LOCAL_RENDERER_BIN",
            Some(real_sidecar.into_os_string()),
        ),
        (
            "BOOTHY_DARKTABLE_CLI_BIN",
            Some(argument_logging_darktable.into_os_string()),
        ),
    ]);
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("local renderer should close the preview");

    let arguments_log =
        fs::read_to_string(&arguments_log_path).expect("argument log should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(
        arguments_log.contains("--disable-opencl"),
        "real local renderer sidecar should disable opencl to avoid first-run kernel compile stalls in the booth"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn real_local_renderer_sidecar_normalizes_windows_paths_for_darktable_bridge() {
    let base_dir = unique_test_root("local-renderer-normalized-paths");
    write_preset_scoped_preview_render_route_policy(&base_dir, false);
    let (argument_logging_darktable, arguments_log_path) =
        write_powershell_argument_logging_fake_darktable_binary_with_version(&base_dir, "5.4.1");
    let real_sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .join("sidecar")
        .join("local-renderer")
        .join("local-renderer-sidecar.cmd");
    let _env_guard = scoped_env_vars(vec![
        (
            "BOOTHY_LOCAL_RENDERER_BIN",
            Some(real_sidecar.into_os_string()),
        ),
        (
            "BOOTHY_DARKTABLE_CLI_BIN",
            Some(argument_logging_darktable.into_os_string()),
        ),
    ]);
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("normalized sidecar paths should still let the preview close");

    let arguments_log =
        fs::read_to_string(&arguments_log_path).expect("argument log should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(
        !arguments_log.contains(r"\\?\"),
        "sidecar should strip extended-length Windows prefixes before calling darktable"
    );
    assert!(
        arguments_log.contains(":/"),
        "sidecar should hand darktable forward-slash Windows paths so candidate outputs land in the intended session folder"
    );
    assert!(
        !arguments_log.contains(r":\"),
        "sidecar should not forward raw backslash Windows paths into the darktable bridge"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn direct_darktable_preview_disables_opencl_for_the_booth_safe_lane() {
    let base_dir = unique_test_root("direct-preview-disable-opencl");
    fs::create_dir_all(&base_dir).expect("test root should exist");
    let (argument_logging_darktable, arguments_log_path) =
        write_argument_logging_fake_darktable_binary_with_version(&base_dir, "5.4.1");
    let _env_guard = scoped_env_vars(vec![
        (
            "BOOTHY_DARKTABLE_CLI_BIN",
            Some(argument_logging_darktable.into_os_string()),
        ),
        ("BOOTHY_LOCAL_RENDERER_BIN", None),
    ]);
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

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);
    let ready_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("direct darktable preview should close the preview");

    let arguments_log =
        fs::read_to_string(&arguments_log_path).expect("argument log should be readable");

    assert_eq!(ready_capture.render_status, "previewReady");
    assert!(
        arguments_log.contains("--disable-opencl"),
        "the booth-safe direct preview lane should disable opencl so small truthful closes do not pay unnecessary gpu startup cost"
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

fn seed_pending_canonical_preview(
    base_dir: &PathBuf,
    session_id: &str,
    capture_id: &str,
) -> PathBuf {
    let paths = SessionPaths::new(base_dir, session_id);
    let canonical_preview_path = paths.renders_previews_dir.join(format!("{capture_id}.jpg"));
    fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
    fs::write(&canonical_preview_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
        .expect("pending preview should be writable");

    let manifest_path = paths.manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");
    let capture = manifest
        .captures
        .iter_mut()
        .find(|value| value.capture_id == capture_id)
        .expect("capture should exist");
    capture.preview.asset_path = Some(canonical_preview_path.to_string_lossy().into_owned());
    capture.preview.ready_at_ms = None;
    capture.render_status = "previewWaiting".into();
    capture.timing.fast_preview_visible_at_ms = Some(capture.raw.persisted_at_ms + 5);

    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    canonical_preview_path
}

fn write_preview_render_route_policy(base_dir: &PathBuf, session_id: &str, force_disable: bool) {
    let branch_config_dir = base_dir.join("branch-config");
    fs::create_dir_all(&branch_config_dir).expect("branch config dir should exist");

    let forced_fallback_routes = if force_disable {
        vec![serde_json::json!({
            "route": "local-renderer-sidecar",
            "sessionId": session_id,
            "reason": "manual-disable"
        })]
    } else {
        Vec::new()
    };

    fs::write(
        branch_config_dir.join("preview-renderer-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "sessionId": session_id,
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.03.20",
              "reason": "test-canary"
            }
          ],
          "forcedFallbackRoutes": forced_fallback_routes
        }))
        .expect("preview route policy should serialize"),
    )
    .expect("preview route policy should be writable");
}

fn write_branch_scoped_preview_render_route_policy(base_dir: &PathBuf, branch_id: &str) {
    let branch_config_dir = base_dir.join("branch-config");
    fs::create_dir_all(&branch_config_dir).expect("branch config dir should exist");

    fs::write(
        branch_config_dir.join("preview-renderer-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "branchId": branch_id,
              "reason": "branch-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }))
        .expect("preview route policy should serialize"),
    )
    .expect("preview route policy should be writable");
}

fn write_preset_scoped_preview_render_route_policy(base_dir: &PathBuf, force_disable: bool) {
    let branch_config_dir = base_dir.join("branch-config");
    fs::create_dir_all(&branch_config_dir).expect("branch config dir should exist");

    let forced_fallback_routes = if force_disable {
        vec![serde_json::json!({
            "route": "local-renderer-sidecar",
            "presetId": "preset_soft-glow",
            "presetVersion": "2026.03.20",
            "reason": "manual-disable"
        })]
    } else {
        Vec::new()
    };

    fs::write(
        branch_config_dir.join("preview-renderer-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.03.20",
              "reason": "test-canary"
            }
          ],
          "forcedFallbackRoutes": forced_fallback_routes
        }))
        .expect("preview route policy should serialize"),
    )
    .expect("preview route policy should be writable");
}

fn write_branch_rollout_state(base_dir: &PathBuf, branch_id: &str) {
    let branch_config_dir = base_dir.join("branch-config");
    fs::create_dir_all(&branch_config_dir).expect("branch config dir should exist");

    fs::write(
        branch_config_dir.join("state.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "branch-rollout-store/v1",
          "approvedBaselines": [],
          "branches": [
            {
              "branchId": branch_id,
              "displayName": "Gangnam Booth",
              "deploymentBaseline": {
                "buildVersion": "boothy-2026.03.20.4",
                "presetStackVersion": "preset-stack-2026.03.20",
                "approvedAt": "2026-03-20T09:44:49+09:00",
                "actorId": "operator-noah",
                "actorLabel": "Noah Lee"
              },
              "rollbackBaseline": null,
              "pendingBaseline": null,
              "localSettings": {
                "contactPhone": null,
                "contactEmail": null,
                "contactKakao": null,
                "supportHours": null,
                "operationalToggles": []
              },
              "activeSession": null
            }
          ]
        }))
        .expect("branch rollout store should serialize"),
    )
    .expect("branch rollout store should be writable");
}

fn write_fake_local_renderer_sidecar(base_dir: &PathBuf, mode: &str) {
    let sidecar_dir = base_dir.join("sidecar").join("local-renderer");
    fs::create_dir_all(&sidecar_dir).expect("fake sidecar dir should exist");
    let script_path = sidecar_dir.join("local-renderer-sidecar.ps1");
    let wrapper_path = sidecar_dir.join("local-renderer-sidecar.cmd");

    let script_template = r#"
param(
  [string]$requestPath,
  [string]$responsePath
)

$request = Get-Content -Path $requestPath -Raw | ConvertFrom-Json
$mode = "__MODE__"

if ($mode -eq "malformed") {
  [System.IO.File]::WriteAllText($responsePath, "{not-json")
  exit 0
}

if ($mode -eq "timeout") {
  Start-Sleep -Milliseconds 200
}

if ($mode -eq "error-envelope") {
  $response = @{
    schemaVersion = "local-renderer-response/v1"
    error = @{
      message = "darktable bridge failed inside the sidecar"
    }
  }
  [System.IO.File]::WriteAllText(
    $responsePath,
    ($response | ConvertTo-Json -Depth 5)
  )
  exit 1
}

if ($mode -eq "noisy-stderr") {
  foreach ($index in 1..20000) {
    [Console]::Error.WriteLine(("renderer-noise-" + $index.ToString("D5")))
  }
}

$candidatePath = [string]$request.candidateOutputPath
[System.IO.File]::WriteAllBytes($candidatePath, [byte[]](255,216,255,217))

$sessionId = [string]$request.sessionId
$captureId = [string]$request.captureId
$presetVersion = [string]$request.presetVersion
$writtenAt = [int64]$request.capturePersistedAtMs + 10
$completionOrdinal = 1

switch ($mode) {
  "wrong-session" { $sessionId = "session_other" }
  "wrong-capture" { $captureId = "capture_other" }
  "wrong-preset-version" { $presetVersion = "2026.03.21" }
  "stale" { $writtenAt = [int64]$request.capturePersistedAtMs - 1 }
  "duplicate" { $completionOrdinal = 2 }
}

$response = @{
  schemaVersion = "local-renderer-response/v1"
  route = "local-renderer-sidecar"
  sessionId = $sessionId
  captureId = $captureId
  requestId = [string]$request.requestId
  presetId = [string]$request.presetId
  presetVersion = $presetVersion
  candidatePath = $candidatePath
  candidateWrittenAtMs = $writtenAt
  elapsedMs = 120
  fidelity = @{
    verdict = "matched"
    detail = "deltaE=0.4"
  }
  attempt = @{
    retryOrdinal = 0
    completionOrdinal = $completionOrdinal
  }
}

[System.IO.File]::WriteAllText(
  $responsePath,
  ($response | ConvertTo-Json -Depth 5)
)
"#;
    fs::write(&script_path, script_template.replace("__MODE__", mode))
        .expect("fake sidecar script should be writable");
    fs::write(
        &wrapper_path,
        "@echo off\r\npowershell -NoProfile -ExecutionPolicy Bypass -File \"%~dp0local-renderer-sidecar.ps1\" %*\r\n",
    )
    .expect("fake sidecar wrapper should be writable");
}

fn create_published_bundle(catalog_root: &PathBuf) {
    create_named_published_bundle(catalog_root, "preset_soft-glow", "Soft Glow", "2026.03.20");
}

fn write_fake_darktable_binary_with_version(base_dir: &PathBuf, version: &str) -> PathBuf {
    let binary_path = base_dir.join("fake-darktable-versioned.cmd");
    fs::write(
        &binary_path,
        format!(
            concat!(
                "@echo off\r\n",
                "setlocal EnableExtensions EnableDelayedExpansion\r\n",
                "if /I \"%~1\"==\"--version\" (\r\n",
                "  echo darktable-cli {version}\r\n",
                "  exit /b 0\r\n",
                ")\r\n",
                "set \"output=%~3\"\r\n",
                "if \"%output%\"==\"\" exit /b 2\r\n",
                "for %%I in (\"%output%\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\n",
                "powershell -NoProfile -ExecutionPolicy Bypass -Command \"$bytes=[Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAgEAACAQQCAwAAAAAAAAAAAAABAgMABAURITESQVFh/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAT/xAAZEQADAQEBAAAAAAAAAAAAAAAAARECEiH/2gAMAwEAAhEDEQA/AJ9b0qS2K4wqY5lW9L0L4b2E6b9K1+JrZk3QmY2Dg5Nf/2Q==');[IO.File]::WriteAllBytes('%output%',$bytes)\"\r\n",
                "exit /b 0\r\n"
            ),
            version = version
        ),
    )
    .expect("versioned fake darktable should be writable");

    binary_path
}

fn write_counting_fake_darktable_binary_with_version(
    base_dir: &PathBuf,
    version: &str,
) -> (PathBuf, PathBuf) {
    let binary_path = base_dir.join("fake-darktable-counting.cmd");
    let version_counter_path = base_dir.join("fake-darktable-version-counter.log");
    fs::write(
        &binary_path,
        format!(
            concat!(
                "@echo off\r\n",
                "setlocal EnableExtensions EnableDelayedExpansion\r\n",
                "set \"counter_file={counter_file}\"\r\n",
                "if /I \"%~1\"==\"--version\" (\r\n",
                "  >>\"%counter_file%\" echo version\r\n",
                "  echo darktable-cli {version}\r\n",
                "  exit /b 0\r\n",
                ")\r\n",
                "set \"output=%~3\"\r\n",
                "if \"%output%\"==\"\" exit /b 2\r\n",
                "for %%I in (\"%output%\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\n",
                "powershell -NoProfile -ExecutionPolicy Bypass -Command \"$bytes=[Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAgEAACAQQCAwAAAAAAAAAAAAABAgMABAURITESQVFh/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAT/xAAZEQADAQEBAAAAAAAAAAAAAAAAARECEiH/2gAMAwEAAhEDEQA/AJ9b0qS2K4wqY5lW9L0L4b2E6b9K1+JrZk3QmY2Dg5Nf/2Q==');[IO.File]::WriteAllBytes('%output%',$bytes)\"\r\n",
                "exit /b 0\r\n"
            ),
            version = version,
            counter_file = version_counter_path.to_string_lossy()
        ),
    )
    .expect("counting fake darktable should be writable");

    (binary_path, version_counter_path)
}

fn write_argument_logging_fake_darktable_binary_with_version(
    base_dir: &PathBuf,
    version: &str,
) -> (PathBuf, PathBuf) {
    let binary_path = base_dir.join("fake-darktable-args.cmd");
    let arguments_log_path = base_dir.join("fake-darktable-args.log");
    fs::write(
        &binary_path,
        format!(
            concat!(
                "@echo off\r\n",
                "setlocal EnableExtensions EnableDelayedExpansion\r\n",
                "set \"args_log={args_log}\"\r\n",
                "if /I \"%~1\"==\"--version\" (\r\n",
                "  echo darktable-cli {version}\r\n",
                "  exit /b 0\r\n",
                ")\r\n",
                ">>\"%args_log%\" echo %*\r\n",
                "set \"output=%~3\"\r\n",
                "if \"%output%\"==\"\" exit /b 2\r\n",
                "for %%I in (\"%output%\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\n",
                "powershell -NoProfile -ExecutionPolicy Bypass -Command \"$bytes=[Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAgEAACAQQCAwAAAAAAAAAAAAABAgMABAURITESQVFh/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAT/xAAZEQADAQEBAAAAAAAAAAAAAAAAARECEiH/2gAMAwEAAhEDEQA/AJ9b0qS2K4wqY5lW9L0L4b2E6b9K1+JrZk3QmY2Dg5Nf/2Q==');[IO.File]::WriteAllBytes('%output%',$bytes)\"\r\n",
                "exit /b 0\r\n"
            ),
            version = version,
            args_log = arguments_log_path.to_string_lossy()
        ),
    )
    .expect("argument logging fake darktable should be writable");

    (binary_path, arguments_log_path)
}

fn write_powershell_argument_logging_fake_darktable_binary_with_version(
    base_dir: &PathBuf,
    version: &str,
) -> (PathBuf, PathBuf) {
    let script_path = base_dir.join("fake-darktable-args.ps1");
    let arguments_log_path = base_dir.join("fake-darktable-args-ps.log");
    fs::write(
        &script_path,
        format!(
            concat!(
                "param([Parameter(ValueFromRemainingArguments=$true)][string[]]$argv)\r\n",
                "$ErrorActionPreference = 'Stop'\r\n",
                "if ($argv.Count -gt 0 -and $argv[0] -eq '--version') {{\r\n",
                "  Write-Output 'darktable-cli {version}'\r\n",
                "  exit 0\r\n",
                "}}\r\n",
                "$argsLog = '{args_log}'\r\n",
                "[System.IO.File]::AppendAllText($argsLog, (($argv -join ' ') + [Environment]::NewLine))\r\n",
                "$output = if ($argv.Count -ge 3) {{ $argv[2] }} else {{ '' }}\r\n",
                "if ([string]::IsNullOrWhiteSpace($output)) {{ exit 2 }}\r\n",
                "$outputDir = [System.IO.Path]::GetDirectoryName($output)\r\n",
                "if (-not [string]::IsNullOrWhiteSpace($outputDir)) {{ [System.IO.Directory]::CreateDirectory($outputDir) | Out-Null }}\r\n",
                "$bytes = [Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAgEAACAQQCAwAAAAAAAAAAAAABAgMABAURITESQVFh/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAT/xAAZEQADAQEBAAAAAAAAAAAAAAAAARECEiH/2gAMAwEAAhEDEQA/AJ9b0qS2K4wqY5lW9L0L4b2E6b9K1+JrZk3QmY2Dg5Nf/2Q==')\r\n",
                "[System.IO.File]::WriteAllBytes($output, $bytes)\r\n",
                "exit 0\r\n"
            ),
            version = version,
            args_log = arguments_log_path.to_string_lossy().replace('\\', "\\\\")
        ),
    )
    .expect("powershell argument logging fake darktable should be writable");

    (script_path, arguments_log_path)
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

fn request_capture_with_helper_success(
    base_dir: &PathBuf,
    session_id: &str,
) -> CaptureRequestResultDto {
    request_capture_with_helper_success_for_request_id(base_dir, session_id, None)
}

fn request_capture_with_helper_success_for_request_id(
    base_dir: &PathBuf,
    session_id: &str,
    request_id: Option<&str>,
) -> CaptureRequestResultDto {
    write_ready_helper_status(base_dir, session_id);
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
            request_id: request_id.map(str::to_string),
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

fn append_file_arrived_event(
    base_dir: &PathBuf,
    session_id: &str,
    request: &CanonHelperCaptureRequestMessage,
    capture_id: &str,
    raw_path: &std::path::Path,
    fast_preview_path: Option<&std::path::Path>,
    fast_preview_kind: Option<&str>,
) {
    let mut event = serde_json::json!({
      "schemaVersion": CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
      "type": "file-arrived",
      "sessionId": request.session_id.clone(),
      "requestId": request.request_id.clone(),
      "captureId": capture_id,
      "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
      "rawPath": raw_path.to_string_lossy().into_owned(),
    });

    if let Some(path) = fast_preview_path {
        event["fastPreviewPath"] = serde_json::Value::String(path.to_string_lossy().into_owned());
    }

    if let Some(kind) = fast_preview_kind {
        event["fastPreviewKind"] = serde_json::Value::String(kind.into());
    }

    append_helper_event(base_dir, session_id, event);
}

fn append_fast_preview_ready_event(
    base_dir: &PathBuf,
    session_id: &str,
    request: &CanonHelperCaptureRequestMessage,
    capture_id: &str,
    fast_preview_path: &std::path::Path,
    fast_preview_kind: Option<&str>,
) {
    let mut event = serde_json::json!({
      "schemaVersion": CANON_HELPER_FAST_PREVIEW_READY_SCHEMA_VERSION,
      "type": "fast-preview-ready",
      "sessionId": request.session_id.clone(),
      "requestId": request.request_id.clone(),
      "captureId": capture_id,
      "observedAt": current_timestamp(SystemTime::now()).expect("fast preview timestamp should serialize"),
      "fastPreviewPath": fast_preview_path.to_string_lossy().into_owned(),
    });

    if let Some(kind) = fast_preview_kind {
        event["fastPreviewKind"] = serde_json::Value::String(kind.into());
    }

    append_helper_event(base_dir, session_id, event);
}

fn append_fast_thumbnail_attempted_event(
    base_dir: &PathBuf,
    session_id: &str,
    request: &CanonHelperCaptureRequestMessage,
    capture_id: &str,
    fast_preview_kind: Option<&str>,
) {
    let mut event = serde_json::json!({
      "schemaVersion": CANON_HELPER_FAST_THUMBNAIL_ATTEMPTED_SCHEMA_VERSION,
      "type": "fast-thumbnail-attempted",
      "sessionId": request.session_id.clone(),
      "requestId": request.request_id.clone(),
      "captureId": capture_id,
      "observedAt": current_timestamp(SystemTime::now()).expect("fast thumbnail attempt timestamp should serialize"),
    });

    if let Some(kind) = fast_preview_kind {
        event["fastPreviewKind"] = serde_json::Value::String(kind.into());
    }

    append_helper_event(base_dir, session_id, event);
}

fn append_fast_thumbnail_failed_event(
    base_dir: &PathBuf,
    session_id: &str,
    request: &CanonHelperCaptureRequestMessage,
    capture_id: &str,
    detail_code: &str,
    fast_preview_kind: Option<&str>,
) {
    let mut event = serde_json::json!({
      "schemaVersion": CANON_HELPER_FAST_THUMBNAIL_FAILED_SCHEMA_VERSION,
      "type": "fast-thumbnail-failed",
      "sessionId": request.session_id.clone(),
      "requestId": request.request_id.clone(),
      "captureId": capture_id,
      "observedAt": current_timestamp(SystemTime::now()).expect("fast thumbnail failure timestamp should serialize"),
      "detailCode": detail_code,
    });

    if let Some(kind) = fast_preview_kind {
        event["fastPreviewKind"] = serde_json::Value::String(kind.into());
    }

    append_helper_event(base_dir, session_id, event);
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
