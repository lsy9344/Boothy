# Hardware Validation Ledger

Last Updated: 2026-04-30 11:02 +09:00
Sprint Artifact Owner: Boothy sprint operator
Canonical Path: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

## Policy Summary

- Truth-critical stories do not move to `done` from automated evidence alone.
- `done` requires both `automated pass` and a canonical hardware ledger row marked `Go`.
- If hardware evidence is missing, incomplete, or recorded as `No-Go`, the story stays in `review` or returns to `review`.
- Release promotion stays on `release hold` until every gated story needed for the release baseline has a `Go` row in this ledger.
- Preview-track official `Go / No-Go` ownership lives in this ledger. Rerun notes, story artifacts, and route memos may summarize evidence, but they do not override the verdict recorded here.

## Current Preview Track Interpretation

- Current preview-track official release gate is only `originalVisibleToPresetAppliedVisibleMs <= 3000ms` on approved hardware, expressed in product language as `preset-applied visible <= 3000ms`.
- `sameCaptureFullScreenVisibleMs` and first-visible numbers remain useful, but only as reference, comparison, and product-feel metrics.
- Story `1.30` is the current bounded `No-Go` evidence package for the actual-primary lane because repeated approved-hardware reruns did not close the official `preset-applied visible <= 3000ms` gate.
- Story `1.10` is now the closed `No-Go` baseline for the old `resident first-visible` line because the latest one-session rerun revalidated the lane but still failed the official gate.
- Story `1.31` remains unopened and is reserved as the success-side default / rollback gate, not as the current rerun path.
- Story `1.26` remains the active reserve path, and its latest official ledger state is now `Go`.
- The 11:38 pass and `hardware-validation-run-1777434275752` resident-labeled pass remain retracted as false Go evidence. The latest run fixes the evidence contract by labeling the actual runtime as `engineMode=per-capture-cli`.
- Native RAW approximation and metadata-only `preset-applied-preview` remain comparison-only. Explicit raw-original per-capture full-preset output is accepted when it passes approved hardware.
- This worktree no longer treats the older `resident first-visible` line as the active execution lane; it remains comparison evidence only.
- That older line is not release-proof. Historical better runs are comparison evidence only until they are revalidated against the single official hardware gate.
- GPU-enabled acceleration on the old line belongs here only as optional comparison evidence. It is not a standalone release decision.

## Preview Route Decision Snapshot

