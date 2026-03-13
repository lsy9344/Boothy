---
stepsCompleted:
  - 1
  - 2
  - 3
  - 4
  - 5
  - 6
  - 7
  - 8
inputDocuments:
  - '_bmad-output/planning-artifacts/prd-rewrite-brief-2026-03-11.md'
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/ux-design-specification.md'
  - '_bmad-output/planning-artifacts/research/technical-boothy-wrapping-efficiency-research-2026-03-08.md'
  - 'docs/refactoring/research-codex.md'
  - 'docs/research-checklist-2026-03-07-boothy-greenfield.md'
  - 'docs/business_context/context.md'
  - 'docs/release-baseline.md'
  - '_bmad-output/project-context.md'
workflowType: 'architecture'
project_name: 'Boothy'
user_name: 'Noah Lee'
date: '2026-03-11'
lastStep: 1
lastStep: 8
status: 'complete'
completedAt: '2026-03-11'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
Boothy currently defines 9 functional requirements. Architecturally, they cluster into seven capability groups. First, the booth must support a very low-friction session start based on session-name-first entry and a bounded preset choice. Second, it must normalize readiness and capture eligibility so customers only see preparation, ready, waiting, or phone-required states rather than device internals. Third, it must persist captures into the active session and show latest-photo confidence while preserving strict current-session scope. Fourth, it must support bounded in-session cleanup behavior: current-session review, deletion, and forward-only preset changes for future captures. Fifth, it must run a timing and completion model that includes adjusted end time, warning and end alerts, and explicit post-end outcome states. Sixth, it must provide an internal preset-authoring and publication workflow for authorized users. Seventh, it must expose bounded operator diagnostics, recovery, and lifecycle visibility. Architecturally, this is a booth-first, preset-driven Windows product with three distinct user surfaces: booth customer, operator, and authorized preset management.

**Non-Functional Requirements:**
Six NFRs strongly shape the architecture. The customer surface must stay copy-light and free of technical or authoring language. Branches must remain consistent in preset catalog, timing rules, and booth journey except for tightly approved local settings. The booth must acknowledge customer actions quickly and show latest-photo feedback within a defined budget on approved hardware. Session isolation is strict: cross-session asset leakage is unacceptable. Timing rules and post-end transitions must be reliable enough to preserve customer trust. Release behavior must support staged rollout, rollback, and zero forced updates during active sessions. In addition, the loaded UX specification still contributes useful non-functional constraints around touch-friendly capture layouts, separate operator density, WCAG 2.2 AA accessibility targets, and deployment-oriented responsive behavior, even though its customer-editor continuity model is no longer authoritative.

**Scale & Complexity:**
The customer journey is simpler than the previously assumed capture-to-editor product, but the architectural complexity remains high because the system must coordinate local session truth, real camera state, preset lifecycle, timing policy, operator recovery, and branch-safe deployment in one booth runtime. This is not cloud-scale complexity; it is boundary and workflow complexity centered on a local Windows desktop product.

- Primary domain: Windows desktop booth photo product with hardware integration, local session storage, and internal preset-authoring support
- Complexity level: high
- Estimated architectural components: 8

### Technical Constraints & Dependencies

- The primary runtime is an approved Windows desktop booth PC and monitor.
- The customer workflow must stay booth-first, local-first, and usable without browser navigation or manual OS file browsing.
- The authoritative product definition is now the rewrite brief plus the rewritten PRD, not the older capture-to-full-editor assumption.
- Current documents still show legacy operational patterns such as name-plus-last-four check-in and older handoff habits, so session identity and handoff rules must be normalized explicitly rather than inherited casually.
- Real camera readiness, trigger, capture persistence, and latest-photo confirmation are product-critical dependencies.
- The customer sees only 1-6 approved published presets; detailed RapidRAW-derived controls are restricted to authorized internal use.
- Timing policy is a core product dependency: adjusted end time, 5-minute warning, exact-end alert, export-waiting/completion states, and operator extensions all affect flow truth.
- Branch variance must stay tightly controlled; active branches should differ only through approved local settings such as contact information or bounded operational toggles.
- Staged rollout, rollback, and zero forced updates during active sessions are hard desktop-operational constraints.
- Remote operator intervention remains part of the operating model when bounded recovery cannot restore a safe booth state.

### Cross-Cutting Concerns Identified

- Session identity, naming, and downstream handoff consistency
- Session-scoped asset persistence, deletion, and privacy isolation
- Camera-state normalization into customer-safe and operator-diagnostic views
- Preset lifecycle from internal authoring to approval, publication, activation, and forward-only in-session changes
- Timing-policy calculation, warning/alert behavior, and post-end state transitions
- Completion, export-waiting, and handoff guidance without reintroducing customer-side detailed editing
- Operational logging, exception classification, and bounded recovery
- Branch consistency, rollout safety, and rollback compatibility

## Starter Template Evaluation

### Primary Technology Domain

