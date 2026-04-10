# Canon Camera Helper Sidecar Protocol

## 목적

이 문서는 Boothy가 Canon 계열 실카메라 상태를 Tauri/Rust host에서 어떻게 관찰하고 정규화하는지에 대한
최소 계약 기준을 고정한다.

이 계약의 목표는 두 가지다.

1. React가 Canon SDK truth를 직접 보지 않게 한다.
2. booth `Ready`와 operator `카메라 연결 상태`가 같은 host-owned truth에서 나오게 한다.

## Authoritative Source / 소비 경계

- 문서 기준: 이 문서
- Rust 기준: `src-tauri/src/capture/sidecar_client.rs`, `src-tauri/src/capture/normalized_state.rs`
- helper 기준: `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs`
- durable example path:
  - `sidecar/protocol/examples/helper-ready.json`
  - `sidecar/protocol/examples/camera-status.json`
  - `sidecar/protocol/examples/file-arrived.json`
  - `sidecar/protocol/examples/recovery-status.json`
  - `sidecar/protocol/examples/helper-error.json`

## 현재 채택 결정

- Boothy의 현재 채택안은 Windows 전용 Canon EDSDK 기반 `canon-helper.exe`다.
- helper target runtime은 Windows 10/11 x64용 `net8.0` baseline이다.
- 이 문서의 helper 계약은 위 실행 결정을 전제로, Tauri host와 helper exe 사이의 canonical sidecar protocol을 고정한다.
- Story 1.14는 protocol 의미와 fixture baseline만 잠그고, helper packaging/runtime deeper profile은 Story 1.15가 계속 소유한다.
- app instance당 helper는 하나의 활성 카메라와 하나의 in-flight capture만 동시에 소유한다.

## 구현 프로파일 문서

- 현재 채택된 helper의 구체 구현 기준선은 `docs/contracts/camera-helper-edsdk-profile.md`에 둔다.
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

- transport: session diagnostics file boundary
  - request input: `sessions/<sessionId>/diagnostics/camera-helper-requests.jsonl`
  - status output: `sessions/<sessionId>/diagnostics/camera-helper-status.json`
  - event output: `sessions/<sessionId>/diagnostics/camera-helper-events.jsonl`
  - helper는 필요 시 같은 JSON payload를 stdout에 echo할 수 있지만, canonical host contract는 위 파일 경계다
- framing: BOM 없는 UTF-8 JSON Lines, 메시지 1개당 1줄
- versioning: 모든 메시지는 `schemaVersion`을 가진다
- binary rule: RAW 이미지 bytes는 JSON payload로 보내지 않고 filesystem handoff로 전달한다
- correlation rule: host와 helper는 `sessionId`, `requestId`, `captureId`를 함께 사용한다
- ownership rule: `requestId`는 host가 생성해 `request-capture`에 싣고, `captureId`는 helper가 실제 RAW handoff 시점에 확정해 `file-arrived`로 돌려준다
- session rule: helper가 쓰는 status/event는 현재 바인딩된 `sessionId`와 일치해야 하며 mismatch는 false-ready가 아니라 recovery/error 진단으로만 남긴다
- supported trigger rule: canonical capture success path는 host가 booth 앱의 `사진 찍기` 버튼에 응답해 보낸 `request-capture`다
- request log rule: helper는 소비한 `requestId`를 재시작 후에도 기억해 기존 request log의 과거 line을 재실행하면 안 된다
- in-flight rule: helper는 `capture-accepted` 이후 `file-arrived` 또는 `helper-error`/`recovery-status`로 닫히기 전까지 추가 촬영을 동시에 시작하면 안 된다

## 메시지 종류 v1

### host -> helper

- `helper-hello`
- `configure-runtime`
- `bind-session`
- `request-capture`
- `request-recovery`
- `shutdown`

### helper -> host

- `helper-ready`
- `camera-status`
- `capture-accepted`
- `fast-preview-ready`
  - RAW handoff 이전 또는 직후에 same-capture first-visible candidate가 있을 때 보내는 advisory event
- `fast-thumbnail-attempted`
  - diagnostic-only event
