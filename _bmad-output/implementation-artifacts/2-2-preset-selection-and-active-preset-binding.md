# Story 2.2: Preset Selection and Active Preset Binding

Status: done

Story Key: `2-2-preset-selection-and-active-preset-binding`

## Summary

Turn the current partial preset-selection salvage path into the approved booth behavior: a customer must explicitly choose one approved preset for the active session before capture begins, the confirmed preset must be persisted through the typed `select_session_preset` host boundary, and the UI must show that preset as the active look after binding succeeds. The current repo already contains reusable catalog, host command, manifest persistence, and capture-surface preset display pieces, but it does not yet satisfy Story 2.2 because `PresetScreen` has no confirm action, `SessionFlowProvider` auto-loads a default/last-used selection, and card taps immediately persist the preset instead of separating browse/select from confirm.

## Story

As a booth customer,
I want to select one preset before capture begins,
so that my chosen look is applied to subsequent captures.

## Acceptance Criteria

1. Given the preset catalog is displayed, when I select a preset card and confirm, then the selected preset becomes the active preset for the session, and the UI reflects the active preset state clearly.
2. Given no preset is selected, when I attempt to proceed to capture, then the app prevents continuation, and prompts me to choose a preset.

## Tasks / Subtasks

- [x] Separate preset browsing from preset confirmation in the initial preset-selection surface. (AC: 1, 2)
  - [x] Update `PresetScreen` so card interaction chooses a candidate preset for the current screen state instead of immediately persisting it through the host.
  - [x] Add an explicit primary confirm/continue action for the chosen preset and keep it disabled until the customer has made a session-scoped choice.
  - [x] Keep the screen within the customer copy budget and do not introduce operator, authoring, or branch-audit language.

- [x] Bind the confirmed preset to the active session through the existing typed host path. (AC: 1)
  - [x] Reuse `presetSelectionService.selectPreset()` and the Rust `select_session_preset` command as the only persistence path for the initial preset binding.
  - [x] Persist `activePreset` and `activePresetName` in the session manifest only after confirmation succeeds.
  - [x] Advance from `preset-selection` to `capture-loading` only after the confirmed selection succeeds and the reducer stores the new active preset.

- [x] Enforce the "no preset selected, no capture" rule at the state-machine level. (AC: 2)
  - [x] Start each new session in preset selection with no confirmed preset bound for that session.
  - [x] Do not treat `lastUsedPresetId` or a default catalog entry as an already-confirmed choice for the current session.
  - [x] Show customer-safe prompt/validation when the customer attempts to continue without selecting a preset.

- [x] Keep active-preset display and downstream capture state aligned with the confirmed session preset. (AC: 1)
  - [x] Reuse the existing active-preset display seams in `CustomerFlowScreen`, `CaptureScreen`, and the capture-confidence selectors instead of inventing a parallel preset store.
  - [x] Ensure capture-ready UI reflects the manifest-backed active preset after confirmation rather than a synthetic fallback choice.
  - [x] Preserve the bounded approved catalog from shared contracts; Story 2.2 must not add branch-specific catalog logic or preset-authoring controls.

- [x] Add regression coverage for initial preset confirmation and gating behavior. (AC: 1, 2)
  - [x] Add UI/provider tests proving the screen cannot continue until a preset has been explicitly selected.
  - [x] Add or update service/reducer tests proving host persistence happens on confirm, not on card browse/highlight.
  - [x] Keep Rust-side persistence coverage for manifest `activePreset` updates and add command-level failure coverage if the initial confirm flow changes.

### Review Follow-ups (AI)

