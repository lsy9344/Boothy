---
workflowType: 'prd'
workflow: 'edit'
classification:
  domain: 'general'
  projectType: 'desktop_app'
  complexity: 'high'
date: '2026-03-11'
inputDocuments:
  - 'docs/business_context/context.md'
  - 'docs/research-checklist-2026-03-07-boothy-greenfield.md'
  - '_bmad-output/planning-artifacts/sprint-change-proposal-2026-03-11.md'
  - '_bmad-output/planning-artifacts/sprint-change-proposal-2026-03-12.md'
  - '_bmad-output/planning-artifacts/validation-report-2026-03-12.md'
stepsCompleted:
  - 'step-e-01-discovery'
  - 'step-e-02-review'
  - 'step-e-03-edit'
lastEdited: '2026-03-12'
editHistory:
  - date: '2026-03-08'
    changes: 'BMAD structure, measurable FR/NFR, traceability, desktop app requirements added'
  - date: '2026-03-08'
    changes: 'NFR metrics, desktop app rollout/update requirements, FR measurability refinements added'
  - date: '2026-03-08'
    changes: 'frontmatter date added, core state summary added, Owner/Brand traceability clarified, FR-007 wording refined'
  - date: '2026-03-08'
    changes: 'customer-facing camera connection requirements and related journey, FR, NFR, and release gates updated'
  - date: '2026-03-10'
    changes: 'Full PRD rewrite for unified same-package camera plus full RapidRAW editor product; MVP scope, journeys, FR/NFR, and release gates reset'
  - date: '2026-03-10'
    changes: 'Validation-driven refinements: explicit MVP editor feature inventory baseline, session lifecycle summary, and tighter FR measurability wording'
  - date: '2026-03-11'
    changes: 'Full PRD rewrite to corrected booth-first preset-driven product; customer full editor removed, timing and handoff elevated, internal RapidRAW preset authoring separated from booth workflow'
  - date: '2026-03-11'
    changes: 'Validation-driven refinements: FR measurability wording tightened, FR-008 internal controls abstraction applied, and NFR-001 copy-budget threshold defined'
  - date: '2026-03-12'
    changes: 'Edit workflow refinements: customer preset-only booth flow locked, direct customer editing explicitly excluded, and RapidRAW-equivalent capabilities restricted to internal preset authoring'
  - date: '2026-03-12'
    changes: 'Validation-driven refinements: NFR-006 rollout and rollback measurability tightened, post-end state boundaries clarified, and repeated internal-authoring boundary wording compressed'
  - date: '2026-03-12'
    changes: 'Validation-driven cleanup: removed missing input document reference from frontmatter'
---

# Boothy Product PRD

Written: 2026-03-07  
Last Edited: 2026-03-12  
Status: Draft v1.1 (Corrected Product Definition)  
Document Type: Product Requirements Document  
Note: This PRD defines what the product must do and why it matters. It does not decide implementation frameworks, adapter internals, or code-level reuse strategy.

## Executive Summary

### Product Definition

Boothy is a booth-first Windows desktop photo product for booth customers. The customer enters a simple session name, chooses one approved preset, captures photos confidently, reviews only the current session, and completes a clearly timed session through export-waiting, completion, or handoff guidance. The booth customer does not enter a direct photo-editing workflow before or after capture.

RapidRAW-derived or RapidRAW-equivalent detailed controls are not part of the customer booth workflow. They are used only by authorized internal users to create and maintain the approved presets that customers later choose from in the booth.

### Business Problem

Current self-photo booth experiences can lose customer confidence when session start feels administrative, look choice is unclear, capture success is ambiguous, or session timing changes appear late or unpredictably.

At the same time, businesses need consistent looks across branches without exposing editing complexity to customers, and operators need bounded tools when the booth cannot continue safely.

The product must solve both problems at once:

- fast booth entry
- bounded creative choice through approved presets
- reliable capture and latest-photo confidence
- current-session-only review and cleanup
- visible and trustworthy timing guidance
- completion and handoff clarity
- bounded operational recovery

### Primary Users

