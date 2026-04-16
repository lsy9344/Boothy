# Story 1.13: guarded cutover와 original-visible-to-preset-applied-visible hardware validation gate

Status: backlog

Architecture Sequencing Note: preview architecture adoption 순서는 `1.21 metric reset -> 1.22 evidence chain reset -> 1.23 local full-screen lane prototype -> 1.24 hardware canary validation -> 1.25 default decision and rollback gate -> 1.27 corrective local hot path validation -> 1.13 guarded cutover / release close`로 읽어야 한다. 이번 스토리는 resident lane를 새로 활성화하는 단계가 아니라, Story 1.25의 default/rollback proof와 Story 1.27의 corrective local hot-path evidence를 승인된 부스 장비에서 최종 `Go / No-Go`로 닫는 canonical release-close owner다.

### Validation Gate Reference

- Prerequisite:
  - Story 1.25 rollback proof와 Story 1.27 canonical local-lane `Go` candidate evidence 확보
  - approved preset/version scope가 host-owned `preview-renderer-policy.json`에서 승인된 local-lane proof 기준으로 관리되는 상태
  - repeated resident success-path evidence 확보
- Supporting evidence family:
  - `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`
  - `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`
  - route policy snapshot (`branch-config/preview-renderer-policy.json`)
  - published `bundle.json`, `catalog-state.json`
  - booth/operator visual evidence
  - one-action rollback proof
  - canonical hardware ledger `Go / No-Go` row
- Current hardware gate: `No-Go`
- Close policy:
  - Story 1.19 evidence package와 automated pass만으로는 release-truth `Go`를 주장하지 않는다.
  - Story 1.13은 activation 이전의 implementation corrective를 흡수하지 않는다.
  - promoted resident lane success path, parity, fallback 안정성, rollback evidence 중 하나라도 비면 `No-Go`로 유지한다.

## Story

As a owner / brand operator,
I want activation이 끝난 resident preview lane를 guarded cutover 기준으로 최종 검증하고 싶다,
so that booth가 original-visible responsiveness와 preset-applied truth, rollback safety를 함께 만족한 구조만 release-safe하게 승격할 수 있다.

## Acceptance Criteria

1. Story 1.25 rollback proof와 Story 1.27 canonical local-lane `Go` candidate evidence가 완료된 approved scope에서만 Story 1.13 rerun이 시작되어야 한다. `preview-renderer-policy.json`은 host-owned rollout artifact로만 제어되어야 하며, active session은 route policy 변경으로 재해석되면 안 된다. `No-Go` 시 one-action rollback 경로가 남아 있어야 한다.
2. 승인된 Windows booth hardware에서 canonical evidence package를 fresh capture 기준으로 다시 수집해야 한다. 패키지는 최소 `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`, route policy snapshot, published `bundle.json`, `catalog-state.json`, booth/operator visual evidence, rollback proof를 포함해야 하며, `sameCaptureFullScreenVisibleMs`를 primary, `firstVisibleMs`와 `replacementMs`를 comparison/diagnostic, `originalVisibleToPresetAppliedVisibleMs`를 legacy comparison value로 읽을 수 있어야 한다.
3. promoted resident lane cutover 이후에도 same-capture guarantee, same-slot truthful replacement, preset fidelity, session isolation, truthful `Preview Waiting`, post-end truth가 유지돼야 한다. queue saturation, warm-state loss, invalid output, wrong-session output, stale bundle, protocol mismatch, rollback trigger가 발생하면 booth는 false-ready, false-complete, cross-session leakage 없이 approved inline truthful fallback으로 내려가야 한다.
4. operator-safe diagnostics와 governance evidence는 현재 lane owner, fallback reason, route stage, warm state, parity 판정, rollback 상태, blocker를 읽을 수 있어야 한다. 하지만 customer-facing copy는 계속 booth-safe plain language만 사용해야 하며, darktable, sidecar, protocol, queue, OpenCL, PIX 같은 내부 용어를 노출하면 안 된다.
5. `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`, `docs/runbooks/booth-hardware-validation-checklist.md`, `docs/release-baseline.md`는 Story 1.13의 fresh rerun 결과를 기준으로 같은 `Go / No-Go`, blocker, owner, evidence path, rerun prerequisite 의미를 유지해야 한다. Story 1.19와 Story 1.27은 supporting evidence owner일 뿐 canonical close owner가 아니다.
6. Story 1.13은 automated regression/build proof와 canonical hardware ledger `Go`가 모두 준비되기 전까지 `done`으로 닫히면 안 된다. 목표 미달 시에는 `No-Go`, rollback reason, rerun prerequisite를 남기고 branch를 `release hold`에 유지해야 한다.

