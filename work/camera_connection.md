# Boothy 카메라 커넥션 이슈 대응 가이드 (새 세션용)

이 문서는 새 세션에서 LLM이 카메라 연결/램프/UI 메시지 문제를 빠르게 분석하고 수정하기 위한 실전 가이드입니다.
목표는 다음 3가지입니다.

1. 필요한 파일만 읽고 원인에 도달하기
2. 로그 증거 기반으로 수정하기
3. 검증 명령까지 실행하고 결과를 남기기

---

## 1. 문제 유형 정의

### 자주 발생하는 증상
1. 라이브러리 화면에서 `촬영을 준비 중입니다...` 메시지가 고정됨
2. 카메라 램프가 빨강으로 유지됨
3. 카메라 전원 토글 시 초록으로 잠깐 바뀐 뒤 다시 빨강
4. 연결 복구가 되지 않고 재시작 루프가 반복됨

### 실제 원인 레이어
1. 프론트 상태 계산 문제 (메시지/램프 조건)
2. Rust IPC 요청 타이밍/동시성 문제
3. Sidecar Named Pipe 연결 문제
4. 카메라 하드웨어/SDK 초기화 지연

핵심: 증상은 UI로 보이지만, 원인은 IPC/파이프 경합인 경우가 매우 많습니다.

---

## 2. 새 세션 시작 시 권장 분석 순서 (토큰 절약 핵심)

## Step A. 로그 먼저
코드보다 로그를 먼저 봐야 원인을 빠르게 좁힐 수 있습니다.

```powershell
Get-ChildItem C:\Users\KimYS\AppData\Roaming\Boothy\logs |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 10 | Format-Table LastWriteTime, Length, Name -AutoSize
```

최신 파일 2개를 고릅니다.
1. `boothy-YYYYMMDD.log`
2. `boothy-sidecar-YYYYMMDD.log`

그리고 키워드 검색:

```powershell
rg -n "camera-status-refresh|boothy_camera_get_status|camera.getStatus|IPC pipe write timeout|Failed to start camera sidecar|os error 231|statusChanged|statusHint|cameraDetected" C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260305.log -S

rg -n "Pipe server error|Named Pipe|boothy_camera_sidecar|WaitForConnection|GetStatus|no camera|session open succeeded|Shutdown" C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-20260305.log -S
```

### 로그에서 먼저 보는 판단 포인트
1. `IPC pipe write timeout during camera.getStatus` 존재 여부
2. `Failed to connect ... os error 231` 존재 여부
3. Sidecar의 `모든 파이프 인스턴스가 사용 중입니다` 반복 여부
4. `cameraDetected`가 `true -> null/false`로 반복 흔들리는지

---

## Step B. 프론트는 App.tsx 전체가 아니라 "카메라 관련 구간만"

```powershell
rg -n "refreshCameraStatus|boothy-camera-status-hint|boothy-camera-status|isCameraReady|customerCameraStatusMessage|nextCustomerCameraLampConnectionState|cameraStatusRecoveryPoll" C:\Code\Project\Boothy\apps\boothy\src\App.tsx -S
```

이후 `Get-Content`는 라인 범위로만 읽습니다.

### 꼭 보는 포인트
1. `refreshCameraStatus`의 in-flight 제어
2. `boothy-camera-status-hint` debounce
3. recovery polling(250ms/1000ms) 조건
4. `isCameraReady` 판정식
5. `customerCameraStatusMessage` 계산 인자

---

## Step C. 백엔드 IPC 경로 확인

```powershell
rg -n "send_request_with_options|start_status_monitor|start_sidecar|connect_to_pipe_with_retries|IPC_WRITE_TIMEOUT|stop_sidecar_for_restart|camera.getStatus" C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs -S
```

### 꼭 보는 포인트
1. 요청 직렬화/동시 호출 차단이 있는지
2. write timeout 시 어떤 재시작 경로를 타는지
3. 상태 모니터(getStatus poll)가 프론트 요청과 경합하는지
4. 연결 실패 시 상태(disconnected/reconnecting) 전환이 일관적인지

---

## Step D. Sidecar 파이프 인스턴스 정책 확인

```powershell
rg -n "NamedPipeServerStreamAcl.Create|maxNumberOfServerInstances|ListenAsync" C:\Code\Project\Boothy\apps\camera-sidecar\IPC\NamedPipeServer.cs -S
```

