//! Story 7.7 packaging guard.
//!
//! **번들 설정은 제품 결정이다.** 조용히 되돌아가면 clean offline 재현이 조용히 깨진다.
//! Story 7.6의 scope guard와 같은 방식으로, 「절대 하지 말 것」과 「승인된 기본안」을
//! 실행되는 단언으로 고정한다.
//!
//! 여기 있는 단언이 깨지면 그 회차의 설치본은 HV-18A 후보가 아니다.

use std::path::PathBuf;

use boothy_lib::session::session_repository::resolve_app_session_base_dir;

/// 번들 설정 원문. 키가 사라지면 이 파일이 먼저 깨진다.
const TAURI_CONF: &str = include_str!("../tauri.conf.json");

/// `Cargo.toml`을 원문 그대로 읽는다. 의존성이 늘면 인벤토리가 함께 바뀐다.
const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");

fn conf() -> serde_json::Value {
    serde_json::from_str(TAURI_CONF).expect("tauri.conf.json은 유효한 JSON이어야 한다")
}

/// **`identifier`는 스타터 기본값으로 되돌아가지 않는다** (「핵심 설계 결정 6」).
///
/// 이 값은 앱 데이터 루트, WebView2 user data 폴더, NSIS 업그레이드 식별에 쓰인다.
/// 첫 서명 release candidate가 지점에 깔린 뒤에 바꾸면 업그레이드가 "다른 앱 설치"가 된다.
#[test]
fn the_bundle_identifier_is_the_product_identity_not_the_starter_default() {
    let conf = conf();

    assert_eq!(
        conf["identifier"].as_str(),
        Some("com.boothy.booth"),
        "identifier가 바뀌면 %LOCALAPPDATA% 앱 데이터 루트와 업그레이드 경로가 함께 바뀐다"
    );
    assert_ne!(
        conf["identifier"].as_str(),
        Some("com.tauri.dev"),
        "스타터 기본값으로 되돌아가면 안 된다"
    );
}

/// **NSIS 하나만 만든다.** MSI를 함께 만들면 서명·해시·업그레이드 검증 대상이 두 벌이 된다.
#[test]
fn the_bundle_targets_only_nsis() {
    let conf = conf();
    let targets = conf["bundle"]["targets"]
        .as_array()
        .expect("bundle.targets는 배열이어야 한다");

    assert_eq!(
        targets
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>(),
        vec!["nsis"],
        "`all`은 MSI와 NSIS를 둘 다 만든다"
    );
}

/// **자동 업데이터는 명시적으로 꺼져 있다.** 기본값에 기대지 않는다.
#[test]
fn the_updater_artifacts_are_explicitly_disabled() {
    let conf = conf();

    assert_eq!(
        conf["bundle"]["createUpdaterArtifacts"],
        serde_json::Value::Bool(false),
        "release-baseline.md가 주장하는 가드레일은 설정에 실제로 적혀 있어야 한다"
    );
}

/// **WebView2는 오프라인 설치본으로 동봉한다.**
///
/// v2 스키마의 네 변형 중 인터넷 없이 성립하는 것은 `offlineInstaller` 하나다.
/// 기본값 `downloadBootstrapper`로 되돌아가면 clean offline VM에서 창이 뜨지 않는다.
#[test]
fn the_webview2_runtime_ships_inside_the_installer() {
    let conf = conf();
    let mode = &conf["bundle"]["windows"]["webviewInstallMode"];

    assert_eq!(
        mode["type"].as_str(),
        Some("offlineInstaller"),
        "downloadBootstrapper / embedBootstrapper는 인터넷을 요구한다"
    );
    assert_eq!(mode["silent"], serde_json::Value::Bool(true));
}

/// **공용 부스 PC이므로 `perMachine`이다.** darktable 트리를 담아야 한다.
#[test]
fn the_installer_is_per_machine_with_a_named_start_menu_folder() {
    let conf = conf();
    let nsis = &conf["bundle"]["windows"]["nsis"];

    assert_eq!(nsis["installMode"].as_str(), Some("perMachine"));
    assert_eq!(nsis["startMenuFolder"].as_str(), Some("Boothy"));
}

