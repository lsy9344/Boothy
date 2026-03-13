# Story 4.1: Session Timing Model and Visible Adjusted End Time

Status: review

Story Key: `4-1-session-timing-model-and-visible-adjusted-end-time`

## Summary

Re-baseline the reopened Story 4.1 so one host-owned adjusted end time stays visibly correct from preparation through preset-selection and capture, and so fresher host timing cannot be overwritten by a slow initial read. This story stops at visible adjusted end-time truth only: warning/exact-end scheduling, ended-state copy, and post-end resolution remain with Stories 4.2 and 4.3.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a booth customer,
I want to see the adjusted session end time from the start,
so that I can trust how long I have.

## Acceptance Criteria

1. Given an active session already has an authoritative adjusted end time, when the customer is on preparation (including `phone-required`), preset-selection, or capture surfaces, then the same visible end-time value is rendered on each surface and reflects the current host-owned `actualShootEndAt`.
2. Given host timing changes after the session starts, when a newer capture-confidence snapshot or equivalent host-confirmed payload updates `actualShootEndAt`, then the visible end time updates on every currently visible customer surface without waiting for screen changes or leaving stale timing behind.
3. Given a slow `get_session_timing` response arrives after a newer host snapshot has already updated the same session, when the initial read resolves, then it must not overwrite the newer `sessionTiming` state for that session.
4. Given Story 4.1 is limited to visible adjusted end-time truth, when implementation for this story is completed, then warning/exact-end scheduling, ended-state copy, and post-end resolution behavior are either removed from this story's change set or explicitly deferred to Stories 4.2 and 4.3, and regression coverage includes preset-selection timing updates plus the slow-initial-read overwrite race.

## In Scope / Out of Scope

### In Scope

- Show one authoritative adjusted end time across preparation, `phone-required`, preset-selection, and capture surfaces.
- Keep `sessionTiming` synchronized from the existing typed timing read path and later host-confirmed snapshot updates.
- Prevent stale timing overwrites when older async reads complete after newer host state has already been applied.
- Add regression coverage for the reopened Story 4.1 timing-sync cases.

### Out of Scope

- 5-minute warning scheduling or exact-end alerts.
- Ended-state copy, post-end resolution, handoff, or recovery transitions.
- New countdown math, browser-owned timer models, or alert orchestration that belongs to Stories 4.2 and 4.3.

## Tasks / Subtasks

- [x] Keep one authoritative visible end-time path live on every customer surface that shows session timing. (AC: 1, 2)
  - [x] Ensure preset-selection and preparation `phone-required` states subscribe to the same fresh host timing updates already used elsewhere, so timing changes do not stay stale while those surfaces remain visible.
  - [x] Keep the visible value sourced from `timing.actualShootEndAt` through shared state and selectors; do not re-derive duration in screen components or revive the legacy `shootEndsAt` field as a parallel source.
  - [x] Preserve the compact trust-oriented copy and shared presentation treatment across preparation, preset-selection, and capture without introducing screen-specific timing caches.

- [x] Make `sessionTiming` freshness-safe when multiple host timing sources race. (AC: 2, 3)
  - [x] Update the provider/reducer merge path so a late `session_timing_loaded` result cannot overwrite newer timing that already arrived through `capture_confidence_updated` or another fresher host-confirmed update for the same session.
  - [x] Keep freshness handling session-scoped and reset-safe so stale payloads do not survive a session switch, clear, or manifest change.
  - [x] Document the precedence rule clearly in code comments/tests: newer host-confirmed timing wins over older async reads for the active session.

- [x] Re-cut Story 4.1 back to visible adjusted end-time truth only. (AC: 4)
  - [x] Remove, disable, or explicitly defer warning/exact-end scheduling logic from the Story 4.1 implementation path if that behavior was introduced under this story.
  - [x] Remove, disable, or explicitly defer ended-state copy and post-end resolution behavior from the Story 4.1 implementation path so closure of this story does not depend on Story 4.2/4.3 work.
  - [x] Keep centralized timing formatting and customer-safe wording in selector/presentation seams, but do not add alerting or post-end workflow responsibilities here.

