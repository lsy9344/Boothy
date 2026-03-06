# Boothy Greenfield Rebuild Research

작성일: 2026-03-06  
작성자: Codex  
문서 용도: LLM 및 숙련 개발자가 "기존 구현을 수정하지 않고", 레퍼런스 2개를 기반으로 Boothy를 백지부터 다시 설계/구현할 때 사용하는 기준 문서

## 0. Decision Lock

이 문서는 다음 결정을 이미 확정된 전제로 둔다.

1. 현재 `apps/boothy/src-tauri/src/camera/*`, `apps/camera-sidecar/*` 구현은 수정 대상이 아니다.
2. 기존 구현은 재사용 후보가 아니라 실패 패턴과 요구사항을 알려주는 레거시 참고물이다.
3. 새 시스템은 레퍼런스 2개를 기반으로 greenfield로 다시 시작한다.
4. 새 시스템의 목적은 "카메라 제어 엔진 + 사진 편집/표시 앱"의 책임을 명확히 분리하는 것이다.

따라서 이 문서는 in-place refactor 문서가 아니다.  
이 문서는 greenfield rebuild 문서다.

## 1. Scope of the New Build

### 1.1 제품의 실제 목적

새로 만들 시스템의 본질은 다음이다.

- 고객은 버튼 또는 리모컨으로 촬영만 한다.
- 카메라는 PC로 파일을 전송한다.
- 편집 앱은 새 파일을 ingest 한다.
- 미리 정한 필터를 자동 적용한다.
- 고객은 결과를 즉시 본다.

즉, 제품의 핵심은 "고객용 실시간 결과 표시"다.  
제품의 핵심은 "복잡한 카메라 제어 UI"가 아니다.

### 1.2 새 시스템에서 유지해야 하는 제품 요구사항

- Windows desktop
- Canon camera tethering
- 오프라인 우선
- 세션 폴더 계약 유지
  - `Raw/`: 카메라 원본 수신
  - `Jpg/`: 결과/내보내기
- 고객 모드 / 관리자 모드 분리
- Boothy 내부 필터 적용 및 결과 표시

### 1.3 새 시스템에서 버려야 하는 암묵적 목표

다음은 greenfield 설계에서 목표로 삼지 않는다.

- 현재 카메라 sidecar 구조를 최대한 보존하는 것
- 현 Rust IPC 클라이언트의 로직을 이식하는 것
- 현 UI readiness 계산 방식을 재현하는 것
- Canon SDK 세부 상태를 고객 화면에 그대로 반영하는 것

## 2. Reference-Driven Rebuild Strategy

### 2.1 Reference A: 사진 편집/UI 레이어

기준 역할:

- 사진 보기
- 필터/프리셋 적용
- 결과 렌더링
- 세션 라이브러리 표시

현재 기준 레퍼런스:

- RapidRAW 계열 UI/편집 구조

이 레퍼런스는 "결과 소비자" 역할의 기준점이다.

### 2.2 Reference B: 카메라 엔진 레이어

기준 역할:

- Canon camera detect
- capture trigger
- tethered transfer
- camera status surface
- SDK/session recovery

우선순위:

1. `digiCamControl` 기반 접근
2. Canon 전용 참고 구현으로 `EosMonitor`

이 레퍼런스는 "카메라 제어 엔진" 역할의 기준점이다.

### 2.3 핵심 원칙

새 시스템은 "현재 Boothy를 계속 고치기"가 아니라 다음 조합으로 본다.

- 편집/UI 앱: Reference A 계열
- 카메라 엔진: Reference B 계열
- 둘을 얇은 경계로 연결

즉, 새 시스템은 기존 Boothy camera stack의 연장선이 아니라 "editor host + camera engine" 재구성이다.

## 3. Legacy Code Status

### 3.1 현재 구현의 취급 방식

현재 구현은 다음 용도로만 읽는다.

1. 실패 패턴 파악
2. 제품 요구사항 추출
3. 세션 폴더/파일 계약 추출
4. 운영 UX 제약 파악

현재 구현은 다음 용도로 읽지 않는다.

1. 재사용 가능한 카메라 아키텍처 베이스
2. 옮겨 심을 상태 머신
3. 유지해야 할 IPC/상태 동기화 패턴

### 3.2 재사용 금지 영역

