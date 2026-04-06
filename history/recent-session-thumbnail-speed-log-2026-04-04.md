# 2026-04-04 Recent Session Thumbnail Speed Log

## Source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a315216eca9a58`
- files reviewed:
  - `diagnostics/timing-events.log`
  - `diagnostics/camera-helper-events.jsonl`

## Latest measured captures

Latest session contained 3 recent captures.

1. `request_000000000000064e9cf6868060`
   - `request-capture -> fast-thumbnail-attempted`: about `1.0s`
   - `request-capture -> file-arrived`: about `2.0s`
   - `request-capture -> recent-session-pending-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6466ms`
   - `pending-visible -> recent-session-visible`: about `3.0s`
2. `request_000000000000064e9cf6fc6db0`
   - `request-capture -> fast-thumbnail-attempted`: about `1.0s`
   - `request-capture -> file-arrived`: about `2.0s ~ 3.0s`
   - `request-capture -> recent-session-pending-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6648ms`
   - `pending-visible -> recent-session-visible`: about `4.0s`
3. `request_000000000000064e9cf77e1778`
   - `request-capture -> fast-thumbnail-attempted`: about `1.0s`
   - `request-capture -> file-arrived`: about `2.0s ~ 3.0s`
   - `request-capture -> recent-session-pending-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6405ms`
   - `pending-visible -> recent-session-visible`: about `4.0s`

## Product reading

- `recent-session` first visible is no longer the old `8s ~ 10s` class. It is now around `3s`.
- The customer-facing preset-applied close is still too slow. Latest truthful close stayed around `6.4s ~ 6.6s`.
- The remaining large split is now `pending-visible -> truthful close`, not only RAW arrival.
- Latest `preview-render-ready` details show `sourceAsset=fast-preview-raster`, so close ownership has moved off the old raw-original path. That is an improvement, but not enough yet.

## Important diagnostic notes

- Latest session already shows:
  - `request-capture`
  - `capture-accepted`
  - `fast-thumbnail-attempted`
  - `file-arrived`
  - `button-pressed`
  - `preview-render-start`
  - `capture_preview_ready`
  - `recent-session-visible`
- `button-pressed` is currently appended from the client and can land later than host-owned events, so it should not be treated as a precise latency anchor yet.
- In this latest session, helper logs showed `camera-thumbnail` attempted first, but the successful fast preview still came from `windows-shell-thumbnail` after RAW arrival.

## Change introduced after this review

- `camera-thumbnail` immediate failure now emits `fast-thumbnail-failed`, so the next hardware run should show why the earlier same-capture thumbnail missed.
- `legacy-canonical-scan` promotion now also records `fast-preview-visible`, so the per-session seam is easier to close from one log.
- fast-preview-raster truthful close cap was reduced again from `256px` to `192px` to push the recent-session preset-applied close lower on the next run.

## 2026-04-04 later run regression update

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a315b7a23e1784`

### What changed in the latest run

- `camera-thumbnail` failure is now confirmed in all latest captures.
  - `detailCode=fast-thumbnail-download-failed`
- first visible seam is clearer.
  - `fast-preview-visible` now lands in the same session log.
- the first capture of the session regressed badly.
  - `capture_preview_ready elapsedMs=10473`
- the next two captures were lower again.
  - `capture_preview_ready elapsedMs=6333`
  - `capture_preview_ready elapsedMs=6848`

### Root-cause reading

- This latest regression is not just "192px render is slower."
- The slowest first capture shows:
  - `preview-render-start` once at fast-preview promotion time
  - another `preview-render-start` about 4 seconds later
  - final close only after that later render
- Product interpretation:
  - the cold first capture is still trying the speculative close path
  - that speculative path misses its join window
  - the booth then pays for a duplicate truthful render
  - this is why the first cut felt worse again

### Follow-up change after this regression review

- Production now skips the speculative close path for the very first capture of a real session.
- The first capture still shows the same pending first-visible image, but it no longer waits for a cold speculative lane before starting the truthful close.
- A new session timing marker is written when this happens:
  - `event=speculative-preview-skipped`
  - `detail=reason=first-capture-cold-start`

### Next expectation

- First capture should stop jumping into the `10s` class just because the speculative worker was cold.
- If the next hardware run still stays above roughly `6s`, the remaining problem is no longer speculative waiting. At that point the next target is reducing the actual fast-preview-raster truthful render cost itself or changing close ownership again.

## 2026-04-04 latest follow-up run

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3160c7a664308`

### Measured captures

1. `request_000000000000064e9d32b627c0`
   - `request-capture -> file-arrived`: about `2.0s`
   - `request-capture -> recent-session-pending-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6380ms`
   - `pending-visible -> recent-session-visible`: about `3.0s`
