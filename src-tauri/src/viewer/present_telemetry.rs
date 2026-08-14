//! Story 7.2: trusted input → actual monitor present 계측.
//!
//! 공식 KPI는 `trusted capture input → 물리 모니터의 qualifying frame`이며 하나의 monotonic
//! clock으로 측정한다. 그 clock은 host의 `Instant`다.
//!
//! booth WebView와 viewer WebView는 서로 다른 time origin을 가지므로, 각 document가
//! `stamp_clock_probe` 왕복으로 host clock에 자기를 보정한 값을 함께 보고한다.
//!
//! **보고 규칙:** 측정 오차는 숨기지 않는다. KPI는 항상 `present 상한 − input 하한`인
//! 보수적 구간으로 계산해, 오차가 결과를 실제보다 빠르게 보이게 만들 수 없도록 한다.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::contracts::dto::{DisplayPresentSpansDto, HostErrorEnvelope};
use crate::session::session_paths::SessionPaths;

pub const PRESENT_TELEMETRY_FILE_NAME: &str = "viewer-present.jsonl";
pub const PRESENT_TELEMETRY_SCHEMA_VERSION: &str = "viewer-present/v1";

/// 이 값을 넘는 총 불확실도를 가진 표본은 `low-confidence`로 표시한다.
/// 버리지 않는다 — 조용히 제외하면 결과가 실제보다 좋아 보인다.
pub const PRESENT_UNCERTAINTY_BUDGET_MICROS: u64 = 5_000;

/// 한 표본의 전 구간 span. 전부 host monotonic micros다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentSampleRecord {
    pub schema_version: String,
    pub session_id: String,
    pub request_id: String,
    pub generation_id: Option<String>,
    pub sample_variant: Option<String>,
    pub lane_mode: String,
    pub outcome: String,
    pub reject_reason: Option<String>,

    // 공식 KPI 양 끝.
    pub trusted_input_at_micros: Option<u64>,
    pub actual_present_at_micros: Option<u64>,
    /// 보수적 구간. `present 상한 − input 하한`.
    pub qualifying_latency_micros: Option<u64>,
    pub total_uncertainty_micros: u64,
    pub confidence: String,
    /// 합성 이벤트로 시작된 표본은 qualifying이 아니다.
    pub is_trusted_input: bool,

    // 진단 span. 어느 것도 KPI 종료점이 아니다.
    pub host_accepted_at_micros: Option<u64>,
    pub sample_write_start_at_micros: Option<u64>,
    pub file_ready_at_micros: Option<u64>,
    pub probe_ok_at_micros: Option<u64>,
    pub pointer_committed_at_micros: Option<u64>,
    pub event_emitted_at_micros: Option<u64>,
    pub viewer_receipt_at_micros: Option<u64>,
    pub decode_start_at_micros: Option<u64>,
    pub decode_end_at_micros: Option<u64>,
    pub swap_committed_at_micros: Option<u64>,
    pub img_on_load_at_micros: Option<u64>,
    pub element_timing_render_at_micros: Option<u64>,
    pub is_element_render_time: bool,

    /// AC 4 실패 조건. trusted input 이후 viewer 창 생성/navigation이 있었는가.
    pub viewer_window_events_after_input: u32,
}

/// viewer window 생성/재생성/navigation 횟수.
///
/// AC 4는 **trusted capture input 이후의 viewer 생성이나 navigation을 실패 결과**로 규정한다.
/// booth가 `get_capture_readiness`를 주기적으로 조회하면서 창을 재생성할 수 있는 경로가 실제로
/// 존재하므로(`capture_commands.rs`의 `ensure_viewer_window_state`), 계측으로 잡아낸다.
static VIEWER_WINDOW_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
const MAX_VIEWER_WINDOW_EVENT_TIMESTAMPS: usize = 4_096;

fn viewer_window_event_timestamps() -> &'static Mutex<VecDeque<u64>> {
    static TIMESTAMPS: OnceLock<Mutex<VecDeque<u64>>> = OnceLock::new();

    TIMESTAMPS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn record_viewer_window_event_at(event_at_micros: u64) {
    let mut timestamps = viewer_window_event_timestamps()
        .lock()
        .expect("viewer window event timestamps lock poisoned");

    if timestamps.len() == MAX_VIEWER_WINDOW_EVENT_TIMESTAMPS {
        timestamps.pop_front();
    }

    timestamps.push_back(event_at_micros);
}

pub fn record_viewer_window_event() {
    VIEWER_WINDOW_EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
    record_viewer_window_event_at(crate::viewer::current_monotonic_micros());
}

pub fn viewer_window_event_count() -> u64 {
    VIEWER_WINDOW_EVENT_COUNT.load(Ordering::SeqCst)
}

fn count_window_events_between(
    timestamps: impl IntoIterator<Item = u64>,
    input_lower_bound_micros: u64,
    present_upper_bound_micros: u64,
) -> u32 {
    if present_upper_bound_micros < input_lower_bound_micros {
        return 0;
    }

    timestamps
        .into_iter()
        .filter(|timestamp| {
            *timestamp >= input_lower_bound_micros && *timestamp <= present_upper_bound_micros
        })
        .count()
        .min(u32::MAX as usize) as u32
}

/// trusted input의 보수적 하한부터 actual-present의 보수적 상한까지 발생한 창 이벤트 수.
/// IPC 도착 순서와 무관하게 host monotonic timestamp로 판정한다.
pub fn viewer_window_event_count_between(
    input_lower_bound_micros: u64,
    present_upper_bound_micros: u64,
) -> u32 {
    let timestamps = viewer_window_event_timestamps()
        .lock()
        .expect("viewer window event timestamps lock poisoned");

    count_window_events_between(
        timestamps.iter().copied(),
        input_lower_bound_micros,
        present_upper_bound_micros,
    )
}