다음은 greenfield build에서 직접 계승하지 않는다.

- `apps/boothy/src-tauri/src/camera/ipc_client.rs`의 polling / restart heuristic
- `apps/boothy/src/App.tsx`의 snapshot/hint/pull 혼합 상태 처리
- 현재 sidecar lifecycle orchestration
- current hybrid camera status model

### 3.3 참고만 할 가치가 있는 영역

다음은 개념 또는 계약 차원에서만 참고한다.

- 세션 폴더 구조
- 파일 ingest 흐름
- preset snapshot 적용 시점
- 고객/관리자 모드 분리
- 현장 로그에서 드러난 failure mode

## 4. Postmortem: Why the Previous Architecture Failed

### 4.1 failure summary

기존 구조는 다음 이유로 장기 유지 비용이 과도했다.

- 같은 종류의 상태를 여러 계층이 동시에 판단했다.
- 상태 표시 경로가 촬영 경로를 방해했다.
- 복구 권한이 여러 계층에 분산되었다.
- `capture accepted`와 `file arrived`를 충분히 분리하지 못했다.

### 4.2 three-brain model

기존 구조는 사실상 아래 세 계층이 같은 camera truth를 각각 재해석했다.

- sidecar
- Rust backend
- React UI

이 패턴은 새 시스템에서 절대 재현하면 안 된다.

### 4.3 문서 검토 결과의 의미

기존 문서 검토 결과는 다음으로 요약된다.

- 에픽 문서 방향 자체가 완전히 틀린 것은 아니었다.
- 하위 설계에서 Rust에 camera orchestration 책임이 과도하게 들어갔다.
- 이후 single-source-of-truth를 도입하려 했지만 기존 fallback을 충분히 제거하지 못했다.
- 구현은 그 위에 heuristic을 더 올려 더 꼬였다.

이 결론은 "기존 구조를 다듬으면 된다"가 아니라 "이 구조를 기반으로 다시 쌓지 말라"는 뜻으로 읽어야 한다.

## 5. Greenfield Architecture Goal

### 5.1 새 시스템의 상위 구조

권장 상위 구조는 다음과 같다.

```text
[Customer/Admin UI]
        |
        v
[Editor Host App]
  - session manager
  - file watcher / ingest
  - preset application
  - result rendering
        |
        +----------------------+
        |                      |
        v                      v
[Camera Adapter]         [Session Filesystem]
        |                      ^
        v                      |
[Camera Engine] ---capture---> Raw/
```

### 5.2 역할 분리

#### Editor Host App

책임:

- 세션 생성/선택
- `Raw/` 감시
- ingest
- preset 적용
- 결과 표시
- 고객/관리자 화면 분기

비책임:

- Canon SDK session management
- camera reconnect heuristic
- hardware truth 판단

#### Camera Engine

책임:

- camera detect
- capture
- transfer
- camera truth 판단
- SDK/session recovery

비책임:

- image ingest
- filter application
- gallery management

#### Camera Adapter

책임:

- Editor Host와 Camera Engine 사이의 최소 command/result 경계 제공
- capture command 전달
- engine status를 앱이 이해할 수 있는 최소 모델로 정규화

비책임:

- camera 상태를 새로 추론
- fallback polling 기반 복구 엔진 역할

## 6. Preferred Integration Model

### 6.1 filesystem-first integration

새 시스템은 가능하면 filesystem-first로 붙는 것이 좋다.

의미:

1. Camera Engine은 세션 `Raw/` 폴더로 파일을 쓴다.
2. Editor Host는 `Raw/` 폴더를 감시한다.
3. 최종 성공 기준은 새 파일의 실제 도착이다.

장점:

- 엔진 내부 이벤트 신뢰성에 덜 의존
- transfer 완료를 앱 내부 추측 상태가 아니라 파일 존재로 검증 가능
- camera/app coupling 감소

### 6.2 command channel은 최소화

Editor Host가 Camera Engine에 요구하는 command는 최소화한다.

권장 최소 command:

- `initializeSession(rawPath)`
- `capture()`
- `getAdminDiagnostics()` 또는 동등 기능
- `restartEngine()` 또는 동등 기능

권장 status feed:

- `ready`
- `busy`
- `transferring`
- `error`

여기서 status는 고객 화면을 위한 단순 모델이어야 한다.

### 6.3 capture success semantics

