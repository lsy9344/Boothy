# Preview Architecture Sprint Plan

Date: 2026-04-15
Scope: preview architecture track reset after updated `prd.md`, `architecture.md`, and `epics.md`

## Planning Decision

- Old track and new track are now separated.
- Old track is retained as comparison, rollback rehearsal, and historical evidence only.
- Stories 1.23 through 1.27 remain completed prototype/evidence/gate history and must not be interpreted as actual primary lane implementation complete.
- Stories 1.28 through 1.31 are the only forward implementation path that can reopen preview promotion and release close.
- Story 1.13 stays in the plan, but it remains blocked until Story 1.31 proves a canonical actual-lane `Go` candidate with rollback evidence on approved hardware.
- Story 1.26 is not part of the default implementation path. It opens only if the actual-lane forward path still repeatedly fails the approved hardware KPI after Stories 1.28 through 1.31 have been exercised.

## Prerequisite Check

| Item | Status | Notes |
| --- | --- | --- |
| `prd.md` updated to new full-screen KPI and reserve-only remote boundary | Done | Product acceptance is now centered on `same-capture preset-applied full-screen visible <= 2500ms`. |
| `architecture.md` updated to new local full-screen truthful artifact path | Done | New local lane, contract freeze baseline, and new implementation priorities are documented. |
| `epics.md` updated to old/new track split and gate order | Done | Old track = 1.18-1.20, baseline = 1.21-1.22, prototype track = 1.23-1.27, actual track = 1.28-1.31, release close = 1.13, reserve track = 1.26. |
| Shared contract freeze for implementation prerequisites | Done | Stories 1.14, 1.15, 1.16, 1.17 are already closed and the contract documents are present. |
| Required contract documents exist | Done | `session-manifest`, `preset-bundle`, `host-error-envelope`, `camera-helper-sidecar-protocol`, `camera-helper-edsdk-profile`, `runtime-capability-model`, `authoring-publication`, `authoring-publication-payload`, `branch-rollout`, `canonical-preset-recipe`, `local-dedicated-renderer`, `render-worker`. |
| Release baseline documents aligned to the new track | Done | Root `release-baseline.md` and `docs/release-baseline.md` both reflect the new promotion logic and gate wording. |

## Track Separation

### Old Track: Historical / Legacy Only

- Story 1.18: retired dedicated close baseline evidence
- Story 1.19: legacy parity and instrumentation ledger
- Story 1.20: legacy route activation validation track

Use:
- comparison baseline
- rollback rehearsal reference
- historical evidence package

Do not use:
- current release-close ownership
- default-route promotion authority
- final `Go / No-Go` decision input

### Metric / Evidence Baseline

1. Story 1.21: metric reset and acceptance alignment
2. Story 1.22: capture-to-full-screen evidence chain reset

### Prototype / Evidence Track

1. Story 1.23: local full-screen lane prototype and truthful artifact generation
2. Story 1.24: hardware canary validation
3. Story 1.25: default decision and rollback gate
4. Story 1.27: local hot-path darktable isolation and KPI revalidation

### Actual Implementation Track

1. Story 1.28: actual primary lane close owner implementation and prototype route separation
2. Story 1.29: actual primary lane evidence and vocabulary realignment
3. Story 1.30: actual primary lane hardware canary revalidation
4. Story 1.31: actual primary lane default decision and rollback gate

Exit rule:
- Story 1.31 evidence must be accepted as a canonical actual-lane `Go` candidate with rollback proof before Story 1.13 can start as release-close owner.

### Final Close Owner

- Story 1.13: final guarded cutover and hardware validation gate

Start condition:
- do not start until Story 1.31 evidence is accepted as a canonical actual-lane `Go` candidate with rollback proof

### Conditional Reserve Track

- Story 1.26: remote reserve POC

Open condition:
- only if the actual forward path still misses the approved hardware KPI after actual primary lane implementation, vocabulary realignment, actual-lane canary validation, and actual-lane default-decision review

## Immediate Next Recommendation

Recommended next story: `1.28 actual primary lane close owner implementation and prototype route separation`

Recommended next action: `Start Story 1.28, then advance through Stories 1.29, 1.30, and 1.31. Keep Story 1.13 blocked/No-Go unless the actual-lane track is accepted as the canonical actual-lane Go candidate with rollback proof, and keep Story 1.26 closed unless repeated approved-hardware KPI failure is confirmed after that track.`

Why this is the next decision:
- Stories 1.21 through 1.27 are already documented as metric/evidence baseline plus prototype-track history, so the unresolved question is no longer prototype viability.
- Stories 1.28 through 1.31 are the missing actual architecture implementation and revalidation path that determines whether Story 1.13 can reopen as release-close owner.
- Story 1.26 remains closed unless that actual-lane track still confirms repeated approved-hardware KPI failure.

## Gate Placement In The Plan

### Hardware Validation Gate

- Story 1.24 is the prototype-track hardware validation gate for the local lane.
- Story 1.30 is the actual implementation hardware validation gate for the actual primary lane.
- Required evidence bundle:
  - `sameCaptureFullScreenVisibleMs`
  - `replacementMs`
  - `fallbackRatio`
  - wrong-capture result
  - fidelity drift result
- A KPI miss, fallback-heavy behavior, wrong-capture, fidelity drift, or missing rollback proof keeps the plan at `No-Go`.
- Story 1.30 actual-lane canary must be accepted before Story 1.31 can close, and Story 1.31 must be accepted before Story 1.13 can reopen as the final release-close gate.

### Rollback Gate

- Story 1.25 is the prototype-track default-promotion and rollback gate.
- Story 1.31 is the actual-lane default-promotion and rollback gate that matters for final release-close reopening.
- Required proof:
- `preview-renderer-policy.json` shows controlled `canary -> default` or rollback transitions
- one-action rollback evidence exists
- active sessions are not reinterpreted by route-policy changes
- Without Story 1.31 rollback proof and accepted actual-lane evidence, the plan must not proceed to Story 1.13.

## Recommended Execution Order

1. Keep Stories 1.21 through 1.22 recorded as completed metric/evidence baseline.
2. Keep Stories 1.23 through 1.27 recorded as completed prototype/evidence history.
3. Start Story 1.28 actual primary lane implementation.
4. Close Story 1.29 to realign evidence and vocabulary around the actual lane.
5. Run Story 1.30 as the actual-lane canary gate.
6. Run Story 1.31 as the actual-lane default decision and rollback gate.
7. Re-open Story 1.13 only if that track is accepted as the canonical actual-lane `Go` candidate with rollback proof.
8. Keep Story 1.13 blocked and open Story 1.26 only if repeated approved-hardware KPI failure is confirmed after the actual-lane track.

## Planning Summary

- The prerequisite contract/doc baseline is effectively complete for implementation.
- The immediate next step is not Story 1.27 evidence review; it is Story 1.28 actual primary lane implementation.
- Hardware validation gate and rollback gate now have a prototype-track form and an actual-lane form; only the actual-lane form can reopen Story 1.13.
