# Story 1.17: canonical preset recipe와 XMP adapter 기준 동결

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
booth runtime과 authoring/fallback이 공유할 canonical preset recipe를 먼저 고정하고 싶다,
그래서 GPU lane, darktable fallback, publication bundle이 같은 룩 진실을 기준으로 움직일 수 있다.

## Acceptance Criteria

1. canonical preset recipe 최소 스키마가 authoritative 문서와 계약 테스트로 확정되어야 하며, TypeScript shared schema, Rust host loader, authoring publication flow가 같은 의미로 해석해야 한다.
2. published preset bundle 또는 동등한 publication artifact 구조에서 canonical preset recipe reference와 darktable-compatible artifact reference가 함께 연결되어야 하며, XMP의 역할이 compatibility / fallback / parity adapter임이 명시되어야 한다.
3. booth runtime은 특정 편집기 내부 표현 하나만을 runtime truth로 삼지 않아야 하며, 현재 pinned darktable `5.4.1`, safe-relative-path 규칙, preview/final profile baseline, future-session-only publication guardrail을 유지해야 한다.
4. 현재 booth customer-facing semantics는 그대로 유지되어야 한다. 고객 화면에 darktable/XMP 용어가 노출되면 안 되고, `previewReady`/`finalReady`, active session binding, release baseline 책임을 이 스토리에서 임의로 바꾸면 안 된다.

## Tasks / Subtasks

- [x] canonical preset recipe authoritative contract를 새 기준선으로 고정한다. (AC: 1, 2)
  - [x] `docs/contracts/canonical-preset-recipe.md` 또는 동등한 authoritative 문서를 추가해 recipe의 최소 필드를 명시한다.
  - [x] recipe 최소 스키마는 최소한 preset identity, published version, booth-safe display metadata, preview/final render intent, noise policy, adapter artifact reference를 포함해야 한다.
  - [x] darktable module slider, full shader graph, live GPU execution plan까지 이 스토리에서 설계하지 않는다. Story 1.17의 목표는 “주 진실의 최소 shape”를 잠그는 것이지 renderer 구현을 끝내는 것이 아니다.

- [x] publication/bundle 계약을 recipe truth 기준으로 재정렬한다. (AC: 1, 2, 3)
  - [x] `docs/contracts/preset-bundle.md`, `docs/contracts/authoring-publication.md`, `docs/contracts/authoring-publication-payload.md`, `docs/contracts/authoring-validation.md`, `docs/contracts/render-worker.md`, `docs/contracts/local-dedicated-renderer.md`를 recipe/XMP adapter 의미에 맞게 정렬한다.
  - [x] published bundle이 계속 유지된다면 `recipe reference + darktable adapter reference`를 함께 담는 구조를 명시하고, XMP 단독 truth처럼 읽히는 문구를 제거한다.
  - [x] 현재 `published-preset-bundle/v1`을 additive 변경할지, 명시적 새 schema version으로 분리할지 명확히 결정한다. 현재 TypeScript/Rust parser가 unknown field를 strict reject하므로 부분 적용은 허용되지 않는다.

- [x] authoring/runtime/schema 구현 경로를 원자적으로 맞춘다. (AC: 1, 2, 3)
  - [x] `src/shared-contracts/schemas/preset-core.ts`, `src/shared-contracts/schemas/preset-authoring.ts`, `src/shared-contracts/contracts.test.ts`를 새 기준에 맞게 갱신한다.
  - [x] `src-tauri/src/contracts/dto.rs`, `src-tauri/src/preset/preset_bundle.rs`, `src-tauri/src/preset/authoring_pipeline.rs`, `src-tauri/src/preset/default_catalog.rs`를 같은 의미로 정렬한다.
  - [x] default preset seed와 fixture bundle이 새 recipe reference 기준을 충족하도록 `tests/fixtures/contracts/preset-bundle-v1/**` 또는 후속 fixture 경로를 함께 갱신한다.
  - [x] `xmpTemplatePath`, `darktableVersion`, optional `darktableProjectPath`는 계속 adapter metadata로 다루고, GPU lane이 필요로 할 future-facing recipe truth와 혼동하지 않는다.

