# Story 1.23: host-owned local native/GPU resident full-screen lane prototype과 truthful artifact generation

Status: in-progress

Interpretation Note: Story 1.23의 구현 문맥은 재정리되지만, 현재 상태는 `done`이 아니라 `in-progress`다. Story 1.24, 1.25, 1.27, 1.13이 각각 canary, default/rollback, corrective rerun, final close ownership을 계속 가지며, Story 1.23은 governance verification, scope separation, release-field wording correction이 끝나기 전까지 완료로 취급하지 않는다. [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] [Source: docs/release-baseline.md]

Ordering Note: active forward path는 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25 -> 1.27 -> 1.13`이다. Story 1.23은 Story 1.22가 selected-capture evidence chain을 다시 잠근 다음에 와야 하며, local lane prototype owner만 맡는다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md] [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260415.md]

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
host-owned local native/GPU resident full-screen lane prototype로 display-sized preset-applied truthful artifact를 만들고 싶다,
그래서 darktable-compatible path를 parity, fallback, final reference로 유지한 채 future primary close candidate를 실제 하드웨어에서 검증할 수 있다.

## Acceptance Criteria

1. approved booth hardware와 feature flag 또는 scoped route가 있을 때 host-owned local lane prototype을 활성화하면, resident local lane은 same-capture, same-session, same-preset-version 기준의 display-sized preset-applied truthful artifact를 생성해야 한다. darktable-compatible path는 parity/fallback/final reference로 계속 남아야 한다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: _bmad-output/planning-artifacts/architecture.md]
2. first-visible source와 preset-applied truthful preview를 함께 운영할 때, customer-safe projection은 먼저 보여도 `previewReady` truth owner가 되면 안 된다. truthful close와 full-screen visible 판단은 local lane artifact 준비 이후에만 닫혀야 한다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: docs/contracts/render-worker.md] [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
3. local lane이 실패하거나 health를 잃을 때 booth는 false-ready, false-complete, wrong-capture, cross-session leakage 없이 darktable-compatible baseline path로 fail-closed 되어야 한다. operator-safe evidence에는 최소 `laneOwner`, `fallbackReasonCode`, `routeStage`, `warmState`, `truthfulArtifactReadyAtMs`, `visibleOwner`, `visibleOwnerTransitionAtMs`, `sameCaptureFullScreenVisibleMs`와 capture-time route/catalog snapshot이 남아야 한다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: docs/contracts/local-dedicated-renderer.md] [Source: docs/runbooks/preview-promotion-evidence-package.md]

## Tasks / Subtasks

- [x] host-owned local lane을 same-capture truthful artifact prototype으로 고정한다. (AC: 1, 2)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`, `src-tauri/src/render/mod.rs`, `src-tauri/src/capture/ingest_pipeline.rs` 또는 동등 경로에서 local lane output이 display-sized preset-applied truthful artifact를 만들고, `capture_preview_ready` 및 `capture_preview_transition_summary` 계열 진단과 연결되게 유지한다.
  - [x] `previewReady`와 full-screen visible close는 actual truthful artifact readiness 이후에만 닫히고, same-capture first-visible은 supporting signal로만 남게 한다.
  - [x] RAW copy, placeholder, bundle 대표 preview tile, tiny preview, recent strip update를 truthful close success로 승격하지 않는다.

- [ ] darktable-compatible parity/fallback/final reference를 유지한다. (AC: 1, 3)
  - [x] `docs/contracts/render-worker.md`, `docs/contracts/local-dedicated-renderer.md`, `docs/contracts/session-manifest.md`와 같은 계약 문서 기준으로 darktable-compatible path를 제거하거나 local lane과 같은 의미로 합치지 않는다.
  - [ ] local lane은 prototype owner이고, darktable-compatible path는 parity oracle, fallback truth, final/export truth reference로 남긴다.
  - [x] Story 1.23 안에서 remote renderer / edge appliance를 기본 경로처럼 열지 않는다.

- [ ] capture-time route와 session snapshot truth를 재사용한다. (AC: 1, 3)
  - [x] `activePreviewRendererRoute`, `activePreviewRendererWarmState`, capture-bound preset/version binding은 host-owned session snapshot 위에서만 해석한다.
  - [x] publish, rollback, later policy change가 생겨도 이미 기록된 active session capture meaning을 live state로 재해석하지 않는다.
  - [ ] Story 1.23은 prototype owner까지만 맡고, Story 1.24 canary, Story 1.25 default/rollback, Story 1.27 corrective rerun, Story 1.13 final close ownership을 흡수하지 않는다.

