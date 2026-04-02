---
workflow: correct-course
project: Boothy
date: 2026-04-01 18:50:09 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-02 10:16:48 +09:00
approval_decision: yes
trigger_reference: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md
---

# Sprint Change Proposal - capture preview latency 보정

## 0. 워크플로우 프레이밍

- 이번 correct-course는 2026-04-01 리서치 문서 `_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md`를 트리거 입력으로 사용했다.
- 인터랙티브 질의 대신 `Batch` 모드를 가정했다.
- `project-context.md`는 찾지 못해 별도 프로젝트 컨텍스트 파일은 없는 것으로 처리했다.
- 검토한 핵심 문서:
  - `prd.md`
  - `epics.md`
  - `architecture.md`
  - `ux-design-specification.md`
  - `sprint-status.yaml`
  - `1-5`, `1-7`, `1-8` implementation story

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거: Story 1.8 `게시된 프리셋 XMP 적용과 preview/final render worker 연결`
  - 보조 영향: Story 1.5 `truthful Preview Waiting`, Story 1.7 `RAW handoff correlation`
- [x] 1.2 Core problem defined
  - 이슈 유형: 구현 중 발견된 기술적 제약과 story 분해 미세조정 필요
  - 문제 진술: 현재 제품은 첫 고객 가시 썸네일을 사실상 `darktable-cli` 기반 XMP preview 완료와 묶고 있어, 촬영 저장 후 한동안 "아무 것도 안 보이는" 체감 공백이 생긴다.
- [x] 1.3 Evidence gathered
  - 현재 프런트는 pending preview를 보여줄 수 있다.
  - 현재 helper/host 계약은 `rawPath` 중심이라 fast preview를 실전 경로에 태우지 못한다.
  - `darktable-cli` 1회성 cold-start 비용과 queue 지연이 preview latency를 키운다.
  - Lightroom Classic / darktable는 "즉시 보이는 첫 이미지"와 "나중에 더 정확해지는 이미지"를 분리하는 패턴을 사용한다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 유지 가능하다.
  - 다만 Story 1.8만으로는 "preset-applied preview truth"는 닫히지만 "첫 가시 preview latency"는 충분히 해소되지 않는다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필요 없다.
  - Epic 1에 fast preview 전용 corrective follow-up story를 추가하는 것이 적절하다.
- [x] 2.3 Remaining epics reviewed
  - Epic 2는 rail 구조를 유지한 채 pending image 노출 기준만 보강하면 된다.
  - Epic 3은 final truth 경로를 그대로 유지한다.
  - Epic 4와 Epic 5는 계약/diagnostics 언어 보강 영향이 있다.
- [x] 2.4 Future epic invalidation checked
  - 무효화되는 epic은 없다.
  - MVP 축소나 구조적 재기획도 필요 없다.
- [x] 2.5 Epic priority/order checked
  - Story 1.8은 유지한다.
  - 신규 latency corrective story는 다음 truth-critical follow-up으로 우선순위를 높여야 한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD 방향과 충돌하지 않는다.
  - 다만 현재 문구는 `previewReady`와 "첫 가시 preview"를 충분히 분리하지 않아 해석 여지가 있다.
- [x] 3.2 Architecture conflict reviewed
  - Architecture 방향 전환은 필요 없다.
  - helper optional preview handoff, canonical preview promotion, split timing telemetry를 더 선명히 적어야 한다.
- [x] 3.3 UX impact reviewed
  - UI 전면 개편은 필요 없다.
  - `Preview Waiting` 중에도 same-capture pending image를 보여줄 수 있다는 운영 기준만 반영하면 된다.
- [x] 3.4 Other artifacts reviewed
  - sidecar protocol, session manifest, render-worker, hardware validation runbook, timing logs가 후속 보강 대상이다.

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Medium
  - Risk: Medium
  - Story 1.8은 유지하고, fast preview path를 별도 follow-up으로 추가한다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
  - Story 1.8을 되돌려 raw-copy나 placeholder를 ready truth로 허용하면 false-ready 위험이 다시 열린다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Medium
  - 이번 문제는 MVP 축소 사안이 아니라 기존 MVP를 더 정직하게 구현하는 사안이다.
