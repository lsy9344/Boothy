# Story 3.3: Current-Session Review and Deletion

Status: done

Story Key: `3-3-current-session-review-and-deletion`

## Summary

Expose a true current-session review surface inside the capture-ready booth flow so customers can inspect and delete only their own session photos without leaking foreign assets, breaking capture confidence, or bypassing the host-owned session manifest. This story must reuse the existing review rail, delete dialog, gallery contracts, and Rust thumbnail guard seams already present in the repo, while closing the current gaps around original-asset deletion and immediate audit recording.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want to review and delete only my current-session photos,
so that I can remove unwanted shots safely.

## Acceptance Criteria

1. Given the review surface is opened, when thumbnails are loaded, then only current-session photos are shown, and no cross-session assets are accessible.
2. Given a customer deletes a current-session photo, when the deletion is confirmed, then the original and derived assets are removed, and the session manifest and audit record are updated immediately.

## Tasks / Subtasks

- [x] Wire the current-session review surface into the active capture experience without creating a parallel flow. (AC: 1)
  - [x] Reuse the existing review UI pieces already present in the repo: `LatestPhotoReviewRail`, `ReviewScreen`, `DeletePhotoDialog`, and `reviewRailSelectors`.
  - [x] Add the missing orchestration in the customer/session seam so the review rail can be opened from the capture-ready surface and stay bound to `state.activeSession`, `state.reviewGallery`, and `state.selectedReviewCaptureId`.
  - [x] Keep review progression state-driven inside the existing `/customer` surface; do not add separate review routes or browser persistence for selected captures.

- [x] Load gallery data only from the active session manifest and preserve strict session isolation. (AC: 1)
  - [x] Add or expose a session-domain review service/provider path that calls the existing host-backed `captureAdapter.loadSessionGallery()` with `sessionId` and `manifestPath` from `state.activeSession`.
  - [x] Keep `thumbnailIsolation.ts` and the Rust `ThumbnailGuard` as the session-isolation gate; do not bypass them with raw filesystem reads in React.
  - [x] Ensure review refreshes preserve customer selection when the selected capture still exists and fall back to the host-selected or latest capture when it does not.

- [x] Make delete remove all capture assets required by the approved planning baseline. (AC: 2)
  - [x] Reuse the existing `delete_session_photo` command path and `DeleteSessionPhotoRequest/Response` contracts instead of inventing a second delete API.
  - [x] If Story 3.2 has already introduced a richer capture record, extend that canonical manifest shape; otherwise upgrade the current single-file `ManifestCaptureRecord` so one capture tracks both original and processed asset references needed for safe deletion.
  - [x] Delete both the original asset and the current-session derived/preview asset for the requested capture inside the host boundary, with rollback behavior if manifest persistence fails after file mutation begins.

- [x] Record an immediate deletion audit trail without depending on later operator epics. (AC: 2)
  - [x] Use the already-provisioned session-local `events.ndjson` artifact as the minimum audit record for photo deletion so Story 3.3 is independently completable now.
  - [x] Append one bounded session-local deletion event per successful delete, including at least `eventType`, `occurredAt`, `sessionId`, and `captureId`, while keeping customer-facing surfaces free of audit terminology.
  - [x] Do not block this story on the later Epic 6/Epic 7 SQLite diagnostics surface; if centralized logging is also added, keep the session-local event as the source-of-fact audit for this story.

- [x] Preserve capture-review UX guardrails and deletion safety in the booth shell. (AC: 1-2)
  - [x] Keep customer copy simple, current-session-scoped, and free of operator or internal diagnostic language.
  - [x] Reuse the existing modal focus trap behavior so enlarged photo review and delete confirmation preserve focus, support `Esc`, and restore prior focus when closed.
  - [x] Keep the active preset and timing trust anchors visible in the capture-ready shell; opening review must not collapse back into an unrelated editor-style workflow.

- [x] Add regression coverage across the TypeScript and Rust boundaries. (AC: 1-2)
  - [x] Extend contract tests for any manifest or gallery schema changes required to represent original-plus-derived asset deletion safely.
  - [x] Add or update integration coverage proving the booth only renders session-owned thumbnails, opens the review surface from the current session, and updates the rail after delete.
  - [x] Extend Rust thumbnail-guard tests to cover dual-asset deletion, out-of-root rejection, stale capture rejection, manifest rollback, and audit-event append behavior.

## Dev Notes

### Developer Context

