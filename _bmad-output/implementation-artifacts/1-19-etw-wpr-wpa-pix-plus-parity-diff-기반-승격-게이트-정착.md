# Story 1.19: ETW/WPR/WPA/PIX + parity diff 기반 승격 게이트 정착

Status: done

Promotion Gate Note: Story 1.18이 resident GPU-first 후보와 seam evidence를 이미 남겼다. 이번 스토리는 새 렌더러를 또 발명하는 단계가 아니라, 그 증거를 어떤 도구와 기준으로 수집하고 해석해 승격 여부를 판정할지 고정하는 단계다. canonical release close owner는 계속 Story 1.13이며, 이번 스토리만으로 hardware `Go`를 주장하면 안 된다.

### Validation Gate Reference

- Canonical ledger: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Supporting evidence family:
  - `capture_preview_transition_summary` + session diagnostics package
  - host-owned `branch-config/preview-renderer-policy.json` snapshot
  - darktable baseline/fallback oracle against parity diff package
  - ETW/WPR/WPA/PIX 또는 동등 계측 runbook
  - Story 1.13 canonical `Go / No-Go` row
- Current hardware gate: `No-Go`
- Close policy:
  - `automated pass`만으로 renderer promotion을 닫지 않는다.
  - 속도 지표만 좋아도 parity drift, fallback 상시 발생, route-policy rollback 불능, active-session truth drift가 있으면 `No-Go`다.
  - 이번 스토리는 guarded cutover 의미를 바꾸지 않는다. shadow/canary/default/rollback ownership은 계속 host-owned route policy와 Story 1.13 ledger row가 소유한다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
latency, parity, fallback evidence를 한 기준으로 수집하고 싶다,
그래서 renderer 승격을 체감 속도와 품질 기준으로 동시에 판단할 수 있다.

## Acceptance Criteria

1. renderer 승격 판단용 booth evidence package는 같은 capture correlation 안에서 `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`, `laneOwner`, `fallbackReason`, `routeStage`, `warmState`, parity diff 결과를 함께 읽을 수 있어야 한다. 이 패키지는 `session.json`, `timing-events.log`, published `bundle.json`, `catalog-state.json`, route policy snapshot, booth/operator 화면 근거를 빠뜨리면 안 된다.
2. ETW/WPR/WPA/PIX 또는 동등 계측 체계는 runbook과 실행 스크립트 수준에서 고정되어야 한다. 어떤 trace를 켜는지, 어떤 correlation id로 묶는지, 어떤 산출물을 evidence package에 넣는지, 어떤 저장 경로를 쓰는지가 재실행 가능하게 문서화되어야 한다.
3. parity diff는 resident GPU lane 결과를 darktable baseline/fallback oracle against 기준으로 비교해야 하며, same-capture / same-session / same-preset-version 전제가 깨진 비교는 승격 근거가 되면 안 된다. 허용 임계치, diff 이미지 또는 수치 출력, fallback 발생 시 판정 규칙이 함께 남아야 한다.
4. hardware ledger와 release baseline은 속도만이 아니라 parity와 fallback 안정성까지 함께 판정해야 한다. `Go / No-Go` 행은 최소한 latency, parity, fallback ratio, route policy state, rollback evidence, blocker, owner, evidence path를 함께 읽을 수 있어야 하며, automated proof만으로 `Go`를 주장하면 안 된다.
5. 이번 스토리는 resident GPU lane의 최종 promotion을 닫지 않는다. Story 1.13의 guarded cutover ownership, active-session truth 불변, one-action rollback 요구사항을 유지한 채, 다음 hardware rerun이 실질적인 `Go / No-Go`를 판단할 수 있을 정도로 측정/판정 체계를 준비하는 범위까지만 닫아야 한다.

## Tasks / Subtasks

- [x] seam evidence와 승격 판정 입력을 단일 계약으로 고정한다. (AC: 1, 3, 4)
  - [x] `src-tauri/src/render/dedicated_renderer.rs`의 기존 `capture_preview_transition_summary`와 warm-state evidence를 재사용하고, story 문서/계약 문서에서 canonical field set으로 명시한다.
  - [x] `docs/contracts/local-dedicated-renderer.md`, `docs/contracts/render-worker.md`, `docs/contracts/session-manifest.md` 또는 동등 계약 문서에 evidence package 필수 항목, parity 비교 입력, route policy snapshot 요구사항을 추가한다.
  - [x] same-capture / same-session / same-preset-version이 아닌 비교를 parity 근거에서 제외한다는 규칙을 명시한다.

