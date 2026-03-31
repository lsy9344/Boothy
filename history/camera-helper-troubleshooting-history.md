# Camera Helper Troubleshooting History

## 목적

이 문서는 Boothy의 실카메라/helper/readiness 문제를 다음 에이전트가 빠르게 이어서 처리할 수 있게 만들기 위한 운영 이력이다.

목표는 세 가지다.

1. 이미 겪은 문제를 다시 처음부터 추정하지 않게 한다.
2. 증상별로 먼저 확인해야 할 파일, 프로세스, 명령을 고정한다.
3. "실제 원인"과 "헷갈리기 쉬운 오진"을 분리해 중복 작업을 줄인다.

## 현재까지 확인된 핵심 문제

### 1. 앱이 helper를 자동으로 붙이지 못했던 문제

- 초기 상태에서는 Rust/Tauri host가 helper status 파일은 읽었지만, 실제 `canon-helper.exe`를 active session에 맞춰 자동 실행하지 않았다.
- 그 결과 카메라 전원을 나중에 켜도 status 파일이 생성되지 않아 booth/operator가 runtime truth를 갱신하지 못했다.

해결:

- session start
- booth readiness 조회
- capture 요청
- operator diagnostics / recovery 진입

경로에서 helper supervisor를 붙여 현재 session 기준 helper를 자동 실행하도록 수정했다.

관련 파일:

- [helper_supervisor.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/helper_supervisor.rs)
- [session_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/session_commands.rs)
- [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs)
- [operator_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/operator_commands.rs)
- [lib.rs](/C:/Code/Project/Boothy/src-tauri/src/lib.rs)

### 2. helper status JSON이 UTF-8 BOM 때문에 `invalid-status`로 버려지던 문제

- 실제 helper는 JSON 파일을 UTF-8 BOM 포함 형식으로 썼다.
- Rust host의 status/event parser는 BOM을 제거하지 않아 valid JSON도 `invalid-status`로 판단했다.
- 이 상태에서는 helper가 실제로 `ready/healthy`여도 host는 `helper-preparing` 또는 blocked path로만 내려갔다.

실제 관찰 포인트:

- `camera-helper-status.json` 내용은 정상처럼 보인다.
- 하지만 host에서 직접 readiness를 계산하면 `detail_code=invalid-status`가 나온다.

해결:

- Rust host parser가 status/event line 앞의 BOM을 제거하도록 수정했다.
- helper도 이후에는 BOM 없이 파일을 쓰도록 바꿨다.

관련 파일:

- [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs)
- [JsonFileProtocol.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs)
- [camera-helper-sidecar-protocol.md](/C:/Code/Project/Boothy/docs/contracts/camera-helper-sidecar-protocol.md)

### 3. freshness 파서가 `Z`만 허용해서 helper status를 계속 stale로 보던 문제

- helper는 `observedAt`를 `2026-03-27T19:37:08.3928625+00:00` 같은 형식으로 쓴다.
- 기존 `rfc3339_to_unix_seconds(...)`는 `...Z` 형식만 허용했다.
- 그래서 parser가 offset 형식을 읽지 못했고, freshness 계산이 항상 실패해 `ready` 상태도 `stale`로 간주됐다.
- 이 경우 helper status JSON은 정상이고 session match도 맞지만, host readiness는 `Preparing / camera-preparing`로 떨어진다.

실제 관찰 포인트:

- helper status 파일에는 `cameraState=ready`, `helperState=healthy`
- host readiness 직접 계산 결과는 `freshness=stale`

해결:

- `rfc3339_to_unix_seconds(...)`가 `Z`와 `+00:00` 같은 offset 형식을 모두 읽도록 수정했다.

관련 파일:

- [session_manifest.rs](/C:/Code/Project/Boothy/src-tauri/src/session/session_manifest.rs)
- [session_manifest.rs test](/C:/Code/Project/Boothy/src-tauri/tests/session_manifest.rs)

### 4. 프런트가 일반 `phone-required`까지 post-end 확정 상태처럼 붙잡던 문제

- session provider는 `phone-required`를 post-end finalized reason과 같은 강도로 보존했다.
- 그래서 일시적인 host 보호 상태가 들어오면, 뒤이어 `Preparing`이나 `Ready`가 와도 화면이 쉽게 풀리지 않을 수 있었다.
- 이건 "종료 후 확정된 보호 상태"와 "촬영 중 일시 보호 상태"를 프런트가 구분하지 못한 문제다.

해결:

- explicit `postEnd`가 없는 일반 blocked `phone-required`는 영구 보존하지 않도록 완화했다.
- `postEnd`가 명시된 finalized case만 보존한다.

관련 파일:

- [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx)
- [session-provider.test.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.test.tsx)

### 5. 현재 남아 있는 문제: 최신 세션 helper status가 `disconnected`로 고정되는 사례

- `Phone Required` 과보호는 줄어들어 현재 customer 화면이 `Preparing`으로 바뀌는 것까지는 확인됐다.
- 하지만 최신 dev 실행에서 실제 active session의 helper status가 `ready`가 아니라 `disconnected`로 기록되는 사례가 다시 확인됐다.
- 이 경우 booth 화면이 `Preparing / 촬영 준비 상태를 다시 확인하고 있어요.`에 머무르는 것은 프런트 캐시 때문이 아니라, helper가 카메라를 다시 찾지 못했다고 host에 보고하고 있기 때문이다.

실제 관찰 예시:

- 실행 프로세스
  - `boothy.exe` 시작 시각: `2026-03-28 10:53`
  - `canon-helper.exe` 시작 시각: `2026-03-28 10:53`
- 최신 session
  - `session_000000000018a0de72f92dd220`
- 해당 session의 helper status
  - `cameraState: "disconnected"`
  - `helperState: "healthy"`
  - `detailCode: "camera-not-found"`

해석:

- 이 단계에서는 host freshness/parser 문제보다 helper의 실제 camera discovery truth를 우선 의심해야 한다.
- 즉 "화면이 예전 보호 상태를 붙잡는다"가 핵심이 아니라, helper가 최신 세션에서도 카메라를 못 찾는다고 계속 쓰는 것이 현재 본문제다.
- 사용자가 말한 "전원 on/off를 해도 동일" 증상도 이 관찰과 일치한다.

다음 확인 우선순위:

1. 최신 session의 `camera-helper-status.json`이 정말 `ready`로 올라오는 순간이 있는지 먼저 본다.
2. helper process command line이 최신 session으로 바인딩돼 있는지 다시 확인한다.
3. helper가 카메라 전원 off 상태에서 시작된 뒤 on으로 바뀌어도 `EdsGetCameraList` 결과가 계속 0인지 확인한다.
4. 필요하면 `canon-helper.exe --self-check`와 runtime mode의 camera discovery 동작 차이를 비교한다.
5. helper 쪽 reconnect/detection loop를 강화하거나, camera-not-found가 오래 지속될 때 session-bound helper 재기동 정책을 추가하는 쪽을 검토한다.

이번 단계에서 분리된 사실:

- host 직접 readiness 계산이 `Ready|ready|true|captureReady`로 나오는 session도 있었고, 그 문제는 freshness/parser 쪽 수정으로 해결됐다.
- 하지만 최신 dev 실행에서는 별도의 새 session에서 helper status 자체가 `disconnected`로 기록되는 사례가 다시 생겼다.
- 따라서 앞으로는 "host가 ready를 못 읽는다"와 "helper가 camera-not-found를 보고한다"를 서로 다른 이슈로 분리해 봐야 한다.

