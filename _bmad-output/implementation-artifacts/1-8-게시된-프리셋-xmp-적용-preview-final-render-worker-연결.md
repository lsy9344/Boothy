# Story 1.8: 게시된 프리셋 XMP 적용과 preview/final render worker 연결

Status: review

Correct Course Note: 아키텍처와 Epic 공통 요구사항은 이미 `darktable-cli` 기반 render worker를 프리셋 적용의 권위 경로로 고정했지만, Story 1.5/1.7/3.2/4.3 분해 과정에서는 "preview/final 상태가 존재한다"와 "선택된 프리셋이 실제로 적용된 preview/final이 생성된다"가 같은 것으로 취급되며 구현 책임이 빠졌다. 이 스토리는 그 누락을 메우는 corrective follow-up이며, Story 1.7 직후의 host-owned render truth를 닫는 최초의 직접 구현 소유자다.

## Correct Course Proposal

### 1. Issue Summary

- trigger: booth runtime이 published bundle을 실제 render worker 입력으로 쓰지 않아, selected preset이 preview/final 결과에 반영됐다는 제품 truth가 닫히지 않았다.
- 발견 시점: Story 1.5/1.7/3.2/4.3 구현 이후 current-session preview와 post-end completion을 검토하는 과정에서 "asset exists"와 "preset applied render exists"가 혼용된 것이 확인됐다.
- 제품 영향: 고객이 본 룩과 저장·전달되는 룩이 다를 수 있고, `previewReady`/`Completed`가 실제 render truth보다 먼저 올라갈 수 있었다.

### 2. Impact Analysis

- Epic impact: Epic 1 capture truth와 Epic 3 post-end truth를 다시 묶는 corrective story다.
- Story impact:
  - Story 1.5 preview readiness는 이제 raw copy나 placeholder가 아니라 runtime render 결과만 근거로 삼는다.
  - Story 1.7 RAW persistence 이후 경계는 유지하면서, 그 뒤 preview render owner를 명시했다.
  - Story 3.2 `Completed`는 final render truth가 닫히기 전까지 `export-waiting`에 머무르도록 재잠금했다.
  - Story 4.3 published bundle은 catalog summary뿐 아니라 runtime render metadata의 권위 artifact가 됐다.
- Artifact impact:
  - implementation: Rust host에 render seam 추가, preview/final gating 수정, capture-bound drift protection 추가
  - contracts: `preset-bundle`, `session-manifest`, `render-worker` 계약 보강
  - validation: runtime bundle loader test, capture-bound render regression, post-end final gating regression 추가

### 3. Recommended Approach

- chosen path: Direct Adjustment
- rationale: 기존 PRD/Architecture가 이미 약속한 권위 경로가 있었고, 누락된 연결만 실제 구현으로 닫는 편이 가장 작은 변경으로 제품 truth를 회복한다.
- scope: moderate
- remaining risk: hardware canonical evidence는 아직 없으므로 story close는 `review`에서 유지한다.

### Validation Gate Reference

- Reused supporting evidence families: `HV-05`, `HV-07`, `HV-08`, `HV-11`, `HV-12`
- Missing canonical close proof:
  - booth runtime가 게시된 preset bundle의 `xmpTemplatePath`를 실제로 소비한다는 증거
  - 서로 다른 preset을 고르면 촬영 직후 preview와 종료 후 final이 실제로 다르게 렌더된다는 증거
  - `previewReady`/`finalReady`가 darktable apply 결과물 없이 먼저 올라가지 않는다는 증거
- Current hardware gate: `No-Go`
- Close policy: automated pass만으로 닫지 않는다. 실제 부스 장비에서 "선택된 preset -> XMP apply -> preview/final output"까지 한 패키지로 검증한 canonical evidence가 필요하다.

## Story

booth customer로서,
내가 고른 프리셋이 촬영 직후 preview와 종료 후 결과물에 실제로 반영되길 원한다.
그래서 내가 본 룩과 실제로 저장·전달되는 룩이 다르지 않다고 믿을 수 있다.

## Acceptance Criteria

