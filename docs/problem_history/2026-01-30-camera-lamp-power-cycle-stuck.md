# 카메라 전원 사이클 후 램프(초록/빨강) 상태가 갱신되지 않는 문제 (Power-cycle)

## 요약
- **증상**
  1) 카메라 전원 OFF 상태에서 앱 실행 → 빨강(OFF) 표시 → 카메라 ON → **초록으로 안 바뀜**
  2) 카메라 전원 ON 상태에서 앱 실행 → 초록(ON) 표시 → 카메라 OFF → 빨강 표시 → 카메라 ON → **초록으로 안 바뀜**
- **영향**
  - 고객 모드에서 “카메라 준비됨/연결됨” UX가 신뢰를 잃음
  - 실제 촬영 가능 여부와 UI 표시가 불일치할 수 있음
- **관련 로그 파일**
  - Sidecar: `%APPDATA%\\Boothy\\logs\\boothy-sidecar-YYYYMMDD.log`
  - Boothy(Tauri): `%APPDATA%\\Boothy\\logs\\boothy-YYYYMMDD.log`
  - 예시(이번 이슈에서 확인):  
    - `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-sidecar-20260130.log`
    - `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-20260130.log`

## 환경/구성
- Windows + Canon EDSDK 기반 실카메라 연동
- IPC: Named Pipe (`\\\\.\\pipe\\boothy_camera_sidecar`)
- 앱 구조
  - UI(React): `apps/boothy/src/App.tsx`
  - Tauri(Rust): `apps/boothy/src-tauri/src/camera/*`, `apps/boothy/src-tauri/src/main.rs`
  - Sidecar(.NET): `apps/camera-sidecar/*`

## 로그 해석 시 주의점
- Sidecar 로그는 `...T...Z` 형태로 **UTC** 타임스탬프가 찍힘.
- Boothy 로그는 `YYYY-MM-DD HH:MM:SS` 형태로 **로컬 시간** 기반으로 찍힘.
- 동일 사건을 맞춰볼 때는 **correlation id**(`corr-...`)와 **이벤트명**(`camera.getStatus`, `event.camera.statusHint`)로 묶는 게 가장 정확함.

## 관측된 로그 패턴(핵심)
### 1) OFF 감지는 정상
Sidecar가 Canon state event `Shutdown`을 받고 statusHint를 emit한 뒤, 곧바로 `camera.getStatus`를 처리하며 `cameraDetected=false`로 응답하는 흐름은 정상적으로 관측됨.

예시(boothy-sidecar-20260130.log):
- `Canon state event: Shutdown (camera powered off/disconnected)`
- `Sent Event: event.camera.statusHint`
- `GetStatus: received camera shutdown signal; closing session`
- `GetStatus: probe result detected=False count=0 model=null`

예시(boothy-20260130.log):
- `Camera status hint received`
- `Sent request: camera.getStatus`
- `Received IPC message: camera.getStatus`

### 2) ON 복귀(초록으로 변경) 구간이 로그에 남지 않는 케이스가 존재
한 케이스에서는 “OFF→ON 했는데 변화 없음”이라고 체감되지만, 해당 시점 이후의 `camera.getStatus` 호출/응답 로그가 파일 끝에서 끊기는 경우가 있음.
- 이 경우는 실제로 다음 중 하나일 가능성이 큼:
  - 앱/sidecar가 그 시점에 종료되었음(로그 파일이 더 이상 갱신되지 않음)
  - UI 쪽에서 `camera.getStatus` 폴링이 멈췄음(요청이 in-flight로 고착되거나, UI 갱신 경로가 끊김)

## 가설/원인 후보(RCA)
이 이슈는 “UI 갱신 문제”와 “EDSDK 복구 문제”가 겹칠 수 있어, 두 축으로 분해해 접근함.

### 원인 후보 A) UI가 카메라 상태 변경을 렌더에 반영하지 못함(캐시/의존성 누락)
- Library 화면이 `useMemo`로 캐싱되어 있고, `MainLibrary`에 전달되는 카메라 관련 props가 dependency 배열에 빠지면
  - `cameraStatus`가 바뀌어도 화면이 재렌더링되지 않아 램프가 멈춘 것처럼 보일 수 있음.
- 조치: `apps/boothy/src/App.tsx`의 `memoizedLibraryView` dependency 배열에 카메라 관련 값들을 포함하도록 수정.

### 원인 후보 B) UI 폴링/갱신 루프 자체가 멈춤(in-flight 고착)
- `refreshCameraStatus()`는 `cameraStatusRequestInFlightRef`로 동시 요청을 막는데,
  - 어떤 이유로든 in-flight가 `false`로 되돌아오지 못하면(예: invoke가 걸린 채로 정리되지 않음)
  - 이후 모든 갱신 요청이 큐에만 쌓이고 실제 요청이 더 이상 나가지 않아 램프가 멈춘 것처럼 보일 수 있음.
- 조치: `apps/boothy/src/App.tsx`에 “10초 이상 걸린 in-flight는 stuck으로 간주하고 다음 요청을 허용”하는 watchdog 추가.

### 원인 후보 C) Canon EDSDK 상태 머신이 전원 사이클 후 “복구 불가 상태”로 남음
- Canon state event `Shutdown` 수신 후, SDK 내부 열거/세션 오픈이 불안정해지는 경우가 있음.
- Sidecar가 “카메라 없음(count=0)” 상태로 유지되면 UI는 계속 빨강으로 남을 수 있음.
- 조치(복구 전략 강화):
  - Shutdown/InternalError 발생 시 `sdkResetRequested` 플래그를 세우고,
  - 다음 `camera.getStatus`에서 안전 조건이면 `EdsTerminateSDK → EdsInitializeSDK` 리셋을 수행해 복구를 시도.
  - “no camera(count=0)” 반복에 대한 리셋 트리거/스로틀도 더 공격적으로 조정.

### 원인 후보 D) Sidecar가 EDSDK 네이티브 호출에서 hang → IPC 응답이 timeout/미응답으로 고착
- 전원 사이클/USB 재열거 타이밍에서 Canon SDK가 `EdsGetCameraList/EdsOpenSession/EdsGetEvent` 등에서 멈추는 경우,
  - Sidecar의 `camera.getStatus` 처리 자체가 멈추며,
  - UI는 계속 빨강(또는 stale 상태)로 남게 됨(폴링은 요청하더라도 응답이 없어 timeout/error만 반복되거나, 재연결/재시작이 없으면 계속 동일).
- 조치(프로세스 레벨 복구):
  - **(Tauri/Rust) IPC timeout/채널 종료 시 sidecar를 “unresponsive”로 간주하고 강제 종료 → 다음 요청에서 재시작**
  - **(Sidecar/.NET) `camera.getStatus`가 일정 시간(예: 2.5초) 내 완료되지 않으면 Timeout 응답을 보낸 뒤 프로세스를 종료하여 자동 복구 경로를 열어줌**

## 우리가 시도한 해결(타임라인/변경 요약)
> 아래는 “왜 이 변경을 했는지”를 남기기 위한 기록이며, 최종적으로 어떤 조합이 효과적인지는 재현 테스트 결과로 확정한다.

1) **UI 렌더 캐싱 문제 해결**
- 변경: `apps/boothy/src/App.tsx`의 `memoizedLibraryView` dependency 배열 보강
- 기대: 상태가 변하면 램프가 즉시 재렌더됨

2) **Sidecar 이벤트 펌프 재시작**
- 관측: Shutdown 이후 event pump가 중단되어 hot-plug 이벤트/상태 갱신 경로가 약해질 수 있음
- 변경: `apps/camera-sidecar/Camera/RealCameraController.cs`에서 SDK가 init 상태인데 event pump가 없으면 재시작
- 기대: ON 복귀 시 감지 이벤트/열거 갱신 가능성 상승

3) **Canon EDSDK 강제 리셋(전원 사이클 복구)**
- 변경: `apps/camera-sidecar/Camera/RealCameraController.cs`
  - `sdkResetRequested` 도입(Shutdown/InternalError 시 set)
  - 다음 `camera.getStatus`에서 조건 충족 시 “요청된 리셋” 수행 후 재초기화
  - `no camera(count=0)` 반복 리셋 조건/스로틀 조정
- 기대: OFF→ON 이후에도 일정 시간 내 자동 복구(초록 전환)

