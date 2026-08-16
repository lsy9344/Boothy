---
stepsCompleted:
  - step-01-document-discovery
  - step-02-prd-analysis
  - step-03-epic-coverage-validation
  - step-04-ux-alignment
  - step-05-epic-quality-review
  - step-06-final-assessment
overallReadinessStatus: NEEDS_WORK
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/sprint-change-proposal-20260811-021653.md
  - _bmad-output/implementation-artifacts/7-1-전용-관람-창-준비와-화면-크기-계약.md
---

# Implementation Readiness Assessment Report

**Date:** 2026-08-11
**Project:** Boothy

## 1. 문서 탐색 및 평가 범위

### 핵심 계획 문서

- PRD: `prd.md` (69,575 bytes, 2026-08-11 02:24:27)
- 아키텍처: `architecture.md` (70,323 bytes, 2026-08-11 02:28:20)
- 에픽 및 스토리: `epics.md` (73,887 bytes, 2026-08-11 02:23:38)
- UX 설계: `ux-design-specification.md` (39,541 bytes, 2026-08-11 02:24:35)

### 보조 및 변경 기준 문서

- 확정 변경안: `sprint-change-proposal-20260811-021653.md`
- 최신 Story 7.1: `7-1-전용-관람-창-준비와-화면-크기-계약.md`
- 검색 중 발견됐지만 승인 변경안에 따라 평가에서 제외: `prd-validation-report-20260320-015539.md`

### 버전 및 범위 결정

- 필수 네 문서는 모두 단일 문서 형식이며 분할본과의 중복은 없다.
- 변경 등급은 Moderate로 확정한다.
- Epic 1 전방 의존성 정리와 Epic 7의 10개 Story 재구성을 최신 기준으로 사용한다.
- 교체된 Story 7.1은 축소된 범위의 ready-for-dev 문서를 사용하고, 분리 범위는 Story 7.2에 보존된 것으로 간주한다.
- Story 7.10/HV-18D 완료 전까지 MVP 출시는 No-Go로 평가한다.
- 다음 실행 기준은 Story 7.1 구현과 HV-13A 증거 수집이다.

### 탐색 결과

- 필수 문서 누락 없음
- 전체본과 분할본의 중복 없음
- 사용자가 평가 대상과 최신 변경 기준을 확인함

## 2. PRD 분석

### 기능 요구사항

#### FR-001 Simple Session Start

Users can start a booth session by entering a non-empty customer name and a four-digit phone suffix, which together form the customer-facing booth alias for the active session, as the only required booth-start input.

Acceptance Criteria:

- The booth-start surface accepts exactly two required user-entered values: a non-empty customer name and a four-digit phone suffix.
- Invalid, empty, or malformed booth-alias input is shown before the customer proceeds.
- Valid input creates a customer-facing booth alias and an active session identity for the current booth session.
- The customer can continue into the preparing or ready flow without mandatory reservation verification or full phone-number entry.

#### FR-002 Approved Published Preset Catalog Selection

Users can choose one approved published preset from a bounded catalog before shooting begins.

Acceptance Criteria:

- The booth presents only 1-6 approved published presets to the customer.
- Each preset includes a customer-facing name and one preview image or standardized preview tile.
- Each booth-consumable preset has a stable published identity and version under the approved preset catalog.
- The customer can activate one preset at session start and can revisit or change it later during the same session.
- The activated preset becomes the active preset for subsequent captures until the customer changes it.
- No direct photo-editing workspace, darktable terminology, or detailed image-adjustment controls are exposed in preset selection.

#### FR-003 Readiness Guidance and Valid-State Capture

Users can understand whether the booth is preparing, ready, preview-waiting, export-waiting, or phone-required and can capture only in approved valid states.

Acceptance Criteria:

- The customer sees plain-language readiness guidance before first capture.
- The booth blocks capture when session or capture-state truth is not approved for capture.
- Customer-facing states avoid technical diagnostics and render-engine language.
- Blocked states tell the customer whether to wait or call rather than troubleshoot.
- The booth does not imply that preview readiness or final completion already exists when only capture readiness is true.

#### FR-004 Current-Session Capture Persistence and Truthful Preview Confidence

Users can capture photos into the active session and receive truthful current-session confidence while booth-safe preview feedback progresses from waiting to ready.

Acceptance Criteria:

- A successful capture means the new source photo is associated with and safely persisted under the active session before booth success feedback is shown.
- The booth can distinguish capture acceptance from preview readiness in customer-safe language when preview preparation is still in progress.
- The booth may show a same-capture pending preview before `previewReady`, but it must not imply that the preset-applied booth-safe preview is already ready.
- The latest customer-visible confirmation includes only current-session assets.
- The active preset name remains visible on the capture surface and preview confirmation surface while that preset is active.

