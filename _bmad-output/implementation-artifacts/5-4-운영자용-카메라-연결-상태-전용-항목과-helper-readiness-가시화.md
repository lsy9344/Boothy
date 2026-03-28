# Story 5.4: 운영자용 카메라 연결 상태 전용 항목과 helper readiness 가시화

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

remote operator로서,
카메라 연결 상태를 별도 항목으로 바로 보고 싶다.
그래서 generic blocked-state만으로 추측하지 않고 false-ready 위험과 장비 연결 문제를 빠르게 식별할 수 있다.

## Acceptance Criteria

1. operator console이 active 또는 blocked 세션을 렌더링할 때, 화면은 기존 `blocked-state category`와 세 경계 카드와 별도로 `카메라 연결 상태` 전용 항목을 표시해야 한다. 이 항목은 현재 세션 문맥 안에서 항상 같은 위치에 보여야 하며, generic blocked-state를 대체하면 안 된다.
2. 전용 항목은 최소 `미연결`, `연결 중`, `연결됨`, `복구 필요` 또는 동등한 operator-safe 상태 집합을 가져야 한다. 이 상태 집합은 shared contract에 고정돼 TypeScript와 Rust가 같은 의미를 공유해야 하며, booth customer copy나 raw helper terminology를 재사용하면 안 된다.
3. 전용 항목의 상태는 raw helper stdout/stderr, filesystem path, 내부 enum 이름이 아니라 host-normalized camera/helper truth에서 계산돼야 한다. 특히 `camera-preparing`, `helper-preparing`, `ready/capture-ready`, degraded-after-ready 또는 stale truth 위험을 React가 임의 해석하지 말고 host projection에서 먼저 정규화해야 한다.
4. booth가 다른 이유로 blocked가 아니더라도 camera/helper truth가 흔들리는 경우, operator는 이 전용 항목에서 false-ready 위험을 먼저 읽을 수 있어야 한다. 반대로 preview/render/post-end 경계가 막혀 있어도 camera/helper가 건강하면 전용 항목은 `연결됨`을 유지할 수 있어야 한다.
5. operator surface는 이 전용 항목을 통해 helper readiness를 더 잘 보게 되더라도, booth customer surface에는 내부 진단 용어가 새지 않아야 한다. customer는 계속 plain-language readiness guidance만 보아야 한다.

## Tasks / Subtasks

- [ ] operator diagnostics shared contract에 dedicated camera connection projection을 추가한다. (AC: 1, 2, 3, 4)
  - [ ] `src/shared-contracts/schemas/operator-diagnostics.ts`, `src/shared-contracts/schemas/operator-recovery.ts`, `src/shared-contracts/dto/operator.ts`, `src-tauri/src/contracts/dto.rs`에 `카메라 연결 상태`용 typed object를 추가한다.
  - [ ] projection은 loose string 여러 개가 아니라 하나의 전용 summary object 또는 동등한 typed 구조로 정의하고, 최소 `state`, operator-safe `title`, `detail`, 필요 시 `observedAt`만 허용한다.
  - [ ] machine enum은 `disconnected`, `connecting`, `connected`, `recovery-required` 또는 동등한 집합으로 고정하고, 한국어 라벨은 UI에서 별도 매핑하되 booth copy를 재사용하지 않는다.
- [ ] host diagnostics/recovery read-model에 camera/helper truth projection을 추가한다. (AC: 2, 3, 4)
  - [ ] `src-tauri/src/diagnostics/mod.rs`에서 `normalize_capture_readiness(...)`, 현재 lifecycle stage, 최근 diagnostics context를 입력으로 dedicated camera connection summary를 계산한다.
  - [ ] `camera-preparing`과 `helper-preparing`은 동일한 blocked-state category로 뭉개지 말고, operator가 "연결 중"인지 "복구 필요"인지 구분할 수 있게 정규화한다.
  - [ ] first-connect 대기와 degraded-after-ready를 구분할 신호가 현재 부족하면, 최소한의 host-owned read-only context를 추가하되 React에서 ad-hoc 추정하지 않는다.
  - [ ] preview/render/post-end blockage 때문에 blocked-state category가 바뀌어도 camera/helper health projection은 독립적으로 유지한다.
