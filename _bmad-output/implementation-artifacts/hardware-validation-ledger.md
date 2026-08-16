# Hardware Validation Ledger

Last Updated: 2026-08-17 01:00 +09:00
Sprint Artifact Owner: Boothy sprint operator
Canonical Path: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

## Policy Summary

- Truth-critical stories do not move to `done` from automated evidence alone.
- `done` requires both `automated pass` and a canonical hardware ledger row marked `Go`, except the bounded technology-spike No-Go closure rule below.
- If hardware evidence is missing, incomplete, or recorded as `No-Go`, the story stays in `review` or returns to `review`.
- Release promotion stays on `release hold` until every gated story needed for the release baseline has a `Go` row in this ledger.
- Epic 7 follows evidence dependencies: `HV-13A -> HV-13B -> HV-14 route decision -> HV-15 -> HV-16 adoption decision -> HV-17 -> HV-18A -> HV-18B -> HV-18C -> HV-18D`. A later Go cannot erase a missing or No-Go prerequisite.
- 2026-08-12 correct-course: 120fps+ physical monitor frame evidence for the display endpoint moved from HV-13B to HV-18B. A gate may be re-scoped only by recording the transfer in both rows and in a sprint change proposal; evidence is never deleted, and the receiving gate carries a reopen condition for the story that transferred it. See `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md`.
- Story 7.3 and 7.5 may close their bounded experiment with a documented technology No-Go when comparison evidence and an approved alternative decision are complete. The rejected candidate remains disabled, and HV-18D cannot record release Go without an approved production route.
- 2026-08-16 HV-16 closed as `Technology No-Go`. A resident renderer cannot reach NFR-003 on its own: even a zero-cost display render leaves 4568 ms against a 3000 ms warm p50. The remaining latency is owned by Story 7.6 (render queue wait, publication) and Story 7.8 (camera RAW transfer, present). HV-16 also recorded a new, previously unknown fact — Windows can decode CR2 in-process via `Microsoft.RawImageExtension` — which becomes a Story 7.7 dependency question, not a Story 7.5 adoption.

## Canonical Release-Gated Stories

| Story | HV checklist IDs | Canonical pre-close status | Supporting notes |
| --- | --- | --- | --- |
| Story 1.4 | Historical HV-02, HV-03, HV-10 | `done` / regression evidence | Customer readiness projection and capture-guard evidence is preserved; canonical real-camera release ownership is Story 1.6. |
| Story 1.5 | Historical HV-04, HV-05 | `done` / regression evidence | Customer saved/waiting-state evidence is preserved; canonical capture/render release ownership is Story 1.7 and Story 1.8. |
| Story 1.6 | HV-02, HV-03, HV-10 | `review` until `Go` | Helper/readiness truth must include reconnect-safe evidence. |
| Story 1.7 | HV-04 | `done` only with `Go` | Real capture request, RAW arrival, and session persistence correlation owner. |
| Story 1.8 | HV-05, HV-07, HV-08, HV-11, HV-12 | `review` until `Go` | Preset-applied previewReady, render truth, final differentiation, and drift protection owner. |
| Story 3.2 | HV-08, HV-11 | `review` until `Go` | `Completed` truth cannot close from automated state alone. |
| Story 4.2 | HV-01, HV-09 | `review` until `Go` | Validation failure isolation and published-only booth visibility must both hold. |
| Story 4.3 | HV-01, HV-07, HV-12 | `review` until `Go` | Immutable publish, darktable application, and catalogSnapshot drift protection all remain release-gated. |
| Story 7.1 | HV-13A | `review` until `Go` | Pre-capture viewer readiness, approved monitor targeting, session binding, and physical display-size contract. |
| Story 7.2 | HV-13B | `done` on re-scoped `Go` | Immutable sample publication, opaque double-buffer, one-clock actual-present, and telemetry completeness. Physical frames moved to HV-18B on 2026-08-12; an HV-18B display-endpoint defect returns this story to `review`. |
| Story 7.3 | HV-14 | `done` on technology No-Go + approved route | Enabler closed on 2026-08-16: all fast-source candidates remain disabled and `raw-original + pinned darktable 5.4.1` is the approved production route. |
| Story 7.4 | HV-15 | `done` on `Go` | Closed 2026-08-16 with the approved raw-original + darktable route, real-camera display/delete recovery evidence, and explicit product exceptions for underexposure and unreadable environment/visual-review details. |
| Story 7.5 | HV-16 | `done` on technology No-Go + approved fallback | Adoption decision recorded 2026-08-16 as `Technology No-Go`: WebGL2 cannot decode CR2 and preset operations are only 1.26% of the darktable render, so the resident candidate stays disabled and `raw-original + pinned darktable 5.4.1` remains the approved production route. The 5-person blind review, MTF50, actual-present, and resource measurements are explicitly not passed; they are mandatory only if a resident candidate is reactivated. |
| Story 7.6 | HV-17 (`HV-17A` + `HV-17B`) | `review` until **both** record `Go` | RAW-refined seamless replacement, priority scheduling, stale cancellation, and frame-level integrity. **The two sub-gates are judged independently and neither may inherit the other's result** (AC 5). An HV-18B display-endpoint defect returns this story to `review`. |
| Story 7.7 | HV-18A | `review` until `Go` | Signed inventory and clean offline install/launch/self-check/fixture/upgrade/uninstall reproduction. |
| Story 7.8 | HV-18B | `review` until `Go` | Warm 100-shot performance, quality/privacy, cold/idle/reconnect/burst/failure recovery, and the physical-frame display endpoint evidence inherited from HV-13B on 2026-08-12. |
| Story 7.9 | HV-18C | `review` until `Go` | Staged rollout, active-session protection, old-session compatibility, and rollback evidence. |
| Story 7.10 | HV-18D | `review` until final decision | Canonical final MVP Go/No-Go aggregation, including UX-EV-01 and UX-EV-02. |

