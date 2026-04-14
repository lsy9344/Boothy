use std::{
    collections::HashSet,
    env, fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

use crate::{
    contracts::dto::{
        validate_branch_rollback_input, validate_branch_rollout_input,
        validate_preview_renderer_route_promotion_input,
        validate_preview_renderer_route_rollback_input,
        validate_preview_renderer_route_status_input, BranchActiveSessionDto,
        BranchCompatibilityVerdictDto, BranchLocalSettingsPreservationDto,
        BranchReleaseBaselineDto, BranchRollbackInputDto, BranchRolloutActionResultDto,
        BranchRolloutApprovalDto, BranchRolloutAuditEntryDto, BranchRolloutBranchResultDto,
        BranchRolloutBranchStateDto, BranchRolloutInputDto, BranchRolloutOverviewResultDto,
        BranchRolloutRejectionDto, CapabilitySnapshotDto, HostErrorEnvelope,
        PreviewRendererRouteMutationResultDto, PreviewRendererRoutePolicyAuditEntryDto,
        PreviewRendererRoutePromotionInputDto, PreviewRendererRouteRollbackInputDto,
        PreviewRendererRouteStatusInputDto, PreviewRendererRouteStatusResultDto,
    },
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    handoff::sync_post_end_state_in_dir,
    session::{
        session_manifest::current_timestamp, session_paths::SessionPaths,
        session_repository::read_session_manifest,
    },
    timing::sync_session_timing_in_dir,
};

const BRANCH_ROLLOUT_STORE_SCHEMA_VERSION: &str = "branch-rollout-store/v1";
const BRANCH_ROLLOUT_AUDIT_ENTRY_SCHEMA_VERSION: &str = "branch-rollout-audit-entry/v1";
const BRANCH_ROLLOUT_HISTORY_STORE_SCHEMA_VERSION: &str = "branch-rollout-history-store/v1";
const BRANCH_ROLLOUT_OVERVIEW_SCHEMA_VERSION: &str = "branch-rollout-overview/v1";
const BRANCH_ROLLOUT_ACTION_RESULT_SCHEMA_VERSION: &str = "branch-rollout-action-result/v1";
const PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION: &str = "preview-renderer-route-policy/v1";
const PREVIEW_RENDERER_ROUTE_POLICY_AUDIT_ENTRY_SCHEMA_VERSION: &str =
    "preview-renderer-route-policy-audit-entry/v1";
const PREVIEW_RENDERER_ROUTE_POLICY_HISTORY_SCHEMA_VERSION: &str =
    "preview-renderer-route-policy-history/v1";
const PREVIEW_RENDERER_ROUTE_POLICY_HISTORY_WRITE_FAILURE_ENV: &str =
    "BOOTHY_TEST_PREVIEW_ROUTE_POLICY_HISTORY_WRITE_FAILURE";
const BRANCH_ROLLOUT_LOCK_RETRY_DELAY_MS: u64 = 10;
const BRANCH_ROLLOUT_LOCK_MAX_ATTEMPTS: u32 = 500;
const BRANCH_ROLLOUT_LOCK_STALE_AFTER_MS: u64 = 30_000;
const PREVIEW_RENDERER_ROUTE_DARKTABLE: &str = "darktable";
const PREVIEW_RENDERER_ROUTE_LOCAL_SIDECAR: &str = "local-renderer-sidecar";

static BRANCH_ROLLOUT_AUDIT_COUNTER: AtomicU64 = AtomicU64::new(0);
static PREVIEW_RENDERER_ROUTE_POLICY_AUDIT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchRolloutStore {
    schema_version: String,
    #[serde(default)]
    approved_baselines: Vec<BranchReleaseBaselineDto>,
    #[serde(default)]
    branches: Vec<BranchStateRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchStateRecord {
    branch_id: String,
    display_name: String,
    deployment_baseline: BranchReleaseBaselineDto,
    rollback_baseline: Option<BranchReleaseBaselineDto>,
    pending_baseline: Option<BranchReleaseBaselineDto>,
    local_settings: BranchLocalSettingsRecord,
    active_session: Option<BranchActiveSessionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchLocalSettingsRecord {
    #[serde(default)]
    contact_phone: Option<String>,
    #[serde(default)]
    contact_email: Option<String>,
    #[serde(default)]
    contact_kakao: Option<String>,
    #[serde(default)]
    support_hours: Option<String>,
    #[serde(default)]
    operational_toggles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchRolloutHistoryStore {
    schema_version: String,
    #[serde(default)]
    entries: Vec<BranchRolloutAuditEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRendererRoutePolicyStore {
    schema_version: String,
    default_route: String,
    #[serde(default)]
    default_routes: Vec<PreviewRendererRoutePolicyRule>,
    #[serde(default)]
    canary_routes: Vec<PreviewRendererRoutePolicyRule>,
    #[serde(default)]
    forced_fallback_routes: Vec<PreviewRendererRoutePolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRendererRoutePolicyRule {
    route: String,
    preset_id: String,
    preset_version: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRendererRoutePolicyHistoryStore {
    schema_version: String,
    #[serde(default)]
    entries: Vec<PreviewRendererRoutePolicyAuditEntryDto>,
}

struct BranchRolloutStoreLock {
    lock_path: PathBuf,
}

impl Drop for BranchRolloutStoreLock {
    fn drop(&mut self) {
        if self.lock_path.exists() {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

pub fn load_branch_rollout_overview_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<BranchRolloutOverviewResultDto, HostErrorEnvelope> {
    ensure_settings_access(capability_snapshot)?;
    let _lock = acquire_branch_rollout_store_lock(base_dir)?;

    let mut store = load_branch_rollout_store(base_dir)?;
    if sync_branch_runtime_state(base_dir, &mut store)? {
        persist_branch_rollout_store(base_dir, &store)?;
    }

    Ok(BranchRolloutOverviewResultDto {
        schema_version: BRANCH_ROLLOUT_OVERVIEW_SCHEMA_VERSION.into(),
        approved_baselines: store.approved_baselines,
        branches: store.branches.iter().map(build_branch_state_dto).collect(),
        recent_history: load_branch_rollout_history(base_dir)?
            .into_iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
    })
}

pub fn apply_branch_rollout_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: BranchRolloutInputDto,
) -> Result<BranchRolloutActionResultDto, HostErrorEnvelope> {
    ensure_settings_access(capability_snapshot)?;
    validate_branch_rollout_input(&input)?;

    let approval = BranchRolloutApprovalDto {
        approved_at: current_timestamp(SystemTime::now())?,
        actor_id: input.actor_id.clone(),
        actor_label: input.actor_label.clone(),
    };
    let target_baseline = BranchReleaseBaselineDto {
        build_version: input.target_build_version.clone(),
        preset_stack_version: input.target_preset_stack_version.clone(),
        approved_at: approval.approved_at.clone(),
        actor_id: input.actor_id.clone(),
        actor_label: input.actor_label.clone(),
    };

    apply_action(
        base_dir,
        "rollout",
        &input.branch_ids,
        Some(target_baseline),
        approval,
    )
}

pub fn apply_branch_rollback_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: BranchRollbackInputDto,
) -> Result<BranchRolloutActionResultDto, HostErrorEnvelope> {
    ensure_settings_access(capability_snapshot)?;
    validate_branch_rollback_input(&input)?;

    let approval = BranchRolloutApprovalDto {
        approved_at: current_timestamp(SystemTime::now())?,
        actor_id: input.actor_id.clone(),
        actor_label: input.actor_label.clone(),
    };

    apply_action(base_dir, "rollback", &input.branch_ids, None, approval)
}

pub fn promote_preview_renderer_route_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: PreviewRendererRoutePromotionInputDto,
) -> Result<PreviewRendererRouteMutationResultDto, HostErrorEnvelope> {
    ensure_settings_access(capability_snapshot)?;
    validate_preview_renderer_route_promotion_input(&input)?;

    let approval = BranchRolloutApprovalDto {
        approved_at: current_timestamp(SystemTime::now())?,
        actor_id: input.actor_id.clone(),
        actor_label: input.actor_label.clone(),
    };
    let _lock = acquire_branch_rollout_store_lock(base_dir)?;
    let mut policy = load_preview_renderer_route_policy(base_dir)?;
    let mut history = load_preview_renderer_route_policy_history(base_dir)?;
    let canary_success_count =
        count_repeated_canary_success_path(base_dir, &input.preset_id, &input.published_version)?;
    let audit_entry = build_preview_renderer_route_policy_audit_entry(
        "promote",
        &input.preset_id,
        &input.published_version,
        &input.target_route_stage,
        &approval,
        if input.target_route_stage == "default" && canary_success_count < 2 {
            "rejected"
        } else {
            "applied"
        },
        canary_success_count,
    )?;

    if input.target_route_stage == "default" && canary_success_count < 2 {
        history.entries.push(audit_entry.clone());
        persist_preview_renderer_route_policy_history(base_dir, &history.entries)?;
        append_preview_renderer_route_policy_audit_record(
            base_dir,
            "promote",
            &audit_entry,
            "repeated canary success-path evidence가 부족해 default 승격을 적용하지 않았어요.",
            Some("preview-route-default-evidence-missing"),
        );
        return Err(HostErrorEnvelope::validation_message(
            "반복된 canary success-path evidence를 먼저 확보해 주세요.",
        ));
    }

    history.entries.push(audit_entry.clone());
    upsert_preview_route_policy_for_promotion(&mut policy, &input);
    persist_preview_renderer_route_artifacts(base_dir, &policy, &history.entries)?;
    append_preview_renderer_route_policy_audit_record(
        base_dir,
        "promote",
        &audit_entry,
        if input.target_route_stage == "default" {
            "반복된 canary success-path evidence를 확인하고 default route를 승격했어요."
        } else {
            "승인된 preset/version scope를 canary route로 승격했어요."
        },
        None,
    );

    Ok(PreviewRendererRouteMutationResultDto {
        schema_version: PREVIEW_RENDERER_ROUTE_POLICY_AUDIT_ENTRY_SCHEMA_VERSION.into(),
        action: "promote".into(),
        preset_id: input.preset_id,
        published_version: input.published_version,
        route_stage: input.target_route_stage,
        approval,
        audit_entry,
        message: "preview route policy 승격을 기록했어요.".into(),
    })
}

pub fn rollback_preview_renderer_route_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: PreviewRendererRouteRollbackInputDto,
) -> Result<PreviewRendererRouteMutationResultDto, HostErrorEnvelope> {
    ensure_settings_access(capability_snapshot)?;
    validate_preview_renderer_route_rollback_input(&input)?;

    let approval = BranchRolloutApprovalDto {
        approved_at: current_timestamp(SystemTime::now())?,
        actor_id: input.actor_id.clone(),
        actor_label: input.actor_label.clone(),
    };
    let _lock = acquire_branch_rollout_store_lock(base_dir)?;
    let mut policy = load_preview_renderer_route_policy(base_dir)?;
    let mut history = load_preview_renderer_route_policy_history(base_dir)?;
    let canary_success_count =
        count_repeated_canary_success_path(base_dir, &input.preset_id, &input.published_version)?;
    let audit_entry = build_preview_renderer_route_policy_audit_entry(
        "rollback",
        &input.preset_id,
        &input.published_version,
        "shadow",
        &approval,
        "applied",
        canary_success_count,
    )?;

    history.entries.push(audit_entry.clone());
    upsert_preview_route_policy_for_rollback(&mut policy, &input);
    persist_preview_renderer_route_artifacts(base_dir, &policy, &history.entries)?;
    append_preview_renderer_route_policy_audit_record(
        base_dir,
        "rollback",
        &audit_entry,
        "one-action rollback으로 promoted scope를 shadow fallback으로 되돌렸어요.",
        Some("preview-route-rollback"),
    );

    Ok(PreviewRendererRouteMutationResultDto {
        schema_version: PREVIEW_RENDERER_ROUTE_POLICY_AUDIT_ENTRY_SCHEMA_VERSION.into(),
        action: "rollback".into(),
        preset_id: input.preset_id,
        published_version: input.published_version,
        route_stage: "shadow".into(),
        approval,
        audit_entry,
        message: "preview route policy rollback을 기록했어요.".into(),
    })
}

pub fn load_preview_renderer_route_status_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: PreviewRendererRouteStatusInputDto,
) -> Result<PreviewRendererRouteStatusResultDto, HostErrorEnvelope> {
    ensure_settings_access(capability_snapshot)?;
    validate_preview_renderer_route_status_input(&input)?;

    let policy = load_preview_renderer_route_policy(base_dir)?;

    let (route_stage, resolved_route, reason, message) = if let Some(rule) = policy
        .forced_fallback_routes
        .iter()
        .find(|rule| {
            rule.preset_id == input.preset_id && rule.preset_version == input.published_version
        }) {
        (
            "shadow".to_string(),
            rule.route.clone(),
            rule.reason
                .clone()
                .unwrap_or_else(|| "forced-fallback".into()),
            "이 프리셋 버전은 shadow 상태예요.".to_string(),
        )
    } else if let Some(rule) = policy.default_routes.iter().find(|rule| {
        rule.preset_id == input.preset_id && rule.preset_version == input.published_version
    }) {
        (
            "default".to_string(),
            rule.route.clone(),
            rule.reason
                .clone()
                .unwrap_or_else(|| "host-approved-default".into()),
            "이 프리셋 버전은 default 상태예요.".to_string(),
        )
    } else if let Some(rule) = policy.canary_routes.iter().find(|rule| {
        rule.preset_id == input.preset_id && rule.preset_version == input.published_version
    }) {
        (
            "canary".to_string(),
            rule.route.clone(),
            rule.reason
                .clone()
                .unwrap_or_else(|| "host-approved-canary".into()),
            "이 프리셋 버전은 canary 상태예요.".to_string(),
        )
    } else if policy.default_route == PREVIEW_RENDERER_ROUTE_DARKTABLE {
        (
            "shadow".to_string(),
            policy.default_route.clone(),
            "default-route-shadow".into(),
            "이 프리셋 버전은 아직 shadow 상태예요.".to_string(),
        )
    } else {
        (
            "default".to_string(),
            policy.default_route.clone(),
            "global-default-route".into(),
            "이 프리셋 버전은 default 상태예요.".to_string(),
        )
    };

    Ok(PreviewRendererRouteStatusResultDto {
        schema_version: "preview-renderer-route-status-result/v1".into(),
        preset_id: input.preset_id,
        published_version: input.published_version,
        route_stage,
        resolved_route,
        reason,
        message,
    })
}

pub(crate) fn ensure_settings_access(
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<(), HostErrorEnvelope> {
    if capability_snapshot.is_admin_authenticated
        && capability_snapshot
            .allowed_surfaces
            .iter()
            .any(|surface| surface == "settings")
    {
        return Ok(());
    }

    Err(HostErrorEnvelope::capability_denied(
        "승인된 settings surface에서만 지점 배포 거버넌스를 열 수 있어요.",
    ))
}

fn apply_action(
    base_dir: &Path,
    action: &str,
    requested_branch_ids: &[String],
    target_baseline: Option<BranchReleaseBaselineDto>,
    approval: BranchRolloutApprovalDto,
) -> Result<BranchRolloutActionResultDto, HostErrorEnvelope> {
    let _lock = acquire_branch_rollout_store_lock(base_dir)?;
    let previous_store = load_branch_rollout_store(base_dir)?;
    let mut next_store = previous_store.clone();
    let runtime_state_changed = sync_branch_runtime_state(base_dir, &mut next_store)?;
    let history_before = load_branch_rollout_history(base_dir)?;

    let has_duplicate_branch_ids = {
        let mut unique = HashSet::new();
        requested_branch_ids
            .iter()
            .any(|branch_id| !unique.insert(branch_id.clone()))
    };
    let baseline_is_approved = target_baseline
        .as_ref()
        .map(|target| {
            next_store.approved_baselines.iter().any(|baseline| {
                baseline.build_version == target.build_version
                    && baseline.preset_stack_version == target.preset_stack_version
            })
        })
        .unwrap_or(true);

    let fallback_baseline = target_baseline
        .clone()
        .or_else(|| next_store.approved_baselines.first().cloned())
        .unwrap_or_else(|| BranchReleaseBaselineDto {
            build_version: "boothy-2026.03.20.4".into(),
            preset_stack_version: "catalog-2026.03.20".into(),
            approved_at: approval.approved_at.clone(),
            actor_id: approval.actor_id.clone(),
            actor_label: approval.actor_label.clone(),
        });

    let mut outcomes = Vec::new();
    let mut store_changed = runtime_state_changed;

    for branch_id in requested_branch_ids {
        if has_duplicate_branch_ids {
            outcomes.push(rejected_outcome(
                branch_id.as_str(),
                branch_id.as_str(),
                "rejected",
                fallback_baseline.clone(),
                None,
                empty_local_settings_summary(),
                incompatible_verdict("같은 지점을 중복해서 선택해서 적용하지 않았어요."),
                Some(rejection(
                    if action == "rollback" {
                        "missing-rollback-baseline"
                    } else {
                        "unapproved-target-baseline"
                    },
                    "같은 지점을 중복해서 선택할 수 없어요.",
                    "지점 선택을 정리한 뒤 다시 시도해 주세요.",
                )),
            ));
            continue;
        }

        let Some(branch) = next_store
            .branches
            .iter_mut()
            .find(|branch| branch.branch_id == *branch_id)
        else {
            outcomes.push(rejected_outcome(
                branch_id.as_str(),
                branch_id.as_str(),
                "rejected",
                fallback_baseline.clone(),
                None,
                empty_local_settings_summary(),
                incompatible_verdict(
                    "등록되지 않은 지점이라 release 거버넌스를 적용하지 않았어요.",
                ),
                Some(rejection(
                    "branch-not-found",
                    "승인된 지점 목록에 없는 식별자예요.",
                    "지점 목록을 새로고침한 뒤 다시 선택해 주세요.",
                )),
            ));
            continue;
        };

        let local_settings = summarize_local_settings(&branch.local_settings);

        if action == "rollout" && !baseline_is_approved {
            outcomes.push(rejected_outcome(
                &branch.branch_id,
                &branch.display_name,
                "rejected",
                branch.deployment_baseline.clone(),
                branch.pending_baseline.clone(),
                local_settings,
                incompatible_verdict("승인되지 않은 release baseline이라 적용하지 않았어요."),
                Some(rejection(
                    "unapproved-target-baseline",
                    "승인된 release baseline만 rollout할 수 있어요.",
                    "승인 목록에 있는 build와 preset stack 조합을 다시 선택해 주세요.",
                )),
            ));
            continue;
        }

        let outcome = if action == "rollback" {
            resolve_rollback_outcome(branch, &approval)
        } else {
            resolve_rollout_outcome(
                branch,
                target_baseline
                    .as_ref()
                    .expect("rollout target baseline should exist"),
                &approval,
            )
        };

        store_changed |= outcome_store_changed(branch_id, &outcome);

        outcomes.push(outcome);
    }

    let audit_entry = BranchRolloutAuditEntryDto {
        schema_version: BRANCH_ROLLOUT_AUDIT_ENTRY_SCHEMA_VERSION.into(),
        audit_id: build_audit_id(action, &approval.approved_at),
        action: action.into(),
        requested_branch_ids: requested_branch_ids.to_vec(),
        target_baseline: target_baseline.clone(),
        approval: approval.clone(),
        outcomes: outcomes.clone(),
        noted_at: current_timestamp(SystemTime::now())?,
    };

    if store_changed {
        if let Err(error) = persist_branch_rollout_store(base_dir, &next_store) {
            return Err(error);
        }
    }

    let mut history_after = history_before.clone();
    history_after.push(audit_entry.clone());
    if persist_branch_rollout_history(base_dir, &history_after).is_err() {
        if store_changed {
            let _ = persist_branch_rollout_store(base_dir, &previous_store);
        }

        let failed_outcomes = outcomes
            .iter()
            .map(|outcome| audit_write_failed_outcome(outcome, store_changed))
            .collect::<Vec<_>>();
        let failed_audit_entry = BranchRolloutAuditEntryDto {
            outcomes: failed_outcomes.clone(),
            ..audit_entry
        };

        return Ok(build_result(
            action,
            requested_branch_ids.to_vec(),
            target_baseline,
            approval,
            failed_outcomes,
            failed_audit_entry,
            if store_changed {
                "감사 기록을 저장하지 못해 지점 baseline 변경을 적용하지 않았어요."
            } else {
                "감사 기록을 저장하지 못해 이번 평가를 기록하지 않았어요."
            },
        ));
    }

    append_release_governance_audit_records(
        base_dir,
        action,
        &approval,
        &audit_entry.outcomes,
        &audit_entry.noted_at,
    );

    let message = build_action_message(action, &outcomes);

    Ok(build_result(
        action,
        requested_branch_ids.to_vec(),
        target_baseline,
        approval,
        outcomes,
        audit_entry,
        message,
    ))
}

fn resolve_rollout_outcome(
    branch: &mut BranchStateRecord,
    target_baseline: &BranchReleaseBaselineDto,
    _approval: &BranchRolloutApprovalDto,
) -> BranchRolloutBranchResultDto {
    let local_settings = summarize_local_settings(&branch.local_settings);

    if let Some(outcome) = reject_incompatible_active_session(branch, &local_settings) {
        return outcome;
    }

    if let Some(existing_pending) = branch.pending_baseline.clone() {
        if existing_pending == *target_baseline {
            return deferred_outcome(
                branch,
                local_settings,
                "진행 중인 세션 때문에 이미 같은 staged rollout이 대기 중이에요.",
                "세션 종료 후 staged rollout이 적용돼요.",
            );
        }

        return rejected_outcome(
            &branch.branch_id,
            &branch.display_name,
            "rejected",
            branch.deployment_baseline.clone(),
            branch.pending_baseline.clone(),
            local_settings,
            active_session_incompatible_verdict(
                branch,
                "이미 다른 staged baseline이 있어 이번 rollout으로 덮어쓰지 않았어요.",
            ),
            Some(rejection(
                "compatibility-check-failed",
                "다른 staged baseline이 있어 지금은 새 rollout을 겹쳐 둘 수 없어요.",
                "기존 staged transition이 적용되거나 rollback으로 취소된 뒤 다시 시도해 주세요.",
            )),
        );
    }

    if branch.active_session.is_some() && branch.deployment_baseline != *target_baseline {
        branch.pending_baseline = Some(target_baseline.clone());
        return deferred_outcome(
            branch,
            local_settings,
            "진행 중인 세션이 있어 지금은 바로 전환하지 않았어요.",
            "세션 종료 후 staged rollout이 적용돼요.",
        );
    }

    if branch.deployment_baseline != *target_baseline {
        branch.rollback_baseline = Some(branch.deployment_baseline.clone());
        branch.deployment_baseline = target_baseline.clone();
        branch.pending_baseline = None;
    }

    BranchRolloutBranchResultDto {
        branch_id: branch.branch_id.clone(),
        display_name: branch.display_name.clone(),
        result: "applied".into(),
        effective_baseline: branch.deployment_baseline.clone(),
        pending_baseline: branch.pending_baseline.clone(),
        local_settings,
        compatibility: BranchCompatibilityVerdictDto {
            status: "compatible".into(),
            summary: "현재 세션을 끊지 않고 바로 적용할 수 있어요.".into(),
            session_baseline: None,
            safe_transition_required: false,
        },
        rejection: None,
    }
}

fn resolve_rollback_outcome(
    branch: &mut BranchStateRecord,
    _approval: &BranchRolloutApprovalDto,
) -> BranchRolloutBranchResultDto {
    let local_settings = summarize_local_settings(&branch.local_settings);

    if let Some(outcome) = reject_incompatible_active_session(branch, &local_settings) {
        return outcome;
    }

    if branch.active_session.is_some() && branch.pending_baseline.is_some() {
        branch.pending_baseline = None;

        return BranchRolloutBranchResultDto {
            branch_id: branch.branch_id.clone(),
            display_name: branch.display_name.clone(),
            result: "applied".into(),
            effective_baseline: branch.deployment_baseline.clone(),
            pending_baseline: None,
            local_settings,
            compatibility: BranchCompatibilityVerdictDto {
                status: "compatible".into(),
                summary: "진행 중인 세션은 그대로 유지하고 staged rollout만 취소했어요.".into(),
                session_baseline: branch
                    .active_session
                    .as_ref()
                    .map(|session| session.locked_baseline.clone()),
                safe_transition_required: false,
            },
            rejection: None,
        };
    }

    let Some(target_baseline) = branch.rollback_baseline.clone() else {
        return rejected_outcome(
            &branch.branch_id,
            &branch.display_name,
            "rejected",
            branch.deployment_baseline.clone(),
            branch.pending_baseline.clone(),
            local_settings,
            incompatible_verdict("되돌릴 승인 baseline이 아직 없어요."),
            Some(rejection(
                "missing-rollback-baseline",
                "되돌릴 승인 baseline이 아직 없어요.",
                "먼저 승인된 rollout 이력이 있는지 확인해 주세요.",
            )),
        );
    };

    if branch.active_session.is_some() && branch.deployment_baseline != target_baseline {
        branch.pending_baseline = Some(target_baseline.clone());
        return deferred_outcome(
            branch,
            local_settings,
            "진행 중인 세션이 있어 지금은 바로 rollback하지 않았어요.",
            "세션 종료 후 staged rollback이 적용돼요.",
        );
    }

    let previous_deployment = branch.deployment_baseline.clone();
    branch.deployment_baseline = target_baseline;
    branch.rollback_baseline = Some(previous_deployment);
    branch.pending_baseline = None;

    BranchRolloutBranchResultDto {
        branch_id: branch.branch_id.clone(),
        display_name: branch.display_name.clone(),
        result: "applied".into(),
        effective_baseline: branch.deployment_baseline.clone(),
        pending_baseline: branch.pending_baseline.clone(),
        local_settings,
        compatibility: BranchCompatibilityVerdictDto {
            status: "compatible".into(),
            summary: "현재 세션을 끊지 않고 승인된 이전 baseline으로 되돌렸어요.".into(),
            session_baseline: None,
            safe_transition_required: false,
        },
        rejection: None,
    }
}

fn build_result(
    action: &str,
    requested_branch_ids: Vec<String>,
    target_baseline: Option<BranchReleaseBaselineDto>,
    approval: BranchRolloutApprovalDto,
    outcomes: Vec<BranchRolloutBranchResultDto>,
    audit_entry: BranchRolloutAuditEntryDto,
    message: impl Into<String>,
) -> BranchRolloutActionResultDto {
    BranchRolloutActionResultDto {
        schema_version: BRANCH_ROLLOUT_ACTION_RESULT_SCHEMA_VERSION.into(),
        action: action.into(),
        requested_branch_ids,
        target_baseline,
        approval,
        outcomes,
        audit_entry,
        message: message.into(),
    }
}

fn build_action_message(action: &str, outcomes: &[BranchRolloutBranchResultDto]) -> String {
    if outcomes.iter().any(|outcome| outcome.result == "deferred") {
        return if action == "rollback" {
            "진행 중인 세션이 있는 지점은 staged rollback으로 보류했어요.".into()
        } else {
            "진행 중인 세션이 있는 지점은 staged rollout으로 보류했어요.".into()
        };
    }

    if outcomes.iter().all(|outcome| outcome.result == "rejected") {
        return if action == "rollback" {
            "rollback을 진행하지 않았어요.".into()
        } else {
            "rollout을 진행하지 않았어요.".into()
        };
    }

    if action == "rollback" {
        "선택한 지점에 승인된 rollback을 적용했어요.".into()
    } else {
        "선택한 지점에 승인된 rollout을 적용했어요.".into()
    }
}

fn outcome_store_changed(_branch_id: &str, outcome: &BranchRolloutBranchResultDto) -> bool {
    outcome.result != "rejected"
}

fn audit_write_failed_outcome(
    outcome: &BranchRolloutBranchResultDto,
    reverted_store_change: bool,
) -> BranchRolloutBranchResultDto {
    rejected_outcome(
        &outcome.branch_id,
        &outcome.display_name,
        "rejected",
        outcome.effective_baseline.clone(),
        outcome.pending_baseline.clone(),
        outcome.local_settings.clone(),
        incompatible_verdict(if reverted_store_change {
            "감사 기록을 저장하지 못해 release baseline을 바꾸지 않았어요."
        } else {
            "감사 기록을 저장하지 못해 이번 release 평가를 확정하지 않았어요."
        }),
        Some(rejection(
            "audit-write-failed",
            "감사 기록을 저장하지 못해 변경을 확정하지 않았어요.",
            "저장 상태를 확인한 뒤 다시 시도해 주세요.",
        )),
    )
}

fn append_release_governance_audit_records(
    base_dir: &Path,
    action: &str,
    approval: &BranchRolloutApprovalDto,
    outcomes: &[BranchRolloutBranchResultDto],
    occurred_at: &str,
) {
    for outcome in outcomes {
        try_append_operator_audit_record(
            base_dir,
            OperatorAuditRecordInput {
                occurred_at: occurred_at.into(),
                session_id: None,
                event_category: "release-governance",
                event_type: match (action, outcome.result.as_str()) {
                    ("rollout", "applied") => "branch-rollout-applied",
                    ("rollout", "deferred") => "branch-rollout-deferred",
                    ("rollout", _) => "branch-rollout-rejected",
                    ("rollback", "applied") => "branch-rollback-applied",
                    ("rollback", "deferred") => "branch-rollback-deferred",
                    _ => "branch-rollback-rejected",
                },
                summary: format!(
                    "{} 지점 release 거버넌스를 평가했어요.",
                    outcome.display_name
                ),
                detail: outcome
                    .rejection
                    .as_ref()
                    .map(|rejection| rejection.guidance.clone())
                    .unwrap_or_else(|| outcome.compatibility.summary.clone()),
                actor_id: Some(approval.actor_id.clone()),
                source: "branch-config",
                capture_id: None,
                preset_id: None,
                published_version: None,
                reason_code: outcome
                    .rejection
                    .as_ref()
                    .map(|rejection| rejection.code.clone()),
            },
        );
    }
}

fn rejected_outcome(
    branch_id: &str,
    display_name: &str,
    result: &str,
    effective_baseline: BranchReleaseBaselineDto,
    pending_baseline: Option<BranchReleaseBaselineDto>,
    local_settings: BranchLocalSettingsPreservationDto,
    compatibility: BranchCompatibilityVerdictDto,
    rejection: Option<BranchRolloutRejectionDto>,
) -> BranchRolloutBranchResultDto {
    BranchRolloutBranchResultDto {
        branch_id: branch_id.into(),
        display_name: display_name.into(),
        result: result.into(),
        effective_baseline,
        pending_baseline,
        local_settings,
        compatibility,
        rejection,
    }
}

fn rejection(code: &str, message: &str, guidance: &str) -> BranchRolloutRejectionDto {
    BranchRolloutRejectionDto {
        code: code.into(),
        message: message.into(),
        guidance: guidance.into(),
    }
}

fn incompatible_verdict(summary: &str) -> BranchCompatibilityVerdictDto {
    BranchCompatibilityVerdictDto {
        status: "incompatible".into(),
        summary: summary.into(),
        session_baseline: None,
        safe_transition_required: false,
    }
}

fn active_session_incompatible_verdict(
    branch: &BranchStateRecord,
    summary: &str,
) -> BranchCompatibilityVerdictDto {
    BranchCompatibilityVerdictDto {
        status: "incompatible".into(),
        summary: summary.into(),
        session_baseline: branch
            .active_session
            .as_ref()
            .map(|session| session.locked_baseline.clone()),
        safe_transition_required: true,
    }
}

fn reject_incompatible_active_session(
    branch: &BranchStateRecord,
    local_settings: &BranchLocalSettingsPreservationDto,
) -> Option<BranchRolloutBranchResultDto> {
    let active_session = branch.active_session.as_ref()?;

    if active_session.locked_baseline == branch.deployment_baseline {
        return None;
    }

    Some(rejected_outcome(
        &branch.branch_id,
        &branch.display_name,
        "rejected",
        branch.deployment_baseline.clone(),
        branch.pending_baseline.clone(),
        local_settings.clone(),
        active_session_incompatible_verdict(
            branch,
            "진행 중 세션의 고정 baseline과 현재 배포 baseline이 달라 compatibility check를 통과하지 못했어요.",
        ),
        Some(rejection(
            "compatibility-check-failed",
            "진행 중 세션의 고정 baseline을 확인하지 못해 지금은 적용하지 않았어요.",
            "현재 세션 상태를 다시 동기화한 뒤 rollout 또는 rollback을 다시 시도해 주세요.",
        )),
    ))
}

fn deferred_outcome(
    branch: &BranchStateRecord,
    local_settings: BranchLocalSettingsPreservationDto,
    message: &str,
    guidance: &str,
) -> BranchRolloutBranchResultDto {
    BranchRolloutBranchResultDto {
        branch_id: branch.branch_id.clone(),
        display_name: branch.display_name.clone(),
        result: "deferred".into(),
        effective_baseline: branch.deployment_baseline.clone(),
        pending_baseline: branch.pending_baseline.clone(),
        local_settings,
        compatibility: BranchCompatibilityVerdictDto {
            status: "deferred-until-safe-transition".into(),
            summary: "진행 중인 세션은 기존 baseline을 유지하고 종료 후에만 전환돼요.".into(),
            session_baseline: branch
                .active_session
                .as_ref()
                .map(|session| session.locked_baseline.clone()),
            safe_transition_required: true,
        },
        rejection: Some(rejection("active-session-deferred", message, guidance)),
    }
}

fn build_branch_state_dto(branch: &BranchStateRecord) -> BranchRolloutBranchStateDto {
    BranchRolloutBranchStateDto {
        branch_id: branch.branch_id.clone(),
        display_name: branch.display_name.clone(),
        deployment_baseline: branch.deployment_baseline.clone(),
        rollback_baseline: branch.rollback_baseline.clone(),
        pending_baseline: branch.pending_baseline.clone(),
        local_settings: summarize_local_settings(&branch.local_settings),
        active_session: branch.active_session.clone(),
    }
}

fn summarize_local_settings(
    local_settings: &BranchLocalSettingsRecord,
) -> BranchLocalSettingsPreservationDto {
    let mut preserved_fields = Vec::new();

    if local_settings
        .contact_phone
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        preserved_fields.push("contact-phone".into());
    }
    if local_settings
        .contact_email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        preserved_fields.push("contact-email".into());
    }
    if local_settings
        .contact_kakao
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        preserved_fields.push("contact-kakao".into());
    }
    if local_settings
        .support_hours
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        preserved_fields.push("support-hours".into());
    }
    if !local_settings.operational_toggles.is_empty() {
        preserved_fields.push("bounded-operational-toggle".into());
    }

    let summary = if preserved_fields.contains(&"bounded-operational-toggle".to_string())
        && preserved_fields
            .iter()
            .any(|field| matches!(field.as_str(), "contact-phone" | "contact-email"))
    {
        "지점 연락처와 승인된 운영 토글은 그대로 유지돼요."
    } else if preserved_fields
        .iter()
        .any(|field| matches!(field.as_str(), "contact-phone" | "contact-email"))
    {
        "지점별 연락처는 그대로 유지돼요."
    } else {
        "승인된 지점별 설정은 그대로 유지돼요."
    };

    BranchLocalSettingsPreservationDto {
        preserved_fields,
        summary: summary.into(),
    }
}

fn empty_local_settings_summary() -> BranchLocalSettingsPreservationDto {
    BranchLocalSettingsPreservationDto {
        preserved_fields: vec!["bounded-operational-toggle".into()],
        summary: "승인된 지점별 설정은 그대로 유지돼요.".into(),
    }
}

fn sync_branch_runtime_state(
    base_dir: &Path,
    store: &mut BranchRolloutStore,
) -> Result<bool, HostErrorEnvelope> {
    let mut changed = false;

    for branch in &mut store.branches {
        changed |= sync_branch_active_session(base_dir, branch)?;

        if branch.active_session.is_none() {
            if let Some(pending_baseline) = branch.pending_baseline.clone() {
                if branch.deployment_baseline != pending_baseline {
                    branch.rollback_baseline = Some(branch.deployment_baseline.clone());
                    branch.deployment_baseline = pending_baseline;
                }
                branch.pending_baseline = None;
                changed = true;
            }
        }
    }

    Ok(changed)
}

fn sync_branch_active_session(
    base_dir: &Path,
    branch: &mut BranchStateRecord,
) -> Result<bool, HostErrorEnvelope> {
    let Some(active_session) = branch.active_session.clone() else {
        return Ok(false);
    };
    let manifest_path = SessionPaths::new(base_dir, &active_session.session_id).manifest_path;

    if !manifest_path.is_file() {
        branch.active_session = None;
        return Ok(true);
    }

    let manifest = read_session_manifest(&manifest_path)?;
    let manifest =
        sync_session_timing_in_dir(base_dir, &manifest_path, manifest, SystemTime::now())?;
    let manifest =
        sync_post_end_state_in_dir(base_dir, &manifest_path, manifest, SystemTime::now())?;
    let timing_phase = manifest
        .timing
        .as_ref()
        .map(|timing| timing.phase.as_str())
        .unwrap_or("active");
    let session_is_active = timing_phase != "ended" && manifest.post_end.is_none();

    if session_is_active {
        return Ok(false);
    }

    branch.active_session = None;
    Ok(true)
}

fn read_branch_rollout_store_from_path(
    path: &Path,
) -> Result<BranchRolloutStore, HostErrorEnvelope> {
    let bytes = fs::read_to_string(path).map_err(map_fs_error)?;
    let mut parsed: BranchRolloutStore = serde_json::from_str(&bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("branch rollout store를 읽지 못했어요: {error}"))
    })?;

    if parsed.schema_version != BRANCH_ROLLOUT_STORE_SCHEMA_VERSION {
        parsed.schema_version = BRANCH_ROLLOUT_STORE_SCHEMA_VERSION.into();
    }

    Ok(parsed)
}

