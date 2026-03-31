# Story 3.2: Export Waiting과 truthful completion 안내

Status: review

Correct Course Note: false-complete 방지 evidence(HV-08, HV-11)가 닫히기 전까지 제품 관점 완료로 보지 않는다. Story 6.2 canonical ledger 기준으로 end-of-session hardware evidence가 아직 없으므로 Story 3.2는 `review`를 유지한다.

### Hardware Gate Reference

- Canonical ledger: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- Required HV checklist IDs: `HV-08`, `HV-11`
- Current hardware gate: `No-Go`
- Close policy: `automated pass` alone does not close this story; a ledger row with `Go` is required before `done`.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
내 최종 결과물이 아직 준비 중인지 이미 끝났는지 알고 싶다.
그래서 너무 일찍 떠나거나 아직 처리 중인 세션을 실패로 오해하지 않을 수 있다.

## Acceptance Criteria

1. 촬영이 종료되었지만 부스 측 필수 결과물이 아직 준비되지 않았을 때 사후 상태를 평가하면, 부스는 `Export Waiting` 안내를 보여줘야 한다. 또한 이 안내가 보이는 동안 촬영은 계속 비활성화되어야 한다.
2. 부스 측 필수 작업이 실제로 모두 끝났을 때 `Completed`에 진입하면, 결과는 `Local Deliverable Ready` 또는 `Handoff Ready` 중 하나로 해석되어야 한다. 또한 필요한 부스 측 작업이 끝나기 전에는 UI가 완료를 주장하면 안 된다.
3. 조정된 종료 시각에 도달한 뒤 호스트가 사후 상태 평가를 마무리하면, 90% 이상의 세션은 예정 종료 시각 기준 10초 이내에 명시적 사후 상태로 진입해야 한다. 또한 render retry 또는 failure가 이미 저장된 현재 세션 캡처를 무효화하면 안 된다.

## Tasks / Subtasks

- [x] 호스트 소유의 사후 완료 진실 계약을 정리한다. (AC: 1, 2, 3)
  - [x] `src/shared-contracts/`와 `src-tauri/src/contracts/`에서 `export-waiting`, `completed`, completion variant(`localDeliverableReady`, `handoffReady`)를 표현할 canonical contract를 정리하고, `session-manifest`의 `postEnd`가 더 이상 항상 `null`만 허용되지 않도록 확장한다.
  - [x] `CaptureReadinessSnapshot`와 session manifest/capture record 사이의 역할을 분리해, 고객 화면 copy는 readiness가 담당하고 durable post-end truth는 manifest/post-end record가 담당하도록 경계를 고정한다.
  - [x] Story 3.3 범위인 상세 handoff recipient/contact UX를 미리 과구현하지 않되, 이번 스토리에서 variant truth를 수용할 수 있는 필드는 선반영한다.
- [x] Rust host에 post-end evaluation과 truthful completion 판정을 구현한다. (AC: 1, 2, 3)
  - [x] `src-tauri/src/handoff/` 또는 동등한 host module을 추가해 종료 직후 상태를 `Export Waiting`, `Completed`, `Phone Required` 중 정확히 하나로 정규화하는 평가 경로를 만든다.
  - [x] `Completed`는 booth-side required work가 실제로 끝난 뒤에만 내보내고, local deliverable readiness와 handoff readiness를 host에서 판정한다.
  - [x] render retry/failure가 이미 저장된 현재 세션 원본/미리보기 자산을 지우거나 invalid 처리하지 않도록 기존 session-scoped asset 보존 규칙을 유지한다.
  - [x] 사후 상태 판정 시각과 결과를 lifecycle/audit/timing log 경계에 남겨 NFR-005의 10초 진입 측정을 가능하게 한다.
