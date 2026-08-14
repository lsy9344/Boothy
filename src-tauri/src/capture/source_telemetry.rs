//! Story 7.3: source 비교 lane의 스위치와 표본 기록.
//!
//! **이 lane은 계측 도구이지 제품 경로가 아니다.** 기본값은 `off`이고, off일 때 제품 경로는
//! 지금과 완전히 동일하게 동작한다.
//!
//! 기록은 Story 7.2의 `viewer-present.jsonl`과 **의도적으로 분리된 파일**에 남는다.
//! 두 파일을 합쳐 집계하는 도구를 만들지 않는다 — 합치는 순간 보정되지 않은 JPEG의 빠른
//! 도착 시각이 preset-applied KPI를 실제보다 좋아 보이게 만든다.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{
    capture::{
        embedded_jpeg::extract_embedded_jpeg,
        sidecar_client::read_latest_source_object_arrival,
        source_probe::{
            block_order_for, evaluate_source_candidate, probe_source_bytes, SourceAdmissionContext,
            SourceProducerOutcome,
        },
    },
    contracts::dto::{
        is_known_source_reject_reason, is_known_source_route, HostErrorEnvelope,
        SourceCandidateDto, SourceComparisonSampleDto, SOURCE_COMPARE_MODE_AB,
        SOURCE_COMPARE_MODE_EMBEDDED, SOURCE_COMPARE_MODE_OFF, SOURCE_COMPARE_MODE_PAIRED,
        SOURCE_COMPARE_MODE_SHELL, SOURCE_COMPARISON_SCHEMA_VERSION, SOURCE_OBJECT_ROLE_JPEG,
        SOURCE_REJECT_ABSENT, SOURCE_REJECT_EXTRACTION_FAILED, SOURCE_REJECT_WRONG_CAPTURE,
        SOURCE_ROUTE_CAMERA_PAIRED_JPEG, SOURCE_ROUTE_EMBEDDED_JPEG,
        SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL,
    },
    display::image_probe::content_hash,
    session::{session_manifest::SessionCaptureRecord, session_paths::SessionPaths},
};

pub const SOURCE_COMPARE_MODE_ENV: &str = "BOOTHY_SOURCE_COMPARE_MODE";
pub const SOURCE_COMPARISON_FILE_NAME: &str = "source-comparison.jsonl";

/// 측정 산출물이 놓이는 **session-scoped 전용 디렉터리**.
///
/// `renders/previews/`(canonical fast preview 경로)를 절대 건드리지 않는다. 그 경로는 이미
/// booth 사진 레일에 표시되므로, 측정 산출물을 그곳에 쓰면 lane이 제품 UI를 조용히 바꾼다.
pub const SOURCE_ARTIFACT_DIR_NAME: &str = "sources";

/// source 비교 lane 모드. 기본은 반드시 `Off`다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCompareMode {
    Off,
    Embedded,
    Paired,
    Shell,
    Ab,
}

impl SourceCompareMode {
    pub fn is_enabled(self) -> bool {
        !matches!(self, SourceCompareMode::Off)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SourceCompareMode::Off => SOURCE_COMPARE_MODE_OFF,
            SourceCompareMode::Embedded => SOURCE_COMPARE_MODE_EMBEDDED,
            SourceCompareMode::Paired => SOURCE_COMPARE_MODE_PAIRED,
            SourceCompareMode::Shell => SOURCE_COMPARE_MODE_SHELL,
            SourceCompareMode::Ab => SOURCE_COMPARE_MODE_AB,
        }
    }

    /// 이 모드에서 측정 대상인 route 목록.
    ///
    /// `Ab`는 세 route를 모두 잰다. **incumbent를 빼지 않는다** — 기준선이 없으면
    /// "새 route가 더 빠르다"는 주장에 비교 대상이 없다.
    pub fn enabled_routes(self) -> &'static [&'static str] {
        match self {
            SourceCompareMode::Off => &[],
            SourceCompareMode::Embedded => &[SOURCE_ROUTE_EMBEDDED_JPEG],
            SourceCompareMode::Paired => &[SOURCE_ROUTE_CAMERA_PAIRED_JPEG],
            SourceCompareMode::Shell => &[SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL],
            SourceCompareMode::Ab => &[
                SOURCE_ROUTE_EMBEDDED_JPEG,
                SOURCE_ROUTE_CAMERA_PAIRED_JPEG,
                SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL,
            ],
        }
    }
}

