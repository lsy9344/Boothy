//! Story 7.2: immutable display generation과 atomic pointer 회귀 테스트.
//!
//! 여기서 고정하는 것은 **순서**와 **거부**다.
//! - pointer는 확정 파일이 존재한 뒤에만 갱신된다
//! - partial / stale epoch / lower seq / older request / 크기 미달 / 다른 세션은 전부 거부된다
//! - 같은 generation 경로를 두 번 쓰면 하드 에러다
//! - 세션 교체, photo rect 확대, capture 삭제는 pointer를 정직하게 비운다

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use boothy_lib::{
    contracts::dto::{
        DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS, DISPLAY_REJECT_LOWER_GENERATION,
        DISPLAY_REJECT_OLDER_CAPTURE, DISPLAY_REJECT_OLDER_REQUEST, DISPLAY_REJECT_PARTIAL_FILE,
        DISPLAY_REJECT_PRESET_MISMATCH, DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH,
        DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED, DISPLAY_REJECT_SESSION_MISMATCH,
        DISPLAY_REJECT_STALE_EPOCH, DISPLAY_REJECT_TIER_DOWNGRADE, DISPLAY_REJECT_UNDECODABLE,
        DISPLAY_REJECT_UNKNOWN_GENERATION, DISPLAY_REJECT_VIEWER_NOT_READY,
        DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY, DISPLAY_TIER_RAW_REFINED_DISPLAY,
        DISPLAY_TIER_SAMPLE, VIEWER_DISPLAY_SCHEMA_VERSION,
    },
    display::{
        display_artifact::DisplayState,
        generation_publisher::{publish_generation_in_dir, PublishOutcome, PublishRequest},
        generation_repository::{
            display_root, generation_path, read_journal, read_pointer, remove_request_generations,
            write_reconciled_pointer,
        },
        image_probe::probe_jpeg,
        raw_refined_publisher::{
            evaluate_raw_refined_eligibility, parse_raw_refined_lane_mode, RawRefinedSkipReason,
            RAW_REFINED_TIER_JUSTIFICATION,
        },
        DisplayLaneFlags,
    },
    viewer::present_telemetry::{
        append_present_sample, conservative_latency_micros, read_present_samples,
        PresentSampleRecord, PRESENT_TELEMETRY_SCHEMA_VERSION,
    },
};

const SESSION_A: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
const SESSION_B: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g2n";

/// 1080p 승인 profile의 실제 요구치 근사값. Story 7.1의 photoRect에서 온다.
const REQUIRED_WIDTH_1080P: u32 = 1620;
const REQUIRED_HEIGHT_1080P: u32 = 1080;

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-display-{test_name}-{stamp}"))
}

/// 실제 번들 fixture를 읽는다. 합성 바이트가 아니라 배포되는 자산으로 검증한다.
fn sample_fixture_bytes(variant: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("storage")
        .join("fixtures")
        .join("display-sample")
        .join(format!("sample-{variant}.jpg"));

    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "sample fixture must exist at {}: {error}",
            path.to_string_lossy()
        )
    })
}

struct StepClock {
    ticks: std::cell::Cell<u64>,
}

impl StepClock {
    fn new() -> Self {
        Self {
            ticks: std::cell::Cell::new(1_000),
        }
    }

    fn now(&self) -> u64 {
        let next = self.ticks.get() + 10;
        self.ticks.set(next);
        next
    }
}

fn publish(
    base_dir: &PathBuf,
    state: &mut DisplayState,
    clock: &StepClock,
    session_id: &str,
    bound_session_id: Option<&str>,
    request_id: &str,
    variant: &str,
    viewer_epoch: u64,
    required: (u32, u32),
    bytes: &[u8],
) -> PublishOutcome {
    let request = PublishRequest {
        session_id,
        request_id,
        capture_id: Some("capture-1"),
        tier: DISPLAY_TIER_SAMPLE,
        sample_variant: Some(variant),
        proxy_provenance: None,
        source_bytes: bytes,
        bound_session_id,
        request_viewer_epoch: viewer_epoch,
        current_viewer_epoch: viewer_epoch,
        capture_order: None,
        required_source_width_px: required.0,
        required_source_height_px: required.1,
        lanes: DisplayLaneFlags::none(),
        refined_tier_justified: false,
    };

    publish_generation_in_dir(base_dir, state, &request, &|| clock.now())
        .expect("publish should not fail with an IO error")
}

fn bound_state(session_id: &str) -> DisplayState {
    let mut state = DisplayState::default();
    state.bind_session(session_id);
    state
}

#[test]
fn bundled_sample_fixtures_satisfy_the_display_fit_contract() {
    for variant in ["a", "b"] {
        let bytes = sample_fixture_bytes(variant);
        let probe = probe_jpeg(&bytes).expect("bundled fixture must pass the structural probe");

        // 승인 4K profile의 requiredSource(3240x2160)를 상회해야 한다.
        assert!(
            probe.width_px >= 3240 && probe.height_px >= 2160,
            "fixture {variant} is {}x{}, too small for the 4K display-fit contract",
            probe.width_px,
            probe.height_px
        );
        // 3:2를 유지해야 crop/scale jump 판정이 성립한다.
        assert_eq!(probe.width_px * 2, probe.height_px * 3);
        // EXIF orientation 회전은 허용하지 않는다.
        assert!(probe.orientation.is_none() || probe.orientation == Some(1));
    }

    let first = probe_jpeg(&sample_fixture_bytes("a")).expect("probe");
    let second = probe_jpeg(&sample_fixture_bytes("b")).expect("probe");

    // 두 fixture의 픽셀 크기가 다르면 교체 자체가 scale jump가 된다.
    assert_eq!(
        (first.width_px, first.height_px),
        (second.width_px, second.height_px)
    );
}

