use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    branch_config::{
        apply_branch_rollback_in_dir, apply_branch_rollout_in_dir,
        load_branch_rollout_overview_in_dir, load_preview_renderer_route_status_in_dir,
        promote_preview_renderer_route_in_dir, rollback_preview_renderer_route_in_dir,
    },
    commands::runtime_commands::capability_snapshot_for_profile,
    contracts::dto::{
        BranchRollbackInputDto, BranchRolloutInputDto, PreviewRendererRoutePromotionInputDto,
        PreviewRendererRouteRollbackInputDto, PreviewRendererRouteStatusInputDto,
    },
};

static BRANCH_ROLLOUT_TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-branch-rollout-{test_name}-{stamp}"))
}

fn lock_branch_rollout_test_env() -> MutexGuard<'static, ()> {
    BRANCH_ROLLOUT_TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("branch rollout test env lock should not be poisoned")
}

#[test]
fn rollout_targets_only_selected_branches_and_preserves_local_settings() {
    let base_dir = unique_test_root("explicit-branch-set");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_branch_store(&base_dir, false);

    let result = apply_branch_rollout_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRolloutInputDto {
            branch_ids: vec!["gangnam-01".into()],
            target_build_version: "boothy-2026.03.27.1".into(),
            target_preset_stack_version: "catalog-2026.03.27".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("rollout should succeed");

    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(result.outcomes[0].branch_id, "gangnam-01");
    assert_eq!(result.outcomes[0].result, "applied");

    let overview =
        load_branch_rollout_overview_in_dir(&base_dir, &capability_snapshot).expect("load state");
    let gangnam = overview
        .branches
        .iter()
        .find(|branch| branch.branch_id == "gangnam-01")
        .expect("gangnam branch should exist");
    let hongdae = overview
        .branches
        .iter()
        .find(|branch| branch.branch_id == "hongdae-02")
        .expect("hongdae branch should exist");

    assert_eq!(
        gangnam.deployment_baseline.build_version,
        "boothy-2026.03.27.1"
    );
    assert_eq!(
        hongdae.deployment_baseline.build_version,
        "boothy-2026.03.20.4"
    );
    assert!(gangnam.local_settings.summary.contains("연락처"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rollout_defers_active_session_branches_until_safe_transition_then_applies() {
    let base_dir = unique_test_root("defer-until-safe-transition");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_branch_store(&base_dir, true);
    seed_active_session_manifest(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "active",
        None,
    );

    let result = apply_branch_rollout_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRolloutInputDto {
            branch_ids: vec!["hongdae-02".into()],
            target_build_version: "boothy-2026.03.27.1".into(),
            target_preset_stack_version: "catalog-2026.03.27".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("staged rollout should succeed");

    assert_eq!(result.outcomes[0].result, "deferred");

    seed_active_session_manifest(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "ended",
        Some("completed"),
    );

    let overview =
        load_branch_rollout_overview_in_dir(&base_dir, &capability_snapshot).expect("load state");
    let hongdae = overview
        .branches
        .iter()
        .find(|branch| branch.branch_id == "hongdae-02")
        .expect("hongdae branch should exist");

    assert_eq!(
        hongdae.deployment_baseline.build_version,
        "boothy-2026.03.27.1"
    );
    assert!(hongdae.pending_baseline.is_none());
    assert!(hongdae.active_session.is_none());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rollback_cancels_staged_rollout_during_an_active_session() {
    let base_dir = unique_test_root("cancel-staged-rollout");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_branch_store(&base_dir, true);
    seed_active_session_manifest(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "active",
        None,
    );

    let rollout = apply_branch_rollout_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRolloutInputDto {
            branch_ids: vec!["hongdae-02".into()],
            target_build_version: "boothy-2026.03.27.1".into(),
            target_preset_stack_version: "catalog-2026.03.27".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("rollout should defer");

    assert_eq!(rollout.outcomes[0].result, "deferred");

    let rollback = apply_branch_rollback_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRollbackInputDto {
            branch_ids: vec!["hongdae-02".into()],
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("rollback should cancel the staged rollout");

    assert_eq!(rollback.outcomes[0].result, "applied");
    assert_eq!(
        rollback.outcomes[0].effective_baseline.build_version,
        "boothy-2026.03.20.4"
    );
    assert!(rollback.outcomes[0].pending_baseline.is_none());

    let overview =
        load_branch_rollout_overview_in_dir(&base_dir, &capability_snapshot).expect("load state");
    let hongdae = overview
        .branches
        .iter()
        .find(|branch| branch.branch_id == "hongdae-02")
        .expect("hongdae branch should exist");

    assert_eq!(
        hongdae.deployment_baseline.build_version,
        "boothy-2026.03.20.4"
    );
    assert!(hongdae.pending_baseline.is_none());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rollback_restores_last_approved_baseline_and_rejects_missing_baselines() {
    let base_dir = unique_test_root("rollback-restore");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_branch_store(&base_dir, false);

    let rollout = apply_branch_rollout_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRolloutInputDto {
            branch_ids: vec!["gangnam-01".into()],
            target_build_version: "boothy-2026.03.27.1".into(),
            target_preset_stack_version: "catalog-2026.03.27".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("rollout should succeed");
    assert_eq!(rollout.outcomes[0].result, "applied");

    let rollback = apply_branch_rollback_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRollbackInputDto {
            branch_ids: vec!["gangnam-01".into(), "itaewon-03".into()],
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("rollback should return typed outcomes");

    assert_eq!(rollback.outcomes[0].branch_id, "gangnam-01");
    assert_eq!(rollback.outcomes[0].result, "applied");
    assert_eq!(rollback.outcomes[1].branch_id, "itaewon-03");
    assert_eq!(rollback.outcomes[1].result, "rejected");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rejected_rollout_requests_are_still_written_to_audit_history() {
    let base_dir = unique_test_root("rejected-audit-history");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_branch_store(&base_dir, false);

    let result = apply_branch_rollout_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRolloutInputDto {
            branch_ids: vec!["foreign-branch".into()],
            target_build_version: "boothy-2026.03.27.1".into(),
            target_preset_stack_version: "catalog-2026.03.27".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("typed rejection should still succeed");

    assert!(result
        .outcomes
        .iter()
        .all(|outcome| outcome.result == "rejected"));

    let history_path = base_dir.join("branch-config").join("rollout-history.json");
    let history_bytes = fs::read_to_string(history_path).expect("history file should exist");
    let history: serde_json::Value =
        serde_json::from_str(&history_bytes).expect("history should deserialize");
    let entries = history["entries"]
        .as_array()
        .expect("entries should be an array");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["outcomes"][0]["result"], "rejected");

    let operator_audit_path = base_dir.join("diagnostics").join("operator-audit-log.json");
    let operator_audit_bytes =
        fs::read_to_string(operator_audit_path).expect("operator audit should exist");
    let operator_audit: serde_json::Value =
        serde_json::from_str(&operator_audit_bytes).expect("operator audit should deserialize");
    let audit_entries = operator_audit["entries"]
        .as_array()
        .expect("operator audit entries should be an array");
    let release_audit = audit_entries
        .iter()
        .find(|entry| entry["eventCategory"] == "release-governance")
        .expect("release-governance audit should exist");

    assert!(release_audit["sessionId"].is_null());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn malformed_release_baseline_is_rejected_before_mutation() {
    let base_dir = unique_test_root("invalid-rollout-input");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_branch_store(&base_dir, false);

    let result = apply_branch_rollout_in_dir(
        &base_dir,
        &capability_snapshot,
        BranchRolloutInputDto {
            branch_ids: vec!["gangnam-01".into()],
            target_build_version: "boothy-2026.99.bad".into(),
            target_preset_stack_version: "catalog-2026.03.27".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    );

    assert!(result.is_err());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_promotion_and_rollback_are_host_owned_and_auditable() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-audit");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_002",
        "capture_20260412_002",
    );
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        go_canary_assessment(
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "request_20260412_002",
            "capture_20260412_002",
        ),
    );

    let promote_canary = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "canary".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("canary promotion should succeed");
    assert_eq!(promote_canary.route_stage, "canary");

    let promote_default = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("default promotion should succeed after repeated canary evidence");
    assert_eq!(promote_default.route_stage, "default");
    assert_eq!(
        promote_default.decision_summary.decision_stage.as_deref(),
        Some("default")
    );
    let promoted_policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            base_dir
                .join("branch-config")
                .join("preview-renderer-policy.json"),
        )
        .expect("policy should exist after default promotion"),
    )
    .expect("policy should deserialize after default promotion");
    assert_eq!(promoted_policy["defaultRoute"], "darktable");
    assert_eq!(
        promoted_policy["defaultRoutes"][0]["presetId"],
        "preset_soft-glow"
    );
    assert_eq!(
        promoted_policy["defaultRoutes"][0]["presetVersion"],
        "2026.04.10"
    );
    assert_eq!(
        promoted_policy["defaultRoutes"][0]["route"],
        "actual-primary-lane"
    );

    let rollback = rollback_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRouteRollbackInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("rollback should remain a one-action host-owned path");
    assert_eq!(rollback.route_stage, "shadow");
    assert_eq!(
        rollback.decision_summary.decision_stage.as_deref(),
        Some("rollback")
    );
    assert_eq!(
        rollback.decision_summary.implementation_track.as_deref(),
        Some("prototype-track")
    );
    assert_eq!(rollback.decision_summary.lane_owner, "inline-truthful-fallback");
    assert_eq!(rollback.decision_summary.fallback_reason, None);

    let policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            base_dir
                .join("branch-config")
                .join("preview-renderer-policy.json"),
        )
        .expect("policy should exist"),
    )
    .expect("policy should deserialize");
    assert_eq!(policy["defaultRoute"], "darktable");
    assert!(policy["defaultRoutes"]
        .as_array()
        .expect("defaultRoutes should be an array")
        .is_empty());
    assert_eq!(
        policy["forcedFallbackRoutes"][0]["presetId"],
        "preset_soft-glow"
    );

    let history: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            base_dir
                .join("branch-config")
                .join("preview-renderer-policy-history.json"),
        )
        .expect("preview route policy history should exist"),
    )
    .expect("preview route policy history should deserialize");
    let entries = history["entries"]
        .as_array()
        .expect("entries should be an array");
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0]["action"], "promote");
    assert_eq!(entries[1]["action"], "promote");
    assert_eq!(entries[2]["action"], "rollback");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_without_repeated_canary_success_path() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-default-gate");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("default promotion should require repeated canary success-path evidence");

    assert_eq!(error.code, "validation-error");
    let history: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            base_dir
                .join("branch-config")
                .join("preview-renderer-policy-history.json"),
        )
        .expect("preview route policy history should exist"),
    )
    .expect("preview route policy history should deserialize");
    let entries = history["entries"]
        .as_array()
        .expect("entries should be an array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["result"], "rejected");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_duplicate_evidence_for_the_same_capture() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-duplicate-evidence");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("duplicate evidence from one capture should not unlock default promotion");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_without_typed_go_canary_assessment() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-missing-canary-assessment");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("default promotion should fail closed without a typed canary Go verdict");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("actual primary lane"));
    assert!(error.message.contains("typed canary Go verdict"));
    assert!(!error.message.contains("Story 1.24"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_prototype_track_evidence_even_with_go_assessment() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-prototype-track-evidence");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:00:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "requestId": "request_20260412_001",
            "captureId": "capture_20260412_001",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 1605,
            "sameCaptureFullScreenVisibleMs": 2410,
            "replacementMs": 2410,
            "originalVisibleToPresetAppliedVisibleMs": 805,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/sessions/session/diagnostics/captured-preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/sessions/session/diagnostics/captured-catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture.jpg",
            "warmStateDetailPath": null,
            "implementationTrack": "prototype-track"
        }),
    );
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:02:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "requestId": "request_20260412_002",
            "captureId": "capture_20260412_002",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "warmState": "warm-hit",
            "firstVisibleMs": 1580,
            "sameCaptureFullScreenVisibleMs": 2395,
            "replacementMs": 2395,
            "originalVisibleToPresetAppliedVisibleMs": 815,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/sessions/session/diagnostics/captured-preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/sessions/session/diagnostics/captured-catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture-2.jpg",
            "warmStateDetailPath": null,
            "implementationTrack": "prototype-track"
        }),
    );
    let mut assessment = go_canary_assessment(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    assessment["laneOwner"] = serde_json::json!("dedicated-renderer");
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        assessment,
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("prototype-track evidence should not unlock actual-lane default promotion");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_prototype_track_assessment_even_with_actual_evidence() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-prototype-track-assessment");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    let mut assessment = go_canary_assessment(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    assessment["implementationTrack"] = serde_json::json!("prototype-track");
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        assessment,
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("prototype-track assessment should not unlock actual-lane default promotion");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_legacy_dedicated_renderer_owner_even_with_actual_track(
) {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-legacy-owner");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:00:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "requestId": "request_20260412_001",
            "captureId": "capture_20260412_001",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "dedicated-renderer",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 1605,
            "sameCaptureFullScreenVisibleMs": 2410,
            "originalVisibleToPresetAppliedVisibleMs": 805,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/branch-config/preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/preset-catalog/catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture.jpg",
            "warmStateDetailPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warm-state.json",
            "improvementSummary": "promotionGateTargetMs=2500"
        }),
    );
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:02:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "requestId": "request_20260412_002",
            "captureId": "capture_20260412_002",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "dedicated-renderer",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-hit",
            "firstVisibleMs": 1580,
            "sameCaptureFullScreenVisibleMs": 2395,
            "originalVisibleToPresetAppliedVisibleMs": 815,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/branch-config/preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/preset-catalog/catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture-2.jpg",
            "warmStateDetailPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warm-state.json",
            "improvementSummary": "promotionGateTargetMs=2500"
        }),
    );
    let mut assessment = go_canary_assessment(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    assessment["laneOwner"] = serde_json::json!("dedicated-renderer");
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        assessment,
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("legacy dedicated-renderer owner should not unlock actual-lane default promotion");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_when_latest_canary_assessment_is_no_go() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-no-go-canary-assessment");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        serde_json::json!({
            "schemaVersion": "preview-promotion-canary-assessment/v1",
            "generatedAt": "2026-04-12T08:10:00.000Z",
            "bundleManifestPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/preview-promotion-evidence-bundle.json",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "captureId": "capture_20260412_002",
            "requestId": "request_20260412_002",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "routeStage": "canary",
            "laneOwner": "local-fullscreen-lane",
            "gate": "No-Go",
            "implementationTrack": "actual-primary-lane",
            "nextStageAllowed": false,
            "summary": "rollback proof missing keeps the canary at No-Go.",
            "blockers": ["rollback-proof-missing"],
            "checks": {
                "kpi": {
                    "status": "pass",
                    "reason": "same-capture KPI satisfied",
                    "actualMs": 2410,
                    "thresholdMs": 2500
                },
                "fallbackStability": {
                    "status": "pass",
                    "reason": "fallback ratio is within the approved bound",
                    "actualRatio": 0.1,
                    "thresholdRatio": 0.2
                },
                "wrongCapture": {
                    "status": "pass",
                    "reason": "same selected capture chain preserved"
                },
                "fidelityDrift": {
                    "status": "pass",
                    "reason": "parity stayed inside the approved bound",
                    "parityResult": "pass"
                },
                "rollbackReadiness": {
                    "status": "fail",
                    "reason": "rollback proof missing",
                    "evidenceCount": 0
                },
                "activeSessionSafety": {
                    "status": "pass",
                    "reason": "capture-time route snapshot preserved"
                }
            }
        }),
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("default promotion should fail closed when the latest typed canary verdict is No-Go");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("No-Go") || error.message.contains("rollback"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_when_fallback_stability_fails_even_with_go_gate() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-fallback-stability-fails");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    let mut assessment = go_canary_assessment(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    assessment["checks"]["fallbackStability"]["status"] = serde_json::json!("fail");
    assessment["checks"]["fallbackStability"]["reason"] =
        serde_json::json!("fallback ratio exceeded the approved threshold");
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        assessment,
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("default promotion should reject fallback-heavy typed canary evidence");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("fallback") || error.message.contains("안정성"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_when_latest_assessment_is_not_for_the_selected_capture_chain() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-mismatched-canary-assessment");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        "request_20260412_002",
        "capture_20260412_002",
    );
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1z",
        go_canary_assessment(
            "session_01hs6n1r8b8zc5v4ey2x7b9g1z",
            "request_20260412_099",
            "capture_20260412_099",
        ),
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("default promotion should reject typed canary verdicts that are detached from the selected capture chain");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("selected capture") || error.message.contains("evidence chain"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_status_reports_canary_for_a_promoted_preset_version() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-status");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);

    let promote_canary = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "canary".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("canary promotion should succeed");
    assert_eq!(promote_canary.route_stage, "canary");

    let status = load_preview_renderer_route_status_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRouteStatusInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("status should load");

    assert_eq!(status.route_stage, "canary");
    assert_eq!(status.resolved_route, "actual-primary-lane");
    assert_eq!(status.message, "이 프리셋 버전은 canary 상태예요.");
    assert_eq!(status.decision_summary.decision_stage, None);
    assert_eq!(status.decision_summary.fallback_reason, None);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_status_keeps_legacy_policy_route_visible_until_the_policy_is_updated() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-legacy-status");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    let policy_path = base_dir
        .join("branch-config")
        .join("preview-renderer-policy.json");
    let mut policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&policy_path).expect("policy should exist"),
    )
    .expect("policy should deserialize");
    policy["canaryRoutes"] = serde_json::json!([
        {
            "route": "local-renderer-sidecar",
            "presetId": "preset_soft-glow",
            "presetVersion": "2026.04.10",
            "reason": "host-approved-canary"
        }
    ]);
    fs::write(
        &policy_path,
        serde_json::to_string_pretty(&policy).expect("policy should serialize"),
    )
    .expect("policy should persist");

    let status = load_preview_renderer_route_status_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRouteStatusInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("status should load");

    assert_eq!(status.route_stage, "canary");
    assert_eq!(status.resolved_route, "local-renderer-sidecar");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_promotion_rolls_back_when_history_persist_fails() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-history-failure");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);
    let _failure_guard = ScopedEnvVarGuard::set(
        "BOOTHY_TEST_PREVIEW_ROUTE_POLICY_HISTORY_WRITE_FAILURE",
        "true",
    );

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "canary".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("history persist failure should reject the route mutation");
    assert_eq!(error.code, "session-persistence-failed");

    let policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            base_dir
                .join("branch-config")
                .join("preview-renderer-policy.json"),
        )
        .expect("policy should still exist"),
    )
    .expect("policy should deserialize after rollback");
    assert_eq!(policy["defaultRoute"], "darktable");
    assert!(
        policy["canaryRoutes"]
            .as_array()
            .expect("canaryRoutes should be an array")
            .is_empty(),
        "failed mutation must not leave a promoted canary route behind"
    );

    let history_path = base_dir
        .join("branch-config")
        .join("preview-renderer-policy-history.json");
    assert!(
        !history_path.exists(),
        "failed mutation must not leave a partial history file behind"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_rejects_records_missing_snapshot_evidence() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-incomplete-evidence");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        "request_20260412_001",
        "capture_20260412_001",
    );
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:01:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "requestId": "request_20260412_002",
            "captureId": "capture_20260412_002",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 2815,
            "replacementMs": 3610,
            "originalVisibleToPresetAppliedVisibleMs": 795,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/sessions/session/diagnostics/captured-catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture.jpg",
            "warmStateDetailPath": null
        }),
    );

    let error = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect_err("default promotion should reject incomplete success-path evidence");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_does_not_add_a_budget_gate_in_story_1_21() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-over-budget-evidence");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:00:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "requestId": "request_20260412_001",
            "captureId": "capture_20260412_001",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 1810,
            "replacementMs": 2495,
            "originalVisibleToPresetAppliedVisibleMs": 685,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/branch-config/preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/preset-catalog/catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture.jpg",
            "warmStateDetailPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warm-state.json",
            "improvementSummary": "promotionGateTargetMs=2500"
        }),
    );
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:01:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "requestId": "request_20260412_002",
            "captureId": "capture_20260412_002",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 1890,
            "replacementMs": 3610,
            "originalVisibleToPresetAppliedVisibleMs": 1720,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/branch-config/preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/preset-catalog/catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture-over-budget.jpg",
            "warmStateDetailPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warm-state.json",
            "improvementSummary": "promotionGateTargetMs=2500"
        }),
    );
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        go_canary_assessment(
            "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "request_20260412_002",
            "capture_20260412_002",
        ),
    );

    let promoted = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("story 1.21 should not change default-promotion gating behavior");
    assert_eq!(promoted.route_stage, "default");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn preview_route_default_promotion_accepts_canonical_full_screen_metric_without_legacy_alias() {
    let _env_lock = lock_branch_rollout_test_env();
    let base_dir = unique_test_root("preview-route-policy-canonical-full-screen-evidence");
    let capability_snapshot = capability_snapshot_for_profile("operator-enabled", true);

    seed_preview_renderer_policy(&base_dir);
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:00:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "requestId": "request_20260412_001",
            "captureId": "capture_20260412_001",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 1605,
            "sameCaptureFullScreenVisibleMs": 2410,
            "originalVisibleToPresetAppliedVisibleMs": 805,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/branch-config/preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/preset-catalog/catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture.jpg",
            "warmStateDetailPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warm-state.json",
            "improvementSummary": "promotionGateTargetMs=2500"
        }),
    );
    seed_preview_promotion_evidence_line(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:02:15+09:00",
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "requestId": "request_20260412_002",
            "captureId": "capture_20260412_002",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-hit",
            "firstVisibleMs": 1580,
            "sameCaptureFullScreenVisibleMs": 2395,
            "originalVisibleToPresetAppliedVisibleMs": 815,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/branch-config/preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/preset-catalog/catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture-2.jpg",
            "warmStateDetailPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warm-state.json",
            "improvementSummary": "promotionGateTargetMs=2500"
        }),
    );
    seed_preview_promotion_canary_assessment(
        &base_dir,
        "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
        go_canary_assessment(
            "session_01hs6n1r8b8zc5v4ey2x7b9g1n",
            "request_20260412_002",
            "capture_20260412_002",
        ),
    );

    let promoted = promote_preview_renderer_route_in_dir(
        &base_dir,
        &capability_snapshot,
        PreviewRendererRoutePromotionInputDto {
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            target_route_stage: "default".into(),
            actor_id: "release-kim".into(),
            actor_label: "Kim Release".into(),
        },
    )
    .expect("canonical metric evidence should satisfy repeated canary success path");
    assert_eq!(promoted.route_stage, "default");
    assert_eq!(
        promoted.decision_summary.implementation_track.as_deref(),
        Some("actual-primary-lane")
    );
    assert_eq!(promoted.decision_summary.lane_owner, "local-fullscreen-lane");
    assert_eq!(promoted.decision_summary.canary_gate.as_deref(), Some("Go"));
    assert_eq!(promoted.decision_summary.kpi_status.as_deref(), Some("pass"));
    assert!(promoted.decision_summary.rollback_proof_present);
    assert!(promoted.decision_summary.blockers.is_empty());

    let _ = fs::remove_dir_all(base_dir);
}

