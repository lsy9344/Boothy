# RapidRAW Brownfield Architecture Document

> Source repo: `https://github.com/CyberTimon/RapidRAW`  
> Data sources: `RapidRaw_ingest.txt` + upstream clone at `upstream/RapidRAW` (commit `a931728`). Treat file references as “best-effort” and verify against upstream when implementing.

## Introduction

RapidRAW is a blazingly-fast, non-destructive, GPU-accelerated RAW image editor implemented as a **Tauri 2** desktop app:

- **Frontend**: React + TypeScript + Vite + Tailwind (UI, interaction, state).
- **Backend**: Rust (file IO, image decoding/RAW develop, GPU processing, AI features, export, tagging).

This document captures the **CURRENT STATE** of the RapidRAW codebase (as observed), including real constraints and likely coupling points, to support AI agents working on enhancements and forks (e.g., RapidTetherRAW/Boothy).

### Document Scope

Focused on areas most relevant to PRD `prd-rapidraw-customer-mode-v1.1.1.md`:

- Application shell/UI architecture (to support “Customer Mode” vs “Admin Mode” flows)
- File/library navigation + session folder handling
- Sidecar metadata (non-destructive adjustments) + privacy reset requirements
- Export pipeline (single + batch, “Export Lock” gate)
- Backend command surface (Tauri `invoke` contract)
- Integration points (AI models, ComfyUI, GitHub community presets)

### Change Log

| Date       | Version | Description                 | Author |
| ---------- | ------- | --------------------------- | ------ |
| 2026-01-02 | 1.0     | Initial brownfield analysis | Mary   |
| 2026-01-02 | 1.1     | Verified upstream clone; filled missing entrypoints/API details | Mary   |

## Quick Reference — Key Files and Entry Points

### Frontend

- **HTML entry**: `index.html` (currently references `/src/main.jsx`; repo contains `src/main.tsx`)
- **React entry**: `src/main.tsx`
- **App shell**: `src/App.tsx` (library/editor/community view orchestration; event listeners)
- **App-wide command contract**: `src/components/ui/AppProperties.tsx` (see `Invokes` enum)
- **Editor core UI**: `src/components/panel/Editor.tsx`
- **Folder navigation UI**: `src/components/panel/FolderTree.tsx`
- **Panels**: `src/components/panel/right/*` (Adjustments/Crop/Export/Masks/Metadata/Presets/AI)
- **Adjustments UI**: `src/components/adjustments/*`
- **Core domain types**: `src/utils/adjustments.tsx`
- **Keyboard shortcuts/history**: `src/hooks/useKeyboardShortcuts.tsx`, `src/hooks/useHistoryState.tsx`

### Backend (Rust / Tauri)

- **Tauri entrypoint + invoke registration**: `src-tauri/src/main.rs` (`tauri::Builder`, `AppState`, `generate_handler![...]`)
- **Rust deps & features**: `src-tauri/Cargo.toml`
- **Tauri config**: `src-tauri/tauri.conf.json`, `src-tauri/tauri.linux.conf.json`, `src-tauri/tauri.macos.conf.json`
- **File/library/presets/settings**: `src-tauri/src/file_management.rs` (sidecars, thumbnails, folder tree, import/copy/move, presets/settings)
- **RAW develop**: `src-tauri/src/raw_processing.rs` (uses `rawler`)
- **Image decode + orientation**: `src-tauri/src/image_loader.rs`
- **Image processing + metadata schema**: `src-tauri/src/image_processing.rs` (`ImageMetadata`, CPU ops; calls GPU path)
- **GPU compute pipeline**: `src-tauri/src/gpu_processing.rs` (wgpu + WGSL shaders)
- **AI models (SAM/U2NET/CLIP)**: `src-tauri/src/ai_processing.rs` (downloads models; ONNX via `ort`)
- **ComfyUI integration**: `src-tauri/src/comfyui_connector.rs` (HTTP + WebSocket, workflow JSON)
- **Masks & inpainting/denoise**: `src-tauri/src/mask_generation.rs`, `src-tauri/src/inpainting.rs`, `src-tauri/src/denoising.rs`
- **Culling (quality analysis)**: `src-tauri/src/culling.rs` (`#[tauri::command] cull_images`)
- **Tagging (AI tags + sidecar writes)**: `src-tauri/src/tagging.rs`
- **GPU shaders**: `src-tauri/src/shaders/*.wgsl`
- **Workflows**: `src-tauri/workflows/generative_replace.json`

