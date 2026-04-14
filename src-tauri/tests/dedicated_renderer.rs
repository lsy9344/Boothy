use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard, Once, OnceLock},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    branch_config::rollback_preview_renderer_route_in_dir,
    capture::{
        normalized_state::{get_capture_readiness_in_dir, request_capture_in_dir},
        sidecar_client::{
            read_capture_request_messages, CanonHelperCaptureRequestMessage,
            CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION, CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        },
    },
    contracts::dto::{
        CaptureReadinessInputDto, CaptureRequestInputDto, PreviewRendererRouteRollbackInputDto,
        SessionStartInputDto,
    },
    diagnostics::recovery::load_operator_recovery_summary_in_dir,
    preset::{
        default_catalog::ensure_default_preset_catalog_in_dir,
        preset_catalog::resolve_published_preset_catalog_dir,
    },
    render::dedicated_renderer::complete_capture_preview_with_dedicated_renderer_in_dir,
    session::{
        session_manifest::current_timestamp,
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

static FAKE_DARKTABLE_SETUP: Once = Once::new();
static DEDICATED_RENDERER_TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn unique_test_root(test_name: &str) -> PathBuf {
    ensure_fake_darktable_cli();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-dedicated-renderer-{test_name}-{stamp}"))
}

fn ensure_fake_darktable_cli() {
    FAKE_DARKTABLE_SETUP.call_once(|| {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", script_path);
        std::env::set_var("BOOTHY_TEST_RENDER_QUEUE_LIMIT", "unbounded");
    });
}

fn lock_dedicated_renderer_test_env() -> MutexGuard<'static, ()> {
    DEDICATED_RENDERER_TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("dedicated renderer test env lock should not be poisoned")
}

