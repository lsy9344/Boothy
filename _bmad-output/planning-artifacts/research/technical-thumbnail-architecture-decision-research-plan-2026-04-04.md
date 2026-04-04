---
workflowType: 'research-plan'
research_type: 'technical'
research_topic: 'thumbnail latency next-phase architecture decision'
date: '2026-04-04'
author: 'Codex'
status: 'draft'
---

# Thumbnail Architecture Decision Research Plan

## Purpose

이 문서는 `최근 세션 썸네일` 문제를 더 이상 미세 튜닝 과제로 보지 않고,
`구조 전환이 필요한지`를 결정하기 위한 후속 리서치 아웃라인이다.

핵심 질문은 하나다.

- **현재 darktable 기반 truthful close 구조를 유지한 채 목표를 달성할 수 있는가, 아니면 아키텍처/플랫폼 전환이 필요한가?**

## Why This Research Exists

현재까지의 실측은 다음을 반복해서 보여준다.

- same-capture `first-visible`은 이미 약 `3초대`
- 고객이 실제로 기다리는 `preset-applied preview close`는 여전히 약 `7초 ~ 10초`
- 최근 회귀에서는 첫 preview render 실패 후 재시도로 `10초대`까지 다시 튐

즉 다음 단계의 중심은 `조금 더 줄이기`가 아니라,
`50% 추가 단축이 가능한 구조가 무엇인지`를 고르는 것이다.

## Decision Goal

이번 리서치의 목표는 아래 3안 중
Boothy에 가장 현실적이고 효과적인 방향을 하나 추천하는 것이다.

1. `Local dedicated renderer`
2. `Watch-folder based external render bridge`
3. `Edge render appliance + thin client`

## Out Of Scope

이번 리서치에서 하지 않을 것:

- 현 darktable 경로의 추가 미세 최적화 나열
- UI 표현 개선 중심의 논의
- 클라우드 중심 분산 아키텍처 확장
- 최종 결과물 export 전체 파이프라인 재설계

## Candidate Options

### Option A. Local Dedicated Renderer

현재 앱 셸은 유지하고,
`preset-applied preview first-visible`만 전용 저지연 렌더러로 분리한다.

핵심 기대:

- 가장 작은 제품 변경 폭
- 로컬 hot path 유지
- darktable fallback 병행 가능

주요 질문:

- 현재 preset/XMP를 이 전용 렌더러에 얼마나 정확히 이식할 수 있는가
- `2초대` 또는 `50% 단축` 가능성이 실제로 있는가

### Option B. Watch-Folder External Render Bridge

현재 앱은 capture orchestration에 집중하고,
렌더는 외부 엔진 또는 외부 제품 파이프라인으로 넘긴다.

핵심 기대:

- 렌더 엔진 교체 유연성
- 기존 앱과 새 렌더 경로를 비교 운영하기 쉬움

주요 질문:

- 파일 기반 bridge가 low-latency 목표에 충분히 빠른가
- 운영 복잡도와 failure handling이 제품적으로 감당 가능한가

### Option C. Edge Render Appliance + Thin Client

카메라 제어와 저지연 렌더를 booth 옆 edge 노드로 옮기고,
현재 앱은 얇은 UI client 성격으로 축소한다.

핵심 기대:

- 가장 큰 구조 개선 여지
- 장기적으로 GPU-resident path 설계 가능

주요 질문:

- 운영 복잡도 증가가 허용 가능한가
- 단일 booth 제품에 비해 얻는 이득이 충분한가

## Research Questions

이번 후속 리서치에서 반드시 답해야 할 질문:

1. `preset-applied preview close 50% 추가 단축` 가능성이 가장 높은 옵션은 무엇인가
2. 각 옵션이 `Preview Waiting truth`와 `same-slot replacement` 계약을 유지할 수 있는가
3. 각 옵션이 현재 preset 품질 일관성을 어디까지 유지할 수 있는가
4. 첫 visible 과 truthful close를 분리한 `dual artifact model`을 자연스럽게 지원하는가
5. 오프라인 현장 환경에서 장애 복구와 운영 난이도는 어느 수준인가
6. 현재 darktable fallback을 병행하면서 점진 전환이 가능한가

## Required Evidence

각 옵션 비교에서 최소한 아래 증거가 필요하다.

- 예상 latency budget
- hot path 단계 수
- 로컬 GPU/CPU 사용 방식
- preset fidelity risk
- failure mode와 fallback 방식
- 구현 난이도
- 운영 복잡도
- 단계적 롤아웃 가능성

## Evaluation Criteria

최종 비교표는 아래 기준으로 정리한다.

- `Latency potential`
- `Preset fidelity`
- `Implementation risk`
- `Operational complexity`
- `Fallback compatibility`
- `Incremental rollout fit`
- `Boothy product fit`

## Expected Output

최종 산출물은 아래 3개다.

1. `3안 비교표`
2. `권장안 1개`
3. `권장안 기준 30/60/90일 구현 로드맵`

## Initial Recommendation To Validate

현재 문서와 실측만 기준으로 한 출발 가설은 아래다.

- 1차 추천 검토안은 `Local dedicated renderer`
- 이유:
  - 현재 tech 문서가 가장 현실적인 다음 단계로 이미 제시함
  - 현 앱 셸과 제품 계약을 덜 깨뜨림
  - `darktable fallback`을 남긴 상태로 점진 전환하기 좋음

단, 이 가설은 후속 리서치에서
`실제 50% 단축 가능성`과 `preset fidelity`를 확인한 뒤 확정해야 한다.

## Research Execution Order

1. 기존 tech 문서에서 각 옵션의 근거 문장 재정리
2. Boothy 현재 계약과 충돌하는 지점 식별
3. 옵션별 hot path 초안 작성
4. 옵션별 latency/fidelity/운영 리스크 비교
5. 권장안 선정
6. 프로토타입 범위 정의

## Final Decision Gate

이번 리서치의 끝은 아래 둘 중 하나여야 한다.

- `현 구조 유지 + 추가 최적화`가 여전히 합리적이라는 결론
- `구조 전환 승인`이 필요하다는 결론

현재 기준선과 tech 문서 방향을 함께 보면,
이번 리서치는 두 번째 결론으로 갈 가능성이 높다.
