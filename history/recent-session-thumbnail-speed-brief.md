# 최근 세션 썸네일 속도 단축 브리프

## 목적

이 문서는 booth 앱에서 고객이 `사진 찍기` 버튼을 누른 뒤
`현재 세션 사진` 레일에 같은 촬영의 썸네일이 보이기까지 걸리는 시간을 줄이기 위한
조사 내용, 근거, 의견, 가설, 구현 계획을 한 곳에 모아 둔 문서다.

다음 구현 에이전트는 이 문서를 기준으로 작업한다.

관련 기존 문서:

- [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md)
- [current-session-photo-troubleshooting-history.md](/C:/Code/Project/Boothy/history/current-session-photo-troubleshooting-history.md)
- [_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md)

## 2026-04-03 실장비 업데이트

최신 실장비 재검증 결과,
최근 세션 썸네일 경로는 이제 설계 의도대로 동작한다.

확인된 점:

- `사진 찍기` 직후 `Preview Waiting`이 먼저 진입한다.
- 같은 촬영의 fast preview가 `현재 세션 사진` 최신 슬롯에 먼저 나타난다.
- 나중에 preset-applied preview가 같은 슬롯을 교체한다.
- 즉 Story 1.9가 목표로 한 `first-visible same-capture preview -> later preset replacement` 흐름은 실장비에서 재현됐다.

이번 확인으로 정리된 남은 문제:

- 최근 세션 썸네일 blank waiting 자체는 완화됐지만, 콜드스타트와 preset-applied preview 준비 시간은 여전히 체감상 길다.
- 따라서 다음 성능 작업의 중심은 `fast thumbnail이 보이느냐`가 아니라 `cold start`, `preview render warm-up`, `preset apply elapsed`를 줄이는 쪽으로 이동했다.
- 이 변경으로 고객 불안은 줄었지만, 전체 체감 속도는 아직 충분히 빠르다고 보기 어렵다.

연결 증거:

- 스토리 기록: [_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md](/C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md)
- 하드웨어 검증 장부: [_bmad-output/implementation-artifacts/hardware-validation-ledger.md](/C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/hardware-validation-ledger.md)

## 이번 검토에서 확인한 현재 상태

이 문서는 `2026-04-02` 현재 워킹트리 기준으로 다시 검토했다.

이번 검토에서 확인한 점:

- fast thumbnail 이벤트 브리지와 `pendingFastPreview` 기반 recent-session 반영은 이미 구현 중이며 관련 테스트가 통과한다.
- host에서 actual preview render를 시작하기 전 넣어 두었던 `120ms` 대기는 현재 워킹트리에서 이미 제거되어 있다.
- 따라서 이 문서는 `fast thumbnail first` 방향 자체는 유지하되,
  이미 반영된 변경과 아직 비어 있는 계측 갭을 구분해서 읽어야 한다.

## 한 줄 요약

지금 느린 가장 유력한 이유는
`화면이 느려서`가 아니라
`고객에게 먼저 보여 줄 진짜 같은 촬영 썸네일이 늦게 들어와서`다.

fast thumbnail이 빨리 오면 레일은 비교적 빨리 보여 줄 수 있다.
fast thumbnail이 늦거나 실패하면 결국 더 느린 actual preview render를 기다리게 된다.

단, 이 문장은 현재 코드 구조와 과거 기록을 합친 가장 강한 가설이지,
실장비 `requestId` 단위 구간 계측으로 아직 완전히 닫힌 결론은 아니다.

## 이 문서에서 구분하는 것

이 문서는 아래 둘을 분리해서 본다.

1. 확정된 사실
2. 아직 실장비 계측이 더 필요한 가설

이 구분을 지키지 않으면
느린 원인을 UI 탓으로 잘못 보거나,
반대로 카메라 쪽 병목을 놓칠 수 있다.

## 현재까지 확정된 사실

### 1. 고객이 보는 첫 썸네일은 이미 `빠른 경로`가 따로 있다

현재 구조는 아래 순서다.

