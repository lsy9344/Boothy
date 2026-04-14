# Preview Promotion Evidence Package

## Purpose

This runbook freezes the evidence package Boothy uses before arguing for preview-renderer promotion.

## Canonical Inputs

- `session.json`
- `diagnostics/timing-events.log`
- `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`
- `diagnostics/captured-preview-renderer-policy.json`
- published `bundle.json`
- `diagnostics/captured-catalog-state.json`
- booth/operator visual evidence for the same capture correlation
- optional WPR/WPA/PIX exports for the same run

## Rules

- Promotion evidence is valid only when the compared artifacts are same-capture, same-session, and same-preset-version.
- `preview-promotion-evidence.jsonl` is the canonical machine-readable summary for lane owner, fallback reason, route stage, warm state, and preview timing deltas.
- The machine-readable record must point to capture-time policy/catalog snapshots so later policy changes do not reinterpret an already recorded booth run.
- The assembled evidence bundle also computes `fallbackRatio` from the matching session/preset/version evidence family for the same promoted route stage.
- `default` promotion is valid only after repeated `canary` success-path evidence is present for the same approved preset/version scope.
- Automated pass alone is never `Go`.
- Any fallback-heavy run, parity drift above threshold, missing route policy snapshot, or missing rollback evidence remains `No-Go`.

## Scripts

- Start trace planning: `scripts/hardware/Start-PreviewPromotionTrace.ps1`
- Stop trace/export planning: `scripts/hardware/Stop-PreviewPromotionTrace.ps1`
- Assemble booth evidence bundle: `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`

## Recommended Flow

1. Start the trace plan before the booth run.
2. Capture the booth session and preserve the same-capture booth/operator visuals.
3. Stop trace collection and export the approved analysis views.
4. Assemble the evidence bundle with the session id, capture id, preset id, and published version.
5. Compare the resident lane output against the darktable baseline oracle and optional fallback oracle.
6. Record the final `Go / No-Go` decision in `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`.

## Parity Gate

- Default numeric threshold: mean absolute channel delta `<= 6`
- A baseline pass is `pass`.
- A fallback-only pass is `conditional` evidence and does not close promotion by itself.
- Missing oracle input is `not-run`.
- Any measured fail is `No-Go` until rerun evidence is captured.
