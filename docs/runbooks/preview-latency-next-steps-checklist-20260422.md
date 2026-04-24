---
documentType: execution-checklist
status: active
date: 2026-04-22
scope: preview-track
---

# Preview Latency Next Steps Checklist

## 왜 이 문서가 필요한가

이 문서는 preview-track의 다음 시도를
에이전트가 단계별로 그대로 따라갈 수 있게 정리한 current execution checklist다.

핵심 목적은 두 가지다.

1. 무엇이 이미 닫혔는지와 무엇이 아직 blocker인지 한 번에 보이게 한다.
2. 각 단계가 끝날 때 어떤 검증 결과를 남겨야 하는지 고정한다.

## 현재 기준 판단

- official gate는 계속 `preset-applied visible <= 3000ms` 하나다.
- comparable latency baseline은 계속 `session_000000000018a8673fd974df10`이다.
- latest software attempt session `session_000000000018a868febfab83c0`는 first-shot truth regression 때문에 reject한다.
- latest rejected field rerun `session_000000000018a88c71e9375868`는 first-shot duplicate render overlap 때문에 reject한다.
- latest approved rerun `session_000000000018a88d53fa8f00c4`는 first-shot overlap family가 사라졌지만 official gate는 아직 `No-Go`였다.
- 이전 immediate blocker는 startup/save나 false-ready가 아니라, accepted current-code evidence 기준 전 컷 공통으로 남은 steady-state truthful-close gap이었다.
  `session_000000000018a92a6c02e7f2d4`에서는 이 gap이 official gate 안으로 들어왔지만, 이후 code review patch가 적용된 current-code rerun에서는 다시 gate 밖으로 나갔다.
- latest current-code hardware-validation-runner evidence `session_000000000018a934f66a92fe80`는
  non-truthful preview가 product-ready로 닫히는 false pass class를 막은 뒤 수집한 package다.
  preview ownership은 `preset-applied-preview`로 truthful했지만, capture 1 direct metric이
  `originalVisibleToPresetAppliedVisibleMs=3037ms`로 official `<= 3000ms` gate를 `37ms` 넘었다.
  runner는 이 회차를 `preview-truth-gate-failed`, `capturesPassed=0/5`로 중단했으므로 Story `1.26`은 current-code 기준 `review / No-Go`다.
- 다만 latest approved hardware package `session_000000000018a88d53fa8f00c4`는 아직 pre-direct-metric evidence라
  `timing-events.log`에 official gate 구간이 direct metric으로는 없었다.
- current worktree는 이제 `capture_preview_ready` detail에
  `originalVisibleToPresetAppliedVisibleMs`, `firstVisibleAtMs`, `presetAppliedVisibleAtMs`
  를 같이 남기고, 관련 자동 검증도 통과했다.
- latest accepted current-code evidence `session_000000000018a8fe95ea36f8f4`는
  startup/connect가 정상이고 5컷 모두 truthful close로 닫혔으며,
  `preview-render-ready elapsedMs`가 `3317`, `3315`, `3314`, `3215`, `3316`,
  `originalVisibleToPresetAppliedVisibleMs`가 `3358`, `3368`, `3357`, `3284`, `3361`
  으로 first shot 포함 전 컷이 조금 더 낮은 steady-state band에 모였다.
- latest hardware-validation-runner evidence `session_000000000018a91e89791d5370`는
  `--disable-opencl`을 `--core` 뒤로 옮겨 darktable core option으로 실제 적용한 뒤 수집한 package다.
  5컷 모두 `previewReady`로 닫혔고 direct metric은
  `2953`, `2960`, `3039`, `3197`, `2953`으로 3/5컷이 official gate 안에 들어왔다.
  하지만 full package 기준은 2컷 tail miss 때문에 아직 `No-Go`다.
- latest hardware-validation-runner evidence `session_000000000018a926e98958c25c`는
  fast-preview JPEG truthful-close XMP에서 `highlights`, `cacorrectrgb`를 추가 제거한 뒤 수집한 package다. `lens`와 `hazeremoval` 제거는 code review 후 truthful preset look 기준에 맞지 않아 되돌렸다.
  5컷 모두 `previewReady`로 닫혔고 direct metric은
  `3039`, `2955`, `3034`, `3032`, `2956`으로 평균 `3003.2ms`까지 붙었다.
  하지만 full package 기준은 3컷이 official gate를 `32ms ~ 39ms` 넘어서 아직 `No-Go`다.
- previous positive hardware-validation-runner evidence `session_000000000018a92a6c02e7f2d4`는
  fast-preview cached XMP의 `iop_order_list`를 실제 유지된 preview history operation/priority만 남기도록 줄인 뒤 수집한 package다.
  5컷 모두 `previewReady`로 닫혔고 direct metric은
  `2956`, `2951`, `2961`, `2954`, `2960`으로 전부 official `<= 3000ms` 안에 들어왔다.
  다만 이 package는 code review patch 이전의 historical positive evidence로만 읽는다.
- latest accepted invocation args에는
  `.boothy-darktable/preview/xmp-cache/preset-new-draft-2-2026-04-10-look2-fast-preview.xmp`
  가 실제로 남아, preview fast-preview-raster lane이 raw-only darktable history 일부를 덜어낸 cached XMP를 실제로 사용한 것이 확인됐다.
- latest rejected experiment evidence `session_000000000018a8fdb7a8e88590`는
  same-capture truthful close cap을 `192x192`로 더 줄였지만,
  `originalVisibleToPresetAppliedVisibleMs`가 `3523`, `3434`, `3599`, `3602`, `3437`
  으로 오히려 개선을 만들지 못했다.
- latest rejected invocation args에는 `--disable-opencl`, `--library :memory:`, `--width 192`, `--height 192`가 같이 남아,
  단순 raster cap 축소는 current blocker를 닫는 방향이 아니라는 점도 확인됐다.
- 따라서 기존 false-ready 위험은 code review patch와 runner truth gate로 막혔지만,
  current-code product blocker는 capture 1의 `3037ms` truthful-close tail miss로 다시 좁혀졌다.
  다음 작업은 visual acceptability나 success-side default 판단이 아니라, current-code latency를 official gate 안으로 되돌린 뒤 approved hardware package를 다시 수집하는 것이다.
- 사용자가 해상도 하향 경로를 더 쓰지 말라고 명시했으므로,
  current worktree는 same-capture truthful close cap을 `256x256`으로 유지하고
  방금 확인한 `192x192` 실험도 reject한 채 더 낮은 해상도 실험은 중단한다.
- current worktree는 추가로 preview renderer warm-up source를 tiny PNG에서 JPEG raster로 바꿔,
  첫 실전 컷이 real fast-preview lane과 다른 decoder cold-start를 다시 내지 않도록 보강했다.
- latest app session은 이 JPEG warm-up 보강 뒤 first-shot cold spike가 실제로 사라졌고,
  hardware validation runner latest session에서도 extreme first-shot regression이 재발하지 않았다는 검증 evidence로 유지한다.