fn read_branch_rollout_history_from_path(
    path: &Path,
) -> Result<Vec<BranchRolloutAuditEntryDto>, HostErrorEnvelope> {
    let bytes = fs::read_to_string(path).map_err(map_fs_error)?;
    let parsed = serde_json::from_str::<BranchRolloutHistoryStore>(&bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("branch rollout history를 읽지 못했어요: {error}"))
    })?;

    if parsed.schema_version != BRANCH_ROLLOUT_HISTORY_STORE_SCHEMA_VERSION {
        return Err(HostErrorEnvelope::persistence(
            "branch rollout history 형식을 확인하지 못했어요.",
        ));
    }

    Ok(parsed.entries)
}

fn load_branch_rollout_store(base_dir: &Path) -> Result<BranchRolloutStore, HostErrorEnvelope> {
    let store_path = resolve_branch_rollout_store_path(base_dir);
    let backup_path = store_path.with_extension("json.bak");

    if !store_path.exists() {
        if backup_path.is_file() {
            return read_branch_rollout_store_from_path(&backup_path);
        }

        return Ok(BranchRolloutStore {
            schema_version: BRANCH_ROLLOUT_STORE_SCHEMA_VERSION.into(),
            approved_baselines: Vec::new(),
            branches: Vec::new(),
        });
    }

    match read_branch_rollout_store_from_path(&store_path) {
        Ok(store) => Ok(store),
        Err(error) if backup_path.is_file() => {
            read_branch_rollout_store_from_path(&backup_path).or(Err(error))
        }
        Err(error) => Err(error),
    }
}