- [x] 4.4 Recommended path selected
  - 선택안: Option 1 `Direct Adjustment`
  - 이유: 제품 방향은 맞고, fast preview와 preset-applied preview의 역할 분리만 추가로 닫으면 되기 때문이다.

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
  - 승인 일시: 2026-04-02 10:16:48 +09:00
- [x] 6.4 `sprint-status.yaml` update completed
  - Story 1.9 backlog entry를 반영했다.
- [x] 6.5 Final handoff confirmation recorded
  - 승인된 planning 범위를 PRD, Architecture, UX, Epic, sprint tracking에 반영했다.

## 2. 이슈 요약

### Problem Statement

현재 Boothy는 촬영 저장 이후 고객이 처음 보는 이미지를 사실상 `darktable-cli` 기반 XMP preview 완료와 묶고 있다. 그 결과 고객은 "사진은 저장됐다"는 메시지를 본 뒤에도 최신 사진 레일이 비어 있는 시간을 경험할 수 있고, 이는 제품 신뢰를 깎는다.

핵심 병목은 UI 구조보다 preview 생산 파이프라인이다.

- 프런트는 이미 pending preview를 표시할 수 있다.
- helper/host 계약에는 fast preview 경로가 없다.
- preview render는 per-capture `darktable-cli` cold start를 반복한다.

즉, 문제의 본질은 "썸네일 구조가 틀렸다"가 아니라 "첫 노출용 same-capture preview와 나중에 교체될 preset-applied preview를 분리하지 않았다"에 가깝다.

### Discovery Context

- 2026-04-01 리서치 결과, 현재 구조 안에서도 체감 개선 여지는 크다.
- 가장 효과가 큰 조치는 UI 재설계가 아니라 fast preview handoff와 2단계 preview 파이프라인 도입이다.
- 경쟁 제품도 즉시성은 "먼저 보여주고, 나중에 교체하는" 패턴에서 얻는다.

## 3. 영향 분석

### Epic 영향

- Epic 1 영향
  - Story 1.8은 그대로 유지한다.
  - 대신 fast preview latency를 닫는 새 corrective story를 Epic 1에 추가해야 한다.
- Epic 2 영향
  - `Latest Photo Rail` 구조는 유지한다.
  - 다만 `Preview Waiting` 중 same-capture pending image를 허용하는 기준이 planning 문서에 반영되어야 한다.
- Epic 3 영향
  - final truth gating 방향은 변하지 않는다.
  - preview 즉시성 보강이 final truth를 느슨하게 만들면 안 된다는 note가 있으면 충분하다.
- Epic 4 영향
  - published bundle은 여전히 runtime render truth의 권위 artifact다.
  - 다만 representative preset tile은 selection aid일 뿐 first-visible capture preview truth가 아니라는 점을 더 분명히 해야 한다.
- Epic 5 영향
  - 운영자 진단은 `fast-preview-missed`, `render-cold-start`, `render-queue-delay` 같은 분리된 원인을 다룰 수 있어야 한다.

### MVP 영향

- MVP 축소는 필요 없다.
- UI 전면 개편도 필요 없다.
- 이번 변경은 "저장 성공과 preset-applied ready를 분리한다"는 기존 MVP를 더 설득력 있게 만드는 보정이다.

### 리스크

- 변경을 하지 않을 경우
  - 촬영 직후 blank time이 지속된다.
  - 사용자는 저장 성공과 booth responsiveness를 의심할 수 있다.
  - Story 1.8의 render truth는 맞아도 실제 체감 속도 문제는 계속 남는다.
- 변경을 할 경우
  - helper/host 계약과 telemetry가 약간 복잡해진다.
  - fast preview가 same-capture 정합성을 잃으면 오히려 신뢰를 해칠 수 있으므로 검증이 중요하다.

