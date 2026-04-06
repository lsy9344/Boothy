---
workflow: correct-course
project: Boothy
date: 2026-04-06 11:04:30 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-06 11:11:35 +09:00
approval_decision: yes
trigger_reference:
  - docs/recent-session-preview-architecture-update-input-2026-04-06.md
  - history/recent-session-thumbnail-speed-agent-context.md
  - history/recent-session-thumbnail-speed-brief.md
  - history/recent-session-thumbnail-speed-deep-research-brief-2026-04-05.md
  - history/recent-session-thumbnail-speed-log-2026-04-04.md
supersedes:
  - _bmad-output/planning-artifacts/sprint-change-proposal-20260404-232052.md
---

# Sprint Change Proposal - latest large preview replacement 중심 재정렬 초안

## 0. 워크플로우 프레이밍

- 이번 correct-course는 `2026-04-06` 입력 문서를 직접 트리거로 사용했다.
- 사용자 입력이 별도로 없어서 검토 모드는 `batch`로 가정했다.
- 이번 변경은 `2026-04-04` 승인안의 `local dedicated renderer sidecar + darktable fallback` 방향을 뒤집는 제안이 아니다.
- 대신 그 승인안이 여전히 일부 `thumbnail / recent-session rail` 중심 언어를 남기고 있어, 제품 목표와 설계 목표를 `latest large preview replacement` 중심으로 다시 고정하는 제안이다.

검토 문서:

- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
- `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
- `_bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar와-truthful-preview-close-canary-routing.md`
- `docs/recent-session-preview-architecture-update-input-2026-04-06.md`
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260404-232052.md`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 현재 직접 트리거는 Epic 1의 preview latency corrective track 전체이며, 실제로는 `Story 1.11` 단계에서 드러난 문제 정의 불일치다.
  - 구현은 이미 `truthful close`를 줄이려는 방향으로 움직였지만, 문서와 일부 스토리 문구는 여전히 `thumbnail / recent-session rail`을 중심 목표처럼 읽힌다.
- [x] 1.2 Core problem defined
  - 이슈 유형: 구현 중 드러난 기술 한계 + 원래 성공 지표에 대한 오해 수정
  - 문제 진술: `same-capture first-visible` 개선은 실제로 있었지만, 고객이 크게 보고 기다리는 것은 `preset-applied latest large preview`다. 지금 문서 세트는 이 차이를 충분히 못 박지 못해, rail 속도 개선과 제품 성공을 혼동할 위험이 남아 있다.
- [x] 1.3 Evidence gathered
  - `2026-04-04` latest 4컷 재확인: `same-capture first-visible` 평균 `3115ms`, `preset-applied close` 평균 `7715ms`, 첫 컷 `10403ms`
  - `2026-04-05` best run: `same-capture first-visible` 평균 `2959ms`, `preset-applied close` 평균 `6372ms`
  - `2026-04-05` later booth baseline: 완료 close `7772ms`, `8520ms`, `7298ms`, `7526ms`
  - 결론: `3초대`는 first-visible 성과이고, 고객 체감 병목은 여전히 `6초대 후반 ~ 10초대`의 truthful close다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 여전히 유효하다.
  - 다만 현재 Epic 1의 일부 스토리는 `recent-session rail`을 중심 artifact처럼 읽혀, 최신 큰 프리뷰 교체라는 실제 목표를 약하게 만든다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필요 없다.
  - 대신 Epic 1의 기존 `1.11` 문구를 수정하고, artifact/계측 재정렬 전용 follow-up story를 추가하는 편이 적절하다.
- [x] 2.3 Remaining epics reviewed
  - Epic 2는 유지하되, `rail`은 주 artifact가 아니라 보조 artifact라는 점을 명시해야 한다.
  - Epic 3은 직접 영향은 낮지만 `latest large preview`와 `post-end completion truth`를 섞지 않도록 문서 경계를 유지해야 한다.
  - Epic 5는 operator diagnostics가 `selected route`, `close owner`, `latest large preview close` 근거를 더 분명히 보여줘야 한다.
  - Epic 6은 canary / rollback 기준을 `thumbnail`이 아니라 `latest large preview replacement` 기준으로 읽게 해야 한다.
- [x] 2.4 Future epic invalidation checked
  - 무효화되는 epic은 없다.
  - MVP 축소도 필요 없다.