- Customer: the primary user who wants a quick start, one clear look choice, confident capture, and a clear finish
- Remote operator: the secondary user who diagnoses blocked states and performs limited recovery actions
- Authorized preset manager: the internal user who creates, tunes, approves, and publishes booth presets
- Owner / brand operator: the business stakeholder who needs consistent product behavior, safe rollout, and measurable operational outcomes

### Product Thesis

Boothy's value is not booth automation alone and not end-user editor power. Its value is confident booth use without technical burden.

Customers start quickly, choose a look they understand, capture photos with immediate confidence, and finish a clearly timed session without ambiguity. Internal teams retain creative control through preset authoring, while customers interact only with the approved results of that work.

### Differentiation

- Session-name-based booth start with minimal friction
- Small approved preset catalog built from internal RapidRAW-derived preset authoring
- Guided capture with immediate latest-photo confidence
- Current-session-only review, deletion, and forward-only preset changes
- Adjusted end time visible from the start of the session
- Sound-backed 5-minute warning and exact-end alert
- Separate customer, operator, and internal preset-authoring capability boundaries
- Local-first operation, rollout safety, and branch consistency

### Core Product Modes

Boothy operates across three distinct but related modes.

**Mode A: Customer Booth Flow**
- session name entry
- preset selection
- readiness and capture
- current-session review
- timing guidance
- export-waiting, completion, and handoff guidance

**Mode B: Operator Recovery Surface**
- current session visibility
- fault diagnosis
- bounded recovery actions
- lifecycle and intervention review

**Mode C: Internal Preset Authoring Surface**
- preset creation and tuning
- preset approval and publication
- preset rollback and maintenance

### Core Product State Model

The customer-facing experience should translate booth state into simple, action-oriented language.

- `Preparing`: the booth is starting the session or confirming readiness
- `Ready`: the booth can begin or continue capture
- `Capturing`: the booth is actively capturing photos into the current session
- `Review`: the customer can see current-session photos and approved cleanup actions
- `Warning`: the booth is inside the final 5-minute guidance window before the scheduled end time
- `Export Waiting`: shooting has ended and the booth is preparing the end-of-session result state or handoff
- `Completed`: the session is ready to close or continue to the approved next step
- `Phone Required`: safe continuation is blocked beyond approved recovery bounds

## Success Criteria

### Primary Business Outcome

Within the first 60 days of pilot rollout, Boothy should let customers start quickly, choose an approved look, capture confidently, and complete a clearly timed booth session while reducing support burden compared with the previous booth flow.

### KPI Table

| Metric | Baseline | Target | Measurement Method |
| --- | --- | --- | --- |
| Monthly phone / remote support incidents | Approx. 500 incidents per month across all branches | 250 or fewer incidents per month | Combined support and remote assistance logs |
| Self-start session success rate | Unknown | 85% or higher | Sessions that reach first successful capture without operator intervention |
| Preset selection completion rate | New metric | 95% or higher | Sessions that reach preset selection and choose a preset within 20 seconds without operator intervention |
| Latest-photo confidence feedback latency | New metric | Latest photo visible within 5 seconds for 95th-percentile successful captures | Lifecycle logs and pilot timing review |
| Adjusted end-time visibility correctness | New metric | 99% or higher | Sessions where the displayed end time matches the approved timing policy from session start |
| Warning and end alert reliability | New metric | 99% or higher | Qualifying sessions where the 5-minute warning and exact-end alert occur within +/- 5 seconds of scheduled time |
| Post-end resolution rate | New metric | 90% or higher | Sessions that reach `Completed` or `Phone Required` within 2 minutes of scheduled end time |
| Cross-session photo leak incidents | Must be zero | 0 | Privacy test cases, pilot defect logs, and operator incident reports |
| Operator first action time after critical failure | Unknown | Average 3 minutes or less | Time from `Phone Required` entry to first operator action |

### Qualitative Outcomes

