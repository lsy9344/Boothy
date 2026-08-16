//! Story 7.5: 상주(resident) display renderer 검증 spike의 host 계약.
//!
//! **이 파일은 렌더하지 않는다.** 상주 renderer는 WebView2 안의 WebGL2 엔진이고,
//! 여기서는 그 엔진이 지켜야 하는 계약만 소유한다.
//!
//! - 촬영 **전에** 만들 수 있는 versioned resident recipe (hot path에서 XMP를 다시 읽지 않기 위해)
//! - 엔진이 실제로 구현한 operation allowlist와, 벗어난 recipe를 정확 경로로 내리는 사유 코드
//! - fixture로 얻은 결과가 production 채택 근거로 승격되지 못하게 막는 capability gate (AC 6)
//! - Story 7.4의 `publish_generation_in_dir` 게시 경계 재사용 (AC 7)
//!
//! 성공 조건은 "빠르다"가 아니라 **"빠르면서 정확하고, 아니면 정확 경로로 내려간다"**이다.

use std::path::Path;

use crate::contracts::dto::{
    validate_resident_producer_provenance, DisplayProxyProvenanceDto,
    ResidentProducerProvenanceDto, DISPLAY_RENDER_QUALITY_FAST,
    DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY, RESIDENT_APPROVED_DIRECT_DECODERS,
    RESIDENT_INPUT_PREDECODED_FIXTURE, RESIDENT_INPUT_REAL_CAPTURE_DIRECT,
    RESIDENT_INPUT_REAL_CAPTURE_VIA_ONE_SHOT, RESIDENT_MODE_OFF, RESIDENT_MODE_SHADOW,
};
use crate::contracts::dto::{HostErrorEnvelope, RESIDENT_MODE_EVIDENCE};
use crate::display::DisplayLaneFlags;
use crate::preset::preset_bundle::{PublishedPresetProxyPublication, PublishedPresetRuntimeBundle};

use super::display_artifact::DisplayState;
use super::generation_publisher::{publish_generation_in_dir, PublishOutcome, PublishRequest};
use super::image_probe::content_hash;

pub const RESIDENT_RENDERER_MODE_ENV: &str = "BOOTHY_RESIDENT_RENDERER_MODE";

/// 상주 recipe 계약 버전. recipe의 **모양**이 바뀌면 올린다.
pub const RESIDENT_RECIPE_SCHEMA_VERSION: &str = "resident-recipe/v1";

/// 1순위 후보 엔진의 식별자. `reference_renderer`(darktable)와 절대 섞이지 않는다.
pub const RESIDENT_ENGINE_WEBGL2: &str = "webgl2-resident";

/// spike 엔진의 계약 버전. 엔진이 실제로 구현한 operation 집합이 바뀌면 올린다.
pub const RESIDENT_ENGINE_VERSION: &str = "0.1.0-spike";

/// spike 엔진이 **실제로 구현한** darktable operation 목록.
///
/// 세 승인 preset(`preset_daylight` / `preset_mono-pop` / `preset_soft-glow`)이 사용하는
/// 연산 전체이며, 이 목록 밖의 연산이 하나라도 있으면 recipe 전체가 `resident-ineligible`이다.
/// **부분 적용은 하지 않는다** — 일부만 적용한 화면은 틀린 화면이다.
pub const RESIDENT_OPERATION_ALLOWLIST: &[&str] = &[
    "temperature",
    "exposure",
    "sigmoid",
    "monochrome",
    "bloom",
    "sharpen",
];

/// 엔진이 재현할 수 있다고 확인한 blend operation 파라미터.
///
/// darktable의 `blendop_params`는 압축된 불투명 blob이다. 값을 해석하지 않고
/// **알려진 값과 정확히 같을 때만** 통과시킨다. 모르는 blend를 "아마 기본값"이라고
/// 가정하면 화면이 조용히 달라진다.
pub const RESIDENT_KNOWN_BLENDOP_PARAMS: &[&str] = &[
    "gz11eJxjYIAACQYYOOHEgAZY0QWAgBGLGANDgz0Ej1Q+dcF/IADRAGpyHQU=",
    "gz08eJxjYGBgYAFiCQYYOOHEgAZY0QWAgBGLGANDgz0Ej1Q+dlAx68oBEMbFxwX+AwGIBgCbGCeh",
];

/// 상주 renderer 실행 mode.
///
/// **기본은 `Off`다.** HV-16이 `Go`를 기록해도 Story 7.6 전에는 production 기본값을 바꾸지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentRendererMode {
    /// 아무것도 하지 않는다. 알 수 없는 설정값도 전부 여기로 닫힌다.
    Off,
    /// 렌더하고 계측하지만 **고객 화면 pointer는 건드리지 않는다.**
    Shadow,
    /// HV-16 evidence 회차. 이때만 게시 경계를 통과한다.
    Evidence,
}

impl ResidentRendererMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ResidentRendererMode::Off => RESIDENT_MODE_OFF,
            ResidentRendererMode::Shadow => RESIDENT_MODE_SHADOW,
            ResidentRendererMode::Evidence => RESIDENT_MODE_EVIDENCE,
        }
    }

    /// 렌더를 수행하는가? (shadow도 렌더는 한다 — 게시만 하지 않는다)
    pub fn renders(self) -> bool {
        !matches!(self, ResidentRendererMode::Off)
    }

    /// 고객 화면 pointer를 갱신할 수 있는가?
    pub fn publishes(self) -> bool {
        matches!(self, ResidentRendererMode::Evidence)
    }
}