### 꼭 보는 포인트
1. `maxNumberOfServerInstances` 값
2. reconnect race에서 `ERROR_PIPE_BUSY(231)`를 증폭시키는 구조인지
3. 예외 발생 후 backoff/재시도 방식

---

## 3. 파일 우선순위 맵 (읽는 순서)

### 1순위 (원인 파악 핵심)
1. `apps/boothy/src-tauri/src/camera/ipc_client.rs`
2. `apps/camera-sidecar/IPC/NamedPipeServer.cs`
3. `%APPDATA%\Boothy\logs\boothy-*.log`
4. `%APPDATA%\Boothy\logs\boothy-sidecar-*.log`

### 2순위 (UI 증상 확정)
1. `apps/boothy/src/App.tsx`
2. `apps/boothy/src/camera/customerCameraLamp.ts`
3. `apps/boothy/src/camera/customerCameraStatusMessage.ts`

### 3순위 (스키마/에러 코드 매핑)
1. `apps/boothy/src-tauri/src/camera/ipc_models.rs`
2. `apps/camera-sidecar/IPC/IpcMessage.cs`

---

## 4. 증상 -> 로그 패턴 -> 원인 가설 매핑

## 케이스 A
- 증상: 초록불 잠깐 후 빨강 유지 + 준비 메시지 고정
- 로그:
1. `camera-status-refresh-success`에서 `cameraDetected=true`가 보임
2. 직후 `IPC pipe write timeout during camera.getStatus`
3. 이어서 sidecar restart 반복
- 원인 가설 우선순위:
1. getStatus 중첩 호출 경합
2. write timeout 기준이 너무 공격적
3. 파이프 재연결 경합

## 케이스 B
- 증상: 앱 시작 시 빨강에서 오래 못 벗어남
- 로그:
1. `GetStatus: no camera reported` 반복
2. 늦게 `session open succeeded -> detected`
- 원인 가설 우선순위:
1. SDK 초기화/세션 오픈 지연
2. snapshot TTL과 pull polling 타이밍 불일치

## 케이스 C
- 증상: 재연결 시도할수록 더 악화
- 로그:
1. `Failed to connect ... os error 231`
2. sidecar `Pipe server error` 반복
- 원인 가설 우선순위:
1. 파이프 인스턴스 경합
2. 재시작 루프(자기증폭)

---

## 5. 토큰 절약 규칙 (강력 권장)

1. `App.tsx` 전체 읽지 말 것
2. 로그 파일 전체 출력하지 말 것
3. `rg -n`으로 위치 찾고 라인 범위만 보기
4. 한 번에 하나의 가설만 검증하기
5. 변경 전/후 로그 문자열로 효과 비교하기

### 나쁜 접근
- UI 이상이니까 UI부터 대규모 수정
- timeout/pipe 로그를 보지 않고 debounce만 조정

### 좋은 접근
- 로그 5분 -> 원인 축소 -> 파일 2~3개만 정밀 수정

---

## 6. 수정 작업 플레이북

## 플레이북 1: UI 메시지/램프만 수정
수정 파일:
1. `src/camera/customerCameraLamp.ts`
2. `src/camera/customerCameraStatusMessage.ts`
3. 필요 시 `src/App.tsx` 일부

검증:
```powershell
npm run test -- src/camera/__tests__/customerCameraLamp.test.ts src/camera/__tests__/customerCameraStatusMessage.test.ts
```

주의:
- 백엔드 timeout/재시작 로그가 존재하면 UI만 고쳐도 재발 가능

## 플레이북 2: 커넥션 안정성 수정
수정 파일:
1. `src-tauri/src/camera/ipc_client.rs`
2. `apps/camera-sidecar/IPC/NamedPipeServer.cs`

검증:
```powershell
dotnet build C:\Code\Project\Boothy\apps\camera-sidecar\Boothy.CameraSidecar.csproj -c Release
```

추가 확인:
- 수정 후 같은 재현 시나리오에서 `os error 231`, `IPC pipe write timeout` 빈도가 줄었는지

---

## 7. 재현 시나리오 표준화

새 세션에서 항상 동일한 시나리오로 재현해야 비교가 됩니다.

### 시나리오 S1
1. 카메라 전원 ON
2. 앱 실행
3. 라이브러리 화면 진입
4. 램프/메시지 상태 확인 (초기 10초)

### 시나리오 S2
1. 빨강 상태에서 카메라 OFF -> ON
2. 5~10초 대기
3. 램프가 초록으로 안정되는지 확인

