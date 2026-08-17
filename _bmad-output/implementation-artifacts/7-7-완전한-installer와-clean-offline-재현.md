# Story 7.7: 완전한 installer와 현재 PC lifecycle 재현

Status: done

Type: Packaging / Release Enablement (고객 화면 동작 변경 없음)

> **2026-08-17 승인된 범위 변경 — 이 문서의 이전 clean/offline·서명 필수 문구보다 우선한다.**
> Noah Lee의 지시에 따라 MVP 내부 검증은 현재 승인된 PC에서 수행한다. 코드 서명 인증서,
> Canon EDSDK 재배포 권리 증거, clean Windows 및 네트워크 차단은 `waived-by-owner`다.
> 면제는 통과 증거가 아니므로 공개 배포 적합성·법적 재배포 권리·새 PC 독립 설치를 주장하지 않는다.
> 면제되지 않은 inventory integrity, 전체 lifecycle, 데이터 보존, 실제 카메라 pipeline,
> Story 7.6/HV-17 선행 조건은 그대로 필수다.

Epic 7 dependency: `7.1 Go` → `7.2 Go` → `7.3 route decision` → `7.4 Go` → `7.5 adoption decision` → `7.6 Go` → **`7.7 Go`** → `7.8` → `7.9` → `7.10 final decision`

> ## 착수 전 반드시 읽을 것 — 상류가 남긴 사실
>
> - **Story 7.6은 아직 `in-progress`이고 HV-17은 `No-Go`다 (2026-08-17 기준).**
>   Epic 7 규칙상 `7.6 Go`가 `7.7`의 선행 조건이고, ledger의 Story 7.7 rerun prerequisite도
>   `HV-17 Go and one approved production route with exact fallback`이다.
>   **따라서 이 Story의 코드·패키징 작업은 지금 진행할 수 있지만, HV-18A 실장비 회차는
>   Story 7.6이 닫힌 빌드를 release candidate로 삼아 실행해야 한다.** 7.6 이전 빌드로 얻은
>   설치 증거는 HV-18A로 인정하지 않는다 — 인벤토리에 들어가는 실행 파일이 달라지기 때문이다.
> - **승인된 production route는 `raw-original + pinned darktable 5.4.1`이다** (2026-08-16, Noah Lee).
>   HV-14의 fast source 후보 3종과 HV-16의 상주 renderer 후보는 전부 `Technology No-Go`로 비활성이다.
>   **이 Story는 그 판정을 뒤집지 않는다.** 인벤토리는 살아 있는 경로만 담는다.
> - **HV-16이 판정을 이 Story로 넘긴 항목이 하나 있다.** Windows가 `Microsoft.RawImageExtension`
>   2.5.24.0으로 CR2를 in-process 디코드할 수 있다는 사실을 HV-16이 발견했지만, 그것은 Microsoft Store
>   appx이고 **오프라인 부스 installer에 동봉하는 경로가 확인되지 않았다.**
>   이 Story는 **"오프라인 배포가 가능한가"만 판정해 기록한다.** 채택은 별도 승인 안건이다.
> - **서명 인증서는 2026-08-17 책임자 승인으로 MVP 내부 검증에서 면제되었다.** 설치본은
>   `unsigned`를 정직하게 기록한다. 기계식 게이트는 `Go-candidate-with-waivers`, 책임자가 승인한
>   현재-PC 내부 배포 판정은 `Go-with-waivers`이며 공개 배포 적합성을 뜻하지 않는다.

---

## 이 Story의 진짜 발견 — 오늘의 빌드는 clean offline에서 동작하지 않는다

착수 전에 실제 설정 파일과 경로 해석 코드를 전부 읽었다. **오늘 `pnpm release:desktop`이 만드는
NSIS 설치본은 AC 2의 clean offline VM에서 부팅조차 하지 못한다.** 결손은 추정이 아니라 확인된 사실이다.

