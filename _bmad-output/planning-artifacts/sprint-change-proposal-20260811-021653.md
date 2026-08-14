---
workflow: correct-course
project: Boothy
date: 2026-08-11 02:16:53 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: incremental
approval_status: approved
approved_at: 2026-08-11 02:27:25 +09:00
approval_decision: yes
trigger_reference: _bmad-output/planning-artifacts/implementation-readiness-report-2026-08-11.md
excluded_evaluation_documents:
  - _bmad-output/planning-artifacts/prd-validation-report-20260320-015539.md
  - _bmad-output/planning-artifacts/implementation-readiness-report-20260320.md
individual_proposals_approved:
  - epic-1-forward-dependency-ownership
  - story-7-1-scope-split
  - story-7-6-release-scope-split
  - epic-7-evidence-dependency-rules
  - architecture-source-and-fr-010-traceability
  - prd-and-ux-traceability
  - sprint-and-hardware-ledger-restructure
---

# Sprint Change Proposal - 구현 준비성 보정과 Epic 7 재구성

## 0. 워크플로 프레이밍

- 변경 트리거는 `implementation-readiness-report-2026-08-11.md`의 `NEEDS WORK` 판정이다.
- 현재 FR 커버리지는 100%이며 Story 7.1 착수는 가능하다.
- Epic 7 전체 계획은 스토리 의존성과 범위 조정이 필요하다.
- 신규 MVP 출시는 최종 검증 전까지 `No-Go`다.
- 검토 방식은 점진적 검토이며 7개 개별 변경안은 모두 사용자 승인을 받았다.
- 과거 평가 문서 2개는 이번 판단 근거에서 제외한다.
- 필수 문서인 PRD, Epics, Architecture, UX Design과 현재 Sprint Status를 확인했다. `project-context.md`와 `docs/index.md`는 존재하지 않았다.

## 1. Issue Summary

### 문제 진술

제품 요구사항과 FR 추적 자체는 완전하지만, 현재 실행 계획은 세 가지 이유로 전체 구현 준비성이 부족하다.

1. Epic 1의 Story 1.4와 1.5가 후속 Story의 실제 하드웨어 통합과 검증 증거를 자신의 완료 조건처럼 참조한다.
2. Story 7.1과 기존 Story 7.6이 각각 여러 독립 결과와 검증 책임을 한 스토리에 결합한다.
3. 아키텍처 본문은 FR-010을 지원하지만 자체 검증 문구는 FR-001~FR-009만 선언하고, frontmatter에는 이번 평가에서 제외한 과거 PRD 검증 보고서가 남아 있다.

### 발견 맥락과 증거

- Implementation Readiness 최종 판정: `NEEDS WORK`
- PRD FR 10개 중 Epic 추적 FR: 10개, 누락 FR: 0개
- Story 7.1: 현재 요구·UX·아키텍처 경계가 일치해 착수 가능
- Epic 1:
  - Story 1.4는 실제 helper/camera freshness를 Story 1.6에서 구현하면서도 관련 완료 증거를 직접 요구한다.
  - Story 1.5는 실제 capture round-trip과 render-backed preview가 Story 1.7/1.8에 있으면서도 해당 결과를 자신의 완료 조건에 포함한다.
- Epic 7:
  - Story 7.1은 viewer lifecycle, physical sizing, immutable sample, double-buffer, actual-present, prewarm A/B를 한 번에 결합한다.
  - 기존 Story 7.6은 설치 재현, 100-shot 성능, 복구, 배포·롤백, 최종 출시 판정을 한 번에 결합한다.
- Architecture:
  - `Functional Requirements Coverage`가 FR-001~FR-009만 명시한다.
  - frontmatter가 제외 대상인 `prd-validation-report-20260320-015539.md`를 참조한다.
- Sprint Status:
  - Story 7.1은 `ready-for-dev`다.
  - Story 7.2~7.6은 `backlog`다.
  - 제품 출시는 기존 HV-18이 Go가 되기 전까지 No-Go다.

