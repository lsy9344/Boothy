---
documentType: route-decision-note
status: active
date: 2026-04-18
scope: preview-track
---

# Preview Track Route Decision

## 결론

- Story `1.30`의 추가 하드웨어 재검증은 여기서 중단한다.
- 현재 actual-primary-lane 경로는 canonical `Go` 후보가 아니라 official `preset-applied visible <= 3000ms` gate 기준 bounded `No-Go` 판단 대상으로 본다.
- Story `1.31`은 열지 않는다.
- Story `1.13`은 계속 blocked 상태로 둔다.
- `2026-04-20` 기준 Story `1.10` old `resident first-visible` line은 closed `No-Go` baseline으로 확정한다.
- `2026-04-20` 기준 Story `1.26 reserve path`는 이제 공식 오픈된 다음 실험 트랙이다.

## 왜 여기서 멈추는가

- 승인 하드웨어에서 반복 측정이 계속 제품 KPI `preset-applied visible <= 3000ms`, 즉 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`를 크게 벗어났다.
- 최근 조정들은 일부 개선은 만들었지만, 제품 승인 관점에서 의미 있는 수준의 수렴을 만들지 못했다.
- 따라서 지금 단계의 추가 반복은 제품 결정 품질보다 기록량만 늘릴 가능성이 높다고 보고, 현재 route baseline은 `1.30 bounded No-Go`로 유지한다.

## 지금 사용하는 방법

현재는 `fail-closed route decision` 방법을 사용한다.

- release 기준을 먼저 고정한다
- 실제 하드웨어에서 반복 증거를 확인한다
- 목표에 근접하지 못하고 반복 실패가 누적되면 open-ended 최적화를 멈춘다
- 그 상태를 canonical `No-Go`로 기록하고 다음 경로 판단으로 넘어간다

즉, "조금 좋아졌는가"가 아니라 "이 경로가 제품 gate를 닫을 수 있는가"로 판단한다.

이 문서에서 말하는 canonical `No-Go`는 pending ratification이 아니라 현재 운영 해석이다.

여기서 제품 gate는 하나다.

- official gate: `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
- product wording: `preset-applied visible <= 3000ms`
- `sameCaptureFullScreenVisibleMs`는 reference/comparison metric일 뿐 official pass line이 아니다.

## 용어 정리

### 되돌아갈 기본 후보가 없다는 뜻

- 과거 경로 중 지금 current actual-primary-lane보다 더 좋은 실기기 수치는 있었다.
- 대표적으로 legacy `local dedicated renderer + first-visible lane` 계열에서 `replacementMs=4411`, `4455`, `3494`와 `presetAppliedDeltaMs=4245`, `3514`, `3516` 같은 기록이 남아 있다.
- 하지만 이 수치들은 현재 release-close decision field가 아니라 legacy comparison field 또는 과거 seam 기준 기록이다.
- 즉, "그 경로로 그냥 되돌리면 현재 release gate를 닫는다"는 뜻의 증거는 아니다.
- 현재까지 확인한 범위에서는 승인 하드웨어에서 official `preset-applied visible <= 3000ms` gate를 닫았다고 말할 수 있는 과거 architecture 증거를 찾지 못했다.

따라서 여기서 말하는 `기본 후보 없음`은 "과거에 더 나았던 시도조차 없었다"는 뜻이 아니라, `지금 바로 되돌려도 현재 공식 합격선을 만족한다고 증명된 fallback architecture가 없다`는 뜻이다.

### 다음 공식 시도라는 뜻

- 여기서 `공식`은 현재 저장소의 release/sprint/epic 문서에 이미 정의된 순서를 뜻한다.
- 그 순서상 actual-lane track은 `1.28 -> 1.29 -> 1.30 -> 1.31`이고, reserve track은 `1.26`이다.
- `1.31`은 `1.30`이 accepted canary `Go` 후보를 만들었을 때만 이어지는 default/rollback gate다.
- 반대로 현재처럼 `1.30`이 bounded `No-Go` baseline으로 닫혀 있으면, 다음에 검토할 공식 실험 후보는 임의의 새 아이디어가 아니라 문서상 reserve track인 `1.26`이다.
- `2026-04-20` opening 판단으로 이제 그 reserve track은 실제 active scope가 됐다.

## 과거 더 좋았던 기록의 해석

- 과거 실기기 기준으로 현재 actual-primary-lane보다 더 좋은 기록은 확인됐다.
- 다만 그 대부분은 `replacementMs` 또는 `presetAppliedDeltaMs` 같은 legacy seam/비교 지표에 속한다.
- 현재 공식 release-close 판단은 `originalVisibleToPresetAppliedVisibleMs <= 3000ms` 하나만 본다.
- `sameCaptureFullScreenVisibleMs`와 legacy better run은 `비교 기준`으로는 중요하지만, `지금 바로 되돌릴 release-proof architecture`로 읽으면 안 된다.

## 공식 히스토리 문서 검토 결과

검토 대상:

- `docs/preview-architecture-history-and-agent-guide.md`
- `docs/release-baseline.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md`
- `_bmad-output/planning-artifacts/research/technical-boothy-preview-architecture-alternatives-research-20260414.md`
- `history/camera-capture-validation-history.md`
- `C:\Code\Project\Boothy_thumbnail-latency-seam-reinstrumentation\history\recent-session-thumbnail-speed-brief.md`
- `C:\Code\Project\Boothy_thumbnail-latency-seam-reinstrumentation\docs\recent-session-preview-direction-change-and-execution-plan-2026-04-07.md`

이번 기준에서 확인한 반영 상태:

1. 공식 히스토리 문서는 이제 `과거 더 좋은 수치가 있었다`는 사실과, 그 수치가 `legacy comparison metric`이며 current release-proof가 아니라는 점을 current single-gate wording으로 함께 적고 있다.
2. 공식 히스토리 문서는 `first-visible worker -> local dedicated renderer -> reserve candidate`로 이어진 초기 의사결정 사다리를 현재 route-decision 해석과 충돌하지 않는 수준으로 요약한다.
3. `watch-folder bridge`와 `edge appliance`는 여전히 research/next-step ladder 성격이지만, 현재 공식 문서 해석에서는 implemented candidate와 research candidate를 구분해 읽도록 정리돼 있다.
4. 승인 하드웨어에서 current official gate를 실제로 닫은 architecture를 아직 찾지 못했다는 점도 현재 공식 문서 집합에 직접 요약돼 있다.
5. `1.31`과 `1.26`의 의미도 이제 현재 문서 기준에서 분리돼 있다. `1.31`은 success-side default/rollback gate이고, `1.26`은 repeated failure 뒤 검토하는 reserve experiment candidate다.

현재 판단:

- 공식 히스토리 문서의 큰 흐름은 현재 route-decision baseline과 맞는다.
- 이후 보강이 필요하다면 이는 gate 정의 수정이 아니라, 새 evidence가 생겼을 때 사례를 더 추가하는 수준으로 본다.

## 운영 해석

1. `1.30` actual-primary-lane은 현재 canonical bounded `No-Go` baseline이다.
2. `1.31`은 success-side default/rollback gate로만 남고, 지금은 열지 않는다.
3. `1.13`은 계속 blocked 상태로 둔다.
4. old `resident first-visible` line은 Story `1.10` 기준 closed `No-Go` baseline으로 고정한다.
5. Story `1.26 reserve path`는 공식 오픈된 active experiment track으로 읽는다.
