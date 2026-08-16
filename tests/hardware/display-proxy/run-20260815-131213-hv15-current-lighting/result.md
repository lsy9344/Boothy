# HV-15 현재 조명 상태 재검증

- 실행일: 2026-08-15 (Asia/Seoul)
- 요청 조건: 조명·노출 조정 없이 현재 상태 유지
- 카메라: Canon EOS 700D
- display proxy: on / sample: off / source compare: off
- 세션: `session_000000000018cbdf04a181798c`
- 판정: **HV-15 Go (2026-08-16 제한 예외 승인 포함)**

## 결과

- 실촬영 5회 모두 RAW 저장, immutable proxy commit, 고객 화면 actual-present 성공
- Daylight 연속 3회 성공
- Mono Pop 전환 후 1회 성공, 이전 Daylight frame의 늦은 덮어쓰기 없음
- 최신 Mono Pop 삭제 성공: RAW/레일 제거, `request-forgotten`, viewer standby 확인
- 삭제 후 Mono Pop 재촬영 성공, viewer가 정상 복구
- 카메라 busy, RAW download timeout, display proxy failure 없음
- 세로 634x953 proxy를 1429x953 영역에 crop/upscale 없이 contain 표시

5개 actual-present latency는 7.765543~9.735243초, median 8.284043초, p95/max 9.735243초다. 느린 표본을 제외하지 않았다.

## 현재 조명 결과

- Daylight: 평균 luma 12.27/255, 16 미만 78.3%
- 삭제 전 Mono Pop: 평균 luma 7.80/255, 16 미만 84.6%
- 삭제 후 Mono Pop: 평균 luma 7.39/255, 16 미만 85.4%

사용자 요청대로 조명이나 노출을 변경하지 않았다. 화면 적합·프리셋 결속·삭제 복구는 통과했지만 촬영 결과는 이전 회차보다 더 어두웠다.

2026-08-16 Noah Lee는 이 밝기를 **품질 기준 통과가 아닌 제품 예외**로 허용했다. 원래 밝기 수치와 고객 품질 위험은 그대로 보존한다. 결정 기록: `product-decision-20260816.md`.

HV-14 운영 경로는 `raw-original + pinned darktable 5.4.1`로 확정됐다. 2026-08-16에 확인 가능한 환경 정보를 보완하고 읽을 수 없는 세부값 및 별도 metrics/blind review를 Story 7.4 한정 예외로 승인했다. `BOOTHY_DISPLAY_PROXY_MODE` 기본값은 `on`으로 전환하며, Story 7.5의 상주 렌더러 화질 검토는 별도로 수행한다.

## 게이트 해석

전체 세션 journal에는 Daylight와 Mono Pop이 함께 있고 삭제된 generation도 보존되어 있어 단일-preset/모든-asset-present 전용 checker 입력으로는 사용할 수 없다. 이 사실을 숨기지 않고 `full-session/`에 원본을 보존했다. 삭제 후 복구된 최종 Mono Pop generation을 `final-gate/`와 `session-evidence/`에 분리해 기계식 gate와 telemetry completeness gate를 통과시켰다.

2026-08-16 기본값 전환 후 Rust lib 151/151, `viewer_display` 35/35, 포맷, lint, HV-15 기계식 gate와 계측 완결성이 통과했다. 전체 Vitest의 2개 거버넌스 실패와 TypeScript build 실패는 기존 기준선 문제로 동일하게 남아 있으며 `regression/summary.md`에 분리 기록했다.
