---
workflow: correct-course
project: Boothy
date: 2026-03-31 23:14:37 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: batch
approval_status: approved
approved_at: 2026-03-31 23:18:00 +09:00
approval_decision: yes
trigger_reference: _bmad-output/implementation-artifacts/1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md
---

# Sprint Change Proposal - 게시된 프리셋 적용 render truth 보강

## 0. 워크플로우 프레이밍

- 이번 correct-course는 지정된 Story 1.8 문서를 "이미 구현된 스토리"가 아니라 "추가 기능 변경 요청을 설명하는 참조 문서"로 간주해 수행했다.
- 사용자 요청에 따라 코드 수정은 수행하지 않았고, 계획/문서 영향과 실행 핸드오프만 정리했다.
- 인터랙티브 모드 질의 대신 `Batch` 모드를 가정했다.
- `project-context.md`는 찾지 못해 별도 프로젝트 컨텍스트 파일은 없는 것으로 처리했다.
- 검토한 기준 문서:
  - PRD
  - Epics
  - Architecture
  - UX Design Specification
  - `docs/contracts/preset-bundle.md`
  - `docs/contracts/render-worker.md`
  - `sprint-status.yaml`

## 1. 체크리스트 요약

### 1. Trigger and Context

- [x] 1.1 Trigger story identified
  - 트리거 문서: Story 1.8 참조 문서
  - 요약: published preset bundle이 booth runtime의 preview/final render truth까지 닫아야 하는데, 기존 스토리 분해에서는 이 연결 책임이 빠졌다.
- [x] 1.2 Core problem defined
  - 이슈 유형: 기존 요구사항 분해 과정의 책임 누락
  - 문제 진술: "프리셋을 선택했다"와 "선택한 프리셋이 실제 preview/final 결과물에 적용되었다"가 같은 것으로 취급되어 제품 진실이 느슨해졌다.
- [x] 1.3 Evidence gathered
  - PRD는 preview readiness와 final readiness를 capture success와 분리한다.
  - Epics는 `darktable-cli` render worker를 권위 경로로 이미 요구한다.
  - Architecture는 Rust render worker와 capture/render truth 분리를 이미 고정한다.
  - UX는 고객이 선택한 룩과 실제 결과가 일관될 것이라는 정신 모델을 전제한다.

### 2. Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 기존 목표를 유지할 수 있다.
  - 다만 Story 1.5/1.7만으로는 "preset-applied preview truth"가 닫히지 않으므로 corrective follow-up이 필요하다.
- [x] 2.2 Epic-level change identified
  - 새 epic은 필요 없다.
  - Epic 1 안에서 corrective story를 명시적으로 유지하는 것이 맞다.
- [x] 2.3 Remaining epics reviewed
  - Epic 3은 final truth gating 영향이 있다.
  - Epic 4는 published bundle이 catalog metadata를 넘어 runtime artifact라는 점이 더 분명해져야 한다.
  - Epic 5는 operator diagnostics 분리가 유지되어야 한다.
- [x] 2.4 Future epic invalidation checked
  - 무효화되는 epic은 없다.
  - 새 epic도 필요 없다.
- [x] 2.5 Epic priority/order checked
  - 우선순위 재정렬은 필요 없다.
  - 단, Story 1.8은 Epic 1의 truth closure와 Epic 3/4 dependency를 묶는 선행 corrective story로 취급해야 한다.

### 3. Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - PRD 방향과 충돌하지 않는다.
  - 오히려 FR-004, FR-007, Published Preset Artifact Model의 제품 약속을 더 명시적으로 닫아야 한다.
- [x] 3.2 Architecture conflict reviewed
  - Architecture 방향 전환은 필요 없다.
  - 이미 선언된 render worker 원칙을 capture-bound bundle resolution과 ready gating 수준까지 더 선명하게 적어야 한다.
- [x] 3.3 UX impact reviewed
  - 새 화면 발명은 필요 없다.
  - `Preview Waiting`, latest photo rail, post-end completion copy의 truth 기준만 강화하면 된다.
- [x] 3.4 Other artifacts reviewed
  - 계약 문서와 hardware evidence 묶음은 유지 또는 보강 대상이다.
  - `sprint-status.yaml`은 이미 Story 1.8이 `ready-for-dev`로 존재하므로 구조 수정이 필요하지 않다.

### 4. Path Forward Evaluation

- [x] 4.1 Option 1 Direct Adjustment
  - Viable
  - Effort: Medium
  - Risk: Medium
  - 기존 문서와 계획을 유지하면서 누락된 truth 연결만 corrective change로 닫을 수 있다.
