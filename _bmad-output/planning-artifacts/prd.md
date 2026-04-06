---
workflowType: 'prd'
workflow: 'edit'
classification:
  domain: 'general'
  projectType: 'desktop_app'
  complexity: 'high'
date: '2026-03-17'
status: 'draft-v1.2-darktable-foundation-alignment'
documentType: 'product-requirements-document'
inputDocuments:
  - '_bmad-output/planning-artifacts/architecture.md'
  - 'docs/recent-session-preview-architecture-update-input-2026-04-06.md'
  - 'refactoring/2026-03-15-boothy-darktable-agent-foundation.md'
stepsCompleted:
  - 'step-e-01-discovery'
  - 'step-e-02-review'
  - 'step-e-03-edit'
lastEdited: '2026-04-06'
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
  - date: '2026-03-16'
    changes: 'PRD realigned to the approved darktable foundation pivot and revised architecture: darktable preset artifact model, separate camera and render boundaries, booth-runtime-safe execution, and removal of remaining RapidRAW-era product language from the body'
  - date: '2026-03-16'
    changes: 'Validation-driven refinement pass: named policy references added, post-end completion taxonomy clarified, and operator recovery inventory bounded more explicitly'
  - date: '2026-03-16'
    changes: 'Implementation-readiness alignment: minimum policy baselines added for current-session deletion, session timing, and operator recovery'
  - date: '2026-03-17'
    changes: 'Approved epic-structure correction: FR-008 scope tightened to explicit draft-validation-approval-publication-rollback workflow semantics without reducing authorized internal scope'
  - date: '2026-03-20'
    changes: 'Booth-start identity realigned to name plus phone-last-four alias entry, sample-cut-based preset selection clarified, and customer-facing alias language separated from durable session identity'
  - date: '2026-03-20'
    changes: 'Validation-report follow-up: removed broken source references from frontmatter and source list, and normalized FR-004, FR-006, FR-007, and FR-009 to actor-first phrasing'
  - date: '2026-03-20'
    changes: 'Final cleanup pass: normalized PRD title to BMAD template convention and added a short executive digest for faster stakeholder review'
  - date: '2026-04-06'
    changes: 'Problem definition, KPI framing, FR-004, NFR-003, and validation assumptions updated from thumbnail speed toward latest preset-applied large preview replacement with explicit first-visible vs truthful close separation'
---

# Product Requirements Document - Boothy

This PRD defines product behavior, product boundaries, and required operational truth for Boothy. It does not decide implementation frameworks, adapter internals, or code-level reuse strategy.

**Executive Digest**

- Product shape: Boothy is a booth-first Windows desktop photo product with preset-driven capture, latest large preview replacement as the core post-capture value, current-session-only review, and clearly timed completion guidance.
- Customer boundary: customers start with a simple name-plus-phone-last-four booth alias, select one approved published preset, capture confidently, and never enter a direct editing workflow.
- Operational boundary: capture success, first-visible reassurance, preset-applied preview readiness, and final completion are separate truths that must be reported honestly.
- Internal control: authorized staff manage preset authoring, approval, publication, and rollback outside the booth runtime.
- Release focus: the MVP succeeds when branches deliver consistent presets, truthful latest-preview replacement, bounded operator recovery, and no cross-session privacy leaks.

## Source Documents

- [Architecture decision document](./architecture.md)
- [Recent session preview architecture update input](../../docs/recent-session-preview-architecture-update-input-2026-04-06.md)
- [Darktable foundation pivot brief](../../refactoring/2026-03-15-boothy-darktable-agent-foundation.md)

## Executive Summary

### Product Definition

Boothy is a booth-first Windows desktop photo product for booth customers. The customer enters a simple booth alias composed of a name and the last four digits of a phone number, chooses one approved published preset from representative sample cuts, captures photos confidently, reviews only the current session, and completes a clearly timed session through preview waiting, export waiting, completion, or handoff guidance. The booth customer does not enter a direct photo-editing workflow before or after capture.

Boothy's booth runtime consumes only approved published preset artifacts. Those artifacts are created and approved in an authorized internal workflow and define the preset identity, pinned render compatibility, and the booth-safe preview and final rendering behavior that customers later experience.

Camera truth and render truth are separate product responsibilities. A successful capture means the active session has safely persisted the new source photo. The primary post-capture customer value is that the latest large preview on the booth screen is replaced quickly by the preset-applied result. Rail thumbnails are supporting artifacts, not the main product truth. Preview readiness and final export readiness are later booth-safe outcomes that must be reported truthfully to the customer.

### Business Problem

Current self-photo booth experiences lose customer confidence when session start feels administrative, look choice is unclear, capture success is ambiguous, the latest large preview replacement feels slow or misleading, or end-of-session guidance changes too late.

At the same time, businesses need branch-consistent looks without exposing authoring complexity to customers, and operators need bounded tools when either capture or rendering cannot continue safely.

The product must solve both problems at once:

- fast booth entry
- bounded creative choice through approved published presets
- reliable capture truth and fast, truthful latest preset-applied preview replacement
- current-session-only review and cleanup
- visible and trustworthy timing guidance
- truthful post-end waiting, completion, and handoff guidance
- bounded operational recovery across both capture and render failures

### Primary Users

- Customer: the primary booth user who wants a quick start, one clear look choice, confident capture, and a clear finish
- Remote operator: the secondary runtime user who diagnoses blocked booth states and performs limited recovery actions
- Authorized preset manager: the internal user who authors, approves, publishes, and rolls back booth preset artifacts for future sessions
- Owner / brand operator: the business stakeholder who needs consistent product behavior, safe rollout, and measurable operational outcomes

### Product Thesis

Boothy's value is not booth automation alone and not end-user editor power. Its value is confident booth use without technical burden, powered by internally crafted looks that customers consume only as approved published results.

Customers start quickly, choose a look they understand, capture photos with truthful booth feedback, and see the latest large preview move from early reassurance to preset-applied replacement without ambiguity. Internal teams retain creative control through darktable-backed preset authoring and publication, while the booth runtime exposes only approved preset choices and booth-safe status.

### Differentiation

- Session-name-based booth start with minimal friction
- Small approved preset catalog built from published preset artifacts rather than live editing controls
- Truthful booth separation between capture success, first-visible reassurance, preset-applied preview readiness, and final completion
- Guided capture with latest large preview replacement as the main post-capture value and current-session-only confidence
- Current-session-only review, deletion, and forward-only preset changes
- Adjusted end time visible from the start of the session
- Sound-backed 5-minute warning and exact-end alert
- Separate customer, operator, and authorized preset publication capability boundaries
- Local-first operation, rollout safety, and branch-consistent preset behavior