Desktop application with a React SPA frontend and a native Tauri/Rust host boundary.

This fits the current product definition directly. Boothy is a Windows booth application that must keep the customer flow, operator recovery flow, and internal preset-authoring capability inside one packaged local-first product. It needs explicit control over camera boundary handling, session-scoped filesystem truth, timed booth states, and bounded internal capability exposure.

### Starter Options Considered

1. **Official `create-tauri-app` with `React + TypeScript`**
   - Officially maintained Tauri entry path.
   - Gives a valid Tauri + React + TypeScript baseline quickly.
   - Good option if we optimize for fast official scaffolding over structural control.

2. **Official `Vite react-ts` + manual `Tauri CLI` initialization**
   - Also an official Tauri-supported path.
   - Best fit for Boothy because it keeps the frontend scaffold minimal while preserving a clear Tauri host boundary.
   - Makes it easier to impose the project’s domain-first structure, contract-first adapters, session-truth rules, and selective RapidRAW Host/UI reuse without first undoing starter opinions.

3. **Electron Forge with `vite-typescript`**
   - Viable and maintained as an Electron path.
   - Weaker fit because the current project context and research are already centered on Tauri capabilities, sidecar packaging, and Rust host boundaries.
   - Electron Forge’s Vite path is still marked experimental, and pnpm requires additional linker configuration.

4. **Next.js static export + Tauri**
   - Technically possible if reduced to static export.
   - Poorer fit because Tauri’s frontend guidance favors SPA/Vite setups for most projects, and server-based SSR is not the intended model.
   - Adds framework weight without helping the booth-state, camera-boundary, or session-folder architecture.

### Selected Starter: Official `Vite react-ts` + manual `Tauri CLI` initialization

**Rationale for Selection:**
This is the best fit for the redesigned Boothy architecture. It stays on an official Tauri path, matches Tauri’s current SPA guidance, and gives the smallest scaffold around the real product boundaries. Boothy needs one packaged runtime with a clear host boundary, explicit sidecar/camera integration, and strict session/data rules. A minimal Vite frontend plus manual Tauri initialization gives us that without inheriting unnecessary starter structure. It also leaves room to keep internal preset-authoring capability in the same package while preventing the customer booth surface from turning back into a full editor product.

**Initialization Command:**

```bash
pnpm create vite boothy --template react-ts --no-interactive
cd boothy
pnpm add -D @tauri-apps/cli@latest
pnpm exec tauri init
```

Recommended Tauri init values:

```bash
App name: Boothy
Window title: Boothy
Web assets location: ../dist
Dev server URL: http://localhost:5173
Frontend dev command: pnpm dev
Frontend build command: pnpm build
```

**Architectural Decisions Provided by Starter:**

**Language & Runtime:**
React + TypeScript frontend with a Rust-based Tauri host boundary.

**Styling Solution:**
No heavy styling decision is forced by the starter. This is useful because Boothy still needs product-specific customer, operator, and internal-authoring surfaces rather than a generic starter design system.

**Build Tooling:**
Vite handles the frontend development/build loop, and Tauri connects the packaged desktop runtime to that frontend through `devUrl` and built static assets.

**Testing Framework:**
No strong testing stack is imposed. That is acceptable here because Boothy needs a custom test strategy centered on contracts, session manifests, host adapters, sidecar protocol behavior, and booth workflow seams.

**Code Organization:**
Minimal scaffold only. This supports the project’s domain-first architecture instead of pushing a starter-defined app structure that would need to be dismantled later.

**Development Experience:**
Fast frontend iteration, straightforward desktop packaging, explicit native boundary, and low starter baggage. This is especially useful for proving `capture shell -> normalized host state -> session-folder truth -> operator/internal surfaces` without structural noise.

**Note:** Project initialization using this command should be the first implementation story.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Boothy remains one packaged Tauri application, but it is split into three capability-gated surfaces: `booth customer shell`, `operator console`, and `internal preset-authoring`.
- The durable source of truth for active booth work is a session-scoped filesystem root, not route state, UI memory, or SQLite.
- RapidRAW-derived detailed editing capability is retained only as an internal preset-authoring domain and shared render/preset core, not as a customer-facing editing workspace.
- The Rust host is the single normalization point for camera/helper truth, timing truth, and post-end workflow truth before those states are translated to UI.
- Camera integration is isolated behind a bundled helper/sidecar boundary with versioned messages and filesystem handoff; camera SDK truth does not leak into React.
- Session timing rules, warning alerts, exact-end behavior, and post-end state transitions are host-owned workflow rules.
- Release behavior must preserve staged rollout, rollback, and zero forced update during active customer sessions.