## Release Evidence Matrix

| Evidence ID | Scope | Evidence Owner | Reviewers | Required By | Status | Evidence Path |
| --- | --- | --- | --- | --- | --- | --- |
| UX-EV-01 | UX-DR16: WCAG 2.2 AA, semantic HTML, focus placement, modal focus trap, ESC close, focus restoration | QA / Release | UX + Architect | Story 7.10 / HV-18D | Not run / No-Go | `TBD` |
| UX-EV-02 | Real-booth touch responsiveness, standing usability, high-contrast simulation, unguided capture success rate | QA / Release | PM + UX | Story 7.10 / HV-18D | Not run / No-Go | `TBD` |

- UX-EV-01 and UX-EV-02 are release evidence, not new PRD scope.
- Neither evidence package may be inferred from automated component tests alone.
- HV-18D cannot record Go while either package is missing or No-Go.

Supporting regression / follow-up notes:

- Story 1.4 and Story 1.5 retain their accepted historical Go rows as regression evidence; those rows do not replace the canonical integration owners below.
- Story 1.7 is the canonical `HV-04` capture-correlation owner. Its accepted 2026-03-31 package remains evidence for the real capture path.
- Story 1.8 is the canonical `HV-05` render-backed preview owner and remains `review` until one hardware package ties `session.json` preset binding, `bundle.json` render metadata, preview/final outputs, and diagnostics together.
- Story 2.3 is the supporting follow-up validation note for `HV-06`; Story 1.3 is not reopened as an independent close owner.

## Sprint Review Gateboard

