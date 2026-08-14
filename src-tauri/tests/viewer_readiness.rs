//! Story 7.1: viewer readiness truth와 capture eligibility gate 회귀 테스트.

use boothy_lib::{
    commands::viewer_commands::{should_attempt_window_creation, should_recreate_unhealthy_window},
    contracts::dto::{
        validate_viewer_layout_report, CaptureReadinessDto, ViewerLayoutReportDto,
        ViewerReadinessSnapshotDto, VIEWER_READINESS_SCHEMA_VERSION,
    },
    viewer::{
        display_profile::{
            classify_display_profile, required_source_dimensions, select_customer_monitor,
            MonitorDescriptor, MonitorSelection, MONITOR_TARGETING_APPROVED,
            MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK, MONITOR_TARGETING_UNAPPROVED_PROFILE,
            MONITOR_TARGETING_UNAVAILABLE,
        },
        readiness_gate::{apply_viewer_gate, CAPTURE_REASON_VIEWER_PREPARING},
        should_offload_window_creation,
        viewer_state::{
            ReportOutcome, ViewerState, VIEWER_REASON_LAYOUT_NOT_READY,
            VIEWER_REASON_LISTENER_NOT_READY, VIEWER_REASON_MONITOR_NOT_APPROVED,
            VIEWER_REASON_READY, VIEWER_REASON_SESSION_MISMATCH, VIEWER_REASON_SESSION_UNBOUND,
            VIEWER_REASON_STALE_EPOCH, VIEWER_REASON_STALE_REPORT, VIEWER_REASON_WINDOW_CLOSED,
            VIEWER_REPORT_STALE_AFTER_MS,
        },
    },
};

const SESSION_A: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
const SESSION_B: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g2n";

fn approved_1080p_monitor() -> MonitorDescriptor {
    MonitorDescriptor {
        name: Some("BOOTH-CUSTOMER".into()),
        width_px: 1920,
        height_px: 1080,
        scale_factor: 1.0,
        position_x: 1920,
        position_y: 0,
        is_primary: false,
    }
}

fn operator_monitor() -> MonitorDescriptor {
    MonitorDescriptor {
        name: Some("OPERATOR".into()),
        width_px: 1920,
        height_px: 1080,
        scale_factor: 1.0,
        position_x: 0,
        position_y: 0,
        is_primary: true,
    }
}

fn approved_selection() -> MonitorSelection {
    select_customer_monitor(
        &[operator_monitor(), approved_1080p_monitor()],
        Some("BOOTH-CUSTOMER"),
    )
}

fn layout_report(viewer_epoch: u64, session_id: Option<&str>) -> ViewerLayoutReportDto {
    ViewerLayoutReportDto {
        viewer_epoch,
        session_id: session_id.map(str::to_string),
        css_width: 1620.0,
        css_height: 1080.0,
        device_pixel_ratio: 1.0,
        layout_ready: true,
    }
}

/// 승인된 모니터에서 세션 binding까지 완료된 정상 상태를 만든다.
fn ready_state(now_ms: u64) -> ViewerState {
    let mut state = ViewerState::default();
    state.bind_session(SESSION_A);
    state.mark_window_open(&approved_selection());
    let epoch = state.viewer_epoch();
    state.apply_listener_report(epoch, now_ms);
    state.apply_layout_report(&layout_report(epoch, Some(SESSION_A)), now_ms);

    state
}

fn capture_ready_readiness() -> CaptureReadinessDto {
    CaptureReadinessDto {
        schema_version: "capture-readiness/v1".into(),
        session_id: SESSION_A.into(),
        surface_state: "captureReady".into(),
        customer_state: "Ready".into(),
        can_capture: true,
        primary_action: "capture".into(),
        customer_message: "사진을 찍어 주세요.".into(),
        support_message: "준비가 끝났어요.".into(),
        reason_code: "ready".into(),
        latest_capture: None,
        recent_captures: Vec::new(),
        live_capture_truth: None,
        post_end: None,
        timing: None,
    }
}

// --- display profile / physical display-size contract -------------------------------------

