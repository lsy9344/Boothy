# Story 1.2: Booth App Shell and Session Start Screen

Status: done

Story Key: `1-2-booth-app-shell-and-session-start-screen`

## Summary

Replace the current two-step customer entry and legacy reservation/phone check-in with the corrected booth-first session-start surface: one customer-visible session name input, one start action, and inline empty-state validation. Reuse the Story 1.1 shell foundation and preserve later Story 1.3/1.4 scope boundaries by avoiding premature host-side session provisioning or session-root creation in this story.

## Story

As a booth customer,
I want a simple session-start screen with a single session name input,
so that I can begin a booth session quickly without extra friction.

## Acceptance Criteria

1. When the app opens the customer start flow, the first actionable screen shows exactly one session name input and one start action, and it does not expose reservation name, phone suffix, or any other booth-start inputs.
2. When the session name field is empty or whitespace and the customer attempts to continue, the UI shows an inline validation message and blocks progress until a non-empty value is entered.
3. The customer-facing start screen remains within the approved booth copy boundary: one primary instruction sentence, one supporting sentence, and one primary action label, with no technical or internal-authoring language.
4. The start interaction reuses the existing Story 1.1 booth shell foundation and does not create the real session root, write `session.json`, or call host-side session provisioning yet; Story 1.3 remains responsible for durable session creation.
5. The new session-name-only flow preserves booth-first accessibility and touch requirements, including large touch targets, clear focus behavior, and customer-safe validation feedback on approved Windows booth layouts.

## Tasks / Subtasks

- [x] Replace the legacy customer entry form model with a session-name-only start screen. (AC: 1, 2, 3)
  - [x] Remove customer-facing `reservationName` and `phoneSuffix` inputs from the Story 1.2 entry path.
  - [x] Introduce one `sessionName` input with trimmed non-empty validation and customer-safe inline error copy.
  - [x] Keep the screen to one primary instruction sentence, one supporting sentence, and one primary action label.
- [x] Rewire the customer entry flow to stop at UI-only validation in this story. (AC: 2, 4)
  - [x] Ensure the start action does not invoke host-side session provisioning yet.
  - [x] Keep Story 1.3 responsible for actual session root creation and `session.json` persistence.
  - [x] Preserve a clean handoff seam so Story 1.3 can attach real provisioning without reworking the screen.
- [x] Align the current implementation with the corrected PRD and remove legacy check-in assumptions. (AC: 1, 4)
  - [x] Replace current reservation/phone-specific copy, DTO assumptions, and UI wiring where they directly drive the Story 1.2 entry experience.
  - [x] Avoid broad unrelated rewrites in later capture, preset, or operator flows unless they are required to remove direct Story 1.2 contradictions.
- [x] Add regression coverage for the corrected start flow. (AC: 1, 2, 5)
  - [x] Add or update component tests for the new session-name-only entry screen.
  - [x] Add or update flow/integration tests proving empty input is blocked and valid input advances only within the UI seam expected for Story 1.2.
  - [x] Verify no reservation-name or phone-suffix fields remain visible in the customer start path.

## Dev Notes

### Developer Context

- The corrected PRD is explicit: booth customers start with a single session name input only. The current implementation still uses a legacy reservation-name plus phone-suffix check-in model in the customer entry path, so Story 1.2 is a direct correction of an existing product-definition violation.
- Keep this story UI-bounded. Story 1.3 owns real session identity creation, session-root creation, and `session.json` persistence. Story 1.2 should prepare the correct entry screen and a clean handoff seam only.
- Preserve the existing Story 1.1 shell foundation where possible instead of rebuilding the customer shell from scratch.
- Do not reintroduce customer editing, reservation lookup, phone-number capture, or any internal RapidRAW/preset-authoring controls into the booth entry flow.

### Technical Requirements

- The customer entry path must expose exactly one editable start field: `sessionName`.
- Validation for this story is minimal and customer-safe: trim whitespace, reject empty input, and show inline guidance without technical wording.
- Do not call host-side session provisioning from the Story 1.2 start action. If a temporary local transition is needed, keep it clearly bounded so Story 1.3 can replace it with real provisioning.
- Remove direct dependencies on `reservationName` and `phoneSuffix` from the Story 1.2 UI path. If those types or reducers still exist for later salvage work, isolate them away from the customer start screen instead of leaving them active in the entry experience.
- Keep customer-visible copy aligned with `NFR-001`: one primary instruction sentence, one supporting sentence, one primary action label.
- Maintain touch-first behavior, semantic HTML, and visible focus handling.

### Architecture Compliance

