# Story 3.1: Readiness States and Capture Gating

Status: review

Story Key: `3-1-readiness-states-and-capture-gating`

## Summary

Formalize Epic 3's approved readiness model on the actual repo seam that already exists: typed camera readiness contracts, host normalization, session-domain state, and booth customer screens. The current implementation collapses all blocked capture states into a `checking-camera`/`phone-required` split and hard-disables the capture action in the capture surface; this story should replace that interim behavior with approved `preparing`, `waiting`, `ready`, and `phone-required` states while keeping capture blocked whenever the booth is not ready and without leaking diagnostics to the customer.

## Story

As a booth customer,
I want to see clear readiness guidance and only capture when the booth is ready,
so that I know when it's safe to shoot.

## Acceptance Criteria

1. Given the booth is preparing, waiting, or phone-required, when the capture surface is shown, then the UI displays a customer-safe readiness state message and capture is blocked until the state becomes ready.
2. Given the booth becomes ready, when the readiness state transitions, then the capture action is enabled and the UI reflects the ready state without technical diagnostics.

## Tasks / Subtasks

- [x] Replace the interim three-state readiness contract with the approved four-state capture-gating model. (AC: 1, 2)
  - [x] Update the TypeScript and Rust readiness DTOs so the host-normalized booth state can express `preparing`, `waiting`, `ready`, and `phone-required` directly while preserving the typed error envelope and `captureEnabled`.
  - [x] Map retryable degraded or reconnecting helper states to `waiting`, non-recoverable or threshold-exceeded states to `phone-required`, and only fully approved states to `ready`.
  - [x] Keep the lower-level sidecar/device snapshot separate from the customer-facing readiness contract so React never infers booth truth from raw helper states.
- [x] Apply readiness gating across the customer flow without discarding session context or the chosen preset. (AC: 1, 2)
  - [x] Update `SessionFlowProvider`, `sessionReducer`, and related selectors so readiness changes can move the booth between blocked and ready states while preserving `activeSession` and the selected preset.
  - [x] Prevent temporary readiness regressions from forcing the customer back through preset selection or clearing the active preset unless the session itself becomes invalid.
  - [x] Keep capture blocked whenever the host-normalized readiness state is not `ready`.
- [x] Align booth UI copy and action affordances to the approved readiness states. (AC: 1, 2)
  - [x] Add customer-safe `waiting` copy distinct from `preparing` and `phone-required`, staying within the copy budget and without internal diagnostics.
  - [x] Show the branch phone number only for `phone-required`.
  - [x] Replace the hard-coded capture-button disablement in the capture surface with readiness-driven gating; do not fabricate a successful capture result in this story.
- [x] Keep Story 3.1 scoped to readiness and gating, not capture persistence. (AC: 1, 2)
  - [x] Do not implement fake latest-photo generation or session persistence here; Story 3.2 owns capture completion and latest-photo confirmation.
  - [x] If a typed request handoff is needed for button wiring, keep it boundary-safe and explicit, but leave persisted capture outcomes to Story 3.2.
- [x] Add regression coverage for host mapping, state transitions, and customer-safe copy. (AC: 1, 2)
  - [x] Update contract tests for the approved readiness-state enum and typed error reuse.
  - [x] Add frontend integration tests for `preparing -> waiting -> ready`, `ready -> waiting -> ready` without losing the active preset, and `phone-required` escalation with branch phone display.
  - [x] Add Rust tests for host mapping from sidecar status/error conditions into approved readiness states and capture gating.

### Review Follow-ups (AI)

