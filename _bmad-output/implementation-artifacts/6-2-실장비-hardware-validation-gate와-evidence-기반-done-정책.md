# Story 6.2: 실장비 hardware validation gate와 evidence 기반 done 정책

Status: backlog

Correct Course Note: automated pass는 구현 완료의 근거일 뿐 제품 관점 완료가 아니다. 지정된 hardware validation evidence가 `Go`로 잠기기 전까지 truth-critical story는 `review`를 유지해야 한다.

## Story

owner / brand operator로서,
truth-critical story가 실장비 검증 증거 없이 제품 관점 `done`으로 닫히지 않게 하고 싶다.
그래서 구현 완료와 release truth를 혼동하지 않고, booth `Ready`와 `Completed`를 실제 증거 기반으로만 선언할 수 있다.

## Acceptance Criteria

1. Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3을 대상으로 하는 canonical hardware gate 목록과 HV checklist 매핑이 하나의 sprint-owned artifact에 고정되어야 한다. 또한 이 기준은 현재 runbook과 planning artifact 사이의 범위 불일치를 해소해야 하며, 지정 evidence가 `Go`로 기록되기 전까지 해당 story는 `review` 또는 동등한 pre-close 상태를 유지해야 한다.
2. 각 gated story의 closure evidence는 최소 `story key`, 대응 `HV checklist ID`, `evidence package path`, `executedAt`, `validator`, `booth PC`, `camera model`, 필요한 경우 `darktable pin`과 `helper identifier`, `Go / No-Go result`, `release blocker`, `follow-up owner`를 남겨야 한다. 증거는 단순 스크린샷 묶음이 아니라 `session.json`, `timing-events.log`, `bundle.json`, `catalog-state.json` 같은 핵심 근거 경로와 연결되어야 한다.
3. sprint review와 release baseline은 `automated pass`와 `hardware pass`를 분리해 기록해야 한다. automated test가 모두 통과해도 hardware pass가 없으면 release는 booth `Ready` 또는 `Completed` truth를 제품 관점 완료로 주장할 수 없어야 한다.
4. hardware validation 결과가 `No-Go`이거나 증거 패키지가 누락되면 관련 story는 계속 `review`에 머물거나 `review`로 되돌아가야 한다. 또한 release decision은 보류되어야 하고, blocker, rerun 조건, owner가 같은 운영 artifact에 기록되어야 한다.
5. 현재 영향을 받는 story 문서와 sprint 운영 산출물은 이 정책에 맞게 소급 정렬되어야 한다. 최소한 Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3은 각자 어떤 HV evidence로 닫히는지 명시적으로 참조해야 하고, sprint 운영자가 한 곳에서 자동 테스트 통과와 hardware gate 상태를 함께 확인할 수 있어야 한다.

## Tasks / Subtasks

- [x] canonical hardware gate 범위와 evidence registry를 고정한다. (AC: 1, 2, 5)
  - [x] `_bmad-output/implementation-artifacts/` 아래에 sprint-owned hardware evidence ledger 또는 동등한 운영 artifact를 만들고, story별 HV 매핑과 pass/no-go 기록 형식을 고정한다.
  - [x] canonical 매핑은 기본적으로 아래를 따른다.
    - [x] Story 1.4 -> HV-02, HV-03, HV-10
    - [x] Story 1.6 -> HV-02, HV-03, HV-10
    - [x] Story 1.5 -> HV-04, HV-05
    - [x] Story 3.2 -> HV-08, HV-11
    - [x] Story 4.2 -> HV-01, HV-09
    - [x] Story 4.3 -> HV-01, HV-07, HV-12
  - [x] 현재 `booth-hardware-validation-checklist.md`가 Story 1.3까지 범위에 넣고 있는 부분은 approved planning artifact와 충돌 여부를 검토해 정리한다. 권장 기본값은 Story 1.3을 독립 pre-close blocker로 재오픈하지 않고, HV-06/HV-12를 Story 2.3 / 4.3의 supporting regression evidence 또는 follow-up validation note로 분리하는 것이다. 이것은 현행 epics와 sprint change proposal을 따른 운영 추론이다.
  - [x] evidence row/template는 runbook의 기록 양식을 재사용하되, sprint close에 필요한 공통 필드를 빠짐없이 담도록 표준화한다.

