//! Story 7.2: immutable display generation의 승격 규칙과 pointer 상태.
//!
//! 정확성은 전송 순서가 아니라 여기의 monotonic guard에서 나온다. 늦게 도착하거나 중복된
//! 게시가 현재 화면을 되돌리지 못하게 하는 판정을 전부 순수 함수로 분리해 둔다.

use std::collections::{BTreeMap, BTreeSet};

use crate::contracts::dto::{
    display_tier_order, DisplayGenerationDto, DisplayPointerSnapshotDto,
    DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS, DISPLAY_REJECT_LOWER_GENERATION,
    DISPLAY_REJECT_OLDER_CAPTURE, DISPLAY_REJECT_OLDER_REQUEST, DISPLAY_REJECT_PRESET_MISMATCH,
    DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH, DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED,
    DISPLAY_REJECT_SESSION_MISMATCH, DISPLAY_REJECT_STALE_EPOCH, DISPLAY_REJECT_TIER_DOWNGRADE,
    DISPLAY_REJECT_UNKNOWN_GENERATION, DISPLAY_REJECT_VIEWER_NOT_READY,
    DISPLAY_TIER_RAW_REFINED_DISPLAY, VIEWER_DISPLAY_SCHEMA_VERSION,
};
use crate::display::DisplayLaneFlags;

/// 승격 판정에 필요한 host 측 문맥. 전부 host가 소유한 truth다.
#[derive(Debug, Clone, Copy)]
pub struct AdmissionContext<'a> {
    pub bound_session_id: Option<&'a str>,
    pub viewer_epoch: u64,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
}

/// 후보의 preset 결속. Story 7.4의 proxy generation만 값을 갖는다.
///
/// sample fixture는 preset과 무관하므로 `None`이고, 그때는 preset 판정을 건너뛴다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetBinding<'a> {
    pub preset_id: &'a str,
    pub preset_version: &'a str,
}

/// 아직 활성화되지 않은 후보 generation.
#[derive(Debug, Clone, Copy)]
pub struct GenerationCandidate<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub capture_id: Option<&'a str>,
    pub viewer_epoch: u64,
    pub tier: &'a str,
    pub generation_seq: u64,
    pub source_width_px: u32,
    pub source_height_px: u32,
    /// 이 request가 처음 관측된 순서. 값이 작을수록 오래된 request다.
    pub request_order: u64,
    /// 이 request 안에서 capture가 처음 관측된 순서. 값이 작을수록 오래된 capture다.
    pub capture_order: Option<u64>,
    pub preset_binding: Option<PresetBinding<'a>>,
    /// Story 7.6. AC 6의 detail 축(MTF50) gate 판정.
    ///
    /// `rawRefinedDisplay` 후보에서만 의미가 있다. **측정되지 않은 tier 차이는 게시하지 않는다** —
    /// 두 결과가 측정 한계 안에서 같으면 교체는 연출일 뿐이고, 고객에게 아무 가치가 없으면서
    /// darktable 부하만 두 배가 된다. proxy/sample 후보에서는 이 값이 무시된다.
    pub refined_tier_justified: bool,
}

/// `object-fit: contain`으로 표시할 때 픽셀 확대가 필요하지 않은지 판정한다.
/// 한 축이 목표 경계에 닿으면 다른 축은 letterbox 여백으로 남는다.
pub fn fits_contain_without_upscale(
    source_width_px: u32,
    source_height_px: u32,
    required_source_width_px: u32,
    required_source_height_px: u32,
) -> bool {
    source_width_px > 0
        && source_height_px > 0
        && required_source_width_px > 0
        && required_source_height_px > 0
        && (source_width_px >= required_source_width_px
            || source_height_px >= required_source_height_px)
}

/// generation에 실린 preset 결속을 읽는다. sample generation은 `None`이다.
fn generation_preset_binding(generation: &DisplayGenerationDto) -> Option<PresetBinding<'_>> {
    generation
        .proxy_provenance
        .as_ref()
        .map(|provenance| PresetBinding {
            preset_id: provenance.preset_id.as_str(),
            preset_version: provenance.preset_version.as_str(),
        })
}