fn seed_branch_store(base_dir: &Path, with_active_session: bool) {
    let branch_config_dir = base_dir.join("branch-config");
    fs::create_dir_all(&branch_config_dir).expect("branch config directory should exist");

    let active_session = if with_active_session {
        serde_json::json!({
            "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "lockedBaseline": {
                "buildVersion": "boothy-2026.03.20.4",
                "presetStackVersion": "catalog-2026.03.20",
                "approvedAt": "2026-03-20T00:10:00.000Z",
                "actorId": "release-kim",
                "actorLabel": "Kim Release"
            },
            "startedAt": "2026-03-27T00:00:00.000Z",
            "safeTransition": "after-session-end"
        })
    } else {
        serde_json::Value::Null
    };

    let store = serde_json::json!({
        "schemaVersion": "branch-rollout-store/v1",
        "approvedBaselines": [
            {
                "buildVersion": "boothy-2026.03.20.4",
                "presetStackVersion": "catalog-2026.03.20",
                "approvedAt": "2026-03-20T00:10:00.000Z",
                "actorId": "release-kim",
                "actorLabel": "Kim Release"
            },
            {
                "buildVersion": "boothy-2026.03.27.1",
                "presetStackVersion": "catalog-2026.03.27",
                "approvedAt": "2026-03-27T00:10:00.000Z",
                "actorId": "release-kim",
                "actorLabel": "Kim Release"
            }
        ],
        "branches": [
            {
                "branchId": "gangnam-01",
                "displayName": "강남 1호점",
                "deploymentBaseline": {
                    "buildVersion": "boothy-2026.03.20.4",
                    "presetStackVersion": "catalog-2026.03.20",
                    "approvedAt": "2026-03-20T00:10:00.000Z",
                    "actorId": "release-kim",
                    "actorLabel": "Kim Release"
                },
                "rollbackBaseline": {
                    "buildVersion": "boothy-2026.03.13.2",
                    "presetStackVersion": "catalog-2026.03.13",
                    "approvedAt": "2026-03-13T00:10:00.000Z",
                    "actorId": "release-kim",
                    "actorLabel": "Kim Release"
                },
                "pendingBaseline": null,
                "localSettings": {
                    "contactPhone": "02-555-0101",
                    "contactEmail": "gangnam@boothy.local",
                    "operationalToggles": ["queue-badge-enabled"]
                },
                "activeSession": null
            },
            {
                "branchId": "hongdae-02",
                "displayName": "홍대 2호점",
                "deploymentBaseline": {
                    "buildVersion": "boothy-2026.03.20.4",
                    "presetStackVersion": "catalog-2026.03.20",
                    "approvedAt": "2026-03-20T00:10:00.000Z",
                    "actorId": "release-kim",
                    "actorLabel": "Kim Release"
                },
                "rollbackBaseline": {
                    "buildVersion": "boothy-2026.03.13.2",
                    "presetStackVersion": "catalog-2026.03.13",
                    "approvedAt": "2026-03-13T00:10:00.000Z",
                    "actorId": "release-kim",
                    "actorLabel": "Kim Release"
                },
                "pendingBaseline": null,
                "localSettings": {
                    "contactPhone": "02-555-0102",
                    "contactEmail": "hongdae@boothy.local",
                    "operationalToggles": ["queue-badge-enabled"]
                },
                "activeSession": active_session
            },
            {
                "branchId": "itaewon-03",
                "displayName": "이태원 3호점",
                "deploymentBaseline": {
                    "buildVersion": "boothy-2026.03.20.4",
                    "presetStackVersion": "catalog-2026.03.20",
                    "approvedAt": "2026-03-20T00:10:00.000Z",
                    "actorId": "release-kim",
                    "actorLabel": "Kim Release"
                },
                "rollbackBaseline": null,
                "pendingBaseline": null,
                "localSettings": {
                    "contactPhone": "02-555-0103",
                    "contactEmail": "itaewon@boothy.local",
                    "operationalToggles": []
                },
                "activeSession": null
            }
        ]
    });

    fs::write(
        branch_config_dir.join("state.json"),
        serde_json::to_vec_pretty(&store).expect("store should serialize"),
    )
    .expect("store should write");
}

