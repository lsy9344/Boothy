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
    /// Story 7.4. 없으면 게시는 오늘과 동일하게 성공하고 `proxyCompatible = false`가 된다.
    #[serde(default)]
    pub proxy_publication: Option<ProxyPublicationPayloadDto>,
}

/// Story 7.4. 게시 시점에 함께 기록하는 display-fit proxy lane 자격과 승인 근거.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyPublicationPayloadDto {
    pub proxy_compatible: bool,
    pub supported_operations: Vec<String>,
    pub proxy_recipe_version: String,
    pub reference_renderer: String,
    pub reference_renderer_version: String,
    pub output_profile: ProxyOutputProfileDto,
    pub visual_approval: ProxyVisualApprovalDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyOutputProfileDto {
    pub color_space: String,
    pub jpeg_quality: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icc_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyVisualApprovalDto {
    pub approved_at: String,
    pub approved_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpus_path: Option<String>,
}

/// 게시 입력의 proxy 블록을 검증한다.
///
/// **부분 승인은 거절한다.** 반쯤 승인된 룩이 고객 화면에 오르는 것보다 게시가 막히는 편이 낫다.
pub fn validate_proxy_publication_payload(
    payload: &ProxyPublicationPayloadDto,
    pinned_renderer: &str,
    pinned_renderer_version: &str,
) -> Result<(), HostErrorEnvelope> {
    let reject = || {
        HostErrorEnvelope::validation_message("화면 적합 미리보기 승인 정보를 다시 확인해 주세요.")
    };

    if !payload.proxy_compatible {
        return Err(reject());
    }

    if payload.supported_operations.is_empty()
        || payload
            .supported_operations
            .iter()
            .any(|operation| !is_non_blank(operation))
    {
        return Err(reject());
    }

    let non_blank = [
        payload.proxy_recipe_version.as_str(),
        payload.output_profile.color_space.as_str(),
        payload.visual_approval.approved_at.as_str(),
        payload.visual_approval.approved_by.as_str(),
    ];

    if non_blank.iter().any(|value| !is_non_blank(value)) {
        return Err(reject());
    }

    if !payload
        .output_profile
        .color_space
        .eq_ignore_ascii_case("srgb")
    {
        return Err(reject());
    }

    if payload
        .output_profile
        .icc_intent
        .as_deref()
        .is_some_and(|intent| {
            !matches!(
                intent.trim().to_ascii_lowercase().as_str(),
                "perceptual"
                    | "relative_colorimetric"
                    | "relative colorimetric"
                    | "saturation"
                    | "absolute_colorimetric"
                    | "absolute colorimetric"
            )
        })
        || payload
            .visual_approval
            .corpus_path
            .as_deref()
            .is_some_and(|path| !is_non_blank(path))
    {
        return Err(reject());
    }

    if !(1..=100).contains(&payload.output_profile.jpeg_quality) {
        return Err(reject());
    }

    // 승인된 화질 증거가 지금 도는 렌더러의 것이 아니면 그 승인은 이 실행에 적용되지 않는다.
    if payload.reference_renderer != pinned_renderer
        || payload.reference_renderer_version != pinned_renderer_version
    {
        return Err(reject());
    }

    // recipe 경로는 호출자가 정하지 않는다. 참조 렌더러가 darktable인 동안 recipe의 실체는
    // 번들이 이미 싣고 있는 XMP template이며, 게시 host가 그 경로를 직접 기록한다.
    // 호출자가 경로를 넣을 수 있게 두면 bundle root 밖을 가리킬 여지가 생긴다.

    Ok(())
}

#[cfg(test)]
mod proxy_publication_validation_tests {
    use super::*;

    fn valid_payload() -> ProxyPublicationPayloadDto {
        ProxyPublicationPayloadDto {
            proxy_compatible: true,
            supported_operations: vec!["exposure".into()],
            proxy_recipe_version: "2026.08.14".into(),
            reference_renderer: "darktable".into(),
            reference_renderer_version: "5.4.1".into(),
            output_profile: ProxyOutputProfileDto {
                color_space: "sRGB".into(),
                jpeg_quality: 95,
                icc_intent: Some("perceptual".into()),
            },
            visual_approval: ProxyVisualApprovalDto {
                approved_at: "2026-08-14T00:00:00+09:00".into(),
                approved_by: "Noah Lee".into(),
                corpus_path: None,
            },
        }
    }

    #[test]
    fn proxy_publication_rejects_profiles_the_renderer_cannot_execute() {
        let mut unsupported_color = valid_payload();
        unsupported_color.output_profile.color_space = "Display P3".into();
        assert!(
            validate_proxy_publication_payload(&unsupported_color, "darktable", "5.4.1").is_err()
        );

        let mut unsupported_intent = valid_payload();
        unsupported_intent.output_profile.icc_intent = Some("guess".into());
        assert!(
            validate_proxy_publication_payload(&unsupported_intent, "darktable", "5.4.1").is_err()
        );

        let mut blank_corpus = valid_payload();
        blank_corpus.visual_approval.corpus_path = Some("   ".into());
        assert!(validate_proxy_publication_payload(&blank_corpus, "darktable", "5.4.1").is_err());
    }
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

/// Story 7.4가 `v1` → `v2`로, Story 7.5가 `v2` → `v3`로, Story 7.6이 `v3` → `v4`로 올렸다.
///
/// v2는 generation이 tier별로 다른 모양을 갖게 됐기 때문이고, v3는 **프레임을 실제로 만든
/// renderer와 참조 renderer를 분리해서 기록**해야 하기 때문이다. 상주 renderer가 만든 프레임에
/// `referenceRenderer=darktable`만 남기면 evidence가 거짓말을 한다.
///
/// v4는 `rawRefinedDisplay` tier가 생기면서 **generation만 보고 두 tier를 구분할 수 있어야**
/// 하기 때문이다 (`renderQuality`). 두 tier는 같은 RAW·같은 XMP·같은 renderer·같은 목표 크기를
/// 쓰고 `--hq`만 다르므로, 이 필드가 없으면 evidence에서 어느 쪽이 화면에 있었는지 알 수 없다.
///
/// **읽기는 v1·v2·v3도 받는다** — 새 필드가 전부 `#[serde(default)]`이라 옛 JSON이 그대로
/// 역직렬화된다. Story 7.9의 pre-upgrade session 호환이 이 규칙 위에 선다.
pub const VIEWER_DISPLAY_SCHEMA_VERSION: &str = "viewer-display/v4";
pub const VIEWER_DISPLAY_SCHEMA_VERSION_V1: &str = "viewer-display/v1";
pub const VIEWER_DISPLAY_SCHEMA_VERSION_V2: &str = "viewer-display/v2";
pub const VIEWER_DISPLAY_SCHEMA_VERSION_V3: &str = "viewer-display/v3";
/// 이벤트 봉투 자체의 모양은 바뀌지 않았다. 안의 pointer가 자기 버전을 들고 다닌다.
pub const VIEWER_DISPLAY_UPDATE_SCHEMA_VERSION: &str = "viewer-display-update/v1";

/// Story 7.2가 등록한 계측용 tier.
pub const DISPLAY_TIER_SAMPLE: &str = "sample";
/// Story 7.4가 등록한 첫 고객 성공 화면 tier.
pub const DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY: &str = "displayFitPresetProxy";
/// Story 7.6이 등록한 RAW 정밀본 승급 tier.
///
/// **`final`은 여기 없고 앞으로도 추가하지 않는다.** 5184×3456 전체 해상도 산출물을
/// 1429×953 사진 영역에 올리면 축소를 브라우저가 하게 되어, darktable 축소보다 품질이 낮으면서
/// decode 비용은 훨씬 크다. `--upscale false` 기반 display-fit 계약과도 충돌한다.
pub const DISPLAY_TIER_RAW_REFINED_DISPLAY: &str = "rawRefinedDisplay";

/// proxy generation의 확정 파일 경로에 들어가는 variant 자리.
/// 빈 문자열을 넘기면 `000001-.jpg`가 만들어져 사람도 스크립트도 읽지 못한다.
pub const DISPLAY_PROXY_PATH_VARIANT: &str = "proxy";
/// Story 7.6. RAW 정밀본 generation의 확정 파일 경로 variant 자리 (`<seq>-refined.jpg`).
pub const DISPLAY_RAW_REFINED_PATH_VARIANT: &str = "refined";

/// Story 7.6. `--hq false`로 만든 프레임 (proxy lane).
pub const DISPLAY_RENDER_QUALITY_FAST: &str = "fast";
/// Story 7.6. `--hq true`로 만든 프레임 (정밀본 lane).
pub const DISPLAY_RENDER_QUALITY_HIGH: &str = "high";

/// tier가 요구하는 렌더 품질. `None`이면 그 tier는 렌더 품질을 갖지 않는다 (`sample`).
pub fn display_tier_required_render_quality(tier: &str) -> Option<&'static str> {
    match tier {
        DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY => Some(DISPLAY_RENDER_QUALITY_FAST),
        DISPLAY_TIER_RAW_REFINED_DISPLAY => Some(DISPLAY_RENDER_QUALITY_HIGH),
        _ => None,
    }
}

/// tier 순서. 값이 클수록 높은 tier이며 **같은 촬영 안에서의** downgrade는 거부된다.
pub fn display_tier_order(tier: &str) -> Option<u8> {
    match tier {
        DISPLAY_TIER_SAMPLE => Some(0),
        DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY => Some(1),
        DISPLAY_TIER_RAW_REFINED_DISPLAY => Some(2),
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
/// Story 7.4. capture-bound preset identity/version이 현재 활성 generation과 다르다.
/// `older-request`와 뭉뚱그리면 순서 문제와 정체성 문제를 회차 분석에서 구분할 수 없다.
pub const DISPLAY_REJECT_PRESET_MISMATCH: &str = "preset-mismatch";
/// Story 7.4. 같은 request 안에서 더 오래된 capture의 generation이 늦게 도착했다.
pub const DISPLAY_REJECT_OLDER_CAPTURE: &str = "older-capture";
/// Story 7.6. RAW 정밀본의 실측 픽셀 크기가 현재 활성 proxy와 정확히 같지 않다.
///
/// **AC 4의 crop/scale 점프 0을 만드는 기계적 장치다.** 붙일 활성 proxy 자체가 없는 경우도
/// 같은 사유로 거부한다 — 맞출 geometry가 없으면 크기 동일성은 성립할 수 없고,
/// 정밀본은 승급이지 첫 성공 화면의 대체가 아니다 (UX-DR19).
pub const DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH: &str = "refined-dimension-mismatch";
/// Story 7.6. AC 6의 detail 축(MTF50) gate가 `justified`를 기록하지 않았다.
///
/// 측정되지 않은 tier 차이는 게시하지 않는다. **host 전용 판정이다** — viewer는
/// 이 gate를 통과한 generation만 보므로 스스로 이 사유를 주장할 수 없다.
pub const DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED: &str = "refined-tier-not-justified";
/// commit·notify까지 끝났지만 유예 시간 안에 viewer의 terminal 보고가 도착하지 않은 generation.
///
/// **host만 이 값을 쓴다.** viewer가 보고할 수 있는 결론이 아니라, 보고 자체가 오지 않았다는
/// host의 관측이다. 이 코드가 없으면 그런 generation은 계측에 **아무 행도 남기지 않아**,
/// "표시되지 않았다"와 "보고가 유실됐다"를 evidence에서 구분할 수 없다.
pub const DISPLAY_REJECT_PRESENT_UNREPORTED: &str = "present-unreported";

/// Story 7.4. display-fit preset proxy generation의 출처.
///
/// AC 1이 요구하는 결속을 하나도 빠짐없이 자산에 실어 둔다. 이 값들이 없으면
/// "화면에 올라간 사진이 정말 이 촬영·이 프리셋·이 화면 크기의 결과인가"를 나중에 증명할 수 없다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayProxyProvenanceDto {
    pub preset_id: String,
    pub preset_version: String,
    /// 이 proxy가 proxy lane에 들어간 **근거**.
    /// `exact-reference-renderer`(근사 아님) 또는 `visual-approval`(승인된 근사).
    /// evidence에서 두 종류의 프레임이 반드시 구분되어야 한다.
    pub approval_basis: String,
    pub proxy_recipe_version: String,
    pub reference_renderer: String,
    pub reference_renderer_version: String,
    pub render_profile_id: String,
    pub output_color_space: String,
    pub jpeg_quality: u32,
    pub source_route: String,
    /// **입력** source의 내용 해시. generation의 `source_hash`는 게시 결과 파일의 해시라 다른 값이다.
    pub source_asset_hash: String,
    pub target_width_px: u32,
    pub target_height_px: u32,
    pub display_profile_id: String,
    pub device_pixel_ratio: f64,
    /// Story 7.5. **이 프레임을 실제로 만든 renderer.** `None`이면 참조 renderer가 직접 만들었다.
    ///
    /// 상주 renderer 결과에 이 블록이 없으면 evidence는 darktable이 만든 프레임과
    /// 다른 엔진이 만든 프레임을 구분할 수 없다.
    #[serde(default)]
    pub resident_provenance: Option<ResidentProducerProvenanceDto>,
    /// Story 7.6. 이 프레임을 만든 darktable pixelpipe 품질 (`fast` / `high`).
    ///
    /// proxy와 정밀본은 같은 RAW·같은 XMP·같은 renderer·같은 목표 크기를 쓰고 `--hq`만 다르다.
    /// 이 필드가 없으면 **generation만 보고 두 tier를 구분할 수 없다.**
    ///
    /// v1~v3 JSON에는 이 필드가 없다. 그 시절 게시 경로는 proxy lane 하나였고 항상
    /// `--hq false`였으므로 `fast`로 채우는 것은 추측이 아니라 그 빌드가 실제로 한 일이다.
    #[serde(default = "default_display_render_quality")]
    pub render_quality: String,
}

fn default_display_render_quality() -> String {
    DISPLAY_RENDER_QUALITY_FAST.to_string()
}

/// Story 7.5. 상주 renderer 실행 mode. **알 수 없는 값은 전부 `off`다.**
pub const RESIDENT_MODE_OFF: &str = "off";
/// 렌더하고 계측하지만 **고객 화면 pointer는 건드리지 않는다.**
pub const RESIDENT_MODE_SHADOW: &str = "shadow";
/// HV-16 evidence 회차에서만 명시적으로 켠다. 이때만 게시 경계를 통과한다.
pub const RESIDENT_MODE_EVIDENCE: &str = "evidence";

/// 오프라인에서 미리 디코드해 둔 raster. **engine feasibility 자료 전용이다.**
pub const RESIDENT_INPUT_PREDECODED_FIXTURE: &str = "predecoded-fixture";
/// 상주 프로세스가 실제 촬영 원본을 **직접** 읽어서 만든 raster.
pub const RESIDENT_INPUT_REAL_CAPTURE_DIRECT: &str = "real-capture-direct";
/// 실제 촬영 원본이지만 별도 one-shot process가 raster를 먼저 만들었다.
/// 그 process의 startup 비용은 hot path에 그대로 남아 있다.
pub const RESIDENT_INPUT_REAL_CAPTURE_VIA_ONE_SHOT: &str = "real-capture-via-one-shot";

/// **현재 승인된 direct real-capture decoder는 하나도 없다.**
///
/// Story 7.3(HV-14)이 embedded JPEG / paired JPEG / Windows shell thumbnail을 전부
/// `Technology No-Go`로 닫았고, 그 뒤 승인된 대체 route가 없다. 이 목록이 비어 있는 한
/// 어떤 상주 후보도 `productionEligible`을 주장할 수 없다 (Story 7.5 AC 6).
///
/// 값을 추가하려면 dependency·installer·라이선스·성능 범위를 적은 **별도 승인**이 필요하다.
pub const RESIDENT_APPROVED_DIRECT_DECODERS: &[&str] = &[];

/// Story 7.5. 상주 renderer가 만든 generation의 producer 신원.
///
/// 참조 renderer(`DisplayProxyProvenanceDto::reference_renderer`)와 **다른 질문에 답한다.**
/// 참조 renderer는 "무엇과 비교해서 정확한가"이고, 이 블록은 "누가 이 픽셀을 만들었는가"다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidentProducerProvenanceDto {
    pub producer_renderer: String,
    pub producer_renderer_version: String,
    pub producer_build_id: String,
    /// `shadow` 또는 `evidence`. `off`는 렌더 자체를 하지 않으므로 여기 나타날 수 없다.
    pub execution_mode: String,
    pub recipe_schema_version: String,
    pub compiled_recipe_hash: String,
    pub program_hash: String,
    /// 상주 context가 만들어진 시각. 촬영보다 **앞서야** 한다.
    pub context_initialized_at_micros: u64,
    /// 측정 대상 hot path에서 일어난 shader/program compile 횟수. **0이 아니면 AC 1 실패다.**
    pub hot_path_program_compile_count: u32,
    /// 측정 대상 hot path에서 일어난 renderer process 시작 횟수. **0이 아니면 AC 1 실패다.**
    pub hot_path_process_start_count: u32,
    pub input_provenance: String,
    pub input_producer: String,
    pub input_producer_version: String,
    /// 입력 raster가 상주 renderer에게 **읽을 수 있는 상태로 도착한** 시각.
    ///
    /// 지연 구간의 시작점이다. 이 값이 없으면 "렌더러가 빠르다"와 "촬영부터 화면까지 빠르다"를
    /// 분리해서 볼 수 없다.
    pub source_ready_at_micros: u64,
    /// 입력 raster를 만든 one-shot process의 startup 비용. 숨기지 않고 여기 남긴다.
    pub input_startup_cost_micros: u64,
    /// **production 채택 자격.** Story 7.5 spike에서는 승인된 direct decoder가 없으므로 항상 false다.
    pub production_eligible: bool,
    pub adoption_block_reason: Option<String>,
    pub gpu_vendor: String,
    pub gpu_renderer: String,
    pub fallback_reason: Option<String>,
}

/// Story 7.5. 상주 계획 전달 계약. viewer가 촬영 **전에** 한 번 받아 간다.
pub const RESIDENT_RENDERER_PLAN_SCHEMA_VERSION: &str = "resident-renderer-plan/v1";

/// recipe 안의 한 연산. **params는 원문 그대로 넘긴다** — 해석은 엔진 한 곳에서만 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidentRecipeOperationDto {
    pub order: u32,
    pub name: String,
    pub enabled: bool,
    pub mod_version: u32,
    pub params_hex: String,
    pub blendop_params: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidentRecipeDto {
    pub schema_version: String,
    pub preset_id: String,
    pub preset_version: String,
    pub engine: String,
    pub engine_version: String,
    pub recipe_version: String,
    pub operations: Vec<ResidentRecipeOperationDto>,
    pub output_color_space: String,
    pub icc_intent: String,
    pub jpeg_quality: u32,
    pub recipe_hash: String,
}

/// preset 하나가 상주 경로에 들어가지 못한 이유. **조용한 누락을 만들지 않는다.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidentRecipeRefusalDto {
    pub preset_id: String,
    pub preset_version: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidentRendererPlanDto {
    pub schema_version: String,
    /// `off` / `shadow` / `evidence`. `off`이면 `recipes`는 비어 있다.
    pub mode: String,
    pub engine: String,
    pub engine_version: String,
    pub recipes: Vec<ResidentRecipeDto>,
    pub refusals: Vec<ResidentRecipeRefusalDto>,
}

/// `productionEligible`이 거짓 주장을 하지 못하게 막는 기계적 gate.
///
/// fixture로 얻은 빠른 결과에 `productionEligible: true`를 붙이는 것이 이 Story에서
/// 가장 쉬운 자기기만이다. 그래서 게시 경계 **앞에서** 거부한다.
pub fn validate_resident_producer_provenance(
    provenance: &ResidentProducerProvenanceDto,
) -> Result<(), HostErrorEnvelope> {
    let invalid = |detail: &str| {
        log::warn!("resident_provenance_rejected detail={detail}");
        Err(HostErrorEnvelope::validation_message(
            "표시 자산 게시 값을 다시 확인해 주세요.",
        ))
    };

    if !is_non_blank(&provenance.producer_renderer)
        || !is_non_blank(&provenance.producer_renderer_version)
        || !is_non_blank(&provenance.producer_build_id)
        || !is_non_blank(&provenance.recipe_schema_version)
        || !is_non_blank(&provenance.compiled_recipe_hash)
        || !is_non_blank(&provenance.program_hash)
        || !is_non_blank(&provenance.input_producer)
        || !is_non_blank(&provenance.input_producer_version)
        || !is_non_blank(&provenance.gpu_vendor)
        || !is_non_blank(&provenance.gpu_renderer)
    {
        return invalid("producer identity fields must not be blank");
    }

    if provenance.context_initialized_at_micros == 0
        || provenance.source_ready_at_micros == 0
        || provenance.context_initialized_at_micros > provenance.source_ready_at_micros
    {
        return invalid("resident context must be initialized before the source becomes ready");
    }

    if !matches!(
        provenance.execution_mode.as_str(),
        RESIDENT_MODE_SHADOW | RESIDENT_MODE_EVIDENCE
    ) {
        return invalid("execution mode must be shadow or evidence");
    }

    let known_input = matches!(
        provenance.input_provenance.as_str(),
        RESIDENT_INPUT_PREDECODED_FIXTURE
            | RESIDENT_INPUT_REAL_CAPTURE_DIRECT
            | RESIDENT_INPUT_REAL_CAPTURE_VIA_ONE_SHOT
    );

    if !known_input {
        return invalid("input provenance must be one of the three declared values");
    }

    if provenance.hot_path_program_compile_count > 0 || provenance.hot_path_process_start_count > 0
    {
        return invalid("a resident evidence frame requires an empty hot path");
    }

    if !provenance.production_eligible {
        // 채택 불가라고 스스로 말하는 프레임은 그 이유를 반드시 들고 있어야 한다.
        return match provenance.adoption_block_reason.as_deref() {
            Some(reason) if !reason.trim().is_empty() => Ok(()),
            _ => invalid("a non-eligible resident frame must carry its block reason"),
        };
    }

    // 여기부터는 `productionEligible: true`를 주장하는 경우다. 조건이 전부 맞아야 한다.
    if provenance.input_provenance != RESIDENT_INPUT_REAL_CAPTURE_DIRECT {
        return invalid("production eligibility requires a direct real-capture input");
    }

    if !RESIDENT_APPROVED_DIRECT_DECODERS.contains(&provenance.input_producer.as_str()) {
        return invalid("input decoder is not on the approved direct decoder list");
    }

    if provenance.adoption_block_reason.is_some() {
        return invalid("an eligible resident frame cannot also carry a block reason");
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayGenerationDto {
    pub generation_id: String,
    pub generation_seq: u64,
    /// host가 request를 처음 관측한 순서. 이전 pointer에는 없으므로 optional이다.
    #[serde(default)]
    pub request_order: Option<u64>,
    /// 촬영 확정 시점에 고정한 순서. 렌더 완료 순서 대신 이 값으로 stale capture를 막는다.
    #[serde(default)]
    pub capture_order: Option<u64>,
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
    /// 계측 fixture에서만 의미가 있다. proxy generation에서는 `None`이다.
    /// v1 JSON에는 항상 값이 있으므로 `default`는 v2 proxy 행에만 쓰인다.
    #[serde(default)]
    pub sample_variant: Option<String>,
    /// proxy generation에서만 존재한다. v1 JSON에는 이 필드가 없다.
    #[serde(default)]
    pub proxy_provenance: Option<DisplayProxyProvenanceDto>,
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
    /// **표시 이미지가 계측용 fixture인지.** Story 7.2의 sample lane 상태 그대로다.
    /// Story 7.4의 proxy lane은 실제 제품 결과이므로 이 값을 켜지 않는다.
    pub measurement_lane_enabled: bool,
    /// **두 surface가 present 계측 IPC를 해야 하는지.** `sample lane || proxy lane`이다.
    /// 이 값을 `measurement_lane_enabled`와 합치면 proxy lane 회차의 actual-present가 0건이 된다.
    #[serde(default)]
    pub present_telemetry_enabled: bool,
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
                    | DISPLAY_REJECT_PRESET_MISMATCH
                    | DISPLAY_REJECT_OLDER_CAPTURE
                    | DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH
                    | DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED
            )
        });
    let coherent_outcome = match report.outcome.as_str() {
        "presented" => {
            report.reject_reason.is_none() && report.spans.actual_present_at_micros.is_some()
        }
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

// ---------------------------------------------------------------------------
// Story 7.7 — release-inventory/v1 / install-self-check/v1
//
// TS(Zod) 정의와 **한 쌍**이다. 세 번째 정의를 만들지 않는다.
// 서술 계약은 `docs/contracts/release-inventory.md`가 소유한다.
// ---------------------------------------------------------------------------

pub const RELEASE_INVENTORY_SCHEMA_VERSION: &str = "release-inventory/v1";
pub const INSTALL_SELF_CHECK_SCHEMA_VERSION: &str = "install-self-check/v1";

/// AC 1이 열거한 구성요소 종류. **인벤토리는 이 아홉 개를 전부 담아야 한다.**
pub const RELEASE_INVENTORY_ROLES: [&str; 9] = [
    "app",
    "camera-helper",
    "edsdk-runtime",
    "source-adapter",
    "display-renderer",
    "color-profile",
    "proxy-recipes",
    "raw-renderer",
    "webview2-runtime",
];

/// 인벤토리 대조 실패 사유. **한 덩어리로 뭉치지 않는다** — 운영자 조치가 사유마다 다르다.
pub const RELEASE_INVENTORY_REASON_CODES: [&str; 12] = [
    "inventory-not-staged",
    "inventory-component-missing",
    "inventory-digest-mismatch",
    "inventory-unexpected-component",
    "digest-tool-unavailable",
    "darktable-not-bundled",
    "darktable-tree-incomplete",
    "helper-binary-missing",
    "helper-version-check-failed",
    "webview2-runtime-unreadable",
    "inventory-manifest-unreadable",
    "session-manifest-unreadable",
];

/// `bundled-resource`가 아니면 **그 회차는 핀이 깨진 회차다.**
pub const DARKTABLE_RESOLUTION_SOURCES: [&str; 6] = [
    "env-override",
    "bundled-resource",
    "program-files-bin",
    "program-w6432-bin",
    "localappdata-programs-bin",
    "path",
];

const RELEASE_COMPONENT_STATUSES: [&str; 4] = ["present", "embedded", "not-applicable", "missing"];
const RELEASE_SIGNING_STATUSES: [&str; 3] = ["signed", "unsigned", "not-applicable"];
const SELF_CHECK_COMPONENT_STATUSES: [&str; 3] = ["pass", "fail", "skipped"];

pub fn is_valid_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|char| char.is_ascii_digit() || ('a'..='f').contains(&char))
}

