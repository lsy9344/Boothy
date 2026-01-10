# Epic 1: RapidTetherRAW Customer Mode MVP

**Epic Goal**: RapidRAW 기반(React+TS+Tauri+Rust)의 강점을 유지하면서, 무인 셀프 스튜디오 운영을 위한 Customer Mode 상태머신(Idle→Setup→Capture→ExportLock→Complete→Reset)과 Admin Mode(PIN), EOS 700D EDSDK 테더링, 캡처 프리셋 자동 적용, Smart Export Pipeline, ExportLock 게이트, 프라이버시 리셋/운영 진단을 통합해 “촬영→수령→초기화”의 안정적인 end-to-end 경험을 제공한다.

**Integration Requirements**:
- (라이선스/배포) RapidRAW 기반 **AGPL-3.0** 준수, Canon **EDSDK 비재배포/비동봉**(사용자 로컬 설치 + 경로 설정).
- (플랫폼) 운영 환경은 **Windows 10/11 x64**. 개발은 macOS에서 진행 가능하나 EDSDK는 Windows에서만 실검증 필요.
- (데이터) 비파괴 워크플로우 유지: RAW 옆 sidecar(`*.rrdata` 또는 호환) 저장 및 복원.
- (파일/네비게이션) Customer Mode는 **세션 폴더 격리** 및 파일 탐색/저장 경로 노출 금지.
- (커맨드 계약) Frontend↔Backend는 기존 `invoke`/event 패턴을 유지하되, 신규 커맨드/이벤트는 명시적으로 추가·문서화(기존 이름 변경 최소화).
- (Export) 파일명 템플릿 `휴대폰뒤4자리-{hh}시-{sequence}` 지원(휴대폰번호 뒤4자리 런타임 주입) 및 ExportLock 상태/진행률 계약 일관성 유지.
- (운영) 무인 장애 대응을 위한 Admin 진단/로그 내보내기, 리셋의 안정적 작업 취소/정리(세션 파일 삭제 금지).

## Story 1.1 Mode Shell & Guided State Machine Scaffold

As a studio operator,  
I want the app to launch into a guided Customer Mode with a clear Admin escape hatch,  
so that customers cannot access advanced controls or the filesystem.

### Acceptance Criteria
1: 앱은 기본으로 Customer Mode `Idle` 화면으로 부팅한다.  
2: Admin PIN 진입 동선이 존재하며, 성공 시 Admin Mode로 전환된다(최소 UI/라우팅 완성).  
3: Customer Mode에서는 파일 탐색/고급 패널/수동 Export 옵션이 노출되지 않는다(화이트리스트 방식).  
4: Admin Mode에서는 기존 RapidRAW의 “이미지 열기/보정/Export” 핵심 플로우가 유지된다(회귀 방지).  
5: Customer Mode 기능은 설정 플래그로 비활성화 가능해야 한다(롤백/운영 비상용).  

### Integration Verification
IV1: Admin Mode에서 기존 RapidRAW 편집/Export가 정상 동작한다.  
IV2: 모드 라우팅 추가가 기존 `invoke`/event 리스너를 깨지 않는다.  
IV3: 앱 시작/화면 전환 성능이 기존 대비 유의미하게 악화되지 않는다.

## Story 1.2 Admin PIN & Configuration Storage

As a studio operator,  
I want to configure kiosk settings (PIN, session root, export rules, EDSDK path) in one place,  
so that the kiosk can be operated consistently without ad-hoc manual steps.

### Acceptance Criteria
1: Admin PIN 설정/변경이 가능하며, 저장 시 평문이 아니라 해시 기반으로 저장된다.  
2: 설정 항목을 제공한다: 세션 루트 폴더, 세션 폴더명 템플릿, Export 출력 경로/품질/리사이즈, 파일명 템플릿, UI Lock-down 레벨(0~2), EDSDK 경로.  
3: 설정은 Tauri `app_data_dir`에 영속 저장되며 재실행 후에도 유지된다.  
4: 필수 설정(경로/권한 등)이 유효하지 않으면 Customer Mode는 “진입 불가 + 도움 요청”으로 안전하게 차단된다.  
5: 설정 초기화/기본값 복원이 가능하다(운영 롤백).  

