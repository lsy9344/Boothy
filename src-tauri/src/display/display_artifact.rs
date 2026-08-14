//! Story 7.2: immutable display generation의 승격 규칙과 pointer 상태.
//!
//! 정확성은 전송 순서가 아니라 여기의 monotonic guard에서 나온다. 늦게 도착하거나 중복된
//! 게시가 현재 화면을 되돌리지 못하게 하는 판정을 전부 순수 함수로 분리해 둔다.

use std::collections::{BTreeMap, BTreeSet};

use crate::contracts::dto::{
    display_tier_order, DisplayGenerationDto, DisplayPointerSnapshotDto,
    DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS, DISPLAY_REJECT_LOWER_GENERATION,
    DISPLAY_REJECT_OLDER_REQUEST, DISPLAY_REJECT_SESSION_MISMATCH, DISPLAY_REJECT_STALE_EPOCH,
    DISPLAY_REJECT_TIER_DOWNGRADE, DISPLAY_REJECT_UNKNOWN_GENERATION,
    DISPLAY_REJECT_VIEWER_NOT_READY, VIEWER_DISPLAY_SCHEMA_VERSION,
};

/// 승격 판정에 필요한 host 측 문맥. 전부 host가 소유한 truth다.
#[derive(Debug, Clone, Copy)]
pub struct AdmissionContext<'a> {
    pub bound_session_id: Option<&'a str>,
    pub viewer_epoch: u64,
    pub required_source_width_px: u32,
    pub required_source_height_px: u32,
}

/// 아직 활성화되지 않은 후보 generation.
#[derive(Debug, Clone, Copy)]
pub struct GenerationCandidate<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub viewer_epoch: u64,
    pub tier: &'a str,
    pub generation_seq: u64,
    pub source_width_px: u32,
    pub source_height_px: u32,
    /// 이 request가 처음 관측된 순서. 값이 작을수록 오래된 request다.
    pub request_order: u64,
}

/// 현재 활성 generation과 비교해 후보를 승격할 수 있는지 판정한다.
///
/// 각 거부 경로는 고유 reason을 돌려준다. 조용한 무시는 허용하지 않는다.
pub fn evaluate_admission(
    current: Option<&DisplayGenerationDto>,
    current_request_order: Option<u64>,
    candidate: &GenerationCandidate<'_>,
    context: &AdmissionContext<'_>,
) -> Result<(), &'static str> {
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

    if candidate.source_width_px < context.required_source_width_px
        || candidate.source_height_px < context.required_source_height_px
    {
        return Err(DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS);
    }

    let Some(current) = current else {
        return Ok(());
    };

    let current_order = display_tier_order(&current.tier).unwrap_or(0);
    let candidate_order = display_tier_order(candidate.tier).unwrap_or(0);

    if candidate_order < current_order {
        return Err(DISPLAY_REJECT_TIER_DOWNGRADE);
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
            let insufficient = active.source_width_px < required_source_width_px
                || active.source_height_px < required_source_height_px;

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
        measurement_lane_enabled: bool,
        observed_at_host_micros: u64,
    ) -> DisplayPointerSnapshotDto {
        DisplayPointerSnapshotDto {
            schema_version: VIEWER_DISPLAY_SCHEMA_VERSION.into(),
            session_id: self.session_id.clone(),
            revision: self.revision,
            active_generation: self.active.clone(),
            required_source_width_px,
            required_source_height_px,
            measurement_lane_enabled,
            observed_at_host_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generation(
        request_id: &str,
        generation_seq: u64,
        width: u32,
        height: u32,
    ) -> DisplayGenerationDto {
        DisplayGenerationDto {
            generation_id: format!("{request_id}-{generation_seq:06}"),
            generation_seq,
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
            sample_variant: "a".into(),
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
            viewer_epoch: 3,
            tier: "sample",
            generation_seq,
            source_width_px: 3840,
            source_height_px: 2560,
            request_order,
        }
    }

    const SESSION: &str = "session_01hzzzzzzzzzzzzzzzzzzzzzzz";

    #[test]
    fn admits_first_qualifying_generation() {
        assert_eq!(
            evaluate_admission(
                None,
                None,
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
                None,
                None,
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
            evaluate_admission(None, None, &stale, &context(SESSION)),
            Err(DISPLAY_REJECT_STALE_EPOCH)
        );
    }

    #[test]
    fn rejects_when_viewer_has_not_measured_photo_rect() {
        let mut unmeasured = context(SESSION);
        unmeasured.required_source_width_px = 0;
        unmeasured.required_source_height_px = 0;

        assert_eq!(
            evaluate_admission(None, None, &candidate(SESSION, "req-1", 1, 0), &unmeasured),
            Err(DISPLAY_REJECT_VIEWER_NOT_READY)
        );
    }

    #[test]
    fn rejects_insufficient_dimensions() {
        let mut small = candidate(SESSION, "req-1", 1, 0);
        small.source_width_px = 384;
        small.source_height_px = 256;

        assert_eq!(
            evaluate_admission(None, None, &small, &context(SESSION)),
            Err(DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS)
        );
    }

    #[test]
    fn rejects_lower_and_equal_generation_seq() {
        let current = generation("req-1", 5, 3840, 2560);

        assert_eq!(
            evaluate_admission(
                Some(&current),
                Some(0),
                &candidate(SESSION, "req-1", 4, 0),
                &context(SESSION)
            ),
            Err(DISPLAY_REJECT_LOWER_GENERATION)
        );
        assert_eq!(
            evaluate_admission(
                Some(&current),
                Some(0),
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
                Some(&current),
                Some(1),
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
                Some(&current),
                Some(0),
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
