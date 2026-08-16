//! Story 7.3: source route 비교 lane의 회귀 테스트.
//!
//! 여기서 고정하는 것은 **경계**다.
//! - 측정 lane이 꺼져 있으면 어떤 source 산출물도 만들어지지 않는다
//! - 측정 산출물은 booth 레일에 표시되는 canonical preview 경로를 절대 건드리지 않는다
//! - object 하나가 실패해도 이미 저장된 RAW truth는 유지된다
//! - 보정되지 않은 source는 어떤 경로로도 preset-applied 집계에 들어가지 않는다
//! - 세션이 바뀌면 이전 세션의 source 후보와 표본이 따라오지 않는다 (NFR-004)

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use boothy_lib::{
    capture::{
        source_probe::{
            block_order_for, evaluate_source_candidate, SourceAdmissionContext,
            SourceProducerOutcome,
        },
        source_telemetry::{
            append_source_comparison_sample, build_source_comparison_sample,
            parse_source_compare_mode, read_source_comparison_samples,
            run_source_comparison_for_capture_with_mode, source_artifact_path,
            source_comparison_path, SourceCompareMode,
        },
    },
    contracts::dto::{
        SourceCandidateDto, SOURCE_REJECT_CANCELLED, SOURCE_REJECT_UNSUPPORTED_COMBINATION,
        SOURCE_REJECT_WRONG_SESSION, SOURCE_ROUTE_CAMERA_PAIRED_JPEG, SOURCE_ROUTE_EMBEDDED_JPEG,
        SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL,
    },
    display::image_probe::content_hash,
    session::{
        session_manifest::{
            CaptureTimingMetrics, FinalCaptureAsset, PreviewCaptureAsset, RawCaptureAsset,
            SessionCaptureRecord, SESSION_CAPTURE_SCHEMA_VERSION,
        },
        session_paths::SessionPaths,
    },
};

const SESSION_A: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
const SESSION_B: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g2n";
const REQUEST: &str = "capture_req_20260812_001";
const CAPTURE: &str = "capture_20260812_001";
const REQUEST_STARTED: u64 = 10_000;

const ALL_ROUTES: [&str; 3] = [
    SOURCE_ROUTE_EMBEDDED_JPEG,
    SOURCE_ROUTE_CAMERA_PAIRED_JPEG,
    SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL,
];

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-source-{test_name}-{stamp}"))
}

/// 최소 구조의 유효 JPEG. Story 7.2의 probe 규칙을 통과한다.
fn build_jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];

    let mut sof = vec![8u8];
    sof.extend_from_slice(&height.to_be_bytes());
    sof.extend_from_slice(&width.to_be_bytes());
    sof.push(3u8);
    sof.extend_from_slice(&[0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);

    bytes.extend_from_slice(&[0xFF, 0xC0]);
    bytes.extend_from_slice(&((sof.len() + 2) as u16).to_be_bytes());
    bytes.extend_from_slice(&sof);
    bytes.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
    bytes.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    bytes.extend_from_slice(&[0xFF, 0xD9]);

    bytes
}

