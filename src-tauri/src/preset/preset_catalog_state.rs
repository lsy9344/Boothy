use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

use crate::{
    contracts::dto::{
        validate_rollback_preset_catalog_input, CapabilitySnapshotDto,
        CatalogVersionHistoryItemDto, HostErrorEnvelope, PresetCatalogStateResultDto,
        PresetCatalogStateSummaryDto, PublishedPresetSummaryDto, RollbackPresetCatalogInputDto,
        RollbackPresetCatalogResultDto,
    },
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    preset::{
        authoring_pipeline::ensure_authoring_access, preset_bundle::load_published_preset_summary,
        preset_catalog::resolve_published_preset_catalog_dir,
    },
    session::session_manifest::{current_timestamp, ActivePresetBinding},
};

const PRESET_CATALOG_STATE_SCHEMA_VERSION: &str = "preset-catalog-state/v1";
const PRESET_CATALOG_HISTORY_SCHEMA_VERSION: &str = "preset-catalog-history/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogStateRecord {
    schema_version: String,
    catalog_revision: u64,
    updated_at: String,
    live_presets: Vec<CatalogLivePresetEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogLivePresetEntry {
    preset_id: String,
    published_version: String,
}

#[derive(Debug)]
pub struct CatalogActivationOutcome {
    pub catalog_revision: u64,
    pub summary: PresetCatalogStateSummaryDto,
    pub audit_entry: CatalogVersionHistoryItemDto,
}

pub fn capture_live_catalog_snapshot(
    base_dir: &Path,
) -> Result<(u64, Vec<ActivePresetBinding>), HostErrorEnvelope> {
    let state = load_or_initialize_catalog_state(base_dir)?;
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundles_by_id = load_published_presets_grouped_by_id(&catalog_root)?;
    let mut live_presets = state
        .live_presets
        .iter()
        .filter_map(|entry| {
            let live_summary = bundles_by_id
                .get(&entry.preset_id)
                .and_then(|versions| {
                    versions
                        .iter()
                        .find(|summary| summary.published_version == entry.published_version)
                })?
                .clone();

            Some((live_summary, entry))
        })
        .collect::<Vec<_>>();
    live_presets.sort_by(|left, right| {
        left.0
            .display_name
            .cmp(&right.0.display_name)
            .then_with(|| left.0.preset_id.cmp(&right.0.preset_id))
    });
    live_presets.truncate(6);

    let snapshot = live_presets
        .into_iter()
        .map(|(_, entry)| ActivePresetBinding {
            preset_id: entry.preset_id.clone(),
            published_version: entry.published_version.clone(),
        })
        .collect();

    Ok((state.catalog_revision, snapshot))
}

pub fn load_preset_catalog_state_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<PresetCatalogStateResultDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;

    build_catalog_state_result(base_dir)
}