### 시나리오 S3
1. 연결 중 화면 전환/새로고침 유사 이벤트
2. 재연결 루프 여부 확인

각 시나리오마다 아래 3개를 기록:
1. 사용자 체감 결과
2. boothy 로그 핵심 라인
3. sidecar 로그 핵심 라인

---

## 8. 새 세션용 입력 프롬프트 템플릿

```text
카메라 커넥션/램프 문제 분석.
반드시 아래 순서로 진행:
1) 최신 boothy/boothy-sidecar 로그에서 getStatus timeout, os error 231, statusChanged/statusHint 패턴 추출
2) 결과에 따라 아래 파일만 부분 읽기
   - apps/boothy/src-tauri/src/camera/ipc_client.rs
   - apps/camera-sidecar/IPC/NamedPipeServer.cs
   - apps/boothy/src/App.tsx (camera 관련 라인만)
3) 로그 증거 기반으로 단일 근본 원인 제시
4) 최소 수정 적용
5) 빌드/테스트 실행 결과 보고
전체 파일 dump 금지, rg -n + line range 방식으로만 진행
```

---

## 9. 완료 보고 템플릿 (세션 기록 표준)

아래 형식으로 남기면 다음 세션에서 컨텍스트 재구성이 매우 빨라집니다.

1. 증상:
2. 로그 증거 3개:
3. 근본 원인:
4. 수정 파일/핵심 변경:
5. 검증 명령/결과:
6. 남은 리스크:

예시:
1. 증상: 준비 메시지 고정, 빨간 램프 유지
2. 증거:
   - `IPC pipe write timeout during camera.getStatus`
   - `Failed to connect ... os error 231`
   - sidecar `Pipe server error`
3. 원인: getStatus 동시성 경합 + 재시작 루프
4. 수정: ipc_client 요청 직렬화, NamedPipe 인스턴스 정책 보완
5. 검증: sidecar build 성공, camera UI 테스트 통과
6. 리스크: 실제 카메라 모델별 초기화 지연 편차

---

## 10. 금지 체크리스트

아래 항목이 하나라도 있으면 분석 품질이 급격히 떨어집니다.

1. 로그 확인 없이 코드 수정 시작
2. 대형 파일 전체 읽기
3. 증거 없이 "고쳐졌을 것" 판단
4. 검증 명령 미실행 상태에서 완료 선언

---

## 11. 참고 경로 요약

1. 프론트:
- `C:\Code\Project\Boothy\apps\boothy\src\App.tsx`
- `C:\Code\Project\Boothy\apps\boothy\src\camera\customerCameraLamp.ts`
- `C:\Code\Project\Boothy\apps\boothy\src\camera\customerCameraStatusMessage.ts`

2. Rust 백엔드:
- `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`
- `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_models.rs`

3. Sidecar:
- `C:\Code\Project\Boothy\apps\camera-sidecar\IPC\NamedPipeServer.cs`
- `C:\Code\Project\Boothy\apps\camera-sidecar\IPC\IpcMessage.cs`

4. 런타임 로그:
- `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-YYYYMMDD.log`
- `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-YYYYMMDD.log`

---

이 문서를 새 세션 시작 시 먼저 읽고, 반드시 로그 기반으로 원인부터 확정한 다음 수정하세요.

---

## 12. 2026-03-06 세션 기록: 자동 에디터 이동 버그

### 증상
1. 앱 실행 후 `Start Session` 클릭
2. 바로 라이브러리 화면으로 이동
3. 몇 초 뒤 카메라 상태를 다시 확인하는 것처럼 보임
4. 갑자기 사용자가 건드리지 않았는데 에디터 화면으로 자동 이동

### 로그 증거
1. `2026-03-05 22:44:31.866` `Session opened - library constrained to Raw/`
2. 직후 `2026-03-05 22:44:33.364` `IPC pipe write timeout during camera.getStatus - skipping sidecar restart for polling request`
3. 이어서 `2026-03-05 22:44:38.438` `camera-status-refresh-success | {"cameraDetected":null,"connected":null,"hasLastError":true,"ipcState":"reconnecting"}`

핵심: 로그만 보면 카메라 재체크가 원인처럼 보이지만, 실제 자동 화면 전환 트리거는 UI 상태 플래그였다.

