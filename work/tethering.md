# Tethering 문제 해결 런북

작성일: 2026-03-06

이 문서는 다음 세션의 LLM이 `촬영 버튼을 눌렀고 캡처는 된 것처럼 보이는데 Raw 폴더에 결과물이 안 올라온다`는 류의 문제를 빠르게 좁히기 위한 실전 가이드다.

목표:
1. 5분 안에 실패 경계를 특정한다.
2. 불필요한 파일 dump 없이 필요한 로그와 코드만 본다.
3. 수정 전후 검증과 세션 기록을 표준화한다.

## 1. 먼저 알아야 할 사실

1. 현재 코드에서 `"촬영 성공"`은 `camera.capture` 응답의 `success=true`를 뜻한다.
- 이것은 사실상 `EdsSendCommand(TakePicture)`까지의 성공이다.
- 실제 Raw 파일이 세션 `Raw/`에 생겼다는 보장은 아니다.

2. 진짜 완료 신호는 아래 순서다.
- sidecar 로그의 `Photo transferred: ... -> ...\\Raw\\...`
- Boothy 로그의 `Photo transferred notification received: ...`
- Boothy 로그의 `File stable, triggering import: ...`

3. `Raw` 폴더에 파일이 없으면 React/UI부터 보면 안 된다.
- 이 경우 우선순위는 `Program.cs` / `RealCameraController.cs` / `ipc_client.rs`다.

4. `boothy-photo-transferred`, `boothy-new-photo`는 이벤트 이름이지, 항상 로그 문자열로 남는 것은 아니다.
- 로그 검색은 이벤트 이름보다 실제 로그 문구로 해야 한다.

5. `src-tauri/src/ingest/file_watcher.rs`와 `src-tauri/src/watcher/file_watcher.rs`를 혼동하지 말 것.
- 이번 이슈의 1차 경로는 `ingest/file_watcher.rs`다.
- `watcher/file_watcher.rs`는 세션 파일 변화 감시용이며, 의도적으로 `boothy-new-photo`를 emit하지 않는다.

6. 외부 셔터와 버튼 촬영은 완전히 같은 출발점이 아니다.
- 버튼 촬영은 `camera.capture` 요청이 먼저 간다.
- 외부 셔터는 `camera.capture` 로그 없이도 `HandleObjectEvent`에서 바로 `photoTransferred`가 나올 수 있다.

## 2. 정상 골든 패스

### A. 버튼 촬영 기준

1. 프론트
- `apps/boothy/src/App.tsx`
- `handleTriggerCapture()`가 `boothy_trigger_capture`를 invoke한다.

2. Rust/Tauri
- `apps/boothy/src-tauri/src/main.rs`
- `boothy_trigger_capture()`가 active session의 `raw_path`를 잡고 `capture_with_session_destination()`를 호출한다.

3. IPC client
- `apps/boothy/src-tauri/src/camera/ipc_client.rs`
- Boothy 로그 기대 문자열:
  - `Preparing to send request: camera.capture`

4. sidecar request handler
- `apps/camera-sidecar/Program.cs`
- sidecar 로그 기대 문자열:
  - `Received Request: camera.capture`
  - `Capture requested`
  - `Capture request applying session destination: <session> -> <Raw path>`

5. Canon capture
- `apps/camera-sidecar/Camera/RealCameraController.cs`
- `CaptureAsync()`는 `EdsSendCommand(TakePicture)`까지 담당한다.
- 여기까지 성공해도 Raw 파일은 아직 없을 수 있다.

6. Canon object event -> download
- 같은 파일의 `HandleObjectEvent()`
- sidecar 로그 기대 문자열:
  - `Photo transferred: IMG_xxxx.CR3 -> C:\...\Raw\IMG_xxxx.CR3`

7. sidecar event relay
- `apps/boothy/src-tauri/src/camera/ipc_client.rs`
- Boothy 로그 기대 문자열:
  - `Photo transferred: IMG_xxxx.CR3 (12345678 bytes)`

8. stabilization + ingest
- `apps/boothy/src-tauri/src/ingest/file_watcher.rs`
- Boothy 로그 기대 문자열:
  - `Photo transferred notification received: ...`
  - `Starting file stability check: ...`
  - `File stable, triggering import: ...`

