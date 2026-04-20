---
workflow: correct-course
project: Boothy
date: 2026-04-19 12:12:17 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-19 12:13:02 +09:00
approval_decision: yes
trigger_reference: docs/runbooks/current-preview-gpu-direction-20260419.md
---

# Sprint Change Proposal - preview-track GPU 방향과 canonical 문서 정렬

## 0. 워크플로우 프레이밍

- 이번 correct-course는 `docs/runbooks/current-preview-gpu-direction-20260419.md`와 `docs/README.md`를 기준으로 현재 preview-track 방향을 다시 planning artifact에 맞추는 작업이다.
- 사용자 승인에 따라 `Batch` 모드로 진행했다.
- `project-context.md`는 찾지 못해 별도 프로젝트 컨텍스트 파일은 없는 것으로 처리했다.
- 검토한 핵심 문서:
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `docs/README.md`
  - `docs/runbooks/current-preview-gpu-direction-20260419.md`
  - `docs/runbooks/current-actual-lane-handoff-20260419.md`
  - `docs/runbooks/preview-track-route-decision-20260418.md`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거: Story 1.10 `known-good preview lane 복구와 상주형 first-visible worker 도입`
  - 보조 트리거: current preview-track runbooks와 non-BMAD canonical doc map
- [x] 1.2 Core problem defined
  - 이슈 유형: 실패한 접근 이후 다음 방향을 더 명확히 고정해야 하는 corrective planning change
  - 문제 진술: 현재 BMAD epics/story artifact는 old `resident first-visible` line 재검증과 GPU 가설을 어느 정도 담고 있지만, `CPU baseline 재닫기 -> 같은 lane의 GPU 비교 -> 부족하면 native/GPU resident lane 검토`라는 현재 실행 순서와 canonical 문서 진입점이 planning artifact에 충분히 명시돼 있지 않다.
- [x] 1.3 Evidence gathered
  - newer `actual-primary-lane`은 bounded `No-Go`로 정리돼 있다.
  - 현재 worktree는 old `resident first-visible` line revalidation lane으로 해석된다.
  - GPU는 release shortcut이 아니라 validation hypothesis로 정리돼 있다.
  - non-BMAD 문서가 실제 current direction의 더 정확한 진입점이 되었지만, BMAD artifact는 이 읽기 순서를 명확히 안내하지 않는다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 유지 가능하다.
  - Story 1.10의 역할 해석만 더 명확히 고정하면 된다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필요 없다.
  - Epic 1 Story 1.10 설명과 운영 해석을 보정하면 충분하다.
- [x] 2.3 Remaining epics reviewed
  - 직접 영향은 Epic 1이 가장 크다.
  - 다른 epics는 preview-track canonical 해석을 잘못 끌어오지 않도록 sprint status와 architecture note만 맞추면 된다.
- [x] 2.4 Future epic invalidation checked
  - 무효화되는 epic은 없다.
  - MVP 축소나 제품 방향 변경도 필요 없다.
- [x] 2.5 Epic priority/order checked
  - Story 1.10은 여전히 현재 corrective focus다.
  - 다만 active implementation restart가 아니라 validation lane 해석이 우선이라는 점을 더 분명히 해야 한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD는 이미 `old lane은 validation candidate`, `GPU는 hypothesis`, `dual gate 유지`를 담고 있어 추가 수정이 필수는 아니다.
- [x] 3.2 Architecture conflict reviewed
  - architecture 문서는 구조적으로 유효하지만, current preview-track 해석은 newer runbook과 canonical doc map을 먼저 읽어야 한다는 안내가 더 필요하다.
- [x] 3.3 UX impact reviewed
  - UX 변경은 필요 없다.
  - booth customer promise는 그대로 유지된다.
- [x] 3.4 Other artifacts reviewed
  - `epics.md`, Story 1.10 implementation artifact, `sprint-status.yaml`은 current direction과 canonical doc entrypoint를 더 명확히 적어야 한다.

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Low
  - Risk: Low
  - 기존 epic/story 구조를 유지한 채 해석과 실행 순서만 보정한다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
  - 최근 planning artifact를 되돌릴 이유는 없고, 오히려 현재 해석을 흐리게 만든다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Medium
  - 이번 이슈는 MVP 재정의가 아니라 current technical direction 고정 문제다.
- [x] 4.4 Recommended path selected
  - 선택안: Option 1 `Direct Adjustment`
  - 이유: 제품 방향은 유지하고, BMAD artifact의 해석 순서와 current route execution order만 정밀하게 맞추는 편이 가장 안전하다.

### 5. Proposal Components

- [x] 5.1 Issue summary prepared
- [x] 5.2 Epic and artifact impact summarized
- [x] 5.3 Recommended path documented
- [x] 5.4 MVP impact and action plan defined
- [x] 5.5 Handoff plan defined

### 6. Final Review and Handoff

- [x] 6.1 Checklist completion reviewed
- [x] 6.2 Proposal consistency reviewed
- [x] 6.3 User approval obtained
  - 승인 일시: 2026-04-19 12:13:02 +09:00
- [x] 6.4 `sprint-status.yaml` update completed
  - preview-track canonical entrypoint와 next validation sequence를 반영했다.
- [x] 6.5 Final handoff confirmation recorded
  - planning artifact와 non-BMAD canonical docs가 같은 해석을 가리키도록 정렬했다.

## 2. 이슈 요약

### Problem Statement

현재 preview-track의 실제 방향은 이미 많이 정리돼 있다. 문제는 방향 자체보다, 어디를 먼저 읽어야 하고 Story 1.10을 지금 어떻게 해석해야 하는지가 BMAD artifact에서 충분히 고정돼 있지 않다는 점이다.

