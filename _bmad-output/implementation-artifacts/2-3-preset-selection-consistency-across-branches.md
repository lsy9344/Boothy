# Story 2.3: Preset Selection Consistency Across Branches

Status: review

Story Key: `2-3-preset-selection-consistency-across-branches`

## Summary

Enforce one approved preset-catalog baseline across booth branches so the preset-selection surface, active preset state, and host validation all use the same approved ordering and display names. If branch-local metadata, cached preset state, or future branch-fed catalog inputs deviate from that approved baseline, the booth must fall back safely to the bundled approved catalog and persist an audit event through the existing operational logging pipeline without exposing diagnostics to customers.

## Story

As an operator,
I want the booth to enforce the approved preset catalog for the branch,
so that customers see consistent preset options across locations.

## Acceptance Criteria

1. Given the booth loads the preset catalog, when the catalog is retrieved, then it matches the approved published preset list for the branch.
2. Given any deviation is detected, when the catalog is resolved, then the booth triggers a safe fallback and logs an audit event.

## Tasks / Subtasks

- [x] Centralize approved preset-catalog authority and remove parallel drift paths. (AC: 1, 2)
  - [x] Keep `src/shared-contracts/presets/presetCatalog.json` and its typed wrapper as the single approved catalog baseline for the current MVP until Epic 5 introduces preset publication.
  - [x] Stop letting booth screens depend on raw `mvpPresetCatalog` imports as the effective source of truth; route preset resolution through one typed catalog service or selector.
  - [x] Preserve deterministic approved order, display names, and bounded size rules already enforced by the shared preset schemas.
- [x] Add branch-aware catalog verification with customer-safe fallback behavior. (AC: 1, 2)
  - [x] Verify any branch-local or cached preset-catalog input against the approved baseline before the customer preset screen or in-session preset picker renders it.
  - [x] If the candidate catalog is missing, stale, reordered, renamed, oversized, or contains unsupported preset IDs, fall back to the bundled approved catalog rather than rendering a branch-specific variant.
  - [x] Keep stale `lastUsedPresetId` handling aligned with the verified catalog so invalid IDs still resolve to the first approved preset.
- [x] Record an operational audit event when fallback or mismatch occurs. (AC: 2)
  - [x] Extend the existing operational logging contract with a lifecycle-style audit event for preset-catalog fallback or mismatch detection.
  - [x] Persist the event through the current TypeScript adapter -> Tauri command -> Rust SQLite logging pipeline using branch ID and a bounded reason code, not raw catalog payload dumps.
  - [x] Ensure the event remains customer-invisible while still queryable for operator and later branch-audit workflows.
- [x] Propagate the verified catalog through preset-selection and active-preset change flows. (AC: 1, 2)
  - [x] `PresetScreen` and `PresetCatalogSheet` must render the same verified approved catalog.
  - [x] `resolveDefaultPresetId`, active preset lookup, and `select_session_preset` / active preset change flows must continue rejecting or normalizing IDs outside the approved catalog.
  - [x] Do not introduce branch-specific preset names, preview assets, or ordering through Tauri Store or component-local state.
- [x] Add regression coverage for cross-branch consistency, fallback, and audit logging. (AC: 1, 2)
  - [x] Extend contract tests for approved preset ordering, naming, and bounded-size guarantees.
  - [x] Add integration coverage proving the booth falls back to the approved catalog when branch-local candidate data is invalid and still renders customer-safe preset selection.
  - [x] Add log-schema and Rust logging tests proving the new preset-catalog audit event is accepted and stored correctly.

### Review Follow-ups (AI)

