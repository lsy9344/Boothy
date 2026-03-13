---
stepsCompleted:
  - step-01-validate-prerequisites
  - step-02-design-epics
  - step-03-create-stories
inputDocuments:
  - 'C:\Code\Project\Boothy\_bmad-output\planning-artifacts\prd.md'
  - 'C:\Code\Project\Boothy\_bmad-output\planning-artifacts\architecture.md'
  - 'C:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md'
  - 'C:\Code\Project\Boothy\_bmad-output\planning-artifacts\sprint-change-proposal-2026-03-11.md'
  - 'C:\Code\Project\Boothy\_bmad-output\planning-artifacts\sprint-change-proposal-2026-03-12.md'
  - 'C:\Code\Project\Boothy\_bmad-output\planning-artifacts\implementation-readiness-report-2026-03-12.md'
  - 'C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\sprint-status.yaml'
---

# Boothy - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Boothy, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

Planning alignment notes:
- Story numbering is intentionally preserved to avoid destabilizing existing implementation artifacts and the approved salvage baseline in `sprint-status.yaml`. Corrections below therefore use targeted reclassification and rewording instead of broad renumbering.
- Approved foundation/platform baseline work remains explicit for execution even where it is not reintroduced as renumbered customer-flow stories: starter/runtime bootstrap, signing-ready Windows build and release verification, contract surfaces (`session manifest`, `preset bundle schema`, `error envelope`, `sidecar protocol`, `runtime profile/capability model`), and lifecycle/intervention logging baseline.
- Branch consistency, rollout governance, and release-audit responsibilities stay in Epic 7 so Epic 2 and Epic 3 remain customer-value focused.

## Requirements Inventory

### Functional Requirements

FR1: Users can start a booth session by entering a non-empty session name as the only required booth-start input.
FR2: Users can choose one approved preset from a bounded catalog (1-6) before capture begins, without detailed image-adjustment controls.
FR3: Users see readiness guidance and can capture only in approved valid states (preparing, ready, waiting, phone-required).
FR4: The system persists captured photos to the active session and shows the latest current-session result as confidence feedback, with the active preset name visible.
FR5: Users can review only current-session photos, delete approved current-session photos, and change the active preset for future captures only; no detailed editing controls are exposed.
FR6: The system manages session timing with coupon-adjusted end time, a 5-minute warning, an exact-end alert, and state-appropriate guidance.
FR7: The system guides post-end outcomes through explicit states (export-waiting, completed, or handoff) with clear next actions and safe wait/call guidance if unresolved.
FR8: Authorized internal users can create, tune, approve, and publish presets with detailed controls; booth customers see only approved published presets, with controlled rollout/rollback.
FR9: The system detects blocked states and provides bounded operator diagnostics and recovery actions, while recording lifecycle and intervention events.

### NonFunctional Requirements

NFR1: Customer state screens must stay within the copy budget (one primary instruction sentence, one supporting sentence, one primary action label) and expose zero internal diagnostic or preset-authoring terms, per release copy audit.
NFR2: 100% of active branches use the same approved preset catalog and customer-visible timing rules, with variance limited to approved local settings, per branch rollout audit.
NFR3: Latest-photo confirmation appears within 5 seconds for 95th-percentile successful captures and primary customer actions are acknowledged within 1 second on approved Windows hardware.
NFR4: Zero cross-session photo leaks across capture, review, deletion, and completion flows; privacy validation covers active and reopened sessions.
NFR5: 5-minute warning and exact-end alert fire within +/- 5 seconds for 99% of qualifying sessions; >=90% of sessions enter a post-end state within 10 seconds and resolve within 2 minutes.
NFR6: Staged rollout and rollback are supported with zero forced updates during active sessions; branches can return to the last approved build while preserving approved local settings.

### Additional Requirements

