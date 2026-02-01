# Boothy Brownfield Enhancement Architecture (Legacy)

> NOTE (2026-01-24): 최신/정합한 아키텍처는 sharded 문서 `docs/architecture/`가 소스 오브 트루스입니다.
> - 시작점(ToC): `docs/architecture/index.md`
> - IPC/API: `docs/architecture/api-design-and-integration.md`
> - 컴포넌트/통합 흐름: `docs/architecture/component-architecture.md`
> - Epic 4(실 카메라): `docs/prd/epic-4-real-camera-hardware-integration-field-validation.md`

이 파일은 엔트리포인트로 유지하며, 아래 “Legacy” 섹션은 과거 초안으로 일부 가정/디폴트(예: sessionsRoot, 레퍼런스 코드 중심 서술 등)가 현재 구현과 다를 수 있습니다.

## Sharded Architecture Index

- `docs/architecture/index.md`

## Legacy (Deprecated)

## Introduction

이 문서는 **Boothy**를 단일 **Windows** 데스크탑 앱(**Tauri + React**)으로 확장하여, 촬영 → 실시간 확인 → 프리셋/편집 → 내보내기까지 하나의 UX로 통합하기 위한 목표(TO‑BE) 아키텍처를 정의합니다. 주요 목적은 AI 기반 개발(스토리/태스크 실행)이 기존(브라운필드) 리포지토리 현실과 충돌하지 않도록, 통합 지점과 경계를 명확히 하는 것입니다.

**기존 아키텍처와의 관계:**
현재 리포는 1차 Boothy 앱 코드가 아직 없고, 두 개의 OSS 레퍼런스 스택이 `reference/` 아래에 있습니다. 본 문서는 `docs/brownfield-architecture.md`의 AS‑IS 분석을 보완하며, 신규 1차 컴포넌트가 레퍼런스 스택과 어떻게 상호작용/이행(migration)할지 규정합니다. 기존 패턴과 충돌 시, 일관성을 유지하기 위한 우선순위를 제시합니다.

### 범위 적합성 및 입력(검증용)

이번 작업은 **아키텍처 설계가 필요한 수준의 브라운필드 통합**입니다(두 OSS 스택 결합 + 신규 통합 UX + customer/admin 모드 UI 게이팅 + 실시간 파일 감지/프리셋 파이프라인). 따라서 본 아키텍처 문서를 진행합니다.