### Core Product Modes

Boothy operates across two runtime surfaces and one authorized adjacent workflow.

**Mode A: Customer Booth Flow**
- booth alias entry using name plus phone-last-four
- preset selection
- readiness and capture
- preview waiting and current-session review
- timing guidance
- export waiting, completion, and handoff guidance

**Mode B: Operator Recovery Surface**
- current session visibility
- capture-state and render-state fault diagnosis
- bounded recovery actions
- lifecycle, queue, and intervention review

**Mode C: Authorized Preset Publication Workflow**
- preset authoring in an authorized internal workflow
- artifact approval and publication
- preset rollback and maintenance for future sessions

### Core Product State Model

The customer-facing experience should translate booth state into simple, action-oriented language.

- `Preparing`: the booth is starting the session or confirming whether capture can begin
- `Ready`: the booth can accept capture because the active session and capture boundary are in a valid state
- `Capturing`: the booth is actively creating a new current-session capture
- `Preview Waiting`: the booth has accepted a current-session capture and is waiting for the host-validated preset-applied latest-preview replacement
- `Review`: the customer can see current-session photos and approved cleanup actions
- `Warning`: the booth is inside the final 5-minute guidance window before the scheduled end time
- `Export Waiting`: shooting has ended and the booth is preparing the end-of-session deliverable or handoff package
- `Completed`: all booth-side required work is complete and the state resolves as either `Local Deliverable Ready` or `Handoff Ready`; handoff guidance is presentation inside the latter, not a separate lifecycle truth
- `Phone Required`: safe continuation is blocked beyond approved recovery bounds

## Success Criteria

### Primary Business Outcome

Within the first 60 days of pilot rollout, Boothy should let customers start quickly, choose an approved look, capture confidently, and complete a clearly timed booth session while reducing support burden compared with the previous booth flow, preserving branch-consistent preset results, and making the latest preset-applied large preview feel promptly trustworthy.

### KPI Table

| Metric | Baseline | Target | Measurement Method |
| --- | --- | --- | --- |
| Monthly phone / remote support incidents | Approx. 500 incidents per month across all branches | 250 or fewer incidents per month | Combined support and remote assistance logs |
| Self-start session success rate | Unknown | 85% or higher | Sessions that reach first successful current-session raw persistence without operator intervention |
| Preset selection completion rate | New metric | 95% or higher | Sessions that reach preset selection and choose a preset within 20 seconds without operator intervention |
| First-visible same-capture latency | Recent measured average about `3.0s - 3.5s`, best recent run `2959ms` | Measured separately on approved booth hardware so the team can improve early reassurance speed without treating it as the final success condition | Request-level seam logs and pilot timing review |
| Latest preset-applied preview replacement latency | Recent measured average about `6.4s - 8.5s`; worst recent first-cut reading `10403ms` | Latest preset-applied large preview replacement visible within 5 seconds for 95th-percentile successful captures after raw persistence | Lifecycle logs, per-session seam logs, and pilot timing review |
| Published preset reproducibility rate | New metric | 99% or higher | Same approved preset version produces expected preview and final output within approved variance across pilot branches |
| Adjusted end-time visibility correctness | New metric | 99% or higher | Sessions where the displayed end time matches the approved timing policy from session start |
| Warning and end alert reliability | New metric | 99% or higher | Qualifying sessions where the 5-minute warning and exact-end alert occur within +/- 5 seconds of scheduled time |
| Explicit post-end state entry rate | New metric | 90% or higher | Sessions that enter `Export Waiting`, `Completed`, or `Phone Required` within 10 seconds of scheduled end time |
| Cross-session asset leak incidents | Must be zero | 0 | Privacy test cases, pilot defect logs, and operator incident reports |
| Operator first action time after critical failure | Unknown | Average 3 minutes or less | Time from `Phone Required` entry to first operator action |

### Qualitative Outcomes

- Customers should feel they can start immediately without learning software.
- Customers should feel their main creative choice is clear and bounded.
- Customers should feel that choosing a preset is sufficient and that no manual editing step is required to complete the booth session.
- Customers should trust that the booth does not claim preview readiness or completion before those states are actually ready, even if an earlier same-capture image appears first.
- Customers should understand what happens at warning time, end time, preview waiting, export waiting, and completion without staff improvisation.
- Operators should feel informed and bounded, not responsible for guessing whether failure is in capture truth or render truth.
- Owners should feel the product is consistent, supportable, and marketable across branches.

### Qualitative Validation Checks

| Goal | Operational Signal | Measurement Method |
| --- | --- | --- |
| Session start and preset choice feel obvious | 80% or more of observed pilot users start a session and select a preset without asking what to do next | Pilot observation logs and moderated walkthroughs |
| State truth feels honest | 90% or more of observed pilot users can correctly state whether the booth is ready to capture, waiting for preview, or waiting for completion | Pilot observation logs and session-end interviews |
| Timing feels trustworthy | 90% or more of observed pilot users can correctly state whether shooting can continue after warning and exact-end alerts | Pilot observation logs and session-end interviews |
| Internal controls remain invisible to customers | 100% of customer pilot sessions expose no darktable, XMP, preset-authoring, or render-diagnostic language | UX acceptance review and pilot defect logs |
| Operators remain bounded | 90% or more of top exception drills are handled using approved recovery actions only | Operator drill logs and support retrospectives |

## Product Scope

### MVP In Scope for the Booth Customer

- One packaged Windows desktop booth application for the customer workflow
- Very simple session start centered on name plus phone-last-four booth alias entry
- Approved preset selection from a small bounded catalog of published preset artifacts
- Customer-safe readiness guidance and capture control
- Real camera readiness, trigger, and source-photo persistence for the required booth workflow
- Truthful preview waiting and latest large preview replacement confidence
- Current-session review only
- Deletion of current-session captures according to the current-session deletion policy
- In-session preset changes at any time, with changes applying from that moment forward without rewriting past captures
- Adjusted end-time visibility from session start
- Sound-backed 5-minute warning and exact-end alert
- Export waiting, completion, and handoff guidance
- Bounded customer-safe failure and escalation messaging

### MVP In Scope for Internal or Authorized Users

