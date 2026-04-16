---
workflow: correct-course
project: Boothy
date: 2026-04-14 22:19:30 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-04-15 11:20:21 +09:00
approval_decision: yes
scope_classification: Major
handoff_recipients:
  - Product Manager / Architect
  - Product Owner / Scrum Master
  - Development Team
trigger_references:
  - _bmad-output/planning-artifacts/research/technical-boothy-preview-architecture-alternatives-research-20260414.md
  - _bmad-output/planning-artifacts/preview-architecture-reassessment-report-20260414.md
  - _bmad-output/planning-artifacts/implementation-readiness-report-20260413.md
  - _bmad-output/planning-artifacts/sprint-change-proposal-20260413-155159.md
---

# Sprint Change Proposal - Preview Architecture 방향 전환

## 1. Issue Summary

이번 변경 트리거는 2026-04-14 기준 최신 하드웨어 검증과 대체 architecture 연구 결과다.

- 직접 트리거 스토리: `Story 1.20` activation 결과와 `Story 1.13` release-close 재진입 판단
- 이슈 유형: `Failed approach requiring different solution`
- 핵심 문제:
  - `local dedicated renderer activation/canary`는 실제로 적용됐다.
  - 그러나 제품 합격선인 `same-capture preset-applied full-screen visible <= 2500ms`는 반복적으로 미달했다.
  - 즉 지금 문제는 “새 경로가 아직 안 켜졌다”가 아니라, **현재 dedicated close 구조가 목표 체감 속도를 닫지 못한다**는 점이다.

핵심 evidence는 아래와 같다.

- 2026-04-14 reassessment는 실제 운영 경로에서 `laneOwner=dedicated-renderer`, `routeStage=canary`, `fallbackReason=none`, `warmState=warm-hit`를 확인했다.
- 하지만 같은 reassessment는 최근 세션에서 `replacementMs=5533`, `4411`, `4455`, `3494`를 기록해 `<= 2500ms` 목표를 충족하지 못했다고 정리했다.
- 2026-04-14 alternatives research는 현 구조 미세조정보다 `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + darktable-compatible truth/parity reference`가 더 설득력 있는 방향이라고 결론냈다.

요약하면, 2026-04-13의 correct-course는 `activation gap`을 메우는 데는 성공했다. 그러나 2026-04-14의 evidence는 **activation 성공 이후에도 제품 KPI가 닫히지 않는다**는 점을 확인했고, 이제는 preview architecture의 목표 경로 자체를 바꿔야 한다.

## 2. Impact Analysis

### Epic Impact

- `Epic 1`
  - 영향이 가장 크다.
  - 기존 `1.18 -> 1.19 -> 1.20 -> 1.13` 체계는 “activation 성공”까지는 유효했지만, 더 이상 최종 해법 경로로 유지되기 어렵다.
  - 기존 dedicated renderer activation chain은 `baseline / learning / parity reference` 가치로 남기고, 새 local native/GPU resident full-screen lane 중심 story chain을 추가해야 한다.
- `Epic 2`, `Epic 3`
  - 고객 타이밍/종료 경험에는 직접 영향이 없다.
  - 다만 preview close 정의가 바뀌므로 downstream timing evidence와 post-end truth는 같은 KPI 기준에 종속된다.
- `Epic 4`
  - canonical preset recipe와 publication artifact는 더 중요해진다.
  - 새 local lane이 `display-sized truthful artifact`를 만들려면 publication bundle이 native lane과 darktable parity reference 둘 다를 안정적으로 지원해야 한다.
- `Epic 5`
  - operator diagnostics는 유지되지만, focus가 바뀐다.
  - 앞으로는 “dedicated renderer activation 여부”보다 `full-screen truthful artifact lane`, `parity drift`, `artifact promotion`, `reserve-path decision gate`를 읽을 수 있어야 한다.
- `Epic 6`
  - branch rollout / rollback governance는 계속 유효하다.
  - 새 architecture도 `shadow -> canary -> default`와 one-action rollback 규칙을 그대로 따라야 한다.

### Story Impact

- `Story 1.18`, `Story 1.19`, `Story 1.20`
  - 폐기 대상은 아니다.
  - 다만 “현재 방향의 activation/canary baseline과 evidence foundation”으로 위치가 바뀐다.
  - 새 full-screen lane 경로의 최종 sign-off owner로 읽으면 안 된다.