- [x] fail-closed fallback과 operator-safe evidence를 잠근다. (AC: 3)
  - [x] queue saturation, warm-state loss, invalid output, timeout, capture mismatch, preset drift 시 booth는 `Preview Waiting`과 approved fallback path로만 내려가게 유지한다.
  - [x] selected-capture evidence bundle은 capture-time `preview-renderer-policy.json`, `catalog-state.json`, `preview-promotion-evidence.jsonl`, `timing-events.log`, booth/operator visual evidence를 함께 유지한다.
  - [x] `src/shared-contracts/schemas/hardware-validation.ts`, `src/shared-contracts/schemas/operator-diagnostics.ts`, `src/operator-console/services/operator-diagnostics-service.ts`, `src/operator-console/screens/OperatorSummaryScreen.tsx` 또는 동등 경로에서 lane/fallback/warm-state/full-screen KPI vocabulary를 일관되게 읽는다.

- [ ] prototype scope를 회귀 테스트와 governance wording으로 고정한다. (AC: 1, 2, 3)
  - [ ] `src-tauri/tests/dedicated_renderer.rs`, `src-tauri/tests/session_manifest.rs`, `src-tauri/tests/branch_rollout.rs`, `src/shared-contracts/contracts.test.ts`, `src/operator-console/services/operator-diagnostics-service.test.ts`, `tests/hardware-evidence-scripts.test.ts`, `src/governance/hardware-validation-governance.test.ts` 또는 동등 검증에서 prototype success, fallback, wrong-capture 차단, snapshot continuity를 계속 보호한다.
  - [ ] Story 1.23의 완료 의미는 prototype proof까지이며, approved-hardware `Go`, repeated success threshold, one-action rollback proof, final release close는 요구하지 않는다.

## Current Unresolved Issues

- `src/governance/hardware-validation-governance.test.ts`가 아직 red라서 Story 1.23 완료 문구를 governance가 승인하지 않는다.
- `src-tauri/src/branch_config/mod.rs`에는 canary/default gate 성격의 로직이 남아 있어 Story 1.23 prototype owner 경계와 완전히 분리됐다고 보기 어렵다.
- `docs/contracts/local-dedicated-renderer.md`가 요구하는 `sameCaptureFullScreenVisibleMs`와 legacy `replacementMs` 분리 해석이 evidence 기록과 아직 완전히 일치하지 않는다.

## Dev Notes

### Product Intent

- Story 1.23은 active forward path에서 local lane prototype owner다. 이 스토리는 `same-capture preset-applied full-screen visible <= 2500ms` 제품 KPI를 직접 닫는 final gate가 아니라, 그 KPI를 local lane이 설명할 수 있는 artifact owner 후보를 올려보는 단계다. [Source: _bmad-output/planning-artifacts/prd.md] [Source: _bmad-output/planning-artifacts/architecture.md]
- Story 1.22가 selected-capture evidence chain을 다시 잠갔기 때문에, Story 1.23은 같은 correlation 위에서 artifact owner 후보만 local lane으로 옮기는 follow-up이어야 한다. [Source: _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md]

### Scope Boundaries

- 이번 스토리가 소유하는 것:
  - host-owned local native/GPU resident full-screen lane prototype
  - display-sized preset-applied truthful artifact generation
  - fail-closed fallback과 selected-capture-safe evidence continuity
- 이번 스토리가 소유하지 않는 것:
  - hardware canary `Go / No-Go`
  - default route promotion 또는 one-action rollback completion
  - corrective rerun acceptance
  - final guarded cutover / release close

### Current Repository Baseline