- latest requested current-code rerun `session_000000000018a93c85f1238a00`는
  warm-up raster를 approved `256x256` truthful-close lane과 맞추고 preview-only darktable process scheduling priority를 올린 뒤 수집한 package다.
  preview ownership은 `preset-applied-preview`로 닫혔고 camera/helper readiness는 healthy였으며,
  `capturesPassed=5/5`와 direct official metrics `2819`, `2835`, `2861`, `2884`, `2831`ms를 기록했다.
  하지만 code review 후 이 package는 official Story `1.26` `Go`가 아니라 comparison evidence로만 읽는다. 이유는 original Story `1.26` boundary가 host-owned reserve path를 요구하고, look-affecting operation trimming은 truthful preset look 기준을 약하게 만들 수 있기 때문이다.

canonical reading order:

1. `docs/README.md`
2. `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
3. `docs/runbooks/current-preview-gpu-direction-20260419.md`
4. `docs/runbooks/preview-track-route-decision-20260418.md`
5. `docs/runbooks/preview-latency-next-steps-checklist-20260422.md`

## 최신 출발점

- latest session:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c85f1238a00`
- latest current-code hardware validation evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c85f1238a00`
  - runner summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777018073947\run-summary.json`
  - startup/connect는 정상으로 닫혔다.
  - invalid warm-up JPEG stderr class는 재발하지 않았다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 truthful하게 닫혔다.
  - direct metric은 전 컷 official gate 안이다.
    - `preview-render-ready elapsedMs`: `2801`, `2806`, `2810`, `2869`, `2789`
    - `originalVisibleToPresetAppliedVisibleMs`: `2819`, `2835`, `2861`, `2884`, `2831`
    - `capture_preview_ready`: `5421`, `5378`, `5439`, `5444`, `5506`
  - runner는 `status=passed`, `capturesPassed=5/5`를 기록했다.
- immediately preceding no-priority evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c5bc6cceaa0`
  - runner summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777017892848\run-summary.json`
  - warm-up source는 이미 corrected였지만 preview process priority fix 전이라 3/5컷 통과 뒤 capture 4에서 실패했다.
  - direct metric:
    - `preview-render-ready elapsedMs`: `2865`, `2922`, `2888`, `3058`
    - `originalVisibleToPresetAppliedVisibleMs`: `2877`, `2960`, `2913`, `3088`
    - `capture_preview_ready`: `5457`, `5519`, `5543`, `5634`
  - process tail jitter 확인용 rejected package다.
- previous positive hardware validation evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92a6c02e7f2d4`
  - runner summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776998171366\run-summary.json`
  - startup/connect는 정상으로 닫혔다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - latest invocation args에는 `--width 256 --height 256 --core --disable-opencl --configdir ... --library :memory:`와 trimmed fast-preview XMP cache가 남았다.
  - cached XMP `iop_order_list`는 `temperature,0,hazeremoval,0,flip,0,exposure,0,colorin,0,channelmixerrgb,0,sigmoid,0,colorout,0,gamma,0`로 줄었다.
  - direct metric은 historical package 기준 official gate를 닫았다.
    - `preview-render-ready elapsedMs`: `2916`, `2914`, `2918`, `2919`, `2915`
    - `originalVisibleToPresetAppliedVisibleMs`: `2956`, `2951`, `2961`, `2954`, `2960`
    - `capture_preview_ready`: `5010`, `4865`, `5124`, `4999`, `5153`
- latest post-fix hardware validation evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a926e98958c25c`
  - runner summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776994312446\run-summary.json`
  - startup/connect는 정상으로 닫혔다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - latest invocation args에는 `--width 256 --height 256 --core --disable-opencl --configdir ... --library :memory:`와 trimmed fast-preview XMP cache가 남았다.
  - direct metric은 official gate 바로 위까지 줄었지만 full package는 아직 gate 밖이다.
    - `preview-render-ready elapsedMs`: `3014`, `2914`, `3015`, `3012`, `2913`
    - `originalVisibleToPresetAppliedVisibleMs`: `3039`, `2955`, `3034`, `3032`, `2956`
    - `capture_preview_ready`: `5167`, `4844`, `4964`, `5078`, `4885`
- latest post-fix hardware validation evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a91e89791d5370`
  - runner summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776985103763\run-summary.json`
  - startup은 `sdk-initializing -> session-opening -> camera-ready`까지 정상 진입했다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - helper correlation도 5컷 모두 `capture-accepted -> file-arrived -> fast-preview-ready`로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
  - latest invocation args에는 `--width 256 --height 256 --core --disable-opencl --configdir ... --library :memory:`가 남아, OpenCL disable flag가 darktable core option 영역에 들어간 것이 확인됐다.
  - direct metric은 current route에서 가장 낮은 band지만 full package는 아직 gate 밖이다.
    - `preview-render-ready elapsedMs`: `2916`, `2913`, `3019`, `3115`, `2913`
    - `originalVisibleToPresetAppliedVisibleMs`: `2953`, `2960`, `3039`, `3197`, `2953`
    - `capture_preview_ready`: `5144`, `5024`, `4929`, `5119`, `4990`
- latest accepted current-code evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4`
  - startup은 다시 `camera-ready`까지 정상 진입했다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - helper status도 session 끝까지 `cameraState=ready`, `helperState=healthy`로 유지됐다.
  - helper correlation도 5컷 모두 `capture-accepted -> file-arrived -> fast-preview-ready`로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
  - latest `timing-events.log` render invocation에는 `--disable-opencl`, `--library :memory:`와 함께 `.boothy-darktable/preview/xmp-cache/...fast-preview.xmp`가 실제로 기록됐다.
  - direct metric은 first-shot miss가 사라진 채 accepted band보다 조금 더 낮은 steady-state band로 읽혔다.
    - `preview-render-ready elapsedMs`: `3317`, `3315`, `3314`, `3215`, `3316`
    - `originalVisibleToPresetAppliedVisibleMs`: `3358`, `3368`, `3357`, `3284`, `3361`
    - `capture_preview_ready`: `5757`, `5668`, `5632`, `5561`, `5623`
- latest rejected narrower-cap experiment:
  - startup은 다시 `camera-ready`까지 정상 진입했다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - helper status도 session 끝까지 `cameraState=ready`, `helperState=healthy`로 유지됐다.
  - helper correlation도 5컷 모두 `capture-accepted -> file-arrived -> fast-preview-ready`로 닫혔다.
  - latest `timing-events.log` render invocation에는 `--disable-opencl`, `--library :memory:`, `--width 192`, `--height 192`가 실제로 기록됐다.
  - direct metric은 accepted band를 더 낮추지 못했다.
    - `preview-render-ready elapsedMs`: `3515`, `3416`, `3515`, `3515`, `3415`
    - `originalVisibleToPresetAppliedVisibleMs`: `3523`, `3434`, `3599`, `3602`, `3437`
    - `capture_preview_ready`: `6632`, `6905`, `6789`, `7042`, `6617`
- latest approved hardware package:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88d53fa8f00c4`
  - startup은 다시 `camera-ready`까지 정상 진입했다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - helper correlation도 5컷 모두 `capture-accepted -> file-arrived -> fast-preview-ready`로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
  - `timing-events.log`에는 첫 컷 포함 전 컷이 `preview-render-ready ... truthOwner=display-sized-preset-applied`로 닫혔고, 직전 reject에서 보였던 두 번째 `preview-render-start`나 `existing-preview-fallback` close는 사라졌다.
