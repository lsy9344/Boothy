# Boothy Canonical Document Set

Created: 2026-03-20

This repository now stores its canonical planning and reference documents in the BMAD-aligned paths below so BMAD workflows can discover the right artifacts directly.

## Canonical BMAD Locations

1. `_bmad-output/planning-artifacts/prd.md`
   Product definition, boundaries, FR/NFR, and booth-safe behavior.
2. `_bmad-output/planning-artifacts/architecture.md`
   Technical boundaries, runtime ownership, contracts, and deployment model.
3. `_bmad-output/planning-artifacts/ux-design-specification.md`
   Customer-facing UX constraints, copy rules, and approved interaction direction.
4. `refactoring/2026-03-15-boothy-darktable-agent-foundation.md`
   Locked concept-pivot and agent-facing foundation for darktable, camera boundary, and workflow separation.
5. `docs/release-baseline.md`
   Windows release baseline and active-session-safe release guardrails.

## Supporting Reference

6. `reference/darktable/README.md`
   Supporting reference for darktable artifact strategy, CLI apply path, and operating assumptions.

## Legacy Root Copies

The root-level markdown files remain as legacy import copies for now. BMAD workflows and future edits should treat the paths above as authoritative.

## Reset Baseline Summary

- Customer start input is `name + phone-last-four`.
- Customer selects approved presets from representative preview tiles or sample cuts only.
- Customers never adjust raw values directly.
- Operator and authoring surfaces are inside the same packaged app, gated by admin password and capability checks.
- `boothAlias` is customer-facing; durable storage uses an opaque `sessionId`.
- darktable is the preset authoring truth source and headless render/apply engine.
- darktable/gphoto2 tethering is a camera-boundary candidate reference, not the currently approved camera truth source.
- Camera truth remains owned by a separate camera service boundary in the current baseline.

## Note

`epics`, `stories`, and `sprint-status` are not part of this canonical reset bundle. They should be regenerated against this baseline rather than treated as authority.

## Current Artifact Status

- `_bmad-output/planning-artifacts/prd.md` is the current authoritative PRD and received its latest cleanup pass on 2026-03-20.
- `_bmad-output/planning-artifacts/architecture.md` and `_bmad-output/planning-artifacts/ux-design-specification.md` remain the active companion planning artifacts for this baseline.
- Any existing PRD validation report is supporting review context only; the planning-artifacts documents above remain the source of truth after subsequent cleanup edits.
- `epics`, `stories`, and `sprint-status` are intentionally excluded from the reset baseline and must be regenerated from the current planning artifacts before implementation planning resumes.
- WDS-specific `design-artifacts` are not initialized for this baseline yet, so their absence should not be interpreted as a missing canonical planning artifact set.

## Windows Development Bootstrap Notes

- The desktop baseline for this repository is `Vite react-ts + Tauri 2`.
- Use Node.js `20.19+` or `22.12+` so the current Vite toolchain runs without version drift.
- Install Rust with the MSVC toolchain and confirm both `rustc` and `cargo` are available on `PATH`.
- Install Microsoft Visual Studio C++ Build Tools for Windows desktop Rust builds.
- Install the Microsoft Edge WebView2 runtime because Tauri uses it to render the desktop shell on Windows.
- Helpful verification commands:
  - `node -v`
  - `pnpm -v`
  - `rustc -V`
  - `cargo -V`
  - `winget list Microsoft.EdgeWebView2Runtime`
