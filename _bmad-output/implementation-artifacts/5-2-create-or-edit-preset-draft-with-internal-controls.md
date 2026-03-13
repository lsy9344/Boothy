# Story 5.2: Create or Edit Preset Draft with Internal Controls

Status: ready-for-dev

Story Key: `5-2-create-or-edit-preset-draft-with-internal-controls`

## Summary

Introduce a privileged internal preset-authoring workflow that lets authorized users create a new preset draft or reopen an existing draft, adjust detailed internal controls, save those parameters behind a versioned draft identifier, and restore the saved preview later without mutating booth session data or exposing any authoring controls to the booth customer surface.

## Story

As an authorized preset manager,
I want to create or edit a preset draft using detailed internal controls,
so that I can prepare a look for approval.

## Acceptance Criteria

1. Given the preset editor is opened, when I adjust internal controls and save, then a draft preset is stored with a versioned identifier and it is not visible in the booth customer catalog.
2. Given a draft preset exists, when I reopen it, then the saved parameters and preview are restored and no booth session data is modified.

## Tasks / Subtasks

- [ ] Add a privileged internal authoring entry and keep it isolated from the booth customer surface. (AC: 1, 2)
  - [ ] Add a top-level internal authoring surface such as `/authoring` or an equivalent capability-gated shell entry; keep `/customer` unchanged for normal booth builds.
  - [ ] Gate the authoring surface behind a runtime-profile or capability check so booth customers cannot reach the draft editor through route guessing or shared navigation.
  - [ ] Keep authoring composition separate from `customer-flow`; do not expand the booth route tree into an editor workflow.

- [ ] Define typed preset-draft contracts and persistence separate from the approved booth catalog. (AC: 1, 2)
  - [ ] Introduce a shared TypeScript and Rust draft contract that includes a stable draft identifier, version/revision metadata, editor-adjustment payload, preview metadata, and updated timestamps.
  - [ ] Persist draft presets in a host-managed authoring store under app-local data or another host-owned location; do not reuse active session folders, `session.json`, or the approved booth catalog asset.
  - [ ] Ensure saved drafts are explicitly excluded from the approved booth preset catalog until Story 5.3 handles approval/publication.

- [ ] Build the internal preset editor state and save/reopen workflow. (AC: 1, 2)
  - [ ] Add an authoring domain with reducer/context or equivalent state that can create a new draft, load an existing draft, track dirty state, and save a new revision.
  - [ ] Reuse donor concepts from the RapidRAW reference selectively for adjustment structure and control grouping, but do not copy the donor monolith or direct `invoke` usage into the new app surface.
  - [ ] Support both create and edit flows using the same typed save/load seam so reopening a draft restores the exact saved adjustments.

- [ ] Generate and restore draft previews without touching booth session artifacts. (AC: 1, 2)
  - [ ] Add a host-backed preview-generation path for draft presets that writes preview outputs to authoring-owned storage or returns typed preview references for the editor.
  - [ ] Restore the latest saved preview when reopening a draft instead of recomputing booth session state or reading customer captures.
  - [ ] Keep preview generation isolated from the active booth-session lifecycle, current-session gallery, and capture manifest updates.

- [ ] Keep authoring controls internal and prevent customer-catalog leakage. (AC: 1)
  - [ ] Do not append draft presets to `src/shared-contracts/presets/presetCatalog.json` or the current approved booth preset-loading path.
  - [ ] Keep detailed control names, authoring actions, and draft metadata out of customer copy modules and booth screens.
  - [ ] Ensure the draft editor does not alter `activePreset`, `activePresetName`, or any current session state while authoring.

- [ ] Add regression coverage for draft persistence, restoration, and isolation boundaries. (AC: 1, 2)
  - [ ] Add contract tests for draft schema versioning and draft/public separation.
  - [ ] Add frontend integration coverage for entering the internal authoring surface, saving a draft, reopening it, and restoring both adjustments and preview.
  - [ ] Add Rust-side persistence tests proving draft saves do not mutate booth session manifests or approved booth catalog data.

## Dev Notes

### Developer Context

- Epic 5 is the internal preset-authoring capability boundary for FR-008. Story 5.2 is not a customer editor story and must not weaken the booth-first product boundary established in the PRD, UX specification, architecture, and project-context rules.
- The refreshed planning baseline is explicit:
  - booth customers only consume approved published presets
  - detailed RapidRAW-derived controls are internal only
  - draft presets must stay outside the customer-facing catalog until a later approval/publication workflow