- latest rejected field evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a88c71e9375868`
  - startup/connect는 정상으로 닫혔지만, 첫 컷에서
    `preview-render-start -> preview-render-start -> preview-render-failed reason=render-process-failed -> existing-preview-fallback`
    순서가 다시 남았다.
  - latest `preview-stderr-*.log` 2개에는 모두
    `Magick: caught exception 0xC0000005 "Access violation"...`
    만 남아 있었다.
  - current code는 same-capture speculative close가 renderer timeout 안에 살아 있는 동안
    second darktable render를 다시 열지 않도록 single-lane wait로 보강했다.
- latest direct gate reading from current-code app session:
  - `capture_preview_ready`: `5757ms`, `5668ms`, `5632ms`, `5561ms`, `5623ms`
  - `preview-render-ready elapsedMs`: `3317ms`, `3315ms`, `3314ms`, `3215ms`, `3316ms`
  - `originalVisibleToPresetAppliedVisibleMs`: `3358ms`, `3368ms`, `3357ms`, `3284ms`, `3361ms`
- current code logging guardrail:
  - latest approved hardware package는 아직 pre-direct-metric evidence로 남아 있다.
  - current worktree는 next session의 `capture_preview_ready` detail에
    `originalVisibleToPresetAppliedVisibleMs`, `firstVisibleAtMs`, `presetAppliedVisibleAtMs`
    가 직접 함께 남도록 보강됐고, targeted automated coverage도 통과했다.
  - current worktree는 preview renderer warm-up source를 JPEG raster로 바꾼 뒤
    `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`
    를 다시 통과했다.
  - current worktree는 preview truthful-close path에서 `--disable-opencl`을 항상 싣도록 보강했고,
    2026-04-24에는 이 flag를 `--core` 뒤로 이동해 실제 darktable core option으로 적용되게 했다.
    `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_uses_display_sized_render_arguments -- --nocapture`,
    `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_prefers_same_capture_raster_when_available -- --nocapture`
    도 다시 통과했다.
  - current worktree는 preview truthful-close path에서 preview 전용 disk library 대신 `--library :memory:`를 쓰도록 보강했고,
    `cargo test --manifest-path src-tauri/Cargo.toml final_invocation_keeps_full_resolution_render_arguments -- --nocapture`,
    `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview -- --nocapture`
    도 다시 통과했다.
  - current worktree는 speculative preview source staging에서 same-volume copy 대신 hard link를 우선 쓰도록 보강했고,
    `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_source_is_staged_to_a_stable_copy -- --nocapture`,
    `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_wait_budget_stays_bounded_even_while_another_capture_is_in_flight -- --nocapture`
    도 다시 통과했다.
  - current worktree는 preview fast-preview-raster lane가 raw-only darktable history 일부를 뺀 cached XMP를 실제 invocation에 쓰도록 보강했고,
    `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_xmp_trim_removes_raw_only_operations_from_history_and_iop_order -- --nocapture`,
    `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_trimmed_cached_xmp_when_source_xmp_is_available -- --nocapture`
    도 다시 통과했다.
  - latest `192x192` narrower-cap experiment는 hardware rerun에서 accepted band보다 나빠져 reject했고,
    current worktree는 same-capture truthful close cap을 다시 `256x256`으로 유지한다.
    `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_wait_budget_stays_bounded_without_in_flight_capture -- --nocapture`,
    `cargo test --manifest-path src-tauri/Cargo.toml speculative_preview_wait_budget_stays_bounded_even_while_another_capture_is_in_flight -- --nocapture`
    도 다시 통과했다.
- latest rejected evidence:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a868febfab83c0`
  - capture-time warm-up 시도가 render queue 경쟁을 만들어
    첫 컷을 `existing-preview-fallback` close로 밀어냈던 rejected patch evidence로만 유지한다.
- comparable latency baseline:
  - `session_000000000018a8673fd974df10`
  - `capture_preview_ready`: `8039ms`, `7580ms`, `7339ms`, `6942ms`, `5656ms`
  - `preview-render-ready elapsedMs`: `4342ms`, `4016ms`, `3718ms`, `3817ms`, `3416ms`

해석:

- latest approved rerun은 first-shot overlap family 제거 evidence로는 합격이다.
- latest current-code app session은 JPEG warm-up 뒤 extreme first-shot cold-start miss가 다시 나오지 않는다는 점을 보여 줬다.
- 다음 주력 작업은 general startup/debugging 반복이 아니라,
  전 컷 공통으로 남은 steady-state gap을 줄이는 일이다.

## 실행 체크리스트

### 1. 현재 baseline을 latest session으로 고정한다

- [x] latest startup/connect와 first-shot save가 이번 회차 기준으로 닫혔는지 확인한다.

검증 결과:

- date: `2026-04-22`
- session: `session_000000000018a8673fd974df10`
- result: `Pass as baseline evidence`
- note:
  - startup은 `camera-ready`까지 정상 진입했다.
  - 첫 컷 포함 5컷 모두 저장과 preview close까지 닫혔다.
  - 이번 회차의 주 blocker는 startup family가 아니라 latency로 읽는다.

### 2. latest field evidence를 canonical 기록에 반영한다

- [x] latest session 결과를 `history/`와 필요 시 ledger에 canonical wording으로 남긴다.

검증 결과:

- date: `2026-04-22`
- files updated:
  - `history/camera-capture-validation-history.md`
  - `docs/runbooks/preview-latency-next-steps-checklist-20260422.md`
- session: `session_000000000018a89961df9c18a0`
- result: `Pass`
- note:
  - latest canonical evidence를 `history/`에 남겨 current blocker가 다시 steady-state truthful-close latency라는 점을 고정했다.
  - post-change approved hardware rerun은 아직 없어서 ledger는 이번 턴에 갱신하지 않았다.

완료 기준:

- next agent가 이 세션을 다시 뒤지지 않아도 현재 blocker가 latency라는 점을 읽을 수 있어야 한다.

### 3. latency seam을 latest package 기준으로 다시 고정한다

- [x] latest session 기준 latency를 아래 세 seam으로 다시 읽고 기록한다.

필수 기록 항목:

- `capture acknowledged -> file arrived`
- `file arrived -> fast preview visible`
- `fast preview visible -> preset-applied visible`
- per-capture `preview-render-ready elapsedMs`

검증 결과:

- date: `2026-04-22`
- session: `session_000000000018a8673fd974df10`
- result: `Pass`
- seam summary:
  - `capture acknowledged -> file arrived`
    - `3258ms`, `3195ms`, `3272ms`, `2832ms`, `1963ms`
  - `file arrived -> fast preview visible`
    - `377ms`, `297ms`, `302ms`, `256ms`, `254ms`
  - `fast preview visible -> preset-applied visible`
    - `4404ms`, `4088ms`, `3765ms`, `3854ms`, `3439ms`
  - `preview-render-ready elapsedMs`
    - `4342ms`, `4016ms`, `3718ms`, `3817ms`, `3416ms`
- note:
  - latest remaining cost는 `file-arrived` 이후 same-capture first-visible이 아니라 `fast preview -> truthful close`였다.
  - seam 3가 `preview-render-ready elapsedMs`와 거의 겹쳐, current booth-visible hot path owner가 여전히 per-capture `darktable-cli`라는 점을 다시 확인했다.

완료 기준:

- 다음 구현이 “어디를 줄일지”를 숫자로 고정할 수 있어야 한다.
- 이번 회차처럼 first-shot/save가 정상인 상태에서 남은 비용이 어디인지 분리되어야 한다.