- [x] [AI-Review][High] Add command-level failure coverage for `select_session_preset` so invalid preset/session inputs prove the typed failure envelope expected by Story 2.2 and the frontend schema. [src-tauri/src/commands/session_commands.rs]
- [x] [AI-Review][Medium] Surface a customer-safe error state when preset confirmation fails; the current flow now keeps the customer on preset selection with retry guidance. [src/session-domain/state/SessionFlowProvider.tsx]
- [x] [AI-Review][Medium] Reconcile the story File List with the actual verification artifacts used for this story so future reviews can audit the implementation cleanly. [src/session-domain/services/presetSelection.spec.ts]
- [x] [AI-Review][High] Preserve the confirmed active preset when readiness temporarily regresses after initial confirmation; the reducer/provider now keep the confirmed preset and resume capture for the same session instead of re-entering initial preset selection. [src/session-domain/state/sessionReducer.ts] [src/session-domain/state/SessionFlowProvider.tsx]
- [x] [AI-Review][Medium] Preserve and translate the typed host failure envelope when preset confirmation fails instead of collapsing every failure into the same retry copy; the provider now stores the typed failure and maps session-integrity failures to restart guidance. [src/session-domain/state/SessionFlowProvider.tsx]
- [x] [AI-Review][Medium] Add provider or integration coverage for the failed confirm path; integration coverage now proves `confirmPresetSelection()` retains the typed failure envelope and shows the correct customer-safe guidance. [src/session-domain/state/sessionReducer.story-2-2.spec.ts] [tests/integration/presetSelectionFlow.test.tsx]
- [x] [AI-Review][Medium] Make the `pnpm test:run` verification claim reproducible or document the current suite result; the stale pass claim was replaced with fresh verification notes showing current unrelated frontend failures. [Verification]
- [x] [AI-Review][High] Guard `confirmPresetSelection()` against stale async results so an in-flight response from an old session cannot drive a newer or cleared session into `capture-loading` with the wrong preset state; the provider now correlates responses to the still-active session and the reducer rejects mismatched session actions. [src/session-domain/state/SessionFlowProvider.tsx] [src/session-domain/state/sessionReducer.ts]
- [x] [AI-Review][Medium] Prevent stale preset-confirm responses from overwriting branch-level last-used preset preferences after the active session has been cleared or replaced; the provider now skips `saveLastUsedPresetId()` when the request no longer belongs to the active session. [src/session-domain/state/SessionFlowProvider.tsx]
- [x] [AI-Review][Medium] Add integration coverage for the stale confirm case by clearing the session while preset confirmation is in flight and proving the old response is ignored. [tests/integration/presetSelectionFlow.test.tsx]
- [x] [AI-Review][Medium] Sync the story verification notes with the current workspace result now that full `pnpm test:run` passes. [Verification]
- [x] [AI-Review][High] Replace preset-selection failure message parsing with typed repository/host error codes so session restart vs retry guidance is derived from contract codes only. [src-tauri/src/diagnostics/error.rs] [src-tauri/src/session/session_repository.rs] [src-tauri/src/commands/session_commands.rs] [src/session-domain/state/SessionFlowProvider.tsx]
- [x] [AI-Review][Medium] Remove unsafe casts from the typed preset-selection failure regression test so schema drift is caught by the actual service/result contract. [tests/integration/presetSelectionFlow.test.tsx]
- [x] [AI-Review][Medium] Restore full verification reproducibility by fixing the remaining timing/watcher test drift and stale in-flight preset-change harness timing. [tests/integration/customerReadinessFlow.test.tsx] [tests/integration/presetSelectionFlow.test.tsx] [src/session-domain/state/sessionReducer.story-4-1.spec.ts] [src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx]
- [x] [AI-Review][Medium] Correct the story note about contradictory readiness payloads: the implementation now enforces the strict normalized-readiness contract and the story artifact reflects that stricter behavior. [src/session-domain/state/customerPreparationState.ts] [src/session-domain/state/customerPreparationState.spec.ts]

## Dev Notes

### Developer Context

- Story 2.1 does not have a saved `ready-for-dev` artifact yet, but the repo already contains a salvageable preset foundation:
  - `src/customer-flow/data/mvpPresetCatalog.ts` builds the bounded customer catalog from shared approved preset contracts.
  - `src/customer-flow/screens/PresetScreen.tsx` renders the catalog screen.
  - `src/session-domain/services/presetSelection.ts` already calls the typed `select_session_preset` command.
  - `src-tauri/src/commands/session_commands.rs` and `src-tauri/src/session/session_repository.rs` already persist `activePreset` into `session.json`.
