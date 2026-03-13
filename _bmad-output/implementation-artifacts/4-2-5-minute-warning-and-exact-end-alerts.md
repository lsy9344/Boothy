# Story 4.2: 5-Minute Warning and Exact-End Alerts

Status: in-progress

Story Key: `4-2-5-minute-warning-and-exact-end-alerts`

## Summary

Extend the current timing foundation so the active capture surface can issue a sound-backed 5-minute warning and a definitive exact-end alert from the authoritative session end time that already lives in the manifest and capture-confidence snapshot. Reuse the existing `sessionTimingService -> deriveTimingThresholds -> SessionFlowProvider -> CaptureScreen` seam, re-arm correctly when operator extensions change `actualShootEndAt`, log the lifecycle events exactly once, and make the capture UI clearly show whether shooting can still continue or has ended without jumping ahead into Story 4.3's post-end state machine.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want clear warnings before the session ends and a definitive end alert,
so that I can finish without confusion.

## Acceptance Criteria

1. Given the session is within 5 minutes of the adjusted end time, when the warning threshold is reached, then a sound-backed warning alert is triggered, and a visible warning badge is shown.
2. Given the adjusted end time is reached, when the end threshold is hit, then a sound-backed end alert is triggered, and the UI clearly indicates whether shooting can continue or has ended.

## Tasks / Subtasks

- [ ] Build one authoritative timing-alert scheduler from the stored session end time instead of inventing a second timing model. (AC: 1, 2)
  - [ ] Reuse `actualShootEndAt` / `shootEndsAt` from the existing timing and capture-confidence contracts as the only source for alert scheduling.
  - [ ] Reuse `deriveTimingThresholds()` for `warningAt` and exact-end computation; do not recompute coupon or session-duration policy inside React.
  - [ ] Key warning/end scheduling to the effective timing revision so operator extensions or refreshed timing loads cancel stale timers and arm new ones cleanly.

- [ ] Store warning/end alert state in the session-domain seam and drive the capture screen from it. (AC: 1, 2)
  - [ ] Add explicit customer-facing timing-alert state for at least `none`, `warning`, and `ended`.
  - [ ] Compose capture availability from both readiness and timing status so the exact-end threshold disables capture without introducing another one-off boolean.
  - [ ] Remove the current hard-coded `captureActionDisabled` behavior in the active capture surface and replace it with a state-driven gate.

- [ ] Surface a visible 5-minute warning badge and a definitive exact-end message without hiding the rest of the trust anchors. (AC: 1, 2)
  - [ ] Keep session name, visible end time, current preset, and latest-photo confirmation on screen while alerts are shown.
  - [ ] Reuse or extend the current time banner / capture guidance area instead of introducing a new route or full-screen modal.
  - [ ] Keep customer copy within the copy budget and free of internal timing-policy or diagnostic language.

- [ ] Add sound-backed warning and exact-end alerts using platform audio primitives, not a new dependency. (AC: 1, 2)
  - [ ] Play distinct warning and end sounds exactly once per threshold crossing for the effective timing revision.
  - [ ] Handle audio playback promise failures without blocking the visual alert path.
  - [ ] Do not assume autoplay success; treat sound as best-effort enhancement while the visual alert remains mandatory.

- [ ] Log timing alert milestones as lifecycle events exactly once. (AC: 1, 2)
  - [ ] Extend the existing lifecycle logger with dedicated methods for `warning_shown` and `actual_shoot_end`.
  - [ ] Include the session identity, branch context, current stage, and authoritative `actualShootEndAt` in those log writes.
  - [ ] Prevent duplicate warning/end log writes during rerenders, watch re-subscriptions, and timer rearming.

- [ ] Keep Story 4.2 scoped to alerts and exact-end gating, not the full post-end workflow. (AC: 2)
  - [ ] Do not implement `export-waiting`, `completed`, or `handoff` transitions here; Story 4.3 owns that state machine.
  - [ ] If exact end is reached before Story 4.3 lands, the capture surface may remain the active surface temporarily, but it must clearly show that shooting has ended and the primary capture action is disabled.
  - [ ] If an operator extension moves the authoritative end time back into the future, the alert state must re-evaluate cleanly from the new timing record rather than staying permanently ended.