- [x] Story 1.18/1.19를 위한 회귀 가드레일을 준비한다. (AC: 3, 4)
  - [x] `src-tauri/tests/preset_authoring.rs`, `src-tauri/tests/dedicated_renderer.rs`, `src-tauri/tests/capture_readiness.rs`, `src-tauri/tests/session_manifest.rs` 또는 동등 테스트에서 recipe/XMP drift가 booth truth를 깨지 않음을 검증한다.
  - [x] negative test를 추가해 path escape, missing recipe reference, partial metadata mismatch, XMP-only truth regression을 막는다.
  - [x] current booth semantics는 유지한다: same-capture preview close, future-session-only publication, active session immutability, customer-safe wording, release/hardware gate 분리.

## Dev Notes

### 스토리 범위와 제품 목적

- 이 스토리는 GPU-first renderer 자체를 구현하는 작업이 아니라, 그 구현이 의존할 preset truth baseline을 먼저 고정하는 foundational contract story다.
- Story 1.15가 publication state machine과 immutable bundle guardrail을 닫았고, Story 1.16이 build/release baseline을 닫았다면, Story 1.17은 “preset truth를 XMP 단독 표현과 분리해서 잠그는 일”을 맡는다.
- 결과물은 새 customer-facing 기능이 아니라, authoring/publish/runtime/fallback이 같은 preset truth를 공유하게 만드는 개발 기준선이다.

### 왜 지금 필요한가

- 2026-04-12 correct-course는 실행 우선순위를 `canonical preset recipe 고정 -> resident GPU-first display lane prototype -> telemetry/parity gate` 순서로 재정렬했다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260412-044022.md]
- architecture는 이미 preset publication truth가 canonical preset recipe를 중심으로 정렬돼야 하고, XMP가 adapter artifact로 남아야 한다고 적고 있다. [Source: _bmad-output/planning-artifacts/architecture.md]
- Epic 1의 follow-up sequence도 1.17을 1.18, 1.19보다 먼저 두고 있다. 즉 recipe truth를 먼저 잠그지 않으면 이후 GPU lane story가 다시 XMP-centered 해석으로 흔들릴 가능성이 높다. [Source: _bmad-output/planning-artifacts/epics.md]

### 현재 워크스페이스 상태

- 현재 저장소에는 explicit `canonical preset recipe` authoritative contract 파일이 없다.
- published bundle baseline은 이미 존재하지만, 현재 shape는 사실상 XMP-centered다.
  - 문서: `docs/contracts/preset-bundle.md`
  - TypeScript parser: `src/shared-contracts/schemas/preset-core.ts`
  - Rust loader: `src-tauri/src/preset/preset_bundle.rs`
  - fixture: `tests/fixtures/contracts/preset-bundle-v1/preset_soft-glow/2026.04.10/bundle.json`
  - seed catalog: `src-tauri/src/preset/default_catalog.rs`
- authoring draft/publish path도 동일하게 XMP adapter와 darktable pin을 중심으로 움직인다.
  - `src/shared-contracts/schemas/preset-authoring.ts`
  - `src-tauri/src/preset/authoring_pipeline.rs`
  - `docs/contracts/authoring-validation.md`
  - `docs/contracts/authoring-publication*.md`
- authoring validation은 이미 XMP history stack 존재 여부, pinned darktable version, safe workspace path를 검증하고 있으며, `.dtpreset` 없이도 XMP-only draft를 허용한다. 즉 “XMP adapter baseline”은 이미 있고, 이번 스토리는 그 위에 “canonical recipe truth”를 추가로 잠그는 일이다.
- 별도의 `project-context.md`는 발견되지 않았다. 현재 작업의 canonical context는 planning artifacts, `docs/contracts/*`, authoring/runtime schema와 fixture다.

### 이전 스토리 인텔리전스

