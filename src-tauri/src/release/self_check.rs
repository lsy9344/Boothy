//! Story 7.7. `boothy.exe --self-check` — 설치본이 자기 인벤토리와 일치하는지 확인한다.
//!
//! **콘솔이 아니라 파일과 종료 코드로 말한다.** 릴리스 빌드는 `windows_subsystem = "windows"`라
//! stdout이 아무 데도 닿지 않는다. `AttachConsole`을 부르는 것은 새 Windows API 의존을
//! 들이는 일이라 이 Story의 원칙(의존성 동결)에 어긋난다.
//!
//! **창을 만들지 않는다.** clean offline VM 자동화에는 창을 닫을 방법이 없다. 그래서 이
//! 진입점은 Tauri 빌더를 세우기 **전에** 처리되고 그 자리에서 프로세스를 끝낸다.
//!
//! 종료 코드:
//!   `0` 통과 / `1` 인벤토리 불일치 / `2` 검사 불가 또는 보고서 기록 실패
//!
//! **`2`를 `1`과 합치지 않는다.** "검사에 실패했다"와 "검사를 못 했다"는 다른 사실이고,
//! 후자만이 설치된 페이로드에 대해 아무 말도 하지 않는다.
//! (Story 7.6이 `persist_present_record` 실패를 성공으로 반환하지 않게 고친 것과 같은 규칙.)

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};

use crate::{
    contracts::dto::{
        DarktableResolutionDto, InstallSelfCheckComponentDto, InstallSelfCheckReportDto,
        ReleaseInventoryDto, INSTALL_SELF_CHECK_SCHEMA_VERSION,
    },
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    release::{
        bundle_identity,
        inventory::{compute_component_digest, load_inventory, DigestError, InventoryLoadError},
    },
    render::{
        bundled_darktable_root, missing_bundled_darktable_entries, resolve_darktable_cli_binary,
    },
    session::{session_manifest::current_timestamp, session_repository::read_session_manifest},
};

pub const SELF_CHECK_FLAG: &str = "--self-check";
pub const SELF_CHECK_REPORT_FLAG: &str = "--report";
pub const SELF_CHECK_VERIFY_SESSION_FLAG: &str = "--verify-session";
pub const INVENTORY_FILE_NAME: &str = "inventory.json";
pub const HELPER_RELATIVE_PATH: &str = "sidecar/canon-helper/canon-helper.exe";
/// 빌드 스크립트와 staging이 남기는 결손 표시. 설치본 안에서 발견되면 그 자체가 사유다.
pub const NOT_STAGED_MARKER: &str = "PAYLOAD-NOT-STAGED.md";

pub const EXIT_PASS: i32 = 0;
pub const EXIT_INVENTORY_MISMATCH: i32 = 1;
pub const EXIT_CHECK_NOT_POSSIBLE: i32 = 2;

/// WebView2 Evergreen Runtime의 EdgeUpdate 클라이언트 GUID.
const WEBVIEW2_CLIENT_GUID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfCheckRequest {
    pub report_path: Option<PathBuf>,
    pub session_manifest_path: Option<PathBuf>,
}

/// `--self-check [--report <path>]` 인지 판별한다. 아니면 `None`이고 앱은 평소대로 뜬다.
pub fn parse_self_check_request(args: &[String]) -> Option<SelfCheckRequest> {
    let arguments = args.iter().skip(1).collect::<Vec<_>>();

    if !arguments
        .iter()
        .any(|value| value.as_str() == SELF_CHECK_FLAG)
    {
        return None;
    }

    let report_path = arguments
        .iter()
        .position(|value| value.as_str() == SELF_CHECK_REPORT_FLAG)
        .and_then(|index| arguments.get(index + 1))
        .filter(|value| !value.starts_with("--"))
        .map(|value| PathBuf::from(value.as_str()));

    let session_manifest_path = arguments
        .iter()
        .position(|value| value.as_str() == SELF_CHECK_VERIFY_SESSION_FLAG)
        .and_then(|index| arguments.get(index + 1))
        .filter(|value| !value.starts_with("--"))
        .map(|value| PathBuf::from(value.as_str()));

    Some(SelfCheckRequest {
        report_path,
        session_manifest_path,
    })
}

fn passing(name: &str) -> InstallSelfCheckComponentDto {
    InstallSelfCheckComponentDto {
        name: name.into(),
        expected_digest: None,
        actual_digest: None,
        status: "pass".into(),
        reason_code: None,
    }
}