| Route | Current role | Current status | Interpretation | Evidence |
| --- | --- | --- | --- | --- |
| newer `actual-primary-lane` | Story `1.30` bounded evidence lane | `bounded No-Go` | repeated approved-hardware reruns failed the official `preset-applied visible <= 3000ms` gate; further open-ended tuning is paused | `docs/runbooks/preview-track-route-decision-20260418.md` |
| older `resident first-visible` line | Story `1.10` closed baseline | `closed No-Go baseline` | latest one-session rerun closed the baseline evidence package, but still failed the official `preset-applied visible <= 3000ms` gate; keep it as comparison evidence only | `docs/runbooks/current-actual-lane-handoff-20260419.md`; `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`; `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\` |
| reserve path | Story `1.26` active reserve path | `Go` | latest requested hardware-validation passed `5/5` on `session_000000000018aabe5833c11d8c`; all captures closed with honest per-capture full-preset route evidence and official timing `2387ms ~ 2480ms`. | `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`; `_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md`; `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aabe5833c11d8c\`; `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777442288984\run-summary.json` |
| success-side default / rollback gate | Story `1.31` | `unopened` | kept closed unless a success-side fallback or rollback decision is explicitly reopened | `docs/runbooks/preview-track-route-decision-20260418.md` |
| GPU-enabled acceleration on old lane | optional comparison evidence | `unproven` | may still be tested for side evidence, but does not change the release gate or the fact that Story `1.26` is the active route | `docs/runbooks/current-actual-lane-handoff-20260419.md` |

## Canonical Release-Gated Stories

| Story | HV checklist IDs | Canonical pre-close status | Supporting notes |
| --- | --- | --- | --- |
| Story 1.4 | HV-02, HV-03, HV-10 | `review` until `Go` | Shared readiness evidence may overlap Story 1.6, but close ownership is tracked here. |
| Story 1.5 | HV-04, HV-05 | `review` until `Go` | Story 1.7 may supply supporting correlation proof, but release close is tracked here. |
| Story 1.6 | HV-02, HV-03, HV-10 | `review` until `Go` | Helper/readiness truth must include reconnect-safe evidence. |
| Story 3.2 | HV-08, HV-11 | `review` until `Go` | `Completed` truth cannot close from automated state alone. |
| Story 4.2 | HV-01, HV-09 | `review` until `Go` | Validation failure isolation and published-only booth visibility must both hold. |
| Story 4.3 | HV-01, HV-07, HV-12 | `review` until `Go` | Immutable publish, darktable application, and catalogSnapshot drift protection all remain release-gated. |

Supporting regression / follow-up notes:

- Story 1.7 supplies implementation-level capture correlation evidence for `HV-04` and `HV-05`, but it is not the canonical release close owner in this ledger.
- Story 1.8 is the corrective follow-up that proves selected preset apply truth across preview/final boundaries, and it is now `Go` after the 2026-04-29 hardware package tied `session.json` preset binding, `bundle.json` render metadata, preview/final outputs, and diagnostics together for two published presets.
- Story 1.9 is now `Go` after the 2026-04-30 requested hardware validation proved same-capture first-visible fast preview plus later raw-original preset-applied preview truth inside the official `<= 3000ms` gate.
- Story 2.3 is the supporting follow-up validation note for `HV-06`; Story 1.3 is not reopened as an independent close owner.
- Story 1.10 currently serves as the revalidation spec for the old `resident first-visible` lane. Historical implementation evidence exists, but the lane is not counted as release-proof until a new hardware package closes the official `preset-applied visible <= 3000ms` gate recorded in this ledger.

## Sprint Review Gateboard

| Story Key | Automated Pass | Hardware Pass | Go / No-Go | Blocker | Owner | Evidence Path |
| --- | --- | --- | --- | --- | --- | --- |
| 1.4 | Pass | Pass | Go | Closed. HV-02/HV-03/HV-10 package confirmed complete for close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\` |
| 1.5 | Pass | Pass | Go | Closed. HV-04/HV-05 package confirmed from persisted RAW, preview, and session timing metrics. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\` |
| 1.6 | Pass | Pass | Go | Closed. User confirmed `HV-02/HV-03/HV-10` helper/readiness package complete on 2026-04-29; reconnect-safe readiness truth is accepted for story close. | Noah Lee | `_bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md`; `history/camera-helper-troubleshooting-history.md` |
| 1.8 | Pass | Pass | Go | Closed 2026-04-29 16:30 +09:00. Approved booth hardware evidence covers two published presets: `look2` passed 5/5 on `session_000000000018aac3004bc9a1f4` and `Mono Pop` passed 5/5 on `session_000000000018aac34258cf3c8c`. Both sessions have capture-bound preset binding, bundle `xmpTemplatePath` correlation, render-backed preview outputs, post-end final outputs, and completed/handoff-ready truth. Preview/final hashes differ across presets. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac3004bc9a1f4\`; `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac34258cf3c8c\`; `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777447408988\run-summary.json`; `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777447692673\run-summary.json` |
| 1.9 | Pass | Pass | Go | Closed 2026-04-30 11:02 +09:00. Requested validation passed `5/5`; helper same-capture fast preview was observed for all captures, later preset-applied preview truth stayed renderer-owned, and official timing band was `2882ms ~ 2979ms`. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aafff97b2bb744\`; `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777514449927\run-summary.json` |
| 1.10 | Pass | Pass | No-Go for release judgment | Latest one-session rerun package closed helper correlation, same-session replacement, and `capture-ready` recovery on approved hardware, but official `preset-applied visible <= 3000ms` timing still landed at `8972ms`, `7942ms`, and `7967ms`. `sameCaptureFullScreenVisibleMs` (`4685ms`, `3587ms`, `3270ms`) remains comparison evidence only. Official `Go / No-Go` ownership remains in this ledger. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\` |
| 1.26 | Pass | Pass | Go | `2026-04-29 14:59 +09:00` requested validation passed `5/5`. Route evidence is now honest: `engineMode=per-capture-cli`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthProfile=original-full-preset`; official timing band was `2387ms ~ 2480ms`. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aabe5833c11d8c\`; `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777442288984\run-summary.json` |
| 3.2 | Pass | Partial pass | No-Go | 2026-04-30 requested hardware command passed 5/5 generic capture validation, but the run did not enter post-end truth (`postEnd=null`, timing `active`), so `HV-08/HV-11` close evidence is still missing. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777535901457\run-summary.json` |
| 4.2 | Pass | Validation failure isolated, publish proof pending | No-Go | `HV-09` failure was observed, but `HV-01` success evidence is still pending. | Noah Lee | `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md` |
| 4.3 | Pass | Not run | No-Go | `HV-01/HV-07/HV-12` hardware proof is not yet recorded in a canonical close row. | Noah Lee | `TBD` |