- The problem is behavioral mismatch, not missing infrastructure. Current flow auto-enters preset selection with a default or last-used preset already selected and persists immediately on card tap. That violates Story 2.2's explicit choose-and-confirm requirement and makes AC2 impossible to verify.
- Story 2.2 should therefore rework the existing preset-selection seam, not rebuild the catalog stack and not absorb Story 2.3 branch-consistency concerns.
- Story 1.4 already established the active-session handoff seam. Treat an existing `activeSession` as a prerequisite and keep this story focused on initial preset binding for that session.
- UX guidance still talks about live preview and ultra-fast preset switching, but the approved planning baseline for this story only requires bounded preset choice, clear active-preset feedback, and gating before capture. Do not add a real-time camera preview requirement here unless the product definition is updated.

### Technical Requirements

- Introduce a clear distinction between:
  - candidate preset selected on the preset-selection screen
  - confirmed active preset bound to the session manifest
- `selectedPresetId` in session state should no longer mean "already persisted" by itself. Either repurpose it as a candidate-selection field or add a separate candidate field, but do not let screen selection imply host persistence until the confirm action fires.
- A new session entering `preset-selection` must not silently inherit `lastUsedPresetId` as its confirmed session preset. If last-used behavior is kept, treat it as a suggestion only; the customer still needs an explicit current-session choice.
- The confirm action should call the existing typed service:
  - frontend: `presetSelectionService.selectPreset({ sessionId, presetId })`
  - host command: `select_session_preset`
  - host persistence target: `session.json` `activePreset` and `activePresetName`
- Preserve the approved catalog constraint from shared contracts:
  - deterministic order
  - approved names from `src/shared-contracts/presets/presetCatalog.json`
  - bounded set of 1-6 customer presets
- Do not allow capture progression from the initial preset-selection surface while no preset has been explicitly chosen. This guard must exist in the reducer/provider flow, not only as button styling.
- Current fallback/mock preset behavior uses a default preset (`warm-tone`) when no session preset exists. Do not let that fallback masquerade as a confirmed customer choice during the initial binding flow.
- After confirmation succeeds, the active preset shown in capture-ready UI and capture-confidence snapshots must match the persisted manifest-backed selection.

### Architecture Compliance

- Keep React components free of direct Tauri `invoke` calls. `PresetScreen` and other UI components should emit intent only; adapter/service modules own host communication.
- Keep top-level routing unchanged. Story 2.2 is a state-driven progression inside the customer flow, not a new `/preset-confirm` or `/capture-entry` route.
- The Rust host remains the only durable owner of session manifest writes. Do not introduce browser-only preset persistence for the active session.
- Keep the customer surface free of internal diagnostics, preset-authoring terms, or branch-audit logic. Story 2.2 is booth-customer scope only.
- Preserve contract-first behavior: TypeScript Zod schemas and Rust DTO handling must stay aligned for preset selection payloads/results.
- Reuse the existing session-domain and capture-domain seams instead of introducing a generic global state store for preset binding.

### Library / Framework Requirements

- Current repo baselines from `package.json` and `src-tauri/Cargo.toml`:
  - React `^19.2.0`
  - React DOM `^19.2.0`
  - React Router `7.9.4`
  - Zod `^4.3.6`
  - `@tauri-apps/api` `^2.10.1`
  - `@tauri-apps/cli` `2.10.1`
  - `tauri` crate `2.10.3`
  - `tauri-plugin-store` `2`
- Continue using React 19 patterns already present in the repo, especially `useEffectEvent` and `startTransition`, where they help keep preset-confirmation side effects non-blocking and localized.
- Keep using Tauri v2's command boundary for frontend-to-host preset persistence. Do not introduce a second IPC style or direct filesystem reads in the frontend.
- Keep using Zod 4 for preset DTO/schema validation. Do not replace existing shared preset schemas with ad hoc checks.
- The Tauri Store plugin is already enabled in `src-tauri/capabilities/default.json` and can keep storing last-used preset preference, but that preference must not become the authoritative active preset for a new session.

### File Structure Requirements

- Primary frontend/UI files expected to change:
  - `src/customer-flow/screens/PresetScreen.tsx`
  - `src/customer-flow/components/PresetOptionCard.tsx`
  - `src/customer-flow/copy/presetSelectionCopy.ts`
  - add a dedicated preset-selection confirm test file if one does not exist yet
