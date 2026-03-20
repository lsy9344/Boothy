# 스프린트 변경 제안서

- 프로젝트: Boothy
- 작성일시: 2026-03-20 02:48:05
- 변경 트리거: 구현 준비도 평가 결과 `NEEDS WORK`
- 진행 모드: Batch
- 작성 근거:
  - `_bmad-output/planning-artifacts/implementation-readiness-report-20260320.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`

## 1. 이슈 요약

현재 계획 산출물은 FR 커버리지와 PRD/UX/아키텍처의 큰 방향 정합성은 확보했지만, 구현 시작 전 필요한 운영 경계와 실패 경로 정의가 충분히 닫히지 않아 구현 준비 상태가 `NEEDS WORK`로 평가되었다.

이번 변경의 핵심 문제는 다음 네 가지다.

1. Epic 5가 `운영자 복구`와 `지점 배포/롤백 거버넌스`라는 서로 다른 가치 흐름을 한 에픽에 묶어 응집도가 낮다.
2. Story 2.3, 4.3, 5.4가 모두 고위험 시나리오인데도 실패/거절 경로 acceptance criteria가 부족하다.
3. 일부 고위험 스토리에 PRD의 핵심 NFR 경계가 충분히 재기록되지 않아 구현자가 임계 동작을 해석으로 메워야 할 가능성이 있다.
4. 아키텍처 문서에 현재 승인된 입력 집합과 맞지 않는 오래된 참조(`PRD rewrite brief`, `Epic 4-8`)가 남아 있어 후속 구현 지시가 흔들릴 수 있다.

이 제안서는 코드 구현 방향을 바꾸기 위한 것이 아니라, 구현 전에 계획 산출물을 정정해 해석 여지를 줄이기 위한 변경이다.

## 2. 영향 분석

### 에픽 영향

- 현재 Epic 5는 서로 다른 주체를 가진 두 운영 가치 흐름을 혼합하고 있다.
  - 운영자 복구와 감사 로그: 주체가 `remote operator`
  - 지점 배포/롤백 거버넌스: 주체가 `owner / brand operator`
- 이 상태를 유지하면 운영자 복구 구현이 배포 거버넌스 결정과 묶여 스프린트 순서와 검증 범위가 불필요하게 커질 가능성이 높다.
- 따라서 에픽 재구성이 필요하다.

### 스토리 영향

- Story 2.3은 프리셋 전환 실패 시 이전 바인딩 유지 규칙이 없다.
- Story 4.3은 게시 거절 조건과 게시 실패 후 상태 보존 규칙이 없다.
- Story 5.4는 활성 세션 보호, 롤백 자격 실패, 거절 시 감사 로그 규칙이 없다.
- Story 1.5, 2.1, 2.4, 3.2, 5.4는 PRD의 성능/격리/타이밍/배포 안전 기준을 더 직접적으로 받아야 한다.

### 아티팩트 충돌

- PRD: 직접 충돌 없음. 현재 PRD는 이미 필요한 제품 경계와 릴리스 게이트를 정의하고 있다.
- UX: 직접 충돌 없음. 현재 UX는 제품 경계를 유지하고 있으며 이번 변경은 구현 준비도 보강 성격이다.
- Architecture: 문서 위생 정리가 필요하다.
  - 현재 승인 입력에 없는 `PRD rewrite brief` 참조 제거 또는 정정 필요
  - `Epic 4-8` 기준 문구를 현재 에픽 구조에 맞게 정정 필요

### 기술 영향

- 코드 구현 전에 story 계약을 먼저 보강하면 운영자 복구, 게시 불변성, 배포 거부 규칙에서 테스트 가능한 실패 경계가 생긴다.
- 특히 `active session protection`, `future-session-only publication`, `previous preset binding preservation`는 구현 시 해석 차이를 막는 핵심 경계다.

## 3. 권장 접근 방식

### 옵션 평가

- Option 1: Direct Adjustment
  - 실행 가능성: 높음
  - 노력: Medium
  - 리스크: Low
  - 판단: 계획 문서 수정만으로 해결 가능하며 MVP 자체를 흔들지 않는다.

- Option 2: Potential Rollback
  - 실행 가능성: 낮음
  - 노력: High
  - 리스크: Medium
  - 판단: 이미 확보한 PRD/UX/Architecture 정합성을 되돌릴 이유가 없다.

