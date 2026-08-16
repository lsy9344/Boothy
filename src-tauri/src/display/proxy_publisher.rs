//! Story 7.4: 화면 적합 immutable preset proxy lane.
//!
//! **게시 순서를 새로 만들지 않는다.** Story 7.2의 `generation_publisher`를 그대로 호출하고,
//! 이 파일은 그 앞단 — 자격 판정, source 선택, 렌더 호출, provenance 조립 — 만 소유한다.
//!
//! 이 lane이 만드는 이미지는 계측 fixture가 아니라 **실제 고객의 첫 성공 화면**이다.

use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::dto::{
    DisplayProxyProvenanceDto, HostErrorEnvelope, DISPLAY_RENDER_QUALITY_FAST,
    DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
};
use crate::display::DisplayLaneFlags;
use crate::preset::preset_bundle::{
    ProxyIneligibleReason, PublishedPresetProxyPublication, PublishedPresetRuntimeBundle,
};
use crate::render::{
    display_job_key_for_context, render_display_proxy_to_path_in_dir, DisplayProxyRenderOutcome,
    DisplayProxyRenderSpec, DisplayRenderDisposition, DisplayRenderSchedule, JobCoordinates,
    JobPriority,
};
use crate::session::session_paths::SessionPaths;

use super::display_artifact::{fits_contain_without_upscale, DisplayState};
use super::generation_publisher::{publish_generation_in_dir, PublishOutcome, PublishRequest};
use super::image_probe::{content_hash, probe_jpeg};

pub const PROXY_LANE_MODE_ENV: &str = "BOOTHY_DISPLAY_PROXY_MODE";

/// proxy lane 모드.
///
/// **기본은 `On`이다.** HV-15 `Go` 이후 승인된 정확 RAW 경로를 고객 화면에 사용한다.
/// 운영자가 명시적으로 `off`를 주거나 알 수 없는 값을 주면 안전하게 끈다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyLaneMode {
    Off,
    On,
}

impl ProxyLaneMode {
    pub fn is_enabled(self) -> bool {
        matches!(self, ProxyLaneMode::On)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ProxyLaneMode::Off => "off",
            ProxyLaneMode::On => "on",
        }
    }
}

/// 환경 변수가 없으면 승인된 제품 기본값 `On`이다.
/// 알 수 없는 명시값은 전부 `Off`다. 오타가 고객 화면 경로를 켜는 일은 없어야 한다.
pub fn parse_proxy_lane_mode(raw: Option<&str>) -> ProxyLaneMode {
    match raw.map(str::trim) {
        None => ProxyLaneMode::On,
        Some("on") => ProxyLaneMode::On,
        _ => ProxyLaneMode::Off,
    }
}

pub fn current_proxy_lane_mode() -> ProxyLaneMode {
    parse_proxy_lane_mode(std::env::var(PROXY_LANE_MODE_ENV).ok().as_deref())
}

/// proxy lane이 **시작조차 하지 않은** 이유. 전부 고유 코드이며 조용한 무시는 없다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySkipReason {
    /// lane이 운영 설정으로 꺼져 있다.
    LaneOff,
    /// Story 7.2의 fixture lane과 동시에 켜졌다. 같은 pointer를 다투면 evidence가 오염된다.
    SampleLaneConflict,
    /// viewer가 아직 photo rect를 보고하지 않았다.
    ViewerNotReady,
    /// 렌더 중 viewer 세션·epoch·물리 크기 또는 display profile이 바뀌었다.
    ViewerContextChanged,
    /// capture record에 preset 결속이 없다.
    PresetUnbound,
    /// capture-bound published bundle을 찾지 못했다.
    BundleUnavailable,
    /// bundle이 proxy lane 자격을 갖추지 못했다. **정확한 RAW 경로로 fallback한다.**
    ProxyIneligible(ProxyIneligibleReason),
    /// source 파일이 없거나 읽히지 않는다.
    SourceUnavailable,
    /// **source가 화면보다 작다.** 렌더해도 upscale 없이는 화면을 채울 수 없다.
    /// 이 판정은 렌더 **전에** 한다 — 뒤에 하면 CPU만 태우고 화면은 비어 있다.
    InsufficientSourceDimensions,
    /// Story 7.6. 같은 좌표의 렌더가 이미 진행 중이라 병합됐다. 두 번 렌더하지 않는다.
    RenderCoalesced,
    /// Story 7.6. 삭제·세션 교체·epoch 변경으로 취소됐다.
    /// **RAW·preview·final·현재 화면 truth는 그대로다.**
    RenderCancelled,
}

