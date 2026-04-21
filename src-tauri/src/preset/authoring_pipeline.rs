use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use image::ImageReader;

use crate::{
    contracts::dto::{
        validate_draft_preset_edit_input, validate_draft_validation_input,
        validate_publish_validated_preset_input, validate_repair_invalid_draft_input,
        AuthoringWorkspaceResultDto, CapabilitySnapshotDto, DraftNoisePolicyDto,
        DraftPresetEditPayloadDto, DraftPresetPreviewReferenceDto, DraftPresetSummaryDto,
        DraftRenderProfileDto, DraftValidationFindingDto, DraftValidationReportDto,
        DraftValidationSnapshotDto, HostErrorEnvelope, InvalidDraftArtifactDto,
        PresetPublicationAuditRecordDto, PublishValidatedPresetInputDto,
        PublishValidatedPresetResultDto, PublishedPresetSummaryDto, RepairInvalidDraftInputDto,
        ValidateDraftPresetInputDto, ValidateDraftPresetResultDto,
    },
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    preset::{
        preset_catalog::resolve_published_preset_catalog_dir,
        preset_catalog_state::publish_preset_to_live_catalog,
    },
    session::session_manifest::current_timestamp,
};

const AUTHORING_WORKSPACE_SCHEMA_VERSION: &str = "preset-authoring-workspace/v1";
const DRAFT_PRESET_ARTIFACT_SCHEMA_VERSION: &str = "draft-preset-artifact/v1";
const DRAFT_PRESET_VALIDATION_SCHEMA_VERSION: &str = "draft-preset-validation/v1";
const DRAFT_PRESET_VALIDATION_RESULT_SCHEMA_VERSION: &str = "draft-preset-validation-result/v1";
const DRAFT_PRESET_PUBLICATION_RESULT_SCHEMA_VERSION: &str = "draft-preset-publication-result/v1";
const PRESET_PUBLICATION_AUDIT_SCHEMA_VERSION: &str = "preset-publication-audit/v1";
const PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION: &str = "published-preset-bundle/v1";
const AUTHORING_WINDOW_LABEL: &str = "authoring-window";
const PINNED_DARKTABLE_VERSION: &str = "5.4.1";
const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
const VALIDATION_RENDER_PROBE_MAX_WIDTH_PX: u32 = 64;
const VALIDATION_RENDER_PROBE_MAX_HEIGHT_PX: u32 = 64;

pub fn resolve_draft_authoring_root(base_dir: &Path) -> PathBuf {
    base_dir.join("preset-authoring").join("drafts")
}

pub fn load_authoring_workspace_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<AuthoringWorkspaceResultDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;

    let drafts_root = resolve_draft_authoring_root(base_dir);

    if !drafts_root.exists() {
        return Ok(empty_authoring_workspace());
    }

    let draft_dirs = fs::read_dir(&drafts_root).map_err(|error| {
        HostErrorEnvelope::persistence(format!("draft 작업공간을 읽지 못했어요: {error}"))
    })?;
    let mut drafts = Vec::new();
    let mut invalid_drafts = Vec::new();

    for entry in draft_dirs {
        let draft_dir = match entry {
            Ok(entry) => {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };

                if !file_type.is_dir() || file_type.is_symlink() {
                    continue;
                }

                entry.path()
            }
            Err(_) => continue,
        };

        let draft_path = draft_dir.join("draft.json");

        match inspect_draft_artifact(base_dir, &draft_dir, &draft_path) {
            DraftArtifactInspection::Valid(summary) => drafts.push(summary),
            DraftArtifactInspection::Invalid(artifact) => invalid_drafts.push(artifact),
        }
    }

    drafts.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.display_name.cmp(&right.display_name))
            .then_with(|| left.preset_id.cmp(&right.preset_id))
    });

    Ok(AuthoringWorkspaceResultDto {
        schema_version: AUTHORING_WORKSPACE_SCHEMA_VERSION.into(),
        supported_lifecycle_states: supported_lifecycle_states(),
        drafts,
        invalid_drafts,
    })
}

pub fn create_draft_preset_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: DraftPresetEditPayloadDto,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_draft_preset_edit_input(&input)?;

    let drafts_root = resolve_draft_authoring_root(base_dir);
    fs::create_dir_all(&drafts_root).map_err(map_fs_error)?;

    let draft_path = resolve_draft_file_path(&drafts_root, &input.preset_id);
    ensure_draft_file_path_within_root(&drafts_root, &draft_path)?;

    if draft_path.exists() {
        return Err(HostErrorEnvelope::validation_message(
            "같은 presetId의 draft가 이미 있어요.",
        ));
    }

    let summary = build_draft_summary(&input, 1, Vec::new(), Vec::new())?;
    write_draft_summary(&draft_path, &summary)?;

    Ok(summary)
}

pub fn repair_invalid_draft_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: RepairInvalidDraftInputDto,
) -> Result<(), HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_repair_invalid_draft_input(&input)?;

    let drafts_root = resolve_draft_authoring_root(base_dir);

    if !drafts_root.exists() {
        return Err(HostErrorEnvelope::validation_message(
            "정리할 손상 draft를 찾지 못했어요.",
        ));
    }

    let draft_path = resolve_draft_file_path(&drafts_root, &input.draft_folder);
    ensure_draft_file_path_within_root(&drafts_root, &draft_path)?;
    let draft_dir = draft_path
        .parent()
        .ok_or_else(|| HostErrorEnvelope::persistence("손상 draft 위치를 확인하지 못했어요."))?;

    if !draft_dir.exists() {
        return Err(HostErrorEnvelope::validation_message(
            "정리할 손상 draft를 찾지 못했어요.",
        ));
    }

    match inspect_draft_artifact(base_dir, draft_dir, &draft_path) {
        DraftArtifactInspection::Valid(_) => {
            return Err(HostErrorEnvelope::validation_message(
                "정상 draft는 손상 정리 대상이 아니에요.",
            ));
        }
        DraftArtifactInspection::Invalid(artifact) if !artifact.can_repair => {
            return Err(HostErrorEnvelope::validation_message(artifact.guidance));
        }
        DraftArtifactInspection::Invalid(_) => {}
    }

    fs::remove_dir_all(draft_dir).map_err(map_fs_error)?;

    Ok(())
}

pub fn save_draft_preset_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: DraftPresetEditPayloadDto,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_draft_preset_edit_input(&input)?;

    let drafts_root = resolve_draft_authoring_root(base_dir);
    let draft_path = resolve_draft_file_path(&drafts_root, &input.preset_id);
    ensure_draft_file_path_within_root(&drafts_root, &draft_path)?;
    let existing_draft = load_required_draft_summary(
        base_dir,
        &draft_path,
        "먼저 새 draft를 만들어 주세요.",
        "저장된 draft 기록이 손상되어 다시 저장할 수 없어요. 새 draft를 만들어 필요한 내용을 다시 옮겨 주세요.",
    )?;
    ensure_mutable_authoring_lifecycle(&existing_draft.lifecycle_state, "저장")?;
    let summary = build_draft_summary(
        &input,
        existing_draft.draft_version + 1,
        existing_draft.validation.history,
        existing_draft.publication_history,
    )?;

    write_draft_summary(&draft_path, &summary)?;

    Ok(summary)
}

