# Boothy Brownfield Architecture Document

## Introduction

This document captures the **current state** of the `Boothy` repository as it exists today (including technical debt, workarounds, and “reference/vendor” code). It is intended as a practical reference for AI agents and developers working in this repo.

### Document Scope

Focused on the two open-source capability areas that will be merged into a new product:

- **Camera capture/control**: `reference/camerafunction/`
- **RAW editing + presets + high-res JPG export**: `reference/uxui_presetfunction/` (currently a pointer to upstream; see below)

A Brownfield Enhancement PRD now exists at `docs/prd.md` (and target architecture at `docs/architecture.md`). Earlier versions of this document predated those artifacts.

### Change Log

| Date       | Version | Description                 | Author  |
| ---------- | ------- | --------------------------- | ------- |
| 2026-01-13 | 1.0     | Initial brownfield analysis | Winston |
| 2026-01-13 | 1.1     | Add merge-scope notes       | Winston |
| 2026-01-14 | 1.2     | Note PRD/TO-BE docs exist   | Winston |

## Project Purpose (User-Provided Requirements)

Build a new software product by combining two open-source systems:

1. **Camera shooting**: connect/operate camera hardware and capture images (incl. RAW capture workflows).
2. **Editing app**: adjust RAW parameters, manage/apply presets/filters, save configurations, and export high-resolution JPG outputs.

### Confirmed Product Decisions (Owner Clarifications)

- **Primary app/runtime**: Tauri + React (the new “Boothy” product UX lives here)
- **Integration style**: filesystem coupling (camera capture writes session outputs; editor imports from session folder; editor exports JPG back to a session/output folder)
- **Target platform**: Windows-only
- **Source layout intent**:
  - Camera OSS lives under `reference/camerafunction/`
  - Editor OSS should live under `reference/uxui_presetfunction/` (not just a link)

## Quick Reference - Key Files and Entry Points

### Repository-Level

- **Project readme**: `README.md`
- **Agent/runtime configuration**: `AGENTS.md`
- **BMAD core config**: `.bmad-core/core-config.yaml`

### Editing App OSS (RapidRAW) (vendored)

Located under: `reference/uxui_presetfunction/`

- **Upstream link**: `reference/uxui_presetfunction/UPSTREAM.md`
- **Frontend build config**: `reference/uxui_presetfunction/package.json`
- **Frontend entry**: `reference/uxui_presetfunction/src/main.tsx`
- **Frontend app root**: `reference/uxui_presetfunction/src/App.tsx`
- **Tauri backend**: `reference/uxui_presetfunction/src-tauri/Cargo.toml`
- **Tauri entry**: `reference/uxui_presetfunction/src-tauri/src/main.rs`

### Camera Control / Photo Booth Reference Stack (vendored)

Located under: `reference/camerafunction/digiCamControl-2.0.0/`

- **Solution (main)**: `reference/camerafunction/digiCamControl-2.0.0/CameraControl.sln`
- **Core “service locator”**: `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Core/ServiceProvider.cs`
- **Settings**: `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Core/Classes/Settings.cs`
- **Photo booth app**: `reference/camerafunction/digiCamControl-2.0.0/PhotoBooth/PhotoBooth.csproj`
  - UI: `reference/camerafunction/digiCamControl-2.0.0/PhotoBooth/PhotoBoothWindow.xaml`
  - Camera adapter: `reference/camerafunction/digiCamControl-2.0.0/PhotoBooth/PhotoBoothCamera.cs`
- **Command-line tool**: `reference/camerafunction/digiCamControl-2.0.0/CameraControlCmd/Program.cs`
- **Remote command tool (Named Pipe)**: `reference/camerafunction/digiCamControl-2.0.0/CameraControlRemoteCmd/CommandProcessor.cs`

## High Level Architecture

### Technical Summary

This repository is currently a **starter shell** for `Boothy` plus **large vendored reference codebases**. Per the stated project intent, `Boothy` will merge:

- a **camera control / capture stack** (currently represented by `reference/camerafunction/…/digiCamControl-2.0.0/`)
- a **RAW editor + presets/export stack** (RapidRAW, vendored under `reference/uxui_presetfunction/`)

