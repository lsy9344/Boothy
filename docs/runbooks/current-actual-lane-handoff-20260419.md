---
documentType: handoff-note
status: active
date: 2026-04-19
scope: preview-architecture
sourceWorkspace: C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40
---

# Current Actual-Lane Handoff

## Why This Note Exists

This worktree intentionally returns to the older `thumbnail-latency-seam-reinstrumentation` line because the newer actual-primary-lane track did not close the current release gate on approved hardware.

Use this note as the minimum current-state transfer package while working in this older architecture line.

## Current Release Gate

- Canonical release judgment now uses one official product gate only:
  - `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
  - product wording: `preset-applied visible <= 3000ms`
- `sameCaptureFullScreenVisibleMs` remains a reference/comparison metric for first-visible speed and route regression reading.
- `first-visible`, tiny preview, or recent-strip speed alone do not count as release success.

Source snapshot taken from the newer workspace:

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\docs\release-baseline.md`

## Why The Newer Track Was Stopped

- Story `1.30` actual-primary-lane canary became bounded `No-Go` evidence against the official `preset-applied visible <= 3000ms` gate.
- Repeated hardware reruns improved some symptoms but never approached the release gate closely enough.
- Additional reruns were judged low-value compared with route change.

Decision snapshot source:

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\docs\runbooks\preview-track-route-decision-20260418.md`

## Latest Actual-Lane Field Evidence

Latest approved-hardware style field sessions from the newer workspace still missed the gate badly:

- Early rerun band: `sameCaptureFullScreenVisibleMs = 9129..24076`
- Improved but still failing band: `7532..9169`
- After speculative-close fix: `7551..9368`
- After lower preview cap: `7805..9122`
- After in-memory preview library change: `11170..29915`
- After join-window widening: `8540..13019`
- After preview `--configdir` removal: `8798..9958`
- Latest `originalVisibleToPresetAppliedVisibleMs` still stayed at `5257..6347`

Practical conclusion:

- The remaining blocker was no longer only race noise.
- The core hot path still cost too much to close the official `preset-applied visible <= 3000ms` gate even after multiple host-side reductions.

Primary evidence source snapshot:

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-30-actual-primary-lane-hardware-canary-재검증.md`

## What Was Already Tried On The Newer Track

The newer line already attempted several reductions before this handoff:

- speculative close race removal
- smaller same-capture fast-preview cap (`512x512`)
- in-memory preview library for preview intent
- larger join window to avoid duplicate close launches
- preview host invocation without preview `--configdir`

Result:

- correctness improved
- false-failure noise reduced
- KPI still stayed far above the release gate

## Historical Context For This Older Line

This older line is being revisited because it was the closest line to Lightroom-style behavior:

- resident first-visible worker
- earlier perceptible image replacement
- later truthful close

Historical real-hardware style evidence in this line was meaningfully better than the newer actual-primary-lane numbers, for example:

- `firstVisibleMs = 2935 / 2819 / 2827 / 2810 / 3110`
- `replacementMs = 3694 / 3451 / 3852 / 3615 / 3707`

Important boundary:

- those numbers were better for user-perceived speed
- they still do not prove the official `preset-applied visible <= 3000ms` gate
- they are the reason this line is worth revisiting, not proof that a simple rollback is enough

Primary historical evidence source already present in this worktree:

- `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`

## Working Interpretation

- Returning here is not a claim that this older line is already the winner.
- Returning here means this line remains the strongest product-feel candidate for a Lightroom-like fast preview experience.
- Latest rerun now closes this line as a focused baseline/comparison lane, not as automatic release proof.
- Story `1.31` remains unopened.
- Story `1.26` is now the officially opened reserve path because this comparison lane still did not support a credible route change.

## Current Operational Use In This Worktree

- Keep this older line as a closed `No-Go` baseline and comparison reference.
- Do not treat additional old-line tuning as the primary execution path.
- Use any further GPU comparison only as side evidence while Story `1.26` progresses as the active reserve path.

## Source Snapshots To Keep In Mind

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\docs\runbooks\preview-track-route-decision-20260418.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\docs\release-baseline.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\docs\preview-architecture-history-and-agent-guide.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-30-actual-primary-lane-hardware-canary-재검증.md`
