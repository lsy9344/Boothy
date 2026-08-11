//! 승인된 고객 모니터 선택과 physical display profile 분류.
//!
//! Tauri 타입에 의존하지 않는 순수 로직만 둔다. `tauri::Monitor` -> `MonitorDescriptor`
//! 변환은 command/setup 경계에서 수행한다.

/// 승인된 booth 카메라(EOS 700D) 출력과 동일한 3:2를 고정한다.
pub const PHOTO_ASPECT_RATIO: f64 = 3.0 / 2.0;

pub const MONITOR_TARGETING_UNRESOLVED: &str = "unresolved";
pub const MONITOR_TARGETING_APPROVED: &str = "approved-customer-monitor";
pub const MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK: &str = "single-monitor-fallback";
pub const MONITOR_TARGETING_UNAPPROVED_PROFILE: &str = "unapproved-profile";
pub const MONITOR_TARGETING_UNAVAILABLE: &str = "monitor-unavailable";

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorDescriptor {
    pub name: Option<String>,
    pub width_px: u32,
    pub height_px: u32,
    pub scale_factor: f64,
    pub position_x: i32,
    pub position_y: i32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorSelection {
    pub monitor: Option<MonitorDescriptor>,
    pub targeting: &'static str,
    pub profile_id: Option<&'static str>,
}

/// 승인된 display profile 분류. 목록 밖 해상도는 `None`이며 호출자는
/// `unapproved-profile`로 보고한다.
pub fn classify_display_profile(width_px: u32, height_px: u32) -> Option<&'static str> {
    match (width_px, height_px) {
        (1920, 1080) => Some("1080p"),
        (2560, 1440) => Some("1440p"),
        (3840, 2160) => Some("4k"),
        _ => None,
    }
}

/// 고객 모니터를 결정적으로 선택한다.
///
/// 우선순위:
/// 1. 승인된 모니터 이름이 설정되어 있으면 이름이 일치하는 모니터만 사용한다.
///    일치하는 모니터가 없으면 임의 대체 없이 `monitor-unavailable`로 보고한다.
/// 2. 이름 설정이 없고 모니터가 2개 이상이면 primary가 아닌 첫 모니터를 사용한다.
/// 3. 모니터가 하나뿐이면 그 모니터를 쓰되 `single-monitor-fallback`으로 정직하게 보고한다.
pub fn select_customer_monitor(
    monitors: &[MonitorDescriptor],
    approved_monitor_name: Option<&str>,
) -> MonitorSelection {
    if monitors.is_empty() {
        return MonitorSelection {
            monitor: None,
            targeting: MONITOR_TARGETING_UNAVAILABLE,
            profile_id: None,
        };
    }

    let (selected, targeting) = match approved_monitor_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        Some(approved_name) => match monitors
            .iter()
            .find(|monitor| monitor.name.as_deref() == Some(approved_name))
        {
            Some(monitor) => (monitor, MONITOR_TARGETING_APPROVED),
            None => {
                return MonitorSelection {
                    monitor: None,
                    targeting: MONITOR_TARGETING_UNAVAILABLE,
                    profile_id: None,
                }
            }
        },
        None => {
            if monitors.len() == 1 {
                (&monitors[0], MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK)
            } else {
                match monitors.iter().find(|monitor| !monitor.is_primary) {
                    Some(monitor) => (monitor, MONITOR_TARGETING_APPROVED),
                    None => (&monitors[0], MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK),
                }
            }
        }
    };

    let profile_id = classify_display_profile(selected.width_px, selected.height_px);
    let targeting = if profile_id.is_none() {
        MONITOR_TARGETING_UNAPPROVED_PROFILE
    } else {
        targeting
    };

    MonitorSelection {
        monitor: Some(selected.clone()),
        targeting,
        profile_id,
    }
}

/// CSS rect와 DPR에서 필요한 소스 픽셀 수를 파생한다.
///
/// 올림을 쓰는 이유는 내림이 1px upscale을 허용하기 때문이다. 이 값은 host가
/// viewer report에서 직접 재계산하며 client가 보낸 계산 결과를 신뢰하지 않는다.
pub fn required_source_dimensions(
    css_width: f64,
    css_height: f64,
    device_pixel_ratio: f64,
) -> (u32, u32) {
    let width = (css_width * device_pixel_ratio).ceil().max(0.0);
    let height = (css_height * device_pixel_ratio).ceil().max(0.0);

    (width as u32, height as u32)
}
