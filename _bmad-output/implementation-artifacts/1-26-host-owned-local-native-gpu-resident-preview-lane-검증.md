# Story 1.26: host-owned local native/GPU resident preview lane 검증

Status: done

Latest product status: Story `1.26` has fresh approved-hardware `Go` evidence after `hardware-validation-run-1777442288984` / `session_000000000018aabe5833c11d8c`. The accepted route is explicitly labeled `engineMode=per-capture-cli`, not resident, and all 5 captures closed with raw-original full-preset route evidence inside the official 3s product gate.

Latest implementation decision: the product boundary now accepts an explicit per-capture full-preset route when it is honest about runtime mode, uses raw-original input, produces `truthProfile=original-full-preset`, and passes approved hardware. Partial native approximation and metadata-only `preset-applied-preview` remain comparison-only.

Correct Course Note: `2026-04-20` preview-track route decision에 따라, Story `1.10` old `resident first-visible` line은 closed `No-Go` baseline으로 고정하고, Story `1.26`이 다음 official reserve path를 소유한다. `2026-04-29` review follow-up 뒤 이 story의 official path는 거짓 resident label이 아니라, 실제 동작을 `per-capture-cli`로 정직하게 기록하는 raw-original full-preset preview route로 좁혀졌다.

## Current Role In This Worktree

- `2026-04-20` 기준 이 문서는 현재 preview-track의 active reserve path story다.
- current official release judgment는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`, 즉 `preset-applied visible <= 3000ms` 하나뿐이다.
- `sameCaptureFullScreenVisibleMs`와 first-visible 수치는 계속 남기되, reference / comparison / feel metric으로만 읽는다.
- Story `1.30`은 actual-primary-lane bounded `No-Go` evidence, Story `1.10`은 old line closed `No-Go` baseline, Story `1.31`은 unopened success-side default/rollback gate다.
- old line GPU/OpenCL comparison은 필요 시 side evidence로 남길 수 있지만, 이 story의 primary critical path는 아니다.

### Canonical Reading Order

- First read `docs/runbooks/story-1-26-agent-operating-guide.md` and follow its Agent Reading Budget. Do not read this full story file unless the latest concise sections conflict or the user explicitly asks for full history.
- 이 story만 단독으로 읽지 않는다.
- 먼저 `docs/README.md`, `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`, `docs/runbooks/current-preview-gpu-direction-20260419.md`, `docs/runbooks/preview-track-route-decision-20260418.md`를 읽고 해석한다.
- official `Go / No-Go` ownership은 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 소유한다.

### Validation Gate Reference

- Supporting evidence family:
  - approved booth hardware latency package
  - per-session seam log package
  - truthful `Preview Waiting -> Preview Ready` evidence
- Current hardware gate:
  - `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
  - product wording: `preset-applied visible <= 3000ms`
- Reference / comparison metrics:
  - `sameCaptureFullScreenVisibleMs`
  - first-visible product feel
- Official verdict owner:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Close policy:
  - automated pass만으로 닫지 않는다.
  - 승인 하드웨어 one-session package가 same-session correctness, truthful close, wrong-capture 0, cross-session leakage 0, official gate 판정을 함께 닫아야 한다.
  - darktable parity/fallback/final-export path는 유지하되, per-capture preview hot path owner로 되돌리지 않는다.

## Story

booth customer로서,
프리셋이 적용된 확인용 사진이 제품 기준 시간 안에 보여지길 원한다.
그래서 방금 찍은 사진이 보여도 실제 확인용 결과를 오래 기다리거나, truth가 느슨해지는 일을 겪지 않는다.

## Current Validation Questions

1. resident/long-lived full-preset engine이 per-capture `darktable-cli` fallback보다 official gate에 실제로 더 가까운가
2. display-sized preset-applied truthful artifact를 current customer contract 안에서 close owner로 유지할 수 있는가
3. same-session, same-capture correctness와 `Preview Waiting` truth를 유지한 채 process spawn, cold-start, queue jitter 비용을 줄일 수 있는가
4. native RAW approximation을 comparison-only로 둔 상태에서, 실제 preset engine 결과만 official truth로 승격할 수 있는가

## Current Implementation Direction

Next work is not darktable fallback tuning, and it is not more partial native approximation.

The selected path is an honest full-preset close owner: the route may be per-capture `darktable-cli` when it says so, but it must make a same-capture `preset-applied-preview` artifact from raw-original input and must not claim resident ownership.

Required route evidence:

- `binary=fast-preview-handoff`
- `source=fast-preview-handoff`
- `engineSource=host-owned-native`
- `inputSourceAsset=raw-original`
- `sourceAsset=preset-applied-preview`
- `truthOwner=display-sized-preset-applied`
- `truthProfile=original-full-preset`
- `engineMode=per-capture-cli`

Rejected as official truth:

- `inputSourceAsset=fast-preview-raster`
- `profile=operation-derived`
- self-labeled resident route backed by per-capture `darktable-cli`
- host-owned output that has not passed full-preset parity / eligibility checks

Implementation rule:

- If the renderer cannot preserve the full preset look for the active preset, that artifact must remain comparison evidence only and must not own `previewReady`.
- Native RAW approximation remains useful for comparison and diagnostics, but it is not the next product path.
- Per-capture darktable preview is allowed only when route evidence honestly says `engineMode=per-capture-cli` and approved hardware closes the official gate.

## Acceptance Criteria