fn failing(name: &str, reason_code: &str) -> InstallSelfCheckComponentDto {
    InstallSelfCheckComponentDto {
        name: name.into(),
        expected_digest: None,
        actual_digest: None,
        status: "fail".into(),
        reason_code: Some(reason_code.into()),
    }
}

fn skipped(name: &str) -> InstallSelfCheckComponentDto {
    InstallSelfCheckComponentDto {
        name: name.into(),
        expected_digest: None,
        actual_digest: None,
        status: "skipped".into(),
        reason_code: None,
    }
}

fn check_session_compatibility(manifest_path: &Path) -> InstallSelfCheckComponentDto {
    match read_session_manifest(manifest_path) {
        Ok(_) => passing("session-compatibility"),
        Err(_) => failing("session-compatibility", "session-manifest-unreadable"),
    }
}

/// 검사 결과 하나하나를 모은 것. **판정은 마지막에 한 번만 한다.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfCheckOutcome {
    pub report: InstallSelfCheckReportDto,
    pub exit_code: i32,
}

/// 인벤토리 매니페스트만 보고 구성요소 검사 결과를 만든다.
///
/// darktable / helper / WebView2 는 별도 함수가 담당한다. 여기서는 **인벤토리가 주장한 것과
/// 디스크가 가진 것**만 비교한다.
pub fn check_inventory_components(
    install_root: &Path,
    inventory_path: &Path,
) -> Vec<InstallSelfCheckComponentDto> {
    let inventory = match load_inventory(inventory_path) {
        Ok(inventory) => inventory,
        Err(InventoryLoadError::Unreadable) | Err(InventoryLoadError::Malformed) => {
            return vec![failing(
                "release-inventory",
                "inventory-manifest-unreadable",
            )];
        }
    };

    let staged = match inventory {
        ReleaseInventoryDto::NotStaged(_) => {
            return vec![failing("release-inventory", "inventory-not-staged")];
        }
        ReleaseInventoryDto::Staged(staged) => staged,
    };

    let mut components = vec![passing("release-inventory")];

    for component in &staged.components {
        // **인벤토리가 스스로 결손이라고 적은 항목은 통과가 아니다.**
        // 동봉하지 않기로 **결정한** 것(`embedded` / `not-applicable`)과
        // 있어야 하는데 **없는** 것(`missing`)은 다른 사실이다.
        if component.status == "missing" {
            components.push(failing(&component.role, "inventory-component-missing"));
            continue;
        }

        if component.status != "present" {
            // 동봉하지 않기로 한 것은 검사 대상이 아니다. **없다고 실패시키지 않는다** —
            // 그 판단은 명세와 게이트가 이미 내렸다.
            components.push(skipped(&component.role));
            continue;
        }

        let (Some(selection), Some(relative_path), Some(expected_digest)) = (
            component.entry_selection.as_ref(),
            component.install_relative_path.as_ref(),
            component.staged_tree_digest.as_ref(),
        ) else {
            components.push(failing(&component.role, "inventory-manifest-unreadable"));
            continue;
        };

        let root = relative_path
            .trim_end_matches('/')
            .split('/')
            .fold(install_root.to_path_buf(), |accumulated, segment| {
                accumulated.join(segment)
            });

        // **결손 표시가 설치본 안에 있으면 그것이 곧 사유다.**
        // 해시부터 재면 "digest-mismatch"로만 보이고, 운영자는 무엇을 해야 할지 알 수 없다.
        // `verify-inventory.ps1`이 staging 쪽에서 하는 판단과 같은 판단을 런타임도 한다.
        if root.join(NOT_STAGED_MARKER).is_file() {
            components.push(failing(&component.role, "inventory-component-missing"));
            continue;
        }

        match compute_component_digest(&root, selection) {
            Err(DigestError::Unreadable) => {
                components.push(failing(&component.role, "digest-tool-unavailable"));
            }
            Ok(None) => {
                components.push(failing(&component.role, "inventory-component-missing"));
            }
            Ok(Some((actual_digest, file_count))) => {
                let file_count_matches = component
                    .file_count
                    .map(|expected| expected as usize == file_count)
                    .unwrap_or(false);

                if &actual_digest == expected_digest && file_count_matches {
                    components.push(InstallSelfCheckComponentDto {
                        name: component.role.clone(),
                        expected_digest: Some(expected_digest.clone()),
                        actual_digest: Some(actual_digest),
                        status: "pass".into(),
                        reason_code: None,
                    });
                } else {
                    components.push(InstallSelfCheckComponentDto {
                        name: component.role.clone(),
                        expected_digest: Some(expected_digest.clone()),
                        actual_digest: Some(actual_digest),
                        status: "fail".into(),
                        reason_code: Some("inventory-digest-mismatch".into()),
                    });
                }
            }
        }
    }

    components
}

