# 썸네일 시간 단축 에이전트 컨텍스트

## 목적

이 문서는 새 에이전트가 `history/recent-session-thumbnail-speed-brief.md` 전체를 다시 읽지 않고도,
현재 썸네일 시간 단축 문제를 바로 이어서 해결할 수 있게 만드는 handoff 문서다.

기준은 `2026-04-04` 시점이며,
최근 실로그 브리프와 그 이후 반영된 `resident first-visible worker` 변경 상태를 함께 반영한다.

## 지금 제품 기준에서 사실로 봐야 하는 것

- 고객이 느끼는 핵심 문제는 `사진 찍기` 이후 같은 컷이 레일에 아주 잠깐 보이는지보다,
  `preset-applied preview`가 제품적으로 닫히는 데 아직 오래 걸린다는 점이다.
- `fastPreviewVisibleAtMs`는 same-capture `first-visible` 지표다.
  이것은 빠른 중간 표시를 뜻할 뿐, truthful `previewReady`가 아니다.
- `previewVisibleAtMs`와 `xmpPreviewReadyAtMs`는 preset-applied preview truth를 닫는 지표다.
- 고객 surface는 preset-applied preview가 실제로 닫히기 전까지 계속 `Preview Waiting`을 유지해야 한다.
- fast preview가 먼저 보여도 canonical preview path는 같은 슬롯에서 later replacement 되어야 하며,
  다른 컷이나 다른 세션 자산이 섞이면 안 된다.

## 최근 실측에서 유지되는 제품 해석

최신 브리프 기준으로 반복 확인된 패턴은 아래와 같다.

- same-capture `first-visible`은 최근 여러 회차에서 대체로 `약 3.0s ~ 3.5s`까지 내려왔다.
- 하지만 preset-applied `preview close`는 여전히 `약 7s ~ 9s` 구간을 오간다.
- 최신 4컷 재확인에서는:
  - same-capture first-visible 평균: `약 3115ms`
  - preset-applied preview close 평균: `약 7715ms`
  - 첫 컷 final close: `10403ms`
- 즉 사용자가 말한 `3초대까지 내려온 것 같다`는 체감은 사실이지만,
  그 기준은 `first-visible`이고 제품 목표인 final close 기준으로는 아직 충분하지 않다.

## 브리프에서 꼭 이어받아야 할 문제 정의

- 기존 병목은 단순히 `wait budget이 짧다` 하나로 정리되지 않았다.
- 최근 로그에서는 아래 두 종류가 번갈아 확인됐다.
  - speculative lane 자체가 아직 `4초대`로 무거운 문제
  - 이미 진행 중인 same-capture close가 있는데 host/direct render가 겹쳐 들어가며 duplicate render 경쟁이 생기는 문제
- 따라서 이번 문제를 계속 풀 때도 목표를
  `same-capture first-visible 더 빠르게`
  하나로 좁히면 안 된다.
- 진짜 목표는
  `preset-applied preview close를 실제로 줄이면서 truthful replacement를 안정적으로 닫는 것`
  이다.

## worker 변경 이후 현재 기준선

최근 코드 상태는 이미 `상주형 first-visible worker`를 도입한 뒤다.
즉 새 에이전트는 `worker 도입 전`을 전제로 다시 설계하면 안 된다.

현재 반영된 방향:

- default preview lane은 `known-good booth-safe invocation`을 기준으로 정리됐다.
- `resident first-visible worker`가 세션/프리셋 기준 warm 상태를 유지하도록 들어갔다.
- preset 선택 또는 세션 시작 시 worker warm-up / preload / cache priming 경로가 있다.
- capture path는 per-capture one-shot spawn보다 resident worker 경로를 우선 사용할 수 있게 조정됐다.
- worker miss, queue saturation, warm-state loss, invalid output 시에는 false-ready 없이 기존 truthful fallback으로 내려가게 정리됐다.
- fast preview가 먼저 보이더라도 `previewReady`와 `preview.readyAtMs`의 truth owner는 여전히 render worker다.

## 이번 브랜치에서 실제로 구현한 것