| # | 결손 | 근거 (실제 코드/설정) | clean offline VM에서 벌어지는 일 |
| --- | --- | --- | --- |
| 1 | **WebView2 런타임이 동봉되지 않는다** | `tauri.conf.json`에 `bundle.windows`가 아예 없다 → `webviewInstallMode` 기본값은 `downloadBootstrapper`(인터넷 필요). `README.md:58`은 운영자에게 수동 설치를 지시한다 | 설치는 되지만 **앱 창이 뜨지 않는다.** 부트스트래퍼가 다운로드를 시도하고 실패한다 |
| 2 | **카메라 helper가 동봉되지 않는다** | `bundle.resources`에 fixture JPEG 2장만 있다. `sidecar/canon-helper`는 어디에도 없다 | `resolve_helper_launch_target()`이 전부 실패 → `helper-binary-missing`. 촬영 불가 |
| 3 | **helper가 self-contained가 아니다** | `CanonHelper.csproj`는 `net8.0` framework-dependent. `--self-contained`/`-r win-x64` 설정 없음 | .NET 8 런타임 없는 VM에서 helper가 실행되지 않는다. AC 1이 "self-contained camera helper"를 명시한다 |
| 4 | **pinned darktable이 동봉되지 않는다** | `render/mod.rs:1721 darktable_cli_binary_candidates()`가 `ProgramFiles\darktable\bin\`, `ProgramW6432\...`, `LOCALAPPDATA\Programs\...`만 본다. 전부 실패하면 `"darktable-cli"`를 **PATH에 있다고 가정하고** 반환한다 | 고객 화면 렌더가 통째로 실패한다. **AC 2가 "separately installed renderer 없이"를 명시하므로 별도 설치는 답이 아니다** |
| 5 | **EDSDK 페이로드가 저장소에 없다** | `.gitignore:` `sidecar/canon-helper/vendor/canon-edsdk/`. 저장소에는 `vendor/README.md`만 있다 | CI가 helper를 **빌드조차 할 수 없다.** `ValidateCanonSdkInputs` 타깃이 에러로 중단시킨다 |
| 6 | **CI가 helper를 빌드하지 않는다** | `.github/workflows/release-windows.yml`은 `pnpm build:desktop` / `pnpm release:desktop`만 실행한다. `dotnet` 단계가 없다 | 인벤토리에 helper가 들어갈 수 없다 |
| 7 | **identifier가 스타터 기본값이다** | `tauri.conf.json: "identifier": "com.tauri.dev"` | `%LOCALAPPDATA%\com.tauri.dev\`가 앱 데이터 루트가 된다. **첫 release candidate 이후에 바꾸면 업그레이드 경로와 WebView2 user data가 끊긴다.** 지금이 바꿀 수 있는 마지막 시점이다 |
| 8 | **`targets: "all"`이 MSI와 NSIS를 둘 다 만든다** | `tauri.conf.json: "bundle.targets": "all"` | 서명·해시·업그레이드를 검증해야 할 산출물이 두 벌이 된다. `release-baseline.md`는 NSIS 하나만 이야기한다 |
| 9 | **release baseline 문서가 두 벌로 갈라져 있고, 루트 사본이 거짓을 담고 있다** | `release-baseline.md`(루트)와 `docs/release-baseline.md`가 **서로 다르다.** `README.md:17`이 가리키는 정본은 `docs/` 쪽이고 그 문서는 정직하다("the current workflow does not yet enforce an automated identity assertion"). **루트 사본은 "CI identity check: the workflow verifies that the produced installer matches `Boothy_${version}_x64-setup.exe`"라고 적고 있는데 워크플로에 그런 단계가 없다.** 두 사본 모두 `createUpdaterArtifacts: false`를 "유지한다"고 적지만 **설정에 그 키가 없다**(기본값이 `false`라 결과는 같으나 명시되지 않았다) | 계약 정의가 두 벌이면 안 된다는 아키텍처 규칙 위반이고, 낡은 쪽이 증거로 인용될 수 있다. 이 Story가 정본을 하나로 만들고 코드와 일치시킨다 |

**이 아홉 가지를 닫는 것이 이 Story의 실체다.** 새 고객 기능은 하나도 없다.

---

## 승인이 필요한 결정 (기본안이 있으므로 dev agent는 멈추지 않는다)

아래 네 항목은 **기본안대로 진행한다.** 승인자가 다르게 결정하면 그때 되돌린다.
기본안 없이 물어보느라 착수를 미루지 않는다.

1. **darktable 동봉 방식 — 기본안: 설치 트리를 앱 resource로 동봉한다.**
   darktable 자체 installer를 체이닝하지 않는다. 근거는 AC 2의 "without a separately installed renderer"와
   **핀 고정**이다. 별도 설치된 darktable은 운영자가 업그레이드할 수 있고, 그 순간 고객 화면의 픽셀이 바뀐다.
   부작용: 설치본 용량이 크게 늘어난다(추정 200~400 MB). **실측값을 T8에 기록한다.**
2. **WebView2 — 기본안: `webviewInstallMode: { "type": "offlineInstaller" }`.**
   v2 스키마가 제공하는 변형은 `skip` / `downloadBootstrapper` / `embedBootstrapper` / `offlineInstaller`
   네 가지뿐이고, 오프라인에서 성립하는 것은 마지막 하나다. **`fixedRuntime`은 v2 스키마에 없다.**
   그래서 런타임 버전을 설치본으로 고정할 수 없다 → **HV 회차마다 실제 WebView2 버전을 기록한다.**
3. **NSIS 설치 모드 — 기본안: `perMachine`.**
   부스 PC는 공용 장비이고 darktable 트리를 담아야 한다. 대가는 설치 시 UAC 승격 1회다.
   세션 데이터는 그대로 `%USERPROFILE%\Pictures\dabi_shoot`에 쓰므로 쓰기 권한 문제는 없다.
4. **서명 — 기본안: 서명 파이프라인을 완성해 두고, 인증서 조달은 별도 안건으로 올린다.**
   `signCommand` 또는 `certificateThumbprint` + `timestampUrl` 경로를 설정·문서·CI에 완성하고,
   인증서가 없는 동안에는 **"미서명"을 인벤토리에 정직하게 기록한다.**
   미서명 설치본을 서명 통과로 적지 않는다. 2026-08-17 책임자가 내부 검증에서 명시적으로 면제했다.

---

## Story

부스 소유자/브랜드 운영자로서,
**현재 승인된 시험 PC에서 설치 파일 하나로 전체 관람 경로와 설치 lifecycle을 재현하기를** 원한다.
그래야 지점을 늘릴 때마다 "이 PC에서만 왜 안 되지"를 겪지 않고, 성능·배포 승인을 그 위에서 시작할 수 있다.

---

## Acceptance Criteria

1. **Given** 승인된 release candidate가 있을 때, **when** 그 인벤토리를 검사하면,
   **then** 앱, **self-contained 카메라 helper**, 승인된 EDSDK 런타임, 선택된 source adapter,
   display renderer 또는 shader 번들, color profile, proxy recipe, **pinned darktable 의존성**이
   **정확한 버전과 해시와 함께** 포함되어야 하고, **integrity 증거가 첨부**되어야 한다.
   signing과 Canon EDSDK redistribution 증거는 책임자 승인 면제로 별도 기록한다.
2. **Given** 책임자가 승인한 **현재 Windows 시험 PC**에서,
   **when** 설치 → 실행 → self-check → fixture 표시 → 업그레이드 → 롤백 → 제거를 수행하면,
   **then** viewer/camera/proxy/RAW/final 런타임 전체가 재현되어야 하고,
   **누락되거나 불일치하는 인벤토리는 release를 막고 운영자가 조치할 수 있는 결과를 남겨야 한다.**
3. **Given** 구현과 자동 packaging 테스트가 끝났을 때, **when** Story 7.7을 종료 심사하면,
   **then** HV-18A가 인벤토리 무결성, 현재 PC 설치 lifecycle, 완전한 런타임 재현으로
   기계식 `Go-candidate-with-waivers`와 책임자의 범위 승인(`Go-with-waivers`)이 함께 기록될 때까지
   `review`를 유지해야 한다.

### 이 Story가 스스로에게 추가로 부과하는 판정 (AC 1의 "정확한 버전과 해시"를 기계화한다)

4. **Given** 인벤토리 명세(`release/inventory-spec.json`)와 실제 staged 페이로드가 있을 때,
   **when** 검증 스크립트를 실행하면, **then** 명세에 있는데 없는 구성요소, 해시가 다른 구성요소,
   명세에 없는데 들어온 구성요소가 **각각 고유한 사유 코드로** 실패해야 하고,
   **한 건이라도 있으면 릴리스 산출물이 만들어지지 않아야 한다.**
5. **Given** 설치된 앱이 시작될 때, **when** self-check가 실행되면,
   **then** 동봉된 인벤토리 매니페스트와 디스크의 실제 파일을 대조한 `install-self-check/v1` 보고서를
   남기고 종료 코드로 결과를 알려야 하며, **실패 항목은 운영자 진단 경로로 투영되어야 한다.**
   **고객 화면에는 새 문구를 만들지 않는다.**

---

## Scope Boundary

### In Scope

- `tauri.conf.json` 번들 설정: `identifier`, `targets`, `bundle.windows`(WebView2·NSIS·서명),
  `resources`(helper 트리, darktable 트리, 인벤토리 매니페스트), `createUpdaterArtifacts` 명시
- helper self-contained publish 파이프라인 (`-r win-x64 --self-contained true`)과 EDSDK 페이로드 조달 경로
- pinned darktable 5.4.1 트리 조달·staging·동봉과 **번들 경로를 최우선으로 하는 바이너리 해석 순서 변경**
- `release-inventory/v1` 계약, 생성기, 기계식 검증 스크립트
- `boothy.exe --self-check` 진입점과 `install-self-check/v1` 보고서
- 인벤토리 실패의 **운영자 진단 투영** (기존 diagnostics/audit 경로 재사용)
- clean offline VM 절차 문서와 HV-18A evidence 게이트 스크립트
- CI 워크플로: helper 빌드 → darktable staging → 인벤토리 생성·검증 → 서명 → 산출물 이름 검증
- `Microsoft.RawImageExtension` 오프라인 배포 가능 여부 **판정과 기록** (채택 아님)
- 영향 문서 갱신: `release-baseline.md`, `README.md`, `architecture.md` 디렉터리 구조/배포 절,
  `docs/contracts/release-inventory.md`, ledger Story 7.7 행

### Explicitly Out of Scope

- **고객 화면 동작·문구·타이밍 변경 일체.** 이 Story는 픽셀을 바꾸지 않는다
- **`BOOTHY_DISPLAY_PROXY_MODE`, `BOOTHY_RAW_REFINED_MODE`, `BOOTHY_RESIDENT_RENDERER_MODE`,
  `BOOTHY_DISPLAY_SAMPLE_MODE`, `BOOTHY_SOURCE_COMPARE_MODE` 기본값 변경**
- **HV-14 fast source 후보 / HV-16 상주 renderer 후보의 재승인 또는 활성화**
- **`Microsoft.RawImageExtension` 채택** — 이 Story는 배포 가능성만 판정한다
- 100-shot 성능·cold/idle/reconnect 회차 → Story 7.8
- **120fps+ 물리 monitor frame과 compositor→photon 오프셋** → HV-18B 단독 소유
- staged rollout / rollback **운영 검증** → Story 7.9.
  이 Story는 업그레이드·롤백·제거의 **설치 lifecycle**만 증명한다. 지점 승급 단계는 다루지 않는다
- 자동 업데이터 활성화. `createUpdaterArtifacts`는 **명시적으로 `false`**로 고정한다
- `session.json` 스키마 변경, display 계약(`viewer-display/v4`) 변경, 렌더 인자 변경
- Story 7.6이 만든 scheduler·취소·tier 로직에 대한 수정

---

## Dev Notes

### 코드베이스 현실 — 착수 전 반드시 인지할 것

| 항목 | 현재 상태 |
| --- | --- |
| 번들 설정 | `src-tauri/tauri.conf.json`. `bundle.windows` **없음**, `targets: "all"`, `resources`는 fixture JPEG 2장, `identifier: "com.tauri.dev"`, `version: "0.1.0"` |
| 릴리스 스크립트 | `package.json`: `build:desktop`(= `tauri build --debug`), `release:desktop`(= `tauri build`) |
| CI | `.github/workflows/release-windows.yml` 단 하나. Node 22 + pnpm 10 + rust stable. **dotnet 단계 없음, 서명 단계 없음, 산출물 검증 없음** |
| helper 해석 | `capture/helper_supervisor.rs:98 resolve_helper_launch_target()`. 순서: `BOOTHY_CANON_HELPER_EXE` → (debug일 때) dotnet 프로젝트 → `<repo>/sidecar/canon-helper/canon-helper.exe` → publish 경로 → **`current_exe/sidecar/canon-helper/canon-helper.exe`** |
| helper 해석의 함정 | `resolve_helper_dir()`가 `env!("CARGO_MANIFEST_DIR")`를 쓴다 — **빌드 머신의 절대 경로가 바이너리에 박힌다.** clean VM에서는 존재하지 않으므로 `current_exe` 후보만 남는다. **그 후보가 이 Story의 착지점이다** |
| helper 프로젝트 | `sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`. `net8.0`, `AssemblyName=canon-helper`, `Version=0.1.0`, `System.Drawing.Common 8.0.0`. EDSDK `.cs` interop을 **소스로 컴파일**하고 `EDSDK_64/Dll/**`를 출력에 복사한다. `ValidateCanonSdkInputs` 타깃이 SDK 없으면 **빌드를 에러로 중단**시킨다 |
| helper CLI | `--version`, `--self-check --sdk-root <path>`, `--runtime-root <dir> --session-id <id>` (`CanonHelperOptions.cs`) |
| EDSDK 페이로드 | `.gitignore`로 제외됨. `sidecar/canon-helper/vendor/README.md`가 출처를 기록: **EDSDK 13.19.0 / 2025-02-28 / zip SHA256 `C4F15E9D…B3AA699D`** |
| darktable 해석 | `render/mod.rs:1686 resolve_darktable_cli_binary()`. `BOOTHY_DARKTABLE_CLI_BIN` → `ProgramFiles` → `ProgramW6432` → `LOCALAPPDATA\Programs` → **폴백 `"darktable-cli"` (PATH 가정)** |
| darktable 핀 | 코드 곳곳에 문자열 `"5.4.1"`이 박혀 있다 (`contracts/dto.rs`, `display/proxy_publisher.rs:578 PINNED`, `raw_refined_publisher.rs:550`). ledger의 darktable pin은 `release-5.4.1 / c3f96ca` |
| 렌더 인자 | `--upscale false`, `--hq false`(proxy) / `true`(refined), `--icc-type SRGB`, `--icc-intent PERCEPTUAL`, JPEG 품질은 core 설정 `plugins/imageio/format/jpeg/quality` |
| **color profile 실체** | **별도 ICC 파일이 없다.** `--icc-type SRGB`로 darktable 내장 sRGB를 쓴다. 인벤토리는 이 사실을 그대로 적는다 — 없는 파일을 지어내지 않는다 |
| **proxy recipe 실체** | 세 preset의 XMP 템플릿이 `preset/default_catalog_assets/*.xmp`이고 `include_str!`로 **바이너리 안에 컴파일된다** (`preset/default_catalog.rs:20-25`). 첫 실행 시 `ensure_default_preset_catalog_in_dir`가 런타임 카탈로그로 펼친다. 인벤토리는 "앱 바이너리에 내장"으로 기록한다 |
| fixture 표시 | `BOOTHY_DISPLAY_SAMPLE_MODE`(기본 `off`). fixture는 `bundle.resources`로 이미 동봉되어 있고 `commands/display_commands.rs:433 resolve_sample_fixture_bytes()`가 `BaseDirectory::Resource` → 저장소 폴백 순으로 읽는다. **AC 2의 "fixture display"가 딛고 설 발판이 이미 있다** |
| 세션 루트 | `session/session_repository.rs:444` — `%USERPROFILE%\Pictures\dabi_shoot`, 실패 시 `app_local_data_dir/dabi_shoot`. **후자가 `identifier`에 딸려 움직인다** |
| asset protocol scope | `tauri.conf.json`: `$PICTURE/dabi_shoot/**`, `$APPLOCALDATA/dabi_shoot/**`. **`identifier`를 바꾸면 두 번째가 가리키는 실경로가 바뀐다** |
| viewer 창 | 단일 모니터에서도 `MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK`으로 **창모드로 열린다** (`commands/viewer_commands.rs:317-345`). clean VM(모니터 1개)에서 fixture 표시가 가능한 이유다 |
| 앱 진입점 | `src-tauri/src/main.rs` → `boothy_lib::run()`. **릴리스 빌드는 `windows_subsystem = "windows"`라 콘솔이 없다** — self-check 결과를 stdout으로 알릴 수 없다 |
| 롤아웃 거버넌스 | `branch_config/mod.rs`가 `branch-rollout-store/v1` 등을 이미 소유한다. 계약은 `docs/contracts/branch-rollout.md`. **이 Story는 여기에 손대지 않는다** |
| Rust 직접 의존성 | **5개뿐**: `serde_json`, `serde`, `log`, `tauri`, `tauri-plugin-log`. Story 7.3(LibRaw 제거)과 7.6(taskkill 선택)이 **이 Story를 위해** 지켜 온 값이다 |
| 하드웨어 게이트 관례 | `tests/hardware/<domain>/hv-XX/`에 `README.md` + `check-*.ps1`(기계식 게이트) + `test-check-*.ps1`(게이트 자체의 테스트). hv-17은 여기에 TS 러너(`run-tier-evidence.ts` + `.test.ts`)를 더했다 |

### 핵심 설계 결정 1 — 인벤토리는 문장이 아니라 **대조 가능한 파일**이다

AC 1은 "정확한 버전과 해시"를 요구한다. 사람이 쓴 표는 그 요구를 만족시키지 못한다 —
빌드가 바뀌면 표는 조용히 낡는다. **명세와 산출물을 분리하고, 둘을 기계가 대조한다.**

```
release/inventory-spec.json     (소스, 사람이 유지)  →  무엇이 반드시 있어야 하는가
        ↓ 빌드가 staging 트리를 만든다
release/dist/inventory.json     (생성물)            →  실제로 무엇이 들어갔는가 + sha256
        ↓ verify가 둘을 대조 (실패 시 릴리스 중단)
설치본에 inventory.json 동봉     (resource)          →  런타임 self-check가 다시 대조
```

- **해시 단위는 "구성요소"다.** darktable 트리의 파일 수천 개를 개별로 적지 않는다.
  구성요소마다 `sourceArchiveSha256`(내려받은 원본)과 `stagedTreeDigest`(정렬된 상대경로+파일해시를
  연결해 다시 해시한 값) 두 개를 기록한다. 후자가 있으면 파일 하나가 바뀌어도 잡힌다.
- **누락·불일치·미신고 세 가지를 서로 다른 사유 코드로 실패시킨다.**
  `inventory-component-missing`, `inventory-digest-mismatch`, `inventory-unexpected-component`.
  한 덩어리 "검증 실패"로 뭉치면 운영자가 무엇을 해야 할지 모른다 (AC 2의 "actionable operator result").
- `inventory-spec.json`에 각 구성요소의 **license 필드를 필수로** 둔다. AC 1의 licensing 증거가
  "따로 관리하는 문서"가 아니라 **인벤토리 자체의 일부**가 되어야 낡지 않는다.

### 핵심 설계 결정 2 — darktable은 동봉하고, **번들 경로를 최우선으로 해석한다**

오늘의 해석 순서는 `ProgramFiles`를 먼저 본다. 트리를 동봉만 하고 순서를 그대로 두면
**"부스 PC에 이미 깔린 아무 버전의 darktable"이 고객 화면을 만든다.** 핀이 무의미해진다.

**바꿀 순서:**

| 우선순위 | source 라벨 | 경로 |
| --- | --- | --- |
| 1 | `env-override` | `BOOTHY_DARKTABLE_CLI_BIN` (개발/실험용, 유지) |
| 2 | **`bundled-resource`** | **`<current_exe 디렉터리>/darktable/bin/darktable-cli.exe`** |
| 3 | `program-files-bin` 등 | 기존 후보 3개 (개발 루프 유지용) |
| 4 | `path` | 기존 폴백 |

- **기존 후보를 지우지 않는다.** 개발자 PC에는 번들 트리가 없다. 순서만 앞에 끼운다.
- **`DarktableBinaryResolution.source`가 이미 telemetry에 실린다.** 실장비 회차에서 `bundled-resource`가
  아닌 값이 찍히면 **그 회차는 핀이 깨진 것**이다. HV-18A 게이트가 이 값을 검사한다.
- **트리 전체를 통째로 옮긴다.** `darktable-cli.exe` 하나만 복사하면 동작하지 않는다 —
  `lib/`, `share/darktable/`가 형제로 있어야 한다.
  `bundle.resources`를 **객체 표기 + 트레일링 슬래시**로 쓴다:
  `{"../release/vendor/darktable-5.4.1/": "darktable/"}`.
  **glob(`**/*`)을 쓰면 안 된다** — 객체 표기에서 glob은 하위 디렉터리 구조를 보존하지 않는다.

### 핵심 설계 결정 3 — helper는 `externalBin`이 아니라 `resources`로 간다

`externalBin`은 (a) 파일명에 `-x86_64-pc-windows-msvc` 접미사를 요구하고,
(b) **동반 DLL을 다루지 않는다.** self-contained publish 결과는 `canon-helper.exe` 하나가 아니라
.NET 런타임 어셈블리 + `EDSDK.dll` + `EdsImage.dll`이 함께 있는 **폴더**다.

**결론:** `bundle.resources`에 `{"../release/dist/canon-helper/": "sidecar/canon-helper/"}`로 넣는다.
그러면 설치 디렉터리에 `sidecar/canon-helper/canon-helper.exe`가 생기고,
`resolve_helper_launch_target()`의 **기존 후보 `current_dir.join("sidecar/canon-helper/canon-helper.exe")`가
그대로 적중한다** (`helper_supervisor.rs:126`). **helper 해석 코드는 수정하지 않는다.**

- publish 명령: `dotnet publish -c Release -r win-x64 --self-contained true`
- `PublishSingleFile`은 쓰지 않는다. EDSDK 네이티브 DLL이 단일 파일에서 추출 경로를 타면
  진단이 어려워지고, 부스에서 얻는 이득이 없다.
- publish 후 **`canon-helper.exe --version`과 `--self-check`를 staging 단계에서 실제로 실행**해
  통과해야 인벤토리에 들어간다. 빌드는 됐는데 실행이 안 되는 산출물을 담지 않는다.

### 핵심 설계 결정 4 — self-check는 콘솔이 아니라 **파일 + 종료 코드**로 말한다

릴리스 바이너리는 `windows_subsystem = "windows"`다. `--self-check`를 stdout으로 출력하면
**운영자에게 아무것도 보이지 않는다.** 콘솔을 붙이려고 `AttachConsole`을 부르는 것은
새 Windows API 의존을 들이는 일이고, 이 Story의 원칙(의존성 동결)에 어긋난다.

**계약:**

- `boothy.exe --self-check [--report <path>]`
- Tauri 빌더를 **세우기 전에** `run()` 초입에서 인자를 판별하고, 검사 후 프로세스를 종료한다.
  창을 만들지 않는다 — clean VM 자동화가 창을 닫을 방법이 없다.
- 기본 보고서 경로: `%LOCALAPPDATA%\<identifier>\diagnostics\install-self-check.json`
- 스키마: `install-self-check/v1`. 필드: `schemaVersion`, `checkedAt`, `appVersion`, `identifier`,
  `installRoot`, `components[]`(`name`, `expectedDigest`, `actualDigest`, `status`, `reasonCode`),
  `webview2Version`, `darktableResolution`(`binary`+`source`), `helperVersion`, `overall`
- 종료 코드: `0` 통과 / `1` 인벤토리 불일치 / `2` 보고서 기록 실패.
  **`2`를 `1`과 합치지 않는다.** "검사에 실패했다"와 "검사를 못 했다"는 다른 사실이다
  (Story 7.6이 `persist_present_record` 실패를 성공으로 반환하지 않게 고친 것과 같은 규칙).
- **정상 실행 경로에서도 같은 검사를 한 번 돌린다.** 실패하면 기존
  `diagnostics::audit_log`에 `release-governance` 계열 레코드를 남겨 **운영자 진단으로 투영**한다.

### 핵심 설계 결정 5 — 고객 화면에는 아무것도 추가하지 않는다

AC 2의 "actionable operator result"를 고객 문구로 오해하면 안 된다.

- **새 고객 카피 0개.** NFR-001의 copy budget은 이미 차 있다.
- 인벤토리 실패는 **운영자 진단과 릴리스 게이트**의 문제다. 고객은 기존 wait/call 안내만 본다.
- **새 booth readiness 사유 코드를 만들지 않는다.** 카메라 helper 부재는 이미
  `helper-binary-missing`으로 표현된다. 렌더러 부재는 렌더 실패 경로가 이미 소유한다.
- 관람 화면의 조작 요소·진단 표시는 **0**을 유지한다.

### 핵심 설계 결정 6 — `identifier`는 **지금** 바꾼다. 나중은 없다

`com.tauri.dev`는 스타터 기본값이다. 이 값은 (a) `%LOCALAPPDATA%\<identifier>\` 앱 데이터 루트,
(b) WebView2 user data 폴더, (c) NSIS 업그레이드 식별에 쓰인다.
**첫 서명 release candidate가 지점에 깔린 뒤에 바꾸면 업그레이드가 "다른 앱 설치"가 된다.**

- 새 값 기본안: `com.boothy.booth` (역방향 도메인, 소문자, `.dev`/`.tauri` 제거)
- 영향 범위는 좁다: 세션 데이터는 `%USERPROFILE%\Pictures\dabi_shoot`에 있고
  `app_local_data_dir`는 **폴백 경로**다 (`session_repository.rs:445`).
- 그래도 **개발 PC에 남아 있는 `%LOCALAPPDATA%\com.tauri.dev\dabi_shoot\` 세션은 보이지 않게 된다.**
  이 사실을 릴리스 노트와 `README.md`에 적는다. 조용히 바꾸지 않는다.
- `bundle.targets`는 `["nsis"]`로 좁힌다. MSI를 함께 만들면 서명·해시·업그레이드 검증 대상이
  두 배가 되는데, `release-baseline.md`도 HV-18A도 NSIS 하나만 이야기한다.

### 핵심 설계 결정 7 — `Microsoft.RawImageExtension`은 **판정만** 하고 닫는다

HV-16이 이 Story로 넘긴 것은 채택이 아니라 **질문**이다: 오프라인 부스에 배포할 수 있는가?

- 판정 산출물: `tests/hardware/installer/hv-18a/raw-image-extension-verdict.md`
- 판정해야 할 것: (1) Store 외 오프라인 설치 수단이 존재하는가,
  (2) 그 수단이 재배포 라이선스상 허용되는가, (3) 버전 고정이 가능한가.
- **세 개 중 하나라도 아니오면 `offline-distribution: not-possible`로 닫는다.**
  HV-16이 이미 parity 미달(SSIM 0.013~0.159)을 기록했으므로, **이 판정이 긍정이어도 채택은 아니다.**
  `RESIDENT_APPROVED_DIRECT_DECODERS`는 비어 있는 채로 둔다.
- 판정 결과를 ledger Story 7.7 행과 HV-16 후속 항목에 **양쪽 다** 적는다. 한쪽만 적으면 다음 회차에 또 묻는다.

### 핵심 설계 결정 8 — 업그레이드·롤백·제거는 **데이터 보존**으로 판정한다

AC 2의 upgrade/rollback/uninstall은 "설치 프로그램이 오류 없이 끝났다"가 아니다.
**설치 전후로 무엇이 살아남았는가**가 판정 기준이다.

| 시나리오 | 반드시 살아남아야 하는 것 | 반드시 사라져야 하는 것 |
| --- | --- | --- |
| 업그레이드 (v_a → v_b) | 세션 루트 전체, published preset 카탈로그, branch config, rollout history, viewer display pointer | 이전 버전의 프로그램 파일 |
| 롤백 (v_b → v_a 재설치) | 위와 동일. **v_b가 만든 세션이 v_a에서 읽혀야 한다** (pointer 스키마 v1~v4 하위호환) | v_b의 프로그램 파일 |
| 제거 | 세션 루트와 고객 사진 (**사용자 데이터는 지우지 않는다**) | 프로그램 파일, 번들된 darktable 트리, helper 트리, 시작 메뉴 항목 |

- **제거가 고객 사진을 지우면 안 된다.** 세션 루트는 `Pictures` 아래이고 installer 소유가 아니다.
  NSIS가 그 경로를 건드리지 않는지 **실제로 확인해 증거로 남긴다.**
- 롤백은 Story 7.9의 지점 승급 롤백이 아니라 **같은 PC에서 이전 설치본을 다시 까는 것**이다.
  혼동하지 않는다.

### 아키텍처 가드레일 (위반 시 리뷰 반려)

- **새 Rust crate를 추가하지 않는다.** 직접 의존성 5개를 유지한다. sha256이 필요하면
  Windows 내장 `certutil -hashfile <path> SHA256`을 절대 경로로 호출하거나, 빌드 시점 TS 도구에서
  Node `node:crypto`로 계산한다. **런타임 self-check가 해시를 다시 계산해야 하므로**,
  호스트 측은 `%SystemRoot%\System32\certutil.exe`를 절대 경로로 부른다
  (Story 7.6이 `taskkill`을 절대 경로로 부른 것과 같은 규칙과 같은 이유).
- 계약 정의는 TS/Rust **한 쌍**만 존재한다. 세 번째 정의를 만들지 않는다.
- 인벤토리 계약도 예외가 아니다: `src/shared-contracts/schemas/release-inventory.ts` +
  `src-tauri/src/contracts/dto.rs`의 대응 구조체. `docs/contracts/release-inventory.md`가 서술 계약.
- Rust command는 얇은 진입점이고 도메인 로직은 모듈에 둔다. self-check 로직은
  **`src-tauri/src/release/`** 새 모듈에 둔다 (`commands/`에 넣지 않는다).
- 파일 I/O를 하는 command는 `#[tauri::command(async)]`다.
- **`session.json`을 확장하지 않는다.** 인벤토리는 세션 진실이 아니다.
- 중간 산출물은 세션 루트 밖으로 새지 않는다. self-check 보고서는 앱 데이터 진단 폴더에 쓴다.
- 저장소에 대용량 벤더 페이로드를 커밋하지 않는다. `release/vendor/`는 `.gitignore`에 넣고
  **조달 절차와 해시를 문서로 고정한다** (EDSDK가 이미 쓰는 방식과 동일).

