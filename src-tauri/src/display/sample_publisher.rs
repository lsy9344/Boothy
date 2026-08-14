//! Story 7.2: 계측용 sample generation 게시 orchestration.
//!
//! **이 lane은 계측 도구이지 제품 경로가 아니다.** 표시되는 이미지는 preset 적용 결과가 아니라
//! fixture이므로 기본값은 `off`이며, 켠 채로 출시하면 실제 고객이 fixture 사진을 본다.
//!
//! Story 7.4는 여기의 fixture 읽기만 실제 display-fit preset proxy 렌더로 교체하면 된다.
//! generation/pointer/journal 계약은 그대로 재사용된다.

use std::path::{Path, PathBuf};

use crate::contracts::dto::{
    DisplayGenerationDto, DisplayPointerSnapshotDto, HostErrorEnvelope,
    DISPLAY_REJECT_ORIENTATION_UNSUPPORTED, DISPLAY_REJECT_PARTIAL_FILE,
    DISPLAY_REJECT_UNDECODABLE, DISPLAY_TIER_SAMPLE, VIEWER_DISPLAY_SCHEMA_VERSION,
};

use super::display_artifact::{
    evaluate_admission, AdmissionContext, DisplayState, GenerationCandidate,
};
use super::generation_repository::{
    append_journal, discard_staged, generation_id, generation_path, promote_generation,
    stage_generation, write_pointer, write_reconciled_pointer, DisplayJournalRecord,
    StageGenerationError,
};
use super::image_probe::ProbeError;

pub const SAMPLE_LANE_MODE_ENV: &str = "BOOTHY_DISPLAY_SAMPLE_MODE";
pub const SAMPLE_LANE_GAP_ENV: &str = "BOOTHY_DISPLAY_SAMPLE_GAP_MS";
pub const DEFAULT_SAMPLE_GAP_MS: u64 = 800;

/// 계측 lane 모드. 기본은 반드시 `Off`다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleLaneMode {
    Off,
    VisibleStandby,
    HiddenPrewarm,
}

impl SampleLaneMode {
    pub fn is_enabled(self) -> bool {
        !matches!(self, SampleLaneMode::Off)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SampleLaneMode::Off => "off",
            SampleLaneMode::VisibleStandby => "visible-standby",
            SampleLaneMode::HiddenPrewarm => "hidden-prewarm",
        }
    }
}

/// 알 수 없는 값은 전부 `Off`로 떨어뜨린다. 오타가 lane을 켜는 일은 없어야 한다.
pub fn parse_sample_lane_mode(raw: Option<&str>) -> SampleLaneMode {
    match raw.map(str::trim) {
        Some("visible-standby") => SampleLaneMode::VisibleStandby,
        Some("hidden-prewarm") => SampleLaneMode::HiddenPrewarm,
        _ => SampleLaneMode::Off,
    }
}

pub fn current_sample_lane_mode() -> SampleLaneMode {
    parse_sample_lane_mode(std::env::var(SAMPLE_LANE_MODE_ENV).ok().as_deref())
}

pub fn parse_sample_gap_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value <= 60_000)
        .unwrap_or(DEFAULT_SAMPLE_GAP_MS)
}

pub fn current_sample_gap_ms() -> u64 {
    parse_sample_gap_ms(std::env::var(SAMPLE_LANE_GAP_ENV).ok().as_deref())
}

/// 번들된 sample fixture 경로. `bundle.resources`로 배포된다.
pub fn sample_fixture_path(resource_dir: &Path, variant: &str) -> PathBuf {
    resource_dir
        .join("fixtures")
        .join("display-sample")
        .join(format!("sample-{variant}.jpg"))
}

