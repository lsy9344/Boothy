# Story 6.2: Bounded Operator Recovery Actions

Status: ready-for-dev

Story Key: `6-2-bounded-operator-recovery-actions`

## Summary

Introduce the first bounded operator recovery surface that reuses the repository's existing timing-extension, camera-readiness, normalized-error, and diagnostics seams so an operator can execute only approved recovery actions with explicit outcome and risk labeling, while the customer surface stays free of diagnostics and unresolved cases fall back to safe wait/call guidance.

## Story

As an operator,
I want a limited set of recovery actions,
so that I can restore safe operation without unsafe interventions.

## Acceptance Criteria

1. Given a blocked or fault state is detected, when the operator views recovery actions, then only approved bounded actions are available, and each action is labeled with expected outcome and risk.
2. Given an operator executes a recovery action, when it completes, then the system records the intervention event, and the booth returns to a safe customer state or a wait/call state.

## Tasks / Subtasks

- [ ] Create an operator-only recovery surface and typed action view model. (AC: 1)
  - [ ] Add a top-level operator entry surface separate from `/customer`; do not place recovery buttons inside booth customer screens.
  - [ ] Define a typed recovery-action contract/view model that includes a stable action id, customer-safe result target, expected outcome, risk level, availability, and disabled reason.
  - [ ] Build action eligibility from current session context, timing state, and host-normalized error/recovery hints rather than free-form UI booleans or local string matching.

- [ ] Reuse existing typed host seams for currently approved recovery actions. (AC: 1, 2)
  - [ ] Keep session-time extension routed through `sessionTimingService.extendSessionTiming()` and the existing Rust `extend_session_timing` command instead of duplicating timing math in React.
  - [ ] Reuse `camera_run_readiness_flow` or a small typed adapter wrapper for the "check cable and retry" readiness recovery path instead of adding a second readiness orchestration flow.
  - [ ] If `restartHelper` is exposed, add it as a bounded host command owned by `operator_commands.rs` and the camera-host layer; do not call plugin-shell or arbitrary process controls from React.

- [ ] Enforce the bounded action catalog and explicit risk/outcome labeling. (AC: 1)
  - [ ] Limit the operator-visible catalog to approved actions only: retry readiness/check cable, helper restart, timing extension, and escalation/contact support unless a later approved artifact changes the catalog.
  - [ ] Attach human-readable expected-outcome and risk copy to every action and keep that copy out of the customer surface.
  - [ ] Do not expose raw helper commands, filesystem repair, manifest editing, SQL access, or any other unbounded controls under the recovery UI.

- [ ] Route action results back into a safe booth state. (AC: 2)
  - [ ] After each action, refresh readiness and/or timing snapshots through typed adapters so the customer surface reflects host truth rather than optimistic UI guesses.
  - [ ] On success, return the booth to a safe `preparing` or `ready` path as appropriate.
  - [ ] On failure or unresolved conditions, preserve bounded wait/call guidance and do not leak operator diagnostics into booth copy.

- [ ] Record every operator intervention through the existing diagnostics boundary. (AC: 2)
  - [ ] Reuse `recordOperatorIntervention()` and the Rust `record_operator_intervention` path instead of creating a second audit channel.
  - [ ] Standardize allowed `interventionOutcome` values in one shared constant or enum-like contract so the operator UI does not log arbitrary prose.
  - [ ] Include current stage, branch/session identity, extension status when applicable, and recent fault category if available.

- [ ] Add regression coverage for bounded actions, capability isolation, and safe outcomes. (AC: 1, 2)
  - [ ] Add frontend tests proving only approved actions render for a given fault/timing context and each action shows expected outcome and risk labeling.
  - [ ] Add frontend/service tests proving action execution always goes through typed adapters and records intervention results.
  - [ ] Add Rust tests proving new operator commands stay bounded, persist intervention logs, and leave booth/customer recovery in safe states.

## Dev Notes

### Developer Context

