# Boothy capture failed / capture pipeline 조사 가이드

이 문서는 새 세션에서 LLM이 `capture failed`, `Failed to configure camera.`, `photoTransferred 미도착`, `import 미완료` 같은 촬영 관련 이슈를 빠르게 좁히기 위한 실전 가이드다.

목표:
1. 캡처 파이프라인을 단계별로 끊어서 보기
2. 로그 증거로 실패 레이어를 먼저 확정하기
3. 필요한 파일만 부분 읽기
4. 수정 후 같은 형식으로 세션 기록 남기기

---

## 1. 캡처 파이프라인 맵

실제 촬영 흐름은 아래 순서다.

1. 프론트 `촬영` 버튼 클릭
2. Rust `boothy_trigger_capture`
3. Rust `set_session_destination()`
4. Rust `camera.capture` IPC 요청
5. sidecar `HandleCaptureAsync()`
6. `RealCameraController.CaptureAsync()`
7. Canon EDSDK `EdsSendCommand(TakePicture)`
8. sidecar object event에서 다운로드
9. sidecar `event.camera.photoTransferred`
10. 프론트 `boothy-photo-transferred` 수신
11. Rust `boothy_handle_photo_transferred`
12. `FileArrivalWatcher` 안정화 후 `boothy-new-photo`
13. 프론트 라이브러리 새 사진 반영

핵심: `capture failed`는 4~7단계 실패일 수도 있고, 사용자는 같은 증상으로 느껴도 실제 원인은 8~13단계일 수도 있다.

---

## 1A. 핵심 참조 문서 (새 세션 빠른 점프)

촬영 버튼 클릭 시 "카메라 신호 전송 → 촬영/전송 완료 수신 → 앱 반영" 흐름을 가장 빠르게 파악하려면 아래 5개를 우선 본다.

1. `work/capture.md` (현재 문서)
   - 운영/디버깅 관점의 실제 파이프라인과 로그 판정 기준
2. `docs/architecture/api-design-and-integration.md`
   - `camera.setSessionDestination`, `camera.capture`, `event.camera.photoTransferred` 메시지 계약
3. `docs/architecture/component-architecture.md`
   - Sidecar(cmd/evt)와 Boothy watcher/ingest 연결 구조
4. `docs/stories/4.1.real-camera-capture-transfer-ingest-preset-export.md`
   - E2E 요구사항(촬영, 전송완료 후 ingest, preset 적용, export)과 검증 기준
5. `reference/camerafunction/digiCamControl-2.0.0` (코드 참조)
   - 문서보다 구현 기준으로 확인
   - `CameraControl.Core/Classes/PipeServerCommands.txt`: capture 관련 명령 포맷
   - `CameraControl.Application/WebServer/index.html`: Capture 클릭 시 `/?CMD=Capture`
   - `CameraControl.Core/Classes/WebServerModule.cs`: `CMD` 처리 및 `liveview.jpg` 제공
   - `CameraControl/MainWindow.xaml.cs`: `PhotoCaptured` 수신 후 파일 전송/세션 반영/UI 선택

토큰 절약 원칙:
1. 먼저 2, 3, 4를 읽고 계약/구조/요구사항을 맞춘다
2. 구현 불일치가 의심될 때만 5번 코드 참조를 라인 범위로 확인한다

## 2. 새 세션 시작 시 권장 분석 순서

### Step A. 로그 먼저
아래 두 로그를 같이 봐야 한다.

1. `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-YYYYMMDD.log`
2. `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-YYYYMMDD.log`

권장 검색:

```powershell
rg -n "Failed to configure camera|Capture failed|camera.capture|camera.setSessionDestination|boothy-camera-error|boothy-photo-transferred|boothy-new-photo|Recovering sidecar before camera.capture|IPC pipe write timeout" C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260306.log -S

rg -n "Received Request: camera.capture|Capture requested|Sent Error: event.camera.error|Sent Response: camera.capture|Setting session destination|Photo transferred|EdsSendCommand|EdsDownload|Session destination not set" C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-20260305.log -S
```

### Step B. 실패 레이어 판정

