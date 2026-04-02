---
stepsCompleted:
  - local-codepath-review
  - competitor-doc-research
  - feasibility-brief
inputDocuments:
  - src-tauri/src/commands/capture_commands.rs
  - src-tauri/src/capture/ingest_pipeline.rs
  - src-tauri/src/capture/sidecar_client.rs
  - src-tauri/src/render/mod.rs
  - src/session-domain/selectors/current-session-previews.ts
  - src-tauri/tests/capture_readiness.rs
workflowType: 'research'
lastStep: 3
research_type: 'technical'
research_topic: 'capture-to-xmp-preview latency in booth runtime'
research_goals: 'Compare Lightroom Classic and darktable behavior, identify whether Boothy can be made materially faster within the current structure, and determine whether thumbnail structure changes are required.'
user_name: 'Noah Lee'
date: '2026-04-01'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-04-01
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

이번 조사는 두 축으로 진행했다.

1. Boothy 현재 구현에서 `촬영 저장 -> XMP 적용 preview render -> 최근 세션 노출` 경로를 코드 기준으로 추적했다.
2. Lightroom Classic, darktable의 공식 문서를 기준으로 "왜 체감상 빠르게 보이는가"를 비교했다.

핵심 질문은 세 가지였다.

1. 지금 구조 안에서 체감 속도를 크게 줄일 수 있는가
2. 경쟁사 수준의 즉시성에 가까워지려면 어떤 아키텍처 패턴이 필요한가
3. 이를 위해 최근 세션 썸네일 구조 자체를 바꿔야 하는가

---

## Executive Summary

결론부터 말하면, **지금 구조 안에서도 꽤 큰 폭의 체감 단축은 가능하다.**
다만 **현재처럼 "첫 노출"과 "XMP가 반영된 정식 preview"를 사실상 같은 단계로 묶어두면 Lightroom Classic이나 darktable GUI가 주는 즉시성까지는 어렵다.**

가장 중요한 판단은 아래와 같다.

- **썸네일 UI 구조를 전면 개편해야 하는 상황은 아니다.**
- Boothy는 이미 코드상으로 **"pending preview 파일이 있으면 ready 전에도 최근 세션에 보여줄 수 있는" 경로**를 가지고 있다.
- 하지만 현재 실제 캡처 헬퍼 계약은 `rawPath`만 전달하고 있어서, 그 경로가 실전에서는 거의 활용되지 못하고 있다.
- 따라서 가장 큰 체감 개선은 **최근 세션 구조 변경보다 `빠른 1차 썸네일 생성`과 `느린 XMP 렌더 교체`의 2단계 파이프라인 도입**에서 나온다.

즉,

- **단기:** 현재 구조 유지 + fast preview 주입 + darktable cold-start/queue 최적화
- **중기:** "즉시 노출용 preview"와 "XMP 반영 preview"를 분리한 2단계 파이프라인
- **장기:** darktable-cli 1회성 프로세스 기동 비용을 줄일 수 있는 상주형 render worker 전략 검토

---

## Current Boothy Findings

### 1. 현재 제품은 RAW 저장까지만 빠르게 끝내고, XMP preview는 그 뒤 별도 스레드에서 생성한다

현재 `request_capture`는 먼저 capture helper의 round trip을 완료해 RAW 저장을 확인한 뒤, 결과를 즉시 반환한다. preview render는 그 뒤 **120ms sleep 이후** 별도 스레드에서 `complete_preview_render_in_dir`를 호출한다.

의미:

- 고객은 촬영 성공을 빨리 받지만,
- 최근 세션에 보이는 썸네일은 별도 preview render가 끝나야 안정적으로 보이게 된다.

이 120ms는 줄일 수 있지만, **핵심 병목은 아니다.**
근본 병목은 이후의 `darktable-cli` 호출과 결과 승격이다.

### 2. preview render는 darktable-cli를 매 촬영마다 새로 띄우는 구조다

현재 preview render는 다음 특징을 갖는다.

- `darktable-cli` 1회성 프로세스 실행
- preview 크기 cap: 1280 x 1280
- `--hq false`
- 전용 `configdir`, `library` 지정
- in-flight render queue 최대 2개

