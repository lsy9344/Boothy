# Hardware Validation Ledger

Last Updated: 2026-04-16 17:09 +09:00
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
| Story 1.13 | HV-00, HV-04, HV-05, HV-07, HV-08, HV-10, HV-11, HV-12 | `review` until `Go` | Story 1.11 / 1.12 supporting proof does not close guarded cutover; canonical preview architecture release close is tracked here. |
| Story 3.2 | HV-08, HV-11 | `review` until `Go` | `Completed` truth cannot close from automated state alone. |
| Story 4.2 | HV-01, HV-09 | `review` until `Go` | Validation failure isolation and published-only booth visibility must both hold. |
| Story 4.3 | HV-01, HV-07, HV-12 | `review` until `Go` | Immutable publish, darktable application, and catalogSnapshot drift protection all remain release-gated. |

Supporting regression / follow-up notes:

- Story 1.7 supplies implementation-level capture correlation evidence for `HV-04` and `HV-05`, but it is not the canonical release close owner in this ledger.
- Story 1.8 is the corrective follow-up that proves selected preset apply truth across preview/final boundaries; close was confirmed on 2026-04-10 after one canonical package tied `session.json` preset binding, `bundle.json` render metadata, preview/final outputs, and diagnostics together.
- Story 1.9 hardware latency correction package was verified on 2026-04-10 from `session_000000000018a4ff284e180d5c`; same-capture first-visible, later same-slot replacement, split timing, truthful `Preview Waiting`, and completed post-end가 확인됐다. This is recorded as supporting evidence, not a separate release close owner.
- Story 1.10 corrective hardware package was verified on 2026-04-10 from `session_000000000018a5007b5fecf020`; 5 `request-capture`, 5 `file-arrived`, 5 `fast-preview-promoted`, 5 `preview-render-start`, 5 `capture_preview_ready`, 5 `capture_preview_transition_summary`, 5 `recent-session-visible`, `lifecycle.stage=completed`, `postEnd.state=completed`, `5 originals / 5 previews / 1 final`이 함께 확인됐다. Story 1.10 baseline close는 이 패키지로 `Go`로 승격하고, 현재 워크스페이스의 preview-topology regression은 후속 Story 1.11~1.12 scope로 분리 추적한다.
- Story 1.11 established the dedicated renderer sidecar boundary and Story 1.12 locked same-slot truthful replacement, but neither story owns the canonical preview architecture release close. Story 1.13 remains the guarded cutover owner and stays `No-Go` until a post-reset rerun closes the active 1.21-1.25 release contract on approved hardware.
- Story 1.12 supporting hardware run was inspected on 2026-04-10 from `session_000000000018a5007b5fecf020`; 5 `request-capture`, 5 `capture_preview_ready`, 5 `capture_preview_transition_summary`, 5 `recent-session-visible`, and repeated `post-end-evaluated state=completed variant=local-deliverable-ready` were confirmed together with 5 originals / 5 previews / 1 final. `capture_preview_transition_summary`에서 first-visible 이후 preset-applied close까지의 시간은 `replacementMs=3694, 3451, 3852, 3615, 3707`로 실제 기록됐고, `recent-session-pending-visible -> recent-session-visible` 연쇄와 사용자 현장 확인으로 same-slot replacement supporting proof도 확보됐다. 사용자는 2026-04-11에 replayable UI evidence 요구를 waived/pass로 처리하도록 승인했다. 같은 날 stale result reuse, operator recovery block, summary metric 회귀 patch를 닫았고, 2026-04-13에는 Story 1.12를 supporting implementation story로 `done` 처리했다. guarded cutover 최종 hardware gate와 canonical release-truth `Go / No-Go`는 계속 Story 1.13이 이어받는다.
- Story 1.19 establishes the replayable promotion-evidence gate. Its contribution is the canonical bundle contract, trace planning scripts, parity oracle rules, and ledger semantics that future reruns must use before claiming `Go`.
- Stories 1.21 through 1.25 now own the active rerun prerequisites for Story 1.13: metric reset, selected-capture evidence reset, local full-screen lane prototype, hardware canary validation, and default/rollback gate. Story 1.20 remains legacy comparison evidence only.
- Story 2.3 is the supporting follow-up validation note for `HV-06`; Story 1.3 is not reopened as an independent close owner.
- Story 1.21 resets preview promotion acceptance: `sameCaptureFullScreenVisibleMs` is the new-track release field for `same-capture preset-applied full-screen visible <= 2500ms`, while legacy `replacementMs` remains comparison-only or backward-compatible alias data.
- Story 1.22 resets the selected-capture evidence chain: bundles must preserve one `sessionId/requestId/captureId`, keep `visibleOwner` / `visibleOwnerTransitionAtMs`, and copy capture-time route/catalog snapshots rather than rereading live policy or whole-session timing logs.
- Preview confirmation also uses the same `2500ms` threshold. `firstVisibleMs`, tiny-preview success, or recent-strip updates cannot replace the new-track release field when this ledger records `Go / No-Go`.
- Legacy Stories 1.18, 1.19, and 1.20 remain `legacy comparison only`; Stories 1.21 through 1.25 own the active forward path and its release wording.
- Story 1.22 does not own prototype, canary, default-route, rollback, or final cutover proof. Those ownership boundaries stay with Stories 1.23, 1.24, 1.25, and 1.13 respectively.

