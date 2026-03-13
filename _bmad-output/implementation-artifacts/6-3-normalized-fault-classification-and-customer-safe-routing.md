# Story 6.3: Normalized Fault Classification and Customer-Safe Routing

Status: ready-for-dev

Story Key: `6-3-normalized-fault-classification-and-customer-safe-routing`

## Summary

Establish the reusable fault-classification baseline that Epic 6 is currently missing: one host-owned category model that standardizes severity, retryability, and customer routing before operator summary and recovery workflows build on top of it. The current repo already has a typed normalized error envelope, customer-safe wait/call copy, and operational-log fields such as `recentFaultCategory`, but fault meaning is still implicit and camera-specific; this story should make that fault context explicit, reusable, and safe for both customer routing and later operator-facing work.

## Story

As an operator,
I want faults normalized into clear categories,
so that customer guidance remains safe and consistent.

## Acceptance Criteria

1. Given a fault occurs, when it is classified by the host, then it maps to a standard category with severity and retryability.
2. Given a fault occurs, when it is classified by the host, then the customer-facing state is a safe wait or call guidance.

## Tasks / Subtasks

- [ ] Introduce one canonical fault-classification contract across TypeScript and Rust instead of relying on camera-specific ad hoc meanings. (AC: 1, 2)
  - [ ] Extend the normalized error envelope and related readiness contracts so every blocked-state fault carries an explicit standard category in addition to the existing severity and retryability fields.
  - [ ] Keep the category set bounded and implementation-facing, covering at least the blocked-state classes already visible in the repo: retryable reconnect/helper instability, non-recoverable device unavailability, contract or payload failure, session/manifest integrity failure, and unsupported host-state failure.
  - [ ] Preserve the existing error-envelope shape as the single cross-boundary failure model; do not create a second operator-only fault payload for the same event.
- [ ] Make the Rust host the only place where fault meaning and routing decisions are derived. (AC: 1, 2)
  - [ ] Centralize classification in the native boundary so sidecar outcomes, manifest/load failures, and contract failures are normalized once before the frontend sees them.
  - [ ] Reuse existing `NormalizedErrorEnvelope` constructors or introduce a tightly scoped classifier helper rather than scattering category assignment across multiple commands.
  - [ ] Ensure retryable faults route to safe wait guidance and non-retryable faults route to safe call guidance without exposing raw helper or filesystem details.
- [ ] Project normalized faults through the current customer-flow seam without leaking diagnostics. (AC: 2)
  - [ ] Update the readiness/session-domain mapping so customer state is driven by host-normalized category and route intent, not by UI-side guesses from raw strings.
  - [ ] Keep customer copy within the copy budget and diagnostics-free while preserving operator-visible details in the normalized error envelope.
  - [ ] Maintain compatibility with the current repo seam where customer guidance is still delivered through preparation/readiness surfaces, even if Story 3.1's fuller `waiting` state has not yet landed.
- [ ] Wire normalized fault context into the existing operational-log baseline so later Epic 6 stories can consume stable fault meaning. (AC: 1)
  - [ ] Reuse the existing `recentFaultCategory` fields in TypeScript logging schemas and Rust persistence instead of introducing duplicate log columns or payload keys.
  - [ ] Make lifecycle or escalation logging capable of recording the normalized category whenever customer routing transitions to a blocked safe state.
  - [ ] Keep this story focused on classification and routing; Story 6.4 still owns the broader lifecycle/intervention audit feature surface.
- [ ] Add regression coverage for fault-category normalization, customer-safe routing, and log payload integrity. (AC: 1, 2)
  - [ ] Update contract tests so the normalized error envelope and readiness status require the shared category field and keep severity/retryability aligned.
  - [ ] Add frontend integration tests covering retryable wait guidance, non-retryable call guidance, and continued suppression of raw technical diagnostics in customer copy.
  - [ ] Add Rust tests covering host-side category mapping, readiness/error projection, and persistence of `recent_fault_category` in operational-log writes.

## Dev Notes

### Developer Context

