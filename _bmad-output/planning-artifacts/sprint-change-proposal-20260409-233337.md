---
workflow: correct-course
project: Boothy
date: 2026-04-09 23:33:37 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-09 23:33:37 +09:00
approval_decision: yes
trigger_reference: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md
---

# Sprint Change Proposal - preset-applied preview architecture decision 반영

## 0. 워크플로우 프레이밍

- 이번 correct-course는 [technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md](C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\planning-artifacts\research\technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md)를 직접 트리거 입력으로 사용했다.
- 이번 제안은 연구 문서의 결론을 현재 스프린트 산출물에 반영하기 위한 변경 제안이다.
- 검토 문서:
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거: `preset-applied preview architecture decision` 연구 결과
  - 직접 영향 범위: Epic 1 preview/render architecture
- [x] 1.2 Core problem defined
  - 이슈 유형: `Failed approach requiring different solution`
  - 문제 진술: 현재 구조는 `first-visible` 개선 중심 corrective 흐름에 머물러 있다. 그러나 연구 결과는 제품 목표 달성 확률이 가장 높은 다음 구조가 `local dedicated renderer + different close topology`라고 결론낸다.
- [x] 1.3 Evidence gathered
  - 연구 문서는 `local dedicated renderer + different close topology`를 1순위 구조로 제시한다.
  - `edge appliance`는 2차안으로 보류한다.
  - `watch-folder bridge`와 `lighter truthful renderer`는 기본 구조로 채택하지 않는다.
  - 핵심 KPI는 `original visible -> preset-applied visible <= 2.5s`다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 유지 가능하다.
  - 다만 preview/render 구조를 반영하는 story 정렬이 필요하다.
- [x] 2.2 Epic-level change identified
  - 새 epic 추가는 필수는 아니다.
  - Epic 1 안에서 dedicated renderer와 close topology 변경을 반영하는 story 보강 또는 재작성 필요
- [x] 2.3 Remaining epics reviewed
  - Epic 2는 customer-visible promise 유지 차원의 영향이 있다.
  - Epic 3은 직접 영향이 작다.
  - Epic 4/5는 renderer compatibility와 diagnostics wording 정렬이 필요하다.
- [x] 2.4 Future epic invalidation checked
  - MVP 축소는 필요 없다.
  - 다만 기본 preview/render 구조 결정은 연구 결론에 맞게 재정렬되어야 한다.
- [x] 2.5 Epic priority/order checked
  - dedicated renderer와 dual-close topology 관련 작업을 preview track의 우선순위로 올려야 한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD 방향과 충돌하지 않는다.
  - 다만 KPI와 NFR wording은 연구 문서의 핵심 지표와 close 분리를 더 직접 반영해야 한다.
- [x] 3.2 Architecture conflict reviewed
  - 현재 architecture는 resident worker / render worker 문맥은 있으나, 연구 문서가 고른 핵심 구조를 더 직접적으로 적어야 한다.
- [x] 3.3 UX impact reviewed
  - UX 전면 개편은 필요 없다.
  - 고객 약속은 그대로 유지하되, first-visible과 preset-applied truthful close의 관계를 더 명확히 정리해야 한다.
- [x] 3.4 Other artifacts reviewed
  - `render-worker.md`와 `session-manifest.md`는 연구 결론 기준으로 wording 정렬이 필요하다.
  - `sprint-status.yaml`은 dedicated renderer 관련 작업 우선순위를 반영해야 한다.

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Not selected as the final framing
  - Effort: High
  - Risk: Medium
  - 설명: planning artifacts 정렬 자체는 direct adjustment로 수행 가능하지만, 이번 결정의 성격을 충분히 설명하는 최종 프레이밍은 아니다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable as primary path
  - Effort: High
  - Risk: High
  - 설명: 현재 방향을 되돌리는 것이 아니라, 연구가 고른 구조로 전진하는 것이 맞다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Low
  - 설명: MVP 축소 문제가 아니라 아키텍처 선택 문제다.
- [x] 4.4 Recommended path selected
  - 선택안: `Architecture Pivot Adoption`
  - 의미: 연구 문서가 고른 next structure를 공식 구조 결정으로 채택하고, planning artifacts와 sprint priority를 그 방향으로 재정렬

