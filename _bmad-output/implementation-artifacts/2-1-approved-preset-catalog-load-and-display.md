# Story 2.1: Approved Preset Catalog Load and Display

Status: done

Story Key: `2-1-approved-preset-catalog-load-and-display`

## Summary

Load the bounded approved preset catalog into the preset-selection surface from one authoritative approved source, render each option with a customer-facing name plus preview image or standard preview tile, and handle empty or unavailable catalog cases with customer-safe booth messaging. Reuse the current checked-in catalog and preset-card UI where it helps, but align this story toward a dedicated `preset-catalog` ownership seam instead of leaving catalog loading buried in `customer-flow/data`.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want to view a bounded catalog of approved presets with clear previews,
so that I can choose one look quickly.

## Acceptance Criteria

1. Given an active session exists, when the preset selection surface loads, then the booth displays only 1-6 approved presets, and each preset shows a customer-facing name with a preview image or standard preview tile.
2. Given the catalog is unavailable or empty, when the selection surface loads, then a customer-safe error or wait state is shown, and no internal diagnostic details are displayed.

## Tasks / Subtasks

- [x] Establish one authoritative approved preset-catalog load path for booth display. (AC: 1-2)
  - [x] Source the visible booth catalog from the approved published catalog only; until preset publication infrastructure exists, reuse the checked-in approved catalog asset and typed schemas rather than introducing branch-specific, session-specific, or ad hoc customer-flow data sources.
  - [x] Keep the returned display set bounded to 1-6 approved entries in deterministic approved order and preserve the existing approved preset IDs as the only selectable booth-facing identifiers.
  - [x] Introduce an explicit catalog load result that can represent `loading`, `ready`, `empty`, or `unavailable` so the booth surface can satisfy AC2 without leaking technical errors.

- [x] Render the preset-selection surface from the catalog load result instead of a hardwired screen-local import. (AC: 1)
  - [x] Feed `PresetScreen` and `PresetOptionCard` from the approved catalog load seam so display ownership can move toward the planned `preset-catalog` domain without breaking the current customer flow.
  - [x] Ensure each rendered card shows a customer-facing name and preview image or standard preview tile, keeps touch-friendly booth sizing, and remains free of internal preset-authoring terms or detailed adjustment controls.
  - [x] Continue showing active session context such as the session name from the existing session-domain handoff; do not derive preset display state from route params, branch config, or capture-session assets.

- [x] Add customer-safe empty and unavailable catalog handling. (AC: 2)
  - [x] Show a customer-safe wait/error surface when no approved presets are available or the catalog cannot be loaded, using approved booth copy only.
  - [x] Do not expose raw exception text, filesystem paths, branch IDs, publication status jargon, or internal preset-authoring language in the empty/unavailable state.
  - [x] Block downstream preset-selection progression from a missing catalog state instead of rendering empty grid chrome that implies the booth is ready.

- [x] Keep Story 2.1 scoped to load/display while preserving the salvage path for Story 2.2. (AC: 1-2)
  - [x] Do not introduce new active-preset persistence semantics, future-capture binding rules, or branch-consistency audit logic here; those belong to Story 2.2 and the later operational-governance work.
  - [x] If the current salvage path still couples card click directly to `selectPreset`, isolate catalog loading/display responsibilities cleanly so Story 2.2 can own session-bound activation behavior without redoing the catalog source.
  - [x] Preserve the current booth-first preset boundary: the customer is choosing from approved looks, not entering a live editor or internal preset-authoring workflow.

- [x] Add regression coverage for bounded catalog display and unavailable-state handling. (AC: 1-2)
  - [x] Keep contract tests proving the approved booth catalog stays deterministic, bounded, and tied to the approved preset list.
  - [x] Add or update service/UI tests to prove the preset-selection surface renders the approved cards when the catalog loads successfully and switches to a customer-safe state when the catalog is empty or unavailable.
  - [x] Keep regression checks that preset previews never come from session-scoped capture paths and that branch-local settings do not alter the booth catalog contents.

## Dev Notes

### Developer Context

