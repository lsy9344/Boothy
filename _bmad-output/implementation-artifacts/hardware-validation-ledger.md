# Hardware Validation Ledger

Last Updated: 2026-04-22 11:38 +09:00
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
- Story `1.26` is now the officially opened reserve path for the next preview-route attempt.
- This worktree no longer treats the older `resident first-visible` line as the active execution lane; it remains comparison evidence only.
- That older line is not release-proof. Historical better runs are comparison evidence only until they are revalidated against the single official hardware gate.
- GPU-enabled acceleration on the old line belongs here only as optional comparison evidence. It is not a standalone release decision.

## Preview Route Decision Snapshot

| Route | Current role | Current status | Interpretation | Evidence |
| --- | --- | --- | --- | --- |
| newer `actual-primary-lane` | Story `1.30` bounded evidence lane | `bounded No-Go` | repeated approved-hardware reruns failed the official `preset-applied visible <= 3000ms` gate; further open-ended tuning is paused | `docs/runbooks/preview-track-route-decision-20260418.md` |
| older `resident first-visible` line | Story `1.10` closed baseline | `closed No-Go baseline` | latest one-session rerun closed the baseline evidence package, but still failed the official `preset-applied visible <= 3000ms` gate; keep it as comparison evidence only | `docs/runbooks/current-actual-lane-handoff-20260419.md`; `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`; `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\` |
| reserve path | Story `1.26` active reserve path | `active / official No-Go` | the latest approved-hardware rerun removed the first-shot duplicate render overlap reject and kept truthful same-session close healthy, but the official `preset-applied visible <= 3000ms` gate still failed at `5905ms`, `5982ms`, `5763ms`, `5864ms`, and `5803ms`; route remains active only as the current truthful-close latency debugging lane | `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`; `_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md`; `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\` |
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
- Story 1.8 is the corrective follow-up that proves selected preset apply truth across preview/final boundaries, and it remains `review` until one hardware package ties `session.json` preset binding, `bundle.json` render metadata, preview/final outputs, and diagnostics together.
- Story 2.3 is the supporting follow-up validation note for `HV-06`; Story 1.3 is not reopened as an independent close owner.
- Story 1.10 currently serves as the revalidation spec for the old `resident first-visible` lane. Historical implementation evidence exists, but the lane is not counted as release-proof until a new hardware package closes the official `preset-applied visible <= 3000ms` gate recorded in this ledger.

## Sprint Review Gateboard

| Story Key | Automated Pass | Hardware Pass | Go / No-Go | Blocker | Owner | Evidence Path |
| --- | --- | --- | --- | --- | --- | --- |
| 1.4 | Pass | Pass | Go | Closed. HV-02/HV-03/HV-10 package confirmed complete for close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\` |
| 1.5 | Pass | Pass | Go | Closed. HV-04/HV-05 package confirmed from persisted RAW, preview, and session timing metrics. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\` |
| 1.6 | Pass | Partial helper/readiness proof | No-Go | Reconnect-safe `HV-10` package and canonical helper metadata were not normalized into one close row. | Noah Lee | `history/camera-helper-troubleshooting-history.md` |
| 1.8 | Pass | User field observation recorded; canonical package still missing | No-Go | 2026-04-03 최신 재현 세션에서 `Preview Waiting`은 즉시 보였지만 fast preview는 여전히 비어 있었고, 약 `3.3초 ~ 3.4초` 뒤 render-backed preset preview만 나타났다. `file-arrived`는 fast thumbnail 시도보다 먼저 닫혔으나 helper가 `fast-thumbnail-download-failed` 뒤 customer-visible fast preview를 만들지 못했다. `HV-05/HV-07/HV-08/HV-11/HV-12` canonical evidence는 아직 한 회차로 묶이지 않았다. | Noah Lee | `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md` |
| 1.10 | Pass | Pass | No-Go for release judgment | Latest one-session rerun package closed helper correlation, same-session replacement, and `capture-ready` recovery on approved hardware, but official `preset-applied visible <= 3000ms` timing still landed at `8972ms`, `7942ms`, and `7967ms`. `sameCaptureFullScreenVisibleMs` (`4685ms`, `3587ms`, `3270ms`) remains comparison evidence only. Official `Go / No-Go` ownership remains in this ledger. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7c3f52370b574\` |
| 1.26 | Pass | Pass package collected; verdict No-Go | No-Go | Latest approved-hardware rerun `session_000000000018a88d53fa8f00c4` confirmed the first-shot duplicate render overlap reject is gone: startup reached `camera-ready`, all five captures closed as `previewReady`, every capture had `preview.kind = preset-applied-preview` plus non-null `xmpPreviewReadyAtMs`, helper correlation closed as `capture-accepted -> file-arrived -> fast-preview-ready`, and all five timing closes kept `truthOwner=display-sized-preset-applied`. But the official gate still failed at `5905ms`, `5982ms`, `5763ms`, `5864ms`, and `5803ms`. The remaining blocker is truthful-close latency, not startup/save, false-ready, or duplicate-render overlap. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\` |
| 3.2 | Pass | Missing | No-Go | `HV-08/HV-11` execution and evidence package are not yet recorded. | Noah Lee | `TBD` |
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
- evidence package path: `history/camera-helper-troubleshooting-history.md`
- executedAt: `2026-03-29T22:01:35+09:00`
- validator: `Noah Lee (retro normalization pending)`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 and history/camera-helper-troubleshooting-history.md`
- Go / No-Go result: `No-Go`
- release blocker: `The previous pass report is not yet normalized into one canonical row with reconnect-safe evidence, booth/operator captures, and helper metadata.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `Capture blocked, ready, disconnect, recovery-status, and fresh camera-status evidence in one linked package.`
- target rerun date: `TBD`
- core evidence paths:
  - `history/camera-helper-troubleshooting-history.md`
  - `history/camera-capture-validation-history.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.8

