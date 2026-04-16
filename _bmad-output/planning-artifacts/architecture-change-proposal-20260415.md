---
documentType: architecture-change-proposal
status: proposed
targetDocument: _bmad-output/planning-artifacts/architecture.md
project_name: Boothy
date: 2026-04-15
references:
  - _bmad-output/planning-artifacts/research/technical-boothy-preview-architecture-alternatives-research-20260414.md
  - _bmad-output/planning-artifacts/architecture.md
  - docs/contracts/local-dedicated-renderer.md
---

# Architecture Change Proposal

## 목적

이 문서는 현재 `architecture.md`를 2026-04-14 연구 결론에 맞게 보정하기 위한 변경 제안이다. 목표는 기존 문서의 큰 방향을 폐기하는 것이 아니라, preview hot path의 **primary architecture direction**을 명확히 재선언하고, 이미 검증된 계약과 운영 원칙 가운데 유지할 것과 강등할 것을 분리하는 데 있다.

## 제안 요약

`architecture.md`는 다음 방향으로 보정되어야 한다.

- primary architecture는 `local native/GPU resident full-screen lane`으로 명시한다.
- 사용자에게 닫히는 결과물은 `display-sized preset-applied truthful artifact`로 명시한다.
- `capture-bound truth/evidence contract`는 유지한다.
- `darktable-compatible truth/parity reference`는 유지한다.
- 시스템 패턴은 계속 `modular monolith + dedicated sidecar + anti-corruption adapter + branch-by-abstraction rollout`으로 유지한다.
- `existing local dedicated renderer`는 성공한 activation baseline이지만, 더 이상 primary close architecture 후보로 두지 않는다.
- `full microservices`, `broker-first hot path`, `gateway-first hot path`는 기본안으로 채택하지 않는다.
- `remote renderer / edge appliance`는 local primary path가 승인 하드웨어에서 반복적으로 KPI를 닫지 못할 때만 reserve option으로 연다.

## 변경 배경

현재 연구는 Boothy의 제품 합격선을 `same-capture preset-applied full-screen visible <= 2500ms`로 고정했다. 여기서 중요한 점은 작은 preview, raw first-visible, recent strip update, first-visible image 자체는 성공으로 세지 않는다는 것이다. 사용자가 실제로 보게 되는 닫힘 조건은 **같은 캡처의 프리셋 적용 결과물이 24인치 가로 full-screen으로 보이는 것**이다.

기존 `architecture.md`는 이미 일부 문장에서 이 전환을 반영하고 있지만, 전체적으로는 아직 `local dedicated renderer activation`과 `approved preview architecture pivot` 사이의 경계가 충분히 선명하지 않다. 따라서 이번 제안은 문서 전반에서 다음 구분을 명확히 하려는 것이다.

- 무엇이 이미 성공적으로 검증된 baseline인가
- 무엇이 앞으로의 primary architecture인가
- 무엇이 기본안이 아닌 reserve path인가

## 유지되는 아키텍처 불변 조건

다음 항목은 이번 보정 이후에도 유지되어야 한다.

### 1. Capture-Bound Truth / Evidence Contract 유지

`local-dedicated-renderer` 계약이 고정한 핵심 규칙은 계속 유지한다.

- same-capture, same-session, capture-bound preset version 유지
- booth의 `previewReady` 조기 승격 금지
- booth-safe waiting과 approved fallback 유지
- capture-bound evidence bundle 조립 가능성 유지
- parity diff는 same-capture / same-session / same-preset-version 비교에만 사용

즉, 새 primary lane으로 바뀌더라도 `truth ownership`, `promotion evidence`, `fallback reason`, `route stage`, `warm state`, `correlation identifiers`의 기준은 약화되면 안 된다.

### 2. Darktable-Compatible Truth / Parity Reference 유지

연구 결론은 darktable-compatible path를 제거하라고 말하지 않는다. 오히려 이 경로는 계속 다음 역할을 맡아야 한다.

- fidelity oracle
- parity validation reference
- booth-safe fallback path
- final/export truth reference

새 local full-screen lane은 빠른 사용자 닫힘을 책임지지만, darktable-compatible truth path는 preset fidelity와 신뢰성을 지키는 기준선으로 남는다.

### 3. 시스템 패턴 유지

Boothy의 주 시스템 패턴은 계속 다음 조합이 되어야 한다.

- modular monolith
- dedicated sidecar
- anti-corruption adapter
- branch-by-abstraction rollout

즉, core booth flow는 하나의 로컬 제품 경계 안에 남기고, camera SDK, darktable/XMP semantics, native renderer semantics는 경계 밖 adapter/sidecar에 고립시키며, rollout은 `shadow -> canary -> default -> rollback` 모델로 운영한다.

## Primary Architecture 변경 제안

### 기존 문서의 primary wording 보정

`architecture.md`는 preview hot path의 주 경로를 다음처럼 명시해야 한다.

> Boothy의 primary preview architecture는 `host-owned local native/GPU resident full-screen lane`이 생성하는 `display-sized preset-applied truthful artifact`이다.

