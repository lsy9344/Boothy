# Story 3.4: In-Session Preset Change for Future Captures

Status: in-progress

Story Key: `3-4-in-session-preset-change-for-future-captures`

## Summary

Extend the existing capture-ready preset-change seam so a customer can switch to another approved preset during an active session without rewriting earlier captures, mutating current-session review assets, or exposing any authoring controls. Reuse the current `SessionFlowProvider -> activePresetService -> select_session_preset` path and the host-owned manifest/capture-confidence contracts instead of inventing a second review-only preset workflow.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want to change the active preset during my session for future captures,
so that I can try a different approved look.

## Acceptance Criteria

1. Given an active session with captures already taken, when I select a different preset, then the new preset is applied only to future captures, and existing captures remain unchanged.
2. Given a preset change is applied, when I return to capture, then the UI shows the new active preset name, and no detailed editing controls are exposed.

## Tasks / Subtasks

- [x] Reuse the existing capture-ready preset-change path instead of creating a parallel flow. (AC: 1, 2)
  - [x] Keep in-session preset changes routed through `SessionFlowProvider.applyActivePresetChange()` and `session-domain/services/activePresetService.ts`.
  - [x] Continue using the typed `select_session_preset` host command through `capture-adapter/host/presetChangeAdapter.ts`; do not add direct `invoke(...)` calls to `CaptureScreen`, review UI, or other presentation components.
  - [x] Preserve the current no-op guard for selecting the already-active preset so the sheet closes cleanly without unnecessary host writes.

- [x] Persist only the session's active preset for future work and keep prior captures immutable. (AC: 1)
  - [x] Keep the host update limited to `activePresetName` and `activePreset` in the active session manifest via `src-tauri/src/session/session_repository.rs`.
  - [x] Do not rewrite existing processed assets, gallery entries, `latestCaptureId`, or previously rendered latest-photo confirmation data when the preset changes.
  - [x] Treat existing captures as historical session artifacts: the new preset becomes the source of truth only for later capture requests and later host snapshots.

- [x] Keep current-session review and latest-photo context stable across the preset switch. (AC: 1)
  - [x] Preserve the currently displayed latest-photo asset and any loaded review gallery data until a new capture or gallery refresh replaces them.
  - [x] Keep session-isolation guards active by continuing to validate gallery/delete responses through `thumbnailIsolation.ts` and the host thumbnail guard.
  - [x] Do not infer or relabel older captures with the newly selected preset name; if per-capture preset history is needed later, that requires an explicit contract change and is out of scope for this story.

- [x] Reflect the new preset name immediately in the capture surface without exposing editing controls. (AC: 2)
  - [x] Keep the capture surface using the existing preset selector sheet and current-preset summary card in `CaptureScreen.tsx`; do not introduce slider, curve, mask, or other authoring-style controls.
  - [x] Maintain the current `pendingActivePresetId` handoff so UI feedback remains aligned until the host/capture-confidence snapshot confirms the new preset.
  - [x] Ensure the preset sheet highlight, success toast, and visible active preset label all converge on the same approved preset identifier after a successful change.

- [x] Keep the implementation aligned with the approved preset catalog and typed contracts. (AC: 1, 2)
  - [x] Accept only preset identifiers defined by `shared-contracts/presets/presetCatalog.json` and enforced by `presetSchemas.ts` / `presetChangeSchemas.ts`.
  - [x] Continue using approved preset names from the catalog as the only allowed `displayName` / label values returned to the UI.
  - [x] Do not add branch-local overrides, ad hoc customer-visible preset labels, or hidden fallback presets outside the approved bounded catalog.

- [x] Add regression coverage for forward-only preset application and session isolation. (AC: 1, 2)
  - [x] Extend TypeScript tests around `captureFlowState`, `SessionFlowProvider`, and/or `CustomerFlowScreen` to prove a successful preset switch updates the active preset path without mutating existing latest-photo or review-gallery context.
  - [x] Keep or expand the capture flow integration test so it proves the sheet closes, the toast appears, the new preset label is shown, and previously displayed photo metadata remains unchanged until a later capture.
  - [x] Add or extend Rust tests around `select_session_preset` and gallery/manifest behavior to prove preset changes do not mutate capture ordering, latest-capture identity, or session binding.