pub fn validate_draft_preset_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: ValidateDraftPresetInputDto,
) -> Result<ValidateDraftPresetResultDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_draft_validation_input(&input)?;

    let drafts_root = resolve_draft_authoring_root(base_dir);
    let draft_path = resolve_draft_file_path(&drafts_root, &input.preset_id);
    ensure_draft_file_path_within_root(&drafts_root, &draft_path)?;
    let existing_draft = load_required_draft_summary(
        base_dir,
        &draft_path,
        "검증할 draft를 찾지 못했어요.",
        "저장된 draft 기록이 손상되어 검증을 이어갈 수 없어요. 새 draft를 만들고 메타데이터와 자산 참조를 다시 저장해 주세요.",
    )?;
    ensure_mutable_authoring_lifecycle(&existing_draft.lifecycle_state, "검증")?;
    let checked_at = current_timestamp(SystemTime::now())?;
    let report = build_validation_report(&draft_path, &existing_draft, checked_at.clone());
    let mut history = existing_draft.validation.history.clone();
    history.push(report.clone());

    let updated_draft = DraftPresetSummaryDto {
        lifecycle_state: if report.status == "passed" {
            "validated".into()
        } else {
            "draft".into()
        },
        validation: DraftValidationSnapshotDto {
            status: report.status.clone(),
            latest_report: Some(report.clone()),
            history,
        },
        publication_history: existing_draft.publication_history,
        updated_at: checked_at,
        ..existing_draft
    };

    write_draft_summary(&draft_path, &updated_draft)?;

    Ok(ValidateDraftPresetResultDto {
        schema_version: DRAFT_PRESET_VALIDATION_RESULT_SCHEMA_VERSION.into(),
        draft: updated_draft,
        report,
    })
}

pub fn publish_validated_preset_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: PublishValidatedPresetInputDto,
) -> Result<PublishValidatedPresetResultDto, HostErrorEnvelope> {
    ensure_authoring_access(capability_snapshot)?;
    validate_publish_validated_preset_input(&input)?;

    let drafts_root = resolve_draft_authoring_root(base_dir);
    let draft_path = resolve_draft_file_path(&drafts_root, &input.preset_id);
    ensure_draft_file_path_within_root(&drafts_root, &draft_path)?;
    let existing_draft = load_required_draft_summary(
        base_dir,
        &draft_path,
        "게시할 draft를 찾지 못했어요.",
        "저장된 draft 기록이 손상되어 게시를 이어갈 수 없어요. 새 draft를 만들고 다시 검증해 주세요.",
    )?;
    let noted_at = current_timestamp(SystemTime::now())?;

    if input.scope != "future-sessions-only" {
        return reject_publication(
            base_dir,
            &draft_path,
            existing_draft,
            &input,
            "future-session-only-violation",
            "게시는 미래 세션 catalog에만 반영할 수 있어요.",
            "현재 진행 중인 세션이나 활성 바인딩을 직접 바꾸지 말고 future-sessions-only 범위로 다시 게시해 주세요.",
            noted_at,
        );
    }

    let Some(latest_report) = existing_draft.validation.latest_report.as_ref() else {
        return reject_publication(
            base_dir,
            &draft_path,
            existing_draft,
            &input,
            "draft-not-validated",
            "검증을 통과한 draft만 게시할 수 있어요.",
            "host validation을 다시 실행해 validated 상태를 만든 뒤 게시해 주세요.",
            noted_at,
        );
    };

    if existing_draft.lifecycle_state != "validated"
        || existing_draft.validation.status != "passed"
        || latest_report.status != "passed"
        || latest_report.lifecycle_state != "validated"
    {
        return reject_publication(
            base_dir,
            &draft_path,
            existing_draft,
            &input,
            "draft-not-validated",
            "검증을 통과한 draft만 게시할 수 있어요.",
            "latest validation이 passed인 validated draft만 승인 후 게시할 수 있어요.",
            noted_at,
        );
    }

    if existing_draft.draft_version != input.draft_version
        || latest_report.draft_version != existing_draft.draft_version
        || latest_report.checked_at != input.validation_checked_at
    {
        return reject_publication(
            base_dir,
            &draft_path,
            existing_draft,
            &input,
            "stale-validation",
            "게시 기준이 된 validation 결과가 최신 draft와 맞지 않아요.",
            "draft를 다시 불러와 최신 저장본에서 validation을 다시 실행한 뒤 게시해 주세요.",
            noted_at,
        );
    }

    if existing_draft.display_name != input.expected_display_name
        || existing_draft.darktable_version != PINNED_DARKTABLE_VERSION
    {
        return reject_publication(
            base_dir,
            &draft_path,
            existing_draft,
            &input,
            "metadata-mismatch",
            "승인 검토에 사용한 metadata와 현재 draft metadata가 일치하지 않아요.",
            "표시 이름과 pinned darktable metadata를 다시 확인한 뒤 최신 상태로 검토해 주세요.",
            noted_at,
        );
    }

    let Some(draft_dir) = draft_path.parent() else {
        return Err(HostErrorEnvelope::persistence(
            "draft 게시 경로를 준비하지 못했어요.",
        ));
    };

    let preview_source = match resolve_workspace_file_for_publication(
        draft_dir,
        &existing_draft.preview.asset_path,
        "preview.assetPath",
    ) {
        Ok(path) => path,
        Err((reason_code, message, guidance)) => {
            return reject_publication(
                base_dir,
                &draft_path,
                existing_draft,
                &input,
                reason_code,
                message,
                guidance,
                noted_at,
            )
        }
    };
    let sample_cut_source = match resolve_workspace_file_for_publication(
        draft_dir,
        &existing_draft.sample_cut.asset_path,
        "sampleCut.assetPath",
    ) {
        Ok(path) => path,
        Err((reason_code, message, guidance)) => {
            return reject_publication(
                base_dir,
                &draft_path,
                existing_draft,
                &input,
                reason_code,
                message,
                guidance,
                noted_at,
            )
        }
    };
    let darktable_source = match resolve_workspace_file_for_publication(
        draft_dir,
        &existing_draft.darktable_project_path,
        "darktableProjectPath",
    ) {
        Ok(path) => path,
        Err((reason_code, message, guidance)) => {
            return reject_publication(
                base_dir,
                &draft_path,
                existing_draft,
                &input,
                reason_code,
                message,
                guidance,
                noted_at,
            )
        }
    };
    let xmp_source = match resolve_workspace_file_for_publication(
        draft_dir,
        &existing_draft.xmp_template_path,
        "xmpTemplatePath",
    ) {
        Ok(path) => path,
        Err((reason_code, message, guidance)) => {
            return reject_publication(
                base_dir,
                &draft_path,
                existing_draft,
                &input,
                reason_code,
                message,
                guidance,
                noted_at,
            )
        }
    };

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let final_bundle_dir = catalog_root
        .join(&existing_draft.preset_id)
        .join(&input.published_version);

    if final_bundle_dir.exists() {
        return reject_publication(
            base_dir,
            &draft_path,
            existing_draft,
            &input,
            "duplicate-version",
            "같은 published version이 이미 존재해서 immutable 게시 규칙을 지킬 수 없어요.",
            "새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.",
            noted_at,
        );
    }

    let temp_bundle_dir = resolve_temp_bundle_dir(&final_bundle_dir);
    let created_bundle = create_published_bundle_from_draft(
        &temp_bundle_dir,
        &existing_draft,
        &input,
        &noted_at,
        &preview_source,
        &sample_cut_source,
        &darktable_source,
        &xmp_source,
    );

    if let Err(error) = created_bundle {
        let _ = fs::remove_dir_all(&temp_bundle_dir);
        return Err(error);
    }

    fs::create_dir_all(final_bundle_dir.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("게시 디렉터리 부모 경로를 준비하지 못했어요.")
    })?)
    .map_err(map_fs_error)?;

    if let Err(error) = fs::rename(&temp_bundle_dir, &final_bundle_dir) {
        let _ = fs::remove_dir_all(&temp_bundle_dir);
        return Err(map_fs_error(error));
    }

    let preview_asset_path =
        build_absolute_asset_path(&final_bundle_dir, "preview", &preview_source)?;
    let previous_publication_history =
        load_publication_history(base_dir, &existing_draft.preset_id);
    let approved_record = build_publication_audit_record(
        &existing_draft,
        &input,
        "approved",
        input.review_note.as_deref(),
        None,
        "승인 검토가 완료되었고 immutable 게시 아티팩트를 확정하고 있어요.",
        &noted_at,
    );
    let published_record = build_publication_audit_record(
        &existing_draft,
        &input,
        "published",
        None,
        None,
        "게시가 완료되었고 이 버전은 미래 세션 catalog에서만 선택할 수 있어요.",
        &noted_at,
    );
    let mut publication_history = previous_publication_history.clone();
    publication_history.push(approved_record);
    publication_history.push(published_record.clone());
    let updated_draft = DraftPresetSummaryDto {
        lifecycle_state: "published".into(),
        publication_history: publication_history.clone(),
        updated_at: noted_at.clone(),
        ..existing_draft.clone()
    };
    let preview_alt_text = updated_draft.preview.alt_text.clone();

    if let Err(error) =
        persist_publication_history(base_dir, &existing_draft.preset_id, &publication_history)
    {
        return Err(rollback_publication_side_effects(
            base_dir,
            &draft_path,
            &existing_draft,
            &previous_publication_history,
            Some(&final_bundle_dir),
            error,
        ));
    }

    if let Err(error) = write_draft_summary(&draft_path, &updated_draft) {
        return Err(rollback_publication_side_effects(
            base_dir,
            &draft_path,
            &existing_draft,
            &previous_publication_history,
            Some(&final_bundle_dir),
            error,
        ));
    }

    if let Err(error) = publish_preset_to_live_catalog(
        base_dir,
        &existing_draft.preset_id,
        &input.published_version,
        &input.actor_id,
        &input.actor_label,
        &noted_at,
    ) {
        return Err(rollback_publication_side_effects(
            base_dir,
            &draft_path,
            &existing_draft,
            &previous_publication_history,
            Some(&final_bundle_dir),
            error,
        ));
    }

    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: noted_at.clone(),
            session_id: None,
            event_category: "publication-recovery",
            event_type: "publication-approved",
            summary: "게시 승인 검토를 기록했어요.".into(),
            detail: "future session catalog에 반영할 승인 이력이 중앙 audit store에 기록되었어요."
                .into(),
            actor_id: Some(input.actor_id.clone()),
            source: "preset-authoring",
            capture_id: None,
            preset_id: Some(existing_draft.preset_id.clone()),
            published_version: Some(input.published_version.clone()),
            reason_code: None,
        },
    );
    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: noted_at.clone(),
            session_id: None,
            event_category: "publication-recovery",
            event_type: "publication-published",
            summary: "새 preset 게시를 완료했어요.".into(),
            detail: "immutable published bundle과 future session catalog 반영이 함께 확정되었어요."
                .into(),
            actor_id: Some(input.actor_id.clone()),
            source: "preset-authoring",
            capture_id: None,
            preset_id: Some(existing_draft.preset_id.clone()),
            published_version: Some(input.published_version.clone()),
            reason_code: None,
        },
    );

    Ok(PublishValidatedPresetResultDto::Published {
        schema_version: DRAFT_PRESET_PUBLICATION_RESULT_SCHEMA_VERSION.into(),
        draft: updated_draft,
        published_preset: PublishedPresetSummaryDto {
            preset_id: input.preset_id.clone(),
            display_name: existing_draft.display_name.clone(),
            published_version: input.published_version.clone(),
            booth_status: "booth-safe".into(),
            preview: crate::contracts::dto::PresetPreviewAssetDto {
                kind: "preview-tile".into(),
                asset_path: preview_asset_path,
                alt_text: preview_alt_text,
            },
        },
        bundle_path: final_bundle_dir.to_string_lossy().replace('\\', "/"),
        audit_record: published_record,
    })
}