#### FR-005 Current-Session Review, Deletion, and In-Session Preset Change

Users can review only current-session photos, delete current-session captures according to the `Current-Session Deletion Policy`, and change the active preset for future captures.

Acceptance Criteria:

- The review surface exposes current-session assets only.
- The customer can delete current-session captures and their correlated current-session booth artifacts only when the `Current-Session Deletion Policy` allows it.
- The customer can change the active preset at any point during the session.
- Each preset change applies to subsequent captures without rewriting already captured assets.
- The product does not expose a direct photo-editing workspace, detailed editing controls, or authoring tools as part of review.

#### FR-006 Coupon-Adjusted Timing, Warning Alerts, and Exact-End Behavior

Users can rely on customer session timing that follows the `Session Timing Policy` and presents state-appropriate guidance as session end approaches and arrives.

Acceptance Criteria:

- The adjusted session end time is visible from the beginning of the active session.
- A sound-backed warning occurs 5 minutes before the adjusted end time.
- A sound-backed alert occurs at the adjusted end time.
- Customer guidance explicitly states whether shooting can continue or has ended.
- Updated timing behavior follows the active `Session Timing Policy` rather than a generic slot rule.

#### FR-007 Export Waiting, Final Readiness, and Handoff Guidance

Users can move through the end-of-session outcome after shooting ends with truthful final-readiness and handoff guidance.

Acceptance Criteria:

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

Acceptance Criteria:

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

Acceptance Criteria:

- Customer-facing failure states use plain-language wait or call guidance.
- Operators can view current session context, timing state, recent failure context, and the allowed action set defined for the active blocked-state category by the `Operator Recovery Policy`.
- Operators can distinguish blocked capture states from blocked preview, render, or completion states using the diagnostics defined by the `Operator Recovery Policy`.
- Approved operator actions are limited to retry, approved boundary restart, allowed time extension under the `Session Timing Policy`, or recovery routing to `Phone Required`.
- Lifecycle, queue, and intervention events are recorded for support, timing, and completion analysis.

#### FR-010 Pre-Opened Full-View Progressive Preset Display

Users can see the current capture's selected look on a dedicated viewing display that is ready before capture and upgrades from the first qualifying display-fit preset image to the RAW-refined image without visual interruption.

Acceptance Criteria:

- The read-only customer viewer is created, visible or otherwise explicitly approved as ready, layout-complete, and subscribed to the current session before capture is allowed.
- The viewer determines the required image size from the approved photo rectangle, monitor mode, and device-pixel ratio rather than using a fixed thumbnail size.
- The first qualifying customer-viewer frame is associated with the same session, request, capture, preset identity, and published version as the accepted capture.
- An unfiltered camera image, review-rail thumbnail, file-ready signal, renderer-ready signal, or image-load callback alone does not satisfy viewer success.
- A RAW-refined display replaces an earlier qualifying preset proxy only after complete decode and without a blank, spinner, prior capture, crop or scale jump, or quality-tier downgrade.
- Reload, listener loss, delayed work, and overlapping captures cannot cause an older generation or another session's asset to replace the active display.

**총 FR: 10개**

### 비기능 요구사항

#### NFR-001 Customer Guidance Density and Simplicity

The system shall keep 100% of primary customer state screens within a copy budget of no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, booth alias, and local phone number, while exposing 0 internal diagnostic, authoring, or render-engine terms on customer-visible screens, as measured by release copy audit.

Acceptance Criteria:

- All primary customer states pass copy audit before release.
- Each primary customer state contains no more than one primary instruction sentence, one supporting sentence, and one primary action label, excluding dynamic session values such as time, progress percentage, booth alias, and local phone number.
- Customer-visible wording uses approved booth-state terminology only.
- No customer state includes raw technical, filesystem, darktable, XMP, or direct editing-control labels.

#### NFR-002 Cross-Branch Preset and Timing Consistency

The system shall keep 100% of active branches on the same approved customer preset catalog, approved preset versions, customer-visible timing rules, and core booth journey states except approved local contact settings, as measured by branch rollout audit.

Acceptance Criteria:

- Active branches use the same approved preset catalog, ordering, and published preset versions.
- Active branches use the same customer-visible timing rules and warning behavior.
- Branch variance is limited to approved local settings such as contact information and approved operational toggles.

#### NFR-003 Booth Responsiveness and Qualifying Viewer Readiness

The system shall acknowledge primary customer actions within 1 second and, on approved Windows hardware, present the first qualifying preset-applied image on the pre-opened customer viewer within a warm p50 of 3 seconds, warm p95 of 4 seconds, and hard maximum of 5 seconds from trusted capture input, as measured from the same monotonic clock against actual monitor presentation.

