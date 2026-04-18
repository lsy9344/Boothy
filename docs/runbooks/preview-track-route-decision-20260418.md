---
documentType: route-decision-note
status: proposed
date: 2026-04-18
scope: preview-track
---

# Preview Track Route Decision

## 결론

- Story `1.30`의 추가 하드웨어 재검증은 여기서 중단한다.
- 현재 actual-primary-lane 경로는 canonical `Go` 후보가 아니라 bounded `No-Go` 판단 대상으로 본다.
- Story `1.31`은 열지 않는다.
- Story `1.13`은 계속 blocked 상태로 둔다.
- 다음 판단 대상은 `1.26 reserve path` 개시 여부다.
- 다만 `1.26`은 자동 오픈이 아니다. `1.30` 중단을 repeated actual-lane failure의 충분한 증거로 승인한 뒤에만 다음 공식 실험 트랙이 될 수 있다.

## 왜 여기서 멈추는가

- 승인 하드웨어에서 반복 측정이 계속 제품 KPI `same-capture preset-applied full-screen visible <= 3000ms and original-visible-to-preset-applied-visible <= 3000ms`를 크게 벗어났다.
- 최근 조정들은 일부 개선은 만들었지만, 제품 승인 관점에서 의미 있는 수준의 수렴을 만들지 못했다.
- 따라서 지금 단계의 추가 반복은 제품 결정 품질보다 기록량만 늘릴 가능성이 높다.

## 지금 사용하는 방법

현재는 `fail-closed route decision` 방법을 사용한다.

- release 기준을 먼저 고정한다
- 실제 하드웨어에서 반복 증거를 확인한다
- 목표에 근접하지 못하고 반복 실패가 누적되면 open-ended 최적화를 멈춘다
- 그 상태를 canonical `No-Go`로 기록하고 다음 경로 판단으로 넘어간다

즉, "조금 좋아졌는가"가 아니라 "이 경로가 제품 gate를 닫을 수 있는가"로 판단한다.

## 용어 정리

### 되돌아갈 기본 후보가 없다는 뜻

- 과거 경로 중 지금 current actual-primary-lane보다 더 좋은 실기기 수치는 있었다.
- 대표적으로 legacy `local dedicated renderer + first-visible lane` 계열에서 `replacementMs=4411`, `4455`, `3494`와 `presetAppliedDeltaMs=4245`, `3514`, `3516` 같은 기록이 남아 있다.
- 하지만 이 수치들은 현재 release-close decision field가 아니라 legacy comparison field 또는 과거 seam 기준 기록이다.
- 즉, "그 경로로 그냥 되돌리면 현재 release gate를 닫는다"는 뜻의 증거는 아니다.
- 현재까지 확인한 범위에서는 승인 하드웨어에서 `sameCaptureFullScreenVisibleMs <= 3000ms`와 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`를 함께 만족한 과거 architecture 증거를 찾지 못했다.

따라서 여기서 말하는 `기본 후보 없음`은 "과거에 더 나았던 시도조차 없었다"는 뜻이 아니라, `지금 바로 되돌려도 현재 공식 합격선을 만족한다고 증명된 fallback architecture가 없다`는 뜻이다.

### 다음 공식 시도라는 뜻

- 여기서 `공식`은 현재 저장소의 release/sprint/epic 문서에 이미 정의된 순서를 뜻한다.
- 그 순서상 actual-lane track은 `1.28 -> 1.29 -> 1.30 -> 1.31`이고, reserve track은 `1.26`이다.
- `1.31`은 `1.30`이 accepted canary `Go` 후보를 만들었을 때만 이어지는 default/rollback gate다.
- 반대로 `1.30`이 bounded `No-Go`로 닫히면, 다음에 검토할 공식 실험 후보는 임의의 새 아이디어가 아니라 문서상 reserve track인 `1.26`이다.
- 다만 `1.26`은 `1.13 blocked`만으로 열리는 것이 아니라, actual-lane repeated failure가 충분히 확인됐다고 승인할 때만 열린다.

## 과거 더 좋았던 기록의 해석

- 과거 실기기 기준으로 현재 actual-primary-lane보다 더 좋은 기록은 확인됐다.
- 다만 그 대부분은 `replacementMs` 또는 `presetAppliedDeltaMs` 같은 legacy seam/비교 지표에 속한다.
- 현재 공식 release-close 판단은 `sameCaptureFullScreenVisibleMs`와 `originalVisibleToPresetAppliedVisibleMs`를 함께 본다.
- 그래서 legacy better run은 `비교 기준`으로는 중요하지만, `지금 바로 되돌릴 release-proof architecture`로 읽으면 안 된다.

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

누락 의심 또는 보강 필요 항목:

1. 공식 히스토리 문서는 `과거 더 좋은 수치가 있었다`는 사실은 일부 담고 있지만, 그 수치가 `legacy comparison metric`이며 current release-proof가 아니라는 점을 더 명시적으로 적지 않는다.
2. 공식 히스토리 문서는 `first-visible worker -> local dedicated renderer -> reserve candidate`로 이어진 초기 의사결정 사다리를 배경으로만 다룬다. 워크트리 히스토리와 초기 기술조사 문서에는 이 사다리가 더 분명히 남아 있다.
3. `watch-folder bridge`와 `edge appliance`는 실제 주력 구현 이력이라기보다 research/next-step ladder에 가까운데, 공식 히스토리 문서는 이 차이를 더 또렷하게 적을 필요가 있다.
4. 승인 하드웨어에서 current release decision field를 실제로 닫은 architecture를 아직 찾지 못했다는 점이 공식 히스토리 문서에 더 직접적으로 요약돼 있지 않다.
5. `1.31`과 `1.26`의 의미가 실무 해석에서 섞일 여지가 있다. `1.31`은 success-side default/rollback gate이고, `1.26`은 repeated failure 뒤 reserve experiment라는 점을 더 선명히 적는 편이 안전하다.

현재 판단:

- 공식 히스토리 문서의 큰 흐름은 대체로 맞다.
- 다만 `과거 더 좋았던 수치`, `실제 release-proof 부재`, `research candidate와 implemented candidate의 구분`, `1.31 vs 1.26 gate 해석`은 보강이 필요하다.

## 남은 절차

1. `sprint-status.yaml`에서 `1.30`을 더 이상 다음 활성 검증 루프로 두지 않도록 갱신한다.
2. Story `1.30` 문서에 추가 하드웨어 rerun 중단과 bounded `No-Go` 정리 메모를 남긴다.
3. `hardware-validation-ledger.md`에 actual-lane loop를 canonical `No-Go`로 기록한다.
4. `1.31`은 계속 backlog/blocked 상태로 둔다.
5. `1.13`은 계속 blocked 상태로 둔다.
6. `1.26`을 공식적으로 열 조건이 충족됐는지 판단한다.
7. `1.26`을 열면 reserve path 범위를 좁게 정의한 새 컨텍스트로 이어간다.
