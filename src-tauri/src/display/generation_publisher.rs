//! immutable display generation 게시의 **유일한** 순서 구현.
//!
//! Story 7.2가 계측용 fixture로 이 순서를 증명했고, Story 7.4의 display-fit preset proxy가
//! **같은 함수**를 호출한다. 두 번째 게시 경로를 만들면 한쪽이 조용히 어긋나고,
//! 7.2가 증명한 표시 종단점은 더 이상 같은 종단점이 아니게 된다.
//!
//! `staging write → flush → sync_all → close → 구조 probe → 크기/correlation 검증
//!  → rename → pointer rename → journal append → (호출자가) notify`

use std::path::Path;

use crate::contracts::dto::{
    display_tier_required_render_quality, DisplayGenerationDto, DisplayPointerSnapshotDto,
    DisplayProxyProvenanceDto, HostErrorEnvelope, DISPLAY_PROXY_PATH_VARIANT,
    DISPLAY_RAW_REFINED_PATH_VARIANT, DISPLAY_REJECT_ORIENTATION_UNSUPPORTED,
    DISPLAY_REJECT_PARTIAL_FILE, DISPLAY_REJECT_SESSION_MISMATCH, DISPLAY_REJECT_UNDECODABLE,
    DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY, DISPLAY_TIER_RAW_REFINED_DISPLAY, DISPLAY_TIER_SAMPLE,
    VIEWER_DISPLAY_SCHEMA_VERSION,
};
use crate::display::DisplayLaneFlags;

use super::display_artifact::{
    evaluate_admission, AdmissionContext, DisplayState, GenerationCandidate, PresetBinding,
};
use super::generation_repository::{
    append_journal, discard_staged, generation_id, generation_path, promote_generation,
    stage_generation, write_pointer, write_reconciled_pointer, DisplayJournalRecord,
    StageGenerationError,
};
use super::image_probe::ProbeError;

#[derive(Debug, Clone)]
pub struct PublishRequest<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub capture_id: Option<&'a str>,
    /// `DISPLAY_TIER_SAMPLE` / `DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY` /
    /// `DISPLAY_TIER_RAW_REFINED_DISPLAY`.
    pub tier: &'a str,
    /// 계측 fixture 전용. proxy·정밀본에서는 `None`이어야 한다.
    pub sample_variant: Option<&'a str>,
    /// proxy·정밀본 전용. sample에서는 `None`이어야 한다.
    pub proxy_provenance: Option<&'a DisplayProxyProvenanceDto>,
    pub source_bytes: &'a [u8],
    /// host가 소유한 현재 세션. 다르면 `session-mismatch`다.
    pub bound_session_id: Option<&'a str>,
    /// **request가 시작될 때** 관측한 viewer 세대.
    ///
    /// 게시는 request 수락 후 수 초 뒤에 일어날 수 있고, 그 사이 관람 창이 재생성되면
    /// 이 자산은 죽은 viewer 세대에 속한다. 그래서 현재 epoch와 따로 들고 다닌다.
    pub request_viewer_epoch: u64,
    /// 게시 시점에 읽은 현재 viewer 세대.
    pub current_viewer_epoch: u64,
    /// **촬영 시점에 고정한** capture 순서 좌표. 게시 시점 좌표가 아니다.
    pub capture_order: Option<u64>,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
    /// snapshot에 실을 lane 플래그. 환경 변수를 이 함수 안에서 읽지 않는다 (테스트 결정성).
    pub lanes: DisplayLaneFlags,
    /// Story 7.6. AC 6의 detail 축 gate 판정. **정밀본 tier에서만 의미가 있다.**
    ///
    /// `false`면 게시하지 않고 `refined-tier-not-justified`로 journal에 남는다.
    /// proxy·sample 요청에서는 무시된다.
    pub refined_tier_justified: bool,
}