fn persist_branch_rollout_store(
    base_dir: &Path,
    store: &BranchRolloutStore,
) -> Result<(), HostErrorEnvelope> {
    let store_path = resolve_branch_rollout_store_path(base_dir);
    let store_dir = store_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("branch rollout store 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(store_dir).map_err(map_fs_error)?;
    let bytes = serde_json::to_vec_pretty(store).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "branch rollout store를 직렬화하지 못했어요: {error}"
        ))
    })?;

    write_json_bytes_atomically(&store_path, &bytes)
}

fn load_branch_rollout_history(
    base_dir: &Path,
) -> Result<Vec<BranchRolloutAuditEntryDto>, HostErrorEnvelope> {
    let history_path = resolve_branch_rollout_history_path(base_dir);
    let backup_path = history_path.with_extension("json.bak");

    if !history_path.exists() {
        if backup_path.is_file() {
            return read_branch_rollout_history_from_path(&backup_path);
        }

        return Ok(Vec::new());
    }

    match read_branch_rollout_history_from_path(&history_path) {
        Ok(history) => Ok(history),
        Err(error) if backup_path.is_file() => {
            read_branch_rollout_history_from_path(&backup_path).or(Err(error))
        }
        Err(error) => Err(error),
    }
}

fn persist_branch_rollout_history(
    base_dir: &Path,
    entries: &[BranchRolloutAuditEntryDto],
) -> Result<(), HostErrorEnvelope> {
    let history_path = resolve_branch_rollout_history_path(base_dir);
    let history_dir = history_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("branch rollout history 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(history_dir).map_err(map_fs_error)?;
    let bytes = serde_json::to_vec_pretty(&BranchRolloutHistoryStore {
        schema_version: BRANCH_ROLLOUT_HISTORY_STORE_SCHEMA_VERSION.into(),
        entries: entries.to_vec(),
    })
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "branch rollout history를 직렬화하지 못했어요: {error}"
        ))
    })?;

    write_json_bytes_atomically(&history_path, &bytes)
}

