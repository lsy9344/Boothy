# Sprint Change Proposal: Realign Boothy to the Correct Booth-First Preset-Driven Product

**Date:** 2026-03-11  
**Project:** Boothy  
**Prepared By:** John (PM Agent)  
**Workflow:** Correct Course  
**Mode:** Batch  
**Change Type:** Product Definition Correction / Major Replanning  
**Scope Classification:** Major  
**Change Trigger:** Authoritative product clarification issued in `prd-rewrite-brief-2026-03-11.md`

**Primary Reference Documents:**
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\prd-rewrite-brief-2026-03-11.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\prd.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\architecture.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\epics.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\epics-legacy-2026-03-10.md`
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\sprint-status.yaml`

## 1. Issue Summary

### Problem Statement

The current Boothy planning set is built on the wrong product definition.

As of 2026-03-10, the active PRD and architecture define Boothy as a unified capture-to-full-editor product where the customer takes photos and then uses a full RapidRAW-derived in-app editor. That definition is no longer authoritative.

As of 2026-03-11, the authoritative clarification states that:

- the customer enters the booth and starts quickly
- the customer primarily enters a session name with minimal friction
- the customer selects one approved preset from a small bounded catalog
- the customer begins shooting immediately
- the customer does not use RapidRAW detailed editing controls
- RapidRAW-derived detailed controls exist for internal preset authoring, not for booth customers

This is not a minor requirement tweak. It changes the product promise, MVP boundary, customer journey, scope hierarchy, and downstream planning assumptions.

### Context of Discovery

The issue was not discovered as a narrow implementation defect in one story. It was discovered through product-owner clarification and formalized in `prd-rewrite-brief-2026-03-11.md`.

That means the root cause is planning drift, not just isolated code drift.

### Evidence

**Latest authoritative source:**
- `prd-rewrite-brief-2026-03-11.md` explicitly states that customer-facing full editing is out of scope and that RapidRAW is the internal preset-authoring foundation.

**Conflicting planning sources:**
- `prd.md` last edited on 2026-03-10 defines a customer-facing full in-app editor, editor parity, and capture-to-editor continuity as the core product value.
- `architecture.md` last completed on 2026-03-10 treats `editor-workspace` as a first-class customer product domain.
- `ux-design-specification.md` last completed on 2026-03-10 treats editor entry as the central continuity moment of the customer journey.
- `sprint-status.yaml` still reflects the 2026-03-10 approved reset for a full-editor product direction.

## 2. Impact Analysis

### Epic Impact

**Impact Level:** Major

The backlog is not uniformly wrong, but its authoritative chain is broken.

- `epics-legacy-2026-03-10.md` is no longer authoritative.
- `epics.md` already reflects corrected high-level epic headings, but it is incomplete and does not yet provide a regenerated story-level implementation baseline.
- `sprint-status.yaml` still tracks a different 2026-03-10 reset centered on a customer full-editor product.

The result is that the current epic layer is partially corrected in one document and still materially incorrect in the actual execution-control artifact.

### Story Impact

**Impact Level:** Mixed, but significant

Stories fall into three categories:

1. **Direct conflict with corrected product definition**
   - Example: `1-5-check-in-validation-and-session-identity-provisioning.md` currently assumes reservation name plus phone last four digits as the canonical booth-start contract. The corrected brief instead says the customer should start primarily by entering a session name with minimal friction.

2. **Preservable with dependency or wording changes**
   - Examples: preset selection, in-session preset change, capture confidence, current-session review, timing, warning, export-waiting, completion/handoff, operator recovery.
   - These mostly align with the corrected booth-first preset-driven direction and should be revalidated, not discarded.

3. **Execution-control mismatch**
   - `sprint-status.yaml` still uses the superseded 2026-03-10 replacement epic map for a capture-plus-full-editor product and therefore cannot remain the active control artifact after this correction is approved.

### Artifact Conflicts

**PRD conflict:** Major  
The current PRD overstates customer-facing editing scope and incorrectly positions editing continuity as the core product promise.

**Architecture conflict:** Major  
The current architecture allocates first-class product space to a customer `editor-workspace` domain and related continuity assumptions that no longer match the product.

