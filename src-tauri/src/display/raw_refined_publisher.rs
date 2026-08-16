//! Story 7.6: RAW 정밀본(`rawRefinedDisplay`) lane.
//!
//! **게시 순서를 새로 만들지 않는다.** Story 7.2의 `generation_publisher`를 그대로 호출하고,
//! `proxy_publisher`를 지우거나 대체하지도 않는다. 두 publisher 모두 같은 게시 경계를 쓴다 —
//! 표시 종단점이 두 벌이 되는 순간 Story 7.2가 증명한 것이 무효가 된다.
//!
//! 이 lane이 만드는 것은 **승급**이지 첫 성공 화면이 아니다 (UX-DR19).
//! proxy가 commit되지 않았으면 정밀본도 존재하지 않는다.
//!
//! ## 이 tier의 차이는 `--hq` 하나뿐이다
//!
//! HV-14 이후 proxy의 source가 이미 RAW original이다. 그래서 "RAW에서 온다"는 것은
//! 더 이상 tier의 차이가 아니다. 같은 RAW, 같은 capture-bound XMP, 같은 pinned darktable,
//! **완전히 같은 목표 크기**를 쓰고 darktable pixelpipe의 downsampling 품질만 다르다.

use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::dto::{
    DisplayProxyProvenanceDto, HostErrorEnvelope, DISPLAY_RENDER_QUALITY_HIGH,
    DISPLAY_TIER_RAW_REFINED_DISPLAY,
};
use crate::display::DisplayLaneFlags;
use crate::preset::preset_bundle::{
    ProxyIneligibleReason, PublishedPresetProxyPublication, PublishedPresetRuntimeBundle,
};
use crate::render::{
    display_job_key_for_context, render_raw_refined_display_to_path_in_dir,
    DisplayProxyRenderOutcome, DisplayProxyRenderSpec, DisplayRenderDisposition,
    DisplayRenderSchedule, JobCoordinates, JobPriority,
};

use super::display_artifact::DisplayState;
use super::generation_publisher::{publish_generation_in_dir, PublishOutcome, PublishRequest};
use super::image_probe::{content_hash, probe_jpeg};
use super::proxy_publisher::proxy_render_staging_path;

pub const RAW_REFINED_MODE_ENV: &str = "BOOTHY_RAW_REFINED_MODE";

/// 정밀본 lane 모드.
///
/// **기본은 `Off`다.** HV-17A와 HV-17B가 각각 독립적으로 `Go`를 기록하기 전까지는
/// 고객 화면 경로를 켜지 않는다. 알 수 없는 값도 전부 `Off`로 닫힌다 —
/// 오타가 고객 화면 경로를 켜는 일은 없어야 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawRefinedLaneMode {
    Off,
    On,
}

impl RawRefinedLaneMode {
    pub fn is_enabled(self) -> bool {
        matches!(self, RawRefinedLaneMode::On)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RawRefinedLaneMode::Off => "off",
            RawRefinedLaneMode::On => "on",
        }
    }
}

pub fn parse_raw_refined_lane_mode(raw: Option<&str>) -> RawRefinedLaneMode {
    match raw.map(str::trim) {
        Some("on") => RawRefinedLaneMode::On,
        _ => RawRefinedLaneMode::Off,
    }
}

pub fn current_raw_refined_lane_mode() -> RawRefinedLaneMode {
    parse_raw_refined_lane_mode(std::env::var(RAW_REFINED_MODE_ENV).ok().as_deref())
}

/// AC 6의 **detail 축(MTF50) 판정**. 이 tier의 존재 조건이다.
///
/// 두 결과가 측정 한계 안에서 같으면 교체는 연출일 뿐이고, 고객에게 아무 가치가 없으면서
/// darktable 부하만 두 배가 된다. **없는 차이를 만들어 내지 않는다.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierJustification {
    /// 아직 측정하지 않았다. **미실행은 통과가 아니다.**
    NotMeasured,
    /// 측정했고 기준을 넘지 못했다 (`tier-not-justified`).
    NotJustified,
    /// `median MTF50(refined) ≥ 1.10 × median MTF50(proxy)` 이고 어떤 표본에서도 역행이 없다.
    Justified,
}