- Current repo reality is much narrower than the target Epic 5 scope:
  - `src/App.tsx` exposes only `/customer`
  - `src/customer-flow/*` and `src/session-domain/*` own the current booth journey
  - `src-tauri/capabilities/default.json` is the only capability file today
  - `src-tauri/src/lib.rs` registers booth/session/capture/operator commands only; there is no preset-authoring host module yet
- Story 5.1 is the conceptual predecessor for authoring-surface access and preset library, but there is no saved `5-1` implementation artifact yet in `_bmad-output/implementation-artifacts`. Treat Story 5.2 as implementation-ready while keeping 5.1 surface-access work explicit inside the task plan instead of assuming it already exists in code.
- The current approved booth preset pipeline must remain untouched as the public boundary:
  - `src/shared-contracts/presets/presetCatalog.json`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/preset-catalog/services/presetCatalogService.ts`
  - `src/shared-contracts/schemas/presetSchemas.ts`
- The strongest selective-reuse candidates from the donor reference are structural, not architectural:
  - `reference/uxui_presetfunction/src/utils/adjustments.tsx` for adjustment taxonomy and payload shape ideas
  - `reference/uxui_presetfunction/src/components/panel/right/ControlsPanel.tsx` for grouped control organization
  - `reference/uxui_presetfunction/src/hooks/usePresets.ts` and `src/components/panel/right/PresetsPanel.tsx` for draft CRUD interaction ideas
  - `reference/uxui_presetfunction/src/components/modals/AddPresetModal.tsx` and `RenamePresetModal.tsx` for modal affordances
- The donor reference also shows what not to port directly:
  - `reference/uxui_presetfunction/src/App.tsx` is a monolithic editor shell with direct `invoke`/`listen` usage and broad desktop-editor scope
  - `reference/uxui_presetfunction/src/components/ui/AppProperties.tsx` hardcodes a large invoke surface that does not fit Boothy's adapter-first architecture
  - the donor dependency graph (`framer-motion`, `@dnd-kit/core`, `konva`, etc.) should not be imported wholesale just because it exists in the reference
- Scope boundaries for this story:
  - in scope: draft create/edit, draft persistence, preview restore, privileged authoring entry, and strict isolation from booth data
  - out of scope: approval/publication, rollback to prior approved versions, branch rollout changes, customer catalog mutation, or exposing full RapidRAW editor parity to booth users

### Technical Requirements

- Define a dedicated preset-draft contract instead of stretching booth-session or approved-catalog schemas. At minimum, the draft model should carry:
  - `draftId`
  - `version` or `revision`
  - draft display metadata such as `name` and status
  - saved adjustment payload
  - preview reference or preview artifact metadata
  - `createdAt` / `updatedAt`
- Treat draft identifiers as versioned authoring identifiers, not approved booth preset IDs. Draft IDs must not overlap with the bounded booth catalog IDs currently enforced by `presetIdSchema`.
- Persist draft data in host-owned authoring storage. Do not store authoring drafts in:
  - `session.json`
  - booth session folders under the customer session root
  - `branch-config.json`
  - `src/shared-contracts/presets/presetCatalog.json`
- Reopen behavior must restore the last saved draft state exactly:
  - saved adjustments rehydrate into the authoring editor
  - saved preview or preview reference is restored
  - no booth-session state is reconstructed or modified as a side effect
- Keep authoring preview generation isolated from the customer session pipeline:
  - no writes to the active session manifest
  - no writes to `captures/processed/`
  - no updates to `activePreset`, `activePresetName`, or capture-confidence snapshots
- Reuse donor adjustment semantics selectively. The donor adjustment shape is a useful source for detailed control fields, section groupings, and preset serialization ideas, but Boothy must wrap that shape in its own typed contracts and adapter layer.
- Keep save and load flows behind typed services and host commands. UI code must not read/write draft files directly or issue raw Tauri command strings from React components.
- Favor additive authoring persistence over destructive overwrite. When a user saves, the story should support a stored version/revision model so later approval/publication flows can distinguish draft revisions from approved immutable bundles.
- Customer-surface isolation is a release guardrail for this story:
  - draft presets must not appear in `PresetSelectionSurface`
  - customer copy modules must not gain internal control labels
  - authoring preview assets must not be reused as authoritative booth catalog entries until Story 5.3 publishes them explicitly

### Architecture Compliance

- Keep React Router limited to top-level surface entry. Story 5.2 may add an internal authoring entry such as `/authoring`, but draft workflow truth should live in an authoring domain state model, not in nested route choreography.
- Preserve the architecture's domain-first split by introducing a dedicated `authoring` or `preset-authoring` domain instead of hiding the new workflow inside `customer-flow`, `preset-catalog`, or `shared-ui`.
- Respect the existing adapter rule from `_bmad-output/project-context.md`: React components do not call Tauri directly. Add typed authoring services/adapters for draft load/save/preview operations.
- Keep booth customer, operator, and authoring boundaries explicit:
  - customer code remains in `customer-flow`
  - booth preset consumption remains in `preset-catalog`
  - internal authoring lives in a separate privileged domain
- Add a real capability/profile boundary on the Tauri side. The current `default.json` capability is too broad and too generic to stand in for an internal authoring boundary by itself. Story 5.2 should move toward explicit capability separation for privileged surfaces.
- Do not make `branch-config` or Tauri Store a second source of truth for draft content. Minimal runtime/profile settings may live there, but draft payloads and previews belong to a dedicated authoring repository on the host side.
- Preserve the approved booth preset contract as a separate immutable-consumption boundary. Authoring work should prepare future publishable bundles, not mutate the live approved catalog in place.
- Do not port the donor editor shell architecture. Reuse authoring primitives selectively, but keep Boothy's normalized host boundary, typed DTOs, and capability separation intact.

### Library / Framework Requirements

- Current workspace baselines from the live repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/plugin-store`: `~2`
  - `@tauri-apps/plugin-shell`: `~2.3.5`
  - `zod`: `^4.3.6`
  - Rust `tauri`: `2.10.3`
  - `rusqlite`: `0.38.0`
  - `chrono`: `0.4.42`