#[test]
fn queue_saturated_dedicated_renderer_submission_falls_back_without_false_ready() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("queue-saturated-fallback");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "integration-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let readiness_before = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should resolve before preview completion");

    assert_eq!(readiness_before.reason_code, "preview-waiting");
    assert_eq!(
        readiness_before
            .latest_capture
            .as_ref()
            .and_then(|value| value.preview.ready_at_ms),
        None
    );

    let env_guard =
        ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "queue-saturated");
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("queue saturation should fall back to the inline truthful renderer");
    drop(env_guard);

    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(completed_capture.preview.ready_at_ms.is_some());
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id))
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        completed_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.as_str())
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("event=preview-render-queue-saturated"));
    assert!(timing_events.contains("event=capture_preview_ready"));
    assert!(timing_events.contains("event=capture_preview_transition_summary"));
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=render-queue-saturated"));

    let readiness_after = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should resolve after fallback completion");
    assert_eq!(readiness_after.reason_code, "ready");
    assert_eq!(readiness_after.surface_state, "previewReady");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn route_policy_shadow_mode_keeps_inline_truth_even_when_dev_env_requests_sidecar() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("route-policy-shadow");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let env_guard = ScopedEnvVarGuard::set("BOOTHY_DEDICATED_RENDERER_ENABLE_SPAWN", "true");
    let outcome_guard =
        ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("shadow route should still close via the inline truthful renderer");
    drop(outcome_guard);
    drop(env_guard);

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=route-policy-shadow"));
    assert!(timing_events.contains("routeStage=shadow"));
    assert!(
        !timing_events.contains("laneOwner=dedicated-renderer"),
        "shadow route should ignore dev env sidecar opt-in as a release substitute"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn accepted_dedicated_renderer_result_claims_truthful_close_without_inline_overwrite() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("accepted-close-owner");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "integration-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    overwrite_catalog_state_marker(&base_dir, "capture-time");
    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    overwrite_catalog_state_marker(&base_dir, "late-state");
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    let dedicated_renderer_bytes = [0xFF, 0xD8, 0xFF, 0xD9];
    fs::write(&canonical_preview_path, dedicated_renderer_bytes)
        .expect("accepted dedicated renderer output should exist");

    let env_guard = ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("accepted dedicated renderer output should close the truthful preview");
    drop(env_guard);

    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(completed_capture.preview.ready_at_ms.is_some());
    let normalized_canonical_preview_path =
        canonical_preview_path.to_string_lossy().replace('\\', "/");
    assert_eq!(
        completed_capture
            .preview
            .asset_path
            .as_deref()
            .map(|path| path.replace('\\', "/")),
        Some(normalized_canonical_preview_path)
    );
    assert_eq!(
        fs::read(&canonical_preview_path).expect("accepted preview should stay on disk"),
        dedicated_renderer_bytes,
        "accepted dedicated renderer output should not be overwritten by the inline fallback path"
    );
    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("event=capture_preview_ready"));
    assert!(timing_events.contains("event=capture_preview_transition_summary"));
    assert!(timing_events.contains("laneOwner=dedicated-renderer"));
    assert!(timing_events.contains("fallbackReason=none"));
    assert!(timing_events.contains("routeStage=canary"));
    assert!(timing_events.contains("originalVisibleToPresetAppliedVisibleMs="));
    let evidence_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("dedicated-renderer")
        .join("preview-promotion-evidence.jsonl");
    let evidence_lines =
        fs::read_to_string(&evidence_path).expect("preview promotion evidence should exist");
    let evidence_record: serde_json::Value = serde_json::from_str(
        evidence_lines
            .lines()
            .last()
            .expect("preview promotion evidence should include a record"),
    )
    .expect("preview promotion evidence should be valid json");
    assert_eq!(
        evidence_record
            .get("schemaVersion")
            .and_then(|value| value.as_str()),
        Some("preview-promotion-evidence-record/v1")
    );
    assert_eq!(
        evidence_record
            .get("laneOwner")
            .and_then(|value| value.as_str()),
        Some("dedicated-renderer")
    );
    let expected_route_policy_snapshot_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("dedicated-renderer")
        .join(format!(
            "captured-preview-renderer-policy-{}.json",
            capture.capture.capture_id
        ))
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(
        evidence_record
            .get("routePolicySnapshotPath")
            .and_then(|value| value.as_str()),
        Some(expected_route_policy_snapshot_path.as_str())
    );
    let expected_catalog_snapshot_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("dedicated-renderer")
        .join(format!(
            "captured-catalog-state-{}.json",
            capture.capture.capture_id
        ))
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(
        evidence_record
            .get("catalogStatePath")
            .and_then(|value| value.as_str()),
        Some(expected_catalog_snapshot_path.as_str())
    );
    let expected_timing_events_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("timing-events.log")
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(
        evidence_record
            .get("timingEventsPath")
            .and_then(|value| value.as_str()),
        Some(expected_timing_events_path.as_str())
    );
    let route_policy_snapshot: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            SessionPaths::new(&base_dir, &session.session_id)
                .diagnostics_dir
                .join("dedicated-renderer")
                .join(format!(
                    "captured-preview-renderer-policy-{}.json",
                    capture.capture.capture_id
                )),
        )
        .expect("captured route policy snapshot should exist"),
    )
    .expect("captured route policy snapshot should deserialize");
    assert_eq!(route_policy_snapshot["defaultRoute"], "darktable");
    let manifest_snapshot: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(SessionPaths::new(&base_dir, &session.session_id).manifest_path)
            .expect("manifest should exist after accepted close"),
    )
    .expect("manifest should deserialize after accepted close");
    let catalog_snapshot: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            SessionPaths::new(&base_dir, &session.session_id)
                .diagnostics_dir
                .join("dedicated-renderer")
                .join(format!(
                    "captured-catalog-state-{}.json",
                    capture.capture.capture_id
                )),
        )
        .expect("captured catalog snapshot should exist"),
    )
    .expect("captured catalog snapshot should deserialize");
    assert_eq!(
        catalog_snapshot["catalogRevision"],
        manifest_snapshot["catalogRevision"]
    );
    assert_eq!(
        catalog_snapshot["livePresets"],
        manifest_snapshot["catalogSnapshot"]
    );
    assert_ne!(catalog_snapshot["marker"], serde_json::json!("late-state"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn capture_time_catalog_snapshot_stays_bound_to_session_catalog_snapshot() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("catalog-snapshot-session-bound");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "integration-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let manifest_snapshot: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(SessionPaths::new(&base_dir, &session.session_id).manifest_path)
            .expect("manifest should exist after preset selection"),
    )
    .expect("manifest should deserialize after preset selection");
    create_published_bundle_variant(&catalog_root, "preset_soft-glow", "2026.04.11", "Soft Glow");
    overwrite_catalog_state_live_version(&base_dir, "preset_soft-glow", "2026.04.11", 99);

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    fs::write(&canonical_preview_path, [0xFF, 0xD8, 0xFF, 0xD9])
        .expect("accepted preview should exist");

    let env_guard = ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let _ = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("accepted dedicated renderer output should close the truthful preview");
    drop(env_guard);

    let captured_catalog_snapshot: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            SessionPaths::new(&base_dir, &session.session_id)
                .diagnostics_dir
                .join("dedicated-renderer")
                .join(format!(
                    "captured-catalog-state-{}.json",
                    capture.capture.capture_id
                )),
        )
        .expect("captured catalog snapshot should exist"),
    )
    .expect("captured catalog snapshot should deserialize");
    assert_eq!(
        captured_catalog_snapshot["catalogRevision"],
        manifest_snapshot["catalogRevision"]
    );
    assert_eq!(
        captured_catalog_snapshot["livePresets"],
        manifest_snapshot["catalogSnapshot"]
    );
    assert_ne!(
        captured_catalog_snapshot["livePresets"],
        serde_json::json!([
            {
                "presetId": "preset_soft-glow",
                "publishedVersion": "2026.04.11"
            }
        ]),
        "capture evidence must not drift to the future-session live catalog"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_promotion_evidence_write_failures_are_logged_for_hardware_gate_visibility() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("promotion-evidence-write-failure");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "integration-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    fs::write(&canonical_preview_path, [0xFF, 0xD8, 0xFF, 0xD9])
        .expect("accepted dedicated renderer output should exist");

    let outcome_guard =
        ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let failure_guard = ScopedEnvVarGuard::set(
        "BOOTHY_TEST_PREVIEW_PROMOTION_EVIDENCE_WRITE_FAILURE",
        "disk-full",
    );
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("preview should still close even if evidence persistence fails");
    drop(failure_guard);
    drop(outcome_guard);

    assert_eq!(completed_capture.render_status, "previewReady");
    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("event=preview-promotion-evidence-write-failed"));
    assert!(timing_events.contains("disk-full"));

    let evidence_path = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("dedicated-renderer")
        .join("preview-promotion-evidence.jsonl");
    assert!(
        !evidence_path.exists(),
        "failed evidence persistence should not leave a partial evidence log behind"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn active_session_keeps_selected_route_even_after_policy_rolls_back() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("session-route-snapshot");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "integration-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");
    rollback_preview_renderer_route_in_dir(
        &base_dir,
        &boothy_lib::commands::runtime_commands::capability_snapshot_for_profile(
            "operator-enabled",
            true,
        ),
        PreviewRendererRouteRollbackInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("host-owned rollback should succeed");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    fs::write(&canonical_preview_path, [0xFF, 0xD8, 0xFF, 0xD9])
        .expect("accepted dedicated renderer output should exist");

    let env_guard = ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("active session route should stay on its selected canary snapshot");
    drop(env_guard);

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(timing_events.contains("laneOwner=dedicated-renderer"));
    assert!(timing_events.contains("routeStage=canary"));
    assert!(
        !timing_events.contains("fallbackReason=route-policy-rollback"),
        "policy rollback after preset selection should not reinterpret the active session"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn stale_dedicated_renderer_result_is_ignored_when_no_sidecar_run_happens() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("stale-result-ignored");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "darktable",
          "defaultRoutes": [],
          "canaryRoutes": [
            {
              "route": "local-renderer-sidecar",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "integration-canary"
            }
          ],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let paths = SessionPaths::new(&base_dir, &session.session_id);
    let stale_result_path = paths
        .diagnostics_dir
        .join("dedicated-renderer")
        .join(format!(
            "{}-{}.preview-result.json",
            capture.capture.capture_id, capture.capture.request_id
        ));
    let canonical_preview_path = paths
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    fs::create_dir_all(
        stale_result_path
            .parent()
            .expect("stale result should have a parent directory"),
    )
    .expect("dedicated renderer diagnostics directory should exist");
    fs::write(&canonical_preview_path, [0xFF, 0xD8, 0xFF, 0xD9])
        .expect("canonical preview placeholder should exist");
    fs::write(
        &stale_result_path,
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "dedicated-renderer-preview-job-result/v1",
          "sessionId": session.session_id,
          "requestId": capture.capture.request_id,
          "captureId": capture.capture.capture_id,
          "status": "accepted",
          "diagnosticsDetailPath": stale_result_path.to_string_lossy().replace('\\', "/"),
          "outputPath": canonical_preview_path.to_string_lossy().replace('\\', "/"),
          "detailCode": "accepted",
          "detailMessage": "stale accepted result"
        }))
        .expect("stale result should serialize"),
    )
    .expect("stale result should be writable");

    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("stale dedicated renderer result should not block inline fallback completion");

    let timing_events = fs::read_to_string(paths.diagnostics_dir.join("timing-events.log"))
        .expect("timing log should exist");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(timing_events.contains("event=capture_preview_transition_summary"));
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=resident-not-warmed"));
    assert!(timing_events.contains("warmState=cold"));
    assert!(
        !timing_events.contains("laneOwner=dedicated-renderer"),
        "stale accepted result should not be reused as the truthful close owner"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn forced_fallback_route_logs_route_policy_rollback_reason() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("forced-fallback-route");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": [
            {
              "route": "darktable",
              "presetId": "preset_soft-glow",
              "presetVersion": "2026.04.10",
              "reason": "rollback"
            }
          ]
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("rollback route should fall back safely");

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=route-policy-rollback"));
    assert!(timing_events.contains("routeStage=shadow"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn resident_warmup_updates_manifest_and_operator_projection_before_preview_close() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("resident-warmup-manifest");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    boothy_lib::render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        None,
        &base_dir,
        &session.session_id,
        "preset_soft-glow",
        "2026.04.10",
    );

    let manifest: boothy_lib::session::session_manifest::SessionManifest = serde_json::from_str(
        &fs::read_to_string(SessionPaths::new(&base_dir, &session.session_id).manifest_path)
            .expect("manifest should be readable after warmup"),
    )
    .expect("manifest should deserialize after warmup");
    let warm_state = manifest
        .active_preview_renderer_warm_state
        .expect("warm state should be recorded after warmup");
    assert_eq!(warm_state.preset_id, "preset_soft-glow");
    assert_eq!(warm_state.published_version, "2026.04.10");
    assert_eq!(warm_state.state, "warm-ready");

    let capability_snapshot =
        boothy_lib::commands::runtime_commands::capability_snapshot_for_profile(
            "operator-enabled",
            true,
        );
    let operator_summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should project warm state");
    assert_eq!(
        operator_summary.preview_architecture.warm_state.as_deref(),
        Some("warm-ready")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn resident_preview_warm_hit_claims_truthful_close_from_dedicated_renderer() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("resident-preview-warm-hit");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    boothy_lib::render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        None,
        &base_dir,
        &session.session_id,
        "preset_soft-glow",
        "2026.04.10",
    );

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("warmed resident renderer should close the truthful preview");
    assert_eq!(completed_capture.render_status, "previewReady");

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("laneOwner=dedicated-renderer"));
    assert!(timing_events.contains("fallbackReason=none"));
    assert!(timing_events.contains("warmState=warm-hit"));

    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    assert!(
        canonical_preview_path.is_file(),
        "resident renderer should produce the canonical preview output"
    );

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest: boothy_lib::session::session_manifest::SessionManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable after warm hit"),
    )
    .expect("manifest should deserialize after warm hit");
    let warm_state = manifest
        .active_preview_renderer_warm_state
        .as_ref()
        .expect("warm state should remain present after placeholder fallback");
    assert_eq!(warm_state.state, "warm-hit");

    let capability_snapshot =
        boothy_lib::commands::runtime_commands::capability_snapshot_for_profile(
            "operator-enabled",
            true,
        );
    let operator_summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should project dedicated renderer warm-hit evidence");
    assert_eq!(
        operator_summary.preview_architecture.warm_state.as_deref(),
        Some("warm-hit")
    );
    assert_eq!(
        operator_summary.preview_architecture.lane_owner.as_deref(),
        Some("dedicated-renderer")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn active_preset_warm_state_is_not_overwritten_by_older_capture_completion() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("active-preset-warm-state-guard");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    create_published_bundle_variant(&catalog_root, "preset_cool-tone", "2026.04.11", "Cool Tone");
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("first preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);

    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_cool-tone".into(),
            published_version: "2026.04.11".into(),
        },
    )
    .expect("second preset should become active before preview completion");

    let env_guard = ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let _ = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("older capture should still complete safely");
    drop(env_guard);

    let manifest: boothy_lib::session::session_manifest::SessionManifest = serde_json::from_str(
        &fs::read_to_string(SessionPaths::new(&base_dir, &session.session_id).manifest_path)
            .expect("manifest should remain readable"),
    )
    .expect("manifest should deserialize after older capture completion");
    let warm_state = manifest
        .active_preview_renderer_warm_state
        .expect("active preset warm state should remain present");
    assert_eq!(warm_state.preset_id, "preset_cool-tone");
    assert_eq!(warm_state.published_version, "2026.04.11");
    assert_eq!(warm_state.state, "cold");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn resident_preview_logs_warm_state_loss_falls_back_safely() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("resident-preview-warm-loss");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    boothy_lib::render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        None,
        &base_dir,
        &session.session_id,
        "preset_soft-glow",
        "2026.04.10",
    );

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let mut manifest: boothy_lib::session::session_manifest::SessionManifest =
        serde_json::from_str(
            &fs::read_to_string(&manifest_path)
                .expect("manifest should be readable after warm hit"),
        )
        .expect("manifest should deserialize after warm hit");
    let diagnostics_dir = SessionPaths::new(&base_dir, &session.session_id)
        .diagnostics_dir
        .join("dedicated-renderer");
    let warm_state_detail_path =
        diagnostics_dir.join("warm-state-preset_soft-glow-2026.04.10.json");
    fs::remove_file(&warm_state_detail_path).expect("warm state evidence should be removable");
    manifest.active_preview_renderer_warm_state = Some(
        boothy_lib::session::session_manifest::PreviewRendererWarmStateSnapshot {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            state: "warm-ready".into(),
            observed_at: "2026-04-12T08:00:00Z".into(),
            diagnostics_detail_path: Some(
                warm_state_detail_path.to_string_lossy().replace('\\', "/"),
            ),
        },
    );
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should persist forced warm-loss setup");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("warm-state loss should fall back safely");
    assert_eq!(completed_capture.render_status, "previewReady");

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=warm-state-loss"));
    assert!(timing_events.contains("warmState=warm-state-lost"));

    let capability_snapshot =
        boothy_lib::commands::runtime_commands::capability_snapshot_for_profile(
            "operator-enabled",
            true,
        );
    let operator_summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should project warm-state loss");
    assert_eq!(
        operator_summary.preview_architecture.warm_state.as_deref(),
        Some("warm-state-lost")
    );

    let readiness = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should remain truthful after warm-state loss");
    assert_eq!(readiness.reason_code, "ready");
    assert_eq!(readiness.customer_state, "Ready");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn resident_preview_spawn_failure_clears_stale_warm_state_for_operator_truth() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("resident-preview-spawn-failure");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    boothy_lib::render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        None,
        &base_dir,
        &session.session_id,
        "preset_soft-glow",
        "2026.04.10",
    );

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let failure_guard = ScopedEnvVarGuard::set(
        "BOOTHY_TEST_DEDICATED_RENDERER_START_FAILURE",
        "sidecar-unavailable",
    );
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("spawn failure should fall back safely");
    drop(failure_guard);

    assert_eq!(completed_capture.render_status, "previewReady");
    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=sidecar-unavailable"));
    assert!(timing_events.contains("warmState=warm-state-lost"));

    let manifest: boothy_lib::session::session_manifest::SessionManifest = serde_json::from_str(
        &fs::read_to_string(SessionPaths::new(&base_dir, &session.session_id).manifest_path)
            .expect("manifest should be readable after spawn failure"),
    )
    .expect("manifest should deserialize after spawn failure");
    assert_eq!(
        manifest
            .active_preview_renderer_warm_state
            .as_ref()
            .map(|snapshot| snapshot.state.as_str()),
        Some("warm-state-lost")
    );

    let capability_snapshot =
        boothy_lib::commands::runtime_commands::capability_snapshot_for_profile(
            "operator-enabled",
            true,
        );
    let operator_summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should project the degraded warm state");
    assert_eq!(
        operator_summary.preview_architecture.warm_state.as_deref(),
        Some("warm-state-lost")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn resident_warmup_spawn_failure_clears_stale_warm_state_for_operator_truth() {
    let _env_lock = lock_dedicated_renderer_test_env();
    let base_dir = unique_test_root("resident-warmup-spawn-failure");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
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
    write_preview_renderer_policy(
        &base_dir,
        serde_json::json!({
          "schemaVersion": "preview-renderer-route-policy/v1",
          "defaultRoute": "local-renderer-sidecar",
          "defaultRoutes": [],
          "canaryRoutes": [],
          "forcedFallbackRoutes": []
        }),
    );
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    boothy_lib::render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        None,
        &base_dir,
        &session.session_id,
        "preset_soft-glow",
        "2026.04.10",
    );

    let failure_guard = ScopedEnvVarGuard::set(
        "BOOTHY_TEST_DEDICATED_RENDERER_START_FAILURE",
        "sidecar-unavailable",
    );
    boothy_lib::render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        None,
        &base_dir,
        &session.session_id,
        "preset_soft-glow",
        "2026.04.10",
    );
    drop(failure_guard);

    let manifest: boothy_lib::session::session_manifest::SessionManifest = serde_json::from_str(
        &fs::read_to_string(SessionPaths::new(&base_dir, &session.session_id).manifest_path)
            .expect("manifest should be readable after warmup spawn failure"),
    )
    .expect("manifest should deserialize after warmup spawn failure");
    assert_eq!(
        manifest
            .active_preview_renderer_warm_state
            .as_ref()
            .map(|snapshot| snapshot.state.as_str()),
        Some("warm-state-lost")
    );

    let capability_snapshot =
        boothy_lib::commands::runtime_commands::capability_snapshot_for_profile(
            "operator-enabled",
            true,
        );
    let operator_summary = load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
        .expect("operator summary should project the degraded warm state");
    assert_eq!(
        operator_summary.preview_architecture.warm_state.as_deref(),
        Some("warm-state-lost")
    );

    let _ = fs::remove_dir_all(base_dir);
}