4) **UI 폴링 stuck 방지**
- 변경: `apps/boothy/src/App.tsx`
  - `cameraStatusRequestInFlightRef`가 10초 이상 지속되면 stuck으로 간주하고 다음 요청을 허용
- 기대: 특정 invoke hang/레이스 상황에서도 폴링이 영구 중단되지 않음

5) **IPC timeout 시 sidecar 강제 재시작 + getStatus 워치독**
- 변경:  
  - Tauri(Rust) `apps/boothy/src-tauri/src/camera/ipc_client.rs`
    - IPC timeout/응답 채널 종료 발생 시 `stop_sidecar()`로 강제 종료(다음 요청에서 `start_sidecar()`로 복구)
  - Sidecar(.NET) `apps/camera-sidecar/Program.cs`
    - `camera.getStatus`가 2.5초 내 완료되지 않으면 Timeout 응답을 보내고 프로세스를 종료(EDSDK hang 시 “재시작만이 유일한 복구”인 케이스 대응)
- 기대: 전원 사이클 중 SDK가 hang 하더라도 **Boothy 재시작 없이** 카메라 서비스가 자동 재기동되며 램프가 재동기화됨

## 재현/테스트 방법(권장)
### 공통
- 테스트할 때는 **Boothy 앱을 종료했다가 다시 실행**하는 조건도 포함해서 케이스를 분리한다.
- Sidecar exe는 실행 중이면 잠겨서 `dotnet build`가 실패할 수 있음.
  - 빌드 전: Boothy/Sidecar 프로세스 종료 필요

### 2026-01-30 현장 재현 결과(추가 보고)
사용자 보고 기준으로 아래 2가지 시나리오 모두에서 “ON 복귀 시 초록으로 전환되지 않고 빨강에 고정”이 지속됨.

1) 카메라 전원 OFF → 앱 실행(메인 대기 화면) → 카메라 전원 ON → **빨강 유지**
2) 카메라 전원 ON → 앱 실행 → 초록 → 전원 OFF → 빨강 → 전원 ON → **빨강 유지**

### 케이스 1
1) 카메라 전원 OFF
2) Boothy 실행
3) UI에서 빨강 확인
4) 카메라 전원 ON
5) **10~15초 내** 초록으로 전환되는지 확인
6) 전환되지 않으면 로그에서 아래 키워드가 있는지 확인:
   - Sidecar: `performing requested EDSDK reset`, `Canon EDSDK initialized`, `session open succeeded`
   - Boothy: `Sent request: camera.getStatus`가 주기적으로 찍히는지

### 케이스 2
1) 카메라 전원 ON
2) Boothy 실행 → 초록 확인
3) 카메라 전원 OFF → 빨강 확인
4) 카메라 전원 ON
5) **10~15초 내** 초록으로 전환되는지 확인

### 케이스 3 (권장: 자동 복구 경로 검증)
> “전원 사이클 후에도 앱 재시작 없이” 초록으로 복귀하도록 하기 위한 자동 복구가 동작하는지 확인한다.

1) 카메라 전원 OFF → Boothy 실행
2) 10~15초 동안 빨강 유지 확인(정상)
3) 카메라 전원 ON
4) 이후 10~20초 사이에 아래 로그가 1회 이상 발생하는지 확인
   - Boothy(Tauri): `Auto-restarting sidecar`
   - Sidecar: `returning Timeout and terminating sidecar for recovery` (또는 getStatus timeout 관련 로그)
5) 재시작 이후 1~2회 폴링 내 초록으로 복귀하는지 확인

## 후속 디버깅 체크리스트(테스트 실패 시)
1) Boothy 로그에서 `Sent request: camera.getStatus`가 **OFF→ON 이후에도 계속 찍히는지**(폴링 살아있는지)
2) Sidecar 로그에서 `camera.getStatus` 요청 자체가 **OFF→ON 이후에도 들어오는지**
3) Sidecar 로그에서 `sdkResetRequested` 경로가 타는지(리셋 수행/초기화/세션 오픈)
4) 물리 환경 점검(가능성 높은 순)
   - EOS Utility 등 다른 앱이 카메라를 점유하고 있지 않은지
   - 카메라 USB 모드(PTP/PC 연결 모드) / 케이블 / 허브 / 포트
   - 전원 사이클 타이밍(너무 빠르면 드라이버가 재열거 전에 폴링이 먼저 도는 경우가 있음)

## 상태
- **2026-01-30 기준: 증상 지속 보고 반영 → “Sidecar hang/미응답” 복구 경로(D)까지 포함해 수정 적용**  
  - 다음 재현 테스트는 “OFF→ON 이후 `camera.getStatus`가 timeout되는지”와 “timeout 후 sidecar가 재시작되어 초록으로 복귀하는지”를 확인한다.
  - 실패 시에는 `boothy-sidecar-YYYYMMDD.log`에서 `camera.getStatus exceeded` / `terminating sidecar` 로그와, Boothy 로그에서 `IPC timeout` / `restarting sidecar` 로그가 찍히는지 확인한다.

- **2026-01-30 추가: “timeout이 아니어도(응답은 오지만 cameraDetected=false가 지속)” 자동 재시작으로 복구 시도**  
  - 조건: `cameraDetected=false`가 연속으로 지속될 때(이전에는 초록이었다가 빨강으로 내려온 케이스는 더 빠르게), sidecar를 재시작하고 `camera.getStatus`를 즉시 재시도한다.
  - 기대: 전원 사이클 후 Canon SDK가 “카메라 없음”으로 고착되는 케이스에서도 프로세스 재기동으로 복구.

- **2026-01-30 추가: 자동 재시작/워치독 적용 후에도 증상 동일(사용자 재확인)**
  - 재현 시나리오(사용자 보고):
    - OFF → 앱 실행 → ON → 빨강 유지
    - ON → 앱 실행(초록) → OFF(빨강) → ON → 빨강 유지
  - 결론: 현재까지 적용된 (UI 폴링 stuck 방지 / sidecar getStatus timeout 워치독 / IPC timeout 시 sidecar 재시작 / 연속 미감지 시 auto-restart) 조합만으로는 현장 증상을 해결하지 못함.

## 추가 근본 원인 분석 (2026-01-30 로그 심층 분석)

### 원인 후보 E) EDSDK Event Pump가 카메라 재연결 이벤트를 처리하지 못함 ⭐ 핵심 원인

**발견된 사실**:
1. `RealCameraController.cs`에서 `EdsSetCameraAddedHandler`로 camera added 핸들러를 등록하고 있음 (line 1028-1033)
2. `HandleCameraAddedEvent`는 "cameraAdded" statusHint를 emit하도록 구현되어 있음 (line 934-952)
3. **그러나 로그에 "Canon camera added event received" 메시지가 전혀 없음** ← 핵심 관측!

**로그 증거**:
```
# 카메라 OFF 시 (정상 작동):
16:05:20.505 [WARNING] Canon state event: Shutdown (camera powered off/disconnected)
16:05:20.508 [DEBUG] Sent Event: event.camera.statusHint

# 카메라 ON 시 (문제 발생):
# "Canon camera added event received" 로그 없음!
# statusHint 이벤트 발생하지 않음
# 5초 폴링만 작동하지만 감지 실패
```

**근본 원인 분석**:

`RealCameraController.cs:480-535` StartEventPumpUnsafe():
```csharp
while (!token.IsCancellationRequested)
{
    bool shouldPoll = false;
    lock (sdkLock)
    {
        if (!sdkInitialized) break;

        // ⚠️ 문제: session이 없으면 EdsGetEvent()를 호출하지 않음!
        shouldPoll = sessionOpen && cameraRef != IntPtr.Zero && !cameraShutdownSignal;
    }

    if (shouldPoll)
    {
        var err = EdsdkNative.EdsGetEvent();  // ← 이 함수를 호출해야 콜백이 실행됨
        // ...
    }
    await Task.Delay(200, token);
}
```

**왜 이것이 문제인가**:
1. Canon EDSDK의 이벤트 핸들러(`EdsSetCameraAddedHandler` 등)는 **`EdsGetEvent()`를 호출해야 실제 콜백이 트리거됨**
2. 카메라 shutdown 시:
   - session이 닫힘 (`sessionOpen = false`)
   - Event Pump의 `shouldPoll`이 `false`가 됨
   - `EdsGetEvent()`를 더 이상 호출하지 않음