- [x] [AI-Review][High] Restore safe fallback to the bundled approved catalog when candidate input is missing or invalid; the current Story 2.3 service returns `status: 'unavailable'` and blocks preset selection instead of serving the approved catalog baseline. [`src/preset-catalog/services/presetCatalogService.ts:149-197`]
- [x] [AI-Review][High] Wire a real branch-local or cached candidate source into the runtime preset-catalog service; the default exported service is instantiated without `loadCatalogSource`, so branch-aware verification never executes outside test doubles. [`src/preset-catalog/services/presetCatalogService.ts:180-203`, `src/session-domain/state/SessionFlowProvider.tsx:156-163`]
- [x] [AI-Review][Medium] Narrow preset-catalog fallback audit deduplication so it suppresses duplicate logs for a single resolution cycle only; the current `branchId:auditReason` key suppresses all repeated occurrences for the lifetime of the provider. [`src/session-domain/state/SessionFlowProvider.tsx:902-925`]
- [x] [AI-Review][Medium] Update the Story 2.3 tests to assert approved-catalog fallback behavior instead of blocked preset selection when candidate input is missing or invalid; the current specs codify behavior that conflicts with the story tasks. [`src/preset-catalog/services/presetCatalogService.spec.ts:22-54`, `tests/integration/presetSelectionFlow.test.tsx:551-598`]

## Dev Notes

### Developer Context

- Epic 2 already defines the customer-facing preset-selection experience in Stories 2.1 and 2.2, but there are no saved Epic 2 implementation-artifact stories yet. Story 2.3 should therefore extend the existing repo's salvage path rather than assume prior Epic 2 story files exist.
- Current repo state already contains the core consistency building blocks:
  - `src/shared-contracts/presets/presetCatalog.json` is a source-controlled approved catalog.
  - `src/shared-contracts/presets/presetCatalog.ts` and `src/shared-contracts/schemas/presetSchemas.ts` already enforce deterministic order, approved IDs, approved names, and the 1-6 preset bound.
  - Rust `select_session_preset` already resolves preset IDs against the same JSON asset at compile time.
- Current gap: booth UI surfaces still import `mvpPresetCatalog` directly, branch config only models phone/toggle settings, and no explicit audit event exists yet for branch-catalog mismatch or fallback.
- Scope boundary:
  - in scope: catalog verification, safe fallback, audit logging, and consistent approved-catalog rendering
  - out of scope: remote preset publication, branch rollout dashboards, or the full internal authoring/publishing workflow from Epic 5
- Dependency note: Story 2.3 is safest after the 2.1 and 2.2 baseline catalog/selection flow lands. If implemented earlier, carry the missing preset-loading seam as explicit prerequisite work in the same branch instead of inventing a second catalog pipeline.

### Technical Requirements

- Treat the current approved catalog asset as the branch-published baseline for MVP:
  - `src/shared-contracts/presets/presetCatalog.json`
  - `src/shared-contracts/presets/presetCatalog.ts`
- Do not store full branch-specific preset catalogs in `branch-config.json`. Branch config is still limited to approved local settings plus, if absolutely necessary, lightweight catalog metadata such as a version or checksum. It must not become a second source for preset names, descriptions, preview assets, or ordering.
- Safe fallback means the customer still receives a valid approved preset list or a customer-safe bounded state. It must never surface raw mismatch diagnostics, unsupported preset IDs, or internal branch-audit terms on the booth screen.
- Keep preset ID normalization strict:
  - unsupported preset IDs must not become active presets
  - stale cached `lastUsedPresetId` values must resolve to the first approved catalog item
  - active preset display names must still match the approved catalog entry exactly
- Prefer one small typed catalog-resolution layer rather than repeating validation in every screen. A targeted domain-first addition such as `src/preset-catalog/services/approvedPresetCatalogService.ts` or an equivalent existing-domain seam is appropriate if it removes direct screen imports and keeps logic reusable.
- Audit-event payloads must stay bounded and privacy-safe. Log reason codes such as `invalid_id`, `reordered_catalog`, `name_mismatch`, `oversized_catalog`, or `missing_catalog_input`; do not serialize full preview URLs, customer identifiers beyond existing allowed session context, or raw branch configuration blobs into operational logs.
- Reuse the existing operational log path before adding new persistence:
  - frontend contract/schema in `src/shared-contracts/logging/operationalEvents.ts`
  - TS client in `src/diagnostics-log/services/operationalLogClient.ts`
  - Rust validation/storage in `src-tauri/src/diagnostics/lifecycle_log.rs`
  - SQLite tables initialized by `src-tauri/migrations/0001_init.sql`
