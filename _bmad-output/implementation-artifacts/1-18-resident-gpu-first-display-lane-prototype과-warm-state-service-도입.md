# Story 1.18: resident GPU-first display lane prototype과 warm-state service 도입

Status: backlog

Architecture Pivot Note: Story 1.17이 `canonical preset recipe + darktable adapter` 기준선을 이미 고정했다. 이번 스토리는 그 기준선을 소비하는 `resident GPU-first` 후보를 현 저장소의 `shadow / canary / default` 라우팅, dedicated renderer contract, booth-safe copy 원칙 위에서 검증 가능한 prototype으로 올리는 범위다.

## Story

owner / brand operator로서,
display lane의 기본 후보를 resident GPU-first service로 분명히 검증하고 싶다,
그래서 full-size preset-applied visible latency를 darktable-only 경로보다 더 직접적으로 줄일 수 있다.

## Acceptance Criteria

1. approved booth hardware와 capture-bound preset input이 있을 때, host-owned resident GPU-first lane 후보가 기존 `preview-job-v1` / `warmup-v1` 계약 또는 동등한 typed contract 위에서 동작해야 한다. 이 lane은 `canonical preset recipe`를 소비하는 truthful close 후보여야 하지만, session truth 자체를 재정의하면 안 된다.
2. preview route 승격은 기존 host-owned `branch-config/preview-renderer-policy.json` 경계 안에서만 이뤄져야 한다. `shadow`, `canary`, `default`, `rollback` 의미가 유지되어야 하며, dev-only 우회 토글이나 WebView/React 직접 호출이 release substitute가 되면 안 된다.
3. warm-state service는 preset/version별 준비 상태와 warm hit/miss를 남겨야 하고, session manifest / diagnostics / operator summary에서 `route`, `routeStage`, `laneOwner`, `fallbackReasonCode`, `hardwareCapability`와 함께 읽을 수 있어야 한다. 고객 화면에는 기술 용어가 노출되면 안 된다.
4. queue saturation, protocol mismatch, invalid output, wrong-session output, route-policy rollback, sidecar unavailable, warm-state loss가 발생해도 booth는 false-ready, false-complete, cross-session leakage 없이 approved darktable baseline / truthful fallback path로 내려가야 한다. `previewReady`와 `finalReady` 의미는 유지되어야 한다.
5. prototype 결과는 Story 1.19가 읽을 수 있는 seam evidence를 남겨야 한다. 최소한 `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`, lane owner, fallback reason, route stage, warm/cold hit 성격을 한 evidence family에서 다시 읽을 수 있어야 한다.
6. 이번 스토리는 resident GPU lane의 최종 promotion이나 hardware `Go`를 닫지 않는다. Story 1.13의 guarded cutover / release hold ownership을 유지하고, Story 1.19의 ETW/WPR/WPA/PIX + parity diff gate 준비까지만 이어져야 한다.

## Tasks / Subtasks

- [x] resident GPU-first candidate를 기존 sidecar/route boundary 위로 올린다. (AC: 1, 2, 4)
  - [x] `sidecar/dedicated-renderer/main.rs` shadow placeholder를 실제 resident warm-state prototype이 동작할 수 있는 실행 경계로 대체하거나, 동등한 sidecar binary를 같은 packaging boundary에 연결한다.
  - [x] `src-tauri/src/render/dedicated_renderer.rs`, `src-tauri/src/render/mod.rs`, `src-tauri/src/commands/preset_commands.rs`에서 현재 `local-renderer-sidecar` route를 resident GPU candidate로 확장하되, existing `preview-job-v1` / `warmup-v1` request/result contract와 canonical output validation을 유지한다.
  - [x] `src-tauri/tauri.conf.json`, `src-tauri/capabilities/*.json`, `scripts/prepare-dedicated-renderer-sidecar.mjs`, `src-tauri/build.rs`를 실제 resident binary packaging 경계와 맞춘다. 고객/운영 surface가 허용된 sidecar boundary 밖으로 새 실행 경로를 만들면 안 된다.

