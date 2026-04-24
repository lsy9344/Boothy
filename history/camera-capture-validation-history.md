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

### 2026-04-19 03:18 +09:00 최신 다중 촬영 로그 재검증: 촬영 성공은 유지됐고, 남은 개선 포인트는 legacy canonical fast preview의 불필요한 speculative lane였다

사용자 최신 제보:

1. 앱을 다시 실행해 같은 세션에서 여러 장을 연속 촬영했다.
2. 이번에는 `Phone Required`로 잠기지 않았고, 로그 확인과 검증 기록 업데이트를 요청했다.

실제 확인 근거:

- 최신 앱 로그는 `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`였고,
  최신 실기기 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a784657d261d40`였다.
- 이 세션의 `session.json` 기준으로
  - `lifecycle.stage = capture-ready`
  - `captures`가 4장 모두 저장됨
  - 4장 모두 `renderStatus = previewReady`
  였다.
- 같은 세션의 `camera-helper-events.jsonl`에는 네 요청 모두
  - `capture-accepted`
  - `file-arrived`
  - `fast-preview-ready`
  까지 정상으로 닫혀 있었다.
- 즉 이번 최신 실기기 재현에서는
  **다중 촬영 성공 여부 자체는 정상**으로 봐도 된다.

남아 있던 이상 징후:

- 각 촬영의 `capture_preview_ready`는 계속 `previewBudgetState=exceededBudget`였다.
  - 1장: `elapsedMs=8340`
  - 2장: `elapsedMs=7313`
  - 3장: `elapsedMs=6984`
  - 4장: `elapsedMs=7946`
- 동시에 앱 로그에는 각 촬영마다 아래 경고가 반복됐다.
  - `resident_first_visible_render_failed ... reason_code=render-output-missing`
- 실제 preview worker stderr 로그(`.boothy-darktable/preview/logs/preview-stderr-1776534155884956900.log`)에는
  `libpng warning: IDAT: Extra compressed data`, `libpng error: Not enough image data`가 남아 있었다.

이번 회차 해석:

- 최신 실기기 기준 문제는 더 이상 `촬영 실패`나 `Phone Required` 재발이 아니다.
- 실제 남은 문제는
  **helper handoff 메타데이터 없이 canonical preview로 늦게 올라온 fast preview(`legacy-canonical-scan`)까지
  resident first-visible speculative render lane에 태우고 있었다는 점**이었다.
- 이 경로는 customer correctness에는 직접 필요하지 않은데,
  실기기에서는 출력이 닫히지 않거나 실패 로그만 남기고,
  capture close에서 불필요한 대기와 잡음을 추가할 수 있었다.

이번 회차 수정:

- capture ingest에서 fast preview 승격 결과에 `kind`를 함께 보존하도록 보강했다.
- `legacy-canonical-scan`으로 올라온 fast preview는
  - `persist_capture_in_dir(...)`
  - `complete_preview_render_in_dir(...)`
  두 경계 모두에서 resident speculative first-visible render를 시작하지 않게 바꿨다.
- 따라서 canonical same-capture preview가 늦게 보이는 경우에도
  - current/latest rail의 빠른 썸네일은 그대로 유지하고
  - 불필요한 speculative worker/lock/source staging 없이
  다음 truthful preview close로 바로 넘어가게 정리했다.

검증:

- 실기기 로그 확인:
  - 최신 세션 `session_000000000018a784657d261d40`
  - 4연속 촬영 성공, `capture-ready` 유지 확인
- 회귀 테스트:
  - `cargo test --manifest-path src-tauri/Cargo.toml complete_preview_render_reuses_a_late_same_capture_preview_before_raw_fallback -- --exact`
  - 통과
- 관련 스위트:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness`
  - 기본 병렬 실행에서는 일부 테스트가 `session-persistence-failed`로 흔들렸다.
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
  - 직렬 실행에서는 `65 passed; 0 failed`

현재 판단:

- `Phone Required` 재발 없이 여러 장 촬영이 닫히는 최신 제품 기준은 유지됐다.
- 이번 코드 변경은 다중 촬영 correctness를 바꾸기보다,
  **legacy canonical fast preview가 남기던 불필요한 speculative worker 흔적과 실패 잡음을 줄이는 안정화 개선**으로 기록한다.

### 2026-04-19 03:20 +09:00 최신 다중 촬영 로그 재확인: 일부 사진만 프리셋 체감이 달랐던 원인은 truthful preview close의 source 불일치였다

사용자 최신 제보:

1. 앱을 다시 실행해 여러 장을 다시 촬영했다.
2. `Phone Required`는 없었지만, 일부 사진은 프리셋 적용이 덜 된 것처럼 보였다고 제보했다.

실제 확인 근거:

- 최신 실기기 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a78554f6baf6cc`였다.
- 이 세션의 `session.json` 기준으로
  - `lifecycle.stage = capture-ready`
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - 총 6장 모두 `renderStatus = previewReady`
  였다.
- 즉 최신 실기기 기준으로는
  **다중 촬영 저장 자체는 정상**이었다.

이번에 새로 확인된 이상 징후:

- 같은 세션의 `diagnostics/timing-events.log`에서 6장의 `preview-render-ready`를 비교해 보니,
  - 5장은 `sourceAsset=fast-preview-raster`, `widthCap=256;heightCap=256`
  - 1장은 `sourceAsset=raw-original`, `widthCap=384;heightCap=384`
  로 닫히고 있었다.
- 같은 preset bundle(`preset_new-draft-2 / 2026.04.10 / look2`)을 썼는데도
  **truthful preview close의 render source가 shot마다 달랐다**는 뜻이다.
- 사용자가 본 "일부 사진만 프리셋 적용이 안 된 것처럼 보임"은
  이 불일치와 직접 맞닿아 있다고 판단했다.

이번 회차 해석:

- fast thumbnail/canonical preview JPEG는 current/latest rail의 즉시 가시성에는 유효하다.
- 하지만 이를 그대로 `previewReady`의 truth source로 확정하면,
  일부 shot은 RAW 기반 preset 결과가 아니라
  빠른 JPEG 기반 결과로 닫히게 된다.
- 즉 문제는 촬영 실패가 아니라,
  **첫 화면용 fast preview와 제품 기준의 최종 preview-ready를 같은 것으로 취급하던 점**이었다.

이번 회차 수정:

- `complete_preview_render_in_dir(...)`의 truthful preview close는
  fast thumbnail/speculative output이 있어도 `RawOriginal` 기준으로만 닫히게 바꿨다.
- 이미 끝난 speculative preview output은
  first-visible 자산 보강까지만 허용하고,
  더 이상 `preview-render-ready` / `capture_preview_ready`를 만족시키지 않게 정리했다.
- `camera-thumbnail` fast preview는 resident speculative render 시작 대상에서도 제외했다.
- 따라서 제품 기준으로는
  - 빠른 썸네일 노출은 유지하고
  - 닫힌 preview-ready는 shot마다 동일하게 RAW 기반 preset 결과로 맞추게 됐다.

검증:

- 실기기 로그 확인:
  - 최신 세션 `session_000000000018a78554f6baf6cc`
  - 6장 모두 저장 성공, `capture-ready` 유지 확인
  - `preview-render-ready` source가 `fast-preview-raster`와 `raw-original`로 섞여 있던 증거 확인
- 회귀 테스트:
  - `cargo test --manifest-path src-tauri/Cargo.toml preview_render_keeps_fast_first_visible_but_closes_truthfully_from_raw_after_handoff -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml complete_preview_render_ignores_a_finished_speculative_preview_until_raw_truth_is_ready -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml complete_preview_render_prefers_raw_truth_even_when_a_healthy_speculative_close_finishes_first -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml complete_preview_render_still_avoids_a_duplicate_render_while_speculative_close_is_active -- --exact`
  - 모두 통과
- 관련 스위트:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
  - `65 passed; 0 failed`

현재 판단:

- 최신 제품 기준에서 다중 촬영 안정성은 유지되고 있다.
- 이번 수정의 핵심은
  **사용자가 최종적으로 보게 되는 preview-ready를 fast thumbnail 기반 근사치가 아니라 RAW 기반 preset 결과로 통일했다는 점**이다.

### 2026-04-19 19:37 +09:00 최신 실기기 세션 기록: `look2`는 booth runtime 문제가 아니라 no-op preset artifact였다

사용자 최신 제보:

1. 앱을 다시 실행한 뒤 같은 세션에서 약 3회 촬영했다.
2. 로그를 확인하고 기록으로 남겨 달라고 요청했다.
3. 체감상 `preset_new-draft-2 / look2`가 적용되지 않는 것 같다고 제보했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7baa73c91f894`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - 총 3장 저장
  - 1장은 `renderStatus = previewWaiting`으로 남았고 `preview-render-failed`
  - 2장과 3장은 `renderStatus = previewReady`
  였다.
- 같은 세션의 `timing-events.log` 기준으로 2장과 3장은
  - `preview-render-ready`
  - `capture_preview_ready`
  - `recent-session-visible`
  까지 정상으로 닫혔다.
- 즉 이번 최신 세션 기준으로는
  **촬영 저장과 preview close 자체는 대체로 정상**이었다.

하지만 이번 회차에서 새로 확인된 핵심 증거:

- 같은 RAW에 대해 아래 세 가지를 직접 비교했다.
  - 세션에 실제 저장된 canonical preview
  - 동일 RAW + `preset_new-draft-2 / look2.xmp`
  - 동일 RAW + no-XMP baseline
- 픽셀 비교 결과는 세 경우 모두 차이가 `0`이었다.
  - 즉 `look2`는 로그상 `previewReady`가 찍혀도
    **고객이 실제로 보게 되는 booth preview에서는 기본 렌더와 구분되는 변화가 없었다.**
- 같은 방식으로 `preset_mono-pop` XMP를 비교하면 no-XMP 대비 픽셀 차이가 `2.298`로 확인됐다.
  - 따라서 이번 문제는 darktable/XMP 적용 엔진 전체가 죽은 것이 아니라,
    **현재 게시된 `preset_new-draft-2 / look2` artifact가 사실상 no-op**이라는 쪽으로 보는 것이 맞다.

이번 회차 수정:

- draft preset validation에 `render proof`를 추가했다.
- validation은 대표 preview 자산을 기준으로
  - no-XMP baseline render
  - XMP 적용 render
  를 각각 실행해 픽셀 차이를 비교한다.
- 두 결과가 동일하면 이제 `render-delta-missing`으로 실패한다.
- 따라서 `look2`처럼 booth customer가 보기에 기본 렌더와 같은 no-op preset은
  더 이상 booth-safe published artifact로 통과할 수 없다.

이번 시점의 제품 판단:

- 최신 세션 기준으로 capture path와 preview close는 유지됐다.
- 하지만 `preset_new-draft-2 / look2`는 현재 게시 artifact 자체가 no-op이므로,
  **운영 관점에서는 다시 authoring/export 후 재검증이 필요한 preset**으로 본다.
- 이번 코드 변경의 목적은 runtime을 바꾸는 것이 아니라,
  이런 no-op preset이 다시 부스 경로로 올라오는 일을 사전에 차단하는 것이다.

### 2026-04-19 20:08 +09:00 추가 확인: 최신 세션에서도 preset switch 자체는 정상이며, 체감상 "미적용"은 `look2` no-op 특성으로 재현됐다

추가 확인 배경:

1. 사용자가 "프리셋을 다른 걸로 바꿨는데도 적용이 안 되는 것 같다"고 다시 제보했다.
2. 가장 최근 세션과 실제 render proof를 다시 확인해,
   switch 미반영인지 preset artifact 문제인지 최종적으로 분리할 필요가 있었다.

최신 세션 기준 증거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7bc6ede49630c`였다.
- 같은 세션의 `session.json` 기준으로
  - 앞 3장은 `preset_test-look / 2026.03.31`
  - 뒤 2장은 `preset_new-draft-2 / 2026.04.10`
  으로 저장되었다.
- 즉 같은 세션 안에서도 booth runtime은
  **preset switch 자체를 무시하지 않았고, 이후 촬영부터 새 preset binding을 정상 반영했다.**
- 같은 세션의 `timing-events.log`에서도 뒤 2장 preview close는
  - `activePresetId=preset_new-draft-2`
  - `preview-render-ready`
  - `.../preset_new-draft-2/2026.04.10/xmp/look2.xmp`
  로 기록되었다.

render proof 재확인 결과:

- 같은 RAW(`capture_20260419105011545_bbf40b020a.CR2`)를 기준으로
  - booth session canonical preview
  - no-XMP baseline render
  - `look2.xmp` render
  - `test-look.xmp` render
  를 다시 비교했다.
- 비교 결과는 다음과 같았다.
  - session preview vs `look2`: `0.0`
  - baseline vs `look2`: `0.0`
  - baseline vs `test-look`: `1.130307`
- 따라서 최신 세션 기준으로도
  **"프리셋 전환이 안 됐다"기보다, `look2`가 booth preview에서 기본 렌더와 픽셀 수준으로 동일한 no-op preset처럼 동작했다**고 판단하는 것이 맞다.

이번 시점 결론:

- runtime preset switch 경로는 이번 최신 세션에서도 정상 동작했다.
- 고객 체감 문제의 직접 원인은 여전히 `preset_new-draft-2 / look2` artifact 자체였다.
- 이번 authoring validation 보강으로
  이런 no-op preset은 이후 draft validation에서 `render-delta-missing`으로 차단된다.
- 관련 회귀 검증:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test preset_authoring`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test operator_audit`
  - 모두 통과

### 2026-04-19 20:28 +09:00 최신 실기기 세션 로그 추가 기록: 다중 촬영 안정성은 유지됐고, 최종 preview close는 여전히 7초대 후반에서 8초대 초반이다

사용자 최신 요청:

1. 가장 최근 세션 로그를 다시 확인해 달라고 요청했다.
2. 몇 초대까지 나왔는지와 현재 진행 상태를 함께 기록해 달라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7be86a69cb3b8`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `activePresetDisplayName = look2`
  - 총 4장 모두 `renderStatus = previewReady`
  - 세션 종료 시점 `lifecycle.stage = capture-ready`
  였다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 최종 상태는
  - `cameraState = ready`
  - `helperState = healthy`
  로 닫혔다.
- 같은 세션의 `diagnostics/camera-helper-events.jsonl`에서는 4개 요청 모두
  - `capture-accepted`
  - `file-arrived`
  - `fast-preview-ready`
  까지 정상으로 이어졌다.
- 같은 세션의 `diagnostics/timing-events.log`에서 `capture_preview_ready`는 아래처럼 기록됐다.
  - 1장: `elapsedMs=8441`
  - 2장: `elapsedMs=7370`
  - 3장: `elapsedMs=7431`
  - 4장: `elapsedMs=7423`
- 따라서 이번 최신 세션 기준으로 최종 preview close는
  **대략 7.3초대에서 8.4초대까지** 확인됐다.
- 같은 로그의 `preview-render-ready` 자체는 아래 범위였다.
  - 1장: `elapsedMs=4566`
  - 2장: `elapsedMs=4312`
  - 3장: `elapsedMs=4436`
  - 4장: `elapsedMs=4331`
  즉 RAW 기반 preset render 자체는 **대략 4.3초대에서 4.6초대**였다.

이번 시점의 제품 판단:

- 최신 세션에서는 `Phone Required`, `capture-download-timeout`, `preview-render-failed` 재발 증거가 없었다.
- 즉 현재 제품 상태는 **촬영 correctness와 연속 촬영 안정성은 유지**되고 있다고 본다.
- 다만 모든 shot의 `capture_preview_ready`가 계속 `budgetState=exceededBudget`로 남아 있어,
  **고객 체감 응답성은 아직 목표 시간 안으로 들어오지 못한 상태**다.
- 또한 이번 세션도 `look2`를 사용했지만, 이번 로그만으로는 기존에 확인했던
  `look2` no-op artifact 결론을 뒤집는 새 근거는 없었다.

운영 브리핑:

1. 지금 단계의 핵심 진척은 "안정성 확보"다. 최신 세션에서도 4연속 촬영이 정상 저장되고 다시 `capture-ready`로 복귀했다.
2. 현재 남은 제품 이슈는 "속도"와 "preset artifact 품질" 쪽이다. 촬영 실패보다는 preview close가 7~8초대로 느린 점이 더 직접적인 체감 문제다.
3. 따라서 다음 우선순위는 capture path 복구가 아니라, preview-ready 체감 시간을 줄이거나 `look2` artifact를 다시 authoring/export해서 운영 품질을 맞추는 쪽으로 보는 것이 맞다.

### 2026-04-19 21:40 +09:00 최신 실패 세션 기록: 두 번째 샷은 `capture-accepted` 뒤 RAW handoff가 30초 budget 안에 닫히지 못했다

사용자 최신 제보:

1. 가장 최근 세션 로그를 확인해 달라고 요청했다.
2. 촬영 중 두 번째 샷에서 멈췄다고 제보했고, 개선도 함께 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c217f2a2948c`였다.
- 같은 세션의 `session.json` 기준으로
  - 총 1장만 저장
  - 세션 최종 stage는 `capture-ready`
  - 첫 장은 `renderStatus = previewReady`
  였다.
- 같은 세션의 `camera-helper-requests.jsonl`에는 요청이 2개 있었고,
  - 첫 요청 `request_000000000000064fcf62c29790`는 정상 저장됐다.
  - 둘째 요청 `request_000000000000064fcf63499fc8`는 요청만 남고 저장본이 생기지 않았다.
- 같은 세션의 `camera-helper-events.jsonl` 기준으로 둘째 요청은
  - `capture-accepted`
  - 이후 `file-arrived` 없음
  - `recovery-status(detailCode=capture-download-timeout)`
  - `helper-error(detailCode=capture-download-timeout)`
  순서로 끝났다.
- 최종 `camera-helper-status.json`은 다시
  - `cameraState = ready`
  - `helperState = healthy`
  로 회복했다.

이번 회차 해석:

- 이번 실패는 프런트 과승격이 아니라, **둘째 샷의 실제 RAW handoff가 helper 기본 timeout 안에 닫히지 못한 케이스**로 보는 것이 맞다.
- 첫 장 성공 뒤 곧바로 둘째 요청이 `capture-accepted`까지 간 점을 보면, 요청 소비 자체보다 **follow-up transfer completion budget 부족** 쪽 증거가 더 강했다.
- 세션이 나중에 다시 `capture-ready`로 풀린 것도, 카메라 전체 상실보다 **느린 handoff 후 recovery** 패턴에 가깝다.

이번 회차 수정:

- Canon helper 기본 capture completion timeout을 `30초 -> 45초`로 늘렸다.
- host capture round-trip timeout도 `35초 -> 50초`로 늘려 helper보다 먼저 실패 잠금을 걸지 않게 맞췄다.
- timeout 기본값을 테스트로 고정해, 이후 follow-up capture headroom이 다시 줄어드는 회귀를 막는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml default_capture_round_trip_timeout_keeps_additional_headroom`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`
  - `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745202892851_pAVdAAA7pU`
- 모두 통과

이번 시점 제품 판단:

- 최신 실패 세션의 직접 증거는 둘째 샷 `capture-download-timeout`이었다.
- 이번 수정은 촬영 correctness 로직을 바꾸기보다, **실장비 follow-up capture가 느릴 때 premature timeout으로 세션 흐름이 끊기지 않도록 headroom을 늘린 안정화 조정**으로 기록한다.

### 2026-04-19 22:07 +09:00 최신 baseline rerun 세션 기록: one-session package는 닫혔지만 official `preset-applied visible <= 3000ms` gate는 여전히 실패했다

사용자 최신 요청:

1. 최근 세션을 확인해서 그대로 진행해 달라고 요청했다.
2. `1.10` old resident first-visible baseline evidence lane으로 쓸 수 있는지 판단이 필요했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - 총 3장 모두 `renderStatus = previewReady`
  - 세션 종료 시점 `lifecycle.stage = capture-ready`
  였다.
- 같은 세션의 `camera-helper-status.json` 최종 상태는
  - `cameraState = ready`
  - `helperState = healthy`
  - `cameraModel = Canon EOS 700D`
  로 닫혔다.
- 같은 세션의 `camera-helper-events.jsonl`에는 3개 요청 모두
  - `capture-accepted`
  - `file-arrived`
  - `fast-preview-ready`
  가 같은 `requestId` / `captureId`로 이어졌다.
- 같은 세션의 `timing-events.log`에는 3개 요청 모두
  - `request-capture`
  - `file-arrived`
  - `preview-render-start`
  - `recent-session-pending-visible`
  - `capture_preview_ready`
  - `recent-session-visible`
  가 남아 있었다.
- 즉 이번 최신 세션은 `1.10` 기준의 one-session evidence package로는 사용할 수 있다고 판단했다.

이번 세션의 직접 수치:

- same-capture first-visible reference:
  - 1장: `4685ms`
  - 2장: `3587ms`
  - 3장: `3270ms`
- official release gate인 preset-applied visible:
  - 1장: `8972ms`
  - 2장: `7942ms`
  - 3장: `7967ms`
- `preview-render-ready elapsedMs`:
  - 1장: `5226ms`
  - 2장: `4337ms`
  - 3장: `4685ms`

추가 메타데이터:

- booth PC: `NOAHLEE`
- observed darktable CLI version: `5.4.0`
- observed OpenCL/GPU capability: `darktable-cltest`는 현장 재확인 시 `120s timeout`으로 관찰값을 닫지 못했다.

이번 회차 해석:

- 이번 최신 세션은 촬영 저장, helper correlation, same-session replacement, 최종 `capture-ready` 복귀까지는 정상이다.
- 따라서 old line baseline evidence lane 관점에서는
  **revalidation evidence package 자체는 닫혔다**고 보는 것이 맞다.
- 하지만 공식 제품 게이트는 여전히
  `preset-applied visible <= 3000ms`
  인데,
  이번 세션은 3장 모두 `7.9s ~ 9.0s` 수준에 머물렀다.
- 즉 이번 최신 세션의 결론은
  **revalidation success / release gate fail**이다.

이번 시점 제품 판단:

1. `1.10`은 더 이상 `rerun pending`으로 보기 어렵다. 최신 세션 하나로 baseline evidence package는 실제로 수집됐다.
2. 다만 이 결과는 old line이 release-proof라는 뜻이 아니다. official gate 실패 증거가 다시 한 번 확인된 것이다.
3. 따라서 현재 단계의 의미는 `old line이 재현 가능한지 확인`까지이며, 다음 route 판단은 여전히 `1.26 reserve path` 개시 여부 쪽으로 읽는 편이 맞다.

### 2026-04-20 11:54 +09:00 최신 `1.26` reserve path 하드웨어 검증 기록: official gate는 실패했고, intended reserve close owner도 실제로 관찰되지 않았다

사용자 최신 요청:

1. 앱을 실행해 하드웨어 검증을 마쳤다고 알렸다.
2. 최신 로그를 확인해 데이터 기록과 현재 다음 절차를 정리해 달라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - 총 4장 촬영
  - 1장은 `renderStatus = previewWaiting`
  - 나머지 3장은 `renderStatus = previewReady`
  - 세션 종료 시점 `lifecycle.stage = capture-ready`
  였다.
- 같은 세션의 `camera-helper-status.json` 최종 상태는
  - `cameraState = ready`
  - `helperState = healthy`
  - `cameraModel = Canon EOS 700D`
  로 닫혔다.
- 같은 세션의 `camera-helper-events.jsonl`에서는 4개 요청 모두
  - `capture-accepted`
  - `file-arrived`
  - `fast-preview-ready`
  까지 이어졌다.
- 그러나 같은 세션의 `timing-events.log`에서 customer-visible close owner를 보면
  - `fast-preview-promoted kind=legacy-canonical-scan`
  - `preview-render-ready ... binary=C:\Program Files\darktable\bin\darktable-cli.exe`
  - `sourceAsset=raw-original`
  로 기록됐다.
- 즉 이번 회차에서는 Story `1.26`이 의도한
  `same-capture preset-applied-preview -> host-owned truthful close owner`
  가 실제로 관찰되지 않았고,
  여전히 `darktable-cli + raw-original` preview close가 제품 close owner로 남아 있었다.

이번 세션의 직접 수치:

- same-capture first-visible reference:
  - 1장: `3100ms`
  - 2장: `2962ms`
  - 3장: `3015ms`
  - 4장: `3698ms`
- official release gate인 preset-applied visible:
  - 1장: `미닫힘` (`previewWaiting`, `preview-render-failed`)
  - 2장: `7486ms`
  - 3장: `7716ms`
  - 4장: `8796ms`
- preview close 자체의 `previewReady - raw persisted`:
  - 2장: `4831ms`
  - 3장: `5122ms`
  - 4장: `6453ms`

이번 회차 해석:

- 세션 자체는 완전히 붕괴하지 않았다. helper는 healthy로 돌아왔고, 마지막 stage도 `capture-ready`로 복귀했다.
- 하지만 이번 패키지는 Story `1.26`의 성공 근거가 아니다.
- 이유는 두 가지다.
  - official `preset-applied visible <= 3000ms` gate를 전혀 닫지 못했다.
  - 더 중요하게는 intended reserve truthful close owner인 `preset-applied-preview`가 실제 field evidence에서 보이지 않았다.
- 이번 세션의 실제 close owner는 여전히 `darktable-cli` 기반 `raw-original` preview close로 읽는 것이 맞다.
- 따라서 이번 하드웨어 패키지의 결론은
  **Story `1.26` hardware `No-Go`**
  이다.

이번 시점 제품 판단:

1. `1.26`은 아직 release candidate가 아니다.
2. 지금 필요한 것은 하드웨어를 한 번 더 찍는 것이 아니라, 왜 booth hardware에서 reserve path의 intended close owner가 실제로 발동하지 않았는지 software/diagnostics mismatch를 먼저 좁히는 일이다.
3. 다음 단계는 `1.26`을 `review / No-Go`로 기록하고, reserve path가 실제 field에서 `preset-applied-preview`를 만들지 못한 이유를 먼저 디버깅한 뒤에 rerun 하는 것이다.

### 2026-04-20 12:46 +09:00 최신 `1.26` reserve path 하드웨어 재검증 기록: owner attribution은 보였지만 official gate는 여전히 실패했다

사용자 최신 요청:

1. reserve path owner logging 수정 뒤 다시 하드웨어 검증을 마쳤다고 알렸다.
2. 최신 로그를 확인해 데이터 기록과 다음 단계를 정리해 달라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f3c5b88c698c`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - 총 4장 모두 `renderStatus = previewReady`
  - 세션 종료 시점 `lifecycle.stage = capture-ready`
  였다.
- 같은 세션의 `camera-helper-status.json` 최종 상태는
  - `cameraState = ready`
  - `helperState = healthy`
  - `cameraModel = Canon EOS 700D`
  로 닫혔다.
