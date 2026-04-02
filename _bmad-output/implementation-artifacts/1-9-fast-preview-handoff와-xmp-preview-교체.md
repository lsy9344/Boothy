# Story 1.9: fast preview handoff와 XMP preview 교체

Status: review

Correct Course Note: 2026-04-02 승인된 sprint change proposal에 따라, Story 1.8은 render-backed `previewReady` / `finalReady` truth owner로 유지하고, Story 1.9는 blank waiting을 줄이기 위한 first-visible same-capture preview latency 보정을 별도 corrective follow-up으로 소유한다. 이 스토리의 목적은 "정식 preview truth를 빠르게 만든다"가 아니라 "정식 preview truth를 느슨하게 만들지 않으면서도 고객이 방금 찍은 shot을 더 빨리 보게 한다"이다.

### Validation Gate Reference

- Supporting evidence family:
  - `HV-05` truthful `Preview Waiting -> Preview Ready`
  - `HV-07` selected preset apply truth 유지
  - approved booth hardware latency package
- Missing canonical close proof:
  - helper fast preview가 same-capture / same-session 정합성을 유지한다는 증거
  - pending fast preview가 보여도 booth state가 계속 truthful `Preview Waiting`으로 남는다는 증거
  - later XMP preview가 같은 canonical path를 교체하고 그때만 `previewReady`가 올라간다는 증거
  - burst capture와 cross-session 상황에서도 잘못된 이미지가 섞이지 않는다는 증거
- Current hardware gate: `Not run`
- Close policy: automated pass만으로 닫지 않는다. canonical hardware evidence는 first-visible fast preview, later XMP replacement, timing split, cross-session isolation을 한 패키지로 묶어야 한다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
정식 preset-applied preview가 아직 준비되지 않았더라도 방금 찍은 사진이 현재 세션에 최대한 빨리 보이길 원한다.
그래서 부스가 "사진은 저장됐다"고 말한 뒤에도 한동안 빈 상태로 기다리는 불안을 겪지 않는다.

## Acceptance Criteria

1. Story 1.7 경로로 active session의 RAW persistence가 성공한 뒤 helper 또는 host가 same-capture fast preview 경로를 제공할 수 있어야 한다. 이 handoff는 optional이어야 하며, fast preview가 없다고 capture success가 실패로 승격되면 안 된다.
2. fast preview handoff가 존재할 때 host는 same-session, same-capture, allowed-path 규칙과 파일 유효성을 검증한 뒤에만 그 자산을 canonical preview 경로 `renders/previews/{captureId}.jpg` 또는 동등 canonical path로 승격할 수 있어야 한다. 이 시점에는 `previewReady`와 `preview.readyAtMs`를 올리면 안 된다.
3. booth가 `Preview Waiting` 상태인 동안 valid한 same-capture fast preview가 이미 canonical preview path에 있으면 latest-photo rail과 confirmation surface는 그 pending preview를 먼저 보여줄 수 있어야 한다. 다만 booth state와 customer copy는 여전히 "확인용 사진 준비 중"을 유지해야 하며, preset-applied booth-safe preview가 이미 준비된 것처럼 보이면 안 된다.
4. Story 1.8 render worker가 later preset-applied preview를 만들면 runtime은 그 결과로 같은 canonical preview path를 교체해야 하며, 그때만 `previewReady`, `preview.readyAtMs`, 관련 readiness update를 기록할 수 있어야 한다.
5. fast preview가 missing, invalid, stale, wrong-session, wrong-capture, 손상 파일, 비허용 경로 등으로 판정되면 host는 그 자산을 조용히 폐기하고 기존 truthful `Preview Waiting` + normal render follow-up으로 안전하게 fallback 해야 한다. 이 경우에도 저장된 RAW와 active session truth는 유지되어야 한다.
6. instrumentation과 diagnostics는 fast preview first-visible과 later preset-applied preview ready를 분리해 기록해야 한다. approved booth hardware 검증에서는 same-capture correctness, burst capture queue delay, cross-session leakage 0, 그리고 `Preview Waiting` copy truthfulness를 함께 증명해야 한다.

## Tasks / Subtasks

- [x] helper/host 계약에 optional fast preview handoff를 추가한다. (AC: 1, 5, 6)
  - [x] `docs/contracts/camera-helper-sidecar-protocol.md`에 `file-arrived` optional metadata로 `fastPreviewPath`, `fastPreviewKind` 또는 동등 필드를 추가하고 backward compatibility 규칙을 명시한다.
  - [x] `src-tauri/src/capture/sidecar_client.rs`의 `CanonHelperFileArrivedMessage`와 `CompletedCaptureRoundTrip`에 optional fast preview 정보를 추가한다.
  - [x] helper가 fast preview를 주지 못해도 기존 RAW-only path가 그대로 동작하도록 keep-compat 경계를 유지한다.

