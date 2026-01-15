# Next Steps

## Story Manager Handoff

아래 프롬프트를 Story Manager(PO/SM)에게 전달해 첫 스토리를 생성하세요:

> `docs/architecture.md`와 `docs/prd.md`를 기준으로 Boothy MVP 통합을 위한 첫 스토리를 작성해 주세요.  
> 핵심 통합 경계는 (1) 세션 폴더 계약: `%USERPROFILE%\\Pictures\\dabi_shoot\\<session>\\{Raw,Jpg}` (세션명=`sessionName` 입력, 폴더 충돌 시 `_YYYY_MM_DD_HH` suffix 또는 기존 세션 열기) (2) RapidRAW 기반 앱에 “세션 모드” 추가 (3) 카메라 기능은 digiCamControl 기반 headless sidecar + Named Pipe IPC 입니다.  
> 첫 스토리는 “세션 생성/활성화 + Raw/Jpg 폴더 생성 + RapidRAW 현재 폴더를 Raw로 고정 + 신규 파일(수동 드롭/테스트 파일) 감지 시 자동 refresh/자동 선택”까지를 범위로 하고, NFR5(무결성)와 NFR3(실시간) 검증 포인트를 포함해 주세요.  
> customer/admin 모드 정책은 UI 숨김 중심이며, 우회 호출 방지는 MVP 범위 밖입니다.

## Developer Handoff

개발자가 바로 착수할 수 있도록, 구현 순서를 다음처럼 권장합니다:

1. **앱 베이스 승격:** `reference/uxui_presetfunction`을 `apps/boothy`로 승격하고(리브랜딩 포함), 빌드/실행 경로를 Boothy 기준으로 고정
2. **세션 매니저:** 세션 폴더 생성/열기 규칙(`sessionName` sanitize + 충돌 처리) + `Raw/`/`Jpg/` 생성 + RapidRAW 폴더 고정
3. **파일 감지→UI 반영:** `Raw/` 신규 파일 안정화 감지 → `.rrdata` 생성(프리셋 스냅샷) → 이미지 리스트 refresh + 자동 선택
4. **프리셋 스냅샷:** “현재 프리셋”을 저장하고, 신규 사진에만 적용(FR8–FR10)
5. **sidecar 통합:** 카메라 sidecar(IPC) 연결/상태/촬영/전송 완료 이벤트까지 확장
6. **모드/숨김:** customer/admin UI 숨김 정책 적용 및 UX 마감
