//! Story 7.5. 상주 renderer spike의 host 경계.
//!
//! **이 파일은 촬영 hot path에 들어가지 않는다.** viewer는 창이 준비될 때 계획을 한 번
//! 받아 가고, 그 뒤에는 host에 recipe를 다시 묻지 않는다. 그것이 AC 1이 요구하는
//! "hot path에서 preset XML을 순회하지 않는다"의 구현이다.

use std::fs;
use std::path::Path;

use crate::contracts::dto::{
    HostErrorEnvelope, ResidentRecipeDto, ResidentRecipeOperationDto, ResidentRecipeRefusalDto,
    ResidentRendererPlanDto, RESIDENT_RENDERER_PLAN_SCHEMA_VERSION,
};
use crate::display::resident_renderer::{
    compile_resident_recipe, current_resident_renderer_mode, ResidentIneligibleReason,
    ResidentRecipe, ResidentRendererMode, RESIDENT_ENGINE_VERSION, RESIDENT_ENGINE_WEBGL2,
};
use crate::preset::preset_bundle::PublishedPresetRuntimeBundle;
use crate::preset::preset_catalog::{
    find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
};

/// 계획을 만든다. **자격이 없는 preset은 조용히 빠지지 않고 사유와 함께 남는다.**
pub fn build_resident_renderer_plan_in_dir(
    base_dir: &Path,
    mode: ResidentRendererMode,
    presets: &[(String, String)],
) -> ResidentRendererPlanDto {
    let mut recipes = Vec::new();
    let mut refusals = Vec::new();

    if mode.renders() {
        let catalog_root = resolve_published_preset_catalog_dir(base_dir);

        for (preset_id, preset_version) in presets {
            match compile_one(&catalog_root, preset_id, preset_version) {
                Ok(recipe) => recipes.push(to_dto(&recipe)),
                Err(reason) => refusals.push(ResidentRecipeRefusalDto {
                    preset_id: preset_id.clone(),
                    preset_version: preset_version.clone(),
                    reason: reason.as_str().into(),
                }),
            }
        }
    }

    ResidentRendererPlanDto {
        schema_version: RESIDENT_RENDERER_PLAN_SCHEMA_VERSION.into(),
        mode: mode.as_str().into(),
        engine: RESIDENT_ENGINE_WEBGL2.into(),
        engine_version: RESIDENT_ENGINE_VERSION.into(),
        recipes,
        refusals,
    }
}

fn compile_one(
    catalog_root: &Path,
    preset_id: &str,
    preset_version: &str,
) -> Result<ResidentRecipe, ResidentIneligibleReason> {
    let bundle: PublishedPresetRuntimeBundle =
        find_published_preset_runtime_bundle(catalog_root, preset_id, preset_version)
            .ok_or(ResidentIneligibleReason::BundleUnavailable)?;

    let publication = bundle
        .proxy_publication
        .clone()
        .map_err(|_| ResidentIneligibleReason::ProxyPublicationMissing)?;

    let xmp_source = fs::read_to_string(&publication.proxy_recipe_path)
        .map_err(|_| ResidentIneligibleReason::RecipeUnresolvable)?;

    compile_resident_recipe(&bundle, &publication, &xmp_source)
}

fn to_dto(recipe: &ResidentRecipe) -> ResidentRecipeDto {
    ResidentRecipeDto {
        schema_version: recipe.schema_version.clone(),
        preset_id: recipe.preset_id.clone(),
        preset_version: recipe.preset_version.clone(),
        engine: recipe.engine.clone(),
        engine_version: recipe.engine_version.clone(),
        recipe_version: recipe.recipe_version.clone(),
        operations: recipe
            .operations
            .iter()
            .map(|operation| ResidentRecipeOperationDto {
                order: operation.order,
                name: operation.name.clone(),
                enabled: operation.enabled,
                mod_version: operation.mod_version,
                params_hex: operation.params_hex.clone(),
                blendop_params: operation.blendop_params.clone(),
            })
            .collect(),
        output_color_space: recipe.output_color_space.clone(),
        icc_intent: recipe.icc_intent.clone(),
        jpeg_quality: recipe.jpeg_quality,
        recipe_hash: recipe.recipe_hash.clone(),
    }
}

/// viewer가 창 준비 시점에 한 번 부른다.
#[tauri::command]
pub fn get_resident_renderer_plan(
    app: tauri::AppHandle,
    presets: Vec<(String, String)>,
) -> Result<ResidentRendererPlanDto, HostErrorEnvelope> {
    let base_dir = crate::commands::display_commands::session_base_dir_for(&app)?;

    Ok(build_resident_renderer_plan_in_dir(
        &base_dir,
        current_resident_renderer_mode(),
        &presets,
    ))
}

/// 상주 lane 상태를 booth 로그에 남긴다. 켜져 있으면 실험 중임을 분명히 한다.
pub fn log_resident_renderer_mode() {
    let mode = current_resident_renderer_mode();

    if mode.renders() {
        log::warn!(
            "resident_renderer_lane_enabled mode={} — Story 7.5 spike이며 production 채택 결정이 아니다",
            mode.as_str()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("boothy-resident-plan-{label}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn the_off_mode_hands_out_no_recipes_at_all() {
        let base_dir = temp_dir("off");
        let plan = build_resident_renderer_plan_in_dir(
            &base_dir,
            ResidentRendererMode::Off,
            &[("preset_daylight".into(), "2026.03.27".into())],
        );

        assert_eq!(plan.mode, "off");
        assert!(plan.recipes.is_empty());
        // off일 때는 자격 판정 자체를 하지 않는다. 거절 목록도 비어 있어야 한다.
        assert!(plan.refusals.is_empty());

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn a_missing_bundle_is_refused_with_a_reason_not_dropped() {
        let base_dir = temp_dir("missing");
        let plan = build_resident_renderer_plan_in_dir(
            &base_dir,
            ResidentRendererMode::Shadow,
            &[("preset_daylight".into(), "2026.03.27".into())],
        );

        assert!(plan.recipes.is_empty());
        assert_eq!(plan.refusals.len(), 1);
        assert_eq!(plan.refusals[0].reason, "resident-bundle-unavailable");

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn the_plan_pins_the_engine_identity_it_was_compiled_for() {
        let base_dir = temp_dir("identity");
        let plan =
            build_resident_renderer_plan_in_dir(&base_dir, ResidentRendererMode::Shadow, &[]);

        assert_eq!(plan.schema_version, RESIDENT_RENDERER_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.engine, RESIDENT_ENGINE_WEBGL2);
        assert_eq!(plan.engine_version, RESIDENT_ENGINE_VERSION);

        let _ = fs::remove_dir_all(&base_dir);
    }
}