- [ ] operator recovery summary와 UI를 함께 확장한다. (AC: 1, 2, 4, 5)
  - [ ] `OperatorRecoverySummary`가 `OperatorSessionSummary`를 확장하고 있으므로, recovery summary payload에서도 같은 camera connection projection을 노출한다.
  - [ ] `src/operator-console/screens/OperatorSummaryScreen.tsx`에 `카메라 연결 상태` 전용 카드 또는 사실상 동등한 강조 영역을 추가하고, current session facts와 blocked-state hero 사이에서 빠르게 읽히도록 배치한다.
  - [ ] `Capture Boundary` 카드와 혼동되지 않도록 시각 위계를 분리하고, `연결됨`이더라도 preview/render/post-end blockage는 별도로 계속 읽히게 유지한다.
- [ ] 브라우저 fixture, typed service, 테스트를 갱신한다. (AC: 1, 2, 3, 4, 5)
  - [ ] `src/operator-console/services/operator-diagnostics-service.ts`와 관련 테스트에서 schema 변경 후 fixture 파싱과 same-session guard가 계속 유효한지 검증한다.
  - [ ] `src-tauri/tests/operator_diagnostics.rs`에 `미연결`, `연결 중`, `연결됨`, `복구 필요` 시나리오를 각각 추가한다.
  - [ ] `src/operator-console/screens/OperatorSummaryScreen.test.tsx`에서 dedicated item 노출, operator-safe copy, 기존 blocked-state/경계 카드와의 공존을 검증한다.
  - [ ] customer booth copy 테스트는 기존처럼 internal helper 용어 비노출을 유지해야 한다.

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 Epic 5 / FR-009의 보강 조각이다. 목적은 새 recovery action을 더 만드는 것이 아니라, 이미 존재하는 operator diagnostics 위에 `카메라 연결 상태`를 1급 운영 신호로 승격하는 것이다.
- Story 5.1이 "현재 세션과 막힌 경계"를 보여 줬다면, 5.4는 그 안에서 false-ready와 helper readiness 리스크를 generic boundary summary 뒤에 숨기지 않게 만드는 단계다.
- 이번 스토리는 booth customer readiness truth를 다시 설계하는 작업이 아니다. customer는 계속 plain-language readiness를 보고, operator만 더 밀도 높은 진단을 본다.

### 스토리 기반 요구사항