pub fn rollback_preset_catalog_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: RollbackPresetCatalogInputDto,
) -> Result<RollbackPresetCatalogResultDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_rollback_preset_catalog_input(&input)?;

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundles_by_id = load_published_presets_grouped_by_id(&catalog_root)?;
    let state = load_or_initialize_catalog_state(base_dir)?;
    let current_summary =
        build_catalog_state_summary_for_preset(base_dir, &bundles_by_id, &state, &input.preset_id)?;

    if state.catalog_revision != input.expected_catalog_revision {
        return Ok(RollbackPresetCatalogResultDto::Rejected {
            schema_version: "preset-catalog-rollback-result/v1".into(),
            reason_code: "stale-catalog-revision".into(),
            message: "방금 전 상태를 기준으로 다시 확인해 주세요. live catalog가 이미 바뀌었어요."
                .into(),
            guidance:
                "최신 catalog 상태를 다시 불러온 뒤 원하는 rollback target을 다시 선택해 주세요."
                    .into(),
            catalog_revision: state.catalog_revision,
            summary: current_summary,
        });
    }

    let Some(current_live_version) = state
        .live_presets
        .iter()
        .find(|entry| entry.preset_id == input.preset_id)
        .map(|entry| entry.published_version.clone())
    else {
        return Ok(RollbackPresetCatalogResultDto::Rejected {
            schema_version: "preset-catalog-rollback-result/v1".into(),
            reason_code: "target-missing".into(),
            message: "이 preset은 지금 rollback할 live catalog 항목이 없어요.".into(),
            guidance: "먼저 승인된 게시 버전이 있는지 확인해 주세요.".into(),
            catalog_revision: state.catalog_revision,
            summary: current_summary,
        });
    };

    if current_live_version == input.target_published_version {
        return Ok(RollbackPresetCatalogResultDto::Rejected {
            schema_version: "preset-catalog-rollback-result/v1".into(),
            reason_code: "already-live".into(),
            message: "이미 현재 미래 세션 catalog에 노출 중인 버전이에요.".into(),
            guidance: "다른 승인 버전을 선택하거나 현재 상태를 유지해 주세요.".into(),
            catalog_revision: state.catalog_revision,
            summary: current_summary,
        });
    }

    match ensure_target_bundle_is_valid(
        &catalog_root,
        &bundles_by_id,
        &input.preset_id,
        &input.target_published_version,
    ) {
        Ok(_) => {}
        Err(TargetBundleValidation::Missing) => {
            return Ok(RollbackPresetCatalogResultDto::Rejected {
                schema_version: "preset-catalog-rollback-result/v1".into(),
                reason_code: "target-missing".into(),
                message: "선택한 승인 버전을 찾지 못했어요.".into(),
                guidance: "version 목록을 새로고침한 뒤 다시 선택해 주세요.".into(),
                catalog_revision: state.catalog_revision,
                summary: current_summary,
            })
        }
        Err(TargetBundleValidation::Incompatible) => {
            return Ok(RollbackPresetCatalogResultDto::Rejected {
                schema_version: "preset-catalog-rollback-result/v1".into(),
                reason_code: "target-incompatible".into(),
                message: "선택한 버전은 booth-safe published bundle 기준을 통과하지 못했어요."
                    .into(),
                guidance:
                    "승인된 게시 bundle 상태를 다시 확인하고, 다른 승인 버전을 선택해 주세요."
                        .into(),
                catalog_revision: state.catalog_revision,
                summary: current_summary,
            })
        }
    }

    let happened_at = current_timestamp(SystemTime::now())?;
    let outcome = activate_catalog_preset_version(
        base_dir,
        &input.preset_id,
        &input.target_published_version,
        "rollback",
        &input.actor_id,
        &input.actor_label,
        &happened_at,
    )?;

    Ok(RollbackPresetCatalogResultDto::RolledBack {
        schema_version: "preset-catalog-rollback-result/v1".into(),
        catalog_revision: outcome.catalog_revision,
        summary: outcome.summary,
        audit_entry: outcome.audit_entry,
        message:
            "선택한 승인 버전으로 되돌렸어요. 이미 진행 중인 세션은 기존 바인딩을 계속 유지해요."
                .into(),
    })
}

