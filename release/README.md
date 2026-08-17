# Release Staging and Vendor Procurement

Everything a booth PC needs has to be inside one installer, because a booth PC has no internet and
no SDKs. This directory is where the parts that are not source code get assembled before the
installer is built.

Nothing large is committed here. `release/vendor/` and `release/dist/` are ignored by git; this
document is what makes them reproducible.

## Layout

| Path | Tracked | What it is |
| --- | --- | --- |
| `release/inventory-spec.json` | yes | What the installer MUST contain. Human-maintained source of truth. |
| `release/build-inventory.ts` | yes | Walks the staged trees and writes `release/dist/inventory.json`. |
| `release/verify-inventory.ps1` | yes | Machine gate. Compares spec, generated inventory, and disk. |
| `release/licenses/` | yes | Licensing evidence. Documents belong in version control, not in the ignored payload tree. |
| `release/vendor/` | **no** | Procured payloads: `darktable-5.4.1/`. |
| `release/dist/` | **no** | Staging output: `canon-helper/`, `inventory.json`, `inventory.release.json`. |

## One-Time Build Prerequisites

| Tool | Version | Needed for |
| --- | --- | --- |
| Node | 22.18+ (or 24.x) | Runs `build-inventory.ts` directly; the script uses erasable type syntax only. |
| pnpm | 10.31.0 | Workspace scripts. |
| Rust | 1.77.2+ MSVC | `tauri build`. |
| .NET SDK | 8.0.x | Camera helper publish. |
| PowerShell | 5.1+ | `verify-inventory.ps1`. |

A booth PC needs none of these. They are build-machine requirements only.

## Step 1 — Procure the Canon EDSDK payload

The SDK is under a Canon redistribution agreement and is never committed.

- Canon release: `EDSDK 13.19.0` (2025-02-28)
- Archive SHA256: `C4F15E9DF3E24D439405828F69C2AD46DAE0490C29022E31DCB31969B3AA699D`
- Extract to `sidecar/canon-helper/vendor/canon-edsdk/`, or point `BOOTHY_CANON_SDK_ROOT` at it.

Verify:

```powershell
Get-FileHash -Algorithm SHA256 <downloaded-zip>
Test-Path sidecar\canon-helper\vendor\canon-edsdk\Windows\EDSDK_64\Dll\EDSDK.dll
Test-Path sidecar\canon-helper\vendor\canon-edsdk\Windows\Sample\CSharp\CameraControl\CameraControl\EDSDK.cs
```

Redistribution evidence lives in `release/licenses/canon-edsdk-13.19.0.md`. If that evidence is not
complete, the inventory records the gap instead of hiding it.

## Step 2 — Procure the pinned darktable tree

The render pin is exact. darktable 5.4.1 is the version HV-15 validated the render flags against
(`--icc-intent PERCEPTUAL` works there; `INTENT_PERCEPTUAL` does not). A different build silently
changes customer-visible pixels.

- Pin: `5.4.1` (`release-5.4.1` / `c3f96ca`)
- Official Windows build: <https://github.com/darktable-org/darktable/releases/tag/release-5.4.1>
- Extract the installed tree to `release/vendor/darktable-5.4.1/`

The tree must have these as siblings — copying only `darktable-cli.exe` does not work:

```
release/vendor/darktable-5.4.1/
├── bin/darktable-cli.exe
├── lib/
└── share/darktable/
```

Verify the extracted tree actually is 5.4.1 before it goes anywhere near the inventory:

```powershell
Get-FileHash -Algorithm SHA256 <downloaded-installer-or-archive>
.\release\vendor\darktable-5.4.1\bin\darktable-cli.exe --version
```

A version string that is not `5.4.1` is a procurement failure, not a warning.

Record both hashes in `release/inventory-spec.json`:

- `pinnedSourceArchiveSha256` — the archive you downloaded.
- `pinnedStagedTreeDigest` — printed by `pnpm release:stage`, which reports the digest of every
  staged tree.

Until those pins are filled in, the component is **unpinned**. `release:verify:strict` fails on an
unpinned component, and HV-18A never accepts a run built from an unpinned tree.

