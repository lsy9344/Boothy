# Story 5.4: Safe Publication Rollback to Prior Approved Version

Status: ready-for-dev

Story Key: `5-4-safe-publication-rollback-to-prior-approved-version`

## Summary

Introduce a host-owned preset publication history and rollback flow for authorized preset managers so a prior approved published preset version can become the active booth catalog baseline without mutating immutable bundles, interrupting active sessions, or bypassing the approved-catalog guardrails already established in Story 2.3. Replace the current compile-time-only preset catalog path with a version-aware published-bundle resolver that seeds from the bundled approved catalog, pins active sessions to a concrete published revision, and records every rollback in the operational audit store.

## Story

As an authorized preset manager,
I want to roll back to a prior approved preset version,
so that I can recover quickly from a bad publish.

## Acceptance Criteria

1. Given an approved preset has multiple published versions, when I select a prior version to roll back, then the booth catalog reverts to the selected approved version.
2. Given a rollback occurs, when the publication state is updated, then the rollback action is recorded for audit.

## Tasks / Subtasks

- [ ] Establish immutable preset publication history and an active published pointer. (AC: 1, 2)
  - [ ] Add shared DTO/schema support for preset publication records, rollback requests, rollback results, and published catalog snapshots instead of overloading the current booth-only preset catalog contract.
  - [ ] Persist approved published preset bundles and their version metadata in host-managed storage, with one explicit "currently published" pointer per preset or per catalog revision; do not edit historical bundles in place.
  - [ ] Keep `src/shared-contracts/presets/presetCatalog.json` only as the seed/fallback baseline until Epic 5 publication data exists; it must stop being the sole mutable runtime authority once rollback is implemented.

- [ ] Pin active booth sessions to a concrete published preset revision so rollback is safe. (AC: 1)
  - [ ] Extend the session preset contract and manifest persistence so a session stores the published preset version or catalog revision it was bound to, not just `presetId` and display name.
  - [ ] Resolve and persist that published revision at session start and/or preset selection time through the host command path, so active sessions continue using the version they already selected even after a later rollback.
  - [ ] Do not let rollback silently change the preset list or active preset behavior for an already-running customer session; rollback affects future sessions and future non-active catalog loads only.

- [ ] Add an authorized authoring rollback workflow on the typed frontend-to-host boundary. (AC: 1, 2)
  - [ ] Introduce a dedicated preset-authoring service and Tauri command such as `rollback_published_preset_version` instead of wiring rollback through booth customer screens or branch-config helpers.
  - [ ] If the current branch still lacks Epic 5.1-5.3 seams, first add the minimal authoring-enabled route/screen and publication-history picker needed to execute rollback, then use that same seam for this story rather than inventing a temporary second UI.
  - [ ] Only allow rollback targets that are already approved and published, and prevent rollback to drafts, unpublished bundles, or the currently active published version.

- [ ] Route booth catalog resolution through host publication state instead of compile-time JSON alone. (AC: 1)
  - [ ] Replace the current static runtime authority in `presetCatalogService.ts` and `session_commands.rs::approved_preset_catalog()` with a host-resolved published catalog snapshot that can reflect approved rollback state.
  - [ ] Preserve a safe fallback to the bundled approved catalog if publication storage is missing or empty, but do not let Tauri Store, branch config, or React component state become a second catalog source.
  - [ ] Keep all booth catalog consumers aligned on the same published snapshot so preset selection, in-session preset change, and capture-confidence rendering do not drift after a rollback.

- [ ] Record rollback lineage and operator intent in SQLite audit storage. (AC: 2)
  - [ ] Add a migration and repository seam for preset publication / rollback audit rows instead of trying to squeeze rollback history into the existing lifecycle or operator-intervention tables.
  - [ ] Record bounded metadata only: preset identifier, from-version, to-version, actor identity, occurred-at timestamp, and rollout scope if present; do not dump raw preset parameter payloads or session data into audit rows.
  - [ ] Make rollback writes atomic with the publication-pointer swap so audit state cannot claim success if the active published version failed to update.

