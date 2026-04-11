# Story 1.13: guarded cutover와 original-visible-to-preset-applied-visible hardware validation gate

Status: review

Architecture Pivot Note: `epics.md` 본문은 아직 1.11~1.13을 개별 스토리로 재생성하지 않았지만, 2026-04-09 승인된 preview architecture decision과 Story 1.11/1.12 handoff에 따라 이번 스토리는 `local dedicated renderer + different close topology`의 세 번째 단계인 guarded cutover, 실장비 latency close 검증, release-truth `Go` 판단 범위를 복원한다.

### Validation Gate Reference

- Supporting evidence family:
  - `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`
  - capture-correlated seam package (`request-capture -> file-arrived -> fast-preview-visible -> capture_preview_ready -> recent-session-visible`)
  - `original visible -> preset-applied visible` p50/p95 hardware latency package
  - lane owner / fallback reason / canary-vs-default cutover proof
  - hardware ledger canonical `Go / No-Go` row
- Current hardware gate: `No-Go`
- Close policy:
  - automated proof만으로는 release-truth `Go`를 주장하지 않는다.
  - 이번 스토리는 dedicated renderer를 무조건 기본값으로 켜는 단계가 아니라, guarded cutover와 rollback 가능한 운영 경계 위에서 실장비 증거를 닫는 단계다.
  - latency 목표 미달, fallback 상시 발생, same-capture mismatch, false-ready/false-complete가 남으면 `No-Go`로 유지하고 기존 approved path로 즉시 rollback 가능해야 한다.

## Story

As a owner / brand operator,
I want local dedicated renderer path를 guarded cutover하고 실장비 evidence로 최종 검증하고 싶다,
so that booth가 same-capture truthful preview를 목표 latency 안에서 release-safe하게 제공한다고 자신 있게 승격할 수 있다.

## Acceptance Criteria

1. dedicated renderer truth lane의 승격은 `shadow -> limited canary -> default` 또는 동등한 guarded rollout 순서를 따라야 한다. cutover는 host-owned 설정 또는 승인된 rollout boundary로만 제어되어야 하며, dev-only ad-hoc 토글에 의존하면 안 된다. active session은 forced update 없이 기존 truth를 유지해야 하고, `No-Go` 시 한 액션 rollback 경로가 남아 있어야 한다.
2. 승인된 Windows booth hardware에서 동일 capture 단위 correlation으로 `request-capture`, `file-arrived`, `fast-preview-visible`, `capture_preview_ready`, `recent-session-visible`, lane owner, fallback reason, `first-visible-ms`, `replacement-ms`, `originalVisibleToPresetAppliedVisibleMs`를 다시 읽을 수 있는 canonical evidence package가 수집돼야 한다. 이 패키지는 `original visible -> preset-applied visible <= 2.5s` 목표와 warm p50/p95, fallback 비율, mismatch `0` 여부를 판단할 수 있어야 한다.
3. cutover된 booth runtime은 same-capture guarantee, preset fidelity, session isolation, truthful `Preview Waiting`, same-slot replacement를 유지해야 한다. dedicated renderer lane이 실패하거나 queue saturation, warm-state loss, protocol mismatch, invalid output, wrong-session output, stale bundle이 발생하면 booth는 false-ready, false-complete, cross-session leakage 없이 approved inline truthful fallback path로 내려가야 한다.
4. operator-safe diagnostics와 governance evidence는 현재 lane owner, fallback reason, canary/default 상태, hardware capability, blocker 여부를 읽을 수 있어야 한다. 하지만 customer-facing copy는 계속 booth-safe plain language만 사용해야 하며, darktable, sidecar, protocol, queue, OpenCL 같은 내부 용어를 노출하면 안 된다.
5. `docs/runbooks/booth-hardware-validation-checklist.md`, `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`, `docs/release-baseline.md`는 Story 1.13이 preview architecture track의 canonical `Go / No-Go` close owner라는 사실을 반영해야 한다. evidence path, blocker, owner, rerun prerequisite, promotion hold 상태가 문서 간에 같은 의미로 정렬돼야 한다.
6. Story 1.13은 automated regression/build proof와 canonical hardware ledger `Go`가 모두 준비되기 전까지 `done`으로 닫히면 안 된다. 목표 미달 시에는 `No-Go`와 rollback 이유를 남기고, supporting story(1.11/1.12) evidence를 release close와 혼동하지 않도록 유지해야 한다.