- [x] 2.5 Epic priority/order checked
  - `1.11`은 계속 진행할 수 있지만, 목표 문구를 즉시 교정해야 한다.
  - 신규 `1.12`는 `latest large preview artifact ownership + seam reinstrumentation`을 소유하는 backlog story로 두는 것이 안전하다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD는 `first-visible`과 `preset-applied readiness`를 분리하고 있어 방향 자체는 맞다.
  - 하지만 아직 `large preview가 주 artifact`라는 제품 관점이 충분히 명시돼 있지 않다.
- [x] 3.2 Architecture conflict reviewed
  - Architecture는 `first-visible lane`과 `truth lane`을 분리하고 있어 구조 방향은 맞다.
  - 그러나 `latest large preview`와 `rail thumbnail`이 같은 close owner를 공유해야 한다는 artifact hierarchy가 약하다.
- [x] 3.3 UX impact reviewed
  - UX 전면 개편은 필요 없다.
  - 다만 `Latest Photo Rail`은 고객 확신을 돕는 보조 surface이지, 제품 성공의 주 측정 대상이 아니라는 점을 명시해야 한다.
- [x] 3.4 Other artifacts reviewed
  - `_bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar와-truthful-preview-close-canary-routing.md`
  - `_bmad-output/implementation-artifacts/2-1-현재-세션-사진-레일과-세션-범위-검토.md`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
  - `sprint-status.yaml`

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Medium
  - Risk: Medium
  - 문서와 스토리 목표를 바로 교정할 수 있다.
  - 단, 현재 진행 중인 `1.11` 범위를 건드리므로, cross-cutting 계측 정리를 별도 follow-up으로 떼는 편이 안전하다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
  - `2026-04-04` 이후 얻은 routeable topology, fallback, canary 자산을 되돌릴 이유는 없다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Medium
  - 이번 문제는 MVP 축소가 아니라 MVP 성공 정의의 중심 artifact를 교정하는 문제다.
- [x] 4.4 Recommended path selected
  - 선택안: `Hybrid`
  - 설명: 기존 `1.11`은 목표 문구를 `latest large preview replacement` 중심으로 수정하고, 신규 `1.12`에서 artifact ownership과 seam reinstrumentation을 독립 관리한다.
  - 이유: 이미 시작된 route 실험을 살리면서도, 이번에 드러난 문서/계측 불일치를 별도 backlog로 안전하게 닫을 수 있기 때문이다.

### 5. Proposal Components

- [x] 5.1 Issue summary prepared
- [x] 5.2 Epic and artifact impact summarized
- [x] 5.3 Recommended path documented
- [x] 5.4 MVP impact and high-level action plan defined
- [x] 5.5 Handoff plan defined

### 6. Final Review and Handoff

- [x] 6.1 Checklist completion reviewed
- [x] 6.2 Proposal consistency reviewed
- [x] 6.3 User approval obtained
  - 승인 일시: 2026-04-06 11:11:35 +09:00
- [x] 6.4 `sprint-status.yaml` update completed
- [x] 6.5 Handoff plan drafted

## 2. 이슈 요약

현재 Boothy는 이미 한 가지를 개선했다.

- 고객이 같은 컷을 "먼저 한 번 보는" 속도는 `2026-04-04`와 `2026-04-05` 실측에서 `약 3초` 수준까지 내려왔다.

하지만 고객이 실제로 기다리는 것은 그게 아니다.

- 고객이 크게 보고 있는 최신 사진이 `프리셋 적용 상태`로 교체되는 시점은 여전히 대체로 `6초대 후반 ~ 8초대`, 나쁜 경우 `10초대`다.

즉 지금 남은 문제는 `thumbnail speed`가 아니라 `latest preset-applied preview replacement latency`다.

이번 변경의 핵심은 두 가지다.

1. 제품 목표를 `rail thumbnail`이 아니라 `latest large preview` 중심으로 다시 고정한다.
2. `first-visible artifact`와 `truthful close artifact`가 같은 slot, 같은 close owner, 같은 canonical path 규칙 아래 연결되도록 문서와 스토리를 다시 쓴다.

## 3. 영향 분석

### Epic 영향

- Epic 1
  - 직접 영향이 가장 크다.
  - `1.11`은 계속 유효하지만, 성공 기준을 `recent-session rail close`가 아니라 `latest large preview truthful close`로 다시 써야 한다.
  - 신규 `1.12`가 필요하다.
  - `1.12`는 artifact ownership, same-slot shared close owner, seam reinstrumentation을 소유한다.
