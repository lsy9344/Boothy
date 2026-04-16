# 카메라 촬영 기능 검증 히스토리

## 목적

이 문서는 [camera-helper-troubleshooting-history.md](/C:/Code/Project/Boothy/history/camera-helper-troubleshooting-history.md)에서
`카메라 촬영 기능`과 `사진찍기` 요청에 직접 연결된 이력만 따로 추린 문서다.

다음 에이전트가 아래 상황을 빠르게 이어받을 수 있게 하는 것이 목적이다.

- `사진찍기` 버튼을 눌렀을 때 오래 로딩되거나 실패하는 경우
- 첫 촬영은 되지만 같은 세션의 다음 촬영에서 회귀가 나는 경우
- helper, host, frontend 중 어디에서 촬영 round-trip이 끊겼는지 빠르게 분리해야 하는 경우

카메라 연결 상태를 앱에 반영하는 일반 이력은 원문 문서를 기준으로 보고,
이 문서는 `촬영 요청 -> 요청 수락 -> 파일 도착 -> 고객 화면 반영` 경계에만 집중한다.

## 이 문서가 다루는 범위

포함:

- `request_capture` 호출 이후의 문제
- `사진찍기` 직후 `Phone Required`로 떨어지는 문제
- 첫 촬영 성공 후 후속 촬영 회귀
- helper binary, request log, event log, requestId correlation 문제

제외:

- helper status BOM, freshness, offset parser, Tauri runtime detection 같은 일반 상태 반영 이력
- 고객 화면의 연결 상태 문구 조정 자체

단, 마지막의 `앱 반영 참고` 섹션에는 촬영 기능 검증과 직접 맞닿는 최소한의 상태 반영 원칙만 남긴다.

## 촬영 성공 판정 기준

촬영 기능은 아래 네 가지가 같은 세션 안에서 닫혀야 성공으로 본다.

1. host normalized readiness가 `Ready`이고 `canCapture=true`다.
2. 같은 `requestId`에 대해 helper가 `capture-accepted`를 남긴다.
3. 같은 `requestId`에 대해 helper가 `file-arrived`를 남긴다.
4. 실제 RAW 파일이 해당 세션의 `captures/originals/` 아래에 존재하고, 최근 결과가 화면에 반영된다.

중요한 점:

- helper raw 상태가 `ready`여도 위 네 경계가 닫히지 않으면 촬영 성공이 아니다.
- 반대로 일시적인 재확인 상태라고 해서 곧바로 `Phone Required`로 올리면 안 된다.

## 최종 상태

- `2026-03-29` 최종 하드웨어 검증에서 카메라 상태가 정상 유지되는 것을 확인했다.
- 같은 세션에서 `사진 찍기` 버튼을 연속으로 눌러도 7장까지 모두 정상 촬영됐다.
- 각 촬영 결과는 최근 썸네일/결과 화면에 이어서 반영됐고, 재현되던 `Phone Required`, 무한 로딩, 세션 초기화 회귀는 더 이상 나타나지 않았다.

## 결국 어떻게 해결했는가

- helper binary/runtime attach를 먼저 복구해 촬영 경계 자체를 다시 열었다.
- 프런트가 일시적인 재확인 상태를 `Phone Required`로 과번역하지 않게 조정했다.
- 기존 세션 재사용 시 stale request replay를 막아 새 촬영 `requestId` correlation이 어긋나지 않게 했다.
- 촬영 직후 same-session `session-not-found`가 들어와도 현재 세션과 촬영 화면을 유지하게 보정했다.
- host/helper capture timeout budget을 현실값으로 늘려 느린 RAW handoff를 너무 빨리 실패 처리하지 않게 맞췄다.
- 촬영 중 keep-alive 간섭을 막고 RAW transfer 처리를 더 직렬화해 follow-up capture transfer를 안정화했다.

짧은 결론:

- 이번 성공은 한 군데 수정이 아니라, `실행 경계 복구 -> 실패 해석 보정 -> requestId 정합성 복구 -> 세션 보존 -> transfer 안정화`를 순서대로 정리한 결과다.
- 최종적으로 고객 기준 문제였던 `카메라 상태`, `사진 찍기` 버튼 연속 촬영, 최근 썸네일 반영이 모두 정상 흐름으로 돌아왔다.

## 현재까지 확인된 핵심 촬영 이슈

### 1. 2026-03-29: `helper-binary-missing`으로 촬영 경계 자체가 열리지 않던 문제

증상:

- 최신 session의 `camera-helper-status.json`에 `helper-binary-missing`이 기록됐다.
- 이 시점에는 카메라 발견 이전에 helper 런타임 자체가 붙지 못해 촬영 경로가 시작되지 않았다.

실제 원인:

- workspace 안에 `canon-helper.exe`가 없었다.
- helper project가 repo 내부 vendor SDK만 고정 참조하고 있어 fresh workspace에서는 build와 runtime attach가 함께 막힐 수 있었다.

조치:

- helper supervisor가 publish/debug exe를 못 찾아도 dev 환경에서는 helper source project를 `dotnet run`으로 띄울 수 있게 fallback을 추가했다.
- helper project가 `BOOTHY_CANON_SDK_ROOT` 또는 local vendor를 SDK root로 읽을 수 있게 보강했다.
- supervisor가 `vendor/README.md`의 selected SDK path와 `C:\Code\cannon_sdk\*` fallback도 함께 탐색하도록 보강했다.
- debug helper binary를 다시 생성했다.

검증 결과:

- `canon-helper.exe --version` 성공
- `canon-helper.exe --self-check --sdk-root <local sdk root>` 성공
- 실패 지점이 `helper-binary-missing`에서 `camera-not-found`로 이동했고,
  이후 사용자가 앱을 다시 실행해 실제 카메라 통신과 촬영 경계가 복원됐다고 확인했다.

운영 판단:

- 촬영이 전혀 시작되지 않으면 카메라 on/off보다 먼저 helper artifact 존재 여부를 본다.
- 이 단계가 열리지 않으면 나머지 requestId, event correlation 분석은 전부 후순위다.

### 2. 2026-03-29: 첫 촬영 성공 뒤 두 번째 촬영이 `Phone Required`로 과승격되던 문제

증상:

- 세션 이름 입력 후 첫 촬영은 정상 저장되고 최근 썸네일까지 표시됐다.
- 같은 세션에서 `촬영`을 한 번 더 누르면 고객 화면이 `Phone Required`로 떨어졌다.

실제 원인:

- 이 패턴은 실제 카메라 치명 장애보다, 촬영 직후의 임시 재확인 상태를 프런트가 과하게 번역한 문제에 가까웠다.
- 프런트 `capture runtime` 정규화는 `request_capture` 실패에서 readiness가 비어 있거나 파싱되지 않으면 이를 보수적으로 `Phone Required`로 승격하고 있었다.
- 그래서 `preview-waiting`, `camera-preparing`, 일반 `capture-not-ready` 같은 일시 상태와 실제 보호 전환이 고객 화면에서 구분되지 않았다.

조치:

- `request_capture` 실패 정규화에서 readiness 없는 `capture-not-ready`와 일반 host 실패를 더 이상 자동 `Phone Required`로 바꾸지 않도록 조정했다.
- 이런 경우 고객 화면은 `Preparing / 잠시 기다리기` 계열의 일시 상태로 내리고,
  `Phone Required`는 host가 명시적으로 보낸 경우에만 유지되게 했다.
- 같은 세션에서 follow-up capture가 임시 실패해도 기존 최근 썸네일이 사라지지 않도록 회귀 테스트를 추가했다.

검증 결과:

- 관련 targeted test와 lint는 통과했다.
- 이 이슈의 핵심은 카메라 연결 자체가 아니라, 촬영 실패 해석을 프런트가 과보수적으로 올렸다는 점이다.

운영 판단:

- 첫 촬영 성공 후에만 다음 촬영이 `Phone Required`로 떨어지면,
  helper disconnect보다 먼저 `request_capture` 실패 정규화와 readiness payload 유무를 본다.

### 3. 2026-03-29: duplicate-shutter 완화 뒤 새 촬영이 오래 로딩 후 `Phone Required`로 떨어지던 회귀

증상:

- `사진찍기` 버튼을 누르면 결과가 바로 오지 않고 로딩이 길게 유지됐다.
- 이후 고객 화면이 `Phone Required`로 내려갔다.
- 사용자가 보기에는 "촬영이 안 된다" 또는 "카메라가 자기 혼자 셔터를 찍는다"로 체감될 수 있었다.

실제 원인:

- 새 helper가 `camera-helper-processed-request-ids.txt`가 아직 없던 기존 세션에 붙을 때가 문제였다.
- 이 경우 helper는 request log를 처음부터 읽으면서 예전에 이미 성공했던 촬영 요청도 아직 처리 안 된 새 요청으로 오인할 수 있었다.
- 그러면 helper는 방금 누른 새 `requestId`보다 먼저 오래된 `requestId`에 반응해 셔터를 실행하고,
  host는 새 `requestId`의 `capture-accepted` 또는 `file-arrived`를 기다리다 timeout으로 `Phone Required`에 떨어질 수 있었다.

핵심 해석:

- 증상은 "새 촬영이 실패한다"였지만,
- 실제로는 helper replay 대상이 잘못돼 host correlation이 어긋난 것이 더 직접 원인이었다.

조치:

- helper startup 시 processed-request 파일만 보지 않고,
  기존 `camera-helper-events.jsonl`의 `capture-accepted`와 `file-arrived` requestId도 함께 읽어
  이미 처리된 요청 집합을 backfill 하도록 보강했다.
- 그래서 업그레이드 전 세션처럼 processed file이 비어 있어도,
  이미 성공 이력이 있는 request는 다시 실행하지 않는다.
- request log는 새로 append된 완전한 line만 incremental read 하도록 유지했다.
- helper regression test를 추가해 아래를 고정했다.
  - processed file 기반 재시작 중복 방지
  - event log 기반 기존 성공 request backfill
  - partial trailing request line 보류

당시 검증 상태:

- helper regression test와 helper build는 통과했다.
- 부스 앱과 실카메라에서 최종 customer flow 재확인은 별도 하드웨어 검증이 필요했다.

운영 판단:

- `사진찍기` 후 오래 로딩되다가 `Phone Required`가 뜨면,
  새 요청이 안 간 것보다 먼저 `requestId` correlation drift와 stale request replay를 의심한다.
- 특히 기존 세션을 이어서 쓰는 재현에서 이 가능성이 높다.

### 4. 2026-03-29 13:52:43 +09:00: `사진찍기` 직후 세션 입력 화면으로 튕기던 회귀

증상:

- 사용자가 카메라 `ready`를 확인한 뒤 `사진찍기`를 누르면, 고객 화면이 세션 입력 화면으로 튀는 사례가 있었다.
- 다시 세션 이름을 입력하고 `사진찍기`를 누르면, 직전 요청의 셔터 반응처럼 보이는 늦은 반응이 뒤따를 수 있었다.
- 이후 한 번 더 `사진찍기`를 누르면 `Phone Required`로 이어질 수 있어, 사용자 체감상으로는
  "첫 촬영에서 세션이 날아갔고, 다음 촬영에서 카메라가 뒤늦게 반응한 뒤 결국 보호 상태로 잠긴다"에 가깝게 보였다.

실제 해석:

- 이 패턴에서 첫 번째 문제는 "세션이 실제로 종료됐다"보다,
  **촬영 직후의 same-session `session-not-found`를 프런트가 세션 초기화로 과하게 번역한 것**일 가능성이 컸다.
- 촬영 경계에서 host가 일시적으로 manifest를 다시 확인하는 동안 같은 세션의 `session-not-found`가 들어오면,
  이전 프런트 로직은 이를 곧바로 `처음 화면으로` 해석할 수 있었다.
- 그러면 아직 같은 세션 아래에서 진행 중인 request correlation 문제를 고객 화면이 먼저 잃어버리고,
  사용자는 세션을 다시 입력하게 된다.
- 그 뒤 늦게 도착한 직전 셔터 반응이나 후속 timeout이 겹치면,
  실제 원인은 capture round-trip correlation인데 화면 체감은 "첫 클릭에 세션이 날아간 뒤 다음 클릭에서 고장"처럼 왜곡될 수 있다.

조치:

- `request_capture` 경계에서 들어온 same-session `session-not-found`는 더 이상 즉시 `start-session`으로 승격하지 않고,
  고객 기준 `Preparing / 잠시 기다리기` 상태로 일단 유지되게 조정했다.
- 추가로 `SessionProvider`는 **촬영 진행 중**에 같은 세션의 `session-not-found`가 readiness refresh 경로에서 들어와도,
  그 순간에는 세션을 바로 초기화하지 않고 현재 capture 화면과 세션 문맥을 보존하도록 완화했다.
- 이 보정으로 고객 화면은 촬영 직후의 애매한 재확인 상태에서 세션 입력 화면으로 튀지 않고,
  현재 세션을 유지한 채 재확인 상태를 보여 주게 된다.

코드 검증 결과:

- 아래 자동 검증은 통과했다.
  - `pnpm vitest run src/capture-adapter/services/capture-runtime.test.ts src/session-domain/state/session-provider.test.tsx`
  - `pnpm exec eslint src/capture-adapter/services/capture-runtime.ts src/capture-adapter/services/capture-runtime.test.ts src/session-domain/state/session-provider.tsx src/session-domain/state/session-provider.test.tsx`
  - `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`
  - `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- 당시에는 실카메라에서의 최종 customer flow 결과를 아직 이 문서에 확정하지 않았다.
- 최종 하드웨어 검증 결과는 아래 `9번 항목`에 후속 기록했다.

운영 판단:

- `사진찍기` 직후 세션 입력 화면으로 튀면, 세션이 정말 사라졌다고 먼저 단정하지 않는다.
- 같은 세션의 `session-not-found`가 capture boundary에서 일시적으로 들어온 것인지 먼저 본다.
- 특히 직후에 늦은 셔터 반응이나 후속 `Phone Required`가 이어지면,
  세션 lifecycle보다 capture request correlation과 프런트 세션 보존 경계를 함께 봐야 한다.

### 5. 2026-03-29 14:10:00 +09:00 작업 시작 기록: follow-up capture가 실제 `capture-download-timeout`으로 잠기고 있었다

사용자 최신 제보:

1. 첫 촬영은 성공했고 최근 썸네일도 확인됐다.
2. 이어서 다시 `사진 찍기`를 누르면 버튼이 오래 로딩된 뒤 `Phone Required`가 발생했다.

이번 회차에서 실제 런타임 evidence를 다시 확인한 결과, 이 패턴은 이번에는 프런트 과승격이 아니라
**helper/host capture timeout budget이 실제 RAW handoff 시간보다 먼저 닫히는 문제**로 보는 것이 맞았다.

실제 확인 근거:

- 최신 하드웨어 세션 `session_000000000018a136020549d324`에서는
  - `2026-03-29 13:38:36 +09:00` 첫 요청 `request_000000000018a136035d3f6b74`가 성공했다.
  - `2026-03-29 13:38:41 +09:00` 두 번째 요청 `request_000000000018a136047e273448`는 `capture-accepted`까지만 기록되고,
    `2026-03-29 13:38:47 +09:00`에 helper가 `capture-download-timeout`으로 `recovery-status`와 `helper-error`를 남겼다.
- 더 최신 세션 `session_000000000018a136cb6a10bce4`에서도 같은 패턴이 반복됐다.
  - 앞의 두 요청은 `file-arrived`까지 성공했다.
  - 세 번째 요청 `request_000000000018a136cf7d3a1728`는 `capture-accepted` 뒤 `file-arrived`가 오지 않았고,
    `2026-03-29 13:53:19 +09:00`에 같은 `capture-download-timeout`으로 떨어졌다.
- 두 세션 모두 실패 뒤 최종 helper status는 다시 `cameraState=ready`, `helperState=healthy`로 돌아왔다.
  즉 이번 실패는 "카메라를 완전히 잃었다"보다, **후속 촬영 RAW transfer가 5초 기본 timeout 안에 닫히지 못한 것**에 더 가깝다.

현재 판단:

- `capture-accepted`가 이미 찍혔으므로 새 요청 소비 자체가 막힌 것은 아니다.
- 이번 회차의 직접 원인은 stale replay보다 **host/helper 기본 capture timeout이 실장비 follow-up capture에서 너무 짧은 것**이다.
- 구현은 host와 helper의 기본 timeout budget을 함께 늘려, 실제 늦은 RAW handoff를 premature `Phone Required`로 고정하지 않도록 맞춘다.

### 6. 2026-03-29 14:22:00 +09:00 구현 후 기록: host/helper capture timeout budget을 현실값으로 늘렸다

이번 회차 조치:

- host `capture round-trip` 기본 timeout을 `5초 -> 20초`로 늘렸다.
- Canon helper의 RAW download completion 기본 timeout을 `5초 -> 15초`로 늘렸다.
- helper도 host와 같은 runtime root의 `.camera-helper-capture-timeout-ms` override 파일을 읽도록 맞췄다.
  그래서 현장/검증 환경에서 timeout을 조정할 때 host와 helper가 서로 다른 값을 보지 않는다.

의도:

- 실장비 follow-up capture에서 `capture-accepted` 뒤 RAW handoff가 5초를 넘겨도 바로 `Phone Required`로 고정하지 않는다.

### 7. 2026-03-31 22:32 +09:00 재검증: 남아 있던 회귀는 `capture-trigger-failed(0x00008d01)` 경로였다

사용자 재검증 결과:

1. `사진 찍기`를 누르면 카메라가 초점을 잡으려다 실패했다.
2. 그 뒤 고객 화면은 다시 `Phone Required`로 떨어졌다.

이번 회차에서 실제 evidence를 다시 맞춰 보니, 남아 있던 실패는 `capture-start-timeout`이 아니었다.

실제 확인 근거:

- 최신 앱 로그는 같은 세션 `session_000000000018a1f048a428ef78`에 대해
  `capture_readiness ... customer_state=Phone Required ... live_truth=fresh:matched:ready:healthy`
  를 반복 기록했다.
- 즉 프런트 과승격이 아니라, host가 이미 `phone-required`를 유지하고 있었다.
- 같은 세션의 `camera-helper-events.jsonl`에는 두 번째 요청에서
  `helper-error(detailCode=capture-trigger-failed, message=셔터 명령을 보낼 수 없었어요: 0x00008d01)`
  가 남아 있었다.
- helper status는 이후 다시 `camera-ready`로 회복했지만,
  `session.json` lifecycle stage는 그대로 `phone-required`였다.

결론:

- 1차 수정은 `capture-start-timeout`만 완화했고,
  실제 남아 있던 하드웨어 경로인 `AF_NG(0x00008d01)` 기반 `capture-trigger-failed`는 놓쳤다.
- 그래서 helper는 회복해도, host manifest가 이미 잠긴 뒤라 고객 화면은 계속 `Phone Required`로 보였다.

이번 회차 조치:

- helper가 `AF_NG(0x00008d01)`를 `capture-focus-not-locked` 재시도 가능 오류로 기록하게 바꿨다.
- host는 이 오류와 legacy `capture-trigger-failed + 0x00008d01` 흔적을 retryable capture failure로 취급한다.
- retryable trigger failure는 더 이상 manifest를 `phone-required`로 저장하지 않는다.
- 이전 버전이 남긴 같은 유형의 `phone-required`도 helper live truth가 `fresh/matched/ready/healthy`로 회복하면 `capture-ready`로 풀리게 했다.

### 8. 2026-03-31 22:50 +09:00 재검증: 성공 촬영 자체는 됐지만 세션 결과 저장이 비어 있었다

사용자 최신 제보:

1. 초점이 안 잡힐 때는 이제 `Ready`를 유지한다.
2. 이후 실제 초점을 잡고 촬영하면 셔터는 동작한다.
3. 하지만 `현재 세션 사진`에는 결과물이 올라오지 않는다.

실제 확인 근거:

- 최신 세션 `session_000000000018a1f147327c30ec`
- helper request/event log 기준으로는 아래가 모두 정상으로 닫혔다.
  - 실패 두 번: `capture-focus-not-locked`
  - 성공 두 번: `capture-accepted -> file-arrived`
- 실제 RAW 파일도 아래 경로에 생성됐다.
  - `captures/originals/capture_20260331135039387_b799da5e40.CR2`
  - `captures/originals/capture_20260331135043383_fe05ed0534.CR2`
- 그런데 같은 세션 `session.json`은 여전히
  - `captures: []`
  - `lifecycle.stage: preset-selected`
  로 남아 있었다.

결론:

- 이번 회차의 직접 원인은 helper나 초점 실패가 아니라, **성공 촬영 뒤 host manifest persist가 비는 문제**다.
- customer 화면이 다시 `Ready`로만 남은 것도 이 저장 실패가 고객 친화적인 명시 오류로 전달되지 않았기 때문이다.

이번 회차 조치:

- `write_session_manifest(...)`에 Windows sharing/rename race 대응 retry를 추가한다.
- 성공 `file-arrived` 뒤 capture persist가 retry budget 안에서 회복되면 그대로 세션 결과에 연결한다.
- retry budget 밖 persist 실패는 더 이상 조용히 `Ready`로 복귀시키지 않고, 명시적인 보호 상태로 올린다.
- capture boundary 로그를 추가해 다음 재현에서는 `file-arrived 이후 저장`과 `preview 반영` 단계를 바로 구분할 수 있게 한다.

### 9. 2026-03-31 23:05 +09:00 보완: 유휴 상태 재시작처럼 보인 현상은 dev watcher 재실행이었다

사용자 제보:

1. 변경 검증 전에 앱을 빌드하고 실행했다.
2. 아무 동작을 하지 않아도 앱이 스스로 껐다 켜지는 것처럼 보였다.

실제 로그 해석:

- `tauri dev`가 먼저 Rust compile error를 만났다.
- 이후 `src-tauri/tests/capture_readiness.rs`, `src-tauri/src/session/session_repository.rs` 같은 파일 변경을 계속 감지했다.
- 그때마다 `cargo run --no-default-features --color always --`가 다시 호출됐고,
  `target/debug/boothy.exe`가 반복 실행됐다.
- 즉 이 현상은 product runtime self-restart가 아니라, **dev watcher 기반 재빌드/재실행 루프**였다.

이번 회차 조치:

- stable Rust에서 막히던 2024 전용 문법 사용을 정리해 compile loop 원인을 제거했다.
- 개발 검증용 `pnpm run dev:desktop:stable` 스크립트를 추가했다.
- 이 경로는 `pnpm tauri dev --no-watch`를 써서, 테스트/문서 파일 변경 때문에 앱이 다시 뜨는 현상을 막는다.
- 동시에 host 기본값을 helper보다 조금 더 길게 두어, 실제 helper-side failure가 있으면 host timeout보다 먼저 helper 판단이 올라오게 했다.

코드 검증 결과:

- `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check` 통과
- `dotnet build sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj` 통과
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj` 통과