- Epic 6 / FR-009 is about bounded operator recovery, not a general-purpose admin console. Story 6.2 should add approved recovery controls without weakening the customer/operator boundary defined in the PRD and architecture.
- Current repo reality already provides important foundations this story should extend rather than replace:
  - `src/App.tsx` exposes only the `/customer` route today; no operator surface exists yet.
  - `src/shared-contracts/dto/errorEnvelope.ts`, `src/shared-contracts/dto/cameraErrorContract.ts`, and `src/capture-adapter/host/cameraErrorMapping.ts` already carry operator-facing recovery hints such as `operatorCameraConnectionState` and `operatorAction`.
  - `src/timing-policy/services/sessionTimingService.ts`, `src/shared-contracts/schemas/sessionTimingSchemas.ts`, `src-tauri/src/commands/operator_commands.rs`, and `src-tauri/src/session/session_repository.rs` already implement typed session-time extension.
  - `src/diagnostics-log/services/operationalLogClient.ts`, `src/shared-contracts/logging/operationalEvents.ts`, `src-tauri/src/diagnostics/operator_log.rs`, and `src-tauri/tests/operational_log_foundation.rs` already persist operator interventions to SQLite.
  - `src/capture-adapter/host/cameraAdapter.ts`, `src/capture-adapter/host/cameraCommands.ts`, `src/shared-contracts/dto/cameraContract.ts`, `src-tauri/src/commands/capture_commands.rs`, and `src-tauri/src/capture/camera_host.rs` already provide the typed readiness-flow path that should anchor a retry/check-cable action.
- The main gap is orchestration, not raw plumbing:
  - there is no operator-console domain
  - there is no top-level operator route/window
  - there is no typed action catalog with bounded labels and risk metadata
  - there is no bounded helper-restart command yet
  - there is no action-result flow that returns the booth to safe customer guidance
- Dependency note:
  - No refreshed Story 6.1 implementation artifact exists yet in `implementation-artifacts`.
  - Do not invent a second session-summary model for Story 6.2. Reuse the current active session, timing, readiness, and normalized error seams as the minimum decision context, and leave richer normalized fault taxonomy to Story 6.3.
- Planning nuance from readiness validation:
  - `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md` explicitly called out "exact bounded operator recovery actions" as still ambiguous.
  - Story 6.2 should remove that ambiguity by defining the approved action catalog in code and tests, not by leaving room for developer improvisation.

### Technical Requirements

- Recovery actions must be typed and bounded. Each action record should include:
  - stable action id
  - label
  - expected outcome
  - risk level
  - availability / disabled reason
  - audit outcome token
  - resulting customer-safe state target
- Use the current normalized error and timing seams as the action-input baseline:
  - `NormalizedErrorEnvelope.operatorAction`
  - `NormalizedErrorEnvelope.operatorCameraConnectionState`
  - current readiness snapshot / last safe customer state
  - session timing state including `operatorExtensionCount`
- Until Story 6.3 lands, do not invent a second raw fault taxonomy in the UI. If additional operator categorization is needed, keep it a thin mapping layer over the existing normalized host envelope and readiness state.
- "Retry" must re-run the host readiness flow rather than toggling local UI flags or fabricating a recovered state in React memory.
- "Extend session timing" must preserve manifest truth and the existing `operatorExtensionCount` / timing snapshot behavior. Do not duplicate extension math or write the manifest from the frontend.
- If a helper-restart action is added, bound it to the packaged camera helper only. No arbitrary executable paths, shell text commands, generic process kill/restart utilities, or unrestricted sidecar control are allowed.
- Escalation/contact-support remains a bounded recovery result, not a no-op button:
  - record the operator intervention
  - preserve or transition the customer booth into wait/call-safe guidance
  - keep the next action operator-visible without leaking diagnostics to the customer
- Logging requirements:
  - continue using the existing operational context fields and sensitive-field guards in `operationalEvents.ts`
  - standardize `interventionOutcome` values in one shared contract or helper instead of allowing free-text drift
  - include branch id, session id/session name when available, current stage, extension status if relevant, and recent fault category if available
- Every recovery action must present expected outcome and risk copy to the operator. Use a bounded risk vocabulary such as `low`, `medium`, `high` rather than ad hoc prose if a new DTO is added.
- No recovery action may bypass typed host services, mutate unrelated session files, or expose cross-session information while resolving the current booth problem.

### Architecture Compliance

- Keep the operator surface separate from customer UI structure and capability boundaries. Do not place recovery controls under `/customer` or hide them behind conditional rendering inside customer screens.
- Introduce an `operator-console` domain consistent with the architecture document instead of burying recovery logic inside `customer-flow`, `shared-ui`, or `diagnostics-log`.
- React components must not call Tauri directly. The operator UI should talk to typed adapters/services, and those modules should own `invoke`, channel subscriptions, and host orchestration.
- Keep routing limited to top-level surfaces. If a route is added, use a top-level `/operator` entry and keep recovery workflow truth in state/services, not nested route state.
- Tauri permissions/capabilities must enforce operator-only access for any new recovery commands. The current single `src-tauri/capabilities/default.json` capability is too coarse if new privileged commands are added; do not assume route-level hiding is a sufficient safety boundary.
- Preserve host authority and session-folder truth. Recovery results must be confirmed by host-managed readiness/timing state before the booth UI claims recovery succeeded.
- Customer-visible state translation remains centralized in customer selectors/copy modules. Operator diagnostics, risk labels, and recovery metadata must not leak into booth-facing copy.

