# Story 1.27: local hot path darktable 절연과 2500ms KPI 재검증

Status: done

Ordering Note: Story 1.27은 Story 1.25 이후의 corrective local-path follow-up이다. 이 스토리는 Story 1.13 final guarded close를 대신하지 않고, Story 1.26 reserve track도 열지 않는다. 목적은 local lane hot path가 실제로 darktable-bound close에서 벗어날 수 있는지 다시 입증하는 것이다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
local lane hot path가 darktable preview run에 다시 묶이지 않는지 증명하고 싶다,
그래서 reserve path를 열기 전에 local forward path의 실제 가능성을 마지막으로 검증할 수 있다.

## Acceptance Criteria

1. approved booth hardware와 approved preset/version scope가 있을 때, local lane hot path를 실행하면 `display-sized preset-applied truthful artifact`는 host-owned local lane에서 닫혀야 한다. 또한 darktable-compatible path는 parity/fallback/final reference로만 남아야 하며, operator-safe evidence에는 local hot path가 darktable-compatible preview run으로 떨어졌는지 여부와 reason이 함께 남아야 한다.
2. cold 1컷 + 연속 3~5컷 hardware run을 같은 evidence bundle 기준으로 검토할 때, `sameCaptureFullScreenVisibleMs`, `wrong-capture`, `fidelity drift`, `fallback ratio`, `follow-up capture completion`을 함께 읽을 수 있어야 한다. 또한 KPI miss 또는 follow-up capture timeout이 남으면 verdict는 fail-closed `No-Go`로 유지되어야 한다.
3. local path가 health를 잃거나 parity/fidelity를 만족하지 못할 때, booth는 false-ready, wrong-capture, cross-session leakage 없이 baseline path로 fail-closed 되어야 한다. 또한 Story 1.13 또는 Story 1.26은 이 스토리 결과만으로 자동 개시되면 안 된다.

## Tasks / Subtasks