- [x] [AI-Review][High] Add an explicit unsubscribe/cancellation path for readiness watch so React cleanup stops the native polling thread instead of only clearing `onmessage`. [src/capture-adapter/host/cameraAdapter.ts:302, src-tauri/src/commands/capture_commands.rs:62]
- [x] [AI-Review][High] Add an explicit unsubscribe/cancellation path for capture-confidence watch so React cleanup stops the native polling thread instead of only clearing `onmessage`. [src/capture-adapter/host/cameraAdapter.ts:400, src-tauri/src/commands/capture_commands.rs:91]
- [x] [AI-Review][Medium] Replace snapshot re-polling with a real readiness stream or equivalent bounded host watcher; the current implementation respawns the mock sidecar process for every 750 ms poll and only samples terminal snapshot results. [src-tauri/src/commands/capture_commands.rs:66, src-tauri/src/capture/camera_host.rs:41, src-tauri/src/capture/sidecar_client.rs:199]
- [x] [AI-Review][Medium] Extend the non-Tauri fallback readiness path to represent the full approved blocked-state model, including `phone-required`, so browser/dev flows can exercise the Story 3.1 escalation UX. [src/capture-adapter/host/cameraAdapter.ts:146, src/capture-adapter/host/cameraAdapter.ts:196]
- [x] [AI-Review][Medium] Reconcile the Story 3.1 File List and review record with the actual readiness seam files and a git-trackable source diff. [C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/3-1-readiness-states-and-capture-gating.md:117, C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/3-1-readiness-states-and-capture-gating.md:300]

## Dev Notes

### Developer Context

- The approved Epic 3 baseline requires four customer-facing readiness outcomes for capture: `preparing`, `waiting`, `ready`, and `phone-required`. Current repo code only exposes `preparing`, `ready`, and `phone-required`, with `checking-camera` acting as a catch-all blocked state. That mismatch is the core drift this story corrects.
- The repo already has the right implementation seam for this work:
  - `src/shared-contracts/dto/cameraStatus.ts` and `src-tauri/src/contracts/dto.rs` define the typed readiness DTOs.
  - `src-tauri/src/capture/camera_host.rs` normalizes helper readiness into customer-facing status.
  - `src/session-domain/state/customerPreparationState.ts` and `src/session-domain/state/SessionFlowProvider.tsx` translate host status into customer state and gate flow transitions.
  - `src/customer-flow/screens/PreparationScreen.tsx`, `src/customer-flow/screens/CustomerFlowScreen.tsx`, and `src/customer-flow/screens/CaptureScreen.tsx` render the current booth readiness and capture entry surfaces.
- Two implementation drifts are already visible in the current repo and should be treated as story bugs, not product rules:
  - `CaptureScreen` is rendered with `captureActionDisabled` hard-coded from `CustomerFlowScreen`, so the capture CTA never becomes readiness-driven.
  - `sessionReducer` currently clears preset-selection/capture state when readiness leaves `ready`, which would incorrectly force the customer back through preset choice during a temporary blocked state.
- The March 13 implementation-readiness refresh confirms the latest planning corrections preserved Story 3.1's scope and resolved the earlier epic-sequencing concerns that were broader than this story. The remaining plan-level caution is sprint-tracker drift, not a Story 3.1 scope change.
- This repository already contains the contract, host, and customer-flow scaffolding needed to make Story 3.1 independently actionable. Reuse and tighten those seams rather than introducing new readiness infrastructure.
- Scope boundary:
  - In scope: normalized readiness-state modeling, customer-safe blocked-state copy, and capture gating behavior.
  - Out of scope: real capture persistence, latest-photo completion/confirmation, review/delete behavior, timing warning alerts, and operator recovery tooling beyond the existing phone-escalation path.

### Technical Requirements

- The customer-facing readiness contract must align to the approved Epic 3 language. Replace the current interim customer connection model (`preparing` / `checking-camera` / `ready`) with an explicit approved-state model that can represent:
  - `preparing`: booth/session initialization before the camera is meaningfully available
  - `waiting`: temporary blocked state where the customer should wait, not call
  - `ready`: capture is allowed
  - `phone-required`: bounded escalation state where the customer should call the branch
