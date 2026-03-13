# Story 6.1: Operator Summary and Session Context Visibility

Status: ready-for-dev

Story Key: `6-1-operator-summary-and-session-context-visibility`

## Summary

Introduce the first operator-facing summary surface for the rewritten booth product so an internal user can see the active session, current timing truth, booth readiness, and recent failure context without depending on customer-only screens or exposing diagnostics to the customer route. Reuse the existing session manifest, timing contracts, normalized readiness envelope, and SQLite operational log foundation instead of inventing a second session source of truth or reading database/filesystem state directly from React.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an operator,
I want a summary view of the current session and booth state,
so that I can quickly assess issues without exposing diagnostics to customers.

## Acceptance Criteria

1. Given the operator console is opened, when the summary loads, then it shows current session context, timing state, and recent failure context, and customer-facing screens remain free of diagnostic data.

## Tasks / Subtasks

- [ ] Introduce the first operator route and keep session identity shared without creating a second frontend truth source. (AC: 1)
  - [ ] Move `SessionFlowProvider` to an app-level boundary or equivalent shared wrapper so both `/customer` and `/operator` can read the same active session identity during one runtime.
  - [ ] Add a top-level `/operator` route and an `OperatorSummaryScreen` that renders a safe empty state when no active session exists instead of crashing or guessing a session from disk.
  - [ ] Keep `/customer` as the default route and do not alter customer copy, readiness messaging, or navigation semantics while adding the operator surface.

- [ ] Define a typed operator-summary contract and service instead of assembling summary data ad hoc in the UI. (AC: 1)
  - [ ] Add shared TypeScript DTO/schema files for an operator summary request/response envelope keyed by `sessionId` and `manifestPath`.
  - [ ] Create an operator-summary service module that owns the Tauri `invoke(...)` call and Zod parsing for the summary response.
  - [ ] Keep React screens/selectors consuming typed summary data only; no operator component should call `invoke(...)` or parse raw JSON payloads directly.

- [ ] Implement host-side operator-summary aggregation from the authoritative session, timing, readiness, and log sources. (AC: 1)
  - [ ] Add a new Rust command that accepts the active session identity, validates it against the manifest, and returns one normalized summary payload.
  - [ ] Load session context from the current manifest and include the fields the operator needs immediately: `sessionId`, `sessionName`, active preset context, latest capture identity if present, and durable session paths only if they are already approved for operator use.
  - [ ] Load timing state from `manifest.timing` so the operator sees authoritative adjusted end-time truth, session type, and operator extension count without recalculating timing in React.
  - [ ] Reuse `CameraHost::get_readiness_snapshot(sessionId)` and the normalized error envelope so booth state and suggested operator action come from the same host-normalized truth that already drives customer-safe readiness handling.
  - [ ] Query the existing SQLite operational log tables for the most recent relevant lifecycle/intervention row for the active session and return bounded failure context such as `currentStage`, `recentFaultCategory`, `occurredAt`, `extensionStatus`, and `interventionOutcome` when available.

- [ ] Render a bounded internal summary view that stays separate from customer UI and later recovery controls. (AC: 1)
  - [ ] Create an operator-console view model/selector that groups the summary into scannable sections such as session context, timing state, booth status, and recent failure context.
  - [ ] Reuse existing presentational primitives where practical, but keep operator-facing layout/content in a dedicated `operator-console` domain rather than expanding `customer-flow` or `shared-ui` with operator business logic.
  - [ ] Show operator-facing diagnostic fields such as normalized error code, retryability, and suggested operator action only on the operator route.
  - [ ] Do not add action buttons for retry/restart/extension in this story; Story 6.1 is visibility only, and Story 6.2 owns bounded recovery execution.