fn seed_active_session_manifest(
    base_dir: &Path,
    session_id: &str,
    timing_phase: &str,
    post_end_state: Option<&str>,
) {
    let session_root = base_dir.join("sessions").join(session_id);
    fs::create_dir_all(&session_root).expect("session root should exist");

    let manifest = serde_json::json!({
        "schemaVersion": "session-manifest/v1",
        "sessionId": session_id,
        "boothAlias": "Kim 4821",
        "customer": {
            "name": "Kim",
            "phoneLastFour": "4821"
        },
        "createdAt": "2026-03-27T00:00:00Z",
        "updatedAt": "2026-03-27T00:00:00Z",
        "lifecycle": {
            "status": "active",
            "stage": if timing_phase == "ended" { "ended" } else { "capture-ready" }
        },
        "activePreset": null,
        "activePresetId": null,
        "activePresetDisplayName": null,
        "timing": {
            "schemaVersion": "session-timing/v1",
            "sessionId": session_id,
            "adjustedEndAt": if timing_phase == "ended" { "2026-03-27T00:00:00Z" } else { "2099-03-27T00:00:00Z" },
            "warningAt": "2026-03-26T23:55:00Z",
            "phase": timing_phase,
            "captureAllowed": timing_phase != "ended",
            "approvedExtensionMinutes": 0,
            "approvedExtensionAuditRef": null,
            "warningTriggeredAt": null,
            "endedTriggeredAt": if timing_phase == "ended" {
                serde_json::json!("2026-03-27T00:00:00Z")
            } else {
                serde_json::Value::Null
            }
        },
        "captures": [],
        "postEnd": match post_end_state {
            Some("completed") => serde_json::json!({
                "state": "completed",
                "evaluatedAt": "2026-03-27T00:00:01Z",
                "completionVariant": "local-deliverable-ready",
                "approvedRecipientLabel": serde_json::Value::Null,
                "nextLocationLabel": serde_json::Value::Null,
                "primaryActionLabel": "안내가 끝났어요. 천천히 이동해 주세요.",
                "supportActionLabel": serde_json::Value::Null,
                "showBoothAlias": false
            }),
            Some("phone-required") => serde_json::json!({
                "state": "phone-required",
                "evaluatedAt": "2026-03-27T00:00:01Z",
                "primaryActionLabel": "가까운 직원에게 알려 주세요.",
                "supportActionLabel": "직원에게 도움을 요청해 주세요.",
                "unsafeActionWarning": "다시 찍기나 기기 조작은 잠시 멈춰 주세요.",
                "showBoothAlias": false
            }),
            _ => serde_json::Value::Null
        }
    });

    fs::write(
        session_root.join("session.json"),
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");
}

fn seed_preview_renderer_policy(base_dir: &Path) {
    let policy_path = base_dir
        .join("branch-config")
        .join("preview-renderer-policy.json");
    fs::create_dir_all(
        policy_path
            .parent()
            .expect("policy path should have parent"),
    )
    .expect("policy directory should exist");
    fs::write(
        policy_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "preview-renderer-route-policy/v1",
            "defaultRoute": "darktable",
            "defaultRoutes": [],
            "canaryRoutes": [],
            "forcedFallbackRoutes": []
        }))
        .expect("policy should serialize"),
    )
    .expect("policy should write");
}