- React UI must continue to avoid direct raw Tauri invocation. Keep boundary logic in typed services/adapters only.
- Maintain domain-first structure. The relevant surface remains in `src/customer-flow/`, with state support in `src/session-domain/` and presentation primitives in `src/shared-ui/`.
- Do not use routes as the source of truth for session progression. The current `CustomerEntryScreen` local entry transition is acceptable as a seam; expanding workflow truth into route transitions is not.
- Do not introduce durable session truth into React local state. Real session lifecycle remains host-owned in later stories.
- Keep customer-facing state translation plain-language and free of diagnostics or authoring language.

### Library / Framework Requirements

- Use the current workspace baselines already installed in `package.json`: React `19.2.x`, TypeScript `5.9.x`, Vite `7.3.x`, React Router `7.9.4`, Tauri `2.10.1`, and Zod `4.3.x`.
- Preserve the architecture-pinned `react-router` line (`7.9.4`). Do not silently upgrade routing while implementing this story.
- The project is already on the approved Node baseline `>=20.19.0 <21 || >=22.12.0`; keep any new dev/test commands compatible with that floor.
- Keep validation logic compatible with Zod 4 schemas if shared schema updates become necessary, but do not over-expand schema churn beyond Story 1.2 needs.

### File Structure Requirements

- Reuse and update the existing customer entry files first:
  - `src/customer-flow/screens/CustomerEntryScreen.tsx`
  - `src/customer-flow/screens/CustomerStartScreen.tsx`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/copy/customerStartCopy.ts`
- Expect follow-on updates where the old check-in contract leaks into the current entry flow:
  - `src/customer-flow/screens/CheckInScreen.tsx`
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/session-domain/services/reservationValidation.ts`
  - `src/shared-contracts/dto/session.ts`
  - `src/shared-contracts/schemas/sessionSchemas.ts`
- Keep new or changed files in their owning domain directories. Do not push story-specific state logic into `shared-ui`.

### Testing Requirements

- Update the existing Story 1.1 screen test coverage in `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx` where needed, but keep Story 1.1 shell assertions intact.
- Update or replace the current legacy check-in integration coverage in `tests/integration/checkInFlow.test.tsx` so it validates the corrected single-field start behavior.
- Update `src/customer-flow/screens/CustomerFlowScreen.spec.tsx` to remove reservation/phone assumptions from the entry path and to verify the corrected Story 1.2 seam.
- Add focused validation tests for whitespace-only input, successful trimmed input, and “no legacy fields visible” behavior.
- If shared DTO/schema changes are required, add or update the nearest unit/contract tests rather than relying only on UI tests.

### Previous Story Intelligence

- Story 1.1 already established the reusable booth shell primitives and customer entry seam:
  - `CustomerStartScreen` already renders the approved shell container, lifecycle logging, and primary CTA.
  - `CustomerEntryScreen` already owns the local handoff from the shell into the next customer surface.
  - `HardFramePanel` and `PrimaryActionButton` are already in place and should be reused, not replaced.
- Story 1.1 completion notes show that later salvage work already pushed the app beyond the original baseline, including readiness, preset, capture, and review flows. Story 1.2 must therefore be surgical: correct the first actionable entry path without destabilizing later surfaces.
- The strongest regression risk is not visual styling. It is contract drift: the current entry path and flow tests still assume `reservationName + phoneSuffix` and host provisioning on submit. That assumption must be removed from the Story 1.2 path.

### Git Intelligence Summary

- Recent commits are dominated by repo restructuring and camera/readiness work, not by the corrected session-name-only entry definition.
- The latest visible commit titles indicate salvage-heavy evolution:
  - `그린필드 MVP 문서 패키지로 저장소 재구성`
  - `카메라 상태 흐름 정리 및 그린필드 재구축 문서화`
  - `카메라 연결 램프 정상`
- Practical implication: do not trust current customer entry implementation just because it is recent. Treat it as salvage code that must be revalidated against the 2026-03-12 PRD and sprint correction.

### Latest Technical Information