fn build_cr2(jpeg: &[u8]) -> Vec<u8> {
    const IFD_OFFSET: usize = 16;
    const ENTRY_BYTES: usize = 12;
    let jpeg_offset = IFD_OFFSET + 2 + 3 * ENTRY_BYTES + 4;
    let entries = [
        (0x0103u16, 3u16, 1u32, 6u32),
        (0x0111u16, 4u16, 1u32, jpeg_offset as u32),
        (0x0117u16, 4u16, 1u32, jpeg.len() as u32),
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"II");
    bytes.extend_from_slice(&0x002Au16.to_le_bytes());
    bytes.extend_from_slice(&(IFD_OFFSET as u32).to_le_bytes());
    bytes.extend_from_slice(b"CR\x02\x00");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, value_type, count, value) in entries {
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&value_type.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        if value_type == 3 {
            bytes.extend_from_slice(&(value as u16).to_le_bytes());
            bytes.extend_from_slice(&[0, 0]);
        } else {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(jpeg);
    bytes
}

fn write_paired_arrival_event(root: &PathBuf, paired: &PathBuf, jpeg_len: usize) {
    let events = SessionPaths::new(root, SESSION_A)
        .diagnostics_dir
        .join("camera-helper-events.jsonl");
    fs::create_dir_all(events.parent().expect("events parent")).expect("events dir");
    let event = serde_json::json!({
        "schemaVersion": "canon-helper-source-object-arrived/v1",
        "type": "source-object-arrived",
        "sessionId": SESSION_A,
        "requestId": REQUEST,
        "captureId": CAPTURE,
        "observedAt": "2026-08-12T10:15:31Z",
        "assetPath": paired.to_string_lossy(),
        "byteSize": jpeg_len,
        "objectIndex": 1,
        "groupId": 42,
        "objectRole": "jpeg",
        "usedFallbackCorrelation": false,
        "roleSignal": "sdkformat"
    });
    fs::write(&events, format!("{event}\n")).expect("write helper event");
}

fn capture_record(root: &PathBuf, session_id: &str) -> SessionCaptureRecord {
    let paths = SessionPaths::new(root, session_id);
    SessionCaptureRecord {
        schema_version: SESSION_CAPTURE_SCHEMA_VERSION.into(),
        session_id: session_id.into(),
        booth_alias: "B-001".into(),
        active_preset_id: Some("preset_soft-glow".into()),
        active_preset_version: "2026.08.12".into(),
        active_preset_display_name: Some("Soft Glow".into()),
        capture_id: CAPTURE.into(),
        request_id: REQUEST.into(),
        raw: RawCaptureAsset {
            asset_path: paths
                .captures_originals_dir
                .join(format!("{CAPTURE}.cr2"))
                .to_string_lossy()
                .into_owned(),
            persisted_at_ms: 20_000,
        },
        preview: PreviewCaptureAsset {
            asset_path: None,
            enqueued_at_ms: Some(20_000),
            ready_at_ms: None,
        },
        final_asset: FinalCaptureAsset {
            asset_path: None,
            ready_at_ms: None,
        },
        render_status: "captureSaved".into(),
        post_end_state: "active".into(),
        timing: CaptureTimingMetrics {
            capture_acknowledged_at_ms: 10_000,
            preview_visible_at_ms: None,
            fast_preview_visible_at_ms: None,
            xmp_preview_ready_at_ms: None,
            capture_budget_ms: 1_000,
            preview_budget_ms: 5_000,
            preview_budget_state: "pending".into(),
        },
    }
}

fn candidate(session_id: &str, route: &str, bytes: &[u8]) -> SourceCandidateDto {
    SourceCandidateDto {
        capture_id: Some(CAPTURE.into()),
        request_id: REQUEST.into(),
        session_id: session_id.into(),
        route: route.into(),
        asset_path: Some("C:/tmp/sources/capture-embedded.jpg".into()),
        width_px: Some(5184),
        height_px: Some(3456),
        byte_size: Some(bytes.len() as u64),
        exif_orientation: None,
        decode_valid: true,
        source_hash: Some(content_hash(bytes)),
        object_index: Some(0),
        group_id: Some(42),
        ready_at_host_micros: Some(REQUEST_STARTED + 5_000),
        extraction_cost_micros: Some(12_000),
    }
}

fn context<'a>(session_id: &'a str, routes: &'a [&'a str]) -> SourceAdmissionContext<'a> {
    SourceAdmissionContext {
        bound_session_id: Some(session_id),
        active_request_id: Some(REQUEST),
        expected_capture_id: Some(CAPTURE),
        request_started_at_host_micros: REQUEST_STARTED,
        enabled_routes: routes,
        used_fallback_correlation: false,
    }
}