- Option 3: PRD MVP Review
  - 실행 가능성: 낮음
  - 노력: High
  - 리스크: High
  - 판단: 현재 문제는 MVP 범위 정의가 아니라 에픽/스토리 구현 계약의 부족이다.

### 권장안

`Direct Adjustment`를 권장한다.

이 변경은 MVP 축소나 전략 피벗이 아니라, 구현 전 backlog 구조와 고위험 acceptance criteria를 보강하는 계획 정정이다. 가장 안전한 경로는 PRD는 유지하고, `epics.md`와 `architecture.md`를 중심으로 정리한 뒤 구현 준비도 평가를 재실행하는 것이다.

### 일정/리스크 평가

- 일정 영향: 낮음에서 중간
- 구현 리스크 감소 효과: 높음
- 팀 모멘텀 영향: 긍정적
- 유지보수성 영향: 긍정적

## 4. 상세 변경 제안

### 4.1 Stories / Epics

#### 변경 제안 A: Epic 5 분리

섹션: `epics.md` / Epic 정의 및 FR Coverage Map

OLD:

```md
### Epic 5: 운영자 복구, 감사 로그, 지점 배포 거버넌스
운영자가 안전한 범위에서 문제를 진단·복구하고, 제품이 지점별 일관성과 단계적 배포/롤백 규칙을 지키며 운영될 수 있게 한다.
```

NEW:

```md
### Epic 5: 운영자 복구와 감사 로그
운영자가 안전한 범위에서 현재 세션을 진단·복구하고, 개입 및 결과를 감사 가능하게 남길 수 있게 한다.
**Primary coverage:** FR9

### Epic 6: 지점 배포와 롤백 거버넌스
Owner / brand operator가 선택된 지점 집합에 대해 빌드와 승인된 프리셋 스택을 안전하게 배포·롤백할 수 있게 한다.
**Primary coverage:** NFR-002, NFR-006, rollout/rollback additional requirements
```

스토리 재배치:

```md
OLD: Story 5.4: 지점별 단계적 배포와 단일 액션 롤백 거버넌스
NEW: Story 6.1: 지점별 단계적 배포와 단일 액션 롤백 거버넌스
```

근거:

- 구현 준비도 보고서가 지적한 에픽 응집도 문제를 직접 해소한다.
- 아키텍처의 `release-governance work as separate downstream epic tracks` 방향과 일치한다.
- 운영자 복구 구현과 릴리스 거버넌스 구현의 순서를 독립적으로 잡을 수 있다.

#### 변경 제안 B: Story 2.3 실패 경로 보강

섹션: `epics.md` / Story 2.3 Acceptance Criteria

OLD:

```md
**Given** an active session with an already selected preset
**When** the customer chooses a different approved published preset
**Then** the new preset becomes the active preset for future captures only
**And** previously captured session assets remain bound to the preset version used at capture time

**Given** the customer is reviewing existing captures
**When** the active preset changes
**Then** the UI clearly indicates the newly active preset for upcoming captures
**And** it does not imply that prior captures were re-edited or re-bound
```

NEW:

```md
**Given** an active session with an already selected preset
**When** the customer chooses a different approved published preset
**Then** the new preset becomes the active preset for future captures only
**And** previously captured session assets remain bound to the preset version used at capture time

**Given** the customer is reviewing existing captures
**When** the active preset changes
**Then** the UI clearly indicates the newly active preset for upcoming captures
**And** it does not imply that prior captures were re-edited or re-bound

**Given** the customer requests a preset switch
**When** the selected preset is no longer available or the preset binding cannot be applied safely
**Then** the previously active preset remains the active preset for future captures
**And** the customer sees plain-language guidance to keep the current preset or choose another approved preset

**Given** a preset switch succeeds
**When** the booth acknowledges the change
**Then** the active-preset confirmation is acknowledged within 1 second on approved hardware
**And** no current-session asset outside future captures is mutated by the switch
```

근거:

- 실패 시 이전 바인딩 유지가 없으면 FR5와 session truth 경계가 흔들린다.
- PRD의 NFR-003, NFR-004와 현재 세션/미래 촬영 분리 규칙을 story 수준으로 닫는다.

#### 변경 제안 C: Story 4.3 게시 거절 경로 보강

