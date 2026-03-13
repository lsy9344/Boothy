# Sprint Change Proposal: Align Planning Artifacts to the Approved Execution Baseline

**Date:** 2026-03-12
**Project:** Boothy
**Prepared By:** Codex
**Workflow:** Correct Course
**Mode:** Batch
**Change Type:** Planning Alignment Correction
**Scope Classification:** Moderate
**Change Trigger:** `implementation-readiness-report-2026-03-12.md`

**Primary Reference Documents:**
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\implementation-readiness-report-2026-03-12.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\epics.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\prd.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\architecture.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md`
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\sprint-status.yaml`
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\1-3-host-facing-camera-contract-and-session-schema-baseline.md`
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\1-4-minimal-lifecycle-and-intervention-logging-foundation.md`

## 1. Issue Summary

### Problem Statement

The current Boothy planning set is directionally correct at the PRD and architecture level, but the planning-to-execution chain is still misaligned.

The 2026-03-12 implementation readiness report concluded that the project is "NOT READY" because it detected:

- missing foundational implementation stories
- forward dependencies inside story sequencing
- misplaced operational stories inside customer-flow epics
- UX expectations that are not clearly separated between hard requirements and guidance

That finding is partially correct, but it is based too narrowly on the current `epics.md` representation.

The actual execution baseline already includes foundational implementation artifacts such as:

- starter and release baseline work
- host-facing contract and session schema baseline
- lifecycle and intervention logging foundation

As a result, the main problem is not missing platform work. The main problem is that the current planning artifacts do not fully reflect the already approved execution baseline and salvage policy.

### Context of Discovery

This issue was discovered during implementation readiness review on 2026-03-12 after the 2026-03-11 product-definition correction had already been approved.

The project is no longer suffering from the 2026-03-10 full-editor product-definition error. That issue was already corrected. The current issue is narrower:

- `epics.md` does not fully match the approved 7-epic execution map and salvage baseline
- the readiness report evaluated the planning set as if the execution baseline did not already contain foundational stories
- some UX expectations are still expressed as if they were mandatory product contracts when they are better treated as design guidance unless promoted explicitly

### Evidence

**Readiness finding:**
- `implementation-readiness-report-2026-03-12.md` marks the project as not ready and cites missing foundation stories and sequencing defects.

**Execution-control evidence:**
- `sprint-status.yaml` already records the approved 2026-03-11 course correction, the replacement 7-epic map, and the salvage-and-revalidate policy.

**Implementation-baseline evidence:**
- `1-2-baseline-signed-build-and-release-verification.md`
- `1-3-host-facing-camera-contract-and-session-schema-baseline.md`
- `1-4-minimal-lifecycle-and-intervention-logging-foundation.md`

These artifacts directly contradict the idea that no foundational implementation work exists.

## 2. Impact Analysis

### Epic Impact

**Impact Level:** Moderate

The epic titles themselves are largely correct and already aligned with the 2026-03-11 corrected product definition. The main defect is at the story structure and sequencing layer.

Key impacts:

- Epic 1 does not visibly express the full foundational story chain in `epics.md`, even though the implementation baseline already does.
- Epic 2 contains at least one branch-consistency story that aligns better with operational governance than with customer preset selection.
- Epic 3 includes logging or audit expectations that appear to depend on capabilities introduced later, even though the approved baseline already established the logging foundation.
- Epic 6 story wording creates the impression of forward dependencies between operator summary, recovery, and fault-classification work.

### Story Impact

**Impact Level:** Moderate

The primary story-level issues are:

1. **Foundation visibility gap**
   - `epics.md` does not cleanly expose the contract-first and logging-first foundations already represented in implementation artifacts.

2. **Story sequencing ambiguity**
   - Some acceptance criteria imply that a capability appears later than it actually should in the approved execution chain.

3. **Story grouping drift**
   - Operational consistency and branch audit work are mixed into customer-flow epics.

### Artifact Conflicts

**PRD conflict:** Low
- The PRD already reflects the corrected booth-first preset-driven product definition.
- No major PRD scope reset is required for this correction.

**Architecture conflict:** Low
- The architecture already prioritizes contract-first implementation and stable boundary foundations.
- It supports the proposed correction rather than conflicting with it.

