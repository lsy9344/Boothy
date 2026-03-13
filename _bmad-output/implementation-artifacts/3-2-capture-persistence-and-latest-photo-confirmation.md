# Story 3.2: Capture Persistence and Latest-Photo Confirmation

Status: review

Story Key: `3-2-capture-persistence-and-latest-photo-confirmation`

## Summary

Turn the current capture-ready placeholder into a real session-scoped capture pipeline. A successful shot must persist a customer-visible asset under the active session, update manifest-backed capture truth, and refresh the latest-photo confidence panel within the current session while keeping the active preset name visible.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want my captures saved to my session and see the latest photo quickly,
so that I trust the booth captured my photo.

## Acceptance Criteria

1. Given an active session and active preset, when a capture completes successfully, then the capture is persisted in the session folder, and the latest photo preview is shown within 5 seconds for 95th-percentile captures.
2. Given the latest photo preview is displayed, when the capture surface is visible, then the active preset name remains visible, and only current-session assets are shown.

## Tasks / Subtasks

- [x] Add a typed capture request path that extends the existing host boundary instead of bypassing it. (AC: 1)
  - [x] Extend the shared capture contract under `src/shared-contracts/dto/` and `src-tauri/src/contracts/` so the frontend can request a capture without raw `invoke` calls in UI code.
  - [x] Reuse the existing sidecar protocol family in `src/shared-contracts/dto/cameraContract.ts` and `sidecar/protocol/examples/`; do not invent a second ad hoc capture event format when `capture.progress` fixtures already exist.
  - [x] Keep the capture action session-scoped by requiring the active `sessionId` and the currently selected preset context on the host boundary.

- [x] Persist successful captures into the active session manifest and customer-visible processed asset path. (AC: 1-2)
  - [x] Add a host-side capture flow in `src-tauri/src/commands/capture_commands.rs` and the capture domain that writes the latest successful image into the active session path and appends a `ManifestCaptureRecord`.
  - [x] Update the manifest so `latestCaptureId`, `captures[]`, and `activePresetName` remain coherent after each successful capture.
  - [x] Reuse the current `processedDir` + manifest-backed gallery model already consumed by `ThumbnailGuard`; do not silently introduce a second capture index, browser cache, or untracked file location in this story.

- [x] Drive the capture-ready UI from real capture action state instead of the current placeholder wiring. (AC: 1-2)
  - [x] Replace the current `captureActionDisabled` / `onCapture={() => undefined}` placeholder path in `CustomerFlowScreen` with a typed session-domain action that requests capture through an adapter or service.
  - [x] If an in-progress latest-photo state is shown, reuse `LatestPhotoState.updating` and the existing `mergeCaptureConfidenceState()` behavior rather than adding a parallel UI-only loading flag.
  - [x] Keep the capture surface focused on the latest-photo confidence panel; do not add review/delete controls here because Story 3.3 owns current-session review and deletion.

- [x] Make capture-confidence snapshots reflect manifest-backed latest-photo truth. (AC: 1-2)
  - [x] Update `src-tauri/src/capture/camera_host.rs` so `build_capture_confidence_snapshot()` can emit `ready` when the manifest has a valid latest capture and a previewable processed asset exists.
  - [x] Ensure snapshot `revision` changes when capture-relevant state changes. The current implementation ties revision to `operator_extension_count`, which is insufficient for latest-photo updates and can cause the frontend merge logic to ignore real capture changes.
  - [x] Preserve session isolation by rejecting or sanitizing any file path or snapshot payload that does not belong to the active session.

- [x] Add regression coverage for capture persistence, latest-photo confirmation, and session isolation. (AC: 1-2)
  - [x] Extend contract tests for capture request/progress messages and any new capture command DTOs.
  - [x] Add frontend adapter/provider/integration coverage proving the capture action becomes enabled in capture-ready state, a successful capture updates the latest-photo panel, and the preset label remains visible.
  - [x] Add Rust tests proving manifest append, `latestCaptureId` reconciliation, processed-file path validation, and snapshot generation for the latest current-session photo.

### Review Follow-ups (AI)

