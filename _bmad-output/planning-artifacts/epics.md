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

### NonFunctional Requirements

NFR1: 고객용 주요 상태 화면은 동적 세션 값을 제외하고 기본 지시 문장 1개, 보조 문장 1개, 주요 액션 라벨 1개 이내의 문구 밀도를 유지해야 하며, 기술 진단어·darktable 용어·저작 도구 용어를 노출하면 안 된다.
NFR2: 모든 활성 지점은 승인된 프리셋 카탈로그, 게시 프리셋 버전, 고객용 타이밍 규칙, 핵심 부스 여정을 동일하게 유지해야 하며, 차이는 승인된 지역 설정으로만 제한되어야 한다.
NFR3: 주요 고객 액션은 1초 이내에 응답이 인지되어야 하며, 성공적으로 저장된 촬영의 현재 세션 프리뷰 확인은 승인된 Windows 하드웨어에서 95백분위 기준 5초 이내에 보여야 한다.
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
- SQLite는 라이프사이클, 개입, 게시, 롤아웃 감사 로그를 저장하되 사진 또는 세션 자산의 원본 진실을 소유하면 안 된다.
- 지점별 최소 설정과 런타임 플래그만 로컬 설정 저장소에 보관해야 하며, 지점 차이는 승인된 로컬 설정으로 한정되어야 한다.
- 운영자 및 프리셋 저작 기능은 관리자 비밀번호 인증과 capability check 통과 후에만 노출되어야 한다.
- 프론트엔드와 호스트 사이 계약은 TypeScript의 `Zod 4` 검증과 Rust 재검증을 함께 사용해야 한다.
- React Router는 `/booth`, `/operator`, `/authoring`, `/settings` 같은 최상위 surface 중심으로 제한해야 한다.
- UI 컴포넌트는 직접 `invoke` 호출을 하지 않고, 타입이 지정된 adapter/service 계층을 통해서만 호스트 기능에 접근해야 한다.
- 다음 계약 산출물은 구현 전제 조건으로 동결돼야 한다: `session.json` 스키마, preset bundle 스키마, sidecar protocol 메시지, authoring publication payload 계약.
- 타이밍 정책, 경고/종료 알림, 사후 상태 전환, 강제 업데이트 금지, 단계적 배포/롤백 규칙은 호스트 소유 워크플로 규칙으로 구현되어야 한다.
- Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3은 자동 테스트 통과만으로 제품 관점 `done`으로 간주하지 않는다.
- 지정된 booth hardware validation checklist evidence가 수집되기 전까지 해당 story는 `review` 또는 동등한 pre-close 상태에 머물러야 한다.
- booth `Ready`와 `Completed`는 각각 false-ready, false-complete 방지 evidence가 확보된 뒤에만 release truth로 인정한다.

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
Operational NFRs: Epic 6 - 지점별 단계적 배포와 단일 액션 롤백 거버넌스

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

**Acceptance Criteria:**

**Given** an active session and selected preset
**When** live Tauri host receives real camera/helper readiness changes on approved booth hardware
**Then** the booth translates runtime truth into plain-language customer states such as `Preparing`, `Ready`, or wait/call guidance
**And** the booth shows `Ready` only when both the capture boundary and helper boundary are actually ready
**And** customer copy stays within the approved low-density guidance rule

**Given** the booth is not in an approved capture state
**When** the customer attempts to capture
**Then** capture is blocked
**And** the booth tells the customer whether to wait or call without exposing technical diagnostics

**Given** the booth loses camera or helper readiness after previously being ready
**When** the live capture boundary degrades or disconnects
**Then** the booth immediately exits `Ready`
**And** the `사진 찍기` action becomes disabled without waiting for browser fallback or stale readiness refresh

**Given** Story 1.4 implementation and automated tests are complete
**When** the team evaluates done status
**Then** the story remains in `review` until HV-02, HV-03, and HV-10 evidence is collected on approved booth hardware

### Story 1.5: 현재 세션 촬영 저장과 truthful preview waiting 피드백