## Sprint Review Gateboard

| Story Key | Automated Pass | Hardware Pass | Go / No-Go | Latency | Parity | Fallback Ratio | Route Policy State | Rollback Evidence | Blocker | Owner | Evidence Path |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1.4 | Pass | Pass | Go | N/A | N/A | N/A | N/A | N/A | Closed. HV-02/HV-03/HV-10 package confirmed complete for close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\` |
| 1.5 | Pass | Pass | Go | RAW save and preview waiting split confirmed | N/A | N/A | N/A | N/A | Closed. HV-04/HV-05 package confirmed from persisted RAW, preview, and session timing metrics. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a1cccdd183a524\` |
| 1.6 | Pass | Pass | Go | N/A | N/A | N/A | N/A | N/A | Closed. HV-02/HV-03/HV-10 package was visually verified and linked evidence was accepted for close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\` |
| 1.8 | Pass | Pass | Go | Preview/final close timings recorded | Pass against published XMP bundle truth | Low in accepted hardware package | Published bundle route only | N/A | Closed. HV-05/HV-07/HV-08/HV-11/HV-12 package confirmed from two published preset sessions with divergent preview/final assets and matching XMP bundle paths. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\ ; C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\` |
| 1.10 | Pass at story-close baseline; current follow-up workspace scope tracked separately | Pass | Go | 5-shot booth package confirmed seam timing | Supporting-only | Low in corrective baseline package | Host route package aligned for corrective run | Supporting-only | Closed. 5-shot completed booth package confirmed Story 1.10 corrective baseline and seam close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\` |
| 1.13 | Pass | Post-reset local-lane release proof not yet recorded | No-Go | `sameCaptureFullScreenVisibleMs` release proof not yet recorded | Selected-capture evidence contract not yet proven in a canonical post-reset rerun bundle | Not yet re-measured on the active local lane | Current local-lane route proof not yet captured in a canonical rerun bundle | One-action rollback proof not yet re-recorded on the active lane | Story 1.23 has returned to `in-progress`, and Story 1.13 still has no post-reset canonical rerun bundle proving `sameCaptureFullScreenVisibleMs <= 2500ms`, selected-capture evidence continuity, repeated approved-hardware local-lane success-path behavior, and one-action rollback. Pre-reset shadow/dedicated-renderer evidence remains comparison-only and cannot promote release close. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\ ; C:\Users\KimYS\Pictures\dabi_shoot\branch-config\preview-renderer-policy.json` |
| 3.2 | Pass | Pass | Go | Export waiting and completed timing confirmed | N/A | N/A | N/A | N/A | Closed. HV-08/HV-11 package confirmed from one failure-isolation session and one completed local-deliverable session. | Noah Lee | `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df139592b950\ ; C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\` |
| 4.2 | Pass | Pass | Go | N/A | N/A | N/A | Published-only booth visibility confirmed | N/A | Closed. `HV-09` failure isolation and `HV-01` published booth visibility were visually confirmed. | Noah Lee | `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md` ; `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_new-draft-2\2026.04.10\bundle.json` ; `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json` |
| 4.3 | Pass | Not run | No-Go | Not recorded | Not recorded | Not recorded | Immutable publish route not re-proved on hardware | Active-session rollback evidence missing | `HV-01/HV-07/HV-12` hardware proof is not yet recorded in a canonical close row. | Noah Lee | `TBD` |

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
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\`
- executedAt: `2026-04-10T10:12:02+09:00`
- validator: `User visual verification confirmed 2026-04-10`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None. HV-02/HV-03/HV-10 close package confirmed complete from linked session evidence and visual verification.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-04-10`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-events.jsonl`
  - `history/camera-helper-troubleshooting-history.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.8