- Primary session-domain/state files expected to change:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/session-domain/services/presetSelection.ts`
  - `src/branch-config/services/presetSelectionStore.ts` only if last-used behavior needs to become suggestion-only
- Shared contract/catalog files to update only if state semantics change:
  - `src/shared-contracts/schemas/presetSchemas.ts`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/shared-contracts/presets/presetChangeSchemas.ts` only if active-preset flow contracts need clarification
- Capture/preset downstream files to verify for alignment:
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/customer-flow/components/PresetCatalogSheet.tsx`
  - `src/customer-flow/selectors/captureConfidenceView.ts`
  - `src/capture-adapter/host/presetChangeAdapter.ts`
  - `src/capture-adapter/host/fallbackPresetSessionState.ts`
  - `src/capture-adapter/host/cameraAdapter.ts`
- Rust host files to verify or update:
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/capture/camera_host.rs`
- Keep all work in these existing domain-first locations. Do not create a parallel `preset-flow` architecture for one story.

### Testing Requirements

- Add a new UI test for the initial preset-selection surface proving:
  - no confirm/continue action is enabled before a customer makes a selection
  - selecting a card updates the candidate state
  - confirming persists and advances only after the service succeeds
- Update service/reducer/provider coverage around:
  - `src/session-domain/services/presetSelection.spec.ts`
  - new or existing tests for `SessionFlowProvider`
  - reducer transitions in `src/session-domain/state/sessionReducer*.spec.ts`
- Verify the last-used preset preference path does not auto-bind a preset on new session entry.
- Keep or extend Rust-side persistence tests so `select_session_preset` still updates manifest `activePreset`/`activePresetName` correctly and returns typed failure envelopes for invalid preset/session inputs.
- Add or update integration coverage so the customer cannot reach capture-ready from the initial preset-selection seam without explicit selection and confirmation.

### Previous Story Intelligence

- The saved Story 1.4 artifact is the most relevant predecessor: it already frames `SessionFlowProvider` and `CustomerFlowScreen` as the correct handoff seam from active session to downstream customer flow. Story 2.2 should extend that seam instead of replacing it.
- There is no saved Story 2.1 artifact yet, but the current codebase already contains the equivalent structural pieces for catalog display. Reuse them as the display foundation, then tighten behavior for explicit confirmation.
- `PresetCatalogSheet.tsx` is future-capture preset-change UI. It can share card components with Story 2.2, but do not let its in-session change behavior redefine the stricter initial preset-binding requirement for a new session.

### Git Intelligence Summary

- Recent git history is still dominated by the repository reset and camera-state stabilization work, but the current checked-in source already proves this repo is not starting Story 2.2 from zero.
- The strongest salvage signals in the codebase are:
  - typed preset contracts and approved catalog asset already exist
  - host persistence for `select_session_preset` already exists
  - reducer/provider phases already distinguish `preset-selection`, `capture-loading`, and `capture-ready`
  - capture surfaces already know how to display an active preset after binding
- The main implementation trap is layering an explicit confirm step on top of the current "card click immediately persists" flow without untangling semantics. Replace that coupling cleanly instead of keeping both behaviors alive.

### Latest Technical Information