#[test]
fn classifies_only_approved_display_profiles() {
    assert_eq!(classify_display_profile(1920, 1080), Some("1080p"));
    assert_eq!(classify_display_profile(2560, 1440), Some("1440p"));
    assert_eq!(classify_display_profile(3840, 2160), Some("4k"));
    assert_eq!(classify_display_profile(1280, 720), None);
    assert_eq!(classify_display_profile(3440, 1440), None);
}

#[test]
fn derives_required_source_dimensions_from_rect_and_dpr() {
    assert_eq!(
        required_source_dimensions(1620.0, 1080.0, 1.0),
        (1620, 1080)
    );
    assert_eq!(
        required_source_dimensions(1728.0, 1152.0, 1.25),
        (2160, 1440)
    );
    assert_eq!(
        required_source_dimensions(1620.0, 1080.0, 2.0),
        (3240, 2160)
    );
}

#[test]
fn rounds_required_source_dimensions_up_so_upscale_is_never_allowed() {
    assert_eq!(
        required_source_dimensions(1620.4, 1080.2, 1.5),
        (2431, 1621)
    );
}

#[test]
fn fixed_384px_thumbnail_never_satisfies_an_approved_profile() {
    for (width, height, dpr) in [
        (1620.0, 1080.0, 1.0),
        (1728.0, 1152.0, 1.25),
        (1620.0, 1080.0, 2.0),
    ] {
        let (required_width, required_height) = required_source_dimensions(width, height, dpr);

        assert!(
            384 < required_width && 384 < required_height,
            "384px thumbnail이 display-fit source로 취급되면 안 된다"
        );
    }
}

// --- monitor targeting --------------------------------------------------------------------

#[test]
fn selects_the_approved_customer_monitor_by_name() {
    let selection = approved_selection();

    assert_eq!(selection.targeting, MONITOR_TARGETING_APPROVED);
    assert_eq!(selection.profile_id, Some("1080p"));
    assert_eq!(
        selection.monitor.as_ref().map(|monitor| monitor.position_x),
        Some(1920)
    );
}

#[test]
fn reports_unavailable_when_the_approved_monitor_is_missing() {
    let selection = select_customer_monitor(&[operator_monitor()], Some("BOOTH-CUSTOMER"));

    assert_eq!(selection.targeting, MONITOR_TARGETING_UNAVAILABLE);
    assert!(
        selection.monitor.is_none(),
        "승인된 모니터가 없으면 임의 모니터로 대체하면 안 된다"
    );
}

#[test]
fn reports_unavailable_when_no_monitor_is_enumerated() {
    let selection = select_customer_monitor(&[], None);

    assert_eq!(selection.targeting, MONITOR_TARGETING_UNAVAILABLE);
}

#[test]
fn unavailable_monitor_keeps_the_viewer_window_absent() {
    let selection = select_customer_monitor(&[], None);
    let mut state = ViewerState::default();
    state.mark_window_creating();
    state.mark_window_creation_blocked(&selection);

    let snapshot = state.snapshot(1_000_000);
    assert_eq!(snapshot.window_state, "absent");
    assert_eq!(snapshot.monitor_targeting, MONITOR_TARGETING_UNAVAILABLE);
    assert_eq!(snapshot.reason_code, "viewer-absent");
}

#[test]
fn falls_back_honestly_on_a_single_monitor_machine() {
    let selection = select_customer_monitor(&[operator_monitor()], None);

    assert_eq!(
        selection.targeting,
        MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK
    );
}

#[test]
fn prefers_the_non_primary_monitor_when_no_name_is_configured() {
    let selection = select_customer_monitor(&[operator_monitor(), approved_1080p_monitor()], None);

    assert_eq!(selection.targeting, MONITOR_TARGETING_APPROVED);
    assert_eq!(
        selection
            .monitor
            .as_ref()
            .and_then(|monitor| monitor.name.clone()),
        Some("BOOTH-CUSTOMER".into())
    );
}

