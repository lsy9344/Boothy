# Story 1.6: 실카메라/helper readiness truth 연결과 false-ready 차단

Status: done

Correct Course Note: 기존 Story 1.6에 섞여 있던 실제 촬영 round-trip 책임은 Story 1.7로 분리한다. 이 문서는 `canon-helper.exe` baseline, host spawn/health, `helper-ready`와 `camera-status` 기반 readiness truth, freshness/disconnect/reconnect false-ready 차단까지만 소유한다. 2026-04-10 기준 linked session evidence와 직접 시각 검증이 canonical ledger에 반영돼 Story 1.6은 `done`으로 닫는다.

### Hardware Gate Reference

- Canonical ledger: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Required HV checklist IDs: `HV-02`, `HV-03`, `HV-10`
- Current hardware gate: `Go`
- Close policy: `automated pass` alone does not close this story; linked ledger evidence와 `Go` 확인 뒤에만 `done`으로 닫는다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
부스가 실제 helper와 카메라가 fresh readiness를 보고한 뒤에만 `Ready`를 열어 주길 원한다.
그래서 browser preview, stale status, reconnect 직후 같은 synthetic truth 때문에 false-ready를 믿고 촬영하려 하지 않는다.

## Acceptance Criteria

1. 승인된 booth hardware에서 Tauri host가 bundled `canon-helper.exe` baseline을 실제로 spawn하고 health를 감시해야 한다. helper boot 이후 fresh `helper-ready`와 `camera-status`를 받더라도, host가 first fresh `camera-status=ready`, session match, freshness를 확인하기 전에는 booth가 `Ready`와 `사진 찍기` 활성화를 보여주면 안 된다. 여기서 `Ready`는 booth 앱의 `사진 찍기` 경로 기준 readiness를 뜻하며, 카메라 본체 셔터 직접 입력의 tethered capture 성공까지 보장하지 않는다.
2. browser preview, fixture, stale readiness, session mismatch, helper 미기동, helper preparing, camera disconnect, degraded-after-ready, reconnect-before-fresh-truth 상태에서는 booth가 `Ready`를 주장하면 안 된다. customer는 계속 plain-language wait/call guidance만 보고, internal helper terminology는 보지 않는다.
3. previously-ready 상태에서 helper process exit, health timeout, camera disconnect, readiness degrade가 발생하면 booth는 즉시 `Ready`를 해제하고 capture를 차단해야 한다. reconnect 또는 helper restart 이후에도 fresh `camera-status`가 다시 확인되기 전까지 자동 복귀하면 안 된다.
4. Story 1.6은 실제 helper 프로젝트 골격, host spawn/health 관리, HV-02/HV-03/HV-10 evidence가 checklist 기준으로 닫히기 전까지 `done`으로 닫지 않고 `in-progress` 또는 `review`에 머물러야 한다.

## Tasks / Subtasks

- [x] shared readiness truth family를 real-helper 기준으로 고정한다. (AC: 1, 2, 3)
  - [x] `src/shared-contracts/schemas/capture-readiness.ts`, `src/shared-contracts/dto/capture.ts`, `src-tauri/src/contracts/dto.rs`에서 이미 정의된 `liveCaptureTruth` shape를 계속 재사용하고 별도 helper truth contract를 새로 만들지 않는다.
  - [x] `src/shared-contracts/schemas/operator-diagnostics.ts`가 booth와 같은 `liveCaptureTruth` family를 재사용하도록 유지한다.
  - [x] browser preview와 fixture path는 계속 `browser-preview` / `fixture` source로만 남기고, synthetic `Ready`를 만들지 못하게 유지한다.

- [x] `sidecar/canon-helper/` 아래에 실제 Windows helper baseline을 만든다. (AC: 1, 4)
  - [x] `canon-helper.exe` 프로젝트 골격, 빌드 출력 위치, diagnostics 출력 경계를 고정한다.
  - [x] helper는 boot self-check 뒤 `helper-ready`를 송신하고, 이후 `camera-status`를 freshness 가능한 형태로 송신해야 한다.
  - [x] helper version, sdk version, runtime platform, diagnostics path를 확인 가능한 최소 진단 surface를 남긴다.
  - [x] Canon SDK DLL/headers는 공개 저장소에 직접 커밋하지 말고 private build input 또는 승인된 내부 artifact 경계로 유지한다.