pub fn preview_rollback_preset_catalog_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: RollbackPresetCatalogInputDto,
) -> Result<RollbackPresetCatalogResultDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_rollback_preset_catalog_input(&input)?;

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundles_by_id = load_published_presets_grouped_by_id(&catalog_root)?;
    let state = load_or_initialize_catalog_state(base_dir)?;
    let current_summary =
        build_catalog_state_summary_for_preset(base_dir, &bundles_by_id, &state, &input.preset_id)?;

    if state.catalog_revision != input.expected_catalog_revision {
        return Ok(RollbackPresetCatalogResultDto::Rejected {
            schema_version: "preset-catalog-rollback-result/v1".into(),
            reason_code: "stale-catalog-revision".into(),
            message: "방금 전 상태를 기준으로 다시 확인해 주세요. live catalog가 이미 바뀌었어요."
                .into(),
            guidance:
                "최신 catalog 상태를 다시 불러온 뒤 원하는 rollback target을 다시 선택해 주세요."
                    .into(),
            catalog_revision: state.catalog_revision,
            summary: current_summary,
        });
    }

    let Some(current_live_version) = state
        .live_presets
        .iter()
        .find(|entry| entry.preset_id == input.preset_id)
        .map(|entry| entry.published_version.clone())
    else {
        return Ok(RollbackPresetCatalogResultDto::Rejected {
            schema_version: "preset-catalog-rollback-result/v1".into(),
            reason_code: "target-missing".into(),
            message: "이 preset은 지금 rollback할 live catalog 항목이 없어요.".into(),
            guidance: "먼저 승인된 게시 버전이 있는지 확인해 주세요.".into(),
            catalog_revision: state.catalog_revision,
            summary: current_summary,
        });
    };

    if current_live_version == input.target_published_version {
        return Ok(RollbackPresetCatalogResultDto::Rejected {
            schema_version: "preset-catalog-rollback-result/v1".into(),
            reason_code: "already-live".into(),
            message: "이미 현재 미래 세션 catalog에 노출 중인 버전이에요.".into(),
            guidance: "다른 승인 버전을 선택하거나 현재 상태를 유지해 주세요.".into(),
            catalog_revision: state.catalog_revision,
            summary: current_summary,
        });
    }

    match ensure_target_bundle_is_valid(
        &catalog_root,
        &bundles_by_id,
        &input.preset_id,
        &input.target_published_version,
    ) {
        Ok(_) => {}
        Err(TargetBundleValidation::Missing) => {
            return Ok(RollbackPresetCatalogResultDto::Rejected {
                schema_version: "preset-catalog-rollback-result/v1".into(),
                reason_code: "target-missing".into(),
                message: "선택한 승인 버전을 찾지 못했어요.".into(),
                guidance: "version 목록을 새로고침한 뒤 다시 선택해 주세요.".into(),
                catalog_revision: state.catalog_revision,
                summary: current_summary,
            })
        }
        Err(TargetBundleValidation::Incompatible) => {
            return Ok(RollbackPresetCatalogResultDto::Rejected {
                schema_version: "preset-catalog-rollback-result/v1".into(),
                reason_code: "target-incompatible".into(),
                message: "선택한 버전은 booth-safe published bundle 기준을 통과하지 못했어요."
                    .into(),
                guidance:
                    "승인된 게시 bundle 상태를 다시 확인하고, 다른 승인 버전을 선택해 주세요."
                        .into(),
                catalog_revision: state.catalog_revision,
                summary: current_summary,
            })
        }
    }

    Ok(RollbackPresetCatalogResultDto::Rejected {
        schema_version: "preset-catalog-rollback-result/v1".into(),
        reason_code: "stage-unavailable".into(),
        message: "이 단계에서는 롤백을 실행하지 않아요.".into(),
        guidance: "approval 준비 상태까지만 확인하고, 실제 롤백은 다음 단계에서 진행해 주세요."
            .into(),
        catalog_revision: state.catalog_revision,
        summary: current_summary,
    })
}

pub fn publish_preset_to_live_catalog(
    base_dir: &Path,
    preset_id: &str,
    published_version: &str,
    actor_id: &str,
    actor_label: &str,
    happened_at: &str,
) -> Result<CatalogActivationOutcome, HostErrorEnvelope> {
    activate_catalog_preset_version(
        base_dir,
        preset_id,
        published_version,
        "published",
        actor_id,
        actor_label,
        happened_at,
    )
}

fn load_or_initialize_catalog_state(
    base_dir: &Path,
) -> Result<CatalogStateRecord, HostErrorEnvelope> {
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundles_by_id = load_published_presets_grouped_by_id(&catalog_root)?;
    let state_path = resolve_catalog_state_path(base_dir);

    if let Some(parsed_state) = read_catalog_state(&state_path)? {
        let normalized = normalize_catalog_state(parsed_state, &bundles_by_id);
        let _ = persist_catalog_state(base_dir, &normalized);

        return Ok(normalized);
    }

    let state =
        build_bootstrap_catalog_state(&bundles_by_id, current_timestamp(SystemTime::now())?);
    let _ = persist_catalog_state(base_dir, &state);

    Ok(state)
}

fn activate_catalog_preset_version(
    base_dir: &Path,
    preset_id: &str,
    target_published_version: &str,
    action_type: &str,
    actor_id: &str,
    actor_label: &str,
    happened_at: &str,
) -> Result<CatalogActivationOutcome, HostErrorEnvelope> {
    activate_catalog_preset_version_with_summary_builder(
        base_dir,
        preset_id,
        target_published_version,
        action_type,
        actor_id,
        actor_label,
        happened_at,
        build_catalog_state_summary_for_preset,
    )
}