1. 활성 세션에 published preset binding이 있고 Story 1.7 경로로 RAW persistence가 성공한 상태에서 host가 render 후속 처리를 시작하면, booth runtime은 capture record에 묶인 `presetId + publishedVersion`으로 immutable published bundle을 다시 해석해야 한다. 또한 preview render는 그 bundle의 `xmpTemplatePath`, `darktableVersion`, `previewProfile`을 사용해 `darktable-cli`를 호출하고, 실제 raster preview 파일이 `renders/previews/` 아래에 생성된 뒤에만 `previewReady`를 기록해야 한다.
2. preview render가 성공하면 현재 세션 레일과 capture confirmation은 raw copy, placeholder SVG, bundle 대표 preview tile이 아니라 방금 촬영본에 선택된 preset이 적용된 booth-safe preview asset을 보여줘야 한다. preview render가 아직 끝나지 않았거나 실패하면 booth는 truthful `Preview Waiting` 또는 bounded failure guidance를 유지해야 하며, preset이 적용된 것처럼 보이는 false-ready를 만들면 안 된다.
3. 촬영 종료 후 host가 post-end truth를 평가할 때 final deliverable이 필요한 세션이라면, final render는 같은 capture record의 preset binding과 published bundle의 `xmpTemplatePath`, `darktableVersion`, `finalProfile`을 사용해 `renders/finals/` 아래 실제 산출물을 만든 뒤에만 `finalAsset`과 `finalReady`를 기록해야 한다. 또한 Story 3.2의 `Completed`는 이 final truth가 실제로 닫히기 전에는 올라가면 안 된다.
4. capture 이후 publish/rollback이나 live catalog 변경이 일어나더라도, 이미 저장된 capture의 preview/final render는 해당 capture record에 저장된 `activePresetId`와 `activePresetVersion`을 기준으로 동작해야 한다. 또한 새 live catalog pointer나 더 최신 published version으로 조용히 drift하면 안 된다.
5. `darktable-cli` 부재, bundle 메타데이터 손상, `xmpTemplatePath` 누락, timeout, queue saturation, render failure, wrong-session asset path 같은 예외가 생기면 host는 RAW와 현재 세션 자산을 보존한 채 bounded render failure truth만 기록해야 한다. 또한 RAW booth path에서 raw copy나 placeholder를 성공 산출물처럼 승격해 문제를 숨기면 안 된다.
6. 이 스토리는 승인된 booth hardware에서 두 개 이상의 서로 다른 published preset을 선택해 같은 booth path로 촬영했을 때 preview/final 결과가 실제로 달라지고, 그 차이가 선택된 bundle의 `xmpTemplatePath`와 correlation 되는 evidence가 수집되기 전에는 `review` 또는 동등한 pre-close 상태를 유지해야 한다.

## Tasks / Subtasks

- [x] runtime이 booth-safe preview summary만이 아니라 render-critical preset artifact를 읽도록 published bundle loader를 확장한다. (AC: 1, 3, 4, 5)
  - [x] `src-tauri/src/preset/preset_bundle.rs`에 booth catalog tile summary와 별도로 runtime render가 읽는 typed loader를 추가했다.
  - [x] runtime loader가 `presetId`, `publishedVersion`, `darktableVersion`, `xmpTemplatePath`, `previewProfile`, `finalProfile`을 canonical field로 반환하도록 고정했다.
  - [x] runtime apply truth는 `XMP sidecar template`를 우선 사용하고, `.dtpreset`는 publication artifact로만 유지한다.

- [x] host-owned darktable render worker와 bounded queue를 구현한다. (AC: 1, 3, 5)
  - [x] `src-tauri/src/render/mod.rs`에 camera helper와 분리된 render seam을 추가했다.
  - [x] preview/final 모드별 `configdir`, `library`, `--hq` 정책을 분리하고 pinned darktable 5.4.1 metadata를 강제한다.
  - [x] bounded queue와 typed failure reason을 render seam 한 곳에서 소유한다.

- [x] Story 1.5 / 1.7 preview path를 "preset-applied raster only" 기준으로 다시 잠근다. (AC: 1, 2, 5)
  - [x] `complete_preview_render_in_dir(...)`는 raw copy나 placeholder를 `previewReady`로 승격하지 않게 수정했다.
  - [x] render 성공 전까지는 `previewWaiting`을 유지하고, render 성공 뒤에만 preview rail이 갱신된다.
  - [x] sidecar placeholder가 늦게 도착해도 render completion 자체는 기다리지 않도록 regression을 추가했다.

- [x] Story 3.2 post-end truth와 final render를 실제 runtime worker에 연결한다. (AC: 3, 5)
  - [x] `finalReady`는 실제 final asset 생성 결과로만 올라가게 연결했다.
  - [x] final render가 없으면 `Completed`를 주장하지 않도록 post-end evaluator를 `export-waiting -> finalReady -> completed` 순으로 재정렬했다.
  - [x] render failure가 저장된 RAW와 기존 preview asset을 지우지 않도록 session-scoped asset 보존 규칙을 유지한다.