/// 현재 활성 display truth와 그 순서 좌표.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveDisplay<'a> {
    pub generation: Option<&'a DisplayGenerationDto>,
    /// 활성 generation이 속한 request의 관측 순서.
    pub request_order: Option<u64>,
    /// 활성 generation이 속한 capture의 관측 순서.
    pub capture_order: Option<u64>,
}

/// 현재 활성 generation과 비교해 후보를 승격할 수 있는지 판정한다.
///
/// 각 거부 경로는 고유 reason을 돌려준다. 조용한 무시는 허용하지 않는다.
pub fn evaluate_admission(
    active: ActiveDisplay<'_>,
    candidate: &GenerationCandidate<'_>,
    context: &AdmissionContext<'_>,
) -> Result<(), &'static str> {
    let current = active.generation;
    let current_request_order = active.request_order;
    if context.bound_session_id != Some(candidate.session_id) {
        return Err(DISPLAY_REJECT_SESSION_MISMATCH);
    }

    if candidate.viewer_epoch != context.viewer_epoch {
        return Err(DISPLAY_REJECT_STALE_EPOCH);
    }

    if display_tier_order(candidate.tier).is_none() {
        return Err(DISPLAY_REJECT_UNKNOWN_GENERATION);
    }

    if context.required_source_width_px == 0 || context.required_source_height_px == 0 {
        return Err(DISPLAY_REJECT_VIEWER_NOT_READY);
    }

    if !fits_contain_without_upscale(
        candidate.source_width_px,
        candidate.source_height_px,
        context.required_source_width_px,
        context.required_source_height_px,
    ) {
        return Err(DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS);
    }

    let is_refined_candidate = candidate.tier == DISPLAY_TIER_RAW_REFINED_DISPLAY;

    // Story 7.6. **측정되지 않은 tier 차이는 게시하지 않는다** (AC 6).
    // 파일을 이미 만들었더라도 여기서 멈춘다 — 화면에는 proxy가 그대로 남는다.
    if is_refined_candidate && !candidate.refined_tier_justified {
        return Err(DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED);
    }

    let Some(current) = current else {
        // Story 7.6 / UX-DR19. 정밀본은 **승급**이지 첫 성공 화면의 대체가 아니다.
        // proxy 없이 정밀본만 뜨면 고객의 첫 화면이 3배 느려지고, 맞출 geometry도 없다.
        if is_refined_candidate {
            return Err(DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH);
        }

        return Ok(());
    };

    // 현재 활성 generation과 후보가 **같은 촬영**인가.
    // capture 좌표가 없는 표본(계측 fixture, v1 generation)은 오늘의 동작을 그대로 유지한다.
    let same_capture = match (current.capture_id.as_deref(), candidate.capture_id) {
        (Some(current_capture), Some(candidate_capture)) => current_capture == candidate_capture,
        _ => true,
    };

    let current_order = display_tier_order(&current.tier).unwrap_or(0);
    let candidate_order = display_tier_order(candidate.tier).unwrap_or(0);

    // **tier 비교는 같은 촬영 안에서만 의미가 있다.**
    //
    // Story 7.6이 `rawRefinedDisplay`(2)를 추가하면서 이 구분이 필수가 됐다. 촬영 A가 정밀본까지
    // 올라간 뒤 촬영 B의 proxy(1)가 도착하면, tier만 비교할 경우 **고객의 다음 사진이 통째로
    // 거부된다.** 다른 촬영의 낮은 tier는 하락이 아니라 새 사진이며, 순서는 아래의
    // capture/request 좌표가 판정한다.
    if same_capture && candidate_order < current_order {
        return Err(DISPLAY_REJECT_TIER_DOWNGRADE);
    }

    // 한 촬영에는 capture-bound preset이 정확히 하나다. 같은 촬영의 다른 preset/version이
    // 화면을 갈아치우면 고객은 자기 사진의 룩이 바뀌는 것을 본다.
    // catalog rollback(Story 4.4) 뒤 재렌더가 이 경로에 도달할 수 있다.
    if let (Some(candidate_preset), Some(current_preset)) =
        (candidate.preset_binding, generation_preset_binding(current))
    {
        let bound_to_same_capture =
            current.capture_id.is_some() && current.capture_id.as_deref() == candidate.capture_id;

        if bound_to_same_capture && candidate_preset != current_preset {
            return Err(DISPLAY_REJECT_PRESET_MISMATCH);
        }
    }

    // **렌더가 길면 완료 순서가 촬영 순서와 어긋난다.**
    // `generation_seq`와 `request_order`는 게시 시점에 부여되므로, 먼저 시작해 늦게 끝난
    // proxy가 둘 다 더 큰 값을 들고 도착한다. 촬영 시점에 고정한 좌표로만 이 경우를 막을 수 있다.
    if let (Some(candidate_capture_order), Some(current_capture_order)) =
        (candidate.capture_order, active.capture_order)
    {
        if candidate_capture_order < current_capture_order {
            return Err(DISPLAY_REJECT_OLDER_CAPTURE);
        }
    }

    // 오래된 request가 더 높은 seq를 들고 뒤늦게 도착할 수 있다.
    // seq만으로는 이 경우를 막지 못하므로 request 관측 순서를 따로 본다.
    if current.request_id != candidate.request_id {
        if let Some(current_request_order) = current_request_order {
            if candidate.request_order < current_request_order {
                return Err(DISPLAY_REJECT_OLDER_REQUEST);
            }
        }
    }

    // Story 7.6. **정밀본은 활성 proxy와 픽셀 크기가 정확히 같을 때만 승급한다.**
    //
    // 이것이 AC 4의 crop/scale 점프 0을 만드는 기계적 장치다. 크기가 한 픽셀이라도 다르면
    // `object-fit: contain` 박스가 달라져 교체 순간 사진이 튄다. 같은 촬영이 아니면
    // 화면에 맞출 proxy 자체가 없다는 뜻이므로 같은 사유로 거부한다 —
    // 순서 문제는 위에서 이미 걸러졌고, 여기 남는 것은 "붙일 곳이 없는 정밀본"뿐이다.
    if is_refined_candidate {
        let geometry_matches = current.capture_id.is_some()
            && current.capture_id.as_deref() == candidate.capture_id
            && current.source_width_px == candidate.source_width_px
            && current.source_height_px == candidate.source_height_px;

        if !geometry_matches {
            return Err(DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH);
        }
    }

    if candidate.generation_seq <= current.generation_seq {
        return Err(DISPLAY_REJECT_LOWER_GENERATION);
    }

    Ok(())
}

