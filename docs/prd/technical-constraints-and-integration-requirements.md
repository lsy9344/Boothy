# Technical Constraints and Integration Requirements

## Existing Technology Stack

**Languages**: TypeScript/React (frontend), Rust (Tauri backend), and a headless camera sidecar (C#/.NET) for Canon EDSDK-based tethering; direct Rust FFI is deferred unless required  
**Frameworks**: Tauri (RapidRAW currently uses `tauri` 2.9), Vite, Tailwind CSS, Framer Motion, Lucide icons  
**Database**: None required for MVP; prefer file-based session metadata (e.g., JSON) stored alongside session folders and/or app data dir  
**Infrastructure**: Windows desktop packaging via Tauri bundler; primary storage is filesystem session folders  
**External Dependencies**: Canon EDSDK (MVP), GPU acceleration via `wgpu` (already present in RapidRAW backend), RapidRAW’s RAW pipeline (`rawler` etc.)

## Integration Approach

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

## Code Organization and Standards

**File Structure Approach**: Start from RapidRAW’s `src/` (React) + `src-tauri/` (Rust) structure and introduce dedicated camera/session modules (e.g., `src-tauri/src/camera/*`, `src-tauri/src/session/*`, `src/components/booth/*`).  
**Naming Conventions**: Match existing RapidRAW conventions (React components in `PascalCase`, hooks `useX`, Rust modules `snake_case`).  
**Coding Standards**: Keep customer-mode UI minimal and mode gating centralized; avoid duplicating business rules between frontend/backend.  
**Documentation Standards**: Keep `docs/design_concept.md` as the authoritative “what is hidden in customer mode”, and ensure PRD + stories reference it.

## Deployment and Operations

**Build Process Integration**: Use the existing RapidRAW Vite + Tauri build pipeline as baseline, adding Canon EDSDK integration and any required runtime dependencies.  
**Deployment Strategy**: Windows installer packaging (Tauri bundler) with clear requirements for Canon camera drivers/SDK.  
**Monitoring and Logging**: Structured logs in the backend for camera connect/capture/transfer/export; surface concise errors in customer mode and detailed diagnostics in admin mode.  
**Configuration Management**: Store settings (admin password hash, default base folder, camera defaults) in app data; store session-specific state in the session folder.

## Licensing & Distribution (MVP)

- **RapidRAW license (AGPL-3.0):** We accept AGPL obligations and will ship license notices + corresponding source for every distributed Boothy build (see `docs/decisions/adr-002-agpl-compliance.md`).
- **Canon EDSDK bundling (internal):** For internal store deployments, the Boothy installer will bundle the required Canon EDSDK DLLs alongside the camera sidecar (see `docs/decisions/adr-003-canon-edsdk-bundling.md`).
- **Release gating:** No external/public release until the owner explicitly approves public distribution scope; internal deployments proceed with the above compliance approach.

## Risk Assessment and Mitigation

**Technical Risks**:

- Canon EDSDK integration complexity in the camera layer (SDK interop, threading/event callbacks, model variability).
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
