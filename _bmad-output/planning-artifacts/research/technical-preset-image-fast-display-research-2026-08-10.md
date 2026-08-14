---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments: []
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Boothy 프리셋 적용 사진의 최소 지연 표시 아키텍처'
research_goals: '전체 기술 아키텍처와 검증 테스트 이력을 파악하고, 프리셋 적용 사진을 최대한 빨리 화면에 표시하는 현재 방식이 최적인지 최신 기술 및 실측 근거로 검증한다. 사용자가 관측한 Lightroom Classic 3~4초대를 비교 목표로 삼되 Adobe가 공개하지 않은 내부 구현은 추정으로 구분한다.'
user_name: 'Noah Lee'
date: '2026-08-10'
web_research_enabled: true
source_verification: true
---

# Boothy 프리셋 사진 최소 지연 표시: 종합 기술 연구

**Date:** 2026-08-10–2026-08-11
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

이 연구는 Boothy의 실제 코드, EOS 700D 세션 진단 기록, 현재 및 과거 검증 브랜치, 테스트·설치·CI 이력과 Adobe·Canon·Microsoft·Tauri·darktable·LibRaw의 최신 공식 자료를 함께 분석했다. 비교 기준은 작은 thumbnail이나 파일 생성 시각이 아니라, 사용자가 요청한 **촬영 전에 준비된 관람 창에 화면 크기의 프리셋 적용 사진이 실제로 표시되는 시점**이다. Adobe가 공개하지 않은 Lightroom 내부 구현과 3~4초 수치는 사실이 아닌 외부 비교 목표로 분리했다.

핵심 결론은 현재 `main`이 정확한 RAW fallback은 제공하지만 최적 경로는 아니라는 것이다. 전용 관람 창이 없고 출력이 384px이며, 프리셋 미리보기는 RAW 전송 뒤 매 capture마다 새 darktable 프로세스로 만들어진다. 권장 구조는 `fast JPEG source → display-fit preset proxy → raw-refined display → final` 계층이며, 첫 성공 화면은 프리셋이 적용된 display-fit proxy여야 한다. 카메라 원본 thumbnail은 진행 피드백일 수 있지만 성공으로 세지 않는다.

새 실측은 EOS 700D CR2의 고해상도 embedded JPEG와 상주 display renderer가 유력한 개선 후보임을 보여주지만 아직 생산 증명은 아니다. LibRaw 결과는 CR2 두 샘플뿐이고, Lightroom과 동등한 button→monitor-frame head-to-head 측정도 남아 있다. 따라서 보고서는 입증된 현재 상태, 고신뢰 가설, 제품 release gate를 구분하고 30~100회 실장비 검증과 화질 승인 뒤에만 기본 경로를 승격하도록 제안한다. 최종 의사결정 요약은 문서 하단의 **Research Synthesis and Final Decision**에 있다.

---

<!-- Content will be appended sequentially through research workflow steps -->

## Technical Research Scope Confirmation

**Research Topic:** Boothy 프리셋 적용 사진의 최소 지연 표시 아키텍처
**Research Goals:** 전체 기술 아키텍처와 검증 테스트 이력을 파악하고, 프리셋 적용 사진을 최대한 빨리 화면에 표시하는 현재 방식이 최적인지 최신 기술 및 실측 근거로 검증한다. 사용자가 관측한 Lightroom Classic 3~4초대를 비교 목표로 삼되 Adobe가 공개하지 않은 내부 구현은 추정으로 구분한다.

**Technical Research Scope:**

- Architecture Analysis - design patterns, frameworks, system architecture
- Implementation Approaches - development methodologies, coding patterns
- Technology Stack - languages, frameworks, tools, platforms
- Integration Patterns - APIs, protocols, interoperability
- Performance Considerations - scalability, optimization, patterns

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Comprehensive technical coverage with architecture-specific insights

**Scope Confirmed:** 2026-08-10

## Technology Stack Analysis

### Runtime Topology

```text
React 19 / TypeScript / Vite / WebView2
        │  Tauri command + event, asset URL
        ▼
Tauri 2 / Rust host (상태·세션·큐·프로세스 조정)
        ├─ local JSON·JSONL·XMP·JPEG·RAW filesystem
        ├─ .NET 8 Canon EDSDK helper (capture/transfer/fast raster)
        └─ darktable-cli 5.4.1 (XMP preset pixel processing)
```

