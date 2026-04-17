# Branch Rollout Contract

Boothy keeps branch rollout governance as a host-owned boundary separate from booth session truth.

## Baseline Model

- `deploymentBaseline`: the approved build and preset stack that the branch will use for the next safe session entry point.
- `rollbackBaseline`: the last approved baseline that can be restored with one rollback action.
- `pendingBaseline`: a staged baseline that was approved during an active session and must wait for a safe transition point.
- `activeSession.lockedBaseline`: the build and preset stack already pinned by the in-flight customer session.

## Safe Transition Rule

- If a targeted branch has no active customer session, rollout or rollback can apply immediately.
- If a targeted branch has an active customer session, the host does not force-update the branch.
- The host records a deferred outcome, preserves local settings, and stages the requested baseline in `pendingBaseline`.
- When the branch reaches `after-session-end`, the staged baseline becomes the new `deploymentBaseline`.

## Local Settings Preservation

Rollout governance may change only the approved build and preset-stack baseline.

- Preserved settings include branch contact information and bounded operational toggles.
- Branch-local settings are summarized in the UI and audit payload, but the raw values remain in branch-owned config.

## Audit Shape

- Dedicated history lives under `branch-config/rollout-history.json`.
- Each entry records:
  - action
  - requested branch set
  - target baseline
  - approval metadata
  - per-branch outcome
- Matching operator audit events are also appended under the host-owned `release-governance` taxonomy so release actions remain queryable next to other operational history.

## Rejection Guidance

The host returns operator-safe refusal guidance for these cases:

- unapproved target baseline
- missing rollback baseline
- unknown branch identifier
- active-session defer
- audit write failure rollback guard

## Preview Route Decision Summary

- Preview route governance surfaces must read `decisionSummary.implementationTrack` first, then `decisionSummary.laneOwner`.
- `implementationTrack=actual-primary-lane` means the decision summary is carrying actual-lane promotion proof.
- `implementationTrack=prototype-track` or `null` means the summary is comparison-only and must not be read as release-relevant actual-lane completion.
- `laneOwner` remains a backward-compatible machine field, but human-facing settings/operator copy should treat it as the close-owner label inside the selected track, not as the primary proof-family discriminator.