fn activate_catalog_preset_version_with_summary_builder<F>(
    base_dir: &Path,
    preset_id: &str,
    target_published_version: &str,
    action_type: &str,
    actor_id: &str,
    actor_label: &str,
    happened_at: &str,
    build_summary: F,
) -> Result<CatalogActivationOutcome, HostErrorEnvelope>
where
    F: Fn(
        &Path,
        &HashMap<String, Vec<PublishedPresetSummaryDto>>,
        &CatalogStateRecord,
        &str,
    ) -> Result<Option<PresetCatalogStateSummaryDto>, HostErrorEnvelope>,
{
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundles_by_id = load_published_presets_grouped_by_id(&catalog_root)?;
    let _ = ensure_target_bundle_is_valid(
        &catalog_root,
        &bundles_by_id,
        preset_id,
        target_published_version,
    )
    .map_err(|_| {
        HostErrorEnvelope::persistence(
            "live catalog version을 바꾸기 전에 게시 bundle 유효성을 다시 확인하지 못했어요.",
        )
    })?;
    let previous_state = load_or_initialize_catalog_state(base_dir)?;
    let previous_history = load_catalog_history(base_dir, preset_id)?;
    let mut next_state = previous_state.clone();
    let previous_live_version = next_state
        .live_presets
        .iter()
        .find(|entry| entry.preset_id == preset_id)
        .map(|entry| entry.published_version.clone());

    if let Some(existing) = next_state
        .live_presets
        .iter_mut()
        .find(|entry| entry.preset_id == preset_id)
    {
        existing.published_version = target_published_version.into();
    } else {
        next_state.live_presets.push(CatalogLivePresetEntry {
            preset_id: preset_id.into(),
            published_version: target_published_version.into(),
        });
    }
    next_state
        .live_presets
        .sort_by(|left, right| left.preset_id.cmp(&right.preset_id));
    next_state.catalog_revision = next_catalog_revision(previous_state.catalog_revision);
    next_state.updated_at = happened_at.into();

    let audit_entry = CatalogVersionHistoryItemDto {
        schema_version: PRESET_CATALOG_HISTORY_SCHEMA_VERSION.into(),
        preset_id: preset_id.into(),
        action_type: action_type.into(),
        from_published_version: previous_live_version.clone(),
        to_published_version: target_published_version.into(),
        actor_id: actor_id.trim().into(),
        actor_label: actor_label.trim().into(),
        happened_at: happened_at.into(),
    };
    let mut next_history = previous_history.clone();
    next_history.push(audit_entry.clone());

    if let Err(error) = persist_catalog_state(base_dir, &next_state) {
        return Err(error);
    }

    if let Err(error) = persist_catalog_history(base_dir, preset_id, &next_history) {
        let _ = persist_catalog_state(base_dir, &previous_state);
        let _ = persist_catalog_history(base_dir, preset_id, &previous_history);
        return Err(error);
    }

    let post_persist_result = (|| -> Result<PresetCatalogStateSummaryDto, HostErrorEnvelope> {
        let refreshed_bundles = load_published_presets_grouped_by_id(&catalog_root)?;
        build_summary(base_dir, &refreshed_bundles, &next_state, preset_id)?.ok_or_else(|| {
            HostErrorEnvelope::persistence(
                "catalog state를 갱신한 뒤 live summary를 다시 계산하지 못했어요.",
            )
        })
    })();
    let summary = match post_persist_result {
        Ok(summary) => summary,
        Err(error) => {
            return Err(rollback_catalog_activation_side_effects(
                base_dir,
                preset_id,
                &previous_state,
                &previous_history,
                error,
            ));
        }
    };

    if action_type == "rollback" {
        try_append_operator_audit_record(
            base_dir,
            OperatorAuditRecordInput {
                occurred_at: happened_at.into(),
                session_id: None,
                event_category: "publication-recovery",
                event_type: "catalog-rollback",
                summary: "future session catalog를 선택한 승인 버전으로 되돌렸어요.".into(),
                detail: "이미 진행 중인 세션 바인딩은 유지한 채 live catalog pointer만 갱신했어요."
                    .into(),
                actor_id: Some(actor_id.trim().into()),
                source: "preset-catalog",
                capture_id: None,
                preset_id: Some(preset_id.into()),
                published_version: Some(target_published_version.into()),
                reason_code: Some("rollback".into()),
            },
        );
    }

    Ok(CatalogActivationOutcome {
        catalog_revision: next_state.catalog_revision,
        summary,
        audit_entry,
    })
}

