# Story 1.29: actual primary lane evidence와 vocabulary realignment

Status: done

Dependency Note: Story 1.29는 Story 1.28이 actual primary lane의 distinct runtime close owner를 실제 제품 경계에 올린 뒤에 시작한다. Story 1.28이 legacy owner labeling 단계에 머무르면 Story 1.29를 완료로 닫으면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]

## Summary

Story 1.29는 actual primary lane이 실제 주 경로가 된 뒤, 운영자가 읽는 governance/operator surface와 shared contract, hardware evidence script, rollout gate를 actual-lane proof family 기준으로 다시 잠그는 작업이다. backward compatibility를 위해 legacy 필드를 남길 수는 있지만, 더 이상 `laneOwner=dedicated-renderer`나 legacy route kind가 actual-lane success의 release-relevant 증거로 읽히면 안 된다.

## Acceptance Criteria

1. preview route decision summary, operator diagnostics, branch rollout contract, hardware evidence bundle을 검토할 때 actual lane의 primary discriminator는 distinct runtime owner와 actual-lane proof family여야 한다. legacy `laneOwner`, legacy route kind, prototype wording은 comparison/prototype evidence로만 남아야 한다. [Source: docs/contracts/branch-rollout.md] [Source: docs/runbooks/preview-promotion-evidence-package.md]
2. branch rollout gate, canary/default promotion script, evidence bundle assembly는 actual-lane success를 판정할 때 legacy `laneOwner=dedicated-renderer` 또는 legacy route kind를 허용하면 안 된다. compatibility field를 남기더라도 success path는 actual-lane owner/route 기준으로 fail-closed 되어야 한다. [Source: src-tauri/src/branch_config/mod.rs] [Source: scripts/hardware/Test-PreviewPromotionCanary.ps1] [Source: scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1]
3. sprint status, release baseline, hardware-validation ledger, Story 1.13 reopen 조건을 함께 검토할 때 `prototype done`, `actual implementation done`, `canary/default gate`, `release-close owner`가 구분되어야 한다. Story 1.23~1.27 또는 legacy wording 정리만으로 actual primary lane 구현 완료처럼 읽히면 안 된다. [Source: docs/release-baseline.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
4. Story 1.29 완료 이후에만 Story 1.30 canary 재검증이 열려야 하며, governance/operator wording이 여전히 legacy owner success semantics를 내포하면 Story 1.30 추천으로 이동하면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.30: actual primary lane hardware canary 재검증]

## Tasks / Subtasks

- [x] governance/operator surface를 actual-lane proof family 기준으로 재정렬한다. (AC: 1)
  - [x] `src/branch-config/components/PreviewRouteGovernancePanel.tsx`, `src/operator-console/screens/OperatorSummaryScreen.tsx`, `src/shared-contracts/schemas/operator-diagnostics.ts` 또는 동등 경로에서 actual-lane proof와 comparison-only prototype proof를 구분해 노출한다.
  - [x] `Close Owner`, `evidence track`, `evidence usage` 라벨이 legacy wording을 actual-lane success로 오해시키지 않게 정리한다.

- [x] branch rollout과 hardware evidence gate의 success semantics를 legacy owner에서 분리한다. (AC: 2)
  - [x] `src-tauri/src/branch_config/mod.rs`에서 actual-lane success path가 legacy `laneOwner=dedicated-renderer`에 의존하지 않게 수정한다.
  - [x] `scripts/hardware/Test-PreviewPromotionCanary.ps1`, `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1` 또는 동등 경로에서 actual-lane success와 fallback 판정이 distinct actual owner/route를 기준으로 fail-closed 되게 만든다.
  - [x] compatibility field가 남더라도 comparison/audit 용도로만 해석되게 테스트로 잠근다.

- [x] 스프린트와 release 문서를 actual-lane forward path 기준으로 다시 잠근다. (AC: 3, 4)
  - [x] `sprint-status.yaml`, `docs/release-baseline.md`, `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` 또는 동등 경로에서 Story 1.28/1.29 완료 조건과 Story 1.30 시작 조건을 다시 맞춘다.
  - [x] Story 1.13 reopen 조건은 Story 1.31 rollback-backed canonical actual-lane `Go` 이후로 유지한다.

- [x] 회귀 검증을 추가한다. (AC: 1, 2, 3, 4)
  - [x] TypeScript test: governance/operator contracts가 actual-lane proof family를 primary로 읽는지 검증한다.
  - [x] Rust/PowerShell test: legacy owner나 legacy route kind가 actual-lane success로 통과되지 않는지 검증한다.
  - [x] Story 1.29는 wording sweep만이 아니라 gate semantics correction을 포함한다는 점을 테스트와 문서에 남긴다.

### Review Findings

- [x] [Review][Patch] Preview route status가 non-shadow route를 모두 `actual-primary-lane`으로 재라벨링함 [src-tauri/src/branch_config/mod.rs:541]
- [x] [Review][Patch] Rollback 응답이 이전 actual-lane `implementationTrack`를 그대로 상속함 [src-tauri/src/branch_config/mod.rs:458]
- [x] [Review][Patch] Canary gate가 `visibleOwner`와 close owner 일치를 검증하지 않음 [scripts/hardware/Test-PreviewPromotionCanary.ps1:581]
- [x] [Review][Patch] Comparison-only history가 `fallbackRatio`를 실제 fallback처럼 오염시킴 [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:240]
- [x] [Review][Patch] Shared contract가 지원하지 않는 `implementationTrack` 문자열을 허용함 [src/shared-contracts/schemas/branch-rollout.ts:269]
- [x] [Review][Patch] Operator diagnostics가 legacy route와 actual-lane track을 같은 세션에서 모순되게 노출함 [src/operator-console/screens/OperatorSummaryScreen.tsx:194]

