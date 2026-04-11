## Deferred from: code review of 1-14-공유-계약-동결과-검증-기준-확정.md (2026-04-10)

- Session timing can start with warning time already in the past near the hourly cutoff — pre-existing timing behavior in `src-tauri/src/session/session_manifest.rs` and not introduced by Story 1.14 changes.

## Deferred from: code review of 1-16-windows-desktop-build-release-baseline과-ci-proof-설정.md (2026-04-10)

- Sprint status contains unrelated story-state transitions in the mixed working tree; Story 1.16 itself only needs its own `in-progress` sync and should not be treated as the owner of the broader sprint-state drift.
- Hardware governance close-state expectations in `src/governance/hardware-validation-governance.test.ts` are pre-existing Story 6.2 scope and were not introduced by the release-baseline changes in Story 1.16.

## Deferred from: code review of 1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md (2026-04-10)

- `cargo test --test operator_diagnostics` still fails in untouched Rust diagnostics code; live-capture freshness and camera-connection classification do not match the existing test expectations in `src-tauri/src/diagnostics/mod.rs`, but that regression is not introduced by the current Story 1.10 working-tree changes.

## Deferred from: code review of 4-2-부스-호환성-검증과-승인-준비-상태-전환.md (2026-04-11)

- Story 4.2 review diff included booth readiness/customer-copy changes in `src-tauri/src/contracts/dto.rs`, but the current branch’s canonical post-end/readiness contract is intentionally owned by later closed stories such as Story 3.2 and related follow-ups, so this is not a safe current-branch patch target.