이 구조는 서버가 없는 Windows 로컬 우선 데스크톱 애플리케이션이다. Tauri가 Windows에서 OS WebView인 WebView2를 사용하고 Rust 코어와 WebView를 분리한다는 공식 구조와 일치한다. 사진 바이트를 JSON IPC로 복사하지 않고 로컬 경로만 전달한 뒤 WebView가 디코드한다. [Tauri Process Model](https://v2.tauri.app/concept/process-model/), [Tauri 개요](https://v2.tauri.app/start/)

### Programming Languages

- **TypeScript/TSX** — React 화면, 세션 상태, typed adapter/service, Zod 계약 검증을 담당한다. 실제 의존성은 React/React DOM 19.2.4, React Router 7.13.1, Zod 4.3.6, TypeScript 5.9.3이다. 근거: [`package.json`](../../../package.json) `:19-44`.
- **Rust 2021** — Tauri host, session/preset truth, helper 감독, render queue, 파일 원자적 승격과 오류 정규화를 담당한다. 실제 런타임은 Tauri 2.10.3이며 Rust 최소 버전은 1.77.2다. 근거: [`src-tauri/Cargo.toml`](../../../src-tauri/Cargo.toml) `:1-25`.
- **C#/.NET 8** — Canon EDSDK 세션, 셔터, 전송, 빠른 JPEG 추출을 격리한 Windows helper를 구현한다. `System.Drawing.Common 8.0.0`과 Canon의 C# interop source를 사용한다. 근거: [`CanonHelper.csproj`](../../../sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj) `:1-45`. `System.Drawing.Common`은 공식적으로 Windows 전용이므로 현재 Windows-only 제품 결정과는 정합적이다. [Microsoft .NET 호환성 문서](https://learn.microsoft.com/en-us/dotnet/core/compatibility/core-libraries/6.0/system-drawing-common-windows-only)
- **JSON/JSONL/XMP** — 세션 manifest, 진단/감사/branch 상태, helper request/event, 불변 preset bundle, darktable history stack의 경계 형식이다.

성능상 중요한 사실은 TypeScript·Rust·C# 중 어느 것도 프리셋 픽셀 파이프라인을 직접 실행하지 않는다는 점이다. 실제 색 처리와 RAW 현상은 매 작업마다 외부 프로세스로 실행되는 `darktable-cli`가 담당한다. 따라서 프리셋 표시의 초 단위 지연을 언어 런타임 비교만으로 설명할 수 없다.

### Development Frameworks and Libraries

- **UI/desktop:** React 19.2.4 + React Router 7.13.1 + Tauri API 2.10.1 + WebView2이다. Tauri `convertFileSrc`와 `protocol-asset`을 이용해 Pictures/AppLocalData의 로컬 이미지에 접근한다. 이는 Tauri가 문서화한 공식 로컬 파일 URL 경로다. 근거: [`preset-preview-src.ts`](../../../src/booth-shell/components/preset-preview-src.ts), [`tauri.conf.json`](../../../src-tauri/tauri.conf.json) `:23-31`; [Tauri `convertFileSrc`](https://v2.tauri.app/reference/javascript/api/namespacecore/#convertfilesrc).
- **UI image display:** 최신 사진은 일반 `<img>`에 `loading=eager`, `decoding=sync`, `fetchPriority=high`를 적용한다. 별도 Canvas, WebCodecs, `createImageBitmap`, `ImageDecoder` 또는 GPU image library는 없다. 근거: [`SessionPreviewImage.tsx`](../../../src/booth-shell/components/SessionPreviewImage.tsx) `:106-150`.
- **Host communication:** frontend→Rust는 Tauri command, Rust→frontend는 generic Tauri event다. 설계 문서의 ordered Channel과 달리 현재는 `emit/listen`이며, Canon helper 경계도 설계 문서의 stdio가 아니라 JSONL 파일 polling이다. 근거: [`capture_commands.rs`](../../../src-tauri/src/commands/capture_commands.rs) `:22-113`, [`capture-runtime.ts`](../../../src/capture-adapter/services/capture-runtime.ts) `:692-720`.
- **Camera:** .NET 8 helper + Canon EDSDK 13.19.0 + Windows Shell/GDI/System.Drawing이다. Canon은 EDSDK를 USB 유선 제어와 고속 이미지 전송용 SDK로 설명하므로 별도 native boundary 선택은 역할상 타당하다. [Canon CAP 개요](https://asia.canon/en/campaign/developerresources/camera/cap)
- **Preset renderer:** darktable-cli 5.4.1 + approved XMP, preview JPEG 384×384, `--hq false`, isolated config/library DB다. 공식 CLI가 XMP apply, 크기 제한, low-HQ export, custom preset DB 비활성화를 지원한다. 근거: [`render/mod.rs`](../../../src-tauri/src/render/mod.rs) `:20-28,839-915`; [darktable-cli 공식 문서](https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/).

현재 실행 경로에는 두 종류의 사진이 존재한다. Canon/Windows가 추출한 **빠르지만 프리셋 미적용인 JPEG**와, RAW 전체 전송 후 darktable이 생성한 **프리셋 적용 JPEG**다. fast JPEG에 XMP를 먼저 적용하는 speculative renderer는 구현돼 있지만 호출 연결이 없어 현재 실행되지 않는다. 이 사실은 후속 성능 분석의 핵심 입력이다.

### Database and Storage Technologies

- 제품 데이터베이스나 원격 object storage는 현재 없다. 기본 runtime root는 `%USERPROFILE%/Pictures/dabi_shoot`이며 각 세션은 `session.json`, `captures/originals`, `renders/previews`, `renders/finals`, `handoff`, `diagnostics`로 구성된다. 근거: [`session_repository.rs`](../../../src-tauri/src/session/session_repository.rs) `:441-446`, [`session_paths.rs`](../../../src-tauri/src/session/session_paths.rs) `:25-40`.
- 운영 감사, branch rollout, preset catalog state도 SQLite가 아니라 JSON 파일과 lock/backup 파일로 저장된다.
- darktable의 `.boothy-darktable/{preview|final}/library.db`는 image worker의 격리된 지원 상태이며 Boothy의 business truth DB가 아니다.
- 설계 문서에는 SQLite audit와 Tauri Store가 명시돼 있지만 실제 Cargo 의존성, migration, Store plugin은 없다. 따라서 현재 아키텍처 판단에서는 filesystem을 사실 기준으로 사용해야 한다. 근거: [`architecture.md`](../architecture.md) `:231-245`와 [`src-tauri/Cargo.toml`](../../../src-tauri/Cargo.toml)의 비교.

파일 기반 설계는 이미지 바이트를 IPC로 직렬화하지 않는다는 점에서 적절하다. 반면 helper request polling 250ms와 host event polling 10ms는 촬영 요청 pickup에 최대 약 250ms의 변동을 추가할 수 있어, 후속 통합·성능 단계에서 별도로 측정한다.

### Development Tools and Platforms

- **패키지/빌드:** pnpm 10.31.0, Node 기반 Vite 8.0.1, TypeScript project build, Cargo/Tauri CLI, .NET SDK/MSBuild를 사용한다. 현재 조사 PC는 Node 24.18.0, Rust 1.97.1, .NET SDK 8.0.423이며 `C:\Program Files\darktable\bin\darktable-cli.exe` 5.4.1을 실제로 해석한다.
- **Frontend test:** Vitest 4.1.0 + jsdom 29 + Testing Library. **Rust test:** Cargo 기본 test harness. **Helper test:** xUnit 2.9.3 + Microsoft.NET.Test.Sdk 17.14.1. 근거: [`package.json`](../../../package.json) `:26-44`, [`CanonHelper.Tests.csproj`](../../../sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj) `:1-20`.
- xUnit 공식 문서는 v2가 maintenance mode이며 신규 기능은 v3에 집중된다고 명시한다. 이는 유지보수 갭이지만 프리셋 표시 지연의 직접 원인은 아니다. [xUnit v2 공식 안내](https://xunit.net/docs/getting-started/v2/getting-started)
- **CI:** GitHub Actions의 단일 Windows workflow가 Tauri shell build만 수행한다. lint/test, Canon helper publish, EDSDK/darktable 주입, installer smoke test, artifact upload와 signing verification은 없다. 근거: [`release-windows.yml`](../../../.github/workflows/release-windows.yml).
- Playwright/Cypress 기반 end-to-end 또는 실제 WebView 화면 성능 회귀 테스트는 없다.

### Cloud Infrastructure and Deployment

- 런타임 cloud/API/backend는 없다. 확장 단위는 중앙 처리량이 아니라 개별 booth PC다.
- Tauri bundle 설정은 앱 shell만 포함하고 `externalBin`/resource로 Canon helper·EDSDK·darktable을 묶지 않는다. 현재 PC에는 darktable 5.4.1이 별도 설치돼 있어 개발 실행은 가능하지만 installer 단독 재현성은 보장되지 않는다.
- Windows UI는 WebView2 Runtime에 의존한다. Microsoft는 대부분의 앱에 자동 갱신되는 Evergreen Runtime을 권장한다. [WebView2 Evergreen vs Fixed](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/evergreen-vs-fixed-version)
- 따라서 현재 CI 성공은 “desktop shell을 빌드할 수 있음”을 뜻할 뿐 “촬영 및 프리셋 렌더가 가능한 booth installer”를 증명하지 않는다.

### Technology Adoption Trends

- **Tauri/Web stack:** 저장소의 Tauri 2.10.x보다 2.11.x가 공개돼 있고 npm 의존성도 일부 patch/minor가 뒤처져 있다. 다만 공식 자료에는 이 업그레이드가 사진 표시 시간을 줄인다는 근거가 없으므로 성능 대책 우선순위는 낮다. [Tauri release index](https://v2.tauri.app/release/)
- **darktable:** 저장소 고정 5.4.1보다 5.6.0이 최신이다. 5.6으로 올리면 5.4의 library/config가 역호환되지 않는다는 upstream 경고가 있으므로 XMP 출력 동등성 및 cold/warm benchmark 없이 승격하면 안 된다. [darktable 5.6.0 release](https://github.com/darktable-org/darktable/releases/tag/release-5.6.0)
- **Canon:** 저장소의 EDSDK 13.19.0보다 공식 release note의 13.20.21이 최신이다. EOS 700D/x64 호환성, 배포 라이선스와 회귀 검증 후 업그레이드할 수 있으나 현재 표시 지연의 직접 해법이라는 근거는 없다. [Canon EDSDK release notes](https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note)
- **.NET:** .NET 8은 2026-11-10 지원 종료 예정이므로 .NET 10 LTS 전환 계획이 필요하다. 이것도 지원성 과제이지 즉각적인 렌더 속도 해법은 아니다. [Microsoft .NET lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/microsoft-net-and-net-core)
- **Lightroom Classic 비교:** Adobe 공식 자료로 확인되는 핵심은 embedded/minimal→Camera Raw standard/1:1의 계층형 preview, Camera Raw cache, SSD-local preview cache, GPU 기반 표시·조정·preview 생성, 작은 Smart Preview proxy다. [Adobe 성능 가이드](https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html), [GPU preview 생성](https://helpx.adobe.com/lightroom-classic/desktop/kb/gpu-preview-generation.html), [Smart Preview](https://helpx.adobe.com/lightroom-classic/help/lightroom-smart-previews.html). Adobe는 3~4초를 SLA로 공개하지 않았으므로 그 수치는 사용자 관측 목표로 취급한다. 공개 자료로부터 가능한 고신뢰 추정은 Lightroom의 빠른 체감이 **점진적 preview + 장수 프로세스/cache + GPU** 조합에서 온다는 것이다.

### Cross-Technology Analysis

현재 스택의 큰 방향, 즉 `camera fast raster → display-size preset preview → RAW final/refinement`는 Lightroom의 공개 전략과 유사하고 합리적이다. 그러나 실제 Boothy는 첫 단계와 두 번째 단계가 연결되지 않아 프리셋 적용 사진이 항상 RAW 전체 전송과 새로운 darktable-cli 프로세스를 기다린다. 따라서 React/Vite나 asset URL보다 카메라 proxy 확보, speculative preset wiring, darktable cold/warm/GPU 상태가 우선 조사 대상이다.

### Quality Assessment

- **높은 신뢰:** 저장소 버전, 실제 호출/파일 경계, DB·cloud 부재, helper/darktable installer 미포함, speculative renderer 호출 부재.
- **중간 신뢰:** 매 촬영 darktable process/OpenCL 초기화가 Lightroom 대비 주요 구조적 비용이라는 판단. 코드와 upstream 초기화 설명에 근거하지만 구간 실측이 필요하다.
- **사용자 관측 기준:** Lightroom Classic 3~4초. Adobe 공식 SLA나 공개 benchmark는 확인되지 않았다.
- **미해결:** EOS 700D에서 직접 embedded thumbnail의 실제 성공률, darktable OpenCL 활성 여부, cold/warm process 비용, `preview ready → img onLoad` UI tail. 후속 테스트 이력 및 실측 단계에서 검증한다.

## Integration Patterns Analysis

### API Design Patterns

현재 제품 경계는 `React invoke → Rust command → Canon helper → filesystem → darktable → Tauri event → React`다. 문제는 API의 존재가 아니라 **촬영 시작 명령과 수 초짜리 완료 대기가 한 동기 호출에 묶여 있다는 점**이다. `request_capture`는 helper가 RAW 도착을 알릴 때까지 반환하지 않으며, 그 내부에서 fast-preview 이벤트를 emit한다. Tauri 공식 문서상 동기 command는 main thread에서 실행되므로 오래 걸리는 작업에는 async command가 권장된다. 실제 WebView2에서 이 호출 중 asset URL fetch와 paint가 선행되는지는 검증되지 않았으며, 현재 자동 테스트는 Rust callback 시점만 확인한다. [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/)

권장 계약은 `begin_capture`가 즉시 `requestId`와 accepted 상태를 돌려주고, 카메라 왕복과 렌더는 background worker에서 계속되는 구조다. 각 요청은 다음 불변 키를 한 번만 확정해야 한다.

`(sessionId, requestId, captureId, presetId, presetVersion, sourceHash)`

진행 상태는 `captureAccepted → cameraPreviewReady → presetProxyReady → rawPersisted → rawRefinedReady → finalReady | failed`처럼 단조 증가해야 한다. 현재처럼 요청 시점의 preset 정보를 helper에 기록해 놓고 capture 저장 시 manifest의 최신 active preset을 다시 읽으면, 촬영 중 preset 변경이 끼어 다른 프리셋으로 귀속될 수 있다.

- **REST/GraphQL/Webhook:** 동일 PC의 first-visible hot path에는 추가 HTTP 계층이 필요 없으므로 부적합하다.
- **gRPC:** 렌더러가 별도 PC나 edge appliance로 분리될 때만 유력하다. 현재 부모 프로세스와 1개 helper 사이에는 과하다. [gRPC 개요](https://grpc.io/docs/what-is-grpc/introduction/)
- **Binary image IPC:** 채택하지 않는다. 이미지 바이트가 아니라 완성된 로컬 자산의 불변 경로와 version만 전달하는 현재 원칙이 맞다.

### Communication Protocols

현재 live transport는 설계 문서의 stdio가 아니라 session별 JSONL/status 파일 네 개다. helper는 기본 250ms 주기로 요청을 읽고 EDSDK event도 같은 loop에서 pump하며, Rust는 event JSONL을 10ms마다 읽는다. 따라서 요청 pickup에 0~250ms 변동이 생기고, Rust reader는 누적 event 파일을 반복 파싱한다. 이것은 최적은 아니지만 현재 4~6초대 병목의 전부도 아니다.

가장 적합한 같은-PC 통신은 **장기 실행 helper의 duplex stdio JSONL**이다. 부모 Tauri가 helper 하나를 소유하므로 연결·주소·ACL 관리가 단순하고, helper에는 이미 `--stdio` 출력과 flush 코드가 있다. 현재 supervisor가 이 옵션을 켜지 않고 stdout/stderr를 버릴 뿐이다. stdout/stderr는 교착을 막기 위해 전용 비동기 reader가 계속 drain해야 한다. Microsoft는 redirected child stdio가 anonymous pipe로 구현됨을 설명하고, Rust도 piped stream을 읽지 않으면 child가 block될 수 있음을 경고한다. [Microsoft redirected I/O](https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output), [Rust `Stdio`](https://doc.rust-lang.org/std/process/struct.Stdio.html)

권장 분리는 다음과 같다.

- **Live plane:** stdin/stdout의 correlated JSONL, 또는 독립 서비스가 필요할 때만 duplex Named Pipe.
- **Recovery/audit plane:** 기존 JSONL journal과 manifest를 비동기 기록하고 앱 재시작 시 snapshot으로 재조정한다.
- **Frontend plane:** 요청별 ordered Tauri Channel과 최신 snapshot command를 함께 쓴다. Tauri는 Event보다 Channel을 빠르고 ordered한 streaming data에 권장한다. 현재 이벤트 양이 적어 Channel 자체가 수 초를 줄이진 않지만, event/poll 역전과 요청 상관관계 오류를 줄인다. [Tauri Channels](https://v2.tauri.app/develop/calling-frontend/)

Named Pipe는 helper가 앱 수명과 독립적으로 재연결되거나 다중 client를 받아야 할 때만 우선한다. Windows Named Pipe는 duplex/비동기 통신을 지원하지만 이름, reconnect, ACL이 추가된다. [Microsoft Named Pipes](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes)

### Data Formats and Standards

핵심 자산은 Canon CR2 RAW, 카메라/embedded JPEG, XMP preset, display-size JPEG, final JPEG다. 현재 무보정 fast JPEG, speculative JPEG, RAW refined JPEG가 같은 `{captureId}.jpg` 경로를 덮어쓰며, manifest에는 tier와 provenance가 충분히 남지 않는다. 이 구조는 WebView cache, 열린 파일 교체, stale worker 결과가 섞일 위험을 만든다.

같은 촬영 안에서도 자산을 분리해야 한다.

| Tier | 목적 | 공개 조건 |
| --- | --- | --- |
| `originalFastPreview` | 즉각적인 동일 촬영 확인 | camera/embedded JPEG 완성 및 decode 검증 |
| `presetProxy` | 첫 프리셋 적용 표시 | proxy+XMP 렌더 프로세스 종료, 완전 decode, preset/source 상관관계 확인 |
| `rawRefined` | RAW 기반 화면 교체 | RAW+XMP 렌더 검증 및 품질 gate 통과 |
| `final` | 저장·인계 | full/HQ RAW 렌더 검증 |

각 tier는 request별 고유 staging 파일에 완전히 쓴 뒤 close하고, 같은 volume의 최종 불변 이름으로 승격한 다음에만 readiness를 보낸다. Windows는 기존 파일 교체에 `ReplaceFile`, 같은-volume 이동에 `MoveFileEx`를 제공한다. [ReplaceFile](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew), [MoveFileEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)

현재 dormant speculative adopter는 렌더 프로세스 종료를 기다리지 않고 JPEG header만 보고 파일을 채택할 수 있고, 출력명에 `requestId`가 없다. 따라서 호출부만 복구하면 partial JPEG나 이전 요청 결과를 올릴 수 있다. 신규 계약에는 `schemaVersion`, monotonic `sequence/revision`, stage, preset/source hash, asset generation을 포함하고, 마지막 불완전 JSONL line은 다음 append까지 보류해야 한다.

### System Interoperability Approaches

현재 실제 critical path는 다음과 같다.

```text
버튼
  → 동기 Tauri command
  → 250ms file-poll helper request
  → Canon thumbnail 시도 ──→ 무보정 JPEG event
  → RAW 전체 전송·file-arrived
  → manifest 저장
  → 새 darktable-cli(RAW + XMP)
  → 같은 preview 경로 교체
  → generic readiness event / polling
  → WebView <img>
```

제품 목표에 맞는 통합은 다음과 같다.

```text
버튼 → 즉시 ack + background capture
            ├─ small/embedded JPEG → priority preset-proxy worker → presetProxyReady → 화면
            └─ RAW transfer ──────→ RAW refinement ─────────────→ rawRefinedReady → 무중단 교체

live state: ordered Channel + monotonic snapshot
durability: async JSONL/manifest journal
image handoff: immutable local path, bytes IPC 없음
```

Canon EDSDK를 격리한 sidecar와 darktable을 renderer adapter로 둔 큰 경계는 타당하다. 다만 현재 EOS 700D 실측에서 direct camera thumbnail은 0/2회였고, Windows Shell fallback은 RAW 도착 뒤 0.83~1.69초 후에 생성됐다. 동시에 host는 `file-arrived`에서 round-trip을 닫고 곧 RAW render로 넘어가므로 이 늦은 proxy가 preset path에 합류하지 못한다. 가장 현실적인 입력 개선은 카메라를 RAW+small JPEG로 명시 설정하고 두 transfer object를 한 request에 pair하는 것이다. 장비가 이를 안정적으로 지원하지 않으면 LibRaw 같은 in-process extractor로 CR2 embedded preview를 꺼내는 A/B가 다음 선택이다. LibRaw는 RAW metadata와 embedded thumbnail 접근을 제공하지만 production-quality RAW rendering engine은 아니므로 proxy source 역할로 한정해야 한다. [LibRaw 개요](https://www.libraw.org/about)

Lightroom Classic 15.4는 지원 Canon camera의 tether 기본 transport를 새 PTP로 바꾸고 Canon SDK로 되돌리는 옵션을 제공한다. Adobe는 같은 15.4 release에서 Canon tethering이 더 빠르고 안정적이라고 발표했지만, PTP만의 독립 benchmark나 초 단위 개선 폭은 공개하지 않았다. 같은 EOS 700D·동일 설정·20회 조건에서 Lightroom의 PTP와 `Use Canon SDK`를 A/B하면 사용자가 관측한 3~4초 중 전송 계층의 기여도를 직접 분리할 수 있다. [Adobe tethered camera support](https://helpx.adobe.com/lightroom-classic/desktop/kb/tethered-camera-support.html), [Adobe 15.4 release notes](https://helpx.adobe.com/lightroom-classic/help/whats-new/release-notes.html)

API Gateway, service mesh, ESB, cloud message broker는 single-booth local hot path에는 이득보다 hop과 운영 복잡도를 늘리므로 현재 범위에서 제외한다.

### Microservices Integration Patterns

이 제품은 마이크로서비스가 아니라 **한 booth PC의 modular monolith + local sidecars**가 적합하다. 필요한 분리는 배포 서비스 수가 아니라 작업 우선순위다.

- 우선순위 FIFO: `preset proxy > RAW refinement > final > warm-up`.
- 같은 request의 proxy 성공 시 중복 fallback을 취소한다.
- renderer saturation을 즉시 제품 실패로 만들지 않고, bounded wait와 truthful fallback을 둔다.
- 외부 renderer가 반복 실패하면 circuit breaker로 proxy를 건너뛰고 검증된 RAW fallback으로 degrade한다.
- export, upload, final, 재처리는 background queue에 둘 수 있지만 first-visible preset path는 일반 queue 뒤에 세우지 않는다. Microsoft도 queue-based load leveling이 minimal latency 응답에는 적합하지 않을 수 있다고 명시한다. [Queue-Based Load Leveling](https://learn.microsoft.com/en-us/azure/architecture/patterns/queue-based-load-leveling)

Saga, service discovery, distributed transaction은 이 로컬 촬영 경로에 해당하지 않는다. 렌더가 별도 edge PC로 이동할 때만 gRPC streaming, circuit breaker, authenticated service discovery를 다시 평가한다.

### Event-Driven Integration

현재 fast-preview event와 180ms/1.2s readiness polling은 같은 React state를 갱신하지만 revision이 없다. 늦게 끝난 poll이 더 최신 event를 되돌릴 수 있고, in-flight `camera-preparing` 상태는 active request를 무효화해 fast preview를 지울 수 있다. listener 등록 실패도 조용히 polling fallback으로 내려가 조기 표시 이점을 잃는다.

이벤트 상태기는 다음 원칙이 필요하다.

1. `sessionId/requestId/captureId`와 monotonic `revision/stage`를 모두 비교한다.
2. 더 낮은 revision이나 quality tier는 현재 표시 자산을 지우거나 되돌릴 수 없다.
3. 정상적인 in-flight `camera-preparing`은 capture cancellation과 구분한다.
4. Channel 유실·reload 시 snapshot을 읽어 수렴한다.
5. `assetReady`, Channel 수신, WebView decode, double-`requestAnimationFrame` paint, viewport intersection을 별도 시각으로 기록한다.

전체 event sourcing이나 외부 broker는 불필요하다. requestId 기반 append journal은 진단과 replay에 충분하며, live UI는 ordered Channel과 snapshot reducer가 책임지는 가벼운 CQRS가 적합하다.

### Integration Security Patterns

현재 Tauri asset scope를 Pictures/AppLocalData 아래로 제한한 것은 옳다. 반면 `csp: null`, 넓은 path 문자열 처리, helper 실행 전 session 검증 부재는 성능과 별개의 보안 부채다.

- session/capture/preset ID를 allowlist 형식으로 검증하고 canonical path가 지정 root 안인지 확인한다. 문자열 prefix만으로 `..` 또는 junction escape를 막아서는 안 된다.
- live message의 schema/version, request correlation, allowed state transition을 검증한다.
- stdio는 부모-자식 1:1 범위로 노출을 최소화한다. Named Pipe를 택하면 logon SID 기반 ACL과 remote access 차단을 적용한다.
- Tauri capability는 capture/readiness/channel과 좁은 asset scope만 허용하고 CSP를 명시한다.
- installer가 helper, EDSDK, renderer를 버전 고정·서명·무결성 검증과 함께 배포해야 한다. 현재 shell-only CI는 이 보안·운영 경계를 증명하지 않는다.

### Integration Test and Evidence Assessment

| 증거 | 결과 | 해석 |
| --- | --- | --- |
| 2026-08-10/11 현 PC, EOS 700D 2컷 | direct thumbnail `0/2`; RAW 약 `1.38~1.49s`; RAW 뒤 Shell JPEG `0.83~1.69s`; RAW darktable preset render `4.839~5.767s` | 현재 `main`은 3~4초 목표의 최적 경로가 아니다. |
| 과거 크기 축소 실험 | full `8.652s`, 1280px `5.973s`, 640px `6.894s` | 단순 출력 크기 축소만으로 해결되지 않았고 cold/process/pixelpipe 영향이 크다. |
| 2026-08-11 frontend 관련 6 files | `119/119` pass | correlation, polling, cache URL, image priority는 보호하지만 실제 Tauri/WebView paint는 검증하지 않는다. |
| current Rust overlap test | pass지만 speculative starter를 호출하지 않음 | 테스트 이름과 달리 RAW와 proxy 렌더 overlap을 증명하지 않는 false-positive다. |
| 미병합 원격 branch Story 1.26 | EOS 700D `5/5`, 공식 `originalVisibleToPresetAppliedVisibleMs=2387~2480ms` | 현재 `main`보다 빠른 검증 후보지만, 버튼/셔터→표시 E2E가 아니고 다른 PC의 증거이며 현재 branch에 병합되지 않았다. |
| 미병합 원격 branch Story 1.9 | EOS 700D `5/5`, 같은 공식 지표 `2882~2979ms` | 3초 gate를 통과한 과거 경로가 있었음을 보여 주지만 Lightroom 3~4초와 동일 지표로 비교할 수 없다. |

원격 branch의 성공 이력을 발견한 것은 중요하다. 새 엔진을 처음부터 만들기 전에 `Story 1.26`의 exact diff, preset truth, 프로세스 실행 방식과 측정기를 현재 `main` 옆 shadow lane으로 복원해 동일 PC·동일 프리셋·동일 E2E 지표로 재실행하는 것이 가장 빠른 조사 순서다. 문서상 “resident”라는 이름과 달리 실제 pixel render가 매번 `darktable-cli` 프로세스를 새로 실행한 흔적도 있으므로, 명칭이 아니라 process telemetry와 출력 parity로 판단해야 한다.

### Integration Decision

현재 통합은 정확한 RAW fallback과 로컬 path handoff라는 기반은 좋지만 최적은 아니다. **첫 프리셋 사진을 빠르게 만드는 결정적 변경은 transport만 바꾸는 것이 아니라, same-capture proxy 확보 즉시 안전한 preset-proxy render를 RAW 전송과 병렬 실행하고 그 결과를 실제 WebView까지 비동기 전달하는 것**이다. JSONL→stdio/Channel 전환은 주로 수백 ms와 순서 안정성을 개선하며, Lightroom 수준을 겨냥한 초 단위 이득은 proxy overlap, warm renderer/cache/GPU, 그리고 과거 3초 gate 경로의 재현에서 찾아야 한다.

## Architectural Patterns and Design

### System Architecture Patterns

Boothy에 가장 맞는 형태는 cloud microservices가 아니라 **Windows 로컬 modular monolith + app-lifetime sidecars + progressive preview pipeline**이다. Rust host가 capture lifecycle의 단일 mediator이자 state owner가 되고, Canon transport와 pixel renderer는 교체 가능한 adapter로 둔다. 이 구조는 UI나 실제 장비 없이도 core use case를 검증하고 외부 기술을 adapter로 교체한다는 Ports & Adapters 원칙과 맞는다. [Alistair Cockburn의 Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture)

```text
React/WebView
  │ async begin_capture + ordered progress Channel
  ▼
Rust CaptureCoordinator ───────── durable snapshot/journal
  ├─ CameraPort ── EDSDK adapter / PTP research adapter
  ├─ P0 ProxyRenderer ── small JPEG + approved preset plan
  ├─ P1 ExactPreviewRenderer ── RAW + full XMP
  └─ P2 FinalRenderer ── RAW + full/HQ
            │
            ▼
    immutable local artifacts
            │ asset URL
            ▼
       WebView image paint
```

후보별 판정은 다음과 같다.

| 후보 | 강점 | 한계 | 판정 |
| --- | --- | --- | --- |
| 현재 `main`의 RAW 직렬 경로 | RAW/full-preset truth가 단순함 | proxy overlap 부재, late fallback 누락, 촬영별 CLI | 정확성 fallback으로 유지 |
| 미병합 Story 1.26 선별 복원 | 실제 EOS 700D 5/5와 3초 이내 교체 기록, correlation/worker 개선 | 지표가 button E2E가 아니며 여전히 per-capture CLI | 가장 먼저 재현할 기준선 |
| JPEG preset proxy → RAW refinement | 가장 빠른 first preset display, RAW와 병렬화 | proxy/RAW 시각 동등성 계약 필요 | 목표 아키텍처 |
| 진짜 상주 full-preset 엔진 | 초기화 상각과 일관된 p95 가능 | darktable public daemon API가 없어 개발·업그레이드 위험 큼 | 4초 gate 실패 시 조건부 연구 |
| LUT/간소화 GPU renderer 단독 | 매우 낮은 latency 가능 | arbitrary XMP, lens/denoise/mask 완전 재현 불가 | proxy-compatible preset에만 사용 가능 |

Adobe가 공개한 Lightroom 구조도 카메라 embedded preview, 화면 크기의 Standard preview, Camera Raw cache, Smart Preview, GPU preview generation처럼 여러 품질 tier를 분리한다. tether 내부 scheduler는 비공개지만, 하나의 고품질 렌더가 끝날 때까지 UI를 비워 두는 제품은 아니라는 점은 분명하다. [Adobe import preview options](https://helpx.adobe.com/lightroom-classic/desktop/import-photos/photo-video-import-options.html), [Adobe Smart Previews](https://helpx.adobe.com/lightroom-classic/desktop/viewing-photos/lightroom-smart-previews.html)

_Source: [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture), [Adobe preview options](https://helpx.adobe.com/lightroom-classic/desktop/import-photos/photo-video-import-options.html), [Tauri process model](https://v2.tauri.app/concept/process-model/)_

### Design Principles and Best Practices

첫째 원칙은 **truthful progressive rendering**이다. `previewReady` 하나로 모든 상태를 표현하지 않고 다음 product truth를 구분한다.

- `cameraProxy`: 같은 촬영임은 확인됐지만 프리셋 미적용. preset SLA 성공으로 세지 않는다.
- `presetProxy`: 선택된 preset의 승인된 proxy plan을 적용한 첫 고객 표시 자산.
- `rawPresetPreview`: RAW와 전체 XMP를 사용한 정확한 화면용 결과.
- `final`: 저장·인계용 full/HQ 결과.

둘째는 **preset을 next-capture contract로 고정**하는 것이다. 촬영 시작 시 preset ID/version/hash와 proxy compatibility를 snapshot하고 이후 active preset 변경이 해당 capture에 영향을 주지 못하게 한다.

셋째는 **approximation을 숨기지 않는 것**이다. 게시된 preset마다 `proxyCompatible`, supported operation set, proxy recipe version, reference renderer/version과 시각적 승인 결과를 기록한다. LUT·전역 tone/color로 안전하게 표현 가능한 preset은 proxy lane에 태우고, lens correction·denoise·dehaze·mask 등 차이를 승인하지 못한 preset은 RAW exact lane만 사용한다. 과거 XMP trimming의 2.8초 결과가 look-affecting module 제거 때문에 comparison evidence로 강등된 이력이 이 원칙을 뒷받침한다.

넷째는 **Strangler-style selective migration**이다. 수백 파일이 갈라진 Story 1.26 branch를 통째로 병합하지 않고, current main 옆 shadow lane에 correlation, speculative handoff, timing, scheduler만 선별 재구현한다. 같은 입력을 두 lane에 보내 품질·시간을 비교한 뒤 성공한 preset/booth부터 전환한다. [Microsoft Strangler Fig pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig)

다섯째는 **측정 기준을 제품 화면으로 고정**하는 것이다. 공식 KPI는 `trusted button pointer-up → 같은 capture/preset의 preset-applied image가 decode 후 next animation frame에 paint`다. 현재 `captureAcknowledgedAtMs`는 버튼 시점이 아니고 `previewVisibleAtMs`도 파일 준비 시점이므로 이름과 의미를 바로잡아야 한다. W3C User Timing은 같은 context 안에서 monotonic high-resolution 측정을 제공한다. [W3C User Timing Level 3](https://www.w3.org/TR/user-timing-3/)

_Source: [Microsoft Strangler Fig](https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig), [W3C User Timing](https://www.w3.org/TR/user-timing-3/), [Adobe non-destructive editing](https://helpx.adobe.com/lightroom-classic/desktop/help/applying-adjustments-develop-module-basic.html)_

### Scalability and Performance Patterns

이 제품의 scaling 문제는 서버 horizontal scale이 아니라 **한 booth의 deadline latency와 resource isolation**이다. 평균 처리량보다 현재 고객이 기다리는 한 장을 우선해야 한다.

- P0: 현재 촬영의 preset proxy 한 작업만 최우선 실행.
- P1: preset proxy가 보인 뒤 RAW refinement.
- P2: final/export/warm-up/과거 capture 재처리는 session idle에서 실행.
- 새 capture 또는 preset generation이 오면 stale P0/P1을 취소하고 결과 승격을 금지한다.
- 단순 `in-flight <= 2` 대신 priority FIFO와 class별 capacity 1을 사용한다.
- Windows child process tree는 Job Object로 묶어 취소 시 확실히 종료한다. preview는 `ABOVE_NORMAL`부터 검증하고, 과거 실측상 이득이 없었던 `HIGH`는 사용하지 않는다. Microsoft도 high priority가 다른 thread의 CPU를 소진할 수 있다고 경고한다. [Windows process priority](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setpriorityclass), [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

현재 darktable-cli는 입력을 받고 export한 뒤 종료하는 one-shot 도구이며 daemon/stdin job API를 제공하지 않는다. 따라서 Rust worker thread나 idle process pool이라는 이름만으로 pixel engine이 resident가 되지 않는다. OpenCL도 프로세스 시작 때 device 확인, context/pipeline, source/kernel 준비를 수행하므로 현재 1×1 PNG warm-up은 실제 JPEG render state를 유지하지 못한다. [darktable-cli](https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/), [darktable OpenCL activation](https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/)

단기 cache 대상은 촬영 결과가 아니라 반복되는 static work다.

- preset publish/select 시 proxy execution plan, LUT, XMP parse 결과, color profile을 준비한다.
- stable renderer config/cache directory와 실제 256~384px JPEG recipe로 warm-up한다.
- unique shot output은 다른 촬영에 재사용하지 않는다.
- 256/384px JPEG proxy는 과거 CPU-only 결과를 기준선으로 삼고, CPU/OpenCL을 cold/warm 10~20회 A/B한 뒤 선택한다.
- RAW refinement/final에서 GPU가 더 유리할 수 있지만, concurrent CLI로 CPU/I/O/VRAM 경쟁을 만들지 않는다.

과거 branch에서 256px JPEG proxy 변환은 약 2.8초였지만 전체 capture-to-ready는 약 5.38~5.51초였다. 따라서 Lightroom 3~4초 목표는 proxy render만 줄여서는 안 되고, camera JPEG를 촬영 후 가급적 0.5~1초 안에 확보한다는 가설도 함께 검증해야 한다. 이는 보장치가 아니라 다음 A/B의 설계 budget이다. 제안 product gate는 warm 30-shot 기준 성공 30/30, p50≤3.0초, p95≤4.0초, max≤5.0초다.

_Source: [darktable CLI](https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/), [darktable OpenCL](https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/), [Adobe performance guide](https://helpx.adobe.com/lightroom-classic/desktop/technical-support/performance-guidelines/optimize-performance-lightroom.html), [Adobe GPU preview generation](https://helpx.adobe.com/lightroom-classic/desktop/kb/gpu-preview-generation.html)_

### Integration and Communication Patterns

제어는 동기, 작업은 비동기인 **command + ordered progress + durable reconciliation** 혼합형이 적합하다.

1. UI가 `begin_capture`를 호출하면 Rust가 request contract를 저장하고 즉시 ack한다.
2. app-lifetime CameraAdapter가 촬영을 진행하고 작은 JPEG와 RAW를 같은 capture에 correlate한다.
3. helper와 host의 live control은 persistent duplex stdio를 사용하고 JSONL file은 audit/recovery로 비동기 유지한다.
4. Rust가 요청별 Tauri Channel로 revision이 있는 stage update를 보낸다.
5. WebView는 JPEG bytes가 아니라 immutable asset path를 받아 직접 읽는다.
6. reload/listener loss/gap이 있으면 manifest snapshot으로 수렴한다.

Tauri는 무거운 작업에 async command를 선호하며, sync command는 별도 표시가 없으면 main thread에서 실행한다. 또한 빠르고 ordered한 streaming에는 Channel을 권장한다. [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/), [Tauri Channels and events](https://v2.tauri.app/develop/calling-frontend/)

Canon transport는 별도 port로 둬야 한다. Lightroom Classic 15.4는 EOS 700D를 지원하면서 Canon 기본 tether를 새 PTP로 바꿨고 Canon SDK fallback을 남겼다. 같은 release에서 Adobe는 Canon tether가 더 빠르고 안정적이라고 발표했다. 이는 PTP research adapter를 만들 근거지만, 독립 성능 수치가 없으므로 동일 PC/카메라/RAW 설정에서 Lightroom PTP와 Canon SDK를 먼저 A/B하고 자체 구현 여부를 결정한다. [Adobe tether support](https://helpx.adobe.com/lightroom-classic/desktop/kb/tethered-camera-support.html), [Adobe 15.4 release notes](https://helpx.adobe.com/lightroom-classic/help/whats-new/release-notes.html)

REST, GraphQL, cloud broker, API gateway, service mesh는 이 same-machine hot path에 추가하지 않는다. 외부 edge renderer가 실제로 도입될 때만 authenticated gRPC/streaming을 다시 평가한다.

_Source: [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/), [Tauri frontend communication](https://v2.tauri.app/develop/calling-frontend/), [Adobe tether support](https://helpx.adobe.com/lightroom-classic/desktop/kb/tethered-camera-support.html)_

### Security Architecture Patterns

속도 경로도 least privilege와 provenance를 유지해야 한다.

- `sessionId/requestId/captureId`를 allowlist 형식으로 검증하고 canonicalized path가 session root 안인지 확인한다.
- 모든 message에 schemaVersion, processEpoch, sequence/revision, preset/source hash를 포함한다.
- WebView asset scope는 Pictures 전체가 아니라 실제 preview JPEG와 필요한 preset card asset으로 좁힌다.
- `csp: null`을 제거하고 bundled self, IPC, asset image source만 허용한다. Tauri는 CSP를 가능한 제한적으로 설정하고 trusted asset만 허용하라고 권장한다. [Tauri CSP](https://v2.tauri.app/security/csp/), [Tauri asset protocol](https://v2.tauri.app/security/asset-protocol/)
- capture/operator/authoring command를 window별 Tauri capability로 분리하고 host에서도 window label을 검증한다. [Tauri Capabilities](https://v2.tauri.app/security/capabilities/)
- stdio는 parent-child 1:1 경계를 이용한다. Named Pipe가 필요하면 current logon SID 전용 ACL, local-only, nonce handshake를 사용한다.
- published immutable preset만 hot path에서 실행하고, renderer/helper/EDSDK의 version과 hash를 boot preflight에서 확인한다.

_Source: [Tauri CSP](https://v2.tauri.app/security/csp/), [Tauri Capabilities](https://v2.tauri.app/security/capabilities/), [Microsoft named-pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)_

### Data Architecture Patterns

사진 bytes는 filesystem에, 상태 truth는 versioned manifest에 두는 현재 큰 방향을 유지한다. 초 단위 성능을 위해 DB가 필수는 아니다. 다만 같은 `{captureId}.jpg`를 계속 덮어쓰는 방식은 제거해야 한다.

권장 artifact key:

`sessionId / captureId / requestId / presetVersion / renderProfile / generation`

권장 commit 순서:

1. request별 unique temp에 write.
2. process 종료와 완전 JPEG decode를 확인.
3. immutable final filename으로 same-volume rename.
4. manifest revision과 `activeDisplayArtifactId` pointer를 짧은 single-writer transaction으로 commit.
5. commit된 revision/path만 Channel로 publish.

manifest에는 tier, generation, preset bundle/XMP hash, renderer/version/mode, source kind/hash, readyAt, byte size를 기록한다. `session.json`은 durable business truth, helper status는 live projection, Channel/event는 transient notification, timing JSONL은 observability로 역할을 고정한다. startup reconciler는 invalid primary→backup 복구, stale staging/lock 정리, RAW/proxy가 존재하는 `previewWaiting` resume, accepted-but-no-terminal capture의 `indeterminate` 처리를 담당한다. 물리 shutter 이후 불명확한 요청을 자동 재촬영해서는 안 된다.

SQLite는 향후 다량 metadata query나 stronger transaction이 필요할 때 WAL 기반 metadata index로 추가할 수 있지만 image blobs나 first-display critical path에 넣지 않는다. 지금은 immutable files + atomic pointer + revisioned journal이 가장 작은 안전한 변화다.

_Source: [Microsoft moving and replacing files](https://learn.microsoft.com/en-us/windows/win32/fileio/moving-and-replacing-files), [Microsoft FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers), [Tauri asset protocol](https://v2.tauri.app/security/asset-protocol/)_

### Deployment and Operations Architecture

현재 installer/CI는 앱 shell만 보장하므로 성능 아키텍처의 일부가 배포에서 빠져 있다. Canon helper, EDSDK, renderer, color profile, proxy plan compiler를 version-pinned runtime unit으로 배포해야 한다. Tauri는 sidecar를 `bundle.externalBin`으로 포함하는 공식 경로를 제공한다. [Tauri sidecars](https://v2.tauri.app/develop/sidecar/)

검증은 세 층으로 나눈다.

| Gate | 필수 항목 |
| --- | --- |
| PR | TS/Rust/.NET test, lint, helper publish, bundle inventory, preset proxy parity fixture |
| Release candidate | clean Windows VM 설치/제거, helper self-check, 실제 EDSDK/darktable version/hash, sample render, signing/attestation |
| Approved booth hardware | EOS 700D, 실제 GPU/driver/USB/power plan에서 cold/warm/reconnect/30-shot p50·p95·max |

제품 SLA 후보는 `button pointer-up → presetProxy 또는 rawPresetPreview의 next-frame paint`이며 cameraProxy는 성공으로 세지 않는다. warm 30-shot `30/30`, p50≤3초, p95≤4초, max≤5초를 목표로 하고, cold first-shot 5회와 10분 idle/reconnect 후 10회도 별도로 관리한다. 이는 Adobe 공식 SLA가 아니라 Lightroom 3~4초 사용자 관측을 Boothy release gate로 바꾼 제안이다.

가장 빠른 rollout은 다음 네 단계다.

1. Story 1.26의 correlation, truthful route evidence, timing과 병렬 worker를 current main 옆 shadow lane으로 선별 복원한다.
2. EOS 700D에서 Lightroom 15.4 PTP/Canon SDK와 Boothy를 동일 `button→paint` 지표로 다시 잰다.
3. RAW+small JPEG pairing과 LibRaw embedded JPEG를 A/B해 더 빠르고 안정적인 proxy source를 채택하고, preset proxy→RAW refinement를 출시한다.
4. display-fit darktable one-shot이 현재 실측에서 목표를 넘었으므로 resident CPU/GPU proxy engine을 짧은 Phase 4 spike로 즉시 검증한다. 다만 full production 투자는 pre-created viewer, fast-source race, immutable handoff를 같은 KPI로 측정한 뒤 결정한다. darktable CLI process pool은 공식 job API가 없어 투자 대상에서 제외한다.

Windows 패키지는 서명되어야 하며 clean deployment는 운영 재현성을 높인다. GitHub self-hosted runner는 EOS 700D/GPU가 연결된 승인 PC를 label로 분리해 hardware gate를 실행할 수 있다. [Microsoft Windows packaging](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/), [MSIX signing](https://learn.microsoft.com/en-us/windows/msix/package/signing-package-overview), [GitHub self-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/use-in-a-workflow)

_Source: [Tauri sidecars](https://v2.tauri.app/develop/sidecar/), [Windows packaging](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/), [GitHub self-hosted runner](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/use-in-a-workflow)_

### Architecture Decision Summary

권장 ADR은 여섯 가지다.

1. **KPI:** `button → preset-applied next-frame paint`를 공식 지표로 채택한다.
2. **Migration:** Story 1.26을 전체 merge하지 않고 검증된 seam만 shadow lane으로 선별 복원한다.
3. **Preview policy:** `cameraProxy → approved presetProxy → rawPresetPreview → final` 계층을 명시한다.
4. **Scheduler:** 현재 촬영 preset proxy 한 작업을 최우선으로 하고 stale work를 취소한다.
5. **Renderer:** darktable RAW path는 parity oracle/fallback/final로 유지한다. 현재 one-shot display-fit 실측이 목표를 넘으므로 true resident proxy engine의 작은 spike는 진행하되, 본개발 승격은 실제 viewer KPI와 화질 gate 통과 후 결정한다.
6. **Transport:** Lightroom 15.4 PTP와 Canon SDK를 동일 EOS 700D에서 A/B한 뒤 camera adapter 투자를 결정한다.

이 구조에서 가장 중요한 판단은 “darktable을 더 빠르게 호출할 것인가”가 아니라 **“어떤 자산을 언제부터 고객에게 프리셋 적용 결과라고 정직하게 보여줄 수 있는가”**다. 현재 `main`은 정확한 fallback이지만 최적 경로는 아니며, 과거 branch는 유력한 복구 기준선이지만 아직 Lightroom과 같은 E2E 지표를 통과한 최종 해답도 아니다.

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

사용자가 요구한 제품 경험은 작은 thumbnail rail이 아니다. **촬영 전에 이미 열려 있는 관람 창의 주 이미지 영역에, 최종 RAW가 아니더라도 화면 크기로 충분히 선명하고 프리셋이 적용된 사진이 거의 즉시 나타나는 것**이다. 따라서 첫 적용 전략은 기존 384px preview를 미세 조정하는 것이 아니라 다음 progressive display 계층을 새로 도입하는 것이다.

1. `cameraSource`: 카메라 JPEG 또는 CR2 내장 JPEG. 진행 상태에는 쓸 수 있지만 프리셋이 없으므로 성공 화면은 아니다.
2. `displayFitPresetProxy`: 실제 관람 창 viewport×DPR에 맞춘 프리셋 적용 JPEG. 첫 고객 성공 화면이다.
3. `rawRefinedDisplay`: RAW와 전체 XMP로 만든 화면용 정밀본. proxy가 보인 뒤 무중단 교체한다.
4. `final`: 저장·인계용 최대 품질 결과. 첫 표시를 막지 않는다.

현재 앱에는 전용 viewer route/window가 없고 booth 창의 168~220px rail만 있다. 사용자의 제품 요구를 만족하는 가장 안전한 첫 vertical slice는 세션 시작 시 `viewer-window`를 대상 모니터에 미리 생성해 window 생성, navigation, 첫 layout, WebView2/GPU 초기화를 촬영 직후 critical path에서 제거하는 것이다. 실제 관람 면을 visible standby로 유지하는 안을 기본으로 하되, pre-created hidden window도 같은 KPI를 통과하는지는 동일 장비에서 A/B한다. WebView2는 control/environment 재사용과 GPU rendering을 성능 원칙으로 권고한다. [WebView2 performance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance)

빠른 source의 우선순위는 새 실측으로 조정한다.

- 현재 장비의 EOS 700D CR2 두 파일에는 5184×3456 embedded JPEG가 있었고, LibRaw 도구로 외부 프로세스 비용을 포함해 cold 약 50ms, warm 약 12ms에 추출됐다. 이 두 샘플에서는 RAW 전송 약 1.38~1.49초 뒤 Windows Shell fallback의 추가 0.83~1.69초를 제거할 가능성을 확인했다. 그러나 표본이 두 개뿐이므로 기본 경로 확정 근거는 아니며, 실제 촬영 30개 이상에서 존재율·orientation·손상·decode·화질을 승인해야 한다. 여기서 full-resolution source는 원본 크기 JPEG를 뜻하고 pre-created full viewer와는 별도 요구다. LibRaw는 embedded thumbnail/preview API와 EOS 700D 지원을 공식 문서화한다. [LibRaw API](https://www.libraw.org/docs/API-CXX.html), [supported cameras](https://www.libraw.org/supported-cameras)
- 동시에 EDSDK의 `ImageQuality` capability descriptor와 RAW+JPEG `groupID`를 이용해 RAW+S2/S1/Medium/Large 중 실제 카메라가 허용하는 조합을 조사한다. JPEG가 RAW보다 먼저 안정적으로 도착할 때만 우선 source로 승격한다. EOS 700D가 RAW+small 조합을 반드시 지원한다고 가정하지 않고 runtime capability를 truth로 사용한다. [Canon CAP](https://asia.canon/en/campaign/developerresources/camera/cap/cap), [EOS 700D specification](https://asia.canon/en/support/6200160400)
- Lightroom Classic 15.4의 Canon PTP는 별도 adapter spike다. Adobe는 새 기본 PTP를 더 빠르고 안정적인 tethering으로 설명하지만 내부 구현과 latency SLA는 공개하지 않았다. 동일 EOS 700D에서 PTP와 Canon SDK를 black-box A/B하고, 독립 WPD/PTP 경로가 실제로 remote capture와 object transfer를 지원할 때만 후속 투자한다. [Adobe tethered camera support](https://helpx.adobe.com/lightroom-classic/desktop/kb/tethered-camera-support.html), [WPD capture command](https://learn.microsoft.com/en-us/windows/win32/wpd_sdk/wpd-command-still-image-capture-initiate-command)

renderer 전략도 실측으로 바뀐다. display-fit 1920px Soft Glow proxy의 local A/B에서 darktable one-shot은 CPU 약 3.53초, OpenCL 약 4.99초였지만 실제 pixelpipe 계산은 약 0.3초였다. 즉 source가 1.5초에 준비돼도 총 5초대가 되어 공식 목표를 넘고, 비용 대부분은 매 capture process·core·OpenCL 초기화다. 표준 `darktable-cli`에는 미래 작업을 받는 daemon/stdin/RPC mode가 없다. [darktable-cli](https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/), [OpenCL activation](https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/)

따라서 resident renderer를 **본개발이 아니라 짧은 검증 spike**로 승격할 근거는 충분하다. 가장 작은 후보는 pre-created viewer의 WebGL2 context에 preset 선택 시 shader/LUT와 1920급 texture/FBO를 precompile·preallocate하는 방식이다. Windows native 대안은 WIC decode + Direct2D/D3D11 effect graph다. Exposure, temperature/tint, sharpen, blur, 3D LUT, color management은 OS effect로 구성하고 sigmoid/bloom 같은 차이만 custom shader로 처리한다. [Direct2D effects](https://learn.microsoft.com/en-us/windows/win32/direct2d/effects-overview)

이 엔진은 모든 darktable XMP를 암묵적으로 근사해서는 안 된다. preset publication 때 `proxyCompatible` 판정과 versioned compiled recipe를 만들고, 지원 operation allowlist 밖의 mask·heal·AI·복잡한 geometry가 있으면 정확한 darktable RAW 경로로 fallback한다. darktable은 화면 정밀본·final·시각 parity oracle로 유지한다.

과거 Story 1.26 브랜치는 전체 merge/cherry-pick하지 않는다. 실제 renderer가 여전히 capture마다 darktable CLI를 실행했고 256/288px 및 `original-visible→preset-visible` 지표를 사용했기 때문이다. 다만 late-preview recovery, request/session correlation, earliest-visible 보존, source/render/publish/display span, request-scoped staging, truthful engine mode, freshness 검증 같은 작은 seam은 current main 위에 재구현한다. 새 viewer와 tier contract를 먼저 만든 뒤 shadow route에서 검증한다.

### Development Workflows and Tooling

개발 workflow는 기능 단위가 아니라 latency span과 evidence 단위로 운영한다.

1. 한 capture의 `button`, helper accepted, transfer start/end, proxy source ready, queue/render start/end, immutable publish, viewer event receipt, decode, first present, RAW swap을 같은 request/capture/preset/generation으로 연결한다.
2. viewer는 자신의 CSS photo rect, DPR, monitor mode, 실제 natural image dimensions를 보고한다. output long edge는 hard-coded 384가 아니라 physical fit size class로 결정한다.
3. preset publication 시 raw XMP와 별도로 immutable proxy recipe/LUT를 compile하고 hash한다. renderer hot path에서는 XML parsing·module 탐색·shader compile을 하지 않는다.
4. performance change는 동일 source/preset/hardware의 before/after raw samples, cold/warm 구분, output image와 screen evidence를 함께 제출한다.
5. past branch는 selective port checklist로만 사용하며 branch 전체 merge를 금지한다. 각 port는 current contracts와 current hardware gate를 새로 통과해야 한다.

일상 도구는 기존 pnpm/Vitest, Cargo test/clippy/fmt, dotnet test에 real JPEG decoder, renderer golden corpus, native viewer harness를 추가한다. Windows 단계별 CPU/GPU/I/O 분석은 Windows Performance Recorder/Analyzer를 사용하고 darktable 기준선은 `-d opencl -d perf`를 함께 남긴다. [Windows Performance Toolkit](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/)

### Testing and Quality Assurance

공식 KPI는 파일 생성이나 `<img onLoad>`가 아니라 **사전 열린 실제 관람 화면의 qualifying frame present**다. viewer는 촬영 최소 1초 전에 visible·layout-ready 상태여야 하고, 촬영 뒤 window 생성·navigation·resize가 없어야 한다.

- 시작 `T0`: native harness가 UI Automation/SendInput으로 촬영 버튼을 누르기 직전 QPC.
- 종료 `Tpaint`: DXGI Desktop Duplication frame의 `LastPresentTime`. 같은 QPC clock에서 측정한다. [Desktop Duplication API](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api), [DXGI frame info](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/ns-dxgi1_2-dxgi_outdupl_frame_info)
- qualifying frame: 같은 session/request/capture/preset@version, 주 photo rect가 완전히 채워짐, preset 적용, `naturalWidth/Height ≥ CSS rect×DPR`, 두 연속 present frame 유지.
- unfiltered camera JPEG, thumbnail rail, file-ready, renderer-ready, `<img onLoad>`는 진단 span일 뿐 성공 종료점이 아니다.

출하 display matrix의 최소 기준은 다음과 같다.

| Display profile | 3:2 contain에 필요한 최소 photo pixels | 기본 proxy class |
| --- | ---: | ---: |
| 1920×1080, DPR 1 | 약 1620×1080 | 1920×1280 |
| 2560×1440, DPR 1.25 | 약 2160×1440 | 2592×1728 또는 fit class |
| 3840×2160, DPR 2 | 약 3240×2160 | 3456×2304 또는 fit class |

기본 1080p profile은 sRGB JPEG quality 90을 시작점으로 한다. local A/B에서 1920px q90은 약 342KB였고 q95 약 791KB 대비 wall time 차이는 거의 없으면서 decode/I/O 부담이 작았다. 최종값은 실제 booth monitor의 피부, highlight, shadow, 미세 질감 blind review로 고정한다.

화질 gate는 모든 published preset과 EOS 700D 대표 corpus에서 orientation/crop/sRGB를 일치시켜 실행한다. 초기 자동 기준은 SSIM≥0.95, median ΔE00≤3, p95≤8, skin ROI median≤3, proxy MTF50≥RAW 정밀본의 80%, clipping 증가≤2%p, upscale 0으로 두고, 5명×30 transition blind review에서 “프리셋이 달라 보이거나 교체가 거슬린다”가 5%를 넘으면 proxy compatibility를 승인하지 않는다. [OpenCV SSIM example](https://docs.opencv.org/4.8.0/d5/dc4/tutorial_video_input_psnr_ssim.html), [CIEDE2000 reference](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/)

자동·실장비 test matrix는 다음과 같다.

- PR: real JPEG decode/dimensions/hash, tier state machine, 10,000 seeded event permutations, viewer decode/double-buffer swap, 3개 viewport/DPR screenshot, TS/Rust/.NET full test.
- Nightly: 100,000 randomized race, process kill·partial write·locked file·rename fault, 모든 preset×24 RAW/proxy corpus, long sequential captures.
- Transport experiment: EDSDK와 PTP, RAW-only embedded preview와 지원되는 RAW+JPEG 조합 각각 `5 warm-up + 30 measured`, randomized AB/BA block.
- Release candidate: 선택 route와 최악 비용 preset으로 5 warm-up 뒤 100 measured shots, 앱 재시작·10분 idle·reconnect 별도 10회, raw samples와 95% confidence report.
- viewer swap: proxy 첫 frame부터 RAW 안정화 500ms 뒤까지 모든 frame을 저장하고 blank, spinner, 이전/다른 capture, crop·scale jump, tier downgrade가 0인지 검사한다.

현재 speculative test는 RAW가 이미 존재한 뒤 completion을 호출하므로 overlap 증거가 아니다. 새 integration test는 proxy가 실제 관람 화면에 paint될 때까지 RAW availability를 의도적으로 막고, fast source→preset render→publish→present가 먼저 끝났음을 증명해야 한다.

### Deployment and Operations Practices

현재 Windows workflow는 앱 shell build만 수행하고 Canon helper, EDSDK, darktable/renderer를 설치물에 포함하지 않는다. lint/test, .NET publish, clean-install smoke, signing, artifact upload도 없다. 따라서 현재 installer success는 촬영 가능한 제품 증거가 아니다.

배포는 하나의 signed runtime baseline으로 고정한다.

- app, self-contained win-x64 Canon helper, 승인 EDSDK native DLL, resident proxy renderer 또는 viewer shader bundle, color profile, proxy recipe를 version/hash inventory에 포함한다.
- darktable은 라이선스를 확인한 pinned managed prerequisite 또는 resource로 배포하고 startup preflight에서 실제 executable version/hash와 OpenCL 상태를 확인한다.
- Tauri `externalBin`/resources를 사용하고, 네트워크가 차단되고 Node/Rust/.NET SDK/darktable이 없는 pristine Windows VM에서 install, launch, self-check, fake-camera full-view paint, upgrade, uninstall을 검증한다. [Tauri sidecars](https://v2.tauri.app/develop/sidecar/), [Tauri resources](https://v2.tauri.app/develop/resources/)
- release에서도 bounded rotating local log를 활성화하고 사진/PII를 제외한 support bundle을 만든다. [Tauri logging](https://v2.tauri.app/plugin/logging/)
- 전용 EOS 700D+출하 GPU/display lab runner는 protected signed tag/manual environment만 실행한다. untrusted PR code를 USB hardware runner에서 실행하지 않는다. [GitHub self-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/use-in-a-workflow)

rollout mode는 `legacy`, `layered-shadow`, `layered-canary`, `layered-default` 네 단계다. capture 시작 시 mode를 고정하고 중간 변경하지 않는다. lab→1개 pilot branch→10~20%→전체로 승격하며 source miss, viewer unready, quality failure, p95 regression, wrong-generation event가 발생하면 마지막 승인 unit으로 즉시 rollback한다. 활성 세션 중 update/install은 금지한다. updater를 쓸 경우 signature 검증과 safe transition 적용을 유지한다. [Tauri updater](https://v2.tauri.app/plugin/updater/)

Canon SDK와 LibRaw/darktable의 배포·라이선스 조건은 release gate다. Canon SDK의 지역·object-code 배포 조건과 연간 갱신 owner를 정하고, LibRaw의 LGPL-2.1/CDDL dual-license 의무와 darktable GPL 계열 사용 방식을 법무와 확인한다. [Canon SDK terms](https://asia.canon/en/campaign/developerresources/terms-conditions-for-digital-camera-software-development-kit-sdk), [LibRaw repository/license](https://github.com/LibRaw/LibRaw)

### Team Organization and Skills

최소 delivery team은 다음 책임을 명시적으로 가진다.

- Windows/Rust/.NET owner: EDSDK transfer grouping, LibRaw boundary, async capture, process/job lifecycle, installer.
- Imaging/render owner: preset operation matrix, compiled proxy recipe, WebGL2 또는 Direct2D graph, color management와 parity corpus.
- Frontend/WebView performance owner: pre-opened viewer, DPR/viewport contract, double buffer, present telemetry.
- QA/release owner: DXGI harness, hardware lab, signed artifact, canary/rollback evidence.

2명의 core engineer가 imaging과 platform을 나누고 frontend QA/branch ops를 겸임하는 최소 구성은 가능하지만, 화질 승인자와 release owner는 이름이 정해져야 한다. 1차 production-like canary는 약 2~3주가 합리적이다. 모든 preset operation을 resident engine에서 완전 지원해야 한다면 별도 2~6주 연구가 추가될 수 있다.

### Cost Optimization and Resource Management

새 cloud나 microservice는 필요하지 않다. latency, 개인정보, 장비 결합을 고려하면 single-booth local architecture가 비용과 운영 위험 모두 가장 낮다. 필수 투자는 production-like lab PC, EOS 700D, 실제 display, code signing, Canon SDK secure storage, 현장 soak 시간이다.

비용을 낮추는 순서는 다음과 같다.

1. 이미 확보된 CR2 embedded JPEG와 과거 branch의 검증 seam을 먼저 활용한다.
2. renderer pixel work가 약 0.3초임을 이용해 process startup을 제거하는 작은 3-preset resident spike를 먼저 만든다.
3. RAW+JPEG와 PTP는 실제 source acquisition을 의미 있게 앞당기는지 A/B한 뒤 투자한다.
4. darktable fork나 전체 XMP 호환 엔진은 proxy-compatible preset 범위로 목표를 못 맞출 때만 진행한다.
5. hardware workflow는 RC/nightly에만 돌려 카메라 wear와 CI 시간을 줄인다.

GPU 추가 구매나 HIGH process priority는 선행 조건이 아니다. 과거 HIGH는 ABOVE_NORMAL 대비 의미 있는 이득이 없었고 one-shot GPU는 초기화 때문에 CPU보다 느렸다. 현재 PC의 resident WebGL2/Direct2D prototype이 p95를 통과하는지 먼저 확인한다.

### Risk Assessment and Mitigation

| 위험 | 영향 | 완화 |
| --- | --- | --- |
| camera JPEG가 늦거나 없는 capture | 첫 화면 SLA 실패 | LibRaw embedded JPEG를 기본 안전 경로로 두고 RAW+JPEG를 경쟁 source로 사용 |
| proxy와 RAW의 색·crop·noise 차이 | 고객이 교체를 인지하거나 프리셋 신뢰 하락 | preset별 `proxyCompatible`, fixed sRGB/orientation/crop, automated+blind parity gate |
| resident recipe가 XMP 일부를 지원하지 않음 | 잘못된 look | operation allowlist, publication-time compiler, unsupported preset exact fallback |
| 새 viewer가 늦게 뜨거나 이벤트를 놓침 | 사진 미표시 | visible standby, listener-ready handshake, monotonic revision, snapshot reconciliation |
| 오래된 capture가 새 capture를 덮음 | 심각한 고객 오표시 | immutable generation URL, full correlation, latest-wins scheduler, stale cancellation |
| proxy→RAW swap 중 blank/flicker | 제품 품질 하락 | next asset 완전 decode 후 opaque double-buffer swap, frame-level gate |
| LibRaw/EDSDK/darktable 배포 조건 | 출시 중단 | license owner, signed versioned runtime unit, clean-VM inventory gate |
| sync→async 전환 시 현재 readiness가 active request를 지움 | fast path 회귀 | 명시적 capture state machine과 `camera-preparing` semantics 수정 후 전환 |
| 과거 branch 대규모 병합 | current main regression | seam별 재구현, shadow output, capture-boundary feature flag |
| file-ready 지표로 성능을 과장 | 잘못된 Go 판단 | DXGI/QPC actual-present KPI와 실패 포함 percentile |

## Technical Research Recommendations

### Implementation Roadmap

**Phase 0 — 기준선과 관람 창 계약, 0.5~1.5일**

- 공식 KPI를 button→qualifying full-view present로 고정한다.
- 출하 monitor mode, photo rect, DPR, 1080p/1440p/4K fit class를 기록한다.
- current main, Lightroom 15.4 PTP/SDK, 과거 evidence를 같은 지표로 다시 분류한다.

**Phase 1 — pre-opened viewer vertical slice, 1.5~3일**

- startup/session start에 pre-created `viewer-window`와 read-only `/viewer` surface를 만든다. visible standby를 기본안으로 하고 hidden prewarm도 A/B한다.
- viewer-ready handshake, snapshot revision, targeted ordered update를 추가한다.
- immutable sample asset을 decode한 뒤 double-buffer swap하고 DXGI present를 기록한다.

**Phase 2 — immutable display tier와 shadow lane, 2~4일**

- `displayFitPresetProxy`, `rawRefinedDisplay`, `final`을 서로 다른 immutable generation 파일로 저장한다.
- temp write→full decode/dimension 검증→atomic rename→manifest commit→event 순서를 고정한다.
- shadow mode로 성능·화질을 모으되 고객 화면은 legacy를 유지한다.

**Phase 3 — fast source race, 1~6일**

- 우선 LibRaw embedded JPEG를 실제 CR2 30개에서 p95, orientation, corruption, dimensions로 승인한다.
- EDSDK capability descriptor와 groupID 기반 RAW+JPEG pairing을 20~100 capture로 A/B한다.
- 더 먼저 도착한 승인 source를 선택한다. CR2 embedded path는 30개 이상 품질·안정성 gate를 통과한 뒤에만 기본 또는 fallback 지위를 부여한다.

**Phase 4 — resident display-fit preset renderer, 3~6일 spike**

- 현재 3개 승인 preset을 WebGL2 또는 WIC+Direct2D recipe로 compile하고 selection 때 prewarm한다.
- 1920×1280 q90을 시작점으로 source-ready→present와 parity를 비교한다.
- unsupported operation fallback과 darktable reference output을 유지한다.

**Phase 5 — RAW refinement와 deadline scheduler, 2~4일**

- 우선순위를 `현재 capture preset proxy P0 → RAW display refine P1 → final/warmup P2`로 둔다.
- current capture 중 기본 renderer concurrency는 1, latest-wins/coalesce, stale child tree cancellation을 적용한다.
- RAW 결과는 완전 decode 뒤 무중단 swap하고 최초 proxy paint KPI를 보존한다.

**Phase 6 — production packaging과 canary, 2~3일 + soak**

- signed complete installer, clean offline VM, hardware 100-shot gate를 통과한다.
- lab→pilot→10~20%→default rollout과 instant forced fallback을 운영한다.
- PTP는 SDK/embedded source보다 p95가 최소 300ms 또는 15% 개선되고 100회 성공률 99% 이상일 때만 채택한다. 이 수치는 공식 업계 기준이 아니라 초기 투자 판단을 위한 운영 threshold다.

### Technology Stack Recommendations

| 영역 | 권장 | 역할 |
| --- | --- | --- |
| Desktop shell | 현 Tauri 2 + React 유지 | viewer window, local asset URL, operator UI |
| Live viewer renderer | WebGL2 우선 spike, Direct2D/D3D11 대안 | app-lifetime GPU context와 compiled preset proxy |
| Camera control | 현 Canon EDSDK 유지, PTP는 A/B adapter | remote capture와 transfer truth |
| Fast source | LibRaw embedded JPEG 유력 후보 + capability-gated RAW+JPEG race | 30개 이상 실장비 승인 뒤 display-fit source 경로 확정 |
| RAW truth/final | pinned darktable + full XMP | rawRefinedDisplay, final, parity oracle |
| Live IPC | app-lifetime helper duplex stdio + Tauri Channel | ordered request/stage delivery; bytes가 아니라 path+revision 전송 |
| Durable state | immutable files + revisioned manifest/journal | crash recovery, provenance, rollback |
| Windows process control | Job Object + class priority | stale renderer tree 종료와 deadline isolation |
| Measurement | QPC + DXGI Desktop Duplication + WPT | 실제 관람 화면 present와 span 진단 |

Tauri asset URL로 이미지 path만 넘기고 WebView가 로컬 파일을 읽는 현재 방향은 유지한다. 이미지 bytes를 IPC로 복사하지 않는다. React/Vite/Tauri 단순 upgrade는 보안·지원 관점의 별도 작업이며 초 단위 latency 해법으로 간주하지 않는다.

### Skill Development Requirements

팀이 확보해야 할 핵심 역량은 다음과 같다.

- Canon EDSDK object lifetime, transfer grouping, image-quality descriptor, camera reconnect.
- LibRaw thumbnail/preview API와 Windows native library packaging/licensing.
- color science 기초: sRGB, white balance, tone curve, 3D LUT, ΔE00, highlight/shadow clipping.
- WebGL2 또는 Direct2D/D3D11 shader/effect pipeline, GPU resource lifetime과 double buffering.
- Tauri multi-window lifecycle, WebView2 throttling, ordered event/snapshot reconciliation.
- Windows QPC, DXGI Desktop Duplication, Job Object, code signing과 clean-machine deployment.
- percentile/confidence, randomized race/fault injection, visual golden corpus 운영.

### Success Metrics and KPIs

제품 성공 지표는 다음으로 확정한다.

| 지표 | Release gate |
| --- | --- |
| 공식 E2E | trusted button input→pre-opened viewer의 첫 qualifying preset-applied present |
| Warm latency | p50≤3.0초, p95≤4.0초, hard max≤5.0초 |
| Reliability | release 100-shot 성공률≥99%; timeout/실패 샘플을 percentile에서 제외하지 않음 |
| Correctness | wrong session/request/capture/preset/version 0, unfiltered qualifying frame 0 |
| Display quality | physical fit dimensions 충족, upscale 0, preset별 automated+blind parity 승인 |
| Swap quality | blank/spinner/이전 사진 frame 0, crop·scale jump 0, tier downgrade 0 |
| Source | embedded 또는 paired JPEG source-ready→viewer present p95≤100ms를 renderer 내부 목표로 추적 |
| Cold/recovery | cold-start 5회 각각≤5초 목표, 10분 idle/reconnect 10회 p95≤4초 |
| Regression | 직전 승인 build 대비 p95 악화≤10%, visual gate 자동 완화 금지 |
| Deployment | signed installer SHA, helper/EDSDK/renderer/darktable/preset hashes와 clean-VM·hardware evidence 완비 |

이 지표에서 현재 main은 전용 viewer 부재, 384px output, RAW 이후 4.8~5.8초 one-shot render, file/onLoad 중심 측정 때문에 즉시 No-Go다. CR2 두 샘플에서 full-resolution embedded JPEG와 약 0.3초 pixel work가 확인된 사실은 pre-created viewer+resident display renderer 가설을 강화한다. 그러나 Adobe는 내부 SLA를 공개하지 않았고 LibRaw 표본도 작으므로, 3~4초는 사용자 관측 기반의 외부 비교 목표일 뿐이다. 동일 KPI의 Phase 4 hardware prototype과 Lightroom head-to-head 전에는 동급 성능을 주장할 수 없다.

## Research Synthesis and Final Decision

### Executive Summary

현재 Boothy는 사용자가 정의한 제품 경험, 즉 **촬영 전에 준비된 관람 창에 화면 크기의 프리셋 적용 사진이 거의 즉시 표시되는 경험**을 충족하지 못한다. 현재 구조는 작은 384px preview와 RAW 이후 one-shot darktable render에 의존하고, 전용 viewer와 실제 monitor-present 측정이 없다. 따라서 현재 방식은 최적이 아니며 release 기준으로 No-Go다.

목표 구조는 `fast JPEG source → display-fit preset proxy → raw-refined display → final`이다. 첫 고객 성공 화면은 프리셋이 적용된 display-fit proxy이고, 카메라 원본 JPEG는 진행 피드백일 뿐 성공으로 계산하지 않는다. RAW 정밀본은 proxy가 보인 뒤 blank나 crop jump 없이 교체한다.

EOS 700D CR2 두 샘플에서 5184×3456 embedded JPEG가 발견됐고 추출 자체는 약 12~50ms였다. display-fit 1920px darktable one-shot은 CPU 약 3.53초, OpenCL 약 4.99초였지만 실제 pixelpipe 계산은 약 0.3초였다. 이는 fast source와 resident renderer가 유력함을 보여주지만, LibRaw 표본 30개 이상과 실제 viewer prototype이 통과하기 전에는 production proof가 아니다.

**Key Technical Findings**

- 현재 main의 Canon helper/Tauri/darktable local architecture는 유지할 수 있지만 live display path는 재설계해야 한다.
- 조기 JPEG에 프리셋을 적용하는 speculative 경로는 production 호출에 연결되지 않았고 현재 preset preview는 RAW를 기다린다.
- 과거 2.4~2.9초 기록은 original-visible→preset-visible이며 button→full-view paint가 아니다.
- Lightroom 공개 패턴은 embedded/minimal→standard preview, 화면 크기 preview, cache, 조건부 GPU다. Adobe는 3~4초 SLA나 내부 preset scheduling을 공개하지 않았다. [Lightroom performance](https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html.html), [GPU Preview generation](https://helpx.adobe.com/lightroom-classic/desktop/kb/gpu-preview-generation.html)
- Lightroom Classic 15.4는 Canon 기본 tether transport를 새 PTP로 바꾸고 faster/reliable이라고 설명하지만, Boothy에서의 이득은 별도 A/B가 필요하다. [Adobe release notes](https://helpx.adobe.com/uk/lightroom-classic/help/whats-new/release-notes.html), [tether support](https://helpx.adobe.com/uk/lightroom-classic/kb/tethered-camera-support.html)
- darktable 5.6.0이 최신이지만 단순 upgrade가 one-shot process 초기화를 제거하지 않으므로 성능 해법으로 단정할 수 없다. [darktable 5.6.0](https://www.darktable.org/2026/06/darktable-5.6.0-released/)

**Strategic Recommendations**

1. 전용 viewer와 actual-present KPI를 먼저 만든다.
2. LibRaw embedded JPEG와 EDSDK RAW+JPEG를 경쟁 source로 검증한다.
3. display-fit one-shot 기준선을 유지하면서 3개 승인 preset의 resident WebGL2/Direct2D spike를 수행한다.
4. darktable은 raw-refined display, final, parity oracle로 유지한다.
5. immutable generation, exact correlation, deadline scheduler와 forced fallback 뒤에만 canary를 시작한다.

### Table of Contents

1. Technical Research Introduction and Methodology
2. Technical Landscape and Architecture Analysis
3. Implementation Approaches and Best Practices
4. Technology Stack Evolution and Current Trends
5. Integration and Interoperability Patterns
6. Performance and Scalability Analysis
7. Security and Compliance Considerations
8. Strategic Technical Recommendations
9. Implementation Roadmap and Risk Assessment
10. Future Technical Outlook and Innovation Opportunities
11. Technical Research Methodology and Source Verification
12. Technical Appendices and Reference Materials

### 1. Technical Research Introduction and Methodology

이 연구의 핵심은 renderer 단일 구간이 아니라 `button→camera→source→preset render→publish→decode→monitor present` 전체를 같은 기준으로 측정하는 데 있다. 코드와 문서, 실제 EOS 700D session evidence, 과거 branch, current dependency·installer·CI를 조사하고 공식 primary source로 기술적 가능성과 최신성을 검증했다.

근거는 세 수준으로 분리했다.

- **입증:** 현재 코드나 보존된 실측으로 재현 가능한 사실.
- **고신뢰 가설:** 공식 기술 구조와 local measurement가 지지하지만 hardware prototype이 필요한 판단.
- **미확인:** Adobe 내부 구현, EOS 700D RAW+small JPEG capability, 독립 PTP 우위처럼 실험 전에는 단정할 수 없는 항목.

### 2. Technical Landscape and Architecture Analysis

현재 시스템은 React/TypeScript/WebView2 frontend, Tauri/Rust host, .NET/Canon EDSDK helper, darktable/XMP renderer, filesystem manifest/journal로 구성된 local modular monolith다. 개인정보·장비 결합·single-booth latency를 고려하면 cloud/microservice 전환은 필요 없다.

문제는 구성 요소가 아니라 critical path다. unfiltered camera JPEG는 먼저 표시할 수 있지만 preset preview는 RAW persistence 이후에 시작되고, dormant speculative path는 실제 호출되지 않는다. viewer surface도 384px rail 수준이라 사용자 요구와 계측 기준이 다르다.

목표 architecture는 app-lifetime camera helper와 viewer, progressive tier, immutable asset pointer, ordered live notification, durable filesystem recovery를 결합한다. source 확보, preset transform, actual display를 서로 독립된 span으로 측정한다.

### 3. Implementation Approaches and Best Practices

첫 vertical slice는 pre-created viewer, read-only snapshot, monotonic revision, immutable sample asset, double-buffer swap, QPC/DXGI telemetry다. 이 단계에서 visible standby와 hidden prewarm을 동일 PC에서 비교한다.

다음으로 `displayFitPresetProxy`, `rawRefinedDisplay`, `final`을 서로 다른 generation filename으로 저장한다. 완전 쓰기→실제 decode/dimension 검증→atomic rename→manifest revision commit→viewer notification 순서를 지킨다. previous asset은 next asset decode가 끝날 때까지 유지한다.

fast source는 LibRaw embedded JPEG와 capability-gated RAW+JPEG를 A/B한다. LibRaw는 embedded thumbnail/preview를 반환하는 공식 API와 EOS 700D 지원을 제공한다. [LibRaw API](https://www.libraw.org/docs/API-CXX.html), [supported cameras](https://www.libraw.org/supported-cameras)

resident renderer는 WebGL2를 가장 작은 spike로 보고 WIC+Direct2D/D3D11을 Windows-native 대안으로 둔다. preset publish 때 operation allowlist와 versioned recipe/LUT를 compile하며 unsupported preset은 exact darktable path로 fallback한다. Direct2D는 WIC image에 GPU-backed effect graph와 custom effect를 적용할 수 있다. [Direct2D effects](https://learn.microsoft.com/en-us/windows/win32/direct2d/effects-overview)

### 4. Technology Stack Evolution and Current Trends

현 Tauri/React stack과 local asset URL은 유지한다. WebView2는 GPU 사용과 environment 재사용을 권장하므로 app-lifetime viewer와 renderer context 방향이 타당하다. [WebView2 performance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance)

LibRaw stable 0.22.2는 유력한 extraction boundary지만 배포·license·corpus 검증이 필요하다. [LibRaw](https://www.libraw.org/about)

darktable 최신 5.6은 preview와 OpenCL 개선을 포함하지만 current 5.4.1과 output parity·process wall-time A/B 없이 승격하지 않는다. 새 버전 자체보다 one-shot lifetime이 더 큰 구조적 병목이다.

Lightroom에서 참고할 공개 기술은 작은 embedded preview, display-size standard preview, Camera Raw/preview cache, Smart Preview, 조건부 GPU다. Adobe 내부 engine이나 scheduling은 복제 대상으로 단정하지 않는다. [Smart Previews](https://helpx.adobe.com/lightroom-classic/desktop/viewing-photos/lightroom-smart-previews.html)

### 5. Integration and Interoperability Patterns

live plane은 app-owned helper와 duplex stdio, Tauri ordered Channel, path+revision payload를 권장한다. JSONL/manifest는 audit와 recovery plane으로 비동기 유지한다. image bytes를 IPC로 복사하지 않고 WebView가 immutable local URL을 읽는다.

모든 update는 sessionId, requestId, captureId, presetId/version, renderGeneration을 가진다. event는 notification이며 durable snapshot revision을 truth로 삼는다. late/duplicate/out-of-order update는 낮은 revision이나 stale generation으로 거절한다.

Canon RAW+JPEG는 각 transfer object를 group 단위로 수집하고 모든 object를 download 또는 cancel해야 한다. 현재처럼 첫 object만 capture로 가정하는 방식은 pairing 전에 교체해야 한다.

### 6. Performance and Scalability Analysis

공식 E2E는 `trusted button input→pre-created viewer의 첫 qualifying preset-applied monitor present`다. `<img onLoad>`, file-ready, XMP-ready는 진단값일 뿐 종료점이 아니다. DXGI Desktop Duplication의 QPC 기반 present time으로 실제 monitor frame을 판정한다. [Desktop Duplication API](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api)

초기 제품 gate는 warm p50≤3초, p95≤4초, hard max≤5초다. unfiltered frame, wrong capture/preset, upscale, blank, crop/scale jump는 0이어야 한다. 이는 Adobe SLA가 아니라 사용자 관측을 Boothy release gate로 바꾼 제품 목표다.

1080p의 시작 proxy는 1920×1280 sRGB JPEG q90이며 실제 viewport×DPR에 따라 1440p/4K fit class로 승격한다. Adobe도 Standard Preview를 화면 긴 변보다 작지 않게 선택하고 Low/Medium JPEG 품질을 권장한다. [Lightroom performance guidance](https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html.html)

단일 booth에서는 throughput보다 최근 capture deadline이 중요하다. scheduler는 current preset proxy P0, RAW refine P1, final/warmup P2로 나누고 P0 실행 중 renderer concurrency 1을 기본으로 한다. stale capture는 coalesce/cancel한다.

### 7. Security and Compliance Considerations

현재 asset scope와 CSP, command surface, helper file protocol은 production hardening이 필요하다. viewer는 preview directory만 읽게 scope를 좁히고 originals/diagnostics를 노출하지 않는다. capability와 window label을 host에서 재검증한다.

installer는 app shell뿐 아니라 helper, EDSDK, renderer, color profile, preset recipe, pinned darktable 또는 managed prerequisite를 하나의 signed inventory로 제공해야 한다. Tauri는 external sidecar와 resource packaging을 공식 지원한다. [Tauri sidecars](https://v2.tauri.app/develop/sidecar/), [Tauri resources](https://v2.tauri.app/develop/resources/)

Canon SDK 지역·object-code 배포 조건, LibRaw LGPL/CDDL dual license, darktable GPL 계열 사용 방식을 release 전에 법무와 확정한다. [Canon SDK terms](https://asia.canon/en/campaign/developerresources/terms-conditions-for-digital-camera-software-development-kit-sdk), [LibRaw repository](https://github.com/LibRaw/LibRaw)

### 8. Strategic Technical Recommendations

투자 우선순위는 다음과 같다.

1. viewer와 actual-present measurement.
2. immutable tier와 full correlation.
3. LibRaw/RAW+JPEG source race.
4. resident 3-preset renderer spike.
5. RAW refinement와 deadline scheduler.
6. signed installer, hardware gate, canary.
7. PTP와 전체 preset engine은 앞 단계 evidence에 따라 조건부 투자.

단순 React/Tauri upgrade, asset URL 변경, 192px 축소, HIGH priority, darktable one-shot pool은 first-order 해법이 아니다. 과거 branch도 전체 병합하지 않고 late recovery, correlation, timing, freshness seam만 재구현한다.

### 9. Implementation Roadmap and Risk Assessment

| Phase | 산출물 | 낙관–보수 추정 |
| --- | --- | ---: |
| 0 | KPI·display profile·동일 기준선 | 0.5~1.5일 |
| 1 | pre-created viewer·double-buffer·DXGI evidence | 1.5~3일 |
| 2 | immutable display tier·shadow lane | 2~4일 |
| 3 | LibRaw 30+ corpus·RAW+JPEG A/B | 1~6일 |
| 4 | 3-preset resident renderer spike | 3~6일 |
| 5 | RAW refinement·deadline scheduler | 2~4일 |
| 6 | complete installer·100-shot canary·soak | 2~3일+soak |

이 범위는 확정 일정이 아니다. LibRaw 품질, preset operation compatibility, multi-window integration, license review에 따라 늘어날 수 있다. production-like canary는 약 2~3주를 예상하지만 전체 XMP 호환 renderer는 별도 2~6주 연구가 될 수 있다.

가장 큰 위험은 source variability, proxy/RAW look 차이, unsupported preset, stale capture display, swap flicker, incomplete installer다. 대응은 30+ corpus, `proxyCompatible`, exact fallback, immutable generation, frame-level gate, signed clean-VM inventory다.

### 10. Future Technical Outlook and Innovation Opportunities

- Lightroom PTP/Canon SDK와 Boothy SDK/PTP의 동일 hardware head-to-head.
- preset publication 때 3D LUT/effect graph compile.
- viewer-reported physical size 기반 dynamic proxy class.
- WebGL2와 Direct2D의 latency·color parity·driver reliability 비교.
- 실제 source p95가 2초를 넘을 때 독립 PTP/WPD adapter 연구.
- 제한된 proxy recipe로 목표를 못 맞출 때만 custom native renderer 또는 darktable fork 평가.

### 11. Technical Research Methodology and Source Verification

Primary sources는 Adobe Lightroom help/release notes, Canon CAP/specification/SDK terms, Microsoft WebView2/DXGI/Direct2D/WPD 문서, Tauri 공식 문서, darktable manual/release/source, LibRaw API/support/license다. local evidence는 current EOS 700D session, timing JSONL, real CR2/JPEG output, archived branch ledger와 test execution이다.

높은 신뢰 결론은 current main No-Go, 384px 부적합, one-shot overhead, 과거 metric 비동등, installer gap이다. 중간 신뢰는 LibRaw fast source와 resident renderer 가능성이다. 미확인은 EOS 700D 전체 capture에서 embedded preview의 일관성, RAW+small JPEG capability/order, PTP 우위, Lightroom 내부 preset scheduling이다.

30-shot/100-shot은 초기 제품 decision gate이며 장기 신뢰성의 통계적 보증이 아니다. raw sample을 누락 없이 보존하고 percentile·failure·environment fingerprint를 함께 보고한다.

### 12. Technical Appendices and Reference Materials

**핵심 local benchmark**

| 항목 | 관측 |
| --- | ---: |
| Canon direct thumbnail | 0/2 success |
| RAW transfer | 약 1.38~1.49초 |
| Shell fallback after RAW | 추가 약 0.83~1.69초 |
| Current RAW preset render | 약 4.84~5.77초 |
| CR2 embedded JPEG | 2/2에서 5184×3456; 표본 부족 |
| LibRaw extraction | cold 약 50ms, warm 약 12ms; 외부 process 포함 |
| 1920px darktable CPU/OpenCL | 약 3.53초 / 4.99초 |
| 실제 pixelpipe | 약 0.3초 |

**실무 release evidence**

- exact installer SHA와 app/helper/EDSDK/renderer/darktable/preset hashes.
- OS, display/DPR/ICC/HDR, GPU/driver, CPU/RAM/disk, WebView2, camera/firmware/USB/power profile.
- per-shot timeline과 proxy/RAW swap 주변 monitor frames.
- clean offline VM, cold/reconnect, sequential capture, quality corpus 결과.

## Technical Research Conclusion

### Summary of Key Technical Findings

현재 방식은 최적이 아니며 사용자의 full-view preset requirement를 충족하지 못한다. 그러나 Tauri/Canon/darktable 기반 전체를 폐기할 필요는 없다. 사전 준비된 관람 창, 승인된 fast JPEG source, display-fit resident preset proxy라는 세 부분을 새 hot path로 만들고 기존 RAW path를 정밀본·final·fallback으로 유지하는 것이 가장 낮은 위험의 해법이다.

### Strategic Technical Impact Assessment

가장 큰 개선은 rendering parameter가 아니라 작업 순서와 engine lifetime에서 나온다. embedded 또는 paired JPEG를 빨리 확보하고 process startup을 display path에서 제거하면 Lightroom과 유사한 layered-preview 체감에 접근할 가능성이 있다. 다만 3~4초는 아직 외부 비교 목표이며 같은 button→monitor-frame 측정 전에는 달성을 주장하지 않는다.

### Next Steps Technical Recommendations

1. Phase 0–1 viewer/KPI vertical slice를 먼저 구현한다.
2. LibRaw 30+ corpus와 EDSDK RAW+JPEG capability 실험을 병렬 실행한다.
3. current 3 presets의 resident renderer spike를 darktable reference와 비교한다.
4. warm p95≤4초, quality parity, zero-flicker를 통과한 route만 canary로 승격한다.
5. signed complete installer와 dedicated EOS 700D/display runner 없이는 release하지 않는다.

---

**Technical Research Completion Date:** 2026-08-11  
**Research Period:** 2026-08-10–2026-08-11 comprehensive technical analysis  
**Source Verification:** Current official primary sources and local reproducible evidence  
**Technical Confidence:** High for current-state No-Go and architectural bottlenecks; medium for fast-source/resident-renderer feasibility pending hardware proof