- [x] typed adapter, provider, selector를 post-end truth 중심으로 연결한다. (AC: 1, 2)
  - [x] `src/capture-adapter/`와 `src/session-domain/`에서 same-session guard를 유지한 채 `export-waiting`/`completed` readiness와 post-end variant를 소비하도록 갱신한다.
  - [x] `SessionProvider`는 이미 들어온 timing phase(`ended`)와 새 post-end truth를 충돌 없이 병합하고, `ended` 상태에 오래 머무르지 않도록 host-normalized 결과를 우선 반영한다.
  - [x] `customerStatusCopy` 또는 동등 selector에서 `Preview Waiting`과 구분되는 post-end용 고객 문구를 제공하되, completion claim은 host truth가 확인된 뒤에만 노출한다.
- [x] booth UI에 Export Waiting/Completed 안내를 추가한다. (AC: 1, 2)
  - [x] `CaptureScreen`, `ReadinessScreen`, 또는 새 post-end panel component에서 `Export Waiting`과 `Completed` 상태를 별도로 표현한다.
  - [x] `Export Waiting`은 "촬영은 끝났고 결과를 준비 중"이라는 사실과 현재 해야 할 행동 하나만 보여주고, `Completed`는 "부스 측 준비가 끝났다"는 사실만 truthful하게 안내한다.
  - [x] `Handoff Ready`의 상세 수령자/이동 안내와 `Phone Required` 보호 카드 완성은 Story 3.3 범위로 남겨 두고, 이번 스토리에서는 이를 방해하지 않는 데이터/레이아웃 경계만 마련한다.
- [x] 테스트로 false completion과 post-end 회귀를 잠근다. (AC: 1, 2, 3)
  - [x] contract/unit test: post-end schema 확장, completion variant parsing, `postEnd` durable truth, false-complete rejection을 검증한다.
  - [x] UI test: `Export Waiting` 문구, 촬영 비활성화 유지, `Completed` 진입 후 truthful copy, 고객 화면 내 low-density copy budget 준수를 검증한다.
  - [x] provider/integration test: same-session post-end update만 반영, `ended`에서 `export-waiting`/`completed`로 전환, stale/foreign readiness 무시를 검증한다.
  - [x] Rust test: 종료 후 10초 이내 명시적 post-end state 전환, render retry/failure 중 자산 보존, `Completed`의 premature claim 차단을 검증한다.

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 Epic 3 / FR-007의 첫 번째 고객 가치 조각으로, 종료 후 결과 준비 상태와 실제 완료 상태를 거짓 없이 구분해 주는 것이 핵심이다.
- 핵심은 "완료처럼 보이는 문구"를 빨리 띄우는 것이 아니라, host가 booth-side required work의 실제 완료 여부를 판정한 뒤에만 `Completed`를 노출하는 것이다.
- Story 3.3이 `Handoff Ready`와 `Phone Required`의 보호 안내를 완성하는 단계라면, Story 3.2는 그 이전에 `Export Waiting`과 truthful completion taxonomy를 product truth로 고정하는 단계다.

### 스토리 기반 요구사항

- PRD의 `Post-End Completion Taxonomy` 기준선:
  - `Export Waiting`: 촬영은 끝났지만 booth-side required work는 아직 끝나지 않음
  - `Completed`: booth-side required work가 완료되었고 추가 booth-side processing이 필요하지 않음
  - `Completed / Local Deliverable Ready`: 로컬 deliverable이 준비됨
  - `Completed / Handoff Ready`: 부스 작업은 끝났고 승인된 인계 다음 행동으로 넘어갈 수 있음
- 고객은 종료 직후 "끝났는지", "아직 기다려야 하는지", "지금 무엇을 해야 하는지"를 technical diagnostics 없이 이해해야 한다.
- 이미 저장된 현재 세션 캡처는 render retry/failure가 있더라도 보존되어야 하며, false completion보다 bounded waiting 또는 escalation이 우선이다.

### 선행 의존성과 구현 순서

