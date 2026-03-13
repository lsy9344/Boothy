# Story 1.4: Session Context Storage and Next-Surface Handoff

Status: done

Story Key: `1-4-session-context-storage-and-next-surface-handoff`

## Summary

Formalize the active-session handoff immediately after session provisioning so the customer flow uses one host-backed session identity across preparation and preset-selection entry states. This story should extend the current `SessionFlowProvider` and `CustomerFlowScreen` seam instead of inventing a second session store, while ensuring downstream booth surfaces disappear or reset cleanly when no active session is present.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want the app to remember my session after start,
so that subsequent screens can use the active session identity.

## Acceptance Criteria

1. Given a session has been created successfully, when the app receives the session identity response, then the active session context is stored in the session domain state, and the UI transitions into the next surface entry state (preset selection entry placeholder).
2. Given no active session exists, when a user tries to access downstream surfaces, then they are redirected to the Session Start screen, and no session-specific UI is shown.

## Tasks / Subtasks

- [x] Stabilize one active-session context path from host response to customer-flow state. (AC: 1)
  - [x] Keep `SessionStartResult` or its 1.3-aligned successor as the only frontend session-identity payload stored after provisioning succeeds; do not create a parallel session cache in route state, component-local state, or branch config storage.
  - [x] Ensure `SessionFlowProvider` stores the successful session result in one domain-owned field and clears stale session-specific state when provisioning fails or a later readiness regression invalidates the active flow.
  - [x] If Story 1.3 revised the start-session payload from the legacy `reservationName + phoneSuffix` shape to a session-name-first contract, update this story's consumers to follow the new host response instead of preserving the obsolete input contract in downstream code.

- [x] Drive next-surface handoff from active session truth rather than UI-only assumptions. (AC: 1)
  - [x] Keep the post-start progression state-driven: provisioning success -> preparing/entry placeholder -> preset-selection entry when readiness rules allow.
  - [x] Ensure `CustomerFlowScreen` and related selectors derive preparation, preset-selection, and capture-entry surfaces only from `activeSession`, readiness state, and approved preset-selection status.
  - [x] Reuse the existing delayed reveal or equivalent UX entry behavior only if it still respects the refreshed Epic 1 contract and does not skip the active-session handoff checkpoint.

- [x] Guard downstream surfaces when active session context is absent. (AC: 2)
  - [x] Prevent preset-selection, capture-entry, and any future downstream booth surfaces from rendering when `activeSession` is null or invalid for the current flow.
  - [x] Route or fall back to the Session Start surface from the current `/customer` entry path when session context is missing instead of showing stale preparation, preset, or capture UI.
  - [x] Ensure session-scoped selectors and review/preset helpers do not continue rendering stale labels, manifests, or capture data after session context is cleared.

- [x] Preserve host-owned session truth and manifest continuity through the handoff. (AC: 1-2)
  - [x] Continue sourcing `sessionId`, `sessionName`, `manifestPath`, and any session-scoped command inputs from the host-backed session result written by `start_session`.
  - [x] Reuse the existing `session.json` manifest path and session folder contract from Story 1.3; do not add a second durable session-context file or browser-only persistence mechanism in this story.
  - [x] Keep preset-selection and later timing/capture flows keyed to the active session identity so future stories build on the same manifest-backed session root.

- [x] Add regression coverage for session handoff and missing-session fallback. (AC: 1-2)
  - [x] Extend the current provider/screen tests to prove a successful session start stores active session context and advances into the preset-selection entry path.
  - [x] Add or update tests showing downstream booth UI collapses back to the Session Start surface when active session context is unavailable or invalidated.
  - [x] Keep contract and integration tests aligned with the canonical session-start DTO/schema if the Story 1.3 salvage path changes any session-start fields.

## Dev Notes

### Developer Context

- The refreshed Epic 1 sequence says Story 1.4 is not about creating session identity from scratch. Story 1.3 owns session identity creation and `session.json` provisioning; Story 1.4 owns how that host-created session becomes the single active context for the customer flow after start.
- Current repo reality already contains a partial salvage path:
  - `src/session-domain/services/sessionLifecycle.ts` invokes `start_session` through a typed adapter and parses the result with Zod.
  - `src/session-domain/state/SessionFlowProvider.tsx` already dispatches `provisioning_succeeded` and stores the returned session object in `state.activeSession`.
  - `src/customer-flow/screens/CustomerFlowScreen.tsx` already gates preparation, preset-selection, and capture-ready surfaces behind `activeSession`.
  - `src-tauri/src/commands/session_commands.rs` and `src-tauri/src/session/session_repository.rs` already persist the host-owned session root plus `session.json`.