### 근본 원인
1. `src/App.tsx`의 `applyBoothySession()`가 세션 적용 시마다 무조건 `setShouldAutoOpenEditor(true)`와 `setActiveView('editor')`를 실행했다.
2. 같은 세션이 `invoke(boothy_create_or_open_session)` 반환값과 `boothy-session-changed` 이벤트로 다시 적용될 수 있었다.
3. 사용자가 라이브러리로 돌아가도 `handleBackToLibrary()`가 `shouldAutoOpenEditor`를 끄지 않았다.
4. 이후 이미지 목록이 늦게 채워지면 `shouldAutoOpenEditor`를 감시하는 `useEffect`가 뒤늦게 `handleImageSelect()`를 호출해 에디터를 열었다.

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src\App.tsx`
2. `C:\Code\Project\Boothy\apps\boothy\src\sessionViewState.ts`
3. `C:\Code\Project\Boothy\apps\boothy\src\__tests__\sessionViewState.test.ts`

### 핵심 변경
1. 중복 세션 재적용 시 에디터 자동 진입을 다시 걸지 않도록 분기
2. 사용자가 라이브러리로 수동 복귀하면 pending auto-open을 즉시 취소
3. 세션 뷰 전환 의사결정을 `sessionViewState.ts`로 분리

### 회귀 테스트
1. 중복 세션 이벤트일 때 에디터를 다시 열지 않는지 확인
2. 라이브러리 복귀 시 auto-open이 취소되는지 확인

### 검증
```powershell
npm run test -- src/__tests__/sessionViewState.test.ts
npm run test
npm run build
```

### 새 세션에서 바로 볼 포인트
1. `src/App.tsx`의 `applyBoothySession`
2. `src/App.tsx`의 `handleBackToLibrary`
3. `src/App.tsx`의 `shouldAutoOpenEditor` 관련 `useEffect`
4. `src/sessionViewState.ts`

---

## 13. 2026-03-06 세션 기록: 초록불인데 촬영 버튼 비활성 버그

### 증상
1. 라이브러리 상단 램프는 초록불 유지
2. 고객 모드의 촬영 버튼은 계속 비활성
3. 사용자는 `카메라가 계속 준비 상태인가?`로 인식

### 로그 증거
1. `2026-03-06 05:44:51.810` `camera-status-refresh-success | {"cameraDetected":true,"connected":true,"hasLastError":false,"ipcState":"connected"}`
2. sidecar 로그 `2026-03-05T20:44:51.782Z` `GetStatus: session open succeeded -> detected model=Canon EOS 700D`
3. 그런데 세션 시작 직후 `2026-03-06 05:44:57.989` `IPC pipe write timeout during camera.setSessionDestination - restarting sidecar`

핵심: `camera.getStatus`는 성공했으므로 카메라가 계속 준비 중이었던 것은 아니다. 버튼 비활성의 직접 원인은 UI readiness 판정 불일치였다.

### 근본 원인
1. `src/App.tsx`의 촬영 버튼 readiness(`isCameraReady`)는 `CAMERA_SNAPSHOT_TTL_MS = 500` 안의 fresh snapshot을 강하게 우선했다.
2. stale snapshot이 메모리에 남아 있으면 `camera.getStatus` pull 결과(`connected=true`, `cameraDetected=true`)로 fallback하지 못했다.
3. 반면 고객용 램프는 pull 상태 기반으로 초록불을 유지할 수 있었다.
4. 결과적으로 `램프는 초록`, `버튼은 disabled`라는 모순 상태가 발생했다.

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src\App.tsx`
2. `C:\Code\Project\Boothy\apps\boothy\src\cameraReadiness.ts`
3. `C:\Code\Project\Boothy\apps\boothy\src\__tests__\cameraReadiness.test.ts`

### 핵심 변경
1. fresh snapshot이 있으면 그것을 우선 사용
2. fresh snapshot이 없거나 stale이면 `camera.getStatus` pull 결과로 fallback
3. 촬영 버튼의 readiness 계산을 `cameraReadiness.ts`로 분리

### 회귀 테스트
1. fresh ready snapshot이 있으면 capture 가능
2. stale snapshot이어도 pull status가 `connected + cameraDetected`면 capture 가능
3. 최신 fresh snapshot이 `noCamera`면 capture 불가

### 검증
```powershell
npm run test -- src/__tests__/cameraReadiness.test.ts
npm run test
npm run build
```

### 중요 해석
아래 현상은 서로 다르다.
1. `램프 초록 + 버튼 비활성`: 대개 프론트 readiness 계산 불일치 가능성
2. `램프 빨강/노랑 + 버튼 비활성`: 실제 카메라/IPC 준비 상태 문제 가능성
3. `버튼 활성 + 첫 촬영 실패`: `camera.setSessionDestination` 또는 capture backend 문제 가능성