**Important Decisions (Shape Architecture):**
- SQLite stores lifecycle, timing, intervention, publication, and rollout audit data, but it does not own photo or session artifact truth.
- Presets are stored as versioned approved bundles and published into a bounded booth catalog; active sessions reference preset versions explicitly.
- Tauri Store or equivalent local config keeps only minimal branch-local settings and runtime profile flags.
- Boundary validation uses `Zod 4` in TypeScript and revalidation in Rust before file mutation, helper control, or preset publication.
- React Router `7.x` is limited to top-level surfaces such as `/booth`, `/operator`, `/authoring`, and `/settings`.
- The React frontend remains domain-first, and all host interaction stays behind typed adapter/service modules.

**Deferred Decisions (Post-MVP):**
- Centralized preset distribution service
- Stronger authoring authentication such as SSO or hardware-backed identity
- Remote log export and centralized observability
- Promotion from sidecar stdio to named pipes or a longer-lived local service if hardware evidence requires it

### Data Architecture

- **Primary session truth:** Every active booth session owns one local session root that contains manifest metadata, captured originals, derived booth-facing images, handoff-ready outputs, and diagnostics snapshots.
- **Suggested session structure:** `session.json`, `captures/originals/`, `captures/processed/`, `handoff/`, and optional `diagnostics/` under one session boundary.
- **Capture correlation:** Each capture is tracked by stable identifiers such as `sessionId`, `captureId`, `requestId`, active preset version, and file references.
- **Deletion model:** Approved customer deletion removes the current session’s correlated original and derived artifacts and records the deletion in manifest and audit data immediately.
- **Preset data model:** Presets are published as immutable versioned bundles with metadata, preview assets, and render parameters. Booth sessions only consume approved published bundles.
- **Preset/session separation:** Preset-authoring never edits active booth session data directly. It produces future preset versions that later sessions may reference.
- **Operational store:** SQLite stores lifecycle events, timing transitions, operator interventions, preset publication audits, and rollout history.
- **Configuration store:** Minimal local config stores branch phone number, approved operational toggles, and runtime profile such as `booth` or `authoring-enabled`.
- **Validation strategy:** Shared boundary schemas are validated with `Zod 4` in TypeScript and revalidated in Rust.
- **Migration strategy:** `session.json` and preset bundles carry explicit schema versions; SQLite uses forward-only migrations; no migration may mutate active session artifacts in place.
- **Caching strategy:** In-memory caches may accelerate active screens, but no cache is allowed to outrank session folders or approved preset bundles.

### Authentication & Security

- **Booth customer authentication:** None. The booth customer flow is intentionally login-free.
- **Authorization model:** Access is enforced through Tauri capabilities, runtime profile gating, window/surface separation, and host command boundaries.
- **Surface restriction rule:** Booth customers cannot access diagnostics, recovery controls, helper process management, or preset-authoring capabilities.
- **Authoring restriction rule:** Internal preset-authoring is enabled only for approved authoring profiles or installations, not as a normal booth runtime path.
- **Data minimization:** Persist only the minimum session-identifying data approved by the PRD and operating model.
- **PII protection:** Logs, diagnostics, and handoff surfaces must not expose cross-session references or unnecessary customer identifiers.
- **Host authority:** Only the Rust host may spawn or control the helper, mutate session files, publish preset bundles, or apply rollout-sensitive actions.
- **Security posture:** MVP security is based on local least privilege, capability isolation, bounded local profiles, and strict session separation rather than network-style account auth for the customer path.

### API & Communication Patterns

- **Frontend to host:** Tauri commands are the request-response path for session start, preset selection, capture, delete, timing updates, completion transitions, diagnostics queries, operator actions, and preset publication.
- **Host to frontend streaming:** Tauri channels carry ordered state changes for readiness, capture progress, latest-photo availability, timing transitions, completion state, and operator diagnostics.
- **Host to helper:** The camera/helper boundary uses bundled sidecar stdio with versioned JSON-line messages.
- **Helper contract shape:** The first contract should cover session configuration, capture request, health/status, restart/recovery, and correlation of file arrival back to the host.
- **Image transfer rule:** Raw image bytes and derived booth files move by filesystem handoff, not by large JSON IPC payloads.
- **Preset/render core rule:** Shared render logic derived from RapidRAW stays behind host/pipeline boundaries and is exposed as booth-safe outputs or internal authoring tools, not as direct customer editing APIs.
- **Error handling standard:** All host-facing failures use one typed envelope with machine-readable code, severity, retryability, customer-safe state, and operator-facing next action.
- **State normalization:** Camera/helper truth, timing truth, and completion truth are normalized in the host once, then translated into booth copy or operator diagnostics separately.

### Frontend Architecture