/// 알 수 없는 값은 전부 `Off`로 떨어뜨린다. 오타가 lane을 켜는 일은 없어야 한다.
pub fn parse_source_compare_mode(raw: Option<&str>) -> SourceCompareMode {
    match raw.map(str::trim) {
        Some(SOURCE_COMPARE_MODE_EMBEDDED) => SourceCompareMode::Embedded,
        Some(SOURCE_COMPARE_MODE_PAIRED) => SourceCompareMode::Paired,
        Some(SOURCE_COMPARE_MODE_SHELL) => SourceCompareMode::Shell,
        Some(SOURCE_COMPARE_MODE_AB) => SourceCompareMode::Ab,
        _ => SourceCompareMode::Off,
    }
}

pub fn current_source_compare_mode() -> SourceCompareMode {
    parse_source_compare_mode(std::env::var(SOURCE_COMPARE_MODE_ENV).ok().as_deref())
}

/// 두 계측 lane이 동시에 켜졌는지 확인한다.
///
/// 같은 촬영에 두 lane이 붙으면 표본이 서로 오염된다. 끄지는 않는다 — 조용히 끄면 왜 표본이
/// 비었는지 알 수 없다. 경고를 남기고 호출자가 source lane을 우선하게 한다.
pub fn warn_if_lanes_conflict(
    source_mode: SourceCompareMode,
    display_lane_enabled: bool,
) -> Option<&'static str> {
    if source_mode.is_enabled() && display_lane_enabled {
        let warning = "source-compare lane과 display-sample lane이 동시에 켜져 있습니다. \
표본 오염을 막기 위해 source lane을 우선합니다.";
        log::warn!("{warning}");
        return Some(warning);
    }

    None
}

/// 측정 산출물 디렉터리. `<session_root>/renders/sources/`.
pub fn source_artifact_dir(base_dir: &Path, session_id: &str) -> PathBuf {
    SessionPaths::new(base_dir, session_id)
        .renders_previews_dir
        .parent()
        .map(|renders| renders.join(SOURCE_ARTIFACT_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(SOURCE_ARTIFACT_DIR_NAME))
}

pub fn source_artifact_path(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    route_suffix: &str,
) -> PathBuf {
    source_artifact_dir(base_dir, session_id).join(format!("{capture_id}-{route_suffix}.jpg"))
}

pub fn source_comparison_path(base_dir: &Path, session_id: &str) -> PathBuf {
    SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(SOURCE_COMPARISON_FILE_NAME)
}

/// 표본 1행을 기록한다.
///
/// **모든 시도에 행이 하나 남아야 한다.** 거부된 시도도, 후보가 아예 없던 시도도 남는다.
/// 행이 없는 시도가 생기면 분모가 조용히 줄어 성공률이 실제보다 좋아 보인다.
/// Story 7.2가 정확히 이 결함으로 한 회차를 잃었다.
pub fn append_source_comparison_sample(
    base_dir: &Path,
    session_id: &str,
    sample: &SourceComparisonSampleDto,
) -> Result<(), HostErrorEnvelope> {
    validate_source_comparison_sample(session_id, sample)?;

    let path = source_comparison_path(base_dir, session_id);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            HostErrorEnvelope::persistence(format!("source 계측 경로를 준비하지 못했어요: {error}"))
        })?;
    }

    let mut line = serde_json::to_string(sample).map_err(|error| {
        HostErrorEnvelope::persistence(format!("source 계측 기록을 준비하지 못했어요: {error}"))
    })?;
    line.push('\n');

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!("source 계측 기록을 열지 못했어요: {error}"))
        })?;

    file.write_all(line.as_bytes()).map_err(|error| {
        HostErrorEnvelope::persistence(format!("source 계측 기록을 저장하지 못했어요: {error}"))
    })
}

