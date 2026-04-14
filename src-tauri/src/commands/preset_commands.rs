use tauri::Manager;

use crate::{
    capture::helper_supervisor::try_ensure_helper_running,
    commands::runtime_commands::resolve_runtime_capability_snapshot,
    contracts::dto::{
        AuthoringWorkspaceResultDto, DraftPresetEditPayloadDto, DraftPresetSummaryDto,
        HostErrorEnvelope, LoadPresetCatalogInputDto, PresetCatalogResultDto,
        PresetCatalogStateResultDto, PresetSelectionInputDto, PresetSelectionResultDto,
        PublishValidatedPresetInputDto, PublishValidatedPresetResultDto,
        RepairInvalidDraftInputDto, RollbackPresetCatalogInputDto, RollbackPresetCatalogResultDto,
        ValidateDraftPresetInputDto, ValidateDraftPresetResultDto,
    },
    preset::{
        authoring_pipeline::{
            create_draft_preset_in_dir, load_authoring_workspace_in_dir,
            publish_validated_preset_in_dir, repair_invalid_draft_in_dir, save_draft_preset_in_dir,
            validate_draft_preset_in_dir,
        },
        preset_catalog::load_preset_catalog_in_dir,
        preset_catalog_state::{
            load_preset_catalog_state_in_dir, preview_rollback_preset_catalog_in_dir,
        },
    },
    render::dedicated_renderer::schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir,
    session::session_repository::{resolve_app_session_base_dir, select_active_preset_in_dir},
};

#[tauri::command]
pub fn load_preset_catalog(
    app: tauri::AppHandle,
    input: LoadPresetCatalogInputDto,
) -> Result<PresetCatalogResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);

    load_preset_catalog_in_dir(&base_dir, input)
}

#[tauri::command]
pub fn select_active_preset(
    app: tauri::AppHandle,
    input: PresetSelectionInputDto,
) -> Result<PresetSelectionResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let session_id = input.session_id.clone();
    let result = select_active_preset_in_dir(&base_dir, input)?;
    schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
        Some(&app),
        &base_dir,
        &session_id,
        &result.active_preset.preset_id,
        &result.active_preset.published_version,
    );
    try_ensure_helper_running(&base_dir, &session_id);

    Ok(result)
}

#[tauri::command]
pub fn load_authoring_workspace(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<AuthoringWorkspaceResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn create_draft_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: DraftPresetEditPayloadDto,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    create_draft_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn save_draft_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: DraftPresetEditPayloadDto,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    save_draft_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn validate_draft_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: ValidateDraftPresetInputDto,
) -> Result<ValidateDraftPresetResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    validate_draft_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn repair_invalid_draft(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: RepairInvalidDraftInputDto,
) -> Result<(), HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    repair_invalid_draft_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn publish_validated_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: PublishValidatedPresetInputDto,
) -> Result<PublishValidatedPresetResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();

    publish_validated_preset_at_base_dir(&base_dir, window.label(), &capability_snapshot, input)
}

#[tauri::command]
pub fn load_preset_catalog_state(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<PresetCatalogStateResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn rollback_preset_catalog(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: RollbackPresetCatalogInputDto,
) -> Result<RollbackPresetCatalogResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    preview_rollback_preset_catalog_in_dir(&base_dir, &capability_snapshot, input)
}

fn publish_validated_preset_at_base_dir(
    base_dir: &std::path::Path,
    window_label: &str,
    capability_snapshot: &crate::contracts::dto::CapabilitySnapshotDto,
    input: PublishValidatedPresetInputDto,
) -> Result<PublishValidatedPresetResultDto, HostErrorEnvelope> {
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window_label)?;
    publish_validated_preset_in_dir(base_dir, capability_snapshot, input)
}

