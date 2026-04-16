# Sprint Change Proposal

Date: 2026-04-16 11:13 +09:00
Project: Boothy
Mode: Batch
Prepared by: Codex (`bmad-correct-course`)

## 1. Issue Summary

### Trigger

- 기준 문서: `_bmad-output/planning-artifacts/implementation-readiness-report-20260415.md`
- 핵심 판정: PRD / Architecture / Epics / UX 정합성은 양호하고 FR coverage는 100%이지만, readiness는 `NEEDS WORK`
- 직접 원인:
  - 실행 순서와 스토리 번호/상태 표현이 어긋남
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`의 `recommended_next_story`가 아직 `1.25`로 남아 있음
  - Story `1.25`는 이미 구현 산출물상 `done`으로 기록되어 있음
  - Story `1.13`은 문서상으로는 `1.25` 이후에만 시작 가능하지만, 제품 게이트 관점에서는 아직 시작 가능 상태가 아님
  - Story `1.26`은 reserve track인데, 현 상태를 잘못 읽으면 조기 개시로 오해될 수 있음

### Core Problem Statement

현재 계획 문서는 구조적으로 맞지만, 실행 추적 문서가 `구현 완료(done)`와 `제품 게이트 통과(Go candidate / release-close 진입 가능)`를 충분히 분리하지 못하고 있다. 그 결과 실제 제품 상태는 아직 `release hold / No-Go`에 가까운데, 스프린트 추적상 다음 작업이 잘못 추천되고 있다.

### Evidence

- readiness 보고서는 Story `1.13`이 Story `1.25` 이후에만 가능하다고 명시하고, active order를 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25 -> 1.13`으로 고정한다.
- `epics.md`는 Story `1.13`을 final guarded cutover / release-close owner로, Story `1.26`을 repeated KPI failure 때만 열리는 reserve track으로 정의한다.
- `release-baseline.md`와 `docs/release-baseline.md`는 2026-04-15 기준 preview architecture가 여전히 `release hold`이며, canonical ledger가 `No-Go`라고 명시한다.
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`는 Story `1.13`의 canonical row를 여전히 `No-Go`로 기록한다.
- Story `1.23`, `1.24`, `1.25` 구현 문서는 모두 `Status: done`이지만, 각 문서 자체가 final release-close ownership은 Story `1.13`에 남겨 둔다.

## 2. Impact Analysis

### Epic Impact

- Epic 1의 구조 자체는 유지 가능하다.
- 다만 Epic 1 preview track의 `실행 가능 상태`와 `제품 게이트 상태`를 더 명시적으로 분리해야 한다.
- 새 Epic 추가, Epic 삭제, MVP 범위 축소는 필요 없다.

### Story Impact

- Story `1.13`
  - 번호상 다음이 아니라, `local-lane Go candidate + rollback evidence accepted` 이후에만 재개되는 final close owner로 다시 못박아야 한다.
- Story `1.23`
  - 구현 스토리로 `done`일 수는 있으나, 제품 KPI 증명 완료로 읽히지 않게 해야 한다.
- Story `1.24`
  - canary gate implementation 완료와 canonical `Go` acceptance를 분리해서 표현해야 한다.
- Story `1.25`
  - default/rollback gate implementation 완료와 `1.13` 재개 승인 여부를 분리해서 표현해야 한다.
- Story `1.26`
  - 현 시점에서 열지 않는 reserve track임을 더 명시해야 한다.

### Artifact Conflict Assessment

- PRD: 변경 불필요
  - KPI와 release truth 기준은 이미 적절히 정의됨
- Architecture: 변경 불필요
  - local lane, canary, default/rollback, final close, reserve track의 ownership이 이미 정리됨
- UX: 변경 불필요
  - 본 이슈는 UX 계약이 아니라 실행 추적/스프린트 운영 문제임
- Epics: 소규모 명시 강화 필요
  - Story `1.13` 시작 조건과 Story `1.26` 개시 조건을 status interpretation까지 포함해 더 분명히 해야 함
- Sprint Status: 즉시 보정 필요
  - `recommended_next_story`와 preview track gate 상태 표현이 현실과 어긋남
- Hardware Ledger: 변경 불필요
  - canonical `No-Go` 근거 문서로서 이미 현실을 반영 중

### Technical / Operational Impact

- 코드 구현 자체를 롤백할 필요는 없다.
- 현재 필요한 작업은 계획/트래킹 보정이다.
- 잘못된 `next story` 추천을 유지하면 BMAD 다음 자동 흐름이 다시 잘못 열릴 수 있다.

## 3. Checklist Result

### Section 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - Story `1.13`, `1.23`, `1.25`, `1.26` 및 `sprint-status.yaml`
- [x] 1.2 Core problem defined
  - 구현 완료와 release gate clearance가 추적 문서에서 혼선
- [x] 1.3 Supporting evidence gathered

### Section 2. Epic Impact Assessment

- [x] 2.1 Current epic can continue with adjustments
- [x] 2.2 Epic-level change needed
  - Epic 1 sequencing/interpretation note 강화
- [x] 2.3 Remaining epics reviewed
  - Epic 2~6 직접 영향 없음
- [N/A] 2.4 New epic required
- [x] 2.5 Order/priority review completed
  - 실행 순서 자체는 유지, 표현만 보정

### Section 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD checked
  - conflict 없음
- [x] 3.2 Architecture checked
  - conflict 없음
- [x] 3.3 UX checked
  - conflict 없음
- [x] 3.4 Other artifacts checked
  - sprint status / hardware ledger / release baseline 확인 완료

### Section 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment: Viable
  - Effort: Low
  - Risk: Low
- [x] 4.2 Option 2 Potential Rollback: Not viable
  - Effort: Medium
  - Risk: Medium
  - Reason: 코드 롤백 문제가 아니라 운영/추적 표현 문제임
- [x] 4.3 Option 3 PRD MVP Review: Not viable
  - Effort: High
  - Risk: Unnecessary
  - Reason: MVP/요구사항 자체는 유효함
- [x] 4.4 Recommended path selected
  - Selected approach: Option 1 Direct Adjustment

### Section 5. Proposal Components

- [x] 5.1 Issue summary created
- [x] 5.2 Epic impact and artifact adjustment documented
- [x] 5.3 Recommended path documented
- [x] 5.4 MVP impact and action plan defined
- [x] 5.5 Handoff plan defined

### Section 6. Final Review and Handoff

- [x] 6.1 Checklist reviewed
- [x] 6.2 Proposal accuracy reviewed
- [!] 6.3 User approval pending
- [!] 6.4 sprint-status.yaml update pending approval
- [!] 6.5 Final handoff pending approval

## 4. Recommended Approach

### Chosen Path

Direct Adjustment

### Rationale

- 요구사항 변경이 아니라 실행 추적 보정이다.
- PRD / Architecture / UX / Epic 구조는 유지한 채, BMAD 운영 산출물만 바로잡으면 된다.
- 구현 완료 상태와 제품 게이트 상태를 분리해 기록하면 이후 자동 추천과 스토리 오픈 판단이 안정된다.

### MVP Impact

- MVP 범위 변화 없음
- 제품 KPI 변화 없음
- 릴리스 기준 변화 없음
- 해석만 수정: `done != release-close ready`

### Timeline / Risk

- Effort: Low
- Scope: Moderate
  - 이유: 구현 롤백은 아니지만 backlog / sprint tracking 보정이 필요함
- Primary risk if skipped:
  - 잘못된 next story 추천
  - Story `1.13` 조기 개시 오해
  - Story `1.26` 조기 개시 오해

## 5. Detailed Change Proposals

### A. Epics Clarification

Artifact: `_bmad-output/planning-artifacts/epics.md`

#### Proposal EPIC-A1: Story 1.13 시작 조건을 "스토리 완료 여부"가 아니라 "제품 게이트 승인 여부"로 명시 강화

OLD:

```md
- Story 1.13은 Story 1.25가 local lane `Go` 후보를 만든 뒤에만 수행되는 final guarded cutover / release-close owner다.
```

NEW:

```md
- Story 1.13은 Story 1.25가 구현상 `done`인지만으로 열리지 않는다.
- Story 1.25 산출물이 canonical hardware evidence와 route-policy review에서 실제 local lane `Go` 후보로 인정된 뒤에만 수행되는 final guarded cutover / release-close owner다.
```

Rationale:

- 현재 핵심 문제는 `1.25 done`과 `1.13 startable`이 동일하게 해석되는 점이다.
- Epic 문서가 이 해석 오차를 막아야 한다.

#### Proposal EPIC-A2: Story 1.26 reserve track 개시 조건을 더 명시

OLD:

```md
- Story 1.26은 local lane이 승인 하드웨어에서 같은 KPI를 반복 실패할 때만 열리는 reserve experiment다.
```

NEW:

```md
- Story 1.26은 Story 1.13이 아직 blocked라는 이유만으로 열리지 않는다.
- Story 1.26은 local forward path의 canonical evidence가 approved hardware KPI 반복 실패를 보여 줄 때만 열리는 reserve experiment다.
```

Rationale:

- `1.13 blocked`와 `1.26 open`은 동의어가 아니다.
- reserve path는 실패 증거 기반으로만 열려야 한다.

### B. Sprint Status Normalization

Artifact: `_bmad-output/implementation-artifacts/sprint-status.yaml`

#### Proposal STATUS-B1: `recommended_next_story`를 현실에 맞게 수정

OLD:

```yaml
recommended_next_story: "1.25-local-lane-default-decision과-rollback-gate"
```

NEW:

```yaml
recommended_next_story: "none-release-hold-local-lane-go-candidate-not-yet-proven"
recommended_next_action: "Reconcile preview-track product gate and reopen Story 1.13 only after canonical local-lane Go candidate evidence is accepted."
```

Rationale:

- `1.25`는 이미 구현상 `done`이다.
- `1.13`은 아직 blocked다.
- `1.26`은 아직 open condition을 충족하지 않았다.
- 따라서 지금은 `next executable story`가 아니라 `release-hold 상태 정리`가 현실에 가깝다.

#### Proposal STATUS-B2: preview track에 제품 게이트 상태 축 추가

OLD:

```yaml
preview_architecture_tracks:
  old_track:
    - "1.18 retired dedicated close baseline evidence"
    - "1.19 legacy parity and instrumentation ledger"
    - "1.20 legacy route activation validation"
  new_track:
    - "1.21 metric reset and acceptance alignment"
    - "1.22 capture-to-full-screen evidence chain reset"
    - "1.23 local full-screen lane prototype"
    - "1.24 local lane hardware canary validation"
    - "1.25 local lane default decision and rollback gate"
  final_close_owner: "1.13 guarded cutover and hardware validation gate"
  reserve_track: "1.26 remote reserve POC only after repeated local-lane KPI failure"