의미:

- 이미 preview 품질을 낮추는 최소한의 배려는 들어가 있다.
- 하지만 경쟁사 GUI처럼 **상주 프로세스 + 메모리 캐시 + 기존 썸네일 재활용** 구조가 아니라, **촬영마다 외부 렌더 프로세스를 다시 여는 비용**이 반복된다.
- burst capture 상황에서는 queue 포화가 나면 체감 지연뿐 아니라 실패 가능성도 커진다.

### 3. 프런트는 이미 "pending 썸네일"을 보여줄 수 있다

`selectCurrentSessionPreviews`는 아래 조건이면 preview를 보여줄 수 있다.

- `renderStatus`가 `captureSaved` 또는 `previewWaiting`
- `preview.assetPath`가 세션 범위 안에 있음
- `preview.readyAtMs`는 아직 null

즉, **정식 render 완료 전이라도 같은 capture의 임시 preview 파일만 있으면 최근 세션에 먼저 노출할 수 있다.**

이건 매우 중요하다.

- 최근 세션 UI가 느린 것이 본질적 병목이 아니라는 뜻
- **빠른 임시 preview를 넣는 백엔드 경로만 열리면, 프런트 구조를 거의 안 바꾸고도 체감 속도를 크게 줄일 수 있다는 뜻**

### 4. 이 fast-path는 테스트로도 이미 검증돼 있다

테스트에는 "preview render가 아직 pending이어도, 같은 capture의 빠른 썸네일 JPEG가 이미 있으면 노출을 유지한다"는 시나리오가 있다.

의미:

- 제품이 원하는 UX 패턴은 이미 코드상으로 일부 수용하고 있다
- 문제는 capability가 없는 것이 아니라, **실전 캡처 파이프라인이 그 capability를 채워주지 못하는 것**

### 5. 현재 helper 계약은 RAW만 넘기고 preview는 넘기지 않는다

`CanonHelperFileArrivedMessage`와 `CompletedCaptureRoundTrip`은 현재 아래 정보만 실질적으로 보존한다.

- `captureId`
- `rawPath`
- capture/persist timing

preview 경로, embedded JPEG 경로, camera JPEG 경로가 없다.

의미:

- 캡처 직후 빠른 썸네일이 존재하더라도, 현재 계약만으로는 host가 이를 표준 경로로 연결하기 어렵다
- 즉시성 개선의 핵심은 UI보다 **helper contract / ingest contract 보강**이다

### 6. 현재 코드의 5초 preview budget은 목표일 뿐, 구조적으로 안정 보장은 없다

세션 manifest 상 preview budget은 5,000ms다.
하지만 실제 구조는 매번 외부 프로세스를 다시 띄우고, heavy preset/XMP의 복잡도와 GPU/OpenCL 상태에 따라 변동한다.

따라서 현재 구조만으로는 "실사용에서 항상 짧게"를 보장하기 어렵다.

---

## Competitor Research

## Lightroom Classic

### 공식 문서에서 확인된 점

- Adobe는 tethered capture에서 **가장 최근 촬영된 사진을 preview area에 자동 표시**할 수 있다고 명시한다.
- 같은 tethered capture bar에서 **Develop Settings preset을 import 시점에 적용**할 수 있다.
- Import 옵션에는 **Embedded & Sidecar preview**가 있으며, Adobe는 이를 "camera가 가진 가장 큰 preview를 즉시 보여주는 더 빠른 방식"으로 설명한다.
- Preferences에는 **"Replace embedded previews with standard previews during idle time"** 옵션이 있다.
- Lightroom Classic은 GPU를 사용해 display, image processing, export를 가속할 수 있다.

### 제품적으로 읽히는 의미

공식 문서를 종합하면 Lightroom Classic은 체감 속도를 위해 아래 패턴을 쓴다.

1. **즉시 보이는 것**: 카메라가 이미 가진 embedded preview를 최대한 빨리 노출
2. **나중에 교체되는 것**: 더 정확한 standard preview 또는 develop-rendered preview
3. **별도 가속 축**: GPU를 이용해 display/image processing/export를 가속