## Evidence Registry

### Story 1.4

- story key: `1-4-준비-상태-안내와-유효-상태에서만-촬영-허용`
- HV checklist ID: `HV-02`, `HV-03`, `HV-10`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\`
- executedAt: `2026-03-29T14:55:45Z`
- validator: `Noah Lee (close confirmed 2026-03-31)`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None. HV-02/HV-03/HV-10 close package confirmed complete.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-03-31`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_daylight\2026.03.27\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.5

- story key: `1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백`
- HV checklist ID: `HV-04`, `HV-05`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\`
- executedAt: `2026-03-31T02:42:26Z`
- validator: `Noah Lee (close confirmed 2026-03-31)`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None. HV-04/HV-05 close package confirmed complete from persisted capture timing metrics and preview assets.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-03-31`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\captures\originals\capture_20260331024159916_11d0256f05.CR2`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\captures\originals\capture_20260331024225748_68ebbd3c92.CR2`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\renders\previews\capture_20260331024159916_11d0256f05.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\renders\previews\capture_20260331024225748_68ebbd3c92.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_daylight\2026.03.27\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.6

- story key: `1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단`
- HV checklist ID: `HV-02`, `HV-03`, `HV-10`
- evidence package path: `_bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md`; `history/camera-helper-troubleshooting-history.md`
- executedAt: `2026-04-29 15:35:40 +09:00`
- validator: `Noah Lee`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 and history/camera-helper-troubleshooting-history.md`
- Go / No-Go result: `Go`
- release blocker: `None. User confirmed HV-02/HV-03/HV-10 helper/readiness validation complete; automated readiness-focused regression passed.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for Story 1.6 close. Preview reserve-path regressions remain tracked outside this readiness story.`
- target rerun date: `N/A`
- core evidence paths:
  - `history/camera-helper-troubleshooting-history.md`
  - `history/camera-capture-validation-history.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.8

- story key: `1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결`
- HV checklist ID: `HV-05`, `HV-07`, `HV-08`, `HV-11`, `HV-12`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac3004bc9a1f4\ ; C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac34258cf3c8c\ ; C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777447408988\run-summary.json ; C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777447692673\run-summary.json`
- executedAt: `2026-04-29 16:30 +09:00`
- validator: `Codex hardware validation runner + post-end final evidence inspection`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 + camera-helper-events.jsonl + timing-events.log`
- Go / No-Go result: `Go`
- release blocker: `None. Two-preset approved-hardware package closed selected preset -> XMP apply -> preview/final output truth. look2/preset_new-draft-2 2026.04.10 and Mono Pop/preset_mono-pop 2026.03.27 each passed 5/5 capture validation; both were post-ended to completed/handoff-ready with 5/5 finalReady records and physical final JPEGs. Bundle xmpTemplatePath correlation is present in timing diagnostics, and preview/final SHA-256 values differ across the two presets.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for Story 1.8 close. Rerun only if published preset render contract or final handoff behavior changes.`
- target rerun date: `Closed 2026-04-29`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac3004bc9a1f4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac3004bc9a1f4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac3004bc9a1f4\renders\previews\`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac3004bc9a1f4\renders\finals\`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac34258cf3c8c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac34258cf3c8c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac34258cf3c8c\renders\previews\`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aac34258cf3c8c\renders\finals\`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777447408988\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777447692673\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_new-draft-2\2026.04.10\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_mono-pop\2026.03.27\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.10

- story key: `1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입`
- HV checklist ID: `HV-05` plus current preview-track official gate package
- evidence package path: `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md ; docs/runbooks/current-actual-lane-handoff-20260419.md ; history/recent-session-thumbnail-speed-brief.md ; _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
- executedAt: `historical implementation 2026-04-04; baseline rerun 2026-04-19 22:07 +09:00`
- validator: `Noah Lee`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `observed darktable-cli 5.4.0 via C:\Program Files\darktable\bin\darktable-cli.exe`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `No-Go`
- release blocker: `Latest baseline rerun on session_000000000018a7c3f52370b574 closed the one-session evidence package and kept helper/session truth healthy, but official preset-applied visible timing still landed at 8972ms, 7942ms, and 7967ms. The lane is now revalidated as a baseline evidence lane, but it still fails the official gate and remains non-release-proof. Observed OpenCL/GPU capability could not be closed from darktable-cltest because the command timed out after 120s on the booth PC.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for route decision. Story 1.10 is now a closed No-Go baseline; use it only as comparison evidence while Story 1.26 proceeds.`
- target rerun date: `Closed 2026-04-20`
- core evidence paths:
  - `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`
  - `docs/runbooks/current-actual-lane-handoff-20260419.md`
  - `docs/runbooks/preview-track-route-decision-20260418.md`
  - `history/recent-session-thumbnail-speed-brief.md`
  - `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\diagnostics\camera-helper-events.jsonl`

