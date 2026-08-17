# HV-18A — 완전한 installer 와 현재 PC lifecycle 재현 (Story 7.7)

> **2026-08-17 범위 변경:** Noah Lee 승인에 따라 이 PC에서 시험한다. C7 코드 서명,
> C8 네트워크 차단, C9 clean 환경은 `waivers.json`이 있을 때 `waived-by-owner`로 처리한다.
> Canon EDSDK 재배포 증거도 같은 승인으로 면제한다. 면제는 검증 통과가 아니며 공개 배포 적합성,
> 법적 재배포 권리, 새 PC 독립 설치 가능성을 증명하지 않는다. 이 변경이 아래의 기존
> clean/offline·서명 필수 설명보다 우선한다.

이 디렉터리는 Story 7.7 의 실장비 evidence 루트다.
`viewer-present/`, `capture-source/`, `display-proxy/`, `resident-renderer/`, `raw-refined/` 와
**섞지 않는다.**

## 이 gate 가 증명하는 것과 증명하지 않는 것

| | |
| --- | --- |
| **증명한다** | 승인된 현재 PC에서 설치 파일 하나로 viewer/camera/proxy/RAW/final 런타임이 재현된다는 것, 설치본 인벤토리가 실제 파일과 일치한다는 것, 업그레이드·롤백·제거가 고객 사진을 지키면서 끝난다는 것 |
| **증명하지 않는다** | 코드 서명, Canon EDSDK 재배포 권리, clean/offline 새 PC 독립 설치, 100-shot 성능, cold/idle/reconnect 회복 (→ **Story 7.8 / HV-18B**), 120fps+ 물리 frame 과 compositor→photon 오프셋 (→ **HV-18B 단독 소유**), 지점 승급·롤백 운영 (→ **Story 7.9 / HV-18C**) |

**이 회차의 롤백은 같은 PC 에 이전 설치본을 다시 까는 것이다.** 지점 승급 롤백과 혼동하지 않는다.

## 착수 전 선행 조건

| 조건 | 상태를 확인하는 곳 |
| --- | --- |
| Story 7.6 이 닫힌 빌드를 release candidate 로 쓴다 | `_bmad-output/implementation-artifacts/sprint-status.yaml` |
| HV-17A / HV-17B 가 `Go` | `hardware-validation-ledger.md` |
| 코드 서명 인증서 | 2026-08-17 책임자 면제 (`waivers.json`) |
| 벤더 페이로드(EDSDK, darktable 5.4.1)가 조달되어 있다 | `release/README.md` |

**7.6 이전 빌드로 이 회차를 돌지 않는다.** 인벤토리에 들어가는 실행 파일이 달라진다.

**미서명 설치본은 서명 통과로 적지 않는다.** 승인된 면제 파일이 있으면
`Go-candidate-with-waivers`의 현재 PC 내부 검증 근거로만 사용할 수 있다.

## 회차 디렉터리 구조

```
run-<timestamp>-hv18a/
├── environment.md                      # 아래 environment.md 템플릿을 채운다
├── result.md                           # 사람이 쓰는 판정문
├── prerequisites.json                  # Node/Rust/.NET SDK/darktable/WebView2 미설치 증거
├── network/
│   ├── adapters.txt                    # Get-NetAdapter 출력 원본
│   └── connectivity.json               # 연결 시도와 실패 기록
├── installer/
│   ├── inventory.release.json          # 봉인된 인벤토리 사본 (빌드 머신에서 가져온다)
│   └── installer.sha256.txt            # VM 에서 다시 계산한 설치본 해시
├── self-check/
│   ├── install-self-check.json         # v_a 설치 직후
│   ├── after-upgrade.json              # v_b 업그레이드 후
│   └── after-rollback.json             # v_a 재설치 후
├── lifecycle.json                      # 8단계 결과
├── data-preservation.json              # 업그레이드·롤백·제거의 데이터 보존 판정
├── fixture/
│   └── display.png                     # fixture 표시 화면 촬영 (사람이 검토한다)
├── pipeline/
│   ├── capture-evidence.json           # 실제 Canon 촬영 1건과 capture-bound 산출물 목록
│   └── artifacts/                      # raw-original / proxy / RAW 정밀본 / final 사본
└── gate.json                           # check-installer-evidence.ps1 출력
```

