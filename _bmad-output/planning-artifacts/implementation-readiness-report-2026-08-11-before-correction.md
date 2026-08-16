---
stepsCompleted:
  - step-01-document-discovery
  - step-02-prd-analysis
  - step-03-epic-coverage-validation
  - step-04-ux-alignment
  - step-05-epic-quality-review
  - step-06-final-assessment
assessmentStatus: NEEDS_WORK
includedDocuments:
  prd: prd.md
  architecture: architecture.md
  epics: epics.md
  ux: ux-design-specification.md
excludedDocuments:
  - prd-validation-report-20260320-015539.md
  - implementation-readiness-report-20260320.md
---

# Implementation Readiness Assessment Report

**Date:** 2026-08-11
**Project:** Boothy

## 문서 인벤토리

### PRD

- 전체 문서: `prd.md` (69,103 bytes, 2026-08-11 01:35:51)
- 분할 문서: 없음

### 아키텍처

- 전체 문서: `architecture.md` (68,553 bytes, 2026-08-11 01:38:59)
- 분할 문서: 없음

### 에픽 및 스토리

- 전체 문서: `epics.md` (67,985 bytes, 2026-08-11 01:40:22)
- 분할 문서: 없음

### UX 설계

- 전체 문서: `ux-design-specification.md` (39,003 bytes, 2026-08-11 01:36:39)
- 분할 문서: 없음

### 평가 범위 결정

- 필수 문서 4종이 모두 확인되었으며 전체본/분할본 중복은 없습니다.
- `prd-validation-report-20260320-015539.md`와 `implementation-readiness-report-20260320.md`는 현재 FR-010·Epic 7 변경 검증의 입력에서 제외합니다.

## PRD 분석

### 기능 요구사항

#### FR-001 Simple Session Start

Users can start a booth session by entering a non-empty customer name and a four-digit phone suffix, which together form the customer-facing booth alias for the active session, as the only required booth-start input.

**Acceptance Criteria**

- The booth-start surface accepts exactly two required user-entered values: a non-empty customer name and a four-digit phone suffix.
- Invalid, empty, or malformed booth-alias input is shown before the customer proceeds.
- Valid input creates a customer-facing booth alias and an active session identity for the current booth session.
- The customer can continue into the preparing or ready flow without mandatory reservation verification or full phone-number entry.

#### FR-002 Approved Published Preset Catalog Selection

Users can choose one approved published preset from a bounded catalog before shooting begins.

**Acceptance Criteria**

- The booth presents only 1-6 approved published presets to the customer.
- Each preset includes a customer-facing name and one preview image or standardized preview tile.
- Each booth-consumable preset has a stable published identity and version under the approved preset catalog.
- The customer can activate one preset at session start and can revisit or change it later during the same session.
- The activated preset becomes the active preset for subsequent captures until the customer changes it.
- No direct photo-editing workspace, darktable terminology, or detailed image-adjustment controls are exposed in preset selection.

#### FR-003 Readiness Guidance and Valid-State Capture

Users can understand whether the booth is preparing, ready, preview-waiting, export-waiting, or phone-required and can capture only in approved valid states.

**Acceptance Criteria**

- The customer sees plain-language readiness guidance before first capture.
- The booth blocks capture when session or capture-state truth is not approved for capture.
- Customer-facing states avoid technical diagnostics and render-engine language.
- Blocked states tell the customer whether to wait or call rather than troubleshoot.
- The booth does not imply that preview readiness or final completion already exists when only capture readiness is true.

#### FR-004 Current-Session Capture Persistence and Truthful Preview Confidence

Users can capture photos into the active session and receive truthful current-session confidence while booth-safe preview feedback progresses from waiting to ready.

**Acceptance Criteria**

- A successful capture means the new source photo is associated with and safely persisted under the active session before booth success feedback is shown.
- The booth can distinguish capture acceptance from preview readiness in customer-safe language when preview preparation is still in progress.
- The booth may show a same-capture pending preview before `previewReady`, but it must not imply that the preset-applied booth-safe preview is already ready.
- The latest customer-visible confirmation includes only current-session assets.
- The active preset name remains visible on the capture surface and preview confirmation surface while that preset is active.

#### FR-005 Current-Session Review, Deletion, and In-Session Preset Change

Users can review only current-session photos, delete current-session captures according to the `Current-Session Deletion Policy`, and change the active preset for future captures.