// ---------------------------------------------------------------------------
// 측정 lane이 꺼져 있을 때 제품 경로는 그대로다
// ---------------------------------------------------------------------------

#[test]
fn compare_lane_is_off_unless_explicitly_enabled() {
    // 실제 프로세스 환경에서도 기본이 off여야 한다.
    assert_eq!(parse_source_compare_mode(None), SourceCompareMode::Off);
    assert!(!SourceCompareMode::Off.is_enabled());
}

#[test]
fn no_source_artifact_is_admitted_while_the_lane_is_off() {
    let bytes = build_jpeg(5184, 3456);
    let off_routes = SourceCompareMode::Off.enabled_routes();

    assert!(off_routes.is_empty(), "off 모드에는 측정 대상 route가 없다");

    // 완전히 정상적인 후보라도 lane이 꺼져 있으면 승격되지 않는다.
    for route in ALL_ROUTES {
        assert_eq!(
            evaluate_source_candidate(
                &candidate(SESSION_A, route, &bytes),
                SourceProducerOutcome::Produced,
                Some(&bytes),
                &context(SESSION_A, off_routes),
            ),
            Err(SOURCE_REJECT_UNSUPPORTED_COMBINATION),
            "route {route} 가 off 모드에서 승격되었다"
        );
    }
}

