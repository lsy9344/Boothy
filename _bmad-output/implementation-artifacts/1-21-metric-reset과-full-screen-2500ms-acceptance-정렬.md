# Story 1.21: metric reset과 full-screen 2500ms acceptance 정렬

Status: done

Ordering Note: 새 preview architecture track의 첫 스토리는 1.21이어야 한다. 이 스토리가 먼저 닫혀야 Story 1.22 trace reset과 Story 1.23 local full-screen lane prototype이 잘못된 성공 지표를 기준으로 증거를 쌓지 않게 막을 수 있다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
제품의 preview 합격 기준을 customer-visible full-screen KPI로 다시 고정하고 싶다,
그래서 이후 trace reset과 local lane prototype 검증이 실제 고객 체감과 같은 기준으로 진행될 수 있다.

## Acceptance Criteria

1. release sign-off 기준을 재정의할 때, 제품 기준 문서와 운영 runbook은 primary acceptance를 `same-capture preset-applied full-screen visible <= 2500ms`로 통일해야 한다. preview confirmation은 별도의 보조 가드레일로만 남아야 하며, tiny preview, first-visible, recent-strip update만으로 합격을 선언하면 안 된다.
2. operator-safe diagnostics, evidence contract, hardware ledger template를 검토할 때, `sameCaptureFullScreenVisibleMs`가 필수 판정 필드로 승격되어야 한다. 또한 legacy `replacementMs`는 비교용 또는 backward-compatible alias로만 남아야 하며, `firstVisibleMs`는 comparison/diagnostic 값으로만 읽혀야 하고 release 판단을 대체하면 안 된다.
3. legacy preview track(Stories 1.18, 1.19, 1.20) evidence를 다시 읽을 때, 기존 activation baseline은 보존되어야 한다. 또한 새 metric reset 이후 생성되는 sign-off, dashboard, bundle, ledger 문구와 혼용되지 않도록 `legacy comparison only`와 `new-track release field`의 경계가 분명해야 한다.

## Tasks / Subtasks

