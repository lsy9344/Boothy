---
stepsCompleted:
  - 'step-01-validate-prerequisites'
  - 'step-02-design-epics'
  - 'step-03-create-stories'
  - 'step-04-final-validation'
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/architecture.md'
  - '_bmad-output/planning-artifacts/ux-design-specification.md'
  - '_bmad-output/planning-artifacts/research/technical-boothy-preview-architecture-alternatives-research-20260414.md'
  - '_bmad-output/planning-artifacts/preview-architecture-reassessment-report-20260414.md'
---

# Boothy - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Boothy, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: 고객은 이름과 휴대전화 뒤 4자리만 입력해서 현재 부스 세션을 시작할 수 있어야 하며, 전체 전화번호 입력이나 예약 검증 없이 준비 또는 촬영 가능 상태로 진입할 수 있어야 한다.
FR2: 고객은 1~6개의 승인된 게시 프리셋만 볼 수 있어야 하며, 각 프리셋의 이름과 대표 프리뷰 타일 또는 샘플컷을 바탕으로 하나의 활성 프리셋을 선택할 수 있어야 한다.
FR3: 고객은 부스가 `Preparing`, `Ready`, `Preview Waiting`, `Export Waiting`, `Phone Required` 중 어떤 상태인지 평이한 언어로 이해할 수 있어야 하며, 허용된 상태에서만 촬영할 수 있어야 한다.
FR4: 고객은 현재 세션에 사진을 촬영해 안전하게 저장할 수 있어야 하며, 프리뷰 준비가 아직 끝나지 않았더라도 저장 성공과 프리뷰 준비 상태를 구분해서 안내받아야 한다.
FR5: 고객은 현재 세션의 사진만 검토할 수 있어야 하고, `Current-Session Deletion Policy`가 허용하는 범위에서만 삭제할 수 있어야 하며, 프리셋은 세션 중 언제든 변경할 수 있어야 한다. 변경은 이후 촬영부터 반영되고 이미 저장된 촬영본은 유지되어야 한다.
FR6: 고객은 세션 시작 시점부터 조정된 종료 시각을 확인할 수 있어야 하며, 5분 전 경고와 종료 시각 알림을 통해 남은 촬영 가능 여부와 종료 후 행동을 명확히 안내받아야 한다.
FR7: 고객은 촬영 종료 후 `Export Waiting`, `Completed`, `Phone Required` 중 하나의 명시적 상태로 진입해야 하며, 완료 또는 인계 준비 상태를 기술 진단어 없이 이해할 수 있어야 한다.
FR8: 승인된 내부 프리셋 관리자는 드래프트 프리셋을 작성, 검증, 승인, 게시, 롤백할 수 있어야 하며, 게시된 불변 프리셋 아티팩트만 미래 세션 카탈로그에 반영되어야 한다.
FR9: 운영자는 현재 세션 문맥, 실패 상태, 허용된 복구 액션, 라이프사이클 로그를 확인할 수 있어야 하며, `Operator Recovery Policy` 범위 안에서만 복구를 수행할 수 있어야 한다.

### NonFunctional Requirements

NFR1: 고객용 주요 상태 화면은 동적 세션 값을 제외하고 기본 지시 문장 1개, 보조 문장 1개, 주요 액션 라벨 1개 이내의 문구 밀도를 유지해야 하며, 기술 진단어, darktable 용어, 저작 도구 용어를 노출하면 안 된다.
NFR2: 모든 활성 지점은 승인된 프리셋 카탈로그, 게시 프리셋 버전, 고객용 타이밍 규칙, 핵심 부스 여정을 동일하게 유지해야 하며, 차이는 승인된 지역 설정으로만 제한되어야 한다.
NFR3: 주요 고객 액션은 1초 이내에 응답이 인지되어야 하며, first-visible 이미지, preset-applied preview readiness, `same-capture preset-applied full-screen visible`이 서로 분리 계측되어야 한다. 승인된 Windows 하드웨어에서 release sign-off 기준은 `same-capture preset-applied full-screen visible <= 2500ms`이고, 5초 이내 preview confirmation은 보조 운영 가드레일이어야 한다.
NFR4: 소스 캡처, 프리뷰, 최종 결과물, 검토, 삭제, 완료 흐름 전반에서 교차 세션 자산 누출은 0건이어야 하며, 저장되는 고객 식별 정보는 승인된 최소 범위로 제한되어야 한다.
NFR5: 5분 전 경고와 종료 시각 알림은 99% 세션에서 허용 오차 내에 동작해야 하고, 세션의 90% 이상은 종료 시각 10초 내에 명시적 사후 상태로 진입해야 하며, 렌더 재시도나 실패가 이미 저장된 유효 촬영을 훼손하면 안 된다.
NFR6: 제품은 선택된 지점 집합에 대한 단계적 배포와 단일 승인 액션 기반 롤백을 지원해야 하며, 활성 고객 세션 중 강제 업데이트는 0건이어야 하고, 승인된 프리셋 카탈로그의 렌더 호환성이 유지되어야 한다.

### Additional Requirements

- Epic 1의 시작부에는 공식 `Vite react-ts` + 수동 `Tauri CLI` 초기화 기반의 프로젝트 부트스트랩이 포함되어야 한다.
- 제품은 하나의 패키지된 Tauri 애플리케이션 안에서 고객 부스, 운영자 콘솔, 내부 프리셋 저작 화면의 3개 capability-gated surface를 제공해야 한다.
- 활성 세션의 내구적 진실은 UI 메모리나 SQLite가 아니라 세션 단위 파일시스템 루트와 `session.json` 매니페스트가 소유해야 한다.
- 고객용 부스 별칭은 이름+휴대전화 뒤4자리 조합으로 유지하되, 내구적 내부 식별자인 `sessionId`와 분리되어야 한다.
- Rust 호스트는 카메라 상태, 타이밍 상태, 프리뷰/완료 상태를 정규화하는 단일 진실 계층이어야 하며, React는 정규화된 상태만 소비해야 한다.
- 카메라 연동은 번들된 Windows Canon EDSDK helper sidecar 경계 뒤에 격리되어야 하며, 버전드 JSON-line 메시지와 파일시스템 핸드오프로 통신해야 한다.
- 첫 카메라 구현 프로파일은 Windows 전용 Canon EDSDK helper를 기준으로 삼아야 하며, 범용 멀티벤더 추상화는 후행 검증 이후로 미뤄야 한다.
- canonical preset recipe는 booth runtime, GPU lane, darktable fallback/oracle이 함께 참조하는 공통 진실이어야 한다.
- darktable 기반 프리셋 아티팩트와 `darktable-cli` 경로는 baseline / fallback / parity oracle / final-export reference 역할을 유지해야 하며, 고객에게는 일반 편집기나 darktable 내부 개념이 노출되면 안 된다.
- 프리셋은 불변 게시 번들로 저장되어야 하고, 활성 세션은 정확한 프리셋 버전을 참조해야 하며, 게시/롤백은 미래 세션에만 영향을 줘야 한다.
- SQLite는 라이프사이클, 타이밍, 개입, 게시, 롤아웃 감사 로그를 저장하되 사진 또는 세션 자산의 원본 진실을 소유하면 안 된다.
- 운영자 및 프리셋 저작 기능은 관리자 비밀번호 인증과 capability check 통과 후에만 노출되어야 한다.
- 프론트엔드와 호스트 사이 계약은 TypeScript의 `Zod 4` 검증과 Rust 재검증을 함께 사용해야 한다.
- React Router는 `/booth`, `/operator`, `/authoring`, `/settings` 같은 최상위 surface 중심으로 제한해야 한다.
- UI 컴포넌트는 직접 Tauri `invoke`를 호출하지 않고, 타입이 지정된 adapter/service 계층을 통해서만 호스트 기능에 접근해야 한다.
- preview/render 핵심 구조는 `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + darktable-compatible truth/parity reference`를 기준 아키텍처로 채택해야 한다.
- preset-applied truthful full-screen close owner는 display-sized truthful artifact를 만드는 host-owned local native/GPU resident lane이어야 하며, darktable-compatible path는 parity/fallback/final reference로 유지되고, first-visible source는 customer-safe projection으로 남되 `previewReady` truth owner가 되면 안 된다.
- latest-photo canonical path는 same-capture first-visible에서 시작해, 나중에 preset-applied truthful preview가 준비되면 같은 슬롯에서 자연스럽게 교체되어야 한다.
- preview architecture의 즉시 목표는 실장비 기준 `same-capture preset-applied full-screen visible <= 2500ms`를 달성하는 구조를 입증하는 것이며, same-capture 보장, preset fidelity, preview/final truth drift 0을 함께 만족해야 한다.
- 구현 전 동결되어야 하는 계약 산출물은 `session.json` 스키마, preset bundle 스키마, sidecar protocol 메시지, Canon helper profile, authoring publication payload이다.
- hardware validation은 독립 사용자 가치 스토리가 아니라 truth-critical 기능 전반에 적용되는 공통 release truth gate로 다뤄져야 한다.
- Epic 1에는 공유 계약 동결, Canon helper/profile 및 publication contract 확정, Windows desktop build-release baseline과 CI proof를 위한 선행 foundational story가 추가되어야 한다.
- Epic 1에는 local native/GPU resident full-screen lane, display-sized preset-applied truthful artifact, darktable-compatible truth/parity reference, `same-capture preset-applied full-screen visible <= 2500ms` 승격 게이트, route promotion/rollback authority, 그리고 reserve-only remote renderer decision gate를 위한 follow-up story가 추가되어야 한다.
- 타이밍 정책, 경고/종료 알림, 사후 상태 전환, 강제 업데이트 금지, 단계적 배포/롤백 규칙은 호스트 소유 워크플로 규칙으로 구현되어야 한다.

### UX Design Requirements