당시 남은 확인:

- 이번 수정은 실제 로그에 남은 `capture-download-timeout` 패턴을 기준으로 한 budget 보정이었다.
- 후속 하드웨어 검증 결과는 아래 `9번 항목`에 기록했다.

### 7. 2026-03-29 14:15:00 +09:00 추가 작업 시작 기록: timeout budget 확대 후에도 두 번째 촬영이 여전히 transfer 단계에서 멈췄다

사용자 재검증 결과:

1. 첫 번째 `사진 찍기`는 정상 저장되고 썸네일까지 완료됐다.
2. 두 번째 촬영은 여전히 `Phone Required`로 떨어졌다.

최신 evidence:

- 새 세션 `session_000000000018a1379d04825f04`
  - 첫 요청 `request_000000000018a1379e8e765624`는 `2026-03-29 14:08:03 +09:00`에 `file-arrived`까지 성공했다.
  - 두 번째 요청 `request_000000000018a137a0b4430a40`는 `capture-accepted` 뒤
    `2026-03-29 14:08:27 +09:00`에 다시 `capture-download-timeout`으로 `recovery-status`와 `helper-error`를 남겼다.
- 즉 이전처럼 `5초`에서 조급하게 포기하던 문제는 줄었지만,
  **이번에는 15초 budget 안에서도 실제 RAW transfer가 닫히지 않는 근본 문제가 남아 있었다.**

현재 추가 판단:

- 새 요청 소비나 프런트 과승격 문제가 아니라, **helper capture/download 경계에서 follow-up transfer가 막히는 문제**를 더 직접 봐야 한다.
- 특히 현재 helper는 capture in-flight 동안에도 keep-alive 명령을 계속 보낼 수 있고,
  object event에서 내려온 RAW download를 별도 `Task.Run(...)` thread로 넘기고 있다.
- 이번 회차에서는 이 두 경계를 줄여, **촬영 중 추가 SDK 명령 간섭을 막고 transfer 처리를 더 직렬화하는 쪽**으로 보정한다.

### 8. 2026-03-29 14:35:00 +09:00 구현 후 기록: 촬영 중 keep-alive 간섭을 막고 RAW transfer를 callback 경계에서 직접 처리하도록 조정했다

이번 회차 조치:

- helper는 **capture in-flight 동안 `ExtendShutDownTimer` keep-alive 명령을 더 이상 보내지 않게** 조정했다.
- `DirItemRequestTransfer` object event가 오면 RAW download를 별도 `Task.Run(...)` threadpool로 넘기지 않고,
  **SDK callback 경계에서 바로 처리**하도록 바꿨다.

의도:

- follow-up capture 중 helper가 추가 SDK 명령을 섞어 보내며 transfer를 흔드는 가능성을 줄인다.
- RAW download를 임의 thread hop 없이 더 일관된 경계에서 처리해,
  두 번째 촬영에서 `capture-accepted` 뒤 transfer가 사라지는 패턴을 완화한다.

코드 검증 결과:

- `dotnet build sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj` 통과
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj` 통과

당시 남은 확인:

- 이번 수정은 최신 `2026-03-29 14:08 +09:00` 세션 evidence를 기준으로 한 helper-side stabilization이었다.
- 후속 하드웨어 검증 결과는 아래 `9번 항목`에 기록했다.

### 9. 2026-03-29 최종 하드웨어 검증 결과: 카메라 상태와 연속 촬영이 7장까지 정상 동작했다

사용자 최종 검증 결과:

1. 카메라 상태가 정상으로 유지됐다.
2. 같은 세션에서 `사진 찍기` 버튼을 연속으로 눌러도 7장까지 모두 정상 동작했다.
3. 각 촬영 결과가 썸네일과 최근 결과 화면에 이어서 반영됐다.
4. 기존에 재현되던 `Phone Required`, 버튼 무한 로딩, 세션 입력 화면 복귀는 더 이상 나타나지 않았다.

최종 해석:

- 이번 문제는 단일 원인 하나가 아니라, 촬영 경계 여러 곳의 작은 실패가 합쳐져 고객에게는 `두 번째 촬영부터 고장나는 문제`처럼 보이던 케이스였다.
- helper 실행 경계 복구, 프런트 실패 해석 보정, stale request replay 차단, same-session 세션 보존, timeout budget 확대, helper transfer 안정화를 순서대로 정리한 뒤 연속 촬영이 정상화됐다.

최종 결론:

- 같은 세션의 follow-up capture round-trip이 `request_capture -> capture-accepted -> file-arrived -> 썸네일 반영`까지 반복해서 닫히는 것이 확인됐다.
- 이 문서에 남아 있던 `hardware validation 필요` 상태는 이번 사용자 검증으로 해소됐다.

### 10. 2026-03-29 추가 확인: 이전 session helper orphan이 남아 새 연결이 바로 `Phone Required`로 떨어질 수 있었다

증상:

- 최신 수정 전까지 정상 동작하던 카메라 연결 상태가 다시 `Phone Required`로 떨어지는 회귀가 보고됐다.
- 이번 케이스는 첫 촬영/두 번째 촬영 경계보다 더 앞단에서, `카메라 연결상태 확인` 단계부터 바로 막히는 쪽에 가까웠다.

실제 원인:

- host 로그는 `live_truth=fresh:matched:error:error`를 남겼고,
  최신 session helper status는 `detailCode=session-open-failed`를 기록하고 있었다.
- 동시에 실행 중인 `canon-helper.exe`를 확인하면,
  **이전 session에 묶인 helper가 여전히 `ready/healthy` 상태로 살아 있는 경우**가 있었다.
- 이 orphan helper가 카메라 세션을 계속 잡고 있으면, 새 session helper는 `EdsOpenSession(...)`에서 충돌할 수 있고
  booth는 이를 실제 보호 상태로 해석해 `Phone Required`로 내려갈 수 있었다.

조치:

- Rust helper supervisor가 새 helper를 띄우기 전에
  같은 runtime root를 바라보는 stale helper process를 먼저 정리하도록 보강했다.
- 새 helper 실행 시 부모 앱 PID를 함께 넘기고,
  helper는 부모 프로세스가 사라지면 스스로 종료하도록 보강했다.

운영 판단:

- 최신 session status가 `session-open-failed`인데,
  별도 `canon-helper.exe`가 이전 session id로 계속 떠 있으면
  카메라 미발견보다 먼저 **stale helper orphan 충돌**을 본다.

## 오진하기 쉬운 포인트

### "`Phone Required`가 떴으니 카메라 연결이 바로 끊긴 것이다"

반드시 그렇지 않다.

- 첫 촬영 성공 뒤 두 번째 촬영에서만 재현되면 프런트 과승격일 수 있다.
- 새 요청과 오래된 요청의 `requestId`가 어긋나 timeout이 난 경우에도 같은 증상이 나온다.

### "`Ready`가 보였으니 촬영 성공도 곧 따라와야 한다"

틀릴 수 있다.

- `Ready`는 촬영 가능 진입 조건이지, 촬영 성공 확정이 아니다.
- 같은 `requestId`에 대한 `capture-accepted`와 `file-arrived`, 실제 파일 존재까지 봐야 한다.

### "셔터가 혼자 다시 눌린 것 같으니 새 요청이 중복 발행됐다"

직접 원인이 다를 수 있다.

- 실제로는 helper가 오래된 request log line을 새 요청으로 잘못 재소비했을 가능성이 있다.
- 이 경우 프런트 새 요청보다 helper의 stale replay 방지를 먼저 본다.

### "세션 입력 화면으로 튕겼으니 세션이 실제로 끝났거나 사라졌다"

반드시 그렇지 않다.

- 촬영 직후의 same-session `session-not-found`를 프런트가 과하게 초기화로 번역했을 수 있다.
- 이 경우 실제 본문제는 세션 lifecycle이 아니라 capture boundary 재확인 또는 request correlation일 수 있다.
- 특히 직후에 늦은 셔터 반응이 뒤따르면, 세션 종료보다 stale request replay나 round-trip mismatch를 함께 의심한다.

### "카메라를 못 찾는 것 같으니 무조건 장비 문제다"

촬영이 아예 시작되지 않는 시점이라면 장비보다 helper artifact가 먼저 막혔을 수 있다.

- `helper-binary-missing`이면 discovery 이전에 helper attach부터 복구해야 한다.

## 다음 에이전트용 진단 순서

### 1. helper 실행 경계부터 확인

먼저 `boothy.exe`와 `canon-helper.exe`가 실제로 떠 있는지 본다.

```powershell
Get-Process | Where-Object {
  $_.ProcessName -like 'boothy*' -or $_.ProcessName -like '*canon-helper*'
} | Select-Object ProcessName, Id, StartTime, Path
```

helper가 안 떠 있거나 binary가 없으면, 촬영 round-trip 분석보다 helper artifact 복구가 먼저다.

### 2. 최신 session 하나를 고정해서 본다

런타임 root:

```text
%LOCALAPPDATA%\com.tauri.dev\booth-runtime
```

최신 session 확인:

```powershell
Get-ChildItem -Path $env:LOCALAPPDATA\com.tauri.dev\booth-runtime\sessions -Directory |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 5 FullName, Name, LastWriteTime
```

반드시 같은 session 기준으로 아래 파일을 함께 본다.

- `diagnostics/camera-helper-requests.jsonl`
- `diagnostics/camera-helper-events.jsonl`
- `diagnostics/camera-helper-status.json`
- `diagnostics/camera-helper-processed-request-ids.txt`

### 3. 최신 `requestId`가 어디까지 갔는지 본다

우선 request log와 event log를 tail 해서 같은 `requestId`가 이어지는지 확인한다.

```powershell
Get-Content -Tail 20 <camera-helper-requests.jsonl 경로>
```

```powershell
Get-Content -Tail 50 <camera-helper-events.jsonl 경로>
```

특정 `requestId` 추적:

```powershell
Select-String -Path <camera-helper-events.jsonl 경로> -Pattern "<requestId>"
```

판단 기준:

- `capture-accepted`와 `file-arrived`가 같은 `requestId`로 이어지면 helper correlation은 대체로 정상이다.
- 새 요청 직후 오래된 `requestId`가 다시 등장하면 stale replay를 먼저 의심한다.
- 이벤트가 전혀 없으면 helper 소비 경계나 helper 실행 자체를 우선 본다.

### 4. host가 임시 실패를 어떻게 번역했는지 본다

같은 시점의 `request_capture` 오류 payload나 capture runtime 로그에서 readiness가 같이 내려왔는지 확인한다.

판단 기준:

- readiness 없는 일반 실패를 프런트가 `Phone Required`로 올리면 과승격 문제다.
- host가 명시적으로 `Phone Required`를 보낸 경우만 실제 보호 상태로 본다.

### 5. 촬영 성공은 파일과 최근 결과까지 확인한다

`file-arrived`만 보고 끝내지 말고 실제 파일과 최근 화면 반영까지 본다.

- 세션 `captures/originals/` 아래에 파일이 실제로 있는지 확인한다.
- 최근 썸네일이 이미 보이던 세션이라면, 후속 촬영 실패가 나도 이전 결과가 사라지지 않는지 함께 본다.

## 앱 반영 참고

카메라 상태 반영 자체는 현재 별도 경계에서 정리되어 있으므로,
촬영 기능 관점에서는 아래 원칙만 기억하면 된다.

| 상황 | 고객 화면 반영 | 핵심 원칙 |
| --- | --- | --- |
| host readiness가 `Ready`이고 `canCapture=true` | `사진찍기` 가능 상태 | helper raw 상태가 아니라 host normalized readiness를 기준으로 삼는다. |
| `camera-preparing`, `preview-waiting`, 일반 `capture-not-ready`, readiness 미포함 일반 실패 | `Preparing / 잠시 기다리기` 계열 | 임시 재확인 상태를 `Phone Required`로 올리지 않는다. |
| capture 직후 same-session `session-not-found` | 현재 세션 유지 + `Preparing / 잠시 기다리기` 계열 | 촬영 중 일시 재확인을 곧바로 세션 초기화로 번역하지 않는다. |
| host가 명시적으로 `Phone Required` 또는 post-end finalized 보호 상태를 반환 | `Phone Required` | 프런트 추정이 아니라 host 명시일 때만 보호 상태를 고정한다. |
| 첫 촬영 성공 후 다음 촬영이 임시 실패 | 기존 최근 썸네일 유지 + 일시 대기 상태 | 이미 성공한 결과를 지우거나 세션을 불필요하게 보호 상태로 잠그지 않는다. |

짧게 정리하면:

- 앱은 helper raw 상태를 직접 믿지 말고 host normalized readiness를 기준으로 움직여야 한다.
- `사진찍기` 실패는 모두 같은 실패가 아니며, temporary wait와 true block을 분리해야 한다.
- 촬영 성공 여부는 버튼 클릭이 아니라 `requestId` correlation과 실제 파일 도착으로 판단해야 한다.

### 2026-03-31 23:56 +09:00 추가 확인: 이번 `Phone Required`는 카메라가 아니라 legacy preset bundle 렌더 호환성 문제였다

최신 실재현 세션:

- `session_000000000018a1f4e305186810`

실제 확인 근거:

- `camera-helper-events.jsonl`에는 같은 요청에 대해
  - `capture-accepted`
  - `file-arrived`
  가 정상으로 남았다.
- `camera-helper-status.json` 최종 상태도
  - `cameraState=ready`
  - `helperState=healthy`
  - `detailCode=camera-ready`
  였다.
- 즉 촬영 자체와 helper correlation은 정상으로 닫혔다.
- 그런데 `session.json`은
  - `captures[0].renderStatus = renderFailed`
  - `lifecycle.stage = phone-required`
  로 남았다.
- 같은 세션 `diagnostics/timing-events.log`에는
  - `render-failed stage=preview reason=bundle-resolution-failed`
  가 기록됐다.
- 실제 `renders/previews/<captureId>.jpg` 파일은 존재했으므로, 이번 `Phone Required`는 카메라 실패가 아니라
  **촬영 후 preview render 경계가 현재 선택된 preset bundle을 runtime render bundle로 해석하지 못한 문제**였다.

원인:

- 현재 선택된 `preset_test-look@2026.03.31` published bundle은 구형 형식이라
  `previewProfile` / `finalProfile` 필드가 없었다.
- 새 render 경계는 이 필드를 필수로 보면서 `bundle-resolution-failed`로 떨어졌고,
  그 결과 촬영 성공 직후 세션이 `Phone Required`로 승격됐다.

이번 회차 조치:

- legacy published bundle에 `previewProfile` / `finalProfile`가 없어도,
  host가 안전한 기본 render profile을 만들어 runtime bundle로 받아들이도록 호환 경로를 추가한다.
- preview/final render 실패 시
  - `capture_preview_render_failed`
  - `capture_final_render_failed`
  로그에 `reason_code`와 세부 원인을 함께 남긴다.
- 재발 방지를 위해
  - legacy published bundle loader 테스트
  - legacy bundle 선택 상태의 capture -> preview 준비 테스트
  를 추가한다.

## 관련 파일 / 문서

- [camera-helper-troubleshooting-history.md](/C:/Code/Project/Boothy/history/camera-helper-troubleshooting-history.md)
- [camera-helper-sidecar-protocol.md](/C:/Code/Project/Boothy/docs/contracts/camera-helper-sidecar-protocol.md)
- [capture-runtime.ts](/C:/Code/Project/Boothy/src/capture-adapter/services/capture-runtime.ts)
- [capture-runtime.test.ts](/C:/Code/Project/Boothy/src/capture-adapter/services/capture-runtime.test.ts)
- [session-provider.test.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.test.tsx)
- [CanonHelperService.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs)
- [JsonFileProtocol.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs)
- [SessionPaths.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/SessionPaths.cs)

### 2026-04-01 00:03 +09:00 구현 완료 메모

- `preset_bundle.rs`에서 legacy published bundle이 `previewProfile` / `finalProfile` 없이도 runtime render bundle로 로드되도록 호환 fallback을 추가했다.
- 새 fallback은 기존 `darktableProjectPath`, `xmpTemplatePath`, preview asset을 그대로 쓰고, preview/final render profile만 안전한 기본값으로 합성한다.
- 그 결과 `preset_test-look@2026.03.31` 같은 구형 published bundle에서도 capture 이후 preview render가 `bundle-resolution-failed`로 끊기지 않도록 복구했다.
- `ingest_pipeline.rs`에는 `capture_preview_render_failed`, `capture_final_render_failed` 경고 로그를 추가해, 다음 재발 시 카메라 실패와 렌더 실패를 로그 한 줄로 바로 구분할 수 있게 했다.
- 회귀 방지 테스트:
  - `published_runtime_bundle_loader_accepts_legacy_bundles_without_render_profiles`
  - `capture_flow_legacy_published_bundle_without_render_profiles_still_prepares_preview`
- 검증:
  - `cargo test --test session_manifest --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
  - `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
  - 둘 다 통과했다.

### 2026-04-01 00:15 +09:00 사용자 실기기 재검증 결과

- 사용자가 실제 부스에서 다시 확인한 결과, 초점이 잡히지 않는 경우에도 customer 상태가 다시 `Ready`로 복귀했다.
- 같은 세션에서 곧바로 다시 `사진 찍기`를 누르면 후속 촬영도 정상 진행됐다.
- 즉 이번 회차에서 문제였던
  - 초점 실패 뒤 `Phone Required` 고정
  - 다음 촬영 차단
  두 회귀는 현장 기준으로 해소됐다.

후속 관찰:

- 카메라 회복과 별도로, 사용자는 `최근 세션` 영역에서 이미지가 제대로 표현되지 않는 새 이슈를 보고했다.
- 이 문제는 capture/blocking 경계보다 preview asset 노출 또는 session rail 표시 경계를 우선 의심해야 한다.
- 후속 분석은 `current-session-photo-troubleshooting-history.md`에 이어서 정리한다.

### 2026-04-01 00:44 +09:00 후속 구현 메모

- 후속 분석 결과, `최근 세션` 미표시는 카메라 회귀 재발이 아니라 render worker 결손이었다.
- 기존 render seam은 실제 preview raster 대신 invocation metadata 텍스트를 `.jpg`로 기록할 수 있었고,
  그 상태에서도 `previewReady`가 올라갈 수 있었다.
- 이번 회차에서 render worker를 실제 `darktable-cli` 실행 + 출력 JPEG 검증 기준으로 교체했고,
  legacy `.svg` / invalid `.jpg` preview truth는 자동 복구 경로를 추가했다.
- 따라서 현 시점의 경계는
  - 카메라 촬영/재시도 회귀: 해결됨
  - 최근 세션 preview 표시: 실제 raster 산출물 기준으로 복구됨
  으로 정리한다.

### 2026-04-01 01:18 +09:00 `촬영 후 Phone Required` 재발의 실제 원인 정리

이번 재발은 카메라 readiness 퇴행이 아니었다.

실측 근거:

- 최신 런타임 로그 `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`에서
  실제 실패 세션은 `session_000000000018a1f7a4fb1a8a7c`였다.
- 같은 세션의 helper 증거는 정상으로 닫혔다.
  - `diagnostics/camera-helper-events.jsonl`: `capture-accepted -> file-arrived`
  - `diagnostics/camera-helper-status.json`: `cameraState=ready`, `helperState=healthy`
- 그런데 세션 manifest와 timing log는 아래처럼 남아 있었다.
  - `captures[0].renderStatus = renderFailed`
  - `lifecycle.stage = phone-required`
  - `timing-events.log`: `render-failed stage=preview reason=bundle-resolution-failed`

즉 고객 화면의 `Phone Required`는
카메라/초점/sidecar round-trip 실패가 아니라,
**촬영 성공 뒤 preview render가 기본 preset bundle을 runtime render bundle로 해석하지 못한 결과**였다.

직접 원인:

- 실제 부스 런타임 루트 `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published` 아래의
  기본 preset bundle들 중
  - `preset_daylight@2026.03.27`
  - `preset_mono-pop@2026.03.27`
  - `preset_soft-glow@2026.03.27`
  는 구형 summary-only 형식이었다.
- 이 bundle들은 `darktableVersion`, `xmpTemplatePath`, render profile 메타데이터가 빠져 있었고,
  새 render loader에서는 `bundle-resolution-failed`로 이어졌다.
- `ensure_default_preset_catalog_in_dir()`도 기존에는
  "이미 published bundle이 하나라도 있으면 seed bootstrap을 건너뛴다"는 조건이라,
  오래된 기본 bundle이 영구히 업그레이드되지 못했다.

이번 회차 수정:

- 기본 preset seed bootstrap을 보강해,
  기존 catalog가 이미 있어도 기본 seed bundle이 구형 형식이면 runtime render metadata를 backfill 하도록 수정했다.
- readiness 계산에 recoverable render failure 복구를 추가해,
  latest capture가 `renderFailed`라도
  helper truth가 `fresh/matched/ready`이면 preview rerender를 다시 시도하고 성공 시 세션을 회복시킨다.
- 실제 현장 런타임 폴더의 기본 bundle 3종도 같은 metadata로 backfill 해 두었다.

검증:

- `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test --test preset_authoring --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- 둘 다 통과했다.

재발 시 우선 확인할 것:

- helper 이벤트가 `capture-accepted -> file-arrived`로 닫혔는데도 `Phone Required`가 뜨면,
  먼저 `timing-events.log`의 `render-failed` reason을 본다.
- 기본 preset 사용 중이면 `preset-catalog/published/<preset>/<version>/bundle.json`에
  `darktableVersion`, `xmpTemplatePath`, render profile 메타데이터가 실제로 있는지 확인한다.

### 2026-04-01 01:28 +09:00 추가 재발 정리: 이번에는 `render-cli-missing`이 직접 원인이었다

사용자 재현:

- `사진 찍기`를 누르면 실제 카메라 셔터는 정상 동작한다.
- 하지만 곧바로 customer 상태가 다시 `Phone Required`로 바뀐다.