- Starter template: `pnpm create vite boothy --template react-ts` plus manual `tauri` CLI init; this should be the first implementation story.
- Runtime is a Windows desktop app with React + TypeScript frontend and a Rust Tauri host boundary.
- Each active session owns a local session root with `session.json`, `captures/originals/`, `captures/processed/`, `handoff/`, and optional `diagnostics/`.
- Camera integration uses a bundled sidecar helper with versioned JSON-line messages; image bytes move by filesystem handoff, not IPC payloads.
- The Rust host normalizes camera readiness, timing, and completion truth; UI uses typed adapters/services (no direct Tauri invoke in UI components).
- Surfaces are separated: booth customer, operator console, and internal preset-authoring; no customer editor surface exists.
- Timing policy, warning alerts, exact-end behavior, and post-end state transitions are host-owned workflow rules.
- Rollout/rollback and zero forced updates are required; branch-local config is limited to approved contact info and operational toggles.
- Boundary schemas are validated with Zod 4 in TypeScript and revalidated in Rust before file mutation or helper control.
- Approved execution-baseline preservation: signing-ready Windows release verification, contract-surface freezing, and lifecycle/intervention logging baseline are required platform work even when this document preserves current story numbering to reduce churn.
- Booth-first large touchscreen UX: touch targets >=80px and immediate session-name validation with clear text.
- Session header shows session name and remaining time; a progress tracker shows stage (input, selection, capture, completion).
- Visual badge feedback plus sound for success/warning/error; 5-minute warnings are sound-backed.
- Responsive breakpoints: booth main 1024px+, operator 768-1023px, mobile <=767px (status-only).
- Accessibility target WCAG 2.2 AA with high contrast palette and multimodal alerts.
- Implementation guidelines: use `rem` units, semantic HTML, focus trapping for modals, and ESC to close.
- Design system uses Tailwind CSS + Headless UI with Brutal Core tokens (>=4px borders, hard offset shadows), Warm Beige background, Bold Black ink, and Pretendard Variable font.
- Course correction constraint: product is booth-first preset-driven; customer full editor is out of scope and RapidRAW detailed controls are internal preset-authoring only.
- Course correction constraint: epic map must align to the 7-epic replacement map in `sprint-status.yaml`.
- Course correction constraint: existing implementation-artifact stories are salvage candidates only and must be revalidated against the 2026-03-11 planning baseline before reuse.

### FR Coverage Map

FR1: Epic 1 - Minimal Session Start and Session Name Provisioning
FR2: Epic 2 - Approved Preset Catalog and Customer Preset Selection
FR3: Epic 3 - Guided Capture, Latest-Photo Confidence, and Current-Session Review
FR4: Epic 3 - Guided Capture, Latest-Photo Confidence, and Current-Session Review
FR5: Epic 3 - Guided Capture, Latest-Photo Confidence, and Current-Session Review
FR6: Epic 4 - Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff
FR7: Epic 4 - Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff
FR8: Epic 5 - Internal Preset Authoring and Approved Catalog Publication
FR9: Epic 6 - Exception Recovery, Operator Control, and Diagnostics

## Epic List

### Epic 1: Minimal Session Start and Session Name Provisioning
Customers can start a booth session quickly with a simple session name and reach a valid entry state.
**FRs covered:** FR1

### Epic 2: Approved Preset Catalog and Customer Preset Selection
Customers can view a bounded approved preset catalog and select one preset to begin capture.
**FRs covered:** FR2

### Epic 3: Guided Capture, Latest-Photo Confidence, and Current-Session Review
Customers can understand readiness, capture in valid states, see latest-photo confirmation, and review/delete current-session photos with forward-only preset changes.
**FRs covered:** FR3, FR4, FR5

### Epic 4: Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff
Customers can see adjusted session time, receive warning/end alerts, and follow clear post-end outcomes.
**FRs covered:** FR6, FR7

### Epic 5: Internal Preset Authoring and Approved Catalog Publication
Authorized users can create, tune, approve, and publish presets for the booth catalog with controlled rollout.
**FRs covered:** FR8

### Epic 6: Exception Recovery, Operator Control, and Diagnostics
Operators can diagnose blocked states, apply bounded recovery actions, and ensure safe booth operation.
**FRs covered:** FR9

