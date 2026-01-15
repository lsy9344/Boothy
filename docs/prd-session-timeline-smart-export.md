# Boothy Enhancement PRD: Session Timeline Management & Smart Export

## Document Information

**Enhancement ID:** Enhancement-002
**Version:** 0.1
**Date:** 2026-01-15
**Author:** John (Product Manager)
**Status:** Draft
**Parent PRD:** docs/prd.md (Unified Boothy Booth App v0.2)

---

## Intro Project Analysis and Context

### Enhancement Complexity Check (Scope Fit)

This enhancement introduces **strict session time management for unattended kiosk operation** and **intelligent background export processing**. This is a substantial brownfield enhancement affecting session lifecycle, export pipeline, and navigation flows, justifying a full PRD process.

### Existing Project Overview

#### Analysis Source

- ✅ **Existing PRD available** at: `docs/prd.md` (v0.2, comprehensive baseline PRD for unified Boothy booth app)
- ✅ **Architecture documentation** at: `docs/architecture.md` and sharded sections
- ✅ **Tech stack documentation** at: `docs/architecture/tech-stack.md`
- IDE-based fresh analysis for this enhancement scope

#### Current Project State

**Boothy** is a unified Windows desktop application (Tauri + React) that combines:
- **Tethered camera capture** (Canon EDSDK integration planned/in progress)
- **RapidRAW-based editing, presets, and high-res JPEG export**
- **Session-based workflow** with customer/admin mode gating (customer mode by default, admin via password)

**Current capabilities:**
- Session creation with folder-based organization
- Real-time photo ingest from tethered cameras
- Preset selection and automatic application to newly captured photos
- Export functionality with mode-based feature gating
- Customer-focused kiosk UI with hidden advanced controls in customer mode

**Current tech stack:**
- Tauri 2.9.x (desktop framework)
- React 19.2.3 + TypeScript 5.9.3 + Vite 7.3.0
- Rust backend with tokio async runtime
- RapidRAW image processing pipeline (wgpu, rawler)
- Windows-only target with NSIS packaging

---

### Enhancement Scope Definition

#### Enhancement Type

Based on the feature specification document (`add_funct.md`):

- ✅ **New Feature Addition** (session timeline management system)
- ✅ **Major Feature Modification** (export process rearchitecture with background processing)
- ✅ **UI/UX Overhaul** (navigation flow changes, modal workflows, countdown timers)
- ⬜ Performance/Scalability Improvements (secondary benefit)
- ⬜ Integration with New Systems
- ⬜ Technology Stack Upgrade
- ⬜ Bug Fix and Stability Improvements

#### Enhancement Description

This enhancement introduces **strict session time management for unattended kiosk operation** and **intelligent background export processing** to ensure reliable, predictable booth operation cycles.

**Key additions:**
1. **Hourly session windows** (N:00 to N:50) with real-time countdown display
2. **Time-triggered events** (T-5 warning modal, T-0 forced export/lockout, N:59 reset)
3. **Background pre-processing** of Raw photos during idle time
4. **User-choice export flow** (overwrite all vs. continue from background progress)
5. **Navigation flow optimization** (direct-to-editor on session start, bidirectional editor↔library navigation)

#### Impact Assessment

- ⬜ Minimal Impact (isolated additions)
- ⬜ Moderate Impact (some existing code changes)
- ✅ **Significant Impact** (substantial existing code changes - timer system, export workflow, navigation routing)
- ⬜ Major Impact (architectural changes required)

**Rationale:** This affects multiple subsystems (session lifecycle, export pipeline, UI routing, background processing orchestration) but builds on existing Tauri/React architecture without fundamentally changing the tech stack or data flow patterns.

---

### Goals and Background Context

#### Goals

- Enforce strict **50-minute operating windows per hour** to enable safe unmanned kiosk operation and predictable maintenance/reset cycles
- **Minimize export wait time** by pre-processing photos in background during user idle time
- Provide **user choice** at export time (full re-export vs. incremental completion)
- **Prevent incomplete sessions** by forcing export completion before hourly reset
- Optimize **navigation flow** to reduce friction in customer-facing booth workflow
- Maintain **data integrity** by ensuring all captured photos are exported before session termination

#### Background Context

The current Boothy application supports camera capture and export workflows, but lacks time-bound session management for unattended/kiosk deployments. In unmanned booth scenarios, **predictable session boundaries** are critical for:

- **Operational reliability:** Forcing clean session closure prevents incomplete states and resource leaks
- **Maintenance windows:** The N:50-N:59 gap allows automated cleanup/reset/diagnostics
- **User experience:** Pre-processing photos during idle time eliminates long export waits, and clear time warnings prevent user frustration

This enhancement adapts the existing session and export systems to support **time-boxed operation** while maintaining the existing capture/preset/export core functionality.

---

### Change Log

| Change | Date | Version | Description | Author |
|--------|------|---------|-------------|--------|
| Enhancement PRD initiated | 2026-01-15 | 0.1 | Session timeline & smart export brownfield PRD drafted | John (PM) |
| Human decisions resolved + requirements updated | 2026-01-15 | 0.2 | Added admin override capability, admin settings UI (FR17-18), export destination confirmed (`Jpg` folder), Story 2.2 scope expanded | John (PM) |

---

## Requirements

These requirements are based on analysis of the existing Boothy system and the feature specification document. Please review carefully and confirm they align with your project's operational needs.