### UX 가드레일

- **고객 문구 신규 0개.** 설치·인벤토리·서명은 고객이 알 필요가 없는 사실이다.
- 관람 화면은 조작 요소 0, 진단 표시 0을 유지한다.
- 운영자 화면에도 **새 화면을 만들지 않는다.** 기존 진단/감사 목록에 레코드를 추가할 뿐이다.
- 설치 중 UAC 승격이 1회 발생한다 (`perMachine`). 이것은 운영자 절차 문서에 적는다.

### 절대 하지 말 것 (disaster prevention)

1. **darktable을 "별도 설치 후 PATH로 찾기"로 해결하지 않는다.** AC 2가 명시적으로 금지한다.
2. **번들 트리를 동봉하고 해석 순서를 그대로 두지 않는다.** 그러면 핀이 무의미해진다 (결정 2).
3. **미서명 설치본으로 HV-18A를 `Go`로 적지 않는다.** AC 1이 서명 증거를 요구한다.
4. **인벤토리 표를 사람 손으로 유지하지 않는다.** 기계가 대조하지 않는 표는 다음 빌드에서 거짓이 된다.
5. **`identifier` 변경을 다음 Story로 미루지 않는다.** 첫 배포 후에는 되돌릴 수 없다 (결정 6).
6. **새 Rust crate를 추가하지 않는다.** Story 7.3이 LibRaw를, 7.6이 `windows-sys`를 참은 이유가 여기다.
7. **`externalBin`으로 helper를 넣으려 하지 않는다.** 동반 DLL을 못 싣는다 (결정 3).
8. **`bundle.resources` 객체 표기에서 glob을 쓰지 않는다.** 디렉터리 구조가 뭉개진다.
9. **고객 문구·고객 화면·렌더 인자·lane 기본값을 건드리지 않는다.** HV-15 `Go`가 그 위에 서 있다.
10. **제거 시 세션 루트를 지우지 않는다.** 고객 사진이다.
11. **`RawImageExtension`을 채택하지 않는다.** 판정만 하고 닫는다 (결정 7).
12. **7.6 이전 빌드로 HV-18A를 실행하지 않는다.** 인벤토리에 들어가는 실행 파일이 다르다.

