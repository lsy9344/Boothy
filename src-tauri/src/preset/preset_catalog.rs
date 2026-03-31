use std::path::{Path, PathBuf};

use crate::{
    contracts::dto::{
        validate_preset_selection_input, validate_session_id, HostErrorEnvelope,
        LoadPresetCatalogInputDto, PresetCatalogResultDto, PublishedPresetSummaryDto,
    },
    session::{
        session_manifest::{ActivePresetBinding, SessionManifest},
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
    },
};

use super::{
    preset_bundle::{
        load_published_preset_runtime_bundle, load_published_preset_summary,
        PublishedPresetRuntimeBundle,
    },
    preset_catalog_state::capture_live_catalog_snapshot,
};

pub fn resolve_published_preset_catalog_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("preset-catalog").join("published")
}

pub fn load_preset_catalog_in_dir(
    base_dir: &Path,
    input: LoadPresetCatalogInputDto,
) -> Result<PresetCatalogResultDto, HostErrorEnvelope> {
    validate_session_id(&input.session_id)?;
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let manifest = ensure_catalog_snapshot_pinned(base_dir, &paths.manifest_path)?;

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);

    if !catalog_root.exists() {
        return Ok(PresetCatalogResultDto {
            session_id: input.session_id,
            state: "empty".into(),
            presets: Vec::new(),
        });
    }

    let presets = load_selectable_published_presets_for_snapshot(
        &catalog_root,
        manifest.catalog_snapshot.as_deref().unwrap_or(&[]),
    );

    Ok(PresetCatalogResultDto {
        session_id: input.session_id,
        state: if presets.is_empty() { "empty" } else { "ready" }.into(),
        presets,
    })
}

pub fn find_published_preset_summary(
    catalog_root: &Path,
    preset_id: &str,
    published_version: &str,
) -> Option<PublishedPresetSummaryDto> {
    if validate_preset_selection_input(preset_id, published_version).is_err() {
        return None;
    }

    load_published_preset_summary(&catalog_root.join(preset_id).join(published_version))
}

pub fn find_published_preset_runtime_bundle(
    catalog_root: &Path,
    preset_id: &str,
    published_version: &str,
) -> Option<PublishedPresetRuntimeBundle> {
    if validate_preset_selection_input(preset_id, published_version).is_err() {
        return None;
    }

    load_published_preset_runtime_bundle(&catalog_root.join(preset_id).join(published_version))
}

pub fn find_selectable_published_preset_summary(
    catalog_root: &Path,
    preset_id: &str,
    published_version: &str,
    catalog_snapshot: &[ActivePresetBinding],
) -> Result<Option<PublishedPresetSummaryDto>, HostErrorEnvelope> {
    validate_preset_selection_input(preset_id, published_version)?;

    Ok(find_selectable_published_preset_summary_in_snapshot(
        catalog_root,
        catalog_snapshot,
        preset_id,
        published_version,
    ))
}

pub fn find_selectable_published_preset_summary_in_snapshot(
    catalog_root: &Path,
    catalog_snapshot: &[ActivePresetBinding],
    preset_id: &str,
    published_version: &str,
) -> Option<PublishedPresetSummaryDto> {
    if !catalog_snapshot.iter().any(|binding| {
        binding.preset_id == preset_id && binding.published_version == published_version
    }) {
        return None;
    }

    load_published_preset_summary(&catalog_root.join(preset_id).join(published_version))
}

pub fn load_selectable_published_presets_for_snapshot(
    catalog_root: &Path,
    catalog_snapshot: &[ActivePresetBinding],
) -> Vec<PublishedPresetSummaryDto> {
    let mut presets = catalog_snapshot
        .iter()
        .filter_map(|binding| {
            load_published_preset_summary(
                &catalog_root
                    .join(&binding.preset_id)
                    .join(&binding.published_version),
            )
        })
        .collect::<Vec<_>>();
    presets.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then_with(|| left.preset_id.cmp(&right.preset_id))
    });
    presets.truncate(6);

    presets
}

fn ensure_catalog_snapshot_pinned(
    base_dir: &Path,
    manifest_path: &Path,
) -> Result<SessionManifest, HostErrorEnvelope> {
    if !manifest_path.is_file() {
        return Err(HostErrorEnvelope::session_not_found(
            "진행 중인 세션을 찾지 못했어요. 처음 화면에서 다시 시작해 주세요.",
        ));
    }

    let mut manifest = read_session_manifest(manifest_path)?;
    if manifest.catalog_revision.is_some() && manifest.catalog_snapshot.is_some() {
        return Ok(manifest);
    }

    let (catalog_revision, catalog_snapshot) = capture_live_catalog_snapshot(base_dir)?;
    manifest.catalog_revision = Some(catalog_revision);
    manifest.catalog_snapshot = Some(catalog_snapshot);
    write_session_manifest(manifest_path, &manifest)?;

    Ok(manifest)
}