## 2. 이슈 요약

이번 변경의 핵심은 제품 목표에 가장 가까운 preview architecture를 선택하는 일이다. 연구 문서는 `local dedicated renderer + different close topology`가 현재 Boothy에 가장 높은 성공 확률을 가진 구조라고 결론냈다.

따라서 이번 correct-course의 목적은 새로운 아이디어를 추가로 만드는 것이 아니라, 이미 선택된 구조를 현재 PRD, architecture, epic backlog, contract 문서에 맞춰 반영하는 것이다.

## 3. 영향 분석

### Epic 영향

- Epic 1
  - preview/render architecture 관련 story는 dedicated renderer와 close topology 반영 기준으로 정렬 필요
- Epic 2
  - rail UX와 same-slot replacement promise 유지
- Epic 4 / 5
  - render compatibility, diagnostics, fallback wording 정렬 필요

### MVP 영향

- MVP 축소 없음
- booth-first, preset-driven, truthful waiting 유지
- darktable-backed truth source 유지
- local dedicated renderer를 next structure로 채택

### 리스크

- 반영하지 않을 경우
  - planning artifacts와 실제 아키텍처 결정이 어긋난다.
  - backlog가 이전 corrective 방향을 계속 가리킬 수 있다.
- 반영할 경우
  - planning artifact 업데이트와 우선순위 재정렬 비용이 발생한다.
  - 하지만 제품 결정과 구현 방향의 불일치를 줄일 수 있다.

## 4. 권장 접근

### Chosen Path

`Architecture Pivot Adoption: local dedicated renderer + different close topology`

### Why This Path

- 연구 문서가 1순위 구조로 결론냈다.
- same-capture guarantee, preset fidelity, preview/final truth 계약을 유지하면서 병목을 직접 줄일 수 있다.

### Scope Classification

`Major`

### Timeline Impact

1. PRD에서 preview KPI와 truthful close 기준을 먼저 정렬
2. architecture / UX / contract 문서를 PRD 기준으로 정렬
3. backlog / sprint priority를 새 제품 기준에 맞춰 재정렬
4. dedicated renderer prototype와 hardware validation 계획 수립

## 5. 상세 변경 제안

### A. Stories / Epic Proposal

**Artifact:** `_bmad-output/planning-artifacts/epics.md`

#### A-1. Preview architecture story wording 정렬

OLD:
- current preview stories는 first-visible corrective와 resident worker 중심 문맥이 강하다.

NEW:
- Epic 1 preview/render 관련 story는 `local dedicated renderer + different close topology`를 기준 구조로 읽히도록 정렬한다.
- first-visible 개선은 유지하되, preset-applied truthful close owner가 dedicated renderer라는 점을 더 직접적으로 반영한다.

Rationale:
- 연구 문서의 핵심 결론을 epic/story 수준에서도 같은 방향으로 읽을 수 있어야 한다.

#### A-2. 구현 우선순위 정렬

NEW:
- dedicated renderer protocol
- close topology 분리
- hardware validation / cutover
순으로 preview track 우선순위를 정리한다.

Rationale:
- 연구 문서의 Implementation Roadmap을 backlog에 반영해야 한다.

### B. PRD Proposal

**Artifact:** `_bmad-output/planning-artifacts/prd.md`

#### B-1. KPI Table 보강

OLD:
- first-visible latency와 preset-applied preview readiness가 분리돼 있다.

NEW:
- 연구 문서의 핵심 KPI인 `original visible -> preset-applied visible`를 직접 읽을 수 있게 wording을 보강한다.
- first-visible, preset-applied readiness, close latency의 관계를 더 분명히 적는다.

Rationale:
- 아키텍처 선택 이유가 PRD에서도 보이도록 해야 한다.

#### B-2. Runtime truth wording 보강

OLD:
- preset-applied truth는 render behavior가 소유한다.

NEW:
- preset-applied truthful close owner가 local dedicated renderer lane임을 더 명확히 적는다.
- first-visible source는 customer-safe projection이고 truth close owner와 다름을 유지한다.

Rationale:
- 연구 결론이 PRD의 제품 진실값 설명과 맞아야 한다.

### C. Architecture Proposal

**Artifact:** `_bmad-output/planning-artifacts/architecture.md`

