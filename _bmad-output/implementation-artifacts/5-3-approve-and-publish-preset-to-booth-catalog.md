# Story 5.3: Approve and Publish Preset to Booth Catalog

Status: ready-for-dev

Story Key: `5-3-approve-and-publish-preset-to-booth-catalog`

## Summary

Introduce the missing preset-publication pipeline for internal authoring so an approved preset draft becomes an immutable published bundle with version metadata, is recorded in a dedicated publication audit trail, and becomes selectable in the booth catalog without exposing draft-only or authoring-only state to customer flows. The host must own publication, preserve prior published versions for later rollback, and keep the checked-in approved catalog and Tauri Store out of runtime publication truth.

## Story

As an authorized preset manager,
I want to approve and publish a preset,
so that it appears in the booth's approved catalog.

## Acceptance Criteria

1. Given a preset draft is ready, when I mark it approved and publish, then an immutable preset bundle is created with version metadata, and the approved preset becomes available to the booth catalog.
2. Given a preset is not approved, when the booth catalog is loaded, then that preset is excluded, and only approved published presets are shown.

## Tasks / Subtasks

- [ ] Freeze the preset publication contract before wiring UI or persistence. (AC: 1, 2)
  - [ ] Introduce one shared preset-bundle schema across TypeScript and Rust that captures immutable publication data at minimum: `schemaVersion`, stable `presetId`, `presetVersion`, display metadata, preview references, render parameter snapshot, source draft revision or identifier, `approvedAt`, and `publishedAt`.
  - [ ] Add typed approve/publish request and publish-result DTOs so the frontend never passes ad hoc objects or filesystem paths into Tauri commands.
  - [ ] Preserve the separation between draft state and published state: drafts remain editable authoring inputs, published bundles are immutable outputs that booth sessions can reference safely.

- [ ] Implement a host-owned publication pipeline and dedicated publication audit storage. (AC: 1, 2)
  - [ ] Add a Rust preset-publication domain instead of forcing publication logic into `session_commands.rs` or customer-flow code.
  - [ ] Create a forward-only SQLite migration for publication history such as `preset_publications` and any minimal catalog-pointer metadata needed to know which published version is currently active for a given `presetId`.
  - [ ] Do not overload `session_events` or `operator_interventions` for preset publication history; those tables are currently scoped to booth session and operator activity.
  - [ ] Persist prior published versions so Story 5.4 can roll back without reconstructing history from git, JSON assets, or Store state.

- [ ] Publish approved bundles into the booth catalog through one typed resolution path. (AC: 1, 2)
  - [ ] Replace the assumption that the checked-in approved catalog is the runtime publication source of truth; treat it as a seed or fallback reference only until published bundles exist.
  - [ ] Ensure booth catalog resolution returns only approved published bundles and never includes draft-only or unpublished authoring records.
  - [ ] Preserve deterministic customer catalog ordering and the 1-6 preset bound while introducing publication-backed catalog entries.
  - [ ] Keep session preset selection and active preset validation aligned with the resolved published catalog so unpublished or stale preset IDs remain invalid.

- [ ] Add an authoring-only approve/publish workflow without broadening booth access. (AC: 1)
  - [ ] Add the minimum `preset-authoring` route, state, and screen/service seam needed to trigger explicit approval and publish actions from an internal surface.
  - [ ] Keep booth customer routes free of authoring controls, approval actions, and publication metadata.
  - [ ] Gate publication through runtime profile and/or Tauri capability checks so booth-only installs cannot invoke publish commands successfully.
  - [ ] If Story 5.1 or 5.2 seams are still missing, carry only the smallest prerequisite authoring shell required for publish execution rather than building a second parallel workflow.

- [ ] Keep runtime publication truth out of branch config, session folders, and checked-in assets. (AC: 1, 2)
  - [ ] Do not write publication state into `branch-config.json` beyond a bounded runtime profile or other approved local setting.
  - [ ] Do not mutate `src/shared-contracts/presets/presetCatalog.json` at runtime to simulate publishing.
  - [ ] Do not store publication bundles under active session folders; preset publication is future-session catalog data, not session-scoped customer data.
  - [ ] Keep booth customer copy free of authoring terms such as draft, approval, publish, or rollback.

