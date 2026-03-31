use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::os::windows::fs::symlink_file;

use boothy_lib::{
    commands::runtime_commands::capability_snapshot_for_profile,
    contracts::dto::{
        DraftNoisePolicyDto, DraftPresetEditPayloadDto, DraftPresetPreviewReferenceDto,
        DraftRenderProfileDto, LoadPresetCatalogInputDto, PresetCatalogStateResultDto,
        PresetPublicationAuditRecordDto, PresetSelectionInputDto, PublishValidatedPresetInputDto,
        PublishValidatedPresetResultDto, RepairInvalidDraftInputDto, RollbackPresetCatalogInputDto,
        RollbackPresetCatalogResultDto, SessionStartInputDto, ValidateDraftPresetInputDto,
    },
    preset::{
        authoring_pipeline::{
            create_draft_preset_in_dir, ensure_authoring_window_label,
            load_authoring_workspace_in_dir, publish_validated_preset_in_dir,
            repair_invalid_draft_in_dir, resolve_draft_authoring_root, save_draft_preset_in_dir,
            validate_draft_preset_in_dir,
        },
        default_catalog::ensure_default_preset_catalog_in_dir,
        preset_bundle::load_published_preset_runtime_bundle,
        preset_catalog::{load_preset_catalog_in_dir, resolve_published_preset_catalog_dir},
        preset_catalog_state::{load_preset_catalog_state_in_dir, rollback_preset_catalog_in_dir},
    },
    session::{
        session_manifest::SessionManifest,
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-authoring-{test_name}-{stamp}"))
}

#[test]
fn draft_authoring_round_trips_through_a_separate_workspace_root() {
    let base_dir = unique_test_root("round-trip");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let create_result = create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("authorized authoring should create a draft");

    let saved_result = save_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        DraftPresetEditPayloadDto {
            display_name: "Soft Glow Draft v2".into(),
            notes: Some("preview 재검토".into()),
            ..sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft v2")
        },
    )
    .expect("existing draft should save");
    let workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should load after draft save");

    assert_eq!(create_result.draft_version, 1);
    assert_eq!(saved_result.draft_version, 2);
    assert_eq!(saved_result.lifecycle_state, "draft");
    assert_eq!(saved_result.darktable_version, "5.4.1");
    assert_eq!(saved_result.validation.status, "not-run");
    assert_eq!(workspace.drafts.len(), 1);
    assert_eq!(workspace.supported_lifecycle_states.len(), 4);
    assert_eq!(workspace.drafts[0].display_name, "Soft Glow Draft v2");
    assert_eq!(
        resolve_draft_authoring_root(&base_dir),
        base_dir.join("preset-authoring").join("drafts")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_authoring_rejects_invalid_input_and_missing_capability() {
    let base_dir = unique_test_root("guards");
    let denied_snapshot = capability_snapshot_for_profile("booth", false);
    let denied_error = create_draft_preset_in_dir(
        &base_dir,
        &denied_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect_err("booth profile should not create draft presets");

    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let invalid_escape_error = create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        DraftPresetEditPayloadDto {
            preview: DraftPresetPreviewReferenceDto {
                asset_path: "../outside/preview.jpg".into(),
                alt_text: "escaped preview".into(),
            },
            ..sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft")
        },
    )
    .expect_err("workspace escape should be rejected");
    let invalid_profile_error = create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        DraftPresetEditPayloadDto {
            final_profile: DraftRenderProfileDto {
                output_color_space: String::new(),
                ..render_profile("final-standard", "Final Standard")
            },
            ..sample_draft_payload("preset_soft-glow-draft-2", "Soft Glow Draft 2")
        },
    )
    .expect_err("incomplete render profile should be rejected");

    assert_eq!(denied_error.code, "capability-denied");
    assert_eq!(invalid_escape_error.code, "validation-error");
    assert_eq!(invalid_profile_error.code, "validation-error");
    assert_eq!(
        ensure_authoring_window_label("booth-window")
            .expect_err("booth window should not run authoring commands")
            .code,
        "capability-denied"
    );
    ensure_authoring_window_label("authoring-window").expect("authoring window should be accepted");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_promotes_to_validated_only_after_required_artifacts_exist() {
    let base_dir = unique_test_root("validated");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");

    let result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should succeed");

    assert_eq!(result.report.status, "passed");
    assert_eq!(result.report.lifecycle_state, "validated");
    assert!(result.report.findings.is_empty());
    assert_eq!(result.draft.lifecycle_state, "validated");
    assert_eq!(result.draft.validation.status, "passed");
    assert_eq!(result.draft.validation.history.len(), 1);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_authoring_rejects_invalid_darktable_version_format_at_the_host_boundary() {
    let base_dir = unique_test_root("invalid-darktable-version-format");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let error = create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        DraftPresetEditPayloadDto {
            darktable_version: "5.4".into(),
            ..sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft")
        },
    )
    .expect_err("host should reject malformed version strings");

    assert_eq!(error.code, "validation-error");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_rejects_xmp_templates_without_a_history_stack() {
    let base_dir = unique_test_root("xmp-without-history");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_xmp_without_history_assets(&base_dir, "preset_soft-glow-draft");

    let result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should return a report");

    assert_eq!(result.report.status, "failed");
    assert_eq!(result.draft.lifecycle_state, "draft");
    assert!(result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "render-compatibility-check"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_rejects_marker_only_xmp_templates_that_are_not_structurally_valid() {
    let base_dir = unique_test_root("xmp-marker-only");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_marker_only_xmp_assets(&base_dir, "preset_soft-glow-draft");

    let result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should return a report");

    assert_eq!(result.report.status, "failed");
    assert!(result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "render-compatibility-check"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_rejects_history_markers_hidden_inside_comments() {
    let base_dir = unique_test_root("xmp-comment-markers");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_commented_marker_xmp_assets(&base_dir, "preset_soft-glow-draft");

    let result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should return a report");

    assert_eq!(result.report.status, "failed");
    assert!(result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "render-compatibility-check"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_accepts_standard_darktable_sidecar_history_entries() {
    let base_dir = unique_test_root("standard-darktable-sidecar");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_standard_darktable_sidecar_assets(&base_dir, "preset_soft-glow-draft");

    let result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should return a report");

    assert_eq!(result.report.status, "passed");
    assert_eq!(result.draft.lifecycle_state, "validated");
    assert!(!result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "render-compatibility-check"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_returns_actionable_findings_without_mutating_catalog_or_active_session() {
    let base_dir = unique_test_root("validation-guards");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should start before preset selection");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("published preset should bind to the session");

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        DraftPresetEditPayloadDto {
            darktable_version: "5.5.0".into(),
            ..sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft")
        },
    )
    .expect("draft creation should not fail");
    scaffold_invalid_render_assets(&base_dir, "preset_soft-glow-draft");

    let manifest_path = SessionPaths::new(&base_dir, &session.session_id).manifest_path;
    let manifest_before = fs::read_to_string(&manifest_path).expect("manifest should exist");
    let catalog_before = snapshot_tree(&catalog_root);
    let catalog_result_before = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("published catalog should load before validation");

    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should return a report");

    let manifest_after = fs::read_to_string(&manifest_path).expect("manifest should still exist");
    let persisted_manifest: SessionManifest =
        serde_json::from_str(&manifest_after).expect("manifest should deserialize");
    let catalog_after = snapshot_tree(&catalog_root);
    let catalog_result_after = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("published catalog should still load");

    assert_eq!(validation_result.report.status, "failed");
    assert_eq!(validation_result.draft.lifecycle_state, "draft");
    assert!(validation_result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "darktable-version-mismatch"));
    assert!(validation_result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "render-compatibility-check"));
    assert_eq!(catalog_before, catalog_after);
    assert_eq!(manifest_before, manifest_after);
    let catalog_before_ids: Vec<_> = catalog_result_before
        .presets
        .iter()
        .map(|preset| {
            format!(
                "{}:{}:{}",
                preset.preset_id, preset.published_version, preset.display_name
            )
        })
        .collect();
    let catalog_after_ids: Vec<_> = catalog_result_after
        .presets
        .iter()
        .map(|preset| {
            format!(
                "{}:{}:{}",
                preset.preset_id, preset.published_version, preset.display_name
            )
        })
        .collect();

    assert_eq!(catalog_before_ids, catalog_after_ids);
    assert_eq!(
        persisted_manifest
            .active_preset
            .as_ref()
            .map(|preset| preset.preset_id.as_str()),
        Some("preset_soft-glow")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn draft_validation_fails_when_required_artifacts_are_missing() {
    let base_dir = unique_test_root("missing-artifacts");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");

    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should return a report");

    assert_eq!(validation_result.report.status, "failed");
    assert!(validation_result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "darktable-project-missing"));
    assert!(validation_result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "xmp-template-missing"));
    assert!(validation_result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "preview-asset-missing"));
    assert!(validation_result
        .report
        .findings
        .iter()
        .any(|finding| finding.rule_code == "sample-cut-missing"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn published_records_cannot_be_saved_or_revalidated_from_the_4_2_authoring_flow() {
    let base_dir = unique_test_root("published-read-only");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass");

    publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect("publish should succeed");

    let save_error = save_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        DraftPresetEditPayloadDto {
            display_name: "Soft Glow Draft Updated".into(),
            ..sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft Updated")
        },
    )
    .expect_err("published records should not be editable in the 4.2 flow");
    let validate_error = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect_err("published records should not be revalidated in the 4.2 flow");

    assert_eq!(save_error.code, "validation-error");
    assert!(save_error.message.contains("새 draft"));
    assert_eq!(validate_error.code, "validation-error");
    assert!(validate_error.message.contains("새 draft"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn publication_rejects_actor_label_and_review_note_that_exceed_host_contract_limits() {
    let base_dir = unique_test_root("publish-input-length-guards");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass before publish");

    let actor_label_error = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "K".repeat(121),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect_err("actor label longer than the shared contract should be rejected");
    let review_note_error = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: Some("n".repeat(2001)),
        },
    )
    .expect_err("review note longer than the shared contract should be rejected");

    assert_eq!(actor_label_error.code, "validation-error");
    assert_eq!(review_note_error.code, "validation-error");
    assert!(actor_label_error.message.contains("게시 승인자"));
    assert!(review_note_error.message.contains("2000자"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn authoring_workspace_skips_persisted_drafts_with_mismatched_latest_validation_truth() {
    let base_dir = unique_test_root("mismatched-latest-validation");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass");

    let draft_path = resolve_draft_authoring_root(&base_dir)
        .join("preset_soft-glow-draft")
        .join("draft.json");
    let mut persisted_draft: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&draft_path).expect("draft should exist"))
            .expect("draft json should deserialize");
    persisted_draft["validation"]["latestReport"]["presetId"] =
        serde_json::Value::String("preset_other-draft".into());
    fs::write(
        &draft_path,
        serde_json::to_vec_pretty(&persisted_draft).expect("draft should serialize"),
    )
    .expect("draft should write");

    let workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should still load");

    assert!(workspace.drafts.is_empty());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn malformed_persisted_draft_returns_actionable_repair_guidance_during_validation() {
    let base_dir = unique_test_root("malformed-validate-guidance");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");

    let draft_path = resolve_draft_authoring_root(&base_dir)
        .join("preset_soft-glow-draft")
        .join("draft.json");
    fs::write(&draft_path, "{ not-valid-json").expect("corrupt draft should write");

    let error = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect_err("corrupted persisted draft should surface repair guidance");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("손상"));
    assert!(error.message.contains("다시 저장") || error.message.contains("새 draft"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn authoring_workspace_surfaces_corrupted_drafts_as_repair_needed_entries() {
    let base_dir = unique_test_root("workspace-invalid-draft");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");

    let broken_draft_dir = resolve_draft_authoring_root(&base_dir).join("preset_broken-draft");
    fs::create_dir_all(&broken_draft_dir).expect("broken draft directory should exist");
    fs::write(broken_draft_dir.join("draft.json"), "{ not-valid-json")
        .expect("broken draft should write");

    let workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should still load with repair-needed entries");

    assert_eq!(workspace.drafts.len(), 1);
    assert_eq!(workspace.invalid_drafts.len(), 1);
    assert_eq!(
        workspace.invalid_drafts[0].draft_folder,
        "preset_broken-draft"
    );
    assert!(workspace.invalid_drafts[0].message.contains("손상"));
    assert!(workspace.invalid_drafts[0].guidance.contains("새 draft"));
    assert!(workspace.invalid_drafts[0].can_repair);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn repairing_invalid_draft_preserves_publication_history_and_allows_recreation() {
    let base_dir = unique_test_root("repair-invalid-draft");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let broken_draft_dir = resolve_draft_authoring_root(&base_dir).join("preset_broken-draft");
    let audit_path = base_dir
        .join("preset-authoring")
        .join("publication-audit")
        .join("preset_broken-draft.json");
    let audit_history = vec![PresetPublicationAuditRecordDto {
        schema_version: "preset-publication-audit/v1".into(),
        preset_id: "preset_broken-draft".into(),
        draft_version: 3,
        published_version: "2026.03.26".into(),
        actor_id: "manager-noah".into(),
        actor_label: "Noah Lee".into(),
        review_note: Some("이전 거절 이력 보존".into()),
        action: "rejected".into(),
        reason_code: Some("stale-validation".into()),
        guidance: "최신 검증을 다시 실행해 주세요.".into(),
        noted_at: "2026-03-26T09:30:00+09:00".into(),
    }];

    fs::create_dir_all(&broken_draft_dir).expect("broken draft directory should exist");
    fs::write(broken_draft_dir.join("draft.json"), "{ not-valid-json")
        .expect("broken draft should write");
    fs::create_dir_all(audit_path.parent().expect("audit directory should exist"))
        .expect("audit directory should exist");
    fs::write(
        &audit_path,
        serde_json::to_vec_pretty(&audit_history).expect("audit history should serialize"),
    )
    .expect("audit file should write");

    repair_invalid_draft_in_dir(
        &base_dir,
        &capability_snapshot,
        RepairInvalidDraftInputDto {
            draft_folder: "preset_broken-draft".into(),
        },
    )
    .expect("invalid draft repair should succeed");

    assert!(!broken_draft_dir.exists());
    assert!(audit_path.exists());

    let workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should load after repair");
    assert!(workspace.invalid_drafts.is_empty());

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_broken-draft", "Broken Draft Recreated"),
    )
    .expect("same preset id should be reusable after repair");
    let recreated_workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should reload with preserved publication history");
    assert_eq!(recreated_workspace.drafts.len(), 1);
    assert_eq!(
        recreated_workspace.drafts[0].publication_history.len(),
        audit_history.len()
    );
    assert_eq!(
        recreated_workspace.drafts[0].publication_history[0].action,
        "rejected"
    );
    assert_eq!(
        recreated_workspace.drafts[0].publication_history[0]
            .review_note
            .as_deref(),
        Some("이전 거절 이력 보존")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn repairing_a_valid_draft_is_rejected() {
    let base_dir = unique_test_root("repair-valid-draft");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");

    let error = repair_invalid_draft_in_dir(
        &base_dir,
        &capability_snapshot,
        RepairInvalidDraftInputDto {
            draft_folder: "preset_soft-glow-draft".into(),
        },
    )
    .expect_err("valid drafts should not be removable via invalid-draft repair");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("정상 draft"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn mismatched_folder_name_requires_manual_inspection_instead_of_auto_repair() {
    let base_dir = unique_test_root("repair-folder-mismatch");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    let original_draft_dir = resolve_draft_authoring_root(&base_dir).join("preset_soft-glow-draft");
    let draft_dir = resolve_draft_authoring_root(&base_dir).join("preset_folder-mismatch");
    fs::rename(&original_draft_dir, &draft_dir).expect("draft directory should be renamed");

    let workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should still load");
    assert_eq!(workspace.invalid_drafts.len(), 1);
    assert!(!workspace.invalid_drafts[0].can_repair);
    assert!(workspace.invalid_drafts[0].guidance.contains("수동 점검"));

    let error = repair_invalid_draft_in_dir(
        &base_dir,
        &capability_snapshot,
        RepairInvalidDraftInputDto {
            draft_folder: "preset_folder-mismatch".into(),
        },
    )
    .expect_err("folder mismatch should require manual inspection");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("수동 점검"));
    assert!(draft_dir.exists());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn malicious_preset_id_does_not_escape_publication_audit_root() {
    let base_dir = unique_test_root("malicious-preset-id");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");

    let draft_path = resolve_draft_authoring_root(&base_dir)
        .join("preset_soft-glow-draft")
        .join("draft.json");
    let mut persisted_draft: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&draft_path).expect("draft should exist"))
            .expect("draft json should deserialize");
    persisted_draft["presetId"] = serde_json::Value::String("../outside".into());
    fs::write(
        &draft_path,
        serde_json::to_vec_pretty(&persisted_draft).expect("draft should serialize"),
    )
    .expect("draft should write");

    let workspace = load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
        .expect("workspace should still load");
    assert_eq!(workspace.drafts.len(), 0);
    assert_eq!(workspace.invalid_drafts.len(), 1);
    assert!(!workspace.invalid_drafts[0].can_repair);

    let error = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect_err("malicious preset id should be rejected before audit path lookup");

    assert_eq!(error.code, "validation-error");
    assert!(error.message.contains("손상") || error.message.contains("다시 저장"));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn validated_draft_publishes_an_immutable_bundle_and_future_sessions_can_select_it() {
    let base_dir = unique_test_root("publish-success");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let active_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("active session should start");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: active_session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("active session should keep an older published preset");

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass before publish");
    let manifest_path = SessionPaths::new(&base_dir, &active_session.session_id).manifest_path;
    let manifest_before = fs::read_to_string(&manifest_path).expect("manifest should exist");

    let publish_result = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: Some("현재 세션 유지".into()),
        },
    )
    .expect("publish should succeed");

    let published_bundle_dir = catalog_root
        .join("preset_soft-glow-draft")
        .join("2026.03.26");
    assert!(published_bundle_dir.join("bundle.json").is_file());
    assert!(published_bundle_dir
        .join("preview")
        .join("soft-glow.jpg")
        .is_file());
    assert!(published_bundle_dir
        .join("xmp")
        .join("soft-glow.xmp")
        .is_file());

    match publish_result {
        PublishValidatedPresetResultDto::Published {
            draft,
            published_preset,
            audit_record,
            ..
        } => {
            assert_eq!(draft.lifecycle_state, "published");
            assert_eq!(published_preset.published_version, "2026.03.26");
            assert_eq!(audit_record.action, "published");
            assert_eq!(draft.publication_history.len(), 2);
            assert_eq!(draft.publication_history[0].action, "approved");
            assert_eq!(
                draft.publication_history[0].review_note.as_deref(),
                Some("현재 세션 유지")
            );
            assert_eq!(draft.publication_history[1].action, "published");
            assert_eq!(draft.publication_history[1].review_note, None);
        }
        PublishValidatedPresetResultDto::Rejected { .. } => {
            panic!("publish should not be rejected")
        }
    }

    let audit_path = base_dir
        .join("preset-authoring")
        .join("publication-audit")
        .join("preset_soft-glow-draft.json");
    let audit_bytes = fs::read_to_string(audit_path).expect("audit should be written");
    assert!(audit_bytes.contains("\"action\": \"approved\""));
    assert!(audit_bytes.contains("\"action\": \"published\""));
    assert!(audit_bytes.contains("\"reviewNote\": \"현재 세션 유지\""));

    let manifest_after = fs::read_to_string(&manifest_path).expect("manifest should still exist");
    assert_eq!(manifest_before, manifest_after);

    let future_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Lee".into(),
            phone_last_four: "1932".into(),
        },
    )
    .expect("future session should start");
    let future_catalog = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: future_session.session_id,
        },
    )
    .expect("future session catalog should load");
    assert!(future_catalog
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow-draft"
            && preset.published_version == "2026.03.26"));
    let catalog_state = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect("authoring should read the live catalog state after publish");
    assert_eq!(catalog_state.catalog_revision, 2);
    assert!(catalog_state
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow-draft"
            && preset.live_published_version == "2026.03.26"));
    assert!(catalog_state
        .presets
        .iter()
        .find(|preset| preset.preset_id == "preset_soft-glow-draft")
        .expect("published preset should be summarized")
        .version_history
        .iter()
        .any(
            |entry| entry.action_type == "published" && entry.to_published_version == "2026.03.26"
        ));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn default_catalog_bootstraps_first_run_booth_presets() {
    let base_dir = unique_test_root("default-catalog-bootstrap");

    ensure_default_preset_catalog_in_dir(&base_dir)
        .expect("default booth presets should be created for first run");

    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should start after default catalog bootstrap");
    let catalog = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("default catalog should load");

    assert_eq!(catalog.state, "ready");
    assert_eq!(catalog.presets.len(), 3);
    assert!(catalog
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow"));
    assert!(catalog
        .presets
        .iter()
        .all(|preset| preset.preview.asset_path.ends_with(".svg")));

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn default_catalog_bootstrap_does_not_overwrite_existing_published_presets() {
    let base_dir = unique_test_root("default-catalog-existing");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_custom", "2026.03.27", "Custom");

    ensure_default_preset_catalog_in_dir(&base_dir)
        .expect("existing published presets should keep their catalog");

    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should start against existing catalog");
    let catalog = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: session.session_id,
        },
    )
    .expect("existing catalog should stay loadable");

    assert_eq!(catalog.presets.len(), 1);
    assert_eq!(catalog.presets[0].preset_id, "preset_custom");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn default_catalog_bootstrap_upgrades_legacy_default_seed_bundles_for_runtime_rendering() {
    let base_dir = unique_test_root("default-catalog-upgrade-legacy");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let bundle_dir = catalog_root.join("preset_daylight").join("2026.03.27");
    fs::create_dir_all(&bundle_dir).expect("legacy bundle directory should exist");
    fs::write(bundle_dir.join("preview.svg"), "<svg/>").expect("preview should exist");
    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "published-preset-bundle/v1",
          "presetId": "preset_daylight",
          "displayName": "Daylight",
          "publishedVersion": "2026.03.27",
          "lifecycleStatus": "published",
          "boothStatus": "booth-safe",
          "preview": {
            "kind": "preview-tile",
            "assetPath": "preview.svg",
            "altText": "Daylight preview"
          }
        }))
        .expect("legacy bundle should serialize"),
    )
    .expect("legacy bundle should be writable");

    ensure_default_preset_catalog_in_dir(&base_dir)
        .expect("legacy default bundle should be upgraded for runtime rendering");

    let runtime_bundle = load_published_preset_runtime_bundle(&bundle_dir)
        .expect("upgraded default bundle should be runtime-loadable");

    assert_eq!(runtime_bundle.preset_id, "preset_daylight");
    assert_eq!(runtime_bundle.darktable_version, "5.4.1");
    assert!(runtime_bundle.xmp_template_path.is_file());

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rollback_switches_only_future_sessions_to_the_selected_approved_version() {
    let base_dir = unique_test_root("rollback-success");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.21", "Soft Glow");

    let active_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("active session should start against the current live catalog");
    let active_catalog_before = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: active_session.session_id.clone(),
        },
    )
    .expect("active session should load its pinned catalog");
    assert!(active_catalog_before
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow"
            && preset.published_version == "2026.03.21"));
    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: active_session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.21".into(),
        },
    )
    .expect("active session should bind the live version before rollback");
    let manifest_path = SessionPaths::new(&base_dir, &active_session.session_id).manifest_path;
    let manifest_before = fs::read_to_string(&manifest_path).expect("manifest should exist");

    let rollback_result = rollback_preset_catalog_in_dir(
        &base_dir,
        &capability_snapshot,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow".into(),
            target_published_version: "2026.03.20".into(),
            expected_catalog_revision: 1,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect("rollback should succeed");

    match rollback_result {
        RollbackPresetCatalogResultDto::RolledBack {
            catalog_revision,
            summary,
            audit_entry,
            ..
        } => {
            assert_eq!(catalog_revision, 2);
            assert_eq!(summary.live_published_version, "2026.03.20");
            assert_eq!(audit_entry.action_type, "rollback");
            assert_eq!(
                audit_entry.from_published_version.as_deref(),
                Some("2026.03.21")
            );
            assert_eq!(audit_entry.to_published_version, "2026.03.20");
        }
        RollbackPresetCatalogResultDto::Rejected { .. } => {
            panic!("rollback should not be rejected")
        }
    }

    assert!(catalog_root
        .join("preset_soft-glow")
        .join("2026.03.21")
        .join("bundle.json")
        .is_file());
    let manifest_after = fs::read_to_string(&manifest_path).expect("manifest should still exist");
    assert_eq!(manifest_before, manifest_after);
    let active_catalog_after = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: active_session.session_id.clone(),
        },
    )
    .expect("active session should keep its original snapshot");
    assert!(active_catalog_after
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow"
            && preset.published_version == "2026.03.21"));

    let future_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Lee".into(),
            phone_last_four: "1932".into(),
        },
    )
    .expect("future session should start after rollback");
    let future_catalog = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: future_session.session_id.clone(),
        },
    )
    .expect("future session should see the rolled back catalog");
    assert!(future_catalog
        .presets
        .iter()
        .any(|preset| preset.preset_id == "preset_soft-glow"
            && preset.published_version == "2026.03.20"));
    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: future_session.session_id,
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.20".into(),
        },
    )
    .expect("future session should bind the rolled back live version");

    let catalog_state: PresetCatalogStateResultDto =
        load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
            .expect("catalog state should stay readable after rollback");
    assert_eq!(catalog_state.catalog_revision, 2);
    assert!(
        catalog_state
            .presets
            .iter()
            .find(|preset| preset.preset_id == "preset_soft-glow")
            .expect("preset should be summarized")
            .version_history
            .iter()
            .any(|entry| entry.action_type == "rollback"
                && entry.to_published_version == "2026.03.20")
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rollback_rejection_keeps_catalog_state_and_active_session_unchanged() {
    let base_dir = unique_test_root("rollback-rejection");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.21", "Soft Glow");
    let active_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("active session should start");
    select_active_preset_in_dir(
        &base_dir,
        PresetSelectionInputDto {
            session_id: active_session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.03.21".into(),
        },
    )
    .expect("active session should bind the current live version");

    let catalog_state_before = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect("authoring should load catalog state before rejection");
    let catalog_tree_before = snapshot_tree(&catalog_root);
    let manifest_path = SessionPaths::new(&base_dir, &active_session.session_id).manifest_path;
    let manifest_before = fs::read_to_string(&manifest_path).expect("manifest should exist");

    let rejection = rollback_preset_catalog_in_dir(
        &base_dir,
        &capability_snapshot,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow".into(),
            target_published_version: "2026.03.21".into(),
            expected_catalog_revision: catalog_state_before.catalog_revision,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect("same-version rollback should return a structured rejection");

    match rejection {
        RollbackPresetCatalogResultDto::Rejected {
            reason_code,
            catalog_revision,
            summary,
            ..
        } => {
            assert_eq!(reason_code, "already-live");
            assert_eq!(catalog_revision, catalog_state_before.catalog_revision);
            assert_eq!(
                summary
                    .expect("current summary should be returned")
                    .live_published_version,
                "2026.03.21"
            );
        }
        RollbackPresetCatalogResultDto::RolledBack { .. } => {
            panic!("same-version rollback should not succeed")
        }
    }

    let stale_rejection = rollback_preset_catalog_in_dir(
        &base_dir,
        &capability_snapshot,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow".into(),
            target_published_version: "2026.03.20".into(),
            expected_catalog_revision: 999,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect("stale catalog revision should return a rejection");
    match stale_rejection {
        RollbackPresetCatalogResultDto::Rejected { reason_code, .. } => {
            assert_eq!(reason_code, "stale-catalog-revision");
        }
        RollbackPresetCatalogResultDto::RolledBack { .. } => {
            panic!("stale catalog revision should not succeed")
        }
    }

    let catalog_state_after = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect("authoring should still load catalog state after rejection");
    let manifest_after = fs::read_to_string(&manifest_path).expect("manifest should still exist");
    let catalog_tree_after = snapshot_tree(&catalog_root);
    assert_eq!(
        catalog_state_before.catalog_revision,
        catalog_state_after.catalog_revision
    );
    assert_eq!(catalog_tree_before, catalog_tree_after);
    assert_eq!(manifest_before, manifest_after);

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn malformed_catalog_state_does_not_silently_reset_the_live_catalog() {
    let base_dir = unique_test_root("rollback-malformed-catalog-state");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.21", "Soft Glow");

    let catalog_state_before = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect("authoring should initialize catalog state");
    assert_eq!(catalog_state_before.catalog_revision, 1);

    rollback_preset_catalog_in_dir(
        &base_dir,
        &capability_snapshot,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow".into(),
            target_published_version: "2026.03.20".into(),
            expected_catalog_revision: catalog_state_before.catalog_revision,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect("rollback should persist the older live version before corruption");

    let state_path = base_dir.join("preset-catalog").join("catalog-state.json");
    fs::write(&state_path, "{ not-valid-json").expect("corrupt state should write");

    let future_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Lee".into(),
            phone_last_four: "1932".into(),
        },
    )
    .expect("future session should still start before loading the catalog");
    let catalog_error = load_preset_catalog_in_dir(
        &base_dir,
        LoadPresetCatalogInputDto {
            session_id: future_session.session_id,
        },
    )
    .expect_err(
        "malformed catalog state should fail instead of silently resetting the live version",
    );
    assert_eq!(catalog_error.code, "session-persistence-failed");

    let authoring_error = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect_err("authoring should also fail while catalog state is malformed");
    assert_eq!(authoring_error.code, "session-persistence-failed");
    assert_eq!(
        fs::read_to_string(&state_path).expect("corrupt state should remain untouched"),
        "{ not-valid-json"
    );

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn malformed_catalog_audit_blocks_mutation_before_history_is_overwritten() {
    let base_dir = unique_test_root("rollback-malformed-catalog-audit");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.20", "Soft Glow");
    create_published_bundle(&catalog_root, "preset_soft-glow", "2026.03.21", "Soft Glow");

    let catalog_state_before = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect("authoring should initialize catalog state");
    rollback_preset_catalog_in_dir(
        &base_dir,
        &capability_snapshot,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow".into(),
            target_published_version: "2026.03.20".into(),
            expected_catalog_revision: catalog_state_before.catalog_revision,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect("first rollback should create catalog history");

    let state_path = base_dir.join("preset-catalog").join("catalog-state.json");
    let state_before = fs::read_to_string(&state_path).expect("catalog state should exist");
    let history_path = base_dir
        .join("preset-catalog")
        .join("catalog-audit")
        .join("preset_soft-glow.json");
    fs::write(&history_path, "{ not-valid-json").expect("corrupt audit should write");

    let error = rollback_preset_catalog_in_dir(
        &base_dir,
        &capability_snapshot,
        RollbackPresetCatalogInputDto {
            preset_id: "preset_soft-glow".into(),
            target_published_version: "2026.03.21".into(),
            expected_catalog_revision: 2,
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
        },
    )
    .expect_err("malformed audit should block a new mutation before history is replaced");
    assert_eq!(error.code, "session-persistence-failed");
    assert_eq!(
        fs::read_to_string(&state_path).expect("catalog state should remain unchanged"),
        state_before
    );
    assert_eq!(
        fs::read_to_string(&history_path).expect("corrupt audit should remain untouched"),
        "{ not-valid-json"
    );

    let authoring_error = load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
        .expect_err("authoring should fail while catalog audit is malformed");
    assert_eq!(authoring_error.code, "session-persistence-failed");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn publication_rejection_keeps_catalog_and_active_session_unchanged_and_records_audit_history() {
    let base_dir = unique_test_root("publish-rejection");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);
    let active_session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("active session should start");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    create_published_bundle(
        &catalog_root,
        "preset_soft-glow-draft",
        "2026.03.26",
        "Soft Glow Draft",
    );
    let catalog_before = snapshot_tree(&catalog_root);
    let manifest_path = SessionPaths::new(&base_dir, &active_session.session_id).manifest_path;
    let manifest_before = fs::read_to_string(&manifest_path).expect("manifest should exist");

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass before rejection");

    let rejection = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect("duplicate publish should return a rejection");

    match rejection {
        PublishValidatedPresetResultDto::Rejected {
            draft,
            reason_code,
            audit_record,
            ..
        } => {
            assert_eq!(draft.lifecycle_state, "validated");
            assert_eq!(reason_code, "duplicate-version");
            assert_eq!(audit_record.action, "rejected");
        }
        PublishValidatedPresetResultDto::Published { .. } => {
            panic!("duplicate publish should not succeed")
        }
    }

    let catalog_after = snapshot_tree(&catalog_root);
    let manifest_after = fs::read_to_string(&manifest_path).expect("manifest should still exist");
    assert_eq!(catalog_before, catalog_after);
    assert_eq!(manifest_before, manifest_after);

    let audit_path = base_dir
        .join("preset-authoring")
        .join("publication-audit")
        .join("preset_soft-glow-draft.json");
    let audit_bytes = fs::read_to_string(audit_path).expect("audit should be written");
    assert!(audit_bytes.contains("duplicate-version"));

    let stale_rejection = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: "2026-03-26T00:00:00.000Z".into(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.27".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect("stale validation should also return a rejection");
    match stale_rejection {
        PublishValidatedPresetResultDto::Rejected { reason_code, .. } => {
            assert_eq!(reason_code, "stale-validation");
        }
        PublishValidatedPresetResultDto::Published { .. } => {
            panic!("stale validation should not succeed")
        }
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn publication_rejects_metadata_mismatch_and_future_session_scope_violations() {
    let base_dir = unique_test_root("publish-metadata-rejections");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass before rejection checks");

    let metadata_rejection = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft Renamed".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect("metadata mismatch should return a rejection");
    match metadata_rejection {
        PublishValidatedPresetResultDto::Rejected { reason_code, .. } => {
            assert_eq!(reason_code, "metadata-mismatch");
        }
        PublishValidatedPresetResultDto::Published { .. } => {
            panic!("metadata mismatch should not succeed")
        }
    }

    let scope_rejection = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.27".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "active-session".into(),
            review_note: None,
        },
    )
    .expect("active-session scope should return a rejection");
    match scope_rejection {
        PublishValidatedPresetResultDto::Rejected { reason_code, .. } => {
            assert_eq!(reason_code, "future-session-only-violation");
        }
        PublishValidatedPresetResultDto::Published { .. } => {
            panic!("active-session scope should not succeed")
        }
    }

    let _ = fs::remove_dir_all(base_dir);
}

#[cfg(windows)]
#[test]
fn publication_rejects_workspace_symlink_escapes_without_creating_a_bundle() {
    let base_dir = unique_test_root("publish-path-escape");
    let capability_snapshot = capability_snapshot_for_profile("authoring-enabled", true);

    create_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        sample_draft_payload("preset_soft-glow-draft", "Soft Glow Draft"),
    )
    .expect("draft creation should succeed");
    scaffold_valid_draft_assets(&base_dir, "preset_soft-glow-draft");
    let validation_result = validate_draft_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        ValidateDraftPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
        },
    )
    .expect("validation should pass before path escape rejection");

    let outside_preview = resolve_draft_authoring_root(&base_dir).join("outside-preview.jpg");
    fs::write(&outside_preview, "outside preview").expect("outside preview should write");

    let draft_root = resolve_draft_authoring_root(&base_dir).join("preset_soft-glow-draft");
    let linked_preview = draft_root.join("previews").join("soft-glow.jpg");
    fs::remove_file(&linked_preview).expect("existing preview should be removable");
    symlink_file(&outside_preview, &linked_preview).expect("preview symlink should be created");

    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let rejection = publish_validated_preset_in_dir(
        &base_dir,
        &capability_snapshot,
        PublishValidatedPresetInputDto {
            preset_id: "preset_soft-glow-draft".into(),
            draft_version: validation_result.draft.draft_version,
            validation_checked_at: validation_result.report.checked_at.clone(),
            expected_display_name: "Soft Glow Draft".into(),
            published_version: "2026.03.26".into(),
            actor_id: "manager-kim".into(),
            actor_label: "Kim Manager".into(),
            scope: "future-sessions-only".into(),
            review_note: None,
        },
    )
    .expect("path escape should return a rejection");

    match rejection {
        PublishValidatedPresetResultDto::Rejected { reason_code, .. } => {
            assert_eq!(reason_code, "path-escape");
        }
        PublishValidatedPresetResultDto::Published { .. } => {
            panic!("path escape should not succeed")
        }
    }

    assert!(!catalog_root
        .join("preset_soft-glow-draft")
        .join("2026.03.26")
        .exists());

    let _ = fs::remove_dir_all(base_dir);
}

fn sample_draft_payload(preset_id: &str, display_name: &str) -> DraftPresetEditPayloadDto {
    DraftPresetEditPayloadDto {
        preset_id: preset_id.into(),
        display_name: display_name.into(),
        lifecycle_state: "draft".into(),
        darktable_version: "5.4.1".into(),
        darktable_project_path: "darktable/soft-glow.dtpreset".into(),
        xmp_template_path: "xmp/soft-glow.xmp".into(),
        preview_profile: render_profile("preview-standard", "Preview Standard"),
        final_profile: render_profile("final-standard", "Final Standard"),
        noise_policy: DraftNoisePolicyDto {
            policy_id: "balanced-noise".into(),
            display_name: "Balanced Noise".into(),
            reduction_mode: "balanced".into(),
        },
        preview: DraftPresetPreviewReferenceDto {
            asset_path: "previews/soft-glow.jpg".into(),
            alt_text: "Soft Glow draft portrait".into(),
        },
        sample_cut: DraftPresetPreviewReferenceDto {
            asset_path: "samples/soft-glow-cut.jpg".into(),
            alt_text: "Soft Glow sample cut".into(),
        },
        description: Some("부드러운 피부톤 baseline".into()),
        notes: Some("승인 전 내부 검토용".into()),
    }
}

fn render_profile(profile_id: &str, display_name: &str) -> DraftRenderProfileDto {
    DraftRenderProfileDto {
        profile_id: profile_id.into(),
        display_name: display_name.into(),
        output_color_space: "sRGB".into(),
    }
}

fn scaffold_valid_draft_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        "<darktable><history><item operation=\"exposure\"></item></history></darktable>",
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
}

fn scaffold_invalid_render_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        "incompatible xmp payload",
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
}

fn scaffold_standard_darktable_sidecar_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description xmlns:darktable="http://darktable.sf.net/">
      <darktable:history>
        <rdf:Seq>
          <rdf:li darktable:operation="exposure" />
        </rdf:Seq>
      </darktable:history>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#,
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
}