/// 설정이 없거나 알 수 없으면 `Off`다. 오타가 고객 화면 경로를 켜는 일은 없어야 한다.
pub fn parse_resident_renderer_mode(raw: Option<&str>) -> ResidentRendererMode {
    match raw.map(str::trim) {
        Some(RESIDENT_MODE_SHADOW) => ResidentRendererMode::Shadow,
        Some(RESIDENT_MODE_EVIDENCE) => ResidentRendererMode::Evidence,
        _ => ResidentRendererMode::Off,
    }
}

pub fn current_resident_renderer_mode() -> ResidentRendererMode {
    parse_resident_renderer_mode(std::env::var(RESIDENT_RENDERER_MODE_ENV).ok().as_deref())
}

/// 상주 후보가 이 촬영을 **처리할 수 없는** 이유. 전부 고유 코드이며 조용한 무시는 없다.
///
/// 모든 사유는 결과가 같다: **pinned darktable 정확 경로로 내려간다.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentIneligibleReason {
    /// 상주 lane이 꺼져 있다 (제품 기본값).
    LaneOff,
    /// capture-bound published bundle을 찾지 못했다.
    BundleUnavailable,
    /// bundle이 Story 7.4의 proxy 게시 자격조차 갖추지 못했다.
    ProxyPublicationMissing,
    /// recipe 원본(XMP)을 읽거나 해석하지 못했다.
    RecipeUnresolvable,
    /// recipe에 엔진이 구현하지 않은 operation이 있다.
    OperationUnsupported,
    /// recipe에 해석할 수 없는 blend operation이 있다.
    BlendOperationUnsupported,
    /// recipe에 mask가 있다. spike 엔진은 mask를 구현하지 않는다.
    MaskUnsupported,
    /// 엔진 버전이 recipe가 컴파일된 버전과 다르다.
    EngineVersionMismatch,
    /// 컴파일된 recipe hash가 게시된 recipe와 다르다.
    RecipeHashMismatch,
    /// 출력 색공간 또는 intent가 승인된 값과 다르다.
    OutputProfileMismatch,
    /// WebGL2 / GPU를 사용할 수 없다.
    CapabilityUnavailable,
    /// 상주 context가 아직 준비되지 않았거나 손실됐다.
    ContextNotReady,
    /// viewer가 아직 화면 크기를 보고하지 않았다.
    ViewerNotReady,
    /// **승인된 실제 촬영 입력 경로가 없다.** Story 7.5 AC 6의 기계적 gate.
    SourceRouteUnapproved,
    /// 입력 raster를 별도 one-shot process가 만들었다. startup 비용이 hot path에 남아 있다.
    OneShotStartupNotEliminated,
}

impl ResidentIneligibleReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ResidentIneligibleReason::LaneOff => "resident-lane-off",
            ResidentIneligibleReason::BundleUnavailable => "resident-bundle-unavailable",
            ResidentIneligibleReason::ProxyPublicationMissing => {
                "resident-proxy-publication-missing"
            }
            ResidentIneligibleReason::RecipeUnresolvable => "resident-recipe-unresolvable",
            ResidentIneligibleReason::OperationUnsupported => "resident-operation-unsupported",
            ResidentIneligibleReason::BlendOperationUnsupported => "resident-blend-unsupported",
            ResidentIneligibleReason::MaskUnsupported => "resident-mask-unsupported",
            ResidentIneligibleReason::EngineVersionMismatch => "resident-engine-version-mismatch",
            ResidentIneligibleReason::RecipeHashMismatch => "resident-recipe-hash-mismatch",
            ResidentIneligibleReason::OutputProfileMismatch => "resident-output-profile-mismatch",
            ResidentIneligibleReason::CapabilityUnavailable => "resident-capability-unavailable",
            ResidentIneligibleReason::ContextNotReady => "resident-context-not-ready",
            ResidentIneligibleReason::ViewerNotReady => "resident-viewer-not-ready",
            ResidentIneligibleReason::SourceRouteUnapproved => "resident-source-route-unapproved",
            ResidentIneligibleReason::OneShotStartupNotEliminated => {
                "resident-one-shot-startup-not-eliminated"
            }
        }
    }
}

/// recipe 안의 한 연산. **params는 해석하지 않고 그대로 들고 다닌다.**
///
/// host가 float로 풀어 두면 엔진과 host 두 곳에 같은 해석 규칙이 생기고, 둘이 갈라지면
/// 어느 쪽이 진실인지 알 수 없다. 해석은 엔진 한 곳에서만 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentOperation {
    pub order: u32,
    pub name: String,
    pub enabled: bool,
    pub mod_version: u32,
    pub params_hex: String,
    pub blendop_params: String,
}

/// 촬영 **전에** 확정되는 상주 실행 계획.
///
/// 이 값이 존재한다는 것은 hot path에서 XMP를 다시 순회할 이유가 없다는 뜻이다 (AC 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentRecipe {
    pub schema_version: String,
    pub preset_id: String,
    pub preset_version: String,
    pub engine: String,
    pub engine_version: String,
    pub recipe_version: String,
    pub operations: Vec<ResidentOperation>,
    pub output_color_space: String,
    pub icc_intent: String,
    pub jpeg_quality: u32,
    /// 계획 전체의 해시. 엔진이 자기가 컴파일한 program hash와 함께 generation에 싣는다.
    pub recipe_hash: String,
}

impl ResidentRecipe {
    /// 계획의 안정적인 정규 표현. 해시 입력이자 엔진에 넘기는 직렬화 형식이다.
    pub fn canonical_form(&self) -> String {
        let mut canonical = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.preset_id,
            self.preset_version,
            self.engine,
            self.engine_version,
            self.recipe_version,
            self.output_color_space,
            self.icc_intent,
            self.jpeg_quality
        );

