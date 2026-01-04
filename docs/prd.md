# Boothy (RapidTetherRAW) Brownfield Enhancement PRD

문서 버전: v0.1 (Draft)  
작성일: 2026-01-02  
대상 릴리스: MVP → 1.0  

---

## Intro Project Analysis and Context

### Existing Project Overview

- Boothy(working title: RapidTetherRAW)는 **RapidRAW(업스트림)** 를 기반으로 하는 **Windows 전용** 데스크톱 앱을 목표로 한다.
- RapidRAW 업스트림은 **Tauri 2** 기반 데스크톱 앱이며, 대략적인 구성은 다음과 같다.
  - Frontend: React + TypeScript + Vite + Tailwind
  - Backend: Rust (Tauri commands, RAW develop, GPU processing(wgpu/WGSL), AI features 등)
  - 비파괴 워크플로우: 이미지 옆에 sidecar(`*.rrdata`)로 보정값 저장
- 현재 작업공간에는 업스트림 코드가 `upstream/RapidRAW` 에 포함되어 있고, 상세한 구조/엔트리포인트/연동 포인트는 `docs/brownfield-architecture.md` 에 정리되어 있다.

### Available Documentation Analysis

- `docs/brownfield-architecture.md`: 업스트림(RapidRAW) 기반 **현재 코드 구조/연동 포인트/기술 부채/리스크** 정리(문서-프로젝트 분석 결과)
- `project-analysis.md`: Boothy(포크 컨텍스트) 관점에서의 핵심 핫스팟 요약
- `prd-rapidraw-customer-mode-v1.1.1.md`: Customer/Admin Mode + 세션/ExportLock/리셋 + EDSDK 테더링 요구사항(한국어 PRD)

### Understanding to Validate (Assumptions)

아래 항목은 현재 저장소의 문서/분석 결과를 기반으로 정리한 “현 상태 이해”이다. Requirements 작성 전에 사용자 확인이 필요하다.

1. (플랫폼) **Windows-only** 로 MVP/1.0을 개발한다.
2. (카메라) 지원 기종은 **Canon EOS 700D 단일 기종**이다.
3. (테더링) 테더링은 **Canon EDSDK를 앱에 동봉/재배포하지 않고**, 사용자가 로컬 설치 후 **경로 설정**으로 연결한다.
4. (캡처 저장) 촬영 결과물은 **active session folder에 직접 저장**되며, 앱은 이를 즉시 반영(리스트/썸네일 갱신)한다.
5. (Customer Mode 네비게이션) Customer Mode에서 파일/폴더 탐색은 **세션 폴더로 제한**된다.
6. (ExportLock) Export 중에는 촬영이 잠기며, Export 완료 전에는 **다음 세션 시작이 불가**하다.
7. (프라이버시 리셋) 리셋은 **앱 상태/캐시/백그라운드 작업 정리**에 한정되며, 세션 폴더의 이미지/사이드카 파일은 **삭제하지 않는다**.
8. (Export 파일명) Export 파일명 패턴은 `휴대폰뒤4자리-{hh}시-{sequence}` 이며, `{hh}`/`{sequence}`는 업스트림의 템플릿 로직을 활용하고 `휴대폰번호 뒤4자리`는 런타임에 주입한다.

Confirmed by user: 2026-01-02

### Enhancement Scope Definition

본 PRD는 “기존 코드(RapidRAW) 위에 상당한 규모의 동작/UX/백엔드 커맨드/새 모듈(테더링)을 추가”하는 **브라운필드 대형 개선**을 대상으로 한다. 현재 저장소 컨텍스트 기준으로 포함 범위는 다음과 같다.

- **Customer Mode(키오스크 플로우)**: 예약 확인 → 필터 선택 → 촬영 → 자동 Export/전송 → 자동 리셋의 단일 상태머신(Full-screen + 최소 조작)
- **Admin Mode(PIN)**: 운영/관리 기능(프리셋, Export 규칙, 장애 진단 등)
- **세션 타이머 + ExportLock 게이트**: 시간 종료 시 촬영 잠금 및 Export 강제 전환, Export 완료 전 다음 세션 시작 불가
- **프라이버시 리셋**: 앱 상태/캐시 초기화, 백그라운드 작업 정리(세션 폴더 파일/사이드카 삭제 금지)
- **Canon EOS 700D 테더링(EDSDK)**: Windows-only, 로컬 설치된 EDSDK를 경로 설정으로 사용(재배포/동봉 없음), 캡처 결과는 세션 폴더로 직접 저장

### Goals and Background Context

- 무인 셀프 스튜디오에서 고객이 “컴퓨터 조작”이 아니라 **단순 촬영/선택 경험**만 하도록 UX를 최소화한다.
- 운영자는 Admin Mode에서 카메라/프리셋/Export 정책을 관리하고, 고객 모드에서는 실수 가능성을 구조적으로 제거한다.
- 기존 RapidRAW의 비파괴 편집/프리셋/Export 파이프라인 장점을 유지하면서, 테더 촬영 및 세션 기반 운영을 추가한다.

### Change Log

| Date       | Version | Description | Author |
| ---------- | ------- | ----------- | ------ |
| 2026-01-02 | v0.1    | Initial draft from existing PRD + brownfield analysis | PM (John) |

---

## Requirements

These requirements are based on my understanding of your existing system. Please review carefully and confirm they align with your project's reality.