- The March 12 implementation-readiness report called out a sequencing problem inside Epic 6: Story 6.1 ("recent failure context") and Story 6.2 (bounded recovery recording) are not independently solid until normalized fault meaning exists first. Story 6.3 should therefore be treated as an enabling platform story for the rest of Epic 6, not as a thin UI-only enhancement.
- The current repo already contains partial building blocks for this work:
  - TypeScript `normalizedErrorEnvelopeSchema` already carries `severity`, `retryable`, customer-state fields, and operator-action fields.
  - Rust `NormalizedErrorEnvelope` already normalizes retryable reconnect faults versus non-retryable unavailable faults.
  - Frontend customer routing already splits customer-safe copy from operator diagnostics via `mapCameraErrorToViewModel()` and `selectCustomerCameraStatusCopy()`.
  - Operational logging already has `recentFaultCategory` fields in both TypeScript schemas and the SQLite migration.
- The missing baseline is that fault meaning is still implicit:
  - the envelope is currently camera-specific rather than category-driven
  - `recentFaultCategory` exists in log schemas but is not yet the canonical shared classification source
  - customer-safe routing is still driven by a narrow `cameraReconnectNeeded` / `cameraUnavailable` split rather than a reusable normalized category model
- Scope boundary for this story:
  - in scope: canonical fault category contract, host-owned classification, customer-safe wait/call routing, and normalized fault context usable by later Epic 6 work
  - out of scope: building the operator summary UI, implementing the full bounded recovery UI, broad audit-query surfaces, or introducing non-approved operator actions
- Important compatibility constraint:
  - Story 3.1 is already written to evolve readiness toward explicit `waiting` and `phone-required` states, but that story is not implemented yet in the checked-in repo
  - Story 6.3 should not block on that refactor; it should introduce the normalized classification seam in a way that works with the current readiness/preparation flow and remains forward-compatible with the later `waiting` state model

### Technical Requirements

- Extend the shared failure contract so every host-normalized blocked-state fault includes:
  - `faultCategory` (new canonical category)
  - existing `severity`
  - existing `retryable`
  - existing customer-safe route fields and operator detail fields
- Keep the category set bounded, typed, and reusable across the current codebase. The initial set should cover at least:
  - retryable camera/helper reconnect or instability
  - non-recoverable camera/helper unavailability
  - invalid contract or payload conditions
  - session/manifest integrity failures relevant to safe continuation
  - unknown or unsupported host-state failures that must route safely
- Do not infer customer routing from raw message text. The host should emit enough normalized information that React only translates approved route intent into customer copy.
- Preserve the existing guarantee that customer copy never exposes internal diagnostics, SDK details, raw helper errors, paths, or contract failure text. Operator-facing details may remain in `message` / `details`.
- `severity` and `retryable` must remain host-owned. UI code may present the result, but it must not decide severity or retryability on its own.
- Normalize blocked-state routing through one clear rule:
  - retryable or transient faults -> safe wait guidance
  - non-retryable or non-recoverable faults -> safe call guidance
  - no customer-visible troubleshooting instructions beyond approved wait/call behavior
- Reuse `recentFaultCategory` as the persisted/logged name for the normalized category so later operator views can query stable fault meaning instead of reconstructing it from free-form text.
- This story should not widen scope into general-purpose taxonomy design. The category model should be just broad enough to support current blocked-state behavior plus near-term Epic 6/Epic 7 dependencies.

### Architecture Compliance

- Preserve the architecture rule that the Rust host normalizes device/helper/session fault truth once and the frontend consumes that result through typed adapters and services.
- Keep React components out of direct `invoke(...)` usage. Any new routing/classification fields must flow through the existing adapter/service layer and shared DTOs.
- Do not create a parallel "operator fault DTO", "customer route DTO", or UI-only fault enum. There should be one canonical normalized fault contract shared across TypeScript and Rust.
- Keep customer-facing translation centralized in selectors/copy modules, not inline inside `PreparationScreen`, `SessionFlowProvider`, or other React components.
- Do not treat logs as the source of truth for customer routing. Logging should persist the already-normalized category after the host has classified the fault; logs do not decide routing.
- Maintain the current separation of concerns:
  - shared DTOs and schemas in `src/shared-contracts/*`
  - host adapter/view-model mapping in `src/capture-adapter/*`
  - customer-safe state derivation in `src/session-domain/*` and `src/customer-flow/*`
  - native normalization and persistence in `src-tauri/src/contracts/*`, `src-tauri/src/capture/*`, and `src-tauri/src/diagnostics/*`