- [x] ETW/WPR/WPA/PIX 기반 수집 경로와 자동화 스크립트를 만든다. (AC: 1, 2, 5)
  - [x] `scripts/` 아래에 Windows용 capture start/stop, trace export, evidence bundle assemble 스크립트를 추가하거나 동등 경로를 만든다.
  - [x] host/renderer에 ETW provider 또는 동등 저오버헤드 trace entry를 추가해 `sessionId`, `requestId`, `captureId`, route stage, lane owner, fallback reason을 WPR/WPA에서 상관분석 가능하게 만든다.
  - [x] PIX timing capture, 필요한 경우 `pixtool.exe` 기반 CSV/PNG export, PDB 준비, 저장 디렉터리 정책을 runbook과 스크립트에서 맞춘다.

- [x] parity diff 산출물과 판정 규칙을 제품 기준으로 정리한다. (AC: 1, 3, 4)
  - [x] resident preview 결과와 darktable baseline/fallback 결과를 같은 capture 단위로 수집해 diff image, numeric score, threshold, pass/fail reason을 남기는 경로를 정한다.
  - [x] fallback이 발생했을 때 diff를 어떻게 기록하고 `No-Go` 또는 conditional evidence로 처리할지 규칙을 문서화한다.
  - [x] false-positive를 줄이기 위해 비교 대상 crop/resize/color-space 규칙을 명시하고, 비교 전처리가 고객 truth를 바꾸지 않는다는 전제를 잠근다.

- [x] runbook, ledger, release governance를 Story 1.13 close owner 기준으로 정렬한다. (AC: 2, 4, 5)
  - [x] `docs/runbooks/booth-hardware-validation-checklist.md`에 Story 1.19 측정 절차, evidence package 구조, parity/fallback 판정 항목을 추가한다.
  - [x] `docs/release-baseline.md`와 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 latency-only gate가 아니라 parity + fallback + rollback evidence gate를 함께 읽도록 정리한다.
  - [x] Story 1.13 문서와 충돌 없이 “1.19는 gate establishment, 1.13은 canonical close owner”라는 역할 분리를 유지한다.

- [x] 회귀/거버넌스 검증을 추가한다. (AC: 1, 2, 3, 4, 5)
  - [x] `src/governance/hardware-validation-governance.test.ts` 또는 동등 검증에 Story 1.19 관련 문서/ledger/runbook 정합성 체크를 추가한다.
  - [x] `src/shared-contracts/contracts.test.ts`, `src-tauri/tests/dedicated_renderer.rs` 또는 동등 테스트에서 evidence field, warm-state, parity 입력 drift를 막는다.
  - [x] scripts dry-run 또는 fixture 기반 검증으로 evidence bundle naming/path drift를 막는다.

## Dev Notes

### 스토리 목적과 범위

- 이번 스토리의 본질은 `측정 가능한 승격 게이트`를 제품 기준으로 잠그는 것이다.
- 렌더러 자체를 더 빠르게 만드는 일은 부수효과일 수 있지만, 주 목표는 아니다.
- Story 1.13이 여전히 canonical release close owner이고, 이번 스토리는 그 판단이 더 이상 사람 기억이나 ad-hoc 해석에 기대지 않게 만드는 역할이다.

### 이미 있는 기반