### 4. booth-visible truthful close의 hot path를 더 싼 경계로 줄인다

- [ ] `per-capture darktable-cli`가 booth-visible truthful close를 사실상 소유하는 현재 비용을 줄이는 구현 시도를 한다.

이번 단계의 방향:

- `Preview Waiting -> Preview Ready` truth contract는 유지한다.
- darktable는 parity reference / fallback / final-export truth로 남길 수 있다.
- 하지만 booth-visible truthful close는 더 낮은 비용 경계로 당기는 쪽을 우선 검토한다.

검증 결과:

- date: `2026-04-22`
- files updated:
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src/capture-adapter/services/capture-runtime.ts`
  - `src/session-domain/state/session-provider.tsx`
  - `src/capture-adapter/services/capture-runtime.test.ts`
  - `src/session-domain/state/session-provider.test.tsx`
- tests:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
  - `pnpm test:run src/capture-adapter/services/capture-runtime.test.ts --testNamePattern "primes the preview runtime"`
  - `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime when an active-preset session enters capture flow"`
  - `pnpm test:run src/session-domain/state/session-provider.test.tsx --testNamePattern "primes the preview runtime|waits for an in-flight preview runtime prime|starts the preview runtime prime before the capture-flow effect can race ahead of the first request"`
- result: `Prime race fix still held in latest rerun, but completion criteria not met`
- note:
  - latest relaunch capture session `session_000000000018a89961df9c18a0`에서도
    first shot direct metric은 `3820ms / 3877ms`로 latest reject family의 cold spike 없이 닫혔다.
  - 따라서 preview prime scheduling race 보강은 latest rerun에서도 계속 효과가 있었던 것으로 읽는 편이 맞다.
  - latest session helper log는 `fast-preview-ready fastPreviewKind=windows-shell-thumbnail`를 남겼는데도,
    persist 단계에서는 canonical preview를 다시 `legacy-canonical-scan`으로 읽어
    host-owned early preview metadata를 잃고 있었다.
  - current worktree는 helper `fast-preview-ready`가 먼저 도착한 경우
    그 `kind`와 `visibleAtMs`를 manifest까지 그대로 보존하고,
    `windows-shell-thumbnail` 같은 host-owned same-capture source에서는
    `file-arrived` 이후 seed를 기다리지 않고 reserve close를 바로 시작하게 보강했다.
  - targeted regression `early_windows_shell_thumbnail_is_preserved_and_starts_reserve_close_before_file_arrival_metadata`
    와 full `capture_readiness` suite는 모두 통과했다.
  - 그 뒤 latest 실행 세션 `session_000000000018a89b380d42939c`는 preview latency family가 아니었다.
    `timing-events.log`는 `request-capture`까지만 남았고,
    helper status는 오래된 `cameraState=capturing / helperState=healthy / detailCode=capture-in-flight`
    에 멈췄으며,
    `camera-helper-processed-request-ids.txt`에는 해당 request id가 남아 있었다.
  - current worktree는 이 accepted-only stall을 별도 family로 취급해,
    stale `capture-in-flight` helper status는 약 `45s` 이후 restart 대상에 포함하고,
    저장된 capture가 없는 `phone-required` 세션도
    processed request evidence가 있을 때만 helper ready 복구 뒤 `capture-ready`로 되돌리게 보강했다.
  - targeted regression `readiness_releases_phone_required_without_saved_capture_once_helper_is_ready_again`,
    helper unit `stale_capture_in_flight_status_requests_a_helper_restart`,
    full `capture_readiness` suite는 모두 통과했다.
  - 반면 5컷 전체가 여전히 `preview-render-ready 3.4s ~ 3.9s`,
    `originalVisibleToPresetAppliedVisibleMs 3.4s ~ 3.9s`
    에 머물러 남은 blocker는 steady-state truthful-close latency다.
  - current worktree는 사용자 요청에 맞춰 same-capture `fast-preview-raster` truthful close cap을
    `256x256`으로 유지한다.
  - 이번 latest 세션은 그 `256x256` 복귀 상태에서 수집된 evidence다.
  - capture request path에서 새 warm-up을 시작하지는 않으므로,
    previous reject 원인이었던 capture-time overlap family는 되살리지 않게 유지했다.
  - 아직 comparable hardware rerun은 다시 수집하지 않았으므로,
    hot path reduction success로는 아직 셀 수 없다.

완료 기준:

- latest comparable run에서 `preview-render-ready elapsedMs`와 `capture_preview_ready`가 함께 내려와야 한다.
- first-visible만 빨라지고 truthful close는 그대로인 경우 성공으로 세지 않는다.

### 5. truth contract와 correctness guardrail을 유지한다

- [x] latency를 줄이는 동안 아래 guardrail이 깨지지 않는지 함께 검증한다.

필수 guardrail:

- `previewReady`는 계속 truthful close asset만 소유한다.
- same-session / same-capture correctness 유지
- wrong-capture `0`
- cross-session leakage `0`
- false-ready `0`

검증 결과:

- date: `2026-04-22`
- tests:
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness client_recent_session_visibility_events_are_mirrored_into_session_timing_logs -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_allows_next_capture_once_same_capture_fast_preview_is_visible -- --exact`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_keeps_waiting_for_a_slow_speculative_close -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_still_avoids_a_duplicate_render_while_speculative_close_is_active -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_closes_with_existing_same_capture_preview_when_raw_refinement_fails -- --exact`
- session: `session_000000000018a868febfab83c0` regression evidence, `session_000000000018a88c71e9375868` latest duplicate-render reject, `session_000000000018a8673fd974df10` comparable baseline
- result: `Pass in automated regression coverage`
- note:
  - latest field session은 first-shot false-ready regression을 보여 줬지만,
    current code는 capture-time warm-up 경로를 제거한 뒤 full `capture_readiness` sequential suite를 다시 통과했다.
  - 즉 current worktree 기준 소프트웨어 guardrail은 다시 truth-first 상태로 돌아왔다.
  - latest field rerun `session_000000000018a88c71e9375868`는
    slow speculative close가 settle되기 전에 second render를 다시 연 흔적으로 읽혔고,
    current code는 renderer timeout 경계 안에서는 duplicate render를 다시 열지 않게 보강했다.
  - latest approved rerun `session_000000000018a88d53fa8f00c4`에는
    official gate 구간이 direct metric으로는 없었고,
    current code는 이제 `capture_preview_ready` detail에
    `originalVisibleToPresetAppliedVisibleMs`를 같이 남기도록 보강했다.
  - 이번 latest 로그 재확인에서도 더 새로운 capture package는 생기지 않았으므로,
    direct metric의 실제 hardware evidence는 다음 rerun에서 다시 확인해야 한다.
  - approved hardware one-session package는 이번 logging patch 뒤 아직 다시 수집하지 않았다.

완료 기준:

- latency 개선이 있더라도 truth contract를 느슨하게 만드는 변경은 reject한다.

### 6. 승인 하드웨어 one-session package로 다시 판정한다

- [x] software change 뒤 approved hardware one-session package를 한 번 수집한다.

필수 판정 항목:

- official gate:
  - `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
- supporting evidence:
  - startup/connect 정상 진입
  - first-shot save 정상
  - truthful close owner
  - same-session correctness

검증 결과:

- date: `2026-04-22`
- session: `session_000000000018a88d53fa8f00c4`
- result: `No-Go`
- gate numbers:
  - `5905ms`, `5982ms`, `5763ms`, `5864ms`, `5803ms`
- note:
  - startup/connect는 `camera-ready`까지 정상 진입했고, first-shot save도 다시 정상으로 닫혔다.
  - 5컷 모두 `previewReady`는 truthful close asset인 `display-sized-preset-applied`만 소유했다.
  - 직전 reject session `session_000000000018a88c71e9375868`에서 보였던 first-shot duplicate render overlap과 `existing-preview-fallback` close는 latest approved rerun에서 사라졌다.
  - 하지만 official gate `preset-applied visible <= 3000ms`는 한 컷도 통과하지 못했다.
  - 이번 verdict는 ledger에 `No-Go`로 기록했다.

완료 기준:

- official `Go / No-Go`는 ledger에 기록한다.
- 이번 단계도 gate 실패면 다음 route change 여부를 다시 판단한다.

## 에이전트 실행 규칙

- 새 작업을 시작할 때 먼저 이 문서의 미완료 `[ ]` 중 가장 위의 항목을 잡는다.
- 작업이 끝나면 해당 항목을 `[x]`로 바꾸기 전에 `검증 결과`를 먼저 채운다.
- 하나의 턴에서 범위를 넓히지 않는다.
- startup family가 다시 깨지지 않은 한, first-shot debugging으로 임의 복귀하지 않는다.
- current blocker가 다시 달라졌다면 이 문서의 `현재 기준 판단`과 `최신 출발점`을 먼저 갱신한다.

## 현재 다음 단계

지금 바로 다음 단계는 Story `1.26`의 product close 판단이 아니라, code review patch가 적용된 current-code latency를 official gate 안으로 다시 넣는 것이다.

- latest hardware validation runner session `session_000000000018a934f66a92fe80`에서 false-ready는 차단됐고, runner가 `preview-truth-gate-failed`로 멈췄다.
- capture 1은 truthful `preset-applied-preview`로 닫혔지만 `3037ms`로 official gate를 37ms 넘었다.
- 따라서 다음 작업은 visual acceptability나 Story `1.31` 판단이 아니라, current-code truthful-close tail을 줄인 뒤 동일 runner로 approved hardware package를 다시 수집하는 것이다.
- 다음 hardware rerun에서는 `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`, `originalVisibleToPresetAppliedVisibleMs <= 3000ms`를 모두 만족해야 한다.
- latest relaunch capture sessions
  `session_000000000018a88f5792a50534`,
  `session_000000000018a88fa09dcbdca0`,
  `session_000000000018a89441063a0c54`,
  `session_000000000018a895d248f89000`,
  `session_000000000018a8975f5f4f0b34`,
  `session_000000000018a8986df67c7e38`,
  `session_000000000018a89961df9c18a0`
  에서는 direct metric이 실제로 찍혔고,
  latest app session `session_000000000018a8e91cef5631a8`에서는
  first shot도 extreme spike 없이 `3440ms`로 닫혔지만
  3번째 컷이 `3599ms`까지 다시 올라와,
  blocker가 여전히 steady-state band 안에서 흔들리는 상태로 남았다.
- 참고:
  - latest startup-only relaunch session `session_000000000018a88adfee94784c`는 `camera-ready`까지 정상 진입했다.
  - 이번 턴에서는 app re-entry가 existing active preset capture flow로 돌아올 때도
    reserve lane을 다시 warm state로 데우고,
    그 in-flight prime이 first capture 전에 먼저 settle되도록 보강했다.
  - latest evidence를 반영해,
    preview prime 자체를 `useEffect`보다 앞선 session/preset 전이 시점에도 즉시 시작하도록 보강했다.
  - 사용자가 요청한 대로 same-capture `fast-preview-raster` truthful close cap은
    `256x256`으로 유지하고, 더 낮은 해상도 실험은 중단한다.
  - current worktree는 추가로 preview renderer warm-up source를 JPEG raster로 바꿔
    first 실전 컷이 real fast-preview family와 같은 decoder path를 먼저 타게 보강했다.
  - current worktree는 추가로 preview truthful-close path에서 OpenCL startup cost를 빼기 위해
    `--disable-opencl`을 실제 render invocation에 싣도록 보강했다.
  - current worktree는 추가로 preview truthful-close path에서 preview 전용 sqlite startup을 빼기 위해
    `--library :memory:`를 실제 render invocation에 싣도록 보강했다.
  - current worktree는 추가로 speculative source staging에서 same-volume copy 대신 hard link를 우선 쓰도록 보강했지만,
    latest runner 기준으로는 gate-closing improvement까지 이어지지 않았다.
  - current worktree는 추가로 preview fast-preview-raster lane가 raw-only darktable history 일부를 덜어낸 cached XMP를 실제 invocation에 쓰도록 보강했고,
    latest runner `session_000000000018a8fe95ea36f8f4`에서는 `.boothy-darktable/preview/xmp-cache/...fast-preview.xmp`가 실제 args에 남으면서
    direct band가 `3284ms ~ 3368ms`까지 조금 더 내려왔다.
  - latest 판단은 code review patch 이후 hardware rerun에서
    `originalVisibleToPresetAppliedVisibleMs=3037ms`가 나와 `No-Go`를 유지했다.

### 2026-04-23 22:13 +09:00 hardware validation runner latest session에서는 preview fast-preview-raster lane가 trimmed XMP cache를 실제로 사용했고 steady-state band가 조금 더 낮아졌지만 gate는 아직 닫히지 않았다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 다시 확인해 story `1-26`, ledger, 관련 문서에 기록하고 다음 시도해야 할 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- fast-preview-raster preview lane가 raw-only darktable history 일부를 덜어낸 cached XMP를 쓰도록 보강한 뒤
  `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- runner summary는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8fe95ea36f8f4`로 닫혔다.
- latest session `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a8fe95ea36f8f4`는 startup/connect가 정상으로 닫혔다.
  - `camera-helper-startup.log`: `sdk-initializing -> session-opening -> camera-ready`
  - `camera-helper-status.json`: session 종료 시점에도 `cameraState=ready`, `helperState=healthy`
- helper correlation을 보면 5컷 모두
  `capture-accepted -> file-arrived -> fast-preview-ready`
  로 닫혔고 `fastPreviewKind = windows-shell-thumbnail`였다.
- latest `timing-events.log` render invocation args에는 `--disable-opencl`, `--library :memory:`와 함께
  `.boothy-darktable/preview/xmp-cache/preset-new-draft-2-2026-04-10-look2-fast-preview.xmp`
  가 실제로 남았다.
- direct metric은 first-shot extreme spike가 아니라, 다섯 컷 모두 조금 더 낮은 steady-state band로 읽혔다.
  - `preview-render-ready elapsedMs`: `3317`, `3315`, `3314`, `3215`, `3316`
  - `originalVisibleToPresetAppliedVisibleMs`: `3358`, `3368`, `3357`, `3284`, `3361`
  - `capture_preview_ready elapsedMs`: `5757`, `5668`, `5632`, `5561`, `5623`

이번 회차 해석:

- first-shot extreme spike는 latest session에서도 재발하지 않았다.
- preview fast-preview-raster lane가 lighter XMP를 실제로 쓴 것은 확인됐고, steady-state band도 accepted `256x256` evidence보다 약간 더 내려왔다.
- 하지만 official gate `<= 3000ms`는 여전히 넘고 있어 current blocker는 계속 steady-state truthful-close latency다.

이번 회차 조치:

- preview fast-preview-raster lane가 raw-only darktable history 일부를 덜어낸 cached XMP를 실제 invocation에 쓰도록 보강했다.
- story `1-26`, hardware validation ledger, preview latency checklist, validation history를 latest session 기준으로 다시 갱신했다.
- latest runner evidence를 canonical 문서들에 다시 연결했다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_xmp_trim_removes_raw_only_operations_from_history_and_iop_order -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_trimmed_cached_xmp_when_source_xmp_is_available -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_invocation_prefers_same_capture_raster_when_available -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source_is_written_as_jpeg -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`

