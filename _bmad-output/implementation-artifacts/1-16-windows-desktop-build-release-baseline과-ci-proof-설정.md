# Story 1.16: Windows desktop build-release baseline과 CI proof 설정

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
초기 Windows desktop build / release baseline을 먼저 확보하고 싶다,
그래서 기능 개발과 별개로 packaging, CI validation, release proof 기준을 안정적으로 유지할 수 있다.

## Acceptance Criteria

1. 로컬 개발 환경에서 baseline build proof를 실행하면 `pnpm build:desktop` 또는 동등한 로컬 baseline build 경로가 동작해야 하고, 실패 시 packaging 기준을 확인할 수 있는 문서 기준이 존재해야 한다.
2. 저장소에서 Windows release 검증을 자동화할 때 `.github/workflows/release-windows.yml`가 unsigned baseline validation path를 제공해야 한다.
3. `pnpm release:desktop` 및 signing-ready 입력 규칙이 문서와 CI workflow에서 일치해야 한다.
4. active booth session을 강제 업데이트하지 않는 release guardrail이 유지되어야 한다.
5. automated proof와 hardware proof가 별도 gate라는 사실이 운영 기준에 반영되어야 한다.

## Tasks / Subtasks

- [x] 로컬 Windows baseline build 경로를 canonical release baseline으로 고정한다. (AC: 1, 3)
  - [x] `package.json`의 `build:desktop`과 `release:desktop`를 canonical entrypoint로 유지하고, 별도의 중복 desktop build 명령을 추가하지 않는다.
  - [x] `docs/release-baseline.md`가 현재 저장소의 실제 prerequisite와 일치하도록 정리한다: Windows canonical proof path, Node.js `20.19+` 또는 `22.12+`, `pnpm` 10.x, Rust `1.77.2+`, signing input source.
  - [x] `src-tauri/tauri.conf.json`과 release 문서가 updater 비활성 baseline, unsigned local proof, signing-ready draft proof의 구분을 같은 의미로 유지하도록 정렬한다.
  - [x] baseline build 실패 시 개발자가 확인해야 할 packaging 기준, prerequisite, output expectation을 문서에서 바로 찾을 수 있게 만든다.

- [x] `.github/workflows/release-windows.yml`를 draft release proof workflow로 정렬하거나 보강한다. (AC: 2, 3)
  - [x] `pull_request` to `main`과 `push` to `main`에서 unsigned Windows baseline validation이 수행되도록 유지한다.
  - [x] `workflow_dispatch`와 `boothy-v*` tag에서 signing-ready draft build path가 수행되도록 유지한다.
  - [x] release workflow 구현 경로를 하나로 정한다: 현재처럼 explicit `pnpm build:desktop` / `pnpm release:desktop`를 유지하거나, 공식 Tauri GitHub Action으로 치환하되 중복 workflow path를 남기지 않는다.
  - [x] signing-ready path가 문서화된 입력 규칙(`BOOTHY_WINDOWS_CERT_PATH` 또는 `BOOTHY_WINDOWS_CERT_BASE64`, `BOOTHY_WINDOWS_CERT_PASSWORD`, optional timestamp URL)과 충돌하지 않게 만든다.
  - [x] CI proof가 release review에서 확인 가능한 evidence를 남기도록 한다. 필요하면 workflow summary 또는 artifact upload를 사용하되, hardware proof를 자동화 proof로 가장하지 않는다.
  - [x] runner drift가 문제라면 `windows-latest`를 계속 쓸지, `windows-2025`로 pin할지 명시적으로 결정한다.

- [x] release proof를 branch rollout / active-session safety와 연결한다. (AC: 4)
  - [x] release baseline 변경은 `docs/contracts/branch-rollout.md`의 `deploymentBaseline`, `rollbackBaseline`, `pendingBaseline`, `activeSession.lockedBaseline` 의미를 재사용해야 한다.
  - [x] active session 중에는 rollout/rollback이 즉시 적용되지 않고 safe transition point까지 defer된다는 규칙을 유지한다.
  - [x] updater, forced auto-install, active session interruption 같은 별도 경로를 새로 추가하지 않는다.
  - [x] 필요 시 `src-tauri/tests/branch_rollout.rs` 또는 동등한 테스트에서 active-session-safe rollout/rollback 회귀를 보강한다.