- [x] Add regression coverage for the reopened review findings. (AC: 2, 3, 4)
  - [x] Add frontend/integration coverage proving preset-selection reflects a timing update while that surface remains visible.
  - [x] Add coverage for preparation `phone-required` timing visibility/update behavior so host changes do not require navigating away to refresh the visible end time.
  - [x] Add a reducer/provider race test proving a slow initial `get_session_timing` response cannot overwrite a newer snapshot-driven `sessionTiming` value for the same active session.
  - [x] Keep assertions focused on the visible adjusted end time and freshness rules, not on warning/end scheduling or post-end flows.

### Review Follow-ups (AI)

- [x] Close the preset-selection and `phone-required` stale-timing gap before returning this story to review.
- [x] Close the slow initial timing read overwrite race before returning this story to review.
- [x] Remove Story 4.2/4.3 alert and post-end responsibilities from Story 4.1 completion criteria.
- [x] Land the new regression cases for preset-selection live updates and the slow-read overwrite race.

## Dev Notes

### Developer Context

- Epic 4 is the first timing-focused epic in the rewritten plan, but this repo is not starting from zero:
  - TypeScript already defines a typed timing contract in `src/shared-contracts/dto/sessionTiming.ts` and `src/shared-contracts/schemas/sessionTimingSchemas.ts`.
  - The session manifest already persists authoritative timing under `timing.actualShootEndAt`.
  - Rust timing logic already exists in `src-tauri/src/timing/shoot_end.rs` and `src-tauri/src/timing/extension_rules.rs`.
  - `CaptureScreen` already renders a time banner, but only after timing arrives indirectly through the capture-confidence view model.
- Current gap:
  - Story 4.1 already introduced visible timing on multiple customer surfaces, but the host timing resync path is still too narrow and can leave preset-selection plus preparation `phone-required` showing stale end times.
  - `SessionFlowProvider.tsx` still accepts `session_timing_loaded` too eagerly, so a slower initial read can overwrite a newer capture-confidence timing update for the same session.
  - The current implementation still carries warning/end scheduling and post-end behavior that belong to Stories 4.2 and 4.3, which makes Story 4.1 harder to close cleanly.
- Important scope boundary:
  - in scope: visible adjusted end time, freshness-safe display updates on preparation/`phone-required`/preset-selection/capture, and reuse of the authoritative timing contracts
  - out of scope: 5-minute warning behavior, exact-end alerts, ended-state copy, shoot-stop/post-end transition rules, export-waiting/completed/handoff states, or operator recovery UI
- Planning caution:
  - `sprint-status.yaml` still carries the approved 2026-03-12 planning-alignment correction notes.
  - Treat this story as aligned to the current rewritten Epic 4 baseline only; do not reuse superseded timing assumptions from the archived pre-reset snapshot.

### Technical Requirements

- Use one timing source of truth end-to-end:
  - manifest persistence via `timing.actualShootEndAt`
  - typed frontend reads via `sessionTimingService`
  - host-confirmed updates via capture-confidence snapshots
- The customer display must reflect host-owned timing truth. React may format that value for presentation, but it must not recalculate booth duration from reservation rules inside the screen layer.
- Reuse the existing time-formatting seam:
  - `src/timing-policy/selectors/sessionTimeDisplay.ts`
  - if new display variants are needed, extend this selector layer instead of adding new `Intl.DateTimeFormat` instances across screens
- Keep update behavior precise:
  - initial session timing should be available as soon as the active session is known
  - later host changes should replace the displayed end time consistently on preparation, `phone-required`, preset-selection, and capture
  - slower initial reads must not overwrite fresher snapshot-driven timing for the same session
  - stale timing must not survive a session switch or session clear
- Keep customer wording simple and within the PRD/UX copy budget:
  - one clear end-time label/value
  - no diagnostic explanation of coupon math, host commands, or internal scheduling details
- Do not introduce a parallel browser-owned timer model for this story. Countdown, alert scheduling, ended-state copy, and post-end workflow behavior belong to later stories and to host-owned workflow rules.
- Guard against an existing schema trap:
  - `src/shared-contracts/dto/sessionManifest.ts` still includes an optional `shootEndsAt` field
  - the implementation should continue to treat `timing.actualShootEndAt` as authoritative and avoid splitting timing reads across both fields

