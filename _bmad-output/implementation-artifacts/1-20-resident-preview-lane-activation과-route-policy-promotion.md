# Story 1.20: resident preview lane activation과 route policy promotion

Status: backlog

Activation Ownership Note: Story 1.18이 resident GPU-first 후보와 warm-state evidence를 만들었고, Story 1.19가 promotion evidence gate를 고정했다. 이번 스토리는 그 기반을 실제 booth 운영 경계로 승격하는 activation owner다. canonical release close owner는 계속 Story 1.13이며, 이번 스토리만으로 hardware `Go`를 주장하면 안 된다.

### Activation Gate Reference

- Pre-activation baseline:
  - Story 1.18 `done`
  - Story 1.19 `done`
  - canonical hardware ledger에서 Story 1.13은 여전히 `No-Go`
- Current blocker baseline:
  - observed route policy는 `defaultRoute=darktable`
  - recorded booth package는 `laneOwner=inline-truthful-fallback`
  - `fallbackReason=shadow-submission-only`
  - `originalVisibleToPresetAppliedVisibleMs=none`
- Activation deliverables:
  - host-owned `branch-config/preview-renderer-policy.json` promotion
  - repeated resident success-path evidence
  - one-action rollback proof
  - Story 1.13 rerun을 activation gap 없이 시작할 수 있는 상태
- Close policy:
  - 이번 스토리는 `prototype -> activation -> guarded cutover -> release close` 중 `activation`만 소유한다.
  - speed alone, automated proof alone, canary 한 번의 통과만으로 `default` 승격을 닫으면 안 된다.
  - active session truth, preset binding, same-slot replacement, truthful `Preview Waiting`, booth-safe fallback은 activation 중에도 깨지면 안 된다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
approved preset/version scope를 resident lane canary/default route로 안전하게 승격하고 싶다,
그래서 preview architecture가 shadow 증거에 머물지 않고 실제 booth 운영 경로에서 repeated success-path를 만들 수 있다.

## Acceptance Criteria

1. approved preset/version과 host-owned route policy가 있을 때 activation은 `preview-renderer-policy.json`을 통해 approved scope를 `shadow` 밖 `canary` 또는 `default`로 승격할 수 있어야 한다. promotion은 future-safe rollout artifact로 기록돼야 하며, active session은 기존 route snapshot과 preset binding을 유지한 채 나중 정책 변경으로 재해석되면 안 된다.
2. promoted route에서 실행한 실세션 evidence는 same-capture / same-session / same-preset-version 기준으로 repeated success-path를 남겨야 한다. 최소 `laneOwner=dedicated-renderer`, `fallbackReason=none`, `routeStage=canary|default`, `warmState=warm-ready|warm-hit`, `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`를 `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`, route policy snapshot과 함께 다시 읽을 수 있어야 한다.
3. activation 이후에도 booth의 customer-safe semantics는 유지돼야 한다. queue saturation, warm-state loss, invalid output, wrong-session output, sidecar unavailable, parity fail, rollback trigger가 발생하면 booth는 false-ready, false-complete, cross-session leakage 없이 approved inline truthful fallback 또는 darktable baseline path로 내려가야 하며, truthful `Preview Waiting`과 same-slot replacement 의미를 깨면 안 된다.
4. activation deliverable은 운영 artifact 수준으로 닫혀야 한다. evidence bundle은 route policy state, parity result, fallback ratio, booth/operator visual evidence, rollback evidence를 포함해야 하며, Story 1.13 rerun이 “activation 구현”이 아니라 final guarded cutover / `Go / No-Go` 판단만 수행할 수 있게 만들어야 한다.
5. operator-safe diagnostics와 governance artifact는 activation 결과를 읽을 수 있어야 한다. 최소 route stage, lane owner, fallback reason, warm state, rollback state, blocker가 operator/release evidence에 남아야 하며, customer-facing copy는 계속 plain language만 유지하고 darktable, sidecar, queue, PIX, ETW 같은 내부 용어를 노출하면 안 된다.

## Tasks / Subtasks