새 시스템에서 `capture()`의 성공은 "셔터 요청이 받아들여졌다" 수준의 중간 의미다.  
진짜 성공은 다음이다.

1. 새 파일이 `Raw/`에 도착
2. 파일 안정화 완료
3. ingest 성공
4. preset 적용 성공
5. 결과 표시 성공

## 7. Single-Owner Rules for the New System

### 7.1 camera truth

소유자: Camera Engine only

예:

- 카메라가 연결되었는가
- 지금 촬영 가능한가
- transfer 중인가
- SDK/session이 정상인가

### 7.2 process / adapter truth

소유자: Camera Adapter / Host backend

예:

- engine 프로세스가 응답하는가
- adapter 통신 경로가 살아있는가

중요:

process truth로 camera truth를 재구성하지 않는다.

### 7.3 file pipeline truth

소유자: Editor Host

예:

- `Raw/` 새 파일 감지
- 파일 안정화 완료
- ingest 성공
- preset 적용 성공

### 7.4 customer-visible state

소유자: UI 표현 계층

권장 고객 상태:

- `ready`
- `capturing`
- `importing`
- `error`

중요:

이 상태는 "카메라 내부 저수준 상태"가 아니라 "부스가 지금 고객에게 약속할 수 있는 경험 상태"다.

## 8. What the New Build Must Not Repeat

### 8.1 금지 패턴

- 카메라 상태를 engine, host backend, UI가 동시에 추론하는 것
- status polling이 capture path를 방해하는 것
- timeout heuristics를 여러 계층에 나눠 넣는 것
- `statusHint -> getStatus -> restart -> capture retry` 같은 루프를 기본 흐름으로 삼는 것
- destination/session 준비를 여러 계층에서 중복 적용하는 것
- UI가 stale snapshot과 pull 결과를 섞어 hardware readiness를 재계산하는 것

### 8.2 금지 목표

- current sidecar의 구조적 연속성을 확보하는 것
- 현재 IPC client의 복구 전략을 새 시스템의 베이스로 삼는 것
- "현 코드와 최대 호환"을 중요한 목표로 두는 것

## 9. Build Recommendation

### 9.1 recommended path

현재 결정 조건을 반영하면 가장 바람직한 방향은 다음이다.

1. Editor/UI는 RapidRAW 계열 레퍼런스를 기준으로 새로 구성
2. Camera Engine은 `digiCamControl` 계열 접근을 1순위로 검토
3. 둘 사이는 thin adapter로 연결
4. success truth는 파일 도착과 ingest 완료로 판단

### 9.2 why this is the best fit

이 방향이 가장 적합한 이유는 다음과 같다.

- 사용자 요구사항이 복잡한 camera UI가 아니라 결과 확인 중심이기 때문
- camera complexity를 제품 코어에서 분리할 수 있기 때문
- 기존 hybrid 구조의 실패 원인을 그대로 가져오지 않기 때문
- file boundary가 명확해 integration이 단순해지기 때문

### 9.3 fallback path

만약 `digiCamControl` 채택이 어렵다면 다음이 차선책이다.

- 현재 sidecar를 고쳐 쓰지 말고
- Canon 전용 최소 camera engine을 새로 설계
- 기능은 detect/capture/transfer/status push까지만 허용

중요:

이 경우에도 "현 sidecar를 점진적으로 고친다"가 아니라 "새 최소 엔진을 다시 만든다"가 기준이다.

### 9.4 why this architecture is useful and effective

이 아키텍처 제안은 단순히 "깔끔해 보이는 구조"라서 추천하는 것이 아니다.  
다음과 같은 실질적 효용이 있기 때문에 추천한다.

#### A. 문제의 중심을 제품 코어에서 분리한다

현재 가장 어려운 영역은 Canon SDK, USB tethering, reconnect, transfer, busy/retry 같은 하드웨어 통합 영역이다.  
이 영역은 편집 앱의 핵심 가치가 아니다.

`editor host + camera engine` 분리는 이 복잡도를 제품 코어 밖으로 밀어낸다.  
그 결과, 편집/UI 앱은 세션, ingest, preset, 결과 표시라는 본질에 집중할 수 있다.

#### B. 실패 표면적이 줄어든다

