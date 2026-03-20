# 스프린트 변경 제안서

- 프로젝트: Boothy
- 작성일시: 2026-03-20 09:39:39
- 변경 트리거: 갱신된 구현 준비도 평가 결과 `NEEDS WORK`
- 진행 모드: Batch
- 작성 근거:
  - `_bmad-output/planning-artifacts/implementation-readiness-report-20260320.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`

## 1. 이슈 요약

이전 코스 수정으로 Epic 5/6 분리, 고위험 실패 경로 보강, 아키텍처 입력 위생 정리는 대부분 해소되었다. 다만 갱신된 readiness 보고서 기준으로 구현 전 마지막 좁은 결함이 남아 있다.

핵심 문제는 두 가지다.

1. Story 6.1이 단계적 배포/롤백의 거절 경로는 잘 다루지만, PRD의 NFR-006과 NFR-002 일부를 acceptance criteria에 충분히 재기록하지 못했다.
2. `architecture.md`는 gap 분석에서 source-input 정리가 완료됐다고 말하면서도, 후반 future enhancement 목록에는 여전히 같은 일을 남은 작업처럼 적고 있어 문구 일관성이 흔들린다.

이번 변경은 범위 재조정이나 스프린트 구조 변경이 아니라, rollout safety 계약을 구현 가능한 수준으로 완전히 닫고 문서 일관성을 맞추는 마감 정리다.

## 2. 영향 분석

### 에픽 영향

- Epic 5/6 구조 자체는 적절하다.
- 에픽 재분리나 재번호 부여는 더 이상 필요 없다.
- 영향은 Epic 6의 Story 6.1 문구 보강에 집중된다.

### 스토리 영향

- Story 6.1에 다음 두 계약이 추가로 필요하다.
  - rollout/rollback 시 승인된 local settings 보존
  - 기존 승인 baseline에서 합법적으로 지속 중인 active session compatibility 보존

### 아티팩트 충돌

- PRD: 충돌 없음. 기존 NFR-002, NFR-006을 story 수준으로 더 명시하면 된다.
- UX: 충돌 없음.
- Architecture: 구조적 충돌은 없고, 문구 일관성만 정리하면 된다.
- sprint-status: 에픽/스토리 ID 변경이 없으므로 구조 수정 없음.

### 기술 영향

- rollout safety 계약이 더 구체화되면 구현자와 테스트 작성자가 `무엇을 유지해야 하는가`를 해석에 맡기지 않게 된다.
- 특히 `approved local settings preservation`과 `active-session compatibility preservation`는 release-governance 검증의 핵심 경계다.

## 3. 권장 접근 방식

### 옵션 평가

- Option 1: Direct Adjustment
  - 실행 가능성: 높음
  - 노력: Low
  - 리스크: Low
  - 판단: 문서 보강만으로 해결 가능하며 backlog 구조 변경이 필요 없다.

- Option 2: Potential Rollback
  - 실행 가능성: 낮음
  - 노력: Medium
  - 리스크: Medium
  - 판단: 이전 수정은 유효하며 되돌릴 이유가 없다.

- Option 3: PRD MVP Review
  - 실행 가능성: 낮음
  - 노력: High
  - 리스크: High
  - 판단: 남은 문제는 MVP 정의가 아니라 story-level traceability의 마지막 빈칸이다.

### 권장안

`Direct Adjustment`

Epic 6 Story 6.1과 `architecture.md`의 한 줄을 바로 수정하는 것이 가장 안전하고 빠르다.

## 4. 상세 변경 제안

### 4.1 Stories / Epics

#### 변경 제안 A: Story 6.1 rollout 시 local settings 보존 명시

섹션: `epics.md` / Story 6.1 Acceptance Criteria

OLD:

```md
**Given** a new approved build or preset stack is ready
**When** a rollout is initiated
**Then** the system targets an explicitly selected branch set rather than all branches at once
**And** the rollout records the branch set, target build, approved preset stack, approval timestamp, and actor
```

NEW:

```md
**Given** a new approved build or preset stack is ready
**When** a rollout is initiated
**Then** the system targets an explicitly selected branch set rather than all branches at once
**And** the rollout records the branch set, target build, approved preset stack, approval timestamp, and actor

**Given** a rollout targets one or more approved branches
**When** the new build or preset stack is applied
**Then** each targeted branch preserves its approved local settings such as contact information and bounded operational toggles
**And** the rollout mutates only the approved build and preset-stack state for that branch set
```

근거:

- NFR-002와 NFR-006의 `approved local settings preservation`를 story 수준으로 닫는다.

#### 변경 제안 B: Story 6.1 active-session compatibility 보존 명시

섹션: `epics.md` / Story 6.1 Acceptance Criteria

OLD:

```md
**Given** any targeted branch has an active customer session
**When** rollout would interrupt that session
**Then** the system defers or rejects rollout for that branch
**And** no forced update is applied to the active session
**And** the refusal or deferral reason is surfaced to the initiating operator and recorded in audit history
```

NEW:

```md
**Given** any targeted branch has an active customer session
**When** rollout would interrupt that session
**Then** the system defers or rejects rollout for that branch
**And** no forced update is applied to the active session
**And** the refusal or deferral reason is surfaced to the initiating operator and recorded in audit history

**Given** a targeted branch has a customer session that legitimately continues on the currently approved baseline
**When** rollout or rollback state is evaluated for that branch
**Then** the active session remains compatible with its existing approved build and preset baseline until a safe transition point is reached
**And** the deployment transition does not invalidate or corrupt the in-flight session
```