```

NEW:

```yaml
preview_architecture_tracks:
  old_track:
    - "1.18 retired dedicated close baseline evidence"
    - "1.19 legacy parity and instrumentation ledger"
    - "1.20 legacy route activation validation"
  new_track:
    - "1.21 metric reset and acceptance alignment"
    - "1.22 capture-to-full-screen evidence chain reset"
    - "1.23 local full-screen lane prototype"
    - "1.24 local lane hardware canary validation"
    - "1.25 local lane default decision and rollback gate"
  final_close_owner: "1.13 guarded cutover and hardware validation gate"
  reserve_track: "1.26 remote reserve POC only after repeated local-lane KPI failure"
  product_gate_state:
    overall: "release-hold"
    canonical_local_lane_go_candidate: "not-yet-proven"
    story_1_13: "blocked"
    story_1_26: "closed-until-repeated-local-kpi-failure"
```

Rationale:

- 현재 sprint status는 실행 순서는 적지만 제품 게이트 상태가 부족하다.
- 이 축이 있어야 `done`과 `startable`을 분리할 수 있다.

#### Proposal STATUS-B3: Story 1.23 해석 보정 메모 추가

OLD:

```yaml
  1-23-local-full-screen-lane-prototype과-truthful-artifact-generation: done
```

NEW:

```yaml
  1-23-local-full-screen-lane-prototype과-truthful-artifact-generation: done