- [ ] Add regression coverage for the new operator summary path and keep customer surfaces diagnostics-free. (AC: 1)
  - [ ] Add contract/schema tests for operator summary request/response parsing, including active-session and no-active-session shapes.
  - [ ] Add service/unit tests proving the operator summary client parses success/failure envelopes and preserves typed diagnostic fields.
  - [ ] Add route/integration coverage for `/operator`, including the empty state when no active session exists and the summary state when an active session is present.
  - [ ] Add Rust tests for summary aggregation so manifest identity validation, latest-failure lookup ordering, and normalized readiness projection remain stable.

## Dev Notes

### Developer Context

- Epic 6 is the first operator-focused epic in the rewritten plan, and the repo currently has an asymmetry:
  - the customer flow is already implemented across `customer-flow`, `session-domain`, `capture-adapter`, and `timing-policy`
  - the architecture expects a separate `operator-console` domain and `/operator` route
  - the current app still exposes only `/customer`
- Current implementation ingredients already exist:
  - active session identity is created and stored in `SessionFlowProvider`
  - manifest timing is authoritative through `timing.actualShootEndAt`
  - the host already normalizes booth readiness and operator-facing error/action hints through `CameraHost::get_readiness_snapshot(...)`
  - lifecycle and intervention records already persist into SQLite through `session_events` and `operator_interventions`
- Current gap:
  - there is no read-side summary contract that aggregates session context, timing truth, booth state, and recent failure context for internal operators
  - there is no `src/operator-console` domain yet
  - `SessionFlowProvider` is mounted under the customer entry flow, so a sibling operator route cannot currently reuse active session identity
- Important implementation consequence:
  - `captureConfidence` is not sufficient as the operator summary source because it is capture-phase specific and absent during earlier phases such as `preparing` and `preset-selection`
  - Story 6.1 therefore needs a host-read summary shape that can load from the manifest and readiness/log sources even before capture begins
- Scope boundary:
  - in scope: operator summary visibility, active-session/timing/failure context aggregation, operator route scaffolding, and typed contracts
  - out of scope: bounded recovery execution buttons, fault classification taxonomy changes, audit-history browsing, rollout controls, or authoring access

### Technical Requirements

- Define one typed summary contract end-to-end.
  - Request payload should be keyed by the active session identity already known to the app (`sessionId`, `manifestPath`).
  - Response should distinguish between `no-active-session` and `active-session` states rather than overloading host failures for the empty-state case.
- Keep session identity authoritative and explicit.
  - The operator route should reuse the active session identity already created by `start_session`; do not guess the "current" session by scanning folders or choosing the newest manifest on disk.
  - If the operator route is opened before a session exists in the current runtime, show a bounded empty state instead of manufacturing a session summary from stale artifacts.
- Load session context from the manifest, not from UI memory alone.
  - Summary context should be based on manifest fields such as `sessionId`, `sessionName`, `activePresetName`, `activePreset`, `latestCaptureId`, and capture inventory as needed.
  - Do not make `captureConfidence` the only source for active preset or timing because it is not guaranteed to exist in pre-capture phases.
- Load timing from `manifest.timing`.
  - `timing.actualShootEndAt` remains the authoritative adjusted end time.
  - Include `sessionType`, `operatorExtensionCount`, and `lastTimingUpdateAt` so the operator can tell whether timing has already been extended.
  - Do not calculate booth timing locally in React.
- Prefer live normalized readiness context when a fault is active.
  - Use `CameraHost::get_readiness_snapshot(sessionId)` so operator summary status and recommended action come from the same normalized host truth already used in customer readiness handling.
  - If `readiness.error` exists, that error envelope should be the highest-signal live failure context.
  - If no live readiness error exists, fall back to the most recent relevant persisted lifecycle/intervention record.
- Query recent failure context from the existing SQLite schema before adding migrations.
  - The current tables already store `current_stage`, `actual_shoot_end_at`, `extension_status`, `recent_fault_category`, and `intervention_outcome`.
  - Story 6.1 should reuse those read-side fields first; do not add a migration unless an actual gap is proven during implementation.