impl ProxySkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ProxySkipReason::LaneOff => "proxy-lane-off",
            ProxySkipReason::SampleLaneConflict => "proxy-sample-lane-conflict",
            ProxySkipReason::ViewerNotReady => "proxy-viewer-not-ready",
            ProxySkipReason::ViewerContextChanged => "proxy-viewer-context-changed",
            ProxySkipReason::PresetUnbound => "proxy-preset-unbound",
            ProxySkipReason::BundleUnavailable => "proxy-bundle-unavailable",
            ProxySkipReason::ProxyIneligible(reason) => reason.as_str(),
            ProxySkipReason::SourceUnavailable => "proxy-source-unavailable",
            ProxySkipReason::InsufficientSourceDimensions => "proxy-insufficient-source-dimensions",
            ProxySkipReason::RenderCoalesced => "proxy-render-coalesced",
            ProxySkipReason::RenderCancelled => "proxy-render-cancelled",
        }
    }
}

/// proxy가 렌더할 source. Story 7.3의 route 결정이 이 값을 정한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySourceRoute {
    /// 오늘의 정확 경로. **HV-14 route 결정 전의 기본값이다.**
    RawOriginal,
    /// Story 7.3이 승인한 fast source (`embedded-jpeg` / `camera-paired-jpeg` / `windows-shell-thumbnail`).
    ApprovedFastSource(&'static str),
}

impl ProxySourceRoute {
    pub fn as_str(self) -> &'static str {
        match self {
            ProxySourceRoute::RawOriginal => "raw-original",
            ProxySourceRoute::ApprovedFastSource(route) => route,
        }
    }

    fn is_raw_original(self) -> bool {
        matches!(self, ProxySourceRoute::RawOriginal)
    }
}

/// source가 화면 크기를 만족하는지에 대한 **렌더 전** 판정.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFitVerdict {
    Sufficient {
        width_px: u32,
        height_px: u32,
    },
    Insufficient {
        width_px: u32,
        height_px: u32,
    },
    /// RAW 원본처럼 값싸게 픽셀 크기를 알 수 없는 source.
    ///
    /// 이 경우에만 렌더 뒤의 크기 gate에 판정을 맡긴다. 임의로 "충분하다"고 가정하지 않고,
    /// 판정을 하지 않았다는 사실 자체를 기록한다.
    Unknown,
}

/// raster source의 실제 픽셀 크기로 display fit을 미리 판정한다.
///
/// **1600px Windows Shell 썸네일이 4K profile을 통과하지 못하는 것을 여기서 잡는다.**
/// `--upscale false`이므로 작은 source는 작은 결과를 만들고, 그 결과는 어차피 거부된다.
pub fn evaluate_source_display_fit(
    route: ProxySourceRoute,
    source_bytes: &[u8],
    required_source_width_px: u32,
    required_source_height_px: u32,
) -> SourceFitVerdict {
    if route.is_raw_original() {
        return SourceFitVerdict::Unknown;
    }

    match probe_jpeg(source_bytes) {
        Ok(probe) => {
            if fits_contain_without_upscale(
                probe.width_px,
                probe.height_px,
                required_source_width_px,
                required_source_height_px,
            ) {
                SourceFitVerdict::Sufficient {
                    width_px: probe.width_px,
                    height_px: probe.height_px,
                }
            } else {
                SourceFitVerdict::Insufficient {
                    width_px: probe.width_px,
                    height_px: probe.height_px,
                }
            }
        }
        // 구조를 읽지 못하는 source는 크기를 주장할 수 없다. 렌더 뒤 gate가 판정한다.
        Err(_) => SourceFitVerdict::Unknown,
    }
}