- `src-tauri/src/render/dedicated_renderer.rs`는 `preview-promotion-evidence-record/v1`, `capture_preview_ready`, `capture_preview_transition_summary`, `laneOwner`, `routeStage`, `visibleOwner`, `truthfulArtifactReadyAtMs`, `sameCaptureFullScreenVisibleMs` 계열을 이미 남긴다. 새 evidence family를 만들기보다 현재 host-owned render/evidence boundary를 prototype owner 의미로 읽는 편이 맞다. [Source: src-tauri/src/render/dedicated_renderer.rs] [Source: src/shared-contracts/schemas/hardware-validation.ts]
- `src-tauri/src/session/session_manifest.rs`와 `docs/contracts/session-manifest.md` 기준으로 active session은 `activePreviewRendererRoute`와 `activePreviewRendererWarmState` snapshot을 유지한다. Story 1.23은 이 capture-time snapshot을 재사용해야지, live route/catalog state로 recorded capture를 다시 해석하면 안 된다. [Source: src-tauri/src/session/session_manifest.rs] [Source: docs/contracts/session-manifest.md]
- `src-tauri/src/branch_config/mod.rs`와 `src-tauri/tests/branch_rollout.rs`는 canary/default promotion evidence 누적을 다루지만, 그 decision gate owner는 Story 1.24/1.25 이후에 있다. Story 1.23은 promotion authority를 가져오지 않는다. [Source: src-tauri/src/branch_config/mod.rs] [Source: src-tauri/tests/branch_rollout.rs]
- `tests/hardware-evidence-scripts.test.ts`, `docs/runbooks/preview-promotion-evidence-package.md`, `src/governance/hardware-validation-governance.test.ts`는 selected-capture bundle, operator-safe evidence, governance wording을 이미 강하게 보호하고 있다. Story 1.23은 이 규칙을 깨지 않는 범위에서 prototype path를 연결해야 한다. [Source: tests/hardware-evidence-scripts.test.ts] [Source: docs/runbooks/preview-promotion-evidence-package.md] [Source: src/governance/hardware-validation-governance.test.ts]

### Architecture Compliance

- primary customer-visible truthful close owner 후보는 host-owned local native/GPU resident full-screen lane이다. 단, darktable-compatible path는 계속 parity/fallback/final reference로 남는다. [Source: _bmad-output/planning-artifacts/architecture.md] [Source: docs/contracts/render-worker.md]
- `previewReady`는 canonical recipe intent와 same-capture preset-applied preview file 생성 뒤에만 기록돼야 한다. first-visible image가 먼저 보이더라도 booth 상태는 `Preview Waiting`을 유지해야 한다. [Source: docs/contracts/render-worker.md] [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- remote renderer / edge appliance는 reserve path다. local lane prototype, parity, canary, corrective rerun을 반복해도 KPI를 못 닫을 때만 열 수 있으며, Story 1.23에서 기본 대안처럼 취급하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md]

### File Structure Notes

- 우선 확인/변경 후보:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/branch_config/mod.rs`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/operator-console/services/operator-diagnostics-service.ts`
  - `src/operator-console/screens/OperatorSummaryScreen.tsx`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/session_manifest.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `tests/hardware-evidence-scripts.test.ts`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/local-dedicated-renderer.md`
  - `docs/contracts/session-manifest.md`
  - `docs/runbooks/preview-promotion-evidence-package.md`
- 주의할 경계:
  - `docs/release-baseline.md`
  - `release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - final close owner나 reserve path wording은 Story 1.23 범위를 넘기지 않도록 유지한다.

### UX and Evidence Guardrails

- 고객 경험에서는 "사진은 저장되었고 확인용 사진을 준비 중"이라는 `Preview Waiting` 의미가 먼저 유지돼야 한다. same-capture first-visible이 먼저 나타나도 truthful close 준비 전에는 상태 자체가 성공처럼 바뀌면 안 된다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- latest photo rail은 same-capture 확인 이미지를 먼저 보여줄 수 있어도, 나중에 booth-safe preset-applied truthful preview가 같은 자리에서 자연스럽게 교체되어야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- operator surface는 `routeStage`, `laneOwner`, `fallbackReasonCode`, `warmState` 같은 내부 진단어를 볼 수 있지만, 고객-facing copy에는 이런 내부 용어를 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md]

### Testing Requirements

- 최소 필수 자동 검증:
  - local lane prototype이 same `sessionId/requestId/captureId/preset/version` 기준의 truthful artifact를 만든다.
  - `previewReady`, `truthfulArtifactReadyAtMs`, `visibleOwnerTransitionAtMs`, `sameCaptureFullScreenVisibleMs`가 같은 selected-capture chain으로 남는다.
  - local lane failure, timeout, warm-state loss, invalid output, capture mismatch 시 booth가 approved fallback으로 fail-closed 된다.
  - capture-time route/catalog snapshot이 evidence bundle에서 보존되고 live recopy가 금지된다.
  - operator diagnostics와 contract fixture가 lane owner, fallback reason, route stage, warm state, full-screen KPI를 함께 읽는다.
- 이번 스토리에서 요구하지 않는 검증:
  - final `Go / No-Go`
  - repeated approved-hardware canary success threshold
  - one-action rollback completion
  - Story 1.13 release-close row 갱신

