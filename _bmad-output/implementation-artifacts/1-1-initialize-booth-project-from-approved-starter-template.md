# Story 1.1: Initialize Booth Project from Approved Starter Template

Status: done

Story Key: `1-1-initialize-booth-project-from-approved-starter-template`

## Summary

Establish and preserve the approved Boothy foundation: a Vite React + TypeScript frontend paired with a manually initialized Tauri host, using the exact starter commands and config values approved in planning. Because the current repository already contains later domain work on top of that foundation, implementation for this story is a narrow salvage-and-verification pass: confirm the starter baseline, fix only starter/config drift, and do not tear down later customer-flow, session, or diagnostics code.

## Story

As a developer,
I want to initialize the Boothy project using the approved Vite + Tauri starter,
so that the codebase follows the architecture baseline from the first commit.

## Acceptance Criteria

1. Given a new project folder is created, when the following commands are executed in order, then the repo is initialized with Vite React + TypeScript and a Tauri app: `pnpm create vite boothy --template react-ts --no-interactive`, `pnpm add -D @tauri-apps/cli@latest`, `pnpm exec tauri init`.
2. Given the Tauri initialization is completed, when `src-tauri/tauri.conf.json` is configured, then App name is `Boothy`, Window title is `Boothy`, Web assets location is `../dist`, Dev server URL is `http://localhost:5173`, Frontend dev command is `pnpm dev`, and Frontend build command is `pnpm build`.
3. Given the starter initialization completes, when dependencies are installed, then the project builds successfully with `pnpm build`.
4. Given the starter baseline is in place, when later booth features are added, then the starter foundation remains aligned with the approved architecture baseline instead of being replaced by an alternate scaffold.

## Tasks / Subtasks

- [x] Confirm the root starter scaffold matches the approved Vite React + TypeScript baseline. (AC: 1, 3)
  - [x] Keep `package.json` on the Vite + React + TypeScript path with the standard frontend scripts required by the Tauri host: `dev`, `build`, `preview`, and `tauri`.
  - [x] Preserve a minimal starter-compatible root entry set: `index.html`, `src/main.tsx`, `src/App.tsx`, `vite.config.ts`, and TypeScript config files.
  - [x] If starter drift exists, repair the scaffold in place instead of re-running generators over the current repository.
- [x] Confirm the Tauri host initialization matches the approved manual-init configuration. (AC: 1, 2)
  - [x] Keep `@tauri-apps/cli` installed as a dev dependency and `src-tauri/` present as the host boundary.
  - [x] Verify `src-tauri/tauri.conf.json` uses `Boothy` naming plus `frontendDist: ../dist`, `devUrl: http://localhost:5173`, `beforeDevCommand: pnpm dev`, and `beforeBuildCommand: pnpm build`.
  - [x] Preserve the packaged desktop entry instead of swapping to Electron, Next.js, or another runtime.
- [x] Preserve the Story 1.1 foundation that later stories already build on. (AC: 4)
  - [x] Keep the current root app boot path (`src/main.tsx` -> `src/App.tsx`) intact unless starter alignment requires a small correction.
  - [x] Reuse the existing shell primitives and Story 1.1 smoke coverage rather than rebuilding the customer entry shell from scratch.
  - [x] Do not pull Story 1.2+ scope into this story; session-name validation, session provisioning, preset selection, and capture behavior remain separate stories.
- [x] Add or maintain baseline verification for the starter foundation. (AC: 2, 3, 4)
  - [x] Keep or update Story 1.1 smoke coverage proving the baseline customer shell renders and the primary action wiring is intact.
  - [x] Run `pnpm build` as the minimum required verification step after any starter/config correction.
  - [x] If Tauri configuration changes, perform a `pnpm tauri dev` smoke check or document why it could not be run in the current environment.

## Dev Notes

### Developer Context

- This story is foundational, but the repository is not empty anymore. Current code already contains later customer-flow, session-domain, diagnostics, and capture-related work layered on top of the starter baseline.
- Treat Story 1.1 as a salvage-and-verification story, not a destructive resync. If the approved starter config is already present, preserve it and move on; do not delete later domain code just to recreate a cleaner scaffold.
- Later stories already depend on a “Story 1.1 shell foundation.” In the current repo that baseline includes:
  - `src/customer-flow/screens/CustomerStartScreen.tsx`
  - `src/shared-ui/components/HardFramePanel.tsx`
  - `src/shared-ui/components/PrimaryActionButton.tsx`
  - `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`
