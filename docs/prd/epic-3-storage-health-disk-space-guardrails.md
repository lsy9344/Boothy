# Epic 3: Storage Health (Disk Space) Guardrails & Admin Cleanup

- Epic ID: epic-3
- Type: Brownfield enhancement
- Baseline: Existing Boothy session storage + export pipeline (from Epic 1)
- Parent PRD: `docs/prd.md` (NFR5, NFR7; error/recovery states incl. low disk space)

## Goal

Prevent booth downtime and data loss caused by low disk space by proactively monitoring storage health, surfacing kiosk-safe warnings, and enabling admin-only cleanup workflows.

## Existing System Context

- Session storage contract: `%USERPROFILE%\\Pictures\\dabi_shoot\\<session>\\{Raw,Jpg}` (session folder is the integration boundary).
- Export output: written to session `Jpg/` folder.
- Offline-first: no required sign-in and no default network calls in the booth-critical workflow.
- Current gaps observed:
  - Low disk space is listed as an error/recovery scenario in product requirements, but there is no proactive disk monitoring and the `DISK_FULL` error code exists without verified end-to-end handling.

## Enhancement Details

### What’s being added/changed

- **Storage health monitor** (backend): periodically checks free space on the drive that contains the active session root.
- **Kiosk-safe UX** (frontend): surfaces warnings and critical lockout messaging that does not expose technical details in customer mode.
- **Admin diagnostics + cleanup**: admin-only visibility into drive stats/thresholds and guided actions to recover space without breaking session safety rules.

### Integration Approach

- Backend emits an event (e.g., `boothy-storage-health`) to drive UI updates.
- Backend stores thresholds/config in existing AppData settings (`settings.json`, `boothy.*` namespace).
- Guardrails enforce path safety: any cleanup/delete actions are limited to the session root and must not allow traversal outside `%USERPROFILE%\\Pictures\\dabi_shoot`.

## Acceptance Criteria

1. Given a session is active, Boothy continuously monitors free disk space for the session storage drive and reports status to the UI at a fixed interval (e.g., every 10 seconds) without impacting UI responsiveness.
2. Given free space drops below a **warning threshold**, the UI displays a non-blocking warning in customer mode and a detailed diagnostic view in admin mode.
3. Given free space drops below a **critical threshold**, Boothy blocks booth-critical write actions (capture/transfer ingest/export) in customer mode with a customer-safe message (e.g., `디스크 공간이 부족합니다. 직원에게 문의해 주세요.`) and preserves read-only access to already-imported photos.
4. Given admin mode is unlocked, the admin can view current disk usage/free space, the configured thresholds, and can take a recovery action (at minimum: open the session root in Explorer; optionally: delete old sessions with explicit confirmation).
5. Given disk space returns above the warning threshold, Boothy automatically clears warnings/lockout state and booth workflow resumes without restarting the app.
6. Existing session folder structure and export output remain unchanged; no new network dependencies are introduced.

## Story Sequence (dependency-ordered)

1. **Story 3.1: Storage Health Monitor (Backend) + UI Status Surface**
   - Implement periodic free-space checks and emit `boothy-storage-health`.
   - Add a lightweight UI status surface (banner/toast) for warning state.

2. **Story 3.2: Critical Lockout Guardrails + Error Wiring**
   - Block write operations when critical threshold is hit (customer mode), keep browsing/export of already-created JPGs when safe.
   - Wire consistent error handling using existing error framework and ensure `DISK_FULL`/low-space paths are exercised.

3. **Story 3.3: Admin Storage Diagnostics + Recovery Actions**
   - Admin-only panel to view thresholds and current disk stats.
   - Provide recovery actions (at minimum: open session root in Explorer).

4. **Story 3.4: Admin Guided Cleanup: Safe Deletion of Old Sessions**
   - Optional enhancement: safe deletion of selected old sessions under session root with strict confirmation + active-session protection.
   - Must enforce path safety (no traversal outside session root) and remain offline-first.

## Compatibility Requirements

- [ ] Session folder structure remains unchanged (`<session>\\{Raw,Jpg}`).
- [ ] No schema-breaking changes to existing settings; new settings are append-only and backward compatible.
- [ ] Cleanup actions are constrained to the session root (no path traversal).
- [ ] Offline-first policy remains intact (no telemetry/update checks added).

## Risk Mitigation

- **Primary Risk:** False-positive lockouts (e.g., incorrect drive detection or transient IO errors) blocking booth workflow.
  - **Mitigation:** Determine free space based on the canonicalized session root path; treat IO errors as an `unknown` status (warn admin only) rather than hard lockout unless confirmed critical.
  - **Rollback Plan:** Feature flag / setting to disable the monitor and guardrails (admin-only), default enabled for production.

## Definition of Done

- [ ] Acceptance criteria met for warning + critical scenarios.
- [ ] Manual regression confirms: session start → import/new photo detection → preset apply → export still works when disk is healthy.
- [ ] Manual scenario confirms: warning threshold UI appears; critical threshold blocks write actions but allows browsing existing photos.
- [ ] Admin diagnostics and recovery actions function and are constrained to session root.
- [ ] No regression in offline behavior and no new network calls are introduced.

## Story Manager Handoff

Please develop detailed user stories for this brownfield epic. Key considerations:

- Integration points: session folder contract, existing export pipeline, AppData settings, Tauri event system.
- Compatibility: no session schema changes; enforce strict path safety for any cleanup actions.
- UX: customer mode must stay kiosk-safe (hide/disable appropriately, with minimal and clear messaging; admin mode can show diagnostics).
- Testing: include scenarios for warning/critical thresholds and recovery back to healthy state without restart.