3. 카메라가 다시 켜져도:
   - `cameraAdded` 콜백이 호출되지 않음 (EdsGetEvent를 호출하지 않으므로)
   - statusHint 이벤트가 발생하지 않음
   - 5초 폴링에만 의존하게 됨
4. 5초 폴링은:
   - `ProbeFirstCamera()` 호출하지만 Canon SDK 상태가 이미 "stale"함
   - SDK reset 조건을 만족하지 못하거나, reset 후에도 재초기화가 실패할 수 있음

**해결 방안**:

**방안 E-1: Event Pump가 session 없을 때도 기본 이벤트를 polling하도록 수정**

```csharp
// apps/camera-sidecar/Camera/RealCameraController.cs:506 수정

// 변경 전:
shouldPoll = sessionOpen && cameraRef != IntPtr.Zero && !cameraShutdownSignal;

// 변경 후 (옵션 1 - 안전한 접근):
// SDK가 초기화된 상태면 항상 EdsGetEvent 호출
// (cameraAdded 이벤트를 받기 위해 필수)
shouldPoll = sdkInitialized && !cameraShutdownSignal;

// 변경 후 (옵션 2 - 더 안전한 접근):
// try-catch로 감싸서 session 없을 때 EdsGetEvent가 안전한지 확인
if (sdkInitialized && !cameraShutdownSignal)
{
    try
    {
        var err = EdsdkNative.EdsGetEvent();
        if (err != EdsdkNative.EDS_ERR_OK)
        {
            // session이 없을 때 에러가 발생할 수 있으나
            // cameraAdded 이벤트는 여전히 처리됨
            if (sessionOpen && cameraRef != IntPtr.Zero)
            {
                // session이 있는데 에러 발생 → 심각한 문제
                lock (sdkLock)
                {
                    cameraShutdownSignal = true;
                    pendingTransferCorrelationId = null;
                    CloseCameraSession();
                    StopEventPumpUnsafe();
                }
                Logger.Warning("system", $"EDSDK event pump error (EdsGetEvent=0x{err:X8}); marking camera as shutdown");
            }
        }
    }
    catch (Exception ex)
    {
        Logger.Warning("system", $"EdsGetEvent threw exception: {ex.Message}");
        // 계속 polling은 시도
    }
}
```

**주의사항**:
- Canon EDSDK 버전에 따라 session 없이 `EdsGetEvent()`를 호출하면 크래시할 수 있음
- try-catch로 안전하게 처리하거나, Canon SDK 문서에서 확인 필요
- 또는 `EdsGetEvent()` 대신 명시적으로 camera list를 주기적으로 probe하는 방법도 있음

**방안 E-2: 폴링 기반 카메라 재연결 감지 강화 (더 안전)**

Event Pump 수정이 불안정하다면, GetStatus에서 카메라 재연결을 명시적으로 감지하고 statusHint emit:

```csharp
// apps/camera-sidecar/Camera/RealCameraController.cs
// RealCameraController 클래스에 필드 추가:
private int lastCameraCount = 0;
private DateTime lastCameraCountChangeAt = DateTime.MinValue;

// GetStatus() 메서드 내부에서 (line 260 근처):
var currentCameraCount = probeResult.CameraCount;
if (lastCameraCount == 0 && currentCameraCount > 0)
{
    // 카메라가 0에서 1 이상으로 변경됨 = 재연결 감지!
    var now = DateTime.UtcNow;
    if ((now - lastCameraCountChangeAt).TotalSeconds > 1)
    {
        // 디바운싱: 1초 이내 중복 emit 방지
        lastCameraCountChangeAt = now;
        Logger.Info(correlationId, $"Camera reconnection detected: count {lastCameraCount} -> {currentCameraCount}");
        EmitStatusHint(correlationId, "cameraReconnected");
    }
}
lastCameraCount = currentCameraCount;
```

이 방법은:
- EDSDK Event Pump를 수정하지 않아 안전함
- 5초 폴링이 재연결을 감지하면 즉시 statusHint를 emit
- 프론트엔드에서 statusHint를 받으면 즉시 getStatus 재호출하여 빠른 업데이트

### 원인 후보 F) 프론트엔드 statusHint 이벤트 핸들러 중복 호출

로그에서 shutdown 이벤트 발생 시 **3개의 getStatus 요청이 동시에 발생**:
```
16:05:20.508 - statusHint 이벤트 수신
16:05:20.508 - camera.getStatus 요청 3개 동시 발생:
  corr-1769756689872-...
  corr-1769756703677-...
  corr-1769756718677-...
```

이는 이전 세션 깜빡임 문제와 동일한 중복 호출 문제입니다.

**해결 방안 F-1: statusHint 이벤트 핸들러 디바운싱**

```typescript
// apps/boothy/src/App.tsx
// useEffect 내부에서:

const statusHintDebounceRef = useRef<number | null>(null);

listen('boothy-camera-status-hint', () => {
  if (!isEffectActive) return;

  // 이미 예약된 호출이 있으면 취소
  if (statusHintDebounceRef.current !== null) {
    window.clearTimeout(statusHintDebounceRef.current);
  }

  // 200ms 후에 실행 (짧은 시간 내 여러 이벤트 발생 시 마지막 것만 처리)
  statusHintDebounceRef.current = window.setTimeout(() => {
    statusHintDebounceRef.current = null;
    refreshCameraStatus();
  }, 200);
}),

// cleanup에서:
return () => {
  // ...
  if (statusHintDebounceRef.current !== null) {
    window.clearTimeout(statusHintDebounceRef.current);
  }
};
```

## 권장 해결 순서 (우선순위)

**1단계 (필수, 안전)**:
- ✅ **방안 E-2**: 폴링 기반 카메라 재연결 감지 + statusHint emit
- ✅ **방안 F-1**: statusHint 이벤트 디바운싱

**2단계 (추가 개선)**:
- ⚠️ **방안 E-1**: Event Pump polling 조건 수정 (Canon SDK 버전별 안정성 테스트 필요)

**3단계 (검증)**:
- 재현 테스트로 1단계만으로 문제가 해결되는지 확인
- 해결되지 않으면 2단계 적용 및 추가 로그 분석

---

## 작업 기록 (2026-01-30, 로그 기반 수정 반영)

### 사용자 보고(추가)
- “새로고침을 눌러야 ON→OFF 상태 변환되고 실시간으로 변화되지 않음”
- “OFF→ON은 새로고침을 해도 램프 변화 없음”

### 추가 관측(핵심, 2026-01-30 18:18 KST 로그)
- Boothy가 시작한 sidecar가 **`Mode: mock`** 으로 실행됨.
  - Boothy 로그: `[2026-01-30 18:18:52.xxx] Sidecar path: C:\\Code\\Project\\Boothy\\apps\\camera-sidecar\\bin\\Debug\\net8.0\\Boothy.CameraSidecar.exe`
  - Sidecar 로그: `[2026-01-30T09:18:52.383Z] Mode: mock`
- 이 상태에서는 물리 카메라 전원 ON/OFF가 EDSDK 경로로 전달되지 않으므로 램프가 “실시간”으로 변하지 않는 것이 정상이며,
  “새로고침 시에만 일부 반영/복구”처럼 보이는 현상이 발생할 수 있음(IPC/프로세스 상태만 변하기 때문).

### 추가 원인(근본 원인)
- Sidecar가 **x64로 실행되면** 프로젝트에 번들된 Canon EDSDK(x86)와 아키텍처가 맞지 않아
  `ResolveMode()`가 “EDSDK 존재 여부” 판단에서 실패하고 기본값으로 `mock`을 선택할 수 있음.
  - 이때는 “EDSDK.dll이 파일로 존재하더라도” 아키텍처 검증에서 탈락하면 `mock`으로 떨어짐.

### 결론(원인)
- **카메라 전원 이벤트는 `camera.getStatus` 호출이 없으면 UI까지 전파되지 않는 구간이 존재**함.
  - Sidecar의 Canon 콜백(Shutdown/CameraAdded)은 `EdsGetEvent()` 펌프가 돌 때만 처리됨.
  - 기존 구현은 아래 이유로 **전원 사이클 구간에서 event pump가 멈추거나(또는 폴링이 중단되어) 콜백이 더 이상 올라오지 않을 수 있음**:
    1) `StartEventPumpUnsafe()`가 **세션이 있을 때만** `EdsGetEvent()`를 호출함
    2) `HandleStateEvent(Shutdown/InternalError)`에서 `StopEventPumpUnsafe()`를 호출해 **Shutdown 감지 직후 event pump를 스스로 중지**함
  - 이 상태에서는 UI 폴링/새로고침(= `camera.getStatus` 호출)이 들어오기 전까지 램프가 갱신되지 않는 체감이 발생할 수 있음.