1. boothy 로그에 `Preparing to send request: camera.setSessionDestination`만 있고 sidecar 로그에 `Received Request: camera.setSessionDestination`가 없으면
   - Rust IPC write 단계 실패

2. sidecar 로그에 `Received Request: camera.capture`는 있는데 곧바로 `Sent Error: event.camera.error`가 나오면
   - sidecar/SDK capture 단계 실패

3. `camera.capture` 응답은 성공인데 `event.camera.photoTransferred`가 없으면
   - EDSDK object event / 다운로드 단계 실패

4. `event.camera.photoTransferred`는 있는데 `boothy-new-photo`가 안 나오면
   - Rust ingest/stabilization 단계 실패

---

## 3. 꼭 읽어야 하는 파일 우선순위

### 1순위: 캡처 실패 직결
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\main.rs`
   - `boothy_trigger_capture`
   - `boothy_handle_photo_transferred`
2. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`
   - `send_request_with_options`
   - `set_session_destination`
   - `event.camera.error` / `event.camera.photoTransferred` 처리
3. `C:\Code\Project\Boothy\apps\camera-sidecar\Program.cs`
   - `HandleCaptureAsync`
4. `C:\Code\Project\Boothy\apps\camera-sidecar\Camera\RealCameraController.cs`
   - `CaptureAsync`
   - object event 다운로드 처리

### 2순위: 프론트 체감 증상 확인
1. `C:\Code\Project\Boothy\apps\boothy\src\App.tsx`
   - `handleTriggerCapture`
   - `boothy-capture-started`
   - `boothy-photo-transferred`
   - `boothy-camera-error`
2. `C:\Code\Project\Boothy\apps\boothy\src\captureStatus.ts`

### 3순위: ingest 후속 단계
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\ingest\file_watcher.rs`
2. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src/error.rs`
3. `C:\Code\Project\Boothy\apps\camera-sidecar\IPC\IpcMessage.cs`

---

## 4. 라인 범위로만 읽을 것

토큰 낭비를 막기 위해 전체 파일 dump 금지.

권장 방식:

```powershell
rg -n "boothy_trigger_capture|boothy_handle_photo_transferred" C:\Code\Project\Boothy\apps\boothy\src-tauri\src\main.rs -S
rg -n "send_request_with_options|set_session_destination|event.camera.error|event.camera.photoTransferred" C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs -S
rg -n "HandleCaptureAsync|camera.capture" C:\Code\Project\Boothy\apps\camera-sidecar\Program.cs -S
rg -n "CaptureAsync|EdsSendCommand|EdsDownload|EmitError|Session destination not set" C:\Code\Project\Boothy\apps\camera-sidecar\Camera\RealCameraController.cs -S
rg -n "boothy-capture-started|boothy-photo-transferred|boothy-camera-error|handleTriggerCapture" C:\Code\Project\Boothy\apps\boothy\src\App.tsx -S
```

---

## 5. 증상 -> 로그 패턴 -> 우선 가설

### 케이스 A. `Failed to configure camera.`
로그 패턴:
1. `camera.setSessionDestination` write timeout
2. sidecar에 `Received Request: camera.setSessionDestination` 없음

우선 가설:
1. stale pipe / writer 경합
2. polling `camera.getStatus`가 선행 timeout으로 파이프를 독성 상태로 남김

### 케이스 B. `Capture failed.` 또는 준비 메시지 반복
로그 패턴:
1. sidecar에 `Received Request: camera.capture`
2. 바로 `Sent Error: event.camera.error`
3. 응답 payload는 `{ success: false }`

우선 가설:
1. `Session destination not set`
2. `EnsureCameraSessionOpen()` 실패
3. `EdsSendCommand(TakePicture)` 실패

### 케이스 C. 버튼은 반응했는데 새 사진이 안 들어옴
로그 패턴:
1. `camera.capture` 성공
2. `event.camera.photoTransferred` 없음 또는 늦음

우선 가설:
1. Canon object event 미수신
2. `EdsDownload` / `EdsDownloadComplete` 실패

### 케이스 D. transfer 됐는데 라이브러리에 안 뜸
로그 패턴:
1. sidecar `Photo transferred`
2. 프론트 `boothy-photo-transferred`
3. 이후 `boothy-new-photo` 없음 또는 import error