#### C-1. Core preview/render decision 보강

OLD:
- render worker, resident worker, fast preview replacement 중심 설명

NEW:
- preview/render 핵심 구조를 `local dedicated renderer + different close topology`로 직접 명시한다.
- host-owned local renderer, canonical close ownership, dual-close model을 architecture 핵심 결정으로 올린다.

Rationale:
- 연구 문서의 최종 권고가 architecture 문서의 핵심 결정과 같아야 한다.

#### C-2. 2차안과 비권장안 명시

NEW:
- `edge appliance`는 2차 실험안으로 보류
- `watch-folder bridge`와 `lighter truthful renderer`는 기본 구조로 채택하지 않음을 남긴다.

Rationale:
- 후보 비교 결과도 architecture artifact에 남겨야 drift가 줄어든다.

### D. UX Proposal

**Artifact:** `_bmad-output/planning-artifacts/ux-design-specification.md`

#### D-1. Preview Waiting promise 보강

OLD:
- fast preview, worker output 등 source 후보 설명이 섞여 있다.

NEW:
- 고객 경험은 "먼저 같은 컷이 보이고, 나중에 truthful preset-applied 결과가 같은 자리에서 안정화된다"는 설명으로 정리한다.
- source 종류보다 close topology와 truthful waiting promise를 우선한다.

Rationale:
- UX 설명도 연구 결론과 같은 제품 약속을 유지해야 한다.

### E. Contract / Tracking Proposal

#### E-1. Render contract wording 정렬

**Artifact:** `docs/contracts/render-worker.md`

OLD:
- current render worker wording

NEW:
- local dedicated renderer ownership, warm-up, preload, queue saturation, truthful fallback 의미를 더 직접적으로 반영한다.

Rationale:
- 계약 문서가 연구 문서보다 뒤처지면 구현 drift가 다시 생긴다.

#### E-2. Session manifest timing wording 정렬

**Artifact:** `docs/contracts/session-manifest.md`

OLD:
- `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs`, `previewVisibleAtMs`

NEW:
- timing field 설명이 `first-visible`, `truthful close`, `preset-applied readiness` 의미를 더 직접 반영하도록 정리한다.

Rationale:
- close topology를 정확히 읽을 수 있어야 한다.

#### E-3. Sprint Tracking 정렬

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

OLD:
- current preview track status

NEW:
- dedicated renderer와 close topology 관련 작업이 preview architecture의 다음 우선순위로 반영되도록 정렬한다.

Rationale:
- sprint tracking도 research decision과 같은 방향을 가리켜야 한다.

## 6. 상위 액션 플랜

1. 이번 proposal 승인
2. PRD에서 preview 목표와 truthful close 기준을 먼저 수정
3. architecture / UX / contract wording을 PRD 기준으로 정렬
4. preview architecture backlog 우선순위 재정렬
5. dedicated renderer prototype 및 hardware validation 계획 수립

## 7. 구현 핸드오프

### Scope

`Major`

### Recommended Recipients

- Product Manager / Solution Architect
  - architecture decision 반영
  - planning artifacts 정렬 승인
- Product Owner / Scrum Master
  - backlog priority 재정렬
- Development team
  - dedicated renderer prototype 방향 정리
- QA / Ops
  - hardware validation 기준 정렬

### Success Criteria

- research와 sprint change proposal이 같은 구조 결론을 가리킨다.
- planning artifacts가 dedicated renderer와 dual-close topology를 반영한다.
- backlog가 새 구조 기준으로 우선순위 정렬된다.

## 8. 승인 게이트

현재 상태:

- Proposal status: `approved`
- Approval decision: `yes`
- Review mode: `batch`

승인 후 반영 예정:

승인 반영 완료:

- `epics.md`
- `prd.md`
- `architecture.md`
- `ux-design-specification.md`
- `render-worker.md`
- `session-manifest.md`
- `sprint-status.yaml`

## 9. 워크플로우 완료 전 요약

- Issue addressed: preview architecture decision을 planning artifacts와 backlog에 반영하는 문제
- Proposed scope: Major
- Artifacts impacted: Epics, PRD, Architecture, UX, Contracts, Sprint Tracking
- Planned route: PM + Architect + PO/SM + Dev + QA/Ops