현재 구조는 상태 판단, recovery, capture orchestration이 여러 계층에 분산되어 있다.  
새 구조는 truth domain마다 소유자를 하나로 제한한다.

효과:

- 상태 충돌 감소
- 중복 retry/restart 감소
- capture path와 status path 간섭 감소

#### C. 최종 성공 기준이 명확해진다

새 구조는 `capture accepted`보다 `file arrived in Raw/`를 더 강한 truth로 사용한다.

이 방식은 현장 품질 관점에서 유리하다.

- 셔터 요청 수락과 실제 파일 도착을 구분 가능
- 고객이 보는 결과와 시스템의 성공 기준이 일치
- transfer 관련 버그를 control-plane 추측이 아니라 파일 기준으로 검증 가능

#### D. 테스트 가능성이 높아진다

camera engine과 editor host가 분리되면 테스트 전략도 분리된다.

- engine: detect/capture/transfer 상태 테스트
- editor host: file arrival/ingest/preset/render 테스트
- end-to-end: `Raw/` 파일 도착을 기준으로 검증

즉, 새 구조는 "카메라 이벤트를 맞게 흉내 내는 테스트"보다 "실제 제품 성공 기준을 검증하는 테스트"를 쓰기 쉬워진다.

#### E. 카메라 엔진 교체 가능성이 생긴다

새 구조는 camera engine을 replaceable component로 다룬다.

효과:

- 초기에 `digiCamControl` 기반으로 시작
- 이후 필요하면 Canon 전용 최소 엔진으로 교체 가능
- 앱 코어가 특정 SDK binding 구조에 고정되지 않음

#### F. 고객 UX와 관리자 UX를 자연스럽게 분리한다

고객은 단순 상태만 보면 된다.

- `ready`
- `capturing`
- `importing`
- `error`

관리자는 진단을 더 자세히 볼 수 있다.

- 엔진 프로세스 상태
- 마지막 오류
- 카메라 모델
- 재시작 가능 여부

이 분리는 현재처럼 "관리자에게 필요한 상태 복잡도"가 고객 UI로 새어 나오는 문제를 막는다.

### 9.5 explicit review prompts for other agents

다른 에이전트가 이 문서를 검토할 때는 아래 주장 단위로 평가해야 한다.

#### Claim 1. 이 구조는 기존 three-brain 문제를 재생산하지 않는다

검토 질문:

- camera truth의 단일 소유자가 실제로 정의되어 있는가?
- backend와 UI가 camera truth를 다시 계산할 여지가 남아 있는가?
- fallback이 주 경로로 승격될 위험이 있는가?

#### Claim 2. 이 구조는 현장 신뢰성을 높인다

검토 질문:

- capture 성공 기준이 실제 파일 도착 기준으로 귀결되는가?
- status path가 capture path를 방해할 여지가 제거되었는가?
- transfer 누락/지연을 파일 기반으로 검증할 수 있는가?

#### Claim 3. 이 구조는 MVP 구현 난이도를 낮춘다

검토 질문:

- camera engine이 외부 reference를 활용해 충분히 대체 가능한가?
- editor host가 과도한 camera orchestration 책임을 다시 갖지 않는가?
- 초기 구현 범위를 최소 command/status/filesystem contract로 제한했는가?

#### Claim 4. 이 구조는 운영성과 디버깅 가능성을 높인다

검토 질문:

- 고객 상태 모델이 충분히 작고 안정적인가?
- 관리자 진단 모델이 별도로 존재하는가?
- 문제 발생 시 어느 계층이 책임자인지 추적이 쉬운가?

#### Claim 5. 이 구조는 미래 교체 비용을 줄인다

검토 질문:

- camera engine 교체 시 editor host 수정량이 제한되는가?
- 앱 코어가 특정 SDK 또는 특정 IPC 방식에 과도하게 결합되지 않는가?
- filesystem boundary와 minimal adapter contract가 유지되는가?

다른 에이전트는 위 다섯 claim에 대해 각각 `accept`, `concern`, `reject`로 판단하는 것이 바람직하다.

## 10. Greenfield Build Boundaries

### 10.1 새 시스템에 가져갈 것

- 제품 요구사항
- 세션 폴더 계약
- 고객/관리자 UX 분리 원칙
- ingest / preset / export 개념
- 현장 장애 로그에서 얻은 failure knowledge

### 10.2 새 시스템에 가져가지 않을 것