이번 corrective 라운드에서 이미 반영된 구현은 아래 범주다.

- preview invocation baseline 정리
  - default booth path에서 known-good baseline을 기준으로 preview invocation 정책을 다시 고정했다.
  - 승인되지 않은 experimental/speculative 조합이 기본 운영 경로를 지배하지 않도록 정리했다.
- resident first-visible worker 도입
  - 세션/프리셋 단위 worker key, queue, idle timeout, restart, warm-up 경로가 들어갔다.
  - preset 선택 또는 세션 시작 시 worker를 미리 덥히는 경로가 추가됐다.
- capture path 재배선
  - per-capture one-shot render만 보던 구조에서 resident worker 경로를 우선 사용할 수 있게 바뀌었다.
  - enqueue 실패나 unsafe output이면 capture truth를 깨지 않고 바로 truthful fallback으로 내려가게 연결했다.
- duplicate render와 close ownership 보정
  - speculative close가 살아 있을 때 direct preview render가 중복으로 경쟁하지 않도록 보정했다.
  - resident/speculative 결과가 성공적으로 preset-applied preview file을 만들면 그 close를 truth로 인정하되,
    false-ready는 여전히 금지했다.
- truth/UX 계약 재고정
  - fast preview가 먼저 보여도 `Preview Waiting`은 유지된다.
  - canonical same-slot replacement는 유지하되, `previewReady` owner는 later render-backed close로 고정했다.

## 구현 후 자동 검증에서 확인된 것

현재 코드베이스에서 fresh run으로 다시 확인한 자동 검증 결과는 아래와 같다.

- Rust 회귀 검증:
  - `cargo test --test capture_readiness -- --nocapture --test-threads=1`
  - 결과: `65 passed`, `0 failed`
- Frontend 회귀 검증:
  - `pnpm vitest run src/booth-shell/components/LatestPhotoRail.test.tsx`
  - 결과: `1 passed`, `0 failed`

자동 검증에서 특히 의미 있는 시나리오:

- `helper_fast_preview_handoff_promotes_to_the_canonical_preview_path_and_later_render_reuses_it`
  - same-capture fast preview가 canonical path로 먼저 승격되고 later render가 그 경로를 재사용하는지 본다.
- `complete_preview_render_treats_a_finished_speculative_preview_as_preview_ready`
  - speculative close가 truth close로 승격될 수 있는지 본다.
- `complete_preview_render_does_not_start_a_duplicate_render_while_speculative_close_is_active`
  - active speculative close 위에 duplicate render가 다시 뜨지 않는지 본다.
- `readiness_allows_next_capture_once_same_capture_fast_preview_is_visible`
  - same-capture first-visible 확보 후 다음 촬영 readiness 정책이 유지되는지 본다.
- `client_recent_session_visibility_events_are_mirrored_into_session_timing_logs`
  - `recent-session-visible`가 timing log로 미러링되는지 본다.
- `resident_preview_worker_reports_queue_saturation_for_full_async_queue`
  - queue saturation fallback이 bounded failure로 처리되는지 본다.
- `resident_preview_worker_restarts_after_idle_timeout`
  - worker warm state가 끊긴 뒤 restart 가능한지 본다.

주의할 점:

- 자동 검증은 현재 브랜치의 correctness 회귀를 막아 주지만,
  이것만으로 booth hardware latency가 해결됐다고 볼 수는 없다.
- story 기준 hardware gate는 여전히 `No-Go`다.

## 실제 실측 데이터에서 확인된 것

새 에이전트가 이어받을 때 가장 중요한 세션 데이터는 아래다.

### A. latest 4컷 재확인: first-visible은 3초대지만 final close는 아직 7.7초 평균

- 세션: `session_000000000018a31080f82decc8`
- 컷별 수치:
  - `capture_20260404053350037_45bce70109`
    - first-visible `2896ms`
    - preview close `10403ms`
    - speculative close `3821ms`
  - `capture_20260404053402205_56cc9ca9b1`
    - first-visible `2975ms`
    - preview close `6501ms`
  - `capture_20260404053410363_e1b35b4236`
    - first-visible `3087ms`
    - preview close `6692ms`
  - `capture_20260404053428017_dd6f41ec2f`
    - first-visible `3501ms`
    - preview close `7266ms`