pub fn read_source_comparison_samples(
    base_dir: &Path,
    session_id: &str,
) -> Result<Vec<SourceComparisonSampleDto>, HostErrorEnvelope> {
    let path = source_comparison_path(base_dir, session_id);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(&path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("source 계측 기록을 읽지 못했어요: {error}"))
    })?;

    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| {
            let sample: SourceComparisonSampleDto =
                serde_json::from_str(line).map_err(|error| {
                    HostErrorEnvelope::persistence(format!(
                        "source 계측 기록 {}행이 손상됐어요: {error}",
                        index + 1
                    ))
                })?;
            validate_source_comparison_sample(session_id, &sample)?;
            Ok(sample)
        })
        .collect()
}

/// 표본 1행을 만든다. `is_preset_applied`는 여기서 **항상 false로 고정된다.**
///
/// 호출자가 이 값을 정할 수 있게 두지 않는 것이 요점이다. FR-010의 qualifying frame은
/// "같은 capture의 preset이 적용된 이미지"이며, 이 lane의 어떤 산출물도 거기에 해당하지 않는다.
#[allow(clippy::too_many_arguments)]
pub fn build_source_comparison_sample(
    candidate: crate::contracts::dto::SourceCandidateDto,
    accepted: bool,
    reject_reason: Option<&str>,
    object_role: Option<&str>,
    used_fallback_correlation: bool,
    block_order: &str,
    block_index: u32,
    is_warm_up: bool,
    randomization_seed: u64,
    recorded_at_host_micros: u64,
) -> Result<SourceComparisonSampleDto, HostErrorEnvelope> {
    if accepted != reject_reason.is_none() {
        return Err(HostErrorEnvelope::persistence(
            "source 승격 결과와 거부 사유가 서로 모순돼요.",
        ));
    }

    if let Some(reason) = reject_reason {
        if !is_known_source_reject_reason(reason) {
            return Err(HostErrorEnvelope::persistence(
                "알 수 없는 source 거부 사유예요.",
            ));
        }
    }

    let sample = SourceComparisonSampleDto {
        schema_version: SOURCE_COMPARISON_SCHEMA_VERSION.into(),
        candidate,
        accepted,
        reject_reason: reject_reason.map(str::to_string),
        object_role: object_role.map(str::to_string),
        used_fallback_correlation,
        block_order: block_order.into(),
        block_index,
        is_warm_up,
        randomization_seed,
        is_preset_applied: false,
        recorded_at_host_micros,
    };
    validate_source_comparison_sample(&sample.candidate.session_id, &sample)?;
    Ok(sample)
}