- [x] host fast preview promotion 경로를 구현한다. (AC: 2, 5)
  - [x] `src-tauri/src/capture/normalized_state.rs` 또는 `src-tauri/src/capture/ingest_pipeline.rs`에서 fast preview validate/promote seam을 추가한다.
  - [x] canonical preview path는 existing session root `renders/previews/{captureId}.jpg` 경로를 우선 재사용한다.
  - [x] promote 시 `preview.assetPath`만 채우고 `preview.readyAtMs`는 계속 `null`, `renderStatus`는 계속 `previewWaiting`으로 유지한다.
  - [x] invalid fast preview는 capture success를 깨지 않고 discard + fallback 되게 한다.

- [x] pending preview를 current session UI에 그대로 연결한다. (AC: 2, 3, 5)
  - [x] `seed_pending_preview_asset_path(...)`와 existing pending-preview path를 우선 재사용하고, 별도 second rail schema를 먼저 만들지 않는다.
  - [x] `src/session-domain/selectors/current-session-previews.ts`의 pending preview 조건이 new fast preview path와 자연스럽게 이어지도록 regression을 보강한다.
  - [x] `src/booth-shell/components/SessionPreviewImage.tsx`와 관련 booth surface가 `readyAtMs === null` 상태에서도 same-capture pending preview를 정상 표시하되, customer copy는 `Preview Waiting`으로 유지하도록 한다.

- [x] Story 1.8 render path와 same-path replacement를 연결한다. (AC: 4)
  - [x] `src-tauri/src/render/mod.rs`의 canonical preview output path를 fast preview promote path와 충돌 없이 공유한다.
  - [x] later render-backed output이 pending preview를 같은 canonical path에서 교체하고, 그때만 `previewReady`를 기록하도록 유지한다.
  - [x] Story 1.8의 "render-backed `previewReady` only" 규칙을 절대 느슨하게 만들지 않는다.

- [x] timing / diagnostics 분리를 추가한다. (AC: 6)
  - [x] `timing-events.log` 또는 동등 진단 로그에 `fast-preview-promote-start`, `fast-preview-promoted`, `fast-preview-invalid`, `preview-render-start`, `preview-render-ready`, `preview-render-failed`, `preview-render-queue-saturated`를 기록한다.
  - [x] current capture timing에는 `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs` 또는 동등 비교 가능 지표를 남긴다.
  - [x] operator-safe diagnostics는 capture 문제와 render cold-start / queue delay를 구분할 수 있어야 한다.

- [ ] regression test와 hardware validation 패키지를 준비한다. (AC: 1, 2, 3, 4, 5, 6)
  - [ ] fast preview 있음 / 없음 / 손상 / stale / wrong-session / wrong-capture / burst queue 케이스를 Rust integration test에 추가한다.
  - [x] current session rail selector와 booth surface가 pending preview를 보여주되 false-ready를 만들지 않는 UI/provider regression을 추가한다.
  - [ ] approved booth hardware에서 first-visible fast preview, later XMP replacement, timing split, cross-session isolation을 한 패키지로 수집한다.

### Review Findings

- [x] [Review][Patch] Existing canonical preview can be deleted before replacement is safely promoted [src-tauri/src/render/mod.rs:315]
- [x] [Review][Patch] Fast preview allowed-path validation trusts any session-scoped JPEG instead of a narrow handoff path [src-tauri/src/capture/ingest_pipeline.rs:547]
- [x] [Review][Patch] Preview repair sync can replace canonical `captureId.jpg` with newer suffixed siblings and break same-path truth [src-tauri/src/capture/normalized_state.rs:1150]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 Story 1.8의 render-backed preview truth를 대체하지 않는다.
- 핵심은 blank waiting을 줄이는 것이다.
- 고객이 먼저 보게 되는 이미지는 "체감 개선용 same-capture preview"이고, 진실 소스는 여전히 Story 1.8의 preset-applied render다.
- 따라서 이 스토리는 속도 개선 story이지, preview truth를 완화하는 story가 아니다.

### 왜 이 스토리가 새로 필요해졌는가