- Keep the authoritative readiness decision host-owned. React should consume one normalized readiness object from the host/adapter layer rather than deriving `waiting` vs `phone-required` from raw helper/device details in UI code.
- Preserve the typed error envelope, but do not force UI code to reverse-engineer customer booth state from envelope details alone. The host-normalized readiness payload should already make the approved customer state clear.
- `captureEnabled` remains the single implementation gate for the capture CTA. It must be `true` only for `ready` and `false` for `preparing`, `waiting`, and `phone-required`.
- Retryable reconnect/degraded conditions should resolve to `waiting`; non-retryable camera-unavailable conditions and approved threshold escalations should resolve to `phone-required`.
- Temporary readiness regressions after preset selection or capture entry must preserve:
  - `activeSession`
  - the active preset selection
  - session-scoped timing context already attached to the active session
  They may hide or disable capture affordances, but they should not push the customer back to the check-in flow or force preset re-selection.
- Replace hard-coded capture disablement with readiness-driven behavior. If the actual capture command is still a later-story boundary, the CTA may remain functionally stubbed behind its existing typed seam, but the UI must not fake a completed capture in Story 3.1.
- If browser fallback readiness remains in use for development/tests, update it so non-Tauri flows can represent the approved blocked/ready progression without bypassing the new contract shape.

### Architecture Compliance

- Preserve the architecture rule that the Rust host normalizes camera/helper truth once and React consumes that normalized result through typed adapters/services. Do not move sidecar/device interpretation into `PreparationScreen`, `CaptureScreen`, or selectors.
- Keep all readiness reads on the existing `cameraAdapter` / Tauri command + channel seam. No direct `invoke` calls from React components and no direct session-file reads from the frontend.
- Customer-visible messaging must remain customer-safe and copy-budget compliant:
  - one primary instruction sentence
  - one supporting sentence
  - one primary action label
  - no diagnostic filenames, SDK references, filesystem paths, or internal recovery jargon
- Preserve the current domain-first structure:
  - customer flow UI under `src/customer-flow/*`
  - session/readiness orchestration under `src/session-domain/*`
  - host bridge logic under `src/capture-adapter/*`
  - native normalization under `src-tauri/src/capture/*` and `src-tauri/src/commands/*`
- Do not broaden this story into operator-console capabilities, export/handoff states, or timing-warning flows. Those belong to later epic work even if some timing-based escalation logic is reused here.
- Do not let temporary readiness loss become a cross-session state bug. Session-scoped capture confidence, preset labels, and active-session identity must stay isolated to the current session and must not leak to another session while the booth is blocked.

### Library / Framework Requirements

- Use the repo's current checked-in baselines from `package.json` and `src-tauri/Cargo.toml`:
  - React `19.2.x`
  - React DOM `19.2.x`
  - React Router `7.9.4`
  - Zod `4.3.x`
  - `@tauri-apps/api` / CLI `2.10.x`
  - Rust `tauri` `2.10.x`
  - Rust `chrono 0.4.42` and `rusqlite 0.38.0`
- Keep the existing React 19 effect/event pattern already used in `SessionFlowProvider.tsx`. `useEffectEvent` remains the correct fit for readiness watchers and transition side effects; do not replace it with ad hoc mutable globals or event-bus logic.
- Keep Tauri v2 commands/channels as the host boundary for readiness snapshots and watch streams. That remains consistent with current official Tauri v2 guidance for calling Rust and streaming updates to the frontend.
- Continue using Zod 4 as the TypeScript contract gate for readiness DTOs and command payloads/results. Do not introduce a second hand-rolled runtime validator for the same readiness shapes.
- React Router should remain limited to top-level surfaces. Do not create new route states such as `/waiting` or `/phone-required`; readiness remains workflow state inside the customer-flow/session-domain seam.
- No dependency upgrade is required to complete Story 3.1. Implement against the checked-in stack unless a security or compatibility issue is directly blocking the readiness contract work.

### File Structure Requirements

- Primary TypeScript contract/state/UI seam:
  - `src/shared-contracts/dto/cameraStatus.ts`
  - `src/shared-contracts/dto/cameraErrorContract.ts`
  - `src/shared-contracts/dto/errorEnvelope.ts`
  - `src/shared-contracts/dto/cameraContract.ts`
  - `src/capture-adapter/host/cameraAdapter.ts`
  - `src/session-domain/state/customerPreparationState.ts`
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/customer-flow/copy/preparationScreenCopy.ts`
  - `src/customer-flow/selectors/customerCameraStatusCopy.ts`
  - `src/customer-flow/screens/PreparationScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