우선 가설:
1. stabilization timeout
2. file lock
3. preset apply / background export 연쇄 문제

---

## 6. 새 세션에서 바로 볼 핵심 포인트

1. `camera.capture`가 sidecar까지 갔는지 먼저 확인
2. sidecar `event.camera.error`가 capture response보다 먼저 나왔는지 확인
3. sidecar restart가 있었다면 `sessionDestination`가 유실됐는지 확인
4. `set_session_destination()` 성공 후 `camera.capture` 직전에 다시 복구/restart가 끼어드는지 확인
5. `photoTransferred`가 안 오면 capture가 아니라 transfer 문제로 분리

---

## 7. 완료 보고 템플릿

1. 증상:
2. 로그 증거 3~5개:
3. 근본 원인:
4. 수정 파일:
5. 핵심 변경:
6. 검증 명령/결과:
7. 남은 리스크:

---

## 8. 2026-03-06 세션 기록은 아래에 추가

아래 형식으로 계속 누적 기록한다.

---

## 9. 2026-03-06 세션 기록: `capture failed` 직전 recovery가 session destination을 날림

### 증상
1. `Failed to configure camera.`는 더 이상 주원인이 아니었음
2. `camera.capture` 요청은 sidecar까지 도달함
3. 그러나 사용자 체감상 촬영은 실패하고 `capture failed` 계열 에러가 발생
4. 실제 사진 전송(`photoTransferred`)은 시작되지 않음

### 로그 증거
1. `2026-03-06 07:01:12.290` sidecar `Received Request: camera.setSessionDestination`
2. `2026-03-06 07:01:12.292` sidecar `Setting session destination: 3333 -> C:\Users\KimYS\Pictures\dabi_shoot\3333\Raw`
3. `2026-03-06 07:01:17.816` boothy `IPC pipe write timeout during camera.getStatus - skipping sidecar restart for polling request`
4. `2026-03-06 07:01:17.817` boothy `Recovering sidecar before camera.capture because the previous polling write timed out`
5. `2026-03-06 07:01:19.162` sidecar `Received Request: camera.capture`
6. 직후 `2026-03-06 07:01:19.167` sidecar `Sent Error: event.camera.error`
7. 같은 구간에 sidecar 로그에는 두 번째 `Setting session destination:`가 없음

핵심: `camera.capture`는 실제로 sidecar까지 갔지만, 그 직전 recovery가 sidecar를 재시작하면서 방금 설정한 `sessionDestination` 상태를 날려 버렸다.

### 근본 원인
1. 이전 수정에서 `should_recover_before_request()`가 polling `camera.getStatus` write timeout 이후 모든 non-polling 요청에 대해 선제 recovery를 수행했다
2. `boothy_trigger_capture`는 `set_session_destination()` 후에 `camera.capture`를 연속 호출한다
3. 그런데 `camera.capture` 직전 recovery가 다시 sidecar를 restart 하면서, 바로 직전에 성공한 `sessionDestination`이 새 sidecar 인스턴스에 복원되지 않았다
4. 그 결과 sidecar `CaptureAsync()`는 `Session destination not set` 계열 오류를 보내고 `success=false` 응답을 반환할 수 있었다

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`

### 핵심 변경
1. `should_recover_before_request()`의 recovery 대상을 `camera.setSessionDestination`로만 제한
2. 즉 stale polling write timeout 복구는 destination 설정 직전에만 수행
3. `camera.capture` 직전에는 sidecar를 다시 restart 하지 않도록 변경
4. 결과적으로 recovery 후 새 sidecar에 destination을 먼저 다시 심고, 그 다음 capture가 같은 인스턴스에서 실행되게 함

### 회귀 테스트
1. `non_polling_requests_recover_after_poll_write_timeout`
2. 기대 규칙:
   - `camera.setSessionDestination` => recovery 대상
   - `camera.capture` => recovery 대상 아님
   - `camera.getStatus` => recovery 대상 아님

### 검증
```powershell
cargo test non_polling_requests_recover_after_poll_write_timeout --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### 남은 리스크
1. 이 수정은 `camera.capture` 직전의 잘못된 restart를 막는 것이다
2. 이후에도 `event.camera.error`가 계속 나오면 다음 분석 대상은 `RealCameraController.CaptureAsync()` 내부의 `EnsureCameraSessionOpen()`와 `EdsSendCommand(TakePicture)` 실패 원인이다
3. `camera.capture` 성공 후 `photoTransferred`가 없으면 transfer/object event 레이어로 문제를 분리해야 한다