1. 버튼을 누른다.
2. helper가 같은 촬영의 fast thumbnail을 구하면 host로 `capture-fast-preview-update`를 보낸다.
3. 프런트는 이 값을 `pendingFastPreview`로 받아 `현재 세션 사진` 레일에 먼저 합친다.
4. 나중에 actual preview가 준비되면 같은 자리에서 자연스럽게 교체한다.

현재 워킹트리 기준으로는
이 fast preview update가 `file-arrived`로 capture 저장이 닫히기 전에 먼저 올라올 수 있도록
host-side bridge와 테스트가 같이 들어와 있다.

근거:

- host fast-preview emit: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L88)
- helper fast-thumbnail callback: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L697)
- provider fast-preview sync: [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx#L1539)
- rail merge: [CaptureScreen.tsx](/C:/Code/Project/Boothy/src/booth-shell/screens/CaptureScreen.tsx#L26)
- round-trip test: [capture_readiness.rs](/C:/Code/Project/Boothy/src-tauri/tests/capture_readiness.rs#L1190)

### 2. 버튼 자체가 주원인은 아니다

이전에는 background readiness refresh 때문에 버튼이 깜빡이거나 클릭이 무시되는 회귀가 있었다.
이 문제는 따로 수정되었다.

즉 지금 사용자가 느끼는 `오래 기다림`의 중심은
버튼 비활성화보다는 `사진이 실제로 레일에 늦게 뜨는 문제`다.

근거:

- 관련 기록: [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md)

### 3. 프런트가 이미지를 받은 뒤의 로딩 우선순위는 이미 올려 둔 상태다

현재 이미지 컴포넌트는 최신 카드와 pending 카드에 대해
`eager`, `sync`, `high` 우선순위를 준다.

즉 `이미지가 앱까지 들어온 뒤`의 지연은
현재 구조상 최우선 의심 구간이 아니다.

근거:

- image priority: [SessionPreviewImage.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/SessionPreviewImage.tsx#L108)

### 4. host 이벤트 polling 자체는 1차 병목으로 보기 어렵다

host는 helper event file을 `10ms` 간격으로 확인한다.
이 비용은 수초 단위 지연에 비해 매우 작다.

즉 지금 병목이 있다면
이 polling보다는 그 앞 단계인 `카메라가 fast thumbnail을 줄 때까지 기다리는 시간`일 가능성이 더 크다.

근거:

- event polling loop: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L307)
- poll sleep: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L437)

### 5. actual preview render는 여전히 무겁다

기존 측정 기록상 darktable preview render는 이미 수초 단위였다.

기록:

- full-size preview render: 약 `8652ms`
- `1280x1280` low-res preview render: 약 `5973ms`
- `640x640` low-res preview render: 약 `6894ms`

의미:

- fast thumbnail이 늦거나 실패하면
  사용자는 사실상 이 무거운 경로를 기다리게 된다.
- 따라서 actual preview 최적화도 중요하지만,
  첫 체감 개선에는 `fast thumbnail first`가 더 효과가 크다.

근거:

- 기록: [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md)

### 6. 현재 워킹트리에는 이미 반영된 속도 보정이 있다

현재 host는 capture 저장 뒤 actual preview render를 바로 시작한다.
이전에 있던 host-side `120ms` 고정 대기는 현재 워킹트리에서 제거되어 있다.

의미:

- Track C에서 말하는 `render 시작 전 대기 제거`는 새 작업이 아니라
  이미 적용된 변경의 유지/회귀 방지 항목으로 보는 편이 정확하다.
- 이제 남은 fallback 비용은 `대기 삽입`보다 actual render 자체 비용과
  fast path miss 빈도 쪽에서 봐야 한다.

근거:

- render kickoff: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L138)

## 현재 구조를 아주 쉽게 설명하면

지금 booth는 이렇게 움직인다.

1. 버튼을 누른다.
2. 카메라가 사진을 넘기기 시작한다.
3. helper가 그중에서 `작은 진짜 썸네일`을 먼저 꺼내 보려고 시도한다.
4. 성공하면 그 썸네일을 레일에 먼저 보여 준다.
5. 실패하거나 늦으면 더 무거운 preview render를 기다린다.

즉 현재 문제는
`레일이 느리다`보다
`3번이 늦거나 자주 실패하면 5번으로 떨어진다`에 더 가깝다.

## 왜 다른 테더링 프로그램들은 빨라 보이는가

이 부분은 외부 문헌 인용이 아니라
현재 제품 조사 기준의 working research다.

빠른 테더링 프로그램들은 보통 아래 원칙을 쓴다고 보는 것이 합리적이다.

1. 먼저 카메라 안의 작은 미리보기 사진을 즉시 보여 준다.
2. 큰 RAW는 그 뒤에 받는다.
3. 무거운 색 보정이나 최종 preview는 더 나중에 바꿔 낀다.

중요한 점:

- 빠른 프로그램은 `완성본`이 빨라서 빠른 것이 아니라
  `먼저 보여 줄 가벼운 진짜 사진`을 빨리 꺼내기 때문에 빠르게 느껴진다.
- 우리도 이미 같은 방향으로 가고 있지만,
  실제 장비에서 fast thumbnail이 충분히 빠르고 안정적으로 오는지 아직 더 조사해야 한다.

## 현재 가장 의심되는 원인과 남겨야 할 단서

### 1순위 가설: 카메라가 같은 촬영의 fast thumbnail을 늦게 준다

helper는 `DownloadCapture` 안에서 `TryDownloadPreviewThumbnail`을 호출한다.
즉 셔터 직후 아무 때나 바로 생기는 것이 아니라,
카메라 SDK가 transfer 이벤트를 열어 준 다음에야 다운로드를 시도한다.

이 구간이 길면 앱은 할 수 있는 일이 거의 없다.

근거:

- transfer path: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L664)
- thumbnail extraction: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L843)