UX-DR1: 고객 기본 흐름은 booth-first, preset-driven 구조를 유지해야 하며, 고객에게 세부 조정 화면, darktable 용어, 내부 제작 도구를 노출하면 안 된다.
UX-DR2: 세션 시작 화면은 이름과 휴대전화 뒤4자리 두 입력만 요구해야 하며, 잘못된 형식이나 빈 값은 즉시 검증해 다음 진행 전에 분명히 안내해야 한다.
UX-DR3: 고객 화면은 현재 활성 프리셋과 방금 찍은 current-capture 결과를 가장 먼저 크게 보여주고, 현재 세션 범위의 이전 사진은 보조 히스토리로만 이해할 수 있게 보여줘야 한다.
UX-DR4: 조정된 종료 시각은 세션 시작부터 명확히 보여야 하며, 5분 전 경고와 종료 후 다음 행동은 plain-language 고객 안전 문구로 전달되어야 한다.
UX-DR5: 고객용 문구는 낮은 문구 밀도 원칙을 따라야 하며, 기술 진단어, 내부 운영 용어, 원인 분석형 오류 설명을 포함하면 안 된다.
UX-DR6: `Preview Waiting` 화면은 첫 문장에서 사진 저장 완료 사실을 먼저 말하고, 둘째 문장에서 확인용 사진 준비 중임을 설명하며, 현재 가능한 다음 행동을 함께 제시해야 한다.
UX-DR7: `Preview Waiting` 상태에서는 current-capture spotlight에 same-capture first-visible이 먼저 보이더라도 상태 copy는 truthful preset-applied preview가 준비될 때까지 `Preview Waiting`을 유지해야 하며, 세션 히스토리가 비어 있어도 정상임을 보조 문구로 알려야 한다.
UX-DR8: current-capture spotlight은 가능한 경우 same-capture first-visible 이미지를 먼저 크게 보여주고, 이후 booth-safe preset-applied truthful preview가 준비되면 같은 자리에서 자연스럽게 교체되어야 한다. 세션 히스토리는 이전 컷 보조 레일로만 남겨야 한다.
UX-DR9: `Phone Required` 화면은 도움 요청 중심의 보호 화면이어야 하며, 현재 세션 보존 여부 설명, 단일 연락 액션, 고객이 하지 말아야 할 행동을 짧게 포함해야 한다.
UX-DR10: 운영자/내부 프리셋 관리 진입점은 고객 기본 흐름에서 숨겨져 있어야 하며, 관리자 비밀번호 인증 이전에는 시각적으로도 노출되지 않아야 한다.
UX-DR11: 부스 UI는 Warm Brutalist / Brutal Core 방향의 고대비 스타일을 유지해야 하며, Warm Beige, Bold Black, Clay Orange와 시맨틱 경고색을 일관되게 사용해야 한다.
UX-DR12: 타이포그래피는 `Pretendard Variable` 중심의 강한 위계를 유지해야 하며, 멀리서도 읽히는 헤드라인과 명확한 행동 우선순위를 제공해야 한다.
UX-DR13: 모든 핵심 버튼과 입력은 서서 조작하는 대형 터치스크린 환경을 기준으로 넉넉한 터치 영역을 가져야 하며, `80x80px`은 초기 설계 기준선으로 검토되어야 한다.
UX-DR14: 프리셋 카탈로그는 큰 프리셋 카드 컴포넌트로 구현되어야 하며, 각 카드는 예시 이미지, 룩 이름, 선택 상태 강조를 포함해야 한다.
UX-DR15: 시간 안내 컴포넌트는 디지털 타이머와 상태별 시각 강조를 제공해야 하며, 5분 전과 종료 시점에 사운드 알림과 함께 동작해야 한다.
UX-DR16: `Preview Waiting Panel`과 `Phone Required Support Card`는 별도 재사용 컴포넌트로 설계되어, 고객 보호 메시지 위계와 단일 행동 원칙을 일관되게 유지해야 한다.
UX-DR17: 핵심 상태 변화는 시각 배지와 브랜드 사운드를 함께 사용해야 하며, 특히 경고, 종료, 에스컬레이션 상태는 공포를 키우지 않는 안정적 위계로 설계해야 한다.
UX-DR18: 부스 메인 화면은 1024px 이상 대형 터치스크린 기준으로 최적화하고, 운영자 화면은 768px 이상 데스크톱급 범위를 지원하되, MVP에서는 고객용 모바일 화면을 만들지 않아야 한다.
UX-DR19: 접근성 목표는 WCAG 2.2 AA 수준으로 두어야 하며, 고대비, 명확한 포커스 표시, 멀티모달 알림, 초보 사용자 무가이드 사용성을 함께 검증해야 한다.

### FR Coverage Map

FR1: Epic 1 - 세션 시작 입력과 고객용 booth alias 생성
FR2: Epic 1 - 승인 프리셋 카탈로그와 활성 프리셋 선택
FR3: Epic 1 - 준비 상태 안내와 유효 상태에서만 촬영 허용
FR4: Epic 1 - 현재 세션 저장, `Preview Waiting`, truthful preview 전환
FR5: Epic 2 - 현재 세션 검토, 삭제, 세션 중 프리셋 변경
FR6: Epic 2 - 조정된 종료 시각, 5분 경고, 종료 시각 알림
FR7: Epic 3 - 종료 후 명시적 상태, 완료, 인계, 도움 요청 안내
FR8: Epic 4 - 내부 프리셋 작성, 검증, 승인, 게시, 롤백
FR9: Epic 5 - 운영자 진단, 복구, 감사 로그

## Epic List

### Epic 1: 빠른 세션 시작과 신뢰 가능한 첫 촬영
고객이 최소 입력으로 세션을 시작하고, 승인된 프리셋을 고르고, 촬영 가능 상태를 이해한 뒤, 첫 촬영과 `Preview Waiting`까지 신뢰할 수 있게 한다.
**FRs covered:** FR1, FR2, FR3, FR4
**Status:** in-progress

### Epic 2: 현재 세션 중심의 촬영 제어와 시간 인지
고객이 현재 세션 사진만 검토하고 삭제하며, 세션 중 프리셋을 바꾸고, 남은 시간을 이해하면서 촬영을 이어갈 수 있게 한다.
**FRs covered:** FR5, FR6
**Status:** done

### Epic 3: 종료 후 결과 준비와 인계의 진실한 마무리
고객이 촬영 종료 후 `Export Waiting`, `Completed`, `Phone Required` 중 현재 상태를 정확히 이해하고 다음 행동을 혼란 없이 수행할 수 있게 한다.
**FRs covered:** FR7
**Status:** done

### Epic 4: 승인 기반 프리셋 게시와 미래 세션 카탈로그 운영
내부 프리셋 관리자가 프리셋을 작성, 검증, 승인, 게시, 롤백하여 미래 세션용 카탈로그를 안전하게 운영할 수 있게 한다.
**FRs covered:** FR8
**Status:** in-progress

### Epic 5: 운영자 복구와 감사 로그
운영자가 현재 세션 문제를 안전한 범위에서 진단하고 복구하며, 개입 결과를 감사 가능하게 남길 수 있게 한다.
**FRs covered:** FR9
**Status:** in-progress

### Epic 6: 지점 배포와 롤백 거버넌스
브랜드/운영 측이 선택된 지점 집합에 대해 빌드와 승인된 프리셋 스택을 안전하게 배포하고 롤백할 수 있게 한다.
**Primary coverage:** NFR2, NFR6
**Status:** done

<!-- Repeat for each epic in epics_list (N = 1, 2, 3...) -->

## Epic 1: 빠른 세션 시작과 신뢰 가능한 첫 촬영

고객이 최소 입력으로 세션을 시작하고, 승인된 프리셋을 고르고, 촬영 가능 상태를 이해한 뒤, 첫 촬영과 `Preview Waiting`까지 신뢰할 수 있게 한다.

**Status:** in-progress

### Story 1.1: 초기 프로젝트 부트스트랩과 Tauri 표준 시작점 구성

**Status:** done

As a owner / brand operator,
I want 하나의 패키지 안에 booth, operator, authoring surface가 분리된 초기 런타임 골격을 갖추고 싶다,
So that 고객용 가치 개발을 안전한 제품 경계 위에서 시작할 수 있다.

**Acceptance Criteria:**

**Given** 새로운 MVP 런타임을 초기화할 때
**When** 프로젝트 부트스트랩을 수행하면
**Then** 공식 `Vite react-ts` + 수동 `Tauri CLI` 초기화 기반으로 앱이 구성되어야 한다
**And** `/booth`, `/operator`, `/authoring`, `/settings` 최상위 surface 경계가 정의되어야 한다

**Given** 기본 앱 실행 상태
**When** 고객이 부스를 열면
**Then** 기본 진입은 `/booth`로 한정되어야 한다
**And** operator 또는 authoring 진입점은 관리자 인증 이전에 노출되지 않아야 한다

**Given** 프론트엔드가 호스트 기능을 호출할 때
**When** 세션, 프리셋, 촬영 기능을 연결하면
**Then** UI 컴포넌트는 직접 Tauri `invoke`를 호출하지 않아야 한다
**And** 타입이 지정된 adapter/service 계층을 통해서만 접근해야 한다

### Story 1.2: 이름과 뒤4자리 기반 세션 시작

**Status:** done

As a booth customer,
I want 이름과 휴대전화 뒤4자리만으로 세션을 시작하고 싶다,
So that 예약 확인이나 전체 번호 입력 없이 바로 촬영 준비를 시작할 수 있다.

**Acceptance Criteria:**

**Given** 세션 시작 화면
**When** 고객이 비어있지 않은 이름과 정확한 네 자리 숫자를 입력하면
**Then** 고객용 booth alias가 생성되어야 한다
**And** 내부적으로는 별도의 `sessionId`가 발급되어야 한다

**Given** 유효한 입력이 제출되면
**When** 세션 생성이 완료되면
**Then** 세션 단위 파일시스템 루트와 초기 `session.json` 매니페스트가 생성되어야 한다
**And** 다음 준비 또는 프리셋 선택 흐름으로 이동할 수 있어야 한다

**Given** 이름이 비어 있거나 뒤4자리가 잘못된 형식이면
**When** 고객이 다음 단계로 진행하려고 하면
**Then** 진행이 차단되어야 한다
**And** 전체 전화번호나 예약 검증을 요구하지 않는 plain-language 검증 문구가 보여야 한다

### Story 1.3: 승인된 프리셋 카탈로그와 활성 프리셋 선택

**Status:** done

As a booth customer,
I want 승인된 프리셋 중 하나를 직관적으로 선택하고 싶다,
So that 촬영 전에 내가 얻게 될 룩을 자신 있게 이해할 수 있다.

**Acceptance Criteria:**

**Given** 활성 세션이 존재할 때
**When** 고객이 프리셋 선택 화면에 진입하면
**Then** 1개에서 6개 사이의 승인된 게시 프리셋만 노출되어야 한다
**And** 각 프리셋은 이름과 대표 타일 또는 샘플컷을 포함해야 한다

**Given** 고객이 프리셋 카드를 선택하면
**When** 선택이 반영되면
**Then** 해당 프리셋이 활성 프리셋으로 저장되어야 한다
**And** 이후 촬영에 적용될 게시 버전 정보가 세션에 연결되어야 한다