### Epic 7: Operational Visibility and Safe Branch Delivery
Operations teams can monitor lifecycle and rollout health while delivering staged releases with rollback safety and zero forced updates.
**NFRs emphasized:** NFR1, NFR2, NFR3, NFR4, NFR5, NFR6

## Epic 1: Minimal Session Start and Session Name Provisioning

Customers can start a booth session quickly with a simple session name and reach a valid entry state.
**Execution classification:** Story 1.1 is explicit foundation/platform setup; Stories 1.2-1.4 remain booth-customer session-start flow.

### Story 1.1: Initialize Booth Project from Approved Starter Template

As a developer,
I want to initialize the Boothy project using the approved Vite + Tauri starter,
So that the codebase follows the architecture baseline from the first commit.

**Acceptance Criteria:**

**Given** a new project folder is created
**When** the following commands are executed in order
**Then** the repo is initialized with Vite React + TypeScript and a Tauri app: `pnpm create vite boothy --template react-ts --no-interactive`, `pnpm add -D @tauri-apps/cli@latest`, `pnpm exec tauri init`.
**And** `tauri.conf.json` is configured with App name `Boothy`, Window title `Boothy`, Web assets location `../dist`, Dev server URL `http://localhost:5173`, Frontend dev command `pnpm dev`, and Frontend build command `pnpm build`.

**Given** the starter initialization completes
**When** dependencies are installed
**Then** the project builds successfully with `pnpm build`
**And** the Tauri configuration references `../dist` and the Vite dev server URL.

### Story 1.2: Booth App Shell and Session Start Screen

As a booth customer,
I want a simple session-start screen with a single session name input,
So that I can begin a booth session quickly without extra friction.

**Acceptance Criteria:**

**Given** the app is run via `pnpm tauri dev`
**When** the booth window opens
**Then** the Session Start screen is shown with exactly one session name input and a start action
**And** no reservation, phone, or other inputs are present.

**Given** the session name field is empty or whitespace
**When** the customer attempts to proceed
**Then** an inline validation message is shown
**And** the start action is disabled or blocked until a non-empty name is entered.

### Story 1.3: Session Identity Creation and Session Root Manifest

As a booth customer,
I want my session name to create a valid session identity,
So that my booth session is uniquely tracked from the start.

**Acceptance Criteria:**

**Given** a non-empty session name
**When** the customer confirms session start
**Then** the host creates a new session root folder
**And** `session.json` is written with at least `schemaVersion`, `sessionId`, `sessionName`, and `createdAt`.

**Given** a host validation or filesystem error occurs
**When** session start is attempted
**Then** a typed error envelope is returned
**And** a customer-safe error message is shown without internal diagnostics.

### Story 1.4: Session Context Storage and Next-Surface Handoff

As a booth customer,
I want the app to remember my session after start,
So that subsequent screens can use the active session identity.

**Acceptance Criteria:**

**Given** a session has been created successfully
**When** the app receives the session identity response
**Then** the active session context is stored in the session domain state
**And** the UI transitions into the next surface entry state (preset selection entry placeholder).

**Given** no active session exists
**When** a user tries to access downstream surfaces
**Then** they are redirected to the Session Start screen
**And** no session-specific UI is shown.

## Epic 2: Approved Preset Catalog and Customer Preset Selection

Customers can view a bounded approved preset catalog and select one preset to begin capture.
**Execution classification:** This epic stays booth-customer facing; branch consistency and audit governance are handled in Epic 7.

### Story 2.1: Approved Preset Catalog Load and Display

As a booth customer,
I want to view a bounded catalog of approved presets with clear previews,
So that I can choose one look quickly.

**Acceptance Criteria:**

**Given** an active session exists
**When** the preset selection surface loads
**Then** the booth displays only 1-6 approved presets
**And** each preset shows a customer-facing name with a preview image or standard preview tile.