- Scope boundary:
  - In scope: starter-template alignment, root boot/config verification, and baseline shell/smoke-test preservation.
  - Out of scope: session-name-only input correction (Story 1.2), durable session creation (Story 1.3), active-session handoff (Story 1.4), preset selection, capture, timing, diagnostics, or internal preset authoring.

### Technical Requirements

- The approved starter command sequence remains the canonical initialization path for this project:
  - `pnpm create vite boothy --template react-ts --no-interactive`
  - `pnpm add -D @tauri-apps/cli@latest`
  - `pnpm exec tauri init`
- The current repository must preserve the same baseline outcome even if implementation has advanced beyond the initial scaffold:
  - root package name `boothy`
  - frontend entry through Vite + React + TypeScript
  - host boundary under `src-tauri/`
  - `pnpm build` producing `dist/` for Tauri consumption
- Required `src-tauri/tauri.conf.json` values for this story:
  - `productName: "Boothy"`
  - window title `Boothy`
  - `build.frontendDist: "../dist"`
  - `build.devUrl: "http://localhost:5173"`
  - `build.beforeDevCommand: "pnpm dev"`
  - `build.beforeBuildCommand: "pnpm build"`
- Keep the root frontend boot chain simple and starter-compatible:
  - `src/main.tsx` mounts the React app
  - `src/App.tsx` remains the top-level composition entry
  - `vite.config.ts` keeps port `5173` and Tauri-safe watch behavior for `src-tauri`
- Preserve the Rust/Tauri baseline already present in the repo:
  - `src-tauri/Cargo.toml` uses Tauri 2.x and Rust version `1.77.2`
  - do not introduce a parallel desktop host runtime or move desktop orchestration into the frontend

### Architecture Compliance

- Preserve the architecture decision that Boothy is one packaged Tauri desktop app with a React SPA frontend and a Rust host boundary. Do not replace the starter with Electron, Next.js, or a browser-only runtime.
- Keep React components free of direct raw Tauri orchestration. Story 1.1 may establish the shell and boot path, but host communication patterns still belong behind typed adapters/services in later stories.
- Maintain domain-first structure already growing from the starter baseline. This story should not flatten the repo back into a toy single-file app just because it is foundational.
- Keep React Router limited to top-level surface entry. Do not use Story 1.1 to encode booth workflow progression into routes.
- Do not let starter cleanup destroy the existing separation between customer flow, shared UI, session domain, diagnostics, and `src-tauri` host modules.

### Library / Framework Requirements

- Current workspace baselines already aligned to this story:
  - `react`: `^19.2.0`
  - `react-dom`: `^19.2.0`
  - `typescript`: `~5.9.3`
  - `vite`: `^7.3.1`
  - `@vitejs/plugin-react`: `^5.1.1`
  - `@tauri-apps/cli`: `2.10.1`
  - `@tauri-apps/api`: `^2.10.1`
  - `react-router`: `7.9.4`
- Node runtime baseline is already declared in `package.json` and should remain intact: `>=20.19.0 <21 || >=22.12.0`.
- Rust host baseline is already declared in `src-tauri/Cargo.toml` and should remain intact:
  - `rust-version = "1.77.2"`
  - `tauri = "2.10.3"`
- Keep Story 1.1 focused on starter alignment. Do not turn it into a dependency-upgrade sweep across React Router, Tailwind, testing libraries, or Tauri plugins unless a starter-breaking incompatibility is proven.
- If an initialization/config issue is found, fix it against the current major lines already used by the repo rather than backsliding to older Tauri 1.x or pre-Vite-7 patterns.

### File Structure Requirements

