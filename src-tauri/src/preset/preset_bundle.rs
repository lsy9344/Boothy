use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::contracts::dto::{
    is_non_blank, is_valid_darktable_version, is_valid_preset_id, is_valid_published_version,
    PresetPreviewAssetDto, PublishedPresetSummaryDto,
};

const PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION: &str = "published-preset-bundle/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPresetRenderProfile {
    pub profile_id: String,
    pub display_name: String,
    pub output_color_space: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPresetRuntimeBundle {
    pub preset_id: String,
    pub display_name: String,
    pub published_version: String,
    pub darktable_version: String,
    pub xmp_template_path: PathBuf,
    pub preview_profile: PublishedPresetRenderProfile,
    pub final_profile: PublishedPresetRenderProfile,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PublishedPresetBundle {
    schema_version: String,
    preset_id: String,
    display_name: String,
    published_version: String,
    lifecycle_status: String,
    booth_status: String,
    preview: BundlePreviewAsset,
    #[serde(default)]
    darktable_version: Option<String>,
    #[serde(default)]
    darktable_project_path: Option<String>,
    #[serde(default)]
    xmp_template_path: Option<String>,
    #[serde(default)]
    preview_profile: Option<BundleRenderProfile>,
    #[serde(default)]
    final_profile: Option<BundleRenderProfile>,
    #[serde(default)]
    sample_cut: Option<BundlePreviewAsset>,
    #[serde(default)]
    source_draft_version: Option<u32>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    published_by: Option<PublishedByMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct BundlePreviewAsset {
    kind: String,
    asset_path: String,
    alt_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct BundleRenderProfile {
    profile_id: String,
    display_name: String,
    output_color_space: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(untagged)]
enum PublishedByMetadata {
    Label(String),
    Actor(PublishedByActorMetadata),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct PublishedByActorMetadata {
    actor_id: String,
    actor_label: String,
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

pub fn load_published_preset_runtime_bundle(
    bundle_dir: &Path,
) -> Option<PublishedPresetRuntimeBundle> {
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

    if !is_valid_preset_id(&bundle.preset_id)
        || !is_valid_published_version(&bundle.published_version)
        || !is_non_blank(&bundle.display_name)
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

    let darktable_version = bundle.darktable_version?;
    if !is_valid_darktable_version(&darktable_version) {
        return None;
    }

    let xmp_template_path =
        resolve_bundle_asset_path(bundle_dir, bundle.xmp_template_path?.as_str())?;
    let preview_profile = resolve_render_profile(
        bundle.preview_profile,
        &bundle.preset_id,
        &bundle.display_name,
        "preview",
    )?;
    let final_profile = resolve_render_profile(
        bundle.final_profile,
        &bundle.preset_id,
        &bundle.display_name,
        "final",
    )?;

    Some(PublishedPresetRuntimeBundle {
        preset_id: bundle.preset_id,
        display_name: bundle.display_name,
        published_version: bundle.published_version,
        darktable_version,
        xmp_template_path,
        preview_profile,
        final_profile,
    })
}

fn resolve_render_profile(
    profile: Option<BundleRenderProfile>,
    preset_id: &str,
    display_name: &str,
    profile_kind: &str,
) -> Option<PublishedPresetRenderProfile> {
    match profile {
        Some(profile) => normalize_render_profile(profile),
        None => Some(PublishedPresetRenderProfile {
            profile_id: format!("{preset_id}-{profile_kind}"),
            display_name: format!(
                "{} {}",
                display_name,
                match profile_kind {
                    "preview" => "Preview",
                    "final" => "Final",
                    _ => "Render",
                }
            ),
            output_color_space: "sRGB".into(),
        }),
    }
}

fn resolve_preview_path(bundle_dir: &Path, asset_path: &str) -> Option<PathBuf> {
    resolve_bundle_asset_path(bundle_dir, asset_path)
}

fn resolve_bundle_asset_path(bundle_dir: &Path, asset_path: &str) -> Option<PathBuf> {
    let bundle_root = fs::canonicalize(bundle_dir).ok()?;
    let preview_path = fs::canonicalize(bundle_dir.join(asset_path)).ok()?;

    if !preview_path.is_file() || !preview_path.starts_with(&bundle_root) {
        return None;
    }

    Some(preview_path)
}

fn normalize_render_profile(profile: BundleRenderProfile) -> Option<PublishedPresetRenderProfile> {
    if !is_non_blank(&profile.profile_id)
        || !is_non_blank(&profile.display_name)
        || !is_non_blank(&profile.output_color_space)
    {
        return None;
    }

    Some(PublishedPresetRenderProfile {
        profile_id: profile.profile_id,
        display_name: profile.display_name,
        output_color_space: profile.output_color_space,
    })
}

#[cfg(test)]
mod tests {
    use super::PublishedPresetBundle;

    #[test]
    fn published_bundle_contract_rejects_unknown_top_level_fields() {
        let json = r#"{
          "schemaVersion": "published-preset-bundle/v1",
          "presetId": "preset_soft-glow",
          "displayName": "Soft Glow",
          "publishedVersion": "2026.04.10",
          "lifecycleStatus": "published",
          "boothStatus": "booth-safe",
          "darktableVersion": "5.4.1",
          "xmpTemplatePath": "xmp/template.xmp",
          "previewProfile": {
            "profileId": "soft-glow-preview",
            "displayName": "Soft Glow Preview",
            "outputColorSpace": "sRGB"
          },
          "finalProfile": {
            "profileId": "soft-glow-final",
            "displayName": "Soft Glow Final",
            "outputColorSpace": "sRGB"
          },
          "preview": {
            "kind": "preview-tile",
            "assetPath": "preview.jpg",
            "altText": "Soft Glow sample portrait"
          },
          "unexpectedMetadata": true
        }"#;

        let result = serde_json::from_str::<PublishedPresetBundle>(json);

        assert!(result.is_err(), "unknown bundle fields should be rejected");
    }
}
