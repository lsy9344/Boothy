# Release Baseline

Boothy keeps an explicit Windows release baseline even before production signing is fully activated.

## Environment Prerequisites

- Windows environment for the canonical installer proof path
- Node.js 20.19+ or 22.12+
- `pnpm` 10.x
- Rust 1.77.2+
- Microsoft Visual Studio C++ Build Tools for the MSVC toolchain
- Microsoft Edge WebView2 runtime
- Signing secrets provided outside the repository through CI secrets or local environment variables only

## Canonical Commands

- `pnpm build:desktop`
  - Canonical unsigned local proof path
  - Runs `pnpm tauri build --debug --no-sign`
  - Intended to prove packaging prerequisites and bundle generation without enabling updater or signing behavior
  - Prepares the packaged `dedicated renderer` shadow binary before Tauri packaging starts
- `pnpm release:desktop`
  - Canonical release-draft proof path
  - Runs the same frontend build, then validates `BOOTHY_WINDOWS_CERT_*` inputs if they are present
  - If no signing inputs are present, the command remains an unsigned draft proof
  - Current baseline keeps this path at input validation only; production signing activation is still operationally gated

## Signing Inputs

- `BOOTHY_WINDOWS_CERT_PATH` or `BOOTHY_WINDOWS_CERT_BASE64`
- `BOOTHY_WINDOWS_CERT_PASSWORD`
- Optional: `BOOTHY_WINDOWS_TIMESTAMP_URL`

`release:desktop` and `.github/workflows/release-windows.yml` use the same rules:

- Provide exactly one certificate source: file path or base64 PFX
- Provide a password whenever a certificate source is provided
- Local and self-hosted proof can use `BOOTHY_WINDOWS_CERT_PATH` or `BOOTHY_WINDOWS_CERT_BASE64`
- Hosted GitHub Actions draft proof consumes `BOOTHY_WINDOWS_CERT_BASE64` from repository secrets and does not rely on runner-local certificate paths
- `BOOTHY_WINDOWS_TIMESTAMP_URL` is recorded as release-proof context only in the current draft baseline
- Missing signing inputs do not block the unsigned draft proof path

## Output Expectations

- Unsigned local proof output root: `src-tauri/target/debug/bundle/`
- Release-draft proof output root: `src-tauri/target/release/bundle/`
- CI proof summary is appended to `GITHUB_STEP_SUMMARY`
- CI evidence artifact is uploaded from `release-proof/`
- CI proof summary records `Proof path`, `Proof outcome`, `Hardware gate status`, and `Promotion state`

## Packaging Failure Checklist

If `pnpm build:desktop` or `pnpm release:desktop` fails, check these first:

- Node.js version still matches `20.19+` or `22.12+`
- `pnpm -v` still reports 10.x
- `rustc -V` still reports `1.77.2+`
- MSVC Build Tools and WebView2 runtime are installed on the Windows machine
- `src-tauri/tauri.conf.json` still keeps `bundle.createUpdaterArtifacts: false`
- `src-tauri/tauri.conf.json` keeps a product-unique `identifier` and does not fall back to `com.tauri.dev`
- No forced-update or updater activation was introduced outside the rollout contract

## Release Workflow

The draft workflow lives at `.github/workflows/release-windows.yml`.

- The workflow is pinned to `windows-2025` to reduce runner-image drift in release proof evidence.
- Pull requests to `main` and pushes to `main` run the unsigned Windows baseline validation path through `pnpm build:desktop`.
- `workflow_dispatch` from `main` and `boothy-v*` tags run the canonical draft release path through `pnpm release:desktop`.
- `workflow_dispatch` from any other ref fails fast instead of emitting ambiguous release proof.
- `workflow_dispatch` from `main` runs only the draft release proof path; it does not also rerun the unsigned baseline lane.
- The workflow runs release baseline governance checks via `pnpm test:run src/governance/release-baseline-governance.test.ts`.
- The workflow runs branch rollout safety tests via `cargo test --test branch_rollout` before collecting release proof artifacts.
- The workflow uploads a proof artifact and records the automated proof summary in `GITHUB_STEP_SUMMARY`.

## Release Behavior Guardrails

- `src-tauri/tauri.conf.json` keeps `bundle.createUpdaterArtifacts: false`
- `src-tauri/tauri.conf.json` uses the product bundle identifier `com.boothy.desktop`
- Tauri `beforeBuildCommand`/`beforeDevCommand` still prepare the shadow `dedicated renderer` binary before app packaging
- No updater auto-install path is enabled in this story
- Release promotion remains outside the active booth session path
- Branch rollout governance applies build and preset-stack baselines only at safe transition points and never force-updates an active customer session

## Release Truth Gates

- `automated proof` and `hardware proof` are separate release gates.
- The canonical hardware close record lives in `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`.
- Story 1.20 is the preview architecture activation owner; Story 1.13 remains the final guarded cutover / release-close owner.
- Story 1.13 is the canonical preview architecture close owner for guarded cutover, rollback evidence, and `preview-renderer-policy.json` proof.
- Automated build/test success can prove implementation readiness, but booth `Ready` / `Completed` truth is not release-claimable until the ledger records `Go`.
- Any `No-Go`, missing evidence package, or unresolved blocker in the ledger keeps the branch on `release hold`.
- Failed or skipped automation proof keeps `Promotion state` on `release hold` even if earlier hardware evidence exists.
- CI proof artifacts remain evidence only; `Promotion state` stays non-release until the hardware ledger clears the gated stories for close.
- Preview architecture promotion evidence must include the host-owned `branch-config/preview-renderer-policy.json` state together with the booth session package so shadow, canary, default, and rollback boundaries stay auditable.
- The canonical booth package must preserve capture-time route-policy and catalog snapshots inside the session diagnostics bundle before evidence is assembled.
- Sprint review and release sign-off must read `Automated Pass`, `Hardware Pass`, `Go / No-Go`, blocker, owner, and evidence path together.
- Preview promotion sign-off also reads latency, parity, fallback ratio, route policy state, and rollback evidence together; speed alone cannot produce `Go`.
- Repeated `canary` success-path evidence and one-action rollback proof are prerequisites before any `default` route claim is considered.

## Current State

Signing-ready blocker: final certificate issuance and trusted-signing provider rollout remain intentionally gated until operational approval is complete. The current baseline keeps `release:desktop` and CI on an unsigned draft proof unless `BOOTHY_WINDOWS_CERT_*` inputs are deliberately supplied for validation.

The repo also includes a host-owned `branch-config` rollout boundary so selected branch sets can stage rollout or rollback without mutating booth session truth.

On April 11, 2026, Story 1.13 remains on `release hold`: the observed `branch-config/preview-renderer-policy.json` still keeps `defaultRoute` on `darktable`, with only a manual canary for `preset_test-look@2026.03.31`, so the canonical ledger remains `No-Go` until a fresh booth package proves promoted dedicated-renderer close behavior and one-action rollback evidence.