fn create_published_bundle(catalog_root: &PathBuf) {
    create_published_bundle_variant(catalog_root, "preset_soft-glow", "2026.04.10", "Soft Glow");
}

fn create_published_bundle_variant(
    catalog_root: &PathBuf,
    preset_id: &str,
    published_version: &str,
    display_name: &str,
) {
    let bundle_dir = catalog_root.join(preset_id).join(published_version);
    fs::create_dir_all(bundle_dir.join("xmp")).expect("xmp directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should exist");
    fs::write(
        bundle_dir.join("xmp").join("template.xmp"),
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">",
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">",
            "<rdf:Description xmlns:darktable=\"http://darktable.sf.net/\">",
            "<darktable:history><rdf:Seq><rdf:li><darktable:module>bundle</darktable:module></rdf:li></rdf:Seq></darktable:history>",
            "</rdf:Description></rdf:RDF></x:xmpmeta>"
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
        "profileId": "soft-glow-preview",
        "displayName": "Soft Glow Preview",
        "outputColorSpace": "sRGB"
      },
      "finalProfile": {
        "profileId": "soft-glow-final",
        "displayName": "Soft Glow Final",
        "outputColorSpace": "sRGB"
      },
      "preview": {
        "kind": "preview-tile",
        "assetPath": "preview.jpg",
        "altText": "Soft Glow sample portrait"
      }
    });

    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&bundle).expect("bundle should serialize"),
    )
    .expect("bundle should be writable");
}

