# Sprint Change Proposal - 2026-03-28 02:37:25 +09:00

## 1. 이슈 요약

- Trigger Story: Story 1.6 `실카메라/helper readiness truth 연결과 false-ready 차단`
- 발견된 문제: 현재 Story 1.6 문서가 실제 `Ready` 진실을 여는 최소 helper baseline 범위와, 실제 촬영 round-trip 및 RAW handoff 범위를 함께 품고 있어 스토리 닫힘 기준이 흐려져 있었다.
- 핵심 문제 진술: `canon-helper.exe` 프로젝트 골격, host spawn/health, `helper-ready`, `camera-status`만으로 닫혀야 할 readiness truth story와 `request-capture`, `capture-accepted`, RAW download, `file-arrived`, capture correlation 같은 실제 촬영 round-trip 책임이 한 스토리에 섞여 있었다.

## 2. 영향 분석

### Epic 영향

- Epic 1은 유지한다.
- Story 1.6은 readiness truth 최소 helper baseline story로 재정의한다.
- Story 1.7을 추가해 실제 capture round-trip과 RAW handoff correlation 책임을 분리한다.

### 산출물 영향

- `_bmad-output/planning-artifacts/epics.md`
  - Story 1.6 acceptance criteria를 readiness-only 범위로 조정
  - Story 1.7 신규 추가
- `_bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md`
  - 상태를 `review`에서 `in-progress`로 재조정
  - 실제 helper exe 없이는 닫히면 안 되는 항목만 남기도록 task/notes 정리
  - 실제 촬영 round-trip 책임을 Story 1.7로 분리 명시
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - Story 1.6 -> `in-progress`
  - Story 1.7 -> `backlog`

### PRD / Architecture / UX 영향

- 현재 변경은 backlog/story boundary 정리에 가깝다.
- PRD, Architecture, UX 본문 수정은 이번 범위에서 필수는 아니다.
- 현재 문서 집합만으로도 구현 우선순위와 닫힘 기준은 충분히 정리된다.

## 3. 권장 접근

- 선택안: Option 1 `Direct Adjustment`
- 이유:
  - 이미 확보된 readiness hardening과 실제 helper exe baseline 작업을 분리해 관리 난이도를 낮춘다.
  - Story 1.6을 더 정직한 상태(`in-progress`)로 되돌려 false-ready 관련 release 판단을 보수적으로 유지한다.
  - 실제 촬영 round-trip을 Story 1.7로 분리하면 추후 helper 프로토콜 구현과 파일 handoff 검증을 독립적으로 추적할 수 있다.
- 범위 판단: Moderate
  - 코드 롤백은 필요 없지만 backlog 재정렬과 story closure 기준 수정이 필요하다.

## 4. 상세 변경 제안

### Story 1.6

OLD:
- helper readiness와 실제 capture round-trip 책임이 함께 있었다.
- 문서 상태가 `review`여서 실제 helper exe 부재와 충돌했다.

NEW:
- `canon-helper.exe` 프로젝트 골격
- host spawn/health/recovery
- `helper-ready`, `camera-status`
- freshness, disconnect, reconnect, false-ready 차단
- 상태: `in-progress`

Rationale:
- 1.6 제목과 acceptance criteria는 readiness truth 쪽에 더 가깝고, 실제 helper exe가 없으면 닫히면 안 된다.

### Story 1.7

NEW:
- `request-capture`
- `capture-accepted`
- RAW download
- `file-arrived`
- in-flight capture guard
- capture correlation

Rationale:
- 실제 촬영 round-trip은 컨텍스트가 커서 readiness story와 분리할수록 구현/검증/회고가 쉬워진다.

## 5. 구현 핸드오프

- Product / Scrum:
  - Story 1.6과 1.7 경계를 기준으로 backlog와 sprint 상태를 계속 유지
- Development:
  - 1.6에서는 실제 helper baseline과 readiness truth만 닫기
  - 1.7에서 실제 capture round-trip 구현 이어받기
- Validation:
  - 1.6은 HV-02, HV-03, HV-10 evidence 없이는 닫지 않음
  - 1.7은 실제 capture round-trip evidence 확보 전까지 open 유지

## 6. 승인 메모

- 사용자 직접 지시로 변경 범위와 분할 기준이 명확히 승인된 상태로 간주한다.
- 본 제안은 2026-03-28 작업 반영 기록으로 남긴다.
