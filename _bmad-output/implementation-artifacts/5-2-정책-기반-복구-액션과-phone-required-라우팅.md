# Story 5.2: 정책 기반 복구 액션과 Phone Required 라우팅

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

remote operator로서,
현재 장애 범주에 허용된 복구 액션만 실행하고 싶다.
그래서 위험하거나 무제한인 조치를 하지 않고 부스를 안전한 다음 상태로 되돌리거나 `Phone Required`로 보호 전환할 수 있다.

## Acceptance Criteria

1. 차단된 세션 범주가 식별되면 운영자가 available actions panel을 열 때, 콘솔은 해당 범주에서 `Operator Recovery Policy`가 허용한 액션만 보여줘야 한다. 또한 허용되지 않은 액션은 UI에서 실행할 수 없어야 한다.
2. retry, 승인된 boundary restart, 허용된 time extension 같은 허용 액션을 운영자가 선택해 완료되면, 세션은 올바른 다음 normalized state로 전환되거나 안전 복구를 계속할 수 없을 때 `Phone Required`로 전환돼야 한다. 또한 이 액션은 고객 흐름에 unsafe recovery control을 노출하면 안 된다.

## Tasks / Subtasks

- [x] operator recovery action 계약과 정책 분류를 추가한다. (AC: 1, 2)
  - [x] `src/shared-contracts/`와 `src-tauri/src/contracts/`에 blocked category, operator-safe diagnostics summary, allowed action enum/list, action request/result, typed rejection reason을 추가한다.
  - [x] blocked category는 최소 `capture`, `preview-or-render`, `timing-or-post-end`로 정규화하고, action set은 PRD의 최소 기준인 `retry`, `approved boundary restart`, `approved time extension`, `route to Phone Required`만 허용한다.
  - [x] disallowed action은 "UI에서 숨김"만으로 끝내지 말고 host가 다시 한 번 정책 위반을 거절하는 typed guard를 둔다.
- [x] Rust host에 정책 평가와 복구 실행 seam을 구현한다. (AC: 1, 2)
  - [x] `src-tauri/src/commands/operator_commands.rs`와 그 하위 domain module을 추가해 현재 세션의 blocked category별 허용 액션 집합을 계산하고 실행한다.
  - [x] retry / restart / extension / phone-required routing은 기존 host-owned 진실 위에서만 동작하게 하고, React local state나 raw helper text를 기반으로 직접 판정하지 않는다.
  - [x] 성공 시 normalized readiness 또는 post-end truth를 갱신하고, 실패 또는 정책 한계 도달 시 `Phone Required` 보호 상태로 수렴시킨다.
  - [x] 이번 스토리에서는 full audit explorer를 만들지 않고, 후속 Story 5.3가 확장할 수 있는 host-owned intervention record seam만 남긴다.
- [x] operator console UI에 bounded actions panel을 추가한다. (AC: 1, 2)
  - [x] `OperatorSummaryScreen` placeholder를 확장하거나 분리해 현재 세션 문맥, blocked category, 최근 실패 요약, 허용 액션만 보여 준다.
  - [x] operator route 안에서만 recovery CTA가 노출되게 유지하고, booth customer flow 어디에도 operator action affordance를 새로 만들지 않는다.
  - [x] action 결과는 운영자가 이해할 수 있는 진단 요약과 다음 상태만 보여 주고 raw filesystem path, helper stdout/stderr, Rust panic text를 그대로 노출하지 않는다.
- [x] typed frontend service/provider 경계로 operator action을 연결한다. (AC: 1, 2)
  - [x] `src/operator-console/services/` 또는 동등한 domain service를 추가해 command 호출과 schema 검증을 캡슐화한다.
  - [x] stale/foreign session action response가 현재 operator view를 오염시키지 못하게 request/response session 일치성 검증을 둔다.
  - [x] one-shot privileged mutation은 command 중심으로 처리하고, 결과 반영은 explicit refresh 또는 기존 normalized state update를 재사용한다.
