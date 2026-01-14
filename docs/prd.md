# Boothy Brownfield Enhancement PRD

## Intro Project Analysis and Context

### Enhancement Complexity Check (Scope Fit)

This effort appears to be a substantial brownfield enhancement/integration (merging two large OSS codebases + new unified product UX + role-based UI), so a full PRD is appropriate. If you intended a smaller 1–2 session change, we should stop and reduce scope before proceeding.

### Existing Project Overview

#### Analysis Source

- IDE-based analysis of the current repo
- Existing brownfield analysis doc: `docs/brownfield-architecture.md`
- Feature/UI change notes: `docs/design_concept.md`

#### Current Project State

Boothy repo currently contains mostly vendored/reference codebases:

- Camera capture/control: `reference/camerafunction/` (digiCamControl 2.0.0, C#/.NET Framework 4.0, WPF)
- RAW editing + presets + high-res JPG export: `reference/uxui_presetfunction/` (RapidRAW, React/Vite + Tauri/Rust)

There is no first-party Boothy app code at the repo root yet. The target product is a single Windows application built with **Tauri + React** (WPF UI is not allowed). The camera reference app is used for feature reference, but the camera UX must be newly designed in Tauri/React and visually consistent with the RapidRAW editing app’s design concept/style.

### Available Documentation Analysis

- [x] Tech Stack Documentation (in `docs/brownfield-architecture.md`)
- [x] Source Tree/Architecture (in `docs/brownfield-architecture.md`)
- [ ] Coding Standards (not found yet in `docs/`)
- [x] API Documentation (partial: digiCamControl Named Pipe / remote cmd notes in `docs/brownfield-architecture.md`)
- [ ] External API Documentation (camera vendor SDK details not fully captured yet)
- [x] UX/UI Guidelines (partial: `docs/design_concept.md`)
- [x] Technical Debt Documentation (in `docs/brownfield-architecture.md`)
- [x] Other: vendored upstream docs in `reference/uxui_presetfunction/README.md`, `reference/uxui_presetfunction/UPSTREAM.md` (and digiCamControl docs under `reference/camerafunction/.../Docs/`)

### Enhancement Scope Definition

#### Enhancement Type (draft)

- [x] Integration with New Systems (camera stack + editor stack)
- [x] New Feature Addition (new first-party Boothy UI + workflows)
- [x] UI/UX Overhaul (admin/customer mode, simplified UI)
- [x] Major Feature Modification (removing/hiding parts of OSS UIs)
- [ ] Performance/Scalability Improvements
- [ ] Technology Stack Upgrade
- [ ] Bug Fix and Stability Improvements
- [ ] Other: TBD

#### Enhancement Description (draft)

Combine camera shooting workflows and RAW editing/export workflows into a single Windows product UX (Tauri + React), with **no WPF UI**. The unified application must:

- Show tethered/captured photos from the camera in real time in the same “center image” area currently used by the editing app.
- Replace the camera app’s bottom “preview strip” of captured photos with the editing app’s folder-based thumbnail list (session photos list).
- Apply the currently selected **PRESET** to incoming photos so the user immediately sees the filtered look.
- Support **customer mode by default**, with an **admin/customer toggle → password** flow to access/administer advanced features.

Unless explicitly called out as removed/hidden, existing capabilities should be preserved (potentially gated to admin mode).

#### Impact Assessment (draft)

- [ ] Minimal Impact (isolated additions)
- [ ] Moderate Impact (some existing code changes)
- [x] Significant Impact (substantial existing code changes)
- [x] Major Impact (architectural changes required)

### Goals and Background Context

#### Goals (draft)

- Provide a single “Boothy” experience spanning capture → real-time review → edit/preset → export.
- Show newly captured tethered photos immediately in the editor’s main view and session list.
- Apply a selected preset automatically to new photos for a consistent booth “look”.
- Keep existing OSS core functionality working, while simplifying customer-facing UI.
- Support role-based UI gating (admin/customer) across camera and editor experiences.
- Use filesystem session folders as the primary integration boundary.
- Ship Windows-only with a reliable install/build path.

#### Background Context (draft)

The repo currently holds two separate OSS stacks that solve key parts of the desired product but with UIs that don’t match the target “photo booth” customer workflow. The enhancement aims to productize these capabilities into a cohesive Tauri/React Windows application, using the OSS projects primarily as capability references/starting points while redesigning the camera UX to match the RapidRAW editing style and integrating camera capture into the editing experience (real-time session feed + preset application).

### Change Log

| Change | Date | Version | Description | Author |
| --- | --- | --- | --- | --- |
| Initial draft | 2026-01-13 | 0.1 | Create brownfield enhancement PRD skeleton + intro analysis | John (PM) |
| Requirements + concept confirmed | 2026-01-13 | 0.2 | Confirm Tauri-only UI, customer/admin gating, realtime tethered import, preset-per-photo, Canon MVP | John (PM) |

## Requirements

These requirements are based on my understanding of your existing system and the clarified concept in this chat. Please review carefully and confirm they align with your project’s reality.

### Functional

1. FR1: The system must deliver a single unified “Boothy” Windows desktop application that combines camera capture (tethering) and RapidRAW editing/preset/export workflows in one UI.
2. FR2: The system must not use WPF for the product UI; the user experience must be implemented in Tauri + React and visually consistent with RapidRAW’s design concept/style.
3. FR3: The system must start each workflow by creating/opening exactly one active session via a user-provided session name, and the session browser must show only that session folder during the session.
4. FR4: The system must display an image in the central main viewport (RapidRAW’s center image area), and selecting a thumbnail must update the central viewport to that photo.
5. FR5: The system must allow a customer-mode user to trigger camera capture (shoot) from within the Boothy UI.
6. FR6: Captured photo files must be saved into the active session folder, and once file transfer to PC completes, the system must automatically detect/import the new photo without requiring manual refresh.
7. FR7: After import, the newest captured photo must appear in the session thumbnail list (replacing the camera app’s bottom preview strip concept) and be visible to the user immediately.
8. FR8: The system must provide PRESET selection in customer mode, and the currently selected preset must be automatically applied to each newly imported photo at the time it arrives.
9. FR9: Changing the selected preset must only affect photos imported after the change; previously imported photos must keep their originally applied preset (no retroactive updates).
10. FR10: The system must persist (at least within the session) the “preset assignment” per photo so export and re-rendering use the correct preset for each photo.
11. FR11: The system must support customer-mode actions: preset selection, capture, thumbnail selection, export (via RapidRAW “Export image” action), delete.
12. FR12: In customer mode, the export UI must be limited to RapidRAW’s “Export image” button (no advanced export options). Export must generate high-resolution JPEG outputs using each photo’s assigned preset and write outputs to a session output location (e.g., under the active session folder). Advanced export controls/options (if any) must be hidden in customer mode and only shown in admin mode.
13. FR13: Delete must remove selected photo file(s) from the active session folder and update the session list accordingly.
14. FR14: Rotate (CW/CCW) must be available in admin mode (hidden in customer mode) and must affect both on-screen preview and exported JPEG result for the rotated photo(s).
15. FR15: Customer mode must be the default on app launch.
16. FR16: Admin mode access must be “toggle → password”; without the correct password the app must remain in customer mode.
17. FR17: In customer mode, advanced/complex camera and editor controls must be hidden (not disabled) according to `docs/design_concept.md`, and those controls must be exposed in admin mode.
18. FR18: In customer-facing photo lists/thumbnail strips, the UI must not show camera metadata overlays (F, ISO, Exposure, FL, EB, histogram); thumbnails should present photos only.
19. FR19: In admin mode, the system must expose the full camera feature set equivalent to the digiCamControl reference (all camera features available, per scope), and advanced editor features, while maintaining RapidRAW-aligned UI style.
20. FR20: The system must surface camera connection state and actionable errors (disconnected, capture failed, transfer failed) without crashing and without blocking browsing/export of existing session photos.
21. FR21: MVP camera support must target Canon cameras, using Canon EDSDK-based capability mapping (digiCamControl as functional reference), with other camera ecosystems deferred until after MVP.

### Non Functional

1. NFR1: Platform must be Windows-only (MVP and initial releases).
2. NFR2: The product UI must be Tauri + React; WPF UI is prohibited.
3. NFR3: Real-time behavior: after file transfer completes, the new photo should appear in the session list within a target latency (proposal: ≤ 1s) and show a preset-applied preview in the main viewport within a target latency (proposal: ≤ 3s) on target hardware.
4. NFR4: Preset application/RAW processing/export must run in background so the UI remains responsive during capture/import/export.
5. NFR5: Data integrity: the system must not lose captured photos; photos must be written to disk before being considered imported, and partial transfers must not produce corrupted imports.
6. NFR6: Admin password must be stored securely (salted hash) and never stored or logged in plaintext.
7. NFR7: The application must work fully offline (no network dependency for core capture/edit/export).
8. NFR8: The system must provide logs/diagnostics for capture/import/export/preset processing sufficient to debug failures in the field.

### Offline / No-Account Policy (MVP)

- The Boothy product build must **not require sign-in** and must **not make any network calls by default** for the core booth workflow (session → capture → ingest → preset → export).
- Any RapidRAW baseline features that rely on network services (e.g., account auth, community pages, auto model downloads, telemetry, update checks) are **out of scope for Boothy MVP** and must be removed or fully disabled in the Boothy build.
- If any optional online feature is retained for admin troubleshooting in later phases, it must be **explicitly opt-in** and must never block/impact customer-mode operation.

### Compatibility Requirements

1. CR1: Existing API compatibility: RapidRAW preset definitions and export behavior must remain compatible (existing presets should still load and produce the same look).
2. CR2: Database/schema compatibility: if persistent storage for settings/session history/photo assignments is introduced, it must support forward/backward-compatible migrations between versions.
3. CR3: UI/UX consistency: new camera capture UX must be visually and interaction-consistent with RapidRAW (shared design system/components; no mixed UI styles).
4. CR4: Integration compatibility: camera→editor integration must follow the agreed session folder contract and detect photos only after transfer completion; no manual “import” step required.

## User Interface Enhancement Goals

### Integration with Existing UI

The product UI should be built by extending/reworking RapidRAW’s existing layout and design system (Tailwind-based styling, iconography, panels), so the new camera UX feels native rather than “bolted on”.

Key integration decisions:

- The “center image” viewport remains the primary focus; the latest tethered photo becomes the current selection and displays there.
- The camera app’s bottom preview strip concept is replaced by RapidRAW’s session-based thumbnail list (folder images list), constrained to a single active session folder in customer mode.
- Customer mode exposes only the booth-operational controls (capture, preset, export image, delete) using large, touch-friendly affordances and minimal panels.
- Admin mode reveals advanced camera controls (full digiCamControl feature scope) and advanced editor/export controls within the same visual language.

### Modified/New Screens and Views

- **Session Start**: enter session name and initialize the session folder (and optionally choose base directory).
- **Main Booth Screen (Customer Mode)**: center image viewport + session thumbnail list + preset selection + capture + “Export image” + delete, plus an admin toggle.
- **Admin Unlock Modal**: toggle → password prompt; on success, reveal admin UI.
- **Admin Mode Panels/Views**:
  - Camera advanced controls (mode/ISO/shutter/etc, advanced properties, and other digiCamControl-equivalent features).
  - RapidRAW advanced panels (metadata/image properties, advanced export options, etc).
  - Maintenance/config screens (password management, storage locations, camera diagnostics).
- **Error/Recovery States**: camera disconnected, capture failed, transfer failed, low disk space, export failed (customer-friendly messaging + admin diagnostics).

### UI Consistency Requirements

- Use RapidRAW’s typography, spacing, color system, icons, and panel behaviors; no “legacy” WPF look/feel.
- Customer mode must hide advanced controls (no “disabled clutter”); admin mode reveals them.
- Thumbnail/preview UI must show photos only (no F/ISO/exposure/histogram overlays).
- Customer-mode flow should be kiosk/booth friendly: minimal clicks, large targets, and clear feedback for capture/import/export progress.

## Technical Constraints and Integration Requirements

### Existing Technology Stack

**Languages**: TypeScript/React (frontend), Rust (Tauri backend), C/C++ FFI surface as needed for Canon EDSDK integration  
**Frameworks**: Tauri (RapidRAW currently uses `tauri` 2.9), Vite, Tailwind CSS, Framer Motion, Lucide icons  
**Database**: None required for MVP; prefer file-based session metadata (e.g., JSON) stored alongside session folders and/or app data dir  
**Infrastructure**: Windows desktop packaging via Tauri bundler; primary storage is filesystem session folders  
**External Dependencies**: Canon EDSDK (MVP), GPU acceleration via `wgpu` (already present in RapidRAW backend), RapidRAW’s RAW pipeline (`rawler` etc.)

### Integration Approach

**Database Integration Strategy**: Avoid a DB for MVP. Store per-photo preset assignment and session state in a deterministic file format (e.g., `session.json`) so sessions are portable and debuggable.  
**API Integration Strategy**: Use Tauri commands/events as the internal API boundary:

- Frontend invokes backend commands for camera connect/capture/export operations.
- Backend emits events for “capture complete”, “file downloaded”, “import ready”, “export complete”, and error states.

**Frontend Integration Strategy**:

- Build on RapidRAW’s existing editor + library/thumbnails model.
- Constrain the library to the active session folder in customer mode.
- On “new photo imported” event, append to the thumbnail list and auto-select it for display in the main viewport.
- Implement customer/admin mode gating as a single source of truth (UI visibility rules driven by mode).

**Testing Integration Strategy**:

- Unit-test session metadata (photo→preset assignment), mode gating logic, and file path/session folder rules.
- Integration-test Tauri command flows with mocked camera backend (simulate transfer-complete events).
- End-to-end smoke tests for core booth flow (start session → capture → auto-import → preset apply → export image).

### Code Organization and Standards

**File Structure Approach**: Start from RapidRAW’s `src/` (React) + `src-tauri/` (Rust) structure and introduce dedicated camera/session modules (e.g., `src-tauri/src/camera/*`, `src-tauri/src/session/*`, `src/components/booth/*`).  
**Naming Conventions**: Match existing RapidRAW conventions (React components in `PascalCase`, hooks `useX`, Rust modules `snake_case`).  
**Coding Standards**: Keep customer-mode UI minimal and mode gating centralized; avoid duplicating business rules between frontend/backend.  
**Documentation Standards**: Keep `docs/design_concept.md` as the authoritative “what is hidden in customer mode”, and ensure PRD + stories reference it.

### Deployment and Operations

**Build Process Integration**: Use the existing RapidRAW Vite + Tauri build pipeline as baseline, adding Canon EDSDK integration and any required runtime dependencies.  
**Deployment Strategy**: Windows installer packaging (Tauri bundler) with clear requirements for Canon camera drivers/SDK.  
**Monitoring and Logging**: Structured logs in the backend for camera connect/capture/transfer/export; surface concise errors in customer mode and detailed diagnostics in admin mode.  
**Configuration Management**: Store settings (admin password hash, default base folder, camera defaults) in app data; store session-specific state in the session folder.

### Licensing & Distribution Gate (MVP)

- **RapidRAW license (AGPL-3.0):** Any distribution of a Boothy build that incorporates RapidRAW-derived code must comply with AGPL obligations (source availability for the distributed build, notices, etc.). If the intended product distribution model conflicts with AGPL, we must either negotiate an alternative license or replace the editor baseline.
- **Canon EDSDK redistribution:** Canon EDSDK redistribution terms must be confirmed before bundling SDK DLLs in an installer. Until confirmed, the MVP installer plan must assume **no Canon SDK DLL redistribution** (explicit prerequisites + user-supplied/installed dependencies).
- **Release gating:** No external/public release until licensing/redistribution constraints are resolved; internal testing/dev releases only.

### Risk Assessment and Mitigation

**Technical Risks**:

- Canon EDSDK integration complexity in Rust (FFI, threading, event callbacks).
- Real-time preset application latency on large RAW files.
- Large functional scope (full digiCamControl feature set) increasing delivery risk.

**Integration Risks**:

- Ensuring “import only after transfer complete” reliably across camera models and edge cases.
- Keeping per-photo preset assignment stable while still allowing rapid iteration on UI/processing.

**Deployment Risks**:

- Shipping with required camera SDK/runtime dependencies and licensing constraints.
- Windows packaging/signing friction and camera driver variability.

**Mitigation Strategies**:

- Spike/prototype the Canon capture→download→event pipeline early; keep camera layer isolated behind an interface so it can be mocked.
- Implement a fast preview pipeline (quick render) and refine in background for responsiveness.
- Deliver in increments: booth-critical flow first; then expand admin-visible digiCamControl feature parity story-by-story.
- Add defensive file integrity checks (size-stable checks, temp filenames) even when using transfer-complete events.

## Epic and Story Structure

**Epic Structure Decision**: Single comprehensive epic.

Rationale (grounded in current repo reality):

- The enhancement is one cohesive outcome: a single Tauri/React Boothy application that unifies camera tethering and RapidRAW editing/export with customer/admin gating.
- The major risks (Canon EDSDK integration, real-time import, per-photo preset assignment, kiosk-safe UI gating) are tightly coupled and should be sequenced within one epic to avoid integration drift.
- We can still manage delivery risk by slicing stories to deliver the booth-critical path first (Canon MVP), then expand admin-visible feature parity toward “all digiCamControl features”.

## Epic 1: Unified Boothy Booth App (Camera + RapidRAW)

### Epic Goal

Deliver a single Windows Tauri/React application that unifies tethered camera capture and RapidRAW-based editing/presets/export into a kiosk-friendly customer workflow with an admin-unlock path, using the session folder as the integration boundary.

### Epic Description

**Existing System Context:**

- Current relevant functionality: digiCamControl 2.0.0 provides camera capture/control reference; RapidRAW provides RAW viewing/editing, presets, and JPG export.
- Technology stack: React/Vite/Tailwind + Tauri (Rust) for the first-party app; Canon EDSDK integration via Rust FFI for MVP camera support.
- Integration points: Tauri commands/events between frontend/backend; filesystem session folders (new photos written/imported, exports written back).

**Enhancement Details:**

- What's being added/changed: a new first-party Boothy UI built on RapidRAW’s design system that supports session creation, real-time photo ingest, preset selection, capture, export, and role-based feature gating (customer by default; admin via password).
- How it integrates: camera backend downloads captures into the active session folder and emits “new photo” events; frontend appends to the session thumbnail list and auto-selects the newest photo; selected preset is applied automatically to newly ingested photos (non-retroactive).
- Success criteria:
  - Start a session → capture → photo appears in the main viewport and session list after transfer completion, with the currently selected preset applied.
  - Customer mode shows only booth-critical actions; admin mode reveals advanced camera and editor capabilities.
  - No WPF UI is used; UI is visually consistent with RapidRAW.

### Stories (High-Level)

1. **Story 1: Booth App Foundation + Mode Gating**
   - Base the app on RapidRAW (Tauri + React) and introduce session folder lifecycle (create/select session, constrain library to active session).
   - Implement customer/admin mode as a centralized source of truth, including admin-unlock (password) UX and “hide not disable” policy.

2. **Story 2: Camera Capture → Session Ingest → Preset Apply**
   - Implement Canon camera connect/capture/download pipeline in Rust (EDSDK) with reliable transfer-complete handling.
   - Emit events to frontend to ingest/append new photos, auto-select newest, and apply the currently selected preset to newly ingested photos.

3. **Story 3: Production Hardening + Admin Feature Surface**
   - Add robust error/recovery states (disconnect, transfer failure, low disk, export failure) with customer-safe messaging and admin diagnostics.
   - Implement admin-only advanced controls/panels and configuration management (password, storage locations, camera defaults), and validate Windows packaging.

### Compatibility Requirements

- [ ] No WPF UI; all UX implemented in Tauri/React
- [ ] Existing RapidRAW RAW pipeline remains the baseline for rendering and export
- [ ] Session folder semantics stay stable and debuggable (portable sessions)
- [ ] UI changes follow RapidRAW patterns and design language
- [ ] Performance impact is minimal for the booth-critical flow (responsive capture→view)

### Risk Mitigation

- **Primary Risk:** Canon EDSDK integration reliability (FFI, threading, callbacks, model variability) impacting the capture→transfer→ingest loop.
- **Mitigation:** isolate camera layer behind an interface; spike early on capture/download/events; add defensive file integrity checks and robust retry/timeout handling; provide a mock camera backend for integration testing.
- **Rollback Plan:** ship with camera features behind an admin toggle/feature flag; allow manual session-folder import to preserve editing/export value if camera integration is blocked.

### Definition of Done

- [ ] Booth-critical flow works end-to-end (start session → capture → auto-ingest → preset apply → export)
- [ ] Customer/admin mode gating is enforced via “hide not disable” rules
- [ ] Integration points (events/commands + session folder) are stable and documented
- [ ] Regression testing verifies existing editing/export remains intact
- [ ] Windows build/package succeeds with documented prerequisites (SDK/driver)

### Story Manager Handoff

Please develop detailed user stories for this brownfield epic. Key considerations:

- This is an enhancement to an existing system running Tauri (Rust) + React/Vite/Tailwind (RapidRAW baseline) with Canon EDSDK camera integration.
- Integration points: Tauri commands/events; filesystem session folders; preset assignment/session metadata stored alongside session folders.
- Existing patterns to follow: RapidRAW UI/design system + app structure; “hide not disable” customer/admin gating.
- Critical compatibility requirements: no WPF UI; keep editing/export pipeline intact; preserve session folder portability.

Each story must include verification that existing functionality remains intact.