### Functional

1. FR1: 앱은 기본적으로 **Customer Mode**로 부팅한다.
2. FR2: **Admin Mode**는 **PIN**으로만 진입 가능하며(설정 가능), Customer Mode에서는 고급 설정/진단/수동 Export 옵션을 숨긴다.
3. FR3: Customer Mode는 다음 **단일 상태머신(railroaded workflow)** 만 제공한다: `Idle → Setup → Capture → ExportLock → Complete → Reset` (임의 화면 이동/우회 불가).
4. FR4: Idle 단계에서 고객 입력은 `QR 스캔(가능 시)` 또는 `휴대폰번호 뒤4자리 입력`을 지원한다.
5. FR5: 예약 정보 소스는 최소 1개를 지원한다: `로컬 DB(JSON/SQLite)` (옵션: 외부 예약 시스템 API 어댑터).
6. FR6: 세션 시작 시 앱이 예약 정보를 확인하고, 고객이 폴더명/경로를 입력하지 않으며 **세션 폴더를 자동 생성**한다(기본 예시: `YYYYMMDD_HH00_휴대폰뒤4자리_이름(선택)`, 규칙은 Admin에서 설정 가능).
7. FR7: Customer Mode에서 파일/폴더 탐색은 **세션 폴더로 제한**되며, 파일 시스템/저장 경로/Export 상세 옵션을 고객 UI에 노출하지 않는다.
8. FR8: Canon **EOS 700D** 테더링(EDSDK)을 지원한다: 연결/해제/재연결, 저장 위치 `PC(Host)` 강제, 촬영 이벤트 핸들러 등록/정리(Object/Property/State).
9. FR9: **라이브뷰**는 Customer Mode에서 메인 화면 내 임베드를 우선하며, Admin Mode에서는 팝업 제공을 허용한다(가능 범위).
10. FR10: Customer Mode 촬영은 **앱 트리거(큰 버튼/터치) 우선**으로 동작한다. (옵션) 물리 리모컨 촬영 감지는 운영 정책에 따라 on/off 가능하다.
11. FR11: 촬영 파일 수신 후 앱은 다음을 수행한다: (1) 세션 폴더에 저장 (2) UI에 즉시 로드 및 최근 촬영본 자동 선택 (3) 현재 선택된 “캡처 프리셋”을 **이후 촬영분의 초기값**으로 자동 적용.
12. FR12: 비파괴 워크플로우를 유지한다: 각 RAW 옆에 sidecar(예: `.rrdata` 또는 호환 포맷)를 저장하고, 재오픈 시 편집 값이 복원된다.
13. FR13: “캡처 프리셋”은 **새로 들어오는 촬영본의 초기값**에만 적용되며, 과거 촬영본의 보정값은 자동으로 바뀌지 않는다.
14. FR14: Customer Mode UI는 최소 컨트롤만 노출한다: 예약 확인/필터 선택/촬영/끝내기/수령 안내(필요 시 필터 강도 1개 슬라이더). 현상 슬라이더/커브/Undo 등 고급 기능은 Admin 전용(또는 옵션)으로 제공한다.
15. FR15: Admin Mode는 프리셋(필터) 관리 기능을 제공한다: 추가/삭제/정렬, 미리보기 생성, “캡처 프리셋”/“일반 프리셋” 개념 유지, Customer Mode에서 노출할 최대 프리셋 개수 제한(예: 6~12개, 설정).
16. FR16: **카메라 세팅 Lock(가드레일)** 을 제공한다: 세션 시작 시 지정 값 강제 적용(ISO/셔터/조리개/WB 등), 세션 중 변경 감지(폴링/이벤트) 및 자동 복구, 반복 변경 시 경고 및 필요 시 Capture 잠금(안전 모드), 상태를 UI에 표시(Customer는 단순/관리자는 상세).
17. FR17: Export 기능은 JPEG(필수)를 지원하고, 단일/다중/세션 전체 Export를 지원한다(수동 Export는 Admin 중심, Customer는 자동 흐름).
18. FR18: Export 파일명은 템플릿 기반이며 기본 패턴은 `휴대폰뒤4자리-{hh}시-{sequence}` 를 지원한다. `{hh}`/`{sequence}`는 템플릿 플레이스홀더로 동작하고, `휴대폰번호 뒤4자리`는 예약 확인 결과를 런타임에 주입한다.
19. FR19: **Smart Export Pipeline** 을 제공한다: 촬영본 수신 시 백그라운드 큐에 JPEG 생성 작업을 enqueue하고, 큐 상태(남은 n장)를 UI에 표시한다(고객은 단순, 관리자는 상세/재시도).
20. FR20: **ExportLock(게이트)** 동작을 제공한다: 타이머 만료 또는 고객 “끝내기” 시 ExportLock으로 강제 전환되고 촬영(셔터 트리거)이 차단된다. Export가 완료되기 전에는 다음 세션 시작이 불가하다.
21. FR21: ExportLock 화면에서 고객이 할 수 있는 동작은 최소화한다: 진행률 보기 + 도움 요청(연락처/원격지원 코드). 완료 후에는 수령 안내(QR/이메일/로컬 출력 폴더 등 옵션) 후 자동 카운트다운으로 Reset된다.
22. FR22: **프라이버시 리셋**은 세션 종료 시 자동 수행한다: UI에서 이전 세션 사진 제거, 임시 캐시/프리뷰 삭제, 백그라운드 작업 정리. (중요) Reset은 세션 폴더의 이미지/sidecar 파일을 삭제하지 않는다.
23. FR23: (옵션) Admin에서 세션 보관/삭제 정책을 설정할 수 있다(기간/디스크 임계치). 이 정책은 “Reset(즉시 초기화)”와 분리된 운영/정리 기능으로 취급한다.
24. FR24: **장애 UX/헬프 플로우**를 제공한다: Idle/Setup 단계에서 자동 점검(카메라/EDSDK 경로/저장 권한/디스크/이전 Export 미완료/큐 상태)을 수행하고, 실패 시 Customer는 “진행 불가 + 도움 요청”만 제공한다. Admin에는 오류 코드/로그/재시도/리포트 내보내기 등을 제공한다.
25. FR25: 무인 운영 안정성을 위해 **UI Lock-down 레벨(0~2)** 을 제공한다(가능 범위 내): 작업표시줄/창 닫기/Alt+Tab 등 일부 시스템 인터랙션 제한을 옵션으로 구성한다.