**Given** 프리셋 카탈로그가 보일 때
**When** 고객이 선택을 수행하면
**Then** 직접 편집 도구, darktable 용어, 내부 제작 개념은 노출되지 않아야 한다
**And** 큰 터치 타겟과 명확한 선택 강조가 유지되어야 한다

### Story 1.4: 준비 상태 안내와 유효 상태에서만 촬영 허용

**Status:** done

As a booth customer,
I want 지금 촬영 가능한지 기다려야 하는지를 분명히 알고 싶다,
So that 장비 내부 상태를 몰라도 부스를 믿고 사용할 수 있다.

**Acceptance Criteria:**

**Given** 활성 세션과 활성 프리셋이 준비된 상태
**When** 카메라 helper 상태와 호스트 정규화 상태가 갱신되면
**Then** 고객 화면은 `Preparing`, `Ready`, 대기 또는 도움 요청 상태를 plain-language로 보여야 한다
**And** 고객 문구는 낮은 문구 밀도 원칙을 지켜야 한다

**Given** 촬영 경계나 helper 경계가 실제로 준비되지 않은 상태
**When** 고객이 촬영 버튼을 누르면
**Then** 촬영은 차단되어야 한다
**And** 고객에게는 기다리기 또는 도움 요청 중 하나의 안전한 다음 행동만 보여야 한다

**Given** 이전에 `Ready`였던 세션이 준비 상태를 잃으면
**When** 카메라 연결 또는 helper 신선 상태가 저하되면
**Then** 부스는 즉시 `Ready`에서 빠져나와야 한다
**And** 오래된 준비 상태를 유지한 채 촬영을 허용하면 안 된다

### Story 1.5: 현재 세션 촬영 저장과 truthful preview waiting 피드백

**Status:** done

As a booth customer,
I want 촬영 저장 성공과 확인 사진 준비 상태를 분리해서 알고 싶다,
So that 프리뷰가 늦어도 방금 찍은 사진이 안전하게 저장됐다는 사실로 바로 안심할 수 있다.

**Acceptance Criteria:**

**Given** 부스가 유효한 촬영 상태에 있을 때
**When** 고객이 촬영에 성공하면
**Then** 새 원본 사진은 현재 세션 아래에 안전하게 저장되어야 한다
**And** 저장 성공 안내는 프리뷰 준비 완료보다 먼저 보여도 된다

**Given** 저장은 완료됐지만 preset-applied preview가 아직 준비되지 않았을 때
**When** 화면이 `Preview Waiting` 상태가 되면
**Then** 첫 문장은 저장 완료 사실을 먼저 알려야 한다
**And** 둘째 문장은 확인용 사진을 준비 중이라는 사실과 지금 가능한 다음 행동을 알려야 한다
**And** same-capture first-visible가 먼저 보여도 상태 copy와 성공 해석은 truthful preset-applied preview가 준비될 때까지 `Preview Waiting`에 머물러야 한다

**Given** `Preview Waiting` 상태
**When** latest photo rail이 아직 비어 있으면
**Then** 현재 세션에서는 정상일 수 있다는 보조 문구가 보여야 한다
**And** 내부 렌더 원인이나 기술 진단어는 노출되지 않아야 한다

### Story 1.6: 실카메라 helper readiness truth 연결과 false-ready 차단

**Status:** done

As a booth customer,
I want 실제 helper와 카메라가 신선한 준비 상태를 보냈을 때만 `Ready`가 열리길 원한다,
So that 오래된 상태나 가짜 준비 신호를 믿고 촬영하지 않게 된다.

**Acceptance Criteria:**

**Given** 승인된 booth hardware 환경
**When** 번들된 helper가 실행되고 host가 fresh `helper-ready`와 `camera-status`를 받으면
**Then** booth는 fresh camera-ready truth가 확인된 뒤에만 `Ready`로 들어갈 수 있어야 한다
**And** `helper-ready`만으로는 촬영 가능 상태가 열리면 안 된다

**Given** browser preview, fixture mode, stale readiness, disconnected 또는 degraded camera/helper 상태
**When** readiness를 평가하면
**Then** booth는 `Ready`를 주장하면 안 된다
**And** 촬영은 plain-language wait 또는 call guidance와 함께 계속 차단되어야 한다

**Given** 이전에 `Ready`였던 세션
**When** helper가 종료되거나 카메라 연결이 끊기거나 readiness가 degrade되면
**Then** booth는 즉시 `Ready`를 벗어나야 한다
**And** fresh `camera-status` truth를 다시 보기 전까지 자동 복귀하면 안 된다

### Story 1.7: 실카메라 capture round-trip과 raw handoff correlation

**Status:** done

As a booth customer,
I want 실제 촬영이 내 세션으로 올바른 파일이 돌아왔을 때만 끝났다고 믿고 싶다,
So that 셔터 수락이나 중간 handoff를 저장 성공으로 오해하지 않는다.

**Acceptance Criteria:**

**Given** 승인된 booth hardware의 fresh `Ready` 상태
**When** 고객이 booth 앱의 `사진 찍기` 버튼으로 촬영을 시작하면
**Then** host와 helper는 단일 in-flight capture correlation을 유지해야 한다
**And** second capture나 stale request가 parallel로 열리면 안 된다

**Given** helper가 `capture-accepted`를 보냈더라도
**When** correlated `file-arrived`와 실제 RAW 파일 존재가 active session root 아래에서 확인되기 전이면
**Then** host는 `capture-saved`를 확정하면 안 된다
**And** capture record는 `sessionId`, `requestId`, `captureId`, `activePresetVersion`, RAW asset path를 함께 남겨야 한다

**Given** wrong session correlation, missing file, partial file, timeout, duplicate arrival 또는 in-flight 중 second capture 시도
**When** host가 결과를 정규화하면
**Then** false success와 cross-session leakage가 차단되어야 한다
**And** booth는 customer-safe wait 또는 recovery guidance만 보여야 한다

### Story 1.8: 게시된 프리셋 XMP 적용 preview/final render worker 연결

**Status:** done

As a booth customer,
I want 내가 고른 승인된 프리셋이 preview와 final render에 같은 기준으로 반영되길 원한다,
So that 화면에서 보는 룩과 실제 결과의 기준이 흔들리지 않는다.

**Acceptance Criteria:**

**Given** 활성 세션에 승인된 게시 프리셋 버전이 바인딩된 상태
**When** capture 저장 후 render 작업이 시작되면
**Then** preview와 final worker는 같은 published preset version과 XMP-compatible artifact reference를 사용해야 한다
**And** draft 또는 미승인 프리셋 자산은 booth render truth에 사용되면 안 된다

**Given** render worker가 preset artifact를 읽을 때
**When** bundle을 검증하면
**Then** booth-safe preview behavior와 final render behavior가 같은 publication contract family 안에서 연결되어야 한다
**And** 세션은 어떤 preset version을 사용했는지 capture record와 manifest에 남겨야 한다

**Given** published artifact가 누락되거나 호환성이 깨지면
**When** render를 시도하면
**Then** booth는 false complete나 preview success를 주장하면 안 된다
**And** truthful waiting 또는 안전한 fallback 경로를 유지해야 한다

### Story 1.9: fast preview handoff와 XMP preview 교체

**Status:** done

As a booth customer,
I want 같은 컷의 빠른 확인 이미지를 먼저 보고 나중에 더 정확한 preset-applied preview로 자연스럽게 이어지길 원한다,
So that 체감 속도와 결과 신뢰를 둘 다 얻을 수 있다.

**Acceptance Criteria:**

**Given** same-capture first-visible 이미지가 더 빨리 준비될 수 있는 환경
**When** RAW 저장이 끝나고 booth가 고객 확인 이미지를 보여주면
**Then** fast preview는 same-capture 기준으로 먼저 보일 수 있어야 한다
**And** booth 상태 copy는 truthful preset-applied preview가 준비될 때까지 `Preview Waiting`을 유지해야 한다

**Given** later preset-applied preview가 준비되면
**When** latest photo 또는 current-capture 슬롯이 갱신되면
**Then** 같은 슬롯에서 자연스럽게 교체되어야 한다
**And** 고객이 다른 컷으로 오해할 정도의 불연속성이 생기면 안 된다

**Given** fast preview handoff가 실패하거나 지연되면
**When** booth가 fallback을 선택하면
**Then** fast path 자체를 성공으로 간주하면 안 된다
**And** current-session isolation과 truthful waiting은 계속 유지되어야 한다

### Story 1.10: known-good preview lane 복구와 상주형 first-visible worker 도입

**Status:** done

As a owner / brand operator,
I want known-good first-visible lane를 상주형 worker로 안정화하고 싶다,
So that same-capture early visibility를 반복 가능하게 확보할 수 있다.

**Acceptance Criteria:**

**Given** approved booth hardware와 capture-bound input이 있을 때
**When** resident first-visible worker를 실행하면
**Then** same-capture 기준의 booth-safe first-visible artifact를 생성할 수 있어야 한다
**And** cold start와 warm hit 상태를 구분해 기록할 수 있어야 한다

**Given** first-visible lane가 손상되거나 준비 상태를 잃으면
**When** booth가 capture 이후 초기 가시성을 계산하면
**Then** known-good lane 복구 여부와 warm state가 operator-safe evidence에 남아야 한다
**And** preview truth owner처럼 행동하면 안 된다

**Given** 상주형 first-visible worker가 도입된 상태
**When** booth가 current-capture projection을 보여주면
**Then** current-session scope와 same-capture correctness가 유지되어야 한다
**And** 다른 세션 자산이나 이전 capture가 섞이면 안 된다

### Story 1.11: local dedicated renderer sidecar baseline과 capture-bound preview job protocol 도입

**Status:** done

As a owner / brand operator,
I want dedicated local renderer lane와 preview job protocol을 명시적으로 두고 싶다,
So that booth UI와 분리된 상태에서 close latency와 fallback을 검증할 수 있다.

**Acceptance Criteria:**

**Given** capture 저장과 preset binding이 끝난 상태
**When** preview job이 제출되면
**Then** dedicated renderer 경계는 `sessionId`, `captureId`, `requestId`, preset version, route metadata를 포함한 capture-bound job을 받아야 한다
**And** booth는 lane owner와 route stage를 host-normalized truth로만 소비해야 한다