| Story Key | Automated Pass | Hardware Pass | Go / No-Go | Blocker | Owner | Evidence Path |
| --- | --- | --- | --- | --- | --- | --- |
| 1.4 | Pass | Pass | Go | Historical regression close preserved; product real-camera readiness is owned by Story 1.6. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\` |
| 1.5 | Pass | Pass | Go | Historical customer truthfulness close preserved; canonical capture/render release ownership moved to Story 1.7/1.8. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\` |
| 1.6 | Pass | Partial helper/readiness proof | No-Go | Reconnect-safe `HV-10` package and canonical helper metadata were not normalized into one close row. | Noah Lee | `history/camera-helper-troubleshooting-history.md` |
| 1.7 | Pass | Pass | Go | Canonical HV-04 capture-correlation ownership; accepted 2026-03-31 package preserved. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\` |
| 1.8 | Pass | User field observation recorded; canonical package still missing | No-Go | 2026-04-03 최신 재현 세션에서 `Preview Waiting`은 즉시 보였지만 fast preview는 여전히 비어 있었고, 약 `3.3초 ~ 3.4초` 뒤 render-backed preset preview만 나타났다. `file-arrived`는 fast thumbnail 시도보다 먼저 닫혔으나 helper가 `fast-thumbnail-download-failed` 뒤 customer-visible fast preview를 만들지 못했다. `HV-05/HV-07/HV-08/HV-11/HV-12` canonical evidence는 아직 한 회차로 묶이지 않았다. | Noah Lee | `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md` |
| 3.2 | Pass | Missing | No-Go | `HV-08/HV-11` execution and evidence package are not yet recorded. | Noah Lee | `TBD` |
| 4.2 | Pass | Validation failure isolated, publish proof pending | No-Go | `HV-09` failure was observed, but `HV-01` success evidence is still pending. | Noah Lee | `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md` |
| 4.3 | Pass | Not run | No-Go | `HV-01/HV-07/HV-12` hardware proof is not yet recorded in a canonical close row. | Noah Lee | `TBD` |
| 7.1 | Not run | Not run | No-Go | Viewer readiness and physical display-size contract are not implemented or validated. | Noah Lee | `TBD` |
| 7.2 | Pass | Software evidence passed / physical frame pending | No-Go | 70 hardware captures produced 140 valid generations and 120/120 measured actual-present rows; 120fps+ photon/transition evidence remains unavailable. | Noah Lee | `tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/` |
| 7.3 | Pass | 35-capture real-camera comparison complete | Technology No-Go / route approved | All 105 route rows produced 0 accepted fast sources. Noah Lee approved `raw-original + pinned darktable 5.4.1` as the production route on 2026-08-16; rejected candidates remain disabled and the Enabler is closed. | Noah Lee | `tests/hardware/capture-source/run-20260813-115426-hv14/` |
| 7.4 | Pass | Current-lighting rerun: 5/5 displayed plus delete recovery | Go | Approved raw-original + darktable route; 5/5 actual-present, preset switch, delete-to-standby and recovery passed. Underexposure and unreadable environment/visual-review details are explicit Story 7.4 product exceptions, not fabricated passes. Story 7.5 must perform its own renderer parity review. | Codex (operator: Noah Lee) | `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/` |
| 7.5 | Pass | Component-level measurement complete / no live capture session | Technology No-Go / route retained | 420 darktable one-shot runs on the HV-14 real-capture corpus show preset operations cost 47 ms of a 3716 ms render (1.26%), so a resident engine that only replaces preset work cannot help. WebGL2 in WebView2 cannot decode CR2 at all. Windows WIC can decode CR2 in-process at 1284 ms median, but it depends on the unapproved `Microsoft.RawImageExtension` Store package and fails parity (SSIM 0.013–0.159, ΔE00 median 22.96–26.56 against SSIM ≥ 0.95 / ≤ 3). Even a free renderer leaves 4568 ms against the 3000 ms warm p50. Blind-review panel and MTF50 remain open and are recorded as not-passed. | Codex (operator: Noah Lee) | `tests/hardware/resident-renderer/hv-16/` |
| 7.6 | Pass | Real EOS 700D tier rerun complete; cancellation evidence incomplete | No-Go / route rejected | Normal-exposure RAW 3×3 preset measurement found no detail gain (MTF50 ratio 0.99947, five regressions) and look drift (median ΔE00 4.52, p95 15.01). The refined route is now explicitly `tier-not-justified` and stays off. HV-17A still lacks the four live cancellation rounds and a new 5-shot burst. | Codex (operator: Noah Lee) | `tests/hardware/raw-refined/run-20260817-023031-hv17-rerun/` |
| 7.7 | Not run | Not run | No-Go | Signed complete installer and clean offline reproduction are pending. | Noah Lee | `TBD` |
| 7.8 | Not run | Not run | No-Go | 100-shot performance and recovery evidence are pending. | Noah Lee | `TBD` |
| 7.9 | Not run | Not run | No-Go | Staged rollout, active-session protection, and rollback evidence are pending. | Noah Lee | `TBD` |
| 7.10 | Not run | Not run | No-Go | Final MVP evidence aggregation, UX-EV-01 accessibility, and UX-EV-02 real-booth usability decisions are pending. | Noah Lee | `TBD` |

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

### Story 1.7

- story key: `1-7-실카메라-capture-round-trip과-raw-handoff-correlation`
- HV checklist ID: `HV-04`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\`
- executedAt: `2026-03-31T02:42:26Z`
- validator: `Noah Lee (ownership normalized 2026-08-11)`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None for HV-04. Accepted persisted RAW and correlation package transferred from the historical Story 1.5 close row without deleting that regression record.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for the accepted HV-04 package.`
- target rerun date: `Closed 2026-03-31`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\captures\originals\capture_20260331024159916_11d0256f05.CR2`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\captures\originals\capture_20260331024225748_68ebbd3c92.CR2`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\diagnostics\camera-helper-status.json`

### Story 1.8

Canonical evidence package name: `Selected preset -> XMP apply -> preview/final differentiation package`.

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
- release blocker: `2026-04-03 직접 점검한 세션 session_000000000018a2aa911a1263d8에서 helper는 file-arrived를 먼저 기록해 저장 완료 경계를 닫았지만, 이어진 fast preview 단계에서는 fast-thumbnail-download-failed 뒤 fast-preview-ready를 만들지 못했다. host fast-preview-promoted와 session timing fastPreviewVisibleAtMs도 비어 있었고 고객 화면에는 약 3.3초 ~ 3.4초 뒤 render-backed preset-applied preview만 도달했다. selected preset -> first-visible fast preview -> same-slot replacement -> preview/final differentiation package는 여전히 one-run canonical evidence로 기록되지 않았다.`
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

### Story 3.2

- story key: `3-2-export-waiting과-truthful-completion-안내`
- HV checklist ID: `HV-08`, `HV-11`
- evidence package path: `tests/hardware/capture-source/run-20260813-115426-hv14/` (preflight only; qualifying evidence pending)
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

### Story 7.1

- story key: `7-1-전용-관람-창-준비와-화면-크기-계약`
- HV checklist ID: `HV-13A`
- evidence package path: `tests/hardware/viewer-present/run-20260811-155154`
- Go / No-Go result: `Go`
- executed at: `2026-08-11 16:02:54 +09:00`
- validator: `Codex (operator: Noah Lee)`
- booth PC: `NOAH_WIN`
- camera model: `Canon EOS 700D`
- helper identifier: `local Debug canon-helper.exe; camera-helper-sidecar/v1; canon-edsdk 13.19.0`
- release blocker: `None for HV-13A. The previous viewer recreation hang/blank blocker is closed by same-device recovery and post-recovery capture evidence.`
- follow-up owner: `None`
- rerun prerequisite: `N/A`
- hardware verification: `Approved DISPLAY3 ran fullscreen at 1920x1080 DPR 1; viewer converged before capture, recreated at epoch 2 after forced close, and enabled a second successful Canon capture. Invalid monitor configuration remained fail-closed without creating a viewer on an arbitrary display. Session replacement converged at a higher revision, and viewer routing exposed no booth controls.`
- profile scope: `The production-approved customer display is DISPLAY3 1080p. 1440p/4K dimension calculations and single-monitor fallback remain covered by automated contract tests; the connected portrait 4K panel is an operator display, not an approved customer display.`
- target rerun date: `N/A`
- required evidence: `viewer profile`; `viewer snapshot`; `monitor targeting`; `session binding`; `listener/layout readiness`; `reload/gap recovery`; `physical display-size contract`
- core evidence paths: `pre-capture/viewer-readiness-approved-1080p.json`; `capture/capture-round-trip.json`; `recovery/viewer-readiness-epoch-2.json`; `recovery/capture-after-recreation.json`; `monitor-targeting/capture-blocked.json`; `recovery/session-switch.json`

### Story 7.2

- story key: `7-2-immutable-sample과-actual-present-계측`
- HV checklist ID: `HV-13B`
- evidence package path: `tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/`
- Go / No-Go result: `Go` (re-scoped 2026-08-12 — see scope transfer below)
- executedAt: `2026-08-11T20:35:00+09:00` to `2026-08-11T20:50:00+09:00`
- validator: `Codex (operator: Noah Lee)`
- booth PC / camera / monitor: `NOAH_WIN / Canon EOS 700D / approved DISPLAY3 1920x1080`
- release blocker: `None for the re-scoped gate. This gate does NOT prove the physical monitor frame; that evidence is owned by HV-18B.`
- scope transfer (2026-08-12 correct-course): `120fps+ physical monitor frames, compositor-to-photon offset, and frame-level zero-transition-defect moved to Story 7.8 / HV-18B. Rationale: the physical frame proves the compositor-to-photon interval, at most 0.017 s on a 60 Hz monitor, while the measured customer-visible latency is 3.7-4.5 s and is owned by the source and renderer stories; and the subject worth filming, an actual preset-applied photo, only reaches the screen after Story 7.4. Approved by Noah Lee. Full rationale and defence lines: _bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md.`
- reopen condition: `An HV-18B transition defect attributable to the display endpoint (blank, spinner, stale image, wrong capture, crop jump, scale jump, tier downgrade) returns Story 7.2 to review and this row to No-Go.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for this gate. The telemetry-completeness fix shipped after this package was recorded and rides on the next booth session (HV-14) using tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1.`
- target rerun date: `N/A`
- required evidence (re-scoped): `immutable generations`; `pointer commits`; `decode/dimension/correlation checks`; `timing spans`; `telemetry completeness — exactly one terminal row per committed generation`; `visible-standby vs hidden-prewarm comparison`; `AC 4 window-event report`
- evidence NOT provided by this gate: `actual monitor frames`; `compositor-to-photon offset`; `frame-level zero-transition-defect report` — all owned by HV-18B
- observed evidence: `70 physical captures produced 140/140 integrity-valid generations and 120/120 measured actual-present rows, all presented/measured/trusted with zero viewer-window events after input. Visible-standby combined p50/p95/max was 3710.617/4280.416/4672.717 ms; hidden-prewarm was 3774.393/4540.441/15024.476 ms. Qualifying physical high-speed frames: 0.`
- core evidence paths: `result.md`; `environment.md`; `visible-standby/integrity-check.json`; `hidden-prewarm/integrity-check.json`; `timing/summary.json`; `window-events/summary.json`; `ab/summary.json`; `frames/README.md`
- interim observation (not an HV-13B qualifying run): `2026-08-12 daylight spot check at tests/hardware/viewer-present/run-20260812-081157-hv13b-light-rerun/. Two real Canon captures displayed A then B full screen on approved DISPLAY3 with 4/4 integrity-valid generations, 4/4 measured actual-present rows, and zero viewer-window events after input. Two defects surfaced outside the display path: the first photo card stayed in the rendering state after its preview finished because readiness carried only latestCapture (fixed, four regression tests, not yet re-validated on hardware), and both XMP previews exceeded the 5000 ms budget at 7109/7632 ms with darktable-cli alone taking 4326/4735 ms. The preview budget overrun stays owned by Stories 7.3 through 7.6 and the threshold was not raised. Captured photos were visibly underexposed, which is a camera setup action, not a display-path defect.`

- interim observation (not a qualifying physical-frame run): `2026-08-12 hidden-prewarm/offscreen direct run at tests/hardware/viewer-present/run-20260812-172000-hv13b-offscreen-direct/. Five real Canon captures produced 10/10 integrity-valid immutable generations; approved DISPLAY3 showed final sample B full screen. Nine terminal rows were presented/measured/trusted with zero viewer-window events. Generation request_0000000000000658d5590a55f0-000008 was committed but has no terminal viewer-present row, so evidence completeness is 9/10. The prior stale photo-card defect did not recur across five completed cards. Five XMP previews exceeded budget, including one overlapping camera request at 24028 ms. HV-13B remains No-Go because of the missing terminal row and zero qualifying 120fps+ physical frames.`
- follow-up on the missing terminal row (code fixed, not yet re-validated on hardware): `2026-08-12. Root cause traced in the evidence: the host committed and notified generation 000008 with no rejection or telemetry warning, so the viewer report never arrived. The viewer cancelled its after-paint stamp whenever the effect was torn down after the swap had already committed, and that cancel could only ever destroy a legitimate sample. The fifth shutter press landed 22 ms before the 000008 commit while the hidden-prewarm reveal re-applied set_position/show/set_fullscreen on every generation, so a layout re-report tore the effect down inside the one-frame window. Four fixes shipped: the committed swap keeps its stamp until unmount; the host closes any committed generation without a terminal report as present-unreported so exactly one terminal row exists per generation; the hidden-prewarm reveal now runs once per viewer window generation; and present-report validation failures are logged instead of returning silently. Completeness is now checked mechanically by tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1, which reproduces the 9/10 finding on this evidence package and is a required gate for the next run. Nine regression tests were added (three TS, six Rust); the two client tests fail against the pre-fix component. The only remaining HV-13B blocker is the absence of a 120fps+ capture rig.`

### Story 7.3

- story key: `7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교`
- HV checklist ID: `HV-14`
- evidence package path: `tests/hardware/capture-source/run-20260813-115426-hv14/`
- Go / No-Go result: `Technology No-Go / approved alternative route`
- release blocker: `None for the Story 7.3 route decision. All fast-source candidates remain disabled; raw-original + pinned darktable 5.4.1 is the approved production route. This decision does not approve Story 7.4 image quality.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for closure. Rerun only if the team intentionally reopens a rejected fast-source candidate with new evidence.`
- dependency decision (2026-08-12, approved by Noah Lee): `Route A does NOT use LibRaw. CR2 is a TIFF container, so src-tauri/src/capture/embedded_jpeg.rs reads IFD#0 StripOffsets/StripByteCounts directly and validates the extracted bytes with Story 7.2's image_probe. This removes the LGPL-2.1/CDDL license gate and leaves the Story 7.7 offline clean-machine inventory unchanged — no new Rust crate was added. The contract route is named embedded-jpeg, not libraw-embedded-jpeg, so the contract does not name a library the implementation does not use. CR3 (ISO BMFF) is out of scope for this parser and is rejected with a distinct reason; the approved EOS 700D writes CR2. Extraction is read-only and cannot reach RAW truth.`
- target rerun date: `Closed 2026-08-16`
- required evidence: `capability descriptor`; `per-object correlation`; `raw samples`; `p50/p95/max`; `success rate`; `orientation/decode/corruption report`; `quality corpus review`; `approved primary/fallback/No-Go route decision`
- hardware preflight (2026-08-13 11:54 +09:00): `Approved NOAH_WIN detected Canon EOS 700D on USB port 6. Canon helper 0.1.0 initialized EDSDK 13.19.0 and reported exactly one camera with camera-ready. Both measurement modes were unset, C: had 572.6 GB free, and no Canon/Boothy camera-owning process was running. Firmware, lens, card, power, cable/hub, and pre-capture Image Quality remain operator-confirmed prerequisites; start-measurement.ps1 blocks until environment.md has those values.`
- evidence package scaffold (2026-08-13): `tests/hardware/capture-source/hv-14/ — README with the collection plan; check-source-completeness.ps1, which enforces 35 requests × all three canonical routes, exact per-request route membership, warm-up 5 + measured 30, one session/seed, AB+BA order, and a separate final-package gate; and test-check-source-completeness.ps1, which reproducibly proves telemetry and final-package PASS/FAIL boundaries plus six telemetry defects: missing route, broken request matrix, wrong warm-up split, missing required field, mixed sessions, and malformed JSONL. Story 7.2's check-telemetry-completeness.ps1 rides along on the same session.`
- implementation status (2026-08-14 limited hardware run): `Canon helper 42 passed, capture-source integration 20 passed, and the HV-14 synthetic gate passed. Session session_000000000018cbaaed889b09ec completed 35 shutters and 105 comparison rows with no busy/timeout interruption. RAW+JPEG capability and group correlation were confirmed, but embedded/paired admission failed orientation validation and shell admission ran before the later fast-preview-ready event. Source completeness passed. The final package gate remains failed only because the user-directed fixed-composition run cannot provide the four-scene human quality manifest; no quality approval is fabricated.`
- route decision (2026-08-16, approved by Noah Lee): `The three fast-source candidates are closed as Technology No-Go. The approved production route is raw-original + pinned darktable 5.4.1. This closes the Story 7.3 Enabler without promoting any rejected candidate and clears only the Story 7.4 source-route prerequisite. Proposal: _bmad-output/planning-artifacts/sprint-change-proposal-20260816-010151.md.`
- baseline note (2026-08-12): `The comparison must include the shipped incumbent as a third route. The current fast preview is a Windows Shell thumbnail (IShellItemImageFactory, ThumbnailOnly), not an embedded JPEG, and the host fast-preview budget of 120 ms was exhausted in 5 of 5 captures on 2026-08-12. Measured context for the source interval: capture-to-RAW 2806-19524 ms and capture-to-XMP-preview 6804-24028 ms in the same run. A route claim without the incumbent measured under identical conditions is not a comparison.`
- correction recorded during implementation (2026-08-12): `The helper emits four fastPreviewKind values, not two: camera-thumbnail, windows-shell-thumbnail, raw-sdk-preview, raw-fallback-preview. The contract document previously showed embedded-jpeg, a value no implementation produces. Corrected in docs/contracts/camera-helper-sidecar-protocol.md and pinned by a contract test that reads the helper source directly.`
- defect found and fixed before the run (2026-08-12): `CanonSdkCamera.HandleObjectEvent downloaded only the first transfer object per capture and released the rest with no record (Interlocked.Exchange on DownloadStarted). Measuring RAW+JPEG in that state would have produced a false "camera does not support it" conclusion. Completion judgement now lives at request scope via CaptureObjectCorrelator, which pairs objects by EdsDirectoryItemInfo.GroupID, falls back to filename stem plus an arrival window while flagging usedFallbackCorrelation, and reports every rejected object with a reason. The default/off product path does not change ImageQuality or emit source-comparison artifacts; only paired/ab mode enables two-object collection. In single-object mode, non-RAW or unknown objects are explicitly rejected until the RAW original arrives.`