### Non Functional

1. NFR1: 촬영 후 “최근 촬영본 표시”는 목표 **2초 이내**를 달성한다(Windows 운영환경 기준).
2. NFR2: Customer Mode에서 UI 프리즈(멈춤)를 허용하지 않는다. Export/Smart Export 큐는 백그라운드에서 처리한다.
3. NFR3: 백그라운드 JPEG 생성이 프리뷰/라이브뷰를 방해하지 않도록 우선순위/쓰레드/스로틀링을 조정한다.
4. NFR4: 케이블 분리/절전/오류 등 비정상 상태에서 명확한 상태 전환과 재연결 경로를 제공한다.
5. NFR5: 촬영 이벤트 중복/누락을 방지한다(파일 전송 완료 확인 후 처리, 중복 이벤트 방어).
6. NFR6: 충돌/강제 종료 후에도 마지막 sidecar 기준으로 복구 가능해야 한다(비파괴 편집 값 보존).
7. NFR7: 무인 운영 프라이버시를 최우선으로 한다: 세션 격리, 자동 리셋, 캐시 정리, Customer Mode에서 OS 제어 최소화(가능 범위).
8. NFR8: 라이선스/컴플라이언스를 준수한다: RapidRAW 기반 **AGPL-3.0** 준수, Canon EDSDK 바이너리/헤더/런타임을 저장소 및 배포물에 포함하지 않는다.

### Compatibility Requirements

1. COMP1: RapidRAW의 비파괴 편집(사이드카) 및 프리셋 개념(캡처 프리셋/일반 프리셋)을 유지하거나, 변경 시 마이그레이션/호환성 전략을 제공한다.
2. COMP2: 기존 RapidRAW의 RAW 현상 엔진/보정 파라미터 기본 동작(특히 sidecar 기반 복원)을 깨지 않도록 통합한다.
3. COMP3: 운영 OS는 Windows 10/11 x64를 목표로 하며, 배포/설치 경로에서 안정적으로 동작해야 한다.
4. COMP4: Canon EOS 700D + 사용자 로컬 설치 EDSDK에 대한 경로 설정 방식과 배포 제약(비동봉/비재배포)을 일관되게 유지한다.
5. COMP5: Export/진행 이벤트/백그라운드 작업 등은 기존 RapidRAW의 구조적 패턴(커맨드 호출, 진행 이벤트, 캐시/썸네일 관리)과 무리 없이 통합되도록 설계한다.

---

## User Interface Enhancement Goals

### Integration with Existing UI

- RapidRAW 업스트림의 UI 구조(React + TS + Tailwind)와 컴포넌트 레이어를 유지하고, “Customer/Admin Mode + 세션 상태머신”은 **최상위 App Shell 레벨에서 오케스트레이션**한다(예: `App.tsx` 중심).
- Customer Mode 화면(Idle/Setup/Capture/ExportLock/Complete/Reset)은 “새로운 화면 집합”이지만, 이미지 미리보기/필름스트립/패널 등은 가능한 한 **기존 컴포넌트를 재사용**하고 “노출/권한/네비게이션 제한”으로 구현한다.
- Backend와의 연동은 기존 RapidRAW 패턴(tauri `invoke()` + progress/event emit)을 따르되, Customer Mode에서는 이벤트/에러를 **짧고 단일 행동(도움 요청)** 중심으로 표현한다.
- “UI Lock-down 레벨(0~2)”은 Tauri 창/단축키/시스템 인터랙션 제한(가능 범위)에 맞춰 옵션화하고, UI에는 현재 레벨/제한 상태를 운영자(Admin)에게만 노출한다.

### Modified/New Screens and Views

- 신규(고객): `Idle`(예약 입력/상태 점검), `Setup`(예약 확인 + 필터 선택 + 시작), `Capture`(큰 타이머 + 라이브뷰/최근 사진 + 촬영/끝내기), `ExportLock`(진행률 + 도움 요청), `Complete/Reset`(수령 안내 + 자동 초기화 카운트다운)
- 신규(관리자): `Admin PIN`(진입), `Camera Settings & Lock Policy`, `Preset Management`(정렬/미리보기), `Export Rules`, `Diagnostics/Logs`, `Retention/Cleanup Policy`
- 수정(공통): App Shell(모드 전환/상태머신/라우팅), Export 진행 UX(ExportLock), 오류/헬프 플로우, 세션 폴더 기반 네비게이션 제한(고객)
- 수정(업스트림 기반): 라이브러리/필름스트립/에디터 화면에서 Customer Mode 전용 “컨트롤 최소화/패널 숨김/모달 화이트리스트” 적용