/// host가 소유하는 display pointer 상태.
///
/// pointer 갱신은 이 구조체를 감싼 하나의 Mutex 안에서만 일어난다 (single writer).
#[derive(Debug, Default)]
pub struct DisplayState {
    session_id: Option<String>,
    revision: u64,
    active: Option<DisplayGenerationDto>,
    next_generation_seq: u64,
    request_order: BTreeMap<String, u64>,
    next_request_order: u64,
    /// **촬영 시점**에 고정하는 순서 좌표. 게시 시점 좌표(`request_order`)와 달리
    /// 렌더 완료 순서가 뒤바뀌어도 흔들리지 않는다.
    capture_order: BTreeMap<String, u64>,
    next_capture_order: u64,
    poisoned_generation_ids: BTreeSet<String>,
}

impl DisplayState {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn active_generation(&self) -> Option<&DisplayGenerationDto> {
        self.active.as_ref()
    }

    pub fn is_poisoned(&self, generation_id: &str) -> bool {
        self.poisoned_generation_ids.contains(generation_id)
    }

    fn bump(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn clear_active(&mut self) -> bool {
        if self.active.is_none() {
            return false;
        }

        self.active = None;
        self.bump();
        true
    }

    /// 세션이 바뀌면 이전 세션의 표시 상태를 전부 버린다. NFR-004는 0 tolerance다.
    pub fn bind_session(&mut self, session_id: &str) -> bool {
        if self.session_id.as_deref() == Some(session_id) {
            return false;
        }

        self.session_id = Some(session_id.to_string());
        self.active = None;
        self.next_generation_seq = 0;
        self.request_order.clear();
        self.next_request_order = 0;
        self.capture_order.clear();
        self.next_capture_order = 0;
        self.poisoned_generation_ids.clear();
        self.bump();
        true
    }

    pub fn clear_session(&mut self) -> bool {
        if self.session_id.is_none() && self.active.is_none() {
            return false;
        }

        self.session_id = None;
        self.active = None;
        self.next_generation_seq = 0;
        self.request_order.clear();
        self.next_request_order = 0;
        self.capture_order.clear();
        self.next_capture_order = 0;
        self.poisoned_generation_ids.clear();
        self.bump();
        true
    }

    /// 현재 viewer truth와 pointer를 맞춘다. 호출자는 게시/조회 전에 반드시 부른다.
    ///
    /// - 세션이 바뀌었으면 pointer를 비운다.
    /// - viewer epoch가 바뀌어 `requiredSource*`가 커졌으면 활성 generation을 다시 판정한다.
    ///   이전 epoch에서 통과한 크기 검증을 그대로 상속하지 않는다.
    pub fn reconcile(
        &mut self,
        bound_session_id: Option<&str>,
        required_source_width_px: u32,
        required_source_height_px: u32,
    ) -> bool {
        let mut changed = match bound_session_id {
            Some(session_id) => self.bind_session(session_id),
            None => self.clear_session(),
        };

        if let Some(active) = self.active.as_ref() {
            let session_mismatch = bound_session_id != Some(active.session_id.as_str());
            let viewer_unmeasured = required_source_width_px == 0 || required_source_height_px == 0;
            let insufficient = !fits_contain_without_upscale(
                active.source_width_px,
                active.source_height_px,
                required_source_width_px,
                required_source_height_px,
            );

            if session_mismatch || viewer_unmeasured || insufficient {
                changed |= self.clear_active();
            }
        }

        changed
    }

    /// 게시 시작 시점에 seq와 request 순서를 예약한다.
    /// 거부되어도 seq는 소비된다 — 단조성만 지키면 되고 조밀할 필요는 없다.
    pub fn reserve_generation(&mut self, request_id: &str) -> (u64, u64) {
        self.next_generation_seq = self.next_generation_seq.saturating_add(1);

        let request_order = match self.request_order.get(request_id) {
            Some(order) => *order,
            None => {
                let order = self.next_request_order;
                self.next_request_order = self.next_request_order.saturating_add(1);
                self.request_order.insert(request_id.to_string(), order);
                order
            }
        };

        (self.next_generation_seq, request_order)
    }

    pub fn request_order(&self, request_id: &str) -> Option<u64> {
        self.request_order.get(request_id).copied()
    }

    pub fn active_request_order(&self) -> Option<u64> {
        self.active
            .as_ref()
            .and_then(|active| self.request_order(&active.request_id))
    }

    /// **촬영이 확정된 시점에** 이 capture의 순서 좌표를 고정한다. 이미 있으면 그 값을 돌려준다.
    ///
    /// 게시 시점이 아니라 촬영 시점이어야 한다. 3.5초짜리 렌더가 끼면 두 시점의 순서가 갈린다.
    pub fn observe_capture(&mut self, capture_id: &str) -> u64 {
        match self.capture_order.get(capture_id) {
            Some(order) => *order,
            None => {
                let order = self.next_capture_order;
                self.next_capture_order = self.next_capture_order.saturating_add(1);
                self.capture_order.insert(capture_id.to_string(), order);
                order
            }
        }
    }

    /// 게시 요청이 들고 온 촬영 시점 좌표를 상태에 남긴다.
    ///
    /// 이것이 없으면 commit 뒤에 `active_capture_order()`가 `None`이 되어,
    /// **다음 후보와 촬영 순서를 비교할 근거 자체가 사라진다.**
    /// 먼저 관측된 값이 이긴다 — 좌표는 촬영 시점에 한 번 정해지고 흔들리지 않는다.
    pub fn record_capture_order(&mut self, capture_id: &str, order: u64) {
        self.capture_order
            .entry(capture_id.to_string())
            .or_insert(order);
        self.next_capture_order = self.next_capture_order.max(order.saturating_add(1));
    }

    pub fn capture_order(&self, capture_id: &str) -> Option<u64> {
        self.capture_order.get(capture_id).copied()
    }

    pub fn active_capture_order(&self) -> Option<u64> {
        self.active
            .as_ref()
            .and_then(|active| active.capture_id.as_deref())
            .and_then(|capture_id| self.capture_order(capture_id))
    }

    /// 현재 활성 display truth와 그 순서 좌표를 한 번에 읽는다.
    pub fn active_display(&self) -> ActiveDisplay<'_> {
        ActiveDisplay {
            generation: self.active.as_ref(),
            request_order: self.active_request_order(),
            capture_order: self.active_capture_order(),
        }
    }