- 같은 세션의 `timing-events.log`를 보면
  - 1장은 `preview-render-ready ... sourceAsset=raw-original`
  - 2장~4장은 `preview-render-ready ... sourceAsset=preset-applied-preview`
  - `recent-session-visible`도 2장~4장은 `previewKind=preset-applied-preview`
  로 남았다.
- 즉 이번 회차에서는 이전 blocker였던 owner attribution ambiguity는 일부 해소됐다. reserve path close owner가 적어도 2장~4장에서는 field evidence에 그대로 드러났다.
- 다만 1장은 여전히 `raw-original` close로 닫혔고, reserve path가 모든 샷에서 일관되게 product close owner로 올라오지는 못했다.

이번 세션의 직접 수치:

- same-capture first-visible reference:
  - 1장: `3942ms`
  - 2장: `3282ms`
  - 3장: `2861ms`
  - 4장: `2816ms`
- official release gate인 preset-applied visible:
  - 1장: `8616ms`
  - 2장: `7712ms`
  - 3장: `8165ms`
  - 4장: `7643ms`

이번 회차 해석:

- 이전과 달리 `preset-applied-preview`가 field/UI evidence에 실제 close owner로 찍히는 샷이 확인됐다.
- 따라서 지난 회차의 핵심 blocker였던 `owner logging mismatch`는 이번 evidence로는 주된 이슈가 아니다.
- 그러나 제품 합격선인 `preset-applied visible <= 3000ms`는 여전히 전혀 닫히지 않았다.
- 더구나 첫 샷은 아직 `raw-original` close로 남아서 reserve path coverage도 완전히 닫히지 않았다.
- 따라서 이번 세션의 결론도
  **Story `1.26` hardware `No-Go`**
  이다.

이번 시점 제품 판단:

1. `1.26`은 owner attribution 관점에서는 한 단계 전진했지만, release candidate는 아니다.
2. 이제 우선순위는 `누가 닫았는지`보다 `왜 reserve close가 7초대 후반~8초대에서 닫히는지`, 그리고 `왜 첫 샷은 아직 raw-original로 닫히는지`를 줄이는 일이다.
3. 다음 단계는 하드웨어를 반복 촬영하는 것이 아니라, `first-shot coverage`와 `preset-applied close latency`를 먼저 줄인 뒤 approved hardware에서 다시 한 세션을 재검증하는 것이다.

### 2026-04-20 13:27 +09:00 최신 `1.26` 현장 실패 세션 기록: 첫 컷은 실제 preset-applied truth가 아니었고, 둘째 요청은 `capture-accepted` 뒤 handoff가 끝내 닫히지 않았다

사용자 최신 요청:

1. 방금 실장비 검증을 마쳤으니 최신 로그를 확인해 기록하라고 요청했다.
2. 첫 번째 사진은 프리셋이 실제로 적용되지 않았고, 두 번째 촬영은 한참 멈춘 뒤 실패했다고 제보했다.
3. 위 두 증상을 같이 수정해 달라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f61aa8bc153c`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - 저장된 컷은 1장뿐이었고
  - 그 1장은 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`
  - 세션 최종 stage는 `phone-required`
  였다.
- 그러나 같은 세션의 `camera-helper-events.jsonl`를 보면 첫 번째 요청 `request_000000000000064fdcb391aef8`은
  - `capture-accepted`
  - `file-arrived`
  - `fast-preview-ready`
  까지 닫혔지만,
  - `fast-preview-ready.fastPreviewKind = windows-shell-thumbnail`
  로 남아 있었다.
- 같은 첫 번째 요청의 `timing-events.log`는
  - `fast-preview-promoted kind=preset-applied-preview`
  - `preview-render-ready ... sourceAsset=preset-applied-preview`
  - `recent-session-visible ... previewKind=preset-applied-preview`
  를 기록했다.
- 즉 host는 첫 컷을 `preset-applied-preview`로 닫았지만, helper truth는 실제로 `windows-shell-thumbnail`였다.
  이번 회차의 첫 컷은 **진짜 preset-applied close가 아니라 same-capture shell thumbnail 오판정**으로 읽는 것이 맞다.
- 두 번째 요청 `request_000000000000064fdcb44c3298`은
  - `camera-helper-requests.jsonl`에 기록됐고
  - `camera-helper-events.jsonl`에는 `capture-accepted`와 `fast-thumbnail-attempted(camera-thumbnail)`까지만 남았으며
  - 이후 `file-arrived`, `fast-preview-ready`, `recovery-status`, `helper-error`가 끝내 오지 않았다.
- 같은 세션의 마지막 `camera-helper-status.json`은
  - `cameraState = capturing`
  - `helperState = healthy`
  - `requestId = request_000000000000064fdcb44c3298`
  - `detailCode = capture-in-flight`
  에 멈춰 있었다.

이번 회차 해석:

- 첫 컷 문제의 직접 원인은, host가 same-capture canonical preview를 진짜 `preset-applied-preview`처럼 올려 버린 truth classification bug였다.
- 둘째 요청 문제의 직접 증거는 `capture-accepted` 뒤 `file-arrived`도 `capture-download-timeout`도 안 남은 채 helper가 `capturing`에 걸려 버렸다는 점이다.
- 따라서 이번 세션은 단순한 속도 문제만이 아니라,
  - `first-shot truthful owner 오판정`
  - `follow-up capture orphan/stall`
  두 경계가 같이 드러난 실패 패키지다.
- 이번 세션의 결론도
  **Story `1.26` hardware `No-Go`**
  이다.

이번 회차 수정:

- host는 canonical same-capture scan을 다시 `legacy-canonical-scan` pending path로만 취급하고,
  실제 truthful handoff가 있을 때만 `preset-applied-preview` close owner를 허용하도록 되돌렸다.
- helper에는 active capture task가 timeout을 넘기고도 끝나지 않을 때
  service loop가 강제로 `capture-download-timeout` recovery로 닫는 watchdog을 추가했다.
- preview direct fallback은 speculative render가 큐를 점유 중일 때 바로 hard fail하지 않도록,
  queue saturation에서 짧게 기다렸다가 재시도하도록 보강했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`
  - `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745202892851_pAVdAAA7pU`
- 모두 통과

이번 시점 제품 판단:

1. 최신 현장 로그 기준으로, 첫 컷이 `preset-applied-preview`처럼 보였던 것은 성공이 아니라 오판정이었다.
2. 둘째 촬영 정지는 helper가 follow-up capture를 orphan 상태로 남기고 회복 로그도 못 쓰는 경계가 실제로 열려 있었던 것으로 본다.
3. 지금 필요한 다음 단계는 post-fix build로 approved hardware 세션 1개를 다시 찍어,
   첫 컷이 더 이상 shell thumbnail을 truthful close로 주장하지 않는지와,
   둘째 요청이 최소한 `file-arrived` 또는 `capture-download-timeout` recovery로 닫히는지를 확인하는 일이다.

### 2026-04-20 14:02 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: 이전 first-shot truth 문제는 남아 있었고, 최신 재검증에서는 첫 요청 RAW handoff 자체가 live helper loop를 붙잡고 멈췄다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 로그를 확인하고 기록하라고 요청했다.
2. 첫 번째 사진은 프리셋이 적용되지 않았다고 체감했고, 두 번째 사진은 멈춘 뒤 결국 촬영되지 않았다고 제보했다.
3. 로그 확인 후 실제 멈춘 경계를 고치라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f7ff7a8886b4`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - 세션 최종 stage는 `phone-required`
  였다.
- 같은 세션의 `camera-helper-events.jsonl`에는
  - `capture-accepted`
  - `fast-thumbnail-attempted(camera-thumbnail)`
  만 남았고,
  - `file-arrived`
  - `fast-preview-ready`
  - `recovery-status`
  - `helper-error`
  는 끝내 오지 않았다.
- 같은 세션의 `camera-helper-status.json` 마지막 상태는
  - `cameraState = capturing`
  - `helperState = healthy`
  - `requestId = request_000000000000064fdd2fef19e0`
  - `detailCode = capture-in-flight`
  로 멈춰 있었다.
- 같은 세션의 `captures/originals`에는
  - `capture_20260420050112090_f16e3f9885.downloading.CR2`
  - `Length = 0`
  인 임시 RAW 파일만 남아 있었다.
- `timing-events.log`에는 `request-capture` 외에 저장/preview 완료 경계가 전혀 남지 않았다.
- 운영 감사 로그 `operator-audit-log.json`은 이 세션을 `capture-round-trip-failed` / `capture-timeout`으로 기록했다.
- 바로 직전 실패 세션 `session_000000000018a7f61aa8bc153c`에서는 이미
  - 첫 저장 컷이 helper truth 기준 `windows-shell-thumbnail`였는데도 host가 `preset-applied-preview`로 닫아 버린 first-shot truthful close 오판정
  - 둘째 요청이 `capture-accepted` 뒤 `capture-in-flight`에 걸린 정지
  가 동시에 확인돼 있었다.

이번 회차 해석:

- 최신 14:02 세션은 첫 요청에서 이미 RAW handoff가 닫히지 않았기 때문에, 이번 로그만으로는 첫 컷 프리셋 적용 여부를 다시 판정할 단계까지 가지도 못했다.
- 대신 최신 로그는 더 근본적인 경계를 보여 줬다. helper의 live capture path가 `camera-thumbnail` 시도 직후 RAW transfer를 끝내 닫지 못했고, 그 동안 helper loop도 timeout recovery/status 갱신을 쓰지 못했다.
- 즉 이번 시점의 직접 원인은
  - 이전 세션에서 이미 확인된 `first-shot truthful close 오판정`
  - 최신 세션에서 새로 확인된 `live RAW handoff가 helper loop 자체를 붙잡는 stall`
  두 갈래로 정리된다.
- 이번 세션의 결론도
  **Story `1.26` hardware `No-Go`**
  이다.

이번 회차 수정:

- helper RAW download는 SDK object callback을 오래 붙잡지 않도록 전용 long-running worker로 다시 분리했다.
- live `camera-thumbnail` 즉시 추출은 EOS 700D 현장에서는 RAW handoff를 멈추게 할 수 있어, post-download raw fallback을 우선하는 쪽으로 껐다.
- 따라서 helper loop는 capture stall 중에도 살아 있어서 `capture-download-timeout` recovery를 계속 쓸 수 있게 됐다.
- 첫 컷 truthful close는 이전 회차에서 이미 고친 대로, 실제 preset-applied truth가 아닌 same-capture preview는 더 이상 `preset-applied-preview`로 주장하지 않게 유지된다.

검증:

- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`
  - `6 passed`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_capture_download_timeout_recovers -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_times_out_when_helper_accepts_but_no_file_arrives -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_saved_keeps_a_fast_same_capture_thumbnail_visible_while_preview_render_is_still_pending -- --exact`
  - 모두 통과
- 참고로 `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1` 전체 스위트는 이번 턴에도 첫 실패 1건 뒤 mutex poison으로 연쇄 실패가 나와, 제품 수정 검증은 helper 전체 + 관련 Rust 개별 회귀 테스트로 확인했다.

이번 시점 제품 판단:

1. latest failure는 “둘째 샷만 hang”이 아니라, 아예 첫 요청부터 live RAW handoff가 helper loop를 묶어 버릴 수 있음을 보여 준다.
2. 따라서 reserve path의 다음 승인 기준은 속도보다 먼저, `capture-accepted` 뒤 helper가 무한 정지하지 않고 반드시 `file-arrived` 또는 `capture-download-timeout` recovery 중 하나로 닫히는지다.
3. post-fix approved hardware 재검증에서는
   - 첫 컷이 실제 preset-applied truth 없이 그렇게 주장하지 않는지
   - 최신처럼 `.downloading.CR2`만 남기고 멈추지 않는지
   를 같은 세션 패키지에서 같이 확인해야 한다.

### 2026-04-20 14:17 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: helper는 떠 있었지만 첫 status를 전혀 쓰지 못해 preset-selected / preparing에 고정됐다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 파일을 확인하고 기록하라고 요청했다.
2. 이번에는 화면이 `preparing`에서 멈춰 있다고 제보했다.
3. 최신 로그를 기준으로 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f8e4828da598`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - 세션 최종 stage는 `preset-selected`
  였다.
- 같은 세션의 `diagnostics` 폴더에는
  - `camera-helper-status.json`
  - `camera-helper-events.jsonl`
  - `timing-events.log`
  가 아예 생성되지 않았다.
- 전역 운영 감사 로그 `operator-audit-log.json`에도 이 세션은 `session-started` 1건만 남았고,
  이후 `capture-round-trip-failed` 같은 후속 기록도 없었다.
- 반면 같은 시점의 프로세스 상태를 보면
  - `dotnet run --project ... CanonHelper.csproj --session-id session_000000000018a7f8e4828da598`
  - `canon-helper.exe --session-id session_000000000018a7f8e4828da598`
  가 둘 다 살아 있었다.
- 즉 이번 회차는 helper supervisor가 launch 자체를 실패한 것이 아니라,
  helper가 실제로 떠 있는 동안 첫 `camera-status` 파일을 쓰기 전 경계에서 막혀
  host가 live truth를 영원히 `missing`으로 읽어 `Preparing`에 고정된 케이스로 보는 것이 맞다.

이번 회차 해석:

- 이전 14:02 세션은 RAW handoff stall이었고, 이번 14:17 세션은 그보다 더 앞단인 helper startup/connect stall이다.
- 최신 세션에서는 촬영 요청 단계까지도 가지 못했기 때문에, 원본/preview/timeout diagnostics가 하나도 남지 않았다.
- 제품 관점의 직접 원인은
  - helper launch 이후 첫 status 파일이 없으면 host가 `camera-preparing`으로만 해석하는 구조
  - startup 상태가 오래 stale 되어도 이를 failure boundary로 승격하지 않는 readiness 규칙
  이 두 개가 겹친 것이다.
- 이번 세션의 결론도
  **Story `1.26` hardware `No-Go`**
  이다.

이번 회차 수정:

- helper supervisor는 helper를 띄우는 즉시 `cameraState=connecting`, `helperState=starting`, `detailCode=helper-starting` startup status를 먼저 기록하도록 바꿨다.
- host readiness는 이 startup status가 5초 이상 stale 되면 더 이상 `Preparing`으로 붙잡지 않고 `Phone Required`로 승격하도록 바꿨다.
- 따라서 같은 종류의 startup hang이 다시 나더라도, 제품은 무한 `preparing` 대신 운영자 개입이 필요한 고장 경계로 닫히게 된다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::helper_supervisor::tests::starting_helper_status_is_written_before_live_status_arrives -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_stays_blocked_until_live_helper_truth_is_fresh -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance -- --exact`
  - 모두 통과

이번 시점 제품 판단:

1. 최신 `preparing` 정지는 helper가 없어서가 아니라, helper가 첫 status를 못 남긴 startup stall이었다.
2. 이번 패치로 같은 stall은 더 이상 고객 화면에서 무한 `preparing`으로 남지 않는다.
3. 다만 low-level camera open 자체가 왜 막혔는지는 아직 hardware 재검증이 필요하므로, post-fix 확인에서는 먼저 `preparing` 무한정지가 사라졌는지부터 다시 보면 된다.

### 2026-04-20 14:31~14:32 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: startup status stale 승격이 작동해 phone required는 떴지만, 실제 원인은 dotnet helper cold start 경로였다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 파일을 확인하고 기록하라고 요청했다.
2. 이번에는 화면에 `phone required`가 발생했다고 제보했다.
3. 최신 실패를 기준으로 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 연속된 두 개였다.
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f9acaf600638`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f9b49261d518`
- 두 세션의 `session.json` 기준 공통 상태는
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - 세션 stage는 둘 다 `preset-selected`
  였다.
- 두 세션의 `diagnostics` 폴더에는
  - `camera-helper-status.json`만 있었고
  - `camera-helper-events.jsonl`
  - `timing-events.log`
  - `camera-helper-requests.jsonl`
  는 생성되지 않았다.
- 두 세션의 `camera-helper-status.json`은 둘 다
  - `cameraState = connecting`
  - `helperState = starting`
  - `detailCode = helper-starting`
  - `sequence = 0`
  만 남아 있었다.
- 전역 운영 감사 로그 `operator-audit-log.json`에도 두 세션은 `session-started` 외의 추가 실패 이벤트가 없었다.
- 같은 시점 로컬 환경을 확인한 결과
  - `sidecar/canon-helper/src/CanonHelper/bin/Debug/net8.0/canon-helper.exe`는 이미 존재했고
  - `dotnet --version`도 정상 응답했다.
- 즉 현재 launcher는 이미 실행 가능한 `canon-helper.exe`가 있어도 debug 환경에서는 `dotnet run --project ...`를 우선 택하고 있었고,
  이 cold start 동안 seeded startup status가 stale 되어 host readiness가 `phone required`로 승격된 것으로 읽는 편이 맞다.

이번 회차 해석:

- 이번 `phone required`는 새로운 capture failure가 아니라, 직전 회차에서 넣은 `stale startup => failure escalation`이 실제로 작동한 결과였다.
- 그 자체는 무한 `preparing`보다 낫지만, 이번 evidence는 왜 stale 되었는지가 더 중요했다.
- 최신 두 세션의 직접 원인은 camera raw path가 아니라 `helper launch path`였다.
  - 이미 존재하는 `canon-helper.exe` 대신
  - 느린 `dotnet run` cold start를 우선 택하면서
  - 첫 live status 갱신 전에 startup seed가 stale 되었다.
- 따라서 이번 세션의 결론도
  **Story `1.26` hardware `No-Go`**
  이지만,
  blocker 해석은 `camera startup itself`보다 `slow launch selection on dev booth path` 쪽으로 더 좁혀졌다.

이번 회차 수정:

- helper supervisor는 이제 실행 가능한 `canon-helper.exe`가 존재하면 `dotnet run`보다 먼저 그 실행 파일을 우선 사용한다.
- startup seed status 기록과 stale startup 승격은 유지한다.
- 따라서 같은 dev booth 환경에서는 cold start 빌드 시간 때문에 `phone required`로 넘어가는 경로를 먼저 줄이고,
  정말 helper가 시작되지 못할 때만 failure boundary가 남도록 정리했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::helper_supervisor::tests::existing_helper_executable_is_preferred_over_dotnet_run -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib capture::helper_supervisor::tests::starting_helper_status_is_written_before_live_status_arrives -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_stays_blocked_until_live_helper_truth_is_fresh -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance -- --exact`
  - 모두 통과

이번 시점 제품 판단:

1. 최신 `phone required`는 새 촬영 failure가 아니라, launch가 느린 helper 경로 선택 때문에 생긴 startup stale escalation이었다.
2. 이번 패치로 booth 개발 환경에서는 이미 빌드된 helper exe를 바로 써서 이 경로를 줄였다.
3. post-fix 실장비 확인에서는 우선 `preset 선택 뒤 phone required로 바로 튀지 않는지`부터 다시 보면 된다.

### 2026-04-20 14:41 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: launch 경로는 정상화됐지만 helper 내부 camera connect 시도가 loop를 붙잡아 첫 live status를 못 남겼다

사용자 최신 요청:

1. `history` 문서를 보면 어떤 조건에서 카메라 상태가 보이고 촬영 준비가 되는지 적혀 있으니 그 기준으로 다시 해결하라고 요청했다.
2. 같은 문제가 계속 발생하니 최신 로그를 다시 확인하고 기록하라고 요청했다.
3. 잘 되던 기능에서 왜 이런 문제가 반복되느냐는 제품 관점 설명도 함께 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fa2f55d79a94`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 세션의 `diagnostics/camera-helper-status.json`에는 host가 launch 직후 심어 둔 seed 1건만 남아 있었다.
  - `cameraState = connecting`
  - `helperState = starting`
  - `detailCode = helper-starting`
  - `sequence = 0`
- 하지만 같은 시점 프로세스 상태를 보면 실제로는
  - `canon-helper.exe --runtime-root C:\Users\KimYS\Pictures\dabi_shoot --session-id session_000000000018a7fa2f55d79a94`
  가 살아 있었다.
- 별도로 helper binary를 직접 `--self-check`로 확인하면
  - runtime DLL present
  - SDK initialize 성공
  - camera count = 1
  - `detailCode = camera-ready`
  가 즉시 나왔다.
- 즉 이번 최신 failure는 더 이상 `dotnet run` cold start가 아니고,
  이미 올라온 helper 내부에서 **camera connect / session open 시도가 helper loop를 붙잡는 경계**가 남아 있었던 것으로 읽는 편이 맞다.

`history` / 계약 문서 기준으로 이번 상태를 다시 읽으면:

- booth가 `Ready`가 되려면 helper raw truth가 최소
  - same session
  - fresh status
  - `cameraState = ready`
  - `helperState = healthy`
  로 닫혀야 한다.
- 반대로 `connecting`, `connected-idle`, `starting`, `recovering`은 모두 blocked path다.
- 따라서 latest session처럼 helper가 살아 있어도 첫 live `camera-status`가 제품적으로 닫히지 않으면 booth는 준비 완료가 될 수 없다.

이번 회차 해석:

- 14:31~14:32 수정으로 `launch target` 문제는 줄였지만, 그 바로 다음 경계인 `helper 내부 camera open`이 여전히 bounded 되지 않아 같은 제품 문제로 보인 것이다.
- 그래서 현장에서는
  - 어떤 회차는 `preparing`
  - 어떤 회차는 `phone required`
  - 어떤 회차는 첫 shot false close
  - 어떤 회차는 둘째 shot stall
  처럼 다른 증상으로 보였지만,
  실제로는 reserve path 주변의 adjacent boundary들이 순서대로 드러난 것으로 보는 것이 맞다.
- "예전에는 되던 기능"처럼 보였던 이유도,
  예전에는 이 경계가 운 좋게 빨리 닫혀 증상이 가려졌고, 최근엔 truthful close / RAW handoff / stale startup guard를 하나씩 추가하면서 숨겨져 있던 정지 경계가 더 빨리 드러난 것이다.

이번 회차 수정:

- helper의 camera connect 시도를 main loop 바깥 background task로 분리했다.
- 따라서 SDK initialize / camera scan / session open이 느리거나 막히더라도 helper loop 자체는 계속 살아 있어 live status를 주기적으로 쓸 수 있다.
- helper는 connect 진행 중 상태를 더 잘게 남긴다.
  - `sdk-initializing`
  - `scanning`
  - `session-opening`
  - `session-opened`
- 그리고 connect 시도가 일정 시간 안에 닫히지 않으면 더 이상 무한 준비 상태로 두지 않고 `camera-connect-timeout` 명시 failure로 승격한다.

검증:

- helper targeted tests
  - connect attempt가 background로 분리돼 loop를 block하지 않는지
  - connect timeout이 explicit error로 승격되는지
  - 기존 RAW callback non-blocking / capture watchdog 회귀
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest failure는 더 이상 launch path 문제가 아니다. helper 내부 connect/open 경계가 bounded 되지 않았던 것이 직접 원인이다.
2. 이번 패치로 같은 종류의 정지는 최소한 live status와 explicit failure로 닫히므로, `살아 있는 helper인데 booth는 이유 없이 준비되지 않음` 상태는 줄어든다.
3. 이 기능이 계속 깨져 보인 이유는 하나의 버그가 흔들린 것이 아니라, reserve path 주변에 붙어 있던 서로 다른 경계들이 최근 truthful/health guard 도입으로 차례대로 드러났기 때문이다.

### 2026-04-20 14:59 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: `camera-connect-timeout`은 카메라 미발견이 아니라 async connect 이후 SDK 경합으로 읽는 편이 맞다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 파일을 확인하고 기록하라고 요청했다.
2. 이번에도 `phone required`가 발생했다고 제보했다.
3. 같은 문제를 실제로 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fb29e752039c`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 세션의 `diagnostics`에는 `camera-helper-status.json`만 남았고, 최종 상태는
  - `cameraState = error`
  - `helperState = error`
  - `detailCode = camera-connect-timeout`
  였다.
- 별도로 같은 helper binary를 `--self-check`로 직접 실행하면
  - SDK initialize 성공
  - camera count = 1
  - `detailCode = camera-ready`
  가 즉시 나왔다.
- 즉 이번 세션의 의미는 "카메라가 실제로 없다"가 아니라,
  booth helper runtime 안에서 connect/session-open 경계가 제 시간 안에 닫히지 못했다는 쪽이 더 맞다.

이번 회차 해석:

- 직전 회차에서 helper connect를 background task로 분리하면서 loop 생존성은 좋아졌지만,
  그 이후에도 main loop가 session-open 전 SDK event pump를 계속 만질 수 있었다.
- Canon EDSDK는 이런 startup 경합에 민감해서,
  self-check처럼 단독 실행에서는 바로 `camera-ready`가 나와도
  실제 booth helper runtime에서는 connect/open이 스스로 방해받아 `camera-connect-timeout`으로 닫힐 수 있다.
- 따라서 이번 `phone required`는 새로운 hardware fault라기보다,
  **비동기 connect 보강 뒤 session-open 전 SDK 호출이 겹친 회귀**로 읽는 편이 맞다.

이번 회차 수정:

- helper는 camera session이 실제로 열린 뒤에만 SDK event pump를 돌리도록 바꿨다.
- 즉 connect/open이 진행 중일 때는 main loop가 EDSDK event pump를 건드리지 않게 막았다.
- 이로써
  - loop는 계속 살아 있어 status를 기록하고
  - connect/open 자체는 단독 경로로 닫히며
  - 둘 사이의 startup SDK 경합은 줄어든다.

검증:

- helper targeted tests
  - connect attempt가 background로 분리돼 loop를 block하지 않는지
  - connect timeout이 explicit error로 승격되는지
  - RAW callback non-blocking / capture watchdog 회귀
  - **session-open 전에는 SDK event pump를 건드리지 않는지**
  - 모두 통과
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest `phone required`는 카메라가 실제로 사라진 것이 아니라, helper startup/connect 경합이 만든 false failure였다.
2. 이번 패치로 launch 뒤 connect/open과 event pump가 서로 경합하던 경로를 직접 줄였다.
3. 반복된 재발처럼 보인 이유는 reserve path 주변 여러 경계가 순차적으로 드러난 데 더해, 이번에는 그 중 하나를 막는 과정에서 startup SDK 경합 회귀가 새로 생겼기 때문이다.

### 2026-04-20 15:17 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: `camera-connect-timeout`의 직접 원인은 async connect를 `Task.Run`으로 옮기며 STA 문맥을 잃은 쪽이 더 유력하다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 파일을 확인하고 기록하라고 요청했다.
2. 이번에도 `phone required`가 발생했다고 제보했다.
3. 같은 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fc2aba129e1c`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 세션의 `camera-helper-status.json` 최종 상태는 다시
  - `cameraState = error`
  - `helperState = error`
  - `detailCode = camera-connect-timeout`
  였다.
- 그런데 같은 helper binary를 별도로 `--self-check`로 직접 실행하면 여전히
  - SDK initialize 성공
  - camera count = 1
  - `detailCode = camera-ready`
  가 즉시 나왔다.