        for operation in &self.operations {
            canonical.push_str(&format!(
                "\n{}:{}:{}:{}:{}:{}",
                operation.order,
                operation.name,
                operation.enabled,
                operation.mod_version,
                operation.params_hex,
                operation.blendop_params
            ));
        }

        canonical
    }
}

/// darktable XMP 한 건에서 상주 실행 계획을 만든다. **hot path에서 부르지 않는다.**
pub fn compile_resident_recipe(
    bundle: &PublishedPresetRuntimeBundle,
    publication: &PublishedPresetProxyPublication,
    xmp_source: &str,
) -> Result<ResidentRecipe, ResidentIneligibleReason> {
    if has_mask_history(xmp_source) {
        return Err(ResidentIneligibleReason::MaskUnsupported);
    }

    let operations = parse_xmp_operations(xmp_source)?;

    if operations.is_empty() {
        return Err(ResidentIneligibleReason::RecipeUnresolvable);
    }

    for operation in &operations {
        if !operation.enabled {
            // 꺼진 연산은 결과에 영향을 주지 않으므로 지원 여부를 따지지 않는다.
            continue;
        }

        if !RESIDENT_OPERATION_ALLOWLIST.contains(&operation.name.as_str()) {
            return Err(ResidentIneligibleReason::OperationUnsupported);
        }

        if !operation.blendop_params.is_empty()
            && !RESIDENT_KNOWN_BLENDOP_PARAMS.contains(&operation.blendop_params.as_str())
        {
            return Err(ResidentIneligibleReason::BlendOperationUnsupported);
        }
    }

    let mut recipe = ResidentRecipe {
        schema_version: RESIDENT_RECIPE_SCHEMA_VERSION.into(),
        preset_id: bundle.preset_id.clone(),
        preset_version: bundle.published_version.clone(),
        engine: RESIDENT_ENGINE_WEBGL2.into(),
        engine_version: RESIDENT_ENGINE_VERSION.into(),
        recipe_version: publication.proxy_recipe_version.clone(),
        operations,
        output_color_space: publication.output_color_space.clone(),
        icc_intent: publication.icc_intent.clone(),
        jpeg_quality: publication.jpeg_quality,
        recipe_hash: String::new(),
    };

    recipe.recipe_hash = content_hash(recipe.canonical_form().as_bytes());

    Ok(recipe)
}

/// `masks_history`에 실제 항목이 있는지. 비어 있는 `<rdf:Seq/>`는 mask가 아니다.
fn has_mask_history(xmp_source: &str) -> bool {
    let Some(start) = xmp_source.find("<darktable:masks_history>") else {
        return false;
    };
    let Some(end) = xmp_source[start..].find("</darktable:masks_history>") else {
        // 열려 있고 닫히지 않았다면 구조를 신뢰할 수 없다. 안전한 쪽으로 판정한다.
        return true;
    };

    xmp_source[start..start + end].contains("<rdf:li")
}

/// XMP history를 순서대로 읽는다. 값 해석은 하지 않고 원문을 보존한다.
fn parse_xmp_operations(
    xmp_source: &str,
) -> Result<Vec<ResidentOperation>, ResidentIneligibleReason> {
    let Some(history_start) = xmp_source.find("<darktable:history>") else {
        return Err(ResidentIneligibleReason::RecipeUnresolvable);
    };
    let Some(history_len) = xmp_source[history_start..].find("</darktable:history>") else {
        return Err(ResidentIneligibleReason::RecipeUnresolvable);
    };

    let history = &xmp_source[history_start..history_start + history_len];
    let mut operations = Vec::new();

    for entry in history.split("<rdf:li").skip(1) {
        let Some(name) = read_attribute(entry, "darktable:operation") else {
            // `<darktable:module>` 형식의 최소 템플릿에는 operation 속성이 없다.
            // 상주 후보가 무엇을 적용해야 하는지 알 수 없으므로 계획을 만들지 않는다.
            return Err(ResidentIneligibleReason::RecipeUnresolvable);
        };

        let order = read_attribute(entry, "darktable:num")
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(ResidentIneligibleReason::RecipeUnresolvable)?;
        let mod_version = read_attribute(entry, "darktable:modversion")
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(ResidentIneligibleReason::RecipeUnresolvable)?;
        let params_hex = read_attribute(entry, "darktable:params")
            .ok_or(ResidentIneligibleReason::RecipeUnresolvable)?;

        operations.push(ResidentOperation {
            order,
            name,
            enabled: read_attribute(entry, "darktable:enabled").as_deref() == Some("1"),
            mod_version,
            params_hex,
            blendop_params: read_attribute(entry, "darktable:blendop_params").unwrap_or_default(),
        });
    }

    operations.sort_by_key(|operation| operation.order);

    Ok(operations)
}

fn read_attribute(entry: &str, attribute: &str) -> Option<String> {
    let needle = format!("{attribute}=\"");
    let start = entry.find(&needle)? + needle.len();
    let end = entry[start..].find('"')? + start;

    Some(entry[start..end].to_string())
}

/// 상주 후보가 무엇을 입력으로 받았는지. **이 값 하나가 AC 6의 판정을 결정한다.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentInputProvenance {
    /// 오프라인에서 미리 디코드한 raster. engine feasibility 자료 전용이다.
    PredecodedFixture {
        producer: String,
        producer_version: String,
    },
    /// 상주 프로세스가 실제 촬영 원본을 직접 읽었다.
    RealCaptureDirect {
        decoder: String,
        decoder_version: String,
    },
    /// 실제 촬영 원본이지만 별도 one-shot process가 raster를 먼저 만들었다.
    RealCaptureViaOneShot {
        producer: String,
        producer_version: String,
        startup_cost_micros: u64,
    },
}

