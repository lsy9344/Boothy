use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::session::session_manifest::{
    ActivePresetBinding, SessionCaptureRecord, SessionManifest, SessionPostEnd, SessionTiming,
};

const SESSION_ID_PREFIX: &str = "session_";
const PRESET_ID_PREFIX: &str = "preset_";
const ACTOR_LABEL_MAX_CHARS: usize = 120;
const OPTIONAL_TEXT_MAX_CHARS: usize = 2000;

pub fn is_valid_session_id(session_id: &str) -> bool {
    let suffix = match session_id.strip_prefix(SESSION_ID_PREFIX) {
        Some(suffix) => suffix,
        None => return false,
    };

    suffix.len() == 26 && suffix.chars().all(|char| char.is_ascii_alphanumeric())
}

pub fn is_valid_preset_id(preset_id: &str) -> bool {
    let suffix = match preset_id.strip_prefix(PRESET_ID_PREFIX) {
        Some(suffix) => suffix,
        None => return false,
    };

    !suffix.is_empty()
        && suffix
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-')
}

pub fn is_valid_published_version(published_version: &str) -> bool {
    if published_version.len() != 10 {
        return false;
    }

    published_version
        .chars()
        .enumerate()
        .all(|(index, char)| match index {
            4 | 7 => char == '.',
            _ => char.is_ascii_digit(),
        })
}

pub fn is_non_blank(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn is_trimmed_length_within(value: &str, max_chars: usize) -> bool {
    value.trim().chars().count() <= max_chars
}

pub fn is_valid_actor_id(actor_id: &str) -> bool {
    let mut chars = actor_id.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }

    chars.all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-')
}

pub fn is_valid_darktable_version(value: &str) -> bool {
    let mut segments = value.trim().split('.');
    let Some(major) = segments.next() else {
        return false;
    };
    let Some(minor) = segments.next() else {
        return false;
    };
    let Some(patch) = segments.next() else {
        return false;
    };

    if segments.next().is_some() {
        return false;
    }

    [major, minor, patch]
        .into_iter()
        .all(|segment| !segment.is_empty() && segment.chars().all(|char| char.is_ascii_digit()))
}

pub fn is_safe_workspace_reference(reference: &str) -> bool {
    if !is_non_blank(reference) {
        return false;
    }

    let path = Path::new(reference);

    if path.is_absolute() {
        return false;
    }

    let mut saw_normal_component = false;

    for component in path.components() {
        match component {
            Component::Normal(_) => saw_normal_component = true,
            _ => return false,
        }
    }

    saw_normal_component
}

