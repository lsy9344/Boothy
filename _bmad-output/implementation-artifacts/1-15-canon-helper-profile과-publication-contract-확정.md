# Story 1.15: Canon helper profile과 publication contract 확정

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
capture boundary와 preset publication boundary 계약을 먼저 확정하고 싶다,
그래서 실카메라 연동과 future-session publication이 구현마다 다르게 해석되지 않도록 할 수 있다.

## Acceptance Criteria

1. `docs/contracts/camera-helper-edsdk-profile.md`가 Boothy의 Canon helper implementation profile authoritative baseline으로 정리되어야 하며, `helper-ready`, `camera-status`, `recovery-status`, `helper-error` 의미가 `src-tauri/src/capture/sidecar_client.rs`, `src-tauri/src/capture/normalized_state.rs`, `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs`와 일치해야 한다.
2. helper truth는 host-normalized truth와 연결되어야 하며, stale/missing/mismatched helper status는 booth `Ready`를 유지시키면 안 된다. operator의 `카메라 연결 상태`와 booth capture readiness는 같은 helper semantics에서 계산되어야 하고, raw helper vocabulary가 그대로 customer copy로 노출되면 안 된다.
3. `docs/contracts/authoring-publication.md`와 publication payload contract가 필수 publish input, rejection reason, audit record, immutable bundle rule, approval state transition(`validated -> approved -> published`)을 포함해 동결되어야 하며, `src/shared-contracts/schemas/preset-authoring.ts`, `src/shared-contracts/dto/preset.ts`, `src-tauri/src/contracts/dto.rs`, `src-tauri/src/preset/authoring_pipeline.rs`와 같은 의미를 가져야 한다.
4. future-session-only publication / rollback rule이 계약과 테스트에서 명시적으로 보장되어야 하며, publish/rollback이 active session manifest, current capture binding, existing published bundle을 직접 바꾸지 않고 live catalog pointer + audit history만 변경한다는 점이 검증 가능해야 한다.

## Tasks / Subtasks

- [x] Canon helper implementation profile을 문서/코드/예시에서 같은 기준으로 잠근다. (AC: 1, 2)
  - [x] `docs/contracts/camera-helper-edsdk-profile.md`, `docs/contracts/camera-helper-sidecar-protocol.md`, `sidecar/canon-helper/README.md`를 기준으로 Windows 10/11 x64, `canon-helper.exe`, `net8.0`, 단일 활성 카메라, 단일 in-flight capture, request/session/capture correlation 규칙을 명시한다.
  - [x] `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs`, `sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs`, `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`와 `src-tauri/src/capture/sidecar_client.rs`의 schema version, message type, 필수 필드를 교차 정렬한다.
  - [x] 현재 `sidecar/protocol/examples/`에 없는 `helper-ready`, `recovery-status`, `helper-error` durable example을 추가하거나 동등한 fixture/test 경로로 승격한다.

- [x] host-normalized booth/operator truth 연결을 명확히 잠근다. (AC: 1, 2)
  - [x] `src-tauri/src/capture/normalized_state.rs`에서 freshness, session match, helper/camera degraded semantics가 `Ready`와 `Phone Required`를 어떻게 가르는지 문서 기준과 맞춘다.
  - [x] stale helper, session mismatch, degraded-after-ready, reconnect-pending이 false-ready로 이어지지 않도록 guardrail을 남긴다.
  - [x] Story 5.4의 operator `카메라 연결 상태` 기대치와 연결되도록 raw detailCode -> normalized operator state 매핑을 문서화한다.

- [x] publication payload contract와 approval state transition을 authoritative contract로 동결한다. (AC: 3)
  - [x] `docs/contracts/authoring-publication.md`와 `docs/contracts/authoring-publication-payload.md`의 역할을 재정렬한다. 현재 payload 문서가 validation 중심이면 publication payload contract 문서로 고치거나 validation contract를 별도 문서로 분리해 의미 충돌을 없앤다.
  - [x] `src/shared-contracts/schemas/preset-authoring.ts`, `src/shared-contracts/dto/preset.ts`, `src-tauri/src/contracts/dto.rs`, `src-tauri/src/preset/authoring_pipeline.rs`의 publish input, rejection reason, audit shape, publication history 규칙을 같은 baseline으로 맞춘다.
  - [x] `duplicate-version`, `stale-validation`, `metadata-mismatch`, `path-escape`, `future-session-only-violation` 거절 규칙과 `approved -> published` history 기록 순서를 계약 수준으로 잠근다.