- [ ] Add regression coverage for rollback safety, booth consistency, and session isolation. (AC: 1, 2)
  - [ ] Add TypeScript contract and service tests for publication history, rollback validation, and host-loaded published catalog fallback behavior.
  - [ ] Add integration coverage proving a rollback changes the published catalog for new sessions or authoring views while an already-active customer session remains pinned to its previously selected published revision.
  - [ ] Add Rust repository / command / migration tests proving rollback rejects invalid targets, preserves immutable publication history, updates the active published pointer, and inserts an audit record.

## Dev Notes

### Developer Context

- Epic 5 is the first point where the preset catalog stops being just a bundled booth asset and becomes a managed internal-authoring output with approval, publication, and rollback discipline.
- Current repo reality is still pre-Epic-5:
  - `src/shared-contracts/presets/presetCatalog.json` is the only approved preset catalog artifact in the workspace.
  - `src/preset-catalog/services/presetCatalogService.ts` maps that static asset directly into booth display data.
  - `src-tauri/src/commands/session_commands.rs` compiles the same JSON with `include_str!(...)` and resolves active presets from it at runtime.
  - `src/App.tsx` exposes only the `/customer` surface.
  - `src/branch-config/services/branchConfigSchema.ts` has no runtime profile or authoring gating field yet.
  - SQLite migrations currently create only `session_events` and `operator_interventions`; there is no publication or rollback audit storage.
- That means Story 5.4 must not be implemented as a small toggle on top of the current preset-selection flow. Safe rollback requires the first real publication-history seam.
- The most important hidden risk in the current codebase is session drift:
  - current session manifests persist only `presetId` and `displayName`
  - they do not persist a published preset version or catalog revision
  - if rollback were added without version pinning, an active booth session could silently change looks mid-session when the global catalog changes
- Story 2.3 is the closest predecessor intelligence even though it is in Epic 2:
  - it explicitly treats `presetCatalog.json` as the approved MVP baseline until Epic 5 introduces real publication
  - it forbids parallel customer-visible catalog sources
  - Story 5.4 should evolve that baseline into host-managed publication state rather than bypass it
- Scope boundary:
  - in scope: rollback of a previously approved published preset version, publication-pointer swap, authoring-side rollback selection, booth-catalog consistency, active-session safety, and rollback audit persistence
  - out of scope: full RapidRAW-style authoring UI implementation, remote preset distribution service, rollout dashboards, installer rollback, customer-facing authoring controls, or forced refresh of active booth sessions
- Dependency note:
  - no refreshed Epic 5.1, 5.2, or 5.3 implementation-artifact stories are present in `_bmad-output/implementation-artifacts`
  - conceptually this story depends on those seams
  - if they are still missing on the implementation branch, carry the smallest possible authoring/publication prerequisites in the same branch instead of building a second rollback-only path

### Technical Requirements

- Treat published preset bundles as immutable versioned records. Rollback changes only which approved published version is currently active; it must never rewrite the contents of older bundles in place.
- Move runtime catalog authority into the host:
  - the bundled JSON asset remains the seed/default baseline
  - published bundle history and the active published pointer live in host-managed storage
  - frontend catalog consumers read a typed published snapshot, not a mutable local constant
- Extend the active session preset contract so it is version-aware. A safe rollback implementation needs at least one durable identifier beyond `presetId` and `displayName`, such as:
  - `publishedVersionId`
  - `catalogRevisionId`
  - or an equivalent immutable bundle reference
- Keep active sessions stable:
  - an already-running booth session must remain bound to the published revision selected earlier
  - rollback must not force a visible preset change into an active session
  - if the customer opens an in-session preset picker after rollback, the session should still resolve against its pinned session revision until the session ends
- Keep booth-customer copy and behavior unchanged except where the catalog content legitimately reflects the new published baseline for future sessions. This is an internal-authoring story, not a customer-surface rewrite.
- Runtime profile / authorization must stay explicit:
  - rollback is allowed only from an authoring-enabled path
  - booth customer routes must not gain rollback capability
  - if strong auth is not implemented yet, pass and validate a bounded local actor identifier for audit completeness rather than leaving the rollback actor unknown
- Do not store full preset publication history or bundle payloads in `branch-config.json` or `preset-selection.json`. Tauri Store remains for lightweight local config only.
- Do not let rollback mutate session folders, captured assets, or existing `session.json` files. Session folders remain customer-session truth, not publication history storage.
- Audit metadata must stay bounded and privacy-safe:
  - preset id
  - from-version
  - to-version
  - actor identity
  - occurred-at timestamp
  - optional branch / rollout scope
  - no raw parameter blobs, preview binaries, filesystem dumps, or unrelated session identifiers
