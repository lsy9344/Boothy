# Component Architecture

## New Components

### Boothy Session Manager (Tauri Backend)

**Responsibility:** 세션 생성/선택/종료를 관리하고, “활성 세션 1개” 정책(FR3)을 강제합니다. 세션 폴더 구조(`Raw/`, `Jpg/`)를 생성하고, UI/카메라 사이드카/파일 감지 컴포넌트에 “현재 세션 경로”를 배포합니다.

**Integration Points:** 세션 루트(`boothy.sessionsRoot`) 아래에 세션 폴더를 생성/활성화하고, RapidRAW의 라이브러리 루트/현재 폴더를 `Raw/`로 고정합니다. 카메라 사이드카에는 “저장 대상 폴더 = Raw/”를 전달합니다.

**Key Interfaces:**
- `tauri::command boothy_create_or_open_session(sessionName: string) -> Session`
- `tauri::command boothy_set_active_session(sessionFolderName: string) -> Session`
- `tauri::command boothy_get_active_session() -> Option<Session>`
- `event boothy-session-changed { session }`

**Dependencies:**
- **Existing Components:** RapidRAW settings (`settings.json`, `AppSettings.lastRootPath` 등), RapidRAW folder tree/listing commands
- **New Components:** Boothy File Arrival Watcher, Boothy Camera IPC Client, Boothy Mode/Auth

**Technology Stack:** Rust + Tauri backend, Windows path resolution(가능하면 KnownFolder(Pictures) 기반), tokio

### Boothy Camera IPC Client (Tauri Backend)

**Responsibility:** Camera Sidecar Service에 연결/제어(촬영/상태/설정)하고, 카메라 이벤트(연결 상태, 오류, 촬영 완료/전송 완료)를 Boothy 앱 이벤트로 변환합니다.

**Integration Points:** Named Pipe 기반 IPC로 sidecar를 제어합니다. IPC 장애 시 자동 재연결/재시작을 수행하고, UI에 actionable error 상태를 전달합니다(FR20, NFR8).

**Key Interfaces:**
- `connect()`, `disconnect()`, `get_state()`
- `set_session_destination(rawPath: string)`
- `capture()` / `capture_burst(...)`
- `set_property(key, value)` / `get_properties()` / `list_capabilities()`
- `event camera-state { connected, model, error? }`
- `event camera-photo-transferred { path }`

**Dependencies:**
- **Existing Components:** (참고) digiCamControl Named Pipe 패턴 (`DCCPipe`)
- **New Components:** Camera Sidecar Service, Boothy Session Manager, Logging/Diagnostics

**Technology Stack:** Rust + tokio, Windows Named Pipe(버전드 JSON 메시지 권장)

### Camera Sidecar Service (C#/.NET, Headless)

**Responsibility:** Canon 카메라 연결/제어/촬영/파일 전송을 수행하고, 전송 완료된 파일을 “활성 세션 Raw/”에 저장합니다. Boothy가 요구하는 “카메라 기능 100%”를 sidecar 내부에서 충족시키는 것을 목표로 합니다(FR19, FR21).

**Integration Points:** digiCamControl의 디바이스/SDK 레이어(EDSDK 포함)를 재사용하여, 제품 UI(WPF)를 포함하지 않고 기능만 제공하는 headless 프로세스로 구성합니다. Boothy(Tauri)가 session destination을 변경하면, sidecar는 이후 촬영 결과를 해당 폴더로 저장합니다.

**Key Interfaces:**
- `IPC server: \\.\pipe\\BoothyCamera` (예시)
- `cmd: setSessionDestination`, `cmd: capture`, `cmd: listCapabilities`, `cmd: setProperty`, `cmd: getState`
- `evt: photoTransferred { path }`, `evt: error { code, message }`, `evt: connected/disconnected`

**Dependencies:**
- **Existing Components:** `reference/camerafunction/digiCamControl-2.0.0/*` (기능/패턴 레퍼런스)
- **New Components:** (없음; sidecar는 독립 프로세스)

**Technology Stack:** C#/.NET(초기에는 digiCamControl 재사용 용이성을 우선), Windows Named Pipe, log4net(또는 단순 파일 로그)

### Boothy File Arrival Watcher (Tauri Backend)

**Responsibility:** 활성 세션의 `Raw/`를 감시하여 “전송 완료된 신규 파일”만을 import 대상으로 확정합니다. 파일 안정화(stabilization) 체크로 partial transfer/락 파일을 배제하여 데이터 무결성(NFR5)을 보장합니다.

**Integration Points:** sidecar의 `photoTransferred` 이벤트를 1차 신호로 사용하되, 파일 시스템 watcher(`notify`)를 폴백으로 둡니다. 확정된 신규 파일에 대해 “프리셋 스냅샷 저장(.rrdata)”와 “UI 갱신 이벤트”를 트리거합니다.

**Key Interfaces:**
- `start_watch(rawPath: string)`
- `stop_watch()`
- `event boothy-new-photo { path }`
- `event boothy-import-error { path?, reason }`

**Dependencies:**
- **Existing Components:** RapidRAW의 파일 읽기/락 감지 패턴(예: `try_lock_shared`), 이미지 로딩/썸네일 파이프라인
- **New Components:** Boothy Preset Assignment Service, Boothy Session Manager