**Given** dedicated renderer가 작업 결과를 반환하면
**When** host가 결과를 정규화하면
**Then** success, fallback, failure reason이 typed status로 남아야 한다
**And** booth UI는 직접 renderer 내부 상태를 해석하면 안 된다

**Given** dedicated renderer lane가 실패하거나 비활성화되면
**When** fallback을 선택하면
**Then** same-capture correlation과 booth-safe waiting이 유지되어야 한다
**And** false-ready나 cross-session leakage가 생기면 안 된다

### Story 1.12: dual close topology 정착과 same-slot truthful replacement 전환

**Status:** done

As a booth customer,
I want 먼저 보이는 이미지와 나중에 닫히는 truthful preview가 한 흐름으로 이어지길 원한다,
So that 빠른 확인과 정확한 결과가 충돌하지 않고 같은 사진 경험으로 느껴진다.

**Acceptance Criteria:**

**Given** early first-visible lane와 later preset-applied close lane가 함께 존재할 때
**When** 같은 capture에 대해 두 단계가 모두 발생하면
**Then** dual close topology는 early visibility와 later truthful close를 분리된 책임으로 유지해야 한다
**And** booth는 두 상태를 하나의 애매한 성공 메시지로 뭉개면 안 된다

**Given** preset-applied truthful preview가 준비되면
**When** current-capture 또는 latest-photo 슬롯이 갱신되면
**Then** same-slot replacement가 same-capture continuity를 유지한 채 일어나야 한다
**And** tiny preview나 rail-only update는 close success로 계산되면 안 된다

**Given** dual close topology에서 fallback이 발생하면
**When** host가 customer-safe state를 계산하면
**Then** booth는 truthful waiting 또는 approved fallback으로만 내려가야 한다
**And** preview/final truth drift와 false complete를 허용하면 안 된다

### Story 1.13: fail-closed guarded cutover와 hardware validation gate

**Status:** backlog

As a owner / brand operator,
I want 새 local full-screen lane를 fail-closed guarded cutover 기준으로 최종 검증하고 싶다,
So that booth가 responsiveness와 preset-applied truth, repeated hardware safety, rollback safety를 함께 만족한 새 경로만 release-safe하게 승격할 수 있다.

**Acceptance Criteria:**

**Given** Stories 1.28, 1.29, 1.30, 1.31이 actual primary lane 기준으로 닫혀 있고 canonical hardware evidence와 route-policy review에서 actual lane `Go` 후보를 만든 approved scope
**When** Story 1.13 rerun을 시작하면
**Then** `preview-renderer-policy.json`은 host-owned rollout artifact로만 제어되어야 한다
**And** final release close는 fail-closed로 동작해 ledger `Go` 전까지 preview architecture `release hold`와 one-action rollback 경로를 유지해야 한다

**Given** 승인된 Windows booth hardware에서 canonical evidence package를 수집할 때
**When** 첫 컷 cold path와 같은 세션의 연속 촬영 steady-state path를 함께 검증하면
**Then** `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`, route policy snapshot, published bundle, catalog state, booth/operator visual evidence, rollback proof가 같은 evidence bundle에 포함되어야 한다
**And** bundle 하나로 `sameCaptureFullScreenVisibleMs`, `firstVisibleMs`, `replacementMs`, `laneOwner`, `fallbackReason`, `routeStage`, `warmState`, parity 결과, follow-up capture timeout 유무를 함께 읽을 수 있어야 한다

**Given** `same-capture preset-applied full-screen visible <= 2500ms` 미달, `wrong-capture != 0`, parity or fidelity drift, fallback 상시 발생, follow-up capture timeout, rollback proof 부재, evidence incomplete, false-ready, false-complete 중 하나라도 남아 있으면
**When** release close를 판정하면
**Then** canonical hardware ledger는 `Go`가 아니라 `No-Go`를 유지해야 한다
**And** `firstVisibleMs`, tiny preview, recent-session strip 업데이트, raw thumbnail, fast preview가 먼저 보였다는 사실 alone, `previewReady` 이전의 어떤 고객 화면 변경도 성공으로 세면 안 된다

### Story 1.14: 공유 계약 동결과 검증 기준 확정

**Status:** done

As a owner / brand operator,
I want 구현 전에 공통 계약을 먼저 동결하고 싶다,
So that booth, operator, authoring, host 구현이 같은 기준을 따를 수 있다.

**Acceptance Criteria:**

**Given** 구현 착수를 준비하는 팀이 있을 때
**When** 공통 계약 기준을 확정하면
**Then** `session.json` session manifest schema가 버전 포함 형태로 동결되어야 한다
**And** capture correlation, preset version reference, post-end state 필드 기준이 문서화되어야 한다

**Given** preset publication과 booth consumption 경계를 확정할 때
**When** 계약 산출물을 정리하면
**Then** preset bundle schema와 runtime profile / capability model이 함께 확정되어야 한다
**And** booth, operator, authoring surface가 같은 계약을 참조해야 한다

**Given** 계약 동결 결과를 개발팀이 사용할 때
**When** 기준 문서를 검토하면
**Then** error envelope와 helper/sidecar protocol contract가 확인 가능해야 한다
**And** 테스트 가능한 예시 또는 검증 기준이 함께 남아 있어야 한다

### Story 1.15: Canon helper profile과 publication contract 확정

**Status:** done

As a owner / brand operator,
I want capture boundary와 preset publication boundary 계약을 먼저 확정하고 싶다,
So that 실카메라 연동과 future-session publication이 구현마다 다르게 해석되지 않도록 할 수 있다.

**Acceptance Criteria:**

**Given** 카메라 boundary 구현을 시작할 때
**When** Canon helper 구현 기준을 확정하면
**Then** Windows Canon EDSDK helper implementation profile이 문서화되어야 한다
**And** helper-ready, camera-status, stale-helper recovery 의미가 host-normalized truth와 연결되어야 한다

**Given** preset publication 경계를 정리할 때
**When** publication contract를 정의하면
**Then** authoring publication payload contract가 필수 필드와 approval state transition을 포함해 확정되어야 한다
**And** immutable published artifact 요구사항이 함께 명시되어야 한다

**Given** 운영 규칙을 계약 수준에서 고정할 때
**When** publication / rollback semantics를 검토하면
**Then** future-session-only publication / rollback rule이 명시되어야 한다
**And** operator diagnostics와 booth-safe state truth에 필요한 helper semantics가 연결되어야 한다

### Story 1.16: Windows desktop build-release baseline과 CI proof 설정

**Status:** done

As a owner / brand operator,
I want 초기 Windows desktop build / release baseline을 먼저 확보하고 싶다,
So that 기능 개발과 별개로 packaging, CI validation, release proof 기준을 안정적으로 유지할 수 있다.

**Acceptance Criteria:**

**Given** 로컬 개발 환경에서 데스크톱 빌드를 검증할 때
**When** baseline build proof를 실행하면
**Then** `pnpm build:desktop` 또는 동등한 로컬 baseline build 경로가 동작해야 한다
**And** 실패 시 packaging 기준을 확인할 수 있는 문서 기준이 존재해야 한다

**Given** 저장소에서 Windows release 검증을 자동화할 때
**When** CI workflow를 구성하면
**Then** `.github/workflows/release-windows.yml`가 unsigned baseline validation path를 제공해야 한다
**And** signing-ready 입력 규칙과 release proof 기준이 문서와 일치해야 한다

**Given** release safety 기준을 정의할 때
**When** 운영 guardrail을 검토하면
**Then** active booth session을 강제 업데이트하지 않는 정책이 유지되어야 한다
**And** automated proof와 hardware proof가 별도 gate라는 사실이 운영 기준에 반영되어야 한다

### Story 1.17: canonical preset recipe와 XMP adapter 기준 동결

**Status:** done

As a owner / brand operator,
I want booth runtime과 authoring/fallback이 공유할 canonical preset recipe를 먼저 고정하고 싶다,
So that GPU lane, darktable fallback, publication bundle이 같은 룩 진실을 기준으로 움직일 수 있다.

**Acceptance Criteria:**

**Given** booth runtime, publication bundle, fallback/oracle 경계를 함께 설계할 때
**When** preset truth 기준을 고정하면
**Then** canonical preset recipe 최소 스키마가 문서와 계약 테스트로 확정되어야 한다
**And** XMP는 compatibility / fallback / parity 자산으로서의 역할이 명시되어야 한다

**Given** 새 preset version이 게시될 때
**When** bundle 구조를 검토하면
**Then** canonical preset recipe reference와 darktable-compatible artifact reference가 함께 연결되어야 한다
**And** booth runtime은 특정 편집기 내부 표현 하나에만 묶이지 않아야 한다

### Story 1.18: retired dedicated close 후보 activation baseline evidence 보존

**Status:** backlog

As a owner / brand operator,
I want 기존 resident preview activation 결과를 retired dedicated close 후보의 baseline evidence로 고정하고 싶다,
So that 새 주력 track 평가가 과거 candidate 결과와 섞이지 않도록 할 수 있다.

**Acceptance Criteria:**

**Given** 기존 resident preview activation evidence를 유지해야 할 때
**When** legacy baseline을 정리하면
**Then** Story 1.18 산출물은 retired dedicated close candidate baseline으로 명시돼야 한다
**And** current forward path owner로 표기되면 안 된다

**Given** 새 local lane과 비교 가능한 기준이 필요할 때
**When** legacy evidence package를 조회하면
**Then** 기존 lane owner, warm-state, activation stage, fallback 결과를 재확인할 수 있어야 한다
**And** 새 lane의 `Go / No-Go` 판단 근거와 혼용되면 안 된다

### Story 1.19: legacy validated track parity와 instrumentation ledger 정리

**Status:** backlog

As a owner / brand operator,
I want 기존 ETW/WPR/WPA/PIX 및 parity evidence를 legacy validated track ledger로 유지하고 싶다,
So that 새 metric/trace reset 이후에도 비교 가능한 과거 기준을 잃지 않을 수 있다.

**Acceptance Criteria:**

**Given** 과거 resident candidate evidence를 다시 읽어야 할 때
**When** legacy ledger를 검토하면
**Then** ETW/WPR/WPA/PIX 또는 동등 계측 기반 결과가 legacy comparison baseline으로 분류되어야 한다
**And** current promotion gate의 최종 통과 증거로 오인되면 안 된다

**Given** 새 주력 track이 same KPI를 검증할 때
**When** comparison report를 만들면
**Then** legacy ledger는 parity, fallback 안정성, visibility timing의 기준점으로 재사용 가능해야 한다
**And** Story 1.25 default decision authority를 직접 가지면 안 된다

