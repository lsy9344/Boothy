# Recent Session Preview Architecture Update Input

작성일: `2026-04-06`

## 문서 목적

이 문서는 Boothy의 프로젝트 설계 문서, 아키텍처 문서, 후속 에이전트 브리프를 수정할 때
반드시 반영해야 할 최신 판단을 한 곳에 정리한 입력 문서다.

이 문서는 다음 내용을 담는다.

- 지금까지의 조사와 실측이 말해 주는 사실
- 문제의 진짜 원인과 현재 해석
- 제품 목표와 아키텍처 목표
- 이미 시도한 방향과 그 결과
- 앞으로 설계 문서에서 수정되어야 할 방향
- 계획 변경이 필요할 때 판단 기준이 되는 정보

## 이 문서가 반영해야 할 제품 관점의 변화

이 문제를 더 이상 `썸네일 속도`만의 문제로 보면 안 된다.

실제 고객 흐름은 아래에 가깝다.

1. 고객은 최신 사진을 크게 띄워 두고 본다.
2. 새 촬영이 끝나면 그 큰 화면의 최신 사진이 교체된다.
3. 고객이 기다리는 것은 `프리셋이 입혀진 상태의 최신 사진`이다.

즉 제품 관점의 진짜 목표는 아래다.

- `현재 세션 사진 레일에 같은 컷이 잠깐 빨리 보이는가`
  가 아니다.
- `고객이 크게 보고 있는 최신 사진이 프리셋 적용 상태로 얼마나 빨리 교체되는가`
  가 핵심이다.

따라서 앞으로의 설계 문서는 `thumbnail speed`가 아니라
`latest large preview replacement speed`
를 중심 문제로 다시 써야 한다.

## 이 문서의 소스

- `history/recent-session-thumbnail-speed-agent-context.md`
- `history/recent-session-thumbnail-speed-brief.md`
- `history/recent-session-thumbnail-speed-deep-research-brief-2026-04-05.md`
- `history/recent-session-thumbnail-speed-log-2026-04-04.md`

## 한 줄 요약

- `3초대` 개선은 실제로 있었지만 그것은 `first-visible` 성과다.
- 최신 later capture 기준으로 `원본 표시 -> 프리셋 적용본 표시`는 `약 3.4초`까지 내려왔지만, 제품 목표 `2.5초`에는 아직 못 미친다.
- 고객이 기다리는 `preset-applied truthful close`는 여전히 first-visible 이후 추가 대기를 만든다.
- 지금까지의 미세 조정으로는 절반 정도만 해결됐고,
  앞으로는 `더 가벼운 truthful renderer` 또는 `다른 close topology`
  판단이 필요한 단계에 들어섰다.

## 현재 사실

### 1. 이미 좋아진 것

- same-capture `first-visible`은 최근 다수 회차에서 `약 3.0s ~ 3.5s`까지 내려왔다.
- helper의 `camera-thumbnail`가 실패해도 `windows-shell-thumbnail`로 same-capture first-visible을 닫는 흐름은 반복 확인됐다.
- fast preview가 먼저 보이고 같은 슬롯에서 later replacement 되는 방향 자체는 맞다.

### 2. 아직 해결되지 않은 것

- `preset-applied preview close`는 아직 충분히 빠르지 않다.
- 최신 좋은 구간도 대체로 `약 6.5s` 수준이며,
  원본이 먼저 보인 뒤 프리셋 적용본으로 바뀌기까지는 아직 `약 3.4s`가 걸린다.
- 다른 회차는 여전히 `7s+`, `8s+`, 첫 컷 `10s+`가 나온다.
- 사용자가 실제로 느끼는 느림은 여전히 `최종 프리셋 적용본이 늦게 닫히는 것`에 있다.

### 3. 실측 기준으로 봐야 하는 대표 수치

- 2026-04-04 latest 4컷 재확인:
  - same-capture first-visible 평균 `3115ms`
  - preset-applied close 평균 `7715ms`
  - 첫 컷 `10403ms`
- 2026-04-05 Test Look best run:
  - same-capture first-visible 평균 `2959ms`
  - preset-applied close 평균 `6372ms`
- 2026-04-05 later booth baseline:
  - 완료 close `7772ms`, `8520ms`, `7298ms`, `7526ms`
- 2026-04-06 latest later captures:
  - `fastPreviewVisibleAtMs`: `3065ms`, `3126ms`
  - `previewVisibleAtMs`: `6509ms`, `6524ms`
  - `presetAppliedDeltaMs = previewVisibleAtMs - fastPreviewVisibleAtMs`:
    `3444ms`, `3398ms`

## 문제의 진짜 원인 해석

지금까지의 로그와 세션 evidence가 반복해서 보여 준 원인은 아래 범주다.

### 1. 문제는 `same-capture source 발견`만이 아니다

- same-capture 이미지는 비교적 빨리 잡힌다.
- 병목은 점점 `truthful close owner가 실제로 닫히는 비용` 쪽으로 이동했다.

### 2. 과거에는 `중복 렌더 경쟁`이 컸다