### Story 7.4

- story key: `7-4-화면-적합-immutable-preset-proxy`
- HV checklist ID: `HV-15`
- evidence package path: `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/`
- Go / No-Go result: `Go`
- release blocker: `None within Story 7.4. Underexposure and unreadable environment/visual-review details are explicit product exceptions and not recorded as normal threshold passes. Later Story 7.5 through 7.10 gates remain independent release blockers.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None for Story 7.4 closure. Rerun if the approved hardware, lens family, display profile, darktable pin, preset recipe, or proxy publication contract changes.`
- target rerun date: `Closed 2026-08-16`
- required evidence: `display profile`; `proxy bundle metadata`; `generation manifest`; `publish/decode evidence`; `actual-present spans`; `zero-wrong-frame report`
- evidence NOT provided by this gate: `warm 100-shot performance`; `120fps+ physical monitor frames`; `compositor-to-photon offset` — all owned by HV-18B. `RAW-refined seamless replacement` is owned by HV-17.
- latency note (2026-08-14): `Story 7.4 AC 5 explicitly allows the one-shot renderer to miss the final latency gate. The measured display-fit baseline is ~3.53 s CPU per capture while actual pixel work is ~0.3 s; the remainder is per-capture process and core startup, which Story 7.5 owns. Thresholds must not be raised and slow samples must not be excluded to obtain a pass.`
- mechanical gate (2026-08-14): `tests/hardware/display-proxy/hv-15/check-proxy-evidence.ps1` verified against synthetic fixtures — it accepts one complete run and rejects 11 seeded defects (undersized output, missing provenance, wrong preset version, 384px target, missing terminal row, duplicated terminal row, no proxy generation, mixed fixture/proxy lanes, missing committed asset, an exact-path claim rendered from a fast source, malformed JSONL). `Script PASS is telemetry and provenance completeness only; it is not HV-15 Go.`

