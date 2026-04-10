# Story 1.11: local dedicated renderer sidecar baseline과 capture-bound preview job protocol 도입

Status: done

Architecture Pivot Note: `epics.md` 본문은 아직 1.11~1.13을 개별 스토리로 재생성하지 않았지만, 2026-04-09 승인된 preview architecture decision과 Story 1.10 handoff에 따라 이번 스토리는 `local dedicated renderer + different close topology`의 첫 단계인 dedicated renderer sidecar baseline과 capture-bound preview job protocol을 먼저 고정하는 범위로 본다.

### Validation Gate Reference

- Supporting evidence family:
  - `HV-05` truthful `Preview Waiting -> Preview Ready`
  - per-session seam log package (`request-capture -> file-arrived -> fast-preview-visible -> preview-render-start -> capture_preview_ready -> recent-session-visible`)
  - dedicated renderer spawn/build proof
- Current hardware gate: `Pending`
- Close policy:
  - automated proof만으로는 release-truth `Go`를 주장하지 않는다.
  - 이번 스토리는 dedicated renderer boundary, packaging, protocol, fallback contract를 닫는 준비 단계다.
  - 실제 booth cutover와 `original visible -> preset-applied visible` close hardware package는 후속 Story 1.13에서 닫는다.

## Story

owner / brand operator로서,
host-owned local dedicated renderer lane를 별도 sidecar/protocol 경계로 먼저 고정하고 싶다.
그래서 preset-applied truthful close가 inline host thread나 ad-hoc spawn에 기대지 않고 capture-bound contract로 안전하게 확장될 수 있다.

## Acceptance Criteria

1. booth runtime은 Canon helper와 별도로 `local dedicated renderer` 실행 경계를 가져야 한다. Tauri packaging과 capability 설정은 dedicated renderer binary를 앱 bundle에 포함하고 host가 승인된 인수로만 spawn할 수 있게 고정되어야 하며, 현재 `core:default`만 있는 booth/operator/authoring capability는 필요한 shell execute allowlist를 포함하도록 보강되어야 한다.
2. host -> dedicated renderer preview job contract는 capture-bound truth를 잃지 않도록 최소 `sessionId`, `requestId`, `captureId`, `presetId`, `publishedVersion`, `darktableVersion`, `xmpTemplatePath`, `previewProfile`, `sourceAssetPath`, canonical preview output path, diagnostics detail path를 포함해 동결되어야 한다. live catalog pointer, display name, current active preset lookup만으로 runtime 해석을 대신하면 안 된다.
3. dedicated renderer는 warm-up, preset preload, queue saturation, restart, invalid output을 typed 상태로 보고할 수 있어야 하지만, capture truth와 preview truth를 섞으면 안 된다. 실제 preset-applied preview file이 canonical path에 안전하게 승격되기 전에는 renderer가 성공 신호를 보내더라도 booth는 `previewReady`를 올리지 않아야 한다.
4. 현재 host inline render path는 한 번에 폐기하지 않는다. Story 1.11 범위에서는 dedicated renderer submission/warm-up/health path를 우선 연결하되, shadow mode 또는 gated path로 시작할 수 있어야 하며, sidecar unavailable / protocol mismatch / bundle resolution failure가 발생하면 기존 truthful `Preview Waiting` + approved fallback path로 안전하게 내려가야 한다.
5. diagnostics와 session contract는 dedicated renderer 경계를 분명히 읽을 수 있어야 한다. `preview-render-start`, `preview-render-ready`, `preview-render-failed`, `preview-render-queue-saturated`, `fast-preview-visible`, `capture_preview_ready`, `recent-session-visible`는 같은 session diagnostics root 아래 남아야 하며, wrong-session / wrong-capture output이나 non-canonical output promotion은 차단돼야 한다.
6. Story 1.11은 packaging/build proof, protocol contract proof, host integration test가 준비되기 전까지 `review` 이상으로 올리면 안 된다. 특히 approved booth hardware cutover 전에는 local dedicated renderer가 실제 `previewReady` truth owner라고 주장하면 안 되며, hardware ledger `Go`는 후속 story에서만 닫는다.

## Tasks / Subtasks