fn reject_publication(
    base_dir: &Path,
    draft_path: &Path,
    mut draft: DraftPresetSummaryDto,
    input: &PublishValidatedPresetInputDto,
    reason_code: &str,
    message: &str,
    guidance: &str,
    noted_at: String,
) -> Result<PublishValidatedPresetResultDto, HostErrorEnvelope> {
    let previous_draft = draft.clone();
    let previous_publication_history = load_publication_history(base_dir, &draft.preset_id);
    let audit_record = build_publication_audit_record(
        &draft,
        input,
        "rejected",
        input.review_note.as_deref(),
        Some(reason_code),
        guidance,
        &noted_at,
    );
    let mut publication_history = previous_publication_history.clone();
    publication_history.push(audit_record.clone());
    if let Err(error) =
        persist_publication_history(base_dir, &draft.preset_id, &publication_history)
    {
        return Err(error);
    }
    draft.publication_history = publication_history;
    draft.updated_at = noted_at.clone();
    if let Err(error) = write_draft_summary(draft_path, &draft) {
        return Err(rollback_publication_side_effects(
            base_dir,
            draft_path,
            &previous_draft,
            &previous_publication_history,
            None,
            error,
        ));
    }

    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: noted_at.clone(),
            session_id: None,
            event_category: "publication-recovery",
            event_type: "publication-rejected",
            summary: message.into(),
            detail: guidance.into(),
            actor_id: Some(input.actor_id.clone()),
            source: "preset-authoring",
            capture_id: None,
            preset_id: Some(draft.preset_id.clone()),
            published_version: Some(input.published_version.clone()),
            reason_code: Some(reason_code.into()),
        },
    );

    Ok(PublishValidatedPresetResultDto::Rejected {
        schema_version: DRAFT_PRESET_PUBLICATION_RESULT_SCHEMA_VERSION.into(),
        draft,
        reason_code: reason_code.into(),
        message: message.into(),
        guidance: guidance.into(),
        audit_record,
    })
}

fn build_publication_audit_record(
    draft: &DraftPresetSummaryDto,
    input: &PublishValidatedPresetInputDto,
    action: &str,
    review_note: Option<&str>,
    reason_code: Option<&str>,
    guidance: &str,
    noted_at: &str,
) -> PresetPublicationAuditRecordDto {
    PresetPublicationAuditRecordDto {
        schema_version: PRESET_PUBLICATION_AUDIT_SCHEMA_VERSION.into(),
        preset_id: draft.preset_id.clone(),
        draft_version: draft.draft_version,
        published_version: input.published_version.clone(),
        actor_id: input.actor_id.trim().to_string(),
        actor_label: input.actor_label.trim().to_string(),
        review_note: normalize_optional_text(review_note),
        action: action.into(),
        reason_code: reason_code.map(|code| code.to_string()),
        guidance: guidance.into(),
        noted_at: noted_at.into(),
    }
}

fn load_publication_history(
    base_dir: &Path,
    preset_id: &str,
) -> Vec<PresetPublicationAuditRecordDto> {
    let audit_path = resolve_publication_audit_path(base_dir, preset_id);
    let Ok(bytes) = fs::read_to_string(audit_path) else {
        return Vec::new();
    };
    let Ok(history) = serde_json::from_str::<Vec<PresetPublicationAuditRecordDto>>(&bytes) else {
        return Vec::new();
    };

    history
        .into_iter()
        .filter(is_valid_publication_audit_record)
        .collect()
}

fn resolve_publication_audit_path(base_dir: &Path, preset_id: &str) -> PathBuf {
    base_dir
        .join("preset-authoring")
        .join("publication-audit")
        .join(format!("{preset_id}.json"))
}

fn is_valid_publication_audit_record(record: &PresetPublicationAuditRecordDto) -> bool {
    record.schema_version == PRESET_PUBLICATION_AUDIT_SCHEMA_VERSION
        && crate::contracts::dto::is_valid_preset_id(&record.preset_id)
        && record.draft_version > 0
        && crate::contracts::dto::is_valid_published_version(&record.published_version)
        && crate::contracts::dto::is_valid_actor_id(&record.actor_id)
        && crate::contracts::dto::is_non_blank(&record.actor_label)
        && record
            .review_note
            .as_ref()
            .map(|note| crate::contracts::dto::is_non_blank(note))
            .unwrap_or(true)
        && matches!(
            record.action.as_str(),
            "approved" | "published" | "rejected"
        )
        && record
            .reason_code
            .as_ref()
            .map(|code| {
                matches!(
                    code.as_str(),
                    "draft-not-validated"
                        | "stale-validation"
                        | "metadata-mismatch"
                        | "duplicate-version"
                        | "path-escape"
                        | "future-session-only-violation"
                )
            })
            .unwrap_or(matches!(record.action.as_str(), "approved" | "published"))
        && crate::contracts::dto::is_non_blank(&record.guidance)
        && crate::contracts::dto::is_non_blank(&record.noted_at)
}