#[test]
fn prefers_an_approved_profile_over_the_first_unapproved_non_primary_monitor() {
    let portrait_monitor = MonitorDescriptor {
        name: Some("PORTRAIT-DISPLAY".into()),
        width_px: 1440,
        height_px: 2560,
        scale_factor: 1.0,
        position_x: 2560,
        position_y: 411,
        is_primary: false,
    };

    let selection = select_customer_monitor(
        &[
            operator_monitor(),
            portrait_monitor,
            approved_1080p_monitor(),
        ],
        None,
    );

    assert_eq!(selection.targeting, MONITOR_TARGETING_APPROVED);
    assert_eq!(selection.profile_id, Some("1080p"));
    assert_eq!(
        selection
            .monitor
            .as_ref()
            .and_then(|monitor| monitor.name.clone()),
        Some("BOOTH-CUSTOMER".into())
    );
}

#[test]
fn reports_unapproved_profile_for_an_unknown_resolution() {
    let odd_monitor = MonitorDescriptor {
        width_px: 1280,
        height_px: 720,
        ..operator_monitor()
    };
    let selection = select_customer_monitor(&[odd_monitor], None);

    assert_eq!(selection.targeting, MONITOR_TARGETING_UNAPPROVED_PROFILE);
    assert_eq!(selection.profile_id, None);
}

#[test]
fn fallback_and_unapproved_monitors_never_become_viewer_ready() {
    let now_ms = 1_000_000;
    let fallback = select_customer_monitor(&[operator_monitor()], None);
    let unapproved = select_customer_monitor(
        &[MonitorDescriptor {
            width_px: 1280,
            height_px: 720,
            ..operator_monitor()
        }],
        None,
    );

    for selection in [fallback, unapproved] {
        let mut state = ViewerState::default();
        state.bind_session(SESSION_A);
        state.mark_window_open(&selection);
        let epoch = state.viewer_epoch();
        state.apply_listener_report(epoch, now_ms);
        state.apply_layout_report(&layout_report(epoch, Some(SESSION_A)), now_ms);

        let snapshot = state.snapshot(now_ms);
        assert!(!snapshot.viewer_ready);
        assert_eq!(snapshot.reason_code, VIEWER_REASON_MONITOR_NOT_APPROVED);
    }
}

#[test]
fn rejects_layout_reports_that_exceed_the_host_dimension_contract() {
    let mut report = layout_report(1, Some(SESSION_A));
    report.css_width = u32::MAX as f64;
    report.device_pixel_ratio = 2.0;

    assert!(validate_viewer_layout_report(&report).is_err());
}

// --- readiness state machine --------------------------------------------------------------

#[test]
fn reports_ready_only_after_the_full_handshake() {
    let now_ms = 1_000_000;
    let state = ready_state(now_ms);
    let snapshot = state.snapshot(now_ms);

    assert!(snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_READY);
    assert_eq!(snapshot.schema_version, VIEWER_READINESS_SCHEMA_VERSION);
    assert_eq!(snapshot.session_id.as_deref(), Some(SESSION_A));
    assert_eq!(
        snapshot.photo_rect.as_ref().map(|rect| (
            rect.required_source_width_px,
            rect.required_source_height_px
        )),
        Some((1620, 1080))
    );
    assert_eq!(
        snapshot
            .display_profile
            .as_ref()
            .map(|profile| profile.profile_id.clone()),
        Some("1080p".into())
    );
}

#[test]
fn is_not_ready_before_a_session_is_bound() {
    let now_ms = 1_000_000;
    let mut state = ViewerState::default();
    state.mark_window_open(&approved_selection());

    let snapshot = state.snapshot(now_ms);

    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_SESSION_UNBOUND);
}

#[test]
fn is_not_ready_before_the_listener_subscribes() {
    let now_ms = 1_000_000;
    let mut state = ViewerState::default();
    state.bind_session(SESSION_A);
    state.mark_window_open(&approved_selection());

    let snapshot = state.snapshot(now_ms);

    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_LISTENER_NOT_READY);
}

#[test]
fn is_not_ready_before_layout_completes() {
    let now_ms = 1_000_000;
    let mut state = ViewerState::default();
    state.bind_session(SESSION_A);
    state.mark_window_open(&approved_selection());
    let epoch = state.viewer_epoch();
    state.apply_listener_report(epoch, now_ms);

    let snapshot = state.snapshot(now_ms);

    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_LAYOUT_NOT_READY);
}