- `Story 1.13`
  - 유지 가능하지만 scope 변경이 필요하다.
  - 기존 dedicated renderer promoted route를 닫는 story가 아니라, **새 local native/GPU resident full-screen lane**의 final hardware close owner로 재정의해야 한다.
- 신규 story 필요
  - `Story 1.21`: display-sized preset-applied truthful artifact prototype
  - `Story 1.22`: darktable-compatible truth/parity reference 정착
  - `Story 1.23`: local full-screen lane canary/default promotion 및 reserve-path decision gate

### Artifact Conflict Analysis

- `prd.md`
  - KPI 문구가 여전히 `once the dedicated renderer path is enabled`에 묶여 있어 현재 문제 진단과 충돌한다.
  - PRD는 제품 합격선을 특정 엔진 활성화가 아니라 `same-capture preset-applied full-screen visible <= 2500ms`로 다시 고정해야 한다.
- `architecture.md`
  - 현재는 `host-owned dedicated renderer lane`이 hot path owner로 읽힌다.
  - 최신 연구는 hot path를 `display-sized truthful artifact` 중심의 local native/GPU resident full-screen lane으로 재정의하라고 요구한다.
- `epics.md`
  - 현재 sequencing note는 `1.18 -> 1.19 -> 1.20 -> 1.13`으로 닫힌다.
  - 이 순서는 activation gap 보정에는 맞았지만, 2026-04-14 evidence 이후에는 새 architecture story chain이 반영돼야 한다.
- `ux-design-specification.md`
  - blocking conflict는 없다.
  - 현재 UX는 이미 `same-capture first-visible -> later truthful close`, `Preview Waiting`, `same-slot replacement`를 허용하므로 유지 가능하다.
  - 즉 이번 pivot은 UX 재설계보다 **backend hot path와 sign-off 기준 전환**에 가깝다.
- Secondary artifacts
  - 승인 시 `sprint-status.yaml`, `hardware-validation-ledger.md`, `docs/release-baseline.md`는 새 sequence와 sign-off owner에 맞게 업데이트돼야 한다.

## 3. Recommended Approach

### Option 1: Direct Adjustment

- 평가: `Viable`
- 노력: `High`
- 리스크: `Medium`
- 의미:
  - 기존 `local dedicated renderer + different close topology`를 더 미세조정하는 대신,
  - `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + darktable-compatible truth/parity reference`로 hot path를 교체한다.

### Option 2: Potential Rollback

- 평가: `Not viable`
- 노력: `Medium`
- 리스크: `High`
- 이유:
  - darktable-default 또는 기존 path로 되돌려도 `<= 2500ms` 목표를 닫는 근거가 없다.
  - activation, route policy, evidence governance에서 이미 얻은 운영 학습만 잃게 된다.

### Option 3: PRD MVP Review

- 평가: `Not viable`
- 노력: `High`
- 리스크: `High`
- 이유:
  - 지금 단계에서 KPI를 완화하면 제품 핵심 가치가 약해진다.
  - local 대체 architecture를 충분히 시도하기 전에 목표 자체를 낮추는 것은 시기상조다.

### Selected Path

`Direct Adjustment`

단, 의미는 단순 유지보수가 아니다.  
이번 direct adjustment는 **현 경로 미세조정**이 아니라 **local-first를 유지한 채 hot path 구조를 교체하는 architecture pivot**이다.

핵심 권고는 아래와 같다.

1. 제품 sign-off KPI를 `same-capture preset-applied full-screen visible <= 2500ms`로 다시 고정한다.
2. `display-sized preset-applied truthful artifact`를 first-class artifact로 승격한다.
3. hot path는 `local native/GPU resident full-screen lane`이 담당한다.
4. `darktable-compatible truth/parity reference`는 fidelity oracle, fallback, final/export reference로 남긴다.
5. `remote renderer / edge appliance`는 reserve option으로만 유지하고, local lane 반복 실패가 검증된 뒤에만 POC를 연다.

## 4. Detailed Change Proposals

### 4.1 PRD Change Proposal

#### Proposal PRD-A: KPI row를 엔진 활성화 기준에서 제품 sign-off 기준으로 전환

