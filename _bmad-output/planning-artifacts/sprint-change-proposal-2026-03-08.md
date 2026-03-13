# Sprint Change Proposal: Live Camera Integration Replan

**Date:** 2026-03-08  
**Project:** Boothy  
**Prepared By:** Codex  
**Workflow:** Correct Course  
**Mode:** Batch  
**Triggering Story:** Story 1.6 - Customer-Safe Preparation and Camera Readiness State

## 1. Issue Summary

### Problem Statement

During live booth validation, the customer flow behaved correctly through check-in and preparation-state rendering, but the booth did not advance from preparation when a real camera was physically connected. The current implementation supports customer-safe readiness UX and host-side readiness translation, but it does not yet include a real hardware-backed camera helper implementation that can detect a usable connected camera and deliver live readiness truth into the host boundary.

### Discovery Context

The issue was discovered during manual validation of the post-check-in preparation flow while Story 1.6 was in progress. The UI and state model behaved as planned up to the blocked preparation screen, but readiness was still effectively limited to the mocked helper milestone established earlier in the backlog.

### Trigger Classification

- Technical limitation discovered during implementation
- Misunderstanding of original requirements and milestone boundaries

### Evidence

- The PRD still treats direct camera-readiness detection as an assumption to validate rather than an explicit MVP implementation dependency.
- The epics document decomposes the host-only mocked milestone but does not provide a dedicated live camera integration epic or executable story set.
- The architecture document correctly assumes later replacement of the mock with a real bundled helper, but backlog sequencing does not make that dependency actionable.
- Story 1.6 already limits itself to customer-safe readiness UX and approved contract consumption; it does not define deep sidecar implementation as part of its scope.
- The repository contains `sidecar/mock/mock-camera-sidecar.mjs` and protocol examples, but no implemented `sidecar/canon-helper/*` helper code.
- `sprint-status.yaml` currently shows downstream work as active or ready without explicitly capturing the missing live camera dependency.

## 2. Impact Analysis

### Epic Impact

#### Epic 1: Guided Session Start and Booth Readiness

Epic 1 remains valid, but only as a mocked or contract-backed readiness milestone. It cannot continue to imply final live booth readiness.

Required change:

- Keep Story 1.6 scoped to customer-safe readiness UX and host-contract consumption
- Clarify that Epic 1 proves mocked or helper-agnostic readiness flow, not final real hardware acceptance

#### Epic 2: Confident Capture and Photo Review

Epic 2 can continue for customer UI and host-contract work, but real booth acceptance is blocked until live helper integration delivers actual readiness and capture truth.

#### Epic 3: Timed Session Control and Result Handoff

Epic 3 can continue for timing and export-state logic, but real booth acceptance remains blocked until real readiness and real capture artifacts are available.

#### Epic 4: Exception Recovery and Operator Control

Epic 4 depends heavily on real reconnect, fault classification, helper restart, and degraded hardware behavior. Mock-only readiness is not enough to validate this epic in booth conditions.

#### Missing Epic Identified

The backlog is missing a dedicated epic for live hardware integration.

**Required new epic:** `Epic 6: Live Camera Helper Integration and Hardware Validation`

### Artifact Conflict and Impact

#### PRD

The PRD is directionally correct, but one key statement is now too weak. Live camera readiness can no longer remain only an assumption; it is an MVP implementation dependency and release gate.

#### Architecture

The architecture remains correct. The issue is not architectural direction but the absence of an explicit backlog stream and validation gate for replacing the mock with the real helper.

#### Epics

`epics.md` under-specifies live integration. It currently leaves a major execution gap between mocked readiness and real hardware-backed booth behavior.

#### Story 1.6

Story 1.6 itself is mostly aligned. Its current dev notes already say not to broaden scope into deep sidecar implementation. The needed change is stronger scope clarification so nobody reads it as full live camera delivery.

#### UX Specification

No major UX redesign is required. Customer-safe readiness messaging, calm waiting patterns, and operator/customer surface separation remain valid.

#### Sprint Status

`sprint-status.yaml` creates a misleading readiness signal because Epics 2 through 4 appear to be progressing without an explicit live camera dependency being tracked.