impl ResidentInputProvenance {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResidentInputProvenance::PredecodedFixture { .. } => RESIDENT_INPUT_PREDECODED_FIXTURE,
            ResidentInputProvenance::RealCaptureDirect { .. } => RESIDENT_INPUT_REAL_CAPTURE_DIRECT,
            ResidentInputProvenance::RealCaptureViaOneShot { .. } => {
                RESIDENT_INPUT_REAL_CAPTURE_VIA_ONE_SHOT
            }
        }
    }

    pub fn producer(&self) -> &str {
        match self {
            ResidentInputProvenance::PredecodedFixture { producer, .. }
            | ResidentInputProvenance::RealCaptureViaOneShot { producer, .. } => producer,
            ResidentInputProvenance::RealCaptureDirect { decoder, .. } => decoder,
        }
    }

    pub fn producer_version(&self) -> &str {
        match self {
            ResidentInputProvenance::PredecodedFixture {
                producer_version, ..
            }
            | ResidentInputProvenance::RealCaptureViaOneShot {
                producer_version, ..
            } => producer_version,
            ResidentInputProvenance::RealCaptureDirect {
                decoder_version, ..
            } => decoder_version,
        }
    }

    pub fn startup_cost_micros(&self) -> u64 {
        match self {
            ResidentInputProvenance::RealCaptureViaOneShot {
                startup_cost_micros,
                ..
            } => *startup_cost_micros,
            _ => 0,
        }
    }
}

/// production 채택 자격 판정. **Story 7.5 AC 6의 유일한 구현 지점이다.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentAdoptionVerdict {
    /// 실제 촬영 입력을 승인된 경로로 직접 처리했다.
    ProductionEligible,
    /// 결과는 engine feasibility 자료로만 쓸 수 있다.
    EngineFeasibilityOnly(ResidentIneligibleReason),
}

impl ResidentAdoptionVerdict {
    pub fn production_eligible(self) -> bool {
        matches!(self, ResidentAdoptionVerdict::ProductionEligible)
    }

    pub fn block_reason(self) -> Option<&'static str> {
        match self {
            ResidentAdoptionVerdict::ProductionEligible => None,
            ResidentAdoptionVerdict::EngineFeasibilityOnly(reason) => Some(reason.as_str()),
        }
    }
}

/// 입력 provenance만으로 채택 자격을 판정한다.
///
/// **속도는 이 판정에 들어오지 않는다.** 빠른 fixture 결과가 채택 근거가 되는 것을 막는 것이
/// 이 함수의 유일한 목적이다.
pub fn evaluate_resident_adoption(provenance: &ResidentInputProvenance) -> ResidentAdoptionVerdict {
    match provenance {
        ResidentInputProvenance::PredecodedFixture { .. } => {
            ResidentAdoptionVerdict::EngineFeasibilityOnly(
                ResidentIneligibleReason::SourceRouteUnapproved,
            )
        }
        ResidentInputProvenance::RealCaptureViaOneShot { .. } => {
            ResidentAdoptionVerdict::EngineFeasibilityOnly(
                ResidentIneligibleReason::OneShotStartupNotEliminated,
            )
        }
        ResidentInputProvenance::RealCaptureDirect { decoder, .. } => {
            if RESIDENT_APPROVED_DIRECT_DECODERS.contains(&decoder.as_str()) {
                ResidentAdoptionVerdict::ProductionEligible
            } else {
                ResidentAdoptionVerdict::EngineFeasibilityOnly(
                    ResidentIneligibleReason::SourceRouteUnapproved,
                )
            }
        }
    }
}

/// 상주 렌더 한 건이 hot path에서 무엇을 했는지에 대한 정직한 관측.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentRenderObservation {
    pub producer_build_id: String,
    pub program_hash: String,
    pub context_initialized_at_micros: u64,
    /// 입력 raster가 읽을 수 있는 상태로 도착한 시각. 지연 구간의 시작점이다.
    pub source_ready_at_micros: u64,
    pub hot_path_program_compile_count: u32,
    pub hot_path_process_start_count: u32,
    pub gpu_vendor: String,
    pub gpu_renderer: String,
}

/// generation에 실을 producer provenance를 조립한다.
///
/// **`production_eligible`을 호출자가 넘기지 않는다.** 입력 provenance에서 유도한다.
pub fn build_resident_provenance(
    mode: ResidentRendererMode,
    recipe: &ResidentRecipe,
    input: &ResidentInputProvenance,
    observation: &ResidentRenderObservation,
    fallback_reason: Option<&str>,
) -> ResidentProducerProvenanceDto {
    let verdict = evaluate_resident_adoption(input);

    ResidentProducerProvenanceDto {
        producer_renderer: recipe.engine.clone(),
        producer_renderer_version: recipe.engine_version.clone(),
        producer_build_id: observation.producer_build_id.clone(),
        execution_mode: mode.as_str().into(),
        recipe_schema_version: recipe.schema_version.clone(),
        compiled_recipe_hash: recipe.recipe_hash.clone(),
        program_hash: observation.program_hash.clone(),
        context_initialized_at_micros: observation.context_initialized_at_micros,
        hot_path_program_compile_count: observation.hot_path_program_compile_count,
        hot_path_process_start_count: observation.hot_path_process_start_count,
        input_provenance: input.as_str().into(),
        input_producer: input.producer().into(),
        input_producer_version: input.producer_version().into(),
        source_ready_at_micros: observation.source_ready_at_micros,
        input_startup_cost_micros: input.startup_cost_micros(),
        production_eligible: verdict.production_eligible(),
        adoption_block_reason: verdict.block_reason().map(str::to_string),
        gpu_vendor: observation.gpu_vendor.clone(),
        gpu_renderer: observation.gpu_renderer.clone(),
        fallback_reason: fallback_reason.map(str::to_string),
    }
}

