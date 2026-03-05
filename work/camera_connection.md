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