1. reserve path는 `raw-original -> preset-applied-preview` full-preset route를 current booth-visible preview hot path의 주 경계로 사용해야 한다. per-capture `darktable-cli`를 쓰는 경우 runtime mode를 `per-capture-cli`로 정직하게 기록해야 한다.
2. reserve path가 booth에 표시하는 truthful close asset은 `display-sized preset-applied truthful artifact`여야 한다. `previewReady`, `preview.readyAtMs`, 관련 readiness update는 이 artifact만 소유할 수 있다.
3. first-visible 또는 intermediate image가 더 먼저 보이더라도 booth는 truthful `Preview Waiting`을 유지해야 하며, preset-applied truthful artifact가 실제로 닫히기 전까지 false-ready를 만들면 안 된다.
4. reserve path는 same-session, same-capture correctness, same-slot continuity, wrong-capture discard, cross-session leakage 0 규칙을 유지해야 한다.
5. approved booth hardware 검증에서는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`만 official pass condition으로 읽어야 하며, `sameCaptureFullScreenVisibleMs`와 first-visible feel은 reference/comparison metric으로만 남겨야 한다.
6. reserve path가 official gate를 못 닫거나 correctness guardrail을 깨면 story는 `review` 또는 동등 route-hold 상태에 남고, Story `1.31`은 이 결과만으로 자동 재오픈되면 안 된다.

## Tasks / Subtasks

- [x] reserve topology boundary를 고정한다. (AC: 1, 2, 3, 4, 5)
  - [x] host-owned local native/GPU resident full-screen lane의 owner boundary를 문서와 코드에서 한 곳으로 고정한다.
  - [x] `display-sized preset-applied truthful artifact`가 어떤 시점과 경로에서 생성되고 `previewReady`를 소유하는지 명시한다.
  - [x] darktable path가 parity reference, fallback, final/export truth로 남는 경계를 분리한다.

- [x] truthful close ownership과 booth contract를 유지한다. (AC: 2, 3, 4)
  - [x] first-visible / intermediate asset이 있어도 `Preview Waiting`과 later truthful close ownership이 느슨해지지 않게 한다.
  - [x] same-slot continuity, wrong-capture discard, cross-session isolation guardrail을 reserve path에 맞게 다시 잠근다.
  - [x] `previewReady`가 non-truthful asset에서 먼저 올라가지 않도록 회귀를 막는다.

- [x] per-session instrumentation과 gate readout을 유지한다. (AC: 4, 5)
  - [x] one-session package만으로 official gate와 reference metrics를 함께 읽을 수 있게 seam logging을 유지하거나 보강한다.
  - [x] request-level correlation 키가 preview hot path와 truthful close까지 이어지도록 유지한다.
  - [x] ledger readout에 필요한 evidence path 형식을 미리 고정한다.

- [x] host-owned original/full-preset truthful artifact path를 구현한다. (AC: 1, 2, 3, 4, 5)
  - [x] `profile=operation-derived`와 incomplete host-owned handoff가 `previewReady`를 소유하지 못하게 막는다.
  - [x] RAW 저장 직후 host-owned `raw-original -> preset-applied-preview` handoff를 시작한다.
  - [x] approved hardware `.CR2`를 native decode source로 통과시켜 `inputSourceAsset=raw-original` evidence를 만든다.
  - [x] active preset `look2` native RAW approximation이 full-preset truth로 승격되지 못하게 막는다.
  - [x] native approximation route를 product path에서 제외하고 comparison-only evidence로 유지한다.
  - [x] per-capture darktable-compatible full-preset route를 resident로 오표기하지 않게 한다.
  - [x] full preset 결과를 만드는 engine output에만 `truthProfile=original-full-preset` route evidence를 부여한다.
  - [x] metadata-only / filename-only false Go를 hardware validation과 ingest path에서 막는다.
  - [x] 승인 하드웨어에서 official gate를 재검증한다.


- [x] hardware validation package를 수집한다. (AC: 5, 6)
  - [x] 승인 하드웨어 one-session package를 수집한다.
  - [x] official gate, correctness, truth ownership을 ledger에 기록한다.
  - [x] 결과에 따라 `Go` 또는 bounded `No-Go`를 선언한다.

### Review Findings

- [x] [Review][Decision] Story 1.26 `Go` 판정이 문서상 reserve path와 맞지 않음 — resolved: original host-owned reserve path boundary를 유지하고, `session_000000000018a93c85f1238a00`는 comparison evidence로 낮춘다.
- [x] [Review][Patch] fast-preview XMP가 look-affecting operation을 제거한 결과를 `preset-applied-preview` truth로 통과시킬 수 있음 [src-tauri/src/render/mod.rs:44]
- [x] [Review][Patch] preview warm-up이 실패하거나 timeout이어도 validation step이 `passed`로 기록될 수 있음 [src-tauri/src/automation/hardware_validation.rs:444]
- [x] [Review][Patch] official gate 계산이 timestamp 역전 evidence를 `0ms`로 false-pass시킬 수 있음 [src-tauri/src/automation/hardware_validation.rs:636]

- [x] [Review][Patch] late fast-preview recovery가 non-truthful 이벤트에서 즉시 반환해, timeout 안에 뒤따라오는 `preset-applied-preview` truthful close를 놓칠 수 있음 [src-tauri/src/capture/sidecar_client.rs:531]
- [x] [Review][Patch] fast-preview XMP cache 파일명이 sanitize된 경로 조각만 써서 서로 다른 프리셋이 같은 cache path를 공유하고 wrong-look preview를 만들 수 있음 [src-tauri/src/render/mod.rs:1486]
- [x] [Review][Patch] fast-preview XMP trimming에서 모든 history block이 제거되면 빈 history에 `darktable:history_end="0"`이 남아 잘못된 XMP를 만들 수 있음 [src-tauri/src/render/mod.rs:1628]
- [x] [Review][Patch] fast-preview XMP parser가 self-closing `<rdf:li .../>`만 처리해, 유효한 `<rdf:li>...</rdf:li>` preset XMP에서는 trimming을 조용히 포기함 [src-tauri/src/render/mod.rs:1587]
- [x] [Review][Patch] hardware validation runner가 `can_capture=true`인 warning 상태를 준비 완료로 인정하지 않아, 제품상 촬영 가능한 세션을 readiness timeout으로 실패시킬 수 있음 [src-tauri/src/automation/hardware_validation.rs:1157]

- [x] [Review][Patch] timeout evidence 복구보다 latest helper error 분기가 먼저 평가되어, unrelated helper error 한 건만 끼어도 무저장 `capture-timeout` 세션이 계속 `phone-required`에 남을 수 있음 [src-tauri/src/capture/normalized_state.rs:926]
- [x] [Review][Patch] stale `capture-in-flight` helper 재시작을 no-capture 세션으로 제한해, 이전 캡처가 있는 세션의 다음 촬영 stall은 readiness poll에서 자동 복구되지 않음 [src-tauri/src/capture/normalized_state.rs:989]

- [x] [Review][Patch] `capture-round-trip` 실패 진단 파일 기록이 실패하면 세션은 이미 `phone-required`로 바뀐 뒤 요청 경로만 오류로 끝나 partially-applied failure 상태를 남길 수 있음 [src-tauri/src/capture/normalized_state.rs:799]
- [x] [Review][Patch] fresh helper ready 이후에도 이전 helper error 이벤트를 그대로 읽어 retryable recovery 판단에 써서 stale failure context로 잘못 unblock 또는 block할 수 있음 [src-tauri/src/capture/normalized_state.rs:905]
- [x] [Review][Patch] timeout 기반 무저장 복구 경로가 `requestId` 없이 evidence를 기록해 request-level correlation 요구를 만족하지 못함 [src-tauri/src/capture/normalized_state.rs:912]
- [x] [Review][Patch] truthful promotion 후 emitted readiness update가 resolved truthful kind 대신 원래 handoff kind를 실어 보내 close owner를 잘못 알릴 수 있음 [src-tauri/src/capture/ingest_pipeline.rs:168]

- [x] [Review][Patch] 손상됐지만 현재 복구 앵커와 매칭된 failure evidence를 무조건 `capture-timeout`으로 간주해 실제 다른 실패도 `capture-ready`로 잘못 복구할 수 있음 [src-tauri/src/capture/normalized_state.rs:1673]
- [x] [Review][Patch] early non-truthful preview 뒤 같은 canonical path로 truthful close가 올라오면 UI upgrade 이벤트를 다시 내보내지 않아 stale preview가 유지될 수 있음 [src-tauri/src/capture/normalized_state.rs:466]

- [x] [Review][Patch] 손상된 `latest-capture-round-trip-failure.json`이 이전 시도의 stale 증거여도 현재 `phone-required` 세션을 잘못 `capture-ready`로 복구할 수 있음 [src-tauri/src/capture/normalized_state.rs:1667]
- [x] [Review][Patch] truthful fast preview가 끝내 오지 않는 경로에서는 late fast preview recovery가 전체 timeout까지 대기해 first-visible 표시를 불필요하게 늦출 수 있음 [src-tauri/src/capture/sidecar_client.rs:526]

- [x] [Review][Patch] `capture-in-flight` helper restart가 45초가 아니라 약 5초 stale 상태에서 readiness poll로 먼저 발동됨 [src-tauri/src/capture/normalized_state.rs:927]
- [x] [Review][Patch] 저장된 capture가 없는 `phone-required` 세션이 processed request evidence만으로 광범위하게 `capture-ready`로 복구될 수 있음 [src-tauri/src/capture/normalized_state.rs:891]
- [x] [Review][Patch] `fast-preview-ready` 이벤트가 파일보다 먼저 기록되면 첫 매칭 메시지에서 대기를 끝내 metadata handoff를 놓칠 수 있음 [src-tauri/src/capture/sidecar_client.rs:518]
- [x] [Review][Patch] 같은 request에서 더 늦게 도착한 non-truthful fast preview가 first-visible 시각과 reference gate 계측을 덮어쓸 수 있음 [src-tauri/src/capture/normalized_state.rs:1611]

- [x] [Review][Patch] resident full-preset route가 실제로는 캡처마다 `darktable-cli`를 새로 실행하면서 `engineMode=resident-full-preset` evidence를 붙일 수 있음 [src-tauri/src/render/mod.rs:514]
- [x] [Review][Patch] `preset-applied-preview` kind 또는 파일명만으로 raw-original/full-preset route evidence를 생성해 실제 renderer proof 없이 `previewReady`를 닫을 수 있음 [src-tauri/src/capture/ingest_pipeline.rs:2386]
- [x] [Review][Patch] hardware validation gate가 self-labeled `engineMode=resident-full-preset` 문자열을 신뢰해 per-capture darktable 실행을 official Go로 통과시킬 수 있음 [src-tauri/src/automation/hardware_validation.rs:1039]
- [x] [Review][Patch] Story checklist가 latest Go summary와 충돌해 resident engine 구현/검증 subtasks를 아직 미완료로 표시함 [_bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md:113]
- [x] [Review][Patch] `capture-timeout` 자동 복구가 같은 readiness poll 안의 timing sync에 의해 건너뛰어질 수 있음 [src-tauri/src/capture/normalized_state.rs:927]
- [x] [Review][Patch] truthful `preset-applied-preview`가 더 늦게 도착한 non-truthful metadata에 의해 다시 강등될 수 있음 [src-tauri/src/capture/ingest_pipeline.rs:1649]
- [x] [Review][Patch] 손상되거나 부분 기록된 `latest-capture-round-trip-failure.json`이 있으면 `capture-timeout` 복구 근거가 사라져 세션이 계속 `phone-required`에 남을 수 있음 [src-tauri/src/capture/normalized_state.rs:1667]
- [x] [Review][Patch] helper status의 `observed_at`이 파싱 불가이면 stale `capture-in-flight` 재시작이 영구히 막힐 수 있음 [src-tauri/src/capture/normalized_state.rs:1004]
- [x] [Review][Patch] fast-preview XMP trimming이 `lens`를 RAW-only operation으로 제거해, 렌즈 보정이 필요한 프리셋에서 preview가 최종 룩과 달라질 수 있음 [src-tauri/src/render/mod.rs:47]
- [x] [Review][Patch] fast-preview XMP cache writer가 process-id 기반 임시 파일과 remove-then-rename을 사용해, 첫 동시 preview render에서 cache 파일을 서로 지우거나 in-flight render의 XMP를 순간적으로 없앨 수 있음 [src-tauri/src/render/mod.rs:1558]
- [x] [Review][Patch] 렌더 실패 fallback이 non-truthful 기존 미리보기만으로도 `previewReady`를 만들 수 있음 [src-tauri/src/capture/ingest_pipeline.rs:1102]
- [x] [Review][Patch] hardware validation runner가 `previewReady` 상태만 보고 truthful `preset-applied-preview` close와 official gate를 검증하지 않음 [src-tauri/src/automation/hardware_validation.rs:521]

- [x] [Review][Patch] fast-preview XMP trimming이 `highlights`와 `cacorrectrgb`를 제거한 결과를 truthful preset look으로 통과시킬 수 있음 [src-tauri/src/render/mod.rs:44]
- [ ] [Review][Patch] main preview path가 여전히 per-capture `darktable-cli`와 Windows high priority scheduling에 의존해 host-owned reserve path boundary를 만족하지 못함 [src-tauri/src/render/mod.rs:2170]
- [x] [Review][Patch] preview runtime warm-up 실패가 같은 세션/프리셋에서 성공처럼 cache되어 이후 재시도되지 않음 [src/session-domain/state/session-provider.tsx:779]
- [x] [Review][Patch] debug helper launch가 현재 C# source보다 stale local executable을 먼저 선택할 수 있음 [src-tauri/src/capture/helper_supervisor.rs:221]
- [x] [Review][Patch] Canon SDK connect timeout 뒤 늦게 끝난 이전 connect attempt가 새 attempt 상태를 오염시킬 수 있음 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:702]
- [x] [Review][Patch] duplicate fast-preview event에서 speculative source staging 파일이 early return으로 누적될 수 있음 [src-tauri/src/capture/ingest_pipeline.rs:288]
- [x] [Review][Patch] hardware validation readiness timeout이 runtime reconnect/recovery budget보다 짧아 정상 복구 경로를 실패로 기록할 수 있음 [src-tauri/src/automation/hardware_validation.rs:1244]
- [x] [Review][Patch] branch diff에 local tool config, temporary images, and test build artifacts가 포함되어 release review와 재현성을 흐림 [.codex/config.toml:1]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 old line 미세 조정이 아니다.
- 이번 스토리는 `darktable-only hot path tuning`이나 partial native approximation이 아니라, 실제 full preset 결과를 만드는 resident/long-lived engine path 검증이다.
- 제품 약속은 그대로다. 고객이 먼저 같은 컷을 볼 수는 있어도, official 성공은 `preset-applied visible <= 3000ms`를 닫는지로만 읽는다.
- Story `1.31`은 이번 스토리 성공 전까지 열지 않는다.

### 왜 이 스토리가 열렸는가

- Story `1.30` actual-primary-lane은 repeated hardware rerun 끝에 bounded `No-Go`로 닫혔다.
- Story `1.10` old `resident first-visible` line도 latest approved hardware session에서 baseline evidence package는 다시 닫았지만 official gate는 `8972ms`, `7942ms`, `7967ms`로 실패했다.
- 따라서 preview-track은 old lane comparison evidence를 유지하되, 실제 다음 구현 경로는 새 reserve topology로 넘어가야 한다.

### 구현 순서

1. reserve topology owner boundary를 고정한다.
2. truthful close owner를 `display-sized preset-applied truthful artifact`로 다시 닫는다.
3. instrumentation과 same-capture correctness guardrail을 맞춘다.
4. 승인 하드웨어 one-session package를 수집한다.
5. ledger `Go / No-Go`로 route를 판정한다.

### 구현 가드레일

- official gate를 `sameCaptureFullScreenVisibleMs`로 바꾸지 말 것.
- first-visible source나 intermediate asset을 truth owner로 승격하지 말 것.
- old line historical better run을 release proof처럼 해석하지 말 것.
- darktable path를 per-capture preview hot path owner로 조용히 되돌리지 말 것.
- story note만으로 route success를 선언하지 말 것.

### 아키텍처 준수사항

- host가 normalized truth owner라는 구조는 유지한다.
- session truth는 계속 `session.json`과 session-scoped filesystem root가 소유한다.
- darktable는 parity reference, fallback, final/export truth로 남고, booth-visible preview gate를 닫는 primary hot path와는 분리한다.
- front-end는 raw helper status나 low-level renderer status를 직접 해석하지 않고 host-normalized truth만 소비해야 한다.

### 프로젝트 구조 요구사항

- 우선 검토 후보 경로:
  - `src-tauri/src/render/`
  - `src-tauri/src/capture/`
  - `src-tauri/src/commands/`
  - `src-tauri/src/timing/`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
  - `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
- 새 reserve path는 기존 preview truth 계약을 깨지 않는 범위에서 추가/분리되어야 한다.

### 테스트 요구사항

- reserve path hot path가 truthful close ownership과 분리돼도 `Preview Waiting` truth가 유지된다.
- same-session, same-capture correctness, wrong-capture discard, cross-session leakage 0가 유지된다.
- official gate metric과 reference metrics가 한 session diagnostics path에서 함께 읽힌다.
- reserve path fail 시 bounded fallback이 동작하고 false-ready가 생기지 않는다.

### References

