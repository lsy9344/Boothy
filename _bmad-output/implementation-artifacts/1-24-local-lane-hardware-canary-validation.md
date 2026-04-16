# Story 1.24: local lane hardware canary validation

Status: done

Ordering Note: Story 1.24는 Story 1.23이 local full-screen lane prototype과 truthful artifact generation을 잠근 다음에 와야 한다. 이 스토리는 approved hardware canary에서 그 결과를 `Go / No-Go` health gate로 읽는 owner이며, Story 1.25가 default/rollback gate를 닫고, Story 1.13이 마지막 guarded close를 맡는다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
새 local full-screen lane를 hardware canary로 검증하고 싶다,
그래서 prototype 성공이 실제 운영 조건에서도 반복 가능한지 판단할 수 있다.

## Acceptance Criteria

1. approved Windows booth hardware canary scope가 있을 때 local lane canary를 실행하면, `sameCaptureFullScreenVisibleMs`, `replacementMs`, `fallback ratio`, `wrong-capture`, `fidelity drift`를 같은 evidence bundle에서 읽을 수 있어야 한다. 또한 canary session은 active customer session safety를 훼손하면 안 된다.
2. canary 결과를 승격 판단에 사용할 때 health gate를 검토하면, KPI miss, fallback 상시화, wrong-capture, fidelity drift 중 하나라도 남아 있으면 `No-Go`가 유지되어야 한다. 또한 one-action rollback proof가 없으면 다음 단계로 진행하면 안 된다.

## Tasks / Subtasks

- [x] evidence bundle에서 hardware canary health gate를 산출한다. (AC: 1, 2)
  - [x] `scripts/hardware/Test-PreviewPromotionCanary.ps1` 또는 동등 경로에서 selected-capture bundle을 읽어 KPI, fallback stability, wrong-capture, fidelity drift, rollback readiness, active-session safety를 한 번에 판정하는 typed assessment를 만든다.
  - [x] assessment는 missing field, stale evidence, capture correlation drift, non-canary route, rollback proof omission에 대해 fail-closed `No-Go`를 반환해야 한다.
  - [x] assessment output은 operator-safe JSON artifact로 저장하거나 JSON stdout으로 방출할 수 있어야 하며, 이후 Story 1.25가 읽을 수 있는 blocker 목록을 남긴다.

- [x] canary validation contract와 runbook을 잠근다. (AC: 1, 2)
  - [x] `src/shared-contracts/schemas/hardware-validation.ts`, `src/shared-contracts/contracts.test.ts` 또는 동등 경로에 canary assessment schema와 parsing coverage를 추가한다.
  - [x] `docs/runbooks/preview-promotion-evidence-package.md` 또는 동등 문서에서 bundle assemble과 canary gate review 순서, `No-Go` 조건, Story 1.25/1.13 ownership boundary를 명시한다.
  - [x] 이번 스토리 안에서 default promotion authority나 final release close ownership을 흡수하지 않는다.

- [x] hardware canary regression을 자동 검증으로 잠근다. (AC: 1, 2)
  - [x] `tests/hardware-evidence-scripts.test.ts` 또는 동등 테스트에서 `Go`, KPI miss, fallback-heavy, wrong-capture drift, fidelity drift, rollback proof 누락, active-session safety 위반 케이스를 추가한다.
  - [x] 새 검증은 selected-capture evidence chain이 유지될 때만 `Go`가 나오도록 고정하고, 하나라도 실패하면 `No-Go` blocker가 남도록 확인한다.

### Review Findings