- Donor-only libraries in `reference/uxui_presetfunction/package.json` such as `framer-motion`, `@dnd-kit/core`, `konva`, `react-konva`, and `react-window` are not part of the current Boothy workspace baseline. Do not add them by default; only introduce one when a concrete Story 5.2 requirement cannot be met cleanly with the existing stack.
- Keep React 19 patterns aligned with the existing codebase:
  - provider-level effects and `useEffectEvent` remain the preferred pattern for listeners or timers that need fresh state
  - `startTransition` is already used in `CustomerFlowScreen.tsx`; it is acceptable for non-blocking authoring UI state changes where needed
- Keep Tauri communication on typed command boundaries for draft create/load/save/preview flows. Do not bypass the host with direct filesystem access from the frontend.
- Keep Zod 4 as the TypeScript contract gate for new draft DTOs, preview payloads, and authoring command envelopes.
- Keep SQLite and host-side filesystem storage as separate responsibilities:
  - SQLite remains appropriate for operational/audit-style metadata if needed
  - authoring preview files and draft payload artifacts may live in host-managed storage
  - neither should mutate booth session truth while authoring

### File Structure Requirements

- Current frontend composition seams that will likely change:
  - `src/App.tsx`
  - `src/branch-config/services/branchConfigSchema.ts` if a runtime profile or authoring-enabled flag is introduced through approved local config
  - `src/shared-ui/*` for presentation-only primitives reused by authoring screens
- New frontend domain files should live in a dedicated authoring domain, for example:
  - `src/authoring/screens/PresetLibraryScreen.tsx`
  - `src/authoring/screens/PresetEditorScreen.tsx`
  - `src/authoring/components/*`
  - `src/authoring/state/AuthoringProvider.tsx`
  - `src/authoring/state/authoringReducer.ts`
  - `src/authoring/services/presetDraftService.ts`
  - `src/authoring/services/presetPreviewService.ts`
- Shared contract files likely needed:
  - `src/shared-contracts/dto/presetDraft.ts`
  - `src/shared-contracts/schemas/presetDraftSchemas.ts`
  - `src/shared-contracts/index.ts`
  - update `src/shared-contracts/dto/presetCatalog.ts` only if published-vs-draft boundaries need stronger typing, not to merge the data sets