---

## 10. 2026-03-06 세션 기록: `IPC pipe write timeout during camera.capture` 최종 해결

### 최종 상태
1. 캡처 기능 재활성화 확인
2. 이번 이슈의 최종 사용자 체감 오류는 빨간 토스트 `IPC pipe write timeout during camera.capture`
3. 최종 원인은 Canon SDK 자체보다 앞단의 Named Pipe 요청 순서/경합 문제였다

### 이번 세션에서 확인한 로그 사실
1. `2026-03-06 07:25:59.676` boothy `Recovering sidecar before camera.setSessionDestination because the previous polling write timed out`
2. `2026-03-06 07:26:01.188` sidecar `Received Request: camera.setSessionDestination`
3. `2026-03-06 07:26:01.190` sidecar `Setting session destination: 3333 -> C:\Users\KimYS\Pictures\dabi_shoot\3333\Raw`
4. `2026-03-06 07:26:01.687` sidecar `Sent Response: camera.setSessionDestination`
5. `2026-03-06 07:26:06.698` boothy `IPC pipe write timeout during camera.getStatus - skipping sidecar restart for polling request`
6. `2026-03-06 07:26:06.699` boothy `Preparing to send request: camera.capture`
7. `2026-03-06 07:26:11.712` boothy `IPC pipe write timeout during camera.capture - restarting sidecar`
8. 같은 구간 sidecar 로그에는 `Received Request: camera.capture`가 전혀 없음

핵심: `camera.setSessionDestination`는 이미 성공했고, 그 뒤의 `camera.capture`가 sidecar까지 도달하지 못했다. 따라서 이 실패는 카메라 SDK가 아니라 Boothy 쪽 pipe write 경로/요청 순서 문제다.

### 지금까지 시도했던 방법과 판정
1. 캡처 파이프라인을 단계별로 분해:
   - `camera.setSessionDestination` 이전 실패인지
   - `camera.capture`가 sidecar까지 갔는지
   - `event.camera.error`인지
   - `photoTransferred` 이후 ingest 문제인지
2. `Received Request:`가 sidecar 로그에 없으면 Rust IPC write 문제로 분류
3. 첫 번째 수정:
   - polling `camera.getStatus` write timeout 이후 recovery 신호를 유지
   - `camera.capture` 직전 restart가 `sessionDestination`를 날리지 않게 조정
   - 이 수정으로 `Failed to configure camera.` 주원인은 해소됐고 `setSessionDestination`는 성공하게 됨
4. 두 번째 관찰:
   - `setSessionDestination` 성공 후에도 같은 연결에서 다음 `camera.getStatus`와 `camera.capture` write가 다시 막힘
   - sidecar는 첫 요청 하나는 처리하지만 그 다음 요청에서 pipe가 다시 독성 상태가 되는 패턴
5. 최종 전략:
   - 캡처를 "새 sidecar 연결에서 보내는 첫 요청"으로 만들기
   - 캡처 임계구간 동안 `camera.getStatus` polling을 완전히 끼워 넣지 않기
   - `setSessionDestination`와 `camera.capture`를 분리된 두 개 요청으로 보내지 않고, capture payload 안에 destination을 같이 싣기

### 최종 해결 방법
1. Rust `CameraIpcClient`에 캡처 전용 임계구간 추가
2. 캡처 시작 시:
   - 기존 sidecar를 restart
   - 새 sidecar 연결을 수립
   - `camera.capture`를 첫 요청으로 전송
3. `camera.capture` payload 안에 다음 정보를 함께 보냄:
   - `destinationPath`
   - `sessionName`