/// 공식 KPI 계산. 항상 가장 넓은 구간을 돌려준다.
///
/// `input_uncertainty`는 시작을 앞으로, `present_uncertainty`는 끝을 뒤로 민다.
/// 어떤 반올림도 결과를 짧게 만들지 않는다.
pub fn conservative_latency_micros(
    trusted_input_at_micros: u64,
    actual_present_at_micros: u64,
    input_uncertainty_micros: u64,
    present_uncertainty_micros: u64,
) -> u64 {
    let lower_input = trusted_input_at_micros.saturating_sub(input_uncertainty_micros);
    let upper_present = actual_present_at_micros.saturating_add(present_uncertainty_micros);

    upper_present.saturating_sub(lower_input)
}

/// 종료점을 관측하지 못한 행의 confidence.
///
/// `measured`로 남기면 관측하지 못한 표본이 관측된 것처럼 집계된다. 이 값을 가진 행은
/// KPI 계산에서 제외하되 **분모에는 포함**해야 성공률이 정직해진다.
pub const PRESENT_CONFIDENCE_UNREPORTED: &str = "unreported";

pub fn classify_confidence(total_uncertainty_micros: u64) -> &'static str {
    if total_uncertainty_micros > PRESENT_UNCERTAINTY_BUDGET_MICROS {
        "low-confidence"
    } else {
        "measured"
    }
}

/// Element Timing `renderTime`을 actual-present로 승격할 수 있는지 판정한다.
///
/// asset protocol은 cross-origin이라 `Timing-Allow-Origin`이 없으면 `renderTime === 0`이 되고
/// `startTime`이 `loadTime`으로 대체된다. 그 값은 paint 시각이 아니므로 승격할 수 없다.
pub fn element_timing_is_qualifying(spans: &DisplayPresentSpansDto) -> bool {
    spans.is_element_render_time && spans.element_timing_render_at_micros.is_some()
}

pub fn telemetry_path(base_dir: &Path, session_id: &str) -> std::path::PathBuf {
    SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(PRESENT_TELEMETRY_FILE_NAME)
}

/// 세션 범위 append-only 기록. cross-session 누출이 없도록 세션 디렉터리 안에만 쓴다.
pub fn append_present_sample(
    base_dir: &Path,
    session_id: &str,
    record: &PresentSampleRecord,
) -> Result<(), HostErrorEnvelope> {
    let path = telemetry_path(base_dir, session_id);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            HostErrorEnvelope::persistence(format!("계측 경로를 준비하지 못했어요: {error}"))
        })?;
    }

    let mut line = serde_json::to_string(record).map_err(|error| {
        HostErrorEnvelope::persistence(format!("계측 기록을 준비하지 못했어요: {error}"))
    })?;
    line.push('\n');

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!("계측 기록을 열지 못했어요: {error}"))
        })?;

    file.write_all(line.as_bytes()).map_err(|error| {
        HostErrorEnvelope::persistence(format!("계측 기록을 저장하지 못했어요: {error}"))
    })
}

pub fn read_present_samples(base_dir: &Path, session_id: &str) -> Vec<PresentSampleRecord> {
    let Ok(raw) = std::fs::read_to_string(telemetry_path(base_dir, session_id)) else {
        return Vec::new();
    };

    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(render_at: Option<u64>, is_render_time: bool) -> DisplayPresentSpansDto {
        DisplayPresentSpansDto {
            viewer_receipt_at_micros: None,
            decode_start_at_micros: None,
            decode_end_at_micros: None,
            swap_committed_at_micros: None,
            img_on_load_at_micros: None,
            actual_present_at_micros: None,
            element_timing_render_at_micros: render_at,
            is_element_render_time: is_render_time,
        }
    }

    #[test]
    fn conservative_latency_widens_the_interval() {
        // 실제 구간 1_000_000us. 오차 300 + 700 → 항상 더 길게 보고한다.
        assert_eq!(
            conservative_latency_micros(1_000_000, 2_000_000, 300, 700),
            1_001_000
        );
    }

    #[test]
    fn conservative_latency_never_reports_faster_than_raw_interval() {
        let raw = 2_000_000 - 1_000_000;

        for input_uncertainty in [0, 1, 500, 5_000] {
            for present_uncertainty in [0, 1, 500, 5_000] {
                let reported = conservative_latency_micros(
                    1_000_000,
                    2_000_000,
                    input_uncertainty,
                    present_uncertainty,
                );

                assert!(
                    reported >= raw,
                    "reported {reported} must not be shorter than {raw}"
                );
            }
        }
    }

    #[test]
    fn confidence_is_flagged_beyond_budget() {
        assert_eq!(classify_confidence(0), "measured");
        assert_eq!(
            classify_confidence(PRESENT_UNCERTAINTY_BUDGET_MICROS),
            "measured"
        );
        assert_eq!(
            classify_confidence(PRESENT_UNCERTAINTY_BUDGET_MICROS + 1),
            "low-confidence"
        );
    }

    #[test]
    fn element_timing_fallback_is_not_qualifying() {
        assert!(!element_timing_is_qualifying(&spans(Some(1_000), false)));
        assert!(!element_timing_is_qualifying(&spans(None, true)));
        assert!(element_timing_is_qualifying(&spans(Some(1_000), true)));
    }

    #[test]
    fn window_events_are_counted_only_inside_the_qualifying_interval() {
        assert_eq!(
            count_window_events_between([900, 1_000, 1_500, 2_000, 2_100], 1_000, 2_000),
            3
        );
        assert_eq!(count_window_events_between([1_500], 2_000, 1_000), 0);
    }
}
