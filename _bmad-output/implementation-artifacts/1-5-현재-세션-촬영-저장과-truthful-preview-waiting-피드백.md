# Story 1.5: 현재 세션 촬영 저장과 truthful `Preview Waiting` 피드백

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
촬영 성공과 프리뷰 준비 상태가 분리되어 안내되길 원한다.
그래서 확인용 프리뷰가 아직 준비되지 않았더라도 내 사진이 안전하게 저장되었음을 신뢰할 수 있다.

## Acceptance Criteria

1. booth가 유효한 촬영 가능 상태이고 활성 프리셋이 선택된 상태에서 고객이 촬영에 성공하면, 새 소스 사진은 성공 안내 전에 활성 세션에 먼저 안전하게 저장되어야 한다. 또한 활성 프리셋은 캡처 또는 확인 surface에서 계속 보여야 한다.
2. 성공적으로 저장된 촬영의 고객 안전 프리뷰가 아직 준비되지 않았다면 booth는 `Preview Waiting`에 진입해야 하며, 첫 문장은 사진 저장 완료를 확인하고 다음 문장은 확인용 프리뷰 준비 중과 지금 가능한 다음 행동을 설명해야 한다.
3. 성공적으로 인정된 촬영의 즉시 결과는 승인된 하드웨어에서 1초 이내에 인지 가능해야 하며, 현재 세션 프리뷰 확인은 95백분위 기준 5초 이내에 보여야 한다. 준비가 더 걸리면 false-ready 상태 대신 truthful `Preview Waiting`을 유지해야 한다.
4. booth가 `Preview Waiting` 상태일 때 최신 사진 레일이 아직 비어 있어도 현재 세션 기준 정상일 수 있음을 설명해야 하며, 고객에게 내부 렌더 실패 원인이나 helper/filesystem 진단어를 노출하면 안 된다.

## Tasks / Subtasks

- [x] 세션 캡처 truth와 프리뷰 상태 계약을 고정한다. (AC: 1, 2, 3, 4)
  - [x] `session.json` 계약에 최소 `sessionId`, `boothAlias`, `activePresetVersion`, `captureId`, `requestId`, `raw`, `preview`, `final`, `renderStatus`, `postEndState` 상관 필드를 포함하도록 정의 또는 보강한다.
  - [x] `src/shared-contracts/`와 `src-tauri/src/contracts/`가 같은 contract family를 공유하도록 유지하고, 프런트엔드/호스트/sidecar에 중복된 별도 정의를 만들지 않는다.
  - [x] 이미지 자체는 IPC payload로 넘기지 않고 파일시스템 handoff 경로와 typed 상태 payload만 계약에 남긴다.
- [x] 호스트 캡처 파이프라인에서 "저장 성공"을 먼저 보장한다. (AC: 1)
  - [x] capture request 성공 시 source photo를 active session root 아래에 먼저 저장하고 manifest를 갱신한 뒤에만 고객 성공 피드백을 허용한다.
  - [x] raw persistence, preview render enqueue, preview ready를 별도 lifecycle 이벤트로 기록해 capture success와 render success를 분리한다.
  - [x] 활성 프리셋 identity/version이 각 capture record에 고정되도록 한다.
- [x] booth-safe `Preview Waiting` 상태와 copy 계층을 구현한다. (AC: 2, 4)
  - [x] `Preview Waiting Panel` 또는 동등 컴포넌트에서 첫 문장은 저장 완료 사실, 둘째 문장은 확인용 사진 준비 중, 보조 문구는 레일 비어 있음이 정상일 수 있음을 설명하도록 만든다.
  - [x] 가능한 다음 행동은 하나만 강조하고, 고객이 무엇을 할 수 있는지 또는 잠시 기다리면 되는지를 plain-language로 보여준다.
  - [x] active preset name/state가 capture surface와 confirmation surface 모두에서 계속 보이도록 유지한다.
  - [x] customer copy에는 darktable, helper, render queue, filesystem, SDK, raw failure 원인을 직접 노출하지 않는다.