### Architecture Compliance

- Keep the Rust host and manifest as the durable source of timing truth. Do not shift authority into route state, local storage, or component-local clocks.
- React Router remains limited to top-level surfaces. Timing visibility is state-driven inside the current customer flow, not a new route or navigation concern.
- React components must stay out of direct Tauri calls. Timing reads and updates belong in typed services and provider/state seams.
- Preserve the existing architecture rule that customer-visible translation is centralized and diagnostics-free.
- Keep this story isolated from later alerting/post-end behavior:
  - Story 4.1 = visible adjusted end-time truth
  - Story 4.2 = warning and exact-end alerts
  - Story 4.3 = post-end transition and guidance
- If current code paths already mix these concerns, this story should remove or fence that logic instead of expanding it.

### Library / Framework Requirements

- Current workspace baselines in this repo:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `react-router`: `7.9.4`
  - `@tauri-apps/api`: `^2.10.1`
  - `@tauri-apps/cli`: `2.10.1`
  - Rust `tauri`: `2.10.3`
  - `zod`: `^4.3.6`
- Official-source verification performed on 2026-03-12:
  - React 19.2 documentation still supports `useEffectEvent` for effect-driven logic that must read fresh state without spreading unstable callback dependencies. Continue following the current provider pattern instead of adding event-bus style timing plumbing.
  - React 19.2.1 is the latest official security patch line, but this story is not a dependency-upgrade task. Keep implementation compatible with the current React 19.2.x code already in the repo.
  - React Router's current official changelog has moved beyond the locally pinned `7.9.4`, but this story should not upgrade routing packages just to surface timing UI; keep route structure unchanged.
  - Tauri v2 official docs still position commands as the standard frontend-to-Rust request/response boundary. Timing reads should remain on `get_session_timing` rather than direct file access from the UI.
  - Tauri configuration guidance still recommends keeping Tauri package versions aligned within the same minor line; the current workspace already sits on the 2.10.x minor and should stay there for this story.
  - Zod 4 remains the active official docs line and should continue to validate timing payloads/results at the TypeScript boundary.

### File Structure Requirements

- Primary TypeScript seams likely to change:
  - `src/session-domain/state/SessionFlowProvider.tsx`
  - `src/session-domain/state/sessionReducer.ts`
  - `src/customer-flow/screens/CustomerFlowScreen.tsx`
  - `src/customer-flow/screens/PreparationScreen.tsx`
  - `src/customer-flow/screens/PresetScreen.tsx`
  - `src/customer-flow/screens/CaptureScreen.tsx`
  - `src/customer-flow/components/SessionTimeBanner.tsx`
  - `src/timing-policy/selectors/sessionTimeDisplay.ts`
- Existing timing and contract surfaces that should be reused, not replaced:
  - `src/timing-policy/services/sessionTimingService.ts`
  - `src/timing-policy/state/timingSelectors.ts`
  - `src/shared-contracts/dto/sessionTiming.ts`
  - `src/shared-contracts/schemas/sessionTimingSchemas.ts`
  - `src/shared-contracts/dto/sessionManifest.ts`
  - `src/customer-flow/selectors/captureConfidenceView.ts`
- Host files that are relevant for guardrails and regression verification:
  - `src-tauri/src/timing/shoot_end.rs`
  - `src-tauri/src/timing/extension_rules.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/capture/camera_host.rs`
- Test surfaces likely needing updates:
  - `tests/integration/sessionLifecycle.test.ts`
  - `tests/integration/customerReadinessFlow.test.tsx`
  - `tests/integration/presetSelectionFlow.test.tsx`
  - `src/customer-flow/selectors/captureConfidenceView.spec.ts`
  - `src/customer-flow/screens/CaptureScreen.spec.tsx`
  - `src/timing-policy/state/timingSelectors.spec.ts`
  - `src-tauri/tests/session_timing_repository.rs`
  - `src-tauri/tests/camera_contract.rs`
- Keep timing work out of:
  - `src/shared-ui/*` except presentation-only reuse
  - new route trees
  - new persistence stores that duplicate manifest timing

### Testing Requirements