- The planning baseline for Epic 3 is explicit: Story 3.3 is not a generic gallery feature. It is the bounded booth-customer review/delete capability for the active session only, and it must preserve session isolation while staying inside the capture workflow.
- Current repo reality already contains a substantial salvage foundation for this story:
  - `src/shared-contracts/dto/sessionGallery.ts` defines typed gallery and delete request/response contracts.
  - `src/capture-adapter/host/captureAdapter.ts` already exposes `loadSessionGallery()` and `deleteSessionPhoto()` with session-scoped response validation and preview-path normalization.
  - `src/session-domain/state/sessionReducer.ts` already has review gallery, selection, expanded review, and delete state transitions.
  - `src/customer-flow/components/LatestPhotoReviewRail.tsx`, `src/customer-flow/screens/ReviewScreen.tsx`, and `src/customer-flow/components/DeletePhotoDialog.tsx` already implement the main review UI pieces.
  - `src-tauri/src/commands/capture_commands.rs` and `src-tauri/src/export/thumbnail_guard.rs` already expose the host command boundary and current delete/gallery guard behavior.
- The missing part is orchestration. `CustomerFlowScreen.tsx` still renders only the capture shell in `capture-ready`, and `SessionFlowProvider.tsx` does not yet load review galleries or trigger delete flows. The story should connect existing pieces before inventing new abstractions.
- There is also one important data-model gap: the current manifest stores only one capture file reference (`fileName`) under `processedDir`. The approved Epic 3 acceptance criteria require deletion of both original and derived assets. If Story 3.2 has not already widened the capture record, Story 3.3 must do so before delete can satisfy the acceptance criteria honestly.
- The March 12, 2026 implementation-readiness review identified Story 3.3 as forward-dependent because it requires an immediate audit record while formal diagnostics/audit features appear later in Epic 6 and Epic 7. The clean resolution is to use the already-created session-local `events.ndjson` as the minimum deletion audit artifact now rather than waiting for the later operator analytics stack.
- Scope boundary:
  - In scope: current-session gallery load, review UI wiring, safe delete confirmation, original+derived asset removal, manifest update, and immediate session-local audit append.
  - Out of scope: operator diagnostics UI, centralized reporting dashboards, full-session export handling, or cross-session gallery search.

### Technical Requirements

- Treat `state.activeSession.sessionId` and `state.activeSession.manifestPath` as the only approved inputs for review gallery load and delete actions.
- Reuse the current gallery/delete contract surface unless the asset model must expand:
  - `SessionGalleryRequest`
  - `DeleteSessionPhotoRequest`
  - `DeleteSessionPhotoResponse`
- If the manifest capture shape must expand, change the canonical contract once and propagate it consistently through:
  - TypeScript manifest DTO/schema
  - Rust manifest structs
  - session repository tests
  - thumbnail guard logic
  Do not bolt on a second sidecar-only or UI-only capture metadata structure.
- Keep delete atomic from the customer’s perspective:
  - validate session binding
  - resolve all asset paths inside the active session root
  - remove all files for the capture
  - persist manifest changes
  - append the session-local audit event
  - roll back file removal if manifest persistence fails before the operation is considered successful
- Preserve session-owned selection and latest-photo semantics after delete:
  - if items remain, select the nearest surviving capture or the host-defined latest capture
  - if no items remain, the gallery should become empty cleanly
  - the review reducer should show success feedback only after host delete completes
- Keep session-scoped preview paths normalized through the adapter layer with `convertFileSrc()` or equivalent Tauri-safe asset conversion; React should not manually rewrite raw Windows paths.

### Architecture Compliance

- Preserve the architecture rule that session folders and `session.json` remain the durable truth. Review/delete state in React is a projection of manifest-backed host data, not a second source of truth.
- Keep React Router limited to top-level surfaces. The review surface must remain part of the existing booth customer state machine, not a new `/review` route.
- React components must stay free of raw Tauri `invoke()` calls and raw filesystem access. The provider/service layer should own gallery load and delete orchestration.
- Maintain strict session isolation at every layer:
  - host validates manifest-to-session binding
  - asset paths must stay inside the active session root
  - frontend rejects foreign-session gallery payloads through `thumbnailIsolation.ts`
  - UI must never merge current-session thumbnails with historical or foreign-session assets
- Keep customer-facing copy bounded and customer-safe. Audit, diagnostics, manifest paths, and raw host errors must not surface in the booth UI.
- Do not turn review into a general editor surface. The UX and PRD are explicit that customers can review and delete approved current-session photos, not manipulate detailed adjustments.