    /// 검증을 모두 통과한 generation을 활성 truth로 승격한다.
    pub fn commit_generation(&mut self, generation: DisplayGenerationDto) {
        self.active = Some(generation);
        self.bump();
    }

    /// viewer가 픽셀 decode에 실패했다고 보고한 generation은 다시 활성화되지 않는다.
    pub fn mark_poisoned(&mut self, generation_id: &str) -> bool {
        self.poisoned_generation_ids
            .insert(generation_id.to_string());

        let is_active = self
            .active
            .as_ref()
            .is_some_and(|active| active.generation_id == generation_id);

        if is_active {
            return self.clear_active();
        }

        false
    }

    /// capture 삭제 시 해당 request의 표시 상태를 함께 정리한다.
    pub fn forget_request(&mut self, request_id: &str) -> bool {
        let is_active = self
            .active
            .as_ref()
            .is_some_and(|active| active.request_id == request_id);

        if is_active {
            return self.clear_active();
        }

        false
    }

    pub fn snapshot(
        &self,
        required_source_width_px: u32,
        required_source_height_px: u32,
        lanes: DisplayLaneFlags,
        observed_at_host_micros: u64,
    ) -> DisplayPointerSnapshotDto {
        DisplayPointerSnapshotDto {
            schema_version: VIEWER_DISPLAY_SCHEMA_VERSION.into(),
            session_id: self.session_id.clone(),
            revision: self.revision,
            active_generation: self.active.clone(),
            required_source_width_px,
            required_source_height_px,
            measurement_lane_enabled: lanes.measurement_lane_enabled,
            present_telemetry_enabled: lanes.present_telemetry_enabled,
            observed_at_host_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dto::DISPLAY_TIER_SAMPLE;

    fn generation(
        request_id: &str,
        generation_seq: u64,
        width: u32,
        height: u32,
    ) -> DisplayGenerationDto {
        DisplayGenerationDto {
            generation_id: format!("{request_id}-{generation_seq:06}"),
            generation_seq,
            request_order: Some(0),
            capture_order: None,
            session_id: "session_01hzzzzzzzzzzzzzzzzzzzzzzz".into(),
            request_id: request_id.into(),
            capture_id: None,
            viewer_epoch: 3,
            tier: "sample".into(),
            asset_path: "C:/tmp/a.jpg".into(),
            source_width_px: width,
            source_height_px: height,
            byte_size: 1024,
            source_hash: "fnv1a64:0000000000000000".into(),
            sample_variant: Some("a".into()),
            proxy_provenance: None,
            committed_at_host_micros: 1_000,
        }
    }

    fn context<'a>(session_id: &'a str) -> AdmissionContext<'a> {
        AdmissionContext {
            bound_session_id: Some(session_id),
            viewer_epoch: 3,
            required_source_width_px: 1620,
            required_source_height_px: 1080,
        }
    }