이번 회차 실측 근거:

- 최신 실패 세션은 `session_000000000018a1f881d9d4cb74`였다.
- 같은 세션의 `session.json`은
  - `captures[0].renderStatus = renderFailed`
  - `lifecycle.stage = phone-required`
  로 남아 있었다.
- `Boothy.log`와 `diagnostics/timing-events.log`에는 공통으로 아래가 반복됐다.
  - `capture_preview_render_failed ... reason_code=render-cli-missing`
  - `darktable-cli를 시작하지 못했어요: binary=darktable-cli error=program not found`
- 반면 부스 PC에는 실제 실행 가능한 darktable CLI가 설치돼 있었다.
  - `C:\Program Files\darktable\bin\darktable-cli.exe`
  - 수동 실행 결과 같은 RAW/XMP 조합으로 실제 JPEG export가 성공했다.

정리:

- 이번 `Phone Required`는 preset bundle 형식 문제도, helper readiness 문제도 아니었다.
- **render worker가 PATH 안의 `darktable-cli`만 기대하도록 바뀌면서, 표준 설치 경로의 darktable를 못 찾은 경로 탐색 회귀**였다.

잘 되던 조건과 이번에 깨진 조건:

- history/runbook 기준으로 정상 조건은
  - 실카메라 촬영 성공
  - published bundle runtime metadata 유효
  - pinned darktable runtime 사용 가능 상태
  가 모두 맞는 경우다.
- 이번 부스는 darktable runtime 자체는 설치돼 있었지만,
  현재 코드가 그 설치 위치를 직접 찾지 못해서 "사용 가능 상태" 판정을 스스로 놓쳤다.

이번 회차 수정:

- render worker가 `BOOTHY_DARKTABLE_CLI_BIN` env override가 없더라도,
  Windows 표준 설치 경로를 먼저 탐색하도록 바꿨다.
  - `Program Files\\darktable\\bin\\darktable-cli.exe`
  - `ProgramW6432\\darktable\\bin\\darktable-cli.exe`
  - `LOCALAPPDATA\\Programs\\darktable\\bin\\darktable-cli.exe`
- 실패 로그에는 어떤 source에서 binary를 골랐는지 같이 남기도록 보강했다.
- 따라서 PATH에 darktable이 등록돼 있지 않은 부스 PC에서도 촬영 후 preview render가 이어질 수 있다.

검증:

- `cargo test darktable_cli_resolution --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- 둘 다 통과했다.

### 2026-04-01 01:19 +09:00 사용자 실기기 재검증: correctness는 회복됐지만 preview latency가 제품 기준 밖이다

사용자 확인 결과:

- `사진 찍기` 이후 더 이상 `Phone Required`로 고정되지는 않았다.
- 실제 카메라 촬영, preview waiting, `최근 세션` 반영까지 전체 흐름은 다시 동작했다.
- 하지만 `Preview Waiting`에서 실제 preview가 보이고, 같은 사진이 `최근 세션`에 올라오는 시간이
  체감상 `5초 이상`으로 느껴졌다.

제품 판단:

- 이 상태는 correctness bug가 아니라 **성능/응답성 gate 실패**에 가깝다.
- 사용자 체감 기준으로 촬영 직후 확인 피드백이 너무 늦고,
  현재 속도는 booth product로 사용하기 어렵다는 현장 판단이 나왔다.

현재 해석:

- 이번 회차에서 `Phone Required`는 잡혔지만,
  render worker 실행부터 preview visible, recent session 반영까지의 latency는 아직 실장비 기준으로 닫히지 않았다.
- 즉 정상 조건은 이제
  - 실카메라 촬영 성공
  - render worker 성공
  - preview/recent session 반영이 booth-safe latency 안에서 닫힘
  으로 다시 정의해야 한다.

후속 우선 과제:

- 다음 분석에서는 `capture_request_saved -> render-ready -> preview visible -> recent session visible` 구간별 시간을 로그로 분리해,
  실제 병목이
  - darktable render 자체
  - render 완료 후 manifest/readiness 반영
  - 프런트 current/recent session 동기화
  중 어디에 있는지 먼저 좁혀야 한다.
- 이 이슈는 Story 1.8의 "preset apply truth"와는 별개로, product acceptance를 막는 latency blocker로 취급한다.

### 2026-04-01 01:37 +09:00 구현 메모: preview를 display-sized actual render로 낮췄지만, darktable 자체는 여전히 수초 단위였다

이번 회차 구현:

- preview render는 full-resolution이 아니라 booth display용 capped raster로 바꿨다.
  - `darktable-cli --hq false --width 1280 --height 1280`
- final render는 기존 full-resolution 경로를 유지한다.
- host/app 로그에는 아래 시점을 추가로 남기도록 보강했다.
  - `render_job_started`
  - `capture_preview_ready elapsed_ms=...`
  - `current-session-preview-visible`

자동 검증:

- `cargo test darktable_cli_resolution --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test preview_invocation --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test final_invocation --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `pnpm vitest run src/booth-shell/components/SessionPreviewImage.test.tsx src/booth-shell/screens/CaptureScreen.test.tsx src/session-domain/selectors/current-session-previews.test.ts`
- 모두 통과했다.

중요한 수동 측정:

- 같은 RAW/XMP를 실제 설치된 darktable CLI로 직접 측정한 결과,
  - full-size preview render: 약 `8652ms`
  - `1280x1280` low-res preview render: 약 `5973ms`
  - `640x640` low-res preview render: 약 `6894ms`
  수준이었다.

결론:

- preview를 저해상도로 낮추는 것은 분명 개선이지만,
  이 장비에서는 darktable 기반 RAW render 자체가 여전히 `5초+`여서
  **2초 이내 booth preview 목표를 단독으로 닫지 못할 가능성이 높다.**
- 따라서 현재 병목은 "UI 반영만 느리다"보다, darktable RAW render cost 자체가 구조적 제한일 수 있다는 쪽으로 기울었다.

### 2026-04-01 01:43 +09:00 사용자 실기기 최종 확인: 현재 상태를 최신 정상 기준선으로 고정

사용자 최종 확인 결과:

- 현재 부스에서 카메라 촬영, `Preview Waiting`, 현재 세션 사진 반영까지 전체 흐름이 문제없이 동작했다.
- 사용자는 현 상태를 "정상 동작"으로 확인했다.

이번 시점의 제품 판단:

- 현재 워크스페이스 상태는 실기기 기준으로 사용 가능한 최신 정상 상태다.
- 이후 회귀 분석 시 이 시점을 **latest known-good baseline**으로 취급한다.
- 다음 변경은 이 기준선 대비
  - 촬영 직후 `Phone Required` 재발 여부
  - preview/current session 반영 여부
  - 최근 확인된 booth 체감 동작
  를 비교해야 한다.

### 2026-04-01 18:40 +09:00 추가 재발: 1, 2번째 샷 뒤 3번째 샷이 `capture-download-timeout`으로 다시 `Phone Required`에 잠겼다

사용자 최신 제보:

1. 1번째 샷은 정상 저장됐다.
2. 2번째 샷도 문제 없이 반영됐다.
3. 3번째 샷에서 다시 `Phone Required`가 발생했다.

이번 회차에서 실제 최신 세션 evidence를 다시 맞춰 보니,
이번 재발은 render 회귀가 아니라 **follow-up capture boundary가 helper timeout 안에서 닫히지 못한 문제**로 보는 것이 맞았다.

실제 확인 근거:

- 실패 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a21cfdc4106a64` 였다.
- `diagnostics/camera-helper-events.jsonl`에는 아래가 남아 있었다.
  - 첫 번째 요청: `capture-accepted -> file-arrived`
  - 두 번째 요청: `capture-accepted -> file-arrived`
  - 세 번째 요청 `request_000000000018a21d06313e1b80`: `capture-accepted` 뒤
    `2026-04-01T03:12:11Z`에 `recovery-status(detailCode=capture-download-timeout)`와
    `helper-error(detailCode=capture-download-timeout)`가 기록됐다.
- 같은 세션의 `camera-helper-status.json`은 이후 다시
  `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`로 회복했다.
- 반면 `session.json`은
  - `lifecycle.stage = phone-required`
  - `captures`가 2장만 존재
  로 남아 있었고, 세 번째 샷은 세션 결과에 들어오지 못했다.
- `diagnostics/timing-events.log`에는 첫 두 장의 `render-ready`만 있었고,
  세 번째 샷의 render failure 흔적은 없었다.

정리:

- 이번 `Phone Required`는 preview render 실패가 아니라,
  **helper가 세 번째 촬영의 RAW handoff 완료를 timeout 안에 닫지 못해 host가 세션을 보호 상태로 잠근 케이스**였다.
- 특히 helper의 capture completion 경계 안에 촬영 직후 preview 보강 작업이 일부 함께 들어가 있었기 때문에,
  연속 촬영에서는 실제 RAW 저장보다 completion 종료가 더 늦어질 수 있었다.
- 기존 기본 budget인 `helper 15초 / host 20초`는 이 장비의 follow-up capture에 다시 부족하다고 판단했다.

이번 회차 수정:

- Canon helper의 기본 capture completion timeout을 `15초 -> 30초`로 늘렸다.
- host의 capture round-trip timeout도 `20초 -> 35초`로 맞춰,
  helper보다 host가 먼저 세션을 `Phone Required`로 잠그지 않게 했다.
- active capture 중에는 helper의 preview backfill이 끼어들지 않게 막아,
  live capture 경계에서 SDK 경합 가능성을 줄였다.
- 촬영 완료 경계에서는 on-camera thumbnail fast path만 유지하고,
  더 무거운 RAW 기반 preview backfill은 촬영 후 일반 helper loop에서 보강하도록 분리했다.

검증:

- `dotnet build sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness`
- 모두 통과했다.

다음 실기기 확인 기준:

1. 같은 조건으로 최소 5장 이상 연속 촬영해 `3번째 샷` 이후에도 `Phone Required`가 다시 뜨는지 본다.
2. 재발 시에는 같은 세션의 `camera-helper-events.jsonl`, `camera-helper-status.json`, `session.json`을 먼저 한 세트로 확인한다.

### 2026-04-01 18:42 +09:00 사용자 후속 확인: 방금 수정한 동일 증상은 더 이상 재현되지 않았다

사용자 후속 확인 결과:

- 방금까지 재현되던 "1, 2번째 샷은 성공하지만 3번째 샷에서 `Phone Required`" 패턴이
  더 이상 나타나지 않았다.

이번 시점의 판단:

- 직전 회차에서 보정한 follow-up capture timeout / helper completion boundary 조정이
  현재 실기기 조건에서는 유효하게 작동한 것으로 본다.
- 따라서 최신 워크스페이스 상태는
  **연속 촬영 중 3번째 샷 `capture-download-timeout` 재발이 없는 최신 정상 기준선**으로 기록한다.

운영 메모:

1. 이후 다시 같은 패턴이 나오기 전까지는 `2026-04-01 18:40 +09:00` 항목의 원인/수정 조합을 현행 정상 해법으로 본다.
2. 다음 회귀가 생기면 새 session evidence를 다시 분리해서, 같은 root cause의 재발인지 다른 경계의 새 실패인지부터 구분한다.

### 2026-04-14 15:45 +09:00 하드웨어 재검증 후속: dedicated renderer 성공 경로에서 speculative close가 중복 경쟁하며 close 시간이 늘어났다

사용자 최신 제보:

- 앱 실행 후 하드웨어 검증을 다시 했지만, preview 교체가 여전히 느리다고 보고했다.

이번 회차에서 읽은 최신 증거:

- 최신 앱 로그와 session evidence는 `session_000000000018a62558931dd920`를 기준으로 확인했다.
- `Boothy.log`에는 마지막 캡처 `capture_20260414063000251_cffab78a95`에 대해
  - `speculative_preview_render_started ... sourceAsset=fast-preview-raster`
  - `capture_preview_transition_summary ... lane_owner=dedicated-renderer fallback_reason=none first_visible_ms=2819 replacement_ms=5533`
  가 남아 있었다.
- 같은 session의 `diagnostics/timing-events.log`에서도 모든 캡처가
  - `fast-preview-visible`
  - `preview-render-ready ... binary=dedicated-renderer`
  순으로 닫혔고,
  마지막 캡처는 `replacementMs=5533`, `originalVisibleToPresetAppliedVisibleMs=2714`였다.
- 즉 helper fast preview는 이미 살아 있고 dedicated renderer도 실제로 accepted 되었지만,
  close render 자체가 여전히 수초 단위로 남아 있었다.

원인 판단:

- 이번 느림은 helper 썸네일 handoff 누락이 아니라,
  **dedicated renderer canary 성공 경로에서 host의 speculative close render가 같은 capture에 대해 동시에 돌며 렌더 경쟁을 만든 문제**로 보는 것이 가장 타당했다.
- 현재 ingest 단계는 fast preview가 보이자마자 host resident speculative render를 시작한다.
- 동시에 canary에서는 dedicated renderer sidecar도 same-capture truthful close를 소유한다.
- 최신 로그에는 `speculative_preview_render_started`와 dedicated renderer accepted 결과가 같은 capture에 함께 남아 있어,
  성공 경로에서 close render가 사실상 이중 실행되고 있음을 확인했다.

이번 회차 수정:

- active preset이 `local-renderer-sidecar` route에 있고,
  route stage가 `canary/default`,
  그리고 manifest의 active warm state가 같은 preset/version에 대해 `warm-ready` 또는 `warm-hit`인 경우에는
  ingest 단계의 speculative close render를 시작하지 않도록 보정했다.
- 즉 first-visible 원본/thumbnail은 그대로 즉시 보여주되,
  dedicated renderer가 이미 준비된 happy path에서는 same-capture close render를 한 lane만 돌게 했다.
- 세션 timing 로그에는 `speculative-preview-skipped` 이벤트가 남도록 해서,
  다음 하드웨어 검증 시 이 전략이 실제로 적용됐는지 바로 추적할 수 있게 했다.
- dedicated renderer의 개선 요약 버전은 `2026-04-14g`로 올렸고,
  `skipSpeculativeCloseWhenDedicatedRouteWarm=true`를 매번 evidence에 남기도록 했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::ingest_pipeline::tests::skips_speculative_preview_when_dedicated_route_is_warm_and_active -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::ingest_pipeline::tests::keeps_speculative_preview_when_warm_state_is_for_another_preset -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib render::dedicated_renderer::tests::preview_optimization_summary_reports_active_strategy -- --exact --nocapture`