- Native host normalization seam:
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/capture/camera_host.rs`
  - `src-tauri/src/commands/capture_commands.rs`
- Likely supporting test surfaces:
  - `tests/contract/cameraReadinessStatus.test.ts`
  - `tests/contract/cameraContract.test.ts`
  - `tests/integration/customerReadinessFlow.test.tsx`
  - `tests/integration/presetSelectionFlow.test.tsx`
  - `src/customer-flow/selectors/customerCameraStatusCopy.spec.ts`
  - `src/customer-flow/screens/PreparationScreen.spec.tsx`
  - `src/capture-adapter/host/cameraAdapter.spec.ts`
  - `src/session-domain/state/sessionReducer.review.spec.ts`
  - `src-tauri/tests/camera_contract.rs`
  - `src-tauri/tests/mock_sidecar_integration.rs`
- Detected repo variance to account for:
  - `CaptureScreen` is already the intended active capture surface, but `CustomerFlowScreen` currently hard-disables its primary action.
  - `PreparationScreen` currently handles the only blocked-state presentation; decide whether to reuse it for `waiting` or introduce a tightly scoped blocked-state variation without moving state logic into presentation components.
  - Existing readiness tests still assume `checking-camera` as the blocked-state language. Update them to the approved terminology instead of layering aliases indefinitely.

### Testing Requirements

- Keep test coverage layered by boundary:
  - contract tests for the shared readiness DTO and error-envelope compatibility
  - frontend unit/integration tests for readiness translation and customer-flow gating
  - Rust tests for host normalization of sidecar output
- Minimum TypeScript contract coverage:
  - update `tests/contract/cameraReadinessStatus.test.ts` so the approved readiness-state contract is asserted directly
  - update `tests/contract/cameraContract.test.ts` only if the normalized readiness payload/result shape changes
  - keep typed error-envelope reuse intact; do not create a second readiness-only failure shape
- Minimum frontend behavior coverage:
  - update `src/customer-flow/selectors/customerCameraStatusCopy.spec.ts` for distinct `preparing`, `waiting`, `ready`, and `phone-required` messaging
  - update `src/customer-flow/screens/PreparationScreen.spec.tsx` to verify blocked-state copy, phone-number visibility, and capture CTA behavior
  - update `tests/integration/customerReadinessFlow.test.tsx` to cover:
    - initial preparation
    - temporary waiting without escalation
    - phone-required escalation
    - no diagnostic leakage in customer copy
  - update `tests/integration/presetSelectionFlow.test.tsx` or adjacent flow tests so a readiness regression after preset selection blocks capture without discarding the active preset
  - update `src/session-domain/state/sessionReducer.review.spec.ts` if reducer behavior changes to preserve preset/session context during blocked states
- Minimum Rust coverage:
  - update `src-tauri/tests/camera_contract.rs` for approved-state mapping and capture gating
  - update `src-tauri/tests/mock_sidecar_integration.rs` so retryable degraded helper output resolves to the expected blocked waiting state
  - add or update `camera_host.rs` tests if host mapping logic becomes more explicit than the current ready/not-ready split
- Explicit verification targets:
  - customer sees a wait message, not a call instruction, for retryable blocked states
  - customer sees the branch phone number only for `phone-required`
  - capture CTA is enabled only in `ready`
  - active preset survives a temporary readiness regression
  - no raw helper details such as SDK filenames or paths appear in booth copy

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-13.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/workflow-execution-log.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/shared-contracts/dto/cameraStatus.ts`
- `src/shared-contracts/dto/cameraErrorContract.ts`
- `src/shared-contracts/dto/errorEnvelope.ts`
- `src/shared-contracts/dto/cameraContract.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/session-domain/state/customerPreparationState.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/customer-flow/copy/preparationScreenCopy.ts`
- `src/customer-flow/selectors/customerCameraStatusCopy.ts`
- `src/customer-flow/screens/PreparationScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `tests/contract/cameraReadinessStatus.test.ts`
- `tests/contract/cameraContract.test.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- React 19.2 release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri v2 frontend communication docs: https://v2.tauri.app/develop/calling-frontend/
- Zod 4 docs: https://zod.dev/v4