impl TierJustification {
    pub fn is_justified(self) -> bool {
        matches!(self, TierJustification::Justified)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TierJustification::NotMeasured => "not-measured",
            TierJustification::NotJustified => "tier-not-justified",
            TierJustification::Justified => "justified",
        }
    }
}

/// **현재 detail 축 판정은 `NotMeasured`다.**
///
/// AC 6의 측정에는 slanted-edge 대상이 있는 실제 EOS 700D 촬영 최소 3장과 pinned darktable이
/// 필요하다 (T6). 그 원자료는 HV-17 회차에서만 나온다. 그전까지 이 값을 `Justified`로 바꾸면
/// **측정되지 않은 tier 차이를 고객 화면에 게시하는 것**이고, AC 6이 정확히 그것을 금지한다.
///
/// `RESIDENT_APPROVED_DIRECT_DECODERS`가 빈 목록으로 상주 후보를 막는 것과 같은 장치다.
/// 값을 바꾸려면 T6의 원자료(`tests/hardware/raw-refined/hv-17/tier-justification/`)와
/// 그 판정이 evidence 패키지에 있어야 한다.
pub const RAW_REFINED_TIER_JUSTIFICATION: TierJustification = TierJustification::NotMeasured;

/// 정밀본 lane이 **시작조차 하지 않은** 이유. 전부 고유 코드이며 조용한 무시는 없다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawRefinedSkipReason {
    /// lane이 운영 설정으로 꺼져 있다 (제품 기본값).
    LaneOff,
    /// AC 6의 detail 축 gate가 `justified`를 기록하지 않았다.
    TierNotJustified,
    /// 승급할 proxy generation이 화면에 없다. **정밀본은 첫 성공 화면의 대체가 아니다.**
    ProxyNotCommitted,
    /// 렌더 중 viewer 세션·epoch·물리 크기 또는 display profile이 바뀌었다.
    ViewerContextChanged,
    /// capture record에 preset 결속이 없다.
    PresetUnbound,
    /// capture-bound published bundle을 찾지 못했다.
    BundleUnavailable,
    /// bundle이 게시 자격을 갖추지 못했다.
    ProxyIneligible(ProxyIneligibleReason),
    /// source 파일이 없거나 읽히지 않는다.
    SourceUnavailable,
    /// proxy가 기록한 입력 해시와 현재 RAW 내용이 다르다.
    SourceHashMismatch,
    /// P0가 끝났을 때 이미 촬영 예산을 넘겨 P1 신규 투입을 중단했다.
    DeadlineMissed,
    /// 실측 크기가 활성 proxy와 다르다. **AC 4의 crop/scale 점프 0을 만드는 장치다.**
    DimensionMismatch,
    /// 같은 좌표의 렌더가 이미 진행 중이라 병합됐다.
    RenderCoalesced,
    /// 삭제·세션 교체·epoch 변경·더 새 촬영으로 취소됐다.
    RenderCancelled,
}

impl RawRefinedSkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RawRefinedSkipReason::LaneOff => "refined-lane-off",
            RawRefinedSkipReason::TierNotJustified => "refined-tier-not-justified",
            RawRefinedSkipReason::ProxyNotCommitted => "refined-proxy-not-committed",
            RawRefinedSkipReason::ViewerContextChanged => "refined-viewer-context-changed",
            RawRefinedSkipReason::PresetUnbound => "refined-preset-unbound",
            RawRefinedSkipReason::BundleUnavailable => "refined-bundle-unavailable",
            RawRefinedSkipReason::ProxyIneligible(reason) => reason.as_str(),
            RawRefinedSkipReason::SourceUnavailable => "refined-source-unavailable",
            RawRefinedSkipReason::SourceHashMismatch => "refined-source-hash-mismatch",
            RawRefinedSkipReason::DeadlineMissed => "refined-deadline-missed",
            RawRefinedSkipReason::DimensionMismatch => "refined-dimension-mismatch",
            RawRefinedSkipReason::RenderCoalesced => "refined-render-coalesced",
            RawRefinedSkipReason::RenderCancelled => "refined-render-cancelled",
        }
    }
}

