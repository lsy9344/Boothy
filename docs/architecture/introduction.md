# Introduction

이 문서는 **Boothy**를 단일 **Windows** 데스크탑 앱(**Tauri + React**)으로 확장하여, 촬영 → 실시간 확인 → 프리셋/편집 → 내보내기까지 하나의 UX로 통합하기 위한 목표(TO‑BE) 아키텍처를 정의합니다. 주요 목적은 AI 기반 개발(스토리/태스크 실행)이 기존(브라운필드) 리포지토리 현실과 충돌하지 않도록, 통합 지점과 경계를 명확히 하는 것입니다.

**기존 아키텍처와의 관계:**
현재 리포는 1차 Boothy 앱 코드가 아직 없고, 두 개의 OSS 레퍼런스 스택이 `reference/` 아래에 있습니다. 본 문서는 `docs/brownfield-architecture.md`의 AS‑IS 분석을 보완하며, 신규 1차 컴포넌트가 레퍼런스 스택과 어떻게 상호작용/이행(migration)할지 규정합니다. 기존 패턴과 충돌 시, 일관성을 유지하기 위한 우선순위를 제시합니다.

## 범위 적합성 및 입력(검증용)

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

## Existing Project Analysis

### Current Project State

- **Primary Purpose:** RapidRAW(편집/프리셋/익스포트)를 기반으로 **Boothy로 리브랜딩/변형**하고, 카메라 촬영 워크플로우를 통합한 Windows 포토부스 앱(Tauri+React)을 만들기 위한 브라운필드 베이스(레퍼런스 코드 + 문서).
- **Current Tech Stack:** (레퍼런스) RapidRAW: React `19.2.3` + Vite `7.3.0` + Tauri `2.9.x` + Rust(edition `2024`, rust-version `1.92`), (레퍼런스) digiCamControl: C#/.NET Framework `4.0` + WPF + Canon EDSDK, (문서) `docs/*.md`.
- **Architecture Style:** 현재 리포는 “단일 앱” 아키텍처가 아니라, 서로 독립적인 두 앱(편집/카메라)을 같은 리포에 둔 형태입니다. RapidRAW는 Tauri(Rust) ↔ React(프론트) 커맨드/이벤트 중심 구조이고, digiCamControl은 전역 서비스 로케이터 + 이벤트 기반 디바이스 라이프사이클 + Named Pipe IPC 패턴이 관찰됩니다.
- **Deployment Method:** 리포 루트에 1차 Boothy 앱 빌드/배포 파이프라인은 아직 없고, 레퍼런스 스택별로 빌드/패키징 방식이 다릅니다(RapidRAW: GitHub Actions 기반 릴리즈, digiCamControl: Visual Studio 솔루션 + Setup/NSIS).

### Available Documentation

- `docs/prd.md`: 통합 UX/기능 요구사항 + NFR(실시간/백그라운드 처리/오프라인/보안 로그 등)
- `docs/brownfield-architecture.md`: 현재 리포 실체(레퍼런스 스택, 관찰된 패턴, 기술부채, IPC 정보 등)
- `docs/design_concept.md`: customer/admin 모드 정책 + “숨김(비활성화 금지)” UX 규칙 + RapidRAW 스타일 통일 지시
- `reference/uxui_presetfunction/README.md`, `reference/uxui_presetfunction/src-tauri/*`: 편집/프리셋/익스포트 스택 구조 및 의존성 근거
- `reference/uxui_presetfunction/.github/workflows/*`: 레퍼런스 앱의 CI/릴리즈 흐름(참고용)
- `reference/camerafunction/digiCamControl-2.0.0/Docs/` 및 소스: 카메라 제어/캡처/Named Pipe remote cmd 패턴 근거

### Identified Constraints

