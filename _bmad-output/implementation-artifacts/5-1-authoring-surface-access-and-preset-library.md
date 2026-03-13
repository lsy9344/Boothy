# Story 5.1: Authoring Surface Access and Preset Library

Status: ready-for-dev

Story Key: `5-1-authoring-surface-access-and-preset-library`

## Summary

Add a capability-gated internal authoring entry point and preset library snapshot so authoring-enabled runtimes can open a dedicated preset-management surface, see internal presets with `draft`, `approved`, and `published` status, and keep booth customers limited to the existing approved booth catalog.

## Story

As an authorized preset manager,
I want access to a dedicated authoring surface with a preset library,
so that I can view and start managing presets safely.

## Acceptance Criteria

1. Given the runtime profile is authoring-enabled, when the authoring surface is opened, then the preset library screen is accessible, and booth customers cannot access this surface.
2. Given the preset library is shown, when the list loads, then it displays only existing internal presets with status (`draft`, `approved`, `published`), and no customer-facing surfaces are affected.

## Tasks / Subtasks

- [ ] Establish authoring runtime-profile and access contracts. (AC: 1)
  - [ ] Extend branch/runtime config with a typed runtime profile such as `booth` vs `authoring-enabled`, defaulting to booth-only.
  - [ ] Create shared DTO/schema(s) for an authoring access snapshot and internal preset library items/status enum; keep them separate from the customer approved catalog contract.
  - [ ] Make the authoring surface fail closed when the runtime profile or capability gate does not authorize it.

- [ ] Add a true authoring surface boundary instead of a hidden customer route. (AC: 1)
  - [ ] Add a top-level authoring entry path in the React app and a corresponding Tauri window/capability plan so authoring access is separated from the booth customer surface.
  - [ ] Keep the existing customer flow on its booth route; do not embed authoring screens inside current customer journey components.
  - [ ] Ensure booth customers cannot reach authoring by direct navigation, shared controls, or command calls from the default booth window.

- [ ] Introduce a host-backed internal preset library snapshot. (AC: 2)
  - [ ] Add a typed authoring preset-library command/service that returns internal presets with status values limited to `draft`, `approved`, and `published`.
  - [ ] Seed or load the initial library from a deterministic local source that future Stories 5.2-5.4 can replace with persisted draft/approval data.
  - [ ] Keep booth-approved preset catalog loading unchanged so unpublished internal presets never leak into customer selection.

- [ ] Build the authoring preset library screen. (AC: 1, 2)
  - [ ] Create a `preset-authoring` domain screen/shell that renders preset cards with name, preview, status badge, and updated metadata for internal users.
  - [ ] Reuse existing Brutal Core shared UI primitives/tokens where possible without moving authoring logic into `shared-ui`.
  - [ ] Provide explicit authoring actions such as open, create, or continue as placeholders or wired entry points for Story 5.2, but do not implement detailed editor controls yet.

- [ ] Wire host command registration and capability scoping for authoring-only APIs. (AC: 1, 2)
  - [ ] Register new preset-authoring commands through the existing Tauri command DTO pattern rather than ad hoc JSON responses.
  - [ ] Add an `authoring` capability/window configuration and keep booth-window permissions narrower than authoring permissions.
  - [ ] If custom command scoping is needed, update the Tauri app manifest/build configuration so authoring-only commands are not broadly available to the booth window.

- [ ] Add regression coverage for access gating and library separation. (AC: 1, 2)
  - [ ] Add frontend tests proving booth profile hides or redirects authoring access, while authoring-enabled profile renders the preset library.
  - [ ] Add contract tests for runtime profile parsing and authoring preset-library schemas.
  - [ ] Add host-side tests proving authoring snapshot responses are deterministic and that booth-approved catalog data remains unchanged by authoring library loading.

## Dev Notes

### Developer Context

- Epic 5 implements FR-008 and is the first authoring-focused epic in the refreshed plan. Story 5.1 is foundational: it must establish the access boundary and internal preset library baseline that Stories 5.2-5.4 will build on.
- The current repository is still booth-customer-first:
  - `src/App.tsx` only routes `/customer`.
  - `src/preset-catalog/services/presetCatalogService.ts` and `src/shared-contracts/presets/presetCatalog.ts` only represent the approved booth catalog, not internal presets.
  - `src-tauri/src/commands/session_commands.rs` currently exposes booth session commands only.
  - `src-tauri/capabilities/default.json` grants default permissions to the single `main` window.
- The 2026-03-12 implementation-readiness report explicitly flagged `runtime profile/capability model` as missing foundational work. Story 5.1 should resolve enough of that gap to make authoring access real rather than cosmetic.
- Because this is Epic 5 Story 1, there is no prior implementation artifact in the same epic to inherit from. The dev agent should choose extension points that future authoring stories can reuse:
  - authoring access contract
  - internal preset library schema
  - authoring domain folder
  - host preset command module