- **Top-level app model:** One React application with top-level surfaces such as `/booth`, `/operator`, `/authoring`, and `/settings`.
- **Routing strategy:** React Router `7.x` is used only for surface entry and separation. Workflow truth remains state-driven.
- **State management:** Use explicit reducers and React Context by domain: `session-domain`, `preset-catalog`, `capture-adapter`, `timing-policy`, `completion-handoff`, `operator-console`, and `preset-authoring`.
- **Component architecture:** Keep a domain-first structure so booth customer flow, operator flow, and authoring flow do not blur together.
- **Booth shell rule:** The customer UI stays low-choice, touch-friendly, and confidence-oriented. It never expands into a general editor workspace.
- **Authoring rule:** Internal preset-authoring may reuse selected editor primitives, render controls, and preview panels from RapidRAW-derived assets, but only inside the authoring surface.
- **Performance strategy:** Keep the booth shell light, preload bounded preset previews, lazy-load operator and authoring surfaces, and use React `19.x` async patterns for non-blocking transitions.
- **Boundary rule:** React components do not call Tauri directly. Typed adapters and services own all `invoke`, channel subscriptions, and host orchestration.

### Infrastructure & Deployment

- **Runtime hosting:** Approved Windows booth PCs run the booth customer and operator surfaces; approved internal machines may run the authoring-enabled profile.
- **Package strategy:** One package family and one codebase, with capability/profile differences controlling which surfaces are enabled in each deployment context.
- **Build and release:** GitHub Actions plus `tauri-action` handle build, packaging, and signing-ready Windows release flow.
- **Release safety:** Branch rollout is staged, rollback-capable, and must preserve last-approved installers plus active-session compatibility.
- **Update policy:** No forced update may interrupt an active booth session.
- **Environment configuration:** Branch-local configuration stays minimal, explicit, and auditable.
- **Monitoring and logging:** Structured local logs plus SQLite audit tables provide the MVP observability base.
- **Scaling strategy:** The system scales by booth instance, preset publication discipline, and rollout control rather than centralized backend throughput.

### Decision Impact Analysis

**Implementation Sequence:**
1. Freeze the shared contracts: session manifest, preset bundle schema, error envelope, helper protocol, and runtime profile/capability model.
2. Define the session folder structure and preset publication structure.
3. Build the Rust host state model for readiness, capture, timing, completion, and diagnostics.
4. Implement the booth shell against mocked host and mocked helper behavior.
5. Implement operator diagnostics and bounded recovery against the same normalized host truth.
6. Implement internal preset-authoring on top of the shared preset/render core without exposing those controls to booth routes.
7. Integrate the real camera/helper boundary and prove `capture request -> file arrival -> latest-photo confirmation -> handoff state`.
8. Add rollout, rollback, and signing-ready release guardrails.

**Cross-Component Dependencies:**
- Session manifest and capture correlation rules affect booth review, deletion, handoff, diagnostics, and privacy guarantees.
- Preset bundle format affects authoring, booth preset selection, preview rendering, and cross-branch consistency.
- Runtime profile and capability boundaries affect security, packaging, and which UI surfaces exist in each deployment.
- Error envelope and normalized state model affect booth guidance, operator recovery, and helper integration.
- Release safety depends on config discipline, schema compatibility, and preserving active-session behavior across versions.

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:**
9 areas where AI agents could make different choices and silently break compatibility across React, Tauri/Rust, sidecar, and session storage boundaries.

### Naming Patterns

**Database Naming Conventions:**
- SQLite tables use `snake_case` plural names such as `session_events`, `preset_publications`, `operator_interventions`.
- Columns use `snake_case` such as `session_id`, `occurred_at`, `preset_version`.
- Indexes use `idx_<table>_<columns>` such as `idx_session_events_session_id_occurred_at`.

**API Naming Conventions:**
- Rust Tauri command identifiers use `snake_case` such as `start_session`, `select_preset`, `request_capture`.
- TypeScript never hardcodes raw command strings outside the host adapter layer; exported wrapper functions use `camelCase`.
- Channel and event names use `dot.case` namespaces such as `session.stateChanged`, `capture.progress`, `timing.warning`, `handoff.completed`.
- Route paths use `kebab-case` and are reserved for top-level surfaces only, such as `/booth`, `/operator`, `/authoring`, `/settings`.

**Code Naming Conventions:**
- React component names and component filenames use `PascalCase`, such as `BoothShell.tsx`, `OperatorConsole.tsx`, `PresetAuthoringShell.tsx`.
- Hooks use `camelCase` with `use` prefix, such as `useSessionState.ts`, `usePresetCatalog.ts`.
- TypeScript services/adapters use `camelCase` filenames, such as `hostCommands.ts`, `presetPublishService.ts`.
- Rust modules use `snake_case.rs`, such as `session_manifest.rs`, `timing_policy.rs`.
- Domain directories use `kebab-case`, such as `booth-shell`, `operator-console`, `preset-authoring`.

### Structure Patterns

**Project Organization:**
- Frontend code is organized by domain first, not by technical type first.
- `shared-ui` holds presentation-only primitives; domain rules and translation logic stay in the owning domain.
- Tauri command handlers live under `src-tauri/src/commands/`, while domain logic lives in dedicated Rust modules outside the command entrypoint layer.
- Cross-language contract definitions must have one authoritative source per contract family and must not be duplicated casually across frontend, host, and helper.

