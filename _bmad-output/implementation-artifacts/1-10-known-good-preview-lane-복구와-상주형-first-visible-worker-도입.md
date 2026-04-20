# Story 1.10: known-good preview lane 복구와 상주형 first-visible worker 도입

Status: done

Correct Course Note: 2026-04-04 승인된 sprint change proposal에 따라, Story 1.9는 `review / No-Go` 상태로 유지하고 Story 1.10이 다음 truth-critical corrective follow-up을 소유한다. 이번 스토리의 목적은 UI 표현을 다시 만지는 것이 아니라, booth hardware에서 검증된 known-good preview invocation으로 correctness를 복구하고, per-session seam 계측을 다시 닫으며, first-visible 경로를 per-capture one-shot spawn이 아닌 상주형 worker 중심 topology로 승격하는 것이다.

## Closed Role In This Worktree

- `2026-04-20` 기준 이 문서는 active implementation restart를 지시하는 문서가 아니다.
- 현재 이 worktree는 older `resident first-visible` line을 다시 검증하는 `validation candidate spec`이 아니라, 이미 닫힌 closed `No-Go` baseline record로 읽어야 한다.
- 이 lane은 historically better customer-perceived speed를 보였기 때문에 다시 보는 것이며, current official release gate를 이미 닫았기 때문에 돌아온 것이 아니다.
- current official release judgment는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`, 즉 `preset-applied visible <= 3000ms` 하나뿐이다.
- `sameCaptureFullScreenVisibleMs`와 first-visible 수치는 계속 남기되, reference / comparison / feel metric으로만 읽는다.
- GPU-enabled acceleration on this lane은 optional side evidence일 뿐, lane을 자동으로 승격시키는 성공 보장이 아니다.
- 아래 구현 체크리스트와 기록은 historical trace다. current purpose는 old lane을 release-proof로 오해하지 않고 closed baseline으로 고정하는 데 있다.
- current route reading은 `1.30 = official No-Go evidence`, `1.31 = unopened`, `1.26 = officially opened reserve path`다.

### Current Closure Readout

1. old `resident first-visible` lane의 CPU baseline package는 latest approved hardware session으로 다시 닫혔다.
2. official `preset-applied visible <= 3000ms` gate는 여전히 실패했다.
3. 이 story는 release winner candidate가 아니라 closed `No-Go` baseline으로 확정한다.
4. 다음 active path는 Story `1.26 reserve path`다.

### Canonical Reading Order

- 이 story만 단독으로 읽지 않는다.
- current direction은 먼저 `docs/README.md`, `docs/runbooks/current-preview-gpu-direction-20260419.md`, `docs/runbooks/current-actual-lane-handoff-20260419.md`를 읽고 해석한다.
- `_bmad-output` 문서는 current route를 보조 설명하는 planning/trace artifact로 읽는다.

### Validation Gate Reference

- Supporting evidence family:
  - `HV-05` truthful `Preview Waiting -> Preview Ready`
  - approved booth hardware latency package
  - per-session seam log package (`request-capture -> file-arrived -> fast-preview-visible -> preview-render-start -> capture_preview_ready -> recent-session-visible`)
- Rerun execution reference:
  - `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`
- Current hardware gate: `No-Go`
- Official verdict owner:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Close policy:
  - automated pass만으로 닫지 않는다.
  - latest approved booth session 1개만 봐도 first-visible lane과 later render-backed truth lane을 같은 세션 경로에서 다시 닫을 수 있어야 한다.
  - current official release judgment는 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`, 즉 `preset-applied visible <= 3000ms`다.
  - preview lane correctness, replacement close, queue saturation fallback, `Preview Waiting` truth 유지가 함께 증명돼야 한다.
  - `sameCaptureFullScreenVisibleMs`와 historically better first-visible / replacement comparison numbers는 validation priority를 높여 주는 reference 근거일 뿐, current release-proof로 읽으면 안 된다.
  - GPU 활성/가속은 optional acceleration hypothesis로만 다루며, lane의 공식 합격선을 바꾸지 않는다.
  - 이 문서나 rerun note가 `Go`처럼 보이는 표현을 남기더라도 공식 판정은 ledger row가 소유한다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
