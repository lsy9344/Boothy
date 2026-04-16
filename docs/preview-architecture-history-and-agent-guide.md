# Preview Architecture History And Agent Guide

## 목적

이 문서는 Boothy의 촬영 후 preview/thumbnail/풀스크린 close 문제를 새 에이전트가 다시 처음부터 파헤치지 않도록 만드는 운영 문서다.

이 문서의 목표는 네 가지다.

- 과거부터 기준 시점까지 무엇을 조사했고 무엇을 시도했는지 한 번에 보이게 한다.
- 어떤 시도가 실패했는지와 왜 실패했는지를 제품 기준으로 정리한다.
- 어떤 개념 전환과 제품 기준이 형성됐는지를 조사 맥락과 함께 정리한다.
- 다음 에이전트가 어디부터 읽고, 무엇을 다시 검증하고, 무엇은 반복하지 말아야 하는지 지침을 남긴다.

기준 시점은 `2026-04-16`이다.

---

## 먼저 알아야 할 조사 결론

- 문제는 "새 preview 아키텍처가 아직 안 켜져 있다"가 아니었다.
- `local dedicated renderer + first-visible lane 분리` 구조는 실제 하드웨어 canary까지 적용됐다.
- 하지만 이 구조는 제품 KPI인 `same-capture preset-applied full-screen visible <= 2500ms`를 반복적으로 닫지 못했다.
- 따라서 기존 dedicated renderer 경로는 `activation baseline`과 `evidence contract proof`로는 성공했지만, 최종 primary close architecture로는 부족하다는 판단이 문서화되어 있다.
- 이후 forward path는 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact` 기준으로 재정렬됐다.
- 최신 후속 조사에서는 세션 소실 race, preview close latency, follow-up capture timeout이 서로 다른 문제축으로 다시 분리되어 기록됐다.

---

## 제품 기준과 성공 조건

### Canonical KPI

- primary release sign-off: `same-capture preset-applied full-screen visible <= 2500ms`
- same capture 정합성: `wrong-capture = 0`
- preset fidelity drift: `0`
- fallback stability: accepted threshold 이내
- rollback proof: one-action rollback evidence 필수

### 성공으로 세지 않으면 안 되는 것

다음 항목은 중간 신호일 수는 있지만 제품 합격으로 세면 안 된다.

- `firstVisibleMs`
- tiny preview
- recent-session strip 업데이트
- raw thumbnail
- fast preview가 먼저 보였다는 사실 alone
- `previewReady` 이전의 어떤 고객 화면 변경

## 최근 브랜치와 워크트리 맵

### 이번 조사에서 기준으로 본 최근 worktree 2개

1. `C:\Code\Project\Boothy_thumbnail-latency-seam-reinstrumentation`
   - branch: `feature/thumbnail-local-renderer-next-attempt`
   - HEAD: `9eaf357`
   - 성격: 기존 local/dedicated renderer 경로를 더 빠르게 만들기 위한 마지막 미세조정 축

2. `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40`
   - branch: `feature/architecture-refactor-product-update`
   - HEAD: `16e637e`
   - 성격: 기존 activation 축의 한계를 인정하고, primary architecture 정의와 evidence/governance를 다시 잠근 축

### 해석

- 첫 번째 축은 "당시 경로를 더 다듬으면 닫힐 수 있는가"를 확인한 작업 흐름이다.
- 두 번째 축은 "그 축만으로는 KPI를 닫기 어렵다"는 결론 위에서 제품 기준과 다음 아키텍처 방향을 다시 정리한 흐름이다.

---

## 시간순 히스토리

## Phase 1. 촬영 안정화와 카메라/helper 경계 복구

### 시기

- `2026-03-29` ~ `2026-04-01`

### 조사한 문제

- `helper-binary-missing`
- `Phone Required` 과승격
- duplicate shutter 오진
- follow-up capture `capture-download-timeout`
- capture 성공 후 preview/render 실패가 카메라 실패처럼 보이는 문제
- legacy preset bundle 호환성 문제
- `render-cli-missing`

### 주요 시도

- helper 실행 경계 복구
- capture timeout budget 확대
- callback 경계에서 RAW transfer 직접 처리
- preview/render 실패와 camera failure를 분리 logging
- legacy preset bundle fallback 추가
- render CLI resolution fallback 보강
- preview asset truth 복구
- helper backfill과 active capture 경계 분리

### 검증 결과

- 카메라 자체와 helper readiness는 크게 안정화됐다.
- `2026-03-29` 기준 연속 촬영 정상 동작과 camera 상태 유지가 확인됐다.
- 이 단계의 주요 교훈은 "카메라 촬영 성공"과 "preview close 성공"은 다른 문제라는 점이다.

### 후속 조사로 이어진 관찰

- camera/helper 축의 기초 안정화 이후에도 follow-up capture timeout 회귀는 별도 추적 대상으로 남았다.

---

## Phase 2. low-risk 미세조정과 darktable path 경량화

### 시기

- 주로 `2026-04-01` ~ `2026-04-03`

### 핵심 가설

- 당시 병목이 UI가 아니라 preview render 자체라면, preview 크기/프로파일/실행 방식 최적화로 상당 부분 줄일 수 있을 수 있다는 가설을 세웠다.

### 조사 및 시도한 방법

- preview를 display-sized low-res raster로 낮춤
- custom preset 로딩 축소
- warm-up 추가
- `darktable-cli` stdout/stderr 처리 방식 정리
- preview lane에서 `--disable-opencl` 적용
- helper fast preview wait budget 확대
- pending fast preview queue 유지
- pending first-visible 확보 시 다음 촬영을 더 빨리 허용

### 반복 측정에서 본 수치

- `capture_preview_ready`가 계속 대략 `6.8s ~ 7.6s`대에 머묾
- UI lag는 작았고, 병목은 UI 반영이 아니라 preview 생성 자체였음

### 결론

- low-risk 미세조정은 거의 소진됐다.
- 병목은 `warm-up 부족`이나 `stdout pipe` 같은 주변 요인보다 preview 생성 경로 자체에 더 가깝다는 판단이 강화됐다.

### 상태 분류

- `실패라기보다 한계 확인`
- 다시 같은 부류의 튜닝만 반복하는 것은 가치가 낮다.

---

## Phase 3. seam 계측과 first-visible / replacement close 분리

### 시기

- `2026-04-03` ~ `2026-04-10`

### 왜 필요했는가

문제가 "사진이 늦게 뜬다"로 뭉뚱그려져 있으면 다음 시도의 목표가 계속 흐려졌다.

그래서 시간 seam을 둘로 나눠 계측했다.

- seam 1: `request-capture -> first-visible`
- seam 2: `first-visible -> preset-applied same-slot replacement close`

### 새로 정리한 관점

- `first-visible`은 고객 안심 신호다.
- `replacement close`는 제품이 실제로 닫히는 순간이다.
- 둘을 구분하지 않으면 3초대 안심 신호를 2.5초 제품 성공처럼 오해하게 된다.

### 이 단계에서 만든 것

- `capture_preview_transition_summary`
- session timing event 집계
- same-slot truthful replacement 해석 기준

### 확인된 사실

- 이 단계 이후 제품은 두 seam을 모두 로그로 읽을 수 있게 됐다.
- 다만 "측정 가능"과 "바로 판단 가능"은 다르다.
- 이후 모든 시도는 capture 단위 요약과 evidence bundle 위에서 읽어야 한다.

### 상태 분류

- `성공`
- 이 단계는 문제 해결이 아니라 문제를 정확히 읽을 수 있게 만든 baseline이다.

---

## Phase 4. dedicated renderer sidecar와 activation/canary 전환

### 시기

- `2026-04-10` ~ `2026-04-14`

### 목표

- first-visible과 truthful close owner를 분리하고, dedicated renderer를 host-owned sidecar boundary로 운영해 same-slot truthful replacement를 더 빠르게 닫을 수 있는지 검증

### 실제로 한 일

- dedicated renderer accepted result를 truthful close owner에 연결
- `preview-renderer-policy.json` 기반 route control
- warm-state evidence 도입
- route promotion / rollback semantics 정착
- speculative close와 dedicated close 경로 동시 경쟁 제거
- close cap 축소
- early fast preview 중복 승격 제거

### 실제로 확인된 사실

- `laneOwner=dedicated-renderer`
- `routeStage=canary`
- `warmState=warm-hit`
- `fallbackReason=none`

즉, 새 activation/canary 경로는 실제로 적용됐다.

### 대표 수치

- `replacementMs=5533`
- `replacementMs=4411`
- `replacementMs=4455`
- `replacementMs=3494`

### 결론

- activation 성공: 예
- route ownership 전환 성공: 예
- booth-safe semantics 개선: 예
- KPI 달성: 아니오

### 상태 분류

- `경로 증명 성공, 속도 목표 실패`

이 단계가 중요한 이유는 "새 경로가 아직 적용되지 않았다"는 가설을 끝냈기 때문이다.

---

## Phase 5. 아키텍처 재평가와 새 forward path 확정

### 시기

- `2026-04-14` ~ `2026-04-15`

### 핵심 문서 결론

- 기존 `local dedicated renderer`는 activation baseline으로는 성공
- 그러나 primary close architecture로는 부족
- 앞으로의 primary architecture는 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`
- `darktable-compatible path`는 parity/fallback/final reference로 유지
- `remote renderer / edge appliance`는 reserve option으로만 유지

