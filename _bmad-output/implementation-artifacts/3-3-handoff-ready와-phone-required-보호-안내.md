# Story 3.3: Handoff Ready와 Phone Required 보호 안내

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
handoff가 준비됐는지 아니면 도움을 요청해야 하는지 명확히 알고 싶다.
그래서 추측하거나 위험한 행동을 하지 않고 자신 있게 부스를 나갈 수 있다.

## Acceptance Criteria

1. 부스가 세션을 `Handoff Ready`로 판정했을 때 handoff 화면이 표시되면, 고객은 승인된 수령 대상 또는 다음 이동 위치와 승인된 다음 행동을 함께 봐야 한다. 또한 downstream handoff에 필요하다면 booth alias가 같이 보여야 한다.
2. 세션이 승인된 범위 안에서 정상적으로 마무리될 수 없어서 `Phone Required`에 들어가면, 화면은 customer-safe 문구로 보호 상태를 설명하고 하나의 주요 연락 행동만 제공해야 한다. 또한 반복 촬영 시도나 기기 재시도 같은 위험한 self-recovery 행동은 짧고 분명하게 막아야 한다.

## Tasks / Subtasks

- [x] post-end guidance 계약을 host-owned truth로 고정한다. (AC: 1, 2)
  - [x] `Completed`는 계속 단일 post-end lifecycle truth로 유지하고, `Handoff Ready`는 그 안의 completion variant 또는 동등한 typed handoff payload로 표현해 별도 임의 lifecycle state를 발명하지 않는다.
  - [x] `src/shared-contracts/`와 `src-tauri/src/contracts/` 및 `session.json` family를 함께 확장해, 고객 화면에 필요한 최소 handoff 정보만 담는다. 예: `completionVariant`, `approvedRecipientLabel` 또는 `nextLocationLabel`, `primaryActionLabel`, `showBoothAlias`, `supportActionLabel`.
  - [x] 계약은 고객용 plain-language만 실어야 하며, render worker 상태, 내부 reason text, branch-local 진단 문자열을 직접 실으면 안 된다.
- [x] Rust host가 `Handoff Ready`와 `Phone Required` 안내를 정규화하도록 보강한다. (AC: 1, 2)
  - [x] `src-tauri/src/handoff/` 또는 이미 생긴 completion 경계를 우선 사용하고, 없다면 그 방향으로 책임을 모아 post-end 판정과 안내 구성을 구현한다.
  - [x] `SessionPaths.handoff_dir`와 세션 루트 구조를 재사용해 handoff 산출물 준비 여부와 안내 메타데이터를 한 세션 경계 안에서 평가한다.
  - [x] `Phone Required`는 operator recovery나 raw failure를 직접 노출하지 말고, approved boundary 밖이면 언제나 고객 보호 상태와 단일 연락 액션으로 수렴시킨다.
- [x] typed adapter와 provider가 post-end truth를 덮어쓰지 않게 연결한다. (AC: 1, 2)
  - [x] `capture-readiness` 및 관련 DTO가 같은 세션의 post-end payload만 수용하게 하고, stale/foreign session update가 잘못된 handoff 대상이나 booth alias를 보여 주지 못하게 막는다.
  - [x] `SessionProvider`의 local fallback/derive 로직이 `handoff-ready`, `completed`, `phone-required` 같은 host truth를 `capture-ready` 또는 generic preparing 상태로 되돌리지 않게 수정한다.
  - [x] `/booth` surface 내부 상태 전이만으로 해결하고, route 이동 자체를 post-end truth 신호로 사용하지 않는다.
- [x] booth UI에 Handoff Ready / Phone Required 보호 경험을 추가한다. (AC: 1, 2)
  - [x] UX 문서의 `Phone Required Support Card`를 반영한 재사용 컴포넌트 또는 동등 패턴을 추가한다.
  - [x] `Handoff Ready`에서는 승인된 수령 대상 또는 이동 위치, 다음 행동, 필요 시 booth alias만 보여 주고, 고객이 스스로 장비를 조작하거나 재촬영해야 한다는 인상을 주지 않는다.
  - [x] finalized post-end 상태에서는 촬영, 프리셋 변경, 삭제 같은 booth-loop 액션을 숨기거나 비활성화해 unsafe self-recovery를 유도하지 않는다.
