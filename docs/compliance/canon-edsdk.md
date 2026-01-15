# Canon EDSDK Bundling (Internal Deployments)

Boothy requires Canon EDSDK runtime DLLs for Canon tethering. For internal store deployments, we bundle the required DLLs per `docs/decisions/adr-003-canon-edsdk-bundling.md`.

## Required entitlement record

Before rolling out any installer that bundles EDSDK DLLs, complete:

- `docs/compliance/canon-edsdk-entitlement.md`

## Why bundling exists

- Store PCs should be provisioned consistently.
- Avoid manual SDK placement differences that cause “works on one machine” failures.

## Packaging rule (MVP)

- Bundle the required EDSDK DLL set alongside the camera sidecar executable in the installer output.
- Ensure the DLL architecture matches the sidecar (x64/x86).

## Important constraint

Bundling Canon EDSDK DLLs must be done under the Canon terms you are entitled to. This repo documents *how* we bundle, not Canon’s legal permission.