- [ ] Add regression coverage for threshold scheduling, alert visibility, sound playback handling, and exact-end gating. (AC: 1, 2)
  - [ ] Add timing-selector and provider-level tests for in-window mounts, exact-threshold firing, extension-driven rearming, and once-only event emission.
  - [ ] Add capture-surface tests that prove warning copy/badge visibility and exact-end capture disablement.
  - [ ] Add logging/audio tests that prove visual alerts still work when playback rejects and that lifecycle events are written once.

## Dev Notes

### Developer Context

- The refreshed Epic 4 / FR-006 baseline is narrower than a full post-end completion workflow. Story 4.2 only owns the customer-visible 5-minute warning and exact-end alert behavior; Story 4.3 owns the subsequent export-waiting / completion / handoff transition logic.
- The current repo already contains the right foundation for this story:
  - `session.json` persists authoritative timing state with `actualShootEndAt`.
  - `sessionTimingService` loads timing through typed host commands rather than direct manifest reads in the UI.
  - `deriveTimingThresholds()` already derives `warningAt`, `shootStopAt`, and `phoneEscalationAt` from the authoritative timing record.
  - `SessionFlowProvider` already schedules one timing-derived browser timer for preparation phone escalation, so there is an established scheduling seam to extend.
  - `CaptureScreen` and `SessionTimeBanner` already display customer-facing timing context.
  - Lifecycle event schemas already define `warning_shown` and `actual_shoot_end`, but the frontend logger does not currently emit them.
- The main implementation drift in the current repo is that the active capture surface still passes `captureActionDisabled` as a hard-coded constant, so there is not yet a single composed "can capture" decision that includes readiness plus timing.
- There is no refreshed Story 4.1 implementation artifact in `implementation-artifacts` yet. The practical predecessor intelligence for Story 4.2 therefore comes from the current checked-in timing code rather than a prior Epic 4 story document.
- Scope boundary:
  - In scope: warning/end threshold scheduling, visible customer alerts, sound playback, lifecycle logging, and exact-end capture gating.
  - Out of scope: export-waiting state, completion handoff surfaces, operator extension UI, or deeper delivery-package logic.

### Technical Requirements

- Treat the authoritative stored end time as the only timing truth:
  - use `manifest.timing.actualShootEndAt` on the host side
  - use `timing.actualShootEndAt` / `captureConfidence.shootEndsAt` on the frontend side
  - do not derive warning/end behavior from a hard-coded `50 minutes` or `100 minutes` assumption in UI code
- The warning threshold is exactly 5 minutes before the current authoritative end time.
- The exact-end alert occurs at the current authoritative end time.
- Threshold handling must be resilient in all three mount scenarios:
  - before the warning window
  - already inside the warning window but before exact end
  - already past exact end
- Recompute thresholds whenever the effective timing changes, including operator extensions reflected by a new `actualShootEndAt` or timing revision.
- Warning and end alerts must each fire once per effective timing revision. Rerenders, renewed watches, and repeated provider mounts must not replay the same alert indefinitely.
- If an extension moves the end time later after a warning or exact-end alert has already occurred, alert state must be recomputed against the new timing record:
  - stale timers must be cancelled
  - newly relevant timers must be armed
  - the UI must stop claiming the session has ended if the authoritative timing now says it has not
- Sound playback is an enhancement layered onto the visual alert, not the source of truth:
  - the UI must still show the correct warning/end state if sound playback fails
  - playback failures must not crash the capture surface or block timing-state updates
- Exact-end gating must disable capture immediately when the end threshold is reached, but this story should not invent the full post-end completion route. If Story 4.3 is not yet implemented, keep the customer on the current capture surface with clear "shooting has ended" guidance.
- Keep alert visibility customer-safe and product-trust-focused:
  - visible badge or banner
  - short supporting sentence
  - no filesystem paths
  - no "policy", "extension rule", "sidecar", "manifest", or other internal terminology
- Keep session isolation intact:
  - warning/end alert state belongs to the current active session only
  - timing alerts must not outlive or bleed into another session after `active_session_cleared`
  - latest-photo context must remain session-scoped while alerts are shown

### Architecture Compliance

- Preserve the architecture rule that timing policy is host-owned. The UI may schedule notifications from the already-authoritative end timestamp, but it must not recalculate coupon/session policy or invent alternate timing rules.
- Keep timing-alert orchestration inside the existing state-driven session seam. Do not add a new customer route such as `/warning` or `/ended`.
- Keep React components free of direct Tauri access. Timing loads stay behind `sessionTimingService`, and lifecycle writes stay behind `lifecycleLogger` / operational-log services.
- Preserve the domain-first structure:
  - timing derivation in `src/timing-policy/*`
  - session orchestration in `src/session-domain/*`
  - customer alert presentation in `src/customer-flow/*`
  - logging contracts in `src/shared-contracts/*` and `src/diagnostics-log/*`
