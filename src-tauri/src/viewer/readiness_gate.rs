//! Capture eligibility에 viewer readiness를 합성한다.
//!
//! 규칙: **downgrade 전용**. 이미 촬영이 막혀 있으면(post-end, camera 준비 등) 그 원인을
//! 유지하고, 촬영 가능 상태만 viewer 미준비로 내린다.

use crate::contracts::dto::{CaptureReadinessDto, ViewerReadinessSnapshotDto};

pub const CAPTURE_REASON_VIEWER_PREPARING: &str = "viewer-preparing";

pub fn is_viewer_capture_eligible(snapshot: &ViewerReadinessSnapshotDto, session_id: &str) -> bool {
    snapshot.viewer_ready && snapshot.session_id.as_deref() == Some(session_id)
}

pub fn apply_viewer_gate(
    readiness: CaptureReadinessDto,
    snapshot: &ViewerReadinessSnapshotDto,
    session_id: &str,
) -> CaptureReadinessDto {
    if !readiness.can_capture {
        return readiness;
    }

    if is_viewer_capture_eligible(snapshot, session_id) {
        return readiness;
    }

    CaptureReadinessDto {
        surface_state: "blocked".into(),
        customer_state: "Preparing".into(),
        can_capture: false,
        primary_action: "wait".into(),
        customer_message: "화면을 준비하고 있어요.".into(),
        support_message: "곧 촬영을 시작할 수 있어요.".into(),
        reason_code: CAPTURE_REASON_VIEWER_PREPARING.into(),
        ..readiness
    }
}