2. `request_000000000000064e9d332571d8`
   - `request-capture -> file-arrived`: about `3.0s`
   - `request-capture -> recent-session-pending-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `7148ms`
   - `pending-visible -> recent-session-visible`: about `4.0s`
3. `request_000000000000064e9d33c05800`
   - `request-capture -> file-arrived`: about `3.0s`
   - `request-capture -> recent-session-pending-visible`: about `4.0s`
   - `capture_preview_ready elapsedMs`: `6513ms`
   - `pending-visible -> recent-session-visible`: about `3.0s`

### Product reading

- The latest run is better than the earlier `10.473s` regression.
- But it is still not near the product target.
- Latest truthful close is still roughly `6.4s ~ 7.1s`.
- The biggest remaining split is still `pending-visible -> recent-session-visible`.

### Stable pattern confirmed again

- all 3 latest captures still logged `fast-thumbnail-failed`
  - `detailCode=fast-thumbnail-download-failed`
- helper still fell back to `windows-shell-thumbnail`
- so current customer-visible speed is no longer blocked by discovering a same-capture source
- it is blocked by how long the preset-applied close takes after that source is already on screen

### Follow-up change after this run

- truthful recent-session close cap was reduced again from `192px` to `160px`
- speculative detail fallback text now records `widthCap=unknown;heightCap=unknown` instead of the stale hard-coded `256x256`
- reason:
  - the latest logs showed the previous placeholder still looked like `256x256`, which can mislead later analysis
  - the latest measured bottleneck is now the close cost itself, so the next code move should target that cost directly

## 2026-04-04 latest improvement check

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a31661611fddb4`

### Measured captures

1. `request_000000000000064e9d484dd5f8`
   - `capture_preview_ready elapsedMs`: `6628ms`
2. `request_000000000000064e9d48c0e0b8`
   - `capture_preview_ready elapsedMs`: `6668ms`
3. `request_000000000000064e9d494d6b60`
   - `capture_preview_ready elapsedMs`: `6454ms`

### Product reading

- The latest run is tighter than the earlier `6.38s ~ 7.15s` spread, but still not materially faster for customers.
- The system is now clustering around `6.45s ~ 6.67s`.
- `camera-thumbnail` still fails first on every cut, and `windows-shell-thumbnail` still becomes the visible same-capture source.
- So the dominant remaining problem is still the preset-applied close itself.

### Follow-up change after this check

- truthful close cap was reduced again from `160px` to `128px`
- speculative one-shot workers now suppress the late `render-output-missing` failure log when the same capture was already closed truthfully
- reason:
  - the failure log was becoming diagnostic noise after a successful close
  - the next visible improvement now depends on shaving the actual close cost again, not on finding the fast preview source

## 2026-04-04 latest session review

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a316e3d21c162c`

### Measured captures

1. `request_000000000000064e9d69d82758`
   - `request-capture -> file-arrived`: about `2.0s`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6684ms`
2. `request_000000000000064e9d6a52e3b0`
   - `request-capture -> file-arrived`: about `2.0s`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6397ms`
3. `request_000000000000064e9d6aeaa528`
   - `request-capture -> file-arrived`: about `3.0s`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `6512ms`

### Product reading

- The latest run still clusters around `6.4s ~ 6.7s`.
- The booth is now consistently showing the same-capture pending image in about `3s`.
- `camera-thumbnail` still fails first on every cut, then `windows-shell-thumbnail` becomes the usable same-capture source.
- So the remaining customer delay is still the preset-applied close itself, not the discovery of a same-capture image.

### Root-cause reading

- Repeated cap reductions on the final preview output are no longer moving the result enough.
- The stable `3s pending` plus `6.4s ~ 6.7s truthful close` split suggests the booth is still spending too much time processing the source raster that darktable receives after pending is already visible.
- In the current code, the speculative preview source was only copied to a stable path. It was not downscaled before the truthful close render started.

### Follow-up change after this review

- speculative truthful-close input is now staged to a smaller booth-safe raster before darktable runs
- oversized same-capture sources are reduced to roughly a `256px` rail before the preset-applied close
- if the source is already small or resize fails, the booth falls back to the previous copy-only path instead of dropping the preview

### Next expectation

- The next hardware run should tell us whether the remaining `6.4s ~ 6.7s` cluster was mostly input-raster cost.
- If the booth still stays in the same band after this change, the remaining cost is likely process/runtime overhead rather than raster size, and the next move should target close ownership or renderer topology instead of more cap tuning.

## 2026-04-04 quality regression rollback

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a318221f7d3ab0`

### Measured captures

1. `request_000000000000064e9dbb680188`
   - `request-capture -> file-arrived`: about `3.0s`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `7774ms`
2. `request_000000000000064e9dbc192700`
   - `request-capture -> file-arrived`: about `2.0s`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `7438ms`

### Product reading

- This latest run regressed again.
- Customer-facing close moved back to roughly `7.4s ~ 7.8s`.
- The same-capture pending image still appeared around `3s`, so the booth did not get faster where users actually wait.
- The operator also reported that the thumbnail quality became unacceptably poor.

### Root-cause reading

- The previous attempt shrank the speculative truthful-close source itself before darktable ran.
- That added more preprocessing work while also damaging the visible preview quality.
- The booth therefore paid both costs at once:
  - slower truthful close
  - worse thumbnail quality

### Follow-up change after this regression

- the quality-degrading speculative source downscale was rolled back
- speculative close now keeps the previous preview quality level
- instead of re-encoding a second small raster, the booth now reuses the already-promoted canonical session preview in place when it is safe to do so
- request-scoped copy staging still stays in place for non-canonical handoff sources, so the safer path is preserved where it is still needed

### Next expectation

- The next hardware run should restore the earlier preview quality immediately.
- If this in-place reuse removes enough duplicate file I/O, truthful close should come down without further quality loss.
- If the booth still remains in the `7s` class after quality is restored, the next target should move away from raster/source manipulation and toward the remaining render ownership/runtime overhead itself.

