# Sprint Change Proposal: Reframe Boothy as a Unified Camera + Full RapidRAW Editor Product

**Date:** 2026-03-09  
**Project:** Boothy  
**Prepared By:** Codex  
**Workflow:** Correct Course  
**Mode:** Batch  
**Change Type:** Product Definition Reset / Major Replanning  
**Primary Reference Package:** `reference/uxui_presetfunction`

## 1. Change Trigger

### Trigger Summary

The product direction has changed materially.

Boothy is no longer to be planned as a booth-only guided capture application that hands customers off to a separate next-room selection or printing step.

Boothy is now to be planned as a single packaged end-user product that combines:

- camera-guided capture flow
- real camera integration
- full in-app photo editing
- the complete RapidRAW editing feature surface from `reference/uxui_presetfunction`

This is not an epic-level backlog adjustment. It is a product-definition change that affects PRD, architecture, UX interpretation, epic structure, sprint sequencing, and implementation priorities.

### Source Clarifications Confirmed

The following product clarifications were explicitly confirmed by the product owner:

1. "Use RapidRAW as-is" means the complete photo editing feature set is in scope, not a selective subset.
2. The capture application and the editor must coexist inside the same packaged product.
3. The editor is for the final end user, not an operator-only or back-office surface.

These clarifications are now treated as authoritative planning input for this proposal.

## 2. Current Planning Problem

### Current Product Definition No Longer Matches Intended Product

The current planning set describes Boothy as a booth-centric guided customer flow focused on:

- check-in
- readiness
- preset selection
- capture reassurance
- export waiting
- session-name-based handoff to the next room

That planning package does not describe a product where the customer continues into a full-featured photo editor after capture inside the same packaged application.

### Current Direction of Reuse Is Too Narrow

The current architecture and research materials treat RapidRAW primarily as:

- a selective donor
- a source of UI patterns
- a host/UI reuse candidate

That is no longer sufficient.

The new direction requires planning Boothy as a product that operationally contains and exposes the full RapidRAW editor capability set to the end user, rather than merely borrowing selected presentation or interaction ideas.

### Why This Is a Major Change

This decision changes all of the following:

- what the product fundamentally is
- what the final user journey is
- what counts as MVP
- what "done" means for capture flow
- what "handoff" means after capture
- which existing stories remain valid
- which existing epics must be rewritten
- how the package is composed
- how the editor and camera stack interact

This cannot be solved by adding one more epic onto the current plan without first correcting the product definition itself.

## 3. Authoritative Product Decision

### New Product Definition

Boothy shall be redefined as a unified desktop application for end users that combines:

1. guided capture and camera-driven photo taking
2. session-aware image ingestion from the connected camera
3. the complete RapidRAW editor experience and feature set
4. save, export, and downstream print or delivery preparation from within the same packaged application

### Product Identity Statement

**Old identity:**  
Boothy is primarily a booth workflow application that guides customers through capture and then hands them off to a later selection or printing step.

**New identity:**  
Boothy is a camera-connected photo application for end users that begins with guided capture and continues directly into a full-featured in-app editor based on the complete RapidRAW editing surface.

### Mandatory Scope Inclusion

For planning purposes, the RapidRAW editor from `reference/uxui_presetfunction` is now considered a product baseline rather than an optional donor.

The planning assumption is:

- if a feature exists in the RapidRAW editor surface and is part of the shipped end-user workflow there, it is in scope unless explicitly excluded by a later approved scope decision

At this time, no exclusion has been approved.

### Practical Meaning of "Full RapidRAW Feature Surface"

For this replanning cycle, the end-user product scope must include the full editor behavior family represented by the reference package, including but not limited to:

- editor workspace and image canvas
- panel-based adjustment workflow
- preset application behavior
- crop and image adjustment controls
- mask-related editing controls
- history behavior such as undo and redo
- filmstrip or equivalent multi-image navigation
- export workflow
- non-destructive editing model and sidecar-backed edit persistence
- file or library navigation behavior exposed to the end user
- advanced editing features already present in the reference surface
- any shipping end-user editing modules present in the reference package that are not purely debug, packaging, or development scaffolding