### Story 1.20: legacy route activation validation track 유지

**Status:** backlog

As a owner / brand operator,
I want 기존 resident lane activation/canary/default 결과를 legacy validated track으로 남겨두고 싶다,
So that rollback rehearsal과 historical comparison evidence는 유지하되 현재 release-close path와는 분리할 수 있다.

**Acceptance Criteria:**

**Given** 기존 route activation evidence를 보존해야 할 때
**When** legacy track를 문서화하면
**Then** canary/default promotion, rollback rehearsal, route policy snapshot은 legacy validation 결과로 남아야 한다
**And** 새 forward path의 activation owner로 재사용되면 안 된다

**Given** active booth route를 새 local lane 기준으로 승격해야 할 때
**When** Story 1.20 결과를 참고하면
**Then** Story 1.20은 historical reference와 rollback rehearsal 근거만 제공해야 한다
**And** Story 1.13 또는 Story 1.25의 release decision input을 대체하면 안 된다

### Story 1.21: metric reset과 full-screen 2500ms acceptance 정렬

**Status:** done

As a owner / brand operator,
I want 제품 합격 기준을 full-screen visible KPI와 fail-closed release gate로 다시 고정하고 싶다,
So that preview 성능 판단이 고객 체감과 같은 기준으로 다시 정렬되고 중간 신호를 제품 성공으로 오해하지 않게 할 수 있다.

**Acceptance Criteria:**

**Given** release sign-off KPI를 재정의할 때
**When** 성능 기준 문서와 runbook을 갱신하면
**Then** primary acceptance는 `same-capture preset-applied full-screen visible <= 2500ms`로 고정되어야 한다
**And** `5s preview confirmation`은 보조 운영 guardrail로만 남아야 한다

**Given** booth evidence ledger와 운영 대시보드를 검토할 때
**When** 최종 판정 필드를 확인하면
**Then** `sameCaptureFullScreenVisibleMs`, `wrong-capture`, `fidelity drift`, fallback stability, one-action rollback evidence, follow-up capture timeout 유무가 모두 명시적 sign-off 필드여야 한다
**And** 이 중 하나라도 미달이거나 비어 있으면 final verdict는 `Go`가 아니라 `No-Go`여야 한다

**Given** 운영자와 릴리스 오너가 성능 신호를 해석할 때
**When** 합격 여부를 판단하면
**Then** `firstVisibleMs`, tiny preview, recent-session strip 업데이트, raw thumbnail, fast preview가 먼저 보였다는 사실 alone, `previewReady` 이전의 어떤 고객 화면 변경도 제품 성공으로 세면 안 된다
**And** same-capture 정합성, preset fidelity, fallback stability를 함께 닫지 못하면 speed 신호만으로 `done`을 선언하면 안 된다

### Story 1.22: capture -> full-screen visible evidence chain trace reset

**Status:** done

As a owner / brand operator,
I want capture부터 full-screen visible까지의 correlation chain과 evidence completeness를 다시 정렬하고 싶다,
So that 잘못된 캡처 매칭이나 부분 계측 없이 같은 촬영본 기준의 진실한 evidence로 cold path와 steady-state path를 함께 판정할 수 있다.

**Acceptance Criteria:**

**Given** same-capture KPI를 판정해야 할 때
**When** trace/evidence schema를 갱신하면
**Then** capture request, raw persistence, first-visible, preset-applied truthful artifact ready, full-screen visible 이벤트가 같은 correlation id로 연결돼야 한다
**And** session id, capture id, preset version, lane owner, route stage, warm state, visible owner 전환 시점이 함께 남아야 한다

**Given** operator-safe evidence bundle을 수집할 때
**When** 승인된 Windows hardware에서 첫 컷 cold path와 같은 세션의 연속 촬영 steady-state path를 함께 기록하면
**Then** `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`, route policy snapshot, published bundle, catalog state, booth/operator visual evidence, rollback proof가 누락 없이 묶여야 한다
**And** 같은 bundle 하나로 KPI, `wrong-capture`, fidelity drift, fallback reason, follow-up capture timeout, rollback proof를 함께 읽을 수 있어야 한다

**Given** fresh capture evidence를 검토할 때
**When** operator나 release owner가 selected capture를 열면
**Then** wrong-capture, stale-preview, cross-session attribution, false-ready, false-complete, follow-up capture timeout을 판별할 수 있어야 한다
**And** `firstVisibleMs`, tiny preview, recent-session strip 업데이트, raw thumbnail, fast preview가 먼저 보였다는 사실 alone, `previewReady` 이전의 어떤 고객 화면 변경은 supporting signal로만 남고 success verdict를 대체하면 안 된다

### Story 1.23: host-owned local native/GPU resident full-screen lane prototype과 truthful artifact generation

**Status:** done

As a owner / brand operator,
I want host-owned local native/GPU resident full-screen lane prototype로 display-sized preset-applied truthful artifact를 만들고 싶다,
So that darktable-compatible path를 parity/fallback/final reference로 유지한 채 future primary close candidate를 실제 하드웨어에서 검증할 수 있다.

**Acceptance Criteria:**

**Given** approved booth hardware와 feature flag가 있을 때
**When** host-owned local native/GPU resident full-screen lane prototype을 활성화하면
**Then** local lane은 same-capture display-sized preset-applied truthful artifact를 full-screen close owner로 생성해야 한다
**And** darktable-compatible path는 parity/fallback/final reference로 계속 유지되어야 한다

**Given** first-visible source와 preset-applied truthful preview를 함께 운영할 때
**When** same-capture preview를 고객 화면에 반영하면
**Then** customer-safe projection은 먼저 보일 수 있어도 `previewReady` truth owner가 되면 안 된다
**And** truthful close는 local full-screen lane artifact 기준으로만 판단되어야 한다

**Given** local lane이 실패하거나 health를 잃을 때
**When** booth가 fallback을 수행하면
**Then** false-ready, false-complete, wrong-capture, cross-session leakage 없이 darktable-compatible baseline path로 내려가야 한다
**And** operator evidence에는 `laneOwner`, `fallbackReason`, `routeStage`, parity reference linkage가 남아야 한다

### Story 1.24: local lane hardware canary validation

**Status:** done

As a owner / brand operator,
I want host-owned local full-screen lane를 hardware canary로 검증하고 싶다,
So that prototype 성공이 실제 운영 조건에서도 correctness, fidelity, evidence completeness를 유지하며 반복 가능한지 판단할 수 있다.

**Acceptance Criteria:**

**Given** 승인된 Windows booth hardware canary scope가 있을 때
**When** local lane canary를 실행하면
**Then** `sameCaptureFullScreenVisibleMs`, `replacementMs`, `fallback ratio`, `wrong-capture`, `fidelity drift`, `evidence completeness`를 같은 evidence bundle에서 읽을 수 있어야 한다
**And** canary session은 active customer session safety를 훼손하면 안 된다

**Given** canary 결과를 승격 판단에 사용할 때
**When** health gate를 검토하면
**Then** KPI miss, fallback 상시화, wrong-capture, fidelity drift, evidence gap 중 하나라도 남아 있으면 `No-Go`가 유지되어야 한다
**And** darktable-compatible parity/fallback/final reference와의 대조가 불가능하면 다음 단계로 진행하면 안 된다

**Given** rollback readiness를 함께 확인할 때
**When** operator-safe package를 검토하면
**Then** one-action rollback proof가 같은 승인 패키지 안에서 확인돼야 한다
**And** rollback proof가 없으면 Story 1.25로 진행하면 안 된다

### Story 1.25: local lane default decision과 rollback gate

**Status:** done

As a owner / brand operator,
I want local lane의 default 승격 여부를 명시적 gate로 결정하고 싶다,
So that revised primary path의 `Go / No-Go`와 rollback authority를 release-close 직전까지 통제할 수 있다.

**Acceptance Criteria:**

**Given** Story 1.24 canary가 `Go` 후보를 만들었을 때
**When** default decision gate를 실행하면
**Then** host-owned `preview-renderer-policy.json`은 approved scope를 `canary`에서 `default`로 승격하거나 즉시 되돌릴 수 있어야 한다
**And** active session은 route policy 변경으로 재해석되면 안 된다

**Given** default decision 결과를 검토할 때
**When** operator-safe package를 읽으면
**Then** `laneOwner=local-fullscreen-lane`, `routeStage=default|rollback`, `fallbackReason`, KPI 결과, parity/fallback/final reference status, rollback proof가 함께 확인돼야 한다
**And** Story 1.13은 Story 1.25 결과만으로 시작되면 안 되며, actual primary lane 재구현/재검증 track인 Stories 1.28~1.31이 닫히기 전에는 blocked로 유지되어야 한다

**Given** local lane이 아직 반복 hardware evidence를 충분히 만들지 못했을 때
**When** default 승격 여부를 판단하면
**Then** fail-closed `No-Go`가 유지되어야 한다
**And** Story 1.25 구현 완료 사실만으로 release `Go`로 해석하면 안 된다

### Story 1.26: repeated local failure가 증명될 때만 reserve option 개시

**Status:** backlog

As a owner / brand operator,
I want local lane이 반복적으로 실패한다는 사실이 충분히 증명될 때만 reserve option을 열고 싶다,
So that 현재 release path를 불필요하게 분산시키지 않고 correctness, fidelity, evidence completeness를 보존한 상태에서만 예비 구조를 검토할 수 있다.

**Acceptance Criteria:**

**Given** local full-screen lane이 actual primary implementation, evidence/vocabulary realignment, canary, default decision 검증 단계를 거친 뒤에도 승인 하드웨어에서 같은 KPI를 반복 실패할 때
**When** reserve option 개시 여부를 판단하면
**Then** `single off-box renderer cell/stamp` 성격의 reserve track 또는 동등한 remote renderer / edge appliance 실험 경로를 별도 track으로만 열 수 있어야 한다
**And** local lane이 아직 `Go` 후보를 만들 수 있는 동안에는 reserve option을 열면 안 된다

**Given** reserve option 개시 판단을 검토할 때
**When** 승인 패키지를 읽으면
**Then** repeated local KPI miss와 함께 `wrong-capture=0`, `fidelity drift=0`, fallback behavior, evidence completeness preserved가 같이 확인돼야 한다
**And** capture-bound truth/evidence contract가 유지되지 않으면 reserve option 개시 근거로 인정되면 안 된다