- `pnpm vitest run src/shared-contracts/contracts.test.ts`
- `cargo build --manifest-path src-tauri/Cargo.toml`
- 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 연속 촬영 2~3장 이상 진행한다.
2. 새 session의 `Boothy.log`와 `diagnostics/timing-events.log`에서
   - `speculative-preview-skipped`
   - `capture_preview_transition_summary`
   - `capture_preview_optimization_summary`
   를 확인한다.
3. 목표는
   - dedicated renderer accepted 유지
   - `replacementMs` 하락
   - 특히 이전 `5533ms` outlier가 재발하지 않는지
   를 비교하는 것이다.

### 2026-04-14 16:58 +09:00 하드웨어 후속: speculative close 중복은 제거됐지만 dedicated close 자체가 아직 느려 close 해상도와 fast preview 중복 승격을 추가로 줄였다

사용자 최신 제보:

- 앱을 다시 실행해 하드웨어 검증했지만 여전히 느리다고 보고했다.
- 이번 회차부터는 어떤 개선을 넣었는지 로그 파일에도 계속 남겨 달라고 요청했다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a629892187d3a8`였다.
- `diagnostics/timing-events.log`에는 모든 capture에 대해
  - `speculative-preview-skipped`
  - `capture_preview_optimization_summary detail=strategyVersion=2026-04-14g;...;skipSpeculativeCloseWhenDedicatedRouteWarm=true`
  가 남아 있었다.
- 즉 직전 수정으로 speculative close 중복 경쟁은 실제 실행본에 반영됐고, 더 이상 병목의 주원인이 아니었다.
- 하지만 같은 세션의 3건은 여전히
  - `replacementMs=4411`
  - `replacementMs=4455`
  - `replacementMs=3494`
  수준으로 남았고,
  모두 `laneOwner=dedicated-renderer`, `fallbackReason=none`, `warmState=warm-hit`였다.

원인 판단:

- 이제 남은 병목은 helper handoff나 speculative 경쟁이 아니라,
  **dedicated renderer가 same-capture close preview를 만드는 자체 비용**으로 좁혀졌다.
- 현재 구조는 이미 fast preview raster를 소스로 쓰고 있고 happy path도 dedicated renderer 한 lane만 남겼다.
- 따라서 같은 제품 계약을 유지한 상태에서 더 줄일 수 있는 다음 수단은
  1. close preview 렌더 크기를 더 낮추는 것
  2. early fast preview 승격을 두 번 반복하지 않게 정리하는 것
  으로 판단했다.

이번 회차 수정:

- dedicated renderer와 host preview renderer의 display-sized close cap을 더 낮췄다.
  - fast preview 기반 close: `192x192 -> 128x128`
  - raw fallback close: `320x320 -> 256x256`
- capture round-trip 중 `FastPreviewReady`로 이미 같은 preview를 early 승격한 경우에는
  persist 단계에서 같은 자산을 다시 승격하지 않도록 정리했다.
  - 목적은 first-visible 시점 흔들림과 중복 timing event를 줄이는 것이다.
- 다음 실행부터 `capture_preview_optimization_summary`와 evidence에는
  `strategyVersion=2026-04-14h`
  `dedupeEarlyFastPreviewPromotion=true`
  `fastPreviewCapPx=128x128`
  `rawPreviewCapPx=256x256`
  가 기록된다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib render::dedicated_renderer::tests::preview_optimization_summary_reports_active_strategy -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::ingest_pipeline::tests::skips_speculative_preview_when_dedicated_route_is_warm_and_active -- --exact --nocapture`
- `pnpm vitest run src/shared-contracts/contracts.test.ts`
- `cargo build --manifest-path src-tauri/Cargo.toml`
- `rustc --edition=2021 --test sidecar/dedicated-renderer/main.rs -o target\\dedicated_renderer_tests.exe` 후 `target\\dedicated_renderer_tests.exe`
- 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 룩으로 2~3장 연속 촬영한다.
2. 새 로그에서
   - `capture_preview_optimization_summary`
   - `capture_preview_transition_summary`
   - 필요 시 `fast-preview-visible`
   를 확인한다.
3. 이번 비교 목표는
   - `strategyVersion=2026-04-14h`가 실제로 남는지
   - `replacementMs`가 기존 `3494~4455ms`보다 추가로 내려가는지
   - fast preview 관련 중복 이벤트가 줄었는지
   를 확인하는 것이다.

### 2026-04-16 01:29 +09:00 로그 후속: preset 자체는 제출됐지만 preview 완료와 warm-state 동기화가 같은 session manifest를 교차 갱신하며 세션 truth가 사라질 수 있었다

사용자 최신 제보:

- 프리셋이 적용되지 않았고, 세 컷을 찍은 뒤 앱이 초기 화면으로 튕겼다고 보고했다.

이번 회차에서 읽은 최신 증거:

- 최신 실패 세션은 `session_000000000018a6941e2dcd50e8`였다.
- 같은 세션의 `diagnostics/camera-helper-events.jsonl`에는 3번 모두
  - `capture-accepted`
  - `file-arrived`
  - `fast-preview-ready`
  가 순서대로 남아 있었다.
- `diagnostics/camera-helper-status.json`의 최종 상태도
  - `cameraState=ready`
  - `helperState=healthy`
  - `detailCode=camera-ready`
  였다.
- dedicated renderer diagnostics의 각 `*.preview-result.json`도 3건 모두 `status=accepted`였다.
- 반면 세션 루트에는 **`session.json`이 아예 남아 있지 않았고**,
  raw/originals와 canonical preview 파일들만 존재했다.
- 같은 세션의 `diagnostics/timing-events.log`에는
  두 capture의 `preview-render-ready`가 한 줄에 겹쳐 기록된 흔적이 있어,
  동일 시점의 파일 append/write 경합도 함께 확인됐다.

원인 판단:

- 이번 실패는 helper capture나 preset publication 누락보다,
  **preview 완료 경로와 dedicated renderer warm-state 동기화 경로가 같은 `session.json`을 동시에 교체(write-swap)하려 하며 세션 truth를 잃은 문제**로 보는 것이 가장 타당했다.
- 실제 증거상 카메라 촬영과 dedicated renderer 제출은 모두 성공했기 때문에,
  고객이 본 "프리셋이 적용되지 않음" 체감은 preset 계산 실패보다
  **세션 manifest 소실로 booth가 현재 session/capture truth를 복구하지 못한 결과**에 가깝다.
- 특히 second/third capture close가 비슷한 시점에 닫히는 동안,
  `finish_preview_render_in_dir(...)`는 capture pipeline lock 아래에서 manifest를 갱신하지만
  `sync_active_preview_warm_state_in_manifest(...)`는 별도 락 없이 같은 manifest를 다시 읽고 써서
  atomic swap용 `session.json.tmp` / `session.json.bak`를 서로 덮을 수 있었다.

이번 회차 수정:

- session repository에 **manifest 경로별 write 직렬화 락**을 추가해,
  같은 `session.json`에 대한 temp/backup swap이 동시에 실행되지 않게 했다.
- dedicated renderer의 active warm-state manifest 동기화도
  **capture pipeline lock 아래에서 읽기/쓰기** 하도록 맞춰,
  preview close와 warm-state snapshot 갱신이 같은 capture 흐름에서 교차 덮어쓰지 않게 했다.
- 이 회차의 목적은 preview 속도 조정이 아니라,
  **세 컷 이후에도 현재 세션 truth가 사라지지 않고 유지되게 만드는 것**이다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib session::session_repository::tests::concurrent_manifest_writes_keep_the_manifest_present_and_readable -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_recovers_from_a_manifest_backup_left_during_an_atomic_swap_gap -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test dedicated_renderer -- --nocapture`
- 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 3장, 가능하면 5장까지 연속 촬영한다.
2. 이번에는
   - 촬영 후 booth가 초기 화면으로 튀지 않는지
   - 각 컷이 최근 결과에 계속 남는지
   - preset-applied preview가 늦더라도 세션 자체는 유지되는지
   를 본다.
3. 재발 시에는 같은 세션의 아래 파일을 한 세트로 가져온다.
   - `session.json`
   - `diagnostics/timing-events.log`
   - `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`
   - 가능하면 booth 화면 사진 또는 짧은 영상

### 2026-04-16 01:30 +09:00 하드웨어 후속: 세션 소실은 막혔지만 첫 컷 preset close가 다시 느렸고, 두 번째 컷은 helper handoff budget을 다시 넘겨 timeout으로 떨어졌다

사용자 최신 제보:

- 프리셋 적용 후 두 번째 샷에서 멈췄고,
- 프리셋 적용 시간도 오래 걸렸다고 보고했다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a694ccc7295300`였다.
- 이번에는 직전 수정 덕분에 `session.json`은 정상적으로 남아 있었고,
  세션이 초기 화면으로 튀는 manifest 소실 회귀는 재발하지 않았다.