fn scaffold_xmp_without_history_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        "<darktable></darktable>",
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
}

fn scaffold_marker_only_xmp_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        "<darktable>history</darktable>",
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
}

fn scaffold_commented_marker_xmp_assets(base_dir: &Path, preset_id: &str) {
    let draft_root = resolve_draft_authoring_root(base_dir).join(preset_id);

    fs::create_dir_all(draft_root.join("darktable")).expect("darktable directory should exist");
    fs::create_dir_all(draft_root.join("xmp")).expect("xmp directory should exist");
    fs::create_dir_all(draft_root.join("previews")).expect("preview directory should exist");
    fs::create_dir_all(draft_root.join("samples")).expect("sample directory should exist");
    fs::write(draft_root.join("darktable/soft-glow.dtpreset"), "project")
        .expect("project should write");
    fs::write(
        draft_root.join("xmp/soft-glow.xmp"),
        "<darktable><history><!-- <item num=\"1\"></item> --></history></darktable>",
    )
    .expect("xmp should write");
    fs::write(draft_root.join("previews/soft-glow.jpg"), "preview").expect("preview should write");
    fs::write(draft_root.join("samples/soft-glow-cut.jpg"), "sample").expect("sample should write");
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

fn snapshot_tree(root: &Path) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    if !root.exists() {
        return entries;
    }

    collect_tree_entries(root, root, &mut entries);
    entries.sort();
    entries
}

fn collect_tree_entries(root: &Path, current: &Path, entries: &mut Vec<(String, String)>) {
    let read_dir = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in read_dir {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };

        if path.is_dir() {
            collect_tree_entries(root, &path, entries);
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .expect("entry should stay inside root")
            .to_string_lossy()
            .replace('\\', "/");
        let contents = fs::read_to_string(&path).unwrap_or_default();

        entries.push((relative_path, contents));
    }
}