fn persist_publication_history(
    base_dir: &Path,
    preset_id: &str,
    history: &[PresetPublicationAuditRecordDto],
) -> Result<(), HostErrorEnvelope> {
    let audit_path = resolve_publication_audit_path(base_dir, preset_id);

    if history.is_empty() {
        if audit_path.exists() {
            fs::remove_file(&audit_path).map_err(map_fs_error)?;
        }

        return Ok(());
    }

    let audit_dir = audit_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("게시 감사 이력 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(audit_dir).map_err(map_fs_error)?;

    let bytes = serde_json::to_vec_pretty(history).map_err(|error| {
        HostErrorEnvelope::persistence(format!("게시 감사 이력을 직렬화하지 못했어요: {error}"))
    })?;
    write_json_bytes_atomically(&audit_path, &bytes)
}

fn rollback_publication_side_effects(
    base_dir: &Path,
    draft_path: &Path,
    previous_draft: &DraftPresetSummaryDto,
    previous_publication_history: &[PresetPublicationAuditRecordDto],
    final_bundle_dir: Option<&Path>,
    error: HostErrorEnvelope,
) -> HostErrorEnvelope {
    let mut rollback_failures = Vec::new();

    if let Some(bundle_dir) = final_bundle_dir {
        if bundle_dir.exists() {
            if let Err(rollback_error) = fs::remove_dir_all(bundle_dir) {
                rollback_failures.push(format!("bundle rollback failed: {rollback_error}"));
            }
        }
    }

    if let Err(rollback_error) = persist_publication_history(
        base_dir,
        &previous_draft.preset_id,
        previous_publication_history,
    ) {
        rollback_failures.push(format!("audit rollback failed: {}", rollback_error.message));
    }

    if let Err(rollback_error) = write_draft_summary(draft_path, previous_draft) {
        rollback_failures.push(format!("draft rollback failed: {}", rollback_error.message));
    }

    if rollback_failures.is_empty() {
        error
    } else {
        HostErrorEnvelope::persistence(format!(
            "{} 롤백도 일부 실패했어요: {}",
            error.message,
            rollback_failures.join(" / ")
        ))
    }
}

fn resolve_workspace_file_for_publication(
    draft_dir: &Path,
    relative_path: &str,
    field_path: &str,
) -> Result<PathBuf, (&'static str, &'static str, &'static str)> {
    let draft_root = fs::canonicalize(draft_dir).map_err(|_| {
        (
            "metadata-mismatch",
            "draft 작업공간을 다시 확인해 주세요.",
            "draft 작업공간 루트를 다시 불러온 뒤 게시를 다시 시도해 주세요.",
        )
    })?;
    let joined_path = draft_dir.join(relative_path);
    let resolved = fs::canonicalize(&joined_path).map_err(|_| {
        (
            "metadata-mismatch",
            "게시에 필요한 artifact가 validation 이후에 바뀌었거나 없어졌어요.",
            "validation을 다시 실행해 최신 artifact 기준으로 다시 게시해 주세요.",
        )
    })?;

    if !resolved.starts_with(&draft_root) {
        return Err((
            "path-escape",
            "게시에 사용할 artifact 경로가 authoring 작업공간 밖으로 벗어났어요.",
            "symlink나 외부 경로 연결 없이 draft 작업공간 내부 파일만 다시 연결해 주세요.",
        ));
    }

    if !resolved.is_file() {
        return Err((
            "metadata-mismatch",
            "게시에 필요한 artifact가 validation 이후에 바뀌었거나 없어졌어요.",
            "validation을 다시 실행해 최신 artifact 기준으로 다시 게시해 주세요.",
        ));
    }

    let _ = field_path;

    Ok(resolved)
}

fn resolve_temp_bundle_dir(final_bundle_dir: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let version = final_bundle_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "publish".into());

    final_bundle_dir.with_file_name(format!("{version}.tmp-{stamp}"))
}

fn create_published_bundle_from_draft(
    bundle_dir: &Path,
    draft: &DraftPresetSummaryDto,
    input: &PublishValidatedPresetInputDto,
    published_at: &str,
    preview_source: &Path,
    sample_cut_source: &Path,
    darktable_source: &Path,
    xmp_source: &Path,
) -> Result<(), HostErrorEnvelope> {
    if bundle_dir.exists() {
        fs::remove_dir_all(bundle_dir).map_err(map_fs_error)?;
    }

    fs::create_dir_all(bundle_dir).map_err(map_fs_error)?;
    let preview_relative = copy_bundle_asset(bundle_dir, "preview", preview_source)?;
    let sample_cut_relative = copy_bundle_asset(bundle_dir, "sample-cut", sample_cut_source)?;
    let darktable_relative = copy_bundle_asset(bundle_dir, "darktable", darktable_source)?;
    let xmp_relative = copy_bundle_asset(bundle_dir, "xmp", xmp_source)?;
    let bundle_value = serde_json::json!({
        "schemaVersion": PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION,
        "presetId": draft.preset_id,
        "displayName": draft.display_name,
        "publishedVersion": input.published_version,
        "lifecycleStatus": "published",
        "boothStatus": "booth-safe",
        "darktableVersion": draft.darktable_version,
        "sourceDraftVersion": draft.draft_version,
        "publishedAt": published_at,
        "publishedBy": {
            "actorId": input.actor_id,
            "actorLabel": input.actor_label,
        },
        "previewProfile": {
            "profileId": draft.preview_profile.profile_id,
            "displayName": draft.preview_profile.display_name,
            "outputColorSpace": draft.preview_profile.output_color_space,
        },
        "finalProfile": {
            "profileId": draft.final_profile.profile_id,
            "displayName": draft.final_profile.display_name,
            "outputColorSpace": draft.final_profile.output_color_space,
        },
        "preview": {
            "kind": "preview-tile",
            "assetPath": preview_relative,
            "altText": draft.preview.alt_text,
        },
        "sampleCut": {
            "kind": "sample-cut",
            "assetPath": sample_cut_relative,
            "altText": draft.sample_cut.alt_text,
        },
        "darktableProjectPath": darktable_relative,
        "xmpTemplatePath": xmp_relative,
    });
    let bundle_bytes = serde_json::to_vec_pretty(&bundle_value).map_err(|error| {
        HostErrorEnvelope::persistence(format!("published bundle을 직렬화하지 못했어요: {error}"))
    })?;
    write_json_bytes_atomically(&bundle_dir.join("bundle.json"), &bundle_bytes)
}

fn copy_bundle_asset(
    bundle_dir: &Path,
    subdir: &str,
    source_path: &Path,
) -> Result<String, HostErrorEnvelope> {
    let file_name = source_path.file_name().ok_or_else(|| {
        HostErrorEnvelope::persistence("게시할 artifact 파일 이름을 준비하지 못했어요.")
    })?;
    let target_dir = bundle_dir.join(subdir);
    fs::create_dir_all(&target_dir).map_err(map_fs_error)?;
    let target_path = target_dir.join(file_name);
    fs::copy(source_path, &target_path).map_err(map_fs_error)?;

    Ok(format!("{subdir}/{}", file_name.to_string_lossy()))
}

fn build_absolute_asset_path(
    bundle_dir: &Path,
    subdir: &str,
    source_path: &Path,
) -> Result<String, HostErrorEnvelope> {
    let file_name = source_path
        .file_name()
        .ok_or_else(|| HostErrorEnvelope::persistence("게시 자산 경로를 준비하지 못했어요."))?;

    Ok(bundle_dir
        .join(subdir)
        .join(file_name)
        .to_string_lossy()
        .replace('\\', "/"))
}