- Epic 2.1 is the first story in the corrected preset-selection epic. Its job is to make the approved preset catalog available and visible on the booth surface after the active session handoff from Epic 1. Story 2.2 owns the actual active-preset binding semantics; Story 2.1 should not quietly absorb that scope.
- The planning artifacts are clear on the booth-facing boundary:
  - PRD FR-002 requires only 1-6 approved presets, customer-facing names, and preview image or standard preview tile presentation.
  - The PRD also explicitly forbids direct editing workspaces and internal preset-authoring controls on the customer surface.
  - The March 12 readiness report calls out live preview, 0.5-second preset switching, and preview-to-final fidelity as UX guidance rather than hard product requirements. Do not overbuild Story 2.1 into a live filter-rendering or editor pathway just because the UX spec describes that aspiration.
- Current repo reality already contains a strong salvage baseline:
  - `src/shared-contracts/presets/presetCatalog.json` defines a checked-in approved catalog of four presets.
  - `src/customer-flow/data/mvpPresetCatalog.ts` maps that approved catalog to local preview assets and validates the result with Zod.
  - `src/customer-flow/screens/PresetScreen.tsx` plus `src/customer-flow/components/PresetOptionCard.tsx` already render the preset card grid.
  - `src/session-domain/state/SessionFlowProvider.tsx` already moves the customer flow into `preset-selection` after the active session and readiness handoff.
  - `src-tauri/src/commands/session_commands.rs` already validates preset selection against the same checked-in approved catalog asset.
- The main gap is architectural ownership and unavailable-state behavior:
  - There is no dedicated `src/preset-catalog/` domain yet even though the architecture expects one.
  - `PresetScreen` currently assumes the catalog always exists and imports display data through `customer-flow/data`.
  - AC2 therefore still needs explicit empty/unavailable handling instead of relying on a compile-time asset import that can never express a wait/error state.
- Scope boundary:
  - In scope: authoritative approved catalog load path, bounded booth display, customer-safe empty/unavailable state, and alignment toward `preset-catalog` domain ownership.
  - Out of scope: internal preset publication workflow, branch consistency auditing, in-session future-capture preset changes, or exposing RapidRAW-equivalent controls.

### Technical Requirements

- Treat the booth-visible preset catalog as approved published data, not a branch-local or session-local customization point. In the current repo state, that means reusing `src/shared-contracts/presets/presetCatalog.json` plus its Zod-backed mapping layer unless a typed host command is introduced in the same story.
- Preserve the existing bounded catalog guarantees:
  - minimum one approved preset when available
  - maximum six approved presets
  - deterministic approved order
  - customer-facing names must match the approved catalog
- Each booth-visible preset card must provide one clear visual representation:
  - preview image when a safe approved preview asset exists
  - standard preview tile when a concrete image is not available
  - never a session capture thumbnail or a filesystem path under the active session root
- Separate catalog loading state from preset activation state. Story 2.1 needs explicit catalog states such as `loading`, `ready`, `empty`, and `unavailable`; it should not overload `presetSelectionStatus === 'applying'` to represent catalog availability.
- Keep session context and catalog context distinct:
  - session-domain still owns `activeSession` and the current booth journey phase
  - preset-catalog ownership should provide the bounded display data
  - branch config remains limited to approved local settings and last-used preset preference, not catalog composition
- Customer-safe fallback requirements:
  - empty/unavailable handling must use booth-safe copy only
  - no internal authoring, publication, branch rollout, or diagnostics language
  - no raw exception payloads or Tauri/Rust error details on the customer surface
- If a host-backed catalog load command is added in this story, validate the DTO/result on the TypeScript side with Zod before the UI consumes it and keep the frontend-to-host boundary typed and adapter-owned.

### Architecture Compliance

- Preserve the architecture rule that the booth customer surface remains a low-choice booth shell. Story 2.1 must not turn preset selection into an editor workspace, diagnostics surface, or internal publication console.
- Follow the planned ownership split:
  - `session-domain` owns active session truth and journey state
  - `preset-catalog` should own approved catalog consumption for booth surfaces
  - `branch-config` owns minimal approved local settings only
