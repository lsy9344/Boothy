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
/// Story 7.4. 현재 승인된 유일한 참조 렌더러. 다른 값은 proxy 자격에서 강등된다.
pub const PROXY_REFERENCE_RENDERER: &str = "darktable";
const DEFAULT_PROXY_ICC_INTENT: &str = "perceptual";
/// Story 7.4. 정확 경로의 JPEG 품질.
///
/// **실측으로 확정했다** — `darktable-cli`에 품질을 넘기지 않았을 때와
/// `plugins/imageio/format/jpeg/quality=95`를 넘겼을 때의 산출물이 바이트까지 동일하다.
/// 즉 95는 오늘 제품이 이미 만들고 있는 품질이며, 이 값이 정확 경로의 기본값이다.
pub const DEFAULT_DISPLAY_PROXY_JPEG_QUALITY: u32 = 95;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPresetRenderProfile {
    pub profile_id: String,
    pub display_name: String,
    pub output_color_space: String,
}

/// Story 7.4. proxy lane에 들어갈 자격의 **근거**.
///
/// 두 근거는 성질이 다르며, evidence에서 반드시 구분되어야 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyEligibilityBasis {
    /// 이전 evidence reader 호환용. 신규 proxy admission에서는 생성하지 않는다.
    ExactReferenceRenderer,
    /// 엔진이나 source가 정확 경로와 달라 결과가 달라질 수 있다.
    /// 이때만 게시 시점의 시각 승인 증거를 요구한다.
    ApprovedVisualParity,
}

impl ProxyEligibilityBasis {
    pub fn as_str(self) -> &'static str {
        match self {
            ProxyEligibilityBasis::ExactReferenceRenderer => "exact-reference-renderer",
            ProxyEligibilityBasis::ApprovedVisualParity => "visual-approval",
        }
    }
}

/// Story 7.4. display-fit proxy 한 건을 만들 때 쓰는 확정된 설정.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPresetProxyPublication {
    pub basis: ProxyEligibilityBasis,
    pub supported_operations: Vec<String>,
    pub proxy_recipe_version: String,
    pub proxy_recipe_path: PathBuf,
    pub reference_renderer: String,
    pub reference_renderer_version: String,
    pub output_color_space: String,
    pub jpeg_quality: u32,
    pub icc_intent: String,
    /// 시각 승인 증거. 이전 `ExactReferenceRenderer` evidence를 읽을 때만 `None`일 수 있다.
    pub visual_approval_approved_at: Option<String>,
    pub visual_approval_approved_by: Option<String>,
    pub visual_approval_corpus_path: Option<String>,
}

/// proxy 자격이 거절된 이유. **조용한 통과 금지** — 강등에는 항상 이유가 붙는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyIneligibleReason {
    /// 번들에 `proxyPublication` 블록이 없다. **기본값은 언제나 여기다.**
    NotDeclared,
    /// `proxyCompatible: false`로 명시 게시됐다.
    NotCompatible,
    /// 필수 필드가 비었거나 값이 범위를 벗어났다.
    IncompleteMetadata,
    /// recipe가 bundle root 밖을 가리키거나 존재하지 않는다.
    RecipeUnresolvable,
    /// reference renderer 또는 그 버전이 런타임 pin과 다르다.
    ReferenceRendererMismatch,
    /// 시각 승인 증거가 없다.
    VisualApprovalMissing,
}

impl ProxyIneligibleReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ProxyIneligibleReason::NotDeclared => "proxy-not-declared",
            ProxyIneligibleReason::NotCompatible => "proxy-not-compatible",
            ProxyIneligibleReason::IncompleteMetadata => "proxy-incomplete-metadata",
            ProxyIneligibleReason::RecipeUnresolvable => "proxy-recipe-unresolvable",
            ProxyIneligibleReason::ReferenceRendererMismatch => "proxy-reference-renderer-mismatch",
            ProxyIneligibleReason::VisualApprovalMissing => "proxy-visual-approval-missing",
        }
    }
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
    /// `Ok`이면 proxy lane 자격이 있고, `Err`이면 그 이유가 담긴다.
    /// **두 경우 모두 preview/final 로딩은 성공한다** — 기존 경로를 깨지 않는다.
    pub proxy_publication: Result<PublishedPresetProxyPublication, ProxyIneligibleReason>,
}