**UX conflict:** Moderate
- Some UX expectations remain ambiguous between hard requirements and guidance:
  - live preview before capture
  - 0.5-second preset switching and preview target
  - preview-to-final fidelity as an absolute promise
  - always-visible progress tracker details

**Execution-control conflict:** Moderate
- `sprint-status.yaml` is directionally correct, but it should record this new planning-alignment correction explicitly.

### Technical Impact

**Impact Level:** Low to Moderate

This correction does not justify discarding valid implementation foundations.

The following remain valid:

- React + TypeScript + Tauri desktop foundation
- typed host-boundary contracts
- session-folder truth
- SQLite operational logging baseline
- salvage-and-revalidate policy for existing implementation stories

The unsafe part is continuing to treat `epics.md` and the 2026-03-12 readiness report as if they already expressed the real approved execution baseline.

### Timeline and Delivery Impact

**Immediate impact:** Pause any new work that depends on the current uncorrected `epics.md` sequencing.
**Planning effort:** Medium
**Code discard effort:** Low
**Risk if corrected now:** Low to Medium
**Risk if ignored:** Medium to High

## 3. Recommended Approach

### Option Evaluation

**Option 1: Direct Adjustment of Planning Artifacts**  
**Status:** Viable

Update `epics.md`, clarify UX requirement boundaries, and record the planning alignment in `sprint-status.yaml`, then rerun implementation readiness.

**Option 2: Roll Back Existing Implementation Foundations**  
**Status:** Not viable

This would discard valid work and does not address the real problem.

**Option 3: Another Full PRD / Architecture Reset**  
**Status:** Not viable

The current issue is not a product-definition reset. The PRD and architecture are already largely aligned.

### Selected Path

**Recommended Approach:** Direct Adjustment with Selective Salvage Formalization

### Rationale

This is the lowest-risk correction that addresses the real defect:

- preserve the approved booth-first product definition
- preserve the approved 7-epic execution model
- preserve existing foundational implementation artifacts
- repair only the planning and readiness chain that is out of sync with execution reality

## 4. Detailed Change Proposals

### 4.1 Epic and Story Baseline Proposals

**Artifact:** `epics.md`

#### Proposal EPIC-1

**Section:** `Epic 1 story structure`

**OLD**
- Epic 1 primarily starts with customer-flow stories and does not clearly expose the foundational implementation chain.

**NEW**
- Epic 1 should explicitly include the foundational story line before downstream customer-flow stories:
  - Initialize Booth Project from Approved Starter Template
  - Baseline Signed Build and Release Verification
  - Host-Facing Camera Contract and Session Schema Baseline
  - Minimal Lifecycle and Intervention Logging Foundation

**Rationale**
- This matches the real execution baseline and resolves the "missing foundation stories" readiness finding.

#### Proposal EPIC-2

**Section:** `Epic 2 story grouping`

**OLD**
- `Preset Selection Consistency Across Branches` sits inside the customer preset-selection epic.

**NEW**
- Move branch-consistency and audit-oriented work into Epic 7: Operational Visibility and Safe Branch Delivery.

**Rationale**
- This improves epic cohesion and removes an operations concern from a customer-value epic.

#### Proposal EPIC-3

**Section:** `Story acceptance criteria wording`

**OLD**
- Stories such as current-session review and deletion imply new audit capability inside the story itself.

**NEW**
- Story wording should explicitly rely on the pre-existing logging foundation rather than implying a later dependency.

**Rationale**
- This removes false forward dependencies.

#### Proposal EPIC-4

**Section:** `Epic 6 sequencing`

**OLD**
- Operator summary, recovery actions, and normalized fault classification are described in a way that can read as forward-dependent.

**NEW**
- Reword and, if necessary, reorder stories so normalized fault context and logging baseline are treated as available platform capabilities before operator workflows depend on them.

**Rationale**
- This resolves the readiness finding around operator-story independence.

### 4.2 Story Quality Proposals

**Artifact:** `epics.md`

#### Proposal STORY-1

**Section:** `Acceptance criteria language`

**OLD**
- Terms such as `audit event`, `safe fallback`, and `recent failure context` are used without a stable observable definition.

**NEW**
- Replace with explicit observable outputs such as:
  - persisted lifecycle/intervention log entry in SQLite
  - normalized fault category
  - customer-safe wait/call state
  - operator-visible recent event context

