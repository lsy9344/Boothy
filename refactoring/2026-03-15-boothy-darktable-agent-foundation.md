---
title: Boothy Darktable Agent Foundation
date: 2026-03-15
author: Codex
status: Draft
purpose: Single authoritative agent-facing foundation for shifting Boothy toward a darktable-based preset pipeline
required_reading:
  - refactoring/2026-03-15-boothy-darktable-agent-foundation.md
  - reference/darktable/README.md
  - docs/plans/2026-03-14-digicamcontrol-helper-integration.md
source_documents:
  - research/camera-status-integration-from-digicamcontrol.md
  - docs/refactoring/research-codex.md
  - docs/research-checklist-2026-03-07-boothy-greenfield.md
absorbed_history:
  - history/2026-03-14-camera-debug-log.md
supersedes:
  - refactoring/2026-03-15-boothy-prd-foundation.md
  - refactoring/2026-03-15-darktable-course-correction.md
---

# Boothy Darktable Agent Foundation

## 1. 문서 목적

이 문서는 Boothy 리팩토링 방향을 에이전트가 동일하게 해석하도록 만들기 위한 단일 기준 문서다.

이 문서는 다음 역할을 동시에 수행한다.

- PRD foundation 문서의 제품 정의와 잠금 결론 제공
- darktable 채택 이후의 코스 변경 지시 제공
- 현재 코드베이스 기준의 변경 지점과 금지사항 명시
- 여러 에이전트가 병렬 작업할 때의 write scope 분리

이 문서를 읽는 에이전트는 별도 판단으로 아키텍처 방향을 바꾸면 안 된다.

## 2. 필수 선행 문서

에이전트는 최소 아래 문서를 먼저 읽어야 한다.

1. `refactoring/2026-03-15-boothy-darktable-agent-foundation.md`
2. `reference/darktable/README.md`
3. `docs/plans/2026-03-14-digicamcontrol-helper-integration.md`

이 순서를 바꾸면 안 되는 이유:

- 현재 문서가 제품 목표와 작업 경계를 고정한다
- `reference/darktable/README.md`가 preset truth source와 CLI apply 전제를 고정한다
- helper integration 문서는 무엇이 임시 구조인지 알려준다

## 3. 최종 잠금 결론

현재까지의 대화를 기준으로 한 최종 결론은 다음과 같다.

> **Boothy의 최종 제품 구조는 `Booth Runtime App + darktable 기반 Preset Authoring/Apply 경계 + Stateful Camera Service`다.**

이 결론의 의미:

- 고객이 쓰는 메인 제품은 `Booth Runtime App`이다
- RAW 보정 UI는 고객 런타임의 핵심 기능이 아니다
- RAW 보정 기능은 운영자/개발자가 preset을 만드는 내부 workflow에 집중된다
- 운영자 기능과 프리셋 관리 surface는 같은 패키지 안에서 관리자 비밀번호 인증 후에만 노출될 수 있다
- preset authoring과 preset apply의 primary truth source는 `darktable`다
- booth runtime은 가능하면 같은 `darktable` apply path로 결과를 재현한다
- 카메라 상태와 촬영 진실값은 별도 `Stateful Camera Service`가 소유한다
- `CameraControlCmd.exe`를 매번 새 프로세스로 직접 실행하는 구조는 최종 구조가 아니다

## 4. 제품 정의

Boothy는 Windows 기반 포토부스 제품이다. 핵심 목표는 아래 두 가지를 동시에 만족하는 것이다.

1. 고객이 촬영 준비, 촬영, 종료, 결과 인계를 혼란 없이 수행할 수 있어야 한다
2. 카메라 연결, 촬영, 전송, RAW 처리, 세션 저장이 현장에서 안정적으로 동작해야 한다

즉, 이 제품의 핵심은 `RAW 에디터`가 아니라 `무인 포토부스 운영 제품`이다.

## 5. 제품 구성요소

### 5.1 Booth Runtime App

현장에서 고객과 운영자가 실제로 사용하는 메인 앱이다.

주요 책임:

