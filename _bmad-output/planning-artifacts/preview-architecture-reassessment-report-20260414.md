# Preview Architecture Reassessment Report

**Date:** 2026-04-14  
**Project:** Boothy  
**Purpose:** 현재 preview 아키텍처가 최근 새롭게 적용된 것이 맞는지, 그리고 지금 단계에서 미세조정을 계속하는 것이 맞는지 제품 관점에서 재평가한다.

## 1. 검토 범위

이번 재평가는 아래 문서와 최신 하드웨어 검증 evidence를 기준으로 수행했다.

- `_bmad-output/planning-artifacts/implementation-readiness-report-20260413.md`
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260413-155159.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `docs/contracts/local-dedicated-renderer.md`
- `history/camera-capture-validation-history.md`
- 최신 하드웨어 검증 세션
  - `session_000000000018a62558931dd920`
  - `session_000000000018a629892187d3a8`

## 2. 핵심 결론

### 결론 1: 현재 적용 중인 preview 아키텍처는 최근 새롭게 적용된 것이 맞다

문서 기준으로 현재 승인된 방향은 기존 darktable 중심 preview truth owner가 아니라,
`resident GPU-first primary lane + different close topology`, 즉 현재 코드/로그에서 보이는
`local dedicated renderer + first-visible lane 분리` 구조다.

이 방향은 2026-04-13 planning 보정에서

`prototype -> activation -> guarded cutover -> release close`

순서를 따르도록 명시적으로 재정렬되었다.

최신 하드웨어 로그에서는 아래가 실제로 확인됐다.

- `laneOwner=dedicated-renderer`
- `routeStage=canary`
- `fallbackReason=none`
- `warmState=warm-hit`

따라서 현재 시스템은 “새 아키텍처가 아직 미적용인 상태”가 아니라,
**새 아키텍처의 activation/canary 단계가 실제로 적용된 상태**로 판단한다.

### 결론 2: 그러나 기대했던 수준의 극적인 속도 개선은 아직 나오지 않았다

최신 하드웨어 evidence에서는 activation 자체는 성공했지만,
체감 속도 개선은 제한적이었다.

대표 최근 수치:

- `session_000000000018a62558931dd920`
  - `replacementMs=5533` outlier 발생
- `session_000000000018a629892187d3a8`
  - `replacementMs=4411`
  - `replacementMs=4455`
  - `replacementMs=3494`

즉 “새 경로가 실제로 돌고 있다”는 점은 확인됐지만,
목표였던 급격한 시간 단축, 특히 `2.5초 이하` 관점에서는 아직 충분하지 않다.

## 3. 문서와 실제 결과의 일치 여부

### 문서가 기대한 변화

2026-04-13 planning 문서들은 이번 변경을 단순 최적화가 아니라
`activation owner 복구`로 정의했다.

의도는 다음과 같았다.

- resident lane을 실제 primary canary/default route로 올린다
- `preview-renderer-policy.json`을 rollout artifact로 다룬다
- warm-state를 diagnostics 용어가 아니라 activation readiness evidence로 본다
- 이후 Story 1.13은 final validation owner만 맡는다

### 실제로 일어난 변화

실제 최신 로그는 위 activation 의도가 반영됐음을 보여준다.

- `preview-renderer-policy` 기반 route promotion이 실제로 적용됐다
- dedicated renderer가 close owner로 선택됐다
- warm-state evidence가 실제 session proof에 남았다
- speculative close 중복 경쟁 제거도 반영됐다

즉 **문서상 activation gap은 메워졌다.**

### 실제로 일어나지 않은 변화

반면, 문서상 “resident lane 활성화”가 곧바로 “극적인 latency 개선”을 보장하진 않았다.

실제 결과는 아래에 가깝다.

- booth-safe semantics는 개선됐다
- route ownership은 전환됐다
- fallback 비율은 줄었다
- 하지만 final preset-applied close 비용은 여전히 크다

따라서 이번 아키텍처 변화는
**운영 경로와 truth ownership 변화에는 성공했지만, 성능 목표 달성에는 아직 불충분했다**고 보는 것이 맞다.

## 4. 지금 계속 미세조정을 해야 하는가

### 판단

무기한 미세조정을 계속하는 것은 권장하지 않는다.

이유는 간단하다.

1. 이제 문제의 본질은 “새 아키텍처가 안 켜져 있다”가 아니다.
2. 중복 speculative close, late fast preview, path normalization 같은 activation 주변 이슈는 이미 상당 부분 정리됐다.
3. 그 이후에도 `replacementMs`가 여전히 3.5초~4.5초 수준에 남아 있다.

즉 현재 남은 병목은 activation gap이나 주변 wiring 문제가 아니라,
**현재 dedicated close 구조 자체의 비용 한계**일 가능성이 높다.

### 제품 관점 판단

현재 구조는 아래 목적에는 의미가 있었다.

- truthful `Preview Waiting` 유지
- first-visible 와 preset-applied close 분리
- booth-safe fallback 유지
- route promotion / rollback / evidence 관리

하지만 아래 목적에는 아직 충분한 증거가 없다.

- booth 체감에서 분명히 느껴지는 극적인 속도 개선
- `replacementMs <= 2500` 수준 달성

따라서 이 시점부터는
“activation correctness 확보 후 추가 미세조정 몇 번”까지는 가능하지만,
그 결과도 목표를 크게 못 줄이면 계속 같은 방식으로 밀어붙이는 것은 비효율적이다.

## 5. 권장 의사결정

### 현재 아키텍처에 대한 판단

- **Activation 성공:** 예
- **운영 경로 전환 성공:** 예
- **속도 목표 달성:** 아니오
- **추가 미세조정만으로 충분할 가능성:** 낮아지는 중

### 권장 다음 단계

1. 현재 반영된 최신 최적화 버전(`strategyVersion=2026-04-14h`)으로 최종 한 번 더 측정한다.
2. 그 결과도 `replacementMs`가 여전히 3초대 후반~4초대에 머물면,
   현재 구조는 activation 성공 아키텍처로 기록하고, 속도 목표용 최종 해법으로는 재평가하지 않는다.
3. 이후에는 새로운 preview architecture 대안을 검토한다.

## 6. 최종 요약

현재 적용된 preview 아키텍처는 최근 새롭게 적용된 것이 맞다.
다만 이번 변화는 “아직 안 쓰던 구조를 실제 제품 경로로 올린 것”이지,
자동으로 극적인 latency 개선을 만들어낸 것은 아니었다.

따라서 지금 상태를 제품 관점으로 요약하면 다음과 같다.

- 새 아키텍처 적용 여부: **적용됨**
- rollout/activation 관점 성공 여부: **성공**
- 체감 성능 혁신 여부: **아직 아님**
- 지금처럼 미세조정을 계속해야 하는가: **짧게는 가능하지만, 장기적으로는 비권장**
- 새로운 아키텍처를 찾아봐야 하는가: **그렇다. 충분히 검토할 시점이다.**