## 0. VM 준비

Windows 클린 이미지를 쓴다. 개발 도구가 하나라도 있으면 clean offline 회차가 아니다.

```powershell
# 사전 미설치 확인. 모두 "없음"이어야 한다.
Get-Command node -ErrorAction SilentlyContinue
Get-Command rustc -ErrorAction SilentlyContinue
Get-Command cargo -ErrorAction SilentlyContinue
& dotnet --list-sdks                       # 명령 자체가 없어야 한다
Get-Command darktable-cli -ErrorAction SilentlyContinue
Test-Path 'C:\Program Files\darktable'
Test-Path 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application'
reg query 'HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' /v pv
```

결과를 `prerequisites.json` 에 적는다:

```json
{
  "schemaVersion": "hv-18a-prerequisites/v1",
  "checkedAt": "2026-08-20T10:05:00Z",
  "absent": { "node": true, "rust": true, "dotnetSdk": true, "darktable": true, "webview2": true }
}
```

## 1. 네트워크 차단과 그 증거

**"인터넷이 없었다"는 주장이 아니라 측정이어야 한다.**

```powershell
Get-NetAdapter | Tee-Object -FilePath .\network\adapters.txt
Get-NetAdapter | Disable-NetAdapter -Confirm:$false
Get-NetAdapter | Format-Table Name, Status | Tee-Object -FilePath .\network\adapters.txt -Append

# 차단 확인. 모두 실패해야 한다.
Test-NetConnection -ComputerName 8.8.8.8 -InformationLevel Quiet
Test-NetConnection -ComputerName msedge.api.cdp.microsoft.com -Port 443 -InformationLevel Quiet
Resolve-DnsName github.com -ErrorAction SilentlyContinue
```

결과를 `network/connectivity.json` 에 적는다:

```json
{
  "schemaVersion": "hv-18a-network/v1",
  "verifiedAt": "2026-08-20T10:10:00Z",
  "adaptersDisabled": true,
  "probes": [
    { "target": "8.8.8.8", "method": "Test-NetConnection", "reachable": false },
    { "target": "msedge.api.cdp.microsoft.com:443", "method": "Test-NetConnection", "reachable": false },
    { "target": "github.com", "method": "Resolve-DnsName", "reachable": false }
  ]
}
```

WebView2 는 설치본이 오프라인 설치본을 자체 포함해 설치한다. 차단 상태에서 설치가 성공하는 것이
바로 그 사실의 증거다.

## 2. 8단계 lifecycle

| # | 단계 | 무엇을 확인하는가 |
| --- | --- | --- |
| 1 | `install` | v_a 설치본을 실행해 설치가 끝난다. **UAC 승격이 1회 발생한다** (`perMachine`). |
| 2 | `launch` | 앱이 뜨고 관람 창이 만들어진다. 단일 모니터 VM 이면 **관람 창이 창모드로 열리는 것이 정상이다**. |
| 3 | `self-check` | `boothy.exe --self-check` 가 종료 코드 `0` 으로 끝나고 보고서가 남는다. |
| 4 | `fixture-display` | `BOOTHY_DISPLAY_SAMPLE_MODE` 로 fixture 가 관람 화면에 표시된다. |
| 5 | `upgrade` | v_b 설치본을 그 위에 설치한다. 이전 버전 프로그램 파일이 사라진다. |
| 6 | `rollback` | v_a 를 다시 설치한다. **v_b 가 만든 세션이 v_a 에서 읽혀야 한다.** |
| 7 | `uninstall` | 제거한다. 프로그램 파일·번들 darktable·helper 트리·시작 메뉴 항목이 사라진다. |
| 8 | `data-preservation` | **세션 루트와 고객 사진이 살아 있다.** 사용자 데이터는 지우지 않는다. |

각 단계 실행 명령:

```powershell
# 1. 설치 (UAC 승격 1회)
.\Boothy_0.1.0_x64-setup.exe

# VM 에서 설치본 해시를 다시 계산한다 (빌드 머신 말을 믿지 않는다)
(Get-FileHash -Algorithm SHA256 .\Boothy_0.1.0_x64-setup.exe).Hash.ToLower() |
  Out-File .\installer\installer.sha256.txt -Encoding utf8 -NoNewline

# 2. 실행
& 'C:\Program Files\Boothy\Boothy.exe'

# 3. self-check (창을 만들지 않는다. 종료 코드로 말한다)
& 'C:\Program Files\Boothy\Boothy.exe' --self-check --report .\self-check\install-self-check.json
$LASTEXITCODE   # 0 통과 / 1 인벤토리 불일치 / 2 검사 불가

# 4. fixture 표시
$env:BOOTHY_DISPLAY_SAMPLE_MODE = 'visible-standby'
$env:BOOTHY_DISPLAY_PROXY_MODE = 'off' # proxy 우선순위와 충돌하지 않게 fixture 회차만 끈다
& 'C:\Program Files\Boothy\Boothy.exe'
# 새 세션 생성 → 프리셋 선택 → 사진 찍기까지 실행한다.
# 관람 화면에 SAMPLE A/B가 실제로 표시되면 fixture/display.png 로 남긴다.

# 4a. 실제 Canon 카메라 pipeline
# fixture mode를 끄고 proxy를 다시 켠 뒤 Canon 카메라로 1회 촬영한다. 같은 captureId에 귀속된
# raw-original / display proxy / final 결과가 모두 준비될 때까지 기다린다.
# RAW 정밀본은 HV-17 Partial 판정이 tier-not-justified이면 만들지 않고 그 gate를 함께 첨부한다.
Remove-Item Env:BOOTHY_DISPLAY_SAMPLE_MODE -ErrorAction SilentlyContinue
Remove-Item Env:BOOTHY_DISPLAY_PROXY_MODE -ErrorAction SilentlyContinue
& 'C:\Program Files\Boothy\Boothy.exe'
# 촬영 후 session manifest와 파일을 대조해 아래 pipeline 증거를 만든다.

# 5. 업그레이드
.\Boothy_0.1.1_x64-setup.exe
& 'C:\Program Files\Boothy\Boothy.exe' --self-check --verify-session .\self-check\session-under-test.json --report .\self-check\after-upgrade.json

# 6. 롤백 (같은 PC 에 이전 설치본 재설치)
.\Boothy_0.1.0_x64-setup.exe
& 'C:\Program Files\Boothy\Boothy.exe' --self-check --verify-session .\self-check\session-under-test.json --report .\self-check\after-rollback.json

# 7. 제거
# 설정 > 앱 > Boothy > 제거

# 8. 데이터 보존 확인
Test-Path "$env:USERPROFILE\Pictures\dabi_shoot"
(Get-ChildItem "$env:USERPROFILE\Pictures\dabi_shoot" -Recurse -File).Count
Test-Path 'C:\Program Files\Boothy'
Test-Path 'C:\Program Files\Boothy\darktable'
Test-Path 'C:\Program Files\Boothy\sidecar\canon-helper'
```

`lifecycle.json`:

```json
{
  "schemaVersion": "hv-18a-lifecycle/v1",
  "steps": [
    { "id": "install", "status": "pass", "observedAt": "2026-08-20T10:20:00Z", "notes": "UAC 승격 1회" },
    { "id": "launch", "status": "pass", "observedAt": "...", "notes": "단일 모니터 → 관람 창 창모드" },
    { "id": "self-check", "status": "pass", "observedAt": "...", "notes": "exit 0" },
    { "id": "fixture-display", "status": "pass", "observedAt": "...", "notes": "sample-a 표시" },
    { "id": "upgrade", "status": "pass", "observedAt": "...", "notes": "0.1.0 → 0.1.1" },
    { "id": "rollback", "status": "pass", "observedAt": "...", "notes": "0.1.1 → 0.1.0 재설치" },
    { "id": "uninstall", "status": "pass", "observedAt": "...", "notes": "" },
    { "id": "data-preservation", "status": "pass", "observedAt": "...", "notes": "" }
  ]
}
```