즉 Lightroom의 빠름은 "preset 적용을 순식간에 끝낸다"기보다,
**먼저 보이는 자산과 나중에 정확해지는 자산을 분리해 체감을 빠르게 만드는 제품 전략**에 가깝다.

## darktable

### 공식 문서에서 확인된 점

- darktable thumbnails 문서는 첫 import 시 **embedded thumbnail(JPEG)** 을 추출하거나 raw를 직접 처리해 thumbnail을 만들 수 있다고 설명한다.
- 그리고 embedded thumbnail 추출은 **"usually very fast"** 라고 명시한다.
- 다만 그 썸네일은 camera converter의 결과이므로 darktable의 뷰와 다를 수 있고, 이후 **darktable internally processed version으로 교체**된다고 설명한다.
- darktable는 최근 thumbnail을 **디스크/메모리 캐시**에 유지하며, secondary cache 접근은 재처리보다 훨씬 빠르다고 설명한다.
- `darktable-generate-cache`는 누락된 thumbnail을 **background**에서 생성할 수 있다.
- `darktable-cli`는 XMP sidecar를 직접 받아 export할 수 있으며, `--apply-custom-presets false`를 주면 `data.db` preset 로딩 없이 여러 인스턴스 실행이 가능하다고 문서화돼 있다.
- OpenCL 문서는 지원 환경에서 GPU 가속을 활성화할 수 있음을 안내한다.

### 제품적으로 읽히는 의미

darktable GUI가 빠르게 느껴지는 이유를 공식 문서 기준으로 해석하면 아래와 같다.

1. **첫 화면은 embedded thumbnail로 빠르게 채운다**
2. **이후 내부 처리 결과로 교체한다**
3. **썸네일을 캐시에 유지해 재처리를 줄인다**
4. **상주 GUI 프로세스 안에서 동작하므로 cold start 비용이 적다**

여기서 4번은 공식 문서의 직접 문장이라기보다 1~3번과 현재 darktable-cli 구조를 비교한 **합리적 추론**이다.

---

## Comparative Diagnosis

경쟁사와 Boothy의 가장 큰 차이는 XMP 품질 그 자체보다 **"언제 무엇을 보여주느냐"** 에 있다.

### 경쟁사 패턴

- 즉시 노출용 자산: embedded/cached thumbnail
- 나중 교체 자산: 정확한 렌더 결과
- 가속 요소: GPU, cache, idle/background generation

### Boothy 현재 패턴

- 즉시 노출용 자산: 사실상 없음
- 나중 교체 자산: darktable-cli가 만든 preview
- 가속 요소: preview size cap, HQ off 정도만 존재

그래서 현재 Boothy는 사용자 체감상

- "사진은 저장됐는데 최근 세션은 한참 비어 있는" 시간을 만들고,
- 경쟁사는 "일단 뭔가 바로 보이고, 뒤에서 더 정확한 결과로 바뀌는" 경험을 만든다.

이 차이가 체감의 핵심이다.

---

## Can Boothy Get Much Faster Without Changing Thumbnail Structure?

### 결론: 가능하다. 그리고 우선 그렇게 가는 것이 맞다.

현재 최근 세션 썸네일 구조를 전면 개편하지 않아도 아래 방식으로 상당한 개선이 가능하다.

### 가능 경로 A: fast preview 파일을 capture 직후 canonical preview 경로에 먼저 쓴다

가장 유력한 방법이다.

방법:

1. helper가 RAW와 함께 camera embedded JPEG 또는 빠른 preview JPEG를 확보
2. 이를 `renders/previews/{captureId}.jpg` 같은 canonical path에 즉시 기록
3. ingest 시 `seed_pending_preview_asset_path`가 이 파일을 잡아 `previewWaiting` 상태에서도 최근 세션에 노출
4. 나중에 darktable-cli 결과가 같은 경로를 교체하고 `readyAtMs`를 채움

장점:

- 프런트 구조 거의 그대로 사용 가능
- 현재 테스트/selector와 자연스럽게 맞물림
- 고객 체감상 "즉시 최근 세션에 보임"이 가능

주의:

- 이 첫 preview는 XMP가 완전히 반영된 결과가 아닐 수 있다
- 하지만 Lightroom/darktable도 같은 철학을 부분적으로 사용한다