- Story 1.15는 publication state machine, immutable bundle rule, future-session-only publish/rollback, active session immutability를 이미 닫았다. 1.17은 approval/rollback behavior를 다시 설계하지 말고, preset truth shape만 명확히 해야 한다. [Source: _bmad-output/implementation-artifacts/1-15-canon-helper-profile과-publication-contract-확정.md]
- Story 1.16은 build/release baseline을 닫았고, release workflow/signing/updater semantics는 그 스토리의 책임으로 남겨 두라고 분명히 적었다. recipe story에서 release pipeline 변경으로 범위를 번지지 말 것. [Source: _bmad-output/implementation-artifacts/1-16-windows-desktop-build-release-baseline과-ci-proof-설정.md]
- 최근 커밋은 preview lane, dedicated renderer, diagnostics 정리에 집중돼 있다.
  - `40015d1 feat: checkpoint preset applied rendering and diagnostics`
  - `4611eb5 feat: add local renderer contracts and release baseline`
  - `8c30be7 Improve focus retry guidance`
  - `2c89c40 Finalize thumbnail latency worker updates and docs`
  - `9c56c37 Add session seam logging for thumbnail latency reduction`
- 따라서 Story 1.17은 preview runtime을 갈아엎는 것이 아니라, Story 1.18이 사용할 공통 preset truth를 먼저 닫는 방향이 맞다.

### 제품/아키텍처 가드레일

- 고객 화면은 계속 booth-safe vocabulary만 사용해야 한다. darktable, XMP, module, style, library, GPU/OpenCL 용어를 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md]
- current runtime truth를 이 스토리에서 임의로 바꾸면 안 된다. `session-manifest.md`와 `render-worker.md`가 정의한 `previewReady`/`finalReady` 의미, capture-bound preset binding, same-path truthful close 규칙은 그대로 유지한다. Story 1.17은 renderer lane 승격이 아니라 recipe contract freeze다. [Source: docs/contracts/session-manifest.md] [Source: docs/contracts/render-worker.md]
- canonical recipe는 editor-agnostic해야 하지만 과도하게 무거우면 안 된다. darktable module internals나 GPU shader program 전체를 새로운 business schema로 억지로 승격하지 말고, booth runtime과 fallback/oracle이 공유해야 하는 최소 공통 truth만 담는다.
- XMP는 compatibility / fallback / parity adapter다. current booth runtime이 XMP adapter를 계속 사용하더라도, product truth 문서에서는 XMP가 “유일한 룩 진실”로 읽히지 않아야 한다.
- publish/rollback은 계속 future-session-only다. active session manifest, current capture binding, current preview/final truth를 직접 덮어쓰는 경로를 새로 만들면 안 된다. [Source: docs/contracts/authoring-publication.md] [Source: docs/contracts/preset-bundle.md]
- TypeScript와 Rust가 모두 strict parser를 사용하므로, schema 필드 추가/변경은 docs, parsers, runtime loader, fixtures, seeds, tests를 한 번에 맞춰야 한다. 일부만 바꾸면 catalog load, authoring publish, dedicated renderer fixture가 동시에 깨질 수 있다.

### 구현 가드레일

- `published-preset-bundle/v1`를 계속 쓴다면 새 recipe reference를 포함하도록 TypeScript/Rust strict schema를 함께 업데이트해야 한다.
- 새 schema version을 도입한다면 migration 범위를 명확히 적고, default seed bundle과 fixture, authoring publication output, runtime loader, tests를 한 번에 정렬해야 한다.
- `src-tauri/src/preset/default_catalog.rs`의 seed bundle은 현재 XMP-only baseline이다. recipe reference가 필수라면 seed data backfill도 이번 스토리 범위에 포함해야 한다.
- `src-tauri/src/preset/authoring_pipeline.rs`는 publish 시점에 bundle을 직렬화한다. canonical recipe reference를 추가한다면 이 publish path가 authoritative source가 되도록 유지해야 하며, React가 recipe truth를 직접 조립하면 안 된다.
- `docs/contracts/authoring-validation.md`의 current validation은 XMP compatibility 중심이다. recipe baseline을 추가하더라도 기존 XMP compatibility validation은 유지하고, recipe truth와 adapter truth의 책임을 분리해야 한다.
- release workflow, updater behavior, hardware gate, Canon helper profile, operator recovery state machine은 이 스토리의 직접 수정 대상이 아니다.

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `docs/contracts/canonical-preset-recipe.md`
  - `docs/contracts/preset-bundle.md`
  - `docs/contracts/authoring-validation.md`
  - `docs/contracts/authoring-publication.md`
  - `docs/contracts/authoring-publication-payload.md`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/local-dedicated-renderer.md`
  - `src/shared-contracts/schemas/preset-core.ts`
  - `src/shared-contracts/schemas/preset-authoring.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/preset/preset_bundle.rs`
  - `src-tauri/src/preset/authoring_pipeline.rs`
  - `src-tauri/src/preset/default_catalog.rs`
  - `src-tauri/tests/preset_authoring.rs`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src-tauri/tests/session_manifest.rs`
  - `tests/fixtures/contracts/preset-bundle-v1/`