### Technical Impact

- Real camera readiness cannot currently be proven on actual booth hardware
- The current Tauri-side readiness path can only validate mocked or helper-agnostic behavior
- Real capture-to-session-folder handoff remains unproven
- Reconnect, helper restart, and real fault routing cannot be validated against actual hardware
- Downstream capture, export, and operator stories risk passing mock-based checks while still failing in a real booth

## 3. Recommended Approach

### Selected Path

**Selected approach: Hybrid**

- Directly adjust existing artifacts to clarify mocked-versus-live scope boundaries
- Add a new epic dedicated to live camera helper integration and hardware validation
- Re-sequence downstream booth-validation expectations so Epics 2 through 4 do not imply real booth acceptance before Epic 6 completes

### Option Evaluation

#### Option 1: Direct Adjustment Only

Not viable as a standalone fix. Wording changes alone do not create the missing workstream.

- Effort: Medium
- Risk: High
- Role in final recommendation: Required, but only as part of a hybrid solution

#### Option 2: Rollback Recent Work

Not viable. Rolling back Story 1.3, Story 1.4, Story 1.5, or Story 1.6 groundwork would remove valid contract, logging, and UX foundations without solving the real integration gap.

- Effort: High
- Risk: High

#### Option 3: PRD MVP Reduction

Viable only as documentation clarification, not as the main answer. Lowering MVP to a mock-only demo would contradict the product's stated booth-operational goal.

- Effort: Medium
- Risk: Medium for documentation correction, High if used to avoid real integration

### Recommendation Rationale

The architecture is still the right one. The failure is in planning decomposition and sprint sequencing. The least disruptive and most accurate response is to preserve the current architecture, preserve valid groundwork, and add the missing live camera integration track explicitly.

### Estimated Impact

- Effort: High
- Risk: High
- Timeline impact: Material
- Scope classification: Major

## 4. Detailed Change Proposals

### 4.1 Epics Document Update

**Artifact:** `epics.md`

#### Change A: Clarify Epic 1 milestone boundary

**Section:** `Epic 1: Guided Session Start and Booth Readiness`

**OLD**

```md
Customers can enter the booth, complete check-in, receive customer-safe preparation and camera-readiness guidance, and reach a valid session-ready state without operator help.
```

**NEW**

```md
Customers can enter the booth, complete check-in, receive customer-safe preparation and camera-readiness guidance, and reach a valid contract-backed session-ready state without operator help. Epic 1 validates mocked or helper-agnostic readiness flow and customer-safe readiness UX; final real hardware booth acceptance is delivered in Epic 6.
```

**Rationale:** Epic 1 should remain valid, but it must stop implying that customer-safe readiness UX equals real hardware readiness delivery.

#### Change B: Add missing live integration epic

**Section:** `Epic List`

**OLD**

```md
### Epic 5: Operational Visibility and Safe Branch Delivery
Operators and owners can rely on lifecycle logging, KPI-ready operational data, and safe branch rollout or rollback controls to run the product consistently across stores.
```

**NEW**

```md
### Epic 5: Operational Visibility and Safe Branch Delivery
Operators and owners can rely on lifecycle logging, KPI-ready operational data, and safe branch rollout or rollback controls to run the product consistently across stores.

### Epic 6: Live Camera Helper Integration and Hardware Validation
The booth can detect a real connected camera, transition from preparation into true ready-to-shoot state, write real capture artifacts into the current session folder, and validate reconnect, recovery, and booth acceptance behavior against actual hardware.
```

**Suggested Story Set**

- `6-1-real-camera-helper-execution-and-packaging-baseline`
- `6-2-live-camera-readiness-detection-and-state-normalization`
- `6-3-real-capture-to-session-folder-handoff`
- `6-4-helper-reconnect-restart-and-fault-routing`
- `6-5-booth-hardware-smoke-validation-and-release-gates`

**Rationale:** The missing work is too large and too cross-cutting to stay implicit inside Story 1.6 or scattered across later epics.

#### Change C: Add dependency note to Epics 2 through 4

**OLD**

```md
Epic 2, Epic 3, and Epic 4 contain no explicit note that real booth acceptance depends on live camera integration.
```

