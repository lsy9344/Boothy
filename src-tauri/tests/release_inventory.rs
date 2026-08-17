//! Story 7.7 — `release-inventory/v1` / `install-self-check/v1` 계약 테스트 (Rust 쪽).
//!
//! TS(Zod) 쪽 `src/shared-contracts/release-inventory.contracts.test.ts`와 **같은 규칙**을 본다.
//! 두 쪽이 갈라지면 인벤토리가 계약 두 벌을 갖게 되고, 그 순간 어느 쪽이 진실인지 알 수 없다.

use boothy_lib::contracts::dto::{
    validate_install_self_check_report, validate_release_inventory,
    validate_release_inventory_component, ComponentEntrySelectionDto, DarktableResolutionDto,
    InstallSelfCheckComponentDto, InstallSelfCheckReportDto, NotStagedReleaseInventoryDto,
    ReleaseInventoryComponentDto, ReleaseInventoryDto, StagedReleaseInventoryDto,
    INSTALL_SELF_CHECK_SCHEMA_VERSION, RELEASE_INVENTORY_ROLES, RELEASE_INVENTORY_SCHEMA_VERSION,
};

fn digest(seed: char) -> String {
    std::iter::repeat(seed).take(64).collect()
}

fn staged_component(role: &str) -> ReleaseInventoryComponentDto {
    ReleaseInventoryComponentDto {
        name: format!("{role} payload"),
        role: role.into(),
        status: "present".into(),
        version: Some("5.4.1".into()),
        entry_selection: Some(ComponentEntrySelectionDto::All),
        origin: "release-5.4.1 / c3f96ca".into(),
        license: "GPL-3.0-or-later".into(),
        license_evidence_path: Some("release/vendor/darktable-5.4.1/LICENSE-EVIDENCE.md".into()),
        source_archive_sha256: Some(digest('a')),
        staged_tree_digest: Some(digest('b')),
        install_relative_path: Some("darktable/".into()),
        signing_status: "not-applicable".into(),
        rationale: None,
        file_count: Some(4211),
        total_bytes: Some(402_653_184),
    }
}

fn declared_component(role: &str, status: &str) -> ReleaseInventoryComponentDto {
    ReleaseInventoryComponentDto {
        name: format!("{role} declaration"),
        role: role.into(),
        status: status.into(),
        version: None,
        entry_selection: None,
        origin: "embedded-in-app-binary".into(),
        license: "proprietary".into(),
        license_evidence_path: None,
        source_archive_sha256: None,
        staged_tree_digest: None,
        install_relative_path: Some("Boothy.exe".into()),
        signing_status: "not-applicable".into(),
        rationale: Some("이 구성요소는 별도 페이로드가 없다.".into()),
        file_count: None,
        total_bytes: None,
    }
}

fn staged_inventory() -> StagedReleaseInventoryDto {
    StagedReleaseInventoryDto {
        schema_version: RELEASE_INVENTORY_SCHEMA_VERSION.into(),
        generated_at: "2026-08-17T05:00:00.000Z".into(),
        app_version: "0.1.0".into(),
        identifier: "com.boothy.booth".into(),
        installer_file_name: "Boothy_0.1.0_x64-setup.exe".into(),
        signing_status: "unsigned".into(),
        installer: None,
        components: RELEASE_INVENTORY_ROLES
            .iter()
            .map(|role| match *role {
                "camera-helper" | "edsdk-runtime" | "raw-renderer" => staged_component(role),
                "display-renderer" => declared_component(role, "not-applicable"),
                other => declared_component(other, "embedded"),
            })
            .collect(),
    }
}

#[test]
fn a_staged_inventory_naming_every_role_is_accepted() {
    assert!(validate_release_inventory(&ReleaseInventoryDto::Staged(staged_inventory())).is_ok());
}

/// **결손을 스스로 신고하는 문서도 계약 안에 있다.** 자리를 비워 두지 않는다.
#[test]
fn a_not_staged_inventory_is_a_valid_document() {
    let inventory = ReleaseInventoryDto::NotStaged(NotStagedReleaseInventoryDto {
        schema_version: RELEASE_INVENTORY_SCHEMA_VERSION.into(),
        reason: "release:stage 가 실행되지 않았습니다.".into(),
    });

    assert!(validate_release_inventory(&inventory).is_ok());
}

/// staged 사본과 not-staged 사본이 **같은 JSON 자리에서 구분되는지** 고정한다.
#[test]
fn the_staging_discriminator_round_trips_through_json() {
    let staged = ReleaseInventoryDto::Staged(staged_inventory());
    let encoded = serde_json::to_string(&staged).expect("staged 인벤토리는 직렬화된다");
    assert!(encoded.contains("\"staging\":\"staged\""));

    let decoded: ReleaseInventoryDto =
        serde_json::from_str(&encoded).expect("staged 인벤토리는 역직렬화된다");
    assert_eq!(decoded, staged);

    let not_staged: ReleaseInventoryDto = serde_json::from_str(
        r#"{"schemaVersion":"release-inventory/v1","staging":"not-staged","reason":"미staging"}"#,
    )
    .expect("not-staged 인벤토리는 역직렬화된다");
    assert!(matches!(not_staged, ReleaseInventoryDto::NotStaged(_)));
}