## Tasks / Subtasks

- [ ] guarded cutover control과 rollback boundary를 고정한다. (AC: 1, 3, 6)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`, `src-tauri/src/render/mod.rs`에서 dedicated renderer enablement를 승인된 runtime/rollout 경계로만 승격하고, dev-only spawn opt-in이 release 경로를 대신하지 못하게 정리한다.
  - [ ] `src-tauri/src/branch_config/mod.rs`, `src-tauri/src/commands/branch_rollout_commands.rs`, `src-tauri/tests/branch_rollout.rs` 또는 동등 경로에서 cutover/rollback이 active session을 강제 재해석하지 않는다는 규칙을 잠근다.
  - [ ] fallback path 제거는 `Go` 이후 별도 승인 범위로 남기고, 이번 스토리에서는 one-action rollback 가능성을 유지한다.

- [ ] hardware evidence와 seam 계측 패키지를 완성한다. (AC: 2, 3, 4)
  - [x] `capture_preview_transition_summary`와 동등 evidence가 `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`, lane owner, fallback reason을 stable하게 남기도록 점검한다.
  - [x] canary/default lane 구분, fallback rate, queue saturation, renderer restart, invalid output, wrong-session rejection을 한 회차 evidence package에서 읽을 수 있게 정리한다.
  - [ ] 필요하면 `tests/hardware/dual-close-topology/*` 또는 동등한 evidence path를 추가해 실장비 패키지 구조를 고정한다.

- [ ] approved booth hardware validation matrix를 실제로 수행한다. (AC: 2, 3, 5, 6)
  - [ ] Tauri booth 앱 기준으로 `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`를 실행한다.
  - [ ] `session.json`, `timing-events.log`, preview/final output, `bundle.json`, `catalog-state.json`, operator evidence, booth 화면 증거를 canonical package로 묶는다.
  - [ ] `original visible -> preset-applied visible` 목표 미달, fallback 상시 발생, mismatch 발생 시 즉시 `No-Go`로 기록하고 rollback 근거를 남긴다.

- [x] governance / runbook / ledger를 Story 1.13 ownership에 맞게 정렬한다. (AC: 4, 5, 6)
  - [x] `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에 Story 1.13 canonical close row를 추가하거나 동등 수준으로 정리한다.
  - [x] `docs/runbooks/booth-hardware-validation-checklist.md`와 `docs/runbooks/booth-hardware-validation-architecture-research.md`에 guarded cutover와 preview architecture close owner 기준을 반영한다.
  - [x] `docs/release-baseline.md`와 `src/governance/hardware-validation-governance.test.ts`, `src/governance/release-baseline-governance.test.ts`에서 preview architecture `Go / No-Go` hold 조건을 검증한다.

- [ ] automated regression과 packaging proof를 release close 기준으로 다시 실행한다. (AC: 1, 3, 4, 6)
  - [x] `src-tauri/tests/dedicated_renderer.rs`, `src-tauri/tests/operator_diagnostics.rs`, shared contract/UI test, `pnpm build:desktop` 또는 동등 proof path를 Story 1.13 cutover 문맥으로 재실행한다.
  - [ ] canary/default 전환 후에도 same-slot replacement, `Preview Waiting`, operator-safe diagnostics, branch rollout safety가 깨지지 않는지 검증한다.
  - [x] automated pass와 hardware pass가 동시에 닫히기 전에는 sprint/release 상태를 `hold`로 유지한다.

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 `1.11 protocol baseline -> 1.12 dual-close semantics` 다음 단계로, dedicated renderer를 실제 booth preview architecture의 release-truth candidate로 검증하는 단계다.
- 목적은 “코드상 가능하다”를 넘어서, approved booth hardware에서 same-capture truthful close가 목표 latency와 guarded rollout 규칙을 동시에 만족하는지 증명하는 것이다.
- 고객 경험 약속은 유지한다. 먼저 같은 촬영이 보일 수 있어도 truthful close 전까지는 `Preview Waiting`이고, 실패 시에는 booth-safe fallback만 허용된다.

### 왜 이 스토리가 새로 필요해졌는가

- Story 1.11은 dedicated renderer sidecar boundary와 capture-bound protocol을 닫았지만, hardware ledger `Go`는 후속 story가 닫아야 한다고 명시했다. [Source: _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md]
- Story 1.12는 dual-close topology, same-slot replacement, summary metric을 정착시켰지만, 실제 booth-wide cutover와 release `Go`는 Story 1.13 소유라고 남겼다. [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- hardware ledger도 supporting note에서 “guarded cutover 최종 hardware gate는 Story 1.13이 이어받는다”고 명시한다. [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- architecture와 research는 preview architecture 실행 우선순위를 `dedicated renderer ownership -> cutover validation -> release proof`로 정리한다. [Source: _bmad-output/planning-artifacts/architecture.md#Initial Implementation Priorities] [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Implementation Roadmap]

### 스토리 기반 요구사항

- PRD는 first-visible latency, `original visible -> preset-applied visible` close latency, preset-applied readiness latency를 분리 계측해야 한다고 고정한다. [Source: _bmad-output/planning-artifacts/prd.md#KPI Table] [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- PRD는 capture truth, preview truth, final completion truth가 분리된 진실값이어야 하며, booth는 preview/final이 준비되기 전 완료를 암시하면 안 된다고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate] [Source: _bmad-output/planning-artifacts/prd.md#Booth-Safe Runtime Boundary]
- Architecture는 preview pipeline을 `first-visible lane`과 `truth lane`으로 분리하고, host-owned local dedicated renderer lane이 preset-applied close owner라고 명시한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture] [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- UX는 same-capture first-visible이 먼저 보여도 `Preview Waiting`을 유지하고, latest slot은 같은 자리 replacement로 닫혀야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름] [Source: _bmad-output/planning-artifacts/ux-design-specification.md#최신 사진 레일 (Latest Photo Rail)]
- runbook과 release baseline은 hardware proof와 automated proof가 별도 gate이며, canonical close record는 hardware ledger가 소유한다고 고정한다. [Source: docs/runbooks/booth-hardware-validation-checklist.md] [Source: docs/release-baseline.md#Release Truth Gates]

### 현재 워크스페이스 상태

- `src-tauri/src/render/dedicated_renderer.rs`는 dedicated renderer spawn을 여전히 explicit opt-in 환경 변수와 runtime handle 존재 여부에 묶고 있다. 즉 release cutover 기본 경로는 아직 별도 운영 경계로 닫히지 않았다.
- 같은 파일과 `src-tauri/tests/dedicated_renderer.rs`는 accepted dedicated renderer result가 inline overwrite 없이 truthful close를 닫는 경로와 queue saturation fallback을 이미 테스트한다. 즉 story의 핵심 공백은 “기본 기능 부재”보다 “guarded rollout과 canonical hardware close”에 가깝다.
- `capture_preview_transition_summary` metric 회귀는 Story 1.12에서 다시 잠갔지만, hardware ledger canonical row와 runbook scope에는 Story 1.13 close owner가 아직 직접 등록되지 않았다.
- `docs/runbooks/booth-hardware-validation-checklist.md`의 canonical release-gated story 목록에는 Story 1.13이 아직 포함되지 않는다. 반면 ledger supporting note는 Story 1.13 ownership을 이미 암시한다.
- `docs/release-baseline.md`는 automated proof와 hardware proof 분리를 고정하지만, preview architecture cutover를 Story 1.13 owner로 직접 연결한 문구는 아직 없다.
- 현재 worktree는 render, operator diagnostics, session selector, hardware ledger 등 여러 스토리의 변경이 섞여 있다. 1.13 implementer는 unrelated dirty changes를 되돌리지 말고, cutover/governance/hardware evidence 범위로 좁혀 작업해야 한다.

### 이전 스토리 인텔리전스

- Story 1.11은 dedicated renderer sidecar boundary를 공식 allowlist와 protocol contract로 고정했고, hardware close는 후속 story 소유라고 남겼다. [Source: _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md]
- Story 1.12는 same-slot truthful replacement와 summary metric을 정착시키고 supporting hardware run을 확인했지만, release-truth `Go` row는 만들지 않았다. [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- hardware ledger에는 1.10과 1.12 supporting proof가 이미 있어, 1.13은 “새로운 제품 약속을 발명”하기보다 “existing supporting evidence를 canonical cutover close로 승격할 수 있는지 판정”하는 단계로 보는 편이 맞다. [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]

### 구현 가드레일

- `big bang` enablement를 하지 말 것. research가 권장한 `shadow -> canary -> default` guarded adoption을 유지할 것.
- active session 중 forced update, preset rebinding, truth owner 재해석을 허용하지 말 것.
- latency 목표가 좋아 보여도 same-capture mismatch, fallback 상시 발생, false-ready/false-complete가 하나라도 남으면 `Go`를 주장하지 말 것.
- booth hardware evidence는 브라우저 preview가 아니라 Tauri 앱과 실제 카메라로만 수집할 것.
- customer-facing copy는 계속 plain language만 사용하고, operator/ledger evidence에만 기술 상세를 남길 것.
- Story 1.13에서도 fallback path를 삭제하지 말 것. fallback 제거는 `Go` 이후 별도 승인 범위다.

### 아키텍처 준수사항

- Tauri v2 공식 sidecar 문서는 2026-04-11 기준 `externalBin` 번들링과 `app.shell().sidecar(name)` 기반 실행 경로를 기준으로 설명한다. 이번 스토리는 dedicated renderer enablement가 그 승인 경계를 우회하지 않게 유지해야 한다. [Source: https://v2.tauri.app/ko/develop/sidecar/]
- darktable 공식 문서는 `darktable-cli`가 headless export 경로이고 XMP sidecar가 편집 이력 artifact라는 점을 유지한다. dedicated renderer cutover는 새 truth engine 발명이 아니라 이 경로를 faster local topology로 운영하는 문제다. [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/] [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- Playwright 공식 trace viewer 문서는 저장된 trace를 재생 가능한 증거로 남길 수 있다고 설명한다. 현재 replay proof가 필수는 아니더라도, close regression을 읽는 evidence 형식으로는 계속 유효하다. [Source: https://playwright.dev/docs/trace-viewer]
- research는 Strangler Fig 방식의 점진 치환과 hardware-in-loop 검증을 권장한다. 이 문장은 공식 Strangler Fig 패턴과 research 결론을 현재 repo에 적용한 해석이다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technology Adoption Strategies]

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/branch_config/mod.rs`
  - `src-tauri/src/commands/branch_rollout_commands.rs`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/operator_diagnostics.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `src/governance/hardware-validation-governance.test.ts`
  - `src/governance/release-baseline-governance.test.ts`
  - `docs/runbooks/booth-hardware-validation-checklist.md`
  - `docs/runbooks/booth-hardware-validation-architecture-research.md`
  - `docs/release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- 새로 추가될 가능성이 큰 경로:
  - `tests/hardware/dual-close-topology/*`
  - `_bmad-output/implementation-artifacts/hardware-proof-1-13/*`
  - `docs/runbooks/cutover-evidence-template.md` 또는 동등 보조 문서
- Story 1.13은 새로운 customer UI surface를 만들기보다, existing booth flow와 governance/runbook/evidence 경계를 닫는 편이 우선이다.

### 테스트 요구사항

- 최소 필수 자동 검증:
  - dedicated renderer accepted path가 truthful close를 닫고 same-slot replacement를 보존한다.
  - queue saturation / protocol mismatch / invalid output / wrong-session output이 inline truthful fallback으로 내려가며 false-ready를 만들지 않는다.
  - operator diagnostics는 latest invalid session fallback을 허용하지 않고 blocker를 정확히 유지한다.
  - branch rollout / rollback이 active session을 강제 변경하지 않는다.
  - release baseline governance와 hardware validation governance가 Story 1.13 gate ownership을 반영한다.
- 최소 필수 실장비 검증:
  - `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`
  - `original visible -> preset-applied visible` warm p50 / warm p95
  - mismatch `0`, cross-session leak `0`, false-ready `0`, false-complete `0`
  - canary/default enablement과 rollback 결과 비교
- 권장 추가 검증:
  - replay 가능한 UI evidence 또는 동등한 operator-safe replay package
  - GPU/OpenCL capability 차이에 따른 fallback rate 비교

### 최신 기술 / 제품 컨텍스트

- 2026-04-09 research는 `local dedicated renderer + different close topology`를 즉시 시작할 next structure로 선택했고, 목표 미달 시에만 `edge appliance`를 2차안으로 검토하라고 정리했다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Final Recommendation] [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technical Research Conclusion]
- 같은 research는 가장 안전한 adoption strategy를 `shadow lane -> limited booth canary -> default 승격` 순서로 본다. 이 문장은 research 결론을 Story 1.13 cutover 범위에 직접 적용한 해석이다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technology Adoption Strategies]
- 공식 darktable 문서와 현재 contract는 pinned `5.4.1` runtime을 전제로 한다. Story 1.13은 version pin drift 없이 hardware package를 닫아야 한다. [Source: docs/contracts/render-worker.md] [Source: docs/runbooks/booth-hardware-validation-checklist.md]

### Git 인텔리전스

- 최근 5개 commit title:
  - `4611eb5 feat: add local renderer contracts and release baseline`
  - `8c30be7 Improve focus retry guidance`
  - `2c89c40 Finalize thumbnail latency worker updates and docs`
  - `9c56c37 Add session seam logging for thumbnail latency reduction`
  - `b24cfc4 Reduce recent-session preview latency and capture wait blocking`
- 최근 흐름은 local renderer contract 정리, seam logging 강화, first-visible latency correction으로 이어진다.
- 따라서 1.13은 별도 새 방향을 만드는 것보다, 이미 형성된 renderer contract와 seam evidence를 guarded release gate로 닫는 것이 자연스럽다.

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Initial Implementation Priorities]
- [Source: _bmad-output/planning-artifacts/prd.md#KPI Table]
- [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#최신 사진 레일 (Latest Photo Rail)]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Implementation Roadmap]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technology Adoption Strategies]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Final Recommendation]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technical Research Conclusion]
- [Source: _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md]
- [Source: _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: docs/release-baseline.md]
- [Source: https://v2.tauri.app/ko/develop/sidecar/]
- [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]
- [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- [Source: https://playwright.dev/docs/trace-viewer]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-11 11:09:22 +09:00 - `bmad-create-story` workflow 기준으로 config, sprint-status, epics, PRD, architecture, UX, Story 1.11/1.12, hardware ledger, runbook, release baseline, current dedicated renderer / governance / rollout 관련 경로를 교차 분석했다.
- 2026-04-11 11:09:22 +09:00 - `epics.md`에 1.13 본문이 아직 직접 생성되지 않아, approved architecture pivot 우선순위(`dedicated renderer protocol -> close topology 분리 -> hardware validation / cutover`)와 Story 1.11/1.12 handoff를 근거로 스토리 제목과 범위를 복원했다.
- 2026-04-11 11:09:22 +09:00 - 현재 repo가 dedicated renderer accepted/fallback test와 supporting hardware proof는 보유하지만 canonical release `Go` owner와 guarded cutover governance는 아직 비어 있다는 점을 Story 1.13 범위에 반영했다.
- 2026-04-11 11:09:22 +09:00 - Tauri sidecar, darktable CLI/XMP, Playwright trace viewer 공식 문서를 다시 확인해 Story 1.13의 최신 운영 가드레일에 반영했다.
- 2026-04-11 11:28:23 +09:00 - `preview-renderer-policy.json` 기반 shadow/canary/default route resolution을 dedicated renderer path에 연결하고, route stage 및 fallback reason이 `capture_preview_transition_summary`와 dedicated renderer integration test evidence에 남도록 정리했다.
- 2026-04-11 11:33:04 +09:00 - `pnpm test:run src/governance/hardware-validation-governance.test.ts src/governance/release-baseline-governance.test.ts`를 다시 실행해 Story 1.13 ledger/runbook/release hold 정렬이 통과하는지 확인했다.
- 2026-04-11 11:34:46 +09:00 - `cargo test --test dedicated_renderer`, `cargo test --test branch_rollout`를 Story 1.13 cutover 문맥으로 재실행했고, queue saturation/stale result test를 route policy 기준으로 갱신한 뒤 전체 통과를 확인했다.

### Completion Notes List

- host-owned `preview-renderer-policy.json`이 dedicated renderer 승격 경계를 소유하도록 정리했고, dev-only opt-in이 release 경로를 대신하지 못하도록 shadow 기본값을 잠갔다.
- Story 1.13 canonical close owner를 runbook, release baseline, hardware ledger, sprint status에 반영했고 현재 hardware gate를 `No-Go`로 기록했다.
- 실장비 근거는 여전히 shadow-only 상태다. `session_000000000018a5007b5fecf020`에서 `laneOwner=inline-truthful-fallback`, `fallbackReason=shadow-submission-only`, `originalVisibleToPresetAppliedVisibleMs=none`이 관찰돼 story status를 `review`로 유지한다.
- 자동 검증은 통과했다: `pnpm test:run src/governance/hardware-validation-governance.test.ts src/governance/release-baseline-governance.test.ts`, `cargo test --test dedicated_renderer`, `cargo test --test branch_rollout`.

### File List

- _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/release-baseline.md
- release-baseline.md
- docs/runbooks/booth-hardware-validation-checklist.md
- docs/runbooks/booth-hardware-validation-architecture-research.md
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/dedicated_renderer.rs
- src/governance/hardware-validation-governance.test.ts

### Change Log

- 2026-04-11 - host-owned preview renderer route policy를 dedicated renderer truth lane에 연결하고, Story 1.13 `No-Go` ledger/runbook/release hold 기준을 정렬했다.

### Review Findings

- [x] [Review][Patch] Story 1.13 governance proof still expects Story 4.2 to remain `review` / `No-Go`, so the current automated gate fails immediately [src/governance/hardware-validation-governance.test.ts:34]
- [x] [Review][Patch] Story 1.13 is the canonical preview close owner, but the impacted-story governance guard still omits its own story document, leaving `Status` / hardware-gate drift untested [src/governance/hardware-validation-governance.test.ts:12]
- [x] [Review][Patch] Same-preset reselection can reinterpret an active session's rollout lane [src-tauri/src/session/session_repository.rs:144]
- [x] [Review][Patch] Partial preview-transition logs can mix stale and current rollout diagnostics [src-tauri/src/diagnostics/mod.rs:524]
- [x] [Review][Patch] Invalid preview route policy is silently recorded as intentional shadow mode [src-tauri/src/render/dedicated_renderer.rs:1000]