## 2. Change Analysis Checklist

### 2.1 Trigger and Context

- [N/A] 1.1 단일 triggering story
  - 단일 구현 Story가 아니라 2026-08-11 Implementation Readiness 평가가 직접 트리거다.
  - 관련 Story는 1.4, 1.5, 1.6, 1.7, 1.8, 7.1, 기존 7.6이다.
- [x] 1.2 핵심 문제 정의
  - 유형: 요구 오해라기보다 Story 분해와 완료 책임의 계획 결함
  - 제품 요구와 FR 커버리지는 유지된다.
- [x] 1.3 근거 확보
  - 최신 PRD, Epics, Architecture, UX, Sprint Status, Hardware Validation Ledger와 readiness 보고서를 대조했다.

### 2.2 Epic Impact Assessment

- [x] 2.1 현재 Epic 평가
  - Epic 7은 목표를 유지한 채 완료 가능하지만 Story 분할이 필요하다.
- [x] 2.2 Epic 수준 변경
  - 신규 Epic은 추가하지 않는다.
  - Epic 7을 6개 Story에서 10개 Story로 재구성한다.
- [x] 2.3 나머지 Epic 검토
  - Epic 1은 Story 1.4~1.8 사이의 완료 소유권을 정리한다.
  - Epic 2~6의 제품 범위는 변경하지 않는다.
- [x] 2.4 무효화 여부
  - 폐기되는 Epic이나 완료 구현은 없다.
  - 기존 증거는 회귀·역사 증거로 보존한다.
- [x] 2.5 순서와 우선순위
  - Story 7.1 착수를 유지한다.
  - 모든 직전 번호가 아니라 명시된 evidence dependency로 후속 진행을 통제한다.

### 2.3 Artifact Conflict and Impact Analysis

- [x] 3.1 PRD
  - 목표·FR·MVP 범위 충돌은 없다.
  - Story 검증 위치와 최종 출시 판정 참조만 갱신한다.
- [x] 3.2 Architecture
  - 과거 평가 보고서 참조 제거가 필요하다.
  - FR-010 자체 검증 문구와 구조 매핑을 명시해야 한다.
  - Epic 7의 새 번호와 의존성을 반영해야 한다.
- [x] 3.3 UX
  - 화면과 사용자 흐름 변경은 없다.
  - Story 7.1/7.2 책임 분리와 7.1~7.10 로드맵을 반영해야 한다.
- [x] 3.4 기타 산출물
  - `sprint-status.yaml`, Hardware Validation Ledger, Story 1.4/1.5, Story 7.1 구현 문서가 영향을 받는다.
  - 접근성 UX-DR16의 아키텍처·테스트 책임을 명시한다.

### 2.4 Path Forward Evaluation

- [x] 4.1 Direct Adjustment
  - 판정: Viable
  - Effort: Medium
  - Risk: Low~Medium
  - 제품 목표를 바꾸지 않고 Story 책임과 검증 경계를 바로잡을 수 있다.
- [x] 4.2 Potential Rollback
  - 판정: Not viable / unnecessary
  - Effort: High
  - Risk: High
  - 기존 구현과 증거를 되돌릴 이유가 없다.
- [x] 4.3 PRD MVP Review
  - 판정: 범위 축소 불필요
  - 최종 출시 No-Go와 실장비 검증 기준은 그대로 유지한다.
- [x] 4.4 Recommended Path
  - 선택: Direct Adjustment + Backlog Reorganization
  - 변경 등급: Moderate

### 2.5 Proposal and Handoff Readiness

- [x] 5.1 Issue summary prepared
- [x] 5.2 Epic and artifact impacts documented
- [x] 5.3 Recommended path and alternatives documented
- [x] 5.4 MVP impact and action plan documented
- [x] 5.5 Handoff roles defined
- [x] 6.1 Applicable checklist sections reviewed
- [x] 6.2 Proposal consistency reviewed
- [x] 6.3 Final approval obtained
- [x] 6.4 Sprint Status updated with approved Epic 1/Epic 7 changes
- [x] 6.5 Handoff plan prepared