- [x] warm-state / route snapshot / operator diagnostics를 제품 기준으로 정렬한다. (AC: 2, 3, 5)
  - [x] `src-tauri/src/session/session_manifest.rs`, `docs/contracts/session-manifest.md`가 active preview route snapshot을 계속 보존하면서 warm-state 증거를 additive하게 담도록 맞춘다.
  - [x] `src-tauri/src/diagnostics/mod.rs`, `src/shared-contracts/schemas/operator-diagnostics.ts`, `src/shared-contracts/dto/operator.ts`, `src/operator-console/services/operator-diagnostics-service.ts`, `src/operator-console/screens/OperatorSummaryScreen.tsx`에서 route / routeStage / laneOwner / fallbackReasonCode / hardwareCapability를 유지하고, warm hit/miss 또는 동등 진단이 operator-safe vocabulary로 추가되게 한다.
  - [x] 고객 화면 copy와 `CaptureReadinessDto`는 그대로 booth-safe plain language만 유지한다. GPU, queue, sidecar, protocol, warm-state 같은 용어를 고객 surface에 노출하지 않는다.

- [x] fallback, isolation, same-capture guardrail을 resident candidate에도 고정한다. (AC: 1, 4, 6)
  - [x] `src-tauri/src/contracts/dto.rs`, `src/shared-contracts/schemas/dedicated-renderer.ts`, `docs/contracts/local-dedicated-renderer.md`, `docs/contracts/render-worker.md`가 warm-state service 도입 후에도 same-capture / same-session / canonical preview path 규칙을 유지하도록 정렬한다.
  - [x] route-policy invalid, route-policy rollback, sidecar unavailable, queue-saturated, protocol-mismatch, invalid-output, restarted, warm-state loss 케이스에서 inline truthful fallback 또는 approved baseline path로 내려가는 경계를 유지한다.
  - [x] active session이 이미 선택한 preset/version과 route snapshot을 나중 정책 변경으로 재해석하지 않는다는 규칙을 계속 지킨다.

- [x] Story 1.19로 넘길 seam evidence와 계측 준비를 만든다. (AC: 3, 5, 6)
  - [x] `capture_preview_transition_summary` 계열 진단이 `laneOwner`, `fallbackReason`, `routeStage`, `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`를 계속 남기게 유지한다.
  - [x] resident lane prototype이 warm/cold seam을 남길 수 있도록 `diagnostics/dedicated-renderer/` 또는 동등 evidence path를 정리한다.
  - [x] ETW/WPR/WPA/PIX와 parity diff의 최종 해석/승격 규칙은 Story 1.19에 남기고, 1.18은 계측 seam이 끊기지 않는 수준까지만 닫는다.

- [x] 계약/회귀/운영 검증을 준비한다. (AC: 2, 3, 4, 5)
  - [x] `cargo test --test dedicated_renderer`, `cargo test --test operator_diagnostics`, `cargo test --test session_manifest` 또는 동등 검증에 resident candidate, rollback, invalid policy, wrong-session rejection, operator projection 케이스를 보강한다.
  - [x] `pnpm test:run src/shared-contracts/contracts.test.ts src/operator-console/services/operator-diagnostics-service.test.ts src/operator-console/screens/OperatorSummaryScreen.test.ts src/governance/hardware-validation-governance.test.ts` 또는 동등 검증으로 shared contract / operator UI / release hold drift를 막는다.
  - [x] hardware `Go`는 주장하지 않고, pilot evidence package와 Story 1.19 handoff 조건만 명시한다.

### Review Findings