### Library / Framework Requirements

- Current workspace baselines relevant to this story:
  - `react`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
  - `rusqlite`: `0.38.0`
  - `libsqlite3-hotbundle`: `1.520000.0` (SQLite `3.52.0`)
- Follow the existing React 19 provider pattern. `SessionFlowProvider.tsx` already uses `useEffectEvent` for event-like side effects; prefer extending that approach for review gallery synchronization instead of adding imperative global listeners.
- Keep Tauri review/delete work on the current command path:
  - `load_session_gallery`
  - `delete_session_photo`
  Do not create duplicate command names for the same use case unless the canonical contract is intentionally replaced everywhere.
- Keep Zod as the TypeScript contract gate for gallery payloads, delete payloads, and any manifest schema evolution required by dual-asset deletion.
- If audit writes touch SQLite in addition to `events.ndjson`, keep that inside the host boundary. Frontend code should never open or write operational SQLite state directly.

### File Structure Requirements

- Expected primary implementation surfaces:
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/customer-flow/selectors/reviewRailSelectors.ts`
  - `src/customer-flow/components/LatestPhotoReviewRail.tsx`
  - `src/customer-flow/screens/ReviewScreen.tsx`
  - `src/customer-flow/components/DeletePhotoDialog.tsx`
  - `src/capture-adapter/host/captureAdapter.ts`
  - `src/export-pipeline/services/thumbnailIsolation.ts`
- Likely contract and session-model surfaces if asset tracking must expand:
  - `src/shared-contracts/dto/sessionGallery.ts`
  - `src/shared-contracts/dto/sessionManifest.ts`
  - `src/shared-contracts/schemas/manifestSchemas.ts`
  - `src/session-domain/services/sessionManifest.ts`
  - `src/session-domain/services/sessionPaths.ts`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_paths.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/export/thumbnail_guard.rs`
  - `src-tauri/src/commands/capture_commands.rs`
- Keep review-specific orchestration out of:
  - `src/App.tsx` beyond top-level route composition
  - `src/shared-ui/*` except shared presentation primitives
  - `src/branch-config/*` except existing configuration reads
- If a new session-domain service is needed for review/delete, place it under `src/session-domain/services/` or inject the existing adapter into `SessionFlowProvider`; do not bury the logic in a customer-flow component.

### Testing Requirements

- Extend or add frontend tests covering:
  - review gallery load on the active session only
  - thumbnail selection persistence across gallery refresh
  - review modal open/close behavior with focus trapping and `Esc`
  - delete confirmation flow and success feedback
  - empty-state behavior after the final capture is deleted
- Primary existing frontend test surfaces to reuse:
  - `src/session-domain/state/sessionReducer.review.spec.ts`
  - `src/customer-flow/screens/ReviewScreen.spec.tsx`
  - `src/customer-flow/components/DeletePhotoDialog.spec.tsx`
  - `src/customer-flow/components/LatestPhotoReviewRail.spec.tsx`
  - `src/capture-adapter/host/captureAdapter.spec.ts`
  - `tests/integration/sessionGalleryIsolation.test.tsx`
  - `tests/contract/sessionGallery.test.ts`
- Add or update Rust coverage in:
  - `src-tauri/tests/thumbnail_guard.rs`
  - `src-tauri/tests/session_manifest.rs`
  - `src-tauri/tests/session_repository.rs`
  to prove path safety, session binding, dual-asset delete behavior, audit append behavior, and rollback on manifest write failure.
- Avoid snapshot-only coverage. This story is boundary- and state-heavy; tests should assert concrete session IDs, capture IDs, selected capture transitions, and persisted side effects.

### Previous Story Intelligence

- No prior Epic 3 implementation-artifact story files exist yet in `implementation-artifacts`, so there is no same-epic story document to mine for learnings.
- The nearest practical baseline is the current salvage code and tests already in the repo:
  - gallery/delete contracts and adapter wiring exist
  - reducer review state exists
  - review UI components exist
  - Rust thumbnail guard tests already protect session-scoped delete basics
- Practical lesson: this story should finish the unfinished review/delete seam instead of replacing it with a new gallery architecture.

### Git Intelligence Summary