**UX conflict:** Major  
The current UX specification is organized around a capture-to-editor emotional arc and editor-entry continuity that is no longer the customer experience spine.

**Validation/report conflict:** Moderate to Major  
The following reports are now historical, not current decision baselines:

- `implementation-readiness-report-2026-03-08.md`
- `validation-report-2026-03-10.md`

They were valid against the then-current planning set, but that planning set has now changed.

### Technical Impact

**Impact Level:** Moderate

This correction does not justify throwing away all implementation foundations.

The following foundations still appear valid under the corrected product:

- React + TypeScript + Tauri desktop foundation
- typed DTO and host-boundary patterns
- session-folder durability
- current-session privacy boundaries
- camera adapter and sidecar integration baseline
- lifecycle logging and branch-safe rollout discipline
- customer/operator surface separation

The following implementation assumptions are now unsafe if left uncorrected:

- any customer-facing workflow that assumes full editor access is the intended product baseline
- any execution control that continues prioritizing full-editor epic delivery
- any planning artifact that treats export after detailed editing as the main completion model

### Timeline and Delivery Impact

**Immediate impact:** Pause new story starts that depend on the old product definition.  
**Required planning impact:** Complete one planning reset cycle before resuming new implementation.  
**Estimated effort:** High planning effort, medium backlog refresh effort, low-to-medium code discard effort.  
**Risk if corrected now:** Medium.  
**Risk if ignored:** High.

## 3. Recommended Approach

### Option Evaluation

**Option 1: Direct Adjustment Only**  
**Status:** Not viable  
Reason: The source-of-truth artifacts are wrong. Adjusting stories without first correcting the PRD and architecture will recreate drift immediately.

**Option 2: Rollback Existing Implementation**  
**Status:** Not viable  
Reason: The main defect is not completed implementation. The main defect is planning based on the wrong product definition.

**Option 3: PRD MVP Review and Planning Realignment**  
**Status:** Viable  
Reason: The latest authoritative correction directly targets product definition, scope, and journey. Planning must be realigned from that source.

### Selected Path

**Recommended Approach:** Hybrid

This hybrid approach means:

1. Correct the PRD first.
2. Realign architecture and UX to the corrected product.
3. Regenerate or strictly refresh epics and stories from the corrected planning set.
4. Update sprint control only after the new planning baseline is approved.
5. Preserve aligned implementation foundations wherever possible.

### Rationale

This approach minimizes unnecessary technical churn while fixing the actual defect: planning based on a false product assumption.

It also protects the team from two failure modes:

- continuing implementation against stale product-defining documents
- overreacting and discarding valid technical foundations that still support the corrected product

### Recommended Sequence

1. `EP` or equivalent PRD rewrite using `prd-rewrite-brief-2026-03-11.md` as the authority
2. architecture realignment to the booth-first preset-driven concept
3. UX realignment to the corrected customer journey
4. epics/stories regeneration or strict refresh
5. implementation readiness rerun
6. sprint planning regeneration
7. sprint status update and controlled handoff back to implementation

## 4. Detailed Change Proposals

### 4.1 PRD Proposals

**Artifact:** `prd.md`

#### Proposal PRD-1

**Section:** `Executive Summary / Product Definition`

**OLD**
- Boothy is a unified Windows desktop photo product for end users that starts with guided camera capture and continues directly into a full-featured in-app photo editor.

**NEW**
- Boothy is a booth-first Windows desktop photo product where the customer starts quickly, enters a simple session name, selects one approved preset, captures photos confidently, reviews only the current session, and completes a clearly timed session through export-waiting, completion, or handoff guidance.
- RapidRAW-derived detailed controls are internal preset-authoring tools and are not part of the customer booth workflow.

**Justification**
- This is the core product-definition correction established on 2026-03-11.

#### Proposal PRD-2

**Section:** `Product Thesis`

**OLD**
- The value is continuity between guided capture and full editing inside one product.

**NEW**
- The value is continuity between fast booth start, approved look selection, confident capture, current-session review, and clear timed completion.

**Justification**
- The corrected product promise is confidence and timing trust, not full editor continuity.

#### Proposal PRD-3

**Section:** `Core Product Modes`