The exact technical implementation may still be adapted, but the functional promise to the user is full editor availability, not a reduced booth-only subset.

## 4. Required Product Reframe

### Old End-to-End Journey

The current planning set effectively implies this journey:

1. customer starts session
2. customer checks in
3. customer waits for readiness
4. customer chooses preset
5. customer captures photos
6. customer sees latest photo reassurance
7. system exports or prepares results
8. customer moves to next room using a session-name handoff

### New End-to-End Journey

The revised product must instead be planned around this journey:

1. customer launches the app and enters guided capture mode
2. customer checks in or starts a new capture session
3. system confirms camera readiness and capture state
4. customer captures one or more photos
5. captured images are persisted into the active session
6. customer enters the in-app full RapidRAW-derived editor
7. customer edits the captured photos using the full editor capability set
8. customer saves, exports, or prepares output from inside the same product package
9. optional print, delivery, or next-step guidance happens after the editing workflow, not instead of it

### Product Modes

The product should now be explicitly planned as two tightly connected end-user modes within one packaged application:

#### Mode A: Guided Capture Shell

This mode remains valuable, but it is no longer the whole product.

Responsibilities:

- kiosk-style start and session entry
- reservation or session setup
- customer-safe readiness messaging
- camera availability and session creation
- guided capture flow
- immediate reassurance that images were captured successfully

#### Mode B: Full Editing Workspace

This is no longer optional or future-only.

Responsibilities:

- display captured images in the full editor workspace
- expose the complete RapidRAW editing toolset to the end user
- persist edits non-destructively
- support browsing and editing multiple captured images
- support export and final output preparation

### Critical Implication

The previous planning assumption that the customer leaves Boothy after capture is no longer correct.

Capture is now the front half of the product, not the full product.

## 5. What Must Be Preserved from Current Planning

Although the product definition changes, some existing work remains useful.

### Still Valid or Potentially Reusable

- desktop packaging direction using Tauri
- same-package local runtime model
- real camera helper or camera boundary concept
- session-folder-centered durable truth
- typed host boundary and local contract discipline
- readiness translation and customer-safe messaging patterns
- lifecycle and diagnostics logging foundation
- branch configuration and release discipline where still relevant

### No Longer Sufficient as Final Product Definition

- booth-only customer flow
- mandatory post-capture next-room handoff as the primary journey end
- fixed booth-only preset catalog as the main editing abstraction
- planning that treats the editor as out of scope
- selective RapidRAW reuse framing when the intended product is full editor inclusion

## 6. Artifact Impact Analysis

### 6.1 PRD Impact

**Impact level:** Major rewrite required

The current PRD is built around a booth-state product and emphasizes:

- reduced operator intervention
- bounded capture flow
- export waiting
- session-name handoff

That PRD must be reworked so the core product promise becomes:

- capture plus editing in one application
- full editor availability for the final user
- capture-to-editor continuity as a primary requirement
- in-app export and output flow after editing

#### PRD Areas That Must Change

- product vision
- executive summary
- success metrics
- user journey definitions
- functional requirements
- non-functional requirements
- MVP definition
- release gates
- assumptions and exclusions

#### PRD Areas That May Be Partially Preserved

- Windows desktop runtime assumptions
- local-first operational constraints
- camera readiness and capture reliability concerns
- privacy and session-isolation rules
- local packaging and release safety principles

### 6.2 Epics Impact

**Impact level:** Full epic rework required

The current epic set is organized around booth-only progression. That is no longer the right decomposition.

Some existing story work may still be reusable at implementation level, but the epic map itself no longer matches the product.

Specific conflicts include:

- Epic 3 currently treats handoff to next room as a core product end state
- Epic 2 treats preset/capture confidence as the central post-readiness experience rather than the opening of a later editor workflow
- editor parity and editor workspace integration do not exist as first-class epics
- the full RapidRAW editing surface is not decomposed anywhere in the current plan