- [x] 테스트로 정책 경계와 `Phone Required` 수렴을 잠근다. (AC: 1, 2)
  - [x] contract/unit test: action schema, blocked category mapping, 정책 위반 거절 reason, session mismatch rejection을 검증한다.
  - [x] UI test: `/operator` 보호, 허용 액션만 노출, disallowed action 비실행, raw diagnostics 비노출, action 후 상태 갱신을 검증한다.
  - [x] Rust test: category별 allowed action 집합, retry/restart/extension 성공 시 next normalized state, 복구 한계 초과 시 `Phone Required` 라우팅을 검증한다.
  - [x] regression test: active session asset truth가 retry/restart/phone-required routing 때문에 손상되거나 cross-session 상태가 섞이지 않는지 검증한다.

### Review Findings

- [x] [Review][Patch] 승인된 시간 연장이 실제로 세션을 다시 열 수 없을 만큼 늦은 경우에도 성공처럼 처리되던 문제를 막았다. [src-tauri/src/diagnostics/recovery.rs:365]
- [x] [Review][Patch] 거절된 recovery 결과의 next state가 capture-blocked를 전부 `camera-preparing`으로 뭉개던 문제를 실제 booth readiness truth 재조회로 수정했다. [src-tauri/src/diagnostics/recovery.rs:566]
- [x] [Review][Patch] 새 summary를 불러와도 이전 세션의 recovery 결과 카드가 남던 stale operator 결과 문제를 refresh 시점 정리로 수정했다. [src/operator-console/providers/operator-diagnostics-provider.tsx:47]
- [x] [Review][Patch] operator 화면에 raw blocked/reason/rejection enum이 보이던 문제를 설명형 라벨로 교체했다. [src/operator-console/screens/OperatorSummaryScreen.tsx:142]

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 Epic 5 / FR-009의 두 번째 가치 조각이다. 핵심은 운영자에게 "많은 버튼"을 주는 것이 아니라, 정책이 허용한 복구 액션만 안전하게 실행시키는 것이다.
- Story 5.1이 "무엇이 막혔는지 보여 주는 진단 가시화"라면, Story 5.2는 그 정규화된 blocked category 위에서 "무엇을 해도 되는지"를 제한하는 단계다.
- Story 5.3의 본격적인 감사 로그 탐색/조회까지 이번 스토리에 밀어 넣지 않는다. 다만 intervention 결과가 후속 감사 저장으로 이어질 수 있는 host-owned seam은 남겨야 한다.

### 스토리 기반 요구사항

- `Operator Recovery Policy` minimum baseline은 blocked state를 `capture`, `preview or render`, `post-end recovery` 범주로 먼저 정규화한 뒤 operator tool에 노출해야 한다. [Source: _bmad-output/planning-artifacts/prd.md#Named-Policy-References]
- approved operator action은 `retry`, `approved boundary restart`, `allowed time extension`, `Phone Required` 라우팅으로 제한된다. [Source: _bmad-output/planning-artifacts/prd.md#FR-009 Operational Safety and Recovery]
- customer-facing surface는 raw diagnostics, direct device control, internal recovery step을 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#Named-Policy-References]
- 복구가 성공하면 세션은 올바른 다음 normalized state로 돌아가야 하고, 안전 복구를 계속할 수 없으면 `Phone Required`로 수렴해야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 5.2: 정책 기반 복구 액션과 Phone Required 라우팅]
- `Phone Required`는 고객 보호 상태이며, 고객에게 self-recovery를 시키는 화면이 아니다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Phone Required 보호 흐름]

### 선행 의존성과 구현 순서

- 논리적 순서는 Story 5.1 -> Story 5.2 -> Story 5.3이다.
- 현재 `implementation-artifacts`에는 같은 Epic의 이전 스토리 파일인 5.1이 아직 없다. 따라서 5.2 구현 시 최소한의 diagnostics summary/blocked category baseline이 먼저 필요하다.
- 가장 안전한 구현 순서는 다음과 같다.
  - blocked category와 action contract를 먼저 고정한다.
  - host가 category별 allowed action을 계산하고 실행하도록 만든다.
  - operator UI는 그 host truth를 소비만 하도록 붙인다.
  - 마지막에 `Phone Required` 수렴과 회귀 테스트를 잠근다.