- limited real-camera observation (2026-08-14): `One unchanged portrait composition was captured four times with preset_daylight @ 2026.03.27. RAW and product preview persistence succeeded 4/4. Display publication was rejected 4/4 as insufficient-dimensions; an exact reproduction rendered 634x953 against a required 1429x953 display area. No generation committed, so S3-S6, actual-present timing, visual quality, and wrong-frame behavior remain unclaimed.`
- rerun observation (2026-08-15): `The approved booth stack displayed 6/6 real EOS 700D captures after adopting deterministic contain-no-upscale portrait admission. Median qualifying latency was 8.1848655 s and p95/max was 10.464558 s; no slow display sample was excluded. S1-S3 and S6 passed. S4 first exposed a real delete failure; the unbound runtime call was corrected and the rerun removed the RAW, cleared the viewer pointer, appended request-forgotten, and returned the viewer to standby. S5 persisted RAW/preview while blocking proxy publication with proxy-reference-renderer-mismatch, and the production bundle was restored to SHA256 C0D1F57F3AD26B17CB0ECACD628F2F1BCB8D7AD282D2C9A6CCBCD203ED3CA48A. Two setup-time RAW download timeouts required a helper restart and are recorded as camera failures, not omitted display samples.`
- visual quality observation (2026-08-15): `Display composition and correlation are correct, but the captured desk scene is not quality-approved. Daylight mean luma was 13.89/255 with 67.6% below 16; Mono Pop mean luma was 9.79/255 with 82.2% below 16. Lighting/exposure correction is required before Go.`
- environment gap (2026-08-15): `PC, EOS 700D, EDSDK 13.19.0.6400, darktable 5.4.1, WebView2 151.0.4129.78, GTX 1080 driver 32.0.15.6094, and LG FULL HD GSM5B55 1920x1080/DPR1 were recorded. Camera firmware, lens, card, cable/hub, ICC, and HDR remain unknown, so the environment fingerprint checkbox stays open.`
- current-lighting rerun (2026-08-15): `At the user's direction, lighting and exposure were left unchanged. Session session_000000000018cbdf04a181798c produced 5/5 committed and presented generations: three consecutive Daylight captures, one Mono Pop capture after a preset switch, and one Mono Pop recovery capture after deleting the active photo. Delete cleared the viewer to standby and the next capture restored display. No camera-busy, RAW download timeout, or proxy failure occurred. Median qualifying latency was 8.284043 s and p95/max was 9.735243 s. The final mechanical and telemetry-completeness gates passed on the post-delete recovery generation. Daylight mean luma was 12.27/255 and the final Mono Pop mean was 7.39/255, confirming severe underexposure under the accepted unchanged environment; this observation is not converted into a quality approval.`
- product quality exception (2026-08-16, approved by Noah Lee): `The severe underexposure in the 2026-08-15 current-lighting package is accepted for Story 7.4 without changing or claiming the normal quality threshold passed. Original luma measurements and customer-quality risk remain in evidence. This approval does not waive the incomplete environment fingerprint or fabricate missing metrics/blind review. Decision: tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/product-decision-20260816.md.`
- environment and evidence completion (2026-08-16, approved by Noah Lee): `Lens is Canon EF-S 18-55mm bundle lens (exact revision unknown); captures use EDSDK SaveTo Host without a memory card; USB Port 6 uses the existing working cable with direct PC connection and no hub; the customer monitor is LG GSM5B55 1920x1080/60Hz/DPR1; no custom user display ICC association was found and the product output is sRGB/perceptual. Firmware exact version, adapter model, HDR active state, and separate metrics/blind review remain unmeasured facts and are explicitly waived only for Story 7.4. Story 7.5 must produce renderer parity evidence. Decision: tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/environment-completion-20260816.md.`