- Epic 5.4는 operator console이 dedicated `카메라 연결 상태` 항목을 보여 주고, 이 항목이 host-normalized camera/helper truth에서 계산돼야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 5.4: 운영자용 카메라 연결 상태 전용 항목과 helper readiness 가시화]
- FR-009은 operator가 blocked state와 recovery를 bounded하게 다루도록 요구하지만, 그 진단은 customer-safe truth와 분리된 operator-safe projection이어야 한다. [Source: _bmad-output/planning-artifacts/prd.md#FR-009 Operational Safety and Recovery]
- named policy baseline은 customer-facing surface에 raw diagnostics와 internal recovery detail을 노출하지 않도록 못 박는다. [Source: _bmad-output/planning-artifacts/prd.md#Named Policy References]
- operator workflow는 active blocked boundary를 빨리 식별하고 customer-safe guidance를 유지해야 한다. dedicated camera connection item은 이 fault diagnosis 속도를 높이는 보강이어야 한다. [Source: _bmad-output/planning-artifacts/prd.md#Fault Diagnosis and Recovery]

### 선행 의존성과 구현 순서

- 직접 선행 흐름은 Story 1.6 성격의 camera/helper readiness truth, Story 5.1 operator diagnostics baseline, Story 5.2 bounded recovery actions, Story 5.3 audit history다.
- 현재 operator UI는 `OperatorRecoverySummary`를 기준으로 렌더링하므로, safest path는 `OperatorSessionSummary` 공통 payload를 먼저 확장한 뒤 recovery summary와 UI가 이를 재사용하게 만드는 것이다.
- 권장 구현 순서는 다음과 같다.
  - shared contract와 Rust DTO에 dedicated camera connection summary shape를 먼저 고정한다.
  - host diagnostics projection이 `camera-preparing`, `helper-preparing`, `ready`, degraded-after-ready를 한 번에 정규화하도록 만든다.
  - operator recovery summary가 같은 projection을 그대로 재노출하게 만든다.
  - 마지막에 operator UI와 Rust/UI 테스트를 함께 잠근다.

### 현재 워크스페이스 상태

- `src/shared-contracts/schemas/operator-diagnostics.ts`의 `operatorSessionSummarySchema`는 현재 `blockedStateCategory`, `recentFailure`, `captureBoundary`, `previewRenderBoundary`, `completionBoundary`만 가지고 있고 dedicated camera connection field가 없다.
- `src/shared-contracts/schemas/operator-recovery.ts`의 `operatorRecoverySummarySchema`는 `operatorSessionSummarySchema`를 확장하므로, 5.4는 recovery summary 쪽에 별도 중복 field를 설계하기보다 공통 session summary projection을 확장하는 편이 자연스럽다.
- `src-tauri/src/diagnostics/mod.rs`의 `load_operator_session_summary_in_dir(...)`는 이미 `normalize_capture_readiness(...)`를 호출해 reason code와 lifecycle/timing/post-end truth를 읽는다. 하지만 결과를 dedicated camera connection item으로 재투영하지 않고 blocked boundary와 recent failure에만 녹여 넣고 있다.
- `src/operator-console/screens/OperatorSummaryScreen.tsx`는 현재 hero, current session facts, recent failure, recovery action, audit history, boundary cards를 렌더링하지만 `카메라 연결 상태` 전용 카드나 badge는 없다.
- `src/operator-console/services/operator-diagnostics-service.ts`는 recovery summary와 audit history를 schema parse 후 소비한다. 따라서 schema가 바뀌면 fixture와 parsing tests를 함께 갱신해야 한다.

### 관련 이전 스토리 인텔리전스

- `src-tauri/src/capture/normalized_state.rs`는 valid preset 이후 lifecycle stage가 `helper-preparing`, `camera-preparing`, `ready/capture-ready`, `preview-waiting`, `phone-required` 등으로 갈라질 때 customer readiness를 host에서 한 번 정규화한다. 5.4는 이 정규화 결과를 operator 전용 시야로 다시 표현해야지 React에서 새 truth를 만들면 안 된다. [Source: src-tauri/src/capture/normalized_state.rs]
- Story 5.1은 operator-safe diagnostics read-model과 boundary card 구조를 이미 만들었다. 5.4는 이 구조를 대체하지 않고, camera/helper health를 독립된 진단 항목으로 보강해야 한다. [Source: _bmad-output/implementation-artifacts/5-1-운영자용-현재-세션-문맥과-장애-진단-가시화.md#구현-가드레일]
- Story 5.2는 blocked category 기반 allowed action policy를 이미 고정했다. 5.4가 camera connection item을 추가하더라도 recovery policy의 최종 권위는 계속 blocked category + host policy에 있어야 하며, dedicated item이 action allowlist를 임의로 바꾸면 안 된다. [Source: _bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md#구현-가드레일]
- Story 5.3은 operator history query와 audit record를 이미 갖고 있다. degraded-after-ready 판단에 audit/history가 도움이 되더라도 이번 스토리 범위는 read-model 보강이지 새 감사 기능 추가가 아니다. [Source: _bmad-output/implementation-artifacts/5-3-라이프사이클-개입-복구-감사-로그-기록.md#구현-가드레일]

### EDSDK helper projection 기준선

- 현재 camera helper는 `docs/contracts/camera-helper-edsdk-profile.md`를 따르는 Windows 전용 Canon EDSDK helper exe를 기준으로 본다.
- operator용 `카메라 연결 상태`는 helper raw detailCode를 그대로 노출하는 항목이 아니라, host가 EDSDK helper 상태를 bounded vocabulary로 접은 결과여야 한다.
- 구현 초기에는 아래 접힘을 기준선으로 삼는 편이 안전하다.
  - `camera-not-found`, `usb-disconnected`, `unsupported-camera` -> `미연결`
  - `sdk-initializing`, `session-opening`, first fresh status 대기 -> `연결 중`
  - `connected-idle`, `camera-ready` -> `연결됨`
  - `reconnect-pending`, `sdk-init-failed`, degraded-after-ready, 반복 recovery -> `복구 필요`
- 위 접힘은 제품 구현 기준선이지 customer copy가 아니다. booth 화면은 계속 plain-language readiness만 본다.

### 구현 가드레일

- `카메라 연결 상태`는 `blockedStateCategory`의 label 변형이 아니어야 한다. preview/render/post-end blockage와 orthogonal한 별도 신호로 유지해야 한다.
- React component가 `reasonCode`, `lifecycleStage`, `recentFailure`를 조합해 camera connection state를 임의 계산하면 안 된다. host projection이 최종 권위를 가져야 한다.
- raw helper stderr/stdout, absolute path, Rust/internal enum, diagnostics log 원문을 dedicated item detail로 그대로 노출하면 안 된다.
- `연결됨`은 "부스 전체가 정상"을 뜻하지 않는다. camera/helper만 건강하고 preview/render 또는 post-end가 막힌 경우를 계속 표현할 수 있어야 한다.
- 반대로 preview/render 또는 post-end가 정상이라고 해서 camera/helper 상태를 생략하면 안 된다. false-ready 리스크는 별도 항목에서 먼저 보이게 해야 한다.
- browser fixture와 operator service는 schema 변경 후에도 typed parse 실패를 감춰서는 안 된다. 테스트 fixture를 새 계약으로 갱신해야 한다.

### 아키텍처 준수사항

- frontend-to-host 요청/응답은 Tauri command와 typed adapter/service를 통해서만 흐른다. component에서 직접 `invoke`를 추가하지 않는다. [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- camera/helper truth, timing truth, completion truth는 host에서 한 번 정규화된 뒤 booth copy와 operator diagnostics로 갈라져야 한다. 5.4도 같은 원칙을 따라야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- operator surface는 admin authentication + capability gate + operator window label 안에서만 노출돼야 한다. dedicated item 추가가 booth surface 우회 노출로 이어지면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- session folder와 host diagnostics는 product truth를 소유하고, React cache는 그것을 넘어서면 안 된다. dedicated camera connection item도 manifest/readiness/diagnostics snapshot을 읽는 projection이어야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- FR 매핑상 Epic 5 관련 구현 중심 경로는 `src/operator-console/`, `src/shared-contracts/`, `src-tauri/src/diagnostics/`다. 새로운 책임도 이 경계를 우선 따라야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보 경로:
  - `src/shared-contracts/schemas/operator-diagnostics.ts`
  - `src/shared-contracts/schemas/operator-recovery.ts`
  - `src/shared-contracts/dto/operator.ts`
  - `src/shared-contracts/index.ts`
  - `src/shared-contracts/contracts.test.ts`
  - `src/operator-console/services/operator-diagnostics-service.ts`
  - `src/operator-console/services/operator-diagnostics-service.test.ts`
  - `src/operator-console/screens/OperatorSummaryScreen.tsx`
  - `src/operator-console/screens/OperatorSummaryScreen.test.tsx`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/src/diagnostics/mod.rs`
  - `src-tauri/src/diagnostics/recovery.rs`
  - `src-tauri/src/commands/operator_commands.rs`
  - `src-tauri/tests/operator_diagnostics.rs`
- 새 경계가 필요하면 우선 후보:
  - `src/operator-console/components/CameraConnectionCard.tsx`
  - `src/shared-contracts/schemas/operator-camera-connection.ts`
  - `src-tauri/src/diagnostics/camera_connection.rs`
- 단, 새 파일을 꼭 늘리는 것보다 기존 operator diagnostics 경계 안에 자연스럽게 넣는 편이 우선이다.

### UX 구현 요구사항

- operator 화면은 customer보다 높은 정보 밀도를 가질 수 있지만, 여전히 "행동 가능한 운영자 진단"이어야 한다. helper readiness를 보여 주되 raw helper dump처럼 보여서는 안 된다.
- `카메라 연결 상태`는 `Capture Boundary` 카드와 너무 비슷한 위치나 문구로 렌더링하면 안 된다. operator가 "장비 연결 건강"과 "현재 막힌 경계"를 한눈에 분리해서 읽을 수 있어야 한다.
- runbook 기준으로 operator 화면은 카메라 연결 전에는 `Capture 확인 필요`, 연결 후에는 `정상`, 재연결 이슈 시에는 즉시 리스크를 읽을 수 있어야 한다. 5.4는 이 운영 흐름을 더 직접적으로 만드는 보강이다. [Source: docs/runbooks/booth-hardware-validation-checklist.md#앱-실행과-카메라-연결-확인-진입점]
- customer booth 화면에는 계속 기술 진단어가 보이면 안 된다. dedicated item 추가 때문에 booth copy audit가 깨지면 안 된다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]

### 테스트 요구사항

- 최소 필수 테스트 범위:
  - operator summary가 camera disconnected 상태에서 dedicated item을 `미연결` 또는 동등 상태로 보여 준다.
  - lifecycle stage가 `camera-preparing` 또는 `helper-preparing`일 때 dedicated item이 `연결 중`으로 계산된다.
  - `ready/capture-ready`이면서 degraded signal이 없을 때 dedicated item이 `연결됨`으로 계산된다.
  - once-ready 이후 helper/camera truth가 흔들리거나 stale mismatch가 감지되면 dedicated item이 `복구 필요`로 바뀐다.
  - preview/render/post-end blockage가 있어도 camera/helper가 건강한 경우 dedicated item은 `연결됨`을 유지하고, blocked-state category는 기존대로 separate하게 남는다.
  - operator UI에는 dedicated camera item이 보이지만 booth customer copy에는 helper/operator terminology가 새지 않는다.
  - schema 변경 후 browser fixture와 typed service parsing이 계속 통과한다.

### 금지사항 / 안티패턴

- `Capture Boundary` 카드 제목만 바꿔서 `카메라 연결 상태` 요구를 충족했다고 주장하는 것 금지
- JSX 안에서 `reasonCode === 'camera-preparing'` 같은 ad-hoc 분기만으로 최종 state를 확정하는 것 금지
- raw helper path/stdout/stderr/internal error text를 operator-safe detail이라고 주장하며 노출하는 것 금지
- `연결됨`을 booth 전체 health summary처럼 써서 preview/render/post-end 문제를 가리는 것 금지
- `카메라 연결 상태` 추가 때문에 existing blocked-state category, allowed recovery actions, audit history를 깨는 것 금지
- booth customer route 또는 브라우저 preview fallback에 operator-only wording을 노출하는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- 운영 런북: `docs/runbooks/booth-hardware-validation-checklist.md`
- Canon EDSDK helper 구현 프로파일: `docs/contracts/camera-helper-edsdk-profile.md`
- 기존 operator summary UI: `src/operator-console/screens/OperatorSummaryScreen.tsx`
- existing operator diagnostics schema: `src/shared-contracts/schemas/operator-diagnostics.ts`
- existing operator recovery schema: `src/shared-contracts/schemas/operator-recovery.ts`
- host diagnostics projection: `src-tauri/src/diagnostics/mod.rs`
- readiness normalization: `src-tauri/src/capture/normalized_state.rs`
- 이전 스토리: `_bmad-output/implementation-artifacts/5-1-운영자용-현재-세션-문맥과-장애-진단-가시화.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/5-3-라이프사이클-개입-복구-감사-로그-기록.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: 운영자 복구와 감사 로그]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.4: 운영자용 카메라 연결 상태 전용 항목과 helper readiness 가시화]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-009 Operational Safety and Recovery]
- [Source: _bmad-output/planning-artifacts/prd.md#Named Policy References]
- [Source: _bmad-output/planning-artifacts/prd.md#Fault Diagnosis and Recovery]
- [Source: _bmad-output/planning-artifacts/prd.md#Approved Operator Recovery Inventory]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-001 Customer Guidance Density and Simplicity]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/implementation-artifacts/5-1-운영자용-현재-세션-문맥과-장애-진단-가시화.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md#구현-가드레일]
- [Source: _bmad-output/implementation-artifacts/5-3-라이프사이클-개입-복구-감사-로그-기록.md#구현-가드레일]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#앱-실행과-카메라-연결-확인-진입점]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-03-카메라-연결-후-ready-진입-확인]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-10-카메라-분리-후-재연결-복구-확인]
- [Source: docs/contracts/camera-helper-edsdk-profile.md]
- [Source: src/shared-contracts/schemas/operator-diagnostics.ts]
- [Source: src/shared-contracts/schemas/operator-recovery.ts]
- [Source: src-tauri/src/diagnostics/mod.rs]
- [Source: src-tauri/src/capture/normalized_state.rs]