pub fn ensure_authoring_access(
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<(), HostErrorEnvelope> {
    if capability_snapshot.is_admin_authenticated
        && capability_snapshot
            .allowed_surfaces
            .iter()
            .any(|surface| surface == "authoring")
    {
        return Ok(());
    }

    Err(HostErrorEnvelope::capability_denied(
        "승인된 내부 authoring 세션에서만 draft 작업공간을 열 수 있어요.",
    ))
}

pub fn ensure_authoring_window_label(window_label: &str) -> Result<(), HostErrorEnvelope> {
    if window_label == AUTHORING_WINDOW_LABEL {
        return Ok(());
    }

    Err(HostErrorEnvelope::capability_denied(
        "draft authoring 명령은 authoring 전용 창에서만 실행할 수 있어요.",
    ))
}

fn empty_authoring_workspace() -> AuthoringWorkspaceResultDto {
    AuthoringWorkspaceResultDto {
        schema_version: AUTHORING_WORKSPACE_SCHEMA_VERSION.into(),
        supported_lifecycle_states: supported_lifecycle_states(),
        drafts: Vec::new(),
        invalid_drafts: Vec::new(),
    }
}

fn supported_lifecycle_states() -> Vec<String> {
    vec![
        "draft".into(),
        "validated".into(),
        "approved".into(),
        "published".into(),
    ]
}

fn resolve_draft_file_path(drafts_root: &Path, preset_id: &str) -> PathBuf {
    drafts_root.join(preset_id).join("draft.json")
}

enum DraftArtifactInspection {
    Valid(DraftPresetSummaryDto),
    Invalid(InvalidDraftArtifactDto),
}

fn load_required_draft_summary(
    base_dir: &Path,
    draft_path: &Path,
    missing_message: &str,
    malformed_message: &str,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    if !draft_path.exists() {
        return Err(HostErrorEnvelope::validation_message(missing_message));
    }

    let draft_bytes = fs::read_to_string(draft_path).map_err(map_fs_error)?;
    let trusted_preset_id = trusted_draft_folder_name(draft_path)
        .ok_or_else(|| HostErrorEnvelope::validation_message(malformed_message))?;
    let mut summary: DraftPresetSummaryDto = serde_json::from_str(&draft_bytes)
        .map_err(|_| HostErrorEnvelope::validation_message(malformed_message))?;

    if summary.preset_id != trusted_preset_id {
        return Err(HostErrorEnvelope::validation_message(malformed_message));
    }

    summary.publication_history = load_publication_history(base_dir, &trusted_preset_id);

    if !is_valid_draft_summary(draft_path, &summary) {
        return Err(HostErrorEnvelope::validation_message(malformed_message));
    }

    Ok(summary)
}

fn inspect_draft_artifact(
    base_dir: &Path,
    draft_dir: &Path,
    draft_path: &Path,
) -> DraftArtifactInspection {
    let draft_folder = draft_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "unknown-draft".into());

    if !draft_path.exists() {
        return DraftArtifactInspection::Invalid(InvalidDraftArtifactDto {
            draft_folder,
            message: "저장된 draft 기록 파일이 없어서 작업공간에서 다시 열 수 없어요.".into(),
            guidance:
                "목록에서 손상 draft 정리를 실행한 뒤 새 draft를 만들고 필요한 메타데이터와 자산 참조를 다시 저장해 주세요.".into(),
            can_repair: true,
        });
    }

    let Ok(draft_bytes) = fs::read_to_string(draft_path) else {
        return DraftArtifactInspection::Invalid(InvalidDraftArtifactDto {
            draft_folder,
            message: "저장된 draft 기록을 지금은 읽지 못하고 있어요.".into(),
            guidance:
                "파일 잠금이나 권한 문제일 수 있어 자동 정리는 막았어요. 잠시 후 다시 시도하거나 작업공간 접근 상태를 먼저 확인해 주세요."
                    .into(),
            can_repair: false,
        });
    };

    let Ok(summary) = serde_json::from_str::<DraftPresetSummaryDto>(&draft_bytes) else {
        return DraftArtifactInspection::Invalid(InvalidDraftArtifactDto {
            draft_folder,
            message: "저장된 draft JSON 형식이 손상되어 작업공간에서 열 수 없어요.".into(),
            guidance:
                "목록에서 손상 draft 정리를 실행한 뒤 새 draft를 만들고 메타데이터와 자산 참조를 다시 저장해 주세요.".into(),
            can_repair: true,
        });
    };

    let Some(trusted_preset_id) = trusted_draft_folder_name(draft_path) else {
        return DraftArtifactInspection::Invalid(InvalidDraftArtifactDto {
            draft_folder,
            message: "draft 폴더 이름이 현재 authoring 규칙과 맞지 않아 자동 복구를 보류했어요."
                .into(),
            guidance:
                "자동 삭제 대신 작업공간을 수동 점검해 주세요. 폴더 이름과 presetId를 맞춘 뒤 다시 불러오면 기록을 보존할 수 있어요."
                    .into(),
            can_repair: false,
        });
    };

    if summary.preset_id != trusted_preset_id {
        return DraftArtifactInspection::Invalid(InvalidDraftArtifactDto {
            draft_folder,
            message: "draft 폴더 이름과 저장된 presetId가 서로 달라 자동 정리를 막았어요.".into(),
            guidance:
                "자동 삭제 대신 작업공간을 수동 점검해 주세요. 폴더 이름과 presetId를 맞추면 기존 draft와 자산을 보존할 수 있어요."
                    .into(),
            can_repair: false,
        });
    }

    let mut summary = summary;
    summary.publication_history = load_publication_history(base_dir, &trusted_preset_id);

    if !is_valid_draft_summary(draft_path, &summary) {
        return DraftArtifactInspection::Invalid(InvalidDraftArtifactDto {
            draft_folder,
            message: "저장된 draft metadata가 현재 authoring 계약과 맞지 않아 자동 정리를 막았어요."
                .into(),
            guidance:
                "자동 삭제 대신 작업공간을 수동 점검해 주세요. 필요한 경우 metadata를 바로잡은 뒤 다시 불러오거나 새 draft로 이관해 주세요."
                    .into(),
            can_repair: false,
        });
    }

    DraftArtifactInspection::Valid(summary)
}

fn trusted_draft_folder_name(draft_path: &Path) -> Option<String> {
    let draft_folder = draft_path
        .parent()?
        .file_name()?
        .to_str()?
        .trim()
        .to_string();

    if crate::contracts::dto::is_valid_preset_id(&draft_folder) {
        Some(draft_folder)
    } else {
        None
    }
}

fn ensure_mutable_authoring_lifecycle(
    lifecycle_state: &str,
    action: &str,
) -> Result<(), HostErrorEnvelope> {
    if matches!(lifecycle_state, "approved" | "published") {
        return Err(HostErrorEnvelope::validation_message(format!(
            "승인 또는 게시 완료 기록은 이 단계에서 다시 {action}할 수 없어요. 새 draft를 만들어 주세요."
        )));
    }

    Ok(())
}