- [x] 테스트로 post-end 보호 경계를 잠근다. (AC: 1, 2)
  - [x] contract/unit test: handoff payload schema, booth alias 노출 조건, foreign session payload 거부를 검증한다.
  - [x] UI test: `Handoff Ready` 안내, 단일 연락 액션, 위험 행동 차단 문구, 기술 용어 비노출을 검증한다.
  - [x] provider/integration test: host가 보낸 post-end truth를 fallback 로직이 덮어쓰지 않는지, 다른 세션 readiness가 현재 화면을 오염시키지 않는지 검증한다.
  - [x] Rust test: handoff-ready 판정, required handoff metadata 누락 시 안전 fallback, finalized post-end 상태의 capture/delete 차단을 검증한다.

### Review Findings

- [x] [Review][Patch] post-end 전용 UI가 실제 화면에 연결되지 않아 handoff-ready / phone-required 안내가 계속 generic completed 상태로 남아 있어요. [src/booth-shell/screens/ReadinessScreen.tsx:33]
- [x] [Review][Patch] `Phone Required`의 유일한 주요 연락 행동이 비활성 버튼으로 남아 있어 고객이 안내된 다음 행동을 실행할 수 없어요. [src/booth-shell/screens/ReadinessScreen.tsx:70]
- [x] [Review][Patch] `handoff-ready` 계약이 승인된 수령 대상이나 이동 위치 없이도 통과돼 AC 1의 필수 안내가 비어 있을 수 있어요. [src/shared-contracts/schemas/session-manifest.ts:28]
- [x] [Review][Patch] 확장된 `postEnd` 계약이 legacy manifest 역직렬화 전에 적용돼 기존 세션 manifest를 읽는 순간 실패할 수 있어요. [src-tauri/src/session/session_manifest.rs:84]

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 Epic 3 / FR-007의 세 번째 가치 조각이다. 핵심은 post-end 상태를 더 많이 만드는 것이 아니라, 이미 정규화된 post-end truth를 고객이 마지막 순간에 안전하게 이해하도록 만드는 것이다.
- `Handoff Ready`는 새 booth journey가 아니라 `Completed` 안의 presentation variant다. 고객에게는 "어디로 가야 하는지"와 "무엇을 하면 되는지"만 보여야 한다.
- `Phone Required`는 실패 원인 설명 화면이 아니라 보호 화면이다. 고객이 장비를 다시 만지거나 반복 촬영을 시도하지 않도록 다음 행동을 하나로 압축해야 한다.

### 스토리 기반 요구사항

