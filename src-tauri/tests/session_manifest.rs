use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    contracts::dto::{LoadPresetCatalogInputDto, PresetSelectionInputDto, SessionStartInputDto},
    preset::preset_catalog::{load_preset_catalog_in_dir, resolve_published_preset_catalog_dir},
    session::{
        session_manifest::{
            build_session_manifest_at, current_timestamp, normalize_legacy_manifest,
            rfc3339_to_unix_seconds, SessionManifest, SESSION_MANIFEST_SCHEMA_VERSION,
        },
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-{test_name}-{stamp}"))
}

#[test]
fn creates_session_root_and_manifest_for_valid_input() {
    let base_dir = unique_test_root("valid-start");
    let result = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: " Kim  Noah ".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("valid input should create a session");

    let paths = SessionPaths::new(&base_dir, &result.session_id);
    let manifest_bytes =
        fs::read_to_string(&paths.manifest_path).expect("manifest should be written");
    let manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    assert_eq!(result.booth_alias, "Kim Noah 4821");
    assert_eq!(manifest.schema_version, SESSION_MANIFEST_SCHEMA_VERSION);
    assert_eq!(manifest.session_id, result.session_id);
    assert_eq!(manifest.booth_alias, result.booth_alias);
    assert_eq!(manifest.customer.name, "Kim Noah");
    assert_eq!(manifest.customer.phone_last_four, "4821");
    assert_eq!(manifest.lifecycle.status, "active");
    assert_eq!(manifest.lifecycle.stage, "session-started");
    assert!(manifest.active_preset_id.is_none());
    assert!(manifest.captures.is_empty());
    assert!(manifest.post_end.is_none());
    assert!(paths.captures_originals_dir.exists());
    assert!(paths.renders_previews_dir.exists());
    assert!(paths.renders_finals_dir.exists());
    assert!(paths.handoff_dir.exists());
    assert!(paths.diagnostics_dir.exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rejects_invalid_input_without_creating_durable_session_artifacts() {
    let base_dir = unique_test_root("invalid-start");
    let result = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "   ".into(),
            phone_last_four: "12a4".into(),
        },
    );

    let error = result.expect_err("invalid input should fail");

    assert_eq!(error.code, "validation-error");
    assert!(error.field_errors.is_some());
    assert!(!base_dir.join("sessions").exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rejects_phone_suffix_with_surrounding_whitespace() {
    let base_dir = unique_test_root("whitespace-phone");
    let result = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: " 4821 ".into(),
        },
    );

    let error = result.expect_err("suffix with whitespace should fail");

    assert_eq!(error.code, "validation-error");
    assert_eq!(
        error.field_errors.and_then(|fields| fields.phone_last_four),
        Some("휴대전화 뒤 4자리는 숫자 4자리여야 해요.".into()),
    );
    assert!(!base_dir.join("sessions").exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn fails_manifest_build_when_system_time_precedes_unix_epoch() {
    let before_epoch = UNIX_EPOCH
        .checked_sub(Duration::from_secs(1))
        .expect("a pre-epoch timestamp should be constructible");

    let error = build_session_manifest_at(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
        before_epoch,
    )
    .expect_err("pre-epoch time should fail manifest creation");

    assert_eq!(error.code, "session-persistence-failed");
    assert!(error.message.contains("시스템 시계"));
}

#[test]
fn parses_rfc3339_timestamps_with_explicit_utc_offsets() {
    let from_z =
        rfc3339_to_unix_seconds("2026-03-27T19:37:08Z").expect("zulu timestamp should parse");
    let from_offset = rfc3339_to_unix_seconds("2026-03-27T19:37:08.3928625+00:00")
        .expect("utc offset timestamp should parse");

    assert_eq!(from_offset, from_z);
}

#[test]
fn published_preset_catalog_only_returns_booth_safe_published_entries_and_limits_to_six() {
    let base_dir = unique_test_root("preset-catalog");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before loading the preset catalog");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.19",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_mono-pop",
        "2026.03.20",
        "Mono Pop",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_velvet",
        "2026.03.20",
        "Velvet",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_daylight",
        "2026.03.20",
        "Daylight",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_afterglow",
        "2026.03.20",
        "Afterglow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_porcelain",
        "2026.03.20",
        "Porcelain",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_over-limit",
        "2026.03.20",
        "Over Limit",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_draft",
        "2026.03.20",
        "Draft",
        "draft",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_missing-preview",
        "2026.03.20",
        "Missing Preview",
        "published",
        false,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_version-mismatch",
        "2026.03.20",
        "Version Mismatch",
        "published",
        true,
        Some("2026.03.19"),
        None,
    );

    let result = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("published booth-safe catalog should load");

    assert_eq!(result.state, "ready");
    assert_eq!(result.presets.len(), 6);
    assert_eq!(
        result
            .presets
            .iter()
            .filter(|preset| preset.preset_id == "preset_soft-glow")
            .count(),
        1
    );
    assert!(result
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow"
            && preset.published_version == "2026.03.20"));
    assert!(result
        .presets
        .iter()
        .all(|preset| preset.booth_status == "booth-safe"));
    assert!(result
        .presets
        .iter()
        .all(|preset| preset.preview.asset_path.ends_with(".jpg")));
    assert!(!result
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_draft"));
    assert!(!result
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_missing-preview"));
    assert!(!result
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_version-mismatch"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn loading_the_preset_catalog_pins_the_current_catalog_revision_and_snapshot() {
    let base_dir = unique_test_root("catalog-snapshot-pin");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.21",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_mono-pop",
        "2026.03.20",
        "Mono Pop",
        "published",
        true,
        None,
        None,
    );

    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should start before the first catalog load");
    let pinned_catalog = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("first catalog load should pin the current live snapshot");
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest: SessionManifest =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("manifest should exist"))
            .expect("manifest should deserialize");

    assert_eq!(pinned_catalog.presets.len(), 2);
    assert_eq!(manifest.catalog_revision, Some(1));
    assert_eq!(
        manifest
            .catalog_snapshot
            .as_ref()
            .map(|snapshot| snapshot.len()),
        Some(2)
    );
    assert!(manifest
        .catalog_snapshot
        .as_ref()
        .expect("snapshot should be persisted")
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow"
            && preset.published_version == "2026.03.21"));
    assert!(manifest
        .catalog_snapshot
        .as_ref()
        .expect("snapshot should be persisted")
        .iter()
        .any(|preset| preset.preset_id == "preset_mono-pop"
            && preset.published_version == "2026.03.20"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_a_preset_persists_the_binding_in_session_manifest() {
    let base_dir = unique_test_root("preset-selection");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );

    let result = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset selection should be persisted");

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest_bytes = fs::read_to_string(manifest_path).expect("manifest should be readable");
    let manifest: SessionManifest =
        serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

    let active_preset = manifest
        .active_preset
        .expect("selected preset binding should be stored in the manifest");

    assert_eq!(result.active_preset.preset_id, "preset_soft-glow");
    assert_eq!(result.active_preset.published_version, "2026.03.20");
    assert_eq!(active_preset.preset_id, "preset_soft-glow");
    assert_eq!(active_preset.published_version, "2026.03.20");
    assert_eq!(
        manifest.active_preset_id.as_deref(),
        Some("preset_soft-glow")
    );
    assert_eq!(
        manifest.active_preset_display_name.as_deref(),
        Some("Soft Glow")
    );

    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_bytes).expect("manifest JSON should deserialize");

    assert_eq!(
        manifest_json.get("activePresetId"),
        Some(&serde_json::Value::String("preset_soft-glow".into()))
    );
    assert_eq!(
        manifest_json.get("activePresetDisplayName"),
        Some(&serde_json::Value::String("Soft Glow".into()))
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_a_preset_recovers_from_a_manifest_backup_left_by_an_interrupted_write() {
    let base_dir = unique_test_root("preset-selection-manifest-backup");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let backup_path = manifest_path.with_extension("json.bak");

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );

    fs::rename(&manifest_path, &backup_path).expect("manifest should move to backup");

    let result = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("selection should recover from the backup manifest");

    assert_eq!(result.active_preset.preset_id, "preset_soft-glow");
    assert!(manifest_path.is_file());
    assert!(!backup_path.exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn legacy_v1_manifest_with_only_active_preset_id_still_deserializes() {
    let manifest: SessionManifest = serde_json::from_value(serde_json::json!({
      "schemaVersion": "session-manifest/v1",
      "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
      "boothAlias": "Kim 4821",
      "customer": {
        "name": "Kim",
        "phoneLastFour": "4821"
      },
      "createdAt": "2026-03-20T00:00:00Z",
      "updatedAt": "2026-03-20T00:00:00Z",
      "lifecycle": {
        "status": "active",
        "stage": "session-started"
      },
      "activePresetId": null,
      "captures": [],
      "postEnd": null
    }))
    .expect("legacy manifest should deserialize");

    assert!(manifest.active_preset.is_none());
    assert!(manifest.active_preset_id.is_none());
    assert!(manifest.active_preset_display_name.is_none());
}

#[test]
fn legacy_v1_manifest_with_existing_captures_backfills_capture_preset_identity() {
    let mut manifest: SessionManifest = serde_json::from_value(serde_json::json!({
      "schemaVersion": "session-manifest/v1",
      "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
      "boothAlias": "Kim 4821",
      "customer": {
        "name": "Kim",
        "phoneLastFour": "4821"
      },
      "createdAt": "2026-03-20T00:00:00Z",
      "updatedAt": "2026-03-20T00:00:00Z",
      "lifecycle": {
        "status": "active",
        "stage": "capture-ready"
      },
      "activePreset": {
        "presetId": "preset_soft-glow",
        "publishedVersion": "2026.03.20"
      },
      "activePresetId": "preset_soft-glow",
      "activePresetDisplayName": "Soft Glow",
      "captures": [{
        "schemaVersion": "session-capture/v1",
        "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "boothAlias": "Kim 4821",
        "activePresetVersion": "2026.03.20",
        "captureId": "capture_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "requestId": "request_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "raw": {
          "assetPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/captures/originals/capture.jpg",
          "persistedAtMs": 100
        },
        "preview": {
          "assetPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/capture.jpg",
          "enqueuedAtMs": 120,
          "readyAtMs": 200
        },
        "final": {
          "assetPath": null,
          "readyAtMs": null
        },
        "renderStatus": "previewReady",
        "postEndState": "activeSession",
        "timing": {
          "captureAcknowledgedAtMs": 100,
          "previewVisibleAtMs": 200,
          "captureBudgetMs": 1000,
          "previewBudgetMs": 5000,
          "previewBudgetState": "withinBudget"
        }
      }],
      "postEnd": null
    }))
    .expect("legacy manifest with captures should deserialize");

    normalize_legacy_manifest(&mut manifest);

    assert_eq!(
        manifest.captures[0].active_preset_id.as_deref(),
        Some("preset_soft-glow")
    );
    assert_eq!(
        manifest.captures[0].active_preset_display_name.as_deref(),
        Some("Soft Glow")
    );
}

#[test]
fn legacy_phone_required_post_end_without_evaluated_at_still_deserializes() {
    let manifest: SessionManifest = serde_json::from_value(serde_json::json!({
      "schemaVersion": "session-manifest/v1",
      "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
      "boothAlias": "Kim 4821",
      "customer": {
        "name": "Kim",
        "phoneLastFour": "4821"
      },
      "createdAt": "2026-03-20T00:00:00Z",
      "updatedAt": "2026-03-20T00:00:00Z",
      "lifecycle": {
        "status": "active",
        "stage": "phone-required"
      },
      "activePresetId": null,
      "captures": [],
      "postEnd": {
        "state": "phone-required",
        "primaryActionLabel": "가까운 직원에게 알려 주세요.",
        "supportActionLabel": "직원에게 도움을 요청해 주세요.",
        "unsafeActionWarning": "다시 찍기나 기기 조작은 잠시 멈춰 주세요.",
        "showBoothAlias": false
      }
    }))
    .expect("legacy phone-required post-end should deserialize");

    let post_end = manifest.post_end.expect("legacy post-end should exist");

    assert_eq!(post_end.state(), "phone-required");
    assert_eq!(post_end.evaluated_at(), "1970-01-01T00:00:00Z");
}

#[test]
fn selecting_a_preset_preserves_a_later_lifecycle_stage() {
    let base_dir = unique_test_root("preset-selection-stage");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );

    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.lifecycle.stage = "capture-ready".into();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let result = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("preset selection should preserve later stages");

    assert_eq!(result.manifest.lifecycle.stage, "capture-ready");

    let persisted_manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");

    assert_eq!(persisted_manifest.lifecycle.stage, "capture-ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_a_preset_is_blocked_once_exact_end_has_projected_post_end_truth() {
    let base_dir = unique_test_root("preset-selection-post-end-block");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_afterglow",
        "2026.03.20",
        "Afterglow",
        "published",
        true,
        None,
        None,
    );

    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("initial preset selection should persist");

    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    let ended_at = current_timestamp(
        SystemTime::now()
            .checked_sub(Duration::from_secs(10))
            .expect("past timestamp should be valid"),
    )
    .expect("ended timestamp should serialize");
    let warning_at = current_timestamp(
        SystemTime::now()
            .checked_sub(Duration::from_secs(70))
            .expect("past timestamp should be valid"),
    )
    .expect("warning timestamp should serialize");
    let timing = manifest
        .timing
        .as_mut()
        .expect("session timing should exist");
    timing.warning_at = warning_at;
    timing.adjusted_end_at = ended_at;
    timing.phase = "active".into();
    timing.capture_allowed = true;
    timing.warning_triggered_at = None;
    timing.ended_triggered_at = None;
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let error = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_afterglow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect_err("post-end sessions should block preset switching");

    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        error
            .readiness
            .as_ref()
            .map(|readiness| readiness.can_capture),
        Some(false)
    );
    assert_eq!(
        error
            .readiness
            .as_ref()
            .and_then(|readiness| readiness.timing.as_ref())
            .map(|timing| timing.phase.as_str()),
        Some("ended")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_the_same_preset_twice_keeps_the_existing_manifest_timestamp() {
    let base_dir = unique_test_root("preset-selection-idempotent");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );

    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("first selection should persist");

    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.updated_at = "2026-03-20T00:05:00Z".into();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let result = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("re-selecting the same preset should be a no-op");

    let persisted_manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");

    assert_eq!(result.manifest.updated_at, "2026-03-20T00:05:00Z");
    assert_eq!(persisted_manifest.updated_at, "2026-03-20T00:05:00Z");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_the_same_preset_backfills_missing_display_name_without_bumping_timestamp() {
    let base_dir = unique_test_root("preset-selection-backfill-display-name");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );

    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("first selection should persist");

    let mut manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");
    manifest.updated_at = "2026-03-20T00:05:00Z".into();
    manifest.active_preset_display_name = None;
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should be writable");

    let result = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("re-selecting the same preset should backfill display metadata");

    let persisted_manifest: SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should deserialize");

    assert_eq!(result.manifest.updated_at, "2026-03-20T00:05:00Z");
    assert_eq!(persisted_manifest.updated_at, "2026-03-20T00:05:00Z");
    assert_eq!(
        persisted_manifest.active_preset_display_name.as_deref(),
        Some("Soft Glow")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_a_preset_for_a_missing_session_reports_session_not_found_first() {
    let base_dir = unique_test_root("preset-selection-missing-session");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );

    let error = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect_err("missing session should fail before preset lookup");

    assert_eq!(error.code, "session-not-found");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn published_preset_catalog_skips_malformed_bundle_fields_without_failing_the_whole_catalog() {
    let base_dir = unique_test_root("preset-catalog-malformed-fields");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before loading the preset catalog");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(
        &catalog_root,
        "preset_soft-glow",
        "2026.03.20",
        "Soft Glow",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "bad preset id",
        "2026.03.20",
        "Bad Id",
        "published",
        true,
        None,
        None,
    );
    create_published_bundle(
        &catalog_root,
        "preset_blank-name",
        "2026.03.20",
        "   ",
        "published",
        true,
        None,
        None,
    );

    let result = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("invalid bundles should be skipped instead of failing the catalog");

    assert_eq!(result.state, "ready");
    assert_eq!(result.presets.len(), 1);
    assert_eq!(result.presets[0].preset_id, "preset_soft-glow");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn published_preset_catalog_rejects_preview_assets_outside_the_bundle_directory() {
    let base_dir = unique_test_root("preset-catalog-preview-escape");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before loading the preset catalog");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    fs::write(base_dir.join("outside.jpg"), b"outside").expect("outside preview should exist");

    create_published_bundle(
        &catalog_root,
        "preset_escape",
        "2026.03.20",
        "Escape",
        "published",
        false,
        None,
        Some("../../../../outside.jpg"),
    );

    let result = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("escaped previews should be skipped instead of failing the catalog");

    assert_eq!(result.state, "empty");
    assert!(result.presets.is_empty());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn published_preset_catalog_skips_bundles_whose_parent_folder_does_not_match_preset_id() {
    let base_dir = unique_test_root("preset-catalog-preset-folder-mismatch");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before loading the preset catalog");
    let bundle_dir = resolve_published_preset_catalog_dir(&base_dir)
        .join("preset_folder-name")
        .join("2026.03.20");
    fs::create_dir_all(&bundle_dir).expect("bundle directory should be creatable");
    fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should be written");
    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "published-preset-bundle/v1",
          "presetId": "preset_manifest-name",
          "displayName": "Mismatch",
          "publishedVersion": "2026.03.20",
          "lifecycleStatus": "published",
          "boothStatus": "booth-safe",
          "preview": {
            "kind": "preview-tile",
            "assetPath": "preview.jpg",
            "altText": "Mismatch sample portrait",
          }
        }))
        .expect("bundle should serialize"),
    )
    .expect("bundle should be written");

    let result = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("mismatched bundles should be skipped");

    assert_eq!(result.state, "empty");
    assert!(result.presets.is_empty());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn selecting_a_preset_rejects_entries_outside_the_customer_visible_top_six() {
    let base_dir = unique_test_root("preset-selection-visible-top-six");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before selecting a preset");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    for (preset_id, display_name) in [
        ("preset_afterglow", "Afterglow"),
        ("preset_daylight", "Daylight"),
        ("preset_mono-pop", "Mono Pop"),
        ("preset_porcelain", "Porcelain"),
        ("preset_soft-glow", "Soft Glow"),
        ("preset_velvet", "Velvet"),
        ("preset_hidden-zulu", "Zulu"),
    ] {
        create_published_bundle(
            &catalog_root,
            preset_id,
            "2026.03.20",
            display_name,
            "published",
            true,
            None,
            None,
        );
    }

    let error = select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id,
            preset_id: "preset_hidden-zulu".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect_err("presets outside the visible top six should not be selectable");

    assert_eq!(error.code, "preset-not-available");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn published_preset_catalog_uses_stable_tie_breaking_for_duplicate_display_names() {
    let base_dir = unique_test_root("preset-catalog-stable-display-name-ties");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should exist before loading the preset catalog");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    for (preset_id, display_name) in [
        ("preset_alpha", "Alpha"),
        ("preset_beta", "Beta"),
        ("preset_gamma", "Gamma"),
        ("preset_same-a", "Same"),
        ("preset_same-b", "Same"),
        ("preset_theta", "Theta"),
        ("preset_zeta", "Zeta"),
    ] {
        create_published_bundle(
            &catalog_root,
            preset_id,
            "2026.03.20",
            display_name,
            "published",
            true,
            None,
            None,
        );
    }

    let result = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("catalog should load with deterministic ordering");

    assert_eq!(
        result
            .presets
            .iter()
            .map(|preset| preset.preset_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "preset_alpha",
            "preset_beta",
            "preset_gamma",
            "preset_same-a",
            "preset_same-b",
            "preset_theta",
        ],
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn loading_a_preset_catalog_rejects_invalid_session_id_shapes() {
    let base_dir = unique_test_root("preset-catalog-invalid-session-id");

    let error = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: "../session_escape".into(),
        },
    )
    .expect_err("invalid session ids should be rejected before touching disk paths");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

fn create_published_bundle(
    catalog_root: &PathBuf,
    preset_id: &str,
    directory_version: &str,
    display_name: &str,
    lifecycle_status: &str,
    with_preview: bool,
    manifest_version_override: Option<&str>,
    preview_asset_path_override: Option<&str>,
) {
    let bundle_dir = catalog_root.join(preset_id).join(directory_version);
    fs::create_dir_all(&bundle_dir).expect("bundle directory should be creatable");

    if with_preview {
        fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should be written");
    }

    let published_version = manifest_version_override.unwrap_or(directory_version);
    let preview_asset_path = preview_asset_path_override.unwrap_or("preview.jpg");
    let bundle = serde_json::json!({
      "schemaVersion": "published-preset-bundle/v1",
      "presetId": preset_id,
      "displayName": display_name,
      "publishedVersion": published_version,
      "lifecycleStatus": lifecycle_status,
      "boothStatus": "booth-safe",
      "preview": {
        "kind": "preview-tile",
        "assetPath": preview_asset_path,
        "altText": format!("{display_name} sample portrait"),
      }
    });

    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&bundle).expect("bundle should serialize"),
    )
    .expect("bundle should be written");
}