fn write_preview_renderer_policy(base_dir: &PathBuf, policy: serde_json::Value) {
    let policy_path = base_dir
        .join("branch-config")
        .join("preview-renderer-policy.json");
    fs::create_dir_all(
        policy_path
            .parent()
            .expect("policy path should have a parent directory"),
    )
    .expect("policy directory should exist");
    fs::write(
        policy_path,
        serde_json::to_vec_pretty(&policy).expect("policy should serialize"),
    )
    .expect("policy should be writable");
}

fn overwrite_catalog_state_marker(base_dir: &PathBuf, marker: &str) {
    let path = base_dir.join("preset-catalog").join("catalog-state.json");
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("catalog state should exist"))
            .expect("catalog state should deserialize");
    state["marker"] = serde_json::Value::String(marker.into());
    fs::write(
        path,
        serde_json::to_vec_pretty(&state).expect("catalog state should serialize"),
    )
    .expect("catalog state should write");
}

fn overwrite_catalog_state_live_version(
    base_dir: &PathBuf,
    preset_id: &str,
    published_version: &str,
    catalog_revision: u64,
) {
    let path = base_dir.join("preset-catalog").join("catalog-state.json");
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("catalog state should exist"))
            .expect("catalog state should deserialize");
    state["catalogRevision"] = serde_json::json!(catalog_revision);
    state["livePresets"] = serde_json::json!([
        {
            "presetId": preset_id,
            "publishedVersion": published_version
        }
    ]);
    fs::write(
        path,
        serde_json::to_vec_pretty(&state).expect("catalog state should serialize"),
    )
    .expect("catalog state should write");
}