---

## 14. 현재 남아 있는 오픈 리스크

이전 세션에서 오픈 리스크였던 `camera.setSessionDestination` 초기 timeout은 2026-03-06 후속 수정으로 완화했다.

### 해결된 리스크 R1
1. 세션 시작 직후 `camera.setSessionDestination`가 pipe write 단계에서 타임아웃 날 수 있었음
2. 로그 예시:
   - `2026-03-06 05:44:52.981` `Preparing to send request: camera.setSessionDestination`
   - `2026-03-06 05:44:57.989` `IPC pipe write timeout during camera.setSessionDestination - restarting sidecar`
   - `2026-03-06 05:44:57.990` `Failed to set camera session destination: IPC pipe write timeout during camera.setSessionDestination`
3. 현재는 `src-tauri/src/camera/ipc_client.rs`의 `set_session_destination()`가 transient pipe 에러에 한해 제한적 재시도를 수행한다

### 다음 세션에서 이 증상이 나오면 우선 읽을 파일
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\main.rs`
2. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`
3. `C:\Code\Project\Boothy\apps\camera-sidecar\Program.cs`
4. `C:\Code\Project\Boothy\apps\camera-sidecar\Camera\RealCameraController.cs`

### 분석 포인트
1. `boothy_create_or_open_session` 직후 `set_session_destination` 비동기 호출이 polling/getStatus와 경합하는지
2. `boothy_trigger_capture` 직전 재호출되는 `set_session_destination`가 성공하는지
3. sidecar에서 `sessionDestination`이 실제로 세팅되었는지
4. sidecar restart 이후 destination 유실이 반복되는지

---

## 15. 새 세션에서 추가로 읽어야 할 프론트 파일

기존 1순위/2순위 목록에 아래 파일도 추가한다.

1. `C:\Code\Project\Boothy\apps\boothy\src\sessionViewState.ts`
2. `C:\Code\Project\Boothy\apps\boothy\src\cameraReadiness.ts`
3. `C:\Code\Project\Boothy\apps\boothy\src\__tests__\sessionViewState.test.ts`
4. `C:\Code\Project\Boothy\apps\boothy\src\__tests__\cameraReadiness.test.ts`

이 파일들은 각각 다음을 설명한다.
1. 세션 적용 시 자동 에디터 진입 정책
2. 촬영 버튼 readiness 계산 정책
3. 중복 세션/라이브러리 복귀 회귀 테스트
4. stale snapshot fallback 회귀 테스트

---

## 16. 새 세션용 빠른 진단 규칙

### 규칙 A
초록불인데 버튼이 꺼져 있으면:
1. `camera.getStatus` 성공 로그가 있는지 먼저 확인
2. 그 다음 `src/App.tsx`의 `isCameraReadyForCapture` 사용 구간 확인
3. snapshot stale/fresh 여부와 pull fallback 여부를 같이 확인

### 규칙 B
라이브러리에서 갑자기 에디터로 튀면:
1. `boothy-session-changed` 중복 적용 여부 확인
2. `shouldAutoOpenEditor`가 남아 있었는지 확인
3. `handleBackToLibrary`가 auto-open을 취소하는지 확인

### 규칙 C
버튼은 켜졌는데 첫 촬영만 실패하면:
1. UI가 아니라 backend destination 경로 문제로 간주
2. `camera.setSessionDestination` 타임아웃부터 확인
3. sidecar 로그에 `Received Request: camera.setSessionDestination`가 아예 없으면 pipe write 단계에서 막힌 것

---

## 17. 2026-03-06 세션 기록: 버튼 활성인데 촬영 무반응 + Failed to configure camera

### 증상
1. 고객 모드에서 촬영 버튼은 활성화되어 있음
2. 버튼을 눌러도 촬영이 시작되지 않음
3. 상단 빨간 토스트로 `Failed to configure camera.`가 표시됨
4. 같은 시점에 상태 램프는 초록으로 유지될 수 있음

### 로그 증거
1. `2026-03-06 06:00:51.149` `Preparing to send request: camera.setSessionDestination`
2. `2026-03-06 06:00:56.157` `IPC pipe write timeout during camera.setSessionDestination - restarting sidecar`
3. `2026-03-06 06:00:56.157` `Failed to set camera session destination: IPC pipe write timeout during camera.setSessionDestination`
4. 캡처 버튼 클릭 후에도 `2026-03-06 06:01:04.220` 동일 timeout이 재발
5. sidecar 로그에는 같은 시점의 `Received Request: camera.setSessionDestination` 또는 `Setting session destination:`가 없음