- 평균:
  - same-capture first-visible `3115ms`
  - preset-applied preview close `7715ms`
- 해석:
  - 사용자 체감의 `3초대`는 맞지만 first-visible 기준이다.
  - 첫 컷 `10403ms`를 포함해 최종 close는 아직 충분히 짧지 않다.

### B. 연속촬영 3컷 재확인: duplicate render 경쟁이 다시 9.1초대로 밀어 올림

- 세션: `session_000000000018a30fa3bb160dd0`
- 평균:
  - first-visible `3057ms`
  - preset-applied preview close `9116ms`
  - speculative `fast-preview-raster` render elapsed `3821ms`
- 해석:
  - first-visible은 계속 약 `3초`지만 final close는 다시 `9초대`였다.
  - 이 회차에서는 `preview-render-start`가 두 번 찍히는 duplicate render 패턴이 핵심 문제로 기록됐다.

### C. same-capture raster lane 자체도 아직 4초대

- 세션: `session_000000000018a30f15c3a996f8`
- 평균:
  - first-visible `2998ms`
  - preset-applied preview close `7850ms`
  - raw close `4115ms`
  - speculative raster render `4100ms`
- 해석:
  - 단순히 wait budget을 늘리면 되는 문제가 아니라,
    speculative lane 자체가 아직 무거운 상태였다.

### D. worker corrective 전 실장비 재검증에서 남은 가장 강한 경고 신호

- `request-capture -> file-arrived` 평균: `3286ms`
- `request-capture -> fast-preview-ready` 평균: `3863ms`
- `capture acknowledged -> RAW persisted` 평균: `3028ms`
- preview가 실제로 닫힌 4컷의 `capture acknowledged -> preview visible` 평균: `9238ms`
- warm 구간 최근 3컷: `7616ms`, `7761ms`, `8189ms`
- 별도 첫 컷 사례:
  - `capture_preview_ready elapsed_ms=14926`
- 해석:
  - UI 반영보다 preview 생성 자체가 병목이었다.
  - warm 상태처럼 보이는 구간에서도 `7.6s ~ 8.2s`였고,
    첫 컷은 `14.9s`까지 튀었다.

## 이 문서에서 실측 데이터를 읽는 방법

- `fastPreviewVisibleAtMs`
  - 고객이 같은 컷을 처음 볼 수 있게 된 시점
  - 아직 truthful `previewReady`는 아님
- `previewVisibleAtMs`
  - render-backed preview truth가 처음 닫힌 시점
- `capture_preview_ready elapsed_ms`
  - 글로벌 앱 로그에서 preview close를 보는 비교 지표
- `recent-session-visible`
  - UI 반영 close를 보는 지표

새 에이전트는 위 네 지표를 항상 분리해서 해석해야 한다.
특히 `3초대`는 현재까지 일관되게 `first-visible` 쪽의 성과이지,
문제 전체가 해결됐다는 뜻이 아니다.

## 현재 계약상 절대 깨면 안 되는 것

- render worker는 capture record에 저장된 `activePresetId + activePresetVersion` 기준으로만 동작해야 한다.
- pinned darktable version은 `5.4.1`이다.
- default booth path에는 승인 없는 experimental/speculative invocation flag를 다시 섞으면 안 된다.
- same-capture fast preview나 resident worker output이 먼저 보이더라도,
  actual preset-applied preview file이 만들어지기 전에는 `previewReady`로 올리면 안 된다.
- canonical preview path는 유지되어야 하며,
  same-path replacement가 실패해도 기존 canonical preview를 먼저 잃어버리는 downgrade는 금지다.
- `RAW copy`, placeholder, representative tile은 truthful preview/final ready 근거가 될 수 없다.

## 코드 기준으로 보면 현재 무엇이 이미 들어가 있는가

