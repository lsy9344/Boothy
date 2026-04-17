# Story 1.30: actual primary lane hardware canary 재검증

Status: in-progress

Dependency Note: Story 1.30은 Story 1.29가 actual-lane evidence semantics와 operator/governance vocabulary를 actual primary lane 기준으로 잠근 뒤에만 시작한다. 이 스토리는 prototype-track canary history를 재사용하되, actual-lane `Go / No-Go`를 legacy prototype evidence와 분리해 다시 닫아야 한다. Story 1.30은 single pass가 아니라 repeated approved-hardware actual-lane success-path evidence와 canonical ledger verdict를 만드는 단계이며, 이 증거가 없으면 Story 1.31 default/rollback gate로 진행하면 안 되고 Story 1.13 release-close owner도 계속 blocked 상태를 유지해야 한다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] [Source: docs/release-baseline.md]

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
actual primary lane 기준으로 hardware canary를 다시 수행하고 싶다,
그래서 새 주 경로가 prototype evidence가 아니라 실제 구현 기준으로 KPI와 correctness를 입증하는지 확인하고 싶다.

## Acceptance Criteria

1. 승인된 Windows booth hardware canary scope가 있을 때 actual primary lane canary를 실행하면, `sameCaptureFullScreenVisibleMs`, `wrong-capture`, `fidelity drift`, fallback stability, follow-up capture health, `visibleOwner`, `visibleOwnerTransitionAtMs`, route stage, blocker list를 같은 selected-capture evidence bundle에서 읽을 수 있어야 한다. 이때 bundle은 capture-time route/catalog snapshot, `implementationTrack=actual-primary-lane`, rollback proof, `Automated Pass`, `Hardware Pass`, `Go / No-Go`, owner, evidence path를 함께 보존해야 하며, 결과는 prototype-track supporting evidence와 분리된 actual-lane canary verdict로 canonical ledger row에 기록되어야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.30: actual primary lane hardware canary 재검증] [Source: docs/runbooks/preview-promotion-evidence-package.md] [Source: docs/release-baseline.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
2. canary 결과를 다음 단계 입력으로 사용할 때 health gate를 검토하면, KPI miss, fallback-heavy, wrong-capture, fidelity drift, evidence gap, non-canary route, follow-up capture health failure, missing/stale/unapproved hardware scope 중 하나라도 남아 있으면 `No-Go`가 유지되어야 한다. 또한 repeated approved-hardware actual-lane success-path evidence와 accepted ledgered canary verdict 없이는 Story 1.31로 진행하면 안 되며, prototype/untyped evidence나 comparison-only history가 actual-lane success를 대체하면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.30: actual primary lane hardware canary 재검증] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] [Source: docs/runbooks/preview-promotion-evidence-package.md] [Source: docs/release-baseline.md]

## Tasks / Subtasks

- [ ] actual-lane canary evidence bundle과 typed verdict를 재정렬한다. (AC: 1, 2)
  - [ ] `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`, `scripts/hardware/Test-PreviewPromotionCanary.ps1`, `src/shared-contracts/schemas/hardware-validation.ts` 또는 동등 경로에서 selected-capture bundle이 `implementationTrack=actual-primary-lane`, `visibleOwner`, `visibleOwnerTransitionAtMs`, capture-time policy/catalog snapshot, route stage, `sameCaptureFullScreenVisibleMs`, parity/fidelity result, `fallbackRatio`, rollback evidence, follow-up capture health, explicit blocker fields를 함께 읽도록 맞춘다.
  - [ ] prototype-track, legacy owner, untyped evidence, stale route snapshot, missing rollback proof, incomplete selected-capture chain, non-canary route, missing/stale/unapproved hardware scope는 comparison/audit 입력으로만 남기고 actual-lane canary verdict에서는 fail-closed `No-Go`로 처리한다.
  - [ ] canary verdict artifact는 operator-safe JSON/ledger 입력으로 남고, Story 1.31이 그대로 읽을 수 있는 blocker 목록, repeated success-path evidence count, next-stage eligibility를 제공해야 한다.