- Preserve the PRD / NFR guardrail that rollback and publication changes must not create a forced update path during an active customer session.

### Architecture Compliance

- Keep React Router limited to top-level surfaces. If Story 5.4 needs a new authoring entry point, it belongs on a route such as `/authoring`; rollback state must not be modeled through nested booth workflow routes.
- Introduce the missing `preset-authoring` and host `preset` domains instead of burying rollback logic inside `customer-flow`, `preset-catalog`, or `branch-config` files alone.
- Preserve the adapter boundary:
  - React components must not write preset bundles directly
  - React components must not read app-local publication files directly
  - rollback requests go through typed service modules and Tauri commands
- Keep the Rust host authoritative for:
  - validating rollback targets
  - resolving published preset history
  - pinning session revisions
  - updating the active published pointer
  - writing publication audit rows
- Preserve the architecture rule that booth customers consume approved published presets only. Customer-facing code must never render draft/approved-but-unpublished authoring state.
- Branch variance remains tightly controlled. Do not use branch config or per-branch local store data to fork customer-visible preset names, ordering, or version history outside the approved publication model.
- Keep customer flow and authoring flow separated:
  - booth customer screens stay diagnostics-free
  - authoring rollback UI may show version history and approval/publication metadata
  - booth customer UI must not display rollback reasons, authoring terminology, or audit identifiers

### Library / Framework Requirements

- Workspace baselines currently present in the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/plugin-store`: `~2`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
  - `rusqlite`: `0.38.0`
  - bundled SQLite line via `libsqlite3-hotbundle`: `3.52.0`
- React 19.2 is still the current React line used in this workspace, so existing provider-level async/event patterns remain valid for authoring-side data loads or rollback-confirmation effects. Keep complex host coordination in services/providers, not ad hoc component callbacks. [Source: https://react.dev/blog/2025/10/01/react-19-2]
- Tauri v2 official docs still position commands as the primary typed request/response boundary between frontend and Rust. New rollback operations should therefore be exposed as dedicated host commands, not direct frontend filesystem writes. [Source: https://v2.tauri.app/develop/calling-rust/]
- Tauri Store plugin docs remain appropriate for bounded local settings only. They do not justify storing mutable published preset history or rollback audit state in the plugin store. Keep publication truth in host storage / SQLite instead. [Source: https://v2.tauri.app/plugin/store/]
- Zod 4 remains the stable TypeScript schema-validation line. Keep rollback request/result contracts, publication DTOs, and published-catalog snapshots validated in Zod before they cross into Tauri. [Source: https://zod.dev/v4]
- SQLite 3.52.0 includes recent reliability fixes already aligned with this repo's WAL-based host persistence model. Keep rollback audit persistence in the existing Rust SQLite layer rather than introducing a second client-side audit cache. [Source: https://sqlite.org/releaselog/3_52_0.html]

### File Structure Requirements

- Existing frontend seams that will likely need modification:
  - `src/App.tsx`
  - `src/branch-config/services/branchConfigSchema.ts`
  - `src/branch-config/services/branchConfigStore.ts`
  - `src/branch-config/BranchConfigProvider.tsx`
  - `src/preset-catalog/services/presetCatalogService.ts`
  - `src/session-domain/services/presetSelection.ts`
  - `src/session-domain/services/activePresetService.ts`
  - `src/capture-adapter/host/presetChangeAdapter.ts`
  - `src/shared-contracts/presets/presetCatalog.json`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/shared-contracts/schemas/presetSchemas.ts`
  - `src/shared-contracts/dto/sessionManifest.ts`
- New frontend/domain files likely required:
  - `src/preset-authoring/screens/PresetLibraryScreen.tsx`
  - `src/preset-authoring/screens/PublishWorkflowScreen.tsx`
  - `src/preset-authoring/services/presetRollbackService.ts`
  - `src/preset-authoring/state/*`
  - `src/shared-contracts/dto/presetPublication.ts`
  - `src/shared-contracts/schemas/presetPublicationSchemas.ts`