- [x] automated proof와 hardware proof의 분리 기준을 운영 산출물에 반영한다. (AC: 5)
  - [x] `docs/release-baseline.md`와 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 `Automated Pass`, `Hardware Pass`, `Go / No-Go`, blocker, owner, evidence path를 함께 보도록 같은 운영 의미를 유지한다.
  - [x] CI 성공만으로 booth `Ready` / `Completed` truth 또는 release close를 주장하지 않도록 문서와 workflow 설명을 맞춘다.
  - [x] hardware validation을 독립 기능 스토리로 되돌리지 않고 cross-cutting release gate로 유지한다.

- [x] baseline release proof에 필요한 실행 검증을 보강한다. (AC: 1, 2, 3, 4)
  - [x] 가능한 환경에서는 `pnpm build:desktop`을 실제로 실행해 Windows baseline proof를 확인한다.
  - [x] branch rollout release safety와 관련된 Rust 테스트를 실행한다.
  - [x] workflow 변경이 크면 lint 또는 dry-run 대체 검증을 추가해 YAML drift를 줄인다.

### Review Findings

- [x] [Review][Defer] Story 1.16 review unexpectedly rewrites unrelated sprint status entries outside this story’s scope [_bmad-output/implementation-artifacts/sprint-status.yaml:50] — deferred, pre-existing mixed working-tree state. 1.16에서 실제로 반영한 내용은 story 1.16의 `in-progress` 전환과 `last_updated` 갱신뿐이다.
- [x] [Review][Defer] Story 1.16 review also rewrites hardware-governance expectations for other stories, coupling release-baseline work to unrelated close-state history [src/governance/hardware-validation-governance.test.ts:23] — deferred, pre-existing governance follow-up owned by Story 6.2 / mixed working-tree changes. 1.16 deliverable 범위에서는 제외한다.
- [x] [Review][Resolved] release workflow now rejects `workflow_dispatch` outside `main`, records proof path/outcome plus hardware-gated promotion state, and injects documented `BOOTHY_WINDOWS_CERT_*` secrets into the draft release path.
- [x] [Review][Resolved] release governance now guards both `docs/release-baseline.md` and the legacy root `release-baseline.md` copy, and signing-input validation now rejects malformed base64/timestamp inputs instead of silently accepting them.
- [x] [Review][Patch] `workflow_dispatch` on `main` now runs only the draft release path [.github/workflows/release-windows.yml:52]
- [x] [Review][Patch] Blocked manual dispatch no longer uploads a release-proof artifact [.github/workflows/release-windows.yml:75]
- [x] [Review][Patch] Signing mode/source evidence now propagates into proof capture [scripts/prepare-windows-signing.mjs:160]
- [x] [Review][Patch] Promotion state now stays on `release-hold` when automation fails or gated hardware evidence is incomplete [.github/workflows/release-windows.yml:113]
- [x] [Review][Patch] GitHub-hosted draft release path now documents and enforces `BOOTHY_WINDOWS_CERT_BASE64`-based signing input on hosted runners [.github/workflows/release-windows.yml:63]
- [x] [Review][Resolved] Canonical local baseline proof now passes after clearing pre-existing TypeScript blockers, setting the product bundle identifier, and auto-preparing the dedicated renderer packaging stub [package.json:11]

## Dev Notes

### 스토리 범위와 제품 목적

- 이 스토리는 새 customer-facing 기능을 만드는 작업이 아니라, Windows desktop release baseline과 CI proof를 제품 기준으로 닫는 foundational story다.
- 목표는 release pipeline을 거창하게 확장하는 것이 아니라, 이미 저장소에 존재하는 로컬 빌드 명령, draft CI workflow, release baseline 문서, branch rollout guardrail을 하나의 authoritative baseline으로 맞추는 것이다.
- Story 1.14가 shared contract baseline을 닫고, Story 1.15가 Canon helper / publication contract를 닫았다면, Story 1.16은 build/release proof ownership을 닫는 역할이다.

### 왜 지금 필요한가

