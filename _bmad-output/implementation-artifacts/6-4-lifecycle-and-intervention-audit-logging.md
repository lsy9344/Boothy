# Story 6.4: Lifecycle and Intervention Audit Logging

Status: ready-for-dev

Story Key: `6-4-lifecycle-and-intervention-audit-logging`

## Summary

Extend the existing SQLite-backed operational logging foundation into a correlation-aware audit system that records lifecycle transitions and bounded operator interventions through typed contracts, exposes recent event history through a host-owned query boundary, and makes that history available on a separate operator surface without leaking diagnostics into the customer flow.

## Story

As an operator,
I want lifecycle and intervention events logged,
so that operational issues can be reviewed and improved.

## Acceptance Criteria

1. Given a lifecycle transition or operator intervention occurs, when the event is processed, then it is written to the audit log with correlation identifiers.
2. Given a lifecycle transition or operator intervention occurs, when the event is processed, then it can be queried in the operator console.

## Tasks / Subtasks

- [ ] Promote the operational log contract from write-only events to correlation-aware audit records. (AC: 1, 2)
  - [ ] Extend `src/shared-contracts/logging/operationalEvents.ts` and the matching Rust DTOs so new audit writes include a required `correlationId` in addition to the existing branch/session context.
  - [ ] Reuse the repo's existing request/correlation convention from camera and sidecar contracts instead of inventing a second identifier format; carry optional source identifiers such as `requestId` or `captureId` in `payload_json` only when a producer already has them.
  - [ ] Keep lifecycle events and operator interventions as separate typed write paths, but define one normalized read model for operator-facing audit history.

- [ ] Add the host-side persistence and query boundary for audit history. (AC: 1, 2)
  - [ ] Add a forward-only SQLite migration that preserves existing rows while adding correlation-aware columns and query-friendly indexes for recent-session and recent-branch history.
  - [ ] Introduce a Rust diagnostics repository/query module that reads `session_events` and `operator_interventions` into one typed audit result set ordered by occurrence time.
  - [ ] Expose a Tauri command for audit queries instead of letting the frontend read SQLite or log files directly.

- [ ] Expand lifecycle and intervention producers through centralized logging services. (AC: 1)
  - [ ] Keep `session_created` logging host-owned in `session_commands.rs`, but update its payload to use the new correlation-aware contract.
  - [ ] Expand the frontend logging service layer beyond `readiness_reached` so current and upcoming lifecycle producers have explicit helpers for `warning_shown`, `actual_shoot_end`, `export_state_changed`, `session_completed`, and `phone_required`.
  - [ ] Record the currently implemented operator recovery action in `src-tauri/src/commands/operator_commands.rs` as an operator intervention entry with outcome, timing context, and correlation identifier.

- [ ] Add a minimal operator-facing audit surface on the separate operator boundary. (AC: 2)
  - [ ] Add a top-level `/operator` route and keep it separate from `/customer`; do not mix operator diagnostics into the booth customer flow.
  - [ ] Create an operator-console screen/component that can request recent audit history for a session or branch and render lifecycle/intervention entries with safe internal labels.
  - [ ] Support recent-event context that shows event type, occurred time, current stage, session identity, and intervention outcome when applicable.

- [ ] Preserve customer-safe behavior and bounded diagnostics separation. (AC: 1, 2)
  - [ ] Keep audit-query features and diagnostic labels off customer screens, customer selectors, and booth copy modules.
  - [ ] Ensure logging failures remain silent to the customer path and do not block capture, timing, or guidance screens.
  - [ ] Keep operator-visible query payloads limited to approved operational context; do not log or expose disallowed sensitive fields such as phone numbers, payment data, or raw reservation payloads.

- [ ] Add regression coverage for correlation-aware writes and operator queries. (AC: 1, 2)
  - [ ] Extend contract tests for write schemas and add new tests for the operator-facing read/query contract.
  - [ ] Extend Rust persistence tests to cover migration, correlation-aware inserts, and ordered mixed lifecycle/intervention queries.
  - [ ] Add frontend tests for the operator audit client and operator screen so the query path stays typed and isolated from the customer route.

## Dev Notes

### Developer Context

- Epic 6 is the operator and diagnostics epic for FR-009. The March 12, 2026 sprint change proposal explicitly repositions logging as a platform capability that should exist before richer operator workflows depend on it.
- The current repo already has a partial operational logging baseline:
  - `src-tauri/migrations/0001_init.sql` creates `session_events` and `operator_interventions`.
  - `src-tauri/src/db/sqlite.rs` initializes `operational-log.sqlite3` under the Tauri app-local data directory with WAL mode, foreign keys, and a busy timeout.
  - `src/shared-contracts/logging/operationalEvents.ts` defines the current write schemas for lifecycle events and operator interventions.
  - `src-tauri/src/diagnostics/lifecycle_log.rs` and `src-tauri/src/diagnostics/operator_log.rs` validate and persist those writes.
  - `src/diagnostics-log/services/operationalLogClient.ts` already provides typed write adapters for the frontend.