- [x] [Review][Patch] Stale canary evidence is never rejected fail-closed [scripts/hardware/Test-PreviewPromotionCanary.ps1:113]
- [x] [Review][Patch] Wrong-capture validation trusts a timing log path outside the assembled bundle [scripts/hardware/Test-PreviewPromotionCanary.ps1:135]
- [x] [Review][Patch] Rollback readiness accepts any existing file instead of bundle-local proof [scripts/hardware/Test-PreviewPromotionCanary.ps1:204]
- [x] [Review][Patch] Bundle-root safety check uses a prefix match that can be bypassed by sibling paths [scripts/hardware/Test-PreviewPromotionCanary.ps1:224]
- [x] [Review][Patch] Malformed bundle input can crash assessment generation instead of returning typed `No-Go` output [scripts/hardware/Test-PreviewPromotionCanary.ps1:14]
- [x] [Review][Patch] Selected-capture timing validation ignores owner/detail drift inside the preserved event chain [scripts/hardware/Test-PreviewPromotionCanary.ps1:98]

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- sprint plan은 새 preview architecture forward path를 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25`로 고정했고, Story 1.24를 local lane의 첫 explicit hardware validation gate로 정의했다. 이 단계를 건너뛰면 prototype success가 실장비 기준 `Go / No-Go` 판단으로 연결되지 않는다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- epics는 Story 1.24가 KPI, fallback, wrong-capture, fidelity drift를 같은 evidence bundle에서 읽고, blocker가 남아 있으면 `No-Go`를 유지해야 한다고 고정한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.24: local lane hardware canary validation]

### 스토리 목적과 범위

- 이번 스토리는 approved hardware canary evidence를 `Go / No-Go` health gate로 읽는 owner다.
- 이번 스토리는 아래를 소유한다.
  - selected-capture evidence bundle 기반 canary assessment
  - KPI/fallback/wrong-capture/fidelity/rollback blocker를 한 번에 읽는 fail-closed verdict
  - operator-safe canary artifact와 regression coverage
- 아래 작업은 이번 스토리 범위가 아니다.
  - default route promotion 실행
  - one-action rollback mutation 자체 구현
  - final guarded cutover / release close

### 스토리 기반 요구사항

- PRD는 primary release sign-off를 `same-capture preset-applied full-screen visible <= 2500ms`로 고정하고, latency, same-capture correctness, preset fidelity, fallback stability evidence를 함께 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- Story 1.22는 selected-capture evidence chain을 reset owner로 고정했고, Story 1.23은 local lane prototype owner를 잠갔다. Story 1.24는 그 bundle을 읽어 canary verdict를 내리는 follow-up이다. [Source: _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md] [Source: _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md]
- preview promotion evidence package runbook은 fallback-heavy run, parity drift, missing route snapshot, missing rollback evidence를 `No-Go`로 유지해야 한다고 명시한다. [Source: docs/runbooks/preview-promotion-evidence-package.md]

### 현재 워크스페이스 상태

- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`는 selected-capture bundle을 assemble하고 `fallbackRatio`, parity 결과, booth/operator visuals, rollback evidence를 operator-safe package로 복사한다.
- `src/shared-contracts/schemas/hardware-validation.ts`는 evidence record/bundle schema를 이미 잠그고 있지만, canary `Go / No-Go` assessment schema는 아직 없다.
- `src-tauri/src/branch_config/mod.rs`는 repeated canary success-path evidence 수를 세어 Story 1.25 default 승격을 일부 준비하지만, Story 1.24의 health gate verdict 자체는 아직 별도 artifact로 고정하지 않았다.
- `tests/hardware-evidence-scripts.test.ts`는 bundle assembly의 correlation/fallback/parity/rollback bundle 요구사항을 이미 검증하고 있어, 이번 스토리는 그 위에 canary verdict regression을 추가하는 편이 안전하다.
- 별도 `project-context.md`는 발견되지 않았다.

### 이전 스토리 인텔리전스

- Story 1.23은 local lane prototype owner를 추가하면서 selected-capture bundle에 `visibleOwner`, `visibleOwnerTransitionAtMs`, `truthfulArtifactReadyAtMs`, `sameCaptureFullScreenVisibleMs`를 남기도록 잠갔다. Story 1.24는 이 signals를 다시 발명하지 말고 그대로 verdict input으로 재사용해야 한다.
- Story 1.22는 whole-session timing log reuse와 live snapshot recopy를 금지했다. Story 1.24도 wrong-capture와 active-session safety 판정에서 이 rule을 깨면 안 된다.

### 구현 가드레일