이 문장은 단순한 성능 최적화 설명이 아니라, 사용자 경험을 닫는 공식 product architecture를 의미해야 한다.

### 새 primary lane의 책임

새 primary lane은 다음 책임을 가진다.

- capture-bound preset artifact를 소비한다
- same-capture full-screen close를 위한 display-sized truthful artifact를 생성한다
- customer booth에 booth-safe typed status만 노출한다
- truth/promotion/fallback/evidence는 host가 소유한다
- darktable-compatible path와 병렬 비교 가능해야 한다

### 문서상 강등되어야 할 항목

다음 항목은 더 이상 primary wording으로 남아 있으면 안 된다.

- `local dedicated renderer`를 final forward architecture처럼 읽히게 하는 서술
- first-visible image나 same-path replacement 자체를 sign-off close처럼 읽히게 하는 서술
- darktable-only blocking close ownership을 기본 경로처럼 읽히게 하는 서술

## 왜 Existing Local Dedicated Renderer Activation은 성공했지만 KPI를 닫지 못했는가

이 부분은 문서에 명시적으로 들어가야 한다.

### 성공한 것

기존 `local dedicated renderer activation`은 아래를 입증했다.

- dedicated renderer를 host-owned sidecar boundary로 붙일 수 있다
- same-capture / same-session / same-preset-version correlation을 유지할 수 있다
- `preview-promotion-evidence.jsonl` 중심의 capture-bound evidence를 남길 수 있다
- booth-safe fallback, waiting, warm-state vocabulary를 운영 계약으로 고정할 수 있다
- first-visible과 truthful promotion을 구분하는 운영 모델을 세울 수 있다

즉, activation은 **경로가 존재하고, 계약이 작동하고, 증적을 남길 수 있다**는 것을 증명했다.

### 닫지 못한 것

하지만 activation 성공은 곧바로 제품 KPI 달성을 의미하지 않는다. 연구 결론상 이 경로는 다음 이유로 목표를 닫는 primary architecture로는 부족했다.

- sign-off 기준은 `same-capture preset-applied full-screen visible <= 2500ms`인데, 기존 경로는 이 조건을 반복적으로 닫는 데 실패했다.
- activation evidence는 경로 존재와 promotion contract는 증명했지만, **24인치 가로 full-screen 기준의 preset-applied truthful close latency**를 닫았다는 뜻은 아니다.
- 기존 접근은 dedicated close candidate로는 유효했지만, `display-sized preset-applied truthful artifact`를 first-class artifact로 다루는 구조가 충분히 중심화되지 않았다.
- 연구는 현 상태를 더 미세조정하기보다, full-screen artifact 생성 경로 자체를 더 짧은 `local native/GPU resident lane`으로 재구성하는 편이 더 설득력 있다고 결론냈다.
- 현재 병목은 single bug 하나로 단정된 것이 아니라, render/apply hot path 길이, warm-state 유지, queue contention, same-path replacement cost가 결합된 구조 문제로 해석된다.

따라서 문서에는 다음 식의 표현이 필요하다.

> Existing local dedicated renderer activation was a successful activation baseline and evidence contract proof, but it did not close the product KPI for repeated `same-capture preset-applied full-screen visible <= 2500ms` on approved booth hardware. It is retained as baseline evidence for route activation and promotion semantics, not as the final primary close architecture.

## 왜 Full Microservices, Broker-First Hot Path, Gateway-First Hot Path는 기본안이 아닌가

이 부분도 문서에서 명시적으로 닫아야 한다.

### Full Microservices가 기본안이 아닌 이유

Boothy의 현재 핵심 문제는 multi-team cloud decomposition이 아니라 **한 대의 booth PC에서 same-capture truthful full-screen close를 2.5초 안에 만드는 것**이다. 이 상황에서 full microservices는 다음 비용을 추가한다.

- network hop
- service discovery
- distributed failure surface
- remote health and retry complexity
- 운영 복잡도 증가

이 비용은 현재 acceptance problem을 직접 줄이지 않는다. 따라서 full microservices는 현재 단계에서 과설계 가능성이 높다.

### Broker-First Hot Path가 기본안이 아닌 이유

broker-first hot path는 비동기 decoupling에는 유리할 수 있지만, Boothy의 hot path는 메시지 유연성보다 다음 속성이 더 중요하다.

- 낮은 지연
- capture correlation 보존
- operational simplicity
- booth-safe deterministic fallback

capture 직후 full-screen close를 닫아야 하는 경로에 broker를 기본으로 넣으면, 직렬화, 큐 적체, 재시도, 소비 지연, 관측 복잡도가 오히려 hot path를 길게 만들 수 있다. 이벤트와 JSONL evidence seam은 유지하되, 그것을 hot path ownership으로 승격하면 안 된다.

### Gateway-First Hot Path가 기본안이 아닌 이유

API gateway는 원격 서비스 집계와 외부 entrypoint 관리에는 유용하지만, same-PC single-booth hot path에는 부적합하다.

