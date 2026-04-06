# 최근 세션 썸네일 속도 딥리서치 브리프

작성일: `2026-04-05`

## 목적

이 문서는 새 딥리서치 에이전트가 Boothy의 `최근 세션 썸네일` 문제를 현재 맥락 그대로 이어받아,
더 효율적이거나 더 좋은 다음 해법을 조사할 수 있게 만드는 조사 브리프다.

이번 리서치의 목표는 단순한 아이디어 수집이 아니다.
아래 질문에 대해 제품 의사결정에 바로 쓸 수 있는 답을 내는 것이다.

- 현재 구조를 더 다듬으면 `truthful preset-applied preview close`를 의미 있게 더 줄일 수 있는가
- 아니면 이제는 `lighter truthful renderer` 또는 `different close topology` 같은 다음 구조 판단으로 넘어가야 하는가

## 한 줄 현황

- 사용자가 체감하는 `3초대` 개선은 사실이다.
- 하지만 그 기준은 `same-capture first-visible`이다.
- 고객이 실제로 기다리는 `preset-applied truthful close`는 아직 대체로 `6초대 후반 ~ 8초대`, 나쁜 경우 `10초대`까지 남아 있다.
- 따라서 지금 문제는 `같은 컷을 빨리 한번 보이게 하는 것`보다 `truthful close를 더 빨리 닫는 것`이다.

## 지금 제품 기준으로 사실로 봐야 하는 것

- `fastPreviewVisibleAtMs`는 same-capture `first-visible` 지표다.
- `previewVisibleAtMs`와 `xmpPreviewReadyAtMs`는 preset-applied preview truth를 닫는 지표다.
- 고객 surface는 truthful preview가 실제로 닫히기 전까지 계속 `Preview Waiting` 계약을 지켜야 한다.
- fast preview가 먼저 보여도 canonical preview path는 같은 슬롯에서 later replacement 되어야 한다.
- 다른 컷, 다른 세션 자산, placeholder, RAW copy는 truthful preview ready 근거가 될 수 없다.

## 현재 상태 요약

### 1. 이미 좋아진 것

- same-capture `first-visible`은 최근 여러 회차에서 대체로 `약 3.0s ~ 3.5s`까지 내려왔다.
- helper가 `camera-thumbnail`에 실패해도 `windows-shell-thumbnail`로 same-capture first-visible을 닫는 흐름은 반복 확인됐다.
- recent-session rail에서 `무언가 빨리 보이는 것` 자체는 더 이상 최우선 병목이 아니다.

### 2. 아직 안 풀린 핵심

- truthful preset-applied close는 아직 제품 기준으로 충분히 빠르지 않다.
- 최근 best band도 대체로 `약 6.2s ~ 6.5s` 수준이다.
- 다른 최신 회차는 다시 `7s+` 또는 `8s+`로 밀린다.
- 과거에는 duplicate render, join miss, cold first capture가 크게 작용했고,
  최근에는 `winning truthful close lane 자체의 비용`이 더 중심으로 보인다.

### 3. 지금 해석에서 특히 중요한 변화

- 현재 병목은 더 이상 단순히 `same-capture source를 찾는 일`이 아니다.
- 현재 병목은 점점 `truthful close owner가 실제로 닫히는 비용` 쪽으로 수렴하고 있다.
- 특히 `Test Look` canary 기준으로는 truthful close owner가 이미 `fast-preview-raster` 쪽으로 이동한 회차가 있었다.
- 따라서 다음 리서치는 `같은 썸네일을 빨리 보이게 하는 법`보다
  `truthful close를 더 싼 방식으로 만들 수 있는가`를 중심에 두어야 한다.

## 최신 기준선으로 봐야 할 실측

### A. 2026-04-04 재확인 4컷

- same-capture first-visible 평균: `약 3115ms`
- preset-applied preview close 평균: `약 7715ms`
- 첫 컷 final close: `10403ms`

해석:

- 사용자 체감의 `3초대`는 맞다.
- 하지만 final close 기준으로는 아직 충분하지 않다.

### B. 2026-04-05 Test Look best completed run

- session:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a351696c25d93c`
- completed 3컷 평균:
  - same-capture first-visible `약 2959ms`
  - preset-applied preview close `약 6372ms`

해석:

- 현재까지 관찰된 비교적 좋은 truthful close band는 `약 6.3s` 전후다.
- 그래도 제품 관점에서는 아직 느리다.

### C. 2026-04-05 latest booth baseline으로 기록해야 하는 run

- session:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3768babafc5e4`
- completed close:
  - `7772ms`
  - `8520ms`
  - `7298ms`
  - `7526ms`

해석:

- 이 run은 새 배선이 현장에 반영된 증거로 보지 않는다.
- route evidence가 없고 warmup CRC 문제도 남아 있었다.
- 따라서 이 회차는 `최신 baseline data`로는 유효하지만 `최신 fix 검증본`으로는 보지 않는다.

## 이미 반영된 것

아래 범주는 이미 코드에 들어간 상태를 전제로 조사해야 한다.

- resident first-visible worker
- worker warm-up / preload / cache priming
- speculative close와 direct close의 duplicate render 보정
- local renderer sidecar runtime-scoped cache 재사용
- darktable version probe cache 재사용
- truthful close 해상도 회귀 보정
- finished speculative preview의 늦은 승격 보정
- speculative lane이 session-locked preview route policy를 보도록 한 배선