핵심: 요청은 sidecar 핸들러까지 도달하지 못했고, named pipe write 단계에서 먼저 실패했다.

### 근본 원인
1. `boothy_create_or_open_session`와 `boothy_trigger_capture` 둘 다 촬영 전 `set_session_destination()`를 호출한다.
2. 이 호출이 transient pipe 불안정 구간에서 한 번 실패하면 즉시 `Failed to configure camera.`로 종료되었다.
3. `camera.getStatus`는 성공해도, `camera.setSessionDestination`는 별도 요청이라 초기 연결 직후 write timeout의 영향을 별도로 받을 수 있었다.
4. 따라서 `초록불`은 카메라 상태 성공을 의미했지만, `촬영 무반응`은 destination 설정 실패 때문에 발생했다.

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`

### 핵심 변경
1. `should_retry_set_session_destination_error()` 추가
2. 아래 transient 에러에 한해 `set_session_destination()`가 제한적으로 재시도
   - `IPC pipe write timeout during camera.setSessionDestination`
   - `Sidecar not connected`
   - `Pipe not available`
   - `Pipe writer ack channel closed`
   - `IPC response channel closed`
3. 재시도 간 500ms, 1000ms backoff 적용
4. 재시도 전에 연결이 끊긴 상태면 `start_sidecar()`를 best-effort로 다시 호출
5. non-transient 에러는 재시도하지 않고 즉시 실패 유지

### 회귀 테스트
1. transient pipe 에러는 retry 대상임을 확인
2. serialization/capture 같은 non-transient 에러는 retry하지 않음을 확인

### 검증
```powershell
cargo test session_destination_retries_on_transient_pipe_errors --manifest-path src-tauri/Cargo.toml
cargo test session_destination_does_not_retry_on_non_transient_errors --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### 남은 리스크
1. 이 수정은 transient pipe write 실패 완화용이다
2. sidecar 내부에서 실제 `SetSessionDestination` 처리 자체가 오래 걸리거나 SDK 호출에서 멈추는 경우는 별도 이슈다
3. 만약 재현 시 sidecar 로그에 `Received Request: camera.setSessionDestination`가 찍힌 뒤 실패한다면, 다음 분석 대상은 `Program.cs`와 `RealCameraController.cs`다

---

## 18. 2026-03-06 세션 기록: 초기 촬영 버튼 깜빡임 + 클릭 무반응 + 반복 `Failed to configure camera`

### 증상
1. 세션 시작 직후 고객 모드 `촬영` 버튼이 잠깐 비활성/활성을 반복
2. 버튼을 눌러도 체감상 아무 반응이 없음
3. 상단 빨간 토스트 `Failed to configure camera.`가 반복됨
4. 로그상 짧은 시간 안에 여러 `camera.setSessionDestination` correlation id가 겹쳐 나타남

### 로그 증거
1. `2026-03-06 06:14:04.349` `Session opened - library constrained to Raw/`
2. 직후 `2026-03-06 06:14:06.535` `IPC pipe write timeout during camera.getStatus - skipping sidecar restart for polling request`
3. 이어서 `2026-03-06 06:14:06.536` `Preparing to send request: camera.setSessionDestination`
4. `2026-03-06 06:14:24.808` `Failed to set camera session destination: IPC pipe write timeout during camera.setSessionDestination`
5. 같은 구간에 여러 correlation id가 `camera.setSessionDestination failed on attempt 1/2`를 남김
6. sidecar 로그에는 같은 시점 `Received Request: camera.setSessionDestination`가 거의 없고, 대부분 `camera.getStatus`만 보임

핵심: `setSessionDestination`가 sidecar 처리까지 못 가는 경우가 있었고, 동시에 UI가 capture in-flight를 막지 않아 실패가 폭증했다.

