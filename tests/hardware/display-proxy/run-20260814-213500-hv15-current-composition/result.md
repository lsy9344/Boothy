# HV-15 current-composition limited run

## Decision

**No-Go** for the current booth composition.

The camera, USB capture, RAW persistence, preset binding, and repeated capture flow remained operational. However, no display proxy was admitted to the customer viewer.

## Measured result

- Session: `session_000000000018cbab97741ad538`
- Preset: Daylight (`preset_daylight` @ `2026.03.27`)
- Scope: one unchanged physical composition, four captures
- Captures persisted: 4/4 RAW, 4/4 product previews
- Display publication attempts: 4
- Accepted generations: 0
- Rejected generations: 4/4, all `insufficient-dimensions`
- Active customer generation: none
- Required customer image area: 1429 x 953 px
- Exact reproduction output from the current portrait RAW: 634 x 953 px

The current camera composition is portrait while the approved customer display area is landscape. With upscaling and arbitrary crop disabled, the renderer cannot satisfy both required dimensions. Repeating the capture produced the same safe rejection every time.

## Gate interpretation

- S1 single capture: attempted, rejected before publication.
- S2 three consecutive captures: attempted, all three rejected for the same dimension mismatch.
- S3-S6: not continued because no qualifying generation existed to validate switching, clearing, or session isolation.
- Visual quality: not claimed; the user explicitly limited this run to the currently visible composition.
- Telemetry/evidence checker: expected failure because there were zero committed generations and therefore no actual-present records.

## Product action required

Choose and approve one portrait-to-landscape presentation policy before rerunning HV-15: a portrait viewer region, a deterministic approved crop, or a letterboxed policy whose display-fit gate accepts the contained image. The current rules intentionally permit none of these.
