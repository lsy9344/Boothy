# API Design and Integration

## API Integration Strategy

**API Integration Strategy:** 본 제품은 서버/HTTP 기반이 아니라 **로컬(오프라인) 앱**이므로, “API”는 다음 2개 레이어의 **로컬 RPC 계약**으로 정의합니다.
1. **UI(React) ↔ Tauri Backend(Rust):** `tauri::command`(invoke) + `emit`(event) 기반의 in-app API
2. **Tauri Backend(Rust) ↔ Camera Sidecar(.NET):** Windows **Named Pipe** 기반의 sidecar control API (권장: JSON-RPC 스타일)

추가로, 촬영/편집 통합의 핵심 계약은 “API”가 아니라 **파일시스템 세션 폴더 계약**(세션 `Raw/`, `Jpg/`)입니다.

**Authentication:** 네트워크 인증은 없고(오프라인), 보안 요구는 다음으로 제한합니다.
- **Admin 모드 인증:** `argon2id` 해시+salt 저장(NFR6), 평문 저장/로그 금지
- **Sidecar 접근 제어:** Named Pipe는 Windows ACL로 **현재 사용자 세션만 접근 가능**하게 제한(권장). 필요 시 sidecar 시작 시 생성한 1회용 토큰을 Tauri backend가 주고받는 handshake를 추가할 수 있습니다.

**Versioning:** IPC/커맨드 계약의 장기 유지보수를 위해 다음을 강제합니다.
- **Protocol version 필드**(정수)를 모든 메시지에 포함하고, 호환되지 않으면 명시적 에러로 실패(“업데이트 필요” 안내)
- 모든 요청은 `requestId`(UUID) + `correlationId`를 포함하여 end-to-end 로그 상관관계(NFR8)를 확보

## New API Endpoints

### Boothy UI ↔ Tauri Backend (Commands)

#### Create/Activate Session
- **Method:** `invoke`
- **Endpoint:** `boothy_create_or_open_session`
- **Purpose:** `sessionName`으로 세션 폴더를 생성/활성화(존재 시 열기)하고, RapidRAW의 현재 작업 폴더를 세션 `Raw/`로 전환(FR3/FR6)
- **Integration:** 폴더 생성(중복 규칙 포함) → `Raw/`, `Jpg/` 생성 → session 변경 이벤트 발행 → UI는 이미지 리스트를 `Raw/` 기준으로 refresh

**Request**
```json
{
  "sessionName": "Wedding"
}
```

**Response**
```json
{
  "sessionName": "Wedding",
  "sessionFolderName": "Wedding_2026_01_14_15",
  "basePath": "C:\\Users\\KimYS\\Pictures\\dabi_shoot\\Wedding_2026_01_14_15",
  "rawPath": "C:\\Users\\KimYS\\Pictures\\dabi_shoot\\Wedding_2026_01_14_15\\Raw",
  "jpgPath": "C:\\Users\\KimYS\\Pictures\\dabi_shoot\\Wedding_2026_01_14_15\\Jpg"
}
```

#### Capture (Shoot)
- **Method:** `invoke`
- **Endpoint:** `boothy_capture`
- **Purpose:** customer 모드에서도 촬영 트리거(FR5)
- **Integration:** backend가 camera client를 통해 sidecar에 촬영 요청 → 완료/전송 완료는 event로 수신 → 신규 파일은 watcher가 안정화 후 import 처리

**Request**
```json
{
  "mode": "single"
}
```

**Response**
```json
{
  "accepted": true,
  "requestId": "9c2d0c44-5c7b-4c54-9de1-8c6d5c3b8b6b"
}
```

#### Set Current Preset (Snapshot Source)
- **Method:** `invoke`
- **Endpoint:** `boothy_set_current_preset`
- **Purpose:** 현재 선택 프리셋을 “이후 신규 사진”에만 적용하기 위한 기준값으로 설정(FR8/FR9)
- **Integration:** UI의 preset 선택 → backend가 `presetId` + `presetAdjustments`(JSON) 저장 → 신규 파일 도착 시 `.rrdata`에 스냅샷 저장(FR10)