**Technology Stack:** Rust + tokio, `notify`, 파일 안정화(사이즈 변화/락 상태/최소 크기/최소 시간)

### Boothy Preset Assignment Service (Tauri Backend)

**Responsibility:** 현재 선택된 프리셋을 “신규 유입 사진에만” 적용하기 위해, 파일 도착 시점의 프리셋 adjustments를 스냅샷으로 `.rrdata`에 저장합니다(FR8–FR10). 프리셋 변경은 이후 사진에만 영향(FR9)이며, 기존 사진은 수정하지 않습니다.

**Integration Points:** UI에서 선택된 RapidRAW `Preset(adjustments)`를 전달받아 메모리에 유지하고, 신규 파일 확정 시 `.rrdata`의 `ImageMetadata.adjustments`에 스냅샷을 저장합니다.

**Key Interfaces:**
- `tauri::command boothy_set_current_preset(presetId: string, presetAdjustments: json)`
- `apply_preset_snapshot_to_image(path: string)`

**Dependencies:**
- **Existing Components:** RapidRAW Preset 모델(`Preset.adjustments`), RapidRAW `.rrdata`(ImageMetadata)
- **New Components:** Boothy File Arrival Watcher

**Technology Stack:** Rust + serde_json

### Boothy Mode/Auth (Tauri Backend + React UI)

**Responsibility:** customer/admin 모드 전환(토글 → 비밀번호)을 제공하고(FR16), customer 모드에서는 고급 기능을 “숨김” 처리합니다(FR17). 비밀번호는 argon2로 해시 저장하고 평문 저장/로그를 금지합니다(NFR6).

**Integration Points:** 모드 변화 이벤트를 UI에 전달하여 RapidRAW의 `ui_visibility`/`adjustment_visibility` 및 Boothy 전용 컴포넌트 렌더링을 제어합니다.

**Key Interfaces:**
- `tauri::command boothy_admin_login(password: string) -> { success }`
- `tauri::command boothy_set_mode(mode: 'customer'|'admin')`
- `event boothy-mode-changed { mode }`

**Dependencies:**
- **Existing Components:** RapidRAW 설정 저장(`settings.json`), UI visibility 관련 설정(`ui_visibility`, `adjustment_visibility`)
- **New Components:** 없음(상태/정책 컴포넌트)

**Technology Stack:** Rust(argon2) + React(모드 토글/비밀번호 입력 UI)

### Boothy UI Extensions (React)

**Responsibility:** RapidRAW UI 위에 “세션 시작(세션 이름 입력)”, “촬영(셔터)”, “카메라 상태”, “모드 토글”을 추가하고, customer 모드에서 필요한 최소 UI만 남기도록 재구성합니다.

**Integration Points:** 기존 RapidRAW의 폴더 선택/이미지 리스트/프리셋 패널/Export 기능을 재사용하되, 세션 모드에서는 “현재 폴더=활성 세션 Raw/”로 고정하고 신규 사진 이벤트 시 자동 refresh + 자동 선택(메인 뷰 즉시 표시)을 수행합니다.

**Key Interfaces:**
- `listen('boothy-new-photo', ...) -> refreshImageList() + selectImage(path)`
- `invoke('boothy_create_or_open_session', ...)`
- `invoke('boothy_capture', ...)` *(또는 camera client command)*

**Dependencies:**
- **Existing Components:** RapidRAW `App.tsx` 이미지 리스트/선택/프리뷰 이벤트 루프, Presets UI, Export UI
- **New Components:** Boothy Session Start UI, Mode Toggle UI, Camera Status Banner

**Technology Stack:** React + TypeScript, Tauri event/invoke

## Component Interaction Diagram

```mermaid
graph TD
  U[User] --> UI[Boothy UI (React/RapidRAW 기반)]
  UI -->|invoke/listen| TB[Tauri Backend (Rust)]

  subgraph SessionFS[Filesystem]
    ROOT[%USERPROFILE%\\Pictures\\dabi_shoot]
    RAW[Active Session\\Raw\\]
    JPG[Active Session\\Jpg\\]
    RR[.rrdata sidecars]
    ROOT --> RAW
    ROOT --> JPG
    RAW --> RR
  end

  TB --> SM[Boothy Session Manager]
  TB --> AUTH[Boothy Mode/Auth]
  TB --> PA[Preset Assignment Service]
  TB --> FW[File Arrival Watcher]
  TB --> CC[Camera IPC Client]

  CC -->|Named Pipe| CS[Camera Sidecar Service (.NET)]
  CS --> CAM[Canon Camera]
  CS -->|write RAW| RAW

  FW -->|detect stable file| RAW
  FW -->|new photo path| PA
  PA -->|write preset snapshot| RR
  TB -->|emit boothy-new-photo| UI

  UI -->|Export image| TB
  TB -->|write JPG outputs| JPG
```

이 컴포넌트 경계(Boothy backend orchestration + Camera sidecar + filesystem contract)는 RapidRAW의 command/event + `.rrdata` 패턴과, digiCamControl의 IPC/이벤트 기반 카메라 패턴을 결합하는 TO‑BE 구조입니다.