- Epic 1 보정안은 foundation-first 순서를 `1.14 -> 1.15 -> 1.16 -> 1.1`로 명시했고, implementation readiness report도 같은 순서를 권장한다. [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260410-003446.md] [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260410.md]
- architecture는 frozen contract surface의 마지막 foundational scope로 Story 1.16의 build/release proof ownership을 직접 언급한다. [Source: _bmad-output/planning-artifacts/architecture.md#Closed Contract Freeze Baseline]
- PRD의 NFR-006은 staged rollout, rollback, no forced update during active session을 release-level requirement로 고정한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-006 Safe Local Packaging, Rollout, and Version Pinning]

### 현재 워크스페이스 상태

- 이 저장소에는 이미 release baseline의 핵심 시작점이 있다.
  - `package.json`에 `build:desktop`, `release:desktop`가 정의되어 있다.
  - `docs/release-baseline.md`가 release baseline과 signing input, guardrail, truth gate를 정리한다.
  - `.github/workflows/release-windows.yml`가 draft workflow로 존재한다.
  - `src-tauri/tauri.conf.json`은 desktop bundle baseline을 이미 가지고 있다.
  - `docs/contracts/branch-rollout.md`와 `src-tauri/tests/branch_rollout.rs`가 active-session-safe rollout semantics를 이미 정의한다.
- 즉, 이 스토리는 greenfield release setup이 아니라 existing draft baseline을 harden하고 정렬하는 작업이다.
- 별도의 `project-context.md`는 발견되지 않았다. 이 스토리에서는 `README.md`, planning artifacts, release docs를 canonical context로 본다.

### 이전 스토리 인텔리전스

- Story 1.15는 build/release proof를 1.16의 책임으로 남기고 helper packaging이나 broader release pipeline 변경으로 scope를 번지지 않게 하라고 분명히 남겼다. [Source: _bmad-output/implementation-artifacts/1-15-canon-helper-profile과-publication-contract-확정.md]
- Story 1.14와 1.15에서 session/preset/error/capability/helper/publication contract를 이미 잠갔으므로, 1.16은 그 계약을 다시 설계하는 스토리가 아니다.
- 최근 커밋은 preview latency와 capture truth 개선에 집중돼 있다.
  - `8c30be7 Improve focus retry guidance`
  - `2c89c40 Finalize thumbnail latency worker updates and docs`
  - `9c56c37 Add session seam logging for thumbnail latency reduction`
  - `b24cfc4 Reduce recent-session preview latency and capture wait blocking`
  - `12309fa Record thumbnail validation and ship fast preview fallback`
- 따라서 1.16은 preview/capture corrective work와 섞이지 않게 release baseline 정합성에 집중해야 한다.

### 제품/아키텍처 가드레일