- Keep diagnostic output bounded.
  - Operator summary may show normalized `code`, `severity`, `retryable`, `operatorAction`, and recent-failure metadata.
  - Do not dump raw `payload_json`, unbounded sidecar traces, stack traces, or arbitrary filesystem contents into the UI.
- Keep summary loading request/response-based for this story.
  - A one-shot command on route entry and active-session change is enough for Story 6.1.
  - Live streaming or continuously updating operator diagnostics channels can be introduced later if Story 6.2+ needs them.

### Architecture Compliance

- Keep the Rust host as the normalization boundary for operator-visible booth truth.
  - Session context comes from the manifest/session repository.
  - Readiness and suggested operator action come from the host-normalized readiness snapshot.
  - Recent failure context comes from the operational log store.
  - React should render the returned summary, not reconstruct it from raw host fragments.
- Preserve top-level route discipline.
  - Add `/operator` as a top-level route only.
  - Do not introduce nested workflow routes for booth phases or operator sub-phases.
- Maintain strict surface separation.
  - Customer routes remain diagnostics-free.
  - Operator-only summary content lives under `operator-console`.
  - Do not place operator business logic inside `customer-flow`, `shared-ui`, or branch-config modules.
- Keep commands/services as the only frontend-to-host boundary.
  - React components must not call Tauri directly.
  - The operator screen should depend on a typed service module and selector/view-model layer.
- Preserve durable truth ownership.
  - Session folders/manifests remain the durable source for active-session data.
  - SQLite remains the durable source for lifecycle/intervention history.
  - UI route state and component state must not become a new authority for operator diagnostics.
- Keep Story 6.1 separated from later Epic 6 work.
  - Story 6.1 = summary visibility
  - Story 6.2 = bounded recovery actions
  - Story 6.3 = normalized fault classification and customer-safe routing
  - Story 6.4 = lifecycle/intervention audit logging and query depth

### Library / Framework Requirements

- Current workspace baselines in this repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
  - `rusqlite`: `0.38.0`