fn rollback_catalog_activation_side_effects(
    base_dir: &Path,
    preset_id: &str,
    previous_state: &CatalogStateRecord,
    previous_history: &[CatalogVersionHistoryItemDto],
    error: HostErrorEnvelope,
) -> HostErrorEnvelope {
    let mut rollback_failures = Vec::new();

    if let Err(rollback_error) = persist_catalog_state(base_dir, previous_state) {
        rollback_failures.push(format!(
            "catalog state rollback failed: {}",
            rollback_error.message
        ));
    }

    if let Err(rollback_error) = persist_catalog_history(base_dir, preset_id, previous_history) {
        rollback_failures.push(format!(
            "catalog history rollback failed: {}",
            rollback_error.message
        ));
    }

    if rollback_failures.is_empty() {
        error
    } else {
        HostErrorEnvelope::persistence(format!(
            "{} 롤백까지 실패했어요: {}",
            error.message,
            rollback_failures.join(" / ")
        ))
    }
}

fn build_catalog_state_result(
    base_dir: &Path,
) -> Result<PresetCatalogStateResultDto, HostErrorEnvelope> {
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundles_by_id = load_published_presets_grouped_by_id(&catalog_root)?;
    let state = load_or_initialize_catalog_state(base_dir)?;
    let mut preset_ids: Vec<_> = state
        .live_presets
        .iter()
        .map(|entry| entry.preset_id.clone())
        .collect();
    preset_ids.sort();

    let mut presets = Vec::new();

    for preset_id in preset_ids {
        if let Some(summary) =
            build_catalog_state_summary_for_preset(base_dir, &bundles_by_id, &state, &preset_id)?
        {
            presets.push(summary);
        }
    }

    presets.sort_by(|left, right| {
        let left_name = left
            .published_presets
            .iter()
            .find(|preset| preset.published_version == left.live_published_version)
            .map(|preset| preset.display_name.clone())
            .unwrap_or_else(|| left.preset_id.clone());
        let right_name = right
            .published_presets
            .iter()
            .find(|preset| preset.published_version == right.live_published_version)
            .map(|preset| preset.display_name.clone())
            .unwrap_or_else(|| right.preset_id.clone());

        left_name
            .cmp(&right_name)
            .then_with(|| left.preset_id.cmp(&right.preset_id))
    });

    Ok(PresetCatalogStateResultDto {
        schema_version: "preset-catalog-state-result/v1".into(),
        catalog_revision: state.catalog_revision,
        presets,
    })
}

fn build_catalog_state_summary_for_preset(
    base_dir: &Path,
    bundles_by_id: &HashMap<String, Vec<PublishedPresetSummaryDto>>,
    state: &CatalogStateRecord,
    preset_id: &str,
) -> Result<Option<PresetCatalogStateSummaryDto>, HostErrorEnvelope> {
    let Some(published_presets) = bundles_by_id.get(preset_id).cloned() else {
        return Ok(None);
    };
    let Some(live_published_version) = state
        .live_presets
        .iter()
        .find(|entry| entry.preset_id == preset_id)
        .map(|entry| entry.published_version.clone())
    else {
        return Ok(None);
    };

    Ok(Some(PresetCatalogStateSummaryDto {
        preset_id: preset_id.into(),
        live_published_version,
        published_presets,
        version_history: load_catalog_history(base_dir, preset_id)?,
    }))
}

fn build_bootstrap_catalog_state(
    bundles_by_id: &HashMap<String, Vec<PublishedPresetSummaryDto>>,
    updated_at: String,
) -> CatalogStateRecord {
    let mut live_presets = bundles_by_id
        .iter()
        .filter_map(|(preset_id, versions)| {
            versions.last().map(|summary| CatalogLivePresetEntry {
                preset_id: preset_id.clone(),
                published_version: summary.published_version.clone(),
            })
        })
        .collect::<Vec<_>>();
    live_presets.sort_by(|left, right| left.preset_id.cmp(&right.preset_id));

    CatalogStateRecord {
        schema_version: PRESET_CATALOG_STATE_SCHEMA_VERSION.into(),
        catalog_revision: if live_presets.is_empty() { 0 } else { 1 },
        updated_at,
        live_presets,
    }
}