- `src-tauri/src/render/mod.rs`
  - resident preview worker queue/lifecycle
  - preview warm-up source
  - booth-safe preview invocation baseline
  - queue saturation / idle timeout / restart 처리 테스트
- `docs/contracts/render-worker.md`
  - resident worker 우선
  - same-path replacement
  - truthful `Preview Waiting`
  - required diagnostics event set
- `docs/contracts/session-manifest.md`
  - `fastPreviewVisibleAtMs`는 first-visible only
  - `xmpPreviewReadyAtMs`, `previewVisibleAtMs`는 render-backed truth
- `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
  - 이번 corrective 방향과 acceptance 기준
  - 아직 안 닫힌 작업 항목

## 새 에이전트 권장 읽기 순서

새 에이전트가 최종 목표를 향해 가장 빨리 움직이려면 아래 순서가 효율적이다.

1. 이 문서
   - 현재 목표, 최근 실측, 이미 구현/검증한 범위를 먼저 잡는다.
2. `docs/contracts/render-worker.md`
   - 무엇을 깨면 안 되는지부터 고정한다.
3. `docs/contracts/session-manifest.md`
   - `fastPreviewVisibleAtMs`, `xmpPreviewReadyAtMs`, `previewVisibleAtMs` 의미를 정확히 구분한다.
4. `history/recent-session-thumbnail-speed-brief.md`
   - 최신 세션 실측과 실패 패턴의 원문 근거를 확인한다.
5. `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
   - 지금 corrective story의 acceptance와 남은 공백을 확인한다.
6. 아래 tech 문서 3개
   - 구조 판단과 다음 단계 선택 근거를 보강한다.

## 반드시 같이 볼 tech 문서

- `_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md`
  - preview 병목을 엔진/호출비용/렌더 lane 관점에서 넓게 보는 기준 문서다.
- `_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md`
  - 현재 제품의 1차 해법을 `앱 셸 유지 + first-visible 전용 저지연 sidecar/worker`로 정리한 핵심 tech 문서다.
- `_bmad-output/planning-artifacts/research/technical-recent-session-fast-preview-research-2026-04-03.md`
  - same-slot replacement와 recent-session rail 관점에서 왜 UI 대공사보다 source topology가 핵심인지 정리한 문서다.

## tech 문서를 읽을 때의 해석 규칙

- tech 문서만 읽고 `엔진 교체`로 바로 점프하지 말 것
  - 현재 단계의 1차 목적은 resident first-visible worker 체계와 truthful close를 실장비에서 다시 닫는 것이다.
- tech 문서의 일반론보다 latest hardware 실측을 우선할 것
  - 현재 판단의 기준 데이터는 여전히 최신 session artifact와 booth 로그다.
- tech 문서의 제안도 아래 질문으로 다시 걸러야 한다
  - `preset-applied preview close`를 실제로 줄이는가
  - `Preview Waiting` truth를 유지하는가
  - same-slot replacement correctness를 보존하는가
  - session-scoped seam 계측으로 효과를 다시 증명할 수 있는가

## 아직 안 닫힌 공백

- 가장 큰 미완료는 `per-session seam instrumentation` 복구다.
- 목표 이벤트 체인은 한 세션 diagnostics 안에서 아래 순서로 닫혀야 한다.
  - `request-capture`
  - `file-arrived`
  - `fast-preview-visible`
  - `preview-render-start`
  - `capture_preview_ready`
  - `recent-session-visible`
- 현재 story artifact 기준으로 이 seam 복구는 아직 완료 상태가 아니다.
- hardware validation package도 아직 `No-Go / in-progress` 맥락이다.
- 즉 지금은 `worker 도입 자체`보다
  `worker 도입 후 실제 booth hardware에서 latency split을 다시 한 세션으로 닫는 일`
  이 더 급하다.

## 새 에이전트가 우선 확인해야 할 질문

1. 현재 latest booth session 한 개만으로 `first-visible`과 `preset-applied close`를 같은 diagnostics 경로에서 끝까지 추적할 수 있는가
2. resident worker가 실제 `previewReady close owner`가 되는 컷과,
   결국 RAW/direct fallback이 닫는 컷을 구분할 수 있는가
