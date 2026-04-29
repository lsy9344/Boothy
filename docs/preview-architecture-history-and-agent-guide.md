# Preview Architecture History And Agent Guide

## 목적

이 문서는 Boothy의 촬영 후 preview/thumbnail/풀스크린 close 문제를 새 에이전트가 다시 처음부터 파헤치지 않도록 만드는 운영 문서다.

이 문서의 목표는 네 가지다.

- 과거부터 기준 시점까지 무엇을 조사했고 무엇을 시도했는지 한 번에 보이게 한다.
- 어떤 시도가 실패했는지와 왜 실패했는지를 제품 기준으로 정리한다.
- 어떤 개념 전환과 제품 기준이 형성됐는지를 조사 맥락과 함께 정리한다.
- 다음 에이전트가 어디부터 읽고, 무엇을 다시 검증하고, 무엇은 반복하지 말아야 하는지 지침을 남긴다.

기준 시점은 `2026-04-16`이다.

## 현재 워크트리에서 이 문서를 읽는 방법

- `2026-04-19` 기준으로, 이 worktree는 과거 `resident first-visible` 중심 라인을 다시 검증하는 `validation lane`이다.
- 이것은 과거 경로를 자동 승격하거나 그대로 release-proof로 되돌린다는 뜻이 아니다.
- 최신 runbook 해석상 newer `actual-primary-lane`은 bounded `No-Go`이고, 그래서 old lane을 다시 보는 것이다.
- old lane은 historically better product feel candidate일 뿐이며, current official release gate는 `preset-applied visible <= 3000ms`이고 측정값으로는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`다.
- GPU 활성/가속도 이 validation lane 안에서 함께 볼 가설일 뿐, 공식 성공 보장은 아니다.

---

## 먼저 알아야 할 조사 결론

- 문제는 "새 preview 아키텍처가 아직 안 켜져 있다"가 아니었다.
- `local dedicated renderer + first-visible lane 분리` 구조는 실제 하드웨어 canary까지 적용됐다.
- 하지만 이 구조는 공식 제품 게이트인 `preset-applied visible <= 3000ms`를 반복적으로 닫지 못했다.
- 따라서 기존 dedicated renderer 경로는 `activation baseline`과 `evidence contract proof`로는 성공했지만, 최종 primary close architecture로는 부족하다는 판단이 문서화되어 있다.
- `2026-04-16` 기준 판단에서는 이후 forward path가 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact` 쪽으로 재정렬됐다.
- `2026-04-20` 기준으로는 old `resident first-visible` line이 closed `No-Go` baseline으로 확정됐고, 그 forward path가 Story `1.26` reserve path로 공식 오픈됐다.
- 최신 후속 조사에서는 세션 소실 race, preview close latency, follow-up capture timeout이 서로 다른 문제축으로 다시 분리되어 기록됐다.
- `2026-04-29` false-Go 조사 이후에는 native RAW partial approximation을 official truth로 승격하지 않는다. 선택한 다음 방향은 실제 full preset 결과를 만드는 `resident/long-lived darktable-compatible` engine path를 제품 후보로 다시 세우는 것이다.

---

## 제품 기준과 성공 조건

### Canonical KPI

- primary release sign-off: `preset-applied visible <= 3000ms` (`originalVisibleToPresetAppliedVisibleMs <= 3000ms`)
- `sameCaptureFullScreenVisibleMs`: first-visible/reference/comparison metric only, not an official release gate
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
- 둘을 구분하지 않으면 3초대 안심 신호를 3초 제품 성공처럼 오해하게 된다.

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

## Phase 7. 2026-04-29 false-Go 이후 방향 재선택

### 이 문서에 이미 있던 내용

- 과거 문서에는 `resident first-visible` line이 historically better product feel candidate였다는 기록이 있다.
- Phase 5에는 `darktable-compatible path`를 parity/fallback/final reference로 유지한다는 판단이 있다.
- 즉, resident/darktable 계열의 재료는 이미 있었다. 다만 그것을 Story `1.26`의 다음 primary 제품 경로로 다시 선택한다는 결론은 아직 명시되어 있지 않았다.

### 새로 확정한 판단

- native RAW partial approximation은 충분히 빠를 수 있지만, 실제 full preset 결과가 아니면 official truth가 아니다.
- 2026-04-29 false-Go에서 over-white output과 비현실적으로 낮은 timing이 같은 원인에서 나왔다. 원인은 native approximation을 `original-full-preset`처럼 승격한 것이다.
- 따라서 다음 제품 방향은 작은 native operation 일부를 더 흉내 내는 쪽이 아니다.
- 선택한 2번 방법은 실제 preset engine을 계속 살아 있게 만드는 `resident/long-lived darktable-compatible` close path다.
- 이 방향은 per-capture `darktable-cli` fallback tail을 반복 튜닝하자는 뜻이 아니다. 목표는 실제 full preset fidelity를 유지한 채 cold start, process spawn, queue jitter를 줄이는 것이다.
- host-owned native/GPU path는 full preset parity를 검증할 수 있을 때만 다시 official truth 후보가 된다. 그 전에는 comparison evidence로만 남긴다.