- `fast-thumbnail-failed`
  - diagnostic-only event
- `file-arrived`
- `recovery-status`
- `helper-error`

## camera-status 예시

```json
{
  "schemaVersion": "canon-helper-status/v1",
  "type": "camera-status",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "sequence": 42,
  "observedAt": "2026-04-10T01:00:15Z",
  "cameraState": "ready",
  "helperState": "healthy",
  "cameraModel": "Canon EOS 700D",
  "requestId": null,
  "detailCode": "camera-ready"
}
```

## helper-ready 예시

```json
{
  "schemaVersion": "canon-helper-ready/v1",
  "type": "helper-ready",
  "helperVersion": "0.1.0",
  "protocolVersion": "v1",
  "runtimePlatform": "Windows 11 x64 / .NET 8.0",
  "sdkFamily": "canon-edsdk",
  "sdkVersion": "13.20.10"
}
```

## file-arrived 예시

```json
{
  "schemaVersion": "canon-helper-file-arrived/v1",
  "type": "file-arrived",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "requestId": "request_20260410_001",
  "captureId": "capture_20260410_001",
  "arrivedAt": "2026-04-10T01:00:18Z",
  "rawPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/captures/originals/capture_20260410_001.cr3",
  "fastPreviewPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/capture_20260410_001.jpg",
  "fastPreviewKind": "embedded-jpeg"
}
```

## recovery-status 예시

```json
{
  "schemaVersion": "canon-helper-recovery-status/v1",
  "type": "recovery-status",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "recoveryState": "recovering",
  "observedAt": "2026-04-10T01:00:17Z",
  "detailCode": "recovery-reopen-session"
}
```

## helper-error 예시

```json
{
  "schemaVersion": "canon-helper-error/v1",
  "type": "helper-error",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "observedAt": "2026-04-10T01:00:22Z",
  "detailCode": "capture-download-timeout",
  "message": "RAW handoff를 기다리다 시간이 초과되었어요."
}
```

## host 정규화 규칙

- `ready`는 helper raw 상태 하나만으로 성립하지 않는다.
- host는 최소 `cameraState`, `helperState`, `freshness`, `active session`, `active preset`, `blocked post-end 여부`를 함께 본다.
- stale status, reconnect 직후 상태, wrong-session 상태에서는 `Ready`를 만들지 않는다.
- `fast-preview-ready`와 fast thumbnail 계열 이벤트는 advisory telemetry다.
  - RAW handoff를 대체하지 않는다.
  - truthful `previewReady`를 직접 주장하지 않는다.
- booth `canCapture=true`와 operator의 clear camera state는 같은 normalized truth에서 파생되어야 한다.

## file handoff 규칙

- Canon helper는 촬영 파일을 session-scoped filesystem root 아래에 전달한다.
- host는 `file-arrived` correlation과 실제 파일 존재를 함께 확인한다.
- `captures/originals/` 아래 active session root에 들어온 파일만 capture success 후보로 인정한다.
- helper가 `fastPreviewPath`를 같이 보내더라도 host는 same-session, same-capture, allowed-path 검증을 다시 통과한 경우에만 pending preview 후보로 승격할 수 있다.
- `fastPreviewPath` 부재, 손상, stale, wrong-session, wrong-capture는 capture failure 이유가 아니다.
- helper는 partial file이나 아직 close되지 않은 파일을 `file-arrived`로 알리면 안 된다.

## 에러와 복구 원칙

- helper raw 에러는 host error envelope로 다시 포장된 뒤에만 frontend로 간다.
- booth는 customer-safe next action만 받는다.
- operator는 bounded recovery action과 operator-safe detail만 본다.
- helper restart는 allowed recovery path일 수 있지만, restart 중에는 `Ready`가 유지되면 안 된다.

## Story 매핑

- Story 1.4: readiness/capture guard baseline
- Story 1.6: 실카메라/helper truth 연결과 false-ready 차단
- Story 1.14: sidecar protocol fixture와 canonical message baseline 동결
- Story 1.15: Canon helper profile / deeper runtime detail
- Story 5.4: operator `카메라 연결 상태` 전용 항목
