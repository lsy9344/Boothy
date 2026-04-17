# Story 1.28: actual primary lane close owner 구현과 prototype route 분리

Status: done

Ordering Note: Story 1.28은 Stories 1.23~1.27 prototype/evidence/gate history 다음에 시작되는 actual implementation track의 첫 스토리다. 이 스토리는 actual hot path 구현과 prototype route 분리를 소유하지만, Story 1.29 evidence/vocabulary realignment, Story 1.30 canary, Story 1.31 default/rollback, Story 1.13 final release close를 대신하지 않는다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] [Source: docs/release-baseline.md]

Reopen Note: 2026-04-17 리뷰 기준으로 이전 구현은 `implementationTrack=actual-primary-lane`을 legacy `local-renderer-sidecar`/`dedicated-renderer` 경로에 additive하게 붙인 수준이었다. 이는 final architecture가 요구한 distinct runtime close owner 전환을 충족하지 않는다. Story 1.28의 완료는 label 추가가 아니라, 제품의 실제 주 경로가 legacy owner와 분리된 actual lane으로 닫히는 시점이어야 한다. [Source: _bmad-output/planning-artifacts/architecture.md] [Source: docs/preview-architecture-history-and-agent-guide.md]

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
final architecture가 요구한 actual primary lane을 제품의 실제 close owner로 구현하고 싶다,
그래서 booth의 주 경로가 prototype dedicated-renderer evidence track과 분리되고 release-close 판단이 새 구조 위에서만 이뤄질 수 있게 하고 싶다.

## Acceptance Criteria

1. approved booth hardware와 approved preset/version scope가 있을 때 actual primary lane을 실행하면, `display-sized preset-applied truthful artifact`는 final architecture가 정의한 host-owned local native/GPU resident full-screen lane에서 닫혀야 한다. 이때 runtime owner, route kind, selected-capture evidence, diagnostics binary identity가 legacy `local-renderer-sidecar`/`dedicated-renderer` 경로와 구분되어야 하며, legacy route에 `implementationTrack=actual-primary-lane`만 붙인 결과는 actual lane success로 인정되지 않는다. [Source: _bmad-output/planning-artifacts/architecture.md] [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
2. actual primary lane과 darktable-compatible reference를 함께 운영할 때 booth가 same-capture preview close를 계산하면, darktable-compatible path는 parity/fallback/final reference로만 남아야 한다. 또한 latency-critical hot path는 darktable preview invocation completion이나 legacy dedicated-renderer completion을 직접 기다리면 안 된다. [Source: docs/contracts/render-worker.md] [Source: docs/contracts/local-dedicated-renderer.md]
3. actual primary lane이 실패하거나 health를 잃을 때 booth가 fallback을 수행하면, false-ready, false-complete, wrong-capture, cross-session leakage 없이 fail-closed 되어야 한다. 이미 기록된 session/capture evidence는 later rollout change나 wording change로 actual lane success로 재해석되면 안 된다. [Source: docs/contracts/session-manifest.md] [Source: docs/release-baseline.md]
4. actual lane success를 판정하는 automated gate와 selected-capture evidence는 legacy `laneOwner=dedicated-renderer`, legacy route kind, prototype-only result를 actual-lane proof로 받아들이면 안 된다. Story 1.28은 이 부정 조건을 테스트로 고정해야 하며, 이 조건이 남아 있으면 완료로 닫히면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.28: actual primary lane close owner 구현과 prototype route 분리] [Source: docs/runbooks/preview-promotion-evidence-package.md]

## Tasks / Subtasks

- [x] actual primary lane을 legacy sidecar route와 다른 distinct runtime close owner로 도입한다. (AC: 1, 2)
  - [x] `src-tauri/src/render/`, `src-tauri/src/render/mod.rs`, `src-tauri/src/capture/ingest_pipeline.rs` 또는 동등 경로에서 actual lane 전용 route kind 또는 owner identity를 정의한다.
  - [x] actual lane success artifact, binary/provenance, selected-capture diagnostics가 legacy `dedicated-renderer` owner와 다른 값으로 남도록 정리한다.
  - [x] legacy `local-renderer-sidecar` 경로는 prototype/comparison/fallback baseline으로만 남기고 current primary close owner로 읽히지 않게 한다.

- [x] actual lane hot path를 darktable 및 legacy completion으로부터 절연한다. (AC: 2)
  - [x] `previewReady`와 full-screen visible close는 same-capture truthful artifact readiness 이후에만 닫히게 유지한다.
  - [x] darktable preview invocation completion, parity diff, final/export reference completion, legacy dedicated-renderer completion은 actual lane success의 직접 선행조건이 되지 않게 한다.
  - [x] health loss, timeout, invalid output, capture mismatch, preset drift 시 booth는 `Preview Waiting` 또는 approved fallback으로만 내려가고 false-ready/false-complete를 선언하지 않게 한다.