- [x] [Review][Patch] Resident prototype이 4바이트 JPEG stub만 써도 `previewReady` truthful close로 승격될 수 있음 [sidecar/dedicated-renderer/main.rs:6]
- [x] [Review][Patch] Operator preview architecture가 active preset의 최신 warm-state snapshot보다 이전 capture summary를 우선해 stale 상태를 표시할 수 있음 [src-tauri/src/diagnostics/mod.rs:540]
- [x] [Review][Patch] build script가 `TARGET` 이름만 바꾸고 실제 cross-target compile은 하지 않아 잘못된 sidecar 아키텍처를 패키징할 수 있음 [src-tauri/build.rs:14]
- [x] [Review][Patch] Resident sidecar가 실제 preset-applied 결과 대신 synthetic JPEG를 canonical output으로 기록해도 `previewReady` truthful close로 승격될 수 있음 [sidecar/dedicated-renderer/main.rs:192]
- [x] [Review][Patch] Background preview thread가 이전 capture의 preset warm-state를 현재 active preset snapshot 위에 덮어써 operator warm-state truth를 오염시킬 수 있음 [src-tauri/src/render/dedicated_renderer.rs:799]
- [x] [Review][Patch] Resident lane prototype이 아직 canonical truthful-close 후보로 동작하지 않고 warm-hit에서도 항상 fallback으로만 끝남 [sidecar/dedicated-renderer/main.rs:159]
- [x] [Review][Patch] Dedicated renderer sidecar가 요청 payload를 문자열 탐색으로 파싱해 유효한 JSON escape/path도 잘못 해석할 수 있음 [sidecar/dedicated-renderer/main.rs:297]
- [x] [Review][Patch] Warm-state 판단이 파일 존재 여부만 신뢰해 stale evidence나 warmup result 잔재를 실제 warm-hit 또는 warm-state-loss로 오분류할 수 있음 [sidecar/dedicated-renderer/main.rs:106]
- [x] [Review][Patch] Operator warm-state 최신성 비교가 초 단위로 잘려 같은 초 안의 더 새 진단을 stale snapshot으로 덮어쓸 수 있음 [src-tauri/src/diagnostics/mod.rs:563]
- [x] [Review][Patch] Warm-state contract가 문서상 제한 vocabulary와 달리 구현에서는 임의 문자열을 허용해 typed contract drift를 남김 [src/shared-contracts/schemas/dedicated-renderer.ts:59]
- [x] [Review][Patch] Dedicated renderer 실행이 멈추면 booth가 fallback으로 내려가지 못하고 preview waiting에 묶일 수 있음 [src-tauri/src/render/dedicated_renderer.rs:915]
- [x] [Review][Patch] Preview job shared contract가 warmup 전용 상태값 `warmed-up`을 허용해 host/runtime 계약과 drift가 생김 [src/shared-contracts/schemas/dedicated-renderer.ts:38]
- [x] [Review][Patch] Sidecar 소스 변경이 build 재실행 입력으로 선언되지 않아 패키징 시 오래된 resident binary가 그대로 남을 수 있음 [src-tauri/build.rs:3]
- [x] [Review][Patch] Dedicated renderer warm-up launch failure가 active warm-state snapshot을 갱신하지 않아 fallback 이후에도 operator가 이전 warmed 상태를 계속 볼 수 있음 [src-tauri/src/render/dedicated_renderer.rs:149]
- [x] [Review][Patch] Operator warm-state 최신성 비교가 여전히 초 단위라 같은 초 안의 더 새 snapshot이 stale diagnostics에 밀릴 수 있음 [src-tauri/src/diagnostics/mod.rs:563]
- [x] [Review][Patch] 동일 preset 재선택 시 누락된 warm-state snapshot을 기존 `manifest.updated_at`으로 되살려 실제보다 오래된 warm-state로 보이게 할 수 있음 [src-tauri/src/session/session_repository.rs:150]
- [x] [Review][Patch] `warmStateDetailPath`가 가리키는 warm-state evidence 파일에 `observedAt`이 없어 seam evidence 단독으로 freshness를 입증하지 못함 [sidecar/dedicated-renderer/main.rs:334]

## Dev Notes

### 스토리 범위와 제품 목적

- 이번 스토리는 resident GPU-first를 “주력 후보로 검증 가능한 상태”로 올리는 prototype story다.
- 목표는 전체 render/export 파이프라인 재작성이나 darktable 제거가 아니다. 사용자 체감과 직접 연결된 `display + preset apply` 경로를 더 빠르게 닫을 수 있는지 확인하는 것이다.
- 고객 경험 약속은 유지된다. same-capture first-visible이 먼저 보일 수 있어도 truthful close 전까지는 `Preview Waiting`이고, 실패 시에는 booth-safe fallback만 허용된다.

### 왜 지금 필요한가