### UI Consistency Requirements

- Tailwind 기반 스타일/레이아웃/색상 토큰을 유지하고, 새 UI는 가능한 한 기존 컴포넌트/패턴(버튼, 모달, 패널 구조)을 재사용한다.
- Customer Mode는 “큰 터치 타깃 + 최소 텍스트 + 명확한 진행 상태(타이머/Export/큐 잔량)”를 표준으로 한다.
- 상태 전환은 “알림”이 아니라 **강제 전환**으로 구현하며(타이머 만료 → ExportLock), 고객이 다음 행동을 판단하지 않도록 한다.
- 에러는 고객에게 “짧고 구체적인 문장 + 도움 요청 1가지 액션”만 제공하고, 상세 원인/로그/재시도는 Admin 전용으로 제공한다.

---

## Technical Constraints and Integration Requirements

### Existing Technology Stack

**Languages**: TypeScript(React UI), Rust(Tauri backend), WGSL(wgpu shaders), HTML/CSS(Tailwind), JSON(설정/사이드카 등)  
**Frameworks**: Tauri 2.9, React 19, Vite 7, Tailwind 3, Tokio/Rayon(w/ Rust async+parallel), wgpu 28  
**Database**: 업스트림은 “DB 중심”이 아니라 파일 기반(이미지 옆 sidecar `*.rrdata`, Tauri `app_data_dir`의 설정/프리셋 저장). 예약/세션 메타데이터를 위해 `JSON/SQLite` 로컬 저장소를 신규 도입한다.  
**Infrastructure**: 로컬 데스크톱 앱(서버 없음). 번들링/패키징은 Tauri 기반이며 Windows는 NSIS 설치자(업스트림 `tauri.conf.json`의 NSIS 설정 존재).  
**External Dependencies**: Canon EDSDK(사용자 로컬 설치, 경로 설정, 비재배포), RAW 처리(`rawler`), GPU 처리(wgpu/WGSL), 네트워크/WS(reqwest, tokio-tungstenite), (업스트림) AI/모델(ort/ONNX) 및 ComfyUI 연동, (업스트림) Clerk(Auth) 등 — Boothy 스코프에서 필요 기능만 활성/비활성화 필요.

### Integration Approach

**Database Integration Strategy**: 예약 정보/세션 메타(휴대폰번호 뒤4자리, 시작/종료 시각, 세션 폴더 경로, Export 상태, 고객 수령 방식 등)는 Tauri `app_data_dir` 하위에 로컬 저장한다. MVP는 `JSON`으로 시작 가능(구현 단순)하되, 운영 안정/동시성/스키마 확장을 고려하면 `SQLite`로 이행할 수 있도록 “저장소 인터페이스”를 분리한다.  
**API Integration Strategy**: 외부 예약 시스템 연동은 옵션으로 두고, `Adapter` 형태로 분리한다(온라인/오프라인 대응, 타임아웃/재시도 정책 필수). v1에서는 “로컬 DB”만으로도 운영 가능해야 한다.  
**Frontend Integration Strategy**: Customer/Admin Mode 및 세션 상태머신은 App Shell에서 관리한다. Frontend↔Backend는 RapidRAW의 `invoke` 계약(문자열 커맨드)과 이벤트 기반 진행률 패턴을 유지하되, 신규 기능(테더/세션/ExportLock/리셋)에 필요한 커맨드/이벤트를 명확히 추가하고 문서화한다. Customer Mode에서는 네비게이션을 세션 폴더로 제한하고, 기존 패널/모달은 “화이트리스트 방식”으로 노출 범위를 제어한다.  
**Testing Integration Strategy**: 업스트림은 “빌드 중심 CI(테스트 부재/미약)” 경향이 있어, 포크 작업에서는 최소한의 품질 게이트를 추가해야 한다(프론트: lint/typecheck, 백엔드: `cargo check`/핵심 모듈 단위 테스트/스모크 시나리오). 특히 Customer Mode 상태머신/ExportLock/리셋은 회귀 위험이 크므로 자동화된 시나리오 테스트(최소 스모크)를 우선한다.

### Code Organization and Standards

**File Structure Approach**: 업스트림 구조를 유지한다(React: `src/`, Rust: `src-tauri/src/`). 핵심 오케스트레이션은 `src/App.tsx`와 `Invokes`(예: `src/components/ui/AppProperties.tsx`)를 중심으로 한다. 신규 기능은 (예) `src-tauri/src/tethering/*`(EDSDK), `src-tauri/src/session/*`(세션/ExportLock), `src/*/modes/*`(Customer/Admin)처럼 “모듈 단위”로 추가해, `main.rs`의 handler surface를 통제한다.  
**Naming Conventions**: 기존 RapidRAW의 컴포넌트/모듈 네이밍 관례(TypeScript: PascalCase 컴포넌트, camelCase 함수/변수; Rust: snake_case 모듈/함수, PascalCase 타입)를 따른다.  
**Coding Standards**: TypeScript는 ESLint/Prettier/tsconfig 기준을 유지하고, Rust는 edition 2024 + `anyhow` 기반 에러 처리 패턴을 유지한다. Frontend↔Backend 커맨드 이름 변경은 회귀 리스크가 높으므로 “추가” 중심으로 하고, 필요 시 버전드 커맨드/호환 레이어를 둔다.  
**Documentation Standards**: PRD(`docs/prd.md`)를 소스로 유지하고, 커맨드/이벤트 계약(Invokes/emit 이벤트명), Customer Mode 상태머신, 운영자 매뉴얼(EDSDK 경로 설정/장애 대응)을 별도 문서로 축적한다.