**Artifact:** `prd.md`

**OLD**

```md
| Original-visible to preset-applied visible close latency | New metric | Architecture corrective target of `<= 2.5s` on approved booth hardware once the dedicated renderer path is enabled | Request-level seam logs and dedicated hardware validation review |
```

**NEW**

```md
| Same-capture preset-applied full-screen visible latency | New metric | Primary release sign-off target of `<= 2.5s` on approved booth hardware through the approved local full-screen truthful preview lane | Request-level seam logs, full-screen visible evidence, and dedicated hardware validation review |
```

**Rationale**

- 현재 문제는 dedicated renderer activation 여부가 아니라 제품 KPI 미달이다.
- PRD는 특정 구현 경로가 켜졌는지보다 고객이 실제로 2.5초 안에 same-capture preset-applied full-screen result를 보느냐를 기준으로 닫혀야 한다.

#### Proposal PRD-B: NFR-003 acceptance를 dedicated renderer owner에서 artifact-first owner로 전환

**Artifact:** `prd.md`

**OLD**

```md
- The approved preset-applied close owner may be a host-owned local dedicated renderer lane as long as it preserves same-capture correctness, preset fidelity, and preview/final truth contracts.
```

**NEW**

```md
- The approved preset-applied full-screen close owner is the host-owned local native/GPU resident full-screen lane that produces a display-sized preset-applied truthful artifact, while the darktable-compatible path remains the approved truth/parity reference and fallback path.
```

**Rationale**

- 기존 문구는 current canary path를 허용하는 수준이었지만, 이제는 새 기본 방향을 문서로 고정해야 한다.
- 같은 제품 약속을 유지하면서도 hot path와 truth/parity reference의 역할을 분리할 필요가 있다.

### 4.2 Architecture Change Proposal

#### Proposal ARCH-A: preview pipeline model을 dedicated close 중심에서 artifact-first 3-lane model로 전환

**Artifact:** `architecture.md`

**OLD**

```md
- **Preview pipeline model:** The preview pipeline is split into a `first-visible lane` and a `truth lane`. The approved next structure is `resident GPU-first primary lane + different close topology`, where the host owns one resident GPU service for preset-applied close and may still promote an approved same-capture first-visible image into the canonical preview path earlier. darktable remains the baseline, fallback, and parity oracle rather than the default preview truth owner.
```

**NEW**

```md
- **Preview pipeline model:** The preview pipeline is split into a `first-visible lane`, a `display-sized truthful artifact lane`, and a `truth/parity reference lane`. The approved next structure is `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + darktable-compatible truth/parity reference`, where the hot path is optimized for same-capture full-screen visible latency and darktable-compatible execution remains the fidelity oracle, fallback, and final/export reference.
```

**Rationale**

- 이제 hot path를 “dedicated renderer가 켜졌는가”로 정의하면 안 된다.
- 핵심은 `display-sized truthful artifact`를 빠르게 올리는 전용 lane과 parity reference를 분리하는 것이다.

#### Proposal ARCH-B: preview truth rule을 dedicated renderer output에서 full-screen truthful artifact output으로 변경

**Artifact:** `architecture.md`

**OLD**

```md
- **Preview truth rule:** `previewReady` and `readyAtMs` remain reserved for the later preset-applied truthful close produced from the capture-bound published preset artifact by the host-owned dedicated renderer lane.
```

**NEW**

```md
- **Preview truth rule:** `previewReady` and `readyAtMs` remain reserved for the later preset-applied full-screen truthful artifact produced from the capture-bound published preset artifact by the host-owned local native/GPU resident lane. darktable-compatible execution validates parity, fallback, and final/export truth without reclaiming the latency-critical hot path by default.
```

**Rationale**

- 제품 truth는 유지하되, 그 truth를 만드는 owner를 더 짧은 hot path로 바꿔야 한다.
- darktable-compatible path는 계속 필요하지만 default hot path owner일 필요는 없다.

#### Proposal ARCH-C: preset/render core rule을 darktable-backed hot path에서 canonical recipe + native lane hot path로 전환

**Artifact:** `architecture.md`

**OLD**