- That means the implementation goal is to tighten and align the existing seam, not to build a new session-state architecture. Reuse the current provider/reducer pattern unless a clear contract issue forces a targeted refactor.
- There is one important baseline mismatch to keep visible: the refreshed PRD and epics moved the product toward session-name-first start, but the current repo still carries the older `reservationName` and `phoneSuffix` start payload. This story must not deepen that mismatch. Consume whatever canonical start-session contract Story 1.3 leaves behind and keep downstream context keyed by active session identity only.
- Scope boundary:
  - In scope: active session storage, UI handoff to the next customer-flow entry state, and missing-session guards.
  - Out of scope: redesigning the booth-start product input, real capture flows, review rail behavior, operator surfaces, or cross-restart session recovery unless another approved story explicitly requires them.

### Technical Requirements

- Treat the host response from `start_session` as the only approved active-session payload for the customer flow. At minimum the active context must continue carrying:
  - `sessionId`
  - `sessionName`
  - `sessionFolder`
  - `manifestPath`
  - `createdAt`
  - `preparationState`
- Keep active session storage domain-owned in the session state layer. Prefer extending:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  rather than introducing ad hoc `useState` ownership in `CustomerFlowScreen` or route components.
- Downstream flow transitions must remain state-driven and deterministic:
  - provisioning success stores active session context
  - readiness initialization attaches to that active session
  - the next-surface entry state is reached only when the active session exists
  - later preset-selection or capture-entry surfaces must reset if the active session is lost or invalidated
- Keep session-scoped command inputs derived from the active session context, not recomputed from customer-form fields after provisioning completes.
- Do not add browser-local persistence such as `localStorage`, query strings, or route params as a second durable session context. Durable truth already exists in the session root and `session.json` created by the host.
- If active session clearing is needed, clear dependent state in the same reducer transition so stale readiness, preset-selection, review, or capture-confidence data cannot render against a missing session.

### Architecture Compliance

- Preserve the architecture rule that session folders and `session.json` remain the durable source of booth truth. Story 1.4 is about frontend handoff to that truth, not replacing it.
- Keep React Router limited to top-level surface entry. Do not model customer-flow progression through new routes such as `/preset` or `/capture`; the architecture explicitly keeps booth progression state-driven rather than route-driven.
- React components must keep using typed adapter/service modules for host communication. Do not move `invoke('start_session')` or filesystem reads into `CheckInScreen`, `CustomerFlowScreen`, or presentation components.
- Customer-safe UI must remain diagnostics-free. Missing-session fallback should return to the Session Start surface without exposing host paths, manifest locations, or raw provisioning failures beyond the approved customer copy layer.
- Session context must stay isolated to the current booth session. Never let stale session labels, gallery data, or preset state survive after active session context is cleared or replaced.
- Follow the existing customer-shell -> session-domain -> typed adapter -> Rust host direction. Avoid parallel control flow where route state, branch config, or preset storage attempts to become the authoritative session owner.

### Library / Framework Requirements

- Current workspace baselines in the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
  - `libsqlite3-hotbundle`: `1.520000.0` (SQLite `3.52.0`)
- Use the existing React 19 pattern already present in `SessionFlowProvider.tsx`, including `useEffectEvent` for non-reactive readiness/capture event handling. Do not introduce a global event bus or imperative session singleton for this story.
- Keep Tauri communication on the approved v2 command path. Session handoff should continue to consume the typed `start_session` command result; do not bypass that by reading app-local files directly from the frontend.
- Keep Zod 4 as the TypeScript-side contract gate if the session-start DTOs or result shape change while reconciling Story 1.3 and Story 1.4.
- Do not introduce a new persistence dependency for session context. The current stack already has the right split:
  - session durability in the host/session manifest
  - operational logging in SQLite
  - limited branch-local UX settings in Tauri Store

### File Structure Requirements

- Expected primary implementation surface:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/session-domain/services/sessionLifecycle.ts`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CustomerEntryScreen.tsx`
  - `src/customer-flow/screens/CheckInScreen.tsx` only if the fallback or submit flow needs minor wiring changes
- Contract and host files to touch only if Story 1.3 salvage changes session identity or the returned session shape:
  - `src/shared-contracts/dto/session.ts`
  - `src/shared-contracts/schemas/sessionSchemas.ts`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/session/session_repository.rs`
- Test surfaces likely needing updates:
  - `src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
  - `tests/integration/checkInFlow.test.tsx`
  - `tests/integration/presetSelectionFlow.test.tsx`
  - `tests/integration/sessionLifecycle.test.ts`
  - new session-guard coverage if existing tests do not already prove missing-session fallback