## 3. Impact Analysis

### Epic Impact

- Epic 1
  - Story 1.4는 고객 상태 표현과 촬영 차단 UI를 소유한다.
  - Story 1.6은 helper/camera freshness, reconnect, false-ready 검증을 소유한다.
  - Story 1.5는 저장 완료와 Preview Waiting 고객 상태·문구를 소유한다.
  - Story 1.7은 실제 capture request, RAW 도착, 세션 저장 correlation을 소유한다.
  - Story 1.8은 preset-applied `previewReady`와 render truth를 소유한다.
- Epic 2~6
  - 제품 범위와 Story 구조 변경 없음
- Epic 7
  - Story 7.1과 기존 Story 7.6을 분할한다.
  - source 비교와 resident renderer 검증을 명시적 Enabler/Spike로 분류한다.
  - 최종 구조는 Story 7.1~7.10이다.

### Story Impact

- 완료된 Story 1.4/1.5는 재개방하지 않는다.
- 기존 Go evidence는 보존하되 정식 출시 증거 소유권을 1.6/1.7/1.8로 정렬한다.
- Story 7.1은 더 작은 범위로 `ready-for-dev`를 유지한다.
- Story 7.2~7.10은 backlog로 유지하며 별도 구현 Story 파일은 아직 생성하지 않는다.
- Story 7.3과 7.5는 기술 후보가 No-Go여도 완전한 증거와 대체 경로 결정이 있으면 검증 Story 자체는 닫을 수 있다.

### Artifact Impact

- PRD: Story 검증 위치와 최종 출시 판정 추적 갱신
- Architecture: 출처 정리, FR-010 검증, 접근성 매핑, Epic 7 순서 갱신
- UX: 구현 로드맵과 Story 책임 갱신
- Epics: Epic 1 소유권 정리, Epic 7 Story 7.1~7.10 재구성
- Sprint Status: Epic 7 Story key와 상태 재구성
- Hardware Validation Ledger: HV-13/HV-18 분할과 책임 재배치
- Story 1.4/1.5/7.1: 승인된 범위와 완료 책임 반영

### Technical and Operational Impact

- 새로운 제품 기능이나 기술 선택을 추가하지 않는다.
- 구현 범위 총량은 대체로 유지되지만 완료 단위와 증거 소유자가 더 작고 명확해진다.
- 활성 세션, 업그레이드 전 세션, 구버전 rollback, generation pointer 호환성은 Story 7.9의 명시적 검증 범위가 된다.

## 4. Recommended Approach

### Chosen Path

`Direct Adjustment + Backlog Reorganization`

### Rationale

- FR 커버리지와 제품 목표는 이미 완전하다.
- Story 7.1은 착수 가능한 핵심 경계를 가지고 있으므로 전체 재계획이나 롤백은 과도하다.
- Story 책임과 검증 gate만 재구성하면 부분 완료와 잘못된 출시 판정을 방지할 수 있다.
- 기술 실험을 고객 결과 Story처럼 취급하지 않아 No-Go 판단도 유효한 학습 결과로 관리할 수 있다.

### Effort and Timeline

- 계획 문서와 추적 산출물 정리: 약 0.5~1일
- Story 7.1은 기존보다 작은 범위가 되어 착수·검토 가능성이 높아진다.
- Epic 7 전체 구현 범위는 기존 추정치인 약 12~27 엔지니어링 일과 soak 범위 안에서 재추정한다.
- Story 7.3 source 결정과 Story 7.5 renderer 결정 뒤 잔여 일정 추정치를 갱신한다.

### Risk Assessment