### Integration Verification
IV1: 업스트림의 기존 설정/프리셋 저장 로직과 충돌하지 않는다.  
IV2: 설정 I/O가 macOS/Windows에서 일관되게 동작한다(경로/권한 포함).  
IV3: 설정 읽기/검증이 UI를 블로킹하지 않는다.

## Story 1.3 Reservation Check & Session Folder Creation

As a studio customer,  
I want to start a session by confirming my reservation without naming folders,  
so that I can begin shooting quickly with zero file management.

### Acceptance Criteria
1: Customer Mode `Idle`에서 QR 스캔(가능 시) 또는 휴대폰번호 뒤4자리 입력을 지원한다.  
2: 예약 정보는 최소 로컬 저장소(JSON/SQLite)로 조회하며, 실패 시에도 고객이 사용할 수 있도록 필수 값(휴대폰번호 뒤4자리 등)을 자동 생성/설정한다.  
3: 세션 시작 시 설정된 규칙으로 세션 폴더를 자동 생성한다(예: `YYYYMMDD_HH00_휴대폰뒤4자리_이름(선택)`).  
4: 세션 메타데이터(휴대폰번호 뒤4자리, 세션 폴더, 시작 시각 등)를 저장한다.  
5: 세션 생성 실패 시 부분 생성/오염 없이 `Idle`로 복귀할 수 있다(롤백/재시도).  

### Integration Verification
IV1: Admin Mode에서 기존 폴더 탐색/라이브러리 기능이 유지된다(회귀 방지).  
IV2: 활성 세션 폴더가 Frontend/Backend 모두에서 “단일 소스”로 일치한다.  
IV3: 폴더 생성/예약 조회가 체감 지연 없이 동작한다.

## Story 1.4 Session Navigation Restriction (Customer Mode)

As a studio customer,  
I want the app to only show photos from my current session,  
so that I cannot accidentally view other customers’ photos.

### Acceptance Criteria
1: Customer Mode에서는 세션 폴더 외의 폴더/파일을 UI로 접근할 수 없다(폴더 트리/최근 경로/파일 선택기 차단).  
2: 세션 내 이미지 목록은 최근 촬영 순으로 최소 UI(필름스트립/그리드)를 제공한다.  
3: Customer Mode에서 “저장 경로”, “탐색기 열기”, “고급 라이브러리 액션(태그/삭제/이동 등)”은 숨김 또는 비활성화한다.  
4: Admin Mode에서는 전체 기능을 유지하거나(운영 필요 시) 별도 정책에 따라 제한할 수 있다.  
5: 제한 정책은 설정(또는 모드) 변경으로 완화/롤백 가능하다.  

### Integration Verification
IV1: Admin Mode의 기존 라이브러리 탐색이 정상 동작한다.  
IV2: Backend 커맨드가 세션 폴더 경계를 우회하지 않도록 강제(서버 사이드 검증)한다.  
IV3: 세션 전환/리스트 로딩이 성능 병목을 만들지 않는다.

## Story 1.5 Capture Screen Timer & Forced Transition

As a studio customer,  
I want a big countdown timer with a forced end-of-session flow,  
so that I can focus on shooting and the kiosk reliably ends on time.

### Acceptance Criteria
1: `Capture` 화면은 큰 타이머와 최소 액션(촬영/끝내기)만 제공한다.  
2: 세션 시작과 동시에 타이머가 동작하며, 10/5/1분 전 경고(화면+사운드)를 제공한다.  
3: 타이머 만료 시 촬영을 차단하고 **ExportLock 화면으로 강제 전환**한다(우회 불가).  
4: 고객이 “끝내기”를 눌러도 ExportLock으로 강제 전환된다.  
5: 운영자는 타이머/경고 정책을 설정으로 조정하거나 비활성화할 수 있다(롤백).  

### Integration Verification
IV1: Admin Mode의 기존 UI/단축키/패널 동작이 회귀하지 않는다.  
IV2: 상태머신 전이가 Export/큐 상태와 모순되지 않도록 일관된 상태 모델을 사용한다.  
IV3: 타이머/경고가 렌더링/입력 지연을 유발하지 않는다.