- 추가 hop이 늘어난다
- gateway 자체가 latency와 failure surface를 추가한다
- 현재 문제는 public API composition이 아니라 local render close다

따라서 gateway는 remote operator plane이나 reserve remote path management plane에만 의미가 있고, booth hot path의 기본 진입점이 되어서는 안 된다.

### 문서에 넣어야 할 결론 문장

> Boothy does not adopt full microservices, broker-first hot path, or gateway-first hot path as the primary booth architecture because the acceptance problem is single-booth local full-screen close latency with strict capture correlation, not distributed service decomposition.

## Remote Renderer / Edge Appliance를 언제 Reserve Option으로 여는가

`remote renderer / edge appliance`는 금지 대상이 아니라 **조건부 예비안**으로 남겨야 한다.

### Reserve Option 개방 조건

다음 조건이 함께 확인될 때만 reserve option을 연다.

1. `local native/GPU resident full-screen lane`이 승인 하드웨어에서 반복적으로 KPI를 닫지 못한다.
2. 실패 판정은 `same-capture preset-applied full-screen visible <= 2500ms` 기준으로 하며, tiny preview나 first-visible은 성공으로 계산하지 않는다.
3. capture correctness, preset fidelity, fallback stability, evidence completeness가 유지된 상태에서도 local path가 구조적으로 headroom 부족임이 확인된다.
4. local lane의 prototype, parity validation, canary rollout, warm-state tuning을 거친 뒤에도 반복적으로 미달한다.
5. remote 경로를 열더라도 capture-bound truth/evidence contract를 그대로 유지할 수 있다.

### Reserve Option이 열리면 추가되는 요구

reserve option이 현실화되면 다음을 별도 경계로 추가해야 한다.

- gRPC + Protobuf 기반 typed service contract
- mTLS 기반 transport security
- service discovery / health checking
- circuit breaker / timeout / compensating fallback
- local truthful waiting으로 즉시 내려갈 수 있는 rollback rule

즉, remote renderer는 1차 기본안이 아니라, local lane 검증 실패 후에만 여는 2차 구조다.

## architecture.md에 반영해야 할 구체 보정안

### 1. System Overview 보정

`resident GPU rendering`이라는 일반 표현을 다음 수준으로 구체화해야 한다.

- host-owned local native/GPU resident full-screen lane
- display-sized preset-applied truthful artifact
- darktable-compatible truth/parity reference

### 2. Core Architectural Decisions 보정

Critical decision에 아래 성격을 명시해야 한다.

- primary close ownership은 local native/GPU resident full-screen lane에 있다
- customer-visible close artifact는 display-sized preset-applied truthful artifact다
- local dedicated renderer activation은 retired candidate baseline이다
- darktable-compatible path는 parity/fallback/final reference다

### 3. Preview 관련 규칙 보정

문서 전반에서 아래 의미를 일관되게 맞춰야 한다.

- `previewReady`는 first-visible이 아니라 truthful full-screen artifact에만 연결된다
- truth/promotion ownership은 host에 남는다
- same-path replacement는 booth-safe replacement semantics를 유지한다

### 4. Non-Default Alternatives 명시

문서에 다음 항목을 비기본안으로 명시해야 한다.

- full microservices
- broker-first hot path
- gateway-first hot path
- darktable-only blocking close ownership

### 5. Initial Implementation Priorities 보정

현 우선순위는 아래 순서로 고정해야 한다.

1. KPI와 evidence/traces를 `same-capture preset-applied full-screen visible <= 2500ms` 기준으로 재고정
2. local native/GPU resident full-screen lane prototype
3. display-sized truthful artifact generation 및 parity validation
4. shadow -> canary -> default promotion
5. local lane 반복 실패 시에만 remote renderer / edge appliance reserve track 개방

## 제안된 문서 상태 변경

이 proposal이 수용되면 `architecture.md`의 해석은 다음처럼 바뀌어야 한다.

- `local dedicated renderer`는 성공한 activation baseline이자 evidence contract proof다.
- 하지만 primary architecture는 `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`다.
- darktable-compatible path는 truth/parity/fallback reference로 남는다.
- 시스템의 기본 형태는 계속 modular monolith이며, 분산 서비스 구조는 기본값이 아니다.
- remote renderer / edge appliance는 local path 반복 실패 이후에만 여는 reserve option이다.

## 제안 결론

이번 보정의 핵심은 아키텍처를 갈아엎는 것이 아니라, **무엇이 성공한 baseline이고 무엇이 앞으로의 primary direction인지 문서에서 오해 없이 분리하는 것**이다.

Boothy의 현재 primary architecture는 다음 한 문장으로 정리되어야 한다.

> Boothy closes the customer-visible post-capture experience through a host-owned `local native/GPU resident full-screen lane` that produces a `display-sized preset-applied truthful artifact`, while preserving `capture-bound truth/evidence contracts`, retaining a `darktable-compatible truth/parity reference`, and rolling out via `modular monolith + dedicated sidecar + anti-corruption adapter + branch-by-abstraction`.

이 문장이 현재 연구 결론과 기존 계약을 동시에 가장 정확하게 보존한다.