### 근본 원인
1. 세션 시작 직후 backend가 eager하게 `set_session_destination()`를 호출했다
2. 이 시점은 sidecar bootstrap/getStatus polling과 겹쳐 pipe가 가장 불안정한 구간이었다
3. 실패 시 고객용 토스트와 reconnect 플리커가 바로 발생해 버튼이 초반에 깜빡였다
4. 프론트 `handleTriggerCapture()`는 invoke가 진행 중이어도 재클릭을 막지 않아, 사용자가 반응이 없다고 느끼는 동안 `boothy_trigger_capture`가 중복 호출될 수 있었다
5. retry 판정도 `Failed to write to pipe ... (os error 232)`를 transient로 보지 않아, sidecar 재시작 직후 파이프 종료 중 에러에서 회복하지 못했다

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\main.rs`
2. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`
3. `C:\Code\Project\Boothy\apps\boothy\src\App.tsx`
4. `C:\Code\Project\Boothy\apps\boothy\src\captureStatus.ts`
5. `C:\Code\Project\Boothy\apps\boothy\src\__tests__\captureStatus.test.ts`

### 핵심 변경
1. `boothy_create_or_open_session`에서는 더 이상 eager `set_session_destination()`를 호출하지 않음
2. destination 설정은 실제 capture/reconnect 시점으로 지연
3. `set_session_destination()` retry 대상에 `Failed to write to pipe` / `os error 232` 추가
4. retry 전에 sidecar를 명시적으로 stop/start 하여 stale pipe를 버리고 새 연결로 재시도
5. 프론트에서 `captureStatus`가 `capturing/transferring/stabilizing/importing`이면 버튼 재클릭 차단
6. 버튼 클릭 시 즉시 `capturing` 상태로 전환해 사용자에게 무반응처럼 보이지 않게 함

### 회귀 테스트
1. `session_destination_retries_on_transient_pipe_errors`에 `os error 232` 케이스 추가
2. `isCaptureInFlight()` 테스트 추가로 busy 상태에서 중복 클릭 차단 규칙 고정

### 검증
```powershell
cargo test session_destination_retries_on_transient_pipe_errors --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npx vitest run src/__tests__/captureStatus.test.ts
npm test -- --runInBand
npm run build
```

### 새 세션에서 바로 보는 판단 규칙
1. 세션 시작 직후 `Failed to configure camera.`가 뜨면 먼저 `main.rs`에서 session start 경로가 destination 설정을 다시 하고 있지 않은지 확인
2. `촬영` 클릭 후 correlation id가 여러 개 생기면 프론트 in-flight 차단이 깨진 것
3. `Failed to write to pipe ... (os error 232)`가 보이면 stale pipe 재시작 경로가 살아 있는지 확인

---

## 19. 2026-03-06 세션 기록: 초록 램프 유지인데 촬영 버튼이 다시 비활성

### 증상
1. 육안상 고객 모드 카메라 램프는 초록으로 안정화됨
2. 그런데 `촬영` 버튼은 다시 비활성화됨
3. 로그에는 `camera.getStatus` polling timeout이 반복되지만, sidecar 최초 성공 이후 카메라 감지 자체는 이미 확인됨

### 로그 증거
1. `2026-03-06 06:29:39.838` `camera-status-refresh-success | {"cameraDetected":true,"connected":true,"hasLastError":false,"ipcState":"connected"}`
2. 이후 `2026-03-06 06:29:46.196` `IPC pipe write timeout during camera.getStatus - skipping sidecar restart for polling request`
3. `2026-03-06 06:29:51.267` `camera-status-refresh-success | {"cameraDetected":null,"connected":null,"hasLastError":true,"ipcState":"connected"}`

핵심: soft-fail 이후 `ipcState=connected`는 유지되지만 `status=null` report가 들어오면서, 램프와 버튼이 서로 다른 기준을 쓰게 되었다.