### 적용한 수정(해결)
1) **Sidecar: 전원 사이클 구간에서도 event pump가 살아있도록 보강**
   - 파일: `apps/camera-sidecar/Camera/RealCameraController.cs`
   - 변경 요지:
     - `hotplugWatchActive` 플래그 도입
     - `StartEventPumpUnsafe()`에서 `EdsGetEvent()` 폴링 조건을
       - (기존) “세션이 있을 때만” → (변경) “세션이 있거나, Shutdown/InternalError 이후 hotplug 감시 모드일 때”로 확장
     - `HandleStateEvent(Shutdown/InternalError)`에서 **`StopEventPumpUnsafe()` 제거**
       - 대신 `CloseCameraSession()` + `hotplugWatchActive=true` + `sdkResetRequested=true`로 전환
     - event pump의 `EdsGetEvent()` 에러는 **로그 스팸 방지(2초 스로틀)**만 하고, 펌프 자체는 유지
     - 카메라가 정상 감지(세션 유효/복구 성공)되면 `hotplugWatchActive=false`로 되돌려 불필요한 hotplug 폴링을 최소화

2) **UI(Admin): “IPC 연결”이 아니라 “실제 카메라 준비 상태” 기준으로 점 색상 표시**
   - 파일: `apps/boothy/src/components/panel/CameraControlsPanel.tsx`
   - 변경 요지: 그린 점을 `ipcState=connected`가 아닌 `connected && cameraDetected` 기준으로 표시(로딩/재연결은 노랑)

### 기대 결과(재현 절차)
- 전원 ON 상태에서 앱 실행 후 초록 표시 확인
- 전원 OFF 시, **새로고침 없이** 1~5초 내(상태 이벤트/폴링) 빨강으로 내려가야 함
- 전원 OFF→ON 시에도 `event.camera.statusHint` → `boothy-camera-status-hint` → `camera.getStatus` 흐름으로 **초록으로 복귀**해야 함

### 추가 적용(필수, 2026-01-30)
3) **Sidecar 빌드 타깃을 x86로 고정 (EDSDK x86와 일치)**
   - 파일: `apps/camera-sidecar/Boothy.CameraSidecar.csproj`
   - 변경: `<PlatformTarget>x86</PlatformTarget>`
   - 기대: sidecar가 `mock`으로 떨어지지 않고 `real` 모드로 실행되어, 전원 ON/OFF 이벤트/폴링이 실제 카메라 상태를 반영

4) **개발/현장 PC에 x86 .NET 런타임이 없을 수 있음 → self-contained publish 우선 사용**
   - 관측: x86 sidecar 실행 시 `You must install .NET to run this application. (Architecture: x86)` 오류가 발생할 수 있음(PC에 x86 런타임 미설치).
   - 해결 옵션
     - (권장) x86 sidecar를 `dotnet publish -r win-x86 --self-contained true`로 만들어 번들/실행
     - (대안) .NET Desktop/Runtime x86 설치
   - Boothy dev 경로 선택 개선:
     - `apps/boothy/src-tauri/src/camera/ipc_client.rs`가 `bin/<cfg>/net8.0/win-x86/publish/Boothy.CameraSidecar.exe`를 우선 탐색하도록 변경

---

## 추가 작업 기록 (2026-01-31, Library 램프 오표시)

### 사용자 보고
- “카메라 상태가 ON인데, Library 화면의 램프가 RED로 표시됨”

### 원인
- Library 헤더 램프(`MainLibrary`)가 “카메라 준비 상태”를 판단할 때
  - `cameraStatus`(순수 status) + `isCameraPreparing`만 의존했고,
  - `ipcState=reconnecting` / `camera.getStatus` 로딩과 같은 “전환 상태”를 직접 알 수 없어 **RED로 떨어질 수 있는 경로**가 존재했음.
- 특히 `MainLibrary`가 `cameraStatus`만 받는 구조에서는,
  - IPC 연결 상태가 일시적으로 `reconnecting`인 동안에도 “준비됨/연결중/끊김”을 안정적으로 구분하기 어려워
  - 고객 모드에서 램프 오표시(RED)가 발생할 수 있음.

### 적용한 수정
- `MainLibrary`가 판단에 필요한 원천 정보를 직접 받도록 변경
  - `cameraStatusReport`(ipcState 포함) + `cameraStatusLoading` + `isCameraReconnecting` 기반으로 램프를 계산
  - 결과적으로 `reconnecting/loading`은 **YELLOW**, 정상 연결은 **GREEN**, 그 외는 **RED**로 일관되게 표시
- 파일
  - `apps/boothy/src/components/panel/MainLibrary.tsx`
  - `apps/boothy/src/App.tsx`

---

## 추가 작업 기록 (2026-01-31, 로그 분석 기반: 폴링 미동작으로 램프/이벤트 정지)

### 관측된 증상(사용자 보고)
- 앱 실행 후 카메라 전원을 ON/OFF 해도
  - Sidecar 로그에 변화가 거의 남지 않음
  - Library 화면의 상태 램프가 변하지 않음(RED 고정 등)

### 로그 분석(핵심 근거)
#### 0) Sidecar 모드는 `mock`이 아니라 `real` (이번 케이스)
- Sidecar 로그(UTC): `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-sidecar-20260131.log`
  - `2026-01-31T00:56:18.224Z ... Mode: real`
- 따라서 이번 “램프 고정/로그 미기록” 증상은 **mock 모드 폴백 때문이 아니라**, real 모드에서 **상태 갱신 트리거(폴링/힌트)가 멈춘 문제**로 분류해야 함.

#### 1) `camera.getStatus`가 “딱 1회”만 호출되고 이후 폴링이 없음
- Boothy 로그(로컬 시간): `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-20260131.log`
  - `2026-01-31 09:56:18.400`에 `Sent request: camera.getStatus` 1회 관측
  - 이후 동일 실행 구간에서 주기적(5초) 호출 패턴이 **관측되지 않음**
- Sidecar 로그(UTC): `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-sidecar-20260131.log`
  - `2026-01-31T00:56:18.467Z`에 `Received Request: camera.getStatus` 1회 관측
  - 이후 추가 요청이 없어 카메라 전원 ON/OFF가 일어나도 “상태 재평가” 자체가 수행되지 않음

#### 2) Sidecar의 `event.camera.statusHint`가 한 번도 발생하지 않음
- `boothy-20260131.log`에서 `boothy-camera-status-hint` 이벤트 기록이 0회
- 따라서 UI는 “변화 감지 트리거” 없이 최초 상태(RED)를 계속 유지할 수 있음

#### 3) Frontend가 단시간에 여러 번 재부팅되는 패턴(추정 원인 후보)
- `boothy-20260131.log`에서 동일 실행 구간에 `frontend-bootstrap / app-mounted`가 짧은 간격으로 반복됨
  - 예: `2026-01-31 09:56:38.xxx`, `09:56:45.xxx`, `09:56:47.xxx` 연속 발생
- 이 경우 Frontend의 `setInterval` 기반 폴링이 “정상적으로 누적 실행”되지 못할 수 있음(HMR/리로드 루프 등).

### 결론(근본 원인)
- “전원 사이클 이벤트 감지”가 `camera.getStatus` 호출/응답 또는 `statusHint` 이벤트에 의존하는데,
  - 실제 로그에서는 `camera.getStatus`가 1회만 수행되고,
  - `statusHint`는 0회였기 때문에,
  - **Sidecar/Boothy/Frontend 어느 레이어에서도 상태 갱신 트리거가 지속적으로 발생하지 않아 램프가 고정되는 상태**로 판단.

### 에이전트 시도 이력(무엇을 했고 왜 충분하지 않았나)
1) (UI) Library 램프 오표시 개선
   - `MainLibrary`가 `ipcState/loading/reconnecting` 정보를 직접 받아 GREEN/YELLOW/RED 판정 안정화
   - 하지만 이는 “표시 로직” 개선이며, **상태 갱신 트리거 자체가 없는 경우**(폴링/힌트 0회)에는 램프가 여전히 고정될 수 있음