story_gate_notes:
  1-23: "Implementation done. Product KPI proof is not closed by this story alone."
  1-24: "Canary gate implementation exists. Canonical Go acceptance is still tracked separately."
  1-25: "Default/rollback gate implementation exists. This does not by itself reopen Story 1.13."
  1-13: "Blocked until canonical local-lane Go candidate and rollback evidence are accepted."
  1-26: "Do not open unless repeated approved-hardware KPI failure is confirmed."
```

Rationale:

- `1.23`를 backlog나 review로 내리는 것보다, 구현 완료와 제품 게이트 미통과를 분리 기록하는 편이 정확하다.
- 사용자가 요청한 "문서상 done이어도 제품적으로는 아직 KPI 증명이 안 됐다"는 의도를 가장 정확하게 반영한다.

### C. PRD / Architecture / UX

Artifacts:

- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`

Proposal:

- No change

Rationale:

- 세 문서 모두 이번 이슈의 원인이 아니다.
- 이미 current product truth와 gate ownership을 적절히 설명하고 있다.

## 6. Implementation Handoff

### Scope Classification

Moderate

### Why Moderate

- 코드 구현 변경은 아니지만, backlog / sprint tracking / artifact interpretation을 조정해야 한다.
- 잘못 수정하면 BMAD 다음 흐름이 다시 어긋날 수 있다.

### Handoff Recipients

- Product Owner / Scrum Master
  - epics wording 보정
  - sprint status normalization 승인
- Development Team
  - 없음, 승인 후 planning/status artifact edit만 수행

### Success Criteria

- `sprint-status.yaml`이 더 이상 `1.25`를 next story로 추천하지 않는다.
- `1.13 blocked`가 story 번호가 아니라 product gate 기준으로 해석된다.
- `1.26`이 reserve-only 상태로 명확히 유지된다.
- `1.23 done`이 `KPI proof complete`로 오해되지 않는다.

## 7. Approval Outcome

Approval: Yes

Approved by user on: 2026-04-16

Approved change summary:

- PRD / Architecture / UX는 그대로 둔다.
- `epics.md`는 Story `1.13`, `1.26`의 해석을 더 명시적으로 보강한다.
- `sprint-status.yaml`은 `recommended_next_story`를 현실에 맞게 바꾸고, preview track의 제품 게이트 상태를 별도 축으로 추가한다.

Applied artifacts:

1. `_bmad-output/planning-artifacts/epics.md`
2. `_bmad-output/implementation-artifacts/sprint-status.yaml`

## 8. Handoff Completion

- Change scope: Moderate
- Routed to: Product Owner / Scrum Master artifact maintenance path
- Implementation route: planning/status artifact correction only
- Success condition reached:
  - preview track blocking semantics clarified
  - `recommended_next_story` no longer points at already-done Story `1.25`
  - Story `1.13` and Story `1.26` gate interpretation separated from simple numbering/status
