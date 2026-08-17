//! Story 7.6 scope guard.
//!
//! **체크박스로 때우지 않는다.** 「절대 하지 말 것」 목록을 실행되는 단언으로 고정한다.
//!
//! `git diff`로는 이 검증을 할 수 없다. 이 저장소의 `HEAD`는 Story 7.1이고 7.2~7.5 작업이
//! 아직 커밋되지 않아, HEAD와의 diff는 이 Story가 무엇을 바꿨는지 말해 주지 않는다.
//! 그래서 **범위 밖 값 자체를 여기서 직접 단언한다** — 이 방식은 커밋 이력과 무관하게
//! 앞으로도 계속 유효하다.

use boothy_lib::{
    display::{
        proxy_publisher::{parse_proxy_lane_mode, ProxyLaneMode},
        raw_refined_publisher::{
            parse_raw_refined_lane_mode, RawRefinedLaneMode, RAW_REFINED_TIER_JUSTIFICATION,
        },
        resident_renderer::{parse_resident_renderer_mode, ResidentRendererMode},
    },
    render::{MAX_IN_FLIGHT_CURRENT_CAPTURE_RENDER_JOBS, MAX_IN_FLIGHT_RENDER_JOBS},
};

/// `Cargo.toml`을 원문 그대로 읽는다. 의존성이 늘면 이 파일이 먼저 깨진다.
const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");

/// **승인되지 않은 Rust crate를 추가하지 않는다.**
///
/// Story 7.7의 offline 설치 인벤토리가 여기 걸려 있고, Story 7.3이 LibRaw를 뺀 것과 같은
/// 이유다. process tree 종료를 Windows 내장 `taskkill` 절대 경로 호출로 구현한 것도
/// `windows-sys` Job Object 방식이 이 목록을 늘리기 때문이다. `sha2`는 Story 7.7 self-check가
/// 외부 `certutil.exe` 창을 만들지 않고 설치 트리를 검증하기 위해 승인한 유일한 추가 항목이다.
#[test]
fn only_the_story_7_7_in_process_hash_dependency_was_added() {
    let dependencies: Vec<&str> = CARGO_MANIFEST
        .lines()
        .skip_while(|line| line.trim() != "[dependencies]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter_map(|line| line.split('=').next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();

    assert_eq!(
        dependencies,
        vec![
            "serde_json",
            "serde",
            "sha2",
            "log",
            "tauri",
            "tauri-plugin-log"
        ],
        "직접 의존성이 바뀌면 설치 재현 범위와 인벤토리를 함께 검토해야 한다"
    );
}

/// **상주 renderer를 켜지 않는다.** HV-16이 `Technology No-Go`다.
#[test]
fn the_resident_renderer_stays_off_by_default() {
    assert_eq!(
        parse_resident_renderer_mode(None),
        ResidentRendererMode::Off
    );
    assert_eq!(
        parse_resident_renderer_mode(Some("on")),
        ResidentRendererMode::Off
    );
    assert_eq!(
        parse_resident_renderer_mode(Some("EVIDENCE")),
        ResidentRendererMode::Off
    );
    assert!(!parse_resident_renderer_mode(None).publishes());
}

/// **Story 7.4가 `on`으로 확정한 proxy lane 기본값을 바꾸지 않는다.**
/// HV-15 `Go`가 그 위에 있다.
#[test]
fn the_proxy_lane_default_is_untouched_by_story_7_6() {
    assert_eq!(parse_proxy_lane_mode(None), ProxyLaneMode::On);
    assert!(parse_proxy_lane_mode(None).is_enabled());
}

/// **HV-17 `Go` 전까지 정밀본 lane은 기본 `off`다.**
///
/// 스위치를 켜더라도 AC 6의 detail 축 원자료가 없으면 게시되지 않는다.
/// 두 관문이 독립이어야 "스위치만 켜면 나가는" 상태가 만들어지지 않는다.
#[test]
fn the_raw_refined_lane_ships_off_and_unmeasured() {
    assert_eq!(parse_raw_refined_lane_mode(None), RawRefinedLaneMode::Off);
    assert!(
        !RAW_REFINED_TIER_JUSTIFICATION.is_justified(),
        "측정 원자료 없이 tier가 정당화되면 AC 6이 무의미해진다"
    );
}

/// 두 용량 값은 제품 결정이다. 조용히 바뀌면 첫 화면 지연이 조용히 늘어난다.
#[test]
fn the_render_capacity_constants_are_pinned() {
    assert_eq!(
        MAX_IN_FLIGHT_RENDER_JOBS, 2,
        "전체 동시 실행 상한은 오늘 값 유지"
    );
    assert_eq!(
        MAX_IN_FLIGHT_CURRENT_CAPTURE_RENDER_JOBS, 1,
        "현재 촬영 lane은 동시에 하나만 돈다"
    );
}

/// **384px 상수와 booth 사진 레일 경로를 바꾸지 않는다.**
///
/// 값 자체는 `render` 모듈 안에서 private이므로 렌더 인자 문자열로 확인한다.
/// 이 문자열이 바뀌면 Story 1.9의 fast preview 계약이 함께 바뀐 것이다.
#[test]
fn the_384px_preview_rail_is_out_of_scope() {
    let source = include_str!("../src/render/mod.rs");

    for constant in [
        "const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 384;",
        "const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 384;",
        "const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 384;",
        "const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 384;",
    ] {
        assert!(
            source.contains(constant),
            "384px 레일 상수가 사라졌거나 바뀌었다: {constant}"
        );
    }
}

/// **`final`은 관람 화면 tier가 아니다** (「핵심 설계 결정 2」).
#[test]
fn the_full_resolution_final_never_becomes_a_display_tier() {
    let dto = include_str!("../src/contracts/dto.rs");

    assert!(
        dto.contains("pub const DISPLAY_TIER_RAW_REFINED_DISPLAY: &str = \"rawRefinedDisplay\";")
    );
    assert!(
        !dto.contains("DISPLAY_TIER_FINAL"),
        "전체 해상도 산출물을 사진 영역에 올리면 축소를 브라우저가 하게 된다"
    );
    assert!(boothy_lib::contracts::dto::display_tier_order("final").is_none());
}
