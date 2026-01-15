# Data Models and Schema Changes

## New Data Models

### BoothySession (Folder-backed Session)

**Purpose:** “세션 1개만 활성화” 정책(FR3)을 시스템적으로 강제하기 위한 세션 컨텍스트(경로/이름/시간)를 정의합니다.

**Integration:** 세션은 파일시스템 폴더로 표현되며, 세션 루트(`sessionsRoot`) 아래에 생성됩니다. UI는 활성 세션의 `Raw/`만을 “현재 작업 폴더”로 사용하여, 세션 중에는 그 폴더만 보이도록 제한합니다. (필요 시 세션 폴더 루트에 `boothy.session.json`을 추가해 세션 메타데이터를 저장할 수 있습니다.)

**Key Attributes:**
- `sessionName`: string (사용자 입력 세션 이름)
- `sessionFolderName`: string (sanitize된 폴더명, 충돌 시 `_YYYY_MM_DD_HH` suffix 가능)
- `createdAt`: string (ISO 8601)
- `basePath`: string (세션 폴더 절대 경로)
- `rawPath`: string (`basePath\\Raw`)
- `jpgPath`: string (`basePath\\Jpg`)

**Relationships:**
- **With Existing:** RapidRAW의 폴더 트리/핀 폴더/AppSettings(`lastRootPath`, `pinnedFolders`)와 연결되어 “세션 폴더만 브라우징”을 구현
- **With New:** BoothyPerImageState(사진별 프리셋/회전 상태)와 연결

### BoothyPerImageState (Stored in RapidRAW `.rrdata`)

**Purpose:** 사진별로 “프리셋이 언제/무엇으로 적용되었는지”를 **세션 내에서 영속화**하고(FR10), 프리셋 변경이 과거 사진에 영향을 주지 않도록(FR9) **스냅샷 기반**으로 저장합니다.

**Integration:** RapidRAW가 이미 사용하는 이미지 옆 `.rrdata` 사이드카(예: `IMG_0001.CR3.rrdata`)의 `ImageMetadata.adjustments`에 Boothy 전용 키를 **추가(append)**하여 저장합니다. RapidRAW는 `adjustments`를 자유형 JSON으로 취급하므로, Boothy 전용 키는 기존 처리 파이프라인을 깨지 않으면서 함께 저장될 수 있습니다.

**Key Attributes:**
- `adjustments`: object (프리셋 적용 시점의 **adjustments 스냅샷** + 회전/크롭 등 추가 조정)
- `boothyPresetId`: string (RapidRAW `Preset.id`) *(UI 표시/추적용, 스냅샷이 “정답” 소스)*
- `boothyPresetName`: string *(선택, UI 표시용)*
- `boothyAppliedAt`: string (ISO 8601)
- `rotation`: number (RapidRAW가 이미 사용하는 `rotation` 조정 키, FR14)

**Relationships:**
- **With Existing:** RapidRAW `Preset`/`PresetItem`(프리셋 원본) 및 `ImageMetadata`(사이드카 저장 포맷)
- **With New:** 세션 폴더 계약(`Raw/`)과 결합되어 “새 파일 도착 → `.rrdata` 생성/갱신 → 즉시 프리뷰/썸네일 반영”을 지원

### BoothyAppSettings (Extension of RapidRAW `AppSettings`)

**Purpose:** 앱 전역 설정(세션 루트/모드/관리자 인증/카메라 사이드카 설정 등)을 저장합니다.

**Integration:** RapidRAW가 이미 제공하는 `settings.json`(Tauri AppData, `AppSettings`)을 확장하여 Boothy 전용 설정을 저장합니다. 충돌을 피하기 위해 `boothy` 네임스페이스 객체를 사용합니다.

**Key Attributes:**
- `boothy.schemaVersion`: number
- `boothy.sessionsRoot`: string (고정 `%USERPROFILE%\\Pictures\\dabi_shoot`의 **절대 경로**, MVP에서는 변경 불가)
- `boothy.defaultMode`: `"customer"` (FR15)
- `boothy.adminPassword`: object (argon2id 파라미터 + salt + hash, NFR6)
- `boothy.cameraSidecar`: object (exe 경로/자동 시작/pipeName 등)

**Relationships:**
- **With Existing:** RapidRAW `AppSettings.uiVisibility`/`adjustmentVisibility`를 활용해 customer/admin “숨김” 정책을 구현(FR17)
- **With New:** Camera Sidecar Service 및 세션 관리 로직과 연결

## Schema Integration Strategy

**Database Changes Required:**
- **New Tables:** 없음(MVP는 DB 미도입)
- **Modified Tables:** 없음
- **New Indexes:** N/A
- **Migration Strategy:**
  - `settings.json`의 `boothy.schemaVersion`로 설정 마이그레이션을 관리
  - 세션 메타데이터 파일을 도입하는 경우 `boothy.session.json`에 `schemaVersion` 포함
  - `.rrdata`는 RapidRAW `ImageMetadata.version`을 유지하면서, Boothy 키는 `adjustments` 내 **추가 필드**로만 확장(파괴적 변경 금지)

**Backward Compatibility:**
- Boothy 관련 스키마 변경은 **추가(append) 중심**으로만 진행하고, 기존 필드는 유지
- 알 수 없는 키는 무시하도록(serde 기본 동작) 유지해 구버전/신버전 공존 가능
- 프리셋 ID는 “불변”이 아닐 수 있으므로(예: 프리셋 import 시 새 UUID 발급), 사진 결과의 정합성은 `adjustments 스냅샷`을 기준으로 하고, `boothyPresetId/name`은 **추적/표시용 보조 정보**로 취급
