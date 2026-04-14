---
workflow: correct-course
project: Boothy
date: 2026-04-12 04:40:22 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-12 04:45:50 +09:00
approval_decision: yes
scope_classification: Moderate
handoff_recipients:
  - Product Owner / Scrum Master
  - Product Manager / Architect
  - Development Team
trigger_reference: _bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md
---

# Sprint Change Proposal - GPU-first 렌더링 검증 정렬

## 0. 첨부 문서 발췌

첨부 문서 [technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md](C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\planning-artifacts\research\technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md)에서 이번 변경을 유발한 핵심 문장만 추리면 아래 세 가지다.

- `resident GPU-first architecture`가 다음 주력 구조라는 결론은 유지된다.
- `darktable`은 버릴 대상이 아니라 `baseline / fallback / parity oracle`로 재정의하는 편이 현실적이다.
- 실행 우선순위는 `canonical preset recipe 고정 -> display + preset apply GPU prototype -> ETW/WPR/WPA/PIX + parity diff` 순서가 가장 타당하다.

이번 correct-course는 위 발췌를 트리거로 사용한다. 즉 제품 범위를 바꾸는 변경이 아니라, 현재 스프린트 산출물이 검증된 기술 방향을 더 명시적으로 따르도록 정렬하는 변경이다.

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 직접 트리거: `Epic 1 / Story 1.12`, `Story 1.13`
  - 보조 트리거: architecture의 preview pivot 설명과 validation research의 결론 사이에 남아 있는 명시성 부족
- [x] 1.2 Core problem defined
  - 이슈 유형: `Technical limitation discovered during implementation` + `validated architectural correction`
  - 문제 진술: 현재 계획 문서는 preview pivot을 이미 일부 반영했지만, resident GPU-first를 `주력 lane`, darktable를 `baseline/fallback/parity oracle`, canonical preset recipe를 `공통 진실`로 잠그는 수준까지는 정렬되지 않았다.
- [x] 1.3 Evidence gathered
  - validation research는 `resident GPU-first` 방향을 재확인했다.
  - `sprint-status.yaml` 기준 `1.12`, `1.13`은 아직 `review`이며, hardware `Go`도 닫히지 않았다.
  - PRD/UX는 제품 요구를 유지하는 데 충분하지만, architecture/epics는 GPU-first 승격 기준을 더 분명히 적을 여지가 있다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - `Epic 1`은 유지 가능하지만, resident GPU-first 승격 기준을 명시하는 follow-up story가 더 필요하다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필수 아님
  - `Epic 1`에 architecture validation alignment 성격의 story 추가 필요
- [x] 2.3 Remaining epics reviewed
  - `Epic 2`, `Epic 3`: 직접 수정 불필요
  - `Epic 4`: canonical preset recipe / XMP adapter 의미를 follow-up으로 연결할 필요가 있음
  - `Epic 5`: telemetry와 fallback/parity evidence 범위를 follow-up으로 연결할 필요가 있음
- [x] 2.4 Future epic invalidation checked
  - 기존 epics를 폐기할 필요는 없다.
  - 이미 완료되었거나 review 중인 story를 버리기보다, validation이 요구하는 기준을 follow-up story와 문서 수정으로 명시하는 편이 낫다.
- [x] 2.5 Epic priority/order checked
  - 다음 우선순위는 `GPU-first 기준 정렬`이 되어야 한다.
  - 특히 `1.13` release close 이전에 canonical recipe, GPU primary lane 정의, telemetry/parity gate를 backlog 상에서 분명히 해야 한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - 제품 범위 충돌은 없다.
  - MVP 축소나 재정의는 필요 없다.
  - direct PRD 수정은 선택적이며, 이번 제안의 필수 수정 대상은 아니다.
- [x] 3.2 Architecture conflict reviewed
  - 가장 큰 영향이 있다.
  - `darktable Render Worker` 중심 표현을 `resident GPU primary + darktable baseline/fallback` 구조로 더 명확히 고쳐야 한다.
