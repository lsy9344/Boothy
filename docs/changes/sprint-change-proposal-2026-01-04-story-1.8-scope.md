# Sprint Change Proposal: Story 1.8 Scope Clarification (AC2–3)

Date: 2026-01-04  
Owner: Sarah (Product Owner)

## 1) Identified Issue Summary

**Trigger**: Story 1.8 QA gate FAIL due to scope mismatch.

**Observed reality (evidence)**
- Backend camera status command currently returns a placeholder “disconnected” state, with an explicit note that full camera connection logic is follow-up work.
- Admin UI includes connect/disconnect/retry as disabled placeholders with “Story 1.9” hints.

**Core problem**
- Story 1.8 Acceptance Criteria (AC2–3) were written as if **real camera session management** (connect/disconnect/reconnect + health monitoring) would be implemented.
- The implemented work is **scaffolding** (status model, gating, EDSDK validation, diagnostics UI), so AC2–3 cannot pass as originally stated.

## 2) Epic Impact Summary

- Epic 1 remains valid.
- Impact is localized to:
  - Epic 1 / Story 1.8 AC2–3 wording (rescope to scaffolding)
  - Epic 1 / Story 1.9 scope (explicitly include real camera session management + health monitoring)

## 3) Artifact Adjustment Needs

- `docs/stories/1.8.edsdk-path-validation-camera-connect-health.md` (scope + AC2–3 + task wording consistency)
- `docs/prd/epic-1-rapidtetherraw-customer-mode-mvp.md` (Story 1.8 AC2–3 + Story 1.9 AC addition)

No PRD (`docs/prd/*.md`) or Architecture changes required.

## 4) Path Forward Evaluation

### Option 1: Re-scope Story 1.8 (Recommended)
- Update Story 1.8 AC2–3 to explicitly define **scaffolding only**.
- Move/confirm real camera connectivity + health monitoring into Story 1.9.
- Pros: matches current implementation, avoids duplicate work, reduces risk/timeline impact.
- Cons: requires doc updates and QA re-gate on updated criteria.

### Option 2: Implement Full Camera Connectivity inside Story 1.8
- Pros: AC2–3 would pass without doc changes.
- Cons: significant scope increase (risk of 3–5 days+), likely duplicates Story 1.9 work, higher integration risk.

**Selected path**: Option 1 (Re-scope Story 1.8)

## 5) Specific Proposed Edits (Applied)

### A) Epic 1: Story 1.8 AC2–3 + IV3 clarification
File: `docs/prd/epic-1-rapidtetherraw-customer-mode-mvp.md`
- Update Story 1.8 AC2–3 to “camera status model/diagnostics/gating scaffolding” and explicitly defer real connect/disconnect/reconnect + health monitoring to Story 1.9.
- Adjust IV3 to refer to status/diagnostics/gating not freezing UI; real health loop deferred.

### B) Epic 1: Story 1.9 AC addition
File: `docs/prd/epic-1-rapidtetherraw-customer-mode-mvp.md`
- Add AC6 to Story 1.9: implement camera session (connect/disconnect/reconnect), basic background health monitoring, and event-driven status updates.

### C) Story 1.8: scope note + AC2–3 wording + task wording alignment
File: `docs/stories/1.8.edsdk-path-validation-camera-connect-health.md`
- Add explicit scope note: this story is scaffolding; real camera connectivity/health loop is Story 1.9.
- Update AC2–3 to match scaffolding.
- Update task wording to avoid claiming real connect/disconnect/health is already implemented.

## 6) High-Level Action Plan / Next Steps

- QA: re-run gate for Story 1.8 against updated AC2–3.
- SM/PO: ensure Story 1.9 story document (when drafted) includes the added AC6 scope (real camera session + health + event-driven status updates + Admin actions enablement).
- Dev: implement real camera session management + health monitoring + event updates in Story 1.9.

## 7) Success Criteria

- Story 1.8 QA gate passes with the updated, explicit scaffolding scope.
- Story 1.9 acceptance criteria clearly cover real camera connectivity + health monitoring.
- No regression in cross-platform builds/tests.

## Checklist Record (Change Navigation Checklist Sections 1–4)

- [x] Triggering story identified (Story 1.8)
- [x] Issue defined (AC2–3 scope mismatch)
- [x] Evidence gathered (implementation is scaffolding; real connect/health deferred)
- [x] Epic impact assessed (Story 1.8 and Story 1.9 only)
- [x] Artifact impact assessed (Story doc + Epic doc)
- [x] Options evaluated (Rescope vs implement now)
- [x] Recommended path selected (Rescope Story 1.8; move real connectivity to Story 1.9)

## PO Decision (Final)

- Decision date: 2026-01-04
- Confirmed: Story 1.8 remains “scaffolding” (validation + status model + diagnostics UI + gating).
- Confirmed: Story 1.9 owns real camera session/connectivity (connect/disconnect/reconnect), background health monitoring, event-driven status updates, and enabling Admin Diagnostics Connect/Disconnect/Retry actions.