**사용한 입력(리포 내 근거):**
- PRD: `docs/prd.md`
- 현재 상태(AS‑IS) 분석: `docs/brownfield-architecture.md`
- UX/UI 제약: `docs/design_concept.md`
- 레퍼런스 코드:
  - 카메라: `reference/camerafunction/digiCamControl-2.0.0/` (C# / .NET Framework 4.0, WPF, Named Pipe remote cmd)
  - 편집/프리셋/익스포트: `reference/uxui_presetfunction/` (RapidRAW, React/Vite + Tauri/Rust)

**제가 “현재 시스템 현실”로 가정하고 진행할 핵심 관찰(틀리면 수정 필요):**
1. 리포 루트에는 **1차 Boothy 앱 코드가 아직 없고**, `docs/` + `reference/`가 대부분입니다.
2. 목표 제품은 **단일 Windows 앱**이며 UI는 **Tauri + React**, **WPF UI는 금지**입니다.
3. MVP 카메라는 **Canon 중심**이며, 카메라 레퍼런스는 **digiCamControl(EDSDK 포함)**이고 **Named Pipe 기반 원격 명령** 패턴이 관찰됩니다.
4. 촬영→편집 통합의 1차 경계는 **세션 폴더(파일시스템 계약)**이며, “전송 완료 후 자동 감지/가져오기”가 핵심입니다.

문서 언어는 우선 **한국어(핵심 용어는 영어 병기)** 기준으로 작성하겠습니다. 다른 선호가 있으면 알려주세요.

### Existing Project Analysis

#### Current Project State

- **Primary Purpose:** RapidRAW(편집/프리셋/익스포트)를 기반으로 **Boothy로 리브랜딩/변형**하고, 카메라 촬영 워크플로우를 통합한 Windows 포토부스 앱(Tauri+React)을 만들기 위한 브라운필드 베이스(레퍼런스 코드 + 문서).
- **Current Tech Stack:** (레퍼런스) RapidRAW: React `19.2.3` + Vite `7.3.0` + Tauri `2.9.x` + Rust(edition `2024`, rust-version `1.92`), (레퍼런스) digiCamControl: C#/.NET Framework `4.0` + WPF + Canon EDSDK, (문서) `docs/*.md`.
- **Architecture Style:** 현재 리포는 “단일 앱” 아키텍처가 아니라, 서로 독립적인 두 앱(편집/카메라)을 같은 리포에 둔 형태입니다. RapidRAW는 Tauri(Rust) ↔ React(프론트) 커맨드/이벤트 중심 구조이고, digiCamControl은 전역 서비스 로케이터 + 이벤트 기반 디바이스 라이프사이클 + Named Pipe IPC 패턴이 관찰됩니다.
- **Deployment Method:** 리포 루트에 1차 Boothy 앱 빌드/배포 파이프라인은 아직 없고, 레퍼런스 스택별로 빌드/패키징 방식이 다릅니다(RapidRAW: GitHub Actions 기반 릴리즈, digiCamControl: Visual Studio 솔루션 + Setup/NSIS).

#### Available Documentation

- `docs/prd.md`: 통합 UX/기능 요구사항 + NFR(실시간/백그라운드 처리/오프라인/보안 로그 등)
- `docs/brownfield-architecture.md`: 현재 리포 실체(레퍼런스 스택, 관찰된 패턴, 기술부채, IPC 정보 등)
- `docs/design_concept.md`: customer/admin 모드 정책 + “숨김(비활성화 금지)” UX 규칙 + RapidRAW 스타일 통일 지시
- `reference/uxui_presetfunction/README.md`, `reference/uxui_presetfunction/src-tauri/*`: 편집/프리셋/익스포트 스택 구조 및 의존성 근거
- `reference/uxui_presetfunction/.github/workflows/*`: 레퍼런스 앱의 CI/릴리즈 흐름(참고용)
- `reference/camerafunction/digiCamControl-2.0.0/Docs/` 및 소스: 카메라 제어/캡처/Named Pipe remote cmd 패턴 근거

#### Identified Constraints

- **플랫폼/런타임:** Windows-only(NFR1), 오프라인 필수(NFR7)
- **UI/제품 형태:** Tauri + React만 허용(NFR2), WPF UI 금지(기능 참고만)(NFR2/`docs/design_concept.md`)
- **실시간성:** 전송 완료 후 세션 리스트 반영 ≤ 1s 목표, 메인 뷰 프리셋 적용 프리뷰 ≤ 3s 목표(NFR3)
- **성능/반응성:** 프리셋 적용/RAW 처리/익스포트는 백그라운드 처리로 UI block 금지(NFR4)
- **데이터 무결성:** 전송 완료 전 파일을 “수입(import) 완료”로 간주하면 안 됨, partial transfer로 손상된 import 방지(NFR5)
- **보안:** admin 비밀번호는 salted hash로 안전 저장, 평문 저장/로그 금지(NFR6)
- **오프라인/무계정 정책:** Boothy MVP는 **로그인 없이** 동작하고, **기본적으로 네트워크 호출을 하지 않음**(NFR7). RapidRAW 레퍼런스에 포함된 온라인 기능(예: Clerk/auth, 커뮤니티, 모델 다운로드 등)은 Boothy 제품 빌드에서 제거/비활성화가 필요합니다.
- **관찰된 레거시/의존성:** 카메라 레퍼런스(digiCamControl)는 .NET Framework 4.0/WPF 기반이며, Canon EDSDK 등 네이티브/아키텍처(x86/x64) 의존성이 존재(브리징/이행 전략 필요)
- **세션 폴더 계약(TO‑BE):**
  - **세션 루트(`sessionsRoot`) 기본값:** `%USERPROFILE%\\Pictures\\Boothy` (admin에서 변경 가능, 세션 시작 화면에서 base directory 선택 옵션 제공 가능)
  - **세션 생성/열기:** “세션 시작 시” 사용자가 `sessionName`을 입력하면 해당 값으로 세션 폴더를 생성/활성화(존재 시 열기) (FR3)
  - **폴더명 규칙:** `sessionName`은 폴더명으로 안전하게 변환(sanitize)하여 `sessionFolderName`으로 저장(Windows 금지 문자 제거/치환, 길이 제한 등)
  - **중복/충돌 처리:** 동일 `sessionFolderName`이 이미 있고 “새 세션”이 필요하면 `YYYY_MM_DD_HH` suffix로 새 폴더 생성(예: `Wedding_2026_01_14_15`) (선택 UX)
- **하위 폴더:** `Raw/`(촬영 원본 저장), `Jpg/`(Export 결과 저장)
- **리포 현실:** 대용량 벤더 바이너리 포함 및 중첩 git 등으로 운영/빌드/보안(공급망) 고려 필요(`docs/brownfield-architecture.md`)
- **문서 상태:** 코딩 표준/소스 트리/개발환경/테스트 전략은 현재 `docs/architecture.md`에 포함되어 있으며, 필요 시 별도 문서로 분리합니다.
- **라이선스/배포 게이트:** RapidRAW(AGPL-3.0) 및 Canon EDSDK(재배포 조건) 이슈로, **외부 배포 전 라이선스/재배포 정책 확정이 필수**입니다. 정책 확정 전에는 내부 테스트 배포만 허용하며, Canon SDK DLL 번들은 기본값으로 하지 않습니다(명시적 prerequisites 전제).

#### Key Architectural Decisions (TO‑BE)

1. **앱 베이스:** RapidRAW를 제품 베이스로 채택하고 Boothy로 리브랜딩/변형합니다(렌더/프리셋/Export 코어는 호환성 유지, CR1).
2. **카메라 통합 전략(MVP):** digiCamControl을 기능 레퍼런스로 삼아, headless **Camera Sidecar Service**로 제공하고 Boothy(Tauri backend)가 Named Pipe IPC로 제어/이벤트를 수신합니다(FR19–FR21).
3. **세션 계약:** 세션은 폴더로 표현되며 `sessionsRoot` 아래에서 “활성 세션 1개”만 유지합니다. 세션 폴더 내부에 `{Raw,Jpg}`를 사용합니다(FR3/FR6/FR12).

#### Camera Integration: Option A 리스크 및 Option B 해석

**Option A (digiCamControl 외부 프로세스/IPC 래핑)의 주요 리스크**
- **패키징/배포 복잡도:** Tauri 앱 + 카메라 서비스(또는 기존 앱/CLI) + .NET Framework 의존성 + Canon SDK DLL 등 “구성요소”가 늘어 설치/업데이트/서명/안티바이러스 오탐 대응이 어려워질 수 있습니다.
- **프로세스/IPC 신뢰성:** Named Pipe/프로세스 간 통신 장애(서비스 크래시, 파이프 연결 끊김, 버전 불일치) 시 UX가 깨질 수 있어, “자동 재시작/재연결/에러 표면화” 설계가 필수입니다.
- **성능/지연:** 캡처 명령→전송 완료→파일 감지/프리셋 적용까지의 경로가 길어져 NFR3(≤1s/≤3s) 목표 달성이 어려울 수 있으며, 이벤트 기반(카메라 서비스 알림) + 폴더 감지(백업) 이중화가 필요합니다.
- **기능 100%의 ‘UI 매핑’ 비용:** 카메라 기능을 100% 동작시키는 것과, 그 기능을 RapidRAW 스타일의 admin UI로 “모두 노출”하는 것은 별개의 큰 작업입니다(속성/메뉴/상태/에러 처리 포함).
- **레거시 제약:** digiCamControl은 .NET Framework 4.0/WPF 중심으로 설계되어 있어, headless 동작/서비스화 과정에서 예기치 않은 UI 스레드/COM/드라이버 이슈가 발생할 수 있습니다(특히 SDK 콜백, 장치 연결/해제 이벤트).
- **유지보수/업스트림 추적:** 외부 OSS/SDK 조합에 의존하므로, Windows 업데이트/카메라 펌웨어/SDK 변경 시 문제 원인 추적이 복잡해질 수 있습니다.

**Option A 리스크 완화(아키텍처 레벨 대응)**
- **전용 “Camera Sidecar Service”**를 별도 프로세스로 두고(Tauri sidecar로 번들), UI(WPF)는 포함하지 않되 digiCamControl의 디바이스/SDK 레이어를 재사용하는 형태로 수렴시킵니다.
- **버전드 IPC 계약**(예: JSON-RPC over Named Pipe, 명시적 error code/telemetry 포함)을 정의해, 프론트/백엔드/서비스 간 호환성과 장애 원인 추적성을 확보합니다.
- **감시/자가복구**: Tauri backend가 서비스 health-check + 자동 재시작/재연결을 수행하고, “카메라 연결/촬영 실패/전송 실패”를 사용자에게 actionable 하게 표면화합니다(FR20, NFR8).
- **이벤트+폴백 감지**: 서비스가 “전송 완료 파일 경로” 이벤트를 발행하고, 앱은 폴더 watcher + 안정화(stabilization) 로직을 백업으로 둬 NFR3/NFR5를 만족시키는 방향으로 이중화합니다.
- **통합 로깅/상관관계:** capture 요청부터 import/export까지 correlation id를 이어서 기록해 현장 디버깅 가능성을 높입니다(NFR8).

**Option B (Rust에서 Canon EDSDK 직접 연동)는 ‘작업량’만의 문제가 아닙니다**
- **기능/안정성 리스크:** “100% 기능 동작” 기준을 맞추려면 EDSDK의 모든 기능(및 예외 케이스)을 Rust FFI로 안정적으로 감싸고, 다양한 기종/환경에서 검증해야 합니다. 이는 단순 구현량을 넘어 “품질 확보” 난이도가 큽니다.
- **플랫폼/배포/라이선스 고려:** Canon SDK는 배포 조건/바이너리 포함 방식 등 제약이 있을 수 있고, FFI/콜백/스레딩 모델을 잘못 다루면 크래시/메모리 문제로 이어질 수 있습니다.
- **장기적 장점(있음):** 성공한다면 프로세스 1개(Tauri)로 단순화되어 배포/IPC/에러 핸들링이 단순해지고, 장기 유지보수성은 좋아질 수 있습니다. 다만 MVP 목표(빠른 통합 + 기능동작 보장) 관점에선 리스크가 큽니다.

### Change Log

| Change | Date | Version | Description | Author |
| --- | --- | --- | --- | --- |
| Initial draft | 2026-01-13 | 0.1 | Start target architecture based on `docs/prd.md` and current repo analysis | Winston |
| PRD-aligned TO-BE | 2026-01-14 | 1.0 | Align session contract (`sessionName`, configurable `sessionsRoot`) and remove interactive checkpoints | Codex |

## Enhancement Scope and Integration Strategy

### Enhancement Overview

**Enhancement Type:** Integration with New Systems + New Feature Addition + UI/UX Overhaul + Role-based UI gating

**Scope:** RapidRAW(Tauri+React) 기반으로 Boothy 앱을 리브랜딩/변형하고, 카메라 촬영/전송 파이프라인을 통합하여 “세션 폴더(파일시스템)”를 중심으로 촬영→실시간 미리보기→프리셋 자동 적용→Export(JPG)까지 하나의 UX로 제공

**Integration Impact:** Major (카메라 통합/실시간 처리/세션 계약/권한 UI까지 포함)

### Integration Approach

**Code Integration Strategy:**
- **RapidRAW를 제품 베이스로 채택**하고, 신규 Boothy 기능은 “세션 모드(Session Mode)”로 추가하여 기존 편집/프리셋/익스포트 코어를 최대한 재사용합니다.
- 카메라 기능은 (A) **digiCamControl 기반 “Camera Sidecar Service”**로 분리(별도 프로세스)하고, Boothy(Tauri backend)가 IPC로 제어/상태 구독합니다.
- 촬영→편집 통합의 1차 경계는 **세션 폴더 계약**이며, “전송 완료 이후”에만 import 처리합니다.

**Database Integration:**
- MVP는 **전용 DB를 도입하지 않고**, 사진별 비파괴 편집 상태는 RapidRAW의 기존 **이미지 옆 `.rrdata` 사이드카**(ImageMetadata.adjustments)에 저장하여 “프리셋 적용/회전 등”을 영속화합니다(FR10/FR14). 세션 레벨 메타데이터(세션 이름/생성시각 등)가 필요하면 세션 폴더 루트에 `boothy.session.json`을 추가할 수 있습니다(선택).
- 앱 전역 설정(예: admin 모드 설정/기본 경로 등)은 OS별 AppData 영역에 설정 파일로 저장하고, 추후 필요 시 SQLite + 마이그레이션(CR2)로 확장합니다.

**API Integration:**
- 앱 내부는 **React(UI) ↔ Tauri(Rust backend)** 간 command/event 패턴을 사용하고,
- **Tauri backend ↔ Camera Sidecar Service**는 **버전드 IPC 계약**(예: JSON 메시지 + 명시적 error code/telemetry, correlation id)을 사용합니다.
- 외부 네트워크 의존은 최소화하고(오프라인), 진단/로그는 로컬 파일로 남깁니다(NFR7/NFR8).

**UI Integration:**
- UI는 RapidRAW 스타일을 기준으로, Boothy의 **촬영/세션 시작(세션 이름 입력)/모드 토글** 플로우를 추가합니다.
- customer 모드는 기본이며, customer에서는 “숨김” 원칙으로 고급 기능을 제거하고, admin 모드에서만 전체 카메라 기능/고급 편집 기능을 노출합니다(FR15–FR19, `docs/design_concept.md`).
- customer-facing 썸네일/리스트/프리뷰에는 카메라 메타데이터 오버레이(F/ISO/Exposure/히스토그램 등)를 표시하지 않습니다(FR18, `docs/design_concept.md`).

### Compatibility Requirements

- **Existing API Compatibility:** RapidRAW 프리셋 정의/로딩/렌더 및 Export 결과의 호환성을 유지해야 합니다(CR1).
- **Database Schema Compatibility:** MVP는 DB 없이 시작하되, 도입 시 forward/backward-compatible 마이그레이션을 제공합니다(CR2).
- **UI/UX Consistency:** 신규 카메라 UX는 RapidRAW 디자인 시스템/컴포넌트 규칙을 따르며, customer/admin 모드 “숨김” 정책을 전역적으로 일관되게 적용합니다(CR3).
- **Performance Impact:** 파일 전송 완료 후 세션 반영 ≤ 1s / 프리셋 적용 프리뷰 ≤ 3s 목표를 위해, import/프리셋 적용/썸네일 생성/Export는 백그라운드 처리 및 큐잉/취소를 지원해야 합니다(NFR3/NFR4).

이 통합 경계(세션 폴더 + 카메라 사이드카 서비스)는 RapidRAW의 command/event 패턴 및 `.rrdata` 사이드카 저장 모델, 그리고 digiCamControl의 IPC/이벤트 기반 카메라 패턴을 존중하도록 설계합니다.

## Tech Stack

### Existing Technology Stack

| Category | Current Technology | Version | Usage in Enhancement | Notes |
| --- | --- | --- | --- | --- |
| Desktop App Framework | Tauri | `2.9.x` | **Boothy 메인 앱 런타임** | RapidRAW가 이미 Tauri 기반이며 Windows 번들링(NSIS) 구성 존재 |
| Frontend UI | React | `19.2.3` | **Boothy UI 베이스** | RapidRAW UI 스타일/컴포넌트를 기준으로 Boothy 화면 추가 |
| Frontend Language | TypeScript | `5.9.3` | **Boothy UI 개발 언어** | RapidRAW 프론트엔드와 동일 |
| Frontend Build | Vite | `7.3.0` | **프론트 빌드/번들** | RapidRAW 스택 유지 |
| Styling | Tailwind CSS | `3.4.19` | **스타일링** | RapidRAW UI 스타일 일관성 유지에 활용 |
| Animation/UI Utils | framer-motion, lucide-react 등 | `12.23.x`, `0.562.x` | **UI 상호작용** | RapidRAW 의존성 범위 내에서 사용 |
| Backend Language | Rust | `1.92` (edition `2024`) | **Tauri backend(Boothy 로직)** | 세션 관리/파일 감지/프리셋 적용 파이프라인 오케스트레이션 |
| Async Runtime | tokio | `1.x` | **백그라운드 처리** | import/프리셋 적용/썸네일/Export 작업 큐잉 및 UI 비블로킹(NFR4) |
| GPU/Image | wgpu, image, rawler 등 | `28.0`, `0.25.x`, (path) | **RAW 처리/프리셋 적용/Export** | RapidRAW 코어를 재사용(CR1), Boothy는 “언제/어떤 프리셋을 적용”만 제어 |
| Camera Reference Stack | digiCamControl | `2.0.0` | **카메라 기능 기준/재사용 후보** | .NET Framework 4.0 + WPF(제품 UI는 사용 금지), 기능/SDK 연동은 재사용 가능 |
| Canon SDK | Canon EDSDK | (license-dependent DLL) | **MVP Canon 지원** | DLL 번들링은 재배포 정책 확정 후 결정(기본 미포함), x86/x64 정합성 확인 필요 |
| IPC (camera) | Windows Named Pipe | (N/A) | **Boothy ↔ Camera Sidecar 통신** | digiCamControl에 Named Pipe 패턴 관찰, Boothy는 버전드 IPC 계약으로 확장 |
| Packaging | NSIS (Tauri bundler) | (config) | **Windows 설치 패키징** | `reference/uxui_presetfunction/src-tauri/tauri.conf.json`에 nsis 설정 존재 |
| Logging | log + fern (Rust), log4net(.NET) | `0.4`, `0.7`, (legacy) | **진단/현장 디버깅** | correlation id 기반으로 end-to-end 로그 연결(NFR8) 필요 |

### New Technology Additions

| Technology | Version | Purpose | Rationale | Integration Method |
| --- | --- | --- | --- | --- |
| `notify` (Rust crate) | latest compatible | 세션 폴더 실시간 감지 | NFR3(≤1s) 달성을 위해 polling보다 이벤트 기반 감지가 유리 | Tauri backend에서 `Raw/` 신규 파일 생성/완료 감지에 사용 |
| `argon2` (Rust crate) | latest compatible | admin 비밀번호 salted hash | 단순 해시(sha2)보다 password hashing에 적합, NFR6 충족 | 설정 저장 시 hash+salt만 저장(AppData) |

## Data Models and Schema Changes

### New Data Models

#### BoothySession (Folder-backed Session)

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

#### BoothyPerImageState (Stored in RapidRAW `.rrdata`)

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

#### BoothyAppSettings (Extension of RapidRAW `AppSettings`)

**Purpose:** 앱 전역 설정(세션 루트/모드/관리자 인증/카메라 사이드카 설정 등)을 저장합니다.

**Integration:** RapidRAW가 이미 제공하는 `settings.json`(Tauri AppData, `AppSettings`)을 확장하여 Boothy 전용 설정을 저장합니다. 충돌을 피하기 위해 `boothy` 네임스페이스 객체를 사용합니다.

**Key Attributes:**
- `boothy.schemaVersion`: number
- `boothy.sessionsRoot`: string (기본 `%USERPROFILE%\\Pictures\\Boothy`의 **절대 경로**)
- `boothy.defaultMode`: `"customer"` (FR15)
- `boothy.adminPassword`: object (argon2id 파라미터 + salt + hash, NFR6)
- `boothy.cameraSidecar`: object (exe 경로/자동 시작/pipeName 등)

**Relationships:**
- **With Existing:** RapidRAW `AppSettings.uiVisibility`/`adjustmentVisibility`를 활용해 customer/admin “숨김” 정책을 구현(FR17)
- **With New:** Camera Sidecar Service 및 세션 관리 로직과 연결

### Schema Integration Strategy

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

## Component Architecture

### New Components

#### Boothy Session Manager (Tauri Backend)

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

#### Boothy Camera IPC Client (Tauri Backend)

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

#### Camera Sidecar Service (C#/.NET, Headless)

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

#### Boothy File Arrival Watcher (Tauri Backend)

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

#### Boothy Preset Assignment Service (Tauri Backend)

**Responsibility:** 현재 선택된 프리셋을 “신규 유입 사진에만” 적용하기 위해, 파일 도착 시점의 프리셋 adjustments를 스냅샷으로 `.rrdata`에 저장합니다(FR8–FR10). 프리셋 변경은 이후 사진에만 영향(FR9)이며, 기존 사진은 수정하지 않습니다.

**Integration Points:** UI에서 선택된 RapidRAW `Preset(adjustments)`를 전달받아 메모리에 유지하고, 신규 파일 확정 시 `.rrdata`의 `ImageMetadata.adjustments`에 스냅샷을 저장합니다.

**Key Interfaces:**
- `tauri::command boothy_set_current_preset(presetId: string, presetAdjustments: json)`
- `apply_preset_snapshot_to_image(path: string)`

**Dependencies:**
- **Existing Components:** RapidRAW Preset 모델(`Preset.adjustments`), RapidRAW `.rrdata`(ImageMetadata)
- **New Components:** Boothy File Arrival Watcher

**Technology Stack:** Rust + serde_json

#### Boothy Mode/Auth (Tauri Backend + React UI)

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

#### Boothy UI Extensions (React)

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

### Component Interaction Diagram

```mermaid
graph TD
  U[User] --> UI[Boothy UI (React/RapidRAW 기반)]
  UI -->|invoke/listen| TB[Tauri Backend (Rust)]

  subgraph SessionFS[Filesystem]
    ROOT[%USERPROFILE%\\Pictures\\Boothy]
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

## API Design and Integration

### API Integration Strategy

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

### New API Endpoints

#### Boothy UI ↔ Tauri Backend (Commands)

##### Create/Activate Session
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
  "basePath": "C:\\Users\\KimYS\\Pictures\\Boothy\\Wedding_2026_01_14_15",
  "rawPath": "C:\\Users\\KimYS\\Pictures\\Boothy\\Wedding_2026_01_14_15\\Raw",
  "jpgPath": "C:\\Users\\KimYS\\Pictures\\Boothy\\Wedding_2026_01_14_15\\Jpg"
}
```

##### Capture (Shoot)
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

##### Set Current Preset (Snapshot Source)
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

##### Admin Login
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

#### Tauri Backend ↔ Camera Sidecar (Named Pipe RPC)

권장 포맷은 **JSON-RPC 스타일**이며, 단일 파이프에서 request/response + event notification을 멀티플렉싱합니다.

##### Get Camera State
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

##### Set Session Destination (Raw Path)
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
    "rawPath": "C:\\Users\\KimYS\\Pictures\\Boothy\\Wedding_2026_01_14_15\\Raw"
  },
  "meta": { "protocolVersion": 1, "correlationId": "..." }
}
```

**Response**
```json
{ "jsonrpc": "2.0", "id": "a3a8d6f2-1a2f-4b4d-9f4d-8f2c7a0b9d0e", "result": { "ok": true } }
```

##### Photo Transferred (Event Notification)
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
    "path": "C:\\Users\\KimYS\\Pictures\\Boothy\\Wedding_2026_01_14_15\\Raw\\IMG_0001.CR3",
    "capturedAt": "2026-01-14T15:02:33Z"
  },
  "meta": { "protocolVersion": 1, "correlationId": "..." }
}
```

