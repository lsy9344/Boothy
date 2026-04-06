# Story 1.11: local dedicated renderer sidecar와 truthful preview close canary routing

Status: review

Correct Course Note: 2026-04-04 승인된 sprint change proposal에 따라, Story 1.10은 `first-visible lane` 안정화와 seam 복구를 마친 선행 corrective track으로 유지하고, Story 1.11은 `local dedicated renderer sidecar`, feature-gated routing, darktable fallback, booth-scoped canary evidence를 소유하는 구조 실험 스토리로 새로 분리한다. 2026-04-06 승인된 reframe에 따라 이 스토리의 성공 기준은 `rail thumbnail speed`가 아니라 고객이 실제로 보고 있는 `latest large preview truthful close` 단축으로 다시 고정한다.

### Validation Gate Reference

- Supporting evidence family:
  - `HV-05` truthful `Preview Waiting -> Preview Ready`
  - approved booth hardware latency package
  - per-session seam package (`request-capture -> file-arrived -> fast-preview-visible -> preview-render-start -> capture_preview_ready -> latest-large-preview-visible -> recent-session-visible`)
  - renderer route comparison package (`renderer-route-selected -> renderer-route-fallback -> renderer-close-owner`)
  - preset fidelity comparison package
  - rollback drill evidence
- Current hardware gate: `Not run yet`
- Close policy:
  - automated pass만으로 닫지 않는다.
  - approved booth hardware에서 local renderer route와 darktable fallback을 같은 제품 계약 아래 비교해야 한다.
  - 승인 기준은 meaningful `latest large preview truthful close` 단축, preset fidelity 유지, `false-ready` 0건, cross-session leakage 0건, forced fallback / rollback 즉시성이다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

부스 고객으로서,
나는 내가 실제로 크게 보고 있는 latest large preview가 truth를 약화시키지 않으면서 가장 빠른 승인된 경로로 preset-applied 상태로 교체되길 원한다.
그래서 rail에 먼저 보이는 보조 신호와 별개로, 새 renderer route가 건강하지 않을 때도 안전 fallback을 유지한 채 최종적으로 믿을 수 있는 큰 preview를 더 빨리 볼 수 있다.

## Acceptance Criteria

1. host는 capture-bound preset identity, session-scoped paths, same-slot replacement 규칙을 유지한 채 approved local dedicated renderer sidecar를 `candidate result producer`로 호출할 수 있어야 한다. 이때 render truth owner는 sidecar가 아니라 계속 host여야 한다.
2. sidecar가 current capture용 canonical preview candidate를 반환하면 host는 same-session, same-capture, allowed-path, preset-version, raster validity를 검증한 뒤에만 이를 truthful preset-applied preview close로 채택할 수 있어야 한다. `previewReady`는 host validation 성공 전에는 절대 올라가면 안 된다.
3. booth runtime이 둘 이상의 approved preview-close route를 지원할 때 route 선택은 explicit feature-gated policy로 제어돼야 하며, unhealthy sidecar execution은 customer-safe waiting behavior를 깨지 않고 approved darktable path로 즉시 fallback할 수 있어야 한다.
4. 고객이 이미 same-capture first-visible image를 `Preview Waiting` 중에 본 상태여도, sidecar route 또는 fallback route가 truthful preset-applied preview를 닫을 때 canonical latest large preview path와 same-slot replacement 규칙은 그대로 유지돼야 한다. rail thumbnail과 다른 파생 surface는 이 primary close owner를 공유해야 하며 별도 preview truth를 만들면 안 된다. 고객 상태 이름은 계속 `Preview Waiting`과 later ready 상태만 사용해야 한다.
5. approved booth hardware canary에서는 selected route, fallback reason, close-owner result, elapsed timing, fidelity evidence가 하나의 session package 안에서 비교 가능해야 한다. 승인 기준은 meaningful `latest large preview truthful close` 개선, preset fidelity 유지, `false-ready` 0건, cross-session leakage 0건, rollback 즉시성이다.

