# Story 1.26: host-owned local native/GPU resident preview lane 검증

Status: review

Correct Course Note: `2026-04-20` preview-track route decision에 따라, Story `1.10` old `resident first-visible` line은 closed `No-Go` baseline으로 고정하고, Story `1.26`이 다음 official reserve path를 소유한다. 이번 스토리의 목적은 darktable hot path를 더 미세조정하는 것이 아니라, `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact` 범위로 새 preview route를 좁게 정의하고 승인 하드웨어에서 official gate를 다시 검증하는 것이다.

## Current Role In This Worktree

- `2026-04-20` 기준 이 문서는 현재 preview-track의 active reserve path story다.
- current official release judgment는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`, 즉 `preset-applied visible <= 3000ms` 하나뿐이다.
- `sameCaptureFullScreenVisibleMs`와 first-visible 수치는 계속 남기되, reference / comparison / feel metric으로만 읽는다.
- Story `1.30`은 actual-primary-lane bounded `No-Go` evidence, Story `1.10`은 old line closed `No-Go` baseline, Story `1.31`은 unopened success-side default/rollback gate다.
- old line GPU/OpenCL comparison은 필요 시 side evidence로 남길 수 있지만, 이 story의 primary critical path는 아니다.

### Canonical Reading Order

- 이 story만 단독으로 읽지 않는다.
- 먼저 `docs/README.md`, `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`, `docs/runbooks/current-preview-gpu-direction-20260419.md`, `docs/runbooks/preview-track-route-decision-20260418.md`를 읽고 해석한다.
- official `Go / No-Go` ownership은 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 소유한다.

### Validation Gate Reference

- Supporting evidence family:
  - approved booth hardware latency package
  - per-session seam log package
  - truthful `Preview Waiting -> Preview Ready` evidence
- Current hardware gate:
  - `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
  - product wording: `preset-applied visible <= 3000ms`
- Reference / comparison metrics:
  - `sameCaptureFullScreenVisibleMs`
  - first-visible product feel
- Official verdict owner:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Close policy:
  - automated pass만으로 닫지 않는다.
  - 승인 하드웨어 one-session package가 same-session correctness, truthful close, wrong-capture 0, cross-session leakage 0, official gate 판정을 함께 닫아야 한다.
  - darktable parity/fallback/final-export path는 유지하되, per-capture preview hot path owner로 되돌리지 않는다.

## Story

booth customer로서,
프리셋이 적용된 확인용 사진이 제품 기준 시간 안에 보여지길 원한다.
그래서 방금 찍은 사진이 보여도 실제 확인용 결과를 오래 기다리거나, truth가 느슨해지는 일을 겪지 않는다.

## Current Validation Questions

1. host-owned local native/GPU resident lane이 current darktable hot path보다 official gate에 실제로 더 가까운가
2. display-sized preset-applied truthful artifact를 current customer contract 안에서 close owner로 유지할 수 있는가
3. same-session, same-capture correctness와 `Preview Waiting` truth를 유지한 채 hot path 비용을 줄일 수 있는가
4. darktable를 parity reference, fallback, final/export truth로 남겨도 booth-visible preview gate를 닫을 수 있는가

## Acceptance Criteria

