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

## 2026-04-03 후속 회귀 메모: `Preview Waiting` 고착

최신 사용자 확인에서 더 우선인 회귀가 새로 드러났다.

- `Preview Waiting`으로는 들어가지만, 선택된 프리셋이 적용된 preview가 끝내 올라오지 않고 그 상태에 머무는 경우가 있었다.

이번 회차에서 정리한 제품 판단:

- 이 증상은 단순 `조금 느리다`가 아니라, first-visible applied result 자체가 닫히지 않는 correctness 회귀다.
- 따라서 이 구간에서는 공격적인 latency cut보다 `preset-applied preview가 반드시 완료되거나 실패로 명확히 정리되는가`가 우선이다.

이번에 가장 가능성 높게 본 원인:

- 직전 회차에서 darktable preview invocation 자체를 더 공격적으로 줄이기 위해 launch contract를 건드렸다.
- 하지만 이 변경은 실장비에서 `선택된 프리셋이 적용된 preview replacement`의 안정성을 해쳤을 가능성이 높다.
- 즉 preview render hot path는 아직 `조금 더 가볍게`보다 `known-good invocation 유지`가 더 중요하다는 쪽으로 판단을 되돌렸다.

이번 회차에서 바로 반영한 방향:

- darktable preview invocation에서 공격적으로 추가한 옵션/출력 처리 변경은 되돌리고, 검증돼 있던 render contract를 우선 복구한다.
- fast preview reuse와 requestId 계측 강화는 유지하되, preset-applied replacement를 막을 수 있는 invocation 실험은 후순위로 내린다.

이번 메모의 제품 의미:

- 속도 최적화는 `Preview Waiting -> preset-applied preview` 승격을 깨지 않는 범위에서만 유효하다.
- 다음 실장비 확인의 1차 기준은 `더 빨라졌는가`보다 `같은 촬영의 프리셋 적용 결과가 다시 정상 교체되는가`다.

## 2026-04-03 로그 재확인: 남은 주병목은 RAW handoff

최신 보관 세션 로그를 다시 확인했다.

이번 확인에서 분명했던 점:

- 보관된 `session.json` 기준 recent capture들은 `capture acknowledged -> RAW persisted`가 대체로 약 `1.6s ~ 2.1s`였다.
- 같은 세션에서 `RAW persisted -> preview visible`은 대체로 약 `123ms ~ 125ms` 수준이었다.
- 즉 현재 남은 체감 지연의 더 큰 덩어리는 preview replacement보다 `카메라 -> host RAW handoff` 쪽에 더 가깝다.

이번 로그가 주는 제품 의미:

- 지금 단계에서 렌더 파라미터만 조금 더 줄이는 일은 체감 전체를 크게 바꾸기 어렵다.
- 더 큰 개선은 `같은 촬영 preview 준비`를 RAW handoff와 더 많이 겹치게 만들 수 있느냐에서 나온다.

그래서 이번 회차에서 바로 반영한 조정:

- helper의 same-capture camera thumbnail 시도를 `RAW 다운로드 완료 뒤`가 아니라 `RAW 다운로드 시작 전`으로 앞당겼다.
- 목표는 카메라가 thumbnail을 줄 수 있는 장비/상태에서, host가 speculative preview work를 더 일찍 시작하게 만드는 것이다.
- 이 조정은 `capture completion truth`는 유지하면서도, `RAW handoff`와 `preset preview 준비`를 더 많이 겹치게 하려는 목적이다.

다음 실장비 확인 기준:

- same-capture thumbnail이 살아 있는 장비에서는 `capture accepted -> preset-applied first visible`이 지금보다 더 짧아져야 한다.
- 특히 성공 기준은 단순 `previewReady가 된다`가 아니라, `RAW handoff가 끝나자마자 교체될 준비가 이미 앞당겨져 있는가`다.

## 2026-04-03 후속 사용자 재검증 메모

최신 사용자 앱 실행 재검증 결과:

- recent-session 썸네일 노출은 이전보다 **조금 빨라진 것처럼 느껴졌다.**
- 다만 사용자 평가는 여전히 **제품 기준으로는 부족하다** 쪽이었다.

이번 메모의 제품 의미:

- 이번 변경은 무효가 아니며, first-visible recent-session 쪽 체감 개선은 일부 있었던 것으로 본다.
- 하지만 이 수준만으로는 "충분히 빨라졌다"는 판단까지는 도달하지 못했다.
- 따라서 다음 회차는 correctness 재검토보다 **추가 latency 단축 작업**으로 이어져야 한다.

현재 시점의 최신 사용자 판단:

- `조금 개선됨`
- `아직 부족함`

즉 이 문서의 후속 우선순위는 그대로 유지한다.
fast preview 경로의 즉시성은 더 밀어야 하고,
동시에 preset-applied preview 준비 시간과 cold-start 비용도 계속 줄여야 한다.

## 2026-04-03 로그 재확인 후 추가 단축 조치

보관된 실제 런타임 세션 로그를 다시 확인했다.

이번 재확인에서 중요하게 보인 점:

- `C:\Users\KimYS\AppData\Local\com.tauri.dev\booth-runtime\sessions\...` 아래 최근 실세션 흔적에서는 `fast-preview-ready`가 거의 남아 있지 않았다.
- 남아 있는 helper 이벤트는 대부분 `capture-accepted`, `file-arrived` 중심이었다.
- 즉 직전 회차에서 넣은 `겹쳐 돌리기` 최적화는 맞는 방향이었지만, 실장비에서 same-capture fast preview가 충분히 빨리 안 만들어지면 체감 개선이 크게 보이기 어렵다.

이번 판단의 제품 의미:

- 고객이 느끼는 병목은 이제 `겹쳐서 시작하느냐` 하나만의 문제가 아니다.
- 더 중요한 것은 `같은 촬영 썸네일을 얼마나 높은 확률로 즉시 보여 주느냐`다.
- 그래서 이번 회차는 fast preview hit-rate를 먼저 올리고, 그 다음 첫 노출용 preset preview 자체를 더 가볍게 만드는 쪽으로 추가 조정했다.