- [x] capture-bound preset version drift를 막는다. (AC: 4)
  - [x] render worker는 live catalog pointer가 아니라 capture record의 `activePresetId`, `activePresetVersion`으로 bundle을 resolve한다.
  - [x] active preset이 바뀐 뒤에도 기존 capture render가 원래 version을 유지하는 regression을 추가했다.

- [x] operator-safe diagnostics와 recovery seam을 보강한다. (AC: 2, 3, 5)
  - [x] 운영자 화면은 기존 `preview-render-blocked` / `timing-post-end-blocked` 경계를 유지한 채 render event를 추가로 남긴다.
  - [x] 고객 화면에는 darktable, XMP, CLI, queue, filesystem path를 노출하지 않고 safe failure copy만 유지한다.

- [x] contract / integration 테스트로 빠진 연결을 잠근다. (AC: 1, 2, 3, 4, 5)
  - [x] contract test: published bundle runtime loader가 `xmpTemplatePath`, `previewProfile`, `finalProfile`, `darktableVersion`을 loss 없이 읽는지 검증한다.
  - [x] Rust integration test: previewReady gating, finalReady gating, drift protection, post-end completion gating을 검증한다.
  - [x] UI/provider contract 영향은 host DTO shape 유지로 흡수했다.
- [ ] hardware validation: 같은 세션 또는 동일 검증 세트에서 서로 다른 published preset 두 개로 실제 촬영해 `renders/previews/`와 `renders/finals/` 산출물 차이, `session.json` preset binding, `bundle.json`의 `xmpTemplatePath`, operator diagnostics를 함께 남긴다. (AC: 6)

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 "preview가 존재한다"가 아니라 "선택된 preset이 실제로 적용된 preview/final이 존재한다"를 제품 truth로 닫는다.
- Story 1.5는 저장 성공과 preview readiness 분리를 닫았고, Story 1.7은 helper round-trip과 RAW persistence를 닫았으며, Story 3.2는 post-end completion taxonomy를 닫았다.
- 하지만 현재 구현에는 이 셋을 연결하는 `published preset artifact -> darktable-cli render worker -> session preview/final asset` 경계가 없다.
- 따라서 이 스토리는 Epic 1 corrective follow-up이지만, 실제로는 FR-004와 FR-007을 함께 정직하게 만들기 위한 runtime render core story다.

### 왜 이 스토리가 새로 필요해졌는가