즉 이번 리서치는 `worker를 새로 도입할까`를 묻는 단계가 아니다.
지금은 `현재 계열 구조로 더 내려갈 수 있는지`, 아니면 `다른 truthful close 구조로 넘어가야 하는지`를 묻는 단계다.

## 조사에서 절대 놓치면 안 되는 제약

- `Preview Waiting` truth를 깨면 안 된다.
- `previewReady`는 실제 preset-applied preview file이 생기기 전까지 올리면 안 된다.
- same-slot replacement correctness를 깨면 안 된다.
- capture record의 `activePresetId + activePresetVersion` 계약을 깨면 안 된다.
- pinned darktable version `5.4.1` 전제를 함부로 무시하면 안 된다.
- default booth path에 승인되지 않은 experimental/speculative flag를 다시 섞으면 안 된다.
- 조사안은 반드시 session-scoped seam 계측으로 효과를 다시 증명할 수 있어야 한다.

## 이번 딥리서치가 답해야 할 핵심 질문

1. 현재 darktable/local-renderer 계열 구조에서 `truthful close`를 지금보다 크게 더 줄일 현실적인 여지가 아직 있는가
2. 지금 남은 비용의 본체는 `startup/cache`가 아니라 `render body`로 보는 편이 맞는가
3. 그렇다면 다음 해법은 무엇이 가장 현실적인가
4. `lighter truthful renderer`를 도입한다면 현재 preset fidelity와 제품 계약을 어디까지 유지할 수 있는가
5. `different close topology`가 필요하다면 어떤 형태가 Boothy에 가장 맞는가
6. 각 후보가 `제품 리스크`, `구현 난이도`, `현장 운영성`, `점진 롤아웃`, `rollback 용이성` 면에서 어떻게 다른가

## 조사 방향

### 우선 조사해야 할 축

- `현 구조 추가 최적화`가 아직 유효한지
- `local dedicated truthful renderer`가 현실적인지
- `preview 전용 published artifact`를 따로 두는 방식이 가능한지
- `different close topology`가 truth 계약을 유지한 채 가능한지
- `darktable fallback`을 남긴 점진 전환이 가능한지

### 꼭 비교해야 할 후보 범주

- current darktable/local-renderer path를 더 resident / persistent / cached 하게 가져가는 방향
- published XMP를 직접 재현하지 않더라도 preview truth에 맞는 별도 경량 renderer를 두는 방향
- final/export와 recent-session truthful close를 구조적으로 분리하는 방향
- host는 truth owner로 남기고 renderer만 교체 가능한 adapter 구조

### 조사에서 피해야 할 것

- `3초대 first-visible`만 보고 문제를 해결된 것으로 해석하는 것
- UI masking이나 copy 조정으로 문제를 우회하는 것
- false-ready를 허용하는 편법
- 현재 문서에 이미 많이 정리된 미세 cap 조정 나열
- 근거 없이 `엔진을 전부 바꾸자`로 점프하는 것

## 리서치 에이전트에게 기대하는 산출물

### 반드시 포함할 것

- 현재 맥락을 반영한 `권장안 1개`
- 비교 가능한 `후보 2~4개`
- 각 후보의
  - latency potential
  - preset fidelity risk
  - implementation risk
  - ops complexity
  - fallback compatibility
  - incremental rollout fit
- 왜 지금 이 후보가 맞는지에 대한 제품 관점 설명
- `다음 실험 1개`와 `하지 말아야 할 것 1개`

### 가능하면 포함할 것

- `30/60/90일` 수준의 현실적인 다음 단계
- `2주 내 검증 가능한 프로토타입` 범위
- 성공 판단 기준
  - `previewVisibleAtMs`
  - close owner
  - seam completeness
  - false-ready 0건

## 리서치 품질 기준

- 최신 공식 문서와 1차 자료를 우선 사용한다.
- 내부 문서의 제품 계약과 최신 session evidence를 기준점으로 유지한다.
- `추정`, `사실`, `권고`를 구분한다.
- `현실적으로 2~6주 안에 검증 가능한 선택지`와 `장기 옵션`을 분리한다.
- 결론은 `좋아 보이는 아이디어 모음`이 아니라 `다음 의사결정안`이어야 한다.

## 권장 읽기 순서

1. `history/recent-session-thumbnail-speed-agent-context.md`
2. `docs/contracts/render-worker.md`
3. `docs/contracts/session-manifest.md`
4. `history/recent-session-thumbnail-speed-brief.md`
5. `history/recent-session-thumbnail-speed-log-2026-04-04.md`
6. `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
7. `_bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar와-truthful-preview-close-canary-routing.md`
8. `_bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-2026-04-04.md`

## 리서치 시작점에 두는 현재 가설

- cheap win은 이미 여러 번 넣었다.
- 현재 남은 비용은 점점 `truthful close render body` 쪽으로 보인다.
- 그래서 다음 리서치는 `더 작은 cap`보다
  `lighter truthful renderer` 또는 `different close topology`
  판단에 더 무게를 둬야 할 가능성이 높다.

단, 이 가설은 딥리서치 결과로 검증하거나 뒤집어도 된다.
중요한 것은 `현재 구조 안에서 더 내려갈 수 있는지`를 근거 있게 닫는 것이다.

## 새 조사 에이전트용 한 줄 결론

지금 Boothy는 `3초대 first-visible`을 더 줄이는 단계가 아니라,
`truthful preset-applied close`를 지금의 `6초대 후반 ~ 8초대`에서 의미 있게 더 낮출
다음 구조를 고르는 단계에 있다.
