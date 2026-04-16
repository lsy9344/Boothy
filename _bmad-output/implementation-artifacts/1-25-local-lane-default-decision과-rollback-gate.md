# Story 1.25: local lane default decision과 rollback gate

Status: done

Ordering Note: Story 1.25는 Story 1.24가 approved hardware canary를 `Go / No-Go`로 판정한 다음에 와야 한다. 이 스토리는 local lane을 release-close 직전의 운영 기본 경로 후보로 승격할지, 아니면 즉시 rollback할지를 host-owned gate로 확정하는 owner이며, Story 1.13이 마지막 guarded cutover / release close를 맡는다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
local lane의 default 승격 여부를 명시적 gate로 결정하고 싶다,
그래서 canary 성공과 release-close 판단 사이에 운영 가능한 default/rollback owner가 존재하게 할 수 있다.

## Acceptance Criteria

1. Story 1.24 canary가 `Go` 후보를 만들었을 때 default decision gate를 실행하면, host-owned `preview-renderer-policy.json`은 approved scope를 `canary`에서 `default`로 승격하거나 즉시 되돌릴 수 있어야 한다. 또한 active session은 route policy 변경으로 재해석되면 안 된다.
2. default decision 결과를 검토할 때 operator-safe package를 읽으면, `laneOwner`, `routeStage=default|rollback`, `fallbackReason`, KPI 결과, rollback proof를 함께 확인할 수 있어야 한다. 또한 Story 1.13은 Story 1.25가 local lane `Go` 후보를 만들기 전에는 시작되면 안 된다.

## Tasks / Subtasks

- [x] Story 1.24 canary verdict를 Story 1.25 default gate 입력으로 연결한다. (AC: 1, 2)
  - [x] `src-tauri/src/branch_config/mod.rs` 또는 동등 경로에서 scoped default 승격이 raw success-path 흔적만 읽는 느슨한 gate가 아니라, Story 1.24가 남긴 typed canary verdict와 rollback readiness를 함께 확인하는 명시적 decision input을 소비하도록 맞춘다.
  - [x] default 승격은 같은 `presetId/publishedVersion` scope에서 repeated canary success-path evidence, typed `Go`, snapshot completeness, active-session safety를 모두 만족할 때만 열리도록 유지한다.
  - [x] KPI miss, fallback-heavy, wrong-capture, fidelity drift, missing rollback proof, stale or incomplete evidence 중 하나라도 남으면 fail-closed rejection과 typed guidance를 반환해야 한다.

- [x] host-owned scoped default/rollback mutation을 제품 경계에 맞게 잠근다. (AC: 1)
  - [x] `preview-renderer-policy.json` 변경은 Rust host만 수행하고, selected `presetId/publishedVersion` scope에 대한 `canary -> default` 승격 또는 one-action rollback만 허용한다.
  - [x] 이번 스토리의 `default`는 전체 booth global default 교체가 아니라 approved preset/version scope에 대한 운영 기본 경로 승격임을 UI, DTO, audit wording에서 일관되게 유지한다.
  - [x] rollback은 matching `default/canary` scope를 제거하고 shadow fallback을 강제하되, active session이 이미 잡은 `activePreviewRendererRoute`와 capture-time meaning은 재해석하지 않게 유지한다.

- [x] operator-safe review package와 governance surface를 정렬한다. (AC: 2)
  - [x] `src/branch-config/components/PreviewRouteGovernancePanel.tsx`, `src/branch-config/services/branch-rollout-service.ts`, `src/shared-contracts/schemas/branch-rollout.ts` 또는 동등 경로에서 current status, mutation result, canary evidence count, rejection reason이 scoped default decision으로 읽히도록 맞춘다.
  - [x] decision review artifact 또는 package summary에는 `laneOwner`, `routeStage`, `fallbackReason`, KPI/canary verdict, rollback proof presence, scope identity(`presetId`, `publishedVersion`)가 함께 남아 operator가 speed-only 승인을 하지 못하게 해야 한다.
  - [x] 고객용 surface에는 routeStage, laneOwner, rollback 같은 내부 운영 용어를 노출하지 않고, settings/operator surface에서만 governance truth를 읽게 유지한다.