- Customers should feel they can start immediately without learning software.
- Customers should feel their main creative choice is clear and bounded.
- Customers should feel that choosing a preset is sufficient and that no manual editing step is required to complete the booth session.
- Customers should trust that captures succeeded and that time remaining is truthful.
- Customers should understand what happens at warning time, end time, and completion without staff improvisation.
- Operators should feel informed and bounded, not responsible for guessing booth truth.
- Owners should feel the product is consistent, supportable, and marketable across branches.

### Qualitative Validation Checks

| Goal | Operational Signal | Measurement Method |
| --- | --- | --- |
| Session start and preset choice feel obvious | 80% or more of observed pilot users start a session and select a preset without asking what to do next | Pilot observation logs and moderated walkthroughs |
| Timing feels trustworthy | 90% or more of observed pilot users can correctly state whether shooting can continue after warning and exact-end alerts | Pilot observation logs and session-end interviews |
| Internal controls remain invisible to customers | 100% of customer pilot sessions expose no detailed preset-authoring controls or internal authoring language | UX acceptance review and pilot defect logs |
| Operators remain bounded | 90% or more of top exception drills are handled using approved recovery actions only | Operator drill logs and support retrospectives |

## Product Scope

### MVP In Scope for the Booth Customer

- One packaged Windows desktop booth application for the customer workflow
- Very simple session start centered on session name entry
- Approved preset selection from a small bounded catalog
- Customer-safe readiness guidance and capture control
- Real camera readiness, trigger, and image persistence for the required booth workflow
- Latest-photo confidence feedback
- Current-session review only
- Deletion of current-session photos within approved bounds
- Forward-only preset changes for future captures
- Adjusted end-time visibility from session start
- Sound-backed 5-minute warning and exact-end alert
- Export-waiting, completion, and handoff guidance
- Bounded customer-safe failure and escalation messaging

### MVP In Scope for Internal or Authorized Users

- Preset creation and tuning using RapidRAW-derived detailed controls
- Preset approval and publication to the booth catalog
- Diagnostics, bounded recovery, and operational maintenance
- Rollout and rollback controls

### MVP Scope Clarifications

- Customer creative choice is preset selection, not direct image editing.
- The booth customer never enters a direct photo editor or detailed adjustment surface during the session.
- Customer review is limited to the current session and approved cleanup actions.
- Session timing may change based on coupon usage or other approved operational policy, but the adjusted end time must be visible from the beginning of the session.
- Completion may take the form of export-waiting, completion, or handoff guidance. It does not require customer-side detailed editing.
- RapidRAW-derived or RapidRAW-equivalent capability may be reproduced for internal preset authoring, but it is never exposed as a booth-customer editing mode.
- The PRD defines product behavior. Architecture will decide how internal preset authoring is delivered technically.

### Customer Preset Experience Baseline

The customer preset promise is locked to the following booth-facing behaviors. Downstream architecture, UX, and epic artifacts may refine these behaviors, but they may not silently weaken them without an approved PRD revision.

- The booth customer sees only approved published presets.
- The customer selects one active preset at a time from a bounded catalog of 1-6 options.
- Each preset has a descriptive name and preview or equivalent clear representation.
- The active preset remains visible enough for the customer to understand the current look.
- In-session preset changes affect future captures only unless a later approved PRD revision says otherwise.
- The booth customer never opens a direct editing workspace or sees detailed RapidRAW or RapidRAW-equivalent controls, masking tools, curve tools, or low-level color controls.

### Growth / Later

- Reservation verification against external systems if it improves operations without slowing booth start
- Centralized coupon or operational policy sync
- Centralized preset distribution and analytics
- Optional print or downstream delivery integrations
- Marketing, messaging, and follow-up automation
- Expanded branch configuration management

### Explicitly Out of Scope for MVP

- Full end-user editor workspace
- Direct customer-operated photo-editing workflow during the booth session
- Direct use of RapidRAW detailed adjustment tools on the booth customer surface
- Direct use of RapidRAW-equivalent detailed adjustment tools on the booth customer surface
- Mask, curve, and detailed color controls for booth customers
- Customer-side capture-to-editor continuity as the main product journey
- Building a general-purpose cloud photo library product
- Cross-device session editing sync
- Unapproved branch-specific preset catalogs
- Forced updates during active customer sessions

### External Boundaries

