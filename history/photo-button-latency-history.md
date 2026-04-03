# `사진 찍기` 버튼 이후 사용자 체감 속도 히스토리

## 목적

이 문서는 booth 앱에서 고객이 `사진 찍기` 버튼을 누른 뒤
실제로 무엇을 얼마나 빨리 보게 되는지에 대한 이력만 따로 정리한 문서다.

다음 회차에서 속도를 더 개선할 때 아래 질문에 바로 답할 수 있게 하는 것이 목적이다.

- 지금 제품이 채택한 최신 방식이 정확히 무엇인가
- 어디까지는 correctness 문제였고, 어디서부터 latency 문제로 바뀌었는가
- 어떤 변경이 실제로 도움이 되었고, 어떤 시도는 구조적으로 한계가 있었는가
- 지금 추가 계측이 필요한 구간은 어디인가

이 문서는 다음 기존 문서들의 속도 중심 압축판이다.

- [camera-capture-validation-history.md](/C:/Code/Project/Boothy/history/camera-capture-validation-history.md)
- [current-session-photo-troubleshooting-history.md](/C:/Code/Project/Boothy/history/current-session-photo-troubleshooting-history.md)
- [camera-helper-troubleshooting-history.md](/C:/Code/Project/Boothy/history/camera-helper-troubleshooting-history.md)

## 이 문서가 다루는 범위

포함:

- booth 앱의 `사진 찍기` 버튼을 누른 직후부터 고객 화면에 첫 피드백이 뜨는 구간
- `Preview Waiting` 진입, 실제 preview 생성, 현재 세션/최근 세션 반영 속도
- 속도 문제를 만들었던 프런트 readiness 갱신 방식
- darktable preview 경로의 현재 설정과 한계

제외:

- 카메라 연결/탐지 일반 이력
- helper binary 누락, SDK attach, BOM parser 같은 준비 단계 일반 문제
- 제품 정책으로 아직 승인되지 않은 새 UX 아이디어 결정

## 속도 문제를 볼 때의 기준선

속도 이슈는 아래 경계를 분리해서 봐야 한다.

1. 고객이 `사진 찍기` 버튼을 누른다.
2. host가 capture request를 받아 RAW 저장 경계를 닫는다.
3. 고객 화면이 즉시 truthful한 `Preview Waiting`으로 전환된다.
4. preview render가 시작된다.
5. 실제 preview raster가 준비된다.
6. 현재 세션 화면에서 해당 사진이 보인다.
7. 필요하면 최근 세션 영역에도 같은 사진이 보인다.

중요한 점:

- `사진이 저장됐다`와 `preview가 보인다`는 같은 사건이 아니다.
- 고객 경험상 둘 다 중요하지만, 원인과 해결책은 다를 수 있다.
- 지금까지의 경험상 `Phone Required` 재발, helper timeout, stale preview truth, UI polling 간섭은 서로 다른 문제다.

## 현재 최신 상태 요약

`2026-04-02` 기준 최신 상태는 아래처럼 정리한다.

- correctness 기준으로는 `사진 찍기 -> Preview Waiting -> 실제 사진 반영` 경계가 다시 동작하는 최신 기준선이 있다.
- 다만 실제 장비에서 preview가 보이기까지의 체감 속도는 여전히 제품 기준으로 빠르다고 보기 어렵다.
- 추가로 발견된 프런트 문제로, readiness 새로고침 중 `사진 찍기` 버튼이 반복적으로 깜빡이거나 클릭이 먹지 않는 문제가 있었고, 이 문제는 별도로 수정했다.
- 따라서 현재 상태는 `기본 흐름은 다시 열렸지만, preview latency 최적화는 아직 진행 중`으로 보는 것이 맞다.

## 결국 지금 제품이 채택한 최신 방법

현재 booth는 아래 방식을 최종 current method로 쓰고 있다.

### 1. supported capture 시작점은 booth 앱의 `사진 찍기` 버튼이다

- 카메라 본체 셔터 직접 입력은 supported booth success path가 아니다.
- 속도 개선도 반드시 이 버튼 경로 기준으로만 판단해야 한다.

### 2. capture 성공은 RAW 저장 시점에서 먼저 닫는다

- 버튼을 눌렀다고 곧바로 성공으로 보지 않는다.
- helper/host가 RAW handoff를 실제로 닫아야 capture success다.
- 이 시점에서 고객 화면은 즉시 `Preview Waiting`으로 넘어가고, preview는 뒤에서 비동기로 준비한다.

### 3. preview는 darktable 기반 actual render를 비동기로 만든다

현재 preview render 설정:

- preview intent는 `--hq false`
- preview size cap은 `1280x1280`
- final render는 기존 high-quality 경로를 유지

즉 현재 제품은
`capture 직후 빠른 truthful waiting -> 뒤에서 display-sized actual preview render`
구조를 쓰고 있다.

### 4. 고객에게 보이는 속도를 실제 로그로 남긴다

