# User Interface Enhancement Goals

## Integration with Existing UI

The product UI should be built by extending/reworking RapidRAW’s existing layout and design system (Tailwind-based styling, iconography, panels), so the new camera UX feels native rather than “bolted on”.

Key integration decisions:

- The “center image” viewport remains the primary focus; the latest tethered photo becomes the current selection and displays there.
- The camera app’s bottom preview strip concept is replaced by RapidRAW’s session-based thumbnail list (folder images list), constrained to a single active session folder in customer mode.
- Customer mode exposes only the booth-operational controls (capture, preset, export image, delete) using large, touch-friendly affordances and minimal panels.
- Admin mode reveals advanced camera controls (full digiCamControl feature scope) and advanced editor/export controls within the same visual language.

## Modified/New Screens and Views

- **Session Start**: enter session name and initialize the session folder under `%USERPROFILE%\\Pictures\\dabi_shoot`.
- **Main Booth Screen (Customer Mode)**: center image viewport + session thumbnail list + preset selection + capture + “Export image” + delete, plus an admin toggle.
- **Admin Unlock Modal**: toggle → password prompt; on success, reveal admin UI.
- **Admin Mode Panels/Views**:
  - Camera advanced controls (mode/ISO/shutter/etc, advanced properties, and other digiCamControl-equivalent features).
  - RapidRAW advanced panels (metadata/image properties, advanced export options, etc).
  - Maintenance/config screens (password management, storage locations, camera diagnostics).
- **Error/Recovery States**: camera disconnected, capture failed, transfer failed, low disk space, export failed (customer-friendly messaging + admin diagnostics).

## UI Consistency Requirements

- Use RapidRAW’s typography, spacing, color system, icons, and panel behaviors; no “legacy” WPF look/feel.
- Customer mode must hide advanced controls (no “disabled clutter”); admin mode reveals them.
- Thumbnail/preview UI must show photos only (no F/ISO/exposure/histogram overlays).
- Customer-mode flow should be kiosk/booth friendly: minimal clicks, large targets, and clear feedback for capture/import/export progress.