2) (이번 수정) “Frontend 폴링 의존” 제거를 위한 Backend 주도 갱신 트리거 추가

### 적용한 수정(해결책)
1) **Boothy(Tauri/Rust): Backend가 주기적으로 `camera.getStatus`를 폴링**
   - Frontend가 리로드/스톨되어도 상태 감지가 계속 일어나도록 `CameraIpcClient`에 백그라운드 폴러 추가
   - 상태 변화(connected/detected)가 감지되면 `boothy-camera-status-hint`를 emit하여 UI가 즉시 갱신하도록 유도
   - 파일: `apps/boothy/src-tauri/src/camera/ipc_client.rs`

2) **Sidecar(C#): 카메라 카운트가 `>0 → 0`으로 떨어지는 경우도 statusHint emit**
   - 기존에는 `0 → 1+`(재연결)만 힌트 emit
   - 전원 OFF(또는 hot-unplug) 케이스를 보다 빨리 UI에 전파하도록 `cameraDisconnected` 힌트 추가
   - 파일: `apps/camera-sidecar/Camera/RealCameraController.cs`

---

## 추가 작업 기록 (2026-01-31, 프로토콜 버전 불일치로 IPC 실패 → 상태 갱신 불가)

### 시도(사용자 시나리오)
- 카메라 ON → 앱 실행 → `Continue session` 버튼 → Library 진입 → 램프 RED
- 새로고침(리프레시) → 초기 화면 자동 진입 → 다시 session → Library 진입 → 램프 YELLOW

### 결과(로그 관측)
- Sidecar는 `Mode: real`인데도 `camera.getStatus`가 정상적으로 처리되지 않고, **VersionMismatch 에러로 거절**되는 구간이 존재.
- Sidecar 로그(UTC): `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-sidecar-20260131.log`
  - `2026-01-31T13:47:03.482Z ... Protocol version mismatch: expected 1.0.0, got 1.1.0`
  - `2026-01-31T13:56:17.371Z ... Protocol version mismatch: expected 1.0.0, got 1.1.0`
  - 이후 `Pipe is broken` 등 연결 단절 로그가 연쇄로 발생할 수 있음(요청 실패 → 재연결/종료 경로 유발).

### 분석 결과(원인)
- Boothy(Tauri)와 Sidecar가 사용하는 IPC 프로토콜 버전이 달라서, Sidecar가 요청을 거절함.
  - Boothy는 `protocolVersion=1.1.0`으로 요청을 보냈고,
  - 실행 중 sidecar는 `Protocol Version: 1.0.0` 바이너리였음.
- 특히 dev 환경에서 Boothy가 실행하는 sidecar 경로가 `.../win-x86/publish/Boothy.CameraSidecar.exe`(publish 산출물)인 경우,
  - 코드 변경(프로토콜 bump)을 했더라도 publish를 갱신하지 않으면 **구버전 sidecar가 그대로 실행**되어 mismatch가 쉽게 재현됨.
  - Boothy 로그(로컬): `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-20260131.log`
    - `Sidecar path: C:\\Code\\Project\\Boothy\\apps\\camera-sidecar\\bin\\Debug\\net8.0\\win-x86\\publish\\Boothy.CameraSidecar.exe`

### 조치(해결 방향)
- **프로토콜 버전을 1.0.0으로 유지(호환 우선)**하도록 정리해, “기능 추가”가 “버전 불일치로 기능 전체 불능”을 만들지 않도록 함.
  - Sidecar: `apps/camera-sidecar/IPC/IpcMessage.cs`
  - Boothy: `apps/boothy/src-tauri/src/camera/ipc_models.rs`
- sidecar publish 산출물을 최신으로 갱신(중요):
  - `dotnet publish apps/camera-sidecar/Boothy.CameraSidecar.csproj -c Debug -r win-x86 --self-contained true`

### 확인(로컬 검증)
- `protocolVersion=1.0.0`으로 파이프에 요청을 보내면,
  - `event.camera.statusChanged` 이벤트와 `camera.getStatus` 응답이 정상 수신됨(VersionMismatch 없이 동작).
- 남은 확인: 실제 앱 플로우(continue session → library)에서
  - 더 이상 `Protocol version mismatch...`가 발생하지 않는지,
  - `boothy-camera-status` 스냅샷이 UI에 반영되는지,
  - 그리고 “카메라 ON인데 detect=0”이 반복되는 경우는 별도(EDSDK/점유/드라이버) 원인으로 분리하여 추가 분석 필요.

---

## 추가 작업 기록 (2026-02-01 KST / 2026-01-31 UTC, 전원 사이클 지연 + 자동 재시작 + 램프 고착(간헐) 계속)

### 시도(사용자 재현/기억 기반)
> 아래는 “기억” 기반이며, 정확한 타임라인은 로그로 재확인이 필요함.

- 전원 ON → (상태 램프 변화 없음) → 약 20~30초 뒤 갑자기 변화
- 전원 OFF → (정상적으로 변화)
- 전원 ON → 변화 없음
- `F5`로 메인 화면 이동 후 session 버튼으로 재진입 → 노란 램프
- 이후부터 전원 ON/OFF 해도 변화가 없거나, ON→OFF 시에도 노랑/빨강이 랜덤처럼 보임
- 결론: 현재 상태는 “정상적으로 램프가 되기도 / 안 되기도” 하는 간헐 상태

### 결과(로그 관측: 실제로 20~30초 지연 + 자동 재시작(sidecar) 패턴 존재)
로그 파일(이번 재현 구간):
- Sidecar(UTC): `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-20260131.log`
- Boothy(로컬/KST): `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260201.log`

관측된 핵심 타임라인(로컬 KST 기준):
1) **OFF 감지(Shutdown) 후, “카메라 added”까지 ~26.8초 지연 케이스 존재**
   - Sidecar: `2026-01-31T17:35:31.240Z` `Canon state event: Shutdown (camera powered off/disconnected)` (KST `2026-02-01 02:35:31.240`)
   - Sidecar: `2026-01-31T17:35:58.066Z` `Canon camera added event received` (KST `2026-02-01 02:35:58.066`)
   - Δ ≈ 26.8초 (사용자 체감 “20~30초 뒤 갑자기 변화”와 정합)

2) **LostAfterDetected → Boothy가 sidecar 자동 재시작을 트리거**
   - Boothy: `2026-02-01 02:35:31.508` `Camera not detected (streak). Auto-restarting sidecar. reason=Some(LostAfterDetected)`
   - Sidecar: `2026-01-31T17:35:42.641Z` `Received Request: system.shutdown`
   - Sidecar: `2026-01-31T17:35:43.268Z` `Boothy Camera Sidecar Starting...` (재기동)

3) **Shutdown 이벤트가 짧은 시간 내 2회 발생**
   - Sidecar: `2026-01-31T17:35:31.240Z` Shutdown
   - Sidecar: `2026-01-31T17:35:50.610Z` Shutdown
   - 해석: 물리 전원 OFF/드라이버 재열거/SDK 내부 상태에 따라 중복 신호가 올 수 있음 → UI에서는 “OFF 시 빨강/노랑 혼재”처럼 보일 여지가 있음

### 분석 결과(현재까지의 결론/가설)
- “전원 사이클 후 램프가 늦게(20~30초) 변하는” 현상은 로그에서도 실제로 관측됨(Shutdown → CameraAdded 지연).
- Boothy가 `LostAfterDetected`를 감지하면 sidecar를 자동 재시작하는데,
  - 이 과정은 UI 입장에서 “노란 램프(재연결/초기화 중)”처럼 보일 수 있고,
  - 전원 토글 타이밍과 겹치면 “랜덤하게 빨강/노랑이 보이는” 체감으로 이어질 수 있음.