**Given** the catalog is unavailable or empty
**When** the selection surface loads
**Then** a customer-safe error or wait state is shown
**And** no internal diagnostic details are displayed.

### Story 2.2: Preset Selection and Active Preset Binding

As a booth customer,
I want to select one preset before capture begins,
So that my chosen look is applied to subsequent captures.

**Acceptance Criteria:**

**Given** the preset catalog is displayed
**When** I select a preset card and confirm
**Then** the selected preset becomes the active preset for the session
**And** the UI reflects the active preset state clearly.

**Given** no preset is selected
**When** I attempt to proceed to capture
**Then** the app prevents continuation
**And** prompts me to choose a preset.

### Story 2.3: Supported Preset Catalog Availability and Customer-Safe Blocking

As a booth customer,
I want preset selection to show only supported approved presets and fail safely when the catalog is not usable,
So that I can choose from a trustworthy booth catalog without seeing operational complexity.

**Acceptance Criteria:**

**Given** the preset selection surface loads
**When** the approved booth catalog is available for the active runtime
**Then** the customer sees only supported published preset entries
**And** no unpublished, unsupported, or diagnostic-only entries are shown.

**Given** the catalog is unavailable, incomplete, or unsupported for customer use
**When** the customer attempts to continue
**Then** preset selection is blocked with customer-safe wait/call guidance
**And** the booth does not expose branch-comparison, rollout, or audit language on the customer surface.

## Epic 3: Guided Capture, Latest-Photo Confidence, and Current-Session Review

Customers can understand readiness, capture in valid states, see latest-photo confirmation, and review/delete current-session photos with forward-only preset changes.
**Execution classification:** Customer review/deletion stories stay session-scoped and depend only on established session-manifest behavior plus the preserved platform baseline noted above.

### Story 3.1: Readiness States and Capture Gating

As a booth customer,
I want to see clear readiness guidance and only capture when the booth is ready,
So that I know when it’s safe to shoot.

**Acceptance Criteria:**

**Given** the booth is preparing, waiting, or phone-required
**When** the capture surface is shown
**Then** the UI displays a customer-safe readiness state message
**And** capture is blocked until the state becomes ready.

**Given** the booth becomes ready
**When** the readiness state transitions
**Then** the capture action is enabled
**And** the UI reflects the ready state without technical diagnostics.

### Story 3.2: Capture Persistence and Latest-Photo Confirmation

As a booth customer,
I want my captures saved to my session and see the latest photo quickly,
So that I trust the booth captured my photo.

**Acceptance Criteria:**

**Given** an active session and active preset
**When** a capture completes successfully
**Then** the capture is persisted in the session folder
**And** the latest photo preview is shown within 5 seconds for 95th-percentile captures.

**Given** the latest photo preview is displayed
**When** the capture surface is visible
**Then** the active preset name remains visible
**And** only current-session assets are shown.

### Story 3.3: Current-Session Review and Deletion

As a booth customer,
I want to review and delete only my current-session photos,
So that I can remove unwanted shots safely.

**Acceptance Criteria:**

**Given** the review surface is opened
**When** thumbnails are loaded
**Then** only current-session photos are shown
**And** no cross-session assets are accessible.

**Given** a customer deletes a current-session photo
**When** the deletion is confirmed
**Then** the original and derived assets are removed
**And** the session manifest is updated immediately so current-session review remains accurate.

### Story 3.4: In-Session Preset Change for Future Captures

As a booth customer,
I want to change the active preset during my session for future captures,
So that I can try a different approved look.

**Acceptance Criteria:**

**Given** an active session with captures already taken
**When** I select a different preset
**Then** the new preset is applied only to future captures
**And** existing captures remain unchanged.

**Given** a preset change is applied
**When** I return to capture
**Then** the UI shows the new active preset name
**And** no detailed editing controls are exposed.

## Epic 4: Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff

Customers can see adjusted session time, receive warning/end alerts, and follow clear post-end outcomes.

### Story 4.1: Session Timing Model and Visible Adjusted End Time