/// **`bundled-resource`가 아니면 그 회차는 핀이 깨진 회차다.**
///
/// 실행 파일만 있고 `lib/`나 `share/darktable/`이 없으면 렌더가 통째로 실패한다.
/// 그래서 해석 결과와 트리 무결성을 **둘 다** 본다.
pub fn check_darktable(
    resolution_source: &str,
    missing_tree_entries: &[&'static str],
) -> InstallSelfCheckComponentDto {
    if resolution_source != "bundled-resource" {
        return failing("darktable-resolution", "darktable-not-bundled");
    }

    if !missing_tree_entries.is_empty() {
        return failing("darktable-tree", "darktable-tree-incomplete");
    }

    passing("darktable-resolution")
}

pub fn check_helper(install_root: &Path) -> (InstallSelfCheckComponentDto, Option<String>) {
    let helper = HELPER_RELATIVE_PATH
        .split('/')
        .fold(install_root.to_path_buf(), |accumulated, segment| {
            accumulated.join(segment)
        });

    if !helper.is_file() {
        // **새 booth readiness 사유 코드를 만들지 않는다.** 카메라 helper 부재는 이미
        // 이 이름으로 표현된다.
        return (
            failing("camera-helper-binary", "helper-binary-missing"),
            None,
        );
    }

    let output = Command::new(&helper)
        .arg("--version")
        .stdin(Stdio::null())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string);

            (passing("camera-helper-binary"), version)
        }
        _ => (
            failing("camera-helper-binary", "helper-version-check-failed"),
            None,
        ),
    }
}

/// `reg.exe`의 `query` 출력에서 `pv` 값을 뽑는다.
pub fn parse_registry_version(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;

            if name != "pv" {
                return None;
            }

            let _kind = parts.next()?;
            let value = parts.next()?;

            Some(value.to_string())
        })
        .next()
}

/// 설치된 WebView2 런타임 버전.
///
/// **설치본으로 버전을 고정할 수 없다** (v2 스키마에 `fixedRuntime`이 없다). 그래서
/// 회차마다 실제 값을 읽어 기록한다. 읽지 못하면 그 사실도 기록한다.
pub fn read_webview2_version() -> Option<String> {
    let reg = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
        .join("System32")
        .join("reg.exe");

    if !reg.is_file() {
        return None;
    }

    let keys = [
        (
            format!("HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{WEBVIEW2_CLIENT_GUID}"),
            true,
        ),
        (
            format!("HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{WEBVIEW2_CLIENT_GUID}"),
            false,
        ),
        (
            format!("HKCU\\Software\\Microsoft\\EdgeUpdate\\Clients\\{WEBVIEW2_CLIENT_GUID}"),
            false,
        ),
    ];

    for (key, wow6432) in keys {
        let mut command = Command::new(&reg);
        command.arg("query").arg(&key).arg("/v").arg("pv");

        if wow6432 {
            command.arg("/reg:32");
        }

        if let Ok(output) = command.stdin(Stdio::null()).output() {
            if output.status.success() {
                if let Some(version) =
                    parse_registry_version(&String::from_utf8_lossy(&output.stdout))
                {
                    return Some(version);
                }
            }
        }
    }

    None
}

/// 검사 결과 목록에서 종료 코드를 정한다.
///
/// **"못 쟀다"가 "달랐다"를 이긴다.** 매니페스트를 못 읽었거나 해시 도구가 없으면 우리는
/// 설치된 페이로드에 대해 아무 말도 할 수 없고, 그 사실을 `1`로 적으면 거짓이 된다.
pub fn resolve_exit_code(components: &[InstallSelfCheckComponentDto]) -> i32 {
    let cannot_check = components.iter().any(|component| {
        matches!(
            component.reason_code.as_deref(),
            Some("digest-tool-unavailable") | Some("inventory-manifest-unreadable")
        )
    });

    if cannot_check {
        return EXIT_CHECK_NOT_POSSIBLE;
    }

    if components
        .iter()
        .any(|component| component.status == "fail")
    {
        return EXIT_INVENTORY_MISMATCH;
    }

    EXIT_PASS
}

pub fn install_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_report_path(identifier: &str) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(identifier)
        .join("diagnostics")
        .join("install-self-check.json")
}

/// 설치 상태를 한 번 훑어 보고서를 만든다. **파일을 쓰지 않는다** — 쓰는 것은 호출자의 몫이고,
/// 그래야 "검사 결과"와 "기록 실패"가 서로 다른 사실로 남는다.
pub fn evaluate_installation() -> SelfCheckOutcome {
    evaluate_installation_for_session(None)
}