#[test]
fn lane_off_writes_no_comparison_file() {
    let root = unique_test_root("lane-off");
    let _ = fs::remove_dir_all(&root);
    let capture = capture_record(&root, SESSION_A);

    assert_eq!(
        run_source_comparison_for_capture_with_mode(
            &root,
            &capture,
            Some("windows-shell-thumbnail"),
            SourceCompareMode::Off,
            0,
        )
        .expect("off lane should be a no-op"),
        0
    );

    // lane이 꺼져 있으면 아무도 append를 호출하지 않는다. 파일이 없어야 한다.
    assert!(read_source_comparison_samples(&root, SESSION_A)
        .expect("read samples")
        .is_empty());
    assert!(!source_comparison_path(&root, SESSION_A).exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn embedded_lane_runs_through_the_product_entrypoint_and_preserves_raw_truth() {
    let root = unique_test_root("embedded-product-path");
    let capture = capture_record(&root, SESSION_A);
    let raw_path = PathBuf::from(&capture.raw.asset_path);
    fs::create_dir_all(raw_path.parent().expect("raw parent")).expect("raw dir");
    let raw = build_cr2(&build_jpeg(5184, 3456));
    fs::write(&raw_path, &raw).expect("write raw");

    assert_eq!(
        run_source_comparison_for_capture_with_mode(
            &root,
            &capture,
            None,
            SourceCompareMode::Embedded,
            0,
        )
        .expect("run embedded lane"),
        1
    );

    let samples = read_source_comparison_samples(&root, SESSION_A).expect("read samples");
    assert_eq!(samples.len(), 1);
    assert!(samples[0].accepted);
    assert!(source_artifact_path(&root, SESSION_A, CAPTURE, "embedded").is_file());
    assert_eq!(fs::read(&raw_path).expect("read raw"), raw);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn product_entrypoint_compares_freshness_on_the_host_monotonic_clock() {
    let root = unique_test_root("monotonic-freshness");
    let capture = capture_record(&root, SESSION_A);
    let raw_path = PathBuf::from(&capture.raw.asset_path);
    fs::create_dir_all(raw_path.parent().expect("raw parent")).expect("raw dir");
    fs::write(&raw_path, build_cr2(&build_jpeg(5184, 3456))).expect("write raw");

    run_source_comparison_for_capture_with_mode(
        &root,
        &capture,
        None,
        SourceCompareMode::Embedded,
        u64::MAX,
    )
    .expect("record stale source attempt");

    let sample = read_source_comparison_samples(&root, SESSION_A)
        .expect("read samples")
        .into_iter()
        .next()
        .expect("embedded sample");
    assert!(!sample.accepted);
    assert_eq!(sample.reject_reason.as_deref(), Some("stale"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ab_product_entrypoint_records_exactly_one_row_for_each_route() {
    let root = unique_test_root("ab-product-path");
    let mut capture = capture_record(&root, SESSION_A);
    let jpeg = build_jpeg(5184, 3456);
    let raw_path = PathBuf::from(&capture.raw.asset_path);
    fs::create_dir_all(raw_path.parent().expect("raw parent")).expect("raw dir");
    fs::write(&raw_path, build_cr2(&jpeg)).expect("write raw");

    let paired = source_artifact_path(&root, SESSION_A, CAPTURE, "paired");
    fs::create_dir_all(paired.parent().expect("source parent")).expect("source dir");
    fs::write(&paired, &jpeg).expect("write paired jpeg");
    write_paired_arrival_event(&root, &paired, jpeg.len());
    let shell = SessionPaths::new(&root, SESSION_A)
        .renders_previews_dir
        .join(format!("{CAPTURE}.jpg"));
    fs::create_dir_all(shell.parent().expect("preview parent")).expect("preview dir");
    fs::write(&shell, &jpeg).expect("write shell jpeg");
    capture.preview.asset_path = Some(shell.to_string_lossy().into_owned());

    assert_eq!(
        run_source_comparison_for_capture_with_mode(
            &root,
            &capture,
            Some("windows-shell-thumbnail"),
            SourceCompareMode::Ab,
            0,
        )
        .expect("run AB lane"),
        3
    );

    let samples = read_source_comparison_samples(&root, SESSION_A).expect("read samples");
    assert_eq!(samples.len(), 3);
    assert!(samples.iter().all(|sample| sample.accepted));
    assert!(samples.iter().all(|sample| !sample.is_preset_applied));
    assert!(samples.iter().any(|sample| {
        sample.candidate.route == SOURCE_ROUTE_CAMERA_PAIRED_JPEG
            && !sample.used_fallback_correlation
            && sample.candidate.group_id == Some(42)
    }));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn paired_route_uses_helper_group_correlation_when_the_event_is_available() {
    let root = unique_test_root("paired-correlation-event");
    let capture = capture_record(&root, SESSION_A);
    let jpeg = build_jpeg(5184, 3456);
    let paired = source_artifact_path(&root, SESSION_A, CAPTURE, "paired");
    fs::create_dir_all(paired.parent().expect("source parent")).expect("source dir");
    fs::write(&paired, &jpeg).expect("write paired jpeg");

    write_paired_arrival_event(&root, &paired, jpeg.len());

    run_source_comparison_for_capture_with_mode(
        &root,
        &capture,
        None,
        SourceCompareMode::Paired,
        0,
    )
    .expect("run paired lane");

    let sample = read_source_comparison_samples(&root, SESSION_A)
        .expect("read samples")
        .into_iter()
        .next()
        .expect("paired sample");
    assert_eq!(sample.candidate.object_index, Some(1));
    assert_eq!(sample.candidate.group_id, Some(42));
    assert!(!sample.used_fallback_correlation);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn paired_route_rejects_an_artifact_without_a_matching_arrival_record() {
    let root = unique_test_root("paired-missing-correlation");
    let capture = capture_record(&root, SESSION_A);
    let jpeg = build_jpeg(5184, 3456);
    let paired = source_artifact_path(&root, SESSION_A, CAPTURE, "paired");
    fs::create_dir_all(paired.parent().expect("source parent")).expect("source dir");
    fs::write(&paired, jpeg).expect("write paired jpeg");

    run_source_comparison_for_capture_with_mode(
        &root,
        &capture,
        None,
        SourceCompareMode::Paired,
        0,
    )
    .expect("record rejected paired lane");

    let sample = read_source_comparison_samples(&root, SESSION_A)
        .expect("read samples")
        .into_iter()
        .next()
        .expect("paired sample");
    assert!(!sample.accepted);
    assert_eq!(sample.reject_reason.as_deref(), Some("wrong-capture"));
    assert_eq!(sample.candidate.object_index, None);
    assert!(!sample.used_fallback_correlation);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn incomplete_previous_block_stops_the_next_measurement() {
    let root = unique_test_root("incomplete-block");
    let capture = capture_record(&root, SESSION_A);
    let raw_path = PathBuf::from(&capture.raw.asset_path);
    fs::create_dir_all(raw_path.parent().expect("raw parent")).expect("raw dir");
    fs::write(&raw_path, build_cr2(&build_jpeg(5184, 3456))).expect("write raw");

    run_source_comparison_for_capture_with_mode(
        &root,
        &capture,
        None,
        SourceCompareMode::Embedded,
        0,
    )
    .expect("write one-route block");

    let error = run_source_comparison_for_capture_with_mode(
        &root,
        &capture,
        None,
        SourceCompareMode::Ab,
        0,
    )
    .expect_err("mixed or partial AB evidence must stop");
    assert!(error.message.contains("불완전"));
    assert_eq!(
        read_source_comparison_samples(&root, SESSION_A)
            .expect("read samples")
            .len(),
        1
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn malformed_jsonl_is_reported_instead_of_shrinking_the_denominator() {
    let root = unique_test_root("malformed-jsonl");
    let path = source_comparison_path(&root, SESSION_A);
    fs::create_dir_all(path.parent().expect("diagnostics parent")).expect("diagnostics dir");
    fs::write(&path, "{\"truncated\":\n").expect("write malformed row");

    assert!(read_source_comparison_samples(&root, SESSION_A).is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn product_entrypoint_never_reuses_another_sessions_source_artifact() {
    let root = unique_test_root("session-product-isolation");
    let jpeg = build_jpeg(5184, 3456);
    let session_a_artifact = source_artifact_path(&root, SESSION_A, CAPTURE, "paired");
    fs::create_dir_all(session_a_artifact.parent().expect("source parent")).expect("source dir");
    fs::write(&session_a_artifact, jpeg).expect("write session A source");

    let capture_b = capture_record(&root, SESSION_B);
    run_source_comparison_for_capture_with_mode(
        &root,
        &capture_b,
        None,
        SourceCompareMode::Paired,
        0,
    )
    .expect("run session B lane");

    let samples = read_source_comparison_samples(&root, SESSION_B).expect("read session B");
    assert_eq!(samples.len(), 1);
    assert!(!samples[0].accepted);
    assert_eq!(samples[0].reject_reason.as_deref(), Some("absent"));
    assert!(session_a_artifact.is_file());

    let _ = fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 측정 산출물이 제품 UI 경로를 건드리지 않는다
// ---------------------------------------------------------------------------

#[test]
fn source_artifacts_stay_out_of_the_canonical_preview_path() {
    let root = unique_test_root("artifact-path");
    let paths = SessionPaths::new(&root, SESSION_A);
    let artifact = source_artifact_path(&root, SESSION_A, CAPTURE, "embedded");

    // `renders/previews/`는 이미 booth 사진 레일에 표시된다. 측정 산출물이 거기 들어가면
    // lane이 제품 UI를 조용히 바꾼다.
    assert!(!artifact.starts_with(&paths.renders_previews_dir));
    assert!(artifact.starts_with(&paths.session_root));
    assert!(artifact.to_string_lossy().contains("sources"));
}

#[test]
fn comparison_telemetry_is_a_separate_file_from_present_telemetry() {
    let root = unique_test_root("separate-telemetry");
    let comparison = source_comparison_path(&root, SESSION_A);
    let present = SessionPaths::new(&root, SESSION_A)
        .diagnostics_dir
        .join("viewer-present.jsonl");

    // 두 파일을 합쳐 집계하는 순간 무보정 JPEG의 빠른 도착 시각이 preset-applied KPI를
    // 실제보다 좋아 보이게 만든다.
    assert_ne!(comparison, present);
}

// ---------------------------------------------------------------------------
// AC 4: RAW truth는 어떤 실험에도 종속되지 않는다
// ---------------------------------------------------------------------------

#[test]
fn a_failed_source_object_does_not_invalidate_persisted_raw_truth() {
    let root = unique_test_root("raw-truth");
    let paths = SessionPaths::new(&root, SESSION_A);
    fs::create_dir_all(&paths.captures_originals_dir).expect("originals dir");

    // RAW가 이미 저장되어 있다. 촬영은 이 시점에 성공이다.
    let raw_path = paths.captures_originals_dir.join(format!("{CAPTURE}.cr2"));
    fs::write(&raw_path, b"raw-bytes").expect("write raw");

    let bytes = build_jpeg(5184, 3456);
    let cancelled = evaluate_source_candidate(
        &candidate(SESSION_A, SOURCE_ROUTE_CAMERA_PAIRED_JPEG, &bytes),
        SourceProducerOutcome::Cancelled,
        None,
        &context(SESSION_A, &ALL_ROUTES),
    );

    assert_eq!(cancelled, Err(SOURCE_REJECT_CANCELLED));

    // 거부 판정이 RAW를 지우거나 바꾸지 않았다.
    assert!(raw_path.exists(), "RAW truth가 사라졌다");
    assert_eq!(fs::read(&raw_path).expect("read raw"), b"raw-bytes");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn every_attempt_leaves_exactly_one_row_including_failures() {
    let root = unique_test_root("one-row-per-attempt");
    let bytes = build_jpeg(5184, 3456);

    // 성공 1건, 거부 2건 — 세 시도 모두 행이 남아야 한다.
    // 행이 없는 시도가 생기면 분모가 조용히 줄어 성공률이 실제보다 좋아 보인다.
    let attempts = [
        (true, None),
        (false, Some(SOURCE_REJECT_CANCELLED)),
        (false, Some(SOURCE_REJECT_UNSUPPORTED_COMBINATION)),
    ];

    for (index, (accepted, reject_reason)) in attempts.iter().enumerate() {
        let sample = build_source_comparison_sample(
            candidate(SESSION_A, SOURCE_ROUTE_EMBEDDED_JPEG, &bytes),
            *accepted,
            *reject_reason,
            Some("jpeg"),
            false,
            block_order_for(20260812, index as u32),
            index as u32,
            false,
            20260812,
            1_000 + index as u64,
        )
        .expect("valid sample");

        append_source_comparison_sample(&root, SESSION_A, &sample).expect("append sample");
    }

    let samples = read_source_comparison_samples(&root, SESSION_A).expect("read samples");

    assert_eq!(
        samples.len(),
        attempts.len(),
        "시도마다 행이 하나씩 남아야 한다"
    );
    assert_eq!(samples.iter().filter(|sample| sample.accepted).count(), 1);
    assert_eq!(samples.iter().filter(|sample| !sample.accepted).count(), 2);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_recorded_sample_can_claim_a_preset_was_applied() {
    let root = unique_test_root("never-preset-applied");
    let bytes = build_jpeg(5184, 3456);

    for route in ALL_ROUTES {
        let sample = build_source_comparison_sample(
            candidate(SESSION_A, route, &bytes),
            true,
            None,
            Some("jpeg"),
            false,
            "AB",
            0,
            false,
            1,
            2_000,
        )
        .expect("valid sample");

        append_source_comparison_sample(&root, SESSION_A, &sample).expect("append sample");
    }

    let samples = read_source_comparison_samples(&root, SESSION_A).expect("read samples");

    assert_eq!(samples.len(), ALL_ROUTES.len());
    // FR-010의 qualifying frame은 "같은 capture의 preset이 적용된 이미지"다.
    // 이 lane의 어떤 산출물도 거기에 해당하지 않는다.
    assert!(samples.iter().all(|sample| !sample.is_preset_applied));

    // 직렬화된 원문에도 true가 나타나지 않는다.
    let raw = fs::read_to_string(source_comparison_path(&root, SESSION_A)).expect("read jsonl");
    assert!(!raw.contains("\"isPresetApplied\":true"));
    assert_eq!(
        raw.matches("\"isPresetApplied\":false").count(),
        ALL_ROUTES.len()
    );

    let _ = fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// NFR-004: 세션 격리는 0 tolerance다
// ---------------------------------------------------------------------------

#[test]
fn another_sessions_candidate_is_rejected_before_any_other_check() {
    let bytes = build_jpeg(5184, 3456);

    assert_eq!(
        evaluate_source_candidate(
            &candidate(SESSION_B, SOURCE_ROUTE_EMBEDDED_JPEG, &bytes),
            SourceProducerOutcome::Produced,
            Some(&bytes),
            &context(SESSION_A, &ALL_ROUTES),
        ),
        Err(SOURCE_REJECT_WRONG_SESSION)
    );
}

#[test]
fn session_change_leaves_no_source_artifacts_or_samples_behind() {
    let root = unique_test_root("session-isolation");
    let bytes = build_jpeg(5184, 3456);

    let sample = build_source_comparison_sample(
        candidate(SESSION_A, SOURCE_ROUTE_EMBEDDED_JPEG, &bytes),
        true,
        None,
        Some("jpeg"),
        false,
        "AB",
        0,
        false,
        1,
        3_000,
    )
    .expect("valid sample");
    append_source_comparison_sample(&root, SESSION_A, &sample).expect("append sample");

    let artifact = source_artifact_path(&root, SESSION_A, CAPTURE, "embedded");
    fs::create_dir_all(artifact.parent().expect("artifact parent")).expect("artifact dir");
    fs::write(&artifact, &bytes).expect("write artifact");

    // 새 세션은 이전 세션의 표본도, 산출물도 보지 못한다.
    assert!(read_source_comparison_samples(&root, SESSION_B)
        .expect("read other session samples")
        .is_empty());
    assert!(!source_artifact_path(&root, SESSION_B, CAPTURE, "embedded").exists());

    // 세션 루트를 지우면 그 세션의 source 산출물과 표본이 함께 사라진다.
    fs::remove_dir_all(SessionPaths::new(&root, SESSION_A).session_root).expect("drop session");

    assert!(!artifact.exists());
    assert!(read_source_comparison_samples(&root, SESSION_A)
        .expect("read removed session samples")
        .is_empty());

    let _ = fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// AB/BA 배치는 host가 정하고 기록한다
// ---------------------------------------------------------------------------

#[test]
fn ab_ba_assignment_is_reproducible_from_the_recorded_seed() {
    let seed = 20260812u64;
    let first: Vec<&str> = (0..40).map(|index| block_order_for(seed, index)).collect();
    let replay: Vec<&str> = (0..40).map(|index| block_order_for(seed, index)).collect();

    assert_eq!(first, replay, "같은 seed는 같은 배치를 낳아야 한다");
    assert!(first.iter().any(|order| *order == "AB"));
    assert!(first.iter().any(|order| *order == "BA"));

    // 서로 다른 seed는 서로 다른 배치를 낼 수 있어야 한다.
    let other: Vec<&str> = (0..40).map(|index| block_order_for(7, index)).collect();
    assert_ne!(first, other);
}

#[test]
fn ab_mode_measures_all_three_routes_including_the_incumbent() {
    let routes = SourceCompareMode::Ab.enabled_routes();

    assert_eq!(routes.len(), 3);
    for route in ALL_ROUTES {
        assert!(routes.contains(&route), "{route} 가 AB 모드에서 빠졌다");
    }
}