#[derive(Debug, Clone)]
pub struct PublishRequest<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub capture_id: Option<&'a str>,
    pub sample_variant: &'a str,
    pub source_bytes: &'a [u8],
    /// host가 소유한 현재 세션. 다르면 `session-mismatch`다.
    pub bound_session_id: Option<&'a str>,
    /// **request가 시작될 때** 관측한 viewer 세대.
    ///
    /// 게시는 request 수락 후 수백 ms 뒤에 일어날 수 있고, 그 사이 관람 창이 재생성되면
    /// 이 자산은 죽은 viewer 세대에 속한다. 그래서 현재 epoch와 따로 들고 다닌다.
    pub request_viewer_epoch: u64,
    /// 게시 시점에 읽은 현재 viewer 세대.
    pub current_viewer_epoch: u64,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
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
            current_sample_lane_mode().is_enabled(),
            reconciled_at_micros,
        );
        write_reconciled_pointer(base_dir, previous_session_id.as_deref(), &snapshot)?;
    }

    let build_outcome =
        |state: &DisplayState, reason: Option<String>, now_micros: u64| PublishOutcome {
            pointer: state.snapshot(
                request.required_source_width_px,
                request.required_source_height_px,
                current_sample_lane_mode().is_enabled(),
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
            crate::contracts::dto::DISPLAY_REJECT_SESSION_MISMATCH,
            now_micros,
        )?;

        return Ok(build_outcome(
            state,
            Some(crate::contracts::dto::DISPLAY_REJECT_SESSION_MISMATCH.into()),
            now_micros,
        ));
    }

    let (generation_seq, request_order) = state.reserve_generation(request.request_id);

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
        viewer_epoch: request.request_viewer_epoch,
        tier: DISPLAY_TIER_SAMPLE,
        generation_seq,
        source_width_px: staged.probe.width_px,
        source_height_px: staged.probe.height_px,
        request_order,
    };
    let context = AdmissionContext {
        bound_session_id: request.bound_session_id,
        viewer_epoch: request.current_viewer_epoch,
        required_source_width_px: request.required_source_width_px,
        required_source_height_px: request.required_source_height_px,
    };

    if let Err(reason) = evaluate_admission(
        state.active_generation(),
        state.active_request_order(),
        &candidate,
        &context,
    ) {
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
        request.sample_variant,
    )?;

    if let Err(error) = promote_generation(&staged, &final_path) {
        discard_staged(&staged);
        return Err(error);
    }

    let committed_at_host_micros = now();
    let generation = DisplayGenerationDto {
        generation_id: generation_id(request.request_id, generation_seq),
        generation_seq,
        session_id: request.session_id.into(),
        request_id: request.request_id.into(),
        capture_id: request.capture_id.map(str::to_string),
        viewer_epoch: request.request_viewer_epoch,
        tier: DISPLAY_TIER_SAMPLE.into(),
        asset_path: final_path.to_string_lossy().to_string(),
        source_width_px: staged.probe.width_px,
        source_height_px: staged.probe.height_px,
        byte_size: staged.byte_size,
        source_hash: staged.source_hash.clone(),
        sample_variant: request.sample_variant.into(),
        committed_at_host_micros,
    };

    // 7단계: pointer commit. 메모리와 디스크가 어긋나면 안전한 쪽(표시 없음)으로 되돌린다.
    state.commit_generation(generation.clone());

    let pointer = state.snapshot(
        request.required_source_width_px,
        request.required_source_height_px,
        current_sample_lane_mode().is_enabled(),
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
            current_sample_lane_mode().is_enabled(),
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

    #[test]
    fn unknown_or_missing_mode_stays_off() {
        assert_eq!(parse_sample_lane_mode(None), SampleLaneMode::Off);
        assert_eq!(parse_sample_lane_mode(Some("")), SampleLaneMode::Off);
        assert_eq!(parse_sample_lane_mode(Some("on")), SampleLaneMode::Off);
        assert_eq!(parse_sample_lane_mode(Some("true")), SampleLaneMode::Off);
        assert_eq!(
            parse_sample_lane_mode(Some("VISIBLE-STANDBY")),
            SampleLaneMode::Off
        );
    }

    #[test]
    fn known_modes_enable_the_lane() {
        assert_eq!(
            parse_sample_lane_mode(Some("visible-standby")),
            SampleLaneMode::VisibleStandby
        );
        assert_eq!(
            parse_sample_lane_mode(Some(" hidden-prewarm ")),
            SampleLaneMode::HiddenPrewarm
        );
        assert!(SampleLaneMode::VisibleStandby.is_enabled());
        assert!(SampleLaneMode::HiddenPrewarm.is_enabled());
        assert!(!SampleLaneMode::Off.is_enabled());
    }

    #[test]
    fn sample_gap_falls_back_to_default() {
        assert_eq!(parse_sample_gap_ms(None), DEFAULT_SAMPLE_GAP_MS);
        assert_eq!(parse_sample_gap_ms(Some("abc")), DEFAULT_SAMPLE_GAP_MS);
        assert_eq!(parse_sample_gap_ms(Some("999999")), DEFAULT_SAMPLE_GAP_MS);
        assert_eq!(parse_sample_gap_ms(Some("250")), 250);
    }

    #[test]
    fn fixture_path_is_resource_relative() {
        let path = sample_fixture_path(Path::new("C:/app/resources"), "b");

        assert!(
            path.ends_with("fixtures/display-sample/sample-b.jpg".replace('/', "\\"))
                || path.ends_with("fixtures/display-sample/sample-b.jpg")
        );
    }
}
