---
stepsCompleted:
  - step-01-document-discovery
  - step-02-prd-analysis
  - step-03-epic-coverage-validation
  - step-04-ux-alignment
  - step-05-epic-quality-review
  - step-06-final-assessment
filesIncluded:
  prd:
    - prd.md
  architecture:
    - architecture.md
  epics:
    - epics.md
  ux:
    - ux-design-specification.md
---

# Implementation Readiness Assessment Report

**Date:** 2026-03-12
**Project:** Boothy

## Document Discovery

### PRD Files Found

**Whole Documents:**
- prd.md (35,969 bytes, modified 2026-03-11 01:26:11 +09:00)

**Sharded Documents:**
- None

### Architecture Files Found

**Whole Documents:**
- architecture.md (45,973 bytes, modified 2026-03-11 22:03:48 +09:00)

**Sharded Documents:**
- None

### Epics Files Found

**Whole Documents:**
- epics.md (24,968 bytes, modified 2026-03-12 08:05:04 +09:00)

**Sharded Documents:**
- None

### UX Files Found

**Whole Documents:**
- ux-design-specification.md (22,008 bytes, modified 2026-03-11 22:53:31 +09:00)

**Sharded Documents:**
- None

### Issues Found

- No duplicate documents detected.
- No missing required documents detected.

### User Confirmation

- Proceed with the listed whole-document files for assessment.

## PRD Analysis

### Functional Requirements

FR1: Simple Session Start
Users can start a booth session by entering a non-empty session name as the only required booth-start input.
Acceptance Criteria:
- The booth-start surface accepts a non-empty session name as the only required user-entered field.
- Invalid or empty input is shown before the customer proceeds.
- Valid input creates an active session identity for the current booth session.
- The customer can continue into the preparing or ready flow without mandatory reservation verification or phone-number entry.

FR2: Approved Preset Catalog Selection
Users can choose one approved preset from a bounded catalog before shooting begins.
Acceptance Criteria:
- The booth presents only 1-6 approved presets to the customer.
- Each preset includes a customer-facing name and one preview image or standardized preview tile.
- The customer can activate one preset before capture begins.
- The activated preset becomes the active preset for subsequent captures until the customer changes it.
- No detailed image-adjustment controls are exposed in preset selection.

FR3: Readiness Guidance and Valid-State Capture
Users can understand whether the booth is preparing, ready, waiting, or phone-required and can capture only in approved valid states.
Acceptance Criteria:
- The customer sees plain-language readiness guidance before first capture.
- The booth blocks capture when session or device state is not approved for capture.
- Customer-facing states avoid technical diagnostics.
- Blocked states tell the customer whether to wait or call rather than troubleshoot.

FR4: Capture Persistence and Latest-Photo Confidence
The system can persist captured photos into the active session and show the latest current-session result as confidence feedback.
Acceptance Criteria:
- A successful capture is associated with the active session.
- The latest captured photo becomes visible to the customer as confirmation.
- Displayed capture confirmation includes only current-session assets.
- The active preset name remains visible on the capture surface and latest-photo confirmation surface while that preset is active.

FR5: Current-Session Review, Deletion, and Future-Capture Preset Change
Users can review only current-session photos, delete unwanted current-session photos within approved bounds, and change the active preset for future captures.
Acceptance Criteria:
- The review surface exposes current-session photos only.
- The customer can delete approved current-session photos.
- The customer can change the active preset during the session.
- Preset changes affect future captures only unless a later approved PRD revision changes that rule.
- The product does not expose detailed editing controls as part of review.

FR6: Coupon-Adjusted Timing, Warning Alerts, and Exact-End Behavior
The system can manage customer session timing using approved rules and present state-appropriate timing guidance as session end approaches and arrives.
Acceptance Criteria:
- The adjusted session end time is visible from the beginning of the active session.
- A sound-backed warning occurs 5 minutes before the adjusted end time.
- A sound-backed alert occurs at the adjusted end time.
- Customer guidance explicitly states whether shooting can continue or has ended.
- Updated timing behavior follows the adjusted end time rather than a generic slot rule.

FR7: Export-Waiting, Completion, and Handoff Guidance
The system can guide the customer through the end-of-session outcome after shooting ends.
Acceptance Criteria:
- After shooting ends, the product enters one explicit post-end state: export-waiting, completed, or handoff guidance.
- The customer sees the next action without technical diagnostics.
- If a session name is required for downstream handoff, the product displays the session name on the handoff surface.
- If the session cannot resolve normally, the product routes to bounded wait or call guidance.

FR8: Internal Preset Authoring and Approved Catalog Publication
Authorized internal users can create, tune, approve, and publish booth presets using detailed internal preset-authoring controls without exposing those controls to booth customers.
Acceptance Criteria:
- Authorized users can create or tune presets with detailed internal controls not available to booth customers.
- Presets require an approval or publication step before appearing in the customer booth catalog.
- Booth customers see only approved published presets.
- Preset publication changes support controlled rollout and rollback.