## Tasks / Subtasks

- [ ] Story 1.25 rollback proof와 Story 1.27 canonical local-lane `Go` candidate prerequisite를 확인한다. (AC: 1, 6)
  - [ ] `preview-renderer-policy.json`과 승인된 evidence bundle이 local-lane `Go` candidate acceptance 기준을 충족하는지 확인한다.
  - [ ] repeated resident success-path evidence에서 `laneOwner=local-fullscreen-lane`, `fallbackReason=none`, `routeStage=canary|default`, `warmState=warm-ready|warm-hit`를 확인한다.
  - [ ] inline fallback 제거 없이 one-action rollback 경로가 남아 있는지 확인한다.

- [ ] canonical hardware evidence bundle을 fresh run으로 다시 수집한다. (AC: 2, 3, 4, 6)
  - [ ] Tauri booth 앱과 실카메라 기준으로 `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`를 실행한다.
  - [ ] `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`, route policy snapshot, `bundle.json`, `catalog-state.json`, booth/operator visual evidence, rollback proof를 한 canonical package로 묶는다.
  - [ ] `sameCaptureFullScreenVisibleMs`를 primary, `firstVisibleMs`와 `replacementMs`를 comparison/diagnostic, `originalVisibleToPresetAppliedVisibleMs`를 legacy comparison value로 기록한다.

- [ ] promoted lane 기준 guarded cutover / rollback 안전성을 검증한다. (AC: 1, 3, 4, 6)
  - [ ] route policy 변경이 active session truth를 재해석하지 않는지 확인한다.
  - [ ] warm-state loss, queue saturation, invalid output, wrong-session output, stale bundle, rollback trigger가 모두 inline truthful fallback으로 내려가며 false-ready/false-complete를 만들지 않는지 확인한다.
  - [ ] same-slot replacement, truthful `Preview Waiting`, post-end truth가 promoted lane에서도 유지되는지 확인한다.

- [ ] governance와 release-truth artifact를 fresh rerun 결과로 닫는다. (AC: 4, 5, 6)
  - [ ] hardware ledger에 fresh `Go / No-Go` row, blocker, owner, evidence path, rerun prerequisite를 기록한다.
  - [ ] `docs/runbooks/booth-hardware-validation-checklist.md`, `docs/release-baseline.md`와 ledger 의미가 같은지 확인한다.
  - [ ] customer-facing surface에는 내부 진단어를 남기지 않고, operator-safe evidence에만 기술 상세를 남긴다.

- [ ] release-close 문맥에서 자동 검증을 다시 실행한다. (AC: 3, 4, 6)
  - [ ] dedicated renderer, operator diagnostics, branch rollout, shared contract, governance test를 Story 1.13 cutover 문맥으로 재실행한다.
  - [ ] promoted route 이후에도 same-slot replacement, truthful `Preview Waiting`, branch rollout safety가 깨지지 않는지 확인한다.
  - [ ] automated pass와 hardware `Go`가 동시에 닫히기 전에는 sprint/release 상태를 `hold`로 유지한다.

### Review Findings

- [x] [Review][Patch] Preview promotion evidence bundle accepts missing booth/operator visuals and rollback proof [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:436]
- [x] [Review][Patch] Preview promotion evidence bundle re-copies live route policy and catalog state instead of the capture-time snapshot [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:337]
- [x] [Review][Patch] Hardware validation governance baseline still expects Story 1.13 to remain `review` after review-driven status sync [src/governance/hardware-validation-governance.test.ts:42]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 resident lane activation 자체를 구현하는 단계가 아니다.
- 목적은 Story 1.25 rollback proof와 Story 1.27 local-lane `Go` candidate evidence를 실제 부스 장비에서 최종 `Go / No-Go`로 판정하는 것이다.
- 제품 관점의 핵심은 속도 개선 주장 자체가 아니라, same-capture truth와 rollback safety를 유지한 채 release-safe하게 승격할 수 있느냐다.

