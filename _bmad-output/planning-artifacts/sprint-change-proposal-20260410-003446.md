---
workflow: correct-course
project: Boothy
date: 2026-04-10 00:34:46 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: incremental
approval_status: approved
approved_at: 2026-04-10 00:41:53 +09:00
approval_decision: yes
scope_classification: Moderate
handoff_recipients:
  - Product Owner / Scrum Master
  - Product Manager / Architect
  - Development Team
trigger_reference: _bmad-output/planning-artifacts/implementation-readiness-report-20260410.md
---

# Sprint Change Proposal - implementation readiness 구조 보정

## 0. 워크플로우 프레이밍

- 이번 correct-course는 [implementation-readiness-report-20260410.md](C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\planning-artifacts\implementation-readiness-report-20260410.md)에서 확인된 구현 준비도 이슈 3건을 직접 트리거로 사용했다.
- 이번 제안의 목적은 제품 범위를 바꾸는 것이 아니라, 현재 스프린트 산출물을 실제 구현 순서와 릴리스 거버넌스에 맞게 보정하는 것이다.
- 검토 문서:
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`
  - `_bmad-output/planning-artifacts/implementation-readiness-report-20260410.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `docs/release-baseline.md`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거 스토리: `Epic 6 / Story 6.2`
  - 보조 트리거: 계약 동결 스토리 누락, 초기 CI/CD 스토리 누락
- [x] 1.2 Core problem defined
  - 이슈 유형: `Misunderstanding of original requirements` + `implementation-readiness review에서 드러난 구조 결함`
  - 문제 진술: 현재 backlog는 제품 기능 요구를 잘 커버하지만, 구현 착수에 필요한 계약 동결 항목과 릴리스 게이트를 잘못 스토리화하거나 누락해 실제 실행 순서와 완료 판정을 흔들 위험이 있다.
- [x] 1.3 Evidence gathered
  - readiness 결과에서 `Story 6.2`는 독립 사용자 가치가 없는 cross-cutting gate로 판정됐다.
  - architecture는 구현 시작 전에 동결해야 할 계약 산출물을 명시한다.
  - `docs/release-baseline.md`와 architecture는 초기 release baseline과 CI proof를 요구하지만, epics에는 대응 초기 story가 없다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - `Epic 6`은 유지 가능하지만 `Story 6.2`는 제거 또는 재분류가 필요하다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필수 아님
  - `Epic 1`에 선행 foundational story 추가 필요
  - `Epic 6`은 배포/롤백 거버넌스에 집중하도록 정리 필요
- [x] 2.3 Remaining epics reviewed
  - `Epic 2~5`는 제품 요구 측면에서 유지 가능
  - 단, contract freeze 없이 계속 진행하면 구현 해석 drift 위험이 있다.
- [x] 2.4 Future epic invalidation checked
  - 기존 epics를 폐기할 필요는 없다.
  - 다만 foundational stories 추가와 우선순위 조정은 필요하다.
- [x] 2.5 Epic priority/order checked
  - 계약 동결과 CI baseline은 기능 확장보다 먼저 다뤄야 한다.
  - 이미 스프린트가 진행 중이므로 기존 완료 story 번호는 보존하고, 신규 foundational story를 추가하되 우선순위를 가장 높게 둔다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD 핵심 목표와 충돌 없음
  - MVP 축소 또는 요구사항 수정 불필요
- [x] 3.2 Architecture conflict reviewed
  - architecture 수정은 최소화 가능
  - 핵심은 architecture가 요구하는 contract freeze와 release guardrail을 epics에 반영하는 것
- [x] 3.3 UX impact reviewed
  - UX 전면 수정 불필요
  - hardware validation은 UX 요구가 아니라 release truth gate로 유지하는 것이 적절함
- [x] 3.4 Other artifacts reviewed
  - `epics.md`는 직접 수정 필요
  - `sprint-status.yaml`은 승인 후 동기화 필요
  - `docs/release-baseline.md`는 현재 기준과 정합성이 좋아 즉시 수정 필수는 아님

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Medium
  - Risk: Low-Medium
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: High
  - Risk: Medium-High
- [x] 4.4 Recommended path selected
  - 선택안: `Direct Adjustment`
  - 의미: 제품 범위는 유지하고, backlog 구조와 release governance 표현만 바로잡는다.

## 2. 이슈 요약

이번 변경의 핵심은 제품 요구를 다시 정의하는 것이 아니라, 실제 구현과 릴리스 운영이 같은 기준으로 움직이도록 backlog를 바로잡는 것이다.

현재 상태의 문제는 세 가지다.

1. `Story 6.2`는 사용자 가치 story가 아니라 전역 release truth gate인데 story로 잘못 표현돼 있다.
2. architecture가 선행 동결을 요구한 계약 산출물이 epic/story 구조에 드러나지 않는다.
3. greenfield desktop product인데 초기 CI/CD와 release proof baseline이 story로 관리되지 않는다.

이 세 문제를 방치하면 기능 구현은 계속될 수 있어도, 완료 판정과 통합 기준이 흔들리고 팀별 구현 해석 drift가 커질 가능성이 높다.