fn normalize_catalog_state(
    mut state: CatalogStateRecord,
    bundles_by_id: &HashMap<String, Vec<PublishedPresetSummaryDto>>,
) -> CatalogStateRecord {
    if state.schema_version != PRESET_CATALOG_STATE_SCHEMA_VERSION {
        return build_bootstrap_catalog_state(bundles_by_id, state.updated_at);
    }

    let mut seen = HashSet::new();
    state.live_presets.retain(|entry| {
        if !seen.insert(entry.preset_id.clone()) {
            return false;
        }

        bundles_by_id
            .get(&entry.preset_id)
            .map(|versions| {
                versions
                    .iter()
                    .any(|summary| summary.published_version == entry.published_version)
            })
            .unwrap_or(false)
    });

    for (preset_id, versions) in bundles_by_id {
        if state
            .live_presets
            .iter()
            .any(|entry| entry.preset_id == *preset_id)
        {
            continue;
        }

        if let Some(summary) = versions.last() {
            state.live_presets.push(CatalogLivePresetEntry {
                preset_id: preset_id.clone(),
                published_version: summary.published_version.clone(),
            });
        }
    }

    state
        .live_presets
        .sort_by(|left, right| left.preset_id.cmp(&right.preset_id));
    if state.catalog_revision == 0 && !state.live_presets.is_empty() {
        state.catalog_revision = 1;
    }

    state
}

fn next_catalog_revision(current_revision: u64) -> u64 {
    if current_revision == 0 {
        1
    } else {
        current_revision + 1
    }
}

fn load_published_presets_grouped_by_id(
    catalog_root: &Path,
) -> Result<HashMap<String, Vec<PublishedPresetSummaryDto>>, HostErrorEnvelope> {
    let mut grouped = HashMap::<String, Vec<PublishedPresetSummaryDto>>::new();

    if !catalog_root.exists() {
        return Ok(grouped);
    }

    let preset_dirs = fs::read_dir(catalog_root).map_err(|error| {
        HostErrorEnvelope::preset_catalog_unavailable(format!(
            "프리셋 카탈로그를 읽지 못했어요: {error}"
        ))
    })?;

    for preset_dir in preset_dirs {
        let preset_dir = match preset_dir {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };

        if !preset_dir.is_dir() {
            continue;
        }

        let version_dirs = match fs::read_dir(&preset_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for version_dir in version_dirs {
            let version_dir = match version_dir {
                Ok(entry) => entry.path(),
                Err(_) => continue,
            };

            if !version_dir.is_dir() {
                continue;
            }

            if let Some(summary) = load_published_preset_summary(&version_dir) {
                grouped
                    .entry(summary.preset_id.clone())
                    .or_default()
                    .push(summary);
            }
        }
    }

    for versions in grouped.values_mut() {
        versions.sort_by(|left, right| left.published_version.cmp(&right.published_version));
    }

    Ok(grouped)
}

fn ensure_target_bundle_is_valid(
    catalog_root: &Path,
    bundles_by_id: &HashMap<String, Vec<PublishedPresetSummaryDto>>,
    preset_id: &str,
    published_version: &str,
) -> Result<PublishedPresetSummaryDto, TargetBundleValidation> {
    if let Some(summary) = bundles_by_id.get(preset_id).and_then(|versions| {
        versions
            .iter()
            .find(|summary| summary.published_version == published_version)
    }) {
        return Ok(summary.clone());
    }

    let bundle_dir = catalog_root.join(preset_id).join(published_version);
    if bundle_dir.exists() {
        Err(TargetBundleValidation::Incompatible)
    } else {
        Err(TargetBundleValidation::Missing)
    }
}

fn read_catalog_state(path: &Path) -> Result<Option<CatalogStateRecord>, HostErrorEnvelope> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read_to_string(path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("catalog state를 읽지 못했어요: {error}"))
    })?;
    let state = serde_json::from_str(&bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("catalog state를 읽지 못했어요: {error}"))
    })?;

    Ok(Some(state))
}

fn resolve_catalog_state_path(base_dir: &Path) -> PathBuf {
    base_dir.join("preset-catalog").join("catalog-state.json")
}

fn resolve_catalog_history_path(base_dir: &Path, preset_id: &str) -> PathBuf {
    base_dir
        .join("preset-catalog")
        .join("catalog-audit")
        .join(format!("{preset_id}.json"))
}