/// **서명 파이프라인은 완성해 두고, 인증서는 비어 있을 수 있다** (「승인이 필요한 결정」 4).
///
/// 인증서가 없는 동안 빌드는 미서명으로 성립해야 하고, 그 사실이 인벤토리에 기록된다.
/// 서명 입력 자체가 사라지면 인증서가 도착해도 붙일 자리가 없다.
#[test]
fn the_signing_inputs_exist_even_while_the_certificate_is_pending() {
    let conf = conf();
    let windows = &conf["bundle"]["windows"];

    assert_eq!(windows["digestAlgorithm"].as_str(), Some("sha256"));
    assert!(
        windows["timestampUrl"]
            .as_str()
            .is_some_and(|value| value.starts_with("http")),
        "timestamp 서버가 없으면 인증서 만료 후 서명이 무효가 된다"
    );
    assert!(
        windows.get("certificateThumbprint").is_some(),
        "인증서 지정 자리는 남아 있어야 한다 (값은 null일 수 있다)"
    );
}

#[test]
fn release_builds_cannot_bypass_the_inventory_gate() {
    let workflow = include_str!("../../.github/workflows/release-windows.yml");

    assert!(
        workflow
            .matches("steps.verify.outcome == 'success'")
            .count()
            >= 3,
        "release build, seal, evidence upload가 모두 inventory gate 성공 뒤에 있어야 한다"
    );
    assert!(
        workflow.contains(
            "if: github.event_name == 'workflow_dispatch' || startsWith(github.ref, 'refs/tags/boothy-v')"
        ),
        "수동 release도 tag release와 같은 최종 gate를 거쳐야 한다"
    );
}

#[test]
fn skip_helper_publish_can_recover_after_a_previous_marker() {
    let stage = include_str!("../../release/stage.ps1");

    assert!(
        stage
            .matches("Remove-Marker -Directory $helperOutput")
            .count()
            >= 2,
        "publish 전 정리뿐 아니라 검증 성공 경로도 이전 결손 marker를 제거해야 한다"
    );
}

/// **오프라인 재현에 필요한 세 페이로드가 전부 번들 resource로 선언되어 있다.**
///
/// 객체 표기 + 트레일링 슬래시여야 디렉터리 구조가 보존된다. glob을 쓰면 하위 구조가 뭉개진다.
#[test]
fn the_offline_payloads_are_declared_as_bundle_resources() {
    let conf = conf();
    let resources = conf["bundle"]["resources"]
        .as_object()
        .expect("bundle.resources는 객체 표기여야 한다");

    assert_eq!(
        resources
            .get("../release/dist/canon-helper/")
            .and_then(serde_json::Value::as_str),
        Some("sidecar/canon-helper/"),
        "helper는 externalBin이 아니라 resources로 간다 (동반 DLL 때문)"
    );
    assert_eq!(
        resources
            .get("../release/vendor/darktable-5.4.1/")
            .and_then(serde_json::Value::as_str),
        Some("darktable/"),
        "핀 고정된 darktable 트리를 통째로 옮긴다"
    );
    assert_eq!(
        resources
            .get("../release/dist/inventory.json")
            .and_then(serde_json::Value::as_str),
        Some("inventory.json"),
        "런타임 self-check가 대조할 매니페스트가 설치본에 있어야 한다"
    );
    assert_eq!(
        resources
            .get("../release/licenses/")
            .and_then(serde_json::Value::as_str),
        Some("licenses/"),
        "AC 1의 licensing 증거는 설치본과 함께 이동해야 한다"
    );

    for source in resources.keys() {
        assert!(
            !source.contains('*'),
            "bundle.resources 객체 표기에서 glob은 하위 디렉터리 구조를 보존하지 않는다: {source}"
        );
    }
}

/// **Story 7.2의 fixture 동봉을 유지한다.** AC 2의 fixture 표시가 그 위에 선다.
#[test]
fn the_display_sample_fixtures_stay_bundled() {
    let conf = conf();
    let resources = conf["bundle"]["resources"]
        .as_object()
        .expect("bundle.resources는 객체 표기여야 한다");

    for fixture in [
        "../storage/fixtures/display-sample/sample-a.jpg",
        "../storage/fixtures/display-sample/sample-b.jpg",
    ] {
        assert!(
            resources.contains_key(fixture),
            "fixture 표시 회차가 딛고 설 발판이 사라졌다: {fixture}"
        );
    }
}