    fn candidate<'a>(
        session_id: &'a str,
        request_id: &'a str,
        generation_seq: u64,
        request_order: u64,
    ) -> GenerationCandidate<'a> {
        GenerationCandidate {
            session_id,
            request_id,
            capture_id: None,
            viewer_epoch: 3,
            tier: DISPLAY_TIER_SAMPLE,
            generation_seq,
            source_width_px: 3840,
            source_height_px: 2560,
            request_order,
            capture_order: None,
            preset_binding: None,
            refined_tier_justified: false,
        }
    }

    /// 활성 display truth 없이 후보만 판정한다.
    fn no_active<'a>() -> ActiveDisplay<'a> {
        ActiveDisplay::default()
    }

    /// 활성 generation과 그 순서 좌표.
    fn active<'a>(
        generation: &'a DisplayGenerationDto,
        request_order: Option<u64>,
        capture_order: Option<u64>,
    ) -> ActiveDisplay<'a> {
        ActiveDisplay {
            generation: Some(generation),
            request_order,
            capture_order,
        }
    }

    const SESSION: &str = "session_01hzzzzzzzzzzzzzzzzzzzzzzz";

    #[test]
    fn admits_first_qualifying_generation() {
        assert_eq!(
            evaluate_admission(
                no_active(),
                &candidate(SESSION, "req-1", 1, 0),
                &context(SESSION)
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_other_session() {
        let other = "session_01haaaaaaaaaaaaaaaaaaaaaaa";

        assert_eq!(
            evaluate_admission(
                no_active(),
                &candidate(other, "req-1", 1, 0),
                &context(SESSION)
            ),
            Err(DISPLAY_REJECT_SESSION_MISMATCH)
        );
    }

    #[test]
    fn rejects_stale_viewer_epoch() {
        let mut stale = candidate(SESSION, "req-1", 1, 0);
        stale.viewer_epoch = 2;

        assert_eq!(
            evaluate_admission(no_active(), &stale, &context(SESSION)),
            Err(DISPLAY_REJECT_STALE_EPOCH)
        );
    }

    #[test]
    fn rejects_when_viewer_has_not_measured_photo_rect() {
        let mut unmeasured = context(SESSION);
        unmeasured.required_source_width_px = 0;
        unmeasured.required_source_height_px = 0;

        assert_eq!(
            evaluate_admission(no_active(), &candidate(SESSION, "req-1", 1, 0), &unmeasured),
            Err(DISPLAY_REJECT_VIEWER_NOT_READY)
        );
    }

    #[test]
    fn rejects_insufficient_dimensions() {
        let mut small = candidate(SESSION, "req-1", 1, 0);
        small.source_width_px = 384;
        small.source_height_px = 256;

        assert_eq!(
            evaluate_admission(no_active(), &small, &context(SESSION)),
            Err(DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS)
        );
    }

    #[test]
    fn admits_portrait_output_that_reaches_the_contain_boundary_without_upscaling() {
        let mut portrait = candidate(SESSION, "req-1", 1, 0);
        portrait.source_width_px = 720;
        portrait.source_height_px = 1080;

        assert_eq!(
            evaluate_admission(no_active(), &portrait, &context(SESSION)),
            Ok(())
        );
    }

    #[test]
    fn rejects_lower_and_equal_generation_seq() {
        let current = generation("req-1", 5, 3840, 2560);

        assert_eq!(
            evaluate_admission(
                active(&current, Some(0), None),
                &candidate(SESSION, "req-1", 4, 0),
                &context(SESSION)
            ),
            Err(DISPLAY_REJECT_LOWER_GENERATION)
        );
        assert_eq!(
            evaluate_admission(
                active(&current, Some(0), None),
                &candidate(SESSION, "req-1", 5, 0),
                &context(SESSION)
            ),
            Err(DISPLAY_REJECT_LOWER_GENERATION)
        );
    }

    #[test]
    fn rejects_older_request_even_with_higher_seq() {
        // req-2가 활성이고, 더 오래된 req-1의 두 번째 표본이 더 높은 seq로 늦게 도착한 상황.
        let current = generation("req-2", 5, 3840, 2560);

        assert_eq!(
            evaluate_admission(
                active(&current, Some(1), None),
                &candidate(SESSION, "req-1", 6, 0),
                &context(SESSION)
            ),
            Err(DISPLAY_REJECT_OLDER_REQUEST)
        );
    }

    #[test]
    fn admits_newer_request_with_higher_seq() {
        let current = generation("req-1", 5, 3840, 2560);

        assert_eq!(
            evaluate_admission(
                active(&current, Some(0), None),
                &candidate(SESSION, "req-2", 6, 1),
                &context(SESSION)
            ),
            Ok(())
        );
    }

    #[test]
    fn session_change_clears_pointer() {
        let mut state = DisplayState::default();
        state.bind_session(SESSION);
        state.reserve_generation("req-1");
        state.commit_generation(generation("req-1", 1, 3840, 2560));

        assert!(state.active_generation().is_some());

        let changed = state.reconcile(Some("session_01haaaaaaaaaaaaaaaaaaaaaaa"), 1620, 1080);

        assert!(changed);
        assert!(state.active_generation().is_none());
    }

    #[test]
    fn reconcile_drops_active_generation_when_required_size_grows() {
        let mut state = DisplayState::default();
        state.bind_session(SESSION);
        state.reserve_generation("req-1");
        state.commit_generation(generation("req-1", 1, 1920, 1280));

        assert!(!state.reconcile(Some(SESSION), 1620, 1080));
        assert!(state.active_generation().is_some());

        // viewer epoch가 바뀌며 4K photo rect로 커진 경우.
        assert!(state.reconcile(Some(SESSION), 3240, 2160));
        assert!(state.active_generation().is_none());
    }

    #[test]
    fn poisoned_generation_is_dropped_from_pointer() {
        let mut state = DisplayState::default();
        state.bind_session(SESSION);
        let committed = generation("req-1", 1, 3840, 2560);
        let generation_id = committed.generation_id.clone();
        state.commit_generation(committed);

        assert!(state.mark_poisoned(&generation_id));
        assert!(state.active_generation().is_none());
        assert!(state.is_poisoned(&generation_id));
    }

    #[test]
    fn forget_request_clears_only_matching_active_generation() {
        let mut state = DisplayState::default();
        state.bind_session(SESSION);
        state.commit_generation(generation("req-1", 1, 3840, 2560));

        assert!(!state.forget_request("req-2"));
        assert!(state.active_generation().is_some());
        assert!(state.forget_request("req-1"));
        assert!(state.active_generation().is_none());
    }

    #[test]
    fn generation_seq_is_monotonic_and_request_order_is_stable() {
        let mut state = DisplayState::default();
        state.bind_session(SESSION);

        let (first_seq, first_order) = state.reserve_generation("req-1");
        let (second_seq, second_order) = state.reserve_generation("req-1");
        let (third_seq, third_order) = state.reserve_generation("req-2");

        assert!(second_seq > first_seq);
        assert!(third_seq > second_seq);
        assert_eq!(first_order, second_order);
        assert!(third_order > second_order);
    }

    #[test]
    fn revision_never_goes_backwards() {
        let mut state = DisplayState::default();
        let mut previous = state.revision();

        state.bind_session(SESSION);
        assert!(state.revision() > previous);
        previous = state.revision();

        state.commit_generation(generation("req-1", 1, 3840, 2560));
        assert!(state.revision() > previous);
        previous = state.revision();

        state.clear_session();
        assert!(state.revision() > previous);
    }
}