- 이름 + 연락처 뒤4자리 입력 기반 booth alias 체크인
- 세션 생성
- 프리셋 선택
- 촬영 가능 여부 안내
- 최신 사진과 썸네일 표시
- 사진 삭제
- 종료 시각 안내
- preview render 진행 상태 표시
- final export 진행 상태 표시
- 완료 후 booth alias 인계
- 관리자 비밀번호 인증 후 운영자/프리셋 관리 메뉴 진입
- 운영자용 간단 진단 표시

고객에게 full RAW 편집 UI를 노출하지 않는다.

### 5.2 Preset Authoring Tool

프리셋 제작과 조정에 쓰는 내부용 도구다.

주요 책임:

- 기준 RAW를 불러와 look 조정
- 명암, 조도, 대비, 색감, 노이즈 억제 등 초기 세팅 확정
- preset 저장
- preset 미리보기 검수
- runtime이 읽을 수 있는 sidecar/XMP/manifest artifact export

채택 방향:

- 기준 엔진은 `darktable`
- 운영자는 `darktable`에서 look을 확정
- Boothy의 preset catalog는 이름보다 artifact를 기준으로 동작

### 5.3 Render Apply Worker

저장된 프리셋을 실제 RAW에 적용하는 headless 처리 경계다.

주요 책임:

- ingest된 RAW를 render job으로 큐잉
- pinned darktable version으로 apply
- preview render와 final render 분리
- 출력 파일 경로와 완료 이벤트를 runtime에 전달
- retry와 오류 코드 제공

이 경계는 camera worker와 분리되어야 한다.

### 5.4 Stateful Camera Service

카메라 연결과 촬영을 전담하는 별도 백그라운드 프로세스다.

주요 책임:

- 카메라 연결 감지
- 카메라 상태 유지
- busy 상태 관리
- capture 요청 수락/거절
- 전송 완료 확인
- 재연결 및 회복
- 오류 코드와 진단 정보 제공

고객은 직접 보지 않는다.

## 6. Reference 재사용 원칙

### 6.1 `reference/camerafunction/digiCamControl-2.0.0`

용도:

- 카메라 상태/통신/테더링/제어 비교 reference

재사용 대상:

- 카메라 연결 흐름
- 촬영 흐름
- 파일 전송 흐름
- Canon seam
- stateful camera ownership 방식
- Windows 현장 안정성 비교 기준

비채택 대상:

- WPF UI 전체
- PhotoBooth 앱 전체
- CameraControlCmd.exe 직접 호출을 최종 구조로 삼는 방식
- digiCamControl 전체 솔루션을 제품 본체로 삼는 방식
- camera boundary의 유일한 authoritative source로 고정하는 방식

### 6.2 `reference/darktable/README.md`

이 파일은 darktable 전체 소스의 대체물이 아니라, Boothy가 darktable를 어떻게 참고하는지 고정하는 핵심 참조 노트다.

에이전트는 이 파일에서 최소 아래를 반드시 확인해야 한다.

- upstream repo URL
- pinned tag/commit
- darktable를 왜 참고하는지
- Boothy가 darktable를 무엇에 사용하지 않는지
- `XMP sidecar template` 우선 전략
- `darktable-cli` 핵심 명령과 운영 전제
- darktable 공식 tethering/gphoto2 capability를 camera-boundary 후보로 읽는 방법

중요:

- `reference/darktable/README.md`를 읽지 않고 darktable integration 작업을 시작하면 안 된다
- runtime truth는 style 이름이 아니라 artifact와 pinned version이다
- darktable의 tethering capability 존재와 그것을 Boothy의 현재 camera truth source로 채택하는 결정은 구분해야 한다

### 6.3 `reference/uxui_presetfunction`

기존 RapidRAW 계열 reference는 primary engine이 아니라 보조 참고 자산이다.

재사용 가능 대상:

- modern preset UX 아이디어
- 빠른 미리보기 UX 참고
- Rust/Tauri/WGPU 기반 이미지 처리 UI 구조 참고

비채택 대상:

