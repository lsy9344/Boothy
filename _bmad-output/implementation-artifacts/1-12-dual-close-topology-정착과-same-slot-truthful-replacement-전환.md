# Story 1.12: same-capture / preset-applied dual-close topology 정착과 same-slot truthful replacement 전환

Status: review

Architecture Pivot Note: `epics.md` 본문은 아직 1.11~1.13을 개별 스토리로 재생성하지 않았지만, 2026-04-09 승인된 preview architecture decision과 Story 1.11 handoff에 따라 이번 스토리는 `local dedicated renderer + different close topology`의 두 번째 단계인 dual-close product state 정착과 same-slot truthful replacement 전환 범위로 복원한다.

### Validation Gate Reference

- Supporting evidence family:
  - `HV-05` truthful `Preview Waiting -> Preview Ready`
  - capture-correlated seam package (`request-capture -> file-arrived -> fast-preview-visible -> recent-session-pending-visible/current-session-preview-pending-visible -> capture_preview_ready -> recent-session-visible/current-session-preview-visible`)
  - same-slot replacement replay proof
  - dedicated renderer truth-lane candidate / fallback reason proof
- Current hardware gate: `Pass (supporting; Story 1.13 owns release Go)`
- Close policy:
  - automated proof만으로는 release-truth `Go`를 주장하지 않는다.
  - 이번 스토리는 dual-close topology와 same-slot replacement semantics를 제품 상태, UI, 계측에 정착시키는 단계다.
  - 실제 booth-wide cutover와 `original visible -> preset-applied visible` hardware `Go`는 후속 Story 1.13에서 닫는다.

## Story

As a booth customer,
I want 방금 찍은 사진이 먼저 보인 뒤 같은 자리에서 프리셋 적용 결과로 안정적으로 바뀌길 원한다,
so that 긴 blank wait나 썸네일 리셋 없이 지금 세션 사진 준비 과정을 믿고 다음 촬영을 이어갈 수 있다.

## Acceptance Criteria

1. host, session manifest, shared contract, normalized state는 `capture success`, `same-capture first-visible`, `preset-applied truthful close`, `finalReady`를 서로 다른 진실값으로 유지해야 한다. `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs`, canonical preview visibility, customer-facing surface state는 같은 의미로 정렬돼야 하며, first-visible 이벤트만으로 `previewReady` 또는 post-end completion이 올라가면 안 된다.
2. dedicated renderer는 Story 1.11의 submission-only baseline에서 나아가 capture-bound truthful close candidate lane으로 연결돼야 한다. canonical preview path, `sessionId/requestId/captureId` correlation, capture-bound preset pinning, result schema/status 검증을 통과한 same-capture preset-applied output만 `previewReady`와 `preview.readyAtMs`를 닫을 수 있어야 하며, 그 전까지 booth는 `Preview Waiting`을 유지해야 한다.
3. latest photo rail과 current-session preview surface는 dual-close topology를 고객에게 자연스럽게 소비해야 한다. first-visible image는 먼저 보일 수 있지만 같은 capture의 truthful preview가 준비되면 같은 슬롯에서 교체돼야 하며, duplicate slot, thumbnail reset, stale overwrite, wrong-capture backslide, empty-slot flash가 발생하면 안 된다.
4. per-session diagnostics, host log, client timing log는 capture 단위로 `first-visible seam`, `replacement seam`, `original visible -> preset-applied visible`를 다시 읽을 수 있게 남아야 한다. 최소한 lane owner, fallback reason, `first-visible-ms`, `replacement-ms`, `original visible -> preset-applied visible`를 summary 또는 동등한 집계 가능 형태로 확인할 수 있어야 한다.
5. protocol mismatch, queue saturation, renderer restart, warm-state loss, wrong-session output, non-canonical output, stale bundle resolution failure가 발생해도 booth는 false-ready, false-complete, cross-session leakage 없이 approved truthful fallback path로 내려가야 한다. dedicated renderer failure를 capture failure와 동일시하거나 고객 copy에 내부 진단어를 노출하면 안 된다.
6. Story 1.12는 fallback path 제거, booth-wide release truth 선언, hardware ledger `Go`를 소유하지 않는다. Rust contract/integration test, TypeScript contract/state test, React same-slot replacement test, replay 가능한 UI evidence가 준비되기 전까지 `review` 이상으로 올리면 안 되며, release cutover는 Story 1.13 hardware package에서만 닫는다.

