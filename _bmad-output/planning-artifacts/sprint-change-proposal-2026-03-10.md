# Sprint Change Proposal: Reset Planning for a Unified Camera + Full RapidRAW Editor Product

**Date:** 2026-03-10
**Project:** Boothy
**Prepared By:** John (PM Agent)
**Workflow:** Correct Course
**Mode:** Batch
**Change Type:** Product Definition Reset / Major Replanning
**Scope Classification:** Major
**Primary Reference Documents:**
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\sprint-change-proposal-2026-03-09.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\prd.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\architecture.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\epics.md`
- `C:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md`

## 1. Issue Summary

### Problem Statement

The current Boothy planning set no longer matches the intended product.

Current planning defines Boothy as a booth-first guided capture product whose primary endpoint is export readiness and handoff to the next room.

The approved new direction defines Boothy as a single packaged desktop product for the end user that must include:

- guided capture
- real camera integration
- capture-to-editor continuity
- the full RapidRAW editing surface
- in-app save and export after editing

This is not a normal scope extension. It is a product-definition correction.

### Trigger

The trigger is an authoritative product clarification from the product owner:

1. RapidRAW is not a selective donor. Its full end-user editor surface is in scope.
2. Capture and editor must coexist in the same packaged product.
3. The editor is for the final end user, not only for operators or later staff.

### Evidence

The evidence is documented in:

- `sprint-change-proposal-2026-03-09.md`
- the mismatch between current PRD journey and the newly approved product identity
- the absence of first-class editor epics in the current backlog

## 2. Impact Analysis

### PRD Impact

**Impact Level:** Major rewrite required

The current PRD still defines:

- booth-only product identity
- next-room handoff as the dominant post-capture outcome
- MVP scope without full in-app editor parity

The PRD must be rewritten around:

- one same-package product
- guided capture as the front half of the journey
- full editor availability for the end user
- in-app save/export as the final product completion path

### Epic Impact

**Impact Level:** Existing epic map must be replaced

Current epics over-index on:

- booth readiness
- preset/capture confidence
- timed handoff to the next room

Current backlog under-represents or omits:

- core editor workspace parity
- advanced editor feature parity
- capture-to-editor ingestion
- non-destructive edit persistence
- in-app export after editing

### Architecture Impact

**Impact Level:** Major rewrite required

Architecture foundations that remain valid:

- Tauri desktop packaging
- typed host boundary
- sidecar/camera contract
- session-folder durability
- SQLite operational logging
- branch config and release discipline

Architecture assumptions that must change:

- selective RapidRAW reuse framing
- handoff-centered product ending
- capture/export pipeline as the entire product center of gravity

The new architecture must define:

- one packaged app
- one continuous session model across capture and editing
- editor workspace composition
- edit persistence and history
- image ingestion from capture into the editor
- final save/export inside the same product

### UX Impact

**Impact Level:** Significant reinterpretation required

Current UX work remains useful for:

- customer-safe capture entry
- readiness communication
- capture reassurance
- operator diagnostic separation

Current UX work is incomplete for:

- capture-to-editor transition
- same-product continuity between capture and editing
- full editor information architecture
- multi-image editing workflow
- export completion after editing

### Implementation and Sprint Impact

**Impact Level:** Resequencing required

This change does **not** justify throwing away all implementation work.

Reusable implementation foundations already exist in the repository:

- React/Tauri desktop shell
- shared contracts and schema validation
- camera adapter and sidecar protocol baseline
- session manifest model
- timing and logging foundations

The correct reset target is therefore:

- **planning artifacts and backlog:** reset
- **implementation foundations:** salvage selectively

## 3. Recommended Approach

### Selected Path

**Recommended Approach:** Hybrid

This means:

1. Reset planning, not the entire repository.
2. Rewrite product-defining documents before continuing major feature implementation.
3. Preserve reusable technical foundations that remain aligned with the new product direction.

### Why Full Project Reset Is Not Recommended

A full repository reset would discard valid work that still supports the new product, especially:

- desktop runtime baseline
- native boundary contracts
- session durability model
- logging and release safety foundations

That would add cost without reducing the main problem, which is product-definition drift.

### Why Planning Reset Is Recommended

The main defect is not "wrong code only." The main defect is that the planning set still defines the wrong product.

Therefore the highest-leverage correction is:

1. retire or supersede the current PRD
2. rewrite architecture around unified capture + editor composition
3. replace the epic map
4. regenerate stories from the corrected product definition

## 4. Detailed Change Proposals

### 4.1 PRD Proposal

**Artifact:** `prd.md`

**OLD**
- Boothy is primarily a booth capture application.
- The journey ends in export readiness and next-room handoff.
- Editing is not the center of MVP.

**NEW**
- Boothy is a unified desktop camera and photo editing product.
- The journey is `check-in -> readiness -> capture -> editor -> save/export`.
- Full RapidRAW editor capability is assumed in scope unless explicitly excluded later.

**Required Edits**
- Rewrite Product Definition and Executive Summary
- Rewrite MVP scope and exclusions
- Replace handoff-first journey with capture-to-editor continuity
- Replace FR groups with unified-product and editor-capability requirement families

### 4.2 Epic Proposal

**Artifact:** `epics.md`

**OLD**
- Epic 1: Guided Session Start and Booth Readiness
- Epic 2: Confident Capture and Photo Review
- Epic 3: Timed Session Control and Result Handoff
- Epic 4: Exception Recovery and Operator Control
- Epic 5: Operational Visibility and Safe Branch Delivery
- Epic 6: Live Camera Helper Integration and Hardware Validation

**NEW**
- Epic 1: Unified Product Shell and Same-Package App Composition
- Epic 2: Guided Capture Flow and End-User Camera Readiness
- Epic 3: Real Camera Integration and Capture-to-Editor Ingestion
- Epic 4: Core RapidRAW Editor Workspace Parity
- Epic 5: Advanced RapidRAW Editing Feature Parity
- Epic 6: Export, Save, Output, and Final User Delivery Flow
- Epic 7: Operational Controls, Logging, Rollout, and Recovery

**Required Edits**
- Retire the current epic map as the primary backlog structure
- Reclassify reusable foundation stories under the new map
- Mark handoff-centered stories as superseded or rewritten

### 4.3 Architecture Proposal

**Artifact:** `architecture.md`

**OLD**
- RapidRAW is treated as a selective donor
- session and export models emphasize handoff artifacts
- the architecture optimizes for booth completion and handoff

**NEW**
- RapidRAW becomes the primary editor baseline
- capture and editing share one continuous session/project lifecycle
- editor persistence and export become first-class architecture concerns

**Required Edits**
- redefine application composition around capture mode plus editor mode
- add editor state, project persistence, history, and export concerns
- redefine image ingestion from capture into editor availability

### 4.4 UX Proposal

**Artifact:** `ux-design-specification.md`

**OLD**
- customer journey spine is `check-in -> choose -> capture -> review -> handoff`

**NEW**
- customer journey spine becomes `check-in -> capture -> editor -> save/export`

**Required Edits**
- define the transition from capture shell into editor workspace
- define same-product emotional continuity
- specify editor IA, navigation, editing affordances, and completion states

### 4.5 Implementation Policy Proposal

**OLD**
- continue implementing current backlog and add editor later

**NEW**
- pause major implementation against the old product definition
- perform a salvage audit on existing implementation
- continue only with reusable foundations

**Salvage Candidate Areas**
- typed DTOs and shared contracts
- session manifest and session paths
- camera adapter and sidecar protocol
- logging and branch configuration foundations
- Tauri packaging and release baseline

**Likely Rewrite Areas**
- customer flow screens whose terminal logic assumes handoff completion
- backlog-driven UX copy tied to booth-only completion
- any flow that positions review/handoff as the final customer value moment

## 5. Implementation Handoff

### Handoff Recipients and Responsibilities

**Product Manager**
- approve the corrected product identity
- own PRD rewrite direction
- define MVP boundary for the unified product

**Solution Architect**
- redesign same-package composition
- define session continuity from capture into editor
- define persistence, history, and export architecture for editor workflows

**UX Designer**
- redesign the end-user journey beyond capture
- specify capture-to-editor transition and full editor experience
- maintain visual and emotional continuity across both modes

**Product Owner / Scrum Master**
- retire the old epic map as the execution baseline
- regenerate backlog structure and story sequencing
- update sprint planning after new epics are approved

**Development Team**
- pause major work tied to booth-only completion assumptions
- preserve reusable platform foundations
- resume feature implementation only after new planning artifacts are ready

### Scope Classification

**Major**

Reason:
- product identity changes
- MVP definition changes
- PRD, architecture, UX, and epic map all require coordinated update

### Success Criteria for Handoff

This course correction is only considered complete when:

1. the planning reset is approved
2. a rewritten PRD exists
3. a rewritten architecture exists
4. a replacement epic map exists
5. implementation resumes only against the corrected backlog

## 6. Final Recommendation

Approve a **planning reset with selective implementation salvage**.

Do **not** reset the entire repository from scratch unless a later salvage audit proves the implementation is more tightly coupled to the old handoff model than currently expected.

The recommended sequence is:

1. approve this course correction
2. rewrite PRD
3. rewrite architecture
4. replace epic/story structure
5. run implementation readiness again
6. regenerate sprint planning
