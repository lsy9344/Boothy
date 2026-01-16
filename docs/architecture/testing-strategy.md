# Testing Strategy

## Integration with Existing Tests

**Existing Test Framework:** 현재 RapidRAW/레퍼런스 코드에서 “표준 자동화 테스트 프레임워크”가 명확히 구성된 흔적은 제한적입니다(프론트 테스트 러너/백엔드 `#[test]`가 일반적으로 발견되지 않음). 따라서 MVP에서는 “테스트 프레임워크를 즉시 도입”하기보다, **품질을 보장하는 통합 시나리오(수동+자동 가능한 형태)**를 우선 정의합니다.

**Test Organization:** 테스트는 “컴포넌트 경계” 단위로 분류합니다.
- UI(React): customer/admin 숨김 정책, 세션 시작 UX, 촬영 버튼, 신규 사진 자동 선택
- Backend(Rust): 세션 폴더 생성 규칙, 파일 안정화(import 확정) 로직, `.rrdata` 스냅샷 저장
- Sidecar(.NET): 카메라 연결/촬영/전송 완료 이벤트, IPC 계약 준수
- End-to-End: capture→transfer→import→preset→export 흐름

**Coverage Requirements:** 정량 커버리지보다, PRD의 고위험 요구(FR6–FR10, FR16–FR20, NFR3–NFR6)를 “시나리오 기반”으로 반드시 검증하는 것을 품질게이트로 둡니다.

## New Testing Requirements

### Unit Tests for New Components

- **Framework:** Rust `#[test]`(순수 로직), C#(가능하면 xUnit/NUnit), 프론트는 필요 시 최소한의 컴포넌트 테스트 도입
- **Location:** `apps/boothy/src-tauri/src/**`(단위 로직), `apps/camera-sidecar/**`(IPC/프로토콜), 프론트는 `apps/boothy/src/**`
- **Coverage Target:** “핵심 로직(세션명 생성/파일 안정화/스냅샷 저장)”은 단위 테스트로 고정, UI는 smoke 수준부터 시작
- **Integration with Existing:** 기존 RapidRAW 핵심 처리 파이프라인 자체를 단위 테스트로 전면 커버하려 하지 않고, Boothy가 추가한 경계/정책 로직에 집중합니다.

### Integration Tests

- **Scope:** Tauri backend ↔ filesystem ↔ sidecar IPC 계약의 통합
- **Existing System Verification:** RapidRAW의 “폴더 선택→이미지 리스트→메인 프리뷰→Export” 흐름이 Boothy 세션 모드에서도 유지되는지 확인(CR1)
  - **New Feature Testing:**
  - 세션 시작(세션 이름) → 폴더 생성/열기/중복 처리(suffix 등) 적용
  - sidecar destination이 `Raw/`로 설정되는지
  - 파일 전송 완료 이벤트 또는 watcher로 신규 사진이 ≤1s 내 리스트에 반영되는지(NFR3)
  - 신규 사진에만 프리셋 스냅샷이 적용되는지(FR8/FR9/FR10)
  - Export가 `Jpg/`로 저장되고 customer 모드에서 고급 옵션이 숨김인지(FR12/FR17)
  - 카메라 연결/촬영/전송 실패 시 에러가 표면화되고 앱이 계속 동작하는지(FR20)

### Regression Testing

- **Existing Feature Verification:** Boothy 변경으로 인해 RapidRAW의 편집/프리셋/Export 결과가 달라지지 않는지(동일 입력/동일 preset에 대한 결과 비교)(CR1)
- **Automated Regression Suite:** MVP는 “핵심 시나리오 자동화”를 목표로 하고, 무거운 GPU 결과 비교는 초기에는 checksum/메타데이터 기반 또는 샘플 수동 검증으로 시작합니다.
#### Manual Testing Requirements (MVP Gate)

1) customer 모드 기본 진입, admin 토글+비밀번호, 숨김 정책 확인
2) 세션 생성/중복 규칙 확인, `Raw/`/`Jpg/` 생성 확인
3) 촬영 → 전송 완료 후 자동 반영/자동 선택, 프리셋 자동 적용 + 썸네일 오버레이 미표시(FR18) 확인
4) 프리셋 변경 후 신규 사진만 영향, 이전 사진 불변 확인
5) Export가 `Jpg/`로 생성, 삭제/회전(admin) 반영 확인
6) 카메라 분리/전송 실패 시 에러 표시 + 기존 사진 탐색/Export 가능 확인
7) 오프라인(네트워크 차단)에서도 core flow 동작하며, 로그인/클라우드 기능이 기본 동작에 관여하지 않는지 확인(NFR7)
