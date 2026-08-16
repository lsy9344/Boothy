# HV-14 source-comparison telemetry 완결성 결과

- 증거 경로: tests\hardware\capture-source\run-20260813-115426-hv14\session-evidence\diagnostics\source-comparison.jsonl
- 전체 표본 행: 105
- 기대 route: camera-paired-jpeg, embedded-jpeg, windows-shell-thumbnail
- route당 기대: warm-up 5 + measured 30 = 35
- 최종 패키지 게이트: True

## Route별 집계

| route | 표본 행 | 승격 | 거부 | 성공률 | warm-up | measured |
| --- | --- | --- | --- | --- | --- | --- |
| camera-paired-jpeg | 35 | 0 | 35 | 0% | 5 | 30 |
| embedded-jpeg | 35 | 0 | 35 | 0% | 5 | 30 |
| windows-shell-thumbnail | 35 | 0 | 35 | 0% | 5 | 30 |

## AB/BA 배치

- randomizationSeed: 2404820328541599495
- AB block: 15
- BA block: 20

## 거부 사유 분포

| 사유 | 건수 |
| --- | --- |
| orientation-unsupported | 70 |
| absent | 35 |

## Correlation 근거

- groupID로 묶인 표본: 105
- 보조 correlation 표본: 0

## 최종 증거 패키지

- run 디렉터리: C:\Code\Boothy\tests\hardware\capture-source\run-20260813-115426-hv14
- 확인 항목: environment, capability, correlation, quality, aggregate, decision

## 판정

**FAIL** — 1건의 문제가 있습니다.

- quality JSON 증거가 없거나 비었습니다: C:\Code\Boothy\tests\hardware\capture-source\run-20260813-115426-hv14\quality\manifest.json