### 현재 워크스페이스 상태

- `/operator` route와 capability gate는 이미 있다. `src/app/routes.tsx`는 `SurfaceAccessGuard surface="operator"` 아래에 `OperatorSummaryScreen`을 연결하고 있다.
- `src/operator-console/screens/OperatorSummaryScreen.tsx`는 아직 placeholder이며 recovery action, diagnostics panel, audit-aware tooling은 미래 스토리에서 붙이겠다는 문구만 있다.
- `src/app/services/capability-service.ts`와 `src-tauri/src/commands/runtime_commands.rs`는 admin auth + allowed surfaces 조합으로 operator surface 접근을 제한한다.
- 현재 shared contract와 capture readiness에는 이미 `Phone Required` customer state 및 `phone-required` reason code가 있다. 즉, 고객 보호 상태 자체는 새로 발명할 필요가 없다.
- `src/capture-adapter/services/capture-runtime.ts`는 여러 host failure를 `Phone Required` readiness로 정규화해 booth 흐름을 보호한다. operator recovery는 이 customer-safe fallback을 우회하지 말고, 같은 host truth를 더 풍부한 operator-safe diagnostics와 action set으로 확장해야 한다.
- `src/session-domain/state/session-provider.tsx`는 stale/foreign session readiness를 강하게 막고 있다. operator action 응답도 동일한 세션 일치성 규칙을 따라야 한다.
- Rust 쪽에는 아직 `operator_commands.rs`나 `src-tauri/src/diagnostics/` 같은 전용 recovery 경계가 없다. `src-tauri/src/lib.rs`의 invoke handler도 현재 capture/runtime/preset/session command만 등록한다.

### 이전 스토리 인텔리전스