- story key: `1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결`
- HV checklist ID: `HV-05`, `HV-07`, `HV-08`, `HV-11`, `HV-12`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\ ; C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\`
- executedAt: `2026-04-10T13:32:51+09:00`
- validator: `User visual verification + Codex artifact inspection confirmed 2026-04-10`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper fast-preview-ready/v1 via diagnostics/camera-helper-events.jsonl; final correlation confirmed in diagnostics/timing-events.log`
- Go / No-Go result: `Go`
- release blocker: `None. HV-05/HV-07/HV-08/HV-11/HV-12 close package confirmed from two published preset sessions with matching preset/version/XMP bindings and divergent preview/final outputs.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-04-10`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\renders\previews\capture_20260410025910515_dca9711d7a.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\renders\finals\capture_20260410025910515_dca9711d7a.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\diagnostics\camera-helper-events.jsonl`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\renders\previews\capture_20260410043149032_31d55f291d.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4e49821e18790\renders\finals\capture_20260410043149032_31d55f291d.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_daylight\2026.03.27\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_test-look\2026.03.31\bundle.json`
  - `preset_daylight@2026.03.27 -> xmp/template.xmp`
  - `preset_test-look@2026.03.31 -> xmp/test-look.xmp`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 3.2

- story key: `3-2-export-waiting과-truthful-completion-안내`
- HV checklist ID: `HV-08`, `HV-11`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df139592b950\ ; C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\`
- executedAt: `2026-04-10T12:00:23+09:00`
- validator: `User visual verification + Codex artifact inspection confirmed 2026-04-10`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None. HV-08/HV-11 close package confirmed from export-waiting failure isolation and completed local-deliverable evidence.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-04-10`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df139592b950\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df139592b950\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4df863488433c\renders\finals\capture_20260410025910515_dca9711d7a.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.9