### 6.3 Architecture Impact

**Impact level:** Major rewrite required

The current architecture still assumes selective donor reuse and a product centered on capture, export, and handoff.

The new architecture must instead define:

- one packaged application
- one end-user product
- two primary modes: guided capture and full editor
- a unified session model spanning capture and editing
- ingestion of captured images into the editor workflow
- editor persistence, history, and export as first-class architecture concerns

### 6.4 UX Design Impact

**Impact level:** Significant reinterpretation required

The current UX work around guided capture remains useful for the front half of the product, but it is incomplete for the newly confirmed product.

The UX specification must be expanded to cover:

- the transition from guided capture shell into the editing workspace
- the editor-first interaction model after capture
- how RapidRAW's existing editor surface is adopted or minimally adapted in the same package
- how booth-style reassurance and editor-style control coexist without feeling like two unrelated products

### 6.5 Sprint Status Impact

**Impact level:** Resequencing required

Current in-progress and ready-for-dev story statuses assume the old product definition.

They must be reinterpreted under the new plan:

- some stories become reusable foundations
- some become partial steps toward the new product
- some become superseded
- some must be rewritten before implementation continues

## 7. Detailed Product Redefinition

### 7.1 New Product Vision

Boothy is a unified Windows desktop photo product that starts with guided camera capture and continues directly into a full-featured end-user photo editor built around the complete RapidRAW editing experience.

### 7.2 Primary User

The primary user is the final customer.

This means:

- the editing experience is not hidden behind an operator surface
- the editor must be understandable and usable by the same person who just completed capture
- the product cannot be planned as "capture app for customer + separate editing logic for later staff"

### 7.3 Product Value Proposition

The revised product value is no longer only "reduce booth confusion."

It becomes:

- let the customer capture photos and immediately continue into a powerful editing workflow
- preserve continuity between photo taking and photo editing
- remove the gap between booth capture and editing software
- present one packaged product instead of a booth app plus an implied separate editing tool

### 7.4 Product Boundary

The product boundary must now include:

- camera session setup
- capture
- image persistence
- editing workspace
- editing persistence
- export and output preparation

Any external next-room, print-room, or handoff process is now secondary and optional from a product-definition perspective unless later reintroduced as a deployment-specific operational step.

## 8. Recommended Technical Direction After Reframe

### Chosen Packaging Direction

The revised product should be planned as:

- one packaged Tauri desktop application
- one shared installation
- one shared local storage model
- one shared session or project model
- two end-user experiences within the same application package

### Preferred Composition Model

#### Layer 1: Application Shell

Responsibilities:

- launch behavior
- mode selection
- window and routing structure
- session creation and product-level navigation
- customer-safe start flow

#### Layer 2: Capture Domain

Responsibilities:

- camera readiness
- capture triggering
- capture persistence
- latest-photo feedback
- session-aware image ingestion

#### Layer 3: Editor Domain

Responsibilities:

- editor workspace rendering
- adjustment tools
- filmstrip or image navigation
- non-destructive edit state
- history behavior
- export behavior
- advanced editing modules already present in the reference editor

#### Layer 4: Local Data and Asset Domain

Responsibilities:

- session folders
- captured image storage
- edit sidecar or project-state persistence
- thumbnails and preview caching
- export outputs

#### Layer 5: Camera Boundary

Responsibilities:

- physical camera integration
- device readiness
- live capture
- transfer of captured files into the active session

### Editor Adoption Direction

The planning assumption should now be:

- `reference/uxui_presetfunction` is the primary editor baseline to preserve
- Boothy-specific work wraps around it and connects capture flow into it
- the team should minimize unnecessary redesign of the existing full editor surface unless a later product decision explicitly requires simplification

This is a materially different stance from selective donor reuse.

The new working stance is:

- preserve editor capability first
- adapt shell, routing, session flow, and camera ingestion around it

### Session and Data Model Direction

The new architecture should define one continuous session or project lifecycle:

1. session created
2. photos captured into session storage
3. session images opened directly in the editor
4. edits persisted non-destructively
5. exports produced from the edited project

This continuous model replaces the previous planning emphasis on export-waiting and next-room handoff as the main end state.

## 9. Proposed Replacement Epic Structure

The current epic set should not simply be patched. A replacement epic map is recommended.

### Epic 1: Unified Product Shell and Same-Package App Composition

Goal:

- establish the single packaged application structure that contains both the guided capture shell and the full editor workspace

Key outcomes:

- same-package app composition
- startup and routing model
- capture-mode entry
- editor-mode entry
- shared session model
- shell-level visual coherence between capture shell and RapidRAW editor workspace

### Epic 2: Guided Capture Flow and End-User Camera Readiness

Goal:

- deliver the booth-like guided capture front end for the final user

Key outcomes:

- start flow
- check-in or session creation
- readiness
- customer-safe capture state
- capture loop
- latest-photo reassurance

### Epic 3: Real Camera Integration and Capture-to-Editor Ingestion

Goal:

- connect live camera capture to the shared session model and ingest captured images directly into the editor workflow

Key outcomes:

- live readiness
- real capture persistence
- session-folder truth
- editor-side image availability after capture
- multi-capture continuity

### Epic 4: Core RapidRAW Editor Workspace Parity

Goal:

- expose the core RapidRAW editor experience inside the unified packaged product

Key outcomes:

- editor canvas
- right and left panel structure
- filmstrip or image navigation
- core adjustments
- preset behavior
- history model
- non-destructive persistence

### Epic 5: Advanced RapidRAW Editing Feature Parity

Goal:

- preserve the broader advanced editing surface from the RapidRAW package

Key outcomes:

- masking tools
- advanced image operations
- any existing higher-order editing workflows already present in the reference package
- advanced modules that are part of the end-user editor promise

### Epic 6: Export, Save, Output, and Final User Delivery Flow

Goal:

- define how the customer finishes work inside the same product after editing

Key outcomes:

- save behavior
- export workflow
- output preparation
- edited result persistence
- optional print-ready or downstream delivery integration

### Epic 7: Operational Controls, Logging, Rollout, and Recovery

Goal:

- preserve the operational discipline needed to ship and support the product

Key outcomes:

- diagnostics
- lifecycle logs
- branch config where still relevant
- release safety
- rollback
- bounded recovery workflows

## 10. Proposed PRD Rewrite Direction

The PRD should be rewritten around the following requirement groups.

### Proposed Functional Requirement Families

#### FR-001 Unified Product Entry

The system shall launch as one packaged application that contains both guided capture and the full editor workspace.

#### FR-002 Guided Capture Session

The system shall let the final user begin a session, reach camera-ready state, and capture photos inside a guided capture shell.

#### FR-003 Real Camera Capture Persistence

The system shall persist captured images into the active session and make them available to the editor without requiring a separate product or manual file transfer.

#### FR-004 Full In-App Editor Availability

The system shall expose the full RapidRAW editing feature surface to the final user within the same packaged application.

#### FR-005 Non-Destructive Edit Persistence

The system shall preserve edits non-destructively and maintain a consistent project or sidecar model across editing sessions.

#### FR-006 Multi-Image Editing Workflow

The system shall support navigation and editing across multiple captured images in the active session through filmstrip or equivalent editor navigation.

#### FR-007 Export and Output Completion

The system shall let the final user save, export, and prepare final outputs from within the application after editing.

#### FR-008 Camera and Editor Continuity

The system shall preserve continuity between capture and editing so the user does not leave one product and manually enter another.

#### FR-009 Operational Safety and Recovery

The system shall continue to support bounded diagnostics, recovery, and release safety appropriate for the deployment environment.

### Proposed Non-Functional Requirement Families

#### NFR-001 Same-Package Continuity

Capture mode and editor mode must feel like one product, not two unrelated tools bundled together.

#### NFR-002 Full Feature Commitment

The shipped end-user editor surface must preserve the complete RapidRAW editing capability set unless an explicit later scope reduction is approved.