/// 설치 검증 runbook은 실제 parser가 허용하는 mode를 사용해야 한다.
/// `on`은 fail-closed로 `off`가 되므로 fixture가 아닌 standby 화면이 나온다.
#[test]
fn the_hv18a_runbook_enables_the_fixture_lane_with_a_supported_mode() {
    let runbook = include_str!("../../tests/hardware/installer/hv-18a/README.md");

    assert!(
        runbook.contains("$env:BOOTHY_DISPLAY_SAMPLE_MODE = 'visible-standby'"),
        "HV-18A fixture 절차는 parser가 허용하는 visible-standby를 써야 한다"
    );
    assert!(
        !runbook.contains("$env:BOOTHY_DISPLAY_SAMPLE_MODE = 'on'"),
        "알 수 없는 on 값은 안전하게 off로 닫혀 standby만 보인다"
    );
}

/// **`identifier`를 바꿔도 고객 사진과 세션 루트는 움직이지 않는다.**
///
/// 세션 루트는 `%USERPROFILE%\Pictures\dabi_shoot`이고 `app_local_data_dir`는 폴백이다.
/// 폴백만이 `identifier`에 딸려 움직이며, 그 사실을 릴리스 노트와 README에 적는다.
#[test]
fn the_identifier_change_does_not_move_the_customer_session_root() {
    let original_user_profile = std::env::var_os("USERPROFILE");
    let app_local_data_dir = PathBuf::from("C:/local-app-data/com.boothy.booth");

    std::env::remove_var("USERPROFILE");
    let fallback = resolve_app_session_base_dir(app_local_data_dir.clone());
    assert_eq!(
        fallback,
        app_local_data_dir.join("dabi_shoot"),
        "폴백 경로만 identifier를 따라 움직인다"
    );

    std::env::set_var("USERPROFILE", "C:/Users/booth-operator");
    let primary = resolve_app_session_base_dir(app_local_data_dir);
    assert_eq!(
        primary,
        PathBuf::from("C:/Users/booth-operator")
            .join("Pictures")
            .join("dabi_shoot"),
        "고객 사진은 identifier와 무관한 자리에 남는다"
    );

    match original_user_profile {
        Some(value) => std::env::set_var("USERPROFILE", value),
        None => std::env::remove_var("USERPROFILE"),
    }
}

/// **asset protocol scope는 두 자리를 그대로 유지한다.**
///
/// `$APPLOCALDATA/dabi_shoot/**`가 가리키는 실경로는 `identifier`와 함께 움직인다.
/// 두 항목 중 하나라도 사라지면 폴백 경로의 사진이 관람 화면에 뜨지 않는다.
#[test]
fn the_asset_protocol_scope_still_covers_both_session_roots() {
    let conf = conf();
    let scope = conf["app"]["security"]["assetProtocol"]["scope"]
        .as_array()
        .expect("assetProtocol.scope는 배열이어야 한다");

    let entries = scope
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();

    assert!(entries.contains(&"$PICTURE/dabi_shoot/**"));
    assert!(entries.contains(&"$APPLOCALDATA/dabi_shoot/**"));
}

