---
workflow: correct-course
project: Boothy
date: 2026-08-11 10:12:38 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-08-11
approval_decision: all-12-approved-and-apply-in-batch
trigger_reference: _bmad-output/planning-artifacts/implementation-readiness-report-2026-08-11-093703.md
scope_classification: Moderate
issue_count: 12
---

# Sprint Change Proposal - Implementation Readiness 12개 이슈 보정

## 1. Issue Summary

### 문제 진술

최신 Implementation Readiness 보고서는 Story 7.1 착수를 차단하지 않지만, 전체 구현 백로그에는 구현자가 서로 다른 결정을 내릴 수 있는 계획 정합성 이슈 12개가 남아 있다고 판정했다.

- UX·추적성 이슈 3개
- Epic/Story Major 이슈 6개
- Story 구조·증거 Minor 이슈 3개

이번 변경은 보고서의 12개 이슈만 보정한다. 제품 기능, FR/NFR, MVP 범위, Epic 7의 출시 No-Go 기준은 변경하지 않는다.

### 발견 맥락과 증거

- 평가 문서: `implementation-readiness-report-2026-08-11-093703.md`
- 필수 계획 문서: PRD, Epics, Architecture, UX Design 모두 존재
- FR 커버리지: 10/10, 100%
- Story 7.1 구현 착수: READY
- 전체 구현 백로그: NEEDS WORK
- 신규 MVP 출시: Story 7.10/HV-18D Go 전까지 No-Go

## 2. Change Analysis Checklist

### 2.1 Trigger and Context

- [N/A] 1.1 단일 triggering Story
  - 단일 Story가 아니라 최신 Implementation Readiness 평가가 직접 트리거다.
- [x] 1.2 핵심 문제 정의
  - 유형: 계획 문서 충돌, 구현 소유권 누락, Story 수용 기준과 release evidence 추적성 부족
- [x] 1.3 근거 확보
  - 보고서, PRD, Epics, Architecture, UX, 승인 변경안, Sprint Status, 관련 Story 문서를 대조했다.

### 2.2 Epic Impact Assessment

- [x] 2.1 기존 Epic 완료 가능성
  - Epic 1~7의 목표는 유지 가능하다.
- [x] 2.2 Epic 수준 변경
  - 신규 Epic은 추가하지 않는다.
  - Epic 1에 누락된 보안 소유자 Story 1.10을 추가한다.
- [x] 2.3 후속 Epic 영향
  - Epic 4/5/6의 privileged workflow는 Story 1.10 완료 전 release-ready가 될 수 없다.
  - Epic 7은 기존 evidence dependency를 유지한다.
- [x] 2.4 무효화 여부
  - 폐기되는 Epic이나 제품 요구사항은 없다.
- [x] 2.5 순서와 우선순위
  - Story 7.1 착수는 유지한다.
  - Story 1.10은 privileged 경로의 출시 준비 전 선행 보정으로 실행한다.

### 2.3 Artifact Conflict and Impact Analysis

- [x] 3.1 PRD
  - FR/NFR/MVP 변경 불필요. 접근성·사용성은 새 PRD 요구가 아니라 독립 release evidence로 추적한다.
- [x] 3.2 Architecture
  - 관리자 인증 구현 소유권과 UX release evidence contract를 명시했다.
- [x] 3.3 UX
  - 고정 1초 readiness 문구를 진단값으로 정리하고 evidence owner를 지정했다.
- [x] 3.4 기타 산출물
  - Epics, 관련 Story 문서, Sprint Status, Hardware Validation Ledger를 갱신했다.

### 2.4 Path Forward Evaluation

- [x] 4.1 Direct Adjustment: Viable
  - Effort: Medium
  - Risk: Low~Medium
- [x] 4.2 Potential Rollback: Not viable / unnecessary
  - 기존 구현과 증거를 되돌리지 않는다.
- [x] 4.3 PRD MVP Review: Not required
  - 제품 범위와 출시 기준을 유지한다.
- [x] 4.4 Recommended Path
  - Direct Adjustment + Backlog Ownership Correction

### 2.5 Proposal and Handoff Readiness

- [x] 5.1 Issue summary prepared
- [x] 5.2 Epic/artifact impacts documented
- [x] 5.3 Recommended path documented
- [x] 5.4 MVP impact and action plan documented
- [x] 5.5 Handoff roles defined
- [x] 6.1 Applicable checklist reviewed
- [x] 6.2 Proposal consistency reviewed
- [x] 6.3 User approved all 12 changes for batch application
- [x] 6.4 Sprint Status updated
- [x] 6.5 Handoff plan prepared

## 3. Impact Analysis

### Epic Impact