### 왜 이렇게 바뀌었는가

문서 판단상 남은 병목은 activation gap이 아니라 dedicated close 구조 자체의 비용 한계다.

즉, 이 시점에 남은 문제는 단일 bug가 아니라 아래 조합이다.

- render/apply hot path 길이
- warm-state 유지 비용
- queue contention
- same-path replacement 비용

### 이 단계에서 확정한 것

- 새 KPI language
- evidence chain reset
- local full-screen lane prototype owner
- hardware canary owner
- default/rollback gate owner
- final guarded cutover owner

### 상태 분류

- `제품 기준과 아키텍처 판단 재정렬 완료`

---

## Phase 6. 최신 하드웨어 후속에서 추가 확인된 현상

### 시기

- `2026-04-16`

### 최신 확인 1: 세션 소실 race

#### 문제

- preview 완료와 warm-state 동기화가 같은 `session.json`을 동시에 swap하며 session truth가 사라질 수 있었음

#### 수정

- manifest 경로별 write 직렬화
- capture pipeline lock 아래에서 warm-state sync

#### 결과

- 세션이 초기 화면으로 튀는 manifest 소실 회귀는 막힘

### 최신 확인 2: latency와 follow-up capture timeout 재관찰

#### 최신 세션에서 본 사실

- 첫 캡처 `replacementMs=7142`
- 두 번째 요청은 `capture-accepted`까지 간 뒤 `capture-download-timeout`