**Acceptance Criteria**

- The review surface exposes current-session assets only.
- The customer can delete current-session captures and their correlated current-session booth artifacts only when the `Current-Session Deletion Policy` allows it.
- The customer can change the active preset at any point during the session.
- Each preset change applies to subsequent captures without rewriting already captured assets.
- The product does not expose a direct photo-editing workspace, detailed editing controls, or authoring tools as part of review.

#### FR-006 Coupon-Adjusted Timing, Warning Alerts, and Exact-End Behavior

Users can rely on customer session timing that follows the `Session Timing Policy` and presents state-appropriate guidance as session end approaches and arrives.

**Acceptance Criteria**

- The adjusted session end time is visible from the beginning of the active session.
- A sound-backed warning occurs 5 minutes before the adjusted end time.
- A sound-backed alert occurs at the adjusted end time.
- Customer guidance explicitly states whether shooting can continue or has ended.
- Updated timing behavior follows the active `Session Timing Policy` rather than a generic slot rule.

#### FR-007 Export Waiting, Final Readiness, and Handoff Guidance

Users can move through the end-of-session outcome after shooting ends with truthful final-readiness and handoff guidance.

**Acceptance Criteria**

- After shooting ends, the product enters one explicit post-end state from the post-end completion taxonomy: `Export Waiting`, `Completed`, or `Phone Required`.
- In `Export Waiting`, shooting is disabled and the customer sees wait guidance while the end-of-session deliverable or handoff package is not yet ready.
- In `Completed`, all booth-side required work is complete and the state resolves as either `Local Deliverable Ready` or `Handoff Ready`.
- In `Local Deliverable Ready`, the required booth-side deliverable is ready and the customer can leave the booth flow without additional booth-side processing.
- In `Handoff Ready`, the customer sees the identified recipient or next location together with the approved next action.
- The customer sees the next action without technical diagnostics.
- If the approved booth alias is required for downstream handoff, the product displays that booth alias on the handoff surface.
- If the session cannot resolve normally, the product routes to bounded wait or call guidance rather than false completion.

#### FR-008 Authorized Preset Authoring, Approval, and Publication

Authorized preset managers can author draft booth preset versions, validate booth compatibility, approve immutable published versions, publish them to the future-session booth catalog, and roll back to a prior approved version without exposing authoring tools to booth customers.

**Acceptance Criteria**

- Authorized users can create or tune draft preset versions in an approved internal workflow that is unavailable to booth customers.
- Draft preset versions move through explicit lifecycle states `draft`, `validated`, `approved`, and `published`.
- Each approved booth preset is published as an immutable preset artifact with stable identity and approved render compatibility for booth use.
- Publication creates an immutable published artifact bundle plus catalog-facing metadata for future sessions only.
- Presets require approval before appearing in the customer booth catalog.
- Booth sessions consume only approved published preset artifacts.
- Publication changes and rollback apply to future sessions and do not directly mutate active session data or active session preset bindings.
- Booth customers never receive access to preset-authoring controls through the customer flow, review flow, or completion flow.

#### FR-009 Operational Safety and Recovery

Operators can identify blocked states, protect customers from unsafe recovery steps, and use diagnostics, actions, and lifecycle visibility bounded by the `Operator Recovery Policy` across both capture and render boundaries.

**Acceptance Criteria**

- Customer-facing failure states use plain-language wait or call guidance.
- Operators can view current session context, timing state, recent failure context, and the allowed action set defined for the active blocked-state category by the `Operator Recovery Policy`.
- Operators can distinguish blocked capture states from blocked preview, render, or completion states using the diagnostics defined by the `Operator Recovery Policy`.
- Approved operator actions are limited to retry, approved boundary restart, allowed time extension under the `Session Timing Policy`, or recovery routing to `Phone Required`.
- Lifecycle, queue, and intervention events are recorded for support, timing, and completion analysis.

#### FR-010 Pre-Opened Full-View Progressive Preset Display

Users can see the current capture's selected look on a dedicated viewing display that is ready before capture and upgrades from the first qualifying display-fit preset image to the RAW-refined image without visual interruption.

**Acceptance Criteria**