#[test]
fn commits_two_ordered_generations_and_advances_the_pointer() {
    let base_dir = unique_test_root("ordered-generations");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    let first = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &sample_fixture_bytes("a"),
    );

    let first_generation = first
        .committed
        .as_ref()
        .expect("first generation should commit");
    assert!(first.reject_reason.is_none());
    assert_eq!(first.pointer.schema_version, VIEWER_DISPLAY_SCHEMA_VERSION);
    assert!(
        first.file_ready_at_micros.expect("file-ready span")
            < first.probe_ok_at_micros.expect("probe span")
    );
    assert!(
        first.probe_ok_at_micros.expect("probe span")
            < first
                .pointer_committed_at_micros
                .expect("pointer commit span")
    );

    // 확정 파일이 존재한 뒤에야 pointer가 그 generation을 가리킨다.
    assert!(PathBuf::from(&first_generation.asset_path).is_file());
    assert_eq!(
        first
            .pointer
            .active_generation
            .as_ref()
            .map(|g| &g.generation_id),
        Some(&first_generation.generation_id)
    );

    let second = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "b",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &sample_fixture_bytes("b"),
    );
    let second_generation = second
        .committed
        .as_ref()
        .expect("second generation should commit");

    assert!(second_generation.generation_seq > first_generation.generation_seq);
    assert!(second.pointer.revision > first.pointer.revision);

    // immutable: 이전 generation 파일이 그대로 남아 있어야 한다.
    assert!(PathBuf::from(&first_generation.asset_path).is_file());
    assert_ne!(first_generation.asset_path, second_generation.asset_path);

    // durable pointer가 디스크에도 반영된다.
    let persisted = read_pointer(&base_dir, SESSION_A).expect("pointer file should exist");
    assert_eq!(
        persisted
            .active_generation
            .as_ref()
            .map(|g| g.generation_id.clone()),
        Some(second_generation.generation_id.clone())
    );

    let journal = read_journal(&base_dir, SESSION_A);
    assert_eq!(journal.len(), 2);
    assert!(journal.iter().all(|record| record.outcome == "committed"));

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_partial_file_without_touching_the_pointer() {
    let base_dir = unique_test_root("partial-file");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    let full = sample_fixture_bytes("a");
    let truncated = &full[..full.len() / 2];

    let outcome = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        truncated,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PARTIAL_FILE)
    );
    assert!(outcome.committed.is_none());
    assert!(outcome.pointer.active_generation.is_none());
    assert!(read_pointer(&base_dir, SESSION_A).is_none());

    // staging 잔여물이 남으면 안 된다.
    let staging_dir = display_root(&base_dir, SESSION_A).join(".staging");
    let leftovers = fs::read_dir(&staging_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(leftovers, 0);

    let journal = read_journal(&base_dir, SESSION_A);
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].outcome, "rejected");
    assert_eq!(
        journal[0].reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PARTIAL_FILE)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_non_image_bytes_as_undecodable() {
    let base_dir = unique_test_root("undecodable");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    let outcome = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        b"not a jpeg at all",
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_UNDECODABLE)
    );
    assert!(outcome.pointer.active_generation.is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_a_sample_whose_viewer_generation_died_mid_request() {
    let base_dir = unique_test_root("stale-epoch");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let bytes = sample_fixture_bytes("a");

    // sample A는 request가 수락된 epoch 3에서 정상 게시된다.
    let first = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &bytes,
    );
    assert!(first.committed.is_some());

    // A와 B 사이에 관람 창이 재생성되어 현재 epoch가 4가 됐다.
    // sample B는 여전히 request 세대 3에 속하므로 죽은 viewer 세대의 자산이다.
    let stale = publish_generation_in_dir(
        &base_dir,
        &mut state,
        &PublishRequest {
            session_id: SESSION_A,
            request_id: "req-1",
            capture_id: None,
            tier: DISPLAY_TIER_SAMPLE,
            sample_variant: Some("b"),
            proxy_provenance: None,
            source_bytes: &sample_fixture_bytes("b"),
            bound_session_id: Some(SESSION_A),
            request_viewer_epoch: 3,
            current_viewer_epoch: 4,
            capture_order: None,
            required_source_width_px: REQUIRED_WIDTH_1080P,
            required_source_height_px: REQUIRED_HEIGHT_1080P,
            lanes: DisplayLaneFlags::none(),
            refined_tier_justified: false,
        },
        &|| clock.now(),
    )
    .expect("publish should not fail with an IO error");

    assert_eq!(
        stale.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_STALE_EPOCH)
    );
    // 현재 표시는 그대로 유지된다.
    assert_eq!(
        stale
            .pointer
            .active_generation
            .as_ref()
            .and_then(|g| g.sample_variant.clone()),
        Some("a".to_string())
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_insufficient_dimensions_against_the_current_photo_rect() {
    let base_dir = unique_test_root("insufficient-dimensions");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    // fixture는 3840x2560이므로 그보다 큰 요구치를 주면 거부되어야 한다.
    let outcome = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (5000, 3400),
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_INSUFFICIENT_DIMENSIONS)
    );
    assert!(outcome.pointer.active_generation.is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_publication_before_the_viewer_reports_a_photo_rect() {
    let base_dir = unique_test_root("viewer-not-ready");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    let outcome = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (0, 0),
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_VIEWER_NOT_READY)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_other_session_without_writing_any_file() {
    let base_dir = unique_test_root("session-mismatch");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    let outcome = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_B,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_SESSION_MISMATCH)
    );
    // 다른 세션의 디스크 경로를 건드리지 않는다.
    assert!(!display_root(&base_dir, SESSION_B).join("req-1").exists());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn rejects_older_request_arriving_late_with_a_higher_seq() {
    let base_dir = unique_test_root("older-request");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let bytes = sample_fixture_bytes("a");
    let required = (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P);

    // req-1을 먼저 관측해 순서를 부여한다.
    publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        required,
        &bytes,
    );
    // req-2가 활성이 된다.
    let newer = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-2",
        "a",
        3,
        required,
        &bytes,
    );
    assert!(newer.committed.is_some());

    // 이제 req-1의 두 번째 표본이 더 높은 seq로 늦게 도착한다.
    let late = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "b",
        3,
        required,
        &bytes,
    );

    assert_eq!(
        late.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_OLDER_REQUEST)
    );
    assert_eq!(
        late.pointer
            .active_generation
            .as_ref()
            .map(|g| g.request_id.clone()),
        Some("req-2".to_string())
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn duplicate_generation_path_is_a_hard_error() {
    let base_dir = unique_test_root("immutable-path");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let bytes = sample_fixture_bytes("a");

    let first = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &bytes,
    );
    let committed = first.committed.expect("first generation should commit");

    // 같은 seq/variant 경로를 미리 점유해 두면 승격이 하드 에러가 되어야 한다.
    let colliding_path = generation_path(
        &base_dir,
        SESSION_A,
        "req-1",
        committed.generation_seq + 1,
        "b",
    )
    .expect("valid generation path");
    fs::create_dir_all(colliding_path.parent().expect("parent")).expect("prepare dir");
    fs::write(&colliding_path, b"existing immutable artifact").expect("prepare collision");

    let request = PublishRequest {
        session_id: SESSION_A,
        request_id: "req-1",
        capture_id: None,
        tier: DISPLAY_TIER_SAMPLE,
        sample_variant: Some("b"),
        proxy_provenance: None,
        source_bytes: &bytes,
        bound_session_id: Some(SESSION_A),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        capture_order: None,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
        lanes: DisplayLaneFlags::none(),
        refined_tier_justified: false,
    };
    let error = publish_generation_in_dir(&base_dir, &mut state, &request, &|| clock.now())
        .expect_err("overwriting an immutable generation path must be a hard error");

    assert!(error.message.contains("덮어쓸 수 없어요"));
    // 기존 파일은 그대로 남는다.
    assert_eq!(
        fs::read(&colliding_path).expect("existing artifact"),
        b"existing immutable artifact"
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn request_id_cannot_escape_the_display_root() {
    let base_dir = unique_test_root("request-path-containment");
    let clock = StepClock::new();
    let protected_dir = display_root(&base_dir, SESSION_A)
        .parent()
        .expect("renders dir")
        .join("protected");
    fs::create_dir_all(&protected_dir).expect("prepare protected dir");
    let sentinel = protected_dir.join("sentinel.txt");
    fs::write(&sentinel, b"keep").expect("prepare sentinel");

    let error = remove_request_generations(&base_dir, SESSION_A, "../protected", clock.now())
        .expect_err("parent traversal must be rejected");

    assert_eq!(error.code, "validation-error");
    assert!(sentinel.is_file());
    assert!(generation_path(&base_dir, SESSION_A, "..\\protected", 1, "a").is_err());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn staging_io_failure_is_not_reported_as_bad_image_bytes() {
    let base_dir = unique_test_root("staging-io-failure");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let root = display_root(&base_dir, SESSION_A);
    fs::create_dir_all(&root).expect("prepare display root");
    fs::write(root.join(".staging"), b"blocks directory creation")
        .expect("prepare staging blocker");
    let bytes = sample_fixture_bytes("a");

    let request = PublishRequest {
        session_id: SESSION_A,
        request_id: "req-1",
        capture_id: None,
        tier: DISPLAY_TIER_SAMPLE,
        sample_variant: Some("a"),
        proxy_provenance: None,
        source_bytes: &bytes,
        bound_session_id: Some(SESSION_A),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        capture_order: None,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
        lanes: DisplayLaneFlags::none(),
        refined_tier_justified: false,
    };
    let error = publish_generation_in_dir(&base_dir, &mut state, &request, &|| clock.now())
        .expect_err("filesystem failure must be a hard persistence error");

    assert_eq!(error.code, "session-persistence-failed");
    assert!(read_journal(&base_dir, SESSION_A).is_empty());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn journal_failure_rolls_back_the_durable_pointer() {
    let base_dir = unique_test_root("journal-failure");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let root = display_root(&base_dir, SESSION_A);
    fs::create_dir_all(root.join("generations.jsonl"))
        .expect("block journal file creation with a directory");
    let bytes = sample_fixture_bytes("a");

    let request = PublishRequest {
        session_id: SESSION_A,
        request_id: "req-1",
        capture_id: None,
        tier: DISPLAY_TIER_SAMPLE,
        sample_variant: Some("a"),
        proxy_provenance: None,
        source_bytes: &bytes,
        bound_session_id: Some(SESSION_A),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        capture_order: None,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
        lanes: DisplayLaneFlags::none(),
        refined_tier_justified: false,
    };
    let error = publish_generation_in_dir(&base_dir, &mut state, &request, &|| clock.now())
        .expect_err("journal failure must prevent a successful publication");

    assert_eq!(error.code, "session-persistence-failed");
    assert!(state.active_generation().is_none());
    assert!(read_pointer(&base_dir, SESSION_A)
        .expect("rollback pointer should be durable")
        .active_generation
        .is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn session_change_clears_the_pointer() {
    let base_dir = unique_test_root("session-change");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let required = (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P);

    publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        required,
        &sample_fixture_bytes("a"),
    );
    assert!(state.active_generation().is_some());

    // 다음 고객 세션이 시작되면 이전 세션의 표시 자산이 한 프레임도 남으면 안 된다.
    let previous_session_id = state.session_id().map(str::to_string);
    let changed = state.reconcile(Some(SESSION_B), required.0, required.1);
    let snapshot = state.snapshot(
        required.0,
        required.1,
        DisplayLaneFlags::none(),
        clock.now(),
    );
    write_reconciled_pointer(&base_dir, previous_session_id.as_deref(), &snapshot)
        .expect("session transition pointer should persist");

    assert!(changed);
    assert!(state.active_generation().is_none());
    assert!(read_pointer(&base_dir, SESSION_A)
        .expect("old session pointer should be cleared")
        .active_generation
        .is_none());
    assert!(read_pointer(&base_dir, SESSION_B)
        .expect("new session pointer should exist")
        .active_generation
        .is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn growing_required_size_invalidates_the_active_generation() {
    let base_dir = unique_test_root("required-size-growth");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &sample_fixture_bytes("a"),
    );
    assert!(state.active_generation().is_some());

    // viewer epoch가 바뀌며 fixture보다 큰 photo rect가 보고된 경우.
    let changed = state.reconcile(Some(SESSION_A), 5000, 3400);
    let snapshot = state.snapshot(5000, 3400, DisplayLaneFlags::none(), clock.now());
    write_reconciled_pointer(&base_dir, Some(SESSION_A), &snapshot)
        .expect("invalidated pointer should persist");

    assert!(changed);
    assert!(state.active_generation().is_none());
    assert!(read_pointer(&base_dir, SESSION_A)
        .expect("cleared pointer should exist")
        .active_generation
        .is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn deleting_a_capture_removes_its_generations_and_clears_the_pointer() {
    let base_dir = unique_test_root("delete-capture");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);

    let outcome = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P),
        &sample_fixture_bytes("a"),
    );
    let asset_path = PathBuf::from(&outcome.committed.expect("commit").asset_path);
    assert!(asset_path.is_file());

    assert!(state.forget_request("req-1"));
    assert!(state.active_generation().is_none());
    let snapshot = state.snapshot(
        REQUIRED_WIDTH_1080P,
        REQUIRED_HEIGHT_1080P,
        DisplayLaneFlags::none(),
        clock.now(),
    );
    write_reconciled_pointer(&base_dir, Some(SESSION_A), &snapshot)
        .expect("forgotten active pointer should persist");

    remove_request_generations(&base_dir, SESSION_A, "req-1", clock.now())
        .expect("cleanup should succeed");
    assert!(!asset_path.exists());
    assert!(read_pointer(&base_dir, SESSION_A)
        .expect("cleared pointer should exist")
        .active_generation
        .is_none());
    let journal = read_journal(&base_dir, SESSION_A);
    assert_eq!(
        journal.last().map(|record| record.outcome.as_str()),
        Some("request-forgotten")
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn lower_generation_seq_cannot_replace_the_current_display() {
    let base_dir = unique_test_root("lower-generation");
    let clock = StepClock::new();
    let mut state = bound_state(SESSION_A);
    let bytes = sample_fixture_bytes("a");
    let required = (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P);

    publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        required,
        &bytes,
    );
    let current = state
        .active_generation()
        .expect("first generation should be active")
        .clone();

    // 같은 request에서 이미 활성인 seq와 같은 값을 다시 승격하려는 상황을 직접 판정한다.
    let candidate = boothy_lib::display::display_artifact::GenerationCandidate {
        session_id: SESSION_A,
        request_id: "req-1",
        capture_id: Some("capture-1"),
        viewer_epoch: 3,
        tier: DISPLAY_TIER_SAMPLE,
        generation_seq: current.generation_seq,
        source_width_px: 3840,
        source_height_px: 2560,
        request_order: state.request_order("req-1").expect("request order"),
        capture_order: None,
        preset_binding: None,
        refined_tier_justified: false,
    };
    let context = boothy_lib::display::display_artifact::AdmissionContext {
        bound_session_id: Some(SESSION_A),
        viewer_epoch: 3,
        required_source_width_px: required.0,
        required_source_height_px: required.1,
    };

    assert_eq!(
        boothy_lib::display::display_artifact::evaluate_admission(
            state.active_display(),
            &candidate,
            &context
        ),
        Err(DISPLAY_REJECT_LOWER_GENERATION)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn unregistered_tier_has_a_specific_reject_reason() {
    let context = boothy_lib::display::display_artifact::AdmissionContext {
        bound_session_id: Some(SESSION_A),
        viewer_epoch: 3,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
    };
    let candidate = boothy_lib::display::display_artifact::GenerationCandidate {
        session_id: SESSION_A,
        request_id: "req-1",
        capture_id: None,
        viewer_epoch: 3,
        tier: "future-tier",
        generation_seq: 1,
        source_width_px: 3840,
        source_height_px: 2560,
        request_order: 0,
        capture_order: None,
        preset_binding: None,
        refined_tier_justified: false,
    };

    assert_eq!(
        boothy_lib::display::display_artifact::evaluate_admission(
            boothy_lib::display::display_artifact::ActiveDisplay::default(),
            &candidate,
            &context,
        ),
        Err(DISPLAY_REJECT_UNKNOWN_GENERATION)
    );
}

#[test]
fn stale_epoch_candidate_is_rejected() {
    let context = boothy_lib::display::display_artifact::AdmissionContext {
        bound_session_id: Some(SESSION_A),
        viewer_epoch: 9,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
    };
    let candidate = boothy_lib::display::display_artifact::GenerationCandidate {
        session_id: SESSION_A,
        request_id: "req-1",
        capture_id: None,
        viewer_epoch: 3,
        tier: DISPLAY_TIER_SAMPLE,
        generation_seq: 1,
        source_width_px: 3840,
        source_height_px: 2560,
        request_order: 0,
        capture_order: None,
        preset_binding: None,
        refined_tier_justified: false,
    };

    assert_eq!(
        boothy_lib::display::display_artifact::evaluate_admission(
            boothy_lib::display::display_artifact::ActiveDisplay::default(),
            &candidate,
            &context,
        ),
        Err(DISPLAY_REJECT_STALE_EPOCH)
    );
}

#[test]
fn hidden_prewarm_is_opt_in_and_never_the_default() {
    use boothy_lib::commands::viewer_commands::{
        parse_hidden_prewarm_strategy, should_prewarm_hidden, HiddenPrewarmStrategy,
    };
    use boothy_lib::display::sample_publisher::SampleLaneMode;

    // 기본 lane과 visible-standby는 절대 창을 숨기지 않는다.
    assert!(!should_prewarm_hidden(SampleLaneMode::Off));
    assert!(!should_prewarm_hidden(SampleLaneMode::VisibleStandby));
    assert!(should_prewarm_hidden(SampleLaneMode::HiddenPrewarm));

    // A/B는 최소 두 가지 은닉 방식을 시험한다.
    assert_eq!(
        parse_hidden_prewarm_strategy(None),
        HiddenPrewarmStrategy::Invisible
    );
    assert_eq!(
        parse_hidden_prewarm_strategy(Some("offscreen")),
        HiddenPrewarmStrategy::Offscreen
    );
    assert_eq!(
        parse_hidden_prewarm_strategy(Some("nonsense")),
        HiddenPrewarmStrategy::Invisible
    );
}

#[test]
fn hidden_prewarm_reveals_a_window_generation_exactly_once() {
    use boothy_lib::commands::viewer_commands::should_reveal_for_epoch;

    // 첫 qualifying generation에서 한 번만 드러낸다. generation마다 다시 드러내면
    // 측정 구간 한가운데에서 창 상태가 흔들리고 present 계측이 유실된다.
    assert!(should_reveal_for_epoch(u64::MAX, 3));
    assert!(!should_reveal_for_epoch(3, 3));

    // 창이 재생성되면 다시 숨겨진 상태로 시작하므로 새 세대에서는 다시 드러내야 한다.
    assert!(should_reveal_for_epoch(3, 4));
}

#[test]
fn every_committed_generation_has_a_terminal_present_row_budget() {
    use boothy_lib::commands::display_commands::PRESENT_REPORT_GRACE_MICROS;
    use boothy_lib::viewer::present_telemetry::PRESENT_CONFIDENCE_UNREPORTED;

    // 유예 시간은 viewer의 after-paint fallback(250ms)보다 충분히 커야
    // 정상 보고를 `present-unreported`로 잘못 닫지 않는다.
    assert!(PRESENT_REPORT_GRACE_MICROS >= 1_000_000);
    // 관측하지 못한 표본을 `measured`로 남기면 집계가 실제보다 좋아 보인다.
    assert_eq!(PRESENT_CONFIDENCE_UNREPORTED, "unreported");
}

#[test]
fn story_71_liveness_contract_is_not_weakened_for_the_ab_test() {
    use boothy_lib::viewer::viewer_state::{
        VIEWER_HEARTBEAT_INTERVAL_MS, VIEWER_REPORT_STALE_AFTER_MS,
    };

    // hidden 변형이 stale-report로 촬영을 막는 것은 Story 7.1이 의도한 정직한 차단이다.
    // 임계값을 늘려 회피하면 A/B 결과와 readiness 계약이 동시에 거짓이 된다.
    assert_eq!(VIEWER_REPORT_STALE_AFTER_MS, 5_000);
    assert_eq!(VIEWER_HEARTBEAT_INTERVAL_MS, 1_000);
}

#[test]
fn present_telemetry_is_session_scoped_and_reports_conservative_latency() {
    let base_dir = unique_test_root("present-telemetry");
    let latency = conservative_latency_micros(1_000_000, 3_400_000, 250, 750);

    let record = PresentSampleRecord {
        schema_version: PRESENT_TELEMETRY_SCHEMA_VERSION.into(),
        session_id: SESSION_A.into(),
        request_id: "req-1".into(),
        generation_id: Some("req-1-000001".into()),
        sample_variant: Some("a".into()),
        lane_mode: "visible-standby".into(),
        outcome: "presented".into(),
        reject_reason: None,
        tier: Some(DISPLAY_TIER_SAMPLE.into()),
        preset_id: None,
        preset_version: None,
        source_route: None,
        target_width_px: None,
        target_height_px: None,
        trusted_input_at_micros: Some(1_000_000),
        actual_present_at_micros: Some(3_400_000),
        qualifying_latency_micros: Some(latency),
        total_uncertainty_micros: 1_000,
        confidence: "measured".into(),
        is_trusted_input: true,
        host_accepted_at_micros: Some(1_002_000),
        sample_write_start_at_micros: Some(1_050_000),
        file_ready_at_micros: Some(1_200_000),
        probe_ok_at_micros: Some(1_210_000),
        pointer_committed_at_micros: Some(1_220_000),
        event_emitted_at_micros: Some(1_225_000),
        viewer_receipt_at_micros: Some(1_240_000),
        decode_start_at_micros: Some(1_250_000),
        decode_end_at_micros: Some(3_300_000),
        swap_committed_at_micros: Some(3_350_000),
        img_on_load_at_micros: Some(3_290_000),
        element_timing_render_at_micros: None,
        is_element_render_time: false,
        proxy_source_ready_at_micros: None,
        proxy_queue_wait_micros: None,
        proxy_render_start_at_micros: None,
        proxy_process_exited_at_micros: None,
        render_quality: None,
        raw_refined_enqueued_at_micros: None,
        raw_refined_queue_wait_micros: None,
        raw_refined_render_start_at_micros: None,
        raw_refined_process_exited_at_micros: None,
        raw_refined_committed_at_micros: None,
        raw_refined_presented_at_micros: None,
        scheduler_priority: None,
        scheduler_deadline_micros: None,
        scheduler_deadline_missed: None,
        scheduler_coalesced_count: None,
        scheduler_preempted_by: None,
        viewer_window_events_after_input: 0,
    };

    append_present_sample(&base_dir, SESSION_A, &record).expect("telemetry append should succeed");

    let samples = read_present_samples(&base_dir, SESSION_A);
    assert_eq!(samples.len(), 1);
    // 보고 값은 실제 구간(2_400_000)보다 짧을 수 없다.
    assert!(samples[0].qualifying_latency_micros.expect("latency") >= 2_400_000);
    // 다른 세션 디렉터리에는 아무것도 남지 않는다.
    assert!(read_present_samples(&base_dir, SESSION_B).is_empty());

    let _ = fs::remove_dir_all(&base_dir);
}

// ---------------------------------------------------------------------------
// Story 7.4: display-fit preset proxy tier.
//
// 여기서 고정하는 것은 **tier 순서**, **촬영 시점 좌표**, **preset 정체성**이다.
// 세 가지 모두 proxy 렌더가 수 초 걸린다는 사실에서 나온 요구다.
// ---------------------------------------------------------------------------

fn proxy_provenance(
    preset_id: &str,
    preset_version: &str,
) -> boothy_lib::contracts::dto::DisplayProxyProvenanceDto {
    boothy_lib::contracts::dto::DisplayProxyProvenanceDto {
        preset_id: preset_id.into(),
        preset_version: preset_version.into(),
        approval_basis: "exact-reference-renderer".into(),
        proxy_recipe_version: "1".into(),
        reference_renderer: "darktable".into(),
        reference_renderer_version: "5.4.1".into(),
        render_profile_id: "preset_soft-glow-preview".into(),
        output_color_space: "sRGB".into(),
        jpeg_quality: 92,
        source_route: "raw-original".into(),
        source_asset_hash: "fnv1a64:00000000000000aa".into(),
        target_width_px: REQUIRED_WIDTH_1080P,
        target_height_px: REQUIRED_HEIGHT_1080P,
        display_profile_id: "approved-1080p".into(),
        device_pixel_ratio: 1.0,
        resident_provenance: None,
        render_quality: boothy_lib::contracts::dto::DISPLAY_RENDER_QUALITY_FAST.into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_proxy(
    base_dir: &PathBuf,
    state: &mut DisplayState,
    clock: &StepClock,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    capture_order: u64,
    provenance: &boothy_lib::contracts::dto::DisplayProxyProvenanceDto,
    bytes: &[u8],
) -> PublishOutcome {
    let request = PublishRequest {
        session_id,
        request_id,
        capture_id: Some(capture_id),
        tier: DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
        sample_variant: None,
        proxy_provenance: Some(provenance),
        source_bytes: bytes,
        bound_session_id: Some(session_id),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        capture_order: Some(capture_order),
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
        lanes: DisplayLaneFlags {
            measurement_lane_enabled: false,
            present_telemetry_enabled: true,
        },
        refined_tier_justified: true,
    };

    publish_generation_in_dir(base_dir, state, &request, &|| clock.now())
        .expect("publish should not fail with an IO error")
}

#[test]
fn a_preset_proxy_replaces_a_measurement_sample_but_never_the_reverse() {
    let base_dir = unique_test_root("proxy-tier-order");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let required = (REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P);
    let provenance = proxy_provenance("preset_soft-glow", "2026.08.01");

    publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "a",
        3,
        required,
        &sample_fixture_bytes("a"),
    );

    let proxy = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &provenance,
        &sample_fixture_bytes("b"),
    );

    assert_eq!(proxy.reject_reason, None);
    let committed = proxy.committed.expect("proxy should be committed");
    assert_eq!(committed.tier, DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY);
    assert_eq!(committed.sample_variant, None);
    assert_eq!(
        committed
            .proxy_provenance
            .as_ref()
            .map(|value| value.preset_id.as_str()),
        Some("preset_soft-glow")
    );

    // 확정 경로가 사람과 스크립트 모두 읽을 수 있어야 한다.
    assert!(
        committed
            .asset_path
            .replace('\\', "/")
            .ends_with("-proxy.jpg"),
        "proxy asset path must be readable, got {}",
        committed.asset_path
    );

    // fixture가 실제 결과를 덮으면 고객은 자기 사진 대신 계측 이미지를 본다.
    let downgrade = publish(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        Some(SESSION_A),
        "req-1",
        "b",
        3,
        required,
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        downgrade.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_TIER_DOWNGRADE)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_late_finishing_earlier_capture_cannot_replace_the_current_photo() {
    // **이 Story의 핵심 회귀 방지선이다.**
    // proxy 렌더가 수 초 걸리므로 먼저 찍은 사진이 나중에 끝날 수 있고, 그때 seq도
    // request order도 더 크게 부여된다. 촬영 시점 좌표만이 이 경우를 막는다.
    let base_dir = unique_test_root("proxy-older-capture");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let provenance = proxy_provenance("preset_soft-glow", "2026.08.01");

    // 두 번째 촬영이 먼저 끝나 화면에 올라간다.
    let second = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-2",
        "capture-2",
        1,
        &provenance,
        &sample_fixture_bytes("b"),
    );
    assert_eq!(second.reject_reason, None);

    // 첫 번째 촬영의 proxy가 뒤늦게 완료된다. seq는 더 크다.
    let first = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &provenance,
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        first.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_OLDER_CAPTURE),
        "먼저 찍고 늦게 끝난 사진이 현재 사진을 덮으면 안 된다"
    );
    assert_eq!(
        state
            .active_generation()
            .and_then(|generation| generation.capture_id.as_deref()),
        Some("capture-2")
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn one_photo_never_changes_its_look_mid_display() {
    let base_dir = unique_test_root("proxy-preset-mismatch");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();

    let published = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy_provenance("preset_soft-glow", "2026.08.01"),
        &sample_fixture_bytes("a"),
    );
    assert_eq!(published.reject_reason, None);

    // catalog rollback(Story 4.4) 뒤 같은 촬영이 다른 version으로 재렌더된 상황.
    let rolled_back = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy_provenance("preset_soft-glow", "2026.07.01"),
        &sample_fixture_bytes("b"),
    );

    assert_eq!(
        rolled_back.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PRESET_MISMATCH)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_partially_written_proxy_never_becomes_the_customer_photo() {
    let base_dir = unique_test_root("proxy-partial-file");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let mut truncated = sample_fixture_bytes("a");
    truncated.truncate(truncated.len() / 2);

    let outcome = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy_provenance("preset_soft-glow", "2026.08.01"),
        &truncated,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PARTIAL_FILE)
    );
    assert!(state.active_generation().is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn tier_specific_fields_are_rejected_before_any_file_is_written() {
    let base_dir = unique_test_root("proxy-shape-guard");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let provenance = proxy_provenance("preset_soft-glow", "2026.08.01");

    // provenance 없는 proxy는 출처를 증명할 수 없다.
    let result = publish_generation_in_dir(
        &base_dir,
        &mut state,
        &PublishRequest {
            session_id: SESSION_A,
            request_id: "req-1",
            capture_id: Some("capture-1"),
            tier: DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
            sample_variant: None,
            proxy_provenance: None,
            source_bytes: &sample_fixture_bytes("a"),
            bound_session_id: Some(SESSION_A),
            request_viewer_epoch: 3,
            current_viewer_epoch: 3,
            capture_order: Some(0),
            required_source_width_px: REQUIRED_WIDTH_1080P,
            required_source_height_px: REQUIRED_HEIGHT_1080P,
            lanes: DisplayLaneFlags::none(),
            refined_tier_justified: false,
        },
        &|| clock.now(),
    );
    assert!(result.is_err(), "출처 없는 proxy는 게시되면 안 된다");

    // fixture에 preset 출처가 실리면 거짓 출처가 evidence에 남는다.
    let mislabelled = publish_generation_in_dir(
        &base_dir,
        &mut state,
        &PublishRequest {
            session_id: SESSION_A,
            request_id: "req-1",
            capture_id: Some("capture-1"),
            tier: DISPLAY_TIER_SAMPLE,
            sample_variant: Some("a"),
            proxy_provenance: Some(&provenance),
            source_bytes: &sample_fixture_bytes("a"),
            bound_session_id: Some(SESSION_A),
            request_viewer_epoch: 3,
            current_viewer_epoch: 3,
            capture_order: None,
            required_source_width_px: REQUIRED_WIDTH_1080P,
            required_source_height_px: REQUIRED_HEIGHT_1080P,
            lanes: DisplayLaneFlags::none(),
            refined_tier_justified: false,
        },
        &|| clock.now(),
    );
    assert!(mislabelled.is_err());

    // 어느 쪽도 디스크에 자산을 남기지 않았다.
    assert!(!display_root(&base_dir, SESSION_A).join("req-1").exists());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn present_telemetry_stays_on_when_only_the_proxy_lane_is_running() {
    // **이 단언이 깨지면 HV-15 회차의 actual-present 행이 0건이 된다.**
    // proxy lane은 계측 lane이 아니므로 `measurementLaneEnabled`는 false여야 하지만,
    // 계측 IPC는 반드시 돌아야 한다.
    let lanes = DisplayLaneFlags {
        measurement_lane_enabled: false,
        present_telemetry_enabled: true,
    };
    let mut state = bound_state(SESSION_A);
    state.reserve_generation("req-1");

    let snapshot = state.snapshot(REQUIRED_WIDTH_1080P, REQUIRED_HEIGHT_1080P, lanes, 1_000);

    assert!(!snapshot.measurement_lane_enabled);
    assert!(snapshot.present_telemetry_enabled);
}

#[test]
fn the_proxy_lane_defaults_to_on_after_hv15_records_go() {
    use boothy_lib::display::proxy_publisher::{parse_proxy_lane_mode, ProxyLaneMode};

    // 환경 변수와 무관하게 결정적이다. 순수 파서를 직접 검증한다.
    assert_eq!(parse_proxy_lane_mode(None), ProxyLaneMode::On);
    assert_eq!(parse_proxy_lane_mode(Some("")), ProxyLaneMode::Off);
    assert_eq!(parse_proxy_lane_mode(Some("ON")), ProxyLaneMode::Off);
    assert_eq!(parse_proxy_lane_mode(Some("true")), ProxyLaneMode::Off);
    assert_eq!(parse_proxy_lane_mode(Some("1")), ProxyLaneMode::Off);
    assert!(parse_proxy_lane_mode(None).is_enabled());
}

#[test]
fn a_v1_pointer_written_by_an_earlier_build_still_reads() {
    // 한 HV 회차가 빌드 경계를 걸칠 수 있다. 파싱이 실패하면 오래된 표본이
    // 분모에서 조용히 사라진다.
    let base_dir = unique_test_root("proxy-pointer-v1");
    let root = display_root(&base_dir, SESSION_A);
    fs::create_dir_all(&root).expect("display root should be creatable");
    fs::write(
        root.join("pointer.json"),
        r#"{
  "schemaVersion": "viewer-display/v1",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "revision": 4,
  "activeGeneration": {
    "generationId": "req-1-000001",
    "generationSeq": 1,
    "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
    "requestId": "req-1",
    "captureId": "capture-1",
    "viewerEpoch": 3,
    "tier": "sample",
    "assetPath": "C:/tmp/000001-a.jpg",
    "sourceWidthPx": 3840,
    "sourceHeightPx": 2560,
    "byteSize": 700000,
    "sourceHash": "fnv1a64:0123456789abcdef",
    "sampleVariant": "a",
    "committedAtHostMicros": 1500000
  },
  "requiredSourceWidthPx": 1620,
  "requiredSourceHeightPx": 1080,
  "measurementLaneEnabled": true,
  "observedAtHostMicros": 1500000
}"#,
    )
    .expect("v1 pointer should be writable");

    let pointer = read_pointer(&base_dir, SESSION_A).expect("v1 pointer must still parse");
    let generation = pointer
        .active_generation
        .expect("v1 pointer carries an active generation");

    assert_eq!(generation.tier, DISPLAY_TIER_SAMPLE);
    assert_eq!(generation.sample_variant.as_deref(), Some("a"));
    assert_eq!(generation.proxy_provenance, None);

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn the_presets_already_shipping_today_carry_explicit_manual_approval() {
    // 내장 preset도 예외 없이 proxyPublication을 통과해야 한다. 승인 정보가 없는 bundle을
    // RAW 경로라는 이유만으로 올리는 우회로를 만들지 않는다.
    use boothy_lib::display::proxy_publisher::{
        evaluate_proxy_eligibility, ProxyLaneMode, ProxySourceRoute,
    };
    use boothy_lib::preset::default_catalog::ensure_default_preset_catalog_in_dir;
    use boothy_lib::preset::preset_bundle::ProxyEligibilityBasis;
    use boothy_lib::preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    };

    let base_dir = unique_test_root("exact-path-presets");
    fs::create_dir_all(&base_dir).expect("base dir");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should seed");

    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let shipped = [
        ("preset_soft-glow", "2026.03.27"),
        ("preset_mono-pop", "2026.03.27"),
        ("preset_daylight", "2026.03.27"),
    ];

    for (preset_id, published_version) in shipped {
        let bundle =
            find_published_preset_runtime_bundle(&catalog_root, preset_id, published_version)
                .unwrap_or_else(|| panic!("{preset_id} must be a published runtime bundle"));

        assert!(
            bundle.has_approved_visual_parity(),
            "{preset_id} must carry explicit proxyPublication approval"
        );

        let publication = evaluate_proxy_eligibility(
            ProxyLaneMode::On,
            false,
            Some(&bundle),
            ProxySourceRoute::RawOriginal,
            "5.4.1",
            REQUIRED_WIDTH_1080P,
            REQUIRED_HEIGHT_1080P,
        )
        .unwrap_or_else(|reason| {
            panic!(
                "{preset_id} must qualify on the exact path, got {}",
                reason.as_str()
            )
        });

        assert_eq!(
            publication.basis,
            ProxyEligibilityBasis::ApprovedVisualParity
        );
        assert_eq!(publication.reference_renderer, "darktable");
        assert_eq!(publication.reference_renderer_version, "5.4.1");
        assert_eq!(publication.output_color_space, "sRGB");
        // darktable이 품질을 넘기지 않았을 때 만드는 값과 같다 (실측 확인).
        assert_eq!(publication.jpeg_quality, 95);
        // 승인 recipe의 실체는 번들이 이미 싣고 있는 XMP다.
        assert!(publication.proxy_recipe_path.is_file());
        assert_eq!(publication.proxy_recipe_version, published_version);
        assert_eq!(
            publication.visual_approval_approved_at.as_deref(),
            Some("2026-08-14T00:00:00+09:00")
        );
        assert_eq!(
            publication.visual_approval_approved_by.as_deref(),
            Some("Noah Lee (manual product approval)")
        );
    }

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_shipped_preset_uses_its_explicit_approval_for_an_approved_fast_source() {
    use boothy_lib::display::proxy_publisher::{
        evaluate_proxy_eligibility, ProxyLaneMode, ProxySourceRoute,
    };
    use boothy_lib::preset::default_catalog::ensure_default_preset_catalog_in_dir;
    use boothy_lib::preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    };

    let base_dir = unique_test_root("fast-source-needs-approval");
    fs::create_dir_all(&base_dir).expect("base dir");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should seed");

    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let bundle =
        find_published_preset_runtime_bundle(&catalog_root, "preset_soft-glow", "2026.03.27")
            .expect("bundle");

    let publication = evaluate_proxy_eligibility(
        ProxyLaneMode::On,
        false,
        Some(&bundle),
        ProxySourceRoute::ApprovedFastSource("embedded-jpeg"),
        "5.4.1",
        REQUIRED_WIDTH_1080P,
        REQUIRED_HEIGHT_1080P,
    )
    .expect("the seeded preset carries explicit approval metadata");

    assert_eq!(publication.reference_renderer, "darktable");
    assert!(publication.visual_approval_approved_at.is_some());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_preset_pinned_to_another_renderer_version_is_refused_even_with_manual_approval() {
    // 참조 렌더러 버전이 런타임 pin과 다르면 "정확하다"는 주장 자체가 성립하지 않는다.
    use boothy_lib::display::proxy_publisher::{
        evaluate_proxy_eligibility, ProxyLaneMode, ProxySourceRoute,
    };
    use boothy_lib::preset::default_catalog::ensure_default_preset_catalog_in_dir;
    use boothy_lib::preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    };

    let base_dir = unique_test_root("renderer-pin-mismatch");
    fs::create_dir_all(&base_dir).expect("base dir");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should seed");

    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let bundle =
        find_published_preset_runtime_bundle(&catalog_root, "preset_soft-glow", "2026.03.27")
            .expect("bundle");

    let reason = evaluate_proxy_eligibility(
        ProxyLaneMode::On,
        false,
        Some(&bundle),
        ProxySourceRoute::RawOriginal,
        "5.6.0",
        REQUIRED_WIDTH_1080P,
        REQUIRED_HEIGHT_1080P,
    )
    .expect_err("a renderer version mismatch must refuse the exact-path claim");

    assert_eq!(reason.as_str(), "proxy-reference-renderer-mismatch");

    let _ = fs::remove_dir_all(&base_dir);
}

// ---------------------------------------------------------------------------
// Story 7.6: RAW 정밀본(`rawRefinedDisplay`) tier.
//
// 여기서 고정하는 것은 **승급**과 **역행 거부**, 그리고 AC 4의 crop/scale 점프 0을
// 만드는 크기 동일성이다. 세 가지 모두 "한 촬영이 이제 generation을 둘 만든다"는
// 사실에서 나온 요구다.
// ---------------------------------------------------------------------------

fn refined_provenance(
    preset_id: &str,
    preset_version: &str,
) -> boothy_lib::contracts::dto::DisplayProxyProvenanceDto {
    boothy_lib::contracts::dto::DisplayProxyProvenanceDto {
        render_quality: boothy_lib::contracts::dto::DISPLAY_RENDER_QUALITY_HIGH.into(),
        ..proxy_provenance(preset_id, preset_version)
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_refined(
    base_dir: &PathBuf,
    state: &mut DisplayState,
    clock: &StepClock,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    capture_order: u64,
    provenance: &boothy_lib::contracts::dto::DisplayProxyProvenanceDto,
    bytes: &[u8],
    tier_justified: bool,
) -> PublishOutcome {
    let request = PublishRequest {
        session_id,
        request_id,
        capture_id: Some(capture_id),
        tier: DISPLAY_TIER_RAW_REFINED_DISPLAY,
        sample_variant: None,
        proxy_provenance: Some(provenance),
        source_bytes: bytes,
        bound_session_id: Some(session_id),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        capture_order: Some(capture_order),
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
        lanes: DisplayLaneFlags {
            measurement_lane_enabled: false,
            present_telemetry_enabled: true,
        },
        refined_tier_justified: tier_justified,
    };

    publish_generation_in_dir(base_dir, state, &request, &|| clock.now())
        .expect("publish should not fail with an IO error")
}

/// pretty JSON에서 한 필드를 통째로 지운다. 이전 스키마 버전 fixture를 만들기 위해서다.
///
/// 앞 줄의 쉼표까지 함께 제거해야 유효한 JSON이 남는다.
fn strip_json_field(json: &str, field_name: &str) -> String {
    let needle = format!("\"{field_name}\":");
    let mut kept: Vec<String> = Vec::new();

    for line in json.lines() {
        if line.trim_start().starts_with(&needle) {
            if let Some(previous) = kept.last_mut() {
                if let Some(trimmed) = previous.strip_suffix(',') {
                    *previous = trimmed.to_string();
                }
            }

            continue;
        }

        kept.push(line.to_string());
    }

    kept.join("\n")
}

/// 구조만 갖춘 baseline JPEG. `probe_jpeg`가 보는 것은 SOI / SOF 크기 / EOI뿐이다.
/// 크기 불일치 표본을 만들기 위해 필요하다 — 번들 fixture 두 장은 크기가 같다.
fn sized_jpeg_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    bytes.extend_from_slice(b"JFIF\0");
    bytes.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
    bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
    bytes.extend_from_slice(&(height as u16).to_be_bytes());
    bytes.extend_from_slice(&(width as u16).to_be_bytes());
    bytes.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

#[test]
fn a_raw_refined_frame_promotes_the_proxy_of_the_same_capture() {
    let base_dir = unique_test_root("refined-promotion");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    let proxy_outcome = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    );
    assert_eq!(proxy_outcome.reject_reason, None);

    let refined_outcome = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        // 번들 fixture 두 장은 픽셀 크기가 같다. 실제 lane도 같은 크기를 강제한다.
        &sample_fixture_bytes("b"),
        true,
    );

    assert_eq!(refined_outcome.reject_reason, None);
    let committed = refined_outcome
        .committed
        .expect("refined should be committed");
    assert_eq!(committed.tier, DISPLAY_TIER_RAW_REFINED_DISPLAY);
    assert_eq!(committed.sample_variant, None);
    assert_eq!(
        committed
            .proxy_provenance
            .as_ref()
            .map(|value| value.render_quality.as_str()),
        Some("high"),
        "generation만 보고 두 tier를 구분할 수 있어야 한다"
    );
    // 확정 파일 이름이 proxy와 달라야 사람도 스크립트도 읽을 수 있다.
    assert!(committed
        .asset_path
        .replace('\\', "/")
        .contains("-refined.jpg"));
    assert!(PathBuf::from(&committed.asset_path).is_file());

    let pointer = read_pointer(&base_dir, SESSION_A).expect("pointer");
    assert_eq!(
        pointer
            .active_generation
            .as_ref()
            .map(|generation| generation.tier.as_str()),
        Some(DISPLAY_TIER_RAW_REFINED_DISPLAY)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_proxy_cannot_walk_the_same_capture_back_from_its_refined_frame() {
    let base_dir = unique_test_root("refined-no-downgrade");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    );
    publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sample_fixture_bytes("b"),
        true,
    );

    // 같은 촬영의 늦은 proxy 재시도가 정밀본을 되돌리려 한다.
    let late_proxy = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        late_proxy.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_TIER_DOWNGRADE)
    );
    assert!(late_proxy.committed.is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

/// **이 판정이 없으면 고객의 다음 사진이 화면에 영영 뜨지 않는다.**
///
/// 촬영 A가 정밀본(tier 2)까지 올라간 뒤 촬영 B의 proxy(tier 1)가 도착한다.
/// tier만 비교하면 `tier-downgrade`로 거부된다.
#[test]
fn the_next_photo_still_starts_at_the_proxy_tier_after_the_previous_photo_was_refined() {
    let base_dir = unique_test_root("refined-next-photo");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    );
    publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sample_fixture_bytes("b"),
        true,
    );

    let next_photo = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-2",
        "capture-2",
        1,
        &proxy,
        &sample_fixture_bytes("a"),
    );

    assert_eq!(
        next_photo.reject_reason, None,
        "다른 촬영의 낮은 tier는 하락이 아니라 새 사진이다"
    );
    let committed = next_photo.committed.expect("next photo should commit");
    assert_eq!(committed.capture_id.as_deref(), Some("capture-2"));
    assert_eq!(committed.tier, DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY);

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn an_older_captures_refined_frame_still_cannot_replace_the_current_photo() {
    let base_dir = unique_test_root("refined-older-capture");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    );
    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-2",
        "capture-2",
        1,
        &proxy,
        &sample_fixture_bytes("a"),
    );

    // 첫 촬영의 정밀본이 두 번째 촬영이 화면에 오른 뒤 뒤늦게 끝났다.
    let late_refined = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sample_fixture_bytes("b"),
        true,
    );

    assert_eq!(
        late_refined.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_OLDER_CAPTURE),
        "순서 문제는 크기 문제로 뭉뚱그려지면 안 된다"
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_partially_written_refined_frame_never_becomes_the_active_truth() {
    let base_dir = unique_test_root("refined-partial");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    let committed_proxy = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    )
    .committed
    .expect("proxy should commit");

    // EOI trailer 없이 잘린 정밀본.
    let full = sample_fixture_bytes("b");
    let truncated = full[..full.len() / 2].to_vec();
    let outcome = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &truncated,
        true,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PARTIAL_FILE)
    );
    // 화면은 proxy 그대로 남는다.
    let pointer = read_pointer(&base_dir, SESSION_A).expect("pointer");
    assert_eq!(
        pointer
            .active_generation
            .as_ref()
            .map(|generation| generation.generation_id.clone()),
        Some(committed_proxy.generation_id)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

/// **AC 4의 crop/scale 점프 0을 만드는 기계적 장치다.**
/// 한 픽셀만 달라도 `object-fit: contain` 박스가 달라져 교체 순간 사진이 튄다.
#[test]
fn a_refined_frame_whose_pixel_size_differs_from_the_proxy_is_refused() {
    let base_dir = unique_test_root("refined-dimension");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    let proxy_bytes = sample_fixture_bytes("a");
    let proxy_probe = probe_jpeg(&proxy_bytes).expect("probe");
    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &proxy_bytes,
    );

    let outcome = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sized_jpeg_fixture(proxy_probe.width_px - 1, proxy_probe.height_px),
        true,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH)
    );
    assert!(outcome.committed.is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

/// UX-DR19. 정밀본은 **승급**이지 첫 성공 화면의 대체가 아니다.
#[test]
fn a_refined_frame_without_a_committed_proxy_is_refused() {
    let base_dir = unique_test_root("refined-no-proxy");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    let outcome = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sample_fixture_bytes("a"),
        true,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_REFINED_DIMENSION_MISMATCH)
    );
    assert!(
        outcome.pointer.active_generation.is_none(),
        "proxy 없이 정밀본만 뜨면 첫 화면이 3배 느려진다"
    );
    // 거부됐으므로 pointer 파일 자체가 만들어지지 않는다. 화면은 standby 그대로다.
    assert!(read_pointer(&base_dir, SESSION_A).is_none());
    assert!(state.active_generation().is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

/// **측정되지 않은 tier 차이를 게시하지 않는다** (AC 6).
#[test]
fn an_unjustified_refined_tier_never_reaches_the_pointer() {
    let base_dir = unique_test_root("refined-not-justified");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    let committed_proxy = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    )
    .committed
    .expect("proxy should commit");

    let outcome = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sample_fixture_bytes("b"),
        false,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED)
    );
    let pointer = read_pointer(&base_dir, SESSION_A).expect("pointer");
    assert_eq!(
        pointer
            .active_generation
            .as_ref()
            .map(|generation| generation.generation_id.clone()),
        Some(committed_proxy.generation_id)
    );
    // 거부도 감사 기록에 남는다. 조용한 무시는 없다.
    let journal = read_journal(&base_dir, SESSION_A);
    assert!(journal.iter().any(|record| {
        record.reject_reason.as_deref() == Some(DISPLAY_REJECT_REFINED_TIER_NOT_JUSTIFIED)
    }));

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_late_refined_frame_from_a_superseded_preset_cannot_advance_the_pointer() {
    let base_dir = unique_test_root("refined-preset-mismatch");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let current_proxy = proxy_provenance("preset_soft-glow", "2026.08.10");
    let stale_refined = refined_provenance("preset_soft-glow", "2026.08.01");

    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &current_proxy,
        &sample_fixture_bytes("a"),
    );

    let outcome = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &stale_refined,
        &sample_fixture_bytes("b"),
        true,
    );

    assert_eq!(
        outcome.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PRESET_MISMATCH),
        "정체성 문제를 크기 문제로 뭉뚱그리면 회차 분석에서 원인이 사라진다"
    );

    let _ = fs::remove_dir_all(&base_dir);
}