fn validate_source_comparison_sample(
    session_id: &str,
    sample: &SourceComparisonSampleDto,
) -> Result<(), HostErrorEnvelope> {
    let valid_outcome = sample.accepted == sample.reject_reason.is_none();
    let valid_candidate = sample.candidate.session_id == session_id
        && is_known_source_route(&sample.candidate.route)
        && sample
            .reject_reason
            .as_deref()
            .map(is_known_source_reject_reason)
            .unwrap_or(true);
    let missing_measurements_are_null = sample.candidate.asset_path.is_some()
        || (sample.candidate.width_px.is_none()
            && sample.candidate.height_px.is_none()
            && sample.candidate.byte_size.is_none()
            && sample.candidate.object_index.is_none()
            && !sample.candidate.decode_valid
            && sample.candidate.source_hash.is_none());
    let valid_hash = sample
        .candidate
        .source_hash
        .as_deref()
        .map(|value| {
            value.len() == "fnv1a64:".len() + 16
                && value.starts_with("fnv1a64:")
                && value["fnv1a64:".len()..]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        })
        .unwrap_or(true);
    let valid_orientation = sample
        .candidate
        .exif_orientation
        .map(|value| (1..=8).contains(&value))
        .unwrap_or(true);
    let valid_shape = matches!(sample.block_order.as_str(), "AB" | "BA")
        && sample
            .object_role
            .as_deref()
            .map(|role| matches!(role, "raw" | "jpeg"))
            .unwrap_or(true);
    let valid_paired_correlation = sample.candidate.route != SOURCE_ROUTE_CAMERA_PAIRED_JPEG
        || !sample.accepted
        || (sample.candidate.object_index.is_some()
            && (sample.candidate.group_id.is_some() || sample.used_fallback_correlation));
    let valid_accepted_evidence = !sample.accepted
        || (sample.candidate.source_hash.is_some()
            && sample.object_role.as_deref() == Some(SOURCE_OBJECT_ROLE_JPEG));
    let valid_fallback_route = !sample.used_fallback_correlation
        || sample.candidate.route == SOURCE_ROUTE_CAMERA_PAIRED_JPEG;

    if sample.schema_version != SOURCE_COMPARISON_SCHEMA_VERSION
        || sample.is_preset_applied
        || !valid_outcome
        || !valid_candidate
        || !missing_measurements_are_null
        || !valid_hash
        || !valid_orientation
        || !valid_shape
        || !valid_paired_correlation
        || !valid_accepted_evidence
        || !valid_fallback_route
    {
        return Err(HostErrorEnvelope::persistence(
            "source 계측 표본의 불변식이 깨졌어요.",
        ));
    }

    Ok(())
}

/// RAW가 안전하게 저장된 뒤 experimental source lane을 한 번 실행한다.
/// `Off`에서는 파일을 읽거나 디렉터리를 만들지 않는다.
pub fn run_source_comparison_for_capture(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
    fast_preview_kind: Option<&str>,
    request_started_at_host_micros: u64,
) -> Result<usize, HostErrorEnvelope> {
    let mode = current_source_compare_mode();
    let _ = warn_if_lanes_conflict(
        mode,
        crate::display::sample_publisher::current_sample_lane_mode().is_enabled(),
    );
    run_source_comparison_for_capture_with_mode(
        base_dir,
        capture,
        fast_preview_kind,
        mode,
        request_started_at_host_micros,
    )
}

pub fn run_source_comparison_for_capture_with_mode(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
    fast_preview_kind: Option<&str>,
    mode: SourceCompareMode,
    request_started_at_host_micros: u64,
) -> Result<usize, HostErrorEnvelope> {
    if !mode.is_enabled() {
        return Ok(0);
    }

    let previous = read_source_comparison_samples(base_dir, &capture.session_id)?;
    let routes_per_block = mode.enabled_routes().len().max(1);
    if previous.len() % routes_per_block != 0 {
        return Err(HostErrorEnvelope::persistence(
            "source 계측 기록의 마지막 block이 불완전해요. 기존 증거를 복구한 뒤 다시 측정해 주세요.",
        ));
    }
    let block_index = (previous.len() / routes_per_block) as u32;
    let seed = stable_seed(&capture.session_id);
    let block_order = block_order_for(seed, block_index);
    let mut routes = mode.enabled_routes().to_vec();
    if mode == SourceCompareMode::Ab && block_order == crate::contracts::dto::SOURCE_BLOCK_ORDER_BA
    {
        routes.swap(0, 1);
    }

    let is_warm_up = block_index < 5;

    for route in routes {
        let measurement = measure_route(base_dir, capture, route, fast_preview_kind);
        let (candidate, used_fallback_correlation, object_role, direct_reject) = measurement;
        let context = SourceAdmissionContext {
            bound_session_id: Some(&capture.session_id),
            active_request_id: Some(&capture.request_id),
            expected_capture_id: Some(&capture.capture_id),
            request_started_at_host_micros,
            enabled_routes: mode.enabled_routes(),
            used_fallback_correlation,
        };

        let bytes = candidate
            .asset_path
            .as_deref()
            .and_then(|path| fs::read(path).ok());
        let result = direct_reject.map_or_else(
            || {
                evaluate_source_candidate(
                    &candidate,
                    SourceProducerOutcome::Produced,
                    bytes.as_deref(),
                    &context,
                )
            },
            Err,
        );
        let sample = build_source_comparison_sample(
            candidate,
            result.is_ok(),
            result.err(),
            object_role,
            used_fallback_correlation,
            block_order,
            block_index,
            is_warm_up,
            seed,
            crate::viewer::current_monotonic_micros(),
        )?;
        append_source_comparison_sample(base_dir, &capture.session_id, &sample)?;
    }

    Ok(mode.enabled_routes().len())
}