### 가능 경로 B: 현재 darktable-cli 경로 자체를 더 줄인다

추가 단축 포인트는 있다.

1. 120ms 고정 sleep 제거 또는 조건부화
2. preview 해상도 재조정
3. `--apply-custom-presets false` 검토
4. OpenCL/GPU 활성 상태 검증
5. session 시작 또는 preset 선택 시 darktable warm-up 수행

기대 효과:

- 수백 ms ~ 일부 초 단위 단축 가능성은 있다
- 하지만 **이것만으로 Lightroom/darktable GUI 수준의 즉시성 체감을 재현하기는 어렵다**

즉, 이 경로는 해야 하지만 단독 해법은 아니다.

---

## Is a Thumbnail Structure Change Required?

### 결론: "지금 당장 필수"는 아니다

현재 구조만으로도 아래 UX는 구현 가능하다.

- `preview.assetPath`는 이미 존재
- `readyAtMs`가 null이어도 pending preview 표시 가능
- render 완료 후 같은 asset path를 교체 가능

따라서 **최근 세션 썸네일 컴포넌트/선택자 구조 자체가 핵심 장애물은 아니다.**

### 다만, 아래 수준의 구조 보강은 중기적으로 권장된다

추천 보강:

- preview source 구분
  - `embedded`
  - `fast-derived`
  - `xmp-rendered`
- timing 분리
  - `fastPreviewVisibleAtMs`
  - `xmpPreviewReadyAtMs`
- diagnostics 분리
  - `fast-preview-missed`
  - `render-cold-start`
  - `render-queue-delay`

이 보강은 속도 그 자체보다 **운영/품질 관리**를 위해 좋다.
즉, **썸네일 구조 변경은 "속도를 내기 위한 선결 조건"이 아니라, 나중에 운영 품질을 높이기 위한 선택지**에 가깝다.

---

## Recommendation

## Phase 1: 구조 유지, 체감 속도 급개선

가장 먼저 할 일:

1. helper 계약에 fast preview 경로를 추가하거나 helper가 canonical preview 경로에 즉시 JPEG를 쓴다
2. host는 현재 pending preview 표시 경로를 그대로 활용한다
3. darktable render 완료 시 같은 경로를 교체한다
4. `timing-events.log`에 fast preview / xmp preview 단계를 나눠 기록한다

예상 결과:

- 최근 세션 첫 노출 시간은 대폭 단축 가능
- "사용 불가할 정도" 문제는 가장 빠르게 완화 가능
- UI 구조 대공사 불필요

## Phase 2: darktable render 경로 최적화

병행 권장:

1. 120ms delay 재검토
2. preview size cap 재검토
3. OpenCL/GPU 실제 부스 환경 검증
4. `--apply-custom-presets false` A/B 검증
5. render queue 정책 재검토

예상 결과:

- XMP 반영된 정식 preview 완료 시간 단축
- burst 상황 안정성 개선

## Phase 3: 필요 시 구조 보강

만약 Phase 1, 2 이후에도 고객이 "XMP 반영본이 늦어 답답하다"고 느낀다면:

- `preview`를 단일 개념으로 둘지,
- `fastPreview`와 `renderedPreview`를 분리할지

를 재검토한다.

하지만 현재 분석 기준으로는 **이 단계는 우선순위 1이 아니다.**

---

## Implementation Plan

이 섹션은 실제 구현에 바로 사용할 수 있는 권장안이다.

핵심 방향은 하나다.

- **MVP는 기존 manifest / 최근 세션 UI 구조를 최대한 유지한다**
- **빠른 preview만 먼저 기존 preview 슬롯에 넣고**
- **XMP preview가 준비되면 같은 경로를 교체한다**

이 방식이 가장 작은 변경으로 가장 큰 체감 개선을 만든다.

### Implementation Decision

권장 구현안은 아래다.

- **선택안:** helper가 fast preview를 준비하고 host가 이를 canonical preview 경로로 승격
- **비선택안 1:** darktable-cli 최적화만으로 해결
- **비선택안 2:** 썸네일/manifest 구조부터 크게 분리

