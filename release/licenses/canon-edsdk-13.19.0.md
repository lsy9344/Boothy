# Canon EDSDK 13.19.0 — Licensing Evidence

## What Boothy ships

`EDSDK.dll` and `EdsImage.dll` from Canon EDSDK 13.19.0 are copied into the camera helper publish
tree and land at `<install root>\sidecar\canon-helper\`. The EDSDK C# interop source
(`EDSDK.cs`) is compiled into `canon-helper.exe` from the same SDK package.

## Provenance

| Field | Value |
| --- | --- |
| Canon release | EDSDK 13.19.0 |
| Release date | 2025-02-28 |
| Archive SHA256 | `C4F15E9DF3E24D439405828F69C2AD46DAE0490C29022E31DCB31969B3AA699D` |
| Recorded in | `sidecar/canon-helper/vendor/README.md` |

The SDK archive is never committed. `sidecar/canon-helper/vendor/canon-edsdk/` is ignored by git and
the procurement steps live in `release/README.md`.

## Redistribution status — **UNRESOLVED**

Canon distributes the EDSDK under a developer agreement whose redistribution terms are specific to
the licensee. Boothy ships Canon binaries inside an installer, so redistribution rights are required,
not optional.

As of 2026-08-17 this repository does **not** contain:

- a copy of the executed Canon developer agreement, or
- an identified clause permitting redistribution of `EDSDK.dll` / `EdsImage.dll` inside a
  third-party installer.

This is written down rather than assumed. The inventory carries the same state, so a release review
cannot read past it.

### What has to happen before an HV-18A `Go`

1. Locate the executed Canon EDSDK agreement covering this booth deployment.
2. Record here the exact clause and section that permits redistribution, and any attribution or
   notice text it requires.
3. Store the agreement copy outside the repository and record its location and custodian here.
4. If redistribution is **not** permitted, the camera helper cannot ship the Canon binaries and the
   offline installer scope has to change. That is a product decision, not a packaging workaround.

Owner: Noah Lee. Status: **blocked, evidence not yet supplied**.

## Attribution

Canon, EOS, and EDSDK are trademarks of Canon Inc. Boothy is not affiliated with or endorsed by
Canon Inc.
