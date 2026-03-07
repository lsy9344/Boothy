# Refactoring Research Redesign Work Order

> **For the next session:** Revise the refactoring design documents only. Do not implement product code in this task.

**Goal:** Turn the current research into a single, coherent refactoring design package that clearly recommends the most efficient rebuild path for Boothy.

**Architecture:** The recommended path is not "fix the current sidecar" and not "port an entire reference app." The efficient direction is to keep the product UX/value on the Host/UI side, selectively reuse Host/UI ideas from RapidRAW, and isolate Canon camera behavior behind a small Camera Engine Boundary informed by the Canon seam inside digiCamControl.

**Tech Stack:** Markdown documentation, local repository references, RapidRAW reference (`React + Tauri + Rust`), digiCamControl reference (`C#/.NET Framework + Canon EDSDK wrapper concepts`)

## 1. Mission

The next session should redesign the documentation package in `refactoring/` so that:

1. `research-codex.md` becomes the primary source of truth.
2. The most efficient architecture path is stated without ambiguity.
3. Older competing proposals in `research.md` are either demoted, rewritten as superseded context, or explicitly reconciled.
4. A future implementer can understand exactly which parts of the local references are worth learning from and which parts must not become the product base.

This is a documentation redesign task, not an implementation task.

## 2. Locked Recommendation

The next session should preserve this core conclusion unless new local evidence disproves it:

> **Most efficient path:** `RapidRAW Host/UI selective reuse + new Canon-focused Camera Engine Boundary`

Meaning:

- Keep the desktop product centered on the Host/UI and booth result pipeline.
- Reuse RapidRAW selectively for Host/UI ideas or code where it clearly accelerates delivery.
- Treat digiCamControl as a camera-flow research source, not as a full application base.
- If Canon support is the priority, focus extraction analysis on:
  - `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/*`
  - `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/Canon/CanonSDKBase.cs`
  - `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices.Example/Form1.cs`
  - `reference/camerafunction/digiCamControl-2.0.0/CameraControlCmd/Program.cs`
- Do not use the entire digiCamControl solution or the entire `CameraDeviceManager` as the product base.

## 3. Why This Is the Most Efficient Path

The next session should make these efficiency arguments explicit in the revised docs.

### 3.1 It preserves the highest-value reusable asset

The strongest existing reusable asset is not old camera plumbing. It is the customer-facing Host/UI flow and result display experience from the RapidRAW-side reference.

### 3.2 It avoids the most expensive rewrite

Rewriting the product around native desktop UI stacks such as WPF/WinUI or around a new shell like Photino would force unnecessary replacement of the existing web UI strengths.

### 3.3 It keeps camera complexity isolated

Canon/USB/session recovery/live-view edge cases are real, but they do not belong in the editor host. A separate camera boundary limits damage and keeps future replacement possible.

### 3.4 It uses the proven seam in the camera reference

The local camera reference already shows:

- a thin example flow from `connect -> capture -> photo event -> transfer`
- a CLI/headless flow for capture and transfer
- a Canon-specific wrapper seam separate from the full WPF app

This makes Canon-only extraction or reconstitution realistic without inheriting the entire legacy solution.

### 3.5 It avoids paying for unused 80%

The user has already stated that most of the camera reference functionality is unnecessary. The revised docs should treat that as a design constraint, not a cleanup task for later.

## 4. What Must Be Rejected or Demoted

The next session should explicitly mark the following as non-primary paths unless new evidence appears.

### 4.1 Current-stack stabilization as the main strategy

Do not let the document drift back into "fix the current Rust/Tauri/C# sidecar architecture first" as the primary recommendation.

### 4.2 Whole-app digiCamControl adoption

Do not suggest using the full digiCamControl app, WPF UI, or broad device manager stack as the product core.

### 4.3 Full native desktop rewrite

Do not suggest replacing the product UI with a full WPF/WinUI rewrite.

### 4.4 Photino-first or HTTP-sidecar-first as the main redesign

`refactoring/research.md` currently recommends:

- a Photino.NET single-process direction
- or modernizing the current sidecar via HTTP

Those ideas may remain historical context, but they should not stay positioned as the primary forward path if `research-codex.md` remains the main redesign document.

## 5. Document Redesign Tasks

The next session should perform the following documentation tasks.

### Task 1: Establish document precedence

**Files:**
- Modify: `refactoring/research-codex.md`
- Modify or rewrite: `refactoring/research.md`

**Required outcome:**

- `research-codex.md` is clearly the primary redesign document.
- `research.md` no longer competes with it as an equally current recommendation.