- [x] 3.3 UX impact reviewed
  - UX 수정은 불필요하다.
  - existing `Preview Waiting`, same-slot replacement, plain-language 원칙은 validation 결과와 충돌하지 않는다.
- [x] 3.4 Other artifacts reviewed
  - `epics.md` 직접 수정 필요
  - `sprint-status.yaml` 승인 후 동기화 필요
  - review 중인 Story 1.13 close 기준은 follow-up 기준을 반영하도록 업데이트 필요

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
  - Risk: Medium
- [x] 4.4 Recommended path selected
  - 선택안: `Direct Adjustment`
  - 의미: 제품 요구는 유지하고, architecture/backlog/release gate 표현을 validation 결론에 맞게 잠근다.

## 2. 이슈 요약

이번 변경의 핵심은 “GPU-first로 갈 것인가”를 다시 논쟁하는 것이 아니다. 첨부 문서는 그 논쟁을 이미 끝냈다. 핵심은 다음이다.

1. `resident GPU-first`를 명시적 주력 방향으로 문서에 잠근다.
2. `darktable`의 역할을 `baseline / fallback / parity oracle`로 다시 적는다.
3. `canonical preset recipe`와 `telemetry / parity gate`를 backlog에 별도 작업으로 분명히 만든다.

현재 산출물은 preview pivot을 이미 상당 부분 반영했지만, 아직 `darktable truth path 중심 표현`과 `GPU-first 승격 표현`이 섞여 있다. 이 상태를 그대로 두면 구현은 계속되더라도, 이후 story 해석과 release close 기준이 다시 흐려질 수 있다.

## 3. 영향 분석

### Epic 영향

- `Epic 1`
  - 직접 영향이 가장 크다.
  - preview architecture pivot을 “local dedicated renderer candidate” 수준에서 멈추지 않고, `resident GPU-first` 승격 기준까지 backlog에 드러내야 한다.
- `Epic 4`
  - publication은 계속 유지하되, published preset bundle이 장기적으로 `canonical recipe + compatibility assets`를 담는 구조라는 점을 follow-up으로 잠가야 한다.
- `Epic 5`
  - lifecycle/audit만으로는 부족하다.
  - lane owner, fallback reason, parity evidence, warm/cold seam 계측이 제품 승인 기준으로 이어져야 한다.

### Story 영향

- 유지
  - `1.12`, `1.13`의 현재 review 결과는 유효하다.
  - 이미 구현된 dual-close / guarded cutover 작업은 버릴 필요가 없다.
- 추가 필요
  - canonical preset recipe 기준 story
  - resident GPU primary lane prototype / service story
  - telemetry + parity diff + fallback evidence story
- 보정 필요
  - `1.13`의 close 기준이 “dedicated renderer cutover”에 머물지 않고, `GPU primary lane against baseline/fallback oracle` 의미를 더 분명히 담아야 한다.

### Artifact 충돌

- `prd.md`
  - 직접 수정은 선택
  - 현재 제품 약속은 유지 가능
- `architecture.md`
  - 직접 수정 필요
- `epics.md`
  - 직접 수정 필요
- `ux-design-specification.md`
  - 수정 불필요
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - 승인 후 직접 수정 필요

### Technical / Delivery 영향

- GPU-first 방향의 재논쟁 비용 감소
- darktable 역할 오해 감소
- release close 기준과 implementation sequencing 일치
- 후속 prototype과 parity 검증 범위 명확화

## 4. 권장 접근

### 권장안

`Direct Adjustment`

### 이유

- validation research가 방향을 다시 바꾸라고 말하는 것이 아니라, 기존 correction을 더 명확히 잠그라고 말하기 때문이다.
- PRD와 UX는 제품 약속을 유지하는 데 충분하다.
- 따라서 가장 비용 대비 효과가 큰 조치는 `architecture.md`, `epics.md`, `sprint-status.yaml`을 validation 문구에 맞춰 정렬하는 것이다.

### 노력 / 리스크 / 일정 영향