### 근본 원인
1. 고객 램프는 `customerCameraLamp.ts`에서 `status=null` soft-fail report가 와도 이전 초록 상태를 유지한다
2. 반면 촬영 버튼 readiness는 `cameraReadiness.ts`에서 fresh snapshot이 없으면 pull report의 `status.connected && cameraDetected`만 본다
3. soft-fail report는 `status=null`이므로 버튼 readiness가 false로 떨어진다
4. 게다가 `App.tsx`는 `cameraStatusLoading`만으로도 버튼을 비활성화하고 있어, background poll 중에도 버튼이 자주 꺼졌다

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src\cameraReadiness.ts`
2. `C:\Code\Project\Boothy\apps\boothy\src\__tests__\cameraReadiness.test.ts`
3. `C:\Code\Project\Boothy\apps\boothy\src\App.tsx`

### 핵심 변경
1. `ipcState=connected`이고 pull report가 `status=null`인 soft-fail이면 stale snapshot이 `ready`인지 fallback 확인
2. 초록 램프를 유지하는 동안 background `cameraStatusLoading`만으로 버튼을 끄지 않도록 변경
3. 결과적으로 `램프 초록 + 버튼 disabled` 모순 상태를 해소

### 회귀 테스트
1. `soft-fail report(status=null) + stale ready snapshot`이면 capture ready 유지

### 검증
```powershell
npx vitest run src/__tests__/cameraReadiness.test.ts
npm test -- --runInBand
npm run build
```

### 남은 리스크
1. 이 수정은 UI readiness 정합성 수정이다
2. `camera.getStatus` pipe timeout 반복 자체는 backend 안정화 과제로 별도 남아 있다
3. 이후 버튼은 켜졌는데 실제 capture가 실패하면 다시 `setSessionDestination`/pipe recovery 경로를 봐야 한다

---

## 20. 2026-03-06 세션 기록: 촬영 버튼 클릭 후 무반응 + `Failed to configure camera.`

### 증상
1. 고객 모드에서 `촬영` 버튼은 눌릴 수 있음
2. 버튼 클릭 후 실제 카메라는 동작하지 않음
3. 빨간 경고창으로 `Failed to configure camera.`가 표시됨
4. 사용자는 `눌렀지만 아무 반응이 없다`고 체감함

### 로그 증거
1. `2026-03-06 06:39:28.303` `IPC pipe write timeout during camera.getStatus - skipping sidecar restart for polling request`
2. 직후 `2026-03-06 06:39:33.371` `Preparing to send request: camera.setSessionDestination`
3. 이후 `2026-03-06 06:39:38.379` / `06:39:44.775` / `06:39:51.505` 동일한 `IPC pipe write timeout during camera.setSessionDestination - restarting sidecar` 반복
4. 최종적으로 `2026-03-06 06:39:51.505` `Failed to set camera session destination: IPC pipe write timeout during camera.setSessionDestination`
5. 같은 구간의 sidecar 로그에는 `Received Request: camera.setSessionDestination`가 전혀 없음

핵심: `setSessionDestination`는 sidecar 처리기까지 도달하지 못했다. 실제 실패 지점은 SDK나 capture 핸들러가 아니라 named pipe write 단계였다.

### 근본 원인
1. background polling `camera.getStatus`가 pipe write timeout으로 soft-fail 되면, polling 경로는 sidecar restart를 생략한다
2. 이때 diagnostics의 마지막 오류만 남고, 실제 writer/pipe 핸들은 독성 상태로 계속 유지될 수 있었다
3. 직후 들어온 non-polling 요청 `camera.setSessionDestination`는 같은 request lock 아래에서도 그 stale writer를 그대로 재사용했다
4. 결과적으로 `setSessionDestination`는 sidecar에 도달하지 못한 채 write timeout만 반복했고, capture는 시작조차 못 했다

### 수정 파일
1. `C:\Code\Project\Boothy\apps\boothy\src-tauri\src\camera\ipc_client.rs`

### 핵심 변경
1. `should_recover_before_request()` 추가
2. 직전 오류가 `IPC pipe write timeout during camera.getStatus`이고, 다음 요청이 `camera.getStatus`가 아닌 경우 선제적으로 sidecar를 stop/start 하도록 변경
3. 즉 `camera.setSessionDestination`나 `camera.capture`는 polling soft-fail 뒤 stale writer를 재사용하지 않고 새 연결에서 실행되게 함
4. 기존 `set_session_destination()` transient retry 로직은 유지하고, 그보다 한 단계 앞에서 독성 pipe를 버리도록 보강

### 회귀 테스트
1. `non_polling_requests_recover_after_poll_write_timeout`
2. polling `camera.getStatus` write timeout 이후 `camera.setSessionDestination` / `camera.capture`는 복구 대상이고, `camera.getStatus` 자체는 복구 대상이 아님을 고정

### 검증
```powershell
cargo test non_polling_requests_recover_after_poll_write_timeout --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### 새 세션에서 바로 보는 판단 규칙
1. `camera.getStatus - skipping sidecar restart for polling request` 직후 `Failed to configure camera.`가 나오면 stale writer 재사용 여부부터 본다
2. boothy 로그에는 `Preparing to send request: camera.setSessionDestination`가 있는데 sidecar 로그에 `Received Request: camera.setSessionDestination`가 없으면 write 단계에서 막힌 것이다
3. 이 경우 `ipc_client.rs`의 `should_recover_before_request()`와 `send_request_with_options()` 초반 복구 경로가 살아 있는지 확인한다