**File Structure Patterns:**
- Co-locate unit tests close to domain logic where possible.
- Keep cross-boundary contract tests under `tests/contract/`.
- Keep e2e coverage under `tests/e2e/`.
- Keep SQLite migrations under `src-tauri/migrations/`.
- Keep helper protocol examples and fixtures under `sidecar/protocol/`.

### Format Patterns

**API Response Formats:**
- Host command responses use typed DTOs or typed error envelopes, not unstructured `any` or ad hoc object returns.
- Error envelopes follow one standard shape with fields such as `code`, `severity`, `retryable`, `customerState`, `operatorAction`, and `details` only where explicitly allowed.
- Success responses return direct typed payloads unless a command needs a standardized wrapper for versioning or state metadata.

**Data Exchange Formats:**
- TypeScript-facing JSON fields use `camelCase`.
- Rust internal storage and SQLite schemas use `snake_case`.
- Session manifest files use one explicitly versioned schema and must not drift per feature.
- Dates and timestamps use ISO 8601 / RFC3339 strings at boundaries.
- Booleans remain `true/false`; no numeric boolean encoding.
- Nullability must be explicit in schemas; absence and null are not interchangeable.

### Communication Patterns

**Event System Patterns:**
- Use Tauri `channels` for ordered workflow/status streams and reserve generic events for coarse notifications only.
- Event names use `dot.case` and remain domain-qualified.
- Event payloads always include `sessionId`, `type`, and `schemaVersion` where applicable.
- Version helper-facing protocol messages explicitly when they cross the sidecar boundary.

**State Management Patterns:**
- React state is reducer-driven and domain-scoped.
- Actions use `domain/actionVerb` style or equivalent typed constants, such as `session/startRequested`, `timing/warningTriggered`.
- Selectors own UI-facing translation logic; components do not reinterpret raw host state inline.
- Customer-facing and operator-facing state projections must derive from the same normalized host truth, not from separate ad hoc transforms.

### Process Patterns

**Error Handling Patterns:**
- Distinguish between customer-safe messaging, operator-facing diagnosis, and raw internal logs.
- Never surface raw helper, filesystem, or SDK diagnostics directly on customer screens.
- Retry logic must be explicit at the adapter or host orchestration layer, not hidden in UI components.
- Errors that affect session integrity must be logged as lifecycle or intervention records with correlation IDs.

**Loading State Patterns:**
- Use explicit loading states named by workflow meaning, not generic booleans alone: `preparing`, `ready`, `capturePending`, `exportWaiting`, `handoffCompleted`.
- Loading states that affect customer actionability must map to approved customer-facing copy.
- Long-running operations must emit progress or status updates through the normalized host communication path.
- Loading completion is determined by workflow truth, not by route entry or component mount alone.

### Enforcement Guidelines

**All AI Agents MUST:**
- Preserve the session folder as the durable source of truth.
- Keep React UI code out of direct Tauri invocation and helper orchestration.
- Reuse the standardized schema, error, and event naming rules exactly.
- Avoid introducing parallel contract definitions across language boundaries.
- Keep customer-visible state translation centralized and reviewable.

**Pattern Enforcement:**
- Contract-sensitive changes must be reviewed against shared schemas, manifest rules, and event naming rules.
- New domains or files should be placed according to the documented directory grammar before code is merged.
- Pattern violations should be corrected at the boundary layer first, not patched locally in UI components.

### Pattern Examples

**Good Examples:**
- `start_session` Rust command wrapped by `startSession()` in TypeScript
- `session.stateChanged` channel event carrying a typed payload with normalized session status
- `session.json` manifest with explicit `schemaVersion`
- `booth-shell/selectors/customerStatusCopy.ts` owning customer-safe text translation
- `tests/contract/errorEnvelope.test.ts` protecting boundary compatibility

**Anti-Patterns:**
- React components calling `invoke('request_capture')` directly
- Customer UI deciding camera readiness from raw helper error text
- Duplicate `SessionManifest` shapes defined separately in frontend, Rust, and helper code without one source of truth
- Using route changes as the authoritative signal that the product moved from capture to handoff
- Storing cross-session image indexes in a cache that can drift from filesystem truth

## Project Structure & Boundaries

### Complete Project Directory Structure