- Tauri v2 official docs still position Rust commands as the standard request/response bridge for frontend-to-host work. That matches the existing `presetSelectionService -> select_session_preset` path and should remain the only initial preset-binding persistence path. [Source: https://v2.tauri.app/develop/calling-rust/]
- Tauri's Store plugin documentation still supports debounced autosave behavior for local preference state, which fits the current `preset-selection.json` last-used preference service. Keep it as preference storage only, not session truth. [Source: https://v2.tauri.app/plugin/store/]
- React's official docs continue to position `useEffectEvent` and `startTransition` as the right tools for effect-driven event handlers and non-blocking UI transitions. That aligns with the repo's current session-flow implementation style and is preferable to ad hoc imperative orchestration. [Source: https://react.dev/reference/react/useEffectEvent] [Source: https://react.dev/reference/react/startTransition]
- Zod 4 remains the current official validation line and is still the correct basis for shared preset DTO parsing and contract guards in this repo. [Source: https://zod.dev/v4]

### Project Context Reference

- `_bmad-output/project-context.md` remains authoritative for implementation behavior:
  - no direct UI `invoke` calls
  - preserve typed cross-boundary DTOs
  - keep session folders and host-owned manifest writes as durable truth
  - avoid cross-session leakage
  - keep routes limited to top-level surfaces
- For Story 2.2 specifically, the highest-signal planning/runtime references are:
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`
  - `_bmad-output/implementation-artifacts/1-4-session-context-storage-and-next-surface-handoff.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/1-4-session-context-storage-and-next-surface-handoff.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/capabilities/default.json`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/components/PresetOptionCard.tsx`
- `src/customer-flow/components/PresetCatalogSheet.tsx`
- `src/customer-flow/copy/presetSelectionCopy.ts`
- `src/customer-flow/data/mvpPresetCatalog.ts`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/session-domain/services/activePresetService.ts`
- `src/branch-config/services/presetSelectionStore.ts`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetChangeSchemas.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/capture-adapter/host/presetChangeAdapter.ts`
- `src/capture-adapter/host/fallbackPresetSessionState.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/session-domain/services/presetSelection.spec.ts`
- `src/customer-flow/components/PresetCatalogSheet.spec.tsx`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/capture/camera_host.rs`
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri Store plugin docs: https://v2.tauri.app/plugin/store/
- React `useEffectEvent` docs: https://react.dev/reference/react/useEffectEvent
- React `startTransition` docs: https://react.dev/reference/react/startTransition
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `done`
- Primary implementation strategy: salvage and rework the existing preset-selection path instead of creating a second preset-binding architecture
- Primary risk: the current code conflates candidate selection, persisted active preset, and last-used preference
- Primary guardrail: no session may enter capture from the initial preset-selection seam without an explicit current-session choice and confirm action
- Dependency note: assumes Story 1.4's active-session handoff seam remains the canonical way a session reaches preset selection

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation completed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in this repository, so checklist intent was verified manually
- Verification: `pnpm test:run`
- Verification: `pnpm lint`
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml`

### Completion Notes List

- Story context was generated from the current planning artifacts, the active root-level repo implementation, git history, and current official React / Tauri / Zod documentation.
- The repo already contains meaningful Story 2.1/2.2 salvage material, but current behavior does not satisfy the approved Story 2.2 acceptance criteria because preset binding happens too early and without explicit confirmation.
- This story deliberately separates initial preset binding from later in-session preset changes so Story 3.4 behavior does not blur the stricter initial gating rules.
- The most important implementation correction is to stop treating default or last-used presets as already-confirmed current-session choices.
- Added an explicit confirm-only preset selection surface, including disabled continue gating and customer-safe selection guidance until a preset is chosen.
- Updated the session flow so card browse only stores candidate state, while confirm is the only path that persists through `presetSelectionService.selectPreset()` and advances to `capture-loading`.
- Added regression coverage across preset UI, session reducer, integration flow, gallery isolation, and Rust manifest persistence failure handling.
- Addressed review follow-ups by adding command-level preset failure coverage, customer-safe retry guidance for failed confirmation, and a reconciled audit file list.
- Follow-up review found that temporary readiness regressions still discard a confirmed preset for the same session, typed preset-selection failures are flattened into one retry message, the failed-confirm provider path remains uncovered, and the full frontend verification claim was not reproducible within the review window.
- Preserved the confirmed preset across readiness regressions for the active session, retained typed preset-selection failure envelopes in state, and translated session-integrity failures into customer-safe restart guidance.
- Added reducer and integration coverage for typed preset-confirm failures and for ready-waiting-ready regression recovery.
- Guarded preset confirm flow with active-session correlation so stale async responses no longer update reducer state or last-used preset settings after session clear.
- Added integration coverage for the stale in-flight confirm case and kept Story 2.2 reducer coverage aligned with the new sessionId guard.
- Fresh verification on 2026-03-13: `pnpm test:run`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml` all passed after closing the remaining typed preset-failure, timing/watcher, and stale-session follow-ups.
- Verification cleanup on 2026-03-13: replaced preset failure message parsing with typed repository/host error codes, removed unsafe typed-failure test casts, stabilized capture-confidence timing/watcher tests, and confirmed the stricter normalized-readiness contract in both code and story notes.
- Review-fix pass on 2026-03-13 aligned the readiness integration harness with the real preparation-to-preset handoff, removed the stale single-subscription expectation, and reconfirmed the preset-catalog fallback unit contract against the approved-fallback implementation.
- Fresh verification on 2026-03-13 after the review-fix pass: `pnpm test:run`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml` all passed.

### File List

- `_bmad-output/implementation-artifacts/2-2-preset-selection-and-active-preset-binding.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/customer-flow/copy/presetSelectionCopy.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/PresetSelectionSurface.tsx`
- `src/customer-flow/screens/PresetScreen.spec.tsx`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/preset-catalog/services/presetCatalogService.spec.ts`
- `sidecar/protocol/messages.schema.json`
- `src/capture-adapter/host/captureAdapter.ts`
- `src/session-domain/services/presetSelection.spec.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx`
- `src/session-domain/state/customerPreparationState.spec.ts`
- `src/session-domain/state/sessionReducer.story-2-2.spec.ts`
- `src/session-domain/state/sessionReducer.story-4-1.spec.ts`
- `src/session-domain/state/sessionReducer.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src-tauri/src/diagnostics/error.rs`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_repository.rs`
- `tests/contract/cameraContract.test.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `tests/integration/sessionPresetChangeFlow.test.tsx`

### Change Log

- 2026-03-12: Implemented explicit preset confirm flow, removed automatic session preselection, aligned capture progression with confirmed host persistence, and added regression coverage across frontend and Rust seams.
- 2026-03-13: Senior developer review requested changes, added AI review follow-ups, and returned the story to in-progress.
- 2026-03-13: Fixed review follow-ups by surfacing customer-safe preset retry guidance, adding command-level preset failure coverage, reconciling the file list, and returning the story to review.
- 2026-03-13: Follow-up senior review found remaining preset persistence, typed error handling, failure-path coverage, and frontend verification reproducibility issues; story returned to in-progress.
- 2026-03-13: Fixed the remaining preset regression and typed failure follow-ups, added failed-confirm integration coverage, updated verification notes with fresh command results, and returned the story to review.
- 2026-03-13: Follow-up senior review found a stale async preset-confirm session-boundary bug, missing stale-response coverage, and outdated full-suite verification notes; story returned to in-progress.
- 2026-03-13: Fixed the stale async preset-confirm session-boundary bug, prevented stale last-used preset writes, added in-flight clear regression coverage, and returned the story to review.
- 2026-03-13: Addressed the final review items for stale delete/session isolation and typed preset failures, repaired unrelated verification drift in gallery path isolation and readiness normalization tests, and reconfirmed `pnpm test:run`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- 2026-03-13: Replaced preset failure message parsing with typed repository error codes, removed unsafe typed-failure test casts, stabilized remaining timing/watcher regression tests, repaired the stale in-flight preset-change harness, and reconfirmed `pnpm test:run`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- 2026-03-13: Repaired the remaining review-era verification drift by aligning the readiness integration harness with the real preset-selection handoff, removing the stale readiness subscription assertion, revalidating preset-catalog fallback expectations, and reconfirming `pnpm test:run`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- 2026-03-13: Promoted Story 2.2 to `done` after final review close-out and fresh verification with `pnpm test:run`, `pnpm lint`, and `cargo test --manifest-path src-tauri/Cargo.toml` using an isolated Cargo target directory.

### Senior Developer Review (AI)

- Reviewer: GPT-5 Codex
- Date: 2026-03-13
- Outcome: Changes Requested

#### Findings

1. High: A confirmed preset is still lost if readiness temporarily drops after capture has already been unlocked. In `readiness_changed`, the reducer clears `selectedPresetId`, `activePreset`, and `pendingActivePresetId`, and once readiness returns the provider starts `preset_selection_started` with `selectedPresetId: null`, forcing the same session back through initial preset binding. That breaks the story requirement that the confirmed active preset stays aligned with downstream capture state for the active session. See [src/session-domain/state/sessionReducer.ts:288] and [src/session-domain/state/SessionFlowProvider.tsx:639].
2. Medium: The preset-confirm failure path now shows retry copy, but it still throws away the typed host envelope entirely. `confirmPresetSelection()` maps every non-OK result and thrown exception to the same `selectionRetryRequired` message, so retryability and session-integrity distinctions from the contract are lost at the UI boundary. That conflicts with the project-context rule to preserve typed host failure semantics. See [src/session-domain/state/SessionFlowProvider.tsx:251].
3. Medium: The new failure UX is not covered at the provider/integration seam. The current tests only cover the reducer action and presentational hint rendering; there is no test proving `confirmPresetSelection()` dispatches `preset_selection_failed` when `presetSelectionService.selectPreset()` returns a failure envelope. A regression in the actual async branch would not be caught. See [src/session-domain/state/sessionReducer.story-2-2.spec.ts:65], [src/customer-flow/screens/PresetScreen.spec.tsx:64], and [tests/integration/presetSelectionFlow.test.tsx:124].
4. Medium: The story still claims full frontend verification was completed, but `pnpm test:run` did not complete within the review window and timed out after 300 seconds, so that claim is not currently reproducible from the checked-in workspace. See [2-2-preset-selection-and-active-preset-binding.md:275].

#### Verification

- `pnpm test:run` (timed out after 300 seconds during review; full frontend verification not confirmed)
- `pnpm lint`
- `cargo test --manifest-path src-tauri/Cargo.toml`

#### Review Notes

- Acceptance Criteria 1 and 2 are implemented for the happy path, but the active-session preset still is not stable across readiness regressions inside the same session.
- Git-based file auditing remains limited because this worktree reports most of the repository as untracked, so the review relied on direct source inspection plus local verification commands.
- `pnpm lint` and `cargo test --manifest-path src-tauri/Cargo.toml` both passed in this review.

- Reviewer: GPT-5 Codex
- Date: 2026-03-13
- Outcome: Changes Requested

#### Findings

1. High: `confirmPresetSelection()` still has no stale-result guard. It captures the current session before `await presetSelectionService.selectPreset(...)`, but after the await it dispatches `preset_selection_succeeded` or `preset_selection_failed` unconditionally. If the active session is cleared or replaced while that request is in flight, the old response can still mutate the new reducer state and push the wrong session into preset-confirmed flow. The reducer currently accepts those actions without any session correlation check. See [src/session-domain/state/SessionFlowProvider.tsx:260], [src/session-domain/state/SessionFlowProvider.tsx:276], [src/session-domain/state/SessionFlowProvider.tsx:295], [src/session-domain/state/sessionReducer.ts:381], and [src/session-domain/state/sessionReducer.ts:436].
2. Medium: The current integration coverage still does not prove stale preset-confirm responses are ignored across a session reset or restart. Existing tests cover same-session happy path, readiness regression, and typed failure handling, and session-entry coverage separately checks explicit clearing, but there is no combined in-flight confirm boundary test. That leaves the async session-isolation bug above unguarded. See [tests/integration/presetSelectionFlow.test.tsx:191], [tests/integration/presetSelectionFlow.test.tsx:385], [tests/integration/presetSelectionFlow.test.tsx:507], and [tests/integration/sessionEntryFlow.test.tsx:57].
3. Medium: `confirmPresetSelection()` persists `saveLastUsedPresetId(...)` immediately after a successful host response and before any active-session correlation check. That means the same stale in-flight response can also overwrite the branch-level suggested preset for the next session, even if the current session has already been cleared or replaced. See [src/session-domain/state/SessionFlowProvider.tsx:289].

#### Verification

- `pnpm test:run`
- `pnpm lint`
- `cargo test --manifest-path src-tauri/Cargo.toml`

#### Review Notes

- Acceptance Criteria 1 and 2 are implemented for the intended customer flow and the previously failing verification suite now passes. The stale verification note was corrected during this review pass.
- The remaining issue is at the async session boundary: preset-confirm responses are not correlated to the still-active session before reducer updates are applied.
- Git-based file auditing remains limited because this worktree reports most of the repository as untracked, so the review relied on direct source inspection plus local verification commands.
