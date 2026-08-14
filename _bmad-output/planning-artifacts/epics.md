---
stepsCompleted:
  - 'step-01-validate-prerequisites'
  - 'step-02-design-epics'
  - 'step-03-create-stories'
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/architecture.md'
  - '_bmad-output/planning-artifacts/ux-design-specification.md'
project_name: 'Boothy'
date: '2026-03-20'
lastEdited: '2026-08-11'
correctCourseStatus: 'approved-readiness-12-issue-remediation'
---

# Boothy - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Boothy, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: 고객은 이름과 휴대전화 뒤 4자리만 입력해서 현재 부스 세션을 시작할 수 있어야 하며, 전체 전화번호 입력이나 예약 검증 없이 준비 또는 촬영 가능 상태로 진입할 수 있어야 한다.
FR2: 고객은 1~6개의 승인된 게시 프리셋만 볼 수 있어야 하며, 각 프리셋의 이름과 대표 미리보기 타일 또는 샘플컷을 바탕으로 하나의 활성 프리셋을 선택할 수 있어야 한다.
FR3: 고객은 부스가 `Preparing`, `Ready`, `Preview Waiting`, `Export Waiting`, `Phone Required` 중 어떤 상태인지 평이한 언어로 이해할 수 있어야 하며, 허용된 상태에서만 촬영할 수 있어야 한다.
FR4: 고객은 현재 세션에 사진을 촬영해 안전하게 저장할 수 있어야 하며, 프리뷰 준비가 아직 끝나지 않았더라도 저장 성공과 프리뷰 준비 상태를 구분해서 안내받아야 한다.
FR5: 고객은 현재 세션의 사진만 검토할 수 있어야 하고, `Current-Session Deletion Policy`가 허용하는 범위에서만 삭제할 수 있어야 하며, 프리셋은 세션 중 언제든 변경할 수 있어야 한다. 변경은 이후 촬영부터 반영되고 이미 저장된 촬영본은 유지되어야 한다.
FR6: 고객은 세션 시작 시점부터 조정된 종료 시각을 확인할 수 있어야 하며, 5분 전 경고와 종료 시각 알림을 통해 남은 촬영 가능 여부와 종료 후 행동을 명확히 안내받아야 한다.
FR7: 고객은 촬영 종료 후 `Export Waiting`, `Completed`, `Phone Required` 중 하나의 명시적 상태로 진입해야 하며, 완료 또는 인계 준비 상태를 기술 진단어 없이 이해할 수 있어야 한다.
FR8: 승인된 내부 프리셋 관리자는 드래프트 프리셋을 작성, 검증, 승인, 게시, 롤백할 수 있어야 하며, 게시된 불변 프리셋 아티팩트만 미래 세션 카탈로그에 반영되어야 한다.
FR9: 운영자는 현재 세션 문맥, 실패 상태, 허용된 복구 액션, 라이프사이클 로그를 확인할 수 있어야 하며, `Operator Recovery Policy` 범위 안에서만 복구를 수행할 수 있어야 한다.
FR10: 고객은 촬영 전에 준비된 전용 관람 화면에서 같은 촬영과 capture-bound 프리셋의 화면 적합 결과를 확인하고, 이후 RAW 정밀본으로 blank·stale frame·crop/scale jump·quality-tier downgrade 없이 전환되는 경험을 제공받아야 한다.

### NonFunctional Requirements

NFR1: 고객용 주요 상태 화면은 동적 세션 값을 제외하고 기본 지시 문장 1개, 보조 문장 1개, 주요 액션 라벨 1개 이내의 문구 밀도를 유지해야 하며, 기술 진단어·darktable 용어·저작 도구 용어를 노출하면 안 된다.
NFR2: 모든 활성 지점은 승인된 프리셋 카탈로그, 게시 프리셋 버전, 고객용 타이밍 규칙, 핵심 부스 여정을 동일하게 유지해야 하며, 차이는 승인된 지역 설정으로만 제한되어야 한다.
NFR3: 주요 고객 액션은 1초 이내에 응답이 인지되어야 하며, trusted capture input부터 사전 준비된 관람 화면의 첫 qualifying preset-applied monitor frame까지 warm p50 3초 이하, p95 4초 이하, hard max 5초 이하를 충족해야 한다. 100-shot 성공률은 99% 이상이어야 하며 잘못된 촬영·프리셋·세션, 무보정 qualifying frame, blank, stale, upscale, crop/scale jump, tier downgrade는 0건이어야 한다.
NFR4: 소스 캡처, 프리뷰, 최종 결과물, 검토, 삭제, 완료 흐름 전반에서 교차 세션 자산 누출은 0건이어야 하며, 저장되는 고객 식별 정보는 승인된 최소 범위로 제한되어야 한다.
NFR5: 5분 전 경고와 종료 시각 알림은 99% 세션에서 허용 오차 내에 동작해야 하고, 세션의 90% 이상은 종료 시각 10초 내에 명시적 사후 상태로 진입해야 하며, 렌더 재시도나 실패가 이미 저장된 유효 촬영을 훼손하면 안 된다.
NFR6: 제품은 선택된 지점 집합에 대한 단계적 배포와 단일 승인 액션 기반 롤백을 지원해야 하며, 활성 고객 세션 중 강제 업데이트는 0건이어야 하고, 승인된 프리셋 카탈로그의 렌더 호환성이 유지되어야 한다.

### Additional Requirements

- Epic 1 Story 1에는 공식 `Vite react-ts` + 수동 `Tauri CLI` 초기화 기반의 프로젝트 부트스트랩이 포함되어야 한다.
- 제품은 하나의 패키지된 Tauri 애플리케이션 안에서 고객 부스, 운영자 콘솔, 내부 프리셋 저작 화면의 3개 capability-gated surface를 제공해야 한다.
- 활성 세션의 내구적 진실은 SQLite나 UI 메모리가 아니라 세션 단위 파일시스템 루트와 `session.json` 매니페스트가 소유해야 한다.
- 고객용 부스 별칭은 이름+휴대전화 뒤4자리 조합으로 유지하되, 내구적 내부 식별자인 `sessionId`와 분리되어야 한다.
- Rust 호스트는 카메라 상태, 타이밍 상태, 사후 완료 상태를 정규화하는 단일 진실 계층이어야 하며, React는 정규화된 상태만 소비해야 한다.
- 카메라 연동은 번들된 helper/sidecar 경계 뒤에 격리되어야 하며, 버전드 메시지와 파일시스템 핸드오프로 통신해야 한다.
- darktable 기반 프리셋 아티팩트와 `darktable-cli` 렌더 워커가 프리셋 적용의 권위 경로여야 하며, 고객에게는 일반 편집기가 노출되면 안 된다.
- 프리셋은 불변 게시 번들로 저장되어야 하고, 활성 세션은 정확한 프리셋 버전을 참조해야 하며, 게시/롤백은 미래 세션에만 영향을 줘야 한다.
- 버전드 JSON/JSONL journal은 라이프사이클, 개입, 게시, 롤아웃 감사 로그와 release evidence의 MVP 진실을 소유한다. SQLite는 필요 시 journal에서 재생성 가능한 derived query index로만 허용하며 원본 audit 또는 사진·세션 자산의 진실을 소유하면 안 된다.
- 지점별 최소 설정과 런타임 플래그만 로컬 설정 저장소에 보관해야 하며, 지점 차이는 승인된 로컬 설정으로 한정되어야 한다.
- 운영자 및 프리셋 저작 기능은 관리자 비밀번호 인증과 capability check 통과 후에만 노출되어야 한다.
- Story 1.10은 관리자 비밀번호 검증, 보호 저장소, privileged session 발급·만료·로그아웃, capability enforcement, 성공·실패·권한 거부 audit의 명시적 구현 소유자다. 환경변수 기반 인증 플래그와 route hiding만으로는 이 Story를 충족할 수 없다.
- 프론트엔드와 호스트 사이 계약은 TypeScript의 `Zod 4` 검증과 Rust 재검증을 함께 사용해야 한다.
- React Router는 `/booth`, `/operator`, `/authoring`, `/settings` 같은 최상위 surface 중심으로 제한해야 한다.
- UI 컴포넌트는 직접 `invoke` 호출을 하지 않고, 타입이 지정된 adapter/service 계층을 통해서만 호스트 기능에 접근해야 한다.
- 다음 계약 산출물은 구현 전제 조건으로 동결돼야 한다: `session.json` 스키마, preset bundle 스키마, sidecar protocol 메시지, authoring publication payload 계약.
- 타이밍 정책, 경고/종료 알림, 사후 상태 전환, 강제 업데이트 금지, 단계적 배포/롤백 규칙은 호스트 소유 워크플로 규칙으로 구현되어야 한다.
- Story 1.6, 1.7, 1.8, 3.2, 4.2, 4.3은 자동 테스트 통과만으로 제품 출시 관점 `done`으로 간주하지 않는다.
- Story 1.4와 1.5의 기존 `done` 및 Go evidence는 고객 상태·문구 회귀 증거로 보존하되, 실카메라 readiness/capture/render 출시 truth는 각각 Story 1.6, 1.7, 1.8이 소유한다.
- 지정된 booth hardware validation checklist evidence가 수집되기 전까지 truth-critical integration story는 `review` 또는 동등한 pre-close 상태에 머물러야 한다.
- booth `Ready`와 `Completed`는 각각 false-ready, false-complete 방지 evidence가 확보된 뒤에만 release truth로 인정한다.
- 전용 관람 화면은 촬영 전에 현재 세션과 승인 monitor profile에 연결되고 visible·layout-ready·listener-ready 상태를 증명해야 한다.
- Story 7.1은 renderer를 교체하지 않으며 viewer readiness와 physical display-size contract만 먼저 증명한다.
- Story 7.2는 renderer를 교체하지 않은 상태에서 immutable sample double-buffer와 actual monitor-present 계측을 증명한다.
- display asset은 `cameraSource`, `displayFitPresetProxy`, `rawRefinedDisplay`, `final`의 불변 generation으로 분리하며 한 canonical JPEG의 in-place overwrite를 금지한다.
- 공식 성능 종료점은 실제 qualifying monitor frame이며 file-ready, renderer-ready, event receipt, decode, `<img onLoad>`, review rail thumbnail은 진단 span으로만 사용한다.
- LibRaw embedded JPEG와 capability-gated RAW+JPEG는 Story 7.3 실장비 비교 전까지 후보이며 어느 하나도 기본 경로로 가정하지 않는다.
- darktable은 RAW 정밀본, final, parity oracle, exact fallback으로 유지하고 resident display renderer는 Story 7.5 구현 spike와 화질·안정성 gate를 통과한 경우에만 production 경로로 승격한다.
- signed complete installer, clean offline VM, warm 100-shot, cold, 10-minute idle, reconnect, canary, rollback evidence와 Story 7.10 최종 Go 전에는 Epic 7과 MVP release를 `done`으로 처리하지 않는다.
- UX-DR16은 `UX-EV-01` 접근성 release evidence로, 실제 터치 반응성·standing usability·고대비·무가이드 촬영 성공률은 `UX-EV-02` 실부스 사용성 evidence로 추적한다. QA/Release가 evidence package를 소유하고 Story 7.10/HV-18D가 최종 집계한다.

