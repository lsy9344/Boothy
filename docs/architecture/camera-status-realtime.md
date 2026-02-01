# 카메라 상태(ON/OFF) 실시간 반영 아키텍처 설계 (Windows + Canon EDSDK)

## 0. TL;DR (이번 문서의 “명령” 해석)

“잘 되는 기능을 그대로 따라하라”는 요구를 **코드 복사**로 해석하면 위험합니다(레퍼런스에 GPLv3 포함). 대신 본 문서는 아래를 **구현 스펙**으로 고정합니다.

1. **digiCamControl의 동작 흐름을 1:1로 재현**한다(이벤트 → 즉시 재열거 → 상태 확정).
2. Headless sidecar에서 이벤트가 누락되지 않도록 **WPF 환경과 유사한 실행 조건(메시지 루프/STA 또는 확실한 이벤트 펌프)** 을 제공한다.
3. UI 램프는 pull(getStatus) 중심이 아니라 **push(상태 스냅샷 이벤트)** 중심으로 렌더링한다.
4. “Mode: mock” 같은 환경 실패는 숨기지 말고 **스냅샷으로 즉시 표면화**한다(현장 디버깅 가능).

이 문서는 “아키텍처 아이디어”가 아니라, 바로 스토리로 쪼개 구현할 수 있도록 **상태 모델/스레딩/프로토콜/파라미터/검증 시나리오**를 명시합니다.

---

## 0.1 진행 순서(중요): Epic 4에서 4.4를 먼저 해도 되는가?

결론: **가능합니다.** Story 4.4는 “촬영/전송/인제스트 파이프라인”이 아니라 **‘상태/재연결/전원 사이클 복구’** 가 목적이므로 Epic 4의 다른 스토리(4.1~4.3)보다 먼저 진행할 수 있습니다.

단, 아래 선행 조건이 충족되어야 합니다(충족되지 않으면 4.4는 **블로커**입니다).

- (필수) Boothy ↔ Sidecar IPC 경로가 이미 존재해야 함  
  - 최소한 `camera.getStatus` 요청/응답과 `event.camera.statusHint` 수신/전달 경로가 있어야 4.4를 착수할 수 있습니다.
- (필수) sidecar가 **실카메라(real) 모드**로 동작할 수 있어야 함(§1.1, §9)  
  - Story 4.4는 **mock을 허용하지 않습니다.** `Mode: mock`이면 원인 분석/진단이 아니라 “환경 미충족”으로 간주하고 진행을 중단합니다.

---

## 1. 요구사항(기능/품질) — Acceptance Criteria 수준으로 고정

### 1.1 사용자 관점 AC(히스토리 문서 시나리오 고정)

AC-1 (OFF→ON):  
카메라 전원 OFF 상태에서 앱 실행 후 카메라 전원을 ON 하면, **10초 이내** UI 램프가 빨강→초록으로 전환된다(앱 재시작/수동 새로고침 없이).

AC-2 (ON→OFF→ON):  
카메라 전원 ON 상태에서 앱 실행(초록 확인) 후 OFF(빨강 확인) 후 다시 ON 하면, **10초 이내** 초록으로 복귀한다.

AC-3 (모드/진단 가시성):  
sidecar가 `Mode: mock`으로 떨어지거나 EDSDK 로드/아키텍처 문제가 있으면, admin 진단 화면과 로그에서 **원인이 즉시 식별 가능**해야 한다.

AC-4 (폴백/내구성):  
전원 사이클 중 EDSDK가 native call에서 멈추면, **2.5초 이내** `camera.getStatus`가 Timeout으로 종료되고(이미 존재하는 watchdog 활용), Boothy는 sidecar 재시작으로 복구를 시도한다.

AC-5 (Mock 금지 — Story 4.4 고정):  
Story 4.4 범위의 검증/완료 조건에서 sidecar가 `Mode: mock`으로 실행되면 **FAIL**이다. (진단 표면화는 부가 요구사항이며, mock으로 “대체 성공”은 인정하지 않는다.)

### 1.2 시스템 관점 NFR(필수)

- NFR-1: EDSDK 호출은 동시/병렬 호출 금지(단일 컨텍스트로 serialize)
- NFR-2: 이벤트 폭주가 UI/로그를 망치지 않도록 디바운스/코얼레싱 필수
- NFR-3: Offline(네트워크 차단) 환경에서 동일하게 동작

---

## 2. “그대로 따라할 레퍼런스”를 동작으로 스펙화(클린룸 구현용)

> 여기서는 “어떤 파일을 복사”가 아니라 “어떤 **행동 규칙**을 그대로 재현”인지 고정합니다.