- The read-only customer viewer is created, visible or otherwise explicitly approved as ready, layout-complete, and subscribed to the current session before capture is allowed.
- The viewer determines the required image size from the approved photo rectangle, monitor mode, and device-pixel ratio rather than using a fixed thumbnail size.
- The first qualifying customer-viewer frame is associated with the same session, request, capture, preset identity, and published version as the accepted capture.
- An unfiltered camera image, review-rail thumbnail, file-ready signal, renderer-ready signal, or image-load callback alone does not satisfy viewer success.
- A RAW-refined display replaces an earlier qualifying preset proxy only after complete decode and without a blank, spinner, prior capture, crop or scale jump, or quality-tier downgrade.
- Reload, listener loss, delayed work, and overlapping captures cannot cause an older generation or another session's asset to replace the active display.

**총 기능 요구사항: 10개**

### 비기능 요구사항

#### NFR-001 Customer Guidance Density and Simplicity

The system shall keep 100% of primary customer state screens within a copy budget of no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, booth alias, and local phone number, while exposing 0 internal diagnostic, authoring, or render-engine terms on customer-visible screens, as measured by release copy audit.

**Acceptance Criteria**

- All primary customer states pass copy audit before release.
- Each primary customer state contains no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, booth alias, and local phone number.
- Customer-visible wording uses approved booth-state terminology only.
- No customer state includes raw technical, filesystem, darktable, XMP, or direct editing-control labels.

#### NFR-002 Cross-Branch Preset and Timing Consistency

The system shall keep 100% of active branches on the same approved customer preset catalog, approved preset versions, customer-visible timing rules, and core booth journey states except approved local contact settings, as measured by branch rollout audit.

**Acceptance Criteria**

- Active branches use the same approved preset catalog, ordering, and published preset versions.
- Active branches use the same customer-visible timing rules and warning behavior.
- Branch variance is limited to approved local settings such as contact information and approved operational toggles.

#### NFR-003 Booth Responsiveness and Qualifying Viewer Readiness

The system shall acknowledge primary customer actions within 1 second and, on approved Windows hardware, present the first qualifying preset-applied image on the pre-opened customer viewer within a warm p50 of 3 seconds, warm p95 of 4 seconds, and hard maximum of 5 seconds from trusted capture input, as measured from the same monotonic clock against actual monitor presentation.

**Acceptance Criteria**

- Primary customer actions such as session start, preset selection, delete confirmation, and post-end state entry are acknowledged within 1 second.
- The official latency start is the trusted customer capture input and the official end is the first qualifying preset-applied monitor frame on the pre-opened viewer.
- File-ready, renderer-ready, event receipt, image decode, image-load callbacks, unfiltered camera images, and review-rail thumbnails remain diagnostic spans rather than the success endpoint.
- A warm 100-shot release run achieves at least 99% qualifying success, p50 at or below 3 seconds, p95 at or below 4 seconds, and hard maximum at or below 5 seconds without excluding failures or timeouts.
- Wrong-session, wrong-request, wrong-capture, wrong-preset, unfiltered qualifying, blank, stale, upscaled, crop-jump, scale-jump, and tier-downgrade frames remain at zero.
- Cold first-shot, 10-minute idle, and camera reconnect scenarios are reported separately and cannot silently reuse warm-only evidence.
- If no qualifying preset-applied image is ready, the booth remains in truthful waiting guidance rather than implying viewer success.
- Performance is measured on approved branch hardware.

#### NFR-004 Session Isolation and Privacy

The system shall expose 0 cross-session asset leaks across source capture, preview, final output, review, deletion, and completion flows, as measured by privacy test cases, pilot operation, and defect review.

**Acceptance Criteria**

- Customers cannot access another customer's assets through review, deletion, or handoff flows.
- Stored customer identifiers are limited to approved minimum session-identifying data.
- Release privacy validation passes active-session and reopened-session isolation scenarios across source, preview, and final assets.

#### NFR-005 Timing, Post-End, and Render Reliability

The system shall deliver the scheduled 5-minute warning and exact-end alert within +/- 5 seconds for 99% of qualifying sessions, transition 90% or more of sessions to an explicit post-end state within 10 seconds of scheduled end time, and preserve valid current-session assets through render retries or failures, as measured by lifecycle logs and pilot review.

**Acceptance Criteria**

- 99% of qualifying sessions receive the warning and exact-end alert within the allowed tolerance.
- 90% or more of sessions enter `Export Waiting`, `Completed`, or `Phone Required` within 10 seconds of scheduled end time.
- 90% or more of sessions resolve to `Completed` or `Phone Required` within 2 minutes of scheduled end time.
- Render retry or failure does not delete or invalidate already persisted valid current-session source captures.