- Reservation lookup, if later added, remains external and must not block the basic booth-start flow when the MVP input format is valid.
- Coupon or operational-policy lookups may use external operational data sources such as Google Sheets or equivalent approved systems.
- Remote support tools such as AnyDesk and Chrome remain operational tools, not customer-facing product features.
- Architecture will decide implementation strategy, persistence mechanics, and delivery model for internal preset authoring.

## User Journeys

### Primary Personas

#### Customer

- May have low technical confidence
- Wants a quick start, clear next actions, and one understandable creative choice
- Expects preset selection rather than manual adjustment controls or editor tooling
- Expects confidence after each capture and clarity about time remaining

#### Remote Operator

- Intervenes only when the booth cannot proceed within bounded recovery rules
- Needs compressed truth, recent failure context, timing state, and approved recovery actions

#### Authorized Preset Manager

- Creates and tunes approved looks for booth customers
- Needs detailed control over preset quality without exposing those controls in the booth
- Needs approval and publication discipline across branches

#### Owner / Brand Operator

- Needs a consistent, marketable booth experience across branches
- Needs measurable support reduction, visual consistency, and rollout safety

### Session Lifecycle Summary

| Lifecycle State | Entry Condition | Exit Condition |
| --- | --- | --- |
| Preparing | Session name input is accepted or session recovery begins | Ready or Phone Required |
| Ready | Session and booth are in a valid state for capture | Capturing or Phone Required |
| Capturing | One or more photos are being captured into the active session | Review, Ready, Warning, or Phone Required |
| Review | Only active-session photos are visible for confidence checks and approved cleanup | Capturing, Warning, Export Waiting, or Phone Required |
| Warning | The booth is within the final 5-minute timing window | Capturing, Review, Export Waiting, or Phone Required |
| Export Waiting | Scheduled shooting time has ended, shooting is disabled, and the booth is preparing the end-of-session deliverable or handoff package | Completed or Phone Required |
| Completed | The required end-of-session deliverable or approved handoff package is ready and the customer can follow the final next step without more booth actions | Session close |
| Phone Required | Safe continuation is blocked beyond bounded recovery rules | Operator restores a prior valid state or escalates |

### Customer Journey

| Stage | Customer Goal | Product Responsibility | Success Condition |
| --- | --- | --- | --- |
| App launch | Understand what this booth is and how to begin | Present one clear booth entry without OS exploration | Customer starts without asking what app to use |
| Session start | Open a session quickly | Accept simple session-name input, create active session identity, and enter preparation | Customer reaches the preparing or ready flow without operator help |
| Preset selection | Choose a look with minimal hesitation | Show only approved presets with clear names and previews | Customer chooses one preset confidently |
| Preset lock-in | Understand the editing boundary | Keep the booth experience focused on preset choice and capture rather than direct editing | Customer understands they are choosing a finished look, not entering an editor |
| Readiness | Know whether capture can begin | Show plain-language preparation, ready, waiting, or phone-required guidance | Customer understands whether to wait, capture, or call |
| Capture | Take one or more photos with confidence | Trigger capture, confirm persistence, show the latest result, and preserve session isolation | Customer trusts that capture is working |
| Review and cleanup | Confirm captured photos are usable | Show only current-session photos, allow approved deletion, and allow future-capture preset change | Customer sees only their own session assets and can manage them within bounds |
| Timing guidance | Understand how much time remains and what happens next | Show adjusted end time, 5-minute warning, and exact-end behavior clearly | Customer understands whether shooting can continue or has ended |
| Completion or handoff | Finish the session without ambiguity | Keep the customer in export-waiting until the end-of-session deliverable is ready, then show completed or handoff guidance with the right next action | Customer knows whether to keep waiting, move to the next location, or call |

### Operator Journey

| Stage | Operator Goal | Product Responsibility | Success Condition |
| --- | --- | --- | --- |
| Normal operation | Stay out of the way | Let the customer complete the booth flow without intervention | No support action is needed |
| Exception detection | Understand the blocked state quickly | Show current session, timing state, recent failures, and bounded next actions | Operator identifies the situation within 60 seconds |
| Recovery | Restore progress safely | Provide only approved actions such as retry, restart boundary, time extension, or recovery routing | Customer returns to a valid state or receives clear escalation guidance |
| Audit | Understand recurring problems | Record lifecycle and intervention events for review | Support patterns can be categorized and tracked |