## 2026-04-04 latest slowdown root-cause review

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a318e4bc827b14`
- extra evidence reviewed:
  - `C:\Users\KimYS\Pictures\dabi_shoot\.boothy-darktable\preview\logs\preview-stderr-1775290067047366800.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\.boothy-darktable\preview\logs\preview-stderr-1775290079734560900.log`
  - `C:\Users\KimYS\Pictures\dabi_shoot\.boothy-darktable\preview\logs\preview-stderr-1775290090887679400.log`

### Measured captures

1. `request_000000000000064e9deda1b420`
   - `request-capture -> fast-preview-visible`: about `4.0s`
   - first `preview-render-start`: about `4.0s`
   - `capture_preview_ready elapsedMs`: `10282ms`
2. `request_000000000000064e9dee66b000`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - first `preview-render-start`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `9727ms`
3. `request_000000000000064e9def0942b8`
   - `request-capture -> fast-preview-visible`: about `3.0s`
   - first `preview-render-start`: about `3.0s`
   - `capture_preview_ready elapsedMs`: `10372ms`

### Product reading

- This run regressed back into the `9.7s ~ 10.4s` class.
- The same-capture pending image still appeared much earlier than the truthful close.
- So the booth was not waiting on discovery of a thumbnail. It was losing time after the close path already started.

### Root-cause reading

- Each capture showed the same pattern:
  - first `preview-render-start`
  - `preview-render-failed reason=render-process-failed`
  - second `preview-render-start`
  - final `preview-render-ready`
- The darktable stderr logs explain the first failure directly:
  - `error: can't open file //?/C:/.../renders/previews/<capture>.jpg`
  - `no images to export, aborting`
- Product interpretation:
  - the booth passed a Windows extended-length path (`\\?\...`) into the first preview render attempt
  - darktable rejected that source path
  - the booth then paid for a full second preview render on a retry path
  - this is what pushed truthful close back into the `10s` class

### Follow-up change after this review

- darktable CLI arguments now strip Windows extended-length path prefixes before launch
- this targets the first preview render failure directly instead of shrinking image quality again
- capture requests now also trigger a fresh preview renderer warm-up so the booth can spend the camera round-trip time rewarming the preview lane before RAW arrival

### Next expectation

- The next hardware run should stop showing `preview-render-failed reason=render-process-failed` on the first preview attempt for these captures.
- If that first failure disappears, the booth should immediately stop paying the duplicate second render and move back down from the current `10s` class.

## 2026-04-05 Test Look canary follow-up

### Latest source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a351696c25d93c`
- files reviewed:
  - `session.json`
  - `diagnostics/timing-events.log`
  - `diagnostics/camera-helper-events.jsonl`
- policy switched to:
  - `presetId=preset_test-look`
  - `presetVersion=2026.03.31`

### Measured captures

Completed captures in the latest `Test Look` session:

1. `capture_20260405012332551_b25968bcc7`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2847ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6203ms`
2. `capture_20260405012342393_55d853d006`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3041ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6486ms`
3. `capture_20260405012350889_d151538d37`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2988ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6426ms`

Average of the completed cuts:

- same-capture first-visible: about `2959ms`
- preset-applied truthful close: about `6372ms`

### Product reading

- The user’s latest `3초대` impression is still accurate for first-visible.
- But the customer-facing truthful close is still staying in roughly the `6.2s ~ 6.5s` class on `Test Look`.
- This means the main remaining wait is still after the pending same-capture image is already on screen.

### Root-cause reading

- `camera-thumbnail` still failed first on every completed cut.
  - `detailCode=fast-thumbnail-download-failed`
- helper still promoted `windows-shell-thumbnail` after RAW arrival.
- the truthful close owner was already the `fast-preview-raster` route, not the old raw-original path.
- so the latest problem is no longer `which route did we choose?`
- the latest problem is `how much fixed cost does the chosen local renderer route still pay every time it runs?`

Local reproduction and bench notes from the same `Test Look` source:

- `darktable-cli --version` alone cost about `640ms ~ 787ms` on the booth machine.
- applying the published `Test Look` XMP to the same fast-preview JPG still cost about `3.7s ~ 3.8s` in one-off reproduction.
- a quick experiment disabling obvious RAW-only modules in a temporary XMP only saved about `110ms`.
- product interpretation:
  - the cheap next win is not more XMP surgery
  - the cheap next win is removing repeated sidecar fixed cost and preserving cache across sessions

### Latest tech docs re-checked

- darktable official program invocation docs say:
  - `--version` is a separate process path
  - `--cachedir` stores thumbnail cache and precompiled OpenCL binaries for faster startup
  - source: `https://docs.darktable.org/usermanual/3.8/en/special-topics/program-invocation/darktable/`
- Tauri official sidecar docs continue to frame sidecars as normal external binaries launched by the app shell.
  - source: `https://v2.tauri.app/develop/sidecar/`

### Change introduced after this review

- local renderer sidecar now uses a runtime-scoped worker root instead of a session-scoped one
  - `.boothy-local-renderer/preview`
- local renderer now writes and reuses `darktable-version-cache.json`
  - once the darktable binary path, size, and last-write time match, the sidecar skips the repeated `--version` probe
- local renderer now also supplies `--cachedir` under the same runtime-scoped worker root

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml local_renderer -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_reuses_a_runtime_scoped_darktable_version_cache -- --test-threads=1`
  - passed

### Next expectation

- the next real booth run should keep the same `3s` first-visible class
- and it should spend less time in the local renderer fixed-cost part of the close path
- if the next hardware run still stays near the same `6.3s` band even after this cache reuse lands, the next target is no longer sidecar startup overhead
- at that point the remaining cost is the render body itself, and the next step should move toward a lighter truthful renderer or a different close topology

## 2026-04-05 second follow-up: first Test Look capture stayed unfiltered, later closes were still slow, and thumbnail quality regressed

### Latest problematic source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a359b2db1b8584`
- files reviewed:
  - `session.json`
  - `diagnostics/timing-events.log`
  - `renders/previews/*`