### Library / Framework Requirements

- Current workspace baselines in the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - `zod`: `^4.3.6`
  - Rust `tauri`: `2.10.3`
  - `rusqlite`: `0.38.0`
  - `chrono`: `0.4.42`
- Official-source verification performed on 2026-03-13:
  - React official docs continue to position `useEffectEvent` as the correct tool for effect-driven listeners/timers that must read the latest state without resubscribing; keep operator refresh/poll coordination in provider/service effects rather than render callbacks.
  - React Router's official BrowserRouter docs continue to fit a top-level surface-entry role; use routing for `/operator` entry only, not as the source of truth for recovery workflow progression.
  - Tauri v2 official security guidance continues to rely on capabilities/permissions attached to specific windows or webviews; operator recovery commands therefore need explicit capability separation and must not be exposed solely by hidden buttons in the customer surface.
  - Zod 4 official docs remain the stable baseline and support strict object validation; use strict schemas for new operator-action DTOs, restart-helper payloads, and intervention outcome payloads.
- Do not add a new global store, command bus, or persistence library for this story. The existing reducer/context plus typed adapter split is sufficient.

### File Structure Requirements

- Expected new frontend domains/surfaces:
  - `src/operator-console/screens/OperatorRecoveryActionsScreen.tsx`
  - `src/operator-console/components/*`
  - `src/operator-console/selectors/recoveryActionView.ts`
  - `src/operator-console/services/operatorRecoveryService.ts`
  - `src/operator-console/services/operatorActionCatalog.ts`
- Existing frontend files likely to inspect or update:
  - `src/App.tsx`
  - `src/diagnostics-log/services/operationalLogClient.ts`
  - `src/diagnostics-log/services/operationalLogContext.ts`
  - `src/capture-adapter/host/cameraAdapter.ts`
  - `src/capture-adapter/host/cameraCommands.ts`
  - `src/shared-contracts/dto/errorEnvelope.ts`
  - `src/shared-contracts/dto/cameraErrorContract.ts`
  - `src/shared-contracts/logging/operationalEvents.ts`
  - `src/shared-contracts/schemas/sessionTimingSchemas.ts`
  - `src/timing-policy/services/sessionTimingService.ts`
  - `src/session-domain/state/SessionFlowProvider.tsx` only if shared timing/readiness refresh orchestration must be reused; do not move operator logic into the customer reducer
- Host files likely relevant:
  - `src-tauri/src/commands/operator_commands.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/capture/camera_host.rs`
  - `src-tauri/src/capture/sidecar_client.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/diagnostics/operator_log.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/capabilities/default.json`
  - new `src-tauri/capabilities/operator.json` or equivalent capability files if operator-only commands are introduced
- Test surfaces likely needing additions:
  - `src/diagnostics-log/services/operationalLogClient.spec.ts`
  - `src/timing-policy/services/sessionTimingService.spec.ts`
  - new `src/operator-console/**/*.spec.tsx`
  - new `tests/integration/operatorRecoveryFlow.test.tsx`
  - `src-tauri/tests/operational_log_foundation.rs`
  - `src-tauri/tests/camera_contract.rs`
  - new Rust operator-command tests
- Keep bounded recovery logic out of:
  - `src/shared-ui/*` beyond presentation-only primitives
  - `src/customer-flow/*` except customer-safe status consumption
  - ad hoc utility folders that blur the operator/customer boundary

### Testing Requirements

- Add frontend integration coverage proving:
  - only approved bounded actions render for a blocked/fault context
  - each action shows expected outcome and risk labeling
  - unavailable actions explain why they are disabled
  - successful actions refresh the booth state from typed host snapshots
- Add service/adapter tests proving:
  - retry uses `camera_run_readiness_flow`
  - time extension uses `extend_session_timing`
  - every success/failure path records an intervention through `recordOperatorIntervention`
  - customer-facing screens never gain operator recovery controls or raw diagnostics by side effect
- Add contract tests proving:
  - any new operator action DTOs remain strict and typed
  - standardized `interventionOutcome` values are constrained to the approved set
  - no sensitive fields bypass the existing operational event guards