### CI / Packaging

- **Build workflows**: `.github/workflows/*.yml` (build-focused; little/no tests)
- **Desktop metadata**: `data/io.github.CyberTimon.RapidRAW.desktop`, `data/io.github.CyberTimon.RapidRAW.metainfo.xml`
- **Linux packaging**: `packaging/io.github.CyberTimon.RapidRAW.yml`

## High-Level Architecture

### Components

1. **React UI** renders the application shell (library/editor, panels, modals).
2. UI calls into Rust via **Tauri `invoke()`** using string command names centralized in `Invokes`:
   - `src/components/ui/AppProperties.tsx` → `Invokes.*`
3. Rust backend performs:
   - file IO (read images, write sidecars, manage presets/settings)
   - image decode/RAW develop (RAW → linear image)
   - GPU processing (apply adjustments/masks quickly)
   - AI workflows (mask generation, denoising, tagging, ComfyUI “generative replace”)
4. Rust emits progress events back to UI via `app_handle.emit(...)` (e.g., indexing/culling progress).

### “Invoke” Contract (Frontend ↔ Backend)

- **Source of truth**: `src/components/ui/AppProperties.tsx` (`enum Invokes`)
- Pattern: UI imports `Invokes` and uses:
  - `invoke(Invokes.GeneratePreviewForPath, { ... })`
  - `invoke(Invokes.ApplyAdjustments, { ... })`
  - `invoke(Invokes.BatchExportImages, { ... })`
- This creates **tight coupling**: any rename requires synchronized changes in both TS and Rust.

## Frontend Architecture (React/TS)

### Directory Structure (Observed)

- `src/components/panel/*`: primary “pages” and structural panels
  - `panel/Editor.tsx`: central editing view (zoom/pan, crop, masks, waveform/histogram)
  - `panel/FolderTree.tsx`: folder navigation + pinned folders UI
  - `panel/Filmstrip.tsx`: thumbnail strip/grid (virtualized lists)
  - `panel/right/*`: right-side switchable panels (Adjustments/AI/Crop/Export/Masks/Metadata/Presets)
  - `panel/modals/*`: modal workflows (import/export presets, rename, collage, denoise, panorama, etc.)
- `src/components/adjustments/*`: adjustment controls grouped by domain (Basic/Color/Curves/Details/Effects)
- `src/hooks/*`: UI-specific hooks (history state, keyboard shortcuts, thumbnail generation)
- `src/utils/*`: shared domain objects (adjustments schema, masks utilities, themes/palette)
- `src/context/*`: context menus, tagging submenus
- `src/window/TitleBar.tsx`: custom title bar/window controls

### App Shell (`App.tsx`)

`src/App.tsx` is the orchestration hub:

- Drives view switching:
  - `activeView`: library vs community (`CommunityPage`)
  - `selectedImage` presence: library vs editor (`Editor`)
- Owns most “global” UI state: root path, folder tree, image list, selection, adjustments history, export/import state, and modal state.
- Listens to many backend events via `@tauri-apps/api/event.listen` and updates UI state accordingly (see event list in this document).

### State & Data Flow (Typical)

- “Selected image” + “adjustments” live in React state (often with history/undo via `useHistoryState`).
- When user edits controls:
  - UI updates local state immediately
  - Debounced calls are issued to backend (`invoke(...)`) to generate previews/histograms/etc.
- UI consumes returned image bytes as `Blob` URLs for previews and caches them in-memory.

### Editor Core (`Editor.tsx`)

`src/components/panel/Editor.tsx` is the most central UI component and an integration hotspot:

- Handles:
  - zoom/pan (react-zoom-pan-pinch)
  - crop (react-image-crop)
  - masks (Konva-based tooling via `react-konva` + custom mask models)
  - waveform/histogram (backend-generated data)
  - full screen viewer toggles