### What the latest booth evidence showed

First capture:

1. `capture_20260405035512593_24c7ba7114`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `2971ms`
   - `previewVisibleAtMs`: `null`
   - `xmpPreviewReadyAtMs`: `null`
   - session manifest stayed at `renderStatus=previewWaiting`

Later completed captures:

1. `capture_20260405035530621_e43ffb3b9c`
   - `capture acknowledged -> previewVisibleAtMs`: `7115ms`
2. `capture_20260405035541031_71f9d50724`
   - `capture acknowledged -> previewVisibleAtMs`: `8880ms`
3. `capture_20260405035550373_b694687f8b`
   - `capture acknowledged -> previewVisibleAtMs`: `6533ms`

### Detailed root-cause reading

For the first capture, `timing-events.log` showed:

- one `preview-render-start` right after fast preview visibility
- another `preview-render-start` about `5s` later
- immediate `preview-render-queue-saturated`
- no later `preview-render-ready`

But the session preview folder still contained:

- `capture_20260405035512593_24c7ba7114.preview-speculative.jpg`
- `capture_20260405035512593_24c7ba7114.request_000000000000064eae844ef2e0.preview-speculative.detail`

That speculative detail recorded:

- `elapsedMs=9637`
- `widthCap=128;heightCap=128`
- `sourceAsset=fast-preview-raster`

Product reading:

- the first capture did not truly fail to render
- it finished too late for the original follow-up window
- and the app never promoted the finished speculative result back into the canonical latest-capture state

At the same time, truthful preview JPG sizes for later captures were only around:

- `33290` bytes
- `17742` bytes
- `33553` bytes

This matched the user report that the recent-session thumbnail looked too soft.
With the current `128x128` fast-preview truthful cap, that reading was credible.

### Change introduced after this review

- readiness now opportunistically promotes a finished speculative preview close for the latest waiting capture
  - this means the first capture no longer needs another capture or manual action to become truthful-close ready
- capture command follow-up refinement wait expanded from `2s` to `12s`
  - this is deliberately longer than the observed `9637ms` first speculative close
- fast-preview truthful close cap increased from `128x128` to `256x256`
  - still below the raw-original `384x384` path
  - but materially sharper than the regressed `128px` booth rail output

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml readiness_promotes_a_finished_speculative_preview_without_needing_another_capture -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_restores_a_sharper_than_legacy_128_cap -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml local_renderer -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness speculative_preview -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test session_manifest -- --test-threads=1`
  - passed

### Next expectation

- the next real booth run should stop leaving the first `Test Look` capture in `previewWaiting`
- the recent-session truthful thumbnail should look visibly sharper than the `128px` regression
- if completed truthful close still stays in the same `6s+` band after this, the remaining cost should be treated as render-body cost rather than missed promotion or startup fixed cost

## 2026-04-05 third follow-up: second Test Look capture stalled in `촬영 처리 중`, and the stall came from helper transfer never starting rather than from the preview renderer

### Latest problematic source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3741cf8095e30`
- preset:
  `Test Look / 2026.03.31`
- files reviewed:
  - `session.json`
  - `diagnostics/timing-events.log`
  - `diagnostics/camera-helper-events.jsonl`
  - `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`

### What the latest booth evidence showed

Completed captures in the same session:

1. `capture_20260405115912929_7300bdc8d0`
   - `capture acknowledged -> previewVisibleAtMs`: `6418ms`
2. `capture_20260405120023003_f12fb9965e`
   - `capture acknowledged -> previewVisibleAtMs`: `6308ms`

Failed middle request:

1. `request_000000000000064eb547b121e8`
   - `capture-accepted` logged at `11:59:19Z`
   - no `fast-thumbnail-attempted`
   - no `file-arrived`
   - helper recovery/error logged at about `11:59:50Z`
   - effective stall: about `31s`

The same session later showed the same pattern again on:

1. `request_000000000000064eb54be1d300`
   - accepted at `12:00:30Z`
   - timeout recovery at `12:01:01Z`

### Detailed reading

Product reading:

- this was not a "filter applied too slowly" close-path regression
- the booth got stuck waiting before RAW transfer had even really started
- because the helper only knew the request had been accepted, then kept waiting until the full download timeout expired

At the same time, host logs showed preview warmup was still not helping:

- `preview_renderer_warmup_started`
- then `preview_renderer_warmup_failed`
- stderr detail:
  `libpng warning: IDAT: Extra compressed data / libpng error: Not enough image data`

That meant the runtime warmup PNG fixture itself was broken, so warmup never actually completed.

### Change introduced after this review

- helper now distinguishes
  - "capture was accepted"
  - from "RAW transfer actually started"
- if transfer start never arrives, helper now cuts over to
  `capture-transfer-start-timeout`
  after `8s` instead of consuming the full `30s` completion budget
- host recovery now treats `capture-transfer-start-timeout` the same way as the old helper timeout for readiness recovery
- preview renderer warmup input PNG was replaced with a known-good fixture so warmup can finally execute instead of failing immediately

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_releases_phone_required_after_capture_transfer_start_timeout_recovers -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source_matches_the_known_good_png_fixture -- --test-threads=1`
  - passed
- `dotnet build sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`
  - passed
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj --no-restore`
  - passed