Acceptance Criteria:

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

Acceptance Criteria:

- Customers cannot access another customer's assets through review, deletion, or handoff flows.
- Stored customer identifiers are limited to approved minimum session-identifying data.
- Release privacy validation passes active-session and reopened-session isolation scenarios across source, preview, and final assets.

#### NFR-005 Timing, Post-End, and Render Reliability

The system shall deliver the scheduled 5-minute warning and exact-end alert within +/- 5 seconds for 99% of qualifying sessions, transition 90% or more of sessions to an explicit post-end state within 10 seconds of scheduled end time, and preserve valid current-session assets through render retries or failures, as measured by lifecycle logs and pilot review.

Acceptance Criteria:

- 99% of qualifying sessions receive the warning and exact-end alert within the allowed tolerance.
- 90% or more of sessions enter `Export Waiting`, `Completed`, or `Phone Required` within 10 seconds of scheduled end time.
- 90% or more of sessions resolve to `Completed` or `Phone Required` within 2 minutes of scheduled end time.
- Render retry or failure does not delete or invalidate already persisted valid current-session source captures.

#### NFR-006 Safe Local Packaging, Rollout, and Version Pinning

The system shall support staged branch rollout to explicitly selected branch sets and rollback of any promoted branch to the last approved build and approved preset stack within one approved rollback action, while preserving approved local settings and active-session compatibility, enforcing 0 forced updates during active customer sessions, and maintaining approved render compatibility across the active preset catalog, as measured by release controls and branch rollout audit.

Acceptance Criteria:

- Each rollout targets an explicitly selected branch set rather than mandatory same-time deployment to every branch.
- 100% of rollout and rollback actions record the branch set, target build, approved preset stack, approval timestamp, and operator identity in the rollout audit.
- Active customer sessions are never interrupted by forced update behavior.
- Any promoted branch can return to the last approved build and approved preset stack in one approved rollback action while preserving approved local settings and active-session compatibility.
- The release artifact includes the signed app, camera helper, approved EDSDK runtime, display renderer or shader bundle, color profile, proxy recipe, and pinned RAW renderer dependency as one verifiable inventory.
- A clean offline Windows environment can install, launch, self-check, render an approved fixture to the viewer, upgrade, and uninstall without a separately installed development toolchain.

**총 NFR: 6개**

### 추가 요구사항 및 제약

#### 바인딩 정책 기준

- `Current-Session Deletion Policy`: 활성 세션 소유 캡처와 연관 산출물만 삭제할 수 있고, 해당 소스·프리뷰·최종 파생물·manifest 참조만 제거한다. 활성 host-owned mutation 중이거나 post-end completion 확정 후에는 삭제를 차단한다.
- `Session Timing Policy`: 세션 시작 시 승인된 입력으로 조정 종료 시각을 확정하고 전 과정에 표시한다. T-5분 경고와 T=0 종료 동작을 적용하며, 승인·기록된 운영자 연장 없이는 종료 후 신규 촬영을 차단한다.
- `Operator Recovery Policy`: 차단 상태를 capture, preview/render, post-end 범주로 정규화한다. 운영자 조치는 retry, 승인된 boundary restart, 승인된 time extension, `Phone Required` 전환으로 제한하며 고객 화면에는 진단·장치 제어·내부 복구 절차를 노출하지 않는다.

#### 제품 및 플랫폼 제약

- 고객의 유일한 창작 제어는 승인된 프리셋 선택이며 직접 편집기는 MVP 범위 밖이다.
- 부스 런타임은 승인·게시된 immutable preset artifact만 사용한다.
- capture truth, preview truth, viewer-present truth, final completion truth를 분리한다.
- 고객 리뷰·삭제·인계는 현재 세션 자산으로만 제한하며 교차 세션 유출 허용치는 0이다.
- 고객용 Windows 데스크톱 부스 앱은 승인된 지점 하드웨어에서 local-first로 동작해야 한다.
- 고객과 운영자 surface의 진실·권한 수준을 분리한다.
- 활성 세션 중 강제 업데이트를 금지하고 staged rollout 및 rollback을 지원한다.
- 프리셋 카탈로그, 타이밍 규칙, 핵심 고객 여정의 지점별 편차를 승인된 로컬 설정 이외에는 허용하지 않는다.
- 예약 검증은 추후 추가되더라도 유효한 MVP 입력의 기본 부스 시작을 막지 않는다.
- 쿠폰·운영 정책 조회, 원격 지원 도구, preset authoring/render-engine 관리는 외부 운영 경계로 유지한다.

#### MVP 제외 범위