**Rationale**
- This makes story completion and readiness validation more objective.

#### Proposal STORY-2

**Section:** `Story classification`

**OLD**
- Enabling platform work and user-value work are mixed without explicit labeling.

**NEW**
- Mark foundation stories as enabling/platform stories and keep customer-flow stories focused on user-visible value.

**Rationale**
- This makes implementation sequencing and sprint planning easier to reason about.

### 4.3 UX Requirement Boundary Proposals

**Artifact:** `ux-design-specification.md`

#### Proposal UX-1

**Section:** `Responsiveness and experience promises`

**OLD**
- Live preview, 0.5-second responsiveness, and preview-to-final fidelity are expressed with near-contract language.

**NEW**
- Split UX statements into:
  - hard requirement candidates for later product elevation
  - design guidance / aspiration targets

**Hard requirement candidates**
- clear preset representation before capture
- clear latest-photo confidence after capture
- persistent timing/session context where trust depends on it

**Guidance / aspiration examples**
- live preview before capture
- 0.5-second preset switching target
- preview-to-final fidelity aspiration
- exact progress-tracker presentation details

**Rationale**
- This preserves UX intent without letting advisory design language block readiness.

### 4.4 Execution-Control Proposals

**Artifact:** `sprint-status.yaml`

#### Proposal STATUS-1

**Section:** `change_control`

**OLD**
- The 2026-03-11 course correction is recorded, but the 2026-03-12 planning-alignment issue is not.

**NEW**
- Add a new approved correction note that states:
  - `epics.md` must be reconciled with the approved replacement epic map and salvage baseline
  - implementation readiness must be rerun after epic/story correction
  - story revalidation remains salvage-and-revalidate, not full reset

**Rationale**
- The sprint-control artifact should reflect the current approved interpretation of execution readiness.

### 4.5 Readiness Report Handling Proposal

**Artifact:** `implementation-readiness-report-2026-03-12.md`

#### Proposal READY-1

**Section:** `Document handling`

**OLD**
- The report is the latest readiness output and currently says `NOT READY`.

**NEW**
- Treat it as a valid historical finding against the pre-correction `epics.md` baseline.
- Do not manually edit the report body.
- Regenerate implementation readiness after the planning artifacts are corrected.

**Rationale**
- Readiness reports should be regenerated from corrected source artifacts instead of being hand-adjusted.

## 5. Implementation Handoff

### Change Scope Classification

**Moderate**

This is primarily a planning alignment correction. It affects execution safety, but it does not require a new product-definition reset.

### Handoff Recipients and Responsibilities

**Product Owner / Scrum Master**
- regenerate or refresh `epics.md`
- ensure story sequencing and grouping align with the approved execution baseline
- rerun sprint planning after the corrected epic/story set is approved

**Solution Architect**
- confirm that story sequencing still reflects the contract-first and logging-first architecture priorities
- review any story wording that implies false runtime or dependency assumptions

**UX Designer**
- split UX contract candidates from design guidance
- confirm which UX aspirations should remain advisory

**Development Team**
- preserve existing foundational implementation stories as salvageable approved baseline work
- do not treat the 2026-03-12 readiness report as proof that foundational implementation work is absent

### Immediate Action Plan

1. Approve this Sprint Change Proposal.
2. Update `epics.md` to reflect the approved execution baseline and salvage structure.
3. Clarify UX requirement boundaries in `ux-design-specification.md`.
4. Record the planning alignment correction in `sprint-status.yaml`.
5. Rerun implementation readiness.
6. Resume implementation only against the corrected planning chain.

### Success Criteria

This correction is complete only when:

1. `epics.md` reflects the actual approved foundation and story sequencing.
2. operational and branch-governance work is grouped under the correct epic ownership.
3. UX guidance is separated from hard requirements where appropriate.
4. `sprint-status.yaml` records the planning alignment correction.
5. a fresh implementation readiness report is generated from the corrected source set.

## 6. Final Recommendation

Approve a **moderate planning alignment correction with selective salvage preservation**.

Do not reset the PRD or architecture again.

Do not discard existing foundational implementation stories.

The correct action is to align the planning artifacts and readiness gate to the already approved execution baseline, then regenerate readiness and continue implementation from that corrected plan.
