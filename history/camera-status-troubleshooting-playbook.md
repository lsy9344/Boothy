# Camera Status Troubleshooting Playbook

이 문서는 2026-04-21 회차에서 실제로 겪었던 카메라 상태 문제를
에이전트가 빠르게 다시 추적할 수 있게 압축한 탐구 문서다.

대형 chronology는 [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)에 남기고,
이 문서는 아래만 빠르게 답하도록 쓴다.

- 지금 문제가 어느 family인가
- 이번 회차에서 실제 원인은 무엇이었나
- 어떤 순서로 좁혀 갔나
- 최종적으로 무엇이 통했고, 지금 기준 되는 방법은 무엇인가

## 한 줄 결론

이번 회차의 핵심은 하나의 단일 버그가 아니었다.
증상은 계속 비슷해 보였지만 실제로는 아래 3개 family가 순서대로 섞여 있었다.

1. 첫 셔터 `0x00000002` 반복
2. 저장 뒤 `Preview Waiting` 고착
3. 이미 정상 종료된 컷에 늦게 붙는 preview render failure 잡음

즉 비슷한 현장 제보가 다시 들어와도,
먼저 `capture 자체 실패`인지 `preview close 실패`인지 `diagnostics 잡음`인지 분리해야 한다.

## 최종 상태

최종 실기기 기준 latest 세션
`session_000000000018a8607f6a1662a4`
에서는 아래가 확인됐다.

- 5컷 모두 RAW 저장 성공
- 5컷 모두 `renderStatus = previewReady`
- 5컷 모두 `preview.kind = preset-applied-preview`
- helper 최종 status는 `ready / healthy / camera-ready`

즉 제품 기준으로는 이제 촬영 저장과 프리셋 적용 close가 정상 동작한다.

## 이번 회차에서 실제로 겪은 문제 family

### 1. 첫 셔터 `0x00000002` 반복

대표 증거:

- `camera-helper-events.jsonl`
  - `capture-accepted`
  - `helper-error(detailCode=capture-trigger-failed, message=...0x00000002)`
- `timing-events.log`
  - `request-capture-auto-retry attempt=1`
  - `request-capture-auto-retry attempt=2`
- `file-arrived` 없음

이때 제품적으로는
카메라가 안 붙는 문제가 아니라,
**첫 셔터 경계가 계속 실패하는 문제**로 보는 것이 맞았다.

### 2. 저장은 됐는데 `Preview Waiting`에 남음

대표 증거:

- `file-arrived` 존재
- same-capture preview JPG 존재
- 하지만 `preview-render-failed reason=render-process-failed`
- `session.json`은 `previewWaiting`에 남음

이때는 촬영 실패가 아니라,
**저장 뒤 preview refinement render가 깨졌을 때 host가 existing preview로 세션 close를 끝내지 못한 문제**였다.

### 3. 이미 정상 종료된 컷에 늦게 failure 로그가 붙음

대표 증거:

- `session.json`은 이미 `previewReady`
- `timing-events.log`에는 같은 capture에 대해
  - `preview-render-ready`
  - 뒤늦게 `preview-render-failed reason=render-output-missing`
- `preview-render-ready` detail이 `presetId=unknown` 같은 placeholder로 남음

이 경우는 실제 고객 실패가 아니라,
**speculative output promote timing race로 생긴 diagnostics 잡음**이었다.

## 직접 원인 정리

### A. `0x00000002` 반복의 직접 원인

처음에는 reconnect/warmup 부족처럼 보였지만,
최종적으로는 아래가 누적 원인이었다.

1. stale ready를 retry 전에 너무 빨리 다시 믿는 경계
2. reconnect 뒤 첫 셔터 command mode/sequence가 너무 공격적이던 경계
3. capture command가 connect/open과 다른 thread 문맥으로 흘렀을 가능성
4. Canon SDK 호출이 서로 다른 STA worker에 흩어졌던 경계

최종적으로는
**Canon SDK 호출을 단일 STA worker로 모으는 것**
이 핵심이었다.

### B. `Preview Waiting` 고착의 직접 원인

이건 카메라 failure가 아니었다.

- RAW 저장은 됨
- same-capture preview도 있음
- darktable refinement만 실패

그런데 host가 이때
existing same-capture preview를 ready close로 승격하지 못해서
`Preview Waiting`에 남았다.

### C. 늦은 `render-output-missing` 잡음의 직접 원인