- Existing booth-catalog files that must remain separate from draft persistence:
  - `src/shared-contracts/presets/presetCatalog.json`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/preset-catalog/services/presetCatalogService.ts`
  - `src/customer-flow/screens/PresetSelectionSurface.tsx`
- Tauri host files likely needed for Story 5.2:
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/mod.rs`
  - new `src-tauri/src/commands/preset_authoring_commands.rs`
  - new `src-tauri/src/preset/mod.rs`
  - new `src-tauri/src/preset/draft_repository.rs`
  - new `src-tauri/src/preset/preview_service.rs`
  - optional audit support in `src-tauri/src/db/*` only if draft-revision metadata must be queryable there
- Capability/profile files likely needed:
  - `src-tauri/capabilities/default.json`
  - new `src-tauri/capabilities/authoring.json` or equivalent privileged capability file
- Donor reference files to inspect but not copy wholesale:
  - `reference/uxui_presetfunction/src/utils/adjustments.tsx`
  - `reference/uxui_presetfunction/src/hooks/usePresets.ts`
  - `reference/uxui_presetfunction/src/components/panel/right/ControlsPanel.tsx`
  - `reference/uxui_presetfunction/src/components/panel/right/PresetsPanel.tsx`
  - `reference/uxui_presetfunction/src/components/panel/Editor.tsx`
  - `reference/uxui_presetfunction/src/components/modals/AddPresetModal.tsx`
  - `reference/uxui_presetfunction/src/components/modals/RenamePresetModal.tsx`

### Testing Requirements

- Add shared contract tests for:
  - preset-draft schema parsing and validation
  - versioned draft identifier / revision metadata behavior
  - explicit separation between draft presets and approved booth catalog payloads
- Add frontend state/integration coverage for:
  - privileged authoring entry is unavailable in booth-only runtime conditions
  - creating a new draft saves a versioned draft record
  - reopening an existing draft restores saved adjustments and saved preview
  - saving a draft does not change booth customer preset-selection state
  - authoring controls are never rendered in customer-flow screens
- Add or extend host-side Rust tests for:
  - draft persistence round-trips
  - preview reference persistence and reload
  - no mutation to session manifests or approved preset catalog artifacts during authoring saves
  - capability/profile checks or command rejection when authoring is not enabled
- Keep mocks at the typed service layer in frontend tests. Do not spread ad hoc `invoke` mocks through authoring UI components.
- Reuse the project's current testing split:
  - Vitest contract/unit/integration tests in `src` and `tests/contract`
  - Rust tests close to the authoring repository/command modules in `src-tauri/tests/`

### Previous Story Intelligence

- There is no saved Epic 5.1 implementation artifact yet, so Story 5.2 cannot inherit ready-made authoring-surface learnings from a predecessor story file.
- The practical predecessor intelligence comes from the current architecture baseline instead:
  - authoring is a separate privileged surface, not a booth extension
  - the booth catalog is still the approved bounded public set
  - current repo seams already enforce strong preset-catalog boundaries that drafts must not bypass
- Treat Story 5.2 as the first concrete implementation baseline for the internal authoring domain, but keep Story 5.1 access/library concerns explicit in the task plan instead of silently assuming they are already done.

### Git Intelligence Summary

- Recent git history remains dominated by the greenfield reset and camera/readiness normalization work rather than internal authoring implementation:
  - `06ed2b7` restructured the repository around the refreshed planning baseline
  - earlier commits focus on camera reliability and runtime stabilization, not preset-authoring
- Actionable implication for Story 5.2:
  - there is no existing authoring seam to extend directly
  - the current repo should be treated as a booth-first baseline that now needs a new privileged authoring domain added cleanly
- Current repo structure reinforces the gap clearly:
  - `src/App.tsx` routes only to `/customer`
  - `src-tauri/src/lib.rs` registers no preset-authoring commands
  - `src-tauri/capabilities/default.json` is the only capability file
- The booth preset contract is already disciplined and should be preserved:
  - current shared preset IDs are bounded and approved
  - customer preset services already validate deterministic order and approved names
  - Story 5.2 should build parallel draft persistence, not weaken those guardrails

### Latest Tech Information