- 최종 사용자용 전체 편집기와 고객 직접 사진 편집 흐름
- 고객용 darktable UI, XMP 편집, style/library 관리
- darktable style/library state 또는 darktable 자체를 부스 런타임·카메라 제어의 truth source로 사용하는 방식
- customer-side capture-to-editor 연속성을 핵심 여정으로 만드는 방식
- 활성 세션을 직접 변경하는 authoring/publication
- 범용 클라우드 사진 라이브러리와 기기 간 편집 동기화
- 승인되지 않은 지점별 프리셋 카탈로그
- 활성 고객 세션 중 강제 업데이트

#### 열려 있는 검증 가정

- 이름+전화번호 뒤 4자리 alias가 처리량과 식별 모호성 측면에서 충분한가.
- 1-6개 프리셋 범위가 단순성과 고객 매력 사이의 균형을 만족하는가.
- 게시된 preset artifact가 지점 간 외형 일관성을 유지할 수 있는가.
- 승인 하드웨어가 승인 render path의 preview/viewer latency 목표를 지속적으로 달성하는가.
- 사전 개방 viewer가 blank/stale frame 없이 physical display-fit 이미지를 유지·표시하는가.
- LibRaw embedded JPEG 또는 capability-gated RAW+JPEG가 EOS 700D의 안정적인 fast source가 될 수 있는가.
- resident display renderer가 latency 및 visual-parity gate를 동시에 만족하는가.
- final render 또는 handoff completion의 독립 해결이 고객 혼란을 만들지 않는가.
- 운영자가 제한된 진단만으로 capture-state와 render-state 실패를 구분할 수 있는가.
- preset publication/rollback이 활성 세션을 변경하지 않고 미래 세션에만 적용되는가.

#### 필수 출시 게이트

- name+phone-last-four만으로 세션을 시작하고 승인 프리셋 선택 후 운영자 도움 없이 valid capture state에 도달한다.
- 성공 피드백 전에 현재 세션에 source photo가 저장된다.
- live camera/helper truth와 fresh status로만 `Ready`를 인정한다.
- preview waiting과 readiness를 진실하게 구분한다.
- 촬영 전에 전용 viewer가 승인 monitor profile과 현재 세션에 대해 준비된다.
- 최초 qualifying frame은 preset-applied, physical display-fit이며 accepted capture와 capture-bound preset version에 연결된다.
- warm 100-shot 증거가 p50/p95/hard-max/reliability 및 zero-wrong-frame gate를 만족하고 cold/idle/reconnect 결과는 별도 보고한다.
- proxy-to-RAW 교체에서 blank, stale, prior-capture, crop/scale jump, tier downgrade가 0이다.
- current-session-only review/deletion과 future-capture-only preset change를 보장한다.
- 조정 종료 시각, T-5분 경고, 정확한 종료 알림과 post-end taxonomy를 보장한다.
- render failure가 capture failure로 가장하거나 current-session truth를 훼손하지 않는다.
- 고객에게 편집기와 darktable/XMP/module/style/library 용어를 노출하지 않는다.
- publication/rollback 및 branch rollout/rollback이 active session을 변경·중단하지 않는다.
- clean offline Windows 환경에서 signed complete installer로 전체 승인 경로를 재현한다.
- 운영자 surface는 `Operator Recovery Policy`의 진단과 조치로만 제한한다.
- Story 7.10과 HV-18D는 installer, performance/recovery, rollout/rollback 증거가 각각 완성된 후에만 최종 MVP Go/No-Go를 기록한다.

### PRD 완전성 초기 평가

- FR 10개와 NFR 6개가 고유 식별자, 명시적 수용 기준, 출처 링크를 갖고 있다.
- 성능·신뢰성·프라이버시·배포 요구사항은 수치형 검증 기준을 포함한다.
- 고객·운영자·preset manager 경계와 MVP 제외 범위가 명확하다.
- Story 7.10/HV-18D 이전 출시 No-Go가 PRD release gate에 명시되어 있다.
- 다만 frontmatter 상태가 `draft-v1.3-low-latency-viewer-alignment`이며, 열린 가정 10건은 후속 산출물과 실제 증거를 통해 닫혀야 한다.

## 3. 에픽 FR 커버리지 검증

### 커버리지 매트릭스

