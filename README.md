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

## Requirements: Development Machine vs Booth PC

These two lists are different on purpose. A booth PC installs one file and needs nothing else.

### Development machine (building Boothy)

- The desktop baseline for this repository is `Vite react-ts + Tauri 2`.
- Node.js `22.18+` or `24.x`. The release tooling runs `release/build-inventory.ts` directly as
  TypeScript, which needs Node's built-in type stripping.
- `pnpm 10.31.0`.
- Rust with the MSVC toolchain; confirm both `rustc` and `cargo` are on `PATH`.
- Microsoft Visual Studio C++ Build Tools for Windows desktop Rust builds.
- .NET SDK `8.0.x` for the camera helper.
- Microsoft Edge WebView2 runtime — **developers install this themselves**, because a dev build does
  not bundle it.
- Helpful verification commands:
  - `node -v`
  - `pnpm -v`
  - `rustc -V`
  - `cargo -V`
  - `dotnet --list-sdks`
  - `winget list Microsoft.EdgeWebView2Runtime`

### Booth PC (running Boothy)

- Windows x64. **Nothing else.**
- No Node, no Rust, no .NET SDK, no separately installed darktable, no internet.
- The installer carries the WebView2 offline installer, the pinned darktable 5.4.1 tree, and a
  self-contained camera helper. See [`docs/release-baseline.md`](./docs/release-baseline.md) and
  [`release/README.md`](./release/README.md).
- Installation elevates once (the installer is per-machine, because a booth PC is shared equipment
  and the install carries the render engine).
- To check an installation: `Boothy.exe --self-check`. It writes a report and exits — `0` pass,
  `1` inventory mismatch, `2` could not check.

## App Identifier Change

From the first Story 7.7 build the app identifier is `com.boothy.booth`, not the starter default
`com.tauri.dev`. Changing it after a signed build reaches a branch would turn every upgrade into an
unrelated second installation, so it changed while that was still free.

What this does and does not move:

- **Customer photos and session data do not move.** They live in
  `%USERPROFILE%\Pictures\dabi_shoot`, which has nothing to do with the identifier.
- `%LOCALAPPDATA%\<identifier>\dabi_shoot` is only the fallback session root, used when
  `USERPROFILE` is unavailable.
- Sessions left in `%LOCALAPPDATA%\com.tauri.dev\dabi_shoot\` on a development machine will no longer
  be visible. This is stated here rather than changed silently; move that folder if you need it.