### Latest Technical Information

Verified against official docs on 2026-03-13:

- React 19.2 remains the current official release line and still documents `useEffectEvent` as the right pattern for effect-triggered event handling. That matches the existing readiness watcher pattern in `SessionFlowProvider.tsx`; do not regress to ad hoc ref-driven event plumbing here.
- Tauri v2 official docs continue to position commands for request/response and frontend communication channels/events for ongoing updates. The current `cameraAdapter -> get_camera_readiness_snapshot/watch_camera_readiness` seam already matches that guidance and should stay the integration path for this story.
- Zod 4 remains the current schema-validation line. Continue using it to guard readiness DTO changes at the TypeScript boundary instead of adding parallel runtime validation code.
- No story-level dependency upgrade is required. This story should consume the checked-in stack and focus on readiness-state normalization and gating behavior.

### Project Context Reference

- Follow `_bmad-output/project-context.md` as the compressed implementation rule set for this story:
  - keep React UI out of direct Tauri commands
  - preserve typed DTOs across TypeScript and Rust
  - keep code domain-first
  - preserve session folders as durable booth truth
  - avoid customer-facing diagnostics and cross-session leakage
- Highest-signal planning inputs for Story 3.1 were:
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`
  - `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-13.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Treat `workflow-execution-log.md` and the March 13 readiness report as execution-context warnings, not as permission to preserve the interim `checking-camera` state model. Use the refreshed planning artifacts to interpret Story 3.1 scope when sprint-tracker wording lags behind.

## Story Readiness

- Status: `review`
- Primary implementation risk: continuing to conflate temporary waiting and irreversible phone escalation will either block capture too aggressively or give customers the wrong recovery instruction.
- Primary guardrail: the host owns the approved readiness state, and temporary blocked states must not erase active-session or active-preset context.
- Dependency note: Story 3.2 should consume the capture CTA seam created here; Story 3.1 must not fake persisted captures or latest-photo confirmation.
- Scope adjustment from latest baseline: none. The corrected planning baseline reaffirmed the Story 3.1 / Story 3.2 split; this refresh only tightens that boundary language.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Initial story context refresh: `_bmad/bmm/workflows/4-implementation/create-story`
- Review remediation: `_bmad/bmm/workflows/4-implementation/dev-story`
- Review validation: `_bmad/bmm/workflows/4-implementation/code-review`
- Follow-up remediation for host-truth readiness mapping and contradictory-payload regressions: `_bmad/bmm/workflows/4-implementation/dev-story`
- Contract-alignment remediation for Rust/TypeScript readiness invariants and offline degraded fallback normalization: `_bmad/bmm/workflows/4-implementation/dev-story`
- Review remediation for readiness-watch recovery and malformed channel fallback: `_bmad/bmm/workflows/4-implementation/dev-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual checklist validation performed against `_bmad/bmm/workflows/4-implementation/dev-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so validation was completed manually against the generated story content and checklist intent.

### Completion Notes List