/// 상주 렌더 결과 한 건의 게시 명세.
#[derive(Debug, Clone)]
pub struct ResidentPublishJob<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub capture_id: &'a str,
    pub bound_session_id: Option<&'a str>,
    pub request_viewer_epoch: u64,
    pub current_viewer_epoch: u64,
    pub capture_order: u64,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
    pub display_profile_id: &'a str,
    pub device_pixel_ratio: f64,
    pub source_asset_hash: &'a str,
    pub lanes: DisplayLaneFlags,
}

/// 상주 결과를 **Story 7.4와 같은 게시 경계**로 올린다 (AC 7).
///
/// pointer 경로를 새로 만들지 않는다. tier도 `displayFitPresetProxy` 그대로이며,
/// 상주 후보라는 사실은 `proxy_provenance.resident_provenance`가 들고 있다.
/// `evidence` mode가 아니면 **아무것도 게시하지 않는다.**
#[allow(clippy::too_many_arguments)]
pub fn publish_resident_generation_in_dir(
    base_dir: &Path,
    state: &mut DisplayState,
    job: &ResidentPublishJob<'_>,
    recipe: &ResidentRecipe,
    publication: &PublishedPresetProxyPublication,
    resident: &ResidentProducerProvenanceDto,
    rendered_bytes: &[u8],
    now: &dyn Fn() -> u64,
) -> Result<PublishOutcome, HostErrorEnvelope> {
    validate_resident_producer_provenance(resident)?;

    if resident.execution_mode != RESIDENT_MODE_EVIDENCE {
        return Err(HostErrorEnvelope::validation_message(
            "evidence 모드의 상주 결과만 표시 자산으로 게시할 수 있어요.",
        ));
    }

    let provenance = DisplayProxyProvenanceDto {
        preset_id: recipe.preset_id.clone(),
        preset_version: recipe.preset_version.clone(),
        // 상주 후보는 참조 렌더러가 아니다. 근거는 언제나 승인된 시각 parity 쪽이다.
        approval_basis: publication.basis.as_str().into(),
        proxy_recipe_version: recipe.recipe_version.clone(),
        // 참조 렌더러는 여전히 darktable이다. 이 프레임을 만든 것은 아니다.
        reference_renderer: publication.reference_renderer.clone(),
        reference_renderer_version: publication.reference_renderer_version.clone(),
        render_profile_id: format!("{}-resident", recipe.preset_id),
        output_color_space: recipe.output_color_space.clone(),
        jpeg_quality: recipe.jpeg_quality,
        source_route: resident.input_provenance.clone(),
        source_asset_hash: job.source_asset_hash.into(),
        target_width_px: job.required_source_width_px,
        target_height_px: job.required_source_height_px,
        display_profile_id: job.display_profile_id.into(),
        device_pixel_ratio: job.device_pixel_ratio,
        resident_provenance: Some(resident.clone()),
        // Story 7.6. 상주 후보는 proxy lane의 자리(`displayFitPresetProxy`)를 대신 채운다.
        // 그 tier의 계약 품질은 `fast`이며, HV-16 `Technology No-Go` 이후 이 경로는 기본 `off`다.
        render_quality: DISPLAY_RENDER_QUALITY_FAST.into(),
    };

    let request = PublishRequest {
        session_id: job.session_id,
        request_id: job.request_id,
        capture_id: Some(job.capture_id),
        tier: DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
        sample_variant: None,
        proxy_provenance: Some(&provenance),
        source_bytes: rendered_bytes,
        bound_session_id: job.bound_session_id,
        request_viewer_epoch: job.request_viewer_epoch,
        current_viewer_epoch: job.current_viewer_epoch,
        capture_order: Some(job.capture_order),
        required_source_width_px: job.required_source_width_px,
        required_source_height_px: job.required_source_height_px,
        lanes: job.lanes,
        // 이 lane은 정밀본 tier를 게시하지 않는다. AC 6 gate와 무관하다.
        refined_tier_justified: false,
    };

    publish_generation_in_dir(base_dir, state, &request, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::preset_bundle::{
        ProxyEligibilityBasis, PublishedPresetRenderProfile, PublishedPresetRuntimeBundle,
    };
    use std::path::PathBuf;

    const DAYLIGHT_XMP: &str = include_str!("../preset/default_catalog_assets/preset-daylight.xmp");
    const MONO_POP_XMP: &str = include_str!("../preset/default_catalog_assets/preset-mono-pop.xmp");
    const SOFT_GLOW_XMP: &str =
        include_str!("../preset/default_catalog_assets/preset-soft-glow.xmp");
    const NEUTRAL_XMP: &str =
        include_str!("../preset/default_catalog_assets/default-render-template.xmp");

    fn publication() -> PublishedPresetProxyPublication {
        PublishedPresetProxyPublication {
            basis: ProxyEligibilityBasis::ApprovedVisualParity,
            supported_operations: vec!["exposure".into()],
            proxy_recipe_version: "2026.03.27".into(),
            proxy_recipe_path: PathBuf::from("C:/bundle/preset.xmp"),
            reference_renderer: "darktable".into(),
            reference_renderer_version: "5.4.1".into(),
            output_color_space: "sRGB".into(),
            jpeg_quality: 95,
            icc_intent: "perceptual".into(),
            visual_approval_approved_at: Some("2026-08-01T00:00:00+09:00".into()),
            visual_approval_approved_by: Some("Noah Lee".into()),
            visual_approval_corpus_path: Some("quality/corpus".into()),
        }
    }

    fn bundle(preset_id: &str) -> PublishedPresetRuntimeBundle {
        PublishedPresetRuntimeBundle {
            preset_id: preset_id.into(),
            display_name: "Daylight".into(),
            published_version: "2026.03.27".into(),
            darktable_version: "5.4.1".into(),
            xmp_template_path: PathBuf::from("C:/bundle/preset.xmp"),
            preview_profile: PublishedPresetRenderProfile {
                profile_id: format!("{preset_id}-preview"),
                display_name: "Preview".into(),
                output_color_space: "sRGB".into(),
            },
            final_profile: PublishedPresetRenderProfile {
                profile_id: format!("{preset_id}-final"),
                display_name: "Final".into(),
                output_color_space: "sRGB".into(),
            },
            proxy_publication: Ok(publication()),
        }
    }

    #[test]
    fn missing_and_unknown_modes_close_to_off() {
        assert_eq!(
            parse_resident_renderer_mode(None),
            ResidentRendererMode::Off
        );
        assert_eq!(
            parse_resident_renderer_mode(Some("")),
            ResidentRendererMode::Off
        );
        assert_eq!(
            parse_resident_renderer_mode(Some("on")),
            ResidentRendererMode::Off
        );
        assert_eq!(
            parse_resident_renderer_mode(Some("true")),
            ResidentRendererMode::Off
        );
        assert_eq!(
            parse_resident_renderer_mode(Some("EVIDENCE")),
            ResidentRendererMode::Off
        );
        assert_eq!(
            parse_resident_renderer_mode(Some(" shadow ")),
            ResidentRendererMode::Shadow
        );
        assert_eq!(
            parse_resident_renderer_mode(Some("evidence")),
            ResidentRendererMode::Evidence
        );
    }

    #[test]
    fn only_evidence_mode_may_reach_the_publication_boundary() {
        assert!(!ResidentRendererMode::Off.renders());
        assert!(!ResidentRendererMode::Off.publishes());
        assert!(ResidentRendererMode::Shadow.renders());
        assert!(
            !ResidentRendererMode::Shadow.publishes(),
            "shadow는 비교만 한다. 고객 화면을 건드리면 실험이 제품 사고가 된다"
        );
        assert!(ResidentRendererMode::Evidence.publishes());
    }

    #[test]
    fn the_three_approved_presets_compile_into_resident_recipes() {
        for (preset_id, xmp, expected) in [
            (
                "preset_daylight",
                DAYLIGHT_XMP,
                vec!["temperature", "exposure", "sigmoid", "bloom", "sharpen"],
            ),
            (
                "preset_mono-pop",
                MONO_POP_XMP,
                vec![
                    "temperature",
                    "exposure",
                    "sigmoid",
                    "monochrome",
                    "sharpen",
                ],
            ),
            (
                "preset_soft-glow",
                SOFT_GLOW_XMP,
                vec!["temperature", "exposure", "sigmoid", "bloom", "sharpen"],
            ),
        ] {
            let recipe = compile_resident_recipe(&bundle(preset_id), &publication(), xmp)
                .expect("승인된 preset은 계획으로 컴파일되어야 한다");

            let names: Vec<&str> = recipe
                .operations
                .iter()
                .map(|operation| operation.name.as_str())
                .collect();

            assert_eq!(names, expected, "preset={preset_id}");
            assert!(recipe.recipe_hash.starts_with("fnv1a64:"));
            assert_eq!(recipe.engine, RESIDENT_ENGINE_WEBGL2);
            assert_eq!(recipe.schema_version, RESIDENT_RECIPE_SCHEMA_VERSION);
        }
    }

    #[test]
    fn recipe_hash_changes_when_any_parameter_changes() {
        let original =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), DAYLIGHT_XMP)
                .expect("compiles");
        // exposure 값 하나만 바꾼다. 같은 연산 목록이지만 다른 화면이 나온다.
        let tampered_xmp = DAYLIGHT_XMP.replace(
            "00000000000080b9cdcc8c3f00004842000080c00000000001000000",
            "00000000000080b9cdcc8c400000484 2000080c00000000001000000"
                .replace(' ', "")
                .as_str(),
        );
        let tampered =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &tampered_xmp)
                .expect("compiles");

        assert_ne!(
            original.recipe_hash, tampered.recipe_hash,
            "파라미터가 바뀌었는데 해시가 같으면 drift를 잡을 수 없다"
        );
    }

    #[test]
    fn an_operation_outside_the_allowlist_refuses_the_whole_recipe() {
        // 일부만 적용한 화면은 틀린 화면이다. 하나라도 모르면 전부 정확 경로로 내려간다.
        let unsupported = DAYLIGHT_XMP.replace(
            "darktable:operation=\"bloom\"",
            "darktable:operation=\"retouch\"",
        );

        assert_eq!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &unsupported)
                .unwrap_err(),
            ResidentIneligibleReason::OperationUnsupported
        );
    }

    /// 최소 구조의 XMP를 합성한다. 실제 파일의 공백/개행에 의존하지 않는 negative fixture 전용이다.
    fn synthetic_xmp(masks: &str, entries: &[(&str, &str, &str)]) -> String {
        let mut xmp =
            String::from("<x:xmpmeta><rdf:RDF><rdf:Description>\n<darktable:masks_history>\n");
        xmp.push_str(masks);
        xmp.push_str("\n</darktable:masks_history>\n<darktable:history>\n<rdf:Seq>\n");

        for (index, (operation, enabled, blendop)) in entries.iter().enumerate() {
            xmp.push_str(&format!(
                "<rdf:li darktable:num=\"{index}\" darktable:operation=\"{operation}\" darktable:enabled=\"{enabled}\" darktable:modversion=\"1\" darktable:params=\"00000000\" darktable:blendop_params=\"{blendop}\"/>\n"
            ));
        }

        xmp.push_str("</rdf:Seq>\n</darktable:history>\n</rdf:Description></rdf:RDF></x:xmpmeta>");
        xmp
    }

    const KNOWN_BLEND: &str = "gz11eJxjYIAACQYYOOHEgAZY0QWAgBGLGANDgz0Ej1Q+dcF/IADRAGpyHQU=";

    #[test]
    fn a_disabled_unsupported_operation_does_not_refuse_the_recipe() {
        let disabled = synthetic_xmp(
            "<rdf:Seq/>",
            &[
                ("exposure", "1", KNOWN_BLEND),
                ("retouch", "0", KNOWN_BLEND),
            ],
        );

        assert!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &disabled).is_ok(),
            "꺼진 연산은 결과에 영향을 주지 않으므로 강등 사유가 아니다"
        );
    }

    #[test]
    fn an_enabled_unsupported_operation_in_the_same_shape_is_refused() {
        // 위 테스트가 `enabled` 때문에 통과한 것인지 확인하는 대조군이다.
        let enabled = synthetic_xmp(
            "<rdf:Seq/>",
            &[
                ("exposure", "1", KNOWN_BLEND),
                ("retouch", "1", KNOWN_BLEND),
            ],
        );

        assert_eq!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &enabled)
                .unwrap_err(),
            ResidentIneligibleReason::OperationUnsupported
        );
    }

    #[test]
    fn an_unknown_blend_operation_refuses_the_recipe() {
        let unknown_blend = DAYLIGHT_XMP.replace(
            "gz11eJxjYIAACQYYOOHEgAZY0QWAgBGLGANDgz0Ej1Q+dcF/IADRAGpyHQU=",
            "gz11THIS-IS-NOT-A-BLEND-WE-HAVE-VERIFIED",
        );

        assert_eq!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &unknown_blend)
                .unwrap_err(),
            ResidentIneligibleReason::BlendOperationUnsupported
        );
    }

    #[test]
    fn a_recipe_with_masks_is_refused() {
        let masked = synthetic_xmp(
            "<rdf:Seq><rdf:li darktable:mask_id=\"1\"/></rdf:Seq>",
            &[("exposure", "1", KNOWN_BLEND)],
        );

        assert_eq!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &masked)
                .unwrap_err(),
            ResidentIneligibleReason::MaskUnsupported
        );
    }

    #[test]
    fn an_empty_mask_history_is_not_treated_as_a_mask() {
        // 승인된 세 preset이 모두 빈 `<rdf:Seq/>`를 갖고 있다. 이걸 mask로 오인하면
        // 상주 후보가 아무 preset도 처리하지 못하고 실험 자체가 성립하지 않는다.
        let clean = synthetic_xmp("<rdf:Seq/>", &[("exposure", "1", KNOWN_BLEND)]);

        assert!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), &clean).is_ok()
        );
    }

    #[test]
    fn the_minimal_render_template_cannot_produce_a_resident_plan() {
        // `default-render-template.xmp`은 `<darktable:module>` 형식이라 적용할 파라미터가 없다.
        // "아마 아무것도 안 하는 것"이라고 가정하면 중립 렌더가 preset 렌더로 둔갑한다.
        assert_eq!(
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), NEUTRAL_XMP)
                .unwrap_err(),
            ResidentIneligibleReason::RecipeUnresolvable
        );
    }

    #[test]
    fn every_ineligible_reason_has_a_distinct_code() {
        let codes = [
            ResidentIneligibleReason::LaneOff.as_str(),
            ResidentIneligibleReason::BundleUnavailable.as_str(),
            ResidentIneligibleReason::ProxyPublicationMissing.as_str(),
            ResidentIneligibleReason::RecipeUnresolvable.as_str(),
            ResidentIneligibleReason::OperationUnsupported.as_str(),
            ResidentIneligibleReason::BlendOperationUnsupported.as_str(),
            ResidentIneligibleReason::MaskUnsupported.as_str(),
            ResidentIneligibleReason::EngineVersionMismatch.as_str(),
            ResidentIneligibleReason::RecipeHashMismatch.as_str(),
            ResidentIneligibleReason::OutputProfileMismatch.as_str(),
            ResidentIneligibleReason::CapabilityUnavailable.as_str(),
            ResidentIneligibleReason::ContextNotReady.as_str(),
            ResidentIneligibleReason::ViewerNotReady.as_str(),
            ResidentIneligibleReason::SourceRouteUnapproved.as_str(),
            ResidentIneligibleReason::OneShotStartupNotEliminated.as_str(),
        ];
        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();

        assert_eq!(unique.len(), codes.len(), "조용한 무시를 만들지 않는다");
    }

    #[test]
    fn no_direct_decoder_is_approved_today() {
        // Story 7.3(HV-14)이 세 fast source를 닫았고 그 뒤 승인된 대체 경로가 없다.
        // 이 목록이 비어 있는 한 AC 6에 따라 production 채택은 불가능하다.
        assert!(
            RESIDENT_APPROVED_DIRECT_DECODERS.is_empty(),
            "direct decoder를 추가하려면 dependency·installer·라이선스·성능 범위의 별도 승인이 필요하다"
        );
    }

    #[test]
    fn a_predecoded_fixture_can_never_be_production_eligible() {
        let verdict = evaluate_resident_adoption(&ResidentInputProvenance::PredecodedFixture {
            producer: "darktable-cli".into(),
            producer_version: "5.4.1".into(),
        });

        assert!(!verdict.production_eligible());
        assert_eq!(
            verdict.block_reason(),
            Some("resident-source-route-unapproved")
        );
    }

    #[test]
    fn a_one_shot_produced_raster_is_not_production_eligible_even_from_a_real_capture() {
        // 실제 촬영이어도 one-shot startup이 hot path에 남아 있으면 "상주로 빨라졌다"가 아니다.
        let verdict = evaluate_resident_adoption(&ResidentInputProvenance::RealCaptureViaOneShot {
            producer: "darktable-cli".into(),
            producer_version: "5.4.1".into(),
            startup_cost_micros: 3_500_000,
        });

        assert!(!verdict.production_eligible());
        assert_eq!(
            verdict.block_reason(),
            Some("resident-one-shot-startup-not-eliminated")
        );
    }

    #[test]
    fn an_unapproved_direct_decoder_is_not_production_eligible() {
        let verdict = evaluate_resident_adoption(&ResidentInputProvenance::RealCaptureDirect {
            decoder: "windows-wic-raw".into(),
            decoder_version: "2.5.24.0".into(),
        });

        assert!(!verdict.production_eligible());
        assert_eq!(
            verdict.block_reason(),
            Some("resident-source-route-unapproved")
        );
    }

    fn observation() -> ResidentRenderObservation {
        ResidentRenderObservation {
            producer_build_id: "spike-build-1".into(),
            program_hash: "fnv1a64:0123456789abcdef".into(),
            context_initialized_at_micros: 1_000,
            source_ready_at_micros: 2_000,
            hot_path_program_compile_count: 0,
            hot_path_process_start_count: 0,
            gpu_vendor: "unknown".into(),
            gpu_renderer: "unknown".into(),
        }
    }

    #[test]
    fn provenance_derives_eligibility_instead_of_trusting_the_caller() {
        let recipe =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), DAYLIGHT_XMP)
                .expect("compiles");
        let provenance = build_resident_provenance(
            ResidentRendererMode::Evidence,
            &recipe,
            &ResidentInputProvenance::PredecodedFixture {
                producer: "darktable-cli".into(),
                producer_version: "5.4.1".into(),
            },
            &observation(),
            None,
        );

        assert!(!provenance.production_eligible);
        assert_eq!(
            provenance.adoption_block_reason.as_deref(),
            Some("resident-source-route-unapproved")
        );
        assert_eq!(provenance.producer_renderer, RESIDENT_ENGINE_WEBGL2);
        assert_eq!(
            provenance.input_provenance,
            RESIDENT_INPUT_PREDECODED_FIXTURE
        );
        assert!(validate_resident_producer_provenance(&provenance).is_ok());
    }

    #[test]
    fn a_fabricated_eligible_claim_is_rejected_at_the_publication_boundary() {
        let recipe =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), DAYLIGHT_XMP)
                .expect("compiles");
        let mut forged = build_resident_provenance(
            ResidentRendererMode::Evidence,
            &recipe,
            &ResidentInputProvenance::PredecodedFixture {
                producer: "darktable-cli".into(),
                producer_version: "5.4.1".into(),
            },
            &observation(),
            None,
        );

        // fixture 결과에 production 자격을 붙이는 것이 이 Story에서 가장 쉬운 자기기만이다.
        forged.production_eligible = true;
        forged.adoption_block_reason = None;

        assert!(
            validate_resident_producer_provenance(&forged).is_err(),
            "fixture-only 결과는 어떤 경로로도 production eligible이 될 수 없다"
        );
    }

    #[test]
    fn a_hot_path_compile_blocks_an_eligible_claim() {
        let recipe =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), DAYLIGHT_XMP)
                .expect("compiles");
        let mut forged = build_resident_provenance(
            ResidentRendererMode::Evidence,
            &recipe,
            &ResidentInputProvenance::PredecodedFixture {
                producer: "darktable-cli".into(),
                producer_version: "5.4.1".into(),
            },
            &observation(),
            None,
        );
        forged.production_eligible = true;
        forged.adoption_block_reason = None;
        forged.input_provenance = RESIDENT_INPUT_REAL_CAPTURE_DIRECT.into();
        forged.input_producer = "any-decoder".into();
        forged.hot_path_program_compile_count = 1;

        assert!(validate_resident_producer_provenance(&forged).is_err());
    }

    #[test]
    fn a_non_eligible_frame_must_carry_its_block_reason() {
        let recipe =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), DAYLIGHT_XMP)
                .expect("compiles");
        let mut provenance = build_resident_provenance(
            ResidentRendererMode::Shadow,
            &recipe,
            &ResidentInputProvenance::PredecodedFixture {
                producer: "darktable-cli".into(),
                producer_version: "5.4.1".into(),
            },
            &observation(),
            None,
        );
        provenance.adoption_block_reason = None;

        assert!(
            validate_resident_producer_provenance(&provenance).is_err(),
            "이유 없는 강등은 evidence에서 조용한 무시와 구분되지 않는다"
        );
    }

    #[test]
    fn the_off_mode_can_never_appear_on_a_published_frame() {
        let recipe =
            compile_resident_recipe(&bundle("preset_daylight"), &publication(), DAYLIGHT_XMP)
                .expect("compiles");
        let mut provenance = build_resident_provenance(
            ResidentRendererMode::Shadow,
            &recipe,
            &ResidentInputProvenance::PredecodedFixture {
                producer: "darktable-cli".into(),
                producer_version: "5.4.1".into(),
            },
            &observation(),
            None,
        );
        provenance.execution_mode = RESIDENT_MODE_OFF.into();

        assert!(validate_resident_producer_provenance(&provenance).is_err());
    }
}