섹션: `epics.md` / Story 4.3 Acceptance Criteria

OLD:

```md
**Given** a preset version is in the `validated` state
**When** an authorized approver approves and publishes it
**Then** the system creates an immutable published preset artifact bundle with stable identity, version, and catalog metadata
**And** the preset lifecycle advances through `approved` to `published`

**Given** a preset has been published
**When** future sessions load the booth catalog
**Then** the published preset can appear as a selectable catalog item
**And** active sessions are not mutated by the publication event
```

NEW:

```md
**Given** a preset version is in the `validated` state
**When** an authorized approver approves and publishes it
**Then** the system creates an immutable published preset artifact bundle with stable identity, version, and catalog metadata
**And** the preset lifecycle advances through `approved` to `published`

**Given** a preset has been published
**When** future sessions load the booth catalog
**Then** the published preset can appear as a selectable catalog item
**And** active sessions are not mutated by the publication event

**Given** a publication request is attempted
**When** validation is stale, required artifact metadata is incompatible, or publication would violate immutability or future-session-only rules
**Then** publication is rejected
**And** no `published` artifact is created
**And** the preset remains in its prior lifecycle state
**And** the authorized user sees actionable rejection guidance

**Given** a publication request is rejected
**When** the rejection is finalized
**Then** the system records the rejected action, reason, actor, and timestamp in the audit history
**And** the booth catalog and active sessions remain unchanged
```

근거:

- FR8은 성공 경로만으로는 enforce되지 않는다.
- PRD release gate의 `future sessions without mutating active sessions`를 구현 가능한 계약으로 내린다.

#### 변경 제안 D: Story 6.1(구 5.4) 안전/거절 경로 보강

섹션: `epics.md` / Story 6.1 Acceptance Criteria

OLD:

```md
**Given** a new approved build or preset stack is ready
**When** a rollout is initiated
**Then** the system targets an explicitly selected branch set rather than all branches at once
**And** the rollout records the branch set, target build, approved preset stack, approval timestamp, and actor

**Given** a promoted branch must be reverted
**When** rollback is triggered
**Then** the branch returns to the last approved build and approved preset stack in one approved rollback action
**And** no active customer session is interrupted by forced update behavior
```

NEW:

```md
**Given** a new approved build or preset stack is ready
**When** a rollout is initiated
**Then** the system targets an explicitly selected branch set rather than all branches at once
**And** the rollout records the branch set, target build, approved preset stack, approval timestamp, and actor

**Given** any targeted branch has an active customer session
**When** rollout would interrupt that session
**Then** the system defers or rejects rollout for that branch
**And** no forced update is applied to the active session
**And** the refusal or deferral reason is surfaced to the initiating operator and recorded in audit history

**Given** a promoted branch must be reverted
**When** rollback is triggered
**Then** the branch returns to the last approved build and approved preset stack in one approved rollback action
**And** no active customer session is interrupted by forced update behavior

**Given** rollback is requested
**When** no approved rollback baseline exists or compatibility checks fail
**Then** rollback is rejected without mutating the branch state
**And** the initiating operator sees clear refusal guidance and the rejection is audited
```

근거:

- NFR-006과 PRD release gate의 핵심은 성공 경로가 아니라 `강제 업데이트 금지`와 `거절 규칙`이다.
- 지금 상태로는 구현자가 배포 거부 동작을 임의 해석하게 된다.

#### 변경 제안 E: 고위험 스토리의 NFR 재기록

섹션: `epics.md`

추가 대상:

- Story 1.5: preview confirmation 95백분위 5초, 지연 시 `Preview Waiting` 유지
- Story 2.1: current-session-only review, 0 cross-session leakage
- Story 2.4: warning/end alert 타이밍 허용 오차
- Story 3.2: explicit post-end state 진입/완료 경계
- Story 6.1: staged rollout, rollback audit, active-session protection

근거:

- PRD에만 남겨두면 story 구현 시 누락될 수 있다.
- 구현 준비도 보고서의 `story-level traceability uneven` 지적을 직접 해소한다.

#### 변경 제안 F: Story 1.1 주석 보강

섹션: `epics.md` / Story 1.1 설명 또는 주석

제안:

```md
Implementation Note: Story 1.1 is prerequisite scaffolding for greenfield bootstrap and must not be counted as customer-visible Epic 1 value completion by itself.
```