/// **commit된 proxy generation에서 상속하는 값들.**
///
/// 렌더 시점에 viewer를 다시 읽지 않는다. 그 사이 viewer 문맥이 바뀌었다면 정밀본을 버리고
/// 화면은 proxy 그대로 남긴다 — 새로 읽은 크기로 렌더하면 교체 순간 사진이 튄다.
#[derive(Debug, Clone, PartialEq)]
pub struct InheritedProxyGeometry {
    /// 이 정밀본이 승급하려는 proxy generation.
    pub proxy_generation_id: String,
    /// 렌더 요청 시점의 목표 크기. proxy가 쓴 값 **그대로**다.
    pub target_width_px: u32,
    pub target_height_px: u32,
    pub display_profile_id: String,
    pub device_pixel_ratio: f64,
    /// 활성 proxy generation의 **실측** 픽셀 크기. 게시 직전 정확히 같아야 한다.
    pub proxy_source_width_px: u32,
    pub proxy_source_height_px: u32,
    /// capture record에 고정된 preset. proxy와 같아야 한다.
    pub preset_id: String,
    pub preset_version: String,
    /// 입력 RAW의 내용 해시. proxy가 읽은 것과 같은 파일임을 증명한다.
    pub source_asset_hash: String,
    pub source_route: String,
}

/// 한 촬영의 정밀본 작업 명세.
#[derive(Debug, Clone)]
pub struct RawRefinedJob<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub capture_id: &'a str,
    pub source_path: &'a Path,
    pub bound_session_id: Option<&'a str>,
    pub request_viewer_epoch: u64,
    pub current_viewer_epoch: u64,
    pub capture_order: u64,
    pub inherited: InheritedProxyGeometry,
    pub lanes: DisplayLaneFlags,
    /// trusted capture input 시각 + NFR-003 hard max. **순서 결정과 관측에만 쓴다.**
    pub deadline_micros: u64,
}

impl RawRefinedJob<'_> {
    fn scheduler_coordinates(&self) -> JobCoordinates {
        JobCoordinates {
            session_id: self.session_id.to_string(),
            request_id: self.request_id.to_string(),
            capture_id: Some(self.capture_id.to_string()),
            capture_order: Some(self.capture_order),
            viewer_epoch: self.request_viewer_epoch,
        }
    }
}

/// 정밀본 lane 자격을 판정한다. **렌더도 파일 생성도 하지 않는다.**
///
/// proxy lane과 다른 두 관문이 있다: AC 6의 detail 축 gate와, 승급할 proxy의 존재다.
pub fn evaluate_raw_refined_eligibility(
    lane_mode: RawRefinedLaneMode,
    justification: TierJustification,
    proxy_committed: bool,
    bundle: Option<&PublishedPresetRuntimeBundle>,
    pinned_renderer_version: &str,
) -> Result<PublishedPresetProxyPublication, RawRefinedSkipReason> {
    if !lane_mode.is_enabled() {
        return Err(RawRefinedSkipReason::LaneOff);
    }

    // **측정되지 않은 tier 차이를 게시하지 않는다.** 미실행은 통과가 아니다.
    if !justification.is_justified() {
        return Err(RawRefinedSkipReason::TierNotJustified);
    }

    // 승급은 대체가 아니다. proxy 없이 정밀본만 뜨면 첫 화면이 3배 느려진다.
    if !proxy_committed {
        return Err(RawRefinedSkipReason::ProxyNotCommitted);
    }

    let bundle = bundle.ok_or(RawRefinedSkipReason::BundleUnavailable)?;
    let publication = bundle
        .proxy_publication
        .clone()
        .map_err(RawRefinedSkipReason::ProxyIneligible)?;

    if publication.reference_renderer_version != pinned_renderer_version {
        return Err(RawRefinedSkipReason::ProxyIneligible(
            ProxyIneligibleReason::ReferenceRendererMismatch,
        ));
    }

    Ok(publication)
}

/// 정밀본 렌더 산출물의 staging 경로. proxy와 **같은 디렉터리, 다른 파일 이름**이다.
pub fn raw_refined_staging_path(base_dir: &Path, job: &RawRefinedJob<'_>) -> PathBuf {
    proxy_render_staging_path(
        base_dir,
        job.session_id,
        job.request_id,
        job.capture_id,
        DISPLAY_TIER_RAW_REFINED_DISPLAY,
    )
}