- 노력: Medium
- 리스크: Low-Medium
- 일정 영향:
  - 새 story 몇 개가 추가된다.
  - 하지만 기존 구현을 폐기하지 않고 follow-up으로 흡수하므로 대규모 일정 붕괴는 피할 수 있다.

## 5. 상세 변경 제안

### 5.1 Architecture

#### Proposal A: system overview를 `GPU primary + darktable fallback` 구조로 명시

**Artifact:** `architecture.md`

**OLD**

```md
Host --> RenderWorker["darktable Render Worker"]
```

```md
The approved direction is a host-owned local dedicated renderer lane that invokes the darktable truth path after raw ingest...
```

**NEW**

```md
Host --> GpuRenderService["Resident GPU Render Service"]
Host --> BaselineWorker["darktable Baseline / Fallback / Parity Worker"]
```

```md
The approved direction is a host-owned resident GPU-first render service for the display lane.
darktable remains the baseline, fallback, and parity oracle, and is not the default preview truth owner once the GPU lane is promoted.
```

**Justification**

- validation research의 핵심 결론을 architecture에 직접 반영한다.
- 현재 표현은 preview pivot을 설명하지만, darktable가 여전히 주력 truth path처럼 읽힐 여지가 남아 있다.

#### Proposal B: canonical preset recipe를 architecture의 공통 진실로 추가

**Artifact:** `architecture.md`

**OLD**

```md
Presets are published as immutable versioned artifacts with manifest metadata, preview assets, a pinned darktable version, an approved XMP template path, and separate preview/final render profiles.
```

**NEW**

```md
Presets are published as immutable versioned artifacts whose internal truth is a canonical preset recipe.
XMP and darktable-compatible assets remain compatibility, fallback, and parity assets, not the only runtime truth representation.
```

**Justification**

- validation research가 `canonical preset recipe`를 구현 최우선 순위로 제시한다.
- 이 문구가 없으면 follow-up story가 다시 XMP-centered interpretation으로 돌아갈 위험이 있다.

### 5.2 Epics / Stories

#### Proposal C: Epic 1에 GPU-first alignment follow-up stories 추가

**Artifact:** `epics.md`

**OLD**

```md
### Story 1.16: Windows desktop build-release baseline과 CI proof 설정
...
### Story 1.1: Set up initial project from starter template
```

**NEW**

```md
### Story 1.17: canonical preset recipe와 XMP adapter 기준 동결

As a owner / brand operator,
I want booth runtime과 authoring/fallback이 공유할 canonical preset recipe를 먼저 고정하고 싶다,
So that GPU lane, darktable fallback, publication bundle이 같은 룩 진실을 기준으로 움직일 수 있다.

### Story 1.18: resident GPU-first display lane prototype과 warm-state service 도입

As a owner / brand operator,
I want display lane의 기본 후보를 resident GPU-first service로 분명히 검증하고 싶다,
So that full-size preset-applied visible latency를 darktable-only 경로보다 더 직접적으로 줄일 수 있다.

### Story 1.19: ETW/WPR/WPA/PIX + parity diff 기반 승격 게이트 정착

As a owner / brand operator,
I want latency, parity, fallback evidence를 한 기준으로 수집하고 싶다,
So that renderer 승격을 체감 속도와 품질 기준으로 동시에 판단할 수 있다.
```

**Justification**

- validation research의 실행 우선순위를 backlog로 번역한다.
- 현재 story 집합은 preview pivot은 담고 있지만, `canonical recipe`, `GPU-first service`, `telemetry/parity gate`를 각각 독립 작업으로 드러내지 않는다.

#### Proposal D: Story 1.13 close 의미를 validation 결과에 맞게 보정

**Artifact:** `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`

**OLD**

```md
I want local dedicated renderer path를 guarded cutover하고 실장비 evidence로 최종 검증하고 싶다,
so that booth가 same-capture truthful preview를 목표 latency 안에서 release-safe하게 제공한다고 자신 있게 승격할 수 있다.
```

**NEW**