### Discovery Context

- `docs/README.md`와 `docs/runbooks/current-preview-gpu-direction-20260419.md`를 만들면서 non-BMAD canonical entrypoint가 명확해졌다.
- 동시에 BMAD planning artifact 쪽은 여전히 current route execution order를 충분히 직접 적지 않아 해석 drift 가능성이 남았다.
- 이번 correct-course의 목적은 기술 방향 변경이 아니라 planning artifact와 current canonical docs를 다시 맞추는 것이다.

## 3. 영향 분석

### Epic 영향

- Epic 1 영향
  - Story 1.10의 현재 역할을 `validation candidate spec / revalidation context`로 더 분명히 적어야 한다.
  - next route sequence를 story 해석 안에 직접 넣어야 한다.
- Epic 2~6 영향
  - 직접 story 변경은 불필요하다.
  - sprint-level current route interpretation만 일관되게 유지하면 된다.

### MVP 영향

- MVP 축소는 필요 없다.
- booth customer 경험 목표도 변하지 않는다.
- 이번 변경은 planning clarity와 execution order를 고정하는 문서 보정이다.

### 리스크

- 변경을 하지 않을 경우
  - 에이전트가 오래된 BMAD artifact를 active execution plan처럼 읽을 수 있다.
  - GPU를 `바로 켜면 되는 카드`로 오해할 수 있다.
  - non-BMAD canonical docs와 BMAD artifact가 따로 노는 상태가 유지된다.
- 변경을 할 경우
  - 문서 해석 비용이 줄고, 다음 planning/implementation handoff가 더 단순해진다.
  - 기존 code/history trace는 유지한 채 현재 방향만 더 명확해진다.

## 4. 권장 접근

### Chosen Path

`Direct Adjustment`

### Why This Path

- PRD와 architecture의 큰 방향은 이미 틀리지 않았다.
- 실제 문제는 Epic 1 Story 1.10과 sprint status에서 current interpretation이 충분히 전면화되지 않은 점이다.
- 따라서 기존 BMAD artifact를 폐기하거나 재생성하는 것보다, current direction과 canonical doc order를 명시적으로 추가하는 편이 가장 안전하다.

### Scope Classification

`Moderate`

### Timeline Impact

- 구현 일정 자체를 새로 열 필요는 없다.
- 다음 planning handoff에서 적용할 순서는 아래와 같다.
  1. non-BMAD canonical docs를 current direction entrypoint로 유지
  2. Story 1.10을 validation candidate로 읽도록 BMAD artifact 보정
  3. CPU baseline package를 먼저 닫고
  4. 같은 lane의 GPU/OpenCL matched comparison을 열고
  5. 부족하면 native/GPU resident lane 검토로 넘어간다

## 5. 상세 변경 제안

### A. Epic Proposal

**Artifact:** `_bmad-output/planning-artifacts/epics.md`

OLD:
- Story 1.10은 resident worker corrective follow-up으로는 정리돼 있지만, current worktree에서의 읽기 방식과 실행 순서를 직접 고정하지 않는다.

NEW:
- Story 1.10 아래에 current validation role note를 추가한다.
- note에는 아래를 직접 적는다.
  - 현재 worktree는 active implementation restart가 아니라 revalidation lane이다.
  - next execution order는 `CPU baseline package -> GPU/OpenCL matched comparison -> if insufficient, native/GPU resident lane evaluation`이다.
  - canonical current direction은 `docs/README.md`와 `docs/runbooks/current-preview-gpu-direction-20260419.md`를 먼저 읽어 해석한다.

### B. Implementation Story Proposal

**Artifact:** `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`

OLD:
- current role과 GPU hypothesis는 들어 있지만, canonical reading order와 next execution sequence는 약하게 드러난다.

NEW:
- `Current Execution Order`와 `Canonical Reading Order`를 별도 섹션으로 추가한다.
- story 문서만 단독으로 읽지 말고 non-BMAD canonical docs를 먼저 읽도록 명시한다.

### C. Sprint Tracking Proposal

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

OLD:
- current route status는 적혀 있지만, canonical doc entrypoint와 next validation sequence는 없다.

NEW:
- `canonical_direction_docs`를 추가한다.
- `next_validation_sequence`를 추가한다.

### D. Architecture Proposal

**Artifact:** `_bmad-output/planning-artifacts/architecture.md`

OLD:
- architecture 문서 alone으로는 current preview-track phase interpretation을 표현하지 못한다고만 적혀 있다.

NEW:
- `docs/README.md`와 `docs/runbooks/current-preview-gpu-direction-20260419.md`를 먼저 읽어야 한다는 explicit note를 추가한다.

## 6. 구현 인계

### Scope

`Moderate`

### Handoff

- Product / Scrum 문서 정렬
  - BMAD planning artifact가 non-BMAD canonical docs와 같은 방향을 보도록 유지
- Architecture ownership
  - GPU를 validation hypothesis로 유지하고, next lane promotion 기준을 문서로 계속 잠금
- Development ownership
  - Story 1.10을 active restart로 오해하지 않고, CPU baseline package 이후에만 GPU comparison을 열 것

### Success Criteria

- 새 에이전트가 `docs/README.md`부터 읽어 current direction을 해석할 수 있다.
- BMAD artifact만 읽어도 Story 1.10을 validation lane 문맥으로 이해할 수 있다.
- sprint status만 열어도 next validation sequence를 오해하지 않는다.
