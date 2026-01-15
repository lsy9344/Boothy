# Enhancement Scope and Integration Strategy

## Enhancement Overview

**Enhancement Type:** Integration with New Systems + New Feature Addition + UI/UX Overhaul + Role-based UI gating

**Scope:** RapidRAW(Tauri+React) 기반으로 Boothy 앱을 리브랜딩/변형하고, 카메라 촬영/전송 파이프라인을 통합하여 “세션 폴더(파일시스템)”를 중심으로 촬영→실시간 미리보기→프리셋 자동 적용→Export(JPG)까지 하나의 UX로 제공

**Integration Impact:** Major (카메라 통합/실시간 처리/세션 계약/권한 UI까지 포함)

## Integration Approach

**Code Integration Strategy:**
- **RapidRAW를 제품 베이스로 채택**하고, 신규 Boothy 기능은 “세션 모드(Session Mode)”로 추가하여 기존 편집/프리셋/익스포트 코어를 최대한 재사용합니다.
- 카메라 기능은 (A) **digiCamControl 기반 “Camera Sidecar Service”**로 분리(별도 프로세스)하고, Boothy(Tauri backend)가 IPC로 제어/상태 구독합니다.
- 촬영→편집 통합의 1차 경계는 **세션 폴더 계약**이며, “전송 완료 이후”에만 import 처리합니다.

**Database Integration:**
- MVP는 **전용 DB를 도입하지 않고**, 사진별 비파괴 편집 상태는 RapidRAW의 기존 **이미지 옆 `.rrdata` 사이드카**(ImageMetadata.adjustments)에 저장하여 “프리셋 적용/회전 등”을 영속화합니다(FR10/FR14). 세션 레벨 메타데이터(세션 이름/생성시각 등)가 필요하면 세션 폴더 루트에 `boothy.session.json`을 추가할 수 있습니다(선택).
- 앱 전역 설정(예: admin 모드 설정/기본 경로 등)은 OS별 AppData 영역에 설정 파일로 저장하고, 추후 필요 시 SQLite + 마이그레이션(CR2)로 확장합니다.

**API Integration:**
- 앱 내부는 **React(UI) ↔ Tauri(Rust backend)** 간 command/event 패턴을 사용하고,
- **Tauri backend ↔ Camera Sidecar Service**는 **버전드 IPC 계약**(예: JSON 메시지 + 명시적 error code/telemetry, correlation id)을 사용합니다.
- 외부 네트워크 의존은 최소화하고(오프라인), 진단/로그는 로컬 파일로 남깁니다(NFR7/NFR8).

**UI Integration:**
- UI는 RapidRAW 스타일을 기준으로, Boothy의 **촬영/세션 시작(세션 이름 입력)/모드 토글** 플로우를 추가합니다.
- customer 모드는 기본이며, customer에서는 “숨김” 원칙으로 고급 기능을 제거하고, admin 모드에서만 전체 카메라 기능/고급 편집 기능을 노출합니다(FR15–FR19, `docs/design_concept.md`).
- customer-facing 썸네일/리스트/프리뷰에는 카메라 메타데이터 오버레이(F/ISO/Exposure/히스토그램 등)를 표시하지 않습니다(FR18, `docs/design_concept.md`).

## Compatibility Requirements

- **Existing API Compatibility:** RapidRAW 프리셋 정의/로딩/렌더 및 Export 결과의 호환성을 유지해야 합니다(CR1).
- **Database Schema Compatibility:** MVP는 DB 없이 시작하되, 도입 시 forward/backward-compatible 마이그레이션을 제공합니다(CR2).
- **UI/UX Consistency:** 신규 카메라 UX는 RapidRAW 디자인 시스템/컴포넌트 규칙을 따르며, customer/admin 모드 “숨김” 정책을 전역적으로 일관되게 적용합니다(CR3).
- **Performance Impact:** 파일 전송 완료 후 세션 반영 ≤ 1s / 프리셋 적용 프리뷰 ≤ 3s 목표를 위해, import/프리셋 적용/썸네일 생성/Export는 백그라운드 처리 및 큐잉/취소를 지원해야 합니다(NFR3/NFR4).

이 통합 경계(세션 폴더 + 카메라 사이드카 서비스)는 RapidRAW의 command/event 패턴 및 `.rrdata` 사이드카 저장 모델, 그리고 digiCamControl의 IPC/이벤트 기반 카메라 패턴을 존중하도록 설계합니다.