#### NFR-003 Local Performance and Responsiveness

The unified product must remain responsive enough for capture, image loading, editing, and export on the approved Windows target environment.

#### NFR-004 Session Isolation and Privacy

Captured and edited assets must remain session-scoped and must not leak between users.

#### NFR-005 Reliable Capture-to-Editor Transition

The path from successful capture to editable image availability must be durable and deterministic.

#### NFR-006 Safe Local Packaging and Rollout

The application must preserve signed packaging, rollout safety, and rollback discipline.

## 11. Existing Story Disposition

This section recommends how the current story set should be treated during replanning.

### Likely Reusable with Revision

- Story 1.1 foundation work
- Story 1.2 release baseline work
- Story 1.3 host-facing contract and session schema baseline
- Story 1.4 logging foundation
- parts of Story 1.6 readiness UX
- parts of Epic 6 camera integration work

### Reusable but No Longer Sufficient as Final Product Stories

- Story 2.1 preset selection
- Story 2.2 in-session preset change
- Story 2.3 timing policy
- Story 2.4 capture confidence
- Story 2.5 session-scoped review and delete

These may remain useful as capture-shell subfeatures, but they no longer represent the dominant product workflow.

### Likely Superseded or Fundamentally Rewritten

- Epic 3 handoff-centered stories
- any story whose final customer outcome is "go to the next room" rather than "continue into the in-app editor"

### Operationally Preservable but Repositioned

- operator and rollout safety work
- diagnostics and logging work
- bounded recovery workflows

These should move later in priority unless they directly block the new unified app composition.

## 12. Recommended Planning Actions

### Action 1: Approve This Product Reframe

The team must first approve that Boothy is now a unified camera plus full editor product.

Without that approval, artifact edits will remain ambiguous.

### Action 2: Rewrite the PRD

The PRD must be rewritten before architecture and epic generation continue in earnest.

This is required because the previous PRD still defines the wrong product ending and the wrong core value proposition.

### Action 3: Rewrite the Architecture

The architecture must be rewritten around:

- one packaged app
- guided capture shell
- full editor workspace
- unified session flow
- camera ingestion into the editor

### Action 4: Replace the Epic Map

The current epic map should be retired and replaced with the revised epic structure proposed above or an approved equivalent.

### Action 5: Re-sequence Implementation

Implementation must be re-sequenced so the team does not continue building an increasingly polished booth-only shell while the editor product remains underplanned.

## 13. Recommended BMAD Workflow Sequence

This sequence is recommended for controlled replanning:

1. `Correct Course`
   - approve the change formally
2. `Edit PRD`
   - rewrite product definition
3. `Create Architecture`
   - redesign the unified app structure
4. `Create Epics and Stories`
   - generate the new backlog
5. `Implementation Readiness`
   - verify alignment
6. `Sprint Planning`
   - regenerate sequencing

## 14. Risks of Not Replanning Now

If the team continues under the current plan without this reframe, the likely outcome is:

- capture shell continues to improve
- camera integration continues to deepen
- editor remains underdefined or accidentally reduced
- product drifts toward a booth-only app plus future editor promise
- rework cost grows as capture-specific assumptions harden

This is the worst timing to defer the correction because the codebase has enough foundation to continue moving, but not enough product lock to prevent architectural drift.

## 15. Approval Criteria for This Change Proposal

This proposal should be considered accepted only when the team agrees to all of the following:

1. Boothy is not a booth-only app anymore.
2. Boothy is a unified same-package camera plus full editor product.
3. The full RapidRAW editor feature surface is in scope for the final user.
4. The current PRD, architecture, and epics are no longer adequate as-is.
5. Replanning must happen before continuing substantial implementation under the old product definition.

## 16. Final Recommendation

Approve a full product-definition correction.

Do not treat this as an incremental scope extension.

Replan Boothy as a unified end-user `camera + full RapidRAW editor` application in one package, rewrite the PRD and architecture accordingly, and regenerate the epic and story structure from that corrected product definition before continuing major implementation.