```text
boothy/
├── README.md
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
├── eslint.config.js
├── prettier.config.cjs
├── index.html
├── .env.example
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release-windows.yml
├── docs/
│   ├── contracts/
│   │   ├── session-manifest.md
│   │   ├── preset-bundle.md
│   │   ├── error-envelope.md
│   │   └── sidecar-protocol.md
│   ├── architecture/
│   └── runbooks/
├── src/
│   ├── main.tsx
│   ├── app/
│   │   ├── App.tsx
│   │   ├── routes.tsx
│   │   ├── providers/
│   │   └── boot/
│   ├── shared-ui/
│   │   ├── components/
│   │   ├── layout/
│   │   └── tokens/
│   ├── shared-contracts/
│   │   ├── dto/
│   │   ├── schemas/
│   │   ├── events/
│   │   └── errors/
│   ├── booth-shell/
│   │   ├── screens/
│   │   │   ├── SessionStartScreen.tsx
│   │   │   ├── PresetSelectScreen.tsx
│   │   │   ├── ReadinessScreen.tsx
│   │   │   ├── CaptureScreen.tsx
│   │   │   ├── ReviewScreen.tsx
│   │   │   ├── TimingWarningScreen.tsx
│   │   │   └── HandoffScreen.tsx
│   │   ├── components/
│   │   ├── selectors/
│   │   ├── copy/
│   │   └── tests/
│   ├── operator-console/
│   │   ├── screens/
│   │   │   ├── OperatorSummaryScreen.tsx
│   │   │   ├── RecoveryActionsScreen.tsx
│   │   │   ├── DiagnosticsScreen.tsx
│   │   │   └── SessionRepairScreen.tsx
│   │   ├── components/
│   │   ├── selectors/
│   │   └── tests/
│   ├── preset-authoring/
│   │   ├── screens/
│   │   │   ├── PresetLibraryScreen.tsx
│   │   │   ├── PresetEditorScreen.tsx
│   │   │   ├── PresetPreviewScreen.tsx
│   │   │   └── PublishWorkflowScreen.tsx
│   │   ├── components/
│   │   ├── state/
│   │   ├── services/
│   │   └── tests/
│   ├── session-domain/
│   │   ├── state/
│   │   ├── services/
│   │   ├── selectors/
│   │   └── tests/
│   ├── capture-adapter/
│   │   ├── host/
│   │   ├── state/
│   │   ├── services/
│   │   └── tests/
│   ├── timing-policy/
│   │   ├── state/
│   │   ├── services/
│   │   └── tests/
│   ├── completion-handoff/
│   │   ├── state/
│   │   ├── services/
│   │   └── tests/
│   ├── preset-catalog/
│   │   ├── state/
│   │   ├── services/
│   │   └── tests/
│   ├── branch-config/
│   │   ├── services/
│   │   ├── state/
│   │   └── tests/
│   └── diagnostics-log/
│       ├── services/
│       ├── selectors/
│       └── tests/
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   ├── booth-window.json
│   │   ├── operator-window.json
│   │   └── authoring-window.json
│   ├── migrations/
│   │   ├── 0001_init.sql
│   │   ├── 0002_operator_interventions.sql
│   │   ├── 0003_preset_publications.sql
│   │   └── 0004_timing_transitions.sql
│   ├── tests/
│   │   ├── session_manifest.rs
│   │   ├── preset_bundle.rs
│   │   ├── error_envelope.rs
│   │   └── sqlite_logs.rs
│   └── src/
│       ├── main.rs
│       ├── app_state.rs
│       ├── commands/
│       │   ├── session_commands.rs
│       │   ├── capture_commands.rs
│       │   ├── preset_commands.rs
│       │   ├── timing_commands.rs
│       │   ├── handoff_commands.rs
│       │   ├── operator_commands.rs
│       │   └── branch_config_commands.rs
│       ├── contracts/
│       │   ├── dto.rs
│       │   ├── error_envelope.rs
│       │   ├── event_payloads.rs
│       │   └── schema_version.rs
│       ├── session/
│       │   ├── session_manifest.rs
│       │   ├── session_paths.rs
│       │   └── session_repository.rs
│       ├── capture/
│       │   ├── camera_host.rs
│       │   ├── ingest_pipeline.rs
│       │   ├── sidecar_client.rs
│       │   └── normalized_state.rs
│       ├── preset/
│       │   ├── preset_bundle.rs
│       │   ├── preset_catalog.rs
│       │   ├── authoring_pipeline.rs
│       │   └── preview_service.rs
│       ├── timing/
│       │   ├── timing_policy.rs
│       │   ├── alerts.rs
│       │   └── scheduler.rs
│       ├── handoff/
│       │   ├── completion_state.rs
│       │   ├── handoff_service.rs
│       │   └── output_repository.rs
│       ├── diagnostics/
│       │   ├── lifecycle_log.rs
│       │   ├── intervention_log.rs
│       │   └── fault_classifier.rs
│       ├── branch_config/
│       │   ├── config_store.rs
│       │   ├── rollout_guard.rs
│       │   └── updater_policy.rs
│       ├── db/
│       │   ├── sqlite.rs
│       │   ├── migrations.rs
│       │   └── repositories/
│       └── support/
│           ├── clock.rs
│           ├── fs.rs
│           └── tracing.rs
├── sidecar/
│   ├── README.md
│   ├── protocol/
│   │   ├── messages.schema.json
│   │   └── examples/
│   ├── fixtures/
│   └── canon-helper/
│       ├── src/
│       ├── tests/
│       └── build/
├── tests/
│   ├── contract/
│   │   ├── sessionManifest.test.ts
│   │   ├── presetBundle.test.ts
│   │   └── errorEnvelope.test.ts
│   ├── integration/
│   │   ├── captureToReview.test.ts
│   │   ├── timingTransitions.test.ts
│   │   └── handoffCompletion.test.ts
│   ├── e2e/
│   │   ├── booth-flow.spec.ts
│   │   ├── operator-recovery.spec.ts
│   │   └── authoring-flow.spec.ts
│   └── fixtures/
│       ├── sessions/
│       └── sidecar/
└── storage/
    ├── fixtures/
    └── sample-sessions/
```

