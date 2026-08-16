//! Story 7.5: 상주 renderer spike의 게시 경계 회귀 테스트.
//!
//! 여기서 고정하는 것은 **"실험이 제품 경계를 우회하지 못한다"** 하나다.
//!
//! - 상주 결과도 Story 7.4와 **같은** 게시 순서를 지난다 (새 pointer 경로 없음)
//! - 같은 stale / preset-mismatch guard에 똑같이 걸린다
//! - 실패해도 지금 보이는 정상 프레임을 지우지 않는다
//! - **fixture로 얻은 결과가 production 자격을 주장하면 게시 전에 거부된다**

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use boothy_lib::{
    contracts::dto::{
        DISPLAY_REJECT_OLDER_CAPTURE, DISPLAY_REJECT_PRESET_MISMATCH,
        DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY,
    },
    display::{
        display_artifact::DisplayState,
        generation_repository::{read_journal, read_pointer},
        resident_renderer::{
            build_resident_provenance, compile_resident_recipe, publish_resident_generation_in_dir,
            ResidentInputProvenance, ResidentPublishJob, ResidentRecipe, ResidentRenderObservation,
            ResidentRendererMode, RESIDENT_ENGINE_VERSION, RESIDENT_ENGINE_WEBGL2,
        },
        DisplayLaneFlags,
    },
    preset::{
        default_catalog::ensure_default_preset_catalog_in_dir,
        preset_bundle::{ProxyEligibilityBasis, PublishedPresetProxyPublication},
        preset_catalog::{
            find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
        },
    },
};

const SESSION_A: &str = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
const REQUIRED_WIDTH_1080P: u32 = 1620;
const REQUIRED_HEIGHT_1080P: u32 = 1080;
const PUBLISHED_VERSION: &str = "2026.03.27";

fn unique_test_root(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-resident-{test_name}-{stamp}"))
}

/// 실제 배포 fixture를 쓴다. 합성 바이트로는 게시 경계의 probe를 통과할 수 없다.
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

fn bound_state(session_id: &str) -> DisplayState {
    let mut state = DisplayState::default();
    state.bind_session(session_id);
    state
}

/// **실제로 게시된 preset 번들**에서 상주 계획을 컴파일한다.
///
/// 합성 XMP로 검증하면 "우리가 만든 fixture는 통과한다"만 확인하게 된다.
/// 지금 고객에게 나가고 있는 세 preset이 실제로 컴파일되는지가 이 Story의 질문이다.
fn resident_recipe_for(preset_id: &str) -> ResidentRecipe {
    let seed_dir = unique_test_root(&format!("seed-{preset_id}"));
    fs::create_dir_all(&seed_dir).expect("seed dir");
    ensure_default_preset_catalog_in_dir(&seed_dir).expect("default catalog should seed");

    let catalog_root = resolve_published_preset_catalog_dir(&seed_dir);
    let bundle = find_published_preset_runtime_bundle(&catalog_root, preset_id, PUBLISHED_VERSION)
        .expect("published bundle");
    let publication = bundle
        .proxy_publication
        .clone()
        .expect("the seeded bundle carries an approved proxy publication");
    let xmp = fs::read_to_string(&publication.proxy_recipe_path).expect("bundle xmp");

    let recipe = compile_resident_recipe(&bundle, &publication, &xmp)
        .expect("an approved preset must compile into a resident plan");

    assert_eq!(recipe.engine_version, RESIDENT_ENGINE_VERSION);

    let _ = fs::remove_dir_all(&seed_dir);

    recipe
}

fn publication() -> PublishedPresetProxyPublication {
    PublishedPresetProxyPublication {
        basis: ProxyEligibilityBasis::ApprovedVisualParity,
        supported_operations: vec!["exposure".into()],
        proxy_recipe_version: PUBLISHED_VERSION.into(),
        proxy_recipe_path: PathBuf::from("C:/bundle/preset.xmp"),
        reference_renderer: "darktable".into(),
        reference_renderer_version: "5.4.1".into(),
        output_color_space: "sRGB".into(),
        jpeg_quality: 95,
        icc_intent: "perceptual".into(),
        visual_approval_approved_at: Some("2026-08-01T00:00:00+09:00".into()),
        visual_approval_approved_by: Some("Noah Lee".into()),
        visual_approval_corpus_path: Some("quality/corpus".into()),
    }
}

/// hot path가 비어 있던 정직한 관측. 컴파일 0건, process 시작 0건.
fn observation() -> ResidentRenderObservation {
    ResidentRenderObservation {
        producer_build_id: "spike-build-1".into(),
        program_hash: "fnv1a64:bcbf55e37b6cc622".into(),
        context_initialized_at_micros: 900_000,
        source_ready_at_micros: 1_200_000,
        hot_path_program_compile_count: 0,
        hot_path_process_start_count: 0,
        gpu_vendor: "NVIDIA".into(),
        gpu_renderer: "GeForce GTX 1080".into(),
    }
}

