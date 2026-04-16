---
documentType: sprint-change-proposal
status: approved
project_name: Boothy
date: 2026-04-16 17:08:36 +09:00
change_mode: incremental
change_trigger: "기존 Story 1.23~1.27 완료를 prototype/evidence 완료로 유지하면서, actual architecture implementation을 별도 트랙으로 분리해야 한다."
approved_at: 2026-04-16 17:18:00 +09:00
---

# Sprint Change Proposal

## 1. 이슈 요약

현재 Epic 1의 preview architecture 문맥에서 `prototype/evidence done`과 `actual architecture implementation done`이 분리되어야 한다는 변경 요구가 확인됐다.

- 유지 대상: Stories `1.23~1.27`
- 추가 대상: actual architecture implementation / revalidation 전용 신규 구현 트랙
- 의도: prototype 증거 완료가 final architecture 구현 완료로 오인되지 않게 하고, release-close 판단을 새 actual lane 기준에서만 수행하도록 정렬

발견 근거:

- `epics.md`는 이미 Stories `1.28~1.31`을 actual primary lane 트랙으로 정의한다.
- 그러나 `sprint-plan-preview-architecture-track-20260415.md`, `_bmad-output/implementation-artifacts/sprint-status.yaml`, `release-baseline.md`, `docs/release-baseline.md`, 일부 runbook/contract 문서는 아직 `1.27 -> 1.13` 흐름을 주 기준으로 읽히게 한다.
- 이 불일치 때문에 팀이 `1.23~1.27 done`을 actual architecture implementation 완료로 잘못 해석할 위험이 있다.

문제 유형:

- 기존 요구 해석의 불일치
- 구현 중 드러난 구조 분리 필요

## 2. 영향 분석

### Epic 영향

- **Epic 1 직접 영향:** 큼
  - `1.23~1.27`은 유지하되 의미를 `prototype / evidence / governance` 트랙으로 고정
  - `1.28~1.31`을 actual primary lane 구현 및 재검증 forward path로 사용
  - `1.13`은 `1.31` 이후에만 열리는 final guarded cutover / release-close owner로 유지
  - `1.26`은 actual lane까지 검증한 뒤에도 승인 하드웨어 KPI를 반복 실패할 때만 여는 reserve experiment로 재정의
- **Epic 2~6 영향:** 없음
  - 고객 UX 목표, authoring, operator, rollout 에픽 자체는 유지

### 아티팩트 영향

- **PRD:** 직접 수정 필요 낮음
  - KPI, MVP, 제품 목표는 그대로 유효
  - 다만 release-close 판단이 prototype 완료가 아니라 actual lane 검증 기준에서 닫힌다는 해석 보조 문구는 선택적으로 고려 가능
- **Epics:** 핵심 변경은 이미 반영됨
  - `epics.md`는 이번 변경의 사실상 source of truth 상태
- **Architecture:** 수정 필요
  - preview adoption sequencing과 implementation priority가 `prototype track`과 `actual lane track`을 명확히 구분해야 함
- **UX Design:** 수정 불필요
  - 사용자 경험 계약은 변하지 않음
- **Sprint Plan / Sprint Status:** 수정 필요 높음
  - 현재 운영 권고가 `1.27 evidence -> 1.13 reopen` 기준에 묶여 있음
- **Release / Runbook / Contract 문서:** 수정 필요
  - prototype owner와 actual lane owner 구분이 문서 전체에서 일관되게 유지되어야 함

### 기술 영향

- 코드 구현 방향은 이미 분리 의도를 향하고 있을 수 있으나, 스프린트 운영 기준이 뒤따르지 않으면 잘못된 완료 판정이 발생할 수 있음
- release evidence, hardware validation, rollback authority, canary/default gate 판정 주체가 actual lane 기준으로 재정렬되어야 함

## 3. 경로 평가

### Option 1. Direct Adjustment

- 판단: **Viable**
- 내용:
  - existing stories `1.23~1.27` 유지
  - actual architecture implementation stories `1.28~1.31`를 계획/운영 문서에 승격
  - `1.13`, `1.26`, release gate, hardware evidence wording을 새 기준으로 재정렬
- 노력: **Medium**
- 리스크: **Low-Medium**

### Option 2. Potential Rollback

- 판단: **Not viable**
- 내용:
  - 이미 반영된 epics 구조를 되돌리는 것은 혼란만 키움
  - 문제는 구현 삭제가 아니라 문서와 운영 기준 정렬 부족
- 노력: **High**
- 리스크: **High**

### Option 3. PRD MVP Review

- 판단: **Not viable**
- 내용:
  - MVP 자체나 KPI를 바꿔야 하는 상황이 아님
  - 제품 목표가 아니라 release-close ownership과 implementation sequencing 문제
- 노력: **High**
- 리스크: **Medium**

### 권장 경로

**선택: Option 1. Direct Adjustment**