### Previous story intelligence

**Story 7.6 (`in-progress`, HV-17 `No-Go`)**
- `render/scheduler.rs`의 우선순위 큐, `taskkill /T /F` 프로세스 트리 종료, `raw_refined_publisher`가
  이미 들어와 있다. **인벤토리는 이 코드가 포함된 바이너리를 담아야 한다.**
- 7.6이 `windows-sys`를 거절한 명시적 이유가 **"Story 7.7의 offline install inventory를 바꾸기 때문"**이다.
  이 Story가 그 절제를 낭비하면 안 된다.
- HV-17A의 취소 증거가 미완이고 refined tier는 `tier-not-justified`로 **기본 `off`**다.
  인벤토리는 그 상태를 그대로 기록한다 — 켜질 예정인 lane을 켜져 있다고 적지 않는다.

**Story 7.5 (`done`, HV-16 `Technology No-Go`)**
- 상주 renderer는 기본 `off`, `RESIDENT_APPROVED_DIRECT_DECODERS`는 비어 있다.
  AC 1의 "display renderer 또는 shader bundle" 항목은 **"해당 없음 / 후보 비활성"**으로 기록한다.
  없는 번들을 인벤토리에 지어내지 않는다.
- `Microsoft.RawImageExtension` 판정이 이 Story로 넘어왔다 (결정 7).

**Story 7.4 (`done`, HV-15 `Go`)**
- proxy lane 기본 `on`, 렌더 인자 확정. **이 Story는 그 위를 지나가되 아무것도 만지지 않는다.**
- darktable 플래그는 5.4.1에서 **실측으로** 검증됐다 (`--icc-intent PERCEPTUAL`은 되고
  `INTENT_PERCEPTUAL`은 안 된다). **동봉하는 darktable이 정확히 5.4.1이어야 하는 이유다.**

**Story 7.3 (`done`, HV-14 `Technology No-Go`)**
- LibRaw를 뺀 이유가 **"LGPL-2.1/CDDL이라 Story 7.7의 오프라인 인벤토리에 들어간다"**였다.
  AC 1의 "선택된 source adapter"는 `raw-original`이고, 별도 라이브러리가 없다는 것이 정답이다.