| PRD FR | 요구사항 | 에픽·스토리 구현 경로 | 상태 |
| --- | --- | --- | --- |
| FR-001 | 이름+전화번호 뒤 4자리 기반 단순 세션 시작 | Epic 1 / Story 1.2 | Covered |
| FR-002 | 승인된 게시 프리셋 카탈로그 선택 | Epic 1 / Story 1.3; 세션 중 변경은 Story 2.3 | Covered |
| FR-003 | 준비 상태 안내와 유효 상태에서만 촬영 | Epic 1 / Story 1.4, 실장비 readiness truth는 Story 1.6 | Covered |
| FR-004 | 현재 세션 촬영 저장과 truthful preview confidence | Epic 1 / Story 1.5, 1.7, 1.8; 전용 viewer truth는 Epic 7 / Story 7.1-7.6 | Covered |
| FR-005 | 현재 세션 검토·삭제·향후 촬영용 프리셋 변경 | Epic 2 / Story 2.1, 2.2, 2.3 | Covered |
| FR-006 | 조정 종료 시각·5분 경고·정확한 종료 동작 | Epic 2 / Story 2.4 | Covered |
| FR-007 | Export Waiting·Completed·Phone Required 및 인계 안내 | Epic 3 / Story 3.1, 3.2, 3.3 | Covered |
| FR-008 | 내부 프리셋 작성·검증·승인·게시·롤백 | Epic 4 / Story 4.1, 4.2, 4.3, 4.4 | Covered |
| FR-009 | 제한된 운영자 진단·복구·감사 로그 | Epic 5 / Story 5.1, 5.2, 5.3, 5.4 | Covered |
| FR-010 | 촬영 전 전용 viewer와 무중단 progressive preset display | Epic 7 / Story 7.1-7.6; packaging·성능·rollout·최종 판정은 Story 7.7-7.10 | Covered |

### 누락 요구사항

- PRD에 있으나 에픽 문서에서 구현 경로를 찾지 못한 FR: 없음
- 에픽 커버리지 맵에 있으나 PRD에 존재하지 않는 FR: 없음
- 표기 체계는 PRD의 `FR-001` 형식과 에픽의 `FR1` 형식이 다르지만 번호별 의미와 범위는 일치한다.

### 커버리지 통계

- 전체 PRD FR: 10개
- 에픽·스토리에서 커버된 FR: 10개
- 미커버 FR: 0개
- FR 커버리지: 100%

### 커버리지 판정

- 모든 FR에 명시적인 에픽과 스토리 구현 경로가 있다.
- FR-004와 FR-010은 Epic 1의 capture/render truth와 Epic 7의 viewer-present truth가 분리되어 있어 변경안의 범위 분해와 일치한다.
- 이 단계에서는 스토리의 독립성, 수용 기준 품질, 선후 의존성의 타당성은 평가하지 않았다.

## 4. UX 정렬 평가

### UX 문서 상태

- 문서: `ux-design-specification.md`
- 상태: `approved-low-latency-viewer-alignment`
- 최종 수정일: 2026-08-11
- PRD 및 아키텍처를 입력 문서로 명시하고 있으며, 구현 가이드와 구속되는 UX 요구사항을 구분한다.

### UX ↔ PRD 정렬

정렬된 핵심 항목:

- booth-first, preset-driven 경험과 고객 직접 편집 금지
- 이름+전화번호 뒤 4자리 기반 세션 시작
- 승인된 1-6개 프리셋 선택과 현재 활성 프리셋 인지
- capture persistence, `Preview Waiting`, viewer-present, final completion의 진실 분리
- current-session-only 검토·삭제와 미래 촬영에만 적용되는 프리셋 변경
- 조정 종료 시각, T-5분 사운드 경고, 정확한 종료와 post-end taxonomy
- `Phone Required`의 고객 보호형 단일 행동 안내
- 촬영 전 전용 viewer 준비, physical display-fit qualifying frame, RAW 정밀본 무중단 교체
- 고객용 기술 진단어 및 내부 authoring 용어 비노출

PRD 직접 번호 밖의 UX 요구사항:

- WCAG 2.2 AA, semantic HTML, focus trap, ESC 닫기 및 focus restoration은 UX-DR16으로 명시되지만 PRD의 FR/NFR 번호 체계에는 직접 포함되지 않는다.
- 1024px+ booth main, 768-1023px operator view, 승인 monitor profile별 viewer fit은 UX-DR15/18에 구체화되어 있으나 PRD에는 더 상위 수준의 desktop/booth 제약으로만 표현된다.
- 이 항목들은 PRD와 충돌하지 않으며 UX가 구체화한 추가 구현 요구로 취급할 수 있지만, 릴리스 추적성에서는 별도 ID를 유지해야 한다.

### UX ↔ 아키텍처 정렬

아키텍처가 명시적으로 지원하는 항목:

- `/booth`, app-lifetime `/viewer`, `/operator`, `/authoring`의 surface 분리와 capability gating
- host-owned session truth를 공유하는 조작 화면과 read-only viewer
- current session, monitor profile, physical photo rectangle, DPR, listener/layout readiness 및 viewer epoch 계약
- immutable display generation, snapshot reconciliation, stale generation 거부와 opaque higher-tier 교체
- trusted capture input부터 actual monitor present까지의 단일 monotonic clock 측정
- 고객용 저밀도 문구와 운영자 진단 projection 분리
- timing alert, completion/handoff, operator recovery의 독립 도메인 소유권
- UX-DR16을 `shared-ui`, `booth-shell`, `viewer-surface`, `tests/e2e/accessibility.spec.ts`로 직접 매핑