4. sidecar `HandleCaptureAsync()`에서 capture payload를 읽어 capture 직전에 `SetSessionDestination()` 수행
5. capture 중에는 backend status monitor가 `camera.getStatus`를 보내지 않도록 일시 억제
6. 결과적으로 `setSessionDestination -> getStatus poll -> capture` 경합을 `fresh sidecar -> capture(with destination)` 단일 흐름으로 바꿈

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`
2. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\main.rs`
3. `C:\Code\Project\Boothy\apps\camera-sidecar\Program.cs`
4. `C:\Code\Project\Boothy\apps\camera-sidecar\IPC\IpcMessage.cs`

### 핵심 코드 변경
1. `ipc_client.rs`
   - `capture_with_session_destination(...)` 추가
   - `build_capture_request_payload(...)` 추가
   - `capture_in_progress` / `capture_flow_lock` 추가
   - status monitor가 capture 중 `camera.getStatus`를 건너뛰도록 변경
2. `main.rs`
   - `boothy_trigger_capture`가 더 이상 `setSessionDestination`와 `camera.capture`를 따로 보내지 않음
   - `capture_with_session_destination(...)` 한 번으로 capture 수행
   - `boothy_camera_get_status`는 capture 중이면 즉시 skip
3. sidecar
   - `CaptureRequest` payload 타입 추가
   - `HandleCaptureAsync()`가 payload의 destination을 받아 capture 직전에 `SetSessionDestination()` 실행

### 이번 작업에서 추가한 테스트 / 검증
1. Rust 회귀 테스트
   - `non_polling_requests_recover_after_poll_write_timeout`
   - `start_sidecar_noop_preserves_poll_write_timeout_for_followup_recovery`
   - `capture_request_payload_embeds_destination_and_session_name`
   - `status_requests_are_suppressed_while_capture_is_active`
2. 검증 명령
```powershell
cargo test capture_request_payload_embeds_destination_and_session_name --manifest-path src-tauri/Cargo.toml
cargo test status_requests_are_suppressed_while_capture_is_active --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
dotnet build ..\camera-sidecar\Boothy.CameraSidecar.csproj -c Release
dotnet publish ..\camera-sidecar\Boothy.CameraSidecar.csproj -c Release -r win-x86 -o ..\camera-sidecar\bin\Release\net8.0\win-x86\publish
```
3. 검증 결과
   - Rust 테스트 `64 passed, 0 failed`
   - sidecar publish 성공
   - publish exe 갱신 시각 확인: `2026-03-06 07:38`
   - 이후 사용자 확인: 캡처 기능 활성화

### 이번 이슈에서 습득한 실전 규칙
1. sidecar 로그에 `Received Request:`가 없으면 Canon SDK를 보기 전에 Rust IPC write 경로를 먼저 의심한다
2. `sessionDestination` 같은 sidecar 메모리 상태는 restart 이후 자동 복원되지 않는다
3. polling `camera.getStatus`는 harmless하지 않다. capture 직전/직후에는 실제로 pipe write 경합 원인이 될 수 있다
4. 현재 구조에서는 "fresh connection에서 capture를 first request로 보내는 전략"이 가장 안정적이다
5. debug build에서 Boothy는 `apps/camera-sidecar/bin/Release/net8.0/win-x86/publish/Boothy.CameraSidecar.exe`를 우선 사용하므로, sidecar 수정 후 `dotnet publish`까지 해야 실제 실행 파일이 바뀐다
6. `dotnet build`만으로는 publish exe가 갱신되지 않아 현상 재발처럼 보일 수 있다

### 다음에 같은 증상이 나오면 보는 순서
1. boothy 로그에서 `Preparing to send request: camera.capture` 이후 `Sent request:`가 찍혔는지 확인
2. sidecar 로그에 `Received Request: camera.capture`가 찍혔는지 확인
3. 안 찍혔으면 IPC write 문제
4. 찍혔으면 `Sent Error: event.camera.error` 또는 `success=false`인지 확인
5. capture는 성공했는데 `photoTransferred`가 없으면 transfer/object event로 분리
6. `photoTransferred`는 왔는데 라이브러리에 안 뜨면 ingest/file watcher/preset apply로 분리

---

## 11. 2026-03-06 세션 기록: `camera.capture` 내부 중복 warm-up으로 IPC timeout 재발

