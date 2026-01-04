# Epic and Story Structure

## Epic Approach

**Epic Structure Decision**: 단일 Epic (Single Epic) — “Customer Mode + EOS 700D Tethering + Smart Export/Reset 운영 플로우”로 하나의 통합 Epic을 권장한다.

**Rationale**:

- Customer Mode 상태머신(Idle/Setup/Capture/ExportLock/Reset), 테더링(EDSDK), Smart Export 큐, ExportLock 게이트, 프라이버시 리셋은 **서로 강하게 결합된 end-to-end 운영 플로우**로, 각각을 독립적으로 “완료” 정의하기 어렵다.
- 핵심 리스크가 “통합 지점(Frontend↔Backend 커맨드/이벤트 계약, 백그라운드 작업/리소스 경합, 세션 상태 전이)”에 집중되어 있어, Epic을 나누면 **부분 최적화/통합 지연**으로 오히려 일정·리스크가 증가할 가능성이 높다.
- 대신 단일 Epic 내부를 “기반(상태머신/저장소/진단) → 테더 → 프리셋/보정 적용 → Smart Export → ExportLock/Complete/Reset → Admin/운영 정책”처럼 **리스크 최소화 순서로 Story를 세분화**하는 접근이 브라운필드에 적합하다.

Based on my analysis of your existing project, I believe this enhancement should be structured as **single epic** because it is one tightly coupled, end-to-end kiosk workflow that must ship together (state machine + tether + export gate + reset). Does this align with your understanding of the work required?
Confirmed by user: 2026-01-02

---
