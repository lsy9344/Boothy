use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    branch_config::{
        apply_branch_rollback_in_dir, apply_branch_rollout_in_dir,
        load_branch_rollout_overview_in_dir,
    },
    commands::runtime_commands::capability_snapshot_for_profile,
    contracts::dto::{BranchRollbackInputDto, BranchRolloutInputDto},
};

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-branch-rollout-{test_name}-{stamp}"))
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
