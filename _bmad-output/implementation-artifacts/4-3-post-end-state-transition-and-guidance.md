# Story 4.3: Post-End State Transition and Guidance

Status: review

Story Key: `4-3-post-end-state-transition-and-guidance`

## Summary

Extend the current host-owned timing foundation into an explicit post-end customer flow that leaves capture mode at the authoritative session end, resolves exactly one customer-safe outcome (`export-waiting`, `completed`, or `handoff`), shows the session name when handoff requires it, and falls back to bounded wait/call guidance without leaking raw export status codes, filesystem paths, or operator diagnostics.

## Story

As a booth customer,
I want clear guidance after the session ends,
so that I know the next step without technical confusion.

## Acceptance Criteria

1. Given the session ends, when the system transitions post-end, then it enters exactly one of Export Waiting, Completed, or Handoff, and the customer sees clear next-step guidance.
2. Given the session name is required for handoff, when the handoff state is shown, then the session name is displayed on the handoff surface, and if resolution fails, the customer is routed to wait or call guidance.

## Tasks / Subtasks

- [x] Add a host-owned post-end outcome contract instead of leaking raw export status. (AC: 1, 2)
  - [x] Introduce a typed DTO/schema for customer-safe post-end outcomes such as `export-waiting`, `completed`, and `handoff`, with bounded unresolved guidance handled separately from the primary outcome enum.
  - [x] Keep internal export persistence states such as `notStarted`, `queued`, `processing`, `completed`, and `failed` inside the host/session storage layer; the frontend should receive a translated outcome snapshot, not raw persistence codes.
  - [x] Resolve the post-end outcome from the active session manifest, export-status artifact, and authoritative `timing.actualShootEndAt` so the host remains the single source of truth.

- [x] Extend the session flow so capture mode ends at the authoritative shoot-end threshold. (AC: 1)
  - [x] Add a new session phase or completion-handoff state path after `capture-ready`; do not model post-end flow as a route change.
  - [x] Reuse the existing timing seams (`sessionTimingService`, `captureConfidence.shootEndsAt`, and timing selectors) for threshold scheduling or refresh, but require host confirmation before showing the final post-end outcome.
  - [x] When post-end begins, close preset-selection UI, disable capture mutations, and preserve current-session confidence context without allowing new captures.

- [x] Render a customer-safe post-end surface for exactly one explicit outcome. (AC: 1, 2)
  - [x] Create booth-facing screens/components that show either Export Waiting, Completed, or Handoff guidance with copy-budget-compliant instruction/support/action text.
  - [x] Keep diagnostics, raw export codes, filesystem paths, and internal preset-authoring terms out of the customer surface.
  - [x] Use the active session identity as the display source for the session name wherever the handoff outcome requires it.

- [x] Show session-name-aware handoff and bounded unresolved guidance. (AC: 2)
  - [x] On handoff, show the active `sessionName` together with the approved next-step target, recipient, or location when configured.
  - [x] If the post-end flow cannot resolve normally, translate the condition into approved wait/call guidance using branch contact info instead of inventing a fourth customer-facing completion mode.
  - [x] Keep unresolved guidance compatible with the existing `phone-required` safety language and escalation boundaries already used elsewhere in the booth flow.

- [x] Record post-end lifecycle transitions through the existing diagnostics boundary. (AC: 1, 2)
  - [x] Add lifecycle logger/service wrappers for `actual_shoot_end`, `export_state_changed`, `session_completed`, and `phone_required` instead of invoking log commands directly from screens.
  - [x] Include `actualShootEndAt`, current stage, and session identity where required by the existing schema.
  - [x] Keep logging failures silent/customer-safe and do not block the visible guidance surface on diagnostics persistence errors.

- [x] Add regression coverage for host-owned end-state resolution and customer-safe copy. (AC: 1, 2)
  - [x] Add frontend integration tests proving sessions transition out of capture mode at end time into one explicit post-end surface and that handoff shows the session name.
  - [x] Add contract/state tests proving raw export persistence states are translated into the allowed customer outcomes only.
  - [x] Add Rust repository/command tests proving post-end outcome resolution respects session binding, timing truth, export-status changes, and no cross-session leakage.

## Dev Notes

### Developer Context