- [x] 기존 계약과 증거 체인을 재사용하고 중복 구현을 피한다. (AC: 1, 2)
  - [x] Story 1.24의 `preview-promotion-canary-assessment/v1`, `preview-promotion-evidence.jsonl`, captured route/catalog snapshot, rollback proof bundle을 그대로 decision input으로 재사용하고, 별도 parallel evidence family를 발명하지 않는다.
  - [x] `session-manifest/v1`, `render-worker`, `local-dedicated-renderer`, `preview-promotion-evidence-package` 문서가 이미 잠근 active-session snapshot / parity / fallback / rollback semantics를 깨지 않게 유지한다.
  - [x] `laneOwner=local-fullscreen-lane` 같은 새 vocabulary가 필요하더라도, 현재 repo가 이미 사용하는 typed `laneOwner` 계약과 fixture/test baseline을 한 번에 정렬하지 못하면 임의 rename을 하지 않는다.

- [x] 회귀 테스트와 문서 증거를 Story 1.25 기준으로 잠근다. (AC: 1, 2)
  - [x] Rust test: typed `Go` verdict 없이 default 승격이 거절되고, duplicate capture evidence나 incomplete snapshot이 승격을 열지 못하며, rollback이 scope-local one action으로 적용되는지 검증한다.
  - [x] TypeScript test: preview route mutation/result schema, service mapping, settings/governance panel copy가 scoped default/rollback semantics를 유지하는지 검증한다.
  - [x] runbook/release baseline/contract 문서에서 Story 1.25가 default/rollback gate owner임을 유지하고, Story 1.13 final close ownership을 흡수하지 않는지 확인한다.

### Review Findings