### 증상
1. 사용자가 "잘 되던 캡처가 다시 안 된다"고 보고
2. UI에서는 촬영 버튼이 눌리지만 실제 사진은 생성되지 않음
3. 최종 사용자 오류는 `Camera service is not responding. Please restart Boothy.`
4. 이번에는 `camera.capture` 요청 자체는 sidecar까지 정상 도달함

### 로그 증거
1. `2026-03-06 10:12:16.238` boothy `Preparing to send request: camera.capture`
2. `2026-03-06 10:12:16.311` sidecar `Received Request: camera.capture`
3. `2026-03-06 10:12:16.313` sidecar `Capture request applying session destination: 3333 -> C:\Users\KimYS\Pictures\dabi_shoot\3333\Raw`
4. 직후 correlation `corr-1772759536313-...` 에서
   - `EdsSetPropertyData(SaveTo=Host) returned DEVICE_BUSY` 4회 재시도
   - `EdsSetCapacity returned DEVICE_BUSY` 4회 재시도
   - `2026-03-06T01:12:20.437Z` 에 각각 failed(0x00000081)
5. 그 직후 다시 capture correlation `corr-1772759535044-...` 에서 `EdsSetPropertyData(SaveTo=Host)` 재시도가 다시 시작됨
6. `2026-03-06 10:12:21.253` boothy `IPC timeout during camera.capture - restarting sidecar`
7. 같은 구간에서 `EdsSendCommand(TakePicture)` 로그도 없고 `event.camera.photoTransferred`도 없음

### 조사 결론
1. 이번 실패는 IPC write 문제나 `camera.capture` 미도달 문제가 아님
2. `HandleCaptureAsync()`가 payload의 destination을 적용하면서 `SetSessionDestination()`을 호출했고
3. `SetSessionDestination()` 내부에서 `EnsureCameraSessionOpen(..., prepareHostCaptureTarget: true)`가 즉시 실행되며 host capture target warm-up을 한 번 수행함
4. 이어서 같은 요청 안의 `CaptureAsync()`가 다시 `EnsureCameraSessionOpen(..., prepareHostCaptureTarget: true)`를 호출하면서 동일 warm-up을 한 번 더 수행함
5. 즉, 캡처 1회에 대해 `SaveTo=Host`/`EdsSetCapacity` busy 재시도 구간이 2번 연속 실행되며 총 지연이 5초 IPC timeout을 초과함
6. 그래서 `TakePicture` 명령까지 가지 못하고 boothy가 sidecar를 timeout restart 시킴

### 근본 원인
1. 이전 수정에서 `camera.capture` payload에 `destinationPath`를 싣는 구조는 맞았음
2. 그러나 capture 요청 안에서 destination 업데이트를 "단순 경로 설정"이 아니라 "실제 카메라 warm-up"까지 수행하도록 연결해 둔 것이 문제였음
3. 그 결과 capture 경로에서 session destination 적용과 capture 준비가 중복 실행됨

### 이번 수정 사항
1. sidecar의 `SetSessionDestination()` 시그니처를 `prepareHostCaptureTarget` 플래그를 받도록 확장
2. 단독 `camera.setSessionDestination` 요청은 기존처럼 warm-up 유지
3. `camera.capture` 내부에서 destination을 적용할 때는 warm-up 없이 경로만 갱신하도록 변경
4. `RealCameraController.SetSessionDestination(..., prepareHostCaptureTarget: false)`에서는
   - destination path 기록
   - 디렉터리 생성
   - 그리고 즉시 return
   - SDK session open / `SaveTo=Host` / `EdsSetCapacity`는 수행하지 않음
5. 실제 warm-up과 `TakePicture`는 `CaptureAsync()` 한 곳에서만 수행되도록 정리

### 수정 파일
1. `C:\Code\Project\Boothy\apps\camera-sidecar\Program.cs`
2. `C:\Code\Project\Boothy\apps\camera-sidecar\Camera\ICameraController.cs`
3. `C:\Code\Project\Boothy\apps\camera-sidecar\Camera\RealCameraController.cs`
4. `C:\Code\Project\Boothy\apps\camera-sidecar\Camera\MockCameraController.cs`
5. `C:\Code\Project\Boothy\apps\camera-sidecar.Tests\ProgramTests.cs`