- Keep React components free of direct Tauri calls and direct filesystem access. Any new catalog load behavior should flow through a typed service/adapter layer, not be wired straight into `PresetScreen` or `PresetOptionCard`.
- Preserve the architecture statement that no cache outranks approved preset bundles or session folders. A UI convenience cache is acceptable only if it mirrors the approved catalog source and does not become an alternate source of truth.
- Keep routes limited to top-level surfaces. Do not introduce `/preset-catalog`, `/preset-loading`, or similar route-driven booth flow steps for this story.
- Respect branch-consistency constraints from the PRD and architecture:
  - active branches should consume the same approved preset ordering
  - branch-local config may not mutate the visible booth catalog
  - branch audit/reporting behavior belongs to later operational stories, not Story 2.1
- Customer-facing copy must stay copy-light and diagnostics-free. Empty/unavailable states should preserve the booth-safe language boundary and avoid exposing internal preset-authoring concepts.

### Library / Framework Requirements

- Current workspace baselines from the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/plugin-store`: `~2`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
- Keep using the existing React 19 patterns already present in the repo. If catalog loading introduces effect-driven async logic, follow the same provider/service split used elsewhere instead of introducing a global mutable singleton or screen-local imperative fetch chain.
- If the catalog remains frontend-loaded for now, keep it typed through the existing Zod schemas and plain module imports. If it moves behind a host command in this story, keep that command on the Tauri v2 request/response path and parse its response before UI consumption.
- Tauri Store remains appropriate for small persisted UX settings such as `lastUsedPresetId`. It is not the right authority for the approved booth catalog itself.
- Do not add new data-fetching or global-state libraries for Story 2.1. The existing React context/reducer plus typed service pattern is sufficient.

### File Structure Requirements

- Expected primary implementation surfaces:
  - `src/customer-flow/screens/PresetScreen.tsx`
  - `src/customer-flow/components/PresetOptionCard.tsx`
  - `src/customer-flow/data/mvpPresetCatalog.ts` as a salvage baseline only
  - `src/session-domain/state/SessionFlowProvider.tsx` if preset-selection entry needs catalog availability wiring
  - new `src/preset-catalog/` files if the story introduces the architecture-aligned ownership seam now
- Likely supporting contract files if catalog DTOs are formalized further:
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/shared-contracts/presets/presetCatalog.json`
  - `src/shared-contracts/dto/presetCatalog.ts`
  - `src/shared-contracts/schemas/presetSchemas.ts`
- Host-side files should change only if the story deliberately introduces a typed catalog load command:
  - `src-tauri/src/commands/session_commands.rs` or a more appropriate dedicated preset command module
  - matching Rust DTO/contract files if a new response shape is added
- Test surfaces likely needing updates:
  - `tests/contract/presetCatalog.test.ts`
  - `tests/integration/presetSelectionFlow.test.tsx`
  - `src/customer-flow/components/PresetCatalogSheet.spec.tsx`
  - new tests under `src/preset-catalog/` if a service or selector layer is added
- Structure guardrails:
  - move catalog ownership toward `preset-catalog` instead of keeping long-term source-of-truth behavior in `customer-flow/data`
  - keep booth presentation logic in `customer-flow`
  - keep branch-local preference storage in `branch-config`
  - keep internal preset publication logic out of the customer-flow path

### Testing Requirements

- Keep contract coverage for the approved booth catalog strict:
  - deterministic approved ordering
  - no duplicate preset IDs
  - maximum six visible presets
  - customer-facing names remain aligned with the approved catalog asset
- Add or update service tests for whichever load seam Story 2.1 introduces:
  - successful catalog load returns bounded approved entries
  - empty catalog result maps to a customer-safe empty state
  - unavailable/error result maps to a customer-safe unavailable state without raw diagnostics
- Keep integration coverage on the actual preset-selection user surface:
  - after an active session handoff, the preset-selection screen renders the approved catalog cards
  - each card shows a preview image or standard preview tile plus the customer-facing name
  - the screen does not expose internal preset-authoring or diagnostics terms
  - preview URLs/paths do not point into session-scoped capture storage
