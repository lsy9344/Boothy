//! Story 7.2: immutable display generation과 atomic display pointer.
//! Story 7.4: 그 위에 display-fit preset proxy tier를 얹는다.
//!
//! Story 7.6: 그 위에 RAW 정밀본 tier를 얹는다.
//!
//! 이 모듈은 **어떤 자산이 지금 고객 화면의 진실인가**만 소유한다.
//! 우선순위 scheduler와 렌더 실행은 `crate::render::scheduler`가 소유한다 —
//! 대상이 render worker이므로 그쪽이 맞는 자리다.

pub mod display_artifact;
pub mod generation_publisher;
pub mod generation_repository;
pub mod image_probe;
pub mod proxy_publisher;
pub mod raw_refined_publisher;
pub mod resident_renderer;
pub mod sample_publisher;

use std::sync::Mutex;

pub use display_artifact::DisplayState;

/// Tauri `manage`로 등록되는 프로세스 전역 display pointer truth.
///
/// pointer 갱신은 이 Mutex 안에서만 일어난다 (single writer). 게시가 직렬화되므로
/// generation seq 예약과 승격 판정 사이에 경합이 생기지 않는다.
#[derive(Default)]
pub struct DisplayStateHandle(pub Mutex<DisplayState>);

/// pointer snapshot이 싣는 두 lane 플래그.
///
/// **두 값은 서로 다른 질문에 답한다.** 하나로 합치면 proxy lane 회차에서
/// actual-present 행이 0건이 된다 (HV-13B 1차와 같은 실패 모드).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisplayLaneFlags {
    /// 표시 중인 이미지가 계측용 fixture인가? (Story 7.2 sample lane)
    pub measurement_lane_enabled: bool,
    /// booth/viewer가 present 계측 IPC를 해야 하는가? (`sample lane || proxy lane`)
    pub present_telemetry_enabled: bool,
}

impl DisplayLaneFlags {
    /// 어떤 lane도 켜지지 않은 제품 기본 상태.
    pub const fn none() -> Self {
        Self {
            measurement_lane_enabled: false,
            present_telemetry_enabled: false,
        }
    }
}

/// 현재 환경 변수에서 두 lane 상태를 읽는다.
pub fn current_display_lane_flags() -> DisplayLaneFlags {
    let sample_enabled = sample_publisher::current_sample_lane_mode().is_enabled();
    let proxy_enabled = proxy_publisher::current_proxy_lane_mode().is_enabled();

    effective_display_lane_flags(sample_enabled, proxy_enabled)
}

/// 두 lane이 동시에 설정되면 제품 proxy가 fixture sample보다 우선한다.
pub const fn effective_display_lane_flags(
    sample_enabled: bool,
    proxy_enabled: bool,
) -> DisplayLaneFlags {
    let effective_sample_enabled = sample_enabled && !proxy_enabled;

    DisplayLaneFlags {
        measurement_lane_enabled: effective_sample_enabled,
        present_telemetry_enabled: effective_sample_enabled || proxy_enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_lane_wins_when_both_lanes_are_configured() {
        assert_eq!(
            effective_display_lane_flags(true, true),
            DisplayLaneFlags {
                measurement_lane_enabled: false,
                present_telemetry_enabled: true,
            }
        );
    }
}