#[derive(Debug)]
pub enum RawRefinedRenderError {
    /// lane이 정당하게 중단됐다. 오류가 아니라 정상 결과이며 진단에 남는다.
    Skipped {
        reason: RawRefinedSkipReason,
        detail: String,
    },
    /// 렌더 자체가 실패했다. **RAW·preview·final·현재 화면 truth는 건드리지 않는다.**
    Render(Box<crate::render::RenderWorkerError>),
}

#[derive(Debug)]
pub struct RenderedRawRefined {
    pub bytes: Vec<u8>,
    pub staging_path: PathBuf,
    pub render: DisplayProxyRenderOutcome,
    pub provenance: DisplayProxyProvenanceDto,
    /// 렌더 결과의 실측 크기. 활성 proxy와 정확히 같아야 한다.
    pub source_width_px: u32,
    pub source_height_px: u32,
}

fn verify_source_hash(
    source_path: &Path,
    expected_source_hash: &str,
) -> Result<(), RawRefinedRenderError> {
    let source_bytes = fs::read(source_path).map_err(|error| RawRefinedRenderError::Skipped {
        reason: RawRefinedSkipReason::SourceUnavailable,
        detail: format!(
            "정밀본 source를 읽지 못했어요: path={} error={error}",
            source_path.to_string_lossy()
        ),
    })?;
    let actual_source_hash = content_hash(&source_bytes);

    if actual_source_hash != expected_source_hash {
        return Err(RawRefinedRenderError::Skipped {
            reason: RawRefinedSkipReason::SourceHashMismatch,
            detail: format!(
                "proxy와 정밀본의 RAW 해시가 달라요: expected={expected_source_hash} actual={actual_source_hash}"
            ),
        });
    }

    Ok(())
}