`data-preservation.json`:

```json
{
  "schemaVersion": "hv-18a-data-preservation/v1",
  "sessionRootPath": "C:\\Users\\booth\\Pictures\\dabi_shoot",
  "sessionRootSurvivesUninstall": true,
  "customerPhotoCountBeforeUninstall": 42,
  "customerPhotoCountAfterUninstall": 42,
  "sessionReadableAfterUpgrade": true,
  "sessionReadableAfterRollback": true,
  "programFilesRemoved": true,
  "bundledDarktableRemoved": true,
  "helperTreeRemoved": true,
  "startMenuEntryRemoved": true
}
```

`pipeline/capture-evidence.json`은 **하나의 실제 captureId**에 묶인 결과만 기록한다.
각 파일은 회차 디렉터리의 `pipeline/artifacts/` 아래로 복사하고 VM에서 sha256을 다시 계산한다.

```json
{
  "schemaVersion": "hv-18a-capture-evidence/v1",
  "captureId": "capture-actual-001",
  "requestId": "request-actual-001",
  "cameraCapture": { "status": "pass", "source": "canon-camera" },
  "artifacts": [
    { "role": "raw-original", "relativePath": "pipeline/artifacts/capture-actual-001.cr2", "sha256": "<64 hex>" },
    { "role": "display-proxy", "relativePath": "pipeline/artifacts/capture-actual-001-proxy.jpg", "sha256": "<64 hex>" },
    { "role": "raw-refined", "relativePath": "pipeline/artifacts/capture-actual-001-refined.jpg", "sha256": "<64 hex>" },
    { "role": "final", "relativePath": "pipeline/artifacts/capture-actual-001-final.jpg", "sha256": "<64 hex>" }
  ]
}
```

네 산출물 중 하나라도 없거나 비어 있거나 해시가 다르면 HV-18A는 `No-Go`다. fixture 화면만으로
camera/proxy/RAW/final 재현을 대신할 수 없다.

**제거가 고객 사진을 지우면 그 회차는 즉시 `No-Go` 다.** 되돌릴 수 없는 결함이다.

## 3. 기계식 게이트

```powershell
.\check-installer-evidence.ps1 -RunRoot ..\run-<timestamp>-hv18a -OutFile ..\run-<timestamp>-hv18a\gate.json
```

게이트 자체의 테스트:

```powershell
.\test-check-installer-evidence.ps1
```

**PASS 는 `Go` 가 아니다.** 게이트는 기계적으로 확인 가능한 것만 본다. fixture 표시 화면 검토와
승인자 서명은 사람이 한다.

## 4. `identifier` 변경의 영향

이 릴리스부터 앱 identifier 가 `com.tauri.dev` 에서 `com.boothy.booth` 로 바뀐다.

- 고객 사진과 세션 루트는 `%USERPROFILE%\Pictures\dabi_shoot` 에 있고 **움직이지 않는다**.
- `%LOCALAPPDATA%\<identifier>\` 는 세션 루트의 **폴백** 경로다. `USERPROFILE` 이 없는 환경에서만
  쓰이며, 그런 환경의 기존 세션은 새 identifier 아래에서 보이지 않게 된다.
- 개발 PC 에 남아 있던 `%LOCALAPPDATA%\com.tauri.dev\dabi_shoot\` 세션도 같은 이유로 보이지 않는다.

clean VM 회차에서는 이전 설치가 없으므로 이 항목은 관찰 대상이 아니다. 그러나 **기존 부스 PC 를
업그레이드하는 회차에서는 반드시 별도로 확인한다.**

## 5. 결정 7 — `Microsoft.RawImageExtension` 판정

HV-16 이 이 Story 로 넘긴 것은 채택이 아니라 질문이다. 판정문은
[`raw-image-extension-verdict.md`](./raw-image-extension-verdict.md) 가 소유한다.