- Official-source verification refreshed on 2026-03-13:
  - React 19.2 official docs continue to position `useEffectEvent` as the right pattern for effects that need the latest state without re-subscribing listeners unnecessarily. That aligns with the current provider patterns already used in `SessionFlowProvider.tsx`.
  - Tauri v2 official docs continue to treat Rust commands as the standard typed request/response boundary between the frontend and the host. Story 5.2 should keep draft save/load/preview flows on that boundary instead of direct filesystem access from React.
  - Tauri v2 capability docs describe capabilities as the permission model attached to windows/webviews. That reinforces the need to add an explicit privileged authoring capability or equivalent runtime boundary instead of assuming the default capability is sufficient for internal-only tools.
  - Tauri Store plugin docs still position the plugin as file-backed async key-value storage. It remains acceptable for small runtime/profile settings, but Story 5.2 should not use it as the primary repository for full draft payloads and preview artifacts.
  - Zod 4 remains the stable official contract-validation surface for TypeScript-side schema enforcement and should continue to guard new preset-draft DTOs.

### Project Structure Notes

- The current live repo already has strong domain splits for booth concerns:
  - `customer-flow`
  - `session-domain`
  - `preset-catalog`
  - `capture-adapter`
  - `timing-policy`
  - `diagnostics-log`
- What is missing is the architecture-aligned internal authoring domain. Story 5.2 should add it intentionally instead of borrowing booth domain folders as temporary storage.
- The current Tauri host also lacks a `preset` domain directory. Introducing one now aligns the live repo more closely with the architecture document's intended shape.
- Keep shared UI presentation-only. If authoring needs new controls, place business logic in the authoring domain and keep reusable visual primitives in `shared-ui`.

### Project Context Reference

- `_bmad-output/project-context.md` remains active implementation guidance for this story.
- The highest-signal rules from that file for Story 5.2 are:
  - keep React components away from direct Tauri invocation
  - preserve fully typed cross-boundary DTOs
  - keep customer-facing and privileged surfaces clearly separated
  - do not make routes, caches, or UI memory the durable source of truth
  - avoid duplicate contract definitions across frontend and Rust

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `package.json`
- `src/App.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/shared-contracts/dto/presetCatalog.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src/branch-config/services/branchConfigSchema.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src-tauri/Cargo.toml`
- `src-tauri/capabilities/default.json`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/session_commands.rs`
- `reference/uxui_presetfunction/package.json`
- `reference/uxui_presetfunction/src/App.tsx`
- `reference/uxui_presetfunction/src/hooks/usePresets.ts`
- `reference/uxui_presetfunction/src/utils/adjustments.tsx`
- `reference/uxui_presetfunction/src/components/panel/right/ControlsPanel.tsx`
- `reference/uxui_presetfunction/src/components/panel/right/PresetsPanel.tsx`
- `reference/uxui_presetfunction/src/components/panel/Editor.tsx`
- `reference/uxui_presetfunction/src/components/modals/AddPresetModal.tsx`
- `reference/uxui_presetfunction/src/components/modals/RenamePresetModal.tsx`
- `tests/contract/presetCatalog.test.ts`
- `src/preset-catalog/services/presetCatalogService.spec.ts`
- `src-tauri/tests/session_repository.rs`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri v2 capabilities docs: https://v2.tauri.app/learn/security/capabilities/
- Tauri Store plugin docs: https://v2.tauri.app/plugin/store/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Scope: privileged preset-draft authoring, draft save/reopen, preview restore, and strict isolation from booth session and approved catalog boundaries
- Primary implementation risk: accidentally collapsing the new authoring workflow into the booth route tree or reusing booth preset/session contracts as draft persistence
- Primary guardrail: build a separate typed authoring domain with explicit capability/profile gating and host-owned draft storage

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation target: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`

### Completion Notes List

- Story context was generated from the refreshed Epic 5.2 requirement, the live repository state, the donor RapidRAW reference, recent git history, and official React / Tauri / Zod documentation refreshed on 2026-03-13.
- The repository does not contain `_bmad/core/tasks/validate-workflow.xml`, so the checklist step for this story must be validated manually rather than through the missing workflow runner.
- The story intentionally treats draft presets as a separate internal persistence model and preserves the existing approved booth preset catalog as an external consumption boundary.
- Story 5.1 has no saved implementation artifact yet, so this document carries the missing access/library implications explicitly instead of assuming predecessor code exists.

### File List

- `_bmad-output/implementation-artifacts/5-2-create-or-edit-preset-draft-with-internal-controls.md`