### 핵심 코드 포인트
1. `Program.HandleSetSessionDestinationAsync()`
   - `ShouldPrepareCameraForSessionDestinationUpdate(false)` 경유
   - 단독 destination 설정 요청은 warm-up 유지
2. `Program.HandleCaptureAsync()`
   - capture payload destination 적용 시 `ShouldPrepareCameraForSessionDestinationUpdate(true)` 사용
   - capture 내부 destination 업데이트는 warm-up 생략
3. `RealCameraController.SetSessionDestination()`
   - `prepareHostCaptureTarget == false` 이면 SDK session open을 시도하지 않음
4. `CaptureAsync()`
   - 실제 카메라 session open / host target 설정 / `TakePicture`는 여기서만 진행

### 이번에 추가한 회귀 테스트
1. `ProgramTests.ShouldPrepareCameraForSessionDestinationUpdate_MatchesRequestContext`
2. 검증 정책:
   - `isCaptureRequest = true` 이면 warm-up 금지
   - `isCaptureRequest = false` 이면 warm-up 허용

### 검증
1. 명령:
```powershell
dotnet test C:\Code\Project\Boothy\apps\camera-sidecar.Tests\Boothy.CameraSidecar.Tests.csproj --no-restore
dotnet publish C:\Code\Project\Boothy\apps\camera-sidecar\Boothy.CameraSidecar.csproj -c Release -r win-x86 -o C:\Code\Project\Boothy\apps\camera-sidecar\bin\Release\net8.0\win-x86\publish
```
2. 결과:
   - sidecar tests `11 passed, 0 failed`
   - publish 성공
   - 실제 사용 exe 갱신 시각: `2026-03-06 10:18:36`

### 현재 상태 판단
1. 최신 로그 기준 root cause는 코드상 제거됨
2. 아직 이 세션에서 실카메라로 직접 촬영 성공까지는 다시 누르지 못했음
3. 따라서 "코드 수정 + 테스트 + publish 완료" 상태이며, 최종 현장 검증은 사용자 재촬영 로그로 확인 필요

### 다음 확인 포인트
1. 수정 반영 후 새 로그에서 `camera.capture` 직후 `SaveTo=Host`/`EdsSetCapacity` warm-up이 한 번만 나타나는지 확인
2. 그 다음 `EdsSendCommand(TakePicture)`가 찍히는지 확인
3. 이후 `event.camera.photoTransferred`가 오면 capture 경로는 복구된 것
4. 만약 여전히 실패하면 다음 분기:
   - `EdsSendCommand(TakePicture)` 실패면 Canon SDK capture 자체 문제
   - `TakePicture`는 성공했는데 `photoTransferred`가 없으면 object event / download 문제

---

## 12. 2026-03-06 세션 기록: 현재까지 누적 시도 및 방법 요약

### 지금까지 확인한 실패 레이어
1. `camera.setSessionDestination` 자체가 sidecar에 도달하지 못한 IPC write 문제
2. polling `camera.getStatus` write timeout 뒤 잘못된 recovery가 `sessionDestination`을 날리는 문제
3. `camera.capture` 내부에서 destination 적용과 host warm-up이 중복되어 timeout 나는 문제
4. 최신 기준으로는 `camera.capture`는 sidecar까지 도달하지만 Canon SDK 단계에서 `DEVICE_BUSY`가 반복되는 문제

### 누적 수정 축
1. Rust IPC 쪽
   - `camera.capture` 응답 타임아웃을 10초로 연장
   - polling `camera.getStatus` write timeout은 soft failure로 처리
   - poll write timeout 뒤 recovery는 `camera.setSessionDestination` 직전에만 수행
   - capture 중 polling 억제
2. sidecar 요청 순서 쪽
   - fresh sidecar에서 `camera.capture`를 first request로 보내도록 조정
   - startup probe 대기는 Rust가 아니라 sidecar 내부에서 수행
3. sidecar capture 준비 쪽
   - `camera.capture` payload의 destination 적용 시 warm-up 없이 경로만 반영
   - 실제 `SaveTo=Host` / `EdsSetCapacity` / `TakePicture`는 `CaptureAsync()` 한 곳에서만 수행