#[test]
fn blocks_when_the_viewer_reports_a_different_session() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    // 세션이 교체되면 viewer가 새 binding을 다시 보고할 때까지 ready를 유지하면 안 된다.
    state.bind_session(SESSION_B);

    let snapshot = state.snapshot(now_ms);

    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_SESSION_MISMATCH);
}

#[test]
fn recovers_after_the_viewer_re_reports_the_new_session() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    state.bind_session(SESSION_B);
    let epoch = state.viewer_epoch();
    state.apply_layout_report(&layout_report(epoch, Some(SESSION_B)), now_ms);

    let snapshot = state.snapshot(now_ms);

    assert!(snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_READY);
}

#[test]
fn window_loss_immediately_drops_readiness() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    state.mark_window_closed();

    let snapshot = state.snapshot(now_ms);

    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_WINDOW_CLOSED);
}

#[test]
fn stale_close_event_cannot_close_a_recreated_viewer_epoch() {
    let mut state = ViewerState::default();
    state.mark_window_open(&approved_selection());
    let stale_epoch = state.viewer_epoch();
    state.mark_window_open(&approved_selection());

    assert!(!state.mark_window_closed_if_epoch(stale_epoch));
    assert_ne!(
        state.snapshot(1_000_000).reason_code,
        VIEWER_REASON_WINDOW_CLOSED
    );
}

#[test]
fn reload_issues_a_new_epoch_and_invalidates_previous_readiness() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    let previous_epoch = state.viewer_epoch();

    state.mark_window_open(&approved_selection());

    assert!(state.viewer_epoch() > previous_epoch);
    let snapshot = state.snapshot(now_ms);
    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_LISTENER_NOT_READY);
}

#[test]
fn rejects_reports_from_a_stale_epoch() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    let stale_epoch = state.viewer_epoch();
    state.mark_window_open(&approved_selection());

    let outcome = state.apply_layout_report(&layout_report(stale_epoch, Some(SESSION_A)), now_ms);

    assert_eq!(outcome, ReportOutcome::StaleEpoch);
    let snapshot = state.snapshot(now_ms);
    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_STALE_EPOCH);
}

#[test]
fn goes_stale_when_the_viewer_stops_reporting() {
    let now_ms = 1_000_000;
    let state = ready_state(now_ms);

    let still_fresh = state.snapshot(now_ms + VIEWER_REPORT_STALE_AFTER_MS);
    assert!(still_fresh.viewer_ready);

    let stale = state.snapshot(now_ms + VIEWER_REPORT_STALE_AFTER_MS + 1);
    assert!(!stale.viewer_ready);
    assert_eq!(stale.reason_code, VIEWER_REASON_STALE_REPORT);
}

#[test]
fn wall_clock_changes_do_not_change_liveness() {
    let observed_at_ms = 1_000_000;
    let monotonic_ms = 10_000;
    let mut state = ViewerState::default();
    state.bind_session(SESSION_A);
    state.mark_window_open(&approved_selection());
    let epoch = state.viewer_epoch();
    state.apply_listener_report_with_clock(epoch, observed_at_ms, monotonic_ms);
    state.apply_layout_report_with_clock(
        &layout_report(epoch, Some(SESSION_A)),
        observed_at_ms,
        monotonic_ms,
    );

    let backward_clock = state.snapshot_with_clock(1, monotonic_ms + VIEWER_REPORT_STALE_AFTER_MS);
    assert!(backward_clock.viewer_ready);

    let forward_clock =
        state.snapshot_with_clock(u64::MAX, monotonic_ms + VIEWER_REPORT_STALE_AFTER_MS + 1);
    assert!(!forward_clock.viewer_ready);
    assert_eq!(forward_clock.reason_code, VIEWER_REASON_STALE_REPORT);
}

#[test]
fn heartbeat_refreshes_liveness_without_inflating_revision() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    let revision_before = state.revision();
    let epoch = state.viewer_epoch();

    let outcome = state.apply_layout_report(
        &layout_report(epoch, Some(SESSION_A)),
        now_ms + VIEWER_REPORT_STALE_AFTER_MS,
    );

    assert_eq!(outcome, ReportOutcome::Unchanged);
    assert_eq!(state.revision(), revision_before);
    assert!(
        state
            .snapshot(now_ms + VIEWER_REPORT_STALE_AFTER_MS)
            .viewer_ready
    );
}