### UX Design Requirements

UX-DR1: 고객 기본 흐름은 booth-first, preset-driven 구조를 유지해야 하며, 고객에게 세부 조정 화면, darktable 용어, 내부 제작 도구를 노출하면 안 된다.
UX-DR2: 세션 시작 화면은 이름과 휴대전화 뒤4자리 두 입력만 요구해야 하며, 잘못된 형식이나 빈 값은 즉시 검증해 다음 진행 전에 분명히 안내해야 한다.
UX-DR3: 고객 화면은 현재 활성 프리셋, 최신 촬영 결과, 현재 세션 범위의 사진만 이해할 수 있도록 상태 정보를 항상 인지 가능하게 보여줘야 한다.
UX-DR4: 조정된 종료 시각은 세션 시작부터 명확히 보여야 하며, 5분 전 경고와 종료 후 다음 행동은 plain-language 고객 안전 문구로 전달되어야 한다.
UX-DR5: 고객용 문구는 낮은 문구 밀도 원칙을 따라야 하며, 기술 진단어, 내부 운영 용어, 원인 분석형 오류 설명을 포함하면 안 된다.
UX-DR6: `Preview Waiting` 화면은 첫 문장에서 사진 저장 완료 사실을 먼저 말하고, 둘째 문장에서 확인용 사진 준비 중임을 설명하며, 현재 가능한 다음 행동을 함께 제시해야 한다.
UX-DR7: `Preview Waiting` 상태에서는 최신 사진 레일이 아직 비어 있어도 정상임을 보조 문구로 알려야 하며, 지연이 길어져도 고객에게 내부 실패 원인을 노출하면 안 된다.
UX-DR8: `Phone Required` 화면은 도움 요청 중심의 보호 화면이어야 하며, 현재 세션 보존 여부 설명, 단일 연락 액션, 고객이 하지 말아야 할 행동을 짧게 포함해야 한다.
UX-DR9: 운영자/내부 프리셋 관리 진입점은 고객 기본 흐름에서 숨겨져 있어야 하며, 관리자 비밀번호 인증 이전에는 시각적으로도 노출되지 않아야 한다.
UX-DR10: 부스 UI는 고대비, 멀티모달 알림, 터치 친화적 조작을 유지해야 하며, 핵심 터치 요소는 넉넉한 터치 영역을 가져야 한다.
UX-DR11: 프리셋 카탈로그는 큰 프리셋 카드 컴포넌트로 구현되어야 하며, 각 카드는 예시 이미지, 룩 이름, 선택 상태 강조를 포함해야 한다.
UX-DR12: 시간 안내 컴포넌트는 디지털 타이머와 상태별 시각 강조를 제공해야 하며, 5분 전과 종료 시점에 사운드 알림과 함께 동작해야 한다.
UX-DR13: 최신 사진 레일은 현재 세션 썸네일만 가로 스크롤로 보여줘야 하며, 삭제 액션은 현재 세션 삭제 정책 범위 안에서만 노출되어야 한다.
UX-DR14: `Preview Waiting Panel`과 `Phone Required Support Card`는 별도 재사용 컴포넌트로 설계되어, 고객 보호 메시지 위계와 단일 행동 원칙을 일관되게 유지해야 한다.
UX-DR15: 부스 메인 화면은 1024px 이상 대형 터치스크린 기준으로 최적화하고, 운영자 화면은 768~1023px 범위를 지원하되, MVP에서는 고객용 모바일 화면을 만들지 않아야 한다.
UX-DR16: 접근성 목표는 WCAG 2.2 AA 수준으로 두어야 하며, 시맨틱 HTML, 명확한 포커스 관리, 모달 포커스 가두기와 ESC 닫기를 지원해야 한다.
UX-DR17: 주요 상태 변화는 시각 배지와 브랜드 사운드를 함께 사용해야 하며, 특히 경고·종료·에스컬레이션 상태는 공포를 키우지 않는 안정적 위계로 설계해야 한다.
UX-DR18: 대상 고객 모니터의 read-only 관람 화면은 촬영 전에 현재 세션, monitor profile, photo rectangle, DPR, listener, layout 준비 상태를 증명해야 한다.
UX-DR19: 관람 화면의 첫 qualifying frame은 같은 session/request/capture/preset@version에 연결된 physical display-fit preset-applied 이미지여야 하며 무보정 camera image나 review rail thumbnail을 성공으로 계산하면 안 된다.
UX-DR20: proxy에서 RAW 정밀본으로 전환할 때 다음 asset의 완전 decode 전까지 기존 이미지를 유지하고 blank, spinner, 이전 촬영, 다른 preset, crop/scale jump, quality-tier downgrade를 0건으로 유지해야 한다.

### FR Coverage Map

FR1: Epic 1 - 세션 시작 입력과 부스 별칭 생성
FR2: Epic 1 - 승인 프리셋 선택과 활성 프리셋 설정
FR3: Epic 1 - 준비 상태 안내와 유효 상태에서만 촬영 허용
FR4: Epic 1 - 현재 세션 저장 성공과 프리뷰 대기/준비 구분
FR5: Epic 2 - 현재 세션 검토, 삭제, 세션 중 프리셋 변경
FR6: Epic 2 - 조정된 종료 시각, 5분 경고, 종료 시각 행동 안내
FR7: Epic 3 - 종료 후 `Export Waiting` / `Completed` / `Phone Required` 흐름
FR8: Epic 4 - 내부 프리셋 작성, 승인, 게시, 롤백
FR9: Epic 5 - 운영자 진단, 복구, 라이프사이클/개입 로그
FR10: Epic 7 - 촬영 전 전용 관람 화면, 화면 적합 preset 표시, RAW 정밀본 무중단 교체
Operational NFRs: Epic 6 - 지점별 단계적 배포와 단일 액션 롤백 거버넌스
Viewer performance and release NFRs: Epic 7 - actual-present KPI, immutable display tiers, source/renderer evidence, complete installer, 100-shot gate

## Epic List

### Epic 1: 빠른 세션 시작과 자신감 있는 첫 촬영
고객이 이름+휴대전화 뒤4자리로 빠르게 세션을 시작하고, 승인된 프리셋을 고른 뒤, 준비 상태를 이해하며 첫 촬영을 성공적으로 저장하고 프리뷰 대기까지 신뢰할 수 있게 한다.
**FRs covered:** FR1, FR2, FR3, FR4

### Epic 2: 현재 세션 중심의 촬영 제어와 시간 인지
고객이 현재 세션 사진만 검토하고 정책 범위 내에서 삭제하며, 세션 중 언제든 프리셋을 바꾸고, 조정된 종료 시각과 경고 알림을 이해하면서 촬영을 이어갈 수 있게 한다.
**FRs covered:** FR5, FR6

### Epic 3: 종료 후 결과 준비와 인계의 진실한 마무리
고객이 촬영 종료 후 `Export Waiting`, `Completed`, `Phone Required` 중 현재 상태를 정확히 이해하고, 완료 또는 인계 행동을 혼란 없이 수행할 수 있게 한다.
**FRs covered:** FR7