- **중요 의심점(고착 관련)**: `boothy-20260201.log`에서 `02:35:58` 이후로는 `Sent request: camera.getStatus` / `Camera status snapshot received` 로그가 더 이상 나오지 않음.
  - 반면 Frontend 리로드 정황은 존재: `2026-02-01 02:36:57.720` / `2026-02-01 02:37:15.557` `frontend-bootstrap` (사용자 체감 “F5 후 재진입”과 정합)
  - 가능성 1) 해당 시점 직후 앱이 종료/재시작되어 로그가 끊겼음
  - 가능성 2) Backend 폴러/이벤트 루프가 중단되어 이후 상태 갱신 트리거가 사라졌음(“이후부터 전원 ON/OFF 해도 아무 변화 없음” 체감과 부합)
  - 가능성 3) Frontend 리로드 시점에 `apps/boothy/src/tauriMock.ts`가 Tauri 런타임을 오탐지해 `mockIPC`가 활성화됨
    - 이 경우 `invoke/listen`이 실제 Tauri IPC 대신 mock으로 동작해서, **Boothy(backend) 로그에 `camera.getStatus`가 더 이상 찍히지 않는 패턴**이 설명됨(간헐/랜덤처럼 보이는 체감과도 정합)
    - 조치: `tauriMock.ts`의 mock 활성화 조건을 `__TAURI_INTERNALS__` 단독 체크에서 `__TAURI__` 존재 여부로 보강(리로드/F5에서 오탐지 방지)
  - 다음 세션에서는 “증상 발생 후 앱을 종료하지 않은 상태로 1~2분 유지”한 로그를 확보해, 2) 여부를 확정해야 함

### 다음 세션에서 필요한 것(사용자 할 일)
- 증상이 발생한 **정확한 로컬 시간(KST)**을 1~2개만 메모(예: “02:xx:xx에 ON”, “02:yy:yy에 F5”).
- 증상 발생 후 **앱을 바로 종료하지 말고 60~120초 유지**(폴러 중단/재시작 여부 확인 목적).
- 같은 세션의 로그 파일 2개를 함께 공유:
  - `boothy-YYYYMMDD.log` (로컬 날짜 기준)
  - `boothy-sidecar-YYYYMMDD.log` (UTC 날짜 기준일 수 있음)

---

## 추가 작업 기록 (2026-02-01 KST, 테스트 워크플로우 4/6/9/12 FAIL 케이스)

### 재현 정보
- 테스트 워크플로우 파일:
  - `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\test_flow_and_log_dir.txt`
  - 결과: 4 OK, 6 FAIL, 9 FAIL, 12(노란 램프) FAIL
- 로그(0201):
  - `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-20260201.log`
  - `C:\\Users\\KimYS\\AppData\\Roaming\\Boothy\\logs\\boothy-sidecar-20260201.log`

### 분석(원인)
- 카메라 전원 사이클로 인해 sidecar가 **무응답/타임아웃/강제 재시작(stop)** 되는 경로가 발생할 수 있는데,
  - 이 경로는 사용자-facing 에러(`boothy-camera-error`)를 억제하는 설계(백그라운드 폴링/timeout)와 겹치면,
  - **UI가 램프 갱신을 위한 refresh 트리거를 못 받아** 마지막 상태(초록)로 고착될 수 있음.
- 리로드(F5) 상황에서는 start 경합/진행 중 상태가 UI에 남아 **노란 램프(reconnecting/loading)** 가 길게 유지될 수 있음.

### 적용한 수정(2026-02-01)
1) **Boothy(Tauri/Rust): `stop_sidecar()`에서 항상 `boothy-camera-status-hint` emit**
   - 백그라운드 폴링/timeout 등 “에러 이벤트 억제” 경로에서도 UI가 `getStatus`를 다시 호출해 램프를 즉시 갱신하도록 함.
   - 파일: `apps/boothy/src-tauri/src/camera/ipc_client.rs`
2) **Boothy(Tauri/Rust): 백그라운드 폴러가 연속 에러를 감지하면 1회 `status-hint` emit**
   - UI가 즉시 pull(getStatus)로 복구 시도할 수 있도록 최소 트리거 제공(스팸 방지).
   - 파일: `apps/boothy/src-tauri/src/camera/ipc_client.rs`
3) **UI(React): `boothy-camera-status-hint`가 `stopSidecar|backendPollError`이면 최근 snapshot 여부와 무관하게 강제 refresh**
   - “최근 snapshot 수신 최적화”가 복구 트리거를 막지 않도록 예외 처리.
   - 파일: `apps/boothy/src/App.tsx`
4) **Boothy(Tauri/Rust): `start_sidecar()`가 이미 진행 중이면 짧게 대기 후 연결 여부 확인**
   - 리로드/StrictMode에서 start 경합으로 `ipcState=reconnecting`이 고착되는 케이스 완화.
   - 파일: `apps/boothy/src-tauri/src/camera/ipc_client.rs`

### 테스트 재실행 결과(2026-02-01)
- 사용자 보고:
  - 6번 FAIL, 9번 FAIL, 10번(노란 램프) FAIL

### 추가 분석(0201 로그 기반)
- `boothy-20260201.log`에서 `2026-02-01 15:42:58`에 `frontend-bootstrap`이 재발생(리로드/F5 정황)했지만,
  - 같은 시점 이후 `Sent request: camera.getStatus` / sidecar 재시작 로그가 남지 않음
  - 해석: WebView 리로드 타이밍에 `invoke`가 **resolve/reject 없이 멈추는(hang)** 케이스가 있고, 이때 `cameraStatusLoading=true`가 해제되지 않아 램프가 노란색으로 고착될 수 있음.
- `boothy-sidecar-20260201.log`에는 power-off/on에 대응하는 `Shutdown`/`pnpRemoved` 류 이벤트가 충분히 남지 않을 수 있어(USB 장치가 논리적으로는 유지되는 케이스 등),
  - UI가 “push 이벤트만”으로는 OFF/ON 변화를 안정적으로 반영하기 어려움 → **주기적 pull(getStatus) fallback**이 필요.

### 추가 수정(2026-02-01)
1) **UI(React): `invoke(boothy_camera_get_status)` 5초 타임아웃 + stale 응답 무시**
   - 리로드/F5에서 invoke hang → `cameraStatusLoading`이 영구 true(노란 램프) 되는 문제를 차단
   - 파일: `apps/boothy/src/App.tsx`
2) **UI(React): 2초 간격으로 `refreshCameraStatus()` 주기 실행(문서 숨김 시 스킵)**
   - power-cycle에서 push 이벤트가 누락되더라도 lamp가 결국 갱신되도록 안정장치 추가
   - 파일: `apps/boothy/src/App.tsx`
3) **UI(React): 카메라가 이미 ready면 refresh 중이라도 램프를 노란색으로 바꾸지 않음**
   - polling으로 인한 “초록↔노랑” 깜빡임/오해 방지
   - 파일: `apps/boothy/src/components/panel/MainLibrary.tsx`

---

## 추가 작업 기록 (2026-02-01 KST, 테스트 워크플로우 6/9/12 FAIL 지속)

### 재현 정보(추가)
- 테스트 워크플로우 결과(사용자 최신 보고):
  - 6 FAIL, 9 FAIL, 12(황색 램프) FAIL
- 로그(0201, **주의: 날짜 파일이 덮어써져 이전 세션 로그가 소실될 수 있음**):
  - `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260201.log` (LastWrite: 2026-02-01 17:21:45 KST)
  - `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-20260201.log` (LastWrite: 2026-02-01 17:20:38 KST 근처)

### 로그 관측(핵심)
- `boothy-20260201.log`:
  - 17:20:38에 `event.camera.statusChanged` 1회 수신 후(= snapshot 1회 push),
  - 이후 `Sent request: camera.getStatus`가 주기적으로 반복되는 로그가 보이지 않음.
  - 17:21:45에 `frontend-bootstrap`이 다시 찍힘(F5/리로드 정황).
- `boothy-sidecar-20260201.log`:
  - 08:20:38Z(=17:20:38 KST) 부근의 startup/getStatus 로그만 존재하며,
  - power-off/on에 대응하는 `Shutdown`/statusHint/statusChanged 로그가 남아있지 않음.

### 원인(추정: UI 램프가 stale snapshot을 “무조건 우선”으로 사용)
- `MainLibrary`의 램프 계산 로직이 `cameraStatusSnapshot`(push)을 **존재하기만 하면** `cameraStatusReport`(pull)보다 우선으로 사용함.
  - 따라서 power-off 상황에서 sidecar가 `statusChanged`(push)를 못 보내면,
  - snapshot이 `ready`로 남아 **OFF(빨강)로 내려가지 않고** 램프가 멈춘 것처럼 보일 수 있음.
