## Deferred from: code review of 1-14-공유-계약-동결과-검증-기준-확정.md (2026-04-10)

- Session timing can start with warning time already in the past near the hourly cutoff — pre-existing timing behavior in `src-tauri/src/session/session_manifest.rs` and not introduced by Story 1.14 changes.

## Deferred from: code review of 1-16-windows-desktop-build-release-baseline과-ci-proof-설정.md (2026-04-10)

- Sprint status contains unrelated story-state transitions in the mixed working tree; Story 1.16 itself only needs its own `in-progress` sync and should not be treated as the owner of the broader sprint-state drift.
- Hardware governance close-state expectations in `src/governance/hardware-validation-governance.test.ts` are pre-existing Story 6.2 scope and were not introduced by the release-baseline changes in Story 1.16.