- darktable-backed preset authoring in an authorized adjacent workflow or approved runtime profile
- Approval and publication of immutable preset artifacts to the booth catalog
- Diagnostics, bounded recovery, and operational maintenance across capture and render boundaries
- Rollout and rollback controls for the app build and approved preset catalog

### MVP Scope Clarifications

- Customer creative choice is preset selection, not direct image editing.
- The booth customer never enters a direct photo editor or detailed adjustment surface during the session.
- The customer-facing booth alias is composed of a name plus the last four digits of a phone number.
- The product may store a separate durable session identifier internally as long as the customer-facing booth alias remains available for the active session and approved handoff guidance.
- Booth runtime sessions consume only approved published preset artifacts and never consume draft or unapproved presets.
- Capture success means the active session has safely persisted the new source photo.
- Preview readiness and final export readiness are separate booth-safe outcomes after capture and must not be conflated with capture success.
- Customer review is limited to the current session and the current-session deletion policy.
- Session timing may change based on coupon usage or other approved operational policy, but the adjusted end time must be visible from the beginning of the session and must follow the session timing policy.
- Post-end resolution must use the post-end completion taxonomy defined in this PRD. It does not require customer-side detailed editing.
- Authorized preset publication affects future sessions only and must not mutate active session data.
- The PRD defines product behavior and product boundaries. Architecture will decide exact packaging, process ownership, and code-level delivery details.

### Customer Preset Experience Baseline

The customer preset promise is locked to the following booth-facing behaviors. Downstream architecture, UX, and epic artifacts may refine these behaviors, but they may not silently weaken them without an approved PRD revision.

- The booth customer sees only approved published presets.
- The customer selects one active preset at a time from a bounded catalog of 1-6 options.
- Each preset has a descriptive name and representative preview tile or sample cut that represents the approved published preset artifact.
- Representative preset tiles or sample cuts help selection, but they do not replace the current session's own capture preview truth.
- The active preset remains visible enough for the customer to understand the current look.
- The customer can change the active preset at any point during the session. Each change applies from that moment forward and must not rewrite already captured assets.
- The booth customer never opens a direct editing workspace or sees darktable terms, XMP terms, style or library terms, module names, masking tools, curve tools, or low-level color controls.

### Published Preset Artifact Model

Each published booth preset is an approved artifact bundle, not just a display name.

- Each preset has a stable preset identity and published version.
- Each preset is tied to approved render compatibility for booth use.
- Each preset includes the customer-facing look representation needed for booth selection.
- Each preset includes the approved booth-safe latest-preview behavior and final render behavior required for downstream use.
- The latest large preview is the primary booth-facing artifact. Rail thumbnails are derived from or share that latest-preview artifact and do not define a separate preview truth.
- A same-capture first-visible image may appear earlier to reduce blank waiting, and that first-visible source may come from a fast preview, camera thumbnail, intermediate preview, or approved low-latency worker output.
- Preset-applied `previewReady` truth still comes only from the host-validated latest large preview replacement for that capture-bound preset version, even when execution is routed through an approved alternative local renderer adapter.
- Approved renderer routing must preserve same-slot replacement, canonical latest-preview path ownership, capture-bound preset versioning, and darktable-safe fallback behavior.
- Only approved published preset artifacts may appear in the customer booth catalog.

### Booth-Safe Runtime Boundary

The booth runtime promises a safe, bounded customer experience rather than live authoring power.

- The booth runtime owns customer guidance, session state, timing guidance, current-session review behavior, and latest large preview replacement truth.
- The booth runtime treats capture truth, first-visible reassurance, preset-applied preview truth, and final completion as separate responsibilities.
- The booth runtime keeps `Preview Waiting` until the host validates the preset-applied latest-preview replacement for the capture-bound preset version.
- The booth runtime never exposes mutable authoring state, shared library state, or raw render-engine internals to customers.
- The booth runtime may report waiting, ready, completed, or phone-required states, but it must not imply that preview or final output is ready before that state is true.
- The booth runtime must apply the named policy references and post-end completion taxonomy below rather than inventing branch-local customer truth.

### Named Policy References

The PRD uses the following named policy references to remove ambiguity from later UX, architecture, and story work. These minimum baselines are binding for implementation unless a later approved PRD revision replaces them.

- `Current-Session Deletion Policy` minimum baseline:
  - only captures and correlated booth artifacts belonging to the active session are eligible for deletion
  - deletion removes the selected current-session source capture, correlated preview and final derivatives, and the manifest references for that capture only
  - deletion is blocked while the target capture is in an active host-owned mutation state or after post-end completion has been finalized
- `Session Timing Policy` minimum baseline:
  - adjusted end time is fixed from approved timing inputs at session start and remains visible throughout the session
  - the warning occurs at T-5 minutes and exact-end behavior occurs at T=0 relative to the adjusted end time
  - new capture attempts are blocked after exact end unless an approved operator extension is applied and logged
- `Operator Recovery Policy` minimum baseline:
  - blocked states are normalized into capture, preview or render, or post-end recovery categories before they are exposed to operator tools
  - approved operator actions are limited to retry, approved boundary restart, approved time extension, or routing the session to `Phone Required`
  - customer-facing surfaces never expose raw diagnostics, direct device control, or internal recovery steps

### Post-End Completion Taxonomy

The booth must resolve post-end truth using one explicit state and, when applicable, one completion variant.

- `Export Waiting`: shooting has ended and booth-side required work is not yet complete.
- `Completed`: booth-side required work is complete and no additional booth-side processing is required before the customer follows the next step.
- `Completed / Local Deliverable Ready`: the required booth-side deliverable is ready locally.
- `Completed / Handoff Ready`: booth-side work is complete and the customer is guided to the approved recipient, next location, or next action.
- `Phone Required`: the booth cannot resolve normal completion within the Operator Recovery Policy.

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
- Customer-facing darktable UI, XMP editing, style management, or library management
- Using darktable style or library state as booth runtime truth
- Using darktable as the booth camera-control truth source
- Customer-side capture-to-editor continuity as the main product journey
- Authoring or publication actions that mutate active booth sessions directly
- Building a general-purpose cloud photo library product
- Cross-device session editing sync
- Unapproved branch-specific preset catalogs
- Forced updates during active customer sessions

### External Boundaries

- Reservation lookup, if later added, remains external and must not block the basic booth-start flow when the MVP input format is valid.
- Coupon or operational-policy lookups may use external operational data sources such as Google Sheets or equivalent approved systems.
- Remote support tools such as AnyDesk and Chrome remain operational tools, not customer-facing product features.
- Authorized preset authoring and approved render-engine management remain operational dependencies, not booth-customer features.

