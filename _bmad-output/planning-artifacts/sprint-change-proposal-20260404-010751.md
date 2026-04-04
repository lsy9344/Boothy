---
workflow: correct-course
project: Boothy
date: 2026-04-04 01:07:51 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: incremental
approval_status: approved
approved_at: 2026-04-04 01:21:04 +09:00
approval_decision: yes
trigger_reference: history/recent-session-thumbnail-speed-brief.md
---

# Sprint Change Proposal - recent-session thumbnail 구조 변경

## 0. 워크플로우 프레이밍

- 이번 correct-course는 `history/recent-session-thumbnail-speed-brief.md`를 직접 트리거 입력으로 사용했다.
- 사용자는 `incremental` 모드를 선택했고, 주요 변경 제안은 항목별 승인을 받으며 정리했다.
- 검토 문서:
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
  - `docs/contracts/render-worker.md`
  - `_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거: Story 1.9 `fast preview handoff와 XMP preview 교체`
  - 연쇄 영향: Story 1.8 render truth, Epic 1 preview latency track, render worker contract, session-level instrumentation
- [x] 1.2 Core problem defined
  - 이슈 유형: 실장비 검증에서 확정된 기술 구조 한계 + correctness 회귀 가능성
  - 문제 진술: recent-session first-visible 속도가 여전히 제품 기준에 미달하고, 일부 세션에서는 `Preview Waiting` 고착과 preview lane instability가 함께 나타났다. 미세 조정만으로는 더 이상 목표를 맞추기 어렵고, known-good correctness 복구와 상주형 first-visible worker 중심 구조 변경이 필요하다.
- [x] 1.3 Evidence gathered
  - 2026-04-04 실장비 재검증에서 `capture acknowledged -> preview visible` 평균 약 `9238ms`
  - warm 구간 최근 3컷도 `7616ms`, `7761ms`, `8189ms`
  - 일부 컷에서 preview 파일은 있었지만 `renderStatus=previewWaiting`으로 남아 replacement close 실패
  - `preview-render-queue-saturated`, `--disable-opencl` 비지원, preview stderr access violation, missing preview file 흔적 확인
  - 최신 세션에서 per-session `timing-events.log`는 `request-capture`, `file-arrived`, `fast-preview-visible`, `recent-session-visible`를 충분히 닫지 못함

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 유지 가능하다.
  - 다만 Story 1.9의 latency corrective만으로는 더 이상 목표를 달성할 수 없다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필요 없다.
  - Epic 1에 구조 변경용 follow-up story를 추가하는 것이 적절하다.
- [x] 2.3 Remaining epics reviewed
  - Epic 2는 rail UX를 유지하되 first-visible source 다양화를 허용하는 wording 보강이 필요하다.
  - Epic 3은 영향 없음.
  - Epic 4/5는 diagnostics와 known-good render contract 언어 정렬 영향이 있다.
- [x] 2.4 Future epic invalidation checked
  - 무효화되는 epic은 없다.
  - MVP 축소도 필요 없다.
- [x] 2.5 Epic priority/order checked
  - Story 1.9는 `review / No-Go` 상태를 유지한다.
  - 새 Story 1.10을 Epic 1의 다음 truth-critical corrective follow-up으로 우선순위 상향한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD 방향과 충돌하지 않는다.
  - 다만 KPI와 NFR 문구는 first-visible 이미지와 preset-applied ready를 더 분명히 분리해야 한다.
- [x] 3.2 Architecture conflict reviewed
  - Architecture pivot은 필요 없다.
  - 다만 current one-shot render invocation 중심 문맥을 `same engine, different topology`로 보강해야 한다.
- [x] 3.3 UX impact reviewed
  - UX 전면 개편은 필요 없다.
  - 고객 약속은 유지하되 first-visible source가 바뀌어도 보호 흐름이 동일함을 명시하면 된다.
- [x] 3.4 Other artifacts reviewed
  - `render-worker.md`, `sprint-status.yaml`, story backlog ordering, hardware validation evidence 기준 보강이 필요하다.

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Medium-High
  - Risk: Medium
  - same engine 유지, topology 변경, known-good contract 복구, instrumentation repair를 묶는다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
  - Story 1.8/1.9 이전으로 되돌리면 preview truth와 현재 증거를 함께 잃는다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Medium
  - 이번 문제는 MVP 축소가 아니라 현 MVP를 만족시키는 구조 변경 문제다.
- [x] 4.4 Recommended path selected
  - 선택안: Option 1 `Direct Adjustment`
  - 이유: 제품 방향은 유지하면서 구조만 재정렬하면 되고, 엔진 교체 전 마지막 의미 있는 단계이기 때문이다.

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
  - 승인 일시: 2026-04-04 01:21:04 +09:00
- [x] 6.4 `sprint-status.yaml` update completed
  - Story 1.10 backlog entry를 반영했다.
- [x] 6.5 Final handoff confirmation recorded
  - 승인된 범위를 epics, PRD, architecture, UX, render-worker contract, sprint tracking에 반영했다.

## 2. 이슈 요약

현재 recent-session thumbnail 경로는 고객이 실제로 기다리는 병목을 충분히 줄이지 못하고 있다. 최신 실장비 검증에서도 first-visible 속도는 여전히 제품 기준 미달이었고, 일부 컷에서는 preview lane correctness까지 흔들렸다.

이번 판단에서 중요한 점은 두 가지다.

1. 문제의 중심은 UI 반영 속도가 아니라 preview 생성 자체다.
2. 지금 남은 유효한 개선은 옵션 미세 조정이 아니라 구조 변경이다.

따라서 다음 단계는 `known-good correctness 복구 + per-session seam 계측 복구 + 상주형 first-visible worker 설계/도입`을 함께 추진하는 것이다.

## 3. 영향 분석

### Epic 영향

- Epic 1
  - Story 1.9는 `review / No-Go` 유지
  - Story 1.10 신규 추가 필요
- Epic 2
  - latest-photo rail의 고객 경험은 유지
  - first-visible source가 바뀌어도 같은 자리 replacement 원칙 유지
- Epic 4/5
  - render worker contract와 operator diagnostics wording 정렬 필요

### MVP 영향

- MVP 축소 없음
- booth-first, preset-driven, truthful waiting 원칙 유지
- 엔진 교체는 아직 보류
- 이번 단계는 `same engine, different topology`에 해당

### 리스크

- 변경하지 않을 경우
  - first-visible 체감이 계속 기준 미달
  - preview lane regression이 다시 고객 신뢰를 해칠 수 있음
  - 진단 공백 때문에 다음 판단도 느려짐
- 변경할 경우
  - worker lifecycle, queue, warm-up, fallback 경계가 늘어나 구현 복잡도가 증가
  - 하지만 known-good contract와 seam log를 함께 묶으면 통제 가능

## 4. 권장 접근

### Chosen Path

`Direct Adjustment`

### Why This Path

- 구조 변경이 필요하지만 제품 계약은 유지된다.
- 기존 darktable truth와 preset fidelity를 보존할 수 있다.
- 엔진 교체 전 가장 현실적이고 안전한 마지막 큰 단계다.

### Scope Classification

`Moderate`

### Timeline Impact

1. Epic 1 follow-up story 추가
2. PRD KPI/NFR wording 보강
3. architecture/render-worker contract 보강
4. sprint tracking 업데이트
5. 구현 핸드오프

## 5. 상세 변경 제안

### A. Stories / Epic Proposal

**Artifact:** `_bmad-output/planning-artifacts/epics.md`

#### A-1. Story 1.9 상태 명확화

OLD:
- Story 1.9가 fast preview corrective follow-up의 마지막 단계처럼 읽힌다.

NEW:
- Story 1.9는 `review / No-Go` 상태의 fast-preview corrective story로 유지한다.
- latest hardware evidence 기준 미세 조정 단계는 종료되었음을 note로 남긴다.

Rationale:
- 1.9의 목적과 구조 변경 필요 판단을 분리해야 이후 구현이 다시 미세 조정으로 회귀하지 않는다.

#### A-2. Story 1.10 신규 추가

NEW:

Title:
- `Story 1.10: known-good preview lane 복구와 상주형 first-visible worker 도입`

Story:
- booth customer로서,
- 방금 찍은 사진이 제품 기준에 맞는 속도로 최근 세션에 나타나길 원한다.
- 그래서 저장 성공 이후 긴 blank wait나 불안정한 preview replacement 없이 믿고 다음 촬영을 이어갈 수 있다.

Acceptance Criteria:
1. preview lane의 기본 invocation은 booth hardware에서 검증된 known-good contract로 고정된다.
2. per-session diagnostics에는 `request-capture`, `file-arrived`, `fast-preview-visible` 또는 동등 first-visible event, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`가 같은 세션 경로에 남아야 한다.
3. first-visible preview 경로는 per-capture one-shot spawn보다 warm 상태를 유지하는 상주형 worker를 우선 사용한다.
4. worker는 preset 선택 또는 세션 시작 기준 warm-up / preload / cache priming을 허용하되 capture truth를 막지 않는다.
5. canonical preview path와 later preset-applied replacement 규칙은 유지된다.
6. first-visible source는 fast preview, camera thumbnail, intermediate preview, worker output 중 하나일 수 있지만 `previewReady` truth owner는 계속 render worker다.
7. worker failure, queue saturation, warm-state loss가 발생해도 booth는 truthful `Preview Waiting`으로 안전 fallback 한다.