speculative preview output 파일은 이미 생겼지만,
worker가 detail 기록과 마무리를 끝내기 전에
host가 그 파일을 canonical preview로 너무 빨리 promote했다.

그 결과 worker는 자기 output이 사라졌다고 보고
뒤늦게 `render-output-missing`를 남겼다.

## 실제로 통했던 방법

이번 회차에서 최종적으로 유지해야 하는 해법은 아래 순서다.

### 1. `0x00000002` first-shot family

- helper reconnect 뒤 fresh ready stabilization을 확인한다.
- retry 첫 셔터는 더 보수적인 shutter plan을 쓴다.
- capture command를 STA로 보낼 뿐 아니라,
  connect/open과 **같은 단일 STA worker**에서 실행한다.

핵심 규칙:

`capture를 STA로 보냈다`는 사실만으로 충분하다고 보지 말고,
`connect/open과 capture가 같은 STA worker를 공유하는지`를 먼저 본다.

### 2. refinement render 실패 family

- RAW refinement가 실패하더라도,
  현재 capture에 대해 이미 displayable same-capture preview가 있으면
  host는 그 preview로 ready close를 끝낸다.
- 이때 preview kind는 거짓으로 바꾸지 말고,
  실제 확보된 kind를 유지한다.

핵심 규칙:

`preview-render-failed`가 나와도
같은 capture preview가 이미 있으면 제품 상태를 계속 `Preview Waiting`에 두지 않는다.

### 3. speculative promote race family

- speculative output 파일이 보여도
  worker lock이 살아 있고 detail file이 아직 없으면 바로 promote하지 않는다.
- worker detail/lock 정리가 끝난 뒤에만 output을 가져온다.

핵심 규칙:

정상 close 뒤에 늦은 `render-output-missing`가 붙으면
실패 복구보다 먼저 speculative promote 시점을 의심한다.

## 이번 회차에서 실제로 좁혀 간 과정

1. startup/connect family인지 먼저 분리했다.
2. latest session evidence로 `file-arrived` 유무를 먼저 확인했다.
3. `file-arrived`가 없을 때는 `0x00000002` first-shot family로 계속 좁혔다.
4. reconnect/warmup, shutter mode, half-press, same-request fallback, STA, shared STA worker 순으로 직접 원인을 더 좁혔다.
5. 이후 latest에서 `file-arrived`와 preview JPG가 생기자, 문제군을 render/close 쪽으로 바꿨다.
6. `Preview Waiting` 고착은 existing same-capture preview fallback close로 닫았다.
7. 마지막 남은 늦은 failure 로그는 speculative promote race로 정리했다.

## 다음에 비슷한 문제가 나면 이렇게 본다

### 먼저 4개 파일만 확인

- `session.json`
- `diagnostics/timing-events.log`
- `diagnostics/camera-helper-events.jsonl`
- `diagnostics/camera-helper-status.json`

### 분기 1. `file-arrived`가 없나

그러면 먼저 카메라 first-shot / helper path를 본다.

특히 아래면 같은 family다.

- `capture-accepted`는 있다
- `0x00000002`가 반복된다
- 마지막 helper status는 `camera-ready`다

이때는 connect/open보다 먼저
**shared STA worker 경계**를 본다.

### 분기 2. `file-arrived`와 preview JPG는 있나

그런데 `previewWaiting`에 남는다면,
카메라 문제가 아니라 render close 문제다.

이때는
**existing same-capture preview fallback close**
가 있는지 본다.

### 분기 3. 이미 `previewReady`로 닫혔나

그런데 diagnostics에만 늦은 failure가 붙으면,
고객 실패로 바로 오판하지 말고
**speculative promote race**
를 본다.

## 이번 회차의 최종 결론

- 카메라 상태 문제는 겉보기처럼 하나의 문제로 계속 반복된 것이 아니었다.
- 비슷한 제보라도 실제 실패 지점은
  - first-shot trigger
  - preview close
  - diagnostics race
  로 바뀌어 갔다.
- 그래서 다음에도 같은 식으로 보이면,
  에러 이름보다 먼저
  **`file-arrived`가 있느냐, same-capture preview가 있느냐, session이 실제로는 이미 닫혔느냐**
  를 기준으로 family를 다시 나누는 편이 빠르다.

## 바로 참고할 문서

- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)
  - 전체 chronology와 실세션 기록
- [camera-capture-validation-history.md](./camera-capture-validation-history.md)
  - 촬영/preview round-trip evidence
- [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)
  - startup/connect family 오분류 방지