- Story context was refreshed against the March 13 corrected planning baseline and the new implementation-readiness assessment, while keeping Story 3.1 scoped to readiness and capture gating only.
- The main repo/product drift for this story is that current code models blocked readiness as `checking-camera`, while the approved Epic 3 contract requires explicit `waiting` and `phone-required` outcomes.
- The story keeps active-session and preset preservation explicit because current reducer behavior would otherwise clear too much state on a temporary readiness regression.
- Capture persistence and latest-photo confirmation were intentionally kept out of scope so Story 3.2 can own the first real capture outcome path.
- Implementation completed on 2026-03-13 by replacing the interim `checking-camera` model across the TypeScript and Rust readiness seams, preserving active preset/session context through readiness regressions, and aligning customer copy to `preparing`, `waiting`, `ready`, and `phone-required`.
- Verification completed with targeted Vitest coverage, a production `pnpm build`, and full `cargo test --manifest-path src-tauri/Cargo.toml`.
- Senior review fixes on 2026-03-13 removed the deprecated `checking-camera` contract state, restored direct `phone-required` normalization in the session-domain seam, and wired the capture CTA to the shared readiness/timing gate instead of request-status-only disablement.
- Additional regression coverage now asserts deprecated-state rejection, direct `phone-required` handling without an error envelope, readiness/timing-driven capture disablement in `CustomerFlowScreen`, and Rust host mapping for waiting vs phone-required with `capture_enabled = false`.
- Review-record corrections on 2026-03-13 aligned the story status fields, refreshed the audit trail for the code-review/dev-story follow-up, and removed stale test-failure claims from the story record.
- Follow-up review remediation on 2026-03-13 stopped React from re-deriving `waiting` vs `phone-required` from the error envelope, enforced the `connectionState`/`captureEnabled` contract pairing at the shared schema boundary, and added contradictory-payload regressions for the session-domain seam.
- Final review remediation on 2026-03-13 aligned the Rust and TypeScript readiness contracts by adding native `CameraReadinessStatus` validation, escalating degraded `offline` snapshots without an error envelope to `phone-required`, and rejecting contradictory normalized `ready` payloads in the session-domain seam instead of silently remapping them.
- Review follow-up remediation on 2026-03-13 kept the readiness watcher alive through `phone-required` so the booth can recover to `ready`, converted malformed watched readiness payloads into a typed contract-failure `phone-required` fallback, and preserved timing refreshes for blocked preparation without re-opening raw host-state inference in React.
- Final follow-up remediation on 2026-03-13 bounded the post-end timer across browser timeout limits, wired the enabled ready-state CTA into the real customer flow, and stabilized the readiness watcher so state churn does not create duplicate subscriptions.
- Fresh verification on 2026-03-13:
  - `pnpm vitest run src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx` passed (6 tests)
  - `pnpm vitest run tests/integration/customerReadinessFlow.test.tsx` passed (10 tests)
  - `pnpm vitest run tests/integration/postEndFlow.test.tsx` passed (2 tests)
  - `pnpm vitest run` passed (63 files, 213 tests)
  - `pnpm build` passed
  - `cargo test --manifest-path src-tauri/Cargo.toml` passed
- Revalidation on 2026-03-13 for a direct Story 3.1 dev-story rerun confirmed the current repo state still satisfies the story without additional code changes:
  - `pnpm vitest run tests/contract/cameraReadinessStatus.test.ts src/customer-flow/selectors/customerCameraStatusCopy.spec.ts src/customer-flow/screens/PreparationScreen.spec.tsx src/customer-flow/screens/CustomerFlowScreen.spec.tsx src/capture-adapter/host/cameraAdapter.spec.ts src/session-domain/state/customerPreparationState.spec.ts src/session-domain/state/sessionReducer.review.spec.ts tests/integration/customerReadinessFlow.test.tsx tests/integration/presetSelectionFlow.test.tsx` passed (9 files, 48 tests)
  - `pnpm vitest run` passed (64 files, 216 tests)
  - `pnpm lint` passed
  - `pnpm build` passed
  - `cargo test --manifest-path src-tauri/Cargo.toml` passed