### 6. 2026-03-28 추가 확인: Windows에 Canon 항목이 보여도 실제 `IsPresent=false`일 수 있다

- 이번 회차에서 helper `self-check`와 session-bound runtime helper 모두 `cameraCount: 0`, `detailCode: camera-not-found`를 반환했다.
- 같은 시각 Windows `Get-PnpDevice`에는 `Canon EOS 700D`가 보였지만, `Get-PnpDeviceProperty`로 확인한 핵심 값은 아래와 같았다.
  - `DEVPKEY_Device_FriendlyName = Canon EOS 700D`
  - `DEVPKEY_Device_InstanceId = USB\\VID_04A9&PID_3272\\5&17411534&0&9`
  - `DEVPKEY_Device_IsPresent = false`
  - `DEVPKEY_Device_LastArrivalDate = 2026-03-28 04:21:49`
  - `DEVPKEY_Device_LastRemovalDate = 2026-03-28 10:53:52`

해석:

- Windows 장치 목록에 Canon 항목이 남아 있다고 해서, helper가 실제 present device를 잡을 수 있다는 뜻은 아니다.
- 이 상태에서는 "카메라 전원이 켜져 보인다"는 육안 정보와 별개로, host/helper 입장에서는 여전히 `disconnected`가 맞다.
- 즉 current booth symptom이 generic parser bug가 아니라, **Windows/PnP 기준으로도 현재 연결이 살아 있지 않은 상태**일 수 있다.

이번 회차 코드 보정:

- helper는 `camera-not-found`를 쓸 때 Windows 장치 존재 여부를 추가 probe하도록 보강했다.
- booth readiness는 `cameraState=disconnected`일 때 generic `Preparing` 문구 대신, 고객에게 `카메라 전원을 확인하고 있어요.`라는 더 구체적인 안내를 보여 주도록 조정했다.
- 반대로 helper가 `connecting` 또는 `connected-idle`를 주면 `카메라를 확인했고 연결을 마무리하고 있어요.` 쪽 copy로 구분된다.

진단 원칙 갱신:

- `Get-PnpDevice`에 Canon 문자열이 보이더라도 바로 "실제 연결됨"으로 해석하지 말 것.
- 가능하면 `Get-PnpDeviceProperty <instanceId>`에서 `DEVPKEY_Device_IsPresent`를 먼저 본다.
- `IsPresent=false`이면 helper의 `disconnected/camera-not-found`는 host bug보다 실제 Windows 연결 부재일 가능성이 높다.

### 7. 2026-03-28 추가 확인: helper는 이미 `ready`인데 booth 화면만 `Preparing`에 머무르는 사례

- 사용자가 `pnpm tauri dev --no-watch`로 새 인스턴스를 다시 띄운 뒤, 카메라 전원을 켠 후에도 booth 화면이 바뀌지 않는 사례를 재확인했다.
- 같은 시각 최신 session `session_000000000018a0df9dd1bc0f04`의 실제 helper status는 아래처럼 기록돼 있었다.
  - `cameraState: "ready"`
  - `helperState: "healthy"`
  - `cameraModel: "Canon EOS 700D"`
  - `detailCode: "camera-ready"`
- 즉 이 시점의 본문제는 helper readiness truth 자체가 아니라, **프런트가 Tauri runtime을 browser fallback처럼 오인해 host readiness를 반영하지 못하는 가능성**으로 좁혀졌다.

근거:

- repo 전반의 여러 서비스가 Tauri 판별을 `__TAURI_INTERNALS__ in window` 하나에만 의존하고 있었다.
- 같은 패턴이 `capture-runtime`, `start-session`, `active-preset`, `operator-diagnostics`, `runtime-capability`, `main.tsx` 등 여러 경계에 반복돼 있었다.
- 실제 Tauri dev 앱인데도 booth 화면이 helper truth와 분리되어 움직인 증상은 이 판별 취약성과 잘 맞는다.

이번 회차 코드 보정:

- 공통 유틸 [is-tauri.ts](/C:/Code/Project/Boothy/src/shared/runtime/is-tauri.ts)를 추가했다.
- 판별 기준을 `__TAURI_INTERNALS__`만 보지 않고 `__TAURI__`, `__TAURI_IPC__`, `navigator.userAgent`의 `Tauri`까지 함께 보도록 넓혔다.
- 이 공통 판별기를 아래 서비스/부트스트랩 경계에 적용했다.
  - `src/main.tsx`
  - `src/capture-adapter/services/capture-runtime.ts`
  - `src/session-domain/services/start-session.ts`
  - `src/session-domain/services/active-preset.ts`
  - `src/session-domain/services/runtime-capability-gateway.ts`
  - `src/operator-console/services/operator-diagnostics-service.ts`
  - `src/preset-catalog/services/preset-catalog-service.ts`
  - `src/preset-authoring/services/preset-authoring-service.ts`
  - `src/branch-config/services/branch-rollout-service.ts`
  - `src/booth-shell/components/preset-preview-src.ts`

해석:

- helper status가 이미 `ready`인데 화면만 `Preparing`이면, 다음 우선 확인 대상은 camera/helper가 아니라 **프런트 runtime detection**이다.
- 특히 Tauri dev 앱에서 browser fallback처럼 동작하면, 실제 session/helper truth가 파일에 정상으로 들어와도 고객 화면은 로컬 `Preparing`에서 벗어나지 못할 수 있다.

### 8. 2026-03-28 추가 확인: capture runtime gateway가 앱 시작 순간의 browser 판정을 고정할 수 있었다

- 추가 점검 결과, booth의 capture runtime service는 `SessionProvider` 첫 렌더 시 `createCaptureRuntimeService()`를 한 번만 호출하고 있었다.
- 기존 `createDefaultCaptureRuntimeGateway()`는 그 시점의 `isTauriRuntime()` 결과를 그대로 고정했다.
- 따라서 앱 시작 초기에 Tauri bridge가 아직 붙기 전이라 `browser`로 판정되면, 이후 실제 Tauri runtime이 살아나도 capture readiness 경로는 계속 browser fallback gateway를 사용하게 된다.
- 이 경우 helper status 파일이 이미 `ready`여도 booth 화면은 계속 브라우저용 `Preparing` copy에 머무를 수 있다.

이번 회차 코드 보정:

- capture runtime default gateway를 "생성 시점 1회 판정"이 아니라, 각 호출 시점마다 현재 runtime을 다시 결정하는 방식으로 바꿨다.
- readiness polling은 이제 초기 subscribe가 browser fallback으로 시작되더라도, 이후 poll 시점에 Tauri bridge가 살아 있으면 host readiness로 자동 전환될 수 있다.
- 재현 시 바로 확인할 수 있도록 프런트 콘솔에 `[boothy][capture-runtime]` 로그를 추가했다.
  - `gateway-get-readiness`
  - `gateway-request-capture`
  - `gateway-delete-capture`
  - `gateway-subscribe-readiness`

추가 진단 원칙:

- `helper status = ready`인데 booth가 계속 `Preparing`이면, 이제는 helper status 파일뿐 아니라 브라우저 개발자 도구 콘솔에서 아래 두 로그를 같이 본다.
  - `[boothy][capture-runtime] ... mode: 'browser' | 'tauri'`
  - `[boothy][capture] request-readiness / apply-readiness-response / readiness-error`
- 특히 `gateway-get-readiness`가 반복해서 `mode: 'browser'`를 찍으면, 문제는 helper가 아니라 프런트 runtime bridge 인식 경계다.

### 9. 2026-03-28 추가 확인: host는 이미 `Ready`를 반복 계산하므로 남은 병목은 프런트 적용 경계다

- 사용자가 제공한 최신 dev 실행 로그에서 같은 session `session_000000000018a0e212968a9fc8`에 대해 host readiness는 아래처럼 바뀌는 것이 확인됐다.
  - `03:00:23 ~ 03:00:28`: `Preparing / camera-preparing / live_truth=...disconnected|connecting`
  - `03:00:29 이후 반복`: `Ready / ready / can_capture=true / live_truth=fresh:matched:ready:healthy`
- 따라서 이 시점의 본문제는 helper discovery나 host normalized readiness가 아니라, **프런트가 host의 Ready 응답을 실제 화면 상태로 적용하는 마지막 경계**다.

이번 회차 코드 보정:

- 프런트가 남기는 capture debug 로그를 Tauri 터미널에서도 보이도록 `log_capture_client_state` 명령을 추가했다.
- 아래 프런트 로그가 이제 Rust 실행 터미널에도 함께 찍힌다.
  - `[boothy][capture-runtime] gateway-get-readiness`
  - `[boothy][capture-runtime] gateway-readiness-success`
  - `[boothy][capture-runtime] gateway-readiness-error`
  - `[boothy][capture] request-readiness`
  - `[boothy][capture] apply-readiness-response`
  - `[boothy][capture] apply-readiness-state`
  - `[boothy][capture] apply-subscribed-readiness`
  - `[boothy][capture] readiness-error`

다음 재현 때 판단 기준:

- host에 `capture_readiness ... customer_state=Ready`가 찍히고,
- 이어서 `capture_client_state ... label=apply-readiness-state ... customer_state=Ready`까지 찍히면,
- 문제는 더 이상 readiness 상태 저장이 아니라 booth 화면 렌더 경계다.
- 반대로 host는 `Ready`인데 client log가 `gateway-readiness-error` 또는 `readiness-error`를 찍으면, 프런트 parse/invoke 경계를 우선 본다.

### 10. 2026-03-28 EDSDK 준비물 재확인

- `C:\Code\cannon_sdk`에는 동일 크기의 Canon SDK zip 2개와 각 압축 해제본이 존재한다.
- vendor 경로 [canon-edsdk](/C:/Code/Project/Boothy/sidecar/canon-helper/vendor/canon-edsdk)에는 Windows 런타임과 샘플이 실제로 들어와 있다.
  - `Windows/EDSDK/Dll/EDSDK.dll`
  - `Windows/EDSDK/Dll/EdsImage.dll`
  - `Windows/Sample/CSharp/CameraControl/...`
  - `Windows/Sample/VC/CameraControl/...`
  - `Windows/Sample/VC/MultiCamCui/...`
- 즉 현재 `Preparing` 현상은 "SDK 파일이 없어서 helper가 거짓 준비 상태가 된다" 쪽과는 분리해서 봐야 한다.

### 11. 2026-03-31 추가 확인: 초점 실패성 셔터 오류가 `Phone Required`를 남긴 채 회복되는 사례

- 최신 재현 세션 `session_000000000018a1f048a428ef78`에서 첫 촬영은 성공했고, 두 번째 촬영만 실패했다.
- 해당 세션의 `camera-helper-events.jsonl`에는 아래 순서가 남아 있었다.
  - 첫 요청: `capture-accepted -> file-arrived`
  - 두 번째 요청: `capture-accepted -> recovery-status(detailCode=capture-trigger-failed) -> helper-error(detailCode=capture-trigger-failed, message=0x00008d01)`
- Canon SDK vendor header 기준 `0x00008d01`은 `EDS_ERR_TAKE_PICTURE_AF_NG`로, 초점을 잡지 못해 셔터를 시작하지 못한 경우다.
- 실패 직후 helper 최종 status는 다시 `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`로 회복했다.
- 그런데 host manifest는 이미 `phone-required`로 저장돼 있었고, 이후 고객 화면도 계속 `Phone Required`에 머물렀다.

해석:

- 이번 남은 본문제는 `capture-start-timeout`이 아니라, **초점 실패성 `capture-trigger-failed`를 host가 치명 오류로 저장한 경로**였다.
- 즉 helper live truth는 회복됐지만, 세션 lifecycle이 먼저 `phone-required`로 잠겨 화면이 풀리지 않았다.

이번 회차 수정 원칙:

- helper는 `AF_NG(0x00008d01)`를 `capture-focus-not-locked` 재시도 가능 오류로 내린다.
- host는 `capture-focus-not-locked`, `camera-busy`, 그리고 legacy `capture-trigger-failed + 0x00008d01` 흔적을 `Phone Required`가 아닌 재시도 상태로 해석한다.
- 이미 이전 버전이 남긴 `phone-required` manifest라도, 같은 세션 helper truth가 다시 `fresh/matched/ready/healthy`이면 촬영 가능 stage로 복구한다.

### 12. 2026-03-31 추가 확인: 성공 촬영이 `현재 세션 사진`에 반영되지 않는 저장 경계

- 최신 세션 `session_000000000018a1f147327c30ec`에서는 초점 실패 두 번 뒤 실제 성공 촬영이 두 번 있었다.
- 같은 세션의 helper evidence는 정상이다.
  - `capture-accepted -> file-arrived`가 두 번 이어졌다.
  - `captures/originals/` 아래 실제 `.CR2` 파일도 두 개 생성됐다.
- 그런데 같은 시점 `session.json`은 끝까지 `captures: []`, `lifecycle.stage: preset-selected`로 남았다.
- customer 화면도 `Ready`를 계속 유지해, 사용자는 실제 촬영이 됐는데 현재 세션 사진에는 아무 결과가 안 올라오는 상태를 보게 됐다.

해석:

- 이번 남은 문제는 helper capture나 초점 상태가 아니라, **host가 성공 촬영을 session manifest에 반영하는 저장 경계**다.
- 코드상 가장 의심되는 경로는 Windows에서 `session.json -> .bak/.tmp` 원자 교체를 하는 동안 readiness polling/read가 계속 붙어 manifest write가 깨지는 경우다.
- 이 경우 helper와 RAW 파일은 정상이어도, 세션 결과와 화면 반영은 비게 된다.

이번 회차 수정 원칙:

- `write_session_manifest(...)`는 Windows sharing/rename race에 대해 짧은 retry budget을 둔다.
- `file-arrived`까지 성공한 촬영은 retry budget 안에서 반드시 session manifest에 연결되게 한다.
- 그래도 persist가 끝까지 실패하면, 고객 화면이 조용히 `Ready`로 남지 않게 명시적 보호 상태로 올린다.
- capture command 로그에 `capture_request_saved`, `capture_persist_failed`, `capture_preview_ready` 같은 저장 경계 로그를 남긴다.

