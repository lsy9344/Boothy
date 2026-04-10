# Session Timing 1 Minute Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Temporarily shorten newly started booth sessions so the real session countdown and capture cutoff happen after one minute.

**Architecture:** The host owns session timing truth in the Rust session manifest builder. Change the default session duration and warning lead there so every new session inherits the shorter window, and update timing-generation tests first to lock the new product behavior.

**Tech Stack:** Rust, cargo test, Tauri host session manifest builder

---

### Task 1: Lock the new timing behavior in tests

**Files:**
- Modify: `src-tauri/tests/session_manifest.rs`
- Test: `src-tauri/tests/session_manifest.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn default_session_timing_uses_a_one_minute_window() {
    let timing = build_default_session_timing_for_mode(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
        "2026-04-10T10:37:00Z",
        false,
    )
    .expect("default timing should resolve");

    assert_eq!(timing.adjusted_end_at, "2026-04-10T10:38:00Z");
    assert_eq!(timing.warning_at, "2026-04-10T10:37:30Z");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test session_manifest default_session_timing_uses_a_one_minute_window`
Expected: `FAIL` because the host still returns the old 15-minute timing.

- [ ] **Step 3: Update the existing test-mode parity test**

```rust
#[test]
fn session_timing_does_not_change_when_test_mode_flag_is_true() {
    let timing = build_default_session_timing_for_mode(
        "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
        "2026-04-10T10:37:00Z",
        true,
    )
    .expect("timing should stay aligned with durable product defaults");

    assert_eq!(timing.adjusted_end_at, "2026-04-10T10:38:00Z");
    assert_eq!(timing.warning_at, "2026-04-10T10:37:30Z");
}
```

- [ ] **Step 4: Run the targeted test file**

Run: `cargo test --test session_manifest`
Expected: timing tests fail until the constants are updated.

### Task 2: Change host timing defaults

**Files:**
- Modify: `src-tauri/src/session/session_manifest.rs`
- Test: `src-tauri/tests/session_manifest.rs`

- [ ] **Step 1: Write the minimal implementation**

```rust
pub const DEFAULT_SESSION_DURATION_SECONDS: u64 = 60;
pub const WARNING_LEAD_SECONDS: u64 = 30;
```

- [ ] **Step 2: Keep the manifest builder flow unchanged**

```rust
let adjusted_end_at_seconds =
    started_at_seconds.saturating_add(DEFAULT_SESSION_DURATION_SECONDS);
let warning_at = unix_seconds_to_rfc3339(
    adjusted_end_at_seconds.saturating_sub(WARNING_LEAD_SECONDS),
);
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test --test session_manifest default_session_timing_uses_a_one_minute_window`
Expected: `PASS`

- [ ] **Step 4: Run the full targeted verification**

Run: `cargo test --test session_manifest`
Expected: `PASS`