- current camera IPC stack
- current sidecar lifecycle logic
- current frontend camera readiness logic
- current camera status reconciliation heuristic

### 10.3 "참고하되 복사하지 말 것"

다음은 구조가 아니라 아이디어만 참고한다.

- event naming 방식
- diagnostics surface 방식
- session destination 개념
- 관리자 복구 액션의 존재 자체

## 11. Reading Order for Future LLM Work

새 시스템을 설계/구현할 LLM은 다음 순서로 읽는 것이 좋다.

1. `refactoring/research-codex.md`
2. `work/capture.md`
3. `work/tethering.md`
4. `work/camera_connection.md`
5. 제품 요구사항 관련 story/epic 문서
6. `reference` 폴더의 편집/UI 레퍼런스
7. 채택 후보 camera engine 레퍼런스
8. 마지막으로 현재 구현 코드

중요:

현재 구현 코드는 "무엇을 만들지"보다 "무엇을 다시 만들면 안 되는지" 확인하기 위해 늦게 읽는 편이 낫다.

## 12. New-System Invariants

greenfield build 동안 다음 규칙은 깨지면 안 된다.

### 12.1 one owner per truth domain

같은 truth domain에 최종 판단자는 하나여야 한다.

### 12.2 file arrival beats control-plane optimism

command 응답보다 파일 도착이 더 강한 truth다.

### 12.3 customer UI reflects booth state, not SDK internals

고객 UI는 Canon SDK 내부 상태를 번역해 보여주는 화면이 아니다.

### 12.4 admin diagnostics may be detailed, customer state must stay small

관리자 진단은 풍부해도 되지만 고객 상태 모델은 작아야 한다.

### 12.5 camera engine is replaceable

새 구조는 camera engine을 교체 가능하게 설계해야 한다.

즉, app core는 특정 Canon SDK binding 구조에 잠기면 안 된다.

## 13. Practical Implications for the Next LLM

다음 작업을 하는 LLM은 아래처럼 행동해야 한다.

### 13.1 해야 할 일

- 현재 앱을 patching 대상으로 보지 말 것
- 새 app core와 새 camera integration boundary를 먼저 정의할 것
- camera engine contract를 최소 command/status/filesystem 관점에서 정의할 것
- customer state model을 작게 유지할 것
- admin diagnostics는 별도 모델로 분리할 것

### 13.2 하지 말아야 할 일

- `ipc_client.rs`를 개선해 새 구조를 만들려 하지 말 것
- 현재 sidecar와 호환되는 방향으로 인터페이스를 먼저 고정하지 말 것
- old status event model을 그대로 유지하려 하지 말 것
- pull/push/hint 혼합 상태 모델을 다시 설계하지 말 것

## 14. Final Conclusion

이번 결정의 핵심은 "리팩토링"이 아니라 "재시작"이다.

기존 Boothy camera stack은 참고 자료다.  
새 시스템의 출발점은 기존 코드가 아니라 두 개의 레퍼런스다.

따라서 앞으로의 설계 질문은 다음처럼 바뀌어야 한다.

- "현 sidecar를 어떻게 안정화할까?"가 아니라
- "편집 앱과 카메라 엔진을 어떤 최소 경계로 연결할까?"

- "UI가 카메라 상태를 어떻게 더 정확히 알까?"가 아니라
- "고객 경험에 필요한 상태를 얼마나 작게 정의할까?"

- "현재 IPC 구조를 어떻게 고칠까?"가 아니라
- "새 camera engine contract를 어떻게 최소화할까?"

요약하면 다음과 같다.

> 새 시스템은 기존 camera stack을 수리하는 프로젝트가 아니라,  
> 편집/UI 레이어와 카메라 엔진 레이어를 분리한 새 제품을 다시 만드는 프로젝트다.

## 15. Primary References

### 15.1 legacy observation sources

- `work/capture.md`
- `work/tethering.md`
- `work/camera_connection.md`

### 15.2 product / architecture docs

- `docs/decisions/adr-001-camera-integration.md`
- `docs/architecture/component-architecture.md`
- `docs/architecture/camera-status-realtime.md`
- `docs/architecture/api-design-and-integration.md`
- `docs/stories/epic-1-unified-boothy.md`
- `docs/stories/epic-2-session-timeline-smart-export.md`
- `docs/stories/epic-3-storage-health-disk-space-guardrails.md`
- `docs/stories/epic-4-real-camera-hardware-integration.md`