- [x] dedicated renderer sidecar boundary와 bundle baseline을 추가한다. (AC: 1, 4)
  - [x] `src-tauri/Cargo.toml`에 Tauri shell/sidecar 실행에 필요한 plugin 의존성을 추가하고, app bootstrap에서 plugin을 초기화한다.
  - [x] `src-tauri/tauri.conf.json`과 `src-tauri/capabilities/*.json`을 갱신해 dedicated renderer binary를 bundle/externalBin에 포함하고, 승인된 인수만 실행 가능한 allowlist를 정의한다.
  - [x] `sidecar/canon-helper/`와 충돌하지 않는 dedicated renderer 경계(`sidecar/` 하위 sibling 또는 동등 구조)를 만들고, helper와 renderer가 같은 역할을 소유하지 않게 분리한다.

- [x] capture-bound preview job contract를 문서와 DTO로 동결한다. (AC: 2, 5)
  - [x] `docs/contracts/render-worker.md`를 dedicated renderer sidecar 요청/응답/이벤트 계약 기준으로 확장한다.
  - [x] 필요하다면 `sidecar/protocol/examples/`에 preview job request/result 예시를 추가해 helper protocol과 같은 수준의 fixture를 남긴다.
  - [x] `src-tauri/src/contracts/dto.rs`, shared contract schema, Rust domain 타입에서 capture-bound bundle resolution 필드가 같은 의미를 갖도록 맞춘다.

- [x] host-owned dedicated renderer service를 추가한다. (AC: 2, 3, 4, 5)
  - [x] `src-tauri/src/render/` 아래에 sidecar spawn, warm-up, queue, health, fallback을 소유하는 service/module을 분리한다.
  - [x] `src-tauri/src/commands/preset_commands.rs`의 preset selection warm-up과 `src-tauri/src/commands/capture_commands.rs`의 capture 후속 path가 dedicated renderer warm-up / submit API를 통하게 한다.
  - [x] Story 1.10에서 추가된 resident worker, warm-up, queue saturation 규칙은 버리지 말고 dedicated renderer service가 같은 정책을 host-owned boundary에서 계승하도록 정리한다.

- [x] current inline render와 safe fallback을 함께 유지한다. (AC: 3, 4)
  - [x] `src-tauri/src/render/mod.rs`의 inline darktable invocation은 곧바로 삭제하지 말고, sidecar unavailable / protocol invalid / invalid output일 때 fallback owner로 남긴다.
  - [x] dedicated renderer 결과는 canonical preview path validation과 session/capture correlation을 통과한 뒤에만 승격한다.
  - [x] sidecar 결과만으로 `previewReady`, `preview.readyAtMs`, post-end `Completed`를 조기 승격하는 회귀를 막는다.

- [x] diagnostics / seam logging / manifest guardrail을 정렬한다. (AC: 5)
  - [x] `src-tauri/src/timing/mod.rs`, `src-tauri/src/capture/ingest_pipeline.rs`, `src-tauri/src/commands/runtime_commands.rs`를 통해 dedicated renderer 관련 safe event를 같은 session diagnostics root에 남긴다.
  - [x] `docs/contracts/session-manifest.md`와 runtime serialization에서 first-visible, truthful close, preset-applied readiness 의미가 dedicated renderer topology와 충돌하지 않게 유지한다.
  - [x] wrong-session output, stale bundle, non-canonical output promotion을 차단하는 검증을 추가한다.

- [x] regression test와 build proof를 준비한다. (AC: 1, 2, 3, 4, 5, 6)
  - [x] Rust integration test에 sidecar spawn/launch validation, capture-bound preset pinning, queue saturation fallback, invalid output rejection, same-session canonical promotion 시나리오를 추가한다.
  - [x] packaging/build proof에 dedicated renderer binary inclusion, capability allowlist, local launch validation을 포함한다.
  - [x] automated proof와 별도로 후속 Story 1.13 hardware package에 넘길 seam metrics checklist를 남긴다.

### Review Findings

- [x] [Review][Patch] Dedicated renderer launch bypasses the approved Tauri sidecar allowlist [src-tauri/src/render/dedicated_renderer.rs:325]
- [x] [Review][Patch] Real sidecar responses never trigger protocol-mismatch handling because schema and status are not validated at runtime [src-tauri/src/render/dedicated_renderer.rs:262]
- [x] [Review][Patch] Warm-up path always records `fallback-suggested`, so typed warm-up and restart states are never observable from host integration [src-tauri/src/render/dedicated_renderer.rs:207]
- [x] [Review][Patch] Story-marked contract docs were not updated, so the render-worker/session-manifest baseline is not actually frozen in repo docs [C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md:41]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 dedicated renderer를 곧바로 product truth owner로 cutover하는 단계가 아니다.
- 목적은 `host-owned local dedicated renderer`를 별도 sidecar/protocol 경계로 먼저 고정해, 다음 스토리에서 close topology 전환과 hardware cutover를 안전하게 할 수 있게 만드는 것이다.
- 고객 경험 약속은 그대로 유지한다. first-visible은 계속 customer-safe projection이고, preset-applied truthful close만이 `previewReady`를 닫는다.