```md
- **Preset/render core rule:** The host-owned local dedicated renderer lane executes approved darktable-backed preset artifacts through `darktable-cli` or the same approved darktable truth path; booth routes receive only booth-safe outputs and typed status, never module-level editing APIs. If an early same-capture image was already promoted, the later preset-applied truthful output still replaces it at the same canonical path and only then advances `previewReady`.
```

**NEW**

```md
- **Preset/render core rule:** The host-owned local native/GPU resident full-screen lane consumes the canonical preset recipe to generate a display-sized preset-applied truthful artifact for the booth hot path. The darktable-compatible path remains a separate reference for parity validation, fallback, and final/export truth. Booth routes receive only booth-safe outputs and typed status, never module-level editing APIs, and same-slot replacement still closes only when the truthful artifact is ready.
```

**Rationale**

- canonical preset recipe는 유지하되, full-screen close를 darktable-heavy path에 계속 묶어 둘 이유가 줄었다.
- research가 권장한 artifact-first local lane 구조를 architecture rule로 승격해야 한다.

#### Proposal ARCH-D: initial implementation priorities를 새 pivot sequence로 교체

**Artifact:** `architecture.md`

**OLD**

```md
1. Preserve Story 1.18 prototype and Story 1.19 promotion-gate outputs as the pre-activation baseline.
2. Add an activation story that promotes approved preset scope from `shadow` to `canary/default` through host-owned `preview-renderer-policy.json` and proves resident success-path evidence on real booth sessions.
3. Run Story 1.13 only after activation proof exists, so guarded cutover and hardware `Go / No-Go` remain release-close work rather than implementation catch-up work.
4. Continue publication, timing/completion, and release-governance tracks without weakening the approved preview/final truth model.
```

**NEW**

```md
1. Freeze the product sign-off KPI at `same-capture preset-applied full-screen visible <= 2500ms` and reset traces/evidence to that outcome.
2. Treat Stories 1.18, 1.19, and 1.20 as activation baseline evidence for the retired dedicated close candidate rather than the final forward path.
3. Add a new story chain for `display-sized preset-applied truthful artifact` generation, darktable-compatible parity reference, and local full-screen lane canary/default promotion.
4. Run Story 1.13 only after the new local native/GPU resident full-screen lane passes prototype, parity, and canary gates.
5. Open `remote renderer / edge appliance` only as a reserve experiment if the local lane repeatedly fails the same KPI on approved booth hardware.
```

**Rationale**

- 2026-04-13 priorities는 activation gap을 메우는 데는 맞았지만, 2026-04-14 evidence 이후에는 더 이상 forward plan이 아니다.
- 앞으로는 `metric reset -> artifact lane -> parity gate -> canary -> final close -> reserve-only remote` 순서가 맞다.

### 4.3 Epics Change Proposal

#### Proposal EPIC-A: Epic 1 follow-up scope를 dedicated renderer follow-up에서 architecture pivot follow-up으로 확대

**Artifact:** `epics.md`

**OLD**

```md
- Epic 1에는 canonical preset recipe, current-capture full-screen close lane, `replacementMs <= 2500` 승격 게이트, telemetry/parity 증적, 그리고 route promotion/rollback authority를 위한 follow-up story가 추가되어야 한다.
```

**NEW**

```md
- Epic 1에는 local native/GPU resident full-screen lane, display-sized preset-applied truthful artifact, darktable-compatible truth/parity reference, `same-capture preset-applied full-screen visible <= 2500ms` 승격 게이트, route promotion/rollback authority, 그리고 reserve-only remote renderer decision gate를 위한 follow-up story가 추가되어야 한다.
```

**Rationale**

- Epic 1의 본질은 여전히 preview architecture다.
- 다만 follow-up 범위가 기존 dedicated renderer activation을 넘어서 새 local artifact-first lane으로 재정의되어야 한다.

#### Proposal EPIC-B: preview architecture sequencing note를 재작성

**Artifact:** `epics.md`

**OLD**

```md
### Preview Architecture Sequencing Note

- Story 1.18은 prototype owner다.
- Story 1.19는 promotion-gate establishment owner다.
- Story 1.20은 activation owner다.
- Story 1.13은 activation 완료 이후에만 수행되는 final guarded cutover / release-close owner다.
```

**NEW**