## 4. 권장 접근

### Chosen Path

`Direct Adjustment`

### Why This Path

- Story 1.8이 닫은 "preset-applied preview truth"는 유지해야 한다.
- blank waiting 문제는 그 truth를 롤백하지 않고도 fast preview follow-up으로 해결할 수 있다.
- PRD/Architecture/UX가 모두 허용 가능한 범위 안에서 해결된다.

### Scope Classification

`Moderate`

### Timeline Impact

- 새 epic이나 대규모 UI 재설계는 없다.
- 승인 후 해야 할 핵심 순서는 아래와 같다.
  1. Story 1.9 추가
  2. helper contract와 host ingest contract 보강
  3. pending preview 노출과 XMP replacement 연결
  4. telemetry / hardware validation 보강

## 5. 상세 변경 제안

### A. Stories / Epic Proposal

**Artifact:** `epics.md`

#### A-1. Story 1.8 범위 재명시

OLD:
- Story 1.8이 사실상 "첫 고객 가시 preview"와 "preset-applied previewReady truth"를 함께 책임지는 것으로 읽힐 수 있다.

NEW:
- Story 1.8은 `previewReady` / `finalReady`의 truth owner로 유지한다.
- Story 1.8 note에 "same-capture fast preview의 첫 노출은 별도 latency corrective story가 소유하며, `previewReady`를 조기 승격하지 않는다"는 문구를 추가한다.

Rationale:
- 이미 구현된 render truth를 되돌리지 않으면서 latency corrective scope를 분리해야 한다.

#### A-2. Story 1.9 신규 추가

NEW:

Title:
- `Story 1.9: fast preview handoff와 XMP preview 교체`

Story:
- booth customer로서,
- 방금 찍은 사진이 preset-applied preview가 아직 준비되지 않았더라도 곧바로 current session에 보이길 원한다.
- 그래서 blank waiting 없이 내가 방금 찍은 shot이 맞는지 즉시 안심할 수 있다.

Acceptance Criteria:
1. Story 1.7 경로로 RAW persistence가 닫힌 뒤, helper 또는 host는 optional `fastPreviewPath`와 `fastPreviewKind`를 제공할 수 있다. 이 필드가 없더라도 capture success는 유지되어야 한다.
2. host는 fast preview가 same-session, same-capture, allowed path 규칙을 만족할 때만 canonical preview 경로로 승격한다. 이 시점에는 `preview.readyAtMs`를 채우지 않고 `Preview Waiting`을 유지한다.
3. booth의 latest-photo rail과 confirmation surface는 `Preview Waiting` 중에도 same-capture pending preview를 보여줄 수 있다. 다만 고객 상태는 preset-applied preview ready처럼 바뀌면 안 된다.
4. `darktable-cli` 기반 preset-applied preview가 준비되면 host는 같은 canonical preview 경로를 교체하고, 그때만 `previewReady`와 `readyAtMs`를 기록한다.
5. fast preview가 없거나 invalid하거나 늦더라도 booth는 기존 truthful `Preview Waiting` 경로로 안전하게 fallback 해야 하며 capture success를 실패로 승격하면 안 된다.
6. timing instrumentation은 `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs`, `fast-preview-missed`, `render-cold-start`, `render-queue-delay`를 분리해 기록해야 한다.
7. hardware validation은 same-capture correctness, burst capture queue 지연, cross-session leakage 0을 함께 검증해야 한다.

Rationale:
- 이번 리서치의 핵심 개선안은 Story 1.8 롤백이 아니라 2단계 preview 파이프라인 추가다.

#### A-3. Story 순서와 상태 제안

NEW:
- Story 1.8 상태는 `review` 유지
- Story 1.9는 승인 후 `backlog`로 추가
- Story 1.9는 Epic 1의 다음 truth-critical follow-up으로 우선순위를 높임

Rationale:
- 이미 닫힌 render truth를 다시 열지 않고, latency corrective를 독립 추적하는 편이 안전하다.

