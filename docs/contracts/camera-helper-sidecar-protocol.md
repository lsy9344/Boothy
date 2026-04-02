# Canon Camera Helper Sidecar Protocol

## 목적

이 문서는 Boothy가 Canon 계열 실카메라 상태를 Tauri/Rust host에서 어떻게 관찰하고 정규화하는지에 대한
최소 계약 기준을 고정한다.

이 계약의 목표는 두 가지다.

1. React가 Canon SDK truth를 직접 보지 않게 한다.
2. booth `Ready`와 operator `카메라 연결 상태`가 같은 host-owned truth에서 나오게 한다.

## 현재 채택 결정

- Boothy의 현재 채택안은 Windows 전용 Canon EDSDK 기반 `canon-helper.exe`다.
- 이 문서의 helper 계약은 위 실행 결정을 전제로, Tauri host와 helper exe 사이의 canonical sidecar protocol을 고정한다.

## 구현 프로파일 문서

- 현재 채택된 helper의 구체 구현 기준선은 `docs/contracts/camera-helper-edsdk-profile.md`에 둔다.
- 이 문서는 cross-boundary protocol을 고정하고, EDSDK profile 문서는 chosen helper의 런타임/패키징/진단 기준을 고정한다.
- generic protocol과 EDSDK profile이 충돌하면, 메시지 의미는 이 문서를 우선하고 구현 세부는 profile 문서를 우선한다.

## 소유 경계

- Canon/camera helper 실행 경계: `sidecar/canon-helper/`
- Tauri host 진입 경계: `src-tauri/src/capture/sidecar_client.rs`
- 최종 정규화 경계: `src-tauri/src/capture/normalized_state.rs`
- booth/operator 소비 경계:
  - booth: `src/shared-contracts/schemas/capture-readiness.ts`
  - operator: `src/shared-contracts/schemas/operator-diagnostics.ts`

React는 helper raw 메시지나 Canon SDK 상태를 직접 해석하지 않는다.

## 전송 규칙

- transport: bundled sidecar stdio
- framing: BOM 없는 UTF-8 JSON Lines, 메시지 1개당 1줄
- versioning: 모든 메시지는 `schemaVersion`을 가진다
- binary rule: RAW 이미지 bytes는 JSON payload로 보내지 않고 filesystem handoff로 전달한다
- correlation rule: host와 helper는 `sessionId`, `requestId`, `captureId`를 함께 사용한다
- ownership rule: `requestId`는 host가 생성해 `request-capture`에 싣고, `captureId`는 helper가 실제 RAW handoff 시점에 확정해 `file-arrived`로 돌려준다
- supported trigger rule: booth 제품의 supported capture success path는 host가 booth 앱의 `사진 찍기` 버튼에 응답해 보낸 `request-capture`다. 카메라 본체 셔터 직접 입력은 현재 canonical success path가 아니다.
- request log rule: helper는 소비한 `requestId`를 재시작 후에도 기억해 기존 request log의 과거 line을 재실행하면 안 되며, 새로 append된 완전한 line만 소비해야 한다

## EDSDK helper 런타임 요약

현재 채택된 helper profile 기준으로는 아래 분리가 유지돼야 한다.

- helper는 Canon SDK initialize/terminate, camera session open/close, capture trigger, RAW download, reconnect 감지를 소유한다.
- host는 session/preset correlation, freshness, booth/operator projection, capture success 최종 확정을 소유한다.
- `helper-ready`는 helper process boot 완료를 뜻할 뿐, camera `ready`를 뜻하지 않는다.
- `camera-status=ready`가 와도 host가 freshness와 session match를 닫기 전에는 booth `Ready`가 보장되지 않는다.
- helper는 한번에 하나의 in-flight capture만 허용하는 보수적 경계를 기본값으로 본다.

## host가 Canon 상태를 보는 경로

1. Tauri host가 helper 프로세스를 시작하거나 연결한다.
2. host는 helper로 runtime/session context를 보낸다.
3. helper는 Canon camera 상태를 읽고 `health/status` 메시지를 JSON line으로 보낸다.
4. host는 이 raw status를 `camera/helper truth`로 읽되, 바로 UI에 넘기지 않는다.
5. `normalized_state.rs`가 freshness, session match, degraded 여부를 확인한 뒤 booth/operator용 DTO로 정규화한다.
6. booth는 `Ready`/blocked guidance만 보고, operator는 `카메라 연결 상태`와 bounded diagnostics만 본다.

