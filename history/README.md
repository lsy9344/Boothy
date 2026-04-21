# Boothy History Guide

이 문서는 `history/` 폴더를 에이전트가 빠르게 탐색하기 위한 진입점이다.

`history/`는 현재 방향을 결정하는 폴더가 아니다. 먼저 `docs/README.md`로 현재 제품 판단을 읽고, 그 다음 여기서 필요한 근거 문서만 골라 들어간다.

## 먼저 기억할 규칙

- 현재 제품 판단은 항상 `docs/`가 우선이다.
- `history/`는 field evidence, 운영 히스토리, 재현 기록을 찾을 때만 쓴다.
- 큰 문서를 처음부터 다 읽지 말고, 증상에 맞는 가장 짧은 문서부터 연다.
- startup/connect, thumbnail speed처럼 이미 압축 진입 문서가 있는 주제는 원문 대형 문서보다 그 문서를 먼저 읽는다.

## 빠른 진입 순서

1. `docs/README.md`를 읽어 현재 방향을 확인한다.
2. 지금 다루는 문제가 어느 증상군인지 먼저 분류한다.
3. 최근 카메라 상태/첫 촬영/preview close가 함께 흔들린다면 먼저 `camera-status-troubleshooting-playbook.md`를 연다.
4. 아래 "증상별 첫 문서"에서 가장 짧은 진입 문서를 연다.
5. 그 문서가 참조하는 대형 history 문서는 필요할 때만 확장해서 읽는다.

## 최신 카메라 상태 이슈 압축 문서

최근처럼 아래가 한 세트로 섞여 보이는 경우가 있었다.

- 첫 셔터 `0x00000002` 반복
- 저장 뒤 `Preview Waiting` 고착
- 이미 정상 종료된 컷에 늦은 `preview-render-failed` 잡음

이 경우는 먼저 아래 문서를 읽는다.

- [camera-status-troubleshooting-playbook.md](./camera-status-troubleshooting-playbook.md)

이 문서는 2026-04-21 회차의 실제 원인, 좁혀 간 과정, 최종 해법, 재발 시 분기 규칙만 압축해 둔 탐구 문서다.

## 증상별 첫 문서

### 1. 카메라가 `Preparing`에 오래 머물거나 startup/connect에서 반복 실패한다

먼저 읽을 문서:

- [camera-status-troubleshooting-playbook.md](./camera-status-troubleshooting-playbook.md)
- [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)

그 다음 필요하면:

- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)
- [camera-capture-validation-history.md](./camera-capture-validation-history.md)

이 경로를 쓰는 증상:

- 카메라가 붙지 않는다.
- startup 단계가 반복된다.
- 마지막 `detailCode`는 바뀌는데 제품적으로는 같은 startup/connect family처럼 보인다.

### 2. `사진 찍기` 요청 이후 촬영 round-trip이 느리거나 실패한다

먼저 읽을 문서:

- [camera-status-troubleshooting-playbook.md](./camera-status-troubleshooting-playbook.md)
- [camera-capture-validation-history.md](./camera-capture-validation-history.md)

같이 볼 수 있는 문서:

- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)

이 경로를 쓰는 증상:

- `사진 찍기` 직후 오래 멈춘다.
- `Phone Required`로 떨어진다.
- 첫 촬영 뒤 다음 촬영에서 회귀가 난다.
- request 수락, 파일 도착, 고객 화면 반영 중 어디서 끊겼는지 분리해야 한다.

### 3. `현재 세션 사진` 레일이 비어 있거나 placeholder로 남는다

먼저 읽을 문서:

- [current-session-photo-troubleshooting-history.md](./current-session-photo-troubleshooting-history.md)

같이 볼 수 있는 문서:

- [camera-capture-validation-history.md](./camera-capture-validation-history.md)
- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)

이 경로를 쓰는 증상:

- 촬영은 됐는데 `현재 세션 사진`에 안 보인다.
- preview가 실제 사진으로 바뀌지 않는다.
- rail/UI 문제인지 capture pipeline 문제인지 분리해야 한다.

### 4. recent-session/preview 표시 속도를 더 줄이고 싶다

먼저 읽을 문서:

- [recent-session-thumbnail-speed-agent-context.md](./recent-session-thumbnail-speed-agent-context.md)

그 다음 읽을 문서:

- [recent-session-thumbnail-speed-brief.md](./recent-session-thumbnail-speed-brief.md)

필요할 때만:

- [photo-button-latency-history.md](./photo-button-latency-history.md)

이 경로를 쓰는 증상:

- same-capture first-visible과 preset-applied preview close를 구분해야 한다.
- 최근 세션 썸네일이 언제 보이는지가 핵심이다.
- 속도 최적화 맥락만 빠르게 이어받아야 한다.

### 5. helper/readiness/timeout 일반 이력을 넓게 봐야 한다

먼저 읽을 문서:

- [camera-status-troubleshooting-playbook.md](./camera-status-troubleshooting-playbook.md)
- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)

이 경로를 쓰는 증상:

- helper lifecycle 전반을 이해해야 한다.
- readiness, timeout, frontend fallback, session attach 문제를 넓게 봐야 한다.

## 문서 성격 맵

- [startup-connect-triage-checklist.md](./startup-connect-triage-checklist.md)
  - 가장 짧은 triage 문서. startup/connect family를 새 버그로 오판하지 않게 도와준다.
- [camera-status-troubleshooting-playbook.md](./camera-status-troubleshooting-playbook.md)
  - 2026-04-21 카메라 상태 회차 압축 문서. first-shot trigger, preview close, diagnostics race를 한 번에 분리하는 최신 탐구 시작점이다.
- [recent-session-thumbnail-speed-agent-context.md](./recent-session-thumbnail-speed-agent-context.md)
  - 속도 문제용 압축 handoff. 긴 브리프를 열기 전의 첫 진입점이다.
- [photo-button-latency-history.md](./photo-button-latency-history.md)
  - 속도 이슈의 중간 배경 정리. 최신 결론만 필요하면 우선순위가 낮다.
- [current-session-photo-troubleshooting-history.md](./current-session-photo-troubleshooting-history.md)
  - 현재 세션 rail/preview 치환 문제 전용 기록이다.
- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)
  - helper/readiness 전반의 넓은 운영 이력이다.
- [recent-session-thumbnail-speed-brief.md](./recent-session-thumbnail-speed-brief.md)
  - recent-session/preview 속도 이슈의 대형 브리프다.
- [camera-capture-validation-history.md](./camera-capture-validation-history.md)
  - 촬영/preview/final round-trip의 가장 큰 검증 기록이다.

## 대형 문서 진입 규칙

- `camera-capture-validation-history.md`는 촬영 round-trip 자체가 문제일 때만 연다.
- `recent-session-thumbnail-speed-brief.md`는 속도 최적화 작업을 실제로 이어받을 때만 연다.
- `camera-helper-troubleshooting-history.md`는 startup family 체크리스트로 분류가 끝난 뒤 넓은 배경이 필요할 때 연다.

## 에이전트용 최소 행동 규칙

- 증상 분류 전에 가장 큰 문서부터 열지 않는다.
- 현재 판단과 충돌하면 `history/`보다 `docs/`를 따른다.
- 오래된 기록은 구조적 배경으로만 읽고, 실제 판단은 최신 항목과 `docs/`로 교차 확인한다.
- `_bmad-output`보다 `docs/README.md`와 이 문서를 먼저 읽는 것이 안전하다.