```md
I want resident GPU-first primary lane를 baseline/fallback oracle against 기준으로 guarded cutover하고 실장비 evidence로 최종 검증하고 싶다,
so that booth가 full-size preset-applied visible speed와 parity를 함께 만족하는 구조만 release-safe하게 승격할 수 있다.
```

**Justification**

- Story 1.13은 계속 유효하지만, validation 이후에는 비교 기준이 더 분명해졌다.
- 단순 cutover가 아니라 `GPU primary lane vs baseline/fallback oracle` 검증으로 읽혀야 한다.

### 5.3 Sprint Tracking

#### Proposal E: `sprint-status.yaml`에 follow-up story 추가 및 1.13 hold 유지

**Artifact:** `_bmad-output/implementation-artifacts/sprint-status.yaml`

**OLD**

```yaml
1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환: review
1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate: review
```

**NEW**

```yaml
1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환: review
1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate: review
1-17-canonical-preset-recipe와-xmp-adapter-기준-동결: backlog
1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입: backlog
1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착: backlog
```

**Justification**

- 현재 review 상태를 되돌릴 필요는 없지만, release close 전에 따라와야 할 follow-up을 명시해야 한다.
- sprint status가 이를 반영하지 않으면 다음 작업 우선순위가 다시 흐려질 수 있다.

### 5.4 PRD / UX

- `prd.md`
  - direct 필수 수정 없음
  - 필요하면 후속 cleanup에서 `canonical preset recipe` 용어만 보강
- `ux-design-specification.md`
  - direct 수정 불필요
  - existing `Preview Waiting`, same-slot replacement, plain-language 보호 흐름은 유지

## 6. 구현 핸드오프

### Change Scope

`Moderate`

### Handoff Recipients

- Product Manager / Architect
  - `architecture.md`를 validation 결론 기준으로 명시 보정
  - canonical recipe / GPU primary / darktable fallback 역할 정의 확정
- Product Owner / Scrum Master
  - `epics.md`에 1.17~1.19 추가
  - 1.13 close 의미와 story sequencing 재정렬
- Scrum Master
  - `sprint-status.yaml` 동기화
  - 다음 story 생성 순서를 1.17 -> 1.18 -> 1.19 -> 1.13 close 순으로 재정렬
- Development Team
  - follow-up stories 구현
  - 기존 darktable path는 fallback/oracle로 유지하면서 GPU-first prototype을 검증

### Success Criteria

- architecture가 `resident GPU-first primary lane`과 `darktable baseline/fallback/parity oracle`을 분명히 구분한다.
- epics에 `canonical recipe`, `GPU-first service`, `telemetry/parity gate` follow-up story가 추가된다.
- sprint status가 새 우선순위를 반영한다.
- `1.13`은 단순 cutover가 아니라 `GPU primary lane vs fallback oracle` release gate로 읽힌다.

## 7. PRD / MVP 영향과 액션 플랜

### MVP 영향

- MVP 범위 변경 없음
- 고객 경험 약속 변경 없음
- 구현 우선순위와 release close 기준만 보정

### 고수준 액션 플랜

1. `architecture.md` 보정
2. `epics.md`에 1.17~1.19 추가
3. `sprint-status.yaml` 동기화
4. follow-up story 생성 및 수행
5. `1.13` hardware `Go / No-Go` 재판정

## 8. 최종 메모

이번 제안은 기존 작업을 뒤엎는 재계획이 아니다. 첨부 검증 문서가 이미 확인한 결론을 현재 스프린트 산출물에 더 명확히 새기는 보정이다.

즉 바뀌는 것은 제품 약속이 아니라, “어떤 구조를 주력으로 승격할지”를 문서와 backlog에서 더 분명히 말하는 방식이다. 가장 안전한 접근은 direct adjustment이며, 승인 후 곧바로 planning artifact 보정과 sprint tracking 동기화로 이어가면 된다.

## 9. 승인 결과

- 제안 상태: `approved`
- 승인 결정: `yes`
- 반영 완료 문서:
  - `architecture.md`
  - `epics.md`
  - `sprint-status.yaml`
  - `Story 1.13` close 문구