- [ ] actual-lane canary governance와 문서 경계를 잠근다. (AC: 1, 2)
  - [ ] `docs/runbooks/preview-promotion-evidence-package.md`, `docs/runbooks/booth-hardware-validation-checklist.md`, `docs/release-baseline.md`, `release-baseline.md`, `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` 또는 동등 문서에서 Story 1.30이 prototype-track canary와 다른 actual-lane gate임을 명시한다.
  - [ ] ledger/runbook/release copy는 actual-lane canary verdict를 Story 1.30 canonical row/section으로 기록하고, `Automated Pass`, `Hardware Pass`, `Go / No-Go`, blocker, owner, evidence path, latency, parity, fallback ratio, route policy state, rollback evidence를 함께 남기며, repeated success-path evidence 없이는 Story 1.31 및 Story 1.13이 열리지 않는다는 순서를 유지한다.
  - [ ] 이번 스토리 안에서 default promotion authority, rollback mutation 실행, final release close ownership을 흡수하지 않는다.

- [ ] actual-lane canary 회귀 검증과 rerun readiness를 자동화한다. (AC: 1, 2)
  - [ ] `tests/hardware-evidence-scripts.test.ts`, `src/shared-contracts/contracts.test.ts`, `src-tauri/tests/branch_rollout.rs`, `src/governance/hardware-validation-governance.test.ts` 또는 동등 검증에 actual-lane success, KPI miss, fallback-heavy, wrong-capture, fidelity drift, evidence gap, missing rollback proof, follow-up capture unhealthy, route/policy mismatch 케이스를 추가한다.
  - [ ] Story 1.24 prototype canary tests를 재사용하되, Story 1.30은 actual-lane owner/proof family만 통과시켜야 한다는 차이를 테스트로 잠그고, `src-tauri/src/branch_config/mod.rs` 및 `src-tauri/tests/branch_rollout.rs`의 stale `Story 1.24 typed canary Go verdict` default-gate reference를 actual-lane wording으로 교체한다.
  - [ ] dev completion 전에는 approved hardware rerun에 필요한 bundle assemble, canary evaluation, ledger row update 절차가 문서와 스크립트에서 재현 가능해야 한다.

### Review Findings

- [x] [Review][Patch] Story 1.30 omits required selected-capture gate fields and gate outputs [_bmad-output/implementation-artifacts/1-30-actual-primary-lane-hardware-canary-재검증.md:17]
- [x] [Review][Patch] Story 1.30 weakens the canary-to-default prerequisite from repeated success-path evidence to a single accepted verdict [_bmad-output/implementation-artifacts/1-30-actual-primary-lane-hardware-canary-재검증.md:5]
- [x] [Review][Patch] Story 1.30 steers work back into already-closed canary script hardening while leaving the stale prototype default-gate reference unowned [_bmad-output/implementation-artifacts/1-30-actual-primary-lane-hardware-canary-재검증.md:69]
- [x] [Review][Patch] Story 1.30 leaves the actual-lane canary ledger and mirrored governance-doc updates underspecified [_bmad-output/implementation-artifacts/1-30-actual-primary-lane-hardware-canary-재검증.md:28]

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- sprint status는 Story 1.29를 done으로 닫았고, 다음 권장 스토리를 Story 1.30 actual-lane hardware canary로 고정한다. 현재 release hold를 깨는 다음 결정점은 wording 정리가 아니라 actual-lane canary verdict다. [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- release baseline은 Stories 1.28~1.31만이 active actual-lane forward path라고 명시하고, Story 1.30 accepted canary와 Story 1.31 rollback-backed default proof가 나오기 전까지 Story 1.13을 reopen하지 않는다. [Source: docs/release-baseline.md]
- PRD의 sign-off KPI는 `same-capture preset-applied full-screen visible <= 2500ms`이며, release 판단은 latency만이 아니라 same-capture correctness, preset fidelity, fallback stability evidence를 함께 요구한다. Story 1.30은 이 product acceptance를 actual-lane 구현 기준으로 다시 검증하는 owner다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]

### 스토리 목적과 범위