이번 시점 제품 판단:

1. latest hardware-validation-runner session은 5/5 통과와 first-shot spike 미재발, 그리고 trimmed XMP cache 실제 적용을 함께 보여 줬다.
2. 하지만 latest band가 `3284ms ~ 3368ms`로 여전히 official gate 밖이라 Story `1.26`은 계속 `No-Go`다.
3. 다음 개선은 추가 해상도 축소가 아니라, darktable truthful-close fixed cost를 더 줄이거나 host-owned truthful close owner를 더 앞당기는 쪽이어야 한다.

### 2026-04-24 10:00 +09:00 hardware validation runner helper-bootstrap recovery는 readiness timeout을 줄였지만 official gate는 latency tail 때문에 아직 No-Go다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 확인해 story `1-26`, ledger, 관련 문서에 기록하고 다음 시도해야 할 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- 직전 실패 세션 `session_000000000018a92491e8b75984`, `session_000000000018a924971612f514`는 모두 촬영 샘플 없이 `capture-readiness-timeout`으로 실패했다.
- failure diagnostics 수집 시점에는 helper status/startup log가 없었고, 한 세션은 실패 뒤에야 `camera-ready` startup log가 늦게 남았다.
- 이는 app command path가 helper start를 먼저 열어 주는 일반 UI 흐름과 달리, hardware validation runner의 direct library path가 helper bootstrap을 충분히 보장하지 못한 것으로 해석한다.

이번 회차 조치:

- `src-tauri/src/automation/hardware_validation.rs`에서 readiness 대기 중 missing helper status가 1초 이상 유지되면 runner가 helper bootstrap을 직접 한 번 요청하도록 보강했다.
- `cargo test --manifest-path src-tauri/Cargo.toml --test hardware_validation_runner -- --test-threads=1`는 통과했다.
- 요청한 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 한 번 실행했다.

검증 결과:

- runner summary: `status=passed`, `capturesPassed=5/5`
- run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776992377859\run-summary.json`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a925271b1710a0`
- startup/connect: `sdk-initializing -> session-opening -> camera-ready`; session 종료 시점 `cameraState=ready`, `helperState=healthy`
- truthful close: 5컷 모두 `renderStatus=previewReady`, `preview.kind=preset-applied-preview`
- latest invocation은 `--width 256 --height 256 --core --disable-opencl --configdir ... --library :memory:`와 trimmed fast-preview XMP cache를 유지했다.
- direct metric:
  - `preview-render-ready elapsedMs`: `3014`, `2912`, `2923`, `3214`, `2915`
  - `originalVisibleToPresetAppliedVisibleMs`: `3037`, `2952`, `2974`, `3279`, `2954`
  - `capture_preview_ready elapsedMs`: `5237`, `5157`, `5071`, `5331`, `4995`

이번 시점 제품 판단:

1. runner-side readiness timeout family는 이번 단일 실행에서 재발하지 않았다.
2. 다만 official `<= 3000ms` gate는 2/5컷 tail miss로 아직 닫히지 않았다.
3. 다음 시도는 helper/readiness가 아니라 `3014ms`, `3214ms`로 남는 darktable truthful-close tail jitter를 줄이는 방향이어야 한다.

### 2026-04-24 10:19 +09:00 hardware validation runner compact prompt parsing은 식별자를 고쳤지만 official gate는 latency tail 때문에 아직 No-Go다

사용자 최신 요청:

1. 최신 앱 실행 세션 로그를 확인해 story `1-26`, ledger, 관련 문서에 기록하고 다음 시도해야 할 방법을 찾아 개선하라고 요청했다.
2. 코드 개선 뒤 하드웨어 검증 스크립트를 한 번 실행하라고 요청했다.

실제 확인 근거:

- 직전 latest runner summary에서 `Prompt "Kim4821"`이 `boothAlias="Kim4821 0000"`으로 기록됐다.
- 이는 현장 운영자가 이름과 뒤 4자리를 붙여 입력할 때 고객 식별자가 틀어지는 문제다.
- current worktree는 compact prompt를 `Kim 4821`로 분리하도록 runner parsing을 보강했다.
- `cargo test --test hardware_validation_runner -- --test-threads=1` 통과 뒤 요청 커맨드를 한 번 실행했다.

검증 결과:

- runner summary: `status=passed`, `capturesPassed=5/5`
- run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1776993558412\run-summary.json`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a92639f9a96a6c`
- identity: `boothAlias=Kim 4821`, `name=Kim`, `phoneLastFour=4821`
- startup/connect: session 종료 시점 `cameraState=ready`, `helperState=healthy`
- direct metric:
  - `preview-render-ready elapsedMs`: `3115`, `2915`, `2915`, `2913`, `3219`
  - `originalVisibleToPresetAppliedVisibleMs`: `3200`, `2956`, `2955`, `2958`, `3276`
  - `capture_preview_ready elapsedMs`: `5397`, `5001`, `4857`, `4849`, `5184`

이번 시점 제품 판단:

1. latest 단일 실행은 고객 식별자와 5/5 capture pass를 함께 회복했다.
2. 하지만 official gate는 `3200ms`, `3276ms` tail miss 때문에 아직 닫히지 않았다.
3. 다음 개선은 prompt/readiness가 아니라 truthful-close latency tail jitter를 더 줄이는 방향이어야 한다.

### 2026-04-24 10:32 +09:00 extra fast-preview XMP trimming은 tail을 40ms 안쪽으로 좁혔지만 official gate는 아직 No-Go다

최신 실행에서 확인한 점:

- 직전 latest session `session_000000000018a92639f9a96a6c`는 readiness와 prompt가 아니라 darktable render tail 때문에 실패했다.
- current worktree는 fast-preview JPEG 입력에 불필요한 `lens`, `highlights`, `cacorrectrgb` history를 cached XMP에서 추가 제거했다.
- 요청 커맨드는 한 번 실행했고 `status=passed`, `capturesPassed=5/5`로 닫혔다.
- latest session `session_000000000018a926e98958c25c`는 `Kim 4821` 식별자와 5/5 `preset-applied-preview` truthful close를 유지했다.
- direct metric은 `3039`, `2955`, `3034`, `3032`, `2956`ms였다.

판단:

- tail은 이전 `3276ms`에서 `3039ms`까지 줄었고 평균도 `3003.2ms`까지 붙었다.
- 하지만 official `<= 3000ms` gate는 3컷이 `32ms ~ 39ms` 넘어서 아직 닫히지 않았다.
- 다음 시도는 해상도, readiness, prompt가 아니라 cached XMP에 남은 duplicate builtin/default work를 시각 차이 없이 줄이거나 host-owned truthful close owner를 더 앞당기는 방향이다.

### 2026-04-24 11:36 +09:00 fast-preview cached XMP iop-order trimming은 historical Story 1.26 gate를 닫았다

최신 실행에서 확인한 점:

- 직전 latest session `session_000000000018a9292e867e1a68`는 5/5 capture와 truth owner는 유지했지만 `3035ms`, `3036ms`, `3039ms` tail miss가 남았다.
- cached XMP history는 이미 9개 operation으로 줄었지만 `iop_order_list`에는 history에서 제거된 default pipeline 항목이 계속 남아 있었다.
- current worktree는 fast-preview cached XMP의 `iop_order_list`를 실제 유지된 preview history operation/priority만 남기도록 줄였다.
- 요청 커맨드는 한 번 실행했고 `status=passed`, `capturesPassed=5/5`로 닫혔다.
- latest session `session_000000000018a92a6c02e7f2d4`는 `Kim 4821` 식별자와 5/5 `preset-applied-preview` truthful close를 유지했다.
- direct metric은 `2956`, `2951`, `2961`, `2954`, `2960`ms였다.

판단:

- 이 회차만 보면 Story `1.26`의 official `<= 3000ms` gate는 닫혔다.
- 하지만 이후 code review patch가 적용된 current-code rerun이 `3196ms`로 실패했으므로, 이 회차는 historical positive evidence로만 남긴다.

### 2026-04-24 13:59 +09:00 code review patch 후 current-code rerun은 false-ready를 막았지만 official gate를 다시 넘겼다

최신 실행에서 확인한 점:

- code review finding 1은 render failure fallback이 non-truthful 기존 preview만으로 `previewReady`를 만들 수 있다는 문제였다.
- code review finding 2는 hardware validation runner가 `previewReady`만 보고 truthful owner와 official gate를 검증하지 않는 문제였다.
- current worktree는 render failure fallback을 truthful `preset-applied-preview`가 이미 있는 경우에만 닫도록 막고, runner가 `preview.kind`, `xmpPreviewReadyAtMs`, `originalVisibleToPresetAppliedVisibleMs <= 3000ms`를 함께 검증하도록 보강했다.
- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다.
- latest session `session_000000000018a9323a28789b40`는 `Kim 4821` 식별자와 truthful `preset-applied-preview` close를 유지했다.
- direct metric은 capture 1에서 `3196ms`였다.

판단:

- false-ready class는 current-code에서 차단됐다.
- 하지만 current-code approved hardware proof는 official `<= 3000ms` gate를 196ms 넘겨 `No-Go`다.
- 이전 `session_000000000018a92a6c02e7f2d4`의 5/5 `Go` package는 historical positive evidence로만 남기고, 제품 close 근거로 쓰지 않는다.
- 다음 시도는 추가 문서 판단이 아니라 current-code latency를 다시 줄이고 같은 runner로 one approved hardware package를 재수집하는 것이다.

### 2026-04-24 14:22 +09:00 요청 스크립트 재실행도 truthful close는 유지했지만 official gate를 크게 넘겼다

최신 실행에서 확인한 점:

- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
- runner summary는 `status=failed`, `capturesPassed=0/5`, `sessionId=session_000000000018a9337745615574`, `boothAlias=Kim 4821`로 닫혔다.
- failure code는 `preview-truth-gate-failed`였다.
- helper/camera readiness는 정상이다.
  - `cameraState=ready`
  - `helperState=healthy`
  - `detailCode=camera-ready`
- capture 1은 `renderStatus=previewReady`, `preview.kind=preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 truthful하게 닫혔다.
- direct metric은 official gate 밖이다.
  - `preview-render-ready elapsedMs=12984`
  - `originalVisibleToPresetAppliedVisibleMs=13104`
  - `capture_preview_ready elapsedMs=15747`

판단:

- code review patch는 non-truthful false-ready class를 계속 막고 있다.
- 이번 실패는 장비 준비나 owner mismatch가 아니라 current-code truthful-close latency miss다.
- Story `1.26`은 계속 `review / No-Go`이며, 다음 단계는 current-code latency를 official gate 안으로 되돌린 뒤 같은 runner로 다시 검증하는 것이다.

### 2026-04-24 14:49 +09:00 요청 스크립트 재실행은 37ms tail miss로 No-Go를 유지했다

최신 실행에서 확인한 점:

- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
- runner summary는 `status=failed`, `capturesPassed=0/5`, `sessionId=session_000000000018a934f66a92fe80`, `boothAlias=Kim 4821`로 닫혔다.
- failure code는 `preview-truth-gate-failed`였다.
- helper/camera readiness는 정상이다.
  - `cameraState=ready`
  - `helperState=healthy`
  - `detailCode=camera-ready`
- capture 1은 `renderStatus=previewReady`, `preview.kind=preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 truthful하게 닫혔다.
- direct metric은 official gate 밖이다.
  - `preview-render-ready elapsedMs=3013`
  - `originalVisibleToPresetAppliedVisibleMs=3037`
  - `capture_preview_ready elapsedMs=5618`

판단:

- code review patch는 non-truthful false-ready class를 계속 막고 있다.
- 이번 실패는 장비 준비나 owner mismatch가 아니라 37ms current-code truthful-close latency miss다.
- Story `1.26`은 계속 `review / No-Go`이며, 다음 단계는 current-code latency tail을 official gate 안으로 되돌린 뒤 같은 runner로 다시 검증하는 것이다.

### 2026-04-24 15:26 +09:00 latest latency patch는 일부 컷을 gate 안으로 넣었지만 full package는 아직 No-Go다

최신 실행에서 확인한 점:

- 최근 실패 로그의 공통 원인은 장비 준비가 아니라 truthful `preset-applied-preview` close latency tail이었다.
- current worktree는 fast-preview XMP hot path에서 `lens`와 `hazeremoval`까지 제외하고, preview cap을 `192x192`, preview process polling을 `20ms`로 낮췄다.
- 검증 중 best patched run `session_000000000018a936ed27302174`는 3/5컷이 official gate 안에 들어왔다.
  - `originalVisibleToPresetAppliedVisibleMs`: `2983`, `2889`, `2965`, then failure `3051`
- latest requested script rerun은 `session_000000000018a936fcad8c042c`에서 capture 2 `3118ms`로 실패했다.
  - run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777011986840\run-summary.json`
  - timing log: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a936fcad8c042c\diagnostics\timing-events.log`

판단:

- 개선 방향은 맞지만, 5/5 official `<= 3000ms` package는 아직 닫히지 않았다.
- Story `1.26`은 계속 `review / No-Go`다.
- 다음 단계는 단순 cap 축소가 아니라 darktable process tail jitter 자체를 줄이거나, helper/host-owned truthful preview owner를 darktable process 밖으로 더 앞당기는 방향이어야 한다.

### 2026-04-24 16:11 +09:00 runner warm-up은 cold spike를 줄였지만 official package는 아직 No-Go다

최신 실행에서 확인한 점:

- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
- runner가 preset 선택 뒤 `preview-runtime-warmed` step을 기록하도록 보강했고, latest run에서도 `warmupSettled=true`가 남았다.
- first-shot `14475ms` cold spike는 줄었지만, latest requested rerun은 still failed다.
  - run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777014698916\run-summary.json`
  - session: `session_000000000018a9397421a5ad30`
  - timing log: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9397421a5ad30\diagnostics\timing-events.log`
  - `preview-render-ready elapsedMs=3079`
  - `originalVisibleToPresetAppliedVisibleMs=3107`
- best run in this pass는 `session_000000000018a93925842ee7b8`였고 3/5컷이 gate 안에 들어왔다.
  - `originalVisibleToPresetAppliedVisibleMs`: `2883`, `2869`, `2940`, then failure `3306`

판단:

- readiness, camera/helper health, preview ownership은 정상이다.
- blocker는 여전히 truthful `preset-applied-preview` close의 darktable latency tail이다.
- Story `1.26`은 계속 `review / No-Go`다.

### 2026-04-24 16:27 +09:00 invalid warm-up JPEG는 고쳤지만 official package는 아직 No-Go다

최신 실행에서 확인한 점:

- latest preview stderr에는 warm-up source JPEG가 `Invalid JPEG file structure`로 실패하던 흔적이 있었다.
- current worktree는 built-in warm-up JPEG를 decodable JPEG로 교체했고, 관련 테스트를 통과했다.
- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
  - run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777015622619\run-summary.json`
  - session: `session_000000000018a93a4b32aba8c8`
  - timing log: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93a4b32aba8c8\diagnostics\timing-events.log`
  - result: `status=failed`, `capturesPassed=1/5`, failure `preview-truth-gate-failed`
  - capture 1: `preview-render-ready elapsedMs=2855`, `originalVisibleToPresetAppliedVisibleMs=2881`
  - capture 2: `preview-render-ready elapsedMs=3009`, `originalVisibleToPresetAppliedVisibleMs=3035`
- 새 회차에서는 warm-up JPEG stderr 파일이 추가로 생기지 않았고, helper/camera readiness는 `camera-ready` / `healthy`로 정상이다.

판단:

- invalid warm-up JPEG 문제는 닫혔다.
- latest official blocker는 capture 2의 `35ms` latency tail miss다.
- Story `1.26`은 계속 `review / No-Go`다.

### 2026-04-24 16:48 +09:00 approved 256x256 cap 복구 뒤에도 official package는 아직 No-Go다

최신 실행에서 확인한 점:

- latest app/runner 로그를 다시 읽어 current worktree가 previous positive package와 사용자 지시의 기준인 `256x256`이 아니라 `224x224` truthful-close cap으로 내려가 있음을 확인했다.
- current worktree는 `256x256` cap을 복구했고, `hazeremoval` 유지 가설은 하드웨어에서 더 느려져 reject했다.
- 빠른 프리뷰 관련 자동 테스트는 통과했다.
  - `cargo test --manifest-path src-tauri/Cargo.toml fast_preview -- --nocapture`
- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
  - final run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777016810008\run-summary.json`
  - final session: `session_000000000018a93b5fa88505d4`
  - final timing log: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93b5fa88505d4\diagnostics\timing-events.log`
  - result: `status=failed`, `capturesPassed=0/5`, failure `preview-truth-gate-failed`
  - capture 1: `preview-render-ready elapsedMs=3032`, `originalVisibleToPresetAppliedVisibleMs=3056`
- same-turn best run:
  - run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777016421699\run-summary.json`
  - session: `session_000000000018a93b053f7dbab8`
  - result: `status=failed`, `capturesPassed=4/5`
  - `originalVisibleToPresetAppliedVisibleMs`: `2883`, `2965`, `2893`, `2933`, then failure `3173`

판단:

- `224x224` cap drift는 corrected; current route is back on approved `256x256`.
- truth ownership, prompt parsing, helper/camera readiness는 정상이다.
- latest blocker는 여전히 darktable truthful-close tail jitter이며, final package is `No-Go`.
- 다음 시도는 더 낮은 해상도 실험이 아니라, darktable process tail을 더 줄이거나 darktable 밖의 host-owned truthful-close owner를 앞당기는 방향이어야 한다.

### 2026-04-24 17:08 +09:00 preview process tail jitter fix run은 comparison pass로 낮춘다

최신 실행에서 확인한 점:

- 직전 no-priority 실행은 `session_000000000018a93c5bc6cceaa0`에서 3/5컷까지 통과했지만 capture 4가 `3088ms`로 실패했다.
- current worktree는 warm-up raster를 approved `256x256` truthful-close lane과 맞추고, preview darktable process에만 Windows above-normal scheduling priority를 적용했다.
- 관련 자동 테스트를 통과했다.
  - `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml preview_darktable_process_gets_latency_priority_on_windows -- --nocapture`
- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
  - run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777018073947\run-summary.json`
  - session: `session_000000000018a93c85f1238a00`
  - timing log: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a93c85f1238a00\diagnostics\timing-events.log`
  - result: `status=passed`, `capturesPassed=5/5`
  - `preview-render-ready elapsedMs`: `2801`, `2806`, `2810`, `2869`, `2789`
  - `originalVisibleToPresetAppliedVisibleMs`: `2819`, `2835`, `2861`, `2884`, `2831`
  - `capture_preview_ready`: `5421`, `5378`, `5439`, `5444`, `5506`

판단:

- prompt parsing, helper/camera readiness는 정상이다.
- all direct official metrics are under `<= 3000ms`.
- 하지만 code review 뒤 제품 판단은 이 run을 official Story `1.26` `Go`가 아니라 comparison pass로 낮춘다.
- 다음 close 시도는 look-affecting preview XMP operation을 보존하고, Story `1.26`의 host-owned reserve path boundary와 맞는 새 evidence package를 수집해야 한다.

### 2026-04-25 02:09 +09:00 requested high-priority preview rerun은 5/5 gate를 닫았다

최신 실행에서 확인한 점:

- 직전 요청 실행 `hardware-validation-run-1777050318855`는 warm-up과 helper/camera readiness가 정상임에도 capture 1이 `3067ms`로 official gate를 `67ms` 넘겼다.
- current worktree는 preview darktable process priority를 Windows high priority로 올리고, final render priority는 변경하지 않았다.
- 관련 자동 테스트를 통과했다.
  - `cargo test --manifest-path src-tauri\Cargo.toml preview_darktable_process_gets_latency_priority_on_windows`
- 요청 커맨드 `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다.
  - run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777050552254\run-summary.json`
  - session: `session_000000000018a95a0fe32405b8`
  - timing log: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a95a0fe32405b8\diagnostics\timing-events.log`
  - result: `status=passed`, `capturesPassed=5/5`
  - `preview-render-ready elapsedMs`: `2802`, `2845`, `2849`, `2848`, `2855`
  - `originalVisibleToPresetAppliedVisibleMs`: `2825`, `2873`, `2867`, `2864`, `2870`

판단:

- prompt parsing, helper/camera readiness, truthful `preset-applied-preview` close는 정상이다.
- all direct official metrics are under `<= 3000ms`.
- 제품 판정은 기존 기준을 유지한다. 이번 run은 latency tail 개선 evidence이며, Story `1.26`의 official close는 host-owned reserve path boundary와 truthful preset-look preservation까지 다시 맞춘 뒤 판단한다.