- Do not persist warning/end UI state into session storage or branch config. Manifest timing is the durable truth; alert state is runtime orchestration derived from that truth.
- Do not blur Story 4.2 into Story 4.3. Warning/end alerting is a prerequisite for post-end transitions, not a replacement for them.
- Do not add branch-specific timing behavior or local hidden overrides. Warning and exact-end behavior must remain consistent across branches.

### Library / Framework Requirements

- Current workspace baselines from the checked-in repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
- Official-source verification performed on 2026-03-12:
  - React 19.2 continues to document `useEffectEvent` as the correct pattern for event-like logic fired from effects while still reading fresh state. Use that existing provider pattern for timing-alert timers and playback side effects instead of ad hoc refs or global mutable state.
  - Tauri v2 official guidance still uses commands as the standard request/response boundary between the frontend and Rust. Keep timing loads on the typed `get_session_timing` service path rather than bypassing the adapter layer.
  - Zod 4 is the current stable line and remains the TypeScript-side runtime gate for timing DTOs and any alert-related payloads or contracts you add.
  - MDN documents `HTMLMediaElement.play()` as Promise-based and explicitly notes `NotAllowedError` rejection scenarios. Sound playback for alerts must handle rejected promises instead of assuming playback always starts.
- Do not add a third-party audio library for this story. Use platform audio primitives or a tiny local utility around them.

### File Structure Requirements

- Primary timing orchestration seam to inspect and likely modify:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/timing-policy/state/timingSelectors.ts`
  - `src/timing-policy/state/timingSelectors.spec.ts`
  - `src/timing-policy/services/sessionTimingService.ts`
  - `src/timing-policy/services/sessionTimingService.spec.ts`
- Customer-facing alert presentation seam:
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/customer-flow/components/SessionTimeBanner.tsx`
  - `src/customer-flow/selectors/captureConfidenceView.ts`
  - `src/customer-flow/copy/captureFlowCopy.ts`
  - `src/customer-flow/copy/captureScreenCopy.ts`
  - `src/session-domain/state/captureFlowState.ts`
- Logging / contract surfaces likely relevant:
  - `src/diagnostics-log/services/lifecycleLogger.ts`
  - `src/diagnostics-log/services/operationalLogClient.ts`
  - `src/shared-contracts/logging/operationalEvents.ts`
  - `src-tauri/src/diagnostics/lifecycle_log.rs`