### Functional Requirements

#### Session Timeline Management

**FR1: Session Time Window Definition**
The system must define each operational session as a fixed hourly window from **N:00 (top of the hour) to N:50** (50 minutes past the hour), where N represents any hour (0-23).

**FR2: Real-Time Countdown Display**
The system must display a real-time countdown timer in **MM:SS format** in the top section of both the Main Editor and Library screens, showing time remaining until N:50 (end of current session window).

**FR3: Late Entry Time Anchoring**
When a user creates/enters a session after N:00 (e.g., at N:45), the session end time must still be anchored to **N:50 of the current hour**, not N:45 + 50 minutes. The countdown must reflect the actual remaining time to N:50.

**FR4: T-5 Warning Modal** ✅ ENHANCED
When the countdown reaches **05:00** (5 minutes remaining), the system must immediately display a **modal dialog** in the center of the screen with:
- Warning message: "세션 종료가 5분 남았습니다" (configurable via admin settings UI)
- **확인 (Confirm) button** that must be clicked to dismiss the modal and resume work
- In **customer mode:** Modal must block all editing actions until dismissed
- In **admin mode:** Modal displayed but can be dismissed immediately without blocking critical actions

**FR5: T-0 Session Lockout and Forced Export** ✅ ENHANCED
When the countdown reaches **00:00** (N:50), the system must:
- In **customer mode:**
  - Immediately **lock all editing functionality** (capture, preset changes, delete, rotate - read-only mode)
  - **Automatically trigger** the Smart Export Process (Section B) without user interaction
  - Remain locked until export completes and end screen displays
- In **admin mode:**
  - Display lockout notification but allow admin to dismiss/bypass
  - Admin can choose to continue working past N:50 if needed for troubleshooting
  - Log admin override actions for audit trail

**FR6: N:59 Application Reset** ✅ ENHANCED
At **N:59** (59 minutes past the hour), the system must:
- In **customer mode:**
  - Perform a complete application reset (clear session state, reset UI)
  - Return to the **session entry/creation screen** (initial startup state)
  - Be ready for a new session starting at (N+1):00
- In **admin mode:**
  - Display reset notification but allow admin to dismiss/postpone
  - Admin can prevent reset if troubleshooting or testing in progress
  - Log admin override actions for audit trail

**FR7: End Screen Display** ✅ ENHANCED
After export completion (following T-0 forced export) and before N:59 reset, the system must display a **full-screen end message**:
- Default text: "이용해주셔서 감사합니다." (Thank you for using)
- Message text must be configurable via **admin settings UI** (not just config file)
- Admin mode must provide UI to freely edit:
  - End screen message
  - T-5 warning message ("세션 종료가 5분 남았습니다")
  - Session timing parameters (if needed in future)
- No editing or navigation actions permitted during end screen display in customer mode
- User cannot dismiss or bypass this screen in customer mode (automatic transition only at N:59)
- **Admin mode exception:** Admin can bypass end screen and timeline restrictions for troubleshooting

---

#### Smart Export Process