- If catalog ownership is moved into a new `preset-catalog` domain, add focused unit tests there instead of relying only on broad screen snapshots.
- Avoid letting Story 2.1 tests become de facto Story 2.2 coverage. This story should verify availability and presentation, not full active-preset binding semantics.

### Latest Tech Information

Verified against official documentation on 2026-03-12:

- React 19.2 remains the current line used by the workspace, and the official React 19.2 release guidance continues to support `useEffectEvent` and modern non-blocking effect patterns. That fits the repo's existing provider-driven async flow and is sufficient for any new catalog-loading side effects.
- Tauri v2 official docs still position Rust commands as the standard frontend-to-host request/response boundary. If Story 2.1 promotes catalog loading into the host, keep it on that typed command path rather than adding direct frontend file access.
- Tauri Store v2 remains a lightweight persisted key-value option for local settings, which reinforces the current split: store `lastUsedPresetId` there if needed, but do not move the authoritative approved preset catalog into Store.
- Zod 4 remains the stable current major line and should continue to gate any TypeScript-side catalog payload or DTO parsing before the customer UI consumes it.

### Project Structure Notes

- The current repo already follows the domain-first structure needed for this story in most places:
  - `customer-flow` owns the booth preset-selection surface and card rendering
  - `session-domain` owns the active session handoff and preset-selection journey phase
  - `branch-config` owns only approved local settings and the saved last-used preset preference
  - `src-tauri/src/commands` already contains host-side preset validation logic for selection
- The main structural variance is that catalog ownership still lives under `src/customer-flow/data/mvpPresetCatalog.ts` instead of the architecture-planned `src/preset-catalog/` domain. Story 2.1 should correct or at least clearly wrap that ownership seam rather than hardening the current shortcut further.
- `tests/integration/presetSelectionFlow.test.tsx` already proves several important invariants:
  - the grid length matches the bounded approved catalog
  - the first approved preset is used as the fallback when the stored preset is stale
  - preview images are not sourced from session paths
  Preserve those protections while widening the story to cover empty/unavailable catalog states.
- Current repo and planning are aligned on one important point: the booth start flow is now session-name-first. Story 2.1 should build on the active session created by Epic 1 and must not reintroduce the older reservation/phone check-in assumptions.

### Project Context Reference

- `_bmad-output/project-context.md` remains active execution context for this story.
- The most relevant rules from that file are:
  - keep React components away from direct Tauri invocation
  - preserve the session folder as durable truth for session-scoped artifacts
  - keep customer-facing copy free of diagnostics and internal preset-authoring language
  - keep branch variance tightly controlled
  - validate cross-boundary data with Zod before it reaches the UI
- Story 2.1 should be implemented as a boundary-preserving catalog-loading story, not as a shortcut around those rules.

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/customer-flow/data/mvpPresetCatalog.ts`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/customer-flow/components/PresetOptionCard.tsx`
- `src/customer-flow/components/PresetCatalogSheet.tsx`
- `src/customer-flow/copy/presetSelectionCopy.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/services/presetSelection.ts`
- `src/branch-config/services/presetSelectionStore.ts`
- `src-tauri/src/commands/session_commands.rs`
- `tests/contract/presetCatalog.test.ts`
- `tests/integration/presetSelectionFlow.test.tsx`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri Store plugin docs: https://v2.tauri.app/plugin/store/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `ready-for-dev`
- Scope: authoritative approved catalog load/display, customer-safe empty or unavailable handling, and alignment toward `preset-catalog` ownership
- Reuse strategy: extend the existing checked-in approved catalog and preset-card UI rather than inventing a second catalog source
- Contract sensitivity: medium; higher only if the story introduces a new host catalog DTO/command
- Repo variance to watch: catalog ownership currently lives in `customer-flow/data`, while the architecture expects `preset-catalog` domain ownership

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation was completed manually
- Verification commands executed:
  - `pnpm lint`
  - `pnpm test:run`
  - `pnpm build`