**OLD**
- `Mode A: Guided Capture Shell`
- `Mode B: Full Editing Workspace`

**NEW**
- `Customer Booth Flow`
- `Operator Recovery Surface`
- `Internal Preset Authoring Surface`

**Justification**
- Customer full-editor mode must be removed from the product core and replaced with an internal capability boundary.

#### Proposal PRD-4

**Section:** `Product Scope`

**OLD**
- customer-facing full editing workspace
- full RapidRAW end-user editor parity
- non-destructive edit persistence as customer scope
- in-app save/export after editing as the primary completion path

**NEW**
- very simple session setup
- approved preset catalog selection
- readiness guidance and valid-state capture
- latest-photo confidence feedback
- current-session review and deletion
- forward-only preset changes for future captures
- coupon-adjusted end-time visibility from session start
- sound-backed 5-minute warning and exact-end alert
- export-waiting, completion, and handoff guidance

**Explicitly out of scope for the booth customer**
- full end-user editor workspace
- direct RapidRAW detailed adjustments
- masking, curve, and detailed color tools on the customer surface

**Justification**
- MVP boundaries must be restated so downstream architecture and backlog do not silently drift back toward customer editing.

#### Proposal PRD-5

**Section:** `User Journeys / Session Lifecycle Summary`

**OLD**
- `check-in -> capture -> review -> enter editor -> edit -> save/export`

**NEW**
- `session name input -> preset selection -> capture -> review -> warning/end-time -> export-waiting -> completion/handoff`

**Justification**
- The corrected customer journey must become the new planning spine for all downstream artifacts.

#### Proposal PRD-6

**Section:** `Functional Requirements`

**OLD**
- `FR-004 Full In-App Editor Availability`
- `FR-005 Non-Destructive Edit Persistence`
- `FR-006 Session Image Editing Workflow`
- `FR-007 Save, Export, and Output Completion`
- `FR-008 Capture and Editor Continuity`

**NEW**
- `FR-004 Preset-Applied Capture Confidence and Current Result Visibility`
- `FR-005 Current-Session Review, Deletion, and Forward-Only Preset Change`
- `FR-006 Coupon-Adjusted Timing, Warning Alerts, and Exact-End Behavior`
- `FR-007 Export-Waiting, Completion, and Handoff Guidance`
- `FR-008 Internal Preset Authoring and Approved Catalog Publication`

**Justification**
- The FR set must reflect the corrected product behavior while preserving traceability discipline.

### 4.2 Architecture Proposals

**Artifact:** `architecture.md`

#### Proposal ARCH-1

**Section:** `Requirements Overview`

**OLD**
- Boothy is a unified session-based desktop photo product where capture, editing, output, and recovery share one continuous state model.

**NEW**
- Boothy is a booth-first preset-driven desktop product where session start, approved preset selection, capture, current-session review, timing truth, completion/handoff, and bounded recovery share one continuous state model.
- Internal preset authoring is a separate capability, not the customer workflow.

**Justification**
- The architecture must inherit the corrected product definition instead of the outdated PRD.

#### Proposal ARCH-2

**Section:** `Cross-Cutting Concerns`

**OLD**
- capture-to-editor continuity
- non-destructive asset management for editing
- editor readiness

**NEW**
- approved preset catalog publication
- preset selection truth and propagation to future captures
- current-session-only review and deletion boundaries
- coupon-adjusted timing truth and warning scheduling
- completion/handoff truth and escalation routing

**Justification**
- Architectural complexity is still high, but its center of gravity has changed.

#### Proposal ARCH-3

**Section:** `Frontend Architecture / Top-level app model`

**OLD**
- top-level surfaces such as `/customer`, `/editor`, `/operator`, and `/settings`

**NEW**
- top-level surfaces such as `/customer`, `/operator`, `/settings`, and optionally a privileged internal preset-authoring surface
- no customer `/editor` surface should define booth truth

**Justification**
- The route model must not preserve a customer editor surface that the product no longer promises.

#### Proposal ARCH-4

**Section:** `Component architecture / Project structure`

**OLD**
- `guided-capture`
- `editor-workspace`
- `export-pipeline`
- `operator-console`