fn is_valid_draft_summary(draft_path: &Path, summary: &DraftPresetSummaryDto) -> bool {
    if summary.schema_version != DRAFT_PRESET_ARTIFACT_SCHEMA_VERSION
        || summary.draft_version == 0
        || !matches!(
            summary.lifecycle_state.as_str(),
            "draft" | "validated" | "approved" | "published"
        )
        || !crate::contracts::dto::is_non_blank(&summary.display_name)
        || !crate::contracts::dto::is_non_blank(&summary.updated_at)
        || !crate::contracts::dto::is_valid_preset_id(&summary.preset_id)
        || !crate::contracts::dto::is_valid_darktable_version(&summary.darktable_version)
        || !crate::contracts::dto::is_safe_workspace_reference(&summary.darktable_project_path)
        || !crate::contracts::dto::is_safe_workspace_reference(&summary.xmp_template_path)
        || !is_valid_render_profile(&summary.preview_profile)
        || !is_valid_render_profile(&summary.final_profile)
        || !is_valid_noise_policy(&summary.noise_policy)
        || !is_valid_preview_reference(&summary.preview)
        || !is_valid_preview_reference(&summary.sample_cut)
        || !is_valid_validation_snapshot(
            &summary.preset_id,
            summary.draft_version,
            &summary.lifecycle_state,
            &summary.validation,
        )
        || !summary
            .publication_history
            .iter()
            .all(is_valid_publication_audit_record)
    {
        return false;
    }

    draft_path
        .parent()
        .and_then(|dir| dir.file_name())
        .map(|folder| folder.to_string_lossy() == summary.preset_id)
        .unwrap_or(false)
}

fn is_valid_render_profile(profile: &DraftRenderProfileDto) -> bool {
    crate::contracts::dto::is_non_blank(&profile.profile_id)
        && crate::contracts::dto::is_non_blank(&profile.display_name)
        && crate::contracts::dto::is_non_blank(&profile.output_color_space)
}

fn is_valid_noise_policy(policy: &DraftNoisePolicyDto) -> bool {
    crate::contracts::dto::is_non_blank(&policy.policy_id)
        && crate::contracts::dto::is_non_blank(&policy.display_name)
        && crate::contracts::dto::is_non_blank(&policy.reduction_mode)
}

fn is_valid_preview_reference(reference: &DraftPresetPreviewReferenceDto) -> bool {
    crate::contracts::dto::is_safe_workspace_reference(&reference.asset_path)
        && crate::contracts::dto::is_non_blank(&reference.alt_text)
}

fn is_valid_validation_snapshot(
    preset_id: &str,
    draft_version: u32,
    lifecycle_state: &str,
    validation: &DraftValidationSnapshotDto,
) -> bool {
    match validation.status.as_str() {
        "not-run" => {
            if validation.latest_report.is_some() {
                return false;
            }
        }
        "passed" | "failed" => {
            let Some(latest_report) = validation.latest_report.as_ref() else {
                return false;
            };

            if !is_valid_validation_report(latest_report) {
                return false;
            }

            if latest_report.status != validation.status {
                return false;
            }

            if latest_report.preset_id != preset_id || latest_report.draft_version != draft_version
            {
                return false;
            }

            if !validation.history.iter().any(|report| {
                report.checked_at == latest_report.checked_at
                    && report.draft_version == latest_report.draft_version
                    && report.preset_id == latest_report.preset_id
            }) {
                return false;
            }
        }
        _ => return false,
    }

    if !validation
        .history
        .iter()
        .all(|report| is_valid_validation_report(report) && report.preset_id == preset_id)
    {
        return false;
    }

    match lifecycle_state {
        "validated" => {
            validation.status == "passed"
                && validation
                    .latest_report
                    .as_ref()
                    .map(|report| report.lifecycle_state == "validated")
                    .unwrap_or(false)
        }
        "approved" | "published" => {
            validation.status == "passed"
                && validation
                    .latest_report
                    .as_ref()
                    .map(|report| report.lifecycle_state == "validated")
                    .unwrap_or(false)
        }
        "draft" => validation.status != "passed" || validation.latest_report.is_none(),
        _ => false,
    }
}

fn is_valid_validation_report(report: &DraftValidationReportDto) -> bool {
    if report.schema_version != DRAFT_PRESET_VALIDATION_SCHEMA_VERSION
        || !crate::contracts::dto::is_valid_preset_id(&report.preset_id)
        || report.draft_version == 0
        || !matches!(report.lifecycle_state.as_str(), "draft" | "validated")
        || !matches!(report.status.as_str(), "passed" | "failed")
        || !crate::contracts::dto::is_non_blank(&report.checked_at)
    {
        return false;
    }

    let has_error_finding = report
        .findings
        .iter()
        .any(|finding| finding.severity == "error");

    if report.status == "passed" && has_error_finding {
        return false;
    }

    if report.status == "passed" && report.lifecycle_state != "validated" {
        return false;
    }

    if report.status == "failed" && !has_error_finding {
        return false;
    }

    if report.status == "failed" && report.lifecycle_state != "draft" {
        return false;
    }

    report.findings.iter().all(is_valid_validation_finding)
}

fn is_valid_validation_finding(finding: &DraftValidationFindingDto) -> bool {
    !finding.rule_code.is_empty()
        && finding
            .rule_code
            .chars()
            .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-')
        && matches!(finding.severity.as_str(), "error" | "warning")
        && crate::contracts::dto::is_non_blank(&finding.message)
        && crate::contracts::dto::is_non_blank(&finding.guidance)
}

fn build_draft_summary(
    input: &DraftPresetEditPayloadDto,
    draft_version: u32,
    history: Vec<DraftValidationReportDto>,
    publication_history: Vec<PresetPublicationAuditRecordDto>,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    Ok(DraftPresetSummaryDto {
        schema_version: DRAFT_PRESET_ARTIFACT_SCHEMA_VERSION.into(),
        preset_id: input.preset_id.clone(),
        display_name: input.display_name.trim().to_string(),
        draft_version,
        lifecycle_state: "draft".into(),
        darktable_version: input.darktable_version.trim().to_string(),
        darktable_project_path: input.darktable_project_path.trim().to_string(),
        xmp_template_path: input.xmp_template_path.trim().to_string(),
        preview_profile: normalize_render_profile(&input.preview_profile),
        final_profile: normalize_render_profile(&input.final_profile),
        noise_policy: normalize_noise_policy(&input.noise_policy),
        preview: normalize_preview_reference(&input.preview),
        sample_cut: normalize_preview_reference(&input.sample_cut),
        description: normalize_optional_text(input.description.as_deref()),
        notes: normalize_optional_text(input.notes.as_deref()),
        validation: DraftValidationSnapshotDto {
            status: "not-run".into(),
            latest_report: None,
            history,
        },
        publication_history,
        updated_at: current_timestamp(SystemTime::now())?,
    })
}

fn normalize_render_profile(profile: &DraftRenderProfileDto) -> DraftRenderProfileDto {
    DraftRenderProfileDto {
        profile_id: profile.profile_id.trim().to_string(),
        display_name: profile.display_name.trim().to_string(),
        output_color_space: profile.output_color_space.trim().to_string(),
    }
}

fn normalize_noise_policy(policy: &DraftNoisePolicyDto) -> DraftNoisePolicyDto {
    DraftNoisePolicyDto {
        policy_id: policy.policy_id.trim().to_string(),
        display_name: policy.display_name.trim().to_string(),
        reduction_mode: policy.reduction_mode.trim().to_string(),
    }
}

fn normalize_preview_reference(
    reference: &DraftPresetPreviewReferenceDto,
) -> DraftPresetPreviewReferenceDto {
    DraftPresetPreviewReferenceDto {
        asset_path: reference.asset_path.trim().to_string(),
        alt_text: reference.alt_text.trim().to_string(),
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let normalized = value.trim();

        (!normalized.is_empty()).then(|| normalized.to_string())
    })
}

