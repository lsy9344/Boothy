---
name: boothy-hardware-debug-loop
description: "Inspect Boothy app execution logs and hardware-validation evidence, infer the most likely current issue, make the next bounded improvement, and hand back a short product-focused retest request. Use when the user asks to run the app and verify on hardware, says \uC571 \uC2E4\uD589 \uD6C4 \uD558\uB4DC\uC6E8\uC5B4 \uAC80\uC99D\uD588\uB2E4, asks to \uB85C\uADF8 \uD655\uC778\uD558\uACE0 \uBB38\uC81C \uD574\uACB0\uD574\uB2EC\uB77C, reports \uC5EC\uC804\uD788 \uB290\uB9AC\uB2E4 or still slow, or wants a repeated cycle of log review, improvement, hardware retest guidance, and follow-up based on new test results."
---

# Boothy Hardware Debug Loop

## Overview

Use this skill as Boothy's standing hardware-debug partner. Start from the latest logs and hardware evidence, infer the most likely issue, make the smallest safe improvement, then tell the user what to retest next in short product language.

## Workflow

1. Read the latest evidence before suggesting changes.
   - Read `references/source-map.md`.
   - Read the user's newest retest note first.
   - Read only the newest relevant section of `history/camera-capture-validation-history.md`, not the whole file.
   - Read the latest raw logs and session evidence for the current run.
   - Use architecture references only for the changed boundary you are about to touch.

2. Separate expected behavior from observed behavior.
   - Use the newest history entry, raw logs, session evidence, and the user's retest note to define what actually happened.
   - Use the hardware validation gate and the smallest relevant current architecture reference only to confirm whether the observed behavior is acceptable.
   - Do not widen into broad design review unless the evidence forces it.

3. Form one bounded hypothesis per loop.
   - Name the most likely boundary: booth UI, host, helper, renderer, preset/catalog, session storage, or hardware/runtime environment.
   - State what is confirmed by evidence, what is inferred, and what is still missing.
   - If evidence is too weak, improve diagnostics rather than making speculative behavior changes.

4. Make the smallest product-safe improvement.
   - Preserve truthful customer states such as `Ready`, `Preview Waiting`, `Export Waiting`, and completion.
   - Preserve published preset truth, active-session binding, and hardware-validation guardrails.
   - Prefer targeted fixes or better instrumentation over broad refactors during the loop.

5. Verify before handing back.
   - Run the most relevant automated verification for the area you changed.
   - If no automated verification fits, say so explicitly and explain why.
   - Record the cause analysis, implementation summary, verification, and next hardware retest request in `history/camera-capture-validation-history.md`.

6. Hand the loop back to the user.
   - Report the outcome in product terms, not code-level narration.
   - Tell the user exactly what to test next on hardware in short form.
   - Tell the user what evidence to bring back.
   - When the user returns, compare the new evidence with the previous hypothesis and repeat from step 1.

## Evidence Order

- Read `references/source-map.md` to choose the right documents and raw evidence paths.
- Read `references/loop-contract.md` for the turn-by-turn operating contract.

## Guardrails

- Keep the booth surface truthful. Never hide missing readiness or missing completion behind optimistic copy.
- Treat hardware validation as a separate release gate from unit, integration, or local smoke success.
- Prefer the newest dated evidence when history conflicts, and call the conflict out explicitly.
- If the root problem is environment setup or hardware state, say that directly instead of forcing code churn.
- Use architecture documents only as supporting truth for the specific changed boundary, not as the default starting point for every loop.
- Do not read full planning documents by default when a targeted contract, story artifact, or log slice is enough.

## Useful Commands

Use these only as starting points and narrow them to the current failure.

```powershell
Get-Content history\camera-capture-validation-history.md -Tail 200
rg -n "capture|preview|timeout|requestId|ready|error" history src src-tauri sidecar docs
rg --files -g "session.json" -g "timing-events.log" -g "*camera-helper*.json*" -g "preview-promotion-evidence.jsonl" .
git log --oneline -n 20 -- docs/contracts history src src-tauri sidecar _bmad-output/implementation-artifacts
```