### 13. 2026-03-31 추가 확인: 앱이 혼자 껐다 켜지는 것처럼 보인 현상은 `tauri dev` watcher 재실행이었다

- 사용자 제보 기준으로는 "빌드 후 가만히 있었는데 앱이 스스로 껐다 켜졌다"처럼 보였다.
- 실제 dev 로그를 보면 런타임 crash loop보다 아래 순서에 가까웠다.
  - Rust compile error가 발생했다.
  - 그 상태에서 `tauri dev`가 `src-tauri/tests/...`, `src-tauri/src/...` 파일 변경을 계속 감지했다.
  - watcher가 매번 `cargo run`을 다시 호출하면서 `target/debug/boothy.exe`를 반복 재실행했다.
- 즉 이번 현상은 고객용 앱 재시작 로직이 아니라, **개발 watcher가 repo 변경을 따라가며 앱을 다시 띄운 것**이다.

이번 회차 보완:

- Rust 코드는 현재 stable toolchain에서 바로 컴파일되도록 2024 전용 `let chain` 문법을 제거했다.
- 개발 검증용으로 `pnpm run dev:desktop:stable` 스크립트를 추가했다.
- 이 스크립트는 `pnpm tauri dev --no-watch`를 사용하므로, 테스트 파일이나 히스토리 문서 변경 때문에 앱이 재실행되지 않는다.

운영 메모:

- 런타임 behavior만 검증할 때는 `pnpm run dev:desktop:stable`를 우선 사용한다.
- `pnpm tauri dev`는 코드 편집과 함께 볼 때만 쓰고, 테스트 실행/문서 수정과 병행하면 "앱이 스스로 재시작한다"처럼 보일 수 있다.

### 11. 2026-03-28 추가 확인: 프런트 계약 스키마가 helper `observedAt` offset 시간을 거부하고 있었다

- 사용자가 제공한 최신 로그에서, host는 이미 같은 session에 대해 `customer_state=Ready reason_code=ready can_capture=true`를 반복 계산하고 있었다.
- 하지만 같은 시각 프런트 쪽 `capture_client_state`는 아래 에러를 반복 기록했다.
  - `label=gateway-readiness-error`
  - `path=["liveCaptureTruth","observedAt"]`
  - `message="Invalid ISO datetime"`
- 원인은 프런트 계약 [capture-readiness.ts](/C:/Code/Project/Boothy/src/shared-contracts/schemas/capture-readiness.ts)가 `observedAt`를 `z.string().datetime()`으로 검증하면서 `...Z` 형식만 허용하고, helper가 실제로 보내는 `2026-03-28T03:10:57.1234567+00:00` 같은 offset 형식을 거부하던 점이다.

중요한 분리:

- Rust host의 freshness parser는 이미 `Z`와 `+00:00`을 모두 읽도록 고쳐져 있었다.
- 하지만 프런트 계약 스키마가 같은 offset 형식을 다시 거부하면서, host의 `Ready` 응답이 프런트에서는 parse 실패로 떨어지고 customer-safe `Preparing` fallback으로 내려갔다.
- 즉 이 단계의 증상은 helper truth나 host normalized readiness 문제가 아니라, **프런트 contract parse drift**였다.

이번 회차 코드 보정:

- `liveCaptureTruth.observedAt`를 `z.string().datetime({ offset: true })`로 변경했다.
- `+00:00` 형식의 helper timestamp를 그대로 통과시키는 계약 테스트를 추가했다.

앞으로 같은 증상이 보이면 먼저 볼 로그:

- `capture_readiness ... customer_state=Ready`
- `capture_client_state label=gateway-readiness-success`

만약 host는 `Ready`인데 프런트가 계속 `gateway-readiness-error`를 찍으면, helper보다 먼저 프런트 shared-contracts parse drift를 의심한다.

### 12. 2026-03-28 전원 `off -> on -> ready` 성공 경로와 실제 원인 정리

- 사용자가 확인한 성공 시나리오는 아래 순서였다.
  1. 카메라 전원을 `off`한 상태로 앱 실행
  2. booth/session 시작 후 helper가 `disconnected` 또는 `connecting`으로 상태를 기록
  3. 같은 실행을 유지한 채 카메라 전원을 `on`
  4. host 로그가 `Preparing -> Ready`로 전환
- 이 성공 자체는 "앱을 다시 켜서 우연히 잡혔다"가 아니라, **helper의 session-bound reconnect loop가 살아 있었기 때문**이다.
- helper는 루프마다 `EnsureConnectedAsync(...)`를 호출하고 있었고, 세션이 아직 열리지 않았을 때는 `EdsGetCameraList -> EdsOpenSession` 재시도를 계속 수행한다.
- 그래서 카메라가 늦게 켜져도, helper가 같은 세션 아래에서 다시 카메라를 발견하면 host truth는 정상적으로 `ready`로 올라올 수 있다.

다만 같은 성공 시나리오가 화면에 바로 반영되지 않았던 실제 원인은 별개였다.

- host 쪽에서는 이미 `Ready / ready / can_capture=true`가 계산되고 있었지만,
- 프런트 shared-contracts가 helper의 `liveCaptureTruth.observedAt` 값을 `...+00:00` 형식으로 받지 못해 parse error를 냈고,
- 그 결과 customer 화면은 host `Ready`를 버리고 customer-safe `Preparing` fallback을 계속 적용했다.
- 즉 이번 회차에서 확인된 "ready 성공"의 진짜 의미는,
  - helper reconnect는 이미 동작했다.
  - 화면 반영이 막혔던 원인은 helper가 아니라 **프런트 계약 parse drift**였다.

실전 판단 포인트:

- `capture_readiness ... customer_state=Ready reason_code=ready can_capture=true`
  가 찍히면 helper reconnect는 성공한 것이다.
- 그 뒤에도 화면이 `Preparing`이면, helper보다 먼저 프런트 `gateway-readiness-error`를 본다.

### 13. 2026-03-28 추가 확인: `ready` 이후 전원 `off`가 반영되지 않던 원인

- 사용자가 같은 세션에서 `off -> on -> ready`까지 올린 뒤 다시 전원을 `off`했을 때, 화면 상태가 바뀌지 않는 문제가 남아 있었다.
- helper 코드를 확인한 결과, 세션이 열린 뒤에는 keep-alive만 간헐적으로 보내고 있었고, **콘솔 앱에서 Canon SDK 이벤트를 정기적으로 펌프하는 `EdsGetEvent()` 호출이 없었다.**
- Canon SDK header와 MultiCamCui 샘플에는 아래 원칙이 명시되어 있다.
  - 콘솔 애플리케이션에서는 `EdsGetEvent()`를 정기적으로 호출해 카메라 이벤트를 획득해야 한다.
- 우리 helper는 `EdsSetCameraStateEventHandler(... StateEvent_All ...)`는 등록했지만, 이벤트 펌프가 없어서 `StateEvent_Shutdown` 같은 전원 차단/연결 해제 이벤트를 놓칠 수 있었다.
- 그래서 전원을 꺼도 helper snapshot이 계속 이전 `ready`를 붙잡고, host도 바뀐 진실을 보지 못하는 경로가 생겼다.

이번 회차 코드 보정:

- helper 루프에 `EdsGetEvent()` 기반 SDK event pump를 추가했다.
- event pump가 `COMM_DISCONNECTED`, `DEVICE_NOT_FOUND`, `DEVICE_INVALID`, `SESSION_NOT_OPEN` 같은 연결 상실 오류를 반환하면 helper가 즉시 recovery/disconnect 흐름으로 내리도록 보강했다.
- 기존 `StateEvent_Shutdown` callback도 공통 connection-lost 경로를 타도록 정리했다.
- 추가로 keep-alive health probe 간격을 더 짧게 조정해, 카메라가 shutdown 이벤트를 보내지 않는 경우에도 전원 `off`가 더 빨리 반영되도록 보강했다.

수정 후 기대 동작:

1. 카메라 `off` 상태 앱 실행
2. 카메라 `on`
3. helper reconnect로 `Ready`
4. 카메라 `off`
5. helper event pump 또는 keep-alive probe가 세션 상실을 감지
6. status가 `recovering/disconnected` 계열로 내려가고 booth도 다시 준비/전원 확인 상태로 전환

남은 검증:

- 이 경로는 실장비에서 다시 확인해야 한다.
- 현재 수정은 SDK 샘플/헤더 기준으로는 맞지만, 실제 EOS 700D가 전원 차단 시 `StateEvent_Shutdown`과 `EdsGetEvent()` 조합에서 어떤 오류 코드를 내는지는 하드웨어에서 최종 확인이 필요하다.

### 14. 2026-03-29 추가 확인: `helper-binary-missing` 복구와 로컬 SDK fallback으로 실제 통신 경계 복원

- 사용자가 같은 증상을 다시 재현했을 때, 최신 session `session_000000000018a12309c488c584`의 `camera-helper-status.json`은 아래처럼 기록돼 있었다.
  - `cameraState: "error"`
  - `helperState: "error"`
  - `detailCode: "helper-binary-missing"`
- 즉 이 시점의 본문제는 camera discovery가 아니라, **helper 자체가 런타임에서 전혀 뜨지 못한 것**이었다.

실제 확인 사실:

- workspace 안에는 `canon-helper.exe`가 없었다.
- 하지만 로컬 머신에는 `C:\Code\cannon_sdk\1745202892851_pAVdAAA7pU` 아래 Canon SDK 원본이 남아 있었다.
  - `Windows\Sample\CSharp\CameraControl\CameraControl\EDSDK.cs`
  - `Windows\EDSDK_64\Dll\EDSDK.dll`
- 기존 helper project는 repo 내부 `sidecar/canon-helper/vendor/canon-edsdk`만 고정 참조하고 있어, fresh workspace에서는 helper build와 runtime attach가 함께 막힐 수 있었다.

이번 회차 코드 보정:

- helper supervisor가 publish/debug exe를 찾지 못해도, dev 환경에서는 helper source project를 `dotnet run`으로 바로 띄울 수 있게 fallback을 추가했다.
- helper project는 `BOOTHY_CANON_SDK_ROOT` 또는 local vendor를 Canon SDK root로 받아 build/runtime payload를 해석할 수 있게 바꿨다.
- supervisor는 `vendor/README.md`에 기록된 selected SDK path와 `C:\Code\cannon_sdk\*` fallback도 함께 탐색하도록 보강했다.
- 같은 회차에서 helper debug build를 실제로 다시 생성했다.
  - `sidecar/canon-helper/src/CanonHelper/bin/Debug/net8.0/canon-helper.exe`

이번 회차 검증:

- `canon-helper.exe --version` 성공
- `canon-helper.exe --self-check --sdk-root C:\Code\cannon_sdk\1745202892851_pAVdAAA7pU` 성공
- self-check 결과는 더 이상 `helper-binary-missing`이 아니라 `camera-not-found`까지 진입했다.
  - 즉 실패 지점이 "helper 없음"에서 "실제 카메라 발견 단계"로 이동한 것이다.
- 이후 사용자가 앱을 다시 실행해 실제 booth flow를 확인했고, **카메라 통신이 다시 성공했다고 확인했다.**

운영 판단 기준:

- 최신 session status가 `helper-binary-missing`이면 camera on/off 반응 속도보다 먼저 helper artifact 존재 여부를 본다.
- 이 경우 `camera-not-found`와 같은 discovery 단계로 되돌리는 것이 우선이며, 그 뒤에야 reconnect/off-detection 품질을 논할 수 있다.
- fresh workspace나 새 머신에서 dev 실행을 재현할 때는 helper exe 존재만 보지 말고, local Canon SDK root와 helper source fallback까지 같이 점검하는 편이 안전하다.

### 15. 2026-03-29 추가 확인: 첫 촬영 성공 뒤 두 번째 촬영이 `Phone Required`로 과승격되던 원인

- 사용자가 세션 이름 입력 후 카메라 상태를 확인하고 `촬영`을 눌렀을 때, 첫 촬영은 정상 저장되었고 최근 썸네일까지 표시됐다.
- 하지만 이어서 같은 세션에서 `촬영`을 한 번 더 누르면 `Phone Required`가 뜨는 회귀가 재현됐다.
- 이 패턴은 "실제 카메라 치명 장애"라기보다, **직전 촬영 직후의 임시 재확인/일시 busy 상태를 프런트가 `Phone Required`로 잘못 번역한 것**으로 보는 편이 더 맞았다.

실제 확인 사실:

- Rust host 쪽 기본 계약은 `can_capture=false`일 때 같은 session의 readiness를 함께 반환하도록 되어 있고, 이 readiness가 `preview-waiting` 또는 `camera-preparing`이면 고객 화면도 직원 호출이 아니라 대기 상태여야 한다.
- 그런데 프런트 `capture runtime` 정규화는 `request_capture` 실패에서 readiness가 비어 있거나 파싱되지 않으면, 이를 보수적으로 `Phone Required`로 승격하고 있었다.
- 그 결과 "실패 원인을 아직 단정할 수 없는 임시 재확인 상태"와 "실제 보호 전환이 확정된 상태"가 고객 화면에서 같은 `Phone Required`로 합쳐졌다.

이번 회차 코드 보정:

- `request_capture` 실패 정규화에서 readiness가 없는 `capture-not-ready`와 일반 host 실패를 더 이상 자동 `Phone Required`로 바꾸지 않도록 조정했다.
- 이런 경우 고객 화면은 `Preparing / 잠시 기다리기` 계열의 일시 상태로 내리고, `Phone Required`는 host가 명시적으로 그 보호 상태를 보낸 경우에만 유지되게 했다.
- 같은 회차에서 최근 same-session 썸네일이 이미 보이는 상태에서 follow-up capture가 임시 실패해도, 기존 썸네일이 사라지지 않고 세션이 직원 호출 상태로 잠기지 않는 회귀 테스트를 추가했다.

관련 파일:

- [capture-runtime.ts](/C:/Code/Project/Boothy/src/capture-adapter/services/capture-runtime.ts)
- [capture-runtime.test.ts](/C:/Code/Project/Boothy/src/capture-adapter/services/capture-runtime.test.ts)
- [session-provider.test.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.test.tsx)

이번 회차 검증:

- `pnpm vitest run src/capture-adapter/services/capture-runtime.test.ts src/session-domain/state/session-provider.test.tsx`
- `pnpm exec eslint src/capture-adapter/services/capture-runtime.ts src/capture-adapter/services/capture-runtime.test.ts src/session-domain/state/session-provider.test.tsx`
- 검증 결과, 관련 targeted test `54 passed`와 lint 통과를 확인했다.

운영 판단 기준:

- 첫 촬영 성공 뒤 최근 썸네일이 뜬 상태에서만 두 번째 촬영이 `Phone Required`로 떨어지면, camera on/off나 helper binary보다 먼저 **프런트 request failure 정규화**를 의심한다.
- 이 경우 `Phone Required`가 실제 host readiness에 의해 명시된 것인지, 아니면 readiness 없는 request failure를 프런트가 과승격한 것인지 먼저 구분해야 한다.
- 동일 패턴 재발 시에는 `capture-runtime` 로그와 `request_capture` 오류 payload에 readiness가 실려 있는지부터 확인하는 편이 가장 빠르다.

### 16. 2026-03-29 작업 시작 기록: duplicate-shutter 완화 뒤 capture가 무한 로딩 후 `Phone Required`로 떨어지는 회귀

- 이번 회차에서는 "카메라가 자기 혼자 계속 셔터를 찍는다"는 증상을 막기 위해 helper request log 재소비 방어를 넣은 직후,
  반대로 정상 `사진찍기` 요청이 끝까지 닫히지 않고 버튼이 오래 로딩된 뒤 `Phone Required`로 떨어지는 회귀가 보고됐다.
- 사용자 체감 증상은 아래와 같다.
  - `사진찍기` 버튼을 누르면 즉시 결과가 오지 않고 로딩이 길게 유지된다.
  - 이후 customer 화면이 `Phone Required`로 내려간다.
- 이 단계의 1차 의심 지점은 camera discovery나 프런트 문구보다, **helper가 새 request를 실제로 소비하고 `capture-accepted` / `file-arrived` / `helper-error`를 남기는 경계가 막힌 것인지**다.
- 특히 직전 변경에서 아래 경계가 바뀌었으므로 우선순위를 높게 본다.
  - `sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/SessionPaths.cs`
- 이번 회차에서는 먼저
  1. helper request 소비 로직이 새 request를 놓치거나 중복 필터로 잘못 버리지 않는지,
  2. request를 `processed`로 기록하는 시점이 너무 이르지 않은지,
  3. host가 기다리는 event contract와 helper event emission 사이에 drift가 생기지 않았는지
  순서로 다시 확인한다.

실제 원인:

- 이번 회귀의 직접 원인은, 새 helper가 `camera-helper-processed-request-ids.txt`가 아직 없던 **기존 세션**에 붙을 때였다.
- 이 경우 helper는 request log를 처음부터 읽으면서 예전에 이미 성공했던 촬영 요청도 "아직 처리 안 된 request"로 오인할 수 있었다.
- 그러면 helper는 방금 누른 새 `requestId`보다 먼저 **오래된 requestId**에 반응해 셔터를 실행하고,
  host는 새 `requestId`의 `capture-accepted` / `file-arrived`를 기다리다 timeout으로 `Phone Required`에 떨어질 수 있었다.
- 즉 증상은 "캡처가 안 된다"였지만, 실제로는 **helper replay 대상이 잘못돼 host correlation이 어긋난 것**에 가까웠다.

이번 회차 코드 보정:

- helper startup 시 `processed-request` 파일만 보지 않고, 기존 `camera-helper-events.jsonl`의
  `capture-accepted` / `file-arrived` requestId도 함께 읽어 이미 처리된 요청 집합을 backfill 하도록 보강했다.
- 그래서 업그레이드 전 세션처럼 processed file이 비어 있어도, 이미 성공 이력이 있는 request는 다시 실행하지 않는다.
- request log는 계속 새로 append된 완전한 line만 incremental read 하고, 위 backfill 집합과 합쳐 stale request replay를 막는다.
- helper 전용 regression test를 추가해
  - processed file 기반 재시작 중복 방지
  - event log 기반 기존 성공 request backfill
  - partial trailing request line 보류
  를 고정했다.

현재 검증 상태:

- 코드 레벨에서는 helper regression test와 helper build를 통과했다.
  - `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`
  - `dotnet build sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`
- 아직 부스 앱과 실카메라로 다시 눌러 본 최종 결과는 기록하지 않았다.
- 실제 customer flow 결과는 사용자 하드웨어 테스트 후 이 문서에 후속으로 남기는 것이 맞다.

## 오진하기 쉬운 포인트

### "status 파일에 ready가 있으니 host도 ready일 것이다"

틀릴 수 있다.

- BOM 때문에 parser가 실패할 수 있다.
- `observedAt` offset 형식 때문에 freshness 계산이 실패할 수 있다.
- session match는 맞아도 freshness가 stale이면 host는 ready를 주지 않는다.

### "카메라가 켜져 있는데 Phone Required면 helper가 죽었다"

반드시 그렇지 않다.

- helper는 살아 있고 status도 쓰고 있을 수 있다.
- host parser/freshness 계산이 틀려 blocked path로 내려가는 경우가 있다.
- 프런트가 이전 `phone-required`를 붙잡고 있을 수도 있다.

### 17. 2026-03-29 추가 확인: 최신 session 진단이 비고 helper가 이전 session에 묶여 있으면 preset 선택 직후 재부착 경계를 먼저 본다

- 이번 회차에서는 사용자가 `카메라 연결상태 확인`을 눌렀을 때 계속 `Preparing`에 머무르는 증상이 다시 보고됐다.
- 실제 확인 결과는 아래와 같았다.
  - 최신 session manifest는 새 session으로 정상 생성되고 active preset도 기록돼 있었다.
  - 하지만 최신 session의 `diagnostics` 폴더에는 `camera-helper-status.json`이 없었다.
  - 반대로 실행 중인 `canon-helper.exe` command line은 직전 session id에 계속 바인딩돼 있었다.
- 즉 이 케이스의 본문제는 helper truth 내용이 아니라, **latest session으로 helper를 다시 붙이는 경계가 비어 있거나 너무 늦은 것**에 가까웠다.

이번 회차 코드 보정:

- Rust host의 [preset_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/preset_commands.rs)에서 `select_active_preset` 성공 직후에도 `try_ensure_helper_running(...)`을 호출하도록 보강했다.
- 프런트 [active-preset.ts](/C:/Code/Project/Boothy/src/session-domain/services/active-preset.ts)는 runtime 판정을 서비스 생성 시점에 고정하지 않고, 실제 `selectActivePreset(...)` 호출 시점마다 다시 판단하도록 바꿨다.
- 그래서 Tauri bridge가 앱 초기에는 아직 붙지 않았더라도, 이후 실제 선택 동작 시점에는 host command와 helper 재부착으로 자연스럽게 넘어갈 수 있다.

운영 판단 기준:

1. 최신 session의 `diagnostics/camera-helper-status.json`이 비어 있다.
2. 동시에 `canon-helper.exe` process command line이 이전 session id를 가리킨다.
3. 그런데 최신 session manifest에는 `activePreset`이 이미 기록돼 있다.

이 세 조건이 함께 보이면, camera discovery나 freshness parser보다 먼저 **preset 선택 이후 helper session rebind 경계**를 점검하는 편이 빠르다.