sidecar IPC는 구현/운영 상의 버전 관리와 진단을 위해 **JSON-RPC 스타일 메시지 + `protocolVersion`/`requestId`/`correlationId`**를 표준으로 사용합니다.

## Source Tree

### Existing Project Structure

```plaintext
Boothy/
├── .bmad-core/                      # (옵션) 문서/체크리스트 자동화 도구 - 현재 리포에서는 VCS에서 제외됨(.gitignore)
├── docs/
│   ├── prd.md
│   ├── brownfield-architecture.md
│   ├── design_concept.md
│   └── architecture.md              # (this document)
└── reference/
    ├── camerafunction/
    │   └── digiCamControl-2.0.0/     # C#/.NET Framework 4.0 + WPF (레퍼런스)
    └── uxui_presetfunction/          # RapidRAW (React/Vite + Tauri/Rust, 레퍼런스)
```

### New File Organization

```plaintext
Boothy/
├── apps/
│   ├── boothy/                      # ✅ 제품 코드: RapidRAW 기반 Boothy 앱
│   │   ├── src/                     # React UI (+ Boothy UI Extensions)
│   │   ├── src-tauri/               # Rust backend (+ Boothy services)
│   │   └── ...                      # vite/tauri config, assets, packaging
│   └── camera-sidecar/              # ✅ 제품 코드: Headless 카메라 서비스(.NET)
│       ├── Boothy.CameraSidecar.sln
│       ├── src/
│       └── ...                      # IPC, logging, EDSDK integration wrapper
├── docs/
│   └── ...                          # 기존 유지(아키텍처/스토리/QA 등)
└── reference/
    └── ...                          # 레퍼런스 스택은 가능한 “읽기 전용”으로 유지
```