- Uses Tauri commands via `invoke` (imported from `@tauri-apps/api/core`).
- Receives many props; orchestration happens in `src/App.tsx` (selected image → editor view, otherwise library/community views).

## Backend Architecture (Rust / Tauri)

### Technology Stack

From `src-tauri/Cargo.toml`:

- Tauri 2.9 + plugins (dialog, fs, os, process, shell, single-instance)
- `wgpu` for GPU compute (shaders in WGSL)
- `rawler` for RAW decode/develop (path dependency)
- `image`/`imageproc` for CPU image ops + encode/decode
- `rayon` for parallelism in CPU tasks
- `ort` + `tokenizers` + `ndarray` for ONNX-based AI inference
- `reqwest` + `tokio-tungstenite` for network integrations (model download, ComfyUI)

### Tauri Entrypoint & Runtime State

`src-tauri/src/main.rs`:

- Registers all Tauri commands via `tauri::generate_handler![...]` (includes functions in `main.rs` plus `image_processing::*`, `file_management::*`, `tagging::*`, `culling::*`).
- Creates and manages global `AppState` (shared caches + task handles).
- Applies runtime environment config from settings:
  - optional `WGPU_BACKEND` override
  - Linux WebKit GPU workarounds (env vars)
  - sets `ORT_DYLIB_PATH` to the packaged ONNX Runtime dylib under the app `resources/` directory

Key `AppState` fields (high-level):

- Image caches: `original_image`, `cached_preview`, `gpu_image_cache`
- GPU: `gpu_context`, `lut_cache`
- AI: `ai_state` + init lock
- Long-running tasks: `export_task_handle`, `indexing_task_handle`
- Results: `panorama_result`, `denoise_result`
- UX wiring: `initial_file_path` (file association/single-instance open), `thumbnail_cancellation_token`

### Core Concepts

#### Sidecar Metadata (Non-Destructive)

`src-tauri/src/image_processing.rs` defines:

```rust
pub struct ImageMetadata {
  pub version: u32,
  pub rating: u8,
  pub adjustments: Value, // JSON (mirrors TS Adjustments shape)
  pub tags: Option<Vec<String>>,
}
```

Observed behavior in `src-tauri/src/tagging.rs`:

- For each image path, a “sidecar path” is derived via `file_management::parse_virtual_path(&path_str)` and stored next to the source file as `*.rrdata`.
  - “Virtual copies” are represented as `path?vc=<id>` and map to sidecars like `<filename>.<id>.rrdata`.
- If sidecar exists, it is read as JSON and updated; otherwise defaults are used.
- Tags are merged/deduped/sorted and written back (`fs::write(sidecar_path, json_string)`).

Implication:

- Adjustments schema is **JSON-based** and effectively versioned only via `ImageMetadata.version`.
- Any fork adding new adjustments/fields should handle:
  - forward/backward compatibility
  - defaulting behavior in TS (`INITIAL_ADJUSTMENTS`) and Rust (missing keys)

#### Persistent Storage & Cache Paths

Key storage locations are managed in `src-tauri/src/file_management.rs` using Tauri path helpers:

- **Sidecars (per image)**: stored alongside source images as `*.rrdata` via `parse_virtual_path(...)`.
- **App settings**: `app_data_dir/settings.json` (see `load_settings`, `save_settings`).
- **User presets**: `app_data_dir/presets/presets.json` (see `load_presets`, `save_presets`).
- **Thumbnail cache**: `app_cache_dir/thumbnails/*.jpg` (content-addressed by a blake3 hash of path + mtime; see `get_thumb_cache_dir`, `get_cache_key_hash`).

Kiosk/privacy-related maintenance commands:

- `clear_thumbnail_cache(app_handle)`: deletes and recreates the thumbnail cache folder (aligned with “reset app state/cache only”).
- `clear_all_sidecars(root_path: String) -> Result<usize, String>`: deletes all `*.rrdata` under a root folder (available upstream, but not aligned with the current Boothy privacy-reset policy).

#### RAW Pipeline

`src-tauri/src/raw_processing.rs`:

- Uses `rawler` to decode RAW + metadata orientation
- Applies highlight “headroom” rescale and compression
- Produces `DynamicImage::ImageRgba32F` (linear floats) + applies EXIF orientation

`src-tauri/src/image_loader.rs`:

- Detects special formats (EXR, QOI)
- For RAW: calls `develop_raw_image(...)` with panic safety
- For non-RAW: decodes via `image::ImageReader` and applies EXIF orientation
- Supports compositing “AI patches” onto base image via masks

#### GPU Processing

`src-tauri/src/gpu_processing.rs`:

- Initializes a singleton-like `GpuContext` (`wgpu::Device` + `Queue`) stored in app state
- Applies adjustments with compute pipelines (WGSL shaders)
- Manages readback from textures (padded row alignment handling)
- Uses half-float (`f16`) conversion for performance

Reality/gotchas:

- GPU path depends heavily on driver/adapter availability; fallbacks and platform quirks matter.
- Some operations are still CPU-side (rotate/crop helpers exist in `image_processing.rs`).

### AI Features

#### Local Model Inference (ONNX)

`src-tauri/src/ai_processing.rs`:

- Downloads ONNX model artifacts from HuggingFace URLs at runtime (with SHA-256 verification constants).
- Uses `ort` sessions + `tokenizers` for inference.

Operational constraint:

- First-run requires network access and disk space for cached models.
- Any kiosk-style “Customer Mode” needs a strategy for offline installs and pre-warming models.

#### ComfyUI (External Workflow Engine)

`src-tauri/src/comfyui_connector.rs`:

- Uploads source image (+ optional mask) to ComfyUI server
- Modifies workflow JSON dynamically (checkpoints/vae/controlnet/sampler steps/prompt)
- Monitors progress over WebSocket until prompt completes
- Downloads result image bytes

Constraints:

- Assumes ComfyUI server is reachable (`ws://{address}/ws?clientId=...`)
- Enforces 100 MiB upload limit; has custom compression logic for large images

### Background Indexing & Tagging

`src-tauri/src/tagging.rs` shows a “background indexing” flow:

- Walk directories for supported images
- For each image:
  - load/derive thumbnail
  - generate AI tags (CLIP)
  - write tags to sidecar
- Emits progress events: `indexing-progress`, `indexing-finished`, and errors

This is relevant for:

- “Session reset” in kiosk usage (stop tasks, clear caches, avoid leaking prior session tags)

## Data Models & APIs

### Adjustments Model (Frontend Source of Truth)

`src/utils/adjustments.tsx` defines the client-side `Adjustments` interface with many fields:

- Basic (exposure/contrast/highlights/shadows/whites/blacks)
- Color (temp/tint/vibrance/saturation/HSL/color grading)
- Details (clarity/dehaze/structure/sharpen/noise reduction, etc.)
- Crop/rotation/flip/orientation
- Masks (`masks: Array<MaskContainer>`)
- AI patches (`aiPatches: Array<AiPatch>`)

Backend stores adjustments as JSON (`serde_json::Value`) inside sidecars, so the TS shape effectively drives backend expectations.

### Backend Command Surface

`src/components/ui/AppProperties.tsx` lists the canonical invoke names (examples):

- `LoadImage`, `LoadMetadata`, `SaveMetadataAndUpdateThumbnail`
- `GeneratePreviewForPath`, `GenerateFullscreenPreview`, `GenerateHistogram`, `GenerateWaveform`
- `ApplyAdjustments`, `ApplyAdjustmentsToPaths`, `ResetAdjustmentsForPaths`
- `ExportImage`, `BatchExportImages`, `CancelExport`, `EstimateExportSize`
- `GenerateAi*Mask`, `InvokeGenerativeReplace*`, `ApplyDenoising`
- `GetFolderTree`, `ListImagesInDir`, `ImportFiles`, `CopyFiles`, `MoveFiles`

Recommendation for contributors:

- Treat `Invokes` as an API contract; update it alongside Rust command registration.
- Prefer adding new invoke names via `Invokes` enum (not ad-hoc strings).

### Export Pipeline (Observed)

Implemented in `src-tauri/src/main.rs`:

- Single export at a time is enforced via `AppState.export_task_handle`.
- `export_image(original_path, output_path, js_adjustments, export_settings)`:
  - composites AI patches, applies adjustments, encodes `jpg/png/tiff`, optionally preserves metadata/strips GPS, then writes output.
- `batch_export_images(output_folder, paths, export_settings, output_format)`:
  - loads each image + its `*.rrdata` sidecar (adjustments), processes in a limited rayon pool, and generates filenames via `file_management::generate_filename_from_template`.
  - supported template placeholders (upstream): `{original_filename}`, `{sequence}`, `{YYYY}`, `{MM}`, `{DD}`, `{hh}`, `{mm}`.
  - Boothy naming requirement can be met by constructing `filename_template` as `{예약ID}-{hh}시-{sequence}` at runtime (inject reservation ID as literal text).
- `cancel_export()` aborts the running task; `estimate_export_size()` produces a predicted byte size for current settings.

Events emitted include: `batch-export-progress`, `export-complete`, `export-complete-with-errors`, `export-error`.

### Event Channels (Backend → Frontend)

Observed `app_handle.emit(...)` usage (non-exhaustive):

- Preview/rendering: `preview-update-final`, `preview-update-uncropped`
- Analysis: `histogram-update`, `waveform-update`
- Thumbnails: `thumbnail-generated`
- AI model downloads: `ai-model-download-start`, `ai-model-download-finish`
- Indexing: `indexing-started`, `indexing-progress`, `indexing-finished`, `indexing-error`
- Import: `import-start`, `import-progress`, `import-complete`, `import-error`
- Export: `batch-export-progress`, `export-complete`, `export-complete-with-errors`, `export-error` (UI also listens for `export-cancelled`; verify backend)
- File open/single-instance: `open-with-file`
- Culling: `culling-start`, `culling-progress`

These event names are also part of the implicit API contract.

## Integration Points & External Dependencies

- **HuggingFace model hosting**: runtime downloads in `src-tauri/src/ai_processing.rs`
- **ComfyUI server**: configurable address; used for generative replace/inpainting workflows
- **GitHub RapidRAW-Presets**:
  - Frontend fetches sample preview image from GitHub raw: `DEFAULT_PREVIEW_IMAGE_URL` in `src/components/panel/CommunityPage.tsx`
  - Presets fetched via backend invoke `FetchCommunityPresets`
- **Clerk**: `@clerk/clerk-react` is wired in `src/App.tsx` (currently with a hardcoded test publishable key; likely for community/auth features)

## Development & Deployment

### Local Development

From `package.json` and Tauri config:

- Frontend dev server: `npm run dev` (Vite)
- Tauri dev: `npm run start` (runs `tauri dev`, uses `devUrl: http://localhost:1420`)
- Production build: `npm run build` then `tauri build` (via CI)

### CI / Release

`.github/workflows/ci.yml` / `release.yml`:

- Matrix builds across Windows/macOS/Linux (incl. some ARM runners)
- Primary quality gate appears to be “build succeeds”; test steps are not prominent in the repo.

## Testing Reality

No dedicated unit/integration test suites are evident in the upstream repo layout; CI focuses on building installers/bundles.

Practical implication for fork work:

- Add “cheap” validation early (lint/typecheck; smoke test critical commands) to prevent regressions.

## Technical Debt & Known Gotchas (Observed / Likely)

1. **Frontend/backend tight coupling via string invokes**: rename drift risk; hard to refactor safely.
2. **Adjustments stored as untyped JSON on backend**: schema evolution must be handled carefully (defaults, versioning, migrations).
3. **GPU pipeline complexity** (`wgpu` + WGSL): driver-specific issues, readback alignment, performance cliffs; needs robust fallback behavior.
4. **Runtime model downloads** (AI): first-run latency, offline installs, and kiosk reliability concerns.
5. **Concurrent background tasks** (rayon + tokio): must coordinate cancellation/cleanup for “session reset” flows.
6. **Centralized backend entrypoint**: a large surface area lives in `src-tauri/src/main.rs` + `src-tauri/src/file_management.rs` (refactors require care).
7. **Frontend entry mismatch**: `index.html` references `/src/main.jsx` while repo entry is `src/main.tsx` (verify dev/build behavior).
8. **Export cancellation UX**: UI listens for `export-cancelled`, but backend `cancel_export` currently only aborts the task (verify expected event behavior).

