# Technical Constraints and Integration Requirements

## Existing Technology Stack

**Languages**: TypeScript(React UI), Rust(Tauri backend), WGSL(wgpu shaders), HTML/CSS(Tailwind), JSON(설정/사이드카 등)  
**Frameworks**: Tauri 2.9, React 19, Vite 7, Tailwind 3, Tokio/Rayon(w/ Rust async+parallel), wgpu 28  
**Database**: 업스트림은 “DB 중심”이 아니라 파일 기반(이미지 옆 sidecar `*.rrdata`, Tauri `app_data_dir`의 설정/프리셋 저장). 예약/세션 메타데이터를 위해 `JSON/SQLite` 로컬 저장소를 신규 도입한다.  
**Infrastructure**: 로컬 데스크톱 앱(서버 없음). 번들링/패키징은 Tauri 기반이며 Windows는 NSIS 설치자(업스트림 `tauri.conf.json`의 NSIS 설정 존재).  
**External Dependencies**: Canon EDSDK(사용자 로컬 설치, 경로 설정, 비재배포), RAW 처리(`rawler`), GPU 처리(wgpu/WGSL), 네트워크/WS(reqwest, tokio-tungstenite), (업스트림) AI/모델(ort/ONNX) 및 ComfyUI 연동, (업스트림) Clerk(Auth) 등 — Boothy 스코프에서 필요 기능만 활성/비활성화 필요.

## Integration Approach

**Database Integration Strategy**: 예약 정보/세션 메타(휴대폰번호 뒤4자리, 시작/종료 시각, 세션 폴더 경로, Export 상태, 고객 수령 방식 등)는 Tauri `app_data_dir` 하위에 로컬 저장한다. MVP는 `JSON`으로 시작 가능(구현 단순)하되, 운영 안정/동시성/스키마 확장을 고려하면 `SQLite`로 이행할 수 있도록 “저장소 인터페이스”를 분리한다.  
**API Integration Strategy**: 외부 예약 시스템 연동은 옵션으로 두고, `Adapter` 형태로 분리한다(온라인/오프라인 대응, 타임아웃/재시도 정책 필수). v1에서는 “로컬 DB”만으로도 운영 가능해야 한다.  
**Frontend Integration Strategy**: Customer/Admin Mode 및 세션 상태머신은 App Shell에서 관리한다. Frontend↔Backend는 RapidRAW의 `invoke` 계약(문자열 커맨드)과 이벤트 기반 진행률 패턴을 유지하되, 신규 기능(테더/세션/ExportLock/리셋)에 필요한 커맨드/이벤트를 명확히 추가하고 문서화한다. Customer Mode에서는 네비게이션을 세션 폴더로 제한하고, 기존 패널/모달은 “화이트리스트 방식”으로 노출 범위를 제어한다.  
**Testing Integration Strategy**: 업스트림은 “빌드 중심 CI(테스트 부재/미약)” 경향이 있어, 포크 작업에서는 최소한의 품질 게이트를 추가해야 한다(프론트: lint/typecheck, 백엔드: `cargo check`/핵심 모듈 단위 테스트/스모크 시나리오). 특히 Customer Mode 상태머신/ExportLock/리셋은 회귀 위험이 크므로 자동화된 시나리오 테스트(최소 스모크)를 우선한다.

## Code Organization and Standards

**File Structure Approach**: 업스트림 구조를 유지한다(React: `src/`, Rust: `src-tauri/src/`). 핵심 오케스트레이션은 `src/App.tsx`와 `Invokes`(예: `src/components/ui/AppProperties.tsx`)를 중심으로 한다. 신규 기능은 (예) `src-tauri/src/tethering/*`(EDSDK), `src-tauri/src/session/*`(세션/ExportLock), `src/*/modes/*`(Customer/Admin)처럼 “모듈 단위”로 추가해, `main.rs`의 handler surface를 통제한다.  
**Naming Conventions**: 기존 RapidRAW의 컴포넌트/모듈 네이밍 관례(TypeScript: PascalCase 컴포넌트, camelCase 함수/변수; Rust: snake_case 모듈/함수, PascalCase 타입)를 따른다.  
**Coding Standards**: TypeScript는 ESLint/Prettier/tsconfig 기준을 유지하고, Rust는 edition 2024 + `anyhow` 기반 에러 처리 패턴을 유지한다. Frontend↔Backend 커맨드 이름 변경은 회귀 리스크가 높으므로 “추가” 중심으로 하고, 필요 시 버전드 커맨드/호환 레이어를 둔다.  
**Documentation Standards**: PRD(`docs/prd.md`)를 소스로 유지하고, 커맨드/이벤트 계약(Invokes/emit 이벤트명), Customer Mode 상태머신, 운영자 매뉴얼(EDSDK 경로 설정/장애 대응)을 별도 문서로 축적한다.