### Review Follow-ups (AI)

- [x] Replaced the custom preset-change harness coverage with integration coverage that mounts `CustomerFlowContent` through the real `SessionFlowProvider` composition.
- [x] Documented the capture-surface pending-state wiring and test surfaces that were added while fixing the in-flight preset-change race.
- [x] Refreshed Story 3.4 verification notes so the Rust regression test is recorded as passing instead of carrying the stale `resolve_post_end_outcome_at` blocker note.
- [x] Strengthened the provider-level failed preset-change regression so it asserts the returned failure result instead of inferring failure only from unchanged state.
- [ ] [AI-Review][High] Preserve typed in-session preset-change failure codes through `presetChangeAdapter` and route session-integrity failures to restart-required customer guidance instead of collapsing every failure to generic retry copy.
- [ ] [AI-Review][Medium] Harden the async readiness and capture-confidence watcher setup in `SessionFlowProvider` so cleanup that runs before `watchReadiness()` / `watchCaptureConfidence()` resolves cannot leak stale live subscriptions across phase or session changes.
- [ ] [AI-Review][Medium] Add explicit regression assertions for review-gallery selection/state stability during in-session preset changes; current Story 3.4 tests prove latest-photo stability but do not verify the claimed review-gallery invariants.

## Dev Notes

### Developer Context

- The refreshed Epic 3 / FR-005 requirement is broader than a simple button swap. The product rule is: current-session review stays session-scoped, deletion stays bounded, and preset changes affect future captures only.
- Current repo reality already includes a substantial part of this story:
  - `src/customer-flow/screens/CaptureScreen.tsx` already exposes a preset-change sheet and success toast.
  - `src/customer-flow/screens/CustomerFlowScreen.tsx` already calls `applyActivePresetChange()` while keeping the capture surface state-driven.
  - `src/session-domain/state/SessionFlowProvider.tsx` already persists active-preset changes through the typed service layer.
  - `src-tauri/src/commands/session_commands.rs` and `src-tauri/src/session/session_repository.rs` already persist the session's active preset into the manifest.
  - `tests/integration/presetChangeFlow.test.tsx` already proves the UX expectation that older latest-photo metadata stays visible while the new preset applies to subsequent capture intent.
- That means this story should tighten and complete an existing implementation seam rather than replace it.
- There is no refreshed Story 3.3 implementation artifact yet, but the repo already carries the review/gallery contracts and reducer state needed for the 3.4 guardrails:
  - `shared-contracts/dto/sessionGallery.ts`
  - `sessionReducer` review actions
  - `thumbnailIsolation.ts`
  - `src-tauri/src/export/thumbnail_guard.rs`
- Important scope boundary:
  - In scope: active preset changes during an active session, forward-only application, current-session stability, and customer-safe UI feedback.
  - Out of scope: customer photo editing, per-capture preset history redesign, new authoring controls, new branch-local preset behavior, or solving Epic 3.3's later audit/logging dependency.
- Important contract limitation:
  - `sessionManifest` capture records currently store only `captureId`, `fileName`, and `capturedAt`.
  - They do not store a preset per capture.
  - Therefore, do not try to retroactively annotate old captures with the new preset in this story. That is a schema change, not a UI tweak.

### Technical Requirements

- Treat the active session manifest as the only durable source of truth for the currently selected preset during an active booth session.
- Preserve the existing request/response flow:
  - UI intent chooses a new approved preset id.
  - `activePresetService` validates the request with Zod.
  - `presetChangeAdapter` calls `select_session_preset`.
  - Rust persists `activePresetName` and `activePreset`.
  - UI state converges when the command result and/or capture-confidence snapshot reflect the new preset.
- Keep the "future captures only" rule concrete:
  - existing gallery items remain bound to the files already written for the session
  - existing latest-photo confirmation remains valid until another capture supersedes it
  - previously captured assets are not regenerated, relabeled, or re-ordered because the active preset changed