- [x] host-owned route policy promotion 경계를 구현한다. (AC: 1, 3, 4)
  - [x] `branch-config/preview-renderer-policy.json`이 approved preset/version scope를 `shadow -> canary -> default`로 승격할 수 있게 host-owned mutation path를 정리한다.
  - [x] `src-tauri/src/branch_config/mod.rs`, `src-tauri/src/commands/branch_rollout_commands.rs` 또는 동등 경로에서 promotion/rollback이 explicit audit trail과 함께 기록되게 한다.
  - [x] active session이 이미 선택한 route snapshot과 preset binding을 policy 변경으로 재해석하지 않도록 session locking 규칙을 유지한다.

- [x] promoted resident lane success-path를 운영 evidence로 닫는다. (AC: 2, 4, 5)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`, `src-tauri/src/session/session_manifest.rs`, `src-tauri/src/diagnostics/mod.rs` 또는 동등 경로에서 promoted route의 success-path가 existing evidence family로 남게 한다.
  - [x] `preview-promotion-evidence.jsonl`, `session.json`, `timing-events.log`, route policy snapshot, published `bundle.json`, `catalog-state.json`을 같은 capture correlation로 묶어 repeated success-path를 재현 가능하게 한다.
  - [x] `fallbackReason=none`, `routeStage=canary|default`, `warmState=warm-ready|warm-hit`가 activation readiness evidence로 읽히도록 operator-safe projection을 정리한다.

- [x] fallback, rollback, active-session safety를 activation 문맥으로 고정한다. (AC: 1, 3, 4)
  - [x] queue saturation, warm-state loss, invalid output, wrong-session output, sidecar unavailable, parity fail, rollback trigger가 promoted route에서도 booth-safe fallback으로 내려가는지 확인한다.
  - [x] rollback은 one-action path로 유지하고, rollback 이후에도 active session truth와 future-session rollout truth를 섞지 않게 한다.
  - [x] activation 도중에도 same-slot truthful replacement, truthful `Preview Waiting`, post-end truth, cross-session isolation이 유지되게 한다.

- [x] release/governance artifact를 activation owner 기준으로 정렬한다. (AC: 4, 5)
  - [x] `docs/runbooks/preview-promotion-evidence-package.md`, `docs/release-baseline.md`, `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 activation 완료 상태와 Story 1.13 rerun prerequisite를 같은 의미로 읽게 한다.
  - [x] parity result, fallback ratio, route policy state, rollback evidence, blocker를 activation deliverable로 명시한다.
  - [x] Story 1.13이 activation gap을 다시 흡수하지 않도록 prerequisite와 close owner 역할 분리를 유지한다.

- [x] 자동 검증과 실운영 dry-run 근거를 보강한다. (AC: 1, 2, 3, 4, 5)
  - [x] `src-tauri/tests/dedicated_renderer.rs`, `src-tauri/tests/branch_rollout.rs`, `tests/hardware-evidence-scripts.test.ts`, `src/shared-contracts/contracts.test.ts`, `src/governance/hardware-validation-governance.test.ts` 또는 동등 검증에 activation/promotion/rollback drift 방지 케이스를 추가한다.
  - [x] shadow route 우회, manual canary drift, live policy recopy, active-session reinterpretation, missing rollback evidence, fallback-heavy promotion evidence를 실패 케이스로 잠근다.
  - [x] canary에서 repeated success-path를 확보한 뒤에만 default 승격을 허용하는 운영 dry-run 절차를 남긴다.

### Review Findings

- [x] [Review][Patch] `default` 승격이 approved preset/version scope를 넘어 전체 route 기본값을 바꾸고 있어, 다른 preset까지 resident lane으로 유입될 수 있음 [src-tauri/src/branch_config/mod.rs:1305]
- [x] [Review][Patch] one-action rollback이 global `defaultRoute`를 되돌리지 않아, rollback 이후에도 비대상 preset이 계속 promoted lane을 타게 됨 [src-tauri/src/branch_config/mod.rs:1326]
- [x] [Review][Patch] `captured-preview-renderer-policy.json`과 `captured-catalog-state.json`이 capture 시점이 아니라 preview close 시점의 live 상태를 복사해 evidence drift가 발생할 수 있음 [src-tauri/src/render/dedicated_renderer.rs:904]
- [x] [Review][Patch] repeated canary gate가 `sessionId`/`captureId`/`requestId` 중복을 제거하지 않아, 같은 세션의 중복 JSONL만으로 `default` 승격이 풀릴 수 있음 [src-tauri/src/branch_config/mod.rs:1367]

