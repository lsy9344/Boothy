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

## Release Workflow

The draft workflow lives at `.github/workflows/release-windows.yml`.

- Pull requests to `main` and pushes to `main` run the unsigned Windows baseline validation path.
- `workflow_dispatch` and `boothy-v*` tags run the signing-ready draft release path.

## Signing Inputs

- `BOOTHY_WINDOWS_CERT_PATH` or `BOOTHY_WINDOWS_CERT_BASE64`
- `BOOTHY_WINDOWS_CERT_PASSWORD`
- Optional: `BOOTHY_WINDOWS_TIMESTAMP_URL`

## Artifact Path

The NSIS installer is expected under `target/release/bundle/nsis` or `src-tauri/target/release/bundle/nsis`, depending on how the build is invoked.

## Expected Outputs

- Local unsigned baseline proof: `src-tauri/target/release/bundle/nsis/Boothy_0.1.0_x64-setup.exe`
- CI signing-ready proof: draft release created by `.github/workflows/release-windows.yml`
- CI identity check: the workflow verifies that the produced installer matches `Boothy_${version}_x64-setup.exe`

## Release Behavior Guardrails

- The Tauri baseline keeps `createUpdaterArtifacts: false`
- No updater auto-install path is enabled in this story
- Release promotion remains outside the active booth session path

## Current State

Signing-ready blocker: final certificate issuance and trusted-signing provider rollout remain intentionally gated until operational approval is complete. The local and CI signing-ready paths now accept either a materialized certificate path or a base64-encoded PFX supplied through environment variables.