## Reopen Notes

- 이전 Story 1.29 결과는 UI/contract wording 정리에는 진전이 있었지만, success gate가 여전히 legacy `laneOwner=dedicated-renderer`에 의존하는 상태를 남겼다.
- 따라서 Story 1.29는 `done`이 아니라, corrected Story 1.28 이후에 다시 수행해야 하는 backlog 스토리로 되돌린다.
- Story 1.30 canary는 Story 1.29가 이 semantic gap을 닫기 전에는 열리면 안 된다.

## Verification Targets

- `src/shared-contracts/branch-rollout.contracts.test.ts`
- `src/settings/screens/SettingsScreen.test.tsx`
- `src/operator-console/screens/OperatorSummaryScreen.test.tsx`
- `src-tauri/tests/branch_rollout.rs`
- `tests/hardware-evidence-scripts.test.ts`
- `pnpm lint`

## File Targets

- `src/shared-contracts/schemas/branch-rollout.ts`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/branch_config/mod.rs`
- `src-tauri/tests/branch_rollout.rs`
- `src/branch-config/components/PreviewRouteGovernancePanel.tsx`
- `src/shared-contracts/branch-rollout.contracts.test.ts`
- `src/settings/screens/SettingsScreen.test.tsx`
- `src/operator-console/screens/OperatorSummaryScreen.tsx`
- `docs/contracts/branch-rollout.md`
- `scripts/hardware/Test-PreviewPromotionCanary.ps1`
- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Implementation Plan

- actual-lane proof family를 UI/operator/contracts에서 `implementationTrack` 우선 해석으로 정렬한다.
- branch rollout gate와 hardware evidence script가 legacy owner 또는 untyped evidence를 actual-lane success로 승격하지 못하게 fail-closed로 잠근다.
- sprint/release/runbook 문서와 회귀 테스트를 함께 맞춰 Story 1.30이 열리는 조건을 명시적으로 고정한다.

### Debug Log

- 2026-04-17 21:52: `pnpm test:run src/shared-contracts/branch-rollout.contracts.test.ts src/settings/screens/SettingsScreen.test.tsx src/operator-console/screens/OperatorSummaryScreen.test.tsx tests/hardware-evidence-scripts.test.ts` 통과 (60 tests)
- 2026-04-17 21:52: `cargo test --test branch_rollout --manifest-path src-tauri/Cargo.toml` 통과 (21 tests)
- 2026-04-17 21:53: `pnpm lint` 통과
- 2026-04-17 21:55: `pnpm test:run src/governance/hardware-validation-governance.test.ts` 통과 (10 tests)
- 2026-04-17 21:56: `pnpm test:run` 통과 (387 tests)
- 2026-04-17 22:20: `cargo test --test operator_diagnostics --manifest-path src-tauri/Cargo.toml` 통과 (23 tests)

### Completion Notes

- 운영 화면과 governance surface가 actual-lane proof family를 먼저 읽고, prototype/legacy evidence는 comparison-only로 남도록 정렬했다.
- branch rollout 및 hardware evidence gate가 legacy `laneOwner=dedicated-renderer` 또는 untyped/prototype evidence를 actual-lane success로 허용하지 않도록 fail-closed로 잠갔다.
- release baseline, hardware validation ledger, preview promotion runbook, sprint tracking을 actual-lane forward path 기준으로 다시 맞춰 Story 1.30은 Story 1.29 review 이후에만 열리게 정리했다.
- 코드 리뷰에서 나온 patch 6건을 모두 반영해 operator diagnostics, rollback summary, canary gate, shared contract drift를 닫고 Story 1.29를 완료로 닫았다.

## File List

- `_bmad-output/implementation-artifacts/1-29-actual-primary-lane-evidence와-vocabulary-realignment.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/contracts/branch-rollout.md`
- `docs/release-baseline.md`
- `docs/runbooks/preview-promotion-evidence-package.md`
- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- `scripts/hardware/Test-PreviewPromotionCanary.ps1`
- `src/branch-config/components/PreviewRouteGovernancePanel.tsx`
- `src/operator-console/screens/OperatorSummaryScreen.tsx`
- `src/operator-console/screens/OperatorSummaryScreen.test.tsx`
- `src/settings/screens/SettingsScreen.test.tsx`
- `src/shared-contracts/branch-rollout.contracts.test.ts`
- `src/shared-contracts/schemas/branch-rollout.ts`
- `src/shared-contracts/schemas/operator-diagnostics.ts`
- `src-tauri/src/branch_config/mod.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/tests/branch_rollout.rs`
- `tests/hardware-evidence-scripts.test.ts`

## Change Log

- 2026-04-17: Story 1.29를 `backlog`로 되돌리고, wording realignment뿐 아니라 legacy owner 기반 gate semantics 제거를 완료 조건으로 재정의했다.
- 2026-04-17: actual-lane proof family 우선 해석, legacy owner fail-closed gate, sprint/release/runbook realignment, 회귀 검증을 마치고 Story 1.29를 `review`로 이동했다.
- 2026-04-17: 코드 리뷰 patch 6건을 반영하고 operator diagnostics 회귀 검증까지 마쳐 Story 1.29를 `done`으로 닫았다.