- 즉 event pump 경합만 줄여서는 부족했고, helper runtime과 self-check 사이에 남은 결정적 차이는
  **connect/open이 실행되는 thread 문맥**이었다.

이번 회차 해석:

- helper `Program.Main`은 STA thread에서 시작된다.
- 하지만 이전 회차의 async connect 보강은 `Task.Run(TryOpenCamera)`로 connect/open을 threadpool로 옮겼고,
  이 경로는 self-check와 달리 STA 문맥을 보장하지 못한다.
- Canon EDSDK는 startup/open 경계에서 이런 thread 문맥 차이에 민감할 수 있으므로,
  self-check는 바로 `camera-ready`인데 booth helper runtime만 `camera-connect-timeout`이 나는 현재 증거와 가장 잘 맞는 해석은
  **connect worker가 STA를 잃은 회귀**다.

이번 회차 수정:

- helper connect/open worker를 일반 threadpool `Task.Run`이 아니라
  **전용 STA background thread**에서 실행하도록 바꿨다.
- 동시에 직전 회차에서 넣은
  - session-open 전 SDK event pump 차단
  - connect 진행 중 bounded status 유지
  는 그대로 유지한다.
- 따라서 이제 booth helper runtime도 self-check와 같은 STA 문맥에서 camera open을 시도하게 된다.

검증:

- helper targeted tests
  - connect attempt가 loop를 block하지 않는지
  - connect timeout이 explicit error로 승격되는지
  - **connect attempt가 실제로 STA thread에서 도는지**
  - session-open 전에는 SDK event pump를 건드리지 않는지
  - RAW callback non-blocking / capture watchdog 회귀
  - 모두 통과
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest `phone required`는 카메라 미발견이 아니라, helper async connect implementation이 self-check와 다른 thread 문맥으로 실행되던 회귀였다.
2. 이번 패치로 helper runtime도 self-check와 같은 STA 문맥에서 connect/open을 시도하게 맞췄다.
3. 반복된 재발처럼 보인 이유는 reserve path 주변에 숨어 있던 여러 경계가 순서대로 드러났고, 이번엔 그중 startup connect를 비동기로 분리하는 과정에서 thread 문맥 회귀가 새로 생겼기 때문이다.

### 2026-04-20 15:21 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: 최신 session도 여전히 `camera-connect-timeout`이었으므로, 같은 detailCode helper는 supervisor가 자동 재기동하도록 바꿨다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 파일을 확인하고 기록하라고 요청했다.
2. 이번에도 `phone required`가 발생했다고 제보했다.
3. 실제로 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fc5e0caa7cfc`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 세션의 `camera-helper-status.json` 최종 상태는 다시
  - `cameraState = error`
  - `helperState = error`
  - `detailCode = camera-connect-timeout`
  였다.
- 즉 latest field failure는 다른 종류의 새 고장이 아니라,
  **같은 startup connect boundary가 고객 화면에서는 다시 `phone required`로 보인 것**이었다.

이번 회차 해석:

- 앞선 STA worker 보정이 connect/open 실행 문맥을 바로잡는 방향인 것은 맞지만,
  latest field evidence는 현장에서 한 번 더 같은 `camera-connect-timeout`이 남을 수 있음을 보여 줬다.
- 제품 관점에서 남은 문제는
  - helper가 그 상태로 살아 있는 동안
  - booth가 같은 failed helper를 계속 붙잡고
  - customer-visible 상태가 `phone required`로 고정된다는 점이다.
- 따라서 이번 시점의 직접 대응은 root cause 추가 축소와 별개로,
  **같은 detailCode helper를 자동으로 갈아끼워 다음 readiness poll에서 다시 연결을 시도하게 만드는 것**이 더 맞다.

이번 회차 수정:

- Rust helper supervisor는 이제 같은 session helper가 살아 있어도,
  최신 helper status가 `camera-connect-timeout`이면 그 helper를 정상 helper로 보지 않고 종료 후 다시 띄운다.
- 즉 connect timeout helper를 resident failed process로 남겨두지 않는다.
- 기존
  - built exe 우선 launch
  - startup seed 기록
  - stale startup guard
  - session-open 전 SDK event pump 차단
  - STA connect worker
  는 그대로 유지한다.

검증:

- Rust helper supervisor regression
  - `camera-connect-timeout` status가 restart 조건으로 읽히는지
  - built helper exe 우선 launch 회귀
  - 모두 통과
- helper targeted tests
  - async connect loop non-blocking
  - connect timeout handling
  - STA worker
  - pre-session-open event pump 차단
  - RAW callback non-blocking / capture watchdog
  - 모두 통과
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest `phone required`는 새로운 failure가 아니라, 같은 `camera-connect-timeout` helper가 고객 흐름에 그대로 남아 있던 문제였다.
2. 이번 패치로 booth는 connect-timeout helper를 자동으로 다시 띄워 재시도를 할 수 있게 됐다.
3. 즉 지금 단계의 제품 목표는 "왜 한 번 timeout이 났는지"만 보는 것이 아니라, 그 timeout이 곧바로 customer-visible terminal state로 굳지 않게 만드는 쪽까지 함께 닫는 것이다.

### 2026-04-20 15:29 +09:00 앱 재실행 뒤 최신 현장 실패 세션 기록: 이번엔 `phone required`가 아니라 fresh `session-opening`이 계속 남아 booth가 `Preparing`에 머물렀다

사용자 최신 요청:

1. 앱을 다시 실행한 뒤 하드웨어 테스트를 마쳤으니 최신 파일을 확인하고 기록하라고 요청했다.
2. 이번에는 화면이 `Preparing`이라고 제보했다.
3. 같은 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fcd1f65b2f7c`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 세션의 `camera-helper-status.json` 최종 상태는
  - `cameraState = connecting`
  - `helperState = connecting`
  - `cameraModel = Canon EOS 700D`
  - `detailCode = session-opening`
  - `sequence = 5`
  였다.
- 즉 이번 회차는 error 상태가 아니라, helper가 fresh한 `session-opening` truth를 계속 쓰고 있어
  host가 이를 정상 `Preparing`으로 해석한 케이스였다.

이번 회차 해석:

- 앞선 패치로 `camera-connect-timeout` helper는 자동 재시작되게 했지만,
  그 다음 경계에서는 helper가 에러로 닫히지 않고 `session-opening`을 계속 fresh하게 쓰면
  booth가 그것을 stuck state가 아니라 계속 진행 중으로 읽게 된다.
- 그래서 고객 입장에서는 `phone required` 대신 `Preparing`으로만 보이지만,
  제품 관점의 본질은 동일하다.
  - connect/open 경계가 정상적으로 닫히지 않았고
  - failed/stuck helper가 customer flow를 붙잡고 있다.

이번 회차 수정:

- Rust helper supervisor는 이제
  - `camera-connect-timeout`
  뿐 아니라
  - `cameraState=connecting`
  - `helperState=connecting`
  - `detailCode=session-opening`
  상태가 일정 시간 이상 계속되면
  그 helper도 stuck으로 보고 종료 후 다시 띄운다.
- 즉 `Preparing`으로 보이는 startup hang도 resident 상태로 남겨두지 않는다.

검증:

- Rust helper supervisor regression
  - `camera-connect-timeout` status restart
  - prolonged `session-opening` status restart
  - 모두 통과
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest `Preparing`은 새로운 종류의 문제라기보다, 같은 connect/open stall이 error 대신 fresh `session-opening`으로 보였던 변형이다.
2. 이번 패치로 booth는 `phone required`뿐 아니라 stuck `Preparing` helper도 자동으로 갈아끼워 다시 연결을 시도한다.
3. 즉 지금 단계의 목표는 현장에서는 어떤 형태로 보이든, connect/open stall helper가 고객 흐름을 계속 붙잡지 못하게 만드는 것이다.

### 2026-04-20 15:38 +09:00 최신 재확인: 이번엔 stuck helper가 아니라 `camera-connect-timeout -> 자동 재기동 -> session-opening`이 반복되며 제품 상태가 `Preparing`과 error 사이를 흔들었다

사용자 최신 요청:

1. 앱 실행 뒤 카메라 상태 에러와 `preparing` 발생을 다시 확인하고, 파일을 기록한 뒤 실제로 해결하라고 요청했다.
2. 단순 원인 설명이 아니라 제품적으로 닫히는 수정까지 하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fd1be0ffc418`였다.
- 같은 세션의 `session.json` 기준으로
  - `activePresetId = preset_new-draft-2`
  - `activePresetVersion = 2026.04.10`
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 세션의 `diagnostics/camera-helper-status.json`은 처음에는
  - `cameraState = connecting`
  - `helperState = connecting`
  - `detailCode = session-opening`
  으로 계속 갱신됐다.
- 같은 파일을 1초 간격으로 재확인했더니 아래 패턴이 반복됐다.
  - `2026-04-20 15:37:55 ~ 15:37:57 +09:00`: `session-opening`
  - `2026-04-20 15:37:58 +09:00`: `camera-connect-timeout`
  - `2026-04-20 15:38:00 +09:00`: sequence가 다시 `2`로 리셋되며 새 helper의 `session-opening`
  - `2026-04-20 15:38:05 +09:00`: 다시 `camera-connect-timeout`
- 즉 latest failure는
  - helper가 한 번 timeout 나는 것 자체보다
  - **supervisor가 그 failed helper를 즉시 다시 띄워 booth를 다시 `Preparing`으로 열어 버리는 재기동 루프**
  로 보는 편이 맞다.

이번 회차 해석:

- 앞선 수정은 startup hang을 resident stuck helper로 남기지 않게 하는 데는 맞았지만,
  hardware/open boundary가 계속 실패하는 경우에는 자동 재기동이 오히려 customer-visible 상태를 흔들었다.
- 그래서 최신 현장에서는
  - 어떤 순간에는 `Preparing`
  - 어떤 순간에는 `camera-connect-timeout`
  이 보였고,
  제품은 안정된 failure state로 닫히지 못했다.
- 이 단계에서 필요한 것은 restart 자체를 없애는 것이 아니라,
  **자동 재기동을 bounded retry로 제한하고 반복 실패는 안정된 보호 상태로 남기는 것**이었다.

이번 회차 수정:

- Rust helper supervisor에 startup restart budget을 추가했다.
- 같은 세션의 startup failure(`camera-connect-timeout`, prolonged `session-opening`)에 대해서는
  자동 재기동을 한 번만 허용한다.
- 같은 budget window 안에서 다시 같은 종류의 startup failure가 나오면,
  더 이상 helper를 자동 재기동하지 않고 기존 error status를 유지한다.
- 따라서 latest 같은 hardware/open failure가 계속되더라도 booth는
  무한 `Preparing` 재진입 대신 안정된 error/운영자 개입 상태로 닫힌다.

검증:

- 새 regression
  - 첫 startup retry는 허용되는지
  - budget window 안 반복 retry는 막히는지
  - budget window 밖에서는 다시 허용되는지
  - 모두 통과
- Rust helper supervisor regression
  - `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
    - 통과
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
    - 통과
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest `Preparing` 재발은 새 startup 상태가 아니라, `camera-connect-timeout` 뒤 auto-restart가 바로 다시 `session-opening`을 열던 loop였다.
2. 이번 패치로 booth는 transient startup glitch에는 한 번 재시도하되, 반복 실패를 더 이상 customer-visible `Preparing`으로 흔들지 않는다.
3. 따라서 다음 현장 확인 포인트는 "카메라 open 자체가 회복됐는가"와 별개로, **반복 실패 시에도 제품이 안정된 보호 상태로 남는가**다.

### 2026-04-20 15:45 +09:00 최신 재확인: restart loop는 닫혔지만 latest startup connect failure가 그대로 `Phone Required`로 번역돼, customer-safe blocked state로 다시 낮췄다

사용자 최신 요청:

1. 앱 실행 뒤 이번에는 `Phone Required`가 발생했다고 제보했다.
2. 최신 파일을 다시 확인하고 기록한 뒤 실제로 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fdb535d59960`였다.
- 직전 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fda8249bf6c4`였다.
- 직전 세션의 `camera-helper-status.json`은
  - `cameraState = connecting`
  - `helperState = connecting`
  - `cameraModel = Canon EOS 700D`
  - `detailCode = session-opening`
  으로 남아 있었다.
- 최신 세션의 `camera-helper-status.json`은
  - `cameraState = error`
  - `helperState = error`
  - `detailCode = camera-connect-timeout`
  으로 남아 있었다.
- 두 세션 모두
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  - `activePresetId = preset_new-draft-2`
  였다.
- 즉 latest failure는 더 이상 customer-visible state가 `Preparing <-> timeout`으로 흔들리는 loop가 아니라,
  startup connect가 끝내 열리지 못한 세션이 **안정된 `camera-connect-timeout` 상태로 닫힌 뒤 host가 이를 곧바로 `Phone Required`로 번역한 케이스**였다.

이번 회차 해석:

- 이전 회차 수정으로 restart loop 자체는 줄었다.
- 하지만 이번 evidence는 startup connect failure와 capture/persist failure를 제품이 같은 `Phone Required`로 다루고 있음을 보여 줬다.
- latest session에는
  - 촬영 요청 자체가 없었고
  - capture/file-arrived/render failure도 없었으며
  - startup connect/open boundary만 실패했다.
- 따라서 이번 `Phone Required`는 customer protection이 필요한 완료/저장 실패라기보다,
  **startup connect boundary를 customer terminal state로 과번역한 것**으로 읽는 편이 맞다.

이번 회차 수정:

- host readiness normalization은 이제
  - `captures = []`
  - active preset 존재
  - fresh/matched helper truth
  - `detailCode = camera-connect-timeout | camera-open-failed | session-open-failed`
  인 startup connect failure를
  customer-facing `Phone Required`가 아니라 customer-safe blocked `Preparing`으로 투영한다.
- 즉 booth는 여전히 촬영을 막지만,
  startup connect failure를 즉시 고객 보호 전환으로 노출하지 않는다.
- 실제 live truth의 `cameraState = error`, `helperState = error`, `detailCode`는 그대로 유지되므로
  operator/diagnostics에서는 failure 근거를 계속 볼 수 있다.

검증:

- 새 regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_stays_customer_safe_while_capture_remains_blocked -- --exact`
    - red에서 `Phone Required` 재현 후 green 통과
- host regression
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_helper_startup_status_stays_stale -- --exact`
    - 통과
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_preparing_and_phone_required_states_block_capture_with_customer_safe_guidance -- --exact`
    - 통과
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_surfaces_camera_connecting_guidance_when_helper_detects_powered_device -- --exact`
    - 통과

이번 시점 제품 판단:

1. latest `Phone Required`는 새 capture failure가 아니라, stable `camera-connect-timeout` startup failure를 host가 terminal customer state로 번역한 결과였다.
2. 이번 패치로 booth는 startup connect/open failure를 계속 막되, 고객 화면은 `Phone Required` 대신 customer-safe blocked state로 유지한다.
3. 다음 현장 확인 포인트는 두 가지다.
   - latest 같은 startup connect failure가 다시 나도 customer 화면이 `Phone Required`로 튀지 않는지
   - 실제 helper/open failure 자체는 여전히 남는지

### 2026-04-20 16:12 +09:00 최신 재확인: `Preparing` 고정의 실제 원인은 카메라 상태 미기록이 아니라, 기록된 startup error를 host가 customer-safe state로 다시 낮추는 정책이었다

사용자 최신 요청:

1. 앱 실행 뒤 여전히 `Preparing`이 계속 보인다고 제보했다.
2. 최신 파일을 확인하고 기록한 뒤, 카메라 상태가 실제로 반영되도록 다시 고치라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fe66c1b2e418`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  - `activePresetId = preset_new-draft-2`
  였다.
- 하지만 같은 세션의 `diagnostics/camera-helper-status.json`은 실제로 존재했고,
  - `cameraState = error`
  - `helperState = error`
  - `detailCode = camera-connect-timeout`
  - `observedAt = 2026-04-20T06:59:21.9856960+00:00`
  를 남기고 있었다.
- 즉 이번 회차는 helper가 카메라 상태를 못 쓴 것이 아니라,
  **이미 기록된 startup connect failure를 host readiness normalization이 customer-facing `Preparing`으로 다시 낮추고 있던 케이스**로 보는 것이 맞다.

이번 회차 수정:

- host readiness normalization은 더 이상 fresh startup
  - `camera-connect-timeout`
  - `camera-open-failed`
  - `session-open-failed`
  를 customer-safe `Preparing`으로 낮추지 않는다.
- 따라서 booth는 startup 카메라 error를 고객 화면에도 그대로 반영해,
  더 이상 `Preparing`에 고정되지 않고 `Phone Required` 계열 차단 상태로 닫힌다.
- operator/diagnostics에는 기존처럼 `cameraState`, `helperState`, `detailCode`가 그대로 남아 recovery 근거를 계속 제공한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
  - red에서 `Preparing` 재현 후 green 통과
- 이어서 startup stale escalation, helper blocked guidance, helper supervisor regression, `cargo build --manifest-path src-tauri/Cargo.toml`까지 확인

이번 시점 제품 판단:

1. latest `Preparing` 재발은 새 missing-status bug가 아니라, startup camera error를 과하게 완화한 readiness 정책의 결과였다.
2. 이번 패치로 booth는 startup connect failure를 더 이상 `Preparing`으로 숨기지 않고, 실제 카메라 상태를 고객 화면에도 반영한다.
3. 다음 현장 확인 포인트는 같은 startup failure에서 화면이 `Preparing`이 아니라 즉시 차단 상태로 보이는지다.

### 2026-04-20 16:15 +09:00 최신 재확인: 이번엔 `Preparing -> Phone Required` 번역 문제가 아니라 helper startup connect/open이 실제로 `camera-connect-timeout`으로 닫히는 경계가 남아 있었다

사용자 최신 요청:

1. 앱 실행 뒤 `Preparing` 다음 `Phone Required`가 발생하고 카메라 연결이 안 된다고 제보했다.
2. 최신 파일을 확인하고 기록한 뒤 실제 연결 실패 경계를 다시 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7ff58f1724b60`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  - `activePresetId = preset_new-draft-2`
  였다.
- 같은 세션의 `diagnostics/camera-helper-status.json`은
  - `cameraState = error`
  - `helperState = error`
  - `detailCode = camera-connect-timeout`
  - `observedAt = 2026-04-20T07:16:24.1232798+00:00`
  를 남기고 있었다.
- 즉 latest failure는 더 이상 화면 번역만의 문제가 아니라,
  **helper startup connect/open 경계가 실제로 timeout으로 닫히는 상태**였다.

이번 회차 해석:

- booth는 startup failure를 이제 customer 화면에도 반영하고 있었지만,
  helper 기본 connect budget은 여전히 `5초`라 slow startup/open을 false timeout으로 읽을 가능성이 컸다.
- operator 진단도 이 케이스를 `최근 촬영을 세션에 저장하지 못했어요`처럼 읽어,
  실제로는 startup connect failure인데 capture 저장 실패처럼 보일 수 있었다.
- 따라서 이번 시점의 직접 과제는
  - startup connect budget을 실제 현장 속도에 맞게 늘리고
  - timeout이 어느 단계에서 났는지 detail code를 나눠 기록하며
  - operator가 이를 capture failure가 아닌 startup camera failure로 읽게 만드는 것이었다.

이번 회차 수정:

- Canon helper startup connect 기본 timeout을 `5초 -> 15초`로 늘렸다.
- startup timeout은 이제 마지막 진행 단계에 따라
  - `sdk-init-timeout`
  - `session-open-timeout`
  - fallback `camera-connect-timeout`
  으로 나눠 기록한다.
- 같은 timeout family는 supervisor가 기존 `camera-connect-timeout`과 같은 startup failure로 보고 bounded auto-restart 대상으로 다룬다.
- helper는 startup terminal failure가 나면 `camera-helper-events.jsonl`에도 `helper-error`를 남겨,
  status 파일만이 아니라 event 근거도 같이 확인할 수 있게 했다.
- 그리고 startup 단계 전환 자체를 더 추적하기 쉽도록
  `diagnostics/camera-helper-startup.log`에
  `helper-starting -> sdk-initializing/scanning -> session-opening -> timeout/failure`
  progression을 순서대로 append하도록 보강했다.
- operator diagnostics는 no-capture startup timeout family를 더 이상 RAW handoff 저장 실패처럼 설명하지 않고,
  `카메라 연결 시작이 실패했어요` 계열로 분리해 보여 준다.

검증:

- Rust
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test operator_diagnostics operator_diagnostics_describes_startup_connect_failures_without_claiming_capture_saved -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- C#
  - `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745202892851_pAVdAAA7pU`
  - `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter "FullyQualifiedName~TimeoutPolicyTests"`

이번 시점 제품 판단:

1. latest `Preparing -> Phone Required`는 startup camera failure를 다시 반영한 결과였고, 근본 원인은 helper startup connect/open이 실제로 timeout으로 닫히는 경계였다.
2. 이번 패치로 booth는 startup false timeout 가능성을 줄이고, 같은 failure가 나도 어느 단계에서 막혔는지 더 정확히 남긴다.
3. 다음 현장 확인 포인트는 앱 재실행 뒤 같은 조건에서 `camera-connect-timeout` 대신 실제 연결 성공으로 넘어가는지, 실패하더라도 `sdk-init-timeout` / `session-open-timeout`처럼 더 구체적인 이유가 남는지다.

### 2026-04-20 16:48 +09:00 최신 재확인: 이번 `Preparing` 재발은 무기록이 아니라 fresh `session-opening` 반복이라 stale-only 감시로는 끊기지 않았다

사용자 최신 요청:

1. 앱 실행 뒤 다시 `Preparing`이 보이고 카메라 연결이 안 된다고 제보했다.
2. 파일을 확인하고 기록한 뒤, 원인 추적이 어려우면 debug 로그를 더 추가해서라도 해결하라고 요청했다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a80090cacc84ec`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  - `activePresetId = preset_new-draft-2`
  였다.
- 같은 세션의 `diagnostics/camera-helper-status.json`은
  - `cameraState = connecting`
  - `helperState = connecting`
  - `detailCode = session-opening`
  - `cameraModel = Canon EOS 700D`
  - `observedAt = 2026-04-20T07:39:59.8875911+00:00`
  를 남기고 있었다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`는
  - `2026-04-20T07:37:59Z sequence=2 session-opening`
  - `2026-04-20T07:38:25Z sequence=1 sdk-initializing`
  - `2026-04-20T07:38:25Z sequence=2 session-opening`
  - `2026-04-20T07:38:52Z sequence=1 sdk-initializing`
  - `2026-04-20T07:38:52Z sequence=2 session-opening`
  - `2026-04-20T07:39:18Z sequence=1 sdk-initializing`
  - `2026-04-20T07:39:18Z sequence=2 session-opening`
  - `2026-04-20T07:39:45Z sequence=1 sdk-initializing`
  - `2026-04-20T07:39:45Z sequence=2 session-opening`
  패턴을 반복하고 있었다.
- 즉 이번 회차는 helper status가 stale해진 것이 아니라,
  **fresh한 `session-opening`이 반복 재기동되면서 booth가 계속 `Preparing`으로 남는 케이스**였다.

이번 회차 수정:

- helper supervisor는 더 이상 `observedAt` stale 여부만으로 startup stall을 판단하지 않는다.
- 같은 startup phase가 fresh하게 갱신되더라도
  - `helper-starting`
  - `sdk-initializing`
  - `scanning`
  - `session-opening`
  이 일정 시간 이상 지속되면 phase duration으로 stall을 판정한다.
- `session-opening`이 20초 이상 이어지면 `session-open-timeout`으로,
  초기 SDK 단계가 20초 이상 이어지면 `sdk-init-timeout`으로 승격한다.
- 자동 재기동은 기존 정책대로 20초 창에서 1회만 허용하고,
  그 뒤에도 같은 stall이 반복되면 더 이상 `Preparing` 루프를 만들지 않고 안정된 error 상태로 닫힌다.
- 덕분에 최신처럼 status가 계속 신선하게 갱신되는 경우에도 booth가 카메라 연결 정체를 실제 오류로 반영할 수 있다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test operator_diagnostics operator_diagnostics_describes_startup_connect_failures_without_claiming_capture_saved -- --exact`

이번 시점 제품 판단:

1. latest `Preparing` 재발의 직접 원인은 상태 미반영이 아니라, fresh `session-opening` 반복을 stale-only 규칙이 놓치고 있던 점이다.
2. 이번 패치로 booth는 startup 정체를 더 이상 무한 `Preparing`으로 두지 않고, bounded retry 뒤 실제 연결 실패 상태로 전환한다.
3. 이미 추가한 `camera-helper-startup.log` 덕분에 다음 재발 시에도 startup phase가 어디서 반복되는지 즉시 읽을 수 있다.

### 2026-04-21 10:35 +09:00 최신 재확인: startup 전체가 막혀도 phase 전환마다 예산을 다시 잡아 `Preparing`이 이어지고 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했고 여전히 연결 문제가 남아 있다고 보고했다.
2. 최신 로그를 보고 원인을 기록하고, 실제 제품에서 같은 루프가 끝나도록 고쳐 달라고 요청했다.

실제 확인 근거:

- 최신 재현 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83b084a6e28c0`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고, 촬영 단계로 가지 못했다.
- 같은 세션의 `camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = healthy`
  - `detailCode = windows-device-detected`
  - `cameraModel = Canon EOS 700D`
  였다.
- 같은 세션의 `camera-helper-startup.log`는 약 4초 동안 아래 startup family를 반복했다.
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  - `windows-device-detected`

직접 원인:

- 이전 수정으로 `session-opening <-> windows-device-detected` 반복은 같은 stall로 보게 했지만,
  이번 최신 로그에서는 그 사이에 `sdk-initializing`도 다시 끼고 있었다.
- supervisor는 startup phase가 바뀔 때마다 stall 시작 시각을 새로 잡고 있었기 때문에,
  전체로는 오래 준비 완료에 도달하지 못해도 phase 전환만 계속되면 예산이 계속 초기화됐다.
- 즉 이번 회차의 직접 결함은
  **camera startup 전체가 막힌 것을 보지 못하고, 각 phase를 독립 시도로 잘못 쪼개 본 것**이다.

이번 회차 수정:

- helper supervisor는 이제 `sdk-initializing`, `session-opening`, `windows-device-detected`를 startup family 하나로 묶어 본다.
- startup family 안에서 phase가 바뀌어도 최초 stall 시작 시각은 유지한다.
- 따라서 연결이 실제로 준비 완료에 도달하지 못한 채 startup family 안에서만 맴돌면,
  booth는 더 이상 `Preparing`을 계속 유지하지 않고 timeout failure로 승격한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml alternating_startup_phases_keep_the_original_stall_budget`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test operator_diagnostics operator_diagnostics_describes_startup_connect_failures_without_claiming_capture_saved -- --exact`

이번 시점 제품 판단:

1. 최신 재현의 직접 원인은 카메라 자체의 새 실패보다, startup 전체 정체를 단일 budget으로 보지 못한 supervisor 판정 결손이었다.
2. 이번 보강으로 booth는 startup phase가 섞여 반복돼도 `Preparing`에 갇히지 않고 실패 상태로 닫힌다.
3. 다음 실기기 확인 포인트는 같은 조건에서 `Preparing`이 오래 지속되지 않고 `Phone Required` 계열 실패로 승격되는지다.

### 2026-04-21 10:20 +09:00 최신 재확인: `windows-device-detected`가 stall 타이머를 지워 무한 `Preparing`을 다시 만들고 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했지만 여전히 `Preparing`에서 카메라 연결이 끝나지 않는다고 보고했다.
2. 앱 로그를 근거로 원인을 기록하고, 실제 제품에서 같은 루프가 다시 나지 않게 고쳐 달라고 요청했다.

실제 확인 근거:

- 최신 재현 세션은 아래 두 개였다.
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a838c7b6218f3c`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83a1522f88a68`
- 두 세션 모두 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고, 촬영 단계로 진입하지 못했다.
- 최신 `camera-helper-status.json`은
  - `cameraState = connecting`
  - `helperState = connecting`
  - `detailCode = session-opening`
  - `cameraModel = Canon EOS 700D`
  로 끝나 있었다.
- 같은 세션의 `camera-helper-startup.log`는 20초 넘게 아래 두 신호를 반복했다.
  - `detailCode = session-opening`
  - `detailCode = windows-device-detected`

직접 원인:

- helper 자체는 실제로 정상 연결에 도달하지 못하고 있었지만,
  supervisor는 중간에 끼는 `windows-device-detected`를 startup stall의 연속 구간으로 보지 않았다.
- 그 결과 `session-opening`에서 쌓이던 stall 타이머가 매번 초기화됐고,
  booth는 실패로 승격하지 못한 채 다시 `Preparing` 루프에 머물렀다.
- 즉 이번 회차의 핵심 결함은 "camera stall이 없다"가 아니라,
  **stall은 있었지만 supervisor가 bridge status를 정상 전환처럼 처리해 정체 감시를 스스로 해제한 것**이었다.

이번 회차 수정:

- helper supervisor는 이제 `windows-device-detected`를 startup stall 사이에 끼는 bridge status로 취급한다.
- 따라서 `session-opening -> windows-device-detected -> session-opening`처럼 교차해도 같은 startup stall로 이어서 계산한다.
- 이 반복이 budget을 넘기면 booth는 더 이상 무한 `Preparing`으로 남지 않고 `session-open-timeout` 계열의 실패 상태로 승격한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml windows_device_detected_between_session_opening_updates_does_not_reset_stall_tracking`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test operator_diagnostics operator_diagnostics_describes_startup_connect_failures_without_claiming_capture_saved -- --exact`

이번 시점 제품 판단:

1. 오늘 재현의 직접 원인은 helper 연결 실패 자체보다, 그 실패를 `Preparing`에서 끊어 주어야 할 supervisor 판정 결손이었다.
2. 이번 보강으로 booth는 `session-opening`과 `windows-device-detected`가 섞여도 정체를 실제 실패로 승격해 운영자가 루프에 갇히지 않게 한다.
3. 다시 같은 증상이 나오면 `camera-helper-startup.log`에서 두 detail code가 교차하는지만 봐도 동일 계열인지 즉시 판별할 수 있다.

### 2026-04-21 10:42 +09:00 최신 런 디렉토리 재확인: 마지막 stale startup 상태가 `windows-device-detected`면 booth가 다시 `Preparing`에 남고 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 로그를 확인하라고 요청했다.
2. 현재 실행 중인 프로세스는 보지 말고, 이번 런 디렉토리 로그만 분석하라고 제한했다.
3. 원인을 기록하고 같은 증상을 제품에서 다시 막히지 않게 고쳐 달라고 요청했다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83b62d3102e6c`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고, 촬영 단계까지 가지 못했다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = healthy`
  - `detailCode = windows-device-detected`
  로 끝나 있었다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`에는
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  - `windows-device-detected`
  가 교차한 뒤 더 진행하지 못한 기록이 남아 있었다.

직접 원인:

- 이전 수정으로 supervisor의 startup stall budget은 유지되도록 보강됐지만,
  readiness 정규화 계층은 마지막 stale startup 상태가 `windows-device-detected`일 때 이를 startup stall로 인정하지 않았다.
- 그 결과 실제로는 연결 준비가 오래 멈춘 상태여도,
  booth는 최신 stale truth를 `camera-preparing`으로 해석해 다시 `Preparing`에 남았다.
- 즉 이번 회차의 직접 결함은
  **startup stall 자체를 못 찾은 것이 아니라, 마지막 stale detail code가 `windows-device-detected`일 때만 failure 승격 규칙이 비어 있었던 것**이다.

이번 회차 수정:

- readiness 정규화는 이제 stale startup detail code로 `windows-device-detected`도 함께 본다.
- 따라서 최신 런처럼 연결이 `connecting`에 머문 채 마지막 상태가 `windows-device-detected`로 끝나도,
  booth는 더 이상 `Preparing`에 남지 않고 `Phone Required` 계열 실패로 승격한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_windows_device_detected_is_the_last_stale_startup_status`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_helper_startup_status_stays_stale`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`

이번 시점 제품 판단:

1. 최신 런 디렉토리 기준 직접 원인은 카메라 연결 시도 자체보다, 마지막 stale status를 failure로 넘기지 못한 readiness 판정 누락이었다.
2. 이번 보강으로 booth는 startup 단계가 `windows-device-detected`에서 멈춰도 `Preparing`에 갇히지 않는다.
3. 다음 실기기 확인 포인트는 같은 조건에서 더 이상 장시간 `Preparing`이 유지되지 않고 실패 상태로 전환되는지다.

### 2026-04-21 10:55 +09:00 최신 런 디렉토리 재확인: startup loop가 조용히 끊겨도 supervisor가 재시도 루프에만 남을 수 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 로그를 확인하라고 요청했다.
2. 현재 실행 프로세스는 보지 말고, 이번 런 디렉토리 로그만 분석하라고 다시 제한했다.
3. 원인을 기록하고 같은 증상이 다시 `Preparing`으로 남지 않게 고쳐 달라고 요청했다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83c6eed6b7a84`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고, 촬영 단계로 넘어가지 못했다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = healthy`
  - `detailCode = windows-device-detected`
  - `sequence = 21`
  로 끝나 있었다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`는 약 5초 동안
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  이 교차 반복된 뒤 더 이상 후속 상태를 남기지 않았다.
- 같은 런 디렉토리 안에는
  - `camera-helper-events.jsonl`
  - `timing-events.log`
  같은 후속 진행 증거가 없었다.

직접 원인:

- 최신 런 디렉토리만 보면 helper는 startup 연결 루프를 쓰다가 중간에 조용히 멈춘 것으로 읽힌다.
- 현재 supervisor 코드는 살아 있는 helper의 startup stall은 timeout으로 승격하지만,
  startup 중 helper가 먼저 종료되는 경계는 restart budget으로 세지 않아 재시도 루프에만 남을 수 있었다.
- 즉 이번 회차의 직접 결함은
  **startup stall을 못 보는 것이 아니라, startup loop가 조기 종료로 끊기는 경우를 bounded failure로 승격하지 못한 것**이다.

이번 회차 수정:

- helper supervisor는 이제 startup phase 도중 helper가 종료되면 그 역시 startup restart budget으로 계산한다.
- 같은 창 안에서 다시 startup 중 종료되면 더 이상 재시도 루프에만 머물지 않고
  `session-open-timeout` 또는 같은 계열 failure로 승격한다.
- 따라서 최신 런처럼 `windows-device-detected`를 남긴 채 startup loop가 조용히 끊겨도,
  booth는 무한 재시도/무한 `Preparing`로 남지 않는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml startup_exit_during_windows_device_detected_consumes_restart_budget_then_escalates`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml hardware_validation_runner_captures_failure_diagnostics_for_readiness_timeouts -- --exact`

이번 시점 제품 판단:

1. 최신 런 디렉토리 기준 추가 원인은 startup phase 반복 그 자체보다, 그 반복이 조기 종료로 끊길 때 supervisor가 failure로 닫지 못한 점이었다.
2. 이번 보강으로 booth는 startup loop가 timeout이 아니라 exit로 끊겨도 bounded failure로 수렴한다.
3. 다음 실기기 확인 포인트는 같은 조건에서 `Preparing`이 길게 유지되기보다, 재시도 후 곧바로 실패 상태로 닫히는지다.

### 2026-04-21 11:09 +09:00 최신 런 디렉토리 재확인: startup oscillation이 이미 반복 실패 패턴인데도 booth가 20초 budget까지 기다리고 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 런 디렉토리 로그를 보라고 요청했다.
2. 현재 실행 프로세스는 보지 말고, 이번 런 디렉토리만 분석하라고 다시 제한했다.
3. 원인을 기록하고 제품에서 같은 증상을 다시 줄이라고 요청했다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83d3aff931f10`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고, 촬영 단계로 넘어가지 못했다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = connecting`
  - `detailCode = session-opening`
  - `sequence = 15`
  였다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`는 약 4초 안에
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  이 15개 sequence까지 교차 반복된 뒤 끝났다.
- 즉 이번 런은 긴 정체 이전에 이미 startup oscillation이 빠르게 누적된 패턴으로 읽는 것이 맞다.

직접 원인:

- 기존 보강으로 startup stall과 startup 중 조기 종료는 막았지만,
  짧은 시간에 `windows-device-detected <-> session-opening`이 과도하게 반복되는 경우는 여전히
  일반 stall처럼 20초 budget이 지나기 전까지 `Preparing`으로 남을 수 있었다.
- 즉 이번 회차의 직접 결함은
  **이미 deterministic startup failure pattern으로 보이는 빠른 oscillation을, 제품이 너무 늦게까지 “아직 연결 중”으로 취급한 것**이다.

이번 회차 수정:

- helper supervisor는 이제 startup phase에 들어간 뒤 status sequence가 짧은 시간 안에 과도하게 누적되면,
  20초 stall budget을 기다리지 않고 startup timeout failure로 바로 승격한다.
- 따라서 최신 런처럼 약 4초 안에 startup oscillation이 15개 sequence까지 반복되면,
  booth는 더 이상 장시간 `Preparing`에 남지 않고 더 빠르게 실패 상태로 전환된다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml repeated_startup_status_updates_fast_fail_before_the_twenty_second_budget`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml hardware_validation_runner_captures_failure_diagnostics_for_readiness_timeouts -- --exact`

이번 시점 제품 판단:

1. 최신 런 디렉토리 기준 직접 원인은 단순 stall보다, startup oscillation을 실패로 너무 늦게 간주한 판정 기준이었다.
2. 이번 보강으로 booth는 같은 연결 루프를 더 빠르게 끊고 실패 상태로 승격한다.
3. 다음 실기기 확인 포인트는 같은 조건에서 `Preparing`이 20초 가까이 남지 않고 훨씬 빨리 실패 상태로 바뀌는지다.

### 2026-04-21 11:25 +09:00 최신 런 디렉토리 재확인: fresh startup oscillation이면 readiness 자체가 계속 `Preparing`으로 남을 수 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 런 디렉토리 로그를 기준으로 원인을 찾으라고 요청했다.
2. 현재 실행 프로세스는 보지 말고, 이번 런 디렉토리 로그만 분석하라고 다시 제한했다.
3. 원인을 기록하고 문제를 실제로 해결하라고 요청했다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83e1ef6f74400`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고, 촬영 단계로 넘어가지 못했다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = starting`
  - `detailCode = sdk-initializing`
  - `sequence = 36`
  이었다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`는 약 9초 동안
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  이 계속 교차했고, 마지막까지 fresh startup status가 유지됐다.

직접 원인:

- 지금까지의 보강은 supervisor가 timeout/error status를 써 주거나,
  마지막 startup status가 stale로 굳는 경우를 주로 막았다.
- 하지만 최신 런처럼 helper가 fresh startup status를 계속 쓰면,
  readiness 정규화 계층은 이를 여전히 `camera-preparing`/`Preparing`으로 해석할 수 있었다.
- 즉 이번 회차의 직접 결함은
  **startup oscillation이 이미 반복 실패 패턴이어도, 마지막 status가 fresh라는 이유만으로 readiness가 계속 연결 진행 중으로 본 것**이다.

이번 회차 수정:

- readiness는 이제 startup family status가 fresh하더라도,
  세션이 이미 충분히 진행됐고 sequence가 과도하게 누적된 경우 이를 startup oscillation failure로 본다.
- 따라서 최신 런처럼 `sdk-initializing / windows-device-detected / session-opening`이 오래 반복돼 `sequence=36`까지 간 경우,
  booth는 더 이상 `Preparing`에 남지 않고 `Phone Required` 계열 실패로 승격한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_fresh_startup_oscillation_repeats_far_past_session_start`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_windows_device_detected_is_the_last_stale_startup_status`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_helper_startup_status_stays_stale`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml hardware_validation_runner_captures_failure_diagnostics_for_readiness_timeouts -- --exact`

이번 시점 제품 판단:

1. 최신 런 디렉토리 기준 직접 원인은 helper process lifecycle만이 아니라, readiness가 fresh startup oscillation을 실패로 승격하지 못한 점이었다.
2. 이번 보강으로 booth는 fresh status가 계속 찍히더라도 반복 startup loop를 `Preparing`으로 오래 유지하지 않는다.
3. 다음 실기기 확인 포인트는 같은 조건에서 마지막 status가 fresh여도 화면이 `Preparing`에 남지 않고 실패 상태로 전환되는지다.

### 2026-04-21 11:33 +09:00 최신 런 디렉토리 재확인: latest failure는 low-sequence fresh startup family라 기존 oscillation 기준 아래에 숨어 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 런 디렉토리 로그를 확인하라고 요청했다.
2. 현재 실행 프로세스는 보지 말고, 이번 런 디렉토리 로그만 분석하라고 다시 제한했다.
3. 원인을 기록하고 실제 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83e83e5b372f8`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = connecting`
  - `detailCode = session-opening`
  - `sequence = 7`
  이었다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`는
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  이 짧게 반복된 뒤 끝났다.
- 즉 latest failure는 이전처럼 `sequence=15+`까지 간 큰 oscillation이 아니라,
  **세션은 이미 충분히 오래 startup family에 있었지만 마지막 fresh sequence가 7에서 멈춘 케이스**였다.

직접 원인:

- 기존 fresh startup oscillation 승격 기준은 high-sequence case에는 반응했지만,
  latest run처럼 low-sequence fresh startup family에는 아직 너무 관대했다.
- 그 결과 실제로는 startup family에 오래 묶인 같은 실패여도,
  booth는 마지막 fresh status가 `session-opening` / `sdk-initializing`이면 다시 `Preparing`으로 남을 수 있었다.

이번 회차 수정:

- readiness의 fresh startup oscillation 승격 기준을 latest run 수준까지 낮췄다.
- 따라서 세션이 이미 startup budget을 오래 소모했고 fresh startup family가 계속 남아 있으면,
  `sequence=7` 수준이어도 booth는 더 이상 `Preparing`에 머물지 않고 실패 상태로 승격한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_fresh_startup_family_persists_past_session_budget_even_with_low_sequence`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_fresh_startup_oscillation_repeats_far_past_session_start`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_windows_device_detected_is_the_last_stale_startup_status`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_helper_startup_status_stays_stale`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml hardware_validation_runner_captures_failure_diagnostics_for_readiness_timeouts -- --exact`

이번 시점 제품 판단:

1. latest run 기준 직접 원인은 fresh startup oscillation heuristic이 실제 field pattern보다 늦게 동작한 점이었다.
2. 이번 보강으로 booth는 low-sequence fresh startup family도 오래 지속되면 실패로 닫는다.
3. 다음 실기기 확인 포인트는 latest 같은 `sequence=7` 수준의 짧은 startup burst 뒤에도 화면이 `Preparing`에 오래 남지 않는지다.

### 2026-04-21 11:39 +09:00 최신 런 디렉토리 재확인: latest run은 `sequence=20`이어도 startup family fail threshold가 여전히 field wait보다 늦었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 런 디렉토리 로그를 확인하라고 요청했다.
2. 현재 실행 프로세스는 보지 말고, 이번 런 디렉토리 로그만 분석하라고 다시 제한했다.
3. 원인을 기록하고 제품에서 문제를 해결하라고 요청했다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83ed75d7c78cc`였다.
- 같은 세션의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였다.
- 같은 세션의 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = connecting`
  - `helperState = healthy`
  - `detailCode = windows-device-detected`
  - `sequence = 20`
  이었다.
- 같은 세션의 `diagnostics/camera-helper-startup.log`는 약 5초 동안
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  이 20개 sequence까지 반복된 뒤 끝났다.
- 즉 latest run은 startup family가 다시 반복 실패 패턴으로 들어갔지만,
  제품은 아직 too-late threshold 때문에 `Preparing`으로 남을 여지가 있었다.

직접 원인:

- 기존 fresh startup family 승격 기준은 이전보다 개선됐지만,
  latest run처럼 세션이 이미 오래 시작 구간에 묶인 상태에서는 여전히 field 체감보다 늦게 failure로 닫혔다.
- 특히 latest run은 `updatedAt` 기준으로 8초 안팎에서 startup family loop가 반복되고 있었기 때문에,
  이전 12초 기준은 실제 운영 화면에서 `Preparing` 체감을 너무 길게 남길 수 있었다.

이번 회차 수정:

- readiness의 fresh startup family fail age 기준을 `12초`에서 `8초`로 앞당겼다.
- 따라서 latest run처럼 `windows-device-detected / session-opening / sdk-initializing`이 계속 교차하면,
  booth는 더 빨리 `Phone Required` 계열 실패로 전환된다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_low_sequence_startup_family_is_already_over_eight_seconds_old`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_fresh_startup_family_persists_past_session_budget_even_with_low_sequence`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_fresh_startup_oscillation_repeats_far_past_session_start`
- `cargo test --manifest-path src-tauri/Cargo.toml readiness_escalates_when_windows_device_detected_is_the_last_stale_startup_status`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml hardware_validation_runner_captures_failure_diagnostics_for_readiness_timeouts -- --exact`

이번 시점 제품 판단:

1. latest run 기준 직접 원인은 startup family failure를 너무 늦게 `Phone Required`로 넘기던 readiness 타이밍 기준이었다.
2. 이번 보강으로 booth는 같은 증상에서 `Preparing` 체류 시간을 더 짧게 줄인다.
3. 다음 실기기 확인 포인트는 latest와 같은 loop에서 약 8초 전후 안에 실패 상태로 전환되는지다.

### 2026-04-21 12:18 +09:00 최신 런 디렉토리 재확인: 이번 재발은 age threshold보다 먼저 sequence burst가 비정상에 도달했는데 supervisor fast-fail이 너무 늦었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 런 디렉토리 로그만 다시 보라고 요청했다.
2. 현재 실행 프로세스는 보지 말고, [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)를 기준으로 원인을 분류하라고 요청했다.
3. 원인을 기록하고 제품에서 문제를 해결하라고 요청했다.

체크리스트 기준 분류:

- latest run은 여전히 `startup/connect family`다.
- 근거는
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  - startup family detail code 반복
  - 마지막 상태가 `windows-device-detected`
  이 함께 보였기 때문이다.
- 즉 이번 런도 새 detail code bug가 아니라, 같은 startup/connect failure의 또 다른 표면형이다.

실제 확인 근거:

- 최신 런 디렉토리 세션은 계속 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a83ed75d7c78cc`였다.
- `session.json`은
  - `createdAt = 2026-04-21T02:39:07Z`
  - `updatedAt = 2026-04-21T02:39:10Z`
  - `lifecycle.stage = preset-selected`
  - `captures = []`
  였다.
- `camera-helper-startup.log`는 약 5초 동안
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  이 `sequence=20`까지 빠르게 반복됐다.
- 마지막 `camera-helper-status.json`도
  - `cameraState = connecting`
  - `helperState = healthy`
  - `detailCode = windows-device-detected`
  - `sequence = 20`
  으로 끝났다.

직접 원인:

- latest run은 이미 짧은 시간 안에 startup family sequence가 과도하게 누적된 dense burst였다.
- 그런데 supervisor의 fast-fail 기준은 아직 이 밀도를 field 체감보다 늦게 timeout으로 닫고 있었다.
- 그래서 제품은 age threshold를 기다리는 동안 customer-visible `Preparing`에 더 오래 머무를 수 있었다.

이번 회차 수정:

- startup burst fast-fail 기준을 더 앞당겼다.
- 이제 startup family가 약 3초 안에 10 step 이상 누적되면,
  supervisor는 이를 정상 진행이 아니라 dense startup failure로 보고 더 빨리 timeout failure로 닫는다.
- 따라서 latest run처럼 `sequence=20`까지 몰아치는 loop는 더 이상 `Preparing`에 오래 남지 않고 빠르게 실패 상태로 전환된다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml dense_startup_burst_fast_fails_after_ten_repeated_transitions`
- `cargo test --manifest-path src-tauri/Cargo.toml repeated_startup_status_updates_fast_fail_before_the_twenty_second_budget`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`

이번 시점 제품 판단:

1. latest run의 직접 원인은 새 startup family 종류가 아니라, dense burst를 너무 늦게 끊던 supervisor fast-fail 기준이었다.
2. 이번 보강으로 booth는 같은 로그 패턴에서 `Preparing` 체류 시간을 더 줄이고 더 빨리 실패 상태로 닫는다.
3. 다음 실기기 확인 포인트는 latest와 비슷한 5초 안쪽 burst에서도 화면이 더 빨리 `Phone Required` 계열로 전환되는지다.

### 2026-04-21 12:32 +09:00 최신 런 디렉토리 재확인: stale startup failure가 앱 재기동 시 `helper-starting`으로 덮여 다시 `Preparing`으로 낮아질 수 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 앱 로그와 최신 세션 로그를 같이 보라고 요청했다.
2. [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)와 [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)를 참조해 비슷한 과거 해법도 함께 반영하라고 요청했다.
3. 원인을 기록하고 문제를 해결하라고 요청했다.

체크리스트 기준 분류:

- latest run은 여전히 `startup/connect family`다.
- `session.json`은
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  상태였고,
- `camera-helper-status.json`과 `camera-helper-startup.log`는 startup 초반 `sdk-initializing`에서 멈춘 흔적만 남겼다.
- 따라서 이번 건도 helper ready 이후 프런트 fallback 문제가 아니라, startup failure가 안정적으로 닫히지 않는 같은 문제군으로 분류하는 것이 맞다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84082e2726120`였다.
- 같은 세션의 `session.json`은
  - `createdAt = 2026-04-21T03:09:43Z`
  - `updatedAt = 2026-04-21T03:09:45Z`
  - `lifecycle.stage = preset-selected`
  - `captures = []`
  였다.
- 같은 세션의 `camera-helper-startup.log`는
  - `sequence=1`
  - `cameraState = connecting`
  - `helperState = starting`
  - `detailCode = sdk-initializing`
  한 줄만 남았다.
- 마지막 `camera-helper-status.json`도
  - `sequence = 2`
  - `cameraState = connecting`
  - `helperState = starting`
  - `detailCode = sdk-initializing`
  으로 끝났다.
- 반면 최신 앱 로그 후보였던 `com.boothy.desktop\\logs\\Boothy.log`, `com.tauri.dev\\logs\\Boothy.log`에는 이번 세션 ID가 직접 남지 않아,
  latest 증거는 세션 런 디렉토리와 코드 경계 재현으로 확정하는 편이 더 정확했다.

직접 원인:

- 현재 구조에서는 readiness를 읽기 전에 supervisor가 새 helper launch를 준비하면서 `helper-starting` status를 먼저 쓴다.
- 이때 이전 런이 남긴 stale startup family status나 recorded startup timeout도 fresh `helper-starting`으로 덮일 수 있었다.
- 그래서 실제로는 이미 실패로 닫혀야 하는 startup/connect failure가 앱 재기동 또는 helper 재bootstrap 순간마다 다시 `Preparing`으로 낮아질 여지가 있었다.
- 이 패턴은 `camera-helper-troubleshooting-history.md`의 "host truth가 맞더라도 마지막 적용 경계에서 `Preparing`으로 다시 낮아질 수 있다"는 경고와도 맞는다.

이번 회차 수정:

- supervisor는 이제 새 helper를 띄우기 직전에,
  기존 status가 이미 startup timeout이거나 stale startup family면 이를 `helper-starting`으로 덮어쓰지 않는다.
- 따라서 이전 런이 남긴 startup failure truth는 다음 bootstrap 시도 전까지 유지되고,
  booth는 이를 다시 fresh `Preparing`으로 오인하지 않는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml starting_helper_status_does_not_overwrite_a_stale_startup_failure_status`
- `cargo test --manifest-path src-tauri/Cargo.toml starting_helper_status_does_not_overwrite_a_recorded_startup_timeout`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`

이번 시점 제품 판단:

1. latest run의 직접 원인은 새 startup 코드가 아니라, 이미 실패로 봐야 할 startup truth를 bootstrap이 다시 `Preparing` 쪽으로 덮던 supervisor 경계였다.
2. 이번 보강으로 booth는 앱 재기동이나 helper 재bootstrap이 있어도 같은 startup failure를 다시 `Preparing`으로 숨기지 않는다.
3. 다음 실기기 확인 포인트는 같은 조건에서 앱을 다시 띄워도 화면이 오래 `Preparing`으로 돌아가지 않고 failure 상태를 유지하거나 더 빠르게 닫히는지다.

### 2026-04-21 12:44 +09:00 최신 런 디렉토리 재확인: `sequence=31` dense startup loop는 기존 8초 기준 아래에서 여전히 `Preparing`으로 남을 수 있었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 앱 로그와 세션 로그를 다시 보라고 요청했다.
2. [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)와 [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)를 참조해 같은 문제군 해법을 적용하라고 요청했다.
3. 원인을 기록하고 문제를 해결하라고 요청했다.

체크리스트 기준 분류:

- latest run은 다시 `startup/connect family`다.
- 근거는
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  - startup family detail code 반복
  - 마지막 상태가 `session-opening`
  이 함께 보였기 때문이다.
- helper가 `ready/healthy`인 상태는 아니므로, 이번 건은 frontend fallback보다 startup family bounded failure 문제로 보는 편이 맞다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8410dd4b9fc4c`였다.
- 같은 세션의 `session.json` 기준으로
  - `createdAt = 2026-04-21T03:19:40Z`
  - `updatedAt = 2026-04-21T03:19:42Z`
  - `lifecycle.stage = preset-selected`
  - `captures = []`
  였다.
- 같은 세션의 `camera-helper-startup.log`는 약 8초 동안
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
  을 반복했고, 마지막은 `sequence = 31`, `detailCode = session-opening`이었다.
