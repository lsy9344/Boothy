---
workflow: correct-course
project: Boothy
date: 2026-04-04 23:20:52 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-04 23:26:55 +09:00
approval_decision: yes
trigger_reference:
  - _bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-2026-04-04.md
  - _bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-plan-2026-04-04.md
  - history/recent-session-thumbnail-speed-log-2026-04-04.md
supersedes:
  - _bmad-output/planning-artifacts/sprint-change-proposal-20260404-010751.md
---

# Sprint Change Proposal - thumbnail close path 구조 전환 승인안

## 0. 워크플로우 프레이밍

- 이번 correct-course는 사용자가 제출한 2026-04-04 리서치 3종을 직접 트리거 입력으로 사용했다.
- 사용자는 `batch` 모드를 선택했고, 본 문서는 전체 변경안을 한 번에 검토할 수 있도록 구성했다.
- 이번 변경은 기존 `Story 1.10: known-good preview lane 복구와 상주형 first-visible worker 도입`을 무효화하는 제안이 아니다.
- 대신 `1.10`을 계속 진행 가능한 안정화/계측 트랙으로 유지하면서, 그 위에 `local dedicated renderer sidecar + darktable fallback` 구조 실험을 새 backlog 항목으로 승격할지 판단하기 위한 제안이다.

검토 문서:

- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
- `docs/contracts/render-worker.md`
- `_bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-2026-04-04.md`
- `_bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-plan-2026-04-04.md`
- `history/recent-session-thumbnail-speed-log-2026-04-04.md`
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260404-010751.md`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 현재 직접 트리거는 Epic 1의 thumbnail corrective track, 그중에서도 `Story 1.10` 이후 단계다.
  - 이전 change proposal이 `same engine, different topology` 수준의 corrective를 승인했다면, 이번 리서치는 그 다음 질문인 `close hot path 자체를 별도 renderer로 분리할지`를 다룬다.
- [x] 1.2 Core problem defined
  - 이슈 유형: 구현 중 드러난 기술 한계 + 구조 전환 필요 여부 판단
  - 문제 진술: same-capture `first-visible`은 이미 고객이 체감할 수준까지 일부 개선됐지만, 고객이 실제로 기다리는 `preset-applied truthful close`는 여전히 목표를 닫지 못하고 있다. 현재 트랙을 더 미세 조정하는 것만으로는 제품 기준을 안정적으로 만족시키기 어렵고, `host truth 유지 + close hot path 분리` 구조 전환 승인이 필요하다.
- [x] 1.3 Evidence gathered
  - latest hardware log에서 `request-capture -> fast-preview-visible`은 대체로 `3s ~ 4s`
  - 같은 캡처의 `capture_preview_ready`는 `9.7s ~ 10.4s`까지 재상승
  - 일부 구간은 첫 preview render 실패 후 재시도로 close가 지연됨
  - latest speed log 해석상 병목은 thumbnail 발견이 아니라 truthful close 이후 구간
  - 2026-04-04 research 결론은 `local dedicated renderer sidecar + darktable fallback`을 1차 권장안으로 제시

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 여전히 유효하다.
  - 다만 현재 `1.10`만으로는 최종 latency 목표를 닫는 스토리로 보기 어렵고, baseline 안정화/계측 복구의 역할로 좁혀 읽는 것이 안전하다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필요 없다.
  - Epic 1에 `local dedicated renderer sidecar` 구조 실험용 신규 story를 추가하는 편이 적절하다.
- [x] 2.3 Remaining epics reviewed
  - Epic 2는 고객 경험 정의를 유지한다. 구조 변경에도 latest-photo rail / Preview Waiting 의미는 바뀌지 않는다.
  - Epic 3은 직접 영향은 낮지만 post-end handoff 이전까지 preview truth owner 원칙 유지가 중요하다.
  - Epic 5/6은 diagnostics, canary rollout, rollback 거버넌스 언어 보강이 필요하다.
- [x] 2.4 Future epic invalidation checked
  - 무효화되는 epic은 없다.
  - MVP 축소도 필요 없다.
- [x] 2.5 Epic priority/order checked
  - `1.10`은 `in-progress`를 유지하되 역할을 baseline stabilization / seam completion으로 명확히 한다.
  - 신규 `1.11`을 Epic 1의 다음 구조 실험 story로 추가하고 우선순위를 높인다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD 핵심 방향과 충돌하지 않는다.
  - 다만 `approved low-latency worker output` 수준의 표현만으로는 부족하고, host가 truth owner인 상태에서 `renderer route`를 바꿀 수 있다는 제품 기준을 더 명시해야 한다.
- [x] 3.2 Architecture conflict reviewed
  - Architecture는 현재 `darktable-cli` 중심 wording이 강하다.
  - 구조 전환 승인 이후에는 `routeable render topology`, `renderer adapter`, `feature-gated routing`, `darktable fallback` 기준이 추가돼야 한다.
- [x] 3.3 UX impact reviewed
  - UX 전면 개편은 필요 없다.
  - 다만 구조가 바뀌어도 고객 상태는 계속 `Preview Waiting` 한 가지로 유지된다는 보호 원칙을 더 잠가야 한다.
- [x] 3.4 Other artifacts reviewed
  - `docs/contracts/render-worker.md`
  - 신규 `local renderer adapter` 계약 문서
  - `sprint-status.yaml`
  - Story 1.10 / 신규 Story 1.11 스토리 파일
  - hardware validation / canary evidence 기준

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Partially viable
  - Effort: Medium
  - Risk: Medium
  - 현재 문서와 story만 조금 고치는 수준으로는 부족하고, 구조 실험 스토리와 계약 추가가 함께 필요하다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
  - `1.10` 이전으로 후퇴하면 지금까지 확보한 seam instrumentation, worker warm-up, fallback 자산을 함께 잃는다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Medium
  - 이번 문제는 MVP 축소가 아니라 동일 MVP를 어떤 topology로 만족시킬지의 문제다.
- [x] 4.4 Recommended path selected
  - 선택안: `Hybrid`
  - 설명: `1.10`은 안정화/계측 트랙으로 계속 마무리하고, `1.11`에서 구조 실험을 독립 승인한다.
  - 이유: 이미 진행 중인 corrective work를 살리면서도, 제품 목표를 닫기 위한 새 topology 실험을 backlog에서 명확히 분리할 수 있기 때문이다.

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
  - 승인 일시: 2026-04-04 23:26:55 +09:00
- [x] 6.4 `sprint-status.yaml` update completed
- [x] 6.5 Final handoff confirmation recorded

## 2. 이슈 요약

현재 Boothy의 recent-session thumbnail 문제는 더 이상 "first-visible을 조금 더 빠르게"의 문제가 아니다.

최신 증거는 두 가지를 명확히 보여준다.

1. 고객이 같은 컷을 처음 보는 시점은 이미 `3초대`까지 내려온 적이 있다.
2. 그러나 고객이 실제로 기다리는 `preset-applied truthful close`는 여전히 `10초` 전후까지 흔들릴 수 있다.

즉 지금 필요한 판단은 UI 개선이나 이미지 품질 미세 조정이 아니라, `close hot path`를 별도 renderer route로 분리해도 제품 계약을 유지할 수 있는지에 대한 승인이다.

이번 리서치의 결론은 분명하다.

- `full platform pivot`은 아직 과하다.
- 하지만 `same engine, different topology`만으로 끝내기에도 한계 신호가 크다.
- 따라서 다음 제품 액션은 `Rust/Tauri host 유지 + local dedicated renderer sidecar + darktable fallback`을 구조 실험안으로 승인하는 것이다.

## 3. 영향 분석

### Epic 영향

- Epic 1
  - `1.10`은 계속 유효하지만, "최종 해결책"이 아니라 `baseline stabilization / seam completion` 트랙으로 읽어야 한다.
  - 신규 `1.11`이 필요하다.
  - `1.11`은 `local dedicated renderer sidecar`, `feature-gated routing`, `darktable fallback`, `canary evidence`를 소유한다.
- Epic 2
  - latest-photo rail / Preview Waiting 보호 경험은 유지된다.
  - same-slot replacement 규칙도 유지된다.
- Epic 5
  - operator diagnostics는 renderer route, fallback reason, close owner 비교를 다룰 수 있어야 한다.
- Epic 6
  - rollout/rollback 거버넌스는 renderer route canary와 즉시 fallback을 포함해야 한다.

### PRD / 아키텍처 / UX 충돌 여부

- PRD 충돌 없음
  - 고객 약속은 유지된다.
  - 다만 host가 truth owner인 상태에서 렌더 경로를 바꿀 수 있다는 운영 기준이 추가돼야 한다.
- Architecture 부분 수정 필요
  - `darktable-cli only`처럼 읽히는 지점을 `routeable topology`로 바꿔야 한다.
  - host 뒤의 adapter, candidate result producer, feature-gated routing을 문서화해야 한다.
- UX 전면 수정 불필요
  - 고객은 renderer route를 알 필요가 없다.
  - 다만 "같은 컷이 먼저 보이고 나중에 더 정확한 결과로 안정화된다"는 약속이 어떤 route에서도 유지돼야 한다.

### 기술 / 운영 영향

- 새 renderer binary/sidecar 패키징 필요
- host validation, fallback, route selection 정책 필요
- session seam log에 `route`, `close owner`, `fallback reason` 필드 보강 필요
- booth 단위 canary / instant rollback 기준 필요

## 4. 권장 접근

### Chosen Path

`Hybrid`

### Why This Path

- 이미 진행 중인 `1.10` 자산을 버리지 않는다.
- 제품 계약은 유지하면서 구조 실험만 별도 story로 승격할 수 있다.
- 실패 시 darktable fallback으로 빠르게 되돌릴 수 있다.
- 장기 옵션인 edge appliance로 너무 빨리 점프하지 않아도 된다.

### Scope Classification

`Major`

### Timeline Impact

1. `1.10`은 seam completeness와 baseline stabilization을 마무리한다.
2. `1.11`을 추가해 local renderer adapter와 routing canary를 구현한다.
3. PRD / Architecture / Contract / Sprint Tracking을 새 topology 기준으로 보강한다.
4. hardware canary로 sidecar vs fallback close 성능과 fidelity를 비교한다.

## 5. 상세 변경 제안

### A. Stories / Epic Proposal

**Artifact:** `_bmad-output/planning-artifacts/epics.md`

#### A-1. Story 1.10 역할 재명시

Story: `1.10 known-good preview lane 복구와 상주형 first-visible worker 도입`
Section: Scope Note

OLD:
- 이 스토리가 preview latency 구조 변경 전반의 최종 corrective처럼 읽힌다.

NEW:
- 이 스토리는 `first-visible lane` 안정화, per-session seam completeness, truthful fallback baseline을 마무리하는 precondition story로 유지한다.
- `preset-applied truthful close`의 새 owner route 실험은 후속 Story 1.11이 맡는다.

Rationale:
- 이미 진행 중인 작업을 버리지 않으면서도, 이번 구조 전환 범위를 명확히 분리할 수 있다.

#### A-2. Story 1.11 신규 추가

Story: `1.11 local dedicated renderer sidecar와 truthful preview close canary routing`
Section: New Story

NEW:

As a booth customer,
I want the booth to close my preset-applied preview through the fastest approved route without weakening truth,
So that I see the final trustworthy booth preview sooner while the product can still fall back safely if the new renderer route is not healthy.

Acceptance Criteria:

1. host는 capture-bound preset artifact와 session-scoped paths를 유지한 채, approved local dedicated renderer sidecar를 `candidate result producer`로 호출할 수 있어야 한다.
2. `previewReady` truth owner는 계속 host이며, host는 sidecar가 만든 canonical preview output을 검증한 뒤에만 readiness를 올릴 수 있어야 한다.
3. renderer routing은 booth/session/preset 단위 feature gate를 지원해야 하며, 비정상 시 darktable fallback으로 즉시 내려갈 수 있어야 한다.
4. same-slot replacement, cross-session isolation, `Preview Waiting` truth는 새 renderer route에서도 그대로 유지돼야 한다.
5. diagnostics는 same session seam package 안에서 `route selected`, `close owner`, `fallback reason`, `elapsedMs`를 비교 가능하게 남겨야 한다.
6. hardware canary는 `truthful preview close` 단축, preset fidelity 유지, false-ready 0건, rollback 즉시성까지 함께 증명해야 한다.

Rationale:
- 이번 리서치가 실제로 권장한 구조 실험은 resident first-visible worker가 아니라 local dedicated renderer sidecar다.

### B. PRD Proposal

**Artifact:** `_bmad-output/planning-artifacts/prd.md`

#### B-1. Published Preset Artifact Model 보강

Section: `Published Preset Artifact Model`

OLD:
- A same-capture first-visible image may appear earlier to reduce blank waiting, and that first-visible source may come from a fast preview, camera thumbnail, intermediate preview, or approved low-latency worker output.
- Preset-applied `previewReady` truth still comes only from the published artifact's render behavior for that capture-bound preset version.

NEW:
- A same-capture first-visible image may appear earlier to reduce blank waiting, and that first-visible source may come from a fast preview, camera thumbnail, intermediate preview, or approved low-latency worker output.
- Preset-applied `previewReady` truth still comes only from the host-validated render behavior for that capture-bound preset version, even when the host routes execution through an approved alternative local renderer adapter.
- Approved renderer routing must preserve same-slot replacement, capture-bound preset versioning, and darktable-safe fallback behavior.

Rationale:
- 현재 PRD는 low-latency worker까지는 허용하지만, `alternative renderer adapter`와 `host-validated route` 개념은 아직 직접적으로 못 읽는다.

#### B-2. NFR-003 운영 기준 보강

Section: `NFR-003 Booth Responsiveness and Preview Readiness`

OLD:
- Performance is measured on approved branch hardware with request-level seam evidence that distinguishes first-visible latency from preset-applied readiness latency.

NEW:
- Performance is measured on approved branch hardware with request-level seam evidence that distinguishes first-visible latency from preset-applied readiness latency.
- When multiple approved renderer routes exist, the product must compare route-specific close latency, fallback rate, and preset fidelity without weakening preview truth or customer-safe waiting behavior.

Rationale:
- 구조 전환 이후에는 단순 p95만이 아니라 `route 비교 기준`이 있어야 한다.

#### B-3. Release Gates 보강

Section: `Release Gates`

OLD:
- Approved branch hardware can sustain preview-readiness targets under the approved render path.

NEW:
- Approved branch hardware can sustain preview-readiness targets under the approved render path.
- Any newly introduced renderer route must support booth-scoped canary, instant fallback to the approved darktable path, and no increase in false-ready or cross-session leakage incidents.

Rationale:
- 이번 change는 실험 승인과 rollback 안전성이 제품 기준에 직접 들어와야 한다.

### C. Architecture Proposal

**Artifact:** `_bmad-output/planning-artifacts/architecture.md`

#### C-1. Preset/render core rule 갱신

Section: `API & Communication Patterns`

OLD:
- The Rust render worker executes approved darktable-backed preset artifacts through `darktable-cli`; booth routes receive only booth-safe outputs and typed status, never module-level editing APIs.

NEW:
- The Rust host owns render truth and may route preview-close execution through either the approved darktable path or an approved local dedicated renderer adapter behind the same capture-bound contract.
- Any alternative renderer route acts as a candidate-result producer behind the host, not as an independent truth owner.
- Booth routes still receive only booth-safe outputs and typed status, never renderer-internal editing APIs.

Rationale:
- 구조 전환의 핵심은 엔진을 노출하지 않고 routeable topology를 허용하는 것이다.

#### C-2. Data / Telemetry 모델 보강

Section: `Data Architecture` and `Latency telemetry rule`

OLD:
- Preview instrumentation should distinguish fast-preview visibility, render-backed preview readiness, cold-start delay, and render queue delay.

NEW:
- Preview instrumentation should distinguish fast-preview visibility, render-backed preview readiness, cold-start delay, render queue delay, selected renderer route, fallback reason, and close-owner outcome.
- One recent approved session package must be enough to compare sidecar route and darktable fallback behavior without joining mixed global logs.

Rationale:
- 구조 실험의 성공 여부는 세션 한 개 기준으로 닫혀야 한다.

#### C-3. Closed Contract Freeze Baseline 보강

Section: `Closed Contract Freeze Baseline`

OLD:
- Render-worker contract만 frozen baseline으로 읽힌다.

NEW:
- `render-worker contract`에 더해 `local renderer adapter contract`, `renderer routing policy`, `canary/rollback evidence`를 새 frozen surface로 포함한다.

Rationale:
- 새 route는 임시 옵션이 아니라 관리 가능한 계약이어야 한다.

### D. UX Proposal

**Artifact:** `_bmad-output/planning-artifacts/ux-design-specification.md`

#### D-1. Preview Waiting 보호 흐름 보강

Section: `Preview Waiting 보호 흐름`

OLD:
- first-visible source가 fast preview, intermediate preview, 또는 상주형 worker output으로 바뀌더라도 고객에게는 같은 컷이 먼저 보이고 나중에 더 정확한 결과로 안정화된다는 경험이 유지되어야 한다.

NEW:
- first-visible source나 truthful close route가 fast preview, intermediate preview, resident worker, 또는 approved local renderer route로 바뀌더라도 고객에게는 같은 컷이 먼저 보이고 나중에 더 정확한 결과로 안정화된다는 경험이 유지되어야 한다.
- renderer route 변경은 새로운 고객 상태 이름을 만들지 않는다. 고객 상태는 계속 `Preview Waiting`과 later ready 상태만 사용한다.

Rationale:
- 구조는 바뀌지만 고객 약속은 그대로여야 한다.

### E. Contract / Sprint Tracking Proposal

**Artifact:** `docs/contracts/render-worker.md`

#### E-1. render-worker 계약 확장

OLD:
- resident/speculative worker가 같은 capture의 preset-applied preview file을 성공적으로 만들었다면, 그 시점이 곧 truthful `previewReady` close다.

NEW:
- local dedicated renderer adapter는 host 뒤의 candidate-result producer로만 동작한다.
- host validation 없이 sidecar가 직접 `previewReady`를 소유하면 안 된다.
- routing policy, fallback reason, route-specific evidence가 같은 session package에 남아야 한다.

Rationale:
- 이번 구조 전환에서 가장 중요한 ownership rule이다.

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

#### E-2. sprint tracking 업데이트

OLD:
- `1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입: in-progress`

NEW:
- `1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입: in-progress`
- `1-11-local-dedicated-renderer-sidecar와-truthful-preview-close-canary-routing: backlog`

Rationale:
- 이미 진행 중인 1.10은 유지하고, 구조 실험은 새 story로 명시해야 backlog와 검증이 꼬이지 않는다.

## 6. 상위 액션 플랜

1. `1.10`을 seam completeness / baseline stabilization 관점에서 마무리한다.
2. `1.11`을 새로 생성해 local renderer adapter contract와 feature-gated routing을 정의한다.
3. PRD / Architecture / UX / render-worker contract를 새 topology 기준으로 보강한다.
4. `sprint-status.yaml`에 `1.11` backlog를 반영한다.
5. booth-scoped hardware canary로 darktable fallback 대비 close latency, fidelity, fallback rate를 비교한다.

## 7. 구현 핸드오프

### Scope

`Major`

### Recommended Recipients

- Product Manager / Architect
  - 구조 전환 승인
  - local renderer adapter boundary와 rollout 기준 확정
- Product Owner / Scrum Master
  - Epic 1 story sequencing 조정
  - `1.10 -> 1.11` handoff 기준 정리
- Development team
  - sidecar adapter 구현
  - feature-gated routing
  - darktable fallback 및 route-specific diagnostics
- QA / Ops
  - booth canary
  - fidelity validation
  - instant rollback drill

### Success Criteria

- `truthful preview close`가 darktable fallback 대비 의미 있게 단축된다.
- `Preview Waiting` truth와 same-slot replacement는 유지된다.
- false-ready와 cross-session leakage는 0건이다.
- booth/session/preset 단위 canary와 즉시 fallback이 실제로 동작한다.
- 최신 approved session 1개만 봐도 route 선택, fallback, close 결과를 닫을 수 있다.

## 8. 승인 게이트

현재 상태:

- Proposal status: `approved`
- Approval decision: `yes`
- Batch review requested by user: `yes`
- Approved at: `2026-04-04 23:26:55 +09:00`
- Applied actions:
  - 계획 문서에 `1.11` 구조 실험 story 기준 반영
  - `sprint-status.yaml`에 `1.11` backlog 반영
  - 계약/PRD/아키텍처/UX wording 정렬

## 9. 워크플로우 완료 전 요약

- Issue addressed: thumbnail first-visible 이후 close hot path의 구조 한계
- Proposed scope: Major
- Artifacts impacted: Epics, PRD, Architecture, UX, Render Worker Contract, Sprint Tracking
- Planned route: PM/Architect + PO/SM + Dev + QA/Ops