### 왜 이 스토리가 다시 정리돼야 하는가

- 현재 forward path는 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25 -> 1.27 -> 1.13`로 정렬되어야 한다. 따라서 Story 1.13은 더 이상 이전 activation gap을 메우는 스토리가 아니라, local-lane `Go` 후보와 rollback proof 이후 rerun되는 final validation / release-close owner로 읽어야 한다.

### 스토리 기반 요구사항

- `epics.md`는 Story 1.27 evidence가 canonical local-lane `Go` candidate와 rollback proof로 받아들여진 뒤에만 Story 1.13 rerun이 final cutover/hardware `Go / No-Go` 판단을 수행해야 한다고 고정한다. [Source: _bmad-output/planning-artifacts/epics.md]
- architecture는 초기 실행 우선순위를 `1.21 metric reset -> 1.22 evidence chain reset -> 1.23 local full-screen lane prototype -> 1.24 hardware canary validation -> 1.25 default decision and rollback gate -> 1.13 release close`로 재정렬했다. [Source: _bmad-output/planning-artifacts/architecture.md#Initial Implementation Priorities]
- PRD는 `first-visible`과 later preset-applied close를 분리 측정하고, capture truth / preview truth / final completion truth를 섞지 말라고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness] [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- UX는 same-capture first-visible이 먼저 보여도 truthful close 전까지 `Preview Waiting`을 유지하고, latest slot은 같은 자리 replacement로 닫혀야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름] [Source: _bmad-output/planning-artifacts/ux-design-specification.md#최신 사진 레일 (Latest Photo Rail)]
- runbook과 release baseline은 Story 1.13이 preview architecture canonical close owner이고, hardware ledger `Go` 전까지 branch가 `release hold`에 머물러야 한다고 고정한다. [Source: docs/runbooks/booth-hardware-validation-checklist.md] [Source: docs/release-baseline.md]

### 현재 워크스페이스와 제품 상태

- `_bmad-output/implementation-artifacts/sprint-status.yaml` 기준으로 Story 1.13은 현재 `backlog`이며, canonical hardware ledger 기준 release state는 `No-Go` / `release hold`다. [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md] [Source: docs/release-baseline.md]
- hardware ledger와 release baseline은 아직 post-reset canonical rerun bundle이 없어서 Story 1.13을 `No-Go` / `release hold`로 남긴다. Story 1.21부터 1.25까지는 문서상 `done`이지만, Story 1.13은 `sameCaptureFullScreenVisibleMs <= 2500ms`, selected-capture evidence continuity, repeated approved-hardware local-lane success-path behavior, one-action rollback을 함께 증명하는 rerun 없이는 닫히지 않는다. [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md] [Source: docs/release-baseline.md]
- 반대로 repo에는 이미 Story 1.19가 고정한 promotion evidence 계약과 도구가 존재한다. `preview-promotion-evidence.jsonl`, route policy snapshot, parity bundle 규칙, hardware scripts, governance/contract tests는 Story 1.13 rerun에서 그대로 재사용해야 한다. [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md] [Source: docs/runbooks/preview-promotion-evidence-package.md]
- dedicated renderer 및 operator diagnostics 테스트는 이미 `laneOwner`, `fallbackReason`, `routeStage`, `warmState`, `originalVisibleToPresetAppliedVisibleMs`를 읽는 회귀를 포함한다. Story 1.13은 새 telemetry family를 만드는 대신 이 계약 위에서 release-close proof를 닫아야 한다. 이 문장은 repo 테스트와 계약 문서를 종합한 해석이다. [Source: src-tauri/tests/dedicated_renderer.rs] [Source: src-tauri/tests/operator_diagnostics.rs] [Source: docs/contracts/local-dedicated-renderer.md]
- `project-context.md`는 발견되지 않았다.

### 이전 스토리 인텔리전스

- Story 1.12는 dual-close topology와 same-slot truthful replacement를 supporting implementation 단계로 `done` 처리했고, guarded cutover와 canonical release-truth `Go / No-Go`는 계속 Story 1.13이 소유한다고 명시했다. [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- Story 1.19는 ETW/WPR/WPA/PIX + parity diff 기반 gate establishment와 evidence package 구조를 고정했지만, canonical close owner를 가져오지 않았다. [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- Story 1.25는 local-lane `Go` 후보와 rollback proof를 닫는 active decision owner이고, Story 1.27은 corrective validation follow-up이다. Story 1.13 implementer가 그 work를 대신 흡수하면 sequencing이 다시 무너진다. [Source: _bmad-output/planning-artifacts/epics.md]

### 구현 가드레일

- Story 1.13에서 activation 설계를 새로 발명하지 말 것. activation / canary / default decision work는 Story 1.25와 Story 1.27이 소유한다.
- host-owned `preview-renderer-policy.json`만 승격/rollback 경계를 제어해야 한다. dev-only 토글이나 ad-hoc override를 release substitute로 쓰면 안 된다.
- active session truth, preset binding, catalog snapshot은 route policy 변경으로 재해석되면 안 된다.
- promoted lane proof가 좋아 보여도 parity drift, fallback 상시 발생, rollback proof 부재, false-ready, false-complete가 하나라도 남으면 `Go`를 주장하지 말 것.
- customer-facing copy에는 darktable, sidecar, protocol, queue, ETW, PIX 같은 내부 용어를 노출하지 말 것.
- fallback path 제거는 `Go` 이후 별도 승인 범위다. 이번 스토리에서는 rollback 가능성과 inline truthful fallback을 유지해야 한다.

### 프로젝트 구조 요구사항

- 우선 확인/수정 후보 경로:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `docs/runbooks/booth-hardware-validation-checklist.md`
  - `docs/runbooks/preview-promotion-evidence-package.md`
  - `docs/release-baseline.md`
  - `src/governance/hardware-validation-governance.test.ts`
  - `src/governance/release-baseline-governance.test.ts`
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/operator_diagnostics.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `tests/hardware-evidence-scripts.test.ts`
  - `scripts/hardware/Start-PreviewPromotionTrace.ps1`
  - `scripts/hardware/Stop-PreviewPromotionTrace.ps1`
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- Story 1.13은 새로운 customer surface를 만드는 범위가 아니라, existing governance/runbook/evidence/runtime boundary를 release-close 수준으로 재검증하는 범위다.

### 테스트 요구사항

- 최소 필수 자동 검증:
  - promoted route에서도 same-slot replacement와 truthful `Preview Waiting`이 유지된다.
  - route policy change와 rollback이 active session truth를 재해석하지 않는다.
  - `laneOwner`, `fallbackReason`, `routeStage`, `warmState`, `originalVisibleToPresetAppliedVisibleMs` 계약이 drift하지 않는다.
  - governance/release baseline test가 Story 1.13 close owner semantics를 계속 잠근다.
  - hardware evidence script와 contract가 canonical bundle 필수 항목을 빠짐없이 요구한다.
- 최소 필수 실장비 검증:
  - `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`
  - same-capture correlation 기준 latency, parity, fallback ratio
  - one-action rollback evidence
  - cross-session leak `0`, false-ready `0`, false-complete `0`

### 최신 기술 / 제품 컨텍스트

- Tauri v2 sidecar 문서는 sidecar 바이너리를 `externalBin`으로 번들링하고, `Command.sidecar(...)` 호출이 그 설정과 일치해야 한다고 설명한다. Story 1.13은 preview truth 경계가 이 host-owned 배포/실행 규칙을 우회하지 않게 유지해야 한다. [Source: https://v2.tauri.app/ko/develop/sidecar/]
- darktable 공식 문서는 `darktable-cli`를 headless export 경로로 설명하고, XMP sidecar를 편집/복구 기준 artifact로 유지한다. Story 1.13은 dedicated renderer를 새 truth engine으로 취급하지 말고, darktable oracle against promoted lane proof를 닫는 문제로 다뤄야 한다. [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/] [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- Playwright Trace Viewer는 저장된 trace를 로컬 또는 `trace.playwright.dev`에서 재생해 볼 수 있다. 현재 Story 1.19 bundle이 canonical replay artifact를 제공한다면, Story 1.13은 그 증거를 reread 가능한 release-close proof로 활용할 수 있다. [Source: https://playwright.dev/docs/trace-viewer]
- Microsoft ETW와 PIX timing capture 문서는 저오버헤드 tracing과 CPU/GPU/file I/O 상관분석을 지원한다. Story 1.13은 새 계측 체계를 발명하기보다 Story 1.19가 고정한 이 evidence stack을 release-close rerun에 그대로 써야 한다. [Source: https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw] [Source: https://devblogs.microsoft.com/pix/timing-captures-new/]

### References

- [Source: _bmad-output/planning-artifacts/epics.md]
- [Source: _bmad-output/planning-artifacts/architecture.md#Initial Implementation Priorities]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#최신 사진 레일 (Latest Photo Rail)]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260413-155159.md]
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260413.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: docs/release-baseline.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: src-tauri/tests/dedicated_renderer.rs]
- [Source: src-tauri/tests/operator_diagnostics.rs]
- [Source: src-tauri/tests/branch_rollout.rs]
- [Source: tests/hardware-evidence-scripts.test.ts]
- [Source: https://v2.tauri.app/ko/develop/sidecar/]
- [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]
- [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- [Source: https://playwright.dev/docs/trace-viewer]
- [Source: https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw]
- [Source: https://devblogs.microsoft.com/pix/timing-captures-new/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-13 16:28:51 +09:00 - `bmad-create-story` workflow 기준으로 config, sprint status, epics, architecture, PRD, UX, Story 1.12, Story 1.19, hardware ledger, release baseline, runbook, preview activation corrective artifacts를 다시 교차 분석했다.
- 2026-04-13 16:28:51 +09:00 - Story 1.13 implementation artifact가 이미 존재하므로, 최신 sequencing과 현재 backlog / release-hold 기준을 반영하도록 갱신했다.
- 2026-04-13 16:28:51 +09:00 - 2026-04-13 승인된 correct-course와 readiness report를 근거로 Story 1.13을 activation 이후 final validation / release-close owner로 재정렬했다.
- 2026-04-13 16:28:51 +09:00 - Tauri sidecar, darktable CLI/XMP, Playwright trace viewer, Microsoft ETW/PIX 공식 문서를 다시 확인해 latest technical guardrail을 스토리 문맥에 연결했다.
- 2026-04-13 16:39:48 +09:00 - Story 1.13 acceptance criteria의 `fallback ratio` 근거가 canonical evidence bundle 계약에 실제로 존재하지 않는 것을 확인하고, bundle schema/assembler/runbook/contracts를 같은 의미로 잠갔다.
- 2026-04-13 16:39:48 +09:00 - `tests/hardware-evidence-scripts.test.ts`, `src/shared-contracts/contracts.test.ts`, `src/governance/hardware-validation-governance.test.ts`, `src/governance/release-baseline-governance.test.ts`를 재실행해 Story 1.13 release-close evidence 회귀를 확인했다.
- 2026-04-13 16:39:48 +09:00 - Story 1.27 evidence acceptance가 아직 닫히지 않아 promoted hardware rerun과 canonical `Go / No-Go` close는 이번 턴에서 계속 차단된 상태로 유지했다.

### Completion Notes List

- Story 1.13을 최신 planning 기준으로 다시 정렬했다.
- 스토리 범위를 activation 구현이 아니라 guarded cutover와 canonical hardware `Go / No-Go` close owner로 좁혔다.
- Story 1.25 rollback proof와 Story 1.27 canonical local-lane `Go` candidate evidence를 명시적 prerequisite로 추가했다.
- Story 1.19 evidence package와 현재 governance/runbook/contract 자산을 그대로 재사용하도록 가드레일을 정리했다.
- 현재 `backlog` 상태와 `release hold` 문맥을 관련 문서와 일치하도록 재정렬했다.
- canonical preview promotion evidence bundle이 `fallbackRatio`를 직접 기록하도록 고정했다.
- same session/preset/version evidence family 안에서 fallback 발생 비율을 계산하도록 bundle assembler를 보강했다.
- Story 1.13 관련 contract/script/governance 회귀 테스트를 다시 통과시켰다.
- Story 1.27 evidence acceptance 미완료로 인해 hardware rerun prerequisite와 `release hold`는 그대로 유지했다.

### File List

- _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md
- docs/contracts/local-dedicated-renderer.md
- docs/runbooks/preview-promotion-evidence-package.md
- scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1
- src/governance/hardware-validation-governance.test.ts
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/hardware-validation.ts
- tests/hardware-evidence-scripts.test.ts