Rationale:
- 이번 실장비 증거가 요구하는 것은 새 UX가 아니라 구조 변경이다.

### B. PRD Proposal

**Artifact:** `_bmad-output/planning-artifacts/prd.md`

#### B-1. KPI Table 분리

OLD:
- `Preview-readiness latency after raw persistence`

NEW:
- `First-visible current-session image latency`
- `Preset-applied preview readiness latency`

Rationale:
- first-visible과 preset-applied ready를 하나의 숫자로 관리하면 이번 구조 변경의 필요성이 흐려진다.

#### B-2. Published Preset Artifact Model 보강

OLD:
- same-capture fast preview may appear earlier

NEW:
- same-capture first-visible source는 fast preview, camera thumbnail, intermediate preview, 또는 구조 변경 뒤 상주형 worker output일 수 있다.
- `previewReady` truth는 계속 capture-bound published artifact의 render behavior가 소유한다.

Rationale:
- 고객 경험의 즉시성과 제품 truth owner를 동시에 잠가야 한다.

#### B-3. NFR-003 보강

OLD:
- truthful current-session image와 preset-applied preview confirmation을 한 단락 안에서 함께 규정한다.

NEW:
- first-visible current-session image latency는 가능한 한 빠르게 최적화한다.
- preset-applied preview readiness는 5초 p95 목표를 유지한다.
- 둘은 같은 지표가 아니며 request-level seam log로 각각 측정한다.