- [Source: docs/runbooks/story-1-26-reserve-path-opening-20260420.md]
- [Source: docs/runbooks/current-preview-gpu-direction-20260419.md]
- [Source: docs/runbooks/preview-track-route-decision-20260418.md]
- [Source: docs/release-baseline.md]
- [Source: docs/preview-architecture-history-and-agent-guide.md]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.26: host-owned local native/GPU resident reserve path와 display-sized preset-applied truthful artifact 검증]
- [Source: _bmad-output/planning-artifacts/prd.md#Current Preview Release Interpretation]
- [Source: _bmad-output/planning-artifacts/architecture.md#Gap Analysis Results]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]

## Creation Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-20 10:58:00 +09:00 - Story `1.10` latest approved hardware rerun result, preview route docs, PRD/epics/architecture interpretation, and hardware ledger ownership을 함께 읽고 Story `1.26` reserve path opening scope를 정리했다.
- 2026-04-20 11:37:12 +09:00 - reserve path truthful close owner를 `preset-applied-preview` 계약으로 고정하고, `src-tauri/src/capture/ingest_pipeline.rs`, `docs/contracts/render-worker.md`, `docs/contracts/session-manifest.md`, `src-tauri/tests/capture_readiness.rs`를 함께 갱신했다.
- 2026-04-20 11:37:12 +09:00 - `cargo test -- --test-threads=1`, `pnpm test:run`, `pnpm lint`를 실행해 reserve path truthful close 회귀와 기존 booth 흐름이 현재 worktree 기준 모두 통과함을 확인했다.
- 2026-04-20 11:54:11 +09:00 - 승인 하드웨어 최신 세션 `session_000000000018a7f0faf87fd164`를 읽어 one-session package를 수집했다. official gate는 실패했고, field evidence의 actual close owner는 여전히 `darktable-cli + raw-original`로 남아 있었다.
- 2026-04-20 12:46:22 +09:00 - owner attribution 수정 뒤 승인 하드웨어 최신 세션 `session_000000000018a7f3c5b88c698c`를 다시 읽었다. 2장~4장은 field evidence에서 `preset-applied-preview` close owner가 관찰됐지만 official gate는 여전히 `8616ms`, `7712ms`, `8165ms`, `7643ms`로 실패했고 1장은 `raw-original` close로 남았다.
- 2026-04-23 14:46:50 +09:00 - latest app session `session_000000000018a8e59c3f873ffc`를 다시 읽었다. startup은 `camera-ready`까지 정상 진입했고 5컷 모두 `previewReady`/`preset-applied-preview`로 닫혔지만, 1장은 `preview-render-ready elapsedMs=15975`, `originalVisibleToPresetAppliedVisibleMs=16066`으로 크게 튀었고 2장~5장은 `3436`, `3360`, `3277`, `3355`ms band에 모였다.
- 2026-04-23 14:46:50 +09:00 - first-shot cold spike를 darktable warm-up miss로 좁게 해석하고, `src-tauri/src/render/mod.rs`의 preview warm-up source를 tiny PNG에서 JPEG raster로 바꿨다. `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`로 관련 단위 테스트 2건을 다시 통과시켰다.
- 2026-04-23 14:59:20 +09:00 - latest app session `session_000000000018a8e6cb585230d4`를 다시 읽었다. startup은 `camera-ready`까지 정상 진입했고 5컷 모두 `previewReady`/`preset-applied-preview`로 닫혔으며, `preview-render-ready elapsedMs`는 `3415`, `3314`, `3320`, `3414`, `3314`, `originalVisibleToPresetAppliedVisibleMs`는 `3441`, `3366`, `3360`, `3439`, `3356`으로 first-shot cold spike가 사라진 상태로 모였다.
- 2026-04-23 15:03:10 +09:00 - hardware validation runner를 한 번 실행했고 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e716e9987b48`로 닫혔다. latest session direct metric은 `preview-render-ready elapsedMs=3514`, `3516`, `3414`, `3514`, `3616`, `originalVisibleToPresetAppliedVisibleMs=3606`, `3598`, `3439`, `3600`, `3679`로 first-shot spike 없이 steady-state band만 남았다.
- 2026-04-23 15:09:16 +09:00 - preview truthful-close path에서 OpenCL startup cost를 빼기 위해 current worktree가 `--disable-opencl`을 실제 invocation에 싣도록 보강한 뒤, hardware validation runner를 다시 한 번 실행했다. `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e7702849122c`였고 direct metric은 `preview-render-ready elapsedMs=3517`, `3414`, `3313`, `3414`, `3415`, `originalVisibleToPresetAppliedVisibleMs=3612`, `3437`, `3356`, `3439`, `3443`로 first-shot extreme spike 없이 조금 더 낮은 steady-state band에 모였다.
- 2026-04-23 15:30:02 +09:00 - preview truthful-close path가 preview 전용 `library.db` startup cost를 매번 물지 않도록 current worktree를 `--library :memory:`로 보강한 뒤, hardware validation runner를 다시 한 번 실행했다. `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e892447836f8`였고 direct metric은 `preview-render-ready elapsedMs=3417`, `3315`, `3314`, `3415`, `3418`, `originalVisibleToPresetAppliedVisibleMs=3441`, `3366`, `3354`, `3439`, `3443`로 first-shot까지 later-shot band 안에 더 안정적으로 들어왔다.
- 2026-04-23 15:39:58 +09:00 - same-volume speculative source copy cost를 줄이기 위해 current worktree가 request-scoped preview source를 hard link 우선으로 stage하도록 보강한 뒤, hardware validation runner를 다시 한 번 실행했다. `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8e91cef5631a8`였고 direct metric은 `preview-render-ready elapsedMs=3414`, `3315`, `3517`, `3421`, `3320`, `originalVisibleToPresetAppliedVisibleMs=3440`, `3356`, `3599`, `3440`, `3356`으로 latest gate miss는 계속 steady-state band에 남았다.
- 2026-04-23 21:57:39 +09:00 - same-capture truthful-close raster를 더 줄이면 steady-state gate가 닫히는지 확인하기 위해 experimental `192x192` cap으로 hardware validation runner를 다시 한 번 실행했다. `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8fdb7a8e88590`였고 actual invocation에도 `--disable-opencl`, `--library :memory:`, `--width 192`, `--height 192`가 남았다. 하지만 direct metric은 `preview-render-ready elapsedMs=3515`, `3416`, `3515`, `3515`, `3415`, `originalVisibleToPresetAppliedVisibleMs=3523`, `3434`, `3599`, `3602`, `3437`로 오히려 받아들일 만한 개선을 만들지 못해 이 실험은 reject했고 current worktree는 다시 `256x256` cap으로 롤백했다.
- 2026-04-23 22:13:27 +09:00 - preview truthful-close path가 fast-preview-raster 입력에서 raw-only darktable history 일부를 덜어낸 cached XMP를 실제로 쓰도록 current worktree를 보강한 뒤, hardware validation runner를 다시 한 번 실행했다. `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a8fe95ea36f8f4`였고 latest invocation args에는 `C:/Users/KimYS/Pictures/dabi_shoot/.boothy-darktable/preview/xmp-cache/preset-new-draft-2-2026-04-10-look2-fast-preview.xmp`가 실제로 남았다. direct metric은 `preview-render-ready elapsedMs=3317`, `3315`, `3314`, `3215`, `3316`, `originalVisibleToPresetAppliedVisibleMs=3358`, `3368`, `3357`, `3284`, `3361`로 accepted `256x256` band를 조금 더 낮췄지만 official gate `<= 3000ms`는 아직 닫지 못했다.
- 2026-04-24 07:58:56 +09:00 - latest app session `session_000000000018a8fe95ea36f8f4`를 다시 읽어 `--disable-opencl`이 `--core` 앞에 있어 darktable core option으로 적용되지 않을 수 있음을 확인했다. current worktree는 preview invocation 인자를 `--width 256 --height 256 --core --disable-opencl --configdir ... --library :memory:` 순서로 고쳤고, `cargo test preview_invocation_uses_display_sized_render_arguments --manifest-path src-tauri/Cargo.toml`로 red/green을 확인했다. 이어 hardware validation runner를 한 번 실행했고 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a91e89791d5370`였다. direct metric은 `preview-render-ready elapsedMs=2916`, `2913`, `3019`, `3115`, `2913`, `originalVisibleToPresetAppliedVisibleMs=2953`, `2960`, `3039`, `3197`, `2953`으로 3/5컷이 official gate 안에 들어왔지만 2컷 tail miss 때문에 Story `1.26`은 아직 `No-Go`다.
- 2026-04-24 09:49:29 +09:00 - requested three-run hardware validation package를 실행했다. 1회차는 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a92487a7070480`였고 `originalVisibleToPresetAppliedVisibleMs`는 `3035`, `3037`, `2952`, `2957`, `2953`ms, 평균 `2986.8ms` / `2.987s`였다. 이는 relaxed product threshold `<= 3.2s` 기준으로는 latency Go다. 다만 2회차와 3회차는 모두 `capture-readiness-timeout`으로 촬영 샘플을 만들지 못했으므로, story는 `review`에 남기고 latest package는 validation-held로 기록한다.
- 2026-04-24 10:00:10 +09:00 - latest failed sessions `session_000000000018a92491e8b75984`, `session_000000000018a924971612f514`를 다시 읽어 failure snapshot 시점에는 helper status/startup evidence가 없고 이후 한 세션에서만 helper가 늦게 `camera-ready`를 쓴 점을 확인했다. current worktree는 hardware validation runner가 app command path를 우회할 때 missing helper status가 1초 이상 지속되면 helper bootstrap을 직접 요청하도록 보강했다. `cargo test --manifest-path src-tauri/Cargo.toml --test hardware_validation_runner -- --test-threads=1` 통과 뒤 요청 커맨드를 한 번 실행했고 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a925271b1710a0`로 닫혔다. direct metric은 `originalVisibleToPresetAppliedVisibleMs=3037`, `2952`, `2974`, `3279`, `2954`ms, 평균 `3039.2ms`였다. 이전 readiness-timeout family는 이번 실행에서 재발하지 않았지만 2/5컷 tail miss 때문에 official gate는 아직 `No-Go`다.
- 2026-04-24 10:19:51 +09:00 - latest runner summary에서 `Kim4821` 프롬프트가 `Kim4821 0000`으로 잘못 닫히던 것을 확인하고, compact name + last-four 입력을 `Kim 4821`로 분리하도록 보강했다. `cargo test --test hardware_validation_runner -- --test-threads=1` 통과 뒤 요청 커맨드를 한 번 실행했고 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a92639f9a96a6c`, `boothAlias=Kim 4821`로 닫혔다. direct metric은 `originalVisibleToPresetAppliedVisibleMs=3200`, `2956`, `2955`, `2958`, `3276`ms, 평균 `3069.0ms`였다. 고객 식별자 문제는 닫혔지만 2/5컷 tail miss 때문에 Story `1.26`은 계속 official `No-Go`다.
- 2026-04-24 10:32:24 +09:00 - latest runner session `session_000000000018a92639f9a96a6c`를 다시 읽어 remaining blocker가 render tail임을 확인하고, fast-preview JPEG 입력에는 불필요한 RAW correction modules `lens`, `highlights`, `cacorrectrgb`를 cached preview XMP에서 더 제거하도록 보강했다. `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_xmp_trim_removes_raw_only_operations_from_history_and_iop_order -- --nocapture`는 red/green으로 통과했고, 요청 커맨드를 한 번 실행했다. latest run은 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a926e98958c25c`, `boothAlias=Kim 4821`로 닫혔다. direct metric은 `originalVisibleToPresetAppliedVisibleMs=3039`, `2955`, `3034`, `3032`, `2956`ms, 평균 `3003.2ms`였다. 최대 tail은 줄었지만 3/5컷이 official gate를 32~39ms 넘어서 Story `1.26`은 아직 official `No-Go`다.
- 2026-04-24 11:36:44 +09:00 - latest app/hardware-validation session `session_000000000018a9292e867e1a68`를 먼저 읽어 duplicate builtin/default 및 repeated builtin-auto trimming 뒤에도 `3035ms`, `3036ms`, `3039ms` tail miss가 남고 cached XMP `iop_order_list`에는 history에서 제거된 default pipeline 항목이 계속 남는 점을 확인했다. current worktree는 fast-preview cached XMP의 `iop_order_list`를 실제 유지된 preview history operation/priority만 남기도록 보강했고, `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_xmp_trim -- --nocapture`로 red/green을 확인했다. 요청 커맨드를 한 번 실행했고 `status=passed`, `capturesPassed=5/5`, `sessionId=session_000000000018a92a6c02e7f2d4`, `boothAlias=Kim 4821`로 닫혔다. direct metric은 `originalVisibleToPresetAppliedVisibleMs=2956`, `2951`, `2961`, `2954`, `2960`ms로 5/5 모두 official gate 안에 들어와 Story `1.26` hardware ledger 판정은 `Go`다.
- 2026-04-24 13:59:26 +09:00 - code review patch 뒤 요청 커맨드 `hardware-validation-runner.ps1 -Prompt "Kim4821"`를 실행했다. 결과는 `status=failed`, `capturesPassed=0/5`, `sessionId=session_000000000018a9323a28789b40`, `boothAlias=Kim 4821`, failure code `preview-truth-gate-failed`였다. session manifest와 timing log상 preview ownership은 `preset-applied-preview`로 truthful했지만 capture 1의 `originalVisibleToPresetAppliedVisibleMs=3196ms`가 official `<= 3000ms` gate를 196ms 넘어서 Story `1.26`은 `review / No-Go`로 유지한다.
- 2026-04-24 14:22:18 +09:00 - 요청 커맨드 `hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다. 결과는 `status=failed`, `capturesPassed=0/5`, `sessionId=session_000000000018a9337745615574`, `boothAlias=Kim 4821`, failure code `preview-truth-gate-failed`였다. session manifest와 timing log상 preview ownership은 `preset-applied-preview`로 truthful했지만 capture 1의 `originalVisibleToPresetAppliedVisibleMs=13104ms`가 official `<= 3000ms` gate를 10104ms 넘었다. helper/camera readiness는 `camera-ready`/`healthy`였고, blocker는 장비 준비가 아니라 current-code truthful close latency로 유지한다.
- 2026-04-24 14:49:32 +09:00 - 요청 커맨드 `hardware-validation-runner.ps1 -Prompt "Kim4821"`를 다시 실행했다. 결과는 `status=failed`, `capturesPassed=0/5`, `sessionId=session_000000000018a934f66a92fe80`, `boothAlias=Kim 4821`, failure code `preview-truth-gate-failed`였다. session manifest와 timing log상 preview ownership은 `preset-applied-preview`로 truthful했고 helper/camera readiness도 `camera-ready`/`healthy`였지만, capture 1의 `originalVisibleToPresetAppliedVisibleMs=3037ms`가 official `<= 3000ms` gate를 37ms 넘었다. Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 15:26:44 +09:00 - latest app/runner 로그를 다시 읽고 current-code latency tail을 줄이기 위해 fast-preview XMP에서 `lens`와 `hazeremoval`까지 제외하고, preview render cap을 `192x192`로 낮추고, preview process polling을 `20ms`로 줄였다. best patched run `session_000000000018a936ed27302174`는 `2983`, `2889`, `2965`ms로 3/5컷을 통과했지만 capture 4가 `3051ms`로 실패했다. latest requested script rerun `session_000000000018a936fcad8c042c`는 capture 1이 `2955ms`, capture 2가 `3118ms`로 실패해 Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 16:11:50 +09:00 - latest app/runner 로그를 다시 읽어 hardware validation runner가 app command path와 달리 preview runtime warm-up을 기록하지 않는 점과 first-shot `14475ms` cold spike를 확인했다. current worktree는 runner가 preset 선택 뒤 preview runtime warm-up을 기다리고 기록하도록 보강했고, preview polling을 더 촘촘하게 조정했으며, empirical cap 비교 뒤 `224x224` truthful-close cap을 유지했다. best run in this pass `session_000000000018a93925842ee7b8`는 `2883`, `2869`, `2940`ms로 3/5컷을 통과했지만 capture 4가 `3306ms`로 실패했다. latest requested script rerun `session_000000000018a9397421a5ad30`은 capture 1 `3107ms`로 실패해 Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 16:27:20 +09:00 - latest preview stderr 로그에서 warm-up source JPEG가 `Invalid JPEG file structure`로 실패하던 것을 확인하고, current worktree의 built-in warm-up JPEG를 decodable JPEG로 교체했다. `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`와 `cargo test --manifest-path src-tauri/Cargo.toml request_capture_does_not_reprime_preview_warmup_while_camera_save_is_in_flight -- --nocapture`를 통과시킨 뒤 요청 스크립트를 실행했다. latest requested rerun `session_000000000018a93a4b32aba8c8`은 새 warm-up stderr를 만들지 않았고 capture 1은 `2881ms`로 통과했지만 capture 2가 `3035ms`로 실패해 Story `1.26`은 계속 `review / No-Go`다. run summary는 `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777015622619\run-summary.json`이다.
- 2026-04-24 16:48:00 +09:00 - latest app/runner 로그와 `preview-latency-next-steps-checklist-20260422.md`를 다시 읽어, current worktree가 사용자 지시와 previous positive package의 approved cap인 `256x256`이 아니라 `224x224`로 내려가 있음을 확인했다. `fast_preview_raster_uses_gate_safe_truthful_close_cap`을 먼저 `256` 기대값으로 바꿔 RED를 확인한 뒤 cap을 `256x256`으로 복구했다. 같은 턴에서 `hazeremoval` 유지 가설은 장비 실행에서 첫 컷 `3147ms`로 더 나빠져 reject하고, latency를 줄였던 XMP trimming은 유지했다. `cargo test --manifest-path src-tauri/Cargo.toml fast_preview -- --nocapture`는 통과했다. 최종 요청 스크립트 rerun `session_000000000018a93b5fa88505d4`는 truthful `preset-applied-preview`로 닫혔지만 capture 1 `originalVisibleToPresetAppliedVisibleMs=3056ms`로 official gate를 `56ms` 넘었다. 같은 턴의 best run `session_000000000018a93b053f7dbab8`은 `2883`, `2965`, `2893`, `2933`ms로 4/5까지 통과했지만 capture 5가 `3173ms`로 실패했다. Story `1.26`은 계속 `review / No-Go`다. final run summary는 `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777016810008\run-summary.json`이다.
- 2026-04-24 17:08:50 +09:00 - latest no-priority runner `session_000000000018a93c5bc6cceaa0`가 3/5컷을 통과한 뒤 capture 4 `originalVisibleToPresetAppliedVisibleMs=3088ms`로 실패해 remaining blocker가 darktable process tail jitter임을 확인했다. current worktree는 warm-up JPEG를 actual `256x256` preview lane과 맞추고 preview darktable process에만 Windows above-normal scheduling priority를 적용했다. `cargo test --manifest-path src-tauri/Cargo.toml preview_renderer_warmup_source -- --nocapture`와 `cargo test --manifest-path src-tauri/Cargo.toml preview_darktable_process_gets_latency_priority_on_windows -- --nocapture`를 통과한 뒤 요청 스크립트를 다시 실행했다. latest requested rerun `session_000000000018a93c85f1238a00`, run `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777018073947\run-summary.json`는 `status=passed`, `capturesPassed=5/5`로 닫혔다. direct metric은 `preview-render-ready elapsedMs=2801`, `2806`, `2810`, `2869`, `2789`, `originalVisibleToPresetAppliedVisibleMs=2819`, `2835`, `2861`, `2884`, `2831`로 전 컷 official gate 안이다. Story `1.26`은 ledger 기준 `Go`다.
- 2026-04-27 12:05:00 +09:00 - code review 뒤 false `Go` 차단 follow-up을 적용했다. fast-preview XMP는 `highlights`와 `cacorrectrgb`를 더 이상 제거하지 않고, preview darktable process scheduling priority는 official close 근거가 되지 않도록 제거했다. warm-up 실패 retry, debug helper source 우선 실행, Canon connect generation fence, duplicate speculative source cleanup, validation readiness budget, release diff artifact cleanup을 함께 보강했다. `cargo test --manifest-path src-tauri/Cargo.toml --tests -- --test-threads=1`, `pnpm test:run`, `pnpm lint`는 통과했다. Canon helper dotnet test는 local Canon SDK source가 없어 실행 전 `Canon SDK source not found`로 막혔다. Story `1.26`은 hardware ledger `Go`가 아니므로 계속 `in-progress / No-Go`다.
- 2026-04-27 12:37:52 +09:00 - 요청 커맨드를 먼저 실행한 초기 run `hardware-validation-run-1777260672018` / `session_000000000018aa192a350d074c`는 helper/camera readiness와 preview warm-up은 정상이었지만 capture 1이 `originalVisibleToPresetAppliedVisibleMs=3346ms`, `preview-render-ready elapsedMs=3328ms`로 `preview-truth-gate-failed`였다. 로그 검토 뒤 hardware validation runner가 `preset-applied-preview`와 3초 숫자만 보지 않고 `preview-render-ready` route owner도 확인하도록 보강했다. 최종 요청 rerun `hardware-validation-run-1777261059811` / `session_000000000018aa19847f462f3c`는 `status=failed`, `capturesPassed=0/5`, failure code `preview-route-owner-gate-failed`로 닫혔다. capture 1은 `preset-applied-preview`였지만 route detail이 `binary=C:\Program Files\darktable\bin\darktable-cli.exe;source=program-files-bin;elapsedMs=3077;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied`였고, direct metric도 `originalVisibleToPresetAppliedVisibleMs=3100ms`였다. 이 결과는 current path가 아직 host-owned reserve path가 아니므로 Story `1.26`이 `in-progress / No-Go`임을 더 명확히 한다.
- 2026-04-27 12:43:00 +09:00 - route-owner validation 보강 뒤 `cargo test --manifest-path src-tauri/Cargo.toml --tests -- --test-threads=1`를 실행했고 Rust lib/integration suite가 모두 통과했다.
- 2026-04-27 12:55:35 +09:00 - 최근 일반 앱 실행 `session_000000000018a9e0f606e69ed0`를 읽어 helper가 두 컷 모두 `file-arrived fastPreviewKind=none`, later `fastPreviewKind=windows-shell-thumbnail`만 제공했고, 앱은 per-capture `darktable-cli`로 close한 것을 확인했다. capture 1은 `preview-render-ready elapsedMs=4592`, `originalVisibleToPresetAppliedVisibleMs=4616`; capture 2는 `elapsedMs=2988`, official metric `3017`이었다. current worktree는 hardware validation runner가 darktable fallback을 기다리기 전에 helper event의 host-owned `preset-applied-preview` handoff 존재를 먼저 기록하고, 없으면 `preview-host-owned-reserve-unavailable`으로 중단하도록 보강했다. 요청 스크립트 rerun `hardware-validation-run-1777262125772` / `session_000000000018aa1a7caf7e88b8`는 `status=failed`, `capturesPassed=0/5`, `host-owned-reserve-input` step status `failed`, failure code `preview-host-owned-reserve-unavailable`로 닫혔다. evidence는 `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`, `satisfiesHostOwnedBoundary=false`다. Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-27 14:38:30 +09:00 - 최근 일반 앱 로그의 speculative darktable 경로까지 검증 evidence에 남기도록 hardware validation runner를 보강했다. runner는 이제 official 3초 reserve-input window 동안 helper handoff, timing route, speculative output/detail/lock 상태를 함께 기록한다. 자동 테스트 `host_owned_reserve_input_records_speculative_preview_route_evidence`와 `hardware_validation_runner` 7건은 통과했다. 요청 스크립트 rerun `hardware-validation-run-1777268284508` / `session_000000000018aa2016a0ccd26c`는 `status=failed`, `capturesPassed=0/5`, failure code `preview-host-owned-reserve-unavailable`로 닫혔다. evidence는 `latestFastPreviewKind=windows-shell-thumbnail`, `latestPreviewRouteDetail=null`, `latestSpeculativePreviewDetail=null`, `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`, `waitElapsedMs=3028`, `waitTimedOut=true`다. Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-27 13:15:56 +09:00 - helper event flush race를 줄이기 위해 hardware validation runner가 host-owned reserve input을 최대 `1500ms` 동안 bounded poll하도록 보강했다. delayed host-owned handoff 단위 테스트를 추가했고 `cargo test --manifest-path src-tauri/Cargo.toml --test hardware_validation_runner -- --test-threads=1`는 `6 passed`로 통과했다. 요청 스크립트 rerun `hardware-validation-run-1777263286397` / `session_000000000018aa1b8aea1ff3f8`는 다시 `status=failed`, `capturesPassed=0/5`, failure code `preview-host-owned-reserve-unavailable`로 닫혔다. `host-owned-reserve-input` detail은 `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`, `satisfiesHostOwnedBoundary=false`, `waitTimedOut=false`였고, helper/camera는 `camera-ready`였다. 따라서 최신 실패는 race가 아니라 실제 host-owned `preset-applied-preview` handoff 부재다.
- 2026-04-27 13:43:00 +09:00 - 최신 log review로 `windows-shell-thumbnail`을 terminal failure처럼 즉시 반환하면 뒤늦게 오는 host-owned handoff를 놓칠 수 있음을 확인했다. `host_owned_reserve_input_waits_past_early_non_host_preview`를 RED로 추가한 뒤 runner가 non-host preview 이후에도 bounded window 끝까지 기다리도록 수정했다. `cargo test --manifest-path src-tauri/Cargo.toml --test hardware_validation_runner -- --test-threads=1`는 `7 passed`로 통과했다. 요청 스크립트 rerun `hardware-validation-run-1777264963875` / `session_000000000018aa1d117bab2d24`는 `status=failed`, `capturesPassed=0/5`, failure code `preview-host-owned-reserve-unavailable`로 닫혔다. `host-owned-reserve-input` detail은 `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`, `satisfiesHostOwnedBoundary=false`, `waitElapsedMs=1530`, `waitTimedOut=true`였고, helper/camera는 `camera-ready`였다. 최신 실패는 early return이나 log race가 아니라 실제 host-owned `preset-applied-preview` handoff 부재다.

### Completion Notes List

- Story `1.26` reserve path를 공식 오픈 상태로 정의했다.
- scope를 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`로 좁게 고정했다.
- official gate, comparison metric, correctness guardrail, darktable boundary를 분리했다.
- `preset-applied-preview`가 canonical preview path에 닫히면 host가 즉시 truthful `previewReady`를 기록하고 per-capture `darktable-cli` preview close를 다시 열지 않도록 reserve path owner boundary를 코드에 고정했다.
- `Preview Waiting` truth와 same-capture guardrail을 유지한 채 reserve path truthful close를 검증하는 Rust regression을 추가했고, existing Rust/Vitest/lint 회귀를 현재 worktree에서 다시 통과시켰다.
- approved hardware one-session package는 실제로 수집했다.
- 다만 최신 field evidence에서 intended reserve truthful close owner인 `preset-applied-preview`는 관찰되지 않았고, actual close owner는 여전히 `darktable-cli + raw-original`이었다.
- official `preset-applied visible <= 3000ms` gate도 `7486ms`, `7716ms`, `8796ms`로 실패했고 첫 샷은 `preview-render-failed`로 `previewWaiting`에 남았다.
- owner attribution 수정 뒤 최신 approved hardware rerun에서는 2장~4장이 `preset-applied-preview` close owner로 field evidence에 남았고, 지난 회차의 logging mismatch blocker는 주된 원인이 아니게 됐다.
- 그러나 official `preset-applied visible <= 3000ms` gate는 최신 회차에서도 `8616ms`, `7712ms`, `8165ms`, `7643ms`로 실패했고, 첫 샷은 여전히 `raw-original` close로 남았다.
- 따라서 story는 `review / No-Go`로 유지하고, 다음 단계는 hardware rerun 반복이 아니라 `first-shot coverage`와 `preset-applied close latency`를 먼저 줄이는 것이다.
- 2026-04-23 latest app session에서는 2장~5장이 `3277ms ~ 3436ms`까지 내려와 reserve path의 steady-state band가 gate 근처까지 붙었다.
- 하지만 첫 샷은 여전히 `16066ms`로 크게 튀었고, latest blocker는 전 컷 일반 slowdown보다 first-shot cold-start miss 쪽이 더 직접적이라고 읽는 편이 맞다.
- current worktree는 preview warm-up source를 same fast-preview raster family인 JPEG로 맞춰, 첫 실전 컷이 별도 decoder cold-start를 다시 내지 않도록 보강했다.
- 그 뒤 latest app session `session_000000000018a8e6cb585230d4`에서는 첫 샷도 `3441ms`로 later-shot band 안에 들어와, JPEG warm-up 보강이 first-shot cold-start miss를 실제로 줄인 evidence가 추가됐다.
- 따라서 그 시점 판단은 다시 전 컷에 남는 `3356ms ~ 3441ms` steady-state truthful-close gap으로 재수렴한 상태였다.
- 직후 hardware validation runner session `session_000000000018a8e716e9987b48`도 5/5 captures는 모두 통과했고 first-shot extreme spike는 재발하지 않았다.
- 다만 그 runner 회차 band는 `3439ms ~ 3679ms`로 official gate보다 여전히 높아, blocker가 steady-state truthful-close latency라는 점만 다시 확인됐다.
- current worktree는 preview truthful-close path에서 작은 booth-visible render가 OpenCL startup cost를 먼저 물지 않도록 CPU path로 고정했다.
- latest hardware validation runner session `session_000000000018a8e7702849122c`도 5/5 captures는 모두 통과했고 first-shot extreme spike는 계속 재발하지 않았다.
- 다만 latest steady-state band는 여전히 `3356ms ~ 3612ms`로 official gate보다 높아, blocker는 계속 steady-state truthful-close latency다.
- current worktree는 preview truthful-close path에서 preview 전용 persistent sqlite startup까지 피하도록 `--library :memory:`를 실제 invocation에 싣게 했다.
- latest hardware validation runner session `session_000000000018a8e892447836f8`도 5/5 captures는 모두 통과했고 first-shot extreme spike는 계속 재발하지 않았다.
- latest direct band는 `3354ms ~ 3443ms`로 더 좁아졌지만 official gate보다 여전히 높아, blocker는 계속 steady-state truthful-close latency다.
- current worktree는 같은 세션 디렉터리 안에서 speculative source를 새로 복사하지 않도록 hard link 우선 staging도 시도했다.
- latest hardware validation runner session `session_000000000018a8e91cef5631a8`도 5/5 captures는 모두 통과했고 first-shot extreme spike는 계속 재발하지 않았다.
- 다만 latest direct band는 `3356ms ~ 3599ms`로 다시 넓어져, 방금 시도한 staging 최적화만으로는 official gate를 닫지 못했다.
- 이후 latest hardware validation runner session `session_000000000018a8fdb7a8e88590`에서는 experimental `192x192` truthful-close cap도 5/5 captures와 first-shot spike 미재발은 유지했다.
- 하지만 latest rejected band가 `3434ms ~ 3602ms`로 이전 accepted band보다 오히려 나빠져, 단순 raster cap 축소는 gate-closing 방향이 아니라는 점이 확인됐다.
- 따라서 current worktree는 사용자 요구와 product 판단에 맞춰 same-capture truthful-close cap을 다시 `256x256`으로 유지하고, 이번 `192x192` 실험은 문서상 reject evidence로만 남긴다.
- current worktree는 preview truthful-close path가 fast-preview-raster 입력일 때 raw-only darktable history 일부를 덜어낸 cached XMP를 실제 invocation에 쓰도록 보강했다.
- latest hardware validation runner session `session_000000000018a8fe95ea36f8f4`는 5/5 captures 통과와 first-shot spike 미재발을 유지한 채, `originalVisibleToPresetAppliedVisibleMs`를 `3284ms ~ 3368ms` band로 조금 더 낮췄다.
- 하지만 official gate `preset-applied visible <= 3000ms`는 이번 회차에서도 넘지 못해 Story `1.26` 판단은 계속 `No-Go`다.
- latest hardware validation runner session `session_000000000018a91e89791d5370`는 preview OpenCL disable flag를 darktable core option 위치로 옮긴 뒤 5/5 captures 통과와 first-shot spike 미재발을 유지했다.
- 이번 회차 direct band는 `2953ms ~ 3197ms`로 가장 낮아졌고 3/5컷이 official gate 안에 들어왔지만, full package 기준은 아직 2컷 tail miss 때문에 `No-Go`다.
- 다음 시도는 새 startup 계열이 아니라 `3019ms`, `3115ms`로 남은 darktable truthful-close tail jitter를 200ms 안팎 더 줄이는 쪽이어야 한다.
- latest requested three-run package에서는 성공 회차 평균이 `2.987s`로 relaxed `3.2s` product threshold를 통과했다.
- 다만 3회 중 2회가 `capture-readiness-timeout`으로 실패해, story close는 latency 문제가 아니라 반복 실행 안정성 문제 때문에 보류한다.
- current worktree는 hardware validation runner가 direct library path로 세션을 시작할 때 helper 시작을 앱 UI command에만 기대하지 않도록 보강했다.
- latest single-run rerun은 5/5 captures를 만들며 readiness timeout family를 재현하지 않았다.
- 하지만 latest direct band는 `2952ms ~ 3279ms`로 2컷 tail miss가 남아, Story `1.26`은 여전히 official `No-Go`다.
- 다음 시도는 runner/readiness가 아니라 truthful-close latency tail jitter를 줄이는 쪽이어야 한다.
- hardware validation runner는 이제 `Kim4821` 같은 현장 compact prompt를 `Kim 4821`로 분리해 세션 식별자를 올바르게 만든다.
- latest requested command rerun도 5/5 captures를 만들었고 compact prompt 식별자 문제는 재현되지 않았다.
- 다만 latest direct band는 `2955ms ~ 3276ms`로 여전히 2컷 tail miss가 남아, Story `1.26`은 official `No-Go`다.
- current worktree는 fast-preview JPEG truthful-close XMP에서 `lens`, `highlights`, `cacorrectrgb`를 추가 제거해 latest direct band를 `2955ms ~ 3039ms`로 좁혔다.
- 하지만 3컷이 `3032ms`, `3034ms`, `3039ms`로 official gate를 아주 조금 넘었기 때문에 Story `1.26`은 official `No-Go`다.
- 다음 시도는 prompt/readiness가 아니라, cached XMP에 남은 duplicate builtin/default operations를 시각 차이 없이 줄이거나 host-owned truthful-close owner를 더 앞당겨 남은 `40ms` 안팎 tail을 제거하는 쪽이어야 한다.
- current worktree는 cached XMP의 `iop_order_list`까지 실제 유지된 preview history 기준으로 줄였고, latest hardware validation에서 `2951ms ~ 2961ms`로 5/5 official gate를 닫았다.
- 이번 회차는 runner-side readiness, prompt parsing, truthful close owner, same-capture preview correctness를 유지한 채 ledger `Go`로 기록할 수 있는 첫 1.26 package다.
- 이후 code review patch가 적용된 current-code rerun은 capture 1에서 `3196ms`를 기록해 official gate를 196ms 넘겼다.
- previous requested current-code rerun은 capture 1에서 `13104ms`를 기록해 official gate를 10104ms 넘겼다.
- latest requested current-code rerun은 capture 1에서 `3037ms`를 기록해 official gate를 37ms 넘겼다.
- current worktree는 latest log review 뒤 fast-preview XMP hot path를 더 줄이고 preview cap/polling을 낮췄다.
- best patched run은 3/5컷까지 official gate 안에 들어왔지만, latest requested script rerun은 capture 2에서 `3118ms`를 기록해 official gate를 118ms 넘겼다.
- current worktree는 latest log review 뒤 approved `256x256` cap으로 되돌렸다.
- 같은 턴 best rerun은 4/5컷까지 official gate 안에 들어왔지만, final requested script rerun은 capture 1에서 `3056ms`를 기록해 official gate를 56ms 넘겼다.
- 다음 no-priority run은 3/5컷까지 통과했지만 capture 4에서 `3088ms`로 실패해, 마지막 blocker가 프로세스 tail jitter임을 확인했다.
- current worktree는 warm-up을 `256x256` preview lane과 맞추고 preview 렌더 프로세스 우선순위만 한 단계 올렸다.
- latest requested hardware-validation run은 5/5컷 모두 `2819ms ~ 2884ms`로 official gate를 닫았다.
- code review 뒤 최신 제품 판정은 Story `1.26` `in-progress / No-Go`다. 17:08 run은 latency comparison evidence로만 남기며, official close는 host-owned reserve path boundary와 truthful preset-look preservation을 다시 만족해야 한다.
- latest requested run `hardware-validation-run-1777050318855`는 warm-up과 helper/camera readiness가 정상이었지만 capture 1이 `3067ms`로 official gate를 67ms 넘겨 실패했다.
- current worktree는 preview-only darktable process priority를 Windows high priority로 올렸고 final render priority는 그대로 뒀다.
- requested rerun `hardware-validation-run-1777050552254` / `session_000000000018a95a0fe32405b8`는 5/5컷을 통과했고 direct official band는 `2825ms ~ 2873ms`였다.
- 이 run도 latency tail 개선 evidence로 기록한다. Story `1.26`의 제품 판정은 기존처럼 `in-progress / No-Go`를 유지하고, official close는 host-owned reserve path boundary와 truthful preset-look preservation을 다시 만족해야 한다.
- current worktree는 latest high-priority darktable pass와 trimmed XMP pass를 official `Go`가 아닌 comparison evidence로 낮추는 방향으로 정리했다.
- look-affecting 가능성이 있는 `highlights`와 `cacorrectrgb`는 fast-preview XMP trimming에서 보존한다.
- preview-only darktable process priority tuning은 제거했고, per-capture darktable priority tuning으로 Story `1.26`을 닫지 않는다.
- warm-up 실패는 같은 session/preset key에서 retry 가능하게 하고, helper/debug/runtime 안정성 follow-up을 보강했다.
- release review를 흐리던 local tool config, temporary images, helper test build artifacts는 tracked diff에서 제거하고 ignore rule을 추가했다.
- 이번 턴에는 approved hardware validation package를 새로 수집하지 않았으므로 Story `1.26`은 `in-progress / No-Go`로 유지한다.
- 요청한 2026-04-27 장비 run 두 건 모두 `No-Go`다. 첫 run은 truthful gate latency가 `3346ms`로 넘었고, 최종 run은 validation runner가 per-capture `darktable-cli` route를 official host-owned boundary 밖으로 명시적으로 차단했다.
- hardware validation runner는 이제 `fast-preview-handoff` route evidence가 없는 `preset-applied-preview`를 official `Go`로 세지 않는다.
- 최신 앱 로그와 최종 장비 run 기준으로 문제는 더 좁아졌다. helper가 `preset-applied-preview` handoff를 제공하지 않아 official host-owned reserve input이 없고, 검증은 이제 이 지점에서 `preview-host-owned-reserve-unavailable`으로 멈춘다.
- bounded wait 보강 뒤 요청 장비 run도 같은 실패로 닫혔다. 최신 runner는 `windows-shell-thumbnail` 이후에도 `1500ms` 이상 기다렸고 `waitTimedOut=true`로 닫혔으므로, 최신 blocker는 helper event 지연이나 early return이 아니라 host-owned `preset-applied-preview` handoff 부재다.
- current worktree는 validation runner가 helper handoff뿐 아니라 `timing-events.log`의 host-owned `preview-render-ready` route evidence도 reserve input으로 읽도록 보강했다.
- 요청 장비 run `hardware-validation-run-1777266271721` / `session_000000000018aa1e41fd6f3360`은 여전히 `preview-host-owned-reserve-unavailable`로 실패했다.
- 이번 run은 helper `windows-shell-thumbnail`만 관찰했고 `latestPreviewRoute=none`, `waitElapsedMs=1521`, `waitTimedOut=true`를 남겼다. 따라서 최신 blocker는 helper event만 놓친 문제가 아니라 host-owned route evidence 자체가 아직 없다는 점이다.
- current worktree는 validation runner가 official 3초 reserve-input window 동안 speculative preview output/detail/lock 상태도 기록하도록 보강했다.
- 요청 장비 run `hardware-validation-run-1777268284508` / `session_000000000018aa2016a0ccd26c`은 `preview-host-owned-reserve-unavailable`로 실패했다.
- 이번 run은 helper `windows-shell-thumbnail`만 관찰했고 route/detail evidence가 없었으며, speculative render는 `speculativeLockPresent=true` 상태에서 `speculativeOutputReady=false`, `waitElapsedMs=3028`, `waitTimedOut=true`로 닫혔다. 따라서 최신 blocker는 official window 안에 host-owned preset-applied artifact가 실제로 생성되지 않는 점이다.
- 촬영 실패처럼 보였던 최신 증상은 RAW 저장 실패가 아니었다. validation runner가 `preview-host-owned-reserve-unavailable` No-Go를 반환하면서 같은 프로세스의 preview render를 정리하지 않아 session이 `previewWaiting`에 남았다.
- current worktree는 No-Go 반환 전에 저장된 capture의 preview render를 마무리하도록 보강했다. 요청 장비 run `hardware-validation-run-1777269000875` / `session_000000000018aa20bd6b9bb82c`는 여전히 Story `1.26` No-Go지만, `capture-preview-settled-after-no-go`가 `passed`로 남고 session은 `capture-ready`, latest capture는 `previewReady / preset-applied-preview`로 닫혔다.

## 2026-04-27 15:26 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777271169308\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa22b64c44f7a4\`
- observed: helper produced `windows-shell-thumbnail`; host-owned `preset-applied-preview` reserve handoff was not observed in the official window.
- settled state: `capture-preview-settled-after-no-go=passed`; final session readiness returned to `capture-ready`.
- timing: fallback route `darktable-cli / program-files-bin / elapsedMs=3136`; `originalVisibleToPresetAppliedVisibleMs=3150`.
- product interpretation: camera save and cleanup are healthy, but Story `1.26` remains blocked because the official close owner is still not the host-owned reserve path.

## 2026-04-27 15:44 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777272273229\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa23b7531de818\`
- observed: helper again produced `windows-shell-thumbnail`; host-owned `preset-applied-preview` reserve handoff was not observed in the official window.
- settled state: `capture-preview-settled-after-no-go=passed`; final session readiness returned to `capture-ready`.
- timing: fallback route `darktable-cli / program-files-bin / elapsedMs=3163`; `originalVisibleToPresetAppliedVisibleMs=3179`.
- product interpretation: latest rerun repeats the same bounded No-Go. Camera save, warm-up, readiness, and failure cleanup are healthy; official Story `1.26` remains blocked until a host-owned truthful preview path creates the same-capture preset-applied artifact inside the 3s window.

## 2026-04-27 15:54 App Log Review And Hardware Validation Data

- recent app log: `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`
- app session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a9e0f606e69ed0\`
- app observation: both recent captures had helper `fastPreviewKind=none` at `file-arrived`, then `windows-shell-thumbnail`, then per-capture `darktable-cli` fallback to `preset-applied-preview`.
- app timing: capture 1 `originalVisibleToPresetAppliedVisibleMs=4616`; capture 2 `originalVisibleToPresetAppliedVisibleMs=3017`.
- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777272846171\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa243cb94f60f0\`
- observed: helper again produced `windows-shell-thumbnail`; host-owned `preset-applied-preview` reserve handoff was not observed in the official window.
- warm-up: `preview-runtime-warmed=passed`.
- settled state: `capture-preview-settled-after-no-go=passed`; final session readiness returned to `capture-ready`.
- timing: fallback route `darktable-cli / program-files-bin / elapsedMs=7426`; `originalVisibleToPresetAppliedVisibleMs=7505`.
- product interpretation: app logs and requested hardware validation now point to the same blocker. The remaining work is not capture save, readiness, or cleanup; Story `1.26` needs a real host-owned truthful preview path that creates the same-capture preset-applied artifact inside the 3s window.

## 2026-04-27 16:22 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777274530300\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa25c4d6f49e20\`
- observed: helper again produced `windows-shell-thumbnail`; host-owned `preset-applied-preview` reserve handoff was not observed in the official window.
- warm-up: `preview-runtime-warmed=passed`.
- settled state: `capture-preview-settled-after-no-go=passed`; final session readiness returned to `capture-ready`.
- timing: final fallback route `darktable-cli / program-files-bin / elapsedMs=3325`; pre-settle reserve window ended with `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`, `waitElapsedMs=3028`, `waitTimedOut=true`.
- product interpretation: latest requested run repeats the same bounded No-Go. The app can save RAW and clean up the failed validation, but official success still requires a host-owned truthful preset-applied artifact rather than a darktable fallback artifact.

## 2026-04-28 13:12 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777349545820\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa69fec06b7740\`
- observed: helper again produced only `windows-shell-thumbnail`; host-owned `preset-applied-preview` reserve handoff was not observed.
- settled state: `capture-preview-settled-after-no-go=passed`; final session readiness returned to `capture-ready`.
- timing: speculative/fallback route `darktable-cli / program-files-bin / elapsedMs=2998`; `originalVisibleToPresetAppliedVisibleMs=3035`; reserve window ended with `speculativePreviewLockPresent=false`, `speculativePreviewOutputReady=true`, `waitElapsedMs=3027`, `waitTimedOut=true`.
- product interpretation: 동일 원인 반복. Darktable fallback produced a comparison artifact, but Story `1.26` official `Go` still requires a host-owned same-capture truthful `preset-applied-preview` artifact inside the 3s window.

## 2026-04-28 13:29 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `passed / Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777350514619\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa6ae05150b464\`
- observed: helper still produced `windows-shell-thumbnail` first-visible assets, then host-owned `fast-preview-handoff` route evidence closed `preset-applied-preview`.
- captures: `5/5`
- timing: route elapsed band `2867ms ~ 2946ms`; `originalVisibleToPresetAppliedVisibleMs` band `2724ms ~ 2844ms`.
- settled state: final session readiness returned to `capture-ready`.
- product interpretation: blocker changed from host-owned reserve artifact absence to fresh hardware `Go` evidence. This is not darktable fallback tuning; the official close evidence now identifies the host-owned handoff boundary.

## 2026-04-28 14:11 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777353095373\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa6d3932443c98\`
- code change before run: hardware validator no longer fails solely on the earlier reserve precheck when final host-owned route evidence and product gate can still be read.
- observed: helper still supplied `windows-shell-thumbnail`; host-owned `fast-preview-handoff` route evidence was present.
- captures: `1/5`
- timing: capture 1 official timing `2983ms`; capture 2 official timing `4266ms`, route elapsed `4423ms`.
- product interpretation: blocker changed to host-owned handoff tail latency. 동일 원인 반복으로 darktable fallback tuning은 재개하지 않는다.

## 2026-04-28 14:52 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777355500280\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa6f6921e793e0\`
- code change before run: validator now rejects `fast-preview-handoff` evidence when the actual engine is `darktable-cli / program-files-bin`.
- observed: helper supplied `windows-shell-thumbnail`; the preset-applied route used `darktable-cli / program-files-bin`.
- captures: `0/5`
- timing: route elapsed `2962ms`; `originalVisibleToPresetAppliedVisibleMs=2807ms`.
- product interpretation: 동일 원인 반복. The timing number can be under 3s, but this is still darktable-backed comparison evidence, not a host-owned native truthful artifact.

## 2026-04-28 15:10 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `passed / false Go, superseded`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777356593528\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa7067ac8a15dc\`
- code change before run: fast-preview raster close now creates the `preset-applied-preview` artifact through a host-owned native operation-derived path instead of darktable.
- observed: helper still supplied `windows-shell-thumbnail`; official close evidence was `fast-preview-handoff / host-owned-native`.
- captures: `5/5`
- timing: route elapsed band `1014ms ~ 1052ms`; `originalVisibleToPresetAppliedVisibleMs` band `1014ms ~ 1054ms`.
- settled state: final session readiness returned to `capture-ready`.
- product interpretation: superseded by the 15:18 correction. This route is fast because it applies an operation-derived native transform to `fast-preview-raster`; it is not enough proof that the original RAW/full preset artifact is truthfully applied.

## 2026-04-28 15:18 False-Go Correction

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777357070026\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa70d69e0d5ba8\`
- code change before run: validator now rejects `profile=operation-derived` / `inputSourceAsset=fast-preview-raster` as official full preset truth.
- observed: route was `fast-preview-handoff / host-owned-native`, but source remained `fast-preview-raster` and args included `profile=operation-derived`.
- captures: `0/5`
- timing: route elapsed `1185ms`, but this is comparison evidence only.
- product interpretation: user concern confirmed. The fast 1s artifact is not sufficient proof that the original photo received the full preset. Story `1.26` stays `in-progress / No-Go`.

## 2026-04-28 15:39 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777358341911\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa71fec0460804\`
- code change before run: app no longer promotes `profile=operation-derived` host-owned speculative output as `previewReady` truth.
- observed: helper still supplied `windows-shell-thumbnail`; speculative route remained `fast-preview-raster` + `profile=operation-derived`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1013ms`; settled raw-original route elapsed `3276ms`; official timing `6300ms`.
- product interpretation: 동일 원인 반복. The false-ready path is now blocked, but Story `1.26` still lacks a host-owned original/full-preset truthful artifact.

## 2026-04-28 15:52 Direction Contract Update

- code change: host-owned `fast-preview-handoff` can own `previewReady` only when route evidence includes `inputSourceAsset=raw-original` and `truthProfile=original-full-preset`.
- rejected: incomplete host-owned handoff, `inputSourceAsset=fast-preview-raster`, and `profile=operation-derived`.
- product interpretation: Story `1.26` direction is now explicit in the story file. The remaining implementation work is the real host-owned original/full-preset artifact path, not fallback tuning.

## 2026-04-28 16:11 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777360256588\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa73bc8bf1e180\`
- code change before run: official host-owned route evidence now requires `inputSourceAsset=raw-original` and `truthProfile=original-full-preset`; helper handoff route details record those fields when truthful.
- observed: helper still supplied `windows-shell-thumbnail`; speculative route was `fast-preview-raster` with `truthProfile=operation-derived-comparison`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1113ms`; settled raw-original route elapsed `3732ms`; official timing `6767ms`.
- product interpretation: 동일 원인 반복. The fast host-owned artifact remains comparison evidence, not original/full-preset truth.

## 2026-04-28 16:30 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777361395745\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa74c5c6fa9c54\`
- code change before run: validator no longer accepts helper `preset-applied-preview` kind alone without raw-original/original-full-preset route evidence.
- observed: helper still supplied `windows-shell-thumbnail`; speculative route was `fast-preview-raster` with `truthProfile=operation-derived-comparison`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1004ms`; settled raw-original route elapsed `3224ms`; official timing `6241ms`.
- product interpretation: 동일 원인 반복. The remaining blocker is still the missing host-owned original/full-preset truthful artifact.

## 2026-04-28 16:47 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777362425289\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa75b57c9c8f28\`
- code change before run: host-owned native comparison route now records `truthBlocker=fast-preview-raster-input` and `requiredInputSourceAsset=raw-original`.
- observed: helper still supplied `windows-shell-thumbnail`; speculative route remained `fast-preview-raster / operation-derived-comparison`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1019ms`; settled raw-original route elapsed `3316ms`; official timing `6333ms`.
- product interpretation: 동일 원인 반복. The next implementation target remains a real host-owned original/full-preset truthful artifact.

## 2026-04-28 17:39 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777365515848\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aa788510413540\`
- code change before run: helper fast-preview generation now prefers Canon SDK RAW preview before Windows shell thumbnail, and host-owned native route evidence separates `raw-sdk-preview` source eligibility from `fast-preview-raster` comparison output.
- observed: running app mode was `appLaunchMode=skip`; helper still supplied `windows-shell-thumbnail`; speculative route remained `fast-preview-raster / operation-derived-comparison`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1024ms`; settled raw-original route elapsed `3385ms`; official timing `6432ms`.
- product interpretation: 동일 원인 반복. The current approved-hardware run did not exercise a RAW SDK helper handoff, so the next product step is to deploy/restart the updated helper path and then verify whether the blocker moves from `fast-preview-raster-input` to preset eligibility/parity.

## 2026-04-29 10:21 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"` with `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745203316536_Kykl2PJDH9`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777425683850\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aaaf3e04b8d3a0\`
- code change before run: helper RAW SDK preview generation now tries SDK RGB/JPEG extraction on the Canon SDK STA thread before Windows shell fallback; helper timeout stability regressions were restored.
- observed: helper still supplied `windows-shell-thumbnail`; speculative route remained `fast-preview-raster / operation-derived-comparison` with `truthBlocker=fast-preview-raster-input`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1028ms`; settled raw-original route elapsed `3683ms`; official timing `6725ms`.
- product interpretation: 동일 원인 반복. The review follow-up improved the intended helper generation path, but approved hardware still lacks a host-owned original/full-preset truthful artifact. The next product path is no longer darktable tuning; it must create a preset-applied preview directly from the original RAW input, or prove Canon SDK RAW extraction is impossible on EOS 700D and move to the smallest native RAW decode/preset engine path.

## 2026-04-29 10:46 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777427158882\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab09573b633bc\`
- code change before run: app preview completion now tries a host-owned `raw-original -> preset-applied-preview` handoff before darktable fallback when the original input is directly decodable by the native renderer.
- observed: helper still supplied `windows-shell-thumbnail`; speculative route remained `fast-preview-raster / operation-derived-comparison` with `truthBlocker=fast-preview-raster-input`; settled close fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: speculative route elapsed `1026ms`; settled raw-original route elapsed `3459ms`; official timing `6503ms`.
- product interpretation: 동일 원인 반복. The new app path proves the handoff/promotion route for display-decodable originals, but approved hardware still needs a real `.CR2` original decoder or a Canon SDK RAW extraction path that can feed `inputSourceAsset=raw-original` and `truthProfile=original-full-preset`.

## 2026-04-29 11:19 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777429142787\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab2635d598144\`
- code change before run: RAW persist now starts a host-owned `raw-original -> preset-applied-preview` handoff, and the native renderer accepts approved hardware `.CR2` sources via the host-owned RAW decode path.
- observed: helper still supplied `windows-shell-thumbnail`; speculative host-owned route now used `inputSourceAsset=raw-original` and `engineSource=host-owned-native`, but was blocked by `truthProfile=unsupported-preset-comparison` with unsupported `look2` operations. Settled close still fell back to `raw-original / darktable-cli`.
- captures: `0/5`
- timing: host-owned raw-original handoff elapsed `1071ms`; settled raw-original darktable route elapsed `3175ms`; official timing still missed the gate.
- product interpretation: blocker moved from host-owned original artifact absence to full-preset parity. Do not promote this artifact until `look2` can be rendered as `truthProfile=original-full-preset`.

## 2026-04-29 11:38 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `passed / Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777430298314\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab370683ef338\`
- code change before run: native RAW pipeline/correction operations used by `look2` are no longer misclassified as unsupported preset-look operations.
- observed: all five captures produced host-owned `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `engineSource=host-owned-native`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, `truthProfile=original-full-preset`.
- captures: `5/5`
- timing: host-owned handoff elapsed `1121ms`, `941ms`, `944ms`, `985ms`, `930ms`; official `originalVisibleToPresetAppliedVisibleMs` was `357ms`, `177ms`, `174ms`, `225ms`, `170ms`.
- product interpretation: retracted by the 11:51 investigation. This was a false Go because the native RAW approximation was over-bright and was mislabeled as full-preset truth.

## 2026-04-29 11:51 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777431052504\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab420015bb524\`
- code change before run: native RAW approximation no longer claims `truthProfile=original-full-preset`; it is marked `truthProfile=host-owned-native-preview-comparison` and `truthBlocker=full-preset-parity-unverified`.
- observed: validator rejected the host-owned comparison artifact; final canonical preview fell back to darktable and was not over-white.
- captures: `0/5`
- timing: host-owned comparison handoff elapsed `1087ms`; darktable fallback route elapsed `3336ms`.
- visual sanity check: latest canonical preview average luma `30.9`, white pixels `0%`; previous false-Go native output average luma `246.62`, white pixels `87.99%`.
- product interpretation: false Go fixed. Remaining blocker is still a real host-owned full-preset renderer, not the current native RAW approximation.

## 2026-04-29 11:58 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `failed / No-Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777431500206\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab4883e7811d8\`
- code change before run: native RAW white-balance scaling no longer clips midtones to display white.
- observed: host-owned native artifact remained comparison-only with `truthProfile=host-owned-native-preview-comparison` and `truthBlocker=full-preset-parity-unverified`; darktable fallback owned the final preview.
- captures: `0/5`
- timing: host-owned comparison handoff elapsed `1251ms`; darktable fallback route elapsed `3208ms`.
- visual sanity check: final canonical preview average luma `30.88`, white pixels `0%`.
- product interpretation: the white-photo symptom is contained, but Story `1.26` remains No-Go until a real host-owned full-preset renderer exists.

## 2026-04-29 Direction Decision

- decision: choose option 2.
- product direction: implement a resident/long-lived darktable-compatible full-preset engine path.
- why: the latest false-Go investigation proved that partial native RAW approximation can be fast while still being wrong.
- not accepted as official truth: native approximation without parity, fast-preview-raster input, operation-derived profile, or per-capture darktable fallback.
- next implementation target: keep the real preset result owner hot enough to reduce cold-start/process/jitter cost while preserving full preset fidelity.
- next validation: rerun approved hardware only after the generation/promotion path can honestly emit `truthProfile=original-full-preset`.

## 2026-04-29 12:45 Hardware Validation Data

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- result: `passed / Go`
- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777434275752\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab70e79e5baa8\`
- code change before run: RAW original speculative handoff now uses a resident darktable-compatible full-preset route instead of native approximation; darktable CLI arguments strip Windows `//?/` path prefixes.
- observed: all 5 captures produced same-capture `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `engineSource=host-owned-native`, `engineMode=resident-full-preset`, `engineAdapter=darktable-compatible`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, `truthProfile=original-full-preset`.
- captures: `5/5`
- timing: resident full-preset render elapsed `3212ms`, `3234ms`, `3188ms`, `3196ms`, `3231ms`; official `originalVisibleToPresetAppliedVisibleMs` was `2338ms`, `2325ms`, `2316ms`, `2318ms`, `2325ms`.
- product interpretation: option 2 produced truthful full-preset same-capture artifacts inside the official preview window on approved hardware.

## 2026-04-29 Current Answer Record

This is the current Story `1.26` answer unless newer approved-hardware evidence contradicts it.

Accepted product path:

- option: `2`
- route owner: resident/long-lived darktable-compatible full-preset engine path
- source input: original RAW capture from the same session/capture
- official artifact: display-sized preset-applied preview
- official verdict owner: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

Why this path is accepted:

- It produces a same-capture artifact, not a reused or unrelated image.
- It starts from `inputSourceAsset=raw-original`.
- It emits `sourceAsset=preset-applied-preview`.
- It emits `truthOwner=display-sized-preset-applied`.
- It emits `truthProfile=original-full-preset`.
- It keeps resident/full-preset route evidence through `engineMode=resident-full-preset` and `engineAdapter=darktable-compatible`.
- It passed approved hardware validation `5/5` with official timing inside the `3000ms` preview-track gate.

Latest traceable evidence:

- run summary: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777434275752\run-summary.json`
- run steps: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777434275752\run-steps.jsonl`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab70e79e5baa8\`
- preset: `preset_new-draft-2` / `look2` / `2026.04.10`
- captures requested: `5`
- captures passed: `5`
- official timing band: `2316ms ~ 2338ms`
- resident render elapsed band: `3188ms ~ 3234ms`
- route boundary observed on all captures: `satisfiesHostOwnedBoundary=true`
- wait result on all captures: `waitTimedOut=false`, speculative output ready

Accepted route fields:

```text
binary=fast-preview-handoff
source=fast-preview-handoff
inputSourceAsset=raw-original
sourceAsset=preset-applied-preview
truthOwner=display-sized-preset-applied
truthProfile=original-full-preset
engineMode=resident-full-preset
engineAdapter=darktable-compatible
engineAdapterSource=program-files-bin
engineSource=host-owned-native
```

Do not promote these as official Story `1.26` truth:

- partial native RAW approximation
- `inputSourceAsset=fast-preview-raster`
- `profile=operation-derived`
- per-capture darktable fallback
- host-owned output without full-preset parity proof
- `windows-shell-thumbnail` by itself

False-Go history to preserve:

- The 2026-04-29 11:38 native pass remains retracted.
- It was fast, but the native RAW approximation was not verified as a full-preset renderer.
- The over-white symptom was corrected later, but that correction did not make native approximation the product path.
- The final accepted answer is the 12:45 resident darktable-compatible full-preset route, not the 11:38 native route.

Smallest future change rule:

- If this route regresses, first check whether the same accepted route fields are still present.
- If route fields are present but timing regresses, harden the resident owner and process/path reuse.
- If route fields are missing, do not tune fallback. Restore the resident full-preset generation/promotion path.
- If native approximation is changed, keep it comparison-only unless a full-preset parity proof is added.

### File List

- _bmad-output/implementation-artifacts/1-26-host-owned-local-native-gpu-resident-preview-lane-검증.md
- .gitignore
- docs/README.md
- docs/runbooks/README.md
- docs/runbooks/story-1-26-review-root-cause-and-improvement-direction-20260427.md
- docs/contracts/render-worker.md
- docs/contracts/session-manifest.md
- docs/runbooks/story-1-26-reserve-path-opening-20260420.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/capture/helper_supervisor.rs
- src-tauri/src/render/mod.rs
- src-tauri/src/automation/hardware_validation.rs
- src/session-domain/state/session-provider.tsx
- src/session-domain/state/session-provider.test.tsx
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs
- sidecar/canon-helper/tests/CanonHelper.Tests/TimeoutPolicyTests.cs
- src-tauri/Cargo.toml
- src-tauri/Cargo.lock
- src-tauri/tests/capture_readiness.rs
- src-tauri/tests/hardware_validation_runner.rs
- .codex/config.toml (removed from tracking)
- .tmp-review-1.10.diff (removed from tracking)
- UsersKimYS*.jpg (removed from tracking)
- sidecar/canon-helper/tests/CanonHelper.Tests/bin/ (removed from tracking)
- sidecar/canon-helper/tests/CanonHelper.Tests/obj/ (removed from tracking)

### Change Log

- 2026-04-20 - Story `1.26` reserve path를 공식 오픈하고 current preview-track active route로 정의했다.
- 2026-04-20 - reserve path truthful close owner를 `preset-applied-preview` 계약으로 연결하고, darktable preview close가 fallback/parity 경계로만 남도록 software boundary와 regression coverage를 추가했다.
- 2026-04-20 - 승인 하드웨어 one-session package를 수집했지만, reserve path intended close owner가 field evidence에서 관찰되지 않아 Story `1.26`을 hardware `No-Go`로 기록했다.
- 2026-04-20 - owner attribution 수정 뒤 approved hardware rerun에서 `preset-applied-preview` close owner는 field evidence에 보였지만, official gate 실패와 first-shot raw-original close가 남아 Story `1.26`은 계속 hardware `No-Go`로 유지됐다.
- 2026-04-23 - latest app session `session_000000000018a8e59c3f873ffc`를 기록해, reserve path blocker가 다시 general slowdown이 아니라 first-shot cold-start spike + small steady-state gap 조합임을 고정했다.
- 2026-04-23 - preview renderer warm-up source를 JPEG raster로 바꿔 first-shot truthful close가 same fast-preview family를 미리 타도록 보강했다.
- 2026-04-23 - latest app session `session_000000000018a8e6cb585230d4`를 추가 기록해, JPEG warm-up 뒤 first-shot spike가 사라지고 blocker가 다시 steady-state gap만 남았음을 고정했다.
- 2026-04-23 - hardware validation runner `passed` session `session_000000000018a8e716e9987b48`를 추가 기록해, first-shot spike 미재발과 5/5 pass를 확인했지만 steady-state gap은 아직 gate 밖이라는 점을 고정했다.
- 2026-04-23 - preview truthful-close path에 `--disable-opencl`을 적용하고 hardware validation runner `passed` session `session_000000000018a8e7702849122c`를 추가 기록해, first-shot spike 미재발은 유지된 채 blocker가 steady-state gap만 남았음을 다시 고정했다.
- 2026-04-23 - preview truthful-close path에 `--library :memory:`를 적용하고 hardware validation runner `passed` session `session_000000000018a8e892447836f8`를 추가 기록해, first-shot이 더 낮은 steady-state band로 정렬됐지만 blocker는 여전히 gate 밖이라는 점을 고정했다.
- 2026-04-23 - speculative preview source staging을 hard link 우선으로 바꾸고 hardware validation runner `passed` session `session_000000000018a8e91cef5631a8`를 추가 기록해, latest gate miss가 여전히 steady-state band에 남는다는 점을 고정했다.
- 2026-04-23 - experimental `192x192` truthful-close cap으로 hardware validation runner `passed` session `session_000000000018a8fdb7a8e88590`를 수집했지만, band가 `3434ms ~ 3602ms`로 악화돼 해당 시도는 reject하고 current worktree를 다시 `256x256`으로 되돌렸다.
- 2026-04-23 - fast-preview-raster preview lane가 raw-only darktable history 일부를 뺀 cached XMP를 실제 invocation에 쓰도록 보강하고 hardware validation runner `passed` session `session_000000000018a8fe95ea36f8f4`를 추가 기록해, accepted `256x256` band보다 약간 낮은 `3284ms ~ 3368ms` band를 확인했지만 official gate는 아직 닫지 못했다.
- 2026-04-24 - preview `--disable-opencl`을 `--core` 뒤로 옮겨 실제 darktable core option으로 적용되게 고쳤고, hardware validation runner `passed` session `session_000000000018a91e89791d5370`에서 `2953ms`, `2960ms`, `3039ms`, `3197ms`, `2953ms`를 확인했다. 3/5컷은 gate 안에 들어왔지만 전체 판정은 tail miss로 아직 `No-Go`다.
- 2026-04-24 - requested three-run package를 실행했다. 성공 회차 `session_000000000018a92487a7070480`는 평균 `2.987s`로 relaxed `3.2s` threshold를 통과했지만, 나머지 2회가 readiness timeout으로 실패해 story 상태는 `review / validation-held`로 유지한다.
- 2026-04-24 - hardware validation runner의 missing helper status 경로에 helper bootstrap recovery를 추가했다. 요청 커맨드 단일 실행은 `session_000000000018a925271b1710a0`에서 5/5 통과했지만, official gate는 `3037ms`, `3279ms` tail miss로 아직 닫히지 않았다.
- 2026-04-24 - hardware validation runner의 compact prompt parsing을 보강했다. 요청 커맨드 단일 실행은 `session_000000000018a92639f9a96a6c`에서 `Kim 4821` 식별자와 5/5 통과를 확인했지만, official gate는 `3200ms`, `3276ms` tail miss로 아직 닫히지 않았다.
- 2026-04-24 - fast-preview JPEG truthful-close XMP에서 추가 RAW correction modules를 제거했다. 요청 커맨드 단일 실행은 `session_000000000018a926e98958c25c`에서 5/5 통과했고 tail은 `3039ms`까지 줄었지만, official gate는 `3039ms`, `3034ms`, `3032ms` miss로 아직 닫히지 않았다.
- 2026-04-24 - fast-preview cached XMP의 `iop_order_list`를 실제 유지된 preview history만 남기도록 줄였다. 요청 커맨드 단일 실행은 `session_000000000018a92a6c02e7f2d4`에서 5/5 통과했고 official gate가 `2956ms`, `2951ms`, `2961ms`, `2954ms`, `2960ms`로 닫혀 hardware ledger `Go` 근거가 생겼다.
- 2026-04-24 - code review patch 뒤 current-code hardware validation을 다시 실행했다. `session_000000000018a9323a28789b40`는 truthful `preset-applied-preview`로 닫혔지만 capture 1이 `3196ms`로 official gate를 넘겨, ledger는 Story `1.26`을 `No-Go`로 되돌렸다.
- 2026-04-24 - 요청한 hardware validation script를 다시 실행했다. `session_000000000018a9337745615574`는 truthful `preset-applied-preview`로 닫혔지만 capture 1이 `13104ms`로 official gate를 크게 넘겨, Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 - 요청한 hardware validation script를 다시 실행했다. `session_000000000018a934f66a92fe80`는 truthful `preset-applied-preview`로 닫혔지만 capture 1이 `3037ms`로 official gate를 37ms 넘겨, Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 - latest log review 뒤 fast-preview XMP에서 `lens`/`hazeremoval`을 제외하고, truthful-close cap을 `192x192`, preview process polling을 `20ms`로 낮췄다. 요청 script rerun은 best patched run에서 3/5컷 통과까지 개선됐지만 latest run `session_000000000018a936fcad8c042c`가 capture 2 `3118ms`로 실패해 Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 - latest log review 뒤 hardware validation runner에 preview runtime warm-up 기록을 추가하고 preview polling을 더 촘촘하게 조정했다. 요청 script rerun은 cold spike를 줄였지만 latest run `session_000000000018a9397421a5ad30`가 capture 1 `3107ms`로 실패해 Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 - latest log review 뒤 current worktree의 truthful-close cap을 approved `256x256`으로 복구했다. best rerun은 `session_000000000018a93b053f7dbab8`에서 4/5컷 통과까지 개선됐지만, final requested rerun `session_000000000018a93b5fa88505d4`는 capture 1 `3056ms`로 실패해 Story `1.26`은 계속 `review / No-Go`다.
- 2026-04-24 - latest log review 뒤 warm-up raster를 approved `256x256` lane과 맞추고 preview-only darktable process scheduling priority를 올렸다. 요청 script rerun `session_000000000018a93c85f1238a00`는 `capturesPassed=5/5`, direct band `2819ms ~ 2884ms`를 만들었지만, code review 뒤 official Story `1.26` `Go`가 아니라 comparison evidence로 낮췄다.
- 2026-04-25 - requested script rerun `session_000000000018a959d98b7f93f8`는 capture 1 `3067ms`로 실패했다. preview-only darktable process priority를 Windows high priority로 올린 뒤 requested rerun `session_000000000018a95a0fe32405b8`는 `capturesPassed=5/5`, direct band `2825ms ~ 2873ms`를 만들었다. 이 run은 latency tail 개선 evidence로 기록하고 Story `1.26` 제품 판정은 `in-progress / No-Go`로 유지한다.
- 2026-04-27 - latest review follow-up 중 false `Go` 차단과 validation stability 항목을 처리했다. 단, main close owner가 host-owned reserve path와 hardware ledger `Go`를 아직 닫지 못했으므로 Story `1.26`은 `in-progress / No-Go`로 유지한다.
- 2026-04-27 - 요청한 hardware validation script를 먼저 실행해 `preview-truth-gate-failed`를 확인한 뒤, runner가 route owner evidence까지 검증하게 보강했다. 최종 rerun은 `preview-route-owner-gate-failed`로 닫혀 current darktable path를 official host-owned `Go`로 승격하지 않음을 확인했다.
- 2026-04-27 - 최신 일반 앱 로그를 재검토해 host-owned `preset-applied-preview` handoff 자체가 없음을 확인했다. hardware validation runner가 darktable fallback 전 reserve input을 먼저 검사하게 보강했고, 최종 요청 rerun은 `preview-host-owned-reserve-unavailable`로 닫혔다.
- 2026-04-27 - host-owned reserve input 확인에 bounded settle wait와 delayed handoff regression test를 추가했다. 요청 script rerun은 `waitTimedOut=false` 상태에서 `windows-shell-thumbnail`만 관찰되어 다시 `preview-host-owned-reserve-unavailable`로 닫혔다.
- 2026-04-27 - `windows-shell-thumbnail`을 terminal failure처럼 즉시 반환하지 않고 bounded window 끝까지 host-owned handoff를 기다리도록 보강했다. 요청 script rerun은 `waitElapsedMs=1530`, `waitTimedOut=true` 상태에서도 `windows-shell-thumbnail`만 관찰되어 다시 `preview-host-owned-reserve-unavailable`로 닫혔다.
- 2026-04-27 - validation runner가 helper event와 timing route evidence를 함께 읽도록 보강했다. 요청 script rerun `session_000000000018aa1e41fd6f3360`은 `latestPreviewRoute=none`, `waitElapsedMs=1521`, `waitTimedOut=true`로 다시 실패해 Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-27 - validation runner가 speculative preview output/detail/lock evidence까지 기록하도록 보강했다. 요청 script rerun `session_000000000018aa2016a0ccd26c`은 `latestFastPreviewKind=windows-shell-thumbnail`, `speculativeLockPresent=true`, `speculativeOutputReady=false`, `waitElapsedMs=3028`로 실패해 Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-27 - No-Go 반환 시 저장된 capture가 `previewWaiting`에 남는 문제를 수정했다. 요청 script rerun `session_000000000018aa20bd6b9bb82c`은 Story `1.26` 판정은 여전히 `preview-host-owned-reserve-unavailable / No-Go`지만, saved capture는 `previewReady / preset-applied-preview`로 정상 정리됐다.
- 2026-04-27 - No-Go settle 이후 route evidence를 다시 읽어 final failure summary에 반영하도록 보강했다. 요청 script rerun `session_000000000018aa21e80f662534`은 여전히 `preview-host-owned-reserve-unavailable / No-Go`지만, summary가 fallback route `darktable-cli / program-files-bin / elapsedMs=3011`을 남겨 남은 blocker가 host-owned reserve artifact 부재임을 더 명확히 했다.
- 2026-04-27 - No-Go settle 전후 evidence 보존을 보강하고 요청한 hardware validation script를 재실행했다. `hardware-validation-run-1777271169308` / `session_000000000018aa22b64c44f7a4`는 helper `windows-shell-thumbnail`, `speculativeLockPresent=true`, final fallback route `darktable-cli / program-files-bin / elapsedMs=3136`, official timing `3150ms`로 실패해 Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-27 - 요청한 hardware validation script를 다시 실행했다. `hardware-validation-run-1777272273229` / `session_000000000018aa23b7531de818`도 helper `windows-shell-thumbnail`, `speculativeLockPresent=true`, fallback route `darktable-cli / program-files-bin / elapsedMs=3163`, official timing `3179ms`로 실패했다. 반복 증거 기준으로 남은 제품 blocker는 host-owned reserve artifact 부재다.
- 2026-04-27 - 최근 일반 앱 로그와 요청한 hardware validation script를 다시 확인했다. app session `session_000000000018a9e0f606e69ed0`와 requested run `hardware-validation-run-1777272846171` / `session_000000000018aa243cb94f60f0` 모두 helper `windows-shell-thumbnail` 뒤 per-capture `darktable-cli` fallback으로 닫혔다. latest requested run은 warm-up/readiness/save/cleanup은 정상이나 fallback `elapsedMs=7426`, official timing `7505ms`로 실패해 Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-27 - speculative route failure summary를 정규화해 darktable fallback과 host-owned close owner 차이를 더 명확히 기록하도록 보강했다. 요청한 hardware validation script 재실행 `hardware-validation-run-1777274530300` / `session_000000000018aa25c4d6f49e20`도 helper `windows-shell-thumbnail`, pre-settle `speculativeLockPresent=true`, final fallback `darktable-cli / elapsedMs=3325`로 실패해 Story `1.26`은 계속 `in-progress / No-Go`다.
- 2026-04-29 - false-Go correction 이후 방향을 확정했다. Partial native approximation은 comparison-only로 유지하고, 다음 구현은 option 2인 resident/long-lived darktable-compatible full-preset engine path로 진행한다.
- 2026-04-29 - option 2 구현 뒤 approved hardware validation `hardware-validation-run-1777434275752`가 `5/5` 통과했다. Story `1.26`의 현재 정답은 resident darktable-compatible full-preset route이며, native approximation과 per-capture fallback은 official truth가 아니다.
- 2026-04-29 - code review option 1 patch 뒤 `hardware-validation-run-1777434275752`의 `Go` evidence를 retracted 처리했다. 현재 코드는 per-capture `darktable-cli`와 metadata-only `preset-applied-preview`가 official `previewReady` / `Go` evidence를 만들지 못하게 막고, Story `1.26`은 real resident full-preset owner 구현 전까지 `in-progress / No-Go`로 유지한다.
- 2026-04-29 - 사용자 제품 판단에 맞춰 route contract를 정정했다. per-capture darktable full-preset route는 resident로 오표기하지 않고 `engineMode=per-capture-cli`로 기록하며, metadata-only truth close는 계속 차단한다. 요청 hardware validation `hardware-validation-run-1777442288984` / `session_000000000018aabe5833c11d8c`는 `5/5` 통과했고 official timing band는 `2387ms ~ 2480ms`다. Story `1.26`은 ledger 기준 `Go`다.