As a booth customer,
I want capture success and preview readiness to be communicated separately,
So that I know my photo is saved even if the confirmation preview is still being prepared.

**Acceptance Criteria:**

**Given** the booth is in a valid capture state with an active preset
**When** the customer captures a photo successfully
**Then** the new source photo is persisted to the active session before success feedback is shown
**And** the active preset remains visible on the capture or confirmation surface

**Given** a successful capture whose customer-safe preview is not yet ready
**When** the booth enters `Preview Waiting`
**Then** the first message confirms the photo was saved
**And** the next message explains that the confirmation preview is being prepared and what the customer can do next

**Given** a successful capture is acknowledged
**When** the booth reports the immediate outcome on approved hardware
**Then** the primary customer action is acknowledged within 1 second
**And** the current-session preview confirmation is shown within 5 seconds for the 95th percentile of successful captures or the booth remains in truthful `Preview Waiting` until the preview is ready

**Given** the booth is in `Preview Waiting`
**When** the preview rail is still empty
**Then** the UI explains that this can be normal for the current session
**And** no internal render failure cause is shown to the customer

**Given** Story 1.5 implementation and automated tests are complete
**When** the team evaluates done status
**Then** the story remains in `review` until HV-04 and HV-05 evidence confirms persisted RAW truth and truthful preview readiness on approved booth hardware

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
**When** the host sends `request-capture` to the bundled helper
**Then** the helper accepts or rejects one correlated in-flight capture request
**And** the host keeps a single in-flight capture guard for that request

**Given** the helper accepts a capture
**When** shutter trigger, RAW download, and final file handoff are still in progress
**Then** the booth does not treat `capture-accepted` as capture success
**And** success is confirmed only after correlated `file-arrived` and actual file presence are verified

**Given** duplicate arrival, wrong session correlation, missing file, timeout, or a second capture during an in-flight capture
**When** capture resolution is evaluated
**Then** the host blocks false success and unsafe parallel capture
**And** the booth falls back to truthful wait or recovery guidance without cross-session leakage

**Given** Story 1.7 is reviewed for closure
**When** real capture round-trip evidence is missing on approved booth hardware
**Then** the story remains open
**And** it does not inherit closure from Story 1.6 or synthetic preview flow

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

### Story 5.3: 라이프사이클, 개입, 복구 감사 로그 기록

As a owner / operations lead,
I want lifecycle and intervention events to be recorded consistently,
So that we can audit failures, recovery behavior, timing outcomes, and support burden across branches.

**Acceptance Criteria:**

**Given** a session lifecycle transition, operator intervention, publication-related recovery event, or critical failure occurs
**When** the event is finalized by the host
**Then** the system records it in the audit log with timestamp, actor or source, session reference, and event type
**And** the log is queryable by operators for support and retrospective review

**Given** audit data is reviewed
**When** operators or owners inspect session history
**Then** they can distinguish state transitions, intervention attempts, and final outcomes
**And** the log remains separate from durable photo/session asset truth

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

### Story 6.2: 실장비 hardware validation gate와 evidence 기반 done 정책

As a owner / brand operator,
I want sprint closure to require hardware validation evidence for truth-critical stories,
So that implementation completion is not mistaken for product readiness.

**Acceptance Criteria:**

**Given** Story 1.4, 1.5, 1.6, 3.2, 4.2, or 4.3 has completed implementation and automated tests
**When** the team evaluates story closure
**Then** the story does not move to product-level `done` until the mapped hardware validation evidence is attached
**And** the story remains in `review` or an equivalent pre-close state until then

**Given** a truth-critical story is reviewed for hardware validation
**When** the team records closure evidence
**Then** the story references the exact checklist IDs, evidence location, execution date, and Go / No-Go result
**And** sprint review distinguishes automated pass from hardware pass

**Given** a hardware validation scenario results in `No-Go`
**When** the sprint status is updated
**Then** the impacted story remains or returns to `review`
**And** the release decision cannot claim booth `Ready` or `Completed` truth without the missing evidence