### Next expectation

- the same helper-side stall should no longer keep the booth in `촬영 처리 중` for `30s+`
- if the camera/helper boundary fails again, the booth should move into recovery noticeably sooner
- if real hardware still shows `6s+` closes after this, that remaining cost should still be treated as preview-render body cost, not this helper stall

## 2026-04-05 fourth follow-up: latest Test Look run proved the canary speedup was still bypassing the winning speculative close lane

### Latest measured source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3753ffaaa16d8`
- preset:
  `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures in the same session:

1. `capture_20260405122004678_9752b1807b`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `4798ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8555ms`
2. `capture_20260405122013771_7dc7ff617a`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `3933ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7453ms`
3. `capture_20260405122020979_cf1712edb4`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `2849ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6453ms`
4. `capture_20260405122029138_543b9ec46a`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `3182ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6865ms`

### Key reading

- helper stall은 이미 사라졌다
  - 모든 컷이 `capture-accepted -> file-arrived`로 정상 닫혔다
- 그런데 booth close는 여전히 `6.4s ~ 8.6s` band에 남아 있었다
- 더 중요하게는 같은 session package에
  - `renderer-route-selected`
  - `renderer-route-fallback`
  - `renderer-close-owner`
  흔적이 전혀 없었다
- runtime worker root `C:\Users\KimYS\Pictures\dabi_shoot\.boothy-local-renderer\preview`도
  최신 실측 시점에는 실제 산출 흔적이 없었다

Product reading:

- `Test Look` canary policy는 켜져 있었지만,
  실제로 customer-facing close owner가 된 경로는 여전히 resident/speculative lane이었다
- 그리고 그 lane은 session-locked preview route policy를 보지 않고 darktable direct path로 닫고 있었다
- 그래서 직전 회차에 넣은
  - runtime-scoped sidecar cache reuse
  - darktable version probe reuse
  최적화가 최신 winning close path에는 아예 적용되지 못하고 있었다

### Change introduced after this review

- resident/speculative preview close도 이제 same session-locked preview route policy를 본다
- canary preset이면 speculative lane도 local renderer sidecar candidate를 먼저 사용한다
- sidecar가 실패하면 그 speculative lane 안에서 즉시 approved darktable baseline으로 fallback 한다
- speculative output이 나중에 truth owner로 승격될 때도
  - `renderer-route-selected`
  - `renderer-route-fallback`
  - `renderer-close-owner`
  evidence가 session package에 같이 남도록 보강했다

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness speculative -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness local_renderer -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test session_manifest -- --test-threads=1`
  - passed

### Next expectation

- 다음 `Test Look` 실측부터는 latest winning speculative close에도 local renderer canary 이득이 실제로 들어가야 한다
- 같은 session package 하나만 열어도 route selection / fallback / close owner를 바로 확인할 수 있어야 한다
- 만약 그 뒤에도 completed close가 여전히 같은 `6s+` band에 머무르면,
  이제 남은 문제는 truly render body 자체이고 다음 수는 `lighter truthful renderer` 쪽이어야 한다

## 2026-04-05 fifth follow-up: latest booth session still looks like a pre-fix baseline, so record it as data but not as proof that the new route wiring landed on hardware

### Latest measured source

- session path:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3768babafc5e4`
- preset:
  `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures:

1. `capture_20260405124346795_6f4a2114f6`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `2966ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7772ms`
2. `capture_20260405124357452_ddb8d42b39`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `4521ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8520ms`
3. `capture_20260405124406153_478b1ddf63`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `3381ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7298ms`
4. `capture_20260405124415751_e5276d0047`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: about `3359ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7526ms`

Helper evidence:

- all four requests still closed with `capture-accepted -> file-arrived`
- no helper timeout on this run either

But this run also showed two important negatives:

1. the session package still did **not** contain
   - `renderer-route-selected`
   - `renderer-route-fallback`
   - `renderer-close-owner`
2. host log still showed
   - `preview_renderer_warmup_failed`
   - `libpng error: IDAT: CRC error`

### Product reading

- this latest booth run should be treated as a **data baseline**, not as proof that the newest route-wiring change already reached the hardware runtime
- the missing route evidence means the speculative close that won on this session still behaved like the older path
- the warmup CRC error also means this runtime still carried the old broken warmup fixture behavior

### Immediate interpretation

- the newly added speculative-canary wiring is code-complete and test-covered
- but this `21:44` booth run does not yet validate it in the field
- the next approved hardware check must specifically confirm:
  - `renderer-route-selected`
  - `renderer-close-owner`
  - and the disappearance of the warmup CRC error

## 2026-04-06 sixth follow-up: latest hardware sessions prove the 1.11 route wiring is finally live, but every canary render still falls back because the booth runtime is on darktable `5.4.0` while the request pin is `5.4.1`

### Latest measured sources

- session paths:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3aa34bbfaeeb0`
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3990e532fba00`
- preset:
  - `Test Look / 2026.03.31`

### What the latest booth evidence showed

From `session_000000000018a3aa34bbfaeeb0`:

1. `capture_20260406043031157_467c0e39cd`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3749ms`
   - `capture acknowledged -> previewVisibleAtMs`: `14366ms`
   - timing log included:
     - `speculative-preview-skipped`
     - `renderer-route-selected`
     - `renderer-route-fallback`
     - `renderer-close-owner`
   - fallback detail:
     - `darktable version mismatch: requested=5.4.1 actual=5.4.0`
2. `capture_20260406043046499_e8ac721b93`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2991ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7176ms`
   - local-renderer diagnostics response still returned:
     - `darktable version mismatch: requested=5.4.1 actual=5.4.0`
