# 스프린트 변경 제안서

- 프로젝트: Boothy
- 작성일시: 2026-03-27 19:31:47 +09:00
- 변경 트리거: 실카메라/helper readiness truth 미연결로 인한 false-ready 위험과 Story 1.4 done 판정 불일치
- 진행 모드: Batch
- 작성 근거:
  - `_bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md`
  - `_bmad-output/implementation-artifacts/5-1-운영자용-현재-세션-문맥과-장애-진단-가시화.md`
  - `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `docs/runbooks/booth-hardware-validation-checklist.md`

## 1. 이슈 요약

현재 구현은 문서상으로는 `camera/helper ready`를 booth capture eligibility의 핵심 입력으로 다루고 있지만, 제품 관점의 완료 기준은 아직 닫히지 않았다.

핵심 문제는 세 가지다.

1. Story 1.4는 `done`으로 닫혀 있지만, 실카메라/helper readiness truth가 실제 Tauri 런타임과 실장비 검증 증거로 잠기기 전에는 false-ready 위험이 남는다.
2. Story 5.1은 blocked-state 진단은 제공하지만, 운영자가 즉시 확인해야 하는 `카메라 연결 상태`를 전용 항목으로 고정하지 않았다.
3. 하드웨어 검증 런북은 관련 story를 실장비 검증 전 `review`로 유지하거나 별도 hardware-validation gate로 잠그라고 말하는데, 현재 sprint 상태는 관련 story들을 이미 `done`으로 닫고 있다.

이 문제는 단순 구현 품질 이슈가 아니라, 제품이 `Ready`와 `Completed`를 언제 주장해도 되는지에 대한 release truth 이슈다.

## 2. 영향 분석

### 에픽 영향

- Epic 1은 직접 영향권이다. booth `Ready`와 `사진 찍기` 활성화의 기준을 실카메라/helper truth로 더 명확히 잠가야 한다.
- Epic 5는 간접 영향권이다. operator가 false-ready 위험을 조기에 볼 수 있도록 camera connection을 별도 진단 항목으로 보여줘야 한다.
- Epic 4와 Epic 3은 done 정책 영향권이다. runbook이 이미 Story 4.2, 4.3, 3.2를 실장비 검증 범위에 넣고 있으므로 제품 관점 완료 판정과 현재 sprint-status가 어긋난다.
- Epic 6 또는 release-governance 범위에 hardware-validation gate를 명시적으로 추가하는 편이 가장 자연스럽다.

### 스토리 영향

- 즉시 재판정 필요:
  - Story 1.4: `done -> review`
  - Story 4.2: `done -> review`
  - Story 4.3: `done -> review`
  - Story 3.2: `done -> review`
  - Story 1.5: `done -> review`
- 추가 분해 권장:
  - Epic 1 follow-up: 실카메라/helper readiness truth 연결 보강
  - Epic 5 follow-up: operator 전용 camera connection 상태 항목
  - Epic 6 follow-up: hardware validation gate와 evidence 기반 done 정책

### 아티팩트 충돌

- PRD: 충돌 없음. 오히려 PRD의 truthful readiness / truthful completion / bounded operator recovery 원칙을 현재 sprint 판정이 충분히 반영하지 못하고 있다.
- Architecture: 충돌 없음. architecture는 host가 camera/helper truth를 정규화한다고 분명히 말하고 있으므로, 이번 변경은 그 계약을 sprint 운영에 맞추는 작업이다.
- UX: 충돌 없음. 고객용 안내는 여전히 plain-language를 유지하되, `Ready` 주장 시점만 더 보수적으로 잠그면 된다.
- Runbook / Sprint status: 현재 가장 큰 충돌 지점이다. runbook은 하드웨어 검증 전 done 금지를 권장하지만 sprint-status는 이미 다수의 관련 story를 `done`으로 닫고 있다.

### 기술 영향

- booth는 브라우저 fallback 또는 mock truth가 아니라 live host truth에만 의존해야 한다.
- operator는 generic blocked-state만으로는 부족하고, `camera connected / helper connected / capture boundary clear`를 빠르게 읽을 수 있어야 한다.
- done 정책은 자동 테스트 완료와 제품 관점 완료를 분리해야 한다.

## 3. 권장 접근 방식

### 옵션 평가

- Option 1: Direct Adjustment
  - 실행 가능성: 높음
  - 노력: Medium
  - 리스크: Low
  - 판단: 현행 epics를 유지하면서 story 재분해와 sprint status 정책 조정으로 해결 가능하다.

- Option 2: Potential Rollback
  - 실행 가능성: 낮음
  - 노력: High
  - 리스크: Medium
  - 판단: 이미 구현된 readiness, diagnostics, validation 토대를 버릴 이유는 없다. 완료 판정과 hardware gate를 바로잡는 편이 낫다.

- Option 3: PRD MVP Review
  - 실행 가능성: 낮음
  - 노력: High
  - 리스크: High
  - 판단: MVP 자체를 바꿀 필요는 없다. release truth와 sprint closure 기준을 조정하면 된다.

### 권장안

`Direct Adjustment + Release Gate 강화`

구현을 되돌리기보다,

1. Story 1.4를 제품 관점에서 다시 열고
2. operator camera connection visibility를 별도 story로 분리하고
3. hardware validation gate를 sprint 완료 조건으로 올리는 방식이 가장 안전하다.

## 4. 상세 변경 제안

### 4.1 Story 재판정

#### 변경 제안 A: 관련 story의 완료 상태를 hardware gate 이전 단계로 되돌림

섹션: `sprint-status.yaml`

OLD:

```yaml
1-4-준비-상태-안내와-유효-상태에서만-촬영-허용: done
1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백: done
3-2-export-waiting과-truthful-completion-안내: done
4-2-부스-호환성-검증과-승인-준비-상태-전환: done
4-3-승인과-불변-게시-아티팩트-생성: done
```

NEW:

```yaml
1-4-준비-상태-안내와-유효-상태에서만-촬영-허용: review
1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백: review
3-2-export-waiting과-truthful-completion-안내: review
4-2-부스-호환성-검증과-승인-준비-상태-전환: review
4-3-승인과-불변-게시-아티팩트-생성: review
```

근거:

- runbook이 이 다섯 story를 실장비 검증 범위로 직접 지정한다.
- 현재 `done`은 구현 완료에는 맞을 수 있어도 제품 완료에는 이르다.

### 4.2 Story 1.4 정렬

#### 변경 제안 B: Story 1.4를 실장비 truth 기준으로 재정의

스토리: `Story 1.4`
섹션: Acceptance Criteria / Done 판정

OLD:

```md
Given an active session and selected preset
When camera or host readiness changes
Then the booth translates runtime truth into plain-language customer states
And customer copy stays within the approved low-density guidance rule
```

NEW:

```md
Given an active session and selected preset
When live Tauri host receives real camera/helper readiness changes
Then the booth shows `Ready` only when both the capture boundary and helper boundary are actually ready on approved booth hardware
And the `사진 찍기` action remains disabled for every other state including browser preview fallback, disconnected camera, helper recovery, or stale readiness

