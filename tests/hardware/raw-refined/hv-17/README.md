# HV-17 — RAW 정밀본 무중단 교체 (Story 7.6)

이 디렉터리는 Story 7.6의 실장비 evidence 루트다.
`viewer-present/`, `capture-source/`, `display-proxy/`, `resident-renderer/`와 **섞지 않는다.**

## 두 gate는 서로 독립이다

AC 5가 요구하는 것은 하나의 판정이 아니라 **둘**이다.

| gate | 무엇을 증명하는가 | 무엇을 증명하지 않는가 |
| --- | --- | --- |
| **HV-17A** | scheduler span, 우선순위 실제 적용 순서, 용량 상한, 취소·병합·process tree | 화면에 무엇이 보였는가 |
| **HV-17B** | proxy→refined 전환의 결정/상태 수준 무결성, tier 정당성, 사람 탐지 검사 | 스케줄러가 어떻게 돌았는가 |

**한쪽 gate가 다른 쪽 결과를 물려받을 수 없다.** `check-raw-refined-evidence.ps1`은
두 verdict를 각자의 입력만으로 계산하고 따로 출력한다. 자체 검증
(`test-check-raw-refined-evidence.ps1`)이 그 독립성을 케이스로 고정한다.

## 이 gate가 제공하지 않는 것

- **120fps+ 물리 monitor frame**
- **compositor → photon 오프셋**
- **frame 단위 zero-defect 판정**

→ 전부 **HV-18B 소유**다 (2026-08-12 correct-course). Story 7.2/HV-13B가 이 장비 부재로
회차를 잃었고, 같은 실수를 반복하지 않기 위해 HV-17은 그 장비를 기다리지 않는다.

**재개 조건:** HV-18B에서 표시 종단점 귀책의 전환 결함(blank, spinner, stale, wrong capture,
crop jump, scale jump, tier downgrade)이 발견되면 **Story 7.6은 `review`로 되돌아가고
HV-17은 `No-Go`가 된다.**

## 회차 디렉터리 구조

```
run-<timestamp>-hv17/
├── environment.md                              # 아래 environment.md 템플릿을 채운다
├── result.md                                   # 사람이 쓰는 판정문
├── session-evidence/
│   ├── display/generations.jsonl               # 촬영당 generation 2개
│   └── diagnostics/viewer-present.jsonl        # generation당 terminal 행 1개
├── scheduler/
│   ├── burst.json                              # 연속 촬영 5회 이상, P0 탈락 수, 큐 대기 분포
│   └── cancel-rounds.jsonl                     # 취소 회차 4종
├── logs/
│   └── taskkill-<round>.log                    # **taskkill 실행 결과 로그 원본**
├── transition/
│   ├── zero-report.json                        # 전환 결함 zero 보고
│   └── swap.mp4                                # 일반 속도 화면 녹화
├── tier-justification/
│   ├── verdict.json                            # judgeTierJustification() 결과 그대로
│   └── pairs/                                  # proxy/refined 원본 쌍과 ROI 정의
└── detection-trial/
    ├── trials.csv                              # 시행별 원자료
    └── signoff.md                              # 관찰자 3명 실명 + Noah Lee 서명
```

## 실행 순서

```powershell
# 1. 계측 완결성 (촬영당 generation 2개, 각각 terminal 행 1개)
../../viewer-present/hv-13b/check-telemetry-completeness.ps1 `
  -SessionEvidenceDir ./run-<timestamp>-hv17/session-evidence

# 2. HV-17A / HV-17B 기계식 gate
./check-raw-refined-evidence.ps1 -RunRoot ./run-<timestamp>-hv17 -OutFile ./run-<timestamp>-hv17/gate.json

# 3. gate 자체 검증 (회차 전에 한 번)
./test-check-raw-refined-evidence.ps1
```

## PASS는 `Go`가 아니다

기계식 gate가 통과해도 아래는 사람이 한다.

- 일반 속도 화면 녹화 검토
- 전환 탐지 검사 실행과 관찰자 3명 실명 기록
- **최종 승인자: Noah Lee**

## `tier-justification/verdict.json`

`src/quality-metrics/tier-justification.ts`의 `judgeTierJustification()` 결과를 **그대로** 넣는다.
값을 손으로 고치지 않는다. 그 함수가 강제하는 것:

- **detail 축 (tier의 존재 조건):** `median MTF50(refined) ≥ 1.10 × median MTF50(proxy)`,
  역행 0건, slanted-edge 쌍 최소 9개(차트 3장 × 승인 preset 3개)
- **look 축 (AC 4의 전환 결함 판정):** `median ΔE00 ≤ 3`, `p95 ≤ 8`, clipping 증가 `≤ 2%p`
- 크기가 다른 쌍은 지표가 아니라 리샘플러를 재게 되므로 제외되고, 제외 사실이 남는다
- 저노출 표본은 look 축에서 제외되고, 제외 사실이 남는다
- **미실행은 통과가 아니다.** 측정 불가 항목은 `unmeasuredPairs`로 남는다

## `detection-trial/trials.csv`

**선호 검사가 아니라 탐지 검사다.** 묻는 것은 "이 룩이 맞는가"가 아니라 **"교체가 안 보였는가"**다.

| 열 | 값 |
| --- | --- |
| `observer` | 관찰자 식별자 (실명은 `signoff.md`에) |
| `trial` | 1..20 |
| `condition` | `swap` (10회) 또는 `control` (10회). **순서는 무작위** |
| `reportedChange` | `true` / `false` |
| `reportedAtMicros` | 변화를 느꼈다고 보고한 시점 (선택) |

**통과 기준: 교체 시행의 탐지율 ≤ 대조군 오탐율 + 10%p.**

대조군 없이 얻은 "아무도 못 봤다"는 통과로 적지 않는다. 무엇과 비교했는지가 없으면 숫자가 아니다.
5명 × 30 시행 형식은 쓰지 않는다 — 탐지 검사에 그 규모는 불필요하고, Story 7.5가 패널 미확정으로
AC를 닫지 못한 이유가 규모였다.

## `scheduler/burst.json`

```json
{
  "captureCount": 5,
  "p0DroppedCount": 0,
  "queueWaitMicros": { "p50": 0, "p95": 0, "max": 0 },
  "queueWaitShareOfEndToEnd": 0.0
}
```

`queueWaitShareOfEndToEnd`는 **남은 지연의 소유를 Story 7.8로 넘기기 위한 값이다.**
HV-16이 실장비 종단 median 8284 ms 중 display 렌더가 3716 ms임을 지목했고, 렌더를 0으로 만들어도
4568 ms가 남는다. 그 4568 ms 중 큐 대기가 얼마인지 여기서 확정한다.

## `scheduler/cancel-rounds.jsonl`

한 줄이 한 취소 회차다. 네 회차 전부 필요하다: `delete`, `session-replaced`,
`viewer-epoch-changed`, `newer-capture`.

```json
{"round":"delete","cancelRequestedAtMicros":0,"cancelCompletedAtMicros":0,"killOutcome":"exit=0","directKillFallback":false,"cancelOrphanCount":0,"taskkillLogPath":"logs/taskkill-delete.log","finalRenderCompleted":true}
```

- **`cancelOrphanCount`가 없는 것과 0인 것은 다르다.** 없으면 gate가 실패한다
- `finalRenderCompleted`가 `false`면 Story 3.2의 완료 진실이 깨진 것이다.
  **display lane이 급하다는 이유로 final을 취소하면 고객이 결과물을 못 받는다**