- primary preset engine
- runtime apply truth source

## 7. 핵심 제품 가정

아래 가정은 잠금한다.

1. RAW 값 보정 기능은 초기에 몇 번 세팅하고 장기적으로 같은 프리셋을 반복 사용하는 목적이다
2. 현장 고객은 대부분 슬라이더를 조정하지 않는다
3. 고객 경험의 핵심은 편집 자유도보다 `촬영 안정성`, `최근 사진 확인`, `프리셋 선택의 단순함`, `종료/인계의 명확함`이다
4. 운영자는 full editor보다 `프리셋 배포`, `카메라 상태 확인`, `예외 상황 파악`이 더 중요하다
5. runtime 결과는 authoring과 가능한 한 동일한 darktable apply path를 사용해야 한다
6. 단일 booth lane은 시간당 약 150장의 RAW를 처리할 수 있어야 한다
7. 고객에게 먼저 보여줄 것은 `빠른 미리보기`이며, 무거운 노이즈 억제와 최종 export는 후행 가능하다

## 8. 고객/운영자 경험 요구사항

### 8.1 고객 흐름

- 예약자명과 휴대전화 뒤 4자리 입력
- 세션 시작
- 프리셋 선택
- 촬영 준비 안내 확인
- 촬영 진행
- 최신 사진과 썸네일 확인
- 필요 시 사진 삭제
- 종료 시각 확인
- 내보내기 대기
- 세션명 인계 후 셀렉실 이동

### 8.2 고객이 보면 안 되는 것

- 카메라 SDK 상태값
- helper 프로세스 로그
- RAW 편집 슬라이더
- 내부 저장 경로와 기술 정보
- 복잡한 진단 메시지

### 8.3 운영자 요구사항

- 고객 화면 상태와 카메라 상태를 함께 볼 수 있어야 함
- 재시도 또는 복구 판단에 필요한 진단 정보가 있어야 함
- preset catalog와 기본 preset을 관리할 수 있어야 함
- render queue와 backlog를 볼 수 있어야 함
- 예외 종료, 연장, 전화 필요 상태를 빠르게 파악할 수 있어야 함

## 9. 프리셋 기능의 제품 내 위치

핵심 결론:

> **프리셋 보정 기능은 고객 런타임의 중심 기능이 아니라 preset creation workflow의 중심 기능이다.**

따라서:

- `Preset Authoring`에서는 darktable 기반 rich editing UI 허용
- `Booth Runtime`에서는 저장된 preset을 선택하고 적용만 함
- `Runtime`은 full editing controls 없이도 preset 결과를 안정적으로 재현해야 함
- 가능한 한 `authoring engine == runtime apply engine` 원칙 유지

### 9.1 darktable 채택 이유

- fixed preset 저장 후 재적용 workflow가 분명함
- `darktable-cli`를 headless apply 경로로 쓰기 쉬움
- Windows 기준의 실전 운영 성숙도가 RapidRAW 계열보다 높음
- 노이즈 억제와 tone/color 품질이 더 검증됨
- 이 사업에서 중요한 것은 화려한 편집기보다 `예측 가능한 반복 결과`임

### 9.2 preset artifact 원칙

preset은 이름 문자열이 아니라 아래 정보를 묶은 배포 단위다.

- preset id
- display name
- pinned darktable version
- 적용할 XMP sidecar template 또는 equivalent artifact
- preview render profile
- final export profile
- noise handling policy
- output naming policy

### 9.3 darktable capability 범위 잠금

이 섹션은 darktable를 어디까지 제품 진실값으로 채택하고, 어디부터는 Boothy가 자체 surface로 모사하며, 무엇은 명시적으로 배제하는지 고정한다.

#### A. 직접 채택하는 것

- `XMP sidecar + history stack`
  - preset truth의 1차 artifact다
  - preset manifest는 최소 `preset id`, `display name`, `xmp template path`, `darktable version`, `preview profile`, `final profile`를 가져야 한다
  - Rust host는 이 artifact를 검증하고 session manifest에 `raw`, `preview`, `final`, `render status`를 분리 기록한다

