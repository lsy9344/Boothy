# Source Map

Use this file to choose the minimum set of Boothy documents and evidence for each loop. Start from the newest logs. Pull in architecture only for the boundary you are changing.

## Tier 1: Always Read

- User's newest retest note
- `history/camera-capture-validation-history.md`
  - Canonical running log for capture and booth-hardware troubleshooting.
  - Read only the newest relevant dated section first.
- Latest raw evidence for the current run
  - `session.json`
  - `timing-events.log`
  - `camera-helper-events.jsonl`
  - `camera-helper-status.json`
  - `preview-promotion-evidence.jsonl`
  - booth/operator screenshots or screen recordings if the user mentions them

Use ripgrep to locate the latest evidence bundle if the session root is not obvious:

```powershell
rg --files -g "session.json" -g "timing-events.log" -g "*camera-helper*.json*" -g "preview-promotion-evidence.jsonl" .
```

## Tier 2: Read Only When You Need Product Acceptance Or Release Truth

- `docs/runbooks/booth-hardware-validation-checklist.md`
  - Hardware validation gate, required evidence, and stop conditions.
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - Release-close and validation status.
- `release-baseline.md`
  - Release baseline when rollout or close state matters.

## Tier 3: Read Only The Smallest Relevant Current-Architecture Reference

Do not read `_bmad-output/planning-artifacts/architecture.md` by default.

Choose one targeted current-architecture source for the boundary you are about to modify:

- Capture and camera readiness
  - `docs/contracts/camera-helper-edsdk-profile.md`
  - `docs/contracts/host-error-envelope.md`
  - Relevant section in `_bmad-output/planning-artifacts/architecture.md` only if owner boundary is unclear
- Session truth and evidence shape
  - `docs/contracts/session-manifest.md`
  - Relevant section in `_bmad-output/planning-artifacts/architecture.md` only if storage ownership is unclear
- Preview and renderer behavior
  - `docs/contracts/local-dedicated-renderer.md`
  - `history/thumbnail-replacement-timing-history.md`
  - `_bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md`
  - `_bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md`
  - `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
  - `_bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md`
- Preset and publication behavior
  - `docs/contracts/preset-bundle.md`
  - `docs/contracts/canonical-preset-recipe.md`
  - `docs/contracts/authoring-publication.md`
- Rollout or branch policy
  - `docs/contracts/branch-rollout.md`

## Architecture Delta Rule

The architecture is not re-read end-to-end every loop.

- Assume the changed architecture already exists.
- Read only the changed boundary that matters for the current fix.
- Prefer the latest relevant contract or implementation artifact over the full planning document.
- Open the full `_bmad-output/planning-artifacts/architecture.md` only if the smaller references still leave ownership ambiguous.

Useful starting command for targeted history:

```powershell
git log --oneline -n 20 -- docs/contracts history src src-tauri sidecar _bmad-output/implementation-artifacts
```