9. UI refresh
- `apps/boothy/src/App.tsx`
- `boothy-new-photo` 이벤트를 받고 목록을 새로고침한다.
- 이 단계는 기본 로그가 약하므로, 앞 단계 로그와 실제 파일 존재를 먼저 본다.

### B. 외부 셔터 기준

1. `camera.capture` 관련 로그가 없어도 된다.
2. sidecar `HandleObjectEvent()`에서 바로 `Photo transferred:`가 나와야 한다.
3. 이후 Boothy 쪽 안정화/인제스트 흐름은 버튼 촬영과 동일해야 한다.

실전 해석:
1. 외부 셔터는 되는데 버튼 촬영만 안 되면 `App.tsx` -> `boothy_trigger_capture` -> `ipc_client.rs` -> `Program.cs` 쪽을 먼저 본다.
2. 버튼 촬영은 되는데 외부 셔터만 안 되면 Canon event pump / hotplug / external trigger 감지 쪽을 먼저 본다.
3. 둘 다 Raw 미반영이면 `RealCameraController.HandleObjectEvent()` 또는 그 이전 파이프/세션 destination 문제일 가능성이 크다.

## 3. 60초 분기표

1. 버튼 클릭 직후 sidecar 로그에 `Capture requested`가 없다.
- 문제 레이어: `App.tsx` / `boothy_trigger_capture` / `ipc_client.rs` / Named Pipe
- Canon SDK를 보기 전에 IPC write timeout, reconnect race를 확인한다.

2. `Capture requested`는 있는데 `Photo transferred:`가 없다.
- 문제 레이어: `RealCameraController.CaptureAsync()` 또는 `HandleObjectEvent()`
- `Session destination not set`
- `EdsSendCommand(TakePicture) failed`
- `EdsDownload failed`
- `EdsDownloadComplete failed`
- `Error handling Canon object event`

3. sidecar `Photo transferred:`는 있는데 실제 `Raw` 폴더에 파일이 없다.
- 문제 레이어: destination path mismatch, `File.Move`, 다른 세션 폴더로 기록
- 로그에 찍힌 최종 경로와 active session의 `raw_path`를 비교한다.

4. `Raw` 폴더에 파일은 있는데 Boothy 로그에 `Photo transferred notification received:`가 없다.
- 문제 레이어: sidecar event relay
- `ipc_client.rs`의 `event.camera.photoTransferred` 처리와 app emit 경로를 본다.

5. `Photo transferred notification received:`는 있는데 `File stable, triggering import:`가 없다.
- 문제 레이어: stabilization
- 아래 로그를 본다.
  - `File stabilization timeout:`
  - `File still locked after stabilization:`
  - `File not found:`

6. `File stable, triggering import:`까지 있는데 UI에 안 보인다.
- 문제 레이어: library refresh / selection / session path 불일치
- 이때 처음으로 `App.tsx`와 세션 상태를 본다.

## 4. 바로 쓰는 명령

### A. 최신 로그 파일 찾기

```powershell
$logDir = Join-Path $env:APPDATA 'Boothy\\logs'
$boothy = Get-ChildItem $logDir -Filter 'boothy-*.log' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$sidecar = Get-ChildItem $logDir -Filter 'boothy-sidecar-*.log' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$boothy.FullName
$sidecar.FullName
```

### B. 버튼 촬영 기준 핵심 로그만 추리기

```powershell
rg -n "Preparing to send request: camera.capture|IPC pipe write timeout during camera.capture|IPC pipe write timeout during camera.getStatus|Photo transferred:|Photo transferred notification received:|Starting file stability check:|File stable, triggering import:|File stabilization timeout:|File still locked after stabilization:" $boothy.FullName -S
```

```powershell
rg -n "Received Request: camera.capture|Capture requested|Capture request applying session destination|Photo transferred:|Session destination not set|EdsSendCommand\\(TakePicture\\) failed|EdsDownload failed|EdsDownloadComplete failed|Error handling Canon object event|Pipe server error" $sidecar.FullName -S
```

### C. 최근 로그 꼬리 확인

```powershell
Get-Content $boothy.FullName -Tail 120
```

```powershell
Get-Content $sidecar.FullName -Tail 120
```

### D. 관련 최근 변경 확인

```powershell
git status --short -- apps/boothy/src-tauri/src/camera/ipc_client.rs apps/boothy/src-tauri/src/main.rs apps/boothy/src-tauri/src/ingest/file_watcher.rs apps/boothy/src/App.tsx apps/camera-sidecar/Program.cs apps/camera-sidecar/Camera/RealCameraController.cs apps/camera-sidecar/IPC/NamedPipeServer.cs
```