1. reserve path는 `host-owned local native/GPU resident full-screen lane`을 current booth-visible preview hot path의 주 경계로 사용해야 한다. repeated per-capture `darktable-cli` close ownership은 primary hot path로 복귀하면 안 된다.
2. reserve path가 booth에 표시하는 truthful close asset은 `display-sized preset-applied truthful artifact`여야 한다. `previewReady`, `preview.readyAtMs`, 관련 readiness update는 이 artifact만 소유할 수 있다.
3. first-visible 또는 intermediate image가 더 먼저 보이더라도 booth는 truthful `Preview Waiting`을 유지해야 하며, preset-applied truthful artifact가 실제로 닫히기 전까지 false-ready를 만들면 안 된다.
4. reserve path는 same-session, same-capture correctness, same-slot continuity, wrong-capture discard, cross-session leakage 0 규칙을 유지해야 한다.
5. approved booth hardware 검증에서는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`만 official pass condition으로 읽어야 하며, `sameCaptureFullScreenVisibleMs`와 first-visible feel은 reference/comparison metric으로만 남겨야 한다.
6. reserve path가 official gate를 못 닫거나 correctness guardrail을 깨면 story는 `review` 또는 동등 route-hold 상태에 남고, Story `1.31`은 이 결과만으로 자동 재오픈되면 안 된다.

## Tasks / Subtasks

- [x] reserve topology boundary를 고정한다. (AC: 1, 2, 3, 4, 5)
  - [x] host-owned local native/GPU resident full-screen lane의 owner boundary를 문서와 코드에서 한 곳으로 고정한다.
  - [x] `display-sized preset-applied truthful artifact`가 어떤 시점과 경로에서 생성되고 `previewReady`를 소유하는지 명시한다.
  - [x] darktable path가 parity reference, fallback, final/export truth로 남는 경계를 분리한다.

- [x] truthful close ownership과 booth contract를 유지한다. (AC: 2, 3, 4)
  - [x] first-visible / intermediate asset이 있어도 `Preview Waiting`과 later truthful close ownership이 느슨해지지 않게 한다.
  - [x] same-slot continuity, wrong-capture discard, cross-session isolation guardrail을 reserve path에 맞게 다시 잠근다.
  - [x] `previewReady`가 non-truthful asset에서 먼저 올라가지 않도록 회귀를 막는다.

- [ ] per-session instrumentation과 gate readout을 유지한다. (AC: 4, 5)
  - [ ] one-session package만으로 official gate와 reference metrics를 함께 읽을 수 있게 seam logging을 유지하거나 보강한다.
  - [ ] request-level correlation 키가 preview hot path와 truthful close까지 이어지도록 유지한다.
  - [ ] ledger readout에 필요한 evidence path 형식을 미리 고정한다.

- [ ] hardware validation package를 수집한다. (AC: 5, 6)
  - [x] 승인 하드웨어 one-session package를 수집한다.
  - [x] official gate, correctness, truth ownership을 ledger에 기록한다.
  - [x] 결과에 따라 `Go` 또는 bounded `No-Go`를 선언한다.

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 old line 미세 조정이 아니다.
- 이번 스토리는 `darktable-only hot path tuning`이 아니라 `새 reserve topology 검증`이다.
- 제품 약속은 그대로다. 고객이 먼저 같은 컷을 볼 수는 있어도, official 성공은 `preset-applied visible <= 3000ms`를 닫는지로만 읽는다.
- Story `1.31`은 이번 스토리 성공 전까지 열지 않는다.

### 왜 이 스토리가 열렸는가

- Story `1.30` actual-primary-lane은 repeated hardware rerun 끝에 bounded `No-Go`로 닫혔다.
- Story `1.10` old `resident first-visible` line도 latest approved hardware session에서 baseline evidence package는 다시 닫았지만 official gate는 `8972ms`, `7942ms`, `7967ms`로 실패했다.
- 따라서 preview-track은 old lane comparison evidence를 유지하되, 실제 다음 구현 경로는 새 reserve topology로 넘어가야 한다.

### 구현 순서

1. reserve topology owner boundary를 고정한다.
2. truthful close owner를 `display-sized preset-applied truthful artifact`로 다시 닫는다.
3. instrumentation과 same-capture correctness guardrail을 맞춘다.
4. 승인 하드웨어 one-session package를 수집한다.
5. ledger `Go / No-Go`로 route를 판정한다.

### 구현 가드레일

- official gate를 `sameCaptureFullScreenVisibleMs`로 바꾸지 말 것.
- first-visible source나 intermediate asset을 truth owner로 승격하지 말 것.
- old line historical better run을 release proof처럼 해석하지 말 것.
- darktable path를 per-capture preview hot path owner로 조용히 되돌리지 말 것.
- story note만으로 route success를 선언하지 말 것.

### 아키텍처 준수사항

- host가 normalized truth owner라는 구조는 유지한다.
- session truth는 계속 `session.json`과 session-scoped filesystem root가 소유한다.
- darktable는 parity reference, fallback, final/export truth로 남고, booth-visible preview gate를 닫는 primary hot path와는 분리한다.
- front-end는 raw helper status나 low-level renderer status를 직접 해석하지 않고 host-normalized truth만 소비해야 한다.

### 프로젝트 구조 요구사항

- 우선 검토 후보 경로:
  - `src-tauri/src/render/`
  - `src-tauri/src/capture/`
  - `src-tauri/src/commands/`
  - `src-tauri/src/timing/`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
  - `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
- 새 reserve path는 기존 preview truth 계약을 깨지 않는 범위에서 추가/분리되어야 한다.

### 테스트 요구사항

- reserve path hot path가 truthful close ownership과 분리돼도 `Preview Waiting` truth가 유지된다.
- same-session, same-capture correctness, wrong-capture discard, cross-session leakage 0가 유지된다.
- official gate metric과 reference metrics가 한 session diagnostics path에서 함께 읽힌다.
- reserve path fail 시 bounded fallback이 동작하고 false-ready가 생기지 않는다.

### References