3. duplicate render가 완전히 사라졌는가, 아니면 특정 조건에서 여전히 재발하는가
4. 첫 컷 `10초대`와 연속촬영 `6초대 후반~9초대`를 같은 원인으로 봐야 하는가,
   아니면 cold-start seam과 steady-state seam을 분리해야 하는가
5. current worker topology에서 병목이 `render 자체`, `join/wait 정책`, `event correlation`, `replacement close 누락` 중 어디에 남아 있는가

## 다음 작업 우선순위 제안

1. per-session seam 로그를 먼저 닫아,
   global log에 기대지 않고 session folder만으로 latency split이 보이게 만든다.
2. latest hardware session 1개에서
   `fastPreviewVisibleAtMs`와 `previewVisibleAtMs`를 함께 검증한다.
3. resident worker가 성공적으로 닫은 컷과 fallback 컷을 분리해,
   실제 close owner를 다시 분류한다.
4. 첫 컷 cold-start와 연속촬영 steady-state를 따로 보고,
   둘 중 어느 쪽이 현재 더 큰 제품 문제인지 다시 정한다.
5. 그 다음에야 worker tuning, wait/join 조정, source policy 조정을 추가 판단한다.

## 새 에이전트용 짧은 결론

- 문제는 아직 해결되지 않았다.
- 다만 이제 전제는 `worker를 새로 도입할까`가 아니라,
  `도입된 resident first-visible worker 체계가 실제 booth hardware에서 truthful preview close를 얼마나 줄였는지 증명하고 남은 병목을 분리하는 것`이다.
- 다음 에이전트는 `3초대 first-visible`만 보고 성공으로 판단하면 안 된다.
- 판단 기준은 계속 `preset-applied preview close`, `Preview Waiting truth 유지`, `same-slot replacement correctness`, `session 단위 seam 계측 완결성`이다.

## 2026-04-05 추가 handoff: Test Look canary와 local renderer 고정비 절감

이 문서 작성 이후 새로 확인된 사실은 아래와 같다.

- latest `Test Look` session:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a351696c25d93c`
- 완료된 3컷 평균:
  - same-capture first-visible 약 `2959ms`
  - preset-applied preview close 약 `6372ms`
- 해석:
  - 사용자가 느낀 `3초대`는 이번에도 first-visible 기준으로는 맞다.
  - 하지만 `Test Look`에서도 final close는 여전히 약 `6.4초`다.

이번 latest에서 특히 중요한 새 해석:

- `camera-thumbnail`는 계속 실패했고,
  helper는 `windows-shell-thumbnail`를 same-capture first-visible로 닫았다.
- 그러나 truthful close owner는 이미 `fast-preview-raster`였다.
- 즉 현재 병목은 더 이상
  `raw-original fallback이 final close를 잡는다`
  쪽이 아니다.
- 현재 병목은
  `선택된 local renderer truthful close 자체의 비용`
  쪽이다.

같은 회차에서 따로 재현한 booth one-off 근거:

- `darktable-cli --version` 단독 실행도 약 `640ms ~ 787ms`가 들었다.
- `Test Look` published XMP를 same fast-preview JPG에 적용하는 one-off darktable 재현은 약 `3.7s ~ 3.8s`였다.
- XMP의 obvious raw-only 모듈을 임시로 꺼 본 quick test는 약 `110ms` 정도만 줄였다.
- 따라서 이 시점의 가장 싼 다음 수는
  `look/XMP surgery`
  보다
  `sidecar startup/cache 고정비 절감`
  으로 보는 편이 맞다.

이번 회차에서 실제로 반영된 구현:

- local renderer sidecar worker root를 session-scoped에서 runtime-scoped로 이동
  - `.boothy-local-renderer/preview`
- sidecar가 `darktable-version-cache.json`을 써서 repeated `--version` probe를 재사용
  - binary path, last write time, file length가 같으면 재프로브하지 않음
- sidecar에 runtime-scoped `--cachedir` 추가

이번 구현의 회귀 검증:

- `cargo test --manifest-path src-tauri/Cargo.toml local_renderer -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness real_local_renderer_sidecar_reuses_a_runtime_scoped_darktable_version_cache -- --test-threads=1`
- 둘 다 통과

새 에이전트가 이어받을 때 우선 기억할 것:

1. 이제 cheap win 하나는 더 넣었다.
   - repeated darktable version probe와 session-scoped cache loss를 줄이는 쪽이다.
2. 다음 hardware run에서 final close band가 의미 있게 내려오지 않으면,
   남은 병목은 startup이 아니라 render body 자체로 봐야 한다.
3. 그 경우 다음 판단은
   `lighter truthful renderer`
   또는
   `different close topology`
   쪽이어야 하고,
   다시 `3초대 first-visible`만 보고 성공으로 판단하면 안 된다.

## 참고 문서

- `history/recent-session-thumbnail-speed-brief.md`
- `docs/contracts/render-worker.md`
- `docs/contracts/session-manifest.md`
- `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`

## 2026-04-05 추가 handoff: Test Look 첫 컷 미적용 + 저화질 썸네일 보정

사용자 재검증 후 새로 확인된 사실은 아래와 같다.

- latest problematic `Test Look` session:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a359b2db1b8584`
- 사용자 피드백:
  - 첫 사진은 필터 적용이 끝내 닫히지 않았다.
  - 이후 연속 촬영도 여전히 체감이 느렸다.
  - truthful close 썸네일이 너무 흐리고 작게 보였다.