- [x] capture-time snapshot과 selected-capture evidence를 distinct actual lane 기준으로 잠근다. (AC: 1, 3, 4)
  - [x] `src-tauri/src/session/session_manifest.rs`, `src/shared-contracts/schemas/session-manifest.ts`, `src/shared-contracts/schemas/hardware-validation.ts` 또는 동등 경로에서 actual lane route snapshot이 later policy change로 재해석되지 않게 유지한다.
  - [x] evidence bundle과 operator diagnostics는 actual lane provenance를 남기되, legacy prototype evidence를 actual-lane success로 승격시키지 않게 한다.
  - [x] Story 1.29에서 vocabulary realignment를 수행하기 전까지도 actual lane proof와 prototype proof가 기계적으로 구분되도록 bounded discriminator를 남긴다.

- [x] 아키텍처 적합성 회귀 테스트를 추가한다. (AC: 1, 2, 3, 4)
  - [x] Rust test: distinct runtime owner, darktable wait-free hot path, capture-time snapshot continuity, fail-closed fallback을 검증한다.
  - [x] TypeScript/PowerShell test: legacy `laneOwner=dedicated-renderer` 또는 legacy route kind가 actual-lane success로 통과되지 않는지 검증한다.
  - [x] Story 1.28 완료 의미는 actual lane distinct runtime close owner 구현까지이며, canary `Go`, default 승격, rollback proof, final release close를 요구하지 않는다는 범위를 유지한다.

### Reopen Findings

- 이전 additive `implementationTrack` 분리는 route labeling과 review semantics 개선에는 도움이 됐지만, actual lane distinct runtime owner 구현으로 보기에는 부족했다.
- 현재 코드 경계에서 `laneOwner=dedicated-renderer`가 여전히 operator/gate success 해석에 남아 있으면 Story 1.28은 닫히면 안 된다.
- Story 1.29는 Story 1.28이 distinct owner 전환을 완료한 뒤에야 governance/operator wording realignment 단계로 안전하게 진행할 수 있다.

### Review Findings