```powershell
git diff -- apps/boothy/src-tauri/src/camera/ipc_client.rs apps/boothy/src-tauri/src/main.rs apps/boothy/src-tauri/src/ingest/file_watcher.rs apps/boothy/src/App.tsx apps/camera-sidecar/Program.cs apps/camera-sidecar/Camera/RealCameraController.cs apps/camera-sidecar/IPC/NamedPipeServer.cs
```

### E. 관련 코드 위치만 찾기

```powershell
rg -n "handleTriggerCapture|boothy-photo-transferred|boothy-new-photo|boothy-camera-error" apps/boothy/src/App.tsx -S
rg -n "boothy_trigger_capture|boothy_handle_photo_transferred|boothy_get_active_session" apps/boothy/src-tauri/src/main.rs -S
rg -n "capture_with_session_destination|send_request_with_options|event.camera.photoTransferred" apps/boothy/src-tauri/src/camera/ipc_client.rs -S
rg -n "HandleCaptureAsync|SetSessionDestination|camera.capture" apps/camera-sidecar/Program.cs -S
rg -n "CaptureAsync|HandleObjectEvent|HandleStateEvent|SetSessionDestination|EdsSendCommand|EdsDownload|EdsDownloadComplete" apps/camera-sidecar/Camera/RealCameraController.cs -S
rg -n "handle_photo_transferred|File stabilization timeout|File still locked after stabilization|boothy-new-photo" apps/boothy/src-tauri/src/ingest/file_watcher.rs -S
```

## 5. active session 경로 확인

이 이슈에서 매우 자주 놓치는 것이 `파일은 생성됐는데 다른 세션 Raw/에 생성된 경우`다.

확인 포인트:
1. `apps/boothy/src-tauri/src/main.rs`의 `boothy_get_active_session()`가 현재 세션을 돌려준다.
2. `apps/boothy/src-tauri/src/session/models.rs`에서 세션 `raw_path`는 항상 `<base_path>\\Raw`다.
3. sidecar 로그의 `Capture request applying session destination: ...`
4. sidecar 로그의 `Photo transferred: ... -> <finalPath>`

판단 규칙:
1. `<finalPath>`가 active session `raw_path`로 시작하지 않으면 인제스트 문제가 아니라 세션 경로 불일치다.
2. `Capture request applying session destination` 로그가 없다면 destination payload 전달이 끊긴 것이다.
3. `SetSessionDestination`는 sidecar와 camera controller 둘 다 상태를 가진다. restart가 끼면 날아갈 수 있다.

## 6. 어떤 파일부터 읽을지

### 1순위

1. `apps/camera-sidecar/Camera/RealCameraController.cs`
- `CaptureAsync()`
- `HandleObjectEvent()`
- `HandleStateEvent()`
- 여기서 실제 Raw 파일 write가 이뤄진다.

2. `apps/boothy/src-tauri/src/camera/ipc_client.rs`
- `send_request_with_options()`
- `set_session_destination()`
- `capture_with_session_destination()`
- `event.camera.photoTransferred` 처리

3. `apps/camera-sidecar/Program.cs`
- `HandleSetSessionDestinationAsync()`
- `HandleCaptureAsync()`

4. 최신 `boothy-*.log`, `boothy-sidecar-*.log`

### 2순위

1. `apps/boothy/src-tauri/src/ingest/file_watcher.rs`
2. `apps/boothy/src-tauri/src/ingest/stabilizer.rs`
3. `apps/boothy/src/App.tsx`

### 3순위

1. `apps/camera-sidecar/IPC/NamedPipeServer.cs`
2. `apps/boothy/src-tauri/src/camera/ipc_models.rs`
3. `apps/camera-sidecar/IPC/IpcMessage.cs`

## 7. 낭비를 막는 규칙

1. `App.tsx` 전체를 읽지 말 것.
- `rg -n`으로 위치를 찾고 필요한 라인 범위만 읽는다.

2. 로그에서 이벤트 이름을 찾지 말고 실제 로그 문자열을 찾을 것.
- 예: `boothy-new-photo`보다 `File stable, triggering import:`

3. `Raw` 폴더가 비어 있으면 프론트 상태 메시지부터 고치지 말 것.