### Story 7.5

- story key: `7-5-상주-renderer-검증-spike`
- HV checklist ID: `HV-16`
- evidence package path: `tests/hardware/resident-renderer/hv-16/`
- Go / No-Go result: `Technology No-Go`
- release blocker: `None for Story 7.5 closure. The resident prototype exists but remains disabled because no approved real-capture input route exists; raw-original + pinned darktable 5.4.1 remains the production fallback.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `HV-15 Go is recorded. Use the three versioned proxy recipes, preserve the raw-original + pinned darktable exact fallback, and prove an approved real-capture input route before any production Go.`
- target rerun date: `Closed 2026-08-16. Rerun only when a resident candidate or approved direct decoder is intentionally reactivated.`
- required closure evidence: `prototype build`; `input-axis evidence`; `one-shot baseline`; `available parity failure data`; `fallback evidence`; `Technology No-Go adoption decision`
- reactivation-only evidence: `end-to-end raw latency`; `CPU/GPU/memory`; `actual-present`; `MTF50`; `skin ROI`; `5-person blind review with Noah Lee sign-off`
- closure rule: `A fixture-only prototype cannot produce a production Go. Story 7.5 may close Technology No-Go when one independently decisive rejection axis is fully evidenced, the candidate remains disabled, and the exact darktable fallback remains verified. Unrun adoption metrics are not passes and become reactivation prerequisites.`

### Story 7.6

**HV-17 is recorded as two independent gates (2026-08-16).** AC 5 requires that the scheduler
evidence and the seamless-transition evidence be judged separately: **neither gate may inherit the
other's result.** A single combined verdict would let a clean scheduler round carry a transition
defect, or a clean transition carry an orphaned process.

- story key: `7-6-raw-정밀본-무중단-교체`
- HV checklist ID: `HV-17` — recorded as `HV-17A` + `HV-17B`
- evidence package path: `tests/hardware/raw-refined/run-20260817-023031-hv17-rerun/` (normal-exposure rerun; earlier 5-shot run in `run-20260817-003242-hv17/`)
- Go / No-Go result: `HV-17A No-Go`; `HV-17B No-Go`; product disposition `refined route rejected, lane off`
- release blocker: `The rerun captured three normally exposed real EOS 700D RAWs and produced all 9 production-equivalent proxy/refined pairs. The detail axis failed at 0.99947 versus the 1.10 threshold with five regressions; the look axis also failed at median ΔE00 4.52 and median p95 15.01. The route is recorded as tier-not-justified and cannot enter publication. HV-17A still lacks a remediation-run 5-shot burst and the four mandatory live cancellation/process-tree rounds.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `A different refined recipe/renderer must first prove both ≥1.10 detail gain with zero regressions and same-look thresholds on a fresh real RAW 3×3 corpus. Only then can publication, four P1 cancellation rounds, swap recording, and observer testing be reopened.`
- target rerun date: `After a materially different refined approach exists`

