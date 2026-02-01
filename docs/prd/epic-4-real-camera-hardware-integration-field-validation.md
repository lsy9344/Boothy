# Epic 4: Real Camera Hardware Integration & Field Validation

### Epic Goal

Validate the real Canon hardware E2E loop: Boothy UI capture button OR external shutter remote trigger -> transfer complete -> session `Raw/` download -> stabilization -> auto ingest -> preset snapshot apply -> export, with kiosk-safe behavior and Korean user messaging.

### Notes

- Use the already-verified reference camera stack under `reference/camerafunction/` (digiCamControl patterns/components) rather than re-implementing Canon SDK integration from scratch.

### Stories (Implementation Backlog)

- `docs/stories/4.1.real-camera-capture-transfer-ingest-preset-export.md`
- `docs/stories/4.2.admin-camera-diagnostics-reconnect-workflows.md`
- `docs/stories/4.3.hardware-compatibility-matrix-regression-campaign.md`
- `docs/stories/4.4.camera-realtime-status-powercycle-recovery.md`

#### Story 4.1: Real Camera Capture + Transfer-Complete + Auto Ingest + Preset Apply + Export (E2E)

As a booth operator,
I want to connect a real Canon camera to Boothy and trigger capture from within the app (customer-mode capture button), while also supporting an external shutter remote,
so that each new photo is downloaded into the active session `Raw/`, automatically ingested, has the currently selected preset applied (non-retroactive), and can be exported reliably (offline, kiosk-safe).

**Acceptance Criteria (summary)**:
1. Sidecar supports `mock`/`real` and degrades gracefully if prerequisites are missing (capture disabled; browse/edit/export OK).
2. Customer mode includes a UI capture button that triggers Sidecar `camera.capture`.
3. External shutter remote capture is also supported and detected efficiently (Canon SDK event subscription preferred; poll fallback).
4. Sidecar downloads into session `Raw/` and emits transfer-complete only after full write; Boothy performs stabilization before ingest.
5. Boothy shows kiosk-safe progress states and Korean customer messaging; admin mode can show diagnostics (`correlationId`).

**Integration Verification**:
- IV1: Existing RapidRAW edit/export still works on pre-existing session photos with camera disconnected.
- IV2: Offline policy holds: no network calls are required for capture/import/export workflow.
- IV3: After transfer-complete, a new photo appears in the session list and main viewport; preset snapshot is applied only to newly imported photos.
- IV4: Failure paths (disconnected/transfer failed) show Korean customer-safe messages and do not crash the app.


### Story Manager Handoff

Please use the story files above as the source of truth for implementation sequencing and detailed acceptance criteria. Each story must include integration verification that existing functionality remains intact (brownfield regression) and must preserve the offline/no-account policy. Customer-mode messaging must be Korean-only and customer-safe.