fn seed_preview_promotion_evidence(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
) {
    seed_preview_promotion_evidence_line(
        base_dir,
        session_id,
        serde_json::json!({
            "schemaVersion": "preview-promotion-evidence-record/v1",
            "observedAt": "2026-04-12T08:00:15+09:00",
            "sessionId": session_id,
            "requestId": request_id,
            "captureId": capture_id,
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "laneOwner": "local-fullscreen-lane",
            "fallbackReasonCode": null,
            "routeStage": "canary",
            "implementationTrack": "actual-primary-lane",
            "warmState": "warm-ready",
            "firstVisibleMs": 1605,
            "replacementMs": 2410,
            "originalVisibleToPresetAppliedVisibleMs": 805,
            "sessionManifestPath": "C:/boothy/sessions/session/session.json",
            "timingEventsPath": "C:/boothy/sessions/session/diagnostics/timing-events.log",
            "routePolicySnapshotPath": "C:/boothy/sessions/session/diagnostics/captured-preview-renderer-policy.json",
            "publishedBundlePath": "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/bundle.json",
            "catalogStatePath": "C:/boothy/sessions/session/diagnostics/captured-catalog-state.json",
            "previewAssetPath": "C:/boothy/sessions/session/renders/previews/capture.jpg",
            "warmStateDetailPath": null
        }),
    );
}