## Tasks / Subtasks

- [x] dual-close product state와 manifest timing semantics를 고정한다. (AC: 1, 4)
  - [x] `src-tauri/src/session/session_manifest.rs`, `src-tauri/src/capture/ingest_pipeline.rs`, `src-tauri/src/capture/normalized_state.rs`, `src/shared-contracts/schemas/session-manifest.ts`, `src/shared-contracts/schemas/session-capture.ts`, DTO layer가 `first-visible`과 `truthful close`를 같은 의미로 읽도록 맞춘다.
  - [x] first-visible image가 canonical preview path에 먼저 보이더라도 `renderStatus=previewWaiting`과 `Preview Waiting` surface truth가 조기 승격되지 않게 정리한다.
  - [x] `docs/contracts/session-manifest.md`와 shared contract test가 runtime 의미와 일치하도록 유지한다.

- [x] dedicated renderer truth lane candidate를 guarded topology로 연결한다. (AC: 2, 5, 6)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`의 submission-only baseline을 candidate truthful close path로 승격하되, approved cutover 전 guard와 inline fallback은 남긴다.
  - [x] preview result validation이 schema/status/detail/path/session/request/capture correlation을 실제 런타임에서도 확인하도록 보강한다.
  - [x] `src-tauri/src/render/mod.rs`, `src-tauri/src/commands/preset_commands.rs`, `src-tauri/src/commands/capture_commands.rs`에서 warm-up owner, submit owner, fallback owner 경계를 명확히 분리한다.

- [x] same-slot truthful replacement를 booth surface에 정착시킨다. (AC: 1, 2, 3)
  - [x] `src/capture-adapter/services/capture-runtime.ts`, `src/session-domain/state/session-provider.tsx`, `src/session-domain/selectors/current-session-previews.ts`가 first-visible lane과 truthful close lane을 분리 소비하도록 정리한다.
  - [x] `src/booth-shell/components/LatestPhotoRail.tsx`, `src/booth-shell/components/SessionPreviewImage.tsx`, `src/booth-shell/screens/CaptureScreen.tsx`가 duplicate slot, stale overwrite, thumbnail reset 없이 same-slot replacement를 유지하게 만든다.
  - [x] customer copy는 truthful close 전까지 계속 `Preview Waiting`을 유지하고, active preset visibility와 current-session isolation은 그대로 보존한다.

- [x] seam diagnostics와 summary evidence를 보강한다. (AC: 4, 5)
  - [x] `src-tauri/src/commands/runtime_commands.rs`와 per-session diagnostics root에 first-visible / truthful close / same-slot visible / fallback reason / lane owner가 capture-correlated chain으로 남게 한다.
  - [x] `history/thumbnail-replacement-timing-history.md`가 요구한 `first-visible-ms`, `replacement-ms`, `original visible -> preset-applied visible`를 한 줄로 다시 읽을 수 있는 summary 또는 동등 집계 포인트를 추가한다.
  - [x] operator-safe diagnostics는 dual-close topology를 읽을 수 있어야 하지만 customer-facing surface에는 sidecar, protocol, darktable 같은 내부 용어를 노출하지 않는다.

- [x] regression / contract / replayable verification을 추가한다. (AC: 3, 4, 5, 6)
  - [x] Rust 테스트에 first-visible / truthful close 분리, dedicated renderer schema/status validation, wrong-session rejection, non-canonical replacement blocking, fallback continuity를 추가한다.
  - [x] TypeScript/React 테스트에 same-slot replacement continuity, preview-waiting truth 유지, latest slot의 truthful replacement visibility event를 추가한다.
  - [x] replay 가능한 UI evidence를 남긴다. Playwright trace 또는 동등한 재생 가능한 증거가 가능하면 채택하고, 현재 스택에서 불가능하면 Story 1.13 hardware package로 넘길 gap을 명시한다.

### Review Findings

- [x] [Review][Patch] Story 1.12 tracking is marked `done` even though the story gate and completion notes still say unresolved review work keeps it `in-progress` [_bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md:3]
- [x] [Review][Patch] Partial authoring workspace merge can keep repaired `folder-mismatch` invalid draft entries visible because it compares `draftFolder` to `presetId` as if they were the same identifier [src/preset-authoring/screens/PresetLibraryScreen.tsx:230]
- [x] [Review][Patch] `stage-unavailable` publication rejections now allow a `published` draft without requiring proof that the draft had already been published before the rejection [src/shared-contracts/schemas/preset-authoring.ts:690]
- [x] [Review][Patch] app handle이 없는 경로에서 이전 dedicated renderer 결과 파일을 재사용할 수 있음 [src-tauri/src/render/dedicated_renderer.rs:249]
- [x] [Review][Patch] `capture_preview_transition_summary`가 `original visible -> preset-applied visible` 지표를 더 이상 남기지 않음 [src-tauri/src/render/dedicated_renderer.rs:554]
- [x] [Review][Patch] 진단 새로고침 실패 후 마지막 세션 문맥을 보여주면서도 recovery action을 모두 막아 운영 복구가 멈춤 [src/operator-console/providers/operator-diagnostics-provider.tsx:126]
- [x] [Review][Patch] 현재 1.12 후보 변경에는 dual-close 상태 분리와 same-slot surface wiring 핵심 구현이 빠져 있어 AC1/AC3를 충족하지 못함 [_bmad-output/implementation-artifacts/review-1-12.diff:1] — selector/UI regression과 2026-04-10 5컷 실장비 세션 증거로 close
- [x] [Review][Patch] 현재 1.12 후보 변경에는 replay 가능한 UI evidence와 스토리 게이트 증적 패키지가 없어 AC6를 충족하지 못함 [_bmad-output/implementation-artifacts/review-1-12.diff:1] — replayable UI evidence는 2026-04-11 사용자 승인으로 waived/pass 처리

### Hardware Validation Excerpt

- 2026-04-10 실장비 세션 `session_000000000018a5007b5fecf020`에서 5컷 촬영 후 `lifecycle.stage=completed`와 반복 `post-end-evaluated state=completed variant=local-deliverable-ready`를 확인했다.
- 같은 세션의 `capture_preview_transition_summary`에는 “처음 보인 시점에서 프리셋 적용 결과로 바뀌기까지 걸린 시간”이 `replacementMs`로 실제 기록됐다.
- 5컷 발췌값: `3694ms`, `3451ms`, `3852ms`, `3615ms`, `3707ms`
- 같은 로그 줄에는 `firstVisibleMs=2935/2819/2827/2810/3110`도 함께 남아 있어 first-visible seam과 replacement seam을 같이 읽을 수 있다.
- 같은 세션의 `recent-session-pending-visible -> capture_preview_ready -> recent-session-visible` 연쇄와 사용자 현장 확인을 함께 보면, latest slot이 빈칸 없이 same-slot 교체로 닫혔다는 supporting proof로 사용 가능하다.
- 별도 필드명 `originalVisibleToPresetAppliedVisibleMs`가 이 legacy 세션에서는 `none`으로 남았지만, 2026-04-11 코드/테스트 보정으로 동등 지표 회귀는 잠갔다.
- replay 가능한 UI evidence 요구는 2026-04-11 사용자 승인으로 waived/pass 처리한다.

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 dedicated renderer를 booth-wide release truth owner로 최종 cutover하는 단계가 아니다.
- 목적은 `different close topology`를 제품 상태, session manifest, booth surface, diagnostics에 실제로 정착시켜 `first-visible`과 `preset-applied truthful close`가 더 이상 한 경로로 섞이지 않게 만드는 것이다.
- 고객 경험 약속은 유지한다. 먼저 같은 촬영이 보일 수 있지만, truthful close 전까지는 `Preview Waiting`을 유지하고 나중에 같은 슬롯이 preset-applied 결과로 안정화돼야 한다.

### 왜 이 스토리가 새로 필요해졌는가

- Story 1.10은 known-good preview lane, first-visible worker, same-slot replacement baseline을 복구했지만, 다음 구조는 `local dedicated renderer + different close topology`가 소유한다고 명시했다. [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- Story 1.11은 dedicated renderer sidecar boundary와 capture-bound protocol을 먼저 고정했지만, warm-up과 truthful close owner는 아직 inline path에 남아 있다. [Source: _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md]
- 2026-04-09 research는 구현 로드맵을 `different close topology 확정 -> same-capture / preset-applied dual close topology 정착 -> guarded cutover와 hardware validation` 순으로 제시했다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Implementation Roadmap]
- approved sprint change proposal도 preview track 우선순위를 `dedicated renderer protocol -> close topology 분리 -> hardware validation / cutover`로 재정렬하라고 요구한다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260409-233337.md#A-2.-구현-우선순위-정렬]

### 스토리 기반 요구사항

- PRD는 first-visible current-session image와 later preset-applied truthful close를 구분해 측정하고, preset-applied confirmation은 95백분위 5초 이내를 목표로 유지해야 한다고 고정한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- PRD는 capture truth, preview truth, final completion truth를 분리된 진실값으로 유지하라고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- Architecture는 preview pipeline을 `first-visible lane`과 `truth lane`으로 나누고, host-owned local dedicated renderer lane이 preset-applied close owner여야 한다고 명시한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture] [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- UX는 same-capture first-visible image가 먼저 보여도 truthful close가 준비될 때까지 `Preview Waiting`을 유지하고, latest photo rail은 같은 자리 replacement를 지켜야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름] [Source: _bmad-output/planning-artifacts/ux-design-specification.md#촬영 루프 및 실시간 확인 (Capture Loop)]
- Render worker 계약은 first-visible lane과 truthful close owner를 분리하고, same-path replacement와 bounded fallback을 지켜야 한다고 요구한다. [Source: docs/contracts/render-worker.md#Preview 규칙]
- Session manifest 계약은 `fastPreviewVisibleAtMs`와 `xmpPreviewReadyAtMs`를 분리 기록해 dual-close semantics를 유지해야 한다고 고정한다. [Source: docs/contracts/session-manifest.md#필드 규칙]

### 현재 워크스페이스 상태

- `src-tauri/src/render/dedicated_renderer.rs`는 warm-up 결과를 항상 `fallback-suggested`로 돌려주며, preview submit도 결과 파일이 없으면 `shadow-submission-only`로 내려가도록 남아 있다. 즉 Story 1.11 baseline은 아직 submission-only에 머문다.
- 같은 파일은 sidecar spawn을 `std::process::Command` 직접 호출로 수행한다. approved Tauri sidecar allowlist와 실제 실행 경로를 일치시키는 검증은 아직 미완이다.
- `src-tauri/src/capture/ingest_pipeline.rs`에는 canonical preview path에 first-visible asset이 생기면 `render_status = previewReady`로 승격하는 경로가 남아 있어, dual-close semantics를 다시 흐릴 위험이 있다.
- `src/capture-adapter/services/capture-runtime.ts`와 `src/session-domain/state/session-provider.tsx`는 `latestCapture.renderStatus === 'previewReady'`를 customer surface `previewReady` 판단의 핵심 신호로 사용한다.
- `src/booth-shell/components/LatestPhotoRail.test.tsx`는 truthful replacement 시 `recent-session-visible`을 다시 남기는 패턴을 이미 갖고 있으므로, 새 UI surface를 발명하기보다 existing same-slot replacement contract를 강화하는 편이 안전하다.
- 현재 워크트리는 Story 1.11, foundational contract stories, release baseline 관련 대규모 미커밋 변경을 포함한다. 1.12 구현은 이 변경을 되돌리지 말고, render/capture/session/UI 경계를 읽은 뒤 충돌 없이 좁게 얹어야 한다.

### 이전 스토리 인텔리전스

- Story 1.11 review finding에 따르면 dedicated renderer launch는 승인된 Tauri sidecar allowlist를 우회하고 있고, real sidecar response는 schema/status를 런타임에서 충분히 검증하지 않으며, warm-up path는 항상 `fallback-suggested`만 남긴다. 1.12는 이 미해결점을 전제로 진행하면 안 된다. [Source: _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md]
- Story 1.10은 same-slot replacement continuity, wrong-capture discard, `Preview Waiting` truth 유지, resident worker warm-up을 이미 baseline으로 만들었다. 1.12는 새 토폴로지를 도입하더라도 이 guardrail을 버리면 안 된다. [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- Story 1.9는 latest hardware evidence 기준 `review / No-Go` 상태로 남아 있고, 미세 조정만으로는 목표를 맞추기 어렵다는 결론을 남겼다. 1.12는 다시 fast thumbnail-only corrective로 회귀하면 안 된다. [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]

### 구현 가드레일

- first-visible image가 canonical preview path에 먼저 존재하더라도, same-capture preset-applied output validation 전에는 `previewReady`, `preview.readyAtMs`, `Completed`를 올리지 말 것.
- dedicated renderer candidate lane과 inline truthful fallback lane을 동시에 truth owner로 취급하지 말 것. 한 capture에는 하나의 truthful close owner만 있어야 한다.
- wrong-session, wrong-request, wrong-capture, non-canonical output, stale bundle을 고객-visible success로 승격하지 말 것.
- same-slot replacement는 “같은 촬영의 같은 자리 교체”여야지, 슬롯 추가/삭제/초기화/뒤바뀜이 되면 안 된다.
- customer surface에는 sidecar, protocol, darktable, queue saturation, restart 같은 내부 용어를 노출하지 말 것.
- 1.13 hardware cutover 전에는 fallback path를 삭제하거나 release truth를 주장하지 말 것.

### 아키텍처 준수사항

- Tauri v2 공식 sidecar 문서는 `externalBin` 번들링, shell plugin 초기화, `app.shell().sidecar(name)` 호출, capability allowlist와 인수 검증을 기준 경로로 설명한다. 2026-04-10 기준 확인한 공식 문서를 현재 repo 구조에 적용한 해석이다. [Source: https://v2.tauri.app/ko/develop/sidecar/]
- darktable 공식 문서는 `darktable-cli`가 headless export 경로이고, XMP sidecar가 편집 이력 truth artifact라고 설명한다. dedicated renderer는 새 truth engine이 아니라 이 경로를 local-first topology로 재배선하는 bounded worker여야 한다. [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/] [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- research는 trace-based replay proof가 close topology regressions를 읽는 데 유효하다고 본다. 현재 repo에 Playwright가 없더라도, replay 가능한 UI evidence를 남기는 방향은 유지해야 한다. 이 문장은 research와 Playwright 공식 trace viewer 문서를 함께 적용한 해석이다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Development Workflows and Tooling] [Source: https://playwright.dev/docs/trace-viewer]

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/commands/preset_commands.rs`
  - `src-tauri/src/commands/runtime_commands.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src/shared-contracts/schemas/session-manifest.ts`
  - `src/shared-contracts/schemas/session-capture.ts`
  - `src/shared-contracts/dto/capture.ts`
  - `src/capture-adapter/services/capture-runtime.ts`
  - `src/session-domain/state/session-provider.tsx`
  - `src/session-domain/selectors/current-session-previews.ts`
  - `src/booth-shell/components/LatestPhotoRail.tsx`
  - `src/booth-shell/components/SessionPreviewImage.tsx`
  - `src/booth-shell/screens/CaptureScreen.tsx`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