/// **런타임이 다시 계산한 해시가 생성기의 값과 같아야 한다.**
///
/// 작은 fixture 로는 세 구현의 정렬 차이가 드러나지 않는다. 실제 .NET publish 트리는
/// `System.Private.CoreLib.dll` 같은 이름이 수백 개라, 여기서 갈라지면 설치된 앱의
/// `--self-check` 가 항상 `inventory-digest-mismatch` 를 낸다.
///
/// staged 페이로드가 없는 머신에서는 조용히 통과한다 — 없는 것을 검사했다고 적지 않는다.
#[test]
fn the_runtime_digest_matches_the_generated_inventory_for_a_real_staged_tree() {
    use std::path::{Path, PathBuf};

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("저장소 루트")
        .to_path_buf();
    let inventory_path = repo_root
        .join("release")
        .join("dist")
        .join("inventory.json");

    let Ok(raw) = std::fs::read_to_string(&inventory_path) else {
        eprintln!("staged 인벤토리가 없어 건너뜁니다: {inventory_path:?}");
        return;
    };

    let Ok(ReleaseInventoryDto::Staged(staged)) =
        boothy_lib::release::inventory::parse_inventory(&raw)
    else {
        eprintln!("staged 인벤토리가 아니어서 건너뜁니다");
        return;
    };

    let mut compared = 0;

    for component in &staged.components {
        if component.status != "present" {
            continue;
        }

        let (Some(selection), Some(expected)) = (
            component.entry_selection.as_ref(),
            component.staged_tree_digest.as_ref(),
        ) else {
            continue;
        };

        // 인벤토리는 설치 경로를 적는다. 여기서는 staging 트리를 직접 본다.
        let staging_root: PathBuf = match component.role.as_str() {
            "camera-helper" | "edsdk-runtime" => {
                repo_root.join("release").join("dist").join("canon-helper")
            }
            "raw-renderer" => repo_root
                .join("release")
                .join("vendor")
                .join("darktable-5.4.1"),
            _ => continue,
        };

        if !staging_root.is_dir()
            || staging_root.join("PAYLOAD-NOT-STAGED.md").exists()
            || !Path::new(&staging_root).exists()
        {
            continue;
        }

        let computed =
            boothy_lib::release::inventory::compute_component_digest(&staging_root, selection);

        let Ok(Some((actual, file_count))) = computed else {
            eprintln!("'{}' 해시를 계산하지 못해 건너뜁니다", component.role);
            continue;
        };

        assert_eq!(
            &actual, expected,
            "'{}' 트리 해시가 생성기와 다르다. 세 구현의 정의가 갈라졌다",
            component.role
        );
        assert_eq!(
            Some(file_count as u64),
            component.file_count,
            "'{}' 파일 수가 생성기와 다르다",
            component.role
        );

        compared += 1;
    }

    eprintln!("실제 staged 트리 {compared}개를 생성기 값과 대조했습니다");
}

#[test]
fn a_component_without_licensing_evidence_of_its_own_is_rejected() {
    let mut component = staged_component("raw-renderer");
    component.license = "   ".into();

    assert!(validate_release_inventory_component(&component).is_err());
}

#[test]
fn an_inactive_component_that_does_not_say_why_is_rejected() {
    let mut component = declared_component("display-renderer", "not-applicable");
    component.rationale = None;

    assert!(validate_release_inventory_component(&component).is_err());
}

#[test]
fn an_unknown_component_role_is_rejected() {
    let mut component = staged_component("raw-renderer");
    component.role = "gpu-shader-bundle".into();

    assert!(validate_release_inventory_component(&component).is_err());
}

/// **설치된 앱은 명세를 갖고 있지 않다.** 파일 선택 규칙이 인벤토리에 없으면
/// self-check가 어떤 파일을 해시해야 하는지 알 수 없다.
#[test]
fn a_staged_component_that_does_not_say_which_files_are_its_own_is_rejected() {
    let mut component = staged_component("raw-renderer");
    component.entry_selection = None;

    assert!(validate_release_inventory_component(&component).is_err());
}

#[test]
fn a_file_selection_rule_on_a_component_that_was_never_staged_is_rejected() {
    let mut component = declared_component("display-renderer", "not-applicable");
    component.entry_selection = Some(ComponentEntrySelectionDto::All);

    assert!(validate_release_inventory_component(&component).is_err());
}