/// 한 촬영의 display proxy 작업 명세.
///
/// 전부 **촬영이 확정된 시점**에 고정된 값이다. 렌더가 3.5초 걸려도 이 값들은 흔들리지 않는다.
#[derive(Debug, Clone)]
pub struct DisplayProxyJob<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub capture_id: &'a str,
    pub source_path: &'a Path,
    pub route: ProxySourceRoute,
    pub bound_session_id: Option<&'a str>,
    pub request_viewer_epoch: u64,
    pub current_viewer_epoch: u64,
    pub capture_order: u64,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
    pub display_profile_id: &'a str,
    pub device_pixel_ratio: f64,
    pub lanes: DisplayLaneFlags,
    /// Story 7.6. **trusted capture input 시각 + NFR-003 hard max(5초).**
    ///
    /// 같은 우선순위 안의 순서 결정과 `deadline-missed` 관측에만 쓴다.
    /// deadline이 지났다는 이유로 P0를 시작하지 않거나 중단하지 않는다 —
    /// 오늘 실측이 8초대이므로 그렇게 하면 화면에 아무것도 뜨지 않는다.
    pub deadline_micros: u64,
}

impl DisplayProxyJob<'_> {
    /// 스케줄러가 취소 범위를 판정할 때 보는 좌표.
    ///
    /// 이 값이 비면 그 작업은 삭제·세션 교체·epoch 변경 어느 것으로도 취소되지 않는다.
    pub fn scheduler_coordinates(&self) -> JobCoordinates {
        JobCoordinates {
            session_id: self.session_id.to_string(),
            request_id: self.request_id.to_string(),
            capture_id: Some(self.capture_id.to_string()),
            capture_order: Some(self.capture_order),
            viewer_epoch: self.request_viewer_epoch,
        }
    }
}

/// proxy 렌더 산출물의 staging 경로.
///
/// **session root 안**이다. NFR-004는 0 tolerance이고, 세션 정리가 이 경로를 함께 지운다.
/// `.boothy-darktable/display-proxy/`에는 darktable config/library만 남고 고객 이미지는 없다.
///
/// Story 7.6이 경로에 **generation 좌표를 넣었다.** 이전에는 `<captureId>-proxy-render.jpg`
/// 하나뿐이라, 같은 촬영의 두 작업이 동시에 돌면 서로의 staging 파일을 덮어썼다
/// (`deferred-work.md`의 미결 항목). P0와 P1이 함께 도는 지금은 그 확률이 더 올라간다.
pub fn proxy_render_staging_path(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    tier: &str,
) -> PathBuf {
    SessionPaths::new(base_dir, session_id)
        .session_root
        .join("renders")
        .join("display")
        .join(".staging")
        .join(format!("{request_id}-{capture_id}-{tier}-render.jpg"))
}

/// lane 자격을 먼저 판정한다. **렌더도 파일 생성도 하지 않는다.**
///
/// 자격의 근거는 **source route에 따라 달라진다.**
///
/// source route와 무관하게 게시 bundle의 명시적 `proxyPublication` 승인을 요구한다.
/// 내장 preset도 같은 규칙을 따르며, 승인 없는 bundle을 RAW 경로라는 이유로 우회시키지 않는다.
pub fn evaluate_proxy_eligibility(
    lane_mode: ProxyLaneMode,
    sample_lane_enabled: bool,
    bundle: Option<&PublishedPresetRuntimeBundle>,
    _route: ProxySourceRoute,
    pinned_renderer_version: &str,
    required_source_width_px: u32,
    required_source_height_px: u32,
) -> Result<PublishedPresetProxyPublication, ProxySkipReason> {
    if !lane_mode.is_enabled() {
        return Err(ProxySkipReason::LaneOff);
    }

    if sample_lane_enabled {
        return Err(ProxySkipReason::SampleLaneConflict);
    }

    if required_source_width_px == 0 || required_source_height_px == 0 {
        return Err(ProxySkipReason::ViewerNotReady);
    }

    let bundle = bundle.ok_or(ProxySkipReason::BundleUnavailable)?;

    let publication = bundle
        .proxy_publication
        .clone()
        .map_err(ProxySkipReason::ProxyIneligible)?;

    if publication.reference_renderer_version != pinned_renderer_version {
        return Err(ProxySkipReason::ProxyIneligible(
            ProxyIneligibleReason::ReferenceRendererMismatch,
        ));
    }

    Ok(publication)
}