### Story 1.26

- story key: `1-26-host-owned-local-native-gpu-resident-preview-lane-검증`
- HV checklist ID: `preview-track official gate package`
- evidence package path: `_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md ; docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
- executedAt: `Opened 2026-04-20 10:58 +09:00; many bounded No-Go and false-Go packages collected through 2026-04-29; resident-labeled 12:45 pass retracted by code review at 2026-04-29 14:38 +09:00; honest per-capture full-preset route passed approved hardware at 2026-04-29 14:59 +09:00`
- validator: `Noah Lee`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `explicit per-capture darktable-compatible full-preset route is accepted when route evidence is honest and approved hardware gate passes`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None for Story 1.26 current preview-track judgment. Latest approved hardware run passed with honest per-capture full-preset route evidence and official timing inside 3000ms.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `For release hardening, rerun only if the route evidence changes or if approved hardware behavior regresses. Self-labeled resident strings and metadata-only preset-applied previews remain insufficient.`
- target rerun date: `As required by release/human product review policy`
- core evidence paths:
  - `_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md`
  - `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
  - `docs/runbooks/story-1-26-review-root-cause-and-improvement-direction-20260427.md`
  - `docs/runbooks/story-1-26-agent-operating-guide.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab70e79e5baa8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab70e79e5baa8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab70e79e5baa8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777434275752\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777434275752\run-steps.jsonl`
  - `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab4883e7811d8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab4883e7811d8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab4883e7811d8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777431500206\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777425683850\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa243cb94f60f0\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa243cb94f60f0\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa243cb94f60f0\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777272846171\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa25c4d6f49e20\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa25c4d6f49e20\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa25c4d6f49e20\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777274530300\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa23b7531de818\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa23b7531de818\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa23b7531de818\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777272273229\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa2016a0ccd26c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa2016a0ccd26c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa2016a0ccd26c\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777268284508\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a936ed27302174\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a936ed27302174\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777011920165\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a936fcad8c042c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a936fcad8c042c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777011986840\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93925842ee7b8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93925842ee7b8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777014361267\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9397421a5ad30\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9397421a5ad30\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777014698916\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93a4b32aba8c8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93a4b32aba8c8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777015622619\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93b053f7dbab8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93b053f7dbab8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777016421699\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93b5fa88505d4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93b5fa88505d4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777016810008\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c5bc6cceaa0\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c5bc6cceaa0\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777017892848\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c85f1238a00\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c85f1238a00\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777018073947\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a959d98b7f93f8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a959d98b7f93f8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777050318855\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a95a0fe32405b8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a95a0fe32405b8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777050552254\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa192a350d074c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa192a350d074c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777260672018\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa19847f462f3c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa19847f462f3c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777261059811\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9e0f606e69ed0\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9e0f606e69ed0\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9e0f606e69ed0\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1a7caf7e88b8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1a7caf7e88b8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1a7caf7e88b8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777262125772\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1b8aea1ff3f8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1b8aea1ff3f8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1b8aea1ff3f8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777263286397\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1d117bab2d24\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1d117bab2d24\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1d117bab2d24\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777264963875\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1e41fd6f3360\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1e41fd6f3360\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa1e41fd6f3360\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777266271721\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a91e89791d5370\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a91e89791d5370\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a91e89791d5370\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a91e89791d5370\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776985103763\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f3c5b88c698c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f3c5b88c698c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f3c5b88c698c\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f3c5b88c698c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f61aa8bc153c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f61aa8bc153c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f61aa8bc153c\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f61aa8bc153c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f7ff7a8886b4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f7ff7a8886b4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f7ff7a8886b4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f7ff7a8886b4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f8e4828da598\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f9acaf600638\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f9acaf600638\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f9b49261d518\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f9b49261d518\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fa2f55d79a94\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fa2f55d79a94\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fb29e752039c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fb29e752039c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fc2aba129e1c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fc2aba129e1c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fc5e0caa7cfc\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fc5e0caa7cfc\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fcd1f65b2f7c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7fcd1f65b2f7c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84612e5fc2804\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84612e5fc2804\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84612e5fc2804\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a07a464f044\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a07a464f044\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a07a464f044\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a6d28af1130\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a6d28af1130\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84a6d28af1130\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b21e1691d9c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b21e1691d9c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b21e1691d9c\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b9b92af73ac\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b9b92af73ac\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a84b9b92af73ac\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e59c3f873ffc\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e59c3f873ffc\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e59c3f873ffc\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e59c3f873ffc\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e59c3f873ffc\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e6cb585230d4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e6cb585230d4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e6cb585230d4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e6cb585230d4\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e6cb585230d4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e716e9987b48\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e716e9987b48\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e716e9987b48\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e716e9987b48\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e716e9987b48\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e7702849122c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e7702849122c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e7702849122c\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e7702849122c\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e7702849122c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e892447836f8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e892447836f8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e892447836f8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e892447836f8\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e892447836f8\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e91cef5631a8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e91cef5631a8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e91cef5631a8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e91cef5631a8\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8e91cef5631a8\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fdb7a8e88590\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fdb7a8e88590\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fdb7a8e88590\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fdb7a8e88590\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fdb7a8e88590\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a925271b1710a0\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a925271b1710a0\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a925271b1710a0\diagnostics\camera-helper-startup.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a925271b1710a0\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776992377859\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92639f9a96a6c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92639f9a96a6c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92639f9a96a6c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776993558412\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a926e98958c25c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a926e98958c25c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a926e98958c25c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776994312446\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9292e867e1a68\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9292e867e1a68\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776996807774\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92a6c02e7f2d4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92a6c02e7f2d4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92a6c02e7f2d4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776998171366\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9323a28789b40\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9323a28789b40\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9323a28789b40\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777006753341\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9337745615574\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9337745615574\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9337745615574\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777008115329\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a934f66a92fe80\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a934f66a92fe80\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a934f66a92fe80\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777009760926\run-summary.json`