- `darktable-cli`
  - headless preview/final apply의 기준 경로다
  - render worker가 queue, retry, result validation, error mapping을 맡는다
  - capture success와 render success는 같은 사건이 아니다

- 핵심 look module 계열
  - `input color profile`
  - `exposure`
  - `filmic rgb`
  - `color balance rgb`
  - `diffuse or sharpen`
  - denoise 계열
  - 이 모듈들의 파라미터는 approved XMP artifact 안에 봉인되고, booth runtime이 개별 슬라이더 의미를 재해석하지 않는다

- geometry/correction 계열
  - `lens correction`
  - `orientation`
  - `crop`
  - `rotate and perspective`
  - authoring 단계에서 preset에 bake된 범위만 채택한다
  - booth 현장에서 고객/운영자가 임의로 geometry를 다시 조정하는 기능은 두지 않는다

- OpenCL/GPU capability와 preview/final profile 분리
  - preview는 latency 우선
  - final export는 quality 우선
  - GPU 가능 여부는 operator diagnostics에서 보이되 customer language로 번역되지 않는다

#### B. Boothy가 자체적으로 모사하는 것

- preset catalog / publish / rollback UX
  - darktable의 style/library 개념을 그대로 노출하지 않고 Boothy preset catalog로 재구성한다

- internal preset authoring surface
  - darktable GUI 또는 darktable 기반 rich editing flow를 authoring context 안에서만 감싸거나 호출할 수 있다
  - 그러나 제품 계약은 darktable GUI 자체가 아니라 approved artifact publication workflow다

- booth-safe preview/final state transitions
  - customer는 `preset 이름`, `preview readiness`, `final export readiness`만 본다
  - operator는 `version pin`, `queue backlog`, `render failure`, `fallback mode`를 본다

#### C. 명시적으로 제외하는 것

- customer-facing darktable UI
- full photo library manager
- Lightroom preset compatibility layer
- `.dtstyle`를 runtime truth로 삼는 것
- 검증 없이 darktable를 current runtime camera tethering/control truth source로 승격하는 것
- darktable를 제품 전체 base app으로 삼는 것
- watermark/export adornment를 MVP preset truth에 섞는 것

#### D. 경계 규칙

- camera service는 detect, ready truth, capture, transfer, recovery를 소유한다
- darktable apply는 raw transfer 완료 이후에만 시작된다
- darktable state는 customer readiness truth가 될 수 없다
- darktable/gphoto2 tethering path는 Canon 700D camera-boundary 후보 reference가 될 수 있지만, baseline camera truth는 여전히 camera service가 소유한다
- customer surface는 darktable 용어, module 명, OpenCL 상태, library/config 경로를 직접 보지 않는다
- operator surface도 booth-safe boundary를 넘는 raw editor surface가 되어서는 안 된다

## 10. 권장 최종 아키텍처

```text
[Preset Authoring Tool: darktable]
  - raw adjustment
  - sidecar/XMP export
            |
            v
 [Preset Catalog / Manifest / Version Pin]
            |
            v
[Booth Runtime App] <----> [Stateful Camera Service]
  - customer flow             - detect
  - operator flow             - connect
  - session                   - capture
  - preset select             - transfer raw
  - preview orchestration     - recover
  - export/handoff            - diagnostics
            |
            v
   [Render Apply Worker: darktable-cli]
      - fast preview render
      - final render/export
      - retry/queue
            |
            v
      [Session Filesystem]
```

## 11. 현재 코드베이스에서 반드시 바뀌는 전제

현재 코드에는 `capture가 끝나면 processed file도 이미 확정되어 있다`는 전제가 강하게 들어 있다.

대표 지점:

- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src/session-domain/services/sessionManifest.ts`

현재 구조의 문제:

- camera helper가 `original_file_name`과 `processed_file_name`을 한 번에 돌려준다
- manifest capture record도 `원본 1개 + 처리본 1개` 모델이다
- UI는 촬영 완료와 처리 완료를 거의 같은 사건으로 가정한다

새 전제:

- capture success는 `RAW가 세션에 안전하게 저장됨`
- render success는 `선택 preset으로 preview 또는 final asset이 생성됨`
- 하나의 capture에는 `raw`, `preview`, `final` 자산이 단계적으로 생길 수 있음

## 12. history에서 흡수한 문맥과 사용자 의도

삭제될 `history/2026-03-14-camera-debug-log.md`의 핵심 내용은 이 섹션에 흡수한다.

이 섹션은 구현 요구사항 목록이 아니다. 에이전트가 왜 현재 결론이 나왔는지, 어떤 해석을 하면 안 되는지 이해하도록 만드는 배경 설명이다.

### 12.1 사용자가 실제로 겪은 핵심 고통

사용자가 반복적으로 겪은 실제 문제는 아래와 같다.

- UI는 `Ready`처럼 보이는데 실제 셔터는 전혀 반응하지 않음
- 버튼을 눌러도 촬영이 시작되지 않거나, helper가 30초 동안 매달린 뒤 timeout으로 끝남
- 어떤 수정은 준비 단계는 통과시키지만 촬영은 막고, 다른 수정은 촬영을 시도하게 하지만 준비 단계 자체를 막음
- 잘못된 timeout retry는 늦은 두 번째 셔터를 유발할 위험이 있었음
- 별도 readiness helper와 capture helper가 서로 간섭해 실제 셔터 타이밍을 망칠 수 있었음
- stale `CameraControlCmd.exe` 프로세스가 살아남아 이후 모든 촬영 시도를 오염시켰음

에이전트가 기억해야 할 사용자 의도:

- 사용자는 "이론적으로 맞는 설계"보다 `현장에서 셔터가 실제로 반응하는지`를 더 중시한다
- 사용자는 `Ready`라는 단어가 실제 사용자 경험과 어긋나는 것을 강하게 싫어한다
- 사용자는 dead button, fake-ready, silent timeout을 다시 겪고 싶어하지 않는다
- 사용자는 카메라가 연결되어 있어 보여도 실제로는 막혀 있는 branch를 가장 위험한 실패로 본다

### 12.2 history가 보여준 해석 원칙

1. `준비 단계 통과 가능`과 `지금 셔터를 쏠 수 있음`은 같은 진실이 아니다
2. `connected banner + export timeout`은 preparation gate에서는 usable할 수 있지만 capture gate의 완전한 ready truth는 아니다
3. capture 직전 별도 `/export` helper를 여러 번 띄우는 preflight는 실제로 도움이 안 될 수 있고, 오히려 helper contention을 키울 수 있다
4. booth path의 첫 셔터 명령은 reference와 맞춰 `/capture`가 기본이어야 하며, `/capturenoaf`는 제한된 fallback이어야 한다
5. timeout retry는 duplicate shutter 위험이 있으므로 매우 좁은 조건에서만 허용해야 한다
6. timed-out helper는 살아남아 이후 세션을 망칠 수 있으므로 stale process cleanup은 필수다
7. readiness watch는 capture 직전에 멈출 수 있어야 하며, capture와 동시에 별도 helper contention을 만들면 안 된다

### 12.3 session별 맥락 요약

세부 로그 전체를 다시 보지 않아도 되도록, 에이전트는 아래 요약만은 반드시 이해해야 한다.

- `홍길동1234_24`
  - `/capture`가 너무 짧은 timeout에서 잘렸고, transfer가 시작되기 전 helper가 종료될 수 있었다
  - 교훈: 정상인데 느린 capture branch가 있으므로 기본 timeout은 충분히 길어야 한다

- `홍길동1234_25`
  - `/capture` timeout no-transfer branch에서 `/capturenoaf`가 recovery seam으로 보였음
  - 교훈: no-AF는 fallback seam으로 유효할 수 있으나, 이것만으로 구조를 결정하면 안 된다

- `홍길동1234_26`
  - timeout 뒤에 no-AF를 시도하는 것은 너무 늦을 수 있었다
  - 교훈: 늦은 fallback은 이미 망가진 helper state를 되살리지 못할 수 있다

- `홍길동1234`
  - booth가 처음부터 `/capturenoaf`를 쓰는 경로도 hang할 수 있었다
  - 교훈: no-AF first는 보편 해법이 아니며 reference 기본값과 다를 수 있다

- `홍길동1234_2`
  - readiness watch는 `Ready`였지만 capture gate는 실제 ready가 아니었다
  - 교훈: 준비 readiness와 capture safety gate를 문서/코드에서 분리해야 한다

- `홍길동1234_4`
  - pre-capture gate가 너무 엄격해서 실제 셔터 명령 자체를 보내지 않았다
  - 교훈: 안전 게이트가 실제 셔터 시도 자체를 억제하는 dead branch를 만들면 안 된다

- `홍길동1234_5`
  - 별도 readiness preflight를 다 하고도 `/capturenoaf`가 여전히 hang했다
  - 교훈: capture 직전 extra helper launches는 가치가 없고 간섭만 늘릴 수 있다

- `홍길동1234_6`
  - no-AF first 자체가 잘못된 기본값일 가능성이 드러났다
  - 교훈: booth 기본 셔터 경로는 reference와 더 가깝게 `/capture` first여야 한다

- `홍길동1234_7`
  - 올바른 명령을 보내도 stale helper process가 살아 있으면 모두 실패한다
  - 교훈: process lifecycle cleanup은 운영 안정성의 핵심이다

### 12.4 에이전트가 이해해야 할 사용자 요구

- preparation 단계는 가능한 한 부드럽게 진행되어야 한다
- 그러나 capture 단계는 실제 셔터 불능 상태를 숨기면 안 된다
- `Ready`가 보이는 동안 capture 버튼이 dead branch로 이어지면 안 된다
- capture worker는 preflight helper spam을 만들면 안 된다
- app-side 보정이 끝난 뒤에도 실패한다면, 이를 `digiCamControl`, `darktable/gphoto2 tethering path`, 또는 `external camera state` 문제로 분리해서 기록해야 한다

### 12.5 darktable 설계와 history의 연결

darktable 도입은 카메라 문제를 가리는 우회책이 아니다.

오히려 history는 아래 방향을 강하게 지지한다.

- camera service는 `capture + transfer raw`까지만 책임져야 한다
- render apply는 별도 worker로 분리해야 한다
- 그렇지 않으면 camera timeout, helper contention, processed-file 가정이 한 경계에 다시 섞인다

즉, history가 보여준 진짜 교훈은 단순히 digiCamControl tuning이 아니라 `책임 경계 분리 실패가 현장 장애를 키운다`는 점이다.

## 13. 유지해야 할 전제

이번 코스 변경에서도 아래는 유지한다.

- Booth Runtime의 고객 흐름
- preset 선택의 단순한 UX
- session naming과 session lifecycle의 기본 구조
- camera readiness/watch의 분리
- operator diagnostics가 존재해야 한다는 원칙

즉, 바꾸는 것은 `카메라 다음 단계의 처리 경계`이지 제품 전체를 새로 쓰는 것이 아니다.

## 14. 현재 모듈별 영향 범위

### 14.1 Contract / Schema 경계

주요 파일:

- `src/shared-contracts/schemas/presetSchemas.ts`
- `src/shared-contracts/schemas/manifestSchemas.ts`
- `src/shared-contracts/dto/sessionManifest.ts`
- `src/shared-contracts/dto/cameraContract.ts`
- `src/shared-contracts/dto/cameraStatus.ts`

예상 변경:

- preset catalog에 darktable artifact 정보 추가
- session manifest에 render 상태와 asset 종류 추가
- capture completion과 render completion을 구분하는 DTO 추가

### 14.2 Frontend / Session 경계

주요 파일:

- `src/session-domain/state/SessionFlowProvider.tsx`
- `src/session-domain/services/sessionManifest.ts`
- `src/session-domain/services/activePresetService.ts`
- `src/capture-adapter/host/captureAdapter.ts`
- `src/capture-adapter/host/cameraAdapter.ts`
- `src/preset-catalog/services/presetCatalogService.ts`

예상 변경:

- capture 완료 후 즉시 preview가 없을 수 있다는 상태를 허용
- render queue 기반 UI 상태 추가
- final export와 preview ready를 따로 다룸

### 14.3 Rust Render / Export 경계

주요 파일:

- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/export/mod.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`

