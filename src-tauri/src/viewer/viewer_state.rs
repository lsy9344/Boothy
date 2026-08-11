//! Host가 소유하는 viewer readiness truth.
//!
//! `viewer-window`는 booth window와 별개의 WebView이므로 React 상태로 세션을 전달할 수 없다.
//! 현재 세션 binding, viewer epoch, monotonic revision은 반드시 host가 소유한다.

use crate::contracts::dto::{
    ViewerDisplayProfileDto, ViewerLayoutReportDto, ViewerPhotoRectDto, ViewerReadinessSnapshotDto,
    VIEWER_READINESS_SCHEMA_VERSION,
};

use super::display_profile::{
    required_source_dimensions, MonitorSelection, MONITOR_TARGETING_APPROVED,
    MONITOR_TARGETING_UNRESOLVED,
};

/// viewer report가 이 시간보다 오래되면 readiness를 유지하지 않는다.
/// WebView2 background throttling이 readiness를 허위로 유지하지 못하게 하는 장치다.
pub const VIEWER_REPORT_STALE_AFTER_MS: u64 = 5_000;

/// viewer가 layout report를 heartbeat로 재전송하는 주기.
pub const VIEWER_HEARTBEAT_INTERVAL_MS: u64 = 1_000;

pub const VIEWER_WINDOW_STATE_ABSENT: &str = "absent";
pub const VIEWER_WINDOW_STATE_CREATING: &str = "creating";
pub const VIEWER_WINDOW_STATE_OPEN: &str = "open";
pub const VIEWER_WINDOW_STATE_CLOSED: &str = "closed";

pub const VIEWER_REASON_READY: &str = "viewer-ready";
pub const VIEWER_REASON_ABSENT: &str = "viewer-absent";
pub const VIEWER_REASON_WINDOW_CLOSED: &str = "viewer-window-closed";
pub const VIEWER_REASON_LISTENER_NOT_READY: &str = "listener-not-ready";
pub const VIEWER_REASON_LAYOUT_NOT_READY: &str = "layout-not-ready";
pub const VIEWER_REASON_SESSION_UNBOUND: &str = "session-unbound";
pub const VIEWER_REASON_SESSION_MISMATCH: &str = "session-mismatch";
pub const VIEWER_REASON_STALE_EPOCH: &str = "stale-epoch";
pub const VIEWER_REASON_STALE_REPORT: &str = "stale-report";
pub const VIEWER_REASON_MONITOR_NOT_APPROVED: &str = "monitor-not-approved";

/// viewer report 처리 결과. stale epoch report는 조용히 폐기하되 진단을 위해 구분한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportOutcome {
    /// 상태가 실제로 변해 revision이 올랐다.
    Applied,
    /// 유효한 report지만 값이 같아 revision을 올리지 않았다 (heartbeat).
    Unchanged,
    /// 현재 epoch가 아닌 report라 폐기했다.
    StaleEpoch,
}

#[derive(Debug, Clone)]
pub struct ViewerState {
    session_id: Option<String>,
    viewer_epoch: u64,
    revision: u64,
    window_state: &'static str,
    listener_ready: bool,
    layout_ready: bool,
    monitor_targeting: &'static str,
    display_profile: Option<ViewerDisplayProfileDto>,
    photo_rect: Option<ViewerPhotoRectDto>,
    reported_session_id: Option<String>,
    last_report_at_ms: Option<u64>,
    last_report_monotonic_ms: Option<u64>,
    saw_stale_epoch_report: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            session_id: None,
            viewer_epoch: 0,
            revision: 0,
            window_state: VIEWER_WINDOW_STATE_ABSENT,
            listener_ready: false,
            layout_ready: false,
            monitor_targeting: MONITOR_TARGETING_UNRESOLVED,
            display_profile: None,
            photo_rect: None,
            reported_session_id: None,
            last_report_at_ms: None,
            last_report_monotonic_ms: None,
            saw_stale_epoch_report: false,
        }
    }
}