- Primary files for Story 1.1 alignment:
  - `package.json`
  - `vite.config.ts`
  - `src/main.tsx`
  - `src/App.tsx`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/Cargo.toml`
- Existing Story 1.1 shell foundation to preserve:
  - `src/customer-flow/screens/CustomerStartScreen.tsx`
  - `src/shared-ui/components/HardFramePanel.tsx`
  - `src/shared-ui/components/PrimaryActionButton.tsx`
  - `src/customer-flow/copy/customerStartCopy.ts`
  - `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`
- Keep starter-related corrections close to the actual boot/config files. Do not scatter Story 1.1 logic into unrelated domains such as `capture-adapter`, `timing-policy`, `operator-console`, or `preset-authoring`.
- Avoid moving or renaming top-level starter files unless required by a real build/config break. Later stories already assume the current root layout exists.

### Testing Requirements

- Minimum verification after any Story 1.1 change:
  - `pnpm build`
- Preferred smoke verification when starter or Tauri config changed:
  - `pnpm tauri dev`
- Preserve or update the existing Story 1.1 UI smoke test at `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`.
- Keep Story 1.1 assertions focused on baseline behavior:
  - approved first-screen copy renders
  - primary action button renders and fires
  - lifecycle logging still goes through the typed log adapter
- Do not expand Story 1.1 tests into later-story concerns like session provisioning, preset selection, capture persistence, or timing workflows.

### Git Intelligence Summary

- Recent commits show the repository has already moved well beyond starter scaffolding:
  - `06ed2b7` `그린필드 MVP 문서 패키지로 저장소 재구성`
  - `1fb8bb0` `카메라 상태 흐름 정리 및 그린필드 재구축 문서화`
  - `3ef405f` `카메라 연결 램프 정상`
- Practical implication: Story 1.1 should not be implemented by regenerating a fresh app over the top of the repo. Preserve the approved foundation and repair only root-level starter/config drift.
- The strongest regression risk is destructive cleanup, not missing starter files. Avoid “reset to template” behavior that would wipe later story work.

### Latest Technical Information

Verified against official documentation on 2026-03-12:

- Vite's official guide continues to document `pnpm create vite` and the `react-ts` template flow, which matches the approved Story 1.1 initialization command. [Source: https://vite.dev/guide/]
- Vite 7 official release notes confirm the active Node baseline used by this repository: Node.js `20.19+` or `22.12+`. Keep the `package.json` engines field aligned to that floor. [Source: https://vite.dev/blog/announcing-vite7]
- Tauri v2 official project-creation docs still support the frontend-first flow used in planning: create the frontend app, add `@tauri-apps/cli@latest`, then initialize Tauri. [Source: https://v2.tauri.app/start/create-project/]
- Tauri's Vite integration docs continue to use the same development/build bridge this repo already has: `frontendDist`, `devUrl`, `beforeDevCommand`, and `beforeBuildCommand` in `src-tauri/tauri.conf.json`. [Source: https://v2.tauri.app/start/frontend/vite/]
- React 19.2 is the current official release line and remains compatible with the existing root app boot path and `StrictMode` mount used in `src/main.tsx`. [Source: https://react.dev/blog/2025/10/01/react-19-2]

### Project Context Reference

- Follow the active guardrails from `_bmad-output/project-context.md`:
  - keep React code out of raw Tauri command strings
  - preserve typed cross-boundary contracts
  - keep code domain-first
  - treat the desktop host boundary as authoritative for native behavior
  - avoid introducing alternate sources of truth in routes, UI memory, or ad hoc utilities
- Story 1.1-specific implication: even though this is the starter story, do not regress the repo back to a simplistic scaffold that violates the current boundary rules.

### References

- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/project-context.md`
- `package.json`
- `vite.config.ts`
- `src/main.tsx`
- `src/App.tsx`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src/customer-flow/screens/CustomerStartScreen.tsx`
- `src/shared-ui/components/HardFramePanel.tsx`
- `src/shared-ui/components/PrimaryActionButton.tsx`
- `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`
- Vite guide: https://vite.dev/guide/
- Vite 7 release notes: https://vite.dev/blog/announcing-vite7
- Tauri v2 create-project docs: https://v2.tauri.app/start/create-project/
- Tauri v2 Vite integration docs: https://v2.tauri.app/start/frontend/vite/
- React 19.2 release notes: https://react.dev/blog/2025/10/01/react-19-2

## Story Readiness

- Status: `review`
- Scope: starter-template alignment and baseline shell preservation only
- Primary regression risk: destructive “reset to template” cleanup over an already advanced repository
- Primary guardrail: preserve the approved Vite + Tauri root foundation without deleting later domain work
- Reuse targets: current root boot/config files plus the existing Story 1.1 customer shell primitives and smoke test

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Workflow: `_bmad/bmm/workflows/4-implementation/dev-story`
- Sprint tracking input: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Review remediation executed after `_bmad/bmm/workflows/4-implementation/code-review`
- Repository audit caveat: `git status --porcelain` still reports most application sources as untracked, so the file list below is maintained manually from the remediation work instead of `git diff`
- Verified Story 1.1 baseline inputs: `package.json`, `vite.config.ts`, `src/main.tsx`, `src/App.tsx`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`
- Story 1.1 boundary verification: `pnpm vitest run src/App.spec.tsx src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx tests/integration/checkInFlow.test.tsx tests/integration/sessionEntryFlow.test.tsx`
- Provider and downstream regression verification: `pnpm vitest run tests/integration/presetSelectionFlow.test.tsx tests/integration/sessionGalleryIsolation.test.tsx src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx src/session-domain/state/SessionFlowProvider.preset-catalog.spec.tsx tests/integration/postEndFlow.test.tsx`
- Additional downstream stabilization verification: `pnpm vitest run src/capture-adapter/host/cameraAdapter.spec.ts tests/integration/sessionPresetChangeFlow.test.tsx`
- Full regression verification: `pnpm test:run`, `pnpm build`, and `pnpm lint`