fn evaluate_installation_for_session(session_manifest_path: Option<&Path>) -> SelfCheckOutcome {
    let identity = bundle_identity();
    let root = install_root();

    let mut components = check_inventory_components(&root, &root.join(INVENTORY_FILE_NAME));

    let resolution = resolve_darktable_cli_binary();
    let missing_tree_entries = bundled_darktable_root()
        .map(|tree| missing_bundled_darktable_entries(&tree))
        .unwrap_or_else(|| crate::render::BUNDLED_DARKTABLE_REQUIRED_ENTRIES.to_vec());
    components.push(check_darktable(resolution.source, &missing_tree_entries));

    let (helper_component, helper_version) = check_helper(&root);
    components.push(helper_component);

    let webview2_version = read_webview2_version();
    components.push(match webview2_version {
        Some(_) => passing("webview2-runtime"),
        None => failing("webview2-runtime", "webview2-runtime-unreadable"),
    });

    if let Some(manifest_path) = session_manifest_path {
        components.push(check_session_compatibility(manifest_path));
    }

    let exit_code = resolve_exit_code(&components);
    let overall = if components
        .iter()
        .any(|component| component.status == "fail")
    {
        "fail"
    } else {
        "pass"
    };

    SelfCheckOutcome {
        report: InstallSelfCheckReportDto {
            schema_version: INSTALL_SELF_CHECK_SCHEMA_VERSION.into(),
            checked_at: current_timestamp(SystemTime::now())
                .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into()),
            app_version: identity.app_version,
            identifier: identity.identifier,
            install_root: root.to_string_lossy().into_owned(),
            components,
            webview2_version,
            darktable_resolution: Some(DarktableResolutionDto {
                binary: resolution.binary,
                source: resolution.source.into(),
            }),
            helper_version,
            overall: overall.into(),
        },
        exit_code,
    }
}

pub fn write_report(path: &Path, report: &InstallSelfCheckReportDto) -> Result<(), ()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ())?;
    }

    let body = serde_json::to_string_pretty(report).map_err(|_| ())?;

    fs::write(path, format!("{body}\n")).map_err(|_| ())
}

/// `--self-check` 진입점. 종료 코드를 돌려주고, 호출자가 그대로 프로세스를 끝낸다.
pub fn run_self_check(request: &SelfCheckRequest) -> i32 {
    let outcome = evaluate_installation_for_session(request.session_manifest_path.as_deref());
    let report_path = request
        .report_path
        .clone()
        .unwrap_or_else(|| default_report_path(&outcome.report.identifier));

    match write_report(&report_path, &outcome.report) {
        // 보고서를 못 남기면 결과를 아무도 볼 수 없다. **통과를 통과로 보고할 수 없으면
        // 통과가 아니다.**
        Err(()) => EXIT_CHECK_NOT_POSSIBLE,
        Ok(()) => outcome.exit_code,
    }
}

// ---------------------------------------------------------------------------
// 정상 실행 경로의 투영
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGovernanceFinding {
    pub reason_code: String,
    pub detail: String,
}