### Epic 4: 승인 기반 프리셋 게시와 미래 세션 카탈로그 운영
내부 프리셋 관리자가 프리셋을 작성, 검증, 승인, 게시, 롤백하여 미래 세션용 카탈로그를 안전하게 운영할 수 있게 한다.
**FRs covered:** FR8

### Epic 5: 운영자 복구와 감사 로그
운영자가 안전한 범위에서 현재 세션 문제를 진단·복구하고, 개입 및 결과를 감사 가능하게 남길 수 있게 한다.
**FRs covered:** FR9

### Epic 6: 지점 배포와 롤백 거버넌스
owner / brand operator가 선택된 지점 집합에 대해 빌드와 승인된 프리셋 스택을 안전하게 배포·롤백할 수 있게 한다.
**Primary coverage:** NFR2, NFR6, rollout/rollback additional requirements

### Epic 7: 저지연 프리셋 관람 경험의 검증과 출시
고객이 촬영 전에 준비된 전용 관람 화면에서 선택한 프리셋이 적용된 화면 적합 사진을 빠르게 보고, RAW 정밀본으로 시각적 중단 없이 전환되는 경험을 실제 모니터와 완전한 설치 환경에서 검증받게 한다.
**FRs covered:** FR4, FR10
**Primary coverage:** NFR3, NFR4, NFR6, UX-DR18, UX-DR19, UX-DR20

<!-- Repeat for each epic in epics_list (N = 1, 2, 3...) -->

## Epic 1: 빠른 세션 시작과 자신감 있는 첫 촬영

고객이 이름+휴대전화 뒤4자리로 빠르게 세션을 시작하고, 승인된 프리셋을 고른 뒤, 준비 상태를 이해하며 첫 촬영을 성공적으로 저장하고 프리뷰 대기까지 신뢰할 수 있게 한다.

### Story 1.1: Set up initial project from starter template

As a owner / brand operator,
I want a single packaged booth runtime with separated booth, operator, and authoring surfaces,
So that customer-facing features can ship on a safe desktop foundation without exposing internal tools.

Implementation Note: Story 1.1 is prerequisite scaffolding for greenfield bootstrap and must not be counted as customer-visible Epic 1 value completion by itself.

**Acceptance Criteria:**

**Given** a fresh project bootstrap state
**When** the app is initialized for MVP development
**Then** it uses the approved `Vite react-ts + Tauri` baseline with top-level surfaces for `/booth`, `/operator`, `/authoring`, and `/settings`
**And** operator/authoring surfaces are hidden behind capability checks rather than exposed in the default customer flow

**Given** the default customer launch path
**When** a customer opens the app
**Then** only the booth surface is reachable without admin authentication
**And** no internal preset-authoring or operator controls appear in the customer UI

**Given** a clean checkout and the committed lockfile
**When** the bootstrap baseline is verified
**Then** dependency installation, frontend development/build smoke, Tauri integration/package smoke, and the initial CI baseline complete successfully
**And** the accepted command output or CI run is referenced as Story 1.1 completion evidence

### Story 1.2: 이름+뒤4자리 기반 세션 시작과 내구적 세션 생성

As a booth customer,
I want to start a session with only my name and phone last four digits,
So that I can enter the booth quickly without reservation or full phone-number friction.

**Acceptance Criteria:**

**Given** the booth start screen
**When** the customer enters a valid non-empty name and a valid four-digit phone suffix
**Then** the system creates an active session with a customer-facing booth alias and a separate internal `sessionId`
**And** the session is persisted in the session-scoped filesystem root with the initial session manifest

**Given** the booth start screen
**When** the customer enters an empty name, non-numeric suffix, or suffix that is not four digits
**Then** the system blocks continuation and shows plain-language validation guidance
**And** it does not require full phone number entry or reservation verification

### Story 1.3: 승인된 프리셋 카탈로그 표시와 활성 프리셋 선택

As a booth customer,
I want to choose one approved preset from a simple catalog,
So that I can understand the look I am getting before I start shooting.

**Acceptance Criteria:**

**Given** an active session
**When** the customer reaches preset selection at session start or later during the same session
**Then** the booth shows only 1 to 6 approved published presets with a customer-facing name and representative preview tile or sample cut
**And** no direct editing controls or darktable terminology are displayed

**Given** the preset selection screen
**When** the customer selects one preset or changes to a different preset
**Then** that preset becomes the active preset for the session
**And** the active preset identity and published version are stored for later captures without re-binding already saved captures

### Story 1.4: 준비 상태 안내와 유효 상태에서만 촬영 허용

As a booth customer,
I want the booth to clearly tell me when I can shoot and when I should wait,
So that I can trust the capture flow without understanding device internals.

Scope ownership: Story 1.4는 host-normalized readiness의 고객 상태 표현과 capture guard UI를 소유한다. 실제 helper/camera freshness, reconnect, false-ready hardware proof와 제품 `Ready` 출시 truth는 Story 1.6이 소유한다. Story 1.4의 기존 Go evidence는 회귀 증거로 보존한다.

**Acceptance Criteria:**

**Given** an active session and selected preset
**When** the booth receives host-normalized readiness changes
**Then** the booth translates runtime truth into plain-language customer states such as `Preparing`, `Ready`, or wait/call guidance
**And** the UI does not infer `Ready` from browser fallback, fixture state, or raw helper text
**And** customer copy stays within the approved low-density guidance rule

**Given** the booth is not in an approved capture state
**When** the customer attempts to capture
**Then** capture is blocked
**And** the booth tells the customer whether to wait or call without exposing technical diagnostics

**Given** the booth loses camera or helper readiness after previously being ready
**When** the live capture boundary degrades or disconnects
**Then** the booth immediately exits `Ready`
**And** the `사진 찍기` action becomes disabled without waiting for browser fallback or stale readiness refresh

**Given** Story 1.4 UI, contract, and automated tests are complete
**When** the team evaluates Story 1.4 done status
**Then** the story may close without claiming product-level real-camera `Ready` release truth
**And** Story 1.6 remains the canonical owner of HV-02, HV-03, and HV-10

### Story 1.5: 현재 세션 촬영 저장과 truthful preview waiting 피드백

As a booth customer,
I want capture success and preview readiness to be communicated separately,
So that I know my photo is saved even if the confirmation preview is still being prepared.

Scope ownership: Story 1.5는 persisted-capture truth의 고객 상태와 `Preview Waiting` 문구를 소유한다. 실제 capture request, RAW arrival, session persistence correlation은 Story 1.7이 소유하고 preset-applied `previewReady`와 render truth는 Story 1.8이 소유한다. Story 1.5의 기존 Go evidence는 회귀 증거로 보존한다.

**Acceptance Criteria:**

**Given** the booth is in a valid capture state with an active preset
**When** the host reports current-session persisted-capture truth
**Then** success feedback is shown only after that truth is available
**And** the active preset remains visible on the capture or confirmation surface

**Given** a successful capture whose customer-safe preview is not yet ready
**When** the booth enters `Preview Waiting`
**Then** the first message confirms the photo was saved
**And** the next message explains that the confirmation preview is being prepared and what the customer can do next

**Given** a successful capture is acknowledged
**When** the booth reports the immediate outcome
**Then** the primary customer action is acknowledged within 1 second
**And** the booth remains in truthful `Preview Waiting` until Story 1.8 render truth reports the preset-applied preview ready

**Given** the booth is in `Preview Waiting`
**When** the preview rail is still empty
**Then** the UI explains that this can be normal for the current session
**And** no internal render failure cause is shown to the customer

**Given** Story 1.5 state, copy, and automated tests are complete
**When** the team evaluates Story 1.5 done status
**Then** the story may close without claiming real-camera round-trip or render-backed preview release truth
**And** Story 1.7 owns canonical HV-04 while Story 1.8 owns canonical HV-05

### Story 1.6: 실카메라/helper readiness truth 연결과 false-ready 차단

As a booth customer,
I want `Ready` to open only after the real helper and camera report fresh readiness through the live host boundary,
So that the booth never tells me to shoot from stale or synthetic truth.

**Acceptance Criteria:**

**Given** an approved booth hardware environment
**When** the bundled `canon-helper.exe` baseline is launched by the Tauri host and the host receives fresh `helper-ready` and `camera-status`
**Then** the booth may enter `Ready` only after the first fresh camera-ready truth is confirmed
**And** `helper-ready` alone does not enable capture

**Given** the booth is running in browser preview, fixture mode, stale readiness, disconnected or degraded camera/helper state, or reconnect-before-fresh-truth
**When** readiness is evaluated
**Then** the booth does not claim `Ready`
**And** capture remains blocked with plain-language wait or call guidance

**Given** the booth was previously ready and the helper process exits, the camera disconnects, or readiness degrades
**When** the live hardware boundary changes
**Then** the booth immediately exits `Ready`
**And** it does not auto-return until fresh `camera-status` truth is observed again

**Given** Story 1.6 is reviewed for closure
**When** the helper project skeleton, host spawn/health management, or HV-02, HV-03, HV-10 evidence is incomplete
**Then** the story remains in `in-progress` or `review`
**And** it cannot be treated as release-safe readiness truth