- [x] 4.2 Option 2 Potential Rollback
  - Not viable
  - Effort: High
  - Risk: High
  - 이미 정리된 capture/post-end/publication 흐름을 되돌리는 비용이 크고, 문제의 본질도 rollback보다 연결 책임 보강에 가깝다.
- [x] 4.3 Option 3 PRD MVP Review
  - Not viable
  - Effort: Medium
  - Risk: Medium
  - MVP 범위를 줄일 사안이 아니라 기존 MVP 약속을 정직하게 구현하는 사안이다.
- [x] 4.4 Recommended path selected
  - 선택안: Option 1 Direct Adjustment
  - 이유: 제품 방향은 맞고, 빠진 연결 책임만 회수하면 되기 때문이다.

### 5. Proposal Components

- [x] 5.1 Issue summary prepared
- [x] 5.2 Epic and artifact impact summarized
- [x] 5.3 Recommended path documented
- [x] 5.4 MVP impact and action plan defined
- [x] 5.5 Handoff plan defined

### 6. Final Review and Handoff

- [x] 6.1 Checklist completion reviewed
- [x] 6.2 Proposal consistency reviewed
- [!] 6.3 User approval pending
- [N/A] 6.4 `sprint-status.yaml` structural update
  - Story 1.8 entry가 이미 존재하고 상태도 `ready-for-dev`이므로 이번 change proposal 자체로 추가 수정은 필요 없다.
- [!] 6.5 Final handoff confirmation pending

## 2. Issue Summary

### Problem Statement

현재 계획은 "승인된 게시 프리셋이 고객이 보는 룩과 실제 결과물의 기준"이라는 제품 약속을 갖고 있지만, 구현 분해 과정에서는 이 약속을 닫는 직접 책임이 비어 있었다. 그 결과 booth는 다음 두 가지를 혼동할 위험이 있었다.

- preview/final 자산이 존재한다
- 선택된 preset이 실제로 적용된 preview/final 자산이 존재한다

이 혼동이 남아 있으면 고객은 선택한 룩과 실제 결과물이 다를 수 있고, 제품은 `previewReady` 또는 `Completed`를 너무 일찍 주장하게 된다.

### Discovery Context

- Story 1.5는 저장 성공과 preview waiting을 분리했다.
- Story 1.7은 real capture와 RAW persistence 경계를 닫았다.
- Story 3.2는 post-end completion taxonomy를 정리했다.
- Story 4.3은 immutable published bundle을 만들었다.

하지만 이 네 축 사이에서 "capture-bound published bundle -> darktable render worker -> preview/final truth"를 실제 제품 책임으로 닫는 스토리가 필요해졌고, 그 입력으로 Story 1.8 참조 문서가 제시되었다.

## 3. Impact Analysis

### Epic Impact

- Epic 1 영향
  - 고객이 선택한 프리셋이 실제 preview에 반영된다는 신뢰를 닫는 corrective follow-up이 필요하다.
- Epic 3 영향
  - `Completed`는 final-ready truth 이전에 올라가면 안 되므로 final render 기준이 더 엄격해진다.
- Epic 4 영향
  - published bundle은 catalog 표시용 메타데이터를 넘어 runtime render authority로 취급되어야 한다.
- Epic 5 영향
  - 운영자 진단은 capture blockage와 preview/final render blockage를 계속 분리해야 한다.

### MVP Impact

- MVP 범위 축소는 필요 없다.
- 오히려 현재 MVP가 이미 약속한 "truthful preview/final"을 구현 계획상 명시적으로 복구하는 작업이다.

### Risk Assessment

- 제품 리스크
  - 고객이 본 룩과 저장/전달 룩이 다를 수 있다.
  - false-ready, false-complete가 발생할 수 있다.
- 운영 리스크
  - 운영자가 capture 문제와 render 문제를 혼동할 수 있다.
- 릴리스 리스크
  - 하드웨어 evidence 없이 story를 닫으면 release truth가 과장될 수 있다.

## 4. Recommended Approach

### Chosen Path

`Direct Adjustment`

### Why This Path

- PRD, Epics, Architecture가 이미 같은 방향을 가리킨다.
- 새로운 제품 방향 변경이 아니라 빠진 연결 책임을 회수하는 문제다.
- rollback이나 MVP 축소보다 비용이 적고, 기존 팀 문맥도 유지된다.

### Scope Classification

`Moderate`

### Timeline Impact