1. **BMAD Method scaffolding** (`.bmad-core/`, `AGENTS.md`) for creating PRDs, architecture docs, stories, QA gates, etc.
2. **digiCamControl 2.0.0** (C# / .NET Framework 4.0) vendored under `reference/` including:
   - A WPF desktop app (`CameraControl`)
   - Core libraries (`CameraControl.Core`, `CameraControl.Devices`)
   - A WPF “PhotoBooth” app module (`PhotoBooth`)
   - CLI and remote-command utilities (`CameraControlCmd`, `CameraControlRemoteCmd`)
   - Installer/build tooling (`Setup`, `nsis/`)
   - A large set of bundled binaries and assets (`CameraControl.Application/`, `Docs/`, etc.)
3. **RapidRAW** (React/Vite + Tauri/Rust) vendored under `reference/uxui_presetfunction/`.

There is **no first-party “Boothy” application code** at the repository root yet; the dominant contents are reference/vendor stacks.

### Actual Tech Stack (from repo contents)

| Category | Technology | Version | Evidence |
| --- | --- | --- | --- |
| Agent/Docs Tooling | BMAD Method core | n/a (vendored) | `.bmad-core/` |
| Desktop Camera Control | C# / .NET Framework | v4.0 | `reference/camerafunction/digiCamControl-2.0.0/**.csproj` |
| Desktop UI | WPF (XAML) | .NET 4 era | `CameraControl/`, `PhotoBooth/` projects |
| Logging | log4net | 2.0.4 (package) | `reference/camerafunction/digiCamControl-2.0.0/packages/log4net.2.0.4/` |
| Web UI (RAW editor) | React | 19.2.3 | `reference/uxui_presetfunction/package.json` |
| Frontend build | Vite | 7.3.0 | `reference/uxui_presetfunction/package.json` |
| Styling | Tailwind CSS | 3.4.19 | `reference/uxui_presetfunction/package.json` |
| Desktop wrapper | Tauri | 2.9.x | `reference/uxui_presetfunction/src-tauri/Cargo.toml` |
| Rust toolchain | Rust | 1.92 (min) | `reference/uxui_presetfunction/src-tauri/Cargo.toml` |

### Repository Structure Reality Check

- Type: **Monorepo-like shell** with **vendored third-party repos** under `reference/`
- CI/CD: **No repository-level CI found** (note: `reference/uxui_presetfunction/.github/` exists for the editor OSS)
- Docs: `docs/` contains architecture notes; PRD is not yet created

## Source Tree and Module Organization

### Project Structure (Actual)

```text
Boothy/
├── .bmad-core/                       # BMAD method engine (agents/tasks/templates)
├── AGENTS.md                         # Agent definitions (auto-generated)
├── docs/                             # Project docs (currently minimal)
├── reference/
│   ├── camerafunction/
│   │   └── digiCamControl-2.0.0/      # Large vendored C#/.NET solution + assets
│   └── uxui_presetfunction/
│       └── ...                       # RapidRAW (editor app) source tree (vendored)
└── README.md
```

### Key Modules and Their Purpose (digiCamControl solution)

From `reference/camerafunction/digiCamControl-2.0.0/CameraControl.sln`:

- **CameraControl** (`CameraControl/CameraControl.csproj`): Main WPF application UI.
- **CameraControl.Core** (`CameraControl.Core/CameraControl.Core.csproj`): Shared app core: settings, managers, scripting, IPC.
- **CameraControl.Devices** (`CameraControl.Devices/CameraControl.Devices.csproj`): Camera/device abstraction and drivers.
- **PortableDeviceLib** (`PortableDeviceLib/PortableDeviceLib.csproj`): Portable device integration layer (Windows device APIs).
- **PhotoBooth** (`PhotoBooth/PhotoBooth.csproj`): WPF photo booth UI/workflow using the core/device layers.
- **CameraControlCmd** (`CameraControlCmd/CameraControlCmd.csproj`): Local CLI to control cameras and run scripts.
- **CameraControlRemoteCmd** (`CameraControlRemoteCmd/CameraControlRemoteCmd.csproj`): Remote CLI communicating via Named Pipe (`DCCPipe`).
- **CameraControl.Plugins** (`CameraControl.Plugins/CameraControl.Plugins.csproj`): Plugin definitions/implementations.
- **CameraControl.PluginManager** (`CameraControl.PluginManager/CameraControl.PluginManager.csproj`): Plugin discovery/management UI/tooling.
- **CameraControl.Trigger** (`CameraControl.Trigger/CameraControl.Trigger.csproj`): Trigger system (automation hooks).
- **CameraControl.ServerTest** (`CameraControl.ServerTest/CameraControl.ServerTest.csproj`): Server/web testing harness.
- **CameraControl.Test** (`CameraControl.Test/CameraControl.Test.csproj`): WPF “test” harness (not a standard unit test project).
- **MtpTester** (`MtpTester/MtpTester.csproj`): MTP-specific testing tool.
- **Canon.Eos.Framework** (`Canon.Eos.Framework/Canon.Eos.Framework.csproj`): Canon EOS integration layer.
- **DccObsPlugin** (`DccObsPlugin/DccObsPlugin.csproj`): OBS integration plugin.
- **Setup** (`Setup/Setup.csproj`): Installer packaging project.

### Key Architectural Patterns (observed)

- **Static global service container**: `CameraControl.Core/ServiceProvider.cs` owns global singletons (DeviceManager, PluginManager, Trigger, etc.).
- **Event-driven device lifecycle**: Consumers attach to `ServiceProvider.DeviceManager` events (connect/disconnect, photo captured, etc.).
- **IPC via Named Pipes**: `ServiceProvider` starts a pipe server `DCCPipe`; `CameraControlRemoteCmd` is a pipe client.
- **WPF UI modules**: Main apps are WPF projects (`OutputType=WinExe`) with XAML and code-behind.

## Data Models and APIs

### State and Configuration

- **Settings model**: `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Core/Classes/Settings.cs`
- **Runtime data folders/logs**: `ServiceProvider.Configure()` wires logging to `Settings.DataFolder` (see `ServiceProvider.cs`).

### IPC / Remote Control API (Named Pipe)

- **Pipe name**: `DCCPipe` (created in `ServiceProvider.Configure()`).
- **Client implementation**: `reference/camerafunction/digiCamControl-2.0.0/CameraControlRemoteCmd/CommandProcessor.cs`
- **Command framing**: string prefix format `:;command:dcc;param:{...}` (see `CommandProcessor.Send()`).

## Technical Debt and Known Issues (Repository Reality)

### Critical Technical Debt / Constraints

1. **Keep PRD/Architecture in sync**: `docs/prd.md` and `docs/architecture.md` exist and must remain aligned with the actual repo/product decisions (offline/no-account policy, distribution/licensing constraints, and the “reference vs product code” boundary).
2. **No first-party app code**: `Boothy/` root does not contain a runnable application; most code is vendored reference.
3. **Legacy .NET Framework**: digiCamControl targets **.NET Framework v4.0** and uses WPF; modern .NET SDK workflows do not apply directly.
4. **Large vendored binaries**: `CameraControl.Application/` contains `*.exe`/`*.dll` (e.g., `ffmpeg.exe`, `ngrok.exe`, camera SDK DLLs). This increases repo size and adds trust/supply-chain considerations.
5. **Nested git repo**: `reference/uxui_presetfunction/` contains its own `.git/` in the vendored snapshot (this can complicate parent-repo git operations/submodules).

### Workarounds and Gotchas

- **Platform expectations**: WPF/.NET Framework projects are Windows-first; building in non-Windows environments requires additional setup.
- **Architecture (x86/x64)**: Multiple csproj files set `PlatformTarget` to x86/x64; some camera SDK dependencies may be architecture-specific.
- **NuGet path drift**: Some csproj files reference package folders that are not present (example: `CameraControl.Test` references `packages/log4net.2.0.3`, but the repo contains `packages/log4net.2.0.4`).
- **“Test” naming**: `CameraControl.Test` is a WPF executable project (a test harness), not a conventional automated unit test suite.

## Integration Points and External Dependencies

### digiCamControl Bundled/External Components (examples)

Bundled binaries/assets under `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Application/` include (non-exhaustive):

- `EDSDK.dll`, `EdsImage.dll` (Canon SDK components)
- `ffmpeg.exe` (media tooling)
- `dcraw.exe` (RAW conversion tooling)
- `ngrok.exe` (tunneling tooling)

NuGet packages are vendored under `reference/camerafunction/digiCamControl-2.0.0/packages/` (examples):

- `log4net.2.0.4/`
- `Newtonsoft.Json.8.0.1-beta2/`
- `MahApps.Metro.*` (WPF UI framework)
- `WixSharp.1.0.28.0/` (installer tooling)

### RapidRAW Dependencies (high level)

- React UI with Tailwind + Vite build (`reference/uxui_presetfunction/`)
- Tauri 2 backend in Rust (`reference/uxui_presetfunction/src-tauri/`)
- GPU processing via `wgpu` (see `reference/uxui_presetfunction/src-tauri/Cargo.toml`)

### RapidRAW Code Map (vendored copy present)

Backend (Tauri/Rust) modules of interest:

- **Entry point**: `reference/uxui_presetfunction/src-tauri/src/main.rs`
- **File IO / import-export**: `reference/uxui_presetfunction/src-tauri/src/file_management.rs`
- **RAW processing**: `reference/uxui_presetfunction/src-tauri/src/raw_processing.rs`
- **General image processing**: `reference/uxui_presetfunction/src-tauri/src/image_processing.rs`
- **GPU pipeline**: `reference/uxui_presetfunction/src-tauri/src/gpu_processing.rs`
- **Presets/conversion**: `reference/uxui_presetfunction/src-tauri/src/preset_converter.rs`

Frontend entry points:

- **App**: `reference/uxui_presetfunction/src/App.tsx`
- **Bootstrap**: `reference/uxui_presetfunction/src/main.tsx`

### Planned Product Integration (Not Implemented Yet)

User intent is to build a single “Boothy” experience by combining the two stacks. The repo currently contains the stacks, but no glue/application layer exists at the root.

Confirmed constraints:

- **Primary UI/runtime**: Tauri + React
- **Platform**: Windows-only
- **Integration**: filesystem coupling (session folder)

Integration surface areas:

- **File exchange boundary**: Camera capture produces RAW/JPG into a session folder; editor consumes the RAW and exports a high-res JPG.
- **Session model**: photo booth “session” (capture set, selection, edits, export destination).
- **Preset persistence**: where presets/filters/settings live and how they are versioned/shared.
- **Automation/UX**: whether editing is manual, semi-automatic, or fully automated after capture (Tauri UI drives this).

Selected integration approach:

- **Loose coupling (filesystem)**: camera stack writes to a known session folder; editor opens/watches that folder and exports JPG back.

## Development and Deployment

### Local Development Setup (What Exists)

There is no single root-level build. Work is currently per-subproject:

1. **BMAD docs tooling**: see `AGENTS.md` and `.bmad-core/user-guide.md` for how to generate PRDs/stories/architecture docs.
2. **digiCamControl**:
   - Open `reference/camerafunction/digiCamControl-2.0.0/CameraControl.sln` in Visual Studio.
   - Build the desired configuration (x86/x64).
3. **RapidRAW**:
   - `cd reference/uxui_presetfunction`
   - `npm ci`
   - `npm run dev` (frontend) or `npm run start` (Tauri dev)

`reference/uxui_presetfunction/` is now the canonical location for the editor app source.

### Build and Packaging Reality

- **Windows installer tooling exists** inside digiCamControl:
  - `reference/camerafunction/digiCamControl-2.0.0/Setup/` (WixSharp-based)
  - `reference/camerafunction/digiCamControl-2.0.0/nsis/` (NSIS scripts)

## Testing Reality

- **No repo-level test harness** for `Boothy` at the root.
- **digiCamControl** includes executable harnesses/projects:
  - `CameraControl.Test` (WPF app)
  - `CameraControl.ServerTest`, `MtpTester` (tools/harnesses)
- **RapidRAW** includes lint tooling (ESLint) but no explicit automated test suite found in top-level scripts.

## If Enhancement PRD Provided - Impact Analysis

Not available (no PRD found). Once a PRD exists, this document should be updated to:

- Identify which parts of the vendored stacks are being adopted vs. treated as reference-only
- Define the first-party `Boothy` application boundaries (what is “ours”)
- Establish build/test/CI conventions at the repo root

## Appendix - Useful Commands and Scripts

### BMAD (documentation workflow)

- List available agents: `npx bmad-method list:agents`
- Validate BMAD configuration: `npx bmad-method validate`
- Regenerate BMAD core + AGENTS: `npx bmad-method install -f -i codex`

### RapidRAW (vendored)

- Dev (frontend): `npm run dev`
- Build (frontend): `npm run build`
- Dev (Tauri): `npm run start`