### 왜 이 스토리가 새로 필요해졌는가

- Story 1.10은 known-good baseline, seam 계측, truthful waiting guard를 보존하는 corrective baseline이었고, dedicated renderer cutover 자체는 후속 1.11~1.13이 소유한다고 명시했다. [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- 2026-04-09 research는 `local dedicated renderer + different close topology`가 목표 KPI 달성 확률이 가장 높은 다음 구조라고 결론냈다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Final Recommendation]
- approved sprint change proposal도 preview track 우선순위를 `dedicated renderer protocol -> close topology 분리 -> hardware validation / cutover` 순으로 재정렬하라고 요구한다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260409-233337.md#A-2.-구현-우선순위-정렬]

### 스토리 기반 요구사항

- PRD는 first-visible과 preset-applied truthful close를 분리해서 계측해야 하고, preview readiness truth를 조기 완화하면 안 된다고 고정한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- Architecture는 preview/render 핵심 구조를 `local dedicated renderer + different close topology`로 채택했고, host-owned local renderer lane이 preset-applied truthful close owner여야 한다고 명시한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture] [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- Render worker 계약은 dedicated renderer만이 truthful `previewReady`와 `preview.readyAtMs`를 올릴 수 있고, same-path replacement와 queue saturation fallback을 지켜야 한다고 요구한다. [Source: docs/contracts/render-worker.md]

### 현재 워크스페이스 상태

- `src-tauri/src/render/mod.rs`에는 resident preview worker, warm-up source, bounded queue, darktable invocation이 이미 host 내부 모듈로 존재한다.
- `src-tauri/src/commands/preset_commands.rs`는 preset selection 시 `schedule_preview_renderer_warmup_in_dir(...)`를 호출하지만, 이는 여전히 host inline render 모듈에 붙어 있다.
- `src-tauri/src/commands/capture_commands.rs`는 capture 저장 후 background thread에서 `complete_preview_render_in_dir(...)`를 직접 호출한다. 즉 별도 dedicated renderer process/service 경계가 아직 없다.
- `src-tauri/tauri.conf.json`에는 현재 `externalBin` 설정이 없고, `src-tauri/Cargo.toml`에도 `tauri-plugin-shell` 의존성이 없다.
- `src-tauri/capabilities/booth-window.json`, `operator-window.json`, `authoring-window.json`은 모두 `core:default`만 허용한다. dedicated renderer sidecar를 공식 bundle boundary로 운영하려면 capability allowlist를 추가해야 한다.
- 현재 sidecar 경계는 `sidecar/canon-helper/`만 존재한다. dedicated renderer는 helper와 다른 truth owner이므로 sibling boundary로 분리하는 편이 자연스럽다.

### 이전 스토리 인텔리전스

- Story 1.8은 render-backed `previewReady` / `finalReady` truth owner를 이미 고정했다. 1.11은 이 truth owner를 약화시키는 것이 아니라, host inline 구현을 sidecar/protocol 경계로 승격시키는 준비 단계여야 한다. [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- Story 1.9는 same-capture first-visible handoff와 canonical preview path replacement를 도입했지만, 고객 약속은 여전히 `Preview Waiting` truth 유지였다. dedicated renderer 도입도 이 약속을 깨면 안 된다. [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- Story 1.10은 resident worker warm-up, queue saturation fallback, seam logging baseline을 이미 만들었다. 1.11은 이를 버리고 새 구조를 발명하기보다, dedicated renderer service가 같은 정책을 계승하도록 만드는 편이 안전하다. [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]

### 구현 가드레일

- dedicated renderer를 도입하더라도 `previewReady`, `preview.readyAtMs`, `finalReady`를 sidecar ack만으로 올리지 말 것.
- sidecar result가 canonical output path, same-session, same-capture, capture-bound preset pinning을 통과하기 전에는 booth-visible success로 승격하지 말 것.
- `local dedicated renderer`와 `canon-helper` 역할을 섞지 말 것. helper는 capture boundary, renderer는 preset-applied close boundary다.
- sidecar unavailable 상황을 capture failure로 승격하지 말 것. 기존 truthful `Preview Waiting` + bounded fallback path가 있어야 한다.
- live catalog pointer나 현재 active preset만으로 이미 저장된 capture render를 재해석하지 말 것.
- 고객 화면에는 darktable, sidecar, queue, protocol, shell permission 같은 내부 용어를 노출하지 말 것.

### 아키텍처 준수사항

- Tauri sidecar는 bundle에 포함된 외부 바이너리를 host가 이름 기반으로 실행하는 구조여야 하며, permission allowlist를 통해 승인된 인수만 실행해야 한다. 이 문장은 Tauri 공식 sidecar 문서를 현재 repo 구조에 적용한 요약이다. [Source: https://v2.tauri.app/ko/develop/sidecar/]
- darktable truth path는 XMP sidecar와 `darktable-cli` headless apply/export 경계를 유지해야 한다. 이 문장은 darktable 공식 문서와 현재 render contract를 합쳐 해석한 것이다. [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/] [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]
- session truth는 계속 `session.json`과 session-scoped filesystem root가 소유한다. dedicated renderer는 그 truth를 계산하는 bounded worker이지, 별도 business truth store가 아니다. [Source: docs/contracts/session-manifest.md]

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/capabilities/booth-window.json`
  - `src-tauri/capabilities/operator-window.json`
  - `src-tauri/capabilities/authoring-window.json`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
- 새로 추가될 가능성이 큰 경로:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/render/dedicated_renderer_protocol.rs`
  - `sidecar/dedicated-renderer/`
  - `sidecar/protocol/examples/preview-render-request.json`
  - `sidecar/protocol/examples/preview-render-result.json`
- dedicated renderer project를 만들더라도 booth UI나 session-domain selector부터 바꾸지 말고, 먼저 host boundary와 contract를 닫는 순서를 유지할 것.

### 테스트 요구사항

- 최소 필수 테스트:
  - Tauri bundle/capability 설정이 dedicated renderer binary를 승인된 인수로만 실행한다.
  - preview job DTO가 capture-bound preset pinning 필드를 모두 포함하고 loss 없이 round-trip 된다.
  - sidecar unavailable / protocol mismatch / invalid output일 때 booth가 기존 truthful fallback으로 내려간다.
  - wrong-session / wrong-capture / stale output이 canonical preview path로 승격되지 않는다.
  - queue saturation과 warm-state loss가 diagnostics에 남고 false-ready를 만들지 않는다.
  - inline render fallback이 남아 있어도 dedicated renderer 경계가 event/log/contract에서 분명히 읽힌다.

### 최신 기술 / 제품 컨텍스트

- 2026-04-09 technical research는 가장 작은 실험 단위로 `captureId + presetVersion + rawPath`를 받아 canonical preview path에 same-capture preset-applied artifact를 생성하는 resident local renderer lane을 제안했다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Smallest Viable Prototype]
- 같은 research는 `watch-folder bridge`와 `lighter truthful renderer`를 코어 구조 우선순위에서 내리고, dedicated local renderer를 먼저 시도하라고 정리했다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Final Recommendation]
- 공식 darktable 문서는 XMP sidecar가 편집 이력의 truth artifact이고 `darktable-cli`가 headless apply/export 경로라는 점을 재확인한다. 이 스토리는 별도 render truth engine을 발명하는 것이 아니라 이 경로를 sidecar boundary로 재배선하는 단계다. [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/] [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]

### Git 인텔리전스

- 최근 5개 commit title:
  - `8c30be7 Improve focus retry guidance`
  - `2c89c40 Finalize thumbnail latency worker updates and docs`
  - `9c56c37 Add session seam logging for thumbnail latency reduction`
  - `b24cfc4 Reduce recent-session preview latency and capture wait blocking`
  - `12309fa Record thumbnail validation and ship fast preview fallback`
- 최근 흐름은 first-visible 보정과 seam logging 강화 쪽이었다. 1.11은 같은 축을 유지하되, 이제 inline host implementation을 product-approved dedicated renderer boundary로 분리하는 전환점으로 읽는 편이 맞다.

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Final Recommendation]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Smallest Viable Prototype]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260409-233337.md#A-2.-구현-우선순위-정렬]
- [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: https://v2.tauri.app/ko/develop/sidecar/]
- [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-10 14:38:38 +09:00 - `bmad-create-story` workflow 기준으로 config, sprint-status, epics, PRD, architecture, UX, Story 1.8/1.9/1.10, 2026-04-09 architecture research, 2026-04-09/10 sprint change proposal, current render/capture/Tauri config를 교차 분석했다.
- 2026-04-10 14:38:38 +09:00 - `1.11` 본문이 planning artifacts에 아직 직접 생성되지 않아, approved architecture pivot의 우선순위(`dedicated renderer protocol -> close topology -> hardware cutover`)를 근거로 스토리 제목과 범위를 복원했다.
- 2026-04-10 14:38:38 +09:00 - current workspace가 `host inline render + resident worker warm-up` 상태이고 dedicated renderer sidecar bundle/capability가 아직 없다는 점을 guardrail과 task에 반영했다.
- 2026-04-10 15:04:00 +09:00 - Tauri shell plugin, externalBin/capability allowlist, dedicated renderer request/result DTO, host-owned fallback service, protocol fixtures, build placeholder baseline을 추가하고 preset/capture command를 새 경계로 연결했다.
- 2026-04-10 15:04:00 +09:00 - `cargo test dedicated_renderer -- --nocapture`와 `npm test -- src/shared-contracts/contracts.test.ts`로 packaging/contract/fallback 검증을 실행했다.
- 2026-04-10 16:05:00 +09:00 - code review patch로 dedicated renderer spawn을 Tauri sidecar API로 정렬하고, protocol mismatch 및 typed warm-up result 검증, 계약 문서 baseline 갱신을 반영했다.

### Completion Notes List

- dedicated renderer sidecar bundle/capability baseline을 추가해 booth/operator/authoring window가 승인된 `--protocol/--request/--result` 인수만 실행 가능한 경계로 고정했다.
- capture-bound preview request/result DTO와 shared schema, protocol fixture를 추가해 `sessionId/requestId/captureId/presetId/publishedVersion/darktableVersion/xmpTemplatePath/previewProfile/sourceAssetPath/canonicalPreviewOutputPath/diagnosticsDetailPath` 계약을 동결했다.
- host-owned `src-tauri/src/render/dedicated_renderer.rs` service를 추가해 warm-up/submit을 shadow baseline으로 연결하고, sidecar unavailable/protocol mismatch/queue saturation/invalid output일 때 기존 truthful inline render로 안전하게 fallback하도록 정렬했다.
- build.rs가 dedicated renderer packaging proof용 placeholder externalBin을 준비하도록 해 Tauri build validation이 dedicated renderer bundle 경계를 실제로 확인하게 맞췄다.
- Rust integration test와 TS contract test를 추가해 sidecar queue saturation fallback, protocol fixture round-trip, capability allowlist, packaging baseline을 검증했다.
- code review 후 dedicated renderer 실행 경로를 bundled sidecar launch로 제한하고, malformed result를 `protocol-mismatch` 또는 `invalid-output`으로 정규화하며, warm-up typed 상태와 계약 문서를 실제 repo baseline으로 맞췄다.

### File List

- _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- sidecar/dedicated-renderer/README.md
- sidecar/dedicated-renderer/boothy-dedicated-renderer-x86_64-pc-windows-msvc.exe
- sidecar/protocol/examples/preview-render-request.json
- sidecar/protocol/examples/preview-render-result.json
- src-tauri/Cargo.lock
- src-tauri/Cargo.toml
- src-tauri/build.rs
- src-tauri/capabilities/authoring-window.json
- src-tauri/capabilities/booth-window.json
- src-tauri/capabilities/operator-window.json
- src-tauri/src/commands/capture_commands.rs
- src-tauri/src/commands/preset_commands.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/lib.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/src/render/mod.rs
- src-tauri/tauri.conf.json
- src-tauri/tests/contracts_baseline.rs
- src-tauri/tests/dedicated_renderer.rs
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/dedicated-renderer.ts
- src/shared-contracts/schemas/index.ts

### Change Log

- 2026-04-10: dedicated renderer sidecar baseline, capture-bound preview protocol, safe fallback service, packaging proof, Rust/TS guardrail tests를 추가하고 Story 1.11 상태를 `review`로 전환했다.
- 2026-04-10: code review patch로 sidecar launch boundary, runtime result validation, warm-up typed state visibility, contract docs baseline을 보강하고 Story 1.11 상태를 `done`으로 갱신했다.