## User Journeys

### Primary Personas

#### Customer

- May have low technical confidence
- Wants a quick start, clear next actions, and one understandable creative choice
- Expects preset selection rather than manual adjustment controls or editor tooling
- Expects truthful feedback after each capture and clarity about time remaining

#### Remote Operator

- Intervenes only when the booth cannot proceed within bounded recovery rules
- Needs compressed truth, recent failure context, timing state, and approved recovery actions
- Needs enough visibility to distinguish capture-state problems from preview, render, or completion-state problems

#### Authorized Preset Manager

- Creates, validates, approves, publishes, and rolls back booth looks for future sessions
- Needs darktable-backed authoring power without exposing those controls in the booth
- Needs publication discipline, version control, and branch-safe rollout behavior

#### Owner and Brand Operator

- Needs a consistent, marketable booth experience across branches
- Needs measurable support reduction, visual consistency, and rollout safety

### Session Lifecycle Summary

| Lifecycle State | Entry Condition | Exit Condition |
| --- | --- | --- |
| Preparing | Session name input is accepted or session recovery begins | Ready or Phone Required |
| Ready | Session and booth are in a valid state for capture | Capturing or Phone Required |
| Capturing | One or more photos are being captured into the active session | Preview Waiting, Ready, Warning, or Phone Required |
| Preview Waiting | A current-session capture has been accepted and the canonical latest large preview is still waiting for host-validated preset-applied replacement | Review, Warning, Export Waiting, or Phone Required |
| Review | Only active-session photos are visible for confidence checks and approved cleanup | Capturing, Warning, Export Waiting, or Phone Required |
| Warning | The booth is within the final 5-minute timing window | Capturing, Preview Waiting, Review, Export Waiting, or Phone Required |
| Export Waiting | Scheduled shooting time has ended, shooting is disabled, and the booth is preparing the end-of-session deliverable or handoff package | Completed or Phone Required |
| Completed | Booth-side required work is complete and the state resolves as either `Local Deliverable Ready` or `Handoff Ready` before session close | Session close |
| Phone Required | Safe continuation is blocked beyond bounded recovery rules | Operator restores a prior valid state or escalates |

### Customer Journey

| Stage | Customer Goal | Product Responsibility | Success Condition |
| --- | --- | --- | --- |
| App launch | Understand what this booth is and how to begin | Present one clear booth entry without OS exploration | Customer starts without asking what app to use |
| Session start | Open a session quickly | Accept simple session-name input, create active session identity, and enter preparation | Customer reaches the preparing or ready flow without operator help |
| Preset selection | Choose a look with minimal hesitation | Show only approved published presets with clear names and previews | Customer chooses one preset confidently |
| Preset lock-in | Understand the editing boundary | Keep the booth experience focused on preset choice and capture rather than direct editing | Customer understands they are choosing a finished approved look, not entering an editor |
| Readiness | Know whether capture can begin | Show plain-language preparation, ready, waiting, or phone-required guidance | Customer understands whether to wait, capture, or call |
| Capture | Take one or more photos with confidence | Accept capture only in approved booth states, persist the new source photo to the current session, and preserve session isolation | Customer trusts that capture was accepted and stored |
| Preview waiting and confidence | Understand whether the latest large result is ready | Show current-session latest-preview feedback when ready, or explicit waiting guidance while preset-applied replacement is still in progress | Customer understands whether the booth is still preparing the current-session latest preview or is ready for review |
| Review and cleanup | Confirm current-session photos are usable | Show only current-session photos, allow approved deletion, and allow future-capture preset change | Customer sees only their own session assets and can manage them within bounds |
| Timing guidance | Understand how much time remains and what happens next | Show adjusted end time, 5-minute warning, and exact-end behavior clearly | Customer understands whether shooting can continue or has ended |
| Completion or handoff | Finish the session without ambiguity | Keep the customer in `Export Waiting` until booth-side required work is complete, then show `Completed` as either `Local Deliverable Ready` or `Handoff Ready` with the right next action | Customer knows whether to keep waiting, leave with a ready deliverable, move to the next location, or call |

### Customer Journey Reference Points

#### App Launch and Session Start

The booth presents one clear entry point, accepts a name-plus-last-four booth alias, and creates an active session without reservation-style gating.

#### Preset Selection and Lock-In

The customer chooses from a small catalog of approved published presets and understands that preset choice replaces direct editing.

#### Readiness and Capture

The booth shows plain-language readiness states, allows capture only in valid states, and treats persisted source capture as the first truthful success boundary.

#### Preview, Review, and Cleanup

The booth exposes only current-session assets, preserves preset visibility, and allows bounded current-session cleanup without cross-session access.

#### Timing, Completion, and Handoff

The booth shows adjusted end time early, warns before the end, and resolves post-end truth through explicit waiting, completion, or escalation states.

### Operator Journey

| Stage | Operator Goal | Product Responsibility | Success Condition |
| --- | --- | --- | --- |
| Normal operation | Stay out of the way | Let the customer complete the booth flow without intervention | No support action is needed |
| Exception detection | Understand the blocked state quickly | Show current session, timing state, recent failures, and bounded next actions | Operator identifies the situation within 60 seconds |
| Capture and render diagnosis | Distinguish where the booth is blocked | Show the diagnostics defined by the `Operator Recovery Policy` that separate capture-state issues from preview, render, and post-end issues | Operator identifies the relevant failure boundary without guessing |
| Recovery | Restore progress safely | Provide only the operator actions allowed by the `Operator Recovery Policy`, including retry, approved boundary restart, allowed time extension, or recovery routing | Customer returns to a valid state or receives clear escalation guidance |
| Audit | Understand recurring problems | Record lifecycle, queue, and intervention events for review | Support patterns can be categorized and tracked |

### Operator Journey Reference Points

#### Fault Diagnosis and Recovery

The operator workflow exists to identify the active blocked boundary quickly, use only approved actions, and preserve customer-safe guidance during recovery.

#### Audit and Operational Learning

Lifecycle, queue, and intervention records exist so recurring blocked states can be reviewed without expanding booth-floor controls.

### Approved Operator Recovery Inventory

The operator surface is intentionally bounded by the `Operator Recovery Policy`.