/// **helper 해석 코드는 수정하지 않는다** (「핵심 설계 결정 3」).
///
/// `bundle.resources`가 helper 트리를 `sidecar/canon-helper/`로 내려놓고,
/// `resolve_helper_launch_target()`의 **기존 후보**가 그 자리를 이미 보고 있다.
/// 두 사실 중 하나만 바뀌어도 clean VM에서 촬영이 되지 않으므로 함께 고정한다.
#[test]
fn the_bundled_helper_lands_exactly_where_the_existing_resolver_already_looks() {
    let supervisor = include_str!("../src/capture/helper_supervisor.rs");

    assert!(
        supervisor.contains(r#"current_dir.join("sidecar/canon-helper/canon-helper.exe")"#),
        "설치본 안 helper를 찾는 기존 후보가 사라졌다"
    );

    let conf = conf();
    assert_eq!(
        conf["bundle"]["resources"]["../release/dist/canon-helper/"].as_str(),
        Some("sidecar/canon-helper/"),
        "resource 대상 경로가 helper 해석 후보와 어긋나면 촬영이 불가능해진다"
    );
}

/// **externalBin으로 helper를 넣지 않는다.** 동반 DLL을 싣지 못한다.
#[test]
fn the_helper_is_not_smuggled_in_through_external_bin() {
    let conf = conf();

    assert!(
        conf["bundle"].get("externalBin").is_none(),
        "externalBin은 EDSDK 네이티브 DLL과 .NET 런타임 어셈블리를 함께 담지 못한다"
    );
}

/// **self-check 실패는 진단으로만 투영된다.** 고객 흐름을 새로 막지 않는다.
///
/// 부팅 경로의 호출은 반환값이 없고, `release` 모듈에는 command가 없다.
/// 값을 돌려주기 시작하면 그 값으로 무언가를 막고 싶어진다 — 그 문을 여기서 닫는다.
#[test]
fn the_release_governance_check_cannot_block_the_booth_flow() {
    let lib = include_str!("../src/lib.rs");
    let self_check = include_str!("../src/release/self_check.rs");

    assert!(
        lib.contains(
            "release::self_check::run_startup_release_governance_check(&runtime_base_dir);"
        ),
        "부팅 경로의 투영이 사라졌다"
    );
    assert!(
        self_check.contains("pub fn run_startup_release_governance_check(base_dir: &Path) {"),
        "투영 함수가 값을 돌려주기 시작하면 고객 흐름을 막는 데 쓰이게 된다"
    );
    assert!(
        !self_check.contains("#[tauri::command"),
        "self-check는 command가 아니다. Tauri 빌더를 세우기 전에 끝난다"
    );
}

/// **관람 화면과 렌더 인자는 이 Story가 지나갈 뿐 만지지 않는다** (DoD 10).
///
/// Story 7.6의 scope guard와 같은 방식이다. `git diff`로는 이 검증을 할 수 없다.
#[test]
fn the_customer_facing_render_arguments_are_untouched() {
    let render = include_str!("../src/render/mod.rs");

    for fragment in [
        "\"--upscale\".into(),\n        \"false\".into(),",
        "\"--icc-type\".into(),",
        "\"--icc-intent\".into(),",
        "\"srgb\" => Ok(\"SRGB\"),",
        "\"perceptual\" => Ok(\"PERCEPTUAL\"),",
    ] {
        assert!(
            render.contains(fragment),
            "HV-15 `Go`가 서 있는 렌더 인자가 바뀌었다: {fragment}"
        );
    }
}

/// **384px 레일과 lane 기본값을 바꾸지 않는다.**
#[test]
fn the_display_lane_defaults_are_untouched() {
    use boothy_lib::display::{
        proxy_publisher::{parse_proxy_lane_mode, ProxyLaneMode},
        raw_refined_publisher::{parse_raw_refined_lane_mode, RawRefinedLaneMode},
        resident_renderer::{parse_resident_renderer_mode, ResidentRendererMode},
    };

    assert_eq!(
        parse_proxy_lane_mode(None),
        ProxyLaneMode::On,
        "Story 7.4가 확정한 proxy lane 기본값"
    );
    assert_eq!(
        parse_raw_refined_lane_mode(None),
        RawRefinedLaneMode::Off,
        "HV-17 `Go` 전까지 정밀본 lane은 기본 off"
    );
    assert_eq!(
        parse_resident_renderer_mode(None),
        ResidentRendererMode::Off,
        "HV-16 `Technology No-Go`. 이 Story가 되살리지 않는다"
    );

    let render = include_str!("../src/render/mod.rs");
    for constant in [
        "const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 384;",
        "const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 384;",
    ] {
        assert!(
            render.contains(constant),
            "384px 레일이 바뀌었다: {constant}"
        );
    }
}

/// **인벤토리는 세션 진실이 아니다.** `session.json`을 확장하지 않는다.
#[test]
fn the_session_manifest_is_not_extended_with_release_facts() {
    let manifest = include_str!("../src/session/session_manifest.rs");

    for forbidden in ["inventory", "signingStatus", "selfCheck"] {
        assert!(
            !manifest.contains(forbidden),
            "세션 매니페스트에 릴리스 사실이 새어 들어갔다: {forbidden}"
        );
    }
}

/// 직접 의존성은 릴리스 검토가 놓치지 않도록 고정한다.
///
/// `sha2`는 파일마다 외부 콘솔 프로세스를 생성하지 않고 설치 트리를 검증하기 위한 의존성이다.
#[test]
fn story_7_7_direct_dependencies_are_explicit() {
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
        "직접 의존성이 늘면 오프라인 인벤토리와 라이선스 증거가 함께 늘어난다"
    );
}