- story key: `1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결`
- HV checklist ID: `HV-05`, `HV-07`, `HV-08`, `HV-11`, `HV-12`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a2aa911a1263d8\ ; _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
- executedAt: `2026-04-03T08:17:41+09:00`
- validator: `user field observation + Codex artifact inspection`
- booth PC: `TBD`
- camera model: `TBD`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `camera-helper-events.jsonl + timing-events.log (file-arrived before thumbnail attempt, then fast-thumbnail-download-failed / no fast-preview-ready)`
- Go / No-Go result: `No-Go`
- release blocker: `2026-04-03 직접 점검한 세션 session_000000000018a2aa911a1263d8에서 helper는 file-arrived를 먼저 기록해 저장 완료 경계를 닫았지만, 이어진 fast preview 단계에서는 fast-thumbnail-download-failed 뒤 fast-preview-ready를 만들지 못했다. host fast-preview-promoted와 session timing fastPreviewVisibleAtMs도 비어 있었고 고객 화면에는 약 3.3초 ~ 3.4초 뒤 render-backed preset-applied preview만 도달했다. Selected preset -> XMP apply -> preview/final differentiation package와 selected preset -> first-visible fast preview -> same-slot replacement package는 여전히 one-run canonical evidence로 기록되지 않았다.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `적용한 helper fallback 보강 뒤 approved booth hardware에서 재실행해 camera-helper-events.jsonl에 fast-preview-ready 또는 fast-preview-fallback-failed가 어떻게 남는지 확인하고, same-slot fast preview first-visible 여부와 later preset replacement 여부를 session.json / timing-events.log / bundle evidence와 함께 한 패키지로 다시 수집할 것.`
- target rerun date: `TBD`
- core evidence paths:
  - `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a2aa911a1263d8\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a2aa911a1263d8\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a2aa911a1263d8\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_daylight\2026.03.27\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_test-look\2026.03.31\bundle.json`
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
- executedAt: `Opened 2026-04-20 10:58 +09:00; software scope advanced 2026-04-20 11:37 +09:00; hardware package collected 2026-04-20 11:54 +09:00; owner-attribution rerun collected 2026-04-20 12:46 +09:00; field failures captured 2026-04-20 13:27 +09:00, 14:02 +09:00, 14:17 +09:00, 14:31~14:32 +09:00, 14:41 +09:00, 14:59 +09:00, 15:17 +09:00, 15:21 +09:00, and 15:29 +09:00; corrective software patch verified 2026-04-20 afternoon; rejected warm-up attempt recorded 2026-04-22 00:40 +09:00; rollback/current-code rerun collected 2026-04-22 00:40 +09:00; duplicate-render-overlap rerun rejected 2026-04-22 11:21 +09:00; post-patch approved rerun collected 2026-04-22 11:38 +09:00`
- validator: `Noah Lee`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `darktable remains parity/fallback/final-export boundary; active hot-path owner TBD by implementation`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `No-Go`
- release blocker: `Latest approved-hardware rerun session_000000000018a88d53fa8f00c4 confirmed the single-lane guardrail removed the first-shot duplicate render overlap reject: startup reached `camera-ready`, all five captures closed as `previewReady`, every capture had `preview.kind = preset-applied-preview` plus non-null `xmpPreviewReadyAtMs`, helper correlation closed as `capture-accepted -> file-arrived -> fast-preview-ready`, and timing evidence kept `truthOwner=display-sized-preset-applied` on all five closes. But the official reserve-path gate still failed at `5905ms`, `5982ms`, `5763ms`, `5864ms`, and `5803ms`, while per-capture `preview-render-ready elapsedMs` remained `3717ms`, `3617ms`, `3615ms`, `3618ms`, and `3715ms`. Story 1.26 therefore remains blocked on truthful-close latency, not on startup/save, false-ready, or duplicate-render overlap correctness.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `Land a new truthful-close hot-path reduction that keeps `previewReady` truth-first, then collect one more approved-hardware rerun and re-read the official `preset-applied visible <= 3000ms` gate in the same package.`
- target rerun date: `After the next truthful-close reduction candidate is implemented`
- core evidence paths:
  - `_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md`
  - `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a7f0faf87fd164\diagnostics\camera-helper-status.json`
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

### Story 3.2

- story key: `3-2-export-waiting과-truthful-completion-안내`
- HV checklist ID: `HV-08`, `HV-11`
- evidence package path: `TBD`
- executedAt: `TBD`
- validator: `TBD`
- booth PC: `TBD`
- camera model: `TBD`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Export Waiting / Completed hardware proof is not yet recorded.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `Run an end-of-session hardware pass with postEnd truth, failure isolation, and diagnostics evidence.`
- target rerun date: `TBD`
- core evidence paths:
  - `TBD/session.json`
  - `TBD/diagnostics/timing-events.log`
  - `TBD/preset-catalog/catalog-state.json`

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