**FR8: Background Raw Folder Monitoring** ✅ DECIDED
The system must continuously monitor the `Raw` folder during user idle periods and **automatically process** newly detected photos in the background:
- Monitor for new image files written to the Raw folder (via Story 1.1's existing photo detection)
- Trigger export processing for detected files **one photo at a time, sequentially**
- Export processed files to the **existing `Jpg` folder** within the session folder structure
- Perform processing without user interaction or UI indication (silent background operation)
- Update internal completion tracking to distinguish "already processed" vs. "pending" photos

**FR9: Export Trigger Conditions**
The Smart Export Process must be triggered in exactly two scenarios:
1. **User-initiated**: User clicks the 'Export' button in the UI
2. **Time-forced**: System reaches T-0 (N:50) and automatically triggers export per FR5

**FR10: Export Decision Modal**
When export is triggered (FR9), the system must immediately display a modal dialog with:
- Message: "이미 대부분의 사진이 내보내기 완료됐어요. 모두 다시 내보낼까요? 이어서 내보낼까요?"
- **Button 1: [모두 덮어쓰기]** (Overwrite All)
- **Button 2: [이어서 내보내기]** (Continue Export)
- Modal must block until user selects an option

**FR11: Overwrite All Export Mode**
When user selects **[모두 덮어쓰기]**, the system must:
- Ignore all background-processed results
- Re-process **every Raw file** in the session from scratch
- Overwrite any existing exported files in the export destination
- Apply current preset/edit settings to all photos

**FR12: Continue Export Mode**
When user selects **[이어서 내보내기]**, the system must:
- Skip all photos already processed by background monitoring (FR8)
- Identify only the **remaining unprocessed Raw files**
- Process only those remaining files
- Preserve all previously exported files (no overwrites of completed exports)

**FR13: Export Progress Visualization**
During export processing (both modes from FR11/FR12), the system must display:
- **Progress bar** or **buffering indicator** showing completion percentage or activity
- Clear visual feedback that processing is ongoing
- Progress indicator must remain visible until export fully completes
- Indicator should show current file being processed (optional but recommended)

---

#### Navigation Flow Enhancements

**FR14: Direct Editor Entry on Session Start**
When a user completes session name entry and initialization, the system must navigate directly to the **Main Editor screen**, bypassing the Library screen.

**FR15: Editor-to-Library Navigation**
The Main Editor screen must display a **gallery/photo album icon** in the right sidebar. Clicking this icon must navigate to the Library screen.

**FR16: Library-to-Editor Navigation**
The Library screen's **HOME button** routing must be changed to navigate to the **Main Editor screen** (not the session entry screen).

---

#### Admin Configuration UI

**FR17: Admin Settings Interface** ✅ NEW REQUIREMENT
The system must provide an **admin-only settings UI** accessible in admin mode that allows free editing of:
- **End screen message** (default: "이용해주셔서 감사합니다.")
- **T-5 warning message** (default: "세션 종료가 5분 남았습니다")
- Settings changes must take effect immediately (no app restart required)
- Settings must be persisted to configuration file
- Settings UI must include:
  - Text input fields with preview
  - Save/Cancel buttons
  - Validation (non-empty messages)
  - Restore defaults button

**FR18: Admin Timeline Override** ✅ NEW REQUIREMENT
In admin mode, the system must allow admins to bypass timeline restrictions:
- Admin can dismiss T-5 warning without blocking
- Admin can bypass T-0 lockout and continue working past N:50
- Admin can prevent/postpone N:59 reset
- All admin overrides must be logged with timestamp and action type for audit trail
- UI must clearly indicate when operating in "admin override mode" (e.g., visual indicator)

---

### Non-Functional Requirements

**NFR1: Countdown Timer Precision**
The countdown timer display must update at minimum **once per second** and maintain accuracy within ±2 seconds of system time over a full 50-minute session.

**NFR2: Modal Response Time**
T-5 warning modal (FR4) and Export Decision modal (FR10) must appear within **500ms** of trigger condition being met.

**NFR3: Background Processing Efficiency**
Background export processing (FR8) must not cause **perceptible UI lag or frame drops** during user interaction (capture, browsing, editing). Processing must be throttled if system resources are constrained.

**NFR4: Export Completion Time**
When using **Continue Export mode** (FR12) after background processing has kept pace with captures, remaining export time must not exceed **10 seconds** for a typical session (target: ~50 photos, normal hardware).

When using **Overwrite All mode** (FR11), full export time must not exceed **5 minutes** for a typical session (50 photos).

**NFR5: Time Synchronization Requirement**
For multi-kiosk deployments, system clocks must be synchronized (NTP or equivalent) to ensure consistent session window boundaries across devices. The application should log warnings if system time drift is detected.

**NFR6: Graceful Degradation**
If background processing (FR8) fails for any photo, the failure must:
- Be logged for diagnostics
- Not crash or hang the application
- Not prevent that photo from being processed during explicit export (FR10-12)
- Optionally display error in admin mode only

**NFR7: Configuration Persistence**
Session timing parameters (currently N:00-N:50-N:59) and end screen message text must be stored in a configuration file accessible to administrators for potential future adjustments without code changes.

---

### Compatibility Requirements

**CR1: Existing Session Model Compatibility**
Session folder structure and metadata (preset assignments, photo states) must remain unchanged. Timeline features must layer on top of existing session management without breaking current session persistence.

**CR2: Export Pipeline Compatibility**
Background export processing must use the **identical export pipeline** as manual exports to ensure output consistency. File format, quality settings, preset application, and EXIF handling must be identical between background and foreground exports.

**CR3: Navigation State Preservation**
Navigation changes (FR14-16) must preserve the ability to access all existing screens and features. Admin mode must still be able to reach Library directly if needed (no functionality removed, only default paths changed).

**CR4: Admin/Customer Mode Integration** ✅ DECIDED
Timeline enforcement and export modals must respect existing admin/customer mode gating:
- Customer mode: Full timeline enforcement as specified (strict, no bypass)
- Admin mode: Timeline warnings displayed and admin can override/extend session
  - Admin can dismiss T-5 warning and continue past N:50
  - Admin can bypass forced export at T-0
  - Admin can prevent N:59 reset if needed for troubleshooting
  - Recommendation: Log admin overrides for audit trail

---

## Technical Constraints and Integration Requirements

### Existing Technology Stack (Reference)

**From current Boothy architecture:**

| Category | Technology | Version | Relevance to Enhancement |
|----------|-----------|---------|--------------------------|
| Desktop Framework | Tauri | 2.9.x | **Timer events, background tasks, IPC for export pipeline** |
| Frontend | React + TypeScript | 19.2.3, 5.9.3 | **Timer display component, modal dialogs, navigation routing** |
| Frontend Build | Vite | 7.3.0 | No changes required |
| Styling | Tailwind CSS | 3.4.19 | **Modal styling, countdown timer UI** |
| Animation | Framer Motion | 12.23.x | **Modal transitions, progress indicators** |
| Backend Language | Rust | 1.92 | **Session timeline logic, background export orchestration** |
| Async Runtime | tokio | 1.x | **Background file monitoring, export job queue** |
| Image Processing | wgpu, rawler | 28.0, (path) | **Export pipeline (existing - reused for background processing)** |
| File System Ops | std::fs, tokio::fs | (stdlib) | **Raw folder monitoring, export file operations** |

### Integration Approach

#### Session Timeline System Integration

**Backend (Rust/Tauri):**
- Implement a `SessionTimer` service that:
  - Starts when session is created/entered
  - Calculates end time as next N:50 boundary based on current wall-clock time
  - Emits events at T-5, T-0, and N:59 using Tauri event system
  - Runs as a tokio background task with 1-second tick resolution

**Frontend (React/TypeScript):**
- Create `SessionCountdown` component displaying MM:SS countdown
- Subscribe to timer events from Rust backend via Tauri event listeners
- Trigger modal displays on T-5 and export decision events
- Implement UI lockout state at T-0 (disable buttons, overlay UI)

**Integration Contract:**
```rust
// Tauri events emitted by backend
"session:timer-tick"        // Every second: { remaining_seconds: u32 }
"session:t-minus-5"         // At 5:00 remaining
"session:t-zero"            // At 0:00 (force export)
"session:reset"             // At N:59 (reset app)
```

---

#### Background Export Processing Integration

**File Monitoring Strategy:**
- **Leverage existing photo detection from Story 1.1** (already implemented and verified)
- Story 1.1 provides new-photo events when photos are added to session folders (from camera capture or other sources)
- Hook into these existing events to trigger background export queue processing
- Maintain a background export queue (tokio channel-based) that receives events from Story 1.1's detection mechanism

**Export State Tracking:**
- Extend session metadata (existing `session.json` pattern) to track:
  ```json
  {
    "photos": [
      {
        "raw_filename": "IMG_0001.CR3",
        "preset_id": "preset-vintage",
        "background_export_completed": true,
        "background_export_timestamp": "2026-01-15T14:23:45Z"
      }
    ]
  }
  ```

**Export Pipeline Reuse:**
- Background processing must call the **same** `export_photo()` function as manual export
- Use existing RapidRAW pipeline (`rawler` + `wgpu` rendering)
- Run background exports at **lower thread priority** to avoid UI impact (NFR3)
- Implement cooperative cancellation if user starts manual export

---

#### Modal Workflow Integration

**Modal System:**
- Leverage existing React modal pattern (if RapidRAW has one) or implement new `<Modal>` component with:
  - Backdrop overlay (blocks interaction)
  - Centered content area
  - Button action handlers
  - Keyboard trap (accessibility)

**Modal Types:**
1. **T-5 Warning Modal** (`<WarningModal>`)
   - Single "확인" button
   - Blocks until dismissed
   - Configurable warning text

2. **Export Decision Modal** (`<ExportDecisionModal>`)
   - Two buttons: "모두 덮어쓰기" / "이어서 내보내기"
   - Returns selection to Rust backend via Tauri command
   - Shows background completion stats (optional enhancement)

3. **End Screen Modal** (`<EndScreenModal>`)
   - Full-screen overlay (not dismissible)
   - Configurable thank-you message
   - Auto-dismisses at N:59 reset event

**Tauri Command Integration:**
```rust
#[tauri::command]
async fn handle_export_decision(choice: ExportChoice) -> Result<(), String> {
  // choice: OverwriteAll | ContinueFromBackground
  match choice {
    OverwriteAll => export_all_photos().await,
    ContinueFromBackground => export_remaining_photos().await,
  }
}
```

---

#### Navigation Flow Integration

**Routing Changes:**
- Modify existing React Router (or routing solution) configuration:

**Before:**
```
SessionEntry -> Library -> Editor
               ↑           |
               └───────────┘ (HOME button)
```

**After:**
```
SessionEntry -> Editor <-> Library
               (direct)  (bidirectional)
```

**Implementation:**
- `SessionEntry` completion: `navigate('/editor')` instead of `navigate('/library')`
- Editor right sidebar: Add `<GalleryIcon onClick={() => navigate('/library')} />`
- Library HOME button: `onClick={() => navigate('/editor')}` instead of `navigate('/')`

**State Preservation:**
- Navigation must preserve session context (current session folder, selected photo, preset)
- Use React Context or state management to maintain session across route changes

---

### Code Organization and Standards

**New Module Structure:**

```
src-tauri/src/
  session/
    timer.rs          // SessionTimer service, event emission
    export_queue.rs   // Background export job queue
    file_monitor.rs   // Raw folder watching with notify crate

src/
  components/
    session/
      SessionCountdown.tsx    // MM:SS countdown display
      WarningModal.tsx        // T-5 warning
      ExportDecisionModal.tsx // Export choice UI
      EndScreenModal.tsx      // Thank you screen
      ExportProgressBar.tsx   // Progress visualization
  hooks/
    useSessionTimer.ts        // Subscribe to timer events
    useExportProgress.ts      // Track export completion
```

**Naming Conventions:**
- Follow existing RapidRAW patterns (PascalCase components, snake_case Rust modules)
- Event names: `session:` prefix for all timeline events
- Commands: `handle_export_decision`, `get_export_status`, etc.

---

### Deployment and Operations

**Configuration File Structure:**

Add new section to existing app config (or create `session-config.json`):

```json
{
  "session_timeline": {
    "session_end_minute": 50,
    "reset_minute": 59,
    "warning_offset_seconds": 300,
    "end_screen_message": "이용해주셔서 감사합니다."
  },
  "background_export": {
    "enabled": true,
    "debounce_seconds": 3,
    "max_concurrent_exports": 1
  }
}
```

**Logging Requirements (NFR8 from baseline PRD):**
- Log all timeline events with precise timestamps
- Log background export start/complete/fail for each photo
- Log export decision choices and processing mode
- Include correlation IDs linking timer events to export operations

**Monitoring:**
- Expose admin-mode diagnostics showing:
  - Current session timeline status
  - Background export queue depth
  - Export completion percentage
  - Failed background exports (if any)

---

### Risk Assessment and Mitigation

**Technical Risks:**

1. **Timer Drift Risk**
   - **Risk:** System clock changes or sleep/suspend could desync timer
   - **Impact:** Sessions might end early/late, or N:59 reset could be missed
   - **Mitigation:**
     - Use wall-clock time checks (not elapsed time counters)
     - Validate clock on every tick (detect jumps)
     - Force immediate reset if clock jumps forward past N:59
     - Log warnings if system clock changes detected

2. **Background Export Race Conditions**
   - **Risk:** User triggers manual export while background export is processing same file
   - **Impact:** File corruption, duplicate processing, wasted CPU
   - **Mitigation:**
     - Use file-level locking or atomic operations
     - Cancel background export task when manual export starts
     - Maintain single source of truth for "currently exporting" state

3. **File Write Detection Accuracy** _(Low Risk - Already Mitigated in Story 1.1)_
   - **Risk:** Detecting "file complete" vs "still writing" could cause background export to start on incomplete files
   - **Impact:** Background export errors, wasted processing cycles
   - **Mitigation:**
     - **Story 1.1 already implemented and verified** file write completion detection for photo ingest
     - Background export hooks into Story 1.1's validated detection mechanism
     - Additional safety: catch and retry on RAW decode errors during background processing
     - Log any detection timing issues for further tuning if needed

4. **Modal Blocking Issues**
   - **Risk:** T-5 modal might appear during critical action (mid-capture), disrupting workflow
   - **Impact:** User frustration, potential data loss if capture interrupted
   - **Mitigation:**
     - Detect in-progress capture and delay modal until capture completes
     - Ensure modal doesn't block background processes (only UI)
     - Provide clear "resuming work" feedback after modal dismissal

**Integration Risks:**

1. **Export Pipeline Consistency**
   - **Risk:** Background export might use different settings/pipeline than manual export
   - **Impact:** Inconsistent output quality, user confusion
   - **Mitigation:**
     - Strict code reuse (single `export_photo()` function)
     - Unit tests validating identical output for both paths
     - Include export method metadata in output EXIF for debugging

2. **Session State Corruption**
   - **Risk:** N:59 reset while export still running could leave files partially exported
   - **Impact:** Data loss, incomplete session
   - **Mitigation:**
     - Block N:59 reset until export completes (up to timeout)
     - If timeout exceeded, log failure and preserve session state on disk
     - Provide recovery mechanism in next session startup

3. **Navigation State Loss**
   - **Risk:** Direct-to-editor routing might break assumptions about Library initialization
   - **Impact:** Missing thumbnails, incorrect session folder, null reference errors
   - **Mitigation:**
     - Ensure session context is fully loaded before routing to Editor
     - Add route guards checking session validity
     - Lazy-load Library data only when Library route accessed

**Deployment Risks:**

1. **Configuration Access**
   - **Risk:** Admins might not be able to locate/edit config file in deployed package
   - **Impact:** Cannot customize end screen message or timing parameters
   - **Mitigation:**
     - Document config file location clearly in deployment guide
     - Provide admin UI for common config changes (optional future enhancement)
     - Include reasonable defaults for all settings

2. **Performance Variability**
   - **Risk:** NFR4 export time targets might not be achievable on low-end hardware
   - **Impact:** Export still running at N:59, data loss risk
   - **Mitigation:**
     - Define minimum hardware requirements in deployment guide
     - Implement "export still running" detection at N:59 with grace period
     - Log performance metrics for tuning

---

## Epic and Story Structure

### Epic Approach

**Epic Structure Decision: Single Comprehensive Epic**

**Rationale (grounded in project analysis):**

This enhancement should be structured as **one cohesive epic** because:

1. **Tightly Coupled Features:** Session timeline management and smart export are interdependent - the timeline system triggers forced export at T-0, and export behavior must be aware of session boundaries. Splitting these into separate epics would create coordination overhead and integration risk.

2. **Shared Technical Foundation:** Both features require:
   - Session lifecycle extensions (timer service, state tracking)
   - Background task orchestration (timer ticks, export queue)
   - Modal workflow patterns (T-5 warning, export decision, end screen)
   - Configuration management (timing parameters, messages)

3. **Sequential Value Delivery:** The features deliver maximum value as a complete package:
   - Timeline without smart export → long waits at forced export time (poor UX)
   - Smart export without timeline → solves latency but doesn't address kiosk operation constraints
   - Together → predictable, fast, reliable kiosk operation

4. **Consistent with Existing Architecture:** The baseline Boothy PRD structures major integrations as single epics (e.g., "Epic 1: Unified Boothy Booth App"). This enhancement follows the same pattern - one coherent capability set.

5. **Risk Management:** Keeping stories within one epic ensures timeline and export testing happens together, preventing integration surprises late in development.

**Story Sequencing Strategy:**

Stories are ordered to:
- Deliver core timeline enforcement first (operational requirement)
- Layer smart export optimizations progressively
- Keep UI/navigation improvements as low-risk final polish
- Ensure each story maintains existing functionality

---

## Epic 2: Session Timeline Management & Smart Export

### Epic Goal

Enable reliable unattended kiosk operation through strict time-boxed sessions (N:00-N:50) with intelligent background export processing that minimizes user wait times while ensuring all photos are exported before session reset.

### Epic Description

**Existing System Context:**

- **Current functionality:** Boothy supports session-based workflows with camera capture, preset application, and export. Sessions currently have no time constraints and export is manual/synchronous. **Story 1.1 has implemented and verified** automatic detection of new photos added to session folders (from camera or other sources).
- **Technology stack:** Tauri 2.9.x + React 19.2.3 + Rust/tokio backend; existing session management via folder-based state; RapidRAW export pipeline.
- **Integration points:** Session lifecycle hooks, Tauri event system (backend→frontend communication), React Router navigation, existing export pipeline, **Story 1.1's photo detection events**.

**Enhancement Details:**

- **What's being added:**
  1. Session timer service (Rust backend) calculating hourly N:00-N:50 windows with event emission at T-5, T-0, N:59
  2. Real-time countdown display (React component) in Editor/Library headers
  3. Modal workflows (T-5 warning, export decision, end screen) with blocking UI and user choice
  4. Background export queue system that **hooks into Story 1.1's existing photo detection**, pre-processing photos during idle time
  5. Export mode selection (overwrite all vs. continue) with progress visualization
  6. Navigation flow optimization (direct-to-editor, bidirectional editor↔library routing)

- **How it integrates:**
  - Timer service runs as tokio background task, emits Tauri events consumed by React components
  - Background export queue **subscribes to Story 1.1's photo detection events**, shares existing `export_photo()` pipeline, maintains state in session metadata JSON
  - Modals use React portal/overlay pattern, block UI actions while preserving background tasks
  - Navigation changes update React Router configuration without altering route guards or session context management
  - All features layer onto existing session folder contract (no schema changes)

- **Success criteria:**
  - Session automatically locks and exports at N:50, resets at N:59
  - User sees countdown timer accurate to ±2 seconds throughout 50-minute session
  - Late entry (e.g., N:45 start) correctly ends at N:50 (not N:45+50)
  - Background processing keeps pace with typical shooting (export completion <10s when using "Continue" mode)
  - Existing capture/preset/manual-export functionality unaffected
  - Navigation changes preserve all screen accessibility

---

### Stories

#### Story 2.1: Session Timeline Core & Warning System

**As a** kiosk operator,
**I want** the Boothy app to enforce strict 50-minute session windows with advance warning,
**so that** sessions complete predictably and users have time to finish their work before forced closure.

**Scope:**
- Implement `SessionTimer` service (Rust) calculating N:00-N:50 windows based on wall-clock time
- Handle late entry (session start after N:00 anchors to same N:50)
- Emit timer events: `session:timer-tick` (1s), `session:t-minus-5`, `session:t-zero`, `session:reset`
- Create `<SessionCountdown>` React component displaying MM:SS countdown in Editor/Library headers
- Implement T-5 warning modal with "확인" button and blocking behavior (customer mode)
- Implement admin mode detection: T-5 modal displays but doesn't block in admin mode
- Subscribe to timer events in frontend and trigger modal display
- Load warning message text from configuration (configurable via admin settings UI in Story 2.2)

**Acceptance Criteria:**
1. Session started at N:00 displays countdown from 50:00 and reaches 00:00 at exactly N:50
2. Session started at N:42 displays countdown from 08:00 and reaches 00:00 at N:50 (not N:42+50)
3. Countdown updates every second with ±2s accuracy over full session
4. At T-5 (05:00), modal appears within 500ms with warning message
5. **Customer mode:** Modal blocks editing actions until user clicks "확인"
6. **Admin mode:** Modal displays but can be dismissed immediately without blocking
7. Countdown timer visible in both Editor and Library screens

**Integration Verification:**
- IV1: Existing session creation and folder initialization workflows function unchanged
- IV2: Timer events do not interfere with camera capture or preset application operations
- IV3: Modal display does not block background Tauri commands or file system operations

**Dependencies:** None (foundation story)

---

#### Story 2.2: T-0 Lockout, Application Reset & Admin Settings

**As a** kiosk operator,
**I want** the app to automatically lock editing and reset at session boundaries,
**so that** the kiosk is ready for the next user and sessions never overlap.

**As an** administrator,
**I want** to bypass timeline restrictions and configure messages via UI,
**so that** I can troubleshoot issues and customize the kiosk experience without editing config files.

**Scope:**
- Implement T-0 (N:50) event handler with customer/admin mode branching:
  - **Customer mode:** Lock UI (read-only mode) - no bypass
  - **Admin mode:** Display lockout notification with option to dismiss/bypass
- Create lockout state management (disable capture/preset/delete/rotate buttons, show overlay)
- Implement N:59 reset handler with customer/admin mode branching:
  - **Customer mode:** Force reset - no bypass
  - **Admin mode:** Display reset notification with option to dismiss/postpone
- Create `<EndScreenModal>` full-screen component with configurable thank-you message
- Load end screen message from config file (`session-config.json`)
- **NEW: Admin Settings UI** - Create admin-only settings panel:
  - Text input for end screen message
  - Text input for T-5 warning message
  - Save/Cancel buttons
  - Validation (non-empty messages)
  - Restore defaults button
  - Live preview of messages
- **NEW: Admin Override Logging** - Log all admin override actions:
  - T-5 modal bypass
  - T-0 lockout bypass
  - N:59 reset postpone
  - Include timestamp and action type
- Display end screen after export completes (placeholder for Story 2.4 export integration)
- Visual indicator when operating in "admin override mode"

**Acceptance Criteria:**
1. **Customer mode - T-0 lockout:**
   - At T-0 (00:00), all editing buttons become disabled and UI shows "session locked" state
   - Capture, preset changes, delete, and rotate actions do not execute in locked state
   - No bypass option available
2. **Admin mode - T-0 lockout:**
   - At T-0, lockout notification displays with "Dismiss" or "Continue Working" button
   - Admin can bypass lockout and continue editing past N:50
   - Bypass action logged with timestamp
3. **Customer mode - N:59 reset:**
   - At N:59, application clears session context and returns to session entry screen
   - No bypass option available
4. **Admin mode - N:59 reset:**
   - At N:59, reset notification displays with "Postpone" or "Reset Now" options
   - Admin can postpone reset to continue troubleshooting
   - Postpone action logged with timestamp
5. **End screen:**
   - End screen displays configurable message ("이용해주셔서 감사합니다." by default)
   - End screen is full-screen and non-dismissible in customer mode
   - Admin mode can bypass end screen
6. **Admin Settings UI:**
   - Settings panel accessible only in admin mode
   - Can edit end screen message and T-5 warning message
   - Changes saved to config file and take effect immediately (no restart)
   - Restore defaults button resets to original Korean messages
   - Input validation prevents empty messages
7. **Admin override indicator:**
   - Visual indicator (e.g., badge, banner) shows when admin has bypassed timeline restrictions
   - Indicator visible throughout session in admin mode

**Integration Verification:**
- IV1: Existing session browsing/viewing remains functional in locked state (photos can be viewed but not modified)
- IV2: Reset does not corrupt session folder data or in-progress file operations
- IV3: New session created after reset loads correctly without residual state from previous session
- IV4: Customer mode behavior unchanged - strict enforcement regardless of admin features
- IV5: Admin override logs can be retrieved for audit purposes

**Dependencies:** Story 2.1 (timer events)

---

#### Story 2.3: Background Export Queue & File Monitoring

**As a** booth user,
**I want** photos to be automatically processed in the background,
**so that** final export is fast and I don't wait unnecessarily.

**Scope:**
- **Leverage existing photo detection from Story 1.1** (session folder new photo monitoring already implemented and verified)
- Hook into existing new-photo events to trigger background export processing
- Create background export queue (tokio channel-based) processing one photo at a time
- Extend session metadata JSON schema to track `background_export_completed` per photo
- Reuse existing `export_photo()` function with lower thread priority for background processing
- Add background export logging (start/complete/fail events with correlation IDs)
- Implement cooperative cancellation when manual export triggered

**Note:** Story 1.1 already implemented and verified the capability to detect new photos added to session folders (whether from camera capture or other sources). This story builds on that foundation by connecting the detection events to background export processing.

**Acceptance Criteria:**
1. When Story 1.1's photo detection fires (new photo added to session folder), background export queue receives the event
2. Background export starts processing within 5 seconds of receiving the photo detection event
3. Background export uses identical pipeline as manual export (output files byte-identical for same input)
4. **Exported files written to existing `{session_folder}/Jpg/` folder**
5. Session metadata correctly tracks which photos have been background-exported
6. Background processing does not cause UI frame drops or input lag during user interaction
7. If background export fails, error is logged and does not crash app
8. Failed background exports are retried during manual export (not silently skipped)

**Integration Verification:**
- IV1: Story 1.1's existing photo detection continues to work for UI updates (thumbnails, main view) while also triggering background export
- IV2: Existing manual export functionality produces identical results (no regression)
- IV3: Background processing respects existing preset assignments (uses preset active when photo was captured)
- IV4: Session metadata file remains valid JSON and loads correctly after app restart

**Dependencies:** Story 1.1 (photo detection mechanism - prerequisite), Story 2.1 (session context), Story 2.2 (for integration testing with locked state)

---

#### Story 2.4: Smart Export Decision & Progress UI

**As a** booth user,
**I want** to choose whether to re-export all photos or only export remaining ones,
**so that** I have control over quality vs. speed tradeoffs.

**Scope:**
- Create `<ExportDecisionModal>` with two buttons: "모두 덮어쓰기" / "이어서 내보내기"
- Implement Tauri command `handle_export_decision(choice)` receiving user selection
- Implement "Overwrite All" mode: re-export all Raw files, ignore background completion state
- Implement "Continue" mode: filter to only non-background-exported photos, process remaining
- Create `<ExportProgressBar>` component showing completion percentage and current file
- Integrate T-0 forced export trigger calling export decision modal automatically
- Add manual export button click to trigger export decision modal
- Update end screen display to appear after export completion

**Acceptance Criteria:**
1. When export triggered (manual or T-0), decision modal appears within 500ms
2. Selecting "모두 덮어쓰기" exports all Raw files regardless of background completion status
3. Selecting "이어서 내보내기" exports only photos where `background_export_completed: false`
4. Progress bar shows real-time completion percentage (e.g., "23/50 photos")
5. Progress bar displays current file being processed
6. Export completion time <10s for "Continue" mode with typical session (50 photos, 80% background-completed)
7. Export completion time <5min for "Overwrite All" mode with typical session
8. End screen appears immediately after export completes
9. T-0 forced export successfully triggers modal and completes before N:59 reset

**Integration Verification:**
- IV1: Exported files written to `{session_folder}/Jpg/` folder (consistent with Story 2.3 background exports)
- IV2: Export operation does not interfere with session folder browsing in locked state
- IV3: Export failures display appropriate error messages and do not leave app in broken state
- IV4: Both "Continue" and "Overwrite All" modes produce files in same destination folder

**Dependencies:** Story 2.2 (T-0 event, end screen trigger), Story 2.3 (background export state)

---

#### Story 2.5: Navigation Flow Optimization

**As a** booth user,
**I want** streamlined navigation between editing and browsing,
**so that** I spend less time clicking through screens.

**Scope:**
- Modify session entry completion handler to navigate to `/editor` instead of `/library`
- Add gallery/photo-album icon to Editor right sidebar
- Wire icon click to navigate to `/library` route
- Update Library "HOME" button routing to navigate to `/editor` instead of `/`
- Ensure session context (current session folder, selected photo, preset) persists across navigation
- Update route guards if needed to handle direct-to-editor entry

**Acceptance Criteria:**
1. Completing session name entry navigates directly to Editor screen (Library screen not shown)
2. Editor displays gallery icon in right sidebar (consistent with existing sidebar icon style)
3. Clicking gallery icon navigates to Library screen
4. Library "HOME" button navigates to Editor screen
5. Navigation preserves selected photo, active preset, and session folder context
6. Countdown timer remains visible and accurate across all navigation transitions
7. All existing screens remain accessible (no functionality removed)

**Integration Verification:**
- IV1: Existing Library functionality (thumbnail grid, sorting, filtering) works when accessed via Editor gallery icon
- IV2: Session state remains consistent when navigating Editor ↔ Library multiple times
- IV3: Admin mode navigation (if different routing rules exist) still functions correctly

**Dependencies:** Story 2.1 (countdown timer must show on both screens)

---

### Story Sequencing Rationale

**Order justification:**

1. **Story 2.1 first:** Establishes timer foundation required by all other stories; low integration risk
2. **Story 2.2 second:** Completes timeline enforcement contract (lockout + reset); testable without export
3. **Story 2.3 third:** Adds performance optimization (background queue) while existing manual export still works
4. **Story 2.4 fourth:** Completes export workflow integration, leveraging background state from Story 2.3
5. **Story 2.5 last:** Low-risk UI polish that doesn't affect core timeline/export functionality

**This sequence allows:**
- Incremental testing at each stage
- Early validation of timeline reliability (critical operational requirement)
- Deferred optimization (Stories 2.3-2.4) until core enforcement works
- Safe navigation changes after timeline behavior stabilized

---

### Compatibility Requirements

- ✅ No breaking changes to existing session folder structure or metadata format (CR1)
- ✅ Background and manual export use identical pipeline (CR2)
- ✅ All existing screens remain accessible via navigation (CR3)
- ✅ Admin/customer mode gating respected throughout timeline features (CR4)

### Risk Mitigation

- **Primary Risk:** Timer drift or system clock changes causing missed resets or incorrect session boundaries
- **Mitigation:** Use wall-clock time validation on every tick; detect and log clock jumps; force immediate recovery actions
- **Secondary Risk:** Background export race conditions with manual export
- **Mitigation:** Atomic state management; cooperative cancellation; single source of truth for export operations
- **Rollback Plan:** Feature flag to disable timeline enforcement; system reverts to manual-only export and unlimited session duration

### Definition of Done

- ✅ All 5 stories completed with acceptance criteria met
- ✅ Timeline enforces N:00-N:50 windows with accurate countdown display
- ✅ T-5 warning, T-0 lockout, and N:59 reset function reliably across multiple sessions
- ✅ Background export processing keeps pace with typical shooting patterns
- ✅ Export decision modal provides working overwrite/continue options with progress feedback
- ✅ Navigation flows optimized without breaking existing functionality
- ✅ Existing capture/preset/export functionality regression-tested and verified
- ✅ Configuration file allows admin customization of timing and messages
- ✅ Comprehensive logging enables field diagnostics

---

## Next Steps

### Story Manager Handoff

Please develop detailed user stories for this brownfield epic. Key considerations:

- This is an enhancement to existing Boothy (Tauri/Rust + React) session and export systems
- Integration points: Tauri event system, tokio background tasks, React Router, existing export pipeline
- Existing patterns to follow: Session folder contract, RapidRAW export pipeline reuse, customer/admin mode gating
- Critical compatibility: No session folder schema changes, identical export output for background/manual processing

Each story must include verification that existing functionality remains intact.

---

## Appendix

### Source Documents

- Feature specification: `add_funct.md` (Korean language requirements)
- Baseline PRD: `docs/prd.md` (v0.2)
- Architecture: `docs/architecture.md`
- Tech stack: `docs/architecture/tech-stack.md`

### Glossary

- **Session Window:** Fixed hourly time period from N:00 to N:50
- **T-5:** 5 minutes before session end (warning trigger point)
- **T-0:** Session end time (forced export trigger point)
- **N:59:** Reset time (9 minutes after session end)
- **Background Export:** Automatic photo processing during idle time
- **Continue Mode:** Export only unprocessed photos (skip background-completed)
- **Overwrite Mode:** Re-export all photos from scratch
