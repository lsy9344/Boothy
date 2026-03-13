# Boothy Sidecar Protocol Fixtures

These fixtures lock the baseline contract for Story 1.3.

Stable fields:
- `schemaVersion`
- `requestId`
- `correlationId`
- `method`
- `event`
- `sessionId`
- normalized camera status and error envelope field names

Reserved for later stories:
- additional capture metadata
- export destination details
- recovery-only event families

Forbidden shortcuts:
- direct UI `invoke` of raw sidecar methods
- direct UI filesystem reads for capture truth
- raw device text on customer surfaces
- alternate manifest formats
- ad hoc sidecar message shapes
