---
workflow: correct-course
project: Boothy
date: 2026-04-13 15:51:59 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-13 15:59:54 +09:00
approval_decision: yes
scope_classification: Moderate
handoff_recipients:
  - Product Owner / Scrum Master
  - Product Manager / Architect
  - Development Team
trigger_reference: _bmad-output/planning-artifacts/preview-architecture-gap-analysis-20260413.md
---

# Sprint Change Proposal - Preview Activation Gap 보정

## 0. 첨부 문서 발췌

첨부 문서 [preview-architecture-gap-analysis-20260413.md](C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\planning-artifacts\preview-architecture-gap-analysis-20260413.md)에서 이번 변경을 유발한 핵심 문장만 추리면 아래 네 가지다.

- story slicing이 `prototype -> activation -> validation`이 아니라 사실상 `prototype -> validation`으로 압축되었다.
- `preview-renderer-policy.json`을 실제 운영 경계로 승격하는 owner가 backlog에 없다.
- warm-state는 계약 vocabulary로는 존재하지만 운영 readiness gate로 닫히지 않았다.
- 현재 preview performance 이슈는 측정 부족보다 `새 lane이 아직 실제 primary lane으로 활성화되지 않았다`에 더 가깝다.

이번 correct-course는 위 발췌를 트리거로 사용한다. 즉 제품 방향을 다시 바꾸는 제안이 아니라, 이미 승인된 resident GPU-first 방향이 실제 제품 경로로 승격되도록 backlog와 architecture adoption 단계를 보정하는 제안이다.

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거: `Story 1.13`, `Story 1.18`, `Story 1.19`
  - 근거 문서: `preview-architecture-gap-analysis-20260413.md`
- [x] 1.2 Core problem defined
  - 이슈 유형: `Technical limitation discovered during implementation`
  - 문제 진술: resident GPU-first 방향과 승격 게이트는 문서화됐지만, prototype을 실제 primary lane으로 활성화하는 implementation owner가 backlog에 빠져 있다.
- [x] 1.3 Evidence gathered
  - `hardware-validation-ledger.md`에서 Story `1.13`은 `No-Go` 상태이며, `defaultRoute=darktable`, `laneOwner=inline-truthful-fallback`, `fallbackReason=shadow-submission-only`, `originalVisibleToPresetAppliedVisibleMs=none`이 기록되어 있다.
  - `Story 1.18`, `Story 1.19`는 각각 prototype과 gate establishment를 닫았지만, promoted resident lane success path를 실제 booth route에 올리는 스토리는 없다.
  - `architecture.md`는 resident GPU-first primary lane을 말하지만 adoption stage를 명시적으로 나누지 않는다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - `Epic 1`은 유지 가능하다.
  - 다만 preview architecture adoption을 닫기 위한 activation story가 추가되어야 한다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 불필요하다.
  - `Epic 1`에 activation-focused 신규 story 추가가 필요하다.
- [x] 2.3 Remaining epics reviewed
  - `Epic 2`, `Epic 3`: 직접 영향 없음
  - `Epic 4`, `Epic 5`: 직접 backlog 추가보다는 운영 artifact와 diagnostics deliverable 정렬 영향이 있다.
- [x] 2.4 Future epic invalidation checked
  - 기존 epics를 폐기할 필요는 없다.
  - 현재 구조는 유지하되, Story 간 역할 분리를 더 분명히 해야 한다.
- [x] 2.5 Epic priority/order checked
  - 다음 우선순위는 `activation story -> Story 1.13 rerun` 순으로 재배열되어야 한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - 제품 목표와 충돌은 없다.
  - MVP 축소나 PRD 범위 변경은 필요 없다.
- [x] 3.2 Architecture conflict reviewed
  - 영향이 크다.
  - current architecture는 방향은 맞지만 adoption stage와 rollout artifact 의미가 약하다.
- [x] 3.3 UX impact reviewed
  - 직접 수정은 불필요하다.
  - existing `Preview Waiting`, same-slot replacement, plain-language 원칙은 유지한다.
- [x] 3.4 Other artifacts reviewed
  - `epics.md` 직접 수정 필요
  - `sprint-status.yaml` 승인 후 직접 수정 필요
  - `Story 1.13` 역할 축소 필요
  - `hardware-validation-ledger.md` rerun prerequisite 문구 보강 권장

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Medium
  - Risk: Medium
- [ ] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
- [ ] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: High
  - Risk: Medium
- [x] 4.4 Recommended path selected
  - 선택안: `Direct Adjustment`
  - 의미: 기존 방향은 유지하고, activation owner를 backlog에 복구해 prototype과 release validation 사이의 공백을 메운다.