## helper raw 상태 모델 v1

helper는 최소 아래 의미 집합을 표현할 수 있어야 한다.

- `disconnected`: Canon body를 아직 찾지 못했거나 연결이 끊긴 상태
- `connecting`: helper가 장비 연결을 재시도하거나 초기화 중인 상태
- `connected-idle`: 장비는 보이지만 아직 촬영 가능 readiness가 닫히지 않은 상태
- `ready`: Canon capture boundary와 helper boundary가 모두 촬영 가능 상태
- `capturing`: 촬영 요청 처리 중이며 새 촬영 시작을 잠시 막아야 하는 상태
- `recovering`: helper restart, cable reseat, USB port change 이후 복구 중인 상태
- `degraded`: 이전에는 ready였지만 지금은 false-ready를 막기 위해 차단해야 하는 상태
- `error`: bounded recovery 밖의 실패 상태

이 raw 상태는 helper vocabulary다. booth UI는 이를 직접 노출하지 않는다.

## host 정규화 규칙

host는 helper raw 상태를 아래 원칙으로 정규화한다.

- `ready`는 helper raw 상태 하나만으로 성립하지 않는다.
- host는 최소 `camera state`, `helper state`, `freshness`, `active session`, `active preset`, `blocked post-end 여부`를 함께 본다.
- helper 상태가 오래되었거나 `observedAt`/sequence가 stale이면 `Ready`를 만들지 않는다.
- reconnect 이벤트 직후에도 fresh `ready` status가 다시 확인되기 전까지는 blocked로 유지한다.
- `disconnected`, `connecting`, `recovering`, `degraded`, `error`는 모두 false-ready 방지 관점에서 blocked path다.
- booth `canCapture=true`와 operator의 clear camera state는 같은 normalized truth에서 파생되어야 한다.

## booth/customer projection

booth가 받는 최종 결과는 helper raw 상태가 아니라 customer-safe readiness다.

- `Ready`
- `Preparing`
- `Phone Required`
- 필요 시 `Preview Waiting`, `Export Waiting`, `Completed`

고객 화면에는 아래를 노출하지 않는다.

- Canon SDK 이름
- EOS/driver/USB 상태 원문
- helper stderr
- sidecar restart 세부 원인

## operator projection

operator는 helper raw 상태 전체를 그대로 보지 않고, bounded operator-safe projection만 본다.

최소 의미 집합 예시:

- `미연결`
- `연결 중`
- `연결됨`
- `복구 필요`

이 projection은 booth readiness와 모순되면 안 된다.

## 메시지 종류 v1

### host -> helper

- `helper-hello`
  - helper version, protocol version 협상
- `configure-runtime`
  - runtime root, diagnostics path, branch/runtime profile
- `bind-session`
  - `sessionId`, `boothAlias`, correlation seed
- `request-capture`
  - `sessionId`, `requestId`, active preset reference
  - 현재 제품 기준 booth 앱의 `사진 찍기` 버튼에서만 시작되는 supported capture trigger
- `request-recovery`
  - approved restart/recovery action
- `shutdown`

### helper -> host

- `helper-ready`
  - helper boot 완료와 protocol version 확인
  - 현재 profile에서는 helper version, runtime platform, sdk family/version 같은 진단 정보를 함께 남길 수 있어야 한다
- `camera-status`
  - Canon/camera raw 상태와 freshness 정보
- `capture-accepted`
  - helper가 host-owned `requestId`를 수락했음을 알림
- `file-arrived`
  - helper-owned `captureId`, host-owned `requestId`, session-scoped RAW 경로를 함께 보냄
  - optional `fastPreviewPath`, `fastPreviewKind`를 같이 보낼 수 있지만, host는 이를 capture success의 필수 조건으로 취급하지 않는다
- `recovery-status`
  - restart/recovery 진행 상태
- `helper-error`
  - machine-readable code와 bounded detail

## camera-status 예시

```json
{
  "schemaVersion": "canon-helper-status/v1",
  "type": "camera-status",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "sequence": 42,
  "observedAt": "2026-03-27T10:15:30Z",
  "cameraState": "ready",
  "helperState": "healthy",
  "cameraModel": "Canon EOS 700D",
  "requestId": null,
  "detailCode": "camera-ready"
}
```