### Integration Guidelines

- **File Naming:** RapidRAW의 기존 네이밍/레이아웃을 유지하고, Boothy 신규 커맨드는 `boothy_*`(Tauri command) / 이벤트는 `boothy-*`(Tauri event)로 통일합니다. sidecar RPC는 `camera.*` 메서드 네임스페이스를 사용합니다.
- **Folder Organization:** **제품 코드와 레퍼런스를 분리**하기 위해, RapidRAW(현재 `reference/uxui_presetfunction`)는 초기 단계에서 **`apps/boothy`로 승격(migrate)**하여 그 위치에서 리브랜딩/기능 통합을 진행합니다. `reference/`는 카메라 스택 및 업스트림 비교 용도의 “읽기 전용” 영역으로 유지합니다(리포 내 “레퍼런스 vs 제품” 경계 명확화).
- **Import/Export Patterns:** “세션 폴더 계약”을 최우선으로 하고, 앱 내부 통신은 event-driven(새 사진 도착 이벤트 → UI refresh/선택)으로 구성합니다. 저장 포맷은 기존 `.rrdata`를 확장(append-only)합니다.

**Decision (Efficiency)**
RapidRAW가 제품 베이스로 확정된 상태에서는, `reference/` 아래에서 계속 개발하는 것보다 **초기에 `apps/boothy`로 승격**해 “제품 코드 경계”를 명확히 하는 편이 개발/빌드/문서화/온보딩에서 더 효율적입니다. 업스트림 비교는 `UPSTREAM.md` 기록 + git tag/branch(또는 별도 스냅샷 디렉터리)로 대체합니다.