- [x] Tauri host가 real helper spawn/health/recovery 경계를 소유한다. (AC: 1, 2, 3)
  - [x] `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`을 갱신해 packaged sidecar 실행에 필요한 Tauri shell baseline을 닫는다.
  - [x] `src-tauri/src/capture/sidecar_client.rs` 또는 동등 모듈에서 helper 시작, 종료, 비정상 exit, health timeout, 상태 수집 경계를 관리한다.
  - [x] `helper-ready`와 `camera-status`를 분리한 readiness gate를 host에서 강제하고, booth React가 helper를 직접 spawn하거나 raw detail을 해석하지 못하게 유지한다.
  - [x] capture/file handoff 메시지 DTO는 보존할 수 있지만, round-trip closure 책임은 Story 1.7에 남긴다.

- [x] fresh `camera-status` 기준 readiness 복귀 규칙을 실장비 기준으로 닫는다. (AC: 1, 2, 3)
  - [x] `src-tauri/src/capture/normalized_state.rs`의 freshness/session-match gate를 real helper runtime과 연결하고, stale snapshot만으로 `Ready`가 열리지 않게 유지한다.
  - [x] once-ready 이후 helper exit, disconnect, degraded, reconnect-before-fresh-truth를 모두 blocked path로 유지한다.
  - [x] booth `Ready`, capture enablement, operator diagnostics가 같은 host-owned truth에서 파생되도록 유지한다.
  - [x] 현재 코드에 잠긴 helper freshness 기준이 바뀌면 runbook evidence와 함께 조정하고, 임시 완화로 false-ready를 허용하지 않는다. [Source: src-tauri/src/capture/normalized_state.rs]

- [x] Story 1.7과의 경계를 문서와 구현 메모에 고정한다. (AC: 1, 4)
  - [x] `request-capture`, `capture-accepted`, RAW download, `file-arrived`, in-flight capture guard, capture correlation의 end-to-end closure는 Story 1.7 범위라고 명시한다.
  - [x] 1.6에서는 readiness truth를 여는 최소 helper baseline과 recovery-safe block/unblock만 닫는다.

- [x] 테스트와 hardware validation evidence를 준비한다. (AC: 2, 3, 4)
  - [x] `src-tauri/tests/capture_readiness.rs`에 missing helper, stale status, session mismatch, degraded-after-ready, reconnect-before-fresh-truth, helper exit 시나리오를 추가 또는 강화한다.
  - [x] `src-tauri/tests/operator_diagnostics.rs`에서 operator가 같은 `liveCaptureTruth`를 재사용하는지 계속 검증한다.
  - [x] HV-02, HV-03, HV-10 evidence에 helper version, sdk version 또는 helper identifier, 최근 `camera-status` freshness 근거, `session.json` 캡처를 연결한다.

### Review Findings

- [x] [Review][Pass] blocking findings 없음. host-owned freshness gate, helper lifecycle 정리, operator truth 재사용이 스토리 acceptance criteria와 일치한다. [src-tauri/src/capture/normalized_state.rs:603] [src-tauri/src/capture/helper_supervisor.rs:82] [src-tauri/tests/capture_readiness.rs:103] [src-tauri/tests/operator_diagnostics.rs:147]
- [x] [Review][Pass] linked session evidence와 직접 시각 검증을 canonical ledger close row에 반영해 HV-02, HV-03, HV-10 gate를 `Go`로 닫았다. [docs/runbooks/booth-hardware-validation-checklist.md:273] [docs/runbooks/booth-hardware-validation-checklist.md:299] [docs/runbooks/booth-hardware-validation-checklist.md:481] [_bmad-output/implementation-artifacts/hardware-validation-ledger.md:37]

## Dev Notes

### 스토리 범위와 목적

