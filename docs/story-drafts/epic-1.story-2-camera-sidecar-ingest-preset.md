# Story 2: Camera Sidecar IPC → Transfer Complete → Ingest + Preset Apply on Import

- Story ID: epic-1.story-2
- Status: Ready
- References: `docs/prd.md`, `docs/architecture.md`, `docs/decisions/adr-001-camera-integration.md`

## Summary

Introduce a headless camera sidecar process and an IPC contract so Boothy can receive “transfer complete” events, ingest new photos reliably, and apply the currently selected preset to each newly imported photo (non-retroactive).

## Acceptance Criteria

1. Given Boothy is running with an active session, when the sidecar reports a `photoTransferred(path)` event, then Boothy confirms the file is stable and imports it into the active session library within the NFR3 latency target (best effort in dev).
2. Given a user changes the selected preset, when subsequent photos are imported, then only those new photos get the new preset assignment (previous photos remain unchanged).
3. Given Boothy is offline, then camera ingest and preset assignment still function (no network dependency).
4. Given the sidecar is not running or crashes, then Boothy shows a customer-safe error and allows browsing/export of existing session photos.

## Dependencies

- Story 1 session lifecycle and file watcher foundation
- Canon EDSDK DLLs will be bundled for internal deployments; development can proceed with a mock sidecar when camera hardware is unavailable

## Tasks (suggested)

1. Define IPC contract (versioned)
   - Message schema + error codes + correlation ID
   - Events: `photoTransferred`, `cameraState`, `captureFailed`, etc.
2. Implement sidecar skeleton
   - Headless process, structured logging, Named Pipe server/client as appropriate
   - Mock mode: emit transfer events from a folder to enable integration tests without camera hardware
3. Integrate Boothy backend with sidecar
   - Start/monitor sidecar, reconnect, handle version mismatch
   - Translate sidecar events into Boothy events and library refresh
4. Implement preset assignment on import
   - Persist per-image preset assignment (append-only metadata; `.rrdata` extension per architecture)
   - Ensure preset changes are not retroactive (FR9)
5. Validation
   - Integration test with mock sidecar emitting events
   - Manual test: capture event → auto-select newest → export uses assigned preset

## Readiness Checkpoints (Definition of Ready)

1. **IPC contract reference is fixed**
   - IPC message shape and versioning follow `docs/architecture.md` (JSON-RPC style + `protocolVersion` + `correlationId`)
   - A mock sidecar mode exists (or is explicitly in scope) to test without camera hardware

2. **Preset assignment semantics are unambiguous**
   - “Preset applies only to newly imported photos” (non-retroactive, FR9)
   - Per-photo assignment is persisted in an append-only way (per `docs/architecture.md`)

3. **Failure behavior is defined**
   - Sidecar down/crash → customer-safe error + existing photo browse/export remains available (FR20)

## Minimum Regression (Must Pass)

- Start session → import existing photo(s) → export works (baseline)
- Simulated `photoTransferred` events import photos without manual refresh and newest auto-selects
- Change selected preset → next import uses new preset assignment, earlier photos remain unchanged

## Rollback / Recovery

- If sidecar is unavailable, Boothy must continue to operate as an editor/export tool on existing session photos.