## 3. 영향 분석

### Epic 영향

- `Epic 1`
  - 가장 큰 영향이 있다.
  - 이후 기능 확장 전에 foundational contract/CI stories를 추가해야 한다.
- `Epic 2~5`
  - 기능 범위는 유지 가능하다.
  - 다만 shared contract가 불명확하면 후속 구현 정합성 리스크가 커진다.
- `Epic 6`
  - `Story 6.2` 제거 또는 재분류 필요
  - `Story 6.1`은 그대로 유지 가능

### Story 영향

- 제거/재분류 대상
  - `6.2-실장비-hardware-validation-gate와-evidence-기반-done-정책`
- 신규 추가 대상
  - shared contract freeze story
  - Canon helper/publication contract story
  - Windows desktop build/release baseline and CI proof story
- 기존 story 처리 원칙
  - 이미 생성되었거나 완료된 story의 ID는 보존
  - 신규 foundational story는 뒤 번호로 추가하되, 우선순위는 선행으로 조정

### Artifact 충돌

- `prd.md`
  - 수정 불필요
- `architecture.md`
  - 수정 불필요
- `ux-design-specification.md`
  - 수정 불필요
- `epics.md`
  - 직접 수정 필요
- `sprint-status.yaml`
  - 승인 후 직접 수정 필요

### Technical / Delivery 영향

- contract drift 방지
- release truth 판정 일관성 강화
- CI/release baseline 조기 확보
- 현재 스프린트의 in-progress 상태는 유지하되, 다음 생성 story 순서를 재조정

## 4. 권장 접근

### 권장안

`Direct Adjustment`

### 이유

- 제품 범위나 MVP를 바꿀 필요가 없다.
- architecture와 release baseline 문서는 이미 올바른 방향을 제시하고 있다.
- 따라서 가장 비용 대비 효과가 큰 조치는 `epics.md`와 `sprint-status.yaml`을 보정해 구현 순서와 릴리스 기준을 일치시키는 것이다.

### 노력 / 리스크 / 일정 영향

- 노력: Medium
- 리스크: Low-Medium
- 일정 영향:
  - backlog 정리 자체는 작다.
  - 다만 다음 story 생성 순서는 foundational correction을 우선하도록 조정해야 한다.

## 5. 상세 변경 제안

### 5.1 Stories / Backlog

#### Proposal A: `Story 6.2`를 독립 story에서 공통 gate로 재분류

**Artifact:** `epics.md`

**OLD**

```md
### Story 6.2: evidence 기반 hardware validation gate

As a owner / brand operator,
I want truth-critical 스토리가 자동 테스트만으로 완료 처리되지 않게 하고 싶다,
So that 구현 완료와 제품 준비도를 혼동하지 않을 수 있다.
```

**NEW**

```md
### Cross-Cutting Release Truth Gate

- truth-critical stories는 automated pass만으로 제품 관점 `done`이 아니다.
- `hardware-validation-ledger`에 `Go`가 기록되기 전까지 해당 story는 `review` 또는 동등한 pre-close 상태에 머문다.
- booth `Ready`, preset-applied preview truth, `Completed`, preset publication truth는 hardware evidence 없이 release truth로 주장할 수 없다.
```

**Justification**

- 사용자 가치 story가 아니라 전역 운영 규칙이기 때문이다.
- 현재 `sprint-status.yaml`의 done 정의와도 더 잘 맞는다.

#### Proposal B: shared contract freeze story 추가

**Artifact:** `epics.md`

**OLD**

```md
### Story 1.1: Set up initial project from starter template
...
### Story 1.2: 이름과 뒤4자리 기반 세션 시작
```

**NEW**

```md
### Story 1.14: 공유 계약 동결과 검증 기준 확정

As a owner / brand operator,
I want 구현 전에 공통 계약을 먼저 동결하고 싶다,
So that booth, operator, authoring, host 구현이 같은 기준을 따를 수 있다.

Acceptance Criteria:
- session manifest (`session.json`) schema가 확정되어야 한다.
- preset bundle schema가 확정되어야 한다.
- error envelope와 helper/sidecar protocol contract가 확정되어야 한다.
- runtime profile / capability model이 확정되어야 한다.
- 위 계약은 문서와 테스트 가능한 예시 또는 검증 기준과 함께 남아야 한다.
```

**Justification**

- architecture의 implementation sequence 1번 요구사항을 backlog에 반영한다.
- 현재 스프린트가 이미 진행 중이므로 기존 번호는 보존하고 신규 ID를 뒤에 추가한다.

#### Proposal C: Canon helper / publication contract story 추가

**Artifact:** `epics.md`

**NEW**

```md
### Story 1.15: Canon helper profile과 publication contract 확정

As a owner / brand operator,
I want capture boundary와 preset publication boundary 계약을 먼저 확정하고 싶다,
So that 실카메라 연동과 future-session publication이 구현마다 다르게 해석되지 않도록 할 수 있다.

Acceptance Criteria:
- Canon helper implementation profile이 확정되어야 한다.
- authoring publication payload contract가 확정되어야 한다.
- future-session-only publication / rollback rule이 계약 수준에서 명시되어야 한다.
- operator diagnostics와 booth-safe state truth에 필요한 helper semantics가 연결되어야 한다.
```