| Blocked-State Category | Allowed Diagnostics | Allowed Operator Actions | Not Allowed at PRD Level |
| --- | --- | --- | --- |
| Capture blocked | Current session identity, timing state, recent capture failure context, capture boundary status | Retry capture, restart approved capture boundary, route to `Phone Required` | Draft preset editing, direct asset reassignment, customer-state bypass |
| Preview or render blocked | Current session identity, queue status, recent render or preview failure context, post-capture asset status | Retry preview or render, restart approved render boundary, route to `Phone Required` | Silent completion, cross-session asset substitution, unbounded tooling |
| Timing or post-end blocked | Current session identity, timing state, completion-state category, downstream handoff status | Apply allowed time extension under the `Session Timing Policy`, retry completion check, route to `Phone Required` | Custom timing overrides outside policy, fake-complete behavior, direct mutation of active-session history |

### Authorized Preset Manager Journey

| Stage | Internal Goal | Product Responsibility | Success Condition |
| --- | --- | --- | --- |
| Authoring | Create or tune a booth look | Provide darktable-backed authoring in an authorized workflow that is unavailable to booth customers | The preset can be refined without exposing booth customers to authoring controls |
| Artifact packaging | Prepare an approved booth-consumable preset | Produce an immutable published preset artifact with stable identity, approved compatibility, and booth-safe preview and final behavior | The preset artifact is ready for approval review |
| Approval | Decide whether a look is ready for customers | Support approval review before booth publication | Only approved presets move forward |
| Publication | Make a preset available to future booth sessions | Publish the approved preset into the bounded customer-facing catalog for future sessions only | Booth customers see only approved published presets |
| Maintenance | Revise or roll back a preset safely | Support controlled updates and rollback without mutating active sessions directly | Branches remain consistent and recoverable |

### Preset Publication Reference Points

#### Authoring, Approval, and Publication

Authorized users create looks in a separate internal workflow, package them as immutable preset artifacts, and publish only approved versions into the future-session catalog.

#### Rollback and Future-Session Safety

Publication and rollback affect future sessions only and must not mutate active session state that is already in progress.

### Owner and Brand Journey

Owner and brand stakeholders are not direct on-screen users, but they need the product to:

- present a fast, premium, booth-first identity
- keep approved looks consistent across branches
- reduce support burden while preserving trust
- preserve rollout and rollback safety

### Owner and Brand Traceability

| Owner and Brand Need | Supporting Requirements |
| --- | --- |
| Market a fast booth-first experience instead of a complex editing workflow | Executive Summary, FR-001, FR-002, FR-006, NFR-001 |
| Keep approved looks consistent across branches | Product Scope, FR-008, NFR-002, NFR-006 |
| Reduce support burden while preserving safe operations | Success Criteria, FR-009, NFR-005, NFR-006 |
| Protect privacy and trust across customer sessions | Domain Requirements, FR-004, FR-005, NFR-004 |

### Traceability Summary

| Business Objective / Journey | Supporting FRs |
| --- | --- |
| Customers start quickly and choose a look with confidence | FR-001, FR-002, FR-003 |
| Customers trust that current-session capture and preview truth are honest | FR-003, FR-004, FR-005 |
| Customers understand time remaining and end-of-session behavior | FR-006, FR-007 |
| Internal teams control visual quality without exposing complexity to customers | FR-008 |
| Operators remain bounded and useful | FR-009 |

## Domain Requirements

Boothy is a general-domain desktop product, not a regulated healthcare, fintech, or govtech system. Even so, the product has hard operational requirements:

- Customers must never see another customer's raw, preview, final, reviewable, or handoff-related assets.
- The product must treat the customer-facing booth alias, selected preset version, source captures, preview renders, final renders, and exported outputs as session-scoped by default.
- Customer-facing surfaces must use plain-language status and next-action guidance rather than technical diagnostics or preset-authoring language.
- darktable terms, XMP terms, module names, OpenCL terms, style terms, library terms, and low-level image-tuning labels must never appear on booth customer surfaces.
- The displayed end time must be the authoritative customer timing truth for the active session.
- The product must provide a bounded `Phone Required` path when capture or rendering cannot continue safely.
- Booth runtime truth must come from session-scoped assets and host-normalized state, not from mutable authoring or library state.
- Only approved published preset artifacts may be applied in booth sessions.
- Internal preset authoring must never leak detailed controls into the booth customer surface.
- Operators must not gain unbounded product control through the support surface.
- The product must preserve current-session source captures and approved current-session actions in a way that supports trust, retry, and recovery.

## Product Decisions That Shape Delivery

This section records product decisions that later UX, architecture, epic, and story artifacts must preserve.

### Decision 1: Preset Choice Is the Customer's Only Creative Control

- Customers choose from approved published presets instead of entering a direct editing workflow.
- The bounded preset catalog is part of the product promise, not just a launch simplification.
- Later artifacts must preserve low-choice selection, active preset visibility, and the no-editor customer boundary.

### Decision 2: Capture Truth, First-Visible Reassurance, Truthful Preview, and Final Completion Stay Separate

- Capture success means the new source photo is safely persisted to the active session.
- First-visible reassurance may appear earlier, but it is not the same as preset-applied preview truth.
- Preview readiness and final completion are later booth-safe outcomes that must be communicated truthfully.
- The booth may show a same-capture fast preview before preset-applied preview readiness, but it must stay in truthful waiting language until the preset-applied latest large preview is actually ready at the same slot.
- Later artifacts must not collapse these states into one ambiguous success message.

### Decision 3: Internal Craft Stays Behind a Publication Boundary

- darktable-backed authoring exists for authorized internal users only.
- Booth runtime sessions consume immutable approved preset artifacts rather than draft authoring state.
- Later artifacts must preserve approval, publication, rollback, and future-session-only impact.

### Decision 4: Timing Guidance Is a Core Part of the Product Value

- Adjusted end time, warning behavior, exact-end behavior, and post-end resolution are customer-facing product commitments.
- Timing policy must remain visible, predictable, and policy-driven from session start through completion or escalation.
- Later artifacts must keep timing truth explicit instead of treating it as a background implementation detail.

## Project-Type Requirements

Boothy is classified as a `desktop_app`. The product therefore must satisfy the following platform-level conditions:

- The customer workflow runs as a Windows desktop booth application on approved branch hardware.
- The customer flow must remain usable without browser navigation, mobile-device assistance, or manual operating-system file browsing.
- The customer surface should assume a booth environment with clear, low-choice interactions rather than a workstation-style interface.
- The customer booth workflow must not expose a workstation-style editor mode or direct image-adjustment workspace.
- The product must preserve local-first operation for active sessions, including source capture persistence, preview waiting, review, timing guidance, and completion or handoff flows.
- The product must support separate customer and operator surfaces with different levels of truth and control.
- Authorized preset authoring capability must be restricted to approved internal workflows or profiles and must not appear in the booth customer flow.
- The product must preserve separate capture-truth and render-truth boundaries so booth readiness, preview readiness, and final completion are not conflated.
- Branch rollout must support staged deployment, rollback, and no forced update during an active customer session.
- Branch-level variance must remain tightly controlled so preset catalog, timing rules, and core customer journey remain consistent across locations.

## Functional Requirements

### FR-001 Simple Session Start

Users can start a booth session by entering a non-empty customer name and a four-digit phone suffix, which together form the customer-facing booth alias for the active session, as the only required booth-start input.

**Acceptance Criteria**
- The booth-start surface accepts exactly two required user-entered values: a non-empty customer name and a four-digit phone suffix.
- Invalid, empty, or malformed booth-alias input is shown before the customer proceeds.
- Valid input creates a customer-facing booth alias and an active session identity for the current booth session.
- The customer can continue into the preparing or ready flow without mandatory reservation verification or full phone-number entry.

