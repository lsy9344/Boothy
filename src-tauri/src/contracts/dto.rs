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

    if input
        .darktable_project_path
        .as_deref()
        .map(is_safe_workspace_reference)
        == Some(false)
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

pub fn validate_preview_renderer_route_promotion_input(
    input: &PreviewRendererRoutePromotionInputDto,
) -> Result<(), HostErrorEnvelope> {
    validate_preset_selection_input(&input.preset_id, &input.published_version)?;

    if !matches!(input.target_route_stage.as_str(), "canary" | "default") {
        return Err(HostErrorEnvelope::validation_message(
            "승격할 preview route stage를 다시 확인해 주세요.",
        ));
    }

    if !is_valid_actor_id(&input.actor_id)
        || !is_non_blank(&input.actor_label)
        || !is_trimmed_length_within(&input.actor_label, ACTOR_LABEL_MAX_CHARS)
    {
        return Err(HostErrorEnvelope::validation_message(
            "승격 승인자를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

pub fn validate_preview_renderer_route_rollback_input(
    input: &PreviewRendererRouteRollbackInputDto,
) -> Result<(), HostErrorEnvelope> {
    validate_preset_selection_input(&input.preset_id, &input.published_version)?;

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

pub fn validate_preview_renderer_route_status_input(
    input: &PreviewRendererRouteStatusInputDto,
) -> Result<(), HostErrorEnvelope> {
    validate_preset_selection_input(&input.preset_id, &input.published_version)
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
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPresetRecipeDto {
    pub schema_version: String,
    pub preset_id: String,
    pub published_version: String,
    pub display_name: String,
    pub booth_status: String,
    pub preview_intent: DraftRenderProfileDto,
    pub final_intent: DraftRenderProfileDto,
    pub noise_policy: DraftNoisePolicyDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct DarktablePresetAdapterDto {
    pub schema_version: String,
    pub darktable_version: String,
    pub xmp_template_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub darktable_project_path: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub darktable_project_path: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub darktable_project_path: Option<String>,
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
pub struct OperatorPreviewArchitectureSummaryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_state_observed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_visible_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_visible_to_preset_applied_visible_ms: Option<u64>,
    pub hardware_capability: String,
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
    pub preview_architecture: OperatorPreviewArchitectureSummaryDto,
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
    pub preview_architecture: OperatorPreviewArchitectureSummaryDto,
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
pub struct PreviewRendererRoutePromotionInputDto {
    pub preset_id: String,
    pub published_version: String,
    pub target_route_stage: String,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererRouteRollbackInputDto {
    pub preset_id: String,
    pub published_version: String,
    pub actor_id: String,
    pub actor_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererRouteStatusInputDto {
    pub preset_id: String,
    pub published_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererRoutePolicyAuditEntryDto {
    pub schema_version: String,
    pub audit_id: String,
    pub action: String,
    pub preset_id: String,
    pub published_version: String,
    pub target_route_stage: String,
    pub approval: BranchRolloutApprovalDto,
    pub result: String,
    pub canary_success_count: u32,
    pub noted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererRouteMutationResultDto {
    pub schema_version: String,
    pub action: String,
    pub preset_id: String,
    pub published_version: String,
    pub route_stage: String,
    pub approval: BranchRolloutApprovalDto,
    pub audit_entry: PreviewRendererRoutePolicyAuditEntryDto,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererRouteStatusResultDto {
    pub schema_version: String,
    pub preset_id: String,
    pub published_version: String,
    pub route_stage: String,
    pub resolved_route: String,
    pub reason: String,
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
            "초점이 맞지 않았어요.",
            "대상을 다시 맞추는 동안 잠시 기다려 주세요.",
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedicatedRendererRenderProfileDto {
    pub profile_id: String,
    pub display_name: String,
    pub output_color_space: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedicatedRendererWarmupRequestDto {
    pub schema_version: String,
    pub session_id: String,
    pub preset_id: String,
    pub published_version: String,
    pub darktable_version: String,
    pub xmp_template_path: String,
    pub preview_profile: DedicatedRendererRenderProfileDto,
    pub diagnostics_detail_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedicatedRendererWarmupResultDto {
    pub schema_version: String,
    pub session_id: String,
    pub preset_id: String,
    pub published_version: String,
    pub status: String,
    pub diagnostics_detail_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_state_detail_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedicatedRendererPreviewJobRequestDto {
    pub schema_version: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub preset_id: String,
    pub published_version: String,
    pub darktable_version: String,
    pub xmp_template_path: String,
    pub preview_profile: DedicatedRendererRenderProfileDto,
    pub source_asset_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_source_asset_path: Option<String>,
    pub canonical_preview_output_path: String,
    pub diagnostics_detail_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedicatedRendererPreviewJobResultDto {
    pub schema_version: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub status: String,
    pub diagnostics_detail_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_state_detail_path: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::CaptureReadinessDto;

    #[test]
    fn capture_retry_required_uses_focus_guidance_copy() {
        let readiness =
            CaptureReadinessDto::capture_retry_required("session_01hs6n1r8b8zc5v4ey2x7b9g1m", None);

        assert_eq!(readiness.customer_state, "Preparing");
        assert!(!readiness.can_capture);
        assert_eq!(readiness.primary_action, "wait");
        assert_eq!(readiness.customer_message, "초점이 맞지 않았어요.");
        assert_eq!(
            readiness.support_message,
            "대상을 다시 맞추는 동안 잠시 기다려 주세요."
        );
        assert_eq!(readiness.reason_code, "capture-retry-required");
    }
}