이번 회차에서 로그와 세션 산출물을 다시 맞춰 본 결과:

- 첫 컷 `capture_20260405035512593_24c7ba7114`
  - `fastPreviewVisibleAtMs`는 기록됐지만
  - `previewVisibleAtMs`와 `xmpPreviewReadyAtMs`는 끝까지 `null`로 남았다.
  - `timing-events.log`에는
    - 첫 `preview-render-start`
    - 약 5초 뒤 두 번째 `preview-render-start`
    - 곧바로 `preview-render-queue-saturated`
    만 남고 `preview-render-ready`는 없었다.
  - 하지만 세션 폴더에는 늦게 끝난 speculative output이 실제로 남아 있었다.
    - `capture_20260405035512593_24c7ba7114.preview-speculative.jpg`
    - detail 기준 elapsedMs 약 `9637ms`
    - 즉 렌더가 완전히 실패한 것이 아니라,
      `늦게 끝난 truthful close를 앱이 다시 승격하지 못해 첫 컷이 previewWaiting에 고착된 것`이 핵심이었다.
- 이후 완료된 3컷은 닫히긴 했지만 여전히 느렸다.
  - `7115ms`
  - `8880ms`
  - `6533ms`
- 동시에 latest truthful preview JPG 크기는 대체로 `17KB ~ 33KB` 수준이었다.
  - 현재 fast-preview truthful close cap이 `128x128`이라
    booth rail에서 체감 화질이 지나치게 희생된 상태로 해석하는 편이 맞다.

이번 회차에서 실제로 반영한 구현:

- readiness가 latest `previewWaiting` capture를 읽을 때,
  이미 끝난 speculative preview output이 있으면 비차단으로 즉시 승격하도록 추가했다.
  - 즉 `다음 촬영`이나 별도 수동 액션이 없어도 첫 컷 truthful close가 닫힌다.
- capture command의 refinement follow-up 대기창을 `2s -> 12s`로 늘렸다.
  - latest first capture의 실제 speculative elapsedMs(`9637ms`)보다 길게 잡아
    cold first shot도 follow-up update 안에서 닫힐 수 있게 맞췄다.
- fast-preview truthful close 해상도 cap을 `128 -> 256`으로 올렸다.
  - raw-original preview cap(`384`)보다는 여전히 작지만,
    recent-session rail에서 흐릿하게 보이던 회귀는 완화하는 방향이다.

이번 회차 회귀 검증:

- `cargo test --manifest-path src-tauri/Cargo.toml readiness_promotes_a_finished_speculative_preview_without_needing_another_capture -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_invocation_restores_a_sharper_than_legacy_128_cap -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml local_renderer -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness speculative_preview -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml --test session_manifest -- --test-threads=1`
- 전부 통과

이 handoff 시점에 아직 남아 있는 것:

1. booth hardware 재실측은 아직 다시 안 했다.
   - 따라서 `256px truthful close`의 실제 체감 화질 개선과
     `warmup earlier`가 first/continuous latency를 얼마나 줄였는지는
     다음 실장비 런에서 다시 닫아야 한다.
2. 만약 다음 `Test Look` 하드웨어 런에서도
   completed close가 여전히 `6s+`에 머무르면,
   이제 남은 병목은 startup보다 render body 자체로 보는 편이 맞다.
3. 그 경우 다음 단계는 다시 sidecar cache가 아니라
   `lighter truthful renderer` 또는 `different close topology`
   판단으로 넘어가야 한다.

## 2026-04-05 추가 handoff: latest Test Look 실측은 canary route가 winning speculative lane에 적용되지 않고 있었다

latest booth evidence:

- session:
  `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3753ffaaa16d8`
- preset:
  `Test Look / 2026.03.31`
- completed preview close elapsed:
  - `8555ms`
  - `7453ms`
  - `6453ms`
  - `6865ms`
- helper evidence:
  - all four requests closed with `capture-accepted -> file-arrived`
  - no helper timeout on this run

핵심 관찰:

- same session package에는 `renderer-route-selected / fallback / close-owner`가 없었다.
- `.boothy-local-renderer/preview` runtime root에도 latest run 산출 흔적이 없었다.
- 즉 직전 회차에 넣은 local renderer sidecar cache 개선은
  current winning close owner인 resident/speculative lane에는 전혀 적용되지 않았다.
- user-visible close는 여전히 speculative lane이 truth로 승격되는 구조였고,
  그 lane은 session-locked preview route policy를 우회한 채 darktable direct path를 탔다.

이번 회차 구현:

- speculative preview render도 capture-bound context를 읽어 same session-locked preview route policy를 해석한다.
- route가 `local-renderer-sidecar`로 잠겨 있으면,
  speculative lane도 same fast-preview-raster source로 local renderer candidate를 먼저 만든다.
- candidate rejection이면 speculative lane 안에서 darktable baseline으로 즉시 fallback 한다.
- speculative detail에는 selected route / fallback reason / close owner metadata를 함께 남기고,
  실제 promotion 시 timing events로 다시 풀어 써서 session package에서 바로 비교할 수 있게 했다.

새 회귀 테스트:

- `speculative_preview_close_uses_the_local_renderer_canary_when_policy_matches`
  - speculative winning close도 local renderer canary와 route evidence를 남기는지 본다.

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness speculative -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness local_renderer -- --test-threads=1`
- `cargo test --manifest-path src-tauri/Cargo.toml --test session_manifest -- --test-threads=1`
- all passed

## 2026-04-05 추가 handoff: 21:44 booth run은 여전히 pre-fix baseline처럼 보였다

latest booth session after that code work:

- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3768babafc5e4`
- completed close elapsed:
  - `7772ms`
  - `8520ms`
  - `7298ms`
  - `7526ms`
- helper boundary:
  - still healthy on all four captures
  - no timeout recurrence

하지만 이 run의 중요한 reading:

- session package에는 여전히
  `renderer-route-selected / renderer-route-fallback / renderer-close-owner`
  evidence가 없었다.
- app log는 아직
  `preview_renderer_warmup_failed ... libpng error: IDAT: CRC error`
  를 남겼다.

즉 이 run은 새 speculative-canary wiring이 booth hardware에 실제 반영된 증거로 보지 않는다.
운영적으로는:

1. 이 세션은 pre-fix baseline data로 기록한다.
2. 다음 hardware validation은 fresh runtime에서
   route evidence 3종과 warmup CRC disappearance를 먼저 확인한다.
3. 그 다음에도 close가 `7s+`면 truly render-body next step으로 넘어간다.