선택 이유:

- darktable 최적화만으로는 "아무 것도 안 뜨는 시간"을 충분히 줄이기 어렵다
- 구조 대개편 없이도 현재 selector/UI를 그대로 활용할 수 있다
- 실패해도 현재 동작으로 자연스럽게 fallback 가능하다

### MVP Principle

MVP에서는 아래 원칙을 지킨다.

- capture 성공과 fast preview 성공을 분리한다
- fast preview 실패가 capture 실패로 승격되면 안 된다
- fast preview는 고객 체감 개선용이고, 진실 소스는 여전히 capture-bound XMP render다
- session manifest의 `preview.assetPath`와 `readyAtMs` 의미를 재사용한다
- `readyAtMs === null` 이면 "보이는 임시 preview", `readyAtMs !== null` 이면 "XMP 반영 preview"로 해석한다

### Proposed MVP Contract Change

helper와 host 사이에 아래 optional 필드를 추가하는 것을 권장한다.

- `fastPreviewPath`
- `fastPreviewKind`

권장 값:

- `fastPreviewPath`
  - helper가 만든 빠른 JPEG 경로
  - source 예시는 `embedded-jpeg`, `camera-jpeg`, `helper-derived-jpeg`
- `fastPreviewKind`
  - `embedded-jpeg`
  - `camera-jpeg`
  - `helper-derived`

MVP에서 중요한 점:

- 이 필드는 **optional** 이어야 한다
- 값이 없으면 기존 RAW-only 파이프라인으로 그대로 동작해야 한다
- UI 계약은 우선 바꾸지 않는다

### Recommended Delivery Sequence

아래 순서로 구현하는 것이 가장 안전하다.

#### Slice 0. 계측 먼저 추가

목표:

- 지금 얼마나 느린지
- fast preview 도입 후 어디가 얼마나 줄었는지

를 동일 포맷으로 비교 가능하게 만든다.

추가 권장 이벤트:

- `fast-preview-promote-start`
- `fast-preview-promoted`
- `fast-preview-invalid`
- `preview-render-start`
- `preview-render-ready`
- `preview-render-failed`
- `preview-render-queue-saturated`

권장 지표:

- `fastPreviewVisibleMs`
- `xmpPreviewReadyMs`
- `renderQueueWaitMs`
- `renderProcessElapsedMs`

#### Slice 1. helper fast preview handoff

목표:

- 캡처 직후 host가 사용할 수 있는 빠른 JPEG를 helper가 함께 넘긴다

권장 동작:

- helper는 가능하면 capture와 같은 shot의 embedded JPEG 또는 빠른 derived JPEG를 확보한다
- `file-arrived` 이벤트에 optional `fastPreviewPath`를 함께 실어 보낸다
- fast preview가 없으면 필드를 비운다

주의:

- helper는 preview 품질보다 same-capture 정합성을 우선해야 한다
- 다른 세션 사진이나 직전 사진이 섞일 가능성이 있으면 보내지 않는 편이 낫다

#### Slice 2. host가 fast preview를 canonical preview로 승격

목표:

- request capture 응답 안에서 이미 최근 세션에 보일 수 있는 preview 경로를 채운다

권장 방식:

- host가 `fastPreviewPath`를 검증한다
- 검증 통과 시 session canonical path인 `renders/previews/{captureId}.jpg`로 copy 또는 promote 한다
- 그 뒤 기존 `seed_pending_preview_asset_path`가 이 파일을 잡도록 둔다
- manifest에는 `preview.assetPath`만 채우고 `preview.readyAtMs`는 비운다
- `renderStatus`는 계속 `previewWaiting`으로 둔다

이렇게 하면:

- 현재 프런트 selector가 그대로 pending preview를 노출할 수 있다
- 별도 UI 구조 변경 없이 첫 노출 시간을 줄일 수 있다

MVP에서는 아래를 권장한다.

- fast preview가 invalid면 조용히 폐기
- capture는 성공으로 유지
- darktable preview render는 예정대로 진행

#### Slice 3. XMP preview가 같은 경로를 교체

목표:

- 고객이 처음 본 same-capture 썸네일이 나중에 XMP 반영본으로 자연스럽게 바뀌게 한다