- Add or update frontend tests so the adjusted end time is visible in:
  - preparation flow after session creation
  - preparation `phone-required` flow while timing updates arrive
  - preset-selection flow before first capture and while that surface remains active
  - capture-ready flow using the same authoritative timing source
- Add provider/integration coverage proving:
  - `get_session_timing` loads once the active session exists
  - timing state is cleared when the active session clears
  - a later host timing update replaces the rendered value consistently
  - a slower initial `get_session_timing` completion cannot overwrite fresher snapshot timing for the same active session
- Keep selector tests explicit:
  - `selectSessionTimeDisplay()` should remain the only customer-facing formatter
  - timing selectors/providers preserve latest-authoritative end-time precedence without UI recalculation drift
- Keep or extend host/contract tests so:
  - session provisioning preserves initial timing in the manifest
  - operator extension updates `actualShootEndAt`
  - capture-confidence snapshots continue projecting the manifest timing field used by the UI
- Do not rely on visual snapshots alone. Assertions should explicitly check the displayed adjusted end-time value, the visible update on preset/preparation surfaces, and the slow-read race outcome.

### Previous Story Intelligence

- There is no earlier Epic 4 implementation-artifact story yet.
- The practical predecessor intelligence comes from the current repo seams:
  - session creation already writes authoritative timing into the manifest
  - capture-confidence snapshots already expose `shootEndsAt`
  - timing thresholds are already derived from stored host values, not component-level calculations
- Treat those as established precedent and extend them carefully rather than rebuilding timing from scratch.

### Git Intelligence Summary

- Recent git history is still dominated by the March 2026 greenfield reset and camera/readiness stabilization, not by completed Epic 4 timing UI work.
- Actionable guidance from the current repo state:
  - timing contracts already exist in both TypeScript and Rust
  - the missing work is orchestration and customer-surface visibility, not timing-duration math
  - keep UI truth derived from host-normalized state, consistent with the broader camera/capture patterns already present in the branch

### Latest Tech Information

Verified against official docs on 2026-03-12:

- React 19.2 docs continue to document `useEffectEvent` for effect-driven event logic that needs fresh state reads without over-widening dependencies. This matches the current `SessionFlowProvider` implementation style. [Source: https://react.dev/blog/2025/10/01/react-19-2] [Source: https://react.dev/reference/react/useEffectEvent]
- React 19.2.1 is the latest official React security patch line as of March 12, 2026, but the published vulnerability concerns React Server Components and the repo is a Tauri SPA. No dependency upgrade is required to implement Story 4.1. [Source: https://react.dev/blog/2025/12/03/critical-security-vulnerability-in-react-server-components]
- React Router's official changelog is already beyond the repo's pinned `7.9.4`; this story should remain route-stable and avoid package upgrades unrelated to timing visibility. [Source: https://reactrouter.com/changelog]
- Tauri's official manual project-setup docs still support the current `Vite + manual tauri init` foundation, and the official "Calling Rust" docs still describe commands as the standard boundary for frontend-to-host requests. [Source: https://v2.tauri.app/start/create-project/] [Source: https://v2.tauri.app/develop/calling-rust/]
- Tauri's official configuration guidance still recommends keeping Tauri packages current within the same minor line; the workspace is already aligned on 2.10.x and should stay on that line for this story. [Source: https://v2.tauri.app/develop/configuration-files/]
- Zod 4 remains the current official docs line for schema validation. Continue validating timing payloads/results at the TypeScript boundary instead of adding ad hoc guards. [Source: https://zod.dev/v4]

### Project Structure Notes

- The current repo already has the right domain seams for this story:
  - `session-domain` owns active session orchestration
  - `timing-policy` owns timing selectors/services
  - `customer-flow` owns customer-screen rendering
  - `src-tauri/src/timing` and `src-tauri/src/session` own host timing truth
- There is one important local caveat:
  - visible timing has already been threaded into the customer flow, but the live resync path still favors capture-active states
  - preset-selection and preparation `phone-required` remain the specific surfaces most likely to show stale timing unless the provider wiring is corrected
  - Story 4.1 should finish by unifying these surfaces around one freshness-safe timing pipeline rather than duplicating capture-specific behavior elsewhere

### Project Context Reference

- `_bmad-output/project-context.md` remains active guidance for this story.
- The highest-signal rules from that file for Story 4.1 are:
  - keep React components away from direct Tauri invocation
  - preserve session folders/manifests as the durable source of truth
  - keep shared DTOs fully typed and validated with Zod
  - keep routes limited to top-level surfaces
  - avoid branch-local or UI-local workflow shortcuts that drift from host truth

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/project-context.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `package.json`
- `src-tauri/Cargo.toml`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/PreparationScreen.tsx`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/customer-flow/screens/CaptureScreen.tsx`
- `src/customer-flow/components/SessionTimeBanner.tsx`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/timing-policy/services/sessionTimingService.ts`
- `src/timing-policy/state/timingSelectors.ts`
- `src/timing-policy/selectors/sessionTimeDisplay.ts`
- `src/shared-contracts/dto/sessionTiming.ts`
- `src/shared-contracts/schemas/sessionTimingSchemas.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/session-domain/services/sessionManifest.ts`
- `src/session-domain/services/sessionLifecycle.ts`
- `src-tauri/src/timing/shoot_end.rs`
- `src-tauri/src/timing/extension_rules.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/capture/camera_host.rs`
- `tests/integration/sessionLifecycle.test.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- `src/customer-flow/selectors/captureConfidenceView.spec.ts`
- `src/customer-flow/screens/CaptureScreen.spec.tsx`
- `src/timing-policy/state/timingSelectors.spec.ts`
- `src-tauri/tests/session_timing_repository.rs`
- `src-tauri/tests/camera_contract.rs`
- React 19.2 official release: https://react.dev/blog/2025/10/01/react-19-2
- React `useEffectEvent` reference: https://react.dev/reference/react/useEffectEvent
- React 19.2.1 security release context: https://react.dev/blog/2025/12/03/critical-security-vulnerability-in-react-server-components
- React Router changelog: https://reactrouter.com/changelog
- Tauri create-project docs: https://v2.tauri.app/start/create-project/
- Tauri calling Rust docs: https://v2.tauri.app/develop/calling-rust/
- Tauri configuration guidance: https://v2.tauri.app/develop/configuration-files/
- Zod 4 docs: https://zod.dev/v4

## Story Readiness

- Status: `review`
- Primary implementation goal: maintain one freshness-safe visible end-time truth across all customer timing surfaces after closing the reopened review findings
- Reuse strategy: extend the current manifest/timing-service/capture-confidence pipeline instead of inventing new timing math or alert orchestration
- Contract sensitivity: medium-high because customer UI, session manifest timing, and capture-confidence snapshots must stay aligned
- Key guardrail: do not mix this story with warning/end alerts, ended-state copy, or post-end workflow state changes from later Epic 4 stories

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Implementation Plan

- Keep the existing visible timing baseline, but reopen the provider/view wiring so preset-selection and preparation `phone-required` stay synchronized with later host timing updates.
- Add freshness guards so reducer/provider logic rejects older `get_session_timing` completions when newer snapshot timing is already present for the same session.
- Remove or defer warning/end scheduling and post-end behavior from this story's implementation path, then add regression coverage for the preset-surface update and slow-read race cases.

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/dev-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Manual validation performed against `_bmad/bmm/workflows/4-implementation/dev-story/checklist.md`
- Story 4.1 focused regression suite passed on 2026-03-13: `pnpm vitest run src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx src/session-domain/state/sessionReducer.story-4-1.spec.ts tests/integration/presetSelectionFlow.test.tsx tests/integration/customerReadinessFlow.test.tsx`
- Story 4.1 related selector/screen suite passed on 2026-03-13: `pnpm vitest run src/customer-flow/selectors/captureConfidenceView.spec.ts src/customer-flow/screens/CaptureScreen.spec.tsx src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
- Lint passed on 2026-03-13: `pnpm lint`
- Legacy full-suite note retained from earlier verification: `pnpm test:run` on 2026-03-13 surfaced unrelated existing failures in `src/App.spec.tsx`, `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`, `tests/integration/checkInFlow.test.tsx`, and `tests/integration/sessionPresetChangeFlow.test.tsx`

### Follow-up Notes List

- Story 4.1 review-fix pass is complete and the story is returned to `review`.
- Visible end-time rendering now stays on the `sessionTiming.actualShootEndAt` path across preparation, `phone-required`, preset-selection, and capture.
- Fresher host-confirmed timing now wins over older async timing reads for the active session.
- Warning/exact-end alerts, ended copy, and post-end resolution were removed from this story's active implementation path and deferred back to Stories 4.2 and 4.3.
- Regression coverage now includes preset-selection live updates, `phone-required` timing refresh, and a slow initial read overwrite race at reducer/provider level.
- Unrelated legacy failures in Story 1.1 / entry-flow and one Story 3.4 preset-change suite remain outside this story unless they block the new timing regressions directly.

### Completion Notes

- Closed the stale-timing gaps on preset-selection and `phone-required` by keeping capture-confidence timing sync active on those visible pre-capture surfaces without reviving parallel display sources.
- Added freshness guards so an older `session_timing_loaded` payload cannot overwrite a newer snapshot-driven `sessionTiming` revision for the same active session.
- Re-cut the Story 4.1 presentation seam so capture copy stays trust-oriented and no longer projects Story 4.2/4.3 warning/ended messaging.
- Added and passed the reopened regression set plus the touched selector/screen suite, then re-ran lint successfully.

### File List

- `_bmad-output/implementation-artifacts/4-1-session-timing-model-and-visible-adjusted-end-time.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
- `src/customer-flow/screens/customerFlowView.ts`
- `src/customer-flow/screens/PreparationScreen.tsx`
- `src/customer-flow/screens/PresetScreen.tsx`
- `src/customer-flow/screens/PresetSelectionSurface.tsx`
- `src/customer-flow/selectors/captureConfidenceView.ts`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx`
- `src/session-domain/state/sessionReducer.ts`
- `src/timing-policy/selectors/sessionTimeDisplay.ts`
- `src/customer-flow/screens/CaptureScreen.spec.tsx`
- `src/customer-flow/selectors/captureConfidenceView.spec.ts`
- `src/session-domain/state/sessionReducer.story-4-1.spec.ts`
- `tests/integration/customerReadinessFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `tests/integration/sessionPresetChangeFlow.test.tsx`

### Senior Developer Review (AI)

- Reviewer: Noah Lee
- Date: 2026-03-13
- Outcome: Changes requested
- Review notes:
  - `SessionFlowProvider` still does not keep timing resync alive on every visible pre-capture surface. `isCaptureConfidenceActive` requires an active preset and excludes `phone-required`, so coupon/extension updates can remain stale on preset-selection and phone-required preparation screens even though those screens render the session end time.
  - `session_timing_loaded` still overwrites reducer state unconditionally for the active session. A slower `get_session_timing` response can therefore regress a newer host snapshot that already updated `sessionTiming` through `capture_confidence_updated`.
  - Story 4.1 scope is still overrun by later-story timing behavior. The provider schedules warning/end alerts and post-end resolution logic, and the capture view still projects warning/end copy, even though this story's guardrail explicitly says not to mix in Story 4.2/4.3 alert or post-end behavior.
  - Existing Story 4.1 regression tests cover the fixed capture-ready and waiting/preparing cases, but they still do not exercise preset-selection timing updates or the slow-initial-read race described above.

### Change Log

- 2026-03-13: surfaced the authoritative adjusted end time across preparation, preset-selection, and capture screens; synchronized session timing from host updates; added regression coverage and full repo verification for timing visibility and capture gating.
- 2026-03-13: resolved code-review findings by fixing capture banner copy merging, host-snapshot timing seeding, waiting/preparing timing resync, and live end-time capture gating.
- 2026-03-13: follow-up code review reopened Story 4.1 after finding remaining preset-selection/phone-required timing drift, a slow-read timing overwrite race, and out-of-scope warning/post-end behavior.
- 2026-03-13: re-baselined Story 4.1 around the reopened review findings, clarified in-scope vs out-of-scope work, and converted the remaining implementation work into a dev-ready follow-up checklist.
- 2026-03-13: closed the reopened Story 4.1 follow-ups by fixing preset-selection/`phone-required` timing freshness, blocking stale slow-read overwrites, and deferring warning/post-end behavior back to Stories 4.2 and 4.3.