- 같은 컷의 speculative close가 진행 중인데 direct render가 겹쳐 시작되는 패턴이 있었다.
- 이 경우 같은 preview runtime을 두 번 소비하며 `9초대`까지 밀렸다.

### 3. 최근에는 `winning truthful close lane 자체의 비용`이 더 중요해졌다

- Test Look canary 기준으로 truthful close owner가 이미 `fast-preview-raster`로 옮겨간 회차가 있었다.
- 이 상태에서도 close가 `6.4초` 수준에 남았다.
- 즉 이제는 `어느 길이 이기느냐`보다 `이긴 길 자체가 아직 비싸다`는 해석이 강해졌다.

### 4. 일부 회차는 `구현 미반영/계측 미완성`이 판단을 흐린다

- route evidence 부재
- warmup fixture 오류
- fresh runtime 아닌 baseline run

따라서 최신 구조가 현장에 실제 반영되었는지와
구조 자체가 느린지를 구분해 읽어야 한다.

## 지금까지 시도한 방향

### 이미 들어간 큰 방향

- known-good preview invocation baseline 고정
- resident first-visible worker 도입
- worker warm-up / preload / cache priming
- capture path를 resident worker 우선으로 재배선
- duplicate render 회피
- same-path replacement 유지
- local renderer sidecar 도입 및 canary route 실험
- runtime-scoped cache 재사용
- version probe cache 재사용
- 늦게 끝난 speculative close의 승격 보정
- truthful close 해상도 회귀 보정

### 시도들의 공통 의도

- 기존 truth 계약은 유지한다.
- 먼저 보이는 것과 진짜 닫히는 것을 분리한다.
- 빠른 후보 경로가 이기면 채택하고, 아니면 안전한 fallback으로 간다.
- false-ready 없이 later replacement로 닫는다.

## 지금까지의 핵심 결론

### 맞았던 방향

- `앱 셸 유지 + first-visible 전용 저지연 worker/sidecar`
  방향은 1차 해법으로 유효했다.
- same-capture first-visible을 낮추는 데는 실제 효과가 있었다.
- duplicate render 제거, warmup, cache 재사용 같은 보정은 모두 의미가 있었다.

### 한계가 드러난 방향

- cap 축소 같은 미세 튜닝은 일정 구간 이후 효과가 작아졌다.
- source selection이나 wait budget만 만져서는 충분치 않았다.
- startup/cache 절감도 `6초대 초반` 이하로 크게 더 내리는 증거는 아직 없다.

### 지금 시점의 해석

- 지금은 더 이상 `worker를 새로 도입할까`를 묻는 단계가 아니다.
- 지금은 `현재 계열 구조로 meaningful reduction이 더 가능한가`,
  아니면 `lighter truthful renderer` 또는 `different close topology`로 넘어가야 하는가
  를 묻는 단계다.

## 앞으로 설계 문서에서 반드시 바뀌어야 할 문제 정의

기존 설계 문서에서 문제를 아래처럼 쓰면 부족하다.

- `현재 세션 사진 레일에 썸네일이 얼마나 빨리 보이는가`

앞으로는 아래처럼 다시 써야 한다.

- `고객이 크게 보고 있는 최신 사진이 프리셋 적용 상태로 얼마나 빨리 교체되는가`
- `same-capture first-visible과 preset-applied truthful close를 어떻게 분리하고 다시 합칠 것인가`
- `thumbnail rail`은 메인 latest preview 파이프라인의 부산물이지 목표 자체가 아니다.

## 앞으로 설계 문서에서 고정해야 할 목표

### 제품 목표

- 고객이 보는 최신 큰 화면은 최대한 빠르게 새 촬영본으로 교체되어야 한다.
- 그 교체는 가능한 한 빨리 `프리셋 적용 상태`에 도달해야 한다.
- 원본 사진이 먼저 보인 뒤 프리셋 적용본으로 교체되는 추가 대기는
  `2.5초 이내`를 목표로 둔다.
- `썸네일 개선`이 아니라 `latest preview replacement`가 주 목표다.

### 기술 목표

- `fastPreviewVisibleAtMs`와 `previewVisibleAtMs`를 계속 분리해서 관리한다.
- 핵심 목표 지표는
  `presetAppliedDeltaMs = previewVisibleAtMs - fastPreviewVisibleAtMs`
  로 고정하고,
  목표는 `<= 2500ms`로 둔다.
- first-visible과 truthful close를 같은 capture, 같은 slot, 같은 canonical path에서 연결한다.
- latest large preview와 rail thumbnail은 서로 다른 진실값을 만들지 않고 같은 close owner를 공유해야 한다.

### 운영 목표

- 현장 하드웨어에서 same session evidence만으로 latency split을 설명할 수 있어야 한다.
- canary route가 실제로 winning lane에 적용됐는지 session package로 증명 가능해야 한다.

## 절대 깨면 안 되는 계약

- `Preview Waiting` truth를 깨면 안 된다.
- 실제 preset-applied preview file이 생기기 전에는 `previewReady`로 올리면 안 된다.
- `activePresetId + activePresetVersion` 기준이 흔들리면 안 된다.
- canonical same-slot replacement를 깨면 안 된다.
- 기존 canonical preview를 먼저 잃는 downgrade는 금지다.
- RAW copy, placeholder, representative tile은 truthful ready 근거가 될 수 없다.
- pinned darktable version `5.4.1` 전제는 함부로 깨면 안 된다.

