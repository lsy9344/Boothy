# ADR-002: AGPL-3.0 Compliance for Boothy Distributions

- Status: Accepted
- Date: 2026-01-14

## Context

Boothy is planned as a product derived from RapidRAW, which is licensed under **AGPL-3.0** (see `reference/uxui_presetfunction/LICENSE`). Boothy will be distributed to run on multiple Windows PCs (store deployments).

## Decision

We **accept AGPL-3.0 obligations** for all Boothy builds that incorporate RapidRAW-derived code and will implement distribution-time compliance as a first-class requirement.

## Compliance Approach (what we will ship/do)

1. **License notices included with every installer/build**
   - Bundle RapidRAW license text (AGPL-3.0) and third-party notices with the installer.
   - Provide an in-app “Licenses / Source” view (or equivalent) that:
     - states Boothy is based on RapidRAW (AGPL-3.0),
     - provides the source code location for the exact build,
     - lists modifications (high level).

2. **Corresponding Source provided for every distributed build**
   - For each Boothy installer version, publish a matching source snapshot:
     - a git tag in this repo (preferred), and/or
     - a `boothy-src-<version>.zip` bundle produced in CI/release.
   - The source snapshot must include:
     - all Boothy code,
     - any RapidRAW-derived code we ship,
     - build scripts/instructions sufficient to reproduce the build.

3. **Offline-friendly source access**
   - Because deployments may be offline, the installer must also include:
     - a local path to bundled license files, and
     - a “Source” section that points either to:
       - an internal git server URL (if available), or
       - a local/shared file path where the matching source zip is stored.

## Rationale

- Aligns with the user’s stated decision to follow open-source guidance.
- Removes licensing ambiguity as a delivery blocker and makes compliance repeatable.

## Consequences

- Release process must always produce “binary + corresponding source + notices” together.
- The product documentation must clearly describe how to retrieve the exact source for any deployed build.