- [x] future-session-only publication / rollback semantics를 active session 보호 기준과 함께 잠근다. (AC: 4)
  - [x] `src-tauri/src/preset/preset_catalog_state.rs`, `src-tauri/tests/preset_authoring.rs`, `src-tauri/tests/operator_audit.rs`, `docs/contracts/preset-bundle.md`, `docs/runbooks/booth-hardware-validation-checklist.md`를 기준으로 live catalog pointer 변경과 active session binding 보존 규칙을 정렬한다.
  - [x] publish/rollback이 immutable bundle directory를 수정하지 않고, future session catalog와 audit store만 바꾼다는 점을 테스트 가능한 기준으로 남긴다.
  - [x] operator audit / publication recovery 로그가 publication rejection, publication success, catalog rollback을 남기되 active session truth를 건드리지 않는다는 점을 유지한다.

- [x] 실행 가능한 계약 검증과 회귀 테스트를 보강한다. (AC: 1, 2, 3, 4)
- [x] `src/shared-contracts/contracts.test.ts`에 publication input/result, rollback result, helper message fixture 파싱 검증을 추가하거나 보강한다.
- [x] `src-tauri/tests/preset_authoring.rs`, `src-tauri/tests/operator_audit.rs`에서 future-session-only, active-session immutability, rejection audit, catalog rollback regression을 잠근다.
- [x] `sidecar/canon-helper/tests/CanonHelper.Tests/JsonFileProtocolTests.cs` 또는 동등한 helper test에서 processed request persistence, incomplete JSON-line buffering, new helper event fixture 파싱을 유지한다.

### Review Findings

- [x] [Review][Patch] Helper sidecar protocol 문서가 실제 파일 기반 진단 경계 대신 stdio transport를 동결하고 있음 [docs/contracts/camera-helper-sidecar-protocol.md:51]
- [x] [Review][Patch] Publication rejection reason 계약에서 `stage-unavailable`가 빠져 schema/runtime와 문서가 다시 어긋남 [docs/contracts/authoring-publication.md:44]
- [x] [Review][Patch] Publication payload 문서가 rollback 입력/결과 계약을 동결하지 않아 future-session-only rollback wire contract가 문서 기준선에서 비어 있음 [docs/contracts/authoring-publication-payload.md:65]
- [x] [Review][Patch] 새 helper fixture 검증이 host/helper 모두에서 `schemaVersion`과 필수 필드를 실질적으로 고정하지 못함 [src-tauri/src/capture/sidecar_client.rs:596]
- [x] [Review][Patch] Published bundle strictness가 TypeScript 테스트에만 있고 Rust runtime loader는 동일 강도로 막지 않음 [src-tauri/src/preset/preset_bundle.rs:33]

## Dev Notes

### 스토리 범위와 제품 목적

- 이 스토리는 새 booth 기능을 추가하는 작업이 아니라, capture boundary와 publication boundary를 제품 관점에서 더 깊게 잠그는 foundational contract story다.
- Story 1.14가 session/preset/error/capability/protocol의 공용 baseline을 닫았다면, Story 1.15는 그 위에 Canon helper runtime profile과 publication state machine detail을 닫는 역할이다.
- Story 1.16의 build/release baseline까지 끌고 가지 말 것. helper packaging/security가 release pipeline 전체 변경으로 번지면, release proof 자체는 1.16의 책임으로 남겨 둔다.

### 왜 지금 필요한가

- `epics.md`와 2026-04-10 correct-course는 implementation sequence 상 shared contract freeze 다음에 helper/publication contract를 닫아야 한다고 명시했다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.15: Canon helper profile과 publication contract 확정] [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260410-003446.md]
- implementation readiness review도 foundation-first 순서를 `1.14 -> 1.15 -> 1.16 -> starter setup`으로 본다. [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260410.md]
- 현재 저장소는 helper/profile 문서와 publication flow 구현이 이미 존재하지만, contract ownership과 example coverage가 완전히 닫혀 있지는 않다.