## 지금까지의 조사와 리서치가 제시하는 다음 후보

### 1차로 유지할 방향

- host가 truth owner인 구조 유지
- fast first-visible + later truthful replacement 구조 유지
- local canary / fallback / route evidence 유지

### 설계 문서에서 다음 검토 대상으로 올려야 할 것

- `local dedicated truthful renderer`
- `preview 전용 published artifact`
- `different close topology`
- `final/export와 latest preview close의 구조적 분리`
- `host는 유지하고 renderer adapter만 교체 가능한 구조`

## 설계 문서에서 더 이상 중심에 두지 말아야 할 것

- `3초대 first-visible`만 보고 성공으로 판단하는 것
- 레일 썸네일만 기준으로 품질/속도를 보는 것
- 계속되는 cap 미세 조정만으로 문제를 닫을 수 있다고 보는 것
- route evidence 없이 canary가 적용됐다고 가정하는 것
- UX masking으로 기술 병목을 덮는 것

## 현재 상태를 쉬운 말로 정리하면

지금 Boothy는 아래 상태다.

- 먼저 같은 사진을 빨리 한번 보여주는 길은 어느 정도 성공했다.
- 하지만 고객이 실제로 크게 보게 되는 프리셋 적용본은 아직 충분히 빠르지 않다.
- 그래서 지금은 `빨리 보이게 하는 문제`보다 `진짜 적용본을 빨리 닫는 문제`가 중심이다.
- 이미 여러 cheap win을 넣었기 때문에,
  이제는 `더 가벼운 진짜 렌더러` 또는 `닫는 방식 자체 변경`이 다음 질문이다.

## 설계 변경 시 반영해야 할 핵심 판단

### 판단 1

문제의 이름을 `thumbnail latency`로만 두지 말고
`latest preset-applied preview replacement latency`
로 재정의해야 한다.

### 판단 2

latest large preview를 중심 artifact로 보고,
rail thumbnail은 그것을 공유하거나 파생하는 구조로 재정렬하는 것이 맞다.

### 판단 3

지금까지의 방향은 완전히 틀리지 않았다.
다만 앞으로의 최적화 기준은 `first-visible`이 아니라 `truthful close`여야 한다.

### 판단 4

다음 설계 단계는 더 많은 미세 조정보다
`lighter truthful renderer`
또는
`different close topology`
를 선택할 준비를 해야 한다.

## 설계 문서 수정 시 직접 반영할 항목

1. 문제 정의
   - 썸네일 표시 속도 중심 서술을 latest large preview replacement 중심으로 수정
2. 목표 지표
   - `fastPreviewVisibleAtMs`
   - `previewVisibleAtMs`
   - `presetAppliedDeltaMs = previewVisibleAtMs - fastPreviewVisibleAtMs`
   - target `<= 2500ms`
   - close owner
   - same-slot replacement correctness
3. 아키텍처 설명
   - first-visible lane과 truthful close lane의 관계를 명시
   - large preview가 주 artifact라는 점 명시
4. 제약 조건
   - `Preview Waiting`
   - host-owned truth
   - preset-version binding
5. 후속 후보
   - local dedicated truthful renderer
   - preview-only artifact
   - different close topology
6. 검증 계획
   - per-session seam completeness
   - route evidence
   - hardware validation gate

## 다음 에이전트가 이 문서를 읽고 바로 답해야 할 질문

1. latest large preview 교체를 기준으로 보면 current architecture의 주 artifact는 무엇이어야 하는가
2. first-visible artifact와 truthful large preview artifact를 같은 slot에서 어떻게 연결할 것인가
3. current darktable/local-renderer 계열 구조로 `previewVisibleAtMs`를 더 내릴 수 있는가
4. 안 된다면 다음 1차 후보는
   - `lighter truthful renderer`
   - `preview 전용 artifact`
   - `different close topology`
   중 무엇인가
5. 어떤 후보가 현재 제품 계약을 가장 덜 깨면서도 큰 화면 기준 시간을 가장 많이 줄일 수 있는가

## 최종 결론

Boothy는 지금 `썸네일을 얼마나 빨리 보이게 할까`를 묻는 단계가 아니다.

지금은
`고객이 크게 보고 있는 최신 사진을 프리셋 적용 상태로 얼마나 빨리 교체할 수 있는가`
를 기준으로 아키텍처를 다시 써야 하는 단계다.

지금까지의 방향은 일부 맞았고 실제 성과도 있었다.
하지만 현 시점의 설계 문서는 다음을 명확히 받아들여야 한다.

- `3초대` 성과는 first-visible 성과다.
- 진짜 문제는 아직 truthful close이며,
  지금부터의 대표 목표는 `원본 표시 -> 프리셋 적용본 표시 <= 2.5초`다.
- future design work의 중심은 `lighter truthful renderer` 또는 `different close topology` 판단이다.