/// **외부 환경 변수에 따라 건너뛰지 않는 결정적 테스트다** (Story 7.3 리뷰 지적사항).
#[test]
fn the_shipped_configuration_produces_no_refined_artifacts_at_all() {
    // 오늘의 제품 설정: lane 스위치 없음 + detail 축 미측정.
    let verdict = evaluate_raw_refined_eligibility(
        parse_raw_refined_lane_mode(None),
        RAW_REFINED_TIER_JUSTIFICATION,
        true,
        None,
        boothy_lib::render::PINNED_DARKTABLE_VERSION,
    );

    assert_eq!(verdict.unwrap_err(), RawRefinedSkipReason::LaneOff);
    // 스위치를 켜도 측정 원자료가 없으면 여전히 만들어지지 않는다.
    assert_eq!(
        evaluate_raw_refined_eligibility(
            parse_raw_refined_lane_mode(Some("on")),
            RAW_REFINED_TIER_JUSTIFICATION,
            true,
            None,
            boothy_lib::render::PINNED_DARKTABLE_VERSION,
        )
        .unwrap_err(),
        RawRefinedSkipReason::TierNotJustified
    );
}

#[test]
fn deleting_a_capture_clears_both_tiers_and_returns_to_standby() {
    let base_dir = unique_test_root("refined-delete");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let proxy = proxy_provenance("preset_soft-glow", "2026.08.01");
    let refined = refined_provenance("preset_soft-glow", "2026.08.01");

    let proxy_asset = publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &proxy,
        &sample_fixture_bytes("a"),
    )
    .committed
    .expect("proxy should commit")
    .asset_path;
    let refined_asset = publish_refined(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &refined,
        &sample_fixture_bytes("b"),
        true,
    )
    .committed
    .expect("refined should commit")
    .asset_path;

    assert!(PathBuf::from(&proxy_asset).is_file());
    assert!(PathBuf::from(&refined_asset).is_file());

    assert!(state.forget_request("req-1"));
    remove_request_generations(&base_dir, SESSION_A, "req-1", 9_000)
        .expect("removing generations should succeed");

    assert!(state.active_generation().is_none());
    assert!(!PathBuf::from(&proxy_asset).exists(), "proxy 자산이 남았다");
    assert!(
        !PathBuf::from(&refined_asset).exists(),
        "정밀본 자산이 남았다"
    );
    // append-only 감사 기록은 지우지 않는다.
    assert!(!read_journal(&base_dir, SESSION_A).is_empty());

    let _ = fs::remove_dir_all(&base_dir);
}