## 2. 이슈 요약

이번 문제의 본질은 architecture 실패가 아니다. resident GPU-first, darktable fallback/oracle, parity gate라는 큰 방향은 이미 맞다.

문제는 그 방향을 실행 backlog로 번역할 때 `activation` 단계가 빠졌다는 점이다. 지금 구조는 아래처럼 읽힌다.

1. `Story 1.18`: resident lane 후보 검증
2. `Story 1.19`: 측정과 승격 판정 기준 고정
3. `Story 1.13`: 최종 guarded cutover와 hardware `Go / No-Go`

하지만 실제 제품 경로에 필요한 단계는 하나 더 있다.

1. prototype
2. activation
3. guarded cutover
4. release close

현재 `Story 1.13`의 `No-Go`는 단순 validation failure라기보다 activation 공백이 release close story에 흡수된 상태로 읽는 편이 맞다.

## 3. 영향 분석

### Epic 영향

- `Epic 1`
  - 직접 영향이 가장 크다.
  - 신규 activation story를 추가하고 `Story 1.13`의 역할을 final validation owner로 다시 좁혀야 한다.
- `Epic 4`
  - preview-renderer-policy와 published preset/version scope가 activation input으로 쓰이므로 운영 artifact deliverable 정렬 영향이 있다.
- `Epic 5`
  - operator evidence는 계속 필요하지만, 이번 변경의 본질은 diagnostics 확대가 아니라 route promotion ownership 명시다.

### Story 영향

- 유지
  - `Story 1.18` prototype 역할은 유지
  - `Story 1.19` gate establishment 역할은 유지
- 추가 필요
  - `Story 1.20` activation story 추가
- 수정 필요
  - `Story 1.13`은 activation completion을 prerequisite로 두고 final cutover/hardware close owner로 축소

### Artifact 충돌

- `prd.md`
  - 직접 수정 불필요
- `ux-design-specification.md`
  - 직접 수정 불필요
- `architecture.md`
  - adoption stage와 rollout artifact 의미를 더 명시해야 한다
- `epics.md`
  - `Story 1.20` 추가와 `Story 1.13` 역할 재정렬이 필요하다
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `Story 1.20` 추가와 sequencing 반영이 필요하다
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `Story 1.13` rerun prerequisite를 activation story completion 기준으로 읽히게 보강하는 편이 좋다

### Technical / Delivery 영향

- promoted route의 실제 owner가 생긴다
- `preview-renderer-policy.json`가 설정 파일이 아니라 rollout artifact로 관리된다
- warm-state가 diagnostics-only vocabulary가 아니라 activation readiness evidence로 승격된다
- `Story 1.13` rerun 실패가 implementation gap인지 validation gap인지 더 명확히 분리된다

## 4. 권장 접근

### 권장안

`Direct Adjustment`

### 이유

- resident GPU-first 방향을 뒤집을 필요가 없다.
- 이미 구현된 prototype, telemetry, gate establishment는 유효하다.
- 가장 효과적인 보정은 `activation owner를 backlog에 추가`하고 `Story 1.13`을 final validation 역할로 되돌리는 것이다.

### 노력 / 리스크 / 일정 영향

- 노력: Medium
- 리스크: Medium
- 일정 영향:
  - 새 story 하나와 기존 story 정렬이 필요하다.
  - 하지만 기존 구현을 폐기하지 않으므로 재작업 폭은 제한적이다.

## 5. 상세 변경 제안

### 5.1 Architecture

#### Proposal A: adoption stage를 architecture에 명시 추가

**Artifact:** `architecture.md`

**OLD**

```md
- **Preview pipeline model:** The preview pipeline is split into a `first-visible lane` and a `truth lane`. The approved next structure is `resident GPU-first primary lane + different close topology`, where the host owns one resident GPU service for preset-applied close and may still promote an approved same-capture first-visible image into the canonical preview path earlier. darktable remains the baseline, fallback, and parity oracle rather than the default preview truth owner.
```

**NEW**

```md
- **Preview pipeline model:** The preview pipeline is split into a `first-visible lane` and a `truth lane`. The approved next structure is `resident GPU-first primary lane + different close topology`, where the host owns one resident GPU service for preset-applied close and may still promote an approved same-capture first-visible image into the canonical preview path earlier. darktable remains the baseline, fallback, and parity oracle rather than the default preview truth owner.
- **Preview adoption stage:** Preview architecture adoption follows `prototype -> activation -> guarded cutover -> release close`.
- **Route rollout artifact rule:** `preview-renderer-policy.json` is both a route policy artifact and a rollout artifact, and its promoted `canary/default` state is part of release evidence rather than an implementation detail.
- **Warm-state rule:** `warm-ready`, `warm-hit`, `cold`, `warm-state-lost` are not diagnostics-only vocabulary; they are activation-readiness evidence that must be visible in operator-safe proof before guarded cutover can close.
```

