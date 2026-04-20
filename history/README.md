# Boothy History Guide

이 문서는 `history/` 폴더의 역할과 권장 읽기 순서를 정리한 안내서다.

## 원칙

- `history/`는 현재 방향을 결정하는 문서가 아니라, field evidence와 운영 히스토리를 보존하는 문서 집합이다.
- 먼저 `docs/README.md`를 읽고 현재 방향을 이해한 뒤, 필요한 증거를 찾기 위해 `history/`를 읽는다.
- 가장 최신 결론은 history보다 `docs/`에 우선한다.

## 파일 역할

- [camera-capture-validation-history.md](./camera-capture-validation-history.md)
  - 촬영/preview/final 관련 최신 실기기 검증과 수정 기록
- [recent-session-thumbnail-speed-brief.md](./recent-session-thumbnail-speed-brief.md)
  - recent-session 및 preview 속도 이슈의 핵심 측정 요약
- [recent-session-thumbnail-speed-agent-context.md](./recent-session-thumbnail-speed-agent-context.md)
  - 에이전트가 recent-session/preview 속도 문제를 이어받을 때 필요한 압축 컨텍스트
- [photo-button-latency-history.md](./photo-button-latency-history.md)
  - 버튼 이후 preview 체감 지연을 다룬 초기/중간 분석
- [camera-helper-troubleshooting-history.md](./camera-helper-troubleshooting-history.md)
  - helper/카메라 연결/timeout/readiness 관련 문제 해결 이력
- [current-session-photo-troubleshooting-history.md](./current-session-photo-troubleshooting-history.md)
  - 현재 세션 표시/동작 관련 문제 해결 이력

## 권장 읽기 순서

1. `camera-capture-validation-history.md`
2. `recent-session-thumbnail-speed-agent-context.md`
3. `recent-session-thumbnail-speed-brief.md`
4. 필요한 경우에만 `photo-button-latency-history.md`
5. helper 문제를 볼 때만 `camera-helper-troubleshooting-history.md`
6. 세션/레일 문제를 볼 때만 `current-session-photo-troubleshooting-history.md`

## 에이전트용 메모

- history 문서는 최신 항목이 중요하다.
- 오래된 항목은 구조적 배경으로만 보고, 현재 판단은 `docs/`와 최신 history 항목으로 교차 확인한다.
- `_bmad-output`보다 `docs/README.md`와 이 문서를 먼저 읽는 것이 안전하다.