### Deployment and Operations

**Build Process Integration**: 업스트림 기준 `npm run start`(tauri dev), `npm run build`(vite build) + `tauri build`(번들링) 흐름을 따른다. Windows 배포는 NSIS 설치자를 기본으로 하고, Boothy 전용 아이덴티티/아이콘/윈도우 설정(전체화면, 키오스크 제약)을 반영한다.  
**Deployment Strategy**: 운영 환경은 Windows 10/11 x64이며, EDSDK는 설치자에 포함하지 않고 “설치 후 경로 지정” 플로우를 제공한다. 개발은 macOS에서 진행하되, 테더링(EDSDK) 기능은 Windows에서만 검증 가능하므로 별도 Windows 테스트/CI 런너가 필요하다.  
**Monitoring and Logging**: 업스트림은 Rust `log`/`fern` 기반 로깅 구성이 있어, Admin Mode에서 “로그 보기/내보내기(파일)”를 제공해 무인 장애 대응을 가능하게 한다. Customer Mode에는 상세 로그를 노출하지 않는다.  
**Configuration Management**: 설정은 Tauri `app_data_dir`에 저장한다(EDSDK 경로, Admin PIN 정책, 세션 폴더 규칙, Export 규칙, 보관/삭제 정책, Lock-down 레벨 등). PIN은 평문 저장을 피하고(해시/솔트), 설정 변경은 Admin에서만 가능하도록 한다.

### Risk Assessment and Mitigation

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

## Epic and Story Structure

### Epic Approach

**Epic Structure Decision**: 단일 Epic (Single Epic) — “Customer Mode + EOS 700D Tethering + Smart Export/Reset 운영 플로우”로 하나의 통합 Epic을 권장한다.

**Rationale**:

- Customer Mode 상태머신(Idle/Setup/Capture/ExportLock/Reset), 테더링(EDSDK), Smart Export 큐, ExportLock 게이트, 프라이버시 리셋은 **서로 강하게 결합된 end-to-end 운영 플로우**로, 각각을 독립적으로 “완료” 정의하기 어렵다.
- 핵심 리스크가 “통합 지점(Frontend↔Backend 커맨드/이벤트 계약, 백그라운드 작업/리소스 경합, 세션 상태 전이)”에 집중되어 있어, Epic을 나누면 **부분 최적화/통합 지연**으로 오히려 일정·리스크가 증가할 가능성이 높다.
- 대신 단일 Epic 내부를 “기반(상태머신/저장소/진단) → 테더 → 프리셋/보정 적용 → Smart Export → ExportLock/Complete/Reset → Admin/운영 정책”처럼 **리스크 최소화 순서로 Story를 세분화**하는 접근이 브라운필드에 적합하다.

Based on my analysis of your existing project, I believe this enhancement should be structured as **single epic** because it is one tightly coupled, end-to-end kiosk workflow that must ship together (state machine + tether + export gate + reset). Does this align with your understanding of the work required?
Confirmed by user: 2026-01-02

---

## Epic 1: RapidTetherRAW Customer Mode MVP

**Epic Goal**: RapidRAW 기반(React+TS+Tauri+Rust)의 강점을 유지하면서, 무인 셀프 스튜디오 운영을 위한 Customer Mode 상태머신(Idle→Setup→Capture→ExportLock→Complete→Reset)과 Admin Mode(PIN), EOS 700D EDSDK 테더링, 캡처 프리셋 자동 적용, Smart Export Pipeline, ExportLock 게이트, 프라이버시 리셋/운영 진단을 통합해 “촬영→수령→초기화”의 안정적인 end-to-end 경험을 제공한다.

**Integration Requirements**:
- (라이선스/배포) RapidRAW 기반 **AGPL-3.0** 준수, Canon **EDSDK 비재배포/비동봉**(사용자 로컬 설치 + 경로 설정).
- (플랫폼) 운영 환경은 **Windows 10/11 x64**. 개발은 macOS에서 진행 가능하나 EDSDK는 Windows에서만 실검증 필요.
- (데이터) 비파괴 워크플로우 유지: RAW 옆 sidecar(`*.rrdata` 또는 호환) 저장 및 복원.
- (파일/네비게이션) Customer Mode는 **세션 폴더 격리** 및 파일 탐색/저장 경로 노출 금지.
- (커맨드 계약) Frontend↔Backend는 기존 `invoke`/event 패턴을 유지하되, 신규 커맨드/이벤트는 명시적으로 추가·문서화(기존 이름 변경 최소화).
- (Export) 파일명 템플릿 `휴대폰뒤4자리-{hh}시-{sequence}` 지원(휴대폰번호 뒤4자리 런타임 주입) 및 ExportLock 상태/진행률 계약 일관성 유지.
- (운영) 무인 장애 대응을 위한 Admin 진단/로그 내보내기, 리셋의 안정적 작업 취소/정리(세션 파일 삭제 금지).