/// 정밀본을 렌더한다. **크기는 proxy provenance에서 상속하고 viewer를 다시 읽지 않는다.**
///
/// 호출자는 `DisplayState` mutex를 잡은 채로 부르지 **않는다** — 렌더가 수 초 걸리므로
/// 그 사이 pointer 조회가 막히면 관람 화면이 멈춘다.
pub fn render_raw_refined_display(
    base_dir: &Path,
    job: &RawRefinedJob<'_>,
    bundle: &PublishedPresetRuntimeBundle,
    publication: &PublishedPresetProxyPublication,
    now: &dyn Fn() -> u64,
) -> Result<RenderedRawRefined, RawRefinedRenderError> {
    if !job.source_path.is_file() {
        return Err(RawRefinedRenderError::Skipped {
            reason: RawRefinedSkipReason::SourceUnavailable,
            detail: format!(
                "정밀본 source를 찾지 못했어요: path={}",
                job.source_path.to_string_lossy()
            ),
        });
    }

    verify_source_hash(job.source_path, &job.inherited.source_asset_hash)?;

    let output_path = raw_refined_staging_path(base_dir, job);
    let spec = DisplayProxyRenderSpec {
        source_asset_path: job.source_path,
        // proxy와 **같은** capture-bound recipe다. 다른 것을 쓰면 룩이 달라진다.
        xmp_template_path: &publication.proxy_recipe_path,
        output_path: &output_path,
        // **proxy generation에서 상속한 값 그대로.** 렌더 시점 viewer를 읽지 않는다.
        target_width_px: job.inherited.target_width_px,
        target_height_px: job.inherited.target_height_px,
        output_color_space: &publication.output_color_space,
        jpeg_quality: publication.jpeg_quality,
        icc_intent: &publication.icc_intent,
        source_is_raw_original: true,
        schedule: DisplayRenderSchedule {
            // 승급이지 첫 화면이 아니다. P0가 도는 동안 CPU를 나눠 갖지 않는다.
            priority: JobPriority::P1CurrentRawRefined,
            job_key: display_job_key_for_context(
                job.session_id,
                job.request_id,
                job.capture_id,
                DISPLAY_TIER_RAW_REFINED_DISPLAY,
                &bundle.preset_id,
                &bundle.published_version,
                job.request_viewer_epoch,
                job.inherited.target_width_px,
                job.inherited.target_height_px,
                &job.inherited.display_profile_id,
                job.inherited.device_pixel_ratio,
            ),
            deadline_micros: job.deadline_micros,
            coordinates: job.scheduler_coordinates(),
        },
    };

    let render = match render_raw_refined_display_to_path_in_dir(base_dir, &spec, now)
        .map_err(|error| RawRefinedRenderError::Render(Box::new(error)))?
    {
        DisplayRenderDisposition::Rendered(outcome) => *outcome,
        DisplayRenderDisposition::Coalesced { job_key, .. } => {
            return Err(RawRefinedRenderError::Skipped {
                reason: RawRefinedSkipReason::RenderCoalesced,
                detail: format!("같은 좌표의 정밀본 렌더가 이미 진행 중이에요: jobKey={job_key}"),
            })
        }
        DisplayRenderDisposition::Cancelled { reason, .. } => {
            let _ = fs::remove_file(&output_path);

            return Err(RawRefinedRenderError::Skipped {
                reason: RawRefinedSkipReason::RenderCancelled,
                detail: format!("정밀본 렌더가 취소됐어요: reason={reason}"),
            });
        }
    };

    let rendered_bytes = fs::read(&output_path).map_err(|error| {
        let _ = fs::remove_file(&output_path);

        RawRefinedRenderError::Skipped {
            reason: RawRefinedSkipReason::SourceUnavailable,
            detail: format!("정밀본 산출물을 다시 읽지 못했어요: {error}"),
        }
    })?;

    // 게시 전 검증 3단계: 구조 decode. 게시 경계도 같은 probe를 다시 하지만,
    // 여기서 먼저 걸러야 크기가 틀린 파일을 staging 밖으로 옮기지 않는다.
    let probe = probe_jpeg(&rendered_bytes).map_err(|error| {
        let _ = fs::remove_file(&output_path);

        RawRefinedRenderError::Skipped {
            reason: RawRefinedSkipReason::SourceUnavailable,
            detail: format!("정밀본 산출물의 구조를 읽지 못했어요: {error:?}"),
        }
    })?;

    // 게시 전 검증 4단계: **크기 동일성.** AC 4의 crop/scale 점프 0을 만드는 기계적 장치다.
    if probe.width_px != job.inherited.proxy_source_width_px
        || probe.height_px != job.inherited.proxy_source_height_px
    {
        let _ = fs::remove_file(&output_path);

        return Err(RawRefinedRenderError::Skipped {
            reason: RawRefinedSkipReason::DimensionMismatch,
            detail: format!(
                "정밀본 크기가 활성 proxy와 달라요: refined={}x{} proxy={}x{}",
                probe.width_px,
                probe.height_px,
                job.inherited.proxy_source_width_px,
                job.inherited.proxy_source_height_px
            ),
        });
    }

    Ok(RenderedRawRefined {
        bytes: rendered_bytes,
        staging_path: output_path,
        render,
        provenance: DisplayProxyProvenanceDto {
            // 게시 전 검증 5단계: provenance는 전부 proxy generation에서 상속한다.
            preset_id: job.inherited.preset_id.clone(),
            preset_version: job.inherited.preset_version.clone(),
            approval_basis: publication.basis.as_str().into(),
            proxy_recipe_version: publication.proxy_recipe_version.clone(),
            reference_renderer: publication.reference_renderer.clone(),
            reference_renderer_version: publication.reference_renderer_version.clone(),
            // 실제 렌더는 proxy와 같은 display-fit output profile을 쓰며 `--hq`만 다르다.
            render_profile_id: bundle.preview_profile.profile_id.clone(),
            output_color_space: publication.output_color_space.clone(),
            jpeg_quality: publication.jpeg_quality,
            source_route: job.inherited.source_route.clone(),
            source_asset_hash: job.inherited.source_asset_hash.clone(),
            target_width_px: job.inherited.target_width_px,
            target_height_px: job.inherited.target_height_px,
            display_profile_id: job.inherited.display_profile_id.clone(),
            device_pixel_ratio: job.inherited.device_pixel_ratio,
            // 참조 렌더러(darktable)가 직접 만든 프레임이다.
            resident_provenance: None,
            // **이 값이 두 tier를 구분하는 유일한 필드다.**
            render_quality: DISPLAY_RENDER_QUALITY_HIGH.into(),
        },
        source_width_px: probe.width_px,
        source_height_px: probe.height_px,
    })
}