## If PRD Provided — Impact Analysis (RapidTetherRAW Customer/Admin Mode + Tethering)

PRD reference: `prd-rapidraw-customer-mode-v1.1.1.md`

### Likely Areas to Modify (UI)

- **App shell / state machine**: `src/App.tsx` (drives library vs editor by `selectedImage`, and community vs library by `activeView`)
  - Implement mode switching: `Customer Mode` (railroaded flow) vs `Admin Mode` (PIN gated)
  - Add global session timer and hard transitions (Idle → Setup → Capture → ExportLock → Complete → Reset)
- **Library flow**:
  - `src/components/panel/FolderTree.tsx` (limit navigation; pinned folders; kiosk constraints)
    - Confirmed requirement (Boothy): Customer Mode navigation is **restricted to the active session folder**.
  - `src/components/panel/Filmstrip.tsx` (simplify UI for customer; disable advanced actions)
- **Editor flow**:
  - `src/components/panel/Editor.tsx` (restrict controls in Customer Mode; keep “shoot/review” primitives)
  - Right panels in `src/components/panel/right/*` (Admin-only vs Customer-visible subsets)
- **Modals & dialogs**:
  - Many modals exist in `src/components/modals/*`; Customer Mode will require aggressive suppression or whitelisting.

### Likely Areas to Modify (Backend)

- **Session management primitives**:
  - New commands to create/close sessions, enforce export gates, clear state
  - Leverage/extend existing commands like `ClearThumbnailCache`, export cancellation, etc.
- **Export pipeline**:
  - `ExportImage` / `BatchExportImages` / `EstimateExportSize` / `CancelExport` handlers live in `src-tauri/src/main.rs` (uses `AppState.export_task_handle` + emits export progress/completion events)
  - Implement “ExportLock” behavior: disable capture while exporting; gate next session until `export-complete` (and handle errors/cancel)
- **Privacy reset**:
  - Implement a single “reset session” command that:
    - cancels background tasks (indexing/tagging/export)
    - clears thumbnails/temp files
    - clears in-memory UI state (selected image, navigation state, session state machine)
    - does **not** delete session folder files/sidecars (privacy is handled by session-folder isolation + navigation restriction)

### New Modules Likely Needed (Tethering)

RapidRAW upstream is a file-based editor; Canon tethering (EDSDK) is not present in the observed modules.

For Windows-only fork:

- Add a dedicated backend module (e.g., `src-tauri/src/tethering/*`) to:
  - detect/connect camera
  - trigger capture
  - stream/download newly captured files into the active session folder
  - emit UI events (camera connected, capture progress, errors)
- Add corresponding invoke names to `Invokes` (frontend contract) and register Tauri commands.

### Integration Considerations

- Keep the existing “non-destructive” contract: new captures should immediately get:
  - a sidecar created with default adjustments (`INITIAL_ADJUSTMENTS` parity)
  - thumbnails generated (progressive path already exists)
- Confirmed (Boothy): tethered captures are written directly into the session folder; trigger a targeted refresh (e.g., `ListImagesInDir` + thumbnail generation) on capture-complete events.
- If Customer Mode is meant to be “no filesystem exposure”, strongly isolate session folders and disable “ShowInFinder”.

## Appendix — Practical Commands

From RapidRAW root (upstream):

```bash
npm install
npm run start    # tauri dev (runs vite dev server under the hood)
npm run build    # vite build (tauri build handled separately)
```

## Open Questions (Recommended Clarifications)

1. For Boothy/RapidTetherRAW: should Customer Mode be implemented as a separate route/shell, or as a feature-flagged subset of existing panels?
2. Confirmed: “privacy reset” is **app state/cache only** (do not delete session folder files/sidecars).
3. Confirmed: export filename pattern is **`예약ID-{hh}시-{sequence}`** (reservation ID injected into template at runtime; `{hh}`/`{sequence}` supported upstream).
4. Confirmed: captures are saved **directly into the session folder** (not staged/imported).