- Epic 1
  - Story 1.1 bootstrap 증거 기준을 보강했다.
  - Story 1.9를 immutable generation 기준으로 정렬했다.
  - Story 1.10이 관리자 인증과 privileged capability 경계를 소유한다.
- Epic 4/5
  - rollback과 recovery 실패·동시성·audit AC를 보강했다.
- Epic 6
  - Story 6.1은 독립 review gate를 갖는다.
  - Story 6.2는 Story 1.6/1.7/1.8 중심의 최신 hardware truth 소유권을 사용한다.
- Epic 7
  - Story 7.2는 Enabler, Story 7.10은 Release Governance Gate로 분류한다.
  - Story 7.6은 scheduler/cancellation과 seamless transition을 독립 검토한다.
  - Story 7.10은 UX-EV-01/02를 최종 판정에 포함한다.

### Artifact Impact

- PRD: 변경 없음
- UX: readiness 선행 시간과 release evidence owner 보정
- Architecture: 인증 소유권과 release evidence contract 보정
- Epics: 12개 이슈의 Story 계약과 분류 보정
- Story 문서: 기존 구현 이력은 보존하고 최신 계약을 우선하도록 정렬
- Sprint Status: Story 1.10 backlog 추가
- Hardware Validation Ledger: UX-EV-01/02 release evidence matrix 추가

### Technical and Timeline Impact

- 코드 변경은 이번 Correct Course 범위에 포함하지 않는다.
- Story 1.10과 Story 1.9 remediation은 후속 구현 작업이다.
- 기존 Story 7.1 착수 일정은 변경하지 않는다.
- MVP 출시 일정은 기존처럼 Story 7.10/HV-18D 및 모든 선행 evidence 결과에 따라 결정한다.

## 4. Recommended Approach

### Chosen Path

`Direct Adjustment + Backlog Ownership Correction`

### Rationale

- 제품 요구와 FR 커버리지는 이미 완전하다.
- 12개 이슈는 범위 축소나 재설계보다 계약·소유권·검증 기준 정리로 해결할 수 있다.
- 기존 구현 이력은 삭제하지 않고, 최신 승인 기준과 historical evidence를 구분한다.

### Risk Assessment

| 위험 | 수준 | 대응 |
| --- | --- | --- |
| 기존 Story 완료 이력과 새 계약 혼동 | Medium | historical/superseded 표시와 최신 Correct Course Note 우선 적용 |
| 인증 seam을 실제 인증으로 오해 | High | Story 1.10에서 환경 플래그를 scaffolding으로 명시 |
| SQLite가 audit truth로 재도입 | Medium | JSON/JSONL journal truth와 rebuildable derived index 규칙 고정 |
| UX evidence 누락 상태에서 최종 Go | High | UX-EV-01/02를 Story 7.10/HV-18D 필수 입력으로 등록 |
| 큰 Story의 부분 완료를 전체 완료로 오해 | Medium | Story 6.1/7.6 독립 review gate 적용 |

## 5. Detailed Change Proposals

### 5.1 Viewer readiness 고정 1초 기준 제거

OLD:

- 촬영 최소 1초 전 viewer readiness를 release 기준처럼 표현

NEW:

- 촬영 입력 전 readiness 완료를 구속 계약으로 유지
- 선행 시간은 진단값으로 기록하고 승인된 별도 검증 없이는 고정 1초를 release gate로 사용하지 않음

### 5.2 UX-DR16 접근성 추적성

OLD:

- PRD 번호 밖의 UX-DR16을 최종 release matrix에서 독립 추적하는 소유권이 약함

NEW:

- `UX-EV-01`로 독립 추적
- QA/Release가 evidence를 소유하고 UX+Architect가 검토
- Story 7.10/HV-18D의 필수 입력으로 등록

### 5.3 실부스 사용성 release owner

OLD:

- 터치, standing usability, 고대비, 무가이드 성공률의 구체적 release owner 부재

NEW:

- `UX-EV-02`로 묶고 QA/Release가 evidence를 소유
- PM+UX가 공동 검토

### 5.4 Audit 저장 기준 통일

OLD:

- Epics와 일부 Story 문서가 SQLite를 audit truth로 표현

NEW:

- versioned JSON/JSONL journal이 lifecycle/intervention/publication/rollout evidence의 MVP truth
- SQLite는 journal에서 재생성 가능한 derived query index로만 허용

### 5.5 관리자 인증 구현 소유권

OLD:

- route hiding과 environment flag seam은 있으나 password verification/secure storage/session lifecycle의 Story owner가 없음

NEW:

- Story 1.10 `관리자 인증과 privileged capability 경계 완성` 추가
- password verification, protected storage, privileged session, denial, logout/revocation, audit, command-boundary enforcement 소유

### 5.6 Story 6.2 gate matrix 정렬

OLD:

- Story 1.4/1.5를 product-level hardware truth owner로 포함하고 Story 1.7/1.8 누락

NEW:

- Story 1.6/1.7/1.8을 readiness/capture/render canonical owner로 지정
- Story 1.4/1.5는 historical regression evidence로 유지

### 5.7 Story 1.9 immutable generation 정렬

OLD:

- pending preview와 preset-applied preview가 동일 canonical JPEG path를 교체

NEW:

- capture-scoped immutable pending/render-backed generation 게시
- 완전 검증 뒤 active pointer를 atomic 전환
- 기존 same-path 구현은 superseded historical evidence로 유지

### 5.8 Story 6.1/7.6 과대 범위 완화

OLD:

- 여러 독립 결과가 하나의 완료 판정에 결합

NEW:

- Story 6.1: Rollout Governance / Rollback & Compatibility 독립 review gate
- Story 7.6: Scheduler & Cancellation / Seamless Tier Transition 독립 review gate

### 5.9 Story 4.4/5.2 실패 AC 보강

OLD:

- happy path 중심의 rollback/recovery 기준

NEW:

- 승인되지 않은 target, stale revision, 중단, audit failure, duplicate/concurrent action, timeout, idempotency, mutation-free rejection 기준 추가

### 5.10 Epic 7 Story 유형 분류

OLD:

- Story 7.2와 7.10의 유형 불명확

NEW:

- Story 7.2: `Enabler`
- Story 7.10: `Release Governance Gate`

### 5.11 Story 1.1 bootstrap 증거

OLD:

- dependency install, lockfile, dev/build/package smoke, CI baseline이 AC에 직접 없음

NEW:

- clean checkout 기준 설치, frontend/Tauri smoke, CI baseline과 completion evidence reference를 AC로 추가

### 5.12 UX 실사용 검증의 Story 소유권

OLD:

- 실제 사용성 증거가 특정 Story에 배정되지 않음

NEW:

- Story 7.10이 UX-EV-01/02를 최종 release package에 집계
- Hardware Validation Ledger의 Release Evidence Matrix에서 상태와 경로를 추적

## 6. Implementation Handoff

### Scope Classification

`Moderate`

### Handoff Recipients

- Product Owner / Scrum Master
  - Story 1.10 우선순위와 backlog 상태 관리
- Solution Architect
  - auth/session/capability 및 immutable generation contract 검토
- Development Team
  - Story 1.10 구현
  - Story 1.9 immutable generation remediation
- QA / Release
  - UX-EV-01/02 package 수집
  - Story 6.1/7.6 독립 review gate 운영
- PM + UX
  - UX-EV-02 결과 검토
- Product Manager
  - Story 7.10/HV-18D 최종 Go/No-Go 승인

### Success Criteria

- 12개 이슈가 모두 명시적 소유자와 검증 기준을 가진다.
- Story 7.1 착수 가능 상태가 유지된다.
- audit truth, auth ownership, preview generation 기준이 단일화된다.
- Story 7.10/HV-18D 전까지 MVP release는 No-Go다.

## 7. Approval and Application Log

- 사용자 승인: 12개 모두 일괄 승인
- 적용 방식: Batch
- 변경 등급: Moderate
- 원본 구현 이력: 보존
- 코드 변경: 없음

### Applied Artifacts

- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/implementation-artifacts/1-1-set-up-initial-project-from-starter-template.md`
- `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
- `_bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md`
- `_bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md`
- `_bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md`
- `_bmad-output/implementation-artifacts/4-4-미래-세션-대상-롤백과-카탈로그-버전-관리.md`
- `_bmad-output/implementation-artifacts/5-1-운영자용-현재-세션-문맥과-장애-진단-가시화.md`
- `_bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md`
- `_bmad-output/implementation-artifacts/5-3-라이프사이클-개입-복구-감사-로그-기록.md`
- `_bmad-output/implementation-artifacts/6-1-지점별-단계적-배포와-단일-액션-롤백-거버넌스.md`
- `_bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## 8. Workflow Completion

- Issue addressed: Implementation Readiness 보고서의 12개 이슈
- Change scope: Moderate
- Artifacts modified: UX, Architecture, Epics, 관련 Story 문서, Sprint Status, Hardware Validation Ledger
- Routed to: PO/SM, Architect, Development, QA/Release, PM/UX, Product Manager
- Release position: Story 7.10/HV-18D 및 UX-EV-01/02 Go 전까지 No-Go

Correct Course workflow complete, Noah Lee!