## Tasks / Subtasks

- [x] local renderer adapter 계약과 sidecar packaging 경계를 정의한다. (AC: 1, 2, 3)
  - [x] `docs/contracts/local-renderer-adapter.md`를 새로 만들고 request/response/error envelope, schema version, timeout, retry, candidate output metadata, fidelity metadata를 명시한다.
  - [x] 새 renderer는 `sidecar/canon-helper/`에 섞지 말고 `sidecar/local-renderer/` 또는 동등한 독립 경계로 둔다.
  - [x] Tauri-managed sidecar를 선택한다면 `src-tauri/tauri.conf.json`, capability 파일, Rust/plugin wiring에 external binary 권한과 bundle 경로를 최소 범위로 추가한다.

- [x] host validation / promotion gate를 구현한다. (AC: 1, 2, 4)
  - [x] `src-tauri/src/render/mod.rs` 또는 동등 render orchestration 경로에서 candidate 결과를 검증하고 canonical preview path promotion을 host만 수행하게 한다.
  - [x] `previewReady`, `preview.readyAtMs`, 관련 readiness update는 host validation이 끝난 뒤에만 올리게 잠근다.
  - [x] wrong-session, wrong-capture, wrong-preset-version, invalid raster, stale file은 sidecar success처럼 보이더라도 discard + truthful fallback 되게 한다.

- [x] feature-gated routing policy와 darktable fallback을 연결한다. (AC: 3, 5)
  - [x] booth / session / preset 단위 route 선택 정책을 도입하되, 현재 branch rollout baseline과 active-session safe transition 원칙을 깨지 않는다.
  - [x] default booth path는 계속 approved darktable baseline으로 유지하고, 새 route는 opt-in canary로만 시작한다.
  - [x] forced fallback lane과 route disable path를 둬서 unhealthy sidecar를 즉시 우회할 수 있게 한다.

- [x] same-slot replacement와 customer-safe UX를 유지한다. (AC: 2, 4)
  - [x] first-visible lane에서 이미 canonical preview path에 pending image가 있어도 later truthful close는 같은 path를 재사용해 교체한다.
  - [x] `Preview Waiting` copy와 고객 상태 taxonomy는 그대로 유지하고, renderer route 변경 때문에 새 customer-facing 상태 이름을 만들지 않는다.
  - [x] current-session isolation, wrong-shot discard, cross-session leakage 0 원칙을 새 route에서도 다시 잠근다.

- [x] diagnostics / canary evidence / rollback governance를 보강한다. (AC: 3, 5)
  - [x] 한 session diagnostics package만으로 `renderer-route-selected`, `renderer-route-fallback`, `renderer-close-owner`, `elapsedMs`, fidelity verdict를 비교할 수 있게 한다.
  - [x] branch rollout / rollback 거버넌스와 충돌하지 않게 route canary와 forced fallback 기준을 문서화한다.
  - [x] operator-safe diagnostics에는 route와 fallback 이유를 남기되, 고객 화면에는 내부 엔진/sidecar 용어를 노출하지 않는다.

- [x] 계약 테스트, 통합 테스트, hardware canary를 준비한다. (AC: 1, 2, 3, 4, 5)
  - [x] host integration test에 valid candidate acceptance, invalid output discard, wrong-session/capture/preset rejection, immediate darktable fallback, same-slot replacement continuity를 추가한다.
  - [x] adapter contract test에 timeout, malformed payload, idempotent retry, stale output, duplicate completion을 추가한다.
  - [x] approved booth hardware에서 local renderer route vs darktable fallback의 close latency, fidelity, fallback rate, rollback drill을 같은 evidence package로 수집한다.

### Review Findings