- `src-tauri/src/render/dedicated_renderer.rs`는 이미 `capture_preview_transition_summary`에 `laneOwner`, `fallbackReason`, `routeStage`, `warmState`, `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`를 남긴다.
- `src/shared-contracts/schemas/operator-diagnostics.ts`는 operator-safe warm-state projection을 이미 제공한다.
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`, `docs/runbooks/booth-hardware-validation-checklist.md`, `docs/release-baseline.md`, `src/governance/hardware-validation-governance.test.ts`는 release gate 문서/테스트의 현재 기준선이다.
- 현재 `scripts/`에는 dedicated renderer packaging과 Windows signing 준비만 있고, WPR/PIX/parity bundle 자동화는 아직 없다.

### 이전 스토리 인텔리전스

- Story 1.17은 canonical preset recipe를 먼저 잠갔다. parity diff는 이 internal truth를 소비한 결과를 비교해야지, XMP나 임시 산출물만을 유일 진실처럼 취급하면 안 된다. [Source: _bmad-output/implementation-artifacts/1-17-canonical-preset-recipe와-xmp-adapter-기준-동결.md]
- Story 1.18은 warm-state prototype과 seam evidence를 준비했고, Story 1.19가 읽을 필드를 이미 `capture_preview_transition_summary`에 남기도록 고정했다. 새 telemetry family를 따로 발명하기보다 이 evidence family를 canonical 입력으로 재사용하는 편이 안전하다. [Source: _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md]
- Story 1.13은 guarded cutover, rollback proof, route-policy ownership, canonical `Go / No-Go`를 소유한다. 1.19는 여기서 승격 claim을 가져오지 말고, 1.13이 rerun에서 판정할 수 있도록 준비만 해야 한다. [Source: _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md]

### 최근 구현 패턴과 git 인텔리전스

- 최근 커밋은 preset-applied rendering checkpoint, local renderer contract, session seam logging을 순차적으로 강화했다.
- 즉 현재 저장소는 “기능을 새로 만드는 단계”보다 “이미 생긴 seam과 계약을 release-grade gate로 묶는 단계”에 더 가깝다.
- 1.19에서도 파일 경로, DTO 이름, diagnostics field 이름을 불필요하게 다시 바꾸기보다 existing field를 gate semantics로 재사용하는 쪽이 drift를 줄인다. [Source: git log -5 --oneline]

### 아키텍처/제품 가드레일

- host-owned `branch-config/preview-renderer-policy.json`만 승격/rollback 경계를 제어해야 한다. dev-only 토글, React direct call, sidecar 단독 실행은 release substitute가 될 수 없다.
- darktable는 계속 baseline / fallback / parity oracle이다. resident GPU lane의 promotion 판단도 darktable against 비교로 읽어야 한다.
- active session truth는 forced update 없이 유지되어야 한다. runbook/ledger는 기존 세션 route snapshot과 preset binding이 나중 정책 변경으로 재해석되지 않는다는 규칙을 유지해야 한다.
- customer-facing UI에는 ETW, WPR, WPA, PIX, GPU, sidecar, fallback ratio 같은 내부 용어를 노출하면 안 된다.

### 구현 가드레일

- ETW provider나 trace event를 추가하더라도 기존 `timing-events.log` evidence family를 버리면 안 된다. WPR/WPA/PIX는 추가 상관분석 계층이지, 현재 canonical booth diagnostics를 대체하는 단일 진실이 아니다.
- parity diff는 동일 capture와 동일 preset binding 비교만 허용해야 한다. 다른 세션이나 다른 publishedVersion을 비교해 좋은 수치가 나와도 승격 근거가 될 수 없다.
- PIX 관련 수집은 필요한 범위의 display lane 측정에 집중하고, export/final lane 전체를 한 번에 끌어들이지 말 것. 이번 스토리의 범위는 여전히 `display + preset apply` 승격 게이트다.
- evidence bundle naming/path는 운영자가 재실행 가능한 수준으로 단순해야 한다. 회차별 폴더, session id, preset/version, route stage, executedAt가 함께 남는 구조가 바람직하다.

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `src-tauri/src/render/dedicated_renderer.rs`
  - `src-tauri/src/diagnostics/mod.rs`
  - `src/shared-contracts/schemas/dedicated-renderer.ts`
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/governance/hardware-validation-governance.test.ts`
  - `docs/contracts/local-dedicated-renderer.md`
  - `docs/contracts/render-worker.md`
  - `docs/contracts/session-manifest.md`
  - `docs/runbooks/booth-hardware-validation-checklist.md`
  - `docs/release-baseline.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`
- 새로 추가될 가능성이 큰 경로:
  - `scripts/hardware/*.ps1`
  - `scripts/hardware/*.json`
  - `docs/runbooks/preview-promotion-evidence-*.md`
  - `fixtures/hardware-validation/**` 또는 동등 fixture 경로

### 테스트 요구사항

- 최소 필수 자동 검증:
  - governance test가 Story 1.19 문서, runbook, ledger, release baseline의 gate semantics를 함께 잠근다.
  - dedicated renderer evidence formatting이 기존 seam 필드를 유지한다.
  - parity diff 입력/출력 contract가 drift하지 않는다.
  - evidence bundle path/naming 규칙이 scripts나 fixture 검증에서 재현 가능하다.