Rationale:
- 제품 기준 미달과 구조 변경 필요 판단을 PRD 레벨에서 바로 읽을 수 있어야 한다.

### C. Architecture Proposal

**Artifact:** `_bmad-output/planning-artifacts/architecture.md`

#### C-1. Preview pipeline topology 보강

OLD:
- fast preview promotion과 later render replacement를 허용한다.

NEW:
- preview pipeline은 `first-visible lane`과 `truth lane`의 두 단계로 명시한다.
- first-visible lane은 warm 상태의 상주형 worker를 우선 사용한다.
- truth lane은 published preset artifact 기반 render-backed replacement를 계속 소유한다.

Rationale:
- 이번 change의 본질은 same engine, different topology다.

#### C-2. Telemetry requirements 보강

OLD:
- fast-preview visibility, render-backed readiness, cold-start delay, queue delay 구분

NEW:
- per-session seam log 필수 이벤트 세트를 명시한다.
- 구조 변경 승인 조건은 이 이벤트들이 같은 세션 파일에서 닫히는 것이다.

Rationale:
- 다음 하드웨어 검증과 이후 판단의 속도를 높인다.

### D. UX Proposal

**Artifact:** `_bmad-output/planning-artifacts/ux-design-specification.md`

#### D-1. Preview Waiting 보호 흐름 보강