- [x] sprint 운영 문서와 release baseline을 hardware gate 기준으로 정렬한다. (AC: 1, 3, 5)
  - [x] `_bmad-output/implementation-artifacts/sprint-status.yaml`의 상태 운영 메모 또는 companion artifact에서, truth-critical story는 `done` 전에 `review`에서 hardware gate를 기다린다는 규칙을 명시한다.
  - [x] `docs/release-baseline.md`에 automated build/test proof와 approved booth hardware proof가 별도 게이트라는 점을 분명히 남긴다.
  - [x] sprint review에 사용할 표 또는 체크리스트에는 최소 `automated pass`, `hardware pass`, `Go / No-Go`, `blocker`, `owner`, `evidence path` 열을 포함한다.
  - [x] 이미 각 story에 흩어져 있는 `Correct Course Note`와 HV evidence 요구사항을 한곳에서 추적 가능하게 연결한다.

- [x] No-Go 처리 규칙과 release hold 정책을 고정한다. (AC: 2, 4)
  - [x] `No-Go` 시 story를 `review`로 유지 또는 복귀시키는 조건을 checklist의 실패 처리 규칙과 일치하게 정리한다.
  - [x] failure evidence는 삭제하거나 덮어쓰지 않고, 같은 row 또는 같은 evidence package 안에 rerun 전후 결과를 이어서 남기도록 한다.
  - [x] `release blocker`, `follow-up owner`, `rerun prerequisite`, `target rerun date`를 기록하도록 운영 규칙을 확정한다.
  - [x] branch rollout / release promotion은 related gated story 중 하나라도 `No-Go` 또는 evidence missing이면 진행하지 않는다고 명시한다.

- [x] 현재 truth-critical story와 runbook 참조를 소급 정렬한다. (AC: 1, 2, 5)
  - [x] Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3 문서에 동일한 방식의 hardware gate reference가 유지되는지 확인하고, 누락되거나 표현이 제각각이면 정규화한다.
  - [x] checklist, sprint-status, release-baseline, impacted story docs 사이에서 story list와 HV mapping이 서로 다르게 적혀 있지 않게 맞춘다.
  - [x] 첫 운영 회차에서 사용할 빈 evidence row 또는 placeholder를 미리 만들어, QA / operator가 즉시 실행 결과를 기입할 수 있게 한다.

- [x] 정합성 검증 경로를 남긴다. (AC: 1, 2, 3, 4, 5)
  - [x] 문서 기반 구현이면 최소한 runbook, sprint-status, release-baseline, impacted story docs의 cross-check 절차를 completion note에 남긴다.
  - [x] 스크립트나 자동화가 추가되면 HV mapping 누락, required field 누락, gated story의 premature `done` 전이 같은 회귀를 검증하는 테스트를 함께 추가한다.

### Review Findings

- [x] [Review][Patch] Runbook global `Go / No-Go` gate omits `HV-01` and `HV-09`, so 4.x hardware gate failures are not treated as automatic `No-Go` blockers [docs/runbooks/booth-hardware-validation-checklist.md:61]
- [x] [Review][Patch] `HV-09` failure handling still routes operators to `Story 4.2 / 4.3` even though the canonical ledger assigns `HV-09` only to Story 4.2, and the governance test locks that wrong routing in place [docs/runbooks/booth-hardware-validation-checklist.md:596]
- [x] [Review][Patch] Story 1.7 still describes `HV-04` as its primary closure evidence, which contradicts the new canonical policy that Story 1.7 is supporting-only for Story 1.5 close review [C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md:165]
- [x] [Review][Patch] Story 6.2의 `현재 워크스페이스 상태` 섹션이 여전히 pre-implementation 설명을 유지해, 실제 sprint status, ledger, release baseline 정렬 결과와 모순된다 [_bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md:88]
- [x] [Review][Patch] Governance regression test still hard-codes Story 1.4 as `review`, so the canonical `Go` close row now causes `pnpm test:run src/governance/hardware-validation-governance.test.ts` to fail instead of validating the intended `Go -> done` transition [src/governance/hardware-validation-governance.test.ts:81]
- [x] [Review][Patch] Governance test only checks whether each impacted story doc contains any `Go` or `No-Go` token, so contradictory docs can still pass even when the canonical `Current hardware gate` and lower history sections disagree [src/governance/hardware-validation-governance.test.ts:106]
- [x] [Review][Patch] Story 1.5 / 1.6 / 3.2 now advertise `No-Go` + `review` at the top, but their lower completion/change-log sections still say the HV pass was closed and the story was moved to `done`, leaving the retro-aligned governance docs internally contradictory [_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md:221]
- [x] [Review][Patch] Story 6.2 and Story 4.3 still keep stale sprint-status snapshots (`Story 5.4 ready-for-dev`, `Story 4.2 ready-for-dev`, `Story 4.1 backlog`) that no longer match the current sprint ledger/status state, so the “retro aligned” context remains misleading [_bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md:89]

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 booth runtime 기능을 더 만드는 작업이 아니라, release truth를 닫는 운영 경계를 고정하는 governance story다.
- 핵심은 "자동 테스트가 통과했다"와 "실장비에서 제품 진실이 검증됐다"를 분리하는 것이다.
- Story 6.1이 branch rollout / rollback 거버넌스를 잠갔다면, Story 6.2는 어떤 story가 제품 관점 완료로 닫혀도 되는지의 증거 기준을 잠근다.