```md
### Preview Architecture Sequencing Note

- Stories 1.18, 1.19, 1.20은 현재 dedicated close candidate의 activation baseline과 governance evidence owner다.
- Story 1.21은 display-sized preset-applied truthful artifact prototype owner다.
- Story 1.22는 darktable-compatible truth/parity reference와 fidelity gate owner다.
- Story 1.23은 local native/GPU resident full-screen lane canary/default promotion owner다.
- Story 1.13은 Story 1.21~1.23 완료 이후에만 수행되는 final hardware close / release-close owner다.
- `remote renderer / edge appliance`는 Story 1.23이 반복 실패한 뒤에만 reserve experiment로 열린다.
```

**Rationale**

- 현재 sequence는 activation 성공까지는 맞지만, 그 자체로는 더 이상 제품 목표를 닫지 못한다.
- 새 sequence는 기존 학습을 버리지 않으면서 forward path를 분명히 분리한다.

#### Proposal EPIC-C: 신규 story 3개를 Epic 1에 추가

**Artifact:** `epics.md`

**OLD**

```md
### Story 1.20: resident preview lane activation과 route policy promotion
...
### Preview Architecture Sequencing Note
...
```

**NEW**

```md
### Story 1.21: display-sized preset-applied truthful artifact prototype

As a owner / brand operator,
I want same-capture full-screen close를 위한 display-sized preset-applied truthful artifact를 local native/GPU lane에서 만들고 싶다,
So that booth가 final/export truth와 분리된 짧은 hot path로 `<= 2500ms` 목표에 접근할 수 있다.

### Story 1.22: darktable-compatible truth/parity reference 정착

As a owner / brand operator,
I want new artifact lane의 결과를 darktable-compatible reference와 비교 가능한 기준으로 잠그고 싶다,
So that speed를 올리면서도 preset fidelity drift 없이 booth-safe truth를 유지할 수 있다.

### Story 1.23: local full-screen lane canary/default promotion과 reserve-path decision gate

As a owner / brand operator,
I want new local full-screen lane을 shadow -> canary -> default 순서로 승격하고 reserve-path 개시 기준도 함께 고정하고 싶다,
So that local-first 원칙을 유지하면서도 반복 실패 시에만 remote renderer / edge appliance를 열 수 있다.
```

**Rationale**

- activation gap은 이미 메워졌다.
- 이제 필요한 것은 새 architecture 자체를 구현하고 검증하는 owner chain이다.

### 4.4 Story Change Proposal

#### Proposal STORY-A: Story 1.20을 current path activation baseline으로 재분류

**Artifact:** `_bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md`

**OLD**

```md
Activation Ownership Note: Story 1.18이 resident GPU-first 후보와 warm-state evidence를 만들었고, Story 1.19가 promotion evidence gate를 고정했다. 이번 스토리는 그 기반을 실제 booth 운영 경계로 승격하는 activation owner다. canonical release close owner는 계속 Story 1.13이며, 이번 스토리만으로 hardware `Go`를 주장하면 안 된다.
```

**NEW**

```md
Activation Ownership Note: Story 1.20은 기존 dedicated close candidate를 실제 booth 운영 경계까지 올린 activation baseline owner다. 이 스토리의 산출물은 이후 architecture pivot에서 baseline evidence와 governance foundation으로 재사용되며, 더 이상 current forward path의 final sign-off 근거로 사용되지 않는다. canonical release close owner는 계속 Story 1.13이지만, 그 대상 lane은 Story 1.21~1.23이 만드는 새 local native/GPU resident full-screen lane이다.
```

**Rationale**

- Story 1.20을 실패로 처리할 필요는 없다.
- 다만 forward path owner로 계속 읽으면 잘못된 방향이 반복된다.

#### Proposal STORY-B: Story 1.13 prerequisite와 close 대상을 새 lane 기준으로 교체

**Artifact:** `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`

**OLD**

```md
Architecture Sequencing Note: 2026-04-13 승인된 preview activation 보정 이후, preview architecture adoption 순서는 `1.18 prototype -> 1.19 gate establishment -> 1.20 activation -> 1.13 guarded cutover / release close`로 고정된다. 이번 스토리는 resident lane를 새로 활성화하는 단계가 아니라, Story 1.20이 만든 promoted route를 승인된 부스 장비에서 최종 `Go / No-Go`로 닫는 canonical release-close owner다.
```