**Suggested edits:**

- Add a short note near the top of `research-codex.md` stating that it is the primary greenfield redesign basis for future work.
- Rewrite `research.md` into one of these forms:
  - `superseded research summary`
  - `historical alternatives and why they were not chosen`
  - `legacy stabilization paths, not current preferred direction`

### Task 2: Add an explicit "Most Efficient Path" section

**Files:**
- Modify: `refactoring/research-codex.md`

**Required outcome:**

- A reader can answer, in one section, why the recommended path is more efficient than:
  - current-sidecar stabilization
  - Photino/shell replacement
  - full native rewrite
  - whole-app digiCamControl reuse
  - fully fresh Host rewrite

**Suggested content:**

- development speed
- reuse leverage
- risk isolation
- replaceability
- beginner-friendly implementation sequencing

### Task 3: Make the Canon extraction seam concrete

**Files:**
- Modify: `refactoring/research-codex.md`

**Required outcome:**

- The document should name the specific Canon-side source files that justify the extraction strategy.
- It should explain why those files are promising and why `CameraDeviceManager` is too broad.

**Minimum references to mention:**

- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices.Example/Form1.cs`
- `reference/camerafunction/digiCamControl-2.0.0/CameraControlCmd/Program.cs`
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/Canon/CanonSDKBase.cs`
- `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/*`
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/CameraDeviceManager.cs`

### Task 4: Separate "product direction" from "prototype path"

**Files:**
- Modify: `refactoring/research-codex.md`

**Required outcome:**

- The document should distinguish:
  - final product direction
  - short spike path
  - what is merely a temporary learning scaffold

**This matters because:**

- `Canon.Eos.Framework` or `CanonSDKBase` may be useful as spike foundations
- but that does not automatically make them permanent product architecture

### Task 5: Clarify what "independent product" means

**Files:**
- Modify: `refactoring/research-codex.md`

**Required outcome:**

- The revised docs must explain that "independent product" does not mean "write everything from zero."
- It means:
  - new product structure
  - new boundary
  - own public contract
  - selective internal reuse only where justified

### Task 6: Tighten the anti-patterns and non-goals

**Files:**
- Modify: `refactoring/research-codex.md`

**Required outcome:**

- The revised anti-patterns should explicitly block:
  - full-solution reuse from digiCamControl
  - letting Host absorb Canon SDK complexity
  - reviving old IPC stabilization as the central plan
  - assuming reference UI or reference camera code can be adopted unchanged

## 6. Inputs the Next Session Must Read

Read in this order before rewriting the docs:

1. `refactoring/research-redesign-workorder.md`
2. `refactoring/research-codex.md`
3. `refactoring/research.md`
4. `reference/uxui_presetfunction/README.md`
5. `reference/uxui_presetfunction/src/App.tsx`
6. `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices.Example/Form1.cs`
7. `reference/camerafunction/digiCamControl-2.0.0/CameraControlCmd/Program.cs`
8. `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/Canon/CanonSDKBase.cs`
9. `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/CameraDeviceManager.cs`
10. `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/EosCamera.cs`

## 7. Deliverables Expected From the Next Session

The next session should leave behind:

1. A revised `refactoring/research-codex.md` that is cleaner, more directive, and easier for a future implementer to follow.
2. A resolved status for `refactoring/research.md` so it no longer conflicts with the primary recommendation.
3. If necessary, one additional short support document in `refactoring/` only if it reduces ambiguity. Do not create extra docs without a concrete need.

## 8. Acceptance Criteria

The redesign work is complete only if a new reader can answer all of the following from the docs alone:

1. What is the primary recommended architecture path?
2. Why is it more efficient than fixing the current sidecar stack?
3. Why is it more efficient than a Photino or native desktop rewrite?
4. Which exact camera reference files justify Canon-focused extraction?
5. Why is `CameraDeviceManager` too broad to become the product base?
6. What is reusable from RapidRAW, and what is not?
7. What is the first spike path before any real implementation begins?

## 9. Out of Scope

The next session should not:

- implement product code
- rewrite the current app
- build a new camera engine
- modernize the sidecar transport
- change reference source code

This work order is for documentation redesign only.

## 10. Summary for the Next Session

If the next session does only three things, it should do these:

1. Make `research-codex.md` the undisputed primary redesign document.
2. Explicitly lock the efficient path as `RapidRAW Host/UI selective reuse + Canon-focused Camera Engine Boundary`.
3. Resolve the conflict with `research.md` so future work does not oscillate between incompatible architectural directions.