### Story 1.7: 실카메라 capture round-trip과 RAW handoff correlation

As a booth customer,
I want a real capture to finish only when the helper delivers the correct file back to my session,
So that the booth never mistakes shutter acceptance for a saved photo.

**Acceptance Criteria:**

**Given** an approved booth hardware environment in a fresh ready state
**When** the customer uses the booth app `사진 찍기` action and the host sends `request-capture` to the bundled helper
**Then** the helper accepts or rejects one correlated in-flight capture request
**And** the host keeps a single in-flight capture guard for that request

**Given** the camera body shutter is pressed directly without a host `request-capture`
**When** capture resolution is evaluated for the active booth session
**Then** the event is not treated as a supported booth capture success path
**And** it must not be used as Story 1.7 closure evidence or silently attached to the active session success UI

**Given** the helper accepts a capture
**When** shutter trigger, RAW download, and final file handoff are still in progress
**Then** the booth does not treat `capture-accepted` as capture success
**And** success is confirmed only after correlated `file-arrived` and actual file presence are verified

**Given** duplicate arrival, wrong session correlation, missing file, timeout, or a second capture during an in-flight capture
**When** capture resolution is evaluated
**Then** the host blocks false success and unsafe parallel capture
**And** the booth falls back to truthful wait or recovery guidance without cross-session leakage

**Given** Story 1.7 is reviewed for closure
**When** HV-04 primary evidence or HV-05 supporting evidence for the booth-app `사진 찍기` path is missing on approved booth hardware
**Then** the story remains open
**And** it does not inherit closure from Story 1.6, Story 1.5 historical evidence alone, or synthetic preview flow

### Story 1.8: 게시된 프리셋 XMP 적용과 preview/final render worker 연결

As a booth customer,
I want the preset I selected to be actually applied to my booth preview and final output,
So that the look I chose matches what I see and what is later delivered.

**Acceptance Criteria:**

**Given** an active session has a published preset binding and Story 1.7 has already confirmed RAW persistence
**When** the host starts booth render follow-up for that capture
**Then** the runtime resolves the immutable published bundle for the capture-bound `presetId + publishedVersion`
**And** preview render uses the bundle `xmpTemplatePath`, pinned darktable version, and preview profile through `darktable-cli`
**And** `previewReady` is recorded only after a real raster preview file exists under `renders/previews/`

**Given** preview render succeeds for a booth capture
**When** the latest-photo rail and confirmation surface update
**Then** they use the preset-applied capture preview rather than a raw copy, placeholder fallback, or bundle representative tile
**And** if render has not completed or has failed, the booth remains in truthful `Preview Waiting` or bounded failure guidance instead of false-ready

**Given** shooting has ended and the booth evaluates final deliverable truth
**When** final render is required for the active session
**Then** the runtime uses the same capture-bound published bundle and its final profile to write a real asset under `renders/finals/`
**And** `finalReady` and `Completed` are never claimed before that asset actually exists

**Given** a newer catalog publish or rollback happens after a capture was already saved
**When** preview or final render is evaluated for that existing capture
**Then** the runtime uses the preset version stored on that capture record
**And** it does not silently drift to the newer live catalog version

**Given** `darktable-cli` is unavailable, the bundle is malformed, the XMP template is missing, or render fails or times out
**When** render truth is evaluated
**Then** the host preserves RAW and current-session assets, records bounded render failure truth, and avoids cross-session leakage
**And** it does not hide the failure behind a raw-copy success or false `previewReady` / `Completed`

Story 1.8 scope note:
- 이 스토리는 preset-applied `previewReady` / `finalReady` truth의 소유자다.
- first-visible same-capture preview latency 보정은 후속 Story 1.9가 맡되, Story 1.8의 render-backed ready 기준을 느슨하게 만들면 안 된다.
- 2026-08-11 correct-course 이후 이 스토리는 정확한 RAW-refined/final/parity/fallback 경로를 유지한다. 전용 관람 화면의 출시 성능 성공은 Epic 7이 별도로 소유한다.

### Story 1.9: fast preview handoff와 XMP preview 교체

As a booth customer,
I want my just-captured photo to appear in the current session as quickly as safely possible even before the preset-applied preview is fully ready,
So that I do not experience a blank wait after the booth says my photo was saved.

2026-08-11 scope note: 이 스토리는 camera source와 진행 피드백의 참고 구현으로 유지한다. 무보정 fast preview와 사진 레일 표시는 전용 관람 화면의 qualifying preset-applied 성공이나 Epic 7 release evidence를 대신하지 않는다. pending preview와 later preset-applied preview는 capture-scoped immutable generation으로 게시하고 validated pointer를 전환하며, 한 canonical JPEG의 in-place overwrite는 금지한다.

**Acceptance Criteria:**

**Given** Story 1.7 has already confirmed RAW persistence for the active session
**When** the helper or host can provide a same-capture fast preview path
**Then** that handoff is optional and must not become a prerequisite for capture success
**And** the absence of fast preview falls back to the existing truthful `Preview Waiting` path

**Given** a fast preview handoff is available
**When** the host validates same-session, same-capture, and allowed-path rules
**Then** it may publish that asset as an immutable pending-preview generation under the session/capture boundary
**And** the generation carries the correlated session, request, capture, source hash, and revision
**And** it must keep `previewReady` and `readyAtMs` unset until the later preset-applied render actually finishes

**Given** the booth is still in `Preview Waiting`
**When** a valid same-capture pending-preview generation is active
**Then** the latest-photo rail and confirmation surface may show that pending image to reduce blank waiting
**And** the booth must not imply that the preset-applied booth-safe preview is already ready

**Given** the preset-applied preview later completes through the Story 1.8 render worker path
**When** the runtime publishes the real booth-safe preview as a newer immutable generation
**Then** the active preview pointer advances atomically only after full write, decode, dimension, and correlation validation
**And** the pending generation remains immutable and may be retired only by the approved cleanup policy
**And** only that later render-backed output may set `previewReady` and `readyAtMs`

**Given** a fast preview is missing, invalid, stale, cross-session, or otherwise unsafe
**When** the host evaluates promotion
**Then** the booth discards that fast preview without failing capture truth
**And** it continues with truthful `Preview Waiting` and the normal render follow-up path

**Given** approved booth hardware and timing instrumentation are available
**When** the team validates this story
**Then** telemetry separates fast-preview visibility from preset-applied preview readiness
**And** hardware evidence confirms same-capture correctness, cross-session isolation, and safe behavior during burst capture queue delay

### Story 1.10: 관리자 인증과 privileged capability 경계 완성

Type: Security Enabler

As an authorized operator or preset manager,
I want privileged surfaces and host commands to require a verified local admin session,
So that operator, authoring, settings, publication, recovery, and rollout capabilities cannot be unlocked by route knowledge or a process flag alone.

**Acceptance Criteria:**

**Given** an operator, authoring, or settings surface is requested without a verified privileged session
**When** access is evaluated
**Then** the route, window, and host command boundaries all deny access
**And** no privileged navigation, data, or action becomes partially visible

**Given** a valid local admin password is submitted
**When** the Rust host verifies it against OS-appropriate protected verification material
**Then** it issues a bounded privileged session with allowed capabilities, expiry, and explicit logout/revocation behavior
**And** raw password or verification material is never stored in branch config, frontend state persistence, logs, or audit payloads

**Given** an invalid password, expired or revoked session, locked state, or denied capability
**When** a privileged surface or command is requested
**Then** access is rejected consistently with an operator-safe result
**And** the bounded success, failure, denial, logout, and credential-rotation outcome is auditable without secret material

**Given** the current environment-driven `BOOTHY_ADMIN_AUTHENTICATED` seam
**When** production readiness is evaluated
**Then** that seam is treated as development scaffolding only
**And** Epic 4/5 privileged workflows and Epic 6 rollout controls cannot be release-ready until Story 1.10 is complete

## Epic 2: 현재 세션 중심의 촬영 제어와 시간 인지

고객이 현재 세션 사진만 검토하고 정책 범위 내에서 삭제하며, 세션 중 언제든 프리셋을 바꾸고, 조정된 종료 시각과 경고 알림을 이해하면서 촬영을 이어갈 수 있게 한다.

### Story 2.1: 현재 세션 사진 레일과 세션 범위 검토

As a booth customer,
I want to review only the photos from my current session,
So that I can confirm my recent captures without seeing anyone else’s images.

**Acceptance Criteria:**

**Given** an active session with one or more successful captures
**When** the customer opens or views the review rail
**Then** the UI shows only assets correlated to the active session
**And** the latest available current-session photo is visually distinguishable within the rail

**Given** the review surface is rendered
**When** session-scoped assets are loaded
**Then** no prior-session or other-session assets are shown
**And** the customer-facing UI does not expose filesystem, render-engine, or diagnostic terminology

**Given** current-session review data is queried, refreshed, or deleted
**When** booth-safe assets are resolved for the active customer
**Then** only assets correlated to the active session may be returned
**And** cross-session asset leakage remains 0 across review, preview, and deletion flows