pub fn is_safe_draft_folder_name(value: &str) -> bool {
    if !is_non_blank(value) {
        return false;
    }

    let path = Path::new(value);

    if path.is_absolute() {
        return false;
    }

    let mut components = path.components();

    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

pub fn validate_session_id(session_id: &str) -> Result<(), HostErrorEnvelope> {
    if is_valid_session_id(session_id) {
        Ok(())
    } else {
        Err(HostErrorEnvelope::validation_message(
            "세션 정보를 다시 확인해 주세요.",
        ))
    }
}

pub fn validate_operator_audit_query_filter(
    input: &OperatorAuditQueryFilterDto,
) -> Result<(), HostErrorEnvelope> {
    if let Some(session_id) = input.session_id.as_deref() {
        validate_session_id(session_id)?;
    }

    if let Some(limit) = input.limit {
        if !(1..=50).contains(&limit) {
            return Err(HostErrorEnvelope::validation_message(
                "audit query limit 범위를 다시 확인해 주세요.",
            ));
        }
    }

    if input.event_categories.len() > 6 {
        return Err(HostErrorEnvelope::validation_message(
            "audit query category 개수를 다시 확인해 주세요.",
        ));
    }

    for category in &input.event_categories {
        if !matches!(
            category.as_str(),
            "session-lifecycle"
                | "timing-transition"
                | "post-end-outcome"
                | "operator-intervention"
                | "publication-recovery"
                | "release-governance"
                | "critical-failure"
        ) {
            return Err(HostErrorEnvelope::validation_message(
                "audit query category 정보를 다시 확인해 주세요.",
            ));
        }
    }

    Ok(())
}

pub fn validate_operator_recovery_action_input(
    input: &OperatorRecoveryActionInputDto,
) -> Result<(), HostErrorEnvelope> {
    validate_session_id(&input.session_id)?;

    if !matches!(
        input.action.as_str(),
        "retry" | "approved-boundary-restart" | "approved-time-extension" | "route-phone-required"
    ) {
        return Err(HostErrorEnvelope::validation_message(
            "복구 액션 정보를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_preset_selection_input(
    preset_id: &str,
    published_version: &str,
) -> Result<(), HostErrorEnvelope> {
    if !is_valid_preset_id(preset_id) || !is_valid_published_version(published_version) {
        return Err(HostErrorEnvelope::validation_message(
            "프리셋 정보를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_draft_preset_edit_input(
    input: &DraftPresetEditPayloadDto,
) -> Result<(), HostErrorEnvelope> {
    if !is_valid_preset_id(&input.preset_id) {
        return Err(HostErrorEnvelope::validation_message(
            "draft presetId 형식을 다시 확인해 주세요.",
        ));
    }

    if !is_non_blank(&input.display_name) {
        return Err(HostErrorEnvelope::validation_message(
            "draft 이름을 입력해 주세요.",
        ));
    }

    if input.lifecycle_state != "draft" {
        return Err(HostErrorEnvelope::validation_message(
            "draft 저장 요청은 항상 draft lifecycle이어야 해요.",
        ));
    }

    if !is_valid_darktable_version(&input.darktable_version) {
        return Err(HostErrorEnvelope::validation_message(
            "darktableVersion 형식을 `5.4.1`처럼 맞춰 주세요.",
        ));
    }

    if !is_safe_workspace_reference(&input.darktable_project_path)
        || !is_safe_workspace_reference(&input.xmp_template_path)
        || !is_non_blank(&input.preview_profile.profile_id)
        || !is_non_blank(&input.preview_profile.display_name)
        || !is_non_blank(&input.preview_profile.output_color_space)
        || !is_non_blank(&input.final_profile.profile_id)
        || !is_non_blank(&input.final_profile.display_name)
        || !is_non_blank(&input.final_profile.output_color_space)
        || !is_non_blank(&input.noise_policy.policy_id)
        || !is_non_blank(&input.noise_policy.display_name)
        || !is_non_blank(&input.noise_policy.reduction_mode)
        || !is_safe_workspace_reference(&input.preview.asset_path)
        || !is_non_blank(&input.preview.alt_text)
        || !is_safe_workspace_reference(&input.sample_cut.asset_path)
        || !is_non_blank(&input.sample_cut.alt_text)
    {
        return Err(HostErrorEnvelope::validation_message(
            "작업공간 안의 안전한 draft metadata와 artifact 참조만 저장할 수 있어요.",
        ));
    }

    Ok(())
}

pub fn validate_draft_validation_input(
    input: &ValidateDraftPresetInputDto,
) -> Result<(), HostErrorEnvelope> {
    if !is_valid_preset_id(&input.preset_id) {
        return Err(HostErrorEnvelope::validation_message(
            "검증할 draft presetId 형식을 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_repair_invalid_draft_input(
    input: &RepairInvalidDraftInputDto,
) -> Result<(), HostErrorEnvelope> {
    if !is_safe_draft_folder_name(&input.draft_folder) {
        return Err(HostErrorEnvelope::validation_message(
            "정리할 손상 draft 폴더 이름을 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_publish_validated_preset_input(
    input: &PublishValidatedPresetInputDto,
) -> Result<(), HostErrorEnvelope> {
    if !is_valid_preset_id(&input.preset_id) {
        return Err(HostErrorEnvelope::validation_message(
            "게시할 draft presetId 형식을 다시 확인해 주세요.",
        ));
    }

    if input.draft_version == 0 || !is_non_blank(&input.validation_checked_at) {
        return Err(HostErrorEnvelope::validation_message(
            "승인 기준이 된 draft version과 validation 시간을 함께 보내 주세요.",
        ));
    }

    if !is_non_blank(&input.expected_display_name) {
        return Err(HostErrorEnvelope::validation_message(
            "게시 전에 검토한 preset 이름을 다시 확인해 주세요.",
        ));
    }

    if !is_valid_published_version(&input.published_version) {
        return Err(HostErrorEnvelope::validation_message(
            "publishedVersion 형식을 `2026.03.26`처럼 맞춰 주세요.",
        ));
    }

    if !is_valid_actor_id(&input.actor_id)
        || !is_non_blank(&input.actor_label)
        || !is_trimmed_length_within(&input.actor_label, ACTOR_LABEL_MAX_CHARS)
    {
        return Err(HostErrorEnvelope::validation_message(
            "게시 승인자를 다시 확인해 주세요.",
        ));
    }

    if input
        .review_note
        .as_deref()
        .map(|note| is_trimmed_length_within(note, OPTIONAL_TEXT_MAX_CHARS))
        == Some(false)
    {
        return Err(HostErrorEnvelope::validation_message(
            "검토 메모는 2000자 이하여야 해요.",
        ));
    }

    if !matches!(
        input.scope.as_str(),
        "future-sessions-only" | "active-session"
    ) {
        return Err(HostErrorEnvelope::validation_message(
            "게시 범위를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_rollback_preset_catalog_input(
    input: &RollbackPresetCatalogInputDto,
) -> Result<(), HostErrorEnvelope> {
    if !is_valid_preset_id(&input.preset_id) {
        return Err(HostErrorEnvelope::validation_message(
            "롤백할 presetId 형식을 다시 확인해 주세요.",
        ));
    }

    if !is_valid_published_version(&input.target_published_version) {
        return Err(HostErrorEnvelope::validation_message(
            "롤백 target version 형식을 `2026.03.26`처럼 맞춰 주세요.",
        ));
    }

    if !is_valid_actor_id(&input.actor_id)
        || !is_non_blank(&input.actor_label)
        || !is_trimmed_length_within(&input.actor_label, ACTOR_LABEL_MAX_CHARS)
    {
        return Err(HostErrorEnvelope::validation_message(
            "롤백 승인자를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn is_valid_branch_id(branch_id: &str) -> bool {
    let mut chars = branch_id.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !first.is_ascii_alphanumeric() {
        return false;
    }

    branch_id.len() <= 48 && chars.all(|char| char.is_ascii_alphanumeric() || char == '-')
}

pub fn is_valid_build_version(build_version: &str) -> bool {
    let Some(version) = build_version.strip_prefix("boothy-") else {
        return false;
    };
    let mut parts = version.split('.');
    let Some(year) = parts.next() else {
        return false;
    };
    let Some(month) = parts.next() else {
        return false;
    };
    let Some(day) = parts.next() else {
        return false;
    };
    let Some(revision) = parts.next() else {
        return false;
    };

    parts.next().is_none()
        && year.len() == 4
        && month.len() == 2
        && day.len() == 2
        && !revision.is_empty()
        && [year, month, day, revision]
            .into_iter()
            .all(|segment| segment.chars().all(|char| char.is_ascii_digit()))
}

pub fn is_valid_preset_stack_version(preset_stack_version: &str) -> bool {
    let Some(version) = preset_stack_version.strip_prefix("catalog-") else {
        return false;
    };
    is_valid_published_version(version)
}

pub fn validate_branch_rollout_input(
    input: &BranchRolloutInputDto,
) -> Result<(), HostErrorEnvelope> {
    if input.branch_ids.is_empty() || input.branch_ids.len() > 20 {
        return Err(HostErrorEnvelope::validation_message(
            "배포 대상 지점 수를 다시 확인해 주세요.",
        ));
    }

    for branch_id in &input.branch_ids {
        if !is_valid_branch_id(branch_id) {
            return Err(HostErrorEnvelope::validation_message(
                "배포 대상 지점 식별자를 다시 확인해 주세요.",
            ));
        }
    }

    if !is_valid_build_version(&input.target_build_version)
        || !is_valid_preset_stack_version(&input.target_preset_stack_version)
    {
        return Err(HostErrorEnvelope::validation_message(
            "release baseline 값을 다시 확인해 주세요.",
        ));
    }

    if !is_valid_actor_id(&input.actor_id)
        || !is_non_blank(&input.actor_label)
        || !is_trimmed_length_within(&input.actor_label, ACTOR_LABEL_MAX_CHARS)
    {
        return Err(HostErrorEnvelope::validation_message(
            "배포 승인자를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_branch_rollback_input(
    input: &BranchRollbackInputDto,
) -> Result<(), HostErrorEnvelope> {
    if input.branch_ids.is_empty() || input.branch_ids.len() > 20 {
        return Err(HostErrorEnvelope::validation_message(
            "롤백 대상 지점 수를 다시 확인해 주세요.",
        ));
    }

    for branch_id in &input.branch_ids {
        if !is_valid_branch_id(branch_id) {
            return Err(HostErrorEnvelope::validation_message(
                "롤백 대상 지점 식별자를 다시 확인해 주세요.",
            ));
        }
    }

    if !is_valid_actor_id(&input.actor_id)
        || !is_non_blank(&input.actor_label)
        || !is_trimmed_length_within(&input.actor_label, ACTOR_LABEL_MAX_CHARS)
    {
        return Err(HostErrorEnvelope::validation_message(
            "롤백 승인자를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartInputDto {
    pub name: String,
    pub phone_last_four: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPresetCatalogInputDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetPreviewAssetDto {
    pub kind: String,
    pub asset_path: String,
    pub alt_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPresetSummaryDto {
    pub preset_id: String,
    pub display_name: String,
    pub published_version: String,
    pub booth_status: String,
    pub preview: PresetPreviewAssetDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPresetPreviewReferenceDto {
    pub asset_path: String,
    pub alt_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftRenderProfileDto {
    pub profile_id: String,
    pub display_name: String,
    pub output_color_space: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftNoisePolicyDto {
    pub policy_id: String,
    pub display_name: String,
    pub reduction_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftValidationFindingDto {
    pub rule_code: String,
    pub severity: String,
    pub field_path: Option<String>,
    pub message: String,
    pub guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftValidationReportDto {
    pub schema_version: String,
    pub preset_id: String,
    pub draft_version: u32,
    pub lifecycle_state: String,
    pub status: String,
    pub checked_at: String,
    pub findings: Vec<DraftValidationFindingDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftValidationSnapshotDto {
    pub status: String,
    pub latest_report: Option<DraftValidationReportDto>,
    pub history: Vec<DraftValidationReportDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetPublicationAuditRecordDto {
    pub schema_version: String,
    pub preset_id: String,
    pub draft_version: u32,
    pub published_version: String,
    pub actor_id: String,
    pub actor_label: String,
    pub review_note: Option<String>,
    pub action: String,
    pub reason_code: Option<String>,
    pub guidance: String,
    pub noted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPresetSummaryDto {
    pub schema_version: String,
    pub preset_id: String,
    pub display_name: String,
    pub draft_version: u32,
    pub lifecycle_state: String,
    pub darktable_version: String,
    pub darktable_project_path: String,
    pub xmp_template_path: String,
    pub preview_profile: DraftRenderProfileDto,
    pub final_profile: DraftRenderProfileDto,
    pub noise_policy: DraftNoisePolicyDto,
    pub preview: DraftPresetPreviewReferenceDto,
    pub sample_cut: DraftPresetPreviewReferenceDto,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub validation: DraftValidationSnapshotDto,
    #[serde(default)]
    pub publication_history: Vec<PresetPublicationAuditRecordDto>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPresetEditPayloadDto {
    pub preset_id: String,
    pub display_name: String,
    pub lifecycle_state: String,
    pub darktable_version: String,
    pub darktable_project_path: String,
    pub xmp_template_path: String,
    pub preview_profile: DraftRenderProfileDto,
    pub final_profile: DraftRenderProfileDto,
    pub noise_policy: DraftNoisePolicyDto,
    pub preview: DraftPresetPreviewReferenceDto,
    pub sample_cut: DraftPresetPreviewReferenceDto,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateDraftPresetInputDto {
    pub preset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairInvalidDraftInputDto {
    pub draft_folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringWorkspaceResultDto {
    pub schema_version: String,
    pub supported_lifecycle_states: Vec<String>,
    pub drafts: Vec<DraftPresetSummaryDto>,
    #[serde(default)]
    pub invalid_drafts: Vec<InvalidDraftArtifactDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateDraftPresetResultDto {
    pub schema_version: String,
    pub draft: DraftPresetSummaryDto,
    pub report: DraftValidationReportDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidDraftArtifactDto {
    pub draft_folder: String,
    pub message: String,
    pub guidance: String,
    #[serde(default)]
    pub can_repair: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishValidatedPresetInputDto {
    pub preset_id: String,
    pub draft_version: u32,
    pub validation_checked_at: String,
    pub expected_display_name: String,
    pub published_version: String,
    pub actor_id: String,
    pub actor_label: String,
    pub scope: String,
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishValidatedPresetSuccessDto {
    pub schema_version: String,
    pub status: String,
    pub draft: DraftPresetSummaryDto,
    pub published_preset: PublishedPresetSummaryDto,
    pub bundle_path: String,
    pub audit_record: PresetPublicationAuditRecordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishValidatedPresetRejectionDto {
    pub schema_version: String,
    pub status: String,
    pub draft: DraftPresetSummaryDto,
    pub reason_code: String,
    pub message: String,
    pub guidance: String,
    pub audit_record: PresetPublicationAuditRecordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum PublishValidatedPresetResultDto {
    #[serde(rename_all = "camelCase")]
    Published {
        schema_version: String,
        draft: DraftPresetSummaryDto,
        published_preset: PublishedPresetSummaryDto,
        bundle_path: String,
        audit_record: PresetPublicationAuditRecordDto,
    },
    #[serde(rename_all = "camelCase")]
    Rejected {
        schema_version: String,
        draft: DraftPresetSummaryDto,
        reason_code: String,
        message: String,
        guidance: String,
        audit_record: PresetPublicationAuditRecordDto,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersionHistoryItemDto {
    pub schema_version: String,
    pub preset_id: String,
    pub action_type: String,
    pub from_published_version: Option<String>,
    pub to_published_version: String,
    pub actor_id: String,
    pub actor_label: String,
    pub happened_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCatalogStateSummaryDto {
    pub preset_id: String,
    pub live_published_version: String,
    pub published_presets: Vec<PublishedPresetSummaryDto>,
    pub version_history: Vec<CatalogVersionHistoryItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCatalogStateResultDto {
    pub schema_version: String,
    pub catalog_revision: u64,
    pub presets: Vec<PresetCatalogStateSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackPresetCatalogInputDto {
    pub preset_id: String,
    pub target_published_version: String,
    pub expected_catalog_revision: u64,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum RollbackPresetCatalogResultDto {
    #[serde(rename_all = "camelCase")]
    RolledBack {
        schema_version: String,
        catalog_revision: u64,
        summary: PresetCatalogStateSummaryDto,
        audit_entry: CatalogVersionHistoryItemDto,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    Rejected {
        schema_version: String,
        reason_code: String,
        message: String,
        guidance: String,
        catalog_revision: u64,
        summary: Option<PresetCatalogStateSummaryDto>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySnapshotDto {
    pub is_admin_authenticated: bool,
    pub allowed_surfaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorBoundarySummaryDto {
    pub status: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorCameraConnectionSummaryDto {
    pub state: String,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveCaptureTruthDto {
    pub source: String,
    pub freshness: String,
    pub session_match: String,
    pub camera_state: String,
    pub helper_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_code: Option<String>,
}

impl LiveCaptureTruthDto {
    pub fn unknown() -> Self {
        Self {
            source: "unknown".into(),
            freshness: "missing".into(),
            session_match: "unknown".into(),
            camera_state: "unknown".into(),
            helper_state: "unknown".into(),
            observed_at: None,
            sequence: None,
            detail_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorAuditEntryDto {
    pub schema_version: String,
    pub event_id: String,
    pub occurred_at: String,
    pub session_id: Option<String>,
    pub event_category: String,
    pub event_type: String,
    pub summary: String,
    pub detail: String,
    pub actor_id: Option<String>,
    pub source: String,
    pub capture_id: Option<String>,
    pub preset_id: Option<String>,
    pub published_version: Option<String>,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorAuditQueryFilterDto {
    pub session_id: Option<String>,
    #[serde(default)]
    pub event_categories: Vec<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorAuditLatestOutcomeDto {
    pub occurred_at: String,
    pub event_category: String,
    pub event_type: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorAuditQuerySummaryDto {
    pub total_events: u32,
    pub session_lifecycle_events: u32,
    pub timing_transition_events: u32,
    pub post_end_outcome_events: u32,
    pub operator_intervention_events: u32,
    pub publication_recovery_events: u32,
    pub release_governance_events: u32,
    pub critical_failure_events: u32,
    pub latest_outcome: Option<OperatorAuditLatestOutcomeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorAuditQueryResultDto {
    pub schema_version: String,
    pub filter: OperatorAuditQueryFilterDto,
    pub events: Vec<OperatorAuditEntryDto>,
    pub summary: OperatorAuditQuerySummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRecentFailureSummaryDto {
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRecoveryDiagnosticsSummaryDto {
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorSessionSummaryDto {
    pub schema_version: String,
    pub state: String,
    pub blocked_state_category: String,
    pub session_id: Option<String>,
    pub booth_alias: Option<String>,
    pub active_preset_id: Option<String>,
    pub active_preset_display_name: Option<String>,
    pub active_preset_version: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub timing_phase: Option<String>,
    pub updated_at: Option<String>,
    pub post_end_state: Option<String>,
    pub recent_failure: Option<OperatorRecentFailureSummaryDto>,
    pub camera_connection: OperatorCameraConnectionSummaryDto,
    pub capture_boundary: OperatorBoundarySummaryDto,
    pub preview_render_boundary: OperatorBoundarySummaryDto,
    pub completion_boundary: OperatorBoundarySummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_capture_truth: Option<LiveCaptureTruthDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRecoverySummaryDto {
    pub schema_version: String,
    pub state: String,
    pub blocked_state_category: String,
    pub blocked_category: Option<String>,
    pub diagnostics_summary: Option<OperatorRecoveryDiagnosticsSummaryDto>,
    pub allowed_actions: Vec<String>,
    pub session_id: Option<String>,
    pub booth_alias: Option<String>,
    pub active_preset_id: Option<String>,
    pub active_preset_display_name: Option<String>,
    pub active_preset_version: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub timing_phase: Option<String>,
    pub updated_at: Option<String>,
    pub post_end_state: Option<String>,
    pub recent_failure: Option<OperatorRecentFailureSummaryDto>,
    pub camera_connection: OperatorCameraConnectionSummaryDto,
    pub capture_boundary: OperatorBoundarySummaryDto,
    pub preview_render_boundary: OperatorBoundarySummaryDto,
    pub completion_boundary: OperatorBoundarySummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_capture_truth: Option<LiveCaptureTruthDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRecoveryActionInputDto {
    pub session_id: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRecoveryNextStateDto {
    pub customer_state: String,
    pub reason_code: String,
    pub lifecycle_stage: Option<String>,
    pub timing_phase: Option<String>,
    pub post_end_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRecoveryActionResultDto {
    pub schema_version: String,
    pub session_id: String,
    pub action: String,
    pub status: String,
    pub message: String,
    pub rejection_reason: Option<String>,
    pub diagnostics_summary: Option<OperatorRecoveryDiagnosticsSummaryDto>,
    pub next_state: OperatorRecoveryNextStateDto,
    pub summary: OperatorRecoverySummaryDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchReleaseBaselineDto {
    pub build_version: String,
    pub preset_stack_version: String,
    pub approved_at: String,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutApprovalDto {
    pub approved_at: String,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchLocalSettingsPreservationDto {
    pub preserved_fields: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchActiveSessionDto {
    pub session_id: String,
    pub locked_baseline: BranchReleaseBaselineDto,
    pub started_at: String,
    pub safe_transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchCompatibilityVerdictDto {
    pub status: String,
    pub summary: String,
    pub session_baseline: Option<BranchReleaseBaselineDto>,
    pub safe_transition_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutRejectionDto {
    pub code: String,
    pub message: String,
    pub guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutBranchStateDto {
    pub branch_id: String,
    pub display_name: String,
    pub deployment_baseline: BranchReleaseBaselineDto,
    pub rollback_baseline: Option<BranchReleaseBaselineDto>,
    pub pending_baseline: Option<BranchReleaseBaselineDto>,
    pub local_settings: BranchLocalSettingsPreservationDto,
    pub active_session: Option<BranchActiveSessionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutBranchResultDto {
    pub branch_id: String,
    pub display_name: String,
    pub result: String,
    pub effective_baseline: BranchReleaseBaselineDto,
    pub pending_baseline: Option<BranchReleaseBaselineDto>,
    pub local_settings: BranchLocalSettingsPreservationDto,
    pub compatibility: BranchCompatibilityVerdictDto,
    pub rejection: Option<BranchRolloutRejectionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutAuditEntryDto {
    pub schema_version: String,
    pub audit_id: String,
    pub action: String,
    pub requested_branch_ids: Vec<String>,
    pub target_baseline: Option<BranchReleaseBaselineDto>,
    pub approval: BranchRolloutApprovalDto,
    pub outcomes: Vec<BranchRolloutBranchResultDto>,
    pub noted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutOverviewResultDto {
    pub schema_version: String,
    pub approved_baselines: Vec<BranchReleaseBaselineDto>,
    pub branches: Vec<BranchRolloutBranchStateDto>,
    pub recent_history: Vec<BranchRolloutAuditEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutInputDto {
    pub branch_ids: Vec<String>,
    pub target_build_version: String,
    pub target_preset_stack_version: String,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRollbackInputDto {
    pub branch_ids: Vec<String>,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRolloutActionResultDto {
    pub schema_version: String,
    pub action: String,
    pub requested_branch_ids: Vec<String>,
    pub target_baseline: Option<BranchReleaseBaselineDto>,
    pub approval: BranchRolloutApprovalDto,
    pub outcomes: Vec<BranchRolloutBranchResultDto>,
    pub audit_entry: BranchRolloutAuditEntryDto,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCatalogResultDto {
    pub session_id: String,
    pub state: String,
    pub presets: Vec<PublishedPresetSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSelectionInputDto {
    pub session_id: String,
    pub preset_id: String,
    pub published_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSelectionResultDto {
    pub session_id: String,
    pub active_preset: ActivePresetBinding,
    pub manifest: SessionManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReadinessInputDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequestInputDto {
    pub session_id: String,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDeleteInputDto {
    pub session_id: String,
    pub capture_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReadinessDto {
    pub schema_version: String,
    pub session_id: String,
    pub surface_state: String,
    pub customer_state: String,
    pub can_capture: bool,
    pub primary_action: String,
    pub customer_message: String,
    pub support_message: String,
    pub reason_code: String,
    pub latest_capture: Option<SessionCaptureRecord>,
    /// 클라이언트가 카드 상태를 되돌릴 수 있게 하는 화해(reconciliation) 창.
    ///
    /// `latest_capture` 하나만 보내면, 다음 촬영이 접수되는 순간 이전 촬영은 더 이상
    /// latest가 아니므로 그 촬영의 렌더 완료가 클라이언트에 **영원히 도달하지 못한다.**
    /// 2026-08-12 실장비 검증에서 첫 사진 카드가 완료된 뒤에도 `마무리 중`에 남은 원인이다.
    /// 목록은 manifest 순서(오래된 것 -> 최신)이며 `latest_capture`도 포함한다.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_captures: Vec<SessionCaptureRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_capture_truth: Option<LiveCaptureTruthDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_end: Option<SessionPostEnd>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<SessionTiming>,
}

impl CaptureReadinessDto {
    fn build(
        session_id: impl Into<String>,
        surface_state: impl Into<String>,
        customer_state: impl Into<String>,
        can_capture: bool,
        primary_action: impl Into<String>,
        customer_message: impl Into<String>,
        support_message: impl Into<String>,
        reason_code: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        Self {
            schema_version: "capture-readiness/v1".into(),
            session_id: session_id.into(),
            surface_state: surface_state.into(),
            customer_state: customer_state.into(),
            can_capture,
            primary_action: primary_action.into(),
            customer_message: customer_message.into(),
            support_message: support_message.into(),
            reason_code: reason_code.into(),
            latest_capture,
            recent_captures: Vec::new(),
            live_capture_truth: None,
            post_end: None,
            timing: None,
        }
    }

    pub fn with_post_end(mut self, post_end: Option<SessionPostEnd>) -> Self {
        self.post_end = post_end;
        self
    }

    pub fn with_timing(mut self, timing: Option<SessionTiming>) -> Self {
        self.timing = timing;
        self
    }

    pub fn with_latest_capture(mut self, latest_capture: Option<SessionCaptureRecord>) -> Self {
        self.latest_capture = latest_capture;
        self
    }

    pub fn with_recent_captures(mut self, recent_captures: Vec<SessionCaptureRecord>) -> Self {
        self.recent_captures = recent_captures;
        self
    }

    pub fn with_live_capture_truth(mut self, live_capture_truth: LiveCaptureTruthDto) -> Self {
        self.live_capture_truth = Some(live_capture_truth);
        self
    }

    pub fn preset_missing(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "choose-preset",
            "촬영 전에 룩을 먼저 골라 주세요.",
            "선택이 끝나면 바로 찍을 수 있어요.",
            "preset-missing",
            None,
        )
    }

    pub fn camera_preparing(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "촬영 준비 중이에요.",
            "잠시만 기다려 주세요.",
            "camera-preparing",
            None,
        )
    }

    pub fn camera_waiting_for_power(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "카메라 전원을 확인하고 있어요.",
            "카메라를 켜고 연결이 안정되면 바로 촬영할 수 있어요.",
            "camera-preparing",
            None,
        )
    }

    pub fn camera_connecting(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "카메라를 확인했고 연결을 마무리하고 있어요.",
            "잠시만 기다려 주세요.",
            "camera-preparing",
            None,
        )
    }

    pub fn helper_preparing(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "촬영 준비 중이에요.",
            "잠시만 기다려 주세요.",
            "helper-preparing",
            None,
        )
    }

    pub fn viewer_preparing(session_id: impl Into<String>) -> Self {
        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "화면을 준비하고 있어요.",
            "곧 촬영을 시작할 수 있어요.",
            "viewer-preparing",
            None,
        )
    }

    pub fn capture_retry_required(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "사진을 아직 찍지 못했어요.",
            "대상을 다시 맞춘 뒤 잠시 후 다시 시도해 주세요.",
            "capture-retry-required",
            latest_capture,
        )
    }

    pub fn preview_waiting(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "previewWaiting",
            "Preview Waiting",
            false,
            "wait",
            "사진이 안전하게 저장되었어요.",
            "확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.",
            "preview-waiting",
            latest_capture,
        )
    }

    pub fn export_waiting(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Export Waiting",
            false,
            "wait",
            "촬영은 끝났고 결과를 준비하고 있어요.",
            "다음 안내가 나올 때까지 잠시만 기다려 주세요.",
            "export-waiting",
            latest_capture,
        )
    }

    pub fn completed(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Completed",
            false,
            "wait",
            "부스 준비가 끝났어요.",
            "다음 안내를 확인해 주세요.",
            "completed",
            latest_capture,
        )
    }

    pub fn phone_required(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Phone Required",
            false,
            "call-support",
            "지금은 도움이 필요해요.",
            "가까운 직원에게 알려 주세요.",
            "phone-required",
            None,
        )
    }

    pub fn warning(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "captureReady",
            "Ready",
            true,
            "capture",
            "지금 촬영할 수 있어요.",
            "남은 시간 안에 계속 찍을 수 있어요.",
            "warning",
            latest_capture,
        )
    }

    pub fn ended(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Session Ended",
            false,
            "wait",
            "촬영 시간이 끝났어요.",
            "마무리 안내가 나올 때까지 잠시만 기다려 주세요.",
            "ended",
            latest_capture,
        )
    }

    pub fn ready(
        session_id: impl Into<String>,
        surface_state: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            surface_state,
            "Ready",
            true,
            "capture",
            "지금 촬영할 수 있어요.",
            "버튼을 누르면 바로 시작돼요.",
            "ready",
            latest_capture,
        )
    }

    pub fn capture_saved(
        session_id: impl Into<String>,
        latest_capture: SessionCaptureRecord,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "captureSaved",
            "Preview Waiting",
            false,
            "wait",
            "사진이 안전하게 저장되었어요.",
            "확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.",
            "preview-waiting",
            Some(latest_capture),
        )
    }

    pub fn preview_ready(
        session_id: impl Into<String>,
        latest_capture: SessionCaptureRecord,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "previewReady",
            "Ready",
            true,
            "capture",
            "지금 촬영할 수 있어요.",
            "방금 찍은 사진을 아래에서 바로 확인할 수 있어요.",
            "ready",
            Some(latest_capture),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReadinessUpdateDto {
    pub schema_version: String,
    pub session_id: String,
    pub readiness: CaptureReadinessDto,
}

impl CaptureReadinessUpdateDto {
    pub fn new(session_id: impl Into<String>, readiness: CaptureReadinessDto) -> Self {
        Self {
            schema_version: "capture-readiness-update/v1".into(),
            session_id: session_id.into(),
            readiness,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFastPreviewUpdateDto {
    pub schema_version: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub asset_path: String,
    pub visible_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

impl CaptureFastPreviewUpdateDto {
    pub fn new(
        session_id: impl Into<String>,
        request_id: impl Into<String>,
        capture_id: impl Into<String>,
        asset_path: impl Into<String>,
        visible_at_ms: u64,
        kind: Option<String>,
    ) -> Self {
        Self {
            schema_version: "capture-fast-preview-update/v1".into(),
            session_id: session_id.into(),
            request_id: request_id.into(),
            capture_id: capture_id.into(),
            asset_path: asset_path.into(),
            visible_at_ms,
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequestResultDto {
    pub schema_version: String,
    pub session_id: String,
    pub status: String,
    pub capture: SessionCaptureRecord,
    pub readiness: CaptureReadinessDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDeleteResultDto {
    pub schema_version: String,
    pub session_id: String,
    pub capture_id: String,
    pub status: String,
    pub manifest: SessionManifest,
    pub readiness: CaptureReadinessDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostFieldErrors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_last_four: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostErrorEnvelope {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness: Option<CaptureReadinessDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_errors: Option<HostFieldErrors>,
}

impl HostErrorEnvelope {
    pub fn capability_denied(message: impl Into<String>) -> Self {
        Self {
            code: "capability-denied".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn validation_message(message: impl Into<String>) -> Self {
        Self {
            code: "validation-error".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn validation(field_errors: HostFieldErrors) -> Self {
        Self {
            code: "validation-error".into(),
            message: "입력한 내용을 다시 확인해 주세요.".into(),
            readiness: None,
            field_errors: Some(field_errors),
        }
    }

    pub fn persistence(message: impl Into<String>) -> Self {
        Self {
            code: "session-persistence-failed".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn session_not_found(message: impl Into<String>) -> Self {
        Self {
            code: "session-not-found".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn preset_catalog_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "preset-catalog-unavailable".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn preset_not_available(message: impl Into<String>) -> Self {
        Self {
            code: "preset-not-available".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn capture_not_ready(message: impl Into<String>, readiness: CaptureReadinessDto) -> Self {
        Self {
            code: "capture-not-ready".into(),
            message: message.into(),
            readiness: Some(readiness),
            field_errors: None,
        }
    }

    pub fn capture_delete_blocked(
        message: impl Into<String>,
        readiness: CaptureReadinessDto,
    ) -> Self {
        Self {
            code: "capture-delete-blocked".into(),
            message: message.into(),
            readiness: Some(readiness),
            field_errors: None,
        }
    }
}

pub const VIEWER_READINESS_SCHEMA_VERSION: &str = "viewer-readiness/v1";
pub const VIEWER_READINESS_UPDATE_SCHEMA_VERSION: &str = "viewer-readiness-update/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerDisplayProfileDto {
    pub profile_id: String,
    pub monitor_name: Option<String>,
    pub monitor_width_px: u32,
    pub monitor_height_px: u32,
    pub monitor_scale_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerPhotoRectDto {
    pub css_width: f64,
    pub css_height: f64,
    pub device_pixel_ratio: f64,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerReadinessSnapshotDto {
    pub schema_version: String,
    pub session_id: Option<String>,
    pub viewer_epoch: u64,
    pub revision: u64,
    pub window_state: String,
    pub listener_ready: bool,
    pub layout_ready: bool,
    pub monitor_targeting: String,
    pub display_profile: Option<ViewerDisplayProfileDto>,
    pub photo_rect: Option<ViewerPhotoRectDto>,
    pub viewer_ready: bool,
    pub reason_code: String,
    pub observed_at_ms: u64,
    pub last_report_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerReadinessUpdateDto {
    pub schema_version: String,
    pub readiness: ViewerReadinessSnapshotDto,
}

impl ViewerReadinessUpdateDto {
    pub fn new(readiness: ViewerReadinessSnapshotDto) -> Self {
        Self {
            schema_version: VIEWER_READINESS_UPDATE_SCHEMA_VERSION.into(),
            readiness,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerLayoutReportDto {
    pub viewer_epoch: u64,
    pub session_id: Option<String>,
    pub css_width: f64,
    pub css_height: f64,
    pub device_pixel_ratio: f64,
    pub layout_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerListenerReportDto {
    pub viewer_epoch: u64,
}

pub fn validate_viewer_layout_report(
    report: &ViewerLayoutReportDto,
) -> Result<(), HostErrorEnvelope> {
    if let Some(session_id) = report.session_id.as_deref() {
        validate_session_id(session_id)?;
    }

    if !report.css_width.is_finite()
        || !report.css_height.is_finite()
        || !report.device_pixel_ratio.is_finite()
        || report.css_width < 0.0
        || report.css_height < 0.0
        || report.device_pixel_ratio <= 0.0
        || (report.layout_ready && (report.css_width == 0.0 || report.css_height == 0.0))
    {
        return Err(HostErrorEnvelope::validation_message(
            "관람 화면 크기 정보를 다시 확인해 주세요.",
        ));
    }

    let required_source_width = report.css_width * report.device_pixel_ratio;
    let required_source_height = report.css_height * report.device_pixel_ratio;

    if required_source_width > u32::MAX as f64 || required_source_height > u32::MAX as f64 {
        return Err(HostErrorEnvelope::validation_message(
            "관람 화면 크기 정보를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Story 7.2: immutable display generation과 actual-present 계측 계약.
// 필드명/nullability/schemaVersion 문자열은
// `src/shared-contracts/schemas/viewer-display.ts`와 정확히 일치해야 한다.
// ---------------------------------------------------------------------------

pub const VIEWER_DISPLAY_SCHEMA_VERSION: &str = "viewer-display/v1";
pub const VIEWER_DISPLAY_UPDATE_SCHEMA_VERSION: &str = "viewer-display-update/v1";

/// Story 7.2가 등록하는 유일한 tier. 7.4/7.6이 상위 tier를 추가한다.
pub const DISPLAY_TIER_SAMPLE: &str = "sample";

/// tier 순서. 값이 클수록 높은 tier이며 downgrade는 거부된다.
pub fn display_tier_order(tier: &str) -> Option<u8> {
    match tier {
        DISPLAY_TIER_SAMPLE => Some(0),
        _ => None,
    }
}

pub const DISPLAY_REJECT_PARTIAL_FILE: &str = "partial-file";
pub const DISPLAY_REJECT_UNDECODABLE: &str = "undecodable";
pub const DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS: &str = "insufficient-dimensions";
pub const DISPLAY_REJECT_STALE_EPOCH: &str = "stale-epoch";
pub const DISPLAY_REJECT_LOWER_GENERATION: &str = "lower-generation";
pub const DISPLAY_REJECT_OLDER_REQUEST: &str = "older-request";
pub const DISPLAY_REJECT_SESSION_MISMATCH: &str = "session-mismatch";
pub const DISPLAY_REJECT_ORIENTATION_UNSUPPORTED: &str = "orientation-unsupported";
pub const DISPLAY_REJECT_UNKNOWN_GENERATION: &str = "unknown-generation";
pub const DISPLAY_REJECT_TIER_DOWNGRADE: &str = "tier-downgrade";
pub const DISPLAY_REJECT_VIEWER_NOT_READY: &str = "viewer-not-ready";
pub const DISPLAY_REJECT_DECODE_FAILED: &str = "decode-failed";
/// commit·notify까지 끝났지만 유예 시간 안에 viewer의 terminal 보고가 도착하지 않은 generation.
///
/// **host만 이 값을 쓴다.** viewer가 보고할 수 있는 결론이 아니라, 보고 자체가 오지 않았다는
/// host의 관측이다. 이 코드가 없으면 그런 generation은 계측에 **아무 행도 남기지 않아**,
/// "표시되지 않았다"와 "보고가 유실됐다"를 evidence에서 구분할 수 없다.
pub const DISPLAY_REJECT_PRESENT_UNREPORTED: &str = "present-unreported";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayGenerationDto {
    pub generation_id: String,
    pub generation_seq: u64,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: Option<String>,
    pub viewer_epoch: u64,
    pub tier: String,
    pub asset_path: String,
    pub source_width_px: u32,
    pub source_height_px: u32,
    pub byte_size: u64,
    pub source_hash: String,
    pub sample_variant: String,
    pub committed_at_host_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPointerSnapshotDto {
    pub schema_version: String,
    pub session_id: Option<String>,
    pub revision: u64,
    pub active_generation: Option<DisplayGenerationDto>,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
    /// 계측 lane 활성 여부. true면 표시 이미지는 제품 결과가 아니라 계측용 fixture다.
    pub measurement_lane_enabled: bool,
    pub observed_at_host_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayUpdateDto {
    pub schema_version: String,
    pub pointer: DisplayPointerSnapshotDto,
}

impl DisplayUpdateDto {
    pub fn new(pointer: DisplayPointerSnapshotDto) -> Self {
        Self {
            schema_version: VIEWER_DISPLAY_UPDATE_SCHEMA_VERSION.into(),
            pointer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPresentSpansDto {
    pub viewer_receipt_at_micros: Option<u64>,
    pub decode_start_at_micros: Option<u64>,
    pub decode_end_at_micros: Option<u64>,
    pub swap_committed_at_micros: Option<u64>,
    pub img_on_load_at_micros: Option<u64>,
    pub actual_present_at_micros: Option<u64>,
    pub element_timing_render_at_micros: Option<u64>,
    pub is_element_render_time: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPresentReportDto {
    pub generation_id: String,
    pub viewer_epoch: u64,
    pub outcome: String,
    pub reject_reason: Option<String>,
    pub natural_width_px: u32,
    pub natural_height_px: u32,
    pub spans: DisplayPresentSpansDto,
    pub clock_offset_micros: i64,
    pub clock_uncertainty_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockProbeInputDto {
    pub client_sent_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockProbeResultDto {
    pub host_monotonic_micros: u64,
    pub host_epoch_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedInputReportDto {
    pub session_id: String,
    pub request_id: String,
    pub client_input_micros: u64,
    pub clock_offset_micros: i64,
    pub clock_uncertainty_micros: u64,
    pub is_trusted: bool,
}

/// 계측용 sample generation 게시 입력. measurement lane이 켜져 있을 때만 사용한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySamplePublishInputDto {
    pub session_id: String,
    pub request_id: String,
    pub capture_id: Option<String>,
    pub sample_variant: String,
}

pub fn validate_display_present_report(
    report: &DisplayPresentReportDto,
) -> Result<(), HostErrorEnvelope> {
    if report.generation_id.trim().is_empty() {
        return Err(HostErrorEnvelope::validation_message(
            "표시 결과 보고 값을 다시 확인해 주세요.",
        ));
    }

    if !matches!(
        report.outcome.as_str(),
        "presented" | "rejected" | "decode-failed"
    ) {
        return Err(HostErrorEnvelope::validation_message(
            "표시 결과 보고 값을 다시 확인해 주세요.",
        ));
    }

    let known_reject_reason = report
        .reject_reason
        .as_deref()
        .is_some_and(|reject_reason| {
            matches!(
                reject_reason,
                DISPLAY_REJECT_PARTIAL_FILE
                    | DISPLAY_REJECT_UNDECODABLE
                    | DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS
                    | DISPLAY_REJECT_STALE_EPOCH
                    | DISPLAY_REJECT_LOWER_GENERATION
                    | DISPLAY_REJECT_OLDER_REQUEST
                    | DISPLAY_REJECT_SESSION_MISMATCH
                    | DISPLAY_REJECT_ORIENTATION_UNSUPPORTED
                    | DISPLAY_REJECT_UNKNOWN_GENERATION
                    | DISPLAY_REJECT_TIER_DOWNGRADE
                    | DISPLAY_REJECT_VIEWER_NOT_READY
                    | DISPLAY_REJECT_DECODE_FAILED
            )
        });
    let coherent_outcome = match report.outcome.as_str() {
        "presented" => report.reject_reason.is_none(),
        "rejected" => {
            known_reject_reason
                && report.reject_reason.as_deref() != Some(DISPLAY_REJECT_DECODE_FAILED)
        }
        "decode-failed" => report.reject_reason.as_deref() == Some(DISPLAY_REJECT_DECODE_FAILED),
        _ => false,
    };

    if !coherent_outcome {
        return Err(HostErrorEnvelope::validation_message(
            "표시 결과 보고 값을 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_trusted_input_report(
    report: &TrustedInputReportDto,
) -> Result<(), HostErrorEnvelope> {
    validate_session_id(&report.session_id)?;

    if report.request_id.trim().is_empty() {
        return Err(HostErrorEnvelope::validation_message(
            "촬영 입력 계측 값을 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_display_sample_publish_input(
    input: &DisplaySamplePublishInputDto,
) -> Result<(), HostErrorEnvelope> {
    validate_session_id(&input.session_id)?;

    let safe_request_id = input.request_id == input.request_id.trim()
        && input.request_id.chars().count() <= 120
        && !input.request_id.contains(['/', '\\'])
        && is_safe_draft_folder_name(&input.request_id);

    if !safe_request_id || !matches!(input.sample_variant.as_str(), "a" | "b") {
        return Err(HostErrorEnvelope::validation_message(
            "표시 샘플 게시 값을 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Story 7.3: capture source route 비교 계약.
// 필드명/nullability/enum 문자열은
// `src/shared-contracts/schemas/capture-source.ts`와 정확히 일치해야 한다.
// ---------------------------------------------------------------------------

pub const CAPTURE_SOURCE_SCHEMA_VERSION: &str = "capture-source/v1";
pub const SOURCE_COMPARISON_SCHEMA_VERSION: &str = "source-comparison/v1";

/// Route A — RAW 컨테이너에 내장된 full-size JPEG.
/// CR2의 TIFF IFD를 직접 읽는다 (`capture::embedded_jpeg`). 새 의존성을 쓰지 않는다.
pub const SOURCE_ROUTE_EMBEDDED_JPEG: &str = "embedded-jpeg";
/// Route B — 카메라가 RAW와 함께 만든 별도 JPEG transfer object.
pub const SOURCE_ROUTE_CAMERA_PAIRED_JPEG: &str = "camera-paired-jpeg";
/// Route C — 현재 제품에 살아 있는 incumbent. **비교의 기준선이다.**
pub const SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL: &str = "windows-shell-thumbnail";

pub fn is_known_source_route(route: &str) -> bool {
    matches!(
        route,
        SOURCE_ROUTE_EMBEDDED_JPEG
            | SOURCE_ROUTE_CAMERA_PAIRED_JPEG
            | SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL
    )
}

pub const SOURCE_REJECT_ABSENT: &str = "absent";
pub const SOURCE_REJECT_PARTIAL: &str = "partial";
pub const SOURCE_REJECT_CORRUPT: &str = "corrupt";
pub const SOURCE_REJECT_UNDECODABLE: &str = "undecodable";
pub const SOURCE_REJECT_ORIENTATION_UNSUPPORTED: &str = "orientation-unsupported";
pub const SOURCE_REJECT_WRONG_SESSION: &str = "wrong-session";
pub const SOURCE_REJECT_WRONG_REQUEST: &str = "wrong-request";
pub const SOURCE_REJECT_WRONG_CAPTURE: &str = "wrong-capture";
pub const SOURCE_REJECT_STALE: &str = "stale";
pub const SOURCE_REJECT_UNSUPPORTED_COMBINATION: &str = "unsupported-combination";
pub const SOURCE_REJECT_EXTRACTION_FAILED: &str = "extraction-failed";
pub const SOURCE_REJECT_CANCELLED: &str = "cancelled";

pub fn is_known_source_reject_reason(reason: &str) -> bool {
    matches!(
        reason,
        SOURCE_REJECT_ABSENT
            | SOURCE_REJECT_PARTIAL
            | SOURCE_REJECT_CORRUPT
            | SOURCE_REJECT_UNDECODABLE
            | SOURCE_REJECT_ORIENTATION_UNSUPPORTED
            | SOURCE_REJECT_WRONG_SESSION
            | SOURCE_REJECT_WRONG_REQUEST
            | SOURCE_REJECT_WRONG_CAPTURE
            | SOURCE_REJECT_STALE
            | SOURCE_REJECT_UNSUPPORTED_COMBINATION
            | SOURCE_REJECT_EXTRACTION_FAILED
            | SOURCE_REJECT_CANCELLED
    )
}

pub const SOURCE_OBJECT_ROLE_RAW: &str = "raw";
pub const SOURCE_OBJECT_ROLE_JPEG: &str = "jpeg";

pub const SOURCE_BLOCK_ORDER_AB: &str = "AB";
pub const SOURCE_BLOCK_ORDER_BA: &str = "BA";

/// 측정 lane 스위치 값. **기본은 `off`이고, off일 때 제품 경로는 지금과 완전히 동일하다.**
pub const SOURCE_COMPARE_MODE_OFF: &str = "off";
pub const SOURCE_COMPARE_MODE_EMBEDDED: &str = "embedded";
pub const SOURCE_COMPARE_MODE_PAIRED: &str = "paired";
pub const SOURCE_COMPARE_MODE_SHELL: &str = "shell";
pub const SOURCE_COMPARE_MODE_AB: &str = "ab";

pub fn is_known_source_compare_mode(mode: &str) -> bool {
    matches!(
        mode,
        SOURCE_COMPARE_MODE_OFF
            | SOURCE_COMPARE_MODE_EMBEDDED
            | SOURCE_COMPARE_MODE_PAIRED
            | SOURCE_COMPARE_MODE_SHELL
            | SOURCE_COMPARE_MODE_AB
    )
}

/// 한 route가 만들어 낸 source 후보 1건.
///
/// `asset_path`가 `None`이면 후보 자체가 생기지 않은 것이고, 그때도 표본 행은 남는다.
/// **행이 없는 시도를 만들지 않는다** — Story 7.2가 이 결함으로 한 회차를 잃었다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCandidateDto {
    pub capture_id: Option<String>,
    pub request_id: String,
    pub session_id: String,
    pub route: String,
    pub asset_path: Option<String>,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub byte_size: Option<u64>,
    pub exif_orientation: Option<u16>,
    pub decode_valid: bool,
    pub source_hash: Option<String>,
    pub object_index: Option<u32>,
    pub group_id: Option<u32>,
    pub ready_at_host_micros: Option<u64>,
    pub extraction_cost_micros: Option<u64>,
}

/// helper → host. 카메라가 스스로 보고한 image-quality capability descriptor.
///
/// **지원 여부를 가정하지 않는다.** descriptor에 없는 조합은 시도조차 하지 않는다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageQualityCapabilityDto {
    pub descriptor_available: bool,
    pub current_value: Option<i64>,
    pub supported_values: Vec<i64>,
    pub raw_plus_jpeg_supported: bool,
    pub probed_at_host_micros: u64,
}

/// `source-comparison.jsonl` 한 행.
///
/// **Story 7.2의 `viewer-present.jsonl`과 의도적으로 분리된 파일이다.**
/// 두 파일을 합쳐 집계하는 도구를 만들지 않는다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceComparisonSampleDto {
    pub schema_version: String,
    pub candidate: SourceCandidateDto,
    pub accepted: bool,
    pub reject_reason: Option<String>,
    pub object_role: Option<String>,
    /// `groupID`가 없어 파일명 stem + 도착 시각 창으로 묶었는가.
    pub used_fallback_correlation: bool,
    pub block_order: String,
    pub block_index: u32,
    pub is_warm_up: bool,
    pub randomization_seed: u64,
    /// **항상 false다.** 이 lane의 어떤 산출물도 preset이 적용되지 않았다.
    pub is_preset_applied: bool,
    pub recorded_at_host_micros: u64,
}