### 15.3 current implementation to study late

- `apps/boothy/src/App.tsx`
- `apps/boothy/src/cameraReadiness.ts`
- `apps/boothy/src-tauri/src/camera/ipc_client.rs`
- `apps/camera-sidecar/Program.cs`
- `apps/camera-sidecar/Camera/RealCameraController.cs`

### 15.4 external / reference candidates

- RapidRAW: <https://github.com/CyberTimon/RapidRAW>
- digiCamControl: <https://github.com/dukus/digiCamControl>
- EosMonitor: <https://github.com/Helge07/EosMonitor>

### 15.5 open-source reference inventory from prior research

이 섹션은 사용자가 사전에 수집한 오픈소스 후보군을 greenfield rebuild 관점으로 다시 분류한 목록이다.

#### A. Windows desktop camera applications

1. `digiCamControl`
   - URL: <https://github.com/dukus/digiCamControl>
   - 역할: 가장 현실적인 camera engine 후보
   - 이유: Windows tethering app, Canon 지원, 외부 제어 단서 존재
   - 본 문서에서의 위치: primary candidate

2. `EosMonitor`
   - URL: <https://github.com/Helge07/EosMonitor>
   - 역할: Canon 전용 참고 구현
   - 이유: Canon DSLR/DSLM 제어, LiveView, parameter setting, download 흐름 참고 가능
   - 본 문서에서의 위치: secondary reference / Canon-specific example

#### B. Canon EDSDK wrapper / library references

3. `canon-sdk-java`
   - URL: <https://github.com/Blackdread/canon-sdk-java>
   - 역할: Java 기반 Canon EDSDK framework
   - 이유: capture, download, parameter setting, live view, multi-camera 패턴 참고 가능
   - 본 문서에서의 위치: event / control abstraction 참고용

4. `EDSDK-cpp`
   - URL: <https://github.com/hezhao/EDSDK-cpp>
   - 역할: C++ 기반 Canon EDSDK wrapper
   - 이유: keepalive, liveview, tethering, multi-camera 예제 존재
   - 본 문서에서의 위치: low-level native wrapper 설계 참고용

5. `edsdk-python`
   - URL: <https://github.com/Jiloc/edsdk-python>
   - 역할: Python 기반 Canon EDSDK wrapper
   - 이유: 별도 프로세스 기반 camera service 패턴 검토에 유용
   - 본 문서에서의 위치: process boundary / service split 참고용

6. `edsdk-processing`
   - 역할: EDSDK 사용 예제군
   - 본 문서에서의 위치: low-level call pattern 참고용

7. `edsdk4j`
   - 역할: Java 계열 EDSDK 예제군
   - 본 문서에서의 위치: event / callback / camera control 패턴 참고용

#### C. Rust / PTP / lower-level transport references

8. `cam` crate
   - URL: <https://lib.rs/crates/cam>
   - 역할: pure Rust camera/PTP 설계 참고
   - 이유: vendor extension와 event model 방향성 참고 가능
   - 본 문서에서의 위치: exploratory only

9. `mtp-rs`
   - URL: <https://github.com/vdavid/mtp-rs>
   - 역할: Rust MTP/PTP 구현 참고
   - 이유: pure Rust transport 구현을 검토할 때 기반 지식 제공
   - 본 문서에서의 위치: exploratory only

### 15.6 reference prioritization for the greenfield rebuild

새 시스템 기준 우선순위는 다음과 같다.

#### Tier 1: 직접 채택 후보

- `digiCamControl`
- RapidRAW 계열 편집/UI 레퍼런스

#### Tier 2: 설계 참고 후보

- `EosMonitor`
- `canon-sdk-java`
- `EDSDK-cpp`
- `edsdk-python`

#### Tier 3: 탐색 전용 후보

- `cam`
- `mtp-rs`
- `edsdk-processing`
- `edsdk4j`

이 우선순위의 의미는 다음과 같다.

- Tier 1은 제품 구조의 출발점으로 사용 가능
- Tier 2는 구현 패턴, event 흐름, SDK wrapping 방식 참고용
- Tier 3는 직접 채택보다 아이디어/기술 조사 성격이 강함