- story key: `1-9-fast-preview-handoff와-xmp-preview-교체`
- HV checklist ID: `HV-05`, `HV-07`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\`
- executedAt: `2026-04-10T21:40:00+09:00`
- validator: `User visual verification + Codex artifact inspection confirmed 2026-04-10`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None for Story 1.9 supporting hardware package. Canonical release close ownership remains with the release-gated story set.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-04-10`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4fe8468fea6ac\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\renders\previews\capture_20260410123827511_1dea26842b.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\renders\previews\capture_20260410123906172_33b090bf9b.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a4ff284e180d5c\renders\finals\capture_20260410123906172_33b090bf9b.jpg`

### Story 1.10

- story key: `1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입`
- HV checklist ID: `HV-05`, supporting seam-close package
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\`
- executedAt: `2026-04-10T22:09:50+09:00`
- validator: `User visual verification + Codex artifact inspection confirmed 2026-04-10`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `Go`
- release blocker: `None for Story 1.10 baseline close. Current preview-topology regressions belong to later in-progress stories.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-04-10`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\diagnostics\camera-helper-status.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\renders\previews\capture_20260410130307528_565bdd14a6.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\renders\finals\capture_20260410130307528_565bdd14a6.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_new-draft-2\2026.04.10\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 1.13

- story key: `1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate`
- HV checklist ID: `HV-00`, `HV-04`, `HV-05`, `HV-07`, `HV-08`, `HV-10`, `HV-11`, `HV-12`
- evidence package path: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\ ; C:\Users\KimYS\Pictures\dabi_shoot\branch-config\preview-renderer-policy.json`
- executedAt: `2026-04-10T22:02:37+09:00`
- validator: `Codex artifact inspection confirmed 2026-04-11`
- booth PC: `NOAHLEE`
- camera model: `Canon EOS 700D`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `canon-helper-status/v1 via diagnostics/camera-helper-status.json`
- Go / No-Go result: `No-Go`
- release blocker: Story 1.13 still lacks a post-reset canonical rerun bundle that proves `sameCaptureFullScreenVisibleMs <= 2500ms`, preserves the Story 1.22 selected-capture evidence contract, shows repeated approved-hardware local-lane success-path behavior from the Stories 1.23-1.25 track, and records one-action rollback. Pre-reset shadow/dedicated-renderer evidence remains comparison-only and cannot close guarded cutover.
- follow-up owner: `Noah Lee`
- rerun prerequisite: Run a new Story 1.13 hardware package against the post-reset 1.21-1.25 contract using capture-time route/catalog snapshots, selected-capture-only evidence, approved-hardware local-lane validation, and one-action rollback proof; keep release status at `No-Go` until that rerun records `Go`.
- activation bundle rule: Use capture-time snapshots (`captured-preview-renderer-policy.json`, `captured-catalog-state.json`) rather than rereading live policy/catalog state during bundle assembly.
- target rerun date: `TBD`
- core evidence paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\session.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\diagnostics\timing-events.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\renders\previews\capture_20260410130307528_565bdd14a6.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a5007b5fecf020\renders\finals\capture_20260410130307528_565bdd14a6.jpg`
  - `C:\Users\KimYS\Pictures\dabi_shoot\branch-config\preview-renderer-policy.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_new-draft-2\2026.04.10\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

### Story 4.2

- story key: `4-2-부스-호환성-검증과-승인-준비-상태-전환`
- HV checklist ID: `HV-01`, `HV-09`
- evidence package path: `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md`
- executedAt: `2026-04-11T12:14:53+09:00`
- validator: `User visual verification confirmed 2026-04-11`
- booth PC: `NOAHLEE`
- camera model: `N/A (published booth visibility confirmation)`
- darktable pin: `release-5.4.1 / c3f96ca`
- helper identifier: `N/A`
- Go / No-Go result: `Go`
- release blocker: `None. HV-09 failure isolation and HV-01 published booth visibility were visually confirmed.`
- follow-up owner: `Noah Lee`
- rerun prerequisite: `None.`
- target rerun date: `Closed 2026-04-11`
- core evidence paths:
  - `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\published\preset_new-draft-2\2026.04.10\bundle.json`
  - `C:\Users\KimYS\Pictures\dabi_shoot\preset-catalog\catalog-state.json`

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

`sameCaptureFullScreenVisibleMs` is the required decision field for new-track release judgment. `replacementMs` remains comparison-only / backward-compatible alias evidence.

| story key | HV checklist ID | evidence package path | executedAt | validator | booth PC | camera model | darktable pin | helper identifier | Go / No-Go result | sameCaptureFullScreenVisibleMs (primary) | replacementMs (legacy alias) | parity | fallback ratio | route policy state | rollback evidence | release blocker | follow-up owner | rerun prerequisite | target rerun date | core evidence paths |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | `session.json`; `timing-events.log`; `preview-promotion-evidence.jsonl`; `bundle.json`; `catalog-state.json`; `preview-renderer-policy.json` |
