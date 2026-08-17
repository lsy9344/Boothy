# Release Inventory Contract

Boothy proves what a release candidate actually contains with a machine-checked file, not with a
hand-maintained table. A table goes stale the moment a build changes; a digest does not.

Two schemas live here:

- `release-inventory/v1` — what the installer is supposed to contain, and what it actually contained
  when it was staged.
- `install-self-check/v1` — what the installed app found on disk when it checked itself.

Both are defined once for TypeScript (`src/shared-contracts/schemas/release-inventory.ts`) and once
for Rust (`src-tauri/src/contracts/dto.rs`). There is no third definition.

## Why a Contract And Not a Document

Acceptance Criterion 1 of Story 7.7 asks for exact versions and hashes of every shipped component.
A prose list cannot satisfy that: nothing re-reads it when the build changes. So the inventory is
split into a spec the humans maintain and an artifact the build produces, and a gate compares them.

```
release/inventory-spec.json     source, human-maintained   -> what MUST be there
        v  the build stages the payload trees
release/dist/inventory.json     generated                  -> what IS there, with digests
        v  release:verify compares the two (release stops on any difference)
bundled into the installer      resource                   -> the runtime self-check re-compares
```

## Staging States

The top-level document is discriminated on `staging`.

| `staging` | Meaning | Release consequence |
| --- | --- | --- |
| `staged` | The payload trees exist and were digested. | Eligible for release once `release:verify` passes. |
| `not-staged` | `release:stage` never ran. | Not a release candidate. `--self-check` fails with `inventory-not-staged`. |

A `not-staged` document is a first-class part of the contract on purpose. A build that has no
inventory would be silent about it; a build that carries a `not-staged` inventory declares its own
gap, and every downstream gate can read that declaration.

## Component Roles

Exactly one entry per role, and all nine roles must be present.

| Role | What it is in Boothy today |
| --- | --- |
| `app` | The Boothy executable produced by the NSIS bundle. |
| `camera-helper` | The self-contained `canon-helper` publish tree. |
| `edsdk-runtime` | The approved Canon EDSDK native runtime shipped beside the helper. |
| `source-adapter` | `raw-original`. No separate library — HV-14 closed the fast-source candidates. |
| `display-renderer` | Not applicable. HV-16 recorded a `Technology No-Go` for the resident renderer. |
| `color-profile` | darktable's built-in sRGB (`--icc-type SRGB`). There is no separate ICC file. |
| `proxy-recipes` | Preset XMP templates compiled into the app binary with `include_str!`. |
| `raw-renderer` | The pinned darktable 5.4.1 tree. |
| `webview2-runtime` | The offline WebView2 installer embedded by the NSIS bundle. |

## Component Status

| `status` | Requires | Forbids |
| --- | --- | --- |
| `present` | `version`, `stagedTreeDigest`, `installRelativePath`, `licenseEvidencePath`, `fileCount`, `totalBytes` | — |
| `embedded` | `rationale`, `installRelativePath` | `stagedTreeDigest` |
| `not-applicable` | `rationale` | `stagedTreeDigest` |
| `missing` | `rationale` | `stagedTreeDigest` |

`license` is required on every component regardless of status. Licensing evidence is part of the
inventory itself, not a separate document that can drift away from it.

An inactive candidate is written as `not-applicable` with a rationale. It is never written as an
empty string or an omitted entry — the next validation round would have to ask why it was blank.

## Digests

Each staged component carries two hashes:

- `sourceArchiveSha256` — the archive that was procured, so procurement is reproducible.
- `stagedTreeDigest` — the tree as staged, so a single changed file is detectable.

The tree digest is the sha256 of the newline-joined, path-sorted list of `<relative-path>:<sha256>`
entries, with `/` as the separator. Sorting is by UTF-8 byte order, so the TypeScript generator, the
PowerShell gate, and the Rust self-check all produce the same value. It is therefore stable across
directory-listing order and unstable across any content change, any added file, and any removed file.

Each staged component also carries `entrySelection`, which says which files under its install path
belong to it:

| `kind` | Meaning |
| --- | --- |
| `all` | Every file under the component's install path. |
| `only` | Exactly the listed files. |
| `allExcept` | Every file under the path except the listed ones. |

This exists because the installed app does not carry `inventory-spec.json`. Without the selection
rule inside the inventory, `--self-check` could not know which files to hash — `camera-helper` and
`edsdk-runtime` both live in `sidecar/canon-helper/` and each owns a different part of it. No globs
are used anywhere: what a component contains must be decidable by reading the document.

The installer's own hash cannot live inside the installer. `installer` is `null` in the embedded
copy and is filled in by the sealing step, which writes `release/dist/inventory.release.json` next
to the built installer. That sealed copy is what HV-18A compares against the artifact it installed.

## Failure Reason Codes

Verification never reports one undifferentiated "verification failed". Each cause carries its own
code because the operator action differs.

| Reason code | Meaning | `verify-inventory.ps1` exit code |
| --- | --- | --- |
| `inventory-component-missing` | The spec requires a component the staged tree does not have. | 2 |
| `inventory-digest-mismatch` | The component is there, but its digest is not what the spec pinned. | 3 |
| `inventory-unexpected-component` | The staged tree carries a component the spec never declared. | 4 |
| `inventory-not-staged` | `release:stage` never ran. | 5 |
| `inventory-manifest-unreadable` | The inventory file is absent or unparseable. | 6 |
| `inventory-pin-missing` | `-RequirePins` was asked for and a staged component has no pinned digest. | 7 |

When several causes are present at once the gate prints all of them and exits with the first that
applies in the order 6, 5, 2, 3, 4, 7.

`inventory-pin-missing` only applies to components the spec marks `requiresPin: true` — procured
payloads whose bytes are fixed, such as the darktable tree and the EDSDK runtime. Artefacts this
repository builds itself are not pinnable: a `dotnet publish` output differs on every build (PE
timestamps, module IDs), so pinning its digest would make every rebuild fail forever. Those are
proven by the staging run actually executing them instead.

The runtime self-check reuses the same vocabulary and adds `digest-tool-unavailable`,
`darktable-not-bundled`, `darktable-tree-incomplete`, `helper-binary-missing`,
`helper-version-check-failed`, and `webview2-runtime-unreadable`.

`helper-binary-missing` is deliberately the existing booth-readiness reason code. Story 7.7 adds no
new customer-visible reason codes; a missing camera helper already has a name.

## Self-Check Report

`boothy.exe --self-check [--report <path>]` writes `install-self-check/v1` and exits. It does not
create a window, because clean-offline automation has no way to close one.

Default report path: `%LOCALAPPDATA%\<identifier>\diagnostics\install-self-check.json`.

| Exit code | Meaning |
| --- | --- |
| `0` | Every component matched. |
| `1` | The inventory did not match what is on disk. |
| `2` | The check could not be performed or the report could not be written. |

`1` and `2` are never merged. "The check failed" and "the check could not run" are different facts,
and only one of them says anything about the installed payload.

The same comparison runs once on the normal startup path. A failure there is projected into the
existing operator audit log under the `release-governance` category. It never becomes customer copy
and it never blocks the session, capture, or render paths.

## Consequences For Release

- A component that is installed but not in the inventory fails the release.
- A component that is in the inventory but not installed fails the release. Both are the same
  severity.
- An unsigned installer may be used for development verification. It is never the basis of an
  HV-18A `Go`.
- A run whose `darktableResolution.source` is anything other than `bundled-resource` is a run with a
  broken version pin, regardless of how good its numbers look.
