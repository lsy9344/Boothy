# Epic 1: Unified Boothy Booth App (Camera + RapidRAW)

- Epic ID: epic-1
- Type: Brownfield enhancement
- Baseline: `reference/uxui_presetfunction` (RapidRAW) + camera reference `reference/camerafunction`

## Goal

Deliver a single Windows Tauri/React application that unifies tethered camera capture and RapidRAW-based editing/presets/export into a kiosk-friendly customer workflow with an admin-unlock path, using the session folder as the integration boundary.

## Key Architectural Decisions

- Camera integration: sidecar-first (`docs/decisions/adr-001-camera-integration.md`)
- Integration boundary: session folder contract (`%USERPROFILE%\\Pictures\\dabi_shoot\\<session>\\{Raw,Jpg}`)
- Preset persistence: per-image assignment stored append-only (RapidRAW `.rrdata` extension, per `docs/architecture.md`)

## Story Sequence (dependency-ordered)

1. Story 1 — App foundation + session lifecycle + mode gating
2. Story 2 — Camera sidecar IPC + transfer-complete events + ingest + preset assignment on import
3. Story 3 — Hardening + admin surface + packaging/rollback + diagnostics

## Human-only Decisions / Inputs (blockers)

- AGPL-3.0 compliance approach accepted (see `docs/decisions/adr-002-agpl-compliance.md`)
- Canon EDSDK bundling for internal deployments accepted (see `docs/decisions/adr-003-canon-edsdk-bundling.md`)
