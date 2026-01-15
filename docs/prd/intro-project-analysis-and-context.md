# Intro Project Analysis and Context

## Enhancement Complexity Check (Scope Fit)

This effort appears to be a substantial brownfield enhancement/integration (merging two large OSS codebases + new unified product UX + role-based UI), so a full PRD is appropriate. If you intended a smaller 1–2 session change, we should stop and reduce scope before proceeding.

## Existing Project Overview

### Analysis Source

- IDE-based analysis of the current repo
- Existing brownfield analysis doc: `docs/brownfield-architecture.md`
- Feature/UI change notes: `docs/design_concept.md`

### Current Project State

Boothy repo currently contains mostly vendored/reference codebases:

- Camera capture/control: `reference/camerafunction/` (digiCamControl 2.0.0, C#/.NET Framework 4.0, WPF)
- RAW editing + presets + high-res JPG export: `reference/uxui_presetfunction/` (RapidRAW, React/Vite + Tauri/Rust)

There is no first-party Boothy app code at the repo root yet. The target product is a single Windows application built with **Tauri + React** (WPF UI is not allowed). The camera reference app is used for feature reference, but the camera UX must be newly designed in Tauri/React and visually consistent with the RapidRAW editing app’s design concept/style.

## Available Documentation Analysis

- [x] Tech Stack Documentation (in `docs/brownfield-architecture.md`)
- [x] Source Tree/Architecture (in `docs/brownfield-architecture.md`)
- [x] Coding Standards (in `docs/architecture/coding-standards.md`)
- [x] API Documentation (partial: digiCamControl Named Pipe / remote cmd notes in `docs/brownfield-architecture.md`)
- [ ] External API Documentation (camera vendor SDK details not fully captured yet)
- [x] UX/UI Guidelines (partial: `docs/design_concept.md`)
- [x] Technical Debt Documentation (in `docs/brownfield-architecture.md`)
- [x] Other: vendored upstream docs in `reference/uxui_presetfunction/README.md`, `reference/uxui_presetfunction/UPSTREAM.md` (and digiCamControl docs under `reference/camerafunction/.../Docs/`)

## Enhancement Scope Definition

### Enhancement Type (draft)

- [x] Integration with New Systems (camera stack + editor stack)
- [x] New Feature Addition (new first-party Boothy UI + workflows)
- [x] UI/UX Overhaul (admin/customer mode, simplified UI)
- [x] Major Feature Modification (removing/hiding parts of OSS UIs)
- [ ] Performance/Scalability Improvements
- [ ] Technology Stack Upgrade
- [ ] Bug Fix and Stability Improvements
- [ ] Other: TBD

### Enhancement Description (draft)

Combine camera shooting workflows and RAW editing/export workflows into a single Windows product UX (Tauri + React), with **no WPF UI**. The unified application must:

- Show tethered/captured photos from the camera in real time in the same “center image” area currently used by the editing app.
- Replace the camera app’s bottom “preview strip” of captured photos with the editing app’s folder-based thumbnail list (session photos list).
- Apply the currently selected **PRESET** to incoming photos so the user immediately sees the filtered look.
- Support **customer mode by default**, with an **admin/customer toggle → password** flow to access/administer advanced features.

Unless explicitly called out as removed/hidden, existing capabilities should be preserved (potentially gated to admin mode).

### Impact Assessment (draft)

- [ ] Minimal Impact (isolated additions)
- [ ] Moderate Impact (some existing code changes)
- [x] Significant Impact (substantial existing code changes)
- [x] Major Impact (architectural changes required)

## Goals and Background Context

### Goals (draft)

- Provide a single “Boothy” experience spanning capture → real-time review → edit/preset → export.
- Show newly captured tethered photos immediately in the editor’s main view and session list.
- Apply a selected preset automatically to new photos for a consistent booth “look”.
- Keep existing OSS core functionality working, while simplifying customer-facing UI.
- Support role-based UI gating (admin/customer) across camera and editor experiences.
- Use filesystem session folders as the primary integration boundary.
- Ship Windows-only with a reliable install/build path.

### Background Context (draft)

The repo currently holds two separate OSS stacks that solve key parts of the desired product but with UIs that don’t match the target “photo booth” customer workflow. The enhancement aims to productize these capabilities into a cohesive Tauri/React Windows application, using the OSS projects primarily as capability references/starting points while redesigning the camera UX to match the RapidRAW editing style and integrating camera capture into the editing experience (real-time session feed + preset application).

## Change Log

| Change | Date | Version | Description | Author |
| --- | --- | --- | --- | --- |
| Initial draft | 2026-01-13 | 0.1 | Create brownfield enhancement PRD skeleton + intro analysis | John (PM) |
| Requirements + concept confirmed | 2026-01-13 | 0.2 | Confirm Tauri-only UI, customer/admin gating, realtime tethered import, preset-per-photo, Canon MVP | John (PM) |