- Epic 4 combines FR-006 timing truth with FR-007 completion and handoff clarity. Story 4.3 starts when `actualShootEndAt` is reached and must move the booth out of capture mode into exactly one explicit customer-safe post-end outcome.
- Current repo reality already provides the timing and logging foundation this story should extend:
  - `src/session-domain/state/SessionFlowProvider.tsx` already uses `sessionTimingService` and `deriveTimingThresholds()` to schedule timing-based readiness escalation.
  - `src/customer-flow/screens/CaptureScreen.tsx`, `src/customer-flow/selectors/captureConfidenceView.ts`, and `src/timing-policy/selectors/sessionTimeDisplay.ts` already show the authoritative end time from `captureConfidence.shootEndsAt`.
  - `src/shared-contracts/logging/operationalEvents.ts` and `src-tauri/src/diagnostics/lifecycle_log.rs` already reserve lifecycle event kinds for `warning_shown`, `actual_shoot_end`, `export_state_changed`, `session_completed`, and `phone_required`.
  - `src/shared-contracts/dto/sessionManifest.ts` and `src-tauri/src/session/session_manifest.rs` already persist `timing` and an internal `exportState`.
- The main gap is that the current customer flow stops at `capture-ready`:
  - `src/session-domain/state/sessionReducer.ts` has no post-end or completion-handoff phase.
  - `src/customer-flow/screens/CustomerFlowScreen.tsx` has no post-end screen.
  - The host exposes timing and preset commands, but no command or snapshot exists yet for a translated post-end customer outcome.
- Important contract mismatch to keep visible:
  - The manifest/export storage layer currently uses internal states such as `notStarted`, `queued`, `processing`, `completed`, and `failed`.
  - FR-007 and Story 4.3 require customer-facing outcomes of `Export Waiting`, `Completed`, or `Handoff`.
  - Do not leak the storage enum directly to the booth UI. Translate it through a typed host-owned outcome model.
- Scope boundaries:
  - In scope: explicit post-end outcome selection, customer-safe copy, session-name-aware handoff, bounded wait/call guidance, and lifecycle logging.
  - Out of scope: reworking coupon timing math from Story 4.1, warning choreography from Story 4.2, print/export pipeline redesign, operator console UX, or any customer photo-editing capability.
- Dependency note:
  - No refreshed Epic 4.1 or 4.2 implementation artifact exists yet in `implementation-artifacts`.
  - Implement Story 4.3 against the current authoritative timing seam so later warning and timing-display work can plug in without contract drift or duplicated end-time logic.

### Technical Requirements

- Treat the active session manifest plus export-status artifact as the only durable truth for post-end resolution. Do not introduce browser-local persistence, route params, or ad hoc cache files to remember whether a session is completed or handed off.
- The frontend may schedule a threshold check from `actualShootEndAt`, but browser time must only trigger a host refresh or state query. The visible customer outcome must be confirmed by host-owned state, not invented entirely in React memory.
- Add one typed post-end outcome contract for the customer surface. At minimum it should carry:
  - active `sessionId`
  - authoritative `actualShootEndAt`
  - translated outcome kind (`export-waiting`, `completed`, `handoff`)
  - whether bounded unresolved wait/call guidance is required
  - session-name visibility requirement for handoff
- Preserve the PRD state boundary that there are exactly three primary post-end outcomes. If a failure or unresolved path occurs, translate it into approved wait/call guidance layered onto the appropriate customer-safe path instead of creating a fourth public completion mode.
- When post-end begins:
  - capture must be disabled
  - preset-selection sheets or toasts must close cleanly
  - no new host capture or preset-mutation requests should be accepted from the customer surface
  - existing current-session confidence data may remain visible for reassurance, but it must not imply that more capture is still allowed
- Use the active session identity already held in state as the source for the displayed session name on handoff surfaces. Never reconstruct it from filesystem paths or route text.
- Keep copy-budget compliance explicit:
  - one primary instruction sentence
  - one supporting sentence
  - one primary action label
  - dynamic values such as session name, time, and local phone number may appear without expanding the copy budget
- Logging requirements must remain typed and customer-safe:
  - record `actual_shoot_end` when the authoritative end threshold is reached
  - record `export_state_changed` when the translated post-end storage state changes materially
  - record `session_completed` when the completed outcome is resolved
  - record `phone_required` only when bounded wait/call escalation is necessary
- Do not let logging failures block the booth UI. Diagnostics persistence is important, but the customer must still receive clear guidance if the log write fails.

### Architecture Compliance

- Keep React Router limited to top-level surfaces. Story 4.3 must stay inside the state-driven customer flow and must not add a dedicated completion route tree for booth workflow truth.
- Respect the existing adapter boundary: React components do not call Tauri directly. Any new post-end fetch or command belongs in a typed service/adapter module.
- Preserve session-folder truth. Post-end state must derive from session artifacts and host state, not from view-only client caches.
- Introduce the missing `completion-handoff` domain rather than burying the logic entirely inside `customer-flow` or `timing-policy`. This aligns the live repo with the architecture document's intended domain split.
- Keep customer-visible state translation centralized in selectors/copy modules so the customer surface stays free of diagnostics, raw export codes, internal authoring language, or helper-process details.
- Branch-local variance remains limited to approved local settings such as contact information or approved operational toggles. Do not add branch-specific post-end state machines or hidden bypasses.