- canary verdict는 speed 하나만으로 `Go`를 선언하면 안 된다. `sameCaptureFullScreenVisibleMs`, fallback stability, wrong-capture, fidelity drift, rollback readiness를 함께 봐야 한다.
- assessment가 `Go`를 내더라도 Story 1.25가 default/rollback authority를 소유한다는 경계를 깨면 안 된다.
- missing field나 malformed bundle은 pass가 아니라 fail-closed `No-Go`로 해석해야 한다.
- active session safety는 capture-time route/catalog snapshot과 selected-capture timing chain을 잃지 않는 방식으로만 판정해야 한다.

### 프로젝트 구조 요구사항

- 우선 수정/검토 후보 경로:
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
  - `scripts/hardware/Test-PreviewPromotionCanary.ps1`
  - `tests/hardware-evidence-scripts.test.ts`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `docs/runbooks/preview-promotion-evidence-package.md`
- scope를 넘기지 않도록 주의할 경로:
  - `src-tauri/src/branch_config/mod.rs`
  - `src/branch-config/components/PreviewRouteGovernancePanel.tsx`
  - `src/settings/screens/SettingsScreen.tsx`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

### 테스트 요구사항

- 최소 필수 자동 검증:
  - valid selected-capture canary bundle이 `Go` assessment를 만든다.
  - KPI miss면 `No-Go`가 된다.
  - fallback-heavy 또는 selected capture fallback이면 `No-Go`가 된다.
  - wrong-capture correlation drift가 보이면 `No-Go`가 된다.
  - parity/fidelity drift가 fail 또는 conditional이면 `No-Go`가 된다.
  - rollback proof가 없으면 `No-Go`와 next-stage blocked가 남는다.
  - non-canary route 또는 active-session safety rule 위반이면 `No-Go`가 된다.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.24: local lane hardware canary validation]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- [Source: _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md]
- [Source: _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: docs/release-baseline.md]
- [Source: scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: tests/hardware-evidence-scripts.test.ts]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- `Get-Content -Raw _bmad/bmm/config.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `Select-String -Path _bmad-output/planning-artifacts/epics.md -Pattern "### Story 1\\.24" -Context 0,120`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md`
- `Get-Content -Raw docs/runbooks/preview-promotion-evidence-package.md`
- `Get-Content -Raw docs/release-baseline.md`
- `Get-Content -Raw scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- `Get-Content -Raw src/shared-contracts/schemas/hardware-validation.ts`
- `Get-Content -Raw src-tauri/src/branch_config/mod.rs`
- `Get-Content -Raw tests/hardware-evidence-scripts.test.ts`
- `pnpm test:run tests/hardware-evidence-scripts.test.ts src/shared-contracts/contracts.test.ts`
- `pnpm test:run src/governance/hardware-validation-governance.test.ts src/shared-contracts/branch-rollout.contracts.test.ts tests/hardware-evidence-scripts.test.ts src/shared-contracts/contracts.test.ts`
- `pnpm lint`

### Completion Notes

- selected-capture evidence bundle을 읽어 `Go / No-Go`, blocker 목록, next-stage allowed 여부를 반환하는 `preview-promotion-canary-assessment/v1` artifact를 추가했다.
- canary assessment는 KPI miss, fallback-heavy, wrong-capture drift, fidelity drift, rollback proof omission, non-canary safety 위반에 대해 fail-closed `No-Go`를 반환하도록 잠갔다.
- runbook에 bundle assemble 다음 canary assessment 실행 순서를 추가해 Story 1.25/1.13 경계와 `No-Go` 유지 규칙을 문서로 맞췄다.
- hardware evidence script regression, shared contract parsing, governance 회귀, lint를 통과했다.

### File List

- _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/runbooks/preview-promotion-evidence-package.md
- scripts/hardware/Test-PreviewPromotionCanary.ps1
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/hardware-validation.ts
- tests/hardware-evidence-scripts.test.ts

### Change Log

- 2026-04-15 16:30:00 +09:00 - Story 1.24 context created and work started for local lane hardware canary validation.
- 2026-04-15 16:31:30 +09:00 - Added fail-closed canary assessment artifact, contract coverage, runbook flow, and hardware regression tests; story moved to review.
