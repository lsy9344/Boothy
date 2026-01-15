# Epic 2: Session Timeline Management & Smart Export

- Epic ID: epic-2
- Type: Brownfield enhancement
- Baseline: Existing Boothy session management + export pipeline (from Epic 1)
- Parent PRD: `docs/prd-session-timeline-smart-export.md`

## Goal

Enable reliable unattended kiosk operation through strict time-boxed sessions (N:00-N:50) with intelligent background export processing that minimizes user wait times while ensuring all photos are exported before session reset.

## Key Architectural Decisions

- Session timing: Hourly fixed windows (N:00-N:50) using wall-clock time, not relative duration
- Background processing: Hooks into Story 1.1's existing photo detection events (no new file monitoring)
- Export state tracking: Extend session metadata JSON with `background_export_completed` per photo
- Timer implementation: Rust tokio background task emitting Tauri events (1s tick resolution)
- Modal strategy: React portal/overlay pattern blocking UI but not background tasks
- Navigation: Direct-to-editor entry, bidirectional editor↔library routing

## Story Sequence (dependency-ordered)

1. **Story 2.1** — Session Timeline Core & Warning System
   - Foundation story (no dependencies)
   - Implements SessionTimer service (Rust), countdown display (React), T-5 warning modal
   - Includes basic customer/admin mode branching for T-5 modal behavior

2. **Story 2.2** — T-0 Lockout, Application Reset & Admin Settings ✅ EXPANDED
   - Dependencies: Story 2.1 (timer events)
   - Implements UI lockout at N:50, N:59 reset, configurable end screen
   - **NEW:** Admin override capability (bypass T-0 lockout, postpone N:59 reset)
   - **NEW:** Admin Settings UI (edit messages without config file access)
   - **NEW:** Admin override logging (audit trail)
   - **NEW:** Visual indicator for admin override mode
   - Export destination confirmed: `{session_folder}/Jpg/` folder

3. **Story 2.3** — Background Export Queue & File Monitoring ✅ CLARIFIED
   - Dependencies: Story 1.1 (photo detection - prerequisite), Story 2.1, Story 2.2
   - Hooks Story 1.1's photo detection to background export queue
   - Extends session metadata, reuses existing export pipeline
   - **Confirmed:** Exports to `{session_folder}/Jpg/` folder
   - **Confirmed:** Starts immediately when photo detected (no idle wait)

4. **Story 2.4** — Smart Export Decision & Progress UI
   - Dependencies: Story 2.2 (T-0 event, end screen), Story 2.3 (background state)
   - Implements export decision modal (overwrite/continue), progress visualization
   - Integrates T-0 forced export trigger
   - **Confirmed:** Both export modes use `{session_folder}/Jpg/` folder

5. **Story 2.5** — Navigation Flow Optimization
   - Dependencies: Story 2.1 (countdown timer on all screens)
   - Low-risk UI polish: direct editor entry, gallery icon, HOME button routing

## Integration Points with Existing System

**Story 1.1 Foundation (prerequisite):**
- ✅ New photo detection in session folders (already implemented and verified)
- Epic 2 subscribes to these events for background export triggering

**Session Management:**
- Extends existing session lifecycle with time boundaries
- Preserves session folder contract and metadata format
- No breaking changes to session creation/initialization

**Export Pipeline:**
- Reuses existing `export_photo()` function (identical pipeline for background/manual)
- Background exports run at lower thread priority to avoid UI impact
- Session metadata tracks completion state per photo

**UI/Navigation:**
- Timer countdown overlays on existing Editor/Library headers
- Modals use React portal pattern (consistent with existing modal approach if present)
- Navigation routing updates preserve all existing screen accessibility

## Compatibility Requirements

- ✅ No changes to session folder structure or schema (only extends metadata JSON)
- ✅ Export destination uses existing `{session_folder}/Jpg/` folder (no new folders)
- ✅ Background and manual export produce byte-identical outputs
- ✅ All existing screens remain accessible (only default navigation paths change)
- ✅ Admin/customer mode gating respected throughout timeline features:
  - Customer mode: Strict timeline enforcement (no bypass)
  - Admin mode: Timeline visible but can be bypassed/extended for troubleshooting

## Risk Mitigation

**Primary Risk:** Timer drift or system clock changes causing missed resets or incorrect session boundaries

**Mitigation:**
- Use wall-clock time validation on every tick (not elapsed time counters)
- Detect and log clock jumps forward/backward
- Force immediate recovery actions (e.g., reset if clock jumps past N:59)
- Log warnings for diagnostics

**Secondary Risk:** Background export race conditions with manual export

**Mitigation:**
- Atomic state management for "currently exporting" flag
- Cooperative cancellation when manual export starts
- Single source of truth for export operations
- File-level locking or atomic operations for export files

**Tertiary Risk:** File write detection reliability (mitigated - low risk)

**Status:** Story 1.1 already implemented and verified file write completion detection
- Background export hooks into validated mechanism
- Additional safety: catch/retry on RAW decode errors

**Rollback Plan:**
- Feature flag to disable timeline enforcement
- System reverts to manual-only export and unlimited session duration
- Config file allows easy disable: `"session_timeline.enabled": false`