- Add Rust tests proving:
  - helper restart, if introduced, is bounded to the known camera-helper path and rejects malformed or unauthorized requests
  - intervention rows persist with session/timing context
  - safe unresolved outcomes remain compatible with customer wait/call guidance rather than raw host error text

### Previous Story Intelligence

- No refreshed Epic 6 predecessor story artifact exists yet in `implementation-artifacts`.
- The practical predecessors are the repo seams already in place:
  - normalized host errors already carry operator-facing action hints
  - session-time extension already exists end to end
  - operational intervention logging already persists to SQLite
- Treat those seams as the canonical baseline. Story 6.2 should compose them into one bounded recovery workflow instead of replacing them with a second diagnostics stack.

### Git Intelligence Summary

- Recent history remains dominated by the greenfield reset and camera-state stabilization work:
  - `06ed2b7` restructured the repository around the refreshed BMAD planning baseline.
  - Earlier commits continue the same pattern: customer UI should stay derived from host-normalized truth rather than local assumptions.
- Actionable guidance for Story 6.2:
  - keep recovery actions derived from host-normalized readiness/timing truth
  - keep privileged behavior in the Rust/operator boundary
  - avoid ad hoc UI-only fault logic or unrestricted process control

### Latest Tech Information

Verified against official docs on 2026-03-13:

- React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
- React Router BrowserRouter: https://reactrouter.com/api/declarative-routers/BrowserRouter
- Tauri v2 permissions/capabilities: https://v2.tauri.app/security/capabilities/ and https://v2.tauri.app/security/permissions/
- Zod 4 docs: https://zod.dev/v4

### Project Structure Notes

- The current repo only exposes the customer route and has no `operator-console` domain yet. Story 6.2 should introduce that domain intentionally instead of scattering operator logic across customer files.
- `src-tauri/capabilities/default.json` currently grants one coarse capability to the `main` window with `core:default`, `shell:default`, and `store:default`. If new recovery commands are added, narrow the command surface with operator-specific capability configuration instead of treating the current default capability as sufficient.
- The most relevant rules from `_bmad-output/project-context.md` for this story are:
  - keep React components away from direct Tauri invocation
  - preserve session folders and host state as the durable source of truth
  - keep the operator surface separate from the customer surface in both UI structure and capability boundaries
  - avoid exposing raw diagnostics or internal helper details to the customer UI
  - keep shared DTOs fully typed and validated at the TypeScript boundary

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
- `src/App.tsx`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/capture-adapter/host/cameraCommands.ts`
- `src/capture-adapter/host/cameraErrorMapping.ts`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src/diagnostics-log/services/operationalLogContext.ts`
- `src/shared-contracts/dto/errorEnvelope.ts`
- `src/shared-contracts/dto/cameraErrorContract.ts`
- `src/shared-contracts/dto/cameraContract.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src/shared-contracts/schemas/sessionTimingSchemas.ts`
- `src/timing-policy/services/sessionTimingService.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src-tauri/src/commands/operator_commands.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/diagnostics/operator_log.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/default.json`
- `src-tauri/tests/operational_log_foundation.rs`
- `src-tauri/tests/camera_contract.rs`
- React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
- React Router BrowserRouter: https://reactrouter.com/api/declarative-routers/BrowserRouter
- Tauri capabilities: https://v2.tauri.app/security/capabilities/
- Tauri permissions: https://v2.tauri.app/security/permissions/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Scope: bounded operator action catalog, safe recovery execution, intervention logging, and capability-aware operator isolation
- Reuse strategy: extend the existing timing, readiness, normalized-error, and diagnostics seams instead of creating a parallel operator state stack
- Contract sensitivity: high because this story touches privileged commands, operator logging, timing truth, and customer/operator boundary safety
- Key guardrail: do not hide unbounded privileged operations behind the label of "operator recovery"

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation was performed manually

### Completion Notes List

- Story context was generated from the refreshed Epic 6.2 requirement, the current timing/readiness/diagnostics implementation seams, readiness and validation artifacts dated 2026-03-12, recent git history, and official React / React Router / Tauri / Zod documentation verified on 2026-03-13.
- The document intentionally treats existing `operatorAction` error-hint contracts, `extend_session_timing`, and operator intervention logging as canonical reuse seams so the dev agent does not reinvent recovery infrastructure.
- The story explicitly calls out the current capability-model gap: operator-only recovery commands need stronger isolation than the existing single `default.json` capability provides.

### File List

- `_bmad-output/implementation-artifacts/6-2-bounded-operator-recovery-actions.md`
