# Story 1.3: Session Identity Creation and Session Root Manifest

Status: done

Story Key: `1-3-session-identity-creation-and-session-root-manifest`

## Summary

Create the first durable session-start implementation for the corrected booth-first flow: a non-empty `sessionName` is submitted through typed frontend/host contracts, the Tauri host creates a unique session root under the local sessions directory, writes `session.json`, returns the new session identity to the frontend, and surfaces customer-safe failures without leaking filesystem or Rust diagnostics. The current repo still contains legacy `reservationName + phoneSuffix` inputs and an obsolete earlier `1-3-*` story artifact; treat both as drift to be replaced, not reused.

## Story

As a booth customer,
I want my session name to create a valid session identity,
so that my booth session is uniquely tracked from the start.

## Acceptance Criteria

1. Given a non-empty session name, when the customer confirms session start, then the host creates a new session root folder and `session.json` is written with at least `schemaVersion`, `sessionId`, `sessionName`, and `createdAt`.
2. Given a host validation or filesystem error occurs, when session start is attempted, then a typed error envelope is returned and a customer-safe error message is shown without internal diagnostics.

## Tasks / Subtasks

- [x] Replace the legacy session-start contract and validation with a `sessionName`-first payload. (AC: 1, 2)
  - [x] Remove `reservationName` and `phoneSuffix` from the active customer start path, session DTOs, and start-session schemas.
  - [x] Introduce a trimmed non-empty `sessionName` input and a typed session-start payload/result aligned across React, Zod, and Rust.
  - [x] Preserve optional timing fields only if they are still required by downstream timing services; do not keep legacy customer-identifying fields just to satisfy old code.
- [x] Persist the session root and manifest in the Rust host when session start is confirmed. (AC: 1)
  - [x] Reuse the existing host-owned session root pattern under `app_local_data_dir()/sessions/`.
  - [x] Generate a unique same-day session identity from the normalized session name, preserving deterministic suffixing for collisions.
  - [x] Write `session.json` during provisioning with at least `schemaVersion`, `sessionId`, `sessionName`, and `createdAt`, plus any additional fields still required by the active manifest schema.
- [x] Wire the frontend flow to consume the new durable start-session result and typed failures. (AC: 1, 2)
  - [x] Submit the session start through `sessionLifecycleService`, not direct UI `invoke` calls.
  - [x] Update reducer/context flow so successful provisioning stores the returned identity without reintroducing Story 1.4 scope creep.
  - [x] Map validation/filesystem failures to customer-safe copy and keep internal diagnostics out of the customer surface.
- [x] Reconcile or retire legacy manifest/session naming assumptions. (AC: 1, 2)
  - [x] Replace legacy `reservationName + phoneSuffix` naming rules in TS/Rust helpers and tests with session-name-based uniqueness rules.
  - [x] Remove or migrate manifest fields/tests that still require reservation/phone-derived identity.
  - [x] Explicitly ignore the superseded `_bmad-output/implementation-artifacts/1-3-host-facing-camera-contract-and-session-schema-baseline.md` for current story scope.
- [x] Add regression coverage for durable session provisioning and failure handling. (AC: 1, 2)
  - [x] Add or update frontend validation/integration tests for the single-field start flow and typed failure copy.
  - [x] Add or update Rust tests for unique session naming, manifest persistence, and cleanup on provisioning failure.
  - [x] Verify `session.json` contains the required identity fields and no longer depends on reservation/phone inputs.

### Review Follow-ups (AI)

- [x] [AI-Review][High] Split `start_session` failure handling into distinct typed validation/provisioning codes so frontend copy can map validation and filesystem failures separately. [src/shared-contracts/dto/session.ts:24]
- [x] [AI-Review][Medium] Add command-level regression coverage for `start_session` rollback so failed provisioning proves partial session folders are removed. [src-tauri/src/commands/session_commands.rs:143]
- [x] [AI-Review][Medium] Add production-path success/failure coverage for `CustomerFlowScreen` instead of leaving `CustomerFlowScreen.spec.tsx` as a `resolveCaptureView` unit test only. [src/customer-flow/screens/CustomerFlowScreen.spec.tsx:7]
- [x] [AI-Review][Medium] Reconcile the Dev Agent Record with the actual implementation surface and fresh verification counts before closing the story. [_bmad-output/implementation-artifacts/1-3-session-identity-creation-and-session-root-manifest.md:198]

## Dev Notes

### Developer Context

