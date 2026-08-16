# HV-15 실장비 재검증 결과

- 실행일: 2026-08-15 (Asia/Seoul)
- 승인 장비: Canon EOS 700D / 승인 부스 PC / 고객 모니터 `LG FULL HD`
- 실행 모드: `BOOTHY_DISPLAY_PROXY_MODE=on`, sample/source-compare off
- 실촬영 결과: 화면 게시 6/6 성공
- S1-S6: 모두 통과. S4 삭제 결함은 현장에서 수정한 뒤 재실행하여 통과
- 기계식 evidence gate: 통과
- telemetry completeness gate: 통과 (committed 1, terminal 1, qualifying 1)
- 최종 판정: **No-Go 유지**

기능 경로는 세로 원본을 확대하거나 자르지 않고 높이에 맞춘 contain 표시로 정상 동작했다. 프리셋 변경 뒤 이전 프레임이 덮지 않았고, 삭제 뒤 화면이 standby로 돌아갔으며, renderer 버전 불일치는 고유 사유로 게시 전에 차단됐다. 새 세션에서는 이전 세션 사진이 나타나지 않았다.

No-Go 사유는 세 가지다.

1. 선행 HV-14 source-route 결정이 No-Go다.
2. 촬영 결과가 심하게 어둡다. 최종 Daylight 표본 평균 luma 13.89/255, 16 미만 67.6%; Mono Pop 평균 luma 9.79/255, 16 미만 82.2%였다.
3. firmware/lens/card/cable/ICC/HDR 등 승인 환경 fingerprint가 완결되지 않았다.

S5 준비 과정에서 카메라 RAW 다운로드 timeout이 2회 발생했다. helper를 재시작한 뒤 S5 자체는 통과했으며, 실패 표본을 성능 집계에서 숨기지 않고 촬영 계층의 별도 장애로 기록한다.

`BOOTHY_DISPLAY_PROXY_MODE` 기본값은 off로 유지한다.