### Implementation Plan

- Treat Story 1.1 as a salvage-and-verification pass only; confirm the approved Vite + React + TypeScript plus Tauri foundation already present in the repo instead of regenerating scaffolding.
- Reuse the existing Story 1.1 shell coverage and restore the Story 1.1 and Story 1.2 boundary where later work had collapsed the starter shell into the session-name form.
- Keep `startJourney` UI-bounded at the Story 1.1 shell seam and require check-in submission to drive validation and provisioning.
- Limit production edits to the minimum surfaces needed to restore the starter shell, preserve downstream check-in behavior, and remove inaccurate review-trail claims.

### Completion Notes List

- Story 1.1 was regenerated as a salvage-aware foundation story because the sprint tracker already referenced it as `ready-for-dev` but the corresponding implementation artifact file was missing.
- Current repo inspection shows the approved starter baseline is already present in `package.json`, `vite.config.ts`, `src/main.tsx`, `src/App.tsx`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.
- The story explicitly documents the existing shell primitives and smoke test so later Story 1.x work can continue reusing them instead of rebuilding the baseline.
- Official Vite, Tauri, and React documentation was rechecked on 2026-03-12 for version-sensitive starter guidance.
- Post-review remediation on 2026-03-13 restored the Story 1.1 starter shell boundary after later session-name-entry behavior had been reintroduced into `CustomerStartScreen`, the app shell smoke tests, and customer entry integration tests.
- Implementation result: production code changed only where the Story 1.1 boundary had regressed or where full-suite verification surfaced boundary-adjacent regressions. `CustomerStartScreen` remains a CTA-only starter shell, `startJourney` no longer seeds or auto-provisions session names, handoff copy no longer reuses starter-shell field labels, and check-in owns session-name submission while downstream preset/timing test harnesses now mirror that contract through `updateField(...)` plus `submitCheckIn()`.
- Downstream stabilization remained scope-preserving: `CustomerFlowScreen` now forwards the active session-time display consistently, `SessionFlowProvider.requestCapture()` reads the latest state before host capture requests, and `cameraAdapter` normalizes capture-confidence preview paths for the UI.
- Validation result: targeted Story 1.1 and downstream regression tests passed, the full Vitest suite passed (`60` files, `176` tests), `pnpm build` succeeded, and `pnpm lint` completed cleanly.
- Tauri configuration did not change during implementation, so an additional `pnpm tauri dev` smoke run was not required by this story's test gate.

### File List

- `_bmad-output/implementation-artifacts/1-1-initialize-booth-project-from-approved-starter-template.md`
- `src/App.spec.tsx`
- `src/capture-adapter/host/cameraAdapter.spec.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/customer-flow/copy/customerStartCopy.ts`
- `src/customer-flow/screens/CustomerFlowScreen.spec.tsx`
- `src/customer-flow/screens/CustomerFlowScreen.tsx`
- `src/customer-flow/screens/SessionStartHandoffScreen.tsx`
- `src/customer-flow/screens/CustomerStartScreen.story-1-1.spec.tsx`
- `src/customer-flow/screens/CustomerStartScreen.tsx`
- `src/session-domain/state/SessionFlowProvider.preset-catalog.spec.tsx`
- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/state/SessionFlowProvider.timing-alerts.spec.tsx`
- `tests/integration/checkInFlow.test.tsx`
- `tests/integration/postEndFlow.test.tsx`
- `tests/integration/presetSelectionFlow.test.tsx`
- `tests/integration/sessionEntryFlow.test.tsx`
- `tests/integration/sessionGalleryIsolation.test.tsx`
- `tests/integration/sessionPresetChangeFlow.test.tsx`
- `tests/integration/sessionStartFailureFlow.test.tsx`

## Change Log

- 2026-03-12: Verified the approved Story 1.1 starter baseline was already intact, completed targeted and full-project validation, and advanced the story and sprint tracking status to `review` without production code changes.
- 2026-03-13: Fixed review follow-ups by restoring the Story 1.1 starter shell boundary, separating `startJourney` from check-in submission, updating affected Story 1.1 and downstream regression tests, hardening downstream capture/path regressions surfaced during verification, correcting Story 1.1 audit notes, and rerunning targeted plus full verification successfully.