- 0201 로그에서 snapshot이 1회만 들어오고 이후 갱신이 끊긴 정황과 정합.

### 추가 수정(2026-02-01, stale snapshot 자동 무효화 + 스토리 타임아웃 정합)
1) **UI(React): `cameraStatusSnapshot`을 “최근 4초 이내 관측”인 경우에만 신뢰**
   - stale snapshot이면 `cameraStatusReport(getStatus)` 기반으로 램프/메시지를 결정하도록 변경
   - 또한 `getStatus` 결과가 `cameraDetected=false`/`ipcState!=connected`인 경우 snapshot을 즉시 clear하여 OFF 표시를 방해하지 않게 함
   - 파일: `apps/boothy/src/App.tsx`
2) **Tauri(Rust): `boothy_camera_get_status` IPC 타임아웃을 2500ms로 조정**
   - Story 4.4 AC-4(2.5초 내 Timeout 응답)와 정합
   - 파일: `apps/boothy/src-tauri/src/main.rs`

### 다음 확인(사용자 재테스트 필요)
- 동일 워크플로우에서 6/9 단계가 “빨강”으로 바뀌는지 확인 필요.
- 만약 여전히 FAIL이면, **실패 직후 60~120초 앱을 유지**(로그 누락 방지) 후 최신 0201 로그(boothy/sidecar) 다시 확인.

---

## 추가 작업 기록 (2026-02-01 KST, 앱 실행 직후 “상태 변화 반복”)

### 증상(사용자 제보)
- 앱 실행 직후 아무 동작 없이도 카메라 상태(램프/메시지)가 계속 바뀌는 것처럼 보임.

### 로그 근거(0201, KST)
- `boothy-20260201.log`에서:
  - `17:41:58.134`에 `Sent request: camera.getStatus`가 **동시에 2개** 발생
  - `17:42:00.144~00.648`에 `IPC timeout during camera.getStatus - restarting sidecar`
  - 같은 요청에 대해 sidecar는 `17:42:00.708`에 `camera.getStatus exceeded 2500ms; ... terminating sidecar`를 기록(즉, **Rust가 sidecar(2500ms)보다 먼저(2s) 타임아웃**)
- `boothy-sidecar-20260201.log`에서:
  - timeout 이후 `Boothy Camera Sidecar Starting...`가 짧은 간격으로 반복(재시작 루프)

### 원인(추정)
- `camera.getStatus`가 동시에/과빈도로 호출되면서(프론트 폴링 + 백엔드 monitor + dev StrictMode/리로드),
  - sidecar의 EDSDK 호출이 2.5초를 넘겨 timeout → self-terminate,
  - Rust status monitor의 timeout(2s)이 sidecar timeout(2.5s)보다 짧아 **조기 timeout→강제 restart**를 유발,
  - 결과적으로 “연결/재연결/오류” 상태가 반복 노출됨.

### 추가 수정(2026-02-01, 과빈도/조기 timeout 방지)
1) **UI: 2초 주기 polling 제거**
   - push(statusChanged)/hint 기반으로만 갱신하여 getStatus 과부하를 줄임
   - 파일: `apps/boothy/src/App.tsx`
2) **Tauri(Rust): status monitor를 5초 폴링 + 초기 지연 + 에러 시 지수 backoff**
   - sidecar/EDSDK가 불안정할 때 재시작/요청 루프를 완화
   - 파일: `apps/boothy/src-tauri/src/camera/ipc_client.rs`
3) **Tauri(Rust): boothy_camera_get_status의 IPC timeout을 4초로 상향**
   - sidecar(2500ms) timeout 응답을 기다릴 수 있도록 Rust 조기 timeout(2s/2.5s) 경쟁 제거
   - 파일: `apps/boothy/src-tauri/src/main.rs`

---

## 추가 작업 기록 (2026-02-01 KST, 연결 끊김 → 접속 무한 반복)

### 증상(현재 상태)
- 카메라가 물리적으로 끊긴 상태(USB 분리/전원 OFF 등)에서 Library 램프가 **연결 끊김 ↔ 접속 시도**를 반복(무한 재시작처럼 보임).

### 로그 근거(0201)
- `boothy-sidecar-20260201.log`에서:
  - `GetStatus: begin ...` 이후
  - `camera.getStatus exceeded 2500ms; returning Timeout and terminating sidecar for recovery`
  - 이후 `Boothy Camera Sidecar Starting...`가 반복(= 재시작 루프)

### 원인(RCA)
- **Canon 카메라가 실제로 존재하지 않는 상태에서도(sidecar 관점에서) EDSDK로 카메라 리스트를 조회**하는 경로가 실행되고,
  - 일부 환경에서 `EdsGetCameraList` 계열 호출이 **카메라 미연결 상태에서 장시간 블로킹**될 수 있음.
- sidecar는 `camera.getStatus`가 2.5초를 넘기면 “native hang 복구” 목적으로 **프로세스 종료(Exit 2)** 하도록 설계되어 있어,
  - 결과적으로 “미연결 → getStatus hang → sidecar 종료 → Rust가 재시작”이 반복되며
  - UI는 `ipcState=reconnecting`/노란 램프 상태를 반복적으로 보게 됨.

### 적용한 수정(2026-02-01)
1) **Sidecar(.NET): Canon 카메라(USB VID_04A9) 존재 여부를 SetupAPI로 빠르게 판별**
   - Canon 디바이스가 없으면 EDSDK 호출을 건너뛰고 “카메라 없음” 결과를 즉시 반환(= getStatus가 hang으로 가지 않게 차단)
   - 파일: `apps/camera-sidecar/Camera/ImagingDeviceProbe.cs`
2) **Sidecar(.NET): Canon 디바이스가 없으면 CanonEdsdkProbe가 즉시 no-camera 반환**
   - 파일: `apps/camera-sidecar/Camera/Canon/CanonEdsdkProbe.cs`
3) **Sidecar(.NET): Canon 디바이스가 없을 때는 ‘no camera’ 연속에 따른 EDSDK reset 시도도 스킵**
   - 불필요한 SDK reset으로 disconnected 구간이 길어지거나 불안정해지는 것을 방지
   - 파일: `apps/camera-sidecar/Camera/RealCameraController.cs`

### 기대 결과 / 다음 확인
- Canon 카메라가 없는 상태에서 `camera.getStatus`가 **2.5초 타임아웃으로 sidecar를 종료하지 않고**, 빨강 램프로 안정적으로 유지되어야 함.
- 확인 포인트:
  - `boothy-sidecar-20260201.log`에 `camera.getStatus exceeded 2500ms`가 “카메라 미연결 상태”에서는 더 이상 반복되지 않아야 함.
  - 카메라 연결(또는 전원 ON) 시에는 10초 내에 초록으로 복귀하는지(Story 4.4 AC 기준) 재확인.

### 후속 수정(2026-02-01, 회귀: 전원 ON인데 미감지/노란 램프)
- 증상:
  - 카메라 전원을 ON 한 뒤 앱을 실행했는데도 `cameraDetected=false`로 유지되며 노란 램프(준비/재연결)가 지속.
- 원인:
  - Canon 카메라가 Windows에서 항상 `GUID_DEVINTERFACE_IMAGE`(Imaging device interface)로 노출되지 않고,
    “휴대용 장치(WPD)” 등 다른 클래스로만 잡히는 경우가 있어,
    **Imaging interface 기반 VID 체크가 false negative**를 만들 수 있음.
  - 이 false negative가 EDSDK probing 자체를 스킵시켜 “전원 ON인데도 감지 불가”로 이어짐.
- 조치:
  - Canon 디바이스 존재 판별을 `GUID_DEVINTERFACE_IMAGE` 한정이 아니라,
    SetupAPI로 **현재 존재하는 모든 PnP 장치의 HardwareId/CompatibleId(예: `USB\\VID_04A9`)**를 스캔하는 방식으로 변경.
  - 오류/판별 불가 시에는 **fail-open(= EDSDK probing 수행)** 하여 감지 누락을 방지.
  - 파일: `apps/camera-sidecar/Camera/ImagingDeviceProbe.cs`
  - 구현 디테일:
    - `DIGCF_PRESENT|DIGCF_ALLCLASSES`로 디바이스 열거 후
      `SetupDiGetDeviceInstanceId` + `SPDRP_HARDWAREID`/`SPDRP_COMPATIBLEIDS`를 우선 확인.
    - Canon VID(`VID_04A9`)가 확인되면 EDSDK probing을 허용(감지 경로 유지).
    - SetupAPI 호출 실패/버퍼 문제 등으로 판단 불가하면 **false 반환 금지(= true로 처리)** 해서 회귀를 방지.
  - 반영:
    - `dotnet publish apps/camera-sidecar/Boothy.CameraSidecar.csproj -c Debug -r win-x86 --self-contained false`로 `.../win-x86/publish/Boothy.CameraSidecar.exe` 갱신.