- Do not invent remote sync or updater behavior in this story. Cross-branch consistency for MVP is enforced by one approved bundled catalog plus branch-safe validation and audit logging.

### Architecture Compliance

- Keep React components away from direct Tauri logging or catalog parsing details. UI should consume a typed verified catalog service, not parse store payloads or call `invoke` directly.
- Preserve the architecture rule that branch variance is tightly controlled. Branch-local settings remain limited to approved contact info and operational toggles; this story must not quietly reintroduce branch-specific customer preset catalogs.
- Continue using the typed adapter/service boundary for operational logging and preset changes. Do not bypass the existing `record_lifecycle_event` / `select_session_preset` command paths.
- Customer-facing surfaces must remain diagnostics-free and within the booth copy budget. A catalog mismatch is an operational concern, not a booth-customer problem statement.
- Keep domain-first placement. If catalog-resolution logic grows, place it in an owning product domain such as `preset-catalog` or `branch-config/services`, not inside `PresetScreen.tsx` or `PresetCatalogSheet.tsx`.
- Maintain one approved preset-definition source shared by TypeScript and Rust. Do not fork a second constant list for UI convenience.

### Library / Framework Requirements

- Workspace baselines from the current repo:
  - React `^19.2.0`
  - React DOM `^19.2.0`
  - React Router `7.9.4`
  - `@tauri-apps/api` `^2.10.1`
  - `@tauri-apps/plugin-store` `~2`
  - `@tauri-apps/cli` `2.10.1`
  - Tauri Rust crate `2.10.3`
  - Zod `^4.3.6`
  - SQLite `3.52.0` via `libsqlite3-hotbundle`
