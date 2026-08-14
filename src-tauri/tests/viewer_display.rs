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
        DISPLAY_REJECT_OLDER_REQUEST, DISPLAY_REJECT_PARTIAL_FILE, DISPLAY_REJECT_SESSION_MISMATCH,
        DISPLAY_REJECT_STALE_EPOCH, DISPLAY_REJECT_UNDECODABLE, DISPLAY_REJECT_UNKNOWN_GENERATION,
        DISPLAY_REJECT_VIEWER_NOT_READY, VIEWER_DISPLAY_SCHEMA_VERSION,
    },
    display::{
        display_artifact::DisplayState,
        generation_repository::{
            display_root, generation_path, read_journal, read_pointer, remove_request_generations,
            write_reconciled_pointer,
        },
        image_probe::probe_jpeg,
        sample_publisher::{publish_generation_in_dir, PublishRequest},
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
) -> boothy_lib::display::sample_publisher::PublishOutcome {
    let request = PublishRequest {
        session_id,
        request_id,
        capture_id: Some("capture-1"),
        sample_variant: variant,
        source_bytes: bytes,
        bound_session_id,
        request_viewer_epoch: viewer_epoch,
        current_viewer_epoch: viewer_epoch,
        required_source_width_px: required.0,
        required_source_height_px: required.1,
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
            sample_variant: "b",
            source_bytes: &sample_fixture_bytes("b"),
            bound_session_id: Some(SESSION_A),
            request_viewer_epoch: 3,
            current_viewer_epoch: 4,
            required_source_width_px: REQUIRED_WIDTH_1080P,
            required_source_height_px: REQUIRED_HEIGHT_1080P,
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
            .map(|g| g.sample_variant.clone()),
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
        sample_variant: "b",
        source_bytes: &bytes,
        bound_session_id: Some(SESSION_A),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
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
        sample_variant: "a",
        source_bytes: &bytes,
        bound_session_id: Some(SESSION_A),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
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
        sample_variant: "a",
        source_bytes: &bytes,
        bound_session_id: Some(SESSION_A),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
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
    let snapshot = state.snapshot(required.0, required.1, false, clock.now());
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
    let snapshot = state.snapshot(5000, 3400, false, clock.now());
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
        false,
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
        viewer_epoch: 3,
        tier: "sample",
        generation_seq: current.generation_seq,
        source_width_px: 3840,
        source_height_px: 2560,
        request_order: state.request_order("req-1").expect("request order"),
    };
    let context = boothy_lib::display::display_artifact::AdmissionContext {
        bound_session_id: Some(SESSION_A),
        viewer_epoch: 3,
        required_source_width_px: required.0,
        required_source_height_px: required.1,
    };

    assert_eq!(
        boothy_lib::display::display_artifact::evaluate_admission(
            Some(&current),
            state.active_request_order(),
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
        viewer_epoch: 3,
        tier: "future-tier",
        generation_seq: 1,
        source_width_px: 3840,
        source_height_px: 2560,
        request_order: 0,
    };

    assert_eq!(
        boothy_lib::display::display_artifact::evaluate_admission(None, None, &candidate, &context),
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
        viewer_epoch: 3,
        tier: "sample",
        generation_seq: 1,
        source_width_px: 3840,
        source_height_px: 2560,
        request_order: 0,
    };

    assert_eq!(
        boothy_lib::display::display_artifact::evaluate_admission(None, None, &candidate, &context),
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
