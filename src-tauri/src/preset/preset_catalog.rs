use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    contracts::dto::{
        validate_preset_selection_input, validate_session_id, HostErrorEnvelope,
        LoadPresetCatalogInputDto, PresetCatalogResultDto, PublishedPresetSummaryDto,
    },
    session::session_paths::SessionPaths,
};

use super::preset_bundle::load_published_preset_summary;

pub fn resolve_published_preset_catalog_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("preset-catalog").join("published")
}

pub fn load_preset_catalog_in_dir(
    base_dir: &Path,
    input: LoadPresetCatalogInputDto,
) -> Result<PresetCatalogResultDto, HostErrorEnvelope> {
    validate_session_id(&input.session_id)?;
    ensure_session_exists(base_dir, &input.session_id)?;

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);

    if !catalog_root.exists() {
        return Ok(PresetCatalogResultDto {
            session_id: input.session_id,
            state: "empty".into(),
            presets: Vec::new(),
        });
    }

    let presets = load_selectable_published_presets(&catalog_root)?;

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

pub fn find_selectable_published_preset_summary(
    catalog_root: &Path,
    preset_id: &str,
    published_version: &str,
) -> Result<Option<PublishedPresetSummaryDto>, HostErrorEnvelope> {
    validate_preset_selection_input(preset_id, published_version)?;

    Ok(load_selectable_published_presets(catalog_root)?
        .into_iter()
        .find(|summary| {
            summary.preset_id == preset_id && summary.published_version == published_version
        }))
}

fn ensure_session_exists(base_dir: &Path, session_id: &str) -> Result<(), HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;

    if paths.manifest_path.is_file() {
        Ok(())
    } else {
        Err(HostErrorEnvelope::session_not_found(
            "진행 중인 세션을 찾지 못했어요. 처음 화면에서 다시 시작해 주세요.",
        ))
    }
}

pub fn load_selectable_published_presets(
    catalog_root: &Path,
) -> Result<Vec<PublishedPresetSummaryDto>, HostErrorEnvelope> {
    let mut presets = collect_published_presets(catalog_root)?;
    presets.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then_with(|| left.preset_id.cmp(&right.preset_id))
    });
    presets.truncate(6);

    Ok(presets)
}

fn collect_published_presets(
    catalog_root: &Path,
) -> Result<Vec<PublishedPresetSummaryDto>, HostErrorEnvelope> {
    let mut presets_by_id: HashMap<String, PublishedPresetSummaryDto> = HashMap::new();
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
                match presets_by_id.get_mut(&summary.preset_id) {
                    Some(existing) if summary.published_version > existing.published_version => {
                        *existing = summary;
                    }
                    None => {
                        presets_by_id.insert(summary.preset_id.clone(), summary);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(presets_by_id.into_values().collect())
}