fn seed_preview_promotion_evidence_line(
    base_dir: &Path,
    session_id: &str,
    line: serde_json::Value,
) {
    let diagnostics_dir = base_dir
        .join("sessions")
        .join(session_id)
        .join("diagnostics")
        .join("dedicated-renderer");
    fs::create_dir_all(&diagnostics_dir).expect("diagnostics directory should exist");

    let evidence_path = diagnostics_dir.join("preview-promotion-evidence.jsonl");
    let line = serde_json::to_string(&line).expect("evidence record should serialize");
    let existing = fs::read_to_string(&evidence_path).unwrap_or_default();
    let next = if existing.trim().is_empty() {
        format!("{line}\n")
    } else {
        format!("{existing}{line}\n")
    };
    fs::write(evidence_path, next).expect("evidence should write");
}

fn seed_preview_promotion_canary_assessment(
    base_dir: &Path,
    session_id: &str,
    assessment: serde_json::Value,
) {
    let diagnostics_dir = base_dir
        .join("sessions")
        .join(session_id)
        .join("diagnostics")
        .join("dedicated-renderer");
    fs::create_dir_all(&diagnostics_dir).expect("diagnostics directory should exist");
    fs::write(
        diagnostics_dir.join("preview-promotion-canary-assessment.json"),
        serde_json::to_vec_pretty(&assessment).expect("assessment should serialize"),
    )
    .expect("assessment should write");
}