예상 변경:

- `render` 모듈 신설 가능성이 높음
- capture request는 raw persist 이후 성공 반환으로 재정의
- darktable-cli 호출, queue, retry, result validation 추가
- manifest append와 render update를 분리

### 14.4 Camera 경계

주요 파일:

- `src-tauri/src/capture/sidecar_client.rs`
- `src-tauri/src/capture/camera_host.rs`
- `src-tauri/src/capture/mod.rs`

예상 변경:

- camera layer는 render 책임을 가져가면 안 됨
- 현재 helper가 processed file 생성까지 한다면 coupling을 줄여야 함
- camera truth는 여전히 camera service가 소유함
- stale helper cleanup, watch-stop-before-capture, duplicate-shutter 회피 교훈을 잃지 말아야 함

## 15. 에이전트 병렬 작업 분리안

### Agent A: Contracts and Manifest

소유 범위:

- `src/shared-contracts/**`
- `src/session-domain/services/sessionManifest.ts`
- `src-tauri/src/session/session_manifest.rs`

책임:

- preset manifest schema 정의
- capture record v2 정의
- preview/final render status 모델 정의

### Agent B: Rust Render Worker

소유 범위:

- `src-tauri/src/render/**` 신규
- `src-tauri/src/export/**`
- `src-tauri/src/commands/**` 중 render 관련 부분