**NEW**

```md
Add an explicit note under Epic 2, Epic 3, and Epic 4 that real booth acceptance is gated by Epic 6 completion, even when contract or UI work can continue earlier.
```

**Rationale:** This preserves useful downstream work while preventing false booth-readiness claims.

### 4.2 Story 1.6 Scope Clarification

**Artifact:** `1-6-customer-safe-preparation-and-camera-readiness-state.md`

**Section:** `Summary`

**OLD**

```md
Implement the blocked preparation and customer-safe readiness flow that appears immediately after session provisioning. This story must translate host-normalized camera truth into calm customer guidance, move cleanly into a ready-to-shoot state when readiness is confirmed, and escalate to a phone-required state when preparation exceeds the approved threshold without ever exposing SDK, filesystem, or sidecar diagnostics on the customer surface.
```

**NEW**

```md
Implement the blocked preparation and customer-safe readiness flow that appears immediately after session provisioning. This story consumes the approved host-normalized readiness contract and delivers the mocked or helper-agnostic readiness milestone for customer-safe UX. Real hardware-backed readiness detection, bundled helper execution, and booth validation are delivered in Epic 6. This story must translate host-normalized camera truth into calm customer guidance, move cleanly into a ready-to-shoot state when contract-backed readiness is confirmed, and escalate to a phone-required state when preparation exceeds the approved threshold without ever exposing SDK, filesystem, or sidecar diagnostics on the customer surface.
```

**Rationale:** Story 1.6 is already close to the correct scope. This change prevents it from being misread as the place where live helper integration should also land.

### 4.3 PRD Update

**Artifact:** `prd.md`

#### Change A: Promote live readiness from assumption to MVP dependency

**Section:** `Open Assumptions to Validate`

**OLD**

```md
- 카메라 준비 상태를 시스템이 직접 감지할 수 있는지 검증해야 한다.
```

**NEW**

```md
- 승인된 Windows booth image에서 대상 카메라 연결을 시스템이 직접 감지하고, 준비 상태에서 촬영 가능 상태로 전이시키는 기능은 MVP 필수 구현 범위다. 이 항목은 단순 가정 검증이 아니라 구현 및 릴리즈 게이트로 추적해야 한다.
```

**Rationale:** The current discovery shows that live readiness is not a minor assumption. It is a core MVP-enabling dependency.

#### Change B: Add explicit release gate for live booth readiness

**Section:** `Release Gates`

**OLD**

```md
- 카메라 미연결 또는 재연결 상황에서 고객 화면은 현재 연결 확인 중인지, 촬영 가능인지, 전화해야 하는지 구분 가능한 안내를 보여준다.
```

**NEW**

```md
- 카메라 미연결 또는 재연결 상황에서 고객 화면은 현재 연결 확인 중인지, 촬영 가능인지, 전화해야 하는지 구분 가능한 안내를 보여준다.
- 승인된 Windows booth image에서 물리 연결된 대상 카메라가 준비 화면에서 실제로 `촬영 가능` 상태로 전이되고, 그 전이가 host-normalized readiness truth와 session lifecycle log에 일치하게 반영된다.
```

**Rationale:** Mocked readiness cannot be accepted as final release proof.

### 4.4 Architecture Update

**Artifact:** `architecture.md`

#### Change A: Make live helper integration an explicit sequencing step

**Section:** `Implementation Sequence`

**OLD**

```md
5. Integrate a mocked camera sidecar against the fixed contract.
6. Replace the mock with a real bundled helper without changing host-facing semantics.
7. Add signing, staged rollout, and rollback automation.
```

**NEW**

```md
5. Integrate a mocked camera sidecar against the fixed contract.
6. Treat live camera helper integration as a dedicated backlog stream with explicit stories, validation fixtures, and hardware smoke checks.
7. Replace the mock with a real bundled helper without changing host-facing semantics.
8. Gate real booth acceptance for capture, export, and operator recovery workflows on successful live helper and hardware validation.
9. Add signing, staged rollout, and rollback automation.
```

**Rationale:** The architecture already assumed this transition, but it was not strong enough to govern backlog sequencing and acceptance gating.