- Official-source verification performed on 2026-03-13:
  - React 19.2 official docs continue to position `useEffectEvent` as the correct tool for effect-driven logic that needs fresh state reads without broadening dependencies. If the operator screen refreshes on session identity changes or route transitions, keep that logic in effect/service boundaries rather than ad hoc event buses. [Source: https://react.dev/blog/2025/10/01/react-19-2] [Source: https://react.dev/reference/react/useEffectEvent]
  - React Router's official changelog is newer than the repo's pinned `7.9.4`; this story should stay route-stable and add `/operator` without taking on a router upgrade. [Source: https://reactrouter.com/changelog]
  - Tauri v2 official docs continue to position Rust commands as the standard request/response boundary and `State` as the supported pattern for managed application state. Story 6.1 should therefore use a typed command over direct frontend database/file reads. [Source: https://v2.tauri.app/develop/calling-rust/] [Source: https://v2.tauri.app/es/develop/state-management/]
  - Tauri's frontend communication docs continue to distinguish request/response commands from longer-lived event/channel patterns. A request/response summary load is the appropriate fit for Story 6.1. [Source: https://v2.tauri.app/develop/calling-frontend/]
  - Zod 4 remains the active official documentation line and should continue to validate the operator summary contract at the TypeScript boundary. [Source: https://zod.dev/v4]

### File Structure Requirements

- Primary frontend seams likely to change:
  - `src/App.tsx`
  - `src/customer-flow/screens/CustomerEntryScreen.tsx`
  - `src/session-domain/state/SessionFlowProvider.tsx`
- New frontend domain files likely needed:
  - `src/operator-console/screens/OperatorSummaryScreen.tsx`
  - `src/operator-console/selectors/operatorSummaryView.ts`
  - `src/operator-console/services/operatorSummaryService.ts`
  - `src/operator-console/services/operatorSummaryService.spec.ts`
- Shared contract files likely needed:
  - `src/shared-contracts/dto/operatorSummary.ts`
  - `src/shared-contracts/schemas/operatorSummarySchemas.ts`
  - `src/shared-contracts/index.ts` if exports are centralized there for new contract reuse
- Existing frontend files that should be reused, not replaced:
  - `src/diagnostics-log/services/operationalLogClient.ts`
  - `src/shared-contracts/dto/sessionManifest.ts`
  - `src/shared-contracts/dto/sessionTiming.ts`
  - `src/shared-contracts/dto/cameraStatus.ts`
  - `src/shared-contracts/dto/errorEnvelope.ts`
  - `src/shared-ui/components/HardFramePanel.tsx`
  - `src/branch-config/BranchConfigProvider.tsx`
- Primary Rust/host seams likely to change:
  - `src-tauri/src/commands/operator_commands.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/lib.rs`
- New Rust read-model/support files likely needed:
  - `src-tauri/src/diagnostics/operator_summary.rs` or an equivalently named diagnostics query module
- Existing Rust files that should be reused, not replaced:
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/capture/camera_host.rs`
  - `src-tauri/src/db/sqlite.rs`
  - `src-tauri/src/diagnostics/lifecycle_log.rs`
  - `src-tauri/src/diagnostics/operator_log.rs`
  - `src-tauri/migrations/0001_init.sql`
- Guardrail:
  - avoid introducing a new database migration unless summary requirements cannot be satisfied from the existing lifecycle/intervention schema
  - avoid placing operator summary logic under `customer-flow`, `capture-adapter`, or `shared-ui`

### Testing Requirements

- Add TypeScript contract coverage for:
  - operator summary request validation
  - active-session summary response validation
  - no-active-session summary response validation
  - rejection of malformed diagnostic payloads
- Add frontend service tests proving:
  - the operator summary service calls the typed Tauri command
  - success payloads are Zod-validated before reaching the UI
  - failure/empty-state envelopes remain distinguishable
- Add route/UI tests proving:
  - `/operator` renders without breaking the existing `/customer` default redirect
  - no-active-session operator view shows a bounded internal empty state
  - active-session operator view shows session context, timing state, and recent failure context
  - customer route tests continue to confirm diagnostics do not leak into customer copy or components
- Add Rust tests proving:
  - the summary command rejects a mismatched `sessionId` / `manifestPath`
  - the latest relevant failure/intervention row is selected by occurrence time
  - a live readiness error envelope overrides stale persisted failure context when both are available
  - the no-active-session response does not require creating fake session artifacts

### Previous Story Intelligence

- There is no earlier Epic 6 implementation-artifact story yet.
- The most relevant predecessor intelligence comes from already-established repo seams:
  - `SessionFlowProvider` already owns active session identity and customer-phase orchestration
  - `CameraHost` already exposes normalized readiness snapshots with operator-facing action hints
  - the operational log schema and tests already protect lifecycle/intervention persistence and query ordering
- Treat those seams as the baseline to extend rather than creating a parallel operator-only state model.

### Git Intelligence Summary

- Recent git history is still dominated by the March 2026 greenfield reset and camera/readiness stabilization work, not by a completed operator-console implementation.
- Actionable guidance from the current branch:
  - build on the existing normalized readiness and operational log foundations
  - avoid reworking customer-flow architecture just to expose operator data
  - keep operator summary additive and boundary-preserving so later recovery stories can layer on top cleanly

### Latest Tech Information

Verified against official docs on 2026-03-13:

- React 19.2 continues to document `useEffectEvent` for effect-driven logic that needs fresh state reads without widening dependencies. If operator summary loading reacts to active-session changes, keep that orchestration in provider/service effects rather than pushing mutable callbacks through the UI tree. [Source: https://react.dev/blog/2025/10/01/react-19-2] [Source: https://react.dev/reference/react/useEffectEvent]
- React Router's current official changelog is already ahead of the repo's pinned `7.9.4`, but this story should remain route-stable and avoid router upgrades unrelated to introducing `/operator`. [Source: https://reactrouter.com/changelog]
- Tauri v2 official docs continue to describe Rust commands as the standard frontend-to-host request/response boundary and managed `State` as the supported app-state pattern. Story 6.1 should use those patterns rather than direct frontend access to SQLite or manifests. [Source: https://v2.tauri.app/develop/calling-rust/] [Source: https://v2.tauri.app/es/develop/state-management/]
- Tauri's frontend communication docs continue to distinguish streaming/event mechanisms from one-shot calls. Story 6.1 is a one-shot summary read, so a typed command is the correct fit. [Source: https://v2.tauri.app/develop/calling-frontend/]
- Zod 4 remains the current official docs line and should continue to validate all new operator summary payloads before UI consumption. [Source: https://zod.dev/v4]

### Project Structure Notes

- The current repo already has strong domain seams for customer/session/timing/capture work, but it is missing the architecture-planned `operator-console` domain.
- Story 6.1 should close that specific gap without flattening the codebase into generic "admin" or "shared" folders.
- The highest-value structural move is likely lifting `SessionFlowProvider` so `/customer` and `/operator` can observe the same active session identity during one runtime, while still keeping operator summary loading behind a dedicated service and host command.
- Because the operator route is new, this story should also define the first empty-state behavior for "no active session" instead of assuming the summary always has a session.

### Project Context Reference

- `_bmad-output/project-context.md` remains active guidance for this story.
- Highest-signal rules from that file for Story 6.1:
  - keep React components away from direct Tauri invocation
  - preserve session manifests/folders as durable session truth
  - validate cross-boundary DTOs with Zod and keep them fully typed
  - keep routes limited to top-level surfaces
  - do not introduce branch-specific shortcuts or diagnostics leaks on customer-visible screens

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/App.tsx`
- `src/App.spec.tsx`
- `src/customer-flow/screens/CustomerEntryScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/shared-contracts/dto/sessionTiming.ts`
- `src/shared-contracts/dto/cameraStatus.ts`
- `src/shared-contracts/dto/captureConfidence.ts`
- `src/shared-contracts/dto/errorEnvelope.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/capture-adapter/host/cameraErrorMapping.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/operator_commands.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/db/sqlite.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/diagnostics/operator_log.rs`
- `src-tauri/tests/operational_log_foundation.rs`
- `src-tauri/migrations/0001_init.sql`
- `tests/contract/operationalLogSchemas.test.ts`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- React `useEffectEvent` reference: https://react.dev/reference/react/useEffectEvent
- React Router changelog: https://reactrouter.com/changelog
- Tauri calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri state management docs: https://v2.tauri.app/es/develop/state-management/
- Tauri frontend communication docs: https://v2.tauri.app/develop/calling-frontend/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Primary implementation goal: surface a bounded operator summary that exposes current session, timing truth, booth status, and recent failure context without leaking diagnostics to customer routes
- Reuse strategy: build on the current session manifest, readiness snapshot, timing contract, and operational log foundations rather than introducing new truth stores
- Contract sensitivity: high because the story introduces a new cross-boundary operator summary DTO and the first operator route
- Key guardrail: do not expand this story into recovery-action execution or broader diagnostics history

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation must be performed manually

### Completion Notes List

- Story context was generated from the refreshed Epic 6.1 requirement, current repo session/timing/readiness/log seams, recent git history, and official React / React Router / Tauri / Zod documentation verified on 2026-03-13.
- The story intentionally treats active-session identity as an existing runtime concern and avoids inventing filesystem-scanning heuristics for "current session" discovery.
- The document explicitly keeps operator recovery actions, fault classification, and broader audit browsing out of scope so implementation does not bleed into Stories 6.2-6.4.
- The story assumes the operator summary can reuse the current SQLite lifecycle/intervention schema without requiring a migration unless implementation proves a concrete gap.

### File List

- `_bmad-output/implementation-artifacts/6-1-operator-summary-and-session-context-visibility.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