### 정렬 이슈 및 경고

1. **Viewer readiness 선행 시간 모호성 — Moderate**
   - UX Testing Strategy는 촬영 최소 1초 전 viewer readiness를 테스트한다고 적는다.
   - PRD, 아키텍처, Story 7.1의 구속 계약은 모두 “촬영 전 준비 완료”만 요구하며 고정 1초 선행 시간을 요구하지 않는다.
   - UX의 1초가 설계 가이드인지 릴리스 게이트인지 명시하고, 게이트라면 PRD NFR/Story 7.1/HV-13A에 동일하게 반영해야 한다.

2. **접근성 요구의 PRD 번호 추적성 부족 — Minor**
   - 아키텍처에는 UX-DR16 지원 경로가 있지만 PRD에는 접근성 전용 FR/NFR이 없다.
   - 구현은 가능하나 최종 릴리스 매트릭스에서 UX-DR16을 독립 추적 항목으로 유지해야 한다.

3. **실부스 사용성 증거 소유권이 약함 — Minor**
   - UX는 실제 터치 반응성, standing usability, 고대비 시뮬레이션, 무가이드 촬영 성공률 검증을 요구한다.
   - 아키텍처는 UI 및 접근성 테스트 위치를 제공하지만 이 네 가지 제품 사용성 증거의 구체적인 release owner/gate는 명시하지 않는다.
   - QA/Release 단계에서 별도 UX acceptance evidence 묶음과 책임자를 지정하는 것이 필요하다.

### UX 정렬 판정

- 핵심 제품 흐름 및 전용 viewer 방향: 정렬됨
- 아키텍처 지원성: 충분함
- 구현 착수 차단 이슈: 없음
- 정리 필요 사항: viewer readiness의 1초 문구 확정, UX-DR16 및 실부스 사용성 증거의 릴리스 추적성 보강

## 5. Epic 및 Story 품질 검토

### 전체 구조 판정

| Epic | 사용자 가치 | 독립성·순서 | Story 품질 | 판정 |
| --- | --- | --- | --- | --- |
| Epic 1 | 빠른 시작과 신뢰 가능한 첫 촬영 | 후속 Epic 의존 없음; 1.4/1.5의 출시 truth 소유권 분리됨 | 일부 계약 충돌 잔존 | 보완 필요 |
| Epic 2 | 현재 세션 검토·삭제·프리셋 변경·시간 안내 | Epic 1 결과만 사용 | 대체로 명확하고 테스트 가능 | 적합 |
| Epic 3 | 진실한 종료·완료·인계 | 앞선 session/render truth만 사용 | false-complete gate 포함 | 적합 |
| Epic 4 | 승인 프리셋 작성·게시·롤백 | 독립된 내부 사용자 가치 | 인증 전제와 rollback 실패 AC 보완 필요 | 보완 필요 |
| Epic 5 | 운영자 진단·복구·감사 | 앞선 runtime truth만 사용 | 인증 소유권과 복구 실패 AC 보완 필요 | 보완 필요 |
| Epic 6 | 지점 배포·롤백 안전성 | 후속 Epic 의존 없음 | Story 6.1 과대 범위, Story 6.2 gate 불일치 | 보완 필요 |
| Epic 7 | 저지연 관람 경험의 검증과 출시 | 7.1→7.10 evidence dependency가 모두 순방향 | 7.1은 적합; 일부 기술·gate Story 분류와 크기 보완 필요 | 조건부 적합 |

### Critical 위반

- 현재 Story 7.1 착수를 직접 차단하는 Critical 위반은 발견되지 않았다.
- Epic 간 순환 의존 또는 후속 Epic을 선행 조건으로 요구하는 구조는 발견되지 않았다.

### Major 이슈

#### M1. 감사 저장 기준이 계획 문서 내부에서 충돌함

- `epics.md`의 Additional Requirements는 SQLite가 lifecycle·intervention·publication·rollout 감사 로그를 저장한다고 명시한다.
- 최신 아키텍처는 MVP에서 versioned JSON/JSONL journal이 로그와 증거의 truth이며 SQLite는 선택적 derived query index로 연기됐다고 명시한다.
- Story 5.3과 Story 6.1 구현자가 서로 다른 영속화 방식을 선택할 수 있다.

권고: Epics의 SQLite 요구를 최신 journal 기준으로 수정하고, SQLite는 rebuildable derived index로만 허용한다고 통일한다.

