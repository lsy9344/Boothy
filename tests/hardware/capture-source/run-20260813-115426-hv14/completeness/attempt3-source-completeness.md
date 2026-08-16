# HV-14 source-comparison telemetry 완결성 결과

- 증거 경로: C:\Users\dltnd\Pictures\dabi_shoot\sessions\session_000000000018cb8b7655e66b2c\diagnostics\source-comparison.jsonl
- 전체 표본 행: 33
- 기대 route: camera-paired-jpeg, embedded-jpeg, windows-shell-thumbnail
- route당 기대: warm-up 5 + measured 30 = 35
- 최종 패키지 게이트: False

## Route별 집계

| route | 표본 행 | 승격 | 거부 | 성공률 | warm-up | measured |
| --- | --- | --- | --- | --- | --- | --- |
| camera-paired-jpeg | 11 | 0 | 11 | 0% | 5 | 6 |
| embedded-jpeg | 11 | 0 | 11 | 0% | 5 | 6 |
| windows-shell-thumbnail | 11 | 0 | 11 | 0% | 5 | 6 |

## AB/BA 배치

- randomizationSeed: 3103284341835994809
- AB block: 3
- BA block: 8

## 거부 사유 분포

| 사유 | 건수 |
| --- | --- |
| orientation-unsupported | 22 |
| absent | 11 |

## Correlation 근거

- groupID로 묶인 표본: 33
- 보조 correlation 표본: 0

## 판정

**FAIL** — 31건의 문제가 있습니다.

- route 'camera-paired-jpeg'의 표본 행이 . 기대값은 .
- route 'camera-paired-jpeg'의 measured 표본이 . 기대값은 30건입니다.
- route 'embedded-jpeg'의 표본 행이 . 기대값은 .
- route 'embedded-jpeg'의 measured 표본이 . 기대값은 30건입니다.
- route 'windows-shell-thumbnail'의 표본 행이 . 기대값은 .
- route 'windows-shell-thumbnail'의 measured 표본이 . 기대값은 30건입니다.
- 고유 requestId가 11개입니다. 기대 촬영 수는 .
- blockIndex 11 묶음이 없습니다.
- blockIndex 12 묶음이 없습니다.
- blockIndex 13 묶음이 없습니다.
- blockIndex 14 묶음이 없습니다.
- blockIndex 15 묶음이 없습니다.
- blockIndex 16 묶음이 없습니다.
- blockIndex 17 묶음이 없습니다.
- blockIndex 18 묶음이 없습니다.
- blockIndex 19 묶음이 없습니다.
- blockIndex 20 묶음이 없습니다.
- blockIndex 21 묶음이 없습니다.
- blockIndex 22 묶음이 없습니다.
- blockIndex 23 묶음이 없습니다.
- blockIndex 24 묶음이 없습니다.
- blockIndex 25 묶음이 없습니다.
- blockIndex 26 묶음이 없습니다.
- blockIndex 27 묶음이 없습니다.
- blockIndex 28 묶음이 없습니다.
- blockIndex 29 묶음이 없습니다.
- blockIndex 30 묶음이 없습니다.
- blockIndex 31 묶음이 없습니다.
- blockIndex 32 묶음이 없습니다.
- blockIndex 33 묶음이 없습니다.
- blockIndex 34 묶음이 없습니다.