#### NFR-006 Safe Local Packaging, Rollout, and Version Pinning

The system shall support staged branch rollout to explicitly selected branch sets and rollback of any promoted branch to the last approved build and approved preset stack within one approved rollback action, while preserving approved local settings and active-session compatibility, enforcing 0 forced updates during active customer sessions, and maintaining approved render compatibility across the active preset catalog, as measured by release controls and branch rollout audit.

**Acceptance Criteria**

- Each rollout targets an explicitly selected branch set rather than mandatory same-time deployment to every branch.
- 100% of rollout and rollback actions record the branch set, target build, approved preset stack, approval timestamp, and operator identity in the rollout audit.
- Active customer sessions are never interrupted by forced update behavior.
- Any promoted branch can return to the last approved build and approved preset stack in one approved rollback action while preserving approved local settings and active-session compatibility.
- The release artifact includes the signed app, camera helper, approved EDSDK runtime, display renderer or shader bundle, color profile, proxy recipe, and pinned RAW renderer dependency as one verifiable inventory.
- A clean offline Windows environment can install, launch, self-check, render an approved fixture to the viewer, upgrade, and uninstall without a separately installed development toolchain.

**총 비기능 요구사항: 6개**

### 추가 요구사항 및 제약

- 고객의 유일한 창작 제어는 승인된 프리셋 선택이며, 고객용 직접 편집기와 darktable/XMP/저수준 렌더링 용어는 MVP 범위에서 제외됩니다.
- 고객 카탈로그는 승인·게시된 불변 프리셋 아티팩트 1~6개만 노출하며, 활성 세션은 캡처 시점에 바인딩된 프리셋 정체성과 게시 버전을 유지합니다.
- `Current-Session Deletion Policy`는 활성 세션의 선택 캡처와 연관 파생물만 삭제할 수 있게 하고, 진행 중인 변경 상태 및 완료 확정 후 삭제를 금지합니다.
- `Session Timing Policy`는 시작 시 확정된 조정 종료 시각의 지속 노출, T-5분 경고, T=0 종료, 승인·기록된 운영자 연장 외 종료 후 신규 촬영 금지를 요구합니다.
- `Operator Recovery Policy`는 장애를 캡처·프리뷰/렌더·종료 후 범주로 정규화하며, 허용 조치를 재시도·승인된 경계 재시작·승인된 시간 연장·`Phone Required` 전환으로 제한합니다.
- 캡처 성공, 프리뷰 준비, 뷰어 표시 성공, 최종 완료는 서로 다른 제품 진실이며 하나의 성공 상태로 합칠 수 없습니다.
- 전용 뷰어는 촬영 전에 현재 세션을 구독하고 레이아웃과 모니터 프로필이 준비되어야 하며, 실제 표시 크기에 맞는 프리셋 적용 프레임만 성공으로 인정합니다.
- 프록시에서 RAW 정제 이미지로의 교체는 완전 디코드 후 수행하며 빈 화면·이전 캡처·크롭/스케일 점프·품질 하락을 허용하지 않습니다.
- Windows 승인 지점 하드웨어에서 로컬 우선 동작을 보존하고, 고객·뷰어·운영자·내부 프리셋 관리 권한 경계를 분리해야 합니다.
- 배포는 지점별 단계적 승격·한 번의 승인된 롤백·활성 세션 중 강제 업데이트 금지를 충족해야 하며, 오프라인 Windows에서 전체 설치·자가점검·렌더·업그레이드·제거가 재현되어야 합니다.
- FR-010 관련 릴리스 증거는 warm 100-shot, cold first-shot, 10분 idle, 카메라 재연결 시나리오를 구분하고 실제 모니터 표시를 동일 단조 시계로 측정해야 합니다.
- 제품 가정 중 FR-010과 직접 연결된 미검증 항목은 사전 준비된 뷰어의 무결성, EOS 700D의 빠른 소스 경로, 상주 렌더러의 지연·시각 동등성입니다.

### PRD 완전성 초기 평가