impl ViewerState {
    pub fn viewer_epoch(&self) -> u64 {
        self.viewer_epoch
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    fn bump(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    /// 현재 세션을 host truth로 고정한다. 세션이 바뀌면 viewer가 새 binding을 다시
    /// 보고할 때까지 readiness는 `session-mismatch`로 막힌다.
    pub fn bind_session(&mut self, session_id: impl Into<String>) {
        let session_id = session_id.into();

        if self.session_id.as_deref() == Some(session_id.as_str()) {
            return;
        }

        self.session_id = Some(session_id);
        self.bump();
    }

    pub fn clear_session(&mut self) {
        if self.session_id.is_none() {
            return;
        }

        self.session_id = None;
        self.bump();
    }

    pub fn mark_window_creating(&mut self) {
        if self.window_state == VIEWER_WINDOW_STATE_CREATING {
            return;
        }

        self.window_state = VIEWER_WINDOW_STATE_CREATING;
        self.bump();
    }

    /// viewer window 생성/재생성. 새 epoch를 발급하고 이전 epoch의 readiness를 전부 무효화한다.
    pub fn mark_window_open(&mut self, selection: &MonitorSelection) {
        self.viewer_epoch = self.viewer_epoch.saturating_add(1);
        self.window_state = VIEWER_WINDOW_STATE_OPEN;
        self.listener_ready = false;
        self.layout_ready = false;
        self.photo_rect = None;
        self.reported_session_id = None;
        self.last_report_at_ms = None;
        self.last_report_monotonic_ms = None;
        self.saw_stale_epoch_report = false;
        self.monitor_targeting = selection.targeting;
        self.display_profile = selection.monitor.as_ref().and_then(|monitor| {
            selection
                .profile_id
                .map(|profile_id| ViewerDisplayProfileDto {
                    profile_id: profile_id.into(),
                    monitor_name: monitor.name.clone(),
                    monitor_width_px: monitor.width_px,
                    monitor_height_px: monitor.height_px,
                    monitor_scale_factor: monitor.scale_factor,
                })
        });
        self.bump();
    }

    pub fn mark_window_closed(&mut self) {
        if self.window_state == VIEWER_WINDOW_STATE_CLOSED
            && !self.listener_ready
            && !self.layout_ready
        {
            return;
        }

        self.window_state = VIEWER_WINDOW_STATE_CLOSED;
        self.listener_ready = false;
        self.layout_ready = false;
        self.photo_rect = None;
        self.reported_session_id = None;
        self.last_report_at_ms = None;
        self.last_report_monotonic_ms = None;
        self.bump();
    }

    pub fn apply_listener_report(&mut self, viewer_epoch: u64, now_ms: u64) -> ReportOutcome {
        self.apply_listener_report_with_clock(viewer_epoch, now_ms, now_ms)
    }

    pub fn apply_listener_report_with_clock(
        &mut self,
        viewer_epoch: u64,
        observed_at_ms: u64,
        monotonic_ms: u64,
    ) -> ReportOutcome {
        if viewer_epoch != self.viewer_epoch || self.window_state != VIEWER_WINDOW_STATE_OPEN {
            self.saw_stale_epoch_report = true;
            return ReportOutcome::StaleEpoch;
        }

        self.last_report_at_ms = Some(observed_at_ms);
        self.last_report_monotonic_ms = Some(monotonic_ms);
        self.saw_stale_epoch_report = false;

        if self.listener_ready {
            return ReportOutcome::Unchanged;
        }

        self.listener_ready = true;
        self.bump();

        ReportOutcome::Applied
    }

    /// layout report이자 liveness heartbeat.
    /// 값이 변하지 않으면 revision을 올리지 않아 이벤트 폭주를 막는다.
    pub fn apply_layout_report(
        &mut self,
        report: &ViewerLayoutReportDto,
        now_ms: u64,
    ) -> ReportOutcome {
        self.apply_layout_report_with_clock(report, now_ms, now_ms)
    }

    pub fn apply_layout_report_with_clock(
        &mut self,
        report: &ViewerLayoutReportDto,
        observed_at_ms: u64,
        monotonic_ms: u64,
    ) -> ReportOutcome {
        if report.viewer_epoch != self.viewer_epoch || self.window_state != VIEWER_WINDOW_STATE_OPEN
        {
            self.saw_stale_epoch_report = true;
            return ReportOutcome::StaleEpoch;
        }

        self.last_report_at_ms = Some(observed_at_ms);
        self.last_report_monotonic_ms = Some(monotonic_ms);
        self.saw_stale_epoch_report = false;

        let (required_width, required_height) = required_source_dimensions(
            report.css_width,
            report.css_height,
            report.device_pixel_ratio,
        );
        let next_photo_rect = if required_width == 0 || required_height == 0 {
            None
        } else {
            Some(ViewerPhotoRectDto {
                css_width: report.css_width,
                css_height: report.css_height,
                device_pixel_ratio: report.device_pixel_ratio,
                required_source_width_px: required_width,
                required_source_height_px: required_height,
            })
        };
        let next_layout_ready = report.layout_ready && next_photo_rect.is_some();

        if self.layout_ready == next_layout_ready
            && self.photo_rect == next_photo_rect
            && self.reported_session_id.as_deref() == report.session_id.as_deref()
        {
            return ReportOutcome::Unchanged;
        }

        self.layout_ready = next_layout_ready;
        self.photo_rect = next_photo_rect;
        self.reported_session_id = report.session_id.clone();
        self.bump();

        ReportOutcome::Applied
    }

    fn resolve_reason(&self, monotonic_ms: u64) -> &'static str {
        match self.window_state {
            VIEWER_WINDOW_STATE_CLOSED => return VIEWER_REASON_WINDOW_CLOSED,
            VIEWER_WINDOW_STATE_OPEN => {}
            _ => return VIEWER_REASON_ABSENT,
        }

        if self.monitor_targeting != MONITOR_TARGETING_APPROVED {
            return VIEWER_REASON_MONITOR_NOT_APPROVED;
        }

        if self.session_id.is_none() {
            return VIEWER_REASON_SESSION_UNBOUND;
        }

        if !self.listener_ready {
            return if self.saw_stale_epoch_report {
                VIEWER_REASON_STALE_EPOCH
            } else {
                VIEWER_REASON_LISTENER_NOT_READY
            };
        }

        if !self.layout_ready || self.photo_rect.is_none() {
            return if self.saw_stale_epoch_report {
                VIEWER_REASON_STALE_EPOCH
            } else {
                VIEWER_REASON_LAYOUT_NOT_READY
            };
        }

        if self.reported_session_id.as_deref() != self.session_id.as_deref() {
            return VIEWER_REASON_SESSION_MISMATCH;
        }

        match self.last_report_monotonic_ms {
            Some(last_report_monotonic_ms)
                if monotonic_ms.saturating_sub(last_report_monotonic_ms)
                    <= VIEWER_REPORT_STALE_AFTER_MS =>
            {
                VIEWER_REASON_READY
            }
            _ => VIEWER_REASON_STALE_REPORT,
        }
    }

    pub fn snapshot(&self, now_ms: u64) -> ViewerReadinessSnapshotDto {
        self.snapshot_with_clock(now_ms, now_ms)
    }

    pub fn snapshot_with_clock(
        &self,
        observed_at_ms: u64,
        monotonic_ms: u64,
    ) -> ViewerReadinessSnapshotDto {
        let reason_code = self.resolve_reason(monotonic_ms);

        ViewerReadinessSnapshotDto {
            schema_version: VIEWER_READINESS_SCHEMA_VERSION.into(),
            session_id: self.session_id.clone(),
            viewer_epoch: self.viewer_epoch,
            revision: self.revision,
            window_state: self.window_state.into(),
            listener_ready: self.listener_ready,
            layout_ready: self.layout_ready,
            monitor_targeting: self.monitor_targeting.into(),
            display_profile: self.display_profile.clone(),
            photo_rect: self.photo_rect.clone(),
            viewer_ready: reason_code == VIEWER_REASON_READY,
            reason_code: reason_code.into(),
            observed_at_ms,
            last_report_at_ms: self.last_report_at_ms,
        }
    }
}