- [x] [AI-Review] Close the reopened host-truth validation gaps before returning Story 3.2 to review. (AC: 1-2)
  - [x] [AI-Review][High] Normalize helper/host manifest-path comparison so valid Windows capture paths do not fail on `\` vs `/` formatting drift. [`src-tauri/src/commands/capture_commands.rs:253`]
  - [x] [AI-Review][High] Enforce that the capture request preset context matches the active session manifest before persistence mutates `activePreset` truth. [`src-tauri/src/commands/capture_commands.rs:138`]
- [x] [AI-Review] Close the reopened watcher resilience and contract-mirroring gaps before returning Story 3.2 to review. (AC: 1-2)
  - [x] [AI-Review][Medium] Make capture-confidence watching recover or surface failure instead of silently terminating after repeated snapshot read errors. [`src-tauri/src/commands/capture_commands.rs:98`]
  - [x] [AI-Review][Medium] Mirror the helper-facing capture request/success protocol in Rust contract DTOs instead of leaving that boundary hidden in private sidecar-client structs. [`src-tauri/src/contracts/dto.rs:283`]

## Dev Notes

### Developer Context

- The current repo already contains most of the confidence-surface scaffolding this story should extend:
  - `src/customer-flow/screens/CaptureScreen.tsx` and `src/customer-flow/components/LatestPhotoPanel.tsx` already render the capture confidence surface and keep the active preset visible next to the latest-photo panel.
  - `src/session-domain/state/SessionFlowProvider.tsx` already loads and watches `CaptureConfidenceSnapshot` data once the session enters `capture-loading` / `capture-ready`.
  - `src/customer-flow/selectors/captureConfidenceView.ts` and `src/session-domain/state/captureConfidenceState.ts` already encode the `empty` / `updating` / `ready` latest-photo model plus session-isolation merge rules.
  - `src-tauri/src/export/thumbnail_guard.rs` already proves the project expects manifest-backed, session-scoped processed assets for gallery/review behavior.
- The real gaps are specific and should stay specific:
  - `CustomerFlowScreen` still passes `captureActionDisabled` and `onCapture={() => undefined}`.
  - `src-tauri/src/capture/camera_host.rs` currently returns `LatestPhotoState::Empty` for all manifest-backed capture-confidence snapshots.
  - There is no typed capture request command yet, so the UI cannot ask the host to create a new capture.
- Scope boundary:
  - In scope: requesting a capture, persisting the successful result into the active session, surfacing the latest current-session photo, and keeping the active preset visible.
  - Out of scope: full review rail behavior, photo deletion UX, export/handoff logic, or redesigning readiness gating. Story 3.3 owns review/delete and Story 4 owns post-end flows.
- Dependency note:
  - This story assumes the active-session manifest path from Stories 1.3/1.4 and the active-preset persistence from Epic 2 remain the canonical capture context.
  - There is no existing implementation-artifact file for Story 3.1 yet, so this story should reuse the current readiness seam in `SessionFlowProvider` rather than waiting for a separate readiness-state rewrite.

### Technical Requirements

- Keep one capture source of truth per session:
  - successful capture metadata must end up in the active session manifest
  - customer-visible latest-photo files must live under the active session's tracked processed asset path
  - frontend latest-photo rendering must come from the typed snapshot/adapter path, not from component-local blobs or temporary untracked caches
- The capture request boundary must stay typed end to end:
  - add or extend DTOs under `src/shared-contracts/dto/`
  - parse them with Zod on the TypeScript side
  - mirror them in Rust under `src-tauri/src/contracts/`
  - avoid raw `unknown` payload handling in UI or host command code
- Successful capture persistence must update these manifest invariants together:
  - append one `ManifestCaptureRecord`
  - set `latestCaptureId` to the new capture
  - keep `activePresetName` and `activePreset` aligned with the preset applied for that capture
  - preserve `sessionId` binding and normalized wire paths
- `CaptureConfidenceSnapshot.revision` must advance on every latest-photo-relevant change. Do not keep using `manifest.timing.operator_extension_count` as the only revision source because capture updates will otherwise be dropped by `mergeCaptureConfidenceState()`.
- If the implementation surfaces an intermediate capture-in-progress state, reuse:
  - `LatestPhotoState.updating`
  - `mergeCaptureConfidenceState()`
  - `selectCaptureConfidenceView()`
  instead of creating a new parallel view model for "capturing".
- Keep preset visibility explicit on the capture surface:
  - the latest-photo confirmation must not replace or hide the active preset label
  - `activePreset` in the snapshot should remain derived from the manifest-host truth, not inferred from stale UI state alone
- Continue enforcing session isolation:
  - all preview or processed file paths must resolve inside the active session root
  - snapshot payloads must never expose a capture whose `sessionId` differs from the active session
  - do not allow foreign or out-of-root file names into manifest capture records

### Architecture Compliance

- Preserve the architecture rule that the host and the session folder own capture truth. React may request capture and render the resulting snapshot, but it must not become the authoritative owner of latest-photo state.
- Keep React Router limited to top-level surface entry. Do not introduce route-based capture progression such as `/capture/processing` or `/capture/latest`; the capture flow remains state-driven inside the current customer surface.
- React/UI code must keep using adapters and services. Do not call new capture commands directly from `CaptureScreen.tsx`, `LatestPhotoPanel.tsx`, or any presentation-only component.
- Keep the capture surface booth-safe and copy-light:
  - latest-photo confidence belongs on the capture surface
  - review/delete affordances do not
  - diagnostics, raw device errors, filesystem paths, and preset-authoring language remain out of customer UI
- The current implementation path grammar is still `session.json`, `events.ndjson`, `export-status.json`, and `processed/`. The architecture document discusses a richer long-term session tree, but this story should not widen the path contract unless the manifest schema, session-path helpers, Rust tests, and frontend contracts are all updated together.
- Continue following the existing direction of data flow:
  - customer-flow UI
  - session-domain state
  - typed host adapter
  - Rust host command/domain
  - session manifest / processed asset
  - capture-confidence snapshot back to UI

### Library / Framework Requirements

- Current workspace baselines from `package.json`:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - `@tauri-apps/plugin-shell`: `~2.3.5`
  - `zod`: `^4.3.6`
  - `vite`: `^7.3.1`
- Use the existing React 19 pattern already present in `SessionFlowProvider.tsx`, especially `useEffectEvent`, for channel or polling callbacks that should not resubscribe on every render.
- Follow Tauri v2 command guidance for actual capture execution:
  - a real capture request should be an async host command because it can touch IO, helper orchestration, and manifest persistence
  - ordered updates back to the frontend should keep using channel or snapshot patterns already wrapped by `cameraAdapter.ts`
- If sidecar execution changes as part of the capture path, keep it aligned with Tauri's official sidecar model. Do not hardcode machine-specific helper paths or move process spawning into the frontend.
- Keep Zod 4 as the TypeScript boundary gate for any new capture request/result payloads or sidecar-message extensions.

### File Structure Requirements

- Expected primary TypeScript surfaces:
  - `src/capture-adapter/host/cameraCommands.ts`
  - `src/capture-adapter/host/cameraAdapter.ts`
  - `src/capture-adapter/host/cameraChannels.ts` only if capture-progress streaming requires a new typed wrapper
  - `src/shared-contracts/dto/cameraContract.ts`
  - `src/shared-contracts/dto/captureConfidence.ts`
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/customer-flow/selectors/captureConfidenceView.ts`
- Expected primary Rust surfaces:
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/capture/camera_host.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_paths.rs` only if the approved persisted capture path contract changes in the same story
- Sidecar contract and fixture surfaces:
  - `sidecar/protocol/examples/capture-started.json`
  - `sidecar/protocol/examples/capture-completed.json`
  - `sidecar/protocol/messages.schema.json` if the protocol union changes
- Keep new capture-specific code close to the existing capture boundary. Do not scatter capture persistence logic into:
  - `src/shared-ui/*`
  - `src/App.tsx`
  - `src/branch-config/*`
  - unrelated review/delete files except where they consume the same manifest-backed asset contract

### Testing Requirements

- Add or update contract tests for the capture request/progress protocol:
  - `tests/contract/cameraContract.test.ts`
  - any new DTO-specific tests under `tests/contract/`
  - keep the checked-in protocol fixtures in sync with the shared schema union
- Extend frontend adapter/state tests to prove:
  - capture-ready state can issue a capture request through the typed adapter
  - the active preset label remains visible while the latest photo updates
  - stale or foreign-session latest-photo payloads are ignored
  - monotonic capture revisions are required for UI updates
- Extend integration coverage around the real customer flow:
  - start session
  - select preset
  - enter capture-ready
  - request capture
  - receive latest current-session photo
  - remain on the latest-photo confidence surface without review/delete controls
- Add Rust-side coverage close to the host code for:
  - manifest append and `latestCaptureId` update
  - processed-file persistence inside the active session root
  - capture-confidence snapshot generation when a latest capture exists
  - rejection of out-of-root or wrong-session capture paths
- Prefer deterministic temp directories, fake timestamps, and explicit session IDs. Do not write tests that depend on developer machine state, wall-clock timing alone, or shared fixture folders across sessions.

### Previous Story Intelligence

- There is no existing implementation-artifact file yet for `3-1-readiness-states-and-capture-gating`, so there are no direct previous-story dev notes to inherit inside Epic 3.
- The practical predecessor knowledge comes from the current salvage baseline:
  - Stories 1.3 and 1.4 already established host-owned session identity and manifest persistence.
  - Epic 2 already established active preset selection and persistence through `select_session_preset`.
  - The current capture surface already separates latest-photo confidence from review/delete behavior, which should remain true here.
- Reuse, do not replace:
  - `mergeCaptureConfidenceState()` already contains isolation-aware merge behavior worth preserving.
  - `ThumbnailGuard` already codifies the session-scoped processed-file model that Story 3.3 review/delete will rely on.

### Git Intelligence Summary

- Recent history shows the repo is already in a salvage-and-align phase, not a blank greenfield build:
  - `06ed2b7` rebuilt the repository around the new BMAD planning package and current domain-first structure.
  - older camera-focused commits show the project repeatedly paid a cost when UI/device truth drifted apart.
- The actionable implication for this story is to extend the current `customer-flow` -> `session-domain` -> `capture-adapter` -> `src-tauri` seam instead of reviving older app trees or inventing a parallel capture store.
- Current checked-in tests already emphasize session isolation and latest-photo confidence. The new capture persistence path should plug into those same seams rather than introducing a separate temporary gallery model.

### Latest Tech Information

- Verified against official documentation on 2026-03-12:
  - React 19.2 official guidance continues to position `useEffectEvent` as the right tool for effect-fired event handlers. That matches the current `SessionFlowProvider` watcher pattern and should remain the mechanism for capture-confidence subscriptions.
  - Tauri v2 official calling-Rust guidance continues to favor async commands for work that touches IO or longer-running host behavior. A real capture command should therefore be async rather than a blocking synchronous command.
  - Tauri v2 frontend communication guidance continues to use channels for ordered host-to-frontend streaming. If capture progress is surfaced, it should follow the existing typed channel wrapper pattern already used for readiness and capture-confidence data.
  - Tauri's official sidecar guidance still favors bundled sidecars over machine-specific process paths. If this story changes actual helper execution, keep it on the packaged sidecar path.
  - Zod 4 remains the current major line and should continue to validate new capture payloads and protocol extensions before they cross the desktop boundary.

### Project Structure Notes

- The current domain-first repo layout already supports this story:
  - `customer-flow` owns screen composition
  - `session-domain` owns customer-session state and transitions
  - `capture-adapter` owns typed host communication
  - `src-tauri/src/capture` and `src-tauri/src/session` own capture truth and manifest persistence
- The current capture surface intentionally shows only the latest-photo confidence panel plus preset/timing anchors. Preserve that UX split; gallery review/delete should remain in Story 3.3.
- Important repo variance to keep visible:
  - architecture discusses a broader session folder model
  - current implementation and tests still use `processedDir` and manifest capture records as the concrete persisted contract
  - this story should either stay within that current contract or update every dependent schema/test in one coherent change, not halfway
- `_bmad-output/project-context.md` remains active execution guidance for this story. The most relevant rules are:
  - keep React components away from direct Tauri invocation
  - preserve session folders as durable truth
  - prevent cross-session leakage in UI state and fixtures
  - keep routes limited to top-level surfaces
  - keep shared DTOs typed and Zod-validated

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/project-context.md`
- `package.json`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/components/LatestPhotoPanel.tsx`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/session-domain/state/captureConfidenceState.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/capture-adapter/host/cameraCommands.ts`
- `src/shared-contracts/dto/cameraContract.ts`
- `src/shared-contracts/dto/captureConfidence.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_paths.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/export/thumbnail_guard.rs`
- `tests/contract/cameraContract.test.ts`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `src-tauri/tests/thumbnail_guard.rs`
- `src-tauri/tests/session_manifest.rs`
- `sidecar/protocol/examples/capture-started.json`
- `sidecar/protocol/examples/capture-completed.json`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri v2 calling frontend docs: https://v2.tauri.app/develop/calling-frontend/
- Tauri sidecars docs: https://v2.tauri.app/develop/sidecar/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Scope: real capture request, manifest-backed persistence, latest-photo confirmation, and current-session isolation only
- Reuse strategy: extend the existing capture-confidence and manifest seams instead of creating a second capture state model
- Dependency sensitivity: medium; this story depends on active-session truth and active-preset persistence already present in the current repo and should coordinate with Story 3.1 readiness gating if both land close together
- Critical hazards to avoid:
  - non-monotonic `CaptureConfidenceSnapshot.revision`
  - writing capture files outside the active session root
  - leaking review/delete UI into the capture surface
  - bypassing typed adapter/host contracts

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Implementation Plan

- Normalize helper-returned and host-resolved manifest paths through one canonical comparison path so valid Windows separator differences do not fail capture persistence.
- Reject capture requests whose `activePreset` payload diverges from the active session manifest before any capture persistence mutates host-owned preset truth.
- Rework capture-confidence watch failure handling so repeated snapshot read errors either recover cleanly or surface an actionable failure instead of silently stopping the watcher.
- Promote the helper-facing capture request and success payloads into explicit Rust DTOs and keep them synchronized with the shared protocol schema/tests.

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/dev-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation performed against `_bmad/bmm/workflows/4-implementation/dev-story/checklist.md`
- Verification evidence:
  - `pnpm vitest run tests/contract/cameraContract.test.ts src/capture-adapter/host/cameraAdapter.spec.ts src/customer-flow/screens/CaptureScreen.spec.tsx src/session-domain/state/sessionReducer.story-3-2.spec.ts src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx tests/integration/sessionGalleryIsolation.test.tsx`
  - `pnpm lint`
  - `pnpm build`
  - `cargo test --manifest-path src-tauri/Cargo.toml capture_commands --quiet`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test mock_sidecar_integration --quiet`
  - `cargo test --manifest-path src-tauri/Cargo.toml --quiet`
- Manual review-gap verification on 2026-03-13 confirmed the four reopened findings are still pending in `src-tauri/src/commands/capture_commands.rs` and `src-tauri/src/contracts/dto.rs`, so the story remains in-progress until those host/contract updates land.
- Fresh review-fix verification on 2026-03-13 completed successfully:
  - `cargo test --manifest-path src-tauri/Cargo.toml capture_commands --quiet`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test camera_contract --quiet`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test mock_sidecar_integration --quiet`
  - `cargo test --manifest-path src-tauri/Cargo.toml --quiet`
  - `pnpm vitest run tests/contract/cameraContract.test.ts`

### Completion Notes List

- Added a typed `camera.capture` request/result contract and surfaced it through the shared frontend adapter so the customer flow no longer relies on a disabled placeholder capture action.
- Implemented a host-side `request_capture` command that appends manifest-backed capture truth, writes session-scoped processed assets, and emits `capture.progress` updates using the existing protocol family.
- Updated capture-confidence snapshots and session state so the latest-photo panel can enter `LatestPhotoState.updating`, refresh to the newest current-session asset, and keep the active preset label visible.
- Added regression coverage across TypeScript contracts/adapter state and Rust host/session tests for manifest append, revision updates, and session-isolated latest-photo generation.
- Applied post-review fixes so Story 3.2 capture-ready UI stays latest-photo-only, the native command persists sidecar-reported capture truth consistently, and the capture-confidence watcher stops on manifest load failure instead of silently polling forever.
- Closed the follow-up review drift by syncing the shared sidecar capture request/success schema and fixtures, validating helper-returned capture identity against the assigned slot, and retrying transient capture-confidence snapshot read failures before tearing down the watcher.
- Reopened Story 3.2 after manual verification confirmed four remaining AI review gaps around Windows path normalization, preset-truth enforcement, watcher resilience, and explicit Rust sidecar DTO mirroring.
- ✅ Resolved review finding [High]: helper/host manifest path comparison now normalizes separator-only Windows drift before capture validation fails.
- ✅ Resolved review finding [High]: host capture persistence now rejects payload preset context that does not match the active session manifest truth.
- ✅ Resolved review finding [Medium]: capture-confidence watch no longer terminates permanently after repeated snapshot read failures and surfaces the condition through host logging while retrying.
- ✅ Resolved review finding [Medium]: helper-facing capture request and success payloads are now explicit Rust contract DTOs used by the sidecar client and covered by contract tests.

### Follow-up Notes List

- Story 3.2 review follow-ups were re-verified on 2026-03-13 and all four reopened findings are now resolved.
- The top-level `Status: review` line and `_bmad-output/implementation-artifacts/sprint-status.yaml` are the current authoritative state for this story.
- `request_capture()` now validates payload preset context against the host-owned manifest preset before persistence mutates session truth.
- `validate_sidecar_capture_success()` now normalizes manifest paths before comparing helper output with the host-resolved session manifest location.
- `watch_capture_confidence()` now keeps retrying after repeated snapshot read failures and surfaces threshold crossings through host logging instead of silently dying.
- Rust contract DTOs now model the helper-facing capture request/success payloads explicitly and the contract round-trip is covered in `src-tauri/tests/camera_contract.rs`.

### File List

- `_bmad-output/implementation-artifacts/3-2-capture-persistence-and-latest-photo-confirmation.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `sidecar/mock/mock-camera-sidecar.mjs`
- `sidecar/protocol/examples/capture-request.json`
- `sidecar/protocol/examples/capture-success.json`
- `sidecar/protocol/messages.schema.json`
- `src/capture-adapter/host/cameraAdapter.spec.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/capture-adapter/host/cameraCommands.ts`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.story-3-2.spec.ts`
- `src/session-domain/state/sessionReducer.ts`
- `src/shared-contracts/dto/cameraContract.ts`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/capture/sidecar_client.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src-tauri/tests/camera_contract.rs`
- `src-tauri/tests/mock_sidecar_integration.rs`
- `src-tauri/tests/session_repository.rs`
- `tests/contract/cameraContract.test.ts`
- `tests/integration/sessionGalleryIsolation.test.tsx`

## Senior Developer Review (AI)

- Date: 2026-03-13
- Outcome: Changes Requested
- Summary: Story 3.2 still has two high-severity contract/host-truth gaps and two medium-severity robustness/contract-mirroring gaps. The current implementation can reject valid Windows helper results because manifest-path comparison is string-format sensitive, and it still lets stale capture preset payloads overwrite manifest preset truth on the host boundary.
- Findings:
  - High: Normalize helper/host manifest-path comparison for Windows path separators. [`src-tauri/src/commands/capture_commands.rs:253`]
  - High: Validate requested capture preset context against the active session manifest before persistence. [`src-tauri/src/commands/capture_commands.rs:138`]
  - Medium: Capture-confidence watch can die permanently after repeated read failures without provider recovery. [`src-tauri/src/commands/capture_commands.rs:98`]
  - Medium: Rust contracts still do not explicitly mirror the helper-facing capture request/success DTOs used by the sidecar boundary. [`src-tauri/src/contracts/dto.rs:283`]

## Change Log

- 2026-03-13: Implemented typed capture requests, manifest-backed capture persistence, latest-photo snapshot refresh, and regression coverage for Story 3.2.
- 2026-03-13: Closed AI review findings by removing capture-surface review/delete controls, validating sidecar capture truth against the active session manifest, and updating mock/integration coverage.
- 2026-03-13: Synced the shared sidecar capture protocol schema/fixtures with the real helper request-success payloads and hardened watcher/capture validation against helper drift and transient manifest reads.
- 2026-03-13: Re-review reopened Story 3.2 as in-progress and added AI review follow-ups for Windows manifest-path normalization, host preset-truth enforcement, watcher resilience, and Rust sidecar-contract mirroring.
- 2026-03-13: Re-baselined the reopened Story 3.2 document so the remaining AI review findings are grouped into a dev-ready follow-up checklist with explicit host/contract notes.
- 2026-03-13: Addressed the reopened Story 3.2 code review findings by normalizing Windows manifest-path comparisons, enforcing host-owned preset truth at capture time, keeping capture-confidence watch retries alive after repeated read failures, and promoting helper-facing capture DTOs into the Rust contract layer.