/// 렌더가 끝난 정밀본을 Story 7.2의 게시 순서에 태운다.
///
/// **두 번째 게시 경로를 만들지 않는다.** `publish_generation_in_dir` 하나만 쓴다.
///
/// `current_viewer_epoch`는 proxy lane과 같은 이유로 **게시 시점에 다시 읽은 값**을 받는다.
pub fn publish_rendered_raw_refined_in_dir(
    base_dir: &Path,
    state: &mut DisplayState,
    job: &RawRefinedJob<'_>,
    rendered: &RenderedRawRefined,
    justification: TierJustification,
    current_viewer_epoch: u64,
    now: &dyn Fn() -> u64,
) -> Result<PublishOutcome, HostErrorEnvelope> {
    let request = PublishRequest {
        session_id: job.session_id,
        request_id: job.request_id,
        capture_id: Some(job.capture_id),
        tier: DISPLAY_TIER_RAW_REFINED_DISPLAY,
        sample_variant: None,
        proxy_provenance: Some(&rendered.provenance),
        source_bytes: &rendered.bytes,
        bound_session_id: job.bound_session_id,
        request_viewer_epoch: job.request_viewer_epoch,
        current_viewer_epoch,
        capture_order: Some(job.capture_order),
        // pointer의 required size는 viewer photo rect × DPR이다. 실측 raster 크기는
        // generation.sourceWidth/Height에 별도로 남고 admission에서 활성 proxy와 비교한다.
        required_source_width_px: job.inherited.target_width_px,
        required_source_height_px: job.inherited.target_height_px,
        lanes: job.lanes,
        // 게시 전 검증 6단계. 게시 경계가 이 값을 다시 본다 — 렌더까지 끝났더라도
        // 판정이 없으면 pointer는 전진하지 않는다.
        refined_tier_justified: justification.is_justified(),
    };

    let outcome = publish_generation_in_dir(base_dir, state, &request, now);

    // 게시 성공/실패와 무관하게 렌더 staging 산출물은 남기지 않는다.
    let _ = fs::remove_file(&rendered.staging_path);

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::preset_bundle::{ProxyEligibilityBasis, PublishedPresetRenderProfile};

    fn publication() -> PublishedPresetProxyPublication {
        PublishedPresetProxyPublication {
            basis: ProxyEligibilityBasis::ApprovedVisualParity,
            supported_operations: vec!["exposure".into()],
            proxy_recipe_version: "1".into(),
            proxy_recipe_path: PathBuf::from("C:/bundle/recipe.json"),
            reference_renderer: "darktable".into(),
            reference_renderer_version: "5.4.1".into(),
            output_color_space: "sRGB".into(),
            jpeg_quality: 92,
            icc_intent: "perceptual".into(),
            visual_approval_approved_at: Some("2026-08-01T00:00:00+09:00".into()),
            visual_approval_approved_by: Some("Noah Lee".into()),
            visual_approval_corpus_path: Some("quality/corpus".into()),
        }
    }

    fn bundle() -> PublishedPresetRuntimeBundle {
        PublishedPresetRuntimeBundle {
            preset_id: "preset_soft-glow".into(),
            display_name: "Soft Glow".into(),
            published_version: "2026.08.01".into(),
            darktable_version: "5.4.1".into(),
            xmp_template_path: PathBuf::from("C:/bundle/preset.xmp"),
            preview_profile: PublishedPresetRenderProfile {
                profile_id: "preset_soft-glow-preview".into(),
                display_name: "Soft Glow Preview".into(),
                output_color_space: "sRGB".into(),
            },
            final_profile: PublishedPresetRenderProfile {
                profile_id: "preset_soft-glow-final".into(),
                display_name: "Soft Glow Final".into(),
                output_color_space: "sRGB".into(),
            },
            proxy_publication: Ok(publication()),
        }
    }

    const PINNED: &str = "5.4.1";

    /// **HV-17 `Go` 전까지 기본값은 `off`다.**
    /// 이 값이 조용히 `on`이 되면 측정되지 않은 tier가 고객 화면에 게시된다.
    #[test]
    fn the_lane_defaults_off_until_hv17_records_go() {
        assert_eq!(parse_raw_refined_lane_mode(None), RawRefinedLaneMode::Off);
        assert_eq!(
            parse_raw_refined_lane_mode(Some("off")),
            RawRefinedLaneMode::Off
        );
        assert_eq!(
            parse_raw_refined_lane_mode(Some("on")),
            RawRefinedLaneMode::On
        );
        assert_eq!(
            parse_raw_refined_lane_mode(Some(" on ")),
            RawRefinedLaneMode::On
        );
        // 오타가 고객 화면 경로를 켜는 일은 없어야 한다.
        assert_eq!(
            parse_raw_refined_lane_mode(Some("ON")),
            RawRefinedLaneMode::Off
        );
        assert_eq!(
            parse_raw_refined_lane_mode(Some("true")),
            RawRefinedLaneMode::Off
        );
        assert_eq!(
            parse_raw_refined_lane_mode(Some("enabled")),
            RawRefinedLaneMode::Off
        );
        assert_eq!(
            parse_raw_refined_lane_mode(Some("")),
            RawRefinedLaneMode::Off
        );
    }

    /// **미실행은 통과가 아니다.** AC 6의 원자료가 없으면 이 tier는 존재하지 않는다.
    #[test]
    fn an_unmeasured_tier_is_not_a_justified_tier() {
        assert_eq!(
            RAW_REFINED_TIER_JUSTIFICATION,
            TierJustification::NotMeasured
        );
        assert!(!TierJustification::NotMeasured.is_justified());
        assert!(!TierJustification::NotJustified.is_justified());
        assert!(TierJustification::Justified.is_justified());
    }

    #[test]
    fn eligibility_reports_a_unique_reason_for_every_refusal() {
        let bundle = bundle();

        assert_eq!(
            evaluate_raw_refined_eligibility(
                RawRefinedLaneMode::Off,
                TierJustification::Justified,
                true,
                Some(&bundle),
                PINNED,
            )
            .unwrap_err(),
            RawRefinedSkipReason::LaneOff
        );
        assert_eq!(
            evaluate_raw_refined_eligibility(
                RawRefinedLaneMode::On,
                TierJustification::NotMeasured,
                true,
                Some(&bundle),
                PINNED,
            )
            .unwrap_err(),
            RawRefinedSkipReason::TierNotJustified
        );
        assert_eq!(
            evaluate_raw_refined_eligibility(
                RawRefinedLaneMode::On,
                TierJustification::NotJustified,
                true,
                Some(&bundle),
                PINNED,
            )
            .unwrap_err(),
            RawRefinedSkipReason::TierNotJustified
        );
        assert_eq!(
            evaluate_raw_refined_eligibility(
                RawRefinedLaneMode::On,
                TierJustification::Justified,
                false,
                Some(&bundle),
                PINNED,
            )
            .unwrap_err(),
            RawRefinedSkipReason::ProxyNotCommitted
        );
        assert_eq!(
            evaluate_raw_refined_eligibility(
                RawRefinedLaneMode::On,
                TierJustification::Justified,
                true,
                None,
                PINNED,
            )
            .unwrap_err(),
            RawRefinedSkipReason::BundleUnavailable
        );
        assert_eq!(
            evaluate_raw_refined_eligibility(
                RawRefinedLaneMode::On,
                TierJustification::Justified,
                true,
                Some(&bundle),
                "5.4.0",
            )
            .unwrap_err(),
            RawRefinedSkipReason::ProxyIneligible(ProxyIneligibleReason::ReferenceRendererMismatch)
        );
        assert!(evaluate_raw_refined_eligibility(
            RawRefinedLaneMode::On,
            TierJustification::Justified,
            true,
            Some(&bundle),
            PINNED,
        )
        .is_ok());
    }

    /// **오늘의 제품 설정으로는 정밀본이 하나도 만들어지지 않는다.**
    /// 외부 환경 변수에 의존하지 않는 결정적 단언이다 (Story 7.3 리뷰 지적사항).
    #[test]
    fn the_shipped_configuration_produces_no_refined_artifacts() {
        let verdict = evaluate_raw_refined_eligibility(
            parse_raw_refined_lane_mode(None),
            RAW_REFINED_TIER_JUSTIFICATION,
            true,
            Some(&bundle()),
            PINNED,
        );

        assert_eq!(verdict.unwrap_err(), RawRefinedSkipReason::LaneOff);
    }

    #[test]
    fn every_skip_reason_has_a_distinct_code() {
        let codes = [
            RawRefinedSkipReason::LaneOff.as_str(),
            RawRefinedSkipReason::TierNotJustified.as_str(),
            RawRefinedSkipReason::ProxyNotCommitted.as_str(),
            RawRefinedSkipReason::ViewerContextChanged.as_str(),
            RawRefinedSkipReason::PresetUnbound.as_str(),
            RawRefinedSkipReason::BundleUnavailable.as_str(),
            RawRefinedSkipReason::SourceUnavailable.as_str(),
            RawRefinedSkipReason::SourceHashMismatch.as_str(),
            RawRefinedSkipReason::DeadlineMissed.as_str(),
            RawRefinedSkipReason::DimensionMismatch.as_str(),
            RawRefinedSkipReason::RenderCoalesced.as_str(),
            RawRefinedSkipReason::RenderCancelled.as_str(),
            RawRefinedSkipReason::ProxyIneligible(ProxyIneligibleReason::NotDeclared).as_str(),
            RawRefinedSkipReason::ProxyIneligible(ProxyIneligibleReason::NotCompatible).as_str(),
        ];
        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();

        assert_eq!(
            unique.len(),
            codes.len(),
            "조용한 무시를 만들지 않으려면 사유가 전부 달라야 한다"
        );
    }

    #[test]
    fn refined_render_rejects_a_raw_that_changed_after_proxy_commit() {
        let path = std::env::temp_dir().join(format!(
            "boothy-raw-refined-hash-{}-{}.cr2",
            std::process::id(),
            crate::viewer::current_monotonic_micros()
        ));
        fs::write(&path, b"proxy source bytes").expect("write source fixture");
        let expected_hash = content_hash(b"proxy source bytes");
        verify_source_hash(&path, &expected_hash).expect("unchanged source must pass");

        fs::write(&path, b"replaced source bytes").expect("replace source fixture");
        let result = verify_source_hash(&path, &expected_hash);
        let _ = fs::remove_file(&path);

        assert!(matches!(
            result,
            Err(RawRefinedRenderError::Skipped {
                reason: RawRefinedSkipReason::SourceHashMismatch,
                ..
            })
        ));
    }

    #[test]
    fn staging_output_stays_inside_the_session_root_and_never_collides_with_the_proxy() {
        let job = RawRefinedJob {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            request_id: "req-1",
            capture_id: "cap-1",
            source_path: Path::new("C:/session/captures/originals/a.cr2"),
            bound_session_id: Some("session_01hs6n1r8b8zc5v4ey2x7b9g1m"),
            request_viewer_epoch: 3,
            current_viewer_epoch: 3,
            capture_order: 0,
            inherited: InheritedProxyGeometry {
                proxy_generation_id: "req-1-000001".into(),
                target_width_px: 1620,
                target_height_px: 1080,
                display_profile_id: "approved-1080p".into(),
                device_pixel_ratio: 1.0,
                proxy_source_width_px: 1620,
                proxy_source_height_px: 1080,
                preset_id: "preset_soft-glow".into(),
                preset_version: "2026.08.01".into(),
                source_asset_hash: "fnv1a64:00000000000000aa".into(),
                source_route: "raw-original".into(),
            },
            lanes: DisplayLaneFlags::none(),
            deadline_micros: 5_000_000,
        };
        let path = raw_refined_staging_path(Path::new("C:/base"), &job);
        let normalized = path.to_string_lossy().replace('\\', "/");

        assert!(normalized.contains("session_01hs6n1r8b8zc5v4ey2x7b9g1m"));
        assert!(normalized.contains("/renders/display/.staging/"));
        assert!(normalized.contains("rawRefinedDisplay"));
        assert!(!normalized.contains(".boothy-darktable"));
        assert_ne!(
            path,
            proxy_render_staging_path(
                Path::new("C:/base"),
                "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
                "req-1",
                "cap-1",
                crate::contracts::dto::DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
            )
        );
    }
}