3. `capture_20260406043546550_db0682c879`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2990ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7295ms`
   - local-renderer diagnostics response still returned:
     - `darktable version mismatch: requested=5.4.1 actual=5.4.0`
4. `capture_20260406043556525_f47742492d`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3087ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7183ms`
   - local-renderer diagnostics response still returned:
     - `darktable version mismatch: requested=5.4.1 actual=5.4.0`

From `session_000000000018a3990e532fba00`:

- all three captures also wrote local-renderer response envelopes with the same mismatch:
  - `darktable version mismatch: requested=5.4.1 actual=5.4.0`
- that session also recorded an early `preview-render-queue-saturated`, so the booth paid both:
  - the old direct darktable close body cost
  - plus repeated failed canary attempts that could never win

### Product reading

- this is no longer a "new route wiring didn't land on hardware" problem
- the latest session package now contains route evidence, which means the 1.11 canary path is actually executing in the field
- the current blocker is narrower and more concrete:
  - the published booth request pin says `5.4.1`
  - the installed darktable runtime cached by the sidecar is `5.4.0`
  - so every canary attempt is rejected before it can contribute any speedup
- product-wise, the booth is still behaving like a darktable fallback baseline:
  - first capture stayed very slow at `14.4s`
  - follow-up captures stayed in the `7.1s ~ 7.3s` truthful-close band

### Change introduced after this review

- local renderer sidecar now accepts patch skew within the same darktable `major.minor`
- in practice this means the current booth combination
  - request pin `5.4.1`
  - installed runtime `5.4.0`
  is now treated as compatible instead of being rejected
- cross-minor or cross-major mismatches still fail closed and fall back

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_accepts_patch_skew_within_the_same_darktable_minor -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_rejects_an_unpinned_darktable_binary -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_reuses_a_runtime_scoped_darktable_version_cache -- --test-threads=1`
  - passed
- `cargo fmt --manifest-path src-tauri/Cargo.toml`
  - passed

### Next expectation

- the next approved hardware run should keep `renderer-route-selected` / `renderer-close-owner` evidence
- the repeated `requested=5.4.1 actual=5.4.0` mismatch should disappear
- if the booth still closes in the same `7s+` band after that, the remaining work is no longer route enablement; it is the actual truthful render body cost

## 2026-04-06 seventh follow-up: version mismatch is gone on hardware, but the next live blocker is the sidecar's first OpenCL bring-up cost

### Latest measured source

- session path:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3ac38ba08752c`
- preset:
  - `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures:

1. `capture_20260406050726153_eb8e72fafb`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3926ms`
   - `capture acknowledged -> previewVisibleAtMs`: `17587ms`
   - route evidence now showed the canary path executing immediately:
     - `renderer-route-selected`
     - then `renderer-route-fallback`
   - but the fallback reason changed from version mismatch to:
     - `local renderer sidecar가 제한 시간 안에 끝나지 않았어요`
   - same runtime also created a fresh `.boothy-local-renderer\preview\cache\cached_v5_kernels...` tree during that first attempt
2. `capture_20260406050744943_336b29448c`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3343ms`
   - `capture acknowledged -> previewVisibleAtMs`: `11464ms`
   - local-renderer response now failed with:
     - `candidate output missing after darktable bridge`
3. `capture_20260406050757460_92d45c5ca9`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2925ms`
   - `capture acknowledged -> previewVisibleAtMs`: `10651ms`
   - local-renderer response again failed with:
     - `candidate output missing after darktable bridge`

### Product reading

- the earlier version compatibility fix did work:
  - the repeated `requested=5.4.1 actual=5.4.0` rejection is gone from the latest hardware session
- but the booth is still not getting the canary speedup because the local renderer is now losing for a different reason
- the first live attempt paid a large one-time sidecar environment cost:
  - OpenCL kernel cache creation under `.boothy-local-renderer\preview\cache`
  - followed by a `10s` sidecar timeout
- product-wise that means the booth still falls back to baseline darktable and keeps a very slow truthful close band:
  - first capture: `17.6s`
  - follow-up captures: `10.6s ~ 11.5s`

### Change introduced after this review

- local renderer sidecar now forces `--disable-opencl` for booth preview bridge runs
- the goal is to stop the first canary attempt from spending its budget on GPU kernel initialization instead of producing a customer-facing preview

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_disables_opencl_for_preview_bridge -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_accepts_patch_skew_within_the_same_darktable_minor -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_rejects_an_unpinned_darktable_binary -- --test-threads=1`
  - passed

### Next expectation

- the next approved hardware run should no longer show:
  - `local-renderer-timeout` on the first canary attempt
  - or a brand-new OpenCL kernel cache build dominating the first close
- if `candidate output missing after darktable bridge` still survives after this,
  then the remaining blocker is inside the sidecar bridge/output publication itself, not version routing or GPU warm-up

## 2026-04-06 eighth follow-up: the booth feels roughly 2x slower because each capture is now paying a failed sidecar render body and then a second darktable fallback body in sequence

### Latest measured source

- session path:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3aceb669047b8`
- preset:
  - `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures:

1. `capture_20260406052016176_35b175e6fe`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3349ms`
   - `capture acknowledged -> previewVisibleAtMs`: `11630ms`
   - sidecar selected at `05:20:18Z`
   - sidecar fallback logged at `05:20:22Z`
   - darktable close owner logged at `05:20:26Z`
