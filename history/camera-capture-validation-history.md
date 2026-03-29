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

## 관련 파일 / 문서

- [camera-helper-troubleshooting-history.md](/C:/Code/Project/Boothy/history/camera-helper-troubleshooting-history.md)
- [camera-helper-sidecar-protocol.md](/C:/Code/Project/Boothy/docs/contracts/camera-helper-sidecar-protocol.md)
- [capture-runtime.ts](/C:/Code/Project/Boothy/src/capture-adapter/services/capture-runtime.ts)
- [capture-runtime.test.ts](/C:/Code/Project/Boothy/src/capture-adapter/services/capture-runtime.test.ts)
- [session-provider.test.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.test.tsx)
- [CanonHelperService.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs)
- [JsonFileProtocol.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs)
- [SessionPaths.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/SessionPaths.cs)

