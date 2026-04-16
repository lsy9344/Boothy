# Story 1.22: capture -> full-screen visible evidence chain trace reset

Status: done

Ordering Note: Story 1.22는 Story 1.21이 고정한 `same-capture preset-applied full-screen visible <= 2500ms` 기준 바로 다음에 와야 한다. 이 스토리가 먼저 capture correlation chain과 operator-safe evidence를 다시 잠가야 Story 1.23 local full-screen lane prototype이 잘못된 capture 매칭이나 불완전한 trace 위에 새 증거를 쌓지 않게 막을 수 있다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
capture부터 full-screen visible까지의 correlation chain을 다시 정렬하고 싶다,
그래서 잘못된 캡처 매칭이나 부분 계측 없이 같은 촬영본 기준의 진실한 evidence를 수집할 수 있다.

## Acceptance Criteria

1. same-capture KPI를 판정할 때, trace/evidence schema는 `capture request -> raw persistence -> preset-applied truthful artifact ready -> full-screen visible` 이벤트를 같은 correlation chain으로 다시 읽을 수 있어야 한다. first-visible lane은 별도 customer-safe projection으로 남을 수 있지만, truth chain과 섞이면 안 된다. 또한 각 증거는 최소 `sessionId`, `requestId`, `captureId`, `presetId`, `publishedVersion`, `laneOwner`, `routeStage`를 함께 남겨야 하며, 다른 capture나 다른 session의 값이 섞이면 안 된다.
2. operator-safe evidence bundle에서 fresh capture 1건을 열면 wrong-capture, stale-preview, cross-session attribution을 판별할 수 있어야 한다. 또한 fallback reason, visible owner 전환 시점, capture-time route/catalog snapshot이 누락되면 안 되며, 이후 policy 변경이 이미 기록된 booth run을 재해석하면 안 된다.

## Tasks / Subtasks