- Continue using approved catalog-derived labels for all customer-visible preset names.
- If review entry points are added or completed while implementing this story, they must call the same preset-change service path and honor the same session-isolation constraints. Do not create a review-specific preset mutation path.
- Preserve `pendingActivePresetId` semantics so the frontend can distinguish:
  - requested preset not yet reflected in the latest host snapshot
  - confirmed active preset now reflected in the capture-confidence snapshot
- Any failure must remain customer-safe:
  - no raw manifest paths
  - no internal authoring vocabulary
  - no sidecar/native error text on customer surfaces

### Architecture Compliance

- Keep the session folder and manifest as durable booth truth. Do not introduce browser-local persistence, route params, or ad hoc cache files for active preset changes.
- Keep React Router limited to top-level surfaces. Preset change remains part of the state-driven customer flow, not a new route.
- React components must stay out of direct Tauri and filesystem access. Host communication continues through typed adapter/service modules.
- Preserve the architecture rule that customer-visible state translation stays centralized and free of diagnostics or authoring terminology.
- Keep current-session isolation intact:
  - review/gallery operations must remain bound to the active `sessionId`
  - thumbnail paths must remain inside the active session root
  - preset changes must not cause any cross-session gallery mixing or reuse
- Do not blur booth customer behavior with internal preset-authoring behavior. This story changes approved preset choice only; it must not surface editing primitives from the authoring domain.

### Library / Framework Requirements

- Current workspace baselines in the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
- Official-source verification performed on 2026-03-12:
  - React 19.2 continues to document `useEffectEvent` as the right fit for event-like logic that should read fresh state from effects. Follow the existing provider pattern rather than introducing callback registries or a global event bus.
  - Tauri v2 official guidance still uses commands as the normal frontend-to-host request/response boundary. Keep preset mutation on `select_session_preset`; do not bypass it with direct frontend file writes.
  - Zod 4 remains the current stable docs line and should remain the TypeScript-side gate for preset-change request/result validation.
- Do not add any new client persistence library for this story. The stack already has the correct split:
  - manifest durability in the host/session layer
  - optional branch-local remember-last-preset storage in the Store plugin
  - no second customer-flow persistence mechanism

### File Structure Requirements

- Expected primary implementation surfaces:
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/session-domain/services/activePresetService.ts`
  - `src/capture-adapter/host/presetChangeAdapter.ts`
- Current-session review / isolation surfaces to inspect or touch only if needed for consistency:
  - `src/shared-contracts/dto/sessionGallery.ts`
  - `src/export-pipeline/services/thumbnailIsolation.ts`
  - `src/customer-flow/selectors/reviewRailSelectors.ts`
  - `src/capture-adapter/host/captureAdapter.ts`
  - `src/customer-flow/screens/ReviewScreen.tsx`
- Host/contract files likely relevant:
  - `src/shared-contracts/presets/presetChangeSchemas.ts`
  - `src/shared-contracts/schemas/presetSchemas.ts`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/export/thumbnail_guard.rs`
- Test surfaces likely needing updates:
  - `tests/integration/presetChangeFlow.test.tsx`
  - `tests/integration/sessionGalleryIsolation.test.tsx`
  - `src/session-domain/state/captureFlowState.spec.ts`
  - `src/session-domain/state/sessionReducer.review.spec.ts`
  - `src/session-domain/services/activePresetService.ts` companion tests if needed
  - `src-tauri/tests/camera_contract.rs`
  - `src-tauri/tests/thumbnail_guard.rs`
- Keep session/preset behavior out of:
  - `src/shared-ui/*` beyond presentation-only changes
  - `src/branch-config/*` except existing last-used preset storage
  - new customer route trees or new authoring-domain files

### Testing Requirements

- Add or keep frontend regression coverage for the approved customer behavior:
  - open preset selector during capture-ready state
  - change to another approved preset
  - close the sheet immediately on success
  - show the customer-safe toast
  - show the new active preset name
  - keep previously displayed latest-photo metadata intact until a new capture arrives