**Justification**

- architecture의 frozen contract surface를 직접 반영한다.

#### Proposal D: 초기 CI/CD 및 release proof baseline story 추가

**Artifact:** `epics.md`

**NEW**

```md
### Story 1.16: Windows desktop build-release baseline과 CI proof 설정

As a owner / brand operator,
I want 초기 Windows desktop build / release baseline을 먼저 확보하고 싶다,
So that 기능 개발과 별개로 packaging, CI validation, release proof 기준을 안정적으로 유지할 수 있다.

Acceptance Criteria:
- `pnpm build:desktop` 로컬 baseline proof가 동작해야 한다.
- `.github/workflows/release-windows.yml`가 unsigned baseline validation path를 제공해야 한다.
- `pnpm release:desktop` 및 signing-ready 입력 규칙이 문서와 일치해야 한다.
- active booth session을 강제 업데이트하지 않는 release guardrail이 유지되어야 한다.
- automated proof와 hardware proof가 별도 gate라는 사실이 운영 기준에 반영되어야 한다.
```

**Justification**

- `docs/release-baseline.md`와 architecture의 release workflow 요구를 story로 승격한다.
- 지금 시점에서는 Epic 6보다 Epic 1 correction backlog로 두는 편이 더 실용적이다.

### 5.2 Sprint Tracking

#### Proposal E: `sprint-status.yaml` 동기화

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

**OLD**

```yaml
epic-6: in-progress
6-1-지점별-단계적-배포와-단일-액션-롤백-거버넌스: done
6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책: done
```

**NEW**

```yaml
epic-6: in-progress
6-1-지점별-단계적-배포와-단일-액션-롤백-거버넌스: done

epic-1:
  1-14-공유-계약-동결과-검증-기준-확정: backlog
  1-15-canon-helper-profile과-publication-contract-확정: backlog
  1-16-windows-desktop-build-release-baseline과-ci-proof-설정: backlog
```

추가로 상단 workflow notes는 유지하되, hardware validation은 story가 아니라 release truth gate라는 점을 주석 또는 문서 참조로 명확히 한다.

**Justification**

- story ID 재정렬 없이도 현재 스프린트 흐름을 보정할 수 있다.
- 이미 진행 중인 implementation artifact와 충돌을 최소화한다.

### 5.3 PRD / Architecture / UX

- 이번 correct-course에서는 직접 수정하지 않는다.
- 이유:
  - 제품 요구와 UX 방향은 유지 가능
  - architecture는 이미 올바른 기준을 제시하고 있음
  - 문제는 반영 대상인 epics와 sprint tracking 구조에 있다.

## 6. 구현 핸드오프

### Change Scope

`Moderate`

### Handoff Recipients

- Product Owner / Scrum Master
  - `epics.md` 보정
  - foundational story 추가
  - 기존 story 우선순위 재배치
- Scrum Master
  - `sprint-status.yaml` 동기화
  - 다음 생성 story 순서 재조정
- Product Manager / Architect
  - 신규 foundational stories의 acceptance criteria가 architecture freeze baseline과 정확히 일치하는지 최종 검토
- Development Team
  - 승인 후 새 foundational stories부터 순차 구현

### Success Criteria

- `epics.md`에서 `Story 6.2`가 제거되고 공통 release truth gate로 치환된다.
- `epics.md`에 foundational contract/CI stories가 추가된다.
- `sprint-status.yaml`이 새 backlog 구조를 반영한다.
- 다음 implementation story 생성은 foundational correction stories를 우선 대상으로 한다.

## 7. PRD / MVP 영향과 액션 플랜

### MVP 영향

- MVP 범위 변경 없음
- 제품 약속 변경 없음
- 구현 착수 순서와 릴리스 판정 기준만 보정

### 고수준 액션 플랜

1. `epics.md` 수정
2. `sprint-status.yaml` 동기화
3. 필요 시 새 story 파일 생성
4. foundational correction stories 우선 수행
5. readiness 재실행

## 8. 최종 메모

이번 제안은 제품 방향을 흔드는 변경이 아니다. 이미 합의된 PRD, UX, architecture를 실제 backlog와 sprint tracking이 제대로 따르도록 만드는 보정 작업이다. 따라서 가장 안전한 접근은 direct adjustment이며, 승인 후 곧바로 backlog 구조 정리로 이어가면 된다.

## 9. 승인 및 라우팅 결과

- 승인 결과: `yes`
- 변경 범위 분류: `Moderate`
- 라우팅:
  - Product Owner / Scrum Master: backlog 구조 보정과 story 우선순위 조정
  - Product Manager / Architect: foundational story acceptance criteria 최종 정합성 검토
  - Development Team: foundational correction stories 우선 구현
- 다음 단계:
  1. `epics.md` 보정
  2. `sprint-status.yaml` 동기화
  3. 다음 story 생성은 foundational correction story를 우선 대상으로 진행
  4. 변경 반영 후 implementation readiness 재실행