- [x] [Review][Patch] Rollback decision surface는 내부 계약의 `shadow`를 유지하고 operator-safe package/UI에서만 `rollback`으로 별도 노출하도록 정렬 [src/shared-contracts/schemas/branch-rollout.ts:210]
- [x] [Review][Patch] Default gate가 Story 1.24 typed canary assessment의 전체 health contract를 fail-closed로 검증하지 않음 [src-tauri/src/branch_config/mod.rs:1611]
- [x] [Review][Patch] Default gate가 repeated success evidence와 typed canary verdict를 서로 다른 capture/run에서 조합할 수 있음 [src-tauri/src/branch_config/mod.rs:1569]
- [x] [Review][Patch] Status summary가 정상 route reason을 fallback reason으로 잘못 노출함 [src-tauri/src/branch_config/mod.rs:469]

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- sprint plan은 새 preview architecture forward path를 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25`로 고정했고, Story 1.25를 explicit default-promotion / rollback gate로 정의한다. 이 단계가 없으면 canary `Go`와 final release close 사이에 운영 가능한 route decision owner가 비게 된다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- epics도 Story 1.25가 `preview-renderer-policy.json`을 통해 approved scope를 `canary`에서 `default`로 승격하거나 rollback하고, Story 1.13이 그 전에는 시작되면 안 된다고 못 박는다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.25: local lane default decision과 rollback gate]

### 스토리 범위와 목적

- 이번 스토리의 핵심은 local lane을 무조건 release-close로 넘기는 것이 아니라, host-owned policy와 typed evidence를 이용해 `default` 또는 `rollback`을 명시적으로 결정하는 것이다.
- 이번 스토리는 아래를 소유한다.
  - Story 1.24 canary verdict를 읽는 scoped default decision gate
  - `preview-renderer-policy.json` 기반의 host-owned promotion / rollback mutation
  - operator-safe decision package와 audit/history 정렬
- 아래 작업은 이번 스토리 범위가 아니다.
  - local lane prototype 자체 재구현
  - hardware canary 재평가 로직 재설계
  - final guarded cutover / release-close claim
  - local lane 실패 전 reserve remote path 개시

### 범위 조정 메모

- epics의 canonical 표현은 `laneOwner=local-fullscreen-lane` 중심이지만, 현재 repo의 일부 shared contracts, fixtures, tests는 이전 evidence family에서 legacy laneOwner 어휘를 아직 포함한다.
- 따라서 이번 스토리의 안전한 해석은 "host-owned local full-screen lane evidence family를 승인 scope의 운영 기본 경로로 승격할지 판단한다"는 의미다.
- dev agent는 전역 rename으로 vocabulary를 흔들기보다, operator-safe summary wording과 typed contracts를 같은 change set 안에서 정렬할 수 있을 때만 coordinated change를 해야 한다.
- 현재 구현의 `default`는 global booth default route 교체가 아니라 선택한 preset/version scope의 운영 기본 경로 승격이다. `policy.defaultRoute` 자체를 새 아키텍처 전역 기본값처럼 뒤집는 방향은 이번 스토리 해석과 다르다.

### 스토리 기반 요구사항

- PRD는 primary release sign-off를 `same-capture preset-applied full-screen visible <= 2500ms`로 고정하고, release decision이 latency, same-capture correctness, preset fidelity, fallback stability evidence를 함께 읽어야 한다고 명시한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- PRD는 rollout/rollback이 staged, auditable, one approved rollback action, active-session compatibility를 만족해야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-006 Safe Local Packaging, Rollout, and Version Pinning]
- architecture는 `preview-renderer-policy.json`을 route policy이자 rollout artifact로 정의하고, promoted `canary/default` state를 release evidence 일부로 다룬다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- sprint plan과 release baseline은 Story 1.25가 `preview-renderer-policy.json` transition, one-action rollback proof, active session non-reinterpretation을 닫기 전에는 Story 1.13으로 넘어가면 안 된다고 정리한다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md] [Source: docs/release-baseline.md]

### 선행 의존성과 현재 기준선

- 직접 이전 스토리는 Story 1.24다. 이 스토리가 `preview-promotion-canary-assessment/v1` artifact와 rollback readiness, wrong-capture, fidelity drift, active-session safety를 한 번에 읽는 fail-closed canary gate를 잠갔다. [Source: _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md]
- Story 1.23은 local lane prototype owner와 selected-capture evidence family를 정리했고, Story 1.25는 그 결과를 default decision으로 읽는 follow-up이다. [Source: _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md]
- docs/runbooks/preview-promotion-evidence-package.md는 selected-capture bundle, route/catalog snapshot, canary verdict, rollback evidence를 promotion proof 패키지 기준선으로 이미 묶어 두었다. [Source: docs/runbooks/preview-promotion-evidence-package.md]

### 현재 워크스페이스 상태

- `src-tauri/src/branch_config/mod.rs`는 이미 preview route promotion/rollback/status load, policy/history atomic write, operator audit append, repeated canary success-path count를 갖고 있다. 즉 이번 스토리는 route governance를 새로 만드는 작업보다 현재 gate를 Story 1.24 verdict와 정확히 연결하는 hardening에 가깝다.
- `src-tauri/tests/branch_rollout.rs`는 repeated canary success-path evidence 부족, duplicate capture evidence, snapshot evidence 누락, canonical full-screen metric alias 처리, rollback mutation을 이미 테스트하고 있다. Story 1.25는 이 baseline을 확장해야지 버리면 안 된다.
- `src/branch-config/components/PreviewRouteGovernancePanel.tsx`와 `src/branch-config/services/branch-rollout-service.ts`는 settings/operator surface에서 scoped canary/default promotion과 rollback을 조작하는 현재 UI/service 경계다.
- `src/shared-contracts/schemas/branch-rollout.ts`는 preview route mutation/result/status 계약을 이미 typed schema로 갖고 있다. Story 1.25는 ad-hoc JSON parsing보다 이 schema family를 확장하는 편이 안전하다.
- `docs/contracts/session-manifest.md`는 active session이 capture-time route snapshot 위에서만 해석되어야 하며, later policy change 또는 rollback이 이미 선택된 세션 의미를 재해석하면 안 된다고 못 박는다.
- 별도 `project-context.md`는 발견되지 않았다.

### 이전 스토리 및 인접 스토리 인텔리전스

- Story 1.24는 canary gate를 typed `Go / No-Go` assessment artifact로 닫았지만, 현재 host default promotion baseline은 여전히 raw success-path record count 중심으로 읽는 부분이 있다. Story 1.25는 이 간극을 줄여 "canary proof와 default decision"을 같은 제품 언어로 이어야 한다.
- Story 1.23은 selected-capture evidence chain과 local lane prototype ownership을 이미 잠갔다. Story 1.25는 새 timing family나 route snapshot family를 만들지 말고 그 chain을 그대로 decision input으로 재사용해야 한다.
- Story 1.13은 final guarded cutover / release-close owner다. Story 1.25는 `Go` 후보와 rollback proof까지가 범위이며, final ledger close ownership을 흡수하면 안 된다.

### 최근 구현 패턴과 git 인텔리전스

- 최근 커밋은 `local renderer contracts`, `preset applied rendering and diagnostics`, `dedicated renderer rollout work`를 순차적으로 강화하는 방향이다. 현재 repo의 흐름은 existing host-owned boundary를 증거/계약 중심으로 다듬는 쪽이다. [Source: git log -5 --oneline]
- 따라서 Story 1.25도 새 distributed governance subsystem을 만들기보다, existing branch-config + shared-contracts + evidence bundle 흐름을 Story 1.24 verdict에 정확히 연결하는 편이 안전하다. 이 문장은 현재 repo와 최근 커밋 흐름을 근거로 한 해석이다.

### 구현 가드레일

- default 승격은 speed-only decision이면 안 된다. typed canary verdict, snapshot completeness, rollback readiness, active-session safety를 함께 읽어야 한다.
- `preview-renderer-policy.json` mutation은 Rust host만 소유해야 하며, React/browser가 route truth를 직접 계산하거나 policy file을 쓰게 만들면 안 된다.
- `default`는 selected preset/version scope에 대한 운영 기본 경로 승격이다. global booth default route를 전면 교체하는 구현으로 확대 해석하면 안 된다.
- rollback은 one-action이어야 하지만, active session의 capture-time route snapshot과 이미 기록된 booth run meaning을 바꾸는 방식이면 안 된다.
- Story 1.24 canary verdict와 Story 1.25 default decision package를 따로 놀게 만드는 parallel evidence family 추가 금지
- Story 1.25 안에서 Story 1.13 final release close, hardware ledger 최종 `Go`, Story 1.26 reserve remote decision까지 같이 흡수하는 것 금지

### 아키텍처 준수사항

- route rollout artifact는 host-owned `preview-renderer-policy.json`이며, `canary/default` promoted state 자체가 release evidence 일부다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- preview pipeline은 `first-visible lane`, `display-sized truthful artifact lane`, `truth/parity reference lane`으로 분리되어 있고, Story 1.25는 hot path artifact owner를 다시 만드는 단계가 아니라 그 결과를 운영 decision으로 읽는 단계다. [Source: _bmad-output/planning-artifacts/architecture.md#Preview Architecture Realignment] [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- active session truth와 route truth는 host snapshot이 소유하고, later publish/rollback 또는 policy change가 이미 선택된 세션 의미를 바꾸면 안 된다. [Source: docs/contracts/session-manifest.md] [Source: docs/contracts/render-worker.md]
- branch rollout/rollback과 preview route promotion/rollback은 settings/operator surface에서만 열려야 하며, 고객 surface에 노출되면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]

### 프로젝트 구조 요구사항

- 우선 검토 경로:
  - `src-tauri/src/branch_config/mod.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `src/branch-config/components/PreviewRouteGovernancePanel.tsx`
  - `src/branch-config/services/branch-rollout-service.ts`
  - `src/branch-config/services/branch-rollout-service.test.ts`
  - `src/shared-contracts/schemas/branch-rollout.ts`
  - `src/shared-contracts/branch-rollout.contracts.test.ts`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src/settings/screens/SettingsScreen.tsx`
  - `src/settings/screens/SettingsScreen.test.tsx`
  - `docs/runbooks/preview-promotion-evidence-package.md`
  - `docs/contracts/local-dedicated-renderer.md`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
  - `docs/release-baseline.md`
- scope를 넘기지 않도록 주의할 경로:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
  - `_bmad-output/implementation-artifacts/1-26-local-lane-실패-시에만-remote-reserve-poc-개시.md`
- 이번 스토리는 branch-config, shared contracts, evidence/runbook, settings/operator governance boundary를 보강하는 수준에서 끝내고, new runtime architecture fork를 만들지 않는다.

### UX 구현 요구사항

- settings/operator surface는 default/rollback decision을 명확히 보여줄 수 있지만, 고객 surface에는 routeStage, laneOwner, rollback, canary 같은 내부 운영 용어를 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#구속되는-UX-요구사항]
- 내부 review copy도 speed-only 승인을 유도하면 안 된다. operator는 KPI, fallback, rollback proof, scope identity를 함께 읽어야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름] [Source: docs/release-baseline.md]
- governance panel의 `default` 문구는 "전체 부스 기본값 변경"처럼 읽히지 않게 해야 하고, 선택한 preset/version scope에 대한 승격임을 유지해야 한다. 이 문장은 현재 UI copy와 architecture boundary를 바탕으로 한 구현 추론이다.

### 최신 기술 확인 메모

- 현재 로컬 기준선은 `react 19.2.4`, `@tauri-apps/api 2.10.1`, `zod 4.3.6`, Rust `tauri 2.10.3`이다. Story 1.25는 새 의존성 추가보다 이 기준선 안에서 host-owned governance와 typed contract를 정렬하는 편이 안전하다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- Tauri v2 공식 문서는 frontend에서 `invoke('command_name', { ... })`와 Rust `#[tauri::command]` 경계를 기본 command boundary로 설명한다. preview route promotion/rollback은 이미 이 패턴을 따르므로 새로운 IPC 스타일을 발명할 이유가 없다. [Source: https://v2.tauri.app/develop/calling-rust/]
- Tauri v2 공식 문서는 streaming에는 channel을 권장한다. Story 1.25의 preview route decision은 point-in-time mutation/status 성격이므로 기존 command/status load 모델을 유지하는 편이 맞다. 이것은 공식 문서를 바탕으로 한 구현 추론이다. [Source: https://v2.tauri.app/develop/calling-rust/] [Source: https://v2.tauri.app/develop/calling-frontend/]
- Zod 공식 문서는 TypeScript-first schema validation을 전면에 둔다. preview route mutation/result/status, canary assessment, operator-safe package는 ad-hoc parsing보다 현재 shared-contracts schema family를 유지하는 편이 안전하다. [Source: https://zod.dev/]

### 테스트 요구사항

- 최소 필수 테스트 범위:
  - typed canary `Go` verdict와 repeated canary success-path evidence가 모두 있어야 default 승격이 열린다.
  - duplicate capture evidence, incomplete snapshot, stale or missing rollback proof는 승격을 열지 못한다.
  - rollback은 matching preset/version scope만 shadow fallback으로 되돌리고, unrelated scope는 유지한다.
  - active session snapshot은 later policy change 또는 rollback 때문에 재해석되지 않는다.
  - preview route status/result copy가 scoped `shadow/canary/default` semantics를 유지한다.
  - operator-safe decision package가 KPI/canary verdict, fallbackReason, rollback proof, routeStage를 함께 노출한다.
- 권장 추가 검증:
  - raw success-path count와 typed canary assessment가 서로 충돌할 때 fail-closed rejection되는지 확인
  - governance panel이 `default`를 global default change처럼 잘못 설명하지 않는지 copy regression 추가
  - operator audit log가 rejection / apply / rollback reason code를 일관되게 남기는지 확인

### 금지사항 / 안티패턴

- raw `preview-promotion-evidence.jsonl` count만으로 default 승격을 닫고 Story 1.24 typed verdict를 무시하는 것 금지
- React/browser가 `preview-renderer-policy.json`를 직접 수정하거나 route truth를 소유하는 것 금지
- `default`를 전체 booth 전역 기본값 변경으로 확대 해석하는 것 금지
- rollback이 active session meaning이나 capture-time snapshot을 다시 해석하게 만드는 것 금지
- Story 1.25 안에서 final release close, reserve remote option, hardware ledger 최종 승인까지 흡수하는 것 금지

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.25: local lane default decision과 rollback gate]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-006 Safe Local Packaging, Rollout, and Version Pinning]
- [Source: _bmad-output/planning-artifacts/architecture.md#Preview Architecture Realignment]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- [Source: _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md]
- [Source: _bmad-output/implementation-artifacts/1-24-local-lane-hardware-canary-validation.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: docs/release-baseline.md]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: src-tauri/tests/branch_rollout.rs]
- [Source: src/branch-config/components/PreviewRouteGovernancePanel.tsx]
- [Source: src/branch-config/services/branch-rollout-service.ts]
- [Source: src/shared-contracts/schemas/branch-rollout.ts]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/contracts.test.ts]
- [Source: src/shared-contracts/branch-rollout.contracts.test.ts]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]
- [Source: https://v2.tauri.app/develop/calling-rust/]
- [Source: https://v2.tauri.app/develop/calling-frontend/]
- [Source: https://zod.dev/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-15 17:15:53 +09:00 - Story 1.25 context 생성: config, sprint status, epics, PRD, architecture, UX, Story 1.23/1.24, release baseline, runbook, branch-config host/service/UI, shared contracts/tests, 최근 git log를 교차 분석해 default decision과 rollback gate 경계를 문서화했다.
- 2026-04-15 17:35:20 +09:00 - Rust host gate를 Story 1.24 typed canary assessment와 연결하고, repeated canary evidence + typed Go + rollback readiness + active-session safety를 모두 만족할 때만 scoped default promotion이 열리도록 fail-closed rejection을 구현했다.
- 2026-04-15 17:35:20 +09:00 - settings/operator governance surface와 shared contract에 operator-safe decision summary(`laneOwner`, fallback reason, canary verdict, KPI status, rollback proof)를 추가하고, Vitest/Cargo 회귀를 실행했다.

### Completion Notes List

- Story 1.25를 "새 preview governance를 발명하는 일"이 아니라, existing branch-config/policy/history boundary를 Story 1.24 typed canary verdict와 연결하는 gate hardening 스토리로 정리했다.
- scoped default 승격, one-action rollback, active-session non-reinterpretation, operator-safe review package, final close boundary를 명시적으로 분리했다.
- Story 1.24 typed canary verdict가 없거나 `No-Go`이면 Story 1.25 default 승격이 fail-closed로 거절되도록 고정했다.
- settings surface에서만 scoped default/rollback review package를 읽을 수 있게 하고, lane owner, fallback reason, canary verdict, KPI status, rollback proof를 함께 보여 speed-only 승인 경로를 막았다.
- `pnpm lint`, `pnpm test:run`, `cargo test --test branch_rollout`는 통과했고, 전체 `cargo test`는 변경 범위 밖 기존 회귀 `render::tests::preview_invocation_avoids_pending_canonical_preview_assets_during_truthful_close` 1건 때문에 실패했다.

### File List

- _bmad-output/implementation-artifacts/1-25-local-lane-default-decision과-rollback-gate.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src-tauri/src/branch_config/mod.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/tests/branch_rollout.rs
- src/shared-contracts/dto/branch-rollout.ts
- src/shared-contracts/schemas/branch-rollout.ts
- src/shared-contracts/branch-rollout.contracts.test.ts
- src/branch-config/components/PreviewRouteGovernancePanel.tsx
- src/settings/screens/SettingsScreen.test.tsx

### Change Log

- 2026-04-15: Story 1.24 typed canary assessment를 Story 1.25 default/rollback gate에 연결하고, operator-safe decision summary와 회귀 테스트를 추가했다.