As a booth customer,
I want to see the adjusted session end time from the start,
So that I can trust how long I have.

**Acceptance Criteria:**

**Given** a session starts with an adjusted end time
**When** the capture surface is shown
**Then** the remaining time is visible in the session header
**And** the displayed time reflects the adjusted end time.

**Given** the session timing changes (coupon adjustment)
**When** the timing policy recalculates
**Then** the displayed time updates consistently
**And** the customer-facing guidance remains clear.

### Story 4.2: 5-Minute Warning and Exact-End Alerts

As a booth customer,
I want clear warnings before the session ends and a definitive end alert,
So that I can finish without confusion.

**Acceptance Criteria:**

**Given** the session is within 5 minutes of the adjusted end time
**When** the warning threshold is reached
**Then** a sound-backed warning alert is triggered
**And** a visible warning badge is shown.

**Given** the adjusted end time is reached
**When** the end threshold is hit
**Then** a sound-backed end alert is triggered
**And** the UI clearly indicates whether shooting can continue or has ended.

### Story 4.3: Post-End State Transition and Guidance

As a booth customer,
I want clear guidance after the session ends,
So that I know the next step without technical confusion.

**Acceptance Criteria:**

**Given** the session ends
**When** the system transitions post-end
**Then** it enters exactly one of: Export Waiting, Completed, or Handoff
**And** the customer sees clear next-step guidance.

**Given** the session name is required for handoff
**When** the handoff state is shown
**Then** the session name is displayed on the handoff surface
**And** if resolution fails, the customer is routed to wait or call guidance.

## Epic 5: Internal Preset Authoring and Approved Catalog Publication

Authorized users can create, tune, approve, and publish presets for the booth catalog with controlled rollout.

### Story 5.1: Authoring Surface Access and Preset Library

As an authorized preset manager,
I want access to a dedicated authoring surface with a preset library,
So that I can view and start managing presets safely.

**Acceptance Criteria:**

**Given** the runtime profile is authoring-enabled
**When** the authoring surface is opened
**Then** the preset library screen is accessible
**And** booth customers cannot access this surface.

**Given** the preset library is shown
**When** the list loads
**Then** it displays only existing internal presets with status (draft/approved/published)
**And** no customer-facing surfaces are affected.

### Story 5.2: Create or Edit Preset Draft with Internal Controls

As an authorized preset manager,
I want to create or edit a preset draft using detailed internal controls,
So that I can prepare a look for approval.

**Acceptance Criteria:**

**Given** the preset editor is opened
**When** I adjust internal controls and save
**Then** a draft preset is stored with a versioned identifier
**And** it is not visible in the booth customer catalog.

**Given** a draft preset exists
**When** I reopen it
**Then** the saved parameters and preview are restored
**And** no booth session data is modified.

### Story 5.3: Approve and Publish Preset to Booth Catalog

As an authorized preset manager,
I want to approve and publish a preset,
So that it appears in the booth’s approved catalog.

**Acceptance Criteria:**

**Given** a preset draft is ready
**When** I mark it approved and publish
**Then** an immutable preset bundle is created with version metadata
**And** the approved preset becomes available to the booth catalog.

**Given** a preset is not approved
**When** the booth catalog is loaded
**Then** that preset is excluded
**And** only approved published presets are shown.

### Story 5.4: Safe Publication Rollback to Prior Approved Version

As an authorized preset manager,
I want to roll back to a prior approved preset version,
So that I can recover quickly from a bad publish.

**Acceptance Criteria:**

**Given** an approved preset has multiple published versions
**When** I select a prior version to roll back
**Then** the booth catalog reverts to the selected approved version
**And** the rollback action is recorded for audit.

## Epic 6: Exception Recovery, Operator Control, and Diagnostics

Operators can diagnose blocked states, apply bounded recovery actions, and ensure safe booth operation.
**Execution classification:** These stories must be independently completable from current booth state plus preserved contract/logging baselines; they do not assume later operator stories are already implemented.

### Story 6.1: Operator Summary and Current Booth-State Visibility