## Infrastructure and Deployment Integration

### Existing Infrastructure

**Current Deployment:** 현재 Boothy 리포 루트에는 제품용 CI/CD가 없고, RapidRAW 쪽에만 GitHub Actions 워크플로우가 포함되어 있습니다(단, `reference/uxui_presetfunction/.github/workflows/*`는 리포 루트가 아니라서 Boothy 리포의 CI로는 동작하지 않음).

**Infrastructure Tools:** (참고) RapidRAW는 GitHub Actions + `tauri-apps/tauri-action` 기반으로 멀티플랫폼 번들링(NSIS/dmg/AppImage 등)을 수행합니다.

**Environments:** MVP는 Windows-only이며, 네트워크 의존 없이 로컬 설치/실행이 핵심입니다(NFR1/NFR7).

### Enhancement Deployment Strategy

**Deployment Approach:**
- Boothy는 **Tauri 번들링(NSIS installer)**로 배포하며, 설치 대상은 “프로그램 파일(앱)” + “사용자 세션 폴더(사진 저장)” + “AppData 설정/로그”로 구분합니다.
- 카메라 기능은 별도 프로세스인 **Camera Sidecar Service**를 함께 설치/번들하고, Boothy 앱이 런타임에 sidecar를 자동 실행/감시합니다.

**Infrastructure Changes:**
- 리포 루트에 Boothy 전용 CI 워크플로우를 추가(Windows build + NSIS output)하고, 카메라 sidecar 빌드 산출물을 Boothy 번들 리소스에 포함합니다.
- sidecar에는 Canon EDSDK 등 네이티브 DLL이 필요할 수 있으므로 x86/x64 정합성을 엄격히 관리합니다. 단, **Canon EDSDK DLL은 재배포 정책 확인 전까지 설치 패키지에 포함하지 않는 것을 기본값**으로 하며(명시적 prerequisites + 사용자 설치/제공 전제), 정책 확정 후 번들링 여부를 결정합니다.

**Pipeline Integration (권장 구성):**
1. **Build boothy app** (`apps/boothy`)
   - Node(예: 22)로 프론트 빌드 → Rust로 Tauri build → NSIS 산출물 생성
2. **Build camera sidecar** (`apps/camera-sidecar`)
   - .NET 빌드(타겟 프레임워크/런타임은 sidecar 설계에 따름)
   - 산출물(Exe + DLL)을 Boothy 번들 리소스 경로로 복사
3. **Bundle**
   - Tauri `bundle.resources`에 sidecar 산출물을 포함
   - Boothy 런타임에서 sidecar를 “설치 디렉터리/리소스”에서 실행

**Runtime Layout (제안):**
- 설치 경로(예): `C:\\Program Files\\Boothy\\`  
  - `Boothy.exe` (Tauri)
  - `resources\\camera-sidecar\\Boothy.CameraSidecar.exe`
  - `resources\\camera-sidecar\\*.dll` (옵션: EDSDK 등, 재배포 정책에 따름)
- 사용자 데이터(예): `%APPDATA%\\Boothy\\`
  - `settings.json` (RapidRAW + boothy 확장)
  - `logs\\boothy.log`, `logs\\camera-sidecar.log`
- 세션 데이터: `%USERPROFILE%\\Pictures\\Boothy\\<session>\\{Raw,Jpg}`

**Operational Concerns (필수):**
- **버전 동기화:** Boothy 앱과 sidecar는 같은 릴리즈로 배포하고, sidecar는 `protocolVersion`/`appVersion` handshake로 불일치 시 명시적으로 실패(FR20, NFR8).
- **자가복구:** Boothy backend가 sidecar 프로세스를 감시하고 크래시 시 자동 재시작/재연결합니다.
- **오프라인:** 업데이트/다운로드 없이 동작해야 하며, 필드 진단은 로컬 로그로 해결합니다(NFR7/NFR8).

### Rollback Strategy

**Rollback Method:**
- NSIS 인스톨러 단위로 “이전 버전 재설치”를 공식 롤백으로 정의합니다(네트워크 없는 현장에서도 가능).
- 설정/스키마(`boothy.schemaVersion`)는 **하위호환(append-only)** 중심으로 설계하여, 롤백 시에도 기본 기능이 동작하도록 합니다(필요 시 “새 설정을 무시/리셋” 옵션 제공).

**Risk Mitigation:**
- sidecar IPC는 `protocolVersion`으로 강제하며, mismatch는 “정상적으로 촬영이 안 되는 불명확 상태”가 아니라 “업데이트 필요/호환 불가”로 명확히 표면화합니다.
- 세션 폴더/원본 데이터는 롤백과 독립적으로 유지되어야 합니다(원본/출력 손실 금지, NFR5).

**Monitoring:**
- 원격 모니터링은 MVP 범위 밖(오프라인)으로 두고, 로컬 로그/진단 파일의 품질을 품질게이트로 설정합니다(NFR8).


## Coding Standards

### Existing Standards Compliance