- The main gaps this story must close are:
  - the write schema has no explicit `correlationId`
  - there is no host-owned query/read contract for audit history
  - there is no `src/operator-console/` domain yet, and `src/App.tsx` only exposes `/customer`
  - `src/diagnostics-log/services/lifecycleLogger.ts` only covers `readiness_reached`
  - `src-tauri/src/commands/operator_commands.rs` extends timing but does not yet record that intervention into the audit store
- Important sequencing signal from planning:
  - `sprint-change-proposal-2026-03-12.md` says normalized fault context and logging baseline should be treated as available platform capabilities before operator workflows depend on them.
  - Build Story 6.4 as that reusable platform layer, not as a one-off screen hack.

### Technical Requirements

- Use one consistent audit-correlation model across TypeScript and Rust:
  - required `branchId`
  - required `correlationId`
  - session-scoped identifiers such as `sessionId` and `sessionName` when the event belongs to an active session
  - optional source metadata such as `requestId`, `captureId`, `extensionStatus`, or `recentFaultCategory` only when the producer already has them
- Reuse the existing correlation vocabulary already present in `src-tauri/src/contracts/dto.rs` for camera and sidecar traffic. Do not introduce a second incompatible identifier naming scheme.
- Preserve the existing privacy guardrails from `operationalEvents.ts` and `operational_log_foundation.rs`:
  - reject `fullPhoneNumber`
  - reject `paymentData`
  - reject `rawReservationPayload`
  - keep timestamps RFC3339/ISO-8601
- Keep the audit system host-owned:
  - frontend code may call typed query commands
  - frontend code must not open SQLite directly
  - frontend code must not read `events.ndjson` or session filesystem artifacts as a substitute for audit history
- The read/query result should support operator review use cases directly:
  - event kind or intervention marker
  - occurred time
  - branch id
  - correlation id
  - session id/session name if present
  - current stage
  - actual shoot end time if present
  - recent fault category if present
  - intervention outcome when applicable
- The logging layer must remain failure-tolerant for booth flow. A failed audit write or query must never break customer-safe rendering or stall operator recovery commands.

### Architecture Compliance

- Respect the architecture split between `customer` and `operator` surfaces. Story 6.4 should introduce or extend `src/operator-console/` rather than placing audit UI inside `customer-flow`.
- Keep React Router limited to top-level surfaces. Adding `/operator` is valid; using nested routing as workflow truth is not.
- Keep React components away from direct Tauri invocation. Operator UI should call typed services under `src/diagnostics-log/services/` or `src/operator-console/services/`.
- Preserve the domain-first structure described in architecture and project context:
  - `diagnostics-log` owns audit client/query logic
  - `operator-console` owns operator-facing presentation
  - `shared-contracts` owns DTO/schema definitions
  - Rust `diagnostics` and `db` modules own persistence and query execution
- Do not move audit truth into branch config, shared UI, or customer selectors.

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
- Keep Zod as the TypeScript boundary validator for both audit writes and audit-query results.
- Keep Tauri commands as the request/response boundary for operator audit queries; do not replace this with ad hoc events for query-style reads.
- Follow the repo's current React patterns:
  - use provider/service modules for host access
  - use `startTransition` and `useEffectEvent` where UI state coordination needs them
  - do not add a new global state library for diagnostics work

### File Structure Requirements