**Given** reserve track을 시작할 때
**When** 운영 범위를 정의하면
**Then** current release-close path와 customer default route는 local lane 기준을 유지해야 한다
**And** reserve 결과는 별도 승인 없이는 Story 1.13의 `Go / No-Go` 경로를 대체하면 안 된다

### Story 1.27: local hot path의 darktable-bound close와 follow-up capture timeout 재검증

**Status:** done

As a owner / brand operator,
I want local lane hot path가 darktable-compatible preview run에 다시 묶이지 않고 follow-up capture timeout도 남기지 않는지 증명하고 싶다,
So that reserve option을 열기 전에 local forward path의 실제 가능성을 마지막으로 검증할 수 있다.

**Acceptance Criteria:**

**Given** approved booth hardware와 approved preset/version scope가 있을 때
**When** local lane hot path를 실행하면
**Then** `display-sized preset-applied truthful artifact`는 host-owned local native/GPU resident full-screen lane에서 닫혀야 한다
**And** darktable-compatible path는 parity/fallback/final reference로만 남아야 한다

**Given** cold 1컷과 연속 3~5컷 hardware run을 같은 evidence bundle 기준으로 검토할 때
**When** 결과를 읽으면
**Then** `sameCaptureFullScreenVisibleMs`, `wrong-capture`, `fidelity drift`, `fallback ratio`, `follow-up capture timeout`, `follow-up capture completion`을 함께 판단할 수 있어야 한다
**And** darktable-bound close 징후, helper completion boundary 병목, 또는 follow-up timeout이 남으면 fail-closed `No-Go`가 유지되어야 한다

**Given** local path가 health를 잃거나 parity를 만족하지 못할 때
**When** booth가 fallback을 수행하면
**Then** false-ready, wrong-capture, cross-session leakage 없이 baseline path로 내려가야 한다
**And** Story 1.13 또는 Story 1.26은 이 스토리 결과만으로 자동 개시되면 안 된다

### Story 1.28: actual primary lane close owner 구현과 prototype route 분리

**Status:** backlog

As a owner / brand operator,
I want 최종 아키텍처가 요구한 actual primary lane을 제품의 실제 close owner로 구현하고 싶다,
So that booth의 주 경로가 prototype dedicated-renderer evidence track과 분리되고 release-close 판단이 새 구조 위에서만 이뤄질 수 있다.

**Acceptance Criteria:**

**Given** approved booth hardware와 approved preset/version scope가 있을 때
**When** actual primary lane을 실행하면
**Then** `display-sized preset-applied truthful artifact`는 final architecture가 정의한 host-owned local native/GPU resident full-screen lane에서 닫혀야 한다
**And** prototype dedicated-renderer route는 current primary close owner로 해석되면 안 된다

**Given** actual primary lane과 darktable-compatible reference를 함께 운영할 때
**When** booth가 same-capture preview close를 계산하면
**Then** darktable-compatible path는 parity/fallback/final reference로만 남아야 한다
**And** latency-critical hot path는 darktable preview invocation completion을 직접 기다리면 안 된다

**Given** actual primary lane이 실패하거나 health를 잃을 때
**When** booth가 fallback을 수행하면
**Then** false-ready, false-complete, wrong-capture, cross-session leakage 없이 fail-closed 되어야 한다
**And** legacy prototype evidence나 route vocabulary가 actual lane success를 대체하면 안 된다

### Story 1.29: actual primary lane evidence와 vocabulary realignment

**Status:** backlog

As a owner / brand operator,
I want actual primary lane 기준으로 evidence와 운영 vocabulary를 다시 잠그고 싶다,
So that prototype track의 용어와 증거가 final architecture 구현 완료로 오인되지 않게 할 수 있다.

**Acceptance Criteria:**

**Given** actual primary lane evidence bundle과 operator diagnostics를 검토할 때
**When** selected capture 문맥을 읽으면
**Then** lane owner, route kind, route stage, visible owner, rollback proof, parity/fallback/final reference linkage가 actual lane 기준으로 일관되게 해석돼야 한다
**And** legacy dedicated-renderer vocabulary는 comparison/prototype evidence로만 남아야 한다

**Given** 스프린트, release, hardware ledger 문서를 함께 검토할 때
**When** story 상태와 final close ownership을 판단하면
**Then** `prototype done`, `gate done`, `release-close owner`가 구분되어야 한다
**And** Story 1.23~1.27의 완료 사실만으로 actual primary lane 구현 완료로 읽히면 안 된다

### Story 1.30: actual primary lane hardware canary 재검증

**Status:** backlog

As a owner / brand operator,
I want actual primary lane 기준으로 hardware canary를 다시 수행하고 싶다,
So that 새 주 경로가 prototype evidence가 아니라 실제 구현 기준으로 KPI와 correctness를 입증하는지 확인할 수 있다.

**Acceptance Criteria:**

**Given** 승인된 Windows booth hardware canary scope가 있을 때
**When** actual primary lane canary를 실행하면
**Then** `sameCaptureFullScreenVisibleMs`, `wrong-capture`, `fidelity drift`, fallback stability, follow-up capture health를 같은 evidence bundle에서 읽을 수 있어야 한다
**And** 결과는 prototype track supporting evidence와 분리된 actual lane canary verdict로 기록돼야 한다

**Given** canary 결과를 다음 단계 입력으로 사용할 때
**When** health gate를 검토하면
**Then** KPI miss, fallback-heavy, wrong-capture, fidelity drift, evidence gap 중 하나라도 남아 있으면 `No-Go`가 유지돼야 한다
**And** 이 verdict 없이는 Story 1.31로 진행하면 안 된다

### Story 1.31: actual primary lane default decision과 rollback gate

**Status:** backlog

As a owner / brand operator,
I want actual primary lane의 default 승격과 rollback authority를 별도 gate로 닫고 싶다,
So that Story 1.13 final close가 prototype track이 아니라 실제 주 경로 기준에서만 열리게 할 수 있다.

**Acceptance Criteria:**

**Given** Story 1.30이 actual primary lane `Go` 후보를 만들었을 때
**When** default decision gate를 실행하면
**Then** host-owned `preview-renderer-policy.json`은 actual lane scope를 `canary`에서 `default`로 승격하거나 one-action rollback으로 되돌릴 수 있어야 한다
**And** active session은 route policy 변경으로 재해석되면 안 된다

**Given** default decision 결과를 검토할 때
**When** operator-safe package를 읽으면
**Then** actual lane 기준 `laneOwner`, `routeStage=default|rollback`, KPI 결과, rollback proof, parity/fallback/final reference status가 함께 확인돼야 한다
**And** Story 1.13은 Story 1.31이 canonical actual-lane `Go` 후보를 만들기 전에는 시작되면 안 된다

**Given** actual primary lane이 아직 반복 hardware evidence를 충분히 만들지 못했을 때
**When** default 승격 여부를 판단하면
**Then** fail-closed `No-Go`가 유지돼야 한다
**And** Story 1.31 구현 완료 사실만으로 release `Go`로 해석하면 안 된다

### Preview Architecture Sequencing Note

- Stories 1.18, 1.19, 1.20은 retired dedicated close candidate의 activation baseline / legacy validated track이다.
- Stories 1.21, 1.22는 new-track metric/evidence baseline이다.
- Stories 1.23, 1.24, 1.25는 prototype / gate / governance track으로 남으며, 이력 보존 대상이지 final actual primary lane 구현 완료를 뜻하지 않는다.
- Story 1.27은 prototype track의 corrective follow-up이며, actual primary lane 구현 완료를 대체하지 않는다.
- Stories 1.28, 1.29, 1.30, 1.31이 actual primary lane 재구현 및 재검증 forward path다.
- Story 1.13은 Story 1.31이 canonical actual-lane `Go` 후보와 rollback evidence를 만들기 전에는 reopen되면 안 되는 final guarded cutover / release-close owner다.
- Story 1.26은 Story 1.31 이후에도 actual primary lane이 승인 하드웨어에서 같은 KPI를 반복 실패할 때만 열리는 reserve experiment다.

## Epic 2: 현재 세션 중심의 촬영 제어와 시간 인지

고객이 현재 세션 사진만 검토하고 삭제하며, 세션 중 프리셋을 바꾸고, 남은 시간을 이해하면서 촬영을 이어갈 수 있게 한다.

**Status:** done

### Story 2.1: 현재 세션 전용 사진 검토 화면

**Status:** done

As a booth customer,
I want 내 현재 세션 사진만 검토하고 싶다,
So that 다른 사람 사진 없이 지금 촬영 결과만 자신 있게 확인할 수 있다.

**Acceptance Criteria:**

**Given** 현재 세션에 하나 이상의 촬영본이 있을 때
**When** 고객이 검토 영역 또는 latest photo rail을 열면
**Then** 현재 세션 자산만 보여야 한다
**And** 교차 세션 사진이나 이전 세션 흔적은 노출되지 않아야 한다

**Given** 검토 화면이 표시될 때
**When** 상태 정보를 함께 보여주면
**Then** 활성 프리셋과 최신 결과가 인지 가능해야 한다
**And** 고객용 문구는 plain-language와 낮은 문구 밀도 원칙을 유지해야 한다

**Given** 검토 경험을 부스 화면에 배치할 때
**When** 대형 터치스크린 기준으로 UI를 구성하면
**Then** 사진 목록과 핵심 액션은 멀리서도 인지 가능한 위계를 가져야 한다
**And** 현재 세션 범위와 관련 없는 운영자 정보는 노출되지 않아야 한다

### Story 2.2: 현재 세션 삭제 정책 기반 사진 삭제

**Status:** done

As a booth customer,
I want 삭제가 허용된 현재 세션 사진만 지우고 싶다,
So that 원치 않는 컷을 정리하되 다른 자산을 건드리지 않을 수 있다.

**Acceptance Criteria:**

**Given** 삭제 대상 촬영본이 `Current-Session Deletion Policy`를 만족할 때
**When** 고객이 삭제를 확인하면
**Then** 해당 current-session source와 연관 preview/final 파생 자산만 제거되어야 한다
**And** 세션 매니페스트와 감사 기록도 함께 갱신되어야 한다

**Given** 삭제 대상이 활성 mutation 상태이거나 정책상 삭제 불가일 때
**When** 고객이 삭제를 시도하면
**Then** 삭제는 차단되어야 한다
**And** 내부 저장 구조를 드러내지 않는 plain-language 안내가 보여야 한다

**Given** 삭제 확인 UI를 보여줄 때
**When** 고객이 결정을 내려야 하면
**Then** destructive action은 분명히 구분되어야 한다
**And** 실수 방지를 위한 명확한 확인 단계가 제공되어야 한다