### Story 1.1 Mode Shell & Guided State Machine Scaffold

As a studio operator,  
I want the app to launch into a guided Customer Mode with a clear Admin escape hatch,  
so that customers cannot access advanced controls or the filesystem.

#### Acceptance Criteria
1: 앱은 기본으로 Customer Mode `Idle` 화면으로 부팅한다.  
2: Admin PIN 진입 동선이 존재하며, 성공 시 Admin Mode로 전환된다(최소 UI/라우팅 완성).  
3: Customer Mode에서는 파일 탐색/고급 패널/수동 Export 옵션이 노출되지 않는다(화이트리스트 방식).  
4: Admin Mode에서는 기존 RapidRAW의 “이미지 열기/보정/Export” 핵심 플로우가 유지된다(회귀 방지).  
5: Customer Mode 기능은 설정 플래그로 비활성화 가능해야 한다(롤백/운영 비상용).  

#### Integration Verification
IV1: Admin Mode에서 기존 RapidRAW 편집/Export가 정상 동작한다.  
IV2: 모드 라우팅 추가가 기존 `invoke`/event 리스너를 깨지 않는다.  
IV3: 앱 시작/화면 전환 성능이 기존 대비 유의미하게 악화되지 않는다.

### Story 1.2 Admin PIN & Configuration Storage

As a studio operator,  
I want to configure kiosk settings (PIN, session root, export rules, EDSDK path) in one place,  
so that the kiosk can be operated consistently without ad-hoc manual steps.

#### Acceptance Criteria
1: Admin PIN 설정/변경이 가능하며, 저장 시 평문이 아니라 해시 기반으로 저장된다.  
2: 설정 항목을 제공한다: 세션 루트 폴더, 세션 폴더명 템플릿, Export 출력 경로/품질/리사이즈, 파일명 템플릿, UI Lock-down 레벨(0~2), EDSDK 경로.  
3: 설정은 Tauri `app_data_dir`에 영속 저장되며 재실행 후에도 유지된다.  
4: 필수 설정(경로/권한 등)이 유효하지 않으면 Customer Mode는 “진입 불가 + 도움 요청”으로 안전하게 차단된다.  
5: 설정 초기화/기본값 복원이 가능하다(운영 롤백).  

#### Integration Verification
IV1: 업스트림의 기존 설정/프리셋 저장 로직과 충돌하지 않는다.  
IV2: 설정 I/O가 macOS/Windows에서 일관되게 동작한다(경로/권한 포함).  
IV3: 설정 읽기/검증이 UI를 블로킹하지 않는다.

### Story 1.3 Reservation Check & Session Folder Creation

As a studio customer,  
I want to start a session by confirming my reservation without naming folders,  
so that I can begin shooting quickly with zero file management.

#### Acceptance Criteria
1: Customer Mode `Idle`에서 QR 스캔(가능 시) 또는 휴대폰번호 뒤4자리 입력을 지원한다.  
2: 예약 정보는 최소 로컬 저장소(JSON/SQLite)로 조회하며, 실패 시에도 고객이 사용할 수 있도록 필수 값(휴대폰번호 뒤4자리 등)을 자동 생성/설정한다.  
3: 세션 시작 시 설정된 규칙으로 세션 폴더를 자동 생성한다(예: `YYYYMMDD_HH00_휴대폰뒤4자리_이름(선택)`).  
4: 세션 메타데이터(휴대폰번호 뒤4자리, 세션 폴더, 시작 시각 등)를 저장한다.  
5: 세션 생성 실패 시 부분 생성/오염 없이 `Idle`로 복귀할 수 있다(롤백/재시도).  

#### Integration Verification
IV1: Admin Mode에서 기존 폴더 탐색/라이브러리 기능이 유지된다(회귀 방지).  
IV2: 활성 세션 폴더가 Frontend/Backend 모두에서 “단일 소스”로 일치한다.  
IV3: 폴더 생성/예약 조회가 체감 지연 없이 동작한다.

### Story 1.4 Session Navigation Restriction (Customer Mode)

As a studio customer,  
I want the app to only show photos from my current session,  
so that I cannot accidentally view other customers’ photos.

#### Acceptance Criteria
1: Customer Mode에서는 세션 폴더 외의 폴더/파일을 UI로 접근할 수 없다(폴더 트리/최근 경로/파일 선택기 차단).  
2: 세션 내 이미지 목록은 최근 촬영 순으로 최소 UI(필름스트립/그리드)를 제공한다.  
3: Customer Mode에서 “저장 경로”, “탐색기 열기”, “고급 라이브러리 액션(태그/삭제/이동 등)”은 숨김 또는 비활성화한다.  
4: Admin Mode에서는 전체 기능을 유지하거나(운영 필요 시) 별도 정책에 따라 제한할 수 있다.  
5: 제한 정책은 설정(또는 모드) 변경으로 완화/롤백 가능하다.  

#### Integration Verification
IV1: Admin Mode의 기존 라이브러리 탐색이 정상 동작한다.  
IV2: Backend 커맨드가 세션 폴더 경계를 우회하지 않도록 강제(서버 사이드 검증)한다.  
IV3: 세션 전환/리스트 로딩이 성능 병목을 만들지 않는다.