#### Change B: Add validation note

**OLD**

```md
The architecture implies later replacement of the mocked helper.
```

**NEW**

```md
Add an explicit validation note that the mocked helper milestone is intentionally incomplete and must not be used as evidence of final booth readiness.
```

**Rationale:** This reduces future planning ambiguity.

### 4.5 Sprint Status Update

**Artifact:** `sprint-status.yaml`

#### Change A: Add Epic 6 and its stories

**OLD**

```yaml
development_status:
  epic-5: backlog
  5-1-expanded-lifecycle-and-intervention-logging-model: backlog
  5-2-kpi-query-and-support-issue-classification-views: backlog
  5-3-branch-rollout-policy-and-rollback-controls: backlog
```

**NEW**

```yaml
development_status:
  epic-5: backlog
  5-1-expanded-lifecycle-and-intervention-logging-model: backlog
  5-2-kpi-query-and-support-issue-classification-views: backlog
  5-3-branch-rollout-policy-and-rollback-controls: backlog

  epic-6: backlog
  6-1-real-camera-helper-execution-and-packaging-baseline: backlog
  6-2-live-camera-readiness-detection-and-state-normalization: backlog
  6-3-real-capture-to-session-folder-handoff: backlog
  6-4-helper-reconnect-restart-and-fault-routing: backlog
  6-5-booth-hardware-smoke-validation-and-release-gates: backlog
  epic-6-retrospective: optional
```

#### Change B: Capture downstream blocker relationship

**OLD**

```yaml
No explicit blocker or dependency note exists for real booth acceptance of Epics 2 through 4.
```

**NEW**

```yaml
change_control:
  booth_acceptance_blockers:
    epic-2: epic-6
    epic-3: epic-6
    epic-4: epic-6
```

**Rationale:** The status file should stop implying that downstream progress alone equals booth-readiness progress.

### 4.6 UX Specification Update

**Artifact:** `ux-design-specification.md`

**Recommended action:** No direct document change required at this stage.

**Rationale:** The UX specification still correctly defines customer-safe readiness messaging, calm waiting patterns, and operator/customer truth translation at different levels. The failure is implementation sequencing, not interaction design.

## 5. Implementation Handoff

### Scope Classification

**Major**

This change affects backlog structure, acceptance interpretation, sprint sequencing, and multiple downstream epics. It does not require architectural replacement, but it does require PM and architect correction plus PO or SM backlog management.

### Handoff Recipients and Responsibilities

- **Product Manager / Architect**
  - Approve the new epic and the artifact changes
  - Confirm live helper scope, target camera assumptions, and final booth validation gates
- **Product Owner / Scrum Master**
  - Update `epics.md`
  - Update `sprint-status.yaml`
  - Create the Epic 6 story files and sequencing notes
- **Development Team**
  - Keep Story 1.6 limited to customer-safe readiness UX and contract consumption
  - Avoid claiming booth readiness until Epic 6 validation succeeds

### MVP Impact

The MVP does not need a strategic reset, but its release interpretation must change. A mock-backed booth flow is still valuable as a milestone, but it is no longer sufficient evidence of MVP readiness for a real unmanned booth.

### High-Level Action Plan

1. Approve this sprint correction.
2. Update PRD, architecture, epics, and sprint status artifacts.
3. Create Epic 6 stories and resequence acceptance expectations.
4. Continue Story 1.6 only within its corrected mocked or contract-backed boundary.
5. Gate real booth acceptance of Epics 2 through 4 on Epic 6 completion.

### Success Criteria

- A dedicated live camera integration epic exists in planning artifacts
- Story 1.6 is explicitly bounded to mocked or contract-backed readiness UX
- PRD and architecture both treat live camera readiness as an MVP dependency and release gate
- Sprint tracking reflects the new epic and the downstream booth-acceptance blockers
- No team member can reasonably interpret mocked readiness as final real-booth readiness

## Summary Recommendation

Approve a sprint correction that adds a dedicated epic for live camera helper integration and hardware validation. Preserve the current architecture and valid groundwork, but correct the plan so mocked readiness, live readiness, and real booth acceptance are tracked as separate milestones.