### Story 2.2: 현재 세션 삭제 정책에 따른 안전한 사진 삭제

As a booth customer,
I want to delete only my current session photos when allowed,
So that I can clean up unwanted shots without affecting anything outside my session.

**Acceptance Criteria:**

**Given** a current-session capture that is eligible under the `Current-Session Deletion Policy`
**When** the customer confirms deletion
**Then** the selected current-session capture and its correlated booth-safe artifacts are removed from the review surface
**And** no assets outside the active session are affected

**Given** a capture is not deletable under the active policy
**When** the customer attempts deletion
**Then** the system blocks the action
**And** the customer sees plain-language guidance instead of internal policy or storage details

### Story 2.3: 세션 중 활성 프리셋 자유 변경

As a booth customer,
I want to change the active preset at any time during my session,
So that I can switch to a different approved look whenever I want without changing past captures.

**Acceptance Criteria:**

**Given** an active session with an already selected preset
**When** the customer chooses a different approved published preset at any point during the session
**Then** the new preset becomes the active preset immediately for the session
**And** previously captured session assets remain bound to the preset version used at capture time

**Given** the customer is on capture, review, or preset-selection related surfaces
**When** the active preset changes
**Then** the UI clearly indicates the newly active preset for the next capture
**And** it does not imply that prior captures were re-edited or re-bound

**Given** the customer requests a preset switch
**When** the selected preset is no longer available or the preset binding cannot be applied safely
**Then** the previously active preset remains the active preset for the session
**And** the customer sees plain-language guidance to keep the current preset or choose another approved preset

**Given** a preset switch succeeds
**When** the booth acknowledges the change
**Then** the active-preset confirmation is acknowledged within 1 second on approved hardware
**And** no previously saved current-session asset is mutated by the switch

### Story 2.4: 조정된 종료 시각 표시와 경고/종료 알림

As a booth customer,
I want to understand exactly how much booth time I have left and what happens near the end,
So that I can finish my session confidently without guessing.

**Acceptance Criteria:**

**Given** an active session governed by the `Session Timing Policy`
**When** the customer enters the booth flow after session start
**Then** the adjusted end time is visible from the beginning of the session
**And** the time display uses customer-safe wording and visual hierarchy appropriate for the booth UI

**Given** the session reaches 5 minutes before the adjusted end time
**When** the warning threshold is crossed
**Then** the booth plays the approved warning sound and shows a visible warning state
**And** the customer can still understand whether shooting may continue

**Given** the session reaches the adjusted end time
**When** the end threshold is crossed
**Then** the booth plays the approved end alert and updates guidance to reflect whether shooting has ended
**And** the state change occurs without exposing internal scheduler or policy terminology

**Given** a qualifying session reaches the warning or adjusted end threshold
**When** lifecycle timing is evaluated in production conditions
**Then** the 5-minute warning and exact-end alert occur within +/- 5 seconds in 99% of sessions
**And** post-end capture attempts remain blocked unless an approved extension is applied and logged

## Epic 3: 종료 후 결과 준비와 인계의 진실한 마무리

고객이 촬영 종료 후 `Export Waiting`, `Completed`, `Phone Required` 중 현재 상태를 정확히 이해하고, 완료 또는 인계 행동을 혼란 없이 수행할 수 있게 한다.

### Story 3.1: 종료 직후 명시적 사후 상태 진입

As a booth customer,
I want the booth to move into a clear post-end state as soon as shooting ends,
So that I immediately understand whether I should wait, finish, or ask for help.

**Acceptance Criteria:**

**Given** an active session reaches the adjusted end time
**When** shooting is no longer allowed
**Then** the booth transitions into exactly one explicit post-end state from `Export Waiting`, `Completed`, or `Phone Required`
**And** the customer does not remain in an ambiguous in-between capture state

**Given** the booth enters a post-end state
**When** the state is rendered to the customer
**Then** the UI uses customer-safe wording without technical diagnostics
**And** the next action is visually clear

### Story 3.2: Export Waiting과 truthful completion 안내

As a booth customer,
I want to know whether my final deliverable is still being prepared or already complete,
So that I do not leave too early or worry that my session failed when it is still processing.

**Acceptance Criteria:**

**Given** shooting has ended and the booth-side deliverable is not yet ready
**When** the post-end state is evaluated
**Then** the booth shows `Export Waiting` guidance
**And** shooting remains disabled while wait guidance is displayed

**Given** all booth-side required work is actually complete
**When** the booth enters `Completed`
**Then** the result resolves as either `Local Deliverable Ready` or `Handoff Ready`
**And** the UI does not claim completion before the required booth-side work is finished

**Given** the adjusted end time has been reached
**When** the host finalizes post-end evaluation
**Then** 90% or more of sessions enter an explicit post-end state within 10 seconds of scheduled end time
**And** render retries or failures do not invalidate already saved current-session captures

**Given** Story 3.2 implementation and automated tests are complete
**When** the team evaluates done status
**Then** the story remains in `review` until HV-08 and HV-11 evidence confirms no false-complete outcome on approved booth hardware

### Story 3.3: Handoff Ready와 Phone Required 보호 안내

As a booth customer,
I want the booth to clearly tell me where to go next when handoff is ready or help is required,
So that I can leave the booth confidently without guessing or taking unsafe actions.

**Acceptance Criteria:**

**Given** the booth resolves the session as `Handoff Ready`
**When** the handoff screen is shown
**Then** the customer sees the approved recipient or next location together with the approved next action
**And** the booth alias is shown if it is required for downstream handoff

**Given** the session cannot resolve normally within approved bounds
**When** the booth enters `Phone Required`
**Then** the screen explains the protected state in customer-safe language and presents one primary contact action
**And** it briefly blocks unsafe self-recovery actions such as repeated capture attempts or device restart attempts

## Epic 4: 승인 기반 프리셋 게시와 미래 세션 카탈로그 운영

내부 프리셋 관리자가 프리셋을 작성, 검증, 승인, 게시, 롤백하여 미래 세션용 카탈로그를 안전하게 운영할 수 있게 한다.

### Story 4.1: 드래프트 프리셋 작성과 내부 저작 작업공간

As a authorized preset manager,
I want to create and edit draft preset versions in an internal authoring surface,
So that new booth looks can be prepared without exposing authoring tools to booth customers.

**Acceptance Criteria:**

**Given** an authenticated internal authoring session
**When** the preset manager creates a new preset draft or edits an existing draft version
**Then** the work is saved as a draft-only preset artifact candidate within the internal workflow
**And** the customer booth catalog remains unchanged

**Given** the authoring surface is opened
**When** a draft preset is being edited
**Then** the UI exposes only authorized internal controls
**And** those controls are unreachable from the default booth customer flow

### Story 4.2: 부스 호환성 검증과 승인 준비 상태 전환

As a authorized preset manager,
I want to validate a draft preset for booth compatibility before approval,
So that only safe and reproducible presets can advance toward publication.

**Acceptance Criteria:**

**Given** a draft preset version exists
**When** the manager runs booth compatibility validation
**Then** the system evaluates the preset against the required render compatibility and artifact rules
**And** the draft can move from `draft` to `validated` only if the checks pass

**Given** a draft preset fails validation
**When** the validation result is returned
**Then** the preset remains out of the customer catalog
**And** the internal user sees actionable validation feedback without changing active sessions

**Given** Story 4.2 implementation and automated tests are complete
**When** the team evaluates done status
**Then** the story remains in `review` until HV-01 and HV-09 evidence confirms draft and validated artifacts cannot leak into booth runtime

### Story 4.3: 승인과 불변 게시 아티팩트 생성

As a authorized preset manager,
I want to approve a validated preset and publish it as an immutable versioned artifact,
So that future booth sessions can use a stable and traceable preset catalog entry.

**Acceptance Criteria:**

**Given** a preset version is in the `validated` state
**When** an authorized approver approves and publishes it
**Then** the system creates an immutable published preset artifact bundle with stable identity, version, and catalog metadata
**And** the preset lifecycle advances through `approved` to `published`

**Given** a preset has been published
**When** future sessions load the booth catalog
**Then** the published preset can appear as a selectable catalog item
**And** active sessions are not mutated by the publication event

**Given** a publication request is attempted
**When** validation is stale, required artifact metadata is incompatible, or publication would violate immutability or future-session-only rules
**Then** publication is rejected
**And** no `published` artifact is created
**And** the preset remains in its prior lifecycle state
**And** the authorized user sees actionable rejection guidance

**Given** a publication request is rejected
**When** the rejection is finalized
**Then** the system records the rejected action, reason, actor, and timestamp in the audit history
**And** the booth catalog and active sessions remain unchanged

**Given** Story 4.3 implementation and automated tests are complete
**When** the team evaluates done status
**Then** the story remains in `review` until HV-01, HV-07, and HV-12 evidence confirms published bundles drive booth output without preset drift

### Story 4.4: 미래 세션 대상 롤백과 카탈로그 버전 관리

As a authorized preset manager,
I want to roll back the booth catalog to a prior approved preset version,
So that I can recover from a bad release without breaking active sessions.

**Acceptance Criteria:**