책임:

- darktable-cli adapter 작성
- queue/retry/result validation 작성
- preview render와 final render 분리

### Agent C: Frontend Runtime Flow

소유 범위:

- `src/session-domain/**`
- `src/capture-adapter/**`
- `src/customer-flow/**`
- `src/preset-catalog/**`

책임:

- render state를 UI에 반영
- preview-first 흐름 정리
- preset selection과 render result 연결

### Agent D: Camera Boundary Stabilization

소유 범위:

- `src-tauri/src/capture/**`

책임:

- camera helper 책임을 raw capture/transfer 중심으로 제한
- render worker와 camera worker 경계 보존
- readiness/capture regressions 방지

### Agent E: QA / Fixtures / Performance

소유 범위:

- `src/test/**`
- `src-tauri/tests/**`
- 새 fixture 디렉터리

책임:

- sample RAW / XMP / expected preview fixture 구성
- manifest migration test
- preview SLA benchmark harness

## 16. 새로 생겨야 하는 핵심 계약

### 16.1 Preset Manifest

필수 필드 후보:

- `presetId`
- `displayName`
- `darktableVersion`
- `xmpTemplatePath`
- `previewProfile`
- `finalProfile`
- `noisePolicy`

### 16.2 Render Job

필수 필드 후보:

- `sessionId`
- `captureId`
- `rawPath`
- `presetId`
- `renderKind` (`preview` | `final`)
- `requestedAt`

### 16.3 Render Result

필수 필드 후보:

- `captureId`
- `renderKind`
- `status`
- `outputPath`
- `completedAt`
- `errorCode`

## 17. 구현 순서

권장 순서:

1. schema와 manifest를 먼저 바꾼다
2. darktable preset artifact와 fixture를 만든다
3. render worker를 독립적으로 붙인다
4. capture flow에서 raw persist와 render trigger를 분리한다
5. UI 상태를 preview-first 기준으로 바꾼다
6. 성능/queue/백로그 진단을 붙인다

이 순서를 뒤집으면 camera layer와 UI layer가 동시에 흔들려 디버깅이 어려워진다.

## 18. 운영 성능 요구사항