**NEW**
- `customer-flow`
- `preset-catalog`
- `session-review`
- `timing-policy`
- `completion-handoff`
- `operator-console`
- `internal-preset-authoring`

**Justification**
- `editor-workspace` should no longer remain a first-class customer domain.

#### Proposal ARCH-5

**Section:** `Data Architecture`

**OLD**
- strong emphasis on captured originals, edit state, previews, and exported outputs under one customer session

**NEW**
- strong emphasis on session name, preset selection, captures, previews, export readiness, completion state, lifecycle events, and internal preset publication metadata
- no customer edit-state persistence as a core booth session promise

**Justification**
- Session data structures should match the actual customer experience and scope.

### 4.3 UI/UX Proposals

**Artifact:** `ux-design-specification.md`

#### Proposal UX-1

**Section:** `Project Vision`

**OLD**
- Boothy begins with a guided capture shell and continues directly into a full in-app editing workspace.

**NEW**
- Boothy is a booth-first flow that starts fast, offers one approved creative look choice, keeps capture confidence high, and guides the customer through a clearly timed session to completion or handoff.

**Justification**
- The UX vision must stop reinforcing the removed customer editor promise.

#### Proposal UX-2

**Section:** `Critical Success Moments`

**OLD**
- the third critical moment is when the product opens the editor and the customer understands the session has continued

**NEW**
- the third critical moment is when the customer understands that the selected preset is active, sees the latest current-session photo, and trusts the visible session end time and warning behavior

**Justification**
- The emotional peak of the journey has changed.

#### Proposal UX-3

**Section:** `Core User Experience / Experience Mechanics`

**OLD**
- review is the bridge into editing
- editing opens into bounded exploration

**NEW**
- review is the confidence bridge before continued shooting or session completion
- timing guidance, warning states, and completion/handoff states become the core post-capture mechanics

**Justification**
- The UX mechanics must align with the corrected journey rather than preserving editor-entry assumptions.

#### Proposal UX-4

**Section:** `Primary Screen and State Set`

**OLD**
- implicit screen set around capture shell, editor workspace, save/export, and editor loading

**NEW**
- `SessionNameStart`
- `PresetSelection`
- `CaptureReady`
- `CurrentSessionReview`
- `EndTimeWarning`
- `ExportWaiting`
- `CompletedOrHandoff`
- `PhoneRequired`
- separate internal preset-authoring surface for authorized users only

**Justification**
- The customer-facing screen map must be rewritten from the corrected booth journey.

#### Proposal UX-5

**Section:** `Responsive and Accessibility Validation`

**OLD**
- constrained editor layouts and customer editor workflows are part of the main validation matrix

**NEW**
- booth customer validation remains the primary matrix
- internal preset-authoring validation is tested separately as a privileged desktop-first workflow

**Justification**
- The UX test matrix should not continue allocating primary customer effort to a removed editor experience.

### 4.4 Story and Sprint-Control Proposals

**Artifacts:**
- implementation story files under `_bmad-output/implementation-artifacts`
- `sprint-status.yaml`

#### Proposal STORY-1

**Story:** `1-5-check-in-validation-and-session-identity-provisioning`

**Section:** `Story / Acceptance Criteria`

**OLD**
- customer enters reservation name plus phone last four digits as the canonical booth-start model

**NEW**
- customer enters a simple session name with minimal friction as the canonical booth-start model unless a later approved PRD revision reintroduces reservation-based identity

**Justification**
- This story directly conflicts with the 2026-03-11 authoritative brief and must be rewritten first.

#### Proposal STORY-2

**Story:** `2-1-preset-selection-entry-with-bounded-catalog`

**Section:** `Summary / Dependencies`

**OLD**
- initial preset selection depends on the reservation-validation-style check-in model

**NEW**
- initial preset selection follows the corrected minimal session-start flow and should not depend on reservation-verification semantics

**Justification**
- The core preset-selection behavior is still valid, but its upstream dependency assumptions must change.

#### Proposal STORY-3

**Story Group:** `2-2`, `2-3`, `2-4`, `2-5`, `3-1`, `3-2`, `3-3`, `4-1` through `4-4`, `5-1` through `5-3`, `6-1` through `6-5`