- Recent git history is still dominated by the March 7, 2026 greenfield reset and earlier camera-readiness work rather than Epic 3 feature delivery.
- Actionable implication:
  - favor current repo seams over older pre-reset assumptions
  - do not revive route-heavy or unstructured UI state patterns from older branches
  - keep host-owned session truth centralized because the older camera-readiness work is exactly where state ownership could have drifted

### Latest Tech Information

Verified against official sources on 2026-03-12:

- React 19.2 continues to document `useEffectEvent` as the escape hatch for effect-triggered logic that should read the latest state without turning the effect reactive. That fits the existing `SessionFlowProvider` pattern for review/delete synchronization work.
- Tauri v2 official docs still keep Rust commands as the standard request/response boundary for frontend-to-host work. Story 3.3 should extend the existing `load_session_gallery` and `delete_session_photo` commands rather than introducing a separate filesystem bridge.
- Tauri’s JavaScript core API continues to expose `convertFileSrc()` for converting app-local file paths into webview-safe URLs. Keep gallery preview/thumbnail path conversion in the adapter boundary, not in presentation components.
- Zod 4 remains the current official line and still emphasizes TypeScript-first schema validation. Any manifest/capture schema expansion for original-plus-derived asset tracking should stay Zod-validated on the frontend boundary.
- SQLite 3.52.0 includes a fix for a WAL-reset corruption case. Since the repo already bundles that line in the host, Story 3.3 should keep audit persistence host-side and avoid adding ad hoc client persistence for deletion records.

### Project Structure Notes

- Current implementation alignment:
  - `customer-flow` owns booth surface composition
  - `session-domain` owns reducer/provider state
  - `capture-adapter` owns Tauri-facing gallery/delete commands
  - `src-tauri/src/export/thumbnail_guard.rs` owns session-scoped gallery/delete validation
- Current implementation variance to correct:
  - review UI pieces exist but are not rendered from `CustomerFlowScreen.tsx`
  - provider/reducer state exists, but provider does not yet orchestrate review gallery load or delete
  - manifest/session path shape still assumes one processed capture file instead of explicit original-plus-derived asset tracking
  - `events.ndjson` is provisioned for each session but not yet used for Story 3.3’s required immediate delete audit

### Project Context Reference

- `_bmad-output/project-context.md` remains active context for this story.
- The most relevant rules from that file are:
  - keep React components away from direct Tauri invocation
  - preserve session folders as the durable source of truth
  - avoid cross-session leakage in UI state, manifests, and fixtures
  - keep routes limited to top-level surfaces
  - keep shared DTOs fully typed and Zod-validated on the TypeScript side

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/selectors/reviewRailSelectors.ts`
- `src/customer-flow/components/LatestPhotoReviewRail.tsx`
- `src/customer-flow/screens/ReviewScreen.tsx`
- `src/customer-flow/components/DeletePhotoDialog.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/shared-contracts/dto/sessionGallery.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/session-domain/services/sessionManifest.ts`
- `src/session-domain/services/sessionPaths.ts`
- `src/capture-adapter/host/captureAdapter.ts`
- `src/export-pipeline/services/thumbnailIsolation.ts`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/export/thumbnail_guard.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_paths.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/tests/thumbnail_guard.rs`
- `src-tauri/tests/session_manifest.rs`
- `src-tauri/tests/session_repository.rs`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `tests/contract/sessionGallery.test.ts`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri JavaScript core API reference (`convertFileSrc`): https://v2.tauri.app/reference/javascript/api/namespacecore/
- Zod 4 docs: https://zod.dev/v4
- SQLite 3.52.0 release log: https://sqlite.org/releaselog/3_52_0.html
- SQLite foreign key pragma docs: https://sqlite.org/pragma.html#pragma_foreign_keys

## Story Readiness

- Status: `ready-for-dev`
- Primary dependency: Story 3.2 should already establish capture persistence; if it did not yet introduce original-plus-derived capture metadata, this story must extend the canonical capture record before delete work proceeds.
- Reuse strategy: finish the existing review/delete seam instead of creating a second gallery architecture.
- Key risk to manage: current repo only models one processed asset per capture, while the acceptance criteria require original and derived asset deletion plus an immediate audit record.
- Sequencing resolution: satisfy the audit requirement through session-local `events.ndjson` now, without waiting for later operator analytics stories.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation target: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Workflow validator reference in instructions: `_bmad/core/tasks/validate-workflow.xml`

### Completion Notes List