---

## 추가 작업 기록 (2026-02-01 KST, Sidecar(.NET x86) 실행 실패 → Library 램프 갱신 불가)

### 로그 근거
- Boothy 로그: `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260201.log`
  - `You must install .NET to run this application.`
  - `Architecture: x86`
  - `.NET location: Not found`
  - `Failed to resolve hostfxr.dll [not found]. Error code: 0x80008083`
  - 이후 `Failed to connect to Named Pipe after 10 retries ... (os error 2)`가 반복되며 sidecar 재시작 루프

### 원인(RCA)
- dev 환경에서 Boothy가 실행하는 sidecar가 **framework-dependent(x86 런타임 필요)** 형태로 존재하는 경우,
  - 해당 PC에 **.NET Desktop/Runtime x86**가 설치되어 있지 않으면 sidecar가 즉시 종료하며 pipe 서버가 열리지 않음
  - 결과적으로 `camera.getStatus`가 정상 응답하지 못하고 UI는 `ipcState=reconnecting`(노란 램프) 상태에 고착될 수 있음

### 해결(적용)
1) **Sidecar를 win-x86 self-contained + single-file로 고정**
   - 파일: `apps/camera-sidecar/Boothy.CameraSidecar.csproj`
   - publish 예시:
     - `dotnet publish -c Release -r win-x86 --self-contained true /p:PublishSingleFile=true /p:IncludeNativeLibrariesForSelfExtract=true`
2) **번들 리소스에 작은(sidecar) exe가 들어가면 실패하도록 가드 추가**
   - 파일:
     - `apps/boothy/scripts/prepare-sidecar-resources.mjs`
     - `apps/boothy/scripts/packaging-smoke-check.mjs`
   - 기준: `Boothy.CameraSidecar.exe`가 **10MB 미만이면** framework-dependent 가능성이 높으므로 스크립트를 실패 처리
3) **dev에서도 repo resources sidecar를 우선 사용**
   - 파일: `apps/boothy/src-tauri/src/camera/ipc_client.rs`
4) **UI: sidecar 시작 실패는 “재연결(노랑)”이 아니라 “사용 불가(빨강)”로 표시**
   - 파일:
     - `apps/boothy/src/App.tsx`
     - `apps/boothy/src/components/panel/MainLibrary.tsx`
   - 변경: `cameraStatusError`/`lastError`를 preparing보다 우선 처리하여, 고객 모드에서 빨강 + 안내 문구가 보이도록 보강
5) **prepare-sidecar-resources: single-file 전제하에 exe만 복사(Windows file lock 회피)**
   - 파일: `apps/boothy/scripts/prepare-sidecar-resources.mjs`
   - 배경: Windows에서 기존 `resources/camera-sidecar/*` 파일이 잠긴 경우 `cpSync`(폴더 전체 복사)가 `unlink EPERM`으로 실패할 수 있어, 단일 파일 복사로 전환

### 기대 결과
- `boothy-YYYYMMDD.log`에서 `You must install .NET... hostfxr.dll not found` 로그가 더 이상 발생하지 않음
- sidecar가 정상 기동되어 pipe 연결 성공
- Library 램프가 전원 ON 시 **10초 내 초록**으로 복귀(Story 4.4 AC 기준)

### 검증(개발 환경)
- Sidecar publish 산출물 크기 예: `.../win-x86/publish/Boothy.CameraSidecar.exe` ≈ 61MB(= self-contained 신호)
- Frontend 테스트: `cd apps/boothy; npm test` PASS
- Rust 테스트: `cd apps/boothy/src-tauri; cargo test` PASS
  - 참고: 일부 환경에서 workspace `target/` 파일 잠금(Access denied) 이슈가 있어, 필요 시 `%TEMP%`를 `CARGO_TARGET_DIR`로 지정해 실행

---

## 추가 작업 기록 (2026-02-01 KST, 카메라 ON인데 Library 램프가 빨강으로 표시되는 케이스)

### 현상
- 카메라 전원 ON 상태에서 앱 실행 시, Library 화면의 카메라 상태 램프가 **빨강**으로 표시됨.

### 로그 근거
- Boothy 로그: `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260201.log`
  - `Connected to Named Pipe`
  - `Starting camera status monitor (poll=5s)`
  - `GetStatus: session open succeeded -> detected model=Canon EOS 700D`
- Sidecar 로그: `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-20260201.log`
  - `Mode: real`
  - `Canon EDSDK initialized`
  - `GetStatus: session open succeeded -> detected model=Canon EOS 700D`
  - `Sent Event: event.camera.statusChanged`

### 원인(RCA)
- Boothy(Rust)의 `connect_to_pipe_with_retries`가 “기존 sidecar pipe가 이미 떠있는지” 확인(= probe) 용도로도 사용되는데,
  - 이 probe 실패(대부분은 정상: 아직 sidecar 미기동)를 `last_error`로 기록하고 `ipcState=disconnected`로 내려버려,
  - 고객 UI에서 `lastError`를 근거로 “사용 불가(빨강)”로 판단하거나, 준비중(노랑) 상태가 짧게라도 빨강으로 보이는 원인이 됨.
- Frontend의 `invoke(BoothyCameraGetStatus)` 타임아웃(5s)이 cold start + 첫 상태 조회에 타이트(로그 기준 약 4~5초 구간)하여,
  - 환경에 따라 초기 1회 조회가 timeout → 빨강으로 시작/유지되는 케이스가 발생할 수 있음.
- 추가로, **F5(WebView reload)** 시나리오에서는 backend/sidecar가 이미 연결된 상태라 `boothy-camera-connected`/`statusHint`가 새로 오지 않을 수 있는데,
  - 이때 frontend가 `boothy_camera_get_status`를 1회라도 성공적으로 호출하지 못하면(혹은 호출 자체가 누락되면) 램프가 빨강으로 남을 수 있음.
  - 로그 근거: `boothy-20260201.log`에서 F5 이후(`frontend-bootstrap` 재등장) **`Sent request: camera.getStatus` 로그가 발생하지 않음**.

### 해결(적용)
- `apps/boothy/src-tauri/src/camera/ipc_client.rs`
  - Named Pipe “probe” 실패는 `last_error`/`ipcState`를 오염시키지 않도록 분리(`record_error=false`)
  - pipe 연결 성공 및 IPC 요청 성공 시 `last_error` clear
- `apps/boothy/src/App.tsx`
  - 초기 `invoke(BoothyCameraGetStatus)` 타임아웃을 9초로 상향(5s → 9s)
  - `frontend_ready` 성공 시 `refreshCameraStatus()`를 1회 호출하여, F5 이후에도 반드시 pull(getStatus)로 상태를 동기화

### 기대 결과
- 카메라 ON → 앱 실행 시: 빨강으로 시작/고착되는 확률이 감소하고, (필요 시) 노랑(연결 중) → 초록(ready)로 자연스럽게 전환.

### 테스트 플로우 실행 기록 (2026-02-01, `test_flow_and_log_dir.txt` 기반)
- 파일: `C:\Users\KimYS\AppData\Roaming\Boothy\logs\test_flow_and_log_dir.txt`
- 절차:
  1. camera power on
  2. app start
  3. continue session click
  4. lamp check
  5. camera power off
  6. lamp check
  7. go to home
  8. continue session click
  9. lamp check
  10. camera power on
  11. lamp check
  11. push F5 keyboard
  12. lamp check
- 결과(사용자 보고):
  - 4-ok
  - 6-ok
  - 9-ok
  - 11-ok
  - 12-fail (red lamp)
- 참고 로그:
  - `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-20260201.log`
  - `C:\Users\KimYS\AppData\Roaming\Boothy\logs\boothy-sidecar-20260201.log`
