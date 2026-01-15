# ADR-003: Canon EDSDK DLL Bundling for Internal Store Deployments

- Status: Accepted (Internal Deployments Only)
- Date: 2026-01-14

## Context

Canon camera tethering for MVP requires Canon EDSDK components (DLLs) at runtime. Deployments are planned across multiple store PCs, so “manual per-machine SDK setup” increases operational friction and inconsistency.

The repo contains reference materials under `reference/camerafunction/digiCamControl-2.0.0/` which may include Canon-related binaries, but the **right to redistribute Canon SDK binaries** is governed by Canon’s EDSDK terms.

## Decision

For **internal store deployments**, Boothy installers will **bundle the required Canon EDSDK DLLs** so store PCs do not need separate manual SDK installation.

This decision is explicitly scoped to internal deployments and does not imply permission for public redistribution.

## Implementation Notes

- Treat EDSDK DLLs as an explicit “camera runtime dependency” in the Windows installer.
- Enforce architecture compatibility (x64 vs x86) and document which one is supported for MVP.
- Place bundled DLLs adjacent to the camera sidecar executable (or in the same runtime directory) so standard Windows DLL search rules resolve them.

**MVP bitness note:** the EDSDK DLLs currently present in the digiCamControl reference (`EDSDK.dll`, `EdsImage.dll`) are x86. This implies the camera sidecar must target x86 for MVP (it still runs on Windows 11 x64).

## Operational Requirements

- Maintain a single, versioned “camera runtime bundle” (EDSDK DLL set) aligned with each Boothy release.
- Ensure upgrades/rollbacks keep the camera runtime bundle consistent with the installed Boothy version.

## Compliance / Constraint

Boothy must only bundle Canon EDSDK DLLs **under the terms you are entitled to** (e.g., your Canon EDSDK license/agreements). This repo documents the decision and the packaging mechanism, but it cannot validate Canon’s redistribution rights on your behalf.

## Required Evidence Record (Must-Fix Before Rollout)

Before producing or deploying any installer that bundles Canon EDSDK DLLs, maintain an entitlement record:

- `docs/compliance/canon-edsdk-entitlement.md`

This is the project’s operational checkpoint ensuring the “internal deployment bundling” decision is backed by an owner-confirmed basis and an auditable reference to supporting evidence.

Current status:

- Owner confirmation recorded in `docs/compliance/canon-edsdk-entitlement.md`