이 변경은 제품 목표 변경이 아니라, 이미 드러난 architecture truth를 스프린트 운영 체계에 정확히 반영하는 작업이다. rollback이나 MVP 축소 없이도 해결 가능하며, backlog reorganization과 문서 정렬로 충분하다.

## 4. 상세 변경 제안

### 4.1 Stories / Epics

`epics.md`는 현재 방향이 적절하므로 기준 문서로 채택한다.

**상태 제안**

- `1.23~1.27`: 유지, 의미는 `prototype / evidence / gate / corrective follow-up`
- `1.28~1.31`: actual primary lane 구현 및 재검증 트랙으로 backlog 유지
- `1.13`: `1.31` 완료 전까지 blocked
- `1.26`: `1.31` 이후에도 actual lane KPI 반복 실패 시에만 오픈

**추가 편집 필요성**

- `epics.md` 자체는 큰 구조 수정 불필요
- 후속 문서들이 이 구조를 따르도록 동기화 필요

### 4.2 Sprint Plan

대상: `_bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md`

**OLD**

- New track is the only forward path that can reopen preview promotion and release close.
- Story `1.13` stays in the plan, but it remains blocked until Story `1.27` re-proves a canonical local-lane `Go` candidate with rollback evidence on approved hardware.
- Story `1.26` is not part of the default implementation path. It opens only if the local lane repeatedly fails the approved hardware KPI after the new forward path has been exercised.

**NEW**

- Prototype/evidence track and actual architecture implementation track are separate.
- Stories `1.23~1.27` remain completed prototype/evidence/gate history and must not be interpreted as actual primary lane implementation complete.
- Stories `1.28~1.31` become the only forward implementation path that can create the canonical actual-lane `Go` candidate for release-close.
- Story `1.13` stays blocked until Story `1.31` produces accepted canonical actual-lane `Go` evidence with rollback proof.
- Story `1.26` remains closed unless the actual primary lane still repeatedly misses the approved hardware KPI after the `1.28~1.31` track has been exercised.

**Rationale**

- 스프린트 레벨에서 가장 큰 혼동 지점이므로 최우선 수정 대상

### 4.3 Architecture

대상: `_bmad-output/planning-artifacts/architecture.md`

**OLD**

- Preview architecture adoption follows `prototype -> activation -> guarded cutover -> release close`.
- Run Story `1.13` only after the new local native/GPU resident full-screen lane passes prototype, parity, and canary gates.

**NEW**

- Preview architecture adoption must distinguish `prototype/evidence track` from `actual primary lane implementation/revalidation track`.
- Stories `1.23~1.27` prove prototype viability, evidence integrity, canary/default governance, and corrective local follow-up only.
- Stories `1.28~1.31` own actual primary lane implementation, evidence/vocabulary realignment, actual-lane canary, and actual-lane default/rollback decision.
- Story `1.13` is the final guarded cutover / release-close owner only after the actual-lane track closes and canonical rollback-backed `Go` evidence exists.

**Rationale**

- architecture 문서가 계속 old sequencing으로 읽히면 release-close 기준이 다시 흐려짐

### 4.4 Release Baseline

대상:

- `release-baseline.md`
- `docs/release-baseline.md`

**OLD**

- Stories `1.21 through 1.25` are the active forward path, and Story `1.13` remains the final guarded cutover / release-close owner.
- current state remains on hold because Story `1.13` lacks a post-reset rerun package after the earlier local-lane track.

**NEW**

- Stories `1.23~1.27` are completed prototype/evidence/gate history, not the final implementation path.
- Stories `1.28~1.31` are the active actual primary lane forward path for implementation and revalidation.
- Story `1.13` remains the final guarded cutover / release-close owner, but it cannot reopen until Story `1.31` closes with accepted canonical actual-lane `Go` evidence and rollback proof.

**Rationale**

- release truth gate 문구가 잘못되면 제품 관점의 sign-off 기준이 틀어짐

### 4.5 Runbook / Contract 문서

대상 예시:

- `docs/runbooks/preview-promotion-evidence-package.md`
- `docs/contracts/local-dedicated-renderer.md`
- 관련 evidence / contract / guide 문서

**OLD**

- Story `1.23` prototype owner, Story `1.24` canary, Story `1.25` default/rollback, Story `1.13` final close owner

**NEW**

- Stories `1.23~1.27` are prototype/evidence/gate ownership only
- Stories `1.28~1.31` own actual implementation, vocabulary realignment, actual-lane canary, and actual-lane default/rollback authority
- Story `1.13` remains final release-close owner only after the actual-lane track is accepted

**Rationale**

- evidence 해석 규칙이 old track에 고정돼 있으면 운영/검증에서 오판 가능

### 4.6 Sprint Status

대상: `_bmad-output/implementation-artifacts/sprint-status.yaml`

**OLD**

