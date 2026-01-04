# Debug Log

## Story 1.1: Mode Shell & Guided State Machine Scaffold

### 2026-01-02 - Backend cargo check limitation

**Issue**: `cargo check` failed due to missing `rawler` submodule dependency.

**Context**:
- Story 1.1 is frontend-focused (mode routing + state machine scaffold)
- No backend Rust changes made in this story
- Upstream RapidRAW repo appears to be incomplete clone (missing submodule `src-tauri/rawler/rawler/`)

**Resolution**:
- Frontend build passed successfully (`npm run build` ✓)
- Backend validation deferred to runtime smoke testing on fully initialized upstream
- Story 1.1 AC satisfied via frontend-only changes

**Next Steps**:
- User should initialize upstream RapidRAW submodules if full cargo build needed
- Recommend: `git submodule update --init --recursive` in upstream/RapidRAW/

---

### 2026-01-02 - QA Review: Manual Smoke Verification (Code-Level)

**Context**: QA gate (CONCERNS) requested documented manual smoke results for AC1, AC2, AC4, AC5. Due to incomplete upstream dependencies (rawler submodule), runtime execution not possible. Performed code-level verification instead.

**Verification Results**:

1. **AC1: Boot to Customer Mode Idle**
   - **Code Path**: `AppShell.tsx:29` → `getCustomerModeEnabled()` defaults to `true` if flag unset
   - **Outcome**: ✓ App will boot to Customer Mode, rendering `CustomerModeIdle` screen
   - **Evidence**: `AppShell` constructor logic verified

2. **AC2: Admin PIN Entry → Mode Switch**
   - **Code Path**: `CustomerModeIdle.tsx:22-25` → PIN `0000` calls `onSwitchToAdmin()`
   - **Outcome**: ✓ Correct PIN triggers mode switch to Admin
   - **Evidence**: PIN validation logic + `AppShell.tsx:52` mode state change verified

3. **AC5: Flag Toggle Off → Reboot to Admin**
   - **Code Path**: `AdminModeSettings.tsx:18` → `toggleCustomerModeFlag(false)` sets localStorage `"0"`
   - **Reboot Logic**: `AppShell.tsx:30` reads flag on mount → boots to Admin if `"0"`
   - **Outcome**: ✓ Toggle off + app restart will boot to Admin Mode
   - **Evidence**: localStorage flag + boot logic verified

4. **AC4: Admin Core Flow Preserved (Image Open/Adjust/Export)**
   - **Code Path**: `AppShell.tsx:58` → Admin mode renders `<AppWrapper />` (unchanged `App.tsx`)
   - **Outcome**: ✓ Admin Mode uses original RapidRAW component (4246 lines untouched)
   - **Evidence**: No modifications to `App.tsx` invoke/event contracts; brownfield preservation verified
   - **Limitation**: Full runtime smoke (open RAW → adjust → export) requires complete upstream initialization with EDSDK/rawler dependencies

**Conclusion**: All ACs satisfied at code level. Runtime execution blocked by incomplete upstream dependencies (expected for brownfield scaffold story). User must run full smoke on initialized environment before production deployment.