- Epic 공통 요구사항은 이미 `darktable-cli` render worker를 프리셋 적용의 권위 경로로 정의했다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- architecture도 Rust render worker가 RAW ingest 뒤 `darktable-cli`를 호출해야 한다고 고정했다. [Source: _bmad-output/planning-artifacts/architecture.md#Darktable Capability Scope]
- 그러나 Story 1.5는 `preview asset` 존재와 `previewReady` truth를 닫을 뿐, 그 preview가 selected preset apply 결과물이어야 한다는 AC를 직접 소유하지 않는다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- Story 1.7도 preview/render kickoff 시점이 RAW persistence 뒤여야 한다고만 말하고, runtime이 published bundle의 `xmpTemplatePath`를 소비하는 구현 책임은 닫지 않는다. [Source: _bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md]
- Story 4.3은 immutable published bundle 생성까지만 닫고, booth runtime consumption은 catalog summary 수준에 머문다. [Source: _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md]

### 스토리 기반 요구사항

- runtime preset truth는 이름이 아니라 artifact이며, 최소 `xmpTemplatePath`, `darktableVersion`, preview/final profile을 포함해야 한다. [Source: darktable-reference-README.md]
- capture success와 render success는 분리돼야 하고, render worker는 camera helper와 별개 프로세스 경계여야 한다. [Source: darktable-reference-README.md]
- PRD는 고객이 고른 preset이 booth-safe preview/final behavior를 결정한다고 본다. false preview 또는 false completion은 허용되지 않는다. [Source: _bmad-output/planning-artifacts/prd.md#Published Preset Artifact Model] [Source: _bmad-output/planning-artifacts/prd.md#FR-007 Export Waiting, Final Readiness, and Handoff Guidance]
- NFR-003/NFR-005는 preview latency와 post-end completion truth를 요구하므로, render worker는 latency budget과 truthful fallback을 동시에 소유해야 한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness] [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]

### 선행 의존성과 구현 순서

- 직접 선행 책임:
  - Story 1.5: capture-saved / previewWaiting / previewReady contract
  - Story 1.7: helper-backed RAW persistence와 preview/render kickoff boundary
  - Story 3.2: post-end `Export Waiting` / `Completed` taxonomy
  - Story 4.3: immutable published bundle 생성
- 권장 구현 순서:
  1. published bundle runtime loader 확장
  2. darktable render worker와 queue 경계 구현
  3. preview path에서 real render result만 `previewReady`로 승격
  4. final path에서 real render result만 `finalReady`로 승격
  5. drift protection, diagnostics, tests, hardware evidence 정리

### 현재 워크스페이스 상태

- `src-tauri/src/preset/preset_bundle.rs`는 현재 booth catalog tile 표시용 preview 정보만 읽고, `xmpTemplatePath`나 `darktableVersion`을 runtime apply에 사용하지 않는다.
- `src-tauri/src/capture/ingest_pipeline.rs`는 preview asset을 raw copy, 기존 sidecar preview, 또는 fallback SVG로 만들 수 있고, 이 경로만으로 `previewReady`를 올릴 수 있다.
- `src-tauri/src/commands/capture_commands.rs`는 촬영 뒤 background thread에서 `complete_preview_render_in_dir(...)`만 호출한다.
- `src-tauri/src/handoff/mod.rs`와 일부 tests는 `finalReady` 상태를 소비하지만, 실제 runtime에서 `finalReady`를 생산하는 render worker 경로는 없다.
- `src-tauri/tests/capture_readiness.rs`에는 `finalReady` fixture가 존재하지만, 이는 현재 제품이 실제 final render를 닫았다는 뜻이 아니다.
- `history/current-session-photo-troubleshooting-history.md`는 현재 preview 문제의 핵심이 "preset 누락"이 아니라 "RAW 뒤 실제 preview raster를 만들지 못하거나, 만들어도 selected preset apply 경계가 없다"는 점을 정리해 둔다.

### 이전 스토리 인텔리전스

- Story 1.5는 current-session preview rail과 truthful waiting copy를 이미 갖고 있으므로, 이번 스토리는 UI를 새로 발명하기보다 host render truth를 정확하게 연결하는 편이 안전하다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- Story 1.7은 "preview/render kickoff는 RAW persistence 뒤"를 강하게 고정했기 때문에, darktable invocation도 이 경계를 깨면 안 된다. [Source: _bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md]
- Story 3.2는 `Completed`가 actual final truth 없이 오르면 안 된다는 guardrail을 이미 갖고 있으므로, 이번 스토리는 3.2를 우회하지 말고 `finalReady` producer를 제공해야 한다. [Source: _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md]
- Story 4.3은 published bundle immutability와 future-session-only 규칙을 닫았으므로, runtime render path도 live catalog drift 대신 capture-bound version resolution을 따라야 한다. [Source: _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md]

### 구현 가드레일

- booth RAW path에서 raw copy나 placeholder SVG를 `previewReady` 성공 산출물처럼 취급하지 말 것.
- runtime apply truth는 `XMP sidecar template`를 우선 사용하고, `.dtpreset`를 CLI apply truth로 착각하지 말 것.
- `darktable-cli` 호출을 capture helper, React UI, authoring surface에서 각각 따로 만들지 말 것.
- live catalog pointer를 다시 읽어 기존 capture의 preset version을 drift시키지 말 것.
- render failure를 조용한 raw fallback success로 덮지 말 것.
- 고객 화면에는 darktable, XMP, CLI, queue, configdir, library, filesystem path를 절대 노출하지 말 것.
- `previewReady`와 `finalReady`는 "파일이 실제로 존재한다"는 증거 없이 올리지 말 것.

### 아키텍처 준수사항

- runtime render core rule은 Rust render worker가 approved darktable-backed preset artifact를 `darktable-cli`로 실행하는 것이다. [Source: _bmad-output/planning-artifacts/architecture.md#Darktable Capability Scope]
- session manifest는 `raw`, `preview`, `final`, `render status`를 분리해 저장해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Closed Contract Freeze Baseline]
- React는 host-normalized truth만 소비해야 하며, 컴포넌트에서 "이 preset preview처럼 보이니 완료" 같은 ad-hoc 판정을 만들면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#API Boundaries]
- published preset bundle은 immutable하고, active session은 capture-bound version을 유지해야 한다. [Source: docs/contracts/preset-bundle.md]

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/src/preset/preset_bundle.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/handoff/mod.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src-tauri/tests/session_manifest.rs`
  - `src/shared-contracts/schemas/session-capture.ts`
  - `src/shared-contracts/schemas/session-manifest.ts`
  - `src/capture-adapter/services/capture-runtime.ts`
- 새로 추가될 가능성이 큰 경로:
  - `src-tauri/src/render/mod.rs`
  - `src-tauri/src/render/darktable_cli.rs`
  - `src-tauri/src/render/render_queue.rs`
  - `src-tauri/src/render/render_worker.rs`
  - `docs/contracts/render-worker.md`
- 만약 새 top-level `render/` 모듈을 만들지 않는다면, 최소한 darktable invocation/queue/failure mapping을 capture와 handoff 경계에서 중복 구현하지 않도록 단일 seam으로 모아야 한다.

### 테스트 요구사항

- 최소 필수 테스트:
  - published bundle runtime loader가 `xmpTemplatePath`를 실제로 읽는다.
  - preview job은 capture-bound `presetId + publishedVersion`으로 bundle을 resolve한다.
  - darktable invocation 전에는 `previewReady`가 올라가지 않는다.
  - preview render failure 시 raw persistence는 유지되지만 booth는 false preview를 보여주지 않는다.
  - final render가 없으면 `Completed`가 올라가지 않는다.
  - publish/rollback 뒤에도 기존 capture의 final/preview render는 같은 preset version을 유지한다.
  - 서로 다른 preset 두 개의 runtime render argument 또는 fixture output이 다름을 검증한다.
  - operator diagnostics가 preview/final render blockage를 구분한다.

### Git 인텔리전스

- 최근 커밋 흐름은 이미 post-end truth, camera helper handoff, session preview 안정화 쪽으로 이동해 있다.
- 최근 5개 commit title:
  - `8285fa4 Finalize post-end truth fixes and docs updates`
  - `0222e51 fix: stabilize camera helper handoff and session previews`
  - `7933f87 카메라연결/사진찍기 정상동작`
  - `989dbc9 chore: expand local ignore rules`
  - `2cac57f Restore camera helper fallback and document recovery`
- 즉 현재 팀 문맥은 "capture truth를 닫고, post-end truth를 정리하고, preview를 안정화한다" 쪽에 맞아 있으므로, 이번 스토리는 그 위에 실제 preset apply worker를 얹는 corrective extension으로 보는 것이 자연스럽다.

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md`
- 계약 문서: `docs/contracts/preset-bundle.md`
- 계약 문서: `docs/contracts/session-manifest.md`
- 계약 문서: `docs/contracts/authoring-publication.md`
- 계약 문서: `docs/contracts/render-worker.md`
- 참고 기준: `darktable-reference-README.md`
- 이력 문서: `history/current-session-photo-troubleshooting-history.md`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-007 Export Waiting, Final Readiness, and Handoff Guidance]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]
- [Source: _bmad-output/planning-artifacts/architecture.md#Darktable Capability Scope]
- [Source: _bmad-output/planning-artifacts/architecture.md#Closed Contract Freeze Baseline]
- [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- [Source: _bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md]
- [Source: _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md]
- [Source: _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md]
- [Source: docs/contracts/preset-bundle.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/contracts/authoring-publication.md]
- [Source: darktable-reference-README.md]
- [Source: history/current-session-photo-troubleshooting-history.md]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- 현재 제품이 `previewReady`와 `finalReady` 상태 vocabulary를 일부 갖고 있어도, selected preset apply truth를 생산하는 runtime worker는 비어 있음을 확인했다.
- 이번 story는 새 기능 추가라기보다, architecture와 epics가 이미 약속한 render worker를 실제 구현 책임으로 되돌리는 corrective story다.
- 구현 우선순위는 preview/final UI가 아니라 host runtime render truth, capture-bound preset version resolution, 그리고 false-ready / false-complete 차단이다.

### Completion Notes

- published bundle runtime loader, preview render truth, post-end final gating, capture-bound drift protection을 실제 코드로 연결했다.
- 계약 문서 `preset-bundle`, `session-manifest`, `render-worker`를 현재 구현 기준으로 보강했다.
- automated regression은 통과했지만, hardware canonical evidence가 아직 없으므로 story 상태는 `review`에 유지한다.
