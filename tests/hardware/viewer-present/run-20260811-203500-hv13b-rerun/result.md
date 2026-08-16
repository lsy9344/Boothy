# HV-13B rerun result

## 판정

**No-Go — 코드 결함과 software telemetry 증거는 닫혔지만 120fps+ 물리 프레임 증거가 없어 hardware gate 전체를 닫을 수 없다.**

## 완료된 검증

- Canon EOS 700D와 승인 `DISPLAY3`에서 두 변형을 직접 실행했다.
- 변형별 warm-up 5회와 측정 30회, 총 70회 실촬영을 완료했다.
- 측정 구간은 각 변형 30 request / 60 A·B row가 모두 `presented`, `measured`, trusted input으로 기록됐다.
- 측정 120개 row의 `viewerWindowEventsAfterInput`은 모두 0이다.
- 총 140개 immutable generation의 존재·크기·FNV-1a 64 hash가 모두 일치했다.
- 두 변형 모두 마지막 B가 승인 모니터에 표시되는 것을 확인했다.

## Timing

| 변형 | p50 | p95 | max | 성공률 |
|---|---:|---:|---:|---:|
| visible-standby | 3710.617 ms | 4280.416 ms | 4672.717 ms | 60/60 |
| hidden-prewarm/invisible | 3774.393 ms | 4540.441 ms | 15024.476 ms | 60/60 |

hidden-prewarm은 p95가 더 느리고 15.024초 outlier가 있어 기본 동작으로 선택하지 않는다. `visible-standby`를 유지한다.

## 남은 release blocker

120fps 이상의 외부 촬영 장비가 없어 blank/spinner/stale image/crop jump/scale jump가 0 frame인지와 software present–photon offset을 측정하지 못했다. 이 물리 증거가 수집되기 전까지 HV-13B는 No-Go다.

## 핵심 증거

- `timing/summary.json`
- `ab/summary.json`
- `window-events/summary.json`
- `visible-standby/viewer-present.jsonl`
- `hidden-prewarm/viewer-present.jsonl`
- 각 변형의 `display/generations.jsonl`, `display/pointer.json`, `integrity-check.json`
- `frames/README.md`