- [x] [Review][Patch] 첫 캡처 cold-start 판정을 배열 인덱스로 대체해 삭제 이후 late fast preview에서도 speculative close를 잘못 건너뜀 [src-tauri/src/capture/ingest_pipeline.rs:470]
- [x] [Review][Patch] fullscreen viewer가 현재 세션 변경이나 preview 목록 갱신과 동기화되지 않아 stale photo를 계속 노출할 수 있음 [src/booth-shell/screens/CaptureScreen.tsx:154]
- [x] [Review][Patch] canary policy가 local renderer를 global default route로 승격시킬 수 있음 [src-tauri/src/render/mod.rs:1884]
- [x] [Review][Patch] session rule보다 broad booth rule이 먼저 적용돼 active session canary 우선순위를 깨뜨릴 수 있음 [src-tauri/src/render/mod.rs:1951]
- [x] [Review][Patch] fidelity detail을 수집해 놓고 session diagnostics에 남기지 않아 canary evidence가 부족함 [src-tauri/src/render/mod.rs:311]
- [x] [Review][Patch] 런타임이 요구하는 local renderer sidecar 실행 자산이 구현에 포함되지 않아 canary route가 실제 부스에서 항상 fallback될 수 있음 [sidecar/local-renderer/local-renderer-sidecar.cmd:1]
- [x] [Review][Patch] 스토리에서 완료로 표시한 테스트 행렬이 아직 다 닫히지 않았음 [src-tauri/tests/capture_readiness.rs:5552]
- [x] [Review][Patch] branch-scoped preview route rule이 런타임에서 절대 선택되지 않아 booth canary policy가 session/preset rule로만 축소됨 [src-tauri/src/render/mod.rs:2121]
- [x] [Review][Patch] darktable close-owner diagnostics에 fidelity verdict/detail이 없어 route 간 fidelity evidence를 한 session package에서 비교할 수 없음 [src-tauri/src/render/mod.rs:377]
- [x] [Review][Patch] local renderer sidecar가 host의 darktable 경로 탐색을 재사용하지 않아 PATH 밖 설치 부스에서는 canary가 즉시 fallback될 수 있음 [sidecar/local-renderer/local-renderer-sidecar.ps1:14]
- [x] [Review][Patch] local renderer 성공 diagnostics가 실제 source asset 대신 항상 raw-original로 기록돼 canary evidence를 왜곡함 [src-tauri/src/render/mod.rs:362]
- [x] [Review][Patch] preview route policy lock 저장 실패가 세션 시작 자체를 중단시켜 canary 보조 기능이 고객 세션 생성 실패로 번질 수 있음 [src-tauri/src/session/session_repository.rs:68]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 `full platform pivot`이 아니다.
- 제품 코어는 계속 `Rust/Tauri host + session truth + host-owned preview truth`를 유지한다.
- primary booth artifact는 계속 customer가 크게 보고 있는 `latest large preview`이며, rail thumbnail은 같은 close owner를 공유하는 secondary surface다.
- 바꾸는 것은 이 primary artifact를 닫는 `preset-applied truthful close hot path`의 topology다.
- Story 1.10이 `first-visible lane`과 seam 복구를 마무리했다면, Story 1.11은 그 위에서 `routeable local renderer topology`를 실험하고 canary로 검증하는 단계다.
- 고객 약속은 바뀌지 않는다. 먼저 같은 컷이 보일 수 있지만, truthful close가 닫히기 전까지 상태는 계속 `Preview Waiting`이다.

### 왜 이 스토리가 새로 필요해졌는가

