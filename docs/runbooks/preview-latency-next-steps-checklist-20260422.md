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
- 지금 immediate blocker는 startup/save나 false-ready가 아니라, `display-sized-preset-applied truthful close`가 여전히 `3.6s ~ 3.7s`대로 남아 official gate를 넘기지 못하는 일이다.
- 다만 latest approved hardware package `session_000000000018a88d53fa8f00c4`는 아직 pre-direct-metric evidence라
  `timing-events.log`에 official gate 구간이 direct metric으로는 없었다.
- current worktree는 이제 `capture_preview_ready` detail에
  `originalVisibleToPresetAppliedVisibleMs`, `firstVisibleAtMs`, `presetAppliedVisibleAtMs`
  를 같이 남기고, 관련 자동 검증도 통과했다.
- latest relaunch field evidence `session_000000000018a89961df9c18a0`는
  `256x256` 복귀 상태에서 first shot도
  `preview-render-ready elapsedMs=3820`,
  `originalVisibleToPresetAppliedVisibleMs=3877`
  로 latest reject family의 cold spike는 보이지 않았다.
- 따라서 latest rerun 기준으로 `effect-based preview prime race`는 다시 완화된 것으로 읽고,
  current immediate blocker는 다시 전 컷에 남는
  `display-sized-preset-applied truthful close`의 steady-state `3.4s ~ 3.9s`
  로 읽는 편이 맞다.
- 사용자가 해상도 하향 경로를 더 쓰지 말라고 명시했으므로,
  current worktree는 same-capture truthful close cap을 `256x256`으로 유지하고
  더 낮은 해상도 실험은 중단한다.

canonical reading order:

1. `docs/README.md`
2. `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
3. `docs/runbooks/current-preview-gpu-direction-20260419.md`
4. `docs/runbooks/preview-track-route-decision-20260418.md`
5. `docs/runbooks/preview-latency-next-steps-checklist-20260422.md`

## 최신 출발점

- latest session:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a89961df9c18a0`
- latest accepted current-code evidence:
  - startup은 다시 `camera-ready`까지 정상 진입했다.
  - 5컷 모두 `renderStatus = previewReady`, `preview.kind = preset-applied-preview`, `xmpPreviewReadyAtMs != null`로 닫혔다.
  - helper status도 session 끝까지 `cameraState=ready`, `helperState=healthy`로 유지됐다.
  - direct metric은 `256x256` 상태에서 first shot 포함 5컷 모두 relaunch latency band에 머물렀다.
    - `preview-render-ready elapsedMs`: `3820`, `3816`, `3417`, `3517`, `3524`
    - `originalVisibleToPresetAppliedVisibleMs`: `3877`, `3855`, `3441`, `3606`, `3606`
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
- latest official gate reading from relaunch session:
  - `capture_preview_ready`: `6054ms`, `5949ms`, `5489ms`, `5635ms`, `5617ms`
  - `preview-render-ready elapsedMs`: `3820ms`, `3816ms`, `3417ms`, `3517ms`, `3524ms`
- current code logging guardrail:
  - latest approved hardware package는 아직 pre-direct-metric evidence로 남아 있다.
  - current worktree는 next session의 `capture_preview_ready` detail에
    `originalVisibleToPresetAppliedVisibleMs`, `firstVisibleAtMs`, `presetAppliedVisibleAtMs`
    가 직접 함께 남도록 보강됐고, targeted automated coverage도 통과했다.
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
 - 하지만 official gate는 여전히 실패했고, truthful close 비용도 아직 `3.6s ~ 3.7s`대에 남아 있다.
- 다음 주력 작업은 startup/debugging 반복이 아니라,
  booth-visible truthful close hot path를 더 줄일 후보를 다시 찾는 일이다.

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

지금 바로 다음 단계는 `4`다.

- latest approved rerun으로 first-shot overlap family는 current code에서 실제로 사라진 것이 확인됐다.
- 따라서 이제 startup/debugging 반복으로 돌아가지 말고,
  booth-visible truthful close hot path를 더 줄일 새 reduction candidate를 다시 찾는 순서가 맞다.
- 단, 다음 hardware rerun에서는
  `capture_preview_ready detail`에 `originalVisibleToPresetAppliedVisibleMs`가 실제로 찍히는지부터 먼저 확인한다.
- latest relaunch capture sessions
  `session_000000000018a88f5792a50534`,
  `session_000000000018a88fa09dcbdca0`,
  `session_000000000018a89441063a0c54`,
  `session_000000000018a895d248f89000`,
  `session_000000000018a8975f5f4f0b34`,
  `session_000000000018a8986df67c7e38`,
  `session_000000000018a89961df9c18a0`
  에서는 direct metric이 실제로 찍혔고,
  latest session 기준으로는 `256x256` 복귀 상태에서도 first-shot cold spike가 보이지 않아
  blocker가 steady-state truthful-close latency로 유지된다.
- 참고:
  - latest startup-only relaunch session `session_000000000018a88adfee94784c`는 `camera-ready`까지 정상 진입했다.
  - 이번 턴에서는 app re-entry가 existing active preset capture flow로 돌아올 때도
    reserve lane을 다시 warm state로 데우고,
    그 in-flight prime이 first capture 전에 먼저 settle되도록 보강했다.
  - latest evidence를 반영해,
    preview prime 자체를 `useEffect`보다 앞선 session/preset 전이 시점에도 즉시 시작하도록 보강했다.
  - 사용자가 요청한 대로 same-capture `fast-preview-raster` truthful close cap은
    `256x256`으로 유지하고, 더 낮은 해상도 실험은 중단한다.
  - 다음 판단은 hardware rerun에서 relaunch 이후 first shot의
    `preview-render-ready elapsedMs`와
    `originalVisibleToPresetAppliedVisibleMs`
    가 함께 내려오는지로 닫는다.
