# HV-18A 환경 — 템플릿

> **이 파일은 아직 실행되지 않은 회차의 템플릿이다.**
> 회차를 돌 때 `run-<timestamp>-hv18a/environment.md` 로 복사해 채운다.
>
> **읽을 수 없는 값은 만들어내지 않고 `unknown` 으로 적는다.** 추정치를 적으면
> 그 숫자가 나중에 실측으로 인용된다.

실행: `<YYYY-MM-DD HH:MM +09:00>` · Story 7.7 완전한 installer 와 현재 PC lifecycle 재현

> 2026-08-17 책임자 승인: 코드 서명, Canon EDSDK 재배포 증거, clean/offline 환경은
> `waived-by-owner`다. 현재 PC의 실제 상태는 숨기지 않고 그대로 기록한다.

## 이 회차가 무엇이고 무엇이 아닌가

이 회차는 **설치 회차**다. 성능 회차가 아니다.

- 측정하는 것: 설치 → 실행 → self-check → fixture 표시 → 업그레이드 → 롤백 → 제거 → 데이터 보존,
  인벤토리와 디스크의 일치, 서명 상태, darktable 해석 출처
- **측정하지 않는 것: 100-shot 성능, cold/idle/reconnect 회복** → Story 7.8 / HV-18B
- **측정하지 않는 것: 120fps+ 물리 frame, compositor→photon 오프셋** → HV-18B 단독 소유
- **측정하지 않는 것: 지점 승급과 운영 롤백** → Story 7.9 / HV-18C

## 대상 PC

| 항목 | 값 |
| --- | --- |
| 종류 | `<현재 승인 시험 PC>` |
| OS | `<Windows 버전 / build / 아키텍처>` |
| 이미지 | `<클린 이미지 출처와 생성일>` |
| CPU | `<모델 / core / thread>` |
| RAM | `<GB>` |
| GPU | `<모델 / driver 버전>` |
| 디스크 여유 | `<설치 전 GB>` |

HV-15 · HV-17 과 **같은 PC 인지** 명시한다. 다르면 그 회차들의 기준선과 직접 비교할 수 없다.

## 사전 미설치 확인

| 항목 | 확인 명령 | 결과 |
| --- | --- | --- |
| Node | `Get-Command node` | `<없음 / 있음+버전>` |
| Rust | `Get-Command rustc` | `<없음 / 있음+버전>` |
| .NET SDK | `dotnet --list-sdks` | `<없음 / 있음+버전>` |
| darktable | `Test-Path 'C:\Program Files\darktable'` | `<없음 / 있음+버전>` |
| WebView2 | `reg query ... EdgeUpdate\Clients\{F3017226-...}` | `<없음 / 있음+버전>` |

이 값들은 현재 PC의 실제 조건을 기록하기 위한 것이며, clean 환경 통과를 주장하지 않는다.

## 네트워크 차단

| 항목 | 값 |
| --- | --- |
| 차단 방법 | `<어댑터 비활성 / 가상 스위치 분리 / 물리 케이블 분리>` |
| 어댑터 상태 | `<Get-NetAdapter 출력 요약>` |
| 연결 시도 1 | `<대상 / 방법 / 결과>` |
| 연결 시도 2 | `<대상 / 방법 / 결과>` |
| 연결 시도 3 | `<대상 / 방법 / 결과>` |

원본은 `network/adapters.txt` 와 `network/connectivity.json` 에 남긴다.

## 설치본

| 항목 | 값 |
| --- | --- |
| 파일명 | `<Boothy_<version>_x64-setup.exe>` |
| 크기 | `<MB>` **(결정 1 의 실측값)** |
| sha256 (빌드 머신) | `<봉인된 인벤토리의 installer.sha256>` |
| sha256 (VM 재계산) | `<Get-FileHash 결과>` |
| 서명 상태 | `<signed / unsigned>` |
| 서명 인증서 주체 | `<CN / 발급자 / 만료일>` |
| 타임스탬프 | `<서버 / 시각>` |
| 빌드 커밋 | `<git sha>` |
| 빌드 워크플로 실행 | `<GitHub Actions run URL 또는 로컬>` |