### 18. 2026-03-29 추가 확인: helper는 이미 `ready`인데 capture 진입 첫 확인이 fallback이면 화면이 계속 `Preparing`에 남을 수 있다

- 이번 회차에서 최신 session에 helper를 수동으로 다시 붙여 보니, 실제 status는 아래처럼 바로 정상으로 올라왔다.
  - `cameraState: "ready"`
  - `helperState: "healthy"`
  - `detailCode: "camera-ready"`
- 즉 카메라/helper truth는 이미 정상이었는데도 booth 화면이 계속 `Preparing`에 머무를 수 있었다.

실제 원인:

- [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx)는 capture 화면 진입 시 `getCaptureReadiness(...)`를 한 번 호출하고 readiness event subscribe를 한 번 등록한다.
- 이 첫 호출/구독 순간에 runtime bridge가 아직 browser fallback 경로를 타면, 초기 `Preparing` 응답과 no-op subscribe가 그대로 남을 수 있다.
- 이후 helper가 같은 session에서 `ready`가 되어도, 추가 host 재확인이 없으면 화면은 자동으로 회복되지 않는다.

이번 회차 코드 보정:

- capture flow에 들어와 있는 동안에는 주기적으로 readiness를 다시 읽도록 보강했다.
- 그래서 첫 진입 순간 fallback이 있었더라도, 잠시 뒤 host/Tauri 경로가 살아나면 같은 session의 실제 `Ready` 상태로 스스로 회복할 수 있다.

운영 판단 기준:

1. 최신 session `camera-helper-status.json`이 이미 `ready/healthy`다.
2. 그런데 booth 화면은 계속 `Preparing`이다.
3. 최신 session 진단과 helper process session id는 서로 맞다.

이 세 조건이 함께 보이면 helper나 host normalized readiness보다 먼저 **capture 진입 직후 1회성 readiness 조회와 이후 재확인 경계**를 점검하는 편이 빠르다.
- 최신 단계에서는 helper가 살아 있지만 `camera-not-found`를 지속 보고하는 경우도 실제로 확인됐다.

### 19. 2026-03-29 추가 확인: 이전 세션 helper orphan이 남아 새 세션을 `session-open-failed -> Phone Required`로 밀어 올리던 문제

- 이번 회차에서는 고객 화면의 `Phone Required`가 프런트 과승격이 아니라,
  host 로그의 `live_truth=fresh:matched:error:error`와 실제 helper status `detailCode=session-open-failed`로 바로 확인됐다.
- 같은 시각 실행 중인 `canon-helper.exe`를 확인해 보니,
  현재 실패 세션이 아니라 **이전 세션에 바인딩된 helper가 `ready/healthy` 상태로 계속 살아 있었다.**
- 이 상태에서는 새 session-bound helper가 카메라를 찾더라도 `EdsOpenSession(...)`에서 충돌할 수 있고,
  그 결과 최신 session status가 `cameraState=error`, `helperState=error`, `detailCode=session-open-failed`로 기록되며
  booth는 `Phone Required`로 내려갈 수 있었다.

실제 원인:

- helper supervisor는 같은 앱 프로세스 안에서는 child helper를 추적하고 종료했지만,
  **이전 앱 인스턴스가 남긴 orphan helper**까지는 정리하지 못했다.
- 그래서 앱을 다시 켠 뒤 새 session을 시작하면, 이전 helper가 계속 카메라 세션을 잡고 있어
  새 helper 연결이 막히는 레이스가 생길 수 있었다.

이번 회차 코드 보정:

- Rust helper supervisor가 새 helper를 띄우기 전에
  **같은 runtime root를 바라보는 stale helper process를 먼저 정리**하도록 보강했다.
- 새 helper 실행 시 `--parent-pid`를 함께 넘기고,
  helper는 부모 프로세스가 사라지면 스스로 종료하도록 보강했다.
- 이 조합으로
  1. 이미 남아 있는 orphan helper를 다음 실행에서 정리하고,
  2. 이후 새 orphan helper가 다시 쌓이는 것도 줄이게 했다.

운영 판단 기준:

1. 최신 session status가 `session-open-failed`다.
2. 동시에 별도 `canon-helper.exe`가 이전 session id로 계속 떠 있다.
3. 그 이전 helper status는 `ready/healthy`다.

이 세 조건이 함께 보이면 카메라 미발견보다 먼저 **stale helper orphan 충돌**을 의심하는 편이 빠르다.

### "현재 증상은 operator diagnostics 쪽 문제다"

booth `Phone Required`와 operator diagnostics 오류는 같은 root cause에서 같이 나올 수 있지만,
항상 같은 원인은 아니다.

- booth는 capture readiness 경로를 본다.
- operator는 recovery summary / audit load path를 본다.
- 둘 다 session/helper truth를 공유하므로 session 파일과 helper status를 먼저 확인해야 한다.

## 다음 에이전트용 진단 순서

아래 순서를 지키면 중복 추정을 많이 줄일 수 있다.

### 1. 실행 중 프로세스 확인

PowerShell:

```powershell
Get-Process | Where-Object {
  $_.ProcessName -like 'boothy*' -or $_.ProcessName -like '*canon-helper*'
} | Select-Object ProcessName, Id, StartTime, Path
```

확인 포인트:

- `boothy.exe`가 최신 시작 시각인지
- `canon-helper.exe`가 실제 publish 경로에서 떠 있는지

### 2. active runtime root 확인

현재 dev 경로 기준 런타임 root:

```text
%LOCALAPPDATA%\com.tauri.dev\booth-runtime
```

최신 session 확인:

```powershell
Get-ChildItem -Path $env:LOCALAPPDATA\com.tauri.dev\booth-runtime\sessions -Directory |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 5 FullName, Name, LastWriteTime
```

### 3. 최신 session의 manifest와 helper status를 같이 읽기

확인 파일:

- `session.json`
- `diagnostics/camera-helper-status.json`
- 필요 시 `diagnostics/camera-helper-events.jsonl`

반드시 같은 session을 보고 있는지 먼저 맞춘다.

### 4. host가 그 세션을 실제로 어떻게 해석하는지 직접 계산

정적 파일 내용만 보지 말고, Rust host readiness 계산 결과를 직접 확인한다.

판단 기준:

- `ready / fresh / matched`면 booth도 ready가 맞다.
- `stale / matched / ready / healthy`면 freshness 계산 문제를 먼저 의심한다.
- `missing / unknown / invalid-status`면 BOM이나 parser 문제를 먼저 의심한다.
- `fresh / matched / disconnected / healthy`면 parser 문제가 아니라 helper camera discovery 자체를 먼저 의심한다.

### 5. 프런트가 이전 blocked state를 붙잡는지 확인

파일:

- [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx)

특히 확인할 것:

- `mergePreservedPostEndReadiness(...)`
- `applyReadinessState(...)`

## 실제로 유효했던 확인 명령

### helper process command line 확인

```powershell
Get-CimInstance Win32_Process |
  Where-Object { $_.Name -eq 'canon-helper.exe' } |
  Select-Object ProcessId, CommandLine, ExecutablePath
```

### helper status raw bytes 확인

```powershell
Format-Hex -Path <camera-helper-status.json 경로> | Select-Object -First 40
```

이걸로 BOM(`EF BB BF`) 유무를 바로 확인할 수 있다.