### Library / Framework Requirements

- Current workspace baselines in the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - `zod`: `^4.3.6`
  - Rust `tauri`: `2.10.3`
  - `chrono`: `0.4.42`
  - `rusqlite`: `0.38.0`
- Keep React event/timer coordination inside provider-level effects. If the post-end threshold logic needs fresh state without re-registering timers, follow the current `useEffectEvent` pattern already used in `SessionFlowProvider`.
- Keep Tauri communication on typed commands for request/response state queries. If streaming is required later, prefer the existing adapter/channel patterns over ad hoc global events.
- Keep Zod as the TypeScript boundary gate for any new post-end DTOs, manifest-adjacent payloads, and lifecycle logger inputs.
- Do not add a new persistence or global-store library for this story. The repo already has the correct split:
  - durable booth truth in session files and host state
  - typed frontend state in reducer/context domains
  - minimal local settings in the Store plugin only

### File Structure Requirements

- Expected frontend files to inspect or update:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/customer-flow/selectors/captureConfidenceView.ts`
  - `src/timing-policy/selectors/sessionTimeDisplay.ts`
- Expected new frontend domain surfaces:
  - `src/completion-handoff/state/*`
  - `src/completion-handoff/services/*`
  - `src/customer-flow/screens/PostEndScreen.tsx` or split `ExportWaitingScreen.tsx` / `CompletedScreen.tsx` / `HandoffScreen.tsx`
  - `src/customer-flow/selectors/postEndView.ts`
  - `src/customer-flow/copy/postEndCopy.ts`
- Shared contract files likely needing changes:
  - `src/shared-contracts/dto/sessionManifest.ts`
  - `src/shared-contracts/schemas/manifestSchemas.ts`
  - new `src/shared-contracts/dto/postEndOutcome.ts` (or equivalent)
  - new `src/shared-contracts/schemas/postEndOutcomeSchemas.ts` if separated
  - `src/shared-contracts/logging/operationalEvents.ts`
- Host files likely relevant:
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/lib.rs`
  - new `src-tauri/src/handoff/*` or equivalent host-owned completion module
- Keep post-end workflow logic out of:
  - `src/shared-ui/*` beyond presentation-only components
  - `src/branch-config/*` except reading approved local contact information
  - new route trees that treat completion as URL truth

### Testing Requirements

- Add or update frontend integration coverage proving:
  - a session leaves `capture-ready` when the authoritative end threshold is reached
  - only one of the allowed post-end outcomes is rendered at a time
  - the handoff surface includes the active session name when required
  - unresolved cases show bounded wait/call guidance without diagnostics leakage
  - capture and preset-change actions are disabled once post-end has started
- Add reducer/state tests proving:
  - post-end transitions do not wipe active session identity unexpectedly
  - capture-confidence context can remain visible as reassurance without allowing new capture
  - pending preset UI does not remain stranded after post-end transition
- Add contract tests proving:
  - internal export persistence states are never surfaced directly as booth-facing outcome strings
  - any new post-end DTOs stay aligned across TypeScript and Rust
  - copy-budget-sensitive states remain constrained to the approved shape
- Add or extend Rust tests proving:
  - post-end outcome resolution respects `sessionId` binding
  - manifest/export-status changes persist and reload without shape drift
  - `actual_shoot_end_at` remains the authoritative threshold for resolution
  - unresolved or failed export conditions map to bounded guidance rather than raw storage failure codes

### Previous Story Intelligence

- There is no refreshed Epic 4.1 or 4.2 implementation artifact yet in `implementation-artifacts`.
- The practical predecessor intelligence comes from the current repo seams:
  - timing is already persisted in the session manifest and exposed through typed timing commands
  - the customer capture surface already treats `shootEndsAt` as a visible, authoritative banner value
  - readiness escalation in `SessionFlowProvider` already demonstrates the preferred timer-plus-host-query pattern for threshold-driven state changes
- Treat those seams as precedent. Story 4.3 should extend them carefully rather than building a second, parallel timing truth.

### Git Intelligence Summary

- Recent commits remain dominated by the greenfield reset and camera-state normalization work:
  - `06ed2b7` restructured the repository around the refreshed BMAD planning baseline.
  - Earlier camera-focused commits reinforce the same rule: customer UI must stay derived from host-normalized truth, not scattered local assumptions.
- Actionable guidance for Story 4.3:
  - keep post-end outcome resolution host-owned
  - let the frontend react to typed outcome snapshots instead of fabricating completion truth locally
  - reuse existing diagnostics/logging seams rather than adding another ad hoc persistence channel

### Latest Tech Information

- Official-source verification performed on 2026-03-12:
  - React 19.2 official docs continue to position `useEffectEvent` as the right tool for timers and listeners that must read the latest state without re-registering effects. That matches the existing `SessionFlowProvider` pattern and should remain the default approach for post-end threshold coordination.
  - Tauri v2 official docs continue to position commands as the standard request/response boundary between the frontend and Rust host. The same docs note that the event system is more dynamic but not type-safe, always async, cannot return values, and only supports JSON payloads, so it is a poor substitute for authoritative post-end state queries.
  - Zod 4 remains the stable official docs line. Continue using it to validate any new post-end DTOs and lifecycle payloads on the TypeScript boundary before they cross into Tauri.

### Project Structure Notes

- The repo already follows a domain-first split that Story 4.3 should preserve:
  - `customer-flow` owns booth-facing screen composition
  - `session-domain` owns active session lifecycle state
  - `timing-policy` owns timing math and display helpers
  - `capture-adapter` owns typed host-facing capture/timing snapshots
  - `diagnostics-log` owns lifecycle-event persistence helpers
- The missing architecture-aligned domain here is `completion-handoff`. Add that domain intentionally instead of pushing all post-end logic into `CustomerFlowScreen` or `SessionFlowProvider`.
- The most relevant rules from `_bmad-output/project-context.md` for this story are:
  - keep React components away from direct Tauri invocation
  - preserve session folders as the durable source of truth
  - do not expose raw camera/helper/internal diagnostics to customer UI
  - keep routes limited to top-level surfaces
  - keep shared DTOs fully typed and validated with Zod at the TypeScript boundary

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/validation-report-2026-03-12.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/timing-policy/services/sessionTimingService.ts`
- `src/timing-policy/state/timingSelectors.ts`
- `src/timing-policy/selectors/sessionTimeDisplay.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/shared-contracts/dto/sessionTiming.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/timing/shoot_end.rs`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/sessionLifecycle.test.ts`
- `src-tauri/tests/session_timing_repository.rs`
- React 19.2 release: https://react.dev/blog/2025/10/01/react-19-2
- React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Scope: host-owned post-end state resolution, customer-safe completion/handoff guidance, and lifecycle logging alignment
- Reuse strategy: extend the current timing foundation and diagnostics seams instead of inventing a route-based or client-only completion workflow
- Contract sensitivity: high because timing truth, manifest/export state, customer copy boundaries, and lifecycle logs intersect in one story
- Key guardrail: do not expose raw export persistence states or let the frontend invent post-end truth independently of the host

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/dev-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation target: `_bmad/bmm/workflows/4-implementation/dev-story/checklist.md`

### Completion Notes List

- Added a host-owned post-end outcome contract and Tauri command path so the frontend resolves `export-waiting`, `completed`, and `handoff` from manifest/export artifacts without leaking raw persistence codes.
- Extended the session reducer and provider with a `post-end` phase, authoritative end-threshold resolution, lifecycle logging hooks, and customer-safe handoff/wait-or-call rendering while preserving active-session context.
- Stabilized same-session readiness resubscription so readiness changes no longer overwrite current state with stale snapshots during preparation or capture transitions.
- Verified the story with `pnpm test:run -- --isolate --maxWorkers=1`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml`.

### File List

- `_bmad-output/implementation-artifacts/4-3-post-end-state-transition-and-guidance.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/completion-handoff/services/postEndOutcomeService.ts`
- `src/customer-flow/copy/postEndCopy.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/PostEndScreen.tsx`
- `src/customer-flow/selectors/postEndView.ts`
- `src/diagnostics-log/services/lifecycleLogger.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.story-3-2.spec.ts`
- `src/session-domain/state/sessionReducer.story-4-3.spec.ts`
- `src/session-domain/state/sessionReducer.ts`
- `src/shared-contracts/dto/postEndOutcome.ts`
- `src/shared-contracts/schemas/postEndOutcomeSchemas.ts`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/session/session_repository.rs`
- `tests/integration/postEndFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`

### Change Log

- 2026-03-13: Implemented Story 4.3 post-end outcome resolution, customer-safe handoff UI, lifecycle logging hooks, and regression coverage; verified with isolated full Vitest, ESLint, and Cargo test runs.