Given the booth loses camera or helper readiness after previously being ready
When the live boundary degrades
Then the booth immediately exits `Ready`
And capture is blocked without exposing technical diagnostics

Done 판정:
- automated test 통과만으로 닫지 않는다
- HV-02, HV-03, HV-10 evidence가 수집되기 전까지 `review` 유지
```

근거:

- 고객이 보는 `Ready`는 실장비 truth와 1:1로 묶여야 한다.
- false-ready는 UX copy 문제가 아니라 release blocker다.

### 4.3 Story 분해안

#### 변경 제안 C: Epic 1 follow-up story 추가

신규 스토리 권장 ID: `Story 1.6`

제목:
`실카메라/helper readiness truth 연결과 false-ready 차단`

권장 범위:

- booth readiness를 browser fallback, fixture, stale session truth와 명시적으로 분리
- live camera disconnect / reconnect 시 `Ready` 해제와 복귀 확인
- `Ready`와 `사진 찍기` 활성화가 같은 host-owned truth에서만 나오도록 고정
- hardware evidence를 capture/readiness story의 release gate로 연결

권장 AC:

1. approved booth hardware에서 real camera/helper가 모두 준비된 경우에만 booth가 `Ready`와 enabled capture CTA를 보여준다.
2. camera/helper 중 하나라도 준비되지 않으면 booth는 `Ready`를 주장하지 않는다.
3. ready 이후 장비가 분리되면 booth는 즉시 blocked guidance로 내려간다.
4. HV-02, HV-03, HV-10을 통과하기 전까지 story status는 `review`다.

분해 이유:

- 현재 Story 1.4는 구현 토대를 담고 있지만, real-hardware truth closure까지 포함하면 지나치게 넓어진다.
- follow-up story로 분리하면 구현 hardening과 validation evidence를 함께 잠글 수 있다.

#### 변경 제안 D: Epic 5 follow-up story 추가

신규 스토리 권장 ID: `Story 5.4`

제목:
`운영자용 카메라 연결 상태 전용 항목과 helper readiness 가시화`

권장 범위:

- operator summary에 `카메라 연결 상태` 전용 카드 또는 행 추가
- generic blocked-state와 별개로 `camera disconnected / helper preparing / ready / degraded after ready`를 읽을 수 있게 함
- booth copy와 operator diagnostics를 계속 분리

권장 AC:

1. operator console은 blocked-state category 외에 `카메라 연결 상태`를 전용 항목으로 표시한다.
2. 이 항목은 최소 `미연결`, `연결 중`, `연결됨`, `복구 필요` 또는 동등한 집합을 가진다.
3. 상태는 raw helper log가 아니라 operator-safe normalized truth에서 계산된다.
4. booth가 false-ready를 주장할 여지가 있는 경우 operator는 이 전용 항목에서 먼저 이를 발견할 수 있다.

분해 이유:

- 기존 5.1은 진단 요약에는 적합하지만, 운영자가 가장 먼저 확인할 `카메라 연결`을 1급 신호로 승격하지 않았다.

#### 변경 제안 E: Epic 6 follow-up story 추가

신규 스토리 권장 ID: `Story 6.2`

제목:
`실장비 hardware validation gate와 evidence 기반 done 정책`

권장 범위:

- `done`과 `제품 관점 완료`를 분리하는 sprint policy 고정
- 관련 story를 `review`로 유지하는 규칙 추가
- HV evidence 패키지와 Go / No-Go 판정을 sprint closure에 연결

권장 AC:

1. Story 1.4, 1.5, 3.2, 4.2, 4.3은 지정된 HV 항목 통과 전 `done`으로 닫지 않는다.
2. 각 story는 대응 HV evidence 경로를 명시한다.
3. sprint review는 구현 완료와 hardware validation 완료를 별도 체크로 본다.
4. No-Go 시 관련 story는 자동으로 `review` 상태를 유지하거나 되돌린다.

분해 이유:

- 이번 문제는 코드보다 운영 규칙의 누락이 더 크다.
- release-governance story로 올려두는 편이 재발 방지에 적합하다.

### 4.4 Runbook / Planning 문서 정렬

#### 변경 제안 F: epics 또는 sprint 운영 문서에 hardware gate 명시 추가

섹션: `epics.md` Additional Requirements 또는 Epic 6

NEW:

```md
- Story 1.4, 1.5, 3.2, 4.2, 4.3은 자동 테스트 완료만으로 제품 관점 `done`으로 간주하지 않는다.
- 지정된 booth hardware validation checklist evidence가 수집되기 전까지 해당 story는 `review` 또는 동등한 pre-close 상태에 머문다.
- booth `Ready`와 `Completed`는 각각 false-ready, false-complete 방지 evidence가 확보된 뒤에만 release truth로 인정한다.
```

근거:

- 현재 규칙은 runbook에만 있고 planning artifact에는 약하게 반영돼 있다.

## 5. 권장 스프린트 변경안

### 권장 배치

1. 현재 스프린트에서 즉시 수행
   - Story 1.4 재오픈
   - 관련 story status를 `review`로 재분류
   - Story 1.6, 5.4, 6.2 생성

2. 다음 구현 순서
   - 1순위: Story 1.6
   - 2순위: Story 5.4
   - 3순위: Story 6.2
   - 4순위: HV-00, HV-02, HV-03, HV-04, HV-05, HV-08, HV-10, HV-11, HV-12 실행

3. 스프린트 종료 조건
   - booth가 live hardware truth 없이 `Ready`를 주장하지 않음
   - operator가 camera connection 상태를 단독으로 확인 가능
   - 관련 story가 hardware evidence 없이 `done`으로 닫히지 않음

### 제안 상태표

```yaml
epic-1: in-progress
1-4-준비-상태-안내와-유효-상태에서만-촬영-허용: review
1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백: review
1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단: backlog