- Existing host files that will likely need modification:
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/migrations/0001_init.sql` only as reference for migration style
- New host files likely required:
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/preset/mod.rs`
  - `src-tauri/src/preset/preset_bundle.rs`
  - `src-tauri/src/preset/publication_repository.rs`
  - `src-tauri/src/preset/publication_storage.rs`
  - `src-tauri/src/preset/publication_audit.rs`
  - next available migration such as `src-tauri/migrations/0002_preset_publication_audits.sql`
- Keep publication history out of:
  - `src/branch-config/services/presetSelectionStore.ts`
  - session folders under the booth session root
  - React component-local constants or customer-flow-only data files

### Testing Requirements

- Contract tests:
  - add TypeScript coverage for publication DTOs, rollback request/result envelopes, and any session-manifest version-binding changes
  - keep catalog rules explicit so unsupported rollback targets or missing version identifiers fail schema validation early
- Frontend service / state tests:
  - add tests for the new authoring rollback service proving only approved published versions are selectable
  - add tests for host-loaded published catalog fallback when publication storage is missing and the bundled approved catalog must seed the runtime
  - add tests proving an active session remains pinned to its previously selected published revision after a later rollback
- Integration/UI tests:
  - add authoring-surface tests for version-history display, rollback confirmation, and invalid-target rejection
  - add booth-flow regression coverage proving new sessions see the rolled-back published preset while already-running sessions remain unchanged
  - if `App.tsx` gains authoring route gating, add route-level tests that booth profiles cannot access rollback surfaces
- Rust tests:
  - add repository tests proving rollback rejects drafts, unpublished versions, unknown versions, and the currently active version
  - add tests proving the active published pointer updates atomically with audit persistence
  - add tests proving session manifests keep using their pinned published revision after rollback
- Migration / persistence tests:
  - add host tests for the new publication-audit table schema and indexes
  - keep WAL-backed SQLite behavior and migration ordering aligned with the existing operational-log foundation tests

### Previous Story Intelligence

- No refreshed Epic 5.1, 5.2, or 5.3 implementation-artifact files are present yet in `_bmad-output/implementation-artifacts`.
- The most relevant predecessor story artifact is Epic 2 Story 2.3:
  - it explicitly treats `src/shared-contracts/presets/presetCatalog.json` as the approved baseline until Epic 5 introduces real publication
  - it forbids parallel customer-visible catalog sources
  - it already identifies `presetCatalogService.ts`, `branch-config`, and the operational-log pipeline as the seams to evolve
- Current repo precedent to preserve:
  - session preset changes already use typed service -> Tauri command -> Rust repository flow
  - branch config is intentionally narrow and normalized through Zod
  - booth catalog consumers already expect one approved deterministic catalog
- Treat those seams as established precedent and extend them carefully. Do not create a rollback-only shortcut that bypasses the existing preset-selection boundary.

### Git Intelligence Summary

- Recent git history is dominated by the March 2026 greenfield reset and camera-state stabilization work, not by preset-authoring or publication work.
- That means Story 5.4 should be approached as greenfield work inside an already-structured Tauri/React booth app, not as salvage of a half-built authoring module.
- The closest reusable patterns in the current repo are:
  - typed service -> Tauri command -> Rust repository flows
  - Zod-first contract validation
  - SQLite-backed audit persistence
  - deterministic approved preset catalog enforcement in both TypeScript and Rust
- The main missing pieces are orchestration and data modeling:
  - no authoring route
  - no publication history
  - no preset audit table
  - no session version pinning for preset bundles

### Latest Technical Information