- 문서상 선행 스토리는 Story 3.1(종료 직후 명시적 사후 상태 진입)이다.
- 다만 현재 워크스페이스에는 Story 3.1 전용 문서 파일은 없지만, Story 2.4 계열 timing foundation이 이미 일부 들어와 있다:
  - `src-tauri/src/timing/mod.rs`가 `warning`/`ended` phase와 lifecycle stage 반영을 수행한다.
  - `src/shared-contracts/schemas/session-timing.ts`와 `src/shared-contracts/dto/timing.ts`가 host-owned timing snapshot을 정의한다.
  - `src-tauri/src/contracts/dto.rs`와 `src-tauri/src/capture/normalized_state.rs`는 `export-waiting`, `completed`, `phone-required`, `warning`, `ended` readiness DTO를 이미 수용한다.
- 반면 아직 빠진 부분도 명확하다:
  - `derive_capture_lifecycle_stage()`는 현재 `preview-waiting`, `renderFailed`, `capture-ready` 중심이며 종료 후 explicit post-end evaluator가 없다.
  - `src/shared-contracts/schemas/session-manifest.ts`의 `postEnd`는 아직 `null`만 허용한다.
  - `src/booth-shell/selectors/customerStatusCopy.ts`는 `Preview Waiting`만 별도 최적화하고 `Export Waiting`/`Completed` 전용 고객 문구는 없다.
  - 아키텍처가 기대하는 `src/completion-handoff/`, `src-tauri/src/handoff/` 경계는 아직 존재하지 않는다.

### 현재 워크스페이스 상태

- `SessionProvider`는 이미 `warning`/`ended` timing과 `export-waiting`/`completed` stage를 덮어쓰지 않도록 일부 lifecycle merge 규칙을 가지고 있다.
- `capture-runtime`은 Tauri event subscription + polling cleanup 경계를 이미 사용하고 있어, post-end readiness도 같은 typed adapter 경계를 따라가는 편이 안전하다.
- `LatestPhotoRail`은 `postEndState === 'completed'` 사진 삭제 보호 문구를 이미 가지고 있어, post-end truth가 capture-level UX에 일부 스며들기 시작한 상태다.
- Rust host DTO는 `Export Waiting`, `Completed`, `Phone Required`, `Warning`, `Session Ended`용 customer-safe copy primitive를 이미 일부 제공하지만, completion variant와 durable `postEnd` truth는 아직 비어 있다.

### 구현 가드레일

- `Completed`는 host가 booth-side required work 완료를 확인하기 전에는 절대 노출하면 안 된다. latest capture 존재, preview ready, route entry만으로 완료를 추론하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#Post-End Completion Taxonomy]
- 종료 직후 `ended` 상태는 중간 판정 상태일 수 있지만, 고객을 모호한 상태에 오래 두면 안 된다. explicit post-end state로 빨리 수렴시켜야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 3.1: 종료 직후 명시적 사후 상태 진입]
- render retry/failure는 이미 저장된 current-session capture를 지우거나 다른 세션 상태와 섞는 방식으로 복구하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]
- React는 host-normalized truth를 소비해야 하며, JSX 안에서 "마지막 캡처가 있으니 완료" 같은 ad-hoc 판정을 만들면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- `Export Waiting`과 `Completed`의 고객 문구는 plain-language, low-density 원칙을 지켜야 하며 internal render/export pipeline 용어를 직접 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]

### 아키텍처 준수사항

- FR-007의 목표 구조는 `src/completion-handoff/`와 `src-tauri/src/handoff/`이다. 이번 스토리에서 최소 구현이 기존 `capture`/`session` 경계를 일부 활용하더라도 최종 책임은 이 구조로 수렴해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- frontend-to-host post-end 흐름은 typed adapter/service를 통해 들어와야 하며, component에서 raw Tauri 호출이나 임시 state machine을 만들면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- customer-facing completion copy와 operator-facing post-end truth는 같은 normalized host truth에서 파생돼야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#State Management Patterns]
- session folder는 이미지/세션 truth를, SQLite는 timing transition과 lifecycle audit를 소유한다. post-end truth도 이 경계 안에 있어야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]