- Keep session-context logic out of:
  - `src/App.tsx` beyond top-level route wiring
  - `src/branch-config/*` except reading approved branch metadata
  - presentation-only `src/shared-ui/*`
- Do not create a second customer-flow route tree just to represent preset-selection or capture handoff. Use the existing screen composition inside `CustomerFlowScreen`.

### Testing Requirements

- Add or update reducer/provider tests that prove:
  - provisioning success stores the host-backed active session
  - the handoff into preparing/preset-selection uses that active session
  - clearing or invalidating active session state removes downstream session-specific UI
- Keep integration coverage around the real user-visible path:
  - submit start form
  - receive successful session result
  - show preparation/entry placeholder
  - advance into preset-selection entry
- Add a missing-session regression test that proves no preset-selection, capture, or review surface renders when `activeSession` is null.
- If Story 1.3 contract reconciliation changes the session-start DTO or response shape, update contract tests and session lifecycle integration tests in the same change. Do not let the typed adapter and host drift silently.
- Avoid snapshot-only UI tests. This story is about state continuity and gating, so assertions should prove actual phase/session behavior.

### Previous Story Intelligence

- The available previous story artifact in `implementation-artifacts` is the older salvage story `1-3-host-facing-camera-contract-and-session-schema-baseline.md`, not the refreshed Epic 1.3 title. Its useful guidance still applies:
  - keep session and camera boundary contracts typed and host-owned
  - treat `session.json` as canonical local truth
  - keep React/UI code out of direct Tauri and filesystem ownership
- Current repo implementation confirms those ideas were already applied:
  - `sessionLifecycleService.startSession()` parses the host result before UI use
  - `start_session` in Rust provisions the session root and returns canonical identity fields
  - `session_repository.rs` persists manifest changes against the host-owned session root
- Practical lesson for Story 1.4: do not respond to active-session handoff by building another store or client-side manifest cache. The previous story already established where truth lives; this story should wire the customer flow to that truth cleanly.

### Git Intelligence Summary

- Recent history is still dominated by the greenfield reset:
  - `06ed2b7` rebuilt the repository around the BMAD planning package.
  - Older pre-reset commits (`1fb8bb0`, `3ef405f`, `cb12647`, `de9d881`) focused on camera-state reliability and showed how easily UI state can drift from host/device truth when ownership is split across layers.
- Actionable lesson for this story:
  - keep session handoff centralized in the session-domain provider/reducer seam
  - avoid parallel route-state or component-state ownership of the active session
  - prefer tightening the current root-level implementation structure instead of reviving deleted app subtrees or older state-management shortcuts

### Latest Tech Information

Verified against official docs on 2026-03-12:

- React 19.2 remains the current line used by the workspace, and the official React 19.2 release continues to recommend `useEffectEvent` for event-like logic fired from effects. That matches the existing `SessionFlowProvider` pattern and is the right mechanism for session/readiness handoff side effects in this story.
- Tauri v2 official docs still position frontend-to-host commands as the standard request/response boundary. Keep session provisioning and session-context handoff on the typed `start_session` command path rather than introducing direct frontend file access.
- Tauri's current docs and references show the active `@tauri-apps/api` and CLI release line at `2.10.1`, which matches the repo baseline. This story should extend that baseline, not introduce alternate desktop bridge patterns.
- Zod 4 remains the stable line and still positions itself as the latest version with improved performance and JSON Schema support. Continue using it as the TypeScript contract gate if the start-session payload or result changes.
- SQLite 3.52.0 was released on 2026-03-06 and fixed a WAL-reset corruption bug. The repo already bundles that line through `libsqlite3-hotbundle`, so Story 1.4 should not add alternate client-side persistence for session context.
- SQLite still requires connection-specific `PRAGMA foreign_keys` handling, reinforcing the existing architecture split where operational durability stays in the host SQLite layer and not in ad hoc frontend storage.

### Project Structure Notes

- The current implementation already follows the domain-first structure expected by architecture for this story:
  - `customer-flow` owns screen composition
  - `session-domain` owns active session state and reducers
  - `capture-adapter` owns host-facing readiness/capture integration
  - `src-tauri/src/session` owns manifest and session-root persistence
- `App.tsx` currently exposes only `/customer`, and `CustomerEntryScreen.tsx` gates the first booth entry step before handing control to `CustomerFlowScreen`. That means session handoff belongs inside the customer/session-domain seam, not in route expansion.
- Detected variance to account for:
  - planning now expects a session-name-first Epic 1 flow
  - current repo still carries `reservationName` and `phoneSuffix` in the session-start contract
  - Story 1.4 should avoid deepening that variance and instead consume the final Story 1.3 identity contract cleanly