/// 렌더하고 게시한다. 게시 순서 자체는 `generation_publisher`가 소유한다.
///
/// 호출자는 `DisplayState` mutex를 잡은 채로 부르지 **않는다** — 렌더가 수 초 걸리므로
/// 그 사이 pointer 조회가 막히면 관람 화면이 멈춘다. 이 함수가 렌더를 먼저 끝내고
/// 게시 직전에만 state를 받는다.
#[allow(clippy::too_many_arguments)]
pub fn render_display_proxy(
    base_dir: &Path,
    job: &DisplayProxyJob<'_>,
    bundle: &PublishedPresetRuntimeBundle,
    publication: &PublishedPresetProxyPublication,
    now: &dyn Fn() -> u64,
) -> Result<RenderedDisplayProxy, ProxyRenderError> {
    let source_bytes = fs::read(job.source_path).map_err(|error| ProxyRenderError::Skipped {
        reason: ProxySkipReason::SourceUnavailable,
        detail: format!(
            "proxy source를 읽지 못했어요: path={} error={error}",
            job.source_path.to_string_lossy()
        ),
    })?;

    let fit = evaluate_source_display_fit(
        job.route,
        &source_bytes,
        job.required_source_width_px,
        job.required_source_height_px,
    );

    if let SourceFitVerdict::Insufficient {
        width_px,
        height_px,
    } = fit
    {
        return Err(ProxyRenderError::Skipped {
            reason: ProxySkipReason::InsufficientSourceDimensions,
            detail: format!(
                "source가 화면보다 작아 렌더 전에 중단했어요: source={width_px}x{height_px} required={}x{} route={}",
                job.required_source_width_px,
                job.required_source_height_px,
                job.route.as_str()
            ),
        });
    }

    let source_asset_hash = content_hash(&source_bytes);
    // 큰 RAW를 메모리에 계속 들고 있을 이유가 없다. 해시와 크기 판정이 끝나면 놓는다.
    drop(source_bytes);

    let output_path = proxy_render_staging_path(
        base_dir,
        job.session_id,
        job.request_id,
        job.capture_id,
        DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
    );
    let spec = DisplayProxyRenderSpec {
        source_asset_path: job.source_path,
        xmp_template_path: &publication.proxy_recipe_path,
        output_path: &output_path,
        target_width_px: job.required_source_width_px,
        target_height_px: job.required_source_height_px,
        output_color_space: &publication.output_color_space,
        jpeg_quality: publication.jpeg_quality,
        icc_intent: &publication.icc_intent,
        source_is_raw_original: job.route.is_raw_original(),
        schedule: DisplayRenderSchedule {
            // 고객의 첫 성공 화면이다. 이보다 급한 렌더는 없다.
            priority: JobPriority::P0CurrentProxy,
            job_key: display_job_key_for_context(
                job.session_id,
                job.request_id,
                job.capture_id,
                DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
                &bundle.preset_id,
                &bundle.published_version,
                job.request_viewer_epoch,
                job.required_source_width_px,
                job.required_source_height_px,
                job.display_profile_id,
                job.device_pixel_ratio,
            ),
            deadline_micros: job.deadline_micros,
            coordinates: job.scheduler_coordinates(),
        },
    };

    let render = match render_display_proxy_to_path_in_dir(base_dir, &spec, now)
        .map_err(|error| ProxyRenderError::Render(Box::new(error)))?
    {
        DisplayRenderDisposition::Rendered(outcome) => *outcome,
        DisplayRenderDisposition::Coalesced { job_key, .. } => {
            return Err(ProxyRenderError::Skipped {
                reason: ProxySkipReason::RenderCoalesced,
                detail: format!("같은 좌표의 proxy 렌더가 이미 진행 중이에요: jobKey={job_key}"),
            })
        }
        DisplayRenderDisposition::Cancelled { reason, .. } => {
            // 취소는 로그와 span에만 남는다. 고객 화면에는 나타나지 않는다.
            let _ = fs::remove_file(&output_path);

            return Err(ProxyRenderError::Skipped {
                reason: ProxySkipReason::RenderCancelled,
                detail: format!("proxy 렌더가 취소됐어요: reason={reason}"),
            });
        }
    };

    let rendered_bytes = fs::read(&output_path).map_err(|error| {
        // 산출물을 다시 읽지 못하면 그 staging 파일은 다음 시도의 "성공"으로 오인될 수 있다.
        let _ = fs::remove_file(&output_path);

        ProxyRenderError::Skipped {
            reason: ProxySkipReason::SourceUnavailable,
            detail: format!("proxy 산출물을 다시 읽지 못했어요: {error}"),
        }
    })?;

    Ok(RenderedDisplayProxy {
        bytes: rendered_bytes,
        staging_path: output_path,
        render,
        provenance: DisplayProxyProvenanceDto {
            preset_id: bundle.preset_id.clone(),
            preset_version: bundle.published_version.clone(),
            approval_basis: publication.basis.as_str().into(),
            proxy_recipe_version: publication.proxy_recipe_version.clone(),
            reference_renderer: publication.reference_renderer.clone(),
            reference_renderer_version: publication.reference_renderer_version.clone(),
            render_profile_id: bundle.preview_profile.profile_id.clone(),
            output_color_space: publication.output_color_space.clone(),
            jpeg_quality: publication.jpeg_quality,
            source_route: job.route.as_str().into(),
            source_asset_hash,
            target_width_px: job.required_source_width_px,
            target_height_px: job.required_source_height_px,
            display_profile_id: job.display_profile_id.into(),
            device_pixel_ratio: job.device_pixel_ratio,
            // 이 lane은 참조 렌더러(darktable)가 직접 만든 프레임이다. producer가 따로 없다.
            resident_provenance: None,
            // Story 7.6. proxy lane은 `--hq false`다. 이 값은 Story 7.4가 승인받은 렌더 인자를
            // 그대로 서술한 것이며, 바꾸면 HV-15 `Go`의 근거가 흔들린다.
            render_quality: DISPLAY_RENDER_QUALITY_FAST.into(),
        },
        source_fit: fit,
    })
}