**Section:** `Status / Authority`

**OLD**
- treated as active implementation baseline under the existing execution control

**NEW**
- treat as salvage candidates pending strict revalidation against the rewritten PRD, architecture, and UX documents

**Justification**
- These stories are not automatically wrong, but they must not remain authoritative while upstream planning is being reset.

#### Proposal STORY-4

**Artifact:** `sprint-status.yaml`

**Section:** `change_control / replacement_epic_map`

**OLD**
- 2026-03-10 approved replacement epic map for a customer full-editor product

**NEW**
- 2026-03-11 approved replacement epic map for the corrected booth-first preset-driven product:
  - `epic-1: Minimal Session Start and Session Name Provisioning`
  - `epic-2: Approved Preset Catalog and Customer Preset Selection`
  - `epic-3: Guided Capture, Latest-Photo Confidence, and Current-Session Review`
  - `epic-4: Coupon-Adjusted Timing, Warning Alerts, and Completion/Handoff`
  - `epic-5: Internal Preset Authoring and Approved Catalog Publication`
  - `epic-6: Exception Recovery, Operator Control, and Diagnostics`
  - `epic-7: Operational Visibility and Safe Branch Delivery`

**Justification**
- The current sprint-control artifact still reflects the superseded 2026-03-10 product reset and must be replaced after approval.

#### Proposal STORY-5

**Artifact:** `sprint-status.yaml`

**Section:** `execution_guard`

**OLD**
- block new story work until PRD, architecture, and epics are rewritten for the full-editor product reset

**NEW**
- block new story work until PRD, architecture, UX, epics/stories, implementation readiness, and sprint planning are regenerated for the corrected 2026-03-11 booth-first preset-driven product

**Justification**
- The execution guard must be updated so the team does not resume work on a half-corrected planning chain.

### 4.5 Secondary Artifact Proposals

**Artifacts:**
- `implementation-readiness-report-2026-03-08.md`
- `validation-report-2026-03-10.md`

#### Proposal SEC-1

**Section:** `Document status`

**OLD**
- implicitly reusable as current readiness and validation evidence

**NEW**
- mark as historical-only evidence tied to the superseded planning set and regenerate after the corrected planning baseline is approved

**Justification**
- These reports validated the wrong product definition and should not be reused as current readiness gates.

## 5. Implementation Handoff

### Change Scope Classification

**Major**

This course correction requires PM and Architect involvement before development can proceed safely.

### Handoff Recipients and Responsibilities

**Product Manager**
- approve the corrected product identity
- own the PRD rewrite direction
- lock the corrected MVP boundary

**Solution Architect**
- remove customer full-editor assumptions from the architecture
- redefine the primary customer system around booth flow, preset selection, timing, review, and completion/handoff
- define internal preset-authoring boundaries separately

**UX Designer**
- replace the editor-centered customer experience spine
- redesign the customer journey around booth-first preset-driven use
- separate customer flow from internal preset-authoring UX

**Product Owner / Scrum Master**
- retire the old execution baseline
- regenerate epics and stories after planning approval
- refresh sprint planning and sprint status

**Development Team**
- do not start new story work against the superseded planning chain
- preserve aligned technical foundations
- support salvage audit and revalidation after the planning reset

### Immediate Action Plan

1. Approve this Sprint Change Proposal.
2. Rewrite the PRD using `prd-rewrite-brief-2026-03-11.md` as the authority.
3. Realign architecture.
4. Realign UX.
5. Regenerate epics and stories.
6. Rerun implementation readiness.
7. Regenerate sprint planning and update `sprint-status.yaml`.

### Success Criteria

This course correction is complete only when:

1. the corrected product definition is approved
2. the rewritten PRD is approved
3. architecture and UX are realigned
4. the epic and story baseline is regenerated or formally refreshed
5. `sprint-status.yaml` reflects the new approved plan
6. implementation resumes only against the corrected planning chain

## 6. Final Recommendation

Approve a **major planning reset with selective implementation salvage**.

Do not continue new implementation against the 2026-03-10 planning baseline.

Do not discard the entire implementation foundation.

The correct action is to replace the planning authority chain first, then resume implementation from the corrected booth-first preset-driven product definition.