**Request**
```json
{
  "presetId": "7b2d1c5e-0d2b-4d1a-9c4c-2cbd8d0d6a11",
  "presetName": "Boothy Warm",
  "presetAdjustments": {
    "exposure": 0.4,
    "contrast": 0.1
  }
}
```

**Response**
```json
{ "ok": true }
```

#### Admin Login
- **Method:** `invoke`
- **Endpoint:** `boothy_admin_login`
- **Purpose:** 토글→비밀번호로 admin 모드 진입(FR16)
- **Integration:** password 검증(argon2) 성공 시 mode 변경 이벤트 → UI는 숨김 정책 해제

**Request**
```json
{ "password": "********" }
```

**Response**
```json
{ "success": true }
```

### Tauri Backend ↔ Camera Sidecar (Named Pipe RPC)

권장 포맷은 **JSON-RPC 스타일**이며, 단일 파이프에서 request/response + event notification을 멀티플렉싱합니다.

#### Get Camera State
- **Method:** `RPC`
- **Endpoint:** `camera.getState`
- **Purpose:** 연결 상태/기종/오류를 조회(FR20)
- **Integration:** UI 상태 배너/에러 표면화에 사용

**Request**
```json
{
  "jsonrpc": "2.0",
  "id": "b3f6d7d2-2b8d-4c02-8c0f-3b2d42dd2a11",
  "method": "camera.getState",
  "params": {},
  "meta": { "protocolVersion": 1, "correlationId": "..." }
}
```

**Response**
```json
{
  "jsonrpc": "2.0",
  "id": "b3f6d7d2-2b8d-4c02-8c0f-3b2d42dd2a11",
  "result": {
    "connected": true,
    "cameraModel": "Canon EOS R6",
    "lastError": null
  }
}
```

#### Set Session Destination (Raw Path)
- **Method:** `RPC`
- **Endpoint:** `camera.setSessionDestination`
- **Purpose:** 촬영 결과 저장 경로를 활성 세션 `Raw/`로 설정(FR6)
- **Integration:** session 변경 시 반드시 호출(세션 강제)

**Request**
```json
{
  "jsonrpc": "2.0",
  "id": "a3a8d6f2-1a2f-4b4d-9f4d-8f2c7a0b9d0e",
  "method": "camera.setSessionDestination",
  "params": {
    "rawPath": "C:\\Users\\KimYS\\Pictures\\dabi_shoot\\Wedding_2026_01_14_15\\Raw"
  },
  "meta": { "protocolVersion": 1, "correlationId": "..." }
}
```

**Response**
```json
{ "jsonrpc": "2.0", "id": "a3a8d6f2-1a2f-4b4d-9f4d-8f2c7a0b9d0e", "result": { "ok": true } }
```

#### Photo Transferred (Event Notification)
- **Method:** `Event`
- **Endpoint:** `event.camera.photoTransferred`
- **Purpose:** “전송 완료” 시점을 앱에 통지하여 실시간 반영을 돕습니다(NFR3)
- **Integration:** watcher는 이 이벤트를 1차 신호로 받고, 파일 안정화 체크 후 import 확정(NFR5)

**Payload**
```json
{
  "jsonrpc": "2.0",
  "method": "event.camera.photoTransferred",
  "params": {
    "path": "C:\\Users\\KimYS\\Pictures\\dabi_shoot\\Wedding_2026_01_14_15\\Raw\\IMG_0001.CR3",
    "capturedAt": "2026-01-14T15:02:33Z"
  },
  "meta": { "protocolVersion": 1, "correlationId": "..." }
}
```

sidecar IPC는 구현/운영 상의 버전 관리와 진단을 위해 **JSON-RPC 스타일 메시지 + `protocolVersion`/`requestId`/`correlationId`**를 표준으로 사용합니다.