#### M2. 관리자 인증 구현의 명시적 Story 소유자가 없음

- UX와 아키텍처는 operator/authoring surface가 local admin password 인증과 capability check 이후에만 노출돼야 한다고 요구한다.
- Story 1.1은 capability hiding을 다루지만 비밀번호 검증, 보호 저장소, 성공·실패 흐름을 소유하지 않는다.
- Story 4.1은 이미 인증된 authoring session을, Story 5.1은 열려 있는 operator console을 전제로 시작한다.

권고: Epic 4/5 구현 전에 인증 계약과 성공·실패·권한 거부·credential storage를 소유하는 선행 Story 또는 Story 1.1 보완 범위를 명시한다.

#### M3. Story 6.2의 하드웨어 gate 대상이 승인 변경안과 불일치함

- Story 6.2 AC는 Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3을 truth-critical closure 대상으로 열거한다.
- 같은 `epics.md`의 최신 Additional Requirements와 승인 변경안은 정식 실카메라·render 출시 truth를 Story 1.6, 1.7, 1.8로 이동했다.
- 현재 AC는 Story 1.7/1.8을 누락하고, 이미 회귀 증거로 보존하기로 한 Story 1.4/1.5를 여전히 제품-level gate처럼 표현한다.

권고: Story 6.2의 gate matrix를 1.6/1.7/1.8 중심으로 정렬하고 1.4/1.5는 회귀 증거로 명시한다.

#### M4. Story 1.9의 canonical preview 교체 방식이 immutable generation 기준과 충돌함

- Story 1.9는 pending fast preview를 canonical preview path에 올린 뒤 같은 경로에서 preset-applied preview로 교체하도록 요구한다.
- 최신 아키텍처와 같은 Epics의 Additional Requirements는 한 canonical JPEG의 in-place overwrite를 금지하고 generation별 immutable asset과 pointer 전환을 요구한다.
- viewer qualifying 경로가 아니더라도 동일 파일 교체는 torn/stale read와 구현 기준 분기를 만들 수 있다.

권고: Story 1.9도 session/capture-scoped immutable generation으로 정렬하거나, review rail 전용 비 qualifying 경로라는 경계와 atomicity 보장을 명시한다.

#### M5. Story 6.1과 Story 7.6의 범위가 여전히 큼

- Story 6.1은 지점 선택, 배포 audit, active-session defer, 호환성, rollback, 실패 거부까지 하나의 완료 단위에 결합한다.
- Story 7.6은 deadline scheduler, capacity, cancellation/process-tree 종료, seamless RAW swap, race/fault recovery와 hardware evidence를 함께 소유한다.
- 두 Story 모두 독립 검토와 실패 원인 격리가 어려운 크기다.

권고: 6.1은 rollout governance와 rollback/compatibility로, 7.6은 scheduler/cancellation과 seamless tier transition으로 나누거나 하위 deliverable별 독립 review gate를 둔다.

#### M6. 운영 복구와 rollback Story의 실패 AC가 부족함

- Story 4.4에는 승인되지 않은 rollback target, 호환성 실패, rollback 중단 시 불변성에 대한 거부 기준이 없다.
- Story 5.2에는 stale operator view, 중복·동시 action, action timeout/failure, idempotency, 실패 audit 결과가 없다.
- 복구·롤백은 오류 경로가 핵심인 기능이므로 happy path 중심 AC만으로는 충분하지 않다.

권고: 거부·중단·재시도·동시성·audit 보존 시나리오를 Given/When/Then으로 추가한다.

### Minor 우려

#### m1. Epic 7의 기술·의사결정 Story 분류

- Story 7.3과 7.5는 Enabler/Spike로 명확히 분류되어 있다.
- Story 7.2도 기술 계측 기반 enabler 성격이 강하고 Story 7.10은 제품 increment가 아니라 release decision gate다.

권고: 7.2를 Enabler로, 7.10을 release-governance gate로 명시하고 고객 가치 velocity와 구분한다.

#### m2. Starter Story 검증 범위

- Story 1.1 제목은 아키텍처가 요구한 starter-template Story와 일치한다.
- 다만 dependency 설치, lockfile, dev/build/package smoke와 초기 CI 성공이 AC에 직접 나타나지 않는다.

권고: 이미 완료된 증거를 참조하거나 bootstrap AC에 설치·빌드·통합 실행·CI baseline을 추가한다.

#### m3. UX 실사용 검증의 Story 소유권

- 터치 반응성, standing usability, 고대비, 무가이드 성공률은 UX 문서에 있으나 특정 Story AC나 최종 gate에 직접 할당되지 않았다.

권고: QA/Release evidence 항목으로 별도 추적한다.

### 특수 구현 검사