- Add or update state-level coverage so preset changes do not accidentally wipe:
  - `captureConfidence`
  - review gallery selection
  - session isolation guards
  - pending preset reconciliation
- Add or update host tests so `select_session_preset` proves:
  - manifest session binding is preserved
  - `captures` collection is not mutated
  - `latestCaptureId` remains unchanged
  - only approved preset identifiers are accepted
- If any review/gallery code is touched, keep isolation assertions explicit. Tests should fail on foreign-session items or mismatched `sessionId` values rather than relying on snapshots alone.

### Previous Story Intelligence

- No refreshed Epic 3.3 story artifact is present yet in `implementation-artifacts`.
- The practical predecessor intelligence comes from the repo itself:
  - the capture-ready preset sheet is already implemented
  - review/gallery state and host isolation guards already exist
  - preset selection already uses typed approved-catalog contracts
- Treat those seams as established precedent and extend them carefully rather than rebuilding them.

### Git Intelligence Summary

- Recent history remains dominated by the greenfield reset and camera-state stabilization work:
  - `06ed2b7` restructured the repository around the new BMAD planning package.
  - Earlier camera-focused commits show the same recurring lesson: UI truth must stay derived from host-normalized state, not from scattered local assumptions.
- Actionable guidance for this story:
  - keep preset mutation centralized in the session-domain/provider seam
  - let host snapshots confirm the effective preset
  - avoid local UI shortcuts that would make capture/review state drift from manifest truth

### Latest Tech Information

Verified against official docs on 2026-03-12:

- React 19.2 official guidance continues to support `useEffectEvent` for effect-driven event logic that must observe fresh state without forcing unnecessary reactive dependencies. This matches the current `SessionFlowProvider` pattern and should remain the default approach for preset-change side effects.
- Tauri v2 official "Calling Rust" guidance still positions commands as the standard request/response boundary between the frontend and Rust host. Keep `select_session_preset` as the preset-change entry point.
- Zod 4 remains the current official docs line. Continue using it to validate preset-change requests/results and approved preset catalog constraints on the TypeScript boundary.

### Project Structure Notes

- The repo already follows the domain-first split needed for this story:
  - `customer-flow` owns customer-screen composition
  - `session-domain` owns active session and preset change state
  - `capture-adapter` owns typed host-facing preset/capture operations
  - `src-tauri/src/session` owns manifest persistence
  - `src-tauri/src/export` owns session-scoped gallery safety
- There is one meaningful structure/contract caveat to keep visible:
  - preset changes are already implemented in the capture-ready surface before the refreshed 3.3 story artifact exists
  - do not "fix" that by moving the feature into a review-only surface
  - instead, make the current capture-ready implementation the canonical path while preserving compatibility with the review/gallery contracts Epic 3 uses

### Project Context Reference

- `_bmad-output/project-context.md` remains active context for this story.
- The most relevant rules from that file are:
  - keep React components away from direct Tauri invocation
  - preserve session folders as the durable source of truth
  - avoid cross-session leakage in UI state, gallery queries, and fixtures
  - keep routes limited to top-level surfaces
  - keep shared DTOs fully typed and validated with Zod at the TypeScript boundary