impl PublishRequest<'_> {
    /// 확정 파일 경로의 variant 자리. 빈 문자열이 되지 않는다.
    ///
    /// **tier마다 다른 값을 준다.** 한 촬영에 proxy와 정밀본이 함께 존재하므로
    /// 두 확정 파일 이름이 사람과 스크립트 눈에 구분되어야 한다.
    fn path_variant(&self) -> &str {
        match self.sample_variant {
            Some(variant) => variant,
            None if self.tier == DISPLAY_TIER_RAW_REFINED_DISPLAY => {
                DISPLAY_RAW_REFINED_PATH_VARIANT
            }
            None => DISPLAY_PROXY_PATH_VARIANT,
        }
    }

    fn preset_binding(&self) -> Option<PresetBinding<'_>> {
        self.proxy_provenance.map(|provenance| PresetBinding {
            preset_id: provenance.preset_id.as_str(),
            preset_version: provenance.preset_version.as_str(),
        })
    }

    /// tier마다 필요한 필드가 다르다. TS의 `superRefine`과 같은 불변식을 host에서도 강제한다.
    fn validate_shape(&self) -> Result<(), HostErrorEnvelope> {
        let coherent = match self.tier {
            DISPLAY_TIER_SAMPLE => {
                matches!(self.sample_variant, Some("a") | Some("b"))
                    && self.proxy_provenance.is_none()
            }
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY | DISPLAY_TIER_RAW_REFINED_DISPLAY => {
                // Story 7.6. tier와 렌더 품질이 어긋난 generation은 evidence에서 거짓말을 한다.
                // `--hq false`로 만든 프레임이 정밀본으로 게시되면 AC 6의 측정 자체가 무의미해진다.
                let quality_matches = self
                    .proxy_provenance
                    .zip(display_tier_required_render_quality(self.tier))
                    .is_some_and(|(provenance, required)| provenance.render_quality == required);

                self.sample_variant.is_none() && quality_matches
            }
            _ => false,
        };

        if coherent {
            Ok(())
        } else {
            Err(HostErrorEnvelope::validation_message(
                "표시 자산 게시 값을 다시 확인해 주세요.",
            ))
        }
    }
}

/// 게시 결과. **거부는 오류가 아니라 정상 결과**이며 journal에 남는다.
#[derive(Debug, Clone)]
pub struct PublishOutcome {
    pub pointer: DisplayPointerSnapshotDto,
    pub committed: Option<DisplayGenerationDto>,
    pub reject_reason: Option<String>,
    /// close + fsync가 끝난 시각 (진단 span).
    pub file_ready_at_micros: Option<u64>,
    /// 구조 probe 통과 시각 (진단 span).
    pub probe_ok_at_micros: Option<u64>,
    /// pointer가 commit된 시각 (진단 span).
    pub pointer_committed_at_micros: Option<u64>,
}

fn probe_error_reason(error: ProbeError) -> &'static str {
    match error {
        ProbeError::PartialFile => DISPLAY_REJECT_PARTIAL_FILE,
        ProbeError::Undecodable => DISPLAY_REJECT_UNDECODABLE,
        ProbeError::OrientationUnsupported => DISPLAY_REJECT_ORIENTATION_UNSUPPORTED,
    }
}

fn journal_rejection(
    base_dir: &Path,
    request: &PublishRequest<'_>,
    generation_seq: u64,
    reason: &str,
    now_micros: u64,
) -> Result<(), HostErrorEnvelope> {
    let record = DisplayJournalRecord {
        schema_version: VIEWER_DISPLAY_SCHEMA_VERSION.into(),
        outcome: "rejected".into(),
        reject_reason: Some(reason.into()),
        session_id: request.session_id.into(),
        request_id: request.request_id.into(),
        generation_seq,
        generation: None,
        recorded_at_host_micros: now_micros,
    };

    append_journal(base_dir, request.session_id, &record)
}

