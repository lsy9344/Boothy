# Sprint Change Proposal

Date: 2026-04-16 12:45 +09:00
Project: Boothy
Mode: Batch
Prepared by: Codex (`bmad-correct-course`)

## 1. Issue Summary

### Trigger

- 사용자 우선순위 변경: preview architecture와 프리셋 적용 시간단축을 다른 트랙보다 먼저 닫고 싶다는 명시적 요청
- 기준 문서:
  - `docs/preview-architecture-history-and-agent-guide.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `history/camera-capture-validation-history.md`

### Core Problem Statement

문서와 구현 산출물에는 새 preview 방향이 이미 반영되어 있지만, 제품 기준에서는 여전히 `release hold / Story 1.13 No-Go` 상태다. 현재 sprint 추천은 Story `4.3` continuation으로 읽히지만, 가장 큰 release risk는 여전히 preview local lane이 `same-capture preset-applied full-screen visible <= 2500ms`를 닫지 못하고 있다는 점이다.

### Evidence

- `docs/preview-architecture-history-and-agent-guide.md`
  - 현재 문제는 "새 preview 아키텍처가 아직 안 켜져 있다"가 아니라고 명시한다.
  - `local dedicated renderer + first-visible lane 분리`는 canary까지 적용됐지만 KPI를 반복적으로 닫지 못했다고 정리한다.
  - 현재 정답 후보는 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`의 실제 KPI 증명이라고 못박는다.
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - Story `1.13` canonical row는 여전히 `No-Go`다.
  - `sameCaptureFullScreenVisibleMs`가 새 release field이며, 속도 alone으로 `Go`를 줄 수 없다고 명시한다.
- `history/camera-capture-validation-history.md`
  - 2026-04-16 최신 기록에서 warm-hit 조건에서도 `replacementMs`가 6~7초대에 남아 있다.
  - same session follow-up capture timeout이 다시 보고됐다.
  - 병목은 점점 `dedicated renderer가 darktable-cli로 preview close를 만드는 자체 시간`과 helper completion boundary로 좁혀졌다고 정리된다.

## 2. Impact Analysis

### Epic Impact

- Epic 1은 구조를 유지하되, preview track corrective follow-up story를 하나 더 추가해야 한다.
- Epic 4 Story `4.3`은 중요하지만, 현재 release-risk 우선순위에서는 preview corrective work 뒤로 민다.
- Epic 2~6의 scope 자체는 변경하지 않는다.

### Story Impact

- Story `1.13`
  - 계속 blocked 유지
  - 새 corrective story가 local path viability를 다시 입증하기 전에는 reopen하지 않는다
- Story `1.26`
  - 계속 closed 유지
  - Story `1.27` corrective attempt 이후에도 local path가 반복 실패할 때만 reserve track 판단 대상으로 남긴다
- Story `4.3`
  - in-progress 상태는 유지
  - 단, sprint 추천 우선순위에서는 preview corrective work보다 뒤로 민다
- New Story `1.27`
  - local hot path가 darktable-compatible preview run에 다시 묶이지 않는지 증명
  - cold 1컷 + 연속 3~5컷 hardware rerun으로 KPI와 follow-up capture health를 함께 재검증

### Artifact Conflict Assessment

- PRD: 변경 불필요
  - KPI와 release gate는 이미 적절하다
- Architecture: 변경 불필요
  - target architecture 자체는 이미 적절하다
- UX: 변경 불필요
  - 이번 변경은 제품 우선순위와 implementation follow-up 문제다
- Epics: 변경 필요
  - corrective local-path story 추가 필요
- Sprint Status: 즉시 변경 필요
  - recommended next story/action을 preview corrective work 기준으로 바꿔야 한다

### Technical / Operational Impact

- 새 작업은 “또 다른 low-risk tuning 반복”이 아니어야 한다
- local hot path provenance와 hardware proof를 같이 닫아야 한다
- 결과가 실패면 `No-Go` 유지 근거가 더 강해지고, 그때만 reserve track 검토가 정당화된다