- [ ] Add publication-aware regression coverage across contracts, host logic, and customer catalog behavior. (AC: 1, 2)
  - [ ] Add TypeScript contract tests for publish payloads, immutable bundle schemas, and published-catalog filtering.
  - [ ] Add Rust tests for bundle creation, publication persistence, catalog resolution, and preservation of older published versions.
  - [ ] Add integration coverage proving an unpublished draft never appears in the booth catalog and a newly published preset does appear through the approved catalog resolution path.
  - [ ] Add authoring-surface tests proving publish actions are unavailable or rejected outside authoring-enabled runtime profile/capability conditions.

## Dev Notes

### Developer Context

- Epic 5 covers the internal preset lifecycle, but the current repo only implements booth-side approved preset consumption. There is no saved Epic 5 implementation artifact yet for Stories 5.1 or 5.2, so Story 5.3 must document the missing publication seam explicitly instead of assuming a finished authoring stack already exists.
- Current repo reality:
  - `src/preset-catalog/services/presetCatalogService.ts` resolves booth presets from the checked-in approved catalog.
  - `src/shared-contracts/presets/presetCatalog.json` and `src/shared-contracts/presets/presetCatalog.ts` define the approved bounded catalog and ordering rules.
  - `src/session-domain/services/presetSelection.ts` and Rust `select_session_preset` only support selecting an already-approved preset into a live session manifest.
  - `src/App.tsx` exposes only the `/customer` surface.
  - `src-tauri/capabilities/default.json` and `src-tauri/tauri.conf.json` currently define one default main window path, not an authoring-specific capability boundary.
  - SQLite currently stores only `session_events` and `operator_interventions`.
- The 2026-03-12 implementation-readiness report explicitly marked the backlog `NOT READY` and called out missing foundational stories for `preset bundle schema`, `runtime profile/capability model`, and other contract surfaces. For Story 5.3, treat those missing foundations as part of the implementation guardrails for this story rather than hidden prerequisites.
- Scope boundaries:
  - In scope: publish contract, immutable bundle creation, publication persistence, authoring-only publish action, booth catalog promotion, and test coverage.
  - Out of scope: detailed preset draft editor UX from Story 5.2, rollback execution UX from Story 5.4, remote preset distribution service, SSO or hardware-backed authoring authentication, and branch rollout dashboards.
- Dependency note:
  - Conceptually this story follows 5.1 and 5.2.
  - Practically, no Epic 5 story files exist yet, so the dev agent may need to add a thin authoring shell and runtime-profile boundary in the same branch to make publish behavior testable.

### Technical Requirements

- A published preset bundle must be immutable after creation. Publishing a newer version for the same `presetId` creates a new versioned bundle and updates the active catalog pointer; it does not rewrite or delete previous published bundles.
- The host owns publication. Frontend code may request approval/publish through typed services, but Rust must perform final validation, bundle materialization, metadata stamping, and audit persistence.
- Preserve a clean separation between publication truth layers:
  - draft authoring state is internal and mutable
  - published bundles are immutable catalog artifacts
  - booth catalog resolution exposes only approved published bundles
  - active booth sessions reference a published preset version, not a mutable draft
- Do not use Tauri Store as the source of truth for published catalog contents or publication history. The official Store plugin is a persistent key-value store, which fits bounded local settings but not authoritative catalog publication state.
- Do not mutate the checked-in approved catalog asset at runtime. `presetCatalog.json` remains a repository asset and should not become a write target for publication.
- Keep one active published version per `presetId` in the resolved booth catalog while preserving older published versions for future rollback.
- Publication persistence should capture enough information for Story 5.4 without guesswork:
  - stable `presetId`
  - monotonic `presetVersion`
  - source draft identifier or revision
  - approval/publish timestamps
  - active/inactive publication status or equivalent catalog pointer
  - immutable bundle location or payload reference
- Authoring access control must remain bounded even before stronger auth exists:
  - booth customer flows cannot surface publish actions
  - booth-only runtime profiles cannot publish
  - publish commands should reject unauthorized runtime contexts even if the frontend route guard regresses
- Customer booth copy and customer-facing schema projections must remain publication-agnostic:
  - no draft status
  - no approval metadata
  - no internal control labels
  - no rollback language

### Architecture Compliance