fn load_preview_renderer_route_policy(
    base_dir: &Path,
) -> Result<PreviewRendererRoutePolicyStore, HostErrorEnvelope> {
    let policy_path = resolve_preview_renderer_route_policy_path(base_dir);

    if !policy_path.exists() {
        return Ok(PreviewRendererRoutePolicyStore {
            schema_version: PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION.into(),
            default_route: PREVIEW_RENDERER_ROUTE_DARKTABLE.into(),
            default_routes: Vec::new(),
            canary_routes: Vec::new(),
            forced_fallback_routes: Vec::new(),
        });
    }

    let bytes = fs::read_to_string(&policy_path).map_err(map_fs_error)?;
    let mut parsed: PreviewRendererRoutePolicyStore =
        serde_json::from_str(&bytes).map_err(|error| {
            HostErrorEnvelope::persistence(format!("preview route policy를 읽지 못했어요: {error}"))
        })?;
    if parsed.schema_version != PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION {
        parsed.schema_version = PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION.into();
    }
    if parsed.default_route.trim().is_empty() {
        parsed.default_route = PREVIEW_RENDERER_ROUTE_DARKTABLE.into();
    }

    Ok(parsed)
}

fn load_preview_renderer_route_policy_history(
    base_dir: &Path,
) -> Result<PreviewRendererRoutePolicyHistoryStore, HostErrorEnvelope> {
    let history_path = resolve_preview_renderer_route_policy_history_path(base_dir);
    if !history_path.exists() {
        return Ok(PreviewRendererRoutePolicyHistoryStore {
            schema_version: PREVIEW_RENDERER_ROUTE_POLICY_HISTORY_SCHEMA_VERSION.into(),
            entries: Vec::new(),
        });
    }

    let bytes = fs::read_to_string(&history_path).map_err(map_fs_error)?;
    let parsed: PreviewRendererRoutePolicyHistoryStore =
        serde_json::from_str(&bytes).map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "preview route policy history를 읽지 못했어요: {error}"
            ))
        })?;
    Ok(parsed)
}