#### 의미

- session truth 소실과는 별개로
- preview close 자체가 계속 느렸고
- 연속 촬영에서 helper completion boundary도 다시 병목이 됨

### 최신 확인 3: dedicated renderer 자체가 darktable preview run에 묶여 있었음

후속 기록상 남은 병목은 점점 더 `dedicated renderer가 darktable-cli로 preset close를 만드는 자체 시간`으로 좁혀졌다.

### 상태 분류

- `구조 전환 이후에도 목표 close를 만드는 hot path를 확보하지 못함`

---

## 지금까지 조사하고 시도한 방법 목록

아래는 에이전트가 "무엇을 이미 해봤는지"를 빠르게 훑기 위한 압축 목록이다.

| 분류 | 조사/시도 | 의도 | 결과 | 평가 |
| --- | --- | --- | --- | --- |
| camera/helper | helper 실행/timeout/readiness 안정화 | 촬영 경계 자체 복구 | 기본 안정화 성공 | 유지 |
| preset bundle 호환 | legacy bundle fallback | preview render 회복 | correctness 회복 | 유지 |
| render dependency | render-cli fallback | darktable PATH 의존 완화 | correctness 회복 | 유지 |
| preview 크기 | low-res/display-sized preview | preview 생성 시간 절감 | 일부 개선, KPI 미달 | 단독 해법 아님 |
| 실행 방식 | stdout/stderr 처리 정리 | pipe 병목 제거 | 본질 병목 아님 | 비교 기록만 유지 |
| OpenCL | preview lane `--disable-opencl` | 초기화 비용 제거 | 의미 제한적 | 반복 우선순위 낮음 |
| warm-up | preload / warm worker | cold start 이전 비용 이동 | 일부 효과, 결정타 아님 | 보조 |
| helper wait budget | 120ms -> 360ms -> rollback | same-capture fast preview 적중률 향상 | blocking 회귀 발생 | 폐기/주의 |
| UX policy | pending preview 시 다음 촬영 허용 | 체감 blocking 감소 | 흐름은 개선 | 보조 정책 |
| seam logging | first-visible / replacement 분리 | 원인 분리 | 성공 | 필수 baseline |
| dedicated renderer | sidecar close owner | same-slot truthful replacement 가속 | activation 성공, KPI 실패 | baseline only |
| speculative close 제거 | happy path 중복 렌더 차단 | 경쟁 제거 | 적용 성공 | 유지 |
| close cap 축소 | smaller display cap | close 비용 축소 | 개선 제한적 | 미세조정 축 |
| manifest serialization | session truth 보존 | 세션 소실 방지 | 성공 | 유지 |
| timeout 확대 | helper/host timeout 증가 | follow-up capture 실패 완화 | 일부 완화, 근본 해결 아님 | 보조 |
| evidence/governance reset | 1.21~1.25 | KPI, trace, canary, default/rollback 재정렬 | 문서/계약 잠금 성공 | 유지 |