- 같은 세션의 `camera-helper-status.json` 마지막 상태도
  - `cameraState = connecting`
  - `helperState = connecting`
  - `detailCode = session-opening`
  - `sequence = 31`
  이었다.
- 현재 저장된 최신 글로벌 앱 로그 후보에서는 이번 세션 ID가 직접 확인되지 않아,
  latest failure는 세션 런 디렉토리와 host readiness 재현 테스트로 확정하는 편이 더 정확했다.

직접 원인:

- readiness는 fresh startup family를 실패로 승격할 때
  저밀도 loop와 고밀도 loop에 같은 age 기준을 사용하고 있었다.
- 그 결과 latest처럼 `sequence=31`까지 빠르게 누적된 dense startup loop도,
  readiness 기준상 일정 시점 전까지는 여전히 `Preparing`으로 남을 수 있었다.
- 즉 이번 재발은 새 detail code 문제가 아니라,
  **고밀도 startup oscillation을 더 빨리 닫아야 하는 product rule이 readiness 쪽에 아직 부족했던 케이스**다.

이번 회차 수정:

- readiness는 이제 dense startup family를 별도로 본다.
- `sequence`가 충분히 높게 누적된 startup family는,
  기존 저밀도 loop보다 더 짧은 age 기준에서 `Phone Required` 계열 실패로 승격한다.
- 따라서 latest 같은 `sequence=31` loop는 8초를 다 쓰기 전에도 더 빨리 `Preparing`을 벗어난다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_dense_startup_family_is_already_over_five_seconds_old -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_low_sequence_startup_family_is_already_over_eight_seconds_old -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness startup_connect_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness session_open_timeout_routes_to_phone_required_before_first_capture -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml helper_supervisor -- --nocapture`

이번 시점 제품 판단:

1. latest run의 직접 원인은 startup family 자체가 아니라, dense loop를 저밀도 loop와 같은 시간 예산으로 보던 readiness 최종 승격 기준이었다.
2. 이번 보강으로 booth는 `sequence=31` 같은 고밀도 startup loop에서 더 빨리 `Preparing`을 벗어나 failure 상태로 닫힌다.
3. 다음 실기기 확인 포인트는 latest 같은 반복 패턴에서 약 5초 전후부터 더 이상 장시간 `Preparing`이 유지되지 않는지다.

### 2026-04-21 14:35 +09:00 최신 앱 재검증: 이번 `Phone Required`는 startup/connect family가 아니라 retryable 셔터 실패 `capture-trigger-failed(0x00000002)` 과승격이었다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 로그를 기준으로 원인을 다시 확인하라고 요청했다.
2. [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)와 [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)를 참조해, 과거 유효했던 해법과 같은 축으로 해결하라고 요청했다.
3. 결과를 기록하고 실제 재발 경로를 차단하라고 요청했다.

체크리스트 기준 분류:

- latest run은 `startup/connect family`가 아니다.
- 근거는 같은 세션에서 helper가 최종적으로 `ready/healthy`로 회복했기 때문이다.
- 문서 기준으로 이 경우는 startup bounded failure보다, **retryable capture failure를 host가 `phone-required`로 과승격한 경로**로 분리해 보는 편이 맞다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84612e5fc2804`였다.
- 같은 세션 `session.json`은
  - `lifecycle.stage = phone-required`
  - `captures = []`
  로 닫혀 있었다.
- 하지만 같은 세션 `diagnostics/camera-helper-status.json` 마지막 상태는
  - `cameraState = ready`
  - `helperState = healthy`
  - `detailCode = camera-ready`
  였다.
- 같은 세션 `diagnostics/camera-helper-events.jsonl`에는
  - `capture-accepted`
  - `recovery-status(detailCode=capture-trigger-failed)`
  - `helper-error(detailCode=capture-trigger-failed, message=셔터 명령을 보낼 수 없었어요: 0x00000002)`
  가 순서대로 남아 있었다.

직접 원인:

- helper가 `0x00000002` 셔터 실패를 recovery가 필요한 치명 오류로 올리고 있었다.
- 그러면 host는 `recovery-status`를 먼저 보고 즉시 `phone-required`로 세션을 잠근다.
- 이후 helper가 다시 `camera-ready`로 회복해도, 이번 코드값은 기존 retryable legacy 목록에 없어서 세션이 자동으로 풀리지 않았다.

이번 회차 수정:

- helper는 이제 `capture-trigger-failed(0x00000002)`를 recovery-required가 아닌 retryable 셔터 실패로 기록한다.
- host도 legacy `capture-trigger-failed + 0x00000002` 흔적을 재시도 가능 오류로 취급한다.
- 따라서 같은 실패가 다시 나도 즉시 `Phone Required`로 잠그지 않고, 고객 화면은 재시도 가능한 촬영 상태로 남는다.
- 이전 버전이 남긴 같은 유형의 `phone-required`도 helper truth가 `ready/healthy`이면 `capture-ready`로 풀린다.

검증:

- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter "BuildCaptureTriggerException_treats_internal_error_as_retryable_without_recovery|EnsureConnectedAsync_keeps_the_helper_loop_live_while_connect_attempt_runs|EnsureConnectedAsync_escalates_to_an_explicit_error_after_connect_timeout|EnsureConnectedAsync_runs_the_connect_attempt_on_an_sta_thread|PumpEvents_does_not_touch_the_sdk_before_the_camera_session_is_open|ForceCaptureTimeoutIfStuck_fails_an_orphaned_active_capture|HandleObjectEvent_queues_raw_download_without_blocking_the_helper_loop"`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_focus_is_not_locked -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_focus_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest `Phone Required`의 직접 원인은 카메라 연결 실패가 아니라, 회복 가능한 셔터 실패를 치명 오류로 저장하던 경계였다.
2. 이번 보강으로 booth는 같은 `0x00000002` 실패에서 세션을 즉시 잠그지 않고 재시도 가능한 상태를 유지한다.
3. 다음 실기기 확인 포인트는 같은 조건에서 실패가 나더라도 바로 `Phone Required`로 굳지 않고, 다시 촬영 가능한 상태로 남는지다.

### 2026-04-21 15:10 +09:00 최신 앱 재검증: 이번 재발은 세션 잠금이 아니라 retryable 셔터 실패 뒤 helper status가 `camera-ready`로 복귀하지 않던 잔여 상태였다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 로그와 과거 troubleshooting 문서를 다시 보라고 요청했다.
2. 이번에도 같은 문제군인지 먼저 분류하고, 실제 재발 축에 맞는 해법으로 정리하라고 요청했다.

체크리스트 기준 분류:

- latest run은 startup/connect family가 아니다.
- startup은 정상으로 닫혔고, capture 직후 retryable 셔터 실패가 난 뒤 helper가 `ready/healthy`로 남아 있었다.
- 따라서 이번 건은 startup bounded failure가 아니라, **retryable capture failure 이후 helper status truth 정리 문제**로 보는 편이 맞다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a07a464f044`였다.
- 같은 세션 `session.json`은
  - `lifecycle.stage = preset-selected`
  - `captures = []`
  로 남아 있었다.
- 같은 세션 `camera-helper-startup.log`는
  - `sdk-initializing`
  - `session-opening`
  - `session-opened`
  - `camera-ready`
  뒤에
  - `capture-trigger-failed`
  를 남겼다.
- 같은 세션 `camera-helper-events.jsonl`에는
  - `capture-accepted`
  - `helper-error(detailCode=capture-trigger-failed, message=0x00000002)`
  만 있었고, `recovery-status`는 없었다.
- 그런데 같은 세션 `camera-helper-status.json` 마지막 상태는
  - `cameraState = ready`
  - `helperState = healthy`
  - `detailCode = capture-trigger-failed`
  였다.

직접 원인:

- 지난 회차 수정으로 이 실패는 더 이상 세션을 `phone-required`로 잠그지 않게 됐다.
- 하지만 helper snapshot 정리 단계가 retryable 실패 뒤에도 마지막 실패 detail code를 그대로 남겨,
  최신 status와 startup log가 실제 회복 상태를 정확히 반영하지 못했다.
- 그래서 제품은 재시도 가능 상태인데도, 최신 진단 증거만 보면 아직 실패가 계속 유지되는 것처럼 읽히는 잔여 inconsistency가 남아 있었다.

이번 회차 수정:

- retryable capture failure가 끝난 뒤 helper가 `ready/healthy`로 복귀하면,
  helper status detail code도 `camera-ready`로 되돌리도록 맞췄다.
- 실제 실패 원인은 `helper-error` event에 그대로 남기고,
  live status는 현재 카메라가 다시 촬영 가능한 상태라는 truth를 우선 반영한다.

검증:

- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter "ClearCaptureContext_restores_camera_ready_after_retryable_failure|BuildCaptureTriggerException_treats_internal_error_as_retryable_without_recovery|EnsureConnectedAsync_keeps_the_helper_loop_live_while_connect_attempt_runs|EnsureConnectedAsync_escalates_to_an_explicit_error_after_connect_timeout|EnsureConnectedAsync_runs_the_connect_attempt_on_an_sta_thread|PumpEvents_does_not_touch_the_sdk_before_the_camera_session_is_open|ForceCaptureTimeoutIfStuck_fails_an_orphaned_active_capture|HandleObjectEvent_queues_raw_download_without_blocking_the_helper_loop"`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest 재발의 핵심은 startup 실패가 아니라, retryable 셔터 실패 뒤 helper status truth가 덜 정리되던 잔여 문제였다.
2. 이번 보강으로 최신 helper status는 회복 후 다시 `camera-ready`를 기록한다.
3. 다음 실기기 확인 포인트는 같은 실패가 나더라도, operator/customer 쪽 진단이 계속 실패 코드에 머물지 않고 다시 촬영 가능 상태로 복귀하는지다.

### 2026-04-21 15:25 +09:00 최신 앱 재검증: latest는 startup/connect 재발이 아니라 `camera-ready` 직후 첫 촬영 허용이 너무 빨랐던 경계였다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 최신 세션과 지정 문서를 다시 보고 해결하라고 요청했다.
2. 같은 문제군이면 기존 해법을 참고하되, 실제 latest evidence에 맞는 축으로 정리하라고 요청했다.

체크리스트 기준 분류:

- latest run은 startup/connect family가 아니다.
- startup은 정상으로 `camera-ready`까지 닫혔고, helper status 최종 상태도 `camera-ready`였다.
- 따라서 이번 건은 startup failure가 아니라, **startup 직후 첫 촬영 허용 시점이 너무 빨라 생긴 retryable first-shot trigger failure**로 분리하는 편이 맞다.

실제 확인 근거:

- 최신 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a6d28af1130`였다.
- 같은 세션 `camera-helper-startup.log`는
  - `sdk-initializing`
  - `session-opening`
  - `camera-ready`
  로 빠르게 정상 복귀했다.
- 같은 세션 `camera-helper-status.json` 마지막 상태도
  - `cameraState = ready`
  - `helperState = healthy`
  - `detailCode = camera-ready`
  였다.
- 그런데 같은 세션 `camera-helper-events.jsonl`에는
  - `capture-accepted`
  - `helper-error(detailCode=capture-trigger-failed, message=0x00000002)`
  가 바로 뒤따랐다.
- 직전 재현 세션 `session_000000000018a84a07a464f044`도 같은 `0x00000002`였고,
  둘 다 `camera-ready` 직후 거의 바로 첫 셔터가 들어간 패턴이었다.

직접 원인:

- 세션은 startup 직후 낮은 sequence의 첫 `camera-ready`를 곧바로 촬영 가능으로 열고 있었다.
- 실장비에서는 이 very-early ready 구간에서 첫 셔터가 아직 불안정하게 실패할 수 있었고,
  latest `0x00000002`는 그 표면형으로 보는 편이 가장 잘 맞았다.
- 즉 이번 남은 문제는 helper truth나 `phone-required` 잠금이 아니라,
  **첫 ready를 너무 빨리 믿고 첫 촬영을 허용한 readiness timing 경계**였다.

이번 회차 수정:

- host readiness는 이제 startup 직후의 낮은 sequence `camera-ready`를 아주 짧은 안정화 창 동안은 `Preparing`으로 유지한다.
- 안정화 창이 지나면 정상 `Ready`로 풀린다.
- retryable trigger failure 자체는 계속 `phone-required`가 아닌 재시도 경로로 남긴다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_holds_the_first_camera_ready_for_a_brief_startup_stabilization_window -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_the_first_camera_ready_after_the_startup_stabilization_window -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest 재발의 핵심은 startup failure가 아니라, startup 직후 첫 capture enable이 너무 빨랐던 경계였다.
2. 이번 보강으로 booth는 첫 `camera-ready` 직후 아주 짧은 구간을 `Preparing`으로 유지해, 같은 first-shot `0x00000002` 가능성을 줄인다.
3. 다음 실기기 확인 포인트는 preset 선택 직후 첫 촬영 버튼이 아주 짧게만 준비 중에 머문 뒤 열리고, 바로 누를 때 같은 `0x00000002`가 줄어드는지다.

### 2026-04-21 15:35 +09:00 최신 앱 재검증: latest는 same first-shot 경계였고, 기존 안정화 창이 `sequence` 가정 때문에 너무 좁았다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest session과 과거 troubleshooting 문서를 다시 참조해 해결하라고 요청했다.
2. 이번 latest가 방금 수정의 연장선인지 실제 evidence로 다시 정리하라고 요청했다.

체크리스트 기준 분류:

- latest run도 startup/connect family는 아니다.
- startup은 정상으로 닫혔고, helper 최종 status도 `camera-ready`였다.
- 즉 이번 건도 **startup 직후 first-shot trigger failure 축**으로 보는 편이 맞다.

실제 확인 근거:

- latest 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b21e1691d9c`였다.
- 같은 세션 `camera-helper-startup.log`는
  - `sequence=3 camera-ready`
  - `sequence=8 camera-ready`
  로 빠르게 안정화되는 것처럼 보였다.
- 그런데 같은 세션 `camera-helper-events.jsonl`에는
  - `capture-accepted`
  - `helper-error(detailCode=capture-trigger-failed, message=0x00000002)`
  가 남았다.
- `session.json`은 여전히 `preset-selected`, `captures=[]`였고
  helper 최종 `camera-helper-status.json`은 다시 `camera-ready`로 회복했다.

직접 원인:

- 직전 회차의 안정화 창은 startup 직후 낮은 `sequence` ready만 잠깐 막는 가정에 묶여 있었다.
- 실제 latest에서는 첫 촬영 시점에 helper `sequence`가 이미 `8`까지 올라가 있었지만,
  여전히 preset 선택 직후 very-early first shot 구간이었다.
- 따라서 same bug family인데도 `sequence` 기준이 너무 좁아 latest를 놓쳤다.

이번 회차 수정:

- first-shot stabilization은 이제 helper `sequence`가 아니라,
  `preset-selected` 직후 세션 age 기준으로 짧게 유지한다.
- 즉 preset 선택 뒤 처음 몇 초 동안은 helper가 `camera-ready`여도 고객 화면은 잠깐 `Preparing`을 유지하고,
  그 창이 지나면 정상 `Ready`로 풀린다.
- retryable trigger failure는 계속 `phone-required`로 잠그지 않는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_holds_the_first_camera_ready_for_a_brief_preset_selection_window -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_the_first_camera_ready_after_the_preset_selection_window -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest 재발은 새 문제군이 아니라, same first-shot timing bug를 `sequence` 기준이 좁아 놓친 케이스였다.
2. 이번 보강으로 first-shot holdoff는 preset 선택 직후 세션 age 기준으로 동작한다.
3. 다음 실기기 확인 포인트는 preset 선택 직후 약간 더 안정화 시간이 보이더라도, 바로 누를 때 같은 `0x00000002` first-shot failure가 줄어드는지다.

### 2026-04-21 15:45 +09:00 최신 앱 재검증: latest도 same first-shot failure였고, 3초 창은 실제 하드웨어에서 아직 짧았다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest session과 과거 문서를 다시 보고 해결하라고 요청했다.
2. latest가 방금 넣은 preset-selection holdoff 밖에서 재발한 것인지 확인하라고 요청했다.

체크리스트 기준 분류:

- latest run 역시 startup/connect family는 아니다.
- helper는 정상으로 `camera-ready`에 도달했고 최종 상태도 healthy/ready였다.
- 이번 latest도 **same first-shot trigger failure family**로 묶는 편이 맞다.

실제 확인 근거:

- latest 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b9b92af73ac`였다.
- 같은 세션 `camera-helper-startup.log`는
  - `06:33:07 camera-ready`
  - `06:33:09 camera-ready`
  로 보였다.
- 같은 세션 `camera-helper-events.jsonl`에는
  - `capture-accepted`
  - `helper-error(detailCode=capture-trigger-failed, message=0x00000002)`
  가 남았다.
- `session.json`은 `preset-selected`, `captures=[]`였고
  helper 최종 `camera-helper-status.json`은 다시 `camera-ready`로 회복했다.
- 즉 세션이 잠기거나 startup에서 멈춘 게 아니라, first-shot failure만 반복된 latest였다.

직접 원인:

- 직전 회차 보강으로 first-shot holdoff를 preset selection 직후 세션 age 기준으로 바꿨지만,
  실제 latest는 약 3초 부근에서도 여전히 같은 `0x00000002`가 날 수 있었다.
- 즉 문제군 분류는 맞았지만, **3초 창 자체가 실장비 기준으로 아직 짧았다.**

이번 회차 수정:

- first-shot stabilization window를 `3초 -> 5초`로 늘렸다.
- 그래서 preset 선택 직후 첫 촬영 버튼은 조금 더 늦게 열리지만,
  same first-shot failure가 반복되는 범위를 더 많이 흡수한다.
- retryable trigger failure를 `phone-required`로 잠그지 않는 동작은 유지한다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_holds_the_first_camera_ready_for_a_brief_preset_selection_window -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_the_first_camera_ready_after_the_preset_selection_window -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest는 새 bug family가 아니라, same first-shot failure가 3초 부근까지 살아 있음을 보여 준 증거였다.
2. 이번 보강으로 first-shot holdoff는 5초 창으로 넓어졌다.
3. 다음 실기기 확인 포인트는 preset 선택 직후 첫 촬영 버튼이 더 늦게 열리더라도, 같은 `0x00000002` first-shot failure가 실제로 사라지는지다.

### 2026-04-21 16:06 +09:00 최신 앱 로그 재검토: latest는 여전히 same first-shot `0x00000002`였고, holdoff만으로는 부족해 bounded auto-retry를 넓혔다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest 로그와 과거 startup/connect troubleshooting 문서를 다시 보고 해결하라고 요청했다.
2. 같은 문제가 왜 계속 반복되는지도 제품 관점에서 설명 가능한 형태로 정리하라고 요청했다.

체크리스트 기준 분류:

- latest run 둘 다 startup/connect family는 아니다.
- helper는 정상으로 `camera-ready`에 도달했고, 세션도 `preset-selected`에서 첫 촬영 직전까지 정상으로 유지됐다.
- 따라서 이번 latest도 **same first-shot internal trigger failure family**로 분류하는 편이 맞다.

실제 확인 근거:

- latest 세션은
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84c7a2e26fe70`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84ca0e1e4dd6c`
  였다.
- 두 세션 모두 `session.json`은
  - `lifecycle.stage = preset-selected`
  - `captures = []`
  였다.
- 두 세션 모두 `camera-helper-startup.log`는 빠르게 `camera-ready`로 닫혔고,
  마지막 `camera-helper-status.json`도 `ready / healthy / camera-ready`였다.
- 그런데 `camera-helper-events.jsonl`에는 공통으로
  - `capture-accepted`
  - `helper-error(detailCode=capture-trigger-failed, message=셔터 명령을 보낼 수 없었어요: 0x00000002)`
  만 남았다.
- 즉 startup이 아니라, 첫 셔터 순간의 same internal trigger failure만 남아 있는 latest였다.

직접 원인:

- 직전까지의 해법은 `camera-ready`를 너무 빨리 열지 않게 하는 holdoff 쪽에 치우쳐 있었다.
- 하지만 latest evidence는 **holdoff를 지나도 여전히 same first-shot `0x00000002`가 날 수 있다**는 쪽으로 읽혔다.
- 그래서 같은 문제가 계속 다른 것처럼 보인 이유는 startup/connect bug가 되살아난 게 아니라,
  **startup truth 문제를 정리할수록 그 뒤에 숨어 있던 첫 셔터 internal trigger 경계가 계속 표면으로 드러난 것**이었다.

이번 회차 수정:

- first-shot `capture-trigger-failed(0x00000002)`는 이제 host가 `1회`가 아니라 `최대 2회`까지 bounded auto-retry 한다.
- 각 retry 사이에는 짧은 안정화 간격을 두고, 그래도 닫히지 않으면 다시 retryable 상태로만 남기도록 유지한다.
- 즉 제품은 같은 first-shot 흔들림을 더 넓게 흡수하되, 무한 재시도나 즉시 `Phone Required`로는 가지 않는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_once -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_twice_before_escalating -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest 재발은 startup/connect 재발이 아니라, same first-shot `0x00000002` family가 아직 남아 있다는 증거였다.
2. 이번 보강으로 booth는 첫 셔터 internal trigger failure를 한 번 더 넓게 흡수하고, bounded auto-retry 범위도 `2회`까지 가진다.
3. 다음 실기기 확인 포인트는 preset 선택 직후 첫 촬영에서 같은 오류가 나더라도, 고객 화면이 바로 실패로 끝나지 않고 그대로 촬영이 이어지는지다.

### 2026-04-21 16:16 +09:00 최신 앱 재검토: latest는 auto-retry 재발이 아니라 같은 helper session 위에서 `0x00000002`가 세 번 반복된 케이스였고, 다음 해법은 retry 사이 helper reconnect였다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest 로그와 과거 troubleshooting 문서를 다시 보고 해결하라고 요청했다.
2. 이번에도 같은 문제군인지 실제 evidence로 분리해 달라고 요청했다.

체크리스트 기준 분류:

- latest run은 startup/connect family가 아니다.
- helper는 빠르게 `camera-ready`까지 올라왔고, startup log도 그 뒤로 healthy ready를 계속 남겼다.
- 따라서 이번 latest도 **same first-shot internal trigger failure family**로 묶는 편이 맞다.

실제 확인 근거:

- latest 세션은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84d8f5e25a0fc`였다.
- 같은 세션 `session.json`은
  - `lifecycle.stage = preset-selected`
  - `captures = []`
  였다.
- 같은 세션 `camera-helper-startup.log`는
  - `07:08:55 camera-ready`
  - `07:08:58 camera-ready`
  - `07:09:00 camera-ready`
  - `07:09:03 camera-ready`
  로 이어졌다.
- 같은 세션 `camera-helper-events.jsonl`에는
  - 첫 요청 `capture-accepted -> capture-trigger-failed(0x00000002)`
  - 두 번째 요청 `capture-accepted -> capture-trigger-failed(0x00000002)`
  - 세 번째 요청 `capture-accepted -> capture-trigger-failed(0x00000002)`
  가 연속으로 남았다.
- 같은 세션 `timing-events.log`에도
  - `request-capture`
  - `request-capture-auto-retry attempt=1`
  - `request-capture`
  - `request-capture-auto-retry attempt=2`
  - `request-capture`
  가 남아 있었다.

직접 원인:

- 이전 회차 보강으로 host auto-retry 자체는 실제로 동작했다.
- 하지만 latest evidence는 **세 번의 셔터 시도가 모두 같은 helper session 위에서 반복된 것**으로 읽혔다.
- 즉 남아 있던 문제는 retry count가 아니라, `0x00000002` 뒤 helper가 같은 세션을 그대로 유지해 다음 retry가 실질적으로 같은 실패 조건에서 다시 실행된 점이었다.

이번 회차 수정:

- helper는 이제 `capture-trigger-failed(0x00000002)` 뒤 즉시 `camera-ready`를 유지하지 않고,
  내부 카메라 세션을 한 번 정리한 뒤 `reconnect-pending`으로 복귀하게 했다.
- host는 다음 auto-retry를 보내기 전에 helper가 다시 `ready / healthy`로 돌아왔는지 짧게 기다린다.
- 제품 기준으로는 “같은 실패를 같은 세션에 세 번 반복”하던 경계를 끊고, retry가 실제로 새 카메라 세션에서 일어나게 맞춘 것이다.

검증:

- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter "BuildCaptureTriggerException_treats_internal_error_as_retryable_without_recovery|ClearCaptureContext_restores_camera_ready_after_retryable_failure|ClearCaptureContext_marks_internal_trigger_failure_for_reconnect_before_retry"`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_once -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_twice_before_escalating -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. latest는 startup/connect 재발이 아니라, same helper session에 남아 있던 first-shot `0x00000002` 반복 문제였다.
2. 이번 보강으로 auto-retry는 이제 같은 세션 재반복이 아니라 helper reconnect 뒤에만 이어진다.
3. 다음 실기기 확인 포인트는 첫 촬영에서 같은 흔들림이 나더라도, helper가 다시 준비 상태로 돌아온 뒤 실제 저장까지 닫히는지다.

### 2026-04-21 16:31 +09:00 최신 앱 재검토: reconnect 뒤 same first-shot 실패는 여전히 보였고, 그 다음 latest는 helper가 request 자체를 소비하지 못한 stall로 좁혀졌다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest 로그와 기존 startup/connect troubleshooting 문서를 다시 참조해 해결하라고 요청했다.
2. 같은 family인지, 아니면 다른 축으로 옮겨졌는지 실제 증거로 다시 정리하라고 요청했다.

체크리스트 기준 분류:

- latest 두 세션 중 `session_000000000018a84e23aa4303f0`은 startup/connect family가 아니다.
- 가장 최신 `session_000000000018a84e63c502b0d8`도 startup status 자체는 정상 ready였고, 핵심은 request 이후 helper progress가 없었다.
- 따라서 이번 latest는 startup bounded failure라기보다, **capture request consumption stall**로 보는 편이 맞다.

실제 확인 근거:

- 세션 `session_000000000018a84e23aa4303f0`에서는
  - `capture-accepted -> capture-trigger-failed(0x00000002)`가 3번 반복됐다.
  - `camera-helper-startup.log`는 각 retry 사이에 다시 `sdk-initializing -> session-opening -> camera-ready`를 남겼다.
  - 즉 helper reconnect 자체는 실제로 동작했다.
- 가장 최신 세션 `session_000000000018a84e63c502b0d8`에서는
  - `session.json` 최종 stage가 `phone-required`
  - `timing-events.log`에는 `request-capture` 1건만 존재
  - `camera-helper-events.jsonl` 자체가 없었고
  - `camera-helper-processed-request-ids.txt`도 없었다.
  - 반면 마지막 `camera-helper-status.json`은 직전 시점의 `ready / healthy / camera-ready`였다.
- 즉 latest는 helper가 요청을 거절한 게 아니라, **요청을 읽었다는 흔적 자체가 없이 50초 뒤 timeout으로 phone-required에 떨어진 케이스**였다.

직접 원인:

- 직전 회차 보강으로 internal trigger failure 뒤 helper reconnect는 실제로 붙었다.
- 하지만 latest evidence는 그 다음 경계가 남아 있음을 보여 줬다.
- helper가 준비 상태를 마지막으로 남긴 뒤 request 소비 loop가 멎으면, host는 기존에는 그 사실을 복구하지 못하고 capture timeout으로만 닫았다.

이번 회차 수정:

- host는 이제 capture timeout 전에, 해당 request가 helper에 한 번도 소비되지 않았고 helper status도 fresh하지 않으면
  helper stall로 보고 한 번 강제로 재기동을 건다.
- 그 뒤 helper가 다시 fresh ready로 돌아오면 **같은 request를 다시 기다려** 저장 경계를 닫을 기회를 한 번 더 준다.
- 기존 first-shot internal trigger auto-retry와 reconnect 보강은 그대로 유지한다.

검증:

- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter "BuildCaptureTriggerException_treats_internal_error_as_retryable_without_recovery|ClearCaptureContext_restores_camera_ready_after_retryable_failure|ClearCaptureContext_marks_internal_trigger_failure_for_reconnect_before_retry"`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_restarts_helper_once_when_the_request_was_never_consumed -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_once -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_twice_before_escalating -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_shutter_trigger_internal_error_occurs -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_retryable_internal_trigger_failure_recovers -- --exact`

이번 시점 제품 판단:

1. reconnect 보강 뒤 same first-shot failure는 여전히 남아 있었지만, 그 다음 latest는 더 이상 같은 세션 재반복이 아니라 request-consumption stall로 축이 옮겨졌다.
2. 이번 보강으로 booth는 helper가 request를 읽지 못한 채 멎어도, 즉시 timeout으로 세션을 잠그기 전에 한 번 더 복구를 시도한다.
3. 다음 실기기 확인 포인트는 request 직후 멈춘 것처럼 보여도, 잠깐 뒤 실제로 capture-accepted/file-arrived까지 이어지는지다.

### 2026-04-21 16:39 +09:00 최신 앱 재검토: 이번 `session-open-failed` 재발은 카메라 startup/connect family였지만, 직접 오염원은 직전 Rust 검증이 남긴 실제 helper 프로세스였다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest 로그와 기존 startup/connect troubleshooting 문서를 다시 참조해 해결하라고 요청했다.
2. 왜 잘 되던 기능에서 같은 종류의 문제가 계속 반복돼 보이는지 이유까지 설명해 달라고 요청했다.

체크리스트 기준 분류:

- latest `session_000000000018a84ef48af5416c`는 startup/connect family가 맞다.
- `captures = []`, `lifecycle.stage = preset-selected`, latest helper truth가 `session-opening`에 머문 채 반복됐고,
  hardware validation snapshot은 이미 `reasonCode = phone-required`로 묶여 있었다.
- 따라서 이번 런은 새로운 capture-after-startup 문제가 아니라,
  **startup/connect family의 또 다른 표면형**으로 분류하는 편이 맞다.

실제 확인 근거:

- latest session `camera-helper-startup.log`는
  `sdk-initializing -> session-opening -> session-open-failed`를 약 3초 간격으로 3번 반복했다.
- `failure-diagnostics.json`은 같은 시각
  - `customerState = Phone Required`
  - `reasonCode = phone-required`
  - `liveCaptureTruth.detailCode = session-opening`
  - latest helper error `detailCode = session-open-failed`
  를 함께 남겼다.
- 동시에 머신의 실행 중 프로세스를 확인하면,
  **앱 세션이 아니라 임시 test runtime root(`boothy-capture-capture-request-unconsumed-helper-stall-...`)에 묶인 `canon-helper.exe`**가 살아 있었다.
- 즉 latest 앱 helper는 카메라 startup을 반복했지만,
  직전 검증이 남긴 실제 helper가 카메라 세션을 계속 잡고 있어 `EdsOpenSession(...)` 충돌성 `session-open-failed`를 유발한 것으로 보는 편이 증거와 가장 잘 맞는다.

직접 원인:

- 이번 회차의 현장 재발은 제품 로직이 같은 경계를 다시 잘못 번역한 것이 아니라,
  **직전 Rust 회귀 테스트가 실제 helper 프로세스를 clean shutdown 없이 남긴 운영 오염**이었다.
- 이 stray helper는 temp runtime root에 묶여 있어 session 파일만 보면 보이지 않았고,
  그 결과 현장에서는 마치 startup/connect 버그가 다시 되살아난 것처럼 보였다.

이번 회차 수정:

- 해당 회귀 테스트는 이제 시작 전에도 helper supervisor를 비우고,
  종료 시점에는 `Drop` guard로 항상 `shutdown_helper_process()`를 호출한다.
- 즉 테스트가 끝난 뒤 실제 `canon-helper.exe`가 카메라를 계속 붙잡지 않게 해,
  다음 앱 실행이 이전 검증 때문에 오염되지 않도록 막았다.
- 현재 남아 있던 stray helper 프로세스도 즉시 종료해 현장 런을 정리했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_restarts_helper_once_when_the_request_was_never_consumed -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_escalates_when_dense_startup_family_is_already_over_five_seconds_old -- --exact`
- 추가 확인:
  - test 종료 뒤 `Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'canon-helper.exe' }`
  - 결과: lingering `canon-helper.exe` 없음

이번 시점 제품 판단:

1. latest `session-open-failed`는 startup/connect family가 맞지만, 카메라 본체나 readiness 정책보다 먼저 **직전 검증이 남긴 stray helper**가 직접 원인이었다.
2. 그래서 같은 기능이 계속 흔들려 보인 이유는 하나의 미해결 기능 버그가 아니라, 최근 수정 검증 과정에서 남은 runtime 오염이 현장 앱 실행을 다시 망쳤기 때문이다.
3. 이번 보강으로 이후 회귀 테스트는 다음 실장비 런을 같은 방식으로 오염시키지 않아야 한다.

### 2026-04-21 16:47 +09:00 최신 앱 재검토: stray helper 오염은 사라졌고, latest는 reconnect 뒤 first-shot auto-retry가 너무 빨랐던 경계로 더 좁혀졌다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest 로그와 기존 startup/connect troubleshooting 문서를 다시 참조해 해결하라고 요청했다.
2. 최신 런이 다시 startup/connect family인지, 아니면 다른 축으로 옮겨졌는지 증거로 정리하라고 요청했다.

체크리스트 기준 분류:

- latest `session_000000000018a84f5c52118bb8`는 startup/connect family가 아니다.
- helper는 시작 직후 `camera-ready`까지 올라왔고, 각 auto-retry 사이에도 다시 `camera-ready`로 회복했다.
- 따라서 이번 latest는 **same first-shot `capture-trigger-failed(0x00000002)` family**가 reconnect 이후에도 남아 있던 케이스로 보는 편이 맞다.

실제 확인 근거:

- same session `camera-helper-startup.log`는
  - `07:41:54 camera-ready`
  - `07:41:58 camera-ready`
  - `07:41:59 camera-ready`
  - `07:42:00 connected-idle -> camera-ready`
  를 남겼다.
- same session `camera-helper-requests.jsonl`에는 첫 촬영 요청이 총 3번 기록됐다.
- same session `camera-helper-events.jsonl`에는
  - 첫 요청 `capture-accepted -> capture-trigger-failed(0x00000002)`
  - 두 번째 요청 `capture-accepted -> capture-trigger-failed(0x00000002)`
  - 세 번째 요청 `capture-accepted -> capture-trigger-failed(0x00000002)`
  가 연속으로 남았다.
- `timing-events.log`에도
  - `request-capture`
  - `request-capture-auto-retry attempt=1`
  - `request-capture`
  - `request-capture-auto-retry attempt=2`
  - `request-capture`
  만 남았고, 저장으로 닫히는 `file-arrived`는 없었다.
- 동시에 현재 머신의 `canon-helper.exe` lingering count는 `0`이었다.
  즉 직전 회차의 stray helper 오염은 이번 latest 재현에는 개입하지 않았다.

직접 원인:

- reconnect 자체는 이미 실제로 되고 있었다.
- 하지만 latest evidence는 **reconnect 뒤 helper가 `camera-ready`를 찍자마자 host가 다음 auto-retry를 너무 빨리 다시 보내고 있었다**는 쪽으로 더 잘 맞는다.
- 그 결과 새 세션에서 retry하더라도 카메라 안정화 이전에 다시 첫 셔터가 들어가, 같은 `0x00000002`가 반복된 것으로 보는 편이 가장 일관된다.

이번 회차 수정:

- host는 internal trigger failure auto-retry 전에 helper가 `ready / healthy / camera-ready`로 돌아왔는지만 보는 데서 멈추지 않고,
  그 상태가 짧게 안정된 뒤에만 다음 retry를 보내도록 보강했다.
- 제품 기준으로는 reconnect 자체뿐 아니라, **reconnect 뒤 첫 셔터 재시도 타이밍**까지 묶어 안정화한 것이다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_waits_for_helper_ready_to_stabilize_before_internal_auto_retry -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_once -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_auto_retries_the_first_internal_trigger_failure_twice_before_escalating -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_restarts_helper_once_when_the_request_was_never_consumed -- --exact`
- 추가 확인:
  - test 종료 뒤 `Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'canon-helper.exe' }`
  - 결과: lingering `canon-helper.exe` 없음

이번 시점 제품 판단:

1. stray helper 오염은 이번 latest에서 재발하지 않았다.
2. 남은 latest failure는 startup이나 request-consumption stall이 아니라, reconnect 뒤 first-shot retry 타이밍이 아직 너무 빠르던 경계였다.
3. 이번 보강으로 다음 현장 확인 포인트는 첫 촬영이 흔들려도 retry가 너무 급하게 다시 들어가지 않고 실제 저장으로 닫히는지다.

### 2026-04-22 00:00 +09:00 latest preview latency baseline 재고정: startup/save는 닫혔고, 남은 비용은 fast preview 이후 truthful close였다

사용자 최신 요청:

1. 앱을 다시 실행했으니 최신 로그를 확인하고 문제를 해결하라고 요청했다.
2. `docs/runbooks/preview-latency-next-steps-checklist-20260422.md` 순서대로 조치하고 기록하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8673fd974df10`였다.
- `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`는 `2026-04-20 16:40:03 +09:00` 이후 갱신되지 않았고,
  이번 회차의 canonical latest evidence는 session diagnostics 패키지 쪽이 맞았다.
- 같은 session의 `session.json` 기준으로
  - 총 5컷 모두 `renderStatus = previewReady`
  - 5컷 모두 `preview.kind = preset-applied-preview`
  - latest helper status는 `camera-ready`
  로 닫혔다.
- 같은 session의 `camera-helper-events.jsonl` 기준 helper correlation은 5컷 모두
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔고,
  `fastPreviewKind = windows-shell-thumbnail`였다.
- 같은 session의 `timing-events.log` 기준 reserve truthful close owner는 5컷 모두
  `preview-render-ready ... truthOwner=display-sized-preset-applied`
  로 남았지만,
  per-capture render detail은 계속
  `binary=C:\Program Files\darktable\bin\darktable-cli.exe`
  였다.

latest seam summary:

- `capture acknowledged -> file arrived`
  - `3258ms`, `3195ms`, `3272ms`, `2832ms`, `1963ms`
- `file arrived -> fast preview visible`
  - `377ms`, `297ms`, `302ms`, `256ms`, `254ms`
- `fast preview visible -> preset-applied visible`
  - `4404ms`, `4088ms`, `3765ms`, `3854ms`, `3439ms`
- `preview-render-ready elapsedMs`
  - `4342ms`, `4016ms`, `3718ms`, `3817ms`, `3416ms`

이번 회차 해석:

- startup/connect와 first-shot save는 latest baseline에서 더 이상 주 blocker가 아니었다.
- remaining cost는 helper/file-arrived 이후 same-capture fast preview를 띄우는 경계가 아니라,
  **fast preview 이후 per-capture truthful close를 다시 여는 `darktable-cli` hot path**
  였다.
- 특히 `fast preview visible -> preset-applied visible`가 `3.4s ~ 4.4s`로,
  `preview-render-ready elapsedMs`와 거의 같은 크기로 겹쳤다.
- 따라서 다음 software reduction target은 latest session 기준으로도
  `capture/save`가 아니라 `truthful close hot path`로 읽는 편이 맞다.

이번 회차 수정:

- host는 이제 capture request가 시작되면 active preset용 preview renderer warm-up을 한 번 더 건다.
- 목적은 warm-up 비용을 `capture acknowledged -> file arrived` 구간 아래에 숨겨,
  같은 truthful close contract를 유지한 채 이후 `preview-render-ready` 비용을 줄일 기회를 만드는 것이다.
- retryable capture error가 난 경우에는 세션 lifecycle stage도 다시 `capture-ready`로 복구해,
  warm-up 보강이 retry readiness semantics를 흔들지 않게 같이 정리했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_restarts_preview_warmup_while_camera_save_is_in_flight`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_keeps_session_retryable_when_focus_is_not_locked -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`

이번 시점 제품 판단:

1. latest field evidence는 startup family가 아니라 latency family였고, target seam은 fast preview 이후 truthful close다.
2. 이번 software change는 truth contract를 유지한 채 warm-up을 capture save seam 아래로 다시 숨기는 쪽이다.
3. 아직 post-change approved hardware comparable run은 없으므로, official gate 판정과 ledger 갱신은 다음 one-session package 이후에 한다.

### 2026-04-22 00:40 +09:00 latest rerun 재검토: capture 중 warm-up 시도는 first-shot truth를 깨서 reject했고, hot path는 다시 comparable rerun으로 확인해야 한다

사용자 최신 요청:

1. 앱을 다시 실행했으니 latest 로그를 확인하고 문제를 해결하라고 다시 요청했다.
2. 같은 checklist 문서 순서대로 조치하고 기록하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a868febfab83c0`였다.
- 같은 session의 startup/connect는 정상으로 닫혔다.
  - latest helper status: `ready / healthy / camera-ready`
  - 5 requests 모두 `capture-accepted -> file-arrived -> fast-preview-ready`로 이어졌다.
- 하지만 첫 컷 `capture_20260421153144380_13c7122f36`은 같은 session의 `session.json` 기준으로
  - `preview.kind = legacy-canonical-scan`
  - `renderStatus = previewReady`
  - `xmpPreviewReadyAtMs = null`
  이었다.
- 같은 첫 컷의 `timing-events.log`에는
  - `preview-render-start`
  - 다시 한 번 `preview-render-start`
  - `preview-render-queue-saturated`
  - `preview-render-ready ... binary=existing-preview-fallback;sourceAsset=legacy-canonical-scan;truthOwner=existing-preview-fallback`
  가 남았다.
- 즉 이번 latest는 latency improvement attempt가 first-shot truthful close를 더 싸게 만든 것이 아니라,
  **capture 중에 다시 건 warm-up이 render queue를 경쟁시키면서 first-shot을 fallback close로 밀어낸 회귀**
  로 읽는 편이 맞다.

latest numbers:

- first shot:
  - `capture_preview_ready = 2318ms`
  - 하지만 `truth owner = existing-preview-fallback`
  - `preview.kind = legacy-canonical-scan`
  - `xmpPreviewReadyAtMs = null`
- shots 2~5:
  - `capture_preview_ready = 6556ms`, `5916ms`, `5764ms`, `5872ms`
  - `preview-render-ready elapsedMs = 3717ms`, `3616ms`, `3614ms`, `3516ms`

이번 회차 해석:

- 숫자만 보면 2장~5장은 직전 baseline보다 조금 내려갔지만,
  이번 patch는 first-shot truth contract를 깨뜨렸기 때문에 성공으로 셀 수 없다.
- runbook guardrail 기준으로 보면,
  `previewReady`를 truthful close asset만 소유하게 유지해야 한다는 조건이 latest session에서 깨졌다.
- 따라서 capture-time warm-up 시도는 **latency optimization candidate가 아니라 rejected patch**로 정리하는 편이 맞다.

이번 회차 수정:

- capture request 시작 시 preview renderer warm-up을 다시 거는 경로를 제거했다.
- retryable capture error 뒤 lifecycle stage를 `capture-ready`로 복구하는 보정은 유지했다.
- 회귀 테스트는 `capture 요청 중에는 competing warm-up을 다시 열지 않는다`는 쪽으로 뒤집어 고정했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`

이번 시점 제품 판단:

1. latest 앱 세션은 startup/save failure가 아니라, 이전 hot-path reduction attempt가 first-shot truth를 깨뜨린 evidence였다.
2. 그래서 이번 턴의 해결은 warm-up 경쟁 경로를 제거해 truth contract를 다시 우선시하는 쪽이었다.
3. official gate와 comparable latency 판정은 이 rollback 뒤 approved hardware one-session rerun으로 다시 읽어야 한다.

### 2026-04-22 00:40 +09:00 rollback 뒤 current-code rerun: first-shot truth는 복구됐지만 official gate는 여전히 No-Go였다

사용자 최신 요청:

1. 앱을 다시 실행했으니 latest 로그를 확인하고 문제를 해결하라고 요청했다.
2. 같은 checklist 문서 순서대로 조치하고 기록하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a86975cbe622b8`였다.
- 같은 session의 `session.json` 기준으로
  - lifecycle stage는 다시 `capture-ready`
  - 5컷 모두 `renderStatus = previewReady`
  - 5컷 모두 `preview.kind = preset-applied-preview`
  - 5컷 모두 `xmpPreviewReadyAtMs != null`
  로 닫혔다.
- 같은 session의 `camera-helper-status.json` 최종 상태도
  `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`였다.
- 같은 session의 `camera-helper-events.jsonl` 기준 helper correlation은 5컷 모두
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
- 같은 session의 `timing-events.log` 기준 truthful close owner는 5컷 모두
  `preview-render-ready ... truthOwner=display-sized-preset-applied`
  로 남았고, rejected session에서 보였던
  `preview-render-queue-saturated`나 `existing-preview-fallback` close는 더 이상 없었다.

latest numbers:

- `capture_preview_ready`
  - `6021ms`, `6377ms`, `5931ms`, `6075ms`, `5773ms`
- `preview-render-ready elapsedMs`
  - `3616ms`, `4020ms`, `3618ms`, `4018ms`, `3717ms`

이번 회차 해석:

- current rollback code는 first-shot truth regression을 실제로 제거했다.
- 즉 이번 latest는 더 이상 false-ready나 startup/save failure evidence가 아니라,
  **truth는 복구됐지만 official gate는 아직 못 넘긴 current-code No-Go package**
  로 읽는 편이 맞다.
- baseline `session_000000000018a8673fd974df10`와 비교하면
  `capture_preview_ready`는 전반적으로 내려왔지만,
  `preview-render-ready elapsedMs`는 mixed result라 hot path reduction success로 닫을 수는 없다.

이번 회차 조치:

- software change는 추가하지 않았다.
- 이번 턴에서는 rollback 뒤 current code의 approved-hardware one-session package를 canonical verdict로 확정하고,
  checklist / history / hardware ledger를 같은 결론으로 갱신했다.

검증:

- latest field package:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a86975cbe622b8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a86975cbe622b8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a86975cbe622b8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a86975cbe622b8\diagnostics\camera-helper-status.json`
- automated regression status:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`

이번 시점 제품 판단:

1. latest current-code rerun은 first-shot truthful close를 다시 복구했다.
2. 하지만 reserve path의 공식 release gate인 `preset-applied visible <= 3000ms`는 여전히 실패했다.
3. 다음 실질 과제는 startup/debugging 반복이 아니라, booth-visible truthful close hot path를 더 줄일 새 reduction candidate를 찾는 일이다.

### 2026-04-22 11:10 +09:00 latest 앱 재실행 재점검: startup-only session은 새 blocker가 아니었고, 복구 경로에서도 fast-preview seam을 session log에 다시 남기게 보강했다

사용자 최신 요청:

1. 앱을 다시 실행해 테스트했으니 latest 로그와 `history/`를 함께 보고 문제를 해결하라고 요청했다.
2. 조치 뒤에는 canonical 기록도 같이 남기라고 요청했다.

실제 확인 근거:

- latest startup-only session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88adfee94784c`였다.
- 같은 session의 `session.json` 기준으로
  - `captures = []`
  - `lifecycle.stage = preset-selected`
  였다.
- 같은 session의 `diagnostics/camera-helper-startup.log`는
  - `sdk-initializing`
  - `session-opening`
  - `camera-ready`
  로 정상 진입을 보여 줬다.
- 같은 session의 `camera-helper-status.json`도
  - `cameraState = ready`
  - `helperState = healthy`
  - `detailCode = camera-ready`
  - `sequence = 30`
  로 닫혔다.
- 즉 latest relaunch 자체는 startup/connect failure evidence가 아니라, 촬영 없는 short startup session으로 읽는 편이 맞았다.
- 반면 current comparable No-Go package `session_000000000018a86975cbe622b8`를 다시 보면
  `session.json`에는 `fastPreviewVisibleAtMs`가 있었지만,
  `diagnostics/timing-events.log`에는 일부 컷의 `fast-preview-visible` seam이 비어 있었다.

이번 회차 해석:

- 이번 latest는 새 startup blocker가 아니었다.
- current blocker는 여전히 reserve path truthful close latency였다.
- 다만 same-capture preview를 host가 later recovery path로 다시 붙인 경우,
  per-session timing log에 `fast-preview-visible`이 빠져 다음 회차 seam 판독이 불완전해지는 공백이 있었다.

이번 회차 수정:

- host `sync_better_preview_assets_in_manifest(...)`가 `captureSaved` / `previewWaiting` 상태에서
  same-capture preview를 복구하며 `fastPreviewVisibleAtMs`를 다시 채우는 경우,
  같은 시점의 `fast-preview-visible` timing event도 함께 남기도록 보강했다.
- 따라서 이후 latest session은 request flow에서 fast preview가 직접 promote되지 않아도,
  recovery path가 same-capture preview를 살린 사실을 session-local `timing-events.log`로 다시 읽을 수 있다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_allows_next_capture_once_same_capture_fast_preview_is_visible -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --nocapture`
- `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745202892851_pAVdAAA7pU dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --filter "ResolveShutterPlanForNextCaptureLocked_uses_halfway_prime_non_af_and_delay_once_after_internal_trigger_reconnect|ExecuteCaptureShutterPlan_retries_internal_trigger_failure_once_with_half_press_non_af_plan"`

이번 시점 제품 판단:

1. latest 재실행은 startup family 재발이 아니라 healthy startup-only session이었다.
2. current product blocker는 계속 truthful close latency이며, 이번 회차에서 그 판단을 바꿀 새 field failure는 보이지 않았다.
3. 대신 다음 latency 회차가 session-local evidence만으로 seam을 다시 읽을 수 있게, same-capture preview recovery path의 `fast-preview-visible` 계측 공백을 먼저 닫았다.

### 2026-04-22 11:40 +09:00 latest 5-shot rerun은 first-shot duplicate render overlap reject였고, current code는 same-capture speculative close를 single-lane으로 다시 고정했다

사용자 최신 요청:

1. 방금 앱을 다시 실행했으니 latest 로그를 확인해 문제를 해결하라고 요청했다.
2. 같은 checklist 순서대로 조치하고 canonical 기록도 남기라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88c71e9375868`였다.
- 같은 session의 startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- 5컷 모두 helper correlation은 `capture-accepted -> file-arrived -> fast-preview-ready`로 닫혔다.
- 하지만 첫 컷 `capture_20260422022122471_c3d03c15e1`은 `session.json` 기준으로
  - `preview.kind = legacy-canonical-scan`
  - `renderStatus = previewReady`
  - `xmpPreviewReadyAtMs = null`
  로 끝났다.
- 같은 첫 컷의 `diagnostics/timing-events.log`에는
  - `file-arrived`
  - `fast-preview-promoted kind=legacy-canonical-scan`
  - `preview-render-start`
  - 약 7초 뒤 다시 `preview-render-start`
  - `preview-render-failed reason=render-process-failed`
  - `preview-render-ready ... binary=existing-preview-fallback;sourceAsset=legacy-canonical-scan;truthOwner=existing-preview-fallback`
  가 남았다.
- 같은 시각 최신 `preview-stderr-*.log` 2개에는 모두
  `Magick: caught exception 0xC0000005 "Access violation"...`
  만 남았다.

이번 회차 해석:

- startup/save family는 아니었다.
- 가장 그럴듯한 해석은, first-shot speculative close가 cold lane에서 7.2초 join budget 안에 끝나지 못한 상태로 살아 있는데 host가 direct raw refinement를 다시 열어
  **같은 preview runtime/config/library에 두 개의 darktable render를 겹치게 만들었고**, 그 overlap이 두 lane 모두 `render-process-failed`로 무너지게 했다는 것이다.
- 즉 latest는 latency candidate failure가 아니라,
  **first-shot duplicate render overlap이 `existing-preview-fallback` close를 다시 만든 reject evidence**
  로 읽는 편이 맞다.

이번 회차 수정:

- same-capture speculative close가 이미 in-flight이면,
  direct raw refinement가 renderer timeout 경계 안에서 다시 같은 lane을 열지 않도록 join wait를 renderer timeout과 맞췄다.
- 따라서 current code는 slow first-shot speculative close가 있어도 먼저 그 lane이 settle할 때까지 기다리고,
  그 뒤에만 다음 판단을 하게 된다.
- 회귀 테스트 `complete_preview_render_keeps_waiting_for_a_slow_speculative_close`를 추가해,
  8초짜리 slow speculative close에서도 duplicate `preview-render-start`가 다시 열리지 않게 고정했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_keeps_waiting_for_a_slow_speculative_close -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_still_avoids_a_duplicate_render_while_speculative_close_is_active -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_closes_with_existing_same_capture_preview_when_raw_refinement_fails -- --exact`

이번 시점 제품 판단:

1. latest rerun은 startup/connect regression이 아니라 first-shot duplicate render overlap regression이었다.
2. current code는 그 overlap을 막는 single-lane guardrail까지는 다시 복구했다.
3. 다음 제품 판정은 latency work 재개보다 먼저, 이 patch 뒤 approved hardware one-session rerun으로 first-shot truthful close가 실제로 복구됐는지 다시 확인하는 순서가 맞다.

### 2026-04-22 11:38 +09:00 patch 뒤 approved rerun에서는 first-shot overlap family가 사라졌고, verdict는 다시 latency-only No-Go로 돌아갔다

사용자 최신 요청:

1. 방금 앱을 다시 실행했으니 latest 로그를 확인하고 문제를 해결하라고 요청했다.
2. checklist 순서대로 조치하고 canonical 기록까지 남기라고 요청했다.

실제 확인 근거:

- latest approved one-session package는 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4`였다.
- 같은 session의 startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- 같은 session의 `session.json` 기준으로
  - 5컷 모두 `renderStatus = previewReady`
  - 5컷 모두 `preview.kind = preset-applied-preview`
  - 5컷 모두 `xmpPreviewReadyAtMs != null`
  - lifecycle stage는 다시 `capture-ready`
  로 닫혔다.
- 같은 session의 `camera-helper-events.jsonl` 기준 helper correlation도 5컷 모두
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
- 같은 session의 `timing-events.log`에는 첫 컷 포함 전 컷이
  - `fast-preview-promoted kind=legacy-canonical-scan`
  - `preview-render-start`
  - `preview-render-ready ... truthOwner=display-sized-preset-applied`
  순서로만 닫혔다.
- 직전 reject session `session_000000000018a88c71e9375868`에서 보였던
  - 두 번째 `preview-render-start`
  - `preview-render-failed reason=render-process-failed`
  - `existing-preview-fallback`
  증거는 latest rerun에서 더 이상 보이지 않았다.

latest numbers:

- `capture_preview_ready`
  - `5905ms`, `5982ms`, `5763ms`, `5864ms`, `5803ms`
- `preview-render-ready elapsedMs`
  - `3717ms`, `3617ms`, `3615ms`, `3618ms`, `3715ms`

이번 회차 해석:

- single-lane guardrail patch는 latest approved rerun에서 first-shot duplicate render overlap family를 실제로 제거했다.
- 따라서 current product blocker는 다시 startup/save나 false-ready가 아니라,
  **truth는 유지되지만 `preset-applied visible <= 3000ms`를 못 넘는 truthful-close latency**
  하나로 다시 좁혀졌다.

이번 회차 조치:

- software change는 추가하지 않았다.
- 이번 턴에서는 latest approved hardware rerun `session_000000000018a88d53fa8f00c4`를 canonical verdict로 반영하고,
  checklist / history / ledger를 같은 판정으로 갱신했다.

검증:

- latest field package:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\camera-helper-status.json`

이번 시점 제품 판단:

1. current code는 latest approved rerun에서 first-shot overlap reject family를 실제로 제거했다.
2. 하지만 official reserve-path gate `preset-applied visible <= 3000ms`는 여전히 실패했다.
3. 다음 실질 과제는 overlap debugging 반복이 아니라, booth-visible truthful close hot path를 더 줄이는 새 reduction candidate를 찾는 일이다.

### 2026-04-22 12:00 +09:00 latest session에는 official gate 구간이 직접 안 남아 있었고, 이제 `capture_preview_ready` detail에 first-visible -> preset-applied visible 숫자를 같이 남긴다

사용자 최신 요청:

1. 로그에 `원본/first-visible이 먼저 보인 뒤, 프리셋 적용 preview가 보여질 때까지`의 구간이 직접 기록되는지 확인하라고 요청했다.
2. 없다면 session-local 로그에 이 숫자도 기록하게 수정하라고 요청했다.

실제 확인 근거:

- latest approved rerun `session_000000000018a88d53fa8f00c4`의 `diagnostics/timing-events.log`를 다시 보면,
  같은 capture마다 아래는 있었다.
  - `fast-preview-promoted`
  - `preview-render-ready`
  - `capture_preview_ready elapsedMs=...`
- 하지만 `capture_preview_ready` detail에는 아직
  `originalVisibleToPresetAppliedVisibleMs`
  가 직접 없었다.
- 즉 current session-local log만으로도 계산은 가능했지만,
  여전히
  - first-visible 시점
  - preset-applied visible 시점
  - 그 사이 official gate 구간
  을 한 줄에서 바로 읽지는 못하는 상태였다.

이번 회차 수정:

- `capture_preview_ready` timing event detail에 아래를 같이 남기도록 보강했다.
  - `originalVisibleToPresetAppliedVisibleMs`
  - `firstVisibleAtMs`
  - `presetAppliedVisibleAtMs`
- `fastPreviewVisibleAtMs`가 없을 때는
  `originalVisibleToPresetAppliedVisibleMs=unavailable`
  로 명시한다.
- 글로벌 앱 로그의 `capture_preview_ready`도
  `original_visible_to_preset_applied_visible_ms`
  를 같이 남기도록 맞췄다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_closes_with_existing_same_capture_preview_when_raw_refinement_fails -- --exact`

이번 시점 제품 판단:

1. latest session에는 official gate 구간이 direct metric으로는 아직 찍히지 않았다.
2. current code는 다음 session부터 `capture_preview_ready` detail 한 줄로
   first-visible -> preset-applied visible 구간을 바로 읽을 수 있게 됐다.
3. 제품 blocker 자체는 여전히 truthful-close latency이며, 새 hardware rerun에서 이 direct metric이 실제로 찍히는지만 다시 확인하면 된다.

### 2026-04-22 12:20 +09:00 latest 로그 재확인: 새 capture package는 없었고, current worktree는 direct metric guardrail을 자동 검증으로 다시 닫았다

사용자 최신 요청:

1. 방금 앱을 실행했으니 로그파일을 확인해 문제를 해결하고 checklist 순서대로 기록하라고 요청했다.

실제 확인 근거:

- `C:\Users\KimYS\Pictures\dabi_shoot\sessions`를 다시 확인한 결과,
  latest capture package는 여전히
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4`
  였다.
- 따라서 이번 latest app relaunch는 더 새로운 capture evidence를 만들지 않았고,
  current field verdict owner는 계속 위 approved `No-Go` package로 읽는 편이 맞았다.
- 같은 session의 `diagnostics/timing-events.log`를 다시 확인하면
  `capture_preview_ready` detail은 여전히
  `elapsedMs`, `budgetState`, `renderStatus`
  만 남아 있었다.
- 반면 current worktree 기준 `src-tauri/src/capture/ingest_pipeline.rs`는
  `capture_preview_ready` detail에
  `originalVisibleToPresetAppliedVisibleMs`,
  `firstVisibleAtMs`,
  `presetAppliedVisibleAtMs`
  를 같이 남기도록 이미 보강돼 있었다.
- same-capture recovery path의 `fast-preview-visible` seam 복구도 current worktree에 남아 있었고,
  아래 targeted tests를 다시 통과했다.
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness client_recent_session_visibility_events_are_mirrored_into_session_timing_logs -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_allows_next_capture_once_same_capture_fast_preview_is_visible -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_keeps_waiting_for_a_slow_speculative_close -- --exact`

이번 회차 해석:

- latest approved hardware package에서 direct metric이 비어 있는 것은
  이번 latest relaunch로 새로 생긴 blocker가 아니라,
  **아직 logging guardrail이 실제 hardware rerun으로 다시 수집되지 않은 상태**
  로 읽는 편이 맞다.
- 즉 이번 턴의 실질 조치는 새로운 startup/debugging으로 범위를 넓히는 일이 아니라,
  current worktree의 direct metric guardrail이 살아 있는지 다시 검증하고
  next rerun 해석 기준을 고정하는 일이었다.
- 제품 blocker 자체는 계속 truthful-close latency이며,
  startup/save나 false-ready family로 되돌아간 것은 아니다.

이번 회차 조치:

- 추가 software logic은 새로 얹지 않았다.
- current worktree에 이미 들어 있던
  - `capture_preview_ready` direct metric logging
  - same-capture recovery seam logging
  - slow speculative close single-lane guardrail
  을 latest 로그 기준으로 다시 대조하고 targeted automated coverage로 확인했다.
- checklist와 history wording을 같이 갱신해,
  next agent가 latest session의 direct metric 부재를 새 runtime regression으로 오해하지 않게 맞췄다.

이번 시점 제품 판단:

1. latest app relaunch는 `session_000000000018a88d53fa8f00c4`보다 새로운 capture evidence를 만들지 않았다.
2. current worktree는 next hardware rerun에서 official gate direct metric을 바로 읽을 준비가 돼 있다.
3. 다음 실질 작업은 여전히 booth-visible truthful close hot path를 더 줄이는 후보를 찾고,
   그 직후 rerun에서 `originalVisibleToPresetAppliedVisibleMs`가 실제로 찍히는지 확인하는 순서다.

### 2026-04-22 12:35 +09:00 latest relaunch capture sessions도 latency blocker를 그대로 보여 줬고, current code는 capture flow re-entry에서 reserve lane을 다시 prime한다

사용자 최신 요청:

1. 앱을 방금 다시 실행했으니 latest 로그를 확인하고, checklist 순서에 맞춰 문제를 해결한 뒤 기록하라고 요청했다.

실제 확인 근거:

- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88f5792a50534`
  와
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88fa09dcbdca0`
  는 app relaunch 뒤 새로 생긴 5-shot capture sessions였다.
- 두 session의 `diagnostics/timing-events.log`를 보면:
  - 5컷 모두 `fast-preview-promoted kind=legacy-canonical-scan`
  - 5컷 모두 `preview-render-ready ... truthOwner=display-sized-preset-applied`
  - 5컷 모두 `capture_preview_ready detail`에
    `originalVisibleToPresetAppliedVisibleMs`
    가 직접 남아 있었다.
- latest two-session direct metric은 아래 범위였다.
  - `session_000000000018a88f5792a50534`
    - `preview-render-ready elapsedMs`: `3721ms`, `3517ms`, `3614ms`, `3620ms`, `3517ms`
    - `originalVisibleToPresetAppliedVisibleMs`: `3766ms`, `3600ms`, `3686ms`, `3692ms`, `3603ms`
  - `session_000000000018a88fa09dcbdca0`
    - `preview-render-ready elapsedMs`: `3823ms`, `3416ms`, `3819ms`, `3816ms`, `3415ms`
    - `originalVisibleToPresetAppliedVisibleMs`: `3860ms`, `3439ms`, `3848ms`, `3839ms`, `3443ms`
- 즉 latest relaunch에서도 새 blocker는 없었고,
  reserve truthful close hot path가 그대로 남아 있는 쪽으로 읽는 편이 맞았다.
- 코드 확인 결과 current warm-up 예약은
  `start_session` / `select_active_preset`
  경계에만 있었다.
  그래서 앱을 다시 열어 이미 active preset이 잡힌 capture flow로 복귀하는 경우,
  reserve lane이 explicit re-prime 없이 cold state로 다시 시작될 수 있었다.

이번 회차 조치:

- host에 `prime_preview_runtime` command를 추가해
  existing active preset capture flow가 다시 열릴 때
  preview worker runtime prime + preview renderer warm-up을 명시적으로 다시 예약하게 했다.
- client capture runtime service와 session provider는
  capture flow 진입 시 current preset binding으로 위 command를 1회 호출하도록 연결했다.
- 이 재-prime은 capture request path에 넣지 않았기 때문에,
  이전 reject 원인이었던 capture-time warm-up overlap family는 다시 열지 않도록 유지했다.
- 관련 자동 검증:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --exact`
  - `pnpm test:run src/capture-adapter/services/capture-runtime.test.ts --testNamePattern "primes the preview runtime"`
  - `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime when an active-preset session enters capture flow"`

이번 시점 제품 판단:

1. latest relaunch capture sessions도 startup/save failure가 아니라 truthful-close latency blocker를 다시 보여 줬다.
2. current code는 app re-entry가 existing active preset capture flow로 돌아올 때 reserve lane을 다시 warm state로 데우도록 보강됐다.
3. 하지만 comparable hardware rerun은 아직 다시 수집하지 않았으므로, latency reduction success는 아직 선언할 수 없다.
4. 다음 판정은 이 patch 뒤 hardware rerun에서 first shot 포함 `preview-render-ready elapsedMs`와 `originalVisibleToPresetAppliedVisibleMs`가 함께 내려오는지로 닫는 순서가 맞다.

### 2026-04-22 13:56 +09:00 latest relaunch 5-shot run은 first shot만 다시 cold spike였고, current code는 in-flight re-entry prime이 끝날 때까지 첫 capture를 기다리게 보강했다

사용자 최신 요청:

1. 방금 앱을 실행했으니 latest 로그를 확인하고, checklist 순서대로 문제를 해결한 뒤 기록하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a89441063a0c54`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> session-opened -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `timing-events.log`를 보면 5컷 모두 `previewReady` truthful close로 닫혔지만,
  first shot만 direct metric이 다시 크게 튀었다.
  - `preview-render-ready elapsedMs`: `15174`, `3314`, `3718`, `3319`, `3215`
  - `originalVisibleToPresetAppliedVisibleMs`: `15265`, `3357`, `3780`, `3381`, `3273`
- 즉 startup/save/correctness가 무너진 것은 아니고,
  relaunch 뒤 first shot만 reserve lane warm-up보다 먼저 capture가 들어간 흔적으로 읽는 편이 맞았다.

이번 회차 해석:

- 직전 relaunch patch는 capture flow re-entry에서 `prime_preview_runtime`을 다시 호출하게 했지만,
  latest session은 그 re-prime이 **시작만 되고 settle되기 전에 first capture request가 먼저 들어갈 수 있다**는 점을 보여 줬다.
- 숫자가 첫 컷 이후 바로 다시 `3.2s ~ 3.8s`대로 돌아온 것도,
  current blocker가 일반 steady-state latency보다 `first-shot cold prime settle` 쪽에 더 가깝다는 해석을 뒷받침했다.

이번 회차 조치:

- host `prime_preview_runtime` command가 preview renderer warm-up settle까지 bounded wait를 유지하게 보강했다.
- client `SessionProvider`는 capture flow 진입 때 시작한 in-flight preview prime promise를 첫 capture request 전에 기다리도록 연결했다.
- capture request path에서 새 warm-up을 다시 시작하지는 않게 유지했다.
  그래서 이전 reject 원인이었던 capture-time overlap family를 다시 열지는 않는다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --exact`
- `pnpm test:run src/capture-adapter/services/capture-runtime.test.ts --testNamePattern "primes the preview runtime"`
- `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime|waits for an in-flight preview runtime prime"`

이번 시점 제품 판단:

1. latest app run은 새 correctness regression이 아니라 first-shot cold spike evidence였다.
2. current code는 capture flow re-entry warm-up이 끝나기 전에 first shot이 먼저 들어가던 틈을 메웠다.
3. 하지만 hardware rerun으로 direct metric이 실제로 내려오는지 아직 확인하지 않았으므로, 공식 verdict는 여전히 보류다.

### 2026-04-22 14:18 +09:00 latest rerun에서는 first-shot cold spike가 완화됐고, current code는 steady-state truthful-close cap을 192로 더 낮췄다

사용자 최신 요청:

1. 앱을 방금 실행했으니 latest 로그를 확인하고 문제를 해결하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a895d248f89000`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 `timing-events.log`를 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `xmpPreviewReadyAtMs != null`
  로 닫혔다.
- direct metric은 아래처럼 읽혔다.
  - `preview-render-ready elapsedMs`: `3831`, `3417`, `3415`, `3715`, `3413`
  - `originalVisibleToPresetAppliedVisibleMs`: `3963`, `3438`, `3449`, `3767`, `3435`
  - `capture_preview_ready`: `6106`, `5576`, `5536`, `5820`, `5514`

이번 회차 해석:

- 직전 latest `session_000000000018a89441063a0c54`에서 보였던 first-shot cold spike
  `15174 / 15265`
  는 latest rerun에서 사라졌다.
- 즉 capture flow re-entry prime join은 first shot을 다시 steady-state band로 되돌리는 데에는 효과가 있었다.
- 하지만 5컷 전체가 여전히 `3.4s ~ 4.0s`대에 남아 있으므로,
  current blocker는 다시 `fast preview -> display-sized preset-applied truthful close`
  자체의 steady-state darktable 비용으로 읽는 편이 맞다.

이번 회차 조치:

- existing re-entry prime join 보강은 유지했다.
- 추가로 same-capture `fast-preview-raster` truthful close cap을
  `256x256 -> 192x192`
  로 더 낮춰 darktable hot path 자체를 한 단계 더 가볍게 보강했다.
- truth owner, same-session correctness, capture-time overlap guardrail은 바꾸지 않았다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`

이번 시점 제품 판단:

1. latest rerun은 first-shot cold spike가 현재 code에서 완화됐다는 evidence다.
2. 남은 blocker는 다시 steady-state truthful-close latency다.
3. 이번 cap reduction이 실제로 먹히는지는 다음 hardware rerun에서 direct metric을 다시 봐야 한다.

### 2026-04-22 14:55 +09:00 latest rerun에서는 192 cap 아래에서도 first-shot cold spike가 재발했고, current code는 preview prime을 effect 이전 전이 시점에도 시작하게 보강했다

사용자 최신 요청:

1. 방금 앱을 실행했으니 latest 로그파일을 확인해 문제를 해결하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8975f5f4f0b34`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `timing-events.log`를 보면 same-capture truthful close cap `192x192`가 실제로 적용돼 있었다.
  - first shot `preview-render-ready elapsedMs=15174`
  - first shot `originalVisibleToPresetAppliedVisibleMs=15270`
  - 이후 4컷은 `preview-render-ready 3323ms ~ 3716ms`, `originalVisibleToPresetAppliedVisibleMs 3354ms ~ 3767ms`
- 즉 startup/save/correctness regression은 아니었고,
  latest previous rerun에서 잠시 완화됐던 first-shot cold spike가 다시 재발한 쪽으로 읽는 편이 맞았다.

이번 회차 해석:

- same-capture `192x192` cap 자체는 적용됐지만,
  first shot만 다시 `15초`대로 튄 점을 보면 current blocker는 steady-state cap보다도
  **capture flow 진입 직후 사용자가 너무 빨리 누르면 effect 기반 preview prime 시작보다 first request가 먼저 지나갈 수 있는 race**
  에 더 가깝다.
- 기존 code는 in-flight prime promise를 기다리게는 했지만,
  그 promise 생성 자체가 `useEffect`에만 묶여 있어
  아주 빠른 첫 request에서는 아직 늦을 수 있었다.

이번 회차 조치:

- preview prime을 `useEffect`에만 두지 않고
  `startSession` / `selectActivePreset` 시점에도 즉시 시작하도록 보강했다.
- 너무 빠른 첫 request가 아직 반영 전 state를 읽더라도,
  이미 시작한 in-flight preview prime promise를 지우지 않고 그대로 기다리게 보강했다.
- same-capture `192x192` truthful close cap은 유지했다.

검증:

- `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime|waits for an in-flight preview runtime prime|starts the preview runtime prime before the capture-flow effect can race ahead of the first request"`
- `pnpm test:run src/capture-adapter/services/capture-runtime.test.ts --testNamePattern "primes the preview runtime"`

이번 시점 제품 판단:

1. latest rerun은 `192x192` cap만으로는 first-shot cold spike를 막지 못했다.
2. current code는 그 spike의 더 직접적인 원인으로 보이는 preview-prime scheduling race를 줄이는 쪽으로 보강됐다.
3. 다만 hardware rerun이 아직 없으므로, 실제 효과 판정은 다음 latest session의 first shot direct metric으로 다시 닫아야 한다.

### 2026-04-22 15:08 +09:00 latest rerun에서는 first-shot race spike가 다시 사라졌고, current code는 사용자 요청에 따라 truthful close cap을 256으로 복귀시켰다

사용자 최신 요청:

1. 해상도를 낮추는 방법은 더 이상 쓰지 말라고 요청했다.
2. 더 낮은 해상도는 체감 개선도 없고 이질감만 만든다고 보고, 자연스러운 해상도로 다시 세팅하라고 요청했다.
3. 방금 앱 실행 테스트를 했으니 latest 로그를 확인하고 기록 후 해결하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8986df67c7e38`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `timing-events.log`를 보면 same-capture truthful close cap `192x192` 상태에서도
  first shot direct metric이 다시 steady-state band로 내려와 있었다.
  - `preview-render-ready elapsedMs`: `3318`, `3414`, `3515`, `3414`, `3726`
  - `originalVisibleToPresetAppliedVisibleMs`: `3360`, `3439`, `3532`, `3437`, `3762`
  - `capture_preview_ready`: `5462`, `5447`, `5534`, `5454`, `5800`
- 즉 직전 latest `session_000000000018a8975f5f4f0b34`에서 보였던 first-shot cold spike는
  이번 rerun에서는 다시 보이지 않았다.

이번 회차 해석:

- latest session 기준으로는 preview-prime scheduling race 보강이 실제로 먹힌 쪽으로 읽는 편이 맞다.
- 남은 문제는 다시 first-shot race가 아니라,
  5컷 전체에 남는 `3.3s ~ 3.8s`대 steady-state truthful-close latency다.
- 동시에 사용자는 낮은 해상도 자체를 제품적으로 거부했으므로,
  `192x192` 경로는 더 이상 유지할 가치가 없다.

이번 회차 조치:

- preview-prime scheduling race 보강은 유지했다.
- same-capture `fast-preview-raster` truthful close cap은
  `192x192 -> 256x256`
  으로 복귀시켰다.
- 즉 latest field evidence는 `192x192` 상태에서 읽되,
  current worktree는 이질감이 덜한 해상도로 다시 돌아온 상태다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
- `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime|waits for an in-flight preview runtime prime|starts the preview runtime prime before the capture-flow effect can race ahead of the first request"`
- `pnpm test:run src/capture-adapter/services/capture-runtime.test.ts --testNamePattern "primes the preview runtime"`

이번 시점 제품 판단:

1. latest rerun은 first-shot race spike가 current code에서 다시 사라졌다는 evidence다.
2. 남은 blocker는 다시 steady-state truthful-close latency다.
3. 해상도 하향 경로는 사용자 요청에 따라 종료했고, 다음 판단은 `256x256` 복귀 상태의 새 hardware rerun에서 다시 닫아야 한다.

### 2026-04-22 15:18 +09:00 `256x256` 복귀 뒤 latest rerun에서도 first-shot spike는 다시 보이지 않았고, 남은 문제는 steady-state truthful-close latency로 고정됐다

사용자 최신 요청:

1. 방금 앱을 실행했으니 latest 로그를 확인하고 문제를 해결하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a89961df9c18a0`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `timing-events.log`를 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `xmpPreviewReadyAtMs != null`
  로 닫혔다.
- direct metric은 `256x256` 복귀 상태에서 아래처럼 읽혔다.
  - `preview-render-ready elapsedMs`: `3820`, `3816`, `3417`, `3517`, `3524`
  - `originalVisibleToPresetAppliedVisibleMs`: `3877`, `3855`, `3441`, `3606`, `3606`
  - `capture_preview_ready`: `6054`, `5949`, `5489`, `5635`, `5617`

이번 회차 해석:

- 직전 latest `session_000000000018a8986df67c7e38`에서 보였던
  first-shot race spike 완화는 `256x256` 복귀 뒤 latest rerun에서도 유지됐다.
- 즉 current code 기준으로 first-shot cold/race family는 다시 immediate blocker가 아니었다.
- 반면 5컷 전체가 여전히 `3.4s ~ 3.9s`대에 머물렀으므로,
  남은 제품 문제는 다시 `fast preview -> display-sized preset-applied truthful close`
  steady-state latency로 고정하는 편이 맞다.

이번 회차 조치:

- preview-prime scheduling race 보강과 `256x256` truthful close cap 유지는 그대로 뒀다.
- latest `256x256` field evidence를 canonical history에 추가해,
  current blocker가 다시 steady-state truthful-close latency라는 점을 기록으로 고정했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
- `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime|waits for an in-flight preview runtime prime|starts the preview runtime prime before the capture-flow effect can race ahead of the first request"`
- `pnpm test:run src/capture-adapter/services/capture-runtime.test.ts --testNamePattern "primes the preview runtime"`

이번 시점 제품 판단:

1. latest rerun은 `256x256` 복귀 상태에서도 first-shot spike가 다시 나오지 않았다는 evidence다.
2. 해상도 하향 경로를 되살릴 필요는 없고, 남은 blocker는 steady-state truthful-close latency다.
3. 다음 reduction candidate는 해상도 인하가 아니라, `darktable-cli`가 소유한 booth-visible truthful close 비용 자체를 줄이는 방향에서 다시 찾아야 한다.

### 2026-04-22 15:41 +09:00 latest helper fast preview metadata drift를 줄였고, `windows-shell-thumbnail` reserve close는 이제 file-arrived 전에 바로 시작된다

사용자 최신 요청:

1. 계획에 따라 다음 수정 단계를 진행하라고 요청했다.

실제 확인 근거:

- latest field evidence `session_000000000018a89961df9c18a0`를 다시 대조해 보니,
  helper `camera-helper-events.jsonl`에는 5컷 모두
  `fast-preview-ready fastPreviewKind=windows-shell-thumbnail`
  가 직접 남아 있었다.
- 반면 같은 session의 host `timing-events.log`는 같은 canonical preview를
  `fast-preview-promoted kind=legacy-canonical-scan`
  으로 다시 기록하고 있었다.
- 즉 helper가 먼저 확보한 same-capture preview metadata가
  capture persist 경계에서 manifest/timing 쪽으로 제대로 이어지지 않았고,
  reserve close 시작도 그 early helper preview보다 뒤의 seed/fallback 판독에 더 기대고 있었다.

이번 회차 조치:

- host `request_capture` 경계는 helper `fast-preview-ready`를 canonical preview로 승격한 직후,
  그 source가 `windows-shell-thumbnail` 같은 reserve-close 대상이면
  `file-arrived`를 기다리지 않고 speculative truthful close를 바로 시작하게 보강했다.
- capture persist 단계는 이미 앞에서 canonical path로 승격된 early fast preview update가 있으면,
  이를 다시 `legacy-canonical-scan`으로 재분류하지 않고
  원래 `kind`와 `visibleAtMs`를 그대로 manifest에 보존하게 보강했다.
- 그래서 latest 현장 로그에서 보인
  `helper는 windows-shell-thumbnail를 봤는데 host는 legacy-canonical-scan으로 기억하는 drift`
  를 current code에서 줄였다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness early_windows_shell_thumbnail_is_preserved_and_starts_reserve_close_before_file_arrival_metadata -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`

이번 시점 제품 판단:

1. current code는 helper가 더 일찍 확보한 same-capture preview metadata를 host truth까지 끌고 오도록 보강됐다.
2. `windows-shell-thumbnail` reserve close는 이제 raw persist 이후 seed를 다시 읽기 전에 먼저 시작될 수 있다.
3. 다만 아직 새 hardware rerun direct metric은 없으므로, 이 보강이 실제 `originalVisibleToPresetAppliedVisibleMs`를 얼마나 줄였는지는 다음 실장비 session에서 다시 판정해야 한다.

### 2026-04-22 16:07 +09:00 latest 실행은 preview latency가 아니라 accepted-only `capture-in-flight` stall이었고, stale helper restart/recovery 경계를 추가했다

사용자 최신 요청:

1. 방금 앱을 실행했으니 latest 로그를 확인하고 문제를 해결하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a89b380d42939c`였다.
- 이 session의 `timing-events.log`에는 `request-capture`만 남았고,
  `file-arrived`, `capture-persisted`, `preview-render-ready` 같은 후속 event는 없었다.
- `camera-helper-events.jsonl`도 `capture-accepted`까지만 남았고,
  helper는 요청을 거절하지 않았다.