2. `capture_20260406052029539_e7f8287e44`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3839ms`
   - `capture acknowledged -> previewVisibleAtMs`: `11482ms`
   - same pattern repeated:
     - local renderer attempt first
     - `candidate output missing after darktable bridge`
     - then baseline darktable close
3. `capture_20260406052042301_d5c9c7b966`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3264ms`
   - `capture acknowledged -> previewVisibleAtMs`: `11647ms`
   - same late sidecar failure repeated again

One extra clue from the same session:

- the preview folder still contained
  - `capture_20260406052042301_d5c9c7b966.preview-rendering.jpg`
- which means the candidate path was not truly "impossible"; the booth simply failed to trust the sidecar in time and then still retried baseline close

### Why it felt about 2x slower

- the older good baseline on `Test Look` was roughly:
  - `fastPreviewVisibleAtMs` around `3s`
  - `previewVisibleAtMs` around `6.3s ~ 7.3s`
- the newest run is roughly:
  - `fastPreviewVisibleAtMs` still around `3s`
  - `previewVisibleAtMs` around `11.5s ~ 11.6s`
- product-wise the extra time is not coming from camera transfer or first-visible discovery anymore
- it is coming from a duplicated truthful-close body:
  - first the booth spends about `4s` trying the sidecar route
  - then, after that fails, it spends about another `4s` on baseline darktable fallback
- that serial double-pay is why the booth now feels close to `2x` slower than the earlier `6s` class

### Change introduced after this review

- once a session records a local renderer failure, the session-locked preview route policy now adds a forced fallback for the rest of that session
- product effect:
  - the first failed canary capture may still pay the recovery cost
  - but later captures in the same session should stop retrying the same unhealthy sidecar and should fall back directly to the older darktable baseline instead of paying the double render every time

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness local_renderer_failure_forces_darktable_for_the_rest_of_the_session -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness local_renderer_error_envelope_is_recorded_before_fallback -- --test-threads=1`
  - passed

### Next expectation

- the next approved hardware run should show:
  - first failed sidecar attempt, if any
  - then later captures in the same session selecting `policyReason=forced-fallback` immediately
- if that restores the booth from `11s` back toward the older `6s ~ 7s` band, then the immediate product regression is contained
- after that, the remaining engineering work is to fix the sidecar bridge so it can actually win again instead of only being quarantined

## 2026-04-06 ninth follow-up: session quarantine did contain the `11s` regression, and the next preset-applied speed win should come from reducing direct darktable preview fixed cost

### Latest measured source

- session path:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3ad70b60b9790`
- preset:
  - `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures:

1. `capture_20260406052943288_c7bedebde2`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3163ms`
   - `capture acknowledged -> previewVisibleAtMs`: `11442ms`
   - first capture still paid:
     - sidecar attempt
     - `candidate output missing after darktable bridge`
     - then darktable fallback close
2. `capture_20260406052956865_769f0e5223`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3340ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7118ms`
3. `capture_20260406053013263_e51110182f`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3072ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6854ms`

The same session's locked route policy now contained:

- `forcedFallbackRoutes`
  - `sessionId = session_000000000018a3ad70b60b9790`
  - `reason = session-sidecar-health-check-failed`

### Product reading

- the session-sidecar quarantine worked
- the booth no longer stayed in the broken `11s` class for every later shot in the same session
- after the first failed canary attempt, the booth returned to roughly the older baseline band:
  - `7.1s`
  - `6.85s`
- that means the urgent product regression is partially contained
- but the preset-applied large preview is still slower than desired because the darktable baseline close itself remains too expensive

### New root-cause reading

- the latest later captures no longer show another expensive sidecar retry body
- so the remaining wait is now dominated by the direct darktable preview lane again
- one implementation gap was still visible in the invocation path:
  - sidecar preview bridge already had a runtime-scoped `--cachedir`
  - direct darktable preview invocation did not
- product interpretation:
  - the booth was preserving cache reuse on the canary bridge path
  - but not on the approved baseline path that is currently winning after quarantine
  - so the current truthful close owner was still leaving an easy startup/cache win on the table

### Change introduced after this review

- direct darktable preview invocation now also uses a runtime-scoped `--cachedir`
- this aligns the approved baseline close path with the same cache reuse approach already used by the local renderer bridge
- the intended product outcome is to pull the remaining `6.8s ~ 7.1s` preset-applied close band down without sacrificing preview sharpness

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_uses_a_runtime_scoped_cachedir -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml real_local_renderer_sidecar_disables_opencl_for_preview_bridge -- --test-threads=1`
  - passed

### Next expectation

- the next approved hardware run should keep the first-visible `3s` class
- later captures in a quarantined session should stay out of the `11s` regression
- and the remaining preset-applied close should come down from roughly `6.8s ~ 7.1s` if the cached baseline path removes enough startup cost

## 2026-04-06 tenth follow-up: the booth is meaningfully faster now, but the product target must be reframed as `original visible -> preset-applied visible <= 2.5s`

### Latest measured source

- session path:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3ae424e9b17e8`
- preset:
  - `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures:

1. `capture_20260406054445582_62d75f1749`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `5524ms`
   - `capture acknowledged -> previewVisibleAtMs`: `16989ms`
   - `original visible -> preset-applied visible`: `11465ms`
   - first capture still paid:
     - sidecar selection
     - `candidate output missing after darktable bridge`
     - then darktable fallback close
2. `capture_20260406054625081_9f1d9cb71d`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3065ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6509ms`
   - `original visible -> preset-applied visible`: `3444ms`
