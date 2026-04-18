# Release Baseline

Boothy keeps an explicit Windows release baseline even before production signing is fully activated.

## Environment Prerequisites

- Windows environment for the canonical installer proof path
- Node.js 20.19+ or 22.12+
- `pnpm` 10.x
- Rust 1.77.2+
- Signing secrets provided outside the repository through CI secrets or local environment variables only

## Commands

- `pnpm build:desktop`
- `pnpm release:desktop`

Both commands now exist in `package.json` and wrap the current Vite + Tauri desktop build path.

## Release Workflow

The draft workflow lives at `.github/workflows/release-windows.yml`.

- Pull requests to `main` and pushes to `main` run the unsigned Windows baseline validation path through `pnpm build:desktop`.
- `workflow_dispatch` and `boothy-v*` tags run the signing-ready draft release path through `pnpm release:desktop`.

## Signing Inputs

- `BOOTHY_WINDOWS_CERT_PATH` or `BOOTHY_WINDOWS_CERT_BASE64`
- `BOOTHY_WINDOWS_CERT_PASSWORD`
- Optional: `BOOTHY_WINDOWS_TIMESTAMP_URL`

## Artifact Path

The exact NSIS output path depends on how `tauri build` is invoked (`debug` vs release) and the current Tauri workspace layout, so this document treats the bundle location as build-output-dependent rather than hard-coding a single canonical path.

## Expected Outputs

- Local unsigned baseline proof: `pnpm build:desktop` completes successfully on Windows and emits a Tauri desktop bundle.
- CI signing-ready proof: `.github/workflows/release-windows.yml` runs the draft release build path for manual verification.
- Installer naming and signing verification are still manual follow-up checks; the current workflow does not yet enforce an automated identity assertion.

## Release Behavior Guardrails

- The Tauri baseline keeps `createUpdaterArtifacts: false`
- No updater auto-install path is enabled in this story
- Release promotion remains outside the active booth session path
- Branch rollout governance applies build and preset-stack baselines only at safe transition points and never force-updates an active customer session

## Release Truth Gates

- `automated proof` and `hardware proof` are separate release gates.
- The canonical hardware close record lives in `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`.
- Automated build/test success can prove implementation readiness, but booth `Ready` / `Completed` truth is not release-claimable until the ledger records `Go`.
- Any `No-Go`, missing evidence package, or unresolved blocker in the ledger keeps the branch on `release hold`.
- Sprint review and release sign-off must read `Automated Pass`, `Hardware Pass`, `Go / No-Go`, blocker, owner, and evidence path together.

## Current Preview Route Status

- The current official preview-track hardware judgment requires both `sameCaptureFullScreenVisibleMs <= 3000ms` and `originalVisibleToPresetAppliedVisibleMs <= 3000ms`.
- The newer `actual-primary-lane` route is currently treated as a bounded `No-Go`, based on repeated approved-hardware reruns that remained far outside that dual gate.
- This worktree does not claim a replacement release winner yet. It exists to revalidate the older `resident first-visible` line as a validation lane because it previously produced better customer-perceived speed.
- Historical better numbers in that older lane are comparison evidence only. They do not prove the current release gate and must not be read as automatic rollback proof.
- GPU-enabled acceleration is scoped here as a validation hypothesis for the old lane, not as an official success guarantee.
- Until a candidate lane records `Go` in the hardware ledger against the current gate, preview-track release promotion remains on `release hold`.

## Current State

Signing-ready blocker: final certificate issuance and trusted-signing provider rollout remain intentionally gated until operational approval is complete. The local and CI signing-ready paths now accept either a materialized certificate path or a base64-encoded PFX supplied through environment variables.

The repo now also includes a host-owned `branch-config` rollout boundary so selected branch sets can stage rollout or rollback without mutating booth session truth.
