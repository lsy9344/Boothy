# RapidRAW 카메라 접속 감지 방식 작업 기록

## 작업 범위
- 대상: `upstream/RapidRAW/src-tauri` (Tauri + Rust)
- 주제: Windows 카메라 물리 연결 감지(`WM_DEVICECHANGE`) 흐름과, 해당 흐름에 연결된 자동 연결/정리(cleanup) 로직

## 감지 방식(연구/정리)
### 1) 이벤트 소스: `WM_DEVICECHANGE` + 메시지 전용 윈도우
- Windows에서 메시지 전용(hidden) 윈도우를 생성해 `WM_DEVICECHANGE`를 수신합니다.
- `RegisterDeviceNotificationW`로 장치 인터페이스 클래스 변화 알림을 구독합니다.
  - `GUID_DEVINTERFACE_IMAGE` (Imaging)
  - `GUID_DEVINTERFACE_USB_DEVICE` (USB)

### 2) 카메라 판별 규칙(필터링)
- 장치 인터페이스 path/name에 Canon USB Vendor ID 문자열 `VID_04A9`(대소문자 무시)가 포함되면 “카메라 관련 이벤트”로 간주합니다.
- `DBT_DEVNODES_CHANGED`는 식별자(디바이스 path)가 없으므로, SetupAPI 재열거를 트리거해 “현재 연결 상태 스냅샷”을 얻습니다.
- 설계 의도: “아무 USB 변화나 다 감지”가 아니라 Canon 카메라 관련 변화만 보수적으로 감지하여 불필요한 상태 플래핑을 줄입니다.

### 3) 연결 여부 스냅샷(열거)
- SetupAPI로 현재 장치 인터페이스를 열거해 Canon VID 포함 여부로 연결 상태를 판단합니다.
  - `SetupDiGetClassDevsW` → `SetupDiEnumDeviceInterfaces` → `SetupDiGetDeviceInterfaceDetailW`
- Imaging GUID를 우선으로 확인하고, 없으면 USB GUID로 fallback 합니다.

### 4) 컨트롤러 반영 흐름
- 앱 시작 시 `main.rs`에서 `TetheringController::start_physical_connection_watcher()` 호출(Windows only).
- watcher는 `PhysicalCameraEvent`를 전송:
  - `Enumerated { connected }` (시작/재열거 스냅샷)
  - `DeviceArrived/DeviceRemoved { device_path }`
  - `DevNodesChanged`
- 컨트롤러는 이벤트를 받아:
  - `physical_connected` 갱신
  - `CameraStatus` 계산 및 `camera-status-changed` emit
  - 필요 시 disconnect cleanup(백그라운드 teardown) / auto-connect 스폰

## 발생한 컴파일 에러
- 증상: `npm run tauri dev` 중 Rust 컴파일 실패
- 에러 요약: `future cannot be sent between threads safely` (spawn되는 async 블록이 `Send`가 아님)
- 위치: `upstream/RapidRAW/src-tauri/src/tethering/controller.rs`의 disconnect cleanup 태스크 스폰 구간

## 원인(근본 원인)
- `tauri::async_runtime::spawn`은 `Future + Send + 'static`을 요구합니다.
- disconnect cleanup 태스크 내부에서 `controller.apply_physical_connection_state(true).await`를 호출했는데,
  해당 `await`가 non-`Send` future를 포함하면서 스폰되는 async 블록 전체가 `Send` 조건을 만족하지 못해 컴파일이 실패했습니다.

## 수정/적용 내용
- disconnect cleanup 태스크 안에서 `apply_physical_connection_state()`를 재호출/`await` 하지 않도록 변경했습니다.
- cleanup 완료 후 “재플러그(물리 연결 true)”가 감지되면, 아래를 cleanup 태스크 내부에서 직접 수행하도록 인라인 처리했습니다.
  - kiosk config에서 EDSDK 경로를 읽고/정규화하고/검증하여 “Ready/Invalid/NotConfigured”를 판정
  - UI 상태(`CameraStatus`)를 `ConnectedNotReady` 또는 `Connecting`으로 갱신 후 `camera-status-changed` emit
  - EDSDK가 준비된 경우에만 `connect(root)`를 별도 태스크로 스폰(자동 재연결)
- 변경 파일:
  - `upstream/RapidRAW/src-tauri/src/tethering/controller.rs`

## 이유(왜 이렇게 고쳤는가)
- 가장 작은 범위로 `Send` 제약을 만족시키기 위해, spawn된 태스크가 non-`Send` future를 `await` 하지 않도록 구조를 바꿨습니다.
- `apply_physical_connection_state()`를 강제로 `Send`로 만들거나(local runtime/`spawn_local` 등) 런타임 구조를 바꾸는 방법 대비,
  영향 범위가 작고(해당 cleanup 경로만 변경) 기존 UX/자동 재연결 의도를 유지하기 쉽습니다.

## 결과
- `cargo check --no-default-features` 통과(컴파일 에러 제거).
- disconnect cleanup 도중 카메라가 재연결된 케이스에서, cleanup 완료 후 자동 재연결 시도 및 상태 업데이트 흐름을 유지합니다.

## 검증 방법
- Rust만 빠르게 확인: `cd upstream/RapidRAW/src-tauri; cargo check --no-default-features`
- 전체 dev 실행: `cd upstream/RapidRAW; npm run tauri dev`

## 참고 코드
- 물리 감지 watcher: `upstream/RapidRAW/src-tauri/src/tethering/physical_connection_watcher.rs`
- 컨트롤러 상태/자동연결/cleanup: `upstream/RapidRAW/src-tauri/src/tethering/controller.rs`
- 초기화 지점(Windows): `upstream/RapidRAW/src-tauri/src/main.rs`