3. `capture_20260406054633092_f7b16f00bf`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3126ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6524ms`
   - `original visible -> preset-applied visible`: `3398ms`

### Product reading

- the booth did get materially faster versus the earlier `11s` regression
- later captures are now back in the `6.5s` class instead of the broken `11s` class
- but the remaining customer wait is still too long once the original image is already on screen
- the right product metric is now explicit:
  - `presetAppliedDeltaMs = previewVisibleAtMs - fastPreviewVisibleAtMs`
- on the latest good later captures that delta is still:
  - `3444ms`
  - `3398ms`
- so the booth is improved, but it is still about `0.9s` away from the new target band

### Updated target

- the product target is now:
  - `original visible -> preset-applied visible <= 2500ms`
- this is stricter and more product-truthful than only tracking:
  - `capture acknowledged -> previewVisibleAtMs`
- because the guest already sees the original by then and is specifically waiting for the preset-applied replacement

### New root-cause reading

- the winning later lane is currently the direct darktable preview close, not the sidecar
- the latest code path still allowed the booth-safe direct preview lane to run without `--disable-opencl`
- for a small `256px` truthful close, that means the approved baseline lane could still pay unnecessary GPU/OpenCL startup overhead even when the sidecar is already quarantined
- product interpretation:
  - the booth had already removed one large fixed cost by reusing `--cachedir`
  - but it was still leaving another startup cost in the direct close lane that is actually serving customers right now

### Change introduced after this review

- the booth-safe direct darktable preview lane now also forces `--disable-opencl`
- this aligns the approved baseline preview close with the sidecar bridge decision that was already made for the same booth workload
- the intended outcome is to pull the remaining later-capture `presetAppliedDeltaMs` from about `3.4s` toward the new `2.5s` target without weakening truthfulness

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_uses_display_sized_render_arguments -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness direct_darktable_preview_disables_opencl_for_the_booth_safe_lane -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_disables_opencl_for_preview_bridge -- --test-threads=1`
  - passed

### Next expectation

- the next approved hardware run should still keep the `3s` first-visible class
- later captures should keep the restored `6.5s` class or better
- and the more important metric should move next:
  - `previewVisibleAtMs - fastPreviewVisibleAtMs`
  - from about `3.4s`
  - toward `<= 2.5s`

## 2026-04-06 eleventh follow-up: latest hardware run stayed much faster than the old regression, but the remaining `3.4s ~ 3.8s` preset-applied delta exposed a broken preview warmup fixture

### Latest measured source

- session path:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3aec7f04568e8`
- preset:
  - `Test Look / 2026.03.31`

### What the latest booth evidence showed

Completed captures:

1. `capture_20260406055418489_84eba569c0`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3126ms`
   - `capture acknowledged -> previewVisibleAtMs`: `11663ms`
   - `original visible -> preset-applied visible`: `8537ms`
   - first capture still paid:
     - sidecar selection
     - `candidate output missing after darktable bridge`
     - then darktable fallback close
2. `capture_20260406055432199_4210c99eeb`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3729ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7159ms`
   - `original visible -> preset-applied visible`: `3430ms`
3. `capture_20260406055441142_9a5bdb9e7a`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `4151ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7936ms`
   - `original visible -> preset-applied visible`: `3785ms`

The same session also proved:

- the first capture still forced the sidecar quarantine for the rest of that session
- the direct darktable preview lane was now really running with:
  - `--disable-opencl`
  - `--cachedir`

### Product reading

- the booth is still much better than the old `11s` regression
- later captures stayed in roughly the `7s` class instead of collapsing back to `11s`
- but the customer-facing gap from original image shown to preset-applied image shown is still too long:
  - `3430ms`
  - `3785ms`
- that means the latest opencl/cache fixes were helpful but not enough to hit the product target:
  - `original visible -> preset-applied visible <= 2500ms`

### New root-cause reading

- direct darktable preview stderr had been repeatedly collapsing to:
  - `libpng error: IDAT: CRC error`
- the booth runtime warmup fixture at:
  - `C:\Users\KimYS\Pictures\dabi_shoot\.boothy-darktable\preview\warmup\preview-renderer-warmup-source.png`
  was then checked directly
- its `IDAT` chunk CRC was invalid
- product interpretation:
  - the booth believed it had a preview warmup path
  - but the warmup source itself was corrupted
  - so the warmup lane could not reliably pre-prime the approved direct preview runtime that customers are currently waiting on

### Change introduced after this review

- the preview warmup fixture was replaced with a structurally valid `1x1` PNG
- a regression test now verifies that the warmup fixture's PNG chunk CRCs are valid, not just byte-stable
- intended product outcome:
  - warmup should actually warm the direct preview runtime now
  - later captures should spend less time between original-visible and preset-applied-visible

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source_is_written_as_png -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source_fixture_has_valid_png_chunk_crcs -- --test-threads=1`
  - passed
- `cargo test --manifest-path src-tauri/Cargo.toml direct_darktable_preview_disables_opencl_for_the_booth_safe_lane -- --test-threads=1`
  - passed

### Next expectation

- the next approved hardware run should keep the improved `7s`-class later captures
- first capture may still be penalized until the sidecar bridge bug is fixed
- but later captures should now have a realistic chance to move:
  - from `3.4s ~ 3.8s` original-to-preset delta
  - toward the `<= 2.5s` target
