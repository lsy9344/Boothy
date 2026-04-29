# Runbooks Guide

이 문서는 `docs/runbooks/` 안의 문서를 현재 우선순위 기준으로 정리한 안내서다.

## 먼저 읽을 runbook

1. [Story 1.26 Agent Operating Guide](./story-1-26-agent-operating-guide.md)
2. [Story 1.26 Review Root Cause And Improvement Direction](./story-1-26-review-root-cause-and-improvement-direction-20260427.md)
3. [Preview Latency Next Steps Checklist](./preview-latency-next-steps-checklist-20260422.md)
4. [Story 1.26 Reserve Path Opening](./story-1-26-reserve-path-opening-20260420.md)
5. [Current Preview GPU Direction](./current-preview-gpu-direction-20260419.md)
6. [Current Actual-Lane Handoff](./current-actual-lane-handoff-20260419.md)
7. [Preview Track Route Decision](./preview-track-route-decision-20260418.md)

- [Story 1.26 Agent Operating Guide](./story-1-26-agent-operating-guide.md)
  - Story `1.26` 작업 시 AI agent가 읽을 범위, 기록 위치, 반복 실패 요약 방식을 정한 효율 작업 가이드
- [Story 1.26 Review Root Cause And Improvement Direction](./story-1-26-review-root-cause-and-improvement-direction-20260427.md)
  - native approximation false Go correction과 option 2 resident full-preset route의 latest `Go` evidence를 정리한 current root-cause 문서
- [Preview Latency Next Steps Checklist](./preview-latency-next-steps-checklist-20260422.md)
  - latest field session 이후 무엇이 이미 닫혔고, 지금 어떤 순서로 다음 시도를 진행해야 하는지 적어 둔 current execution checklist

## supporting runbook

- [Old First-Visible CPU Baseline Rerun](./old-first-visible-cpu-baseline-rerun-20260419.md)
  - old `resident first-visible` lane을 같은 형식으로 다시 검증했던 baseline runbook. `2026-04-20` 이후에는 historical comparison reference로 읽는다.
- [Booth Hardware Validation Checklist](./booth-hardware-validation-checklist.md)
  - 실기기 검증 순서와 체크포인트
- [Booth Hardware Validation Architecture Research](./booth-hardware-validation-architecture-research.md)
  - OpenCL/GPU capability, validation method, 비교 관찰 포인트를 설명하는 연구 메모

## 읽기 원칙

- 방향 판단이 필요하면 먼저 위 current runbook을 읽는다.
- supporting runbook은 실행 방법이나 세부 비교 기준이 필요할 때만 읽는다.
- `_bmad-output` runbook이나 artifact는 이 폴더의 current runbook보다 우선하지 않는다.