- 이번 스토리는 actual primary lane canary verdict와 rerun readiness를 소유한다.
- 이번 스토리가 소유하는 것:
  - actual-lane proof family 기준의 canary bundle/assessment 정렬
  - KPI, wrong-capture, fidelity, fallback stability, follow-up capture health, rollback readiness를 같은 selected-capture 문맥으로 판독하는 fail-closed verdict
  - operator-safe ledger/runbook/release wording 정렬
  - actual-lane canary regression coverage
- 이번 스토리가 소유하지 않는 것:
  - actual-lane default promotion 실행
  - one-action rollback mutation 자체 구현
  - Story 1.13 final guarded cutover / release-close
  - repeated failure 이후 reserve track Story 1.26 개시

### 스토리 기반 요구사항

- Story 1.24는 prototype-track canary owner로서 `sameCaptureFullScreenVisibleMs`, `fallbackRatio`, wrong-capture, fidelity drift, rollback readiness, active-session safety를 한 bundle에서 읽는 기본 구조를 이미 정의했다. Story 1.30은 이를 재발명하지 말고 actual-lane semantics로 재사용해야 한다. [Source: _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md]
- Story 1.27은 local hot path와 follow-up capture health를 같은 bundle에서 읽는 corrective pattern을 남겼다. Story 1.30은 actual-lane canary verdict에서 이 follow-up capture seam을 계속 blocker 입력으로 읽어야 한다. [Source: _bmad-output/implementation-artifacts/1-27-local-hot-path-darktable-절연과-2500ms-kpi-재검증.md]
- Story 1.29는 actual-lane proof family, vocabulary, rollback track inheritance, visible-owner mismatch, fallback-ratio pollution, schema drift를 닫았다. Story 1.30은 이 corrected semantics를 깨지 않는지 검증해야 하며, legacy wording으로 되돌리면 안 된다. [Source: _bmad-output/implementation-artifacts/1-29-actual-primary-lane-evidence와-vocabulary-realignment.md]
- evidence package runbook은 selected-capture chain만 복사하고, `implementationTrack`가 없거나 `prototype-track`이면 canary/default promotion에서 fail-closed 해야 한다고 고정한다. [Source: docs/runbooks/preview-promotion-evidence-package.md]

### 현재 워크스페이스 상태

- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`는 selected-capture bundle assemble, route policy snapshot, catalog snapshot, rollback evidence copy, `sameCaptureFullScreenVisibleMs`/`fallbackRatio` 계산을 이미 담당한다. Story 1.30은 여기에 actual-lane canary rerun에서 필요한 blocker 입력이 빠지지 않도록 맞추는 것이 안전하다.
- `scripts/hardware/Test-PreviewPromotionCanary.ps1`는 KPI, fallback stability, wrong-capture, fidelity drift, rollback readiness, active-session safety를 typed checks로 이미 판정한다. Story 1.30은 기존 actual-lane owner/proof-family guard를 다시 발명하기보다, repeated approved-hardware success-path evidence와 Story 1.31 진입 경계를 canonical ledger semantics까지 포함해 정렬하는 follow-up이다.
- `src-tauri/src/branch_config/mod.rs`와 `src-tauri/tests/branch_rollout.rs`는 canary/default promotion gate의 host-owned rule을 소유한다. Story 1.30은 accepted repeated canary evidence 없이 default claim이 열리지 않도록 이 경계를 함께 점검해야 하며, 남아 있는 prototype-era gate wording도 actual-lane 기준으로 정리해야 한다.
- `src/shared-contracts/schemas/hardware-validation.ts`와 `src/shared-contracts/contracts.test.ts`는 bundle 및 canary assessment schema를 이미 가진다. ad-hoc JSON parsing을 추가하기보다 이 schema family를 확장하는 편이 안전하다.
- 별도 `project-context.md`는 발견되지 않았다.

### 이전 스토리 인텔리전스

- Story 1.29 완료 메모 기준으로 governance/operator/contracts는 actual-lane proof family를 먼저 읽고, prototype/legacy evidence는 comparison-only로 남겨야 한다. Story 1.30은 legacy owner success semantics를 다시 허용하면 안 된다.
- Story 1.29 review patch는 `visibleOwner` mismatch, rollback track inheritance, fallback-ratio pollution, shared contract drift를 실제 blocker로 드러냈다. Story 1.30 canary는 이 값들을 "있으면 좋음"이 아니라 fail-closed gate 입력으로 계속 다뤄야 한다.
- Story 1.24와 Story 1.27 모두 whole-session 로그 재해석 대신 selected-capture evidence chain을 canonical input으로 유지했다. Story 1.30에서도 whole-session timing log나 later policy state를 reread해 success를 재구성하면 안 된다.

### Git 인텔리전스

- 최근 관련 커밋은 `feat: checkpoint dedicated renderer rollout work`, `feat: checkpoint preset applied rendering and diagnostics`, `feat: add local renderer contracts and release baseline`로 이어지며, 공통적으로 evidence scripts, branch rollout host logic, dedicated renderer diagnostics, release/runbook 문서, shared contracts, 회귀 테스트를 함께 수정했다. Story 1.30도 동일한 경계 묶음을 따라가야 하며, 별도 우회 스크립트나 duplicate contract를 만들면 위험하다. [Source: git log] [Source: git show --stat 16e637e] [Source: git show --stat 40015d1] [Source: git show --stat 4611eb5]

### 구현 가드레일

- prototype-track canary `Go`나 historical supporting evidence를 actual-lane canary success로 승격하면 안 된다.
- `sameCaptureFullScreenVisibleMs` 하나만 맞았다고 `Go`를 내면 안 된다. wrong-capture, fidelity drift, fallback stability, route policy snapshot, rollback evidence, follow-up capture health를 함께 봐야 한다.
- actual-lane canary에서 `implementationTrack`가 없거나 `prototype-track`이면 comparison-only로 남길 수는 있어도 success verdict로 쓰면 안 된다.
- Story 1.30 결과가 좋아 보여도 Story 1.31 default promotion authority나 Story 1.13 release close ownership을 자동 흡수하면 안 된다.
- actual-lane canary artifact는 operator-safe여야 하지만, truth를 흐리기 위해 selected-capture evidence를 축약하거나 whole-session log로 바꿔서는 안 된다.

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보 경로:
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
  - `scripts/hardware/Test-PreviewPromotionCanary.ps1`
  - `tests/hardware-evidence-scripts.test.ts`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src-tauri/src/branch_config/mod.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `src/governance/hardware-validation-governance.test.ts`
  - `docs/runbooks/booth-hardware-validation-checklist.md`
  - `docs/runbooks/preview-promotion-evidence-package.md`
  - `docs/release-baseline.md`
  - `release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- scope를 넘기지 않도록 주의할 경로:
  - `_bmad-output/implementation-artifacts/1-31-actual-primary-lane-default-decision과-rollback-gate.md`
  - `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
  - `_bmad-output/implementation-artifacts/1-26-local-lane-실패-시에만-remote-reserve-poc-개시.md`

### 최신 기술 확인 메모

- 현재 로컬 기준 stack은 `react 19.2.4`, `@tauri-apps/api 2.10.1`, `zod 4.3.6`, Rust `tauri 2.10.3`, Rust toolchain `1.77.2`다. Story 1.30은 새 외부 기술 도입보다 existing canary script, shared schema, host rollout gate를 actual-lane semantics로 재검증하는 것이 핵심이므로, dependency upgrade보다 현재 pinned contract 경계를 유지하는 편이 안전하다. [Source: package.json] [Source: src-tauri/Cargo.toml]

### 테스트 요구사항

- 최소 필수 자동 검증:
  - actual-lane selected-capture bundle이 required evidence를 모두 갖추면 canary assessment가 typed `Go` 후보를 반환한다.
  - KPI miss면 `No-Go`가 된다.
  - fallback-heavy 또는 selected capture fallback pollution이 있으면 `No-Go`가 된다.
  - wrong-capture drift 또는 selected-capture chain mismatch가 있으면 `No-Go`가 된다.
  - parity/fidelity drift가 `fail` 또는 insufficient oracle proof면 `No-Go`가 된다.
  - rollback proof가 없으면 `No-Go`와 Story 1.31 blocked 상태가 남는다.
  - `implementationTrack` missing/prototype, route policy mismatch, stale capture-time snapshot이면 `No-Go`가 된다.
  - follow-up capture health가 unhealthy면 `No-Go`가 된다.
  - `visibleOwnerTransitionAtMs`가 없으면 `No-Go`가 된다.
  - missing/stale/unapproved hardware scope면 `No-Go`가 된다.
- 권장 추가 검증:
  - branch rollout gate가 repeated approved-hardware actual-lane canary evidence 없이 `default` route claim을 허용하지 않는지 확인한다.
  - ledger/runbook/release wording이 prototype/evidence history와 actual-lane canary verdict를 섞지 않는지 governance test로 잠근다.

### Hardware Retest Expectation

- 이 스토리는 코드와 문서만으로 닫히지 않는다. dev 완료 후 approved booth hardware에서 actual-lane canary rerun을 수행하고, assembled bundle path와 `Go / No-Go` verdict를 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에 기록해야 한다.
- retest package는 최소 `session.json`, selected-capture `timing-events.log`, `preview-promotion-evidence.jsonl`, captured route policy/catalog snapshot, published `bundle.json`, rollback evidence, booth/operator visual proof를 포함해야 한다. [Source: docs/runbooks/preview-promotion-evidence-package.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- canonical ledger update는 Story 1.30용 row/section에 `Automated Pass`, `Hardware Pass`, `Go / No-Go`, blocker, owner, evidence path, latency, parity, fallback ratio, route policy state, rollback evidence, repeated success-path evidence count를 함께 남겨야 한다. [Source: docs/release-baseline.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]

### References

- [Source: _bmad-output/planning-artifacts/epics.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md]
- [Source: _bmad-output/implementation-artifacts/1-27-local-hot-path-darktable-절연과-2500ms-kpi-재검증.md]
- [Source: _bmad-output/implementation-artifacts/1-29-actual-primary-lane-evidence와-vocabulary-realignment.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: docs/release-baseline.md]
- [Source: scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1]
- [Source: scripts/hardware/Test-PreviewPromotionCanary.ps1]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/contracts.test.ts]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: src-tauri/tests/branch_rollout.rs]
- [Source: src/governance/hardware-validation-governance.test.ts]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- `Get-Content -Raw _bmad/bmm/config.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-29-actual-primary-lane-evidence와-vocabulary-realignment.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-27-local-hot-path-darktable-절연과-2500ms-kpi-재검증.md`
- `Get-Content -Raw docs/runbooks/preview-promotion-evidence-package.md`
- `Get-Content -Raw docs/release-baseline.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `Get-Content -Raw package.json`
- `Get-Content -Raw src-tauri/Cargo.toml`
- `git log -n 8 --oneline`
- `git show --stat 16e637e`
- `git show --stat 40015d1`
- `git show --stat 4611eb5`
- `cargo test --test branch_rollout preview_route_default_promotion_rejects_without_typed_go_canary_assessment -- --exact`
- `pnpm test:run tests/hardware-evidence-scripts.test.ts src/shared-contracts/contracts.test.ts src/governance/hardware-validation-governance.test.ts`
- `cargo test --test branch_rollout`
- `pnpm lint`
- `pnpm test:run`
- `cargo test`

### Completion Notes List

- [ ] Dev implementation completed
- [ ] Automated verification completed
- [ ] Approved hardware canary rerun completed
- [ ] Hardware ledger updated with actual-lane canary verdict
- Targeted Story 1.30 verification passed after replacing the last stale `Story 1.24` default-gate rejection copy with actual primary lane wording.
- Full JS regression (`pnpm test:run`) passed.
- Full Rust regression is currently blocked by `render::tests::preview_invocation_avoids_pending_canonical_preview_assets_during_truthful_close`, which failed during `cargo test` in the existing render lane worktree state and is outside this story's mapped task list.

### File List

- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- `scripts/hardware/Test-PreviewPromotionCanary.ps1`
- `tests/hardware-evidence-scripts.test.ts`
- `src/shared-contracts/schemas/hardware-validation.ts`
- `src/shared-contracts/contracts.test.ts`
- `src-tauri/src/branch_config/mod.rs`
- `src-tauri/tests/branch_rollout.rs`
- `src/governance/hardware-validation-governance.test.ts`
- `docs/runbooks/preview-promotion-evidence-package.md`
- `docs/release-baseline.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
