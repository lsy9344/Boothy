# HV-16 — 상주 display renderer 검증 (Story 7.5)

**판정: `Technology No-Go`.** 상세는 [`decision.md`](decision.md).

## 이 디렉터리

| 경로 | 내용 |
| --- | --- |
| [`decision.md`](decision.md) | Go/No-Go 판정, 근거, production 기본값, exact fallback, 잔여 위험 |
| [`t0-input-premise.md`](t0-input-premise.md) | T0 입력 전제, 후보 결정표, 남은 지연 구간과 소유 Story |
| [`environment.md`](environment.md) | PC/GPU/driver/WebView2/corpus/알 수 없는 값 |
| `baseline/darktable-oneshot-latency.csv` | pinned darktable one-shot 420회 원자료 |
| `baseline/wic-cr2-decode.csv` | Windows WIC direct CR2 디코드 72회 원자료 |
| `baseline/summary.json` | 위 두 원자료의 집계와 유도값 |
| `capability/engine-contract.json` | 엔진 신원, allowlist, 거절 사유 코드, 채택 gate |
| `parity/wic-vs-darktable-neutral.json` | 6쌍 parity 원자료 (중립 렌더 기준) |
| `parity/wic-vs-darktable-preset.json` | 18쌍 parity 원자료 (preset 렌더 기준) |
| `tools/hv16-parity.test.ts` | parity 회차 실행기 (매니페스트 없으면 skip) |
| `tools/check-resident-evidence.ps1` | 패키지 완결성 기계 gate |
| `tools/test-check-resident-evidence.ps1` | gate 자체의 self-test |

## 한 문장 요약

상주 renderer로 얻을 수 있는 최대 이득은 **47 ms**(전체 렌더의 1.26%)인데,
NFR-003까지 메워야 하는 거리는 **5000 ms 이상**이다. 남은 시간은 렌더 구간 밖
(RAW 전송 · 큐 대기 · 게시 · present)에 있고, 그 구간은 Story 7.6과 7.8이 소유한다.

## 기계 gate 실행

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File `
  tests/hardware/resident-renderer/hv-16/tools/check-resident-evidence.ps1 `
  -RunRoot tests/hardware/resident-renderer/hv-16
```

## parity 회차 재실행

```bash
BOOTHY_HV16_PARITY_MANIFEST=<manifest.json> \
BOOTHY_HV16_PARITY_OUTPUT=<result.json> \
npx vitest run tests/hardware/resident-renderer/hv-16/tools/hv16-parity.test.ts
```

## 열린 항목 (통과로 기록하지 않았다)

- **blind review 패널 5명 미확정** → **AC 3 통과로 기록하지 않는다.** 입력 축 `Technology No-Go` 종료에는 비차단이며 후보 재활성화 시 필수다.
- **MTF50 미측정** — 도구는 있으나 HV-14 corpus에 적합한 slanted-edge 대상이 없다. 후보 재활성화 시 필수다.
- **상주 엔진의 실제 GPU 실행 시간 미측정** — headless 환경에 WebGL2가 없다.

이 세 항목은 `No-Go` 판정을 바꾸지 않는다. 판정은 **입력 부재**로 이미 결정되며,
세 항목이 전부 통과해도 47 ms 상한은 그대로이기 때문이다.