OLD:
- same-capture fast preview를 먼저 보여주고 later preview로 교체할 수 있다고 설명한다.

NEW:
- first-visible source 종류가 바뀌어도 고객에게는 "먼저 같은 컷이 보이고, 나중에 더 정확한 결과로 안정화된다"는 보호 경험을 유지한다고 명시한다.
- 상태 승격은 계속 preset-applied readiness에만 연결한다.

Rationale:
- 구조는 바뀌어도 고객 약속은 유지된다는 점을 문서로 잠근다.

### E. Sprint Tracking Proposal

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

OLD:
- `1-9-fast-preview-handoff와-xmp-preview-교체: review`

NEW:
- `1-9-fast-preview-handoff와-xmp-preview-교체: review`
- `1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입: backlog`

Rationale:
- 구조 변경을 명시적으로 backlog에 올려야 구현, 검증, 회고가 분리된다.

## 6. 상위 액션 플랜

1. 이 Sprint Change Proposal 최종 승인
2. `epics.md`, `prd.md`, `architecture.md`, `ux-design-specification.md` wording 반영
3. `sprint-status.yaml`에 Story 1.10 backlog 추가
4. Story 1.10 상세 스토리 작성 및 구현 핸드오프
5. hardware validation에서 first-visible / preset-applied readiness / seam log close를 함께 검증

## 7. 구현 핸드오프

### Scope

`Moderate`

### Recommended Recipients

- Product Owner / Scrum Master
  - Story 1.10 추가와 우선순위 재배치
- Architect
  - preview topology와 render worker contract 정렬
- Development team
  - known-good invocation 복구
  - 상주형 first-visible worker 도입
  - per-session seam instrumentation 복구
- QA / Ops
  - booth hardware validation과 timing evidence 패키지 수집

### Success Criteria

- recent-session first-visible이 구조 변경 전보다 실질적으로 개선된다.
- preset-applied preview truth는 느슨해지지 않는다.
- `Preview Waiting` 고착과 preview replacement correctness 회귀가 재현되지 않는다.
- 최신 세션 하나만 봐도 seam log에서 병목을 다시 닫을 수 있다.

## 8. 승인 게이트

현재 상태:

- Proposal status: `approved`
- Approval decision: `yes`
- Approved at: `2026-04-04 01:21:04 +09:00`
- Incremental approvals obtained:
  - Epic proposal: approved
  - PRD proposal: approved
  - Architecture proposal: approved
  - UX proposal: approved

Applied changes:

- `epics.md`에 Story 1.10 제안 반영
- `prd.md` KPI/NFR wording 반영
- `architecture.md` preview topology / seam logging wording 반영
- `ux-design-specification.md` first-visible 보호 경험 wording 반영
- `render-worker.md` known-good contract / resident worker 기준 반영
- `sprint-status.yaml`에 Story 1.10 `backlog` 추가

## 9. 워크플로우 완료 전 요약

- Issue addressed: recent-session first-visible 속도 기준 미달과 preview lane correctness/계측 공백
- Proposed scope: Moderate
- Artifacts impacted: Epics, PRD, Architecture, UX, Render Worker Contract, Sprint Tracking
- Planned route: PO/SM + Architect + Dev + QA/Ops