- post-end 진실은 `Export Waiting`, `Completed`, `Phone Required` 중 하나의 명시적 상태로만 표현해야 한다. [Source: _bmad-output/planning-artifacts/prd.md#Post-End Completion Taxonomy]
- `Completed`는 `Local Deliverable Ready` 또는 `Handoff Ready`로만 해석돼야 하며, handoff는 별도 lifecycle truth가 아니라 completion variant다. [Source: _bmad-output/planning-artifacts/prd.md#Post-End Completion Taxonomy]
- handoff 안내는 승인된 recipient 또는 next location과 approved next action만 보여야 하고, 필요할 때만 booth alias를 보여야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 3.3: Handoff Ready와 Phone Required 보호 안내]
- `Phone Required` 화면은 보호 상태 설명, 단일 연락 액션, 짧은 금지 행동 안내를 가져야 한다. [Source: _bmad-output/planning-artifacts/epics.md#UX-DR8]

### 선행 의존성과 구현 순서

- 권장 구현 순서는 Story 3.1 -> Story 3.2 -> Story 3.3이다.
- Story 3.1은 exact-end 이후 `Export Waiting`, `Completed`, `Phone Required` 중 하나로 즉시 들어가는 경계를 만든다.
- Story 3.2는 `Export Waiting`과 truthful completion 기준, 즉 "언제 실제로 완료라고 말해도 되는지"를 잠근다.
- 현재 sprint 기준으로 3.1과 3.2 story artifact는 이미 `ready-for-dev` 상태다. 3.3 구현은 그 baseline을 재사용하는 방향이 가장 안전하다.

### 현재 워크스페이스 상태

- 현재 계약에는 이미 `captureReasonCode`로 `export-waiting`, `completed`, `phone-required`가 있고, `SessionCaptureRecord.postEndState`에는 `handoffReady`와 `completed`가 정의돼 있다.
- 그러나 실제 UI는 아직 `Preview Waiting` 중심으로 설계돼 있고, `Handoff Ready` 전용 customer-safe panel이나 `Phone Required Support Card`는 없다.
- `LatestPhotoRail`은 현재 `postEndState === 'completed'`만 삭제 금지 힌트로 다루므로, `handoffReady`가 finalized completion variant가 되면 이 경계도 함께 정리해야 한다.
- `SessionProvider`의 `deriveLifecycleStage`와 local fallback readiness는 preview/ready/phone-required 중심이며, post-end completion truth를 충분히 보존하도록 설계돼 있지 않다.
- 현재 worktree에는 Epic 2 timing 관련 변경이 진행 중이다. `src-tauri/src/timing/mod.rs`, `src/shared-contracts/dto/timing.ts`, `src/shared-contracts/schemas/session-timing.ts` 등은 이미 추가되었거나 수정 중이므로, 3.3은 이 기반을 재사용해야지 새 timing stack을 다시 만들면 안 된다.

### 이전 스토리 인텔리전스

- Story 1.5는 "저장 완료"와 "결과 준비 중"을 분리하는 truthful waiting 기준을 만들었다. 3.3도 같은 원칙으로, handoff가 실제 준비되기 전에는 완료처럼 말하면 안 된다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md#구현-가드레일]
- Story 2.1과 2.2는 current-session scope, stale/foreign update 차단, session-scoped asset guard를 강화했다. 3.3의 handoff 안내도 다른 세션 alias나 handoff 대상이 섞이면 안 된다. [Source: _bmad-output/implementation-artifacts/2-1-현재-세션-사진-레일과-세션-범위-검토.md#구현-가드레일]
- Story 2.3은 현재 룩과 과거 캡처 바인딩을 분리했다. post-end handoff가 들어와도 과거 캡처 바인딩이나 rail ordering을 다시 쓰면 안 된다. [Source: _bmad-output/implementation-artifacts/2-3-이후-촬영용-활성-프리셋-변경.md#구현-가드레일]
- Story 2.4 worktree 기준선은 timing truth를 host 소유로 밀어 올리는 방향이다. 3.3은 종료 후 고객 안내를 추가하더라도 종료 판정 자체를 React가 새로 계산하면 안 된다. [Source: _bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md#구현-가드레일]

### 구현 가드레일

- `Handoff Ready`를 별도 임의 lifecycle state로 분기하지 말고, PRD taxonomy를 따라 `Completed` 안의 variant로 유지한다.
- booth alias는 downstream handoff에 필요할 때만 노출해야 한다. 모든 완료 화면에 항상 보여 주면 PII 최소화 원칙을 깨기 쉽다.
- `Phone Required`는 단일 연락 액션이 핵심이다. "다시 찍기", "기기 재시작", "앱 닫기" 같은 self-recovery 행동을 primary 또는 동등한 수준으로 두면 안 된다.
- finalized post-end 상태에서는 capture/delete/preset-switch affordance가 더 이상 주 흐름이면 안 된다. 남겨두더라도 disabled 상태와 보호 카피로 명시적으로 막아야 한다.
- local fallback readiness가 host post-end truth를 generic `Preparing`이나 `Ready`로 바꾸지 못하게 해야 한다. 특히 session mismatch, stale event, late preview update가 post-end guidance를 덮으면 안 된다.

### 아키텍처 준수사항

- post-end completion/handoff 책임은 아키텍처상 `src/completion-handoff/` 와 `src-tauri/src/handoff/`로 분리되는 것이 목표다. 현재 폴더가 없다면 이번 스토리 또는 선행 스토리에서 그 방향으로 모아야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- React UI는 typed adapter/service를 통해서만 host truth를 소비해야 한다. JSX에서 직접 `invoke`하거나 route change를 truth로 삼으면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- 세션 루트의 `handoff/` 디렉터리와 `session.json`은 여전히 primary session truth 경계다. handoff-ready 산출물이나 메타데이터도 이 경계를 벗어나면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- 고객 화면과 운영자 화면은 같은 host-normalized truth에서 갈라져야 한다. customer copy와 operator diagnostics가 서로 다른 ad-hoc 판단을 하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보 경로:
  - `src/booth-shell/screens/CaptureScreen.tsx`
  - `src/booth-shell/screens/CaptureScreen.test.tsx`
  - `src/booth-shell/screens/ReadinessScreen.tsx`
  - `src/booth-shell/selectors/customerStatusCopy.ts`
  - `src/booth-shell/selectors/customerStatusCopy.test.ts`
  - `src/booth-shell/components/LatestPhotoRail.tsx`
  - `src/session-domain/state/session-provider.tsx`
  - `src/session-domain/state/session-provider.test.tsx`
  - `src/session-domain/selectors/current-session-previews.ts`
  - `src/shared-contracts/schemas/capture-readiness.ts`
  - `src/shared-contracts/schemas/session-capture.ts`
  - `src/shared-contracts/schemas/session-manifest.ts`
  - `src/shared-contracts/dto/session.ts`
  - `src/shared-contracts/dto/timing.ts`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/timing/mod.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `docs/contracts/session-manifest.md`
- 새 경계를 만들 경우 우선 후보:
  - `src/booth-shell/components/PhoneRequiredSupportCard.tsx`
  - `src/booth-shell/components/HandoffReadyPanel.tsx`
  - `src/completion-handoff/`
  - `src-tauri/src/handoff/`

### UX 구현 요구사항

- `Phone Required`는 일반 오류 박스가 아니라 별도 위계의 보호 화면이어야 한다. 고객은 원인보다 다음 행동을 먼저 이해해야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns]
- `Phone Required Support Card`는 헤드라인, 현재 보호 상태 설명, 단일 연락 액션, 짧은 금지 행동 안내로 구성해야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- 고객 카피 밀도는 낮게 유지해야 하며, handoff와 phone-required 모두 기술 진단어·darktable 용어·운영자 내부 용어를 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]
- handoff-ready 화면은 고객이 "어디로 가야 하는지", "어떤 이름을 말해야 하는지", "지금 무엇을 하면 되는지"를 한 번에 이해하게 만들어야 한다. 버튼이나 문구 수를 늘려 선택지를 복잡하게 만들면 안 된다.

### 최신 기술 확인 메모

- 현재 로컬 기준선은 `react 19.2.4`, `react-router-dom 7.13.1`, `@tauri-apps/api 2.10.1`, `zod 4.3.6`이다. 이번 스토리는 새 의존성보다 existing typed boundary 위에서 post-end guidance를 확장하는 편이 안전하다. [Source: package.json]
- React 공식 문서는 `useEffectEvent`가 최신 render 값을 읽는 effect-fired logic용이며 stable identity를 보장하지 않는다고 설명한다. post-end one-shot focus, alert, cleanup을 넣더라도 dependency shortcut 용도로 오용하면 안 된다. [Source: https://react.dev/reference/react/useEffectEvent]
- Tauri 공식 문서는 frontend event system이 low latency/high throughput 스트림용이 아니고, ordered/fast stream에는 channel이 더 적합하다고 설명한다. post-end는 단발 상태 전환이므로 기존 event 기반이 가능하지만, 연속 progress streaming을 추가하려면 channel 도입을 의식적으로 검토해야 한다. 이것은 공식 문서를 바탕으로 한 구현 추론이다. [Source: https://v2.tauri.app/develop/calling-frontend/]
- Tauri 공식 문서는 테스트에서 `mockIPC(..., { shouldMockEvents: true })` 패턴으로 Rust emit 이벤트를 모킹할 수 있다고 안내한다. post-end readiness 회귀 테스트에 그대로 활용 가능하다. [Source: https://v2.tauri.app/develop/tests/mocking/]
- Zod 공식 문서는 Zod 4가 stable이라고 명시한다. 새 post-end payload도 ad-hoc parsing 대신 shared schema family에 넣는 편이 안전하다. [Source: https://zod.dev/v4/versioning?id=update--july-8th-2025]

### 테스트 요구사항

- 최소 필수 테스트 범위:
  - `Handoff Ready` 화면이 승인된 recipient/location과 next action만 보여 주는지
  - `showBoothAlias`가 참일 때만 booth alias가 노출되는지
  - `Phone Required`에서 primary action이 하나만 노출되고 capture/preset/delete가 보호 상태로 막히는지
  - finalized post-end 상태에서 late preview update, stale timing update, foreign session readiness가 화면을 되돌리지 못하는지
  - customer copy에 helper, SDK, filesystem, render worker, operator policy 같은 내부 용어가 없는지
  - handoff-ready와 phone-required 모두 current-session scope를 벗어난 자산/식별자를 보여 주지 않는지

### 금지사항 / 안티패턴

- `Handoff Ready`를 `Completed`와 별도의 lifecycle truth로 분기하는 것 금지
- booth alias 필요 여부와 무관하게 모든 완료 화면에 alias를 항상 노출하는 것 금지
- `Phone Required`에서 연락 액션 외에 재촬영, 재시작, 앱 종료를 동등한 주요 액션으로 노출하는 것 금지
- route 전환이나 로컬 state만으로 post-end 완료를 판정하는 것 금지
- `handoffReady`나 `completed` finalized state에서도 사진 정리/룩 변경/capture를 계속 정상 액션처럼 남겨 두는 것 금지
- 다른 세션의 alias, handoff 대상, preview asset이 현재 세션에 섞여도 그냥 렌더하는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- 계약 문서: `docs/contracts/session-manifest.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/2-1-현재-세션-사진-레일과-세션-범위-검토.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/2-3-이후-촬영용-활성-프리셋-변경.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.3: Handoff Ready와 Phone Required 보호 안내]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: 종료 후 결과 준비와 인계의 진실한 마무리]
- [Source: _bmad-output/planning-artifacts/epics.md#UX-DR8]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-007 Export Waiting, Final Readiness, and Handoff Guidance]
- [Source: _bmad-output/planning-artifacts/prd.md#Post-End Completion Taxonomy]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-005 Timing, Post-End, and Render Reliability]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns]
- [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/2-1-현재-세션-사진-레일과-세션-범위-검토.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/2-3-이후-촬영용-활성-프리셋-변경.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md#구현-가드레일]
- [Source: docs/contracts/session-manifest.md#필드-규칙]
- [Source: package.json]
- [Source: https://react.dev/reference/react/useEffectEvent]
- [Source: https://v2.tauri.app/develop/calling-frontend/]
- [Source: https://v2.tauri.app/develop/tests/mocking/]
- [Source: https://zod.dev/v4/versioning?id=update--july-8th-2025]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-03-26 02:02:54 +09:00 - Story 3.3 context 생성: Epic 3 / FR-007 / post-end taxonomy, handoff UX, current capture-readiness/session-manifest baseline, current worktree timing changes, React/Tauri/Zod 공식 문서를 함께 분석했다.
- 2026-03-26 02:46:07 +09:00 - handoff-ready / phone-required 계약 정리, host normalization, booth 보호 UI, provider fallback guard, 상태명 정리, contract/UI/Rust regression 및 production build 검증을 완료했다.

### Completion Notes List

- handoff-ready를 `completed` 내부 completion variant로 고정하고, phone-required를 고객 보호 중심 계약으로 정리했다.
- Rust host가 handoff 메타데이터를 세션 경계 안에서 평가하고, 누락 시 local-deliverable-ready로 안전 fallback 하도록 보강했다.
- booth 화면에 Handoff Ready 패널과 Phone Required 보호 카드, finalized post-end 액션 차단을 연결했다.
- stale/foreign readiness가 post-end truth를 덮지 못하도록 provider와 계약 테스트를 잠갔다.
- `pnpm build`, `pnpm vitest run src/shared-contracts/contracts.test.ts src/booth-shell/selectors/customerStatusCopy.test.ts src/booth-shell/screens/CaptureScreen.test.tsx src/session-domain/state/session-provider.test.tsx src/capture-adapter/services/capture-runtime.test.ts`, `cargo test --test capture_readiness`, `cargo test --test session_manifest`를 통과했다.

### File List

- _bmad-output/implementation-artifacts/3-3-handoff-ready와-phone-required-보호-안내.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- docs/contracts/session-manifest.md
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/dto/session.ts
- src/shared-contracts/schemas/capture-readiness.ts
- src/shared-contracts/schemas/session-capture.ts
- src/shared-contracts/schemas/session-manifest.ts
- src/capture-adapter/services/capture-runtime.test.ts
- src/capture-adapter/services/capture-runtime.ts
- src/completion-handoff/post-end.ts
- src/booth-shell/components/HandoffReadyPanel.tsx
- src/booth-shell/components/LatestPhotoRail.tsx
- src/booth-shell/components/PhoneRequiredSupportCard.tsx
- src/booth-shell/screens/CaptureScreen.test.tsx
- src/booth-shell/screens/CaptureScreen.tsx
- src/booth-shell/selectors/customerStatusCopy.test.ts
- src/booth-shell/selectors/customerStatusCopy.ts
- src/session-domain/state/session-provider.test.tsx
- src/session-domain/state/session-provider.tsx
- src-tauri/src/capture/normalized_state.rs
- src-tauri/src/handoff/mod.rs
- src-tauri/src/session/session_manifest.rs
- src-tauri/tests/capture_readiness.rs

### Change Log

- 2026-03-26 02:46:07 +09:00 - handoff-ready / phone-required host-owned post-end guidance, 보호 UI, finalized action 차단, 세션 격리 guard, 상태명 정리, 계약/회귀 테스트를 추가했다.
