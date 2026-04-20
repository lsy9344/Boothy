---
documentType: direction-note
status: active
date: 2026-04-19
scope: preview-architecture
---

# Current Preview GPU Direction

## 왜 이 문서가 필요한가

- 현재 worktree는 old `resident first-visible` line을 다시 검증하는 baseline/comparison lane이다.
- 사용자가 GPU를 다시 검토하려는 이유는 단순한 호기심이 아니라, 현재 아키텍처가 체감 속도에서는 가장 가깝지만 미세조정만으로는 한계가 보이기 때문이다.
- 이 문서는 그 판단이 기술적으로 타당한지와, GPU를 어디에 어떻게 붙여야 하는지를 현재 문서 집합 기준으로 정리한다.

## 현재 기술 판단

- GPU를 검토하는 방향 자체는 타당하다.
- 다만 `현재 darktable-cli 경로에 GPU/OpenCL을 그냥 켜면 충분하다`는 판단은 타당하지 않다.
- 지금까지의 실패 이력을 보면 병목은 단순 `GPU off` 하나가 아니라 아래 조합에 더 가깝다.
  - per-capture process spawn 비용
  - warm-state 유지 실패 또는 미활용
  - same-capture close owner 경쟁
  - same-path replacement 비용
  - darktable preview invocation 자체의 고정 비용

즉 현재 단계의 GPU 논의는 `옵션 on/off`보다 `어떤 runtime boundary에서 GPU를 주력 자원으로 쓰느냐`가 핵심이다.

## 우리가 이미 확인한 것

- newer `actual-primary-lane`은 correctness 개선에는 성공했지만 official `preset-applied visible <= 3000ms` gate를 닫지 못했다.
- old `resident first-visible` line은 user-perceived speed가 가장 좋았던 historical candidate다.
- 따라서 지금 old line으로 돌아온 것은 단순 rollback이 아니라, 가장 유력한 baseline/comparison lane을 다시 읽기 위한 것이다.
- 이 문맥에서 GPU는 `route promotion 근거`가 아니라 `현재 가장 가까운 comparison lane을 더 당길 수 있는 acceleration hypothesis`다.

## GPU를 어디에 써야 하는가

### 1. 지금 당장 의미 있는 GPU 사용

- old `resident first-visible` line을 current contract로 다시 닫은 뒤,
- 동일한 one-session evidence package 형식에서
- GPU/OpenCL capability와 latency uplift를 matched comparison으로 검증한다.

이 단계의 목적은 두 가지다.

- 현재 되돌아온 line에서 GPU가 실제로 first-visible과 follow-up replacement를 함께 줄일 수 있는지 확인
- `좋아 보이는 체감`, reference metric 개선, official release proof를 섞지 않고 증거를 분리

### 2. 현재 아키텍처 안에서의 올바른 GPU 목표

현재 아키텍처 안에서 GPU를 붙일 때 목표는 아래여야 한다.

- first-visible seam을 더 당긴다
- same-slot later truthful close도 같이 당긴다
- `Preview Waiting` truth를 깨지 않는다
- wrong-capture, cross-session leakage, false-ready를 만들지 않는다

즉 GPU는 `더 빨리 그려보자`가 아니라,
`현재 host-owned truth contract를 유지하면서 first-visible lane과 truthful close lane의 hot path를 줄이는 자원`
으로 써야 한다.

## GPU를 어떻게 쓰면 안 되는가

- `darktable-cltest`가 된다는 이유만으로 성공을 주장하면 안 된다.
- GPU 결과만으로 route promotion 또는 release success를 주장하면 안 된다.
- GPU 결과만으로 old line을 release-proof lane으로 읽으면 안 된다.
- current capture close owner를 broker-first, remote-first 구조로 갑자기 바꾸면 안 된다.
- `_bmad-output`의 prototype 문구를 현재 release winner처럼 읽으면 안 된다.

## 현재 기준 다음 단계

`2026-04-20` opening update:

- old line CPU baseline은 Story `1.10`에서 closed `No-Go` baseline으로 확정됐다.
- matched GPU/OpenCL comparison은 이제 mainline이 아니라 optional comparison evidence다.
- 현재 공식 active route는 Story `1.26 reserve path`다.

### Step 1. CPU baseline 재닫기

- old `resident first-visible` line을 approved booth-safe baseline/comparison lane으로 one-session package에 다시 닫는다.
- 여기서는 `disable_opencl=false`, `allow_fast_preview_raster=true` 상태를 baseline으로 읽고,
  GPU/OpenCL capability는 메타데이터로만 기록한다.

### Step 2. GPU comparison gate 열기

- CPU baseline package가 complete할 때만 GPU comparison을 연다.
- 비교 시에는 아래를 같이 남긴다.
  - same capture first-visible owner
  - truthful close owner
  - `sameCaptureFullScreenVisibleMs`
  - `originalVisibleToPresetAppliedVisibleMs`
  - `darktable-cltest`
  - observed GPU/OpenCL capability

metric interpretation:

- official gate: `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
- reference/comparison metric: `sameCaptureFullScreenVisibleMs`

### Step 3. 판단

- 만약 GPU가 old line 안에서 first-visible과 later truthful close를 함께 유의미하게 줄인다면,
  old line + GPU acceleration은 계속 검증할 가치가 있다.
- 만약 uplift가 작거나 first-visible만 줄고 truthful close는 크게 줄지 않는다면,
  current darktable-based topology 안에서의 GPU on/off는 주 해법이 아니다.
- 어느 경우든 release success는 오직 `preset-applied visible <= 3000ms` 충족으로만 읽는다.

### Step 4. 그 다음 카드

- 그 경우 다음 주력 후보는 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`다.
- `2026-04-20` 기준 이 카드는 Story `1.26`으로 공식 오픈됐다.
- 이 경로에서 darktable는 버리는 것이 아니라:
  - parity reference
  - fallback
  - final/export truth
  로 남기는 편이 맞다.

## 현재 내 기술 판단

- 사용자가 GPU를 다시 꺼내 든 이유는 기술적으로 설득력 있다.
- 현재 상태에서 더 이상의 소폭 wait-budget, polling, preview size 조정만으로 목표를 닫을 가능성은 낮다.
- 하지만 지금 바로 `새 GPU 엔진`으로 뛰는 것도 이르다.
- 가장 현실적인 순서는 아래다.

1. Story `1.10` old line을 closed baseline으로 얼린다.
2. Story `1.26`에서 native/GPU resident reserve path를 연다.
3. old line GPU/OpenCL comparison은 필요할 때만 side evidence로 쓴다.

즉 지금의 GPU 방향은 `단순 옵션 튜닝`이 아니라
`현재 가장 가까운 아키텍처를 검증하고, 필요하면 그 다음 native/GPU resident lane으로 넘어가기 위한 단계적 기술 판단`
으로 해석하는 것이 맞다.

story interpretation boundary:

- `1.30`은 official gate 실패를 보여 준 `No-Go` 근거로 남는다.
- `1.31`은 열지 않는다.
- `1.26`은 이제 공식 오픈된 reserve path다.

## 이 문서가 가리키는 canonical 근거

- [Current Actual-Lane Handoff](./current-actual-lane-handoff-20260419.md)
- [Old First-Visible CPU Baseline Rerun](./old-first-visible-cpu-baseline-rerun-20260419.md)
- [Preview Track Route Decision](./preview-track-route-decision-20260418.md)
- [Preview Architecture History And Agent Guide](../preview-architecture-history-and-agent-guide.md)
- [Release Baseline](../release-baseline.md)
- [Booth Hardware Validation Architecture Research](./booth-hardware-validation-architecture-research.md)