fn build_validation_report(
    draft_path: &Path,
    draft: &DraftPresetSummaryDto,
    checked_at: String,
) -> DraftValidationReportDto {
    let draft_dir = match draft_path.parent() {
        Some(path) => path,
        None => {
            return DraftValidationReportDto {
                schema_version: DRAFT_PRESET_VALIDATION_SCHEMA_VERSION.into(),
                preset_id: draft.preset_id.clone(),
                draft_version: draft.draft_version,
                lifecycle_state: "draft".into(),
                status: "failed".into(),
                checked_at,
                findings: vec![validation_error(
                    "draft-root-missing",
                    None,
                    "draft 작업공간 루트를 찾지 못했어요.",
                    "preset-authoring/drafts 아래에 저장된 draft 경로를 다시 확인해 주세요.",
                )],
            }
        }
    };
    let mut findings = Vec::new();

    if !crate::contracts::dto::is_valid_darktable_version(&draft.darktable_version) {
        findings.push(validation_error(
            "darktable-version-format",
            Some("darktableVersion"),
            "darktableVersion 형식이 올바르지 않아요.",
            "darktableVersion을 `5.4.1`처럼 major.minor.patch 형식으로 저장해 주세요.",
        ));
    } else if draft.darktable_version != PINNED_DARKTABLE_VERSION {
        findings.push(validation_error(
            "darktable-version-mismatch",
            Some("darktableVersion"),
            "Pinned darktable 버전과 맞지 않아요.",
            "darktableVersion을 5.4.1로 맞춘 뒤 다시 검증해 주세요.",
        ));
    }

    validate_required_file(
        draft_dir,
        &draft.darktable_project_path,
        "darktableProjectPath",
        &[".dtpreset"],
        "darktable-project-missing",
        "darktable-project-extension",
        "darktable project artifact를 찾지 못했어요.",
        "darktableProjectPath에 draft 작업공간 안의 .dtpreset 파일을 연결해 주세요.",
        &mut findings,
    );
    validate_required_file(
        draft_dir,
        &draft.xmp_template_path,
        "xmpTemplatePath",
        &[".xmp"],
        "xmp-template-missing",
        "xmp-template-extension",
        "XMP template artifact를 찾지 못했어요.",
        "xmpTemplatePath에 draft 작업공간 안의 .xmp 파일을 연결해 주세요.",
        &mut findings,
    );
    validate_required_file(
        draft_dir,
        &draft.preview.asset_path,
        "preview.assetPath",
        &[".jpg", ".jpeg", ".png"],
        "preview-asset-missing",
        "preview-asset-extension",
        "대표 preview 자산을 찾지 못했어요.",
        "preview.assetPath를 draft 작업공간 안의 대표 preview 이미지로 연결해 주세요.",
        &mut findings,
    );
    validate_required_file(
        draft_dir,
        &draft.sample_cut.asset_path,
        "sampleCut.assetPath",
        &[".jpg", ".jpeg", ".png"],
        "sample-cut-missing",
        "sample-cut-extension",
        "대표 sample-cut 자산을 찾지 못했어요.",
        "sampleCut.assetPath에 booth catalog 검토용 샘플 이미지를 추가해 주세요.",
        &mut findings,
    );

    if !is_valid_render_profile(&draft.preview_profile) {
        findings.push(validation_error(
            "preview-profile-incomplete",
            Some("previewProfile"),
            "preview profile metadata가 비어 있어요.",
            "previewProfile의 ID, 이름, output color space를 모두 채워 주세요.",
        ));
    }

    if !is_valid_render_profile(&draft.final_profile) {
        findings.push(validation_error(
            "final-profile-incomplete",
            Some("finalProfile"),
            "final profile metadata가 비어 있어요.",
            "finalProfile의 ID, 이름, output color space를 모두 채워 주세요.",
        ));
    }

    if !is_valid_noise_policy(&draft.noise_policy) {
        findings.push(validation_error(
            "noise-policy-incomplete",
            Some("noisePolicy"),
            "noise policy metadata가 비어 있어요.",
            "noisePolicy의 ID, 이름, reduction mode를 모두 채워 주세요.",
        ));
    }

    let xmp_path = resolve_existing_workspace_file(draft_dir, &draft.xmp_template_path);
    let preview_asset_path = resolve_existing_workspace_file(draft_dir, &draft.preview.asset_path);

    if let Some(xmp_path) = xmp_path.as_ref() {
        if !is_render_compatible_xmp(xmp_path) {
            findings.push(validation_error(
                "render-compatibility-check",
                Some("xmpTemplatePath"),
                "XMP template가 booth render 경로와 호환되는 형식을 확인하지 못했어요.",
                "darktable에서 다시 내보낸 XMP template를 연결하고 history stack이 포함되었는지 확인해 주세요.",
            ));
        }
    }

    if findings.is_empty() {
        if let (Some(preview_asset_path), Some(xmp_path)) =
            (preview_asset_path.as_ref(), xmp_path.as_ref())
        {
            match xmp_produces_visible_render_delta(preview_asset_path, xmp_path) {
                Ok(true) => {}
                Ok(false) => findings.push(validation_error(
                    "render-delta-missing",
                    Some("xmpTemplatePath"),
                    "XMP template가 booth preview proof에서 기본 렌더와 구분되는 변화를 만들지 못했어요.",
                    "대표 preview 자산으로 다시 export해 XMP 적용 결과가 기본 렌더와 실제로 달라지는지 확인한 뒤 재검증해 주세요.",
                )),
                Err(error) => findings.push(validation_error(
                    "render-proof-unavailable",
                    Some("xmpTemplatePath"),
                    "XMP template의 booth render proof를 확인하지 못했어요.",
                    &format!(
                        "darktable-cli render proof를 다시 실행할 수 있게 preview asset과 XMP template를 점검해 주세요. detail={error}"
                    ),
                )),
            }
        }
    }

    let status = if findings.iter().any(|finding| finding.severity == "error") {
        "failed"
    } else {
        "passed"
    };
    let lifecycle_state = if status == "passed" {
        "validated"
    } else {
        "draft"
    };

    DraftValidationReportDto {
        schema_version: DRAFT_PRESET_VALIDATION_SCHEMA_VERSION.into(),
        preset_id: draft.preset_id.clone(),
        draft_version: draft.draft_version,
        lifecycle_state: lifecycle_state.into(),
        status: status.into(),
        checked_at,
        findings,
    }
}

fn validate_required_file(
    draft_dir: &Path,
    relative_path: &str,
    field_path: &str,
    allowed_extensions: &[&str],
    missing_rule_code: &str,
    extension_rule_code: &str,
    missing_message: &str,
    guidance: &str,
    findings: &mut Vec<DraftValidationFindingDto>,
) {
    if let Some(path) = resolve_existing_workspace_file(draft_dir, relative_path) {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if !allowed_extensions
            .iter()
            .any(|extension| lower.ends_with(extension))
        {
            findings.push(validation_error(
                extension_rule_code,
                Some(field_path),
                "artifact 확장자 형식이 booth compatibility 기준과 맞지 않아요.",
                guidance,
            ));
        }

        return;
    }

    findings.push(validation_error(
        missing_rule_code,
        Some(field_path),
        missing_message,
        guidance,
    ));
}

fn resolve_existing_workspace_file(draft_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    let draft_root = fs::canonicalize(draft_dir).ok()?;
    let resolved = fs::canonicalize(draft_dir.join(relative_path)).ok()?;

    if !resolved.starts_with(&draft_root) || !resolved.is_file() {
        return None;
    }

    Some(resolved)
}

fn xmp_produces_visible_render_delta(
    preview_asset_path: &Path,
    xmp_path: &Path,
) -> Result<bool, String> {
    let probe_dir = build_validation_probe_dir()?;
    let baseline_output = probe_dir.join("baseline.jpg");
    let xmp_output = probe_dir.join("xmp.jpg");

    let result = (|| {
        run_render_validation_probe(preview_asset_path, None, &baseline_output)?;
        run_render_validation_probe(preview_asset_path, Some(xmp_path), &xmp_output)?;
        compare_render_probe_outputs(&baseline_output, &xmp_output)
    })();

    let _ = fs::remove_dir_all(&probe_dir);

    result
}

fn build_validation_probe_dir() -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("validation probe timestamp를 만들지 못했어요: {error}"))?
        .as_nanos();
    let probe_dir = std::env::temp_dir().join(format!("boothy-render-proof-{stamp}"));

    fs::create_dir_all(&probe_dir)
        .map_err(|error| format!("validation probe 디렉터리를 만들지 못했어요: {error}"))?;

    Ok(probe_dir)
}