- Story 1.17이 `canonical preset recipe` 기준을 먼저 잠갔기 때문에, 이제 GPU lane 후보가 consume해야 할 preset truth는 흔들리지 않는다. [Source: _bmad-output/implementation-artifacts/1-17-canonical-preset-recipe와-xmp-adapter-기준-동결.md]
- 2026-04-12 correct-course는 실행 우선순위를 `canonical preset recipe -> resident GPU-first display lane prototype -> ETW/WPR/WPA/PIX + parity diff`로 재정렬했다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260412-044022.md]
- 2026-04-11 technical validation도 첫 구현 타겟을 `display + preset apply resident GPU prototype`으로 두고, darktable를 `baseline / fallback / parity oracle`로 남기라고 정리했다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md]

### 현재 워크스페이스 상태

- `src-tauri/src/render/dedicated_renderer.rs`에는 이미 host-owned preview route policy가 있고, `preview-renderer-policy/v1` 기준으로 `shadow / canary / default / rollback`이 동작한다.
- 같은 파일은 `preview-job-v1`, `warmup-v1`, canonical output validation, wrong-session rejection, route snapshot 기록, `capture_preview_transition_summary` logging을 이미 갖고 있다.
- preset 선택 시 warm-up이 이미 트리거된다. `src-tauri/src/commands/preset_commands.rs`는 active preset 선택 후 dedicated renderer warm-up scheduling을 호출한다.
- operator path도 기본 바탕이 있다. `src-tauri/src/diagnostics/mod.rs`, `src/shared-contracts/schemas/operator-diagnostics.ts`, `src/operator-console/screens/OperatorSummaryScreen.tsx`는 이미 route / routeStage / laneOwner / fallback reason / hardware capability를 표시한다.
- 반면 실제 sidecar binary는 아직 placeholder 수준이다. `sidecar/dedicated-renderer/main.rs`는 shadow baseline stub만 두고 있고, `README.md`도 Story 1.11 baseline이라고 명시한다.
- 별도 `project-context.md`는 발견되지 않았다. 현재 canonical context는 planning artifacts, contract docs, current render/session/operator code다.

### 이전 스토리 인텔리전스