- Do not expand this story into route-level navigation changes or operator-console surface work. Routing remains workflow-state driven inside the current customer/session seam.

### Library / Framework Requirements

- Current checked-in workspace baselines for this story:
  - React `^19.2.0`
  - React DOM `^19.2.0`
  - React Router `7.9.4`
  - `zod` `^4.3.6`
  - `@tauri-apps/api` `^2.10.1`
  - `@tauri-apps/cli` `2.10.1`
  - Rust `tauri` `2.10.3`
- Official-source verification performed on 2026-03-12:
  - Tauri's official 2.10.x release pages and v2 docs still support the current command/channel split the repo already uses for host request/response plus streaming updates. Keep fault classification on the existing Tauri boundary instead of introducing direct file reads or UI-owned native logic.
  - Zod 4 remains the active official validation line. Continue guarding changed fault DTOs with Zod rather than introducing hand-rolled runtime validation for the same shape.
  - React 19.2 docs still support `useEffectEvent` for effect-driven state reactions that need fresh values without unstable dependencies. That matches the existing `SessionFlowProvider` pattern and should remain the approach if new fault-routing side effects are required.
  - React Router's official changelog is already beyond the locally pinned `7.9.4`, but Story 6.3 does not require routing-package changes. Keep fault handling state-driven inside the provider/screen layer.
- No dependency upgrade is required to complete this story. Implement against the checked-in stack and keep Tauri packages aligned within the current 2.10.x minor line.

### File Structure Requirements

- Primary TypeScript seams likely to change:
  - `src/shared-contracts/dto/cameraErrorContract.ts`
  - `src/shared-contracts/dto/errorEnvelope.ts`
  - `src/shared-contracts/dto/cameraStatus.ts`
  - `src/shared-contracts/dto/cameraContract.ts`
  - `src/shared-contracts/logging/operationalEvents.ts`
  - `src/capture-adapter/host/cameraErrorMapping.ts`
  - `src/capture-adapter/host/cameraAdapter.ts`
  - `src/session-domain/state/customerPreparationState.ts`
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/customer-flow/copy/preparationScreenCopy.ts`
  - `src/customer-flow/selectors/customerCameraStatusCopy.ts`
  - `src/diagnostics-log/services/operationalLogClient.ts`
  - `src/diagnostics-log/services/lifecycleLogger.ts`
- Likely TypeScript additions if the category contract needs its own home:
  - `src/shared-contracts/dto/faultClassification.ts`
  - keep this as a shared contract module, not a UI-local enum file
- Primary Rust seams likely to change:
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/capture/camera_host.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/diagnostics/lifecycle_log.rs`
  - `src-tauri/src/diagnostics/operator_log.rs`
  - `src-tauri/src/diagnostics/mod.rs`
- Likely Rust addition if classification is centralized cleanly:
  - `src-tauri/src/diagnostics/fault_classifier.rs`
  - this is the architecture-aligned place to keep category mapping logic if constructors in `dto.rs` become too overloaded
- Existing repo-structure caveats the implementation must respect:
  - there is no `src/operator-console/*` surface in the current checked-in root app yet, so Story 6.3 must finish without depending on operator UI files that do not exist
  - customer-safe routing currently lands through the preparation/readiness flow, so fault classification changes must wire into that path first
  - `recentFaultCategory` already exists in logging schemas and SQLite; reuse those fields instead of introducing alternate names such as `faultCode` or `faultType` in persistence

### Testing Requirements

- Minimum TypeScript contract coverage:
  - update `tests/contract/errorEnvelope.test.ts` so the normalized envelope asserts category, severity, retryability, customer-safe projection, and operator detail retention together
  - update `tests/contract/cameraReadinessStatus.test.ts` so readiness-status parsing reuses the category-bearing error envelope rather than a second readiness-only fault shape
  - update `tests/contract/cameraContract.test.ts` only if the command/result schema changes because the shared envelope shape changed
- Minimum frontend behavior coverage:
  - extend `tests/integration/customerReadinessFlow.test.tsx` for at least:
    - retryable fault -> safe wait guidance
    - non-retryable fault -> safe call guidance with branch phone
    - raw diagnostic details remain absent from customer copy
  - update `src/customer-flow/selectors/customerCameraStatusCopy.spec.ts` to cover the category-driven wait/call projections
  - add or update state/provider tests if `SessionFlowProvider` starts logging or reacting to normalized category changes directly