/// 게시 전 과정. 순서를 지키는 것이 이 함수의 유일한 책임이다.
///
/// `now` 는 host monotonic micros를 돌려주는 clock이다. 테스트에서 결정적 값을 주입한다.
pub fn publish_generation_in_dir(
    base_dir: &Path,
    state: &mut DisplayState,
    request: &PublishRequest<'_>,
    now: &dyn Fn() -> u64,
) -> Result<PublishOutcome, HostErrorEnvelope> {
    request.validate_shape()?;

    let previous_session_id = state.session_id().map(str::to_string);
    let reconciled = state.reconcile(
        request.bound_session_id,
        request.required_source_width_px,
        request.required_source_height_px,
    );

    if reconciled {
        let reconciled_at_micros = now();
        let snapshot = state.snapshot(
            request.required_source_width_px,
            request.required_source_height_px,
            request.lanes,
            reconciled_at_micros,
        );
        write_reconciled_pointer(base_dir, previous_session_id.as_deref(), &snapshot)?;
    }

    let build_outcome =
        |state: &DisplayState, reason: Option<String>, now_micros: u64| PublishOutcome {
            pointer: state.snapshot(
                request.required_source_width_px,
                request.required_source_height_px,
                request.lanes,
                now_micros,
            ),
            committed: None,
            reject_reason: reason,
            file_ready_at_micros: None,
            probe_ok_at_micros: None,
            pointer_committed_at_micros: None,
        };

    // 세션이 다르면 파일을 만들지도 않는다. 다른 세션의 디스크 경로를 건드리지 않기 위해서다.
    if request.bound_session_id != Some(request.session_id) {
        let now_micros = now();
        journal_rejection(
            base_dir,
            request,
            0,
            DISPLAY_REJECT_SESSION_MISMATCH,
            now_micros,
        )?;

        return Ok(build_outcome(
            state,
            Some(DISPLAY_REJECT_SESSION_MISMATCH.into()),
            now_micros,
        ));
    }

    let (generation_seq, request_order) = state.reserve_generation(request.request_id);

    // 촬영 시점 좌표를 상태에 남긴다. 이 값이 없으면 commit 뒤에 다음 후보와
    // 촬영 순서를 비교할 근거가 사라져, 먼저 찍고 늦게 끝난 사진이 현재 사진을 덮는다.
    if let (Some(capture_id), Some(capture_order)) = (request.capture_id, request.capture_order) {
        state.record_capture_order(capture_id, capture_order);
    }

    // 1~2단계: 완전히 쓰고 닫은 뒤 구조 probe.
    let staged = match stage_generation(
        base_dir,
        request.session_id,
        request.request_id,
        generation_seq,
        request.source_bytes,
        now,
    ) {
        Ok(staged) => staged,
        Err(StageGenerationError::Probe(error)) => {
            let now_micros = now();
            let reason = probe_error_reason(error);
            journal_rejection(base_dir, request, generation_seq, reason, now_micros)?;

            return Ok(build_outcome(state, Some(reason.into()), now_micros));
        }
        Err(StageGenerationError::Persistence(error)) => return Err(error),
    };

    let file_ready_at_micros = staged.file_ready_at_micros;
    let probe_ok_at_micros = staged.probe_ok_at_micros;

    // 3~5단계: 크기와 correlation 검증.
    let candidate = GenerationCandidate {
        session_id: request.session_id,
        request_id: request.request_id,
        capture_id: request.capture_id,
        viewer_epoch: request.request_viewer_epoch,
        tier: request.tier,
        generation_seq,
        source_width_px: staged.probe.width_px,
        source_height_px: staged.probe.height_px,
        request_order,
        capture_order: request.capture_order,
        preset_binding: request.preset_binding(),
        refined_tier_justified: request.refined_tier_justified,
    };
    let context = AdmissionContext {
        bound_session_id: request.bound_session_id,
        viewer_epoch: request.current_viewer_epoch,
        required_source_width_px: request.required_source_width_px,
        required_source_height_px: request.required_source_height_px,
    };

    if let Err(reason) = evaluate_admission(state.active_display(), &candidate, &context) {
        discard_staged(&staged);
        let now_micros = now();
        journal_rejection(base_dir, request, generation_seq, reason, now_micros)?;

        return Ok(build_outcome(state, Some(reason.into()), now_micros));
    }

    // 6단계: 확정 경로로 승격. 충돌은 하드 에러다.
    let final_path = generation_path(
        base_dir,
        request.session_id,
        request.request_id,
        generation_seq,
        request.path_variant(),
    )?;

    if let Err(error) = promote_generation(&staged, &final_path) {
        discard_staged(&staged);
        return Err(error);
    }

    let committed_at_host_micros = now();
    let generation = DisplayGenerationDto {
        generation_id: generation_id(request.request_id, generation_seq),
        generation_seq,
        request_order: Some(request_order),
        capture_order: request.capture_order,
        session_id: request.session_id.into(),
        request_id: request.request_id.into(),
        capture_id: request.capture_id.map(str::to_string),
        viewer_epoch: request.request_viewer_epoch,
        tier: request.tier.into(),
        asset_path: final_path.to_string_lossy().to_string(),
        source_width_px: staged.probe.width_px,
        source_height_px: staged.probe.height_px,
        byte_size: staged.byte_size,
        source_hash: staged.source_hash.clone(),
        sample_variant: request.sample_variant.map(str::to_string),
        proxy_provenance: request.proxy_provenance.cloned(),
        committed_at_host_micros,
    };

    // 7단계: pointer commit. 메모리와 디스크가 어긋나면 안전한 쪽(표시 없음)으로 되돌린다.
    state.commit_generation(generation.clone());

    let pointer = state.snapshot(
        request.required_source_width_px,
        request.required_source_height_px,
        request.lanes,
        committed_at_host_micros,
    );

    if let Err(error) = write_pointer(base_dir, request.session_id, &pointer) {
        state.mark_poisoned(&generation.generation_id);
        log::error!(
            "display_pointer_write_failed session={} generation={} error={}",
            request.session_id,
            generation.generation_id,
            error.message
        );

        return Err(error);
    }

    // 8단계: journal append. 여기까지 끝난 뒤에야 호출자가 notify한다.
    let record = DisplayJournalRecord {
        schema_version: VIEWER_DISPLAY_SCHEMA_VERSION.into(),
        outcome: "committed".into(),
        reject_reason: None,
        session_id: request.session_id.into(),
        request_id: request.request_id.into(),
        generation_seq,
        generation: Some(generation.clone()),
        recorded_at_host_micros: committed_at_host_micros,
    };

    if let Err(journal_error) = append_journal(base_dir, request.session_id, &record) {
        // 감사 기록 없는 generation은 고객 화면의 truth로 남길 수 없다.
        state.mark_poisoned(&generation.generation_id);
        let rollback_at_micros = now();
        let rollback_pointer = state.snapshot(
            request.required_source_width_px,
            request.required_source_height_px,
            request.lanes,
            rollback_at_micros,
        );

        if let Err(pointer_error) = write_pointer(base_dir, request.session_id, &rollback_pointer) {
            log::error!(
                "display_journal_rollback_failed session={} generation={} journal_error={} pointer_error={}",
                request.session_id,
                generation.generation_id,
                journal_error.message,
                pointer_error.message
            );
            return Err(pointer_error);
        }

        return Err(journal_error);
    }

    Ok(PublishOutcome {
        pointer,
        committed: Some(generation),
        reject_reason: None,
        file_ready_at_micros: Some(file_ready_at_micros),
        probe_ok_at_micros: Some(probe_ok_at_micros),
        pointer_committed_at_micros: Some(committed_at_host_micros),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contracts::dto::{DISPLAY_RENDER_QUALITY_FAST, DISPLAY_RENDER_QUALITY_HIGH};

    fn provenance() -> DisplayProxyProvenanceDto {
        DisplayProxyProvenanceDto {
            preset_id: "preset_soft_glow".into(),
            preset_version: "2026.08.01".into(),
            approval_basis: "exact-reference-renderer".into(),
            proxy_recipe_version: "1".into(),
            reference_renderer: "darktable".into(),
            reference_renderer_version: "5.4.1".into(),
            render_profile_id: "preview".into(),
            output_color_space: "sRGB".into(),
            jpeg_quality: 92,
            source_route: "raw-original".into(),
            source_asset_hash: "fnv1a64:0000000000000001".into(),
            target_width_px: 1620,
            target_height_px: 1080,
            display_profile_id: "approved-1080p".into(),
            device_pixel_ratio: 1.0,
            resident_provenance: None,
            render_quality: DISPLAY_RENDER_QUALITY_FAST.into(),
        }
    }

    /// Story 7.6. 정밀본 tier의 provenance. `--hq true`로 만든 프레임이다.
    fn refined_provenance() -> DisplayProxyProvenanceDto {
        DisplayProxyProvenanceDto {
            render_quality: DISPLAY_RENDER_QUALITY_HIGH.into(),
            ..provenance()
        }
    }

    fn request<'a>(
        tier: &'a str,
        sample_variant: Option<&'a str>,
        proxy_provenance: Option<&'a DisplayProxyProvenanceDto>,
    ) -> PublishRequest<'a> {
        PublishRequest {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            request_id: "req-1",
            capture_id: Some("cap-1"),
            tier,
            sample_variant,
            proxy_provenance,
            source_bytes: &[],
            bound_session_id: Some("session_01hs6n1r8b8zc5v4ey2x7b9g1m"),
            request_viewer_epoch: 1,
            current_viewer_epoch: 1,
            capture_order: Some(0),
            required_source_width_px: 1620,
            required_source_height_px: 1080,
            lanes: DisplayLaneFlags::none(),
            refined_tier_justified: true,
        }
    }

    #[test]
    fn proxy_generations_use_a_readable_path_variant() {
        let provenance = provenance();
        let proxy = request(
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
            None,
            Some(&provenance),
        );

        assert_eq!(proxy.path_variant(), DISPLAY_PROXY_PATH_VARIANT);
        assert!(!proxy.path_variant().is_empty());
    }

    /// 한 촬영에 proxy와 정밀본이 함께 존재한다. 확정 파일 이름이 서로 달라야
    /// 사람도 evidence 스크립트도 어느 tier의 산출물인지 읽을 수 있다.
    #[test]
    fn refined_generations_use_their_own_path_variant() {
        let provenance = refined_provenance();
        let refined = request(DISPLAY_TIER_RAW_REFINED_DISPLAY, None, Some(&provenance));

        assert_eq!(refined.path_variant(), DISPLAY_RAW_REFINED_PATH_VARIANT);
        assert_ne!(refined.path_variant(), DISPLAY_PROXY_PATH_VARIANT);
        assert!(!refined.path_variant().is_empty());
    }

    #[test]
    fn sample_generations_keep_their_fixture_variant_in_the_path() {
        let sample = request(DISPLAY_TIER_SAMPLE, Some("b"), None);

        assert_eq!(sample.path_variant(), "b");
    }

    #[test]
    fn tier_specific_fields_are_enforced_before_any_file_is_written() {
        let provenance = provenance();

        // 올바른 조합만 통과한다.
        assert!(request(DISPLAY_TIER_SAMPLE, Some("a"), None)
            .validate_shape()
            .is_ok());
        assert!(request(
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
            None,
            Some(&provenance)
        )
        .validate_shape()
        .is_ok());

        // fixture에 preset provenance가 실리면 거짓 출처가 evidence에 남는다.
        assert!(request(DISPLAY_TIER_SAMPLE, Some("a"), Some(&provenance))
            .validate_shape()
            .is_err());
        // provenance 없는 proxy는 "이 사진이 어느 촬영·프리셋의 결과인가"를 증명할 수 없다.
        assert!(request(DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY, None, None)
            .validate_shape()
            .is_err());
        // proxy는 계측 fixture가 아니다.
        assert!(request(
            DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
            Some("a"),
            Some(&provenance)
        )
        .validate_shape()
        .is_err());
        // 등록되지 않은 tier는 게시 자체가 되지 않는다.
        assert!(request("final", None, Some(&provenance))
            .validate_shape()
            .is_err());
    }

    /// Story 7.6. **렌더 품질과 tier가 어긋난 generation은 파일이 만들어지기 전에 막는다.**
    ///
    /// 두 tier는 같은 RAW·같은 XMP·같은 renderer·같은 목표 크기를 쓰고 `--hq`만 다르다.
    /// `--hq false` 프레임이 정밀본으로 게시되면 evidence는 "정밀본이 떴다"고 적고,
    /// AC 6의 tier 정당성 측정이 통째로 무의미해진다.
    #[test]
    fn a_generation_whose_render_quality_contradicts_its_tier_never_reaches_disk() {
        let fast = provenance();
        let high = refined_provenance();

        assert!(
            request(DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY, None, Some(&fast))
                .validate_shape()
                .is_ok()
        );
        assert!(request(DISPLAY_TIER_RAW_REFINED_DISPLAY, None, Some(&high))
            .validate_shape()
            .is_ok());

        // `--hq true` 프레임을 proxy tier로 게시할 수 없다.
        assert!(
            request(DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY, None, Some(&high))
                .validate_shape()
                .is_err()
        );
        // `--hq false` 프레임을 정밀본 tier로 게시할 수 없다.
        assert!(request(DISPLAY_TIER_RAW_REFINED_DISPLAY, None, Some(&fast))
            .validate_shape()
            .is_err());
        // provenance 없는 정밀본은 어느 렌더 품질인지 주장할 근거가 없다.
        assert!(request(DISPLAY_TIER_RAW_REFINED_DISPLAY, None, None)
            .validate_shape()
            .is_err());
    }

    #[test]
    fn preset_binding_comes_from_proxy_provenance_only() {
        let provenance = provenance();

        assert_eq!(
            request(DISPLAY_TIER_SAMPLE, Some("a"), None).preset_binding(),
            None
        );
        assert_eq!(
            request(
                DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
                None,
                Some(&provenance)
            )
            .preset_binding(),
            Some(PresetBinding {
                preset_id: "preset_soft_glow",
                preset_version: "2026.08.01",
            })
        );
    }
}