- [x] preview readiness 스트리밍과 현재 세션 레일 갱신을 연결한다. (AC: 2, 3, 4)
  - [x] host normalized state/channel에서 `captureSaved`, `previewWaiting`, `previewReady`처럼 의미가 분명한 상태 전이를 전달한다.
  - [x] 최신 사진 레일은 active session에 상관된 preview asset만 사용하고, preview ready 이후에만 새 항목을 노출한다.
  - [x] in-memory cache는 화면 반응성을 위해 사용할 수 있지만 session folder truth보다 우선하면 안 된다.
- [x] 성능 계측과 truthful fallback을 추가한다. (AC: 3)
  - [x] capture acknowledged 시각과 preview visible 시각을 기록할 seam을 두어 1초/5초 예산 검증이 가능하도록 한다.
  - [x] 5초를 넘는 경우 false completion 또는 빈 성공 상태를 보여주지 말고 explicit `Preview Waiting`을 유지한다.
  - [x] retry 가능한 지연과 safe boundary 초과를 host error envelope로 구분하되, 고객에게는 wait/call guidance만 전달한다.
- [x] 테스트로 truthfulness와 세션 격리를 잠근다. (AC: 1, 2, 3, 4)
  - [x] contract test: session manifest, capture result payload, preview status payload의 필수 필드와 schema version을 검증한다.
  - [x] integration test: `capture -> raw persisted -> preview waiting -> preview ready` 흐름이 순서대로 일어나는지 검증한다.
  - [x] privacy/integration test: 다른 세션 asset이 현재 세션 레일이나 확인 상태에 섞이지 않는지 검증한다.
  - [x] UI test: waiting copy의 문장 순서, active preset visibility, rail-empty helper copy, single-primary-action 원칙을 검증한다.

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 Epic 1 안에서 FR-004의 핵심을 구현한다. 핵심은 "촬영 성공"과 "확인용 프리뷰 준비 완료"를 하나의 성공 메시지로 뭉개지 않는 것이다.
- 고객이 실제로 믿어야 하는 첫 번째 truth boundary는 "source photo가 현재 세션에 안전하게 저장되었다"는 사실이다. preview readiness와 final readiness는 그 뒤의 별도 truth다.
- 이 스토리는 현재 세션 레일과 `Preview Waiting` 보호 경험까지 포함하지만, 삭제 정책 완성(Story 2.2), 세션 전역 프리셋 변경 UX 확장(Story 2.3), post-end completion 전체(Story 3.x)를 앞당겨 구현하는 이야기는 아니다.

### 선행 의존성과 구현 순서

- 정석 순서는 Story 1.2 -> 1.3 -> 1.4 -> 1.5다.
- Story 1.5는 다음 선행 truth를 전제로 한다.
  - Story 1.2: active session 생성, `boothAlias`와 `sessionId` 분리, session root와 초기 `session.json`
  - Story 1.3: active preset 선택과 published preset version 바인딩
  - Story 1.4: host-normalized readiness state와 valid-state capture gating
- 현재 워크스페이스에는 Story 1.1 문서만 존재하고 1.2, 1.3, 1.4 문서는 아직 생성되지 않았다. 따라서 dev agent는 이 스토리를 바로 구현하더라도 위 선행 책임을 중복 구현하지 말고, 필요한 최소 seam만 두고 같은 contract surface 위에서 이어서 구현해야 한다.

### 현재 워크스페이스 상태

- 확인 결과 현재 저장소에는 계획 문서와 Story 1.1 문서만 있고, 실제 앱 scaffold는 아직 없다.
- `package.json`, `pnpm-lock.yaml`, `src/`, `src-tauri/`, `tests/`, `Cargo.toml`이 아직 존재하지 않는다.
- git 저장소도 초기화되어 있지 않아 최근 commit intelligence는 사용할 수 없다.
- 따라서 실제 구현을 시작하려면 Story 1.1의 bootstrap과 typed boundary skeleton이 먼저 착지해야 한다.