#[test]
fn revision_increases_monotonically_across_state_changes() {
    let now_ms = 1_000_000;
    let mut state = ViewerState::default();
    let mut revisions = vec![state.revision()];

    state.bind_session(SESSION_A);
    revisions.push(state.revision());
    state.mark_window_open(&approved_selection());
    revisions.push(state.revision());
    let epoch = state.viewer_epoch();
    state.apply_listener_report(epoch, now_ms);
    revisions.push(state.revision());
    state.apply_layout_report(&layout_report(epoch, Some(SESSION_A)), now_ms);
    revisions.push(state.revision());
    state.mark_window_closed();
    revisions.push(state.revision());

    for window in revisions.windows(2) {
        assert!(window[1] > window[0], "revision은 단조 증가해야 한다");
    }
}

#[test]
fn duplicate_listener_reports_are_idempotent() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    let epoch = state.viewer_epoch();
    let revision_before = state.revision();

    assert_eq!(
        state.apply_listener_report(epoch, now_ms),
        ReportOutcome::Unchanged
    );
    assert_eq!(state.revision(), revision_before);
}

#[test]
fn layout_report_without_layout_ready_does_not_claim_readiness() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    let epoch = state.viewer_epoch();
    let mut broken_layout = layout_report(epoch, Some(SESSION_A));
    broken_layout.layout_ready = false;

    state.apply_layout_report(&broken_layout, now_ms);
    let snapshot = state.snapshot(now_ms);

    assert!(!snapshot.viewer_ready);
    assert_eq!(snapshot.reason_code, VIEWER_REASON_LAYOUT_NOT_READY);
}

#[test]
fn zero_sized_layout_immediately_clears_the_photo_contract() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    let epoch = state.viewer_epoch();
    let collapsed_layout = ViewerLayoutReportDto {
        viewer_epoch: epoch,
        session_id: Some(SESSION_A.into()),
        css_width: 0.0,
        css_height: 0.0,
        device_pixel_ratio: 1.0,
        layout_ready: false,
    };

    assert!(validate_viewer_layout_report(&collapsed_layout).is_ok());
    state.apply_layout_report(&collapsed_layout, now_ms);

    let snapshot = state.snapshot(now_ms);
    assert!(!snapshot.viewer_ready);
    assert!(!snapshot.layout_ready);
    assert!(snapshot.photo_rect.is_none());
    assert_eq!(snapshot.reason_code, VIEWER_REASON_LAYOUT_NOT_READY);
}

#[test]
fn zero_sized_layout_cannot_claim_layout_ready() {
    let report = ViewerLayoutReportDto {
        viewer_epoch: 1,
        session_id: Some(SESSION_A.into()),
        css_width: 0.0,
        css_height: 0.0,
        device_pixel_ratio: 1.0,
        layout_ready: true,
    };

    assert!(validate_viewer_layout_report(&report).is_err());
}

// --- window creation threading (HV-13A 2026-08-11 회귀 방지) -------------------------------

/// event loop가 도는 main thread에서 `WebviewWindowBuilder::build()`를 호출하면
/// event loop가 자기 응답을 기다리는 교착이 된다. 창은 뜨지만 절대 그려지지 않고
/// 호출도 반환되지 않는다. 그래서 그 조합에서만 background 스레드로 넘겨야 한다.
#[test]
fn offloads_window_creation_only_on_the_running_event_loop_thread() {
    assert!(
        should_offload_window_creation(true, true),
        "event loop가 시작된 뒤 main thread에서는 반드시 offload해야 한다"
    );
}

#[test]
fn creates_inline_during_setup_before_the_event_loop_starts() {
    assert!(
        !should_offload_window_creation(true, false),
        "setup 단계는 event loop가 아직 없으므로 main thread에서 직접 만들어도 안전하다"
    );
}

#[test]
fn creates_inline_on_background_threads() {
    assert!(!should_offload_window_creation(false, true));
    assert!(!should_offload_window_creation(false, false));
}

// --- window recreation single-flight / backoff ----------------------------------------------

#[test]
fn first_creation_attempt_is_always_allowed() {
    assert!(should_attempt_window_creation(false, None, 0, 2_000));
}