- React 19.2 remains the current React line already used in this repo, so the existing React 19 patterns in `SessionFlowProvider.tsx` are still valid for catalog-verification side effects and fallback handling. [Source: https://react.dev/blog/2025/10/01/react-19-2]
- Tauri v2 official docs still position typed frontend-to-Rust commands as the primary boundary, so audit logging and preset enforcement should stay on the existing command path instead of introducing direct file access from the UI. [Source: https://v2.tauri.app/develop/calling-rust/]
- Tauri Store v2 remains appropriate for lightweight local settings, and the official plugin docs align with this repo's existing `tauri_plugin_store` + `store:default` setup. Keep using it for bounded local config only, not as a source of customer-visible preset variance. [Source: https://v2.tauri.app/plugin/store/]
- Zod 4 remains the project's TypeScript-side contract gate. Keep new catalog verification and audit payload validation in Zod instead of ad hoc object checks. [Source: https://zod.dev/v4]
- SQLite 3.52.0 is already bundled in the repo and includes recent WAL-safety fixes. Keep audit persistence in the existing host SQLite layer rather than introducing a second client-side audit cache. [Source: https://sqlite.org/releaselog/3_52_0.html]

### File Structure Requirements

- Approved catalog and shared schema surface:
  - `src/shared-contracts/presets/presetCatalog.json`
  - `src/shared-contracts/presets/presetCatalog.ts`
  - `src/shared-contracts/schemas/presetSchemas.ts`
  - `src/shared-contracts/dto/presetCatalog.ts`
- Current customer-flow seams that should stop importing raw catalog data directly:
  - `src/customer-flow/data/mvpPresetCatalog.ts`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/PresetScreen.tsx`
  - `src/customer-flow/components/PresetCatalogSheet.tsx`
- Branch/preset resolution seams:
  - `src/branch-config/services/branchConfigSchema.ts`
  - `src/branch-config/services/branchConfigStore.ts`
  - `src/branch-config/services/presetSelectionStore.ts`
  - add a dedicated verified-catalog service in an owning domain if needed
- Session and active preset consumers that must follow the verified catalog:
  - `src/session-domain/services/presetSelection.ts`
  - `src/session-domain/services/activePresetService.ts`
  - `src/capture-adapter/host/presetChangeAdapter.ts`
- Operational audit path:
  - `src/shared-contracts/logging/operationalEvents.ts`
  - `src/diagnostics-log/services/operationalLogClient.ts`
  - `src/diagnostics-log/services/lifecycleLogger.ts` or a new focused audit logger if that keeps responsibilities clearer
  - `src-tauri/src/diagnostics/lifecycle_log.rs`
  - `src-tauri/src/lib.rs` only if command registration changes
- Existing host preset enforcement seam to preserve:
  - `src-tauri/src/commands/session_commands.rs`

### Testing Requirements

- Contract tests:
  - update `tests/contract/presetCatalog.test.ts`
  - update or add tests around `src/shared-contracts/presets/presetCatalog.spec.ts`
  - ensure approved order, approved names, bounded size, and unsupported-ID rejection still hold after introducing branch verification
- Branch-config and catalog-resolution tests:
  - extend `src/branch-config/services/branchConfigStore.spec.ts` if new lightweight catalog metadata is added
  - add unit tests for the verified-catalog service proving invalid candidate catalogs fall back to the approved baseline
  - prove stale `lastUsedPresetId` handling still resolves against the verified catalog
- Integration/UI tests:
  - extend `tests/integration/presetSelectionFlow.test.tsx` to cover branch-catalog mismatch fallback plus audit logging
  - keep assertions customer-safe: the preset UI should still render approved presets and should not show diagnostics copy
  - add or update tests for `PresetCatalogSheet` if in-session preset changes use the same verified catalog seam
- Operational log tests:
  - extend `tests/contract/operationalLogSchemas.test.ts` for the new preset-catalog audit event type
  - add Rust coverage in `src-tauri/tests/operational_log_foundation.rs` or equivalent for the new event persistence path
- Host integration guardrail:
  - keep or extend tests proving `select_session_preset` still rejects unsupported preset IDs even if frontend fallback logic regresses

### Git Intelligence Summary

- Recent git history is dominated by the March 12, 2026 greenfield reset commit, so current repo structure is the right implementation base rather than older salvaged branch-specific artifacts.
- The active repo already enforces important pieces of preset consistency:
  - the approved catalog is source-controlled
  - `presetCatalogSchema` already rejects reordered or renamed catalogs
  - Rust `select_session_preset` already resolves IDs from the same approved asset
- The missing piece is orchestration, not raw validation:
  - `CustomerFlowScreen` and `PresetCatalogSheet` still render `mvpPresetCatalog` directly
  - branch config currently has no explicit approved-catalog verification responsibility
  - operational logging currently has no event that captures preset-catalog mismatch or fallback
- `tests/integration/presetSelectionFlow.test.tsx` already proves two useful baseline behaviors:
  - stale stored preset IDs fall back to the first approved item
  - branch-local variance should stay out of the customer-visible preset list
  Use that test seam as the main regression harness for this story instead of building a new flow from scratch.

### Latest Technical Information

- Verified against official docs on 2026-03-12:
  - React 19.2 remains the current React release line and continues to support the async/event patterns already used in the repo. [Source: https://react.dev/blog/2025/10/01/react-19-2]
  - Tauri v2 official docs still recommend frontend-to-Rust command calls for native boundaries. [Source: https://v2.tauri.app/develop/calling-rust/]
  - Tauri Store plugin docs remain current for bounded local settings and align with the repo's existing plugin/capability setup. [Source: https://v2.tauri.app/plugin/store/]
  - Zod 4 remains the current stable TypeScript schema-validation line. [Source: https://zod.dev/v4]
  - SQLite 3.52.0 release notes document recent reliability fixes already present in this repo's bundled SQLite line. [Source: https://sqlite.org/releaselog/3_52_0.html]

### Project Structure Notes

- The current repo does not yet have a first-class `src/preset-catalog/` domain directory even though the architecture document expects one conceptually. If this story needs a reusable verified-catalog service, adding that domain is acceptable and cleaner than repeating logic in customer screens.
- `branch-config` currently owns minimal local configuration and is already normalized via Zod. Keep that boundary narrow.
- `CustomerFlowScreen` currently decides which customer surface renders and is therefore the right composition seam for injecting a verified catalog into preset-selection screens.
- `PresetCatalogSheet` currently imports `mvpPresetCatalog` directly for in-session preset changes. That direct dependency is a likely drift point and should be replaced with the same verified catalog used on the main preset-selection screen.

### Project Context Reference

- `_bmad-output/project-context.md` remains active guidance for this story.
- The highest-signal rules from that file for Story 2.3 are:
  - keep React components away from raw Tauri commands
  - preserve fully typed cross-boundary DTOs
  - keep branch variance minimal and explicit
  - avoid duplicate contract definitions across frontend and Rust
  - keep customer-visible state translation free from diagnostics and internal terminology

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/shared-contracts/presets/presetCatalog.json`
- `src/shared-contracts/presets/presetCatalog.ts`
- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/customer-flow/data/mvpPresetCatalog.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/customer-flow/components/PresetCatalogSheet.tsx`
- `src/branch-config/services/branchConfigSchema.ts`
- `src/branch-config/services/branchConfigStore.ts`
- `src/branch-config/services/presetSelectionStore.ts`
- `src/session-domain/services/presetSelection.ts`
- `src/capture-adapter/host/presetChangeAdapter.ts`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src/diagnostics-log/services/lifecycleLogger.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/lib.rs`
- `tests/contract/presetCatalog.test.ts`
- `tests/contract/operationalLogSchemas.test.ts`
- `tests/integration/presetSelectionFlow.test.tsx`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- Tauri v2 calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri Store plugin docs: https://v2.tauri.app/plugin/store/
- Zod 4 docs: https://zod.dev/v4
- SQLite 3.52.0 release log: https://sqlite.org/releaselog/3_52_0.html

## Story Readiness

- Status: `review`
- Primary implementation risk: approved catalog rules already exist in multiple places, but booth rendering and branch verification are not yet centralized.
- Primary guardrail: one approved preset-definition source plus one verified-catalog resolution path must drive all customer preset surfaces.
- Dependency note: conceptually follows Stories 2.1 and 2.2; if those stories are not yet implemented, carry their missing catalog-loading seam as prerequisite work rather than forking the preset flow.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- `pnpm lint`
- `pnpm exec vitest run src/preset-catalog/services/presetCatalogService.spec.ts src/preset-catalog/services/presetCatalogService.runtime.spec.ts src/preset-catalog/services/presetCatalogCandidateStore.spec.ts src/session-domain/state/SessionFlowProvider.preset-catalog.spec.tsx tests/integration/presetSelectionFlow.test.tsx`
- `pnpm exec vitest run`
- `cargo test` (run from `src-tauri`)

### Completion Notes List

- Addressed all Story 2.3 review findings by restoring approved-catalog fallback semantics for missing, invalid, and loader-failure candidate input instead of blocking preset selection.
- Added a runtime candidate source seam backed by Tauri Store (`preset-catalog-candidate.json` / `approvedPresetCatalogCandidate`) and wired the default preset-catalog service through it so branch-aware verification now executes outside test doubles.
- Narrowed fallback audit de-duplication to a single unhealthy cycle by clearing the recorded reason after the catalog recovers, allowing repeated mismatch events to be logged again after a healthy resolution.
- Updated Story 2.3 unit and integration tests to assert approved-catalog fallback behavior, runtime candidate-loader wiring, and repeatable fallback audit logging after recovery.
- Fresh verification evidence at close-out: `pnpm lint` passed; the focused Story 2.3 suite passed (`5` files, `22` tests); full `pnpm exec vitest run` passed (`64` files, `216` tests); and `cargo test` passed.

### File List

- `_bmad-output/implementation-artifacts/2-3-preset-selection-consistency-across-branches.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/preset-catalog/services/presetCatalogCandidateStore.ts`
- `src/preset-catalog/services/presetCatalogCandidateStore.spec.ts`
- `src/preset-catalog/services/presetCatalogService.ts`
- `src/preset-catalog/services/presetCatalogService.spec.ts`
- `src/preset-catalog/services/presetCatalogService.runtime.spec.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/SessionFlowProvider.preset-catalog.spec.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`

### Change Log

- 2026-03-13: Re-verified Story 2.3 without additional code changes; `pnpm lint`, the focused Story 2.3 Vitest suite, full `pnpm exec vitest run`, and `cargo test` all passed.
- 2026-03-13: Applied the Story 2.3 review follow-ups by restoring approved-catalog fallback behavior, wiring the runtime candidate catalog loader through Tauri Store, narrowing fallback audit de-duplication to a single unhealthy cycle, updating regression tests, and re-running lint plus focused/full verification.
- 2026-03-13: Corrected Story 2.3 review findings by removing the store-backed customer catalog path, treating missing explicit candidate input as unavailable with audit logging, moving fallback telemetry ahead of session creation, updating blocked preset-selection guidance, aligning the preset-selection integration harness with the capture-loading seam, and regenerating the sidecar protocol schema golden file.
- 2026-03-13: Senior developer review requested changes for approved-catalog fallback behavior, runtime candidate-source wiring, audit dedupe scope, and Story 2.3 regression expectations.

## Senior Developer Review (AI)

**Reviewer:** Noah Lee  
**Date:** 2026-03-13  
**Outcome:** Changes Requested

### Findings

1. **[High] Missing or invalid candidate input blocks the preset UI instead of falling back to the bundled approved catalog.**
   Story 2.3 marks the fallback behavior complete, but `resolveReadyCatalogState()` returns `status: 'unavailable'` for missing input and parse failures, and `createPresetCatalogService()` explicitly opts into that behavior for runtime candidate loads. That contradicts the checked task that says missing, reordered, renamed, oversized, or unsupported candidate catalogs should fall back to the bundled approved catalog rather than render a variant.  
   Evidence: `src/preset-catalog/services/presetCatalogService.ts:149-197`, story task text at `_bmad-output/implementation-artifacts/2-3-preset-selection-consistency-across-branches.md:28-30`.

2. **[High] Branch-aware verification is not wired into the production runtime.**
   The only non-test service instantiation is `export const presetCatalogService = createPresetCatalogService()` with no `loadCatalogSource`, and both the hook and `SessionFlowProvider` default to that no-loader service. In production, this means no branch-local or cached candidate catalog is ever loaded or compared, so Story 2.3 cannot actually detect a cross-branch deviation unless a test injects a fake loader.  
   Evidence: `src/preset-catalog/services/presetCatalogService.ts:180-203`, `src/preset-catalog/hooks/useApprovedPresetCatalog.ts:3-20`, `src/session-domain/state/SessionFlowProvider.tsx:37-41`, `src/session-domain/state/SessionFlowProvider.tsx:156-163`.

3. **[Medium] Audit logging is deduped too broadly and will drop repeated mismatch events.**
   `SessionFlowProvider` keys fallback logging by `${branchId}:${auditReason}` and never clears that set, so once a branch logs `reordered_catalog` once, every later occurrence of the same mismatch is suppressed for the lifetime of the provider. That undercuts the audit trail the story requires when fallback or mismatch occurs.  
   Evidence: `src/session-domain/state/SessionFlowProvider.tsx:902-925`.

4. **[Medium] The focused Story 2.3 tests now lock in behavior that conflicts with the story requirements.**
   The unit and integration tests explicitly expect `status: 'unavailable'` and blocked preset selection for missing candidate input. As long as those tests remain the acceptance target, the intended approved-catalog fallback behavior in Story 2.3 cannot be restored without first correcting the test suite.  
   Evidence: `src/preset-catalog/services/presetCatalogService.spec.ts:22-54`, `tests/integration/presetSelectionFlow.test.tsx:551-598`, story completion notes at `_bmad-output/implementation-artifacts/2-3-preset-selection-consistency-across-branches.md:258-261`.

### Verification

- `pnpm exec vitest run src/preset-catalog/services/presetCatalogService.spec.ts src/session-domain/state/SessionFlowProvider.preset-catalog.spec.tsx src/customer-flow/screens/PresetScreen.spec.tsx src/customer-flow/screens/PresetSelectionSurface.spec.tsx tests/integration/presetSelectionFlow.test.tsx`
  - Result: 5 files, 24 tests passed.
- `cargo test --test operational_log_foundation`
  - Result: 8 tests passed.

### Git / Review Notes

- The repository is currently dominated by untracked application files, so this review relied on the on-disk story artifact, source files, and fresh focused verification rather than a normal commit diff.
