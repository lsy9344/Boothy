use std::{fs, path::Path};

use serde_json::{json, Value};

use crate::{
    contracts::dto::HostErrorEnvelope,
    preset::{
        preset_bundle::{load_published_preset_runtime_bundle, load_published_preset_summary},
        preset_catalog::resolve_published_preset_catalog_dir,
    },
};

type DefaultPresetSeed = (&'static str, &'static str, &'static str, &'static str);
const DEFAULT_RENDER_TEMPLATE: &str =
    include_str!("default_catalog_assets/default-render-template.xmp");
const AUTHORITATIVE_BUNDLE_SCHEMA_VERSION: &str = "published-preset-bundle/v2";

const DEFAULT_PRESET_SEEDS: [DefaultPresetSeed; 3] = [
    (
        "preset_soft-glow",
        "2026.03.27",
        "Soft Glow",
        include_str!("default_catalog_assets/preset_soft-glow.svg"),
    ),
    (
        "preset_mono-pop",
        "2026.03.27",
        "Mono Pop",
        include_str!("default_catalog_assets/preset_mono-pop.svg"),
    ),
    (
        "preset_daylight",
        "2026.03.27",
        "Daylight",
        include_str!("default_catalog_assets/preset_daylight.svg"),
    ),
];

pub fn ensure_default_preset_catalog_in_dir(base_dir: &Path) -> Result<(), HostErrorEnvelope> {
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let has_any_bundle = contains_any_bundle_json(&catalog_root)?;
    let has_existing_default_seed =
        DEFAULT_PRESET_SEEDS
            .iter()
            .any(|(preset_id, published_version, _, _)| {
                catalog_root
                    .join(preset_id)
                    .join(published_version)
                    .exists()
            });

    if has_any_bundle && !has_existing_default_seed {
        return Ok(());
    }

    for (preset_id, published_version, display_name, preview_svg) in DEFAULT_PRESET_SEEDS {
        let bundle_dir = catalog_root.join(preset_id).join(published_version);
        if has_any_bundle && !bundle_requires_runtime_backfill(&bundle_dir) {
            continue;
        }

        fs::create_dir_all(&bundle_dir).map_err(map_fs_error)?;
        fs::create_dir_all(bundle_dir.join("xmp")).map_err(map_fs_error)?;
        fs::write(bundle_dir.join("preview.svg"), preview_svg).map_err(map_fs_error)?;
        fs::write(
            bundle_dir.join("xmp").join("template.xmp"),
            DEFAULT_RENDER_TEMPLATE,
        )
        .map_err(map_fs_error)?;

        let bundle = json!({
            "schemaVersion": "published-preset-bundle/v2",
            "presetId": preset_id,
            "displayName": display_name,
            "publishedVersion": published_version,
            "lifecycleStatus": "published",
            "boothStatus": "booth-safe",
            "canonicalRecipe": {
                "schemaVersion": "canonical-preset-recipe/v1",
                "presetId": preset_id,
                "publishedVersion": published_version,
                "displayName": display_name,
                "boothStatus": "booth-safe",
                "previewIntent": {
                    "profileId": "preview-jpeg",
                    "displayName": "Booth Preview JPEG",
                    "outputColorSpace": "sRGB",
                },
                "finalIntent": {
                    "profileId": "final-jpeg",
                    "displayName": "Booth Final JPEG",
                    "outputColorSpace": "sRGB",
                },
                "noisePolicy": {
                    "policyId": "balanced-noise",
                    "displayName": "Balanced Noise",
                    "reductionMode": "balanced",
                },
            },
            "darktableAdapter": {
                "schemaVersion": "darktable-preset-adapter/v1",
                "darktableVersion": "5.4.1",
                "xmpTemplatePath": "xmp/template.xmp",
            },
            "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.svg",
                "altText": format!("{display_name} preview"),
            }
        });

        let bundle_bytes = serde_json::to_vec_pretty(&bundle).map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "기본 프리셋 번들을 직렬화하지 못했어요: {error}"
            ))
        })?;

        fs::write(bundle_dir.join("bundle.json"), bundle_bytes).map_err(map_fs_error)?;
    }

    Ok(())
}

fn bundle_requires_runtime_backfill(bundle_dir: &Path) -> bool {
    if !bundle_dir.join("bundle.json").is_file() {
        return true;
    }

    !bundle_uses_authoritative_schema(bundle_dir)
        || load_published_preset_summary(bundle_dir).is_none()
        || load_published_preset_runtime_bundle(bundle_dir).is_none()
}

fn bundle_uses_authoritative_schema(bundle_dir: &Path) -> bool {
    let bundle_bytes = match fs::read_to_string(bundle_dir.join("bundle.json")) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let bundle_value: Value = match serde_json::from_str(&bundle_bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };

    bundle_value.get("schemaVersion").and_then(Value::as_str)
        == Some(AUTHORITATIVE_BUNDLE_SCHEMA_VERSION)
}

fn contains_any_bundle_json(catalog_root: &Path) -> Result<bool, HostErrorEnvelope> {
    if !catalog_root.exists() {
        return Ok(false);
    }

    let preset_dirs = fs::read_dir(catalog_root).map_err(map_fs_error)?;

    for preset_dir in preset_dirs {
        let preset_dir = preset_dir.map_err(map_fs_error)?.path();

        if !preset_dir.is_dir() {
            continue;
        }

        let version_dirs = fs::read_dir(&preset_dir).map_err(map_fs_error)?;

        for version_dir in version_dirs {
            let version_dir = version_dir.map_err(map_fs_error)?.path();

            if version_dir.join("bundle.json").is_file() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("기본 프리셋 카탈로그를 준비하지 못했어요: {error}"))
}