#[derive(Debug)]
pub enum ProxyRenderError {
    /// lane이 정당하게 중단됐다. 오류가 아니라 정상 결과이며 진단에 남는다.
    Skipped {
        reason: ProxySkipReason,
        detail: String,
    },
    /// 렌더 자체가 실패했다. **RAW·preview·final truth는 건드리지 않는다.**
    Render(Box<crate::render::RenderWorkerError>),
}

#[derive(Debug)]
pub struct RenderedDisplayProxy {
    pub bytes: Vec<u8>,
    pub staging_path: PathBuf,
    pub render: DisplayProxyRenderOutcome,
    pub provenance: DisplayProxyProvenanceDto,
    pub source_fit: SourceFitVerdict,
}

/// 렌더가 끝난 proxy를 Story 7.2의 게시 순서에 태운다.
///
/// `current_viewer_epoch`는 **게시 시점에 다시 읽은 값**을 받는다. job에 실린 값을 쓰면
/// 렌더 시작 시점의 epoch를 게시 시점에 재사용하게 되어, 렌더 도중(수 초) 관람 창이
/// 재생성된 경우를 `stale-epoch`로 잡아내지 못한다 (`deferred-work.md` 미결 항목).
pub fn publish_rendered_proxy_in_dir(
    base_dir: &Path,
    state: &mut DisplayState,
    job: &DisplayProxyJob<'_>,
    rendered: &RenderedDisplayProxy,
    current_viewer_epoch: u64,
    now: &dyn Fn() -> u64,
) -> Result<PublishOutcome, HostErrorEnvelope> {
    let request = PublishRequest {
        session_id: job.session_id,
        request_id: job.request_id,
        capture_id: Some(job.capture_id),
        tier: DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
        sample_variant: None,
        proxy_provenance: Some(&rendered.provenance),
        source_bytes: &rendered.bytes,
        bound_session_id: job.bound_session_id,
        request_viewer_epoch: job.request_viewer_epoch,
        current_viewer_epoch,
        capture_order: Some(job.capture_order),
        required_source_width_px: job.required_source_width_px,
        required_source_height_px: job.required_source_height_px,
        lanes: job.lanes,
        // proxy tier에는 AC 6 gate가 적용되지 않는다. 이 값은 정밀본 tier에서만 읽힌다.
        refined_tier_justified: false,
    };

    let outcome = publish_generation_in_dir(base_dir, state, &request, now);

    // 게시 성공/실패와 무관하게 렌더 staging 산출물은 남기지 않는다.
    // 확정 자산은 `generation_repository`가 자기 staging에서 승격한 별도 파일이다.
    let _ = fs::remove_file(&rendered.staging_path);

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::preset_bundle::PublishedPresetRenderProfile;
    use std::path::PathBuf;

    fn publication() -> PublishedPresetProxyPublication {
        PublishedPresetProxyPublication {
            basis: crate::preset::preset_bundle::ProxyEligibilityBasis::ApprovedVisualParity,
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

    fn bundle(
        proxy: Result<PublishedPresetProxyPublication, ProxyIneligibleReason>,
    ) -> PublishedPresetRuntimeBundle {
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
            proxy_publication: proxy,
        }
    }

    #[test]
    fn missing_mode_defaults_on_and_unknown_mode_stays_off() {
        assert_eq!(parse_proxy_lane_mode(None), ProxyLaneMode::On);
        assert_eq!(parse_proxy_lane_mode(Some("")), ProxyLaneMode::Off);
        assert_eq!(parse_proxy_lane_mode(Some("true")), ProxyLaneMode::Off);
        assert_eq!(parse_proxy_lane_mode(Some("ON")), ProxyLaneMode::Off);
        assert_eq!(parse_proxy_lane_mode(Some("enabled")), ProxyLaneMode::Off);
        assert_eq!(parse_proxy_lane_mode(Some("off")), ProxyLaneMode::Off);
        assert_eq!(parse_proxy_lane_mode(Some("on")), ProxyLaneMode::On);
        assert_eq!(parse_proxy_lane_mode(Some(" on ")), ProxyLaneMode::On);
    }

    #[test]
    fn lane_defaults_on_after_hv15_records_go() {
        assert_eq!(parse_proxy_lane_mode(None), ProxyLaneMode::On);
        assert!(ProxyLaneMode::On.is_enabled());
    }

    /// 현재 제품 경로. RAW도 명시적 publication 승인을 통과해야 한다.
    const RAW: ProxySourceRoute = ProxySourceRoute::RawOriginal;
    /// 승인이 필요한 경로. source가 정확 경로와 다르다.
    const FAST: ProxySourceRoute = ProxySourceRoute::ApprovedFastSource("embedded-jpeg");
    const PINNED: &str = "5.4.1";

    #[test]
    fn eligibility_reports_a_unique_reason_for_every_refusal() {
        let eligible = bundle(Ok(publication()));

        assert_eq!(
            evaluate_proxy_eligibility(
                ProxyLaneMode::Off,
                false,
                Some(&eligible),
                FAST,
                PINNED,
                1620,
                1080
            )
            .unwrap_err(),
            ProxySkipReason::LaneOff
        );
        assert_eq!(
            evaluate_proxy_eligibility(
                ProxyLaneMode::On,
                true,
                Some(&eligible),
                FAST,
                PINNED,
                1620,
                1080
            )
            .unwrap_err(),
            ProxySkipReason::SampleLaneConflict
        );
        assert_eq!(
            evaluate_proxy_eligibility(
                ProxyLaneMode::On,
                false,
                Some(&eligible),
                FAST,
                PINNED,
                0,
                1080
            )
            .unwrap_err(),
            ProxySkipReason::ViewerNotReady
        );
        assert_eq!(
            evaluate_proxy_eligibility(ProxyLaneMode::On, false, None, FAST, PINNED, 1620, 1080)
                .unwrap_err(),
            ProxySkipReason::BundleUnavailable
        );
        assert_eq!(
            evaluate_proxy_eligibility(
                ProxyLaneMode::On,
                false,
                Some(&bundle(Err(ProxyIneligibleReason::NotDeclared))),
                FAST,
                PINNED,
                1620,
                1080
            )
            .unwrap_err(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::NotDeclared)
        );
        assert!(evaluate_proxy_eligibility(
            ProxyLaneMode::On,
            false,
            Some(&eligible),
            FAST,
            PINNED,
            1620,
            1080
        )
        .is_ok());
    }

    #[test]
    fn a_fast_source_without_visual_approval_is_refused() {
        // source가 정확 경로와 다르면 결과가 RAW 정밀본과 달라질 수 있다.
        // 명시적 승인 없이는 고객 화면 경로에 들어가지 않는다.
        let reason = evaluate_proxy_eligibility(
            ProxyLaneMode::On,
            false,
            Some(&bundle(Err(ProxyIneligibleReason::NotDeclared))),
            FAST,
            PINNED,
            1620,
            1080,
        )
        .unwrap_err();

        assert_eq!(reason.as_str(), "proxy-not-declared");
    }

    #[test]
    fn a_raw_source_without_explicit_publication_approval_is_refused() {
        let reason = evaluate_proxy_eligibility(
            ProxyLaneMode::On,
            false,
            Some(&bundle(Err(ProxyIneligibleReason::NotDeclared))),
            RAW,
            PINNED,
            1620,
            1080,
        )
        .unwrap_err();

        assert_eq!(reason.as_str(), "proxy-not-declared");
    }

    #[test]
    fn raster_sources_smaller_than_the_screen_are_caught_before_rendering() {
        // 800x533 JPEG. 1080p profile(1620x1080)을 만족하지 못한다.
        let small = jpeg_fixture(800, 533);

        assert_eq!(
            evaluate_source_display_fit(
                ProxySourceRoute::ApprovedFastSource("windows-shell-thumbnail"),
                &small,
                1620,
                1080
            ),
            SourceFitVerdict::Insufficient {
                width_px: 800,
                height_px: 533
            }
        );
    }

    #[test]
    fn raster_sources_large_enough_pass_the_pre_render_gate() {
        let large = jpeg_fixture(5184, 3456);

        assert_eq!(
            evaluate_source_display_fit(
                ProxySourceRoute::ApprovedFastSource("embedded-jpeg"),
                &large,
                1620,
                1080
            ),
            SourceFitVerdict::Sufficient {
                width_px: 5184,
                height_px: 3456
            }
        );
    }

    #[test]
    fn portrait_raster_sources_can_use_contain_without_upscaling() {
        let portrait = jpeg_fixture(720, 1080);

        assert_eq!(
            evaluate_source_display_fit(
                ProxySourceRoute::ApprovedFastSource("embedded-jpeg"),
                &portrait,
                1620,
                1080
            ),
            SourceFitVerdict::Sufficient {
                width_px: 720,
                height_px: 1080
            }
        );
    }

    #[test]
    fn raw_originals_defer_the_size_verdict_instead_of_assuming_success() {
        // RAW의 센서 크기를 값싸게 알 수 없다. "충분하다"고 가정하지 않고 판정을 미룬다.
        assert_eq!(
            evaluate_source_display_fit(ProxySourceRoute::RawOriginal, &[0xFF, 0xD8], 1620, 1080),
            SourceFitVerdict::Unknown
        );
    }

    #[test]
    fn unreadable_raster_sources_defer_rather_than_claim_a_size() {
        assert_eq!(
            evaluate_source_display_fit(
                ProxySourceRoute::ApprovedFastSource("embedded-jpeg"),
                b"not a jpeg at all",
                1620,
                1080
            ),
            SourceFitVerdict::Unknown
        );
    }

    #[test]
    fn staging_output_stays_inside_the_session_root() {
        let path = proxy_render_staging_path(
            Path::new("C:/base"),
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "req-1",
            "cap-1",
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
        );
        let normalized = path.to_string_lossy().replace('\\', "/");

        assert!(normalized.contains("session_01hs6n1r8b8zc5v4ey2x7b9g1m"));
        assert!(normalized.contains("/renders/display/.staging/"));
        assert!(!normalized.contains(".boothy-darktable"));
    }

    /// Story 7.6. **한 촬영의 두 tier가 같은 staging 파일을 다투면 안 된다.**
    ///
    /// P0와 P1이 함께 도는 지금, 이전의 `<captureId>-proxy-render.jpg` 하나로는
    /// 두 작업이 서로의 중간 산출물을 덮어쓴다 (`deferred-work.md` 미결 항목).
    #[test]
    fn staging_paths_never_collide_between_concurrent_jobs() {
        let base = Path::new("C:/base");
        let session = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
        let proxy = proxy_render_staging_path(
            base,
            session,
            "req-1",
            "cap-1",
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
        );
        let refined = proxy_render_staging_path(
            base,
            session,
            "req-1",
            "cap-1",
            crate::contracts::dto::DISPLAY_TIER_RAW_REFINED_DISPLAY,
        );
        let other_request = proxy_render_staging_path(
            base,
            session,
            "req-2",
            "cap-1",
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
        );

        assert_ne!(proxy, refined, "두 tier가 같은 staging 파일을 쓰면 안 된다");
        assert_ne!(
            proxy, other_request,
            "다른 request가 같은 staging 파일을 쓰면 안 된다"
        );
    }

    #[test]
    fn every_skip_reason_has_a_distinct_code() {
        let codes = [
            ProxySkipReason::LaneOff.as_str(),
            ProxySkipReason::SampleLaneConflict.as_str(),
            ProxySkipReason::ViewerNotReady.as_str(),
            ProxySkipReason::ViewerContextChanged.as_str(),
            ProxySkipReason::PresetUnbound.as_str(),
            ProxySkipReason::BundleUnavailable.as_str(),
            ProxySkipReason::SourceUnavailable.as_str(),
            ProxySkipReason::InsufficientSourceDimensions.as_str(),
            ProxySkipReason::RenderCoalesced.as_str(),
            ProxySkipReason::RenderCancelled.as_str(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::NotDeclared).as_str(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::NotCompatible).as_str(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::IncompleteMetadata).as_str(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::RecipeUnresolvable).as_str(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::ReferenceRendererMismatch)
                .as_str(),
            ProxySkipReason::ProxyIneligible(ProxyIneligibleReason::VisualApprovalMissing).as_str(),
        ];
        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();

        assert_eq!(
            unique.len(),
            codes.len(),
            "조용한 무시를 만들지 않으려면 사유가 전부 달라야 한다"
        );
    }

    /// 실제 JPEG 인코더 없이 구조만 갖춘 baseline JPEG을 만든다.
    /// `probe_jpeg`가 보는 것은 SOI / SOF 크기 / EOI뿐이다.
    fn jpeg_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        bytes.extend_from_slice(b"JFIF\0");
        bytes.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
        // SOF0
        bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        bytes.extend_from_slice(&(height as u16).to_be_bytes());
        bytes.extend_from_slice(&(width as u16).to_be_bytes());
        bytes.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
        bytes.extend_from_slice(&[0xFF, 0xD9]);
        bytes
    }
}
