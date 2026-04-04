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
