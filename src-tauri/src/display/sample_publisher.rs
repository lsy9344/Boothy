//! Story 7.2: 계측용 sample lane의 스위치와 fixture 위치.
//!
//! **이 lane은 계측 도구이지 제품 경로가 아니다.** 표시되는 이미지는 preset 적용 결과가 아니라
//! fixture이므로 기본값은 `off`이며, 켠 채로 출시하면 실제 고객이 fixture 사진을 본다.
//!
//! 게시 순서 자체는 `display::generation_publisher`가 소유한다. Story 7.4의 proxy lane이
//! **같은 함수**를 호출하므로 이 파일에는 lane 고유의 설정만 남는다.

use std::path::{Path, PathBuf};

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