- 2026-04-06 입력 문서와 승인된 correct-course는 same-capture `first-visible`이 최근 `약 3.0s ~ 3.5s`, best run `2959ms`까지 내려온 반면 고객이 실제로 기다리는 `preset-applied truthful close`는 best run `6372ms`, 다른 회차는 `7s ~ 10s+`에 남아 있다고 정리했다. [Source: docs/recent-session-preview-architecture-update-input-2026-04-06.md]
- 즉 남은 문제는 rail에 무언가를 빨리 보이는 것보다, 고객이 크게 보고 있는 `latest large preview truthful close` 자체를 더 짧게 만들 수 있는 구조가 무엇인지다. [Source: docs/recent-session-preview-architecture-update-input-2026-04-06.md]
- 2026-04-04 기술 리서치는 `Rust/Tauri host 유지 + local dedicated renderer sidecar + darktable fallback`이 현재 계약을 가장 덜 깨면서도 canary와 rollback을 붙이기 쉬운 1차 권장안이라고 결론 냈다. [Source: _bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-2026-04-04.md]
- 같은 날짜 sprint change proposal은 Story 1.10을 baseline stabilization track으로 유지하고, 구조 실험 책임을 새 Story 1.11로 분리 승인했다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260404-232052.md]
- 2026-04-06 승인안은 이 기술 방향을 뒤집지 않고, Story 1.11의 성공 기준을 `latest large preview replacement` 중심으로 다시 고정하고 cross-cutting artifact/seam 재정렬은 후속 Story 1.12 backlog로 분리했다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260406-110430.md]

### 스토리 기반 요구사항

- PRD는 first-visible image와 preset-applied preview readiness latency를 분리해서 측정하되, 둘 다 `latest large preview` 기준으로 읽어야 하며 route가 바뀌어도 `previewReady` truth는 host-validated render behavior에만 속한다고 못 박는다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- PRD는 `latest large preview`를 primary booth-facing artifact로 정의하고, rail thumbnail은 이를 공유하거나 파생하는 secondary surface일 뿐 별도 truth path가 아니라고 규정한다. [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- PRD release gate는 새 renderer route가 booth-scoped canary, instant fallback, false-ready / leakage 0을 지원해야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#Release Gates]
- Architecture는 preview-close execution이 approved darktable path 또는 approved local dedicated renderer adapter 뒤에서 routeable할 수 있지만, 대체 route는 candidate-result producer일 뿐 독립 truth owner가 아니라고 정의한다. 또한 `latest large preview`가 primary artifact이고 rail은 같은 close owner를 공유하는 secondary surface라고 못 박는다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- Session manifest 계약은 `previewWaiting` 중 canonical latest preview path가 먼저 채워질 수 있어도, `preview.readyAtMs`가 비어 있으면 아직 truthful close가 아니며 `fastPreviewVisibleAtMs`와 `previewVisibleAtMs`를 분리 유지해야 한다고 고정한다. [Source: docs/contracts/session-manifest.md]
- Render worker 계약과 sprint proposal은 diagnostics에 `renderer-route-selected`, `renderer-route-fallback`, `renderer-close-owner`를 남기고, primary latest large preview close를 기준으로 route 비교가 가능해야 한다고 요구한다. [Source: docs/contracts/render-worker.md] [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260406-110430.md]
- UX는 route가 fast preview, resident worker, local renderer로 바뀌어도 고객 상태 이름을 늘리지 말고 같은 컷이 먼저 보였다가 나중에 더 정확한 결과로 안정화되는 경험만 유지하라고 요구한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]

### 선행 의존성과 구현 순서

- 직접 선행 책임:
  - Story 1.8: render-backed `previewReady` / `finalReady` truth owner
  - Story 1.9: canonical preview path promotion, same-slot replacement, `Preview Waiting` 보호 의미
  - Story 1.10: resident first-visible worker, per-session seam 복구, truthful fallback baseline
- 권장 구현 순서:
  1. local renderer adapter contract와 route policy 문서를 먼저 고정한다.
  2. sidecar packaging / execution boundary를 추가한다.
  3. host validation / promotion gate를 구현한다.
  4. feature-gated routing과 darktable fallback을 연결한다.
  5. diagnostics / canary evidence / rollback drill을 보강한다.
  6. contract test -> host integration test -> booth hardware canary 순으로 닫는다.

### 현재 워크스페이스 상태

