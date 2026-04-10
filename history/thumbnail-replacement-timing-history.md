# 첫 섬네일 표시와 프리셋 교체 완료 시간 히스토리

## 목적

이 문서는 새 에이전트가 이 파일 하나만 읽고도
`무엇이 문제인지`, `현재 무엇이 구현돼 있는지`, `다음에 어떻게 시도해야 하는지`,
`무엇이 이미 검증됐는지`를 바로 이어받을 수 있게 만드는 handoff 문서다.

이 문서의 범위는 아래 두 시간 seam으로 한정한다.

1. 촬영 후 첫 사진이 실제로 뜰 때까지의 시간
2. 첫 사진이 뜬 뒤 프리셋 적용 사진으로 교체 완료될 때까지의 시간

관련 문서:

- [photo-button-latency-history.md](/C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40/history/photo-button-latency-history.md)
- [recent-session-thumbnail-speed-brief.md](/C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40/history/recent-session-thumbnail-speed-brief.md)
- [recent-session-thumbnail-speed-agent-context.md](/C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40/history/recent-session-thumbnail-speed-agent-context.md)
- [camera-capture-validation-history.md](/C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40/history/camera-capture-validation-history.md)

## 새 에이전트가 먼저 알아야 할 결론

- 현재 제품은 두 시간 seam 모두를 로그로 추적할 수 있다.
- 현재는 capture마다 `capture_preview_transition_summary` 이벤트가 남아
  `first-visible-ms`, `replacement-ms`, lane owner, fallback reason을 한 줄로 다시 읽을 수 있다.
- 그래서 지금 상태는 `측정 가능`이지만 `즉시 집계 가능`까지는 아니다.
- 다음 회차는 새로운 추측보다, 현재 seam을 capture별 최종 숫자로 더 짧게 읽히게 만드는 쪽이 효율적이다.

## 문제 정의

### 시간 1. 촬영 후 첫 사진이 뜰 때까지의 시간

- 시작점: 고객이 `사진 찍기`를 눌러 같은 capture request가 저장되는 시점
- 종료점: 같은 capture의 첫 썸네일이 고객 화면에서 실제로 보이는 시점

제품 해석:

- 이 첫 사진은 pending same-capture thumbnail 또는 fast preview일 수 있다.
- 아직 preset-applied truth가 닫히지 않았더라도, 고객은 이 시점부터 "방금 찍은 사진이 떴다"라고 느낀다.
- 따라서 이 seam은 `first-visible seam`으로 따로 본다.

### 시간 2. 첫 사진이 뜬 뒤 프리셋 적용 사진으로 교체 완료될 때까지의 시간

- 시작점: 같은 capture의 첫 썸네일이 이미 고객 화면에 보인 시점
- 종료점: 같은 슬롯이 preset-applied preview로 교체되어 실제로 보인 시점

제품 해석:

- 이 seam은 render 완료만 뜻하지 않는다.
- 고객 화면에서 같은 슬롯이 실제로 교체된 것까지 닫혀야 완료로 본다.
- 따라서 이 구간은 `replacement seam` 또는 `preset-applied close`로 본다.

## 원인

현재 이 문서가 필요한 직접 원인은 아래와 같다.

- 두 시간 seam은 이미 제품적으로 중요하지만, 로그 해석이 두 군데로 나뉘어 있다.
- 세션별 `timing-events.log`와 앱 로그를 함께 봐야 해서, 새 에이전트가 읽기 시작할 때 매번 해석 비용이 든다.
- `first-visible`과 `replacement close`는 서로 다른 seam인데, 한 번에 묶여 설명되면 다음 시도에서 목표가 흐려진다.
- 현재 계측은 충분히 있지만, capture 단위 요약이 없어 실험 비교 속도가 느리다.

원인 요약:

- 문제는 `로그가 없다`가 아니라 `로그가 분산돼 있고, 최종 숫자 요약이 없다`다.

## 현재 구현

### 세션별 파일 로그

세션 폴더 아래 `diagnostics/timing-events.log`에 아래 이벤트들이 남는다.

첫 사진 seam에서 보는 이벤트:

- `request-capture`
- `file-arrived`
- `fast-preview-visible`
- `current-session-preview-pending-visible`
- `recent-session-pending-visible`

프리셋 교체 seam에서 보는 이벤트:

- `capture_preview_ready`
- `current-session-preview-visible`
- `recent-session-visible`

구현 해석:

- `fast-preview-visible`은 host 쪽 first-visible 근거다.
- `*-pending-visible`은 고객 화면에서 첫 썸네일이 실제로 보였다는 근거다.
- `capture_preview_ready`는 preset-applied preview truth가 닫혔다는 근거다.
- `*-visible`은 교체된 preview가 고객 화면에 실제로 보였다는 근거다.

### 앱 로그

앱 로그에는 아래처럼 seam 숫자를 바로 읽기 쉬운 라인이 남는다.

- `capture_first_visible_pending ... elapsed_ms=...`
- `capture_preview_ready ... elapsed_ms=...`
- `capture_preview_refinement_ready ... refinement_elapsed_ms=...`

구현 해석:

- `capture_first_visible_pending`은 촬영 ack 이후 첫 same-capture 이미지가 준비된 총 시간을 바로 읽을 수 있다.
- `capture_preview_ready`는 촬영 ack 이후 preset-applied preview truth가 닫힌 총 시간을 바로 읽을 수 있다.
- `capture_preview_refinement_ready`는 first-visible 이후 refinement close가 추가로 얼마나 더 걸렸는지 바로 읽을 수 있다.

### 현재 구현 상태 결론

- 구현은 이미 있다.
- 첫 사진 seam과 교체 seam 모두 추적 가능하다.
- 이제 capture별 최종 summary 이벤트가 추가돼 seam 집계 비용이 줄었다.

## 시도방법

새 에이전트는 아래 순서로 접근하는 것이 가장 효율적이다.

### 1. 목표를 먼저 고정한다

- 목표 1: `촬영 후 첫 사진 표시` 시간을 더 짧게 만들 것인지
- 목표 2: `첫 사진 후 preset-applied 교체 완료` 시간을 더 짧게 만들 것인지

주의:

- 두 목표를 하나로 섞으면 원인 분석이 흐려진다.
- `3초대 first-visible`과 `7초대 replacement close`는 같은 성격의 문제가 아니다.

### 2. 먼저 로그 seam이 실제로 닫혀 있는지 확인한다

한 세션에서 아래 체인이 보이는지 먼저 확인한다.

- `request-capture`
- `file-arrived`
- `fast-preview-visible`
- `recent-session-pending-visible` 또는 `current-session-preview-pending-visible`
- `capture_preview_ready`
- `recent-session-visible` 또는 `current-session-preview-visible`

판단 기준:

- 이 체인이 한 capture에서 닫히지 않으면 성능 최적화보다 계측 정합성부터 다시 봐야 한다.

### 3. 시간을 읽는 방법을 고정한다

첫 사진 seam은 아래 둘 중 하나로 읽는다.

1. 앱 로그의 `capture_first_visible_pending elapsed_ms`
2. `request-capture -> *-pending-visible` 시각 차이

교체 seam은 아래 둘 중 하나로 읽는다.

1. 앱 로그의 `capture_preview_refinement_ready refinement_elapsed_ms`
2. `*-pending-visible -> *-visible` 시각 차이

권장:

- 빠른 판단은 앱 로그 기준
- 세션 단위 증거 보존은 `timing-events.log` 기준

### 4. 다음 구현 시도 우선순위

가장 효율적인 다음 시도 순서:

1. 같은 하드웨어 세션 1개에서 cold start와 연속촬영을 분리해 다시 측정한다.
2. resident worker가 닫은 경우와 fallback/direct render가 닫은 경우를 분리 기록한다.
3. 그 다음에야 wait/join 정책이나 render 경로 튜닝을 다시 판단한다.

## 검증결과

2026-04-09 기준으로 아래 자동 검증으로 현재 구현 상태를 다시 확인했다.

Rust 검증:

- `helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it`
- `client_recent_session_visibility_events_are_mirrored_into_session_timing_logs`

프런트 검증:

- `SessionPreviewImage.test.tsx`
- `LatestPhotoRail.test.tsx`

이번 검증으로 확인한 사실:

- pending first thumbnail visibility 이벤트가 프런트에서 남는다.
- truthful preview로 교체될 때 visible 이벤트가 다시 남는다.
- 이 프런트 이벤트가 세션별 `timing-events.log`로 미러링된다.
- 따라서 `첫 사진 표시`와 `교체 완료`를 같은 세션 로그 seam으로 이어서 볼 수 있다.

## 구현 결과 기록 방법

이 문서는 읽기 전용 설명 문서로 끝나면 안 된다.
다음 회차부터는 아래 형식으로 구현 결과를 같은 문서에 계속 누적 기록한다.

구현 결과를 기록할 때 반드시 남길 항목:

- 작업 날짜
- 작업 목적
- 바꾼 제품 동작
- 수정한 구현 포인트
- 기대한 효과
- 남은 리스크

