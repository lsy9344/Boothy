---
workflowType: 'correct-course'
project_name: 'Boothy'
date: '2026-04-06'
status: 'implemented'
changeScope: 'moderate'
artifactsImpacted:
  - '_bmad-output/planning-artifacts/architecture.md'
  - '_bmad-output/planning-artifacts/prd.md'
  - 'docs/recent-session-preview-architecture-update-input-2026-04-06.md'
mode: 'batch'
---

# Sprint Change Proposal - Recent Session Preview Replacement Reframe

## 1. Issue Summary

최근 실측과 세션 evidence를 기준으로 보면 현재 핵심 병목은 `thumbnail latency` 자체가 아니라 `latest preset-applied preview replacement latency`다.

- same-capture `first-visible`은 최근 기준 대체로 `3.0s - 3.5s`, best run `2959ms`까지 내려왔다.
- 하지만 고객이 실제로 기다리는 `preset-applied truthful close`는 best run `6372ms`, 다른 회차는 `7s - 8.5s`, 첫 컷 worst case `10403ms` 수준에 남아 있다.
- 따라서 기존 문서에서 레일 썸네일 중심으로 읽히던 목표와 KPI를 큰 화면의 latest preview replacement 중심으로 다시 고정해야 한다.

## 2. Impact Analysis

### Epic Impact

- 현재 epic 구조 자체를 새로 갈아엎을 필요는 없다.
- 다만 Story `1.5`와 관련 preview/review 계열 후속 story는 `rail thumbnail speed`보다 `latest large preview replacement`, `Preview Waiting`, `same-slot replacement`, `preset-version binding` 기준으로 다시 읽혀야 한다.
- Epic `4-6`도 publication, renderer routing, hardware validation evidence를 같은 방향으로 해석해야 한다.

### Artifact Impact

- PRD 수정 필요:
  - 문제 정의를 `latest large preview replacement` 중심으로 재정의
  - KPI를 `first-visible` vs `truthful close`로 분리
  - FR-004, NFR-003, release gate를 same-slot replacement와 host-owned truth 기준으로 강화
- Architecture 수정 필요:
  - latest large preview를 primary artifact로 고정
  - rail thumbnail을 derived/shared artifact로 재정의
  - `first-visible lane`과 `truthful close lane` 관계 명시
  - `Preview Waiting`, host-owned truth, preset-version binding, route evidence 계약 고정
- UX 문서는 이번 턴에 직접 수정하지 않았지만, latest photo rail 중심 서술은 다음 정합성 점검 때 large preview 우선 관점으로 맞춰볼 필요가 있다.

### Technical Impact

- 현재 darktable/local-renderer 계열 구조는 유지 가능하다.
- 그러나 최적화 기준은 `first-visible`이 아니라 `previewVisibleAtMs`와 truthful close owner latency여야 한다.
- 다음 아키텍처 검토 후보는 아래 3개다.
  - `local dedicated truthful renderer`
  - `preview-only artifact`
  - `different close topology`

## 3. Recommended Approach

선택 경로는 `Hybrid`다.

- Option 1 Direct Adjustment:
  - 문서 계약과 성공 기준을 먼저 재정렬하고,
  - 그 기준으로 downstream story regeneration과 hardware validation을 다시 읽는다.
- Option 2 Rollback:
  - 필요하지 않다. 기존 방향 중 `fast first-visible + later truthful replacement` 자체는 유효했다.
- Option 3 PRD MVP Review:
  - MVP 범위를 줄일 필요는 없지만, MVP 성공 판정 기준은 반드시 큰 화면의 truthful close 중심으로 바뀌어야 한다.

이 경로를 권장하는 이유는 기존 구조의 성과를 버리지 않으면서도, 실제 고객 대기 시간을 기준으로 다음 설계 판단을 정확히 유도하기 때문이다.

## 4. Detailed Change Proposals

### Architecture

- OLD:
  - latest-photo confidence와 rail/thumbnail 관점이 상대적으로 중심처럼 읽힐 수 있었다.
- NEW:
  - latest large preview를 primary booth artifact로 고정
  - rail thumbnail은 derived/shared artifact로 재정의
  - `first-visible lane`과 `truthful close lane`을 one capture, one slot, one preset-version binding으로 연결
  - `Preview Waiting`, host-owned readiness, same-slot replacement, route evidence를 계약으로 승격

### PRD

- OLD:
  - preview speed는 있었지만 중심 사용자 가치가 large preview replacement인지가 충분히 전면화되지 않았다.
- NEW:
  - Business Problem, Product Thesis, KPI, FR-004, NFR-003, release gate를 latest preset-applied large preview replacement 중심으로 재서술
  - `fastPreviewVisibleAtMs`와 `previewVisibleAtMs`를 분리
  - first-visible 성과만으로 성공 판단하지 않도록 baseline/target을 명확화

## 5. Implementation Handoff

### Scope Classification

- `Moderate`

### Handoff Targets

- Architect:
  - truthful close path의 다음 후보 3개 비교 기준 정리
- PM/PRD owner:
  - KPI와 success narrative가 preview replacement 중심으로 유지되는지 후속 artifact 점검
- Dev/QA:
  - per-session seam completeness, route evidence, hardware validation gate를 최신 계약 기준으로 재점검

### Success Criteria

- downstream 문서와 stories가 `thumbnail speed`만으로 성공을 주장하지 않는다.
- `Preview Waiting`과 `previewReady` 사이 계약이 흔들리지 않는다.
- latest large preview replacement evidence를 세션 패키지 단위로 설명할 수 있다.

## 6. Approval and Outcome

사용자 요청에 따라 본 proposal의 핵심 변경은 같은 턴에서 바로 반영했다.

- 반영 완료 문서:
  - [architecture.md](C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/_bmad-output/planning-artifacts/architecture.md)
  - [prd.md](C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/_bmad-output/planning-artifacts/prd.md)
