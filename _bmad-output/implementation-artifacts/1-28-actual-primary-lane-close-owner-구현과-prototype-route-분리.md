# Story 1.28: actual primary lane close owner 구현과 prototype route 분리

Status: ready-for-dev

Ordering Note: Story 1.28은 Stories 1.23~1.27 prototype/evidence/gate history 다음에 시작되는 actual implementation track의 첫 스토리다. 이 스토리는 actual hot path 구현과 prototype route 분리를 소유하지만, Story 1.29 evidence/vocabulary realignment, Story 1.30 canary, Story 1.31 default/rollback, Story 1.13 final release close를 대신하지 않는다. [Source: _bmad-output/planning-artifacts/epics.md] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] [Source: docs/release-baseline.md]

Interpretation Note: 현재 워크스페이스는 `dedicated-renderer` vocabulary와 fixture가 넓게 퍼져 있다. Story 1.28의 안전한 목표는 actual primary lane을 실제 close owner로 구현하고 prototype route를 기능적으로 분리하는 것이지, 저장소 전체의 evidence/ledger/operator wording을 한 번에 rename하는 것이 아니다. 그 정렬 작업은 Story 1.29가 소유한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.29: actual primary lane evidence와 vocabulary realignment] [Source: src-tauri/src/render/dedicated_renderer.rs] [Source: tests/hardware-evidence-scripts.test.ts]

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
final architecture가 요구한 actual primary lane을 제품의 실제 close owner로 구현하고 싶다,
그래서 booth의 주 경로가 prototype dedicated-renderer evidence track과 분리되고 release-close 판단이 새 구조 위에서만 이뤄질 수 있게 하고 싶다.

## Acceptance Criteria

1. approved booth hardware와 approved preset/version scope가 있을 때 actual primary lane을 실행하면, `display-sized preset-applied truthful artifact`는 final architecture가 정의한 host-owned local native/GPU resident full-screen lane에서 닫혀야 한다. 또한 prototype dedicated-renderer route는 current primary close owner로 해석되면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.28: actual primary lane close owner 구현과 prototype route 분리] [Source: _bmad-output/planning-artifacts/architecture.md] [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
2. actual primary lane과 darktable-compatible reference를 함께 운영할 때 booth가 same-capture preview close를 계산하면, darktable-compatible path는 parity/fallback/final reference로만 남아야 한다. 또한 latency-critical hot path는 darktable preview invocation completion을 직접 기다리면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.28: actual primary lane close owner 구현과 prototype route 분리] [Source: docs/contracts/render-worker.md] [Source: docs/preview-architecture-history-and-agent-guide.md]
3. actual primary lane이 실패하거나 health를 잃을 때 booth가 fallback을 수행하면, false-ready, false-complete, wrong-capture, cross-session leakage 없이 fail-closed 되어야 한다. 또한 legacy prototype evidence나 route vocabulary가 actual lane success를 대체하면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.28: actual primary lane close owner 구현과 prototype route 분리] [Source: docs/contracts/local-dedicated-renderer.md] [Source: docs/release-baseline.md]

## Tasks / Subtasks

- [ ] actual primary lane close owner를 host-owned runtime에 구현한다. (AC: 1, 2)
  - [ ] `src-tauri/src/render/`, `src-tauri/src/capture/ingest_pipeline.rs`, `src-tauri/src/render/mod.rs` 또는 동등 경로에서 same-capture `display-sized preset-applied truthful artifact`를 실제 primary close owner로 만드는 actual lane을 도입한다.
  - [ ] actual lane hot path는 capture-bound `sessionId/requestId/captureId/presetId/publishedVersion`을 그대로 유지하고, `previewReady` 및 full-screen visible close를 actual artifact readiness 이후에만 닫게 유지한다.
  - [ ] darktable preview invocation completion, legacy dedicated-renderer completion, tiny preview, raw thumbnail, recent strip update는 actual lane success를 대신하면 안 된다.