- Story 3.4 should be implemented as a direct extension of those rules, not as an exception to them.

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/validation-report-2026-03-12.md`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/screens/ReviewScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/session-domain/state/captureFlowState.ts`
- `src/session-domain/services/activePresetService.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/capture-adapter/host/presetChangeAdapter.ts`
- `src/capture-adapter/host/captureAdapter.ts`
- `src/shared-contracts/presets/presetChangeSchemas.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/shared-contracts/dto/sessionGallery.ts`
- `src/export-pipeline/services/thumbnailIsolation.ts`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/export/thumbnail_guard.rs`
- `tests/integration/presetChangeFlow.test.tsx`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `src/session-domain/state/sessionReducer.review.spec.ts`
- `src-tauri/tests/camera_contract.rs`
- `src-tauri/tests/thumbnail_guard.rs`
- React 19.2 release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `in-progress`
- Scope: approved in-session preset change, forward-only application, and session-isolated capture/review stability
- Reuse strategy: extend the current capture-ready preset-change seam instead of inventing another workflow
- Contract sensitivity: medium-high because manifest preset fields, capture-confidence snapshots, and session-gallery isolation all intersect here
- Key guardrail: do not backfill historical capture preset metadata or expose authoring controls while satisfying the future-captures-only rule

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation must be performed manually
- Verified Story 3.4 targeted frontend behavior with `pnpm test:run tests/integration/presetChangeFlow.test.tsx tests/integration/sessionPresetChangeFlow.test.tsx src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx`
- Re-verified Story 3.4 review follow-up coverage with `pnpm test:run tests/integration/presetChangeFlow.test.tsx tests/integration/sessionPresetChangeFlow.test.tsx src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx src/customer-flow/components/PresetCatalogSheet.spec.tsx`
- Verified the capture-surface pending-state wiring with `pnpm lint -- src/session-domain/state/SessionFlowProvider.tsx src/customer-flow/screens/CustomerFlowScreen.tsx src/customer-flow/screens/CaptureScreen.tsx src/customer-flow/components/PresetCatalogSheet.tsx tests/integration/sessionPresetChangeFlow.test.tsx`
- Verified production compilation with `pnpm build`
- Verified failed preset-change retry guidance with `pnpm test:run tests/integration/sessionPresetChangeFlow.test.tsx`
- Re-verified provider-level preset-change state handling with `pnpm test:run src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx`
- Re-verified provider-level failed-result semantics for rejected in-session preset changes with `pnpm test:run src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx`
- Verified linting with `pnpm lint`
- Re-verified the Rust manifest immutability guardrail with `cargo test --lib select_session_preset_preserves_existing_captures_and_latest_capture_identity`
- Re-verified the full Rust lib suite with `cargo test --lib`
- Fresh provider-level Story 3.4 regression rerun with `pnpm test:run src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx` now passes
- Fresh full frontend regression rerun with `pnpm test:run` now passes (`61` files, `207` tests)
- Fresh lint verification with `pnpm lint` passes
- Fresh production compilation with `pnpm build` succeeds

### Implementation Plan

- Add a failing integration regression that proves rejected in-session preset changes keep the selector open and show customer-safe retry copy.
- Route failed in-session preset changes back into existing customer-safe preset-selection copy instead of swallowing the failure silently.
- Refresh Story 3.4 verification notes with fresh targeted/full Rust results and the current unrelated frontend test failures so the story record stays accurate.

### Completion Notes List

- Story context was generated from the refreshed Epic 3.4 requirement, current repo seams, readiness/validation artifacts dated 2026-03-12, recent git history, and official React / Tauri / Zod documentation.
- The story intentionally treats the already-landed capture-ready preset sheet and host preset persistence flow as the canonical baseline to extend.
- The document calls out the current contract limitation that captures do not yet store a per-capture preset identifier, so developers do not accidentally overreach into schema redesign while implementing the future-captures-only behavior.
- Updated `SessionFlowProvider.applyActivePresetChange()` to reconcile the visible preset from the typed host result instead of assuming the requested preset id is authoritative.
- Added provider-level regression coverage that proves successful in-session preset changes keep the latest-photo snapshot stable until a later host update and keep same-preset selection as a no-op.
- Added provider-local in-flight preset-change guarding plus capture-surface disable wiring so rapid repeat taps cannot issue overlapping host writes for the active session.
- Reworked Story 3.4 integration coverage to mount the real `CustomerFlowContent` composition instead of a custom harness, so `CustomerFlowScreen` wiring and `SessionFlowProvider` preset reconciliation are exercised together.
- The Rust `select_session_preset` immutability regression remains in place and now re-verifies cleanly in both the targeted test and the current full Rust lib suite.
- Hardened `applyActivePresetChange()` so stale in-flight preset-change responses are ignored once the active session id no longer matches the host result.
- Blocked customer capture and preset-sheet dismissal while an in-session preset change is pending so the next shot cannot slip through with the previous preset.
- Expanded Story 3.4 regression coverage to assert pending capture gating, disabled preset-sheet close controls, and stale preset-change response isolation.
- Added customer-safe retry feedback for failed in-session preset changes while keeping the preset sheet open and leaving the active preset card unchanged.
- Added failure-path regression coverage for in-session preset changes at the customer-flow integration seam.
- Strengthened the provider-level rejected preset-change regression so it asserts the returned `false` result alongside unchanged visible preset state.
- Fixed the capture-confidence resubscription gap by re-running the provider capture-confidence sync when the session phase enters `capture-loading`, which restores the expected Story 3.4 provider transition into `capture-ready`.
- Replaced the stale frontend blocker note with fresh verification evidence: the Story 3.4 provider suite now passes, the full Vitest suite passes (`61` files, `207` tests), `pnpm lint` passes, and `pnpm build` succeeds.

### Senior Developer Review (AI)

- Reviewer: Noah Lee
- Date: 2026-03-13
- Outcome: Changes Requested
- Findings:
  - High: `presetChangeAdapter` discards the typed `error_code` from `select_session_preset` failures and throws a generic `Error(message)`, so `CustomerFlowScreen` can only show retry copy and cannot surface the existing restart-required guidance for session-integrity failures during in-session preset changes.
  - Medium: `SessionFlowProvider` can leak stale readiness/capture-confidence subscriptions when cleanup runs before the async `watchReadiness()` / `watchCaptureConfidence()` calls resolve, because the late unsubscribe handles are never torn down after phase/session churn.
  - Medium: Story 3.4 claims regression coverage for preserving review-gallery context, but the current provider/integration tests assert latest-photo stability only and do not verify review-gallery selection or state retention explicitly.
- Verification:
  - Reviewed Story 3.4 source surfaces in `SessionFlowProvider`, `CustomerFlowScreen`, `PresetCatalogSheet`, `session_repository.rs`, and the Story 3.4 integration/provider test files.
  - Cross-checked the story tasks and file list against the current repository state and the host command enforcement in `src-tauri/src/commands/session_commands.rs`.

### File List

- `_bmad-output/implementation-artifacts/3-4-in-session-preset-change-for-future-captures.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/customer-flow/components/PresetCatalogSheet.spec.tsx`
- `src/customer-flow/components/PresetCatalogSheet.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx`
- `src-tauri/src/session/session_repository.rs`
- `tests/integration/sessionPresetChangeFlow.test.tsx`

## Change Log

- 2026-03-13: Reconciled in-session preset changes against the typed host result, added Story 3.4 provider regression coverage for forward-only preset application, and added a Rust manifest immutability regression for session preset changes.
- 2026-03-13: Fixed review follow-ups by documenting the full file list, refreshing verification notes with the passing Rust regression test, and replacing the custom integration harness with real customer-flow composition coverage.
- 2026-03-13: Addressed follow-up review findings by gating capture/sheet dismissal during in-flight preset changes, guarding stale preset-change responses by active session id, and extending Story 3.4 regression coverage.
- 2026-03-13: Added customer-safe retry guidance for failed in-session preset changes and refreshed Story 3.4 verification notes with fresh Rust and frontend regression results.
- 2026-03-13: Strengthened the rejected preset-change provider regression to assert the returned failure result and aligned the story readiness note with the current in-progress sprint state.
- 2026-03-13: Re-ran the full frontend regression suite, refreshed the failing-file list, and restored the story status to in-progress because unrelated timing/capture regressions still block completion.
- 2026-03-13: Fixed the provider capture-confidence resubscription gap, re-verified the Story 3.4 suite plus the full frontend/lint/build gates, and advanced the story back to review.
- 2026-03-13: Senior developer review reopened Story 3.4 with follow-ups for typed failure-code preservation, async watcher cleanup hardening, and missing explicit review-gallery stability assertions.