- Timing persistence / host surfaces to inspect so frontend work stays aligned with the stored truth:
  - `src/shared-contracts/dto/sessionTiming.ts`
  - `src/shared-contracts/schemas/sessionTimingSchemas.ts`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/timing/shoot_end.rs`
  - `src-tauri/src/timing/extension_rules.rs`
  - `src-tauri/tests/session_timing_repository.rs`
- Test surfaces likely needing additions or updates:
  - `tests/integration/sessionLifecycle.test.ts`
  - `tests/integration/customerReadinessFlow.test.tsx`
  - new or adjacent customer timing-flow integration coverage under `tests/integration/*`
  - `src/timing-policy/state/timingSelectors.spec.ts`
  - `src/customer-flow/screens/CaptureScreen` companion tests if present or newly added
  - `src/diagnostics-log/services/lifecycleLogger.ts` companion tests if added
- Current repo variance to account for:
  - `CustomerFlowScreen` currently hard-disables the capture action instead of composing readiness and timing state.
  - The time banner already exists, but no warning/end badge state is currently modeled.
  - Lifecycle event types already include warning/end milestones, but no frontend logger methods currently write them.

### Testing Requirements

- Add or update selector/state tests to prove threshold math remains anchored to the stored authoritative end time:
  - warning threshold at exactly minus 5 minutes
  - exact-end threshold at exactly `actualShootEndAt`
  - extension-driven rearming when `actualShootEndAt` changes
  - correct behavior when mounting already inside the warning window or already past exact end
- Add provider-level tests with fake timers so timing behavior is deterministic:
  - warning alert appears once at threshold
  - exact-end alert appears once at threshold
  - old timers are cancelled when timing changes
  - capture is disabled at exact end
  - alert state resets or re-evaluates correctly after an extension
- Add UI tests for the active capture surface that prove:
  - a visible warning badge/message is shown in the 5-minute window
  - a visible ended message is shown at exact end
  - session time, preset label, and latest-photo panel remain visible
  - customer copy stays diagnostic-free
- Add logging/audio behavior coverage:
  - warning and exact-end lifecycle events are written once
  - audio playback is attempted once per threshold crossing
  - rejected `play()` promises do not prevent the visual alert from appearing
- Keep contract-sensitive verification explicit:
  - if timing DTOs or envelopes change, update both TypeScript and Rust-side tests
  - if no host contract changes are needed, keep Rust timing-repository tests green to confirm alert work did not drift the stored timing model

### Previous Story Intelligence

- No refreshed Story 4.1 implementation artifact exists yet in `_bmad-output/implementation-artifacts`.
- The practical predecessor intelligence for Story 4.2 comes from the repo's current timing foundation:
  - timing is persisted on session creation and extension
  - the customer surface already displays the end time
  - `deriveTimingThresholds()` already exists
  - `SessionFlowProvider` already schedules a timing-based escalation path
  - lifecycle event schemas already define the warning/end event kinds this story needs
- Treat those seams as the canonical baseline to extend. Do not create a parallel timing-alert framework beside them.

### Git Intelligence Summary

- Recent git history is dominated by the greenfield reset and earlier camera-state stabilization work:
  - `06ed2b7` restructured the repository around the refreshed BMAD planning package.
  - Earlier camera-readiness work repeatedly reinforced the same rule: centralized normalized state beats scattered UI assumptions.
- Actionable guidance for Story 4.2:
  - keep timing-alert truth centralized in the session/timing seam
  - do not scatter alert booleans and duplicate timers across presentation components
  - reuse the typed command/service/logging boundaries that already exist

### Latest Tech Information

Verified against official docs on 2026-03-12:

- React 19.2 official release guidance continues to support `useEffectEvent` for effect-fired event logic that must observe fresh state without forcing unrelated effect dependencies. This fits timing threshold callbacks and sound-playback side effects in `SessionFlowProvider`.
- Tauri v2 official docs continue to define commands as the standard frontend-to-Rust boundary. Use the existing typed `get_session_timing` command/service path for timing loads and keep command usage out of presentation components.
- Zod 4 is the current stable line. Continue using it as the runtime validator for timing DTOs and any alert-related schema additions.
- MDN documents `HTMLMediaElement.play()` as Promise-returning and warns that playback can reject, including `NotAllowedError` scenarios. Alert sound playback must be coded defensively around that Promise result.

### Project Structure Notes

- The current repo already has the right domain split for this story:
  - `timing-policy` owns timing derivation
  - `session-domain` owns active-session orchestration
  - `customer-flow` owns customer-screen rendering
  - `diagnostics-log` owns operational write paths
- One important structure correction should remain explicit:
  - replace the hard-coded capture disablement with a single composed capture gate
  - do not layer a second special-case disable flag on top of the existing hard-coded one
  - alert state should feed the existing capture surface rather than creating a detached warning-only screen

### Project Context Reference

- `_bmad-output/project-context.md` remains the active compressed ruleset for this story.
- The highest-signal project-context rules here are:
  - keep React components away from direct Tauri invocation
  - preserve typed DTOs and Zod validation at the TypeScript boundary
  - keep session folders and persisted timing as durable truth
  - avoid cross-session leakage in UI state and fixtures
  - keep customer-visible copy free of diagnostics and internal terminology
- Story 4.2 should be implemented as a direct extension of those rules, not as an exception.

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/validation-report-2026-03-12.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/workflow-execution-log.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/shared-contracts/dto/sessionTiming.ts`
- `src/shared-contracts/schemas/sessionTimingSchemas.ts`
- `src/shared-contracts/dto/captureConfidence.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src/timing-policy/state/timingSelectors.ts`
- `src/timing-policy/state/timingSelectors.spec.ts`
- `src/timing-policy/services/sessionTimingService.ts`
- `src/timing-policy/services/sessionTimingService.spec.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/session-domain/state/captureFlowState.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/components/SessionTimeBanner.tsx`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/customer-flow/copy/captureFlowCopy.ts`
- `src/customer-flow/copy/captureScreenCopy.ts`
- `src/diagnostics-log/services/lifecycleLogger.ts`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/timing/shoot_end.rs`
- `src-tauri/src/timing/extension_rules.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/tests/session_timing_repository.rs`
- `tests/integration/sessionLifecycle.test.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- React 19.2 release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Zod 4 docs: https://zod.dev/v4
- MDN `HTMLMediaElement.play()`: https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/play

## Story Readiness

- Status: `ready-for-dev`
- Primary implementation risk: duplicate or stale timers when timing loads refresh or operator extensions move the authoritative end time.
- Primary guardrail: warning/end alerts must fire once per authoritative timing revision and must not silently turn into Story 4.3's post-end state machine.
- Reuse strategy: extend the current `session timing -> provider scheduling -> capture surface -> lifecycle log` seam rather than designing a second timing-alert subsystem.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual checklist validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation must be performed manually
- Implemented timing-alert state and scheduling in `SessionFlowProvider`, backed by `deriveTimingThresholds()` and the stored authoritative shoot end time.
- Added customer-facing warning/end rendering in the active capture surface and state-driven capture gating.
- Verification slice passed for `src/session-domain/state`, `src/customer-flow/screens/CaptureScreen.spec.tsx`, `src/customer-flow/selectors/captureConfidenceView.spec.ts`, `src/diagnostics-log/services/lifecycleLogger.spec.ts`, `src/timing-policy/state/timingSelectors.spec.ts`, `tests/integration/postEndFlow.test.tsx`, `tests/integration/sessionGalleryIsolation.test.tsx`, and `tests/integration/presetChangeFlow.test.tsx`.
- Broader verification still reports failing readiness-flow expectations in `tests/integration/customerReadinessFlow.test.tsx`, so the story cannot be marked `review` yet.

### Completion Notes List

- Story context was generated from the refreshed Epic 4.2 requirement, the current timing implementation in the repo, recent git history, and official React / Tauri / Zod / MDN documentation.
- The story intentionally treats the existing timing persistence, threshold derivation, capture-time display, and lifecycle event schema as the canonical baseline to extend.
- The document keeps post-end transitions explicitly out of scope so developers do not accidentally collapse Story 4.2 and Story 4.3 into one unreviewable timing/completion change.
- Added a session-scoped timing-alert state model with `none`, `warning`, and `ended`, plus a single `selectCaptureActionEnabled()` gate so capture disablement now composes readiness and exact-end status.
- Reused `deriveTimingThresholds()` to schedule warning and exact-end transitions from the authoritative timing record, and re-armed timers cleanly when the effective end time changed.
- Added customer-safe warning/end copy, a visible time-banner badge, and exact-end guidance while preserving the session name, visible end time, active preset, and latest-photo trust anchors.
- Added a local platform-audio utility for distinct warning/end sounds and guarded playback so rejected `play()` calls do not block visual alerts.
- Extended lifecycle logging with `warning_shown` and capture-active `actual_shoot_end` writes, deduped once per effective timing revision.
- Added selector, provider, logger, and capture-surface regression coverage for threshold scheduling, extension-driven rearming, once-only alert emission, and exact-end capture disablement.
- Verification is not yet complete because `tests/integration/customerReadinessFlow.test.tsx` still fails against readiness-copy expectations outside the Story 4.2 touchpoints; the story remains `in-progress` until that regression gate is resolved.

### File List

- `_bmad-output/implementation-artifacts/4-2-5-minute-warning-and-exact-end-alerts.md`
- `src/customer-flow/components/SessionTimeBanner.tsx`
- `src/customer-flow/copy/captureFlowCopy.ts`
- `src/customer-flow/screens/CaptureScreen.spec.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/selectors/captureConfidenceView.spec.ts`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/customer-flow/services/timingAlertAudio.ts`
- `src/diagnostics-log/services/lifecycleLogger.spec.ts`
- `src/diagnostics-log/services/lifecycleLogger.ts`
- `src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/session-domain/state/sessionTimingAlertState.ts`
- `src/timing-policy/state/timingSelectors.spec.ts`
- `src/timing-policy/state/timingSelectors.ts`

## Change Log

- 2026-03-13: Implemented Story 4.2 timing-alert scheduling, customer warning/end presentation, once-only audio/logging, and exact-end capture gating; verification remains blocked by unrelated readiness-flow expectation failures in `tests/integration/customerReadinessFlow.test.tsx`.