권장 기록 형식:

### 구현 결과 템플릿

```md
## 구현 결과

### YYYY-MM-DD: 작업 제목

- 목적:
- 변경한 동작:
- 수정한 구현 포인트:
- 기대 효과:
- 남은 리스크:
```

운영 원칙:

- 구현 설명은 코드 나열보다 제품 동작 변화 중심으로 쓴다.
- 다음 에이전트가 "무엇을 왜 바꿨는가"를 먼저 이해할 수 있어야 한다.
- 파일 목록은 필요할 때만 최소한으로 적고, 핵심은 동작 변화와 seam 영향으로 남긴다.

## 테스트 결과 기록 방법

다음 회차부터는 테스트 결과도 같은 문서에 계속 누적 기록한다.

테스트 결과를 기록할 때 반드시 남길 항목:

- 작업 날짜
- 테스트 목적
- 실행한 검증 명령 또는 검증 방식
- 통과 여부
- 확인된 사실
- 실패 또는 미확인 항목

권장 기록 형식:

### 테스트 결과 템플릿

```md
## 테스트 결과

### YYYY-MM-DD: 검증 제목

- 목적:
- 실행:
- 결과:
- 확인된 사실:
- 남은 공백:
```

운영 원칙:

- 단순히 `통과`만 남기지 말고, 그 테스트가 seam의 무엇을 증명했는지 같이 남긴다.
- 하드웨어 실검증과 자동화 테스트는 구분해서 적는다.
- 테스트를 못 돌렸다면 그 사실 자체를 기록한다.

## 다음 회차 기록 규칙

다음 에이전트는 이 문서를 갱신할 때 아래 순서를 지킨다.

1. 먼저 `원인`, `현재 구현`, `시도방법`을 읽고 현재 목표 seam을 다시 고정한다.
2. 구현을 바꿨다면 `구현 결과` 섹션에 먼저 기록한다.
3. 검증을 수행했다면 `테스트 결과` 섹션에 바로 이어서 기록한다.
4. 측정값이 생겼다면 `first-visible`과 `replacement close`를 분리해서 적는다.
5. 실험이 실패했더라도 삭제하지 말고 실패 결과를 그대로 남긴다.

짧은 원칙:

- 성공 이력만 남기지 않는다.
- 실패한 시도도 다음 에이전트의 비용을 줄이는 자산으로 남긴다.

## 구현 결과

### 2026-04-10: capture seam summary와 dedicated renderer truthful close 연결

- 목적: dedicated renderer accepted result를 실제 truthful close owner로 연결하고, capture별 seam 요약을 한 줄로 남긴다.
- 변경한 동작: validated dedicated renderer output은 inline overwrite 없이 same-slot truthful close를 닫고, fallback 경로를 타더라도 `capture_preview_transition_summary`가 남는다.
- 수정한 구현 포인트: dedicated renderer accepted result validation 강화, canonical output 채택 경로 연결, session timing summary 이벤트 추가.
- 기대 효과: capture별 `first-visible-ms`, `replacement-ms`, lane owner, fallback reason을 세션 로그에서 바로 읽을 수 있다.
- 남은 리스크: hardware trace와 booth-wide cutover 판단은 여전히 Story 1.13 package에서 닫아야 한다.

## 테스트 결과

### 2026-04-10: dedicated renderer dual-close summary 검증

- 목적: accepted dedicated renderer close와 inline fallback close가 모두 truthful summary를 남기는지 확인한다.
- 실행: `cargo test --test dedicated_renderer`, `cargo test --lib render::dedicated_renderer::tests`
- 결과: 통과
- 확인된 사실: accepted canonical output은 inline overwrite 없이 채택되고, queue saturation fallback도 `capture_preview_transition_summary`에 lane owner와 fallback reason을 남긴다.
- 남은 공백: UI replay/hardware trace 증거 패키지는 Story 1.13 handoff 범위다.

## 아직 남아 있는 공백

- summary 이벤트는 추가됐지만, cold start/연속촬영 비교와 hardware trace 패키지는 아직 별도로 누적해야 한다.
- 다음 회차에서는 이 summary를 기준으로 실장비 trace를 더 짧게 읽는 쪽이 효율적이다.

## 다음 에이전트용 짧은 결론

- 현재 상태를 `미구현`으로 보면 안 된다.
- 현재 상태는 `구현됨, 검증됨, capture summary도 추가됨`이다.
- 따라서 다음 작업은 summary를 기반으로 실제 하드웨어 trace와 cutover 판단을 더 빠르게 읽는 쪽이 가장 효율적이다.