- Existing files that should almost certainly change:
  - `src/App.tsx`
  - `src/shared-contracts/logging/operationalEvents.ts`
  - `src/shared-contracts/index.ts`
  - `src/diagnostics-log/services/operationalLogClient.ts`
  - `src/diagnostics-log/services/lifecycleLogger.ts`
  - `src-tauri/migrations/0001_init.sql` should remain intact, but a new migration should follow it
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/diagnostics/lifecycle_log.rs`
  - `src-tauri/src/diagnostics/operator_log.rs`
  - `src-tauri/src/commands/operator_commands.rs`
- Likely new TypeScript files:
  - `src/shared-contracts/logging/operationalAuditQuery.ts`
  - `src/diagnostics-log/services/operationalLogQueryClient.ts`
  - `src/operator-console/screens/OperatorAuditScreen.tsx`
  - `src/operator-console/components/OperatorAuditTable.tsx`
  - `src/operator-console/screens/OperatorAuditScreen.spec.tsx`
- Likely new Rust files:
  - `src-tauri/src/commands/diagnostics_commands.rs`
  - `src-tauri/src/diagnostics/audit_query.rs`
  - `src-tauri/src/db/repositories/operational_log_repository.rs`
  - `src-tauri/tests/operational_log_query.rs`
  - `src-tauri/migrations/0002_audit_correlation_and_queries.sql` (or next available migration number)
- Keep audit-query UI out of:
  - `src/customer-flow/*`
  - `src/shared-ui/*` beyond presentation-only primitives
  - direct SQLite or filesystem access from frontend code

### Testing Requirements

- Extend contract coverage in `tests/contract/operationalLogSchemas.test.ts` for:
  - required `correlationId`
  - normalized read/query shapes
  - rejection of sensitive fields on both write and query-adjacent payloads
- Add frontend tests for:
  - typed query client success/failure normalization
  - `/operator` route rendering without affecting `/customer`
  - operator audit screen rendering mixed lifecycle/intervention rows in descending or explicitly selected order
- Extend Rust tests for:
  - migration safety on existing operational-log databases
  - correlation-aware lifecycle insert
  - correlation-aware operator intervention insert
  - mixed read queries that preserve ordering and session isolation
  - query filtering by session id and branch id
- Keep regression focus on compatibility-sensitive surfaces: shared DTOs, migration files, Tauri command payloads, and query result shapes.

### Previous Story Intelligence

- There is no refreshed Epic 6 predecessor story file yet in `_bmad-output/implementation-artifacts`, which is consistent with the planning note that logging baseline should stand on its own before later operator workflows depend on it.
- The most relevant existing precedent is the operational log foundation already in code:
  - session start writes `session_created` on the host in `src-tauri/src/commands/session_commands.rs`
  - readiness logging already uses a typed frontend service in `src/diagnostics-log/services/lifecycleLogger.ts`
  - operator timing extension exists as a bounded recovery action in `src-tauri/src/commands/operator_commands.rs`
- Treat those seams as the baseline. Story 6.4 should unify and extend them rather than replacing them with a parallel audit subsystem.

### Git Intelligence Summary

- Recent git history still reflects the greenfield reset and normalization push:
  - `06ed2b7` restructured the repository around the refreshed BMAD planning baseline.
  - `1fb8bb0`, `3ef405f`, and `cb12647` all reinforce the same design rule: normalize device/runtime truth centrally and keep UI consumers typed and bounded.
- Actionable guidance for Story 6.4:
  - keep audit writes and audit queries behind typed boundaries
  - avoid direct UI knowledge of persistence details
  - favor additive migrations and isolated diagnostics modules over ad hoc logging code scattered across unrelated features

### Latest Tech Information

- Official-source verification performed on 2026-03-13:
  - React official docs continue to document `useEffectEvent` as the right fit when event handlers or timers must read the latest state without forcing effect resubscription. That remains relevant for operator screen refresh triggers or polling controls if needed.
  - Tauri v2 official docs continue to position commands as the standard frontend-to-Rust request/response boundary, which matches this story's need for typed audit queries.
  - Zod 4 remains the current official docs line and should stay the validation layer for new audit DTOs and query envelopes.

### Project Structure Notes

- The live repo currently includes `customer-flow`, `session-domain`, `timing-policy`, `capture-adapter`, `branch-config`, `diagnostics-log`, and shared contracts, but it does not yet include the architecture-expected `operator-console` domain.
- Story 6.4 should establish that missing operator-facing surface in a minimal, disciplined way:
  - route-level separation in `App.tsx`
  - domain-local screens/components under `src/operator-console/`
  - audit host access through `src/diagnostics-log/services/`
- The most relevant rules from `_bmad-output/project-context.md` for this story are:
  - keep React UI code away from direct Tauri invocation
  - preserve typed cross-boundary DTOs
  - keep customer and operator capabilities clearly separated
  - treat compatibility-sensitive schema and migration changes as review-gated work

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/validation-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/sprint-change-proposal-2026-03-12.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/App.tsx`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src/diagnostics-log/services/lifecycleLogger.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src/shared-contracts/index.ts`
- `src-tauri/migrations/0001_init.sql`
- `src-tauri/src/db/sqlite.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/diagnostics/operator_log.rs`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/commands/operator_commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/contracts/dto.rs`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `tests/contract/operationalLogSchemas.test.ts`
- `src-tauri/tests/operational_log_foundation.rs`
- React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Scope: correlation-aware audit writes, host-owned audit queries, and a minimal separate operator surface for review
- Reuse strategy: extend the current SQLite/diagnostics foundation instead of inventing a second logging store
- Contract sensitivity: high because shared DTOs, migrations, Tauri commands, and operator/customer boundary rules all change together
- Key guardrail: do not leak diagnostics into the customer flow and do not let the frontend bypass the host for audit history

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation target: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`

### Completion Notes List

- Story context was generated from the refreshed Epic 6.4 requirement, the March 12, 2026 sprint-alignment proposal, the current operational logging implementation seams, recent git history, and official React / Tauri / Zod documentation.
- The document intentionally treats correlation-aware audit logging as a reusable platform capability that should precede richer operator console stories.
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation must be performed manually.

### File List

- `_bmad-output/implementation-artifacts/6-4-lifecycle-and-intervention-audit-logging.md`