epic-3: in-progress
3-2-export-waiting과-truthful-completion-안내: review

epic-4: in-progress
4-2-부스-호환성-검증과-승인-준비-상태-전환: review
4-3-승인과-불변-게시-아티팩트-생성: review

epic-5: in-progress
5-4-운영자용-카메라-연결-상태-전용-항목과-helper-readiness-가시화: backlog

epic-6: in-progress
6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책: backlog
```

## 6. 구현 인계 계획

### 변경 범주

`Moderate`

이유:

- 에픽 재구성까지는 아니지만, 다수 story의 상태 재판정과 신규 story 생성이 필요하다.
- 개발, PO/SM, 운영 검증 역할이 함께 움직여야 한다.

### 권장 인계 대상

- Product Owner / Scrum Master
  - 관련 story status를 `review`로 재분류
  - Story 1.6 / 5.4 / 6.2 생성
  - sprint closure 기준 수정

- Development Team
  - live readiness wiring과 operator connection visibility 구현
  - hardware validation evidence 수집 준비

- Operator / QA / 현장 검증 담당
  - HV-00 ~ HV-12 중 우선 게이트 실행
  - Go / No-Go 판정 기록

### 성공 기준

1. booth는 실카메라/helper ready일 때만 `Ready`와 `사진 찍기` 활성화를 보여준다.
2. operator는 카메라 연결 상태를 generic blockage와 별도로 본다.
3. Story 1.4, 1.5, 3.2, 4.2, 4.3은 실장비 검증 전 `done`이 아니다.
4. sprint review에서 automated pass와 hardware pass가 분리 기록된다.

## 7. 체크리스트 실행 로그

| 항목 | 상태 | 메모 |
| --- | --- | --- |
| 1.1 트리거 스토리 식별 | [x] Done | Story 1.4 중심 false-ready / done misalignment |
| 1.2 핵심 문제 정의 | [x] Done | readiness truth 미연결 + operator visibility 부족 + done gate 미정렬 |
| 1.3 초기 영향 및 근거 수집 | [x] Done | story docs, architecture, sprint-status, hardware checklist 교차 확인 |
| 2.1 현재 에픽 영향 평가 | [x] Done | Epic 1, 5, 6 직접 영향 |
| 2.2 에픽 수준 변경 도출 | [x] Done | 신규 epic 불필요, follow-up stories 권장 |
| 2.3 남은 에픽 영향 검토 | [x] Done | Epic 3, 4는 done policy 영향 |
| 2.4 미래 에픽 무효화/신규 필요 검토 | [x] Done | 신규 epic보다 Epic 6 follow-up이 적합 |
| 2.5 에픽 순서/우선순위 검토 | [x] Done | readiness -> operator -> gate 순서 권장 |
| 3.1 PRD 충돌 점검 | [x] Done | 충돌 없음, truthful state 강화 |
| 3.2 Architecture 충돌 점검 | [x] Done | host-owned truth 원칙과 일치 |
| 3.3 UX 충돌 점검 | [x] Done | plain-language 유지, ready 주장 시점만 조정 |
| 3.4 기타 아티팩트 영향 | [x] Done | sprint-status / runbook / story status 영향 큼 |
| 4.1 Direct Adjustment 평가 | [x] Done | Viable |
| 4.2 Rollback 평가 | [x] Done | Not viable |
| 4.3 PRD MVP Review 평가 | [x] Done | Not viable |
| 4.4 권장 경로 선택 | [x] Done | Direct Adjustment + Release Gate 강화 |
| 5.1 이슈 요약 작성 | [x] Done | 본 문서 1절 |
| 5.2 에픽/아티팩트 영향 정리 | [x] Done | 본 문서 2절 |
| 5.3 권장 경로와 근거 작성 | [x] Done | 본 문서 3절 |
| 5.4 MVP 영향 및 액션 플랜 | [x] Done | MVP 유지, closure 기준만 조정 |
| 5.5 인계 계획 수립 | [x] Done | Moderate handoff |
| 6.1 체크리스트 종합 검토 | [x] Done | 완료 |
| 6.2 제안서 정확성 검토 | [x] Done | runbook와 current status의 충돌을 중심으로 검토 |
| 6.3 사용자 승인 | [!] Action-needed | 승인 후 status / story 생성 반영 |
| 6.4 sprint-status 반영 | [!] Action-needed | 승인 전이라 실제 파일 미수정 |
| 6.5 다음 단계/인계 확인 | [x] Done | 적용 순서 포함 |

## 8. 최종 권고

이번 변경의 핵심은 "구현을 더 많이 만드는 것"보다 "언제 제품이 Ready라고 말해도 되는가"를 바로잡는 것이다.

권장 결론은 아래와 같다.

1. Story 1.4는 즉시 `review`로 되돌린다.
2. operator camera connection은 별도 story로 승격한다.
3. hardware validation evidence가 모이기 전까지 관련 story를 `done`으로 닫지 않는다.

이렇게 해야 false-ready 위험을 줄이면서도 현재 구현 자산은 살리고, sprint 운영 기준만 제품 truth에 맞게 교정할 수 있다.