- 반면 `camera-helper-status.json` 마지막 상태는
  `cameraState=capturing`,
  `helperState=healthy`,
  `detailCode=capture-in-flight`
  였고,
  `observedAt=2026-04-22T06:52:06.2677440+00:00` 기준 stale 상태로 멈춰 있었다.
- `camera-helper-processed-request-ids.txt`에는 해당 request id
  `request_000000000018a89b39b53e6048`
  가 남아 있었고,
  `session.json`은 `captures=[]`, `lifecycle.stage=phone-required`로 닫혔다.
- 따라서 이번 latest 실행은 preview latency 문제가 아니라,
  helper가 capture request를 소비한 뒤 `capture-in-flight`에서 멈춘 accepted-only stall family였다.

이번 회차 조치:

- helper supervisor는 stale `capture-in-flight` healthy status를
  약 `45초` 이후 restart 대상에 포함하도록 보강했다.
- readiness 복구는 `phone-required`가 곧바로 풀리도록 넓게 열지 않고,
  저장된 capture가 없더라도
  실제로 helper가 request를 소비했다는 `processed request id` evidence가 있을 때만
  helper ready 복귀 뒤 `capture-ready`로 되돌리게 좁혔다.
- 그래서 일반 `phone-required` timeout은 그대로 유지하면서,
  이번 latest처럼 request consumption 이후 멈춘 session만 retryable하게 다시 열 수 있게 정리했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness capture_flow_times_out_when_helper_accepts_but_no_file_arrives -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_without_saved_capture_once_helper_is_ready_again -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml stale_capture_in_flight_status_requests_a_helper_restart`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`

이번 시점 제품 판단:

1. latest 현장 실행은 preview latency family가 아니라, accepted-only helper stall이었다.
2. current code는 이 stall을 stale helper restart와 evidence-gated readiness recovery로 다시 시도 가능한 상태까지 끌어올리도록 보강됐다.
3. 다만 이 최신 복구 보강 뒤 hardware rerun은 아직 없으므로, 다음 실제 실행에서 같은 family가 자동으로 해소되는지 다시 확인해야 한다.

### 2026-04-23 14:46 +09:00 latest app session은 startup 실패가 아니라 first-shot truthful-close cold spike였고, current code는 JPEG warm-up으로 첫 컷 miss를 줄이도록 보강됐다

사용자 최신 요청:

1. 최신 앱 실행 session 로그를 확인하고 story `1-26`, ledger, 관련 문서에 반영한 뒤 개선하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e59c3f873ffc`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔고 `fastPreviewKind=windows-shell-thumbnail`였다.
- direct metric은 전반 실패가 아니라 first-shot cold spike 패턴으로 갈렸다.
  - shot 1:
    - `preview-render-ready elapsedMs=15975`
    - `originalVisibleToPresetAppliedVisibleMs=16066`
    - `capture_preview_ready elapsedMs=18123`
  - shots 2~5:
    - `preview-render-ready elapsedMs=3417`, `3315`, `3214`, `3315`
    - `originalVisibleToPresetAppliedVisibleMs=3436`, `3360`, `3277`, `3355`
    - `capture_preview_ready elapsedMs=5475`, `5391`, `5268`, `5407`

이번 회차 해석:

- latest app run은 startup/save/helper truth가 무너진 회귀가 아니었다.
- 2장~5장이 이미 `3.2s ~ 3.4s` band까지 내려온 점을 보면,
  reserve path 전체가 막힌 것이 아니라 first shot만 warm-up을 다시 놓친 쪽으로 읽는 편이 맞다.
- 따라서 current blocker는 general steady-state slowdown 하나가 아니라,
  **first-shot truthful-close cold miss를 먼저 later-shot band로 끌어내리고,
  그 다음 남는 소폭 steady-state gap을 줄이는 일**로 다시 좁혀졌다.

이번 회차 조치:

- story `1-26`, hardware validation ledger, preview latency checklist에 latest session evidence를 반영했다.
- preview renderer warm-up source를 tiny PNG가 아니라 JPEG raster로 바꿨다.
- 의도는 첫 실전 컷이 실제 fast-preview lane과 다른 decoder cold-start를 다시 내지 않도록,
  warm-up 자체를 same raster family로 맞추는 것이다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`

이번 시점 제품 판단:

1. latest app session은 reserve path가 전반 실패한 것이 아니라, first shot만 크게 튄 cold-start evidence였다.
2. current code는 first-shot miss를 줄이기 위해 warm-up source를 JPEG raster로 맞춘 상태다.
3. 다음 판단은 새 hardware/app rerun에서 첫 컷이 먼저 later-shot band 안으로 내려오는지 확인한 뒤, 남는 `3.2s ~ 3.4s` gap을 추가로 줄일지 정하는 것이다.

### 2026-04-23 14:59 +09:00 latest app session에서는 JPEG warm-up 뒤 first-shot cold spike가 사라졌고, blocker는 다시 steady-state truthful-close gap만 남았다

사용자 최신 요청:

1. 최신 앱 실행 session 로그를 확인하고 기록한 뒤, 코드 개선 후 하드웨어 검증 스크립트도 실행하라고 요청했다.

실제 확인 근거:

- latest session은 `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e6cb585230d4`였다.
- startup/connect는 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔다.
- direct metric은 first-shot failure가 아니라 전 컷 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3415`, `3314`, `3320`, `3414`, `3314`
  - `originalVisibleToPresetAppliedVisibleMs`: `3441`, `3366`, `3360`, `3439`, `3356`
  - `capture_preview_ready elapsedMs`: `5614`, `5429`, `5352`, `5434`, `5387`

이번 회차 해석:

- 직전 latest `session_000000000018a8e59c3f873ffc`에서 보였던 first-shot cold spike `16066ms`는 이번 latest rerun에서 사라졌다.
- 따라서 JPEG warm-up 보강은 first-shot truthful-close miss를 줄이는 데 실제로 효과가 있었던 것으로 읽는 편이 맞다.
- current blocker는 다시 first shot만의 문제보다,
  전 컷에 공통으로 남은 `3356ms ~ 3441ms` steady-state truthful-close gap이다.

이번 회차 조치:

- story `1-26`, hardware validation ledger, preview latency checklist에 latest session evidence를 다시 반영했다.
- current code change 자체는 유지하고, 이번 latest session을 JPEG warm-up 효과 검증 evidence로 고정했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`

이번 시점 제품 판단:

1. latest app session은 first-shot miss가 아니라, 전 컷 공통 steady-state gap만 남았다는 evidence다.
2. current code의 JPEG warm-up 보강은 first-shot cold spike 제거 쪽에서는 효과가 확인됐다.
3. 다음 판단은 새 hardware rerun에서 전 컷 `originalVisibleToPresetAppliedVisibleMs`가 `<= 3000ms`로 내려오는지로 닫아야 한다.

### 2026-04-23 15:03 +09:00 hardware validation runner latest session도 5/5 pass였고, first-shot extreme spike는 재발하지 않았지만 steady-state gap은 그대로 남았다

사용자 최신 요청:

1. 코드 개선 후 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e716e9987b48`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e716e9987b48`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3514`, `3516`, `3414`, `3514`, `3616`
  - `originalVisibleToPresetAppliedVisibleMs`: `3606`, `3598`, `3439`, `3600`, `3679`
  - `capture_preview_ready elapsedMs`: `5721`, `5616`, `5485`, `5617`, `5707`

이번 회차 해석:

- JPEG warm-up 보강 뒤 first-shot `16066ms` 급 cold spike는 latest hardware-validation-runner session에서도 재발하지 않았다.
- 즉 first-shot correctness 방향은 이전보다 안정화된 것으로 읽는 편이 맞다.
- 하지만 다섯 컷 모두 여전히 `3439ms ~ 3679ms`에 머물러,
  current blocker는 계속 steady-state truthful-close latency다.

이번 회차 조치:

- story `1-26`, hardware validation ledger, preview latency checklist를 runner latest session 기준으로 다시 갱신했다.
- hardware validation runner 결과도 canonical evidence에 연결했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과와 first-shot spike 미재발을 보여 줬다.
2. 하지만 official gate 관점에서는 여전히 `<= 3000ms`를 넘기고 있어 Story `1.26`은 계속 `No-Go`다.

### 2026-04-23 15:09 +09:00 hardware validation runner latest session에서도 first-shot spike는 없었고, preview truthful-close는 opencl-disabled 상태로 steady-state gap만 남았다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 다시 확인해 story `1-26`, ledger, 관련 문서에 기록하고 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e7702849122c`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e7702849122c`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔다.
- latest `timing-events.log` render invocation args에는 `--disable-opencl`이 실제로 남았다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3517`, `3414`, `3313`, `3414`, `3415`
  - `originalVisibleToPresetAppliedVisibleMs`: `3612`, `3437`, `3356`, `3439`, `3443`
  - `capture_preview_ready elapsedMs`: `5741`, `5505`, `5395`, `5477`, `5474`

이번 회차 해석:

- JPEG warm-up 보강 뒤 first-shot `16066ms` 급 cold spike는 latest hardware-validation-runner session에서도 재발하지 않았다.
- 이번 추가 보강으로 booth-visible truthful close는 실제 세션에서도 OpenCL startup 없이 실행된 것이 확인됐다.
- 하지만 다섯 컷 모두 여전히 `3356ms ~ 3612ms`에 머물러, current blocker는 계속 steady-state truthful-close latency다.

이번 회차 조치:

- preview truthful-close path가 작은 booth-visible render에서 불필요한 OpenCL startup cost를 먼저 물지 않도록 보강했다.
- story `1-26`, hardware validation ledger, preview latency checklist, validation history를 latest session 기준으로 다시 갱신했다.
- hardware validation runner 결과와 latest session evidence를 canonical 문서들에 함께 연결했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_uses_display_sized_render_arguments -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_prefers_same_capture_raster_when_available -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과, first-shot spike 미재발, 실제 `--disable-opencl` 적용까지 보여 줬다.
2. 하지만 official gate 관점에서는 여전히 `<= 3000ms`를 넘기고 있어 Story `1.26`은 계속 `No-Go`다.

### 2026-04-23 15:30 +09:00 hardware validation runner latest session에서는 preview in-memory library까지 적용됐고, first-shot은 steady-state band 안으로 더 내려왔다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 다시 확인해 story `1-26`, ledger, 관련 문서에 기록하고 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e892447836f8`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e892447836f8`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔다.
- latest `timing-events.log` render invocation args에는 `--disable-opencl`, `--library :memory:`가 실제로 남았다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 더 좁은 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3417`, `3315`, `3314`, `3415`, `3418`
  - `originalVisibleToPresetAppliedVisibleMs`: `3441`, `3366`, `3354`, `3439`, `3443`
  - `capture_preview_ready elapsedMs`: `5594`, `5340`, `5404`, `5521`, `5526`

이번 회차 해석:

- preview in-memory library 보강 뒤 first-shot은 `3612ms -> 3441ms`로 내려와 later-shot band 안에 더 안정적으로 들어왔다.
- 즉 latest 회차는 first-shot special-case가 아니라, 전 컷이 거의 같은 `3354ms ~ 3443ms` band에 모인 상태로 읽는 편이 맞다.
- 하지만 official gate `<= 3000ms`는 아직 넘고 있어 current blocker는 계속 steady-state truthful-close latency다.

이번 회차 조치:

- preview truthful-close path가 preview 전용 persistent sqlite startup cost를 매번 물지 않도록 `--library :memory:`를 적용했다.
- story `1-26`, hardware validation ledger, preview latency checklist, validation history를 latest session 기준으로 다시 갱신했다.
- hardware validation runner 결과와 latest session evidence를 canonical 문서들에 함께 연결했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_uses_display_sized_render_arguments -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_prefers_same_capture_raster_when_available -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml final_invocation_keeps_full_resolution_render_arguments -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과, first-shot spike 미재발, 실제 `--disable-opencl` + `--library :memory:` 적용까지 보여 줬다.
2. 하지만 official gate 관점에서는 여전히 `<= 3000ms`를 넘기고 있어 Story `1.26`은 계속 `No-Go`다.
3. 다음 개선은 first-shot이 아니라, 전 컷 공통 `3.35s ~ 3.44s` steady-state cost를 더 줄이는 쪽이어야 한다.

### 2026-04-23 15:39 +09:00 hardware validation runner latest session에서도 gate는 닫히지 않았고, speculative source hard-link 시도만으로는 steady-state gap이 줄지 않았다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 다시 확인해 story `1-26`, ledger, 관련 문서에 기록하고 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e91cef5631a8`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e91cef5631a8`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔다.
- latest `timing-events.log` render invocation args에는 여전히 `--disable-opencl`, `--library :memory:`가 실제로 남았다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3414`, `3315`, `3517`, `3421`, `3320`
  - `originalVisibleToPresetAppliedVisibleMs`: `3440`, `3356`, `3599`, `3440`, `3356`
  - `capture_preview_ready elapsedMs`: `5546`, `5443`, `5666`, `5425`, `5483`

이번 회차 해석:

- first-shot extreme spike는 latest session에서도 재발하지 않았다.
- 하지만 same-volume speculative source copy를 hard link 우선으로 바꾼 이번 시도만으로는 steady-state band를 더 낮추지 못했다.
- latest 회차도 official gate `<= 3000ms`는 넘고 있어 current blocker는 계속 steady-state truthful-close latency다.

이번 회차 조치:

- speculative preview source staging이 같은 세션 디렉터리 안에서는 새 파일 복사보다 hard link를 먼저 시도하도록 보강했다.
- story `1-26`, hardware validation ledger, preview latency checklist, validation history를 latest session 기준으로 다시 갱신했다.
- latest runner evidence를 canonical 문서들에 다시 연결했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_source_is_staged_to_a_stable_copy -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_wait_budget_stays_bounded_without_in_flight_capture -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_wait_budget_stays_bounded_even_while_another_capture_is_in_flight -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과와 first-shot spike 미재발을 다시 보여 줬다.
2. 하지만 latest band가 `3356ms ~ 3599ms`로 남아 있어 Story `1.26`은 계속 `No-Go`다.
3. 방금 시도한 hard-link staging은 채택할 만한 gate-closing evidence를 만들지 못했고, 다음 개선은 여전히 render 본체 steady-state cost를 더 줄이는 쪽이어야 한다.
3. 다음 개선은 first-shot warm-up이 아니라, 전 컷 공통 steady-state truthful-close cost를 줄이는 쪽이어야 한다.

### 2026-04-23 21:57 +09:00 hardware validation runner latest session에서 192x192 truthful-close 축소 실험은 gate를 닫지 못했고 current worktree에는 채택하지 않았다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 다시 확인해 story `1-26`, ledger, 관련 문서에 기록하고 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- 먼저 same-capture truthful-close raster cap을 experimental `192x192`로 줄인 뒤
  `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8fdb7a8e88590`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fdb7a8e88590`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- 같은 session의 `session.json`과 helper correlation을 보면 5컷 모두
  `renderStatus=previewReady`,
  `preview.kind=preset-applied-preview`,
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔다.
- latest `timing-events.log` render invocation args에는 `--disable-opencl`, `--library :memory:`, `--width 192`, `--height 192`가 실제로 남았다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3515`, `3416`, `3515`, `3515`, `3415`
  - `originalVisibleToPresetAppliedVisibleMs`: `3523`, `3434`, `3599`, `3602`, `3437`
  - `capture_preview_ready elapsedMs`: `6632`, `6905`, `6789`, `7042`, `6617`

이번 회차 해석:

- first-shot extreme spike는 latest session에서도 재발하지 않았다.
- 하지만 `192x192` truthful-close 축소 실험은 accepted band를 더 낮추지 못했고, 일부 컷은 오히려 더 느려졌다.
- 따라서 current blocker는 여전히 steady-state truthful-close latency이며, 단순 raster cap 축소는 채택 가능한 방향이 아니다.

이번 회차 조치:

- experimental `192x192` cap으로 latest hardware rerun evidence를 수집했다.
- story `1-26`, hardware validation ledger, preview latency checklist, validation history를 latest session 기준으로 다시 갱신했다.
- 이 실험은 product 기준에서 reject하고 current worktree는 same-capture truthful close cap을 다시 `256x256`으로 롤백했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_prefers_same_capture_raster_when_available -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_wait_budget_stays_bounded_even_while_another_capture_is_in_flight -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과와 first-shot spike 미재발을 다시 보여 줬다.
2. 하지만 latest rejected band가 `3434ms ~ 3602ms`로 남아 있어 Story `1.26`은 계속 `No-Go`다.
3. 방금 시도한 `192x192` truthful-close 축소는 채택할 만한 개선을 만들지 못했고, current worktree에는 남기지 않았다.
4. 다음 개선은 해상도 추가 축소가 아니라, 전 컷 공통 steady-state truthful-close cost를 줄이는 쪽이어야 한다.

### 2026-04-23 22:13 +09:00 hardware validation runner latest session에서는 preview fast-preview-raster lane가 trimmed XMP cache를 실제로 사용했고 steady-state band가 조금 더 낮아졌지만 gate는 아직 닫히지 않았다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 다시 확인해 story `1-26`, ledger, 관련 문서에 기록하고 다음 시도해야 할 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- fast-preview-raster preview lane가 raw-only darktable history 일부를 덜어낸 cached XMP를 쓰도록 보강한 뒤
  `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8fe95ea36f8f4`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- helper correlation을 보면 5컷 모두
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
- latest `timing-events.log` render invocation args에는 `--disable-opencl`, `--library :memory:`와 함께
  `.boothy-darktable/preview/xmp-cache/preset-new-draft-2-2026-04-10-look2-fast-preview.xmp`
  가 실제로 남았다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 조금 더 낮은 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3317`, `3315`, `3314`, `3215`, `3316`
  - `originalVisibleToPresetAppliedVisibleMs`: `3358`, `3368`, `3357`, `3284`, `3361`
  - `capture_preview_ready elapsedMs`: `5757`, `5668`, `5632`, `5561`, `5623`

이번 회차 해석:

- first-shot extreme spike는 latest session에서도 재발하지 않았다.
- preview fast-preview-raster lane가 lighter XMP를 실제로 쓴 것은 확인됐고, steady-state band도 accepted `256x256` evidence보다 약간 더 내려왔다.
- 하지만 official gate `<= 3000ms`는 여전히 넘고 있어 current blocker는 계속 steady-state truthful-close latency다.

이번 회차 조치:

- preview fast-preview-raster lane가 raw-only darktable history 일부를 덜어낸 cached XMP를 실제 invocation에 쓰도록 보강했다.
- story `1-26`, hardware validation ledger, preview latency checklist, validation history를 latest session 기준으로 다시 갱신했다.
- latest runner evidence를 canonical 문서들에 다시 연결했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_xmp_trim_removes_raw_only_operations_from_history_and_iop_order -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_trimmed_cached_xmp_when_source_xmp_is_available -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_prefers_same_capture_raster_when_available -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source_is_written_as_jpeg -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과와 first-shot spike 미재발, 그리고 trimmed XMP cache 실제 적용을 함께 보여 줬다.
2. 하지만 latest band가 `3284ms ~ 3368ms`로 여전히 official gate 밖이라 Story `1.26`은 계속 `No-Go`다.
3. 다음 개선은 추가 해상도 축소가 아니라, darktable truthful-close fixed cost를 더 줄이거나 host-owned truthful close owner를 더 앞당기는 쪽이어야 한다.

### 2026-04-24 10:00 +09:00 hardware validation runner helper-bootstrap recovery 후 단일 실행은 5/5 통과했지만 official gate는 latency tail 때문에 아직 No-Go다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 확인해 story `1-26`, ledger, 관련 문서에 기록하고 다음 시도해야 할 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- 직전 실패 세션 `session_000000000018a92491e8b75984`, `session_000000000018a924971612f514`는 모두 `capture-readiness-timeout`으로 실패했고 촬영 샘플이 없었다.
- failure diagnostics 수집 시점에는 helper status/startup log가 없었으며, 한 세션은 실패 뒤에야 helper가 늦게 `camera-ready`를 기록했다.
- current worktree는 hardware validation runner의 direct library path에서도 missing helper status가 1초 이상 지속되면 helper bootstrap을 직접 요청하게 보강했다.
- `cargo test --manifest-path src-tauri/Cargo.toml --test hardware_validation_runner -- --test-threads=1` 통과 뒤 요청 커맨드를 한 번 실행했다.

검증 결과:

- runner summary: `status=passed`, `capturesPassed=5/5`
- run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776992377859\run-summary.json`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a925271b1710a0`
- startup/connect: `sdk-initializing -> session-opening -> camera-ready`
- direct metric:
  - `preview-render-ready elapsedMs`: `3014`, `2912`, `2923`, `3214`, `2915`
  - `originalVisibleToPresetAppliedVisibleMs`: `3037`, `2952`, `2974`, `3279`, `2954`
  - `capture_preview_ready elapsedMs`: `5237`, `5157`, `5071`, `5331`, `4995`

이번 시점 제품 판단:

1. latest 단일 실행은 runner-side readiness timeout family를 재현하지 않았다.
2. 하지만 official gate는 `3037ms`, `3279ms` tail miss 때문에 아직 닫히지 않았다.
3. 다음 개선은 readiness가 아니라 truthful-close latency tail jitter를 더 줄이는 방향이어야 한다.

### 2026-04-24 10:19 +09:00 compact prompt parsing 보강 뒤 최신 하드웨어 검증은 5/5 통과했지만 Story 1.26은 latency tail 때문에 아직 No-Go다

최신 실행에서 확인한 점:

- 직전 runner summary는 `Kim4821`을 `Kim4821 0000`으로 저장해 고객 식별자가 틀어졌다.
- runner parsing 보강 뒤 새 session `session_000000000018a92639f9a96a6c`는 `Kim 4821`로 저장됐다.
- 요청 커맨드는 `status=passed`, `capturesPassed=5/5`로 닫혔다.
- 5컷 모두 `previewReady`와 `preset-applied-preview`로 닫혔다.
- direct metric은 `3200`, `2956`, `2955`, `2958`, `3276`ms였다.

판단:

- prompt/readiness 문제는 이번 단일 실행에서 재현되지 않았다.
- 하지만 official `<= 3000ms` gate는 2컷 tail miss로 아직 닫히지 않았다.
- 다음 시도는 고객 식별자나 helper bootstrap이 아니라 truthful-close latency tail을 줄이는 방향이다.

### 2026-04-24 07:58 +09:00 hardware validation runner latest session에서 OpenCL disable core-option 순서 수정은 gate에 가장 가까운 결과를 만들었지만 full package는 아직 No-Go다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 확인해 story `1-26`, ledger, 관련 문서에 기록하고 다음 시도해야 할 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- latest app session `session_000000000018a8fe95ea36f8f4`의 invocation args를 다시 보니 `--disable-opencl`이 `--core` 앞에 있어 darktable core option으로 확실히 적용됐다고 보기 어려웠다.
- current worktree는 preview invocation 순서를 `--width 256 --height 256 --core --disable-opencl --configdir ... --library :memory:`로 고쳤다.
- `cargo test preview_invocation_uses_display_sized_render_arguments --manifest-path src-tauri/Cargo.toml`로 관련 테스트를 red/green 확인했다.
- 요청한 하드웨어 검증 커맨드는 한 번 실행했고 runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a91e89791d5370`였다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a91e89791d5370`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - helper correlation: 5컷 모두 `capture-accepted -> file-arrived -> fast-preview-ready`
- direct metric은 current route에서 가장 낮은 band로 읽혔다.
  - `preview-render-ready elapsedMs`: `2916`, `2913`, `3019`, `3115`, `2913`
  - `originalVisibleToPresetAppliedVisibleMs`: `2953`, `2960`, `3039`, `3197`, `2953`
  - `capture_preview_ready elapsedMs`: `5144`, `5024`, `4929`, `5119`, `4990`

이번 회차 해석:

- 3/5컷은 official `<= 3000ms` gate 안에 들어왔다.
- 하지만 2컷이 `3039ms`, `3197ms`로 남아 Story `1.26`은 full package 기준 아직 `No-Go`다.
- 다음 개선은 startup/save/first-shot이 아니라 darktable truthful-close tail jitter를 약 200ms 더 줄이는 방향이어야 한다.

### 2026-04-24 10:32 +09:00 extra fast-preview XMP trimming 후 최신 하드웨어 검증은 tail을 크게 줄였지만 Story 1.26은 아직 No-Go다

최신 실행에서 확인한 점:

- 직전 latest session `session_000000000018a92639f9a96a6c`는 prompt/readiness가 아니라 render tail 때문에 official gate를 놓쳤다.
- current worktree는 fast-preview JPEG truthful-close XMP에서 `lens`, `highlights`, `cacorrectrgb`를 추가 제거했다.
- 요청 커맨드는 한 번 실행했고 `status=passed`, `capturesPassed=5/5`로 닫혔다.
- latest session `session_000000000018a926e98958c25c`는 `Kim 4821` 식별자와 5/5 `preset-applied-preview` truthful close를 유지했다.
- direct metric은 `3039`, `2955`, `3034`, `3032`, `2956`ms였다.

판단:

- 최대 tail은 `3276ms`에서 `3039ms`로 줄었다.
- 하지만 3컷이 official `<= 3000ms` gate를 `32ms ~ 39ms` 넘어서 Story `1.26`은 아직 `No-Go`다.
- 다음 시도는 남은 `40ms` 안팎 tail을 줄이는 쪽이며, cached XMP에 남은 duplicate builtin/default work를 시각 차이 없이 줄이는지부터 봐야 한다.

### 2026-04-24 11:36 +09:00 fast-preview cached XMP iop-order trimming 후 Story 1.26 hardware gate가 Go로 닫혔다

최신 실행에서 확인한 점:

- 직전 latest session `session_000000000018a9292e867e1a68`는 5/5 capture와 `preset-applied-preview` truth owner는 유지했지만 `3035ms`, `3036ms`, `3039ms` tail miss가 남았다.
- cached XMP history는 이미 9개 operation으로 줄었지만 `iop_order_list`에는 history에서 제거된 default pipeline 항목이 계속 남아 있었다.
- current worktree는 fast-preview cached XMP의 `iop_order_list`를 실제 유지된 preview history operation/priority만 남기도록 줄였다.
- 요청 커맨드는 한 번 실행했고 `status=passed`, `capturesPassed=5/5`로 닫혔다.
- latest session `session_000000000018a92a6c02e7f2d4`는 `Kim 4821` 식별자와 5/5 `preset-applied-preview` truthful close를 유지했다.
- direct metric:
  - `preview-render-ready elapsedMs`: `2916`, `2914`, `2918`, `2919`, `2915`
  - `originalVisibleToPresetAppliedVisibleMs`: `2956`, `2951`, `2961`, `2954`, `2960`
  - `capture_preview_ready elapsedMs`: `5010`, `4865`, `5124`, `4999`, `5153`

판단:

- Story `1.26`의 official `<= 3000ms` gate는 latest hardware package에서 닫혔다.
- 다음 시도는 추가 tail tuning이 아니라 visual acceptability 확인과 Story `1.31` success-side default / rollback gate 판단이다.
