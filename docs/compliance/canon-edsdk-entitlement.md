# Canon EDSDK Entitlement Record (Internal Deployments)

This document records the owner’s basis/entitlement to bundle Canon EDSDK runtime DLLs for **internal store deployments**.

It is referenced by `docs/decisions/adr-003-canon-edsdk-bundling.md` and is a **must-have prerequisite** before rolling out installers that include EDSDK DLLs.

## Scope

- Distribution type: Internal store PCs (not public distribution)
- Purpose: Canon tethering for Boothy camera sidecar
- Target OS: Windows 11 x64 (all store PCs)

## Owner Confirmation

- [x] I confirm I have the right/entitlement to bundle Canon EDSDK DLLs for internal deployments as described above.
- [x] I understand this repo cannot validate Canon’s terms on my behalf; this record documents my basis and operating constraints.

## Evidence (record references)

Do not paste confidential contract text into the repo. Record a reference to where evidence is stored internally.

- Evidence type (choose all that apply):
  - [ ] Canon EDSDK license agreement / contract
  - [ ] Canon email approval
  - [ ] Canon portal/download terms screenshot
  - [x] Other (describe below)
- Evidence description: Owner confirmation recorded in project chat; supporting documents to be filed internally.
- Evidence location (internal path / vault link / ticket ID): `evidence/compliance/canon-edsdk/` (local folder; not committed to git)
- Evidence date: 2026-01-14
- Approved by (name/role): Owner

## Technical Details (for repeatable packaging)

- Target CPU architecture for camera sidecar (must match EDSDK DLL bitness):
  - [ ] x64
  - [x] x86 (runs on Windows 11 x64)
- DLL set (exact filenames to bundle, as used by digiCamControl reference app):
  - `EDSDK.dll` (x86) — source: `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Application/EDSDK.dll`
  - `EdsImage.dll` (x86) — source: `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Application/EdsImage.dll`
- Source of DLLs (internal storage location): `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Application/` (for development/reference) + internal evidence folder above for rollout packaging inputs

## Operating Constraints (if any)

List any constraints required by your entitlement (e.g., “internal use only”, “no onward redistribution”, “must include notice”, etc.).

- Constraints:
  - Internal store deployments only (no public redistribution)
  - Do not re-share the bundled EDSDK DLLs outside the store deployment process

## Sign-off

- Date: 2026-01-14
- Signer: Owner