/// 어떤 파일이 이 구성요소의 것인가.
///
/// **설치된 앱이 인벤토리만 보고 해시를 다시 계산할 수 있어야 한다.** 명세(`inventory-spec.json`)는
/// 설치본에 들어가지 않으므로, 파일 선택 규칙이 인벤토리 안에 있어야 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ComponentEntrySelectionDto {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "only")]
    Only { entries: Vec<String> },
    #[serde(rename = "allExcept")]
    AllExcept { entries: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInventoryComponentDto {
    pub name: String,
    pub role: String,
    pub status: String,
    pub version: Option<String>,
    pub origin: String,
    pub entry_selection: Option<ComponentEntrySelectionDto>,
    /// AC 1의 licensing 증거는 별도 문서가 아니라 **인벤토리 자체의 일부**다.
    pub license: String,
    pub license_evidence_path: Option<String>,
    pub source_archive_sha256: Option<String>,
    /// 정렬된 `<상대경로>:<sha256>` 목록을 다시 sha256 한 값. 파일 하나만 바뀌어도 달라진다.
    pub staged_tree_digest: Option<String>,
    pub install_relative_path: Option<String>,
    pub signing_status: String,
    pub rationale: Option<String>,
    pub file_count: Option<u64>,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInstallerIdentityDto {
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedReleaseInventoryDto {
    pub schema_version: String,
    pub generated_at: String,
    pub app_version: String,
    pub identifier: String,
    pub installer_file_name: String,
    pub signing_status: String,
    /// **봉인 단계에서만 채워진다.** 설치본 안의 사본은 자기 자신의 해시를 담을 수 없다.
    pub installer: Option<ReleaseInstallerIdentityDto>,
    pub components: Vec<ReleaseInventoryComponentDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotStagedReleaseInventoryDto {
    pub schema_version: String,
    pub reason: String,
}

/// **결손을 스스로 신고하는 문서를 계약 안에 둔다.** 자리를 비워 두지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "staging")]
pub enum ReleaseInventoryDto {
    #[serde(rename = "staged")]
    Staged(StagedReleaseInventoryDto),
    #[serde(rename = "not-staged")]
    NotStaged(NotStagedReleaseInventoryDto),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSelfCheckComponentDto {
    pub name: String,
    pub expected_digest: Option<String>,
    pub actual_digest: Option<String>,
    pub status: String,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DarktableResolutionDto {
    pub binary: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSelfCheckReportDto {
    pub schema_version: String,
    pub checked_at: String,
    pub app_version: String,
    pub identifier: String,
    pub install_root: String,
    pub components: Vec<InstallSelfCheckComponentDto>,
    pub webview2_version: Option<String>,
    pub darktable_resolution: Option<DarktableResolutionDto>,
    pub helper_version: Option<String>,
    pub overall: String,
}

fn require_release_text(value: &str, message: &str) -> Result<(), HostErrorEnvelope> {
    if is_non_blank(value) {
        return Ok(());
    }

    Err(HostErrorEnvelope::validation_message(message.to_string()))
}

fn require_optional_digest(value: Option<&String>, message: &str) -> Result<(), HostErrorEnvelope> {
    match value {
        Some(digest) if !is_valid_sha256_digest(digest) => {
            Err(HostErrorEnvelope::validation_message(message.to_string()))
        }
        _ => Ok(()),
    }
}

pub fn validate_release_inventory_component(
    component: &ReleaseInventoryComponentDto,
) -> Result<(), HostErrorEnvelope> {
    require_release_text(
        &component.name,
        "인벤토리 구성요소 이름을 다시 확인해 주세요.",
    )?;
    require_release_text(
        &component.origin,
        "인벤토리 구성요소 출처를 다시 확인해 주세요.",
    )?;
    require_release_text(
        &component.license,
        "인벤토리 구성요소 라이선스를 다시 확인해 주세요.",
    )?;

    if !RELEASE_INVENTORY_ROLES.contains(&component.role.as_str()) {
        return Err(HostErrorEnvelope::validation_message(
            "인벤토리 구성요소 종류를 다시 확인해 주세요.",
        ));
    }

    if !RELEASE_COMPONENT_STATUSES.contains(&component.status.as_str()) {
        return Err(HostErrorEnvelope::validation_message(
            "인벤토리 구성요소 상태를 다시 확인해 주세요.",
        ));
    }

    if !RELEASE_SIGNING_STATUSES.contains(&component.signing_status.as_str()) {
        return Err(HostErrorEnvelope::validation_message(
            "인벤토리 구성요소 서명 상태를 다시 확인해 주세요.",
        ));
    }

    require_optional_digest(
        component.staged_tree_digest.as_ref(),
        "인벤토리 트리 해시 형식을 다시 확인해 주세요.",
    )?;
    require_optional_digest(
        component.source_archive_sha256.as_ref(),
        "인벤토리 원본 아카이브 해시 형식을 다시 확인해 주세요.",
    )?;

    if component.status == "present" {
        if component.staged_tree_digest.is_none() {
            return Err(HostErrorEnvelope::validation_message(
                "동봉된 구성요소에는 트리 해시가 있어야 해요.",
            ));
        }

        if component.install_relative_path.is_none() {
            return Err(HostErrorEnvelope::validation_message(
                "동봉된 구성요소에는 설치 경로가 있어야 해요.",
            ));
        }

        if component.license_evidence_path.is_none() {
            return Err(HostErrorEnvelope::validation_message(
                "동봉된 구성요소에는 라이선스 증거 경로가 있어야 해요.",
            ));
        }

        if component.version.is_none() {
            return Err(HostErrorEnvelope::validation_message(
                "동봉된 구성요소에는 버전이 있어야 해요.",
            ));
        }

        if component.file_count.is_none() || component.total_bytes.is_none() {
            return Err(HostErrorEnvelope::validation_message(
                "동봉된 구성요소에는 파일 수와 크기가 있어야 해요.",
            ));
        }

        if component.entry_selection.is_none() {
            return Err(HostErrorEnvelope::validation_message(
                "설치된 앱이 해시를 다시 계산하려면 파일 선택 규칙이 있어야 해요.",
            ));
        }

        return Ok(());
    }

    if component.entry_selection.is_some() {
        return Err(HostErrorEnvelope::validation_message(
            "동봉하지 않은 구성요소에 파일 선택 규칙을 적을 수 없어요.",
        ));
    }

    if component.rationale.as_deref().map(is_non_blank) != Some(true) {
        return Err(HostErrorEnvelope::validation_message(
            "동봉하지 않은 구성요소에는 그 이유가 있어야 해요.",
        ));
    }

    if component.staged_tree_digest.is_some() {
        return Err(HostErrorEnvelope::validation_message(
            "동봉하지 않은 구성요소에 트리 해시를 적을 수 없어요.",
        ));
    }

    if component.status == "embedded" && component.install_relative_path.is_none() {
        return Err(HostErrorEnvelope::validation_message(
            "앱 바이너리에 내장된 구성요소도 어느 실행 파일에 있는지 적어야 해요.",
        ));
    }

    Ok(())
}

pub fn validate_release_inventory(
    inventory: &ReleaseInventoryDto,
) -> Result<(), HostErrorEnvelope> {
    match inventory {
        ReleaseInventoryDto::NotStaged(not_staged) => {
            if not_staged.schema_version != RELEASE_INVENTORY_SCHEMA_VERSION {
                return Err(HostErrorEnvelope::validation_message(
                    "인벤토리 스키마 버전을 다시 확인해 주세요.",
                ));
            }

            require_release_text(&not_staged.reason, "staged 되지 않은 이유를 적어야 해요.")
        }
        ReleaseInventoryDto::Staged(staged) => {
            if staged.schema_version != RELEASE_INVENTORY_SCHEMA_VERSION {
                return Err(HostErrorEnvelope::validation_message(
                    "인벤토리 스키마 버전을 다시 확인해 주세요.",
                ));
            }

            require_release_text(
                &staged.generated_at,
                "인벤토리 생성 시각을 다시 확인해 주세요.",
            )?;
            require_release_text(
                &staged.app_version,
                "인벤토리 앱 버전을 다시 확인해 주세요.",
            )?;
            require_release_text(
                &staged.identifier,
                "인벤토리 identifier를 다시 확인해 주세요.",
            )?;
            require_release_text(
                &staged.installer_file_name,
                "인벤토리 설치본 파일명을 다시 확인해 주세요.",
            )?;

            if !RELEASE_SIGNING_STATUSES.contains(&staged.signing_status.as_str()) {
                return Err(HostErrorEnvelope::validation_message(
                    "인벤토리 서명 상태를 다시 확인해 주세요.",
                ));
            }

            if let Some(installer) = staged.installer.as_ref() {
                require_release_text(&installer.file_name, "설치본 파일명을 다시 확인해 주세요.")?;

                if !is_valid_sha256_digest(&installer.sha256) || installer.size_bytes == 0 {
                    return Err(HostErrorEnvelope::validation_message(
                        "설치본 해시와 크기를 다시 확인해 주세요.",
                    ));
                }
            }

            for component in &staged.components {
                validate_release_inventory_component(component)?;
            }

            for role in RELEASE_INVENTORY_ROLES {
                let occurrences = staged
                    .components
                    .iter()
                    .filter(|component| component.role == role)
                    .count();

                if occurrences != 1 {
                    return Err(HostErrorEnvelope::validation_message(format!(
                        "인벤토리에 `{role}` 항목이 정확히 하나 있어야 해요."
                    )));
                }
            }

            Ok(())
        }
    }
}

pub fn validate_install_self_check_report(
    report: &InstallSelfCheckReportDto,
) -> Result<(), HostErrorEnvelope> {
    if report.schema_version != INSTALL_SELF_CHECK_SCHEMA_VERSION {
        return Err(HostErrorEnvelope::validation_message(
            "self-check 보고서 스키마 버전을 다시 확인해 주세요.",
        ));
    }

    require_release_text(&report.checked_at, "self-check 시각을 다시 확인해 주세요.")?;
    require_release_text(
        &report.app_version,
        "self-check 앱 버전을 다시 확인해 주세요.",
    )?;
    require_release_text(
        &report.identifier,
        "self-check identifier를 다시 확인해 주세요.",
    )?;
    require_release_text(
        &report.install_root,
        "self-check 설치 경로를 다시 확인해 주세요.",
    )?;

    if report.components.is_empty() {
        return Err(HostErrorEnvelope::validation_message(
            "self-check 보고서에는 검사 항목이 있어야 해요.",
        ));
    }

    for component in &report.components {
        require_release_text(
            &component.name,
            "self-check 항목 이름을 다시 확인해 주세요.",
        )?;

        if !SELF_CHECK_COMPONENT_STATUSES.contains(&component.status.as_str()) {
            return Err(HostErrorEnvelope::validation_message(
                "self-check 항목 상태를 다시 확인해 주세요.",
            ));
        }

        require_optional_digest(
            component.expected_digest.as_ref(),
            "self-check 기대 해시 형식을 다시 확인해 주세요.",
        )?;
        require_optional_digest(
            component.actual_digest.as_ref(),
            "self-check 실제 해시 형식을 다시 확인해 주세요.",
        )?;

        match component.reason_code.as_deref() {
            Some(reason_code) => {
                if !RELEASE_INVENTORY_REASON_CODES.contains(&reason_code) {
                    return Err(HostErrorEnvelope::validation_message(
                        "self-check 사유 코드를 다시 확인해 주세요.",
                    ));
                }

                if component.status == "pass" {
                    return Err(HostErrorEnvelope::validation_message(
                        "통과 항목에는 사유 코드를 적지 않아요.",
                    ));
                }
            }
            None => {
                if component.status == "fail" {
                    return Err(HostErrorEnvelope::validation_message(
                        "실패 항목에는 사유 코드가 있어야 해요.",
                    ));
                }
            }
        }
    }

    if let Some(resolution) = report.darktable_resolution.as_ref() {
        require_release_text(&resolution.binary, "darktable 경로를 다시 확인해 주세요.")?;

        if !DARKTABLE_RESOLUTION_SOURCES.contains(&resolution.source.as_str()) {
            return Err(HostErrorEnvelope::validation_message(
                "darktable 해석 출처를 다시 확인해 주세요.",
            ));
        }
    }

    let has_failure = report
        .components
        .iter()
        .any(|component| component.status == "fail");

    match report.overall.as_str() {
        "pass" if has_failure => Err(HostErrorEnvelope::validation_message(
            "실패 항목이 있으면 전체 결과를 통과로 적을 수 없어요.",
        )),
        "fail" if !has_failure => Err(HostErrorEnvelope::validation_message(
            "실패 항목이 없는데 전체 결과만 실패로 적을 수 없어요.",
        )),
        "pass" | "fail" => Ok(()),
        _ => Err(HostErrorEnvelope::validation_message(
            "self-check 전체 결과를 다시 확인해 주세요.",
        )),
    }
}