- Focused regression commands executed:
  - `pnpm vitest run src/preset-catalog/services/presetCatalogService.spec.ts src/customer-flow/screens/PresetSelectionSurface.spec.tsx`
  - `pnpm vitest run tests/contract/presetCatalog.test.ts tests/integration/presetSelectionFlow.test.tsx src/customer-flow/screens/PresetScreen.spec.tsx src/customer-flow/components/PresetCatalogSheet.spec.tsx src/session-domain/services/presetSelection.spec.ts src/preset-catalog/services/presetCatalogService.spec.ts src/customer-flow/screens/PresetSelectionSurface.spec.tsx`
  - `pnpm vitest run src/customer-flow/screens/PresetSelectionSurface.spec.tsx src/customer-flow/components/PresetCatalogSheet.spec.tsx`
  - `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts tests/integration/customerReadinessFlow.test.tsx src/customer-flow/selectors/customerCameraStatusCopy.spec.ts`

### Completion Notes List

- Story context generated from the corrected Epic 2.1 requirement, current repo seams, the March 12, 2026 implementation-readiness assessment, and official React / Tauri / Zod references.
- The story is intentionally written as a salvage-and-align guide: keep the approved checked-in catalog and current preset-card UI where they are useful, but move ownership toward a dedicated `preset-catalog` seam and add the missing empty/unavailable state behavior.
- Story 2.2 boundary is called out explicitly so implementation does not accidentally bury active-preset binding, forward-only change semantics, or branch-governance concerns inside Story 2.1.
- Added a dedicated `preset-catalog` service that maps the approved checked-in catalog into bounded booth display data and returns explicit `loading`, `ready`, `empty`, and `unavailable` states.
- Routed the preset-selection surface through the new catalog seam, preserved the existing confirm-to-continue Story 2.2 behavior, and added customer-safe wait/error messaging that blocks progression when the catalog is missing.
- Updated preset cards and the in-session preset sheet to share the approved booth catalog seam, including a standard preview-tile fallback when a preview image is unavailable.
- Removed the redundant preset-catalog reload from `PresetSelectionSurface` and `PresetCatalogSheet` when `SessionFlowProvider` has already resolved `catalogState`, so the provider seam remains the single active load path during the customer journey.
- Realigned the readiness contracts with the current implementation by restoring `checking-camera` as an allowed readiness state, removing a stray `detail` field from `CustomerPreparationState`, and fixing the remaining hook dependency warning in `SessionFlowProvider`.
- Fresh verification on March 13, 2026: `pnpm lint` passed, `pnpm build` passed, the story-scoped regression suites passed, and a fresh full `pnpm test:run` completed cleanly (`59` files, `170` tests).

### File List

- `_bmad-output/implementation-artifacts/2-1-approved-preset-catalog-load-and-display.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/customer-flow/components/PresetCatalogSheet.tsx`
- `src/customer-flow/components/PresetCatalogSheet.spec.tsx`
- `src/customer-flow/components/PresetOptionCard.tsx`
- `src/customer-flow/components/PresetOptionCard.spec.tsx`
- `src/customer-flow/copy/presetSelectionCopy.ts`
- `src/customer-flow/data/mvpPresetCatalog.ts`
- `src/customer-flow/screens/CustomerEntryScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/customer-flow/screens/PresetSelectionSurface.spec.tsx`
- `src/customer-flow/screens/PresetSelectionSurface.tsx`
- `src/customer-flow/selectors/customerCameraStatusCopy.spec.ts`
- `src/preset-catalog/services/presetCatalogService.spec.ts`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/customerPreparationState.ts`
- `src/shared-contracts/dto/cameraStatus.ts`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `tests/contract/presetCatalog.test.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`

## Change Log

- 2026-03-12: Implemented the approved booth preset-catalog load seam, customer-safe unavailable/empty preset-selection states, preset preview-tile fallback handling, and the supporting regression coverage; updated sprint tracking to `review`.
- 2026-03-13: Removed redundant surface-level catalog reloads, fixed readiness contract/build blockers found during review follow-up, and refreshed the story verification record to match the current branch state.