- 권장 추가 검증:
  - WPR/PIX capture dry-run이 실패했을 때도 booth product path를 깨지 않는지 확인
  - hardware ledger row template이 latency, parity, fallback ratio, rollback evidence를 모두 담는지 확인

### 최신 기술 / 제품 컨텍스트

- Microsoft ETW 공식 문서는 ETW를 Windows에 내장된 high-speed tracing facility로 설명하며, tracing session을 reboot이나 app restart 없이 동적으로 제어할 수 있다고 말한다. Boothy에서는 현장 장비에서 낮은 오버헤드로 렌더/전환/rollback seam을 수집해야 하므로 이 특성이 직접적으로 맞다. [Source: https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw]
- 같은 문서는 WPR을 권장 controller, WPA를 권장 consumer로 설명한다. 따라서 Story 1.19는 ad-hoc 로그 복사보다 `WPR capture -> WPA analysis`를 runbook에 고정하는 편이 맞다. [Source: https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw]
- Microsoft PIX timing capture 문서는 CPU/GPU 작업 시점, file I/O, memory allocation, counters를 한 캡처 안에서 함께 보고 상관관계를 분석할 수 있다고 설명한다. 이는 `first-visible -> preset-applied visible` seam, fallback 전환, GPU queue 관찰을 같은 회차에서 읽어야 하는 Boothy 목적과 맞는다. [Source: https://devblogs.microsoft.com/pix/timing-captures-new/]
- `pixtool.exe` 문서는 GPU capture에서 event list를 CSV로 저장하거나 screenshot/resource를 PNG로 저장할 수 있다고 설명한다. 이는 Story 1.19의 parity diff package를 반자동으로 만드는 보조 경로가 될 수 있다. 이 문장은 공식 PIX 도구 설명을 현재 repo의 parity package 요구사항에 적용한 해석이다. [Source: https://devblogs.microsoft.com/pix/pixtool/]

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.19: ETW/WPR/WPA/PIX + parity diff 기반 승격 게이트 정착]
- [Source: _bmad-output/planning-artifacts/architecture.md]
- [Source: _bmad-output/planning-artifacts/prd.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260412-044022.md]
- [Source: _bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- [Source: _bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md]
- [Source: _bmad-output/implementation-artifacts/1-17-canonical-preset-recipe와-xmp-adapter-기준-동결.md]
- [Source: _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: docs/runbooks/booth-hardware-validation-architecture-research.md]
- [Source: docs/release-baseline.md]
- [Source: src-tauri/src/render/dedicated_renderer.rs]
- [Source: src/shared-contracts/schemas/dedicated-renderer.ts]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src/governance/hardware-validation-governance.test.ts]
- [Source: scripts/prepare-dedicated-renderer-sidecar.mjs]
- [Source: https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw]
- [Source: https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder]
- [Source: https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-analyzer]
- [Source: https://devblogs.microsoft.com/pix/timing-captures-new/]
- [Source: https://devblogs.microsoft.com/pix/pixtool/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint-status, epics, PRD, architecture, UX spec, 1.13/1.18 story context, hardware runbook, ledger, governance tests, current seam logging 구현을 교차 분석했다.
- 이번 스토리는 resident lane 자체의 새 기능보다 `승격 판정 체계`를 잠그는 story로 다시 정렬했다.
- 별도 `project-context.md`는 발견되지 않았다.
- 최신 외부 확인은 Microsoft ETW/PIX 공식 문서 기준으로만 보강했다.

### Debug Log References

- `Get-Content -Raw _bmad/bmm/config.yaml`
- `Get-Content -Raw _bmad-output/implementation-artifacts/sprint-status.yaml`
- `Get-Content -Raw _bmad-output/planning-artifacts/epics.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/1-18-resident-gpu-first-display-lane-prototype과-warm-state-service-도입.md`
- `rg -n "Story 1\\.19|ETW|WPR|WPA|PIX|parity diff|resident GPU-first" _bmad-output/planning-artifacts`
- `rg -n "capture_preview_transition_summary|warmState|originalVisibleToPresetAppliedVisibleMs" src-tauri/src/render/dedicated_renderer.rs src/shared-contracts/schemas/*.ts`
- `Get-Content -Raw docs/runbooks/booth-hardware-validation-checklist.md`
- `Get-Content -Raw _bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `git log -5 --oneline`
- `pnpm test:run src/shared-contracts/contracts.test.ts src/governance/hardware-validation-governance.test.ts tests/hardware-evidence-scripts.test.ts tests/build-script.test.ts`
- `$env:CARGO_TARGET_DIR='C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\target-codex-story119'; cargo test --manifest-path src-tauri/Cargo.toml --test dedicated_renderer -- --nocapture`

### Completion Notes List

- Story 문서를 생성하고 sprint status를 `ready-for-dev`로 올렸다.
- Story 1.13 release close ownership을 유지한 채, 1.19의 역할을 측정/판정 체계 정착으로 분리했다.
- `preview-promotion-evidence.jsonl` structured record와 shared contract fixture를 추가해 same-capture 승격 판단 입력이 한 계약으로 다시 읽히게 했다.
- Windows hardware evidence flow를 `Start/Stop/New-PreviewPromotion*` 스크립트와 전용 runbook으로 고정해 trace plan, evidence bundle naming, parity gate를 재실행 가능하게 만들었다.
- hardware ledger와 release baseline이 latency-only가 아니라 parity, fallback ratio, route policy state, rollback evidence까지 함께 읽도록 정렬했다.
- Vitest governance/contract/script dry-run 검증과 Rust dedicated renderer integration 검증을 모두 재통과시켜 evidence drift를 막았다.

### File List

- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/1-19-etw-wpr-wpa-pix-plus-parity-diff-기반-승격-게이트-정착.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/local-dedicated-renderer.md
- docs/contracts/render-worker.md
- docs/contracts/session-manifest.md
- docs/release-baseline.md
- docs/runbooks/booth-hardware-validation-checklist.md
- docs/runbooks/preview-promotion-evidence-package.md
- scripts/hardware/Start-PreviewPromotionTrace.ps1
- scripts/hardware/Stop-PreviewPromotionTrace.ps1
- scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1
- src/governance/hardware-validation-governance.test.ts
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/schemas/dedicated-renderer.ts
- src/shared-contracts/schemas/hardware-validation.ts
- src/shared-contracts/schemas/index.ts
- src-tauri/src/render/dedicated_renderer.rs
- src-tauri/tests/dedicated_renderer.rs
- tests/fixtures/contracts/preview-promotion-evidence-record-v1.json
- tests/hardware-evidence-scripts.test.ts

### Review Findings

- [x] [Review][Patch] Trace start/stop 기본 경로가 서로 다른 evidence 디렉터리를 가리킴 [scripts/hardware/Start-PreviewPromotionTrace.ps1:45]
- [x] [Review][Patch] Evidence bundle이 canonical evidence 누락 시 실패하지 않고 route truth를 추정값으로 채움 [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:226]
- [x] [Review][Patch] Parity gate와 evidence record 선택이 same-capture / same-session / same-preset-version 제약을 강제하지 않음 [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:39]
- [x] [Review][Patch] Evidence bundle 계약에 booth/operator visual proof와 rollback evidence를 담을 슬롯이 없음 [src/shared-contracts/schemas/hardware-validation.ts:35]
- [x] [Review][Patch] Bundle이 선택된 capture record 대신 전체 preview-promotion evidence log를 복사함 [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:311]
- [x] [Review][Patch] Corrupt 또는 non-image 입력이 parity 측정을 structured failure가 아니라 script crash로 끝냄 [scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1:136]
- [x] [Review][Patch] Preview evidence 기록 실패가 capture completion 경로에서 무시됨 [src-tauri/src/render/dedicated_renderer.rs:842]

## Change Log

- 2026-04-13 12:34:59 +09:00 - Review patch set applied: trace root default 통일, evidence bundle fail-closed/correlation enforcement 추가, visual/rollback bundle contract 확장, selected-record-only bundle copy 보장, invalid parity decode를 structured failure로 정리, preview promotion evidence write failure를 timing log에 노출.
- 2026-04-13 11:56:00 +09:00 - Story 1.19 implementation complete: structured preview-promotion evidence record 추가, Windows trace/evidence bundle scripts 추가, parity gate/runbook/ledger/release baseline 정렬, governance/contract/script/Rust 검증 재통과, story 상태를 `review`로 전환.