- Verified against official docs on 2026-03-13:
  - React 19.2 remains the active React release line and keeps the current async/provider patterns valid for new authoring data flows. [Source: https://react.dev/blog/2025/10/01/react-19-2]
  - Tauri v2 official docs continue to recommend commands as the standard frontend-to-Rust boundary. New rollback behavior should use dedicated preset commands and be registered through the main invoke handler. [Source: https://v2.tauri.app/develop/calling-rust/]
  - Tauri Store plugin docs still frame the plugin as local key-value storage with capability-controlled access. Keep using it for narrow local configuration rather than mutable publication history. [Source: https://v2.tauri.app/plugin/store/]
  - Zod 4 remains the current stable documentation line for TypeScript validation. Keep new publication and rollback contracts on that line rather than hand-written object guards. [Source: https://zod.dev/v4]
  - SQLite 3.52.0 release notes document the current bundled SQLite line already used by the repo's Rust persistence layer, so rollback audit storage should stay in that same WAL-configured host database path. [Source: https://sqlite.org/releaselog/3_52_0.html]

### Project Structure Notes

- The live repo currently contains booth customer, timing, session, capture, diagnostics, branch-config, and preset-catalog domains, but it does not yet contain the architecture-planned `preset-authoring` frontend domain or `src-tauri/src/preset` host domain.
- `src/App.tsx` currently exposes only `/customer`, so any rollback UI must add a top-level authoring route deliberately rather than smuggling authoring controls into booth customer components.
- `src/preset-catalog/services/presetCatalogService.ts` is currently the frontend composition point that turns the approved static catalog into booth-ready cards. It is the right seam to evolve into a host-backed published-catalog loader with a static fallback.
- `src-tauri/src/commands/session_commands.rs` is currently the host seam that resolves preset identity from the static JSON asset. That logic must be refactored instead of duplicated.
- `src/shared-contracts/dto/sessionManifest.ts` and `src-tauri/src/session/session_manifest.rs` are the critical files for active-session safety because they currently do not preserve a preset publication version or catalog revision.

### Project Context Reference

- `_bmad-output/project-context.md` remains active guidance for this story.
- The highest-signal rules from that file for Story 5.4 are:
  - keep React components away from direct Tauri invocation
  - keep cross-boundary DTOs fully typed and validated with Zod
  - preserve session folders as customer-session truth
  - keep branch variance minimal and explicit
  - avoid duplicate contract definitions across frontend and Rust
  - do not introduce hidden local persistence that outranks host-owned truth

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/implementation-artifacts/2-3-preset-selection-consistency-across-branches.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/App.tsx`
- `src/branch-config/services/branchConfigSchema.ts`
- `src/branch-config/services/branchConfigStore.ts`
- `src/branch-config/BranchConfigProvider.tsx`
- `src/branch-config/services/presetSelectionStore.ts`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/session-domain/services/activePresetService.ts`
- `src/capture-adapter/host/presetChangeAdapter.ts`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/db/sqlite.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/diagnostics/operator_log.rs`
- `src-tauri/migrations/0001_init.sql`
- `tests/integration/presetSelectionFlow.test.tsx`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri Store plugin docs: https://v2.tauri.app/plugin/store/
- Zod 4 docs: https://zod.dev/v4
- SQLite 3.52.0 release log: https://sqlite.org/releaselog/3_52_0.html

## Story Readiness

- Status: `ready-for-dev`
- Primary implementation risk: the current runtime has no preset publication history and no session-level version pinning, so rollback work that skips those foundations will create active-session drift.
- Primary guardrail: move runtime catalog authority into a host-owned published-bundle seam while keeping active sessions pinned to the revision they already selected.
- Dependency note: conceptually follows Stories 5.1-5.3; if those seams are absent on the implementation branch, carry only the minimal authoring/publication prerequisites needed to support rollback through the same architecture-approved path.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation target: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in this repository, so checklist validation must be performed manually

### Completion Notes List

- Story context was generated from the refreshed Epic 5.4 requirement, the current repo's static preset-catalog implementation, Story 2.3's approved-catalog guardrails, recent git history, and official React / Tauri / Tauri Store / Zod / SQLite documentation verified on 2026-03-13.
- The document calls out the current architectural blocker explicitly: session manifests store only preset id and display name today, so safe rollback requires published-revision pinning before any mutable publication path is introduced.
- The story intentionally keeps `presetCatalog.json` as a seed/fallback artifact while moving mutable runtime catalog authority into a host-owned publication-history seam.
- Manual checklist review confirms the story includes summary, requirements, architecture guardrails, technical requirements, testing requirements, prior-story intelligence, git intelligence, latest-tech references, project-context alignment, and a ready-for-dev status update.

### File List

- `_bmad-output/implementation-artifacts/5-4-safe-publication-rollback-to-prior-approved-version.md`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/implementation-artifacts/2-3-preset-selection-consistency-across-branches.md`
- `_bmad-output/project-context.md`
- `src/App.tsx`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/capture-adapter/host/presetChangeAdapter.ts`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/db/sqlite.rs`
- `src-tauri/migrations/0001_init.sql`