fn persist_preview_renderer_route_policy_history(
    base_dir: &Path,
    entries: &[PreviewRendererRoutePolicyAuditEntryDto],
) -> Result<(), HostErrorEnvelope> {
    let history_path = resolve_preview_renderer_route_policy_history_path(base_dir);
    let history_dir = history_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("preview route policy history 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(history_dir).map_err(map_fs_error)?;
    let bytes = serde_json::to_vec_pretty(&PreviewRendererRoutePolicyHistoryStore {
        schema_version: PREVIEW_RENDERER_ROUTE_POLICY_HISTORY_SCHEMA_VERSION.into(),
        entries: entries.to_vec(),
    })
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview route policy history를 직렬화하지 못했어요: {error}"
        ))
    })?;

    write_json_bytes_atomically(&history_path, &bytes)
}

fn persist_preview_renderer_route_artifacts(
    base_dir: &Path,
    policy: &PreviewRendererRoutePolicyStore,
    history_entries: &[PreviewRendererRoutePolicyAuditEntryDto],
) -> Result<(), HostErrorEnvelope> {
    let policy_path = resolve_preview_renderer_route_policy_path(base_dir);
    let history_path = resolve_preview_renderer_route_policy_history_path(base_dir);
    let original_policy_bytes = read_optional_file_bytes(&policy_path)?;
    let original_history_bytes = read_optional_file_bytes(&history_path)?;
    let policy_bytes = serde_json::to_vec_pretty(policy).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview route policy를 직렬화하지 못했어요: {error}"
        ))
    })?;
    let history_bytes = serde_json::to_vec_pretty(&PreviewRendererRoutePolicyHistoryStore {
        schema_version: PREVIEW_RENDERER_ROUTE_POLICY_HISTORY_SCHEMA_VERSION.into(),
        entries: history_entries.to_vec(),
    })
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview route policy history를 직렬화하지 못했어요: {error}"
        ))
    })?;

    write_json_bytes_atomically(&policy_path, &policy_bytes)?;
    if let Err(error) = write_json_bytes_atomically(&history_path, &history_bytes) {
        let policy_restore_error =
            restore_optional_file_bytes_atomically(&policy_path, original_policy_bytes.as_deref())
                .err();
        let history_restore_error = restore_optional_file_bytes_atomically(
            &history_path,
            original_history_bytes.as_deref(),
        )
        .err();
        if let Some(restore_error) = policy_restore_error.or(history_restore_error) {
            return Err(HostErrorEnvelope::persistence(format!(
                "{} 원복까지 실패했어요: {}",
                error.message, restore_error.message
            )));
        }
        return Err(error);
    }

    Ok(())
}

