use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::Value;

use crate::contracts::dto::{
    is_non_blank, is_valid_darktable_version, is_valid_preset_id, is_valid_published_version,
    PresetPreviewAssetDto, PublishedPresetSummaryDto,
};

const PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION_V1: &str = "published-preset-bundle/v1";
const PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION_V2: &str = "published-preset-bundle/v2";
const CANONICAL_PRESET_RECIPE_SCHEMA_VERSION: &str = "canonical-preset-recipe/v1";
const DARKTABLE_PRESET_ADAPTER_SCHEMA_VERSION: &str = "darktable-preset-adapter/v1";
const PINNED_DARKTABLE_VERSION: &str = "5.4.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPresetRenderProfile {
    pub profile_id: String,
    pub display_name: String,
    pub output_color_space: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPresetNoisePolicy {
    pub policy_id: String,
    pub display_name: String,
    pub reduction_mode: String,
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
    pub noise_policy: PublishedPresetNoisePolicy,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PublishedPresetBundleV1 {
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
#[allow(dead_code)]
struct PublishedPresetBundleV2 {
    schema_version: String,
    preset_id: String,
    display_name: String,
    published_version: String,
    lifecycle_status: String,
    booth_status: String,
    canonical_recipe: CanonicalPresetRecipe,
    darktable_adapter: DarktablePresetAdapter,
    preview: BundlePreviewAsset,
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
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct BundleNoisePolicy {
    policy_id: String,
    display_name: String,
    reduction_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct CanonicalPresetRecipe {
    schema_version: String,
    preset_id: String,
    published_version: String,
    display_name: String,
    booth_status: String,
    preview_intent: BundleRenderProfile,
    final_intent: BundleRenderProfile,
    noise_policy: BundleNoisePolicy,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct DarktablePresetAdapter {
    schema_version: String,
    darktable_version: String,
    xmp_template_path: String,
    #[serde(default)]
    darktable_project_path: Option<String>,
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

enum ParsedPublishedPresetBundle {
    V1(PublishedPresetBundleV1),
    V2(PublishedPresetBundleV2),
}

pub fn load_published_preset_summary(bundle_dir: &Path) -> Option<PublishedPresetSummaryDto> {
    let bundle = load_bundle_definition(bundle_dir)?;
    let preset_dir_name = bundle_dir.parent()?.file_name()?.to_string_lossy();

    if !bundle.is_summary_compatible() {
        return None;
    }

    if preset_dir_name != bundle.preset_id() {
        return None;
    }

    let version_dir_name = bundle_dir.file_name()?.to_string_lossy();
    if version_dir_name != bundle.published_version() {
        return None;
    }

    let preview = bundle.preview();
    let preview_path = resolve_preview_path(bundle_dir, &preview.asset_path)?;

    Some(PublishedPresetSummaryDto {
        preset_id: bundle.preset_id().to_string(),
        display_name: bundle.display_name().to_string(),
        published_version: bundle.published_version().to_string(),
        booth_status: bundle.booth_status().to_string(),
        preview: PresetPreviewAssetDto {
            kind: preview.kind.to_string(),
            asset_path: preview_path.to_string_lossy().replace('\\', "/"),
            alt_text: preview.alt_text.to_string(),
        },
    })
}

pub fn load_published_preset_runtime_bundle(
    bundle_dir: &Path,
) -> Option<PublishedPresetRuntimeBundle> {
    let bundle = load_bundle_definition(bundle_dir)?;
    let preset_dir_name = bundle_dir.parent()?.file_name()?.to_string_lossy();

    if !bundle.is_runtime_compatible() {
        return None;
    }

    if preset_dir_name != bundle.preset_id() {
        return None;
    }

    let version_dir_name = bundle_dir.file_name()?.to_string_lossy();
    if version_dir_name != bundle.published_version() {
        return None;
    }

    let darktable_version = bundle.darktable_version()?.to_string();
    let xmp_template_path = resolve_bundle_asset_path(bundle_dir, bundle.xmp_template_path()?)?;
    let preview_profile = bundle.preview_profile()?;
    let final_profile = bundle.final_profile()?;
    let noise_policy = bundle.noise_policy()?;

    Some(PublishedPresetRuntimeBundle {
        preset_id: bundle.preset_id().to_string(),
        display_name: bundle.display_name().to_string(),
        published_version: bundle.published_version().to_string(),
        darktable_version,
        xmp_template_path,
        preview_profile,
        final_profile,
        noise_policy,
    })
}

impl ParsedPublishedPresetBundle {
    fn is_summary_compatible(&self) -> bool {
        if !self.has_valid_common_identity() {
            return false;
        }

        let preview = self.preview();
        (preview.kind == "preview-tile" || preview.kind == "sample-cut")
            && is_non_blank(&preview.alt_text)
    }

    fn is_runtime_compatible(&self) -> bool {
        if !self.has_valid_common_identity() {
            return false;
        }

        let Some(darktable_version) = self.darktable_version() else {
            return false;
        };
        if !is_valid_darktable_version(darktable_version)
            || darktable_version != PINNED_DARKTABLE_VERSION
        {
            return false;
        }

        self.xmp_template_path().is_some()
            && self.preview_profile().is_some()
            && self.final_profile().is_some()
            && self.noise_policy().is_some()
    }

    fn has_valid_common_identity(&self) -> bool {
        if self.lifecycle_status() != "published" || self.booth_status() != "booth-safe" {
            return false;
        }

        if !is_valid_preset_id(self.preset_id())
            || !is_valid_published_version(self.published_version())
            || !is_non_blank(self.display_name())
        {
            return false;
        }

        match self {
            Self::V1(_) => true,
            Self::V2(bundle) => {
                let recipe = &bundle.canonical_recipe;
                let adapter = &bundle.darktable_adapter;
                recipe.schema_version == CANONICAL_PRESET_RECIPE_SCHEMA_VERSION
                    && adapter.schema_version == DARKTABLE_PRESET_ADAPTER_SCHEMA_VERSION
                    && recipe.preset_id == bundle.preset_id
                    && recipe.published_version == bundle.published_version
                    && recipe.display_name == bundle.display_name
                    && recipe.booth_status == bundle.booth_status
            }
        }
    }

    fn preset_id(&self) -> &str {
        match self {
            Self::V1(bundle) => &bundle.preset_id,
            Self::V2(bundle) => &bundle.preset_id,
        }
    }

    fn display_name(&self) -> &str {
        match self {
            Self::V1(bundle) => &bundle.display_name,
            Self::V2(bundle) => &bundle.display_name,
        }
    }

    fn published_version(&self) -> &str {
        match self {
            Self::V1(bundle) => &bundle.published_version,
            Self::V2(bundle) => &bundle.published_version,
        }
    }

    fn lifecycle_status(&self) -> &str {
        match self {
            Self::V1(bundle) => &bundle.lifecycle_status,
            Self::V2(bundle) => &bundle.lifecycle_status,
        }
    }

    fn booth_status(&self) -> &str {
        match self {
            Self::V1(bundle) => &bundle.booth_status,
            Self::V2(bundle) => &bundle.booth_status,
        }
    }

    fn preview(&self) -> &BundlePreviewAsset {
        match self {
            Self::V1(bundle) => &bundle.preview,
            Self::V2(bundle) => &bundle.preview,
        }
    }

    fn darktable_version(&self) -> Option<&str> {
        match self {
            Self::V1(bundle) => bundle.darktable_version.as_deref(),
            Self::V2(bundle) => Some(bundle.darktable_adapter.darktable_version.as_str()),
        }
    }

    fn xmp_template_path(&self) -> Option<&str> {
        match self {
            Self::V1(bundle) => bundle.xmp_template_path.as_deref(),
            Self::V2(bundle) => Some(bundle.darktable_adapter.xmp_template_path.as_str()),
        }
    }

    fn preview_profile(&self) -> Option<PublishedPresetRenderProfile> {
        match self {
            Self::V1(bundle) => resolve_render_profile(
                bundle.preview_profile.as_ref(),
                &bundle.preset_id,
                &bundle.display_name,
                "preview",
            ),
            Self::V2(bundle) => normalize_render_profile(&bundle.canonical_recipe.preview_intent),
        }
    }

    fn final_profile(&self) -> Option<PublishedPresetRenderProfile> {
        match self {
            Self::V1(bundle) => resolve_render_profile(
                bundle.final_profile.as_ref(),
                &bundle.preset_id,
                &bundle.display_name,
                "final",
            ),
            Self::V2(bundle) => normalize_render_profile(&bundle.canonical_recipe.final_intent),
        }
    }

    fn noise_policy(&self) -> Option<PublishedPresetNoisePolicy> {
        match self {
            Self::V1(_) => Some(PublishedPresetNoisePolicy {
                policy_id: "legacy-balanced-noise".into(),
                display_name: "Legacy Balanced Noise".into(),
                reduction_mode: "balanced".into(),
            }),
            Self::V2(bundle) => normalize_noise_policy(&bundle.canonical_recipe.noise_policy),
        }
    }
}

fn load_bundle_definition(bundle_dir: &Path) -> Option<ParsedPublishedPresetBundle> {
    let bundle_path = bundle_dir.join("bundle.json");
    let bundle_bytes = fs::read_to_string(bundle_path).ok()?;
    let value: Value = serde_json::from_str(&bundle_bytes).ok()?;
    let schema_version = value.get("schemaVersion")?.as_str()?;

    match schema_version {
        PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION_V1 => serde_json::from_value(value)
            .ok()
            .map(ParsedPublishedPresetBundle::V1),
        PUBLISHED_PRESET_BUNDLE_SCHEMA_VERSION_V2 => serde_json::from_value(value)
            .ok()
            .map(ParsedPublishedPresetBundle::V2),
        _ => None,
    }
}

fn resolve_render_profile(
    profile: Option<&BundleRenderProfile>,
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

fn normalize_render_profile(profile: &BundleRenderProfile) -> Option<PublishedPresetRenderProfile> {
    if !is_non_blank(&profile.profile_id)
        || !is_non_blank(&profile.display_name)
        || !is_non_blank(&profile.output_color_space)
    {
        return None;
    }

    Some(PublishedPresetRenderProfile {
        profile_id: profile.profile_id.clone(),
        display_name: profile.display_name.clone(),
        output_color_space: profile.output_color_space.clone(),
    })
}

fn normalize_noise_policy(policy: &BundleNoisePolicy) -> Option<PublishedPresetNoisePolicy> {
    if !is_non_blank(&policy.policy_id)
        || !is_non_blank(&policy.display_name)
        || !is_non_blank(&policy.reduction_mode)
    {
        return None;
    }

    Some(PublishedPresetNoisePolicy {
        policy_id: policy.policy_id.clone(),
        display_name: policy.display_name.clone(),
        reduction_mode: policy.reduction_mode.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{load_published_preset_runtime_bundle, PublishedPresetBundleV2};

    fn unique_test_root(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        std::env::temp_dir().join(format!("boothy-preset-bundle-{label}-{stamp}"))
    }

    #[test]
    fn published_bundle_contract_rejects_unknown_top_level_fields() {
        let json = r#"{
          "schemaVersion": "published-preset-bundle/v2",
          "presetId": "preset_soft-glow",
          "displayName": "Soft Glow",
          "publishedVersion": "2026.04.10",
          "lifecycleStatus": "published",
          "boothStatus": "booth-safe",
          "canonicalRecipe": {
            "schemaVersion": "canonical-preset-recipe/v1",
            "presetId": "preset_soft-glow",
            "publishedVersion": "2026.04.10",
            "displayName": "Soft Glow",
            "boothStatus": "booth-safe",
            "previewIntent": {
              "profileId": "soft-glow-preview",
              "displayName": "Soft Glow Preview",
              "outputColorSpace": "sRGB"
            },
            "finalIntent": {
              "profileId": "soft-glow-final",
              "displayName": "Soft Glow Final",
              "outputColorSpace": "sRGB"
            },
            "noisePolicy": {
              "policyId": "balanced-noise",
              "displayName": "Balanced Noise",
              "reductionMode": "balanced"
            }
          },
          "darktableAdapter": {
            "schemaVersion": "darktable-preset-adapter/v1",
            "darktableVersion": "5.4.1",
            "xmpTemplatePath": "xmp/template.xmp"
          },
          "preview": {
            "kind": "preview-tile",
            "assetPath": "preview.jpg",
            "altText": "Soft Glow sample portrait"
          },
          "unexpectedMetadata": true
        }"#;

        let result = serde_json::from_str::<PublishedPresetBundleV2>(json);

        assert!(result.is_err(), "unknown bundle fields should be rejected");
    }

    #[test]
    fn runtime_bundle_rejects_unpinned_darktable_adapter_versions() {
        let bundle_dir = unique_test_root("unpinned-darktable-version")
            .join("preset_soft-glow")
            .join("2026.04.10");
        fs::create_dir_all(bundle_dir.join("xmp")).expect("xmp dir should exist");
        fs::write(bundle_dir.join("preview.jpg"), "preview").expect("preview should exist");
        fs::write(
            bundle_dir.join("xmp").join("template.xmp"),
            "<darktable></darktable>",
        )
        .expect("xmp should exist");
        fs::write(
            bundle_dir.join("bundle.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": "published-preset-bundle/v2",
              "presetId": "preset_soft-glow",
              "displayName": "Soft Glow",
              "publishedVersion": "2026.04.10",
              "lifecycleStatus": "published",
              "boothStatus": "booth-safe",
              "canonicalRecipe": {
                "schemaVersion": "canonical-preset-recipe/v1",
                "presetId": "preset_soft-glow",
                "publishedVersion": "2026.04.10",
                "displayName": "Soft Glow",
                "boothStatus": "booth-safe",
                "previewIntent": {
                  "profileId": "soft-glow-preview",
                  "displayName": "Soft Glow Preview",
                  "outputColorSpace": "sRGB"
                },
                "finalIntent": {
                  "profileId": "soft-glow-final",
                  "displayName": "Soft Glow Final",
                  "outputColorSpace": "sRGB"
                },
                "noisePolicy": {
                  "policyId": "balanced-noise",
                  "displayName": "Balanced Noise",
                  "reductionMode": "balanced"
                }
              },
              "darktableAdapter": {
                "schemaVersion": "darktable-preset-adapter/v1",
                "darktableVersion": "5.5.0",
                "xmpTemplatePath": "xmp/template.xmp"
              },
              "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.jpg",
                "altText": "Soft Glow sample portrait"
              }
            }))
            .expect("bundle should serialize"),
        )
        .expect("bundle should exist");

        let runtime_bundle = load_published_preset_runtime_bundle(&bundle_dir);

        assert!(
            runtime_bundle.is_none(),
            "runtime loader should reject darktable adapter versions outside the pinned baseline"
        );

        let _ = fs::remove_dir_all(
            bundle_dir
                .parent()
                .and_then(|path| path.parent())
                .expect("test root should exist"),
        );
    }
}