근거:

- readiness 보고서가 지적한 `active-session compatibility` 누락을 직접 해소한다.

#### 변경 제안 C: Story 6.1 rollback 시 local settings / compatibility 보존 명시

섹션: `epics.md` / Story 6.1 Acceptance Criteria

NEW 추가:

```md
**Given** rollback is approved for a selected branch set
**When** the prior approved baseline is restored
**Then** each branch preserves its approved local settings while returning to the last approved build and preset stack
**And** active-session compatibility remains protected until each branch reaches a safe transition point
```

근거:

- rollout뿐 아니라 rollback에도 같은 release-governance 계약이 적용됨을 명확히 한다.

### 4.2 Architecture

#### 변경 제안 D: source-input wording 일관성 정리

섹션: `architecture.md` / Areas for Future Enhancement

OLD:

```md
- Clean up residual source-input references
```

NEW:

```md
- Keep source-input wording aligned with the approved artifact set if future revisions change the input baseline
```

근거:

- 이미 해결된 이슈를 미해결 항목처럼 보이게 하지 않으면서, 향후 다시 drift가 생길 수 있다는 점만 남긴다.

## 5. 구현 인계 계획

### 변경 범주

`Minor`

이유:

- 에픽 구조, story ID, sprint-status 구조 변경이 없다.
- 문서 계약 보강과 wording consistency 정리만 필요하다.

### 권장 인계 대상

- Product Owner / Scrum Master
  - Story 6.1 acceptance criteria 보강 반영 확인
- Solution Architect
  - architecture wording consistency 확인
- Development Team
  - 수정된 Story 6.1 기준으로 rollout safety 구현 및 테스트 설계 진행

### 성공 기준

1. Story 6.1이 approved local settings preservation을 명시한다.
2. Story 6.1이 active-session compatibility preservation을 명시한다.
3. `architecture.md`가 source-input 상태를 모순 없이 설명한다.
4. 에픽/스토리 ID와 `sprint-status.yaml` 구조는 변경 없이 유지된다.

## 6. 체크리스트 실행 로그

| 항목 | 상태 | 메모 |
| --- | --- | --- |
| 1.1 트리거 스토리 식별 | [x] Done | 갱신된 readiness 보고서가 직접 Story 6.1을 지목 |
| 1.2 핵심 문제 정의 | [x] Done | Story 6.1의 NFR-006/NFR-002 traceability 누락 + architecture wording inconsistency |
| 1.3 초기 영향 및 근거 수집 | [x] Done | `implementation-readiness-report-20260320.md` 기준 |
| 2.1 현재 에픽 영향 평가 | [x] Done | Epic 6 문구 보강만 필요 |
| 2.2 에픽 수준 변경 도출 | [x] Done | 구조 변경 없음 |
| 2.3 남은 에픽 영향 검토 | [x] Done | Epic 1-5 영향 없음 |
| 2.4 미래 에픽 무효화/신규 필요 검토 | [x] Done | 없음 |
| 2.5 에픽 순서/우선순위 검토 | [x] Done | 변경 없음 |
| 3.1 PRD 충돌 점검 | [x] Done | 충돌 없음, traceability 보강 필요 |
| 3.2 Architecture 충돌 점검 | [x] Done | wording consistency만 필요 |
| 3.3 UX 충돌 점검 | [x] Done | 직접 충돌 없음 |
| 3.4 기타 아티팩트 영향 | [x] Done | sprint-status 구조 변경 없음 |
| 4.1 Direct Adjustment 평가 | [x] Done | Viable |
| 4.2 Rollback 평가 | [x] Done | Not viable |
| 4.3 PRD MVP Review 평가 | [x] Done | Not viable |
| 4.4 권장 경로 선택 | [x] Done | Direct Adjustment |
| 5.1 이슈 요약 작성 | [x] Done | 본 문서 1절 |
| 5.2 에픽/아티팩트 영향 정리 | [x] Done | 본 문서 2절 |
| 5.3 권장 경로와 근거 작성 | [x] Done | 본 문서 3절 |
| 5.4 MVP 영향 및 액션 플랜 | [x] Done | MVP 변경 없음 |
| 5.5 인계 계획 수립 | [x] Done | Minor handoff |
| 6.1 체크리스트 종합 검토 | [x] Done | 완료 |
| 6.2 제안서 정확성 검토 | [x] Done | 보고서와 현재 planning artifacts 교차 확인 |
| 6.3 사용자 승인 | [x] Done | 사용자 제공 보고서를 기준으로 직접 수정 진행 |
| 6.4 sprint-status 반영 | [N/A] | 에픽/스토리 키 변경 없음 |
| 6.5 다음 단계/인계 확인 | [x] Done | readiness 재실행 권장 |

## 7. 최종 권고

이번 남은 이슈는 매우 좁고 명확하다. Story 6.1의 release-governance 계약을 완전히 닫고, architecture wording을 정리하면 된다. 범위를 더 넓힐 이유는 없고, 문서 수정 후 readiness를 한 번 더 재실행하는 것이 가장 안전하다.