fn upsert_preview_route_policy_for_promotion(
    policy: &mut PreviewRendererRoutePolicyStore,
    input: &PreviewRendererRoutePromotionInputDto,
) {
    remove_matching_preview_route_rule(
        &mut policy.forced_fallback_routes,
        &input.preset_id,
        &input.published_version,
    );

    if input.target_route_stage == "default" {
        upsert_preview_route_rule(
            &mut policy.default_routes,
            PreviewRendererRoutePolicyRule {
                route: PREVIEW_RENDERER_ROUTE_LOCAL_SIDECAR.into(),
                preset_id: input.preset_id.clone(),
                preset_version: input.published_version.clone(),
                reason: Some("host-approved-default".into()),
            },
        );
        remove_matching_preview_route_rule(
            &mut policy.canary_routes,
            &input.preset_id,
            &input.published_version,
        );
        return;
    }

    upsert_preview_route_rule(
        &mut policy.canary_routes,
        PreviewRendererRoutePolicyRule {
            route: PREVIEW_RENDERER_ROUTE_LOCAL_SIDECAR.into(),
            preset_id: input.preset_id.clone(),
            preset_version: input.published_version.clone(),
            reason: Some("host-approved-canary".into()),
        },
    );
}

fn upsert_preview_route_policy_for_rollback(
    policy: &mut PreviewRendererRoutePolicyStore,
    input: &PreviewRendererRouteRollbackInputDto,
) {
    remove_matching_preview_route_rule(
        &mut policy.canary_routes,
        &input.preset_id,
        &input.published_version,
    );
    remove_matching_preview_route_rule(
        &mut policy.default_routes,
        &input.preset_id,
        &input.published_version,
    );
    upsert_preview_route_rule(
        &mut policy.forced_fallback_routes,
        PreviewRendererRoutePolicyRule {
            route: PREVIEW_RENDERER_ROUTE_DARKTABLE.into(),
            preset_id: input.preset_id.clone(),
            preset_version: input.published_version.clone(),
            reason: Some("rollback".into()),
        },
    );
}

