# Startup Connect Triage Checklist

이 문서는 카메라가 `Preparing`에 오래 머무르거나 startup/connect 구간에서 반복 실패할 때, 매번 새 버그처럼 보이는 로그를 같은 문제군으로 빠르게 분류하기 위한 1페이지 체크리스트다.

## 먼저 결론부터

- 마지막 `detailCode`가 무엇이든, 아래 조건이 같이 보이면 먼저 `startup/connect family`로 본다.
- 이 문제군의 제품 목표는 "정확한 마지막 에러명 찾기"가 아니라 "고객 화면이 무한 `Preparing`에 머물지 않게 안정된 실패 상태로 닫히는지"다.

## 1. 가장 먼저 볼 것

- 최신 런 디렉토리의 `session.json`
- 최신 런 디렉토리의 `diagnostics/camera-helper-status.json`
- 최신 런 디렉토리의 `diagnostics/camera-helper-startup.log`

## 2. startup/connect family 판별

아래가 동시에 보이면 같은 문제군으로 분류한다.

- `lifecycle.stage`가 오래 `preset-selected`에 머문다.
- `captures`가 비어 있다.
- helper startup 상태가 아래 family 안에서 반복되거나 그중 하나에서 멈춘다.
  - `sdk-initializing`
  - `windows-device-detected`
  - `session-opening`
- 마지막 상태가 `fresh`여도, 세션 나이와 sequence가 누적된 startup 반복이면 같은 문제군으로 본다.
- helper 재시작이나 child exit가 섞여도, startup 중이면 별도 새 버그로 보지 않는다.

## 3. 이 문제군이 아닌 경우를 먼저 제거

다음이면 같은 문제로 묶지 말고 다른 축으로 본다.

- helper는 이미 ready/healthy인데 화면만 `Preparing`이다.
  - frontend/runtime fallback 또는 translation 문제 가능성이 높다.
- startup을 넘긴 뒤 촬영/저장 단계에서 실패한다.
  - capture-after-startup 문제로 분리한다.
- 세션 rail/current-session 표시만 어긋난다.
  - session UI 문제로 분리한다.

## 4. 로그를 읽을 때 금지할 오판

- 마지막 `detailCode`만 보고 새 버그라고 결론내리지 않는다.
- `sequence=7`, `20`, `36`처럼 숫자 차이를 바로 다른 문제로 보지 않는다.
- `session-opening`, `windows-device-detected`, `sdk-initializing`를 각각 독립 버그로 쪼개지 않는다.
- stale/fresh 여부만으로 실패 여부를 단정하지 않는다.

## 5. 항상 확인할 불변조건

- 고객 화면이 bounded time 안에 `Preparing`을 벗어나는가
- startup family 반복이 하나의 연결 시도로 묶여 계산되는가
- helper restart/exit가 retry budget 안에서만 허용되는가
- 실패 시 최종 상태가 운영자가 이해할 수 있는 blocked/failure 상태로 닫히는가

## 6. 수정 전에 먼저 적어야 할 한 줄

다음 문장으로 먼저 요약한 뒤 수정한다.

`이 런은 startup/connect family의 또 다른 표면형이며, 제품이 이를 bounded failure로 닫지 못해 Preparing에 남았다.`

이 한 줄이 성립하지 않으면, 다른 문제군으로 다시 분류한다.

## 7. 참고할 기록

- [camera-capture-validation-history.md](./camera-capture-validation-history.md)
  - 최신 실기기 재현과 startup/connect family 변형 사례
- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)
  - helper/readiness/frontend fallback 분리 사례