### Story 1.5 Capture Screen Timer & Forced Transition

As a studio customer,  
I want a big countdown timer with a forced end-of-session flow,  
so that I can focus on shooting and the kiosk reliably ends on time.

#### Acceptance Criteria
1: `Capture` 화면은 큰 타이머와 최소 액션(촬영/끝내기)만 제공한다.  
2: 세션 시작과 동시에 타이머가 동작하며, 10/5/1분 전 경고(화면+사운드)를 제공한다.  
3: 타이머 만료 시 촬영을 차단하고 **ExportLock 화면으로 강제 전환**한다(우회 불가).  
4: 고객이 “끝내기”를 눌러도 ExportLock으로 강제 전환된다.  
5: 운영자는 타이머/경고 정책을 설정으로 조정하거나 비활성화할 수 있다(롤백).  

#### Integration Verification
IV1: Admin Mode의 기존 UI/단축키/패널 동작이 회귀하지 않는다.  
IV2: 상태머신 전이가 Export/큐 상태와 모순되지 않도록 일관된 상태 모델을 사용한다.  
IV3: 타이머/경고가 렌더링/입력 지연을 유발하지 않는다.

### Story 1.6 Export Rules & Filename Templating

As a studio operator,  
I want to define export rules and a phone-last4-based filename template,  
so that delivered files are consistently named and placed without manual renaming.

#### Acceptance Criteria
1: Admin에서 Export 규칙(출력 경로, JPEG 품질, 리사이즈, 파일명 템플릿)을 설정할 수 있다.  
2: 기본 파일명 템플릿 `휴대폰뒤4자리-{hh}시-{sequence}` 를 지원하고, `{hh}`/`{sequence}`는 충돌 없이 동작한다.  
3: Export는 세션 단위(세션 폴더 전체)로 실행 가능하며, 진행률을 표시한다.  
4: Customer Mode에서는 수동 Export 옵션을 숨기고, 세션 종료 시 자동 Export로 연결된다.  
5: 신규 규칙을 비활성화하면 기존 RapidRAW Export 동작으로 롤백 가능하다.  

#### Integration Verification
IV1: 기존 RapidRAW Export(일반 이미지/폴더) 기능이 유지된다.  
IV2: 템플릿/시퀀싱이 기존 파일명 생성 로직과 호환되며 테스트 가능하다.  
IV3: Export 작업이 UI 프리즈 없이 진행된다(진행 이벤트/취소 경로 포함).

### Story 1.7 Smart Export Pipeline (Background Queue)

As a studio customer,  
I want my photos to be processed in the background during shooting,  
so that waiting time at the end of the session is minimized.

#### Acceptance Criteria
1: 새 이미지가 세션 폴더에 추가되면 백그라운드 큐에 JPEG 생성 작업이 enqueue된다.  
2: Customer Mode에는 “처리 중(남은 n장)” 수준의 큐 상태를 표시한다.  
3: Admin Mode에는 실패 항목/재시도/세부 상태를 표시한다.  
4: 큐가 프리뷰/라이브뷰를 방해하지 않도록 우선순위/스로틀링 정책을 적용한다.  
5: 세션 종료(ExportLock) 시 큐를 drain하여 남은 항목을 처리하고 완료 후 다음 단계로 진행한다.  
6: Smart Export를 끄면 “종료 시 일괄 Export”로 롤백 가능하다.  

#### Integration Verification
IV1: 기존 썸네일/프리뷰 생성 경로가 회귀하지 않는다.  
IV2: 큐 진행 이벤트/상태가 ExportLock UI와 일관되게 연결된다.  
IV3: 백그라운드 처리로 인해 목표(최근 촬영본 표시 2초) 성능이 악화되지 않는다.

### Story 1.8 EDSDK Path Validation & Camera Connect/Health

As a studio operator,  
I want the kiosk to validate EDSDK and maintain stable camera connectivity,  
so that the session cannot start in a broken hardware/software state.

#### Acceptance Criteria
1: Admin에서 EDSDK 경로를 설정하고, 필수 구성요소(DLL/런타임) 존재 여부를 검증한다(Windows).  
2: EOS 700D 연결/해제/재연결을 지원하고, 상태를 UI에 표시한다(Customer는 단순, Admin은 상세).  
3: 케이블 분리/절전/오류 시 안전 상태로 전환하고 재시도 경로를 제공한다.  
4: Customer Mode는 카메라가 준비되지 않으면 Capture로 진행할 수 없다(자동 점검 + 차단).  
5: 테더 기능을 비활성화하면 파일 기반(수동) 모드로 롤백 가능하다(운영 플랜 B).  

#### Integration Verification
IV1: Windows 외 환경에서 빌드/개발이 막히지 않도록 스텁/피처 플래그 전략을 갖는다.  
IV2: EDSDK 코드는 별도 모듈로 격리되어 기존 Rust/Tauri 커맨드 표면을 과도하게 오염시키지 않는다.  
IV3: 연결/헬스체크가 UI/백그라운드 작업과 데드락/프리즈를 만들지 않는다.

### Story 1.9 Capture Trigger, Ingestion, and Auto-Apply Capture Preset