FR9: Operational Safety and Recovery
The system can detect blocked states, protect customers from unsafe recovery steps, and provide operators with bounded diagnostics, recovery actions, and lifecycle visibility.
Acceptance Criteria:
- Customer-facing failure states use plain-language wait or call guidance.
- Operators can view current session context, timing state, recent failure context, and approved recovery actions.
- Approved operator actions are limited to bounded recovery behavior.
- Lifecycle and intervention events are recorded for support, timing, and completion analysis.

Total FRs: 9

### Non-Functional Requirements

NFR1: Customer Guidance Density and Simplicity
The system shall keep 100% of primary customer state screens within a copy budget of no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, session name, and local phone number, while exposing 0 internal diagnostic or preset-authoring terms on customer-visible screens, as measured by release copy audit.

NFR2: Cross-Branch Preset and Timing Consistency
The system shall keep 100% of active branches on the same approved customer preset catalog, customer-visible timing rules, and core booth journey states except approved local contact settings, as measured by branch rollout audit.

NFR3: Booth Responsiveness and Confidence Feedback
The system shall show the latest captured photo within 5 seconds for 95th-percentile successful captures and acknowledge primary customer actions within 1 second on approved Windows hardware, as measured by performance benchmarking and pilot logs.

NFR4: Session Isolation and Privacy
The system shall expose 0 cross-session photo leaks across capture, review, deletion, and completion flows, as measured by privacy test cases, pilot operation, and defect review.

NFR5: Timing and Completion Reliability
The system shall deliver the scheduled 5-minute warning and exact-end alert within +/- 5 seconds for 99% of qualifying sessions and transition 90% or more of sessions to an explicit post-end state within 10 seconds of scheduled end time, as measured by lifecycle logs and pilot review.

NFR6: Safe Local Packaging and Rollout
The system shall support staged branch rollout, rollback, and zero forced updates during active customer sessions, as measured by release controls and branch rollout audit.

Total NFRs: 6

### Additional Requirements

- Domain constraints require strict session isolation, customer-safe language, bounded operator control, and no leakage of internal preset-authoring controls into the booth surface.
- Desktop app constraints require Windows desktop packaging, local-first operation, staged rollout, rollback, and no forced updates during active sessions.
- Product scope explicitly excludes a customer full editor, detailed RapidRAW controls on the booth surface, branch-specific preset catalogs, cross-device editing sync, and forced updates during active sessions.
- External boundaries allow future reservation lookup and coupon policy sync, but they must not block the booth-start flow when MVP inputs are valid.
- Release gates require session-name-only start, bounded preset selection, safe capture confirmation, current-session-only review/delete, visible adjusted end time, correct warning/end alerts, explicit post-end state, no customer exposure to detailed controls, and no forced session interruption.
- Open assumptions remain around session-name-first throughput, preset catalog size, timing comprehension, preset-authoring quality consistency, branch hardware performance, and handoff clarity without customer editing.

### PRD Completeness Assessment

The PRD is structurally complete for readiness validation: it defines product scope, user journeys, 9 numbered FRs, 6 numbered NFRs, lifecycle states, release gates, and traceability. Remaining ambiguity is concentrated in areas that later artifacts must sharpen: exact bounded operator recovery actions, exact timing-policy inputs, handoff/export deliverables, data retention rules for session assets, and security/authorization specifics for internal preset-authoring access.

## Epic Coverage Validation

### Epic FR Coverage Extracted

FR1: Epic 1 - Minimal Session Start and Session Name Provisioning
FR2: Epic 2 - Approved Preset Catalog and Customer Preset Selection
FR3: Epic 3 - Guided Capture, Latest-Photo Confidence, and Current-Session Review
FR4: Epic 3 - Guided Capture, Latest-Photo Confidence, and Current-Session Review
FR5: Epic 3 - Guided Capture, Latest-Photo Confidence, and Current-Session Review
FR6: Epic 4 - Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff
FR7: Epic 4 - Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff
FR8: Epic 5 - Internal Preset Authoring and Approved Catalog Publication
FR9: Epic 6 - Exception Recovery, Operator Control, and Diagnostics

Total FRs in epics: 9

### Coverage Matrix

| FR Number | PRD Requirement | Epic Coverage | Status |
| --------- | --------------- | ------------- | ------ |
| FR1 | Users can start a booth session by entering a non-empty session name as the only required booth-start input. | Epic 1 | ✓ Covered |
| FR2 | Users can choose one approved preset from a bounded catalog before shooting begins. | Epic 2 | ✓ Covered |
| FR3 | Users can understand whether the booth is preparing, ready, waiting, or phone-required and can capture only in approved valid states. | Epic 3 | ✓ Covered |
| FR4 | The system can persist captured photos into the active session and show the latest current-session result as confidence feedback. | Epic 3 | ✓ Covered |
| FR5 | Users can review only current-session photos, delete unwanted current-session photos within approved bounds, and change the active preset for future captures. | Epic 3 | ✓ Covered |
| FR6 | The system can manage customer session timing using approved rules and present state-appropriate timing guidance as session end approaches and arrives. | Epic 4 | ✓ Covered |
| FR7 | The system can guide the customer through the end-of-session outcome after shooting ends. | Epic 4 | ✓ Covered |
| FR8 | Authorized internal users can create, tune, approve, and publish booth presets using detailed internal preset-authoring controls without exposing those controls to booth customers. | Epic 5 | ✓ Covered |
| FR9 | The system can detect blocked states, protect customers from unsafe recovery steps, and provide operators with bounded diagnostics, recovery actions, and lifecycle visibility. | Epic 6 | ✓ Covered |