- [x] 제품 기준 문서와 runbook의 합격 언어를 통일한다. (AC: 1, 3)
  - [x] `docs/release-baseline.md`, 루트 `release-baseline.md`, `docs/runbooks/preview-promotion-evidence-package.md`, `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에서 primary KPI를 `same-capture preset-applied full-screen visible <= 2500ms`로 같은 표현으로 고정한다.
  - [x] preview confirmation은 보조 가드레일로만 남기고, first-visible 또는 tiny-preview 성공이 release close를 의미하지 않는다는 문구를 빠짐없이 반영한다.
  - [x] legacy 1.18~1.20 evidence는 comparison baseline으로만 남기고, active forward path는 1.21~1.25라는 ownership 문구를 문서 전반에서 일관되게 유지한다.

- [x] canonical decision field를 shared contract와 operator-safe projection에 승격한다. (AC: 1, 2)
  - [x] `src/shared-contracts/schemas/hardware-validation.ts`, `src/shared-contracts/schemas/operator-diagnostics.ts`, 관련 DTO/fixture/test에서 `sameCaptureFullScreenVisibleMs`를 canonical field로 추가하거나 승격한다.
  - [x] 1.22 이전 호환성을 위해 legacy `replacementMs`를 즉시 제거하지는 말고, 기존 evidence를 읽을 수 있는 alias 또는 derived mapping을 유지한다.
  - [x] `src/operator-console/screens/OperatorSummaryScreen.tsx`와 관련 service/test에서 goal card, field label, acceptance copy가 새 KPI를 기준으로 동작하게 맞춘다.

- [x] trace reset 이전의 경계와 scope를 명확히 잠근다. (AC: 2, 3)
  - [x] Story 1.21은 metric/acceptance alignment owner이며, capture correlation chain reset은 Story 1.22가 소유한다는 범위 분리를 문서와 테스트에서 분명히 남긴다.
  - [x] existing `preview-promotion-evidence.jsonl`, session diagnostics, route policy snapshot 흐름은 유지하고, 이 스토리에서 trace event family를 새로 발명하거나 lane prototype 동작을 바꾸지 않는다.
  - [x] legacy evidence bundle과 새 track evidence bundle이 같은 용어를 써도 release 의미가 섞이지 않도록 template, fixture, 설명 문구를 정리한다.

- [x] governance/contract/UI 회귀 검증을 추가한다. (AC: 1, 2, 3)
  - [x] `src/governance/hardware-validation-governance.test.ts` 또는 동등 검증에서 release baseline, runbook, ledger wording이 새 KPI로 정렬되는지 고정한다.
  - [x] `src/shared-contracts/contracts.test.ts`, `tests/hardware-evidence-scripts.test.ts`, operator console test에서 canonical field, backward compatibility, first-visible-only false pass 방지 케이스를 추가한다.
  - [x] legacy fixture 하나와 new-track fixture 하나를 함께 검증해 comparison baseline 보존과 new-track primary field 요구사항이 동시에 유지되는지 확인한다.

### Review Findings

- [x] [Review][Patch] Canonical-only evidence still fails branch promotion gating [src-tauri/src/branch_config/mod.rs:1515]
- [x] [Review][Patch] Dedicated renderer evidence records still omit `improvementSummary` expected by the updated contract/test baseline [src-tauri/src/render/dedicated_renderer.rs:111]
- [x] [Review][Patch] Test fixtures now encode the secondary preview-confirmation guardrail as `2500ms` instead of the required `5s` policy [src/shared-contracts/contracts.test.ts:1998]

## Dev Notes

### 왜 이 스토리가 먼저인가

- sprint plan은 새 preview architecture track의 실행 순서를 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25`로 고정했고, immediate next story도 1.21로 명시했다. 이 순서를 어기면 1.22와 1.23이 잘못된 success metric 위에서 증거를 쌓게 된다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- epics도 Story 1.21을 `제품 합격 기준을 full-screen visible KPI로 다시 고정`하는 스토리로 정의했고, 바로 다음 Story 1.22/1.23이 trace reset과 local lane prototype을 담당한다고 분리했다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.21: metric reset과 full-screen 2500ms acceptance 정렬] [Source: _bmad-output/planning-artifacts/epics.md#Story 1.22: capture -> full-screen visible evidence chain trace reset] [Source: _bmad-output/planning-artifacts/epics.md#Story 1.23: local full-screen lane prototype과 truthful artifact generation]

### 스토리 목적과 범위

- 이번 스토리의 본질은 더 빠른 renderer를 만드는 일이 아니라, 어떤 숫자가 제품 `Go / No-Go`를 결정하는지 다시 잠그는 일이다.
- 같은 맥락에서 Story 1.21은 `metric reset / acceptance alignment` owner다.
- 따라서 아래 작업은 이번 스토리 범위가 아니다.
  - 새 capture correlation chain 설계 및 event reset
  - local full-screen lane prototype 구현
  - hardware canary 실행이나 default decision gate
- 이 스토리의 안전한 해석은 다음과 같다.
  - release language를 full-screen KPI로 통일한다.
  - dashboards/contracts/ledger가 동일한 primary field를 본다.
  - legacy baseline은 지우지 않고 comparison-only로 보존한다.

### 스토리 기반 요구사항

- 이번 스토리에서는 preview confirmation을 보조 가드레일로만 남겨 제품 판단 기준이 primary full-screen KPI와 섞이지 않게 한다.
- release gate도 approved booth hardware에서 같은 KPI를 보여야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#Release-Gates]
- architecture는 hot path를 `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + darktable-compatible truth/parity reference`로 재정렬했고, initial implementation priority의 첫 번째도 KPI reset과 trace/evidence reset이다. [Source: _bmad-output/planning-artifacts/architecture.md#Preview-Architecture-Realignment] [Source: _bmad-output/planning-artifacts/architecture.md#Initial-Implementation-Priorities]

### 현재 워크스페이스 상태

- `src/shared-contracts/schemas/hardware-validation.ts`와 `src/shared-contracts/schemas/operator-diagnostics.ts`는 아직 `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs` 구조를 중심으로 되어 있다.
- `src/operator-console/screens/OperatorSummaryScreen.tsx`도 현재 goal card와 주요 지표 표기를 `replacementMs` 기준으로 보여준다.
- 루트 `release-baseline.md`는 여전히 `replacementMs <= 2500` 표현을 쓰고 있지만, `docs/release-baseline.md`와 sprint plan은 이미 새 track ownership과 language를 반영했다.
- 따라서 이번 스토리는 "새 개념 도입"보다는 이미 바뀐 planning truth를 repo 전반의 contract/doc/UI wording에 일치시키는 정렬 작업에 가깝다.

### 이전 스토리 인텔리전스

- Story 1.19는 promotion evidence gate, parity oracle, replayable evidence bundle 기준을 잠갔다. 이번 스토리는 그 evidence family를 버리지 말고, 어떤 field가 primary decision인지 의미만 다시 정렬해야 한다. [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- Story 1.20은 host-owned promotion/rollback audit와 capture-time policy snapshot을 정리한 historical baseline이다. Story 1.21이 이 흐름을 다시 구현하려 들면 범위가 깨진다. [Source: _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md]
- legacy Stories 1.18~1.20은 새 track의 current owner가 아니라 historical baseline이라는 점을 계속 유지해야 한다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]

### 구현 가드레일

- `sameCaptureFullScreenVisibleMs`를 canonical release field로 승격하되, 이미 저장된 legacy evidence를 읽지 못하게 만드는 destructive rename은 금지한다.
- Story 1.21에서 event emission, route policy behavior, parity logic, fallback semantics를 바꾸지 않는다. 그 작업은 Story 1.22 이후 scope다.
- `firstVisibleMs`는 빠른 고객 안심 피드백 또는 comparison metric으로는 남을 수 있지만, primary release decision field처럼 보이면 안 된다.
- operator copy와 ledger wording은 "full-screen visible KPI"를 명확히 말해야 하며, tiny preview나 recent-strip success가 합격처럼 읽히면 안 된다.
- legacy comparison baseline과 new-track release field를 구분하는 설명은 문서와 fixture 양쪽에 함께 남겨야 한다.

### 아키텍처 준수사항

- preview pipeline은 `first-visible lane`, `display-sized truthful artifact lane`, `truth/parity reference lane`로 분리돼 있고, 제품 합격은 첫 번째가 아니라 두 번째 lane 기준으로 닫혀야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- preview truth는 host-owned lane이 만든 truthful artifact에만 귀속되며, early visible image는 상태 안내를 도와도 release truth를 대신할 수 없다. [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- customer UX도 same-capture first-visible image가 먼저 보일 수는 있어도 truthful preset-applied close 전까지 상태는 `Preview Waiting`으로 유지해야 한다고 명시한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview-Waiting-보호-흐름]

### 프로젝트 구조 요구사항

- 우선 수정/검토 경로:
  - `docs/release-baseline.md`
  - `release-baseline.md`
  - `docs/runbooks/preview-promotion-evidence-package.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `tests/hardware-evidence-scripts.test.ts`
  - `tests/fixtures/contracts/preview-promotion-evidence-record-v1.json`
  - `src/operator-console/screens/OperatorSummaryScreen.tsx`
  - `src/operator-console/screens/OperatorSummaryScreen.test.tsx`
  - `src/operator-console/services/operator-diagnostics-service.test.ts`
  - `src/governance/hardware-validation-governance.test.ts`
- 필요 시 수정 가능하지만 scope를 넘기지 말아야 할 경로:
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
  - `src-tauri/src/diagnostics/mod.rs`
  - `src-tauri/src/render/dedicated_renderer.rs`
- 후자 경로는 field alias/projection 보정이 필요할 때만 건드리고, trace reset이나 prototype 동작 변경은 다음 스토리로 남긴다.

### UX 구현 요구사항

- 고객에게 중요한 약속은 `촬영은 저장되었고 확인용 사진을 준비 중`이라는 truthful waiting semantics와, 최종 판단이 same-capture full-screen visible 결과라는 점이다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#2.5-Experience-Mechanics] [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview-Waiting-보호-흐름]
- operator UI는 내부 진단을 보여줄 수 있지만, 어떤 수치가 release-governance의 primary field인지 혼동되지 않게 해야 한다.
- 이번 스토리에서 고객-facing copy contract를 새로 늘리거나 기술 용어를 추가하지 않는다.

### 테스트 요구사항

- 최소 필수 자동 검증:
  - governance test가 release baseline, runbook, ledger wording을 `same-capture preset-applied full-screen visible <= 2500ms`로 함께 잠근다.
  - contract/schema test가 `sameCaptureFullScreenVisibleMs`를 canonical field로 검증하고, legacy `replacementMs` fixture를 backward-compatible하게 읽되 release-primary field와 혼동하지 않도록 막는다.
  - operator UI/service test가 goal card와 주요 metric labeling을 새 KPI 기준으로 표기하는지 확인한다.
  - first-visible-only 또는 preview-ready-only fixture가 false pass로 해석되지 않는 negative case를 추가한다.
- 이번 스토리에서 요구되는 검증은 문서/계약/UI/governance alignment까지다. 새 hardware canary나 local lane prototype 측정은 요구하지 않는다.

### Evidence Expectations

- 체크인된 증거:
  - 문서 diff에서 release baseline, evidence package, hardware ledger가 같은 KPI 문구를 사용한다.
  - contract fixture 또는 sample bundle 하나 이상이 `sameCaptureFullScreenVisibleMs`를 canonical decision field로 포함한다.
  - operator summary test 또는 snapshot이 goal card/field label이 새 KPI를 기준으로 읽힌다는 근거를 남긴다.
- 보존해야 하는 증거:
  - legacy 1.18~1.20 evidence는 삭제/재작성하지 않고 comparison baseline으로 남긴다.
  - `replacementMs` historical value는 필요하면 alias 또는 derived note로 남기되, 새 release-primary field를 대체하지 못하게 한다.
- 이번 스토리에서 요구하지 않는 증거:
  - 새 trace chain 실측
  - local full-screen lane prototype 성공 증명
  - hardware `Go` claim

### 최신 기술 확인 메모

- 이번 스토리는 외부 최신 프레임워크 조사보다 현재 repo의 frozen stack과 planning truth를 정렬하는 작업이다.
- 추가 dependency 도입이나 stack upgrade는 필요하지 않다.
- 따라서 최신 기술 확인은 local contract/doc/test baseline만으로 충분하다. [Source: package.json] [Source: src-tauri/Cargo.toml]

### 금지사항 / 안티패턴

- `replacementMs`를 이름만 바꾸고 여전히 release-primary field처럼 설명하는 것 금지
- legacy evidence를 새 track proof처럼 재분류하는 것 금지
- Story 1.21 안에서 trace schema reset이나 renderer prototype behavior를 함께 구현하는 것 금지
- first-visible, tiny preview, recent-strip visibility를 primary KPI처럼 대시보드/ledger에 표기하는 것 금지
- root `release-baseline.md`와 `docs/release-baseline.md`를 서로 다른 의미로 남겨두는 것 금지

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.21: metric reset과 full-screen 2500ms acceptance 정렬]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.22: capture -> full-screen visible evidence chain trace reset]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.23: local full-screen lane prototype과 truthful artifact generation]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003-Booth-Responsiveness-and-Preview-Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#Release-Gates]
- [Source: _bmad-output/planning-artifacts/architecture.md#Preview-Architecture-Realignment]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Initial-Implementation-Priorities]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview-Waiting-보호-흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#2.5-Experience-Mechanics]
- [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- [Source: _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md]
- [Source: docs/release-baseline.md]
- [Source: release-baseline.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src/operator-console/screens/OperatorSummaryScreen.tsx]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- sprint status, preview track sprint plan, epics, PRD, architecture, UX, legacy 1.19/1.20 story context, release baseline, hardware ledger, operator diagnostics/schema/UI 현황을 교차 분석했다.
- 세 후보 중 첫 구현 스토리는 문서상 권장 순서와 dependency 기준 모두 `metric reset`이 맞다고 판단했다.
- Story 1.22와 1.23 범위를 침범하지 않도록 이번 스토리를 `metric/acceptance alignment + contract/UI/governance reset`으로 한정했다.

### Debug Log References

- `Get-Content _bmad/bmm/config.yaml`
- `Get-Content _bmad-output/implementation-artifacts/sprint-status.yaml`
- `rg -n "1\\.21|1\\.22|1\\.23|metric reset|full-screen 2500ms" _bmad-output/planning-artifacts/epics.md _bmad-output/planning-artifacts/prd.md _bmad-output/planning-artifacts/architecture.md _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md`
- `Get-Content _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md`
- `Get-Content _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md`
- `rg -n "replacementMs|firstVisibleMs|sameCaptureFullScreenVisibleMs" docs/release-baseline.md release-baseline.md _bmad-output/implementation-artifacts/hardware-validation-ledger.md src/shared-contracts/schemas/*.ts src/operator-console/screens/OperatorSummaryScreen.tsx`
- `pnpm test:run src/shared-contracts/contracts.test.ts src/operator-console/screens/OperatorSummaryScreen.test.tsx src/operator-console/services/operator-diagnostics-service.test.ts src/governance/hardware-validation-governance.test.ts tests/hardware-evidence-scripts.test.ts`
- `pnpm test:run`
- `pnpm lint`

### Completion Notes List

- release baseline, promotion evidence package, hardware ledger wording을 `same-capture preset-applied full-screen visible <= 2500ms` 기준으로 통일했다.
- `sameCaptureFullScreenVisibleMs`를 canonical release field로 승격하고, legacy `replacementMs`는 backward-compatible alias로 유지했다.
- operator summary 화면의 primary KPI 라벨과 목표 시간을 full-screen 2500ms 기준으로 정렬했다.
- preview promotion evidence bundle과 operator-safe projection이 새 canonical field를 함께 내보내도록 맞췄다.
- governance, contract, operator console, evidence script 회귀 검증을 추가했고 `pnpm test:run`, `pnpm lint`를 통과했다.
- sprint status에서 Story 1.21을 `done`으로 반영했다.

### File List

- _bmad-output/implementation-artifacts/1-21-metric-reset과-full-screen-2500ms-acceptance-정렬.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/release-baseline.md
- release-baseline.md
- docs/runbooks/preview-promotion-evidence-package.md
- scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1
- src/app/providers/app-providers.tsx
- src/governance/hardware-validation-governance.test.ts
- src/operator-console/screens/OperatorSummaryScreen.test.tsx
- src/operator-console/screens/OperatorSummaryScreen.tsx
- src/operator-console/services/operator-diagnostics-service.test.ts
- src/preset-authoring/providers/use-preset-authoring-service.ts
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/hardware-validation.ts
- src/shared-contracts/schemas/operator-diagnostics.ts
- src-tauri/src/branch_config/mod.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/diagnostics/mod.rs
- src-tauri/src/render/dedicated_renderer.rs
- tests/fixtures/contracts/preview-promotion-evidence-record-v1.json
- tests/hardware-evidence-scripts.test.ts