- Minimum diagnostics/logging coverage:
  - update `tests/contract/operationalLogSchemas.test.ts` if category presence becomes required in specific lifecycle or intervention payloads
  - update `src/diagnostics-log/services/operationalLogClient.spec.ts` and `src/diagnostics-log/services/lifecycleLogger.spec.ts` if new logging helpers or payload fields are added
- Minimum Rust coverage:
  - update `src-tauri/tests/camera_contract.rs` so sidecar or host faults assert the normalized category in addition to severity/retryability
  - update `src-tauri/tests/operational_log_foundation.rs` so persisted lifecycle/intervention rows retain the normalized `recent_fault_category`
  - add focused tests around any new `fault_classifier.rs` helper if introduced
- Explicit verification targets:
  - one canonical category appears on normalized host faults
  - retryable faults do not route customers to call guidance prematurely
  - non-retryable faults do not continue showing indefinite wait guidance
  - operator-visible detail text remains available while customer-visible text stays clean
  - persisted log payloads use the same category naming as the runtime fault envelope

### Previous Story Intelligence

- There is no earlier Epic 6 implementation-artifact story available in `_bmad-output/implementation-artifacts/`.
- The closest reusable precursor work is the current readiness/error/logging seam already in the repo:
  - `NormalizedErrorEnvelope` exists in both TypeScript and Rust
  - customer-safe readiness copy is already split from operator diagnostics
  - operational logs already reserve `recentFaultCategory`
- Treat those seams as the baseline to tighten, not as reasons to add a second abstraction layer.

### Git Intelligence Summary

- Recent history is relevant here for two reasons:
  - `1fb8bb0` ("camera status flow cleanup and greenfield rebuild documentation") shows that recent work already moved toward normalized camera-state handling rather than raw sidecar leakage.
  - `cb12647` ("Improve camera lamp reliability and sidecar recovery") shows that recent reliability work centered on sidecar recovery behavior and customer-safe status handling, not on ad hoc UI troubleshooting.
- Actionable implication:
  - extend the current normalized host-error/readiness path instead of inventing a new diagnostics pipeline
  - keep sidecar reliability concerns inside host normalization
  - do not resurrect legacy `apps/boothy` or editor-era patterns from the old tree when the active root app already has the correct domain seams
- The most recent repository reset commit (`06ed2b7`) largely reorganized the workspace into the current greenfield package, which means Story 6.3 should target the active root `src/` and `src-tauri/` structure only.

### Latest Tech Information

Verified against official docs on 2026-03-12:

- Tauri's official v2 docs still describe commands as the standard frontend-to-Rust request/response boundary and frontend communication channels/events as the mechanism for streamed updates. That matches the current `cameraAdapter -> get_camera_readiness_snapshot/watch_camera_readiness` seam and should remain the path for fault-classification propagation. [Source: https://v2.tauri.app/develop/calling-rust/] [Source: https://v2.tauri.app/develop/calling-frontend/]
- Official Tauri release pages show the workspace is already on the active 2.10.x line (`@tauri-apps/api` 2.10.1 and `tauri` 2.10.x). Story 6.3 should stay within that aligned minor rather than mixing package lines during contract work. [Source: https://tauri.app/release/%40tauri-apps/api/v2.10.1/] [Source: https://v2.tauri.app/release/tauri/v2.10.2/]
- Zod 4 remains the active official docs line. Continue validating updated fault DTOs and logging payloads with Zod at the TypeScript boundary instead of adding parallel validators. [Source: https://zod.dev/v4]
- React 19.2 docs still support `useEffectEvent` for effect-driven reactions that need fresh state without unstable dependency churn. If fault routing changes require provider-side reactions, keep following the existing `SessionFlowProvider` pattern. [Source: https://react.dev/reference/react/useEffectEvent]
- React Router's official changelog has moved beyond the repo's pinned `7.9.4`, but fault routing in this story should remain workflow-state driven and must not introduce route-level status pages or dependency churn. [Source: https://reactrouter.com/changelog]

### Project Structure Notes

- The current root app already has the correct implementation boundaries for this story:
  - contracts and schemas in `src/shared-contracts`
  - host adapters in `src/capture-adapter`
  - customer-safe translation in `src/session-domain` and `src/customer-flow`
  - native normalization and persistence in `src-tauri/src`
- One important repo caveat:
  - the active codebase does not yet have a root-level `operator-console` feature area even though the architecture anticipates one later
  - Story 6.3 should therefore produce the normalized fault platform capability now and leave later operator-surface rendering to Stories 6.1 and 6.2 or follow-on work
- Another important caveat:
  - the current customer-safe routing path is still preparation/readiness oriented
  - classification changes should integrate there first and remain compatible with future broader blocked-state handling

### Project Context Reference

- `_bmad-output/project-context.md` remains the compressed execution guide for this story.
- Highest-signal project-context rules for Story 6.3:
  - keep React UI away from direct Tauri invocation
  - preserve one typed cross-boundary DTO shape instead of duplicate definitions
  - centralize customer-safe translation logic instead of scattering copy decisions
  - preserve session isolation and avoid diagnostic leakage to customer surfaces
  - keep branch-specific behavior limited to approved config such as phone escalation toggles and branch contact info

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-12.md`
- `_bmad-output/planning-artifacts/sprint-change-proposal-2026-03-12.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/shared-contracts/dto/cameraErrorContract.ts`
- `src/shared-contracts/dto/errorEnvelope.ts`
- `src/shared-contracts/dto/cameraStatus.ts`
- `src/shared-contracts/dto/cameraContract.ts`
- `src/shared-contracts/logging/operationalEvents.ts`
- `src/capture-adapter/host/cameraErrorMapping.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/session-domain/state/customerPreparationState.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/customer-flow/copy/preparationScreenCopy.ts`
- `src/customer-flow/selectors/customerCameraStatusCopy.ts`
- `src/diagnostics-log/services/operationalLogClient.ts`
- `src/diagnostics-log/services/lifecycleLogger.ts`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/diagnostics/lifecycle_log.rs`
- `src-tauri/src/diagnostics/operator_log.rs`
- `src-tauri/tests/camera_contract.rs`
- `src-tauri/tests/operational_log_foundation.rs`
- `tests/contract/errorEnvelope.test.ts`
- `tests/contract/cameraReadinessStatus.test.ts`
- `tests/contract/operationalLogSchemas.test.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- Tauri calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri frontend communication docs: https://v2.tauri.app/develop/calling-frontend/
- Tauri API 2.10.1 release: https://tauri.app/release/%40tauri-apps/api/v2.10.1/
- Tauri core 2.10.2 release: https://v2.tauri.app/release/tauri/v2.10.2/
- Zod 4 docs: https://zod.dev/v4
- React `useEffectEvent` reference: https://react.dev/reference/react/useEffectEvent
- React Router changelog: https://reactrouter.com/changelog

## Story Readiness

- Status: `ready-for-dev`
- Primary implementation goal: establish one reusable normalized fault-category baseline that drives safe customer wait/call routing and gives later Epic 6 stories stable fault meaning
- Primary implementation risk: adding category fields in only one layer and letting TypeScript, Rust, logs, and UI drift from each other
- Primary guardrail: the host owns classification, the shared envelope owns the contract, and customer copy remains a projection layer only
- Sequencing note: this story should reduce the Epic 6 forward-dependency problem identified in the March 12 readiness report by making normalized fault meaning available before summary/recovery stories depend on it

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/create-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual checklist validation performed against `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- The workflow-referenced validator `_bmad/core/tasks/validate-workflow.xml` is not present in the repository, so validation must be completed manually against the generated story content and checklist intent

### Completion Notes List

- Story context was built from the March 12 planning baseline, the current repository implementation seams, recent git history, and official Tauri / React / React Router / Zod documentation.
- The key planning insight for this story is that Epic 6 needs normalized fault meaning before operator summary and bounded recovery stories can stand independently.
- The story intentionally keeps operator UI and broad audit-query scope out of band while still requiring normalized `recentFaultCategory` wiring through the existing log schema.
- The document is written to work against the current checked-in readiness/preparation flow while remaining forward-compatible with the later explicit `waiting` state work already captured in Story 3.1.

### File List

- `_bmad-output/implementation-artifacts/6-3-normalized-fault-classification-and-customer-safe-routing.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