- ✅ Resolved review finding [High]: `cameraAdapter` now assigns explicit `watchId` values and calls native `unwatch_camera_readiness` during cleanup so readiness teardown stops the Tauri watch loop instead of only muting the channel.
- ✅ Resolved review finding [High]: capture-confidence cleanup now calls native `unwatch_capture_confidence` with the matching watch id, closing the native polling loop instead of leaving the host thread running.
- ✅ Resolved review finding [Medium]: `watch_camera_readiness` now consumes a single mock-sidecar readiness stream per watcher session instead of respawning the mock sidecar on every 750 ms poll, and Rust integration coverage now asserts the streaming seam.
- ✅ Resolved review finding [Medium]: the non-Tauri fallback readiness watch now supports a deterministic `phone-required` escalation path for browser/dev verification flows keyed off the session id token.
- ✅ Resolved review finding [Medium]: the Story 3.1 file list now reflects the actual readiness-watch remediation seam, and the review record now states explicitly that git verification remains limited while this workspace is still largely untracked.
- ✅ Resolved follow-up review finding [High]: late-resolving readiness watch subscriptions are now cancelled immediately when the provider has already cleaned up, so native unwatch still runs across async teardown races.
- ✅ Resolved follow-up review finding [High]: capture-confidence transport failures no longer mutate booth readiness into `phone-required`; readiness remains host-owned and the capture surface resumes when confidence sync is unavailable during Story 3.1 gating.
- ✅ Resolved follow-up review finding [Medium]: browser fallback readiness snapshots now surface the deterministic `phone-required` dev token immediately instead of always starting from `preparing`.
- ✅ Resolved follow-up review finding [High]: browser fallback capture paths no longer fabricate `captureStarted` / `captureCompleted` events, successful capture results, or synthetic latest-photo updates inside Story 3.1.
- Fresh verification on 2026-03-14:
  - `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts` passed (14 tests)
  - `cargo test --manifest-path src-tauri/Cargo.toml --test mock_sidecar_integration` passed (3 tests)
  - `pnpm test:run` passed (64 files, 219 tests)
  - `pnpm lint` passed
  - `pnpm build` passed
  - `cargo test --manifest-path src-tauri/Cargo.toml` passed
- Follow-up verification on 2026-03-14:
  - `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx tests/integration/customerReadinessFlow.test.tsx` passed (3 files, 34 tests)
  - `pnpm lint` passed
  - `pnpm build` passed
- Review remediation verification on 2026-03-14:
  - `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts` failed first on 3 new regressions covering deterministic `phone-required` fallback and synthetic browser capture/latest-photo behavior.
  - `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts` passed after the fallback remediation changes (18 tests).

### File List

- `_bmad-output/implementation-artifacts/3-1-readiness-states-and-capture-gating.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `sidecar/mock/mock-camera-sidecar.mjs`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/capture-adapter/host/cameraAdapter.spec.ts`
- `src/capture-adapter/host/cameraCommands.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/capture/sidecar_client.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tests/mock_sidecar_integration.rs`

## Change Log

- 2026-03-13: Initial implementation completed and advanced to review.
- 2026-03-13: Senior review fixes applied for deprecated readiness state removal, direct `phone-required` handling, readiness-driven capture CTA disablement, and matching regression coverage.
- 2026-03-13: Review follow-up cleanup aligned story status metadata, corrected the review audit trail, narrowed the File List to actual Story 3.1 implementation/review files, and refreshed the full-suite verification record.
- 2026-03-13: Follow-up review fixes hardened host-truth readiness mapping, rejected contradictory `ready`/`captureEnabled:false` payloads at the shared schema boundary, and added explicit regression coverage for contradictory payload handling.
- 2026-03-13: Final review fixes aligned Rust and TypeScript readiness invariants, escalated degraded offline snapshots without an error envelope to `phone-required`, and refreshed the verification record with passing targeted, Rust, build, and full Vitest runs.
- 2026-03-13: Review follow-up fixes kept readiness watch active through `phone-required`, added malformed watched-payload fallback coverage at the adapter boundary, and corrected the verification record to reflect the remaining non-Story-3.1 Vitest regressions.
- 2026-03-13: Final follow-up fixes bounded post-end timeout scheduling, wired the ready-state CTA into the actual customer flow, stabilized readiness watch subscription scope, and refreshed the verification record with fully passing targeted and full-suite runs.
- 2026-03-13: Revalidated Story 3.1 against the current repo state with targeted readiness tests, the full Vitest suite, lint, production build, and Rust tests; no additional implementation changes were required.
- 2026-03-14: Follow-up code review found 2 High and 3 Medium issues, added AI review follow-ups, and returned Story 3.1 to in-progress.
- 2026-03-14: Addressed the reopened Story 3.1 AI review findings by wiring explicit native unwatch commands for readiness and capture-confidence, replacing readiness re-poll churn with a streamed mock-sidecar watcher, extending browser fallback to exercise `phone-required`, reconciling the file list, and returning the story to review.
- 2026-03-14: Closed the remaining follow-up review gaps by cancelling late-resolving async watcher subscriptions after provider cleanup, keeping capture-confidence failures out of the readiness state machine, extending fallback readiness snapshots to honor the deterministic `phone-required` dev token, and adding focused provider regression coverage.
- 2026-03-14: Removed synthetic browser capture/latest-photo fallback behavior from Story 3.1, kept `phone-required` fallback deterministic across snapshot and watch paths, and corrected the review record so it no longer claims git-trackable verification while the workspace remains largely untracked.

