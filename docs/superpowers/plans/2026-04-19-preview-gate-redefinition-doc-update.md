# Preview Gate Redefinition Doc Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redefine the official preview-track product gate to `preset-applied visible <= 3000ms`, downgrade `sameCaptureFullScreenVisibleMs` to a reference metric, and align related docs, runbooks, BMAD planning artifacts, and implementation records to that decision.

**Architecture:** Update the product baseline documents first, then align route-decision/runbook interpretation, then update BMAD planning and implementation artifacts so release judgment, story status, and hardware-validation language all read from one canonical gate. Preserve historical measurements, but rewrite how they are interpreted.

**Tech Stack:** Markdown documentation, YAML planning artifacts

---

### Task 1: Update Canonical Product Baseline Docs

**Files:**
- Modify: `docs/release-baseline.md`
- Modify: `docs/preview-architecture-history-and-agent-guide.md`

- [ ] **Step 1: Update release baseline gate wording**

Replace dual-gate wording with single-gate wording and explicitly demote first-visible metrics to reference/comparison status.

- [ ] **Step 2: Update architecture history guide**

Replace references to the official dual 3000ms gate with the single `preset-applied visible <= 3000ms` gate, and clarify that `sameCaptureFullScreenVisibleMs` remains only as a comparison metric.

- [ ] **Step 3: Verify no contradictory gate wording remains in these files**

Run: `rg -n "sameCaptureFullScreenVisibleMs <= 3000ms|originalVisibleToPresetAppliedVisibleMs <= 3000ms|dual hardware gate|dual 3000ms gate" docs/release-baseline.md docs/preview-architecture-history-and-agent-guide.md`
Expected: Only intentional reference-metric mentions remain; no statement says both metrics are the official gate.

### Task 2: Update Route-Decision and Runbook Interpretation

**Files:**
- Modify: `docs/runbooks/current-actual-lane-handoff-20260419.md`
- Modify: `docs/runbooks/current-preview-gpu-direction-20260419.md`
- Modify: `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`
- Modify: `docs/runbooks/preview-track-route-decision-20260418.md`

- [ ] **Step 1: Rewrite official gate references**

Change each runbook so the official release gate is only `originalVisibleToPresetAppliedVisibleMs <= 3000ms`.

- [ ] **Step 2: Clarify metric roles**

In the old-line baseline rerun and GPU direction docs, state that `sameCaptureFullScreenVisibleMs` is a reference/comparison metric and not the release sign-off gate.

- [ ] **Step 3: Reframe route decisions**

Make sure `1.30` is described as `preset-applied visible` No-Go evidence, `1.31` stays unopened, and `1.26` remains the next official reserve-path candidate.

- [ ] **Step 4: Verify runbook consistency**

Run: `rg -n "sameCaptureFullScreenVisibleMs <= 3000ms|originalVisibleToPresetAppliedVisibleMs <= 3000ms|No-Go|1.30|1.31|1.26" docs/runbooks -g "*.md"`
Expected: Official gate language consistently points to `originalVisibleToPresetAppliedVisibleMs <= 3000ms`; story references match the new interpretation.

### Task 3: Update BMAD Planning Artifacts

**Files:**
- Modify: `_bmad-output/planning-artifacts/prd.md`
- Modify: `_bmad-output/planning-artifacts/epics.md`
- Modify: `_bmad-output/planning-artifacts/architecture.md`

- [ ] **Step 1: Update PRD release-signoff language**

Rewrite NFR and metric sections so release sign-off depends only on `originalVisibleToPresetAppliedVisibleMs <= 3000ms`, while first-visible and same-capture timing stay comparison or product-feel metrics.

- [ ] **Step 2: Update epics language**

Adjust NFR and acceptance wording so stories no longer inherit a dual hardware gate.

- [ ] **Step 3: Update architecture framing**

Where architecture references release-speed judgment, make it point to the single preset-applied gate and treat first-visible as a supporting metric.

- [ ] **Step 4: Verify planning artifact consistency**

Run: `rg -n "sameCaptureFullScreenVisibleMs <= 3000ms|originalVisibleToPresetAppliedVisibleMs <= 3000ms|dual hardware gate|release sign-off|release judgment" _bmad-output/planning-artifacts -g "*.md"`
Expected: No planning artifact defines the official release gate as dual; supporting metrics remain clearly labeled.

### Task 4: Update Implementation Artifacts and Story Records

**Files:**
- Modify: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Modify: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Modify: `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
- Modify: `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
- Modify: `_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md`

- [ ] **Step 1: Update ledger and sprint status**

Change release-judgment text so it points only to `originalVisibleToPresetAppliedVisibleMs <= 3000ms`.

- [ ] **Step 2: Update story artifacts**

Rewrite each story note that currently references the dual gate so it instead:
- keeps historical measurements intact
- interprets them against the new single official gate
- states whether the story is release-proof, comparison-only, or blocked

- [ ] **Step 3: Verify implementation artifact consistency**

Run: `rg -n "sameCaptureFullScreenVisibleMs <= 3000ms|originalVisibleToPresetAppliedVisibleMs <= 3000ms|dual hardware gate|current release judgment" _bmad-output/implementation-artifacts -g "*.md" -g "*.yaml"`
Expected: No implementation artifact still claims both metrics are the current official gate.

### Task 5: Final Cross-Doc Verification

**Files:**
- Modify: `docs/superpowers/specs/2026-04-19-preview-gate-redefinition-design.md` only if needed for scope alignment

- [ ] **Step 1: Run global verification search**

Run: `rg -n "sameCaptureFullScreenVisibleMs <= 3000ms and originalVisibleToPresetAppliedVisibleMs <= 3000ms|sameCaptureFullScreenVisibleMs <= 3000ms`와 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`|dual 3000ms gate|dual hardware gate" docs _bmad-output -g "*.md" -g "*.yaml"`
Expected: No stale dual-gate canonical wording remains.

- [ ] **Step 2: Run focused confirmation search**

Run: `rg -n "preset-applied visible <= 3000ms|originalVisibleToPresetAppliedVisibleMs <= 3000ms|reference metric|comparison metric" docs _bmad-output -g "*.md" -g "*.yaml"`
Expected: Updated docs consistently describe the official gate and the downgraded role of `sameCaptureFullScreenVisibleMs`.

- [ ] **Step 3: Review changed files in git status**

Run: `git status --short docs _bmad-output`
Expected: Only intended documentation and artifact files are modified for this task.