| 위험 | 수준 | 대응 |
| --- | --- | --- |
| 재번호화 중 참조 누락 | Medium | PRD·UX·Architecture·Epics·Sprint·Ledger 교차 검색 |
| 완료 Story 증거 손실 | Low | 기존 Go/No-Go와 evidence path 보존 |
| 실험 No-Go를 구현 실패로 오해 | Medium | Enabler/Spike 완료 규칙과 production 승격 규칙 분리 |
| 설치·성능·롤백 일부 완료를 전체 출시로 오해 | High | HV-18A~D와 Story 7.10 최종 gate 분리 |
| Story 7.1 범위가 다시 팽창 | Medium | Story 7.2 이후 항목을 Explicitly Out of Scope로 고정 |

## 5. Detailed Change Proposals

### 5.1 Epic 1 Forward Dependency Ownership

#### Story 1.4

OLD:

- 실제 helper/camera readiness 변화와 HV-02, HV-03, HV-10까지 Story 1.4가 직접 완료 조건으로 소유한다.

NEW:

- Story 1.4는 host-normalized readiness의 고객 상태 표현과 촬영 차단 UI를 소유한다.
- Story 1.6은 실제 helper/camera freshness, reconnect, false-ready hardware proof를 소유한다.
- Story 1.4의 구현 완료와 제품 `Ready` 출시 truth를 분리한다.

Rationale:

- UI Story가 후속 hardware integration Story의 구현 결과에 전방 의존하지 않게 한다.

#### Story 1.5

OLD:

- 저장·Preview Waiting UI뿐 아니라 실제 capture round-trip, preset preview 준비, 기존 p95 5초 조건까지 완료 범위에 포함한다.

NEW:

- Story 1.5는 저장 완료와 Preview Waiting의 고객 상태·문구를 소유한다.
- Story 1.7은 실제 촬영 요청, RAW 도착, 세션 저장 correlation을 소유한다.
- Story 1.8은 preset-applied `previewReady`와 render truth를 소유한다.
- 현재 NFR-003의 actual-present 기준과 충돌하는 기존 p95 5초 조건은 제거한다.

Rationale:

- 고객 상태, 실카메라 저장, render-ready를 독립적으로 완료하고 검증할 수 있게 한다.

### 5.2 Story 7.1 Scope Split

OLD:

- Story 7.1 하나가 viewer lifecycle, display sizing, immutable sample, double-buffer, actual-present, prewarm A/B를 모두 소유한다.

NEW:

#### Story 7.1: 촬영 전 전용 관람 창 준비와 화면 크기 계약

- app-lifetime read-only viewer
- 승인 모니터 지정
- 현재 세션 연결
- photo rectangle, DPR, display profile
- listener/layout readiness
- viewer 미준비 시 촬영 차단
- snapshot/reload 복구 계약
- HV-13A

#### Story 7.2: Immutable sample 표시와 actual-present 계측

- request-scoped immutable sample generation
- 완전 decode, 크기, correlation 검증
- atomic display pointer 전환
- opaque double-buffer swap
- trusted input에서 실제 monitor present까지 계측
- visible standby와 hidden prewarm A/B
- HV-13B

Rationale:

- Story 7.1을 독립적으로 구현·검증 가능한 착수 Story로 만든다.

### 5.3 Epic 7 Final Structure

| Story | 목적 | Evidence |
| --- | --- | --- |
| 7.1 | Viewer readiness와 화면 크기 계약 | HV-13A |
| 7.2 | Immutable sample과 actual-present | HV-13B |
| 7.3 | Fast source 비교 Enabler | HV-14 |
| 7.4 | Display-fit immutable preset proxy | HV-15 |
| 7.5 | Resident renderer Architecture Spike | HV-16 |
| 7.6 | RAW 정밀본 무중단 교체 | HV-17 |
| 7.7 | 완전한 installer와 clean offline 재현 | HV-18A |
| 7.8 | 100-shot 성능과 복구 검증 | HV-18B |
| 7.9 | 단계적 배포와 rollback 호환성 | HV-18C |
| 7.10 | 최종 MVP 출시 판정 | HV-18D |

### 5.4 Epic 7 Dependency and Experiment Rules

OLD:

- Story 7.1부터 7.6까지 직전 Story의 Go에만 의존해 순차 실행한다.

NEW:

```text
7.1 Go
  -> 7.2 Go
  -> 7.3 approved source route decision
  -> 7.4 proxy correctness Go
  -> 7.5 renderer adoption decision
  -> 7.6 seamless RAW transition Go
  -> 7.7 installer reproduction Go
  -> 7.8 performance/recovery Go
  -> 7.9 rollout/rollback Go
  -> 7.10 final release decision
```

- Story 7.3은 `Enabler / Experiment`다.
- Story 7.5는 `Architecture Spike`다.
- 후보 기술의 No-Go는 완전한 증거와 대체 경로 결정이 있으면 조사 목적상 완료할 수 있다.
- No-Go 후보는 production 경로로 활성화할 수 없다.
- 최종 출시는 승인된 production route가 반드시 존재해야 한다.

### 5.5 Architecture

#### Source Inputs

OLD:

- frontmatter가 `prd-validation-report-20260320-015539.md`를 입력 문서로 참조한다.

NEW:

- 해당 과거 평가 보고서 참조를 제거한다.
- 현재 PRD, UX, 기술 연구와 승인된 기반 문서만 유지한다.

#### Functional Requirements Coverage

OLD:

> All functional requirements from FR-001 through FR-009 are architecturally supported.

NEW:

> All functional requirements from FR-001 through FR-010 are architecturally supported.

- FR-010을 `viewer-surface`, `display-generation`, Rust `viewer`/`display`, `tests/hardware/viewer-present`에 명시적으로 연결한다.

#### Accessibility Mapping

NEW:

- UX-DR16을 shared UI semantic rules, focus management, modal focus trap/ESC restore, e2e accessibility test 책임에 연결한다.

#### Epic 7 References

NEW:

- Story 7.1~7.10 번호, 범위, evidence dependency를 전체 문서에 반영한다.
- Architecture는 Story 7.1 착수 가능 상태를 유지한다.
- 제품 출시는 Story 7.10 최종 Go 전까지 No-Go다.

### 5.6 PRD and UX

#### PRD

OLD -> NEW:

- Viewer assumption validation: Story 7.1 -> Story 7.1~7.2
- Fast source comparison: Story 7.2 -> Story 7.3
- Resident renderer spike: Story 7.4 -> Story 7.5
- Final release trace: 기존 통합 Story 7.6 -> Story 7.10

제품 요구사항, FR/NFR, MVP 범위와 출시 기준은 변경하지 않는다.

#### UX

OLD:

- Story 7.1이 viewer readiness, physical fit, actual-present, immutable sample, double-buffer를 모두 검증한다.

NEW:

- Story 7.1은 viewer readiness와 physical display contract를 소유한다.
- Story 7.2는 immutable sample, double-buffer, actual-present를 소유한다.
- 구현 로드맵을 Story 7.1~7.10으로 갱신한다.

사용자 화면, 고객 흐름, 접근성 목표는 변경하지 않는다.

### 5.7 Sprint Tracking and Hardware Ledger

#### Sprint Status

```yaml
epic-7: in-progress
7-1-전용-관람-창-준비와-화면-크기-계약: ready-for-dev
7-2-immutable-sample과-actual-present-계측: backlog
7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교: backlog
7-4-화면-적합-immutable-preset-proxy: backlog
7-5-상주-renderer-검증-spike: backlog
7-6-raw-정밀본-무중단-교체: backlog
7-7-완전한-installer와-clean-offline-재현: backlog
7-8-100-shot-성능과-복구-검증: backlog
7-9-단계적-배포와-rollback-호환성-검증: backlog
7-10-최종-mvp-출시-판정: backlog
```

#### Hardware Validation Ledger

- HV-13 -> HV-13A / HV-13B
- HV-14 -> Story 7.3
- HV-15 -> Story 7.4
- HV-16 -> Story 7.5
- HV-17 -> Story 7.6
- HV-18 -> HV-18A / HV-18B / HV-18C / HV-18D
- 기존 evidence path와 Go/No-Go 기록은 보존한다.
- Story 1.4/1.5의 기존 Go evidence는 회귀 증거로 보존하고 정식 출시 책임은 Story 1.6/1.7/1.8로 정렬한다.

