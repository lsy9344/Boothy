# Release Baseline

This is the canonical Windows release baseline. The root-level `release-baseline.md` is a pointer to
this file and holds no content of its own.

> **MVP internal-validation exception (approved 2026-08-17 by Noah Lee):** Story 7.7 / HV-18A is
> executed on the current approved PC. Code signing, Canon EDSDK redistribution evidence, and a
> clean/offline Windows environment are `waived-by-owner`. They are not verified passes, and this
> exception does not establish public-distribution security, redistribution rights, or clean-machine
> independence. Inventory integrity and the complete install lifecycle remain required.

A booth PC has no internet and no SDKs. Everything the product needs therefore travels inside one
installer, and this document describes how that installer is built, what it must contain, and what
stops it from shipping.

## Environment Prerequisites

These are **build machine** requirements. A booth PC needs none of them.

| Tool | Version | Used for |
| --- | --- | --- |
| Windows | 10/11 x64 | the canonical installer proof path |
| Node.js | 22.18+ or 24.x | Vite build and `release/build-inventory.ts` (run directly as TypeScript) |
| pnpm | 10.31.0 | workspace scripts |
| Rust | 1.77.2+ MSVC | `tauri build` |
| .NET SDK | 8.0.x | self-contained camera helper publish |
| PowerShell | 5.1+ | inventory and evidence gates |

Vendor payloads (Canon EDSDK 13.19.0, darktable 5.4.1) are not in the repository. Their procurement is
documented in `release/README.md`, and the ignored directories `release/vendor/` and `release/dist/`
are where they land.

Signing secrets are supplied through CI secrets or local environment variables only. They are never
committed.

## Bundle Configuration

`src-tauri/tauri.conf.json` fixes the following, and `src-tauri/tests/story_7_7_packaging.rs` asserts
each one so it cannot drift back silently.

| Setting | Value | Why |
| --- | --- | --- |
| `identifier` | `com.boothy.booth` | The starter default `com.tauri.dev` would have become the app-data root, the WebView2 user-data folder, and the NSIS upgrade identity of every deployed booth. |
| `bundle.targets` | `["nsis"]` | `"all"` produced an MSI as well, doubling everything that must be signed, hashed, and upgrade-tested. |
| `bundle.createUpdaterArtifacts` | `false`, explicitly | The guardrail is stated in configuration rather than inherited from a default. |
| `webviewInstallMode` | `{ "type": "offlineInstaller", "silent": true }` | The default `downloadBootstrapper` needs internet; on a clean offline VM the app installs but never opens a window. |
| `nsis.installMode` | `perMachine` | A booth PC is shared equipment and the install carries the darktable tree. Costs one UAC elevation at install time. |
| `nsis.startMenuFolder` | `Boothy` | Uninstall must be able to prove the entry is gone. |
| `windows.digestAlgorithm` / `timestampUrl` | `sha256` / DigiCert | Without a timestamp the signature dies when the certificate expires. |
| `windows.certificateThumbprint` | `null` today | The certificate has not been issued. The build still succeeds and the inventory records `signingStatus: "unsigned"`. |

## Bundled Payloads

| Resource | Installs to | Contract |
| --- | --- | --- |
| `release/dist/canon-helper/` | `sidecar/canon-helper/` | Self-contained `win-x64` publish. Lands exactly where `resolve_helper_launch_target()` already looked, so no camera code changed. |
| `release/vendor/darktable-5.4.1/` | `darktable/` | The pinned tree, unmodified, with `bin/`, `lib/` and `share/darktable/` as siblings. |
| `release/licenses/` | `licenses/` | Licensing evidence travels with the installer. |
| `release/dist/inventory.json` | `inventory.json` | What the runtime self-check compares against. |
| `storage/fixtures/display-sample/` | `fixtures/display-sample/` | The fixture the clean-offline round displays. |

The camera helper is shipped through `resources`, not `externalBin`: `externalBin` requires a
`-$TARGET_TRIPLE` filename and carries no companion files, and a self-contained publish is a folder of
.NET runtime assemblies plus `EDSDK.dll` and `EdsImage.dll`.

The bundled darktable is resolved **before** any installed copy. Bundling the tree while leaving the
old resolution order would have meant whatever darktable happens to be installed on a booth PC draws
the customer's photo, and the 5.4.1 pin would exist only in documentation.

## Commands

| Command | What it does |
| --- | --- |
| `pnpm release:stage` | Publishes the helper, proves it runs, checks the darktable tree really is 5.4.1, writes `release/dist/inventory.json`. |
| `pnpm release:verify` | Compares spec, generated inventory and disk. Distinct exit code per reason. |
| `pnpm release:verify:strict` | Additionally requires every staged component to carry a pinned digest. |
| `pnpm release:desktop` | Runs stage and verify first, then builds. |
| `pnpm release:seal` | After the build: records installer name, size and sha256, and **verifies the artifact name**. |
| `pnpm build:desktop` | Debug baseline build. Not a release candidate. |
| `pnpm test:release-gate` | Runs the inventory gate against seeded failures. |

## Inventory

The full contract is `docs/contracts/release-inventory.md`. The parts that matter for release
decisions:

- The installer must carry all nine component roles, each with a version, an origin, a licence, and —
  for anything actually staged — a tree digest over its files.
- A component that is installed but not in the inventory fails the release. A component in the
  inventory but not installed fails the release. Same severity.
- Failures carry distinct reason codes, because the operator action differs per cause.

`release/verify-inventory.ps1` exit codes:

| Exit | Reason code |
| --- | --- |
| 0 | pass |
| 2 | `inventory-component-missing` |
| 3 | `inventory-digest-mismatch` |
| 4 | `inventory-unexpected-component` |
| 5 | `inventory-not-staged` |
| 6 | `inventory-manifest-unreadable` |
| 7 | `inventory-pin-missing` |

## Runtime Self-Check

`boothy.exe --self-check [--report <path>]` compares the bundled manifest against the files on disk,
writes an `install-self-check/v1` report, and exits without ever creating a window.

| Exit code | Meaning |
| --- | --- |
| 0 | every component matched |
| 1 | the inventory did not match what is on disk |
| 2 | the check could not be performed, or the report could not be written |

Default report path: `%LOCALAPPDATA%\com.boothy.booth\diagnostics\install-self-check.json`.

The same structural comparison runs once on the normal startup path. A failure there is recorded in
the operator audit log under `release-governance`; it produces no customer-facing copy and blocks no
session, capture, or render path.

## Release Workflow

`.github/workflows/release-windows.yml`.

- Pull requests and pushes to `main` run lint, frontend and contract tests, the inventory gate
  self-test, the Rust test suite, and the debug desktop build.
- `workflow_dispatch` and `boothy-v*` tags additionally import the signing certificate, stage the
  payloads, verify the inventory, build, seal the inventory, and upload the inventory as an artifact.
- The tag path ends in a release gate that fails on any of: failed inventory verification, vendor
  payloads not restored, an unsigned installer, or a missing digest pin.

**Vendor payload restore is not silent.** When `BOOTHY_VENDOR_CACHE` is unset or incomplete, the
workflow writes the reason to the log and to the job summary and continues with an inventory that
records the gap.

### Signing Inputs

| Name | Where |
| --- | --- |
| `BOOTHY_WINDOWS_CERT_BASE64` | CI secret. A base64-encoded PFX. |
| `BOOTHY_WINDOWS_CERT_PASSWORD` | CI secret. |
| `BOOTHY_WINDOWS_CERT_THUMBPRINT` | Derived by CI after importing the PFX; passed to `tauri build` through a build-time config overlay so the thumbprint never enters the repository. |

Tauri v2 signs Windows bundles from `bundle.windows.certificateThumbprint` or
`bundle.windows.signCommand`. It does not read a certificate path or password directly, which is why
CI imports the PFX and derives a thumbprint rather than passing the file through.

## Artifact

The NSIS installer is produced at `src-tauri/target/release/bundle/nsis/` and must be named
`Boothy_<version>_x64-setup.exe`. **`pnpm release:seal` enforces that name** and fails if the built
artifact does not match what the inventory declares.

Sealing writes `release/dist/inventory.release.json`: the shipped inventory plus the installer's file
name, size and sha256. The copy embedded inside the installer cannot contain its own hash, so the
sealed copy is what HV-18A compares against.

## Release Behavior Guardrails

- The Tauri baseline keeps `createUpdaterArtifacts: false`, now stated explicitly in configuration.
- No updater auto-install path is enabled.
- Release promotion stays outside the active booth session path.
- Branch rollout governance applies build and preset-stack baselines only at safe transition points and
  never force-updates an active customer session.
- Uninstall must not remove the session root. Customer photos live under `Pictures\dabi_shoot` and are
  not owned by the installer.

## Release Truth Gates

- `automated proof` and `hardware proof` are separate release gates.
- The canonical hardware close record lives in
  `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`.
- Automated build/test success can prove implementation readiness, but booth `Ready` / `Completed`
  truth is not release-claimable until the ledger records `Go`.
- Any `No-Go`, missing evidence package, or unresolved blocker in the ledger keeps the branch on
  `release hold`.
- Sprint review and release sign-off must read `Automated Pass`, `Hardware Pass`, `Go / No-Go`,
  blocker, owner, and evidence path together.
- An unsigned installer may be used for development verification. It is never the basis of an HV-18A
  `Go`.
- A run whose `darktableResolution.source` is not `bundled-resource` is a run with a broken version
  pin, whatever its numbers say.

## Current State

Three things block a signed release candidate, and none of them is a code problem:

1. **Signing certificate.** Final certificate issuance and trusted-signing provider rollout remain
   intentionally gated until operational approval is complete. The pipeline is complete and waiting.
2. **Canon EDSDK redistribution right.** No executed agreement clause permitting redistribution of
   `EDSDK.dll` / `EdsImage.dll` inside a third-party installer has been recorded. See
   `release/licenses/canon-edsdk-13.19.0.md`.
3. **Vendor payload route for CI.** `BOOTHY_VENDOR_CACHE` has no agreed source yet, so CI records an
   explicit skip rather than producing a complete installer.

The repository also includes a host-owned `branch-config` rollout boundary so selected branch sets can
stage rollout or rollback without mutating booth session truth.
