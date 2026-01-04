# User Interface Enhancement Goals

## Integration with Existing UI

- RapidRAW 업스트림의 UI 구조(React + TS + Tailwind)와 컴포넌트 레이어를 유지하고, “Customer/Admin Mode + 세션 상태머신”은 **최상위 App Shell 레벨에서 오케스트레이션**한다(예: `App.tsx` 중심).
- Customer Mode 화면(Idle/Setup/Capture/ExportLock/Complete/Reset)은 “새로운 화면 집합”이지만, 이미지 미리보기/필름스트립/패널 등은 가능한 한 **기존 컴포넌트를 재사용**하고 “노출/권한/네비게이션 제한”으로 구현한다.
- Backend와의 연동은 기존 RapidRAW 패턴(tauri `invoke()` + progress/event emit)을 따르되, Customer Mode에서는 이벤트/에러를 **짧고 단일 행동(도움 요청)** 중심으로 표현한다.
- “UI Lock-down 레벨(0~2)”은 Tauri 창/단축키/시스템 인터랙션 제한(가능 범위)에 맞춰 옵션화하고, UI에는 현재 레벨/제한 상태를 운영자(Admin)에게만 노출한다.

## Modified/New Screens and Views

- 신규(고객): `Idle`(예약 입력/상태 점검), `Setup`(예약 확인 + 필터 선택 + 시작), `Capture`(큰 타이머 + 라이브뷰/최근 사진 + 촬영/끝내기), `ExportLock`(진행률 + 도움 요청), `Complete/Reset`(수령 안내 + 자동 초기화 카운트다운)
- 신규(관리자): `Admin PIN`(진입), `Camera Settings & Lock Policy`, `Preset Management`(정렬/미리보기), `Export Rules`, `Diagnostics/Logs`, `Retention/Cleanup Policy`
- 수정(공통): App Shell(모드 전환/상태머신/라우팅), Export 진행 UX(ExportLock), 오류/헬프 플로우, 세션 폴더 기반 네비게이션 제한(고객)
- 수정(업스트림 기반): 라이브러리/필름스트립/에디터 화면에서 Customer Mode 전용 “컨트롤 최소화/패널 숨김/모달 화이트리스트” 적용

## UI Consistency Requirements

- Tailwind 기반 스타일/레이아웃/색상 토큰을 유지하고, 새 UI는 가능한 한 기존 컴포넌트/패턴(버튼, 모달, 패널 구조)을 재사용한다.
- Customer Mode는 “큰 터치 타깃 + 최소 텍스트 + 명확한 진행 상태(타이머/Export/큐 잔량)”를 표준으로 한다.
- 상태 전환은 “알림”이 아니라 **강제 전환**으로 구현하며(타이머 만료 → ExportLock), 고객이 다음 행동을 판단하지 않도록 한다.
- 에러는 고객에게 “짧고 구체적인 문장 + 도움 요청 1가지 액션”만 제공하고, 상세 원인/로그/재시도는 Admin 전용으로 제공한다.

---