### Architectural Boundaries

**API Boundaries:**
- React UI reaches native behavior only through typed adapters/services under `src/*/services` or `src/*/host`.
- Tauri commands in `src-tauri/src/commands/` are the only frontend-to-host entry points.
- Sidecar communication is isolated to `src-tauri/src/capture/sidecar_client.rs` and `sidecar/canon-helper/`.

**Component Boundaries:**
- `booth-shell` owns booth customer flow only.
- `operator-console` owns diagnostics and bounded recovery actions.
- `preset-authoring` owns internal preset creation and publication workflows.
- `session-domain` owns lifecycle truth and shared selectors.

**Service Boundaries:**
- `capture-adapter` owns host-facing capture orchestration.
- `preset-catalog` owns approved preset list consumption on booth surfaces.
- `timing-policy` owns timing calculations, warnings, and end-time transitions.
- `completion-handoff` owns post-end state transitions and guidance.
- `branch-config` owns branch-local config and rollout safety rules.
- `diagnostics-log` owns queryable operational history.

**Data Boundaries:**
- Session folders own image and session truth.
- SQLite owns logs, audits, timing transitions, and publication history.
- Tauri Store owns minimal branch-local config only.
- Sidecar owns live camera truth while running, but not durable product truth.

### Requirements to Structure Mapping

**Feature/FR Mapping:**
- FR-001/FR-002 (session start + preset selection)
  - `src/booth-shell/`
  - `src/preset-catalog/`
  - `src-tauri/src/commands/session_commands.rs`
  - `src-tauri/src/preset/`
- FR-003/FR-004 (readiness + latest-photo confidence)
  - `src/booth-shell/`
  - `src/capture-adapter/`
  - `src-tauri/src/capture/`
- FR-005 (current-session review/delete + future preset change)
  - `src/booth-shell/`
  - `src/session-domain/`
  - `src-tauri/src/session/`
- FR-006 (timing rules, warning, end)
  - `src/timing-policy/`
  - `src-tauri/src/timing/`
- FR-007 (export-waiting / completion / handoff)
  - `src/completion-handoff/`
  - `src-tauri/src/handoff/`
- FR-008 (internal preset authoring / publication)
  - `src/preset-authoring/`
  - `src-tauri/src/preset/`
- FR-009 (operator diagnostics/recovery)
  - `src/operator-console/`
  - `src/diagnostics-log/`
  - `src-tauri/src/diagnostics/`

**Cross-Cutting Concerns:**
- Shared contracts
  - `src/shared-contracts/`
  - `src-tauri/src/contracts/`
  - `tests/contract/`
- Session lifecycle truth
  - `src/session-domain/`
  - `src-tauri/src/session/`
- Rollout/rollback safety
  - `src/branch-config/`
  - `src-tauri/src/branch_config/`
  - `.github/workflows/release-windows.yml`

### Integration Points

**Internal Communication:**
- UI domains call typed adapters/services.
- Adapters call Tauri commands/channels.
- Rust commands delegate into domain modules.
- Rust capture domain talks to helper through the sidecar protocol boundary.

**External Integrations:**
- Canon/camera helper integration lives under `sidecar/canon-helper/`.
- Future reservation/policy sync enters through `branch-config/` or a separate integration module, not through booth flow domains.
- Remote support tools remain external to the product runtime.

**Data Flow:**
- Session start creates session identity and session root.
- Capture writes originals into the session folder and updates session manifest.
- Preset selection binds a preset version to the session.
- Review and deletion operate only on current session assets.
- Timing policy emits warning/end alerts and shifts the workflow state.
- Post-end states transition to export-waiting, completed, or phone-required.
- Diagnostics and operator actions are recorded into SQLite.

### File Organization Patterns

**Configuration Files:**
- Root: frontend/tooling config
- `src-tauri/`: native packaging, capabilities, migrations
- `docs/contracts/`: durable contract documentation

**Source Organization:**
- Frontend is domain-first.
- Rust host is boundary-first and domain-backed.
- Sidecar is isolated as its own implementation boundary.

**Test Organization:**
- Unit tests close to domain code
- Contract tests at the top level
- E2E tests by product flow
- Rust host tests under `src-tauri/tests/`

**Asset Organization:**
- Session/sample assets under `storage/` and `tests/fixtures/`
- Protocol fixtures under `sidecar/protocol/examples/`

### Development Workflow Integration

**Development Server Structure:**
- `pnpm dev` for Vite frontend loop
- `pnpm tauri dev` for integrated desktop loop
- Helper fixtures or mock sidecar selected through environment/config