### Story 3.2

- story key: `3-2-export-waiting과-truthful-completion-안내`
- HV checklist ID: `HV-08`, `HV-11`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777535901457`
- executedAt: `2026-04-30 16:58 +09:00`
- validator: `Noah Lee`
- booth PC: `NOAHLEE`
- camera model: `TBD`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Requested command passed 5/5 capture validation, but the generated session remained active with postEnd null and did not prove Export Waiting / Completed truth for HV-08/HV-11.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `Run an end-of-session hardware pass that records postEnd truth, timing ended/export-waiting or completed transition, and diagnostics evidence.`
- target rerun date: `TBD`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777535901457\run-summary.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777535901457\run-steps.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018ab137c0dfb81f4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018ab137c0dfb81f4\diagnostics\timing-events.log`

### Story 4.2

- story key: `4-2-부스-호환성-검증과-승인-준비-상태-전환`
- HV checklist ID: `HV-01`, `HV-09`
- evidence package path: `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md`
- executedAt: `2026-03-30`
- validator: `Noah Lee`
- booth PC: `NOAHLEE`
- camera model: `N/A (validation failure isolation pass)`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `N/A`
- Go / No-Go result: `No-Go`
- release blocker: `HV-09 failure behavior was confirmed, but HV-01 publish success evidence is still pending.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `Complete a successful published preset pass and attach bundle/catalog proof from the booth surface.`
- target rerun date: `TBD`
- core evidence paths:
  - `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md`
  - `TBD/published/bundle.json`
  - `TBD/preset-catalog/catalog-state.json`

### Story 4.3

- story key: `4-3-승인과-불변-게시-아티팩트-생성`
- HV checklist ID: `HV-01`, `HV-07`, `HV-12`
- evidence package path: `TBD`
- executedAt: `TBD`
- validator: `TBD`
- booth PC: `TBD`
- camera model: `TBD`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Immutable publish and catalogSnapshot drift hardware proof are not yet recorded.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `Finish Story 4.3 implementation and capture immutable publish, darktable differentiation, and active-session drift evidence.`
- target rerun date: `TBD`
- core evidence paths:
  - `TBD/session.json`
  - `TBD/published/bundle.json`
  - `TBD/preset-catalog/catalog-state.json`

## Evidence Row Template

Use this template for the next validation run.

| story key | HV checklist ID | evidence package path | executedAt | validator | booth PC | camera model | darktable pin | helper identifier | Go / No-Go result | release blocker | follow-up owner | rerun prerequisite | target rerun date | core evidence paths |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  | `session.json`; `timing-events.log`; `bundle.json`; `catalog-state.json` |