- [ ] prototype route와 actual lane 의미를 기능적으로 분리한다. (AC: 1, 3)
  - [ ] `src-tauri/src/session/session_manifest.rs`, `src-tauri/src/branch_config/mod.rs`, `branch-config/preview-renderer-policy.json` 소비 경계 또는 동등 경로에서 capture-time route snapshot이 prototype route와 actual lane을 구분해 남도록 정리한다.
  - [ ] 이미 기록된 session/capture evidence는 live policy recopy나 later rollout change로 재해석하지 않게 유지한다.
  - [ ] broad vocabulary rewrite, ledger wording rename, dashboard copy sweep은 Story 1.29 범위로 남기고, Story 1.28에서는 actual implementation separation에 필요한 additive signal만 도입한다.

- [ ] darktable-compatible truth/parity reference와 fail-closed fallback을 유지한다. (AC: 2, 3)
  - [ ] `docs/contracts/render-worker.md`, `docs/contracts/local-dedicated-renderer.md`, `src-tauri/src/render/dedicated_renderer.rs` 또는 동등 경로에서 darktable-compatible path가 parity/fallback/final reference 역할을 계속 수행하게 유지한다.
  - [ ] actual lane health loss, timeout, invalid output, capture mismatch, preset drift 시 booth는 `Preview Waiting` 또는 approved fallback으로만 내려가고 false-ready/false-complete를 선언하지 않게 유지한다.
  - [ ] prototype dedicated-renderer result만 존재하는 상태를 actual lane `Go` 후보처럼 읽히게 만드는 shortcut을 금지한다.

- [ ] selected-capture evidence와 operator-safe diagnostics를 actual implementation 경계에 맞게 보강한다. (AC: 1, 3)
  - [ ] `src/shared-contracts/schemas/hardware-validation.ts`, `src/shared-contracts/schemas/operator-diagnostics.ts`, `src/operator-console/services/operator-diagnostics-service.ts`, `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1` 또는 동등 경로에서 actual lane provenance가 selected capture 문맥으로 남도록 필요한 additive field를 정리한다.
  - [ ] evidence bundle은 `visibleOwner`, `visibleOwnerTransitionAtMs`, `sameCaptureFullScreenVisibleMs`, route/catalog snapshot, fallback reason을 계속 fail-closed로 요구하고, prototype-only evidence로 actual lane success를 통과시키면 안 된다.
  - [ ] Story 1.29에서 vocabulary realignment를 하기 전까지는 existing bundle/script family를 깨지 않도록 additive migration 형태를 우선한다.