### 이전 스토리 인텔리전스

- Story 1.1은 `/booth`, `/operator`, `/authoring`, `/settings` 최상위 surface와 `shared-contracts`, adapter/service 경계를 먼저 만드는 방향으로 정리되어 있다.
- Story 1.5는 그 구조를 이어받아야 하며, capture UI와 host orchestration을 위해 별도 임시 디렉터리나 직접 `invoke` 패턴을 새로 만들면 안 된다.
- Story 1.1은 "실제 앱 골격 + 올바른 경계 + 숨겨진 내부 surface"가 목표였고, Story 1.5는 그 위에 FR-004의 truthful capture/preview 흐름을 얹는 첫 번째 고가치 기능 스토리다.

### 구현 가드레일

- 활성 세션의 durable truth는 route state, local component state, SQLite가 아니라 세션 단위 파일시스템 루트와 `session.json`이 소유해야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- Rust host는 camera/helper truth, timing truth, completion truth를 정규화하는 단일 진실 계층이어야 하며 React는 정규화된 상태만 소비해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- capture success와 render success는 분리되어야 한다. source photo persistence 전에는 성공 피드백을 보여주면 안 되고, preview readiness 전에는 preview-ready처럼 말하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Darktable Capability Scope] [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- 이미지 전송은 대용량 JSON IPC가 아니라 파일시스템 handoff로 처리해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- React UI는 직접 Tauri `invoke`를 호출하지 않고 typed adapters/services를 통해서만 host와 통신해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- 고객 copy는 plain-language와 낮은 문구 밀도 원칙을 지켜야 하며 darktable, helper, SDK, filesystem, raw diagnostics를 보여주면 안 된다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Reading Guide] [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]

### 아키텍처 준수사항

