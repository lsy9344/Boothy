use std::{
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex, Once},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    capture::{
        helper_supervisor::shutdown_helper_process,
        ingest_pipeline::{
            complete_final_render_in_dir, complete_preview_render_in_dir,
            mark_final_render_failed_in_dir, mark_preview_render_failed_in_dir,
        },
        normalized_state::{
            delete_capture_in_dir, export_captures_in_dir, get_capture_readiness_in_dir,
            request_capture_in_dir, request_capture_in_dir_with_fast_preview,
        },
        sidecar_client::{
            read_capture_request_messages, write_capture_request_message,
            CanonHelperCaptureRequestMessage, CAMERA_HELPER_EVENTS_FILE_NAME,
            CAMERA_HELPER_PROCESSED_REQUEST_IDS_FILE_NAME,
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
        CaptureDeleteInputDto, CaptureExportInputDto, CaptureReadinessInputDto,
        CaptureRequestInputDto, CaptureRequestResultDto, SessionStartInputDto,
    },
    preset::default_catalog::ensure_default_preset_catalog_in_dir,
    preset::preset_catalog::resolve_published_preset_catalog_dir,
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

struct HelperSupervisorShutdownGuard;

impl Drop for HelperSupervisorShutdownGuard {
    fn drop(&mut self) {
        shutdown_helper_process();
    }
}

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

fn write_decodable_test_jpeg(path: &std::path::Path) {
    let mut encoded = Vec::new();
    let pixels = [240, 230, 220, 210, 200, 190, 180, 170, 160, 150, 140, 130];
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 86);
    encoder
        .encode(&pixels, 2, 2, image::ColorType::Rgb8.into())
        .expect("jpeg fixture should encode");
    fs::write(path, encoded).expect("jpeg fixture should be writable");
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
fn readiness_escalates_when_helper_startup_status_stays_stale() {
    let base_dir = unique_test_root("stale-helper-startup-status");
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
    let stale_observed_at = current_timestamp(
        SystemTime::now()
            .checked_sub(Duration::from_secs(30))
            .expect("stale startup status should compute"),
    )
    .expect("helper timestamp should serialize");
    write_helper_status_with_detail(
        &base_dir,
        &session.session_id,
        "connecting",
        "starting",
        &stale_observed_at,
        Some("helper-starting"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("stale helper startup readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("stale startup readiness should expose live capture truth");
    assert_eq!(live_truth.freshness, "stale");
    assert_eq!(live_truth.camera_state, "connecting");
    assert_eq!(live_truth.helper_state, "starting");
    assert_eq!(live_truth.detail_code.as_deref(), Some("helper-starting"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_escalates_when_windows_device_detected_is_the_last_stale_startup_status() {
    let base_dir = unique_test_root("stale-windows-device-detected-status");
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
    let stale_observed_at = current_timestamp(
        SystemTime::now()
            .checked_sub(Duration::from_secs(30))
            .expect("stale startup status should compute"),
    )
    .expect("helper timestamp should serialize");
    write_helper_status_with_detail(
        &base_dir,
        &session.session_id,
        "connecting",
        "healthy",
        &stale_observed_at,
        Some("windows-device-detected"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("stale windows-device-detected readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("stale startup readiness should expose live capture truth");
    assert_eq!(live_truth.freshness, "stale");
    assert_eq!(live_truth.camera_state, "connecting");
    assert_eq!(live_truth.helper_state, "healthy");
    assert_eq!(
        live_truth.detail_code.as_deref(),
        Some("windows-device-detected")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_escalates_when_fresh_startup_oscillation_repeats_far_past_session_start() {
    let base_dir = unique_test_root("fresh-startup-oscillation");
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

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.created_at = timestamp_offset(-16);
    manifest.updated_at = timestamp_offset(-14);
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    write_helper_status_with_sequence_and_detail(
        &base_dir,
        &session.session_id,
        36,
        "connecting",
        "starting",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("sdk-initializing"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("fresh startup oscillation readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("fresh startup oscillation should expose live capture truth");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.camera_state, "connecting");
    assert_eq!(live_truth.helper_state, "starting");
    assert_eq!(live_truth.sequence, Some(36));
    assert_eq!(live_truth.detail_code.as_deref(), Some("sdk-initializing"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_escalates_when_dense_startup_family_is_already_over_five_seconds_old() {
    let base_dir = unique_test_root("dense-startup-family-five-seconds");
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

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.created_at = timestamp_offset(-8);
    manifest.updated_at = timestamp_offset(-6);
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    write_helper_status_with_sequence_and_detail(
        &base_dir,
        &session.session_id,
        31,
        "connecting",
        "connecting",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("session-opening"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("dense startup family readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("dense startup family should expose live capture truth");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.camera_state, "connecting");
    assert_eq!(live_truth.helper_state, "connecting");
    assert_eq!(live_truth.sequence, Some(31));
    assert_eq!(live_truth.detail_code.as_deref(), Some("session-opening"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_escalates_when_fresh_startup_family_persists_past_session_budget_even_with_low_sequence(
) {
    let base_dir = unique_test_root("fresh-startup-family-low-sequence");
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

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.created_at = timestamp_offset(-16);
    manifest.updated_at = timestamp_offset(-14);
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    write_helper_status_with_sequence_and_detail(
        &base_dir,
        &session.session_id,
        7,
        "connecting",
        "connecting",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("session-opening"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("fresh startup family low-sequence readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("fresh startup family low-sequence should expose live capture truth");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.camera_state, "connecting");
    assert_eq!(live_truth.helper_state, "connecting");
    assert_eq!(live_truth.sequence, Some(7));
    assert_eq!(live_truth.detail_code.as_deref(), Some("session-opening"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_escalates_when_low_sequence_startup_family_is_already_over_eight_seconds_old() {
    let base_dir = unique_test_root("fresh-startup-family-eight-seconds");
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

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.created_at = timestamp_offset(-11);
    manifest.updated_at = timestamp_offset(-9);
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    write_helper_status_with_sequence_and_detail(
        &base_dir,
        &session.session_id,
        7,
        "connecting",
        "healthy",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("windows-device-detected"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("eight-second startup family readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("eight-second startup family should expose live capture truth");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.camera_state, "connecting");
    assert_eq!(live_truth.helper_state, "healthy");
    assert_eq!(live_truth.sequence, Some(7));
    assert_eq!(
        live_truth.detail_code.as_deref(),
        Some("windows-device-detected")
    );

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
fn readiness_holds_the_first_camera_ready_for_a_brief_preset_selection_window() {
    let base_dir = unique_test_root("ready-preset-selection-stabilization-window");
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
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.created_at = timestamp_offset(-2);
    manifest.updated_at = timestamp_offset(-2);
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");
    write_helper_status_with_sequence_and_detail(
        &base_dir,
        &session.session_id,
        8,
        "ready",
        "healthy",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("camera-ready"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("preset-selection stabilization readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "camera-preparing");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("stabilized readiness should include live truth");
    assert_eq!(live_truth.camera_state, "ready");
    assert_eq!(live_truth.helper_state, "healthy");
    assert_eq!(live_truth.detail_code.as_deref(), Some("camera-ready"));
    assert_eq!(live_truth.sequence, Some(8));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_the_first_camera_ready_after_the_preset_selection_window() {
    let base_dir = unique_test_root("ready-after-preset-selection-stabilization-window");
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
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.created_at = timestamp_offset(-7);
    manifest.updated_at = timestamp_offset(-6);
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");
    write_helper_status_with_sequence_and_detail(
        &base_dir,
        &session.session_id,
        8,
        "ready",
        "healthy",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("camera-ready"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("post-preset-selection stabilization readiness should resolve");

    assert_eq!(readiness.customer_state, "Ready");
    assert!(readiness.can_capture);
    assert_eq!(readiness.reason_code, "ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_rejects_future_dated_helper_status_as_not_fresh() {
    let base_dir = unique_test_root("future-dated-helper-status");
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

    let future_observed_at = current_timestamp(
        SystemTime::now()
            .checked_add(Duration::from_secs(30))
            .expect("future helper timestamp should compute"),
    )
    .expect("helper timestamp should serialize");
    write_helper_status(
        &base_dir,
        &session.session_id,
        "ready",
        "healthy",
        &future_observed_at,
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("future-dated helper readiness should resolve");

    assert_eq!(readiness.customer_state, "Preparing");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "camera-preparing");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("future-dated status should still expose live capture truth");
    assert_eq!(live_truth.freshness, "stale");

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
fn startup_connect_timeout_routes_to_phone_required_before_first_capture() {
    let base_dir = unique_test_root("startup-connect-timeout");
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
    write_helper_status_with_detail(
        &base_dir,
        &session.session_id,
        "error",
        "error",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("camera-connect-timeout"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("startup connect timeout readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("startup connect timeout should expose live capture truth");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.camera_state, "error");
    assert_eq!(live_truth.helper_state, "error");
    assert_eq!(
        live_truth.detail_code.as_deref(),
        Some("camera-connect-timeout")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn session_open_timeout_routes_to_phone_required_before_first_capture() {
    let base_dir = unique_test_root("session-open-timeout");
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
    write_helper_status_with_detail(
        &base_dir,
        &session.session_id,
        "error",
        "error",
        &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
        Some("session-open-timeout"),
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id,
        },
    )
    .expect("session-open-timeout readiness should resolve");

    assert_eq!(readiness.customer_state, "Phone Required");
    assert!(!readiness.can_capture);
    assert_eq!(readiness.reason_code, "phone-required");
    let live_truth = readiness
        .live_capture_truth
        .as_ref()
        .expect("session-open-timeout should expose live capture truth");
    assert_eq!(live_truth.freshness, "fresh");
    assert_eq!(live_truth.camera_state, "error");
    assert_eq!(live_truth.helper_state, "error");
    assert_eq!(
        live_truth.detail_code.as_deref(),
        Some("session-open-timeout")
    );

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
    update_stage(&base_dir, &session.session_id, "capture-ready");
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
        let capture_id = format!("capture_force-process-fail_{}", &request.request_id[8..]);
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
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("legacy-canonical-scan")
    );
    assert_eq!(result.capture.preview.ready_at_ms, None);
    assert_eq!(result.capture.timing.xmp_preview_ready_at_ms, None);
    assert_eq!(
        result.capture.preview.asset_path.as_deref(),
        Some(preview_path.to_string_lossy().as_ref()),
    );
    assert_valid_jpeg(preview_path.to_string_lossy().as_ref());
    assert!(
        result.capture.timing.fast_preview_visible_at_ms.is_some(),
        "same-capture thumbnail should still populate first-visible timing while truthful close stays pending"
    );

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
    assert_eq!(manifest.captures[0].timing.xmp_preview_ready_at_ms, None);

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
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("camera-thumbnail")
    );
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
    assert!(timing_events.contains("originalVisibleToPresetAppliedVisibleMs="));
    assert!(timing_events.contains("presetAppliedVisibleAtMs="));
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
fn preview_render_keeps_fast_first_visible_but_closes_truthfully_from_raw_after_handoff() {
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
    assert!(timing_events.contains("sourceAsset=raw-original"));
    assert!(
        !timing_events.contains("sourceAsset=fast-preview-raster"),
        "camera thumbnails should stay first-visible only and not satisfy preview-ready"
    );
    assert!(timing_events.contains(&format!("request={}", result.capture.request_id)));

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
            .join(format!("{capture_id}.CR2"));

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
    .expect("capture should save");
    helper_thread
        .join()
        .expect("helper capture thread should complete");
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
fn legacy_canonical_scan_activates_reserve_close_and_records_preset_applied_owner() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("legacy-canonical-scan-reserve-close");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_decodable_test_jpeg(&canonical_preview_path);

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
            None,
            None,
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

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("legacy-canonical-scan")
    );

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("legacy canonical preview should stay visible until the speculative truthful close finishes");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=fast-preview-promoted"));
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("kind=legacy-canonical-scan"));
    assert!(timing_events.contains("sourceAsset=raw-original"));
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        2,
        "legacy canonical scan should keep first-visible alive, then raw fallback must own the truthful close"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn early_windows_shell_thumbnail_is_preserved_and_starts_reserve_close_before_file_arrival_metadata(
) {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("early-windows-shell-thumbnail");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_decodable_test_jpeg(&canonical_preview_path);

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
            &canonical_preview_path,
            Some("windows-shell-thumbnail"),
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

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("windows-shell-thumbnail")
    );

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("early helper preview should already own the speculative reserve close");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("kind=windows-shell-thumbnail"));
    assert!(
        !timing_events.contains("kind=legacy-canonical-scan"),
        "early helper preview metadata should stay attached to the capture instead of falling back to canonical-scan inference"
    );
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        2,
        "early non-truthful helper preview starts comparison work once, then raw fallback owns the truthful close"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn later_non_truthful_file_arrived_metadata_does_not_replace_first_visible_baseline() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("late-non-truthful-file-arrived-does-not-replace-baseline");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_decodable_test_jpeg(&canonical_preview_path);

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
            &canonical_preview_path,
            Some("windows-shell-thumbnail"),
        );
        thread::sleep(Duration::from_millis(40));
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&canonical_preview_path),
            Some("camera-thumbnail"),
        );
    });

    let mut preview_updates: Vec<Option<String>> = Vec::new();
    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |update| preview_updates.push(update.kind),
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("windows-shell-thumbnail")
    );
    assert_eq!(
        preview_updates,
        vec![Some("windows-shell-thumbnail".into())],
        "later non-truthful file-arrived metadata should not replace the first visible preview update"
    );

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("capture should still close truthfully");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preset_applied_fast_preview_ready_metadata_does_not_close_truth_without_renderer_proof() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("truthful-fast-preview-is-not-downgraded");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_decodable_test_jpeg(&canonical_preview_path);

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
            &canonical_preview_path,
            Some("preset-applied-preview"),
        );
        thread::sleep(Duration::from_millis(40));
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&canonical_preview_path),
            Some("camera-thumbnail"),
        );
    });

    let mut preview_updates: Vec<Option<String>> = Vec::new();
    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |update| preview_updates.push(update.kind),
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert!(result.capture.preview.ready_at_ms.is_none());
    assert!(result.capture.timing.xmp_preview_ready_at_ms.is_none());
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("preset-applied-preview")
    );
    assert_eq!(
        preview_updates,
        vec![Some("preset-applied-preview".into())],
        "preset-applied fast-preview metadata may be first-visible, but must not close preview truth"
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    let saved_capture = manifest
        .captures
        .last()
        .expect("the saved capture should be present in the manifest");
    assert_eq!(saved_capture.render_status, "previewWaiting");
    assert!(saved_capture.preview.ready_at_ms.is_none());
    assert!(saved_capture.timing.xmp_preview_ready_at_ms.is_none());
    assert_eq!(
        saved_capture.preview.kind.as_deref(),
        Some("preset-applied-preview")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn truthful_close_reemits_when_same_canonical_path_upgrades_preview_kind() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("truthful-close-reemits-same-canonical-path-upgrade");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_decodable_test_jpeg(&canonical_preview_path);

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
            &canonical_preview_path,
            Some("windows-shell-thumbnail"),
        );
        thread::sleep(Duration::from_millis(40));
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &raw_path,
            Some(&canonical_preview_path),
            Some("preset-applied-preview"),
        );
    });

    let mut preview_updates: Vec<Option<String>> = Vec::new();
    let result = request_capture_in_dir_with_fast_preview(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
        |update| preview_updates.push(update.kind),
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    assert_eq!(result.capture.render_status, "previewReady");
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("preset-applied-preview")
    );
    assert_eq!(
        preview_updates,
        vec![
            Some("windows-shell-thumbnail".into()),
            Some("preset-applied-preview".into())
        ],
        "truthful close should emit a second upgrade event even when it reuses the canonical preview path"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn late_windows_shell_thumbnail_is_preserved_after_file_arrival_without_metadata() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("late-windows-shell-thumbnail");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_decodable_test_jpeg(&canonical_preview_path);

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
            None,
            None,
        );
        thread::sleep(Duration::from_millis(400));
        write_decodable_test_jpeg(&canonical_preview_path);
        append_fast_preview_ready_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &canonical_preview_path,
            Some("windows-shell-thumbnail"),
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

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("windows-shell-thumbnail")
    );

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("late helper preview should still own the same-capture source metadata");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("kind=windows-shell-thumbnail"));
    assert!(
        !timing_events.contains("kind=legacy-canonical-scan"),
        "late helper preview metadata should replace canonical-scan inference for the same request"
    );
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        2,
        "late non-truthful helper preview should keep first-visible, then raw fallback owns truth"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn late_windows_shell_thumbnail_is_preserved_even_when_helper_preview_arrives_after_render_start() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("late-windows-shell-thumbnail-after-render-start");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
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
            None,
            None,
        );
        write_test_jpeg(&canonical_preview_path);

        wait_for_timing_event(
            &helper_base_dir,
            &helper_session_id,
            "event=preview-render-start",
        );

        append_fast_preview_ready_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            &capture_id,
            &canonical_preview_path,
            Some("windows-shell-thumbnail"),
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

    assert_eq!(result.capture.render_status, "previewWaiting");
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("windows-shell-thumbnail")
    );

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("late helper preview metadata should still replace canonical-scan inference");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("kind=windows-shell-thumbnail"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn late_preset_applied_handoff_file_does_not_close_truth_without_renderer_proof() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("late-truthful-handoff-without-kind");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
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
            None,
            None,
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

    let truthful_handoff_path = SessionPaths::new(&base_dir, &session.session_id)
        .handoff_dir
        .join("fast-preview")
        .join(format!(
            "{}.preset-applied-preview.jpg",
            result.capture.capture_id
        ));
    fs::create_dir_all(
        truthful_handoff_path
            .parent()
            .expect("truthful handoff path should have a parent"),
    )
    .expect("truthful handoff directory should exist");
    write_test_jpeg(&truthful_handoff_path);

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("late metadata-only handoff should fall back to renderer proof");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=fast-preview-promoted"));
    assert!(timing_events.contains("sourceAsset=raw-original"));
    assert!(
        !timing_events.contains("metadata-only-fast-preview")
            && !timing_events.contains("truthProfile=metadata-only-not-renderer-proof")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preset_applied_fast_preview_metadata_does_not_close_truth_without_renderer_proof() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("preset-applied-fast-preview-truth-owner");
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
        let truthful_preview_path = session_paths
            .handoff_dir
            .join("fast-preview")
            .join(format!("{capture_id}.preset-applied-preview.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::create_dir_all(
            truthful_preview_path
                .parent()
                .expect("truthful preview path should have a parent directory"),
        )
        .expect("truthful preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
        write_test_jpeg(&truthful_preview_path);

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
            Some(&truthful_preview_path),
            Some("preset-applied-preview"),
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

    assert_ne!(
        result.capture.render_status, "previewReady",
        "preset-applied metadata alone must not own truthful close immediately"
    );
    assert!(result.capture.preview.ready_at_ms.is_none());
    assert!(result.capture.timing.xmp_preview_ready_at_ms.is_none());
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("preset-applied-preview")
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should resolve");
    assert_ne!(readiness.surface_state, "previewReady");

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=fast-preview-promoted"));
    assert!(timing_events.contains("kind=preset-applied-preview"));
    assert!(!timing_events.contains("event=preview-render-ready"));
    assert!(!timing_events.contains("event=capture_preview_ready"));
    assert!(!timing_events.contains("truthProfile=original-full-preset"));
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        0,
        "truthful reserve path close should not reopen the darktable hot path"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_reuses_a_late_same_capture_preview_before_raw_fallback() {
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

    let refined_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect(
            "late same-capture preview should stay visible while the raw render closes truthfully",
        );

    assert!(refined_capture.timing.fast_preview_visible_at_ms.is_some());
    assert_eq!(refined_capture.render_status, "previewReady");
    assert_eq!(
        refined_capture.preview.ready_at_ms,
        refined_capture.timing.xmp_preview_ready_at_ms
    );
    assert_eq!(
        refined_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        refined_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=fast-preview-promoted"));
    let speculative_lock_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!(
            "{}.{}.preview-speculative.lock",
            result.capture.capture_id, result.capture.request_id
        ));
    let speculative_source_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!(
            "{}.{}.preview-speculative-source.jpg",
            result.capture.capture_id, result.capture.request_id
        ));
    for _ in 0..40 {
        if !speculative_lock_path.exists() && !speculative_source_path.exists() {
            break;
        }

        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !speculative_lock_path.exists(),
        "legacy canonical reserve closes should clean up the speculative lock"
    );
    assert!(
        !speculative_source_path.exists(),
        "legacy canonical reserve closes should clean up the staged speculative source"
    );
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        2,
        "late non-truthful same-capture preview can keep first-visible alive, then raw fallback must own the truthful close"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_direct_close_reuses_existing_same_capture_preview_as_preset_applied_owner(
) {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("direct-close-prefers-existing-preview");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
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
            None,
            None,
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

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));
    let speculative_lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        result.capture.capture_id, result.capture.request_id
    ));
    let _ = fs::remove_file(&speculative_output_path);
    let _ = fs::remove_file(&speculative_detail_path);
    let _ = fs::remove_file(&speculative_lock_path);

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("existing same-capture preview without truthful metadata should fall back to the raw render");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("kind=legacy-canonical-scan"));
    assert!(timing_events.contains("sourceAsset=raw-original"));
    assert_eq!(
        timing_events.matches("event=preview-render-start").count(),
        2,
        "if the speculative path is gone, the booth should fall back to one direct render after the earlier speculative attempt was already started"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_rejects_non_truthful_existing_preview_when_raw_refinement_fails() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("direct-close-on-raw-refinement-failure");
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
                .expect("preview path should have a parent directory"),
        )
        .expect("preview directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");
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
            None,
            None,
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

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));
    let speculative_lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        result.capture.capture_id, result.capture.request_id
    ));
    let _ = fs::remove_file(&speculative_output_path);
    let _ = fs::remove_file(&speculative_detail_path);
    let _ = fs::remove_file(&speculative_lock_path);
    let failing_darktable_cli = base_dir.join("always-fail-darktable-cli.cmd");
    fs::write(&failing_darktable_cli, "@echo off\r\nexit /b 17\r\n")
        .expect("failing darktable cli should be writable");
    let previous_darktable_cli = std::env::var("BOOTHY_DARKTABLE_CLI_BIN").ok();
    std::env::set_var(
        "BOOTHY_DARKTABLE_CLI_BIN",
        failing_darktable_cli.to_string_lossy().into_owned(),
    );

    let render_result =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id);

    match previous_darktable_cli {
        Some(previous_darktable_cli) => {
            std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", previous_darktable_cli);
        }
        None => std::env::remove_var("BOOTHY_DARKTABLE_CLI_BIN"),
    }

    assert!(
        render_result.is_err(),
        "non-truthful same-capture preview must not close product readiness when raw refinement fails"
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should still resolve after rejected fallback close");
    assert_ne!(readiness.surface_state, "previewReady");

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=preview-render-failed"));
    assert!(!timing_events.contains("sourceAsset=legacy-canonical-scan"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_rejects_fast_preview_raster_speculative_truth_close() {
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

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("raw original preview close should replace fast-raster speculative output");

    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert!(
        completed_capture.timing.xmp_preview_ready_at_ms.is_some(),
        "raw-backed preview should own xmp readiness"
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(
        timing_events.contains("sourceAsset=raw-original"),
        "fast-preview-raster speculative output must not close preview truth: {timing_events}"
    );
    assert!(
        !timing_events.contains("inputSourceAsset=fast-preview-raster"),
        "fast-preview-raster speculative output must remain comparison-only: {timing_events}"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn final_render_waits_for_preview_ready_not_pending_fast_preview() {
    let base_dir = unique_test_root("final-render-waits-for-preview-ready");
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
    let pending_preview_path = paths
        .renders_previews_dir
        .join(format!("{}.jpg", result.capture.capture_id));
    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    write_test_jpeg(&pending_preview_path);

    let mut manifest = read_manifest(&base_dir, &session.session_id);
    let capture = manifest
        .captures
        .iter_mut()
        .find(|capture| capture.capture_id == result.capture.capture_id)
        .expect("capture should exist in manifest");
    capture.preview.asset_path = Some(pending_preview_path.to_string_lossy().into_owned());
    capture.preview.ready_at_ms = None;
    capture.render_status = "previewWaiting".into();
    fs::write(
        &paths.manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");

    let error =
        complete_final_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect_err("pending fast preview must not allow final render");

    assert!(
        error.message.contains("확인용 사진"),
        "customer-safe error should explain preview is not ready: {}",
        error.message
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn active_session_export_prepares_ready_captures_without_post_end_completion() {
    let base_dir = unique_test_root("active-session-export-ready-captures");
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
    .expect("first preview should complete");
    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second preview should complete");

    let result = export_captures_in_dir(
        &base_dir,
        CaptureExportInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("active session export should complete");

    assert_eq!(result.schema_version, "capture-export-result/v1");
    assert_eq!(result.exported_count, 2);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.readiness.reason_code, "ready");
    assert!(result.readiness.can_capture);
    assert_ne!(result.manifest.lifecycle.stage, "completed");
    assert_ne!(result.manifest.lifecycle.stage, "export-waiting");
    assert!(result.manifest.post_end.is_none());

    for capture_id in [
        first_capture.capture.capture_id.as_str(),
        second_capture.capture.capture_id.as_str(),
    ] {
        let capture = result
            .manifest
            .captures
            .iter()
            .find(|candidate| candidate.capture_id == capture_id)
            .expect("exported capture should remain in manifest");
        assert_eq!(capture.render_status, "finalReady");
        assert_eq!(capture.post_end_state, "activeSession");
        assert_valid_jpeg(
            capture
                .final_asset
                .asset_path
                .as_deref()
                .expect("final asset path should be recorded"),
        );
        assert!(
            capture.final_asset.ready_at_ms.is_some(),
            "final readiness time should be recorded"
        );
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn active_session_export_does_not_block_new_capture_or_include_late_preview() {
    let base_dir = unique_test_root("active-session-export-overlap-capture");
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
    .expect("first preview should complete");

    let fake_delay_path = base_dir
        .join(".boothy-darktable")
        .join("fake-darktable-sleep-ms.txt");
    fs::create_dir_all(
        fake_delay_path
            .parent()
            .expect("delay path should have parent"),
    )
    .expect("fake delay directory should exist");
    fs::write(&fake_delay_path, "900").expect("fake delay should write");

    let export_base_dir = base_dir.clone();
    let export_session_id = session.session_id.clone();
    let export_thread = thread::spawn(move || {
        export_captures_in_dir(
            &export_base_dir,
            CaptureExportInputDto {
                session_id: export_session_id,
            },
        )
        .expect("active session export should complete")
    });

    thread::sleep(Duration::from_millis(120));

    let capture_start = Instant::now();
    let late_capture = request_capture_with_helper_success_for_request_id(
        &base_dir,
        &session.session_id,
        Some("request_export_overlap_capture"),
    );
    let capture_elapsed = capture_start.elapsed();

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let late_preview_path = paths
        .renders_previews_dir
        .join(format!("{}.jpg", late_capture.capture.capture_id));
    fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
    write_decodable_test_jpeg(&late_preview_path);

    let mut manifest = read_manifest(&base_dir, &session.session_id);
    let late_capture_record = manifest
        .captures
        .iter_mut()
        .find(|capture| capture.capture_id == late_capture.capture.capture_id)
        .expect("late capture should exist in manifest");
    late_capture_record.preview.asset_path = Some(late_preview_path.to_string_lossy().into_owned());
    late_capture_record.preview.ready_at_ms = Some(late_capture_record.raw.persisted_at_ms + 100);
    late_capture_record.render_status = "previewReady".into();
    fs::write(
        &paths.manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");

    let result = export_thread
        .join()
        .expect("export thread should finish cleanly");

    assert!(
        capture_elapsed < Duration::from_millis(1200),
        "new capture should not wait for active export final render, elapsed={capture_elapsed:?}"
    );
    assert_eq!(result.exported_count, 1);
    let late_capture_from_export = result
        .manifest
        .captures
        .iter()
        .find(|capture| capture.capture_id == late_capture.capture.capture_id)
        .expect("late capture should be preserved in export result manifest");
    assert_ne!(
        late_capture_from_export.render_status, "finalReady",
        "same export run must not include captures created after export started"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn post_end_final_render_skips_precreated_session_scoped_final_files() {
    let base_dir = unique_test_root("post-end-skips-precreated-final");
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
    .expect("first preview should complete");
    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second preview should complete");

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let precreated_final_path = paths
        .renders_finals_dir
        .join(format!("{}.jpg", first_capture.capture.capture_id));
    fs::create_dir_all(&paths.renders_finals_dir).expect("finals directory should exist");
    write_decodable_test_jpeg(&precreated_final_path);

    let mut manifest = read_manifest(&base_dir, &session.session_id);
    let precreated_capture = manifest
        .captures
        .iter_mut()
        .find(|candidate| candidate.capture_id == first_capture.capture.capture_id)
        .expect("first capture should exist");
    precreated_capture.render_status = "finalReady".into();
    precreated_capture.post_end_state = "activeSession".into();
    precreated_capture.final_asset.asset_path =
        Some(precreated_final_path.to_string_lossy().into_owned());
    precreated_capture.final_asset.ready_at_ms = Some(precreated_capture.raw.persisted_at_ms + 100);
    fs::write(
        &paths.manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");

    let alternate_final_path = paths
        .renders_finals_dir
        .join(format!("{}_01.jpg", first_capture.capture.capture_id));

    complete_final_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("post-end final render should skip the precreated final");
    assert!(
        !alternate_final_path.exists(),
        "skip path must not ask darktable to render a duplicate final"
    );

    complete_final_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("post-end final render should create the missing final");
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
    .expect("post-end readiness should resolve");

    assert_eq!(readiness.reason_code, "completed");
    let manifest = read_manifest(&base_dir, &session.session_id);
    let first_saved = manifest
        .captures
        .iter()
        .find(|candidate| candidate.capture_id == first_capture.capture.capture_id)
        .expect("first capture should remain present");
    let second_saved = manifest
        .captures
        .iter()
        .find(|candidate| candidate.capture_id == second_capture.capture.capture_id)
        .expect("second capture should remain present");
    assert_eq!(first_saved.render_status, "finalReady");
    assert_eq!(second_saved.render_status, "finalReady");
    assert_valid_jpeg(first_saved.final_asset.asset_path.as_deref().unwrap());
    assert_valid_jpeg(second_saved.final_asset.asset_path.as_deref().unwrap());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn post_end_final_render_reuses_precreated_final_without_touching_file() {
    let base_dir = unique_test_root("post-end-reuses-precreated-final");
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

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("preview should complete");

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let precreated_final_path = paths
        .renders_finals_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    fs::create_dir_all(&paths.renders_finals_dir).expect("finals directory should exist");
    write_decodable_test_jpeg(&precreated_final_path);
    let precreated_bytes =
        fs::read(&precreated_final_path).expect("precreated final should be readable");

    let mut manifest = read_manifest(&base_dir, &session.session_id);
    let precreated_capture = manifest
        .captures
        .iter_mut()
        .find(|candidate| candidate.capture_id == capture.capture.capture_id)
        .expect("capture should exist");
    precreated_capture.render_status = "finalReady".into();
    precreated_capture.post_end_state = "activeSession".into();
    precreated_capture.final_asset.asset_path =
        Some(precreated_final_path.to_string_lossy().into_owned());
    precreated_capture.final_asset.ready_at_ms = Some(precreated_capture.raw.persisted_at_ms + 100);
    fs::write(
        &paths.manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");

    complete_final_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("post-end final render should reuse the precreated final");

    let saved_bytes = fs::read(&precreated_final_path).expect("final should remain readable");
    assert_eq!(
        saved_bytes, precreated_bytes,
        "post-end final render must not rewrite a session-scoped final that is already ready"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_does_not_promote_operation_derived_speculative_preview() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("reject-operation-derived-speculative-preview");
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
        "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=120;detail=widthCap=256;heightCap=256;hq=false;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/preview-source.jpg output=C:/preview.jpg profile=operation-derived;status=0",
    )
    .expect("speculative render detail should be writable");

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("raw original preview close should replace non-truthful speculative output");

    assert_eq!(completed_capture.render_status, "previewReady");

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(
        timing_events.contains("sourceAsset=raw-original"),
        "operation-derived speculative output should not own previewReady truth: {timing_events}"
    );
    assert!(
        !timing_events.contains("profile=operation-derived"),
        "operation-derived speculative output must remain comparison-only: {timing_events}"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_does_not_promote_host_owned_handoff_without_original_full_preset_truth_profile(
) {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("reject-host-owned-without-full-preset-profile");
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
        "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=120;detail=widthCap=256;heightCap=256;hq=false;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/preview-source.jpg output=C:/preview.jpg;status=0",
    )
    .expect("speculative render detail should be writable");

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("raw original preview close should replace incomplete host-owned handoff");

    assert_eq!(completed_capture.render_status, "previewReady");

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(
        timing_events.contains("sourceAsset=raw-original"),
        "host-owned handoff without original/full-preset truth profile should not own previewReady truth: {timing_events}"
    );
    assert!(
        !timing_events.contains("binary=fast-preview-handoff"),
        "incomplete host-owned handoff must remain comparison-only: {timing_events}"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_promotes_host_owned_original_full_preset_speculative_preview() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("promote-original-full-preset-speculative-preview");
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
        "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=120;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/capture.CR2 output=C:/preview.jpg;status=0",
    )
    .expect("speculative render detail should be writable");

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("host-owned original/full-preset speculative output should close preview");

    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("truthProfile=original-full-preset"));
    assert!(timing_events.contains("inputSourceAsset=raw-original"));
    assert!(timing_events.contains("sourceAsset=preset-applied-preview"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_does_not_promote_unverified_native_raw_handoff() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("raw-original-host-owned-handoff-before-darktable");
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

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_decodable_test_jpeg(&raw_path);

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
            None,
            None,
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
    let paths = SessionPaths::new(&base_dir, &session.session_id);

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("unverified native raw-original handoff should fall back to darktable truth");

    assert_eq!(completed_capture.render_status, "previewReady");
    assert_eq!(
        completed_capture.preview.kind.as_deref(),
        Some("raw-original")
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(!timing_events.contains("truthProfile=original-full-preset"));
    assert!(
        timing_events.contains("source=env-override"),
        "darktable fallback should own the close while native raw handoff is comparison-only: {timing_events}"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_persist_starts_comparison_only_host_owned_raw_original_handoff_before_preview_completion(
) {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("persist-starts-raw-original-handoff");
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

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_decodable_test_jpeg(&raw_path);

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
            None,
            None,
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

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));
    let render_detail = fs::read_to_string(&speculative_detail_path)
        .expect("raw-original handoff detail should exist before preview completion");

    assert!(render_detail.contains("binary=fast-preview-handoff"));
    assert!(render_detail.contains("inputSourceAsset=raw-original"));
    assert!(render_detail.contains("sourceAsset=preset-applied-preview"));
    assert!(render_detail.contains("truthProfile=host-owned-native-preview-comparison"));
    assert!(render_detail.contains("truthBlocker=full-preset-parity-unverified"));
    assert!(!render_detail.contains("truthProfile=original-full-preset"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_waits_for_speculative_detail_before_promoting_output() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("wait-for-speculative-detail");
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
    let speculative_lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        result.capture.capture_id, result.capture.request_id
    ));

    fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
    fs::write(&speculative_lock_path, &result.capture.request_id)
        .expect("speculative lock should be writable");

    let delayed_output_path = speculative_output_path.clone();
    let delayed_detail_path = speculative_detail_path.clone();
    let delayed_lock_path = speculative_lock_path.clone();
    let delayed_writer = thread::spawn(move || {
        write_test_jpeg(&delayed_output_path);
        thread::sleep(Duration::from_millis(400));
        fs::write(
            &delayed_detail_path,
            "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=120;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/capture.CR2 output=C:/preview.jpg;status=0",
        )
        .expect("speculative render detail should be writable");
        fs::remove_file(&delayed_lock_path).expect("speculative lock should be removable");
    });

    let initial_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("speculative output should wait for detail before promotion");

    delayed_writer
        .join()
        .expect("delayed speculative writer should complete");

    assert_eq!(initial_capture.render_status, "previewReady");
    assert_eq!(
        initial_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        initial_capture.preview.kind.as_deref(),
        Some("preset-applied-preview")
    );
    assert_eq!(
        initial_capture.preview.ready_at_ms,
        initial_capture.timing.xmp_preview_ready_at_ms
    );

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    assert!(timing_events.contains("event=preview-render-ready"));
    assert!(timing_events.contains("publishedVersion=2026.03.20"));
    assert!(!timing_events.contains("publishedVersion=unknown"));
    assert!(!timing_events.contains("event=preview-render-failed"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn synchronous_raw_original_handoff_records_preview_render_start_before_ready() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("sync-handoff-render-start");
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
            .join(format!("{capture_id}.CR2"));

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
    .expect("capture should save");
    helper_thread
        .join()
        .expect("helper capture thread should complete");
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let speculative_lock_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.lock",
        result.capture.capture_id, result.capture.request_id
    ));
    let speculative_output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        result.capture.capture_id
    ));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        result.capture.capture_id, result.capture.request_id
    ));
    let _ = fs::remove_file(&speculative_lock_path);
    let _ = fs::remove_file(&speculative_output_path);
    let _ = fs::remove_file(&speculative_detail_path);

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("synchronous raw-original handoff should complete preview");
    assert_eq!(completed_capture.render_status, "previewReady");

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing events should be readable");
    let render_start = format!(
        "capture={}\trequest={}\tevent=preview-render-start",
        result.capture.capture_id, result.capture.request_id
    );
    let render_ready = format!(
        "capture={}\trequest={}\tevent=preview-render-ready",
        result.capture.capture_id, result.capture.request_id
    );
    let render_start_count = timing_events.matches(&render_start).count();
    assert!(
        render_start_count >= 2,
        "synchronous raw-original handoff should record its own render-start instead of reusing an abandoned background start: {timing_events}"
    );
    assert!(timing_events.contains(&render_ready));

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
            "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=1500;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/capture.CR2 output=C:/preview.jpg;status=0",
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
    assert!(timing_events.contains("sourceAsset=preset-applied-preview"));
    assert!(timing_events.contains("inputSourceAsset=raw-original"));
    assert!(
        !timing_events.contains("inputSourceAsset=fast-preview-raster"),
        "raw fallback should not win when speculative close arrives inside the healthy wait window"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_still_avoids_a_duplicate_render_while_speculative_close_is_active() {
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
            "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=4300;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/capture.CR2 output=C:/preview.jpg;status=0",
        )
        .expect("speculative detail should be writable");
        fs::remove_file(&delayed_lock_path).expect("speculative lock should be removable");
    });

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("an active speculative close should not trigger a duplicate preview render");

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
    assert!(timing_events.contains("sourceAsset=preset-applied-preview"));
    assert!(timing_events.contains("inputSourceAsset=raw-original"));
    assert!(
        !timing_events.contains("inputSourceAsset=fast-preview-raster"),
        "duplicate raw fallback should stay out of the way while the same capture close is still active"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn complete_preview_render_keeps_waiting_for_a_slow_speculative_close() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("wait-for-slow-speculative-close");
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
        thread::sleep(Duration::from_millis(8000));
        write_test_jpeg(&delayed_output_path);
        fs::write(
            &delayed_detail_path,
            "presetId=preset_soft-glow;publishedVersion=2026.03.20;binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs=8000;detail=widthCap=display;heightCap=display;hq=false;inputSourceAsset=raw-original;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied;truthProfile=original-full-preset;engineBinary=host-owned-native-preview;engineSource=host-owned-native;args=source=C:/capture.CR2 output=C:/preview.jpg;status=0",
        )
        .expect("speculative detail should be writable");
        fs::remove_file(&delayed_lock_path).expect("speculative lock should be removable");
    });

    let completed_capture =
        complete_preview_render_in_dir(&base_dir, &session.session_id, &result.capture.capture_id)
            .expect("a slow speculative close should still win without a duplicate render");

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
        "even a slow speculative close should stay single-lane until it settles"
    );
    assert!(timing_events.contains("sourceAsset=preset-applied-preview"));
    assert!(timing_events.contains("inputSourceAsset=raw-original"));
    assert!(
        !timing_events.contains("inputSourceAsset=fast-preview-raster"),
        "direct raw refinement should not compete with an in-flight slow speculative close"
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
fn helper_fast_preview_wait_does_not_hold_the_capture_pipeline_lock() {
    let _guard = SPECULATIVE_PREVIEW_TEST_MUTEX
        .lock()
        .expect("speculative preview test mutex should lock");
    let base_dir = unique_test_root("helper-fast-preview-wait-nonblocking");
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
    let speculative_output_path = paths.renders_previews_dir.join(format!(
        "{}.preview-speculative.jpg",
        first_capture.capture.capture_id
    ));
    let speculative_detail_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative.detail",
        first_capture.capture.capture_id, first_capture.capture.request_id
    ));
    let speculative_source_path = paths.renders_previews_dir.join(format!(
        "{}.{}.preview-speculative-source.jpg",
        first_capture.capture.capture_id, first_capture.capture.request_id
    ));
    let _ = fs::remove_file(&speculative_lock_path);
    let _ = fs::remove_file(&speculative_output_path);
    let _ = fs::remove_file(&speculative_detail_path);
    let _ = fs::remove_file(&speculative_source_path);

    let preview_base_dir = base_dir.clone();
    let preview_session_id = session.session_id.clone();
    let preview_capture_id = first_capture.capture.capture_id.clone();
    let preview_thread = thread::spawn(move || {
        complete_preview_render_in_dir(&preview_base_dir, &preview_session_id, &preview_capture_id)
    });

    thread::sleep(Duration::from_millis(120));

    let lock_contender_started = Instant::now();
    mark_preview_render_failed_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("lock contender should complete while helper wait is pending");
    let lock_contender_elapsed = lock_contender_started.elapsed();

    assert!(
        lock_contender_elapsed < Duration::from_millis(800),
        "helper fast-preview wait should not hold the capture pipeline lock: {lock_contender_elapsed:?}"
    );

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
    let timing_log_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("timing-events.log");
    let timing_events =
        fs::read_to_string(timing_log_path).expect("timing-events.log should be readable");
    assert!(
        timing_events.contains("event=fast-preview-visible"),
        "recovered same-capture previews should restore the missing first-visible seam in timing-events.log"
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
fn request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight() {
    let base_dir = unique_test_root("capture-request-does-not-reprime-preview-warmup");
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

    let warmup_source_path = base_dir
        .join(".boothy-darktable")
        .join("preview")
        .join("warmup")
        .join("preview-renderer-warmup-source.jpg");
    let _ = fs::remove_file(&warmup_source_path);

    let result = request_capture_with_helper_success(&base_dir, &session.session_id);

    assert_eq!(result.status, "capture-saved");
    assert!(
        !warmup_source_path.is_file(),
        "capture request should not start a competing warm-up while the same capture still owns the truthful close path"
    );

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
    assert_eq!(
        result.capture.preview.kind.as_deref(),
        Some("legacy-canonical-scan")
    );
    assert_eq!(result.capture.preview.ready_at_ms, None);
    assert_eq!(result.capture.timing.xmp_preview_ready_at_ms, None);
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
fn readiness_repairs_a_legacy_text_jpg_preview_by_rerendering_it() {
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
    .expect("invalid legacy preview should be repaired");

    let repaired_preview_path = readiness
        .latest_capture
        .as_ref()
        .and_then(|capture| capture.preview.asset_path.as_deref())
        .expect("repaired preview path should exist");
    assert_eq!(
        repaired_preview_path,
        preview_path.to_string_lossy().as_ref()
    );
    assert_valid_jpeg(repaired_preview_path);

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
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

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
fn capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs() {
    let base_dir = unique_test_root("capture-trigger-internal-error-retryable");
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
    update_stage(&base_dir, &session.session_id, "capture-ready");
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
              "detailCode": "capture-trigger-failed",
              "message": "셔터 명령을 보낼 수 없었어요: 0x00000002",
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
    .expect_err("internal trigger failure should return retryable readiness");

    helper_thread
        .join()
        .expect("helper retryable internal error thread should complete");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .expect("retryable trigger failure should include readiness")
            .reason_code,
        "capture-retry-required",
    );
    let manifest = read_manifest(&base_dir, &session.session_id);
    assert!(manifest.captures.is_empty());
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

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
fn capture_flow_auto_retries_the_first_internal_trigger_failure_once() {
    let base_dir = unique_test_root("capture-trigger-internal-error-auto-retry");
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
        let first_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": first_request.session_id,
              "requestId": first_request.request_id,
            }),
        );
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
              "type": "helper-error",
              "sessionId": first_request.session_id,
              "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
              "detailCode": "capture-trigger-failed",
              "message": "셔터 명령을 보낼 수 없었어요: 0x00000002",
            }),
        );
        write_helper_status_with_sequence_and_detail(
            &helper_base_dir,
            &helper_session_id,
            2,
            "connecting",
            "connecting",
            &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
            Some("session-opening"),
        );
        thread::sleep(Duration::from_millis(1100));
        write_helper_status_with_sequence_and_detail(
            &helper_base_dir,
            &helper_session_id,
            3,
            "ready",
            "healthy",
            &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
            Some("camera-ready"),
        );

        let second_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 1);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_auto_retry.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_test_jpeg(&raw_path);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": second_request.session_id,
              "requestId": second_request.request_id,
            }),
        );
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &second_request,
            "capture_auto_retry",
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
    .expect("internal trigger failure should be auto-retried once");

    helper_thread
        .join()
        .expect("helper auto retry thread should complete");

    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.readiness.reason_code, "preview-waiting");
    assert_eq!(
        read_manifest(&base_dir, &session.session_id).captures.len(),
        1
    );
    let requests = read_capture_request_messages(&base_dir, &session.session_id)
        .expect("capture requests should be readable");
    assert_eq!(requests.len(), 2);

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=request-capture-auto-retry"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_auto_retries_the_first_internal_trigger_failure_twice_before_escalating() {
    let base_dir = unique_test_root("capture-trigger-internal-error-auto-retry-twice");
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
        for request_index in 0..2 {
            let request = wait_for_latest_capture_request(
                &helper_base_dir,
                &helper_session_id,
                request_index,
            );
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
                  "detailCode": "capture-trigger-failed",
                  "message": "셔터 명령을 보낼 수 없었어요: 0x00000002",
                }),
            );
            let connecting_sequence = 2 + (request_index as u64 * 2);
            write_helper_status_with_sequence_and_detail(
                &helper_base_dir,
                &helper_session_id,
                connecting_sequence,
                "connecting",
                "connecting",
                &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
                Some("session-opening"),
            );
            thread::sleep(Duration::from_millis(1100));
            write_helper_status_with_sequence_and_detail(
                &helper_base_dir,
                &helper_session_id,
                connecting_sequence + 1,
                "ready",
                "healthy",
                &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
                Some("camera-ready"),
            );
        }

        let third_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 2);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_auto_retry_twice.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_test_jpeg(&raw_path);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": third_request.session_id,
              "requestId": third_request.request_id,
            }),
        );
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &third_request,
            "capture_auto_retry_twice",
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
    .expect("internal trigger failure should be auto-retried twice before failing");

    helper_thread
        .join()
        .expect("helper double auto retry thread should complete");

    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.readiness.reason_code, "preview-waiting");
    assert_eq!(
        read_manifest(&base_dir, &session.session_id).captures.len(),
        1
    );
    let requests = read_capture_request_messages(&base_dir, &session.session_id)
        .expect("capture requests should be readable");
    assert_eq!(requests.len(), 3);

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=request-capture-auto-retry"));
    assert!(timing_events.contains("attempt=1"));
    assert!(timing_events.contains("attempt=2"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_waits_for_helper_ready_to_stabilize_before_internal_auto_retry() {
    let base_dir = unique_test_root("capture-trigger-internal-error-auto-retry-stabilization");
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
        let first_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": first_request.session_id,
              "requestId": first_request.request_id,
            }),
        );
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
              "type": "helper-error",
              "sessionId": first_request.session_id,
              "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
              "detailCode": "capture-trigger-failed",
              "message": "셔터 명령을 보낼 수 없었어요: 0x00000002",
            }),
        );
        write_helper_status_with_sequence_and_detail(
            &helper_base_dir,
            &helper_session_id,
            2,
            "connecting",
            "connecting",
            &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
            Some("session-opening"),
        );
        thread::sleep(Duration::from_millis(1100));
        write_helper_status_with_sequence_and_detail(
            &helper_base_dir,
            &helper_session_id,
            3,
            "ready",
            "healthy",
            &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
            Some("camera-ready"),
        );
        let ready_written_at = Instant::now();

        let second_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 1);
        let elapsed_since_ready = ready_written_at.elapsed();
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_auto_retry_stabilized.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_test_jpeg(&raw_path);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": second_request.session_id,
              "requestId": second_request.request_id,
            }),
        );
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &second_request,
            "capture_auto_retry_stabilized",
            &raw_path,
            None,
            None,
        );

        elapsed_since_ready
    });

    let result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect("internal trigger failure should retry after helper readiness stabilizes");

    let elapsed_since_ready = helper_thread
        .join()
        .expect("helper retry stabilization thread should complete");

    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.readiness.reason_code, "preview-waiting");
    assert!(
        elapsed_since_ready >= Duration::from_millis(4500),
        "retry should wait for helper readiness to stabilize, elapsed={elapsed_since_ready:?}"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_ignores_stale_ready_status_before_reconnect_ready_stabilizes() {
    let base_dir = unique_test_root("capture-trigger-internal-error-auto-retry-fresh-ready-only");
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
        let first_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": first_request.session_id,
              "requestId": first_request.request_id,
            }),
        );
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
              "type": "helper-error",
              "sessionId": first_request.session_id,
              "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
              "detailCode": "capture-trigger-failed",
              "message": "셔터 명령을 보낼 수 없었어요: 0x00000002",
            }),
        );

        thread::sleep(Duration::from_millis(1800));
        write_helper_status_with_sequence_and_detail(
            &helper_base_dir,
            &helper_session_id,
            2,
            "connecting",
            "connecting",
            &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
            Some("session-opening"),
        );

        thread::sleep(Duration::from_millis(1100));
        write_helper_status_with_sequence_and_detail(
            &helper_base_dir,
            &helper_session_id,
            3,
            "ready",
            "healthy",
            &current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
            Some("camera-ready"),
        );
        let fresh_ready_written_at = Instant::now();

        let second_request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 1);
        let elapsed_since_fresh_ready = fresh_ready_written_at.elapsed();
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_auto_retry_fresh_ready_only.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_test_jpeg(&raw_path);
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": second_request.session_id,
              "requestId": second_request.request_id,
            }),
        );
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &second_request,
            "capture_auto_retry_fresh_ready_only",
            &raw_path,
            None,
            None,
        );

        elapsed_since_fresh_ready
    });

    let result = request_capture_in_dir(
        &base_dir,
        CaptureRequestInputDto {
            session_id: session.session_id.clone(),
            request_id: None,
        },
    )
    .expect("internal trigger failure should wait for a fresh reconnect-ready status");

    let elapsed_since_fresh_ready = helper_thread
        .join()
        .expect("helper fresh ready retry thread should complete");

    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.readiness.reason_code, "preview-waiting");
    assert!(
        elapsed_since_fresh_ready >= Duration::from_millis(4500),
        "retry should wait for a fresh reconnect-ready status to stabilize, elapsed={elapsed_since_fresh_ready:?}"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_flow_restarts_helper_once_when_the_request_was_never_consumed() {
    shutdown_helper_process();
    let _helper_shutdown_guard = HelperSupervisorShutdownGuard;
    let base_dir = unique_test_root("capture-request-unconsumed-helper-stall");
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
    write_capture_timeout_override(&base_dir, 100);

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session.session_id.clone();
    let helper_thread = thread::spawn(move || {
        let request = wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, 0);
        let stale_observed_at = current_timestamp(
            SystemTime::now()
                .checked_sub(Duration::from_secs(10))
                .expect("stale helper timestamp should compute"),
        )
        .expect("stale helper timestamp should serialize");
        write_helper_status_with_detail(
            &helper_base_dir,
            &helper_session_id,
            "ready",
            "healthy",
            &stale_observed_at,
            Some("camera-ready"),
        );

        thread::sleep(Duration::from_millis(250));
        write_ready_helper_status(&helper_base_dir, &helper_session_id);

        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join("capture_request_unconsumed_restart.jpg");
        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        write_test_jpeg(&raw_path);

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
        append_file_arrived_event(
            &helper_base_dir,
            &helper_session_id,
            &request,
            "capture_request_unconsumed_restart",
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
    .expect("an unconsumed request stall should recover after a helper restart");

    helper_thread
        .join()
        .expect("helper request stall recovery thread should complete");

    assert_eq!(result.status, "capture-saved");
    assert_eq!(result.readiness.reason_code, "preview-waiting");

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing events should be readable");
    assert!(timing_events.contains("event=request-capture-helper-restart"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_phone_required_after_retryable_focus_failure_recovers() {
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
    .expect("retryable focus failure should recover from phone-required");

    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers() {
    let base_dir = unique_test_root("retryable-trigger-internal-error-unlocks");
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
          "message": "셔터 명령을 보낼 수 없었어요: 0x00000002",
        }),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("retryable internal trigger failure should recover from phone-required");

    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_phone_required_after_capture_download_timeout_recovers() {
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
    .expect("capture-download-timeout should recover from phone-required once helper is ready");

    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_phone_required_without_saved_capture_once_helper_is_ready_again() {
    let base_dir = unique_test_root("capture-timeout-without-saved-capture-unlocks");
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
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_accepted_only_timeout",
    );
    write_capture_round_trip_failure_evidence(
        &base_dir,
        &session.session_id,
        "capture-timeout",
        Some("request_accepted_only_timeout"),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("accepted-only timeout should recover from phone-required once helper is ready");

    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_phone_required_without_saved_capture_even_with_unrelated_helper_error() {
    let base_dir = unique_test_root("capture-timeout-with-unrelated-helper-error-unlocks");
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
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_accepted_only_timeout",
    );
    write_capture_round_trip_failure_evidence(
        &base_dir,
        &session.session_id,
        "capture-timeout",
        Some("request_accepted_only_timeout"),
    );
    append_helper_event(
        &base_dir,
        &session.session_id,
        serde_json::json!({
          "schemaVersion": CANON_HELPER_ERROR_SCHEMA_VERSION,
          "type": "helper-error",
          "sessionId": session.session_id,
          "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
          "detailCode": "camera-disconnected",
          "message": "카메라 연결이 잠시 끊겼어요.",
        }),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("accepted-only timeout should still recover when unrelated helper errors exist");

    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "capture-ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_releases_phone_required_without_saved_capture_after_timing_sync_updates_manifest() {
    let base_dir =
        unique_test_root("capture-timeout-without-saved-capture-unlocks-after-timing-sync");
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
    update_timing(
        &base_dir,
        &session.session_id,
        &current_timestamp(SystemTime::now() - Duration::from_secs(5))
            .expect("warning timestamp should serialize"),
        &current_timestamp(SystemTime::now() + Duration::from_secs(300))
            .expect("end timestamp should serialize"),
        "active",
    );
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_accepted_only_timeout",
    );
    write_capture_round_trip_failure_evidence(
        &base_dir,
        &session.session_id,
        "capture-timeout",
        Some("request_accepted_only_timeout"),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("accepted-only timeout should still recover after timing sync updates the manifest");

    assert_eq!(readiness.reason_code, "warning");
    assert!(readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "capture-ready");
    assert_eq!(
        manifest.timing.as_ref().map(|timing| timing.phase.as_str()),
        Some("warning"),
        "the same readiness poll should still persist the timing transition"
    );
    assert!(
        manifest
            .timing
            .as_ref()
            .and_then(|timing| timing.warning_triggered_at.as_ref())
            .is_some(),
        "warning timing evidence should be recorded during the recovery poll"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_without_saved_capture_when_timeout_evidence_is_missing() {
    let base_dir = unique_test_root("capture-timeout-without-saved-capture-stays-blocked");
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
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_unrelated_failure",
    );
    write_capture_round_trip_failure_evidence(
        &base_dir,
        &session.session_id,
        "capture-file-missing",
        Some("request_unrelated_failure"),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should stay blocked without timeout-specific evidence");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_when_timeout_evidence_file_is_corrupted() {
    let base_dir = unique_test_root("capture-timeout-with-corrupted-evidence-recovers");
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
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_accepted_only_timeout",
    );
    write_corrupted_capture_round_trip_failure_evidence(&base_dir, &session.session_id);
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("corrupted timeout evidence should keep the session blocked");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_when_corrupted_timeout_evidence_is_stale() {
    let base_dir = unique_test_root("capture-timeout-with-stale-corrupted-evidence-stays-blocked");
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
    write_corrupted_capture_round_trip_failure_evidence(&base_dir, &session.session_id);
    thread::sleep(Duration::from_secs(3));
    update_stage(&base_dir, &session.session_id, "phone-required");
    update_manifest_updated_at(
        &base_dir,
        &session.session_id,
        &current_timestamp(SystemTime::now()).expect("manifest timestamp should serialize"),
    );
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_accepted_only_timeout",
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("stale corrupted timeout evidence should not unlock the session");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_when_timeout_evidence_request_id_does_not_match_processed_request(
) {
    let base_dir = unique_test_root("capture-timeout-with-mismatched-request-id-stays-blocked");
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
    append_processed_capture_request_id(
        &base_dir,
        &session.session_id,
        "request_processed_but_different",
    );
    write_capture_round_trip_failure_evidence(
        &base_dir,
        &session.session_id,
        "capture-timeout",
        Some("request_timeout_original"),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("mismatched timeout evidence should not unlock the session");

    assert_eq!(readiness.reason_code, "phone-required");
    assert!(!readiness.can_capture);

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_keeps_phone_required_when_latest_helper_error_predates_current_route_hold() {
    let base_dir = unique_test_root("stale-helper-error-does-not-recover-phone-required");
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
          "observedAt": current_timestamp(SystemTime::now() - Duration::from_secs(10)).expect("helper timestamp should serialize"),
          "detailCode": "capture-trigger-failed",
          "message": "셔터 명령을 보낼 수 없었어요: 0x00008d01",
        }),
    );
    update_manifest_updated_at(
        &base_dir,
        &session.session_id,
        &current_timestamp(SystemTime::now()).expect("manifest timestamp should serialize"),
    );
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("stale helper error should not reopen the session");

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
            .expect("persist failure should include readiness")
            .reason_code,
        "phone-required",
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert!(manifest.captures.is_empty());
    assert_eq!(manifest.lifecycle.stage, "preset-selected");

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
fn handoff_ready_completion_without_destination_metadata_falls_back_to_local_deliverable() {
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
    .expect("completion should resolve without invented handoff guidance");

    let post_end = readiness.post_end.expect("completed guidance should exist");
    assert_eq!(post_end.state(), "completed");
    assert_eq!(
        post_end.completion_variant(),
        Some("local-deliverable-ready")
    );
    match post_end {
        SessionPostEnd::Completed(value) => {
            assert!(value.approved_recipient_label.is_none());
            assert!(value.next_location_label.is_none());
            assert_eq!(
                value.primary_action_label,
                "안내가 끝났어요. 천천히 이동해 주세요."
            );
        }
        _ => panic!("expected completed post-end"),
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn invalid_existing_handoff_ready_record_falls_back_to_local_deliverable() {
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
    .expect("completion should avoid invented handoff guidance");

    let post_end = readiness.post_end.expect("completed guidance should exist");
    assert_eq!(
        post_end.completion_variant(),
        Some("local-deliverable-ready")
    );
    match post_end {
        SessionPostEnd::Completed(value) => {
            assert!(value.approved_recipient_label.is_none());
            assert!(value.next_location_label.is_none());
            assert_eq!(
                value.primary_action_label,
                "안내가 끝났어요. 천천히 이동해 주세요."
            );
        }
        _ => panic!("expected completed post-end"),
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn completed_lifecycle_without_post_end_does_not_claim_completion_before_final_truth() {
    let base_dir = unique_test_root("completed-stage-without-post-end");
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
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    let mut manifest = read_manifest(&base_dir, &session.session_id);
    manifest.lifecycle.stage = "completed".into();
    manifest.post_end = None;
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
    .expect("readiness should resolve without false completion");

    assert_eq!(readiness.reason_code, "export-waiting");
    assert_eq!(
        readiness.post_end.as_ref().map(|post_end| post_end.state()),
        Some("export-waiting")
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "export-waiting");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn missing_final_asset_file_stays_export_waiting_until_final_render_finishes() {
    let base_dir = unique_test_root("missing-final-file-recreated");
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

    let stale_final_path = read_manifest(&base_dir, &session.session_id).captures[0]
        .final_asset
        .asset_path
        .clone()
        .expect("test fixture should point at a final asset");
    if std::path::Path::new(&stale_final_path).is_file() {
        fs::remove_file(&stale_final_path).expect("stale final file should be removable");
    }
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
    .expect("readiness should stay waiting without synchronous final render");

    assert_eq!(readiness.reason_code, "export-waiting");
    let final_path = readiness
        .latest_capture
        .as_ref()
        .and_then(|capture| capture.final_asset.asset_path.as_deref())
        .expect("completed readiness should include a final asset path");
    assert!(
        !std::path::Path::new(final_path).is_file(),
        "readiness should not recreate a final file while entering export-waiting"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn foreign_final_asset_path_does_not_complete_post_end_truth() {
    let base_dir = unique_test_root("foreign-final-file-rejected");
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

    let foreign_final_path = base_dir
        .parent()
        .expect("test root should have parent")
        .join(format!("foreign-final-{}.jpg", capture.capture.capture_id));
    write_decodable_test_jpeg(&foreign_final_path);

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    let capture_record = manifest
        .captures
        .iter_mut()
        .find(|candidate| candidate.capture_id == capture.capture.capture_id)
        .expect("capture should exist");
    capture_record.render_status = "finalReady".into();
    capture_record.final_asset.asset_path = Some(foreign_final_path.to_string_lossy().into_owned());
    capture_record.final_asset.ready_at_ms = Some(capture_record.raw.persisted_at_ms + 100);
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");

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
    .expect("readiness should reject foreign final truth");

    assert_eq!(readiness.reason_code, "export-waiting");
    assert_eq!(
        readiness.post_end.as_ref().map(|post_end| post_end.state()),
        Some("export-waiting")
    );

    let _ = fs::remove_file(foreign_final_path);
    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn overlong_handoff_guidance_is_ignored_before_manifest_storage() {
    let base_dir = unique_test_root("handoff-overlong-guidance");
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
            "approvedRecipientLabel": "A".repeat(81),
            "primaryActionLabel": "B".repeat(81),
            "supportActionLabel": "C".repeat(81)
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
    .expect("readiness should ignore contract-invalid handoff guidance");

    assert_eq!(readiness.reason_code, "completed");
    let post_end = readiness.post_end.expect("completed post-end should exist");
    assert_eq!(
        post_end.completion_variant(),
        Some("local-deliverable-ready")
    );
    match post_end {
        SessionPostEnd::Completed(value) => {
            assert!(value.approved_recipient_label.is_none());
            assert!(value.next_location_label.is_none());
            assert!(value.primary_action_label.len() <= 80);
            assert!(value
                .support_action_label
                .as_ref()
                .map(|label| label.len() <= 80)
                .unwrap_or(true));
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
fn ended_preview_ready_capture_stays_export_waiting_until_final_render_completes() {
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
    .expect("post-end readiness should stay waiting while final render is pending");

    assert_eq!(readiness.reason_code, "export-waiting");
    assert_eq!(
        readiness.post_end.as_ref().map(|post_end| post_end.state()),
        Some("export-waiting")
    );

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "export-waiting");
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.post_end_state.as_str()),
        Some("postEndPending")
    );
    assert_eq!(
        manifest
            .captures
            .last()
            .map(|latest_capture| latest_capture.render_status.as_str()),
        Some("previewReady")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn final_ready_capture_with_missing_ready_time_stays_export_waiting() {
    let base_dir = unique_test_root("timing-final-ready-missing-time");
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
    complete_final_render_in_dir(&base_dir, &session.session_id, &capture.capture.capture_id)
        .expect("final should complete");

    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let mut manifest = read_manifest(&base_dir, &session.session_id);
    let saved_capture = manifest
        .captures
        .iter_mut()
        .find(|candidate| candidate.capture_id == capture.capture.capture_id)
        .expect("capture should exist");
    saved_capture.render_status = "finalReady".into();
    saved_capture.final_asset.ready_at_ms = None;
    fs::write(
        &paths.manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");

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
    .expect("post-end readiness should not repair final metadata synchronously");

    assert_eq!(readiness.reason_code, "export-waiting");
    let manifest = read_manifest(&base_dir, &session.session_id);
    let repaired_capture = manifest
        .captures
        .iter()
        .find(|candidate| candidate.capture_id == capture.capture.capture_id)
        .expect("capture should exist");
    assert_eq!(repaired_capture.render_status, "finalReady");
    assert!(repaired_capture.final_asset.ready_at_ms.is_none());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn ended_multi_capture_session_waits_until_every_capture_has_final_truth() {
    let base_dir = unique_test_root("timing-completed-multi-final");
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
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first preview should complete");
    write_ready_helper_status(&base_dir, &session.session_id);

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second preview should complete");

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
    .expect("export-waiting post-end readiness should resolve");

    assert_eq!(readiness.reason_code, "export-waiting");

    let manifest = read_manifest(&base_dir, &session.session_id);
    assert_eq!(manifest.lifecycle.stage, "export-waiting");
    assert_eq!(manifest.captures.len(), 2);

    for capture in &manifest.captures {
        assert_eq!(
            capture.render_status, "previewReady",
            "capture {} should keep waiting until final truth arrives",
            capture.capture_id
        );
        assert!(capture.final_asset.asset_path.is_none());
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn non_latest_final_render_failure_moves_session_to_phone_required() {
    let base_dir = unique_test_root("timing-non-latest-final-failure");
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
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("first preview should complete");
    write_ready_helper_status(&base_dir, &session.session_id);

    let second_capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    complete_preview_render_in_dir(
        &base_dir,
        &session.session_id,
        &second_capture.capture.capture_id,
    )
    .expect("second preview should complete");

    update_timing(
        &base_dir,
        &session.session_id,
        &timestamp_offset(-60),
        &timestamp_offset(-10),
        "active",
    );

    mark_final_render_failed_in_dir(
        &base_dir,
        &session.session_id,
        &first_capture.capture.capture_id,
    )
    .expect("non-latest final render failure should be recorded");

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("phone-required post-end readiness should resolve");

    assert_eq!(readiness.reason_code, "phone-required");

    let manifest = read_manifest(&base_dir, &session.session_id);
    let failed_capture = manifest
        .captures
        .iter()
        .find(|capture| capture.capture_id == first_capture.capture.capture_id)
        .expect("first capture should remain in manifest");
    assert_eq!(failed_capture.render_status, "renderFailed");
    assert_eq!(manifest.lifecycle.stage, "phone-required");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn readiness_recovers_after_a_legacy_default_bundle_is_upgraded_for_rendering() {
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

    let _ = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("default bundle upgrade should trigger preview recovery work");
    write_ready_helper_status(&base_dir, &session.session_id);

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("render failure should recover after helper freshness is renewed");

    assert_eq!(readiness.reason_code, "ready");
    assert!(readiness.can_capture);
    assert_valid_jpeg(
        readiness
            .latest_capture
            .as_ref()
            .and_then(|capture| capture.preview.asset_path.as_deref())
            .expect("recovered preview should exist"),
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
    if let Some(final_path) = capture.final_asset.asset_path.as_deref() {
        write_decodable_test_jpeg(std::path::Path::new(final_path));
    }

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

fn update_manifest_updated_at(base_dir: &PathBuf, session_id: &str, updated_at: &str) {
    let manifest_path = SessionPaths::new(base_dir, session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    manifest.updated_at = updated_at.into();

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
    for _ in 0..1000 {
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
    for _ in 0..1000 {
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

fn append_processed_capture_request_id(base_dir: &PathBuf, session_id: &str, request_id: &str) {
    let processed_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_PROCESSED_REQUEST_IDS_FILE_NAME);
    fs::create_dir_all(
        processed_path
            .parent()
            .expect("processed request path should have a diagnostics directory"),
    )
    .expect("processed request directory should exist");

    let existing = fs::read_to_string(&processed_path).unwrap_or_default();
    let next_contents = if existing.trim().is_empty() {
        format!("{request_id}\n")
    } else {
        format!("{existing}{request_id}\n")
    };

    fs::write(processed_path, next_contents).expect("processed request log should be writable");
}

fn write_capture_round_trip_failure_evidence(
    base_dir: &PathBuf,
    session_id: &str,
    reason_code: &str,
    request_id: Option<&str>,
) {
    let paths = SessionPaths::new(base_dir, session_id);
    fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics dir should exist");
    let manifest = read_manifest(base_dir, session_id);
    let evidence = serde_json::json!({
      "occurredAt": manifest.updated_at,
      "reasonCode": reason_code,
      "requestId": request_id,
    });
    fs::write(
        paths
            .diagnostics_dir
            .join("latest-capture-round-trip-failure.json"),
        serde_json::to_vec_pretty(&evidence).expect("failure evidence should serialize"),
    )
    .expect("failure evidence should be writable");
}

fn write_corrupted_capture_round_trip_failure_evidence(base_dir: &PathBuf, session_id: &str) {
    let paths = SessionPaths::new(base_dir, session_id);
    fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics dir should exist");
    fs::write(
        paths
            .diagnostics_dir
            .join("latest-capture-round-trip-failure.json"),
        b"{ not-valid-json",
    )
    .expect("corrupted failure evidence should be writable");
}

fn wait_for_timing_event(base_dir: &PathBuf, session_id: &str, pattern: &str) {
    let timing_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("timing-events.log");

    for _ in 0..100 {
        let contents = fs::read_to_string(&timing_path).unwrap_or_default();
        if contents.contains(pattern) {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("timing event not observed: {pattern}");
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
    write_helper_status_with_sequence_and_detail(
        base_dir,
        session_id,
        1,
        camera_state,
        helper_state,
        observed_at,
        None,
    );
}

fn write_helper_status_with_detail(
    base_dir: &PathBuf,
    session_id: &str,
    camera_state: &str,
    helper_state: &str,
    observed_at: &str,
    detail_code: Option<&str>,
) {
    write_helper_status_with_sequence_and_detail(
        base_dir,
        session_id,
        1,
        camera_state,
        helper_state,
        observed_at,
        detail_code,
    );
}

fn write_helper_status_with_sequence_and_detail(
    base_dir: &PathBuf,
    session_id: &str,
    sequence: u64,
    camera_state: &str,
    helper_state: &str,
    observed_at: &str,
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
    let mut payload = serde_json::json!({
      "schemaVersion": "canon-helper-status/v1",
      "sessionId": session_id,
      "sequence": sequence,
      "observedAt": observed_at,
      "cameraState": camera_state,
      "helperState": helper_state
    });
    if let Some(detail_code) = detail_code {
        payload["detailCode"] = serde_json::Value::String(detail_code.into());
    }
    fs::write(
        status_path,
        serde_json::to_vec_pretty(&payload).expect("helper status should serialize"),
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