방금 찍은 사진이 제품 기준에 맞는 속도로 최근 세션에 나타나길 원한다.
그래서 저장 성공 이후 긴 blank wait나 불안정한 preview replacement 없이 믿고 다음 촬영을 이어갈 수 있다.

## Current Validation Questions

1. 이 old lane이 실제 approved hardware에서 `빠른 first-visible + later truthful close` 구조를 current contract 그대로 다시 만들 수 있는가
2. 그 결과가 current official `preset-applied visible <= 3000ms` gate에 얼마나 가까운가, 그리고 어디서 여전히 멀어지는가
3. GPU-enabled acceleration이 있다면 first-visible과 truthful close를 함께 줄이는 데 실제 도움이 되는가
4. historical better run이 current release-proof가 아니라는 경계를 깨지 않고도, 다음 실험 범위를 더 좁힐 수 있는가

## Acceptance Criteria

1. booth preview lane이 approved render baseline에서 시작할 때 runtime의 기본 preview invocation은 booth hardware에서 검증된 known-good contract를 사용해야 한다. 별도 승인 전까지 speculative 또는 experimental flag는 기본 booth path에서 제외돼야 한다.
2. booth가 새 capture request를 기록할 때 같은 세션의 diagnostics에는 `request-capture`, `file-arrived`, `fast-preview-visible` 또는 동등 first-visible event, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`가 남아야 한다. 이 이벤트는 mixed global log 없이도 한 recent hardware session에서 latency split을 닫는 데 사용할 수 있어야 한다.
3. 같은 booth session에서 반복 촬영용 first-visible preview work가 필요할 때 runtime은 per-capture one-shot render spawn보다 warm 상태를 유지하는 resident first-visible worker를 우선 사용해야 한다.
4. preset 선택 또는 세션 시작은 preview worker warm-up, preset preload, cache priming을 트리거할 수 있지만 capture truth를 막아서는 안 된다.
5. current capture의 first-visible image가 preset-applied render completion 전에 고객에게 보이더라도 canonical preview path와 same-slot replacement 규칙은 유지돼야 하며, `previewReady` truth는 계속 later render-backed booth-safe preview만 소유해야 한다.
6. first-visible source는 hardware 상태와 path health에 따라 fast preview, camera thumbnail, intermediate preview, resident-worker output 중 아무 approved same-capture source든 선택할 수 있다. 다만 booth customer experience는 preset-applied preview readiness가 실제로 닫히기 전까지 truthful `Preview Waiting`을 유지해야 한다.
7. resident worker가 warm state를 잃거나 queue saturation에 빠지거나 안전한 first-visible result를 만들지 못해도 capture truth는 보존돼야 한다. booth는 false-ready나 cross-session leakage 대신 truthful `Preview Waiting`과 normal render follow-up path로 안전 fallback 해야 한다.

## Historical Implementation Checklist (Trace Only)

이 섹션은 `2026-04-04` 시점의 implementation trace를 보존하기 위한 것이다.
현재 worktree에서는 자동 재개용 구현 계획이 아니라, old lane이 어떤 가정과 경계 위에 서 있었는지 확인하는 근거로만 읽는다.

- [x] known-good preview invocation baseline을 고정한다. (AC: 1, 5, 6, 7)
  - [x] `src-tauri/src/render/mod.rs` 안에 흩어진 preview invocation 조건을 booth hardware validated baseline으로 재정렬하고, 기본 preview lane에서 허용되는 flag/argument/source policy를 한 곳에서 판정하게 한다.
  - [x] 실험용 또는 speculative preview invocation은 비교용 fallback으로만 남기고 default booth path에서는 opt-in 없이 켜지지 않게 한다.
  - [x] current capture의 canonical preview path 재사용과 later same-path replacement 규칙이 유지되는지 회귀 테스트를 보강한다.

- [x] resident first-visible worker lifecycle을 도입한다. (AC: 3, 4, 7)
  - [x] session/preset keyed resident preview worker 또는 동등 topology를 도입해 first-visible preview lane이 per-capture one-shot spawn을 기본값으로 사용하지 않게 한다.
  - [x] `src-tauri/src/commands/preset_commands.rs`의 preset selection warm-up과 session-start 경로를 resident worker warm-up / preload / cache priming으로 확장하되 capture blocking이 생기지 않게 한다.
  - [x] worker warm state loss, queue saturation, restart, teardown 기준을 명시하고 recoverable failure를 bounded fallback으로 연결한다.

- [x] capture path를 resident worker 우선 정책으로 연결한다. (AC: 3, 4, 5, 6, 7)
  - [x] `src-tauri/src/commands/capture_commands.rs`와 `src-tauri/src/capture/ingest_pipeline.rs`에서 current preview completion path가 resident worker를 우선 사용하도록 조정한다.
  - [x] current same-capture source 선택 규칙을 fast preview, camera thumbnail, intermediate preview, resident-worker output 우선순위와 health check 기준으로 정리한다.
  - [x] worker miss 또는 unsafe output일 때 기존 truthful `Preview Waiting` + normal render follow-up으로 즉시 내려가고 capture success는 유지되게 한다.

- [x] per-session seam instrumentation을 복구한다. (AC: 2)
  - [x] `src-tauri/src/timing/mod.rs`, `src-tauri/src/commands/runtime_commands.rs`, `src-tauri/src/capture/ingest_pipeline.rs`, 관련 UI emission 경로를 정리해 required seam events가 하나의 session diagnostics path에 빠짐없이 남게 한다.
  - [x] `requestId`, `captureId`, `sessionId` 상관키가 first-visible / render-ready / recent-session-visible까지 일관되게 이어지도록 보강한다.
  - [x] mixed global log를 다시 합치지 않고도 latest approved hardware session 1개만으로 latency split을 닫을 수 있게 진단 패키지를 정리한다. current rerun checklist와 package definition은 `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`를 기준으로 사용한다.

- [x] truth ownership과 customer-safe UX를 유지한다. (AC: 5, 6, 7)
  - [x] `previewReady`, `preview.readyAtMs`, related readiness update는 계속 later render-backed booth-safe preview만 올리도록 유지한다.
  - [x] fast preview 또는 resident worker output이 먼저 보이더라도 booth copy는 계속 `Preview Waiting`을 유지하고 false-ready를 만들지 않게 한다.
  - [x] same-slot replacement, current-session isolation, wrong-capture/wrong-session discard 규칙을 다시 검증한다.

- [ ] regression test와 hardware validation package를 준비한다. (AC: 1, 2, 3, 4, 5, 6, 7)
  - [x] Rust integration test에 resident worker warm hit / cold fallback / queue saturation / warm-state loss / canonical same-path replacement / cross-session isolation 시나리오를 추가한다.
  - [x] UI/provider regression에 `Preview Waiting` truth 유지, `recent-session-visible` logging, same-slot replacement continuity를 추가한다.
  - [ ] approved booth hardware에서 first-visible latency, later preset-applied readiness, seam log close, replacement correctness를 한 패키지로 다시 수집한다. 현재 rerun 직전 실행 계획은 `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`에 고정한다.

### Review Findings

- [x] [Review][Patch] known-good booth preview baseline에 실장비 비호환 `--disable-opencl`이 남아 있음 [src-tauri/src/render/mod.rs:87]
- [x] [Review][Patch] default booth preview source policy가 여전히 `FastPreviewRaster`를 우선해 speculative lane을 운영 경로에 남김 [src-tauri/src/render/mod.rs:1325]
- [x] [Review][Patch] resident worker enqueue 실패 시 per-capture one-shot fallback 없이 first-visible 시도가 바로 소실됨 [src-tauri/src/capture/ingest_pipeline.rs:220]
- [x] [Review][Patch] preview warm-up이 단일 슬롯 resident queue를 공유해 첫 실제 capture render를 saturation으로 밀어낼 수 있음 [src-tauri/src/render/mod.rs:33]
- [x] [Review][Patch] speculative first-visible 이후 preview refinement guard가 `previewWaiting` capture를 바로 반환시켜 truthful preview close가 멈출 수 있음 [src-tauri/src/capture/ingest_pipeline.rs:539]
- [x] [Review][Patch] readiness 재조회 실패 시 current capture가 아직 `previewWaiting`이어도 UI fallback이 `previewReady`를 내보내 false-ready를 만들 수 있음 [src-tauri/src/commands/capture_commands.rs:197]
- [x] [Review][Patch] story가 요구한 per-session seam event 복구가 이번 diff에서 아직 구현되지 않음 [src-tauri/src/capture/ingest_pipeline.rs:1369]
- [x] [Review][Patch] resident/speculative preset-applied preview가 canonical slot 교체 후에도 truthful `previewReady`를 닫지 못함 [src-tauri/src/capture/ingest_pipeline.rs:707]
- [x] [Review][Patch] sprint-status `last_updated`가 같은 날 더 이른 시각으로 되돌아가 revalidation 추적 순서를 흐림 [_bmad-output/implementation-artifacts/sprint-status.yaml:41]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 Story 1.9의 “same-capture first-visible image를 먼저 보여준다”는 방향을 뒤집지 않는다.
- 대신 실장비에서 흔들린 preview lane correctness를 known-good baseline으로 되돌리고, 제품 기준 미달인 first-visible 속도를 topology 변경으로 해결하는 corrective story다.
- 고객 약속은 그대로다. 먼저 같은 컷이 보일 수는 있지만, preset-applied preview가 닫히기 전까지는 계속 truthful `Preview Waiting`이다.
- 엔진 교체는 아직 범위 밖이다. 이번 단계는 `same engine, different topology`다.
- 다만 `2026-04-19` 현재 이 story의 역할은 active implementation을 밀어붙이는 것이 아니라, old lane을 validation candidate로 다시 읽을 때 필요한 경계와 evidence scope를 잠그는 것이다.

### 왜 이 스토리가 새로 필요해졌는가

- 2026-04-04 실장비 재검증에서 `capture acknowledged -> preview visible` 평균이 약 `9238ms`, warm 구간 최근 3컷도 `7616ms`, `7761ms`, `8189ms`로 제품 기준에 미달했다. [Source: history/recent-session-thumbnail-speed-brief.md]
- 위 수치들은 first-visible / feel metric historical evidence로는 계속 중요하지만, 현재 official `Go / No-Go`는 preset-applied visible 3초 게이트를 ledger에서 닫는지로만 판단한다.
- 일부 컷은 preview 파일과 `fastPreviewVisibleAtMs`가 있었는데도 최종 `session.json`이 `renderStatus=previewWaiting`으로 남아 replacement close 실패가 재현됐다. [Source: history/recent-session-thumbnail-speed-brief.md]
- 같은 증거 묶음에는 `preview-render-queue-saturated`, preview stderr access violation, missing preview file 흔적이 함께 남아 current preview lane이 실험적 invocation 조합에서 불안정해졌다는 신호가 확인됐다. [Source: history/recent-session-thumbnail-speed-brief.md]
- 승인된 2026-04-04 sprint change proposal은 다음 단계로 `known-good correctness 복구 + per-session seam 계측 복구 + 상주형 first-visible worker 설계/도입`을 함께 추진하도록 범위를 재정의했다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260404-010751.md]

### 스토리 기반 요구사항

- PRD는 first-visible current-session image latency와 preset-applied preview readiness latency를 분리해서 측정하라고 요구한다. 공식 제품 게이트는 후자(`preset-applied visible <= 3000ms`)만 사용한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- PRD는 same-capture fast preview를 허용하지만 preview truth를 느슨하게 만들면 안 된다고 명시한다. [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- Architecture는 preview lane을 `first-visible lane`과 `truth lane`으로 나누고, approved same-capture source와 resident low-latency worker를 허용한다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- Render Worker 계약은 known-good contract, resident worker 우선, same-path replacement, seam event 집합을 이미 baseline으로 고정했다. [Source: docs/contracts/render-worker.md]
- UX는 first-visible source가 바뀌어도 고객 경험은 “먼저 같은 컷이 보이고, 나중에 더 정확한 결과로 안정화된다”를 유지해야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]

### 선행 의존성과 구현 순서

- 직접 선행 책임:
  - Story 1.7: helper-backed RAW persistence와 `file-arrived` correlation
  - Story 1.8: render-backed `previewReady` / `finalReady` truth owner
  - Story 1.9: same-capture fast preview handoff, canonical pending preview promotion, split timing basis
- 권장 구현 순서:
  1. known-good invocation baseline과 금지 flag 목록 고정
  2. resident worker lifecycle/warm-up 경계 도입
  3. capture path가 resident worker를 우선 선택하도록 연결
  4. per-session seam event 누락 복구
  5. fallback / same-slot replacement / `Preview Waiting` truth 회귀 잠금
  6. hardware validation package 수집

### 현재 워크스페이스 상태

- `src-tauri/src/render/mod.rs`에는 preview warm-up (`schedule_preview_renderer_warmup_in_dir`)과 fast-preview raster 기반 speculative preview render 경로가 이미 있다.
- `src-tauri/src/commands/preset_commands.rs`는 active preset 선택 시 preview renderer warm-up을 이미 호출한다.
- 반면 `src-tauri/src/commands/capture_commands.rs`는 capture 저장 직후 `thread::spawn`으로 `complete_preview_render_in_dir(...)`를 실행하는 per-capture path를 아직 기본으로 사용한다.
- `src-tauri/src/capture/ingest_pipeline.rs`에는 `fast-preview-visible`, `capture_preview_ready` 기록과 canonical preview promotion 흐름이 이미 존재한다.
- `src-tauri/src/commands/runtime_commands.rs`와 `src-tauri/src/timing/mod.rs`를 통해 `recent-session-visible` 같은 per-session timing event를 남길 기반도 이미 있다.
- 즉 resident worker를 완전히 새로 발명하는 것보다, existing warm-up / queue / timing seams를 default preview topology로 재배선하는 편이 현재 구조와 가장 잘 맞는다.
- 그러나 current worktree에서 이것을 곧바로 다시 구현 재개하라는 뜻은 아니다. 먼저 이 topology를 validation lane으로 재정의하고 current release gate와 비교 가능한 근거를 다시 잠가야 한다.

### 이전 스토리 인텔리전스

- Story 1.9는 first-visible same-capture preview와 later XMP replacement를 도입했지만, 현재 상태는 `review / No-Go`다. 실장비 증거상 official preset-applied-visible gate 미달과 replacement close 실패가 남아 있다. [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- Story 1.9의 핵심 교훈은 “UI가 아니라 preview 생성 topology가 병목”이라는 점이다. 남은 유효한 개선은 option 미세 조정보다 구조 변경이다. [Source: history/recent-session-thumbnail-speed-brief.md]
- Story 1.8은 여전히 render-backed `previewReady` / `finalReady` truth owner다. 1.10도 이 소유권을 절대 건드리면 안 된다. [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- 최근 커밋 패턴은 seam logging, fallback 강화, recent-session latency 보정 순으로 이어졌기 때문에 1.10은 기존 자산을 폐기하지 말고 resident topology로 승격해야 한다.
  - `2026-04-04` `Add session seam logging for thumbnail latency reduction`
  - `2026-04-03` `Reduce recent-session preview latency and capture wait blocking`
  - `2026-04-03` `Record thumbnail validation and ship fast preview fallback`
  - `2026-04-01` `Stabilize booth capture/render flow and baseline docs`

### 구현 가드레일

- default booth preview lane에 experimental flag를 다시 섞지 말 것.
- `previewReady`, `preview.readyAtMs`, `previewVisibleAtMs`의 truth owner를 fast preview 또는 resident worker output으로 옮기지 말 것.
- same-capture correctness가 애매하면 resident worker output도 조용히 버리고 기존 truthful `Preview Waiting`으로 내려갈 것.
- per-session seam log를 mixed global log로 되돌리지 말 것.
- queue saturation, warm-state loss, restart, worker error가 발생해도 cross-session leakage나 false-ready로 복구하면 안 된다.
- customer copy에 darktable, XMP, queue saturation, worker restart 같은 내부 용어를 노출하지 말 것.

### 아키텍처 준수사항

- helper/worker/host 경계는 계속 sidecar + filesystem handoff 중심으로 유지한다. [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- session truth는 `session.json`과 session-scoped filesystem root가 소유한다. [Source: docs/contracts/session-manifest.md]
- render worker는 capture record의 `activePresetId + activePresetVersion`과 pinned darktable `5.4.1`을 계속 사용해야 한다. [Source: docs/contracts/render-worker.md]
- pending fast preview가 이미 canonical preview path에 있더라도 later render-backed output은 같은 canonical path를 재사용해 교체해야 한다. [Source: docs/contracts/render-worker.md]
- front-end는 helper raw message나 worker raw status를 직접 해석하지 않고 host-normalized truth만 소비해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/timing/mod.rs`
  - `src-tauri/src/commands/runtime_commands.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src/session-domain/state/session-provider.tsx`
  - `src/booth-shell/components/SessionPreviewImage.tsx`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
