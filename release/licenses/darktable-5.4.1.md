# darktable 5.4.1 — Licensing Evidence

## What Boothy ships

The complete official darktable 5.4.1 Windows tree (`bin/`, `lib/`, `share/darktable/`) is copied
into the installer and lands at `<install root>\darktable\`.

Boothy invokes `darktable-cli.exe` as a **separate process** over a command line. There is no
linking, no in-process loading, and no shared address space. Under GPL-3.0 that makes Boothy and
darktable an aggregation rather than a combined work, so Boothy's own source is not placed under
the GPL by this arrangement.

**The obligations attached to redistributing the darktable binaries themselves still apply.** They
are listed below, and this document is what discharges them.

## License

- darktable is licensed **GPL-3.0-or-later**.
- Upstream project: <https://github.com/darktable-org/darktable>
- Pinned release: `release-5.4.1`, commit `c3f96ca`
- Release page: <https://github.com/darktable-org/darktable/releases/tag/release-5.4.1>

## Obligations and how each is met

| Obligation | How it is met |
| --- | --- |
| Convey the license text with the binaries | The upstream tree carries `share/doc/darktable/LICENSE` (or equivalent) and is copied verbatim into the installer. The staged tree digest in the inventory covers it, so removing it changes the digest and fails verification. |
| State the licence and that the work is modified or unmodified | The darktable tree is shipped **unmodified**. No patching, no recompilation, no file removal. `stagedTreeDigest` in `release/inventory-spec.json` pins that fact. |
| Offer the corresponding source | The corresponding source is the tagged upstream commit above. Boothy points recipients at `https://github.com/darktable-org/darktable/tree/release-5.4.1` and additionally keeps an archived copy of that tag alongside the release artifacts. |
| Preserve copyright notices | Nothing in the tree is stripped. |

## Corresponding source

- Tag: `release-5.4.1`
- Commit: `c3f96ca`
- Clone command that reproduces exactly what is shipped:

```
git clone --branch release-5.4.1 https://github.com/darktable-org/darktable.git
git -C darktable checkout c3f96ca
```

## Open item for the approver

The exact wording of the redistribution notice that ships to booth operators is a business decision,
not a code decision. Until it is approved, this file records the obligations and the source pointer;
the notice text itself is the approver's to sign off.

Owner: Noah Lee. Status: **pending approval**.

## Why the version is pinned exactly

HV-15 validated the render flags against 5.4.1 specifically — `--icc-intent PERCEPTUAL` is accepted
there and `INTENT_PERCEPTUAL` is not. Shipping a different darktable would change what customers see
without changing a single line of Boothy. This is also why the installed copy is resolved before any
`ProgramFiles` installation.

## Colour profile

Boothy does not ship a separate ICC file. Output uses darktable's built-in sRGB profile through
`--icc-type SRGB`, so the `color-profile` inventory entry points at this same evidence and is
recorded as embedded in the darktable tree rather than as a file of its own.