## Story 1.6 Export Rules & Filename Templating

As a studio operator,  
I want to define export rules and a phone-last4-based filename template,  
so that delivered files are consistently named and placed without manual renaming.

### Acceptance Criteria
1: Admin에서 Export 규칙(출력 경로, JPEG 품질, 리사이즈, 파일명 템플릿)을 설정할 수 있다.  
2: 기본 파일명 템플릿 `휴대폰뒤4자리-{hh}시-{sequence}` 를 지원하고, `{hh}`/`{sequence}`는 충돌 없이 동작한다.  
3: Export는 세션 단위(세션 폴더 전체)로 실행 가능하며, 진행률을 표시한다.  
4: Customer Mode에서는 수동 Export 옵션을 숨기고, 세션 종료 시 자동 Export로 연결된다.  
5: 신규 규칙을 비활성화하면 기존 RapidRAW Export 동작으로 롤백 가능하다.  

### Integration Verification
IV1: 기존 RapidRAW Export(일반 이미지/폴더) 기능이 유지된다.  
IV2: 템플릿/시퀀싱이 기존 파일명 생성 로직과 호환되며 테스트 가능하다.  
IV3: Export 작업이 UI 프리즈 없이 진행된다(진행 이벤트/취소 경로 포함).

## Story 1.7 Smart Export Pipeline (Background Queue)

As a studio customer,  
I want my photos to be processed in the background during shooting,  
so that waiting time at the end of the session is minimized.

### Acceptance Criteria
1: 새 이미지가 세션 폴더에 추가되면 백그라운드 큐에 JPEG 생성 작업이 enqueue된다.  
2: Customer Mode에는 “처리 중(남은 n장)” 수준의 큐 상태를 표시한다.  
3: Admin Mode에는 실패 항목/재시도/세부 상태를 표시한다.  
4: 큐가 프리뷰/라이브뷰를 방해하지 않도록 우선순위/스로틀링 정책을 적용한다.  
5: 세션 종료(ExportLock) 시 큐를 drain하여 남은 항목을 처리하고 완료 후 다음 단계로 진행한다.  
6: Smart Export를 끄면 “종료 시 일괄 Export”로 롤백 가능하다.  

### Integration Verification
IV1: 기존 썸네일/프리뷰 생성 경로가 회귀하지 않는다.  
IV2: 큐 진행 이벤트/상태가 ExportLock UI와 일관되게 연결된다.  
IV3: 백그라운드 처리로 인해 목표(최근 촬영본 표시 2초) 성능이 악화되지 않는다.

## Story 1.8 EDSDK Path Validation & Camera Connect/Health

As a studio operator,  
I want the kiosk to validate EDSDK and maintain stable camera connectivity,  
so that the session cannot start in a broken hardware/software state.

### Acceptance Criteria
1: Admin에서 EDSDK 경로를 설정하고, 필수 구성요소(DLL/런타임) 존재 여부를 검증한다(Windows).  
2: EOS 700D 테더링을 위한 **카메라 상태 모델/진단 UI/게이팅 스캐폴딩**을 제공하고, 상태를 UI에 표시한다(Customer는 단순, Admin은 상세). (실제 연결/해제/재연결/헬스 모니터링은 Story 1.9에서 구현)  
3: 카메라 상태가 not-ready/error일 때 안전 상태로 전환하고 재시도/도움 요청 경로를 제공한다. (실제 케이블 분리/절전 감지 및 자동 재연결은 Story 1.9에서 구현)  
4: Customer Mode는 카메라가 준비되지 않으면 Capture로 진행할 수 없다(자동 점검 + 차단).  
5: 테더 기능을 비활성화하면 파일 기반(수동) 모드로 롤백 가능하다(운영 플랜 B).  