PRD는 FR 10개와 NFR 6개를 명시적으로 번호화하고 수용 기준, 정책 기준선, KPI, 릴리스 게이트까지 연결하고 있습니다. 새 FR-010은 NFR-003, 제품 상태 분리 결정, 100-shot 릴리스 게이트와 직접 연결되어 측정 가능성이 높습니다. 다만 문서 상태가 `draft`이고, FR-010의 핵심 성립 조건 세 가지가 Epic 7의 하드웨어·소스·렌더러 검증에 열린 가정으로 남아 있으므로 구현 준비 판정은 후속 에픽·UX·아키텍처 커버리지 검증 결과에 종속됩니다.

## 에픽 커버리지 검증

### 에픽 FR 커버리지 추출

- FR-001: Epic 1 / Story 1.2
- FR-002: Epic 1 / Story 1.3
- FR-003: Epic 1 / Story 1.4, Story 1.6
- FR-004: Epic 1 / Story 1.5, Story 1.7, Story 1.8 및 Epic 7의 뷰어 진실 경계
- FR-005: Epic 2 / Story 2.1, Story 2.2, Story 2.3
- FR-006: Epic 2 / Story 2.4
- FR-007: Epic 3 / Story 3.1, Story 3.2, Story 3.3
- FR-008: Epic 4 / Story 4.1, Story 4.2, Story 4.3, Story 4.4
- FR-009: Epic 5 / Story 5.1, Story 5.2, Story 5.3, Story 5.4
- FR-010: Epic 7 / Story 7.1~7.6. 핵심 고객 결과는 Story 7.1, 7.3, 7.5, 7.6에, 소스·렌더러 채택 근거는 Story 7.2와 7.4에 배치됨

**에픽에 명시된 총 FR: 10개**

### 커버리지 매트릭스

| FR | PRD 요구사항 | 에픽/스토리 커버리지 | 상태 |
| --- | --- | --- | --- |
| FR-001 | 이름과 네 자리 전화번호 뒷자리만으로 세션 시작 | Epic 1 / Story 1.2 | 커버됨 |
| FR-002 | 승인·게시된 1~6개 프리셋 중 하나 선택 | Epic 1 / Story 1.3 | 커버됨 |
| FR-003 | 준비 상태 안내와 유효 상태에서만 촬영 | Epic 1 / Story 1.4, 1.6 | 커버됨 |
| FR-004 | 활성 세션 캡처 영속화와 진실한 프리뷰 진행 | Epic 1 / Story 1.5, 1.7, 1.8 및 Epic 7 | 커버됨 |
| FR-005 | 현재 세션 검토·삭제·향후 촬영용 프리셋 변경 | Epic 2 / Story 2.1~2.3 | 커버됨 |
| FR-006 | 조정 종료 시각·5분 경고·정확한 종료 동작 | Epic 2 / Story 2.4 | 커버됨 |
| FR-007 | Export Waiting·Completed·Phone Required 종료 진실 | Epic 3 / Story 3.1~3.3 | 커버됨 |
| FR-008 | 내부 프리셋 작성·검증·승인·게시·롤백 | Epic 4 / Story 4.1~4.4 | 커버됨 |
| FR-009 | 정책으로 제한된 운영 진단·복구·감사 | Epic 5 / Story 5.1~5.4 | 커버됨 |
| FR-010 | 촬영 전 준비된 전용 뷰어의 프리셋 적용 표시와 무중단 RAW 정제 전환 | Epic 7 / Story 7.1~7.6 | 커버됨 |

### 누락 요구사항

- PRD에 있으나 에픽 문서에 없는 FR: 없음
- 에픽 문서에 있으나 PRD에 없는 FR: 없음
- FR-010은 Epic 7에 명시적으로 귀속되며 FR Coverage Map, Epic List, Story 7.1~7.6에서 일관되게 추적됩니다.

### 커버리지 통계

- PRD 전체 FR: 10개
- 에픽에 커버된 FR: 10개
- 누락 FR: 0개
- FR 커버리지: 100%

## UX 정합성 평가

### UX 문서 상태

- 상태: 확인됨
- 문서: `ux-design-specification.md`
- 최신 수정일: 2026-08-11
- 문서 상태: `approved-low-latency-viewer-alignment`

### PRD ↔ UX 정합성