fn fixture_input() -> ResidentInputProvenance {
    ResidentInputProvenance::PredecodedFixture {
        producer: "darktable-cli".into(),
        producer_version: "5.4.1".into(),
    }
}

fn job<'a>(
    session_id: &'a str,
    request_id: &'a str,
    capture_id: &'a str,
    capture_order: u64,
) -> ResidentPublishJob<'a> {
    ResidentPublishJob {
        session_id,
        request_id,
        capture_id,
        bound_session_id: Some(session_id),
        request_viewer_epoch: 3,
        current_viewer_epoch: 3,
        capture_order,
        required_source_width_px: REQUIRED_WIDTH_1080P,
        required_source_height_px: REQUIRED_HEIGHT_1080P,
        display_profile_id: "approved-1080p",
        device_pixel_ratio: 1.0,
        source_asset_hash: "fnv1a64:00000000000000aa",
        lanes: DisplayLaneFlags {
            measurement_lane_enabled: false,
            present_telemetry_enabled: true,
        },
    }
}

#[test]
fn a_resident_frame_travels_the_same_immutable_publication_boundary() {
    let base_dir = unique_test_root("publish");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let provenance = build_resident_provenance(
        ResidentRendererMode::Evidence,
        &recipe,
        &fixture_input(),
        &observation(),
        None,
    );

    let outcome = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect("a resident publish must not fail with an IO error");

    assert_eq!(outcome.reject_reason, None);
    let committed = outcome.committed.expect("committed");

    // 새 tier도, 두 번째 pointer 경로도 만들지 않는다 (AC 7).
    assert_eq!(committed.tier, DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY);

    let proxy = committed
        .proxy_provenance
        .as_ref()
        .expect("proxy provenance");
    let resident = proxy
        .resident_provenance
        .as_ref()
        .expect("resident provenance must ride along");

    // 만든 renderer와 비교 기준 renderer가 분리되어 기록된다.
    assert_eq!(resident.producer_renderer, RESIDENT_ENGINE_WEBGL2);
    assert_eq!(proxy.reference_renderer, "darktable");
    // fixture 입력이므로 production 자격이 없고 그 이유가 함께 남는다.
    assert!(!resident.production_eligible);
    assert_eq!(
        resident.adoption_block_reason.as_deref(),
        Some("resident-source-route-unapproved")
    );
    assert_eq!(resident.hot_path_program_compile_count, 0);
    assert_eq!(resident.hot_path_process_start_count, 0);

    let journal = read_journal(&base_dir, SESSION_A);
    assert_eq!(journal.len(), 1);
    let pointer = read_pointer(&base_dir, SESSION_A).expect("pointer");
    assert_eq!(
        pointer.active_generation.map(|value| value.generation_id),
        Some(committed.generation_id)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_forged_production_eligible_resident_frame_never_reaches_the_disk() {
    let base_dir = unique_test_root("forged");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let mut provenance = build_resident_provenance(
        ResidentRendererMode::Evidence,
        &recipe,
        &fixture_input(),
        &observation(),
        None,
    );

    // fixture로 얻은 빠른 결과에 production 자격을 붙이는 것이 이 Story에서 가장 쉬운 자기기만이다.
    provenance.production_eligible = true;
    provenance.adoption_block_reason = None;

    let error = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect_err("a forged eligibility claim must be refused");

    assert_eq!(error.code, "validation-error");
    // 거부는 게시 **전에** 일어난다. 파일도 journal도 pointer도 남지 않는다.
    assert!(read_journal(&base_dir, SESSION_A).is_empty());
    assert!(read_pointer(&base_dir, SESSION_A).is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_resident_frame_from_an_older_capture_cannot_replace_the_current_screen() {
    let base_dir = unique_test_root("older-capture");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let provenance = build_resident_provenance(
        ResidentRendererMode::Evidence,
        &recipe,
        &fixture_input(),
        &observation(),
        None,
    );

    // 나중 촬영이 먼저 끝나 화면에 올라간다.
    let newer = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-2", 1),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect("publish");
    let newer_generation = newer.committed.expect("committed").generation_id;

    // 그 뒤 더 오래된 촬영의 상주 결과가 늦게 도착한다.
    let older = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("b"),
        &|| clock.now(),
    )
    .expect("publish");

    assert_eq!(
        older.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_OLDER_CAPTURE)
    );
    // 실패해도 지금 보이는 정상 프레임은 그대로 남는다.
    let pointer = read_pointer(&base_dir, SESSION_A).expect("pointer");
    assert_eq!(
        pointer.active_generation.map(|value| value.generation_id),
        Some(newer_generation)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_resident_frame_bound_to_another_preset_cannot_replace_the_current_screen() {
    let base_dir = unique_test_root("preset-mismatch");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let soft_glow = resident_recipe_for("preset_soft-glow");
    let daylight = resident_recipe_for("preset_daylight");

    publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &soft_glow,
        &publication(),
        &build_resident_provenance(
            ResidentRendererMode::Evidence,
            &soft_glow,
            &fixture_input(),
            &observation(),
            None,
        ),
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect("publish");

    // 세션 중 preset이 바뀐 뒤(Story 2.3) 이전 preset의 늦은 결과가 도착하는 상황이다.
    let mismatched = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &daylight,
        &publication(),
        &build_resident_provenance(
            ResidentRendererMode::Evidence,
            &daylight,
            &fixture_input(),
            &observation(),
            None,
        ),
        &sample_fixture_bytes("b"),
        &|| clock.now(),
    )
    .expect("publish");

    assert_eq!(
        mismatched.reject_reason.as_deref(),
        Some(DISPLAY_REJECT_PRESET_MISMATCH)
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn the_three_published_presets_compile_into_distinct_resident_plans() {
    use std::collections::BTreeSet;

    let hashes: BTreeSet<String> = ["preset_daylight", "preset_mono-pop", "preset_soft-glow"]
        .into_iter()
        .map(|preset_id| resident_recipe_for(preset_id).recipe_hash)
        .collect();

    // 세 preset이 같은 해시를 내면 drift 판정도 preset 구분도 성립하지 않는다.
    assert_eq!(hashes.len(), 3);
}

#[test]
fn a_shadow_frame_is_still_refused_when_it_hides_why_it_was_demoted() {
    let base_dir = unique_test_root("silent-demotion");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let mut provenance = build_resident_provenance(
        ResidentRendererMode::Shadow,
        &recipe,
        &fixture_input(),
        &observation(),
        None,
    );

    provenance.adoption_block_reason = None;

    let error = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect_err("a silent demotion must be refused");

    assert_eq!(error.code, "validation-error");
    assert!(read_pointer(&base_dir, SESSION_A).is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn deleting_the_capture_returns_the_viewer_to_standby_for_a_resident_frame_too() {
    use boothy_lib::display::generation_repository::remove_request_generations;

    let base_dir = unique_test_root("delete-to-standby");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let provenance = build_resident_provenance(
        ResidentRendererMode::Evidence,
        &recipe,
        &fixture_input(),
        &observation(),
        None,
    );

    publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect("publish");

    assert!(read_pointer(&base_dir, SESSION_A)
        .expect("pointer")
        .active_generation
        .is_some());

    // 사진 삭제는 tier와 producer에 상관없이 같은 저장소 경로를 지난다.
    state.forget_request("req-1");
    remove_request_generations(&base_dir, SESSION_A, "req-1", clock.now())
        .expect("removing the request generations must succeed");

    let snapshot = state.snapshot(
        REQUIRED_WIDTH_1080P,
        REQUIRED_HEIGHT_1080P,
        DisplayLaneFlags::none(),
        clock.now(),
    );

    // 삭제된 사진이 화면에 남아 있으면 고객은 지운 사진을 계속 보게 된다.
    assert!(snapshot.active_generation.is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn a_valid_shadow_frame_never_reaches_the_customer_pointer() {
    let base_dir = unique_test_root("shadow-never-publishes");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let provenance = build_resident_provenance(
        ResidentRendererMode::Shadow,
        &recipe,
        &fixture_input(),
        &observation(),
        None,
    );

    let error = publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect_err("shadow mode must never publish");

    assert_eq!(error.code, "validation-error");
    assert!(read_pointer(&base_dir, SESSION_A).is_none());

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn non_eligible_evidence_still_rejects_hot_path_work_and_invalid_timestamps() {
    let base_dir = unique_test_root("invalid-evidence-provenance");
    let mut state = bound_state(SESSION_A);
    let clock = StepClock::new();
    let recipe = resident_recipe_for("preset_soft-glow");
    let mut hot_path = observation();
    hot_path.hot_path_program_compile_count = 1;
    let hot_path_provenance = build_resident_provenance(
        ResidentRendererMode::Evidence,
        &recipe,
        &fixture_input(),
        &hot_path,
        None,
    );

    publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-1", "capture-1", 0),
        &recipe,
        &publication(),
        &hot_path_provenance,
        &sample_fixture_bytes("a"),
        &|| clock.now(),
    )
    .expect_err("hot-path work must invalidate evidence even when adoption is blocked");

    let mut reversed_time = observation();
    reversed_time.context_initialized_at_micros = 1_300_000;
    let reversed_time_provenance = build_resident_provenance(
        ResidentRendererMode::Evidence,
        &recipe,
        &fixture_input(),
        &reversed_time,
        None,
    );

    publish_resident_generation_in_dir(
        &base_dir,
        &mut state,
        &job(SESSION_A, "req-2", "capture-2", 1),
        &recipe,
        &publication(),
        &reversed_time_provenance,
        &sample_fixture_bytes("b"),
        &|| clock.now(),
    )
    .expect_err("context initialization must precede source readiness");

    assert!(read_pointer(&base_dir, SESSION_A).is_none());
    let _ = fs::remove_dir_all(&base_dir);
}