## 6. Implementation Handoff

### Scope Classification

`Moderate`

### Handoff Recipients

- Product Owner / Scrum Master
  - Epic 7 backlog 재구성, Story 순서와 status 관리
- Solution Architect
  - FR-010, viewer/display contract, evidence dependency 정합성 검토
- Development Team
  - 축소된 Story 7.1부터 구현
  - Story 7.2 범위를 Story 7.1로 다시 끌어오지 않음
- QA / Release / Operations
  - HV-13A/B와 HV-18A~D evidence 분리 운영
  - 최종 출시 전 모든 truth-critical gate 집계
- Product Manager
  - Story 7.10 최종 MVP Go/No-Go 승인

### Success Criteria

- Story 1.4/1.5가 후속 통합 Story 없이 독립적인 완료 책임을 가진다.
- Story 7.1이 viewer readiness와 화면 크기 계약만으로 착수·검토 가능하다.
- 설치, 성능, 복구, 배포, 최종 출시 판정이 각각 독립적으로 추적된다.
- Architecture가 FR-010을 자체 검증 문구와 구조 매핑에서 명시한다.
- 제외된 과거 평가 문서가 Architecture source input에 남지 않는다.
- PRD, UX, Architecture, Epics, Sprint Status, Ledger의 Story 번호와 gate가 일치한다.
- Story 7.10과 HV-18D가 Go이기 전까지 신규 MVP 출시는 No-Go다.

## 7. Final Review State

- 개별 변경안 승인: 완료
- 통합 Sprint Change Proposal 검토: 완료
- 최종 구현 승인: `yes`
- 승인 시각: `2026-08-11 02:27:25 +09:00`
- 원본 산출물 적용: 완료

## 8. Final Approval and Application Log

### Applied Artifacts

- `prd.md`: Story 7.1~7.10 검증 위치와 최종 Story 7.10/HV-18D 출시 추적 반영
- `ux-design-specification.md`: Story 7.1/7.2 책임 분리와 Story 7.1~7.10 구현 로드맵 반영
- `architecture.md`: 과거 PRD 검증 보고서 참조 제거, FR-010 coverage 문구, UX-DR16 매핑, 새 evidence sequence 반영
- `epics.md`: Story 1.4/1.5 소유권 보정과 Epic 7 Story 7.1~7.10 재구성
- `1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md`: UI/contract 완료와 Story 1.6 release truth 분리
- `1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md`: 고객 상태와 Story 1.7/1.8 integration truth 분리
- `7-1-전용-관람-창-준비와-화면-크기-계약.md`: 축소된 ready-for-dev Story 7.1
- `sprint-status.yaml`: Epic 7 Story 7.1~7.10 상태 반영
- `hardware-validation-ledger.md`: Story 1.6/1.7/1.8 ownership과 HV-13A/B, HV-18A~D 반영

### Scope Classification and Routing

- Change scope: `Moderate`
- Routed to: Product Owner / Scrum Master, Solution Architect, Development, QA / Release / Operations, Product Manager
- First implementation item: Story 7.1 - 촬영 전 전용 관람 창 준비와 화면 크기 계약
- Release position: Story 7.10과 HV-18D가 Go이기 전까지 `No-Go`

## 9. Workflow Completion

- Issue addressed: Epic 1 전방 의존성, Story 7.1/기존 7.6 과대 범위, Architecture FR-010 검증 및 source-input 불일치
- Change scope: `Moderate`
- Artifacts modified: PRD, UX, Architecture, Epics, Story 1.4, Story 1.5, Story 7.1, Sprint Status, Hardware Validation Ledger, Sprint Change Proposal
- Routed to: Product Owner / Scrum Master, Solution Architect, Development, QA / Release / Operations, Product Manager
- Success criterion: Story 7.1은 축소 범위로 착수 가능하며 신규 MVP release는 Story 7.10/HV-18D 전까지 No-Go