- FR-003/FR-004의 구조 매핑은 `src/booth-shell/`, `src/capture-adapter/`, `src-tauri/src/capture/`가 중심이다. [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- 세션 관련 영속화 책임은 `src-tauri/src/session/` 아래에서 유지하고, capture orchestration은 `src-tauri/src/capture/`에서 관리한다. [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure]
- booth copy 변환은 selector/copy 계층이 맡고, host의 raw state를 화면 문자열로 직접 흘려보내지 않는다. 아키텍처 예시는 `booth-shell/selectors/customerStatusCopy.ts`를 명시적으로 제안한다. [Source: _bmad-output/planning-artifacts/architecture.md#Pattern Examples]
- shared contracts는 `src/shared-contracts/`와 `src-tauri/src/contracts/`를 대응시키고, contract tests는 `tests/contract/` 아래에 둔다. [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries] [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- session manifest는 `schemaVersion`을 가진 명시적 계약이어야 하며 capture correlation ID, preset version reference, raw/preview/final 필드를 포함하는 frozen baseline을 따라야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Closed Contract Freeze Baseline]

### 프로젝트 구조 요구사항

- Story 1.1이 구현된 뒤 Story 1.5의 주요 작업 대상은 아래 경로를 기준으로 본다.
  - `src/booth-shell/screens/CaptureScreen.tsx`
  - `src/booth-shell/components/PreviewWaitingPanel.tsx`
  - `src/booth-shell/components/LatestPhotoRail.tsx`
  - `src/booth-shell/selectors/customerStatusCopy.ts`
  - `src/capture-adapter/host/`
  - `src/capture-adapter/services/`
  - `src/session-domain/state/`
  - `src/session-domain/selectors/`
  - `src/shared-contracts/schemas/`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/contracts/`
  - `tests/contract/`
  - `tests/integration/captureToReview.test.ts`
- 아직 scaffold가 없으므로 위 경로는 "새 구조를 발명하라"는 뜻이 아니라, Story 1.1이 만든 디렉터리 문법 안으로 이 책임을 배치하라는 가이드다.

### UX 구현 요구사항

- `Preview Waiting`은 로딩 화면이 아니라 고객 보호 상태다. 첫 문장은 저장 완료 사실, 둘째 문장은 확인용 프리뷰 준비 중, 보조 문구는 현재 사진 레일이 비어 있어도 정상일 수 있음을 설명해야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- `Preview Waiting Panel`은 저장 완료 배지 + 대기 메시지 + 선택적 보조 문구 + 현재 가능한 다음 행동으로 구성되는 재사용 컴포넌트로 설계한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- 고객은 현재 활성 프리셋, 최신 촬영 결과, 현재 세션 범위의 사진만 이해할 수 있어야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Reading Guide]
- 상태 전환 copy는 고객을 안심시키는 방향이어야 하며, 지연이 길어져도 내부 원인 설명 대신 도움 필요 여부만 결과로 전달해야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]

### 정책 및 경계 메모

- `Current-Session Deletion Policy`는 Story 2.2에서 본격 구현되지만, Story 1.5에서도 최신 사진 레일과 manifest 설계를 할 때 현재 세션 asset만 상관되도록 미리 구조를 잡아야 한다. [Source: _bmad-output/planning-artifacts/prd.md#Named Policy References]
- `Session Timing Policy`는 Story 2.4의 주 책임이지만, Story 1.5의 preview delay 처리에서도 false completion을 만들지 말고 truthful waiting을 유지해야 한다. [Source: _bmad-output/planning-artifacts/prd.md#Named Policy References]
- render retry 또는 failure가 있어도 이미 저장된 유효 current-session capture는 훼손되면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]
- cross-session asset leakage는 0이어야 하며, preview/review/deletion/completion 전체 흐름에서 같은 기준을 유지해야 한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]

### 최신 기술 확인 메모

- Tauri 공식 Vite 가이드(2025-06-24 업데이트)는 여전히 `pnpm dev` / `pnpm build`, `devUrl: http://localhost:5173`, `frontendDist: ../dist`, `src-tauri` watch ignore 구성을 권장한다. Story 1.5는 이 부트스트랩 기준선을 깨지 말고 그 위에서 capture/preview 흐름만 얹어야 한다. [Source: https://v2.tauri.app/start/frontend/vite/]
- React Router 공식 설치 문서의 latest 채널은 `7.13.1`이며, Vite 템플릿 이후 `createBrowserRouter` + `RouterProvider` 조합을 계속 안내한다. Story 1.5는 workflow truth를 route truth로 바꾸지 말고 surface routing만 유지한다. [Source: https://reactrouter.com/start/data/installation]
- Zod 공식 릴리즈 노트는 Zod 4가 stable임을 명시한다. Story 1.5의 session manifest, capture result, preview status contract는 Zod 4 기준으로 정의하고 Rust에서 재검증해야 한다. [Source: https://zod.dev/v4]

### 테스트 요구사항

- 최소 필수 테스트 범위는 아래와 같다.
  - capture success 전에 source persistence가 완료되지 않으면 성공 상태를 내보내지 않는다.
  - capture success 후 preview가 아직 없으면 `previewWaiting`이 먼저 보이고, preview asset이 준비되면 그때 rail이 갱신된다.
  - 다른 세션의 asset이 현재 세션 latest photo rail에 노출되지 않는다.
  - active preset visibility가 capture/waiting/confirmation 흐름 내내 유지된다.
  - performance instrumentation이 1초 ack, 5초 p95 preview readiness 측정을 지원한다.
- 아키텍처 문서상 테스트 스택은 강하게 고정되어 있지 않다. 대신 contract, session manifest, host adapter, booth workflow seam 중심으로 검증해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Starter Template Evaluation]

### 금지사항 / 안티패턴

- raw persistence 전에 "촬영 완료" UI를 먼저 보여주는 것 금지
- preview가 아직 없는데 preview-ready처럼 보이는 success 상태를 내보내는 것 금지
- React 컴포넌트에서 직접 `invoke('request_capture')` 같은 raw host 호출을 수행하는 것 금지
- latest photo rail을 전역 이미지 캐시나 다른 세션 인덱스에 의존해 구성하는 것 금지
- session truth를 route 전환, local component state, SQLite row만으로 표현하는 것 금지
- 고객에게 render queue, darktable-cli, helper restart, filesystem path 같은 내부 원인을 보여주는 것 금지
- Story 2.x/3.x 범위를 미리 과도하게 구현해 Story 1.5의 핵심 truth separation을 흐리는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.5: 현재 세션 촬영 저장과 truthful preview waiting 피드백]
- [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]
- [Source: _bmad-output/planning-artifacts/prd.md#Named Policy References]
- [Source: _bmad-output/planning-artifacts/architecture.md#System Overview]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Darktable Capability Scope]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#Closed Contract Freeze Baseline]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Reading Guide]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- [Source: https://v2.tauri.app/start/frontend/vite/]
- [Source: https://reactrouter.com/start/data/installation]
- [Source: https://zod.dev/v4]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Story 문서의 초기 메모와 달리 현재 워크스페이스에는 React/Tauri scaffold와 Story 1.2-1.4 범위 구현 흔적이 이미 존재해, 기존 경계를 확장하는 방식으로 작업했다.
- host `request_capture`는 기존에 저장 없는 `accepted` 응답만 반환하고 있었으므로, raw persistence와 preview lifecycle을 분리하는 ingest pipeline을 추가했다.
- 기존 테스트 fixture 다수가 Story 1.4 계약을 가정하고 있어, Story 1.5 계약으로 엄격화하되 이전 fixture도 normalize되도록 shared contract parsing을 보강했다.

### Completion Notes List

- `session.json` captures 배열을 typed capture record로 고정하고, shared contracts/Rust DTO가 같은 capture contract family를 공유하도록 정리했다.
- host capture pipeline이 raw 파일 저장 후에만 `capture-saved` 응답을 반환하고, preview render enqueue/ready를 별도 상태로 추적하도록 구현했다.
- booth UI에 `Preview Waiting` 보호 패널과 현재 세션 전용 최신 사진 레일을 추가해, 저장 사실과 preview 준비 중 상태를 분리해 안내하도록 만들었다.
- Vitest 72개와 Cargo test 24개를 통과했고, Story 1.5용 contract/integration/UI/privacy 테스트를 추가했다.

### File List

- _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/shared-contracts/dto/capture.ts
- src/shared-contracts/schemas/capture-readiness.ts
- src/shared-contracts/schemas/session-capture.ts
- src/shared-contracts/schemas/session-manifest.ts
- src/shared-contracts/schemas/index.ts
- src/shared-contracts/contracts.test.ts
- src/capture-adapter/services/capture-runtime.ts
- src/capture-adapter/services/capture-runtime.test.ts
- src/session-domain/selectors/current-session-previews.ts
- src/session-domain/selectors/index.ts
- src/session-domain/state/session-provider.tsx
- src/session-domain/state/session-provider.test.tsx
- src/booth-shell/selectors/customerStatusCopy.ts
- src/booth-shell/selectors/customerStatusCopy.test.ts
- src/booth-shell/components/PreviewWaitingPanel.tsx
- src/booth-shell/components/LatestPhotoRail.tsx
- src/booth-shell/screens/CaptureScreen.tsx
- src/booth-shell/screens/CaptureScreen.test.tsx
- src/index.css
- src-tauri/src/session/session_manifest.rs
- src-tauri/src/session/session_repository.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/capture/mod.rs
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/src/capture/normalized_state.rs
- src-tauri/src/commands/capture_commands.rs
- src-tauri/tests/capture_readiness.rs

### Change Log

- 2026-03-21 01:13:25 +09:00 - Story 1.5 구현 완료: typed capture/session contract 확장, raw-first host capture persistence, truthful `Preview Waiting` UI, 현재 세션 전용 preview rail, contract/integration/UI/privacy 테스트 추가