- 스프린트 구조 재설계는 불필요하다.
- 다만 Story 1.8을 단순 enhancement가 아니라 truth-critical corrective scope로 다뤄야 한다.

## 5. Detailed Change Proposals

### A. Stories / Epic Proposal

**Artifact:** `epics.md`  
**Target:** Epic 1 corrective story positioning, Epic 3/4 dependency notes

OLD:
- Epic 1은 Story 1.5와 1.7로 preview confidence와 RAW persistence를 닫은 것으로 읽히기 쉽다.
- Epic 3.2는 final truth 기준을 갖고 있지만, runtime final producer 책임이 별도 corrective story라는 점이 약하다.

NEW:
- Story 1.8을 Epic 1의 선택 사항이 아니라 "published preset apply truth를 닫는 필수 corrective follow-up"으로 명시한다.
- Story 1.8의 직접 의존성으로 Story 1.7, 3.2, 4.3을 연결하는 설명을 추가한다.
- Epic 3.2는 Story 1.8 없이 제품 관점 completion truth가 완결되지 않는다는 note를 갖는다.

Rationale:
- capture success, preview readiness, final completion을 하나의 느슨한 성공으로 보지 않도록 계획 수준에서 명확히 묶어야 한다.

### B. PRD Proposal

**Artifact:** `prd.md`  
**Target sections:** `Published Preset Artifact Model`, `FR-004`, `FR-007`, `NFR-003`, `NFR-005`, `Release Gates`

OLD:
- Published preset artifact가 booth-safe preview/final behavior를 포함한다고 설명한다.
- FR-004는 capture persistence와 preview readiness의 분리를 설명한다.
- FR-007은 `Completed`가 booth-side work 완료 후에만 가능하다고 설명한다.

NEW:
- `Published Preset Artifact Model`에 다음을 명시한다.
  - representative preview tile은 selection aid일 뿐 runtime render truth가 아니다.
  - booth runtime은 capture-bound preset version의 published bundle metadata를 실제 preview/final 생성 기준으로 사용해야 한다.
- `FR-004`에 다음을 추가한다.
  - `previewReady`는 방금 촬영본에 선택된 preset이 적용된 booth-safe preview asset이 실제로 생성된 뒤에만 성립한다.
- `FR-007`에 다음을 추가한다.
  - final deliverable이 필요한 세션의 `Completed`는 preset-applied final asset이 실제 생성되기 전에는 성립하지 않는다.
- `NFR-003`, `NFR-005`, `Release Gates`에 다음을 반영한다.
  - latency 목표는 truthful waiting을 전제로 하며 raw fallback이나 placeholder 승격으로 달성한 것으로 간주하지 않는다.

Rationale:
- 제품 약속을 "프리셋 선택 경험"에서 "실제 결과물 truth"까지 닫아야 고객 신뢰와 release gate가 일치한다.

### C. Architecture Proposal

**Artifact:** `architecture.md`  
**Target sections:** `Darktable Capability Scope`, `Closed Contract Freeze Baseline`, implementation priorities

OLD:
- Rust render worker와 `darktable-cli`가 권위 경로라고 선언돼 있다.
- capture와 render truth 분리도 선언돼 있다.

NEW:
- render worker 원칙을 다음 수준까지 명시한다.
  - runtime은 live catalog pointer가 아니라 capture record의 `presetId + publishedVersion`으로 bundle을 resolve한다.
  - `previewReady`와 `finalReady`는 실제 render output이 생성된 뒤에만 올라간다.
  - raw copy, placeholder, catalog tile은 render success를 대신할 수 없다.
  - publish/rollback 이후에도 기존 capture render는 capture-bound version을 유지한다.
  - diagnostics는 render blockage를 operator-safe vocabulary로 분리한다.

Rationale:
- 현재 architecture 방향은 맞지만, 실제 계획/구현 단계에서 빠질 수 있는 drift protection과 ready gating을 더 선명하게 잠가야 한다.

### D. UX Proposal

**Artifact:** `ux-design-specification.md`  
**Target sections:** `User Mental Model`, `Preview Waiting 보호 흐름`, timed completion 관련 설명

OLD:
- UX는 사용자가 선택한 프리셋과 결과물이 충분히 일관되다고 느껴야 한다고 설명한다.
- `Preview Waiting`은 저장 완료와 준비 중 상태를 분리해 안심을 준다.

NEW:
- latest photo rail과 post-end completion surface는 representative sample이나 placeholder가 아니라 실제 preset-applied output truth를 반영해야 한다는 문장을 추가한다.
- `Preview Waiting` 문구는 유지하되, 이 상태가 "저장 완료 후 실제 preview render 대기"임을 더 분명히 적는다.
- `Completed` 관련 UX 서술에는 final deliverable이 실제 준비되기 전 false completion을 허용하지 않는다는 기준을 덧붙인다.

