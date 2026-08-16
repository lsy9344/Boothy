# HV-13B result

## 판정

**No-Go** — 실제 Canon EOS 700D 촬영과 승인 고객 모니터에서 두 표시 변형을 실행했지만, 핵심 actual-present 계측과 물리 프레임 증거가 성립하지 않았다.

## 확인된 항목

- `visible-standby`: 실촬영 5회, immutable generation 10개(A/B 각 5개), pointer revision 11.
- `hidden-prewarm/invisible`: 실촬영 1회, immutable generation 2개(A/B), pointer revision 3.
- 12개 generation 파일이 모두 존재하고 byte size와 `sourceHash`가 원장과 일치했다.
- generation sequence는 각 세션에서 단조 증가했고 pointer는 마지막 B generation을 가리켰다.
- 두 변형 모두 승인된 `DISPLAY3`의 1429×953 photo rect 안에서 sample B를 표시했다.
- hidden-prewarm은 촬영 전 viewer가 hidden이었고 첫 generation에서 공개됐다.

## Release blocker

1. 두 세션 모두 `<session>/diagnostics/viewer-present.jsonl`이 생성되지 않았다. 총 6회 실촬영과 12개 committed generation에 대해 actual-present, latency, confidence, `viewerWindowEventsAfterInput` 표본이 0건이다.
2. 120fps 이상 외부 촬영 rig가 없어 compositor-to-photon 오프셋과 zero-transition-defect를 프레임 단위로 증명할 수 없다.

위 두 조건 때문에 p50/p95/max, 성공률, low-confidence 분포, AC 4 zero-window-event, visible/hidden 기본값 선택을 산출하지 않았다. 유효 표본이 없는 상태에서 30회 측정을 계속하면 거짓 정밀도만 만들기 때문에 warm-up 단계에서 중단했다.

## Rerun prerequisite

- viewer present 보고가 양쪽 변형에서 세션별 JSONL로 지속 저장되도록 수정하고 실제 장비에서 최소 1회 smoke 확인.
- 120fps 이상 외부 촬영 rig 준비.
- 같은 PC·카메라·승인 모니터에서 변형별 warm-up 5회 + 측정 30회를 randomized AB/BA 순서로 재실행.

## Core evidence

- `environment.md`, `environment/system-fingerprint.json`
- `logs/visible-standby.stdout.log`, `logs/hidden-prewarm.stdout.log`
- `visible-standby/generations.jsonl`, `visible-standby/integrity-check.json`, `visible-standby/pointer.json`
- `hidden-prewarm/generations.jsonl`, `hidden-prewarm/integrity-check.json`, `hidden-prewarm/pointer.json`
- `frames/README.md`, `timing/summary.json`, `window-events/summary.json`, `ab/summary.json`