fn measure_route(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
    route: &str,
    fast_preview_kind: Option<&str>,
) -> (
    SourceCandidateDto,
    bool,
    Option<&'static str>,
    Option<&'static str>,
) {
    let started = Instant::now();
    let (path, used_fallback_correlation, object_index, group_id, direct_reject) = match route {
        SOURCE_ROUTE_EMBEDDED_JPEG => {
            let raw = match fs::read(&capture.raw.asset_path) {
                Ok(raw) => raw,
                Err(_) => {
                    return (
                        missing_candidate(capture, route, started),
                        false,
                        Some(SOURCE_OBJECT_ROLE_JPEG),
                        Some(SOURCE_REJECT_EXTRACTION_FAILED),
                    )
                }
            };
            match extract_embedded_jpeg(&raw) {
                Ok(extracted) => {
                    let destination = source_artifact_path(
                        base_dir,
                        &capture.session_id,
                        &capture.capture_id,
                        "embedded",
                    );
                    let jpeg = &raw[extracted.offset..extracted.offset + extracted.byte_size];
                    if persist_source_artifact(&destination, jpeg).is_err() {
                        return (
                            missing_candidate(capture, route, started),
                            false,
                            Some(SOURCE_OBJECT_ROLE_JPEG),
                            Some(SOURCE_REJECT_EXTRACTION_FAILED),
                        );
                    }
                    (Some(destination), false, Some(0), None, None)
                }
                Err(reason) => (None, false, None, None, Some(reason)),
            }
        }
        SOURCE_ROUTE_CAMERA_PAIRED_JPEG => {
            let path =
                source_artifact_path(base_dir, &capture.session_id, &capture.capture_id, "paired");
            if path.is_file() {
                let correlation = match read_latest_source_object_arrival(
                    base_dir,
                    &capture.session_id,
                    &capture.request_id,
                    &capture.capture_id,
                ) {
                    Ok(Some(correlation))
                        if correlation.object_role == SOURCE_OBJECT_ROLE_JPEG
                            && Path::new(&correlation.asset_path) == path
                            && correlation.byte_size > 0 =>
                    {
                        correlation
                    }
                    _ => {
                        return (
                            candidate_from_path(capture, route, &path, None, None, started),
                            false,
                            Some(SOURCE_OBJECT_ROLE_JPEG),
                            Some(SOURCE_REJECT_WRONG_CAPTURE),
                        )
                    }
                };
                (
                    Some(path),
                    correlation.used_fallback_correlation,
                    Some(correlation.object_index),
                    correlation.group_id,
                    None,
                )
            } else {
                (None, false, None, None, Some(SOURCE_REJECT_ABSENT))
            }
        }
        SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL => {
            if fast_preview_kind != Some("windows-shell-thumbnail") {
                (None, false, None, None, Some(SOURCE_REJECT_ABSENT))
            } else if let Some(source) = capture.preview.asset_path.as_deref() {
                let destination = source_artifact_path(
                    base_dir,
                    &capture.session_id,
                    &capture.capture_id,
                    "shell",
                );
                match fs::read(source)
                    .and_then(|bytes| persist_source_artifact(&destination, &bytes).map(|_| bytes))
                {
                    Ok(_) => (Some(destination), false, Some(0), None, None),
                    Err(_) => (
                        None,
                        false,
                        None,
                        None,
                        Some(SOURCE_REJECT_EXTRACTION_FAILED),
                    ),
                }
            } else {
                (None, false, None, None, Some(SOURCE_REJECT_ABSENT))
            }
        }
        _ => (None, false, None, None, Some(SOURCE_REJECT_ABSENT)),
    };

    let candidate = match path {
        Some(path) => candidate_from_path(capture, route, &path, object_index, group_id, started),
        None => missing_candidate(capture, route, started),
    };

    (
        candidate,
        used_fallback_correlation,
        Some(SOURCE_OBJECT_ROLE_JPEG),
        direct_reject,
    )
}