As an operator,
I want a summary view of the current session and booth state,
So that I can quickly assess issues without exposing diagnostics to customers.

**Acceptance Criteria:**

**Given** the operator console is opened
**When** the summary loads
**Then** it shows current session context, timing state, and current booth-state summary
**And** customer-facing screens remain free of diagnostic data.

**Given** no recent issue context is available
**When** the operator views the summary
**Then** the UI shows a clear neutral state such as `No recent issue`
**And** the summary remains usable without undefined failure fields.

### Story 6.2: Bounded Operator Recovery Actions

As an operator,
I want a limited set of recovery actions,
So that I can restore safe operation without unsafe interventions.

**Acceptance Criteria:**

**Given** a blocked or fault state is detected
**When** the operator views recovery actions
**Then** only approved bounded actions are available
**And** each action is labeled with expected outcome and risk.

**Given** an operator executes a recovery action
**When** it completes
**Then** the operator sees the resulting action outcome and booth state
**And** the booth returns to a safe customer state or a wait/call state.

### Story 6.3: Normalized Fault Classification and Customer-Safe Routing

As an operator,
I want faults normalized into clear categories,
So that customer guidance remains safe and consistent.

**Acceptance Criteria:**

**Given** a fault occurs
**When** it is classified by the host
**Then** it maps to a standard category with severity and retryability
**And** the customer-facing state is a safe wait or call guidance.

### Story 6.4: Lifecycle and Intervention Audit Logging

As an operator,
I want lifecycle and intervention events logged,
So that operational issues can be reviewed and improved.

**Acceptance Criteria:**

**Given** a lifecycle transition or operator intervention occurs
**When** the event is processed
**Then** it is written to the audit log with correlation identifiers and a normalized event type
**And** recent operator context can query the latest relevant events without exposing them to customers.

## Epic 7: Operational Visibility and Safe Branch Delivery

Operations teams can monitor lifecycle and rollout health while delivering staged releases with rollback safety and zero forced updates.
**Execution classification:** This epic owns platform/operations governance, branch consistency, rollout safety, and release gates that are intentionally separated from booth-customer flow stories.

### Story 7.1: Active-Session Update Guard and Staged Rollout Control

As an operations lead,
I want rollouts to avoid forced updates during active sessions,
So that customer sessions are never interrupted.

**Acceptance Criteria:**

**Given** an active customer session is in progress
**When** an update becomes available
**Then** the system blocks any forced update for that session
**And** queues the update for the next safe window.

**Given** a rollout is initiated
**When** the rollout policy is applied
**Then** it supports staged branch-by-branch deployment
**And** records the rollout decision in audit logs.

### Story 7.2: Approved Preset and Timing Consistency Audit

As an operations lead,
I want a branch consistency audit for presets and timing rules,
So that all branches stay aligned with the approved catalog and timing policy.

**Acceptance Criteria:**

**Given** a branch runs the booth app
**When** the consistency audit is executed
**Then** it verifies the approved preset catalog and timing rules match the currently approved published baseline
**And** any mismatch is recorded in branch audit output and flagged with a safe operational alert.

### Story 7.3: Customer Copy-Budget Compliance Gate

As a product owner,
I want a release gate that verifies customer copy stays within the copy budget,
So that customer screens remain simple and free of internal terminology.

**Acceptance Criteria:**

**Given** a release candidate is prepared
**When** the copy audit runs
**Then** each primary customer state is checked for the one-instruction/one-supporting/one-action-label budget
**And** any screen containing internal diagnostic or authoring terms fails the gate.

### Story 7.4: Timing Reliability and Privacy Validation Report

As an operations lead,
I want a periodic report on timing reliability and session isolation,
So that I can confirm compliance with reliability and privacy targets.

**Acceptance Criteria:**

**Given** lifecycle and timing events are logged
**When** the report is generated
**Then** it includes warning/end timing accuracy and post-end transition latency statistics
**And** it reports any detected cross-session isolation violations as critical findings.