- 고객 여정은 세션 시작, 승인 프리셋 선택, 촬영 준비, 캡처/프리뷰 대기, 현재 세션 검토, 시간 안내, 종료 후 상태로 PRD와 일치합니다.
- FR-010의 전용 고객 뷰어가 촬영 전에 준비되고 현재 세션·모니터 프로필·photo rectangle·DPR·listener·layout 상태를 증명해야 한다는 기준이 UX에 명시돼 있습니다.
- 첫 성공 프레임을 동일 session/request/capture/preset@version에 연결된 physical display-fit 프리셋 이미지로 제한하고, 무보정 카메라 이미지·작은 rail thumbnail·파일/렌더 준비 이벤트를 성공에서 제외해 PRD와 일치합니다.
- 프록시에서 RAW 정제 이미지로 전환할 때 완전 디코드, 기존 이미지 유지, 불투명 double-buffer 교체, blank·spinner·stale·crop/scale jump·tier downgrade 0건을 요구해 FR-010 및 NFR-003과 일치합니다.
- 고객 화면과 read-only 뷰어의 역할 분리, 고객 직접 편집 금지, `Preview Waiting`/`Phone Required` 보호 문구, 현재 세션 격리는 FR-003~FR-005 및 NFR-001/NFR-004와 일치합니다.
- UX가 PRD에 없는 신규 제품 기능을 추가한 사례는 발견되지 않았습니다. Tailwind/Headless UI와 시각 토큰은 UX 구현 가이드로 표시돼 있어 PRD 범위 확장으로 보지 않습니다.

### UX ↔ 아키텍처 정합성

- 아키텍처의 네 가지 capability-gated surface, app-lifetime `/viewer`, host-owned session truth, immutable display generation, actual-present telemetry가 UX의 화면·상태 요구를 직접 지원합니다.
- `viewer-surface`, `display-generation`, Rust `viewer`/`display` 경계와 하드웨어 테스트 위치가 FR-010 구현 경로로 매핑돼 있습니다.
- 촬영 전 뷰어 준비, physical display-size 계산, 동일 단조 시계 기반 실제 모니터 표시 측정, 완전 검증 후 manifest pointer 전진, stale generation 거부가 UX 요구와 일치합니다.
- darktable의 RAW 정제·최종·동등성 기준·정확 fallback 역할과 별도 display-fit proxy renderer의 증거 기반 승격 경계가 UX의 속도와 시각 진실 요구를 함께 지원합니다.
- 고객/운영자/프리셋 작성 권한 분리, 기술 용어 비노출, host-normalized state는 UX의 사용자별 정보 밀도와 안전 요구를 지원합니다.

### 정합성 이슈

1. **중요 — 아키텍처 자체 검증 문구에서 FR-010 누락**
   - 아키텍처 본문과 구조 매핑은 FR-010을 구체적으로 지원하지만, `Architecture Validation Results`의 Functional Requirements Coverage 문장은 여전히 “FR-001 through FR-009”만 언급합니다.
   - 영향: 후속 검토자가 아키텍처가 FR-010을 공식 검증하지 않았다고 해석할 수 있습니다.
   - 권고: 해당 문장을 FR-001~FR-010으로 갱신하고 Epic 7 뷰어 경계를 명시하십시오.

2. **중요 — 제외하기로 한 과거 PRD 검증 보고서가 아키텍처 frontmatter에 잔존**
   - `architecture.md`의 `inputDocuments`에 `prd-validation-report-20260320-015539.md`가 남아 있으나 본문 Source Inputs에는 없습니다.
   - 영향: 현재 FR-010 기준의 출처 계보가 불명확하고, 아키텍처의 “source-input hygiene reconciled” 자체 평가와 모순됩니다.
   - 권고: frontmatter에서 과거 검증 보고서를 제거해 최신 PRD·UX·연구 문서만 출처로 유지하십시오.

3. **보통 — UX 접근성 요구의 아키텍처 추적 부족**
   - UX는 WCAG 2.2 AA, semantic HTML, 명확한 focus 관리, modal focus restore 및 ESC 닫기를 요구하지만 아키텍처의 요구사항 매핑·테스트 경계에는 해당 항목이 명시적으로 연결되지 않습니다.
   - 영향: 기능 구현은 가능하지만 접근성 수용 기준이 스토리나 테스트에서 빠질 위험이 있습니다.
   - 권고: shared UI 규칙과 e2e/접근성 테스트 책임에 UX-DR16을 명시적으로 매핑하십시오.

### 경고

