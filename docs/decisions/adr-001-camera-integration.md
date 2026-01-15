# ADR-001: Camera Integration Approach (Sidecar First)

- Status: Accepted
- Date: 2026-01-14

## Context

This repo contains two major reference stacks:

- `reference/camerafunction/` (digiCamControl, .NET/WPF, Canon EDSDK integration, Named Pipe patterns)
- `reference/uxui_presetfunction/` (RapidRAW, Tauri/Rust + React/Vite)

The product UI must be Tauri + React (WPF is prohibited for product UI). We also need a robust, diagnosable capture→transfer-complete→import boundary.

## Decision

Adopt a **headless camera sidecar** approach for MVP and early integration:

- Implement camera control/capture/download in a separate **camera sidecar process** (C#/.NET), using digiCamControl patterns and/or Canon EDSDK wrappers as appropriate.
- Communicate between Boothy (Tauri backend) and sidecar via **versioned IPC** (Named Pipe + JSON-RPC-style messages), with `protocolVersion` and `correlationId` for diagnosis.
- Keep the Boothy Tauri backend focused on:
  - session folder lifecycle
  - file stabilization/import confirmation
  - preset assignment snapshotting
  - emitting UI events

## Rationale

- Leverages the existing digiCamControl ecosystem and its known working behavior for Canon tethering.
- Keeps camera SDK concerns isolated (threading callbacks, interop, driver variability).
- Improves operational reliability: sidecar can be restarted independently and can produce its own logs.
- Simplifies testing: Boothy can run with a mocked sidecar that emits `photoTransferred` events for integration tests.

## Consequences

- We must define and version the IPC contract early.
- We must own packaging of the sidecar with the Tauri installer (EDSDK bundling for internal deployments is defined in `docs/decisions/adr-003-canon-edsdk-bundling.md`).
- Direct Canon EDSDK Rust FFI is explicitly deferred unless sidecar approach is blocked.