fn upsert_preview_route_rule(
    rules: &mut Vec<PreviewRendererRoutePolicyRule>,
    next_rule: PreviewRendererRoutePolicyRule,
) {
    if let Some(existing) = rules.iter_mut().find(|rule| {
        rule.preset_id == next_rule.preset_id && rule.preset_version == next_rule.preset_version
    }) {
        *existing = next_rule;
    } else {
        rules.push(next_rule);
    }
}

fn remove_matching_preview_route_rule(
    rules: &mut Vec<PreviewRendererRoutePolicyRule>,
    preset_id: &str,
    published_version: &str,
) {
    rules.retain(|rule| !(rule.preset_id == preset_id && rule.preset_version == published_version));
}

fn count_repeated_canary_success_path(
    base_dir: &Path,
    preset_id: &str,
    published_version: &str,
) -> Result<u32, HostErrorEnvelope> {
    let sessions_root = base_dir.join("sessions");
    if !sessions_root.exists() {
        return Ok(0);
    }

    let mut seen_runs = HashSet::new();
    let mut success_count = 0_u32;
    for entry in fs::read_dir(&sessions_root).map_err(map_fs_error)? {
        let session_root = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };
        let evidence_path = session_root
            .join("diagnostics")
            .join("dedicated-renderer")
            .join("preview-promotion-evidence.jsonl");
        if !evidence_path.is_file() {
            continue;
        }

        let contents = fs::read_to_string(&evidence_path).map_err(map_fs_error)?;
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let parsed = match serde_json::from_str::<serde_json::Value>(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let same_scope = parsed["presetId"].as_str() == Some(preset_id)
                && parsed["publishedVersion"].as_str() == Some(published_version);
            let success_path = parsed["laneOwner"].as_str() == Some("dedicated-renderer")
                && parsed["routeStage"].as_str() == Some("canary")
                && parsed["fallbackReasonCode"].is_null()
                && matches!(
                    parsed["warmState"].as_str(),
                    Some("warm-ready") | Some("warm-hit")
                )
                && parsed["firstVisibleMs"].as_u64().is_some()
                && parsed["replacementMs"].as_u64().is_some()
                && parsed["originalVisibleToPresetAppliedVisibleMs"]
                    .as_u64()
                    .is_some()
                && has_non_blank_string_field(&parsed, "sessionManifestPath")
                && has_non_blank_string_field(&parsed, "timingEventsPath")
                && has_non_blank_string_field(&parsed, "routePolicySnapshotPath")
                && has_non_blank_string_field(&parsed, "catalogStatePath");
            if same_scope && success_path {
                let session_id = parsed["sessionId"].as_str().unwrap_or_default();
                let request_id = parsed["requestId"].as_str().unwrap_or_default();
                let capture_id = parsed["captureId"].as_str().unwrap_or_default();
                let dedupe_key = format!("{session_id}::{request_id}::{capture_id}");
                if !seen_runs.insert(dedupe_key) {
                    continue;
                }
                success_count = success_count.saturating_add(1);
            }
        }
    }

    Ok(success_count)
}