---

## 조사로 확인된 사실과 남은 가설

### 이미 증명된 것

- first-visible과 truthful close는 다른 seam이다.
- UI는 주 병목이 아니다.
- legacy darktable-only blocking close만으로는 제품 기준이 어렵다.
- dedicated renderer activation/canary는 실제로 적용 가능하다.
- route policy, warm-state, evidence bundle, rollback semantics를 운영 계약으로 다룰 수 있다.
- selected-capture evidence chain을 새 track 기준으로 재정렬할 수 있다.
- local lane prototype, canary verdict, default/rollback gate를 문서/계약/테스트로 잠글 수 있다.

### 추가 검증이 필요했던 가설

- approved hardware에서 `sameCaptureFullScreenVisibleMs <= 2500ms`
- repeated canary success로 `Go`를 줄 수 있는 local lane
- one-action rollback proof가 포함된 final release close
- follow-up capture timeout이 충분히 사라졌다는 것

---

## 무엇을 반복하지 말아야 하는가

- `first-visible` 개선만 보고 성공으로 해석하지 말 것
- tiny preview, recent strip update, raw thumbnail을 release success로 취급하지 말 것
- low-risk invocation tuning을 다시 처음부터 반복하지 말 것
- dedicated renderer activation 미적용 가설로 다시 조사하지 말 것
- live policy/catalog를 나중에 다시 읽어 옛 capture를 재해석하지 말 것
- Story `1.18 ~ 1.20` legacy evidence를 final release close 근거처럼 사용하지 말 것
- Story `1.21 ~ 1.25` 완료를 곧바로 release `Go`로 해석하지 말 것

---

## 에이전트용 읽기 순서

새 에이전트가 이 영역을 이어받을 때 권장 읽기 순서는 아래와 같다.

1. 이 문서
2. `docs/release-baseline.md`
3. `history/camera-capture-validation-history.md`
4. `history/thumbnail-replacement-timing-history.md`
5. `history/recent-session-thumbnail-speed-agent-context.md`
6. `_bmad-output/planning-artifacts/preview-architecture-reassessment-report-20260414.md`
7. `_bmad-output/planning-artifacts/architecture-change-proposal-20260415.md`
8. `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
9. `_bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md`

이 순서를 권장하는 이유는 다음과 같다.

- 먼저 큰 그림과 판단 맥락을 이해하고
- 그다음 최신 하드웨어 증거와 seam 정의를 확인하고
- 마지막에 governance와 release gate를 읽는 편이 재작업을 줄인다.

---

## 주요 근거 문서 맵

- 히스토리
  - `history/recent-session-thumbnail-speed-brief.md`
  - `history/recent-session-thumbnail-speed-agent-context.md`
  - `history/thumbnail-replacement-timing-history.md`
  - `history/camera-capture-validation-history.md`
- 제품/아키텍처 판단
  - `_bmad-output/planning-artifacts/preview-architecture-reassessment-report-20260414.md`
  - `_bmad-output/planning-artifacts/architecture-change-proposal-20260415.md`
  - `_bmad-output/planning-artifacts/research/technical-boothy-preview-architecture-alternatives-research-20260414.md`
  - `_bmad-output/planning-artifacts/research/technical-boothy-high-risk-preview-architecture-options-research-20260414.md`
- release/governance reference
  - `docs/release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md`