- Sprint status says the current `1.3` backlog story is `1-3-session-identity-creation-and-session-root-manifest`, but the repo still contains a legacy `reservationName` / `phoneSuffix` flow and an older done artifact with a different `1-3` title. Current implementation must follow the reset sprint status, not the superseded implementation artifact.
- Story 1.2 has not landed in the codebase yet. Current `CustomerEntryScreen -> CustomerFlowScreen -> CheckInScreen` still advances into the old two-field check-in. Either implement Story 1.2 first or carry its session-name-only entry correction as prerequisite changes in the same branch before wiring durable provisioning.
- Keep this story focused on durable session identity creation. Do not absorb Story 1.4's broader downstream access guards or route orchestration beyond the minimal active-session storage the current state machine already needs.
- Planning baseline and legacy business-context documents conflict: older operational notes still describe `reservationName + phoneLast4` identity. That is planning drift for this sprint. Do not preserve it as a required product rule.

### Technical Requirements

- `SessionStartPayload` should become session-name-first: `sessionName` is required, `branchId` remains host-required in the command payload, and any optional `reservationStartAt` / `sessionType` fields should remain only if timing initialization still needs them.
- Trim whitespace before validation. Empty or whitespace-only `sessionName` must fail with a typed error code such as `session_name.required`.
- If filesystem-safe normalization is needed, keep it host-owned and deterministic. Do not require Hangul-only input or 4-digit phone suffix validation anymore.
- Keep host provisioning behind `start_session`. The Rust command should create the session root, write `session.json`, and only then return `sessionId`, `sessionName`, `sessionFolder`, `manifestPath`, `createdAt`, and `preparationState`.
- Preserve or update the current unique same-day naming pattern (`_2`, `_3`, ...) so duplicate session names do not collide on disk.
- `session.json` must include at minimum `schemaVersion`, `sessionId`, `sessionName`, and `createdAt`. If current timing/capture flows require more fields, update TS and Rust schemas together rather than letting the manifest drift.
- Validation or provisioning failure must return a typed envelope/code to the frontend. Customer UI must show approved safe copy only; raw Rust or filesystem error strings must not be rendered on the booth surface.
- Successful provisioning should still insert the session-created lifecycle event through the existing operational log path after manifest creation succeeds.

### Architecture Compliance

- React/UI code must keep using typed adapter/service layers. No direct `invoke('start_session')` in components.
- Session root creation and `session.json` writes remain host-owned in Rust. React local state must not become the durable source of truth.
- Keep cross-boundary payloads `camelCase` on the wire and `snake_case` internally in Rust as already established in `src-tauri/src/contracts/dto.rs`.
- Preserve the app-local sessions directory strategy from the existing host code. If folder topology changes, update path resolvers, manifest schema, and tests together.
- Customer-visible copy must remain customer-safe and within the booth copy budget; no filesystem paths, exception text, or internal diagnostics on the booth screen.
- Do not reintroduce the superseded contract story's broader camera/sidecar scope into this work. Story 1.3 is about durable session identity creation for the corrected booth-start flow.

### Library / Framework Requirements

