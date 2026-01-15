# Story 1: Booth App Foundation + Session Lifecycle + Mode Gating

- Story ID: epic-1.story-1
- Status: Ready
- References: `docs/prd.md`, `docs/architecture.md`, `docs/design_concept.md`, `docs/frontend-architecture.md`

## Summary

Establish a first-party Boothy app workspace (separate from `reference/`), implement session folder lifecycle, and implement customer/admin mode gating with “hide not disable” UI policy.

## Acceptance Criteria

1. Given the app is launched, when no session is active, then the user can create/open a session by entering a session name.
2. Given a session is started, then the app constrains the library/thumbnail list to the active session’s `Raw/` folder only.
3. Given a new image file is added into the active `Raw/` folder (manual drop/test file), then the app automatically refreshes the library and auto-selects the newest image for the center viewport.
4. Given the app is in customer mode, then advanced controls are hidden (not disabled) per `docs/design_concept.md`.
5. Given the user toggles admin mode, then a password prompt is shown; without a correct password the app remains in customer mode.

## Dependencies

- Requires access to RapidRAW baseline code (currently under `reference/uxui_presetfunction/`)
- No camera SDK required in this story (manual drop/test file is sufficient)

## Tasks (suggested)

1. Create product workspace separation
   - Promote/migrate `reference/uxui_presetfunction` into `apps/boothy` (product code boundary)
   - Keep `reference/` read-only for upstream comparison
2. Implement session lifecycle
   - Session name sanitize + collision strategy (open existing vs suffix)
   - Create folder structure `{Raw,Jpg}` under `%USERPROFILE%\\Pictures\\dabi_shoot\\<session>`
   - Persist last-used session name/config (AppData)
3. Implement file ingest for `Raw/` (no camera yet)
   - Watch `Raw/` folder changes and refresh library
   - Implement file stabilization checks before “import confirmed”
4. Implement mode gating
   - Centralized `mode` state (customer/admin)
   - Admin unlock UX (password + salted hash storage)
   - Hide UI panels per `docs/design_concept.md`
5. Validation
   - Manual smoke: start session → drop file → auto-select → export remains available
   - Ensure no network calls in the customer flow (policy compliance)

## Readiness Checkpoints (Definition of Ready)

This story is considered ready to start implementation when the following checkpoints are explicit and testable:

1. **Workspace/run target is defined**
   - Product code lives at `apps/boothy` after migration (not under `reference/`)
   - `apps/boothy` can be launched via:
     - `npm install`
     - `npm start`

2. **Minimum “session mode” regression passes**
   - Existing RapidRAW baseline still launches and can open an image folder
   - Export works on an existing image (high-level smoke), even before camera integration

3. **Offline policy check**
   - Customer flow (start session → view → export) operates with the network disabled

## Minimum Validation (Must Pass)

- Start session → create `{Raw,Jpg}` folders → drop an image into `Raw/` → auto-refresh list → newest auto-selected in center viewport
- Customer mode hides advanced controls per `docs/design_concept.md`

## Risks / Notes

- Migration to `apps/boothy` is a structural change; keep commits small and document new run instructions.
- This story intentionally avoids camera integration to keep the critical path focused.