- Scope boundaries:
  - In scope: runtime-profile gating, authoring entry surface, internal preset library snapshot, authoring-only visibility boundary, and authoring/booth separation tests.
  - Out of scope: detailed RapidRAW-equivalent control editing (Story 5.2), approval/publish workflow (Story 5.3), rollback UI (Story 5.4), or branch rollout plumbing beyond the access boundary needed here.

### Technical Requirements

- Runtime profile must fail closed:
  - default installations stay booth-only
  - authoring surface requires an explicit `authoring-enabled` (or equivalently named) profile/capability
  - direct navigation or stale cached URLs must not reveal authoring content to booth customers
- Do not reuse the customer approved-catalog schema for the authoring library. The existing catalog contract enforces deterministic approved booth entries and cannot represent `draft` or `approved-but-not-published` states.
- Add a dedicated internal preset library contract with at minimum:
  - stable preset identifier
  - internal display name
  - status enum limited to `draft`, `approved`, `published`
  - preview reference or preview asset path
  - last-updated metadata fit for sorting or display
- Keep authoring library loading read-only in this story. Story 5.1 should let internal users view and enter the authoring workflow, not mutate booth catalog selection or session-state contracts.
- Keep booth isolation explicit:
  - booth customer surfaces continue loading only approved published presets
  - authoring terms/status badges never appear in booth customer copy
  - booth session manifests, current-session review flows, and capture commands remain unaffected
- Preserve typed service boundaries:
  - React components call authoring services/adapters
  - services own `invoke` / store access
  - host commands return typed DTO envelopes or typed snapshots, not unstructured objects
- If an authoring library seed source is introduced for bootstrapping, keep it deterministic and local so later stories can replace persistence without breaking the UI contract.

### Architecture Compliance

- Keep top-level surface separation. The authoring surface must remain independent of the booth customer flow and must not be inserted into `customer-flow/screens/*`.
- Respect the repo rule that React components do not call Tauri directly. New authoring screens should use a dedicated `preset-authoring/services/*` adapter layer.
- Because Tauri permissions are scoped to windows/webviews, not individual React routes, access control must not rely only on hiding a route inside the existing default window. Introduce a labeled authoring window/capability or equivalent host-enforced boundary that keeps authoring-only commands out of the booth window.
- Maintain the architecture’s package/profile direction:
  - one codebase
  - capability/profile differences determine which surfaces are enabled
  - booth shell remains low-choice and customer-safe
  - authoring surface may use internal preset-management language
- Keep shared UI presentation-only. Authoring status mapping, access rules, and preset-library loading belong in the `preset-authoring` domain, not in `shared-ui`.
- Do not let Story 5.1 rewrite the booth route taxonomy or existing session/preset selection flow unless required for safe top-level surface separation.

### Library / Framework Requirements

- Current workspace baselines in the repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/plugin-store`: `~2`
  - `zod`: `^4.3.6`
  - Rust `tauri`: `2.10.3`
  - Rust `rusqlite`: `0.38.0`
- Official-source verification performed on 2026-03-13:
  - Tauri v2 capability docs confirm permissions are granted to windows/webviews via capabilities, and multiple capabilities merge on a window. That means a hidden React route inside the current default window is not a sufficient security boundary for authoring-only commands.
  - Tauri v2 build docs for custom command permissions show authoring-only commands can be scoped with an app manifest / capability setup rather than being exposed to every window by default.
  - Tauri Store plugin docs continue to support a single local store with defaults and autosave. Extend the existing `branch-config.json` store for runtime-profile flags instead of creating a second ad hoc config store.
  - React Router v7 docs remain aligned with top-level surface routing. Keep `/customer` and the new authoring entry as top-level routes only; do not model authoring workflow internals with route-driven state.
  - Zod 4 remains the correct TypeScript boundary validator. Use it for runtime-profile parsing and any new authoring-library DTOs.
- Do not add a new global store or persistence library for this story. The current stack already provides the right pieces: React state/context, Tauri commands, and the Store plugin.

### File Structure Requirements

- Existing frontend files to inspect or update:
  - `src/App.tsx`
  - `src/branch-config/BranchConfigContext.ts`
  - `src/branch-config/BranchConfigProvider.tsx`
  - `src/branch-config/services/branchConfigSchema.ts`
  - `src/branch-config/services/branchConfigStore.ts`
  - `src/preset-catalog/services/presetCatalogService.ts`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/shared-contracts/schemas/presetSchemas.ts`
  - `src/shared-ui/components/HardFramePanel.tsx`
  - `src/shared-ui/components/PrimaryActionButton.tsx`
  - `src/index.css`
- Expected new frontend authoring files:
  - `src/preset-authoring/screens/PresetLibraryScreen.tsx`
  - `src/preset-authoring/screens/AuthoringEntryScreen.tsx` or equivalent shell
  - `src/preset-authoring/services/presetLibraryService.ts`
  - `src/preset-authoring/selectors/presetLibraryView.ts`
  - `src/preset-authoring/copy/presetLibraryCopy.ts`
  - new shared contracts such as `src/shared-contracts/dto/authoringPresetLibrary.ts` and/or `src/shared-contracts/schemas/authoringPresetLibrarySchemas.ts`
