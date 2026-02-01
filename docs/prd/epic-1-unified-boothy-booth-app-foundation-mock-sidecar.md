# Epic 1: Unified Boothy Booth App (Foundation + Mock Sidecar)

### Epic Goal

Deliver a single Windows Tauri/React application that unifies session-based ingest, per-photo preset assignment, and RapidRAW export into a kiosk-friendly customer workflow with an admin-unlock path.

### Notes

- Camera integration is abstracted behind a headless sidecar boundary (mock first) to keep the UI and file-ingest pipeline stable.
- MVP supports capture via Boothy customer-mode capture button and via an external shutter remote (camera-side trigger).

### Stories (Implementation Backlog)

- `docs/stories/1.1.booth-app-foundation-session-mode-gating.md`
- `docs/stories/1.2.camera-sidecar-ipc-transfer-ingest-preset-apply.md`
- `docs/stories/1.3.production-hardening-admin-surface-packaging-diagnostics.md`