- 새로 추가될 가능성이 큰 경로:
  - `src-tauri/tests/dedicated_renderer_dual_close.rs` 또는 동등 테스트 파일
  - `src/booth-shell/components/__trace__/*` 또는 동등 replay artifact path
  - `tests/hardware/dual-close-topology/*` 또는 Story 1.13 handoff용 evidence path
- 별도 UI surface나 새 세션 저장소를 만들기보다, existing preview lane / canonical preview path / current-session rail 구조 안에서 topology semantics를 정리하는 편이 우선이다.

### 테스트 요구사항

- 최소 필수 테스트:
  - first-visible asset이 먼저 보이더라도 truthful close 전에는 `renderStatus=previewWaiting`과 customer `Preview Waiting`이 유지된다.
  - dedicated renderer result schema/status/path/session/request/capture validation이 실패하면 inline fallback으로 내려간다.
  - same-slot replacement가 duplicate slot, stale overwrite, thumbnail reset 없이 동작한다.
  - wrong-session / wrong-capture / non-canonical output / stale bundle이 canonical preview path를 오염시키지 않는다.
  - `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs`, visible event chain에서 `first-visible-ms`, `replacement-ms`, `original visible -> preset-applied visible`를 다시 계산할 수 있다.
  - fallback path가 남아 있어도 customer-facing copy는 plain-language를 유지하고, operator-safe diagnostics는 close topology를 읽을 수 있다.
