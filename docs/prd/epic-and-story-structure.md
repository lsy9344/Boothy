# Epic and Story Structure

**Epic Structure Decision**: Single comprehensive epic.

Rationale (grounded in current repo reality):

- The enhancement is one cohesive outcome: a single Tauri/React Boothy application that unifies camera tethering and RapidRAW editing/export with customer/admin gating.
- The major risks (Canon EDSDK integration, real-time import, per-photo preset assignment, kiosk-safe UI gating) are tightly coupled and should be sequenced within one epic to avoid integration drift.
- We can still manage delivery risk by slicing stories to deliver the booth-critical path first (Canon MVP), then expand admin-visible feature parity toward “all digiCamControl features”.