- [x] local hot path provenance를 evidence/diagnostics에 명시적으로 남긴다. (AC: 1, 2, 3)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`, `src/shared-contracts/schemas/hardware-validation.ts`, `src/shared-contracts/schemas/operator-diagnostics.ts` 또는 동등 경로에서 hot path truthful artifact producer와 darktable-compatible fallback/proxy 여부를 구분하는 typed field를 추가하거나 정리한다.
  - [x] `sameCaptureFullScreenVisibleMs`가 local lane close를 기준으로 읽혔는지, darktable fallback close를 기준으로 읽혔는지 operator-safe하게 판별 가능해야 한다.
  - [x] capture-time route/catalog snapshot과 selected-capture correlation 규칙은 Story 1.22 baseline을 그대로 유지한다.

- [x] latency-critical close path가 darktable preview run에 다시 묶이지 않도록 bounded implementation을 적용한다. (AC: 1, 3)
  - [x] `src-tauri/src/render/`, `sidecar/dedicated-renderer/`, `src-tauri/src/capture/ingest_pipeline.rs` 또는 동등 경로에서 display-sized truthful artifact hot path가 darktable-compatible preview invocation을 직접 기다리지 않도록 정리한다.
  - [x] darktable-compatible path는 parity comparison, fallback truth, final/export reference로만 유지하고, 이번 스토리에서 제거하거나 계약을 깨지 않는다.
  - [x] local path가 실패하면 booth는 approved fallback으로만 내려가고, 추정 성공이나 speed-only success를 선언하지 않는다.

- [x] follow-up capture 경계와 local close 경계를 함께 재검증 가능하게 만든다. (AC: 2, 3)
  - [x] `history/camera-capture-validation-history.md`의 2026-04-16 후속처럼 `request-capture -> capture-accepted -> file-arrived` 지연과 `preset-applied truthful close` 지연을 같은 문제로 뭉개지 말고, cold 1컷 + 연속 3~5컷에서 두 seam을 분리해 읽을 수 있게 한다.
  - [x] `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`, `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs`, `src-tauri/src/capture/sidecar_client.rs` 또는 동등 경로에서 best-effort preview maintenance가 새 capture request 소비보다 앞서지 않게 유지하고, helper completion 경계와 preview close 경계를 같은 evidence bundle에서 비교 가능하게 만든다.
  - [x] `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`, `scripts/hardware/Test-PreviewPromotionCanary.ps1`, `docs/runbooks/preview-promotion-evidence-package.md` 또는 동등 경로에서 selected capture bundle이 request-capture, capture-accepted, file-arrived, fast-preview-visible, truthful close, follow-up capture health를 함께 설명하도록 필요한 artifact 복사를 정리한다.
  - [x] local path correction이 있어도 wrong-capture, stale-preview, cross-session attribution drift는 fail-closed로 유지한다.

- [x] hardware rerun gate와 regression을 Story 1.27 기준으로 잠근다. (AC: 1, 2, 3)
  - [x] `tests/hardware-evidence-scripts.test.ts`, `src-tauri/tests/dedicated_renderer.rs`, `src/operator-console/services/operator-diagnostics-service.test.ts` 또는 동등 검증에 local-hot-path success, darktable-bound fallback, KPI miss, follow-up timeout, fidelity drift, wrong-capture negative case를 추가한다.
  - [x] 이번 스토리는 `Go`를 선언하는 owner가 아니라, local path viability를 다시 입증하거나 `No-Go`를 더 명확히 만드는 owner임을 문서와 테스트에서 남긴다.

### Review Findings

- [x] [Review][Patch] Canary가 요구하는 `visibleOwner`/`visibleOwnerTransitionAtMs`를 실제 booth 로그가 남기지 않음 [src/booth-shell/components/SessionPreviewImage.tsx:144]
- [x] [Review][Patch] Evidence bundle/canary가 `capture-accepted`·fast-preview·follow-up capture timeout seam을 전혀 판독하지 못함 [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:77]
- [x] [Review][Patch] Helper maintenance 우선순위 보정이 request 도착 직후 race를 막지 못해 follow-up capture starvation이 남아 있음 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs:56]

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- `docs/preview-architecture-history-and-agent-guide.md`는 현재 문제가 "새 아키텍처 미적용"이 아니라, 새 경로가 적용됐어도 KPI를 닫지 못한 상태라고 정리한다.
- 같은 문서는 다음 정답 후보를 `local native/GPU resident full-screen lane`의 실제 KPI 증명으로 고정하고, low-risk tuning 반복은 가치가 낮다고 정리한다.
- `history/camera-capture-validation-history.md`의 2026-04-16 기록은 warm-hit 조건에서도 6~7초대 close와 follow-up capture timeout이 남아 있고, 병목이 `dedicated renderer가 darktable-cli로 preview close를 만드는 자체 시간`으로 좁혀졌다고 설명한다.
- 같은 히스토리의 2026-04-16 10:29 후속은 어떤 세션에서는 `sameCaptureFullScreenVisibleMs`보다 먼저 `request-capture -> file-arrived` 구간이 30초대까지 밀릴 수 있었고, helper의 optional preview maintenance 우선순위가 새 셔터 요청을 늦춘 사례가 있었음을 남긴다. 따라서 Story 1.27은 darktable-bound close뿐 아니라 capture request consumption 경계도 함께 분리 측정해야 한다.

### 스토리 목적과 범위

- 이번 스토리의 핵심은 local lane이 실제 latency-critical hot path에서 darktable-bound close를 벗어날 수 있는지 다시 증명하는 것이다.
- 이번 스토리는 아래를 소유한다.
  - local hot path provenance 명시화
  - bounded local-path implementation correction
  - cold 1컷 + 연속 3~5컷 기준 hardware rerun readiness
- 아래 작업은 이번 스토리 범위가 아니다.
  - Story 1.13 final guarded cutover / ledger close
  - Story 1.26 reserve remote POC 개시
  - PRD / architecture 방향 재정의

### 스토리 기반 요구사항

- architecture는 primary customer-visible close path를 `host-owned local native/GPU resident full-screen lane`이 만드는 `display-sized preset-applied truthful artifact`로 정의한다. [Source: _bmad-output/planning-artifacts/architecture.md#System Overview]
- architecture와 변경 제안서는 darktable-compatible path를 parity/fallback/final reference로 유지하되, hot path 기본안으로 되돌리면 안 된다고 정리한다. [Source: _bmad-output/planning-artifacts/architecture.md#Preview Architecture Realignment] [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
- release baseline은 Story 1.13이 canonical close owner이며, `No-Go`나 missing evidence가 있으면 계속 `release hold`라고 명시한다. [Source: docs/release-baseline.md]
- preview guide는 Story 1.21~1.25 완료가 곧바로 release `Go`가 아니며, 현재 필요한 것은 local lane KPI 증명이라고 정리한다. [Source: docs/preview-architecture-history-and-agent-guide.md]

### 현재 워크스페이스 상태

- `src-tauri/src/render/dedicated_renderer.rs`와 관련 evidence family는 이미 `sameCaptureFullScreenVisibleMs`, `laneOwner`, `routeStage`, `warmState`, `fallbackReason`를 남긴다. 이번 스토리는 새 evidence family를 발명하기보다 hot path provenance를 더 분명히 하는 follow-up이 맞다.
- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`와 `Test-PreviewPromotionCanary.ps1`는 selected-capture bundle과 canary 판단 기준을 이미 갖고 있다. Story 1.27은 이 bundle이 local hot path와 follow-up capture health를 더 잘 설명하게 맞추는 쪽이 안전하다.
- Story 1.23은 현재 `in-progress`로 되돌아갔고, Story 1.24와 1.25만 canary/default follow-up 구현 문서로 `done` 상태를 유지한다. canonical `Go` acceptance와 final close ownership은 여전히 Story 1.13에 남겨 두고 있다.
- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`는 실제 helper loop에서 pending request 처리와 preview maintenance 순서를 소유하고, `CanonSdkCamera.cs`는 `TryCompletePendingFastPreviewDownload()` 및 `TryBackfillPreviewAssets()` 경계를 갖는다. Story 1.27은 이 helper 경계를 preview close 병목과 구분해 읽어야 한다.
- `src-tauri/src/capture/sidecar_client.rs`와 `src-tauri/tests/capture_readiness.rs`는 `fast-preview-ready` 및 capture round-trip contract를 이미 갖고 있어, 이번 스토리는 별도 ad-hoc seam을 만들기보다 여기서 helper/request 경계 회귀를 잠그는 편이 안전하다.

### 이전 스토리 인텔리전스

- Story 1.22는 selected-capture evidence chain과 capture-time snapshot 규칙을 잠갔다. Story 1.27도 live policy recopy나 whole-session reinterpretation을 허용하면 안 된다. [Source: _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md]
- Story 1.23은 local lane prototype owner를, Story 1.24는 canary gate를, Story 1.25는 default/rollback gate를 잠갔다. Story 1.27은 이 ownership을 흡수하지 않고 corrective proof만 맡는다. [Source: _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md] [Source: _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md] [Source: _bmad-output/implementation-artifacts/1-25-local-lane-default-decision과-rollback-gate.md]

### 구현 가드레일

- `firstVisibleMs`, tiny preview, recent-session strip update만으로 성공으로 해석하면 안 된다.
- darktable-compatible path를 제거하거나 fidelity oracle 역할을 약화하면 안 된다.
- Story 1.27 결과가 좋아 보여도 Story 1.13을 자동 reopen하면 안 된다.
- Story 1.27 결과가 나빠도 Story 1.26을 자동 open하면 안 된다. repeated approved-hardware failure 판단은 별도 gate다.
- local path correction이 helper completion boundary를 더 악화시키면 안 된다. follow-up capture health를 같이 봐야 한다.

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보 경로:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/capture/sidecar_client.rs`
  - `sidecar/dedicated-renderer/`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
  - `scripts/hardware/Test-PreviewPromotionCanary.ps1`
  - `tests/hardware-evidence-scripts.test.ts`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src/operator-console/services/operator-diagnostics-service.test.ts`
  - `sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelperServiceTests.cs`
- scope를 넘기지 않도록 주의할 경로:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
  - `_bmad-output/implementation-artifacts/1-26-local-lane-실패-시에만-remote-reserve-poc-개시.md`

### 최신 기술 확인 메모

- 현재 로컬 기준선은 `react 19.2.4`, `@tauri-apps/api 2.10.1`, `zod 4.3.6`, Rust `tauri 2.10.3`이다. Story 1.27은 새 의존성 도입보다 existing command/contract/evidence 경계를 따라 hot path를 교정하는 편이 안전하다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- Tauri v2 공식 문서는 frontend-host 호출 기본 경계를 `invoke(...)` + Rust `#[tauri::command]`로 설명한다. 따라서 Story 1.27은 새 IPC 스타일을 발명하기보다 현재 command/event/schema 경계 위에서 hot path provenance와 diagnostics를 보강해야 한다. [Source: https://v2.tauri.app/develop/calling-rust/]
- darktable 공식 invocation 문서는 `--disable-opencl`과 `--library <library file>`를 계속 지원한다. 따라서 darktable-compatible path를 유지해야 하는 경우에도 이 옵션들은 bounded optimization 근거일 뿐, darktable가 latency-critical close owner로 복귀해도 된다는 근거는 아니다. [Source: https://darktable-org.github.io/dtdocs/en/special-topics/program-invocation/darktable/]
- Zod 공식 문서는 schema validation을 TypeScript-first contract layer로 유지하는 방향을 설명한다. 이번 스토리도 hardware-validation/operator-diagnostics/session-manifest schema family를 계속 확장하는 편이 ad-hoc parsing보다 안전하다. [Source: https://zod.dev/]

### 테스트 요구사항

- 최소 필수 검증:
  - local hot path success 시 evidence가 darktable-bound fallback과 명확히 구분된다
  - KPI miss면 fail-closed `No-Go` evidence가 남는다
  - `request-capture -> capture-accepted -> file-arrived` 지연과 `sameCaptureFullScreenVisibleMs` 지연이 한 bundle 안에서 분리 판독된다
  - follow-up capture timeout이 남으면 success처럼 읽히지 않는다
  - wrong-capture, fidelity drift, stale snapshot, cross-session contamination은 계속 차단된다
  - local path 실패 시 approved fallback으로만 내려가며 false-ready/false-complete를 선언하지 않는다
- 권장 추가 검증:
  - helper loop가 pending capture request가 있을 때 `TryCompletePendingFastPreviewDownload()` 또는 `TryBackfillPreviewAssets()` 때문에 request consumption을 늦추지 않는지 회귀를 추가한다
  - `fast-preview-ready` contract가 이미 닫힌 capture에서 canonical preview backfill 또는 preview maintenance가 중복 승격/중복 계측을 만들지 않는지 확인한다
  - operator diagnostics와 hardware evidence bundle이 `laneOwner`, `fallbackReasonCode`, `routeStage`, `sameCaptureFullScreenVisibleMs`, follow-up capture health를 같은 selected capture 문맥으로 묶는지 검증한다

### References

- [Source: docs/preview-architecture-history-and-agent-guide.md]
- [Source: history/camera-capture-validation-history.md]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
- [Source: docs/release-baseline.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md]
- [Source: _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md]
- [Source: _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md]
- [Source: _bmad-output/implementation-artifacts/1-25-local-lane-default-decision과-rollback-gate.md]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]
- [Source: https://v2.tauri.app/develop/calling-rust/]
- [Source: https://darktable-org.github.io/dtdocs/en/special-topics/program-invocation/darktable/]
- [Source: https://zod.dev/]

## Dev Agent Record

### Implementation Plan

- local hot path provenance를 typed evidence와 operator diagnostics에 유지하고, selected-capture correlation 및 capture-time snapshot 규칙은 Story 1.22 baseline 그대로 유지한다.
- local route가 warm 상태일 때 speculative close를 건너뛰어 display-sized truthful close가 darktable preview invocation에 다시 묶이지 않게 하고, fallback은 parity/final reference 용도로만 남긴다.
- helper best-effort preview maintenance는 pending capture request보다 뒤로 미루고, evidence bundle/canary에서 request-capture, file-arrived, truthful close, follow-up capture health를 같은 capture 문맥으로 묶어 fail-closed 판독을 유지한다.

### Debug Log

- 2026-04-16 12:25: `pnpm test:run -- tests/hardware-evidence-scripts.test.ts src/operator-console/services/operator-diagnostics-service.test.ts` 통과 (42 tests)
- 2026-04-16 12:31: `$env:CARGO_TARGET_DIR='C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\target-codex-verify-127'; cargo test --test dedicated_renderer -- --nocapture` 통과 (15 tests)
- 2026-04-16 12:46: `$env:CARGO_TARGET_DIR='C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\target-codex-verify-127'; cargo test --test capture_readiness -- --nocapture --test-threads=1` 통과 (66 tests)
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter CanonHelperServiceTests`는 Canon SDK vendor 미구성으로 실행 불가 (`Canon SDK source not found`). 대신 해당 경계에 대한 테스트 파일은 추가되어 있다.

### Completion Notes

- local hot path provenance, `sameCaptureFullScreenVisibleMs`, `visibleOwner`, `fallbackReasonCode`, selected-capture snapshot/evidence bundle이 operator-safe하게 연결되도록 정리됐다.
- warm local route에서는 speculative preview close를 생략하고, helper pending request가 있으면 optional preview maintenance를 뒤로 미뤄 follow-up capture 경계와 truthful close 경계를 분리해 읽을 수 있게 했다.
- canary/evidence regression은 local-hot-path success, darktable-bound fallback, KPI miss, wrong-capture, fidelity drift, follow-up timeout, rollback proof 부재를 `No-Go`로 묶도록 강화됐다.
- 1.27의 2500ms KPI 기준에 맞춰 `capture_readiness` 회귀 테스트 기대치를 최신 preview budget 상수로 정렬했다.

## File List

- docs/runbooks/preview-promotion-evidence-package.md
- scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1
- scripts/hardware/Test-PreviewPromotionCanary.ps1
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs
- sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelperServiceTests.cs
- sidecar/canon-helper/tests/CanonHelper.Tests/CaptureTimeoutTests.cs
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/capture/sidecar_client.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/capture_readiness.rs
- src-tauri/tests/dedicated_renderer.rs
- src/operator-console/services/operator-diagnostics-service.test.ts
- src/shared-contracts/schemas/hardware-validation.ts
- src/shared-contracts/schemas/operator-diagnostics.ts
- tests/hardware-evidence-scripts.test.ts

## Change Log

- 2026-04-16: Story 1.27 구현을 review 상태로 정리했다. local hot path provenance/evidence 강화, helper request 우선순위 보정, canary fail-closed 회귀 확장, 2500ms KPI 기준 검증 정렬을 포함한다.