**Justification**

- 현재 architecture는 방향은 명확하지만 adoption stage owner를 드러내지 않는다.
- 이번 변경의 핵심 공백은 바로 이 stage 누락이다.

#### Proposal B: 초기 실행 우선순위를 activation 단계까지 재정렬

**Artifact:** `architecture.md`

**OLD**

```md
1. Align preview architecture implementation stories to the approved `local dedicated renderer + different close topology` decision, starting with dedicated renderer ownership, dual-close model, and cutover validation gates.
2. Freeze and expand the dedicated renderer, session-manifest, and sidecar-adjacent contracts so preview truth closes through one host-owned local runtime boundary.
3. Implement the preview architecture pivot behind a safe booth fallback path and prove `original visible -> preset-applied visible` against approved hardware validation.
4. Continue publication, timing/completion, and release-governance tracks without weakening the approved preview/final truth model.
```

**NEW**

```md
1. Preserve Story 1.18 prototype and Story 1.19 promotion-gate outputs as the pre-activation baseline.
2. Add an activation story that promotes approved preset scope from `shadow` to `canary/default` through host-owned `preview-renderer-policy.json` and proves resident success-path evidence on real booth sessions.
3. Run Story 1.13 only after activation proof exists, so guarded cutover and hardware `Go / No-Go` remain release-close work rather than implementation catch-up work.
4. Continue publication, timing/completion, and release-governance tracks without weakening the approved preview/final truth model.
```

**Justification**

- 현재 priority는 prototype 이후 곧바로 cutover validation으로 읽힌다.
- 문서와 실증 결과를 맞추려면 activation이 명시돼야 한다.

### 5.2 Epics / Stories

#### Proposal C: Epic 1에 activation story를 신규 추가

**Artifact:** `epics.md`

**OLD**

```md
### Story 1.18: resident GPU-first display lane prototype과 warm-state service 도입
...
### Story 1.19: ETW/WPR/WPA/PIX + parity diff 기반 승격 게이트 정착
...
```

**NEW**

```md
### Story 1.18: resident GPU-first display lane prototype과 warm-state service 도입
...
### Story 1.19: ETW/WPR/WPA/PIX + parity diff 기반 승격 게이트 정착
...
### Story 1.20: resident preview lane activation과 route policy promotion

As a owner / brand operator,
I want approved preset/version scope를 resident lane canary/default route로 안전하게 승격하고 싶다,
So that prototype과 release validation 사이에 실제 운영 전환 owner가 존재하게 할 수 있다.

**Acceptance Criteria:**

**Given** approved preset/version과 host-owned route policy가 있을 때
**When** activation을 실행하면
**Then** `preview-renderer-policy.json`은 approved scope를 `shadow` 밖으로 `canary` 또는 `default`로 승격할 수 있어야 한다
**And** active session은 route policy 변경으로 재해석되면 안 된다

**Given** activation이 성공한 실세션 evidence를 검토할 때
**When** operator-safe package를 읽으면
**Then** `laneOwner=dedicated-renderer`, `fallbackReason=none`, `routeStage=canary|default`, `warmState=warm-ready|warm-hit` success path가 반복 확인돼야 한다
**And** booth-safe fallback과 one-action rollback evidence가 함께 남아야 한다

**Given** activation이 완료되면
**When** Story 1.13 rerun을 준비하면
**Then** Story 1.13은 implementation corrective가 아니라 final cutover/hardware `Go / No-Go` 판단만 수행할 수 있어야 한다
```

**Justification**

- gap analysis가 지적한 owner 공백을 직접 메운다.
- `preview-renderer-policy.json`와 warm-state evidence를 story deliverable로 승격한다.

#### Proposal D: Story 1.13을 final validation owner로 축소

**Artifact:** `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`

**OLD**

```md
- 이번 스토리는 dedicated renderer를 무조건 기본값으로 켜는 단계가 아니라, guarded cutover와 rollback 가능한 운영 경계 위에서 실장비 증거를 닫는 단계다.
```

```md
- [ ] guarded cutover control과 rollback boundary를 고정한다. (AC: 1, 3, 6)
```

**NEW**

```md
- 이번 스토리는 activation 이후 promoted resident lane을 대상으로 guarded cutover와 rollback 가능한 운영 경계 위에서 final hardware evidence를 닫는 단계다.
```

```md
- [ ] activation story 완료를 prerequisite로 확인한다. (AC: 1, 3, 6)
- [ ] final cutover validation과 rollback proof를 고정한다. (AC: 1, 3, 6)
```