## Dev Notes

### 스토리 범위와 제품 목적

- 이번 스토리는 resident lane 자체를 처음 발명하는 단계가 아니다.
- 목표는 이미 존재하는 resident candidate와 promotion gate를 실제 booth 운영 경계로 승격하는 것이다.
- canonical release close owner는 계속 Story 1.13이다. Story 1.20의 완료 조건은 `Go`가 아니라 “activation gap이 닫혔다”는 상태다.

### 왜 지금 필요한가

- architecture와 epics는 preview adoption 순서를 `prototype -> activation -> guarded cutover -> release close`로 고정했다. 현재 빠져 있던 것은 activation owner였다. [Source: _bmad-output/planning-artifacts/architecture.md] [Source: _bmad-output/planning-artifacts/epics.md]
- implementation readiness report도 Story 1.20 implementation artifact가 없으면 다음 preview architecture execution cycle을 시작하면 안 된다고 명시했다. [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260413.md]
- hardware ledger와 release baseline 기준 현재 상태는 still `shadow-by-default`다. 즉 문제는 “검증이 더 필요함” 이전에 “실제 운영 route가 아직 promoted resident lane이 아님”에 가깝다. [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md] [Source: docs/release-baseline.md]

### 현재 워크스페이스 상태

- repo에는 이미 route policy, session route snapshot, operator diagnostics, promotion evidence bundle, rollback governance 기반이 있다.
- `src-tauri/src/render/dedicated_renderer.rs`는 `preview-renderer-policy.json`을 읽고 `shadow`, `canary`, `default` route stage를 해석한다.
- `src-tauri/tests/dedicated_renderer.rs`는 shadow route 강제 fallback, accepted dedicated renderer close, active-session route snapshot 유지, warm-state loss fallback, preview promotion evidence 기록을 이미 다룬다.
- `src-tauri/tests/branch_rollout.rs`는 safe transition 이전 deferred rollout, staged rollout cancel rollback, last approved baseline restore를 검증한다.
- `tests/hardware-evidence-scripts.test.ts`와 `src/shared-contracts/schemas/hardware-validation.ts`는 evidence bundle에 fallback ratio, visual evidence, rollback evidence를 포함하는 현재 기준선을 고정한다.
- 별도 `project-context.md`는 발견되지 않았다.

### 이전 스토리 인텔리전스