- Follow the architecture requirement that preset publication belongs to `preset-authoring` on the frontend and `src-tauri/src/preset/` on the host, not inside booth customer screens or generic shared UI modules.
- Respect the typed boundary rule: React components must not call `invoke()` directly for publish actions. Add a typed service/adapter module for publish commands.
- Keep React Router limited to top-level surfaces. If Story 5.3 adds `/authoring`, treat it as a top-level surface entry only; the authoring workflow itself remains state-driven.
- Preserve capability and runtime-profile separation. Publication commands must not simply be added to the default booth window path with no additional guardrails.
- Keep publication audit persistence separate from session lifecycle persistence. Architecture already anticipates `preset_publications` as a dedicated operational table family.
- Maintain the architecture rule that branch-local config stays minimal. Runtime profile gating may live in bounded config, but catalog contents and publication history must not.

### Library / Framework Requirements

- Current workspace baselines from the repo:
  - React `^19.2.0`
  - React DOM `^19.2.0`
  - React Router `7.9.4`
  - `@tauri-apps/api` `^2.10.1`
  - `@tauri-apps/plugin-store` `~2`
  - `@tauri-apps/cli` `2.10.1`
  - Rust `tauri` `2.10.3`
  - `zod` `^4.3.6`
  - `rusqlite` `0.38.0`
- Official-source verification performed on 2026-03-13:
  - React 19.2 docs continue to recommend `startTransition` for non-blocking UI updates. Use that for publish-progress or authoring-surface state changes that should not stall input responsiveness.
  - React 19.2 docs continue to position `useEffectEvent` as the right tool when an effect-driven authoring screen needs the latest values without re-registering timers or listeners.
  - Tauri v2 docs still position commands as the type-safe primitive for frontend-to-Rust communication and explicitly support arguments, return values, errors, and separate command modules. Keep publish behavior on that boundary.
  - Tauri Store docs describe the plugin as a persistent key-value store and one of several state-management options. That supports its current use for bounded local settings, but not as authoritative publication history.
  - Zod 4 remains stable and is explicitly faster and more `tsc`-efficient, so keep it as the TypeScript-side gate for publish DTOs and bundle schemas.

### File Structure Requirements

- Expected frontend files to inspect or update:
  - `src/App.tsx`
  - `src/preset-catalog/services/presetCatalogService.ts`
  - `src/session-domain/services/presetSelection.ts`
  - `src/branch-config/services/branchConfigSchema.ts`
  - `src/branch-config/services/branchConfigStore.ts`
- Expected new frontend domain files:
  - `src/preset-authoring/screens/PublishWorkflowScreen.tsx`
  - `src/preset-authoring/state/*`
  - `src/preset-authoring/services/presetPublishService.ts`
  - `src/preset-authoring/services/publishedPresetCatalogService.ts` or an equivalent domain-first catalog seam
  - `src/shared-contracts/dto/presetPublication.ts`
  - `src/shared-contracts/presets/presetBundle.ts`
  - `src/shared-contracts/schemas/presetPublicationSchemas.ts`