fn candidate_from_path(
    capture: &SessionCaptureRecord,
    route: &str,
    path: &Path,
    object_index: Option<u32>,
    group_id: Option<u32>,
    started: Instant,
) -> SourceCandidateDto {
    let bytes = fs::read(path).unwrap_or_default();
    let probe = probe_source_bytes(&bytes).ok();
    SourceCandidateDto {
        capture_id: Some(capture.capture_id.clone()),
        request_id: capture.request_id.clone(),
        session_id: capture.session_id.clone(),
        route: route.into(),
        asset_path: Some(path.to_string_lossy().into_owned()),
        width_px: probe.as_ref().map(|value| value.width_px),
        height_px: probe.as_ref().map(|value| value.height_px),
        byte_size: Some(bytes.len() as u64),
        exif_orientation: probe.as_ref().and_then(|value| value.orientation),
        decode_valid: probe.is_some(),
        source_hash: Some(content_hash(&bytes)),
        object_index,
        group_id,
        ready_at_host_micros: Some(crate::viewer::current_monotonic_micros()),
        extraction_cost_micros: Some(started.elapsed().as_micros() as u64),
    }
}

fn missing_candidate(
    capture: &SessionCaptureRecord,
    route: &str,
    started: Instant,
) -> SourceCandidateDto {
    SourceCandidateDto {
        capture_id: Some(capture.capture_id.clone()),
        request_id: capture.request_id.clone(),
        session_id: capture.session_id.clone(),
        route: route.into(),
        asset_path: None,
        width_px: None,
        height_px: None,
        byte_size: None,
        exif_orientation: None,
        decode_valid: false,
        source_hash: None,
        object_index: None,
        group_id: None,
        ready_at_host_micros: Some(crate::viewer::current_monotonic_micros()),
        extraction_cost_micros: Some(started.elapsed().as_micros() as u64),
    }
}

fn persist_source_artifact(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("missing parent"))?;
    fs::create_dir_all(parent)?;
    let temp = path.with_extension("downloading.jpg");
    fs::write(&temp, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp, path)
}