- active booth session은 절대 forced update로 끊기면 안 된다. rollout/rollback은 safe transition point에서만 적용된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-006 Safe Local Packaging, Rollout, and Version Pinning] [Source: docs/contracts/branch-rollout.md]
- release proof는 booth `Ready`, preview truth, completion truth를 대체하지 않는다. automated build/test pass는 implementation readiness일 뿐이고, hardware truth gate는 별도다. [Source: docs/release-baseline.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- updater auto-install path를 이 스토리에서 열면 안 된다. release baseline은 rollout/rollback governance와 safe transition semantics만 다루고, in-session mutation 경로를 새로 만들지 않는다.
- customer-facing copy, operator semantics, helper semantics, publication semantics를 이 스토리에서 다시 정의하지 말 것. 이 스토리의 산출물은 release baseline과 CI proof 정합성이다.

### 구현 가드레일

- `package.json`의 desktop scripts를 기준으로 release workflow를 맞추고, workflow 안에서 별도 ad-hoc command 조합을 발명하지 말 것.
- `.github/workflows/release-windows.yml`가 이미 있으므로 새 release workflow 파일을 추가하기보다 기존 파일을 정렬/보강하는 쪽이 우선이다.
- GitHub release upload를 도입하더라도 build-only validation path와 draft release path가 서로 다른 의미를 갖는다는 점을 유지해야 한다.
- signing certificate material은 저장소에 커밋하면 안 된다. path 또는 base64 PFX는 local env / CI secret로만 주입한다.
- `windows-latest`는 영구적으로 같은 OS 이미지를 뜻하지 않는다. runner drift가 baseline reproducibility를 흔들면 pinning을 검토하되, 문서와 workflow를 함께 바꿔야 한다.
- `createUpdaterArtifacts`, updater config, 강제 재시작/업데이트 정책을 활성화하는 것은 이 스토리의 성공 기준이 아니다.

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `package.json`
  - `.github/workflows/release-windows.yml`
  - `docs/release-baseline.md`
  - `docs/contracts/branch-rollout.md`
  - `README.md`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/tests/branch_rollout.rs`
- 필요 시 추가될 수 있는 후보 경로:
  - workflow helper script 또는 validation script 경로
  - CI artifact 설명용 문서/summary 템플릿
- 수정 우선순위는 문서와 workflow 정렬이 먼저이고, rollout safety regression 보강은 그 다음이다.

### 테스트 요구사항

- release baseline 검증:
  - `pnpm build:desktop`
  - 필요 시 `pnpm release:desktop`의 signing-ready dry path
- rollout safety 검증:
  - `cargo test --test branch_rollout`
- 기본 회귀 검증:
  - `pnpm test:run`
- Windows canonical proof path가 필요하므로, desktop bundle 검증은 Windows 환경에서 확인해야 한다.

### 최신 기술 / 제품 컨텍스트

- 현재 저장소 baseline은 `pnpm@10.31.0`, Node.js `20.19+` 또는 `22.12+`, Rust `1.77.2+`, `@tauri-apps/api` / `@tauri-apps/cli` `2.10.1`, Rust `tauri` `2.10.3`이다. 새 release path는 이 baseline을 기준으로 맞추는 편이 맞다. 이 문장은 현재 저장소 파일을 근거로 한 추론이다. [Source: package.json] [Source: src-tauri/Cargo.toml] [Source: README.md]
- 2026-04-10 확인 기준 공식 `tauri-action` 저장소는 build-only usage와 GitHub Release upload usage를 모두 지원하며, build만 원하면 release inputs를 생략할 수 있다고 안내한다. 즉, Boothy는 raw `pnpm` step을 유지해도 되고 공식 action으로 치환해도 되지만 둘을 중복 유지하면 안 된다. [Source: https://github.com/tauri-apps/tauri-action]
- 2026-04-10 확인 기준 GitHub Docs는 `windows-latest`가 GitHub가 제공하는 최신 stable image일 뿐 vendor의 최신 OS를 의미하지 않는다고 명시한다. runner image drift가 proof stability를 흔들면 `windows-2025` pinning을 검토할 근거가 된다. [Source: https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job]

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.16: Windows desktop build-release baseline과 CI proof 설정]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-006 Safe Local Packaging, Rollout, and Version Pinning]
- [Source: _bmad-output/planning-artifacts/prd.md#Release Gates]
- [Source: _bmad-output/planning-artifacts/architecture.md#Release packaging]
- [Source: _bmad-output/planning-artifacts/architecture.md#Closed Contract Freeze Baseline]
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260410.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260410-003446.md]
- [Source: _bmad-output/implementation-artifacts/1-15-canon-helper-profile과-publication-contract-확정.md]
- [Source: package.json]
- [Source: .github/workflows/release-windows.yml]
- [Source: docs/release-baseline.md]
- [Source: docs/contracts/branch-rollout.md]
- [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- [Source: README.md]
- [Source: src-tauri/tauri.conf.json]
- [Source: src-tauri/tests/branch_rollout.rs]
- [Source: src-tauri/Cargo.toml]
- [Source: https://github.com/tauri-apps/tauri-action]
- [Source: https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint-status, epics, PRD, architecture, implementation-readiness report, sprint change proposal, Story 1.15, release baseline doc, branch rollout contract, hardware validation ledger, current workflow/config/package files를 교차 분석했다.
- 현재 저장소에는 release baseline 초안이 이미 있으므로, 이번 스토리는 “새 release path 생성”보다 “existing build/doc/workflow/guardrail 정렬과 보강”으로 정의했다.
- 최신 외부 확인은 공식 Tauri GitHub Action 저장소와 GitHub Actions runner 문서만 사용했다.

### Debug Log References

- 2026-04-10: `pnpm test:run src/governance/hardware-validation-governance.test.ts src/governance/release-baseline-governance.test.ts` 통과.
- 2026-04-10: `cargo test --test branch_rollout` 통과.
- 2026-04-10: `pnpm build:desktop` 통과, unsigned local proof로 MSI/NSIS bundle 생성 확인.
- 2026-04-10: `pnpm release:desktop` 통과, signing-ready draft proof로 release MSI/NSIS bundle 생성 확인.
- 2026-04-10: `cargo test --test capture_readiness readiness_stays_blocked_when_helper_truth_is_absent_even_in_runtime_dir` 통과.

### Completion Notes List

- release baseline canonical command를 `build:desktop` unsigned local proof와 `release:desktop` draft release proof로 정렬했다.
- Windows release baseline 문서와 legacy root copy를 현재 prerequisite, output expectation, signing input validation, CI evidence 기준에 맞게 갱신했다.
- `.github/workflows/release-windows.yml`를 `windows-2025` baseline, proof summary, artifact upload 흐름으로 보강했다.
- `scripts/prepare-windows-signing.mjs`를 추가해 signing input source validation과 unsigned draft fallback을 canonical release path에 연결했다.
- `src-tauri/tauri.conf.json`에 updater artifact 비활성 baseline을 명시했다.
- code review 후속으로 release workflow를 `main`-scoped manual dispatch, documented secrets wiring, governance/rollout test execution, proof path/outcome/promotion-state evidence까지 보강했다.
- code review 후속으로 release baseline 문서 두 사본의 동기화 검증과 malformed signing input rejection을 governance 기준에 반영했다.
- pre-existing TypeScript blockers, Tauri default bundle identifier, dedicated renderer packaging gap을 정리해 `pnpm build:desktop`과 `pnpm release:desktop`의 실제 proof를 완료했다.

### File List

- _bmad-output/implementation-artifacts/1-16-windows-desktop-build-release-baseline과-ci-proof-설정.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- .github/workflows/release-windows.yml
- docs/release-baseline.md
- package.json
- release-baseline.md
- scripts/prepare-dedicated-renderer-sidecar.mjs
- scripts/prepare-windows-signing.mjs
- src-tauri/tauri.conf.json
- src-tauri/tests/capture_readiness.rs
- src/governance/release-baseline-governance.test.ts
- sidecar/dedicated-renderer/main.rs
- sidecar/dedicated-renderer/README.md
- src/booth-shell/screens/ReadinessScreen.tsx
- src/capture-adapter/services/capture-runtime.test.ts
- src/operator-console/providers/operator-diagnostics-provider.tsx
- src/operator-console/services/operator-diagnostics-service.test.ts
- src/preset-authoring/screens/PresetLibraryScreen.tsx
- src/session-domain/services/active-preset.test.ts
- src/types/prepare-windows-signing.d.ts
- tsconfig.app.json
- _bmad-output/planning-artifacts/architecture.md

### Change Log

- 2026-04-10: Windows release baseline 문서, workflow, updater-disabled Tauri baseline, signing input validation script, governance test를 정렬했다. branch rollout regression과 governance tests는 통과했지만, desktop build proof는 기존 워크트리 타입 오류로 인해 아직 완료되지 않았다.
- 2026-04-10: code-review 지적을 반영해 1.16 산출물 범위를 다시 정리했다. unrelated sprint/governance drift는 pre-existing mixed working-tree issue로 분리했고, 1.16 파일 목록에서 제외했다.
- 2026-04-10: 서브에이전트 코드리뷰 후속으로 workflow_dispatch release boundary를 `main`으로 제한하고, proof path/outcome 및 hardware-gated promotion state를 release evidence에 추가했으며, malformed signing input rejection과 release-baseline 문서 동기화 검증을 보강했다.
- 2026-04-10: pre-existing TypeScript blockers를 정리하고, product bundle identifier를 `com.boothy.desktop`으로 고정했으며, shadow dedicated renderer stub auto-prepare를 추가해 `pnpm build:desktop`과 `pnpm release:desktop` 실제 패키징 proof를 완료했다.