### Authorized Preset Manager Journey

| Stage | Internal Goal | Product Responsibility | Success Condition |
| --- | --- | --- | --- |
| Authoring | Create or tune a booth look | Provide RapidRAW-derived detailed controls in an internal workflow | The preset can be refined without exposing booth customers to those controls |
| Approval | Decide whether a look is ready for customers | Support approval review before booth publication | Only approved presets move forward |
| Publication | Make a preset available in the booth catalog | Publish the approved preset into the bounded customer-facing catalog | Booth customers see only approved presets |
| Maintenance | Revise or roll back a preset safely | Support controlled updates and rollback | Branches remain consistent and recoverable |

### Owner / Brand Journey

Owner and brand stakeholders are not direct on-screen users, but they need the product to:

- present a fast, premium, booth-first identity
- keep approved looks consistent across branches
- reduce support burden while preserving trust
- preserve rollout and rollback safety

### Owner / Brand Traceability

| Owner / Brand Need | Supporting Requirements |
| --- | --- |
| Market a fast booth-first experience instead of a complex editing workflow | Executive Summary, FR-001, FR-002, FR-006, NFR-001 |
| Keep approved looks consistent across branches | Product Scope, FR-008, NFR-002 |
| Reduce support burden while preserving safe operations | Success Criteria, FR-009, NFR-005, NFR-006 |
| Protect privacy and trust across customer sessions | Domain Requirements, FR-004, FR-005, NFR-004 |

### Traceability Summary

| Business Objective / Journey | Supporting FRs |
| --- | --- |
| Customers start quickly and choose a look with confidence | FR-001, FR-002, FR-003 |
| Customers trust that captured photos belong to the current session | FR-004, FR-005 |
| Customers understand time remaining and end-of-session behavior | FR-006, FR-007 |
| Internal teams control visual quality without exposing complexity to customers | FR-008 |
| Operators remain bounded and useful | FR-009 |

## Domain Requirements

Boothy is a general-domain desktop product, not a regulated healthcare, fintech, or govtech system. Even so, the product has hard operational requirements:

- Customers must never see another customer's captured, reviewable, or handoff-related assets.
- The product must treat session name, selected preset, captures, derived previews, and exported outputs as session-scoped by default.
- Customer-facing surfaces must use plain-language status and next-action guidance rather than technical diagnostics or internal preset-authoring language.
- RapidRAW-equivalent adjustment controls, editor terms, and low-level image-tuning labels must never appear on booth customer surfaces.
- The displayed end time must be the authoritative customer timing truth for the active session.
- The product must provide a bounded `Phone Required` path when it cannot continue safely.
- Internal preset authoring must never leak detailed controls into the booth customer surface.
- Operators must not gain unbounded product control through the support surface.
- The product must preserve original captures and approved current-session actions in a way that supports trust and recovery.

## Innovation Analysis

Boothy's differentiation is not "booth only" and not "customer editing power." The innovation is using internal craft to create a simpler, more trustworthy booth experience.

- Internal teams can build and maintain deliberate looks with RapidRAW-derived controls.
- Customers interact only with the approved results of that work through a bounded preset catalog rather than direct image-editing tools.
- The product reduces booth friction by making the main creative choice obvious, not by exposing more tools.
- The timing system becomes part of the product value because it makes session limits visible and predictable from the start.

In practical terms, the product promise is:

- start quickly
- choose a look confidently
- capture within a clearly timed session
- finish without ambiguity

## Project-Type Requirements

Boothy is classified as a `desktop_app`. The product therefore must satisfy the following platform-level conditions:

- The customer workflow runs as a Windows desktop booth application on approved branch hardware.
- The customer flow must remain usable without browser navigation, mobile-device assistance, or manual operating-system file browsing.
- The customer surface should assume a booth environment with clear, low-choice interactions rather than a workstation-style interface.
- The customer booth workflow must not expose a workstation-style editor mode or direct image-adjustment workspace.
- The product must preserve local-first operation for active sessions, including capture, review, timing guidance, and completion or handoff flows.
- The product must support separate customer and operator surfaces with different levels of truth and control.
- Internal preset-authoring capability must be restricted to authorized users and must not appear in the booth customer flow.
- Branch rollout must support staged deployment, rollback, and no forced update during an active customer session.
- Branch-level variance must remain tightly controlled so preset catalog, timing rules, and core customer journey remain consistent across locations.

## Functional Requirements

### FR-001 Simple Session Start

Users can start a booth session by entering a non-empty session name as the only required booth-start input.

**Acceptance Criteria**
- The booth-start surface accepts a non-empty session name as the only required user-entered field.
- Invalid or empty input is shown before the customer proceeds.
- Valid input creates an active session identity for the current booth session.
- The customer can continue into the preparing or ready flow without mandatory reservation verification or phone-number entry.

**Sources**
- Executive Summary / Product Definition
- Customer Journey / App launch, Session start

### FR-002 Approved Preset Catalog Selection

Users can choose one approved preset from a bounded catalog before shooting begins.

**Acceptance Criteria**
- The booth presents only 1-6 approved presets to the customer.
- Each preset includes a customer-facing name and one preview image or standardized preview tile.
- The customer can activate one preset before capture begins.
- The activated preset becomes the active preset for subsequent captures until the customer changes it.
- No direct photo-editing workspace or detailed image-adjustment controls are exposed in preset selection.

**Sources**
- Product Scope / Customer Preset Experience Baseline
- Customer Journey / Preset selection

### FR-003 Readiness Guidance and Valid-State Capture

Users can understand whether the booth is preparing, ready, waiting, or phone-required and can capture only in approved valid states.

**Acceptance Criteria**
- The customer sees plain-language readiness guidance before first capture.
- The booth blocks capture when session or device state is not approved for capture.
- Customer-facing states avoid technical diagnostics.
- Blocked states tell the customer whether to wait or call rather than troubleshoot.

**Sources**
- Customer Journey / Readiness, Capture
- Success Criteria / Self-start session success rate

### FR-004 Capture Persistence and Latest-Photo Confidence

The system can persist captured photos into the active session and show the latest current-session result as confidence feedback.

**Acceptance Criteria**
- A successful capture is associated with the active session.
- The latest captured photo becomes visible to the customer as confirmation.
- Displayed capture confirmation includes only current-session assets.
- The active preset name remains visible on the capture surface and latest-photo confirmation surface while that preset is active.

**Sources**
- Customer Journey / Capture, Review and cleanup
- Domain Requirements / session isolation

### FR-005 Current-Session Review, Deletion, and Future-Capture Preset Change

Users can review only current-session photos, delete unwanted current-session photos within approved bounds, and change the active preset for future captures.

**Acceptance Criteria**
- The review surface exposes current-session photos only.
- The customer can delete approved current-session photos.
- The customer can change the active preset during the session.
- Preset changes affect future captures only unless a later approved PRD revision changes that rule.
- The product does not expose a direct photo-editing workspace or detailed editing controls as part of review.

**Sources**
- Customer Journey / Review and cleanup
- Product Scope / MVP In Scope for the Booth Customer

### FR-006 Coupon-Adjusted Timing, Warning Alerts, and Exact-End Behavior

The system can manage customer session timing using approved rules and present state-appropriate timing guidance as session end approaches and arrives.

**Acceptance Criteria**
- The adjusted session end time is visible from the beginning of the active session.
- A sound-backed warning occurs 5 minutes before the adjusted end time.
- A sound-backed alert occurs at the adjusted end time.
- Customer guidance explicitly states whether shooting can continue or has ended.
- Updated timing behavior follows the adjusted end time rather than a generic slot rule.

**Sources**
- Product Scope / MVP Scope Clarifications
- Customer Journey / Timing guidance

### FR-007 Export-Waiting, Completion, and Handoff Guidance