- Expected host files:
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/preset/mod.rs`
  - `src-tauri/src/preset/preset_bundle.rs`
  - `src-tauri/src/preset/preset_catalog.rs`
  - `src-tauri/src/preset/authoring_pipeline.rs`
  - `src-tauri/migrations/0002_preset_publications.sql`
  - `src-tauri/src/db/migrations.rs`
- Expected capability/config files:
  - `src-tauri/capabilities/default.json`
  - new authoring-specific capability file if commands are window-scoped
  - `src-tauri/tauri.conf.json` if an authoring surface or window configuration is introduced
- Keep publication logic out of:
  - `src/customer-flow/*` except reading the resolved approved catalog
  - `src/shared-ui/*` beyond presentation-only authoring components
  - session-manifest persistence files except where booth sessions later reference published preset version metadata

### Testing Requirements

- Add or update TypeScript contract tests proving:
  - publish request/result schemas reject malformed publication payloads
  - immutable preset bundles include required version metadata
  - unpublished drafts are excluded from resolved booth catalog output
  - booth-side preset selection still rejects unpublished IDs
- Add frontend integration coverage proving:
  - a published preset becomes available to the resolved booth catalog
  - an unpublished draft does not appear in the customer preset-selection flow
  - publish actions are not available on the customer route
  - authoring publish UI remains responsive during publish requests
- Add Rust tests proving:
  - publication migration applies cleanly
  - published bundle creation is immutable and versioned
  - publishing a new version for the same `presetId` preserves the prior published version
  - catalog resolution returns only active approved published versions
  - unauthorized runtime contexts or invalid payloads are rejected before publication side effects occur
- Add audit-storage tests proving:
  - `preset_publications` rows persist across reopen
  - publication metadata is queryable without leaking customer session data into authoring records
  - Story 5.4 rollback prerequisites remain intact because older published versions are preserved

### Previous Story Intelligence

- No Epic 5 predecessor story file exists yet in `_bmad-output/implementation-artifacts`.
- The closest reusable precedent is the existing pattern used across the current repo:
  - shared TypeScript schema first
  - mirrored Rust DTO and repository logic
  - Tauri command wrapper
  - contract tests plus Rust persistence tests
- Story 5.3 should extend that pattern rather than inventing an authoring-only shortcut path.

### Git Intelligence Summary

- Recent git history is still dominated by the greenfield reset and host-normalized camera groundwork, not by authoring or publication features.
- Actionable implication for Story 5.3:
  - prefer domain-first additions that mirror the current contract-first style
  - keep publication host-owned
  - avoid introducing one-off frontend-managed preset publication state
- Current repo signals a consistent approach:
  - static approved catalog in shared contracts
  - typed Tauri wrappers in TypeScript
  - Rust commands delegating to domain modules
  - SQLite migration discipline with forward-only versioning
  Story 5.3 should follow that shape closely.

### Latest Tech Information

- Official docs checked on 2026-03-13:
  - React `startTransition`: https://react.dev/reference/react/startTransition
  - React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
  - Tauri calling Rust: https://v2.tauri.app/develop/calling-rust/
  - Tauri Store plugin: https://v2.tauri.app/plugin/store/
  - Zod 4 release notes: https://zod.dev/v4
- Implementation guidance from those sources:
  - keep publish request/response on typed Tauri commands
  - use React 19 non-blocking patterns for internal publish UI state if the publish step does noticeable work
  - keep persistent key-value settings narrow and separate from publication truth
  - continue using Zod 4 as the contract gate for shared publication schemas

### Project Structure Notes

- The current repo does not yet contain `src/preset-authoring/` or `src-tauri/src/preset/`, even though the architecture explicitly expects those domains.
- `src/App.tsx` currently exposes only `/customer`, so Story 5.3 is likely the first place where `/authoring` or equivalent authoring surface composition appears in the live codebase.
- `src-tauri/capabilities/default.json` currently enables one default permission set for the `main` window only. Story 5.3 should not quietly expose publish commands to that path without additional runtime-profile or capability protection.
- `src/shared-contracts/presets/presetCatalog.json` is currently compile-time approved-catalog input, not a mutable runtime catalog store. Preserve that boundary.

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/validation-report-2026-03-12.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/workflow-execution-log.md`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/App.tsx`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/branch-config/services/branchConfigSchema.ts`
- `src/branch-config/services/branchConfigStore.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/diagnostics/operator_log.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/db/migrations.rs`
- `src-tauri/migrations/0001_init.sql`
- `src-tauri/capabilities/default.json`
- `src-tauri/tauri.conf.json`
- `docs/release-baseline.md`
- `tests/contract/presetCatalog.test.ts`
- `tests/integration/presetSelectionFlow.test.tsx`
- `src-tauri/tests/operational_log_foundation.rs`
- React `startTransition`: https://react.dev/reference/react/startTransition
- React `useEffectEvent`: https://react.dev/reference/react/useEffectEvent
- Tauri calling Rust: https://v2.tauri.app/develop/calling-rust/
- Tauri Store plugin: https://v2.tauri.app/plugin/store/
- Zod 4 release notes: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Primary implementation risk: the repo currently has no preset-authoring or publication domain, so this story must establish the contract and persistence seams carefully instead of improvising them inside booth code.
- Primary guardrail: do not treat Tauri Store, branch config, or checked-in JSON assets as runtime publication truth.
- Rollback guardrail: preserve older published versions and dedicated publication metadata so Story 5.4 can be implemented without schema rework.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation target: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`

### Completion Notes List

- Story context was generated from the refreshed Epic 5.3 requirement, the current repo implementation seams, the 2026-03-12 implementation-readiness findings, recent git history, and official React / Tauri / Zod documentation.
- The create-story validator referenced by the workflow engine, `_bmad/core/tasks/validate-workflow.xml`, is not present in this repository, so checklist validation was performed manually.
- The story intentionally treats `preset bundle schema`, publication audit persistence, and runtime-profile/capability gating as first-class implementation work because the latest readiness report identified those as missing foundations.

### File List

- `_bmad-output/implementation-artifacts/5-3-approve-and-publish-preset-to-booth-catalog.md`
