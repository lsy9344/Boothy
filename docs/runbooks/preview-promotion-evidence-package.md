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
- Primary release acceptance is `same-capture preset-applied full-screen visible <= 2500ms`.
- Preview confirmation follows the same `2500ms` threshold as a supporting guardrail and cannot close release sign-off by itself.
- `preview-promotion-evidence.jsonl` is the canonical machine-readable summary for lane owner, fallback reason, route stage, warm state, and preview timing deltas.
- The operator-safe bundle must preserve the selected capture chain only: `request-capture -> file-arrived -> capture_preview_ready -> recent-session-visible -> capture_preview_transition_summary` for one `sessionId/requestId/captureId`.
- `sameCaptureFullScreenVisibleMs` is the new-track release field; legacy `replacementMs` remains comparison-only or backward-compatible alias data.
- `firstVisibleMs`, tiny preview success, and recent-strip visibility are supporting diagnostics only and cannot replace the new-track release field.
- The machine-readable record must point to capture-time policy/catalog snapshots so later policy changes do not reinterpret an already recorded booth run.
- `visibleOwner` and `visibleOwnerTransitionAtMs` are required evidence. If the selected capture chain drops them, bundle assembly fails closed instead of inferring a close owner.
- Whole-session timing logs are not operator-safe evidence. The assembled bundle must copy only the selected capture timing chain so wrong-capture and cross-session attribution stay auditable.
- The assembled evidence bundle also computes `fallbackRatio` from the matching session/preset/version evidence family for the same promoted route stage.
- Legacy Stories 1.18, 1.19, and 1.20 remain legacy comparison only.
- Stories 1.21 and 1.22 own the metric/evidence baseline.
- Stories 1.23 through 1.27 own prototype/evidence/gate history only and must not be read as final actual-lane implementation proof.
- Stories 1.28 through 1.31 own actual implementation, vocabulary realignment, actual-lane canary, and actual-lane default/rollback proof before Story 1.13 can reopen as the final guarded release-close owner.
- Evidence, dashboard wording, and ledger copy must keep the boundary between `legacy comparison only` and `new-track release field` explicit.
- `default` promotion is valid only after repeated `canary` success-path evidence is present for the same approved preset/version scope.
- Automated pass alone is never `Go`.
- Any fallback-heavy run, parity drift above threshold, missing route policy snapshot, or missing rollback evidence remains `No-Go`.

## Scripts

- Start trace planning: `scripts/hardware/Start-PreviewPromotionTrace.ps1`
- Stop trace/export planning: `scripts/hardware/Stop-PreviewPromotionTrace.ps1`
- Assemble booth evidence bundle: `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- Evaluate canary gate from the assembled bundle: `scripts/hardware/Test-PreviewPromotionCanary.ps1`

## Recommended Flow

1. Start the trace plan before the booth run.
2. Capture the booth session and preserve the same-capture booth/operator visuals.
3. Stop trace collection and export the approved analysis views.
4. Assemble the evidence bundle with the session id, capture id, preset id, and published version.
5. Compare the resident lane output against the darktable baseline oracle and optional fallback oracle.
6. Run `Test-PreviewPromotionCanary.ps1` against the assembled bundle and review the typed checks for KPI, fallback stability, wrong-capture, fidelity drift, rollback readiness, and active-session safety together.
7. Record the final `Go / No-Go` decision in `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`.

## Parity Gate

- Default numeric threshold: mean absolute channel delta `<= 6`
- A baseline pass is `pass`.
- A fallback-only pass is `conditional` evidence and does not close promotion by itself.
- Missing oracle input is `not-run`.
- Any measured fail is `No-Go` until rerun evidence is captured.
- `Test-PreviewPromotionCanary.ps1` keeps the canary at `No-Go` when KPI miss, fallback-heavy behavior, wrong-capture drift, fidelity drift, missing rollback proof, or non-canary active-session safety risk remains.
