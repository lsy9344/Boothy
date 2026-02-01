# Epic 4: Real Camera Hardware Integration & Field Validation

- Epic ID: epic-4
- Type: Brownfield enhancement (hardware integration + E2E validation)
- Baseline: Epic 1~3 완료 상태(세션/프리셋/내보내기/스토리지 가드레일) + Camera Sidecar(Mock) + IPC 계약
- Parent PRD: `docs/prd.md` (FR5, FR6, FR20, FR21)

## Goal

Boothy에서 **실제 Canon 카메라를 USB로 연결**해,
앱 내에서 촬영(trigger) → PC로 전송 완료(transfer complete) → 세션 `Raw/`로 저장 → 자동 ingest → 프리셋 자동 적용(신규 사진만) → 내보내기까지
**실하드웨어 기준 E2E(현장형)로 검증 가능한 제품 상태**를 만든다.

## Why this Epic exists

현재 구현은 카메라 파이프라인을 “통합 가능한 형태”로 먼저 닫기 위해,
Sidecar가 `MockCameraController`로 동작(하드웨어 없이도 `photoTransferred` 이벤트를 발생)하도록 설계/구현되어 있다.
따라서 “카메라 연결”은 단순 테스트 단계가 아니라, **실하드웨어 연동 구현 + 패키징/운영 검증** 범위로 분리한다.

## Scope (MVP)

- Sidecar는 `reference/camerafunction/`의 검증된 레퍼런스(digiCamControl 등) 기반으로 구현한다. Canon SDK 통합을 새로 작성하지 않는다.
- Sidecar는 `mock`/`real` 모드를 지원하고, `real` 모드에서는 Canon 카메라 연결/테더링(촬영 이벤트 감지)/파일 다운로드를 수행한다.
- 촬영 트리거는 **외부 리모컨(셔터 릴리즈)** 과 **Boothy UI 촬영 버튼(Customer mode)** 을 모두 지원한다.
- 전송 완료(transfer complete) 이후에만 `Raw/`에 파일이 확정되며, Boothy는 파일 안정화 확인 후 자동 ingest 및 프리셋을 자동 적용한다(신규 유입 사진에만, 비레트로액티브).
- 고객 모드에서 세션 종료 시간 알림(카운트다운/경고)과 내보내기(Export)를 사용할 수 있어야 한다(세부 플로우는 Epic 2와 연동).
- 현장 장애 시나리오(미연결/전송 실패/중간 분리 등)는 고객 친화 한국어 메시지 + 관리자 진단(로그/코드)로 대응한다.
- 오프라인 정책: 기본 워크플로우에서 네트워크 호출 금지.


## Non-Goals (이번 Epic에서 하지 않음)

- digiCamControl “전체 기능 동등” 수준의 모든 고급 카메라 설정 UI/기능
- 멀티 카메라/멀티 세션 동시 처리
- 공개 배포(라이선스/EDSDK 재배포 게이트 해소 전까지 내부 테스트/배포만)

## Story Sequence (dependency-ordered)

1. **Story 4.1: Real Camera Capture + Transfer-Complete → Auto Ingest → Preset Apply → Export (E2E)**
   - Real 모드 Sidecar 구현(최소 기능) + 외부 리모컨 촬영(Boothy UI 촬영 버튼은 MVP 비필수) + 핵심 E2E 검증

2. (Optional) **Story 4.2: Admin Camera Diagnostics & Reconnect Workflows**
   - 현장 대응을 위한 진단 패널/재연결 UX 강화(최소 운영 도구)

3. (Optional) **Story 4.3: Hardware Compatibility Matrix + Regression Campaign**
   - 지원 카메라/케이블/드라이버 조합별 테스트 캠페인 문서화 + 게이트 증적

## Definition of Done

- [ ] Canon 카메라 실연결 상태에서 “세션 시작 → 촬영 → 자동 유입/프리셋 적용 → 내보내기”가 재현 가능
- [ ] 전송 실패/분리/재연결 시나리오에서 앱이 크래시 없이 복구 가능(고객 모드 안전 메시지 + 관리자 진단 로그)
- [ ] `docs/qa`에 실하드웨어 테스트 증적(절차/결과/로그 위치)과 게이트(PASS/CONCERNS) 기록