- 단일 booth lane은 시간당 약 150장의 RAW 처리량을 감당해야 한다
- capture transfer 완료 후 customer preview는 가능한 한 즉시 표시되어야 한다
- preview SLA는 target hardware에서 `transfer 완료 후 3초 이내`를 1차 목표로 둔다
- 최종 보관용 render는 preview 이후 백그라운드에서 완료될 수 있다
- render queue는 종료 시점까지 밀리지 않아야 하며, backlog가 누적되면 운영자 진단에서 즉시 드러나야 한다
- 노이즈 억제는 preview path와 final path를 분리하여 조정 가능해야 한다

## 19. 비목표

- 고객이 현장에서 Lightroom Classic처럼 직접 RAW 값을 세밀하게 만지는 것
- 고객용 메인 앱에 full editor 기능 전체를 넣는 것
- digiCamControl UI 전체를 제품 본체로 포크하는 것
- 카메라 제어를 매 요청마다 CLI 프로세스로 새로 띄우는 것을 최종 구조로 고정하는 것
- reference 앱 두 개를 하나의 거대한 앱으로 합치는 것
- Lightroom preset 파일을 그대로 darktable preset으로 취급하는 것
- darktable GUI를 고객용 runtime에 넣는 것
- darktable 전체 소스를 지금 repo에 복사하는 것
- preview path와 final path를 하나의 무거운 pipeline으로 고정하는 것
- preparation-ready와 capture-ready를 하나의 불린 값으로 단순화하는 것
- capture 직전 별도 readiness helper를 반복적으로 띄워 셔터 경로를 방해하는 것
- stale `CameraControlCmd.exe` 프로세스 정리를 무시하는 것

## 20. 수용 기준

코스 변경이 성공했다고 보려면 최소 아래가 만족되어야 한다.

1. capture 성공 시 raw는 항상 세션에 남는다
2. render 실패가 capture 실패로 오인되지 않는다
3. preview는 target hardware에서 `transfer 완료 후 3초 이내`를 목표로 동작한다
4. final export는 preview와 별도로 재시도 가능하다
5. 동일 preset + 동일 darktable version에서 결과 재현성이 유지된다
6. operator는 render backlog와 실패 원인을 볼 수 있다
7. `Ready` 상태 의미가 customer flow와 capture behavior에서 문서화된 범위를 넘어서 과장되지 않는다
8. stale helper process가 이후 세션을 오염시키지 않도록 process lifecycle 전략이 존재한다

## 21. 지금 당장 필요한 산출물

- `reference/darktable/README.md`
- preset manifest example JSON
- sample XMP template 1개 이상
- render worker contract 초안
- manifest v2 초안
- history에서 흡수한 사용자 의도와 실패 맥락이 본 문서에 충분히 반영되었는지 검토 기록

## 22. helper integration 문서의 위치

`docs/plans/2026-03-14-digicamcontrol-helper-integration.md`는 여전히 단기 통합 문서로는 유효하다.

하지만 최종 구조로는 한계가 있다.

- stateless CLI 호출 중심
- camera state ownership 약함
- 프로세스 타이밍 이슈 반복
- customer runtime과 camera truth 경계 불안정
- preset authoring/runtime 분리 미반영
- darktable apply engine과 render queue 개념 없음
- preview/final 2단계 렌더 전략 미반영

따라서 이 문서는 최종 아키텍처 문서가 아니라 임시 통합 단계 문서로 읽어야 한다.

## 23. 최종 지시

에이전트는 이번 리팩토링을 `camera refactor`로 좁게 해석하면 안 된다.

이번 변경은 정확히 말하면 `capture-centric synchronous processing`에서 `raw-first asynchronous render pipeline`으로의 전환이다.

darktable 채택의 의미는 UI를 바꾸는 것이 아니라, `preset truth source`와 `render apply path`를 고정하는 것이다.

추가로, 에이전트는 history에 있었던 반복 실패를 단순 기능 요구사항이나 단순 과거 로그로 취급하면 안 된다.

- fake-ready
- dead capture button
- duplicate-shutter risk
- preflight helper contention
- stale helper poisoning

이 다섯 가지는 반드시 회피해야 하는 운영 금지 패턴이다.