fn go_canary_assessment(session_id: &str, request_id: &str, capture_id: &str) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": "preview-promotion-canary-assessment/v1",
        "generatedAt": "2026-04-12T08:10:00.000Z",
        "bundleManifestPath": "C:/boothy/sessions/session/diagnostics/dedicated-renderer/preview-promotion-evidence-bundle.json",
        "sessionId": session_id,
        "captureId": capture_id,
        "requestId": request_id,
        "presetId": "preset_soft-glow",
        "publishedVersion": "2026.04.10",
        "routeStage": "canary",
        "laneOwner": "local-fullscreen-lane",
        "implementationTrack": "actual-primary-lane",
        "gate": "Go",
        "nextStageAllowed": true,
        "summary": "same-capture KPI, rollback proof, and active-session safety passed.",
        "blockers": [],
        "checks": {
            "kpi": {
                "status": "pass",
                "reason": "same-capture KPI satisfied",
                "actualMs": 2410,
                "thresholdMs": 2500
            },
            "fallbackStability": {
                "status": "pass",
                "reason": "fallback ratio is within the approved bound",
                "actualRatio": 0.1,
                "thresholdRatio": 0.2
            },
            "wrongCapture": {
                "status": "pass",
                "reason": "same selected capture chain preserved"
            },
            "fidelityDrift": {
                "status": "pass",
                "reason": "parity stayed inside the approved bound",
                "parityResult": "pass"
            },
            "rollbackReadiness": {
                "status": "pass",
                "reason": "rollback proof bundle preserved",
                "evidenceCount": 1
            },
            "activeSessionSafety": {
                "status": "pass",
                "reason": "capture-time route snapshot preserved"
            }
        }
    })
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