As a studio customer,  
I want to press one big “Shoot” button and immediately see the photo with my selected filter applied,  
so that I get instant feedback without manual importing or editing.

#### Acceptance Criteria
1: Customer Mode의 촬영 버튼이 EDSDK 셔터 트리거를 호출한다(우선 경로).  
2: 촬영 파일은 `PC(Host)`의 활성 세션 폴더로 저장되며, 수신 완료 후 UI에 자동 로드되고 최근 촬영본이 자동 선택된다.  
3: sidecar가 없으면 “현재 캡처 프리셋”으로 초기값을 생성하고 sidecar를 생성한다.  
4: 프리셋 변경 이후 촬영분부터 새 프리셋이 적용되며, 과거 촬영분은 자동 변경되지 않는다.  
5: 이벤트 중복/누락/부분 전송 등 엣지 케이스에서 중복 처리 없이 안정적으로 동작한다(아이템포턴시).  

#### Integration Verification
IV1: 기존 sidecar 기반 편집/복원 동작이 유지된다.  
IV2: 파일 수신 → 프리뷰/썸네일 → Smart Export 큐 enqueue 흐름이 일관되게 연결된다.  
IV3: “촬영 후 최근 촬영본 표시 2초” 목표에 부합한다.

### Story 1.10 Live View in Capture (Customer) + Optional Admin Pop-out

As a studio customer,  
I want live view embedded in the capture screen,  
so that I can frame my shot without touching the camera.

#### Acceptance Criteria
1: Customer Mode `Capture` 화면에 라이브뷰를 임베드로 제공한다(가능 범위).  
2: Admin Mode에서는 라이브뷰를 팝업으로 열 수 있다(옵션).  
3: 라이브뷰는 연결 끊김/일시 중단 후 자동 재개 또는 명확한 상태 안내를 제공한다.  
4: 라이브뷰를 비활성화할 수 있어야 하며, 비활성화 시에도 촬영은 가능하다(롤백).  

#### Integration Verification
IV1: 라이브뷰가 기존 GPU 프리뷰/렌더링 경로와 충돌하지 않는다.  
IV2: Export/Smart Export 등 고부하 시 라이브뷰 리소스 정책이 명확하다(중단/우선순위).  
IV3: 라이브뷰가 UI 응답성/프레임레이트를 심각하게 저하시키지 않는다.

### Story 1.11 ExportLock, Completion, and Privacy Reset (No File Deletion)

As a studio operator,  
I want the kiosk to export, deliver, and reset automatically without leaking prior session data,  
so that unattended operation is safe and repeatable.

#### Acceptance Criteria
1: ExportLock은 세션 종료 시 강제 진입하며, 고객 동작은 “진행률 보기 + 도움 요청”으로 제한된다.  
2: ExportLock에서 Smart Export 큐를 drain하고, Export 완료 후 `Complete/Reset`으로 자동 전환한다.  
3: 완료 화면은 수령 안내(QR/이메일/로컬 출력 폴더 등 옵션)를 제공하고, 카운트다운 후 Reset 된다.  
4: Reset은 (a) UI 상태 초기화 (b) 캐시/프리뷰/썸네일 정리 (c) 백그라운드 작업 취소/정리를 수행한다.  
5: Reset은 세션 폴더의 이미지/sidecar 파일을 삭제하지 않는다(확정 요구사항).  
6: 리셋/Export 실패 시에도 “안전 상태 + 도움 요청(고객) / 재시도/진단(Admin)” 경로가 존재한다.  

#### Integration Verification
IV1: 기존 캐시 정리/Export 취소/백그라운드 작업 모델과 충돌하지 않는다.  
IV2: ExportLock 상태/이벤트가 Frontend↔Backend 간 불일치 없이 종료까지 일관된다.  
IV3: Reset은 빠르게 완료되며 다음 세션 시작에 영향을 주지 않는다.

### Story 1.12 Operational Guardrails: Lock-down Level, Retention, Diagnostics

As a studio operator,  
I want operational guardrails (lock-down, retention, diagnostics) configurable in Admin Mode,  
so that the kiosk is stable over long-term unattended use.

#### Acceptance Criteria
1: UI Lock-down 레벨(0~2)이 Customer Mode에 적용된다(전체화면/창 닫기 제한/단축키 제한 등 “가능 범위” 내).  
2: Admin에서 세션 보관/삭제 정책(예: 30일 보관, 디스크 임계치 시 오래된 세션부터 정리)을 설정할 수 있다.  
3: 보관/정리 작업은 “활성 세션”을 절대 건드리지 않으며, 실행/결과를 로그로 남긴다.  
4: Admin에서 로그/진단 리포트(카메라 상태, 디스크, 큐 상태, 최근 오류)를 조회/내보내기 할 수 있다.  
5: Lock-down/보관 정책은 비활성화/롤백 가능하며 핵심 플로우를 방해하지 않는다.  

#### Integration Verification
IV1: Admin Mode의 창/키 입력 경험이 불필요하게 제한되지 않는다.  
IV2: 정리/진단 작업이 성능/응답성을 눈에 띄게 저하시키지 않는다.  
IV3: 운영 정책이 실제 Windows 환경에서 재현 가능하게 문서화된다(설치/EDSDK/장애 대응 포함).

This story sequence is designed to minimize risk to your existing system. Does this order make sense given your project's architecture and constraints?
Confirmed by user: 2026-01-02