- 이번 Story 1.6의 목적은 release-level `Ready` 진실을 여는 최소 helper baseline을 닫는 것이다.
- 핵심은 bundled helper가 실제로 존재하고, host가 그 helper를 띄우고 감시하며, `helper-ready`와 `camera-status`를 fresh truth로 해석해 false-ready를 막는 데 있다.
- 실제 촬영 요청, RAW 다운로드, `file-arrived` correlation, capture success 최종 확정은 Story 1.7이 소유한다. [Source: _bmad-output/planning-artifacts/epics.md#Story-1.6:-실카메라/helper-readiness-truth-연결과-false-ready-차단] [Source: _bmad-output/planning-artifacts/epics.md#Story-1.7:-실카메라-capture-round-trip과-RAW-handoff-correlation] [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260328-023725.md]
- 따라서 Story 1.6의 `Ready`가 확인됐더라도, 카메라 본체 셔터 직접 입력으로 들어온 out-of-band capture가 active session에 반영되는지는 이 story의 success 범위가 아니다.

### 스토리 기반 요구사항

- Epic 1 Story 1.6은 first fresh `camera-status` truth가 확인되기 전까지 `Ready`를 열지 말고, stale/synthetic truth와 reconnect-before-fresh-truth를 blocked path로 유지하라고 요구한다. [Source: _bmad-output/planning-artifacts/epics.md#Story-1.6:-실카메라/helper-readiness-truth-연결과-false-ready-차단]
- FR-003은 고객이 plain-language readiness guidance만 보고 valid state에서만 capture할 수 있어야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#FR-003-Readiness-Guidance-and-Valid-State-Capture]
- PRD release gate는 booth `Ready`가 browser fallback, stale session state, incomplete helper signal이 아니라 live camera/helper truth로만 인정돼야 한다고 못 박는다. [Source: _bmad-output/planning-artifacts/prd.md#Release-Gates]
- hardware validation runbook은 Story 1.6의 closure evidence를 HV-02, HV-03, HV-10으로 잠그고, helper raw detail이 고객 화면 copy로 새면 안 된다고 정리한다. [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-02-카메라-미연결-차단-확인] [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-03-카메라-연결-후-Ready-진입-확인] [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-10-카메라-분리-후-재연결-복구-확인] [Source: docs/runbooks/booth-hardware-validation-checklist.md#EDSDK-helper-전용-사전-확인]

### 선행 의존성과 구현 순서

- 직접 선행 흐름은 Story 1.4 readiness/capture guard baseline과 Story 1.5의 host-normalized capture pipeline이다.
- Story 1.6은 새 booth UX를 발명하는 것이 아니라, 이미 존재하는 readiness normalization seam을 real helper runtime에 연결하는 작업이다.
- 권장 구현 순서는 다음과 같다.
  - `sidecar/canon-helper/` helper baseline과 diagnostics 경계를 먼저 만든다.
  - host spawn/health/recovery를 `src-tauri/src/capture/sidecar_client.rs` 중심으로 닫는다.
  - `normalized_state.rs`가 real helper freshness/mismatch/degrade를 반영하도록 연결한다.
  - 마지막에 booth/operator projection 테스트와 HV evidence 준비를 잠근다.

### 현재 워크스페이스 상태

- `src-tauri/src/capture/normalized_state.rs`는 이미 `camera-helper-status.json`을 읽어 `freshness`, `sessionMatch`, `cameraState`, `helperState`를 `liveCaptureTruth`로 정규화하고, `Ready`를 first fresh/matched helper truth 뒤에만 열도록 막고 있다. 이 정규화 계층은 재사용해야지 새로 갈아엎을 대상이 아니다. [Source: src-tauri/src/capture/normalized_state.rs]
- `src-tauri/src/capture/sidecar_client.rs`는 helper message DTO와 최신 status snapshot 읽기만 제공한다. 실제 `canon-helper.exe` spawn, child exit 감시, `helper-ready`/`recovery-status` runtime 연결은 아직 없다. [Source: src-tauri/src/capture/sidecar_client.rs]
- `src/shared-contracts/schemas/capture-readiness.ts`는 booth와 operator가 함께 쓰는 `liveCaptureTruth` schema를 이미 갖고 있고, `src/shared-contracts/schemas/operator-diagnostics.ts`도 같은 truth family를 재사용한다. Story 1.6은 이 contract family를 유지해야 한다. [Source: src/shared-contracts/schemas/capture-readiness.ts] [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- `sidecar/canon-helper/`는 현재 `README.md`만 있고 실제 helper 프로젝트는 없다. 즉, readiness hardening 일부는 존재하지만 real helper baseline은 아직 비어 있다. [Source: sidecar/canon-helper/README.md]
- `src-tauri/Cargo.toml`에는 `tauri-plugin-shell`이 없고, `src-tauri/src/lib.rs`에도 plugin init이 없다. `src-tauri/tauri.conf.json`에는 `bundle.externalBin`이 없고 capability 파일도 `core:default`만 가진다. 따라서 packaged sidecar spawn boundary는 아직 닫히지 않았다. [Source: src-tauri/Cargo.toml] [Source: src-tauri/src/lib.rs] [Source: src-tauri/tauri.conf.json] [Source: src-tauri/capabilities/booth-window.json]
- `package.json` 기준 현재 앱 baseline은 `@tauri-apps/api` 2.10.1, `react-router-dom` 7.13.1, `zod` 4.3.6, React 19.2.4다. Story 1.6은 이 baseline 위에서 helper boundary만 보강해야 한다. [Source: package.json]

### 관련 이전 스토리 인텔리전스

- Story 1.5는 host capture pipeline, `session.json` correlation, `Preview Waiting` separation을 `src-tauri/src/capture/ingest_pipeline.rs`, `src-tauri/src/capture/normalized_state.rs`, `src/capture-adapter/services/capture-runtime.ts`, `src/session-domain/state/session-provider.tsx` 중심으로 구현했다. 1.6은 이 seam을 깨지 말고 readiness truth만 real helper와 연결해야 한다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md] [Source: src-tauri/src/capture/ingest_pipeline.rs] [Source: src/capture-adapter/services/capture-runtime.ts] [Source: src/session-domain/state/session-provider.tsx]
- Story 5.4는 operator용 `카메라 연결 상태`가 booth와 같은 host-normalized truth family를 재사용해야 한다고 요구한다. 1.6이 helper truth를 이중화하면 5.4가 바로 흔들린다. [Source: _bmad-output/implementation-artifacts/5-4-운영자용-카메라-연결-상태-전용-항목과-helper-readiness-가시화.md]
- Story 6.2는 Story 1.6을 HV-02/HV-03/HV-10 gate로 닫는 canonical evidence policy를 요구한다. 1.6 문서 안의 closure 기준도 이 운영 규칙과 맞아야 한다. [Source: _bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md]

### Canon helper / sidecar 기준선

- sidecar protocol 계약은 helper가 Canon SDK initialize/session open/capture trigger/RAW download/reconnect 감지를 소유하고, host가 session/preset correlation, freshness, booth/operator projection, capture success 최종 확정을 소유한다고 고정한다. `helper-ready`는 boot 완료일 뿐 camera `ready`가 아니다. [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- EDSDK helper profile은 `canon-helper.exe`를 Windows 전용 helper로 두고, `canon-helper.exe`와 Canon EDSDK DLL을 같은 sidecar 경계에 배치하라고 권장한다. 또한 helper version, sdk version, last fresh `camera-status` sequence를 evidence에 남기라고 요구한다. [Source: docs/contracts/camera-helper-edsdk-profile.md]
- 연구 문서는 최종 채택 방향을 `Windows 전용 Canon EDSDK helper exe + Tauri sidecar contract`로 정리하고, 실무 기본안을 C# self-contained exe로 권장한다. 다른 언어를 선택하더라도 추가 런타임 의존성 없이 같은 packaging/evidence 조건을 충족해야 한다. [Source: _bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md]

### 최신 기술 확인 메모

- Tauri sidecar 공식 문서(마지막 업데이트 2026-01-07)는 packaged sidecar를 위해 `src-tauri/tauri.conf.json`의 `bundle.externalBin`을 사용하고, target triple suffix가 붙은 바이너리를 준비해야 한다고 안내한다. Rust에서 sidecar를 띄울 때는 `tauri_plugin_shell::ShellExt`와 `app.shell().sidecar("name")`를 사용하며, `sidecar()`에는 전체 경로가 아니라 filename만 넘겨야 한다. [Source: https://v2.tauri.app/develop/sidecar/]
- Tauri Shell plugin 공식 문서(마지막 업데이트 2025-02-22)는 plugin 사용 전에 `cargo add tauri-plugin-shell`, `tauri::Builder::default().plugin(tauri_plugin_shell::init())`, 필요 시 `@tauri-apps/plugin-shell` 설치를 요구한다. 현재 repo의 Rust version 1.77.2는 plugin 최소 요구와 맞는다. [Source: https://v2.tauri.app/ko/plugin/shell/] [Source: src-tauri/Cargo.toml]
- 추론: 이번 스토리의 올바른 경계는 Rust host-owned spawn/monitor다. 따라서 booth React가 `Command.sidecar(...)`로 helper를 직접 실행하도록 capability를 열기보다, host 내부에서 spawn하고 React는 typed readiness만 소비하는 편이 architecture와 더 맞다. [Source: _bmad-output/planning-artifacts/architecture.md#API-&-Communication-Patterns] [Source: _bmad-output/planning-artifacts/architecture.md#Frontend-Architecture] [Source: https://v2.tauri.app/develop/sidecar/]
- Canon CAP overview(Updated as of April 2025)는 EDSDK를 USB wired control 경로로 설명하고, Windows 10/11 지원과 Windows sample program 언어 `VB`, `C++`, `C#`를 명시한다. Canon EDSDK release note 페이지에서 확인되는 최신 공개 버전은 2025-09-24 `Ver 13.20.10`이다. [Source: https://asia.canon/en/campaign/developerresources/camera/cap] [Source: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note]

### 구현 가드레일

- `helper-ready`를 `camera ready`로 해석하면 안 된다.
- stale status, mismatched session, reconnect 직후 old status, helper exit 이후 마지막 `ready` snapshot을 그대로 믿으면 안 된다.
- React component가 raw helper detailCode, stdout/stderr, diagnostics path를 읽어 최종 readiness를 판단하면 안 된다.
- booth `Ready`, `사진 찍기` 활성화, operator camera/helper projection은 모두 host normalization에서 파생돼야 한다.
- Story 1.6의 `Ready`를 카메라 본체 셔터 직접 입력까지 지원한다는 의미로 확장 해석하면 안 된다.
- 고객 copy에는 helper, sidecar, USB, SDK, diagnostics path, raw enum 이름이 새면 안 된다.
- Story 1.7 범위인 capture/file handoff를 1.6 closure 조건으로 다시 끌어오면 안 된다.

### 아키텍처 준수사항

- camera/helper truth, timing truth, completion truth는 host에서 한 번 정규화된 뒤 booth copy와 operator diagnostics로 갈라져야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#API-&-Communication-Patterns]
- camera integration은 bundled helper/sidecar boundary 뒤에 두고, camera SDK truth가 React로 직접 새면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Core-Architectural-Decisions]
- FR-003/FR-004 구현 중심 경로는 `src/booth-shell/`, `src/capture-adapter/`, `src-tauri/src/capture/`다. sidecar 관련 새 책임도 이 경계를 우선 따라야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-to-Structure-Mapping]
- sidecar communication은 `src-tauri/src/capture/sidecar_client.rs`와 `sidecar/canon-helper/`에 격리돼야 하며, helper는 두 번째 session/preset/timing truth source가 되면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- session-scoped filesystem root가 active booth work의 durable truth이고, SQLite나 route state가 이를 대체하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Core-Architectural-Decisions]

### 프로젝트 구조 요구사항

- 우선 수정/생성 후보 경로:
  - `sidecar/canon-helper/`
  - `src-tauri/Cargo.toml`
  - `src-tauri/src/lib.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/src/capture/sidecar_client.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src/shared-contracts/schemas/capture-readiness.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src-tauri/tests/capture_readiness.rs`
  - `src-tauri/tests/operator_diagnostics.rs`
- 필요 시 보조 문서/자산 후보:
  - `sidecar/canon-helper/README.md`
  - `docs/contracts/camera-helper-sidecar-protocol.md`
  - `docs/contracts/camera-helper-edsdk-profile.md`
  - `docs/runbooks/booth-hardware-validation-checklist.md`

### UX 구현 요구사항

- customer는 계속 plain-language readiness guidance만 봐야 한다. blocked 상태에서 기술 용어 대신 wait/call guidance만 보여야 한다. [Source: _bmad-output/planning-artifacts/prd.md#FR-003-Readiness-Guidance-and-Valid-State-Capture]
- `Ready`가 아니면 capture가 차단돼야 하고, previously-ready 이후 degrade가 오면 즉시 `Ready`를 내려야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Story-1.4:-준비-상태-안내와-유효-상태에서만-촬영-허용] [Source: _bmad-output/planning-artifacts/epics.md#Story-1.6:-실카메라/helper-readiness-truth-연결과-false-ready-차단]
- customer state copy budget은 한 개의 주요 문장, 한 개의 보조 문장, 한 개의 주요 액션을 넘기면 안 된다. helper hardening 때문에 copy density가 늘어나면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-001-Customer-Guidance-Density-and-Simplicity]

### 테스트 요구사항

- 최소 필수 자동화 범위:
  - helper truth가 없거나 stale이면 `Ready`가 열리지 않는다.
  - session mismatch 상태의 helper status는 `Ready`를 만들지 않는다.
  - first fresh `camera-status=ready`가 들어오면 그때만 `Ready`가 열린다.
  - previously-ready 이후 disconnect, helper exit, degraded-after-ready가 오면 즉시 blocked path로 내려간다.
  - reconnect 후 첫 fresh truth 전에는 자동 복귀하지 않는다.
  - operator diagnostics가 booth와 같은 `liveCaptureTruth` family를 재사용한다.
- 최소 필수 수동 evidence:
  - HV-02: 카메라 미연결 차단 화면, 차단된 촬영 시도, `session.json`
  - HV-03: `Ready` 화면, helper 정상 동작 로그 또는 상태 캡처, 최근 `camera-status` freshness 근거, `session.json`
  - HV-10: 분리 직후 화면, 재연결 후 `Ready` 화면, recovery-status 또는 최근 `camera-status` sequence, `session.json` 전후 비교

### 금지사항 / 안티패턴

- `helper-ready`만 보고 `Ready`를 여는 것 금지
- booth React 또는 browser fixture가 raw helper 상태를 직접 해석해 `Ready`를 만드는 것 금지
- stale status file이 남아 있다는 이유로 reconnect 직후 자동 `Ready` 복귀시키는 것 금지
- `canon-helper.exe`와 Canon SDK payload를 임시 수동 경로에만 두고 packaging 기준을 남기지 않는 것 금지
- customer 화면에 helper, SDK, USB, diagnostics path, enum 이름을 보여 주는 것 금지
- Story 1.7 범위까지 끌어와 1.6 closure를 다시 흐리는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- 스프린트 변경 제안: `_bmad-output/planning-artifacts/sprint-change-proposal-20260328-023725.md`
- Canon helper sidecar 계약: `docs/contracts/camera-helper-sidecar-protocol.md`
- Canon EDSDK helper 구현 프로파일: `docs/contracts/camera-helper-edsdk-profile.md`
- hardware validation runbook: `docs/runbooks/booth-hardware-validation-checklist.md`
- Canon helper 기술 방향 연구: `_bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.4:-준비-상태-안내와-유효-상태에서만-촬영-허용]
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.5:-현재-세션-촬영-저장과-truthful-preview-waiting-피드백]
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.6:-실카메라/helper-readiness-truth-연결과-false-ready-차단]
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.7:-실카메라-capture-round-trip과-RAW-handoff-correlation]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-003-Readiness-Guidance-and-Valid-State-Capture]
- [Source: _bmad-output/planning-artifacts/prd.md#Release-Gates]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-001-Customer-Guidance-Density-and-Simplicity]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core-Architectural-Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#API-&-Communication-Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-to-Structure-Mapping]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260328-023725.md]
- [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- [Source: _bmad-output/implementation-artifacts/5-4-운영자용-카메라-연결-상태-전용-항목과-helper-readiness-가시화.md]
- [Source: _bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md]
- [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- [Source: docs/contracts/camera-helper-edsdk-profile.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: _bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md]
- [Source: package.json]
- [Source: sidecar/canon-helper/README.md]
- [Source: src/shared-contracts/schemas/capture-readiness.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src-tauri/Cargo.toml]
- [Source: src-tauri/src/lib.rs]
- [Source: src-tauri/tauri.conf.json]
- [Source: src-tauri/capabilities/booth-window.json]
- [Source: src-tauri/src/capture/sidecar_client.rs]
- [Source: src-tauri/src/capture/normalized_state.rs]
- [Source: src-tauri/src/capture/ingest_pipeline.rs]
- [Source: src/capture-adapter/services/capture-runtime.ts]
- [Source: src/session-domain/state/session-provider.tsx]
- [Source: https://v2.tauri.app/develop/sidecar/]
- [Source: https://v2.tauri.app/ko/plugin/shell/]
- [Source: https://asia.canon/en/campaign/developerresources/camera/cap]
- [Source: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-03-28 03:xx +09:00 - Story 1.6, sprint change proposal, hardware validation runbook, helper protocol, EDSDK profile, current repo capture/readiness code, official Tauri/Canon docs를 교차 검토해 story를 재작성했다.
- 2026-03-28 03:xx +09:00 - 현재 repo는 host-normalized readiness seam은 이미 존재하지만, real helper project와 packaged spawn boundary는 아직 없다는 점을 확인했다.
- 2026-03-29 22:01:35 +09:00 - stale helper 정리, parent PID 종료 연동, helper handoff 안정화 수정 이후 자동화 검증과 실장비 HV-02/HV-03/HV-10 후보 증거를 확인했다.
- 2026-04-10 10:12:02 +09:00 - linked session evidence와 직접 시각 검증을 canonical ledger close row에 반영해 Story 1.6을 `done`으로 정리했다.

### Implementation Plan

- real `canon-helper.exe` baseline과 packaging 경계를 먼저 만든다.
- host spawn/health/recovery를 Rust 내부로 고정하고 booth/operator가 같은 `liveCaptureTruth`를 재사용하게 유지한다.
- freshness/disconnect/reconnect false-ready 차단과 HV-02/HV-03/HV-10 evidence를 함께 닫는다.

### Completion Notes List

- `helper-ready` 단독 신호로는 `Ready`가 열리지 않도록 host freshness gate를 유지하고, fresh `camera-status` 확인 뒤에만 촬영 가능 상태가 열리도록 고정했다.
- stale `canon-helper.exe`가 다음 세션을 오염시키지 않도록 helper 정리와 parent PID 종료 연동을 추가해 `Phone Required` 회귀를 막았다.
- booth/customer 문구는 계속 plain-language로 유지하고, operator surface만 같은 `liveCaptureTruth`를 재사용하도록 정리했다.
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`, `dotnet build sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`, `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`를 통과했다. linked session evidence와 직접 시각 검증을 ledger close row에 반영해 HV-02/HV-03/HV-10 hardware gate를 `Go`로 닫았고, Story 1.6을 `done`으로 정리했다.

### File List

- _bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- history/camera-helper-troubleshooting-history.md
- history/camera-capture-validation-history.md
- sidecar/canon-helper/src/CanonHelper/CanonHelperOptions.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/ParentProcessMonitor.cs
- src-tauri/src/capture/helper_supervisor.rs
- src-tauri/src/capture/normalized_state.rs
- src-tauri/tests/capture_readiness.rs
- src-tauri/tests/operator_diagnostics.rs

### Change Log

- 2026-04-10 10:12:02 +09:00 - linked session evidence와 직접 시각 검증을 canonical ledger close row에 반영해 Story 1.6 hardware gate를 `Go`로 닫고 상태를 `done`으로 올렸다.