### Project Context Reference

- `_bmad-output/project-context.md` remains active context for this story.
- The most relevant rules from that file are:
  - keep React components away from direct Tauri invocation
  - preserve session folders as the durable source of truth
  - avoid cross-session leakage in UI state and fixtures
  - keep routes limited to top-level surfaces
  - keep shared DTOs fully typed with Zod validation on the TypeScript boundary
- Story 1.4 should be implemented as a boundary-preserving extension of those rules, not an exception to them.

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/session-domain/services/sessionLifecycle.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CustomerEntryScreen.tsx`
- `src/customer-flow/screens/CheckInScreen.tsx`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_repository.rs`
- `tests/integration/checkInFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- `tests/integration/sessionLifecycle.test.ts`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri v2 frontend calling docs: https://v2.tauri.app/develop/calling-frontend/
- Tauri v2 JavaScript path API reference: https://v2.tauri.app/reference/javascript/api/namespacepath/
- Zod 4 docs: https://zod.dev/v4
- SQLite 3.52.0 release log: https://sqlite.org/releaselog/3_52_0.html
- SQLite foreign key pragma docs: https://sqlite.org/pragma.html#pragma_foreign_keys

## Story Readiness

- Status: `done`
- Scope: active session storage, next-surface handoff, and missing-session guards only
- Reuse strategy: extend the current session-domain/customer-flow seam instead of creating new session-state infrastructure
- Contract sensitivity: high if Story 1.3 changes the session-start DTO or result shape
- Repo variance to watch: refreshed planning is session-name-first, while the current repo still exposes legacy reservation/phone fields

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation was completed manually
- Verification commands:
  - `pnpm vitest run tests/contract/sessionContracts.test.ts src/session-domain/services/reservationValidation.spec.ts tests/integration/sessionLifecycle.test.ts tests/integration/sessionEntryFlow.test.tsx tests/integration/customerReadinessFlow.test.tsx tests/integration/presetSelectionFlow.test.tsx tests/integration/sessionGalleryIsolation.test.tsx src/session-domain/state/sessionReducer.story-1-4.spec.ts`
  - `cargo test --quiet`

### Completion Notes List

- Story context generated from the refreshed Epic 1.4 requirement, current repo seams, the prior Story 1.3 contract baseline, the March 12, 2026 readiness assessment, recent git history, and official React / Tauri / Zod / SQLite references.
- The story is intentionally written as a salvage-and-align guide: extend the existing `SessionFlowProvider` / `CustomerFlowScreen` seam instead of creating a parallel session store.
- Current repo variance from the refreshed session-name-first planning baseline is called out explicitly so downstream implementation does not accidentally harden the outdated `reservationName + phoneSuffix` contract.
- Added reducer-backed session clearing and start-journey handling so stale session-scoped readiness, preset, capture, and review state cannot survive a missing-session fallback.
- Aligned the customer/session start path, frontend DTO usage, and Rust `start_session` payload handling to the canonical session-name-first contract without adding alternate persistence.
- Added and updated contract, reducer, integration, and host tests covering session lifecycle handoff, missing-session fallback, preset-selection continuity, and Rust DTO/manifest compatibility.
- Follow-up review fixes on 2026-03-13 restored the `/customer` start surface to the session-name form, reconnected `startJourney()` to real provisioning, and refreshed regression tests around missing-session fallback and retry recovery.

### File List

- `_bmad-output/implementation-artifacts/1-4-session-context-storage-and-next-surface-handoff.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/customer-flow/copy/sessionStartErrorCopy.ts`
- `src/customer-flow/screens/CheckInScreen.tsx`
- `src/customer-flow/screens/CustomerEntryScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
- `src/customer-flow/screens/CustomerStartScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.story-1-4.spec.ts`
- `src/session-domain/state/sessionReducer.ts`
- `tests/integration/checkInFlow.test.tsx`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- `tests/integration/sessionEntryFlow.test.tsx`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `tests/integration/sessionStartFailureFlow.test.tsx`

## Change Log

- 2026-03-12: Completed Story 1.4 by tightening reducer-owned active-session clearing, wiring `/customer` fallback to the session start surface, aligning the session-start contract to `sessionName`, and adding frontend plus Rust regression coverage.
- 2026-03-13: Fixed review follow-ups by restoring the session-name start surface on `/customer`, reconnecting `startJourney()` to provisioning, and updating regression coverage plus the story file list to match the current implementation seam.