/// Story 7.9의 pre-upgrade session 호환이 이 규칙 위에 선다.
#[test]
fn durable_pointers_written_before_v4_are_still_readable() {
    let base_dir = unique_test_root("refined-pointer-compat");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let provenance = proxy_provenance("preset_soft-glow", "2026.08.01");

    publish_proxy(
        &base_dir,
        &mut state,
        &clock,
        SESSION_A,
        "req-1",
        "capture-1",
        0,
        &provenance,
        &sample_fixture_bytes("a"),
    );

    let pointer_path = display_root(&base_dir, SESSION_A).join("pointer.json");
    let current = fs::read_to_string(&pointer_path).expect("pointer file");

    for legacy_version in [
        "viewer-display/v1",
        "viewer-display/v2",
        "viewer-display/v3",
    ] {
        // 그 시절 행에는 `renderQuality`가 없었다. 지우고도 읽혀야 한다.
        let legacy = strip_json_field(
            &current.replace(VIEWER_DISPLAY_SCHEMA_VERSION, legacy_version),
            "renderQuality",
        );
        assert!(
            !legacy.contains("renderQuality"),
            "이전 버전 fixture에 새 필드가 남아 있으면 호환성을 검증하지 못한다"
        );
        fs::write(&pointer_path, &legacy).expect("write legacy pointer");

        let parsed = read_pointer(&base_dir, SESSION_A)
            .unwrap_or_else(|| panic!("{legacy_version} pointer must still be readable"));
        let generation = parsed
            .active_generation
            .expect("legacy pointer keeps its active generation");

        assert_eq!(generation.tier, DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY);
        assert_eq!(
            generation
                .proxy_provenance
                .as_ref()
                .map(|value| value.render_quality.as_str()),
            Some("fast"),
            "그 시절 게시 경로는 proxy lane 하나였고 항상 --hq false였다"
        );
    }

    let _ = fs::remove_dir_all(&base_dir);
}