Only **procured** payloads carry `requiresPin: true`. The camera helper is built from source in this
repository and its bytes change on every publish (PE timestamps and module IDs), so pinning its tree
digest would make every rebuild fail forever; it is `requiresPin: false` and is instead proven by the
staging run's execution checks. The EDSDK runtime is a procured, byte-stable payload and is pinned.

GPL-3.0-or-later obligations for the bundled darktable binaries are recorded in
`release/licenses/darktable-5.4.1.md`, including where the corresponding source is obtained.

## Step 3 — Publish the camera helper self-contained

A booth PC has no .NET runtime. The helper is published for `win-x64` with the runtime included.

```powershell
dotnet publish sidecar\canon-helper\src\CanonHelper\CanonHelper.csproj `
  -c Release -r win-x64 --self-contained true `
  -o release\dist\canon-helper
```

`RuntimeIdentifier` and `SelfContained` are deliberately **not** set in the `.csproj`. Setting them
there slows the `dotnet run` development loop for no packaging benefit; they belong on the publish
command.

`PublishSingleFile` is deliberately not used. The EDSDK native DLLs would then be extracted to a
temporary path at runtime, which makes booth diagnostics harder and buys nothing.

The publish output must contain:

- `canon-helper.exe`
- `EDSDK.dll` and `EdsImage.dll`
- the .NET runtime assemblies (`System.Private.CoreLib.dll`, `hostfxr.dll`, `hostpolicy.dll`, …)

Then prove it actually runs before it is allowed into the inventory. A payload that builds but does
not start is worse than a missing one, because it looks complete:

```powershell
.\release\dist\canon-helper\canon-helper.exe --version
.\release\dist\canon-helper\canon-helper.exe --self-check --sdk-root release\dist\canon-helper
```

`release:stage` runs both checks. If either fails, it writes an explicit `missing` component into
the inventory so `release:verify` can reject the candidate with an actionable reason code.

## Step 4 — Stage, verify, build

```powershell
pnpm release:stage      # helper publish check + darktable tree check + inventory.json
pnpm release:verify     # machine gate: spec vs inventory vs disk
pnpm release:desktop    # runs the two steps above first, then tauri build
pnpm release:seal       # after the build: records installer name, size, sha256
```

To pass a switch to the staging script, call it directly — `pnpm` does not forward `-Switch`
arguments cleanly to PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File release\stage.ps1 -SkipHelperPublish
```

`release:verify` exit codes are the contract, not decoration:

| Exit | Reason code |
| --- | --- |
| 0 | pass |
| 2 | `inventory-component-missing` |
| 3 | `inventory-digest-mismatch` |
| 4 | `inventory-unexpected-component` |
| 5 | `inventory-not-staged` |
| 6 | `inventory-manifest-unreadable` |
| 7 | `inventory-pin-missing` (only with `-RequirePins`) |

## What happens when the payloads are absent

`cargo` builds would otherwise fail outright, because Tauri validates every `bundle.resources` path
at compile time. So `src-tauri/build.rs` creates the missing directories and drops a
`PAYLOAD-NOT-STAGED.md` marker plus a `staging: "not-staged"` inventory.

That keeps `cargo test` and the CI baseline build working on a fresh clone, and it does not create a
false release candidate:

- the marker file says, in the installer itself, that the payload is absent;
- `release:verify` stops the release;
- the installed app's `--self-check` exits `1` with `inventory-not-staged`.

Real staged payloads are never overwritten by the build script.

## Signing

Signing inputs live in `src-tauri/tauri.conf.json` under `bundle.windows`
(`digestAlgorithm`, `timestampUrl`, `certificateThumbprint`) and in the workflow secrets described by
`docs/release-baseline.md`.

`certificateThumbprint` is `null` today because the certificate has not been issued yet. Builds
still succeed; the inventory then records `signingStatus: "unsigned"`, and an unsigned installer is
never the basis of an HV-18A `Go`.

## Installed layout

For reference when reading the self-check report:

```
C:\Program Files\Boothy\
├── Boothy.exe
├── inventory.json
├── darktable\bin\darktable-cli.exe
├── sidecar\canon-helper\canon-helper.exe
├── licenses\
└── fixtures\display-sample\
```