- 다만 같은 세션의 `diagnostics/timing-events.log`에는 첫 번째 캡처
  - `fast-preview-visible`
  - `preview-render-ready`
  - `capture_preview_transition_summary ... warmState=warm-hit ... firstVisibleMs=3034 ... replacementMs=7142`
  가 남아 있어,
  **preset-applied close가 warm-hit 조건에서도 다시 7초대까지 늘어난 것**을 확인했다.
- 이어서 같은 세션의 두 번째 요청 `request_000000000000064f8241ded818`은
  `camera-helper-events.jsonl`에 `capture-accepted`까지만 남았고,
  `2026-04-15T16:32:42Z`에
  - `recovery-status(detailCode=capture-download-timeout)`
  - `helper-error(detailCode=capture-download-timeout)`
  로 닫혔다.
- 최종 `camera-helper-status.json`은 다시 `camera-ready`로 회복했지만,
  세션에는 첫 번째 캡처만 남아 있었다.

원인 판단:

- 이번 회차의 직접 실패는 다시
  **follow-up capture의 RAW handoff가 helper 기본 completion budget 안에 닫히지 못한 문제**로 보는 것이 맞았다.
- 최신 로그상 두 번째 샷은 아예 request 소비 실패가 아니라 `capture-accepted`까지는 성공했으므로,
  stale request나 세션 소실보다 **helper download completion 경계**가 다시 병목이다.
- 동시에 첫 번째 샷의 `replacementMs=7142`는 현재 close preview 비용이 여전히 높다는 뜻이므로,
  사용자가 "프리셋 적용이 느리다"라고 느낀 불만도 이번 증거와 일치한다.

이번 회차 수정:

- helper 기본 capture completion timeout을 `30초 -> 45초`로 늘렸다.
- host capture round-trip timeout도 `35초 -> 50초`로 맞춰,
  helper보다 host가 먼저 같은 follow-up capture를 실패로 잠그지 않게 했다.
- dedicated renderer / host preview close cap을 한 단계 더 낮췄다.
  - fast preview 기반 close: `128x128 -> 96x96`
  - raw fallback close: `256x256 -> 224x224`
- 다음 evidence summary에는
  `strategyVersion=2026-04-16a`
  `fastPreviewCapPx=96x96`
  `rawPreviewCapPx=224x224`
  가 남는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::sidecar_client::tests::capture_round_trip_timeout_uses_the_latest_default_budget_when_unset -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_capture_download_timeout_recovers -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test dedicated_renderer -- --nocapture`
- `cargo build --manifest-path src-tauri/Cargo.toml`
- 위 Rust 검증은 모두 통과했다.
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`는
  현재 워크스페이스에 Canon SDK source가 없어
  `Canon SDK source not found`로 실행하지 못했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 2장, 가능하면 4~5장까지 연속 촬영한다.
2. 이번에는
   - 두 번째 샷이 다시 멈추지 않는지
   - 첫 번째 샷 preset-applied close가 이전 `replacementMs=7142`보다 내려가는지
   - 촬영 후 세션이 계속 유지되는지
   를 함께 본다.
3. 재발 시에는 같은 세션의 아래 파일을 가져온다.
   - `session.json`
   - `diagnostics/camera-helper-events.jsonl`
   - `diagnostics/timing-events.log`
   - `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`

### 2026-04-16 02:05 +09:00 하드웨어 후속: 최신 세션은 더 이상 멈추지 않았고, 남은 체감 지연 중 하나는 early fast preview를 persist 단계에서 다시 canonical scan 하며 중복 승격하던 경로였다

사용자 최신 제보:

- 로그를 확인해서 더 단축시켜 달라고 요청했다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a69568f2f4db0c`였다.
- 이 세션의 `session.json`은 정상적으로 남아 있었고, 세 컷 모두 session capture에 기록돼 있었다.
- `diagnostics/timing-events.log`에는 세 컷 모두
  - `fast-preview-visible`
  - `preview-render-ready`
  - `capture_preview_transition_summary ... warmState=warm-hit ... replacementMs=6540/6789/6551`
  가 남아 있어, 이번 회차의 직접 실패는 아니라는 점이 먼저 확인됐다.
- 같은 세션의 dedicated renderer request/result 파일 타임스탬프를 대조하면 request-to-result가 대략
  - `3487ms`
  - `3834ms`
  - `3576ms`
  로 닫혀 있었고, 현재 남은 문제는 preset close의 순수 latency로 좁혀졌다.
- 동시에 같은 세션의 각 capture에는
  - `fast-preview-promoted kind=windows-shell-thumbnail`
  직후
  - `fast-preview-promoted kind=legacy-canonical-scan`
  이 다시 남아 있었다.
- 즉 helper가 먼저 canonical preview까지 승격해 둔 capture에서도 persist 단계가 그 사실을 전달받지 못해,
  이미 보인 same-capture preview를 다시 canonical scan 하며 한 번 더 승격하려 하고 있었다.

원인 판단:

- 이번 회차에서 바로 줄일 수 있는 병목은
  **early fast preview promotion 결과를 persist 단계가 재사용하지 못해 같은 썸네일을 다시 canonical scan 하던 중복 경로**였다.
- 이 경로는 dedicated renderer의 3초대 close 자체를 없애지는 못하지만,
  첫 visible 이후 host가 같은 preview 자산을 한 번 더 확인/채택하는 불필요한 작업과 중복 timing noise를 만들고 있었다.
- 반대로 darktable/XMP 자체를 크게 바꾸는 쪽은 현재 증거만으로는 preset truth 리스크가 있어 이번 회차에서는 건드리지 않았다.

이번 회차 수정:

- helper의 `fast-preview-ready`가 먼저 canonical preview 승격을 끝낸 경우,
  persist 단계가 그 **이미 검증된 early preview 결과를 그대로 재사용**하도록 바꿨다.
- 이제 같은 request/capture에서는 `legacy-canonical-scan`으로 다시 canonical preview를 찾지 않는다.
- 목적은 첫 visible 직후의 중복 승격과 추가 디스크 스캔을 없애,
  **현재 구조를 유지한 채 체감 지연을 조금 더 깎는 것**이다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness fast_preview_updates_are_emitted_from_the_canonical_preview_path_before_capture_save_closes -- --exact --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test dedicated_renderer -- --nocapture`
- `cargo build --manifest-path src-tauri/Cargo.toml`
- 위 검증은 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 3장, 가능하면 5장까지 연속 촬영한다.
2. 이번에는
   - 첫 썸네일이 뜬 뒤 같은 컷이 한 번 더 흔들리거나 늦게 다시 붙는 느낌이 줄었는지
   - 전체 preset 적용 대기가 이전보다 조금이라도 짧아졌는지
   - 세션이 계속 유지되는지
   를 함께 본다.
3. 재발 또는 체감 차이가 작으면 같은 세션의 아래 파일을 가져온다.
   - `session.json`
   - `diagnostics/timing-events.log`
   - `diagnostics/dedicated-renderer/*.preview-request.json`
   - `diagnostics/dedicated-renderer/*.preview-result.json`

### 2026-04-16 02:18 +09:00 하드웨어 후속: early preview 중복은 사라졌지만 dedicated renderer close는 여전히 darktable preview run 자체가 병목이었고, local bench 기준 `memory library + opencl off` 조합만 의미 있게 줄었다

사용자 최신 제보:

- 로그를 다시 확인해서 더 단축시켜 달라고 요청했다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a6963a96799cb4`였다.
- 이 세션은 5컷 연속 촬영이 모두 성공했고, `session.json`도 정상적으로 유지됐다.
- 이번에는 직전 수정이 실제 반영되어 `diagnostics/timing-events.log`에서
  **`legacy-canonical-scan`이 더 이상 남지 않았다.**
- 그러나 같은 세션의 5건은 여전히
  - `replacementMs=7165`
  - `6634`
  - `6477`
  - `6897`
  - `7327`
  로 남아 있었고,
  전부 `laneOwner=dedicated-renderer`, `fallbackReason=none`, `warmState=warm-hit`였다.
- dedicated renderer request/result 파일 타임스탬프를 대조하면 request-to-result가 대략
  - `4031ms`
  - `3728ms`
  - `3735ms`
  - `3984ms`
  - `4543ms`
  로 닫혀 있었다.
- 즉 이번 회차의 남은 병목은 helper나 host 중복 승격이 아니라,
  **dedicated renderer가 darktable-cli로 preset close를 만드는 자체 시간**이었다.

추가 원인 확인:

- sidecar `warmup-v1` 구현을 다시 읽어 보니 현재 warm-up은 실제 렌더러/CLI를 데우지 않고,
  warm-state marker 파일만 `warm-ready`로 남기는 구조였다.
- 즉 로그의 `warm-hit`는 실제 darktable warm cache hit가 아니라
  **"같은 preset/session용 marker가 있었다"**는 뜻에 가까웠다.
- 실제 장비와 같은 입력 자산으로 local bench를 다시 돌린 결과,
  기본 preview invocation 대비
  **`--library :memory:` + `--disable-opencl` 조합만**
  출력 차이 없이 유의미하게 시간을 줄였다.
  - baseline: 약 `5.2s`
  - optimized: 약 `4.4s`
  - 출력 크기는 동일했고, 비교상 채널 평균 차이는 거의 없었다.

이번 회차 수정:

- dedicated renderer preview close invocation에
  - `--core --library :memory:`
  - `--disable-opencl`
  을 추가했다.
- 목적은 preview close가 매번 디스크 library 상태와 opencl 초기화 비용을 끌고 가지 않게 해,
  **same-capture preset-applied close 자체 시간을 줄이는 것**이다.
- 다음 evidence summary에는
  `strategyVersion=2026-04-16b`
  `previewCliLibrary=memory`
  `previewCliDisableOpencl=true`
  가 남는다.

검증:

- `rustc --edition=2021 --test sidecar/dedicated-renderer/main.rs -o target\\dedicated_renderer_sidecar_tests.exe`
- `target\\dedicated_renderer_sidecar_tests.exe`
- `cargo test --manifest-path src-tauri/Cargo.toml --test dedicated_renderer -- --nocapture`
- `cargo build --manifest-path src-tauri/Cargo.toml`
- 위 검증은 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 3장, 가능하면 5장까지 연속 촬영한다.
2. 이번에는
   - preset-applied close가 이전 `replacementMs=6477~7327`보다 내려가는지
   - 연속 촬영 중 멈춤 없이 계속 유지되는지
   - 첫 썸네일은 그대로 빠르게 뜨면서, preset-applied 교체가 이전보다 덜 늦게 붙는지
   를 함께 본다.
3. 재발 시에는 같은 세션의 아래 파일을 가져온다.
   - `session.json`
   - `diagnostics/timing-events.log`
   - `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`
   - `diagnostics/dedicated-renderer/*.preview-request.json`
   - `diagnostics/dedicated-renderer/*.preview-result.json`

### 2026-04-16 02:14 +09:00 하드웨어 후속: 최신 최적화는 반영됐지만 현행 96x64급 truthful preview가 너무 부드러워졌고, 이번 회차는 화질을 한 단계 복원하면서 renderer startup 최적화는 유지했다

사용자 최신 제보:

- 로그를 다시 확인해서 더 단축시켜 달라고 요청했다.
- 단, 시간 단축을 위해 해상도를 더 낮추지는 말아 달라고 했고, 현재 결과는 너무 깨져서 사용자가 받아들이기 어렵다고 보고했다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a696dfd1fb2a64`였다.
- 이 세션은 5컷 연속 촬영이 모두 성공했고, 세션 유지와 follow-up capture 멈춤 회귀는 재발하지 않았다.
- `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`에는 직전 전략이 실제 적용된
  `strategyVersion=2026-04-16b`
  가 남아 있었다.
- 같은 세션의 `diagnostics/timing-events.log`에는 각 capture가
  - `firstVisibleMs=3335 / 3058 / 2863 / 3095 / 2993`
  - `replacementMs=6787 / 6401 / 6207 / 6395 / 6908`
  로 남아 있어,
  직전 회차의 renderer startup 최적화는 일부 효과가 있었지만 아직 충분하지 않았다.
- 동시에 실제 session preview 파일은 현행 cap 영향으로 `96x64` 수준으로 생성되고 있었고,
  사용자가 말한 "너무 깨진다"는 체감과 일치했다.

원인 판단:

- 최신 병목은 여전히 dedicated renderer close 자체에 남아 있지만,
  이번 제보 기준으로는 **속도를 위해 preview 해상도를 과하게 낮춘 부작용도 이미 제품 문제로 드러난 상태**였다.
- 따라서 다음 개선은 해상도를 더 낮추는 대신,
  직전 회차에서 확보한
  `--library :memory:`
  `--disable-opencl`
  이점을 유지한 채 preview 크기를 한 단계 복원하는 것이 맞다고 판단했다.

이번 회차 수정:

- dedicated renderer truthful preview cap을 한 단계 복원했다.
  - fast preview 기반 close: `96x96 -> 128x128`
  - raw fallback close: `224x224 -> 256x256`
- host preview invocation도 booth-safe preview에서 `--disable-opencl`을 함께 사용하도록 맞춰,
  화질 복원으로 늘 수 있는 fallback 경로 비용을 줄이도록 했다.
- 다음 evidence summary에는
  `strategyVersion=2026-04-16c`
  `hostPreviewDisableOpencl=true`
  `fastPreviewCapPx=128x128`
  `rawPreviewCapPx=256x256`
  가 남는다.

검증:

- `rustc --edition=2021 --test sidecar/dedicated-renderer/main.rs -o target\\dedicated_renderer_sidecar_tests.exe`
- `target\\dedicated_renderer_sidecar_tests.exe`
- `cargo test --manifest-path src-tauri/Cargo.toml --test dedicated_renderer -- --nocapture`
- `cargo build --manifest-path src-tauri/Cargo.toml`
- 위 검증은 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 3장, 가능하면 5장까지 연속 촬영한다.
2. 이번에는
   - preview가 이전 96x64급보다 덜 깨져 보이는지
   - preset-applied close가 여전히 연속 촬영을 막지 않고 유지되는지
   - `strategyVersion=2026-04-16c`가 실제 세션 evidence에 남는지
   를 함께 본다.
3. 여전히 느리면 같은 세션의 아래 파일을 가져온다.
   - `session.json`
   - `diagnostics/timing-events.log`
   - `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`
   - `diagnostics/dedicated-renderer/*.preview-request.json`
   - `diagnostics/dedicated-renderer/*.preview-result.json`

### 2026-04-16 10:29 +09:00 하드웨어 후속: 이번엔 프리셋 렌더보다 capture helper가 새 요청을 늦게 먹은 쪽이 문제였고, best-effort preview backfill이 새 촬영 요청보다 앞서지 않게 순서를 조정했다

사용자 최신 제보:

- 로그를 확인해서 촬영이 안 된 문제를 고치고 기록해 달라고 요청했다.
- 이어서 다음 단계와, 현재 preview 아키텍처를 계속 유지해야 하는지도 물었다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a6b1aa829b32d0`였다.
- 이 세션 자체는 4컷 모두 결국 저장되었지만, 마지막 네 번째 요청 `request_000000000000064f89a6c96b50`가 체감상 "촬영이 안 된다"로 느껴질 정도로 크게 밀렸다.
- 같은 세션의 `diagnostics/timing-events.log`에는
  - `request-capture`가 `2026-04-16T01:21:27Z`
  - 실제 `file-arrived`와 `fast-preview-visible`이 `2026-04-16T01:21:57Z`
  - `capture_preview_ready elapsedMs=32305`
  로 남아 있었다.
- 반면 그 same capture의 `originalVisibleToPresetAppliedVisibleMs=3194`였기 때문에,
  이번 회차의 큰 지연은 preset close 렌더가 아니라
  **촬영 요청 이후 RAW handoff가 시작되기 전 구간**에 있었다.
- `camera-helper-events.jsonl`도 네 번째 요청에서 `capture-accepted` 뒤 실제 `file-arrived`/`fast-preview-ready`가 훨씬 뒤에 닫힌 흐름을 보였다.
- helper 런타임 코드를 다시 읽어 보니 메인 루프는 새 요청을 읽기 전에
  - `TryCompletePendingFastPreviewDownload()`
  - `TryBackfillPreviewAssets()`
  를 먼저 돌리고 있었다.
- 이 둘은 correctness 경계가 아니라 **best-effort preview 보완 작업**이며, 특히 `TryBackfillPreviewAssets()`는 missing preview를 만나면 raw preview 생성까지 동기적으로 수행한다.

원인 판단:

- 이번 회차의 가장 유력한 원인은
  **camera helper가 새 촬영 요청보다 optional preview maintenance를 먼저 처리하면서, 실제 shutter request consumption이 늦어진 구조**였다.
- 최신 세션처럼 preview 파일이 잠깐 비어 있거나 backfill 후보가 생기면 helper loop가 raw preview 보완 쪽으로 들어갈 수 있고,
  그동안 새 `request-capture`는 request log에 써져 있어도 helper가 늦게 읽게 된다.
- 즉 사용자가 본 "촬영이 안 된다" 체감은 이번 증거상
  preset render failure보다 **helper request prioritization 문제**로 보는 것이 가장 타당했다.

이번 회차 수정:

- canon helper 메인 루프에서 **새 capture request 처리 우선순위를 best-effort preview maintenance보다 앞**으로 옮겼다.
- 이제 pending request가 하나라도 있으면
  - `TryCompletePendingFastPreviewDownload()`
  - `TryBackfillPreviewAssets()`
  같은 optional preview 보완 작업은 뒤로 미룬다.
- 목적은 새 셔터 요청이 optional preview 보완에 가로막혀 수초~수십초 늦게 소비되는 경계를 없애는 것이다.

검증:

- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter CanonHelperServiceTests`
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter JsonFileProtocolTests`
- 두 검증 모두 현재 워크스페이스에 **Canon SDK source (`EDSDK.cs`)가 없어 빌드 단계에서 실행하지 못했다.**
- 따라서 이번 회차는 코드 변경과 최신 하드웨어 증거 정합성까지는 확인했지만, helper 자동 검증은 SDK source 복구 후 다시 닫아야 한다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 3장, 가능하면 5장까지 연속 촬영한다.
2. 이번에는 특히
   - 버튼을 누른 뒤 다음 capture의 `fast-preview-visible`이 이번처럼 20~30초씩 밀리지 않는지
   - 세션이 계속 유지되는지
   - `request-capture -> file-arrived` 간격이 이전 마지막 컷보다 짧아졌는지
   를 본다.
3. 재발 시에는 같은 세션의 아래 파일을 가져온다.
   - `session.json`
   - `diagnostics/timing-events.log`
   - `diagnostics/camera-helper-events.jsonl`
   - `diagnostics/camera-helper-status.json`

### 2026-04-16 14:40 +09:00 하드웨어 후속: 이번에는 preview가 128x85 수준으로 너무 작게 나와 화질이 무너졌고, 해상도 캡을 올려 화질과 속도의 균형을 다시 맞췄다

사용자 최신 제보:

- 로그를 다시 확인해서 더 단축해 달라고 요청했다.
- 필터 적용 후 결과 해상도가 너무 깨져서 원본과 비슷하게 보이도록 고쳐 달라고 요청했다.

이번 회차에서 읽은 최신 증거:

- 최신 세션은 `session_000000000018a6bfb7f2a5af2c`였다.
- 이 세션의 dedicated renderer 결과는 `replacementMs=11762`와 `6443`으로 남았고, 첫 컷은 특히 느렸다.
- 같은 세션의 실제 preview 파일은 `128x85`였다.
- `preview-promotion-evidence.jsonl`에는 `sourceAssetPath=raw-original`과 `previewSourceAssetPath=<same session preview>`가 함께 남아 있어, 작은 preview가 다음 렌더의 재료로도 재사용될 수 있는 구조였다.
- 로컬 `darktable-cli` 벤치에서는 `128`, `256`, `384`, `512`, `768`, `1024` 캡이 모두 비슷한 처리 시간을 보여, 화질 상향 여지가 확인됐다.

이번 회차 수정:

- dedicated renderer preview cap을 상향했다.
  - fast preview 기반 close: `768x768`
  - raw fallback close: `1024x1024`
- preview 품질이 너무 낮게 남지 않도록, 결과가 화면에서 더 원본에 가깝게 보이도록 조정했다.
- 다음 evidence summary에는
  `strategyVersion=2026-04-16d`
  `fastPreviewCapPx=768x768`
  `rawPreviewCapPx=1024x1024`
  가 남는다.

검증:

- `rustc --edition=2021 --test sidecar/dedicated-renderer/main.rs -o target\dedicated_renderer_sidecar_tests.exe`
- `cargo build --manifest-path src-tauri/Cargo.toml --target-dir target-codex-verify`
- `cargo test --manifest-path src-tauri/Cargo.toml --target-dir target-codex-verify --test dedicated_renderer -- --nocapture`
- 위 검증은 모두 통과했다.

다음 실기기 확인 기준:

1. 앱을 다시 실행한 뒤 같은 preset으로 최소 3장, 가능하면 5장까지 연속 촬영한다.
2. 이번에는
   - preview가 이전 128x85 수준보다 충분히 선명해졌는지
   - preset-applied close가 더 느려지지 않았는지
   - `strategyVersion=2026-04-16d`가 실제 세션 evidence에 남는지
   를 함께 본다.
3. 여전히 느리거나 깨져 보이면 같은 세션의 아래 파일을 가져온다.
   - `session.json`
   - `diagnostics/timing-events.log`
   - `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`
   - `diagnostics/dedicated-renderer/*.preview-request.json`
   - `diagnostics/dedicated-renderer/*.preview-result.json`