- Story 1.12는 dual-close topology와 same-slot truthful replacement를 제품 상태와 evidence로 고정했다. resident lane 후보는 이 semantics를 깨면 안 된다. [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- Story 1.13은 guarded cutover, rollback evidence, `preview-renderer-policy.json` ownership, release hold 판단을 소유한다. 1.18은 여기서 promotion claim을 가져오면 안 된다. [Source: _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md]
- Story 1.17은 canonical recipe와 darktable adapter 의미를 분리했다. resident lane은 XMP를 유일 truth처럼 해석하면 안 되고, recipe truth를 소비하는 lane이어야 한다. [Source: _bmad-output/implementation-artifacts/1-17-canonical-preset-recipe와-xmp-adapter-기준-동결.md]

### 제품/아키텍처 가드레일

- customer-facing copy는 계속 plain language만 사용해야 한다. GPU/OpenCL, sidecar, queue, protocol, warm-state, fallback reason 같은 내부 용어를 고객에게 노출하면 안 된다.
- `previewReady`, `finalReady`, `Completed` 의미는 유지되어야 한다. fast preview나 warm hit만으로 truthful close를 조기 승격하면 안 된다.
- active session route snapshot과 preset binding은 나중 정책 변경이나 rollback으로 재해석되면 안 된다.
- resident lane은 `single new truth engine`이 아니라 host-owned candidate lane이다. darktable는 baseline / fallback / parity oracle로 계속 남아야 한다.
- hardware `Go`와 promotion close owner는 여전히 Story 1.13 / 1.19다.

### 구현 가드레일

- existing `preview-job-v1` / `warmup-v1` typed contract를 가능한 한 유지하고, 깨야 한다면 TypeScript / Rust / tests / docs를 원자적으로 함께 갱신할 것.
- `preview-renderer-policy.json`이 없는 상태에서 default를 implicit GPU route로 바꾸지 말 것. missing 또는 invalid policy는 계속 `shadow` 해석을 유지하는 편이 안전하다.
- route-policy rollback, sidecar-unavailable, protocol-mismatch, invalid-output, restarted, queue-saturated는 모두 booth-safe fallback으로 연결돼야 한다.
- resident warm-state는 render availability/freshness를 다루는 계층이어야 하며, session manifest truth나 preset publication truth를 직접 소유하면 안 된다.
- placeholder sidecar를 실제 resident prototype으로 바꾸더라도 packaging path(`externalBin`, capability allowlist, prepare script)를 우회하는 ad-hoc 실행 경로를 만들지 말 것.

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `sidecar/dedicated-renderer/main.rs`
  - `sidecar/dedicated-renderer/README.md`
  - `scripts/prepare-dedicated-renderer-sidecar.mjs`
  - `src-tauri/build.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/capabilities/booth-window.json`
  - `src-tauri/capabilities/operator-window.json`
  - `src-tauri/capabilities/authoring-window.json`
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/diagnostics/mod.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src/shared-contracts/schemas/dedicated-renderer.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src/operator-console/services/operator-diagnostics-service.ts`
  - `src/operator-console/screens/OperatorSummaryScreen.tsx`
  - `docs/contracts/local-dedicated-renderer.md`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
  - `docs/runbooks/booth-hardware-validation-checklist.md`
- 새로 추가될 가능성이 큰 경로:
  - `sidecar/dedicated-renderer/src/**` 또는 동등 resident prototype source tree
  - `tests/hardware/resident-gpu-first/*`
  - `Pictures/dabi_shoot/branch-config/preview-renderer-policy.json` 샘플 또는 fixture

### 테스트 요구사항

- 최소 필수 자동 검증:
  - shadow policy에서는 dev env opt-in이나 sidecar 존재 여부와 무관하게 inline truthful fallback이 유지된다.
  - canary/default resident route에서는 accepted result만 canonical preview path를 닫고, invalid/wrong-session/non-canonical result는 fallback으로 내려간다.
  - route-policy rollback 이후에도 active session route snapshot은 재해석되지 않는다.
  - operator summary는 route / routeStage / laneOwner / fallbackReasonCode / hardwareCapability를 계속 읽는다.
  - customer-facing readiness/copy는 resident lane 여부와 무관하게 booth-safe semantics를 유지한다.
- 권장 추가 검증:
  - warm hit/miss evidence가 실제 per-session diagnostics에 남는지 확인
  - Story 1.19가 읽을 seam package (`firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`)가 pilot evidence에서 끊기지 않는지 확인

### 최신 기술 / 제품 컨텍스트

- 2026-04-11 technical validation은 Boothy의 주력 방향을 계속 `resident GPU-first + darktable baseline/fallback + canonical preset recipe + staged rollout`으로 유지했다. 이 스토리에서 `display + preset apply` prototype을 먼저 검증하는 이유도 그 결론에 따른 것이다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md]
- Adobe 공식 문서는 2025-08-13 업데이트에서 Lightroom Classic `14.5`부터 GPU preview generation을 지원한다고 설명한다. 이는 Boothy가 `preview responsiveness`를 GPU lane으로 끌어오는 방향이 제품적으로도 무리한 해석이 아니라는 보조 근거다. 이 문장은 Adobe 공식 문서를 현재 제품 방향에 적용한 해석이다. [Source: https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html]
- Tauri 2 공식 문서와 현재 repo 설정을 함께 보면, resident lane 후보는 계속 `externalBin + shell capability + host-owned command/event boundary` 안에 두는 편이 맞다. 이 문장은 공식 Tauri 문서와 현재 `tauri.conf.json` / capabilities 설정을 함께 적용한 해석이다. [Source: https://v2.tauri.app/concept/architecture/] [Source: https://v2.tauri.app/concept/inter-process-communication/] [Source: https://v2.tauri.app/security/capabilities/]
- darktable CLI 문서는 여전히 headless apply/export baseline을 설명한다. 따라서 darktable를 baseline / fallback / parity oracle로 두고 resident GPU lane을 별도 candidate로 보는 현재 아키텍처 방향은 유지된다. [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.18: resident GPU-first display lane prototype과 warm-state service 도입]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260412-044022.md]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md]
- [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- [Source: _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md]
- [Source: _bmad-output/implementation-artifacts/1-17-canonical-preset-recipe와-xmp-adapter-기준-동결.md]
- [Source: sidecar/dedicated-renderer/README.md]
- [Source: sidecar/dedicated-renderer/main.rs]
- [Source: scripts/prepare-dedicated-renderer-sidecar.mjs]
- [Source: src-tauri/tauri.conf.json]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: src-tauri/src/render/mod.rs]
- [Source: src-tauri/src/commands/preset_commands.rs]
- [Source: src-tauri/src/session/session_manifest.rs]
- [Source: src-tauri/src/diagnostics/mod.rs]
- [Source: src-tauri/src/contracts/dto.rs]
- [Source: src/shared-contracts/schemas/dedicated-renderer.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src/operator-console/screens/OperatorSummaryScreen.tsx]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: docs/release-baseline.md]
- [Source: https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html]
- [Source: https://v2.tauri.app/concept/architecture/]
- [Source: https://v2.tauri.app/concept/inter-process-communication/]
- [Source: https://v2.tauri.app/security/capabilities/]
- [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint-status, epics, architecture, PRD, correct-course proposal, 1.12/1.13/1.17 story context, dedicated renderer contract/code, operator diagnostics path, sidecar packaging 경계를 교차 분석했다.
- 기존 1.18 문서는 방향은 맞았지만, 현재 저장소에 이미 존재하는 `preview-renderer-policy`, route snapshot, operator diagnostics, sidecar packaging baseline을 충분히 반영하지 못하고 있었다.
- 이번 갱신은 resident GPU-first를 “새 구조 발명”이 아니라 “현 render/diagnostics/packaging 경계 위에서 prototype을 닫는 작업”으로 다시 정렬했다.
- 최신 외부 확인은 Adobe, Tauri, darktable 공식 문서만 사용했다.

### Debug Log References

- `rg -n "Story 1\\.18|resident GPU-first|warm-state|display lane" _bmad-output\\planning-artifacts\\epics.md _bmad-output\\planning-artifacts\\architecture.md _bmad-output\\planning-artifacts\\sprint-change-proposal-20260412-044022.md`
- `Get-Content sidecar\\dedicated-renderer\\README.md`
- `Get-Content sidecar\\dedicated-renderer\\main.rs`
- `Get-Content src-tauri\\src\\render\\dedicated_renderer.rs`
- `Get-Content src-tauri\\src\\diagnostics\\mod.rs`
- `Get-Content src\\operator-console\\screens\\OperatorSummaryScreen.tsx`
- `Get-Content src-tauri\\tauri.conf.json`
- `Get-Content scripts\\prepare-dedicated-renderer-sidecar.mjs`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118'; cargo test --test dedicated_renderer -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118'; cargo test --test operator_diagnostics -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118'; cargo test --test session_manifest -- --nocapture`
- `pnpm test:run src/shared-contracts/contracts.test.ts src/operator-console/services/operator-diagnostics-service.test.ts src/operator-console/screens/OperatorSummaryScreen.test.tsx src/governance/hardware-validation-governance.test.ts`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-fail'; cargo test --test dedicated_renderer resident_preview_prototype_keeps_warm_hit_but_does_not_claim_truthful_close -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-fail'; cargo test --test dedicated_renderer active_preset_warm_state_is_not_overwritten_by_older_capture_completion -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-verify'; cargo test --test dedicated_renderer -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-verify'; cargo test --test operator_diagnostics -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-verify'; cargo test --test session_manifest -- --nocapture`
- `rustc --test sidecar/dedicated-renderer/main.rs --edition=2021 -o target-codex-story118-sidecar-tests.exe`
- `target-codex-story118-sidecar-tests.exe --exact tests::preview_returns_accepted_after_warm_hit_when_render_succeeds --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-accepted'; cargo test --test dedicated_renderer resident_preview_warm_hit_claims_truthful_close_from_dedicated_renderer -- --nocapture`
- `target-codex-story118-sidecar-tests.exe --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-final'; cargo test --test dedicated_renderer -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-final'; cargo test --test operator_diagnostics -- --nocapture`
- `$env:CARGO_TARGET_DIR='C:\\Code\\Project\\Boothy_thumbnail-reset-at-2c89c40\\target-codex-story118-final'; cargo test --test session_manifest -- --nocapture`
- `pnpm test:run src/shared-contracts/contracts.test.ts src/operator-console/services/operator-diagnostics-service.test.ts src/operator-console/screens/OperatorSummaryScreen.test.tsx src/governance/hardware-validation-governance.test.ts`
- `pnpm lint` (pre-existing react-hooks/refs errors in `src/app/providers/app-providers.tsx`, `src/preset-authoring/providers/use-preset-authoring-service.ts`)
- `cargo fmt --check -- src/render/dedicated_renderer.rs tests/dedicated_renderer.rs build.rs`
- `rustfmt --check sidecar/dedicated-renderer/main.rs`
- `pnpm lint` (pre-existing react-hooks/refs errors in `src/app/providers/app-providers.tsx`, `src/preset-authoring/providers/use-preset-authoring-service.ts`)

### File List

- _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/local-dedicated-renderer.md
- docs/contracts/render-worker.md
- docs/contracts/session-manifest.md
- sidecar/dedicated-renderer/README.md
- sidecar/dedicated-renderer/main.rs
- sidecar/protocol/examples/preview-render-result.json
- src-tauri/build.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/diagnostics/mod.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/src/session/session_manifest.rs
- src-tauri/src/session/session_repository.rs
- src-tauri/tests/contracts_baseline.rs
- src-tauri/tests/dedicated_renderer.rs
- src-tauri/tests/operator_diagnostics.rs
- src/operator-console/screens/OperatorSummaryScreen.test.tsx
- src/operator-console/screens/OperatorSummaryScreen.tsx
- src/operator-console/services/operator-diagnostics-service.test.ts
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/dedicated-renderer.ts
- src/shared-contracts/schemas/operator-diagnostics.ts
- src/shared-contracts/schemas/session-manifest.ts

### Completion Notes

- resident dedicated renderer placeholder를 실제 실행 가능한 warm-state prototype binary로 교체해 packaging boundary 안에서 `warmup-v1` / `preview-job-v1` 요청을 처리하도록 정렬했다.
- active session manifest에 `activePreviewRendererWarmState` snapshot을 추가하고, operator diagnostics / operator console이 route 정보와 함께 warm state를 읽도록 맞췄다.
- `capture_preview_transition_summary` evidence family에 `warmState`를 추가하고, warm hit 및 warm-state loss fallback이 Story 1.19 seam evidence로 이어지도록 정리했다.
- resident route warmup, warm hit, stale-result fallback, operator projection, session manifest 정합성 회귀를 자동 테스트로 보강했다.
- hardware `Go` 또는 resident lane 최종 promotion은 닫지 않았고, Story 1.13 / 1.19 소유 release gate는 그대로 유지했다.
- resident prototype sidecar는 이제 placeholder output으로는 truthful close를 주장하지 않고, warm-hit evidence만 남긴 채 booth-safe inline fallback으로 내려가도록 고정했다.
- 이전 capture completion이 현재 활성 preset의 warm-state snapshot을 덮어쓰지 못하도록 manifest 동기화 가드를 추가해 operator warm-state truth 오염을 막았다.
- dedicated renderer 회귀, operator diagnostics, session manifest, shared contract/operator UI 검증은 모두 통과했다.
- resident warm-hit 경로가 이제 darktable CLI를 통해 canonical preview를 실제로 만들면 `accepted` 결과로 truthful close를 닫고, 실패 시에만 booth-safe fallback으로 내려가도록 승격했다.
- dedicated renderer integration 회귀에 실제 warm-hit close owner 케이스를 추가했고, sidecar 단위 테스트로 accepted render / stale warm-state / warmup residue 분기를 직접 고정했다.
- `pnpm lint`는 이번 변경과 무관한 기존 ref-access 규칙 위반 3건 때문에 여전히 실패했다.

### Change Log

- 2026-04-12: resident dedicated renderer warm-state prototype, operator warm-state projection, seam evidence 계측, 계약/회귀 테스트를 추가하고 스토리 상태를 review로 올림.
- 2026-04-12: review 후속으로 placeholder truthful-close 차단과 active preset warm-state overwrite guard를 추가하고 검증을 다시 통과시켰다.
- 2026-04-12: resident warm-hit가 actual canonical preview render를 만들면 dedicated renderer가 truthful close owner로 승격되도록 보강하고, sidecar/unit/integration 회귀를 다시 통과시켰다.