#[cfg(test)]
mod tests {
    use super::publish_validated_preset_at_base_dir;
    use crate::contracts::dto::{
        CapabilitySnapshotDto, DraftNoisePolicyDto, DraftPresetEditPayloadDto,
        DraftPresetPreviewReferenceDto, DraftRenderProfileDto, PublishValidatedPresetInputDto,
        PublishValidatedPresetResultDto, ValidateDraftPresetInputDto,
    };
    use crate::preset::authoring_pipeline::{
        create_draft_preset_in_dir, resolve_draft_authoring_root, validate_draft_preset_in_dir,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn publish_dispatch_uses_the_real_publish_pipeline() {
        let base_dir = unique_test_root("publish-command-dispatch");
        let capability_snapshot = authoring_capability_snapshot();

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

        let result = publish_validated_preset_at_base_dir(
            &base_dir,
            "authoring-window",
            &capability_snapshot,
            PublishValidatedPresetInputDto {
                preset_id: "preset_soft-glow-draft".into(),
                draft_version: validation_result.draft.draft_version,
                validation_checked_at: validation_result.report.checked_at,
                expected_display_name: "Soft Glow Draft".into(),
                published_version: "2026.03.26".into(),
                actor_id: "manager-kim".into(),
                actor_label: "김 매니저".into(),
                scope: "future-sessions-only".into(),
                review_note: Some("Ready for publish".into()),
            },
        )
        .expect("publish command dispatch should succeed");

        match result {
            PublishValidatedPresetResultDto::Published { draft, .. } => {
                assert_eq!(draft.lifecycle_state, "published");
            }
            PublishValidatedPresetResultDto::Rejected { reason_code, .. } => {
                panic!("publish command dispatch unexpectedly rejected with {reason_code}")
            }
        }

        let _ = fs::remove_dir_all(base_dir);
    }

    fn authoring_capability_snapshot() -> CapabilitySnapshotDto {
        CapabilitySnapshotDto {
            is_admin_authenticated: true,
            allowed_surfaces: vec!["authoring".into()],
        }
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("boothy-{label}-{unique}"))
    }

    fn sample_draft_payload(preset_id: &str, display_name: &str) -> DraftPresetEditPayloadDto {
        DraftPresetEditPayloadDto {
            preset_id: preset_id.into(),
            display_name: display_name.into(),
            lifecycle_state: "draft".into(),
            darktable_version: "5.4.1".into(),
            darktable_project_path: Some("darktable/soft-glow.dtpreset".into()),
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
                alt_text: "Soft Glow preview".into(),
            },
            sample_cut: DraftPresetPreviewReferenceDto {
                asset_path: "samples/soft-glow-cut.jpg".into(),
                alt_text: "Soft Glow sample cut".into(),
            },
            description: None,
            notes: None,
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
        fs::create_dir_all(draft_root.join("xmp")).expect("xmp dir should exist");
        fs::create_dir_all(draft_root.join("previews")).expect("preview dir should exist");
        fs::create_dir_all(draft_root.join("samples")).expect("sample dir should exist");
        fs::create_dir_all(draft_root.join("darktable")).expect("darktable dir should exist");

        fs::write(
            draft_root.join("xmp").join("soft-glow.xmp"),
            r#"
            <x:xmpmeta xmlns:x="adobe:ns:meta/">
              <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description xmlns:darktable="http://darktable.sf.net/">
                  <darktable:history>
                    <rdf:Seq>
                      <rdf:li>exposure</rdf:li>
                    </rdf:Seq>
                  </darktable:history>
                </rdf:Description>
              </rdf:RDF>
            </x:xmpmeta>
            "#,
        )
        .expect("xmp should write");
        fs::write(draft_root.join("previews").join("soft-glow.jpg"), "preview")
            .expect("preview should write");
        fs::write(
            draft_root.join("samples").join("soft-glow-cut.jpg"),
            "sample",
        )
        .expect("sample cut should write");
        fs::write(
            draft_root.join("darktable").join("soft-glow.dtpreset"),
            "darktable project",
        )
        .expect("darktable project should write");
    }
}
