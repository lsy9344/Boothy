# Dedicated Renderer Replacement Latency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce dedicated renderer `replacementMs` for canary preview closes by making the accepted preset-applied artifact cheaper to produce.

**Architecture:** Keep the current dedicated-renderer canary architecture, but align the sidecar preview job with the host's booth-safe preview invocation profile. Prefer small display-sized preset-applied output and allow the sidecar to use an already-visible fast preview raster as its source when available.

**Tech Stack:** Rust, cargo test, standalone sidecar executable, Tauri host preview diagnostics

---

### Task 1: Lock the cheaper sidecar preview behavior in tests

**Files:**
- Modify: `sidecar/dedicated-renderer/main.rs`
- Modify: `src-tauri/src/render/dedicated_renderer.rs`
- Test: `sidecar/dedicated-renderer/main.rs`
- Test: `src-tauri/src/render/dedicated_renderer.rs`

- [ ] Add a failing sidecar test that proves preview invocation uses display-sized export arguments.
- [ ] Add a failing host-side test that proves preview requests can carry a fast-preview raster source for dedicated renderer jobs.
- [ ] Run the targeted tests and confirm they fail for the expected reason.

### Task 2: Implement the lower-cost dedicated renderer preview path

**Files:**
- Modify: `sidecar/dedicated-renderer/main.rs`
- Modify: `src-tauri/src/render/dedicated_renderer.rs`

- [ ] Add request fields and selection logic so the dedicated renderer can use a session-scoped fast preview raster source when present, otherwise fall back to RAW.
- [ ] Mirror the host preview invocation profile in the sidecar: booth-safe width/height caps and stable darktable core arguments.
- [ ] Keep warm-state and accepted-result contracts intact.

### Task 3: Verify the dedicated renderer path still closes truthfully

**Files:**
- Test: `sidecar/dedicated-renderer/main.rs`
- Test: `src-tauri/tests/dedicated_renderer.rs`

- [ ] Run dedicated renderer unit tests.
- [ ] Run the dedicated renderer integration test file in an isolated cargo target directory.
- [ ] Re-check a recent session request payload with the rebuilt sidecar executable to confirm accepted output still closes correctly.