Rationale:
- UX 레이어는 새 화면이 아니라 상태의 정직함을 강화하는 방향으로만 수정하는 것이 맞다.

### E. Validation / Contracts Proposal

**Artifact:** contracts + validation evidence package  
**Target:** `docs/contracts/preset-bundle.md`, `docs/contracts/render-worker.md`, hardware validation evidence

OLD:
- 계약 문서는 runtime bundle metadata와 render worker 기준을 이미 상당 부분 담고 있다.

NEW:
- 계약 문서는 Story 1.8 corrective scope의 공식 참조로 유지한다.
- hardware validation 패키지에 다음 증거를 필수로 묶는다.
  - 서로 다른 published preset 두 개가 실제 preview/final 결과를 다르게 만든 증거
  - 해당 결과가 capture-bound `presetId`, `publishedVersion`, `xmpTemplatePath`와 상관된다는 증거
  - `previewReady`/`finalReady`가 실제 산출물 이전에 올라가지 않는다는 증거

Rationale:
- 이 change는 문서나 테스트 통과만으로 닫기 어렵고, 제품 truth를 실제 부스 장비에서 증명해야 한다.

### F. Sprint Tracking Proposal

**Artifact:** `sprint-status.yaml`

OLD:
- Story 1.8이 이미 `ready-for-dev`로 존재한다.

NEW:
- 승인 전 구조 변경 없음
- 승인 후에도 story 추가/삭제가 아니라 실행 승인 성격이므로 YAML 구조 수정은 필요 없다.

Rationale:
- 이번 correct-course의 핵심은 새 번호 체계가 아니라 existing corrective story를 공식 실행 범위로 승인하는 것이다.

## 6. High-Level Action Plan

1. Product/Planning 정렬
   - Story 1.8을 feature addition이 아닌 truth-critical corrective scope로 승인한다.
2. 문서 정렬
   - PRD, Architecture, UX에 필요한 명시 문구를 반영한다.
3. 구현 핸드오프
   - 개발은 Story 1.8 범위로 preview/final render truth 연결을 수행한다.
4. 검증 핸드오프
   - QA/운영은 hardware canonical evidence를 수집한다.
5. 종료 기준
   - automated pass만으로 닫지 않고, hardware evidence까지 확보한 뒤 release truth를 판단한다.

## 7. Implementation Handoff

### Scope

`Moderate`

### Recommended Recipients

- Development team
  - Story 1.8 범위 구현
  - preview/final ready gating 연결
  - drift protection 및 diagnostics 분리
- Product Owner / Scrum Master
  - Story 1.8 우선순위와 corrective 성격을 스프린트 단위로 명시
  - 관련 문서 수정안 승인
- Architect
  - PRD/Architecture wording이 계약 문서와 일치하는지 최종 확인
- QA / Operations
  - 하드웨어 evidence 패키지 수집
  - false-ready / false-complete regression 확인

### Success Criteria

- 고객이 선택한 preset과 실제 preview/final 결과 사이의 제품 진실이 일치한다.
- booth는 render truth 이전에 `previewReady` 또는 `Completed`를 주장하지 않는다.
- publish/rollback 이후에도 기존 capture의 결과는 capture-bound preset version을 유지한다.
- 하드웨어 evidence가 story closure와 release truth를 뒷받침한다.

## 8. Approval Gate

이 제안서는 사용자 승인으로 최종 확정되었다.

- Proposal status: `approved`
- Approval decision: `yes`
- Implementation authority: `granted for Story 1.8 corrective scope`
- Sprint tracking action: `no structural sprint-status update required`

### Approved Handoff

- Development team
  - Story 1.8 corrective scope를 구현 기준으로 실행
- Product Owner / Scrum Master
  - corrective priority와 문서 반영 순서 관리
- Architect
  - PRD, Architecture, 계약 문서 정합성 최종 확인
- QA / Operations
  - hardware evidence 수집과 release truth 검증

## 9. Workflow Completion Summary

- Issue addressed: 게시된 preset bundle이 실제 preview/final render truth까지 닫히지 않던 계획 누락
- Change scope: Moderate
- Artifacts impacted: PRD, Epics, Architecture, UX, validation evidence package
- Routed to: Dev team + PO/SM + Architect + QA/Ops

Correct Course workflow complete, Noah Lee!
