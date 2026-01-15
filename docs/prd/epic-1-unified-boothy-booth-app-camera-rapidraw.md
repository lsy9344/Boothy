# Epic 1: Unified Boothy Booth App (Camera + RapidRAW)

## Epic Goal

Deliver a single Windows Tauri/React application that unifies tethered camera capture and RapidRAW-based editing/presets/export into a kiosk-friendly customer workflow with an admin-unlock path, using the session folder as the integration boundary.

## Epic Description

**Existing System Context:**

- Current relevant functionality: digiCamControl 2.0.0 provides camera capture/control reference; RapidRAW provides RAW viewing/editing, presets, and JPG export.
- Technology stack: React/Vite/Tailwind + Tauri (Rust) for the first-party app; Canon camera support via a headless sidecar (digiCamControl-derived patterns) and versioned IPC.
- Integration points: Tauri commands/events between frontend/backend; filesystem session folders (new photos written/imported, exports written back).

**Enhancement Details:**

- What's being added/changed: a new first-party Boothy UI built on RapidRAW’s design system that supports session creation, real-time photo ingest, preset selection, capture, export, and role-based feature gating (customer by default; admin via password).
- How it integrates: camera backend downloads captures into the active session folder and emits “new photo” events; frontend appends to the session thumbnail list and auto-selects the newest photo; selected preset is applied automatically to newly ingested photos (non-retroactive).
- Success criteria:
  - Start a session → capture → photo appears in the main viewport and session list after transfer completion, with the currently selected preset applied.
  - Customer mode shows only booth-critical actions; admin mode reveals advanced camera and editor capabilities.
  - No WPF UI is used; UI is visually consistent with RapidRAW.

## Stories (High-Level)

1. **Story 1: Booth App Foundation + Mode Gating**
   - Base the app on RapidRAW (Tauri + React) and introduce session folder lifecycle (create/select session, constrain library to active session).
   - Implement customer/admin mode as a centralized source of truth, including admin-unlock (password) UX and “hide not disable” policy.

2. **Story 2: Camera Capture → Session Ingest → Preset Apply**
   - Implement Canon camera connect/capture/download pipeline in the headless camera sidecar (EDSDK) with reliable transfer-complete handling.
   - Emit events to frontend to ingest/append new photos, auto-select newest, and apply the currently selected preset to newly ingested photos.

3. **Story 3: Production Hardening + Admin Feature Surface**
   - Add robust error/recovery states (disconnect, transfer failure, low disk, export failure) with customer-safe messaging and admin diagnostics.
   - Implement admin-only advanced controls/panels and configuration management (password, storage locations, camera defaults), and validate Windows packaging.

## Compatibility Requirements

- [ ] No WPF UI; all UX implemented in Tauri/React
- [ ] Existing RapidRAW RAW pipeline remains the baseline for rendering and export
- [ ] Session folder semantics stay stable and debuggable (portable sessions)
- [ ] UI changes follow RapidRAW patterns and design language
- [ ] Performance impact is minimal for the booth-critical flow (responsive capture→view)

## Risk Mitigation

- **Primary Risk:** Canon EDSDK integration reliability (threading, callbacks, model variability) impacting the capture→transfer→ingest loop.
- **Mitigation:** isolate camera layer behind an interface; spike early on capture/download/events; add defensive file integrity checks and robust retry/timeout handling; provide a mock camera backend for integration testing.
- **Rollback Plan:** ship with camera features behind an admin toggle/feature flag; allow manual session-folder import to preserve editing/export value if camera integration is blocked.

## Definition of Done

- [ ] Booth-critical flow works end-to-end (start session → capture → auto-ingest → preset apply → export)
- [ ] Customer/admin mode gating is enforced via “hide not disable” rules
- [ ] Integration points (events/commands + session folder) are stable and documented
- [ ] Regression testing verifies existing editing/export remains intact
- [ ] Windows build/package succeeds with documented prerequisites (SDK/driver)

## Story Manager Handoff

Please develop detailed user stories for this brownfield epic. Key considerations:

- This is an enhancement to an existing system running Tauri (Rust) + React/Vite/Tailwind (RapidRAW baseline) with Canon EDSDK camera integration.
- Integration points: Tauri commands/events; filesystem session folders; preset assignment/session metadata stored alongside session folders.
- Existing patterns to follow: RapidRAW UI/design system + app structure; “hide not disable” customer/admin gating.
- Critical compatibility requirements: no WPF UI; keep editing/export pipeline intact; preserve session folder portability.

Each story must include verification that existing functionality remains intact.