### 현재 워크스페이스 상태

- Canon helper 축은 이미 code/doc baseline이 있다.
  - `docs/contracts/camera-helper-sidecar-protocol.md`
  - `docs/contracts/camera-helper-edsdk-profile.md`
  - `sidecar/canon-helper/README.md`
  - `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`
  - `src-tauri/src/capture/sidecar_client.rs`
  - `src-tauri/src/capture/normalized_state.rs`
- publication 축도 이미 작동하는 흐름이 있다.
  - `docs/contracts/authoring-publication.md`
  - `src/shared-contracts/schemas/preset-authoring.ts`
  - `src/preset-authoring/services/preset-authoring-service.ts`
  - `src-tauri/src/preset/authoring_pipeline.rs`
  - `src-tauri/src/preset/preset_catalog_state.rs`
  - `src-tauri/tests/preset_authoring.rs`
  - `src-tauri/tests/operator_audit.rs`
- 다만 `docs/contracts/authoring-publication-payload.md`는 현재 validation artifact 설명에 가깝고 publication payload contract와 이름/역할이 맞지 않는다. 1.15는 이 drift를 그대로 두면 안 된다.
- `sidecar/protocol/examples/`에는 현재 `camera-status.json`, `file-arrived.json`만 있어 helper-ready / recovery-status / helper-error baseline example이 비어 있다.
- `src-tauri/tauri.conf.json`은 아직 helper sidecar packaging/security 계약을 드러내지 않는다. 이 부분을 건드릴 경우 contract-driven 최소 범위만 다루고, release proof 자체는 1.16과 충돌하지 않게 유지한다.

### 이전 스토리 인텔리전스

- Story 1.14는 shared contract baseline을 잠그면서 helper deeper runtime profile과 publication deeper behavior는 1.15가 닫도록 범위를 남겨 두었다. [Source: _bmad-output/implementation-artifacts/1-14-공유-계약-동결과-검증-기준-확정.md]
- recent preview/capture work는 helper correlation, fast preview, operator guidance를 강화했다. 따라서 1.15는 새 vocabulary를 발명하기보다 이미 쓰이는 requestId / captureId / freshness / future-session-only semantics를 authoritative baseline으로 정리하는 쪽이 맞다.
  - `8c30be7 Improve focus retry guidance`
  - `2c89c40 Finalize thumbnail latency worker updates and docs`
  - `9c56c37 Add session seam logging for thumbnail latency reduction`
  - `b24cfc4 Reduce recent-session preview latency and capture wait blocking`
  - `12309fa Record thumbnail validation and ship fast preview fallback`

### 제품/아키텍처 가드레일