### 다음 에이전트 지침

- `inputSourceAsset=fast-preview-raster`, `profile=operation-derived`, darktable-backed preview fallback, parity 없는 native output은 official truth로 승격하지 말 것.
- `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, `truthProfile=original-full-preset`을 주장하려면 실제 preset engine ownership이나 동등한 parity proof가 필요하다.
- 우선 구현 후보는 resident/long-lived darktable-compatible worker, libdarktable-style in-process owner, 또는 같은 제품 의미를 만족하는 장기 실행 preset renderer다.
- 이번 방향은 과거의 "darktable fallback 튜닝"과 다르다. fallback 결과를 더 빨리 보이게 하는 것이 아니라, 실제 preset result owner를 제품 hot path로 만드는 것이 목표다.
- 구현은 바로 release path로 보지 말고 bounded spike로 시작한다. resident engine이 full preset fidelity, same-capture ownership, approved-hardware timing을 동시에 증명해야 다음 gate로 간다.
- current `darktable-cli` output은 reference/fallback/final correctness evidence로 남긴다. per-capture fallback이 빠른 run 하나를 만들더라도 새 reserve path 성공으로 세지 않는다.

---

## Phase 8. 2026-04-29 option 2 구현 결과

### 현재 정답

- Story `1.26`의 현재 정답은 option 2다.
- 실제 preset engine을 booth hot path 안에 두는 `resident/long-lived darktable-compatible full-preset` 경로다.
- 이 경로는 원본 RAW를 입력으로 받아 같은 촬영본의 `preset-applied-preview`를 만든다.
- 최신 approved hardware run `hardware-validation-run-1777434275752`가 `5/5` 통과했다.

### 왜 정답으로 보는가

- accepted route가 `inputSourceAsset=raw-original`을 갖는다.
- accepted route가 `sourceAsset=preset-applied-preview`를 갖는다.
- accepted route가 `truthOwner=display-sized-preset-applied`를 갖는다.
- accepted route가 `truthProfile=original-full-preset`을 갖는다.
- accepted route가 `engineMode=resident-full-preset`과 `engineAdapter=darktable-compatible`을 갖는다.
- official timing은 `2316ms ~ 2338ms`로 3초 안이다.

### 계속 금지할 것

- native RAW partial approximation을 official truth로 승격하지 않는다.
- fast-preview-raster 기반 결과를 official truth로 승격하지 않는다.
- operation-derived profile을 official truth로 승격하지 않는다.
- per-capture darktable fallback을 Story `1.26` 성공으로 세지 않는다.
- full-preset parity proof가 없는 host-owned output을 성공으로 세지 않는다.

### 다음 변경 시 확인할 것

- route fields가 유지되는지 먼저 본다.
- timing만 나빠졌다면 resident owner, path reuse, warm state를 먼저 본다.
- route fields가 사라졌다면 fallback 튜닝이 아니라 resident full-preset generation/promotion path를 복구한다.
- 자세한 current answer record는 `_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md`와 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`를 따른다.

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
| native RAW approximation | host-owned native 비교 출력 | reserve path 후보 검토 | false-Go/over-white 확인 | official truth 금지 |
| resident/long-lived preset engine | darktable-compatible engine을 계속 살려 실제 preset 결과 생성 | full preset fidelity 유지 + process/cold cost 제거 | 2026-04-29 approved hardware `5/5` Go | 현재 Story 1.26 정답 |
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
- 과거 better run은 존재했지만, 현재 공식 `preset-applied visible <= 3000ms` 게이트를 닫았다고 증명된 historical architecture는 아직 없다.

### 추가 검증이 필요했던 가설

- approved hardware에서 `preset-applied visible <= 3000ms` (`originalVisibleToPresetAppliedVisibleMs <= 3000ms`)
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

1. `docs/README.md`
2. `docs/runbooks/current-preview-gpu-direction-20260419.md`
3. 이 문서
4. `docs/runbooks/current-actual-lane-handoff-20260419.md`
5. `docs/runbooks/preview-track-route-decision-20260418.md`
6. `docs/release-baseline.md`
7. `history/README.md`
8. `docs/contracts/render-worker.md`
9. `docs/contracts/session-manifest.md`

이 순서를 권장하는 이유는 다음과 같다.

- 먼저 canonical doc map과 GPU 방향 판단을 읽고
- 그다음 current route decision과 release gate를 이해하고
- 마지막에 history/contract 문서를 읽는 편이 current worktree 목적과 충돌을 줄인다.

---

## 주요 근거 문서 맵

- 히스토리
  - `history/recent-session-thumbnail-speed-brief.md`
  - `history/recent-session-thumbnail-speed-agent-context.md`
  - `history/camera-capture-validation-history.md`
- 제품/아키텍처 판단
  - `docs/runbooks/current-actual-lane-handoff-20260419.md`
  - `docs/runbooks/preview-track-route-decision-20260418.md`
  - `_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md`
  - `_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md`
- release/governance reference
  - `docs/release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