## 3. Recommended Approach

### Chosen Path

Direct Adjustment

### Rationale

- 요구사항을 바꾸는 문제가 아니라, 현재 release-risk를 먼저 줄이기 위한 sprint priority correction이다
- architecture 방향은 이미 맞다
- 부족한 것은 실제 제품 close proof이며, 그 gap을 메우는 bounded corrective story가 필요하다

### Scope / Risk

- Scope: Moderate
- Risk: Controlled
- Reason:
  - story 추가와 sprint priority 변경은 필요하지만, PRD/Architecture 재작성은 필요 없다
  - Story `1.13`과 Story `1.26`의 gate를 흐리지 않는 bounded follow-up으로 처리 가능하다

## 4. Detailed Change Proposals

### A. Epics Update

Artifact: `_bmad-output/planning-artifacts/epics.md`

#### Proposal EPIC-A1: Story 1.27 추가

NEW:

```md
### Story 1.27: local hot path darktable 절연과 2500ms KPI 재검증

As a owner / brand operator,
I want local lane hot path가 darktable preview run에 다시 묶이지 않는지 증명하고 싶다,
So that reserve path를 열기 전에 local forward path의 실제 가능성을 마지막으로 검증할 수 있다.
```

Rationale:

- 현재 문서상 가장 큰 blocker는 “방향 부재”가 아니라 “실제 hot path proof 부재”다.
- Story `1.27`은 Story `1.13` final close나 Story `1.26` reserve opening을 대체하지 않고, 그 사이의 corrective proof owner가 된다.

#### Proposal EPIC-A2: Preview sequencing note 강화

NEW intent:

```md
- Story 1.27은 local hot path가 darktable-bound close에 다시 묶이지 않는지 교정적으로 검증하는 follow-up이다.
- Story 1.13은 Story 1.27이 local path viability를 다시 입증하기 전에는 reopen하지 않는다.
- Story 1.26은 Story 1.27 이후에도 approved hardware KPI 반복 실패가 확인될 때만 검토한다.
```

### B. Sprint Status Update

Artifact: `_bmad-output/implementation-artifacts/sprint-status.yaml`

#### Proposal STATUS-B1: preview corrective story를 다음 실행 story로 승격

OLD:

```yaml
recommended_next_story: "4.3-승인과-불변-게시-아티팩트-생성"
```

NEW:

```yaml
recommended_next_story: "1.27-local-hot-path-darktable-절연과-2500ms-kpi-재검증"
recommended_next_action: "Implement Story 1.27 first, rerun cold 1-shot plus 3-5 sequential captures on approved hardware, and keep Story 1.13 blocked until the local-lane Go candidate is re-proven."
```

Rationale:

- 현재 제품에서 가장 큰 release blocker는 preview close proof다.
- Story `4.3`은 중요하지만, preview release-hold 해제보다 먼저 닫을 이유는 약하다.

#### Proposal STATUS-B2: Story 1.27을 ready-for-dev로 추가

NEW:

```yaml
1-27-local-hot-path-darktable-절연과-2500ms-kpi-재검증: ready-for-dev
```

## 5. Implementation Handoff

### Scope Classification

Moderate

### Handoff Recipients

- Scrum / planning owner
  - sprint priority correction 반영
- Dev
  - Story `1.27` 구현
- Hardware validation owner
  - approved hardware rerun package 수집 및 ledger 해석

### Success Criteria

- local hot path provenance가 operator-safe evidence와 bundle에서 명확히 읽힌다
- cold 1컷 + 연속 3~5컷 evidence package가 같은 기준으로 수집된다
- `sameCaptureFullScreenVisibleMs`, wrong-capture, fidelity drift, fallback ratio, follow-up capture health를 함께 판단할 수 있다
- 결과가 실패여도 Story `1.13`과 Story `1.26` gate가 더 명확해진다

### Approval / Execution Note

- 사용자 승인에 따라 2026-04-16 본 제안과 연계된 epics / sprint-status / story artifact 보정을 함께 진행한다.