- 2026-04-01 기술 리서치 결과, 현재 구조에서도 UI 대공사 없이 체감 속도 개선 여지가 크다고 확인됐다. [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]
- 현재 프런트는 pending preview를 이미 보여줄 수 있지만, helper/host 계약에는 fast preview path가 없어 실전에서는 거의 활용되지 못한다. [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]
- 경쟁 제품도 "즉시 보이는 first preview"와 "나중에 교체되는 정식 preview"를 분리해 체감 속도를 만든다. [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]
- approved correct-course는 Story 1.8 유지 + Story 1.9 추가를 선택했다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260401-185009.md]

### 스토리 기반 요구사항

- PRD는 capture success, preview truth, final completion을 분리하라고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- PRD는 same-capture fast preview를 허용하되, preset-applied preview ready와 혼동하면 안 된다고 명시한다. [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- NFR-003은 current-session image를 가능한 빨리 보여주되, 5초 기준의 preset-applied preview ready truth를 따로 유지하도록 보정됐다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- Architecture는 helper optional fast-preview handoff, canonical preview promotion, same-path replacement, split telemetry를 허용한다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- UX는 `Preview Waiting` 중 same-capture fast preview를 먼저 보여줄 수 있어도 상태 자체는 그대로 유지하라고 요구한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]

### 선행 의존성과 구현 순서

- 직접 선행 책임:
  - Story 1.5: truthful `Preview Waiting`, current session rail, capture/preview separation
  - Story 1.7: helper-backed RAW persistence와 `file-arrived` correlation
  - Story 1.8: render-backed `previewReady` / `finalReady` truth
- 권장 구현 순서:
  1. helper optional handoff contract
  2. host validate/promote seam
  3. pending preview UI/selector regression
  4. same-path XMP replacement 정렬
  5. timing / diagnostics split
  6. hardware validation package

### 현재 워크스페이스 상태

- `src-tauri/src/capture/sidecar_client.rs`의 `CanonHelperFileArrivedMessage`는 현재 `rawPath`까지만 담고 있고 fast preview metadata는 없다.
- `src-tauri/src/capture/ingest_pipeline.rs`의 `persist_capture_in_dir(...)`는 capture record를 `previewWaiting`으로 만들고, `seed_pending_preview_asset_path(...)`가 이미 canonical preview 경로에 존재하는 raster file을 잡아 pending preview assetPath를 심을 수 있다.
- `seed_pending_preview_asset_path(...)`는 `renders/previews/{captureId}.{jpg|jpeg|png|webp|gif|bmp}`를 이미 찾는다. 즉 canonical path reuse 전략과 잘 맞는다.
- `src/session-domain/selectors/current-session-previews.ts`는 `renderStatus`가 `captureSaved` 또는 `previewWaiting`이고 `preview.readyAtMs === null`인 경우에도 session-scoped preview asset이 있으면 displayable pending preview로 노출한다.
- `src/booth-shell/components/SessionPreviewImage.tsx`는 `readyAtMs === null`을 pending preview로 취급하고 `current-session-preview-pending-visible` telemetry를 남길 준비가 되어 있다.
- `src-tauri/src/commands/capture_commands.rs`는 아직 `120ms` sleep 뒤 `complete_preview_render_in_dir(...)`를 시작한다. 이 경로는 Story 1.9에서도 유지하되, fast preview first-visible path와 분리 측정돼야 한다.
- `src-tauri/src/render/mod.rs`는 이미 canonical preview output path `renders/previews/{captureId}.jpg`를 사용한다. Story 1.9는 이 same path replacement 전략을 유지하는 편이 가장 안전하다.

### 이전 스토리 인텔리전스

