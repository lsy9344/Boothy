# Requirements

These requirements are based on my understanding of your existing system and the clarified concept in this chat. Please review carefully and confirm they align with your project’s reality.

## Functional

1. FR1: The system must deliver a single unified “Boothy” Windows desktop application that combines camera capture (tethering) and RapidRAW editing/preset/export workflows in one UI.
2. FR2: The system must not use WPF for the product UI; the user experience must be implemented in Tauri + React and visually consistent with RapidRAW’s design concept/style.
3. FR3: The system must start each workflow by creating/opening exactly one active session via a user-provided session name, and the session browser must show only that session folder during the session.
4. FR4: The system must display an image in the central main viewport (RapidRAW’s center image area), and selecting a thumbnail must update the central viewport to that photo.
5. FR5: The system must allow a customer-mode user to trigger camera capture (shoot) from within the Boothy UI.
6. FR6: Captured photo files must be saved into the active session folder, and once file transfer to PC completes, the system must automatically detect/import the new photo without requiring manual refresh.
7. FR7: After import, the newest captured photo must appear in the session thumbnail list (replacing the camera app’s bottom preview strip concept) and be visible to the user immediately.
8. FR8: The system must provide PRESET selection in customer mode, and the currently selected preset must be automatically applied to each newly imported photo at the time it arrives.
9. FR9: Changing the selected preset must only affect photos imported after the change; previously imported photos must keep their originally applied preset (no retroactive updates).
10. FR10: The system must persist (at least within the session) the “preset assignment” per photo so export and re-rendering use the correct preset for each photo.
11. FR11: The system must support customer-mode actions: preset selection, capture, thumbnail selection, export (via RapidRAW “Export image” action), delete.
12. FR12: In customer mode, the export UI must be limited to RapidRAW’s “Export image” button (no advanced export options). Export must generate high-resolution JPEG outputs using each photo’s assigned preset and write outputs to a session output location (e.g., under the active session folder). Advanced export controls/options (if any) must be hidden in customer mode and only shown in admin mode.
13. FR13: Delete must remove selected photo file(s) from the active session folder and update the session list accordingly.
14. FR14: Rotate (CW/CCW) must be available in admin mode (hidden in customer mode) and must affect both on-screen preview and exported JPEG result for the rotated photo(s).
15. FR15: Customer mode must be the default on app launch.
16. FR16: Admin mode access must be “toggle → password”; without the correct password the app must remain in customer mode.
17. FR17: In customer mode, advanced/complex camera and editor controls must be hidden (not disabled) according to `docs/design_concept.md`, and those controls must be exposed in admin mode.
18. FR18: In customer-facing photo lists/thumbnail strips, the UI must not show camera metadata overlays (F, ISO, Exposure, FL, EB, histogram); thumbnails should present photos only.
19. FR19: In admin mode, the system must expose the full camera feature set equivalent to the digiCamControl reference (all camera features available, per scope), and advanced editor features, while maintaining RapidRAW-aligned UI style.
20. FR20: The system must surface camera connection state and actionable errors (disconnected, capture failed, transfer failed) without crashing and without blocking browsing/export of existing session photos.
21. FR21: MVP camera support must target Canon cameras, using Canon EDSDK-based capability mapping (digiCamControl as functional reference), with other camera ecosystems deferred until after MVP.

## Non Functional

1. NFR1: Platform must be Windows-only (MVP and initial releases).
2. NFR2: The product UI must be Tauri + React; WPF UI is prohibited.
3. NFR3: Real-time behavior: after file transfer completes, the new photo should appear in the session list within a target latency (proposal: ≤ 1s) and show a preset-applied preview in the main viewport within a target latency (proposal: ≤ 3s) on target hardware.
4. NFR4: Preset application/RAW processing/export must run in background so the UI remains responsive during capture/import/export.
5. NFR5: Data integrity: the system must not lose captured photos; photos must be written to disk before being considered imported, and partial transfers must not produce corrupted imports.
6. NFR6: Admin password must be stored securely (salted hash) and never stored or logged in plaintext.
7. NFR7: The application must work fully offline (no network dependency for core capture/edit/export).
8. NFR8: The system must provide logs/diagnostics for capture/import/export/preset processing sufficient to debug failures in the field.

## Offline / No-Account Policy (MVP)

- The Boothy product build must **not require sign-in** and must **not make any network calls by default** for the core booth workflow (session → capture → ingest → preset → export).
- Any RapidRAW baseline features that rely on network services (e.g., account auth, community pages, auto model downloads, telemetry, update checks) are **out of scope for Boothy MVP** and must be removed or fully disabled in the Boothy build.
- If any optional online feature is retained for admin troubleshooting in later phases, it must be **explicitly opt-in** and must never block/impact customer-mode operation.

## Compatibility Requirements

1. CR1: Existing API compatibility: RapidRAW preset definitions and export behavior must remain compatible (existing presets should still load and produce the same look).
2. CR2: Database/schema compatibility: if persistent storage for settings/session history/photo assignments is introduced, it must support forward/backward-compatible migrations between versions.
3. CR3: UI/UX consistency: new camera capture UX must be visually and interaction-consistent with RapidRAW (shared design system/components; no mixed UI styles).
4. CR4: Integration compatibility: camera→editor integration must follow the agreed session folder contract and detect photos only after transfer completion; no manual “import” step required.