### Integration Verification
IV1: Windows 외 환경에서 빌드/개발이 막히지 않도록 스텁/피처 플래그 전략을 갖는다.  
IV2: EDSDK 코드는 별도 모듈로 격리되어 기존 Rust/Tauri 커맨드 표면을 과도하게 오염시키지 않는다.  
IV3: 상태 조회/진단/게이팅 로직이 UI/백그라운드 작업과 데드락/프리즈를 만들지 않는다. (실제 연결/헬스 모니터링 루프는 Story 1.9)

### Implementation Notes
- SDK_for_developers/Windows/EDSDK/Header/ 내의 헤더 파일들을 참조하여 카메라 관련 sdk 기능구현.

## Story 1.9 Capture Trigger, Ingestion, and Auto-Apply Capture Preset

As a studio customer,  
I want to press one big “Shoot” button and immediately see the photo with my selected filter applied,  
so that I get instant feedback without manual importing or editing.

### Acceptance Criteria
1: Customer Mode의 촬영 버튼이 EDSDK 셔터 트리거를 호출한다(우선 경로).  
2: 촬영 파일은 `PC(Host)`의 활성 세션 폴더로 저장되며, 수신 완료 후 UI에 자동 로드되고 최근 촬영본이 자동 선택된다.  
3: sidecar가 없으면 “현재 캡처 프리셋”으로 초기값을 생성하고 sidecar를 생성한다.  
4: 프리셋 변경 이후 촬영분부터 새 프리셋이 적용되며, 과거 촬영분은 자동 변경되지 않는다.  
5: 이벤트 중복/누락/부분 전송 등 엣지 케이스에서 중복 처리 없이 안정적으로 동작한다(아이템포턴시).  
6: EDSDK 기반 카메라 세션(연결/해제/재연결) + 기본 헬스 모니터링(백그라운드) 및 **이벤트 기반 상태 업데이트**를 구현한다. (Story 1.8에서 만든 상태 모델/진단 UI/게이팅 스캐폴딩을 “실제 상태”로 구동)  
   - Admin Diagnostics의 Connect/Disconnect/Retry 액션을 실제 동작으로 연결한다.  
   - Customer/Admin UI는 상태 변경을 자동 반영한다(수동 Refresh 의존 최소화).  

### Story Split (Implementation Plan)

Note: Story 1.9의 "연결/해제" 감지는 EDSDK가 아니라 Windows `WM_DEVICECHANGE`(물리 연결) 기반으로 구현하며, `get_camera_status`는 EDSDK를 호출해 "연결 여부"를 판단하지 않는다. (상세: Story 1.9.4)
스토리 1.9는 기술/통합 작업량이 커서 아래 3개 스토리로 세분화해 진행한다. (AC는 합산 기준이며, 각 스토리에서 부분 집합을 충족한다)

- **Story 1.9.1**: Tethering controller + camera session lifecycle + status events/health monitoring (주로 AC6, NFR4)  
  (Story file: `docs/stories/1.9.1.tethering-controller-session-lifecycle-status-events.md`)
- **Story 1.9.4**: Windows `WM_DEVICECHANGE` 기반 물리 연결 감지 + 상태 전이/이벤트 구동 (EDSDK로 "연결 여부"를 체크하지 않음)
  (Story file: `docs/stories/1.9.4.wm-devicechange-physical-connection-detection.md`)
- **Story 1.9.2**: Customer “Shoot” → shutter trigger + ingestion/download to active session folder + idempotency + `image-added` event (주로 AC1, AC2, AC5, NFR1, NFR5)  
  (Story file: `docs/stories/1.9.2.shutter-trigger-and-ingestion-to-session-folder.md`)
- **Story 1.9.3**: Capture preset selection + sidecar auto-create for newly captured images only + instant feedback semantics (주로 AC3, AC4, IV1)  
  (Story file: `docs/stories/1.9.3.auto-apply-capture-preset-sidecar-auto-create.md`)

### Integration Verification
IV1: 기존 sidecar 기반 편집/복원 동작이 유지된다.  
IV2: 파일 수신 → 프리뷰/썸네일 → Smart Export 큐 enqueue 흐름이 일관되게 연결된다.  
IV3: “촬영 후 최근 촬영본 표시 2초” 목표에 부합한다.

