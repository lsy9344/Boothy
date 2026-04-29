# Boothy Non-BMAD Doc Map

이 문서는 `_bmad-output` 밖의 문서만 기준으로 현재 프로젝트 문서 진입점을 정리한 canonical 안내서다.

## 문서 사용 원칙

- 새 에이전트나 새 작업은 이 문서부터 읽는다.
- `_bmad-output` 문서는 framework output이므로 기본 진입점으로 읽지 않는다.
- `_bmad-output`은 이 문서가 별도로 가리키는 경우에만 supporting reference로 본다.
- `history/` 문서는 field evidence와 운영 기록이다. 현재 방향 결정의 출발점으로 쓰지 말고, 결정 문서를 읽은 뒤 근거 확인용으로 쓴다.

## 지금 가장 먼저 읽을 문서

1. [Story 1.26 Review Root Cause And Improvement Direction](./runbooks/story-1-26-review-root-cause-and-improvement-direction-20260427.md)
2. [Preview Latency Next Steps Checklist](./runbooks/preview-latency-next-steps-checklist-20260422.md)
3. [Story 1.26 Reserve Path Opening](./runbooks/story-1-26-reserve-path-opening-20260420.md)
4. [GPU Direction](./runbooks/current-preview-gpu-direction-20260419.md)
5. [Current Actual-Lane Handoff](./runbooks/current-actual-lane-handoff-20260419.md)
6. [Preview Track Route Decision](./runbooks/preview-track-route-decision-20260418.md)
7. [Preview Architecture History And Agent Guide](./preview-architecture-history-and-agent-guide.md)
8. [Release Baseline](./release-baseline.md)

## 문서 역할 맵

### 1. 현재 방향

- [Story 1.26 Review Root Cause And Improvement Direction](./runbooks/story-1-26-review-root-cause-and-improvement-direction-20260427.md)
  - native approximation false Go correction과 option 2 resident full-preset route의 latest `Go` evidence를 정리한 current root-cause 문서
- [Preview Latency Next Steps Checklist](./runbooks/preview-latency-next-steps-checklist-20260422.md)
  - latest field session 기준으로 무엇이 이미 닫혔고, 지금 어떤 순서로 다음 시도를 진행해야 하는지 정리한 current execution checklist
- [Story 1.26 Reserve Path Opening](./runbooks/story-1-26-reserve-path-opening-20260420.md)
  - `1.10` old line을 closed `No-Go` baseline으로 확정하고, `1.26` reserve path를 어떤 범위로 공식 오픈했는지 정리한 현재 실행 기준
- [GPU Direction](./runbooks/current-preview-gpu-direction-20260419.md)
  - old line GPU 비교는 이제 optional 참고 실험으로 낮추고, native/GPU resident reserve path와 어떤 관계인지 설명하는 현재 판단 문서
- [Current Actual-Lane Handoff](./runbooks/current-actual-lane-handoff-20260419.md)
  - 왜 old line으로 돌아왔는지와, 왜 지금은 그 lane을 active path가 아니라 closed baseline으로 읽는지 정리한 handoff
- [Preview Track Route Decision](./runbooks/preview-track-route-decision-20260418.md)
  - 공식 route decision과 `1.26` opening 판단 기준
- [Release Baseline](./release-baseline.md)
  - 현재 release gate와 preview-track release hold 상태

### 2. 구조 히스토리

- [Preview Architecture History And Agent Guide](./preview-architecture-history-and-agent-guide.md)
  - 지금까지 어떤 구조를 시도했고 왜 멈췄는지 phase 단위로 정리한 가장 큰 히스토리 문서

### 3. 실행 runbook

- [Runbooks Guide](./runbooks/README.md)
  - runbook 안에서 current 판단 문서와 supporting 문서를 구분하는 진입점
- [Old First-Visible CPU Baseline Rerun](./runbooks/old-first-visible-cpu-baseline-rerun-20260419.md)
  - old resident first-visible line을 다시 검증할 때 쓰는 baseline runbook
- [Booth Hardware Validation Checklist](./runbooks/booth-hardware-validation-checklist.md)
  - 하드웨어 검증 체크리스트
- [Booth Hardware Validation Architecture Research](./runbooks/booth-hardware-validation-architecture-research.md)
  - OpenCL/GPU, darktable-cltest, validation architecture를 더 깊게 설명하는 연구 메모

### 4. 계약 문서

- [Contracts Guide](./contracts/README.md)
  - 계약 문서에서 무엇을 먼저 읽어야 하는지 정리한 진입점
- [Render Worker Contract](./contracts/render-worker.md)
- [Session Manifest Contract](./contracts/session-manifest.md)
- [Preset Bundle Contract](./contracts/preset-bundle.md)
- [Camera Helper Sidecar Protocol](./contracts/camera-helper-sidecar-protocol.md)
- [Camera Helper EDSDK Profile](./contracts/camera-helper-edsdk-profile.md)
- [Branch Rollout Contract](./contracts/branch-rollout.md)
- [Authoring Publication](./contracts/authoring-publication.md)
- [Authoring Publication Payload](./contracts/authoring-publication-payload.md)

## history 사용 규칙

- `history/`는 제품 판단의 근거 로그와 운영 메모를 남기는 곳이다.
- 현재 방향을 알고 싶으면 먼저 `docs/`를 읽고, 그 다음 `history/`에서 최신 실기기 증거를 확인한다.
- `history/`의 권장 읽기 순서는 [history/README.md](../history/README.md)를 따른다.

## 에이전트용 최소 읽기 순서

1. 이 문서
2. `docs/runbooks/story-1-26-review-root-cause-and-improvement-direction-20260427.md`
3. `docs/runbooks/preview-latency-next-steps-checklist-20260422.md`
4. `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
5. `docs/runbooks/current-preview-gpu-direction-20260419.md`
6. `docs/runbooks/current-actual-lane-handoff-20260419.md`
7. `docs/runbooks/preview-track-route-decision-20260418.md`
8. `docs/release-baseline.md`
9. `docs/preview-architecture-history-and-agent-guide.md`
10. `docs/runbooks/README.md`
11. `docs/contracts/README.md`
12. `history/README.md`

## 제외 규칙

- `_bmad-output/`는 canonical 문서 집합이 아니다.
- `_bmad-output/`가 더 자세한 배경을 줄 수는 있지만, 현재 방향 판단은 이 `docs/`와 `history/` 기준으로 먼저 해석한다.
