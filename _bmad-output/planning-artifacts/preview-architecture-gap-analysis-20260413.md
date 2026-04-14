# Preview Architecture Gap Analysis

Date: 2026-04-13  
Author: Codex analysis report  
Status: Draft analysis for backlog and architecture correction

## 목적

이 문서는 preview architecture 변경 이후에도 제품 체감 속도가 기대만큼 개선되지 않은 이유를 문서 구조, story 분해, 운영 구성 관점에서 분석하고, 다음 보정 방향을 제안한다.

이 문서의 초점은 코드 세부 구현이 아니라 다음 질문에 답하는 것이다.

1. 왜 중요한 성능 리스크가 architecture 변경 후 story 구조에 충분히 반영되지 않았는가
2. 어떤 기술/구성 요소가 planning에는 있었지만 execution story에서는 빠졌는가
3. 목표 달성을 위해 story를 새로 만들어야 하는가, 기존 story를 수정해야 하는가

## 분석 범위

- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
- `_bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md`
- `_bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `docs/release-baseline.md`
- `docs/runbooks/preview-promotion-evidence-package.md`
- `C:\Users\KimYS\Pictures\dabi_shoot\branch-config\preview-renderer-policy.json`
- 최근 booth runtime session evidence

## 현재 판단 요약

- architecture 방향 자체는 크게 틀리지 않다.
- 하지만 story slicing이 `prototype -> activation -> validation`의 세 단계를 가져야 할 것을 사실상 `prototype -> validation`으로 압축했다.
- 그 결과 resident GPU-first 방향은 planning 문서에는 존재하지만, 실제 제품 경로에서 그것을 활성화하고 운영 가능한 상태로 만드는 execution owner가 비어 있다.
- 현재 preview performance 이슈는 “측정이 부족해서”보다 “새 lane이 아직 실제 primary lane으로 작동하지 않아서”에 더 가깝다.
- 따라서 현재 문제는 단순 성능 튜닝 누락이 아니라, architecture adoption 단계의 backlog 누락이다.

## 근거 요약

### 1. Architecture와 epics는 목표를 분명히 말한다

`architecture.md`는 다음 방향을 고정한다.

- preview pipeline은 `first-visible lane`과 `truth lane`으로 분리된다.
- approved next structure는 `resident GPU-first primary lane + different close topology`다.
- darktable는 baseline, fallback, parity oracle로 남아야 한다.
- preset-applied truthful close owner는 host-owned resident GPU lane이어야 한다.

`epics.md`도 같은 방향을 반복한다.

- preview/render 핵심 구조는 resident GPU-first primary lane 기준이다.
- truthful close owner는 host-owned resident GPU lane이다.
- 즉시 목표는 실장비 기준 `original visible -> preset-applied visible <= 2.5s`에 가까운 구조를 입증하는 것이다.

즉 제품 목표와 architecture 방향은 모호하지 않다.

### 2. 그러나 implementation story로 내려오면 성격이 달라진다

`Story 1.18`은 resident GPU-first lane을 “prototype”으로 정의한다.

- final promotion을 닫지 않는다.
- hardware `Go`를 주장하지 않는다.
- Story 1.13 ownership을 유지한다.

`Story 1.19`는 측정/판정 gate를 고정하는 story다.

- latency, parity, fallback evidence를 한 기준으로 모은다.
- 최종 promotion을 닫지 않는다.
- Story 1.13이 canonical close owner라고 명시한다.

`Story 1.13`은 guarded cutover와 hardware validation owner다.

- shadow -> canary -> default 순서를 요구한다.
- hardware evidence, rollback evidence, `Go / No-Go`를 소유한다.
- 하지만 story 성격상 implementation corrective owner라기보다 final release-truth owner에 가깝다.

결과적으로 story 구조는 아래처럼 된다.

- 1.18: 후보 lane prototype
- 1.19: 판정 체계
- 1.13: 최종 cutover / hardware close

여기서 빠진 것은 `prototype을 실제 primary lane으로 활성화하는 implementation story`다.

## 누락된 패턴 분석

### 패턴 1. Prototype과 activation 사이의 owner 부재

현재 구조에서 가장 큰 공백이다.

planning은 resident GPU-first를 primary lane으로 말하지만, story는 resident lane을 “후보”로만 올리고 바로 검증 단계로 넘어간다.

그 결과 아래 항목을 직접 소유하는 story가 없다.

- shadow route를 벗어나 canary/default route로 올리는 implementation
- `laneOwner=dedicated-renderer`가 실제 세션 evidence에 안정적으로 찍히게 만드는 작업
- `fallbackReason=none`을 정상 기본 상태로 만드는 작업
- `warm-ready`, `warm-hit`가 운영 evidence에 실제로 나타나게 만드는 작업
- resident lane이 fallback이 아니라 truthful close owner로 동작하게 만드는 작업

이 공백 때문에 “새 architecture의 이점”이 문서에만 있고, 실제 제품 경로에는 남지 못했다.

### 패턴 2. 운영 구성 artifact가 story deliverable로 취급되지 않음

`preview-renderer-policy.json`은 현재 architecture 승격 경계의 핵심 artifact다.

하지만 planning에서는 중요하게 다뤄지면서도, execution 수준에서는 아래 질문의 owner가 분명하지 않다.

- 어떤 preset/version이 canary로 승격되는가
- 언제 defaultRoute를 darktable에서 resident lane으로 바꿀 수 있는가
- route policy 변경은 어떤 story acceptance criteria로 닫히는가
- 정책 변경 후 active session safety는 누가 입증하는가

실제 현장 설정은 아직 `defaultRoute=darktable`이며 manual canary만 남아 있다.  
즉 architecture는 바뀌었지만 운영 구성은 바뀌지 않았다.

### 패턴 3. Warm-state가 계약 필드로만 존재하고 운영 readiness gate로 닫히지 않음

문서, contract, fixture, tests에는 warm-state vocabulary가 존재한다.

- `warm-ready`
- `warm-hit`
- `cold`
- `warm-state-lost`

하지만 실제 최근 세션 evidence에서는 warm-state가 실질적으로 작동하지 않는다.

- shadow fallback 위주
- recent evidence에서 `warmState=none`
- warmup result 자체가 `fallback-suggested`로 끝나는 흔적 존재

즉 warm-state는 “계약과 diagnostics vocabulary”로는 추가됐지만, “제품의 실경로 활성화 기준”으로 닫히지 않았다.

### 패턴 4. Success path보다 fallback path의 ownership이 강함

현재 story와 docs는 실패 시나리오를 매우 잘 정의한다.

- fallback
- rollback
- No-Go
- shadow 유지
- release hold

반면 성공 시나리오의 ownership은 약하다.

아래 조건이 갖춰져야 resident lane이 실제로 성공했다고 볼 수 있는데, 이를 직접 닫는 story가 없다.

- routeStage가 `canary` 또는 `default`
- laneOwner가 resident/dedicated renderer
- fallbackReason이 `none`
- warm-state evidence가 정상
- same-capture close가 목표 지표 안으로 들어옴

즉 failure governance는 강하지만 success activation governance는 약하다.

### 패턴 5. Test fixture와 real policy 사이의 단절

테스트와 contract fixture는 이미 다음 상태를 전제하는 경우가 있다.

- routeStage=`canary`
- warmState=`warm-ready`
- laneOwner=`dedicated-renderer`

하지만 실제 운영 policy와 session evidence는 아직 shadow/darktable default 상태다.

즉 “테스트 상상”과 “현장 설정”이 같은 수준으로 닫히지 않았다.

### 패턴 6. Final validation story가 implementation gap까지 흡수함

Story 1.13은 본래 최종 cutover와 canonical hardware close owner여야 한다.

그런데 현재는 아래 두 역할이 섞여 있다.

- 아직 빠진 implementation corrective를 메우는 역할
- 최종 hardware `Go / No-Go`를 판정하는 역할

이 구조는 위험하다.  
왜냐하면 1.13을 다시 실행할 때, 실제로는 implementation 부족으로 실패하는데 겉으로는 validation failure처럼 보이기 때문이다.

## 왜 이런 누락이 생겼는가

### 원인 1. Architecture decision이 adoption roadmap으로 충분히 번역되지 않았다

architecture는 방향을 잘 고정했지만, adoption 단계는 실제 backlog에 세분화되지 않았다.

필요했던 adoption roadmap은 아래와 같았다.

1. prototype
2. activation
3. guarded cutover
4. release close

실제 story 구조는 아래에 가까웠다.

1. prototype
2. measurement/governance
3. cutover/validation

즉 activation 단계가 backlog로 독립하지 못했다.

### 원인 2. Story 1.18이 “후보 검증”으로 닫힐 수 있도록 설계되었다

1.18은 의도적으로 prototype 범위만 소유한다.  
이 자체는 잘못이 아니다.

문제는 그 다음 story가 prototype을 실제 primary lane으로 올리는 implementation owner가 아니라, governance와 validation owner였다는 점이다.

즉 1.18의 제한이 다음 story에서 메워지지 않았다.

### 원인 3. Correct-course가 구조 보정은 했지만 activation story를 만들지는 않았다

기존 correct-course 흐름은 foundational contract, release gate, preview architecture pivot 정렬에는 성공했다.

하지만 이번 경우 필요한 것은 governance 보정만이 아니라,  
새 route를 실제 운영 경로로 승격하는 activation-focused story였다.

즉 “문서 정렬”은 있었지만 “운영 전환 소유 story”는 생기지 않았다.

### 원인 4. Release-truth mindset가 implementation gap을 가렸다

현재 문서군은 release-truth와 hardware gate를 매우 엄격하게 다룬다.  
이 점은 장점이다.

하지만 그 부작용으로, 문제를 backlog에서 볼 때도 validation 언어로만 읽게 됐다.

그래서 실제로는 implementation gap인 이슈가 아래처럼 읽혔다.

- promotion evidence 부족
- shadow-only evidence
- No-Go
- rerun 필요

이 프레이밍은 맞지만 불완전하다.  
왜 evidence가 shadow-only인지, 왜 rerun 전에 implementation corrective가 필요한지에 대한 owner를 만들지 못했다.

## 현재 목표 달성을 막는 핵심 공백

다음 항목을 소유하는 story가 현재 없다.

1. `preview-renderer-policy.json`를 darktable default에서 resident lane canary/default로 승격하는 작업
2. resident lane이 실제 truthful close owner로 작동하도록 activation하는 작업
3. warm-state를 diagnostics vocabulary가 아니라 운영 준비 상태로 닫는 작업
4. `fallback-heavy` 상태를 implementation 수준에서 줄이는 작업
5. real booth evidence에서 resident lane success path를 반복적으로 만드는 작업

## 필요한 보정 방향

### 방향 1. 새 corrective activation story를 추가한다

가장 권장되는 방향이다.

새 story는 예를 들어 아래 성격을 가져야 한다.

- resident lane activation
- preview route policy promotion
- warm-ready truthful close operationalization
- shadow 탈출
- canary/default success path evidence 확보

이 story는 prototype을 실제 제품 경로로 연결하는 implementation owner여야 한다.

### 방향 2. Story 1.13은 final validation owner로 다시 좁힌다

1.13은 유지하되 scope를 좁히는 편이 좋다.

1.13의 역할은 아래로 제한해야 한다.

- activation 이후 cutover 검증
- hardware matrix rerun
- rollback proof
- ledger canonical row update
- `Go / No-Go` 판정

즉 1.13은 implementation story가 아니라 release-truth close story로 유지하는 것이 맞다.

### 방향 3. Architecture 문서에는 stage를 명시적으로 추가한다

`architecture.md`는 방향은 충분하지만 execution stage가 약하다.

다음 문장을 추가하는 것이 바람직하다.

- preview architecture adoption은 `prototype -> activation -> guarded cutover -> release close` 순서를 따른다.
- `preview-renderer-policy.json`은 route policy artifact이자 rollout artifact다.
- warm-state는 additive diagnostics field를 넘어서 activation readiness evidence로 읽어야 한다.

### 방향 4. Epics 문서는 activation story를 명시적으로 가진다

현재 epics에는 1.18과 1.19가 있지만, activation story가 없다.

따라서 Epic 1 아래에 별도 story를 추가해야 한다.

권장 역할 예시는 아래와 같다.

- approved preset scope를 shadow 밖으로 올린다
- route policy promotion과 rollback safety를 구현한다
- resident lane이 actual truthful close owner로 동작하는 경로를 닫는다
- warm-ready/warm-hit operational evidence를 만든다
- fallbackReason=`none` 기본 경로를 실세션에서 확보한다

### 방향 5. 운영 구성 artifact를 story deliverable로 승격한다

아래 산출물은 더 이상 단순 설정 파일이 아니라 story deliverable로 다뤄야 한다.

- `branch-config/preview-renderer-policy.json`
- dedicated renderer warmup/result evidence
- canary/default route evidence
- rollback proof package

즉 “코드가 있다”가 아니라 “운영 경계가 실제로 바뀌었다”를 story 완료 조건에 포함해야 한다.

## 권장 실행안

### Option A. 신규 story 추가 + 1.13 scope 축소

가장 권장한다.

구성:

- 새 story 추가
  - 예: `Story 1.20: resident preview lane activation과 warm-ready truthful close 승격`
- 1.13 수정
  - final cutover/hardware close owner로 역할 축소
- architecture/epics 보강
  - activation stage를 명시

장점:

- owner가 분명해진다
- implementation gap과 validation gap을 분리할 수 있다
- 1.13 rerun 실패가 “또 validation 실패”처럼 보이는 문제를 막을 수 있다

리스크:

- backlog 번호와 문서 동기화가 필요하다

### Option B. 1.13 내부에 activation subtrack을 추가

가능은 하지만 권장도는 낮다.

이 경우 1.13이 너무 커진다.

- implementation corrective
- rollout activation
- rollback proof
- hardware close

를 한 story가 모두 소유하게 되기 때문이다.

장점:

- 문서 수가 적다

단점:

- owner가 다시 흐려진다
- review와 done 기준이 다시 모호해진다

## 권장안

권장안은 `Option A`다.

즉:

1. 신규 activation story 추가
2. 1.13을 final validation/cutover owner로 축소
3. architecture와 epics에 activation stage를 명시

## 신규 story 초안 방향

새 story는 최소한 아래 acceptance를 가져야 한다.

1. approved preset/version scope에서 resident lane canary/default route가 host-owned policy로 승격될 수 있어야 한다
2. active session은 route policy 변경으로 재해석되지 않아야 한다
3. resident lane success path가 실세션 evidence에서 `laneOwner=dedicated-renderer`, `fallbackReason=none`, `routeStage=canary|default`, `warmState=warm-ready|warm-hit`로 반복 확인돼야 한다
4. 실패 시 booth-safe fallback, rollback, truthful waiting semantics는 유지돼야 한다
5. Story 1.13 hardware rerun이 activation gap이 아니라 release-truth close 판단만 수행할 수 있는 상태가 돼야 한다

## 기존 story 수정 제안

### Story 1.13 수정 방향

- implementation corrective owner 성격을 줄인다
- prerequisite로 activation story completion을 명시한다
- canonical hardware rerun, rollback evidence, ledger update를 중심으로 scope를 재정리한다

### Story 1.18 수정 여부

필수는 아니다.

1.18은 prototype story로서 이미 역할이 분명하다.  
다만 completion note 또는 linked follow-up에 “activation story required”를 명시하면 더 명확해진다.

### Story 1.19 수정 여부

필수는 아니다.

1.19는 gate establishment story로 역할이 적절하다.  
다만 “activation 이후 rerun 기준”과 연결되는 링크를 추가하면 더 좋다.

## 문서 수정 대상 제안

우선순위 순으로 보면 다음과 같다.

1. `_bmad-output/planning-artifacts/epics.md`
2. `_bmad-output/implementation-artifacts/sprint-status.yaml`
3. `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
4. `_bmad-output/planning-artifacts/architecture.md`
5. `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

## 최종 정리

이번 문제는 architecture 방향 실패가 아니다.  
문제는 architecture adoption을 backlog로 번역할 때 핵심 단계 하나가 빠졌다는 점이다.

정확히는 다음이 빠졌다.

- prototype 이후
- validation 이전
- resident lane을 실제 제품 경로로 활성화하는 implementation owner

이 공백을 메우지 않으면, 앞으로도 문서상으로는 resident GPU-first가 primary lane인데 실제 현장에서는 darktable shadow/fallback이 계속 기본 경로로 남을 가능성이 높다.

따라서 다음 보정의 핵심은 “더 정밀한 검증”이 아니라 먼저 “activation owner를 backlog에 복구하는 것”이다.

## 권장 다음 액션

1. activation-focused 신규 story를 생성한다
2. Story 1.13을 final validation owner로 재정렬한다
3. architecture와 epics에 adoption stage를 명시한다
4. 운영 route policy artifact를 story deliverable로 승격한다