### 2순위 가설: fast thumbnail 성공률이 충분히 높지 않을 수 있다

현재 thumbnail 추출은 best-effort다.
실패해도 capture 성공은 계속 유지된다.

이 말은 곧,
사용자는 에러를 보지 않고도 `왜 이렇게 늦지?`를 체감할 수 있다는 뜻이다.

즉 다음 구현에서는
`fast thumbnail이 얼마나 자주 성공하는지`
`실패하면 왜 실패하는지`
를 반드시 남겨야 한다.

근거:

- helper best-effort comment: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L714)

### 3순위 가설: fast path를 놓치면 결국 actual preview render가 기다림을 먹는다

host는 capture 저장 뒤 actual preview render를 백그라운드에서 돌린다.
이 경로는 맞는 구조지만,
fast path가 늦거나 실패했을 때는 체감상 너무 느려진다.

근거:

- render kickoff: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L139)

### 남겨야 할 단서: 일부 장비/설정에서는 camera thumbnail 자체가 약할 수 있다

이 브리프는 `camera thumbnail first`를 우선 가설로 두지만,
과거 실장비 조사에는 아래 단서도 남아 있다.

- 특정 장비/RAW 조합에서는 Canon SDK thumbnail/save path가 실제로 약하거나 불안정할 수 있다.
- Windows shell thumbnail fallback도 이 머신에서는 신뢰 가능한 주경로로 증명되지 않았다.

즉 다음 구현은 `camera thumbnail을 조금 더 다듬으면 끝난다`고 가정하면 안 된다.
실장비 측정 결과 fast thumbnail 성공률이 낮거나 0에 가까우면
곧바로 `alternative same-capture source` 또는 제품 차원의 `booth-safe proxy` 분기를 검토해야 한다.

근거:

- 장비 조사 기록: [current-session-photo-troubleshooting-history.md](/C:/Code/Project/Boothy/history/current-session-photo-troubleshooting-history.md#L503)
- 대안 방향 기록: [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md#L268)
- 외부 제품/도구 해석 근거: [_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md#L172)

## 지금 당장은 주원인으로 보기 어려운 것

다음 항목은 현시점에서 1차 원인 후보로 보기 어렵다.

- 버튼 disabled/busy 회귀
- 최근 세션 레일 자체의 렌더링
- 이미지 loading priority 미설정
- helper event file polling 주기

이 항목들은 미세 조정 대상은 될 수 있어도
현재 사용자가 말하는 `엄청난 딜레이`를 설명하는 핵심 후보는 아니다.

## 근거가 되는 코드 경로 지도

### host

- 촬영 엔트리: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L88)
- fast preview emit: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L102)
- actual preview render 시작: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L139)

