# Preview Gate Redefinition Design

## 목적

이 문서는 preview-track의 공식 제품 게이트를 재정의하고,
관련 문서 해석을 한 기준으로 다시 맞추기 위한 설계 메모다.

이번 변경의 핵심은 다음 두 가지다.

1. 공식 제품 게이트를 `preset-applied visible <= 3000ms` 단일 기준으로 재정의한다.
2. `sameCaptureFullScreenVisibleMs`는 더 이상 release gate가 아니라
   `first-visible/reference/comparison metric`으로 역할을 낮춘다.

## 변경 배경

현재 문서 집합에는 preview-track release sign-off를

- `sameCaptureFullScreenVisibleMs <= 3000ms`
- `originalVisibleToPresetAppliedVisibleMs <= 3000ms`

의 이중 게이트로 읽는 표현이 넓게 퍼져 있다.

하지만 사용자 판단 기준은 이제 전체 시간이나 first-visible이 아니라,
**프리셋이 실제 적용된 결과가 고객에게 보여지는 시간**
즉 `preset-applied visible`만을 공식 제품 게이트로 본다.

따라서 문서 전체를 아래처럼 다시 정렬해야 한다.

- release gate:
  - `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
- reference/comparison metric:
  - `sameCaptureFullScreenVisibleMs`
- diagnostic/product feel metric:
  - `first-visible`, `pending preview`, `recent-session strip` 등

## 설계 결정

### 1. 공식 제품 게이트

공식 preview-track release gate는 아래 하나만 사용한다.

- `originalVisibleToPresetAppliedVisibleMs <= 3000ms`

이를 문서상 표현할 때는 제품 언어로

- `preset-applied visible <= 3000ms`

라고 함께 적는다.

### 2. first-visible 계열 지표의 역할

`sameCaptureFullScreenVisibleMs`는 계속 유지하되,
아래 용도로만 남긴다.

- 체감 속도 비교
- 경로/레인 회귀 탐지
- baseline 간 matched comparison
- resident first-visible line 검증 근거

즉 이 값이 좋아도 release success를 의미하지 않는다.

### 3. route decision 해석 변경

actual-primary-lane의 `No-Go` 판단은 앞으로

- `preset-applied visible 3초 게이트`를 반복적으로 닫지 못했다

로 해석한다.

기존처럼 dual gate를 닫지 못했다는 표현은 제거하거나
역사적 문맥으로만 남긴다.

### 4. old resident first-visible baseline 재검증 목적

old `resident first-visible` line의 재검증 목적은 release winner 선발이 아니다.

이번 기준 변경 이후 이 라인의 목적은 아래 두 가지다.

1. `first-visible/reference metric` baseline을 다시 안정적으로 닫는다.
2. 같은 세션 패키지에서 `preset-applied visible`과의 관계를 다시 읽을 수 있게 한다.

즉 이 라인은

- release proof lane
- 공식 gate 충족 증명

이 아니라

- baseline evidence lane
- comparison candidate lane

으로 문서화한다.

### 5. story 해석

이번 기준 아래 story 해석은 다음처럼 고정한다.

- `1.30`
  - actual-primary-lane 재검증 기록
  - 현재는 `preset-applied visible` 기준 `No-Go` 판단 근거 문서
- `1.31`
  - 열지 않음
  - success-side default/rollback gate로만 남긴다
- `1.13`
  - 계속 blocked
- `1.26`
  - 다음 공식 판단 후보
  - reserve path를 열지 여부를 여기서 판단한다

## 문서 수정 범위

아래 범위는 이번 변경에서 함께 정렬한다.

- 공식 기준 문서
  - `docs/release-baseline.md`
  - `docs/preview-architecture-history-and-agent-guide.md`
- runbook
  - `docs/runbooks/current-actual-lane-handoff-20260419.md`
  - `docs/runbooks/current-preview-gpu-direction-20260419.md`
  - `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`
  - `docs/runbooks/preview-track-route-decision-20260418.md`
- planning / artifact
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`
  - `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
  - `_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md`
  - `_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md`

필요 시 관련 문서의 주변 설명도 함께 바꾼다.
단, 이번 요청과 직접 무관한 기술 설계까지 재작성하지는 않는다.

## 다음 경로 판단 원칙

다음 경로 판단은 `문서상 gate`와 `실기기 evidence package`를 함께 본다.

판단 위치:

- 기준 해석:
  - `docs/runbooks/preview-track-route-decision-20260418.md`
- 현재 track handoff:
  - `docs/runbooks/current-actual-lane-handoff-20260419.md`
- baseline rerun 실행 기준:
  - `docs/runbooks/old-first-visible-cpu-baseline-rerun-20260419.md`
- ledger / sprint 상태:
  - `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
  - `_bmad-output/implementation-artifacts/sprint-status.yaml`

판단 방식:

1. actual-primary-lane이 `preset-applied visible <= 3000ms`를 닫는지 본다.
2. 반복 evidence가 계속 실패면 `1.30`은 `No-Go`로 유지한다.
3. old resident first-visible line은 baseline evidence lane으로 다시 닫는다.
4. 그 결과로도 official gate를 현실적으로 닫기 어렵다면,
   다음 공식 후보인 `1.26 reserve path` 개시 여부를 판단한다.

## old resident first-visible line baseline 재검증 절차

1. old line의 one-session package를 다시 닫는다.
2. 같은 세션에서 아래를 함께 남긴다.
   - first-visible owner
   - same-capture first-visible arrival
   - preset-applied visible close
   - `Preview Waiting` truth 유지 여부
   - wrong-capture 0 여부
3. 여기서 `sameCaptureFullScreenVisibleMs`는 reference metric으로 기록한다.
4. 공식 합격 판정은 오직 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`로 읽는다.
5. baseline evidence가 정리되면 GPU comparison 또는 reserve path 판단으로 넘어간다.

## 사용자에게 최종 설명할 내용

최종 설명은 코드가 아니라 제품 관점으로만 정리한다.

- 새 공식 제품 게이트
- 왜 old line을 baseline으로 다시 닫는지
- 다음 경로 판단을 어디서 어떻게 하는지
- 지금 처리해야 할 story가 무엇인지