**Justification**

- Story 1.13은 유지하되, implementation gap을 흡수하는 구조를 피해야 한다.
- 이렇게 해야 `No-Go` 원인이 activation 부재인지 release validation 실패인지 분리된다.

### 5.3 Sprint Tracking

#### Proposal E: sprint status에 Story 1.20과 sequencing을 반영

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

**OLD**

```yaml
1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate: review
1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입: done
1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착: done
```

**NEW**

```yaml
1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate: review
1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입: done
1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착: done
1-20-resident-preview-lane-activation과-route-policy-promotion: backlog
```

추가 운영 규칙:

```yaml
preview_architecture_sequence:
  - 1.18 prototype
  - 1.19 gate establishment
  - 1.20 activation
  - 1.13 guarded cutover / release close
```

**Justification**

- 현재 sprint tracking만 보면 1.18/1.19 이후 곧바로 1.13 rerun으로 읽힌다.
- activation story를 명시하지 않으면 같은 공백이 반복된다.

### 5.4 Ledger / Release Proof

#### Proposal F: Story 1.13 rerun prerequisite를 activation completion 기준으로 강화

**Artifact:** `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

**OLD**

```md
Promote approved preset scope beyond shadow in preview-renderer-policy.json, rerun the Story 1.13 HV matrix on booth hardware, and capture a fresh package that proves promoted dedicated-renderer close plus one-action rollback evidence without active-session truth drift.
```

**NEW**

```md
Complete Story 1.20 activation, promote approved preset scope beyond shadow in preview-renderer-policy.json, capture repeated resident success-path evidence, then rerun the Story 1.13 HV matrix with one-action rollback proof and no active-session truth drift.
```

**Justification**

- ledger가 activation gap을 명시적으로 읽어야 rerun 조건이 선명해진다.

### 5.5 PRD / UX

- `prd.md`
  - direct 필수 수정 없음
  - current MVP와 KPI 구조는 그대로 유지 가능
- `ux-design-specification.md`
  - direct 수정 없음
  - customer-facing flow는 이미 activation 이후에도 유지될 수 있는 보호 흐름을 갖고 있다

## 6. 구현 핸드오프

### Change Scope

`Moderate`

### Handoff Recipients

- Product Manager / Architect
  - `architecture.md`에 adoption stage와 rollout artifact 의미를 추가
  - `Story 1.13` 역할을 final validation owner로 재정의
- Product Owner / Scrum Master
  - `epics.md`에 `Story 1.20` 추가
  - `Story 1.13` prerequisite와 sequencing 재정렬
- Scrum Master
  - `sprint-status.yaml`에 `Story 1.20` 반영
  - 다음 순서를 `1.20 -> 1.13 rerun`으로 잠금
- Development Team
  - `preview-renderer-policy.json` promotion, resident success-path 확보, rollback evidence 수집을 `Story 1.20` deliverable로 구현

### Success Criteria

- activation owner가 backlog에 명시된다
- `preview-renderer-policy.json`가 story deliverable로 승격된다
- warm-state가 activation readiness evidence로 관리된다
- Story 1.13이 implementation gap이 아니라 final release-close owner로 다시 읽힌다

## 7. PRD / MVP 영향과 액션 플랜

### MVP 영향

- MVP 범위 변경 없음
- 고객 경험 약속 변경 없음
- preview architecture adoption backlog만 보정

### 고수준 액션 플랜

1. `architecture.md`에 adoption stage와 rollout artifact 규칙을 추가한다.
2. `epics.md`에 `Story 1.20`을 추가한다.
3. `Story 1.13`을 final validation owner로 축소한다.
4. `sprint-status.yaml`과 `hardware-validation-ledger.md`를 새 sequencing에 맞게 동기화한다.
5. `Story 1.20` 완료 후 `Story 1.13` hardware rerun을 수행한다.

## 8. 최종 메모

이번 보정의 핵심은 더 정밀한 측정 체계를 추가하는 것이 아니다. 이미 측정과 gate는 충분히 준비돼 있다.

빠진 것은 `prototype을 실제 운영 경로로 올리는 owner`다. 따라서 가장 안전한 해결은 새로운 방향을 발명하는 것이 아니라, activation story를 backlog에 복구하고 Story 1.13을 다시 final close 역할로 좁히는 것이다.

## 9. 승인 요청

- 제안 상태: `approved`
- 승인 결정: `yes`
- 승인 시각: `2026-04-13 15:59:54 +09:00`
- 다음 단계: `Story 1.20` backlog 등록 후 planning artifact 보정과 handoff를 진행한다.