- Existing host files to inspect or update:
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/capabilities/default.json`
- Expected new host files:
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/preset/authoring_library.rs` or equivalent preset-authoring host module
  - `src-tauri/capabilities/authoring.json`
  - optional app-manifest/build wiring if custom command permissions are narrowed by capability
- Keep authoring-specific logic out of:
  - `src/customer-flow/*`
  - booth session manifest contracts
  - customer preset selection state
  - any generic shared UI file beyond pure presentation reuse

### Testing Requirements

- Add frontend route/access tests proving:
  - booth/default profile does not render the authoring surface
  - authoring-enabled profile can reach the authoring entry and preset library
  - direct navigation to authoring fails safely when authorization is absent
- Add authoring library UI tests proving:
  - only internal presets are shown
  - each preset renders one of `draft`, `approved`, or `published`
  - status badges and preview metadata render without affecting booth preset-selection UI
- Add contract tests proving:
  - new runtime-profile schema defaults to booth-safe behavior
  - authoring library DTO/schema rejects unknown statuses or customer-catalog-only shapes
  - approved booth catalog contract remains unchanged and continues to expose only approved published presets
- Add host-side tests proving:
  - authoring preset library snapshots deserialize/serialize through the existing DTO pattern
  - unauthorized/default profiles do not expose authoring library success paths
  - authoring library loads deterministically from its seed source without changing session or booth preset state

### Latest Tech Information

- Tauri v2 permissions guide: capabilities grant permissions to windows/webviews, and permissions from multiple capabilities merge at runtime. Treat authoring access as a window/capability concern, not merely a client-side route concern.
- Tauri v2 build customization docs: when custom commands need permission scoping, use the app-manifest command configuration rather than assuming `generate_handler!` alone is sufficient for per-window isolation.
- Tauri Store plugin docs: store defaults and autosave remain supported in v2; extending the existing branch-config store is the lowest-risk place to carry runtime profile information for this story.
- React Router v7 remains suitable for top-level surface separation. Keep authoring entry at the app shell level and let reducer/context state own workflow progression inside the authoring surface.
- Zod 4 remains the current major line and should own runtime-profile enum validation, authoring preset status validation, and any shared DTO parsing before data enters React state.

### Project Structure Notes

- The current repo is already domain-first enough to add a clean `preset-authoring` domain beside existing `customer-flow`, `session-domain`, `preset-catalog`, and `branch-config`.
- Existing reusable UI/styling seams:
  - `src/shared-ui/components/HardFramePanel.tsx`
  - `src/shared-ui/components/PrimaryActionButton.tsx`
  - Brutal Core tokens and utility classes in `src/index.css`
- Existing booth preset-loading code should stay booth-specific:
  - `src/preset-catalog/services/presetCatalogService.ts`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/customer-flow/screens/PresetSelectionSurface.tsx`
- The most important project-context rules for this story are:
  - no direct React `invoke`
  - keep typed DTOs/contracts
  - keep routes limited to top-level surfaces
  - preserve customer/operator/authoring separation
  - do not leak internal preset-authoring terms into customer-visible screens

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/validation-report-2026-03-12.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/project-context.md`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/App.tsx`
- `src/branch-config/BranchConfigContext.ts`
- `src/branch-config/BranchConfigProvider.tsx`
- `src/branch-config/services/branchConfigSchema.ts`
- `src/branch-config/services/branchConfigStore.ts`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/shared-ui/components/HardFramePanel.tsx`
- `src/shared-ui/components/PrimaryActionButton.tsx`
- `src/index.css`
- `src/customer-flow/screens/PresetSelectionSurface.tsx`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- Tauri permissions/capabilities: https://v2.tauri.app/security/permissions/
- Tauri build customization / command permissions: https://v2.tauri.app/develop/plugins/develop-plugin/
- Tauri Store plugin: https://v2.tauri.app/plugin/store/
- React Router docs: https://reactrouter.com/start/declarative/installation
- Zod 4 docs: https://zod.dev/

## Story Readiness

- Status: `ready-for-dev`
- Scope: authoring access boundary, internal preset library snapshot, and authoring/booth separation groundwork
- Reuse strategy: extend branch config, shared UI primitives, and existing DTO/command patterns instead of reworking booth customer flow
- Contract sensitivity: high because runtime profile, Tauri capabilities, and preset-library contracts affect future Epic 5 stories
- Key guardrail: do not treat hidden route logic inside the default window as sufficient authoring security

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Validation target: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`

### Completion Notes List

- Story context was generated from the refreshed Epic 5.1 requirement, the current customer-only app shell, implementation-readiness findings about missing runtime profile/capability groundwork, and official Tauri / Store / React Router / Zod documentation.
- Current repo reality has no `preset-authoring` domain and no authoring-specific Tauri commands, so this story intentionally establishes the access and contract foundation needed for Stories 5.2-5.4.
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so checklist validation must be performed manually.

### File List

- `_bmad-output/implementation-artifacts/5-1-authoring-surface-access-and-preset-library.md`