The system can guide the customer through the end-of-session outcome after shooting ends.

**Acceptance Criteria**
- After shooting ends, the product enters one explicit post-end state: export-waiting, completed, or handoff guidance.
- In `Export Waiting`, shooting is disabled and the customer sees wait guidance while the end-of-session deliverable or handoff package is not yet ready.
- In `Completed`, the required end-of-session deliverable or approved next-step package is ready and the customer can leave the booth flow without additional booth-side processing.
- In handoff guidance, the customer sees the identified recipient or next location together with the approved next action.
- The customer sees the next action without technical diagnostics.
- If a session name is required for downstream handoff, the product displays the session name on the handoff surface.
- If the session cannot resolve normally, the product routes to bounded wait or call guidance.

**Sources**
- Executive Summary / Product Definition
- Customer Journey / Completion or handoff

### FR-008 Internal Preset Authoring and Approved Catalog Publication

Authorized internal users can create, tune, approve, and publish booth presets using detailed internal preset-authoring controls, including RapidRAW-derived or RapidRAW-equivalent controls, without exposing those controls to booth customers.

**Acceptance Criteria**
- Authorized users can create or tune presets with detailed internal controls not available to booth customers.
- Presets require an approval or publication step before appearing in the customer booth catalog.
- Booth customers see only approved published presets.
- Booth customers never receive access to internal preset-authoring controls through the customer flow, review flow, or completion flow.
- Preset publication changes support controlled rollout and rollback.

**Sources**
- Product Scope / MVP In Scope for Internal or Authorized Users
- Authorized Preset Manager Journey

### FR-009 Operational Safety and Recovery

The system can detect blocked states, protect customers from unsafe recovery steps, and provide operators with bounded diagnostics, recovery actions, and lifecycle visibility.

**Acceptance Criteria**
- Customer-facing failure states use plain-language wait or call guidance.
- Operators can view current session context, timing state, recent failure context, and approved recovery actions.
- Approved operator actions are limited to bounded recovery behavior.
- Lifecycle and intervention events are recorded for support, timing, and completion analysis.

**Sources**
- Operator Journey
- Success Criteria / support burden and operator first action time

## Non-Functional Requirements

### NFR-001 Customer Guidance Density and Simplicity

The system shall keep 100% of primary customer state screens within a copy budget of no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, session name, and local phone number, while exposing 0 internal diagnostic or preset-authoring terms on customer-visible screens, as measured by release copy audit.

**Acceptance Criteria**
- All primary customer states pass copy audit before release.
- Each primary customer state contains no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, session name, and local phone number.
- Customer-visible wording uses approved booth-state terminology only.
- No customer state includes raw technical, filesystem, internal authoring language, or direct editing-control labels.

### NFR-002 Cross-Branch Preset and Timing Consistency

The system shall keep 100% of active branches on the same approved customer preset catalog, customer-visible timing rules, and core booth journey states except approved local contact settings, as measured by branch rollout audit.

**Acceptance Criteria**
- Active branches use the same approved preset catalog and ordering.
- Active branches use the same customer-visible timing rules and warning behavior.
- Branch variance is limited to approved local settings such as contact information and approved operational toggles.

### NFR-003 Booth Responsiveness and Confidence Feedback

The system shall show the latest captured photo within 5 seconds for 95th-percentile successful captures and acknowledge primary customer actions within 1 second on approved Windows hardware, as measured by performance benchmarking and pilot logs.

**Acceptance Criteria**
- 95th-percentile successful captures show latest-photo confirmation within 5 seconds.
- Primary customer actions such as session start, preset selection, delete confirmation, and post-end state entry are acknowledged within 1 second.
- Performance is measured on approved branch hardware.

### NFR-004 Session Isolation and Privacy

The system shall expose 0 cross-session photo leaks across capture, review, deletion, and completion flows, as measured by privacy test cases, pilot operation, and defect review.

**Acceptance Criteria**
- Customers cannot access another customer's assets through review, deletion, or handoff flows.
- Stored customer identifiers are limited to approved minimum session-identifying data.
- Release privacy validation passes active-session and reopened-session isolation scenarios.