4. Canon session 재사용 쪽
   - status probe가 연 Canon session이면 capture 전에 세션을 다시 열도록 변경
   - host capture target 준비 실패를 성공처럼 넘기지 않도록 변경
   - session close 시 probe/capture 준비 상태를 함께 초기화

### 현재까지 사용한 검증 방법
1. `boothy-20260306.log`와 `boothy-sidecar-20260306.log`를 같은 correlation id 기준으로 대조
2. `Received Request: camera.capture` 유무로 IPC 문제와 SDK 문제를 분리
3. `EdsSetPropertyData(SaveTo=Host)` / `EdsSetCapacity` / `EdsSendCommand(TakePicture)` 순서를 비교
4. `cargo test --manifest-path src-tauri/Cargo.toml`
5. `dotnet test C:\Code\Project\Boothy\apps\camera-sidecar.Tests\Boothy.CameraSidecar.Tests.csproj --no-restore`
6. `dotnet publish C:\Code\Project\Boothy\apps\camera-sidecar\Boothy.CameraSidecar.csproj -c Release -r win-x86 -o C:\Code\Project\Boothy\apps\camera-sidecar\bin\Release\net8.0\win-x86\publish`

### 아직 남아 있는 핵심 질문
1. 최신 실패가 여전히 Canon SDK `DEVICE_BUSY` 재시도 문제인지
2. 아니면 다시 `camera.capture` timeout / `camera.getStatus` 간섭 문제로 되돌아갔는지
3. 성공 기록의 실제 컨디션과 비교했을 때, 현재 흐름에서 빠진 요청이 있는지

---

## 13. 2026-03-06 추가 기록: 최신 timeout 분석과 이번 수정

### 최신 로그에서 확인한 실제 실패 지점
1. `boothy-20260306.log`
   - `2026-03-06 11:18:10.854` `Preparing to send request: camera.capture`
   - `2026-03-06 11:18:10.854` `Sent request: camera.capture`
   - `2026-03-06 11:18:20.871` `IPC timeout during camera.capture - restarting sidecar`
2. `boothy-sidecar-20260306.log`
   - `2026-03-06 02:18:10.927Z` `Received Request: camera.capture`
   - 직후 `Capture request applying session destination: 3333 -> ...\\Raw`
   - 이후 `EdsSetPropertyData(SaveTo=Host)`가 `DEVICE_BUSY` 재시도 끝에 실패
   - 마지막 로그가 `Failed to configure host capture target on newly opened Canon session`
3. 즉, 이번 실패는 `camera.capture`가 sidecar에 도달하지 못한 문제가 아니라,
   `SaveTo=Host` 준비 실패 뒤 sidecar가 응답/에러를 보내지 못하고 멈춘 흐름이었다.

### 이번에 적용한 수정
1. Boothy Rust IPC 경로
   - `capture_with_session_destination()`를 다시 `camera.setSessionDestination -> camera.capture` 순서로 복원
   - capture 중 polling 억제는 그대로 유지
   - 성공 기록과 같은 요청 순서를 최대한 맞춤
2. sidecar Canon session 경로
   - host capture target 준비 실패 시 즉시 `false`를 반환
   - 실패 cleanup 과정에서 동기 `CloseCameraSession()`을 호출하지 않도록 변경
   - 목적: `SaveTo=Host` 실패 직후 sidecar thread가 `EdsCloseSession`에서 멈추는 가능성 제거

### 이번 수정 후 로컬 검증
1. `cargo test --manifest-path src-tauri/Cargo.toml` -> `65 passed, 0 failed`
2. `dotnet test C:\Code\Project\Boothy\apps\camera-sidecar.Tests\Boothy.CameraSidecar.Tests.csproj --no-restore` -> `18 passed, 0 failed`
3. `cargo build --manifest-path src-tauri/Cargo.toml` 성공
4. `dotnet publish C:\Code\Project\Boothy\apps\camera-sidecar\Boothy.CameraSidecar.csproj -c Release -r win-x86 -o C:\Code\Project\Boothy\apps\camera-sidecar\bin\Release\net8.0\win-x86\publish` 성공
