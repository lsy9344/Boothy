# HV-13A validation result

## 판정

**No-Go**

승인 1080p 모니터의 정상 준비와 여러 차단 경로는 확인됐지만, 관람 창 종료 후 재생성이 흰 화면에서 멈춰 운영 복구 조건을 만족하지 못했다.

## 통과

- 승인된 `DISPLAY3` 1080p/DPR 1에서 현재 세션 binding, listener/layout readiness, `viewerReady: true` 확인
- 승인 상태에서 부스가 `Ready`이고 촬영 버튼이 활성화됨
- 실제 photo rect 1428.859×952.562 CSS, DPR 1에서 필요 source 1429×953으로 올림 계산 일치
- 존재하지 않는 모니터 이름에서 `monitor-unavailable` 및 촬영 차단
- `DISPLAY1` 1440×2560/DPR 1.5에서 `unapproved-profile` 및 촬영 차단
- heartbeat 중단 후 `stale-report`로 촬영 차단, viewer route 복귀 후 `viewer-ready` 재수렴
- 관람 화면에 고객 조작 요소나 기술 진단 정보 없음
- 관람 창 종료 직후 `viewer-window-closed`로 즉시 촬영 차단

## 실패

- 닫힌 관람 창에 `ensure_viewer_window`를 호출하면 호출이 30초 이상 반환되지 않음
- 새 관람 창은 생성되지만 흰 화면에 머물며 listener/layout readiness로 재수렴하지 않음
- 따라서 새 viewer epoch와 정상 복구 증거를 만들 수 없음

## 미실행/미완료

- 단일 모니터 `single-monitor-fallback` 물리 검증
- 승인 1440p 및 4K profile 검증
- 세션 교체 직후 `session-mismatch`와 재수렴 검증
- 고객 모니터가 실제 지정 화면에 배치됐음을 보여주는 외부 사진
- ICC/HDR 및 helper SDK identifier fingerprint

## 재검증 조건

관람 창 재생성 hang/blank 문제를 수정하고 자동 검증·코드 리뷰를 통과한 뒤 동일 장비에서 HV-13A 전체 시나리오를 다시 실행한다.