근거:

- 스프린트 추적 시 foundation story와 사용자 가치 완료를 혼동하지 않도록 한다.

### 4.2 PRD

#### 변경 제안 G: PRD 본문 수정 없음

판단:

- PRD는 이번 변경의 원인 문서가 아니라 근거 문서다.
- 현재 PRD의 `Open Assumptions to Validate`, `Release Gates`, NFR 정의로 필요한 제품 경계는 이미 충분하다.

조치:

- 본문 수정 없음
- 다만 스토리 수정 시 PRD의 NFR-003, NFR-004, NFR-005, NFR-006을 각 관련 story에 명시적으로 끌어내릴 것

### 4.3 Architecture

#### 변경 제안 H: 오래된 source-input 참조 정리

섹션: `architecture.md` / Source Inputs, Important Gaps

OLD:

```md
- [PRD rewrite brief](./prd-rewrite-brief-2026-03-11.md)
```

NEW:

```md
- Remove the stale `PRD rewrite brief` entry from Source Inputs
- Update the note in Important Gaps to say that source-input hygiene has been resolved against the approved current artifact set
```

근거:

- 구현 준비도 보고서가 직접 경고했다.
- 승인된 현재 입력 집합과 어긋나는 참조는 구현 지시를 흔든다.

#### 변경 제안 I: 에픽 범위 숫자 드리프트 정정

섹션: `architecture.md` / Closed Contract Freeze Baseline, Initial Implementation Priorities

OLD:

```md
... corrected Epic 4-8 story baseline.
1. Regenerate the Epic 4-8 implementation story artifacts ...
```

NEW:

```md
... corrected implementation-story baseline for preset publication, operator recovery, and release-governance tracks.
1. Regenerate the implementation story artifacts for Epic 4-6 against the frozen contract baseline and approved corrected epic map.
```

근거:

- 현재 epics 문서와 번호 체계가 맞지 않는다.
- 숫자보다 도메인 기준 표현으로 바꾸면 이후 드리프트를 줄일 수 있다.

### 4.4 UX

#### 변경 제안 J: UX 본문 수정 없음

판단:

- UX는 현재 문제의 원인이 아니다.
- 다만 구현 단계에서 Brutal Core 시각 선호와 release-gate 계약을 혼동하지 않는다는 경고는 유지해야 한다.

## 5. 구현 인계 계획

### 변경 범주

`Moderate`

이유:

- PRD 재기획이나 MVP 축소는 아니지만, backlog 구조 재조정과 핵심 story 계약 보강이 필요하다.
- 에픽 분리와 스토리 재번호 부여가 수반되므로 PO/SM 조정이 필요하다.

### 권장 인계 대상

- Product Owner / Scrum Master
  - `epics.md` 구조 조정
  - Story 2.3, 4.3, 6.1 acceptance criteria 보강
  - Story-level NFR traceability 주입
- Solution Architect
  - `architecture.md`의 stale source reference 및 epic-range wording 정리
- 이후 필요 시 Development 팀
  - 수정된 story 기준으로 구현 착수

### 성공 기준

1. `epics.md`에서 Epic 5와 rollout governance가 분리되거나, 최소한 동등한 수준의 구조 정정이 반영된다.
2. Story 2.3, 4.3, 6.1에 실패/거절 경로 acceptance criteria가 추가된다.
3. 고위험 story에 필요한 NFR 경계가 다시 연결된다.
4. `architecture.md`의 오래된 입력 참조와 epic-range 숫자 드리프트가 제거된다.
5. 변경 후 구현 준비도 평가를 다시 실행했을 때 현재 지적된 주요 결함이 해소된다.

## 6. 체크리스트 실행 로그

