use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::contracts::dto::{
    is_non_blank, is_valid_preset_id, is_valid_published_version, PresetPreviewAssetDto,
    PublishedPresetSummaryDto,
};

const PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION: &str = "published-preset-bundle/v1";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishedPresetBundle {
    schema_version: String,
    preset_id: String,
    display_name: String,
    published_version: String,
    lifecycle_status: String,
    booth_status: String,
    preview: BundlePreviewAsset,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundlePreviewAsset {
    kind: String,
    asset_path: String,
    alt_text: String,
}

pub fn load_published_preset_summary(bundle_dir: &Path) -> Option<PublishedPresetSummaryDto> {
    let bundle_path = bundle_dir.join("bundle.json");
    let bundle_bytes = fs::read_to_string(bundle_path).ok()?;
    let bundle: PublishedPresetBundle = serde_json::from_str(&bundle_bytes).ok()?;
    let preset_dir_name = bundle_dir.parent()?.file_name()?.to_string_lossy();

    if bundle.schema_version != PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION {
        return None;
    }

    if bundle.lifecycle_status != "published" || bundle.booth_status != "booth-safe" {
        return None;
    }

    if bundle.preview.kind != "preview-tile" && bundle.preview.kind != "sample-cut" {
        return None;
    }

    if !is_valid_preset_id(&bundle.preset_id)
        || !is_valid_published_version(&bundle.published_version)
        || !is_non_blank(&bundle.display_name)
        || !is_non_blank(&bundle.preview.alt_text)
    {
        return None;
    }

    let version_dir_name = bundle_dir.file_name()?.to_string_lossy();

    if version_dir_name != bundle.published_version {
        return None;
    }

    if preset_dir_name != bundle.preset_id {
        return None;
    }

    let preview_path = resolve_preview_path(bundle_dir, &bundle.preview.asset_path)?;

    Some(PublishedPresetSummaryDto {
        preset_id: bundle.preset_id,
        display_name: bundle.display_name,
        published_version: bundle.published_version,
        booth_status: bundle.booth_status,
        preview: PresetPreviewAssetDto {
            kind: bundle.preview.kind,
            asset_path: preview_path.to_string_lossy().replace('\\', "/"),
            alt_text: bundle.preview.alt_text,
        },
    })
}

fn resolve_preview_path(bundle_dir: &Path, asset_path: &str) -> Option<PathBuf> {
    let bundle_root = fs::canonicalize(bundle_dir).ok()?;
    let preview_path = fs::canonicalize(bundle_dir.join(asset_path)).ok()?;

    if !preview_path.is_file() || !preview_path.starts_with(&bundle_root) {
        return None;
    }

    Some(preview_path)
}