### 스토리 기반 요구사항

- epics의 Additional Requirements는 Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3이 자동 테스트 통과만으로 제품 관점 `done`이 아니라고 이미 명시한다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- PRD의 release gate는 booth `Ready`, truthful preview/completion, publication/rollback safety가 실제 제품 진실로 검증되어야 한다고 요구한다. [Source: _bmad-output/planning-artifacts/prd.md#Release Gates]
- hardware validation runbook은 `Go / No-Go`, evidence package, failure handling, BMAD 운영 메모까지 이미 제공하지만, sprint-owned evidence registry와 closure policy는 아직 느슨하다. 6.2는 이 간극을 메운다. [Source: docs/runbooks/booth-hardware-validation-checklist.md] [Source: docs/runbooks/booth-hardware-validation-checklist.md#BMAD 운영 메모]

### 선행 의존성과 구현 순서

- 이 스토리는 Story 1.4, 1.5, 1.6, 3.2, 4.2, 4.3이 이미 구현 완료 또는 review 상태라는 전제 위에서 동작한다.
- 현재 가장 안전한 구현 순서는 다음과 같다.
  - [ ] canonical gated-story 목록과 HV mapping을 먼저 고정한다.
  - [ ] sprint evidence ledger와 release baseline wording을 정리한다.
  - [ ] impacted story docs와 runbook을 소급 정렬한다.
  - [ ] 마지막에 실제 HV 회차 운영을 위한 빈 기록 행과 No-Go 처리 규칙을 남긴다.
- 실제 하드웨어 검증 실행 자체는 Story 6.2의 직접 구현 범위라기보다, 이 story가 준비시키는 follow-up 운영 활동이다.

### 현재 워크스페이스 상태

- `_bmad-output/implementation-artifacts/sprint-status.yaml`은 이제 `hardware_validation_ledger` 경로를 직접 가리킨다. canonical `Go`가 확인된 Story 1.4, 1.5, 1.6, 1.8, 3.2는 `done`으로 닫혀 있고, Story 4.2, 4.3은 `review`로 유지돼 hardware `Go` 전 premature close를 막는다. Story 5.4는 `review`이며, Story 6.2는 governance story로 `done` 상태다. [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- impacted story 문서들은 모두 `Hardware Gate Reference` 섹션으로 canonical ledger와 required HV checklist ownership을 직접 참조하고, 회차별 blocker/owner/evidence path는 sprint-owned `hardware-validation-ledger.md`에 모이도록 정렬됐다. [Source: _bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md] [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md] [Source: _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md] [Source: _bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md] [Source: _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- `docs/runbooks/booth-hardware-validation-checklist.md`는 canonical close record와 sprint review 표를 ledger가 소유한다고 명시하고, `Automated Pass`와 `Hardware Pass`를 함께 보는 운영 기준으로 정리됐다. [Source: docs/runbooks/booth-hardware-validation-checklist.md] [Source: _bmad-output/implementation-artifacts/hardware-validation-ledger.md]
- `docs/release-baseline.md`는 `Release Truth Gates` 섹션에서 automated proof와 hardware proof를 분리하고, ledger의 `Go / No-Go`와 blocker 해소 전까지 branch를 `release hold`에 두는 기준을 명시한다. [Source: docs/release-baseline.md]

### 관련 이전 스토리 인텔리전스

- Story 1.4는 false-ready 방지 evidence로 HV-02, HV-03, HV-10을 직접 요구한다. 6.2는 이 evidence를 운영 close 조건으로 승격해야 한다. [Source: _bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md]
- Story 1.6은 실카메라/helper truth를 제품 진실로 잠그는 follow-up이며, 동일하게 HV-02, HV-03, HV-10 evidence 없이는 제품 관점 완료로 닫히면 안 된다. [Source: _bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md]
- Story 1.5는 raw persistence와 truthful preview separation을 HV-04, HV-05로 닫는다. evidence package에는 `session.json`, preview asset, timing log가 함께 남아야 한다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- Story 3.2는 false-complete 방지를 HV-08, HV-11로 닫는다. `Completed`는 automated state merge가 아니라 실제 post-end truth evidence로 잠겨야 한다. [Source: _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md]
- Story 4.2와 4.3은 각각 draft leak 방지, immutable published bundle / preset drift 방지를 hardware evidence로 닫는다. [Source: _bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md] [Source: _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md]
- Story 5.3은 structured audit를 운영 회고와 retrospective review에 쓸 수 있게 만들었다. 6.2는 session truth를 audit DB로 옮기려는 것이 아니라, hardware evidence와 sprint close 메모를 운영 artifact로 연결하는 작업이다. [Source: _bmad-output/implementation-artifacts/5-3-라이프사이클-개입-복구-감사-로그-기록.md]
- Story 6.1은 safe transition rollout을 다루지만, 어떤 story가 제품 관점 완료인지 판정하는 기준은 아직 별도 거버넌스로 잠겨야 한다. [Source: _bmad-output/implementation-artifacts/6-1-지점별-단계적-배포와-단일-액션-롤백-거버넌스.md]

### 구현 가드레일

- 새로운 story status를 발명하지 않는 편이 안전하다. 현재 워크플로의 `review`를 hardware gate 대기 상태로 재사용하고, `done`은 evidence `Go` 뒤에만 허용한다.
- automated test pass와 hardware pass를 하나의 체크박스로 합치면 안 된다.
- evidence는 자유 서술 메모만 남기면 안 된다. 최소한 어떤 `session.json`, 어떤 `bundle.json`, 어떤 `timing-events.log`, 어떤 화면 캡처가 closure 근거인지 경로로 남겨야 한다.
- `No-Go`가 나오면 실패 증거를 덮어쓰거나 지우지 말고, rerun evidence를 같은 story history에 누적해야 한다.
- 이 story는 booth runtime behavior, session manifest schema, published bundle contract 자체를 다시 설계하는 자리가 아니다. 이미 존재하는 truth artifact를 sprint closure에 연결하는 것이 목적이다.

### 아키텍처 및 운영 준수사항

- release truth는 active session truth를 훼손하지 않는 범위에서만 선언되어야 한다. hardware evidence를 이유로 `session.json`이나 published bundle을 수동 수정하는 방식은 금지다. [Source: docs/runbooks/booth-hardware-validation-checklist.md#절대-금지]
- runbook이 요구하는 evidence package는 `session.json`, `timing-events.log`, `bundle.json`, `catalog-state.json`, 화면 캡처를 함께 남기는 구조여야 한다. [Source: docs/runbooks/booth-hardware-validation-checklist.md#수집-증거]
- branch rollout governance는 safe transition point를 지켜야 하며, hardware gate가 열리지 않은 상태에서 release promotion을 밀어붙이면 안 된다. 이것은 Story 6.1과 release baseline을 합친 운영 추론이다. [Source: docs/contracts/branch-rollout.md] [Source: docs/release-baseline.md#Release-Behavior-Guardrails]
- hardware validation architecture research는 실검증을 "화면 확인"이 아니라 "truth transition evidence"로 다루라고 요구한다. 6.2의 evidence template도 이 원칙을 따라야 한다. [Source: docs/runbooks/booth-hardware-validation-architecture-research.md#결론-1]
- 현재 camera helper baseline은 `docs/contracts/camera-helper-edsdk-profile.md`를 따르는 Windows 전용 Canon EDSDK helper exe다. 따라서 camera-related HV evidence row에는 최소 helper version, sdk version 또는 동등 식별자, 최근 `camera-status`/`recovery-status` 근거를 남길 수 있어야 한다. [Source: docs/contracts/camera-helper-edsdk-profile.md] [Source: docs/runbooks/booth-hardware-validation-checklist.md#EDSDK-helper-전용-사전-확인]

### 프로젝트 구조 요구사항

- 우선 수정/생성 후보 경로:
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `_bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md`
  - `_bmad-output/implementation-artifacts/` 아래 신규 hardware evidence ledger 또는 동등한 sprint-owned artifact
  - `docs/runbooks/booth-hardware-validation-checklist.md`
  - `docs/release-baseline.md`
  - `_bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md`
  - `_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md`
  - `_bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md`
  - `_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md`
  - `_bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md`
- 새 evidence ledger를 만든다면 docs/runbook이 아니라 sprint execution artifact로 두는 편이 좋다. runbook은 절차를 소유하고, sprint artifact는 회차별 pass/no-go 결과를 소유하는 구조가 더 명확하다.

### 테스트 요구사항

- 최소 필수 검증 범위:
  - gated story 목록과 HV mapping이 runbook, sprint-status, impacted story docs에서 서로 모순되지 않는다.
  - evidence template이 `story key`, `HV ID`, `result`, `evidence path`, `owner`, `blocker`를 빠짐없이 요구한다.
  - release baseline과 sprint review artifact가 `automated pass`와 `hardware pass`를 분리해 보여 준다.
  - `No-Go` 또는 evidence missing 상태에서 gated story를 `done`으로 닫는 운영 문구가 남아 있지 않다.
- 구현이 문서 갱신 중심이라면 cross-check 결과를 Completion Notes에 남긴다.
- 스크립트/YAML 검증 도구를 추가한다면 gated story premature close를 막는 회귀 테스트를 함께 둔다.

### 금지사항 / 안티패턴

- `hardware-done`, `validated-done` 같은 비공식 status를 새로 만드는 것 금지
- screenshot 몇 장만 붙이고 `session.json` / `bundle.json` / `timing-events.log` 연결을 생략하는 것 금지
- automated test pass를 release-ready와 같은 의미로 쓰는 것 금지
- runbook의 실패 처리 규칙과 sprint closure 규칙을 서로 다른 story mapping으로 유지하는 것 금지
- `No-Go` 증거 확보 전 session root, diagnostics, published bundle을 수동 수정하거나 삭제하는 것 금지
- Story 6.2 구현을 핑계로 booth runtime 기능 변경을 끼워 넣는 것 금지

### 최신 기술 확인 메모

- 이번 스토리는 새 프레임워크 도입보다 기존 planning artifact, runbook, release governance를 정합적으로 잠그는 일이 핵심이다.
- hardware gate는 이미 로컬 문서 기준으로 darktable pin, session manifest path, published bundle path, catalog state path를 전제로 하므로, 6.2는 기술 baseline을 바꾸지 않고 evidence 운영 기준만 확정하는 편이 안전하다. [Source: docs/runbooks/booth-hardware-validation-checklist.md#운영-고정값] [Source: docs/contracts/session-manifest.md] [Source: docs/contracts/preset-bundle.md]

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- 스프린트 변경 제안: `_bmad-output/planning-artifacts/sprint-change-proposal-20260327-193147.md`
- 하드웨어 검증 런북: `docs/runbooks/booth-hardware-validation-checklist.md`
- 하드웨어 검증 연구 메모: `docs/runbooks/booth-hardware-validation-architecture-research.md`
- Canon EDSDK helper 구현 프로파일: `docs/contracts/camera-helper-edsdk-profile.md`
- 릴리스 기준선: `docs/release-baseline.md`
- branch rollout 계약: `docs/contracts/branch-rollout.md`
- 세션 계약: `docs/contracts/session-manifest.md`
- 프리셋 번들 계약: `docs/contracts/preset-bundle.md`
- 게시 계약: `docs/contracts/authoring-publication.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.2: 실장비 hardware validation gate와 evidence 기반 done 정책]
- [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#Release Gates]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-006 Safe Local Packaging, Rollout, and Version Pinning]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260327-193147.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md]
- [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- [Source: _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md]
- [Source: _bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md]
- [Source: _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md]
- [Source: _bmad-output/implementation-artifacts/5-3-라이프사이클-개입-복구-감사-로그-기록.md]
- [Source: _bmad-output/implementation-artifacts/6-1-지점별-단계적-배포와-단일-액션-롤백-거버넌스.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#실패-처리-규칙]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#수집-증거]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#BMAD-운영-메모]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#절대-금지]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#EDSDK-helper-전용-사전-확인]
- [Source: docs/runbooks/booth-hardware-validation-architecture-research.md#결론-1]
- [Source: docs/contracts/camera-helper-edsdk-profile.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/contracts/preset-bundle.md]
- [Source: docs/contracts/authoring-publication.md]
- [Source: docs/contracts/branch-rollout.md]
- [Source: docs/release-baseline.md]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-03-27 20:xx +09:00 - 기존 Story 6.2, sprint change proposal, sprint-status, impacted story docs, hardware validation runbook, release baseline을 교차 검토해 story를 재작성했다.
- 2026-03-27 20:xx +09:00 - runbook scope와 approved planning artifact 사이의 gated-story 범위 차이를 확인했고, canonical mapping과 evidence registry 중심으로 정리하는 방향을 채택했다.
- 2026-03-31 10:17 +09:00 - `src/governance/hardware-validation-governance.test.ts`에 ledger/runbook/release baseline/sprint-status/impacted story docs 정합성 검증을 먼저 추가하고 실패를 확인했다.
- 2026-03-31 10:24 +09:00 - sprint-owned `hardware-validation-ledger.md`를 신설하고, sprint-status/runbook/release baseline/impacted story docs를 canonical hardware gate 정책에 맞게 정렬했다.
- 2026-03-31 10:21 +09:00 - 실제 session manifest와 helper diagnostics를 확인해 기존 partial hardware evidence를 ledger row에 반영했고, formal close row가 없는 story는 `review` 또는 `in-progress`로 재정렬했다.

### Implementation Plan

- sprint-owned hardware evidence ledger와 canonical HV mapping을 먼저 고정한다.
- sprint-status / release-baseline / impacted story docs를 같은 closure rule로 정렬한다.
- No-Go 처리 규칙과 rerun owner 기록 방식을 운영 artifact에 남긴다.

### Completion Notes List

- sprint-owned `hardware-validation-ledger.md`를 추가해 canonical HV mapping, sprint review gateboard, evidence registry, reusable evidence row template를 한곳에 고정했다.
- `sprint-status.yaml`, hardware validation runbook, release baseline, impacted story docs를 같은 close policy로 정렬해 `automated pass`와 hardware `Go / No-Go`를 분리해 보도록 만들었다.
- canonical `Go` close row가 확인된 Story 1.4, 1.5, 1.6, 1.8, 3.2는 `done`으로 닫고, Story 4.2, 4.3은 아직 `review`로 유지하도록 정렬했다.
- cross-check 절차: ledger의 canonical mapping을 기준으로 runbook scope, sprint-status 상태 규칙, release-baseline release hold 정책, impacted story `Hardware Gate Reference` 섹션이 서로 같은 story/HV ownership을 가리키는지 대조했다.
- 검증: `pnpm test:run` 전체 통과, `pnpm test:run src/governance/hardware-validation-governance.test.ts` 통과. `pnpm lint`는 이번 변경과 무관한 기존 이슈(`src/booth-shell/components/SessionPreviewImage.tsx`, `src/session-domain/selectors/current-session-previews.ts`, `src/session-domain/state/session-provider.tsx`) 때문에 실패했다.

### Change Log

- 2026-03-31 10:24:00 +09:00 - Story 6.2 구현 완료: sprint-owned hardware validation ledger 추가, runbook/release baseline/sprint-status/impacted story docs 정렬, truth-critical story pre-close policy 고정, 문서 정합성 회귀 테스트 추가

### File List

- _bmad-output/implementation-artifacts/6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md
- _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md
- _bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md
- _bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md
- _bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md
- _bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md
- docs/runbooks/booth-hardware-validation-checklist.md
- docs/release-baseline.md
- src/governance/hardware-validation-governance.test.ts