- 필요 시 새로 추가될 가능성이 큰 경로:
  - `tests/fixtures/contracts/canonical-preset-recipe-v1/`
  - recipe reference를 담는 대표 fixture 또는 seed asset 경로

### 테스트 요구사항

- TypeScript shared contract 검증:
  - `pnpm vitest run src/shared-contracts/contracts.test.ts`
- Rust publication/runtime 검증:
  - `cargo test --test preset_authoring`
  - `cargo test --test dedicated_renderer`
  - `cargo test --test capture_readiness`
  - `cargo test --test session_manifest`
- 계약 회귀 기준:
  - recipe reference와 XMP adapter reference가 함께 존재해야 한다.
  - path escape 또는 absolute path는 계속 reject되어야 한다.
  - pinned darktable version drift는 validation failure로 남아야 한다.
  - current booth truth (`previewReady`, `finalReady`, active session binding, customer-safe wording`)는 그대로 유지돼야 한다.
  - default seed bundle과 fixture bundle이 새 parser와 runtime loader를 동시에 통과해야 한다.

### 최신 기술 / 제품 컨텍스트

- 현재 저장소 baseline은 `React 19.2.4`, `react-router-dom 7.13.1`, `Zod 4.3.6`, `@tauri-apps/api`/`@tauri-apps/cli` `2.10.1`, Rust `tauri 2.10.3`, pinned darktable `5.4.1`이다. 이 스토리는 새 기술 스택 도입보다 현재 baseline을 authoritative truth로 정렬하는 작업에 가깝다. 이 문장은 현재 저장소 파일을 근거로 한 추론이다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- darktable 공식 릴리스 공지 기준, 최신 bug-fix stable release는 2026-02-05 공개된 `5.4.1`이며 프로젝트의 current pin과 일치한다. [Source: https://www.darktable.org/2026/02/darktable-5.4.1-released/]
- darktable 공식 sidecar 문서는 `.XMP` sidecar가 editing history를 담지만, import 이후에는 library database entry가 precedence를 가질 수 있다고 설명한다. 이는 XMP를 booth runtime의 유일한 business truth로 삼지 않고 adapter artifact로 다루려는 현재 architecture 방향과 맞는다. [Source: https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/]
- darktable 공식 `darktable-cli` 문서는 optional XMP sidecar를 export input으로 적용할 수 있고, style path는 `configdir`/`data.db` 의존을 동반한다고 설명한다. 이는 runtime truth를 style/library state가 아니라 explicit recipe + adapter artifact reference로 고정해야 한다는 판단을 뒷받침한다. [Source: https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/]

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.17: canonical preset recipe와 XMP adapter 기준 동결]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260412-044022.md]
- [Source: _bmad-output/implementation-artifacts/1-15-canon-helper-profile과-publication-contract-확정.md]
- [Source: _bmad-output/implementation-artifacts/1-16-windows-desktop-build-release-baseline과-ci-proof-설정.md]
- [Source: docs/contracts/preset-bundle.md]
- [Source: docs/contracts/authoring-validation.md]
- [Source: docs/contracts/authoring-publication.md]
- [Source: docs/contracts/authoring-publication-payload.md]
- [Source: docs/contracts/render-worker.md]
- [Source: docs/contracts/local-dedicated-renderer.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: src/shared-contracts/schemas/preset-core.ts]
- [Source: src/shared-contracts/schemas/preset-authoring.ts]
- [Source: src/shared-contracts/contracts.test.ts]
- [Source: src-tauri/src/preset/preset_bundle.rs]
- [Source: src-tauri/src/preset/authoring_pipeline.rs]
- [Source: src-tauri/src/preset/default_catalog.rs]
- [Source: src-tauri/tests/preset_authoring.rs]
- [Source: tests/fixtures/contracts/preset-bundle-v1/preset_soft-glow/2026.04.10/bundle.json]
- [Source: https://www.darktable.org/2026/02/darktable-5.4.1-released/]
- [Source: https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/]
- [Source: https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint-status, epics, architecture, PRD, UX, Story 1.15, Story 1.16, preset bundle/publication/render/session contracts, shared schemas, Rust runtime loaders, authoring pipeline, default seed catalog, contract fixture와 테스트를 교차 분석했다.
- 현재 저장소는 이미 “XMP adapter baseline”을 강하게 가지고 있지만, `canonical preset recipe` authoritative contract는 아직 없다. 따라서 이 스토리는 greenfield renderer 구현이 아니라, publication/runtime/authoring이 함께 참조할 editor-agnostic preset truth를 최소 형태로 먼저 잠그는 작업으로 정의했다.
- 최신 외부 확인은 darktable 공식 release/manual 문서만 사용했다.

### Debug Log References

- `pnpm vitest run src/shared-contracts/contracts.test.ts`
- `cargo test --test contracts_baseline`
- `cargo test --test preset_authoring`
- `cargo test --test dedicated_renderer`
- `cargo test --test capture_readiness -- --test-threads=1`
- `cargo test --test session_manifest -- --test-threads=1`

### Completion Notes List

- canonical preset recipe authoritative contract와 darktable adapter 분리 기준을 문서/fixture/공유 스키마에 반영했다.
- published bundle baseline을 `published-preset-bundle/v2`로 승격하고, runtime loader는 legacy `v1`도 계속 읽을 수 있게 유지했다.
- authoring publish/default seed/runtime loader가 같은 recipe truth를 사용하도록 정렬했고, booth semantics 회귀 테스트를 통과시켰다.

### Change Log

- 2026-04-12 06:20:00 +09:00 - canonical preset recipe baseline, bundle v2 publication contract, runtime loader compatibility, regression guardrail tests를 반영했다.

### File List

- _bmad-output/implementation-artifacts/1-17-canonical-preset-recipe와-xmp-adapter-기준-동결.md
- docs/contracts/canonical-preset-recipe.md
- docs/contracts/preset-bundle.md
- docs/contracts/authoring-publication.md
- docs/contracts/authoring-publication-payload.md
- docs/contracts/authoring-validation.md
- docs/contracts/render-worker.md
- docs/contracts/local-dedicated-renderer.md
- src/shared-contracts/schemas/preset-core.ts
- src/shared-contracts/schemas/preset-authoring.ts
- src/shared-contracts/schemas/presets.ts
- src/shared-contracts/dto/preset.ts
- src/shared-contracts/contracts.test.ts
- src-tauri/src/contracts/dto.rs
- src-tauri/src/preset/preset_bundle.rs
- src-tauri/src/preset/authoring_pipeline.rs
- src-tauri/src/preset/default_catalog.rs
- src-tauri/tests/contracts_baseline.rs
- src-tauri/tests/preset_authoring.rs
- src-tauri/tests/capture_readiness.rs
- tests/fixtures/contracts/preset-bundle-v2/preset_soft-glow/2026.04.10/bundle.json
- tests/fixtures/contracts/preset-bundle-v2/preset_soft-glow/2026.04.10/preview.jpg
- tests/fixtures/contracts/preset-bundle-v2/preset_soft-glow/2026.04.10/xmp/template.xmp