### 2.1 digiCamControl의 Canon 재연결 핵심 패턴(요약)

1) SDK init 후 cameraAdded 핸들러 등록  
- `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/EosFramework.cs`

2) cameraAdded 이벤트 수신 시 즉시 “재열거”  
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/CameraDeviceManager.cs`
  - `_framework_CameraAdded` → `AddCanonCameras()` → `GetCameraCollection()`

3) Shutdown 이벤트 수신 시 즉시 “연결 해제 확정”  
- `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/EosCamera.HandeStateEvents.cs`
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/Canon/CanonSDKBase.cs`

4) OS 레벨 디바이스 이벤트를 힌트로 활용(WIA)  
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/CameraDeviceManager.cs`

### 2.2 Boothy에 적용할 “레퍼런스 패턴 1:1 규칙”(명령어 수준)

- RULE-1: `cameraAdded`(또는 OS device added 힌트)를 받으면, **지연 없이 probe(enumerate)로 전이**한다.
- RULE-2: 상태는 힌트로 단정하지 않고 **probe 결과로 확정**한다(스냅샷).
- RULE-3: `shutdown/internalError`를 받으면 즉시 세션을 닫고 **NoCamera 스냅샷을 확정**한다.
- RULE-4: headless 환경에서도 RULE-1~3이 성립하도록, sidecar에 **이벤트 전달 조건(메시지 루프/STA 또는 강제 펌프)** 을 제공한다.

---

## 3. TO-BE 설계(요약): Hint + Snapshot + Reconciler

### 3.1 개념 분리(절대 섞지 않음)

- `event.camera.statusHint`: 트리거(“변화 가능성”)
- `event.camera.statusChanged`: 정답 스냅샷(“UI가 렌더링하는 상태”)

### 3.2 컴포넌트

Sidecar(.NET):
- `CanonSdkRuntime` : SDK init/핸들러 등록 + (중요) 이벤트 전달 조건 제공
- `ProbeEngine` : enumerate + (가능하면) session open 가능 여부 확인
- `StatusReconciler` : 힌트/프로브/오류를 단일 상태로 수렴시키고 `statusChanged` emit
- `DeviceHintSource` : WIA 우선(레퍼런스와 동일), WMI/Win32는 fallback

Tauri(Rust):
- sidecar 이벤트를 UI 이벤트로 브릿지(`statusChanged` → `boothy-camera-status`)

Frontend(React):
- 램프는 `boothy-camera-status`만으로 렌더링
- 힌트는 (필요 시) 1회 `camera.getStatus`를 트리거하는 폴백으로만 사용

---

## 4. Sidecar 상세 스펙(개발자가 그대로 구현 가능한 수준)

### 4.1 스레딩/동시성 규칙(EDSDK 안전성)

EDSDK는 스레드/동시 호출에 민감할 수 있으므로, 아래 중 하나를 **반드시 선택**하고 지킨다.

선택 A(권장): “단일 SDK 스레드(STA) + 작업 큐”
- 모든 EDSDK 호출은 SDK 스레드에서만 수행
- IPC 요청은 SDK 스레드로 enqueue 후 결과를 받아 응답

선택 B(차선): “전역 락 + 엄격한 serialize + 펌프 스레드 분리”
- 모든 EDSDK 호출은 동일 락으로 serialize
- 펌프(메시지 루프/이벤트 루프)는 별도 스레드에서 유지

**필수 불변**: 어떤 선택이든 EDSDK 호출이 동시에 실행되면 FAIL(스펙 위반).

### 4.2 이벤트 전달 조건(핵심): WPF와 유사한 환경 제공

히스토리에서 cameraAdded 로그가 없었던 관측을 고려하면, 단순 EdsGetEvent 조건부 호출만으로는 부족할 수 있다.

따라서 아래를 구현 옵션으로 명시한다(현장 안정성 최우선).

- `BOOTHY_EDSDK_PUMP_MODE=messageLoop|edsGetEvent|both`
  - `messageLoop`: STA + Windows 메시지 루프(숨은 윈도우/메시지 펌프) 제공
  - `edsGetEvent`: 일정 주기로 `EdsGetEvent()`를 호출(현 구조 확장)
  - `both`: 둘 다 활성화(가장 보수적)

**기본값 결정(프로젝트 특성 기반, 내부 배포 권장): `both`**
- Boothy는 현장(오프라인)에서 “전원 사이클/재열거”가 자주 발생하고, headless sidecar 환경에서 콜백 누락 가능성이 높습니다(히스토리 관측).
- 또한 sidecar는 별도 프로세스이며, 이미 `camera.getStatus` watchdog(2.5초)로 “native hang 시 프로세스 종료→재시작” 복구 경로를 사용하고 있습니다.
- 따라서 “정상 경로에서의 이벤트 전달 확률”을 최대로 올리는 것이 목표이며, **메시지 루프와 `EdsGetEvent`를 모두 켜는 `both`가 기본값**으로 가장 적합합니다.

운영 시나리오별 권장:
- 내부/현장 배포(안정성 최우선): `BOOTHY_EDSDK_PUMP_MODE=both`
- 개발 환경(문제 격리/단순화): `BOOTHY_EDSDK_PUMP_MODE=messageLoop` 또는 `edsGetEvent`로 단독 테스트 가능(이슈 재현/분리 목적)

### 4.2.1 사전 점검(Go / No-Go) — 개발자 에이전트용

Story 4.4 착수 전, 아래 항목을 **로그로 확인**하고 Go/No-Go를 결정합니다.

**Go 조건(모두 만족):**
- sidecar 로그에 `Mode: real`이 찍힌다. (`apps/camera-sidecar/Program.cs`가 startup에 출력)
- sidecar 로그에 Canon SDK 초기화 로그가 찍힌다(예: `Initializing Canon EDSDK...`, `Canon EDSDK initialized`). (`apps/camera-sidecar/Camera/RealCameraController.cs` 경로)
- Boothy 로그에서 `camera.getStatus` 요청/응답이 주기적으로 보인다(또는 backend 폴러가 정상 동작한다).

**No-Go(블로커) 조건(하나라도 해당 시 4.4 진행 중단):**
- sidecar 로그가 `Mode: mock`을 출력한다. (Story 4.4는 mock 허용하지 않음)
- sidecar 로그에서 `EDSDK load failed` / `EDSDK architecture mismatch` / `ResolvedPath=none` 등으로 real 모드가 불가함이 확인된다.
- `camera.getStatus`가 지속적으로 timeout/재시작 루프만 발생하고, 정상 probe 스냅샷이 만들어지지 않는다(먼저 EDSDK/드라이버/점유 앱 등 환경 문제를 해결).

**해석 규칙(중요):**
- `Mode: mock`은 “제품 로직 문제”가 아니라 “환경 미충족”으로 취급한다. 4.4의 목표(전원 사이클에서 이벤트→재열거 패턴 재현)를 검증할 수 없기 때문이다.
  - 즉, mock 상태에서 UI 램프가 바뀌어도 4.4는 완료로 인정하지 않는다.

### 4.3 상태 모델(램프 매핑 포함)

내부 상태:
- `NoCamera`
- `Connecting`
- `Ready`
- `Error`

램프 매핑(기본):
- 빨강 = `NoCamera` 또는 `Error`
- 초록 = `Ready`
- (선택) 노랑 = `Connecting`

### 4.4 `event.camera.statusChanged` (정답 스냅샷) 스펙

Payload(권장):
```json
{
  "seq": 123,
  "observedAt": "2026-01-31T00:00:00.000Z",
  "reason": "startup|probe|cameraAdded|shutdown|pnpAdded|pnpRemoved|sdkReset|ipcTimeout",
  "mode": "real|mock",
  "sdk": {
    "initialized": true,
    "diagnostic": null,
    "resolvedPath": "C:\\\\...\\\\edsdk\\\\EDSDK.dll",
    "platform": "x86|x64"
  },
  "state": "noCamera|connecting|ready|error",
  "connected": true,
  "cameraDetected": true,
  "cameraReady": true,
  "cameraCount": 1,
  "cameraModel": "Canon EOS ..."
}
```

Emit 규칙:
- `seq` 단조 증가
- “동일 스냅샷 반복”은 emit 생략(로그/이벤트 폭주 방지)
- 단, `sdk.diagnostic`가 새로 생긴 경우(예: real 모드 불가 진단)는 1회 emit 허용(진단 목적). **단, Story 4.4의 완료 기준에서는 `mode=mock`은 FAIL** 이다.

### 4.5 Probe 코얼레싱(폭주 방지) 스펙

- `probeInFlight`는 1개만 허용
- 힌트 연속 수신 시:
  - `deviceHint.debounceMs`(기본 500ms) 디바운스
  - 마지막 힌트 reason만 보존하고 1회만 실행
- probe 실행 중 힌트가 오면 `probeRequested=true`로 표시하고, probe 종료 직후 1회 추가 실행(최대 1회)

### 4.6 Boothy 코드 맵(구현 위치를 고정해서 허점 제거)

> 스토리에서 “어디를 고쳐야 하는지”가 애매하지 않도록, 변경 포인트를 파일 단위로 고정합니다.

Sidecar(.NET):
- 모드 결정(실카메라/목카메라) + getStatus watchdog: `apps/camera-sidecar/Program.cs`
  - 요구사항: startup 시점에 `mode`와 EDSDK 진단 정보를 **statusChanged(reason=startup)** 로 1회 push(개발/현장 식별성)
- Canon 이벤트/펌프/핸들러: `apps/camera-sidecar/Camera/RealCameraController.cs`
  - 이미 존재: `EdsSetCameraAddedHandler` + `HandleCameraAddedEvent`, `HandleStateEvent(Shutdown/InternalError)`, `EdsGetEvent` 펌프
  - 요구사항: headless에서도 cameraAdded가 들어오도록 `BOOTHY_EDSDK_PUMP_MODE`에 맞춰 “messageLoop/edsGetEvent/both” 실행
  - 요구사항: 힌트 수신 시 “즉시 재열거(probe) 스케줄” 및 코얼레싱/디바운스 적용
- Canon P/Invoke: `apps/camera-sidecar/Camera/Canon/EdsdkNative.cs`
  - 참고: 에러코드 상수는 제한적이므로, 초기 버전은 “반복 실패/타임아웃/재시작” 중심으로 수렴성을 확보

Tauri(Rust):
- sidecar 이벤트 브릿지: `apps/boothy/src-tauri/src/camera/ipc_client.rs`
  - 요구사항: `event.camera.statusChanged` → `boothy-camera-status` emit

Frontend(React):
- 카메라 램프/상태 표면화: `apps/boothy/src/App.tsx`(또는 상태를 사용하는 UI 컴포넌트)
  - 요구사항: 램프는 `boothy-camera-status` 스냅샷으로 렌더링(힌트는 폴백)

IPC 계약:
- 프로토콜 버전 문자열은 sidecar 로그에 이미 표기됨: `apps/camera-sidecar/Program.cs`(Protocol Version 로그)
  - 요구사항: `event.camera.statusChanged` 추가에 따른 **minor bump** 및 후방 호환 전략 문서화

---

## 5. Tauri/Frontend 적용 스펙(최소 변경 원칙)

### 5.1 Tauri backend
- sidecar `event.camera.statusChanged` 수신 → `boothy-camera-status` 이벤트로 그대로 emit
- 기존 `boothy-camera-status-hint`는 유지(후방 호환/폴백)

### 5.2 UI(React)
- 램프는 `boothy-camera-status`만으로 렌더링
- `boothy-camera-status-hint`는 “필요 시 getStatus 1회” 트리거만 수행(디바운스)

---

## 6. 파라미터(기본값 권장)

- `deviceHint.debounceMs`: 500
- `probe.timeoutMs`: 2000
- `pump.boostWindowMs`: 10000
- `pump.lowIntervalMs`: 500
- `pump.highIntervalMs`: 100~200
- `statusChanged.minIntervalMs`: 250

---

## 7. 검증 시나리오(스토리 Minimum Validation로 그대로 사용)

시나리오 A(OFF→ON):
1) 카메라 OFF
2) Boothy 실행(빨강 확인)
3) 카메라 ON
4) 10초 내 초록 복귀
5) 로그에서 “hint → probe → statusChanged(ready)” 흐름 확인

시나리오 B(ON→OFF→ON):
1) 카메라 ON, Boothy 실행(초록 확인)
2) 카메라 OFF(1초 내 빨강)
3) 카메라 ON(10초 내 초록 복귀)

시나리오 C(환경 오류):
1) EDSDK 아키텍처 불일치 또는 미존재
2) Boothy 실행
3) 램프는 빨강 + admin 진단에 `mode`, `sdk.diagnostic`가 즉시 표시

---

## 8. 롤아웃/리스크 완화

- `BOOTHY_EDSDK_PUMP_MODE`로 현장/개발 리그별 안전한 모드 선택
- `statusChanged` 미지원 조합에서도 기존 폴링/힌트로 동작(후방 호환)

---

## 9. 라이선스/클린룸(반드시 준수)

`reference/camerafunction/digiCamControl-2.0.0`에는 GPLv3 라이선스가 포함되어 있습니다.
따라서:
- “코드”는 복사하지 않는다
- “동작 흐름”을 본 문서로 스펙화하여 새 코드로 구현한다

---

## 부록 A: 기존 초안(참고)

## A0. (기존 초안) 배경 / 문제 정의

본 문서는 Boothy에서 **카메라 전원 사이클(OFF→ON / ON→OFF→ON)** 이후 **UI 램프(초록/빨강)가 실시간으로 갱신되지 않는 문제**를 근본적으로 해결하기 위한 “실시간 상태 반영” 설계를 정리합니다.

- 이슈/시도 이력: `docs/problem_history/2026-01-30-camera-lamp-power-cycle-stuck.md`
- 현재 구성(요약)
  - UI: React (`apps/boothy/src/*`)
  - Backend: Tauri(Rust) + tokio (`apps/boothy/src-tauri/src/camera/*`)
  - Camera Sidecar: .NET 콘솔 프로세스 + Named Pipe 서버 (`apps/camera-sidecar/*`)
  - Canon 제어: EDSDK(아키텍처/번들링 제약 존재)

핵심 관찰(이력 문서 기반):
- OFF 감지는 비교적 잘 되나, **OFF→ON 재연결 감지가 누락**되거나,
- `camera.getStatus` 요청/응답 경로가 끊기거나,
- Sidecar가 **`Mode: mock`** 으로 실행되어 물리 이벤트가 반영되지 않는 케이스가 존재합니다.

이 문서는 “실시간 반영”을 **한 가지 신호(EDSDK 이벤트/폴링/IPC)만으로 달성하지 않고**, 여러 신호를 결합해 **상태를 “관측(Observe) → 판정(Decide) → 전파(Notify) → 복구(Recover)”** 하는 구조로 재구성합니다.

---

## A1. 목표 / 비목표

### 목표
1. 사용자는 카메라 전원 ON/OFF를 하면 **UI 램프가 1초 내**에 상태 변화를 느낄 수 있어야 합니다(이벤트 기반, 단 폴백 포함).
2. 카메라 OFF 상태에서 앱을 켜도, 카메라를 ON 하면 **앱 재시작/수동 새로고침 없이** 정상 복구해야 합니다.
3. 전원 사이클 중 EDSDK가 불안정/멈춤/무응답이 되어도, **자동 복구(재시작/리셋)로 수렴**해야 합니다.
4. “실제 촬영 가능”과 UI 표시가 지속적으로 일치하도록, **상태 스냅샷(정답)** 과 **상태 힌트(트리거)** 를 분리합니다.

### 비목표(현 단계)
- macOS/Linux 지원
- Canon 외 타 제조사 지원 범위 확대
- 고급 촬영 상태(셔터/AF/저장 위치)까지의 완전한 실시간 스트리밍

---

## A2. “잘 동작하는 OSS/레퍼런스”에서 가져올 핵심 아이디어

### 2.1 콘솔/서비스 환경에서 EDSDK 콜백이 안 오는 문제(메시지 펌프/이벤트 펌프)
EDSDK는 GUI 앱(WPF/WinForms)에서는 “자연스럽게” 콜백이 오는 것처럼 보이지만, **콘솔/서비스(Headless) 환경**에서는 이벤트가 자동으로 처리되지 않아 **별도 펌프가 필요**한 것으로 널리 알려져 있습니다.

Boothy는 sidecar가 콘솔 프로세스이므로, 다음 중 하나를 반드시 갖춰야 합니다:
- **(A) `EdsGetEvent()` 루프(이벤트 펌프)**: 일정 주기로 `EdsGetEvent`를 호출해 콜백이 소진되도록 함
- **(B) STA 스레드 + Windows 메시지 루프(숨은 윈도우)**: EDSDK가 요구하는 메시지 펌프 환경 제공

현재 Boothy sidecar는 (A) 형태를 이미 가지고 있으나, “언제 펌프를 돌릴지” 조건이 보수적이라 **카메라 OFF 상태에서 시작한 뒤 ON 되는 시나리오에서 이벤트/재열거가 늦거나 누락**될 수 있습니다.

### 2.2 USB/PNP 변화 감지(디바이스 레벨 이벤트를 ‘힌트’로 활용)
카메라(USB PTP 장치)의 연결/해제는 OS 레벨에서 먼저 감지됩니다. OSS/예제들은 대체로 다음 기법을 사용합니다:
- WMI 기반 감시(간단하지만 지연/노이즈 가능)
- Win32 장치 알림(더 직접적이지만 구현 복잡/윈도우 핸들 필요)

핵심은 “이 이벤트를 곧바로 UI 상태로 **단정**하지 말고”,
**(1) 상태 스냅샷 재조회 트리거(힌트)** 로 사용하고,
**(2) EDSDK 리셋/재열거 시도**를 앞당기는 데 씁니다.

### 2.3 이벤트와 스냅샷의 역할 분리(상태 동기화의 정석)
많은 OSS/실서비스에서 “상태”는 아래처럼 분리합니다.
- **Hint/Event(엣지):** “뭔가 바뀌었을 수 있음”을 빠르게 알림(저비용/저정확)
- **Snapshot(State):** 지금 상태의 정답(고비용/정확)

Boothy에도 그대로 적용합니다:
- Sidecar는 `event.camera.statusHint`(엣지)와 별개로,
- “상태 변화가 관측될 때만” `event.camera.statusChanged`(스냅샷 요약)를 push 합니다.
- Tauri는 이를 UI 이벤트로 전달하여, 프론트가 굳이 `camera.getStatus`를 재호출하지 않아도 램프가 갱신되게 합니다(물론 폴백은 유지).

---

## A3. 제안 아키텍처(TO-BE): Camera Status Reconciler

### 3.1 개요
Sidecar 내부에 “카메라 상태 판정기(Reconciler)”를 두고,
여러 신호를 통합하여 **단일 상태 머신**으로 수렴시킵니다.

**입력 신호(Observe)**
1. EDSDK 이벤트: shutdown/internalError/cameraAdded(가능 시)
2. EDSDK 프로브: `EdsGetCameraList` 기반 카메라 수/모델 감지(현재 `camera.getStatus` 경로)
3. OS 디바이스 변화(선택): USB/PNP added/removed(WMI 또는 Win32)
4. IPC/프로세스 상태: Named Pipe 연결 여부, 요청 타임아웃

**출력(Notify)**
- `event.camera.statusHint` (즉시 트리거용)
- `event.camera.statusChanged` (UI가 바로 렌더링 가능한 스냅샷)

**복구(Recover)**
- 경량 복구: 세션 재오픈, 이벤트 펌프 재가동
- 강한 복구: `EdsTerminateSDK → EdsInitializeSDK` 리셋
- 최후 복구: sidecar 프로세스 종료(이미 적용된 timeout 기반 self-terminate)

### 3.2 상태 모델(권장)
UI 램프는 단순 ON/OFF만 보이더라도, 내부적으로는 더 많은 상태가 있어야 안정적으로 동작합니다.

권장 상태(내부):
- `Disconnected` : IPC/sidecar 미연결(백엔드 레벨)
- `NoCamera` : sidecar는 살아있지만 카메라 미감지
- `Connecting` : 재열거/리셋/세션오픈 시도 중
- `Ready` : 촬영 가능한 상태(카메라 감지 + 세션 정상)
- `Error` : 명시적 오류(아키텍처 미스매치/권한/SDK 미존재 등)

UI로 전달하는 최소 스냅샷(예):
```json
{
  "connected": true,
  "cameraDetected": false,
  "cameraReady": false,
  "mode": "real",
  "reason": "pnpAdded|cameraAdded|probe|shutdown|ipcTimeout|sdkReset",
  "observedAt": "2026-01-31T00:00:00.000Z",
  "seq": 123
}
```

### 3.3 데이터 플로우(메시지 흐름)
```mermaid
flowchart LR
  UI[React UI] <-- tauri event --> TB[Tauri Backend]
  TB <-- Named Pipe --> SC[Camera Sidecar]

  subgraph Sidecar 내부
    PNP[OS PNP Watcher\n(WMI/Win32 선택)] --> R[Status Reconciler]
    EVT[EDSDK Event Pump\n(EdsGetEvent / Message Loop)] --> R
    PROBE[EDSDK Probe\n(GetCameraList/OpenSession)] --> R
  end

  R -->|event.camera.statusHint| SC
  R -->|event.camera.statusChanged| SC
  SC -->|Named Pipe event| TB
  TB -->|boothy-camera-status-hint| UI
  TB -->|boothy-camera-status| UI
```

---

## A4. Boothy에 맞춘 구체 설계(변형 포인트)

### 4.1 Sidecar: 이벤트 펌프 정책을 “항상(저주기)” + “가속(고주기)”로 변경
현재: `apps/camera-sidecar/Camera/RealCameraController.cs`의 `StartEventPumpUnsafe`는
`sessionOpen || hotplugWatchActive`일 때만 `EdsGetEvent()`를 호출합니다.

제안:
- **기본(Always-low)**: SDK init 이후에는 카메라 유무와 상관없이 저주기(예: 500ms~1000ms)로 `EdsGetEvent`를 호출
- **가속(Boost)**: 아래 이벤트가 발생하면 일정 시간(예: 10초) 동안 고주기(예: 50~200ms)로 가속
  - PNP Added/Removed
  - shutdown/internalError
  - probe에서 0→1 이상(재연결) 또는 1→0(단절) 관측

안전장치:
- `EdsGetEvent`가 특정 에러코드를 반환할 때(“카메라 없음/바쁜 상태”)는 **즉시 리셋/종료로 몰지 말고**, backoff로 흡수
- `BOOTHY_EDSDK_EVENT_PUMP_MODE` 플래그로 “always / sessionOrHotplug” 전환 가능하게(현장 안정성 확보)

### 4.2 Sidecar: 프로브 기반 “상태 변화”를 먼저 확정하고 push
현 구조는 UI가 힌트를 받으면 다시 `camera.getStatus`를 호출해서 상태를 갱신하는 형태가 섞여 있습니다.

제안:
- `GetStatus()` 내부 프로브 결과에서 **상태 변화(예: cameraCount 0→>0, >0→0)** 를 감지하면:
  - `event.camera.statusHint`(트리거) 뿐 아니라
  - `event.camera.statusChanged`(스냅샷)을 즉시 emit
- 이렇게 하면, UI는 힌트가 아니라 **스냅샷 이벤트만으로 램프를 갱신**할 수 있습니다.

### 4.3 Sidecar: OS 디바이스 변화 감지를 “힌트”로 추가(선택이지만 강력 권장)
선택지 2개:
1. **WMI(ManagementEventWatcher)**: 구현 단순, sidecar 콘솔에 적합
2. **Win32 장치 알림(RegisterDeviceNotification / CM_Register_Notification)**: 더 직접적이지만 구현 복잡(윈도우 핸들/서비스 핸들 필요)

권장 초기안: WMI로 시작 → 현장 지연/누락이 크면 Win32로 승격.

PNP 이벤트가 오면:
- `hotplugWatchActive=true` / event pump boost
- 즉시 `probe` 수행 스케줄
- `event.camera.statusHint` reason=`pnpAdded|pnpRemoved` 발행

### 4.4 Tauri Backend: “힌트 이벤트”와 “상태 스냅샷 이벤트”를 분리해 UI로 전달
현재도 backend 폴러가 `boothy-camera-status-hint`를 발행합니다(`apps/boothy/src-tauri/src/camera/ipc_client.rs`).

제안:
- Sidecar의 `event.camera.statusChanged`를 수신하면
  - 그대로 `boothy-camera-status` 이벤트로 UI에 전달
  - backend 내부 diagnostics에도 스냅샷을 기록
- UI가 상태표시에 필요한 정답은 `boothy-camera-status`에서 받도록 유도
- `boothy-camera-status-hint`는 폴백으로만 유지(호환성/디버깅)

### 4.5 Frontend(UI): “렌더는 스냅샷, 트리거는 힌트” 원칙으로 단순화
현재의 `listen('boothy-camera-status-hint', ...)` 기반 “즉시 getStatus 재호출” 패턴은,
이벤트 폭주/중복 호출/인플라이트 고착 문제를 만들기 쉽습니다.

제안:
- 램프는 `boothy-camera-status`(스냅샷)만으로 렌더링
- `boothy-camera-status-hint`를 받으면:
  - (옵션) 200ms~500ms 디바운스로 `camera.getStatus`를 1회 호출(폴백)
  - 단, 램프 렌더링 자체는 스냅샷 이벤트가 오면 즉시 갱신되므로, 이 호출은 “치유성”에만 기여

---

## A5. 프로토콜 변경(권장)

### 5.1 Sidecar → Tauri 이벤트
- 기존: `event.camera.statusHint`
- 추가: `event.camera.statusChanged`

`event.camera.statusChanged`의 payload는 `camera.getStatus` 응답의 축약판이어도 좋습니다.
중요한 것은 UI가 “다시 pull하지 않아도” 램프를 갱신할 수 있는 수준의 정보입니다.

### 5.2 버전/호환
- `protocolVersion`을 minor bump 하되,
- Tauri가 `statusChanged`를 모르는 구버전 sidecar와도 동작하도록(이벤트 없으면 기존 폴링/힌트만 사용)

---

## A6. 리스크 / 대응

1. **EDSDK의 `EdsGetEvent` 무세션 호출 안정성**
   - 대응: always-low → 특정 에러코드는 무시/backoff → 문제가 있으면 플래그로 구동 모드 전환
2. **x86/x64 아키텍처 미스매치로 real 모드 불가(Mode: mock)**
   - 대응: 배포/번들링 문서(기존 `docs/architecture/infrastructure-and-deployment-integration.md`)를 “실제 패키징 산출물”과 일치시키고,
     시작 시 sidecar가 자신의 아키텍처/EDSDK 로드 성공 여부를 **statusChanged(reason=diagnostic)** 로 즉시 알림
3. **이벤트 폭주/중복**
   - 대응: 디바운스(프론트), 스로틀(백엔드/sidecar), seq 기반 최신 승자(last-write-wins)

---

## A7. 구현 단계(권장 순서)

1. Sidecar: `event.camera.statusChanged` 추가(프로브 기반 전이 0→>0 / >0→0 우선)
2. Tauri: `event.camera.statusChanged`를 `boothy-camera-status`로 브릿지
3. UI: 램프 렌더링을 `boothy-camera-status` 기반으로 전환(힌트는 폴백)
4. Sidecar: 이벤트 펌프 정책을 always-low + boost로 개선
5. Sidecar: OS PNP watcher(WMI) 추가 및 hint/boost 연결
6. 현장 리그에서 전원 사이클/USB 재열거 반복 테스트로 파라미터 튜닝(backoff/timeout/boost window)

---

## A8. (Architect `research`) Deep Research Prompt (기록용)

> 아래 프롬프트는 “오픈소스/레퍼런스 기반으로 카메라 상태 실시간 반영을 설계”하기 위한 조사 질문을 재사용할 수 있게 남깁니다.

```markdown
## Research Objective
Windows + Canon EDSDK + headless(.NET) sidecar 환경에서, 카메라 연결 상태(ON/OFF, 재연결)를 UI에 1초 내 반영하기 위한 검증된 오픈소스/레퍼런스 패턴을 조사하고, Boothy의 Tauri+NamedPipe 구조에 맞는 설계로 변형한다.

## Background Context
- Boothy: React UI + Tauri backend(Rust) + Camera sidecar(.NET) + Named Pipe IPC
- 이슈: 전원 사이클 후 UI 램프가 빨강에 고정되는 케이스(OFF→ON, ON→OFF→ON)
- 제약: Canon EDSDK(아키텍처/이벤트 모델), headless 콘솔 프로세스

## Research Questions
### Primary Questions (Must Answer)
1. EDSDK 콜백/핫플러그 이벤트가 headless 콘솔에서 누락되는 원인(메시지 펌프/이벤트 펌프 요구사항)은 무엇인가?
2. EDSDK 기반 OSS(예: digiCamControl 계열, EDSDK.NET 등)는 카메라 연결/해제/재연결을 어떻게 감지/복구하는가?
3. Windows에서 USB/PNP 변화 감지(WMI/Win32 notification)를 안정적으로 운영하는 OSS 패턴은 무엇인가?
4. 이벤트 기반 힌트 + 상태 스냅샷을 결합하는 “상태 동기화” 아키텍처의 모범 사례는 무엇인가?

### Secondary Questions (Nice to Have)
1. 에러코드/타임아웃/재시작 정책을 어떻게 설계해야 현장에서 수렴성이 좋은가?
2. UI가 상태를 pull(getStatus)하지 않고도 push만으로 안정적으로 갱신되게 하는 패턴은?

## Research Methodology
- 공식 문서(Win32 device notification)
- OSS/레퍼런스 코드(EDSDK 기반, USB watcher)
- 현장 재현 테스트 로그 패턴과의 매핑

## Expected Deliverables
- 설계 요약(결정/대안/리스크)
- Boothy 적용 설계(컴포넌트/프로토콜/상태모델/복구정책)
- 구현 순서 및 검증 시나리오
```

---

## A9. 참고(외부 레퍼런스)

아래 URL은 설계 근거를 빠르게 다시 확인하기 위한 포인터입니다.
(문서 내에 코드를 직접 복사하기보다는, 패턴/요구사항을 참고하는 용도로만 사용 권장)

- Canon EDSDK 콘솔 환경에서 `EdsGetEvent` 루프 필요성(이벤트 처리): `https://stackoverflow.com/questions/64646556/canon-edsdk-13-11-10-not-saving-to-host-pc`
- EDSDK 이벤트/메시지 펌프 관련 논의: `https://stackoverflow.com/questions/18360402/no-callback-from-event-handler-canon-sdk-2-12`
- Win32 디바이스 알림(RegisterDeviceNotification): `https://learn.microsoft.com/en-us/windows/win32/devio/registering-for-device-notification`
- OSS 예시(Win32 기반 USB 감지): `https://github.com/byGeek/UsbDetector`
- OSS 예시(WMI 기반 USB 감지, MIT): `https://github.com/Jinjinov/Usb.Events`