**Build Process Structure:**
- Frontend build to `dist/`
- Tauri packaging from `src-tauri/`
- Sidecar bundled during desktop packaging

**Deployment Structure:**
- GitHub Actions builds signing-ready Windows installers
- Branch rollout and rollback artifacts remain compatible with the local config and session data model

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
All technology choices are compatible. `React + Vite + Tauri + Rust` aligns with a local Windows booth product. The sidecar boundary, session-folder truth, and host-normalized state model reinforce each other. The internal preset-authoring capability is explicitly separated from booth customer flow, so it no longer conflicts with the booth-first PRD.

**Pattern Consistency:**
Naming, schema ownership, and event conventions are consistent with the chosen stack and the project-context rules. The “adapter-only UI” rule, error envelope shape, and channel/event rules prevent common boundary drift across React, Tauri, and the sidecar helper.

**Structure Alignment:**
The project tree supports the architecture: booth shell, operator console, and preset-authoring are first-class domains; host modules mirror those domains; sidecar is isolated; and session/preset contracts have dedicated homes. This structure implements the boundaries rather than fighting them.

### Requirements Coverage Validation ✅

**Epic/Feature Coverage:**
No epics were provided, but all FR clusters in the PRD map to concrete domains and host modules.

**Functional Requirements Coverage:**
- FR-001/002 covered by booth shell + session/preset modules
- FR-003/004 covered by capture-adapter + host capture pipeline + session manifest
- FR-005 covered by booth review/delete flows + session-domain
- FR-006 covered by timing-policy domain + host timing modules
- FR-007 covered by completion-handoff domain + host handoff modules
- FR-008 covered by preset-authoring + preset publication modules
- FR-009 covered by operator console + diagnostics/log modules

**Non-Functional Requirements Coverage:**
- Customer copy density and simplicity is addressed via centralized copy selectors and restricted customer-facing surfaces.
- Cross-branch consistency is enforced via preset publication and branch-config constraints.
- Responsiveness and latest-photo feedback are prioritized by local-first session truth and host-driven state.
- Session isolation is enforced by session-scoped storage and manifest boundaries.
- Timing and completion reliability is supported by host-owned timing policy and explicit post-end states.
- Safe rollout and no forced update are addressed by release and branch-config decisions.

### Implementation Readiness Validation ✅

**Decision Completeness:**
Critical decisions are documented: session truth, camera boundary, timing policy, surface separation, and internal authoring scope. Deferred decisions are explicitly listed.

**Structure Completeness:**
The structure is concrete and maps requirements to specific directories and host modules. Sidecar boundary and contract documentation are defined.

**Pattern Completeness:**
Naming, schema ownership, event conventions, error handling, and state translation patterns are defined with examples and anti-patterns.

### Gap Analysis Results

**Critical Gaps:** None identified.

**Important Gaps:**
- Lock the exact `session.json` manifest schema and preset bundle schema before implementation begins.
- Provide a concrete helper protocol example set for success, retryable failure, and fatal failure paths.
- Define the exact authoring publication workflow payloads and approval state transitions.

**Nice-to-Have Gaps:**
- Detailed release runbook content per branch
- Fixture naming conventions for contract and E2E tests
- A minimal reference dataset for preset preview assets and timing-policy test cases

### Validation Issues Addressed

No blocking contradictions were found. The main remaining work is specification tightening for contract surfaces.

### Architecture Completeness Checklist

**✅ Requirements Analysis**
- [x] Project context thoroughly analyzed
- [x] Scale and complexity assessed
- [x] Technical constraints identified
- [x] Cross-cutting concerns mapped

**✅ Architectural Decisions**
- [x] Critical decisions documented with versions
- [x] Technology stack fully specified
- [x] Integration patterns defined
- [x] Performance considerations addressed

**✅ Implementation Patterns**
- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Communication patterns specified
- [x] Process patterns documented

**✅ Project Structure**
- [x] Complete directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements to structure mapping complete

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** High

**Key Strengths:**
- Booth-first product definition is now reflected consistently in architecture, patterns, and structure.
- Customer/Operator/Authoring surfaces are capability-gated and correctly separated.
- Session-folder truth and host-normalized state prevent boundary drift.
- Camera boundary is explicitly isolated behind a sidecar contract.
- Rollout/rollback and no forced update are built into the design.

**Areas for Future Enhancement:**
- Formalize preset bundle and session manifest schemas
- Enrich sidecar protocol examples
- Expand release runbooks and fixture taxonomy

### Implementation Handoff

**AI Agent Guidelines:**
- Follow all architectural decisions exactly as documented
- Use implementation patterns consistently across all components
- Respect project structure and boundaries
- Refer to this document for all architectural questions

**First Implementation Priority:**
Freeze the contract surfaces: `session manifest`, `preset bundle schema`, `error envelope`, `sidecar protocol`, and `runtime profile/capability model`.