- Starter template: 공식 `Vite react-ts + manual Tauri CLI`와 Story 1.1이 존재함 — 기본 요건 충족
- 초기 프로젝트 setup Story: 존재함
- 모든 DB table 선생성 Story: 없음 — 위반 없음
- Epic 간 순환 의존: 없음
- PRD FR 추적성: 10/10 유지

### 최신 Story 7.1 상세 판정

- 명확한 고객 가치와 read-only viewer outcome을 가진다.
- In Scope와 Explicitly Out of Scope가 분리되어 Story 7.2 이후 책임을 끌어오지 않는다.
- viewer 미준비, stale epoch, reload, listener loss, wrong-session 등 실패 경로가 수용 기준과 task에 포함된다.
- 자동 검증과 HV-13A 하드웨어 검증을 구분한다.
- Story 7.2, fast source, renderer, actual-present 구현에 대한 전방 완료 의존이 없다.
- **판정: `ready-for-dev` 유지. 구현 완료 후 HV-13A Go 전까지는 `review` 유지.**

## 6. 요약 및 권고

### 전체 준비도 상태

**NEEDS WORK**

범위별 판정:

- **Story 7.1 구현 착수:** READY
- **전체 구현 백로그:** NEEDS WORK
- **신규 MVP 출시:** NOT READY / NO-GO

Story 7.1은 축소된 범위, 명확한 실패 경로, HV-13A gate를 갖춰 바로 구현할 수 있다. 그러나 전체 백로그에는 구현자가 상충된 결정을 내릴 수 있는 계약·소유권 문제가 남아 있어, 해당 Story들이 `ready-for-dev`가 되기 전에 정리해야 한다. 신규 MVP 출시는 계획대로 Story 7.10과 HV-18D가 Go를 기록하기 전까지 No-Go다.

### 즉시 조치가 필요한 핵심 이슈

1. **Story 7.1의 직접 착수 차단 이슈는 없음.** 승인 범위 그대로 구현하고 HV-13A evidence를 수집한다.
2. **계획 기준 충돌 정리:** Epics의 SQLite 요구를 최신 JSON/JSONL journal 기준으로 맞추고 Story 1.9의 canonical overwrite를 immutable generation 기준과 정렬한다.
3. **Gate 소유권 정리:** Story 6.2가 Story 1.6/1.7/1.8의 최신 hardware truth 책임을 반영하도록 수정한다.
4. **권한 경계 소유자 지정:** Epic 4/5 전에 admin-password 인증, 보호 저장소, capability gating의 구현 Story와 실패 AC를 확정한다.
5. **후속 Story 준비도 개선:** Story 6.1/7.6의 범위를 분리하거나 독립 review gate를 두고, Story 4.4/5.2의 실패·동시성·audit AC를 보강한다.

### 권장 실행 순서

1. Story 7.1을 현재 범위 그대로 구현한다.
2. 승인 하드웨어에서 HV-13A의 viewer readiness, monitor targeting, session binding, physical display-size evidence를 수집한다.
3. Story 7.1을 `review`에 두고 HV-13A Go 후에만 완료 처리한다.
4. Story 7.2 착수 전 UX의 “촬영 최소 1초 전 readiness”가 가이드인지 release gate인지 확정한다.
5. Epic 4-6 및 Story 1.9 착수 전에 M1-M4 문서 정합성 항목을 수정한다.
6. Story 6.1/7.6이 `ready-for-dev`로 이동하기 전에 범위와 실패 AC를 재검토한다.
7. UX-DR16과 실제 터치·standing usability·고대비·무가이드 성공률 evidence를 최종 release matrix에 추가한다.
8. Story 7.10/HV-18D 전까지 신규 MVP release 상태를 No-Go로 유지한다.

### 최종 요약

- 필수 문서: 4/4 존재
- PRD 기능 요구사항: 10개
- PRD 비기능 요구사항: 6개
- FR 에픽 커버리지: 10/10, 100%
- Critical 착수 차단 이슈: 0개
- 정리 필요 이슈: 12개
  - UX·추적성 이슈 3개
  - Epic/Story Major 이슈 6개
  - Story 구조·증거 Minor 이슈 3개
- 열린 PRD 검증 가정: 10개

이번 보정으로 Story 7.1은 구현 가능한 상태가 됐다. 전체 제품 준비도는 아직 완료가 아니며, 남은 계획 정합성 문제와 Epic 7의 하드웨어·설치·성능·롤백 증거가 순차적으로 닫혀야 한다.

### 평가 정보

- 평가일: 2026-08-11
- 평가자: Codex — BMAD Implementation Readiness workflow
- 적용 변경 기준: 승인된 Moderate correct-course (`sprint-change-proposal-20260811-021653.md`)