- Epic 2
  - `2.1`의 고객 가치 자체는 유지된다.
  - 다만 rail은 primary artifact가 아니라 secondary confirmation surface라는 scope note가 필요하다.
- Epic 5
  - operator diagnostics는 `latest large preview` 기준의 route evidence를 보여줘야 한다.
- Epic 6
  - canary / rollback 승인 기준은 `thumbnail visible`이 아니라 `latest large preview truthful close` 단축과 fidelity 유지로 읽혀야 한다.

### PRD / 아키텍처 / UX 충돌 여부

- PRD
  - 방향은 맞지만 제품 서술 중심이 부족하다.
  - `large preview가 주 artifact`, `rail thumbnail은 파생 artifact`라는 문장이 추가돼야 한다.
- Architecture
  - routeable renderer와 host-owned truth는 이미 일부 반영돼 있다.
  - 그러나 `latest large preview`와 `rail thumbnail`이 동일 close owner를 공유해야 한다는 artifact hierarchy는 더 명시돼야 한다.
- UX
  - `Preview Waiting` 보호 흐름은 그대로 유지된다.
  - 다만 `Latest Photo Rail` 설명에서 primary/secondary 역할을 분명히 해야 한다.

### 기술 / 운영 영향

- `fastPreviewVisibleAtMs`와 `previewVisibleAtMs`의 기준 대상을 latest large preview 기준으로 다시 문서화해야 한다.
- session seam package는 `recent-session-visible`만이 아니라 `latest-large-preview-visible` 또는 동등 primary artifact evidence를 포함해야 한다.
- current canary evidence는 route comparison만이 아니라 `large preview close owner` 증명을 포함해야 한다.

## 4. 권장 접근

### Chosen Path

`Hybrid`

### Why This Path

- 이미 진행 중인 route 실험은 유지한다.
- 잘못된 목표 문구를 즉시 교정할 수 있다.
- cross-cutting artifact/계측 보정은 별도 story로 분리해 현재 진행 중인 구현을 불필요하게 흔들지 않는다.

### Scope Classification

`Major`

### Timeline Impact

1. `1.11` 목표 문구를 `latest large preview replacement` 기준으로 수정한다.
2. `1.12`를 추가해 artifact ownership / seam reinstrumentation을 backlog로 명시한다.
3. PRD / Architecture / UX / contract 문서를 latest large preview 기준으로 정렬한다.
4. 승인 후 `sprint-status.yaml`에 `1.12`를 backlog로 반영한다.

## 5. 상세 변경 제안

### A. Stories / Epic Proposal

**Artifact:** `_bmad-output/planning-artifacts/epics.md`

#### A-1. Story 1.11 목표 문구 수정

Story: `1.11 local dedicated renderer sidecar와 truthful preview close canary routing`
Section: Story / Acceptance Criteria

OLD:
- `I want the booth to close my preset-applied preview through the fastest approved route without weakening truth`
- current wording은 `preview close`와 `recent-session rail`을 암묵적으로 같은 목표처럼 읽을 여지가 있다.

NEW:
- `I want the latest large preview that I am actually looking at to be replaced by the truthful preset-applied result through the fastest approved route without weakening truth`
- acceptance criteria에 아래를 추가한다.
  - `latest large preview`가 primary artifact다.
  - `rail thumbnail`은 same capture의 secondary surface이며 primary close owner를 공유해야 한다.
  - route canary의 승인 기준은 `latest large preview truthful close` 단축이다.

Rationale:
- 현재 1.11의 기술 방향은 맞지만, 제품 목표가 rail speed로 오해될 여지가 남아 있다.

#### A-2. Story 1.12 신규 추가

Story: `1.12 latest large preview artifact ownership와 seam reinstrumentation`
Section: New Story

NEW:

As a booth product team,
I want the latest large preview to be defined as the primary customer-visible preview artifact and measured with complete seam evidence,
So that route experiments optimize the real customer wait instead of a secondary rail symptom.

Acceptance Criteria:

1. architecture와 contracts는 `latest large preview`를 primary preview artifact로 정의해야 한다.
2. `rail thumbnail`은 same capture의 secondary artifact이며, primary artifact와 같은 close owner를 공유해야 한다.
3. seam instrumentation은 `fastPreviewVisibleAtMs`, `previewVisibleAtMs`, selected route, fallback reason, close owner를 primary artifact 기준으로 설명해야 한다.
4. one recent approved session package만으로 `first-visible -> truthful large preview close`를 닫을 수 있어야 한다.
5. 고객 상태 taxonomy는 여전히 `Preview Waiting`과 later ready 상태만 사용해야 한다.