## Definition of Done

- ✅ All 5 stories completed with acceptance criteria met
- ✅ Timeline enforces N:00-N:50 windows with accurate countdown (±2s)
- ✅ T-5 warning, T-0 lockout, N:59 reset function reliably across multiple sessions
- ✅ **Customer mode:** Strict timeline enforcement (no bypass)
- ✅ **Admin mode:** Timeline visible but admin can override/bypass all restrictions
- ✅ **Admin settings UI:** Freely edit messages (end screen, T-5 warning) without config file access
- ✅ Background export keeps pace with typical shooting (export <10s in "Continue" mode)
- ✅ Export destination: `{session_folder}/Jpg/` folder (existing structure)
- ✅ Export decision modal provides working overwrite/continue options with progress
- ✅ Navigation flows optimized without breaking existing functionality
- ✅ Existing capture/preset/manual-export regression-tested and verified
- ✅ Configuration file allows admin customization (timing, messages)
- ✅ Admin override actions logged for audit trail
- ✅ Comprehensive logging enables field diagnostics

## Human-only Decisions / Inputs ✅ RESOLVED

**Configuration Defaults:** ✅ CONFIRMED
- ✅ Session window timing: N:00-N:50-N:59 confirmed correct
- ✅ Default end screen message: "이용해주셔서 감사합니다." confirmed appropriate
- ✅ T-5 warning message: "세션 종료가 5분 남았습니다" confirmed appropriate
- ⚠️ **NEW REQUIREMENT:** Admin must be able to freely modify these messages via UI (not just config file)

**Admin Override Capability:** ✅ DECIDED
- ✅ **Decision:** Admin mode allows session time extension/override (bypass timeline enforcement)
- Implementation: Admin mode shows timer and warnings but can dismiss/skip forced actions
- Customer mode: Strict enforcement (no bypass)
- Note: Adds complexity but necessary for admin flexibility during troubleshooting/testing

**Background Export Behavior:** ✅ CONFIRMED
- ✅ **Decision:** Start immediately when photo detected (one photo at a time, low priority)
- No idle waiting period required

**Export Destination:** ✅ CONFIRMED
- ✅ **Decision:** Export to existing session folder structure: `{session_folder}/Jpg/`
- Leverages existing `Jpg` folder already present in session structure
- No new folder creation needed

## Success Criteria (from PRD)

- Session automatically locks and exports at N:50, resets at N:59
- User sees countdown timer accurate to ±2 seconds throughout 50-minute session
- Late entry (e.g., N:45 start) correctly ends at N:50 (not N:45+50)
- Background processing keeps pace with typical shooting (50 photos per session)
- Export completion <10s when using "Continue" mode (80%+ pre-processed)
- Existing capture/preset/manual-export functionality unaffected
- Navigation changes preserve all screen accessibility

## Technical Constraints (Summary)

**Technology Stack (from existing):**
- Tauri 2.9.x (timer events, background tasks, IPC)
- React 19.2.3 + TypeScript 5.9.3 (UI components, modals, routing)
- Rust + tokio (SessionTimer service, export queue)
- Existing RapidRAW export pipeline (reused for background processing)

**New Dependencies:**
- None (Story 1.1's photo detection already uses appropriate file watching)

**Performance Targets:**
- Timer tick: 1 second resolution, ±2s accuracy over 50 minutes
- Modal response: <500ms from trigger condition
- Background processing: No perceptible UI lag or frame drops
- Export completion: <10s (continue mode), <5min (overwrite mode) for 50 photos

## Story Manager Handoff

Please develop detailed user stories for this brownfield epic. Key considerations:

- This is an enhancement to existing Boothy (Tauri/Rust + React) session and export systems
- **Critical prerequisite:** Story 1.1's photo detection mechanism (already implemented and verified)
- Integration points: Tauri event system, tokio background tasks, React Router, existing export pipeline, Story 1.1's photo detection events
- Existing patterns to follow: Session folder contract, RapidRAW export pipeline reuse, customer/admin mode gating
- Critical compatibility: No session folder schema changes, identical export output for background/manual processing

Each story must include verification that existing functionality remains intact.

---

## Reference Documents

- **Parent PRD:** `docs/prd-session-timeline-smart-export.md` (comprehensive requirements)
- **Baseline PRD:** `docs/prd.md` (Unified Boothy baseline)
- **Architecture:** `docs/architecture.md` (system architecture context)
- **Tech Stack:** `docs/architecture/tech-stack.md` (technology details)
- **Feature Spec (Korean):** `add_funct.md` (original requirements)

## Notes

**Scope Assessment:**
- This epic has 5 stories, exceeding the typical 1-3 story brownfield epic guideline
- Justification: Features are tightly coupled (timeline triggers export, export needs timeline context)
- Alternative considered: Split into 2 epics (timeline + export) → rejected due to integration risk
- Full PRD process was used appropriately given the scope and integration complexity

**Story 1.1 Dependency:**
- Epic 2 builds on Story 1.1's photo detection capability
- Verification: Story 1.1 successfully detects new photos added to session folders
- Integration: Epic 2 subscribes to Story 1.1's detection events (no re-implementation needed)
