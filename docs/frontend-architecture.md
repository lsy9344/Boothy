# Boothy Frontend Architecture (MVP)

This document scopes the UI architecture for Boothy’s kiosk-friendly workflow on top of the RapidRAW baseline UI, aligned with:

- PRD: `docs/prd.md`
- UX policy: `docs/design_concept.md`
- TO-BE architecture: `docs/architecture.md`

## Key Principles

- Customer mode is the default; admin mode is unlocked via password.
- Customer mode hides advanced controls (do not disable; do not show).
- The “center image viewport” remains the primary surface.
- The session thumbnail list replaces any camera “preview strip”.

## Core Views (MVP)

1. Session Start
   - Inputs: session name
   - Action: create/open session folder under `%USERPROFILE%\\Pictures\\dabi_shoot`, set active session
2. Main Booth Screen (Customer)
   - Center viewport (current selected image)
   - Session thumbnails (active session folder only)
   - Preset picker (customer-visible)
   - Actions: Capture, Export image, Delete selected
   - Admin toggle (opens password modal)
3. Admin Unlock Modal
   - Password prompt, success → admin mode
4. Main Booth Screen (Admin)
   - Adds advanced camera controls + advanced RapidRAW panels (still consistent styling)

## State Model (Frontend)

Single source of truth (suggestion; implementation can vary):

- `mode`: `customer` | `admin`
- `activeSession`:
  - `name`
  - `paths`: `{ base, raw, jpg }`
- `library`:
  - `items`: list of images (active session only)
  - `selectedId`
- `preset`:
  - `selectedPresetId`
  - `perImagePresetAssignment` is read-only in UI; assignment happens on import in backend
- `camera`:
  - `connectionState`: disconnected/connecting/connected/error
  - `lastError` (customer-safe summary + admin diagnostics)

## Event/Command Contract (UI-facing)

Frontend triggers commands; backend emits events:

- Commands (examples):
  - `boothy_start_session(sessionName)`
  - `boothy_capture()`
  - `boothy_delete_selected(ids)`
  - `boothy_export_selected(ids)`
  - `boothy_set_mode(mode)` (admin only after unlock)
- Events (examples):
  - `boothy-session-changed`
  - `boothy-library-updated`
  - `boothy-photo-imported` (auto-select newest)
  - `boothy-camera-state`
  - `boothy-error` (customer/admin variants)

## UI Gating Rules

Authoritative policy list lives in `docs/design_concept.md`. The implementation rule is:

- In customer mode: remove/hide panels and controls, avoid “disabled clutter”.
- In admin mode: reveal panels/controls but keep RapidRAW styling consistent.

## Open Questions (tracked for follow-up stories)

- Which RapidRAW panels map 1:1 vs need re-layout for “booth mode”?
- Minimal accessibility requirements for kiosk/touch usage (focus order, large targets).
