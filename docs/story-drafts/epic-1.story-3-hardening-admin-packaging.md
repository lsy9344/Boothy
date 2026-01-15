# Story 3: Production Hardening + Admin Surface + Packaging/Diagnostics

- Story ID: epic-1.story-3
- Status: Ready
- References: `docs/prd.md`, `docs/architecture.md`

## Summary

Harden error handling, make admin surfaces explicit (camera + editor), and validate Windows packaging/rollback/diagnostics paths for an offline booth environment.

## Acceptance Criteria

1. Given camera disconnect/capture/transfer failures, Boothy surfaces actionable customer-safe messaging and preserves access to existing photos/export.
2. Given admin mode is unlocked, Boothy reveals advanced camera controls and advanced RapidRAW panels consistent with styling and `docs/design_concept.md`.
3. Given an installer build is produced, the packaging includes Boothy + sidecar + required EDSDK DLLs for internal deployments, and documents prerequisites clearly.
4. Given a rollback to a prior installer, user session photos remain intact (no data loss).

## Dependencies

- Story 2 sidecar integration
- EDSDK bundling decision for internal deployments (`docs/decisions/adr-003-canon-edsdk-bundling.md`)
- Entitlement record completed before bundling EDSDK DLLs: `docs/compliance/canon-edsdk-entitlement.md`

## Tasks (suggested)

1. Error/Recovery UX and diagnostics
   - Standardize error codes + customer/admin detail split
   - Ensure logs support correlation across capture→transfer→import→export
2. Admin surface definition
   - Explicit list of admin-only panels and camera controls (no “disabled clutter”)
3. Packaging and rollback plan validation
   - Windows installer output path, resource layout, sidecar bundling
   - Document prerequisites for Canon SDK/driver where required
4. Regression validation
   - Ensure RapidRAW baseline editing/export behavior remains intact

## Readiness Checkpoints (Definition of Ready)

1. **Packaging gate is explicit**
   - Installer bundles camera sidecar + required EDSDK DLLs for internal deployments
   - Owner entitlement record exists before bundling: `docs/compliance/canon-edsdk-entitlement.md`

2. **Minimum regression scenarios are fixed**
   - “Booth-critical flow” smoke scenario list exists and has pass/fail criteria (see below)
   - Rollback procedure is testable on a clean machine (or a VM snapshot) without losing session photos

## Minimum Regression (Must Pass)

1. **Offline smoke**
   - With network disabled: start session → browse existing photo(s) → export image → logs written

2. **Camera failure tolerance**
   - Sidecar not running / crash: app stays up, shows customer-safe error, existing photos remain exportable

3. **Installer contents**
   - NSIS installer produced
   - Installed layout includes x86 sidecar + x86 EDSDK DLLs under app resources (per `docs/architecture.md`)

4. **Rollback**
   - Install version N → create session + photos exist
   - Install version N-1 over it (rollback) → session folder and photos remain intact → app can still browse/export