**NEW**

```md
Architecture Sequencing Note: 2026-04-14 preview architecture pivot 이후, preview architecture adoption 순서는 `1.18/1.19/1.20 baseline evidence -> 1.21 display-sized truthful artifact prototype -> 1.22 darktable-compatible parity reference -> 1.23 local full-screen lane canary/default -> 1.13 guarded cutover / release close`로 고정된다. 이번 스토리는 기존 dedicated route를 닫는 단계가 아니라, 새 local native/GPU resident full-screen lane이 approved booth hardware에서 `same-capture preset-applied full-screen visible <= 2500ms`를 만족하는지 최종 `Go / No-Go`로 닫는 canonical release-close owner다.
```

**Rationale**

- Story 1.13은 계속 최종 hardware close owner로 유지하는 편이 맞다.
- 다만 무엇을 닫는지의 대상이 바뀌어야 한다.

#### Proposal STORY-C: 신규 story chain 생성

**Artifact:** `new implementation stories`

**OLD**

```md
No dedicated implementation stories exist for the artifact-first replacement lane.
```

**NEW**

```md
Story 1.21: display-sized preset-applied truthful artifact prototype
- 핵심 AC: same-capture full-screen artifact 생성, display-sized hot path, booth-safe same-slot replacement 유지

Story 1.22: darktable-compatible truth/parity reference 정착
- 핵심 AC: parity diff 기준선, fidelity drift gate, fallback/oracle 역할 분리

Story 1.23: local full-screen lane canary/default promotion과 reserve-path decision gate
- 핵심 AC: local lane shadow/canary/default 승격, `<= 2500ms` + correctness + stability gate, local 반복 실패 시에만 remote reserve POC 허용
```

**Rationale**

- 현재 story set에는 새 architecture를 실제로 만들 owner가 없다.
- 새 lane은 prototype, parity, canary의 세 단계로 분리돼야 한다.

## 5. Scope Classification

`Major`

이유는 아래와 같다.

- PRD KPI 문구 수정이 필요하다.
- architecture hot path owner가 바뀐다.
- Epic 1 sequence가 재작성된다.
- 새 story chain이 필요하다.
- release-close owner는 유지하되 close 대상이 바뀐다.

즉 backlog 조정 수준을 넘어 **기본 architecture execution plan을 다시 짜야 하는 변경**이다.

## 6. Handoff Recipients

### Product Manager / Architect

- preview architecture 방향 전환 승인
- PRD KPI와 architecture hot path definition 확정
- local-first / reserve-remote 경계 승인

### Product Owner / Scrum Master

- Epic 1 story sequence 재정렬
- `Story 1.21`, `Story 1.22`, `Story 1.23` 추가
- `Story 1.13`, `Story 1.20` 역할 재정의
- 승인 후 `sprint-status.yaml` 반영

### Development Team

- 새 local native/GPU resident full-screen lane 구현
- display-sized truthful artifact와 parity reference 분리
- 기존 1.18~1.20 evidence를 baseline 비교군으로 활용

## 7. Final Recommendation

이번 제안의 핵심은 “기존 correct-course가 틀렸다”가 아니다.  
그 보정은 activation gap을 닫는 데는 정확했다.

하지만 2026-04-14 기준 evidence는 이제 다음 결론을 강제한다.

- 현재 dedicated renderer activation/canary는 **성공했다**
- 그러나 그것만으로는 `same-capture preset-applied full-screen visible <= 2500ms`를 **반복 충족하지 못했다**
- 따라서 다음 스프린트의 목표는 **현재 경로 미세조정**이 아니라 **local native/GPU resident full-screen lane + display-sized truthful artifact 중심의 architecture pivot**이어야 한다
- `remote renderer / edge appliance`는 여전히 **reserve only**로 유지하는 것이 맞다

## 8. Approval Request

이 제안은 현재 `approved` 상태다.

승인에 따라 다음 액션은 아래 순서로 진행하는 것이 적절하다.

1. `prd.md`, `architecture.md`, `epics.md` planning correction
2. `Story 1.20`, `Story 1.13` 재정의
3. `Story 1.21` ~ `Story 1.23` 생성
4. `sprint-status.yaml` 및 release-governance artifact 동기화