fn stable_seed(session_id: &str) -> u64 {
    content_hash(session_id.as_bytes())
        .strip_prefix("fnv1a64:")
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dto::SourceCandidateDto;

    const SESSION: &str = "session_01hzzzzzzzzzzzzzzzzzzzzzzz";

    fn candidate() -> SourceCandidateDto {
        SourceCandidateDto {
            capture_id: Some("capture_0001".into()),
            request_id: "capture_req_0001".into(),
            session_id: SESSION.into(),
            route: SOURCE_ROUTE_EMBEDDED_JPEG.into(),
            asset_path: Some("C:/tmp/a.jpg".into()),
            width_px: Some(5184),
            height_px: Some(3456),
            byte_size: Some(1024),
            exif_orientation: Some(1),
            decode_valid: true,
            source_hash: Some("fnv1a64:0000000000000000".into()),
            object_index: Some(0),
            group_id: Some(3),
            ready_at_host_micros: Some(5_000),
            extraction_cost_micros: Some(12_000),
        }
    }

    #[test]
    fn default_mode_is_off() {
        assert_eq!(parse_source_compare_mode(None), SourceCompareMode::Off);
        assert!(!SourceCompareMode::Off.is_enabled());
        assert!(SourceCompareMode::Off.enabled_routes().is_empty());
    }

    #[test]
    fn unknown_mode_values_fall_back_to_off() {
        // `libraw`는 이 lane이 LibRaw 의존성을 쓰던 시절의 이름이다. 이제 유효한 값이 아니며
        // 남아 있는 설정이 lane을 조용히 켜지 않는다.
        for raw in [
            "", " ", "on", "true", "1", "EMBEDDED", "libraw", "embed", "all",
        ] {
            assert_eq!(
                parse_source_compare_mode(Some(raw)),
                SourceCompareMode::Off,
                "unexpected mode for {raw:?}"
            );
        }
    }

    #[test]
    fn known_mode_values_parse() {
        assert_eq!(
            parse_source_compare_mode(Some("embedded")),
            SourceCompareMode::Embedded
        );
        assert_eq!(
            parse_source_compare_mode(Some("paired")),
            SourceCompareMode::Paired
        );
        assert_eq!(
            parse_source_compare_mode(Some("shell")),
            SourceCompareMode::Shell
        );
        assert_eq!(
            parse_source_compare_mode(Some(" ab ")),
            SourceCompareMode::Ab
        );
    }

    /// AB 모드는 incumbent를 반드시 포함한다.
    #[test]
    fn ab_mode_measures_the_incumbent_baseline_too() {
        let routes = SourceCompareMode::Ab.enabled_routes();

        assert_eq!(routes.len(), 3);
        assert!(routes.contains(&SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL));
    }

    #[test]
    fn conflicting_lanes_are_reported() {
        assert!(warn_if_lanes_conflict(SourceCompareMode::Ab, true).is_some());
        assert!(warn_if_lanes_conflict(SourceCompareMode::Ab, false).is_none());
        assert!(warn_if_lanes_conflict(SourceCompareMode::Off, true).is_none());
    }

    /// 측정 산출물은 canonical preview 경로를 건드리지 않는다.
    #[test]
    fn source_artifacts_never_land_in_the_canonical_preview_dir() {
        let base = Path::new("C:/tmp/boothy");
        let artifact = source_artifact_path(base, SESSION, "capture_0001", "embedded");
        let paths = SessionPaths::new(base, SESSION);

        assert!(artifact.starts_with(&paths.session_root));
        assert!(!artifact.starts_with(&paths.renders_previews_dir));
        assert!(artifact
            .parent()
            .is_some_and(|parent| parent.ends_with(SOURCE_ARTIFACT_DIR_NAME)));
    }

    #[test]
    fn comparison_file_is_separate_from_present_telemetry() {
        let base = Path::new("C:/tmp/boothy");
        let comparison = source_comparison_path(base, SESSION);

        assert!(comparison.ends_with(SOURCE_COMPARISON_FILE_NAME));
        assert!(
            !comparison.ends_with(crate::viewer::present_telemetry::PRESENT_TELEMETRY_FILE_NAME)
        );
    }

    #[test]
    fn built_samples_are_never_marked_preset_applied() {
        let sample = build_source_comparison_sample(
            candidate(),
            true,
            None,
            Some("jpeg"),
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .expect("valid sample");

        assert!(!sample.is_preset_applied);
        assert_eq!(sample.schema_version, SOURCE_COMPARISON_SCHEMA_VERSION);
    }

    #[test]
    fn rejected_attempts_still_produce_a_row() {
        let temp =
            std::env::temp_dir().join(format!("boothy-source-telemetry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);

        let accepted = build_source_comparison_sample(
            candidate(),
            true,
            None,
            Some("jpeg"),
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .expect("valid accepted sample");
        let rejected = build_source_comparison_sample(
            candidate(),
            false,
            Some(crate::contracts::dto::SOURCE_REJECT_ABSENT),
            None,
            false,
            "BA",
            1,
            false,
            42,
            9_500,
        )
        .expect("valid rejected sample");

        append_source_comparison_sample(&temp, SESSION, &accepted).expect("append accepted");
        append_source_comparison_sample(&temp, SESSION, &rejected).expect("append rejected");

        let samples = read_source_comparison_samples(&temp, SESSION).expect("read samples");

        assert_eq!(samples.len(), 2, "모든 시도에 행이 하나씩 남아야 한다");
        assert!(samples.iter().all(|sample| !sample.is_preset_applied));
        assert_eq!(
            samples[1].reject_reason.as_deref(),
            Some(crate::contracts::dto::SOURCE_REJECT_ABSENT)
        );
        assert!(!samples[1].used_fallback_correlation);

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn contradictory_outcomes_are_rejected_before_persistence() {
        assert!(build_source_comparison_sample(
            candidate(),
            true,
            Some(crate::contracts::dto::SOURCE_REJECT_CORRUPT),
            Some("jpeg"),
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .is_err());
    }

    #[test]
    fn accepted_samples_require_hash_and_jpeg_role() {
        let mut missing_hash = candidate();
        missing_hash.source_hash = None;
        assert!(build_source_comparison_sample(
            missing_hash,
            true,
            None,
            Some("jpeg"),
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .is_err());

        assert!(build_source_comparison_sample(
            candidate(),
            true,
            None,
            None,
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .is_err());
        assert!(build_source_comparison_sample(
            candidate(),
            true,
            None,
            Some("raw"),
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .is_err());
    }

    #[test]
    fn fallback_correlation_is_rejected_outside_the_paired_route() {
        assert!(build_source_comparison_sample(
            candidate(),
            false,
            Some(crate::contracts::dto::SOURCE_REJECT_ABSENT),
            None,
            true,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .is_err());

        let mut paired = candidate();
        paired.route = SOURCE_ROUTE_CAMERA_PAIRED_JPEG.into();
        paired.group_id = None;
        assert!(build_source_comparison_sample(
            paired,
            false,
            Some(crate::contracts::dto::SOURCE_REJECT_ABSENT),
            None,
            true,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .is_ok());
    }

    #[test]
    fn release_path_rejects_a_sample_claiming_preset_was_applied() {
        let temp =
            std::env::temp_dir().join(format!("boothy-source-invariant-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let mut sample = build_source_comparison_sample(
            candidate(),
            true,
            None,
            Some("jpeg"),
            false,
            "AB",
            0,
            false,
            42,
            9_000,
        )
        .expect("valid sample");
        sample.is_preset_applied = true;

        assert!(append_source_comparison_sample(&temp, SESSION, &sample).is_err());
        assert!(!source_comparison_path(&temp, SESSION).exists());
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn malformed_jsonl_is_an_error_instead_of_a_missing_sample() {
        let temp =
            std::env::temp_dir().join(format!("boothy-source-malformed-{}", std::process::id()));
        let path = source_comparison_path(&temp, SESSION);
        std::fs::create_dir_all(path.parent().expect("diagnostics parent"))
            .expect("diagnostics dir");
        std::fs::write(&path, "{\"truncated\":\n").expect("write malformed row");

        assert!(read_source_comparison_samples(&temp, SESSION).is_err());
        let _ = std::fs::remove_dir_all(&temp);
    }
}