fn build_preview_renderer_route_policy_audit_entry(
    action: &str,
    preset_id: &str,
    published_version: &str,
    target_route_stage: &str,
    approval: &BranchRolloutApprovalDto,
    result: &str,
    canary_success_count: u32,
) -> Result<PreviewRendererRoutePolicyAuditEntryDto, HostErrorEnvelope> {
    Ok(PreviewRendererRoutePolicyAuditEntryDto {
        schema_version: PREVIEW_RENDERER_ROUTE_POLICY_AUDIT_ENTRY_SCHEMA_VERSION.into(),
        audit_id: build_preview_route_policy_audit_id(action, &approval.approved_at),
        action: action.into(),
        preset_id: preset_id.into(),
        published_version: published_version.into(),
        target_route_stage: target_route_stage.into(),
        approval: approval.clone(),
        result: result.into(),
        canary_success_count,
        noted_at: current_timestamp(SystemTime::now())?,
    })
}

fn has_non_blank_string_field(record: &serde_json::Value, field_name: &str) -> bool {
    record[field_name]
        .as_str()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn append_preview_renderer_route_policy_audit_record(
    base_dir: &Path,
    action: &str,
    audit_entry: &PreviewRendererRoutePolicyAuditEntryDto,
    detail: &str,
    reason_code: Option<&str>,
) {
    let event_type = match (action, audit_entry.result.as_str()) {
        ("promote", "applied") => "preview-route-promotion-applied",
        ("promote", _) => "preview-route-promotion-rejected",
        ("rollback", "applied") => "preview-route-rollback-applied",
        _ => "preview-route-rollback-rejected",
    };
    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: audit_entry.noted_at.clone(),
            session_id: None,
            event_category: "release-governance",
            event_type,
            summary: format!(
                "{} {} route policy를 평가했어요.",
                audit_entry.preset_id, audit_entry.target_route_stage
            ),
            detail: detail.into(),
            actor_id: Some(audit_entry.approval.actor_id.clone()),
            source: "branch-config",
            capture_id: None,
            preset_id: Some(audit_entry.preset_id.clone()),
            published_version: Some(audit_entry.published_version.clone()),
            reason_code: reason_code.map(str::to_string),
        },
    );
}

fn build_preview_route_policy_audit_id(action: &str, approved_at: &str) -> String {
    let safe_timestamp = approved_at
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    let counter = PREVIEW_RENDERER_ROUTE_POLICY_AUDIT_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!(
        "preview-route-{action}-{}-{counter:04}",
        &safe_timestamp[..safe_timestamp.len().min(12)]
    )
}

fn resolve_branch_rollout_store_path(base_dir: &Path) -> PathBuf {
    base_dir.join("branch-config").join("state.json")
}

fn resolve_preview_renderer_route_policy_path(base_dir: &Path) -> PathBuf {
    base_dir
        .join("branch-config")
        .join("preview-renderer-policy.json")
}

fn resolve_preview_renderer_route_policy_history_path(base_dir: &Path) -> PathBuf {
    base_dir
        .join("branch-config")
        .join("preview-renderer-policy-history.json")
}

fn resolve_branch_rollout_history_path(base_dir: &Path) -> PathBuf {
    base_dir.join("branch-config").join("rollout-history.json")
}

fn resolve_branch_rollout_lock_path(base_dir: &Path) -> PathBuf {
    base_dir.join("branch-config").join("governance.lock")
}

fn read_optional_file_bytes(path: &Path) -> Result<Option<Vec<u8>>, HostErrorEnvelope> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(map_fs_error(error)),
    }
}

fn restore_optional_file_bytes_atomically(
    path: &Path,
    original_bytes: Option<&[u8]>,
) -> Result<(), HostErrorEnvelope> {
    match original_bytes {
        Some(bytes) => write_json_bytes_atomically_inner(path, bytes, false),
        None => {
            if path.exists() {
                fs::remove_file(path).map_err(map_fs_error)?;
            }
            Ok(())
        }
    }
}

fn acquire_branch_rollout_store_lock(
    base_dir: &Path,
) -> Result<BranchRolloutStoreLock, HostErrorEnvelope> {
    let lock_path = resolve_branch_rollout_lock_path(base_dir);
    let lock_dir = lock_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("branch rollout lock 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(lock_dir).map_err(map_fs_error)?;

    for _ in 0..BRANCH_ROLLOUT_LOCK_MAX_ATTEMPTS {
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(mut lock_file) => {
                let _ = writeln!(lock_file, "pid={}", std::process::id());
                return Ok(BranchRolloutStoreLock { lock_path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                try_clear_stale_branch_rollout_lock(&lock_path)?;
                thread::sleep(Duration::from_millis(BRANCH_ROLLOUT_LOCK_RETRY_DELAY_MS));
            }
            Err(error) => {
                return Err(HostErrorEnvelope::persistence(format!(
                    "branch rollout 잠금을 준비하지 못했어요: {error}"
                )));
            }
        }
    }

    Err(HostErrorEnvelope::persistence(
        "branch rollout 잠금을 기다리는 중 시간이 초과되었어요.",
    ))
}

fn try_clear_stale_branch_rollout_lock(lock_path: &Path) -> Result<(), HostErrorEnvelope> {
    if !lock_path.exists() || !is_stale_branch_rollout_lock(lock_path)? {
        return Ok(());
    }

    fs::remove_file(lock_path).map_err(map_fs_error)
}

fn is_stale_branch_rollout_lock(lock_path: &Path) -> Result<bool, HostErrorEnvelope> {
    let metadata = match fs::metadata(lock_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(map_fs_error(error)),
    };
    let modified_at = metadata.modified().map_err(map_fs_error)?;

    Ok(SystemTime::now()
        .duration_since(modified_at)
        .unwrap_or_default()
        >= Duration::from_millis(BRANCH_ROLLOUT_LOCK_STALE_AFTER_MS))
}

fn write_json_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), HostErrorEnvelope> {
    write_json_bytes_atomically_inner(path, bytes, true)
}

fn write_json_bytes_atomically_inner(
    path: &Path,
    bytes: &[u8],
    inject_failures: bool,
) -> Result<(), HostErrorEnvelope> {
    if inject_failures
        && path.file_name().and_then(|name| name.to_str())
            == Some("preview-renderer-policy-history.json")
        && env::var_os(PREVIEW_RENDERER_ROUTE_POLICY_HISTORY_WRITE_FAILURE_ENV).is_some()
    {
        return Err(HostErrorEnvelope::persistence(
            "preview route policy history를 저장하지 못했어요.",
        ));
    }

    let temp_path = path.with_extension("json.tmp");
    let backup_path = path.with_extension("json.bak");

    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(map_fs_error)?;
    }

    fs::write(&temp_path, bytes).map_err(map_fs_error)?;

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(map_fs_error)?;
    }

    if path.exists() {
        fs::rename(path, &backup_path).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            map_fs_error(error)
        })?;
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        let _ = fs::remove_file(&temp_path);

        return Err(map_fs_error(error));
    }

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(map_fs_error)?;
    }

    Ok(())
}

fn build_audit_id(action: &str, approved_at: &str) -> String {
    let safe_timestamp = approved_at
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    let counter = BRANCH_ROLLOUT_AUDIT_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!(
        "branch-{action}-{}-{counter:04}",
        &safe_timestamp[..safe_timestamp.len().min(12)]
    )
}

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("branch rollout 파일을 저장하지 못했어요: {error}"))
}