### Story 2.3: 세션 중 활성 프리셋 변경

**Status:** done

As a booth customer,
I want 세션 도중에도 활성 프리셋을 바꾸고 싶다,
So that 이후 촬영부터 다른 룩을 바로 적용할 수 있다.

**Acceptance Criteria:**

**Given** 현재 세션이 진행 중일 때
**When** 고객이 다른 승인 프리셋을 선택하면
**Then** 새 프리셋은 즉시 다음 촬영의 활성 프리셋이 되어야 한다
**And** 이미 저장된 과거 촬영본의 프리셋 바인딩은 바뀌면 안 된다

**Given** 프리셋 전환이 반영되면
**When** 화면이 업데이트되면
**Then** 다음 촬영에 적용될 활성 프리셋이 분명히 보여야 한다
**And** 이전 촬영이 재편집되었다고 오해하게 만들면 안 된다

**Given** 프리셋 변경 UI를 제공할 때
**When** 고객이 현재 세션 도중 카탈로그를 다시 열면
**Then** 승인된 게시 프리셋만 다시 보여야 한다
**And** 직접 편집 도구나 내부 저작 개념은 계속 숨겨져 있어야 한다

### Story 2.4: 조정된 종료 시각 상시 노출

**Status:** done

As a booth customer,
I want 남은 시간과 종료 시각을 세션 내내 확인하고 싶다,
So that 촬영 페이스를 스스로 조절할 수 있다.

**Acceptance Criteria:**

**Given** 활성 세션이 시작되면
**When** 고객이 부스 흐름을 보는 동안
**Then** 조정된 종료 시각이 세션 시작부터 인지 가능해야 한다
**And** 시간 정보는 고객 친화적 시각 위계와 plain-language를 따라야 한다

**Given** 쿠폰 또는 승인된 운영 정책으로 세션 시간이 계산될 때
**When** 호스트가 timing truth를 확정하면
**Then** 부스 화면은 그 조정된 종료 시각을 일관되게 표시해야 한다
**And** 화면별로 서로 다른 시간 진실을 보여주면 안 된다

**Given** 시간 안내 컴포넌트를 구성할 때
**When** 대형 터치스크린 기준으로 배치하면
**Then** 디지털 타이머와 상태별 시각 강조가 인지 가능해야 한다
**And** 고객이 현재 촬영 가능 여부를 함께 해석할 수 있어야 한다

**Given** 경고 또는 정확 종료 임계값에 도달한 qualifying session을 운영 기준으로 평가할 때
**When** timing policy와 로그를 검토하면
**Then** 5분 경고와 정확 종료 알림은 99% 세션에서 `+/- 5초` 이내에 발생해야 한다
**And** 승인된 연장과 감사 기록이 없는 한 종료 후 촬영은 차단되어야 한다

### Story 2.5: 5분 경고와 종료 시각 알림, 촬영 가능 여부 갱신

**Status:** done

As a booth customer,
I want 중요한 시간 임계점에서 무엇을 해야 하는지 분명히 알고 싶다,
So that 마지막 촬영을 급하게 망치지 않고 안전하게 마무리할 수 있다.

**Acceptance Criteria:**

**Given** 세션이 종료 5분 전 임계값에 도달하면
**When** 타이밍 정책이 경고 상태를 발생시키면
**Then** 승인된 사운드 경고와 시각 배지가 함께 보여야 한다
**And** 고객은 여전히 촬영 가능 여부를 이해할 수 있어야 한다

**Given** 세션이 조정된 종료 시각에 도달하면
**When** 종료 알림이 발생하면
**Then** 승인된 종료 사운드와 함께 촬영 가능 여부가 즉시 갱신되어야 한다
**And** 정책 외 추가 촬영은 차단되어야 한다

**Given** 경고 또는 종료 상태를 고객에게 보여줄 때
**When** 상태 문구를 구성하면
**Then** 공포를 키우지 않는 안정적 위계의 안내가 제공되어야 한다
**And** 내부 타이머 구현 세부사항이나 운영자 진단 정보는 노출되지 않아야 한다

**Given** 운영 환경의 qualifying session을 기준으로 warning/end 임계값을 평가할 때
**When** 경고와 종료 알림의 실제 발화 시각을 분석하면
**Then** 5분 경고와 정확 종료 알림은 99% 세션에서 `+/- 5초` 이내에 발생해야 한다
**And** 허용 오차를 벗어난 세션은 후속 분석이 가능하도록 감사 가능하게 기록되어야 한다

**Given** exact-end 직후 explicit post-end truth가 아직 확정되지 않은 짧은 구간이 있을 때
**When** 고객 화면이 bridge copy를 보여주면
**Then** 화면은 "촬영이 끝났고 다음 단계를 준비 중"이라는 중립 안전 문구만 보여야 한다
**And** 이미 `Export Waiting`, `Completed`, `Phone Required` 중 하나가 확정된 뒤에는 generic ended/waiting copy가 그 상태를 되돌리거나 희석하면 안 된다

## Epic 3: 종료 후 결과 준비와 인계의 진실한 마무리

고객이 촬영 종료 후 `Export Waiting`, `Completed`, `Phone Required` 중 현재 상태를 정확히 이해하고 다음 행동을 혼란 없이 수행할 수 있게 한다.

**Status:** done

### Story 3.1: 종료 직후 명시적 사후 상태 진입

**Status:** done

As a booth customer,
I want 촬영 종료 직후 현재 상태가 명확히 바뀌는 것을 알고 싶다,
So that 계속 기다려야 하는지, 완료됐는지, 도움을 받아야 하는지 바로 이해할 수 있다.

**Acceptance Criteria:**

**Given** 조정된 종료 시각에 도달한 활성 세션
**When** 더 이상 촬영이 허용되지 않으면
**Then** 부스는 정확히 하나의 사후 상태인 `Export Waiting`, `Completed`, `Phone Required` 중 하나로 전환되어야 한다
**And** 애매한 중간 촬영 상태에 머물면 안 된다

**Given** 사후 상태가 고객에게 표시될 때
**When** 화면이 렌더링되면
**Then** 기술 진단어 없이 이해 가능한 안내가 제공되어야 한다
**And** 다음 행동이 시각적으로 분명해야 한다

**Given** 종료 직후 상태 전환이 일어날 때
**When** 호스트가 사후 상태를 확정하면
**Then** 세션은 종료 시각 기준 허용된 시간 안에 명시적 사후 상태로 진입해야 한다
**And** 촬영 가능 상태가 잘못 유지되면 안 된다

### Story 3.2: Export Waiting과 truthful completion 안내

**Status:** done

As a booth customer,
I want 결과 준비 중인지 이미 완료됐는지를 정확히 알고 싶다,
So that 아직 처리 중인 세션을 실패로 오해하거나 너무 빨리 떠나지 않을 수 있다.

**Acceptance Criteria:**

**Given** 촬영은 종료됐지만 부스 측 필수 작업이 아직 끝나지 않았을 때
**When** 사후 상태를 계산하면
**Then** 부스는 `Export Waiting`을 보여야 한다
**And** 촬영은 계속 비활성화되어야 한다

**Given** 모든 booth-side required work가 실제로 완료됐을 때
**When** 완료 상태를 노출하면
**Then** `Completed / Local Deliverable Ready` 또는 `Completed / Handoff Ready` 중 하나로 해석되어야 한다
**And** booth-side 작업이 끝나기 전에는 완료를 선언하면 안 된다

**Given** 렌더 재시도나 지연이 발생하더라도
**When** 상태를 갱신하면
**Then** 이미 저장된 current-session 원본은 보존되어야 한다
**And** false-complete 없이 explicit waiting 또는 escalation으로 유지되어야 한다

### Story 3.3: Handoff Ready와 Phone Required 보호 안내

**Status:** done

As a booth customer,
I want 완료 후 다음 행동이나 도움 요청 방식을 한 번에 이해하고 싶다,
So that 임의 조작 없이 안전하게 세션을 마칠 수 있다.

**Acceptance Criteria:**

**Given** 세션이 `Handoff Ready`로 해석될 때
**When** 완료 화면이 표시되면
**Then** 승인된 다음 위치 또는 수령 대상과 다음 행동이 함께 보여야 한다
**And** 필요 시 booth alias도 함께 보여야 한다

**Given** 세션이 정상 완료로 해소되지 못할 때
**When** 부스가 `Phone Required`에 진입하면
**Then** 화면은 현재 세션 보존 여부와 단일 연락 액션을 보여야 한다
**And** 고객이 하지 말아야 할 행동은 짧고 분명하게 차단해야 한다

**Given** `Phone Required` 화면이 표시되면
**When** 시각 표현과 문구를 구성하면
**Then** 공포를 키우는 장애 덤프가 아니라 보호형 도움 요청 화면이어야 한다
**And** 경고 색을 쓰더라도 안정적 위계와 큰 행동 버튼이 우선되어야 한다

## Epic 4: 승인 기반 프리셋 게시와 미래 세션 카탈로그 운영

내부 프리셋 관리자가 프리셋을 작성, 검증, 승인, 게시, 롤백하여 미래 세션용 카탈로그를 안전하게 운영할 수 있게 한다.

**Status:** in-progress

### Story 4.1: 내부 프리셋 작성 작업공간

**Status:** done

As a authorized preset manager,
I want 고객 흐름과 분리된 내부 작성 공간에서 프리셋 초안을 만들고 싶다,
So that booth 고객에게 제작 도구를 노출하지 않고 새 룩을 준비할 수 있다.

**Acceptance Criteria:**

**Given** 관리자 인증에 성공한 authoring 가능 환경
**When** 프리셋 초안을 생성하거나 수정하면
**Then** 작업 결과는 draft 상태의 내부 프리셋 후보로 저장되어야 한다
**And** 고객 카탈로그에는 어떤 변화도 발생하지 않아야 한다

**Given** authoring surface가 열릴 때
**When** 내부 사용자가 프리셋을 편집하면
**Then** 필요한 authoring 제어만 노출되어야 한다
**And** 고객용 booth surface에서는 동일 제어가 보이면 안 된다

**Given** 프리셋 작성 환경을 운영할 때
**When** darktable 기반 authoring 자산을 다루면
**Then** draft authoring state와 published booth artifact는 명확히 분리되어야 한다
**And** active session 데이터는 직접 수정되면 안 된다

### Story 4.2: 부스 호환성 검증과 승인 준비 상태 전환