#### HV-17A — scheduler / capacity / cancellation / process tree

- required evidence: `scheduler spans on every display-lane generation`; `observed execution order matching priority`; `queue wait p50/p95/max and its share of end-to-end latency`; `burst of 5+ consecutive captures with zero P0 dropouts`; `four cancel rounds (delete, session replace, viewer epoch change, newer capture) each with cancelOrphanCount = 0`; `raw taskkill result logs`; `a round proving the final render still completed`
- evidence NOT provided by this gate: `what was actually on screen` — owned by HV-17B
- 2026-08-17 execution: `5 captures, 0 P0 drops, queue wait p50/p95/max 9/22/22 μs, end-to-end median 7,141,922 μs across 4 trusted-input samples, queue share 0.0001260165%, and one final completion. The four cancel rounds and process-tree raw logs were not run, so HV-17A is No-Go.`

#### HV-17B — seamless tier transition / frame integrity

- required evidence: `pointer history and generation journal for proxy commit → refined commit → refined present`; `identical sourceWidthPx/sourceHeightPx per capture across the two tiers`; `normal-speed screen recording from proxy-first display to RAW stabilisation + 500 ms`; `zero report for blank / spinner / previous-or-other capture / other preset / crop jump / scale jump / tier downgrade / stale overwrite`; `AC 6 detail-axis and look-axis raw data with per-sample values`; `detection test with a control group, 3 observers × 20 trials (10 swap / 10 control, randomised)`
- evidence NOT provided by this gate: `how the scheduler ran` — owned by HV-17A
- detection test pass rule (2026-08-16, approved by Noah Lee): `Swap-trial detection rate must not exceed the control false-positive rate by more than 10 percentage points. A result obtained without a control group is not recorded as a pass — without a comparison it is an observation, not a measurement. The 5-observer × 30-trial format is not used; a detection test does not need that scale, and scale is why Story 7.5 could not close its panel AC.`
- tier justification rule (2026-08-16, approved by Noah Lee): `The detail axis (MTF50) is the tier's condition of existence: median MTF50(refined) ≥ 1.10 × median MTF50(proxy) with zero per-sample regressions, measured on at least 9 slanted-edge pairs. The look axis (median ΔE00 ≤ 3, p95 ≤ 8, clipping increase ≤ 2%p) is not a quality bar but the AC 4 transition-defect judgement: leaving it is refined-look-drift, an AC 4 failure, not an unjustified tier. Story 7.4's visual-approval thresholds are not reused for the detail axis, and the two tiers are never compared against a full-resolution final because the resampler choice would decide the result.`
- 2026-08-17 execution: `5 proxy generations and 5 terminal presents were recorded, but 0 refined generations. The images were near-black and contained no usable slanted edge, so detail/look are not-measured and laneDefaultEligible=false. No actual swap existed to record or show to observers; zero-report, trials.csv, and sign-off were intentionally not fabricated. HV-17B is mechanically No-Go and product-blocked pending a valid rerun.`
- 2026-08-17 remediation rerun: `Three normal-exposure EOS 700D RAW captures × Daylight/Soft Glow/Mono Pop produced 9 proxy/refined pairs with matching dimensions. Detail was tier-not-justified (median refined/proxy MTF50 ratio 0.99947; 5 regressions). Look was refined-look-drift (median ΔE00 4.5213; median p95 ΔE00 15.0056; clipping within threshold). laneDefaultEligible=false. Publication and the observer trial remained closed rather than manufacturing a transition.`