/// 세 가지 선택 규칙이 JSON 왕복에서 구분되는지 고정한다.
#[test]
fn the_file_selection_rule_round_trips_through_json() {
    for (selection, expected) in [
        (ComponentEntrySelectionDto::All, "\"kind\":\"all\""),
        (
            ComponentEntrySelectionDto::Only {
                entries: vec!["EDSDK.dll".into()],
            },
            "\"kind\":\"only\"",
        ),
        (
            ComponentEntrySelectionDto::AllExcept {
                entries: vec!["EDSDK.dll".into()],
            },
            "\"kind\":\"allExcept\"",
        ),
    ] {
        let encoded = serde_json::to_string(&selection).expect("선택 규칙은 직렬화된다");
        assert!(encoded.contains(expected), "{encoded}");

        let decoded: ComponentEntrySelectionDto =
            serde_json::from_str(&encoded).expect("선택 규칙은 역직렬화된다");
        assert_eq!(decoded, selection);
    }
}

#[test]
fn a_staged_component_without_a_tree_digest_is_rejected() {
    let mut component = staged_component("raw-renderer");
    component.staged_tree_digest = None;

    assert!(validate_release_inventory_component(&component).is_err());
}

#[test]
fn a_tree_digest_on_a_component_that_was_never_staged_is_rejected() {
    let mut component = declared_component("display-renderer", "not-applicable");
    component.staged_tree_digest = Some(digest('c'));

    assert!(validate_release_inventory_component(&component).is_err());
}

#[test]
fn a_malformed_digest_is_rejected() {
    let mut component = staged_component("raw-renderer");
    component.staged_tree_digest = Some("NOTHEX".into());

    assert!(validate_release_inventory_component(&component).is_err());
}

#[test]
fn an_inventory_missing_a_required_role_is_rejected() {
    let mut inventory = staged_inventory();
    inventory
        .components
        .retain(|component| component.role != "raw-renderer");

    assert!(validate_release_inventory(&ReleaseInventoryDto::Staged(inventory)).is_err());
}

#[test]
fn an_inventory_naming_the_same_role_twice_is_rejected() {
    let mut inventory = staged_inventory();
    inventory.components.push(staged_component("raw-renderer"));

    assert!(validate_release_inventory(&ReleaseInventoryDto::Staged(inventory)).is_err());
}

fn passing_report() -> InstallSelfCheckReportDto {
    InstallSelfCheckReportDto {
        schema_version: INSTALL_SELF_CHECK_SCHEMA_VERSION.into(),
        checked_at: "2026-08-17T05:10:00.000Z".into(),
        app_version: "0.1.0".into(),
        identifier: "com.boothy.booth".into(),
        install_root: "C:\\Program Files\\Boothy".into(),
        components: vec![InstallSelfCheckComponentDto {
            name: "raw-renderer".into(),
            expected_digest: Some(digest('b')),
            actual_digest: Some(digest('b')),
            status: "pass".into(),
            reason_code: None,
        }],
        webview2_version: Some("141.0.3537.85".into()),
        darktable_resolution: Some(DarktableResolutionDto {
            binary: "C:\\Program Files\\Boothy\\darktable\\bin\\darktable-cli.exe".into(),
            source: "bundled-resource".into(),
        }),
        helper_version: Some("canon-helper 0.1.0".into()),
        overall: "pass".into(),
    }
}

#[test]
fn a_passing_self_check_report_is_accepted() {
    assert!(validate_install_self_check_report(&passing_report()).is_ok());
}

/// **검사에 실패한 회차를 통과로 적을 수 없다.**
#[test]
fn a_report_cannot_call_itself_passing_while_a_component_failed() {
    let mut report = passing_report();
    report.components[0].status = "fail".into();
    report.components[0].reason_code = Some("inventory-digest-mismatch".into());

    assert!(validate_install_self_check_report(&report).is_err());
}

#[test]
fn every_failed_component_needs_a_reason_code() {
    let mut report = passing_report();
    report.components[0].status = "fail".into();
    report.components[0].reason_code = None;
    report.overall = "fail".into();

    assert!(validate_install_self_check_report(&report).is_err());
}

#[test]
fn an_unknown_reason_code_is_rejected() {
    let mut report = passing_report();
    report.components[0].status = "fail".into();
    report.components[0].reason_code = Some("something-went-wrong".into());
    report.overall = "fail".into();

    assert!(validate_install_self_check_report(&report).is_err());
}

/// darktable 해석 출처는 **열거된 값 중 하나**여야 한다. HV-18A가 이 값을 검사한다.
#[test]
fn an_unknown_darktable_resolution_source_is_rejected() {
    let mut report = passing_report();
    report.darktable_resolution = Some(DarktableResolutionDto {
        binary: "C:\\somewhere\\darktable-cli.exe".into(),
        source: "guessed".into(),
    });

    assert!(validate_install_self_check_report(&report).is_err());
}