### Latest Technical Context

- 현재 저장소는 `React 19.2.4`, `react-router-dom 7.13.1`, `Zod 4.3.6`, `Vite 8.0.1`, `@tauri-apps/api 2.10.1`, `tauri 2.10.3`, `Rust edition 2021`, `rust-version 1.77.2`를 사용한다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- render worker와 hardware validation 문맥은 darktable adapter pin을 `5.4.1`로 유지한다. Story 1.23에서는 external upgrade보다 현재 repo가 고정한 boundary와 evidence semantics를 정확히 따르는 편이 중요하다. [Source: docs/contracts/render-worker.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- 외부 최신 버전 조사는 이번 스토리의 핵심이 아니며, 위 판단은 현재 repo와 planning artifact를 근거로 한 해석이다.

### Anti-Patterns

- first-visible frame, tiny preview, recent-strip update, placeholder를 truthful full-screen close로 승격하는 것 금지
- live `preview-renderer-policy.json` 또는 live `catalog-state.json`를 다시 읽어 이미 기록된 capture를 재해석하는 것 금지
- darktable-compatible parity/fallback/final reference를 제거하고 local lane만 남기는 것 금지
- Story 1.23 안에서 canary gate, default decision, rollback completion, final close를 함께 흡수하는 것 금지
- remote renderer / edge appliance를 prototype 실패 전부터 기본 대안처럼 취급하는 것 금지

### References

- [Source: _bmad-output/planning-artifacts/epics.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260415.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: docs/release-baseline.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: src-tauri/src/session/session_manifest.rs]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src/operator-console/services/operator-diagnostics-service.ts]
- [Source: src/operator-console/screens/OperatorSummaryScreen.tsx]
- [Source: src-tauri/tests/dedicated_renderer.rs]
- [Source: src-tauri/tests/session_manifest.rs]
- [Source: src-tauri/tests/branch_rollout.rs]
- [Source: tests/hardware-evidence-scripts.test.ts]
- [Source: src/governance/hardware-validation-governance.test.ts]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Recreation Notes

- 2026-04-16 기준 config, epics, PRD, architecture, UX, sprint plan, sprint status, release baseline, hardware ledger, contract docs, current code/test 경로를 다시 교차 분석해 Story 1.23 문서를 새로 구성했다.
- 기존 문서는 제거하고 같은 경로에 최신 planning/contracts 해석을 반영한 새 문서를 다시 생성했다.
- 초기 재생성 시 sprint tracking의 `done` 상태는 release-hold 해석과 연결돼 있어 보드 상태를 유지했지만, 같은 날 재검증 결과 Story 1.23은 `in-progress`로 되돌려야 한다는 판단이 확정됐다.
- 별도 `project-context.md`는 발견되지 않았다.

### Debug Log References

- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `rg -n -C 12 "^### Story 1\\.23|^### Story 1\\.22|^### Story 1\\.24" _bmad-output/planning-artifacts/epics.md`
- `rg -n -C 8 "Story 1\\.23|1\\.23|local full-screen lane|truthful artifact generation|Initial Implementation Priorities|Preview Architecture Realignment" _bmad-output/planning-artifacts/architecture.md _bmad-output/planning-artifacts/prd.md _bmad-output/planning-artifacts/ux-design-specification.md`
- `rg -n -C 8 "Story 1\\.23|1\\.23|1-23" _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md _bmad-output/planning-artifacts/implementation-readiness-report-20260415.md`
- `rg -n "preview-promotion-evidence-record|capture_preview_ready|capture_preview_transition_summary|sameCaptureFullScreenVisibleMs|truthfulArtifactReadyAtMs|laneOwner|fallbackReasonCode|routeStage|visibleOwner" src-tauri src tests docs`
- `Get-Content -Raw package.json`
- `Get-Content -Raw src-tauri/Cargo.toml`

### File List

- _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md

### Change Log

- 2026-04-16 16:00 +09:00 - Story 1.23 문서를 최신 planning/contracts/release-hold 해석에 맞춰 재생성했다. sprint tracking status는 유지하고, 문서 내용만 새 컨텍스트 기준으로 교체했다.
- 2026-04-16 17:09 +09:00 - governance 검증 실패, scope bleed, release-field wording mismatch를 반영해 Story 1.23 상태를 `in-progress`로 되돌리고 관련 완료 체크를 해제했다.