### NFR-005 Timing and Completion Reliability

The system shall deliver the scheduled 5-minute warning and exact-end alert within +/- 5 seconds for 99% of qualifying sessions and transition 90% or more of sessions to an explicit post-end state within 10 seconds of scheduled end time, as measured by lifecycle logs and pilot review.

**Acceptance Criteria**
- 99% of qualifying sessions receive the warning and exact-end alert within the allowed tolerance.
- 90% or more of sessions enter `Export Waiting`, `Completed`, or `Phone Required` within 10 seconds of scheduled end time.
- 90% or more of sessions resolve to `Completed` or `Phone Required` within 2 minutes of scheduled end time.

### NFR-006 Safe Local Packaging and Rollout

The system shall support staged branch rollout to explicitly selected branch sets and rollback of any promoted branch to the last approved build within one approved rollback action, while preserving approved local settings and active-session compatibility and enforcing 0 forced updates during active customer sessions, as measured by release controls and branch rollout audit.

**Acceptance Criteria**
- Each rollout targets an explicitly selected branch set rather than mandatory same-time deployment to every branch.
- 100% of rollout and rollback actions record the branch set, target build, approval timestamp, and operator identity in the rollout audit.
- Active customer sessions are never interrupted by forced update behavior.
- Any promoted branch can return to the last approved build in one approved rollback action while preserving approved local settings and active-session compatibility.

## Risks and Validation Gates

### Open Assumptions to Validate

| Assumption | Why It Matters | Validation Stage | Owner |
| --- | --- | --- | --- |
| Session-name-first start is sufficient for customer throughput and does not create unacceptable ambiguity | Prevents reintroducing more complex booth-start gating unless needed | PRD to UX handoff | PM + UX |
| The approved preset catalog is small enough to keep choice simple but broad enough for customer appeal | Prevents either choice overload or insufficient creative value | UX and pilot validation | PM + UX |
| Customers accept a preset-first booth experience without needing a direct editing step | Prevents reintroducing customer-facing editor complexity to satisfy perceived value gaps | UX and pilot validation | PM + UX |
| Adjusted end-time rules can be explained clearly enough that customers understand warning and end alerts | Prevents timing confusion and trust loss | UX and pilot validation | PM + UX |
| Internal preset authoring can maintain consistent look quality across branches without exposing incomplete presets | Prevents catalog drift and customer-facing inconsistency | Architecture and operational validation | PM + Architect |
| Approved branch hardware can sustain latest-photo feedback and timed session transitions within target budgets | Prevents shipping a concept that feels slow or unreliable in practice | Prototype and smoke validation | Architect + Dev |
| Completion and handoff guidance are understandable without customer editing | Prevents a hidden need to reintroduce customer-side editing | UX and pilot validation | PM + UX |

### Release Gates

- A customer can start a session using session name input only.
- A customer can choose one approved preset and reach a valid capture state without operator help.
- Successful captures show latest-photo confidence without exposing other session assets.
- The customer can review only current-session photos, delete within approved bounds, and change presets for future captures.
- The adjusted end time is visible from session start, and the 5-minute warning plus exact-end alert fire correctly.
- After session end, the product enters one explicit post-end state: export-waiting, completed, or phone-required.
- Booth customers never enter a direct photo-editing workflow and never see RapidRAW-derived or RapidRAW-equivalent detailed controls.
- Branch rollout controls can promote explicitly selected branch sets and roll back a promoted branch to the last approved build without interrupting an active customer session.
- No forced update interrupts an active customer session.

## Conclusion

Boothy MVP is not a customer full-editor product and not a customer-operated RapidRAW reproduction. It is a booth-first preset-driven photo product where the customer starts quickly, chooses from a small approved preset set, captures photos confidently, and finishes within a clearly timed session.

RapidRAW-derived or RapidRAW-equivalent capability remains an internal preset-authoring foundation rather than a booth-customer editing surface.

The product succeeds when customers can start, choose, capture, review, and finish without ambiguity, while operators remain bounded, internal teams maintain approved looks safely, and branches preserve rollout safety and operational consistency.