- **플랫폼/런타임:** Windows-only(NFR1), 오프라인 필수(NFR7)
- **UI/제품 형태:** Tauri + React만 허용(NFR2), WPF UI 금지(기능 참고만)(NFR2/`docs/design_concept.md`)
- **실시간성:** 전송 완료 후 세션 리스트 반영 ≤ 1s 목표, 메인 뷰 프리셋 적용 프리뷰 ≤ 3s 목표(NFR3)
- **성능/반응성:** 프리셋 적용/RAW 처리/익스포트는 백그라운드 처리로 UI block 금지(NFR4)
- **데이터 무결성:** 전송 완료 전 파일을 “수입(import) 완료”로 간주하면 안 됨, partial transfer로 손상된 import 방지(NFR5)
- **보안:** admin 비밀번호는 salted hash로 안전 저장, 평문 저장/로그 금지(NFR6)
- **오프라인/무계정 정책:** Boothy MVP는 **로그인 없이** 동작하고, **기본적으로 네트워크 호출을 하지 않음**(NFR7). RapidRAW 레퍼런스에 포함된 온라인 기능(예: Clerk/auth, 커뮤니티, 모델 다운로드 등)은 Boothy 제품 빌드에서 제거/비활성화가 필요합니다.
- **관찰된 레거시/의존성:** 카메라 레퍼런스(digiCamControl)는 .NET Framework 4.0/WPF 기반이며, Canon EDSDK 등 네이티브/아키텍처(x86/x64) 의존성이 존재(브리징/이행 전략 필요)
- **세션 폴더 계약(TO‑BE):**
  - **세션 루트(`sessionsRoot`) 고정값(MVP):** `%USERPROFILE%\\Pictures\\dabi_shoot` (MVP에서는 변경 불가)
  - **세션 생성/열기:** “세션 시작 시” 사용자가 `sessionName`을 입력하면 해당 값으로 세션 폴더를 생성/활성화(존재 시 열기) (FR3)
  - **폴더명 규칙:** `sessionName`은 폴더명으로 안전하게 변환(sanitize)하여 `sessionFolderName`으로 저장(Windows 금지 문자 제거/치환, 길이 제한 등)
  - **중복/충돌 처리:** 동일 `sessionFolderName`이 이미 있고 “새 세션”이 필요하면 `YYYY_MM_DD_HH` suffix로 새 폴더 생성(예: `Wedding_2026_01_14_15`) (선택 UX)
- **하위 폴더:** `Raw/`(촬영 원본 저장), `Jpg/`(Export 결과 저장)
- **리포 현실:** 대용량 벤더 바이너리 포함 및 중첩 git 등으로 운영/빌드/보안(공급망) 고려 필요(`docs/brownfield-architecture.md`)
- **문서 상태:** 코딩 표준/소스 트리/개발환경/테스트 전략은 현재 `docs/architecture.md`에 포함되어 있으며, 필요 시 별도 문서로 분리합니다.
- **라이선스/배포 정책:** RapidRAW(AGPL-3.0)는 의무를 수용하며(라이선스 고지 + 대응 소스 제공), Canon EDSDK는 **내부 매장 배포용 인스톨러에 DLL 번들링**하는 것으로 확정합니다. (근거: `docs/decisions/adr-002-agpl-compliance.md`, `docs/decisions/adr-003-canon-edsdk-bundling.md`)

### Key Architectural Decisions (TO‑BE)

1. **앱 베이스:** RapidRAW를 제품 베이스로 채택하고 Boothy로 리브랜딩/변형합니다(렌더/프리셋/Export 코어는 호환성 유지, CR1).
2. **카메라 통합 전략(MVP):** digiCamControl을 기능 레퍼런스로 삼아, headless **Camera Sidecar Service**로 제공하고 Boothy(Tauri backend)가 Named Pipe IPC로 제어/이벤트를 수신합니다(FR19–FR21).
3. **세션 계약:** 세션은 폴더로 표현되며 `sessionsRoot` 아래에서 “활성 세션 1개”만 유지합니다. 세션 폴더 내부에 `{Raw,Jpg}`를 사용합니다(FR3/FR6/FR12).

### Camera Integration: Option A 리스크 및 Option B 해석

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

## Change Log

| Change | Date | Version | Description | Author |
| --- | --- | --- | --- | --- |
| Initial draft | 2026-01-13 | 0.1 | Start target architecture based on `docs/prd.md` and current repo analysis | Winston |
| PRD-aligned TO-BE | 2026-01-14 | 1.0 | Align session contract (`sessionName`, fixed `sessionsRoot`) and remove interactive checkpoints | Codex |
| Sharded docs + VCS alignment | 2026-01-14 | 1.1 | Add `docs/architecture/*` sharded references and align notes with repo `.gitignore` | Codex |