fn run_render_validation_probe(
    preview_asset_path: &Path,
    xmp_path: Option<&Path>,
    output_path: &Path,
) -> Result<(), String> {
    let binary = resolve_darktable_cli_binary_for_validation();
    let mut command = Command::new(&binary);

    command.arg(preview_asset_path);
    if let Some(xmp_path) = xmp_path {
        command.arg(xmp_path);
    }
    command
        .arg(output_path)
        .arg("--hq")
        .arg("false")
        .arg("--apply-custom-presets")
        .arg("false")
        .arg("--width")
        .arg(VALIDATION_RENDER_PROBE_MAX_WIDTH_PX.to_string())
        .arg("--height")
        .arg(VALIDATION_RENDER_PROBE_MAX_HEIGHT_PX.to_string())
        .arg("--core");

    let output = command.output().map_err(|error| {
        format!("darktable-cli proof를 시작하지 못했어요: binary={binary} error={error}")
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        return Err(format!(
            "darktable-cli proof가 실패했어요: exitCode={} stderr={stderr}",
            output.status.code().unwrap_or(-1)
        ));
    }

    if !output_path.is_file() {
        return Err("darktable-cli proof output이 생성되지 않았어요.".into());
    }

    Ok(())
}

fn resolve_darktable_cli_binary_for_validation() -> String {
    std::env::var(DARKTABLE_CLI_BIN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "darktable-cli".into())
}

fn compare_render_probe_outputs(baseline_output: &Path, xmp_output: &Path) -> Result<bool, String> {
    let baseline = load_render_probe_pixels(baseline_output)?;
    let xmp = load_render_probe_pixels(xmp_output)?;

    Ok(baseline != xmp)
}

fn load_render_probe_pixels(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let image = ImageReader::open(path)
        .map_err(|error| {
            format!(
                "render proof output을 열지 못했어요: path={} error={error}",
                path.display()
            )
        })?
        .decode()
        .map_err(|error| {
            format!(
                "render proof output을 decode하지 못했어요: path={} error={error}",
                path.display()
            )
        })?
        .to_rgba8();

    let (width, height) = image.dimensions();

    Ok((width, height, image.into_raw()))
}

fn is_render_compatible_xmp(path: &Path) -> bool {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return false,
    };
    let lowercase = strip_xml_noise(&contents).to_ascii_lowercase();
    let Some(history_body) = find_history_body(&lowercase) else {
        return false;
    };

    !history_body.is_empty() && has_supported_history_entry(history_body)
}

fn strip_xml_noise(contents: &str) -> String {
    let mut cleaned = String::with_capacity(contents.len());
    let mut remaining = contents;

    loop {
        let comment_index = remaining.find("<!--");
        let cdata_index = remaining.find("<![CDATA[");
        let next_match = match (comment_index, cdata_index) {
            (Some(comment), Some(cdata)) => {
                if comment <= cdata {
                    Some((comment, "-->"))
                } else {
                    Some((cdata, "]]>"))
                }
            }
            (Some(comment), None) => Some((comment, "-->")),
            (None, Some(cdata)) => Some((cdata, "]]>")),
            (None, None) => None,
        };

        let Some((index, terminator)) = next_match else {
            cleaned.push_str(remaining);
            break;
        };

        cleaned.push_str(&remaining[..index]);
        let section = &remaining[index..];
        let Some(end_index) = section.find(terminator) else {
            break;
        };
        remaining = &section[end_index + terminator.len()..];
    }

    cleaned
}

fn has_supported_history_entry(history_body: &str) -> bool {
    let supported_tags = ["item", "entry", "operation", "module", "li", "rdf:li"];
    let mut remaining = history_body;

    while let Some(tag_start) = remaining.find('<') {
        let candidate = &remaining[tag_start + 1..];

        if candidate.starts_with('/') || candidate.starts_with('!') || candidate.starts_with('?') {
            let Some(tag_end) = candidate.find('>') else {
                return false;
            };
            remaining = &candidate[tag_end + 1..];
            continue;
        }

        let tag_name_end = candidate
            .find(|character: char| {
                character == '>' || character == '/' || character.is_ascii_whitespace()
            })
            .unwrap_or(candidate.len());
        let tag_name = &candidate[..tag_name_end];

        let Some(tag_close_index) = candidate.find('>') else {
            return false;
        };
        let tag_contents = &candidate[..=tag_close_index];

        if supported_tags
            .iter()
            .any(|supported| tag_name == *supported)
        {
            if tag_contents.trim_end().ends_with("/>") {
                return true;
            }

            let closing_tag = format!("</{tag_name}>");
            if candidate[tag_close_index + 1..].contains(&closing_tag) {
                return true;
            }
        }

        remaining = &candidate[tag_close_index + 1..];
    }

    false
}

fn find_history_body(contents: &str) -> Option<&str> {
    find_tag_body(contents, "<darktable:history", "</darktable:history>")
        .or_else(|| find_tag_body(contents, "<history", "</history>"))
}

fn find_tag_body<'a>(contents: &'a str, open_tag_prefix: &str, close_tag: &str) -> Option<&'a str> {
    let open_start = contents.find(open_tag_prefix)?;
    let open_end_offset = contents[open_start..].find('>')?;
    let open_end = open_start + open_end_offset;
    let close_start = contents[open_end + 1..].find(close_tag)?;
    let close_start = open_end + 1 + close_start;

    if close_start <= open_end {
        return None;
    }

    Some(contents[open_end + 1..close_start].trim())
}

fn validation_error(
    rule_code: &str,
    field_path: Option<&str>,
    message: &str,
    guidance: &str,
) -> DraftValidationFindingDto {
    DraftValidationFindingDto {
        rule_code: rule_code.into(),
        severity: "error".into(),
        field_path: field_path.map(|path| path.to_string()),
        message: message.into(),
        guidance: guidance.into(),
    }
}

fn write_draft_summary(
    draft_path: &Path,
    summary: &DraftPresetSummaryDto,
) -> Result<(), HostErrorEnvelope> {
    let draft_dir = draft_path
        .parent()
        .ok_or_else(|| HostErrorEnvelope::persistence("draft 저장 위치를 준비하지 못했어요."))?;
    fs::create_dir_all(draft_dir).map_err(map_fs_error)?;

    let draft_bytes = serde_json::to_vec_pretty(summary).map_err(|error| {
        HostErrorEnvelope::persistence(format!("draft artifact를 직렬화하지 못했어요: {error}"))
    })?;
    write_json_bytes_atomically(draft_path, &draft_bytes)
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

fn ensure_draft_file_path_within_root(
    drafts_root: &Path,
    draft_path: &Path,
) -> Result<(), HostErrorEnvelope> {
    let canonical_root = fs::canonicalize(drafts_root).map_err(map_fs_error)?;
    let draft_dir = draft_path
        .parent()
        .ok_or_else(|| HostErrorEnvelope::persistence("draft 저장 위치를 준비하지 못했어요."))?;

    if draft_dir.exists() {
        let metadata = fs::symlink_metadata(draft_dir).map_err(map_fs_error)?;
        if metadata.file_type().is_symlink() {
            return Err(HostErrorEnvelope::validation_message(
                "draft 작업공간은 authoring 루트 밖으로 연결된 링크를 사용할 수 없어요.",
            ));
        }
    }

    let canonical_draft_dir = if draft_dir.exists() {
        fs::canonicalize(draft_dir).map_err(map_fs_error)?
    } else {
        canonical_root.join(draft_dir.strip_prefix(drafts_root).map_err(|_| {
            HostErrorEnvelope::validation_message(
                "draft 작업공간 경로가 authoring 루트 밖으로 벗어났어요.",
            )
        })?)
    };

    if !canonical_draft_dir.starts_with(&canonical_root) {
        return Err(HostErrorEnvelope::validation_message(
            "draft 작업공간 경로가 authoring 루트 밖으로 벗어났어요.",
        ));
    }

    Ok(())
}

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("draft 작업공간 파일을 저장하지 못했어요: {error}"))
}