권장 방식:

- canonical path는 그대로 유지한다
- darktable render 완료 시 staging 파일을 canonical preview 경로로 교체한다
- 이 시점에만 `preview.readyAtMs`를 채운다
- `renderStatus`를 `previewReady`로 바꾼다
- readiness update 이벤트를 발행한다

이 접근의 장점:

- cache buster가 이미 `readyAtMs` 기반으로 동작하므로 현재 UI 설계와 잘 맞는다
- "같은 자리의 이미지가 더 정확한 버전으로 바뀌는" 제품 경험을 만들 수 있다

#### Slice 4. darktable preview 경로 최적화

fast preview를 붙인 다음에 이 단계를 진행하는 것이 맞다.

우선순위:

1. 120ms 고정 지연 제거 또는 조건부화
2. preview 해상도 cap 재검토
3. 부스 장비에서 OpenCL/GPU 활성 여부 점검
4. `--apply-custom-presets false` 실험
5. render warm-up 검토

권장 해석:

- 이 단계는 "첫 노출"보다 "정식 XMP 반영 시간"을 줄이는 단계다
- 고객 체감 개선의 1차 효과는 Slice 1~3이 더 크다

### MVP Success Criteria

MVP 성공 기준은 아래로 두는 것이 좋다.

- helper fast preview가 있는 shot은 최근 세션에 **XMP ready 전에도 same-capture 이미지가 노출**된다
- XMP preview 완료 후 같은 자리의 이미지가 자연스럽게 교체된다
- fast preview 누락 또는 손상 시 현재 동작으로 안전하게 fallback 된다
- capture 성공은 fast preview 실패 때문에 막히지 않는다
- 다른 세션 사진이 최근 세션에 섞이지 않는다
- preset binding은 여전히 capture-bound로 유지된다

권장 운영 목표:

- `최근 세션 첫 노출`은 건강한 booth 환경에서 `capture accepted` 이후 1초 안쪽을 목표로 둔다
- `XMP preview ready`는 기존 5초 예산을 유지하되 점진적으로 낮춘다

### Explicit Non-Goals For MVP

MVP에서는 아래를 하지 않는 편이 좋다.

- `fastPreview` / `renderedPreview`를 별도 schema field로 즉시 분리하는 일
- final render 파이프라인까지 함께 손대는 일
- 최근 세션 UI를 전면 개편하는 일
- darktable runtime truth 자체를 바꾸는 일

이것들은 MVP가 성공한 뒤 필요성이 남을 때만 검토해도 된다.

### Recommended Validation Strategy

구현 중 검증은 아래 순서가 가장 효율적이다.

1. helper fast preview가 있는 경우
2. helper fast preview가 없는 경우
3. fast preview 파일이 손상된 경우
4. burst capture로 render queue가 밀리는 경우
5. preset이 무거운 경우
6. 다른 세션이 동시에 존재하는 경우

반드시 확인할 것:

- 최근 세션에 잘못된 이전 shot이 잠깐이라도 보이지 않는지
- `previewWaiting -> previewReady` 전환 시 이미지가 stale cache로 남지 않는지
- fast preview가 missing이어도 기존 preview render가 정상 완료되는지

### Suggested Touchpoints

실제 구현에서 먼저 볼 가능성이 큰 위치는 아래다.

- `src-tauri/src/capture/sidecar_client.rs`
- `src-tauri/src/capture/normalized_state.rs`
- `src-tauri/src/capture/ingest_pipeline.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/render/mod.rs`
- `src/session-domain/selectors/current-session-previews.ts`
- `src/booth-shell/components/SessionPreviewImage.tsx`
- `src-tauri/tests/capture_readiness.rs`

### Phase 2 Structural Upgrade Criteria

아래 조건이 남아 있으면 그때 구조 보강을 검토한다.

- fast preview와 XMP preview의 차이가 제품적으로 너무 커서 고객 혼란이 생김
- 운영자가 어떤 preview가 현재 보이는지 진단하기 어려움
- telemetry만으로는 fast preview / rendered preview 구분이 충분하지 않음

그때 검토할 확장안:

- `previewSource`
- `fastPreviewVisibleAtMs`
- `renderedPreviewReadyAtMs`
- `previewReplacementCount`

하지만 이 확장안은 **MVP 선행 조건이 아니라 후속 품질 개선안**으로 두는 것을 권장한다.

---

## Direct Answers To The Original Questions

### 1. 현재 상태로 최대한 짧게도 가능한가?

**예. 꽤 줄일 수 있다.**
특히 "최근 세션에 아무것도 안 뜨는 시간"은 현재 구조 안에서도 크게 줄일 수 있다.

다만 그 핵심은 darktable-cli를 조금 더 빠르게 만드는 것보다,
**빠른 임시 preview를 먼저 보여주고 XMP 결과를 나중에 교체하는 것**이다.

### 2. 썸네일 구조 문제가 핵심인가?

**아니오. 핵심은 썸네일 UI 구조보다 preview 생산 파이프라인이다.**

현재 썸네일 구조는 이미 pending preview 표시를 지원한다.
따라서 핵심 병목은:

- helper 계약에 preview가 없음
- preview가 darktable-cli 완료와 사실상 묶여 있음
- per-capture process cold start가 큼

이다.

### 3. 경쟁사처럼 짧게 만들려면 무엇이 필요한가?

가장 가까운 답은 이것이다.

- **즉시 보이는 preview**
- **나중에 교체되는 정식 XMP preview**
- **캐시와 GPU/OpenCL 활용**

즉, 경쟁사의 빠름은 렌더 엔진 하나의 절대 속도보다
**노출 단계 분리 + cache + background replacement**의 결과다.

---

## Risks And Unknowns

- 이 작업 환경에는 `darktable-cli`와 `darktable-cltest`가 설치돼 있지 않아 실제 부스 머신 기준 실측은 하지 못했다.
- 따라서 GPU/OpenCL, preset complexity, cold-start 비용의 실제 숫자는 booth 장비 로그 확인이 필요하다.
- 특히 XMP에 무거운 denoise/sharpen/lens 단계가 많다면 preview latency는 preset마다 크게 달라질 수 있다.

즉, 이번 결론은 **구조 진단과 공식 문서 비교에는 높은 신뢰**, **실측 ms 추정에는 제한적 신뢰**를 가진다.

---

## Recommended Next Validation

실행 우선순위는 아래가 가장 좋다.

1. 부스 장비에서 최근 세션 지연 사례 5~10건의 `timing-events.log` 수집
2. helper fast preview 주입 프로토타입
3. fast preview first-visible 시간과 xmp preview ready 시간을 분리 측정
4. OpenCL/GPU on/off 비교
5. preview 해상도와 `--apply-custom-presets false` A/B

이 순서면 "구조를 바꿔야 하나?"를 추상 논쟁이 아니라 실측으로 닫을 수 있다.

---

## Sources

### Internal code

- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/capture/ingest_pipeline.rs`
- `src-tauri/src/capture/sidecar_client.rs`
- `src-tauri/src/render/mod.rs`
- `src/session-domain/selectors/current-session-previews.ts`
- `src-tauri/tests/capture_readiness.rs`

### External sources

- Adobe, *Import photos from a tethered camera*, updated Aug 13, 2025  
  https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html

- Adobe, *How to set Lightroom Classic preferences*  
  https://helpx.adobe.com/lightroom-classic/help/setting-preferences-lightroom.html

- Adobe, *Lightroom Classic GPU FAQ*  
  https://helpx.adobe.com/lightroom-classic/kb/lightroom-gpu-faq.html

- Adobe, *How to specify import options in Lightroom Classic*  
  https://helpx.adobe.com/lightroom-classic/help/photo-video-import-options.html

- darktable manual, *thumbnails*  
  https://docs.darktable.org/usermanual/3.6/en/lighttable/digital-asset-management/thumbnails/

- darktable manual, *darktable-generate-cache*  
  https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-generate-cache/

- darktable manual, *storage / XMP sidecar files*  
  https://docs.darktable.org/usermanual/4.6/en/preferences-settings/storage/

- darktable manual, *darktable-cli*  
  https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/

- darktable manual, *activate OpenCL*  
  https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/