## Story 1.10 Live View in Capture (Customer) + Optional Admin Pop-out

As a studio customer,  
I want live view embedded in the capture screen,  
so that I can frame my shot without touching the camera.

### Acceptance Criteria
1: Customer Mode `Capture` 화면에 라이브뷰를 임베드로 제공한다(가능 범위).  
2: Admin Mode에서는 라이브뷰를 팝업으로 열 수 있다(옵션).  
3: 라이브뷰는 연결 끊김/일시 중단 후 자동 재개 또는 명확한 상태 안내를 제공한다.  
4: 라이브뷰를 비활성화할 수 있어야 하며, 비활성화 시에도 촬영은 가능하다(롤백).  

### Integration Verification
IV1: 라이브뷰가 기존 GPU 프리뷰/렌더링 경로와 충돌하지 않는다.  
IV2: Export/Smart Export 등 고부하 시 라이브뷰 리소스 정책이 명확하다(중단/우선순위).  
IV3: 라이브뷰가 UI 응답성/프레임레이트를 심각하게 저하시키지 않는다.

## Story 1.11 ExportLock, Completion, and Privacy Reset (No File Deletion)

As a studio operator,  
I want the kiosk to export, deliver, and reset automatically without leaking prior session data,  
so that unattended operation is safe and repeatable.

### Acceptance Criteria
1: ExportLock은 세션 종료 시 강제 진입하며, 고객 동작은 “진행률 보기 + 도움 요청”으로 제한된다.  
2: ExportLock에서 Smart Export 큐를 drain하고, Export 완료 후 `Complete/Reset`으로 자동 전환한다.  
3: 완료 화면은 수령 안내(QR/이메일/로컬 출력 폴더 등 옵션)를 제공하고, 카운트다운 후 Reset 된다.  
4: Reset은 (a) UI 상태 초기화 (b) 캐시/프리뷰/썸네일 정리 (c) 백그라운드 작업 취소/정리를 수행한다.  
5: Reset은 세션 폴더의 이미지/sidecar 파일을 삭제하지 않는다(확정 요구사항).  
6: 리셋/Export 실패 시에도 “안전 상태 + 도움 요청(고객) / 재시도/진단(Admin)” 경로가 존재한다.  

### Integration Verification
IV1: 기존 캐시 정리/Export 취소/백그라운드 작업 모델과 충돌하지 않는다.  
IV2: ExportLock 상태/이벤트가 Frontend↔Backend 간 불일치 없이 종료까지 일관된다.  
IV3: Reset은 빠르게 완료되며 다음 세션 시작에 영향을 주지 않는다.

## Story 1.12 Operational Guardrails: Lock-down Level, Retention, Diagnostics

As a studio operator,  
I want operational guardrails (lock-down, retention, diagnostics) configurable in Admin Mode,  
so that the kiosk is stable over long-term unattended use.

### Acceptance Criteria
1: UI Lock-down 레벨(0~2)이 Customer Mode에 적용된다(전체화면/창 닫기 제한/단축키 제한 등 “가능 범위” 내).  
2: Admin에서 세션 보관/삭제 정책(예: 30일 보관, 디스크 임계치 시 오래된 세션부터 정리)을 설정할 수 있다.  
3: 보관/정리 작업은 “활성 세션”을 절대 건드리지 않으며, 실행/결과를 로그로 남긴다.  
4: Admin에서 로그/진단 리포트(카메라 상태, 디스크, 큐 상태, 최근 오류)를 조회/내보내기 할 수 있다.  
5: Lock-down/보관 정책은 비활성화/롤백 가능하며 핵심 플로우를 방해하지 않는다.  

### Integration Verification
IV1: Admin Mode의 창/키 입력 경험이 불필요하게 제한되지 않는다.  
IV2: 정리/진단 작업이 성능/응답성을 눈에 띄게 저하시키지 않는다.  
IV3: 운영 정책이 실제 Windows 환경에서 재현 가능하게 문서화된다(설치/EDSDK/장애 대응 포함).

This story sequence is designed to minimize risk to your existing system. Does this order make sense given your project's architecture and constraints?
Confirmed by user: 2026-01-02