| 항목 | 상태 | 메모 |
| --- | --- | --- |
| 1.1 트리거 스토리 식별 | N/A | 특정 구현 스토리 1건이 아니라 구현 준비도 평가 결과가 변경 트리거임 |
| 1.2 핵심 문제 정의 | [x] Done | 에픽 응집도, 실패 경로 부족, NFR traceability 부족, 아키텍처 참조 위생 문제 |
| 1.3 초기 영향 및 근거 수집 | [x] Done | readiness report의 major issues와 recommendations 사용 |
| 2.1 현재 에픽 영향 평가 | [x] Done | Epic 5 구조 문제 확인 |
| 2.2 에픽 수준 변경 도출 | [x] Done | Epic 5 분리 + 새 Epic 6 제안 |
| 2.3 남은 에픽 영향 검토 | [x] Done | Epic 1-4는 유지, 구조 영향은 Epic 5 이후에 집중 |
| 2.4 미래 에픽 무효화/신규 필요 검토 | [x] Done | 기존 에픽 무효화 없음, 새 rollout governance epic 필요 |
| 2.5 에픽 순서/우선순위 검토 | [x] Done | operator recovery와 release governance를 분리해 독립 추적 권장 |
| 3.1 PRD 충돌 점검 | [x] Done | 직접 충돌 없음 |
| 3.2 Architecture 충돌 점검 | [x] Done | stale reference와 epic-range drift 존재 |
| 3.3 UX 충돌 점검 | [x] Done | 직접 충돌 없음 |
| 3.4 기타 아티팩트 영향 | [x] Done | sprint-status는 현재 없음; 승인 후 재생성 대상 |
| 4.1 Direct Adjustment 평가 | [x] Done | Viable |
| 4.2 Rollback 평가 | [x] Done | Not viable |
| 4.3 PRD MVP Review 평가 | [x] Done | Not viable |
| 4.4 권장 경로 선택 | [x] Done | Direct Adjustment |
| 5.1 이슈 요약 작성 | [x] Done | 본 문서 1절 |
| 5.2 에픽/아티팩트 영향 정리 | [x] Done | 본 문서 2절 |
| 5.3 권장 경로와 근거 작성 | [x] Done | 본 문서 3절 |
| 5.4 MVP 영향 및 액션 플랜 | [x] Done | MVP 변경 없음, planning artifact correction 필요 |
| 5.5 인계 계획 수립 | [x] Done | PO/SM + Architect 권장 |
| 6.1 체크리스트 종합 검토 | [x] Done | 제안서 완성 |
| 6.2 제안서 정확성 검토 | [x] Done | readiness report와 planning artifacts로 교차 확인 |
| 6.3 사용자 승인 | [!] Action-needed | 사용자 승인 대기 |
| 6.4 sprint-status 반영 | N/A | 현재 저장소에 `sprint-status.yaml` 없음. README 기준으로 재생성 대상 |
| 6.5 다음 단계/인계 확인 | [!] Action-needed | 승인 후 실행 |

## 7. 최종 권고

지금 필요한 것은 범위 축소나 재기획이 아니라, 구현 전에 계획 문서를 한 번 더 정리하는 것이다. 특히 Epic 5 분리와 Story 2.3 / 4.3 / 6.1의 실패 경로 보강은 구현 안정성과 테스트 가능성을 크게 올리는 반면, 일정 충격은 제한적이다.

승인된다면 다음 순서로 진행하는 것이 가장 안전하다.

1. `epics.md` 구조 수정
2. 고위험 story acceptance criteria 보강
3. `architecture.md` 참조 정리
4. 필요 시 `sprint-status.yaml` 재생성
5. 구현 준비도 재평가

## 8. 승인 및 인계 확정

- 사용자 승인 상태: Approved
- 승인 시각: 2026-03-20 02:50:35
- 최종 변경 범주: Moderate
- 인계 대상:
  - Product Owner / Scrum Master: `epics.md` 구조 조정, story acceptance criteria 보강, story-level NFR traceability 반영
  - Solution Architect: `architecture.md`의 stale source reference 및 epic-range wording 정리
  - Development Team: 수정된 planning artifacts 승인 후 구현 착수
- `sprint-status.yaml` 처리:
  - 현재 저장소에 파일이 존재하지 않음
  - 본 승인에서는 수정 대상이 아니라 재생성 필요 항목으로 확정

## 9. 워크플로 완료 요약

- Issue addressed: 구현 준비도 평가 결과 `NEEDS WORK`
- Change scope: Moderate
- Artifacts to modify: `epics.md`, `architecture.md`
- Artifacts retained as-is: `prd.md`, `ux-design-specification.md`
- Routed to: Product Owner / Scrum Master, Solution Architect
- Deliverables produced:
  - Sprint Change Proposal document
  - Artifact-specific before/after change proposals
  - Implementation handoff plan