- [x] capture-to-full-screen correlation chain을 canonical trace/evidence 계약으로 재고정한다. (AC: 1)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`, `src-tauri/src/diagnostics/mod.rs`, `src-tauri/src/session/session_manifest.rs` 또는 동등 경로에서 `capture request`, `raw persisted`, `truthful artifact ready`, `full-screen visible`를 같은 `sessionId/requestId/captureId` 기준으로 묶는 canonical event family를 정리한다.
  - [x] `src/shared-contracts/schemas/hardware-validation.ts`, `src/shared-contracts/schemas/operator-diagnostics.ts`, 관련 DTO/fixture/test에서 같은 chain을 다시 읽을 수 있는 필드 집합을 명시하고, legacy `replacementMs` alias는 유지하되 chain truth를 대체하지 못하게 막는다.
  - [x] `visible owner`, `lane owner`, `route stage`, `fallback reason`, `preset version`, `capture-time snapshot path`를 누락 없이 남기고, 값이 비어 있거나 다른 capture와 충돌하면 fail-closed 하도록 규칙을 잠근다.

- [x] selected capture 기준의 operator-safe evidence 판별력을 보강한다. (AC: 1, 2)
  - [x] `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`, `docs/runbooks/preview-promotion-evidence-package.md`, `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` 또는 동등 경로에서 evidence bundle이 선택된 fresh capture 1건의 chain만 복사하고 요약하게 맞춘다.
  - [x] same-session이라도 다른 `captureId`를 끌어오면 wrong-capture로, capture-time snapshot 없이 live policy/catalog를 재조회하면 stale-preview attribution risk로, 다른 `sessionId` 흔적이 섞이면 cross-session leak risk로 판별되게 한다.
  - [x] fallback reason과 visible owner 전환 시점이 빠진 evidence는 `No-Go` 또는 bundle assembly failure로 처리하고 추정값으로 메우지 않게 한다.

- [x] trace reset을 새 local lane prototype과 분리된 owner 범위로 고정한다. (AC: 1, 2)
  - [x] Story 1.22는 trace/evidence reset owner이며, Story 1.23은 local full-screen lane prototype owner라는 sequencing을 문서와 테스트에서 분명히 남긴다.
  - [x] 기존 `preview-promotion-evidence.jsonl`, `timing-events.log`, capture-time `captured-preview-renderer-policy.json`, `captured-catalog-state.json` 흐름은 버리지 않고 확장하되, 이번 스토리에서 새 primary renderer route나 default decision 로직을 함께 구현하지 않는다.
  - [x] Story 1.24 hardware canary, Story 1.25 default decision/rollback gate, Story 1.13 guarded cutover ownership을 다시 가져오지 않도록 release/governance wording을 정리한다.

- [x] 회귀 검증으로 wrong-capture / stale-preview / cross-session attribution drift를 잠근다. (AC: 1, 2)
- [x] `tests/hardware-evidence-scripts.test.ts`, `src/shared-contracts/contracts.test.ts`, `src-tauri/tests/dedicated_renderer.rs`, `src/operator-console/services/operator-diagnostics-service.test.ts`, `src/governance/hardware-validation-governance.test.ts` 또는 동등 검증에 selected-capture-only, missing transition, stale snapshot, cross-session contamination 실패 케이스를 추가한다.
  - [x] operator diagnostics와 evidence bundle이 selected capture의 same-capture chain만 읽는지, 그리고 wrong-capture / stale-preview / cross-session attribution을 명확히 구분하는지 고정한다.
  - [x] first-visible만 있고 truthful artifact ready/full-screen visible chain이 닫히지 않은 경우 release-success처럼 해석되지 않는 negative case를 추가한다.

### Review Findings

- [x] [Review][Patch] Selected-capture bundle can never match the real `request-capture` event because the new filter requires `capture=<captureId>`, while the runtime logs that event with `capture=none` [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:44]
- [x] [Review][Patch] `visibleOwnerTransitionAtMs` is recorded as elapsed latency, not an event timestamp, so the canonical chain mixes absolute `*AtMs` fields with a relative value and can no longer be replayed in order [src-tauri/src/render/dedicated_renderer.rs:1044]
- [x] [Review][Patch] `sameCaptureFullScreenVisibleMs` is still derived from preview-ready timing before the later `recent-session-visible` UI event, so the new KPI can under-report the actual full-screen-visible latency seen by customers [src-tauri/src/render/dedicated_renderer.rs:1050]

## Dev Notes

### 왜 이 스토리가 지금 필요한가

- sprint plan은 새 preview architecture forward path를 `1.21 -> 1.22 -> 1.23 -> 1.24 -> 1.25`로 고정했다. metric reset 뒤 바로 trace/evidence reset이 와야 이후 prototype/canary/default decision이 같은 KPI와 같은 capture correlation 기준으로 검증된다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- epics도 Story 1.22를 `capture -> full-screen visible evidence chain trace reset` owner로 정의하고, Story 1.23을 local full-screen lane prototype owner로 분리했다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.22: capture -> full-screen visible evidence chain trace reset] [Source: _bmad-output/planning-artifacts/epics.md#Story 1.23: local full-screen lane prototype과 truthful artifact generation]

### 스토리 목적과 범위

- 이번 스토리의 핵심은 더 빠른 lane을 먼저 구현하는 일이 아니라, 어떤 이벤트와 어떤 artifact가 같은 촬영본을 설명하는지 다시 잠그는 일이다.
- 즉 이번 스토리는 아래를 소유한다.
  - selected capture 기준의 evidence chain reset
  - wrong-capture / stale-preview / cross-session attribution 판별력
  - operator-safe trace/bundle schema 정렬
- 아래 작업은 이번 스토리 범위가 아니다.
  - local full-screen lane prototype 자체 구현
  - hardware canary rerun과 `Go / No-Go` 주장
  - default promotion/rollback decision gate 구현
  - Story 1.13 최종 guarded cutover 재개

### 스토리 기반 요구사항

- architecture는 preview pipeline을 `first-visible lane`, `display-sized truthful artifact lane`, `truth/parity reference lane`으로 분리했고, hot path는 same-capture full-screen visible latency 기준으로 닫혀야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- architecture는 approved hardware validation이 한 recent session log에서 `request-capture`, `file-arrived`, `fast-preview-visible`, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`를 이어 읽을 수 있어야 한다고 명시한다. 이번 스토리는 이 규칙을 full-screen KPI 기준의 evidence chain으로 다시 정렬해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Application-Architecture]
- PRD는 release sign-off를 `same-capture preset-applied full-screen visible <= 2500ms`로 고정했고, tiny preview나 recent-strip success만으로는 합격을 선언할 수 없다고 못 박았다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003-Booth-Responsiveness-and-Preview-Readiness] [Source: _bmad-output/planning-artifacts/prd.md#Release-Gates]
- UX는 same-capture first-visible image를 먼저 보여줄 수 있어도 truthful preset-applied close가 준비되기 전까지 상태는 `Preview Waiting`으로 유지해야 한다고 요구한다. 따라서 event chain은 `early visible`과 `truthful close`를 구분해서 남겨야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview-Waiting-보호-흐름] [Source: _bmad-output/planning-artifacts/ux-design-specification.md#2.5-Experience-Mechanics]

### 현재 워크스페이스 상태

- `src/shared-contracts/schemas/hardware-validation.ts`는 이미 `sameCaptureFullScreenVisibleMs`와 legacy `replacementMs` alias를 함께 읽고 충돌을 막는다. 따라서 Story 1.22는 metric field를 새로 발명하기보다 capture correlation chain과 selected-capture evidence completeness를 강화하는 편이 맞다.
- `src/shared-contracts/schemas/operator-diagnostics.ts`와 `src-tauri/src/diagnostics/mod.rs`는 현재 operator summary에 `routeStage`, `laneOwner`, `fallbackReasonCode`, `warmState`, `sameCaptureFullScreenVisibleMs`를 투영한다. 하지만 chain 전체를 selected capture 기준으로 설명하는 판별력은 더 명확히 잠글 필요가 있다.
- `src-tauri/src/render/dedicated_renderer.rs`는 `preview-promotion-evidence-record/v1`와 `capture_preview_transition_summary`를 남기고 있다. 기존 evidence family를 버리지 말고, 이번 스토리에서 capture-request부터 full-screen visible까지의 correlation completeness를 같은 family 안에서 강화해야 drift를 줄일 수 있다.
- `tests/hardware-evidence-scripts.test.ts`와 `docs/runbooks/preview-promotion-evidence-package.md`는 이미 selected capture bundle, capture-time snapshot, fallback ratio, booth/operator visual evidence, rollback evidence 기준선을 갖고 있다. 이번 스토리는 이를 same-capture chain 판별용으로 더 엄격하게 만드는 follow-up이다.
- 별도 `project-context.md`는 발견되지 않았다.

### 이전 스토리 인텔리전스

- Story 1.19는 ETW/WPR/WPA/PIX + parity diff gate를 정착시키며 `preview-promotion-evidence.jsonl`, bundle scripts, parity/fallback/rollback evidence 규칙을 canonical input으로 만들었다. Story 1.22는 이 family를 버리면 안 되고, selected capture 기준의 chain truth를 더 엄격히 해야 한다. [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- Story 1.20은 route policy promotion과 capture-time policy/catalog snapshot 보존을 정리한 historical baseline이다. Story 1.22가 live policy recopy나 future-session reinterpretation을 허용하면 그 guardrail을 깨게 된다. [Source: _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md]
- Story 1.21은 `sameCaptureFullScreenVisibleMs`를 primary release field로 승격했다. Story 1.22는 이 field를 어떤 chain이 정당화하는지 다시 잠그는 follow-up이며, metric reset을 다시 뒤집어선 안 된다. [Source: _bmad-output/implementation-artifacts/1-21-metric-reset과-full-screen-2500ms-acceptance-정렬.md]

### 최근 구현 패턴과 git 인텔리전스

- 최근 커밋은 local renderer contracts, preset-applied rendering, diagnostics를 순차적으로 강화하는 방향이었다. 지금 저장소는 새 추상화를 늘리기보다 이미 있는 contract/evidence boundary를 더 진실하게 맞추는 단계에 가깝다. [Source: git log -5 --oneline]
- 따라서 Story 1.22도 file path, DTO family, evidence script 이름을 새로 대량 도입하기보다 existing boundary를 selected-capture-safe하게 재정렬하는 편이 안전하다.

### 구현 가드레일

- `preview-promotion-evidence.jsonl`, `timing-events.log`, capture-time snapshot 흐름을 버리고 두 번째 canonical trace family를 만들지 말 것.
- same-capture truth를 증명하지 못하는 값은 추정이나 alias로 채우지 말 것. 특히 missing `captureId`, missing visible owner transition, missing route snapshot은 fail-closed가 맞다.
- `sameCaptureFullScreenVisibleMs`는 release decision metric이지만, Story 1.22의 핵심은 숫자 하나가 아니라 그 숫자가 어떤 capture chain에서 나온 것인지 증명하는 것이다.
- `firstVisibleMs`는 보조 진단으로 남아도 되지만, truthful artifact ready 또는 full-screen visible 이벤트 없이 release-close 근거처럼 보이면 안 된다.
- active session truth와 future-session rollout truth를 섞지 말 것. bundle assembly가 live `preview-renderer-policy.json`나 live `catalog-state.json`를 다시 읽어 이미 기록된 booth run을 재해석하면 안 된다.

### 아키텍처 준수사항

- primary preview architecture는 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + darktable-compatible truth/parity reference`다. Story 1.22는 그 구조를 구현하는 owner가 아니라, 증거가 이 구조를 올바르게 설명하게 만드는 owner다. [Source: _bmad-output/planning-artifacts/architecture.md#Preview-Architecture-Realignment] [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- darktable-compatible path는 parity/fallback/final reference로 남는다. selected capture evidence도 resident lane 결과와 darktable reference가 같은 capture correlation 안에 있을 때만 의미가 있다. [Source: _bmad-output/planning-artifacts/architecture.md#Darktable-Capability-Scope]
- remote renderer / edge appliance는 reserve option이며, 이번 스토리에서 그 경로를 열거나 기본안처럼 취급하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Preview-Architecture-Realignment]

### 프로젝트 구조 요구사항

- 우선 수정/검토 후보 경로:
  - `src/shared-contracts/schemas/hardware-validation.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src/operator-console/services/operator-diagnostics-service.test.ts`
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/diagnostics/mod.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/tests/dedicated_renderer.rs`
  - `tests/hardware-evidence-scripts.test.ts`
  - `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
  - `scripts/hardware/Start-PreviewPromotionTrace.ps1`
  - `scripts/hardware/Stop-PreviewPromotionTrace.ps1`
  - `docs/runbooks/preview-promotion-evidence-package.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- scope를 넘기지 않도록 주의할 경로:
  - `src-tauri/src/branch_config/mod.rs`
  - `src-tauri/tests/branch_rollout.rs`
  - `C:\Users\KimYS\Pictures\dabi_shoot\branch-config\preview-renderer-policy.json`
- 위 경로는 capture-time snapshot truth를 읽는 용도까지만 고려하고, 새 promotion/default policy 동작 변경은 Story 1.25로 남긴다.

### UX 구현 요구사항

- 고객 경험에서는 "사진은 저장되었고 확인용 사진을 준비 중"이라는 truthful waiting semantics가 유지돼야 한다. 이번 스토리의 trace reset도 same-capture first-visible과 truthful preset-applied close를 이 의미에 맞게 분리해 남겨야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview-Waiting-보호-흐름]
- operator evidence는 internal diagnostics를 더 자세히 보여줄 수 있지만, customer-facing copy에 darktable, sidecar, ETW, PIX 같은 내부 용어를 노출하면 안 된다.
- same-capture first-visible image와 later truthful close는 같은 shot처럼 자연스럽게 이어져야 하므로, evidence chain도 visible owner transition을 잃지 않아야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#최신-사진-레일-(Latest-Photo-Rail)]

### 테스트 요구사항

- 최소 필수 자동 검증:
  - selected capture bundle이 같은 `sessionId/requestId/captureId/preset/version` chain만 복사한다.
  - missing capture-time policy/catalog snapshot, missing visible owner transition, missing fallback reason이 fail-closed 된다.
  - same-session 다른 `captureId` 혼입은 wrong-capture로, 다른 `sessionId` 혼입은 cross-session contamination으로, live snapshot recopy는 stale attribution risk로 판별된다.
  - operator diagnostics가 selected capture chain과 `sameCaptureFullScreenVisibleMs`를 같은 이야기로 읽고, first-visible-only false pass를 허용하지 않는다.
- 권장 추가 검증:
  - selected capture 외 다른 record가 evidence log에 섞여 있어도 bundle output은 한 record만 유지하는지 확인
  - truthful artifact ready/full-screen visible chain이 빠진 경우 release ledger row가 `No-Go`로 남는지 확인

### Evidence Expectations

- 체크인된 증거:
  - contract/schema/test가 selected capture correlation chain을 canonical로 읽는다.
  - evidence bundle runbook과 script가 capture-time snapshot을 유지하며 wrong-capture / stale-preview / cross-session attribution을 판별한다.
  - operator-safe diagnostics가 lane owner, visible owner transition, fallback reason, route stage, sameCaptureFullScreenVisibleMs를 함께 설명한다.
- 보존해야 하는 증거:
  - legacy `replacementMs` alias와 old-track evidence는 comparison-only로 남긴다.
  - Story 1.19 / 1.20이 만든 capture-time snapshot, parity/fallback/rollback evidence family는 유지한다.
- 이번 스토리에서 요구하지 않는 증거:
  - local lane prototype 성능 성공 선언
  - approved hardware `Go` claim
  - default route promotion 또는 rollback completion

### 최신 기술 확인 메모

- 이번 스토리는 외부 라이브러리 업그레이드보다 현재 repo의 frozen stack과 evidence contract를 재정렬하는 작업이다.
- `package.json` 기준 현재 프런트엔드 스택은 `React 19.2.4`, `Vite 8.0.1`, `Zod 4.3.6`, `@tauri-apps/api 2.10.1`이며, Story 1.22에서 dependency 추가나 버전 전환은 요구되지 않는다. [Source: package.json]
- 따라서 최신 기술 확인은 external upgrade research보다 현재 planning truth와 local contract baseline을 정확히 따르는 쪽이 중요하다. 이 문장은 현재 repo 상태를 근거로 한 해석이다.

### 금지사항 / 안티패턴

- selected capture evidence를 만들면서 whole-session log를 다시 전부 복사해 wrong-capture 구분을 흐리게 만드는 것 금지
- live policy/catalog를 재조회해 recorded booth run을 나중 정책으로 재해석하는 것 금지
- `replacementMs` alias만 맞으면 chain completeness가 없어도 release-success처럼 취급하는 것 금지
- Story 1.22 안에서 local full-screen lane prototype, canary validation, default decision, final cutover까지 한꺼번에 흡수하는 것 금지
- operator-safe evidence를 고객-facing 상태 copy로 그대로 노출하는 것 금지

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.22: capture -> full-screen visible evidence chain trace reset]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.23: local full-screen lane prototype과 truthful artifact generation]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003-Booth-Responsiveness-and-Preview-Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#Release-Gates]
- [Source: _bmad-output/planning-artifacts/architecture.md#Preview-Architecture-Realignment]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Application-Architecture]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#2.5-Experience-Mechanics]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview-Waiting-보호-흐름]
- [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- [Source: _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md]
- [Source: _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md]
- [Source: _bmad-output/implementation-artifacts/1-21-metric-reset과-full-screen-2500ms-acceptance-정렬.md]
- [Source: docs/runbooks/preview-promotion-evidence-package.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: src/shared-contracts/schemas/hardware-validation.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src-tauri/src/diagnostics/mod.rs]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: tests/hardware-evidence-scripts.test.ts]
- [Source: package.json]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint status, epics, PRD, architecture, UX, Story 1.19/1.20/1.21, runbook, hardware ledger, shared contracts, diagnostics projection, dedicated renderer evidence flow를 교차 분석했다.
- Story 1.22를 `selected capture evidence chain reset owner`로 정의하고, Story 1.23 prototype / Story 1.24 canary / Story 1.25 default decision / Story 1.13 final close ownership과 겹치지 않도록 범위를 고정했다.
- 현재 repo가 이미 `sameCaptureFullScreenVisibleMs`, capture-time snapshot, preview-promotion evidence record를 갖고 있으므로 새 family를 발명하기보다 selected-capture-safe reset을 강화하는 방향으로 문서를 정리했다.
- 별도 `project-context.md`는 발견되지 않았다.

### Debug Log References

- `Get-Content -Raw _bmad/bmm/config.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `rg -n -C 20 "1\\.22|1-22|Story 1\\.22" _bmad-output/planning-artifacts/epics.md _bmad-output/implementation-artifacts/sprint-status.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-21-metric-reset과-full-screen-2500ms-acceptance-정렬.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-20-resident-preview-lane-activation과-route-policy-promotion.md`
- `rg -n -C 6 "sameCaptureFullScreenVisibleMs|correlationId|captureId|routeStage|laneOwner|fallbackReason|visible owner|preview-promotion-evidence|truthful artifact|full-screen" _bmad-output/planning-artifacts/architecture.md _bmad-output/planning-artifacts/prd.md _bmad-output/planning-artifacts/ux-design-specification.md _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md _bmad-output/planning-artifacts/architecture-change-proposal-20260415.md`
- `Get-Content -Raw src/shared-contracts/schemas/hardware-validation.ts`
- `Get-Content -Raw src/shared-contracts/schemas/operator-diagnostics.ts`
- `Get-Content -Raw src-tauri/src/diagnostics/mod.rs`
- `Get-Content -Raw src-tauri/src/render/dedicated_renderer.rs`
- `Get-Content -Raw tests/hardware-evidence-scripts.test.ts`
- `Get-Content -Raw docs/runbooks/preview-promotion-evidence-package.md`
- `Get-Content -Raw package.json`
- `git log -5 --oneline`
- `pnpm vitest run tests/hardware-evidence-scripts.test.ts src/shared-contracts/contracts.test.ts src/operator-console/services/operator-diagnostics-service.test.ts src/governance/hardware-validation-governance.test.ts`
- `cargo test --test dedicated_renderer --manifest-path src-tauri/Cargo.toml`
- `pnpm lint`

### Completion Notes List

- selected capture chain에 `captureRequestedAtMs`, `rawPersistedAtMs`, `truthfulArtifactReadyAtMs`, `visibleOwner`, `visibleOwnerTransitionAtMs`를 추가해 capture-to-full-screen correlation을 canonical evidence record로 다시 잠갔다.
- operator diagnostics와 preview promotion evidence bundle이 selected capture `sessionId/requestId/captureId`만 읽도록 맞추고, whole-session timing log 복사를 중단했다.
- bundle assembly가 wrong-capture request mismatch, live snapshot 재조회, missing visible owner transition을 fail-closed 하도록 잠가 stale-preview / cross-session attribution drift를 차단했다.
- release baseline, runbook, hardware ledger wording에서 Story 1.22 trace reset owner와 Story 1.23/1.24/1.25/1.13 ownership 분리를 명시했다.
- `pnpm vitest run tests/hardware-evidence-scripts.test.ts src/shared-contracts/contracts.test.ts src/operator-console/services/operator-diagnostics-service.test.ts src/governance/hardware-validation-governance.test.ts`, `cargo test --test dedicated_renderer --manifest-path src-tauri/Cargo.toml`, `pnpm lint`를 통과했다.

### File List

- _bmad-output/implementation-artifacts/1-22-capture-full-screen-visible-evidence-chain-trace-reset.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/release-baseline.md
- docs/runbooks/preview-promotion-evidence-package.md
- release-baseline.md
- scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1
- src-tauri/src/contracts/dto.rs
- src-tauri/src/diagnostics/mod.rs
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/dedicated_renderer.rs
- src/governance/hardware-validation-governance.test.ts
- src/operator-console/services/operator-diagnostics-service.test.ts
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/hardware-validation.ts
- src/shared-contracts/schemas/operator-diagnostics.ts
- tests/fixtures/contracts/preview-promotion-evidence-record-v1.json
- tests/hardware-evidence-scripts.test.ts

### Change Log

- 2026-04-15 15:05 +09:00 - selected-capture evidence chain reset 완료. canonical record/bundle 필드 보강, fail-closed bundle assembly, operator diagnostics projection, governance/runbook wording, 회귀 테스트를 함께 갱신했다.