Rationale:
- 이번 변경의 핵심은 단순 route 추가가 아니라, 무엇을 성공으로 볼 것인지의 기준 교정이다.

#### A-3. Story 2.1 scope note 추가

Story: `2.1 현재 세션 사진 레일과 세션 범위 검토`
Section: Scope Note

NEW:
- 이 스토리의 `Latest Photo Rail`은 primary confirmation artifact가 아니라 current-session review와 confidence를 돕는 secondary surface다.
- 제품 성능 성공 기준은 rail 등장 속도만으로 닫지 않고, `latest large preview replacement` 기준을 우선한다.

Rationale:
- rail 자체를 최적화 목표로 오해하지 않게 해야 한다.

### B. PRD Proposal

**Artifact:** `_bmad-output/planning-artifacts/prd.md`

#### B-1. Product Definition / Product Thesis 보강

Section: `Product Definition` 또는 동등 서술부

OLD:
- current-session preview confidence와 preset-applied readiness를 구분한다는 문구는 있으나, `large preview`가 주 artifact라는 표현은 약하다.

NEW:
- 고객이 실제로 기다리는 핵심 결과는 `latest large preview`의 truthful preset-applied replacement다.
- rail thumbnail과 representative tile은 이 primary artifact를 보완하는 surface일 뿐, 제품의 primary preview truth를 대신하지 않는다.

Rationale:
- 제품 목표를 KPI 앞단 서술에서 먼저 고정해야 downstream artifact가 흔들리지 않는다.

#### B-2. FR-004 보강

Section: `FR-004 Current-Session Capture Persistence and Truthful Preview Confidence`

OLD:
- `The latest customer-visible confirmation includes only current-session assets.`

NEW:
- `The latest customer-visible confirmation is anchored on the latest large preview for the current capture and includes only current-session assets.`
- `Secondary surfaces such as the current-session rail must not create a different preview truth from the primary latest large preview close owner.`

Rationale:
- FR 수준에서 primary artifact를 못 박아야 한다.

#### B-3. NFR-003 보강

Section: `NFR-003 Booth Responsiveness and Preview Readiness`

OLD:
- first-visible와 preset-applied readiness를 분리 측정한다고만 읽힌다.

NEW:
- `first-visible`와 `preset-applied readiness`는 모두 `latest large preview` 기준으로 읽어야 한다.
- rail thumbnail visibility는 별도 보조 진단으로 기록할 수 있지만, primary performance success gate를 대체하지 못한다.

Rationale:
- 계측 지표의 중심을 고객이 실제로 보는 surface에 고정해야 한다.

### C. Architecture Proposal

**Artifact:** `_bmad-output/planning-artifacts/architecture.md`

#### C-1. Preview pipeline model 보강

Section: `API & Communication Patterns` 또는 `Preview pipeline model`

OLD:
- `first-visible lane`과 `truth lane` 분리는 있으나, 어느 artifact가 primary인지가 약하다.

NEW:
- `latest large preview`는 primary customer-visible preview artifact다.
- `rail thumbnail`은 same capture의 secondary derivative surface다.
- first-visible lane과 truth lane은 둘 다 primary latest large preview를 중심으로 닫혀야 하며, rail은 그 close owner를 공유하거나 파생한다.

Rationale:
- pipeline의 중심 artifact를 정하지 않으면 route 실험이 잘못된 surface에 최적화될 수 있다.

#### C-2. Session seam logging rule 보강

Section: `Session seam logging rule`

OLD:
- `recent-session-visible`까지의 seam event만 있어도 충분한 것처럼 읽힌다.

NEW:
- 한 recent session log는 `request-capture`, `file-arrived`, `fast-preview-visible`, `preview-render-start`, `capture_preview_ready`, `latest-large-preview-visible`, 필요 시 `recent-session-visible`까지 primary/secondary surface를 구분해 보여줘야 한다.

Rationale:
- 현재 seam은 rail evidence와 hero preview evidence를 분리해 읽을 수 있어야 한다.

#### C-3. Closed Contract Freeze Baseline 보강

Section: `Closed Contract Freeze Baseline`

NEW:
- primary preview artifact ownership rule
- shared close owner rule between latest large preview and rail thumbnail
- primary-artifact seam evidence contract

