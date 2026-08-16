# HV-13A validation result

## 판정

**Go**

현재 부스의 승인 고객 모니터인 `DISPLAY3` 1080p에서 pre-capture readiness, 실제 Canon 촬영, 관람 창 복구, 세션 교체, 실패 경로 차단을 모두 재검증했다. 이전 No-Go 원인이었던 관람 창 종료 후 hang/blank 재생성은 재현되지 않았다.

## 통과

- 승인 `DISPLAY3`에서 viewer가 1920×1080 fullscreen, DPR 1로 촬영 전에 준비됨
- 현재 session binding, listener/layout readiness, monitor profile, photo rectangle 계약이 `viewerReady: true`로 수렴
- photo rect 1428.859×952.562, DPR 1에서 required source 1429×953 올림 계산 일치
- Canon EOS 700D 실제 촬영 2회 성공; 서로 다른 request/capture correlation과 RAW 원본 보존 확인
- 첫 촬영과 관람 창 복구 후 촬영 모두 Daylight preview가 부스 레일에 표시됨
- 관람 창 강제 종료 후 새 epoch 2로 자동 재생성되고 다시 `viewer-ready`로 수렴
- 복구 후 촬영 버튼이 다시 활성화되고 두 번째 실제 촬영 성공
- 승인 모니터 이름 불일치 시 viewer 창이 임의 화면에 생성되지 않고 `monitor-unavailable`로 유지
- 위 실패 경로에서 카메라/helper가 ready여도 `viewer-preparing`, `canCapture: false`로 차단
- viewer WebView의 SPA 경로를 `/booth`로 이동해도 customer-safe blank만 표시되고 조작 요소는 0개
- 현재 세션을 연속 교체했을 때 revision 6→8로 증가하고 새 session binding으로 재수렴
- viewer 표면의 button/link/input은 모두 0개
- 관련 자동 검증: TypeScript 65건, Rust viewer 43건, Rust lib 13건, lint 통과

## 범위 메모

- 현재 프로덕션 승인 고객 모니터는 1080p `DISPLAY3`이다.
- 1440p/4K source-dimension 계산과 single-monitor fallback은 자동 테스트로 유지한다. 현재 연결된 4K 모니터는 portrait 운영 화면이며 승인 고객 모니터가 아니다.
- Story 7.1은 사진 표시 자체가 아니라 viewer readiness와 physical display-size 계약까지만 소유한다.
- 전체 TypeScript build에는 Story 7.1 범위 밖의 기존 오류가 남아 있으며 별도 정리가 필요하다.