- `preview_architecture_tracks.new_track = 1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25`
- `recommended_next_action = Review completed Story 1.27 evidence ... decide whether Story 1.13 can reopen`
- story gate notes와 development status에 `1.28~1.31` 부재

**NEW**

- `preview_architecture_tracks.prototype_track = 1.23 -> 1.24 -> 1.25 -> 1.27`
- `preview_architecture_tracks.actual_implementation_track = 1.28 -> 1.29 -> 1.30 -> 1.31`
- `recommended_next_action = Start Story 1.28 actual primary lane close owner implementation`
- Story `1.13` gate note를 `1.31 accepted actual-lane Go` 기준으로 갱신
- Story `1.26` gate note를 `actual lane repeated failure after 1.31` 기준으로 갱신
- development status에 `1.28~1.31` 추가

**Rationale**

- 실행팀이 가장 직접적으로 참고하는 운영 파일이라 즉시 정렬 필요

### 4.7 PRD / UX

**PRD**

- 제안: 이번 correct-course에서는 직접 수정하지 않음
- 이유: KPI, MVP, 사용자 가치, 성능 기준은 이미 유효

**UX**

- 제안: 수정하지 않음
- 이유: 고객 경험 계약은 변하지 않고, 이번 변경은 implementation ownership과 release governance 범주

## 5. 구현 핸드오프

### 변경 범위 분류

**Moderate**

이유:

- backlog / sprint 운영 기준 재정렬 필요
- planning artifact와 implementation status 동기화 필요
- release/runbook/contract wording까지 연쇄 반영 필요
- 그러나 제품 목표 자체 재정의나 대규모 replan은 아님

### 권장 핸드오프

- **PO / SM**
  - sprint plan과 sprint-status를 새 track 구조로 재편
  - next action을 `1.28` 시작 기준으로 재지정
- **Architect**
  - architecture, release baseline, runbook/contract wording 정렬
- **Development team**
  - actual lane implementation track(`1.28~1.31`) 기준으로 실행
  - prototype evidence 완료를 final implementation 완료로 보고하지 않음

### 성공 기준

- 모든 운영 문서가 `1.23~1.27 = prototype/evidence`, `1.28~1.31 = actual implementation`, `1.13 = final release close`, `1.26 = reserve`로 일관되게 읽힌다.
- next action이 더 이상 `1.27 evidence review -> 1.13`가 아니라 actual lane 구현 트랙 시작으로 정렬된다.
- release / hardware / rollback 판단이 actual-lane evidence 기준으로만 닫힌다.

## 6. 체크리스트 상태

### 1. Trigger and Context

- `[x]` 1.1 Trigger story identified: `1.23~1.27` 완료 이후 actual implementation과 의미 충돌 발견
- `[x]` 1.2 Core problem defined: prototype/evidence 완료와 actual architecture implementation 완료가 혼재됨
- `[x]` 1.3 Evidence gathered: epics vs sprint plan / sprint-status / release docs 불일치 확인

### 2. Epic Impact Assessment

- `[x]` 2.1 Current epic impact assessed
- `[x]` 2.2 Epic-level changes identified
- `[x]` 2.3 Future epic review complete: Epic 2~6 영향 없음
- `[x]` 2.4 New story chain already identified: `1.28~1.31`
- `[x]` 2.5 Priority/order update needed

### 3. Artifact Conflict and Impact Analysis

- `[x]` 3.1 PRD conflict checked: direct conflict 없음
- `[x]` 3.2 Architecture conflict found
- `[x]` 3.3 UX conflict checked: 영향 없음
- `[x]` 3.4 Secondary artifact impact found: sprint plan, sprint-status, release baseline, runbook, contract docs

### 4. Path Forward Evaluation

- `[x]` 4.1 Option 1 viable
- `[x]` 4.2 Option 2 not viable
- `[x]` 4.3 Option 3 not viable
- `[x]` 4.4 Recommended path selected

### 5. Sprint Change Proposal Components

- `[x]` 5.1 Issue summary complete
- `[x]` 5.2 Epic/artifact impact documented
- `[x]` 5.3 Recommended path documented
- `[x]` 5.4 MVP impact and action plan documented
- `[x]` 5.5 Handoff plan documented

### 6. Final Review and Handoff

- `[x]` 6.1 Checklist reviewed
- `[x]` 6.2 Proposal consistency reviewed
- `[!]` 6.3 User approval required
- `[!]` 6.4 `sprint-status.yaml` update pending approval
- `[!]` 6.5 Final handoff confirmation pending approval

## 7. 결론

이번 변경은 새 구현 요구를 추가하는 것이 아니라, 이미 확인된 actual architecture implementation 필요성을 스프린트 운영 체계에 정확히 반영하는 정렬 작업이다.

권장 실행 순서:

1. 이 제안 승인
2. `sprint-status.yaml`과 sprint plan 우선 갱신
3. architecture / release baseline / runbook / contract 문구 동기화
4. next action을 Story `1.28` 착수 기준으로 전환