Rationale:
- 이번 변경은 renderer route만이 아니라 artifact ownership contract의 문제다.

### D. UX Proposal

**Artifact:** `_bmad-output/planning-artifacts/ux-design-specification.md`

#### D-1. Capture Loop 서술 수정

Section: `촬영 루프 및 실시간 확인`

OLD:
- latest photo rail이 고객 확신의 중심처럼 읽힌다.

NEW:
- 고객이 가장 크게 보는 최신 사진 영역이 primary confirmation surface다.
- latest photo rail은 같은 current-session capture를 보조 확인하는 secondary surface다.

Rationale:
- UX 문장 구조에서부터 중심 surface를 바로잡아야 한다.

#### D-2. Latest Photo Rail 설명 수정

Section: `Latest Photo Rail`

OLD:
- rail이 same-capture replacement의 중심 설명처럼 읽힌다.

NEW:
- rail은 primary latest preview replacement를 보조하는 review surface다.
- rail item 교체는 primary latest large preview close owner와 충돌하거나 다른 truth를 만들면 안 된다.

Rationale:
- rail은 중요하지만, 제품의 main target은 아니다.

### E. Contract / Sprint Tracking Proposal

**Artifact:** `docs/contracts/render-worker.md` and `docs/contracts/session-manifest.md`

#### E-1. primary artifact wording 추가

NEW:
- `previewReady`와 `previewVisibleAtMs`는 primary latest large preview 기준으로 정의된다.
- secondary rail surfaces는 primary artifact close owner를 공유해야 한다.

Rationale:
- contract layer에서 primary/secondary 구분이 빠지면 구현이 다시 rail 중심으로 미끄러질 수 있다.

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

#### E-2. sprint tracking 업데이트

NEW:
- `1-11-local-dedicated-renderer-sidecar와-truthful-preview-close-canary-routing: in-progress`
- `1-12-latest-large-preview-artifact-ownership와-seam-reinstrumentation: backlog`

Rationale:
- 승인 후에만 backlog를 반영하는 것이 안전하다.

## 6. 상위 액션 플랜

1. `1.11` 스토리 목표를 latest large preview replacement 기준으로 수정한다.
2. `1.12`를 새로 생성해 artifact ownership과 seam reinstrumentation을 별도 관리한다.
3. PRD / Architecture / UX / contracts를 primary preview artifact 기준으로 정렬한다.
4. 승인 후 `sprint-status.yaml`에 `1.12`를 backlog로 추가한다.
5. 다음 hardware canary는 `thumbnail visible`이 아니라 `latest large preview truthful close` 단축을 중심 성공 기준으로 사용한다.

## 7. 구현 핸드오프

### Scope

`Major`

### Recommended Recipients

- Product Manager / Architect
  - primary preview artifact와 success metric 재정의 승인
  - `1.11` 수정과 `1.12` 신설 범위 승인
- Product Owner / Scrum Master
  - Epic 1 sequencing 조정
  - sprint backlog 반영
- Development team
  - artifact ownership wording 정렬
  - seam reinstrumentation
  - route evidence와 large preview close evidence 정리
- QA / Ops
  - hardware canary success criterion 갱신
  - large preview 중심 latency package 검증

### Success Criteria

- 팀이 `thumbnail speed`와 `latest large preview replacement speed`를 더 이상 혼동하지 않는다.
- `1.11` route experiment가 real customer wait target을 기준으로 평가된다.
- `rail thumbnail`은 secondary surface로 남고, 다른 preview truth를 만들지 않는다.
- approved session package 하나만으로 first-visible과 truthful large preview close를 설명할 수 있다.

## 8. 승인 게이트

현재 상태:

- Proposal status: `approved`
- Approval decision: `yes`
- Batch review assumed: `yes`
- Approved at: `2026-04-06 11:11:35 +09:00`
- Applied actions:
  - official sprint change proposal finalized
  - `sprint-status.yaml`에 `1.12` backlog 반영
  - next handoff scope fixed to `latest large preview replacement` realignment

## 9. 워크플로우 완료 전 요약

- Issue addressed: `thumbnail latency` 중심 정의와 실제 고객 대기 목표 간 불일치
- Proposed scope: Major
- Artifacts impacted: Epics, PRD, Architecture, UX, Contracts, Sprint Tracking
- Planned route: PM/Architect + PO/SM + Dev + QA/Ops