- [x] [Review][Patch] Actual lane provenance is relabeled on top of the legacy dedicated-renderer runtime instead of introducing a distinct close owner boundary. [src-tauri/src/render/dedicated_renderer.rs:1003]
- [x] [Review][Patch] Branch rollout status and settings/governance surfaces still publish `local-renderer-sidecar` as the resolved route, so operators cannot distinguish the actual lane from the prototype route kind. [src/branch-config/components/PreviewRouteGovernancePanel.tsx:141]

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- release baseline과 sprint status는 Stories 1.23~1.27을 prototype/evidence/gate history로 고정하고, Stories 1.28~1.31을 actual-lane forward path로 재정렬했다. 따라서 Story 1.28은 "기존 prototype을 조금 더 다듬는 일"이 아니라 actual primary lane 자체를 제품 코드 위에 올리는 시작점이어야 한다. [Source: docs/release-baseline.md] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- preview architecture history는 dedicated renderer activation 자체는 성공했지만, repeated approved-hardware KPI를 닫는 final primary close architecture로는 부족했다고 정리한다. 즉 이제 필요한 것은 새 actual lane 구현이지 dedicated renderer 미세조정 반복이 아니다. [Source: docs/preview-architecture-history-and-agent-guide.md]
- architecture change proposal은 primary architecture를 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`로 다시 선언하고, existing dedicated renderer는 activation baseline으로 강등한다. Story 1.28은 이 문서 결정을 실제 구현 경계로 옮기는 owner다. [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]

### 스토리 목적과 범위

- 이번 스토리가 소유하는 것:
  - actual primary lane distinct runtime close owner 구현
  - prototype dedicated-renderer route와 actual lane의 기능적 분리
  - darktable-compatible reference 및 fail-closed fallback 유지
  - selected-capture evidence에 actual implementation provenance를 남길 최소 discriminator 정리
- 이번 스토리가 소유하지 않는 것:
  - actual-lane evidence/vocabulary 전체 realignment
  - actual-lane hardware canary `Go / No-Go`
  - actual-lane default promotion과 one-action rollback proof
  - final guarded cutover / release ledger close
  - repeated KPI failure 이후 reserve remote path 개시

### 구현 가드레일

- legacy `local-renderer-sidecar` 또는 `dedicated-renderer` 경로에 `implementationTrack`만 붙여 actual lane 구현 완료처럼 보이게 만들면 안 된다.
- actual lane success를 `laneOwner=dedicated-renderer` 또는 legacy route kind로 판정하는 branch gate, script, diagnostics가 남아 있으면 Story 1.28은 완료가 아니다.
- actual lane hot path가 darktable preview result, parity diff, final/export reference를 직접 기다리게 만들면 안 된다.
- selected-capture evidence 대신 whole-session 로그나 later policy state를 읽어 성공을 재구성하면 안 된다.
- Story 1.28 안에서 Story 1.29, 1.30, 1.31, 1.13의 책임을 함께 흡수하면 안 된다.

### 우선 검토 경로

- `src-tauri/src/render/dedicated_renderer.rs`
- `src-tauri/src/render/mod.rs`
- `src-tauri/src/capture/ingest_pipeline.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/branch_config/mod.rs`
- `sidecar/dedicated-renderer/main.rs`
- `src/shared-contracts/schemas/session-manifest.ts`
- `src/shared-contracts/schemas/hardware-validation.ts`
- `src/shared-contracts/schemas/operator-diagnostics.ts`
- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- `scripts/hardware/Test-PreviewPromotionCanary.ps1`
- `src-tauri/tests/branch_rollout.rs`
- `src-tauri/tests/operator_diagnostics.rs`
- `tests/hardware-evidence-scripts.test.ts`

### 테스트 요구사항

- actual lane success path가 same `sessionId/requestId/captureId/presetId/publishedVersion` 기준의 truthful artifact close를 만든다.
- actual lane success proof는 legacy `local-renderer-sidecar`/`dedicated-renderer` selected evidence와 구분된다.
- darktable reference path가 살아 있어도 actual lane hot path가 darktable completion을 직접 기다리지 않는다.
- legacy route kind, legacy `laneOwner`, prototype-only result로 actual lane success verdict가 닫히지 않는다.
- actual lane health loss, invalid output, timeout, capture mismatch 시 booth가 fail-closed fallback으로 내려간다.
- later policy change나 rollback이 이미 기록된 capture route meaning을 재해석하지 않는다.

### References

- [Source: _bmad-output/planning-artifacts/epics.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: docs/release-baseline.md]
- [Source: docs/preview-architecture-history-and-agent-guide.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: sidecar/dedicated-renderer/README.md]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: src-tauri/src/session/session_manifest.rs]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: tests/hardware-evidence-scripts.test.ts]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Correction Notes

- 2026-04-17 리뷰 결과를 반영해 Story 1.28을 reopen했다.
- 이전 문서의 additive `implementationTrack` 해석 여지를 제거하고, distinct runtime owner 전환이 완료 조건임을 명시했다.
- Story 1.29가 담당해야 할 vocabulary realignment와 Story 1.28이 반드시 끝내야 할 runtime owner 전환을 다시 분리했다.

### Debug Log

- 2026-04-17 17:40: `pnpm vitest run tests/hardware-evidence-scripts.test.ts src/shared-contracts/contracts.test.ts src/shared-contracts/branch-rollout.contracts.test.ts src/operator-console/screens/OperatorSummaryScreen.test.tsx src/settings/screens/SettingsScreen.test.tsx src/operator-console/services/operator-diagnostics-service.test.ts` 통과 (137 tests)
- 2026-04-17 17:48: `cargo test --test branch_rollout --manifest-path src-tauri/Cargo.toml` 통과 (21 tests)
- 2026-04-17 17:52: `cargo test --test operator_diagnostics --manifest-path src-tauri/Cargo.toml` 통과 (23 tests)
- 2026-04-17 17:53: `cargo test --test dedicated_renderer --manifest-path src-tauri/Cargo.toml` 통과 (15 tests)

### Completion Notes

- actual primary lane의 runtime close owner를 legacy `dedicated-renderer`와 분리해 `local-fullscreen-lane`으로 고정했다.
- actual track runtime snapshot은 distinct route kind를 남기고, selected-capture evidence와 promotion gate는 legacy owner를 actual success로 받아들이지 않도록 fail-closed로 정리했다.
- darktable/legacy completion은 reference 또는 fallback 역할로만 남기고, hot path close 판단과 default gate는 actual lane 증거만 읽도록 맞췄다.

## File List

- scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1
- scripts/hardware/Test-PreviewPromotionCanary.ps1
- src-tauri/src/branch_config/mod.rs
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/branch_rollout.rs
- src-tauri/tests/dedicated_renderer.rs
- tests/hardware-evidence-scripts.test.ts
- tests/fixtures/contracts/preview-promotion-evidence-record-v1.json
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/branch-rollout.contracts.test.ts
- src/settings/screens/SettingsScreen.test.tsx
- src/operator-console/screens/OperatorSummaryScreen.test.tsx

### Change Log

- 2026-04-17: Story 1.28을 `in-progress`로 되돌리고, actual lane distinct runtime close owner와 legacy owner rejection을 완료 조건으로 재정의했다.
- 2026-04-17: actual primary lane distinct close owner, runtime snapshot 분리, legacy owner gate rejection, 관련 Rust/TypeScript/PowerShell 회귀 검증을 마치고 스토리를 `review`로 이동했다.
- 2026-04-17: 코드 리뷰에서 blocker 없음으로 확인되어 Story 1.28을 `done`으로 닫고, sprint-status의 다음 권장 흐름을 Story 1.29로 동기화했다.