**Code Style:**
- **TypeScript/React:** `reference/uxui_presetfunction` 기준으로 `singleQuote=true`, `semi=true`, `printWidth=120`(Prettier) 및 React 함수형 컴포넌트 + hooks 패턴을 유지합니다.
- **Rust(Tauri backend):** 모듈 분리(예: `file_management.rs`, `image_processing.rs` 등) 패턴을 유지하고, `tauri::command` 기반의 명시적 API 경계를 유지합니다.
- **C# sidecar:** UI 없는 headless 프로세스 기준으로, 카메라 제어/IPC/로깅을 명확히 분리합니다(제품 UI는 금지).

**Linting Rules:**
- **Frontend:** ESLint + Prettier를 사용하며(`plugin:prettier/recommended`), `quotes: 'single'`, `semi: always`, `no-unused-vars(argsIgnorePattern: '^_')` 규칙을 준수합니다.
- **Rust:** `cargo fmt`/`cargo clippy`를 기본 품질게이트로 사용합니다(추가 설정이 없다면 rustfmt 기본 규칙).
- **C#:** 최소한 `.editorconfig` + 경고 수준 고정(가능하면 treat warnings as errors)으로 일관성을 확보합니다.

**Testing Patterns:**
- RapidRAW 코드베이스에서 명확한 자동화 테스트(프론트 단위 테스트/백엔드 `#[test]`) 패턴은 현재 확인되지 않습니다. 따라서 Boothy는 신규 테스트 프레임워크를 “즉시 도입”하기보다, 먼저 **통합/회귀 시나리오**를 문서화하고(Testing Strategy에서 정의) 위험도가 높은 영역부터 점진적으로 테스트를 추가합니다.

**Documentation Style:**
- 문서는 Markdown(`docs/*.md`)으로 유지하고, “결정/근거/검증 포인트”를 함께 기록합니다(현장 운영/AI 개발 핸드오프 목적).

### Enhancement-Specific Standards

- **Boothy 네임스페이스:** 신규 Tauri command는 `boothy_*`, 이벤트는 `boothy-*`, sidecar RPC는 `camera.*`로 네임스페이스를 고정합니다.
- **Append-only 저장 규칙:** `.rrdata`(`ImageMetadata.adjustments`) 확장은 기존 키를 파괴적으로 변경하지 않고 **추가(append)**만 허용합니다(호환성).
- **Boothy 메타데이터 키:** RapidRAW 조정 키와 충돌을 피하기 위해 Boothy 전용 메타데이터는 `adjustments.boothy`(object) 하위에 저장합니다(예: `presetId`, `presetName`, `appliedAt`). 실제 “프리셋 적용 결과”는 상위 조정 키(exposure 등)로 스냅샷 저장합니다.
- **Background-first:** import/프리셋 적용/썸네일/Export는 UI 스레드를 block하지 않습니다. CPU 집약 작업은 `spawn_blocking`(또는 기존 Rayon 패턴)으로 분리합니다(NFR4).
- **File 안정화:** watcher는 “파일 생성 이벤트”만으로 import 확정하지 않고, 락/사이즈 안정화/최소 시간 등 안정화 체크 후 확정합니다(NFR5).
- **에러는 코드화:** sidecar/IPC/파일 감지/프리셋 처리 에러는 문자열만이 아니라 **에러 코드 + 메시지 + 컨텍스트**로 표준화하고, UI에는 “행동 가능한” 상태로 노출합니다(FR20).

### Critical Integration Rules

- **Existing API Compatibility:** RapidRAW 프리셋 포맷(`presets.json`의 `PresetItem`)과 렌더/Export 파이프라인을 변경하지 않습니다. Boothy는 “프리셋 선택/스냅샷 저장/세션 폴더 제약”만 추가합니다(CR1).
- **Database Integration:** MVP는 DB를 사용하지 않습니다. DB 도입 시 `schemaVersion` + 마이그레이션을 제공하고, 세션 폴더/원본 파일을 DB에 넣지 않습니다(CR2, NFR5).
- **Error Handling:** 카메라 연결/촬영/전송 실패는 앱 크래시로 이어지면 안 되며, 기존 세션 사진의 탐색/Export는 계속 가능해야 합니다(FR20).
- **Logging Consistency:** capture→transfer→import→preset→export의 상관관계(`correlationId`)를 로그로 연결합니다(NFR8). 비밀번호/민감정보(세션 이름 등)는 기본 로그에 평문으로 남기지 않으며, 필요 시 진단 모드에서만 제한적으로 기록합니다(NFR6).

## Testing Strategy

### Integration with Existing Tests

**Existing Test Framework:** 현재 RapidRAW/레퍼런스 코드에서 “표준 자동화 테스트 프레임워크”가 명확히 구성된 흔적은 제한적입니다(프론트 테스트 러너/백엔드 `#[test]`가 일반적으로 발견되지 않음). 따라서 MVP에서는 “테스트 프레임워크를 즉시 도입”하기보다, **품질을 보장하는 통합 시나리오(수동+자동 가능한 형태)**를 우선 정의합니다.

**Test Organization:** 테스트는 “컴포넌트 경계” 단위로 분류합니다.
- UI(React): customer/admin 숨김 정책, 세션 시작 UX, 촬영 버튼, 신규 사진 자동 선택
- Backend(Rust): 세션 폴더 생성 규칙, 파일 안정화(import 확정) 로직, `.rrdata` 스냅샷 저장
- Sidecar(.NET): 카메라 연결/촬영/전송 완료 이벤트, IPC 계약 준수
- End-to-End: capture→transfer→import→preset→export 흐름

**Coverage Requirements:** 정량 커버리지보다, PRD의 고위험 요구(FR6–FR10, FR16–FR20, NFR3–NFR6)를 “시나리오 기반”으로 반드시 검증하는 것을 품질게이트로 둡니다.

### New Testing Requirements

#### Unit Tests for New Components

- **Framework:** Rust `#[test]`(순수 로직), C#(가능하면 xUnit/NUnit), 프론트는 필요 시 최소한의 컴포넌트 테스트 도입
- **Location:** `apps/boothy/src-tauri/src/**`(단위 로직), `apps/camera-sidecar/**`(IPC/프로토콜), 프론트는 `apps/boothy/src/**`
- **Coverage Target:** “핵심 로직(세션명 생성/파일 안정화/스냅샷 저장)”은 단위 테스트로 고정, UI는 smoke 수준부터 시작
- **Integration with Existing:** 기존 RapidRAW 핵심 처리 파이프라인 자체를 단위 테스트로 전면 커버하려 하지 않고, Boothy가 추가한 경계/정책 로직에 집중합니다.

#### Integration Tests

- **Scope:** Tauri backend ↔ filesystem ↔ sidecar IPC 계약의 통합
- **Existing System Verification:** RapidRAW의 “폴더 선택→이미지 리스트→메인 프리뷰→Export” 흐름이 Boothy 세션 모드에서도 유지되는지 확인(CR1)
  - **New Feature Testing:**
  - 세션 시작(세션 이름) → 폴더 생성/열기/중복 처리(suffix 등) 적용
  - sidecar destination이 `Raw/`로 설정되는지
  - 파일 전송 완료 이벤트 또는 watcher로 신규 사진이 ≤1s 내 리스트에 반영되는지(NFR3)
  - 신규 사진에만 프리셋 스냅샷이 적용되는지(FR8/FR9/FR10)
  - Export가 `Jpg/`로 저장되고 customer 모드에서 고급 옵션이 숨김인지(FR12/FR17)
  - 카메라 연결/촬영/전송 실패 시 에러가 표면화되고 앱이 계속 동작하는지(FR20)

