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

## Preview Route Canary

- preview close canary는 release baseline 자체와 분리된 host-owned policy file `branch-config/preview-renderer-policy.json`으로 관리한다.
- 이 policy의 default route는 항상 approved darktable baseline이어야 한다.
- active customer session은 세션 시작 시점의 preview route policy snapshot을 고정하고, 이후 branch policy edit이 first capture 전이든 mid-session이든 현재 세션 route를 뒤집으면 안 된다.
- local renderer canary는 branch / session / preset scope rule로만 opt-in한다.
- forced fallback rule은 unhealthy sidecar를 즉시 darktable path로 우회할 수 있어야 한다.
- active customer session을 깨는 즉시 전환을 만들지 않도록, broad booth rule보다 session-scoped canary를 우선한다.

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