- Use the repo's current baselines from `package.json` and `src-tauri/Cargo.toml`: React `19.2.x`, TypeScript `5.9.x`, Vite `7.3.x`, React Router `7.9.4`, Tauri `2.10.x`, Zod `4.3.x`, and Rust `1.77.2+`.
- Keep `sessionLifecycleService` as the typed Tauri boundary; this is consistent with Tauri v2's documented command-based frontend-to-Rust flow. [Source: https://v2.tauri.app/develop/calling-rust/]
- Keep the current Vite/Tauri scaffold and Node runtime floor rather than mixing this story with toolchain churn. Vite 7's release notes still require Node `20.19+` or `22.12+`. [Source: https://vite.dev/blog/announcing-vite7]
- React `19.2` is current and already compatible with the repo's existing `useEffectEvent` usage; there is no need to downgrade React patterns while implementing this story. [Source: https://react.dev/blog/2025/10/01/react-19-2]
- Stay on Zod 4 for frontend-side input/result schema validation; do not introduce parallel hand-rolled payload validation. [Source: https://zod.dev/v4]

### File Structure Requirements

- Frontend session-start path and copy:
  - `src/customer-flow/screens/CustomerEntryScreen.tsx`
  - `src/customer-flow/screens/CustomerStartScreen.tsx`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CheckInScreen.tsx`
  - `src/customer-flow/copy/customerStartCopy.ts`
  - `src/customer-flow/copy/checkInCopy.ts`
- Frontend state/contract/service seam:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/session-domain/services/sessionLifecycle.ts`
  - `src/session-domain/services/reservationValidation.ts` (or rename it to reflect session-name validation)
  - `src/session-domain/services/sessionNaming.ts`
  - `src/session-domain/services/sessionManifest.ts`
  - `src/session-domain/services/sessionPaths.ts`
  - `src/shared-contracts/dto/session.ts`
  - `src/shared-contracts/dto/sessionManifest.ts`
  - `src/shared-contracts/schemas/sessionSchemas.ts`
  - `src/shared-contracts/schemas/manifestSchemas.ts`
- Rust host provisioning seam:
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_paths.rs`
  - `src-tauri/src/lib.rs` only if command registration or plugins need adjustment
- Keep changes in these existing domain-first locations. Do not revive deleted `apps/boothy` paths and do not redirect story instructions to obsolete implementation artifacts.

### Testing Requirements

- Frontend validation/unit tests:
  - update `src/session-domain/services/reservationValidation.spec.ts` for `sessionName`-only validation and typed error codes
  - update `src/session-domain/services/sessionNaming.spec.ts` for session-name-based uniqueness instead of reservation+phone composition
  - update `src/session-domain/services/sessionManifest.spec.ts` if manifest identity fields change
- UI/integration tests:
  - update `src/App.spec.tsx` to reflect the corrected session-name-first entry flow
  - update `src/customer-flow/screens/CustomerFlowScreen.spec.tsx` to stop typing reservation/phone values and to verify durable provisioning success/failure handling
  - update `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx` only as needed to preserve Story 1.1 shell expectations
- Rust tests:
  - update `src-tauri/tests/session_repository.rs` for same-day duplicate session-name suffixing, manifest persistence, and minimum required identity fields
  - update `src-tauri/tests/session_manifest.rs` for the new manifest identity shape
  - add or adjust command-level failure coverage so provisioning cleanup and typed failure envelopes are verified
- Validation target:
  - verify `session.json` is created on successful provisioning
  - verify failed provisioning does not leave partial session folders behind
  - verify customer UI does not render raw host error details

### Previous Story Intelligence

- Story 1.2 is the immediate conceptual predecessor, and its saved artifact already says the booth entry should be session-name-only and must not provision the real session root yet. That file is the right predecessor for current `1.3`, even though the repo code has not caught up.
- Story 1.1 already established the reusable booth shell, first-screen lifecycle logging, and the `CustomerStartScreen` entry point. Reuse those instead of rebuilding the entry surface.
- The current code proves where the `1.2` / `1.3` boundary drift lives:
  - `CustomerEntryScreen` still flips from shell to flow with a local boolean only.
  - `CheckInScreen`, `SessionFlowProvider`, `sessionReducer`, and `reservationValidation.ts` still assume `reservationName` and `phoneSuffix`.
  - `sessionLifecycleService`, shared session DTOs, and Rust `start_session` still mirror the old two-field payload.
- The old implementation artifact `_bmad-output/implementation-artifacts/1-3-host-facing-camera-contract-and-session-schema-baseline.md` is a useful warning about broad contract work, but it is not the current story definition and should not dictate scope for sprint `1.3` now that the plan has been reset.

### Git Intelligence Summary

- Recent commits show the current root-level Boothy app already contains session-domain, manifest, timing, capture, and Tauri session modules, so Story `1.3` should extend the active repo rather than invent a second scaffolding path.
- The current checked-in code still reflects a legacy check-in model:
  - `src/App.spec.tsx` expects the first click to advance into `예약자명` and `휴대전화 뒤4자리`.
  - `src/session-domain/services/sessionNaming.ts` still composes session names from `reservationName + phoneSuffix`.
  - `src-tauri/src/commands/session_commands.rs` and `src-tauri/src/session/session_repository.rs` still validate and persist `reservation_name` and `phone_last4`.
- The git history also shows a previously completed but now superseded `1-3-host-facing-camera-contract-and-session-schema-baseline` artifact. Treat that as historical context only; the current sprint status is authoritative.
- Practical implementation implication: Story `1.3` is partly a migration story. Do not layer `sessionName` on top of the legacy fields; replace the old identity path end to end.

### Latest Technical Information

- React `19.2` is the current official release line. The repo's existing React 19 patterns remain valid for the session-start flow. [Source: https://react.dev/blog/2025/10/01/react-19-2]
- Vite 7 official release notes confirm the current Node floor used by this repository: `20.19+` or `22.12+`. Keep any new verification commands compatible with that requirement. [Source: https://vite.dev/blog/announcing-vite7]
- Tauri v2 official docs continue to support the frontend-first workflow of creating the app scaffold first and calling Rust via typed commands from the frontend. That matches the existing `sessionLifecycleService -> start_session` architecture in this repo. [Source: https://v2.tauri.app/start/create-project/] [Source: https://v2.tauri.app/develop/calling-rust/]
- Zod 4 remains the correct schema-validation line for the project's shared TypeScript contracts. Do not downgrade to ad hoc validation or duplicate DTO definitions. [Source: https://zod.dev/v4]

### Project Context Reference

- Follow `_bmad-output/project-context.md` as the compressed implementation rule set:
  - keep React UI out of raw Tauri commands
  - preserve typed cross-boundary DTOs
  - keep code domain-first
  - treat session folders as durable truth
  - avoid introducing branch-specific shortcuts or customer-facing diagnostics
- For this story, the highest-signal planning sources are:
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`
  - `_bmad-output/implementation-artifacts/1-2-booth-app-shell-and-session-start-screen.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Treat `docs/business_context/context.md` as legacy operational context, not as the sprint-authoritative product definition for session identity.

## Story Readiness

- Status: `done`
- Primary implementation risk: the legacy identity model is spread across UI, TS contracts, Rust commands, manifest schema, and tests.
- Primary guardrail: replace the old identity path end to end instead of temporarily supporting both models.
- Dependency note: implement after Story `1.2` or carry its session-name-only entry correction as explicit prerequisite work.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Implementation Plan

- Add a typed host-validation failure code to the shared session-start contract and customer-safe copy mapping without widening the public surface beyond Story 1.3.
- Add command-level coverage around `start_session` failure classification and rollback cleanup to prove partial provisioning artifacts are removed safely.
- Replace the placeholder `CustomerFlowScreen.spec.tsx` unit-only coverage with production-path success/failure tests that exercise the actual customer start flow.
- Reconcile the story record with the real changed files and fresh verification output before returning the story to review.

### Debug Log References

- Story context created from planning artifacts, current repo implementation, git history, and official framework/vendor docs.
- `_bmad/core/tasks/validate-workflow.xml` was not present, so the checklist intent was validated manually against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`.
- Added RED tests for typed validation failures at the shared contract, lifecycle boundary, and `CustomerFlowScreen` production path before updating code.
- Added command-level Rust tests for validation-failure classification and rollback cleanup in `start_session`.
- Fresh full verification completed with `pnpm test:run`, `pnpm lint`, and `cargo test`.

### Completion Notes List

- Replaced the customer entry surface with a session-name-first form that validates inline, records the first-screen lifecycle event, and starts durable session creation through `SessionFlowProvider` and `sessionLifecycleService`.
- Kept `sessionName` as the active identity source across validation, host provisioning, manifest persistence, and same-day collision suffixing, while preserving optional timing inputs used by later timing flows.
- Refreshed the related frontend integration tests and fixed the session gallery isolation mock so the full Vitest suite exercises the updated entry path cleanly.
- ✅ Resolved review finding [High]: split the `start_session` boundary so host validation failures now return `session.validation_failed` and the customer copy can distinguish them from provisioning failures.
- ✅ Resolved review finding [Medium]: added command-level regression coverage proving rollback cleanup removes partial session artifacts before returning a safe failure envelope.
- ✅ Resolved review finding [Medium]: replaced the placeholder `CustomerFlowScreen.spec.tsx` coverage with production-path success/failure tests and moved `resolveCaptureView` unit coverage into its own spec.
- ✅ Resolved review finding [Medium]: reconciled this story record with the real implementation surface and fresh verification totals.
- Fresh verification evidence:
  - `pnpm test:run` passed with `64` files and `216` tests.
  - `pnpm lint` passed with exit code `0`.
  - `cargo test` passed with all Rust tests green.
- Final close-out review on `2026-03-13` found no remaining implementation issues for Story 1.3, so the story is closed as `done`.

### File List

- `_bmad-output/implementation-artifacts/1-3-session-identity-creation-and-session-root-manifest.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/App.spec.tsx`
- `src/customer-flow/copy/customerStartCopy.ts`
- `src/customer-flow/copy/sessionStartErrorCopy.ts`
- `src/customer-flow/screens/CustomerEntryScreen.tsx`
- `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`
- `src/customer-flow/screens/CustomerStartScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
- `src/customer-flow/screens/customerFlowView.spec.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/shared-contracts/dto/session.ts`
- `src/shared-contracts/schemas/sessionSchemas.ts`
- `src-tauri/src/commands/session_commands.rs`
- `tests/integration/checkInFlow.test.tsx`
- `tests/integration/sessionEntryFlow.test.tsx`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `tests/integration/sessionLifecycle.test.ts`
- `tests/contract/sessionContracts.test.ts`

## Change Log

- 2026-03-13: Addressed code review findings for Story 1.3 by adding typed host-validation failure handling, rollback cleanup regression coverage, production-path `CustomerFlowScreen` tests, and refreshed story verification records.
- 2026-03-13: Completed final independent review, confirmed no remaining findings, and advanced Story 1.3 from `review` to `done`.