- camera/helper truth는 계속 host-normalized projection 뒤에 있어야 한다. React가 raw helper state, stdout/stderr, detailCode를 직접 해석해 booth/operator truth를 만들면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions] [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- `helper-ready`와 `camera-status=ready`는 다른 뜻이다. helper process가 protocol 대화를 시작할 수 있다는 사실과 실제 촬영 가능 카메라 truth를 합치면 안 된다. [Source: docs/contracts/camera-helper-edsdk-profile.md]
- capture success는 `capture-accepted`가 아니라 `file-arrived` + host file existence recheck 뒤에만 확정돼야 한다. [Source: docs/contracts/camera-helper-edsdk-profile.md] [Source: src-tauri/src/capture/sidecar_client.rs]
- publication success는 immutable bundle, publication history, live catalog pointer, audit가 함께 닫혀야 하고, active session binding을 직접 바꾸면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#FR-008 Authorized Preset Authoring, Approval, and Publication] [Source: docs/contracts/authoring-publication.md] [Source: src-tauri/src/preset/preset_catalog_state.rs]
- publish/rollback은 future sessions only다. rollback은 existing bundle 삭제가 아니라 live pointer 전환이어야 한다. [Source: docs/contracts/preset-bundle.md] [Source: docs/runbooks/booth-hardware-validation-checklist.md]

### 구현 가드레일

- session manifest, preset bundle, error envelope, runtime capability baseline은 1.14에서 이미 잠갔다. 1.15에서 그 범위를 다시 넓히거나 schema version을 가볍게 올리지 말 것.
- Canon helper profile을 정리한다는 이유로 multi-camera, live view UI, network camera control까지 scope를 넓히지 말 것. 현재 baseline은 Windows-only Canon EDSDK single-camera helper다.
- publication contract를 닫는 과정에서 Story 4.2/4.3/4.4의 제품 행동을 선행 구현 수준까지 다 끝내려 하지 말 것. 이 스토리의 목적은 state machine contract freeze와 guardrail 강화다.
- customer copy를 helper vocabulary나 authoring 용어로 오염시키지 말 것.

### 프로젝트 구조 요구사항

- 우선 수정 후보 경로:
  - `docs/contracts/camera-helper-edsdk-profile.md`
  - `docs/contracts/camera-helper-sidecar-protocol.md`
  - `docs/contracts/authoring-publication.md`
  - `docs/contracts/authoring-publication-payload.md`
  - `sidecar/canon-helper/README.md`
  - `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs`
  - `sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs`
  - `sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`
  - `sidecar/protocol/examples/`
  - `src/shared-contracts/schemas/preset-authoring.ts`
  - `src/shared-contracts/dto/preset.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src/preset-authoring/services/preset-authoring-service.ts`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/capture/sidecar_client.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/preset/authoring_pipeline.rs`
  - `src-tauri/src/preset/preset_catalog_state.rs`
  - `src-tauri/tests/preset_authoring.rs`
  - `src-tauri/tests/operator_audit.rs`
- 필요 시 새로 추가될 가능성이 큰 경로:
  - `sidecar/protocol/examples/helper-ready.json`
  - `sidecar/protocol/examples/recovery-status.json`
  - `sidecar/protocol/examples/helper-error.json`
  - publication contract용 fixture 또는 test data 경로

### 테스트 요구사항

- helper message contract:
  - `helper-ready`, `camera-status`, `capture-accepted`, `file-arrived`, `recovery-status`, `helper-error`가 host와 helper에서 모두 같은 schemaVersion/type/field 의미를 가져야 한다.
  - stale status, mismatched session, recovery-required가 booth `Ready`를 만들지 않는 regression guard가 있어야 한다.
- publication contract:
  - validated draft만 publish 가능해야 한다.
  - `approved -> published` history가 success path에서 순서대로 남아야 한다.
  - `duplicate-version`, `stale-validation`, `metadata-mismatch`, `path-escape`, `future-session-only-violation`이 typed rejection으로 남아야 한다.
  - rollback은 live catalog pointer만 바꾸고 active session binding은 유지해야 한다.
- 권장 실행 게이트:
  - `pnpm test:run`
  - `cargo test --test preset_authoring`
  - `cargo test --test operator_audit`
  - `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`

### 최신 기술 / 제품 컨텍스트

- 저장소 기준선은 현재 `React 19.2.4`, `react-router-dom 7.13.1`, `Zod 4.3.6`, `@tauri-apps/api/cli 2.10.1`, Rust `tauri 2.10.3`, helper `net8.0`, pinned darktable `5.4.1`이다. 새 계층을 추가하기보다 이 baseline을 authoritative contract로 정렬하는 편이 맞다. 이 문장은 현재 저장소 파일을 근거로 한 추론이다. [Source: package.json] [Source: src-tauri/Cargo.toml] [Source: sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj] [Source: src-tauri/src/preset/authoring_pipeline.rs]
- 2026-04-10 확인 기준 Canon 공식 CAP release note는 EDSDK `Ver.13.20.10`의 공개 일자를 `2025-09-24`로 안내한다. helper profile 문서는 이 기준과 모순되지 않게 유지한다. [Source: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note]
- 2026-04-10 확인 기준 Tauri 공식 sidecar 문서는 sidecar binary entry와 spawn/execute permission을 명시적으로 설정해야 한다고 안내한다. helper launch contract를 바꾸면 이 규칙을 따라야 한다. [Source: https://tauri.app/develop/sidecar/]

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.15: Canon helper profile과 publication contract 확정]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-008 Authorized Preset Authoring, Approval, and Publication]
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-20260410.md]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260410-003446.md]
- [Source: _bmad-output/implementation-artifacts/1-14-공유-계약-동결과-검증-기준-확정.md]
- [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- [Source: docs/contracts/camera-helper-edsdk-profile.md]
- [Source: docs/contracts/authoring-publication.md]
- [Source: docs/contracts/authoring-publication-payload.md]
- [Source: docs/contracts/preset-bundle.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: sidecar/canon-helper/README.md]
- [Source: sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj]
- [Source: sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs]
- [Source: sidecar/canon-helper/tests/CanonHelper.Tests/JsonFileProtocolTests.cs]
- [Source: src/shared-contracts/schemas/preset-authoring.ts]
- [Source: src/preset-authoring/services/preset-authoring-service.ts]
- [Source: src-tauri/src/contracts/dto.rs]
- [Source: src-tauri/src/capture/sidecar_client.rs]
- [Source: src-tauri/src/capture/normalized_state.rs]
- [Source: src-tauri/src/preset/authoring_pipeline.rs]
- [Source: src-tauri/src/preset/preset_catalog_state.rs]
- [Source: src-tauri/tests/preset_authoring.rs]
- [Source: src-tauri/tests/operator_audit.rs]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]
- [Source: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note]
- [Source: https://tauri.app/develop/sidecar/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Story Creation Notes

- config, sprint-status, epics, PRD, architecture, UX, implementation-readiness report, sprint change proposal, Story 1.14, helper/publication contract docs, helper C# source, Rust host publication/capture code, contract tests를 교차 분석했다.
- 현재 저장소는 helper profile과 publication flow의 baseline 구현을 이미 갖고 있으며, 이번 스토리는 greenfield 기능이 아니라 contract ownership 정렬, example 보강, active-session immutability 증명에 초점을 둔다고 판단했다.
- 최신 외부 확인은 Canon 공식 EDSDK release note와 Tauri 공식 sidecar 문서만 사용했다.

### Completion Notes List

- Canon helper 계약 문서에 Windows 10/11 x64, `canon-helper.exe`, `net8.0`, 단일 활성 카메라, 단일 in-flight capture, correlation 규칙과 durable example baseline을 반영했다.
- `sidecar/protocol/examples/`에 `helper-ready`, `recovery-status`, `helper-error` fixture를 추가하고 `src/shared-contracts/contracts.test.ts`에서 canonical parse 검증을 보강했다.
- publication 문서 역할을 `authoring-publication` / `authoring-publication-payload` / `authoring-validation`으로 분리해 publish input/result/audit와 validation baseline의 의미 충돌을 제거했다.
- `cargo test --test preset_authoring`, `cargo test --test operator_audit`, `cargo test --test capture_readiness`, `pnpm vitest run src/shared-contracts/contracts.test.ts`를 통과했다.
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj`는 Canon SDK vendor 입력물이 없어 실행하지 못했지만, fixture parse 회귀용 테스트는 추가했다.

### Change Log

- 2026-04-10: Canon helper protocol durable example과 publication payload/validation 문서 역할을 정렬하고 Story 1.15를 `review`로 갱신.

### File List

- _bmad-output/implementation-artifacts/1-15-canon-helper-profile과-publication-contract-확정.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/authoring-publication.md
- docs/contracts/authoring-publication-payload.md
- docs/contracts/authoring-validation.md
- docs/contracts/camera-helper-edsdk-profile.md
- docs/contracts/camera-helper-sidecar-protocol.md
- sidecar/canon-helper/README.md
- sidecar/canon-helper/tests/CanonHelper.Tests/JsonFileProtocolTests.cs
- sidecar/protocol/examples/helper-error.json
- sidecar/protocol/examples/helper-ready.json
- sidecar/protocol/examples/recovery-status.json
- src/shared-contracts/contracts.test.ts