이번에 바로 반영한 조치:

- helper가 카메라 thumbnail을 즉시 못 주더라도, RAW 저장 직후 같은 촬영의 fallback fast preview 생성을 바로 시도하도록 앞당겼다.
- 따라서 이전처럼 나중 pending 구간까지 밀리지 않고, 실장비에서 fast preview first-visible 성공률을 더 끌어올릴 수 있게 됐다.
- fast preview raster를 소스로 쓰는 preset-applied preview는 첫 노출용 크기를 더 작게 낮춰, replacement 체감 시간을 추가로 줄이도록 조정했다.

이번 회차의 목표는
`조금 빨라짐`이 아니라
`고객이 바로 느끼는 수준까지 추가 단축`이다.

## 2026-04-03 사용자 후속 메모: 더 느려졌다는 피드백 반영

최신 사용자 피드백은 명확했다.

- 이전 회차 변경 뒤 recent-session 체감은 오히려 더 느려졌다.

이번 회차에서 정리한 가장 가능성 높은 원인:

- 직전 변경에서 helper가 `RAW 저장 직후 fallback fast preview 생성`까지 동기적으로 수행하도록 앞당겨졌다.
- 이 경로는 same-capture preview hit-rate를 올리려는 의도였지만, 반대로 `file-arrived` 자체를 늦춰 RAW handoff 완료 경계를 뒤로 밀 수 있었다.
- 즉 `썸네일을 더 빨리 만들겠다`는 최적화가 `캡처 완료를 늦추는 회귀`로 보였을 가능성이 높다.

그래서 이번에는 방향을 다시 조정했다.

- 카메라 thumbnail이 즉시 있으면 그 경우만 동기 경로에서 바로 사용한다.
- RAW 기반 fallback preview 생성은 다시 pending 후속 경로로 되돌려, `file-arrived`를 먼저 닫고 바로 다음 루프에서 이어서 처리하게 했다.
- 동시에 pending fallback이 실제로 시도됐는지도 더 분명히 남기도록 helper attempt 로그를 보강했다.

이번 판단의 제품 의미:

- first-visible same-capture preview는 중요하지만, 그보다 더 우선인 경계는 `캡처가 즉시 끝난 것처럼 느껴지는가`다.
- 따라서 앞으로의 단축 작업은 `preview를 앞당긴다`와 `capture completion을 막지 않는다`를 함께 만족해야 한다.

## 2026-04-03 로그 재확인: 3.n초 복귀 원인 정리

최신 재확인에서 사용자가 말한 `다시 3.n초대로 돌아갔다`는 감각과 가장 잘 맞는 코드 경계를 찾았다.

이번에 확인된 핵심:

- host preview 완료 경로에는 speculative preview 결과를 기다리는 상한 시간이 남아 있었고, 그 값이 `3.6초`였다.
- fast preview가 늦거나 speculative render가 제때 끝나지 않으면, host는 fallback으로 바로 넘어가지 않고 이 대기 예산을 먼저 소비할 수 있었다.
- 즉 same-capture fast path를 살리려던 보호 장치가, 실제로는 `3.x초 stall`처럼 느껴지는 회귀 원인이 되었을 가능성이 매우 높다.

그래서 이번 회차에서 바로 조정한 내용:

- helper fast preview 대기 예산을 `900ms -> 240ms`로 축소했다.
- speculative preview 채택 대기 예산을 `3.6s -> 320ms`로 축소했다.
- 두 경로 모두 예산이 소진되면 바로 fallback render로 넘어가도록 유지하고, 다음 진단을 위해 `wait budget exhausted` 로그도 남기게 했다.

이번 조정의 제품 의미:

- 이제 same-capture fast path는 `아주 빨리 오면 채택`, 아니면 `즉시 fallback` 쪽으로 훨씬 공격적으로 바뀌었다.
- 목표는 fast path miss 한 번이 전체 first-visible을 다시 3초대로 끌어올리지 못하게 만드는 것이다.

## 2026-04-03 연속 촬영 멈춤 메모

최신 사용자 피드백에서 새로 드러난 문제는 속도보다 더 우선이었다.

- 연속 촬영 시 앱이 멈춘 것처럼 보이는 현상이 있었다.

이번에 보관 로그와 구현을 다시 맞춰 보며 정리한 점:

- 최근 실세션 흔적에는 `두세 장 정상 -> 이후 capture-download-timeout -> phone-required` 패턴이 반복돼 있었다.
- 이 증상은 단순 `3초대 복귀`와 별개로, 연속 촬영 중 helper가 recovery 상태로 빠지며 고객에게 앱 멈춤처럼 보이는 문제다.
- 특히 helper에는 이전 촬영의 pending fast preview 작업을 이어서 처리하는 경로가 남아 있었고, 이 작업이 카메라 SDK 객체를 붙잡은 채 다음 촬영과 겹칠 여지가 있었다.

이번 회차에서 안정성 우선으로 조정한 내용:

- 새 촬영이 시작되면 이전 촬영의 pending fast preview 작업은 즉시 버리고, 새 셔터 경로를 우선하도록 바꿨다.
- pending fast preview 후속 경로에서는 더 이상 이전 촬영의 카메라 thumbnail SDK 다운로드를 붙잡지 않도록 정리했다.
- 즉 연속 촬영 안정성을 위해 `이전 촬영 fast preview 완성도`보다 `다음 촬영 handoff 안전성`을 더 우선하도록 방향을 바꿨다.

이번 판단의 제품 의미:

- 속도 최적화는 고객이 느끼는 연속 촬영 안정성을 깨지 않는 범위에서만 유효하다.
- 앞으로 same-capture fast path는 `빠르면 채택`, 하지만 `다음 촬영을 방해할 가능성이 있으면 즉시 포기`하는 것이 맞다.

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

## 2026-04-03 기술 리서치 기록

이번 회차에서 아래 리서치 문서를 새로 작성했다.

- [_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md)

이번 리서치의 핵심 전환점:

- 문제를 `썸네일이 빨리 보이느냐`보다 `선택된 프리셋이 적용된 같은 촬영 결과가 첫 신뢰 화면으로 얼마나 빨리 보이느냐`로 다시 정의했다.
- 즉 우선순위가 `fast thumbnail first` 단독 최적화에서 `preset-applied first-visible result` 전략으로 이동했다.
- 현재 앱 셸은 유지하되, 선택된 프리셋이 적용된 첫 결과만 전용 저지연 경로로 점진 치환하는 방향이 가장 현실적이라는 결론을 정리했다.
- 현재 기술로 부족하면 다음 단계 후보는 `local dedicated renderer`, `watch-folder 기반 외부 엔진 브리지`, `edge appliance` 순으로 검토하는 것이 맞다고 정리했다.

이번 리서치가 이 브리프에 주는 제품 의미:

- 기존 브리프의 `fast thumbnail 계측` 우선순위는 여전히 유효하지만, 그것만으로 제품 목표를 달성하는지는 별개다.
- 앞으로의 판단 기준은 `같은 촬영의 첫 이미지가 보이느냐`를 넘어서 `선택된 프리셋이 적용된 결과가 first-visible인가`여야 한다.
- 따라서 다음 구현은 단순 UI join 최적화만이 아니라, `preset apply latency`, `renderer warm-up`, `cold start 제거`, `same-capture preset preview 전용 경로`를 직접 다루는 방향으로 확장되어야 한다.

다음 실행 우선순위 갱신:

1. `button-pressed -> preset-preview-visible`를 공식 KPI로 정의한다.
2. requestId 기준으로 preset-applied preview 경로 전 구간 계측을 추가한다.
3. 현재 앱 셸을 유지한 채 `preset-applied first-visible sidecar/worker` 설계에 착수한다.
4. 제한된 환경에서 새 경로를 검증하고, 목표 미달이면 더 원초적인 렌더/플랫폼 전환 옵션을 연다.

## 2026-04-03 구현 후속 메모

이번 후속 구현에서 먼저 반영한 것:

- `button-pressed`, host `preview-render-ready`, client visible 로그가 같은 `requestId`로 더 쉽게 이어지도록 계측을 보강했다.
- client visible 로그에는 이제 `previewKind=pending-fast-preview | preset-applied-preview`가 함께 남는다.
- 이 변경으로 다음 실장비 검증에서는 `선택된 프리셋이 적용된 결과가 실제로 언제 first-visible이 되었는지`를 기존보다 훨씬 직접적으로 확인할 수 있다.

이번 구현의 제품 의미:

- 다음 회차 판단은 감각이 아니라 `preset-applied first-visible latency` 수치로 더 직접적으로 할 수 있다.
- 즉 이제는 "뭔가 빨라진 것 같다"보다 `어떤 requestId가 언제 눌렸고, 언제 preset-applied preview가 실제로 보였는가`를 기준으로 보게 된다.
## 2026-04-03 Follow-up: Fourth-Capture Stall Investigation

- Field verification showed the first three captures completed normally, but the fourth felt frozen for a long time.
- Log analysis isolated the stall to `capture accepted -> RAW file arrived`, not to preset render or thumbnail promotion.
- In the affected session, the fourth capture took about `24.7s` between host acknowledgement and RAW persistence, while preset preview render stayed normal at about `3.5s` once the file arrived.
- This means the current product issue was primarily a `camera handoff wait that looked like an app freeze`, not a filtered-preview render slowdown.
- Product-side mitigation was added so the capture surface now clearly switches into an in-flight state that tells the customer the app is still working and waiting for the camera's source file transfer.

## 2026-04-03 Additional Preview Latency Cut

- Latest re-test logs still showed preset preview render around `3.4s ~ 3.6s`.
- More importantly, the render log still showed `sourceAsset=raw-original`, which meant the faster same-capture JPEG path was not actually being used yet in the live render step.
- To close that gap, the preview render path was changed to:
  `1)` briefly wait for the helper-produced same-capture fast preview,
  `2)` reuse the canonical preview JPEG as the preset render input when available,
  `3)` lower preview render size again for first-visible speed.
- The next validation target is simple:
  latest `timing-events.log` should show `sourceAsset=fast-preview-raster` for preview renders, and elapsedMs should move materially closer to the sub-2-second goal.

## 2026-04-03 History Update: Fast Preview Reuse Optimization

- User feedback after the previous cut was: `definitely improved, but still needs to be reduced more`.
- The latest live session log confirmed the product still felt slow for the same reason:
  preview render elapsed stayed around `3.4s ~ 3.6s`, and the render path was still using `sourceAsset=raw-original`.
- Based on that log, the product direction for this round was not broad tuning but one very specific change:
  make the preset-applied preview reuse the same-capture fast JPEG path for the real preview render whenever possible.

What changed in product terms:

- The host now waits a short bounded window for the helper-produced same-capture fast preview before falling back to RAW-based preview render.
- If that same-capture JPEG is available in the canonical preview slot, the preset-applied preview render uses that raster as its input instead of the RAW original.
- The first-visible preview size cap was reduced again, from `960` to `800`, to push first applied-preview latency down further.

What this means operationally:

- The next hardware validation should no longer be judged only by feel.
- The decisive proof is in the latest session's `diagnostics/timing-events.log`.
- Success for this round means preview render lines begin showing `sourceAsset=fast-preview-raster`.
- If that value still remains `raw-original`, then the fast JPEG handoff is still not being consumed by the live render path and further work should stay focused on that bridge instead of unrelated UI tuning.

Current target remains unchanged:

- `preset-applied first visible` should reach **under 2 seconds**.
- Any result still materially above that should be treated as not yet acceptable.

## 2026-04-03 연속 컷 후속 수정: 프리셋 누락 방지와 2초대 재단축

- 최신 사용자 피드백은 두 가지였다.
- 여러 장 연속 촬영에서 최초 컷과 중간 컷 일부가 `프리셋이 적용되지 않은 것처럼` 보였다.
- 동시에 체감 속도도 여전히 `3초대처럼 느껴진다`는 평가였다.