### 프로젝트 구조 요구사항

- 주요 수정 후보 경로:
  - `src/booth-shell/screens/CaptureScreen.tsx`
  - `src/booth-shell/screens/CaptureScreen.test.tsx`
  - `src/booth-shell/screens/ReadinessScreen.tsx`
  - `src/booth-shell/selectors/customerStatusCopy.ts`
  - `src/booth-shell/selectors/customerStatusCopy.test.ts`
  - `src/session-domain/state/session-provider.tsx`
  - `src/session-domain/state/session-provider.test.tsx`
  - `src/capture-adapter/services/capture-runtime.ts`
  - `src/capture-adapter/services/capture-runtime.test.ts`
  - `src/shared-contracts/schemas/capture-readiness.ts`
  - `src/shared-contracts/schemas/session-capture.ts`
  - `src/shared-contracts/schemas/session-manifest.ts`
  - `src/shared-contracts/dto/session.ts`
  - `src/shared-contracts/dto/timing.ts`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src-tauri/tests/session_manifest.rs`
- 아키텍처 정렬을 위해 새로 추가될 가능성이 큰 경로:
  - `src/completion-handoff/`
  - `src-tauri/src/handoff/`

### UX 구현 요구사항

- `Export Waiting`은 `Preview Waiting`과 비슷한 보호 상태지만 의미가 다르다. 첫 문장에서 "촬영은 끝났고", 둘째 문장에서 "결과를 준비 중"이라는 사실을 알려야 하며, 촬영 가능 여부는 분명히 꺼져 있어야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey-Patterns]
- `Completed`는 booth-side work 완료를 사실대로 알려야 하지만, Story 3.3 이전에는 상세 handoff recipient/contact를 임의로 꾸며내면 안 된다.
- 고객 화면은 동적 값 외에 핵심 지시 1개, 보조 문장 1개, 주요 액션 1개 원칙을 넘기지 않는 편이 안전하다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]
- `Export Waiting`과 `Completed`도 기존 부스 헤더의 booth alias / timing visibility와 충돌하지 않아야 하며, 종료 후에도 현재 세션 맥락은 끊기지 않아야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns]

### 최신 기술 확인 메모

- 현재 로컬 기준선은 `react 19.2.4`, `react-router-dom 7.13.1`, `@tauri-apps/api 2.10.1`, `zod 4.3.6`이다. 이번 스토리는 새 상태관리 라이브러리보다 기존 reducer/context + typed adapter 구조를 확장하는 편이 안전하다.
- React 공식 문서는 `useEffectEvent`가 effect 안에서 최신 state를 읽되 불필요한 재구독을 피하는 패턴에 적합하다고 설명한다. post-end polling/timeout bridge나 one-shot completion cue가 필요할 때 유용하다. [Source: https://react.dev/reference/react/useEffectEvent]
- Tauri 공식 문서는 frontend event listener 등록 후 `unlisten` cleanup을 수행해야 한다고 안내한다. post-end readiness stream도 session 교체나 unmount 때 중복 구독을 남기면 안 된다. [Source: https://v2.tauri.app/develop/_sections/frontend-listen/]
- Tauri 공식 문서는 ordered state changes에는 channels가 적합하다고 설명한다. 종료 직후 짧은 시간 안에 순차 상태를 강하게 보장해야 한다면 event 남발보다 channel 또는 host-finalized snapshot 전략을 우선 검토하는 편이 안전하다. 이것은 문서와 현재 아키텍처를 바탕으로 한 구현 추론이다. [Source: https://v2.tauri.app/develop/calling-frontend/]
- Tauri 공식 문서는 `mockIPC(..., { shouldMockEvents: true })`로 Rust emit 이벤트를 테스트에서 시뮬레이션할 수 있다고 안내한다. `Export Waiting` -> `Completed` 전환 회귀 테스트에 그대로 활용 가능하다. [Source: https://v2.tauri.app/develop/tests/mocking/]
- Zod 공식 문서는 Zod 4가 stable이며 TypeScript-first schema validation을 제공한다고 명시한다. post-end schema 확장도 ad-hoc parsing보다 shared-contracts 쪽 schema 확장으로 처리하는 편이 안전하다. [Source: https://zod.dev/]

### 테스트 요구사항

- 최소 필수 테스트 범위:
  - 종료 직후 `ended`에서 `Export Waiting` 또는 `Completed`로 explicit transition이 생긴다.
  - `Export Waiting` 동안 촬영은 비활성화되고, 고객 문구는 실패처럼 들리지 않는다.
  - `Completed`는 booth-side required work 완료 뒤에만 나온다.
  - `Completed` 결과는 `Local Deliverable Ready` 또는 `Handoff Ready` variant로 해석 가능하다.
  - render retry/failure가 기존 current-session capture asset을 삭제하거나 invalid 처리하지 않는다.
  - foreign/stale post-end update가 현재 세션 UI를 덮지 못한다.
  - 고객 화면에 render queue, export worker, filesystem, darktable 같은 내부 용어가 나타나지 않는다.

### 금지사항 / 안티패턴

- latest preview 존재만으로 `Completed`를 추론하는 것 금지
- `ended` 화면을 프런트 임시 카피로만 오래 유지하고 host post-end evaluator를 생략하는 것 금지
- Story 3.3 범위인 handoff recipient/contact UX를 이번 스토리에서 임의 정책으로 만들어 넣는 것 금지
- render retry/failure 중 기존 current-session asset을 cleanup 대상으로 잘못 포함하는 것 금지
- export/completion 상태를 다른 세션 event나 stale readiness payload로 덮는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- 계약 문서: `docs/contracts/session-manifest.md`
- 기존 스토리: `_bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.2: Export Waiting과 truthful completion 안내]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.1: 종료 직후 명시적 사후 상태 진입]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: 종료 후 결과 준비와 인계의 진실한 마무리]
- [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-007 Export Waiting, Final Readiness, and Handoff Guidance]
- [Source: _bmad-output/planning-artifacts/prd.md#Post-End Completion Taxonomy]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]
- [Source: _bmad-output/planning-artifacts/prd.md#Timing, Completion, and Handoff]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Management Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#구속되는-ux-요구사항]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey-Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns]
- [Source: docs/contracts/session-manifest.md#필드-규칙]
- [Source: https://react.dev/reference/react/useEffectEvent]
- [Source: https://v2.tauri.app/develop/_sections/frontend-listen/]
- [Source: https://v2.tauri.app/develop/calling-frontend/]
- [Source: https://v2.tauri.app/develop/tests/mocking/]
- [Source: https://zod.dev/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-03-26 02:00:05 +09:00 - Story 3.2 context 생성: Epic 3 / FR-007 / Post-End Completion Taxonomy, UX post-end guidance, 현재 workspace의 timing/post-end foundation, shared contracts, Rust normalized readiness, React provider/selectors, 공식 React/Tauri/Zod 문서를 함께 분석했다.
- 2026-03-26 02:44:12 +09:00 - shared contracts, adapter, provider, selector, CaptureScreen 테스트 헬퍼를 정리해 `postEnd`/`timing` optionality와 completion variant 타입 불일치를 해소하고 `pnpm build`, `pnpm test:run`을 통과시켰다.
- 2026-03-26 02:47:44 +09:00 - Rust host의 종료 직후 `export-waiting` durable truth 저장과 preset switch 차단을 보강하고 `cargo test --test capture_readiness --test session_manifest`를 통과시켰다.

### Completion Notes List

- 종료 직후 `ended`에 오래 머무르지 않고 host가 `export-waiting` 또는 truthful `completed`를 durable post-end truth로 기록하도록 정리했다.
- 고객 화면은 `Export Waiting`과 `Completed`를 분리해 안내하고, 완료 주장은 host post-end truth가 확인된 뒤에만 노출되도록 고정했다.
- 종료 후 preset 변경 차단, same-session post-end 병합, 저장된 현재 세션 캡처 자산 보존, post-end audit 로그 기록까지 함께 검증했다.

### File List

- _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md
- src/shared-contracts/schemas/capture-readiness.ts
- src/shared-contracts/schemas/session-capture.ts
- src/shared-contracts/schemas/session-manifest.ts
- src/shared-contracts/dto/session.ts
- src/shared-contracts/contracts.test.ts
- src/capture-adapter/services/capture-runtime.ts
- src/capture-adapter/services/capture-runtime.test.ts
- src/completion-handoff/post-end.ts
- src/booth-shell/selectors/customerStatusCopy.ts
- src/booth-shell/selectors/customerStatusCopy.test.ts
- src/booth-shell/components/HandoffReadyPanel.tsx
- src/booth-shell/components/PhoneRequiredSupportCard.tsx
- src/booth-shell/components/LatestPhotoRail.tsx
- src/booth-shell/screens/CaptureScreen.tsx
- src/booth-shell/screens/CaptureScreen.test.tsx
- src/session-domain/state/session-provider.tsx
- src/session-domain/state/session-provider.test.tsx
- src-tauri/src/lib.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/capture/normalized_state.rs
- src-tauri/src/handoff/mod.rs
- src-tauri/src/session/session_repository.rs
- src-tauri/tests/capture_readiness.rs
- src-tauri/tests/session_manifest.rs

### Change Log

- 2026-03-26: Story 3.2 구현 완료 후 상태를 `review`로 변경하고, post-end truth 계약/호스트 평가/UI 안내/회귀 테스트 결과를 기록했다.
- 2026-03-26: 코드 리뷰에서 확인된 handoff truth 강등, 3.3 범위 UI 조기 노출, handoff 제목 라벨 버그를 수정했다. Story 6.2 canonical ledger 정렬 이후에는 end-of-session hardware evidence가 아직 없어 상태를 `review`로 유지한다.

### Review Findings

- [x] [Review][Patch] `finalReady`가 handoff metadata 파일 상태에 따라 `local-deliverable-ready`로 강등됨 [src-tauri/src/handoff/mod.rs:179]
- [x] [Review][Patch] Story 3.3로 미뤄 둔 handoff/phone-required 상세 패널이 Story 3.2 화면에 이미 노출됨 [src/booth-shell/screens/CaptureScreen.tsx:163]
- [x] [Review][Patch] `nextLocationLabel`만 있는 handoff 안내가 잘못된 제목(`승인된 수령 대상`)으로 렌더링됨 [src/booth-shell/components/HandoffReadyPanel.tsx:12]

#### 2026-03-30 Re-review

- [x] [Review][Patch] `finalReady` 완료가 handoff 안내 파일 누락/파손 시 `local-deliverable-ready`로 강등됨 [src-tauri/src/handoff/mod.rs:314]
- [x] [Review][Patch] Story 3.3 범위의 handoff/phone-required 상세 UI가 Story 3.2 화면에서 그대로 노출됨 [src/booth-shell/screens/ReadinessScreen.tsx:46]
- [x] [Review][Patch] finalized post-end를 보존하는 프런트 병합 로직이 `completed`와 `phone-required` 사이의 host 정정을 무시함 [src/session-domain/state/session-provider.tsx:66]
- [x] [Review][Patch] 기존 `handoff-ready` 레코드에 목적지 라벨이 비어 있어도 그대로 재사용해 schema-invalid truth가 유지됨 [src-tauri/src/handoff/mod.rs:331]
- [x] [Review][Patch] handoff 안내 JSON의 blank string 라벨을 정규화하지 않아 schema-invalid post-end payload를 만들 수 있음 [src-tauri/src/handoff/mod.rs:381]