## Senior Developer Review (AI)

- Reviewer: GPT-5 Codex
- Date: 2026-03-13
- Outcome: approved after fixes
- Fixed findings:
  - Removed deprecated `checking-camera` from the shared TypeScript readiness schema.
  - Updated `deriveCustomerPreparationState()` so a direct `connectionState: 'phone-required'` remains an escalation state even without an accompanying error envelope.
  - Added Rust host-mapping regression tests that prove waiting vs phone-required normalization keeps `capture_enabled` blocked.
  - Replaced request-status-only capture disablement in `CustomerFlowScreen` with the composed `selectCaptureActionEnabled()` gate.
- Verification:
  - `pnpm vitest run tests/contract/cameraReadinessStatus.test.ts src/customer-flow/selectors/customerCameraStatusCopy.spec.ts src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
  - `pnpm vitest run tests/integration/customerReadinessFlow.test.tsx tests/integration/sessionPresetChangeFlow.test.tsx`
  - `pnpm vitest run`
  - `pnpm build`
  - `cargo test --manifest-path src-tauri/Cargo.toml`

### Follow-up Review (2026-03-14)

- Reviewer: GPT-5 Codex
- Outcome: changes requested
- Summary: The current repo still contains native watcher lifecycle leaks, process churn in the readiness watch path, an incomplete browser fallback model, and a Story 3.1 audit trail that does not fully reconcile to the implementation seam.
- Action Items:
  - [x] [High] Add an explicit unsubscribe/cancellation path for readiness watch so React cleanup stops the native polling thread instead of only clearing `onmessage`. [src/capture-adapter/host/cameraAdapter.ts:302, src-tauri/src/commands/capture_commands.rs:62]
  - [x] [High] Add an explicit unsubscribe/cancellation path for capture-confidence watch so React cleanup stops the native polling thread instead of only clearing `onmessage`. [src/capture-adapter/host/cameraAdapter.ts:400, src-tauri/src/commands/capture_commands.rs:91]
  - [x] [Medium] Replace snapshot re-polling with a real readiness stream or equivalent bounded host watcher; the current implementation respawns the mock sidecar process for every 750 ms poll and only samples terminal snapshot results. [src-tauri/src/commands/capture_commands.rs:66, src-tauri/src/capture/camera_host.rs:41, src-tauri/src/capture/sidecar_client.rs:199]
  - [x] [Medium] Extend the non-Tauri fallback readiness path to represent the full approved blocked-state model, including `phone-required`, so browser/dev flows can exercise the Story 3.1 escalation UX. [src/capture-adapter/host/cameraAdapter.ts:146, src/capture-adapter/host/cameraAdapter.ts:196]
  - [x] [Medium] Reconcile the Story 3.1 File List and review record with the actual readiness seam files, and remove claims of git-trackable verification while the current workspace remains largely untracked. [C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/3-1-readiness-states-and-capture-gating.md:117, C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/3-1-readiness-states-and-capture-gating.md:300]