- [ ] 회귀 테스트를 actual implementation start 기준으로 잠근다. (AC: 1, 2, 3)
  - [ ] Rust test: actual lane success, darktable wait-free hot path, capture-time snapshot continuity, fail-closed fallback, prototype-result-only rejection을 검증한다.
  - [ ] TypeScript/PowerShell test: hardware evidence bundle과 operator diagnostics가 actual lane proof 부재 시 fail-closed 되는지, prototype-only vocabulary가 actual lane success를 대체하지 못하는지 검증한다.
  - [ ] Story 1.28 완료 의미는 actual lane implementation start와 route separation까지이며, canary `Go`, default 승격, rollback proof, final release close를 요구하지 않는다는 범위 문구를 테스트/문서에서 유지한다.

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- release baseline과 sprint status는 Stories 1.23~1.27을 prototype/evidence/gate history로 고정하고, Stories 1.28~1.31을 actual-lane forward path로 재정렬했다. 따라서 Story 1.28은 "기존 prototype을 조금 더 다듬는 일"이 아니라 actual primary lane 자체를 제품 코드 위에 올리는 시작점이어야 한다. [Source: docs/release-baseline.md] [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- preview architecture history는 dedicated renderer activation 자체는 성공했지만, repeated approved-hardware KPI를 닫는 final primary close architecture로는 부족했다고 정리한다. 즉 이제 필요한 것은 새 actual lane 구현이지 dedicated renderer 미세조정 반복이 아니다. [Source: docs/preview-architecture-history-and-agent-guide.md]
- architecture change proposal은 primary architecture를 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`로 다시 선언하고, existing dedicated renderer는 activation baseline으로 강등한다. Story 1.28은 이 문서 결정을 실제 구현 경계로 옮기는 owner다. [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]

### 스토리 목적과 범위

- 이번 스토리가 소유하는 것:
  - actual primary lane close owner 구현 시작
  - prototype dedicated-renderer route와 actual lane의 기능적 분리
  - darktable-compatible reference 및 fail-closed fallback 유지
  - selected-capture evidence에 actual implementation provenance를 남길 최소 additive signal 정리
- 이번 스토리가 소유하지 않는 것:
  - actual-lane evidence/vocabulary 전체 realignment
  - actual-lane hardware canary `Go / No-Go`
  - actual-lane default promotion과 one-action rollback proof
  - final guarded cutover / release ledger close
  - repeated KPI failure 이후 reserve remote path 개시

### 현재 워크스페이스 기준선

- 현재 render/runtime/evidence 코드는 `src-tauri/src/render/dedicated_renderer.rs` 중심으로 `laneOwner`, `visibleOwner`, `sameCaptureFullScreenVisibleMs`, `preview-renderer-policy.json` snapshot을 남기고, 테스트와 runbook도 같은 family를 기준으로 움직인다. Story 1.28은 이 baseline을 무시하고 새 evidence family를 병렬로 발명하면 안 된다. [Source: src-tauri/src/render/dedicated_renderer.rs] [Source: tests/hardware-evidence-scripts.test.ts] [Source: docs/runbooks/preview-promotion-evidence-package.md]
- `session-manifest/v1`은 active session이 capture-time route snapshot과 warm-state snapshot을 host-owned truth로 보존한다고 명시한다. actual lane 구현도 later policy change나 fallback change가 이미 기록된 capture meaning을 재해석하게 만들면 안 된다. [Source: docs/contracts/session-manifest.md] [Source: src-tauri/src/session/session_manifest.rs]
- `sidecar/dedicated-renderer/`와 prepare script는 여전히 packaged prototype/shadow binary 경계를 소유한다. Story 1.28은 이 경계를 완전히 지우는 작업보다, actual lane이 이것과 다른 current close owner임을 코드 경계로 드러내는 작업에 가깝다. [Source: sidecar/dedicated-renderer/README.md] [Source: scripts/prepare-dedicated-renderer-sidecar.mjs]
- branch-config / governance / operator surface는 아직 prototype-track language를 넓게 사용한다. broad wording sweep은 Story 1.29가 더 안전하므로, Story 1.28은 implementation separation에 필요한 additive distinction만 도입하는 편이 맞다. [Source: src/branch-config/components/PreviewRouteGovernancePanel.tsx] [Source: src/shared-contracts/schemas/branch-rollout.ts] [Source: _bmad-output/planning-artifacts/epics.md#Story 1.29: actual primary lane evidence와 vocabulary realignment]
- 별도 `project-context.md`는 발견되지 않았다.

### 아키텍처 준수사항

- customer-visible truthful close owner는 `host-owned local native/GPU resident full-screen lane`이어야 하며, 결과물은 `display-sized preset-applied truthful artifact`다. Story 1.28은 이 문장을 실제 runtime ownership으로 반영해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md] [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
- darktable-compatible path는 fidelity oracle, parity validation reference, booth-safe fallback path, final/export truth reference로 남아야 한다. actual lane 구현이 이 경로를 제거하거나 약화하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md] [Source: docs/contracts/render-worker.md]
- `previewReady`는 first-visible, tiny preview, recent-session-visible, prototype proof로 닫히면 안 되고, same-capture preset-applied truthful artifact가 실제 준비된 뒤에만 닫혀야 한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness] [Source: docs/contracts/render-worker.md]
- capture-bound truth/evidence contract는 same-capture, same-session, same-preset-version correlation을 유지해야 하며, selected-capture evidence bundle이 route/catalog snapshot과 visible owner transition을 같이 보존해야 한다. [Source: docs/contracts/local-dedicated-renderer.md] [Source: docs/runbooks/preview-promotion-evidence-package.md]

### 구현 가드레일

- prototype dedicated-renderer result를 단순 rename하거나 `laneOwner` string만 바꿔서 actual lane 구현 완료처럼 보이게 만들면 안 된다.
- actual lane hot path가 darktable preview invocation completion이나 parity reference completion을 직접 기다리면 안 된다.
- live `preview-renderer-policy.json` 또는 live catalog state를 다시 읽어 과거 capture의 의미를 재해석하면 안 된다.
- evidence/diagnostics 문구가 아직 prototype vocabulary를 쓴다는 이유로 actual lane provenance를 남기지 않는 것도 금지다. vocabulary sweep이 어렵다면 additive field 또는 bounded discriminator로 actual lane을 분리해야 한다.
- Story 1.28 안에서 Story 1.29, 1.30, 1.31, 1.13의 책임을 함께 흡수하면 안 된다.
- remote renderer / edge appliance를 local actual-lane 검증 전 기본 대안처럼 열면 안 된다. [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/branch_config/mod.rs`
  - `sidecar/dedicated-renderer/main.rs`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/operator-console/services/operator-diagnostics-service.ts`
  - `tests/hardware-evidence-scripts.test.ts`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/local-dedicated-renderer.md`
  - `docs/contracts/session-manifest.md`
  - `docs/runbooks/preview-promotion-evidence-package.md`
- scope를 넘기지 않도록 주의할 경로:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-hardware-validation-gate.md`
  - `_bmad-output/implementation-artifacts/1-30-actual-primary-lane-hardware-canary-재검증.md`
  - `_bmad-output/implementation-artifacts/1-31-actual-primary-lane-default-decision과-rollback-gate.md`
  - `_bmad-output/implementation-artifacts/1-26-local-lane-실패-시에만-remote-reserve-poc-개시.md`

### 최신 기술 확인 메모

- 현재 저장소 기준선은 `React 19.2.4`, `react-router-dom 7.13.1`, `Zod 4.3.6`, `Vite 8.0.1`, `@tauri-apps/api 2.10.1`, `tauri 2.10.3`, Rust `edition 2021`, `rust-version 1.77.2`다. Story 1.28은 새 프레임워크 도입보다 이 기준선 안에서 actual lane ownership을 정리하는 편이 안전하다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- Tauri v2 공식 문서는 frontend-host 경계를 `invoke(...)`와 Rust `#[tauri::command]` 기반으로 설명한다. actual lane의 무거운 close 작업은 계속 host/native boundary에 두고, React가 렌더 hot path를 직접 소유하게 만들면 안 된다. [Source: https://v2.tauri.app/develop/calling-rust/]
- React 공식 문서의 `startTransition`와 `useDeferredValue`는 비차단 UI 업데이트에 유용하지만, host latency 자체를 숨기는 대안은 아니다. Story 1.28에서 booth UI가 actual lane 상태 변화 때문에 무거워진다면 보조적으로 고려할 수 있지만, 제품 KPI를 닫는 수단으로 오해하면 안 된다. [Source: https://react.dev/reference/react/startTransition] [Source: https://react.dev/reference/react/useDeferredValue]
- Zod 4 공식 문서는 TypeScript-first schema validation을 계속 권장한다. actual lane provenance도 ad-hoc JSON parsing보다 existing shared-contract schema family에 additive field를 넣는 편이 안전하다. [Source: https://zod.dev/]
- darktable command-line reference는 여전히 parity/fallback/final reference path의 운영 근거지만, hot path close owner로 복귀해도 된다는 뜻은 아니다. Story 1.28은 darktable를 reference path로 유지하되 latency-critical owner에서 분리해야 한다. [Source: https://darktable-org.github.io/dtdocs/en/darktable_user_manual.pdf]

### 테스트 요구사항

- 최소 필수 검증:
  - actual lane success path가 same `sessionId/requestId/captureId/presetId/publishedVersion` 기준의 truthful artifact close를 만든다.
  - darktable reference path가 살아 있어도 actual lane hot path가 darktable completion을 직접 기다리지 않는다.
  - prototype-only result 또는 prototype-only evidence로 actual lane success verdict가 닫히지 않는다.
  - actual lane health loss, invalid output, timeout, capture mismatch 시 booth가 fail-closed fallback으로 내려간다.
  - selected capture evidence가 `visibleOwner`, `visibleOwnerTransitionAtMs`, `sameCaptureFullScreenVisibleMs`, route/catalog snapshot을 계속 보존한다.
  - later policy change나 rollback이 이미 기록된 capture route meaning을 재해석하지 않는다.
- 권장 추가 검증:
  - operator diagnostics가 actual lane provenance와 prototype baseline을 동시에 읽더라도 speed-only success로 요약하지 않는지 확인
  - evidence bundle assembly가 additive actual-lane field가 없어도 기존 prototype bundle을 깨지 않으면서, actual-lane proof 부재는 fail-closed로 판단하는지 확인
  - capture follow-up health와 preview close health가 한 bundle 안에서 여전히 분리 판독되는지 확인

### 안티패턴

- `dedicated-renderer` string rename만으로 actual lane 구현이 끝났다고 주장하는 것
- actual lane hot path가 darktable preview result, parity diff, final/export reference를 직접 기다리게 만드는 것
- selected-capture evidence 대신 whole-session 로그나 later policy state를 읽어 성공을 재구성하는 것
- Story 1.28에서 ledger wording, dashboard wording, rollout gate wording까지 한 번에 대수술하는 것
- prototype track 완료 사실을 actual-lane canary/default/release close proof로 재사용하는 것
- reserve remote path를 local actual-lane track보다 먼저 제품 기본 경로처럼 취급하는 것

### References

- [Source: _bmad-output/planning-artifacts/epics.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: docs/release-baseline.md]
- [Source: docs/preview-architecture-history-and-agent-guide.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: history/camera-capture-validation-history.md]
- [Source: sidecar/dedicated-renderer/README.md]
- [Source: scripts/prepare-dedicated-renderer-sidecar.mjs]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: src-tauri/src/session/session_manifest.rs]
- [Source: src-tauri/src/branch_config/mod.rs]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src/operator-console/services/operator-diagnostics-service.ts]
- [Source: src/branch-config/components/PreviewRouteGovernancePanel.tsx]
- [Source: src/shared-contracts/schemas/branch-rollout.ts]
- [Source: tests/hardware-evidence-scripts.test.ts]
- [Source: src-tauri/tests/dedicated_renderer.rs]
- [Source: src-tauri/tests/capture_readiness.rs]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]
- [Source: https://v2.tauri.app/develop/calling-rust/]
- [Source: https://react.dev/reference/react/startTransition]
- [Source: https://react.dev/reference/react/useDeferredValue]
- [Source: https://zod.dev/]
- [Source: https://darktable-org.github.io/dtdocs/en/darktable_user_manual.pdf]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- 2026-04-16 기준 config, sprint status, epics, PRD, architecture, architecture change proposal, release baseline, preview history, capture validation history, Story 1.23/1.25/1.27, current code/test paths를 교차 분석해 Story 1.28 문서를 새로 생성했다.
- 이번 문서는 actual lane 구현과 prototype route separation을 우선 과제로 고정하고, vocabulary/ledger realignment는 Story 1.29로 분리해 scope creep를 막도록 설계했다.
- 별도 `project-context.md`는 발견되지 않았다.

### Debug Log References

- `Get-Content -Raw _bmad/bmm/config.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `rg -n -C 80 "Story 1\\.28|Story 1\\.29|Story 1\\.27|actual primary lane|prototype route" _bmad-output/planning-artifacts/epics.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/architecture.md`
- `Get-Content -Raw _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md`
- `Get-Content -Raw docs/release-baseline.md`
- `Get-Content -Raw docs/preview-architecture-history-and-agent-guide.md`
- `Get-Content -Raw history/camera-capture-validation-history.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-25-local-lane-default-decision과-rollback-gate.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-27-local-hot-path-darktable-절연과-2500ms-kpi-재검증.md`
- `rg -n "dedicated-renderer|laneOwner|preview-renderer-policy|visibleOwner|sameCaptureFullScreenVisibleMs" src src-tauri sidecar docs scripts tests`
- `Get-Content -Raw package.json`
- `Get-Content -Raw src-tauri/Cargo.toml`