현재 남기는 핵심 로그:

- render worker 시작: `render_job_started`
- preview 준비 완료: `preview-render-ready`, `capture_preview_ready`
- 현재 세션 이미지 실제 가시화: `current-session-preview-visible`

중요한 빈칸:

- `최근 세션` 가시화 시점은 아직 별도 이벤트로 완전히 분리되어 있지 않다.
- 따라서 다음 최적화 회차에서는 `current session visible`과 `recent session visible`을 분리 계측하는 편이 좋다.

### 5. readiness는 하나의 구독/폴링 루프를 기준으로 본다

현재 최신 프런트 방식:

- capture runtime service가 `300ms` 기준의 readiness poll을 책임진다.
- session provider가 그 위에 별도 interval을 하나 더 얹어 중복 폴링하지 않도록 정리했다.
- background readiness refresh 때문에 버튼 자체를 busy 처리하지 않도록 보정했다.

즉 지금은
`background 상태 확인`과 `실제 촬영 버튼 상호작용`을 분리하는 쪽으로 정리돼 있다.

## 우리가 했던 일들

### 1. 먼저 correctness를 복구했다

초기 단계의 문제는 속도보다 correctness 붕괴에 가까웠다.

대표 증상:

- 버튼을 눌러도 오래 로딩되다가 `Phone Required`로 떨어짐
- 첫 촬영 뒤 두 번째 촬영에서 세션이 흔들림
- stale request replay나 helper timeout 때문에 새 request와 실제 셔터 결과가 어긋남
- preview가 실제 raster가 아니어도 잘못 ready로 보이던 구간이 있었음

이 단계에서 했던 핵심 일:

- helper binary/runtime attach 복구
- stale request replay 방지
- follow-up capture correlation 복구
- transient readiness를 `Phone Required`로 과승격하지 않도록 프런트 보정
- 실제 raster preview가 없으면 preview ready로 올리지 않도록 truth 정리

이 작업의 결과:

- 제품 문제의 중심이 `아예 안 된다`에서 `되긴 되는데 느리다`로 이동했다.

### 2. `Preview Waiting`을 truthful한 상태로 고정했다

속도 개선을 하려면 먼저 기다림 상태가 정직해야 했다.

그래서 아래 원칙을 유지하게 됐다.

- capture saved와 preview ready를 분리한다.
- preview가 실제로 준비되기 전까지는 `Preview Waiting`을 유지한다.
- 잘못된 placeholder나 stale asset을 ready 근거로 쓰지 않는다.

이 원칙 덕분에
나중에 속도 실험을 하더라도 "빨라 보이지만 실제 truth는 틀린 상태"로 회귀할 가능성을 줄였다.

### 3. preview render 자체를 줄여 봤다

`2026-04-01` 회차에서 preview를 full-size 대신 display-sized actual render로 낮췄다.

의도:

- preview가 booth 화면용이라면 굳이 full-resolution일 필요는 없다고 보고
  darktable 비용을 줄여 보려 했다.

현재 남아 있는 수동 측정 결과:

- full-size preview render: 약 `8652ms`
- `1280x1280` low-res preview render: 약 `5973ms`
- `640x640` low-res preview render: 약 `6894ms`

이 시도의 의미:

- 분명 full-size보다는 나아졌지만,
  이 장비에서는 darktable 기반 RAW render 자체가 여전히 수초 단위다.
- 따라서 현재 병목은 단순히 프런트 stale update나 작은 상태 sync 문제만으로 설명되지 않는다.

### 4. 고객 입력 자체를 방해하던 프런트 readiness 간섭을 줄였다

`2026-04-02`에는 다른 종류의 제품 문제를 추가로 확인했다.

증상:

- `사진 찍기` 버튼이 계속 깜빡여 보임
- 버튼을 눌러도 높은 확률로 아무 반응이 없는 것처럼 느껴짐

실제 원인:

- background readiness refresh가 돌 때마다 버튼이 잠깐 busy/disabled 상태로 바뀌고 있었다.
- 게다가 session provider와 capture runtime service가 readiness를 중복으로 다시 읽는 구조가 겹쳐,
  버튼의 체감 안정성을 더 흔들 수 있었다.

이번 회차 조치:

- background readiness refresh 때문에 `사진 찍기` 버튼을 busy 처리하지 않도록 수정
- readiness polling 책임을 service 쪽 단일 루프로 정리하고, provider 쪽 중복 interval 제거

현재 의미:

- 이 수정은 preview render를 직접 빠르게 만들지는 않는다.
- 대신 `누를 수 있는 버튼` 자체의 신뢰성을 회복해,
  실제 latency 측정을 왜곡하던 UI 간섭을 줄였다.

검증:

- `pnpm vitest run src/booth-shell/screens/CaptureScreen.test.tsx`
- `pnpm vitest run src/session-domain/state/session-provider.test.tsx -t "does not start an extra readiness retry loop on top of the subscription channel"`
- `pnpm vitest run src/session-domain/state/session-provider.test.tsx -t "invalidates an in-flight capture request when a newer subscribed blocked readiness arrives"`