- 권장 추가 검증:
  - guarded dedicated renderer candidate lane이 실제로 `original visible -> preset-applied visible`를 줄였는지 hardware trace package에서 비교
  - replay 가능한 UI evidence로 latest slot replacement 시점을 재생

### 최신 기술 / 제품 컨텍스트

- 2026-04-09 research는 `different close topology`를 후보가 아니라 사실상 필수 보완 원칙으로 본다. first-visible과 preset-applied truthful close를 분리해야 현재 병목을 직접 겨냥할 수 있다고 결론냈다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Design Principles and Best Practices] [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technical Research Conclusion]
- 같은 research는 `resident local renderer + warm state + cache priming + dual close topology` 조합이 실장비 성공 확률이 가장 현실적이라고 정리했다. [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Scalability and Performance Patterns]
- 공식 Playwright trace viewer 문서는 저장된 trace를 로컬 CLI나 `trace.playwright.dev`에서 열어 CI 실패와 UI state 전환을 재생할 수 있다고 설명한다. UI replay proof가 가능하다면 same-slot replacement regressions를 읽는 데 유용하다. [Source: https://playwright.dev/docs/trace-viewer]

### Git 인텔리전스

- 최근 5개 commit title:
  - `8c30be7 Improve focus retry guidance`
  - `2c89c40 Finalize thumbnail latency worker updates and docs`
  - `9c56c37 Add session seam logging for thumbnail latency reduction`
  - `b24cfc4 Reduce recent-session preview latency and capture wait blocking`
  - `12309fa Record thumbnail validation and ship fast preview fallback`
- commit history 기준으로는 아직 thumbnail latency seam, first-visible worker, fallback correction 축이 가장 최근에 닫혔다.
- 반면 현재 워크스페이스에는 committed history에 아직 없는 Story 1.11 baseline 파일과 dedicated renderer 관련 untracked/modified 변경이 존재한다. 1.12 implementer는 “mainline commit history”와 “current workspace candidate state”가 어긋난다는 점을 알고 작업해야 한다.

### References

- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Initial Implementation Priorities]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#촬영 루프 및 실시간 확인 (Capture Loop)]
- [Source: docs/contracts/render-worker.md#Preview 규칙]
- [Source: docs/contracts/session-manifest.md#필드 규칙]
- [Source: history/thumbnail-replacement-timing-history.md]
- [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- [Source: _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar-baseline과-capture-bound-preview-job-protocol-도입.md]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Implementation Roadmap]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-preset-applied-preview-architecture-research-2026-04-09.md#Technical Research Conclusion]
- [Source: https://v2.tauri.app/ko/develop/sidecar/]
- [Source: https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/]
- [Source: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/]
- [Source: https://playwright.dev/docs/trace-viewer]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-10 15:33:55 +09:00 - `bmad-create-story` workflow 기준으로 config, sprint-status, epics, PRD, architecture, UX, Story 1.9/1.10/1.11, 2026-04-04/09/10 sprint change proposal, thumbnail replacement history, current render/capture/session/UI 코드를 교차 분석했다.
- 2026-04-10 15:33:55 +09:00 - `epics.md`에 1.12 본문이 아직 직접 생성되지 않아, approved architecture pivot 우선순위(`dedicated renderer protocol -> close topology 분리 -> hardware validation / cutover`)와 research roadmap을 근거로 스토리 제목과 범위를 복원했다.
- 2026-04-10 15:33:55 +09:00 - Story 1.11 baseline code가 아직 `submission-only + inline truthful fallback owner` 상태에 머물고, session/UI state가 `previewReady`를 조기 해석할 위험이 있다는 점을 1.12 guardrail과 task에 반영했다.
- 2026-04-10 15:33:55 +09:00 - official Tauri sidecar, darktable, Playwright trace viewer 문서를 확인해 sidecar allowlist, headless truth path, replayable UI evidence 방향을 최신 기준으로 다시 점검했다.
- 2026-04-10 15:59:00 +09:00 - dedicated renderer accepted result가 canonical output을 실제 truthful close owner로 채택하도록 연결하고, accepted without output / invalid output은 inline truthful fallback으로 유지하도록 검증 경로를 강화했다.
- 2026-04-10 15:59:00 +09:00 - `capture_preview_transition_summary` 이벤트를 추가해 lane owner, fallback reason, `first-visible-ms`, `replacement-ms`를 per-session timing log와 host log에서 한 줄로 다시 읽을 수 있게 만들었다.
- 2026-04-10 15:59:00 +09:00 - Rust dedicated renderer tests, shared contract tests, React same-slot replacement/state tests를 다시 실행했고, Playwright trace 부재는 Story 1.13 hardware evidence gap으로 문서에 명시했다.
- 2026-04-10 22:08:00 +09:00 - 실장비 세션 `session_000000000018a5007b5fecf020`를 점검해 5컷 촬영, 5 `capture_preview_ready`, 5 `capture_preview_transition_summary`, 5 `recent-session-visible`, `lifecycle.stage=completed`, 반복 `post-end-evaluated state=completed variant=local-deliverable-ready`를 확인했다.
- 2026-04-10 22:08:00 +09:00 - 같은 세션은 5 originals / 5 previews / 1 final을 남겼지만 `capture_preview_transition_summary`의 `originalVisibleToPresetAppliedVisibleMs`가 여전히 `none`이고 replay 가능한 UI evidence도 없어서 Story 1.12를 `done`으로 닫지는 않았다.
- 2026-04-11 00:00:00 +09:00 - 같은 세션 로그에서 first-visible 이후 preset-applied close 시간은 `replacementMs=3694, 3451, 3852, 3615, 3707`로 실제 기록된 것을 다시 확인했다.
- 2026-04-11 11:24:18 +09:00 - 사용자 승인에 따라 replay 가능한 UI evidence 요구는 waived/pass로 처리했고, 남아 있던 review patch를 닫은 뒤에도 truth-critical gate policy에 따라 Story 1.12 상태는 `review`로 유지한다.

### Completion Notes List

- Story 1.12를 `dual-close topology + same-slot truthful replacement` 단계로 복원했다.
- 1.11의 protocol baseline과 1.13의 hardware cutover 사이에서, 제품 상태/manifest/UI/diagnostics를 먼저 정착시켜야 한다는 guardrail을 명확히 적었다.
- current workspace의 `submission-only dedicated renderer`, 조기 `previewReady` 승격 가능성, same-slot replacement 소비 경로를 구체적 구현 포인트로 정리했다.
- Tauri sidecar, darktable truth path, replayable UI evidence 기준을 최신 공식 문서와 연결했다.
- validated dedicated renderer accepted output이 inline overwrite 없이 same-slot truthful close를 닫도록 연결했다.
- fallback이 발생해도 `capture_preview_transition_summary`에서 lane owner와 fallback reason, seam 수치를 다시 읽을 수 있게 만들었다.
- Rust/TypeScript/React 회귀 검증을 통과했고, replay evidence의 하드웨어 패키지 gap은 Story 1.13 handoff로 남겼다.
- 최신 실장비 세션 `session_000000000018a5007b5fecf020`에서 5컷 촬영 후 completed까지의 흐름은 확인했다.
- 다만 Story 1.12는 supporting hardware pass만 확보했으므로, guarded cutover와 release-truth `Go`를 소유하는 Story 1.13 전까지 상태를 `review`로 유지한다.
- 위 세션에서 사용자가 질문한 “처음 보인 시점에서 프리셋 적용 결과로 바뀌기까지 걸린 시간”은 `replacementMs` 값으로 실제 기록되어 있었다.
- replay 가능한 UI evidence 요구는 사용자 승인으로 pass 처리했고, 남은 close gate는 Story 1.13 hardware validation이다.

### File List

- _bmad-output/implementation-artifacts/1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/session-manifest.md
- history/thumbnail-replacement-timing-history.md
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/dedicated_renderer.rs