**Status:** done

As a authorized preset manager,
I want 프리셋 초안을 booth 호환성 기준으로 검증하고 싶다,
So that 안전하고 재현 가능한 프리셋만 승인 단계로 보낼 수 있다.

**Acceptance Criteria:**

**Given** draft 상태의 프리셋 버전이 존재할 때
**When** 부스 호환성 검증을 실행하면
**Then** 렌더 호환성, 필요한 아티팩트 필드, 게시 가능 제약을 평가해야 한다
**And** 통과한 경우에만 `validated` 상태로 이동할 수 있어야 한다

**Given** 검증이 실패할 때
**When** 결과를 반환하면
**Then** 프리셋은 고객 카탈로그에 포함되면 안 된다
**And** 내부 사용자에게는 조치 가능한 검증 피드백이 제공되어야 한다

**Given** 검증 기준을 적용할 때
**When** preset bundle과 publication contract를 확인하면
**Then** 동결된 계약 산출물과 충돌하는 draft는 통과하면 안 된다
**And** booth-safe preview/final behavior 보장이 확인 가능해야 한다

### Story 4.3: 승인과 불변 게시 아티팩트 생성

**Status:** in-progress

As a authorized preset manager,
I want 검증된 프리셋을 승인하고 불변 게시 아티팩트로 만들고 싶다,
So that 미래 세션에서 안정적으로 재사용 가능한 카탈로그 항목을 운영할 수 있다.

**Acceptance Criteria:**

**Given** `validated` 상태의 프리셋 버전
**When** 승인 권한자가 게시를 수행하면
**Then** stable identity와 version을 가진 immutable preset bundle이 생성되어야 한다
**And** 카탈로그 메타데이터와 게시 감사 이력이 함께 기록되어야 한다

**Given** 게시가 완료되면
**When** 미래 세션이 카탈로그를 불러오면
**Then** 새 게시 버전은 선택 가능한 프리셋으로 보일 수 있어야 한다
**And** 이미 진행 중인 활성 세션은 변경되면 안 된다

**Given** 메타데이터 불일치, immutability 위반, future-session-only 규칙 위반이 있을 때
**When** 게시를 시도하면
**Then** 게시가 거부되어야 한다
**And** 부분적으로 생성된 published artifact가 남으면 안 된다

### Story 4.4: 미래 세션 대상 롤백

**Status:** done

As a authorized preset manager,
I want 이전 승인 버전으로 카탈로그를 되돌리고 싶다,
So that 문제가 있는 프리셋 릴리스를 활성 세션을 깨지 않고 회수할 수 있다.

**Acceptance Criteria:**

**Given** 하나의 프리셋 identity에 여러 승인된 게시 버전이 있을 때
**When** 관리자가 롤백 대상을 선택하면
**Then** 선택된 이전 버전이 future-session 카탈로그 기준 버전이 되어야 한다
**And** 현재 진행 중인 세션의 바인딩은 변경되면 안 된다

**Given** 게시 또는 롤백 액션이 완료되면
**When** 감사 기록을 남기면
**Then** preset identity, version, action type, timestamp, actor가 기록되어야 한다
**And** 카탈로그 상태는 일관되게 유지되어야 한다

**Given** 롤백을 수행할 때
**When** 대상 버전을 활성화하면
**Then** 승인된 이전 게시 버전만 복귀 대상으로 선택할 수 있어야 한다
**And** draft 또는 validated 상태의 프리셋은 직접 롤백 기준이 되면 안 된다

## Epic 5: 운영자 복구와 감사 로그

운영자가 현재 세션 문제를 안전한 범위에서 진단하고 복구하며, 개입 결과를 감사 가능하게 남길 수 있게 한다.

**Status:** in-progress

### Story 5.1: 운영자용 현재 세션 문맥과 장애 진단

**Status:** done

As a remote operator,
I want 현재 세션 문맥과 막힌 경계를 한눈에 보고 싶다,
So that capture, preview, render, completion 중 어디서 막혔는지 추측 없이 파악할 수 있다.

**Acceptance Criteria:**

**Given** 활성 또는 차단된 부스 세션
**When** 운영자가 operator console을 열면
**Then** 현재 세션 식별자, 타이밍 상태, 최근 실패 문맥, 정규화된 blocked-state category가 보여야 한다
**And** capture 경계와 preview/render/completion 경계가 구분되어야 한다

**Given** 운영자 진단 화면이 렌더링될 때
**When** 상태 세부 정보를 보여주면
**Then** raw helper 출력이 아니라 운영자용으로 정리된 진단 정보여야 한다
**And** 고객용 copy에는 영향이 없어야 한다

**Given** 운영자 화면에서 문제를 분류할 때
**When** 세션 상태를 요약하면
**Then** 현재 복구 가능한 범위와 blocked boundary가 함께 인지 가능해야 한다
**And** 불필요한 내부 로그 덤프는 기본 화면에 노출되지 않아야 한다

### Story 5.2: 정책 기반 복구 액션과 Phone Required 라우팅

**Status:** done

As a remote operator,
I want 허용된 복구 액션만 실행하고 싶다,
So that 무제한 제어 없이 안전한 범위 안에서 세션을 복구할 수 있다.

**Acceptance Criteria:**

**Given** 차단 상태 카테고리가 식별됐을 때
**When** 운영자가 액션 패널을 열면
**Then** `Operator Recovery Policy`가 허용한 액션만 노출되어야 한다
**And** 허용되지 않은 액션은 실행할 수 없어야 한다

**Given** 운영자가 retry, approved boundary restart, allowed time extension 중 하나를 실행하면
**When** 액션이 완료되면
**Then** 세션은 정규화된 다음 상태로 이동하거나 필요 시 `Phone Required`로 라우팅되어야 한다
**And** 고객 화면에는 unsafe recovery 조작이 노출되면 안 된다

**Given** 복구가 정책 범위를 넘는 실패 상태일 때
**When** 운영자가 세션을 계속 진행시키려 하면
**Then** 시스템은 직접 복구 대신 `Phone Required` 경로를 허용해야 한다
**And** 승인되지 않은 우회 조작은 차단되어야 한다

### Story 5.3: 라이프사이클과 개입 감사 로그

**Status:** done

As a owner / operations lead,
I want 세션 전이와 운영자 개입 기록을 일관되게 남기고 싶다,
So that 장애 유형과 운영 부담을 나중에 정확히 분석할 수 있다.

**Acceptance Criteria:**

**Given** 세션 라이프사이클 전이, 운영자 개입, 게시 관련 복구 이벤트, 중요 실패가 발생하면
**When** 호스트가 이벤트를 확정하면
**Then** timestamp, actor or source, session reference, event type이 감사 로그에 저장되어야 한다
**And** 사진 자산 원본 진실과는 분리된 저장소를 사용해야 한다

**Given** 운영자나 소유자가 기록을 검토할 때
**When** 세션 이력을 조회하면
**Then** 상태 전이, 개입 시도, 최종 결과를 구분해서 볼 수 있어야 한다
**And** 지점/세션 단위 회고 분석에 사용할 수 있어야 한다

**Given** 감사 로그 기준을 유지할 때
**When** 로그를 저장하거나 조회하면
**Then** current-session privacy boundary를 넘는 자산 참조는 포함되면 안 된다
**And** rollout, timing, intervention 이벤트를 함께 상관분석할 수 있어야 한다

### Story 5.4: 카메라 연결 상태 전용 진단 항목

**Status:** review

As a remote operator,
I want 카메라 연결 상태를 별도 진단 항목으로 보고 싶다,
So that false-ready 위험을 일반 오류 요약에 묻히지 않고 먼저 발견할 수 있다.

**Acceptance Criteria:**

**Given** 운영자 콘솔이 활성 또는 차단된 세션을 표시할 때
**When** 진단 정보를 렌더링하면
**Then** `카메라 연결 상태` 전용 항목이 보여야 한다
**And** 이 값은 host-normalized camera/helper truth에서 계산되어야 한다

**Given** 카메라나 helper가 disconnected, preparing, ready, degraded 상태일 때
**When** 운영자가 세션을 보면
**Then** 각 상태가 운영자 친화적 용어로 하나의 명시적 상태값으로 보여야 한다
**And** raw helper output이나 고객용 copy를 그대로 재사용하면 안 된다

**Given** false-ready 가능성을 진단할 때
**When** 카메라 연결 상태를 확인하면
**Then** 최신 helper freshness와 booth readiness truth의 관계를 해석할 수 있어야 한다
**And** 일반 오류 요약 안에 묻히지 않는 독립 진단 항목으로 유지되어야 한다

## Epic 6: 지점 배포와 롤백 거버넌스

브랜드/운영 측이 선택된 지점 집합에 대해 빌드와 승인된 프리셋 스택을 안전하게 배포하고 롤백할 수 있게 한다.

**Status:** done

### Story 6.1: 지점별 단계적 배포와 단일 액션 롤백

**Status:** done

As a owner / brand operator,
I want 선택한 지점 집합에만 빌드와 승인된 프리셋 스택을 배포 또는 롤백하고 싶다,
So that 모든 지점을 동시에 흔들지 않고 안전하게 운영 기준을 맞출 수 있다.

**Acceptance Criteria:**

**Given** 새 승인 빌드 또는 preset stack이 준비됐을 때
**When** rollout을 시작하면
**Then** 대상은 명시적으로 선택된 branch set이어야 한다
**And** branch set, target build, preset stack, approval timestamp, actor가 기록되어야 한다

**Given** 대상 지점에 활성 고객 세션이 있을 때
**When** rollout 또는 rollback이 해당 세션을 방해할 수 있으면
**Then** 해당 지점 전환은 지연되거나 거부되어야 한다
**And** 활성 고객 세션 중 강제 업데이트는 발생하면 안 된다

**Given** 롤백이 승인된 지점 집합에 대해 실행될 때
**When** 이전 승인 기준선으로 복귀하면
**Then** 각 지점은 승인된 로컬 설정을 유지한 채 마지막 승인 build와 preset stack으로 되돌아가야 한다
**And** active-session compatibility가 보호되어야 한다

### Cross-Cutting Release Truth Gate

- truth-critical stories는 automated pass만으로 제품 관점 `done`이 아니다.
- `hardware-validation-ledger`에 `Go`가 기록되기 전까지 해당 story는 `review` 또는 동등한 pre-close 상태에 머문다.
- booth `Ready`, preset-applied preview truth, `Completed`, preset publication truth는 hardware evidence 없이 release truth로 주장할 수 없다.