이번에 다시 정리한 원인 판단:

- recent 보관 세션 `session_000000000018a138ef5c96c18c`와 `session_000000000018a13817723d39f4`의 `session.json` 기준 단일 컷은 `capture acknowledged -> preview visible`이 대략 `2080ms ~ 2180ms` 수준이었다.
- 그런데 실제 제품에서는 same-capture fast preview가 빨리 뜬 뒤 그 결과를 너무 일찍 `완료`로 취급해, RAW 기준의 더 정확한 프리셋 결과로 다시 닫히지 않는 경우가 섞일 수 있었다.
- 즉 이번 문제는 단순 `더 빠르게` 하나가 아니라, `빠른 첫 노출`과 `RAW 기준 최종 적용 보장`이 서로 분리되지 않았던 구조 문제에 더 가까웠다.

이번 회차에서 반영한 제품 방향:

- fast preview나 speculative preview는 계속 첫 노출용으로 사용한다.
- 하지만 그 결과를 최종 종료로 보지 않고, 뒤에서 RAW 기준 preview refinement를 반드시 한 번 더 진행하도록 바꿨다.
- 그래서 고객은 먼저 같은 촬영 결과를 빠르게 보고, 그 뒤 같은 자리에서 더 정확한 프리셋 적용 결과로 자연스럽게 교체받게 된다.
- 동시에 long-running preview render가 capture 파이프라인 전체를 오래 붙잡지 않도록 줄여, 연속 촬영에서 다음 컷이 덜 밀리게 했다.

이번 변경의 성공 기준:

- 더 이상 일부 컷이 `프리셋이 안 먹은 채로 끝난 것처럼` 남지 않아야 한다.
- 최신 실장비 로그에서는 같은 request에 대해 `fast-preview-raster` first-visible 뒤에 `raw-original` refinement가 이어지는 흐름이 보여야 한다.
- 그래도 체감이 다시 `3초대`로 남으면, 다음 판단은 미세 튜닝보다 `상주형 renderer worker` 쪽으로 넘어가는 것이 맞다.

## 2026-04-03 사용자 재검증 반영: 조금 줄었지만 아직 미달

- 최신 사용자 피드백은 `조금 줄어든 것은 맞지만, 아직 허용 가능한 속도는 아니다`였다.

이번에 로컬에서 다시 확인한 로그 사실:

- 현재 남아 있는 최신 보관 capture 세션은 여전히 `2026-03-29` 런타임 세션들이었다.
- 그 보관본 기준 latest single capture는 `capture acknowledged -> preview visible = 2079ms`였다.
- 최근 5컷 기준 평균도 약 `2031.8ms`로, 이미 `2초 바로 위`까지는 내려와 있었다.
- 반면 `2026-04-03`의 [Boothy.log]에는 실제 capture 완료 라인보다 readiness polling 라인이 주로 남아 있었고, 이번 사용자 재검증을 그대로 재구성할 새 capture trace는 충분하지 않았다.

이번 로그가 주는 제품 판단:

- 남은 차이는 이제 수초 단위의 거대한 병목보다는, `fast path miss 시 추가 대기`와 `refinement 완료 사실이 UI에 전달되는 시점` 쪽에 더 가깝다.
- 즉 실제 렌더 시간이 조금 줄어도, 화면이 그 사실을 polling 주기 뒤에 알면 고객은 여전히 느리게 느낄 수 있다.

그래서 이번 회차에서 추가로 반영한 조정:

- helper fast preview 대기 예산을 더 줄여 `240ms -> 120ms`로 낮췄다.
- speculative preview 채택 대기 예산도 `320ms -> 160ms`로 더 공격적으로 줄였다.
- RAW refinement가 뒤에서 끝나면 다음 polling까지 기다리지 않고, host가 readiness update를 한 번 더 즉시 보내도록 바꿨다.
- client readiness polling도 `300ms -> 180ms`로 낮춰, 이벤트 누락 시에도 화면 반영 지연 상한을 줄였다.

이번 회차의 제품 의미:

- 이번 조정은 화질 방향을 더 희생하지 않고, 남아 있던 `수십~수백 ms` 급 대기를 먼저 걷어내는 성격이다.
- 따라서 기대 효과는 `조금 줄었다`를 `2초 아래 체감` 쪽으로 더 당기는 것이다.
- 그래도 다음 실장비에서 여전히 `3초대 체감`이 반복되면, 그때는 더 이상 wait-budget/polling 단계가 아니라 renderer topology 자체를 바꾸는 쪽이 맞다.

## 2026-04-03 로그 재검토: readiness polling 과다

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log) 재검토에서 새 세션 `session_000000000018a2c2fc6ddc04f0` 흔적은 확인됐다.
- 하지만 이 로그에는 실제 `capture_request_saved`, `capture_preview_ready`, `capture_preview_refined`보다 `capture_readiness`와 `gateway-get-readiness`가 매우 촘촘하게 반복되고 있었다.
- 즉 이번 현장 체감 저하를 설명할 직접 capture trace가 부족했고, 동시에 idle 상태에서도 readiness polling이 과도하게 host를 두드리고 있었다.

이번 판단의 제품 의미:

- 이 상태는 진단 품질을 떨어뜨린다.
- 동시에 capture가 없는 대기 구간에도 host와 client가 계속 readiness refresh를 반복해, 실제 capture 직전/직후 경로에 불필요한 잡음을 만든다.

그래서 이번 회차에서 바로 조정한 것:

- readiness polling을 항상 빠르게 유지하지 않고, 세션이 `idle + ready` 상태일 때는 더 느리게 backoff 하도록 바꿨다.
- 대신 capture 대기, preview waiting, refinement waiting 같은 active 구간에서는 기존의 빠른 폴링을 유지한다.
- 즉 고객 체감에 필요한 시점만 민감하게 보고, 평상시에는 host를 덜 흔들도록 바꾼 것이다.

이번 조정의 기대 효과:

- idle 구간의 불필요한 host 호출과 로그 소음을 줄인다.
- 실제 capture/preview 로그가 더 잘 남아 다음 실장비 진단 품질이 좋아진다.
- capture 직전의 background churn을 줄여, 작은 체감 지연도 덜어낼 수 있다.

## 2026-04-03 사용자 판단 유지: 아직 만족 못함

- 최신 사용자 판단은 여전히 동일하다.
- `조금 나아진 요소는 있어도, 아직 고객이 만족할 속도는 아니다.`

이번 판단에서 추가로 정리한 리스크:

- 우리가 correctness를 위해 붙여 둔 RAW refinement가, 연속 촬영에서는 다음 컷의 first-visible 경로와 같은 자원을 경쟁할 수 있다.
- 즉 이전 컷의 `더 정확한 결과 만들기`가 다음 컷의 `더 빨리 보여 주기`를 갉아먹을 수 있다.

그래서 이번에 더 바꾼 방향:

- RAW refinement는 이제 capture round-trip이 완전히 idle일 때까지 더 낮은 우선순위로 미룬다.
- refinement가 render queue 때문에 즉시 못 돌면 바로 포기하지 않고, 짧게 재시도하되 현재 first-visible 경로를 먼저 비켜 준다.

이번 회차의 제품 의미:

- 연속 촬영에서 중요한 건 `이전 컷의 더 예쁜 정교화`보다 `지금 방금 찍은 컷의 first-visible`이다.
- 따라서 다음 실장비 기준도 `정교화가 언젠가 된다`보다 `다음 컷 최신 사진이 즉시 보이느냐`를 더 우선으로 봐야 한다.

## 2026-04-03 로그 재확인: fast preview가 있어도 first-visible이 RAW 저장 뒤로 밀렸다

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log) 재확인에서 세션 `session_000000000018a2c41bff76fdb4`의 최근 두 컷은 여전히 매우 느렸다.
- 실제 로그에는 `helper_fast_preview_wait_budget_exhausted wait_ms=120` 뒤에 곧바로 `render_job_started ... sourceAsset=raw-original`가 찍혔다.
- 같은 컷의 `capture_preview_ready`는 각각 `elapsed_ms=6440`, `elapsed_ms=6353`으로 남아 있었다.
- 즉 고객 체감상 `Preview Waiting`에 오래 머무는 이유는 아직도 `raw-original` 경로가 first-visible을 대표해 버리는 데 있었다.

이번에 새로 정리된 구조적 원인:

- helper가 fast preview를 먼저 알려 줘도, host가 그 사실을 `persist_capture_in_dir` 이후에만 recent-session update로 올리면 고객은 그동안 계속 기다리게 된다.
- 다시 말해 `fast preview 준비`와 `화면 first-visible`이 같은 이벤트로 이어지지 못하고 있었다.

그래서 이번 회차에서 추가로 바꾼 것:

- helper fast preview를 받는 즉시 canonical preview 경로로 승격할 수 있으면, RAW 저장 완료를 기다리지 않고 recent-session에 바로 노출되도록 host 경로를 앞당겼다.
- 이후 `persist_capture_in_dir`가 같은 canonical asset을 다시 돌려줘도 중복 emit은 하지 않도록 막았다.
- 제품 기준으로는 `정답 렌더를 더 빨리 만드는 것`보다 먼저 `같은 컷의 첫 표시를 가능한 한 즉시 띄우는 것`을 우선시한 조정이다.

이번 조정의 기대 효과:

- helper thumbnail이 살아 있는 컷에서는 `Preview Waiting` 체류 시간이 크게 줄어야 한다.
- 특히 로그상 `sourceAsset=raw-original` preview render가 first-visible을 독점하던 경우를 줄일 수 있다.
- 다음 실장비에서도 helper fast preview가 자주 비거나 여전히 `2초 이하`를 못 맞추면, 그때는 구조 조정 후보를 `상주형 renderer worker`뿐 아니라 `camera thumbnail first renderer` 쪽까지 넓혀야 한다.

## 2026-04-03 사용자 판단 유지: 여전히 허용 불가, tech 문서 재검토

- 최신 사용자 판단은 더 분명해졌다.
- `조금 줄었다` 수준으로는 부족하고, 현재 속도는 여전히 제품 기준에서 허용할 수 없다.

이번 회차에서 히스토리 문서가 가리키는 tech 문서를 다시 따라가며 정리한 사실:

- [technical-capture-preview-latency-research-2026-04-01.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md#L246)는 엔진 전면 교체 전에 남아 있는 darktable 경로 최적화로 `OpenCL/GPU 검증`, `--apply-custom-presets false` A/B, `render warm-up`을 명시했다.
- [technical-filtered-thumbnail-latency-research-2026-04-03.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md#L373)는 지금 제품의 1차 정답을 `앱 셸 유지 + first-visible 전용 저지연 sidecar/worker`로 정리했고, 그 다음 단계로 `local dedicated renderer`, `watch-folder bridge`, `edge appliance`를 제시했다.
- [darktable-reference-README.md](/C:/Code/Project/Boothy/darktable-reference-README.md#L95)는 runtime truth는 유지하되, `darktable-cli`를 camera helper와 별개인 render worker로 취급하고 preview/final profile을 분리하라고 못 박고 있다.

이번 재검토에서 현재 워킹트리와 실제 장비 상태를 대조해 본 결과:

- 현재 preview render는 여전히 컷마다 새 `darktable-cli` 프로세스를 직접 띄우는 구조다.
- 현재 invocation에는 tech 문서가 언급한 `--apply-custom-presets false`가 아직 들어가 있지 않다.
- `render warm-up`, `preset preload`, `cache priming`을 상시 유지하는 상주형 경로도 아직 없다.
- `OpenCL/GPU`는 booth 실장비에서 아직 검증이 닫히지 않았다.
- 실제 장비의 `darktable-cli --version`은 `5.4.0`이었고, 참조 문서 pin은 `5.4.1`이다.
- `darktable-cltest`는 이번 점검에서도 제한 시간 안에 끝나지 않아, GPU/OpenCL 가용성 자체가 아직 운영 기준으로 확정되지 않았다.

이번 tech 문서 재검토가 주는 제품 판단:

- `그냥 렌더 엔진을 지금 당장 바꿔야만 한다`까지는 아직 아니다.
- 하지만 `현 구조에서 wait budget`, `polling`, `preview 크기` 같은 미세 조정으로 더 버티는 단계는 거의 끝났다.
- 이제 남아 있는 의미 있는 시도는 `같은 엔진을 더 잘 호출하는 구조 변경` 또는 `first-visible 전용 경로 추가`다.

엔진 교체 전 아직 남아 있는 시도 방법:

1. `darktable 유지 + 상주형 first-visible renderer worker`
   - 컷마다 새 프로세스를 띄우지 말고, 세션 시작 또는 preset 선택 시 warm 상태를 유지하는 쪽이다.
   - 이건 엔진 교체가 아니라 호출 topology 교체에 가깝다.
2. `OpenCL/GPU 실장비 검증 + on/off 비교`
   - 현재는 문서상 후보일 뿐, booth에서 실제로 이득이 나는지 확인이 안 끝났다.
3. `--apply-custom-presets false` A/B
   - custom preset 로딩 비용이 preview first-visible에 실제로 영향을 주는지 darktable 경로에서 아직 닫히지 않았다.
4. `preset preload / cache priming`
   - 세션 시작 또는 preset 변경 시 preview lane을 미리 덥혀서 cold start를 고객 밖으로 밀어내는 방식이다.
5. `same-capture intermediate preview 강화`
   - [technical-filtered-thumbnail-latency-research-2026-04-03.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md#L158)가 말한 것처럼, LibRaw 계열 intermediate preview를 first-visible source 후보로 검토할 여지가 있다.

이번 시점의 추천 판단:

- `엔진 교체`보다 먼저 `상주형 first-visible renderer worker`를 여는 것이 맞다.
- 이건 현재 darktable truth와 preset fidelity를 유지하면서도, cold start와 per-capture spawn 비용을 직접 겨냥하는 가장 현실적인 다음 단계다.
- 그 worker 경로에서도 목표를 못 맞추면, 그때는 `local dedicated renderer` 또는 `watch-folder 기반 외부 엔진 브리지`로 넘어가는 판단이 맞다.

즉 이번 브리프의 최신 결론은 이렇다:

- `미세 튜닝`은 거의 소진됐다.
- `엔진 교체`가 유일한 남은 선택지는 아니다.
- 하지만 다음 단계는 분명히 `구조 변경`이어야 하며, 그 첫 후보는 `same engine, different topology`다.

## 2026-04-03 남은 시도 즉시 반영: preview custom preset 로딩 축소 + preset 선택 직후 warm-up

- 사용자 요청에 따라, tech 문서에서 아직 미시도 상태로 남아 있던 항목 중 지금 코드에 바로 넣을 수 있는 것부터 반영했다.

이번에 실제로 반영한 것:

- preview용 `darktable-cli` invocation에 `--apply-custom-presets false`를 추가했다.
- 즉, first-visible preview lane에서는 `data.db` custom preset 로딩 비용을 더 줄이도록 했다.
- 동시에 preset 선택 직후 `preview renderer warm-up`을 백그라운드로 예약하도록 바꿨다.
- 이 warm-up은 현재 세션/프리셋 기준으로 한 번만 잡히며, render queue가 비어 있을 때만 조용히 실행되도록 했다.
- 목적은 첫 캡처 시점의 cold start 일부를 고객 앞이 아니라 preset 선택 직후로 미리 밀어내는 것이다.

이번 조정의 제품 의미:

- 이 단계는 `엔진 교체`가 아니라 `같은 darktable 엔진을 더 낮은 지연 구조로 쓰기`에 가깝다.
- 즉 지금은 `same engine, different topology`의 첫 실장 적용이다.
- 이 조정까지 들어간 뒤에도 실장비 체감이 여전히 허용 불가라면, 다음 판단은 더 이상 invocation 미세 조정보다 `상주형 first-visible renderer worker` 또는 `local dedicated renderer` 쪽이 된다.

이번 회차 검증:

- preview invocation 단위 테스트에서 새 플래그가 들어간 것을 확인했다.
- warm-up source 준비 테스트를 추가해 background warm-up 준비 경로가 깨지지 않음을 확인했다.
- 기존 fast preview canonical path 조기 노출 테스트도 계속 통과했다.

## 2026-04-03 사용자 판단 유지: warm-up 이후에도 여전히 허용 불가

- 최신 사용자 판단은 바뀌지 않았다.
- warm-up과 preview custom preset 로딩 축소를 넣은 뒤에도, 체감은 여전히 만족할 수준이 아니다.

이번 로그 재확인에서 확인된 사실:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에는 세션 `session_000000000018a2c66b4848bd74`의 `capture_preview_ready elapsed_ms=7241`가 남아 있었다.
- 같은 컷의 client visible은 그 직후였고, 즉 이번 병목은 UI 반영이 아니라 first-visible preview 생성 자체에 남아 있었다.
- 다시 말해 이번 시점의 주병목은 `warm-up이 없어서`라기보다, `첫 표시용 preview 프로파일이 아직 too expensive`인 쪽에 더 가깝다.

그래서 엔진 변경 전 마지막 비엔진 시도 방향:

- recent-session rail의 first-visible preview는 더 공격적으로 작은 전용 프로파일로 낮춘다.
- 이 단계는 최종 화질을 위한 것이 아니라, `현재 세션 사진` 레일에 같은 촬영 결과를 가능한 한 빨리 보이게 하기 위한 전용 절충이다.
- 사용자가 이 수준도 만족하지 못하면, 다음 판단은 더 이상 preview 크기/옵션 미세 조정이 아니라 구조 전환 쪽으로 가야 한다.

## 2026-04-03 사용자 판단 유지: 체감상 여전히 동일, 실행 방식 정리 시도

- 최신 사용자 피드백은 `체감상 여전히 똑같다`였다.

이번 로그 재확인에서 분명했던 점:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c6d30012ff84`의 `capture_preview_ready elapsed_ms=7042`가 남아 있었다.
- 같은 컷의 `recent-session-visible`은 바로 뒤였고 `uiLagMs=19`였다.
- 즉 이번에도 병목은 UI 반영이 아니라 preview render 실행 자체였다.

그래서 이번 회차에서 추가로 손댄 것:

- `darktable-cli` 실행 시 stdout/stderr를 파이프로 오래 붙잡지 않고, stderr는 파일로 흘리도록 바꿨다.
- 목적은 CLI 로그 출력이 파이프 버퍼를 채워 render 자체를 막는 가능성을 없애는 것이다.
- 이건 렌더 엔진 변경이 아니라 실행 방식 안정화에 가깝고, preview/final 품질 진실값도 바꾸지 않는다.

이번 조정의 제품 의미:

- 이번 시도는 `실제로 렌더가 느린 것`과 `실행 방식 때문에 더 느려지는 것`을 분리하기 위한 성격도 있다.
- 이 조정 뒤에도 실장비 체감이 여전히 그대로면, 다음 판단은 더 이상 invocation/출력 처리 미세 조정이 아니라 구조 전환 쪽임이 더 분명해진다.

## 2026-04-03 사용자 판단 유지: 여전히 느림, preview lane에서 OpenCL 초기화 제거 시도

- 최신 사용자 피드백은 그대로였다.
- `조금 나아진 듯해도 허용 가능한 속도는 아니다`, 그리고 체감상으로는 여전히 느리다는 판단이 유지됐다.

이번 로그 재확인에서 새로 확인한 사실:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c7a958978450`의 두 컷이 각각 `capture_preview_ready elapsed_ms=6941`, `6844`로 남아 있었다.
- 두 컷 모두 `recent-session-visible`은 바로 뒤였고 `uiLagMs`는 `21`, `20` 수준이었다.
- 즉 이번에도 제품 체감 병목은 UI 반영이 아니라 `preset-applied-preview 생성` 그 자체였다.
- 같은 로그에서 두 번째 컷은 `helper_fast_preview_wait_budget_exhausted wait_ms=120` 뒤에 `pending-fast-preview`로 들어갔고, 결국 first-visible이 `6.8s`대까지 밀렸다.
- 이건 `fast preview availability`보다도, 실제 preview render lane이 여전히 너무 무겁다는 쪽을 다시 확인해 준다.

그래서 이번 회차에서 추가로 시도한 것:

- preview 전용 `darktable-cli` invocation에 `--disable-opencl`를 추가했다.
- final render truth는 그대로 두고, first-visible preview lane에서만 OpenCL 초기화/디바이스 준비 비용을 제거하는 실험이다.
- 목적은 booth 장비에서 작은 preview 한 장을 뽑기 위해 GPU/OpenCL 런타임을 깨우는 비용이, 실제 이득보다 더 큰 상황을 먼저 배제하는 것이다.
- 즉 이 단계는 `엔진 변경`이 아니라 `preview lane을 더 단순한 CPU 경로로 고정`하는 비엔진 최적화다.

이번 판단의 의미:

- 이제 남은 비엔진 시도는 정말 얇아졌다.
- `preview 크기 축소`, `custom preset 로딩 축소`, `warm-up`, `실행 방식 정리`, `OpenCL 초기화 제거`까지 넣고도 체감이 크게 안 줄면, 그다음은 옵션 미세 조정보다 구조 변경이 맞다.
- 다만 사용자와 합의한 대로, 실제 `엔진/renderer topology` 변경 전에는 먼저 브리핑하고 결정한다.

## 2026-04-03 사용자 판단 유지: 여전히 느림, helper fast preview 적중률 보강

- 최신 사용자 판단은 동일했다.
- `여전히 느리다`, 그리고 이번엔 아예 다음 단계로 계속 진행해 달라는 요청이 있었다.

이번 로그 재확인에서 확인된 사실:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c88ee9325658`의 두 컷이 각각 `capture_preview_ready elapsed_ms=7635`, `7158`로 남아 있었다.
- 첫 컷의 `recent-session-visible`은 그 직후였고 `uiLagMs=187`이었다.
- 둘째 컷은 `helper_fast_preview_wait_budget_exhausted wait_ms=120` 이후 `pending-fast-preview`로 들어갔지만, 결국 `preset-applied-preview`가 `7.1s`대에 닫혔다.
- 즉 이번에도 UI보다 `first-visible source selection`과 `preview render lane`이 더 큰 문제였고, 특히 helper fast preview를 너무 빨리 포기하고 있다는 신호가 분명했다.

그래서 이번 회차에서 추가로 시도한 것:

- host의 helper fast preview 대기 예산을 `120ms -> 360ms`로 늘렸다.
- 목적은 booth 장비에서 helper same-capture preview가 `조금 늦게` 도착해도, 곧바로 `7초대 darktable 경로`로 내려가지 않도록 하는 것이다.
- 동시에 helper 내부에서 pending fast preview를 단일 슬롯으로 들고 있다가 다음 촬영이 시작되면 사실상 버리던 구조를 큐 형태로 바꿨다.
- 즉 immediate camera thumbnail이 실패해도, 이전 컷의 RAW 기반 fallback preview 후보를 다음 컷 때문에 폐기하지 않고 순서대로 계속 살려 두도록 했다.

이번 조정의 제품 의미:

- 이 단계도 여전히 `엔진 변경`은 아니다.
- 다만 `same-capture fast preview가 실제 first-visible에 당첨될 기회`를 늘리는 쪽의 마지막 비엔진 조정에 가깝다.
- 이 단계 후에도 체감이 그대로면, 남은 선택지는 invocation 미세 조정보다 구조 변경 쪽이 더 설득력 있어진다.

## 2026-04-03 사용자 피드백 반영: 조금 더 느려짐, 대기 예산 회귀 정리 + pending preview 즉시 재촬영 허용

- 최신 사용자 피드백은 `조금 더 느려졌다`였다.
- 실제 로그도 그 판단을 뒷받침했다.

이번 로그 재확인에서 중요했던 점:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c90eeaced59c`의 컷이 `capture_preview_ready elapsed_ms=7156`로 남아 있었다.
- 그런데 같은 로그에서 그보다 앞서 `recent-session-pending-visible`이 먼저 찍혔다.
- 다시 말해 이번 시점엔 `same-capture pending fast preview`가 이미 고객 화면에 보였는데도, 앱 전체 상태는 끝까지 `Preview Waiting / can_capture=false`에 머물렀다.
- 즉 방금 늘린 helper 대기 예산은 체감 개선보다 오히려 blocking 시간을 늘린 회귀 성격이 더 강했다.

그래서 이번 회차에서 바로 수정한 것:

- host의 helper fast preview 대기 예산은 다시 `120ms`로 되돌렸다.
- 대신 `pending fast preview`가 실제로 보이는 순간에는, live camera gate가 건강한 조건에서 다음 촬영을 다시 허용하도록 readiness 규칙을 바꿨다.
- 핵심은 `최종 preset-applied preview가 아직 안 끝났더라도, same-capture first-visible이 이미 확보됐으면 booth 흐름은 다시 앞으로 가게 한다`는 제품 정책 전환이다.
- 프런트 쪽 surface state도 이에 맞춰 `previewWaiting`에 고정하지 않고 `captureReady`로 승격될 수 있게 맞췄다.

이번 조정의 제품 의미:

- 이 단계도 렌더 엔진 변경은 아니다.
- 하지만 지금까지의 최적화가 `render ms 절감` 위주였다면, 이번 단계는 `고객이 언제 다음 행동을 할 수 있느냐`를 직접 줄이는 제품 흐름 조정이다.
- 이 조정 후에도 여전히 체감이 만족스럽지 않다면, 남은 다음 단계는 더 분명하게 `상주형 first-visible worker` 같은 구조 변경 쪽이다.

## 2026-04-03 최신 로그 재확인: 여전히 제품 기준 미달, 이제 속도와 안정성을 함께 봐야 한다

- 최신 사용자 판단대로, 현재 recent-session 썸네일 체감은 아직 허용 가능한 속도가 아니다.

이번 회차에서 다시 확인한 최신 근거는 두 갈래였다.

1. 오늘 최신 앱 로그

- [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log) 최신 기록에는 `2026-04-03 17:00:33 KST` 기준 세션 `session_000000000018a2c9e8a9cef54c`의 `capture_preview_ready elapsed_ms=6949`가 남아 있었다.
- 같은 컷의 `recent-session-visible`은 바로 뒤였고 `uiLagMs=23`이었다.
- 즉 지금도 고객 체감 병목은 UI 반영이 아니라 `first-visible preview 생성` 자체에 있다.

2. 보관된 실세션 런타임 흔적

- `2026-03-29` 기준 최근 8개 보관 세션을 다시 확인한 결과, `capture-ready`로 끝난 세션은 3개뿐이었고 `phone-required`로 끝난 세션이 5개였다.
- 성공으로 닫힌 13개 컷은 `request -> RAW handoff`가 평균 약 `2306ms`였고 범위는 `1966ms ~ 2769ms`였다.
- 같은 성공 컷에서 `RAW persisted -> preview ready`는 거의 고정적으로 `123ms ~ 125ms`였다.
- 반면 실패 5건은 `capture-download-timeout`까지 평균 약 `8884ms`였고 범위는 `6268ms ~ 16723ms`였다.

이번 로그가 주는 제품 판단:

- 성공 경로만 봐도 현재 속도는 여전히 빠르지 않다.
- 더 큰 문제는 실패 경로가 아직 자주 열리고, 이때는 체감이 단순 `느림`이 아니라 `앱이 멈추거나 복구 상태로 간다` 쪽으로 악화된다는 점이다.
- 따라서 지금 단계의 핵심 문제는 하나가 아니다.
- `good path`에서는 여전히 first-visible까지가 느리고,
- `bad path`에서는 RAW handoff timeout이 recent-session 경험 전체를 무너뜨린다.

이번 회차에서 함께 드러난 계측 공백:

- 최신 `Boothy.log`에는 `6.9s`대 preview ready가 남아 있는데, 현재 보관 세션 폴더 쪽에는 같은 최신 세션의 request-level artifact가 충분히 남아 있지 않았다.
- 최근 보관 세션들에서도 `timing-events.log` 증거는 사실상 비어 있었고, 실측은 `session.json`과 `camera-helper-requests/events` 조합에 의존했다.
- 즉 지금은 `정확히 어디서 6.9초가 소비됐는가`를 오늘 경로 기준으로 한 번 더 닫아 줄 최신 seam 계측이 필요하다.

그래서 다음 단계는 이렇게 정리한다.

- 첫째, `requestId` 기준 최신 실장비 계측을 복구한다.
- 최소한 `request-capture`, `file-arrived`, `fast-preview-visible`, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`가 같은 세션 폴더에 함께 남아야 한다.
- 둘째, 그 한 번의 실장비 재실행으로 이번 병목이 `RAW handoff` 중심인지, `preview lane` 중심인지 다시 단일 컷 단위로 닫는다.
- 셋째, 만약 최신 seam에서도 `capture_preview_ready`가 여전히 `5s+`면, 다음 판단은 더 이상 invocation 미세 조정이 아니라 `상주형 first-visible renderer` 또는 별도 local fast renderer 같은 구조 변경으로 넘어간다.
- 반대로 `RAW handoff` timeout이 먼저 재현되면, 다음 우선순위는 preview 미세 최적화보다 helper/SDK completion boundary 안정화다.

이번 메모의 결론:

- 현재 문제는 `조금 더 줄이면 될 수준`로 보기 어렵다.
- 제품 관점의 다음 단계는 `최신 seam 재계측 1회 -> 구조 변경 여부 결정`이다.