**Story 6.1 (`done`)**
- `release-baseline.md`, `.github/workflows/release-windows.yml`, `branch_config` 모듈이 여기서 나왔다.
  **`release-baseline.md`가 실제와 어긋난 두 문장을 갖고 있다** (위 결손 표 #9). 이 Story가 바로잡는다.

### Git intelligence

최근 5개 커밋(`de57f1a`, `a55ac9d`, `c390f53`, `89ae52c`, `b24cfc4`)은 전부 Epic 7의
display/capture 경로와 하드웨어 증거다. **패키징을 건드린 커밋이 없다.**
`a55ac9d`("Checkpoint Stories 7.2-7.6")가 helper 빌드 산출물(`bin/`, `obj/`)을 추적 중인 흔적을 남겼다 —
현재 `git status`가 `sidecar/canon-helper/tests/**/bin/**`을 modified로 보고한다.
**이 Story가 인벤토리를 도입하면서 그 노이즈를 정리해야 한다** (T1의 `.gitignore` 항목).

### 최신 기술 정보 (착수 전 확인 완료, Tauri v2 공식 설정 스키마 기준)

- `bundle.windows.webviewInstallMode` 변형은 **`skip` / `downloadBootstrapper` / `embedBootstrapper` /
  `offlineInstaller` 네 가지뿐이고 기본값은 `{ "silent": true, "type": "downloadBootstrapper" }`**다.
  **v1에 있던 `fixedRuntime`은 v2 스키마에 없다** — WebView2 버전을 설치본으로 고정할 수 없으므로
  **HV 회차마다 실제 런타임 버전을 기록**해야 한다.
- `bundle.windows.nsis.installMode`는 `currentUser`(기본) / `perMachine` / `both`.
  그 외 `compression`(기본 `lzma`), `template`, `startMenuFolder`, `installerHooks`(`.nsh`).
- 서명 키: `bundle.windows.signCommand` 또는 `certificateThumbprint` + `digestAlgorithm` + `timestampUrl`.
- `bundle.targets` 허용값: `"all"` 또는 `deb`/`rpm`/`appimage`/`msi`/`nsis`/`app`/`dmg` 배열.
- `bundle.externalBin`은 **`-$TARGET_TRIPLE` 접미사 파일명을 요구**하고 동반 파일을 다루지 않는다.
- `bundle.resources`는 배열(구조 보존) 또는 객체(source→target). **객체 + 트레일링 슬래시**는
  디렉터리를 재귀 복사하며 구조를 보존하지만, **객체 + glob은 하위 구조를 보존하지 않는다.**
- 런타임 해석은 `app.path().resolve("<relative>", BaseDirectory::Resource)`.
  이미 `display_commands.rs:436`이 이 방식을 쓰고 있다.

### 기술 스택 고정값

| 항목 | 값 | 근거 |
| --- | --- | --- |
| Tauri | `2.10.3` (crate) / `@tauri-apps/cli ^2.10.1` | `src-tauri/Cargo.toml`, `package.json` |
| Rust | `1.77.2+` MSVC | `Cargo.toml rust-version`, `release-baseline.md` |
| Node / pnpm | `22.x` / `pnpm 10.31.0` | CI, `package.json packageManager` |
| .NET | `net8.0`, `win-x64`, **self-contained** | `CanonHelper.csproj` + AC 1 |
| EDSDK | `13.19.0` (2025-02-28), zip SHA256 `C4F15E9D…B3AA699D` | `sidecar/canon-helper/vendor/README.md` |
| darktable | **`5.4.1` (`release-5.4.1 / c3f96ca`)** | ledger darktable pin, 코드 상수 `PINNED` |
| 출력 색공간 | darktable 내장 sRGB (`--icc-type SRGB`), 별도 ICC 파일 없음 | `render/mod.rs:852-885` |
| 설치 타깃 | NSIS, `perMachine` | 결정 3·6 |
| WebView2 | `offlineInstaller` (버전 고정 불가 → 회차별 기록) | 결정 2 |
| 업데이터 | `createUpdaterArtifacts: false` **명시** | `release-baseline.md` 가드레일 |

### Project Structure Notes

**새로 만드는 것** (architecture.md의 디렉터리 구조 절도 함께 갱신한다 — Story 7.6이
`display/deadline_scheduler.rs` → `render/scheduler.rs`를 바로잡은 것과 같은 처리):

```
release/                              # 신규 최상위. 릴리스 조달·staging·검증 도구
├── README.md                         # 벤더 페이로드 조달 절차 (EDSDK, darktable) + 빌드 런북
├── inventory-spec.json               # 소스: 필수 구성요소·핀 버전·라이선스·기대 해시
├── build-inventory.ts                # staging 트리 → release/dist/inventory.json 생성
├── build-inventory.test.ts           # 생성기 단위 테스트 (vitest)
├── verify-inventory.ps1              # 기계식 게이트: spec vs staged
├── test-verify-inventory.ps1         # 게이트 자체의 테스트
├── vendor/                           # .gitignore. 조달된 원본 (darktable-5.4.1/, webview2/)
└── dist/                             # .gitignore. staging 산출물 (canon-helper/, inventory.json)

src-tauri/src/release/                # 신규 모듈
├── mod.rs
├── inventory.rs                      # inventory.json 로드 + 실제 파일 대조
└── self_check.rs                     # --self-check 진입점, install-self-check/v1 보고서

src/shared-contracts/schemas/release-inventory.ts   # Zod 계약 (TS 쪽 한 벌)
docs/contracts/release-inventory.md                 # 서술 계약
tests/hardware/installer/hv-18a/
├── README.md                         # clean offline VM 절차
├── check-installer-evidence.ps1      # HV-18A 기계식 게이트
├── test-check-installer-evidence.ps1
├── environment.md                    # 회차 환경 기록 템플릿
└── raw-image-extension-verdict.md    # 결정 7의 판정 산출물
```

**손대는 기존 파일:** `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`(self-check 분기),
`src-tauri/src/render/mod.rs`(darktable 후보 순서), `src-tauri/src/contracts/dto.rs`(DTO),
`package.json`(스크립트), `.github/workflows/release-windows.yml`, `.gitignore`,
`docs/release-baseline.md`(정본) + 루트 `release-baseline.md`(포인터로 축소), `README.md`,
`_bmad-output/planning-artifacts/architecture.md`,
`_bmad-output/implementation-artifacts/hardware-validation-ledger.md`,
`src/governance/hardware-validation-governance.test.ts`.

**손대지 않는 것:** `display/`, `viewer/`, `capture/`(helper 해석 포함), `preset/`, `timing/`,
`handoff/`, `branch_config/`, 모든 고객·운영자 React 화면, 모든 렌더 인자.

---

## Tasks / Subtasks

### T1. 번들 정체성과 타깃을 확정한다 (AC: 1, 2)

- [x] `src-tauri/tauri.conf.json`
  - [x] `identifier`를 `com.tauri.dev` → `com.boothy.booth`로 바꾼다
  - [x] `bundle.targets`를 `"all"` → `["nsis"]`로 좁힌다
  - [x] `bundle.createUpdaterArtifacts: false`를 **명시**한다 (기본값에 의존하지 않는다)
  - [x] `bundle.windows.webviewInstallMode = { "type": "offlineInstaller", "silent": true }`
  - [x] `bundle.windows.nsis.installMode = "perMachine"`, `startMenuFolder = "Boothy"`
  - [x] `bundle.windows`에 `timestampUrl`/`digestAlgorithm`을 두고, 인증서 지정은
        환경변수 기반 `signCommand` 또는 `certificateThumbprint`로 **비어 있을 수 있게** 구성한다
- [x] `.gitignore`에 `release/vendor/`, `release/dist/` 추가
- [x] 현재 추적 중인 `sidecar/canon-helper/**/bin/`, `**/obj/` 잔여물을 인덱스에서 제거한다
      (`.gitignore`는 이미 `src/**`만 덮고 `tests/**`는 빠져 있다 — 규칙도 함께 고친다)
- [x] **회귀 확인:** `identifier` 변경이 `$APPLOCALDATA/dabi_shoot/**` asset scope와
      `session_repository.rs:445` 폴백에 미치는 영향을 테스트로 고정한다

### T2. 인벤토리 계약을 만든다 (AC: 1, 4)

- [x] `docs/contracts/release-inventory.md` — `release-inventory/v1` 서술 계약.
      구성요소 종류: `app`, `camera-helper`, `edsdk-runtime`, `source-adapter`, `display-renderer`,
      `color-profile`, `proxy-recipes`, `raw-renderer`, `webview2-runtime`
- [x] `src/shared-contracts/schemas/release-inventory.ts` — Zod 스키마.
      필수 필드: `name`, `role`, `version`, `origin`, `license`, `licenseEvidencePath`,
      `sourceArchiveSha256`(해당 시), `stagedTreeDigest`, `installRelativePath`, `signingStatus`
- [x] `src-tauri/src/contracts/dto.rs` — 대응 구조체 + 검증
- [x] **비어 있는 구성요소를 정직하게 표현할 방법을 계약에 넣는다.**
      `display-renderer`는 현재 후보 비활성이므로 `status: "not-applicable"` +
      `rationale`을 요구한다. `""`나 누락으로 표현하지 않는다
- [x] `color-profile`은 `origin: "darktable-builtin-srgb"`, `proxy-recipes`는
      `origin: "embedded-in-app-binary"`로 기록한다 (실체와 일치)
- [x] 계약 테스트: 라이선스 필드 누락, `not-applicable`에 rationale 없음, 알 수 없는 role → 전부 파싱 실패

### T3. 벤더 페이로드 조달 경로를 만든다 (AC: 1, 2)

- [x] `release/README.md` — darktable 5.4.1과 EDSDK 13.19.0의 조달 절차.
      다운로드 URL, 기대 SHA256, 추출 방법, 검증 명령을 **재현 가능하게** 적는다
- [x] darktable: 공식 Windows 배포본을 받아 `release/vendor/darktable-5.4.1/`로 추출한다.
      `bin/darktable-cli.exe`, `lib/`, `share/darktable/`이 형제로 존재하는지 확인한다
- [x] **추출 직후 실제로 실행해 버전을 읽는다:** `darktable-cli --version`이 `5.4.1`인지 확인.
      확인 실패는 조달 실패다. 인벤토리에 들어가지 않는다
- [x] darktable **GPL-3.0-or-later 준수 증거**를 `release/vendor/darktable-5.4.1/LICENSE-EVIDENCE.md`에
      모은다: 라이선스 원문 동봉, 대응 소스 확보 경로(정확한 tag/commit), 재배포 고지 문구.
      Boothy는 darktable을 **별도 프로세스로 CLI 호출**하므로 aggregation이지만,
      **동봉하는 바이너리 자체의 의무는 남는다.** 최종 문안은 승인자 확인 항목으로 남긴다
- [x] EDSDK 재배포 라이선스 근거를 `release/vendor/edsdk-13.19.0/LICENSE-EVIDENCE.md`에 기록한다
      (Canon 계약서 사본 경로 + 재배포 허용 조항). **없으면 `signingStatus`처럼 정직하게 결손으로 적는다**
- [x] `System.Drawing.Common 8.0.0`을 포함한 helper의 전이 의존성 라이선스 목록을 생성해 첨부한다

### T4. helper를 self-contained로 publish한다 (AC: 1, 2)

- [x] `CanonHelper.csproj`에 `RuntimeIdentifier`/`SelfContained`를 **강제하지 않는다** —
      개발 루프의 `dotnet run`이 느려진다. publish 명령의 인자로만 준다
- [x] `release/README.md`에 publish 명령을 고정한다:
      `dotnet publish sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj -c Release -r win-x64 --self-contained true -o release/dist/canon-helper`
- [x] publish 결과에 `canon-helper.exe`, `EDSDK.dll`, `EdsImage.dll`, .NET 런타임 어셈블리가
      모두 있는지 확인하는 검사를 `verify-inventory.ps1`에 넣는다
- [x] **staging 단계에서 실제 실행 검증:** `canon-helper.exe --version`과
      `canon-helper.exe --self-check --sdk-root release/dist/canon-helper` 둘 다 성공해야 한다
- [x] `tauri.conf.json` `bundle.resources`에
      `"../release/dist/canon-helper/": "sidecar/canon-helper/"` 추가
- [x] **helper 해석 코드는 수정하지 않는다.** 기존 `current_exe/sidecar/canon-helper/canon-helper.exe`
      후보가 적중하는지 테스트로 고정한다 (`helper_supervisor.rs:126`)

### T5. darktable 트리를 동봉하고 해석 순서를 바꾼다 (AC: 1, 2)

- [x] `tauri.conf.json` `bundle.resources`에
      `"../release/vendor/darktable-5.4.1/": "darktable/"` 추가 (**객체 표기 + 트레일링 슬래시**)
- [x] `render/mod.rs`의 `darktable_cli_binary_candidates()`에 **최우선 후보**를 추가한다:
      `current_exe().parent()/darktable/bin/darktable-cli.exe`, source 라벨 `bundled-resource`
- [x] `resolve_darktable_cli_binary_with_candidates()` 기존 테스트 3건을 유지한 채
      새 테스트를 더한다:
  - [x] 번들 후보가 존재하면 `ProgramFiles` 후보보다 **먼저** 선택된다
  - [x] `BOOTHY_DARKTABLE_CLI_BIN`은 여전히 번들보다 우선한다 (개발용 탈출구 보존)
  - [x] 번들 후보가 없으면 기존 순서가 그대로 유지된다 (개발 루프 무회귀)
- [x] **darktable 트리 무결성 검사를 self-check에 넣는다.** `bin/darktable-cli.exe`만이 아니라
      `lib/`와 `share/darktable/`의 존재까지 본다. 실행 파일만 있고 데이터가 없으면 렌더가 실패한다

### T6. 인벤토리 생성기와 기계식 검증을 만든다 (AC: 1, 4)

- [x] `release/inventory-spec.json` 작성. 위 9개 role을 모두 채우고,
      `display-renderer`는 `not-applicable` + rationale(HV-16 Technology No-Go)
- [x] `release/build-inventory.ts` — staged 트리를 훑어 구성요소별
      `stagedTreeDigest`(정렬된 `<상대경로>:<sha256>` 목록을 다시 sha256)를 계산하고
      `release/dist/inventory.json`을 쓴다
- [x] `release/build-inventory.test.ts` — 파일 하나만 바뀌어도 digest가 바뀌는지,
      파일 순서가 달라져도 digest가 같은지, 빈 디렉터리 처리가 결정적인지 고정
- [x] `release/verify-inventory.ps1` — spec vs staged 대조.
      **세 사유 코드를 각각 다른 종료 코드로** 낸다:
      `inventory-component-missing`(2), `inventory-digest-mismatch`(3), `inventory-unexpected-component`(4)
- [x] `release/test-verify-inventory.ps1` — 위 세 실패를 각각 인공적으로 만들어 게이트가
      **정말 그 코드로 실패하는지** 확인한다 (게이트가 통과만 하는 게이트가 되지 않게)
- [x] `tauri.conf.json` `bundle.resources`에
      `"../release/dist/inventory.json": "inventory.json"` 추가
- [x] `package.json`에 스크립트 추가: `release:stage`(helper publish + darktable 확인 + inventory 생성),
      `release:verify`(verify-inventory), `release:desktop`을 **두 단계 뒤에만** 돌도록 묶는다

### T7. self-check를 만든다 (AC: 2, 5)

- [x] `src-tauri/src/release/mod.rs`, `inventory.rs`, `self_check.rs` 생성
- [x] `lib.rs` `run()` 초입에서 `--self-check` 인자를 판별해 **Tauri 빌더 전에** 처리하고 종료한다
- [x] 보고서 스키마 `install-self-check/v1`을 계약(T2)과 같은 자리에 정의한다
- [x] 검사 항목: 인벤토리 매니페스트 존재 → 구성요소별 실제 digest 재계산 → darktable 해석 결과
      (`binary` + `source`가 `bundled-resource`인지) → helper 존재와 `--version` 성공 →
      darktable 트리 무결성 → WebView2 런타임 버전 판독
- [x] sha256은 `%SystemRoot%\System32\certutil.exe`를 **절대 경로로** 호출해 계산한다.
      새 crate를 넣지 않는다. `certutil` 실패는 무시하지 않고 `2`(검사 불가)로 구분한다
- [x] 종료 코드 `0`/`1`/`2`를 테스트로 고정한다
- [x] **정상 실행 경로에도 같은 검사를 붙인다.** 실패 시 `diagnostics::audit_log`에
      `release-governance` 계열 레코드를 남긴다. **고객 문구는 만들지 않는다**
- [x] 회귀 테스트: self-check 실패가 세션 시작·촬영·렌더 경로를 **막지 않는다**
      (진단으로 투영할 뿐, 고객 흐름을 새로 차단하지 않는다)

### T8. CI를 완성한다 (AC: 1, 2)

- [x] `.github/workflows/release-windows.yml`에 단계 추가:
      `.NET 8 SDK` → helper publish → darktable 벤더 페이로드 복원 → `release:stage` →
      `release:verify` → `tauri build` → 서명 → 산출물 이름 검증
- [x] **벤더 페이로드 조달 방법을 결정하고 문서화한다.** EDSDK와 darktable은 저장소에 없다.
      기본안: self-hosted runner의 고정 캐시 경로 또는 보안 아티팩트 저장소.
      **결정되지 않았다면 CI 단계를 만들되 명시적으로 skip 사유를 로그로 남긴다** —
      조용히 건너뛰지 않는다
- [x] 산출물 이름 검증을 **실제로** 구현한다: `Boothy_<version>_x64-setup.exe`.
      `release-baseline.md`가 이미 있다고 주장하는 그 검사다
- [x] 설치본 크기를 측정해 워크플로 요약과 인벤토리에 기록한다 (결정 1의 실측값)
- [x] 서명: 인증서 시크릿이 없으면 **미서명으로 빌드하되 `signingStatus: "unsigned"`를
      인벤토리에 기록**하고, 릴리스 태그 경로에서는 실패시킨다

### T9. HV-18A 하드웨어 증거를 생산한다 (AC: 2, 3)

- [x] `tests/hardware/installer/hv-18a/README.md` — clean offline VM 절차.
      hv-15/hv-17의 README 구조를 따른다
  - [x] VM 준비: Windows 클린 이미지, **Node/Rust/.NET SDK/darktable/WebView2 미설치 확인 명령**
  - [x] **네트워크 차단 확인 명령**(어댑터 비활성 + 확인). "인터넷이 없었다"는 주장이 아니라 증거여야 한다
  - [x] 8단계 lifecycle: 설치 → 실행 → self-check → fixture 표시 → 업그레이드 → 롤백 → 제거 → 데이터 보존 확인
- [x] `environment.md` 템플릿: 부스 PC, OS 빌드, **WebView2 런타임 실제 버전**,
      모니터/DPR, darktable resolution source, helper 버전, 설치본 해시, 서명 상태
- [x] fixture 표시 회차는 `BOOTHY_DISPLAY_SAMPLE_MODE`로 수행한다.
      **단일 모니터 VM에서 viewer가 창모드로 열리는 것이 정상**임을 절차에 적는다
- [x] `check-installer-evidence.ps1` — 기계식 게이트.
      **모든 검사가 통과해야만 `Go` 후보다:**
  - [x] self-check 보고서가 존재하고 `overall: "pass"`
  - [x] `darktableResolution.source == "bundled-resource"` (**핀이 깨지지 않았다는 증거**)
  - [x] 설치본 sha256이 인벤토리와 일치
  - [x] 8단계 lifecycle 결과가 모두 기록되어 있고 미실행 항목이 없다
  - [x] 제거 후 세션 루트가 **살아 있다**
  - [x] 업그레이드 후 이전 세션이 **읽힌다**
  - [x] `signingStatus`가 `signed`
- [x] `test-check-installer-evidence.ps1` — 위 각 조건을 하나씩 깨뜨려 게이트가 실제로 실패하는지 확인
- [x] `raw-image-extension-verdict.md` — 결정 7의 판정 (오프라인 배포 가능 여부 3문항)

### T10. 영향 문서를 갱신한다 (all AC)

- [x] **release baseline 정본을 하나로 만든다 (결손 표 #9).**
  - [x] `docs/release-baseline.md`를 정본으로 확정한다 (`README.md:17`이 이미 가리키는 쪽이고,
        CI 실태를 정직하게 적고 있는 쪽이다)
  - [x] 루트 `release-baseline.md`는 **정본으로 가는 한 줄 포인터로 축소한다.**
        조용히 지우지 않는다 — 링크가 남아 있을 수 있다
  - [x] 정본에 새 인벤토리·서명·오프라인 절차·번들 darktable·self-check를 반영한다
  - [x] T8이 산출물 이름 검증을 **실제로** 구현한 뒤에만 그 문장을 정본에 적는다.
        구현 전에 먼저 적지 않는다 (그것이 루트 사본이 거짓이 된 경위다)
  - [x] `createUpdaterArtifacts: false`는 T1에서 설정에 **명시된 뒤에** 문서가 그렇게 주장한다
- [x] `README.md` — Windows 부트스트랩 절에 **"개발 환경 요구사항"과 "부스 설치 요구사항"을 분리**한다.
      개발자는 WebView2를 깔지만 **부스는 installer가 넣는다.** `identifier` 변경 영향도 적는다
- [x] `_bmad-output/planning-artifacts/architecture.md`
  - [x] 디렉터리 구조 절에 `release/`와 `src-tauri/src/release/` 추가
  - [x] Deployment Architecture 절에 인벤토리·self-check·번들 darktable 반영
  - [x] `#### Story 7.7 implementation note (2026-08-…)` 추가 — **결정과 근거를 남긴다**
- [x] `docs/contracts/release-inventory.md` 완성
- [x] `hardware-validation-ledger.md`
  - [x] Sprint Review Gateboard의 `7.7` 행 갱신
  - [x] Evidence Registry `### Story 7.7`에 evidence path, executedAt, validator, booth PC,
        **WebView2 버전**, darktable pin, helper 식별자, 설치본 해시, 서명 상태 기록
  - [x] HV-16 후속 항목에 `RawImageExtension` 판정 결과를 **역참조**로 남긴다
- [x] `src/governance/hardware-validation-governance.test.ts` — Story 7.7 행이 ledger에
      존재하고 상태가 스토리 파일과 일치하는지 고정
- [x] `deferred-work.md` — 이 Story가 열어 놓고 닫지 못한 것이 있으면 **이유와 함께** 남긴다

---

## Product Decision Rules

- **인벤토리에 없는 것은 배포되지 않은 것이다.** 설치본에 들어갔는데 인벤토리에 없으면 실패다.
  인벤토리에 있는데 설치본에 없어도 실패다. 둘은 같은 심각도다.
- **미서명 설치본은 release candidate가 아니다.** 개발 검증에는 쓸 수 있지만 HV-18A `Go`의 근거가 아니다.
- **"인터넷이 필요 없다"는 주장이 아니라 측정이다.** 네트워크가 실제로 차단된 상태의 증거가 없으면
  clean offline 회차로 인정하지 않는다.
- **darktable resolution source가 `bundled-resource`가 아닌 회차는 핀이 깨진 회차다.**
  성능이 좋게 나와도 그 숫자는 다른 렌더러의 숫자일 수 있다.
- **제거가 고객 사진을 지우면 그 회차는 즉시 `No-Go`다.** 되돌릴 수 없는 결함이다.
- **후보 기술을 인벤토리에 넣어 살려 내지 않는다.** HV-14·HV-16의 `Technology No-Go`는
  이 Story에서 뒤집히지 않는다.

---

## Definition of Done

1. AC 1~5가 코드·계약·테스트로 성립한다
2. `pnpm test:run`, `cargo test`, `dotnet test`, `pnpm lint`, `pnpm build`가 통과한다
3. `release:stage` → `release:verify` → `release:desktop`이 로컬에서 한 번에 통과한다
4. `verify-inventory.ps1`의 세 실패 사유가 각각 재현되고 각기 다른 종료 코드로 실패한다
5. `check-installer-evidence.ps1`의 모든 검사가 하나씩 깨뜨려질 때 실제로 실패한다
6. 책임자가 승인한 현재 PC에서 8단계 lifecycle 증거가 **미실행 항목 없이** 수집되어 있고,
   clean/offline 제외는 `waived-by-owner`로 남아 있다
7. `RawImageExtension` 오프라인 배포 판정이 기록되어 있다
8. 영향 문서 6종이 실제 상태와 일치한다 (없는 동작을 사실처럼 적은 문장이 남아 있지 않다)
9. 기계식 gate candidate와 책임자의 최종 범위 승인이 모두 기록된 뒤에만 `done`이다
10. **고객 화면 무회귀 증거:** proxy lane·384px 레일·final 경로·렌더 인자가 변하지 않았음을
    테스트와 grep이 아닌 **테스트로** 고정한다 (Story 7.6의 scope guard 테스트 방식을 따른다)

---

## Dependencies

**선행 (2026-08-17 내부 검증 범위에서 정리됨):**
- Story 7.6은 `HV-17A automated-pass + refined tier 제외`의 책임자 승인 `Partial`로 닫혔다.
- 코드 서명, Canon EDSDK 재배포 증거, clean/offline 환경은 내부 검증에서 `waived-by-owner`다.
- EDSDK와 darktable은 현재 승인 PC의 로컬 페이로드로 실제 설치본을 만들고 검증했다. CI 조달 경로는 공개/자동 배포를 다시 열 때 결정한다.

**후행 (이 Story가 풀어 주는 것):**
- Story 7.8 / HV-18B — 완전 설치된 release candidate 위에서만 100-shot을 측정한다
- Story 7.9 / HV-18C — 승급·롤백의 대상이 이 설치본이다
- Story 7.10 / HV-18D — 인벤토리와 라이선스 증거가 최종 판정의 입력이다

---

## Handoff

- **Story 7.8에게:** WebView2 실제 버전과 darktable resolution source를 회차 환경에 반드시 기록해 달라.
  버전이 다르면 present 타이밍 비교가 성립하지 않는다.
- **Story 7.9에게:** 이 Story의 롤백은 **같은 PC의 재설치**다. 지점 승급 롤백은 그쪽 몫이다.
  두 개념을 같은 표에 섞지 말 것.
- **Story 7.10에게:** 인벤토리의 `signingStatus`, `license`, `status: not-applicable` 항목이
  최종 판정에서 그대로 인용 가능하도록 설계했다. 요약하지 말고 원본을 인용해 달라.

---

## References

- [Epic 7 Story 7.7](../planning-artifacts/epics.md#story-77-완전한-installer와-clean-offline-재현)
- [PRD NFR-006 Safe Local Packaging, Rollout, and Version Pinning](../planning-artifacts/prd.md#nfr-006-safe-local-packaging-rollout-and-version-pinning)
- [Architecture — Infrastructure & Deployment](../planning-artifacts/architecture.md#infrastructure--deployment)
- [Architecture — Deployment Architecture](../planning-artifacts/architecture.md#deployment-architecture)
- [Architecture — Story 7.3 / 7.4 / 7.6 implementation notes](../planning-artifacts/architecture.md#approved-2026-08-11-correct-course-baseline)
- [Hardware Validation Ledger — Story 7.7 / HV-18A](./hardware-validation-ledger.md#story-77)
- [HV-16 decision — RawImageExtension 판정 이관](../../tests/hardware/resident-renderer/hv-16/decision.md)
- [Story 7.6 — 의존성 동결과 taskkill 결정](./7-6-raw-정밀본-무중단-교체.md)
- [Story 7.3 — LibRaw 제외와 오프라인 인벤토리 근거](./7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교.md)
- [docs/release-baseline.md](../../docs/release-baseline.md) — **정본.** 루트 `release-baseline.md`는 갈라진 낡은 사본이다
- [docs/contracts/branch-rollout.md](../../docs/contracts/branch-rollout.md)
- [docs/runbooks/booth-hardware-validation-checklist.md](../../docs/runbooks/booth-hardware-validation-checklist.md)
  — HV-00~HV-12 legacy 게이트만 다룬다. **HV-18A는 여기에 없다.** Epic 7 게이트는
  `tests/hardware/<domain>/hv-XX/`가 소유한다. 이 runbook을 HV-18A의 근거로 인용하지 않는다
- Tauri v2 설정 스키마: `bundle.windows.webviewInstallMode`, `bundle.windows.nsis`,
  `bundle.externalBin`, `bundle.resources` (https://v2.tauri.app/reference/config/)

### Review Findings

- [x] [Review][Patch] [HIGH] HV-18A가 실제 camera/proxy/RAW/final 실행 없이도 통과한다 — 실제 카메라 촬영 1회와 capture-bound proxy·RAW·final 산출물을 HV-18A 필수 증거로 추가하고 게이트가 이를 검증한다. 사용자 결정: 실제 촬영 기반 증거 방식. [tests/hardware/installer/hv-18a/check-installer-evidence.ps1:1]
- [x] [Review][Patch] [HIGH] 실패한 inventory verify 뒤에도 workflow_dispatch가 release build·seal을 수행하고 성공할 수 있다 [.github/workflows/release-windows.yml:149]
- [x] [Review][Patch] [MEDIUM] `-SkipHelperPublish` 성공 경로가 이전 `PAYLOAD-NOT-STAGED.md`를 제거하지 않아 복구된 payload를 계속 결손 처리한다 [release/stage.ps1:85]
- [x] [Review][Patch] [MEDIUM] 정상 시작의 release governance가 스토리가 요구한 동일 self-check 대신 최상위 경로 존재만 확인해 manifest·digest·부분 설치 결함을 진단하지 않는다 [src-tauri/src/release/self_check.rs:531]
- [x] [Review][Patch] [MEDIUM] HV-18A 게이트가 절차상 필수인 `after-rollback.json`을 읽지 않아 깨진 rollback 설치도 통과할 수 있다 [tests/hardware/installer/hv-18a/check-installer-evidence.ps1:86]
- [x] [Review][Patch] [LOW] `--report` 다음의 다른 플래그를 파일 경로로 받아 self-check 보고서 저장이 잘못될 수 있다 [src-tauri/src/release/self_check.rs:65]
- [x] [Review][Patch] [LOW] WebView2 버전을 한 보고서에서 두 번 읽어 component 판정과 기록 버전이 모순될 수 있다 [src-tauri/src/release/self_check.rs:398]
- [x] [Review][Patch] [LOW] HV-18A 설치본 해시 비교가 64자리 SHA-256 형식을 검증하지 않아 같은 임의 문자열도 통과한다 [tests/hardware/installer/hv-18a/check-installer-evidence.ps1:138]
- [x] [Review][Patch] [LOW] release runbook이 helper 검증 실패 시 inventory를 쓰지 않는다고 설명하지만 실제 staging은 결손 inventory를 기록한다 [release/README.md:128]

---

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (1M context) — `claude-opus-5[1m]`

### Debug Log References

- `darktable-cli --version`: 5.4.1 통과
- `pnpm release:desktop`: 전체 staging·verify·NSIS build 통과, 완전 설치본 405,281,535 bytes
- `tests/hardware/installer/hv-18a/test-check-installer-evidence.ps1`: 31/31 통과
- `tests/hardware/installer/hv-18a/check-installer-evidence.ps1`: `Go-candidate-with-waivers`
- 실제 Canon EOS 700D 촬영: 동일 capture에 CR2 원본, display proxy, final 결과 생성
- 설치 lifecycle: 0.1.0 설치 → 0.1.1 업그레이드 → 0.1.0 롤백 → 제거; 세션 호환성 2회 통과, 고객 파일 1,152개 보존
- `pnpm lint`: 통과
- `pnpm build`: 통과
- `pnpm test:run`: 620 passed / 1 skipped / 0 failed
- `cargo test` (src-tauri): 564 passed / 0 failed
- `dotnet test` (CanonHelper.Tests): 46 passed / 0 failed
- `pnpm build:desktop`: NSIS 설치본 1개 생성 성공
- `pnpm test:release-gate`: 13/13 — 각 사유 코드가 자기 종료 코드로 실패함을 확인
- `tests/hardware/installer/hv-18a/test-check-installer-evidence.ps1`: 19/19 — 완전한 회차 1건 통과, 결손 18종 거부
- `pnpm release:stage`: helper publish → `--version` → `--self-check` 전부 통과. 인벤토리 생성
- `pnpm release:verify`: 종료 코드 `2` (`inventory-component-missing`, `raw-renderer` 1건) —
  **darktable 페이로드가 없는 현재 상태에서 기대한 동작이다.** DoD 3의
  `release:stage → release:verify → release:desktop` 한 번에 통과는 **darktable 조달 전에는 성립할 수 없다.**

**중간에 잡힌 결함 넷.**

앞의 둘은 **"설치본이 자기 인벤토리와 다르다"로만 보였을 것들**이고, 셋째는 self-check 자체의
판정 구멍, 넷째는 내가 테스트 스위트에 더한 부하다.

1. **Rust 트리 해시가 TS/PowerShell과 다른 값을 냈다.** 임시 파일 이름이 `<pid>-<파일수>`라
   같은 프로세스의 두 구성요소가 동시에 해시될 때 서로의 임시 파일을 지웠다. 일련번호로 고쳤다.
   **골든 벡터가 없었다면 첫 실장비 회차에서 원인 없는 `digest-mismatch`로만 보였을 것이다.**
2. **PowerShell 게이트가 실제 helper 트리에서만 다른 해시를 냈다.** `Sort-Object`의 기본 비교자가
   culture-aware라 `.`과 `-`이 섞인 .NET 어셈블리 이름 311개를 TS/Rust와 다른 순서로 늘어놓았다.
   **작은 fixture에서는 두 순서가 같아 보였다** — 실제 페이로드로 staging 하기 전까지 드러나지 않았다.
   `[Array]::Sort` + `StringComparer.Ordinal`로 고치고, 게이트 fixture에 그 차이가 드러나는
   파일 이름들(`System.Private.CoreLib.dll`, `a-b.dll`, `a.b.dll`, …)을 넣었다.
   **고치기 전 코드로 되돌려 게이트 테스트가 실제로 5건 실패하는 것을 확인했다.**
3. **self-check가 인벤토리의 `missing` 항목을 `skipped`로 넘겼다.** "동봉하지 않기로 결정했다"와
   "있어야 하는데 없다"를 같은 분기로 처리한 탓이다. 그 상태로는 **결손 인벤토리를 담은 설치본이
   self-check를 통과할 수 있었다.** `missing`은 `inventory-component-missing`으로 실패시키고,
   `embedded`/`not-applicable`만 건너뛰도록 갈랐다. 두 경우를 각각 테스트로 고정했다.
   덤으로, 설치본 안에서 `PAYLOAD-NOT-STAGED.md`를 발견하면 해시를 재기 전에 그 사실을 사유로
   보고하도록 했다 — 그러지 않으면 운영자에게는 원인 없는 `digest-mismatch`로만 보인다.
4. **내가 전체 스위트에 부하를 더했다.** 「실제 명세가 계약을 만족하는가」 테스트가 실제 staged
   트리(313파일 315 MB)를 vitest 프로세스 안에서 해시하고 있었다. 그 테스트의 목적은 명세가
   계약에 맞는 문서를 만드는지이지 해시가 아니므로 임시 디렉터리를 쓰도록 바꿨다.
   **세 구현의 해시 일치는 PowerShell 게이트와 Rust 통합 테스트가 실제 트리로 증명한다.**

앞의 세 결함 모두 **실제 페이로드로 한 번 돌려 봤기 때문에** 잡혔다.

**남은 간헐 실패는 이 Story의 것이 아니다.** 전체 vitest 실행에서 `PresetLibraryScreen` /
`SettingsScreen`의 `findBy*` 테스트가 부하가 높은 회차에 이따금 1초 기본 타임아웃을 넘긴다
(`PresetLibraryScreen` 단독 실행만으로도 18개 테스트에 11.4초가 걸린다).
**내 barrel export를 제거한 상태로 재현해 확인했다** — 없는 상태에서도 같은 패턴으로 실패했고,
그 뒤 두 회차는 통과했다. 이 Story는 해당 화면 코드를 건드리지 않는다.

### Completion Notes List

#### 2026-08-17 HV-18A 종료 결과

- darktable 5.4.1 전체 트리를 동봉한 설치본을 실제 생성하고 크기와 해시를 봉인했다.
- 번들 sample B를 1920×1080 관람 화면에 실제 표시해 `fixture/display.png`로 남겼다.
- Canon EOS 700D 실제 촬영으로 RAW original, display proxy, final을 같은 captureId에 묶었다.
- HV-17 `Partial` 판정에 따라 raw-refined는 `tier-not-justified`로 만들지 않았고, lane 기본값은 꺼진 상태를 유지했다.
- 업그레이드와 롤백 설치본이 같은 완료 세션을 제품 self-check로 직접 읽었다.
- 관람 화면 준비 완료 뒤 촬영 화면이 계속 대기하는 경쟁 조건을 재현해 자동 갱신하도록 수정하고 테스트로 고정했다.
- 서명, Canon 재배포 증거, clean/offline 환경은 책임자 승인 면제로 기록했으며 통과로 바꾸지 않았다.
- 기계식 판정은 `Go-candidate-with-waivers`; 책임자가 승인한 현재-PC 내부 범위는 `Go-with-waivers`로 확정해 Story를 `done`으로 닫았다.

#### 무엇을 만들었나

- **번들 정체성 확정.** `identifier`를 `com.boothy.booth`로, `targets`를 `["nsis"]`로,
  `createUpdaterArtifacts: false`를 명시로, WebView2를 `offlineInstaller`로, NSIS를 `perMachine`으로
  바꿨다. 각 값을 `src-tauri/tests/story_7_7_packaging.rs`가 단언으로 고정한다.
- **인벤토리 계약 `release-inventory/v1`.** Zod 한 벌 + Rust 한 벌 + 서술 계약 한 벌.
  구성요소 9종을 전부 요구하고, 동봉하지 않은 것은 `not-applicable`/`embedded`/`missing` +
  `rationale`로 **정직하게** 표현하게 강제한다. 라이선스 필드는 모든 구성요소에 필수다.
- **기계식 검증.** `release/verify-inventory.ps1`이 명세·인벤토리·디스크를 대조하고
  누락(2)·불일치(3)·미신고(4)·미staging(5)·매니페스트 불가독(6)·핀 없음(7)을 각기 다른 종료 코드로 낸다.
- **`--self-check`.** `install-self-check/v1` 보고서를 쓰고 `0`/`1`/`2`로 끝난다. 창을 만들지 않는다.
- **darktable 해석 순서.** 번들 트리가 최우선(`bundled-resource`)이고 기존 후보는 뒤에 남는다.
- **HV-18A 절차와 게이트.** 8단계 lifecycle, 네트워크 차단 증거, 사전 미설치 증거, 데이터 보존 판정.

#### 요구된 기록 항목

#### 실제로 staging 된 것 (이 머신에 EDSDK 페이로드가 있어 끝까지 돌았다)

`BOOTHY_CANON_SDK_ROOT`가 가리키는 EDSDK가 이 머신에 있어 helper 경로는 **문서가 아니라 실행으로**
증명됐다. darktable만 없어서 그 하나만 결손으로 남았다.

| 구성요소 | 실측 | 비고 |
| --- | --- | --- |
| `camera-helper` | **311 파일 / 315,334,154 바이트 (300.7 MB)** | `dotnet publish -r win-x64 --self-contained true` 성공. digest `5dc07c8d…1a23` |
| `edsdk-runtime` | **2 파일 / 2,774,016 바이트** | digest `6b725039…1072` — **명세에 핀으로 기록했다** (조달된 payload라 바이트가 고정이다) |
| `canon-helper.exe --version` | 성공 | `helperVersion 0.1.0`, `sdkVersion 13.19.0`, `camera-helper-sidecar/v2` |
| `canon-helper.exe --self-check` | 성공 | `detailCode: camera-ready`, **카메라 1대 인식**, `sdkInitialized: true` |
| `raw-renderer` (darktable) | **결손** | 페이로드 미조달. 인벤토리가 `missing`으로 기록하고 `release:verify`가 종료 코드 `2`로 막는다 |
| **self-check 해시 재계산 비용** | **313 파일 / 315 MB에 5.7초** (워커 8개) | `--self-check`가 감당 가능한 범위임을 실측으로 확인. 부팅 경로는 해시를 다시 계산하지 않는다 |
| **NSIS 설치본 실제 빌드** | **성공** | `pnpm build:desktop` → `Boothy_0.1.0_x64-setup.exe` 하나만 생성 (`targets: ["nsis"]` 확인). 빌드 로그에 WebView2 오프라인 설치본 다운로드·동봉이 찍힌다 |
| **설치본 크기 (darktable 제외)** | **298,663,653 바이트 = 284.8 MB** | sha256 `3b403d90…0559`. **darktable이 빠진 값이다.** 완전한 설치본은 darktable 트리만큼 더 커진다 |
| `pnpm release:seal` | 성공 | 이름 검증 통과 후 `inventory.release.json`에 이름·크기·sha256 기록. 이름을 `Boothy-setup.exe`로 바꿔 실행하면 종료 코드 `1`로 거부하는 것도 확인 |
| **`boothy.exe --self-check` 실제 실행** | 종료 코드 `1`, 창 없음 | 설치 트리의 `camera-helper`·`edsdk-runtime` 해시가 **생성기 값과 일치**했고, helper 실행 파일이 `sidecar/canon-helper/`에서 실제로 기동했다. `darktable-resolution`은 `darktable-not-bundled`로 정확히 실패했다 (이 개발 PC의 darktable은 `ProgramFiles`에 있다) |
| **WebView2 런타임 실측 버전** | **151.0.4129.86** | 빌드 머신 값. 부스 PC 값은 회차마다 다시 읽는다 |

**세 구현이 실제 311파일 트리에서 같은 해시를 낸다**는 것을 `cargo test`가 대조로 고정한다
(`the_runtime_digest_matches_the_generated_inventory_for_a_real_staged_tree`).
staged 페이로드가 없는 머신에서는 조용히 건너뛴다 — 검사하지 않은 것을 검사했다고 적지 않는다.

#### 요구된 기록 항목

| 항목 | 값 | 비고 |
| --- | --- | --- |
| **설치본 실측 크기** | **284.8 MB (298,663,653 바이트) — darktable 제외** | 실제로 빌드해 측정했다. darktable을 넣으면 그만큼 커지므로 **이 값은 완전한 설치본의 하한이다.** 완전한 값은 CI 봉인 단계가 잡 요약과 `inventory.release.json`에 기록한다. **추정치를 실측처럼 적지 않는다.** |
| **WebView2 실제 버전** | **151.0.4129.86** (빌드 머신 실측) | v2 스키마에 `fixedRuntime`이 없어 설치본으로 고정할 수 없다. `--self-check`가 레지스트리에서 읽어 `webview2Version`에 기록하고, **회차마다 부스 PC의 값을 다시 읽어** `environment.md`에 남긴다. |
| **darktable resolution source** | 이 PC에서 `program-files-bin` → self-check가 `darktable-not-bundled`로 **정확히 실패** | 번들 트리가 없으면 기존 후보로 내려가고, 그 회차는 핀이 깨진 회차로 판정된다. 설계대로 동작하는 것을 실행으로 확인했다. HV-18A 게이트도 같은 값을 검사한다. |
| **서명 상태** | `unsigned` | 인증서가 아직 없다. 파이프라인은 완성했고 인벤토리가 `signingStatus: "unsigned"`를 기록한다. **미서명 설치본은 HV-18A `Go`의 근거가 아니다.** |
| **CI 벤더 페이로드 조달 방식** | **미결** | 워크플로는 `BOOTHY_VENDOR_CACHE`를 읽고, 비어 있으면 **건너뛴 사실과 사유를 로그와 잡 요약에 남긴다.** 조용히 넘어가지 않는다. |
| **`RawImageExtension` 판정** | `offline-distribution: not-possible` / `adoption: not-adopted` | 세 문항(오프라인 설치 수단·재배포 권리·버전 고정)이 전부 「아니오」. `RESIDENT_APPROVED_DIRECT_DECODERS`는 비어 있는 채로 둔다. 판정이 긍정이었어도 HV-16의 parity 미달이 독립적으로 막는다. |

#### 닫지 못한 것과 그 이유

체크하지 않은 3개 subtask가 그대로 남아 있다. 전부 **물리적 페이로드가 있어야 가능한 일**이다.

1. **darktable 5.4.1 트리 조달·추출** — 저장소에 없고 이 환경에서 받을 수 없다.
   절차·기대 해시 자리·검증 명령은 `release/README.md`에 재현 가능하게 적었다.
2. **`darktable-cli --version` 실측 확인** — 위와 같은 이유. `release/stage.ps1`이 실행 시 확인하고,
   `5.4.1`이 아니면 결손 표시를 남겨 인벤토리에서 `missing`이 되게 만들었다.
3. **설치본 크기 실측** — 완전한 설치본을 만들 수 없었다. 측정·기록 코드는 완성되어 있다.

외부 종속 세 건은 코드로 풀 수 없다: **서명 인증서 미발급**, **Canon EDSDK 재배포 권리 미확인**,
**CI 벤더 페이로드 조달 경로 미결**. 전부 `deferred-work.md`에 사유와 함께 남겼다.

#### 승인 필요 결정 — 기본안대로 진행한 결과

| # | 기본안 | 결과 |
| --- | --- | --- |
| 1 | darktable 설치 트리를 앱 resource로 동봉 | 그대로 진행. 크기는 미측정 |
| 2 | `webviewInstallMode: offlineInstaller` | 그대로 진행. 버전 고정 불가라 회차별 기록으로 대응 |
| 3 | NSIS `perMachine` | 그대로 진행. 설치 시 UAC 승격 1회를 절차 문서에 명시 |
| 4 | 서명 파이프라인 완성 + 인증서는 별도 안건 | 그대로 진행. 미서명은 인벤토리에 정직하게 기록 |

#### 명시적으로 다르게 한 것 (승인자 확인 요망)

- **라이선스 증거를 `release/vendor/<component>/LICENSE-EVIDENCE.md`가 아니라 `release/licenses/`에 뒀다.**
  `release/vendor/`는 `.gitignore` 대상이라 그 안의 증거 문서는 **커밋되지 않는다.**
  라이선스 증거가 버전 관리 밖에 있으면 릴리스 심사에서 인용할 수 없다.
  `release/licenses/`는 추적되고, 설치본에 `licenses/`로 함께 실린다.
- **인벤토리 계약에 `entrySelection`을 추가했다.** 설치된 앱은 `inventory-spec.json`을 갖고 있지 않은데
  `camera-helper`와 `edsdk-runtime`이 같은 디렉터리에 살면서 서로 다른 파일을 소유한다.
  선택 규칙이 인벤토리 안에 없으면 self-check가 무엇을 해시할지 알 수 없다.
- **사유 코드를 3개에서 6개로 늘렸다.** 스토리가 요구한 누락/불일치/미신고에 더해
  `inventory-not-staged`, `inventory-manifest-unreadable`, `inventory-pin-missing`을 두었다.
  "한 덩어리로 뭉치지 않는다"는 원칙을 더 좁게 적용한 것이고, 뭉친 것이 없다.
- **`src-tauri/build.rs`가 없는 페이로드 자리를 만든다.** `tauri-build`가 존재하지 않는
  `bundle.resources` 경로를 컴파일 에러로 만들기 때문에, 그대로 두면 신규 clone에서 `cargo test`조차
  돌지 않는다. 자리를 만들되 **내용을 지어내지 않고** `PAYLOAD-NOT-STAGED.md`와 `not-staged`
  인벤토리로 스스로 결손을 신고하게 했다. 실제 페이로드는 절대 덮어쓰지 않는다.
- **`release/dist/inventory.release.json` 봉인 단계를 추가했다.** 설치본 안에 동봉되는 인벤토리는
  자기 자신의 해시를 담을 수 없다. HV-18A가 설치본 해시를 대조하려면 봉인된 사본이 필요하다.

#### 고객 화면 무회귀

새 고객 문구 0개, 새 booth readiness 사유 코드 0개, 새 운영자 화면 0개.
proxy lane 기본값·384px 레일·렌더 인자·`final`의 비-tier 지위를
`src-tauri/tests/story_7_7_packaging.rs`가 테스트로 고정한다.
부팅 경로의 인벤토리 검사는 **반환값이 없어** 고객 흐름을 막을 수 없고, 실패는
기존 `release-governance` 감사 taxonomy로만 투영된다.

### File List

**신규**

- `docs/contracts/release-inventory.md`
- `release/README.md`
- `release/inventory-spec.json`
- `release/build-inventory.ts`
- `release/build-inventory.test.ts`
- `release/stage.ps1`
- `release/verify-inventory.ps1`
- `release/test-verify-inventory.ps1`
- `release/licenses/README.md`
- `release/licenses/darktable-5.4.1.md`
- `release/licenses/canon-edsdk-13.19.0.md`
- `release/licenses/canon-helper-dependencies.md`
- `release/licenses/webview2-runtime.md`
- `src/shared-contracts/schemas/release-inventory.ts`
- `src/shared-contracts/release-inventory.contracts.test.ts`
- `src-tauri/src/release/mod.rs`
- `src-tauri/src/release/inventory.rs`
- `src-tauri/src/release/self_check.rs`
- `src-tauri/tests/release_inventory.rs`
- `src-tauri/tests/story_7_7_packaging.rs`
- `tests/hardware/installer/hv-18a/README.md`
- `tests/hardware/installer/hv-18a/environment.md`
- `tests/hardware/installer/hv-18a/check-installer-evidence.ps1`
- `tests/hardware/installer/hv-18a/test-check-installer-evidence.ps1`
- `tests/hardware/installer/hv-18a/raw-image-extension-verdict.md`
- `tests/hardware/installer/run-20260817-123003-hv18a/**` (실행 증거와 `Go-candidate-with-waivers` gate)

**수정**

- `.github/workflows/release-windows.yml`
- `.gitignore`
- `README.md`
- `package.json`
- `release-baseline.md` (정본으로 가는 포인터로 축소)
- `docs/release-baseline.md` (정본)
- `src-tauri/tauri.conf.json`
- `src-tauri/build.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/render/mod.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/commands/viewer_commands.rs`
- `src/shared-contracts/schemas/index.ts`
- `src/governance/hardware-validation-governance.test.ts`
- `tests/hardware/resident-renderer/hv-16/decision.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/7-7-완전한-installer와-clean-offline-재현.md`

**인덱스에서 제거** (`.gitignore` 규칙 확장에 따른 정리, 파일은 디스크에 남는다)

- `sidecar/canon-helper/tests/CanonHelper.Tests/bin/**` (241개 산출물)
- `sidecar/canon-helper/tests/CanonHelper.Tests/obj/**`

---

## Change Log

| 날짜 | 변경 | 작성자 |
| --- | --- | --- |
| 2026-08-17 | Story 생성. 코드베이스 실사로 clean offline 결손 9건 확정, 승인 필요 결정 4건 기본안 확정 | Bob (SM) |
| 2026-08-17 | 구현 완료. 번들 정체성·오프라인 페이로드 동봉·`release-inventory/v1` 계약·기계식 검증 게이트·`--self-check`·CI·HV-18A 절차와 게이트·영향 문서 6종 갱신. `RawImageExtension`은 `offline-distribution: not-possible`로 판정해 닫음. 상태를 `review`로 올리되 HV-18A `Go` 전까지 `done`이 아니다. 물리 페이로드가 필요한 subtask 3건은 체크하지 않고 사유를 남김 | Amelia (Dev) |
| 2026-08-17 | HV-18A 직접 실행 시도. 전체 darktable·helper·EDSDK payload로 installer를 생성·봉인했고 EOS 700D preflight를 통과함. self-check의 외부 `certutil` 프로세스 폭증을 인프로세스 SHA-256으로 수정하고 전체 번들 검증을 19.109초·자식 프로세스 0개로 재실행함. clean VM·제품 서명·HV-17 Go 부재로 최종 판정은 실행 증거가 있는 `No-Go`, 상태는 `in-progress` 유지 | Codex |
| 2026-08-17 | Noah Lee 승인으로 서명·Canon EDSDK 재배포 증거·clean/offline 환경을 `waived-by-owner`로 변경하고 현재 PC lifecycle을 직접 실행함. 0.1.0 설치/실행/self-check, 0.1.1 업그레이드, 0.1.0 롤백, 인벤토리 누락 거부, 제거와 고객 파일 1,117개 보존은 통과. fixture sample 미표시, upgrade/rollback 후 기존 세션 읽기 직접 증거 부재, HV-17 No-Go에 따른 installed capture-bound pipeline 부재로 HV-18A는 결손 4건 `No-Go` 유지 | Codex |
| 2026-08-17 | 남은 결손을 직접 종료함. sample/proxy 회차 충돌을 분리하고 sample B 표시 증거를 확보했으며, Canon EOS 700D 실제 촬영의 RAW·display proxy·final을 같은 captureId로 봉인함. 세션 호환 self-check를 추가해 업그레이드·롤백에서 모두 통과했고 제거 후 고객 파일 1,152개 보존을 확인함. 촬영 준비 갱신 경쟁 조건도 수정·테스트했으며 HV-18A는 `Go-candidate-with-waivers`, Story는 `review`로 전환 | Codex |
| 2026-08-17 | 촬영 준비 갱신 수정까지 포함한 완전 설치본을 재생성하고 strict inventory 및 전체 자동시험을 통과함. 최신 패키지는 405,260,278바이트, SHA-256 `202518faf04be56d4b47fc1a7ba7e180366e08be551700bc637885e93b94140c`로 별도 보관해 lifecycle 실행에 사용한 패키지와 혼동하지 않음 | Codex |
| 2026-08-17 | 사용자가 승인한 C7/C8/C9 제외 범위에서 남은 실패 0건과 전체 검증 통과를 확인함. 기계식 `Go-candidate-with-waivers`를 보존하고 내부 제품 판정을 `Go-with-waivers`로 승인해 Story 상태를 `done`으로 전환 | Codex |