### Missing Requirements

- No missing FR coverage identified.
- No extra FR references were found in epics outside the PRD FR list.

### Coverage Statistics

- Total PRD FRs: 9
- FRs covered in epics: 9
- Coverage percentage: 100%

## UX Alignment Assessment

### UX Document Status

Found: ux-design-specification.md

### Alignment Issues

- The UX spec assumes real-time preset preview before capture and a "filter-first capture" mental model, but the PRD only requires preset previews or equivalent representation plus post-capture latest-photo confidence. Live preview is not explicitly locked as a product requirement.
- The UX spec sets a 0.5-second target for preset switching and preview generation, while the PRD and architecture only lock broader budgets such as 1-second primary action acknowledgement and 5-second latest-photo confirmation.
- The UX spec requires preview-to-final look consistency at 100%, but the PRD does not define an explicit preview fidelity requirement and the architecture does not elevate this to a contract-level constraint.
- The UX spec expects visible progress guidance such as a progress tracker and persistent session header details, but these are not fully expressed as mandatory PRD requirements.

### Warnings

- UX-specific visual system decisions such as Brutal Core styling, exact token values, and broader motion/sound identity are present in UX guidance, but they are not yet formalized as must-pass product requirements.
- Architecture does acknowledge touch-friendly layouts, responsiveness, accessibility, and customer-safe copy, so the alignment is directionally strong; the remaining gap is that some UX expectations are still advisory rather than contractually binding.

## Epic Quality Review

### Critical Violations

- Story 6.1 (Operator Summary and Session Context Visibility) depends on later stories in the same epic to be independently complete. "Recent failure context" and stable fault meaning are only properly established once Story 6.3 (fault classification) and Story 6.4 (lifecycle/intervention logging) exist.
- Story 6.2 (Bounded Operator Recovery Actions) requires intervention-event recording in its acceptance criteria, but that recording capability is only explicitly introduced in Story 6.4. This is a forward dependency inside the epic.
- Story 3.3 (Current-Session Review and Deletion) requires an "audit record" update immediately after deletion, but the audit/logging capability is defined later in Epic 6/Epic 7 rather than being available inside Epic 3. As sequenced, the story is not independently completable.
- Story 2.3 (Preset Selection Consistency Across Branches) depends on branch-level baseline comparison and audit behavior that align more naturally with Epic 7 (Operational Visibility and Safe Branch Delivery). In Epic 2 it introduces an operational dependency earlier than the supporting capability appears.

### Major Issues

- The architecture explicitly says the first implementation priority is freezing contract surfaces (`session manifest`, `preset bundle schema`, `error envelope`, `sidecar protocol`, `runtime profile/capability model`), but no dedicated foundational story exists before feature stories start.
- For a greenfield desktop product, there is no early CI/CD or build-pipeline story even though architecture and release baseline require GitHub Actions and Windows release validation paths. Build/release safety is pushed too far downstream.
- Story 2.3 is mis-grouped from a user-value perspective. The epic is customer preset selection, but this story is primarily an operator/operations consistency audit story and weakens epic cohesion.

### Minor Concerns

- Story 1.1 is a justified starter-template story because architecture requires it, but it still reads as an enabling technical setup story rather than direct user value. It should be clearly marked as foundational work rather than treated like a normal customer-facing story.
- Most stories use good Given/When/Then structure, but several acceptance criteria rely on terms like "audit event", "safe fallback", and "recent failure context" without defining the exact observable artifact or contract to verify.

## Summary and Recommendations

### Overall Readiness Status

NOT READY

### Critical Issues Requiring Immediate Action

- Remove forward dependencies inside the epic/story sequence, especially around operator logging and audit behavior.
- Re-scope or relocate operational consistency stories that currently sit inside customer-flow epics.
- Add missing foundational implementation stories for contracts and build/release safety before feature delivery begins.

### Recommended Next Steps

1. Rework the epics/stories so that Story 6.1, Story 6.2, Story 3.3, and Story 2.3 no longer depend on capabilities introduced later in the plan.
2. Add explicit foundation stories for `session manifest`, `preset bundle schema`, `error envelope`, `sidecar protocol`, `runtime profile/capability model`, and early CI/CD/release validation.
3. Reconcile UX-only expectations such as live preview, 0.5-second responsiveness, preview-to-final fidelity, and persistent progress guidance with the PRD and architecture so they are either promoted to hard requirements or downgraded to guidance.

### Final Note

This assessment identified 15 issues across 2 categories (UX alignment and epic quality). Address the critical issues before proceeding to implementation. The artifacts are close enough to improve directly, but they are not yet in a safe implementation-ready state.

Assessed by: Codex
Date: 2026-03-12