## Deployment and Operations

**Build Process Integration**: 업스트림 기준 `npm run start`(tauri dev), `npm run build`(vite build) + `tauri build`(번들링) 흐름을 따른다. Windows 배포는 NSIS 설치자를 기본으로 하고, Boothy 전용 아이덴티티/아이콘/윈도우 설정(전체화면, 키오스크 제약)을 반영한다.  
**Deployment Strategy**: 운영 환경은 Windows 10/11 x64이며, EDSDK는 설치자에 포함하지 않고 “설치 후 경로 지정” 플로우를 제공한다. 개발은 macOS에서 진행하되, 테더링(EDSDK) 기능은 Windows에서만 검증 가능하므로 별도 Windows 테스트/CI 런너가 필요하다.  
**Monitoring and Logging**: 업스트림은 Rust `log`/`fern` 기반 로깅 구성이 있어, Admin Mode에서 “로그 보기/내보내기(파일)”를 제공해 무인 장애 대응을 가능하게 한다. Customer Mode에는 상세 로그를 노출하지 않는다.  
**Configuration Management**: 설정은 Tauri `app_data_dir`에 저장한다(EDSDK 경로, Admin PIN 정책, 세션 폴더 규칙, Export 규칙, 보관/삭제 정책, Lock-down 레벨 등). PIN은 평문 저장을 피하고(해시/솔트), 설정 변경은 Admin에서만 가능하도록 한다.

## Risk Assessment and Mitigation

**Technical Risks**:
- (GPU) wgpu/WGSL 기반 처리: 드라이버/기기별 성능/호환 이슈 가능
- (테더) EDSDK FFI: 이벤트 누락/중복, 연결 안정성, 예외 처리 복잡도
- (백그라운드) Smart Export 큐: 라이브뷰/프리뷰와 리소스 경합 → UI 프리즈 위험
- (업스트림 부채) 커맨드 문자열 계약(tight coupling), 설정/보정의 JSON 스키마 진화 리스크
- (업스트림) 런타임 모델 다운로드/네트워크 의존 기능(Clerk/AI/ComfyUI): 키오스크/오프라인 운영 리스크

**Integration Risks**:
- Customer Mode “네비게이션 제한/모달 억제”가 기존 워크플로우(라이브러리/에디터)와 충돌 가능
- ExportLock(강제 전환/차단)과 기존 Export 파이프라인(이벤트/취소) 간 상태 불일치 가능
- `index.html` 엔트리포인트 불일치(`/src/main.jsx` vs `src/main.tsx`) 등 업스트림 설정/빌드 gotcha가 포크 작업을 불안정하게 만들 수 있음

**Deployment Risks**:
- 개발환경(macOS)과 운영환경(Windows) 괴리로 EDSDK 관련 결함이 릴리스 직전까지 숨을 수 있음
- EDSDK 배포 제약(EULA)으로 설치/경로 설정 UX가 실패하면 “앱 사용 불가”로 직결

**Mitigation Strategies**:
- 업스트림 커밋/버전을 핀(pin)하고, 커맨드/이벤트 계약은 “추가” 위주로 관리(변경 최소화)
- Customer Mode 상태머신/ExportLock/리셋에 대해 스모크 자동화(최소 시나리오) + 로깅/리포트 내보내기 제공
- Smart Export 큐는 우선순위/스로틀링 정책을 명시하고(프리뷰/라이브뷰 우선), 큐 백로그를 UI로 가시화
- EDSDK는 “경로 검증 + 자체 진단 + 재시도”를 Idle/Setup의 자동 점검에 포함하고, Admin에서 상세 원인/로그 제공
- 키오스크 운영에서 불필요한 네트워크 의존 기능(Clerk/AI/ComfyUI 등)은 MVP에서 비활성화하거나 완전 제거(명시적 스코프 관리)

---