**Given** multiple approved published versions exist for a preset identity
**When** the manager chooses a rollback target
**Then** the system makes the selected prior approved version the future-session catalog version
**And** active sessions retain their currently bound preset versions

**Given** a publication or rollback action occurs
**When** the action is completed
**Then** the system records the preset identity, version, action type, timestamp, and actor in the audit history
**And** branch-visible catalog state remains internally consistent

**Given** a rollback target is unapproved, missing, incompatible, already active, or based on a stale catalog revision
**When** rollback is requested
**Then** the request is rejected without changing the catalog pointer, immutable bundles, or active-session bindings
**And** the rejection reason, actor, target, and timestamp are preserved in the audit journal

**Given** rollback is interrupted or its audit write cannot be committed
**When** the operation resolves
**Then** the prior valid catalog state remains authoritative and retry is idempotent
**And** no partial target or half-written audit result becomes visible to future sessions

## Epic 5: 운영자 복구와 감사 로그

운영자가 안전한 범위에서 현재 세션 문제를 진단·복구하고, 개입 및 결과를 감사 가능하게 남길 수 있게 한다.

### Story 5.1: 운영자용 현재 세션 문맥과 장애 진단 가시화

As a remote operator,
I want to see the active session context and blocked-state diagnostics,
So that I can understand whether the booth is blocked in capture, preview, render, or completion without guessing.

**Acceptance Criteria:**

**Given** a booth session is active or blocked
**When** the operator opens the operator console
**Then** the console shows current session identity, timing state, recent failure context, and the normalized blocked-state category
**And** the view separates capture-side blockage from preview/render/completion blockage

**Given** the operator console displays a blocked session
**When** diagnostic information is rendered
**Then** the UI uses operator-safe diagnostic detail rather than raw helper output
**And** customer-facing booth copy remains unaffected

### Story 5.2: 정책 기반 복구 액션과 Phone Required 라우팅

As a remote operator,
I want to execute only approved recovery actions for the active failure category,
So that I can restore safe operation without taking unbounded or risky actions.

**Acceptance Criteria:**

**Given** a blocked session category is identified
**When** the operator opens the available actions panel
**Then** the console shows only the actions allowed by the `Operator Recovery Policy` for that category
**And** disallowed actions are not executable from the UI

**Given** an allowed operator action such as retry, approved boundary restart, or allowed time extension is selected
**When** the action completes
**Then** the session transitions to the correct next normalized state or to `Phone Required` if safe recovery cannot continue
**And** the action does not expose unsafe recovery controls to the customer flow

**Given** the operator view is stale, the session changed, or the requested action is no longer allowed
**When** the action reaches the host
**Then** it is rejected against the latest normalized session revision without mutating session truth
**And** the operator receives a refreshed bounded action set

**Given** duplicate or concurrent recovery requests, timeout, boundary failure, or an indeterminate action result
**When** recovery resolution is evaluated
**Then** idempotency and single-writer guards prevent duplicate effects and unsafe state advancement
**And** the final success, rejection, timeout, or failure is preserved in the audit journal before retry or `Phone Required` routing

### Story 5.3: 라이프사이클, 개입, 복구 감사 로그 기록

As a owner / operations lead,
I want lifecycle and intervention events to be recorded consistently,
So that we can audit failures, recovery behavior, timing outcomes, and support burden across branches.

**Acceptance Criteria:**

**Given** a session lifecycle transition, operator intervention, publication-related recovery event, or critical failure occurs
**When** the event is finalized by the host
**Then** the system appends it to a versioned JSON/JSONL audit journal with timestamp, actor or source, session reference, event type, correlation, and schema version
**And** the log is queryable by operators for support and retrospective review

**Given** audit data is reviewed
**When** operators or owners inspect session history
**Then** they can distinguish state transitions, intervention attempts, and final outcomes
**And** the journal remains separate from durable photo/session asset truth
**And** any SQLite view is rebuildable from the journal and never becomes the primary audit truth

### Story 5.4: 운영자용 카메라 연결 상태 전용 항목과 helper readiness 가시화

As a remote operator,
I want camera connection status to appear as a dedicated diagnostic item,
So that I can spot false-ready risk before it is hidden inside a generic blocked-state summary.

**Acceptance Criteria:**

**Given** the operator console is opened for an active or blocked booth session
**When** diagnostics are rendered
**Then** the console shows a dedicated `카메라 연결 상태` item in addition to the generic blocked-state category
**And** the item is derived from host-normalized camera/helper truth

**Given** the camera or helper is disconnected, still preparing, ready, or degraded after readiness
**When** the operator reviews the session
**Then** the dedicated item shows one explicit operator-safe state for that condition
**And** the UI does not expose raw helper output or booth-customer copy

**Given** the booth could otherwise appear ready from stale or incomplete truth
**When** the operator reviews the active session
**Then** the dedicated camera connection item makes the risk visible before a false-ready release decision is made

## Epic 6: 지점 배포와 롤백 거버넌스

owner / brand operator가 선택된 지점 집합에 대해 빌드와 승인된 프리셋 스택을 안전하게 배포·롤백할 수 있게 한다.

### Story 6.1: 지점별 단계적 배포와 단일 액션 롤백 거버넌스

As a owner / brand operator,
I want to roll out and roll back builds and approved preset stacks by selected branch sets,
So that branches stay consistent without forcing updates during active customer sessions.

**Acceptance Criteria:**

**Given** a new approved build or preset stack is ready
**When** a rollout is initiated
**Then** the system targets an explicitly selected branch set rather than all branches at once
**And** the rollout records the branch set, target build, approved preset stack, approval timestamp, and actor

**Given** a rollout targets one or more approved branches
**When** the new build or preset stack is applied
**Then** each targeted branch preserves its approved local settings such as contact information and bounded operational toggles
**And** the rollout mutates only the approved build and preset-stack state for that branch set

**Given** any targeted branch has an active customer session
**When** rollout would interrupt that session
**Then** the system defers or rejects rollout for that branch
**And** no forced update is applied to the active session
**And** the refusal or deferral reason is surfaced to the initiating operator and recorded in audit history

**Given** a targeted branch has a customer session that legitimately continues on the currently approved baseline
**When** rollout or rollback state is evaluated for that branch
**Then** the active session remains compatible with its existing approved build and preset baseline until a safe transition point is reached
**And** the deployment transition does not invalidate or corrupt the in-flight session

**Given** a promoted branch must be reverted
**When** rollback is triggered
**Then** the branch returns to the last approved build and approved preset stack in one approved rollback action
**And** no active customer session is interrupted by forced update behavior

**Given** rollback is requested
**When** no approved rollback baseline exists or compatibility checks fail
**Then** rollback is rejected without mutating the branch state
**And** the initiating operator sees clear refusal guidance and the rejection is audited

**Given** rollback is approved for a selected branch set
**When** the prior approved baseline is restored
**Then** each branch preserves its approved local settings while returning to the last approved build and preset stack
**And** active-session compatibility remains protected until each branch reaches a safe transition point

Independent review gates:

- Gate A — Rollout Governance: explicit branch targeting, approval metadata, audit journal write, active-session defer/reject, and safe-point activation are reviewed as one independently passable deliverable.
- Gate B — Rollback & Compatibility: last-approved baseline recovery, local-settings preservation, missing/incompatible baseline rejection, old-session readability, and idempotent retry are reviewed as a separate deliverable.
- Story 6.1 may remain one Story because implementation already exists, but neither gate may inherit the other gate's pass result.

### Story 6.2: 실장비 hardware validation gate와 evidence 기반 done 정책

As a owner / brand operator,
I want sprint closure to require hardware validation evidence for truth-critical stories,
So that implementation completion is not mistaken for product readiness.

**Acceptance Criteria:**

**Given** Story 1.6, 1.7, 1.8, 3.2, 4.2, or 4.3 has completed implementation and automated tests
**When** the team evaluates story closure
**Then** the story does not move to product-level `done` until the mapped hardware validation evidence is attached
**And** the story remains in `review` or an equivalent pre-close state until then

**Given** historical Story 1.4 and 1.5 evidence
**When** the gate matrix is reviewed
**Then** it remains regression evidence for customer state/copy behavior
**And** real-camera readiness, capture persistence, and render-backed preview release truth remain owned by Story 1.6, 1.7, and 1.8 respectively

**Given** a truth-critical story is reviewed for hardware validation
**When** the team records closure evidence
**Then** the story references the exact checklist IDs, evidence location, execution date, and Go / No-Go result
**And** sprint review distinguishes automated pass from hardware pass

**Given** a hardware validation scenario results in `No-Go`
**When** the sprint status is updated
**Then** the impacted story remains or returns to `review`
**And** the release decision cannot claim booth `Ready` or `Completed` truth without the missing evidence

## Epic 7: 저지연 프리셋 관람 경험의 검증과 출시

고객이 촬영 전에 준비된 전용 관람 화면에서 선택한 프리셋이 적용된 화면 적합 사진을 빠르게 보고, RAW 정밀본으로 시각적 중단 없이 전환되는 경험을 실제 모니터와 완전한 설치 환경에서 검증받게 한다.