- Story 2.4는 timing truth와 extension 성격의 정책 판단을 host가 소유해야 한다는 기준을 굳혔다. time extension이 이번 스토리에 들어오더라도 React가 임의로 시간을 늘리면 안 된다. [Source: _bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md#구현-가드레일]
- Story 3.3은 `Phone Required`를 고객 보호 상태와 단일 연락 행동으로 고정했다. operator recovery가 실패했을 때도 booth는 같은 보호 경험으로 수렴해야지 새 고객용 오류 화면을 만들면 안 된다. [Source: _bmad-output/implementation-artifacts/3-3-handoff-ready와-phone-required-보호-안내.md#구현-가드레일]
- Story 4.4는 privileged action을 typed service + host command + stale response guard 조합으로 연결하는 패턴을 남겼다. operator recovery도 JSX에서 직접 `invoke`하지 말고 같은 패턴을 재사용하는 편이 안전하다. [Source: _bmad-output/implementation-artifacts/4-4-미래-세션-대상-롤백과-카탈로그-버전-관리.md#구현-가드레일]
- Story 2.1 / 2.2 계열은 current-session scope와 manifest-first truth를 강화했다. restart나 retry가 세션 자산 truth를 재작성하거나 다른 세션에 영향을 주면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]

### 최근 Git 인텔리전스

- 최근 5개 commit은 `feat: implement story 1-5 truthful preview waiting flow`, `chore: mark story 1-5 done` 등 Epic 1 중심이다.
- 현재 operator recovery와 직접 맞닿은 최신 커밋 히스토리는 거의 없으므로, 이번 스토리는 commit history보다 현재 코드 구조와 planning artifacts를 더 강한 기준선으로 삼는 편이 안전하다.

### 구현 가드레일

- 허용 액션 목록을 UI 편의상 하드코딩하고 host는 무조건 실행하는 구조로 만들면 안 된다. 정책 평가는 host가 최종 권위를 가져야 한다.
- disallowed action을 disabled 버튼으로만 남겨 두고 keyboard/programmatic path로 실행 가능하게 두면 안 된다. UI와 host 둘 다에서 막아야 한다.
- operator console이 raw helper 출력, filesystem path, internal enum, panic text를 그대로 보여 주면 안 된다. operator-safe diagnostic summary로 번역해야 한다.
- retry나 boundary restart가 session asset truth를 rewrite, cross-session reassignment, fake completion으로 이어지면 안 된다.
- time extension은 `Session Timing Policy` 안에서 승인된 범위만 허용해야 한다. arbitrary minute 입력이나 ad-hoc override slider를 추가하면 안 된다.
- booth customer flow에 operator recovery control, admin-only route, restart CTA를 새로 노출하면 안 된다.
- recovery 결과를 route 이동 자체로 판정하지 말고, host가 반환한 normalized readiness/post-end truth를 기준으로 다음 상태를 해석해야 한다.

### 아키텍처 준수사항

- FR-009의 구조 매핑은 `src/operator-console/`, `src/diagnostics-log/`, `src-tauri/src/diagnostics/`다. 현재 repo에 일부 경계가 없더라도 구현 방향은 이 분리를 향해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-to-Structure-Mapping]
- frontend-to-host privileged mutation은 typed adapter/service -> Tauri command -> Rust domain module 순서를 따라야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#API-&-Communication-Patterns]
- customer-facing projection과 operator-facing projection은 같은 host-normalized truth에서 갈라져야 한다. booth용, operator용 판단 로직을 별도로 중복 구현하면 drift 위험이 크다. [Source: _bmad-output/planning-artifacts/architecture.md#State-Normalization]
- capability gate는 계속 유지되어야 하며 operator surface는 admin authentication과 allowed surface check를 우회하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Authentication-&-Security]
- SQLite/audit store는 lifecycle와 intervention 기록을 위한 보조 저장소이지 photo/session durable truth를 대신하면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보 경로:
  - `src/operator-console/screens/OperatorSummaryScreen.tsx`
  - `src/app/routes.tsx`
  - `src/app/routes.test.tsx`
  - `src/app/providers/app-providers.tsx`
  - `src/app/services/capability-service.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src/shared-contracts/index.ts`
  - `src/shared-contracts/schemas/capture-readiness.ts`
  - `src/shared-contracts/dto/session.ts`
  - `src/capture-adapter/services/capture-runtime.ts`
  - `src/session-domain/state/session-provider.tsx`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/timing/mod.rs`
  - `src-tauri/src/handoff/mod.rs`
  - `src-tauri/src/session/session_manifest.rs`
  - `src-tauri/src/session/session_repository.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/mod.rs`
- 새 경계를 만들 경우 우선 후보:
  - `src/operator-console/services/operator-recovery-service.ts`
  - `src/operator-console/services/operator-recovery-service.test.ts`
  - `src/operator-console/state/`
  - `src/operator-console/components/`
  - `src/shared-contracts/dto/operator.ts`
  - `src/shared-contracts/schemas/operator-recovery.ts`
  - `src-tauri/src/commands/operator_commands.rs`
  - `src-tauri/src/diagnostics/`
  - `src-tauri/tests/operator_recovery.rs`

### UX 구현 요구사항

- 고객용 `Phone Required`는 보호 상태 설명과 단일 연락 행동이 핵심이다. operator recovery가 실패해도 고객 화면은 이 원칙을 깨면 안 된다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Phone Required 보호 흐름]
- operator UI는 고객보다 더 많은 진단 정보를 볼 수 있지만, 여전히 raw helper dump가 아니라 operator-safe diagnostic detail이어야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 5.1: 운영자용 현재 세션 문맥과 장애 진단 가시화]
- booth UI에는 recovery action이 생기면 안 되고, `Phone Required Support Card`의 보호 위계는 그대로 유지돼야 한다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- 고객 문구 밀도와 plain-language 원칙은 recovery 이후에도 유지돼야 한다. operator가 액션을 실행했다고 booth가 기술 진단어를 노출하면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]

### 최신 기술 확인 메모

- 현재 로컬 기준선은 `react 19.2.4`, `react-router-dom 7.13.1`, `@tauri-apps/api 2.10.1`, `zod 4.3.6`, `tauri 2.10.3`이다. 새 프레임워크를 들이는 것보다 기존 typed boundary 안에 operator recovery domain을 세우는 편이 안전하다. [Source: package.json] [Source: src-tauri/Cargo.toml]
- Tauri v2 공식 문서는 privileged backend work를 `#[tauri::command]` 경계로 두는 패턴을 설명한다. operator recovery action은 low-frequency privileged mutation이므로 command 중심 구현이 맞다. [Source: https://v2.tauri.app/develop/calling-rust/]
- Tauri capability 문서는 window/surface 권한을 capability 파일과 runtime gating으로 분리하는 구조를 설명한다. operator recovery도 기존 operator surface gate를 재사용해야 한다. [Source: https://v2.tauri.app/reference/acl/capability/]
- Tauri 문서는 Rust->frontend 이벤트보다 channel이 ordered/high-throughput stream에 더 적합하다고 설명한다. recovery action은 one-shot mutation이므로 command + explicit refresh가 우선이라는 판단은 공식 문서를 바탕으로 한 구현 추론이다. [Source: https://v2.tauri.app/develop/calling-frontend/]
- React 공식 문서는 `useEffectEvent`가 effect-fired logic에서 최신 값을 읽는 용도라고 설명한다. operator auto-refresh나 retry polling을 추가하더라도 dependency shortcut으로 오용하면 안 된다. [Source: https://react.dev/reference/react/useEffectEvent]
- Zod 공식 사이트는 Zod 4가 stable이라고 명시한다. operator recovery DTO도 ad-hoc parsing 대신 shared schema family 안에 넣는 편이 안전하다. [Source: https://zod.dev/]

### 테스트 요구사항

- 최소 필수 테스트 범위:
  - blocked category별 allowed action set이 정책 표와 일치한다.
  - disallowed action은 UI에서 실행할 수 없고, host에 직접 요청해도 거절된다.
  - retry / approved restart / approved extension 성공 시 세션이 올바른 next normalized state로 이동한다.
  - 안전 복구 한계 초과 시 customer flow는 `Phone Required`로 수렴한다.
  - operator action 응답이 다른 `sessionId`를 돌려주면 현재 화면을 오염시키지 못한다.
  - booth customer route 어디에도 operator recovery control이 생기지 않는다.
  - raw helper text, filesystem path, internal enum이 operator UI copy에 직접 노출되지 않는다.
  - recovery action 이후에도 기존 session asset truth와 capture binding이 보존된다.

### 금지사항 / 안티패턴

- operator에게 arbitrary shell-like restart, unbounded retry loop, direct file mutation 기능을 주는 것 금지
- JSX나 component body에서 직접 `invoke('operator_...')`를 호출하는 것 금지
- blocked category를 React에서 별도로 추정해 host truth와 다른 action set을 만드는 것 금지
- recovery 실패 시 booth를 generic error, fake completed, raw panic text로 보내는 것 금지
- `Phone Required` 라우팅을 customer self-recovery CTA와 함께 노출하는 것 금지
- Story 5.3 범위인 full intervention history explorer를 이번 스토리 안에서 과도하게 선구현하는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- 기존 고객 보호 컴포넌트: `src/booth-shell/components/PhoneRequiredSupportCard.tsx`
- 기존 operator placeholder: `src/operator-console/screens/OperatorSummaryScreen.tsx`
- capability gate: `src/app/routes.tsx`
- runtime capability: `src-tauri/src/commands/runtime_commands.rs`
- 관련 이전 스토리: `_bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md`
- 관련 이전 스토리: `_bmad-output/implementation-artifacts/3-3-handoff-ready와-phone-required-보호-안내.md`
- 관련 이전 스토리: `_bmad-output/implementation-artifacts/4-4-미래-세션-대상-롤백과-카탈로그-버전-관리.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.2: 정책 기반 복구 액션과 Phone Required 라우팅]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: 운영자 복구와 감사 로그]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.1: 운영자용 현재 세션 문맥과 장애 진단 가시화]
- [Source: _bmad-output/planning-artifacts/prd.md#Named-Policy-References]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-009 Operational Safety and Recovery]
- [Source: _bmad-output/planning-artifacts/prd.md#Fault Diagnosis and Recovery]
- [Source: _bmad-output/planning-artifacts/prd.md#Approved Operator Recovery Inventory]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-to-Structure-Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#API-&-Communication-Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication-&-Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#State-Normalization]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Phone Required 보호 흐름]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- [Source: _bmad-output/implementation-artifacts/2-4-조정된-종료-시각-표시와-경고-종료-알림.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/3-3-handoff-ready와-phone-required-보호-안내.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/4-4-미래-세션-대상-롤백과-카탈로그-버전-관리.md#구현-가드레일]
- [Source: package.json]
- [Source: src-tauri/Cargo.toml]
- [Source: https://v2.tauri.app/develop/calling-rust/]
- [Source: https://v2.tauri.app/reference/acl/capability/]
- [Source: https://v2.tauri.app/develop/calling-frontend/]
- [Source: https://react.dev/reference/react/useEffectEvent]
- [Source: https://zod.dev/]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-03-26 17:05:19 +09:00 - Story 5.2 context 생성: Epic 5 / FR-009 / operator recovery policy, current operator placeholder, capability gate, `Phone Required` 보호 흐름, 현재 shared contract 및 host 구조, React/Tauri/Zod 공식 문서를 함께 분석했다.
- 2026-03-26 23:34:00 +09:00 - operator recovery summary/action 계약, typed rejection reason, Rust recovery domain/command, bounded action panel, same-session guard를 연결했다.
- 2026-03-26 23:51:23 +09:00 - `pnpm lint`, `pnpm test:run`, `cargo test --manifest-path src-tauri/Cargo.toml`로 프런트와 Rust 회귀를 확인하고 operator recovery 통합 테스트를 통과시켰다.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story 5.1 implementation artifact 부재와 그에 따른 선행 진단 baseline 필요성을 명시했다.
- operator action scope를 `retry`, `approved boundary restart`, `approved time extension`, `Phone Required` routing으로 제한하고, full audit explorer는 Story 5.3로 남겼다.
- `/operator`에서 blocked category별 허용 액션만 노출하고, disallowed action은 UI와 host 양쪽에서 typed rejection으로 막도록 구현했다.
- preview/render retry, approved boundary restart, approved time extension, `Phone Required` 라우팅이 host-owned truth 위에서 다음 normalized state로 수렴하도록 연결했다.
- stale/foreign session action response가 현재 operator 화면을 오염시키지 못하게 same-session guard를 추가하고, 결과 화면은 operator-safe 요약과 다음 상태만 보여 주도록 제한했다.

### File List

- _bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/index.css
- src/operator-console/providers/operator-diagnostics-context.ts
- src/operator-console/providers/operator-diagnostics-provider.tsx
- src/operator-console/screens/OperatorSummaryScreen.test.tsx
- src/operator-console/screens/OperatorSummaryScreen.tsx
- src/operator-console/services/operator-diagnostics-service.test.ts
- src/operator-console/services/operator-diagnostics-service.ts
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/dto/operator.ts
- src/shared-contracts/schemas/index.ts
- src/shared-contracts/schemas/operator-recovery.ts
- src-tauri/src/commands/operator_commands.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/diagnostics/mod.rs
- src-tauri/src/diagnostics/recovery.rs
- src-tauri/src/lib.rs
- src-tauri/src/session/session_manifest.rs
- src-tauri/tests/operator_recovery.rs

### Change Log

- 2026-03-26 17:05:19 +09:00 - Story 5.2 ready-for-dev context와 implementation guardrails를 생성했다.
- 2026-03-26 23:52:02 +09:00 - operator recovery action contract, bounded operator action panel, Rust recovery command/domain, same-session guard, and policy/regression tests를 추가했다.