/// 수동 `--self-check`와 **같은 결과**를 정상 시작의 운영자 진단으로 바꾼다.
/// 고객 흐름을 막지 않기 위해 실제 검사는 별도 스레드에서 실행한다.
pub fn evaluate_startup_release_governance(
    outcome: &SelfCheckOutcome,
) -> Option<ReleaseGovernanceFinding> {
    if outcome.report.overall == "pass" {
        return None;
    }

    let failures = outcome
        .report
        .components
        .iter()
        .filter(|component| component.status == "fail")
        .collect::<Vec<_>>();
    let reason_code = failures
        .first()
        .and_then(|component| component.reason_code.clone())
        .unwrap_or_else(|| "install-self-check-failed".into());
    let detail = if failures.is_empty() {
        "설치 self-check 전체 결과가 실패지만 구성요소 사유를 찾지 못했어요.".into()
    } else {
        failures
            .iter()
            .map(|component| {
                format!(
                    "{}={}",
                    component.name,
                    component.reason_code.as_deref().unwrap_or("unknown")
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    Some(ReleaseGovernanceFinding {
        reason_code,
        detail,
    })
}

/// 검사 결과를 **운영자 진단으로 투영한다.**
///
/// **고객 문구는 만들지 않는다.** 설치·인벤토리·서명은 고객이 알 필요가 없는 사실이다.
/// 세션 시작·촬영·렌더 경로를 막지도 않는다 — 기록만 남긴다.
pub fn run_startup_release_governance_check(base_dir: &Path) {
    let base_dir = base_dir.to_path_buf();

    let _ = std::thread::Builder::new()
        .name("boothy-release-governance".into())
        .spawn(move || {
            let outcome = evaluate_installation();
            let Some(finding) = evaluate_startup_release_governance(&outcome) else {
                return;
            };

            let Ok(occurred_at) = current_timestamp(SystemTime::now()) else {
                return;
            };

            try_append_operator_audit_record(
                &base_dir,
                OperatorAuditRecordInput {
                    occurred_at,
                    session_id: None,
                    event_category: "release-governance",
                    event_type: "install-inventory-mismatch",
                    summary: "설치본 구성요소가 인벤토리와 일치하지 않아요.".into(),
                    detail: finding.detail,
                    actor_id: None,
                    source: "release",
                    capture_id: None,
                    preset_id: None,
                    published_version: None,
                    reason_code: Some(finding.reason_code),
                },
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dto::{
        ComponentEntrySelectionDto, ReleaseInventoryComponentDto, StagedReleaseInventoryDto,
        RELEASE_INVENTORY_SCHEMA_VERSION,
    };

    fn args(values: &[&str]) -> Vec<String> {
        std::iter::once("boothy.exe")
            .chain(values.iter().copied())
            .map(str::to_string)
            .collect()
    }

    fn startup_outcome(components: Vec<InstallSelfCheckComponentDto>) -> SelfCheckOutcome {
        let overall = if components
            .iter()
            .any(|component| component.status == "fail")
        {
            "fail"
        } else {
            "pass"
        };

        SelfCheckOutcome {
            report: InstallSelfCheckReportDto {
                schema_version: INSTALL_SELF_CHECK_SCHEMA_VERSION.into(),
                checked_at: "2026-08-17T05:00:00Z".into(),
                app_version: "0.1.0".into(),
                identifier: "com.boothy.booth".into(),
                install_root: "C:/Program Files/Boothy".into(),
                components,
                webview2_version: Some("141.0.3537.85".into()),
                darktable_resolution: Some(DarktableResolutionDto {
                    binary: "C:/Program Files/Boothy/darktable/bin/darktable-cli.exe".into(),
                    source: "bundled-resource".into(),
                }),
                helper_version: Some("canon-helper 0.1.0".into()),
                overall: overall.into(),
            },
            exit_code: if overall == "pass" {
                EXIT_PASS
            } else {
                EXIT_INVENTORY_MISMATCH
            },
        }
    }

    #[test]
    fn a_normal_launch_is_not_a_self_check() {
        assert_eq!(parse_self_check_request(&args(&[])), None);
        assert_eq!(parse_self_check_request(&args(&["--other"])), None);
    }

    #[test]
    fn the_self_check_flag_is_recognized_with_and_without_a_report_path() {
        assert_eq!(
            parse_self_check_request(&args(&["--self-check"])),
            Some(SelfCheckRequest {
                report_path: None,
                session_manifest_path: None,
            })
        );
        assert_eq!(
            parse_self_check_request(&args(&["--self-check", "--report", "C:/out/report.json"])),
            Some(SelfCheckRequest {
                report_path: Some(PathBuf::from("C:/out/report.json")),
                session_manifest_path: None,
            })
        );
    }

    #[test]
    fn the_self_check_can_verify_a_prior_session_manifest() {
        assert_eq!(
            parse_self_check_request(&args(&[
                "--self-check",
                "--verify-session",
                "C:/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/session.json",
                "--report",
                "C:/out/after-upgrade.json",
            ])),
            Some(SelfCheckRequest {
                report_path: Some(PathBuf::from("C:/out/after-upgrade.json")),
                session_manifest_path: Some(PathBuf::from(
                    "C:/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/session.json"
                )),
            })
        );
    }

    #[test]
    fn prior_session_compatibility_uses_the_product_manifest_reader() {
        let root = std::env::temp_dir().join(format!(
            "boothy-session-compatibility-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create session compatibility root");
        let manifest_path = root.join("session.json");
        let manifest = crate::session::session_manifest::build_session_manifest(
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            crate::contracts::dto::SessionStartInputDto {
                name: "Upgrade Test".into(),
                phone_last_four: "1234".into(),
            },
        )
        .expect("build current session manifest");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");

        assert_eq!(
            check_session_compatibility(&manifest_path),
            passing("session-compatibility")
        );

        fs::write(&manifest_path, b"{broken").expect("replace with malformed manifest");
        assert_eq!(
            check_session_compatibility(&manifest_path),
            failing("session-compatibility", "session-manifest-unreadable")
        );
        let _ = fs::remove_dir_all(root);
    }

    /// `--report` 만 있고 값이 없으면 기본 경로를 쓴다. 인자 하나 때문에 검사를 포기하지 않는다.
    #[test]
    fn a_dangling_report_flag_falls_back_to_the_default_path() {
        assert_eq!(
            parse_self_check_request(&args(&["--self-check", "--report"])),
            Some(SelfCheckRequest {
                report_path: None,
                session_manifest_path: None,
            })
        );
        assert_eq!(
            parse_self_check_request(&args(&["--self-check", "--report", "--other"])),
            Some(SelfCheckRequest {
                report_path: None,
                session_manifest_path: None,
            })
        );
    }

    #[test]
    fn a_clean_result_exits_zero() {
        assert_eq!(
            resolve_exit_code(&[passing("release-inventory"), skipped("app")]),
            EXIT_PASS
        );
    }

    /// **`1`은 "설치본이 자기 주장과 다르다"는 뜻이다.**
    #[test]
    fn a_mismatch_exits_one() {
        assert_eq!(
            resolve_exit_code(&[
                passing("release-inventory"),
                failing("raw-renderer", "inventory-digest-mismatch"),
            ]),
            EXIT_INVENTORY_MISMATCH
        );
        assert_eq!(
            resolve_exit_code(&[failing("release-inventory", "inventory-not-staged")]),
            EXIT_INVENTORY_MISMATCH
        );
    }

    /// **`2`는 "검사를 못 했다"는 뜻이고 `1`과 합치지 않는다.**
    /// 해시 도구가 없으면 우리는 설치된 페이로드에 대해 아무 말도 할 수 없다.
    #[test]
    fn an_unmeasurable_result_exits_two_and_outranks_a_mismatch() {
        assert_eq!(
            resolve_exit_code(&[failing("raw-renderer", "digest-tool-unavailable")]),
            EXIT_CHECK_NOT_POSSIBLE
        );
        assert_eq!(
            resolve_exit_code(&[failing(
                "release-inventory",
                "inventory-manifest-unreadable"
            )]),
            EXIT_CHECK_NOT_POSSIBLE
        );
        assert_eq!(
            resolve_exit_code(&[
                failing("raw-renderer", "inventory-digest-mismatch"),
                failing("camera-helper", "digest-tool-unavailable"),
            ]),
            EXIT_CHECK_NOT_POSSIBLE
        );
    }

    /// **인벤토리가 스스로 결손이라고 적은 항목을 통과로 넘기지 않는다.**
    ///
    /// "동봉하지 않기로 결정했다"와 "있어야 하는데 없다"는 다른 사실이고,
    /// 후자를 `skipped`로 적으면 결손 인벤토리를 담은 설치본이 self-check를 통과한다.
    #[test]
    fn a_component_the_inventory_calls_missing_is_a_failure_not_a_skip() {
        let root = std::env::temp_dir().join(format!("boothy-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("디렉터리를 만들 수 있어야 한다");

        let inventory_path = root.join("inventory.json");
        fs::write(
            &inventory_path,
            serde_json::to_string(&complete_inventory("missing", "darktable/"))
                .expect("인벤토리는 직렬화된다"),
        )
        .expect("인벤토리를 쓸 수 있어야 한다");

        let components = check_inventory_components(&root, &inventory_path);
        let raw_renderer = components
            .iter()
            .find(|component| component.name == "raw-renderer")
            .expect("raw-renderer 항목이 있어야 한다");

        assert_eq!(raw_renderer.status, "fail");
        assert_eq!(
            raw_renderer.reason_code.as_deref(),
            Some("inventory-component-missing")
        );
        assert_eq!(resolve_exit_code(&components), EXIT_INVENTORY_MISMATCH);

        let _ = fs::remove_dir_all(root);
    }

    /// **동봉하지 않기로 결정한 것은 검사 대상이 아니다.** 없다고 실패시키지 않는다.
    #[test]
    fn a_component_that_was_deliberately_not_bundled_is_skipped() {
        let root = std::env::temp_dir().join(format!("boothy-declared-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("디렉터리를 만들 수 있어야 한다");

        let inventory_path = root.join("inventory.json");
        fs::write(
            &inventory_path,
            serde_json::to_string(&complete_inventory("not-applicable", "darktable/"))
                .expect("인벤토리는 직렬화된다"),
        )
        .expect("인벤토리를 쓸 수 있어야 한다");

        let components = check_inventory_components(&root, &inventory_path);

        assert!(components
            .iter()
            .all(|component| component.status != "fail"));
        assert_eq!(resolve_exit_code(&components), EXIT_PASS);

        let _ = fs::remove_dir_all(root);
    }

    /// **결손 표시가 설치본에 실려 오면 그것이 사유다.**
    ///
    /// 해시부터 재면 "달랐다"로만 보인다. 운영자에게 필요한 사실은 "페이로드가 조달되지 않았다"이다.
    #[test]
    fn a_not_staged_marker_inside_the_install_tree_is_reported_as_a_missing_component() {
        let root = std::env::temp_dir().join(format!("boothy-marker-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let component_root = root.join("darktable");
        fs::create_dir_all(&component_root).expect("디렉터리를 만들 수 있어야 한다");
        fs::write(component_root.join("payload.bin"), "payload").expect("파일을 쓸 수 있어야 한다");
        fs::write(component_root.join(NOT_STAGED_MARKER), "# 미조달")
            .expect("표시 파일을 쓸 수 있어야 한다");

        let inventory_path = root.join("inventory.json");
        fs::write(
            &inventory_path,
            serde_json::to_string(&complete_inventory("present", "darktable/"))
                .expect("인벤토리는 직렬화된다"),
        )
        .expect("인벤토리를 쓸 수 있어야 한다");

        let components = check_inventory_components(&root, &inventory_path);
        let raw_renderer = components
            .iter()
            .find(|component| component.name == "raw-renderer")
            .expect("raw-renderer 항목이 있어야 한다");

        assert_eq!(raw_renderer.status, "fail");
        assert_eq!(
            raw_renderer.reason_code.as_deref(),
            Some("inventory-component-missing"),
            "결손 표시를 digest-mismatch 로 뭉뚱그리지 않는다"
        );
        assert_eq!(resolve_exit_code(&components), EXIT_INVENTORY_MISMATCH);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_missing_manifest_is_reported_as_unmeasurable() {
        let components = check_inventory_components(
            Path::new("C:/definitely/not/here"),
            Path::new("C:/definitely/not/here/inventory.json"),
        );

        assert_eq!(components.len(), 1);
        assert_eq!(
            components[0].reason_code.as_deref(),
            Some("inventory-manifest-unreadable")
        );
        assert_eq!(resolve_exit_code(&components), EXIT_CHECK_NOT_POSSIBLE);
    }

    /// **핀이 깨진 회차를 통과시키지 않는다.**
    #[test]
    fn a_darktable_resolved_outside_the_bundle_fails() {
        assert_eq!(
            check_darktable("program-files-bin", &[])
                .reason_code
                .as_deref(),
            Some("darktable-not-bundled")
        );
        assert_eq!(
            check_darktable("path", &[]).reason_code.as_deref(),
            Some("darktable-not-bundled")
        );
        assert_eq!(check_darktable("bundled-resource", &[]).status, "pass");
    }

    /// 실행 파일만 있고 데이터가 없으면 렌더가 실패한다. 통과시키면 안 된다.
    #[test]
    fn a_bundled_darktable_missing_its_data_directories_fails() {
        let component = check_darktable("bundled-resource", &["lib", "share/darktable"]);

        assert_eq!(component.status, "fail");
        assert_eq!(
            component.reason_code.as_deref(),
            Some("darktable-tree-incomplete")
        );
    }

    #[test]
    fn a_missing_helper_reuses_the_existing_readiness_reason_code() {
        let (component, version) = check_helper(Path::new("C:/definitely/not/here"));

        assert_eq!(
            component.reason_code.as_deref(),
            Some("helper-binary-missing")
        );
        assert_eq!(version, None);
    }

    #[test]
    fn the_registry_version_is_read_from_a_reg_query_result() {
        let stdout = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\r\n    pv    REG_SZ    141.0.3537.85\r\n\r\n";

        assert_eq!(
            parse_registry_version(stdout).as_deref(),
            Some("141.0.3537.85")
        );
        assert_eq!(
            parse_registry_version("ERROR: 지정된 레지스트리 키가 없습니다."),
            None
        );
    }

    fn staged_inventory(status: &str, install_relative_path: &str) -> ReleaseInventoryDto {
        ReleaseInventoryDto::Staged(StagedReleaseInventoryDto {
            schema_version: RELEASE_INVENTORY_SCHEMA_VERSION.into(),
            generated_at: "2026-08-17T05:00:00Z".into(),
            app_version: "0.1.0".into(),
            identifier: "com.boothy.booth".into(),
            installer_file_name: "Boothy_0.1.0_x64-setup.exe".into(),
            signing_status: "unsigned".into(),
            installer: None,
            components: vec![ReleaseInventoryComponentDto {
                name: "darktable".into(),
                role: "raw-renderer".into(),
                status: status.into(),
                version: Some("5.4.1".into()),
                origin: "darktable release-5.4.1".into(),
                entry_selection: if status == "present" {
                    Some(ComponentEntrySelectionDto::All)
                } else {
                    None
                },
                license: "GPL-3.0-or-later".into(),
                license_evidence_path: Some("release/licenses/darktable-5.4.1.md".into()),
                source_archive_sha256: None,
                staged_tree_digest: if status == "present" {
                    Some("b".repeat(64))
                } else {
                    None
                },
                install_relative_path: Some(install_relative_path.into()),
                signing_status: "not-applicable".into(),
                rationale: if status == "present" {
                    None
                } else {
                    Some("페이로드가 조달되지 않았다.".into())
                },
                file_count: if status == "present" { Some(1) } else { None },
                total_bytes: if status == "present" { Some(3) } else { None },
            }],
        })
    }

    /// `parse_inventory`는 계약 검증을 통과해야 하므로 아홉 역할을 모두 채운 문서를 만든다.
    /// `raw-renderer`만 인자대로 바꾸고 나머지는 `not-applicable`로 채운다.
    fn complete_inventory(status: &str, install_relative_path: &str) -> ReleaseInventoryDto {
        let mut inventory = staged_inventory(status, install_relative_path);

        if let ReleaseInventoryDto::Staged(staged) = &mut inventory {
            for role in crate::contracts::dto::RELEASE_INVENTORY_ROLES {
                if role == "raw-renderer" {
                    continue;
                }

                staged.components.push(ReleaseInventoryComponentDto {
                    name: role.into(),
                    role: role.into(),
                    status: "not-applicable".into(),
                    version: None,
                    origin: "test".into(),
                    entry_selection: None,
                    license: "not-applicable".into(),
                    license_evidence_path: None,
                    source_archive_sha256: None,
                    staged_tree_digest: None,
                    install_relative_path: None,
                    signing_status: "not-applicable".into(),
                    rationale: Some("테스트 fixture".into()),
                    file_count: None,
                    total_bytes: None,
                });
            }
        }

        inventory
    }

    #[test]
    fn a_not_staged_build_is_projected_into_the_operator_audit_log() {
        let outcome = startup_outcome(vec![failing("release-inventory", "inventory-not-staged")]);
        let finding = evaluate_startup_release_governance(&outcome)
            .expect("not-staged 결과도 정상 시작에서 운영자에게 보여야 한다");

        assert_eq!(finding.reason_code, "inventory-not-staged");
    }

    #[test]
    fn a_component_recorded_as_missing_is_projected_with_its_reason_code() {
        let outcome = startup_outcome(vec![failing("raw-renderer", "inventory-component-missing")]);
        let finding =
            evaluate_startup_release_governance(&outcome).expect("결손은 투영되어야 한다");

        assert_eq!(finding.reason_code, "inventory-component-missing");
        assert!(finding.detail.contains("raw-renderer"));
    }

    #[test]
    fn a_digest_mismatch_is_projected() {
        let outcome = startup_outcome(vec![failing("camera-helper", "inventory-digest-mismatch")]);
        let finding =
            evaluate_startup_release_governance(&outcome).expect("digest 불일치는 투영되어야 한다");

        assert_eq!(finding.reason_code, "inventory-digest-mismatch");
    }

    #[test]
    fn a_passing_self_check_is_not_projected() {
        let outcome = startup_outcome(vec![passing("release-inventory")]);

        assert_eq!(evaluate_startup_release_governance(&outcome), None);
    }

    /// 보고서가 계약을 만족하는지 확인한다. 계약을 어긴 보고서는 게이트가 읽지 못한다.
    #[test]
    fn the_written_report_satisfies_the_contract() {
        let outcome = evaluate_installation();

        assert!(
            crate::contracts::dto::validate_install_self_check_report(&outcome.report).is_ok(),
            "self-check 보고서가 계약을 어겼다: {:?}",
            outcome.report
        );
        assert!(matches!(
            outcome.exit_code,
            EXIT_PASS | EXIT_INVENTORY_MISMATCH | EXIT_CHECK_NOT_POSSIBLE
        ));
    }

    #[test]
    fn the_default_report_path_lives_under_the_bundle_identifier() {
        let path = default_report_path("com.boothy.booth");

        assert!(path.ends_with("com.boothy.booth/diagnostics/install-self-check.json"));
    }
}