- Story 1.8은 preset-applied `previewReady` / `finalReady` truth owner다. fast preview 때문에 이 기준을 느슨하게 만들면 안 된다. [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- Story 1.7은 supported capture success path를 booth 앱 `사진 찍기` 버튼 + `file-arrived` correlation으로 고정했다. Story 1.9도 이 경계를 깨면 안 된다. [Source: _bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md]
- Story 1.5는 `Preview Waiting` copy, rail-empty 안내, current-session-only rail을 이미 닫았다. Story 1.9는 그 UX를 재사용해야 한다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- `current-session-photo-troubleshooting-history.md`는 최근 세션 rail correctness 문제와 latency 문제를 구분해 정리해 두었다. Story 1.9는 correctness를 깨지 않고 latency만 개선해야 한다. [Source: history/current-session-photo-troubleshooting-history.md]

### 구현 가드레일

- fast preview가 있다고 해서 `previewReady`를 올리지 말 것.
- Story 1.8 render worker 외의 다른 경로가 `previewReady` / `readyAtMs`를 올리게 만들지 말 것.
- raw copy, placeholder SVG, representative preset tile을 fast preview 근거로 오해하지 말 것.
- helper가 준 path 문자열만 믿지 말고 session root, captureId, file validity를 다시 검증할 것.
- new `fastPreview` / `renderedPreview` schema field를 성급히 추가하지 말 것. MVP는 existing `preview.assetPath + readyAtMs` semantics를 최대한 재사용한다.
- customer copy에 darktable, XMP, queue, filesystem path, embedded preview 같은 기술 어휘를 노출하지 말 것.
- same-capture correctness가 불명확하면 fast preview를 버리고 기존 truthful `Preview Waiting`으로 fallback 할 것.

### 아키텍처 준수사항

- RAW와 booth-safe derived file은 large JSON IPC가 아니라 filesystem handoff로 이동해야 한다. [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- session truth는 `session.json`과 session-scoped filesystem root가 소유한다. [Source: docs/contracts/session-manifest.md]
- render-backed preview truth는 capture-bound published bundle과 pinned darktable `5.4.1` path가 계속 소유한다. [Source: docs/contracts/render-worker.md]
- capture 이후 publish/rollback 또는 active preset 변경이 있어도 later render는 capture-bound preset version을 유지해야 한다. [Source: docs/contracts/render-worker.md]
- booth/operator surfaces는 host-normalized truth만 소비하고 helper raw message를 직접 해석하지 않는다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/src/capture/sidecar_client.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/render/mod.rs`
  - `src/shared-contracts/schemas/session-capture.ts`
  - `src/shared-contracts/schemas/session-manifest.ts`
  - `src/session-domain/selectors/current-session-previews.ts`
  - `src/booth-shell/components/SessionPreviewImage.tsx`
  - `src-tauri/tests/capture_readiness.rs`
  - `docs/contracts/camera-helper-sidecar-protocol.md`
  - `docs/contracts/session-manifest.md`
  - `docs/contracts/render-worker.md`
- 새 top-level UI 구조를 만들기보다 existing rail / waiting / render seams 안에서 닫는 편이 우선이다.

### UX 구현 요구사항

- `Preview Waiting` 첫 문장은 저장 완료, 둘째 문장은 준비 중이어야 한다. fast preview가 먼저 보여도 copy 구조는 바뀌지 않는다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- latest photo rail은 same-capture pending image를 먼저 보여주고 later booth-safe preview로 같은 자리에서 자연스럽게 교체해야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Latest Photo Rail]
- rail이 비어 있는 fallback path도 유지해야 한다. fast preview가 없다고 UX가 깨지면 안 된다.

### 테스트 요구사항

- 최소 필수 테스트:
  - helper `file-arrived`에 fast preview metadata가 있는 경우 canonical preview path로 promote 된다.
  - fast preview가 있어도 `renderStatus`는 계속 `previewWaiting`이고 `readyAtMs`는 `null`이다.
  - later render-backed preview가 같은 canonical path를 교체하고 그때만 `previewReady`가 된다.
  - fast preview missing / invalid / wrong-session / wrong-capture / corrupted file은 discard + fallback 된다.
  - current-session selector는 pending preview를 노출하지만 false-ready를 만들지 않는다.
  - burst capture 시 queue delay가 있어도 이전 shot이나 다른 session shot이 rail에 섞이지 않는다.
  - stale cache 때문에 pending image가 later render-backed image로 교체되지 않는 회귀가 없어야 한다.

### 최신 기술 / 제품 컨텍스트

- 이번 스토리는 신규 라이브러리 도입이나 버전 업그레이드가 목적이 아니다.
- 최신 external behavior 판단은 2026-04-01 research artifact가 이미 정리했다.
  - 경쟁 제품은 embedded/cached first preview -> later accurate preview replacement 패턴을 쓴다.
  - Boothy도 구조 변경 없이 같은 제품 전략을 제한적으로 도입할 수 있다.
  - darktable cold-start, queue, GPU/OpenCL은 후속 최적화 포인트다.
- 따라서 Story 1.9 구현은 "새 기술 도입"보다 "existing host/helper/render seams를 staged preview로 연결"하는 데 집중해야 한다. [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]

### 금지사항 / 안티패턴

- `previewReady`를 pending fast preview visible과 같은 뜻으로 재정의하지 말 것.
- helper preview absence를 capture failure로 승격하지 말 것.
- capture helper, render worker, UI 각각이 별도 preview truth를 만들지 말 것.
- pending fast preview와 later render preview를 서로 다른 unrelated thumbnail slot으로 분리하지 말 것.
- Story 1.8 hardware close를 우회하기 위해 representative tile이나 raw copy를 ready truth처럼 보여주지 말 것.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.8: 게시된 프리셋 XMP 적용과 preview/final render worker 연결]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.9: fast preview handoff와 XMP preview 교체]
- [Source: _bmad-output/planning-artifacts/prd.md#Published Preset Artifact Model]
- [Source: _bmad-output/planning-artifacts/prd.md#Decision 2: Capture Truth, Preview Truth, and Final Completion Stay Separate]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/architecture.md#Darktable Capability Scope]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Latest Photo Rail]
- [Source: _bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260401-185009.md]
- [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- [Source: _bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md]
- [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/contracts/render-worker.md]
- [Source: history/current-session-photo-troubleshooting-history.md]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-02 10:38:12 +09:00 - Story 1.9 approved context를 기준으로 epics, PRD, architecture, UX, sprint-status, Story 1.5/1.7/1.8, capture latency research를 교차 분석했다.
- 2026-04-02 10:44:31 +09:00 - helper `file-arrived` optional fast preview handoff, host validate/promote seam, same-path render replacement, split timing/logging 경계를 구현했다.
- 2026-04-02 11:05:22 +09:00 - `cargo fmt`, `dotnet test`, `pnpm lint`, `pnpm test:run`, `cargo test --manifest-path src-tauri/Cargo.toml`를 다시 실행해 자동화 proof를 복구했다.
- 2026-04-02 11:28:56 +09:00 - code review patch 3건을 반영해 canonical preview 교체 실패 시 복구, fast preview allowed-path 축소, preview repair canonical path 고정을 다시 검증했다.

### Completion Notes List

- helper가 optional fast preview path를 넘기면 host가 same-session / same-capture / allowed-path / raster validity / staleness를 다시 검증한 뒤 canonical preview path로만 승격하도록 연결했다.
- pending fast preview가 보여도 booth truth는 계속 `Preview Waiting`으로 유지되고, later render-backed preview만 `previewReady`와 `readyAtMs`를 올리도록 Story 1.8 truth owner를 그대로 유지했다.
- `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs`, `fast-preview-*`, `preview-render-*` 진단을 추가해 first-visible과 later render-ready를 분리 기록했다.
- Rust integration, frontend contract/UI regression, helper test, lint/full Vitest/full Cargo validation은 통과했다.
- approved booth hardware evidence와 burst queue canonical proof는 아직 없으므로 Story 1.9 상태는 `review`로 유지한다.
- code review에서 나온 3개 patch finding을 모두 수정했고, 기존 canonical preview 보존/allowed-path 축소/canonical same-path 유지 회귀를 테스트로 잠갔다.

### File List

- _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs
- src-tauri/src/capture/sidecar_client.rs
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/capture/normalized_state.rs
- src-tauri/src/diagnostics/recovery.rs
- src-tauri/src/render/mod.rs
- src-tauri/src/session/session_manifest.rs
- src-tauri/tests/operator_diagnostics.rs
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/session-capture.ts
- src/session-domain/selectors/current-session-previews.ts
- src/booth-shell/components/SessionPreviewImage.tsx
- src-tauri/tests/capture_readiness.rs
- docs/contracts/camera-helper-sidecar-protocol.md
- docs/contracts/session-manifest.md
- docs/contracts/render-worker.md

### Change Log

- 2026-04-02 10:38:12 +09:00 - Story 1.9 ready-for-dev 문서 생성: fast preview handoff, canonical preview promotion, same-path XMP replacement, split telemetry, hardware validation guardrail을 구현 기준으로 정리
- 2026-04-02 11:05:22 +09:00 - Story 1.9 구현 완료: helper optional fast preview handoff, host validate/promote, same-path XMP replacement, split timing/diagnostics, frontend contract regression을 연결하고 lint/Vitest/Cargo/.NET 자동화 검증을 재통과시킴. hardware evidence와 burst queue canonical proof는 미수집 상태라 `review` 유지
- 2026-04-02 11:28:56 +09:00 - Story 1.9 code review patch 적용: canonical preview 교체 실패 시 기존 자산 복구, fast preview 허용 경로를 designated handoff/canonical path로 축소, preview repair가 suffixed sibling으로 canonical path를 갈아끼우지 않도록 고정. lint/Vitest/Cargo 전체 검증 재통과