### B. PRD Proposal

**Artifact:** `prd.md`

#### B-1. Decision 2 보강

OLD:
- Capture success, preview readiness, final completion stay separate.

NEW:
- Capture success, first-visible same-capture preview, preset-applied `previewReady`, final completion은 서로 다른 booth truth다.
- booth는 `Preview Waiting` 중 same-capture fast preview를 보여줄 수 있지만, preset-applied preview ready와 혼동하면 안 된다.

Rationale:
- 리서치 결론을 제품 용어 수준에서 먼저 잠가야 이후 문서가 흔들리지 않는다.

#### B-2. FR-004 보강

OLD:
- booth-safe preview feedback becomes ready when preview becomes available.

NEW:
- current-session confidence는 same-capture fast preview로 먼저 제공될 수 있다.
- `previewReady`는 선택된 preset이 실제 적용된 booth-safe preview asset이 생성된 뒤에만 성립한다.

Rationale:
- 고객 신뢰와 preset truth를 동시에 지키는 최소 wording이 필요하다.

#### B-3. NFR-003 보강

OLD:
- successful captures show current-session preview confirmation within 5 seconds.

NEW:
- 성능 목표를 두 층으로 분리한다.
- `firstVisibleCurrentSessionImage`: healthy booth 환경에서 가능한 한 즉시 노출되도록 최적화한다.
- `presetAppliedPreviewReady`: 기존 5초 p95 목표를 유지하되, 준비 전에는 truthful `Preview Waiting`을 유지한다.

Rationale:
- 현재 NFR은 "무엇이 5초 내 보여야 하는지"를 하나의 preview 개념으로 묶어 오해를 만든다.

#### B-4. Published Preset Artifact Model 보강

OLD:
- preset artifact는 booth-safe preview/final behavior를 정의한다.

NEW:
- representative preset tile은 선택 보조 자산이다.
- capture 직후 same-capture fast preview는 latency 보호용 자산일 수 있다.
- preset-applied preview/final truth는 capture-bound published bundle이 계속 소유한다.

Rationale:
- 선택용 sample과 runtime truth를 구분해야 한다.

### C. Architecture Proposal

**Artifact:** `architecture.md`

#### C-1. Helper / ingest contract 보강

OLD:
- helper contract는 capture request, health/status, file arrival correlation 중심이다.

NEW:
- helper file-arrived contract에 optional `fastPreviewPath`, `fastPreviewKind`를 추가한다.
- host는 large image IPC 대신 filesystem handoff 원칙을 유지한 채 fast preview를 canonical preview 경로로 승격한다.

Rationale:
- 리서치가 지적한 가장 큰 구조적 공백이 바로 이 계약 부재다.

#### C-2. Session manifest / preview pipeline 보강

OLD:
- session manifest는 raw/preview/final과 render status를 분리한다.

NEW:
- MVP는 기존 manifest 구조를 최대한 유지한다.
- 단, pending preview 표시와 rendered preview ready를 운영상 구분할 수 있도록 split timing field 또는 동등 telemetry를 정의한다.
- canonical preview path는 유지하고, same path replacement를 기본 전략으로 명시한다.

Rationale:
- 구조를 갈아엎지 않고도 staged preview를 구현할 수 있다.

#### C-3. Render optimization note 추가

NEW:
- fast preview 도입 뒤에도 아래 최적화 실험을 후속 과제로 남긴다.
- `120ms` fixed delay 재검토
- preview size cap 재검토
- OpenCL/GPU 활성 검증
- `--apply-custom-presets false` A/B 검증
- render warm-up 또는 queue tuning 검토

Rationale:
- 첫 노출과 정식 XMP ready latency는 별개로 줄여야 한다.

### D. UX Proposal

**Artifact:** `ux-design-specification.md`

#### D-1. Preview Waiting 보호 흐름 보강

OLD:
- rail이 비어 있어도 정상일 수 있다고 안내한다.

