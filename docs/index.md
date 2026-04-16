# Boothy Docs Index

## 목적

이 폴더는 새 에이전트가 Boothy의 현재 제품 상태, 계약, 운영 기준, 아키텍처 판단 근거를 빠르게 이어받을 수 있도록 만든 프로젝트 지식 진입점이다.

## 먼저 읽을 문서

1. [preview-architecture-history-and-agent-guide.md](./preview-architecture-history-and-agent-guide.md)
   - 썸네일/preview 아키텍처 문제의 과거 조사, 시도, 검증, 현재 상태, 다음 판단 규칙을 한 번에 정리한 에이전트용 운영 문서
2. [release-baseline.md](./release-baseline.md)
   - 현재 release hold 조건, 하드웨어 검증 게이트, preview architecture sign-off 기준

## 세부 문서 위치

- `contracts/`
  - 제품 계약, session/capture/render/sidecar/rollout 규칙
- `runbooks/`
  - 하드웨어 검증, evidence bundle, 운영 체크리스트
- `superpowers/`
  - 세션 중 생성된 계획 문서와 작업 보조 문서

## preview architecture 관련 핵심 원칙

- 현재 제품의 primary acceptance는 `same-capture preset-applied full-screen visible <= 2500ms`다.
- `first-visible`, tiny preview, recent-strip update는 고객 안심용 중간 신호일 수 있지만 release success가 아니다.
- legacy preview track은 비교/회귀/rollback rehearsal용 기록이며, 현재 forward path는 Story `1.21 -> 1.25`다.
- Story `1.13`이 최종 guarded cutover와 hardware release close owner다.