#[test]
fn does_not_start_a_second_creation_while_one_is_in_flight() {
    assert!(
        !should_attempt_window_creation(true, None, 10_000, 2_000),
        "readiness poll이 반복돼도 창 생성을 중복 실행하면 안 된다"
    );
}

#[test]
fn backs_off_between_failed_creation_attempts() {
    assert!(!should_attempt_window_creation(
        false,
        Some(1_000),
        2_999,
        2_000
    ));
    assert!(should_attempt_window_creation(
        false,
        Some(1_000),
        3_000,
        2_000
    ));
}

#[test]
fn recreates_only_after_a_recoverable_viewer_failure_persists() {
    assert!(!should_recreate_unhealthy_window(
        VIEWER_REASON_LISTENER_NOT_READY,
        Some(1_000),
        10_999,
        10_000,
    ));
    assert!(should_recreate_unhealthy_window(
        VIEWER_REASON_STALE_REPORT,
        Some(1_000),
        11_000,
        10_000,
    ));
    assert!(!should_recreate_unhealthy_window(
        VIEWER_REASON_SESSION_UNBOUND,
        Some(1_000),
        20_000,
        10_000,
    ));
    assert!(!should_recreate_unhealthy_window(
        VIEWER_REASON_MONITOR_NOT_APPROVED,
        Some(1_000),
        20_000,
        10_000,
    ));
}

// --- capture eligibility gate --------------------------------------------------------------

#[test]
fn capture_stays_allowed_when_the_viewer_is_ready() {
    let now_ms = 1_000_000;
    let snapshot = ready_state(now_ms).snapshot(now_ms);
    let gated = apply_viewer_gate(capture_ready_readiness(), &snapshot, SESSION_A);

    assert!(gated.can_capture);
    assert_eq!(gated.reason_code, "ready");
}

#[test]
fn capture_is_blocked_when_the_viewer_is_not_ready() {
    let now_ms = 1_000_000;
    let mut state = ready_state(now_ms);
    state.mark_window_closed();
    let snapshot = state.snapshot(now_ms);

    let gated = apply_viewer_gate(capture_ready_readiness(), &snapshot, SESSION_A);

    assert!(!gated.can_capture);
    assert_eq!(gated.reason_code, CAPTURE_REASON_VIEWER_PREPARING);
    assert_eq!(gated.surface_state, "blocked");
    assert_eq!(gated.primary_action, "wait");
    assert_eq!(gated.customer_state, "Preparing");
}

#[test]
fn capture_is_blocked_when_the_viewer_is_bound_to_another_session() {
    let now_ms = 1_000_000;
    let snapshot = ready_state(now_ms).snapshot(now_ms);

    let gated = apply_viewer_gate(capture_ready_readiness(), &snapshot, SESSION_B);

    assert!(!gated.can_capture);
    assert_eq!(gated.reason_code, CAPTURE_REASON_VIEWER_PREPARING);
}

#[test]
fn viewer_gate_never_overrides_an_existing_block_reason() {
    let now_ms = 1_000_000;
    let snapshot = ready_state(now_ms).snapshot(now_ms);
    let post_end_readiness = CaptureReadinessDto {
        can_capture: false,
        reason_code: "phone-required".into(),
        surface_state: "blocked".into(),
        customer_state: "Phone Required".into(),
        ..capture_ready_readiness()
    };

    let gated = apply_viewer_gate(post_end_readiness, &snapshot, SESSION_A);

    assert_eq!(
        gated.reason_code, "phone-required",
        "viewer gate는 downgrade 전용이며 기존 차단 원인을 덮으면 안 된다"
    );
}

#[test]
fn viewer_gate_never_upgrades_a_blocked_capture() {
    let now_ms = 1_000_000;
    let snapshot: ViewerReadinessSnapshotDto = ready_state(now_ms).snapshot(now_ms);
    let blocked = CaptureReadinessDto {
        can_capture: false,
        reason_code: "camera-preparing".into(),
        ..capture_ready_readiness()
    };

    let gated = apply_viewer_gate(blocked, &snapshot, SESSION_A);

    assert!(!gated.can_capture);
    assert_eq!(gated.reason_code, "camera-preparing");
}