- Vite 7 official release notes confirm the current major line and the Node floor used by this repository: Node.js `20.19+` or `22.12+`. [Source: https://vite.dev/blog/announcing-vite7]
- React `19.2` is the current official release line and includes `useEffectEvent`, which is already used in this workspace and should remain acceptable in Story 1.2 code paths. [Source: https://react.dev/blog/2025/10/01/react-19-2]
- Tauri v2 official setup docs continue to support the frontend-first flow of creating a Vite app and then installing `@tauri-apps/cli@latest`. [Source: https://v2.tauri.app/start/create-project/]
- Tauri Store official docs show `pnpm tauri add store` as the standard setup path and require Rust `1.77.2+`. The existing branch-config baseline should therefore be preserved, not reinvented. [Source: https://v2.tauri.app/plugin/store/]
- React Router’s official changelog shows `7.13.1` as the latest release line as of 2026-02-23, but the project intentionally pins `7.9.4`; Story 1.2 should not mix entry-flow correction with router upgrades. [Source: https://reactrouter.com/changelog]
- Zod v4 release notes continue to support the project’s choice to stay on Zod 4, with substantial parsing and bundle-size improvements over Zod 3. [Source: https://zod.dev/v4]

### Project Context Reference

- Follow the project-context rules in `_bmad-output/project-context.md`:
  - keep React UI free of raw Tauri command strings
  - keep code domain-first
  - preserve typed cross-boundary DTOs
  - prefer adapter-layer mocks in tests
  - do not let routes, caches, or UI memory become session truth
- Story 1.2 is especially constrained by these rules:
  - no ad hoc global mutable store
  - no duplicated session DTO definitions across multiple files
  - no customer-facing diagnostics or internal-authoring language
  - no hidden branch-specific behavior in the customer start path

## Story Readiness

- Status: `done`
- Scope: clear and bounded to customer entry correction
- Primary implementation risk: current legacy check-in assumptions are spread through UI, state, and tests
- Primary guardrail: do not accidentally consume Story 1.3 scope by creating real host-side session provisioning here
- Reuse target: Story 1.1 shell, branch config baseline, shared UI primitives, and current customer entry seam

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Story context created from planning artifacts, current implementation files, and official framework/vendor documentation.
- Automated `validate-workflow.xml` task path referenced by the workflow was not present in `_bmad/core/tasks/`, so validation was completed manually against the loaded story-checklist intent.
- Followed a red-green loop for the review findings by first extending `CustomerStartScreen` and `checkIn` integration coverage, then implementing the flow/accessibility/touch-target fixes.
- Reconciled adjacent fixture drift exposed by the updated entry flow, including the sidecar protocol JSON schema snapshot and capture-adapter lint fallout.
- Verified targeted regressions with `pnpm vitest run tests/integration/presetSelectionFlow.test.tsx src/session-domain/state/SessionFlowProvider.story-3-4.spec.tsx --reporter=dot`.
- Re-verified the Story 1.2 review continuation against readiness and post-end regressions with `pnpm vitest run tests/integration/customerReadinessFlow.test.tsx tests/integration/postEndFlow.test.tsx src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx --reporter=dot`.
- Final verification passed with `pnpm vitest run --reporter=dot` (`63` files, `213` tests) and `pnpm lint`.

### Completion Notes List

- Fixed the reviewed dead-end by turning the handoff surface into an explicit confirmation seam with a continue CTA, busy-state accessibility, and provider wiring back into `submitCheckIn`.
- Kept inline validation visible for whitespace-only retries, preserved provider-driven field/form errors in `CheckInScreen`, and raised the primary CTA touch target to the booth baseline.
- Restored preset/capture progression by enabling capture-confidence subscription during preset selection, wiring the readiness preparation CTA back into provider state, and forcing an immediate snapshot refresh after preset confirmation succeeds.
- Restored provider-driven post-end resolution at the authoritative end threshold without reintroducing the far-future timer overflow path.
- Refreshed the committed sidecar protocol schema fixture to match the current DTO generator and cleaned up adjacent lint/test fallout surfaced while running the full suite.

### File List

- `_bmad-output/implementation-artifacts/1-2-booth-app-shell-and-session-start-screen.md`
- `src/capture-adapter/host/cameraAdapter.spec.ts`
- `sidecar/protocol/messages.schema.json`
- `src/customer-flow/copy/customerStartCopy.ts`
- `src/customer-flow/screens/CheckInScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`
- `src/customer-flow/screens/CustomerStartScreen.tsx`
- `src/customer-flow/screens/SessionStartHandoffScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/index.css`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/checkInFlow.test.tsx`

### Change Log

- Reworked the Story 1.2 handoff so valid session names now advance through an accessible confirmation seam and then resume the existing provisioning flow without dead-ending.
- Tightened the entry-screen validation and booth ergonomics by preserving whitespace validation, surfacing provider errors through the shared start form, and increasing the primary CTA touch target to `88px`.
- Added regression coverage for the reviewed issues, restored readiness/post-end integrations that regressed under the updated entry flow, refreshed the sidecar protocol schema snapshot, and re-verified the repository with full Vitest and lint passes.