Epic 7 execution rule:
- Story는 직전 번호가 아니라 아래 명시된 evidence dependency를 따라 실행한다.
- 기본 dependency는 `7.1 Go -> 7.2 Go -> 7.3 approved route decision -> 7.4 Go -> 7.5 adoption decision -> 7.6 Go -> 7.7 Go -> 7.8 Go -> 7.9 Go -> 7.10 final decision`이다.
- 각 Story는 코드 구현, 관련 PRD/UX/Architecture/contract/story 문서 수정, 자동화 검증, 실장비 evidence를 모두 포함한다.
- Story 7.3 `Enabler / Experiment`와 Story 7.5 `Architecture Spike`는 후보가 No-Go여도 비교 구현, evidence, 대체 경로 결정이 완전하면 조사 목적을 닫을 수 있다. No-Go 후보는 production 경로로 승격하지 않는다.
- Story 7.10과 HV-18D가 Go이기 전까지 Epic 7과 새 MVP release는 No-Go다.
- 2026-08-12 correct-course: **120fps+ 물리 모니터 프레임 증거는 Story 7.8 / HV-18B가 단독으로 소유한다.** HV-13B는 자기가 증명한 범위(immutable publication, opaque swap, one-clock actual-present, 계측 완결성, A/B 결정, AC 4 실패 조건)로 재정의한다. 근거: 물리 프레임이 증명하는 compositor→photon 구간은 60Hz에서 최대 0.017초인 반면 실제 병목은 3.7~4.5초의 source/renderer 구간이며, 물리 검증의 대상이 되어야 할 실제 preset 적용 사진은 Story 7.4 이후에야 화면에 올라간다. 이관 근거와 방어선은 `sprint-change-proposal-20260812-183435.md`에 있다. **HV-18B가 표시 종단점 귀책의 전환 결함을 발견하면 Story 7.2를 `review`로 되돌린다.**

### Story 7.1: 촬영 전 전용 관람 창 준비와 화면 크기 계약

As a booth customer,
I want the viewing display to be ready and correctly sized before I capture,
So that every later image path starts from a trustworthy customer surface.

Scope boundary: 이 Story에서는 renderer, immutable sample publication, double-buffer swap, actual-present 계측을 구현하지 않는다. app-lifetime viewer, monitor targeting, session binding, physical display-size contract, readiness handshake, snapshot/reload reconciliation만 구현·수정·검증한다.

**Acceptance Criteria:**

**Given** a valid session starts on approved booth hardware
**When** capture eligibility is evaluated
**Then** a dedicated read-only viewer window is already created on the approved customer monitor
**And** it reports the current session binding, monitor profile, photo rectangle, DPR, listener readiness, and layout readiness before capture is enabled

**Given** an approved 1080p, 1440p, or 4K display profile
**When** the viewer calculates the required source dimensions
**Then** it derives the physical display-fit class from the actual photo rectangle and DPR
**And** it never treats a fixed 384px thumbnail or upscaled review-rail asset as qualifying

**Given** the viewer reloads, loses its listener, or detects a snapshot revision gap
**When** readiness state is reconciled
**Then** it restores the latest host-owned session binding and display profile from a monotonic snapshot
**And** it cannot remain capture-ready from a stale viewer epoch

**Given** the dedicated viewer is visible to the customer
**When** its production surface is inspected
**Then** it contains no editing, deletion, preset-selection, navigation, or diagnostic controls
**And** any readiness failure is projected to the booth control surface as truthful wait or call guidance

**Given** implementation and automated tests are complete
**When** Story 7.1 is reviewed for closure
**Then** it remains in `review` until HV-13A records Go for pre-capture viewer readiness, approved monitor targeting, current-session binding, and physical display-size contract

### Story 7.2: Immutable sample 표시와 actual-present 계측

Type: Enabler

As a booth product team,
I want immutable sample transitions to be measured on the real customer display,
So that later source and renderer changes are judged against one trustworthy presentation endpoint.

Scope boundary: 이 Story에서는 camera source나 preset renderer를 교체하지 않는다. request-scoped immutable sample generations, atomic display pointer, opaque double-buffer, one-clock actual-present telemetry, visible standby/hidden prewarm A/B만 구현·수정·검증한다.

**Acceptance Criteria:**

**Given** Story 7.1 and HV-13A are Go
**When** two request-scoped immutable sample generations are published in order
**Then** the next generation is fully written, closed, decoded, dimension-validated, and correlated before pointer commit
**And** a partial file, stale epoch, lower revision, older request, or insufficient dimension cannot become active display truth

**Given** one sample generation is currently visible
**When** a newer validated generation becomes ready
**Then** the viewer performs an opaque double-buffer swap while keeping the current image visible
**And** blank, spinner, stale image, crop jump, scale jump, and tier downgrade remain zero

**Given** an instrumented trusted capture action and pre-opened viewer
**When** the team measures the experience
**Then** the official start is trusted capture input and the official end is the qualifying frame on the physical monitor using one monotonic clock
**And** file-ready, renderer-ready, channel receipt, decode, and `<img onLoad>` remain separately recorded diagnostic spans

**Given** visible standby and hidden prewarm variants
**When** both are tested on the same PC, monitor, WebView2 runtime, and display profile
**Then** the selected default and rationale are recorded with raw evidence
**And** viewer navigation or creation after trusted capture input is a failing result

**Given** implementation and automated tests are complete
**When** Story 7.2 is reviewed for closure
**Then** it remains in `review` until HV-13B records Go with immutable publication, correlated timing spans, telemetry completeness where every committed generation has exactly one terminal row, and the visible-standby versus hidden-prewarm decision
**And** 120fps+ physical monitor frames, compositor-to-photon offset, and frame-level zero-transition-defect evidence are owned by Story 7.8 / HV-18B per the 2026-08-12 correct-course, so an HV-18B transition defect attributable to the display endpoint returns Story 7.2 to `review`

### Story 7.3: LibRaw embedded JPEG와 RAW+JPEG fast source 비교

Type: Enabler / Experiment

As a booth product team,
I want to compare the fastest reliable same-capture JPEG sources on the approved camera,
So that the display proxy starts from measured camera evidence rather than an assumption based on two files.

**Acceptance Criteria:**

**Given** EOS 700D captures on approved hardware
**When** the LibRaw embedded-JPEG route is exercised
**Then** at least 5 warm-up and 30 measured captures record source presence, dimensions, orientation, decode validity, corruption, request correlation, ready time, and extraction cost
**And** the route remains experimental until all measured captures and the representative quality corpus are reviewed

**Given** the camera reports its actual image-quality capability descriptor
**When** supported RAW+JPEG combinations are exercised
**Then** at least 5 warm-up and 30 measured captures use group/object correlation to pair every transfer object with one request
**And** all objects are downloaded or cancelled safely without assuming unsupported RAW+small combinations

**Given** both routes can be tested
**When** the team performs randomized AB/BA comparison
**Then** the report identifies p50, p95, max, success rate, source order, quality, and failure modes for each route
**And** selects a primary route, fallback route, or No-Go with explicit evidence

**Given** a source candidate becomes available
**When** product success is evaluated
**Then** the unfiltered source never counts as the qualifying preset-applied viewer frame
**And** wrong-session, wrong-request, wrong-capture, partial, corrupt, or stale sources are rejected without invalidating persisted RAW truth

**Given** implementation and automated tests are complete
**When** Story 7.3 is reviewed for closure
**Then** it remains in `review` until HV-14 records the complete source comparison and approved route decision

### Story 7.4: 화면 적합 immutable preset proxy 생성과 관람 화면 게시

As a booth customer,
I want the first successful viewer image to fill the approved photo area with my selected look,
So that I see a sharp and truthful preset result instead of a stretched thumbnail or unfiltered placeholder.

**Acceptance Criteria:**

**Given** an approved display profile and capture-bound published preset
**When** a display proxy job is created
**Then** its target dimensions come from the viewer photo rectangle and DPR and use the approved sRGB/JPEG quality profile
**And** the job is bound to one session, request, capture, preset identity/version, source hash, render profile, and unique generation

**Given** a published preset is considered for the proxy lane
**When** publication compatibility is evaluated
**Then** the artifact records `proxyCompatible`, supported operations, versioned recipe, reference renderer/version, and visual approval evidence
**And** unsupported or unapproved presets use the exact RAW fallback rather than an implicit approximation

**Given** a display-fit preset proxy completes
**When** it is published to the viewer
**Then** the process has exited or the resident job has completed, the full image decodes, physical dimensions are sufficient, provenance matches, and an immutable generation is atomically committed before notification
**And** a partially written file or in-place overwrite can never become active display truth

**Given** delayed, duplicated, or out-of-order updates
**When** the viewer reconciles channel updates with the latest snapshot
**Then** a lower revision, stale generation, older capture, different preset, or other session cannot replace the current display

**Given** the baseline proxy path is measured
**When** Story 7.4 is reviewed for closure
**Then** correctness and actual-present spans are recorded even if the current one-shot renderer still misses the final latency gate
**And** HV-15 must record Go for immutable publication, display fit, correlation, and zero-wrong-frame behavior