### helper

- RAW/thumbnail 다운로드 경계: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L664)
- fast thumbnail emit: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L706)
- thumbnail extraction 함수: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L843)

### host-side event bridge

- helper event wait loop: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L307)
- fast preview update handling: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L344)
- file-arrived completion handling: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L377)

### frontend

- fast preview state merge: [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx#L1539)
- rail merge into latest list: [CaptureScreen.tsx](/C:/Code/Project/Boothy/src/booth-shell/screens/CaptureScreen.tsx#L26)
- visible event logging: [SessionPreviewImage.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/SessionPreviewImage.tsx#L121)
- current recent-session view model: [current-session-previews.ts](/C:/Code/Project/Boothy/src/session-domain/selectors/current-session-previews.ts#L4)

## 현재 의견

### 의견 1

지금 단계에서 가장 효과가 큰 작업은
`actual preview를 더 싸게 만드는 것`보다
`camera thumbnail이 더 빨리, 더 자주 뜨게 만드는 것`이다.

### 의견 2

우리는 이미 `가짜 placeholder`가 아니라
`같은 촬영의 진짜 썸네일`만 보여 주는 방향을 택했다.
이 방향은 유지해야 한다.

즉 빠르게 만들더라도 아래는 금지다.

- 다른 촬영 사진 재사용
- representative tile을 같은 촬영처럼 보여 주기
- preview가 준비되지 않았는데 `Preview Ready`처럼 보이기

### 의견 3

다음 구현 에이전트는
속도 개선 전에 먼저 `실장비 계측`을 붙여야 한다.

이유:

- 지금 구조상 의심은 strong하지만
  실제로 가장 긴 구간이 `camera -> fast thumbnail`인지,
  `fast thumbnail -> UI visible`인지,
  `fast thumbnail miss -> actual render`인지
  장비 기준 수치가 아직 모자라다.

## 필요한 추가 계측

다음 8개 시점을 같은 requestId 기준으로 한 줄로 이어야 한다.

1. `button-pressed`
2. `capture-accepted`
3. `fast-thumbnail-attempted`
4. `fast-thumbnail-ready`
5. `capture-saved`
6. `recent-session-pending-visible`
7. `preview-render-ready`
8. `recent-session-visible`

현재 상태:

- 일부는 이미 있다.
- `fast-preview-ready`는 현재 provider까지 들어오고 있다.
- `recent-session-pending-visible`도 현재 찍히지만, 아직 `requestId`를 함께 싣지 않는다.
- `button-pressed`, helper의 `fast-thumbnail-attempted`, `fast-thumbnail-failed`는 아직 없다.

중요한 구현 메모:

- 지금 `CurrentSessionPreview`에는 `requestId`가 없다.
- 따라서 `recent-session-pending-visible`에 requestId join 정보를 넣으려면
  단순 로그 문자열 수정이 아니라 selector/state/component까지 requestId를 전달해야 한다.

## 구현 방향

### Track A. 먼저 계측을 닫는다

목표:

- 감이 아니라 수치로 병목을 잡는다.

해야 할 일:

- `button-pressed` 클라이언트 로그 추가
- helper에서 `fast-thumbnail-attempted`와 `fast-thumbnail-failed` 원인 로그 추가
- `recent-session-pending-visible`에 requestId join 정보 추가
- `CurrentSessionPreview` 또는 equivalent view-model에 requestId 전달 경로 추가
- 20회 이상 실장비 샘플 수집

### Track B. fast thumbnail first 경로를 더 강하게 만든다

목표:

- 같은 촬영의 첫 썸네일이 actual render보다 먼저 보이는 비율을 최대화한다.

해야 할 일:

- `TryDownloadPreviewThumbnail` 성공률 확인
- 실패 원인 분류
- fast thumbnail이 이미 있는 경우 host/UI가 절대 늦게 반영하지 않도록 유지
- 같은 canonical path 재사용 정책 유지

### Track C. fast path miss 시 fallback 비용을 줄인다

목표:

- fast thumbnail이 빠졌을 때도 체감 악화를 줄인다.

해야 할 일:

- 이미 제거된 host-side render kickoff 지연이 회귀하지 않도록 유지
- render input/output 경계 계측 보강
- 필요 시 booth rail에서는 camera thumbnail을 더 오래 유지하고,
  actual preview는 나중에 교체

## 구현 우선순위

### 1순위

`fast thumbnail이 실제 장비에서 언제 뜨는지`와
`왜 실패하는지`를 측정한다.

### 2순위

fast thumbnail이 뜬 경우
`host -> provider -> rail -> visible` 구간이 200ms 안팎으로 정리되는지 확인한다.

### 3순위

fast thumbnail miss가 확인되면
camera/SDK 설정 또는 alternative same-capture source를 검토한다.

### 4순위

그 뒤에 actual preview render 최적화를 다시 본다.

### 분기 조건

실장비 측정 결과가 아래처럼 나오면 바로 우선순위를 바꾼다.

- fast thumbnail 성공률이 충분히 높고 `4 -> 6`만 느리다: host/UI join 문제를 먼저 수정
- fast thumbnail 성공률이 낮거나 0에 가깝다: camera/SDK 조정만 고집하지 말고 same-capture 대체 source 검토
- fast thumbnail은 빠르지만 `6 -> 8`이 길다: actual render 최적화 또는 fast thumbnail 유지 시간을 더 길게 설계

## future agent를 위한 첫 구현 순서

1. `button-pressed` 로그 추가
2. helper에 `fast-thumbnail-attempted`, `fast-thumbnail-failed` 로그 추가
3. `CurrentSessionPreview`까지 requestId를 전달해 `recent-session-pending-visible`에 join 정보 추가
4. 장비 20회 측정
5. 결과를 아래 셋 중 하나로 분류

분류 기준:

- `1 -> 4`가 길다: camera/helper 병목
- `4 -> 6`이 길다: host/UI 병목
- `6 -> 8`이 길다: actual preview render 병목

## acceptance 기준

다음 구현은 아래 조건을 만족해야 한다.

- 같은 촬영의 첫 썸네일이 actual render를 기다리지 않고 먼저 보일 것
- `Preview Waiting` truth를 깨지 않을 것
- wrong-session, wrong-capture asset leak이 없을 것
- 버튼 안정성 회귀가 없을 것
- 연속 촬영에서 capture correlation 회귀가 없을 것

## 현재 검증 근거

`2026-04-02` 현재 워킹트리에서 다시 확인한 관련 검증:

- `pnpm vitest run src/booth-shell/components/SessionPreviewImage.test.tsx src/booth-shell/screens/CaptureScreen.test.tsx src/capture-adapter/services/capture-runtime.test.ts src/session-domain/state/session-provider.test.tsx`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness fast_preview`

이 테스트들이 말해 주는 것:

- helper fast preview update가 capture 저장 완료 전에 host로 전달될 수 있다
- fast thumbnail event를 프런트가 받을 수 있다
- pending fast preview가 레일에 먼저 들어간다
- invalid fast preview는 걸러진다
- canonical same-capture preview path 재사용이 유지된다
- 현재 워킹트리 기준 fast path 변경은 실행 가능한 상태다

## 마지막 결론

현재 booth의 속도 문제는
`썸네일을 그리는 화면이 느려서`라기보다
`같은 촬영의 첫 썸네일을 카메라/helper가 충분히 빠르고 안정적으로 주지 못할 때,
느린 actual preview 경로를 기다리게 되는 구조`에 가깝다.

따라서 다음 구현은
`actual preview 최적화`부터 시작하지 말고,
반드시 `fast thumbnail 도착 시간과 성공률`부터 계측하고 줄여야 한다.

다만 실장비에서 camera thumbnail 성공률이 낮게 나오면
그 다음 단계는 UI 미세조정이 아니라
`same-capture 대체 thumbnail source` 또는 `proxy/product split` 결정을 여는 쪽이어야 한다.