NEW:
- rail 비어 있음은 fallback 경로로 유지한다.
- fast preview가 존재하면 same-capture image를 먼저 보여줄 수 있지만, 상태 copy는 계속 `Preview Waiting`을 유지한다.

Rationale:
- 고객은 blank waiting보다 "내가 찍은 사진이 맞다"는 즉시성을 더 신뢰한다.

#### D-2. Latest Photo Rail 보강

OLD:
- 촬영 성공 피드백 및 세션 내 사진 확인

NEW:
- same-capture pending image와 preset-applied ready image를 같은 자리에서 자연스럽게 교체할 수 있어야 한다.
- 별도 UI 대공사보다 상태 기준과 copy 정렬을 우선한다.

Rationale:
- 이번 변화는 구조 재설계보다 behavior refinement에 가깝다.

### E. Validation / Contract Proposal

**Artifact:** contracts + validation evidence + sprint tracking

NEW:
- 계약 문서
  - `camera-helper-sidecar-protocol`
  - `session-manifest`
  - `render-worker`
  - 위 세 문서에 fast preview optional handoff와 same-path replacement를 반영한다.
- 계측
  - `fast-preview-promote-start`
  - `fast-preview-promoted`
  - `fast-preview-invalid`
  - `preview-render-start`
  - `preview-render-ready`
  - `preview-render-queue-saturated`
- hardware validation
  - same-capture fast preview correctness
  - XMP replacement correctness
  - burst capture queue delay
  - cross-session leakage 0
- sprint tracking
  - 승인 후 `sprint-status.yaml`에 Story 1.9를 `backlog`로 추가한다.

Rationale:
- 이번 변경은 UX tweak가 아니라 계약, 계측, validation까지 함께 움직여야 안전하다.

## 6. 상위 액션 플랜

1. Planning 승인
   - Story 1.8은 유지하고 Story 1.9를 신규 corrective follow-up으로 승인한다.
2. 문서 정렬
   - PRD, Architecture, UX wording을 staged preview 기준으로 수정한다.
3. 구현 준비
   - helper optional preview contract와 host canonical preview promotion 범위를 story로 확정한다.
4. 성능 검증
   - booth hardware에서 fast preview visible time과 XMP preview ready time을 분리 측정한다.
5. 후속 최적화
   - darktable cold-start, queue, GPU/OpenCL 경로를 추가 실험한다.

## 7. 구현 핸드오프

### Scope

`Moderate`

### Recommended Recipients

- Product Owner / Scrum Master
  - Story 1.9 추가와 우선순위 조정
- Development team
  - helper optional preview handoff
  - host canonical preview promotion
  - same-path replacement
  - telemetry 분리
- Architect
  - PRD/Architecture wording과 계약 문서 정합성 확인
- QA / Operations
  - fast preview correctness와 burst latency validation

### Success Criteria

- 촬영 직후 current session에 same-capture image가 더 빨리 노출된다.
- preset-applied `previewReady` truth는 느슨해지지 않는다.
- blank waiting 시간이 줄어들어도 false-ready는 생기지 않는다.
- cross-session leakage 없이 staged preview가 동작한다.

## 8. 승인 게이트

이 제안서는 승인되었고, 현재 planning 산출물에 반영되었다.

- Proposal status: `approved`
- Approval decision: `yes`
- Approved at: `2026-04-02 10:16:48 +09:00`
- Applied changes:
  - `epics.md`에 Story 1.9 제안 반영
  - `prd.md`, `architecture.md`, `ux-design-specification.md` wording 반영
  - `sprint-status.yaml`에 Story 1.9 `backlog` 추가

## 9. 워크플로우 완료 요약

- Issue addressed: capture 저장 후 첫 가시 preview latency가 XMP render와 과하게 결합된 문제
- Change scope: Moderate
- Artifacts impacted: Epics, PRD, Architecture, UX, contracts, validation, sprint tracking
- Routed to: PO/SM + Dev + Architect + QA/Ops

Correct Course workflow complete, Noah Lee!