fn load_catalog_history(
    base_dir: &Path,
    preset_id: &str,
) -> Result<Vec<CatalogVersionHistoryItemDto>, HostErrorEnvelope> {
    let history_path = resolve_catalog_history_path(base_dir, preset_id);
    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read_to_string(history_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("catalog audit를 읽지 못했어요: {error}"))
    })?;
    let history =
        serde_json::from_str::<Vec<CatalogVersionHistoryItemDto>>(&bytes).map_err(|error| {
            HostErrorEnvelope::persistence(format!("catalog audit를 읽지 못했어요: {error}"))
        })?;

    Ok(history
        .into_iter()
        .filter(|entry| {
            entry.schema_version == PRESET_CATALOG_HISTORY_SCHEMA_VERSION
                && entry.preset_id == preset_id
                && matches!(entry.action_type.as_str(), "published" | "rollback")
        })
        .collect())
}

fn persist_catalog_state(
    base_dir: &Path,
    state: &CatalogStateRecord,
) -> Result<(), HostErrorEnvelope> {
    let state_path = resolve_catalog_state_path(base_dir);
    let state_dir = state_path
        .parent()
        .ok_or_else(|| HostErrorEnvelope::persistence("catalog state 경로를 준비하지 못했어요."))?;
    fs::create_dir_all(state_dir).map_err(map_fs_error)?;
    let bytes = serde_json::to_vec_pretty(state).map_err(|error| {
        HostErrorEnvelope::persistence(format!("catalog state를 직렬화하지 못했어요: {error}"))
    })?;

    write_json_bytes_atomically(&state_path, &bytes)
}

fn persist_catalog_history(
    base_dir: &Path,
    preset_id: &str,
    history: &[CatalogVersionHistoryItemDto],
) -> Result<(), HostErrorEnvelope> {
    let history_path = resolve_catalog_history_path(base_dir, preset_id);

    if history.is_empty() {
        if history_path.exists() {
            fs::remove_file(&history_path).map_err(map_fs_error)?;
        }

        return Ok(());
    }

    let history_dir = history_path
        .parent()
        .ok_or_else(|| HostErrorEnvelope::persistence("catalog audit 경로를 준비하지 못했어요."))?;
    fs::create_dir_all(history_dir).map_err(map_fs_error)?;
    let bytes = serde_json::to_vec_pretty(history).map_err(|error| {
        HostErrorEnvelope::persistence(format!("catalog audit를 직렬화하지 못했어요: {error}"))
    })?;

    write_json_bytes_atomically(&history_path, &bytes)
}

fn write_json_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), HostErrorEnvelope> {
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

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("catalog state 파일을 저장하지 못했어요: {error}"))
}

enum TargetBundleValidation {
    Missing,
    Incompatible,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn activate_catalog_preset_version_rolls_back_state_and_history_when_summary_build_fails() {
        let base_dir = unique_test_root("catalog-rollback-on-summary-failure");
        let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

        create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
        let _ =
            load_or_initialize_catalog_state(&base_dir).expect("initial catalog state should load");
        create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.26", "Soft Glow");

        let error = activate_catalog_preset_version_with_summary_builder(
            &base_dir,
            "preset_soft-glow",
            "2026.03.26",
            "published",
            "manager-kim",
            "Kim Manager",
            "2026-03-26T00:20:00.000Z",
            |_base_dir, _bundles_by_id, _state, _preset_id| {
                Err(HostErrorEnvelope::persistence(
                    "forced summary failure after catalog update",
                ))
            },
        )
        .expect_err("summary failure should bubble up");

        assert!(error.message.contains("forced summary failure"));

        let state_path = resolve_catalog_state_path(&base_dir);
        let persisted_state = read_catalog_state(&state_path)
            .expect("catalog state should remain readable")
            .expect("catalog state should exist");
        assert_eq!(persisted_state.catalog_revision, 1);
        assert_eq!(persisted_state.live_presets.len(), 1);
        assert_eq!(
            persisted_state.live_presets[0].preset_id,
            "preset_soft-glow"
        );
        assert_eq!(
            persisted_state.live_presets[0].published_version,
            "2026.03.20"
        );

        let history = load_catalog_history(&base_dir, "preset_soft-glow")
            .expect("catalog history should remain readable");
        assert!(history.is_empty());

        let _ = fs::remove_dir_all(base_dir);
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("boothy-{label}-{unique}"))
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
}