- [Source: docs/runbooks/story-1-26-reserve-path-opening-20260420.md]
- [Source: docs/runbooks/current-preview-gpu-direction-20260419.md]
- [Source: docs/runbooks/preview-track-route-decision-20260418.md]
- [Source: docs/release-baseline.md]
- [Source: docs/preview-architecture-history-and-agent-guide.md]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.26: host-owned local native/GPU resident reserve path와 display-sized preset-applied truthful artifact 검증]
- [Source: _bmad-output/planning-artifacts/prd.md#Current Preview Release Interpretation]
- [Source: _bmad-output/planning-artifacts/architecture.md#Gap Analysis Results]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]

## Creation Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-20 10:58:00 +09:00 - Story `1.10` latest approved hardware rerun result, preview route docs, PRD/epics/architecture interpretation, and hardware ledger ownership을 함께 읽고 Story `1.26` reserve path opening scope를 정리했다.
- 2026-04-20 11:37:12 +09:00 - reserve path truthful close owner를 `preset-applied-preview` 계약으로 고정하고, `src-tauri/src/capture/ingest_pipeline.rs`, `docs/contracts/render-worker.md`, `docs/contracts/session-manifest.md`, `src-tauri/tests/capture_readiness.rs`를 함께 갱신했다.
- 2026-04-20 11:37:12 +09:00 - `cargo test -- --test-threads=1`, `pnpm test:run`, `pnpm lint`를 실행해 reserve path truthful close 회귀와 기존 booth 흐름이 현재 worktree 기준 모두 통과함을 확인했다.
- 2026-04-20 11:54:11 +09:00 - 승인 하드웨어 최신 세션 `session_000000000018a7f0faf87fd164`를 읽어 one-session package를 수집했다. official gate는 실패했고, field evidence의 actual close owner는 여전히 `darktable-cli + raw-original`로 남아 있었다.
- 2026-04-20 12:46:22 +09:00 - owner attribution 수정 뒤 승인 하드웨어 최신 세션 `session_000000000018a7f3c5b88c698c`를 다시 읽었다. 2장~4장은 field evidence에서 `preset-applied-preview` close owner가 관찰됐지만 official gate는 여전히 `8616ms`, `7712ms`, `8165ms`, `7643ms`로 실패했고 1장은 `raw-original` close로 남았다.

### Completion Notes List

- Story `1.26` reserve path를 공식 오픈 상태로 정의했다.
- scope를 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`로 좁게 고정했다.
- official gate, comparison metric, correctness guardrail, darktable boundary를 분리했다.
- `preset-applied-preview`가 canonical preview path에 닫히면 host가 즉시 truthful `previewReady`를 기록하고 per-capture `darktable-cli` preview close를 다시 열지 않도록 reserve path owner boundary를 코드에 고정했다.
- `Preview Waiting` truth와 same-capture guardrail을 유지한 채 reserve path truthful close를 검증하는 Rust regression을 추가했고, existing Rust/Vitest/lint 회귀를 현재 worktree에서 다시 통과시켰다.
- approved hardware one-session package는 실제로 수집했다.
- 다만 최신 field evidence에서 intended reserve truthful close owner인 `preset-applied-preview`는 관찰되지 않았고, actual close owner는 여전히 `darktable-cli + raw-original`이었다.
- official `preset-applied visible <= 3000ms` gate도 `7486ms`, `7716ms`, `8796ms`로 실패했고 첫 샷은 `preview-render-failed`로 `previewWaiting`에 남았다.
- owner attribution 수정 뒤 최신 approved hardware rerun에서는 2장~4장이 `preset-applied-preview` close owner로 field evidence에 남았고, 지난 회차의 logging mismatch blocker는 주된 원인이 아니게 됐다.
- 그러나 official `preset-applied visible <= 3000ms` gate는 최신 회차에서도 `8616ms`, `7712ms`, `8165ms`, `7643ms`로 실패했고, 첫 샷은 여전히 `raw-original` close로 남았다.
- 따라서 story는 `review / No-Go`로 유지하고, 다음 단계는 hardware rerun 반복이 아니라 `first-shot coverage`와 `preset-applied close latency`를 먼저 줄이는 것이다.

### File List

- _bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md
- docs/contracts/render-worker.md
- docs/contracts/session-manifest.md
- docs/runbooks/story-1-26-reserve-path-opening-20260420.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/tests/capture_readiness.rs

### Change Log

- 2026-04-20 - Story `1.26` reserve path를 공식 오픈하고 current preview-track active route로 정의했다.
- 2026-04-20 - reserve path truthful close owner를 `preset-applied-preview` 계약으로 연결하고, darktable preview close가 fallback/parity 경계로만 남도록 software boundary와 regression coverage를 추가했다.
- 2026-04-20 - 승인 하드웨어 one-session package를 수집했지만, reserve path intended close owner가 field evidence에서 관찰되지 않아 Story `1.26`을 hardware `No-Go`로 기록했다.
- 2026-04-20 - owner attribution 수정 뒤 approved hardware rerun에서 `preset-applied-preview` close owner는 field evidence에 보였지만, official gate 실패와 first-shot raw-original close가 남아 Story `1.26`은 계속 hardware `No-Go`로 유지됐다.