fn request_capture_with_helper_success(
    base_dir: &PathBuf,
    session_id: &str,
) -> boothy_lib::contracts::dto::CaptureRequestResultDto {
    write_ready_helper_status(base_dir, session_id);
    let existing_request_count = read_capture_request_messages(base_dir, session_id)
        .expect("request log should be readable")
        .len();

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
            request_id: Some(format!(
                "request_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )),
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

fn append_helper_event(base_dir: &PathBuf, session_id: &str, event: serde_json::Value) {
    let event_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("camera-helper-events.jsonl");
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
          "type": "camera-status",
          "sessionId": session_id,
          "sequence": 1,
          "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
          "cameraState": "ready",
          "helperState": "healthy"
        }))
        .expect("helper status should serialize"),
    )
    .expect("helper status should be writable");
}

struct ScopedEnvVarGuard {
    key: String,
    original_value: Option<std::ffi::OsString>,
}

impl ScopedEnvVarGuard {
    fn set(key: &str, value: &str) -> Self {
        let original_value = std::env::var_os(key);
        std::env::set_var(key, value);
        Self {
            key: key.into(),
            original_value,
        }
    }
}

impl Drop for ScopedEnvVarGuard {
    fn drop(&mut self) {
        match self.original_value.take() {
            Some(value) => std::env::set_var(&self.key, value),
            None => std::env::remove_var(&self.key),
        }
    }
}