메모:

- `session-provider` 전체 테스트 파일은 매우 커서 현재 세션에서는 전체 실행이 시간 제한에 걸렸다.
- 따라서 이번 회차에서는 `버튼 안정성`과 `중복 readiness loop 제거`에 직접 연결된 targeted test만 다시 확인했다.

## 원인에 대한 현재 해석

지금까지의 evidence를 합치면, `사진 찍기` 이후 체감 속도 문제는 한 가지 원인으로 설명되지 않는다.

### A. 이미 해결된 원인들

- transient readiness를 과하게 실패로 해석하던 프런트 문제
- helper replay/correlation 붕괴
- 실제 preview truth가 없는데 ready처럼 보이던 문제
- background readiness refresh가 버튼 클릭 자체를 방해하던 문제

### B. 아직 남아 있을 가능성이 큰 원인

- darktable preview render 자체의 구조적 비용
- `render ready -> 실제 화면 visible` 사이 구간의 추가 지연
- `current session visible -> recent session visible` 사이 구간의 별도 지연
- 연속 촬영에서 helper timeout이나 transfer budget이 다시 latency를 키우는 경우

## 현재 상태를 한 줄로 말하면

현재 booth는
`truthful waiting과 기본 correctness는 회복했고, 버튼 입력 안정성도 다시 정리했지만, actual preview latency는 아직 product-grade로 닫히지 않았다`
상태다.

## 다음 속도 개선 회차에서 바로 써야 할 조사 질문

다음 회차에서는 아래 질문을 순서대로 닫는 것이 좋다.

### 1. 어디가 가장 느린가

최소 아래 시점을 한 capture 기준으로 모두 찍어야 한다.

- button pressed
- capture accepted
- raw persisted
- preview render started
- preview render ready
- current session preview visible
- recent session visible

현재는 이 중 일부만 확실히 남고 있으므로, 다음 회차의 첫 작업은 계측 보강이 맞다.

### 2. darktable render가 절대 병목인가

확인해야 할 것:

- 같은 RAW/XMP 기준으로 darktable 실행 자체가 거의 전부를 차지하는가
- 아니면 render 완료 후 manifest/readiness 반영과 이미지 가시화에도 의미 있는 지연이 있는가

지금까지는 darktable 비용이 매우 크다는 evidence가 강하지만,
후속 UI 구간을 완전히 배제할 만큼 계측이 충분하지는 않다.

### 3. booth 제품이 원하는 "빠른 첫 피드백"을 actual preview와 분리할지

현재는 representative tile/sample-cut을 기본 fallback으로 채택하지 않았다.

즉 다음 선택지는 크게 둘이다.

- darktable를 거치지 않는 더 빠른 same-capture thumbnail source를 찾는다.
- 또는 `빠른 proxy`와 `나중 actual render`를 제품 설계상 명확히 분리한다.

이 결정은 기술 문제가 아니라 제품 결정이기도 하므로,
다음 회차에서는 correctness와 제품 truth를 같이 논의해야 한다.

## 다음 작업에서 절대 잊지 말 것

- `Preview Waiting`을 거짓으로 단축하지 말 것
- transient host 재확인 상태를 다시 `Phone Required`로 과승격하지 말 것
- background polling 때문에 버튼이 비활성화되는 회귀를 다시 만들지 말 것
- speed 실험 중에도 supported capture trigger는 계속 booth 앱의 `사진 찍기` 버튼이어야 할 것
- 연속 촬영 회귀와 preview latency 문제를 같은 버그로 섞어 읽지 말 것

## 추천 시작점

다음 속도 개선 회차는 아래 순서로 시작하는 편이 좋다.

1. `button pressed -> recent session visible` 전체 타임라인을 한 requestId 기준으로 로그 상에서 완전히 연결한다.
2. `recent session visible` 전용 계측을 추가한다.
3. 실제 장비에서 구간별 시간을 다시 측정한다.
4. 그 다음에야 darktable 대안, fast proxy, same-capture thumbnail source를 비교한다.

## 참고 코드 위치

- capture runtime polling: `src/capture-adapter/services/capture-runtime.ts`
- session readiness 적용: `src/session-domain/state/session-provider.tsx`
- customer capture 화면: `src/booth-shell/screens/CaptureScreen.tsx`
- current session 이미지 가시화 로그: `src/booth-shell/components/SessionPreviewImage.tsx`
- preview render invocation: `src-tauri/src/render/mod.rs`
- capture 이후 preview 완료 로그: `src-tauri/src/commands/capture_commands.rs`

## 마지막 업데이트

- `2026-04-02`: background readiness refresh 때문에 `사진 찍기` 버튼이 깜빡이거나 클릭이 무시되던 회귀 수정 반영
- 이 문서는 이후 속도 개선 작업의 기준 문서로 유지한다