- `src-tauri/src/render/mod.rs`에는 resident first-visible worker, warm-up, darktable preview/final render path, canonical preview replacement 자산이 이미 있다.
- `src-tauri/src/capture/ingest_pipeline.rs`와 `src-tauri/tests/capture_readiness.rs`에는 same-capture preview promotion과 seam logging 회귀 기반이 있다.
- `src-tauri/src/branch_config/mod.rs`와 `src-tauri/tests/branch_rollout.rs`에는 branch rollout / rollback baseline, active-session defer, pending baseline 적용 규칙이 이미 있다.
- 반면 현재 `src-tauri/tauri.conf.json`과 capability 파일은 `core:default`만 열어 두고 있고, sidecar 외부 바이너리 bundle / execute 권한은 아직 명시돼 있지 않다.
- 현재 repo에는 `sidecar/canon-helper/`만 있고 local renderer 전용 경계는 없다.
- 즉 1.11은 기존 render/capture/session truth 자산을 재사용하되, `새 route의 계약 + 패키징 + host gate`를 추가하는 형태가 가장 자연스럽다.

### 이전 스토리 인텔리전스

- Story 1.8은 selected preset apply truth를 실제 render worker에 연결하며, `previewReady`가 render-backed booth-safe output 이전에 올라가면 안 된다는 기준을 닫았다. 1.11도 이 소유권을 절대 건드리면 안 된다. [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- Story 1.9의 핵심 교훈은 `first-visible` 개선이 곧 `truthful close` 개선을 뜻하지 않는다는 점이다. fast preview나 helper fallback만으로 final close 병목은 해결되지 않았다. [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- Story 1.10은 `known-good` preview baseline, resident worker topology, per-session seam 복구를 마무리했고, 구조 실험과 현장 성능 증명 책임을 1.11로 이관했다. [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- 최근 커밋 흐름도 `fast preview fallback -> seam logging -> topology stabilization -> next attempt 준비` 순으로 이어진다.
  - `367626a` Prepare next thumbnail latency attempt
  - `2c89c40` Finalize thumbnail latency worker updates and docs
  - `9c56c37` Add session seam logging for thumbnail latency reduction
  - `b24cfc4` Reduce recent-session preview latency and capture wait blocking
  - `12309fa` Record thumbnail validation and ship fast preview fallback

### 구현 가드레일

- local renderer sidecar가 `session.json`을 직접 수정하거나 `previewReady`를 직접 선언하게 만들지 말 것.
- `previewReady`, `preview.readyAtMs`, readiness event는 host validation과 canonical promotion 뒤에만 갱신할 것.
- 새 route가 빠르더라도 wrong-session / wrong-capture / wrong-preset-version candidate를 조용히 받아들이지 말 것.
- sidecar unhealthy 상태를 고객 UX 뒤에 숨기려고 false-ready, representative tile, 이전 컷, 다른 세션 컷을 성공 산출물처럼 승격하지 말 것.
- route 선택을 프런트 UI local state나 ad-hoc debug flag에 묻어 두지 말고, host-owned explicit policy로 관리할 것.
- active session이 있는 지점에 canary / rollback을 적용할 때 branch rollout safe transition 원칙을 우회하지 말 것.
- 고객 화면에는 local renderer, darktable fallback, queue saturation, route owner 같은 내부 용어를 절대 노출하지 말 것.

### 아키텍처 준수사항

- session truth는 계속 `session.json`과 session-scoped filesystem root가 소유한다. [Source: docs/contracts/session-manifest.md]
- capture-bound `activePresetId + activePresetVersion` resolution을 유지하고, 새 route도 같은 published bundle identity를 따라야 한다. [Source: docs/contracts/session-manifest.md] [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- darktable fallback은 현재 pinned `5.4.1` baseline을 계속 따른다. 별도 승인 없이 fallback engine/version을 바꾸지 않는다. [Source: docs/contracts/render-worker.md]
- 기존 render-worker 문구 중 resident/speculative 결과를 close owner처럼 읽을 여지가 있더라도, 1.11에서는 그것을 `host validation 뒤에만 성립하는 owner 판정`으로 해석해야 한다. sidecar나 worker가 self-promote하는 해석은 금지다. [Source: docs/contracts/render-worker.md] [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- helper/worker/host 경계는 계속 versioned message + filesystem handoff 중심으로 유지한다. 새 local renderer는 `canon-helper` 경계와 혼합하지 않는다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- React UI는 새 route를 직접 호출하거나 판단하지 않는다. route 상태와 outcome은 host-normalized DTO를 통해서만 소비한다. [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]

### 프로젝트 구조 요구사항

- 우선 검토/수정 대상:
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/branch_config/mod.rs`
  - `src-tauri/src/commands/branch_rollout_commands.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/capabilities/*.json`
  - `src-tauri/tests/capture_readiness.rs`
  - `src-tauri/tests/branch_rollout.rs`
- 신규/보강 권장 문서:
  - `docs/contracts/local-renderer-adapter.md`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/branch-rollout.md`
- 신규 sidecar 자산은 `sidecar/local-renderer/` 또는 동등한 별도 디렉터리에 두고, helper 전용 `sidecar/canon-helper/`와 파일/책임을 섞지 않는다.

### UX 구현 요구사항

- customer copy는 계속 `Preview Waiting` 보호 흐름을 따른다. 첫 문장은 저장 완료, 둘째 문장은 확인용 사진 준비 중이라는 의미를 유지한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- customer가 가장 크게 보는 latest large preview 영역이 primary confirmation surface이며, rail은 current-session confidence를 돕는 secondary surface다. [Source: docs/recent-session-preview-architecture-update-input-2026-04-06.md]
- same-capture first-visible image가 먼저 있더라도 later truthful close는 같은 primary slot에서 자연스럽게 교체돼야 하고, rail은 그 close owner를 공유해야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Latest Photo Rail]
- route가 바뀌었다는 이유로 고객 상태 이름, 버튼 구조, rail taxonomy를 바꾸지 않는다.
- sidecar candidate가 discard되어도 customer experience는 empty rail fallback + truthful `Preview Waiting`으로 안전하게 유지돼야 한다.

### 테스트 요구사항

- 최소 필수 테스트:
  - host가 valid local renderer candidate만 acceptance하고 invalid candidate는 discard + darktable fallback 한다.
  - sidecar success가 있어도 host validation 전에는 `previewReady`가 올라가지 않는다.
  - same-capture pending preview가 이미 canonical latest large preview path에 있어도 later truthful close는 same-path replacement로 이어진다.
  - wrong-session / wrong-capture / wrong-preset-version / stale-output / malformed-output은 모두 rejection 된다.
  - route policy가 booth / session / preset 단위 canary를 지원하고, active session safety 규칙을 깨지 않는다.
  - forced fallback / rollback lane이 적용되면 새 route를 즉시 우회하고도 customer-safe waiting behavior가 유지된다.
  - session seam package 하나만으로 route selected, fallback reason, close owner, elapsedMs, fidelity verdict, `latest-large-preview-visible`를 primary artifact 기준으로 비교할 수 있다.
  - booth hardware canary에서 local renderer route와 darktable fallback을 같은 세션 기준으로 비교하는 evidence package가 재현 가능하다.

### 최신 기술 / 제품 컨텍스트

- 2026-04-06 승인된 제품 재정렬은 Story 1.11의 목표를 `thumbnail speed`가 아니라 `latest large preview replacement` 중심으로 다시 고정했다. Story 1.11은 route experiment와 canary를 유지하되, cross-cutting artifact ownership / seam reinstrumentation은 신규 Story 1.12 backlog로 분리해 scope creep를 막는다. [Source: docs/recent-session-preview-architecture-update-input-2026-04-06.md] [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260406-110430.md]
- 현재 repo는 `@tauri-apps/api` / `@tauri-apps/cli` `2.10.1`, Rust `tauri` `2.10.3`, React `19.2.4`, Zod `4.3.6`, Vitest `4.1.0`을 사용한다. 새 route는 이 현재 메이저 라인과 충돌하지 않는 범위에서 구현해야 한다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- React 공식 문서는 latest major를 `19.2`로 안내한다. 즉 1.11은 UI 프레임워크 교체가 아니라 existing host/booth-shell 경계 위에서 해결해야 한다. [Source: https://react.dev/versions]
- Tauri 2 공식 문서는 external sidecar binary packaging 패턴을 계속 지원한다. 현재 repo에는 shell/sidecar capability wiring이 없으므로, Tauri-managed sidecar를 택한다면 최소 권한으로 보강하는 작업이 필요하다. 이 문장은 공식 sidecar 문서를 현재 `tauri.conf.json`/capabilities 상태에 적용한 추론이다. [Source: src-tauri/tauri.conf.json] [Source: src-tauri/capabilities/booth-window.json] [Source: https://v2.tauri.app/develop/sidecar/]
- darktable 공식 release와 내부 리서치 기준으로 현재 fallback baseline은 `5.4.1`에 맞춰져 있다. 1.11의 목표는 fallback 교체가 아니라 new route 도입과 host-owned truth 유지다. [Source: https://www.darktable.org/2026/02/darktable-5.4.1-released/] [Source: docs/contracts/render-worker.md] [Source: _bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-2026-04-04.md]

### 금지사항 / 안티패턴

- local renderer를 독립 truth owner처럼 취급해 sidecar completion만으로 `previewReady`를 올리지 말 것.
- `canon-helper`에 renderer 책임까지 몰아 넣어 capture boundary와 render boundary를 다시 섞지 말 것.
- 새 route 실험을 global always-on으로 배포하지 말 것.
- branch rollout / rollback과 별개인 shadow config를 만들어 현장 canary 기준을 이중화하지 말 것.
- route 비교 지표를 mixed global log에 다시 의존하게 만들지 말 것.
- sidecar failure를 raw copy, placeholder SVG, representative preset tile로 숨기지 말 것.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.11: local dedicated truthful close route와 canary routing]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- [Source: _bmad-output/planning-artifacts/prd.md#Release Gates]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Latest Photo Rail]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260404-232052.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260406-110430.md]
- [Source: _bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-2026-04-04.md]
- [Source: docs/recent-session-preview-architecture-update-input-2026-04-06.md]
- [Source: history/recent-session-thumbnail-speed-agent-context.md]
- [Source: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md]
- [Source: _bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md]
- [Source: _bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/contracts/branch-rollout.md]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]
- [Source: src-tauri/tauri.conf.json]
- [Source: src-tauri/capabilities/booth-window.json]
- [Source: https://react.dev/versions]
- [Source: https://v2.tauri.app/develop/sidecar/]
- [Source: https://www.darktable.org/2026/02/darktable-5.4.1-released/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-04-04 23:42:26 +09:00 - `bmad-create-story` workflow 기준으로 config, sprint-status, epics, PRD, architecture, UX, Story 1.8/1.9/1.10, sprint change proposal, branch rollout contract, render-worker/session-manifest contract, recent-session history, current code 경계를 교차 분석했다.
- 2026-04-04 23:42:26 +09:00 - Story 1.11을 `local dedicated renderer sidecar + host validation + feature-gated routing + darktable fallback + canary evidence` 중심의 ready-for-dev guide로 정리했다.
- 2026-04-04 23:42:26 +09:00 - React latest major, Tauri sidecar official path, darktable fallback baseline을 공식 문서로 다시 확인해 story guardrail에 반영했다.
- 2026-04-05 01:34:00 +09:00 - `src-tauri/src/render/mod.rs`에 local renderer route policy 로더, sidecar request/response 계약, host validation gate, renderer route selected/fallback/close-owner diagnostics를 추가했다.
- 2026-04-05 01:34:00 +09:00 - `src-tauri/tests/capture_readiness.rs`와 render unit test에 canary acceptance, invalid candidate fallback, malformed/stale/duplicate rejection, route policy contract 회귀를 추가했다.
- 2026-04-05 01:34:00 +09:00 - `docs/contracts/local-renderer-adapter.md`, `docs/contracts/render-worker.md`, `docs/contracts/branch-rollout.md`, `sidecar/local-renderer/README.md`를 갱신해 route governance와 sidecar boundary를 문서화했다.
- 2026-04-06 12:05:00 +09:00 - 승인된 preview replacement reframe을 반영해 Story 1.11 문구를 `latest large preview` 중심으로 재정렬하고, primary/secondary artifact 구분 및 seam evidence guardrail을 보강했다.
- 2026-04-06 12:27:13 +09:00 - `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`, `pnpm test:run`, `pnpm lint`를 다시 실행해 회귀를 검증했고, `hardware-validation-ledger.md`의 Story 1.8 canonical package 문구를 현재 governance 테스트 기대와 다시 정렬했다.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created
- Story 1.10의 범위 재정의와 2026-04-04 구조 전환 승인 내용을 반영해, 1.11을 별도 local renderer topology experiment로 분리했다.
- 2026-04-06 realignment를 반영해 Story 1.11의 성공 기준을 `latest large preview truthful close`로 명확히 고정하고, rail을 shared close owner를 따르는 secondary surface로 다시 정의했다.
- 새 route를 `candidate result producer`로만 제한하고, host validation / fallback / canary / rollback을 필수 범위로 고정했다.
- 기존 render/capture/session/branch rollout 자산을 재사용하되, 새 frozen surface로 `local renderer adapter contract`와 `renderer routing policy`를 명시했다.
- host가 `branch-config/preview-renderer-policy.json` 기반으로 preview close route를 선택하고, local renderer candidate를 검증한 뒤에만 same-slot canonical preview를 승격하도록 구현했다.
- invalid/malformed/stale/duplicate candidate는 모두 truthful darktable fallback으로 우회하고, diagnostics package에 route selected/fallback/close-owner evidence를 남기도록 보강했다.
- full Rust 검증은 `cargo test --manifest-path .\\src-tauri\\Cargo.toml -- --test-threads=1`로 통과했다. 기본 병렬 실행은 기존 shared test runtime 특성 때문에 여전히 비결정적일 수 있어 단일 스레드 검증을 증거 기준으로 기록한다.
- `pnpm test:run` 258개 테스트와 `pnpm lint`까지 다시 통과시켜, story close 전 회귀 게이트와 governance ledger 정합성을 함께 확인했다.
- approved booth hardware canary evidence는 코드/문서 기준으로 준비했으며, 실제 booth 수집 자체는 별도 현장 검증 단계에서 수행해야 한다.

### Change Log

- 2026-04-06 - Story 1.11 문구를 latest large preview replacement 중심으로 재정렬하고, primary/secondary artifact 및 seam evidence 기준을 보강했다.
- 2026-04-06 - 전체 검증 재실행 중 발견된 hardware validation governance ledger 문구 회귀를 정렬하고 story 상태를 `review`로 갱신했다.
- 2026-04-05 - local renderer canary route, host validation gate, darktable fallback, diagnostics evidence package, contract/docs/test coverage를 추가하고 story 상태를 `review`로 전환했다.

### File List

- _bmad-output/implementation-artifacts/1-11-local-dedicated-renderer-sidecar와-truthful-preview-close-canary-routing.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/local-renderer-adapter.md
- docs/contracts/render-worker.md
- docs/contracts/branch-rollout.md
- sidecar/local-renderer/README.md
- src-tauri/src/render/mod.rs
- src-tauri/tests/capture_readiness.rs