#### Regression Testing

- **Existing Feature Verification:** Boothy 변경으로 인해 RapidRAW의 편집/프리셋/Export 결과가 달라지지 않는지(동일 입력/동일 preset에 대한 결과 비교)(CR1)
- **Automated Regression Suite:** MVP는 “핵심 시나리오 자동화”를 목표로 하고, 무거운 GPU 결과 비교는 초기에는 checksum/메타데이터 기반 또는 샘플 수동 검증으로 시작합니다.
- **Manual Testing Requirements (MVP Gate):**
  1) customer 모드 기본 진입, admin 토글+비밀번호, 숨김 정책 확인
  2) 세션 생성/중복 규칙 확인, `Raw/`/`Jpg/` 생성 확인
  3) 촬영 → 전송 완료 후 자동 반영/자동 선택, 프리셋 자동 적용 + 썸네일 오버레이 미표시(FR18) 확인
  4) 프리셋 변경 후 신규 사진만 영향, 이전 사진 불변 확인
  5) Export가 `Jpg/`로 생성, 삭제/회전(admin) 반영 확인
  6) 카메라 분리/전송 실패 시 에러 표시 + 기존 사진 탐색/Export 가능 확인
  7) 오프라인(네트워크 차단)에서도 core flow 동작하며, 로그인/클라우드 기능이 기본 동작에 관여하지 않는지 확인(NFR7)


## Security Integration

### Existing Security Measures

**Authentication:** Boothy MVP는 **계정 로그인 없이** 동작하는 오프라인 앱입니다(NFR7). 참고로 RapidRAW 레퍼런스에는 계정/온라인 기능이 포함될 수 있으나, Boothy 제품 빌드에서는 제거/비활성화하여 “네트워크 없이 기본 기능이 완전 동작”하도록 고정합니다.

**Authorization:** 기존 앱은 역할 기반 권한 모델이 핵심 개념은 아니며, UI 노출/기능 접근 제어는 앱 내부 설정/상태로 결정됩니다.

**Data Protection:** 사진/프리셋/설정은 로컬 파일시스템에 저장됩니다(이미지 파일 + `.rrdata` 사이드카 + AppData의 `settings.json`). 기본적으로 “암호화 저장”은 전제되지 않습니다.

**Security Tools:** Tauri의 capability 기반 권한 모델(`src-tauri/capabilities/*`)과 앱 샌드박스(로컬 번들) 구조를 사용합니다. 다만 RapidRAW 기본 capability에는 `shell`/`process` 권한이 포함되어 있어, Boothy에서는 “최소 권한(least privilege)” 관점에서 재평가가 필요합니다.

### Enhancement Security Requirements

**New Security Measures:**
- **Admin 모드 인증:** admin 비밀번호는 **argon2id**(salt 포함)로 해시 저장하고, 평문 저장/로그를 금지합니다(NFR6).  
- **접근 제한 목적(UX 중심):** customer/admin “숨김”은 “일반 사용자(비전문가)가 실수로 고급 기능을 사용하지 않도록” 하는 목적입니다. MVP 범위에서는 **UI 숨김 중심**으로 구현하고, 악의적/고급 사용자의 우회 호출(개발자 도구/IPC 직접 호출 등) 방지는 별도 보안 하드닝 범위로 둡니다.
- **IPC 접근 제어:** Camera Sidecar의 Named Pipe는 **현재 사용자 세션만 접근** 가능하도록 ACL을 제한합니다(권장). 또한 메시지에 `protocolVersion`/`requestId`/`correlationId`를 포함해 오작동/조사 가능성을 높입니다(NFR8).
- **경로/입력 검증:** `sessionName`/세션 폴더명, `Raw/`/`Jpg/` 경로는 backend에서 검증하며, 파일 삭제/이동은 **활성 세션 루트 하위**로 강제하여 path traversal/오작동을 방지합니다(FR13, NFR5).
- **Tauri Capability 최소화:** Boothy에서 필요 없는 `shell`/`process` 권한을 제거/축소하고, 파일/OS 접근도 필요한 범위만 허용합니다(least privilege).
- **공급망/배포 신뢰:** Windows 배포(NSIS)는 가능하면 **코드 서명**을 적용하고, 번들에 포함되는 sidecar/SDK DLL의 출처/버전을 릴리즈 노트와 해시로 추적합니다(운영 안정성).

**Integration Points:**
- `settings.json`(AppData)에 `boothy.adminPassword`(argon2 파라미터+salt+hash) 저장
- UI 모드 토글 → backend `boothy_admin_login`/`boothy_set_mode` → **UI 노출(visibility) 제어**에 반영
- Tauri backend ↔ sidecar IPC: pipe ACL + 버전드 프로토콜 + 에러 코드 표준화
- 파일시스템 세션 계약: `%USERPROFILE%\\Pictures\\Boothy\\<session>\\{Raw,Jpg}` 경로 검증/강제

**Compliance Requirements:**
- **Offline-first:** 핵심 기능은 오프라인에서 완전 동작(NFR7)
- **Windows-only:** 플랫폼 제약 준수(NFR1)
- **License/Distribution:** RapidRAW(AGPL-3.0) 및 Canon SDK(EDSDK) 배포 조건, digiCamControl(MIT) 사용 조건을 릴리즈/배포 전략에서 명확히 준수해야 합니다. 정책 확정 전에는 **외부 배포를 하지 않고 내부 테스트 배포로 제한**합니다.

### Security Testing

**Existing Security Tests:** 현재 리포에서 자동화된 보안 테스트 체계는 명확히 확인되지 않습니다.

**New Security Test Requirements (MVP):**
- **Password storage:** `settings.json`에 평문 비밀번호가 저장/로그되지 않는지 검증(NFR6)
- **UI hiding:** customer 모드에서 admin 전용 UI/컨트롤이 “비활성화”가 아니라 “숨김”으로 적용되는지 검증(FR17)
- **IPC hardening:** 다른 Windows 사용자/프로세스가 pipe에 접근 가능한지(ACL) 점검, 프로토콜 버전 mismatch 처리 확인
- **Path safety:** 삭제/이동/Export 경로가 세션 루트 밖으로 나갈 수 없는지(경로 정규화/canonicalize) 검증
- **Crash resilience:** sidecar 크래시/연결 끊김 시 앱이 크래시 없이 오류를 표면화하고 계속 탐색/Export 가능한지(FR20)

**Penetration Testing:** MVP의 보안 목표는 “악의적 공격 방어”가 아니라 “일반 사용자 오사용 방지(UX)”이므로, 전통적 펜테스트는 MVP 범위 밖으로 둡니다. 대신 경로 안전성/크래시 내성/로그 품질을 우선 검증합니다.


## Development Environment

### Local Setup (MVP)

**Target OS:** Windows 10/11