#### Both sub-gates

- evidence NOT provided by HV-17 at all: `120fps+ physical monitor frames`; `compositor-to-photon offset`; `frame-level zero-defect adjudication` — all owned by **HV-18B** (2026-08-12 correct-course). HV-17 does not wait for that rig; Story 7.2/HV-13B already lost a round to its absence.
- reopen condition: `An HV-18B transition defect attributable to the display endpoint (blank, spinner, stale image, wrong capture, crop jump, scale jump, tier downgrade) returns Story 7.6 to review and HV-17 to No-Go. This mirrors the condition Story 7.2 carries.`
- mechanical gate (2026-08-16): `tests/hardware/raw-refined/hv-17/check-raw-refined-evidence.ps1` verified against synthetic fixtures by `test-check-raw-refined-evidence.ps1` — it accepts one complete run and rejects 17 seeded defects, and **each rejection is asserted to fail only its own sub-gate**, which is how the AC 5 independence rule is enforced mechanically. `Script PASS is telemetry, provenance and threshold completeness only; it is not HV-17 Go.` The human items remain: normal-speed recording review, detection-test execution with named observers, and Noah Lee sign-off.
- telemetry completeness (2026-08-16): `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1 was updated to v2 for two generations per capture. The completeness contract is unchanged — exactly one terminal row per committed generation — but the KPI denominator is now the first-qualifying-frame tier only, and a refined row carrying qualifyingLatencyMicros fails the gate. Counting refined presents as KPI samples would double-count each capture and make NFR-003 numbers incomparable with runs that publish no refined tier.`

### Story 7.7

- story key: `7-7-완전한-installer와-clean-offline-재현`
- HV checklist ID: `HV-18A`
- evidence package path: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Signed complete installer and clean offline lifecycle reproduction are missing.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `HV-17 Go and one approved production route with exact fallback.`
- target rerun date: `TBD`
- required evidence: `signed inventory`; `versions and hashes`; `licensing/signing`; `clean offline install`; `launch/self-check`; `fixture display`; `upgrade`; `uninstall`; `missing-inventory refusal`

### Story 7.8

- story key: `7-8-100-shot-성능과-복구-검증`
- HV checklist ID: `HV-18B`
- evidence package path: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Warm 100-shot performance, quality/privacy, and recovery evidence are missing. This gate also owns the physical-frame display endpoint evidence inherited from HV-13B on 2026-08-12.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `HV-18A Go on the complete installed release candidate, plus a 120fps+ external capture rig for the inherited physical-frame evidence.`
- target rerun date: `TBD`
- required evidence: `100-shot raw samples`; `p50/p95/max`; `failure/timeout retention`; `cold first-shot`; `10-minute idle`; `reconnect`; `burst`; `renderer failure`; `app restart`; `quality/privacy`; `zero-wrong-frame report`
- inherited from HV-13B (2026-08-12 correct-course): `120fps+ actual monitor frames across preset-applied and RAW-refined transitions`; `compositor-to-photon offset applied to the reported KPI`; `monitor refresh rate and vsync`; `frame-level zero-transition-defect report (blank, spinner, stale, wrong capture, crop jump, scale jump, tier downgrade)`. A display-endpoint defect here returns Story 7.2 to `review` and HV-13B to `No-Go`. Procedure: `tests/hardware/viewer-present/hv-13b/README.md` section 4.

### Story 7.9

- story key: `7-9-단계적-배포와-rollback-호환성-검증`
- HV checklist ID: `HV-18C`
- evidence package path: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Staged promotion, active-session protection, old-session compatibility, and rollback evidence are missing.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `HV-18B Go on the release candidate selected for rollout validation.`
- target rerun date: `TBD`
- required evidence: `lab/pilot/10-20%/default stages`; `active-session no-force proof`; `pre-upgrade session compatibility`; `old generation pointer recovery`; `single-action fallback`; `rollout/rollback audit`

### Story 7.10

- story key: `7-10-최종-mvp-출시-판정`
- HV checklist ID: `HV-18D`
- evidence package path: `TBD`
- Go / No-Go result: `No-Go`
- release blocker: `Required legacy gates and HV-13A through HV-18C have not all produced an acceptable production evidence package.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `All required legacy gates plus HV-13A, HV-13B, HV-14, HV-15, HV-16, HV-17, HV-18A, HV-18B, and HV-18C completed with an approved production route.`
- target rerun date: `TBD`
- required evidence: `gate matrix`; `approved production route`; `disabled No-Go candidates`; `owner signoff`; `final Go/No-Go rationale`; `release or hold decision`

## Evidence Row Template

Use this template for the next validation run.

| story key | HV checklist ID | evidence package path | executedAt | validator | booth PC | camera model | darktable pin | helper identifier | Go / No-Go result | release blocker | follow-up owner | rerun prerequisite | target rerun date | core evidence paths |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  | `session.json`; `timing-events.log`; `bundle.json`; `catalog-state.json` |