**Sources**
- [Product Definition](#product-definition)
- [App Launch and Session Start](#app-launch-and-session-start)

### FR-002 Approved Published Preset Catalog Selection

Users can choose one approved published preset from a bounded catalog before shooting begins.

**Acceptance Criteria**
- The booth presents only 1-6 approved published presets to the customer.
- Each preset includes a customer-facing name and one preview image or standardized preview tile.
- Each booth-consumable preset has a stable published identity and version under the approved preset catalog.
- The customer can activate one preset at session start and can revisit or change it later during the same session.
- The activated preset becomes the active preset for subsequent captures until the customer changes it.
- No direct photo-editing workspace, darktable terminology, or detailed image-adjustment controls are exposed in preset selection.

**Sources**
- [Customer Preset Experience Baseline](#customer-preset-experience-baseline)
- [Published Preset Artifact Model](#published-preset-artifact-model)
- [Preset Selection and Lock-In](#preset-selection-and-lock-in)

### FR-003 Readiness Guidance and Valid-State Capture

Users can understand whether the booth is preparing, ready, preview-waiting, export-waiting, or phone-required and can capture only in approved valid states.

**Acceptance Criteria**
- The customer sees plain-language readiness guidance before first capture.
- The booth blocks capture when session or capture-state truth is not approved for capture.
- Customer-facing states avoid technical diagnostics and render-engine language.
- Blocked states tell the customer whether to wait or call rather than troubleshoot.
- The booth does not imply that preview readiness or final completion already exists when only capture readiness is true.

**Sources**
- [Readiness and Capture](#readiness-and-capture)
- [KPI Table](#kpi-table)
- [Booth-Safe Runtime Boundary](#booth-safe-runtime-boundary)

### FR-004 Current-Session Capture Persistence and Truthful Preview Confidence

Users can capture photos into the active session and receive truthful current-session confidence while the latest large preview progresses from early same-capture reassurance to preset-applied same-slot replacement.

**Acceptance Criteria**
- A successful capture means the new source photo is associated with and safely persisted under the active session before booth success feedback is shown.
- The booth can distinguish capture acceptance, first-visible reassurance, and preset-applied preview readiness in customer-safe language when preview preparation is still in progress.
- The latest large preview is the primary booth-facing artifact. If a same-capture pending preview appears before `previewReady`, it occupies the same canonical slot that the preset-applied booth-safe preview later replaces.
- `Preview Waiting` remains active until the host validates the preset-applied preview file for the capture-bound preset version.
- The latest customer-visible confirmation and any rail derivative include only current-session assets and preserve same-slot replacement correctness.
- The active preset name remains visible on the capture surface and preview confirmation surface while that preset is active.

**Sources**
- [Readiness and Capture](#readiness-and-capture)
- [Preview, Review, and Cleanup](#preview-review-and-cleanup)
- [Domain Requirements](#domain-requirements)

### FR-005 Current-Session Review, Deletion, and In-Session Preset Change

Users can review only current-session photos, delete current-session captures according to the `Current-Session Deletion Policy`, and change the active preset for future captures.

**Acceptance Criteria**
- The review surface exposes current-session assets only.
- The customer can delete current-session captures and their correlated current-session booth artifacts only when the `Current-Session Deletion Policy` allows it.
- The customer can change the active preset at any point during the session.
- Each preset change applies to subsequent captures without rewriting already captured assets.
- The product does not expose a direct photo-editing workspace, detailed editing controls, or authoring tools as part of review.

**Sources**
- [Preview, Review, and Cleanup](#preview-review-and-cleanup)
- [MVP In Scope for the Booth Customer](#mvp-in-scope-for-the-booth-customer)
- [Named Policy References](#named-policy-references)

### FR-006 Coupon-Adjusted Timing, Warning Alerts, and Exact-End Behavior

Users can rely on customer session timing that follows the `Session Timing Policy` and presents state-appropriate guidance as session end approaches and arrives.

**Acceptance Criteria**
- The adjusted session end time is visible from the beginning of the active session.
- A sound-backed warning occurs 5 minutes before the adjusted end time.
- A sound-backed alert occurs at the adjusted end time.
- Customer guidance explicitly states whether shooting can continue or has ended.
- Updated timing behavior follows the active `Session Timing Policy` rather than a generic slot rule.

**Sources**
- [MVP Scope Clarifications](#mvp-scope-clarifications)
- [Timing, Completion, and Handoff](#timing-completion-and-handoff)
- [Named Policy References](#named-policy-references)

### FR-007 Export Waiting, Final Readiness, and Handoff Guidance

Users can move through the end-of-session outcome after shooting ends with truthful final-readiness and handoff guidance.

**Acceptance Criteria**
- After shooting ends, the product enters one explicit post-end state from the post-end completion taxonomy: `Export Waiting`, `Completed`, or `Phone Required`.
- In `Export Waiting`, shooting is disabled and the customer sees wait guidance while the end-of-session deliverable or handoff package is not yet ready.
- In `Completed`, all booth-side required work is complete and the state resolves as either `Local Deliverable Ready` or `Handoff Ready`.
- In `Local Deliverable Ready`, the required booth-side deliverable is ready and the customer can leave the booth flow without additional booth-side processing.
- In `Handoff Ready`, the customer sees the identified recipient or next location together with the approved next action.
- The customer sees the next action without technical diagnostics.
- If the approved booth alias is required for downstream handoff, the product displays that booth alias on the handoff surface.
- If the session cannot resolve normally, the product routes to bounded wait or call guidance rather than false completion.

**Sources**
- [Product Definition](#product-definition)
- [Timing, Completion, and Handoff](#timing-completion-and-handoff)
- [Post-End Completion Taxonomy](#post-end-completion-taxonomy)

### FR-008 Authorized Preset Authoring, Approval, and Publication

Authorized preset managers can author draft booth preset versions, validate booth compatibility, approve immutable published versions, publish them to the future-session booth catalog, and roll back to a prior approved version without exposing authoring tools to booth customers.

**Acceptance Criteria**
- Authorized users can create or tune draft preset versions in an approved internal workflow that is unavailable to booth customers.
- Draft preset versions move through explicit lifecycle states `draft`, `validated`, `approved`, and `published`.
- Each approved booth preset is published as an immutable preset artifact with stable identity and approved render compatibility for booth use.
- Publication creates an immutable published artifact bundle plus catalog-facing metadata for future sessions only.
- Presets require approval before appearing in the customer booth catalog.
- Booth sessions consume only approved published preset artifacts.
- Publication changes and rollback apply to future sessions and do not directly mutate active session data or active session preset bindings.
- Booth customers never receive access to preset-authoring controls through the customer flow, review flow, or completion flow.

**Sources**
- [MVP In Scope for Internal or Authorized Users](#mvp-in-scope-for-internal-or-authorized-users)
- [Published Preset Artifact Model](#published-preset-artifact-model)
- [Authoring, Approval, and Publication](#authoring-approval-and-publication)

### FR-009 Operational Safety and Recovery

Operators can identify blocked states, protect customers from unsafe recovery steps, and use diagnostics, actions, and lifecycle visibility bounded by the `Operator Recovery Policy` across both capture and render boundaries.

**Acceptance Criteria**
- Customer-facing failure states use plain-language wait or call guidance.
- Operators can view current session context, timing state, recent failure context, and the allowed action set defined for the active blocked-state category by the `Operator Recovery Policy`.
- Operators can distinguish blocked capture states from blocked preview, render, or completion states using the diagnostics defined by the `Operator Recovery Policy`.
- Approved operator actions are limited to retry, approved boundary restart, allowed time extension under the `Session Timing Policy`, or recovery routing to `Phone Required`.
- Lifecycle, queue, and intervention events are recorded for support, timing, and completion analysis.

**Sources**
- [Fault Diagnosis and Recovery](#fault-diagnosis-and-recovery)
- [Approved Operator Recovery Inventory](#approved-operator-recovery-inventory)
- [KPI Table](#kpi-table)

## Non-Functional Requirements

### NFR-001 Customer Guidance Density and Simplicity

The system shall keep 100% of primary customer state screens within a copy budget of no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, booth alias, and local phone number, while exposing 0 internal diagnostic, authoring, or render-engine terms on customer-visible screens, as measured by release copy audit.

**Acceptance Criteria**
- All primary customer states pass copy audit before release.
- Each primary customer state contains no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, booth alias, and local phone number.
- Customer-visible wording uses approved booth-state terminology only.
- No customer state includes raw technical, filesystem, darktable, XMP, or direct editing-control labels.

**Sources**
- [Qualitative Validation Checks](#qualitative-validation-checks)
- [Booth-Safe Runtime Boundary](#booth-safe-runtime-boundary)
- [Project-Type Requirements](#project-type-requirements)

### NFR-002 Cross-Branch Preset and Timing Consistency

The system shall keep 100% of active branches on the same approved customer preset catalog, approved preset versions, customer-visible timing rules, and core booth journey states except approved local contact settings, as measured by branch rollout audit.

**Acceptance Criteria**
- Active branches use the same approved preset catalog, ordering, and published preset versions.
- Active branches use the same customer-visible timing rules and warning behavior.
- Branch variance is limited to approved local settings such as contact information and approved operational toggles.

**Sources**
- [Owner and Brand Traceability](#owner-and-brand-traceability)
- [Published Preset Artifact Model](#published-preset-artifact-model)
- [Project-Type Requirements](#project-type-requirements)

### NFR-003 Booth Responsiveness and Preview Readiness

The system shall acknowledge primary customer actions within 1 second, surface a same-capture first-visible current-session image as early as safely possible after source-photo persistence, and replace the latest large booth preview with the preset-applied result within 5 seconds for 95th-percentile successful captures on approved Windows hardware, as measured by performance benchmarking, request-level seam logs, and pilot logs.

**Acceptance Criteria**
- Primary customer actions such as session start, preset selection, delete confirmation, and post-end state entry are acknowledged within 1 second.
- `fastPreviewVisibleAtMs` is measured separately from `previewVisibleAtMs` so the product can improve the early customer-visible result without weakening preview truth or mistaking `first-visible` for success.
- When a same-capture fast preview is available, the booth may surface it at the canonical latest-preview slot before `previewReady`, but the customer state remains explicit `Preview Waiting` until the preset-applied replacement is actually ready.
- The first-visible source may come from an approved fast preview, camera thumbnail, intermediate preview, or low-latency worker output as long as same-capture correctness is preserved.
- Rail thumbnail speed is not a standalone launch metric; rail artifacts must derive from or share the same truthful close owner as the latest large preview.
- 95th-percentile successful captures show latest preset-applied large preview replacement within 5 seconds after source-photo persistence.
- If preset-applied preview confirmation is not yet ready, the booth remains in an explicit preview-waiting state rather than implying completion.
- Performance is measured on approved branch hardware with request-level seam evidence that distinguishes first-visible latency from truthful close latency, selected renderer route, same-slot replacement correctness, and preset-version binding.
- When more than one approved renderer route exists, the product compares route-specific close latency, fallback rate, and preset fidelity without weakening preview truth or customer-safe waiting behavior.

**Sources**
- [KPI Table](#kpi-table)
- [Qualitative Validation Checks](#qualitative-validation-checks)
- [Booth-Safe Runtime Boundary](#booth-safe-runtime-boundary)

### NFR-004 Session Isolation and Privacy

The system shall expose 0 cross-session asset leaks across source capture, preview, final output, review, deletion, and completion flows, as measured by privacy test cases, pilot operation, and defect review.

**Acceptance Criteria**
- Customers cannot access another customer's assets through review, deletion, or handoff flows.
- Stored customer identifiers are limited to approved minimum session-identifying data.
- Release privacy validation passes active-session and reopened-session isolation scenarios across source, preview, and final assets.

**Sources**
- [Domain Requirements](#domain-requirements)
- [Preview, Review, and Cleanup](#preview-review-and-cleanup)
- [Owner and Brand Traceability](#owner-and-brand-traceability)

### NFR-005 Timing, Post-End, and Render Reliability

The system shall deliver the scheduled 5-minute warning and exact-end alert within +/- 5 seconds for 99% of qualifying sessions, transition 90% or more of sessions to an explicit post-end state within 10 seconds of scheduled end time, and preserve valid current-session assets through render retries or failures, as measured by lifecycle logs and pilot review.

**Acceptance Criteria**
- 99% of qualifying sessions receive the warning and exact-end alert within the allowed tolerance.
- 90% or more of sessions enter `Export Waiting`, `Completed`, or `Phone Required` within 10 seconds of scheduled end time.
- 90% or more of sessions resolve to `Completed` or `Phone Required` within 2 minutes of scheduled end time.
- Render retry or failure does not delete or invalidate already persisted valid current-session source captures.

**Sources**
- [KPI Table](#kpi-table)
- [Named Policy References](#named-policy-references)
- [Post-End Completion Taxonomy](#post-end-completion-taxonomy)

### NFR-006 Safe Local Packaging, Rollout, and Version Pinning

The system shall support staged branch rollout to explicitly selected branch sets and rollback of any promoted branch to the last approved build and approved preset stack within one approved rollback action, while preserving approved local settings and active-session compatibility, enforcing 0 forced updates during active customer sessions, and maintaining approved render compatibility across the active preset catalog, as measured by release controls and branch rollout audit.

**Acceptance Criteria**
- Each rollout targets an explicitly selected branch set rather than mandatory same-time deployment to every branch.
- 100% of rollout and rollback actions record the branch set, target build, approved preset stack, approval timestamp, and operator identity in the rollout audit.
- Active customer sessions are never interrupted by forced update behavior.
- Any promoted branch can return to the last approved build and approved preset stack in one approved rollback action while preserving approved local settings and active-session compatibility.

**Sources**
- [MVP In Scope for Internal or Authorized Users](#mvp-in-scope-for-internal-or-authorized-users)
- [Project-Type Requirements](#project-type-requirements)
- [Rollback and Future-Session Safety](#rollback-and-future-session-safety)

## Risks and Validation Gates

### Open Assumptions to Validate

| Assumption | Why It Matters | Validation Stage | Owner |
| --- | --- | --- | --- |
| Name-plus-last-four booth alias entry is sufficient for customer throughput and does not create unacceptable ambiguity | Prevents reintroducing broader personal-data collection or a weaker booth-start identifier | PRD to UX handoff | PM + UX |
| The approved preset catalog is small enough to keep choice simple but broad enough for customer appeal | Prevents either choice overload or insufficient creative value | UX and pilot validation | PM + UX |
| Published preset artifacts can maintain consistent look quality across branches without drift | Prevents branch inconsistency and preset-truth erosion | Architecture and operational validation | PM + Architect |
| Approved branch hardware can sustain latest preset-applied preview replacement targets under the approved truthful close path | Prevents shipping a booth experience that feels slow or misleading in practice | Prototype and smoke validation | Architect + Dev |
| A lighter truthful renderer, preview-only artifact, or different close topology can reduce latest large preview replacement latency without weakening `Preview Waiting` truth or preset-version binding | Prevents the team from over-optimizing first-visible while the real customer wait remains slow | Architecture follow-up and hardware validation | Architect + Dev |
| Final render or handoff completion can resolve independently without confusing customers | Prevents a false need to reintroduce customer-side editing or false-complete states | UX and pilot validation | PM + UX |
| Operators can separate capture-state failure from render-state failure using bounded diagnostics only | Prevents unsafe or overly broad recovery behavior | Operator drill validation | PM + Ops |
| Preset publication and rollback can affect future sessions without mutating active sessions | Prevents live-session instability and unbounded operational risk | Architecture and rollout validation | PM + Architect |

### Release Gates

- A customer can start a session using name plus phone-last-four booth alias input only.
- A customer can choose one approved published preset and reach a valid capture state without operator help.
- Successful capture stores the new source photo under the current session before booth success feedback is shown.
- Booth `Ready` is recognized as release truth only when live camera/helper truth is confirmed through the host-owned runtime boundary with fresh status rather than browser fallback, stale session state, or incomplete helper signals.
- The booth reports preview readiness truthfully: `Preview Waiting` remains until the preset-applied latest large preview is ready at the canonical same slot, and explicit waiting guidance appears whenever that replacement is not yet complete.
- Latest-preview route evidence and same-slot replacement evidence can be recovered from one approved session package for hardware validation review.
- The customer can review only current-session photos, delete only as allowed by the `Current-Session Deletion Policy`, and change presets for future captures.
- The adjusted end time is visible from session start, and the 5-minute warning plus exact-end alert fire correctly.
- After session end, the product enters one explicit post-end state within the allowed timing budget, and `Completed` resolves only as `Local Deliverable Ready` or `Handoff Ready` after booth-side required work is actually complete.
- Render failure does not masquerade as capture failure and does not corrupt current-session truth.
- Any newly introduced renderer route supports booth-scoped canary, instant fallback to the approved darktable path, and no increase in false-ready or cross-session leakage incidents.
- Booth customers never enter a direct photo-editing workflow and never see darktable, XMP, module, style, or library terms.
- Publication and rollback can promote or revert approved preset artifacts for future sessions without mutating active sessions.
- Branch rollout controls can promote explicitly selected branch sets and roll back the app build and approved preset stack without interrupting an active customer session.
- No forced update interrupts an active customer session.
- The operator surface exposes only the diagnostics and recovery actions defined by the `Operator Recovery Policy`.

## Conclusion

Boothy MVP is not a customer full-editor product. It is a booth-first preset-driven photo product where the customer starts quickly, chooses from a small approved preset set, captures photos confidently, and finishes within a clearly timed session.

Its creative system is grounded in approved published preset artifacts prepared through an authorized darktable-backed workflow. Its booth runtime keeps capture truth, preview readiness, final completion, and operator recovery bounded and explicit instead of collapsing them into one ambiguous success state.

The product succeeds when customers can start, choose, capture, review, and finish without ambiguity, while operators remain bounded, internal teams maintain approved looks safely, and branches preserve rollout safety and operational consistency.