impl PublishedPresetRuntimeBundle {
    /// 게시 시점에 명시적 proxy 승인을 받은 preset인가.
    pub fn has_approved_visual_parity(&self) -> bool {
        self.proxy_publication.is_ok()
    }
}

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
    #[serde(default)]
    darktable_version: Option<String>,
    #[serde(default)]
    xmp_template_path: Option<String>,
    #[serde(default)]
    preview_profile: Option<BundleRenderProfile>,
    #[serde(default)]
    final_profile: Option<BundleRenderProfile>,
    /// Story 7.4. 없으면 `proxyCompatible = false`다. 이것이 기존 번들의 안전한 기본값이다.
    #[serde(default)]
    proxy_publication: Option<BundleProxyPublication>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleProxyPublication {
    proxy_compatible: bool,
    #[serde(default)]
    supported_operations: Vec<String>,
    #[serde(default)]
    proxy_recipe_version: Option<String>,
    #[serde(default)]
    proxy_recipe_path: Option<String>,
    #[serde(default)]
    reference_renderer: Option<String>,
    #[serde(default)]
    reference_renderer_version: Option<String>,
    #[serde(default)]
    output_profile: Option<BundleProxyOutputProfile>,
    #[serde(default)]
    visual_approval: Option<BundleProxyVisualApproval>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleProxyOutputProfile {
    color_space: String,
    jpeg_quality: u32,
    #[serde(default)]
    icc_intent: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleProxyVisualApproval {
    approved_at: String,
    approved_by: String,
    #[serde(default)]
    corpus_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundlePreviewAsset {
    kind: String,
    asset_path: String,
    alt_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleRenderProfile {
    profile_id: String,
    display_name: String,
    output_color_space: String,
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
    if !is_non_blank(&darktable_version) {
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

    // proxy 자격은 **부가 판정**이다. 실패해도 preview/final 로딩을 실패시키지 않는다.
    let proxy_publication = resolve_proxy_publication(
        bundle_dir,
        bundle.proxy_publication.as_ref(),
        &darktable_version,
    );

    if let Err(reason) = proxy_publication {
        log::debug!(
            "preset_proxy_lane_ineligible preset_id={} published_version={} reason={}",
            bundle.preset_id,
            bundle.published_version,
            reason.as_str()
        );
    }

    Some(PublishedPresetRuntimeBundle {
        preset_id: bundle.preset_id,
        display_name: bundle.display_name,
        published_version: bundle.published_version,
        darktable_version,
        xmp_template_path,
        preview_profile,
        final_profile,
        proxy_publication,
    })
}

/// Story 7.4. proxy lane 자격 판정.
///
/// **어떤 결함도 조용히 통과시키지 않는다.** 하나라도 어긋나면 고유 사유와 함께 강등되고,
/// 그 preset은 정확한 darktable RAW 경로만 쓴다.
fn resolve_proxy_publication(
    bundle_dir: &Path,
    proxy: Option<&BundleProxyPublication>,
    pinned_renderer_version: &str,
) -> Result<PublishedPresetProxyPublication, ProxyIneligibleReason> {
    let Some(proxy) = proxy else {
        return Err(ProxyIneligibleReason::NotDeclared);
    };

    if !proxy.proxy_compatible {
        return Err(ProxyIneligibleReason::NotCompatible);
    }

    let recipe_version = proxy
        .proxy_recipe_version
        .as_deref()
        .filter(|value| is_non_blank(value))
        .ok_or(ProxyIneligibleReason::IncompleteMetadata)?;
    let recipe_path_value = proxy
        .proxy_recipe_path
        .as_deref()
        .filter(|value| is_non_blank(value))
        .ok_or(ProxyIneligibleReason::IncompleteMetadata)?;
    let reference_renderer = proxy
        .reference_renderer
        .as_deref()
        .filter(|value| is_non_blank(value))
        .ok_or(ProxyIneligibleReason::IncompleteMetadata)?;
    let reference_renderer_version = proxy
        .reference_renderer_version
        .as_deref()
        .filter(|value| is_non_blank(value))
        .ok_or(ProxyIneligibleReason::IncompleteMetadata)?;
    let output_profile = proxy
        .output_profile
        .as_ref()
        .ok_or(ProxyIneligibleReason::IncompleteMetadata)?;

    if proxy.supported_operations.is_empty()
        || proxy
            .supported_operations
            .iter()
            .any(|operation| !is_non_blank(operation))
    {
        return Err(ProxyIneligibleReason::IncompleteMetadata);
    }

    if !is_non_blank(&output_profile.color_space)
        || !(1..=100).contains(&output_profile.jpeg_quality)
    {
        return Err(ProxyIneligibleReason::IncompleteMetadata);
    }

    // 참조 렌더러가 런타임 pin과 다르면, 승인된 화질 증거는 지금 도는 렌더러의 것이 아니다.
    if reference_renderer != PROXY_REFERENCE_RENDERER
        || reference_renderer_version != pinned_renderer_version
    {
        return Err(ProxyIneligibleReason::ReferenceRendererMismatch);
    }

    let visual_approval = proxy
        .visual_approval
        .as_ref()
        .ok_or(ProxyIneligibleReason::VisualApprovalMissing)?;

    if !is_non_blank(&visual_approval.approved_at) || !is_non_blank(&visual_approval.approved_by) {
        return Err(ProxyIneligibleReason::VisualApprovalMissing);
    }

    let proxy_recipe_path = resolve_bundle_asset_path(bundle_dir, recipe_path_value)
        .ok_or(ProxyIneligibleReason::RecipeUnresolvable)?;

    Ok(PublishedPresetProxyPublication {
        basis: ProxyEligibilityBasis::ApprovedVisualParity,
        supported_operations: proxy.supported_operations.clone(),
        proxy_recipe_version: recipe_version.to_string(),
        proxy_recipe_path,
        reference_renderer: reference_renderer.to_string(),
        reference_renderer_version: reference_renderer_version.to_string(),
        output_color_space: output_profile.color_space.clone(),
        jpeg_quality: output_profile.jpeg_quality,
        icc_intent: output_profile
            .icc_intent
            .as_deref()
            .filter(|value| is_non_blank(value))
            .unwrap_or(DEFAULT_PROXY_ICC_INTENT)
            .to_string(),
        visual_approval_approved_at: Some(visual_approval.approved_at.clone()),
        visual_approval_approved_by: Some(visual_approval.approved_by.clone()),
        visual_approval_corpus_path: visual_approval
            .corpus_path
            .as_deref()
            .filter(|value| is_non_blank(value))
            .map(str::to_string),
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
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 실제 published bundle 디렉터리를 만든다. 경로 검증이 canonicalize에 의존하므로
    /// 합성 JSON만으로는 이 판정을 재현할 수 없다.
    fn write_bundle(test_name: &str, proxy_block: Option<&str>) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let bundle_dir = std::env::temp_dir()
            .join(format!("boothy-bundle-{test_name}-{stamp}"))
            .join("preset_soft-glow")
            .join("2026.08.01");
        fs::create_dir_all(&bundle_dir).expect("bundle dir");
        fs::write(bundle_dir.join("preview.jpg"), [0xFF, 0xD8, 0xFF, 0xD9]).expect("preview");
        fs::write(bundle_dir.join("preset.xmp"), "<x/>").expect("xmp");
        fs::write(bundle_dir.join("recipe.json"), "{}").expect("recipe");

        let proxy = proxy_block
            .map(|block| format!(",\n  \"proxyPublication\": {block}"))
            .unwrap_or_default();
        let bundle = format!(
            r#"{{
  "schemaVersion": "published-preset-bundle/v1",
  "presetId": "preset_soft-glow",
  "displayName": "Soft Glow",
  "publishedVersion": "2026.08.01",
  "lifecycleStatus": "published",
  "boothStatus": "booth-safe",
  "darktableVersion": "5.4.1",
  "xmpTemplatePath": "preset.xmp",
  "preview": {{
    "kind": "preview-tile",
    "assetPath": "preview.jpg",
    "altText": "부드러운 빛"
  }}{proxy}
}}"#
        );
        fs::write(bundle_dir.join("bundle.json"), bundle).expect("bundle.json");

        bundle_dir
    }

    fn valid_proxy_block() -> String {
        r#"{
    "proxyCompatible": true,
    "supportedOperations": ["exposure", "filmic rgb"],
    "proxyRecipeVersion": "1",
    "proxyRecipePath": "recipe.json",
    "referenceRenderer": "darktable",
    "referenceRendererVersion": "5.4.1",
    "outputProfile": { "colorSpace": "sRGB", "jpegQuality": 92, "iccIntent": "perceptual" },
    "visualApproval": {
      "approvedAt": "2026-08-01T00:00:00+09:00",
      "approvedBy": "Noah Lee",
      "corpusPath": "quality/corpus"
    }
  }"#
        .into()
    }

    #[test]
    fn a_bundle_without_the_proxy_block_is_never_proxy_compatible() {
        // **모든 기존 게시 번들이 이 경로다.** 조용히 통과하면 승인되지 않은 룩이 화면에 오른다.
        let bundle_dir = write_bundle("no-proxy", None);
        let bundle =
            load_published_preset_runtime_bundle(&bundle_dir).expect("bundle should still load");

        assert!(!bundle.has_approved_visual_parity());
        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::NotDeclared
        );
        // preview/final 경로는 그대로 동작해야 한다. 기존 렌더를 깨지 않는다.
        assert_eq!(bundle.darktable_version, "5.4.1");
        assert!(bundle.xmp_template_path.is_file());

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn a_fully_approved_bundle_enters_the_proxy_lane() {
        let bundle_dir = write_bundle("approved", Some(&valid_proxy_block()));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");
        let publication = bundle.proxy_publication.expect("proxy publication");

        assert_eq!(publication.proxy_recipe_version, "1");
        assert_eq!(publication.output_color_space, "sRGB");
        assert_eq!(publication.jpeg_quality, 92);
        assert_eq!(publication.icc_intent, "perceptual");
        assert_eq!(publication.supported_operations.len(), 2);
        assert!(publication.proxy_recipe_path.is_file());
        assert_eq!(
            publication.visual_approval_corpus_path.as_deref(),
            Some("quality/corpus")
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn an_explicitly_incompatible_bundle_is_recorded_as_such() {
        let block =
            valid_proxy_block().replace("\"proxyCompatible\": true", "\"proxyCompatible\": false");
        let bundle_dir = write_bundle("incompatible", Some(&block));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");

        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::NotCompatible
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn a_reference_renderer_that_does_not_match_the_runtime_pin_is_demoted() {
        // 승인된 화질 증거가 지금 도는 렌더러의 것이 아니면 그 승인은 이 실행에 적용되지 않는다.
        let block = valid_proxy_block().replace(
            "\"referenceRendererVersion\": \"5.4.1\"",
            "\"referenceRendererVersion\": \"5.6.0\"",
        );
        let bundle_dir = write_bundle("renderer-mismatch", Some(&block));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");

        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::ReferenceRendererMismatch
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn a_recipe_outside_the_bundle_root_is_demoted() {
        let block = valid_proxy_block().replace(
            "\"proxyRecipePath\": \"recipe.json\"",
            "\"proxyRecipePath\": \"../../escape.json\"",
        );
        let bundle_dir = write_bundle("recipe-escape", Some(&block));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");

        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::RecipeUnresolvable
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn a_bundle_without_visual_approval_is_demoted() {
        // 화질 승인 없이 proxy lane에 들어가면 approximation을 숨기는 것이 된다.
        let without_approval = r#"{
    "proxyCompatible": true,
    "supportedOperations": ["exposure"],
    "proxyRecipeVersion": "1",
    "proxyRecipePath": "recipe.json",
    "referenceRenderer": "darktable",
    "referenceRendererVersion": "5.4.1",
    "outputProfile": { "colorSpace": "sRGB", "jpegQuality": 92 }
  }"#;
        let bundle_dir = write_bundle("no-approval", Some(without_approval));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");

        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::VisualApprovalMissing
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn incomplete_metadata_is_demoted_rather_than_guessed() {
        let block = valid_proxy_block().replace(
            "\"supportedOperations\": [\"exposure\", \"filmic rgb\"],",
            "",
        );
        let bundle_dir = write_bundle("incomplete", Some(&block));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");

        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::IncompleteMetadata
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn an_out_of_range_jpeg_quality_is_demoted() {
        let block = valid_proxy_block().replace("\"jpegQuality\": 92", "\"jpegQuality\": 0");
        let bundle_dir = write_bundle("bad-quality", Some(&block));
        let bundle = load_published_preset_runtime_bundle(&bundle_dir).expect("bundle");

        assert_eq!(
            bundle.proxy_publication.unwrap_err(),
            ProxyIneligibleReason::IncompleteMetadata
        );

        let _ = fs::remove_dir_all(bundle_dir.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn every_ineligible_reason_has_a_distinct_code() {
        let codes = [
            ProxyIneligibleReason::NotDeclared.as_str(),
            ProxyIneligibleReason::NotCompatible.as_str(),
            ProxyIneligibleReason::IncompleteMetadata.as_str(),
            ProxyIneligibleReason::RecipeUnresolvable.as_str(),
            ProxyIneligibleReason::ReferenceRendererMismatch.as_str(),
            ProxyIneligibleReason::VisualApprovalMissing.as_str(),
        ];
        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();

        assert_eq!(unique.len(), codes.len());
    }
}