### Story 7.5: 상주 display renderer 검증 spike

Type: Architecture Spike

As a booth product team,
I want a working app-lifetime display renderer prototype for the approved presets,
So that we can decide from real latency and visual parity whether process startup can be removed from the customer display path.

**Acceptance Criteria:**

**Given** the three current approved presets and their versioned proxy recipes
**When** the spike initializes before capture
**Then** it pre-creates the rendering context, precompiles the approved shaders/effects/LUTs, and preallocates the required display-fit resources
**And** no shader compilation, preset XML traversal, or renderer process startup is allowed on the measured per-capture hot path unless recorded as a failure

**Given** an approved fast source
**When** the resident renderer produces a display-fit proxy
**Then** source-ready→actual-present latency, CPU/GPU usage, memory, driver state, and failure behavior are compared with the pinned darktable one-shot baseline
**And** unsupported operations route to the exact fallback without presenting an incorrect look

**Given** the approved preset and representative EOS 700D corpus
**When** visual parity is evaluated
**Then** automated color/detail metrics and blind transition review meet the approved thresholds
**And** the report preserves output pairs and cannot waive the visual gate to obtain a faster result

**Given** the WebGL2 candidate is unavailable, unstable, or visually incorrect
**When** the spike decision is made
**Then** the report may evaluate the bounded WIC/Direct2D/D3D11 alternative or record No-Go
**And** a No-Go candidate remains disabled in production while darktable exact fallback stays intact

**Given** implementation and evidence collection are complete
**When** Story 7.5 is reviewed for closure
**Then** HV-16 records a Go/No-Go adoption decision with raw latency, parity, stability, and fallback evidence
**And** a candidate No-Go may close the spike only when the evidence and approved exact fallback decision are complete
**And** production adoption still requires Story 7.6 while final release requires Story 7.10

### Story 7.6: RAW 정밀본 무중단 교체와 deadline scheduler

As a booth customer,
I want the first preset result to improve to the RAW-refined result without a distracting transition,
So that speed does not reduce the visual confidence of the final viewing experience.

**Acceptance Criteria:**

**Given** one current capture has proxy, RAW refinement, final, and maintenance work
**When** jobs are scheduled
**Then** priority is `P0 current displayFitPresetProxy → P1 current rawRefinedDisplay → P2 final/warm-up/history`
**And** the default current-capture renderer capacity is bounded, stale work is coalesced or cancelled, and child process trees cannot outlive cancellation

**Given** a qualifying preset proxy is currently visible
**When** the matching RAW-refined generation becomes available
**Then** it is fully written, decoded, dimension-validated, capture/preset-correlated, and confirmed as a higher approved tier before swap
**And** the current image remains visible until the opaque double-buffer replacement is ready

**Given** overlapping captures, preset changes, deletion, reload, listener loss, render failure, or a late older result
**When** display state is reconciled
**Then** only the latest valid generation for the capture-bound preset may advance the display pointer
**And** saved RAW, current viewer state, and exact fallback remain truthful and recoverable

**Given** proxy-first display through 500ms after RAW stabilization
**When** frame-level evidence is inspected
**Then** blank, spinner, prior/wrong capture, wrong preset, crop/scale jump, quality-tier downgrade, and stale overwrite are all zero

**Given** implementation and automated race/fault tests are complete
**When** Story 7.6 is reviewed for closure
**Then** it remains in `review` until HV-17 records Go with burst, failure, recovery, and frame-level seamless-swap evidence
**And** scheduler/capacity/cancellation/process-tree evidence is reviewed independently from seamless tier-transition/frame-integrity evidence
**And** both review gates must pass; one gate cannot inherit the result of the other

### Story 7.7: 완전한 installer와 clean offline 재현

As a owner / brand operator,
I want one verifiable installer that reproduces the complete viewing path on a clean offline machine,
So that branch installation truth is proven before performance or rollout approval.

**Acceptance Criteria:**

**Given** an approved release candidate
**When** its signed inventory is inspected
**Then** it includes the app, self-contained camera helper, approved EDSDK runtime, selected source adapter, display renderer or shader bundle, color profile, proxy recipes, and pinned darktable dependency with exact versions and hashes
**And** licensing, signing, and integrity evidence is attached

**Given** a clean offline Windows VM without Node, Rust, .NET SDK, or a separately installed renderer
**When** install, launch, self-check, fixture display, upgrade, rollback, and uninstall are exercised
**Then** the complete viewer/camera/proxy/RAW/final runtime is reproducible
**And** missing or mismatched inventory blocks release with an actionable operator result

**Given** Story 7.7 implementation and automated packaging tests are complete
**When** the story is reviewed for closure
**Then** it remains in `review` until HV-18A records Go for signed inventory, clean offline install lifecycle, and complete runtime reproduction

### Story 7.8: 100-shot 성능과 장애 복구 검증

As a owner / brand operator,
I want the fully installed release candidate to pass production-like performance and recovery evidence,
So that customer display speed and correctness are proven without hiding failures or exceptional conditions.

**Acceptance Criteria:**

**Given** approved EOS 700D, GPU, driver, USB, power, WebView2, monitor, DPR, ICC, and branch configuration
**When** 5 warm-up and 100 measured captures run
**Then** qualifying success is at least 99%, warm p50≤3.0 seconds, p95≤4.0 seconds, and hard max≤5.0 seconds with failures and timeouts retained
**And** wrong session/request/capture/preset/version, unfiltered qualifying, blank, stale, upscaled, crop/scale jump, and tier downgrade remain zero

**Given** cold start, 10-minute idle, camera disconnect/reconnect, burst, renderer failure, and app restart scenarios
**When** recovery evidence is collected
**Then** cold first-shot is reported separately, idle/reconnect p95 targets are evaluated, indeterminate physical captures are never auto-repeated, and the approved exact fallback remains available

**Given** the approved customer monitor and a 120fps+ external capture rig
**When** the display endpoint is validated on the fully installed release candidate
**Then** physical monitor frames must show zero blank, spinner, stale image, wrong capture, crop jump, scale jump, and tier downgrade across preset-applied and RAW-refined transitions
**And** the compositor-to-photon offset between software `actualPresent` and the physical frame is measured and applied to the reported KPI, with monitor refresh rate and vsync recorded
**And** this requirement is inherited from HV-13B by the 2026-08-12 correct-course; a defect attributable to the display endpoint returns Story 7.2 to `review`

**Given** Story 7.8 performance and recovery evidence is complete
**When** the story is reviewed for closure
**Then** it remains in `review` until HV-18B records Go for the full result set, quality/privacy integrity, approved recovery behavior, and the inherited physical-frame display endpoint evidence

### Story 7.9: 단계적 배포와 rollback 호환성 검증

As a owner / brand operator,
I want the approved release candidate to promote and roll back without damaging active or older sessions,
So that branch rollout remains recoverable and customer sessions keep their valid display truth.

**Acceptance Criteria:**

**Given** the release candidate passes lab validation
**When** rollout proceeds
**Then** modes advance `legacy → layered-shadow → layered-canary → layered-default` through lab, one pilot branch, 10~20%, and default stages
**And** one approved action can force the last approved fallback without interrupting an active session

**Given** pre-upgrade sessions, an active session, a prior approved build, and an older generation pointer exist
**When** upgrade, safe-point activation, forced fallback, and rollback are exercised
**Then** active sessions are never force-updated, pre-upgrade session data remains readable, and the last valid generation pointer remains recoverable
**And** rollout and rollback audit records identify branch set, build, preset stack, actor, decision, and compatibility result

**Given** Story 7.9 rollout and rollback evidence is complete
**When** the story is reviewed for closure
**Then** it remains in `review` until HV-18C records Go for staged promotion, active-session protection, rollback, and old-session compatibility

### Story 7.10: 최종 MVP 출시 판정

Type: Release Governance Gate

As a product manager,
I want one explicit final decision over every required implementation and hardware gate,
So that a partial installer, benchmark, or rollout pass cannot be mistaken for MVP release readiness.

**Acceptance Criteria:**

**Given** any required legacy or Epic 7 hardware gate is missing or No-Go
**When** release status is evaluated
**Then** Story 7.10, Epic 7, and the new MVP release remain `review`/No-Go
**And** the decision identifies the missing gate, owner, evidence path, and approved next action

**Given** an experimental source or renderer candidate recorded technology No-Go
**When** final release status is evaluated
**Then** release may proceed only if a separately approved production route and exact fallback satisfy every downstream correctness, performance, packaging, and rollback gate
**And** the rejected candidate remains disabled in production

**Given** all legacy truth-critical gates and HV-13A, HV-13B, HV-14, HV-15, HV-16, HV-17, HV-18A, HV-18B, and HV-18C have acceptable evidence
**When** the Product Manager, Architect, QA/Release, and Operations owners review the complete package
**Then** HV-18D records the canonical final Go/No-Go decision for the installed MVP experience
**And** only an HV-18D Go may move Epic 7 and the new MVP release to `done`/Go
**And** `UX-EV-01` confirms UX-DR16 accessibility evidence and `UX-EV-02` confirms real-booth touch, standing-use, high-contrast, and unguided-success evidence