- Story 1.18은 resident GPU-first를 prototype으로 올렸고, route/warm-state/operator-safe projection을 마련했다. 이번 스토리는 그 경로를 운영 승격 owner로 바꾸는 follow-up이다. [Source: _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md]
- Story 1.19는 ETW/WPR/WPA/PIX + parity diff 기반 gate와 evidence bundle을 잠갔다. 이번 스토리는 새 evidence family를 만들지 말고 그 계약을 activation deliverable로 써야 한다. [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- Story 1.13은 activation 완료 이후에만 rerun되는 final guarded cutover / release-close owner다. 1.20 implementer가 1.13 범위를 흡수하면 sequencing이 다시 무너진다. [Source: _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md]

### 제품/아키텍처 가드레일

- promotion은 host-owned `preview-renderer-policy.json`로만 제어해야 한다. dev env opt-in, ad-hoc env var, React direct call은 release substitute가 될 수 없다.
- active session truth와 future-session rollout truth를 섞지 말 것. 이미 열린 세션은 route snapshot/preset binding을 유지해야 한다.
- resident lane이 promoted 되더라도 darktable는 baseline / fallback / parity oracle로 남아야 한다.
- customer surface에는 내부 기술 용어를 추가하지 말 것. activation은 운영 artifact와 diagnostics의 문제이지 UX contract 변경이 아니다.
- shadow evidence 한 번 통과했다고 default 승격을 닫지 말 것. repeated canary evidence와 one-action rollback proof가 먼저다.

### 구현 가드레일

- `preview-renderer-policy.json` mutation은 audit 가능한 host boundary 안에서만 허용할 것.
- evidence bundle은 capture 시점 snapshot을 보존해야 한다. bundle 생성 시 live route policy나 live catalog state를 다시 읽어 덮어쓰면 안 된다.
- promoted route success-path는 same-capture / same-session / same-preset-version correlation을 지켜야 한다.
- fallback-heavy run, parity fail, missing rollback evidence, active-session reinterpretation 흔적이 있으면 promotion 성공으로 닫지 말 것.
- `default` 승격은 canary evidence 반복 확보 이후에만 허용하고, failure 시 shadow/rollback 회귀가 한 액션으로 가능해야 한다.

### 프로젝트 구조 요구사항

- 우선 수정/확인 후보 경로:
  - `src-tauri/src/branch_config/mod.rs`
  - `src-tauri/src/commands/branch_rollout_commands.rs`
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/diagnostics/mod.rs`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `docs/runbooks/preview-promotion-evidence-package.md`
  - `docs/release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `tests/hardware-evidence-scripts.test.ts`
- runtime deliverable 경로:
  - `C:\Users\KimYS\Pictures\dabi_shoot\branch-config\preview-renderer-policy.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\<sessionId>\diagnostics\dedicated-renderer\preview-promotion-evidence.jsonl`

### 테스트 요구사항

- 최소 필수 자동 검증:
  - shadow route는 여전히 inline truthful fallback을 유지하고, dev-only 우회가 release substitute가 되지 않는다.
  - canary/default promoted route는 accepted dedicated renderer close를 남기고, active session은 이후 policy rollback으로도 route snapshot이 재해석되지 않는다.
  - evidence bundle은 route policy snapshot, fallback ratio, visual proof, rollback proof를 capture-correlated artifact로 보존한다.
  - branch rollout/rollback은 safe transition 규칙을 지키고 local settings를 보존한다.
  - operator diagnostics는 route stage, lane owner, fallback reason, warm state를 계속 읽고 customer copy는 기술 용어를 노출하지 않는다.
- 최소 필수 운영 검증:
  - canary route repeated success-path evidence
  - one-action rollback proof
  - parity pass 또는 conditional 해석 규칙
  - false-ready `0`, false-complete `0`, cross-session leak `0`

### 최신 기술 / 제품 컨텍스트

- Microsoft Learn의 Azure App Configuration feature flag 문서는 progressive rollout에서 percentage/group/user targeting과 attached configuration을 사용해 canary/staged rollout을 안전하게 운영할 수 있다고 설명한다. Boothy activation에서도 `preview-renderer-policy.json`을 단순 설정이 아니라 scoped rollout artifact로 취급하는 편이 맞다. 이 문장은 Microsoft 문서를 현재 route policy 설계에 적용한 해석이다. [Source: https://learn.microsoft.com/en-us/azure/azure-app-configuration/manage-feature-flags]
- 같은 Microsoft Learn best practices 문서는 progressive exposure deployment, immutable configuration snapshot, last-known-good rollback이 blast radius를 줄인다고 설명한다. 따라서 Story 1.20은 live policy 덮어쓰기보다 snapshot-preserving evidence와 one-action rollback을 우선해야 한다. 이 문장은 Microsoft 문서를 현재 hardware evidence / rollback contract에 적용한 해석이다. [Source: https://learn.microsoft.com/en-us/azure/azure-app-configuration/howto-best-practices]
- architecture와 현재 repo 테스트는 이미 active-session safe transition, deferred rollout, rollback restore, route snapshot preservation을 구현 방향으로 고정했다. Story 1.20은 새로운 운영 모델을 만들기보다 이 기준선을 promotion owner로 연결해야 한다. 이 문장은 현재 architecture와 test baseline을 종합한 해석이다. [Source: _bmad-output/planning-artifacts/architecture.md] [Source: src-tauri/tests/branch_rollout.rs] [Source: src-tauri/tests/dedicated_renderer.rs]

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.20: resident preview lane activation과 route policy promotion]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260413.md]
- [Source: _bmad-output/planning-artifacts/preview-architecture-gap-analysis-20260413.md]
- [Source: _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md]
- [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- [Source: _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: docs/release-baseline.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: src-tauri/src/commands/branch_rollout_commands.rs]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: src-tauri/src/session/session_manifest.rs]
- [Source: src-tauri/src/diagnostics/mod.rs]
- [Source: src-tauri/tests/dedicated_renderer.rs]
- [Source: src-tauri/tests/branch_rollout.rs]
- [Source: tests/hardware-evidence-scripts.test.ts]
- [Source: https://learn.microsoft.com/en-us/azure/azure-app-configuration/manage-feature-flags]
- [Source: https://learn.microsoft.com/en-us/azure/azure-app-configuration/howto-best-practices]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint status, epics, PRD, architecture, UX, readiness report, preview gap analysis, Story 1.18 / 1.19 / 1.13, hardware ledger, release baseline, evidence runbook, route policy implementation/tests를 교차 분석했다.
- 이번 스토리는 resident lane의 새 prototype이 아니라 `activation owner` 역할을 명시적으로 분리하는 문서로 정렬했다.
- current `No-Go` baseline(`defaultRoute=darktable`, shadow-only evidence)을 story 시작점으로 명시해 Story 1.13 범위를 침범하지 않도록 했다.
- 최신 외부 확인은 Microsoft Learn의 rollout / progressive exposure guidance만 사용했다.

### Debug Log References

- `Get-Content -Raw _bmad/bmm/config.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `Get-Content -Raw _bmad-output/planning-artifacts/epics.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/architecture.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/prd.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/ux-design-specification.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/implementation-readiness-report-20260413.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/preview-architecture-gap-analysis-20260413.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `Get-Content -Raw docs/release-baseline.md`
- `Get-Content -Raw docs/runbooks/preview-promotion-evidence-package.md`
- `Get-Content -Raw src/shared-contracts/schemas/hardware-validation.ts`
- `Get-Content -Raw src-tauri/tests/branch_rollout.rs`
- `Get-Content -Raw src-tauri/tests/dedicated_renderer.rs`
- `rg -n "preview-renderer-policy|routeStage|laneOwner|warmState|fallbackReason" src-tauri/src src/shared-contracts docs tests`
- `cargo test --test branch_rollout --test dedicated_renderer`
- `pnpm test:run tests/hardware-evidence-scripts.test.ts src/governance/hardware-validation-governance.test.ts`

### Completion Notes List

- host-owned preview route promotion/rollback 명령과 감사 이력을 추가해 approved preset/version scope를 `canary`와 `default`로 승격하거나 one-action rollback으로 `shadow`에 되돌릴 수 있게 했다.
- `default` 승격은 repeated canary success-path evidence가 2건 이상일 때만 허용하도록 잠가 activation gate를 운영 규칙으로 고정했다.
- preview promotion evidence가 capture-time `captured-preview-renderer-policy.json`, `captured-catalog-state.json`을 저장해 live policy/catalog recopy drift 없이 rerun 근거를 보존하게 했다.
- active session route snapshot, booth-safe fallback, truthful `Preview Waiting`, same-slot replacement 의미는 기존 guardrail을 유지한 채 host-owned rollback 경로로 재검증했다.
- activation runbook, release baseline, hardware ledger를 Story 1.20 activation owner와 Story 1.13 close owner 분리 기준으로 정렬했다.

### File List

- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/release-baseline.md
- docs/runbooks/preview-promotion-evidence-package.md
- src-tauri/src/branch_config/mod.rs
- src-tauri/src/commands/branch_rollout_commands.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/lib.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/branch_rollout.rs
- src-tauri/tests/dedicated_renderer.rs

### Change Log

- 2026-04-13: Story 1.20 activation owner 구현 완료. host-owned preview route promotion/rollback, capture-time snapshot evidence, repeated canary gate, governance alignment, regression tests를 추가하고 상태를 `review`로 변경했다.
