C:\Code\Project\Boothy\reference\camerafunction (digiCamControl 기능 참고용)
C:\Code\Project\Boothy\reference\uxui_presetfunction (RapidRaw 기능/디자인 참고용)

위 2개 오픈소스의 기능을 결합하여 **하나의 Windows 앱(Tauri + React)**으로 신규 개발합니다.
- WPF UI 사용 금지 (카메라 앱 UI는 Tauri로 새 디자인)
- 전체 UI/스타일은 RapidRAW 디자인 컨셉과 통일

## 핵심 컨셉(통합 UX)

1) 세션 시작 시 세션 이름 입력 → 세션 폴더 1개를 활성화(세션 목록은 그 폴더만 표시)
2) 카메라 테더링 촬영 파일이 PC로 **전송 완료**되면 자동으로 세션 폴더에서 감지/가져오기
3) 가져온 최신 사진은:
   - RapidRAW의 **가운데 메인 이미지 영역**에 즉시 표시
   - 하단/리스트의 **세션 썸네일(폴더 내 이미지 나열)**에 즉시 추가
4) customer 모드에서 선택한 **PRESET**은 **새로 들어오는 사진에만 자동 적용**
   - 프리셋 변경 시점 이후 신규 사진부터 새 프리셋 적용
   - 이전 사진은 기존 프리셋 유지(레트로액티브 변경 없음)

## 모드 정책(중요)

- 기본 모드: customer
- 전환: 토글 → 비밀번호 → admin 모드 진입
- customer 모드에서는 **비활성화가 아니라 “숨김”** 처리 (admin 모드에서 노출)

### Customer Mode에서 노출(필수)

- PRESET 선택
- 촬영(셔터)
- 썸네일 선택(세션 폴더 내 사진 선택)
- Export: RapidRAW의 **“Export image” 버튼만** 노출
- 삭제(delete selected files)

### Admin Mode에서 추가 노출(전체 기능 범위)

- digiCamControl의 **모든 카메라 기능** (부스 운영 필수 기능 외의 나머지 전부)
- RapidRAW의 고급 편집/설정/사이드바 기능(고객에게 불필요한 기능 포함)
- 회전(좌/우 rotate)

## 삭제/숨김/수정 목록(세부)

### 공통(항상 적용)

- 썸네일/프리뷰 사진 배열에 표시되는 오버레이 텍스트(F, Iso, E, FL, EB, histogram)는 제거(사진만 보이게)

### 카메라 기능(digiCamControl 참고) - Customer에서 숨김 / Admin에서 노출

- Advanced Properties: 메뉴 모두
- Mode, Iso, Shutter speed... 등 카메라 설정 사이드바 메뉴: 메뉴 모두
- (참고) 좌측 상단/기능 메뉴: Lv(live view), download photos, bracketing, time lapse, browse session, astronomy, multicamera control, connect with dslrdashboardserver, print 등

### 편집 기능(RapidRAW 참고) - Customer에서 숨김 / Admin에서 노출

- 오른쪽 사이드바: customer 모드에서는 `preset` + `Export image`만 남기고 나머지는 숨김
- 왼쪽 사이드바: 공간 축소(세션 이름 1개만 표시되므로 전체 폭의 사이드바 불필요) → 재디자인 필요

## 구현 가이드 (RapidRAW 기준 매핑, 초안)

이 섹션은 “무엇을 숨길지/남길지”를 RapidRAW 컴포넌트 단위로 매핑하기 위한 초안입니다. (구현 시 실제 컴포넌트/패널 구성에 맞춰 조정)

### 공통 제품 정책 (MVP)

- Boothy는 **오프라인/무계정** 앱이며, 기본 워크플로우에서 로그인/커뮤니티/클라우드/모델 다운로드 등 네트워크 의존 UI는 노출하지 않습니다.

### Customer Mode (최소 UI)

- **Right Panel (표시):** `PresetsPanel` + `ExportPanel`(단, 고객 모드에서는 “Export image” 버튼만 노출)
- **Right Panel (숨김):** `ControlsPanel`, `MetadataPanel`, `CropPanel`, `MasksPanel` 등 고급 편집/메타데이터 패널
- **Left Panel (재구성):** `FolderTree`/일반 폴더 브라우징 중심 UI는 숨기고, “활성 세션 폴더 1개”에 한정된 썸네일/리스트 중심으로 단순화
- **기타 (숨김):** `CommunityPage`, 설정/계정 관련 UI(존재 시) 전부 숨김

### Admin Mode (전체 기능)

- customer 모드에서 숨긴 카메라 고급 기능 + RapidRAW 고급 편집/설정 패널을 노출합니다.

### 메타데이터 오버레이 제거 (필수)

- 썸네일/프리뷰 영역에서 EXIF/히스토그램/촬영값(F/ISO/Exposure/FL/EB 등) 오버레이는 **표시하지 않음**(숨김/제거). 사진만 보이도록 유지합니다.