## file-arrived 예시

```json
{
  "schemaVersion": "canon-helper-file-arrived/v1",
  "type": "file-arrived",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "requestId": "capture_req_20260327_001",
  "captureId": "capture_20260327_001",
  "arrivedAt": "2026-03-27T10:15:33Z",
  "rawPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/captures/originals/capture_20260327_001.cr3",
  "fastPreviewPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/capture_20260327_001.jpg",
  "fastPreviewKind": "embedded-jpeg"
}
```

## freshness 기준

- helper status는 `observedAt` 또는 `sequence` 기준으로 최신성 판단이 가능해야 한다.
- host는 최근 fresh status가 없으면 `Ready`를 만들지 않는다.
- stale status가 마지막으로 남아 있어도 booth는 blocked guidance로 내려간다.
- reconnect 후 첫 fresh `camera-status`가 오기 전에는 `Ready` 복귀가 금지된다.

## file handoff 규칙

- Canon helper는 촬영 파일을 session-scoped filesystem root 아래에 전달한다.
- host는 `file-arrived` correlation과 실제 파일 존재를 함께 확인한다.
- host는 `captures/originals/` 아래 active session root에 들어온 파일만 capture success 후보로 인정한다.
- capture success는 helper가 "촬영 버튼을 받았다"가 아니라 host가 파일 도착과 저장 경계를 닫았을 때 확정된다.
- helper가 `fastPreviewPath`를 같이 보내더라도 host는 same-session, same-capture, allowed-path, 파일 유효성 검증을 다시 통과한 경우에만 이를 pending preview 후보로 승격할 수 있다.
- 현재 구현 기준 allowed fast preview path는 designated handoff 경로(`handoff/fast-preview/...`) 또는 canonical preview path(`renders/previews/{captureId}.jpg`와 동등 경로)로 제한한다.
- `fastPreviewPath` 부재, 손상, stale, wrong-session, wrong-capture는 capture failure 이유가 아니다. 이 경우 host는 RAW handoff만으로 기존 `Preview Waiting` 경로를 계속 유지한다.
- `fastPreviewKind`는 helper가 어떤 후보를 보냈는지 설명하는 optional 진단 힌트일 뿐이며, host preview truth를 직접 결정하지 않는다.
- helper는 partial file이나 아직 close되지 않은 파일을 `file-arrived`로 알리면 안 된다.
- 같은 원칙으로 helper는 partial request line이나 restart 이전의 stale request line을 새 촬영으로 재해석하면 안 된다.
- 카메라 본체 셔터 직접 입력처럼 active `requestId` 없이 발생한 out-of-band 촬영은 현재 host가 active session success로 승격하는 canonical path가 아니다.
- helper가 이런 out-of-band 촬영을 감지하더라도, host는 이를 supported booth capture success나 `Preview Waiting` 시작 근거로 자동 해석하면 안 된다.

## 에러와 복구 원칙

- helper raw 에러는 host error envelope로 다시 포장된 뒤에만 frontend로 간다.
- booth는 customer-safe next action만 받는다.
- operator는 bounded recovery action과 operator-safe detail만 본다.
- helper restart는 allowed recovery path일 수 있지만, restart 중에는 `Ready`가 유지되면 안 된다.
- once-ready 이후 USB 분리나 session loss가 오면 helper는 stale `ready`를 붙들지 말고 즉시 blocked path로 내려가야 한다.

## Story 매핑

- Story 1.4: readiness/capture guard baseline
- Story 1.6: 실카메라/helper truth 연결과 false-ready 차단
- Story 5.4: operator `카메라 연결 상태` 전용 항목

## 현재 상태 메모

아키텍처는 이미 helper 경계를 기대하고 있고, 현재 제품의 실행 결정은 Windows 전용 Canon EDSDK helper exe다.
repo 기준으로는 아직 `sidecar/canon-helper/`와 `src-tauri/src/capture/sidecar_client.rs`가 구현 기준선으로 명시된 상태다.

따라서 이 문서는 generic draft에 머무르지 않고, 채택된 Canon EDSDK helper exe가 따라야 할
canonical contract 기준선으로 본다.