4. sidecar 로그에 `Received Request: camera.capture`가 없으면 Canon SDK를 파지 말 것.

5. `apps/boothy/src-tauri/src/watcher/file_watcher.rs`를 먼저 보지 말 것.
- transfer-complete 경로의 핵심은 `apps/boothy/src-tauri/src/ingest/file_watcher.rs`다.

6. 한 번에 하나의 가설만 검증할 것.
- IPC 요청 유실인지
- Canon transfer 실패인지
- stabilization 실패인지
- UI refresh 실패인지

## 8. 히스토리 기반 알려진 패턴

### 패턴 A. `camera.getStatus` write timeout 이후 recovery가 destination 상태를 날림

증상:
1. `camera.setSessionDestination`는 성공한 것처럼 보인다.
2. 직후 `camera.capture`에서 `Session destination not set` 또는 `success=false`가 나온다.
3. `photoTransferred`는 시작되지 않는다.

핵심 해석:
1. capture 직전 sidecar restart가 끼면 `sessionDestination` 상태가 사라질 수 있다.
2. 이 경우 캡처 실패처럼 보여도 실제 원인은 capture 이전의 sidecar 상태 복구 방식이다.

집중 파일:
1. `apps/boothy/src-tauri/src/camera/ipc_client.rs`
2. `apps/camera-sidecar/Program.cs`
3. `apps/camera-sidecar/Camera/RealCameraController.cs`

### 패턴 B. `camera.capture` write timeout인데 sidecar는 요청을 받은 적이 없음

증상:
1. Boothy 로그에 `IPC pipe write timeout during camera.capture`
2. 같은 시각 sidecar 로그에 `Received Request: camera.capture`가 없음

핵심 해석:
1. Canon SDK 문제가 아니라 Pipe writer / sidecar connection / restart race 문제다.
2. 이 경우 `RealCameraController`를 깊게 보기 전에 `ipc_client.rs`, `NamedPipeServer.cs`부터 본다.

### 패턴 C. sidecar `Photo transferred:`까지는 나오는데 라이브러리에는 안 뜸

증상:
1. sidecar 최종 파일 경로가 찍힌다.
2. Boothy에서 `Photo transferred notification received:` 이후 `File stabilization timeout:` 또는 `File still locked after stabilization:`가 나온다.

핵심 해석:
1. Canon transfer는 끝났다.
2. stabilization timeout 기본값은 현재 10초다.
3. 이 경우 `apps/boothy/src-tauri/src/ingest/stabilizer.rs`와 파일 lock 원인을 본다.

## 9. 수정 후 최소 검증

1. 버튼 촬영 1회 실행
2. sidecar 로그에 아래 두 줄 확인
- `Capture requested`
- `Photo transferred: ... -> ...\\Raw\\...`

3. Boothy 로그에 아래 두 줄 확인
- `Photo transferred notification received: ...`
- `File stable, triggering import: ...`

4. 실제 active session `Raw/` 폴더에 파일 존재 확인

```powershell
Get-ChildItem 'C:\\Users\\KimYS\\Pictures\\<session>\\Raw' -File |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 10 FullName, Length, LastWriteTime
```

5. UI에서 최신 사진이 선택되거나 최소한 목록에 추가되는지 확인

6. 외부 셔터도 범위에 포함되면 별도로 1회 검증
- `camera.capture` 로그가 없어도 `Photo transferred:`부터는 동일해야 한다.

## 10. 세션 기록 템플릿

다음 세션이 이어받기 쉽게 아래 형식으로 기록한다.

1. 증상:
2. 재현 절차:
3. active session raw_path:
4. Boothy 로그 핵심 3줄:
5. sidecar 로그 핵심 3줄:
6. 실패 경계:
7. 근본 원인 가설:
8. 수정 파일:
9. 검증 명령:
10. 남은 리스크:

## 11. 결론

이 문제군의 핵심은 `capture succeeded`와 `Raw 파일 도착`을 절대 같은 의미로 취급하지 않는 것이다.

다음 세션은 아래 순서만 지키면 된다.
1. sidecar가 `camera.capture`를 받았는지 본다.
2. sidecar가 `Photo transferred:`를 찍었는지 본다.
3. Boothy가 `Photo transferred notification received:`와 `File stable, triggering import:`를 찍었는지 본다.
4. 그 다음에야 UI를 본다.