- Story context generated from Epic 3.3 requirements, the March 12, 2026 implementation-readiness report, current repo seams, current sprint tracking, and official React/Tauri/Zod/SQLite sources.
- This story intentionally resolves the readiness-report sequencing issue by using the already-provisioned `events.ndjson` file as the minimum immediate deletion audit artifact.
- The document is written as a salvage-and-complete guide: wire the existing review/delete contracts, reducer state, UI components, and thumbnail guard together before creating new abstractions.
- The current capture asset-model gap is called out explicitly so downstream implementation does not falsely claim compliance while deleting only one processed file.
- Wired the existing customer review rail, review modal, and delete confirmation into the `capture-ready` booth surface through `SessionFlowProvider` and `CustomerFlowScreen`, keeping review state bound to the active session instead of introducing a parallel route.
- Added active-session gallery loading and deletion orchestration through `captureAdapter.loadSessionGallery()` and `captureAdapter.deleteSessionPhoto()`, preserving reducer-driven selection fallback and customer-safe success feedback.
- Expanded the canonical manifest capture record to track both original and processed asset filenames in TypeScript and Rust so host-side deletion can remove both assets without introducing a sidecar metadata model.
- Extended the Rust thumbnail guard delete path to validate both asset roots, roll back file mutations if manifest persistence fails, and append a bounded `photo_deleted` event to the session-local `events.ndjson` audit log.
- Added targeted regression coverage for manifest schema changes, session-isolated review/delete flow in the customer surface, and Rust dual-asset deletion plus audit behavior.
- Verification evidence captured during implementation: targeted Vitest suites for manifest contracts, review reducer state, customer capture screen behavior, customer flow entry behavior, and session-isolated review/delete all passed individually; `cargo test --test thumbnail_guard` passed; `pnpm lint` passed for the current workspace after the story changes.
- Follow-up review fixes now refresh the latest-photo confidence panel after a successful delete, preserve customer-facing delete feedback across the passive gallery refresh triggered by that confidence update, enforce that deletion audits append only to the canonical session-local `events.ndjson`, and align persisted original asset references with the `originals/<captureId>.nef` manifest convention already assumed by the host-side delete path.

### Change Log

- 2026-03-13: Implemented current-session review/delete orchestration in the booth flow, widened manifest capture asset tracking, added host-side dual-asset delete rollback plus session-local audit append, and updated focused frontend/Rust regression coverage.
- 2026-03-13: Fixed post-review gaps by refreshing latest-photo confidence after delete, preserving delete success feedback during the gallery refresh, enforcing the canonical `events.ndjson` audit target, and persisting original capture assets under `originals/<captureId>.nef`.
- 2026-03-13: Senior review fixes prevented capture ID reuse after deletions, preserved sidecar-written original/processed assets instead of overwriting them with host placeholders, and blocked foreign native gallery paths before they reach the customer UI.

### Senior Developer Review (AI)

- Reviewer: Noah Lee
- Date: 2026-03-13
- Outcome: Approve after fixes
- Fixed findings:
  - Prevented deleted capture slots from being reused by deriving the next capture identifier from the highest persisted capture sequence.
  - Updated the mock sidecar and host capture path so sidecar-written original and processed assets are recorded in the manifest without being overwritten by host-generated placeholder files.
  - Tightened the frontend gallery/delete isolation gate to reject native asset paths that fall outside the active session manifest root before conversion to previewable URLs.
- Verification:
  - `pnpm vitest run src/capture-adapter/host/captureAdapter.spec.ts`
  - `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts tests/integration/sessionGalleryIsolation.test.tsx`
  - `cargo test next_capture_id_uses_the_highest_existing_capture_sequence_instead_of_reusing_deleted_slots --lib --tests`
  - `cargo test --test mock_sidecar_integration --test session_repository --test thumbnail_guard`

### File List

- `_bmad-output/implementation-artifacts/3-3-current-session-review-and-deletion.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/capture-adapter/host/captureAdapter.ts`
- `src/capture-adapter/host/captureAdapter.spec.ts`
- `src/export-pipeline/services/thumbnailIsolation.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/shared-contracts/dto/cameraContract.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `tests/contract/manifestSchemas.test.ts`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `sidecar/mock/mock-camera-sidecar.mjs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/capture/sidecar_client.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/export/thumbnail_guard.rs`
- `src-tauri/tests/mock_sidecar_integration.rs`
- `src-tauri/tests/thumbnail_guard.rs`
- `src-tauri/tests/session_repository.rs`
- `src-tauri/tests/camera_contract.rs`
- `src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