- 새 top-level UX surface나 별도 preview schema를 먼저 만들기보다, existing preview lane과 same-slot replacement 규칙 안에서 해결하는 편이 우선이다.

### UX 구현 요구사항

- `Preview Waiting` copy는 그대로 유지한다. 첫 문장은 저장 완료 사실, 둘째 문장은 확인용 사진 준비 중이어야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- same-capture first-visible image가 먼저 보여도 고객은 같은 컷이 먼저 보이고 나중에 더 정확한 결과로 안정화된다고 이해할 수 있어야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Latest Photo Rail]
- resident worker output이든 camera thumbnail이든 customer-visible slot은 current session latest-photo rail의 같은 자리에서 later booth-safe preview로 자연스럽게 교체돼야 한다.
- fast path miss나 worker failure가 있어도 rail empty fallback과 `Preview Waiting` 안전 문구는 유지돼야 한다.

### 테스트 요구사항

- 최소 필수 테스트:
  - known-good default invocation이 approved baseline 외 flag를 싣지 않는다.
  - preset 선택 또는 세션 시작 뒤 resident worker warm-up이 capture truth를 막지 않는다.
  - capture path가 resident worker hit 시 per-capture one-shot spawn 없이 first-visible result를 제공한다.
  - worker queue saturation, warm-state loss, output invalid, restart 시 truthful fallback이 동작한다.
  - `request-capture`, `file-arrived`, `fast-preview-visible`, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`가 같은 session diagnostics path에 남는다.
  - first-visible source가 바뀌어도 `previewReady`는 later render-backed output만 올린다.
  - cross-session leakage 0, wrong-capture discard, same-slot replacement continuity가 유지된다.

### 최신 기술 / 제품 컨텍스트

- 2026-04-03 internal technical research는 당시 제품의 유력 방향을 `앱 셸 유지 + first-visible 전용 저지연 sidecar/worker`로 정리했고, 그 다음 대안으로 local dedicated renderer, watch-folder bridge, edge appliance를 제시했다. `2026-04-19` 현재는 이것을 current release-proof 정답이 아니라 old lane validation candidate의 historical rationale로 읽어야 한다. [Source: _bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md]
- 2026-04-03 recent-session fast preview research는 current front-end가 이미 pending preview를 natural same-slot replacement로 소비할 수 있으므로, 핵심은 UI 대공사가 아니라 same-capture source를 더 빠르고 안정적으로 공급하는 worker topology라고 정리했다. [Source: _bmad-output/planning-artifacts/research/technical-recent-session-fast-preview-research-2026-04-03.md]
- official darktable 문서와 내부 research는 embedded thumbnail/early preview를 먼저 보여주고 later 정확한 preview로 교체하는 staged preview 패턴이 업계적으로 자연스럽다는 점을 재확인한다. 이 문장은 research artifact의 공식 문서 종합을 요약한 해석이다. [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]
- official Tauri sidecar 문서 기준으로도 외부 바이너리를 bundle하고 Rust host에서 sidecar로 호출하는 패턴은 현재 helper/worker 구조와 충돌하지 않는다. 따라서 이번 스토리에 별도 플랫폼 pivot은 필요 없다. 이 문장은 공식 sidecar 문서를 현재 repo 구조에 적용한 추론이다.
- GPU-enabled acceleration은 current worktree에서 함께 점검할 수 있는 유효한 hypothesis지만, 문서상 공식 성공 보장이나 route promotion 근거로 읽어서는 안 된다.
- 현재 공식 경로 메모는 `1.30`을 bounded `No-Go` evidence로 유지하고, `1.31`은 열지 않으며, Story `1.26 reserve path`를 active route로 연다.

### 금지사항 / 안티패턴

- preview lane instability를 감추려고 recent-session-visible만 빠르게 찍고 실제 canonical preview close를 놓치지 말 것.
- first-visible 속도를 위해 representative preset tile, raw copy, 이전 컷, 다른 세션 컷을 current shot처럼 보이게 만들지 말 것.
- resident worker를 도입하더라도 capture request acknowledgement를 worker 준비 완료까지 묶어 지연시키지 말 것.
- worker miss를 capture failure로 승격하지 말 것.
- session seam 진단이 빠졌는데도 hardware gate를 닫았다고 주장하지 말 것.
- story note만으로 official verdict를 선언하지 말 것. `Go / No-Go`는 ledger row가 먼저다.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.10: known-good preview lane 복구와 상주형 first-visible worker 도입]
- [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Latest Photo Rail]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260404-010751.md]
- [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]
- [Source: _bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md]
- [Source: _bmad-output/planning-artifacts/research/technical-recent-session-fast-preview-research-2026-04-03.md]
- [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: history/recent-session-thumbnail-speed-brief.md]

## Historical Implementation Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-04 01:26:15 +09:00 - `bmad-create-story` workflow 기준으로 config, sprint-status, epics, PRD, architecture, UX, Story 1.9, sprint change proposal, recent session hardware history, current render/capture/timing codepath를 교차 분석했다.
- 2026-04-04 01:26:15 +09:00 - Story 1.10을 `known-good baseline + resident worker topology + per-session seam logging + truthful fallback` 중심의 ready-for-dev guide로 정리했다.
- 2026-04-04 01:48:21 +09:00 - `src-tauri/src/render/mod.rs`, `src-tauri/src/capture/ingest_pipeline.rs`, `src-tauri/src/commands/capture_commands.rs`, `src-tauri/src/commands/session_commands.rs`를 수정해 booth-safe preview invocation baseline, resident first-visible worker topology, truthful fallback/readiness ownership을 코드에 연결했다.
- 2026-04-04 01:48:21 +09:00 - `cargo test --test capture_readiness -- --nocapture --test-threads=1`를 실행해 Rust capture regression 59개가 직렬 실행 기준 모두 통과함을 확인했고, Vitest는 `node_modules` 부재로 실행하지 못했다.
- 2026-04-19 12:45:13 +09:00 - `cargo test -- --nocapture --test-threads=1`, `pnpm test:run`, `pnpm lint`를 실행해 resident worker, seam instrumentation, `Preview Waiting` truth, recent-session continuity 회귀가 현재 worktree에서 모두 통과함을 확인했다.
- 2026-04-19 12:45:13 +09:00 - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의 Story 1.8 corrective gate 문구를 현재 governance regression 기대와 다시 맞췄고, Story 1.10 checklist를 software-verified / hardware-rerun-pending 상태로 정리했다.
- 2026-04-19 22:07:58 +09:00 - approved booth hardware latest session `session_000000000018a7c3f52370b574`를 확인해 one-session baseline evidence package를 수집했다. helper correlation, same-session replacement, `capture-ready` 복귀는 닫혔지만 preset-applied visible은 `8972ms`, `7942ms`, `7967ms`로 official gate 실패를 다시 기록했다.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created
- Story 1.9의 review / No-Go 상태와 실장비 근거를 반영해 1.10 범위를 “미세 조정”이 아니라 “구조 변경”으로 고정했다.
- resident worker를 새 엔진 도입이 아닌 current Tauri sidecar / render worker topology 확장으로 정의해 기존 계약과 충돌하지 않게 정리했다.
- sprint tracking과 story file status를 `ready-for-dev` 기준으로 맞췄다.
- booth-safe preview invocation policy를 한 곳에서 고정하고, first-visible worker가 queue miss 또는 unsafe output일 때도 `Preview Waiting` truth를 유지하도록 fallback을 연결했다.
- capture completion 경로는 resident first-visible output을 canonical preview path에 먼저 올릴 수 있지만, `previewReady` / `preview.readyAtMs` / readiness update는 계속 later render-backed close만 소유하도록 되돌렸다.
- per-session seam logging, resident worker 회귀, `Preview Waiting` truth, recent-session continuity에 대한 Rust/Vitest/lint 검증을 2026-04-19 현재 worktree 기준으로 다시 통과시켰다.
- approved booth hardware rerun package는 `2026-04-19 22:07 +09:00` 최신 세션으로 한 번 수집했다. 다만 이 회차도 official `preset-applied visible <= 3000ms` gate를 닫지 못했기 때문에 이 story는 closed `No-Go` baseline으로 확정한다. current official verdict는 ledger 기준 `No-Go`다.
- `2026-04-20` 기준 이 story는 closed `No-Go` baseline으로 확정했고, active reserve path ownership은 Story `1.26`으로 넘긴다.

### File List

- _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/render-worker.md
- docs/contracts/session-manifest.md
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/commands/capture_commands.rs
- src-tauri/src/commands/session_commands.rs
- src-tauri/src/render/mod.rs
- src-tauri/tests/capture_readiness.rs

### Change Log

- 2026-04-04 - booth-safe preview invocation baseline과 resident first-visible worker topology를 연결하고, preview truth ownership을 later render-backed close로 되돌렸다.
- 2026-04-19 - software regression과 governance ledger를 현재 revalidation context에 다시 맞추고, Story 1.10을 hardware rerun 대기 `review` 상태로 재정리했다.
- 2026-04-19 - preview-track official gate를 `preset-applied visible <= 3000ms` 단일 기준으로 다시 읽고, 이 story를 release-proof가 아닌 baseline evidence lane으로 재해석했다.
- 2026-04-19 - 최신 approved hardware session `session_000000000018a7c3f52370b574`로 baseline evidence package를 실제로 수집했고, official gate fail을 ledger에 반영하는 current `No-Go` evidence로 연결했다.
- 2026-04-20 - Story `1.10`을 closed `No-Go` baseline으로 확정하고, active reserve path ownership을 Story `1.26`으로 넘겼다.