- UX와 아키텍처 모두 FR-010의 제품 경험을 충분히 정의하지만, fast source와 resident renderer의 실제 성립 가능성은 하드웨어 검증 전까지 중간 신뢰도입니다.
- `status: complete`인 아키텍처 안에 위 두 개의 최신화 결함이 있어, 문서상 완전성 선언은 수정 후 다시 확인하는 편이 안전합니다.

## 에픽 품질 검토

### 에픽별 준수 결과

| Epic | 사용자 가치 | 독립성/순서 | 스토리 크기 | 수용 기준 | FR 추적 | 결과 |
| --- | --- | --- | --- | --- | --- | --- |
| Epic 1 | 고객의 빠른 시작과 첫 촬영 | 후행 Story 1.6~1.8 의존 존재 | 일부 과대 | 대체로 명확 | FR-001~004 | 수정 필요 |
| Epic 2 | 현재 세션 검토·삭제·시간 안내 | 선행 산출물만 사용 | 적정 | 명확하고 테스트 가능 | FR-005~006 | 통과 |
| Epic 3 | 진실한 종료·인계·지원 안내 | 선행 산출물만 사용 | 적정 | 명확하고 테스트 가능 | FR-007 | 통과 |
| Epic 4 | 프리셋 작성·검증·게시·롤백 | 순차 의존이 앞 방향 | 적정 | 오류·거부 조건 포함 | FR-008 | 통과 |
| Epic 5 | 운영 진단·복구·감사 | 선행 런타임 위에 동작 | 적정 | 명확하고 테스트 가능 | FR-009 | 통과 |
| Epic 6 | 지점 배포·롤백·증거 기반 종료 | 선행 스토리만 참조 | 적정 | 하드웨어 No-Go 포함 | NFR-002/006 | 통과 |
| Epic 7 | 빠르고 진실한 전용 뷰어 경험 | 7.1→7.6 순서는 앞 방향 | 7.1·7.6 과대 | 명확하고 테스트 가능 | FR-004/010, NFR-003/004/006 | 수정 필요 |

### 중대 위반

1. **Epic 1에 금지된 전방 의존성이 존재**
   - Story 1.4는 실제 helper/camera readiness를 수용 기준으로 요구하지만 그 경계는 후행 Story 1.6에서 구현됩니다.
   - Story 1.5는 실제 캡처 영속화와 preview truth를 완료 조건으로 요구하지만 실제 round-trip은 Story 1.7, render-backed preview는 Story 1.8에서 구현됩니다.
   - Story 1.5는 문서 안에서 Story 1.7 이후 증거 소유권을 직접 언급해 독립적으로 완료할 수 없음을 확인합니다.
   - 조치: 1.6/1.7을 앞당기거나, 1.4/1.5를 상태 투영·고객 문구와 실제 하드웨어 통합 스토리로 분리해 각 스토리가 완료 가능한 단위를 갖게 하십시오.

2. **Story 7.6이 하나의 스토리 범위를 넘음**
   - signed inventory, clean offline 설치/실행/업그레이드/롤백/제거, 100-shot 성능 인증, cold/idle/reconnect/burst/failure 복구, canary 승격, 최종 Go 판정을 한 스토리에 결합합니다.
   - 영향: 병렬 소유권, 추정, 부분 완료, 실패 원인 격리가 어렵습니다.
   - 조치: 최소한 설치 재현성, 성능·복구 인증, staged rollout/rollback, 최종 release gate로 분리하고 Epic 7 완료 조건에서 다시 집계하십시오.

### 주요 이슈

1. **Story 7.1 과대 범위**
   - 전용 window lifecycle, 세 가지 display profile, physical sizing, immutable sample publication, opaque double-buffer, actual-present telemetry, visible/hidden prewarm A/B까지 포함합니다.
   - 권고: viewer readiness/display contract와 present telemetry/double-buffer 검증을 독립 스토리로 분리하십시오.

2. **Story 7.2와 Story 7.4가 사용자 스토리가 아닌 기술 실험**
   - 두 스토리는 `As a booth product team` 관점의 source comparison과 resident-renderer spike입니다.
   - 필요한 증거 작업이지만 고객 결과 스토리와 동일한 형식으로 두면 완료 가치가 혼동됩니다.
   - 권고: 명시적 enabler/spike로 표시하고 산출물, timebox, Go/No-Go, 후속 채택 조건을 별도 규칙으로 관리하십시오.