### 최신 status가 실제로 갱신되는지 확인

```powershell
Start-Sleep -Seconds 3
Get-Content -Path <camera-helper-status.json 경로>
```

## 관련 계약 / 스토리 문서

문제 재발 시 아래 문서를 먼저 다시 본다.

- [Story 1.6](/C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md)
- [Story 1.7](/C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md)
- [Camera Helper Sidecar Protocol](/C:/Code/Project/Boothy/docs/contracts/camera-helper-sidecar-protocol.md)
- [Camera Helper EDSDK Profile](/C:/Code/Project/Boothy/docs/contracts/camera-helper-edsdk-profile.md)
- [Hardware Validation Checklist](/C:/Code/Project/Boothy/docs/runbooks/booth-hardware-validation-checklist.md)

## 검증할 때 자주 쓴 명령

Rust:

```powershell
cargo test --test session_manifest --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check
cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check
```

Frontend targeted tests:

```powershell
pnpm vitest run src/session-domain/state/session-provider.test.tsx src/capture-adapter/services/capture-runtime.test.ts
```

Helper:

```powershell
dotnet publish sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj -c Release -r win-x64 --self-contained true /p:PublishSingleFile=false
```

주의:

- `pnpm build`는 현재 operator diagnostics 타입 이슈로 전체 빌드가 막힐 수 있었다.
- 이 문제를 따로 고치지 않는 한, camera/helper 이슈 검증은 targeted vitest 쪽이 더 빠르고 안정적이다.

## 에이전트 작업 원칙

비슷한 문제가 다시 오면 아래 순서로 처리한다.

1. 실행 중 `boothy.exe`와 `canon-helper.exe`를 먼저 확인한다.
2. 최신 session 하나를 고르고 manifest/status/events를 같은 session 기준으로 본다.
3. 파일 내용만 믿지 말고 host readiness 계산 결과를 직접 확인한다.
4. `invalid-status`면 BOM부터, `stale`면 timestamp parser/freshness부터 본다.
5. readiness는 정상인데 화면만 보호 상태면 프런트 state preservation을 본다.
6. 수정 후에는 Rust test, targeted vitest, helper publish까지 최소 세 축을 확인한다.

이 순서를 건너뛰고 추정부터 시작하면 다시 같은 삽질을 반복할 가능성이 높다.

## 2026-04-01 00:03 +09:00 후속 정정: 이번 `Phone Required`는 helper readiness가 아니라 preview render bundle 호환성 문제였다

- 실제 최신 재현 session은 `%LOCALAPPDATA%\com.tauri.dev\booth-runtime`가 아니라
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1f4e305186810` 에 있었다.
- 그 session의 helper 진단은 정상으로 닫혔다.
  - `camera-helper-events.jsonl`: `capture-accepted` -> `file-arrived`
  - `camera-helper-status.json`: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- 그런데 같은 session의 `session.json`은 `renderStatus=renderFailed`, `lifecycle.stage=phone-required`였고,
  `diagnostics/timing-events.log`에는 `render-failed stage=preview reason=bundle-resolution-failed`가 남아 있었다.
- 즉 고객 화면의 `Phone Required`는 카메라/초점/요청 상관관계 실패가 아니라,
  촬영 성공 뒤 preview render가 현재 preset bundle을 runtime bundle로 해석하지 못해 올라간 보호 상태였다.

원인:

- 선택된 published preset `preset_test-look@2026.03.31`는 구형 bundle 형식이라
  `previewProfile` / `finalProfile` 필드가 없었다.
- 새 render loader가 이 두 필드를 사실상 필수처럼 다루면서 `bundle-resolution-failed`를 냈다.

수정:

- `preset_bundle.rs`에서 legacy published bundle에 render profile 필드가 없어도
  안전한 기본 preview/final profile을 합성해서 runtime bundle로 로드하도록 호환 경로를 추가했다.
- `ingest_pipeline.rs`에는
  - `capture_preview_render_failed`
  - `capture_final_render_failed`
  로그를 추가해 다음에는 helper 실패와 render 실패를 바로 구분할 수 있게 했다.

검증:

- `cargo test --test session_manifest --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- 두 테스트 모두 통과했다.

## 2026-04-01 01:18 +09:00 추가 정정: helper 정상인데도 `Phone Required`가 뜨면 기본 preset bundle 업그레이드 누락부터 본다

- 이번 재발의 실제 session은
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1f7a4fb1a8a7c` 였다.
- helper 증거는 여전히 정상으로 닫혔다.
  - `camera-helper-events.jsonl`: `capture-accepted`, `file-arrived`
  - `camera-helper-status.json`: `camera-ready`, `healthy`
- 반면 host 쪽은 `render-failed stage=preview reason=bundle-resolution-failed` 때문에
  `phone-required`로 보호 상태를 올렸다.

이번에 새로 확인한 핵심:

- 문제 bundle은 custom preset이 아니라 오래된 기본 preset seed였다.
- `ensure_default_preset_catalog_in_dir()`가
  "이미 bundle이 있으면 seed bootstrap을 건너뛴다"는 이유로
  구형 기본 bundle을 최신 runtime render bundle 형식으로 올려주지 못하고 있었다.
- 따라서 helper 쪽을 아무리 봐도 원인이 안 나오는 케이스였다.

운영 지침 업데이트:

1. helper가 `capture-accepted -> file-arrived`까지 정상이고도 `Phone Required`가 뜨면 helper 디버깅을 더 깊게 파지 않는다.
2. 같은 session의 `timing-events.log`에서 `render-failed` reason을 먼저 확인한다.
3. 기본 preset 사용 중이면 `preset-catalog/published/<preset>/<version>/bundle.json`의 runtime render metadata 누락 여부를 먼저 본다.
4. 누락 시 helper가 아니라 preset bootstrap/backfill 문제로 분류한다.

## 2026-04-01 01:28 +09:00 추가 정정: helper가 정상이고 bundle도 정상인데 `render-cli-missing`이면 darktable PATH 가정 회귀다

- 이번 추가 재발 세션은
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1f881d9d4cb74` 였다.
- helper 관점에서는 여전히 정상으로 닫혔다.
  - 실제 셔터 동작
  - `capture-accepted -> file-arrived`
  - helper truth `fresh:matched:ready:healthy`
- 그런데 host 로그는 반복해서 아래를 남겼다.
  - `reason_code=render-cli-missing`
  - `binary=darktable-cli error=program not found`

이번에 추가로 확인한 핵심:

- 부스 PC에는 실제 `C:\Program Files\darktable\bin\darktable-cli.exe`가 설치돼 있었다.
- 즉 darktable 자체가 없는 게 아니라, render worker가 PATH 상의 `darktable-cli`만 기대하고 표준 설치 경로를 직접 보지 않는 회귀였다.
- 이 경우 helper를 계속 파도 답이 안 나온다.

운영 지침 추가:

1. helper evidence가 정상이고 `render-failed` reason이 `render-cli-missing`이면 helper 조사에서 바로 빠진다.
2. 해당 PC에 darktable가 실제 설치돼 있는지 `Program Files\\darktable\\bin\\darktable-cli.exe`부터 확인한다.
3. 설치돼 있는데도 실패하면 render worker의 binary resolution regressions로 분류한다.