**미서명 설치본은 이 회차의 근거가 아니다.**

## 런타임 구성요소 실측

| 항목 | 값 | 어디서 읽었는가 |
| --- | --- | --- |
| 앱 버전 | `<0.1.0>` | self-check `appVersion` |
| identifier | `<com.boothy.booth>` | self-check `identifier` |
| 설치 경로 | `<C:\Program Files\Boothy>` | self-check `installRoot` |
| **WebView2 런타임 버전** | `<실측값>` | self-check `webview2Version` |
| **darktable 해석 출처** | `<bundled-resource 여야 한다>` | self-check `darktableResolution.source` |
| darktable 경로 | `<...\Boothy\darktable\bin\darktable-cli.exe>` | self-check `darktableResolution.binary` |
| darktable 버전 | `<5.4.1>` | `darktable-cli --version` |
| helper 버전 | `<canon-helper 0.1.0>` | self-check `helperVersion` |
| EDSDK 버전 | `<13.19.0>` | 인벤토리 `edsdk-runtime` |

## 실제 카메라 pipeline 증거

| 항목 | 값 |
| --- | --- |
| captureId / requestId | `<실제 촬영 식별자>` |
| 카메라 source | `canon-camera` |
| raw-original | `<파일명 / sha256>` |
| display proxy | `<파일명 / sha256>` |
| RAW 정밀본 | `<파일명 / sha256>` |
| final | `<파일명 / sha256>` |

네 파일은 같은 captureId에 귀속되어야 하며 `pipeline/capture-evidence.json`과 일치해야 한다.

**WebView2 버전은 설치본으로 고정할 수 없다** (v2 스키마에 `fixedRuntime` 이 없다).
그래서 회차마다 실측이 필요하다. Story 7.8 이 present 타이밍을 비교할 때 이 값을 참조한다.

**`darktableResolution.source` 가 `bundled-resource` 가 아니면 그 회차는 핀이 깨진 회차다.**

## 디스플레이 (fixture 표시 회차)

| 항목 | 값 |
| --- | --- |
| 모니터 수 | `<1 / 2>` |
| 고객 모니터 | `<모델 / 식별자>` |
| 해상도 | `<W×H>` |
| DPR | `<devicePixelRatio>` |
| 관람 창 모드 | `<전체화면 / 창모드>` |

**단일 모니터 VM 에서 관람 창이 창모드로 열리는 것은 정상이다**
(`MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK`). 결함으로 적지 않는다.

## 세션 데이터

| 항목 | 값 |
| --- | --- |
| 세션 루트 | `<C:\Users\...\Pictures\dabi_shoot>` |
| 업그레이드 전 사진 수 | `<n>` |
| 제거 후 사진 수 | `<n>` |
| 업그레이드 후 이전 세션 읽힘 | `<예 / 아니오>` |
| 롤백 후 세션 읽힘 | `<예 / 아니오>` |

## lane 상태 (인벤토리가 기록한 그대로)

| lane | 상태 | 근거 |
| --- | --- | --- |
| proxy (`BOOTHY_DISPLAY_PROXY_MODE`) | `<실제 Canon 회차 on / fixture 회차 off>` | Story 7.4 / HV-15 `Go`; sample과 동시 실행하면 proxy가 우선됨 |
| RAW 정밀본 (`BOOTHY_RAW_REFINED_MODE`) | `<off>` | HV-17 판정 |
| 상주 renderer (`BOOTHY_RESIDENT_RENDERER_MODE`) | `<off>` | HV-16 `Technology No-Go` |
| display sample (`BOOTHY_DISPLAY_SAMPLE_MODE`) | `<proxy를 끈 fixture 회차에서만 visible-standby>` | Story 7.2 |

**켜질 예정인 lane 을 켜져 있다고 적지 않는다.**

## 검증자

| 항목 | 값 |
| --- | --- |
| 실행자 | `<이름>` |
| 승인자 | `<이름>` |
| 서명 | `<서명 / 날짜>` |