3. **Correct-course 전환 호환성의 전용 수용 기준이 약함**
   - Epic 7은 기존 canonical preview 경로에서 immutable generation 모델로 전환하지만, 기존/진행 중 세션 manifest와 rollback 시 스키마 호환성을 직접 검증하는 수용 기준이 분산돼 있습니다.
   - 권고: 업그레이드 전 세션, 활성 세션, 구버전 rollback, 새 generation pointer를 한 묶음으로 검증하는 호환성 시나리오를 Story 7.6 분할본 또는 별도 스토리에 추가하십시오.

### 경미한 우려

- Story 1.1은 필수 starter-template 스토리로 존재하고 승인된 Vite/Tauri 기반과 surface gating을 다루지만, 의존성 설치 검증·개발 환경 재현·초기 CI 성공 조건은 명시적이지 않습니다.
- 여러 스토리가 구현, 문서 수정, 자동화, 실제 하드웨어 증거를 동시에 완료 조건으로 사용합니다. 제품 진실에는 적합하지만 한 스프린트 추정 시 개발 완료와 제품 검증 상태를 분리해 추적해야 합니다.

### 통과한 품질 항목

- 모든 FR은 에픽과 스토리에 추적됩니다.
- Epic 2~6은 사용자 또는 운영 주체의 결과를 중심으로 구성됐으며 후행 에픽 의존성이 발견되지 않았습니다.
- 대부분의 스토리는 Given/When/Then 형식으로 정상·오류·거부 조건을 테스트 가능하게 제시합니다.
- 전체 데이터베이스를 선행 구축하는 안티패턴은 없습니다. 세션 파일이 제품 진실이며 SQLite는 파생 인덱스로 유예돼 있습니다.
- 아키텍처가 지정한 starter template은 Story 1.1에 반영돼 있습니다.
- Story 7.6의 warm p50 ≤ 3초, p95 ≤ 4초, hard max ≤ 5초 및 99% 성공 기준은 PRD NFR-003과 정확히 일치합니다.

## 요약 및 권고

### 전체 구현 준비 상태

**NEEDS WORK**

- **Story 7.1 착수:** 가능. 촬영 전 뷰어, physical display-size contract, actual-present 측정, immutable sample double-buffer 경계는 PRD·UX·아키텍처·에픽에 일치하게 정의돼 있습니다.
- **Epic 7 전체 구현 계획:** 수정 후 진행. 성능 기준은 정확하지만 일부 스토리 크기와 기존 Epic 1의 전방 의존성이 기준을 위반합니다.
- **신규 MVP 출시:** NOT READY. 문서 자체의 정의대로 Story 7.6/HV-18의 실제 증거 통과 전에는 No-Go입니다.

### 즉시 해결해야 할 중대 이슈

1. Story 1.4→1.6 및 Story 1.5→1.7/1.8 전방 의존성을 제거해 각 스토리가 독립적으로 완료될 수 있게 재구성합니다.
2. Story 7.6을 설치 재현성, 성능·복구 인증, staged rollout/rollback, 최종 Go 판정으로 분할합니다.

### 권장 다음 단계

1. `architecture.md` frontmatter에서 과거 `prd-validation-report-20260320-015539.md`를 제거하고, 자체 검증 문구를 FR-001~FR-010 커버리지로 갱신합니다.
2. Story 1.4/1.5의 선행 관계를 교정하고 Story 7.1/7.6을 구현·검증 책임에 맞게 분할합니다.
3. Story 7.2/7.4를 명시적 evidence-gated enabler/spike로 분류합니다.
4. UX-DR16 접근성과 correct-course 업그레이드/활성 세션/rollback 호환성 수용 기준을 스토리 및 테스트 책임에 추가합니다.
5. 수정 후 Check Implementation Readiness를 다시 실행해 FR-010·Epic 7 기준의 최종 Go/No-Go 문서 상태를 재검증합니다.

### 최종 메모

이번 평가는 출처 위생·정합성·스토리 구조·출시 호환성의 4개 범주에서 총 10개 이슈를 확인했습니다. FR 커버리지는 100%이고 Story 7.1의 시작 경계는 충분하지만, 중대 이슈를 해결하기 전에는 Epic 7 전체 계획이나 신규 MVP 출시를 준비 완료로 간주할 수 없습니다.

**평가일:** 2026-08-11  
**평가자:** Codex — BMad Implementation Readiness Workflow