**Required Tooling (권장):**
- Node.js `22` + npm (RapidRAW workflows 기준)
- Rust `1.92` (edition `2024`) + `cargo`
- Tauri CLI `2.9.x` (`@tauri-apps/cli`)
- (Windows) Visual Studio 2022 Build Tools + Windows SDK (Tauri 빌드 필수)
- (Sidecar) .NET SDK / Visual Studio (sidecar 타겟 프레임워크에 맞춤)
- (Camera) Canon EDSDK 배포/설치 정책에 따른 DLL 준비(프로젝트 정책에 따름)

**Human-only prerequisites (MVP):**
- Canon EDSDK 재배포 정책 확정(번들링 vs 사용자 설치/제공)
- 대상 PC에 카메라 드라이버/SDK 설치(오프라인 현장 설치 절차 포함)
- 배포 형태 결정(내부 테스트 only vs 외부 배포) 및 라이선스 준수(AGPL 포함)

**Local Run (현재 리포 상태 기준):**
- RapidRAW 레퍼런스 실행: `reference/uxui_presetfunction`에서 `npm install` → `npm start`
- Boothy 앱(타겟): `reference/uxui_presetfunction`을 `apps/boothy`로 승격한 뒤, `apps/boothy`에서 `npm install` → `npm run start`(또는 `npm run tauri dev`)
- Sidecar(타겟): `apps/camera-sidecar`에서 빌드/실행 후, Boothy가 IPC로 연결

**Diagnostics:**
- 앱 로그: `%APPDATA%\\Boothy\\logs\\boothy.log`
- sidecar 로그: `%APPDATA%\\Boothy\\logs\\camera-sidecar.log`
- 세션 데이터: `%USERPROFILE%\\Pictures\\Boothy\\<session>\\{Raw,Jpg}`

## Checklist Results Report (참고)

아래 내용은 내부 체크리스트 기반의 **참고용(히스토리) 검증 결과**입니다. 품질 게이트는 본 문서의 **Testing Strategy**를 기준으로 운영합니다.

### Overall Decision

**PASS with CONCERNS** — 핵심 통합 경계(세션 폴더 계약, sidecar 분리, `.rrdata` 스냅샷)와 NFR(오프라인/실시간/무결성/로그)은 설계가 충분히 구체적입니다. 다만 프론트엔드 세부(상태 관리/컴포넌트 레이아웃 변경 범위)와 sidecar 구현 범위/버저닝, 라이선스/배포 조건은 구현 전 추가 명확화가 필요합니다.

### Pass Rates (By Section)

- **1. Requirements Alignment:** ⚠️ PARTIAL (대부분 커버, 일부 UI/엣지 케이스 상세 부족)
- **2. Architecture Fundamentals:** ✅ PASS
- **3. Technical Stack & Decisions:** ⚠️ PARTIAL (버전/정책은 lockfile로 고정되나 문서상 일부 범위 표기, sidecar/.NET 타겟 확정 필요)
- **4. Frontend Design & Implementation:** ⚠️ PARTIAL (RapidRAW 기반이라는 전제는 있으나 “Boothy 세션 모드 UI”의 구조/상태 흐름을 더 명시하면 구현 리스크↓)
- **5. Resilience & Operational Readiness:** ✅ PASS (오프라인/로그/롤백/자가복구 방향 명확)
- **6. Security & Compliance:** ⚠️ PARTIAL (UX 게이팅은 명확, 라이선스/EDSDK 배포 조건은 추가 검토 필요)
- **7. Implementation Guidance:** ⚠️ PARTIAL (코딩/테스트/개발환경은 정의했으나, 초기 스토리 시퀀싱을 더 구체화하면 좋음)
- **8. Dependency & Integration Management:** ⚠️ PARTIAL (의존성/라이선스 언급은 있으나 업데이트/패치 전략은 간단 수준)
- **9. AI Agent Implementation Suitability:** ✅ PASS (컴포넌트/인터페이스/네이밍/소스 트리 제안이 명시적)
- **10. Accessibility (Frontend Only):** N/A (PRD에 명시 요구 없음; 필요 시 후속 작업)

### Key Concerns / Recommended Follow-ups

1. **Sidecar 프로토콜 표준화:** JSON-RPC 스타일 + `protocolVersion`/error code 표준을 구현에 반영(운영 진단/호환성 확보)
2. **세션 모드 UI 상세:** customer 화면에서 “딱 남길 컴포넌트”와 admin에서 노출할 패널/메뉴의 구체 리스트를 RapidRAW 컴포넌트 레벨로 매핑(구현 리스크↓)
3. **라이선스/배포 검토:** RapidRAW(AGPL) 및 Canon EDSDK 배포 조건(재배포 가능 범위/설치 방식) 확정
4. **성능 목표 검증:** NFR3(≤1s/≤3s)를 만족시키기 위한 watcher 안정화 파라미터(대기 시간/락 체크) 튜닝 계획 수립

## Next Steps

### Story Manager Handoff

아래 프롬프트를 Story Manager(PO/SM)에게 전달해 첫 스토리를 생성하세요:

> `docs/architecture.md`와 `docs/prd.md`를 기준으로 Boothy MVP 통합을 위한 첫 스토리를 작성해 주세요.  
> 핵심 통합 경계는 (1) 세션 폴더 계약: `%USERPROFILE%\\Pictures\\Boothy\\<session>\\{Raw,Jpg}` (세션명=`sessionName` 입력, 폴더 충돌 시 `_YYYY_MM_DD_HH` suffix 또는 기존 세션 열기) (2) RapidRAW 기반 앱에 “세션 모드” 추가 (3) 카메라 기능은 digiCamControl 기반 headless sidecar + Named Pipe IPC 입니다.  
> 첫 스토리는 “세션 생성/활성화 + Raw/Jpg 폴더 생성 + RapidRAW 현재 폴더를 Raw로 고정 + 신규 파일(수동 드롭/테스트 파일) 감지 시 자동 refresh/자동 선택”까지를 범위로 하고, NFR5(무결성)와 NFR3(실시간) 검증 포인트를 포함해 주세요.  
> customer/admin 모드 정책은 UI 숨김 중심이며, 우회 호출 방지는 MVP 범위 밖입니다.

### Developer Handoff

개발자가 바로 착수할 수 있도록, 구현 순서를 다음처럼 권장합니다:

1. **앱 베이스 승격:** `reference/uxui_presetfunction`을 `apps/boothy`로 승격하고(리브랜딩 포함), 빌드/실행 경로를 Boothy 기준으로 고정
2. **세션 매니저:** 세션 폴더 생성/열기 규칙(`sessionName` sanitize + 충돌 처리) + `Raw/`/`Jpg/` 생성 + RapidRAW 폴더 고정
3. **파일 감지→UI 반영:** `Raw/` 신규 파일 안정화 감지 → `.rrdata` 생성(프리셋 스냅샷) → 이미지 리스트 refresh + 자동 선택
4. **프리셋 스냅샷:** “현재 프리셋”을 저장하고, 신규 사진에만 적용(FR8–FR10)
5. **sidecar 통합:** 카메라 sidecar(IPC) 연결/상태/촬영/전송 완료 이벤트까지 확장
6. **모드/숨김:** customer/admin UI 숨김 정책 적용 및 UX 마감
