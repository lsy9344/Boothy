# Hardware Validation Ledger

Last Updated: 2026-04-04 23:36 +09:00
Sprint Artifact Owner: Boothy sprint operator
Canonical Path: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

## Policy Summary

- Truth-critical stories do not move to `done` from automated evidence alone.
- `done` requires both `automated pass` and a canonical hardware ledger row marked `Go`.
- If hardware evidence is missing, incomplete, or recorded as `No-Go`, the story stays in `review` or returns to `review`.
- Release promotion stays on `release hold` until every gated story needed for the release baseline has a `Go` row in this ledger.

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
- Story 1.10 is administratively closed as `done` after scope reclassification moved structural experiment and field-proof ownership to Story 1.11.
- Story 2.3 is the supporting follow-up validation note for `HV-06`; Story 1.3 is not reopened as an independent close owner.

## Sprint Review Gateboard

| Story Key | Automated Pass | Hardware Pass | Go / No-Go | Blocker | Owner | Evidence Path |
| --- | --- | --- | --- | --- | --- | --- |
| 1.4 | Pass | Pass | Go | Closed. HV-02/HV-03/HV-10 package confirmed complete for close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\` |
| 1.5 | Pass | Pass | Go | Closed. HV-04/HV-05 package confirmed from persisted RAW, preview, and session timing metrics. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\` |
| 1.6 | Pass | Partial helper/readiness proof | No-Go | Reconnect-safe `HV-10` package and canonical helper metadata were not normalized into one close row. | Noah Lee | `history/camera-helper-troubleshooting-history.md` |
| 1.8 | Pass | User field observation recorded; canonical package still missing | No-Go | 2026-04-03 최신 재현 세션에서 `Preview Waiting`은 즉시 보였지만 fast preview는 여전히 비어 있었고, 약 `3.3초 ~ 3.4초` 뒤 render-backed preset preview만 나타났다. `file-arrived`는 fast thumbnail 시도보다 먼저 닫혔으나 helper가 `fast-thumbnail-download-failed` 뒤 customer-visible fast preview를 만들지 못했다. `HV-05/HV-07/HV-08/HV-11/HV-12` canonical evidence는 아직 한 회차로 묶이지 않았다. | Noah Lee | `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md` |
| 1.10 | Pass | Scope reclassified; field proof moved to 1.11 | Go | Resident first-visible baseline, per-session seam events, and automated regression proof are in place. Structural experiment and approved booth rerun ownership now live in Story 1.11, so Story 1.10 is administratively closed. | Noah Lee | `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md` |
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
- release blocker: `2026-04-03 직접 점검한 세션 session_000000000018a2aa911a1263d8에서 helper는 file-arrived를 먼저 기록해 저장 완료 경계를 닫았지만, 이어진 fast preview 단계에서는 fast-thumbnail-download-failed 뒤 fast-preview-ready를 만들지 못했다. host fast-preview-promoted와 session timing fastPreviewVisibleAtMs도 비어 있었고 고객 화면에는 약 3.3초 ~ 3.4초 뒤 render-backed preset-applied preview만 도달했다. Selected preset -> XMP apply -> preview/final differentiation package와 selected preset -> first-visible fast preview -> same-slot replacement -> preview/final differentiation package는 여전히 one-run canonical evidence로 기록되지 않았다.`
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
- HV checklist ID: `HV-05`
- evidence package path: `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
- executedAt: `2026-04-04T23:33:00+09:00`
- validator: `Codex automated regression verification`
- booth PC: `TBD`
- camera model: `TBD`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `per-session seam events + automated regression suite`
- Go / No-Go result: `Go`
- release blocker: `None for Story 1.10. Structural experiment and approved booth rerun responsibility were moved to Story 1.11 by scope reclassification.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `N/A for Story 1.10. Follow Story 1.11 canary prerequisites instead.`
- target rerun date: `TBD`
- core evidence paths:
  - `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
  - `src-tauri/tests/capture_readiness.rs`
  - `src/booth-shell/components/LatestPhotoRail.test.tsx`
  - `src/session-domain/selectors/current-session-previews.test.ts`

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
