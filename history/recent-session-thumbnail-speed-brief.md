# 최근 세션 썸네일 속도 단축 브리프

## 목적

이 문서는 booth 앱에서 고객이 `사진 찍기` 버튼을 누른 뒤
`현재 세션 사진` 레일에 같은 촬영의 썸네일이 보이기까지 걸리는 시간을 줄이기 위한
조사 내용, 근거, 의견, 가설, 구현 계획을 한 곳에 모아 둔 문서다.

다음 구현 에이전트는 이 문서를 기준으로 작업한다.

관련 기존 문서:

- [photo-button-latency-history.md](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/history/photo-button-latency-history.md)
- [current-session-photo-troubleshooting-history.md](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/history/current-session-photo-troubleshooting-history.md)
- [_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md)

## 2026-04-04 최신 실로그 재확인 4: 사용자가 느낀 3초대는 same-capture first-visible 기준으로는 맞지만, preset-applied close는 아직 평균 약 7.7초이고 첫 컷은 여전히 10.4초였다

이번 회차는 사용자가
`3초대 까지 내려온것 같네요. 하지만 더 줄여야합니다. 일단 문서에 기록만 하세요`
라고 한 최신 체감을 실제 latest 로그와 세션 artifact 기준으로 기록하기 위한 회차다.

기준 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a31080f82decc8`
였고,
최신 앱 로그
`C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`
의 마지막 기록 시각은
`2026-04-04 14:34:34 KST`
였다.

이번 세션의 4컷 직접 수치:

1. `capture_20260404053350037_45bce70109`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2896ms`
   - `capture acknowledged -> previewVisibleAtMs`: `10403ms`
   - speculative close `preview-render-ready elapsedMs`: `3821ms`

2. `capture_20260404053402205_56cc9ca9b1`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2975ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6501ms`

3. `capture_20260404053410363_e1b35b4236`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3087ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6692ms`

4. `capture_20260404053428017_dd6f41ec2f`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3501ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7266ms`

이번 4컷 평균:

- same-capture first-visible: 약 `3115ms`
- preset-applied preview close: 약 `7715ms`

이번 최신 재확인에서 중요한 사실:

- 사용자가 말한 `3초대`
  는 same-capture first-visible 기준으로는 맞다.
- 실제로 최신 4컷의 first-visible은
  `2.90s`, `2.98s`, `3.09s`, `3.50s`
  였다.
- 다만 고객이 실제로 기다리는 preset-applied close는 아직
  `6.5s ~ 7.3s`
  수준이고,
  첫 컷은 다시 `10.4s`
  까지 튀었다.
- 즉 이번 회차는
  `first-visible 체감은 3초대로 내려왔지만 final close는 아직 충분히 짧지 않다`
  고 기록하는 편이 맞다.

이번 세션에서 함께 확인된 패턴:

- 4컷 모두 `fast-preview-promoted kind=legacy-canonical-scan`이 먼저 찍혔다.
- 첫 컷은 `preview-render-start`가 두 번 찍혔고,
  final close가 다시 약 `10.4s`로 크게 밀렸다.
- 반면 2~4컷은 final close가 `6.5s ~ 7.3s`로 내려와,
  연속촬영 구간은 직전 `9초대` 평균보다는 실제로 좋아졌다.
- 최신 앱 로그의 마지막 컷에는
  `speculative_preview_render_started`
  뒤
  `speculative_preview_wait_budget_exhausted wait_ms=2200`
  가 남았지만,
  `resident_first_visible_render_failed render-output-missing`
  는 이미 `recent-session-visible`이 찍힌 뒤에 늦게 남았다.
- 따라서 이번 회차는
  `same-capture first-visible 3초대 진입`
  과
  `연속촬영 final close 6초대 후반 진입`
  은 확인됐지만,
  `콜드스타트 첫 컷 10초대`
  와
  `전체 평균 7초대`
  는 아직 남아 있다고 기록해야 한다.

이번 회차의 제품 결론:

- `3초대까지 내려온 것 같다`
  는 체감은 맞다.
  다만 그 기준은 first-visible이다.
- 최종 목표인 preset-applied close 기준으로는 아직 평균 약 `7.7초`다.
- 특히 첫 컷이 다시 `10초대`로 튀기 때문에,
  제품 체감 기준으로는
  `좋아졌지만 아직 충분하지 않다`
  로 기록하는 편이 정확하다.

## 2026-04-04 최신 실로그 재확인 3: 콜드스타트만의 문제가 아니라, 연속촬영 3컷 모두에서 active speculative close 위에 duplicate preview render가 겹치며 preset-applied close가 다시 약 9.1초로 늘어났다

이번 회차는 사용자가
`콜드스타트도 문제지만, 연속촬영 후 보여지는 프리셋 적용 시간이 오히려 더 늘었다`
고 말한 체감이 실제 최신 로그와 session artifact로도 맞는지 다시 닫기 위한 기록이다.

기준 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30fa3bb160dd0`
였고,
최신 앱 로그
`C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`
의 마지막 기록 시각은
`2026-04-04 14:18:30 KST`
였다.

이번 세션의 3컷 직접 수치:

1. `capture_20260404051800843_7edc1c1526`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3148ms`
   - `capture acknowledged -> previewVisibleAtMs`: `9105ms`
   - speculative close `preview-render-ready elapsedMs`: `3720ms`

2. `capture_20260404051811734_6a88b8ebd4`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3012ms`
   - `capture acknowledged -> previewVisibleAtMs`: `9166ms`
   - speculative close `preview-render-ready elapsedMs`: `3921ms`

3. `capture_20260404051822274_9cd255ea21`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3011ms`
   - `capture acknowledged -> previewVisibleAtMs`: `9076ms`
   - speculative close `preview-render-ready elapsedMs`: `3823ms`

이번 3컷 평균:

- same-capture first-visible: 약 `3057ms`
- preset-applied preview close: 약 `9116ms`
- speculative `fast-preview-raster` render elapsedMs: 약 `3821ms`

이번 최신 재확인에서 중요한 사실:

- 3컷 모두 `fast-preview-promoted kind=legacy-canonical-scan`이 먼저 찍혔다.
- 즉 same-capture first-visible 자체는 계속 약 `3.0s` 수준으로 유지됐다.
- 그러나 3컷 모두 최종 `previewVisibleAtMs`는 다시 약 `9.1s`였다.
- `timing-events.log`에는 3컷 모두 `preview-render-start`가 두 번 찍혔다.
- 최신 앱 로그에는 연속촬영 구간에서
  `resident_first_visible_render_failed ... Access violation`
  이 남았고,
  마지막 컷은 그 직전에
  `speculative_preview_wait_budget_exhausted wait_ms=2200`
  뒤
  `render_job_started ... sourceAsset=fast-preview-raster`
  가 이어졌다.

이번 회차의 제품 결론:

- 지금 병목은
  `speculative close가 느리다`
  하나만이 아니다.
- 더 직접적인 문제는
  `이미 진행 중인 same-capture speculative close가 있는데도 host가 direct preview render를 다시 시작한다`
  는 점이다.
- 이 duplicate render가 preview runtime을 다시 경쟁시키면서,
  resident lane이 실패하고
  사용자 체감 close가 다시 약 `9초대`
  로 밀린다.
- 따라서 이번 단계의 우선순위는
  `더 빠른 새 렌더를 추가로 시작하기`
  가 아니라,
  `이미 시작된 same-capture close를 끝까지 받아오고 duplicate render를 피하기`
  다.

이번 회차에서 바로 반영한 개선:

1. direct preview render로 넘어가기 전에,
   active speculative close가 남아 있으면 한 번 더 join wait 하도록 바꿨다.
2. 그 사이 speculative output이 닫히면 그대로 promote해서 완료한다.
3. 즉 `진행 중인 same-capture close`와 `직접 fallback close`가 같은 preview runtime 위에서 겹쳐 달리지 않도록 조정했다.

다음 실검증에서 다시 볼 기준:

- `capture acknowledged -> fastPreviewVisibleAtMs`
- `capture acknowledged -> previewVisibleAtMs`
- `timing-events.log`에서 capture당 `preview-render-start`가 한 번만 남는지
- 앱 로그에서 `resident_first_visible_render_failed ... Access violation`이 사라지는지
- final `recent-session-visible`이 다시 `9초대`가 아니라 `6초대`로 내려오는지

## 2026-04-04 최신 실로그 재확인: same-capture first-visible은 약 3초지만, preset-applied close는 아직 약 7.9초고 현재 주병목은 speculative lane 자체가 여전히 4초대라는 점이다

이번 최신 재확인은 사용자가
`여전히 오래 걸린다`
고 한 최신 체감이 실제 latest booth 로그와 session artifact로도 맞는지 다시 닫기 위한 기록이다.

기준 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30f15c3a996f8`
였고,
최신 앱 로그
`C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`
의 마지막 기록 시각은
`2026-04-04 14:08:15 KST`
였다.

이번 세션의 3컷 직접 수치:

1. `capture_20260404050749386_05e637b0ad`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2980ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7841ms`
   - raw close `preview-render-ready elapsedMs`: `4123ms`
   - speculative detail `elapsedMs`: `4024ms`

2. `capture_20260404050758697_2ffac168d4`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3131ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8068ms`
   - raw close `preview-render-ready elapsedMs`: `4199ms`
   - speculative detail `elapsedMs`: `4256ms`

3. `capture_20260404050807684_5480293433`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2882ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7642ms`
   - raw close `preview-render-ready elapsedMs`: `4022ms`
   - speculative detail `elapsedMs`: `4019ms`

이번 3컷 평균:

- same-capture first-visible: 약 `2998ms`
- preset-applied preview close: 약 `7850ms`
- raw close `preview-render-ready elapsedMs`: 약 `4115ms`
- speculative `fast-preview-raster` render elapsedMs: 약 `4100ms`

이번 최신 재확인에서 가장 중요한 사실:

- `session.json` 기준 latest 3컷 모두
  `fastPreviewVisibleAtMs`는 약 `2.9s ~ 3.1s`에 들어왔다.
- 그러나 final `previewVisibleAtMs`는 다시 약 `7.6s ~ 8.1s`였다.
- 최신 앱 로그에는 마지막 컷에서
  `speculative_preview_render_started`
  뒤
  `speculative_preview_wait_budget_exhausted wait_ms=720`
  가 찍히고,
  곧바로
  `sourceAsset=raw-original`
  fallback render가 시작됐다.
- 동시에 session artifact에는 각 컷의
  `.preview-speculative.detail`
  파일이 남아 있었고,
  그 안의 speculative `fast-preview-raster` render elapsedMs도 약 `4.0s ~ 4.3s`였다.

이번 회차의 제품 결론:

- 현재 latest 병목은
  `wait budget이 너무 짧다`
  만으로 설명되지 않는다.
- 더 직접적인 병목은
  `same-capture raster를 써도 speculative preset render 자체가 아직 약 4초대`
  라는 점이다.
- 즉 이번 단계의 우선순위는
  `speculative lane이 더 자주 이기게 기다리기`
  이전에,
  `speculative lane 자체를 더 싸게 만들기`
  다.

이번 회차에서 바로 반영한 개선:

1. `fast-preview-raster` preview render cap을
   `384 -> 256`
   으로 더 낮췄다.
2. preview truth lane의 기본 booth-safe source selection이
   same-capture raster preview를 재사용할 수 있게 바꿨다.
   - 즉 fast preview가 이미 canonical preview path에 있으면,
     fallback close도 굳이 RAW original만 강제하지 않도록 조정했다.
3. speculative wait budget은
   `720ms -> 2200ms`
   수준으로 늘려,
   더 가벼워진 raster lane이 닫힐 기회를 실제로 잡을 수 있게 했다.

이번 변경의 의도:

- latest evidence상 same-capture raster render가 아직 아주 빠르진 않지만,
  그래도 RAW original close와 거의 같은 비용을 다시 한 번 쓰는 구조는 낭비가 크다.
- 따라서 이번 라운드는
  `RAW fallback을 빨리 시작하는 것`
  보다
  `lighter raster truth lane이 먼저 닫히도록 만들기`
  에 더 무게를 둔다.

다음 실검증에서 꼭 다시 볼 기준:

- `capture acknowledged -> fastPreviewVisibleAtMs`
- `capture acknowledged -> previewVisibleAtMs`
- `preview-render-ready elapsedMs`
- speculative detail의 `elapsedMs`
- latest `timing-events.log`에서 final close owner가
  `sourceAsset=fast-preview-raster`
  로 실제 바뀌는지

## 2026-04-04 최신 4컷 실로그 재확인: 사용자 체감대로 더 늘어났고, 이번 회차는 resident miss가 조용히 raw close로 넘어가는 쪽에 더 가깝다

이번 회차는 사용자가
`로딩이 더 늘어난 것 같다`
고 말한 최신 체감이 실제 최신 로그와 세션 artifact로도 맞는지 닫기 위한 기록이다.

기준으로 본 최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30deffa5eb40c`
였고,
최신 글로벌 앱 로그
`C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`
의 마지막 기록 시각은
`2026-04-04 13:47:26 KST`
였다.

이 세션에는 총 4개 capture가 남아 있었다.

이번 세션의 직접 수치:

1. `capture_20260404044648078_4a0d6f85ee`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3221ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8389ms`
   - `preview-render-ready elapsedMs`: `4425ms`

2. `capture_20260404044658068_76359b5823`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3222ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8685ms`
   - `preview-render-ready elapsedMs`: `4723ms`

3. `capture_20260404044708359_bf102315c0`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3190ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8160ms`
   - `preview-render-ready elapsedMs`: `4225ms`

4. `capture_20260404044717796_352bd5a282`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3094ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7952ms`
   - `preview-render-ready elapsedMs`: `4122ms`

이번 4컷 평균:

- same-capture first-visible: 약 `3182ms`
- preset-applied preview close: 약 `8297ms`
- `preview-render-ready elapsedMs`: 약 `4374ms`

최근 같은 날 기록과 비교하면 이번 최신 회차는 체감 기준으로 더 나빠졌다.

- `session_000000000018a30c240ea0a1f0`의 평균은
  first-visible 약 `3276ms`,
  preset-applied close 약 `7151ms`
  였다.
- `session_000000000018a30cf108b8edec`의 평균은
  first-visible 약 `2956ms`,
  preset-applied close 약 `7945ms`
  였다.
- 이번 최신 세션은 first-visible은 여전히 약 `3.1s` 수준이지만,
  고객이 실제로 기다리는 preset-applied close는 다시 약 `8.3s`로 늘어났다.

즉 이번 회차는
`조금 더 느려진 것 같다`
는 사용자의 체감이 로그 기준으로도 맞다고 기록하는 편이 정확하다.

이번 세션에서 확인된 중요한 패턴:

- 4컷 모두 `timing-events.log`에
  `fast-preview-promoted kind=legacy-canonical-scan`
  이 먼저 찍혔다.
- 4컷 모두 `recent-session-pending-visible`은 먼저 찍혔고,
  최종 `recent-session-visible`은 그 뒤 약 `8초` 전후에 닫혔다.
- 4컷 모두 `preview-render-start`가 두 번 찍혔지만,
  최종 `preview-render-ready` detail은 전부
  `widthCap=384;heightCap=384;sourceAsset=raw-original`
  이었다.
- 즉 latest runtime drift보다는,
  `현재 runtime 안에서 speculative first-visible path가 최종 close owner가 되지 못하고 raw-original truth lane이 계속 닫는다`
  고 보는 편이 맞다.

이번 세션의 helper 신호도 다시 비슷했다.

- 4컷 모두 `file-arrived`에는 `fastPreviewPath=null`이었다.
- 이후 `fast-preview-ready`는 모두 `windows-shell-thumbnail`로 도착했다.
- 직접 계산하면
  - `request -> file-arrived`: 약 `2954ms ~ 3599ms`
  - `request -> fast-preview-ready`: 약 `3580ms ~ 4371ms`
  - `file-arrived -> fast-preview-ready`: 약 `626ms ~ 771ms`
  - 평균 `file-arrived -> fast-preview-ready`: 약 `690ms`
  였다.
- 즉 same-capture source는 계속 오고 있지만,
  latest booth path는 그 source를 최종 preset-applied close로 연결하지 못하고 있다.

이번 회차에서 특히 중요한 최신 앱 로그 신호:

- 마지막 컷에서는
  `speculative_preview_render_started`
  가 먼저 찍혔다.
- 그러나 곧바로
  `speculative_preview_wait_budget_exhausted wait_ms=720`
  이 남았고,
  직후 raw-original preview render가 시작됐다.
- 이번 최신 로그에는 예전 회차처럼
  `resident_first_visible_render_failed`
  나
  `preview-render-failed`
  가 함께 남지는 않았다.
- 따라서 이번 최신 후퇴는
  `명시적 resident failure`
  보다는
  `speculative lane이 제시간에 닫히지 못한 채 조용히 wait budget을 다 쓰고 raw close로 넘어가는 패턴`
  에 더 가깝다고 적는 편이 맞다.

이번 기록의 제품 결론:

- 최신 4컷 실로그 기준으로도 사용자의 체감은 맞다.
- first-visible은 여전히 약 `3초대`지만,
  고객이 실제로 기다리는 preset-applied result는 다시 약 `8초대`로 늘어났다.
- 이번 최신 회차는
  `개선`
  이 아니라
  `same-capture first-visible은 유지됐지만 preset-applied close는 더 늘어난 최신 후퇴`
  로 기록해야 한다.

## 다음 단계 기록

이번 회차에서는 구현 결론을 더 밀지 말고,
먼저 아래 확인 항목을 같은 형식으로 닫는 편이 맞다.

1. latest runtime에서
   `speculative_preview_render_started -> speculative_preview_wait_budget_exhausted -> raw-original preview-render-ready`
   순서가 마지막 컷만이 아니라 여러 컷에서 반복되는지 확인
2. speculative lane이 실제로는 완료되지만 host가 너무 빨리 포기하는지,
   아니면 실제로 `720ms` 안에 output을 만들지 못하는지
   session artifact와 stderr 기준으로 분리 확인
3. `legacy-canonical-scan`으로 잡히는 same-capture source가
   현재 latest booth path에서
   `first-visible only`
   로만 쓰이고 있는지,
   아니면 어떤 조건에서 later replacement close까지 소유할 수 있는지 다시 확인
4. latest booth package 한 세션만으로 아래 seam이 한 번에 닫히도록 증거를 다시 모으기
   - `request-capture`
   - `file-arrived`
   - `fast-preview-promoted`
   - `recent-session-pending-visible`
   - `speculative_preview_wait_budget_exhausted` 또는 동등 miss 신호
   - `preview-render-ready`
   - `recent-session-visible`
5. 다음 구현 판단 전 제품 기준을 다시 그대로 유지하기
   - 목표는 `same-capture first-visible` 자체가 아니라
     `preset-applied preview close`를 줄이는 것이다.
   - 따라서 다음 단계 판단도
     `8초대 close를 실제로 줄일 수 있는가`
     를 기준으로 해야 한다.

## 2026-04-04 최신 3컷 재시도 기록: 이번 시도는 더 느려졌고, resident first-visible 경로 실패가 다시 보인다

이번 회차는 사용자가 같은 런타임에서 추가로 3컷을 촬영한 뒤,
그 결과가 실제로 좋아졌는지 아닌지를 다시 기록하기 위한 회차였다.

최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30cf108b8edec`
였다.

이번 세션의 직접 수치:

1. `capture_20260404042833898_8d9626df95`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3019ms`
   - `capture acknowledged -> previewVisibleAtMs`: `8275ms`
   - `preview-render-ready elapsedMs`: `4522ms`

2. `capture_20260404042844064_a2ab8a5873`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2901ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7757ms`
   - `preview-render-ready elapsedMs`: `4124ms`

3. `capture_20260404042853753_bcb2770a92`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2949ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7804ms`
   - `preview-render-ready elapsedMs`: `4121ms`

이번 3컷 평균:

- same-capture first-visible: 약 `2956ms`
- preset-applied preview close: 약 `7945ms`

직전 같은 런타임 세션
`session_000000000018a30c240ea0a1f0`
와 비교하면 이번 시도는 더 나빠졌다.

- same-capture first-visible은 `3276ms -> 2956ms`로 약간 좋아졌다.
- 그러나 고객이 실제로 기다리는 preset-applied close는 `7151ms -> 7945ms`로 더 느려졌다.
- 즉 고객 체감 기준으로는 이번 시도를 `개선`으로 기록하면 안 되고,
  `더 느려진 재시도`로 기록하는 편이 맞다.

이번 세션에서 확인된 중요한 패턴:

- 3컷 모두 `timing-events.log`에
  `fast-preview-promoted kind=legacy-canonical-scan`
  이 먼저 찍혔다.
- 그 직후 3컷 모두 `preview-render-start`가 두 번 찍히고,
  이어서 `preview-render-failed` 뒤 `preview-render-ready`로 닫혔다.
- 최종 close는 여전히
  `widthCap=384;heightCap=384;sourceAsset=raw-original`
  render가 소유했다.

이번 세션의 helper 신호도 다시 같았다.

- 3컷 모두 `file-arrived`에는 `fastPreviewPath=null`이었다.
- 이후 약 `0.28s ~ 0.44s` 뒤에야
  `fast-preview-ready`가 `windows-shell-thumbnail`로 도착했다.
- 즉 usable same-capture source는 실제로 오고 있었지만,
  truthful close owner는 계속 raw-original render였다.

이번 회차에서 더 중요했던 실패 신호:

- 앱 로그에는 최신 세션 마지막 컷에서
  `speculative_preview_wait_budget_exhausted wait_ms=720`
  뒤에 raw-original render가 다시 시작된 기록이 남았다.
- 같은 컷에서
  `resident_first_visible_render_failed`
  도 남았고,
  detail에는 darktable-cli가 canonical preview jpg를 열지 못해
  `no images to export, aborting`
  했다고 적혀 있었다.
- `timing-events.log`가 3컷 모두 같은
  `preview-render-failed -> raw-original preview-render-ready`
  순서를 남긴 점을 보면,
  적어도 이번 세션에서는 resident first-visible 경로가 안정적으로 닫히지 못했다고 보는 편이 맞다.

이번 기록의 제품 결론:

- 최신 3컷 재시도는 `더 느려졌다`.
- fast preview는 여전히 약 3초 안팎으로 보이지만,
  고객이 기다리는 preset-applied result는 다시 약 7.8초 ~ 8.3초로 밀렸다.
- 따라서 이번 시도는
  `2초 이하 목표에 가까워진 회차`
  가 아니라
  `first-visible은 유지됐지만 preset-applied close는 오히려 후퇴한 회차`
  로 기록해야 한다.

## 다음 시도 기록

다음 시도에서는 구현 결론을 서두르기보다,
아래 관찰 포인트를 먼저 다시 같은 형식으로 닫는 편이 맞다.

1. 같은 세션에서 3컷 이상 촬영 후
   `preview-render-failed`가 매 컷 반복되는지 확인
2. `resident_first_visible_render_failed`가
   매번 같은 `can't open file ... no images to export`
   패턴인지 확인
3. `speculative_preview_wait_budget_exhausted`
   이후 실제 close owner가 항상 `raw-original`인지 확인
4. `fast-preview-promoted kind=legacy-canonical-scan`
   이 먼저 찍히는 구조가 이번 후퇴와 계속 같이 움직이는지 확인

다음 시도의 기록 기준은 그대로 유지한다.

- `capture acknowledged -> fastPreviewVisibleAtMs`
- `capture acknowledged -> previewVisibleAtMs`
- `preview-render-ready elapsedMs`
- `preview-render-failed` 유무
- `resident_first_visible_render_failed` 유무
- latest helper의 `fast-preview-ready observedAt - file-arrived` 차이

## 2026-04-04 최신 같은 런타임 재검증 기록: runtime drift는 해소됐지만, 여전히 2초 아래는 아니고 병목은 late helper handoff miss와 warm-up 실패 쪽이 더 가깝다

이번 회차는 사용자가 실제로
`C:\Code\Project\Boothy_thumbnail-latency-seam-reinstrumentation`
에서 `pnpm run dev:desktop:stable`로 실행한 뒤 남긴 최신 세션을 기준으로 다시 닫았다.

최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30c240ea0a1f0`
였고,
총 4개 capture가 남아 있었다.

이번 최신 세션의 직접 수치:

1. `capture_20260404041353782_b02b35b3be`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3197ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6876ms`
   - `preview-render-ready elapsedMs`: `3920ms`

2. `capture_20260404041402672_b65421f422`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3815ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7682ms`
   - `preview-render-ready elapsedMs`: `4021ms`

3. `capture_20260404041411637_bad1cf7ca8`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3110ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7117ms`
   - `preview-render-ready elapsedMs`: `4121ms`

4. `capture_20260404041420047_b651a6383c`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2982ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6927ms`
   - `preview-render-ready elapsedMs`: `4031ms`

이번 최신 세션 평균은 다음처럼 정리하는 편이 맞다.

- same-capture first-visible: 약 `3276ms`
- preset-applied preview close: 약 `7151ms`

이번 재검증에서 긍정적으로 바뀐 점:

- latest render detail은 더 이상 예전 `512px + --disable-opencl`이 아니었다.
- 실제 latest `timing-events.log`는 4컷 모두
  `widthCap=384;heightCap=384;sourceAsset=raw-original`
  을 남겼다.
- 즉 이번부터는 runtime drift보다
  `현재 브랜치 자체의 실제 성능`
  을 보는 편이 맞다.

이번 재검증에서 더 중요한 병목:

- helper artifact를 같이 보면 4컷 모두
  `fast-preview-ready`는 실제로 도착했다.
- 하지만 `file-arrived`에는 계속 `fastPreviewPath=null`이었고,
  usable same-capture preview는 그 뒤 약 `0.57s ~ 0.71s` 후에야
  `windows-shell-thumbnail`로 닫혔다.
- 현재 host는 late helper preview를 기다리는 예산이 너무 짧아서,
  이 경로를 놓친 뒤 다시 RAW truth lane으로 내려가고 있었다.
- 결과적으로 사용자는 same-capture 이미지를 먼저 볼 수는 있지만,
  프리셋 적용 close owner는 계속 `raw-original` render였다.

이번 회차에서 함께 확인한 추가 문제:

- 앱 로그에는 preview worker warm-up 실패가 남아 있었다.
- 원인은
  `.boothy-darktable/preview/warmup/preview-renderer-warmup-source.png`
  가 darktable/libpng 기준으로는 손상된 warm-up source였기 때문이다.
- 즉 resident worker와 preview runtime을 미리 데우려던 경로가
  현장에서는 실제로 실패하고 있었을 가능성이 높다.

이번 기록의 결론:

- 최신 런타임 정렬은 성공했다.
- 그러나 최신 실측은 아직도
  `first-visible 약 3.3초`,
  `preset-applied close 약 7.2초`
  수준이다.
- 제품 목표인 `2초 이하`에 맞추려면,
  단순 RAW cap 축소보다
  `late helper handoff를 놓치지 않게 하고`,
  `preview worker warm-up을 실제 장비에서 정상 복구하는 쪽`
  이 우선순위가 더 높다.

## 2026-04-04 최신 실검증 기록: 현재 성과는 worker 도입 전보다 낫다고 보기 어렵고, latest runtime은 여전히 예전 RAW truth lane에 묶여 있다

최신 사용자 판단은 분명했다.

- `최근 세션`의 프리셋 적용 사진은 여전히 느리다.
- 이번 상태는 worker 구성 변경 전보다 좋아졌다고 보기 어렵다.
- 먼저 보이는 same-capture 이미지는 있어도, 고객이 실제로 기다리는 `preset-applied preview close`는 그대로 늦다.

이번 회차에서 확인한 최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30ad7a9bbd20c`
였고,
최신 앱 로그 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)의 마지막 기록 시각은
`2026-04-04 12:50:38 KST`
였다.

이 세션에는 총 4개 capture가 남아 있었고,
4개 모두 패턴이 거의 같았다.

1. `capture_20260404035005875_14bcd041c8`
   - `request -> file-arrived`: `3324ms`
   - `request -> fast-preview-ready`: `4154ms`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3131ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6836ms`

2. `capture_20260404035014073_d3a71f887b`
   - `request -> file-arrived`: `2570ms`
   - `request -> fast-preview-ready`: `3191ms`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `3045ms`
   - `capture acknowledged -> previewVisibleAtMs`: `7249ms`

3. `capture_20260404035022689_9b58be3806`
   - `request -> file-arrived`: `3014ms`
   - `request -> fast-preview-ready`: `3588ms`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2817ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6859ms`
   - 글로벌 앱 로그에도 같은 컷이 `capture_preview_ready elapsed_ms=6859`로 남아 있었다.

4. `capture_20260404035030804_6684c2b560`
   - `request -> file-arrived`: `3240ms`
   - `request -> fast-preview-ready`: `3818ms`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: `2950ms`
   - `capture acknowledged -> previewVisibleAtMs`: `6973ms`
   - 글로벌 앱 로그에도 같은 컷이 `capture_preview_ready elapsed_ms=6973`로 남아 있었다.

이번 최신 세션의 직접 요약:

- same-capture first-visible은 대체로 `2.8s ~ 3.1s`였다.
- 그러나 고객이 기다리는 preset-applied preview close는 `6.8s ~ 7.2s`였다.
- 4컷 평균도
  - `capture acknowledged -> fastPreviewVisibleAtMs`: 약 `2986ms`
  - `capture acknowledged -> previewVisibleAtMs`: 약 `6979ms`
  로,
  worker topology가 고객 핵심 기다림을 절반 이하로 줄였다고 보기는 어렵다.

이번 로그에서 특히 중요했던 기술 관찰:

- 최신 `diagnostics/timing-events.log`의 4개 preview render detail은 모두
  `sourceAsset=raw-original`
  이었다.
- 같은 detail에는 계속
  `widthCap=512;heightCap=512`
  와
  `--disable-opencl`
  이 남아 있었다.
- 즉 latest runtime 기준에서는
  `resident worker가 있더라도 preset-applied truth lane 자체는 여전히 예전 RAW 512 preview render로 닫히고 있다`
  고 보는 편이 맞다.

이번 최신 세션에서 보인 추가 신호:

- helper 이벤트는 4컷 모두 `fast-preview-ready`를 남겼고,
  source kind는 `windows-shell-thumbnail`이었다.
- 그런데 host 앱 로그에는 마지막 컷에서도
  `helper_fast_preview_wait_budget_exhausted wait_ms=120`
  뒤에 바로 RAW preview render가 시작됐다.
- 즉 fast preview는 실제로 오고 있지만,
  고객이 기다리는 preset-applied close는 계속 fast preview가 아니라 RAW render가 소유하고 있었다.

이번 회차에서 특히 중요하게 봐야 할 drift:

- 현재 워킹트리 기준 preview baseline은 더 이상
  `512px + --disable-opencl`
  이 아니고,
  booth-safe 기본값도 그 조합을 유지하지 않도록 바뀌어 있다.
- 그런데 최신 실장비 증거는 여전히
  `512px + --disable-opencl + raw-original`
  을 가리켰다.
- 따라서 이번 latest validation은
  `현재 저장소의 기대 경로가 실제 booth runtime에 완전히 반영되지 않았거나`,
  `실제 반영됐더라도 고객 핵심 지연을 줄이는 경로가 아니라 여전히 예전 truth lane이 실행되고 있다`
  둘 중 하나로 보는 편이 맞다.

이번 세션에서 계측 공백도 다시 확인됐다.

- story 1.10이 요구한
  `request-capture -> file-arrived -> fast-preview-visible -> preview-render-start -> capture_preview_ready -> recent-session-visible`
  전체 seam이 같은 `timing-events.log`에 남아 있지 않았다.
- 실제 최신 세션 `timing-events.log`에는 `preview-render-start`, `preview-render-ready`만 남았고,
  나머지 경계는 앱 로그와 helper artifacts를 함께 봐야 닫혔다.
- 즉 지금도 latest booth package 하나만으로 병목을 바로 닫기에는 계측이 아직 충분하지 않다.

이번 최신 기록의 결론:

- `session_000000000018a30ad7a9bbd20c` 기준 최신 제품 상태는
  `same-capture first-visible은 약 3초`,
  `preset-applied preview close는 여전히 약 7초`
  로 정리하는 편이 맞다.
- 따라서 현재 성과는
  `worker 도입 뒤 제품 목표 달성`이 아니라
  `먼저 보이는 사진은 생겼지만 고객이 실제로 기다리는 프리셋 적용 결과는 여전히 느림`
  으로 기록해야 한다.
- 이번 최신 로그는 또한
  `실제 booth runtime이 아직 예전 RAW 512 truth lane을 쓰고 있다`
  는 drift 의심까지 함께 보여 준다.

## 2026-04-04 추가 환경 대조 기록: 최신 사용자가 본 느린 결과는 현재 브랜치보다 `C:\Code\Project\Boothy` 런타임과 더 정확히 맞는다

이번 회차에서는
`왜 latest session이 여전히 raw-original 512 / --disable-opencl 경로를 가리키는가`
를 코드와 실행 환경까지 대조해 봤다.

결론부터 적으면,
최신 사용자가 본 느린 결과는 현재 작업 중인 브랜치
`C:\Code\Project\Boothy_thumbnail-latency-seam-reinstrumentation`
보다,
별도로 존재하는 원본 저장소
`C:\Code\Project\Boothy`
의 런타임과 더 정확히 맞는다.

이번에 직접 확인한 사실:

- 현재 작업 브랜치 `src-tauri/src/render/mod.rs`는
  booth-safe preview cap을 `384`로 두고,
  기본 profile에서 `disable_opencl=false`로 잠겨 있다.
- 반면 원본 저장소 `C:\Code\Project\Boothy\src-tauri\src\render\mod.rs`는
  아직도 `RAW_PREVIEW_MAX_WIDTH_PX=512`,
  preview invocation 기본 인자에 `--disable-opencl`을 넣고 있었다.
- 최신 세션
  `session_000000000018a30ad7a9bbd20c`
  의 `timing-events.log`는 실제로
  `widthCap=512;heightCap=512;sourceAsset=raw-original;... --disable-opencl`
  를 남겼다.
- 즉 최신 실측 증거는 현재 브랜치보다
  원본 저장소의 preview lane과 더 정확히 일치했다.

이번 회차에서 함께 본 실행 환경 사실:

- 현재 시점에 떠 있던 프로세스는 `boothy-selectroom`뿐이었고,
  booth 앱 프로세스는 살아 있지 않았다.
- 따라서 사용자가 방금 본 느린 체감은
  현재 이 브랜치의 최신 빌드를 계속 띄운 상태라기보다,
  이전에 다른 저장소 또는 다른 빌드 산출물로 실행했던 결과일 가능성이 높다.
- 원본 저장소 `C:\Code\Project\Boothy\src-tauri\target\debug\boothy.exe`
  의 마지막 수정 시각은 `2026-04-03 16:59:50 KST`였고,
  latest log가 가리키는 구현 성격과도 맞아 떨어졌다.

이번 환경 대조가 주는 제품 판단:

- 현재 문제는 단순히 `worker 구조가 생각보다 덜 빠르다`만이 아니다.
- 더 직접적인 문제는
  `우리가 개선 중인 브랜치와 사용자가 실제로 검증한 런타임이 서로 다를 가능성`
  이 매우 높다는 점이다.
- 즉 지금 상태에서는
  `개선 브랜치의 성능이 나쁘다`
  와
  `실제로는 예전 런타임을 검증했다`
  가 섞여 있어,
  제품 판단과 코드 판단이 서로 어긋날 수 있다.

이번 메모의 결론:

- latest booth evidence는 현재 작업 브랜치보다 `C:\Code\Project\Boothy` 쪽 runtime과 더 강하게 일치한다.
- 따라서 다음 단계는
  `어느 저장소/빌드를 실제 booth 검증 대상으로 삼을지`를 먼저 고정하고,
  그 동일 runtime에서 다시 측정해야 한다.
- 그렇지 않으면 worker 구조 자체의 성과와 deployment drift를 구분할 수 없다.

## 2026-04-04 최신 로그 기록: 최근 세션의 preset-applied photo는 여전히 느리고, worker 전과 큰 차이가 없다고 보는 편이 맞다

최신 사용자 피드백은 간단했다.

- `최근 세션`에 뜨는 프리셋 적용 사진은 아직 너무 늦다.
- 체감상으로는 worker 도입 전과 큰 차이가 없다고 느껴진다.

이번 회차는 구현이 아니라,
이 판단이 최신 로그와 실제 session artifact로도 맞는지 닫기 위한 기록이다.

이번에 기준으로 본 최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a30a45d92e1e74`
였다.

이 세션에서 확인된 제품 사실:

- 세션 안에는 총 2개 capture가 남아 있었다.
- 둘 다 same-capture first-visible은 먼저 확보됐지만,
  `previewReady`로 닫히는 preset-applied photo는 여전히 느렸다.
- latest app log와 session manifest가 같은 숫자를 가리켰다.

이번 세션의 핵심 수치:

1. 첫 번째 컷 `capture_20260404033938654_2f79032f7d`
   - `request -> file-arrived`: 약 `3.1s`
   - `request -> fast-preview-ready`: 약 `3.9s`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: 약 `3.1s`
   - `capture acknowledged -> previewVisibleAtMs`: 약 `6983ms`
   - 즉 first-visible은 먼저 왔지만,
     고객이 실제로 기다리는 preset-applied close는 여전히 약 `7.0s`였다.

2. 두 번째 컷 `capture_20260404033946807_75ec5c5f34`
   - `request -> file-arrived`: 약 `3.3s`
   - `request -> fast-preview-ready`: 약 `3.9s`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: 약 `2989ms`
   - `capture acknowledged -> previewVisibleAtMs`: 약 `6838ms`
   - 글로벌 앱 로그에도 같은 컷이
     `capture_preview_ready elapsed_ms=6838`
     로 남아 있었다.

이번 로그가 주는 직접 결론:

- 최신 기준에서도 preset-applied photo close는 대략 `6.8s ~ 7.0s` 수준이다.
- 즉 이번 상태를 `체감상 빨라졌다`보다
  `여전히 제품 기준 미달`로 기록하는 편이 맞다.
- 사용자가 느낀 `별 변함이 없다`는 판단은 과장이 아니라 최신 로그와 맞는다.

이번 세션에서 특히 중요했던 기술적 관찰:

- 최신 `timing-events.log`의 preview render detail은 두 컷 모두
  `sourceAsset=raw-original`
  이었다.
- 즉 지금 worker 변경은 first-visible topology에는 영향을 줬더라도,
  고객이 기다리는 render-backed preset-applied truth lane은 여전히
  RAW original 중심으로 닫히고 있었다.
- 같은 로그 detail에는 여전히
  `widthCap=512;heightCap=512`
  와 `--disable-opencl`
  이 남아 있었다.
- 다시 말해 이번 최신 실측 기준으로는
  `worker를 넣었지만 preset-applied photo 자체가 더 가벼워졌다고 보기 어렵다`
  가 더 정확하다.

이번 세션에서 함께 보인 제품 흐름:

- latest app log에는 먼저
  `recent-session-pending-visible`
  이 찍혔다.
- 그 뒤 실제 `recent-session-visible previewKind=preset-applied-preview`
  는 `capture_preview_ready` 직후에야 닫혔다.
- 즉 고객 입장에서는 `먼저 뭔가 보이긴 하지만, 프리셋 적용된 최근 세션 사진은 여전히 늦게 도착한다`
  는 체감이 자연스럽다.

이번 메모의 결론:

- 최신 세션 `session_000000000018a30a45d92e1e74`는
  worker 도입 이후에도 recent-session preset-applied photo가 여전히 약 `6.8s ~ 7.0s`
  수준에서 닫히는 모습을 보여 줬다.
- 따라서 이번 시점의 최신 제품 판단은
  `별 변함 없음`, `여전히 느림`, `최근 세션의 프리셋 적용 사진 속도는 아직 개선 완료가 아님`
  이다.
- 다음 판단 기준도 그대로다.
  `same-capture first-visible`이 아니라
  `preset-applied preview close`를 실제로 얼마나 줄였는지로 봐야 한다.

## 2026-04-04 최신 사용자 보정: worker 도입 뒤에도 `최근 세션`의 preset-applied photo는 아직 충분히 빨라지지 않았다

최신 사용자 피드백은 이전의 낙관적 메모를 바로 보정해야 할 정도로 분명했다.

- `최근 세션`에 뜨는 프리셋 적용 사진은 여전히 느리다.
- 체감상으로는 worker 도입 전과 큰 차이가 없다고 느껴진다.
- 즉 이번 시점의 실제 병목은 `same-capture first-visible`보다 `preset-applied preview close` 쪽으로 다시 좁혀 봐야 한다.

이번 회차에서 현재 구현을 다시 대조해 본 결과:

- 지금까지의 resident worker 변경은 주로 `먼저 보이는 사진`과 그 준비 경로를 줄이는 데 더 가깝다.
- 반면 `previewReady`를 닫는 render-backed truth lane은 여전히 small rail용이라고 해도 체감상 무거운 preview artifact를 만들고 있었다.
- 그래서 사용자 입장에서는 `사진이 먼저 보이는 것`은 달라졌어도, `프리셋 적용된 결과가 늦게 닫힌다`는 핵심 불만이 그대로 남을 수 있었다.

이번에 바로 반영한 조치:

- recent-session rail의 preset-applied preview truth lane 크기를 한 단계 더 낮췄다.
- booth-safe render-backed preview profile cap을 `512 -> 384`로 줄여, `previewReady`를 닫는 실제 render 비용 자체를 낮추도록 조정했다.
- 목적은 first-visible worker 체감이 약한 이유를 `대기 정책`이 아니라 `truth lane 산출물 크기`에서 먼저 줄이는 것이다.

이번 조정의 제품 의미:

- 이 변경은 `final render` 품질을 낮추는 것이 아니다.
- 고객이 `최근 세션` 레일에서 확인하는 프리셋 적용 결과만 더 작은 booth-safe preview artifact로 빨리 닫히게 하려는 조정이다.
- 즉 이번 회차의 질문은 `worker가 있느냐`보다 `worker 뒤에 닫히는 preset-applied preview 자체가 충분히 가벼운가`에 더 가깝다.

이번 회차 검증:

- preview invocation/계약 회귀는 Rust 테스트로 다시 확인한다.
- 자동 검증이 유지되더라도, 최종 판단은 여전히 실장비에서 `capture_preview_ready elapsed_ms`가 실제로 내려가는지로 닫아야 한다.

## 2026-04-04 현재 저장소 기준 최신 상태: 구조 변경은 반영됐고, 제품 종료 판단에는 실장비 증빙이 아직 남아 있다

이 문서의 맨 위 기록은 현장 사용자 체감 메모로는 유효하다.
다만 현재 워킹트리까지 포함한 최신 상태를 제품 판단용으로 다시 정리하면,
이제는 `문제 확인 단계`가 아니라 `구조 변경 반영 + 자동 검증 통과 + 하드웨어 최종 증빙 대기`로 보는 편이 맞다.

현재 저장소에서 이미 반영된 것:

- Story 1.10 `known-good preview lane 복구와 상주형 first-visible worker 도입`이 `in-progress`로 올라가 있다.
- booth 기본 preview lane은 실장비 비호환 실험 플래그를 기본 경로에서 빼고,
  known-good baseline 쪽으로 다시 고정됐다.
- recent-session first-visible 경로는 per-capture one-shot spawn만 의존하지 않고,
  resident first-visible worker 우선 topology로 옮겨졌다.
- same-capture first-visible이 먼저 보여도 `previewReady` truth owner는 계속 later render-backed close만 소유하도록 다시 잠갔다.
- `capture-download-timeout` 뒤 helper가 회복했는데도 booth가 계속 멈춰 보이던
  `phone-required` 고착 회귀를 풀도록 recovery 규칙이 보강됐다.

현재 저장소에서 확인된 자동 검증:

- `2026-04-04` 현재 `cargo test --test capture_readiness -- --nocapture --test-threads=1`는 `62 passed, 0 failed`였다.
- 이 검증에는 same-capture fast preview handoff, later canonical replacement, recent-session-visible timing mirror,
  speculative wait non-blocking, `capture-download-timeout` recovery 같은 최근 회귀 방지가 포함된다.

아직 제품 완료로 단정하면 안 되는 이유:

- Story 1.10 자체가 아직 `done`이 아니라 `in-progress`다.
- story가 요구한 `한 개의 최신 실장비 session package만 봐도 request-capture -> file-arrived -> first-visible -> preview ready -> recent-session-visible을 다시 닫을 수 있는지`
  는 아직 hardware evidence로 최종 닫히지 않았다.
- 프런트 회귀 검증은 현재 워크스페이스에 `node_modules`가 없어 이번 회차에 다시 돌리지 못했다.

따라서 이번 시점의 제품 판단은 이렇게 남기는 편이 맞다:

- 현장 체감상 좋아졌다는 최신 사용자 메모는 유지한다.
- 동시에 저장소 기준 공식 상태는 `핵심 구조 변경과 자동 회귀 방지는 반영됨`이지만,
  `연속 촬영 안정성 / 2초 이하 체감 / session-seam 증빙`은 실장비에서 한 번 더 닫혀야 한다.
- 즉 지금 상태는 `해결 완료`보다 `제품 합격선에 가까워진 유력 후보`에 더 가깝다.

## 2026-04-04 최신 사용자 확인: 연속 촬영 기능은 복구됐고, preset-applied preview 체감도 2초 이하로 내려왔다

이번 회차의 최신 사용자 확인은 비교적 분명했다.

- 연속 촬영 기능 자체는 이제 동작한다.
- 촬영 직후 같은 컷 사진이 recent-session rail에 바로 올라오는 흐름도 확인됐다.
- 고객이 실제로 기다리는 `프리셋 적용된 결과`도 이제 체감상 `2초 이하`로 느껴진다.

이번 확인이 특히 중요한 이유:

- 최근 회차에서 resident first-visible worker / topology 변경을 넣은 뒤,
  이번에는 사용자가 실제 체감 기준으로도 `2초 이하`라고 확인했다.
- 즉 이번 구조 변경은 단순히 `연속 촬영이 다시 굴러간다`를 넘어서,
  우리가 목표로 삼은 `preset-applied preview latency 단축`에도 유의미한 개선을 만든 것으로 기록해야 한다.

이번 시점의 제품 의미:

- 고객은 이미 `사진이 바로 보이느냐`와 `프리셋 적용 결과가 빨리 닫히느냐`를 구분해서 느끼고 있었고,
  이번 확인은 두 기준 모두가 함께 개선됐다는 의미가 있다.
- 현재 상태는 `same-capture first-visible`뿐 아니라
  핵심 목표였던 `preset-applied preview를 충분히 빨리 보여 주기`도
  체감 기준에서는 일단 합격선에 가까워졌다고 볼 수 있다.
- 따라서 이번 회차 결과는
  `연속 촬영 복구 + immediate first-visible 확보 + preset-applied preview 체감 2초 이하`
  로 기록하는 편이 맞다.

이번 메모의 결론:

- 현재 최신 상태는 `연속 촬영은 동작`, `방금 찍은 사진도 바로 보임`, `프리셋 적용 결과도 체감상 2초 이하`다.
- 이번 회차 기록은 구조 변경 이후 제품 체감이 실제로 좋아졌다는 최신 사용자 판단을 남기는 용도다.
- 이번 회차에서는 추가 구현 없이 이 제품 판단만 최신 히스토리로 남긴다.

## 2026-04-04 추가 실검증 로그 기록: 첫 컷은 약 3.3초대 first-visible, 둘째는 20초대, 셋째는 31초 timeout으로 종료

이번 회차는 수정이 아니라,
사용자 최신 제보
`첫 번째 샷은 빨리 보였지만 2, 3번째 샷은 체감상 20초 이상 걸렸다`
를 실제 session evidence로 다시 닫기 위한 기록이다.

이번에 기준으로 본 최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3058db863a0e0`
였다.

이 세션에서 확인된 request-level 사실:

- 요청은 총 3개였다.
  - `request_000000000000064e98f99e0600`
  - `request_000000000000064e98fa2519f0`
  - `request_000000000000064e98fb823768`
- 하지만 `session.json`에 최종 capture로 남은 것은 2개뿐이었다.
- 세 번째 요청은 capture record를 남기지 못하고 세션이 `phone-required`로 종료됐다.

이번 세션의 숫자는 사용자 체감과 거의 일치했다.

1. 첫 번째 샷
   - `request -> file-arrived`: 약 `3195ms`
   - `request -> fast-preview-ready`: 약 `3915ms`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: 약 `3297ms`
   - `capture acknowledged -> previewVisibleAtMs`: 약 `13796ms`
   - 즉 첫 컷은 truthful preview close는 여전히 느렸지만,
     same-capture first-visible은 약 `3.3s`대에 들어와 사용자가
     `빨리 보였다`고 느낄 수 있는 패턴이었다.

2. 두 번째 샷
   - `request -> file-arrived`: 약 `19898ms`
   - `request -> fast-preview-ready`: 약 `20521ms`
   - `capture acknowledged -> fastPreviewVisibleAtMs`: 약 `20127ms`
   - `capture acknowledged -> previewVisibleAtMs`: 약 `24199ms`
   - 즉 둘째 샷의 `20초+` 체감은 로그로도 확인됐다.
   - 특히 이 컷은 preview render 자체보다,
     `request-capture -> file-arrived` 경계가 약 `19.9s`까지 늘어난 것이 더 큰 병목이었다.
   - 같은 세션 `timing-events.log` 기준 이 컷의 render leg는
     `preview-render-start -> preview-render-ready`가 약 `4226ms`였다.
   - 다시 말해 둘째 컷의 주병목은 preview close 자체보다
     helper completion / RAW handoff 지연 쪽에 더 가까웠다.

3. 세 번째 샷
   - 세 번째 요청 `request_000000000000064e98fb823768`은
     `camera-helper-events.jsonl`에 `capture-accepted`만 남았다.
   - 그 뒤 `file-arrived`나 `fast-preview-ready`는 끝내 오지 않았다.
   - 대신 같은 세션 helper 이벤트는
     `recovery-status(detailCode=capture-download-timeout)`와
     `helper-error(detailCode=capture-download-timeout)`로 종료됐다.
   - `request -> recovery-status`는 약 `31144ms`,
     `request -> helper-error`는 약 `31146ms`였다.
   - 즉 셋째 샷은 고객 체감상 `20초+`가 아니라,
     실제로는 약 `31초`를 기다린 뒤 timeout으로 실패한 셈이다.

이번 세션이 보여 준 제품 의미:

- 사용자 제보
  `첫 번째는 빨랐고, 둘째와 셋째는 20초 이상 걸렸다`
  는 과장이 아니라 최신 세션 evidence와 맞는다.
- 첫 컷은 first-visible이 약 `3.3s`대라 상대적으로 빨랐다.
- 둘째 컷은 first-visible이 실제로 약 `20.1s`,
  truthful preview close는 약 `24.2s`였다.
- 셋째 컷은 preview lane까지 가지도 못하고
  `capture-download-timeout`으로 약 `31.1s` 뒤 실패했다.

이번 세션에서 특히 중요했던 판단:

- 둘째 컷 `20초+`는 `preview-render`만의 문제로 보기 어렵다.
- 실제 병목은 `request-capture -> file-arrived`가 약 `19.9s`까지 늘어난 helper completion 경계였다.
- 셋째 컷은 그 경계가 아예 닫히지 못하고 timeout으로 무너졌다.
- 따라서 이 최신 세션은
  `preview lane이 느리다`만이 아니라,
  `연속 촬영에서 helper completion boundary가 급격히 악화된다`
  는 증거로도 봐야 한다.

이번 회차에서 같이 남겨 둘 계측 공백:

- 이 세션 `timing-events.log`에는 `preview-render-start`, `preview-render-ready`만 남았고,
  story가 요구한 `request-capture`, `file-arrived`, `fast-preview-visible`, `recent-session-visible`는 닫히지 않았다.
- 그래서 이번 판단은
  `camera-helper-requests.jsonl`,
  `camera-helper-events.jsonl`,
  `session.json`
  조합으로 닫았다.

이번 메모의 결론:

- 최신 session `session_000000000018a3058db863a0e0`는
  `1번째 약 3.3s first-visible -> 2번째 약 20.1s first-visible / 24.2s preview close -> 3번째 약 31.1s timeout`
  패턴을 실제로 보여 줬다.
- 즉 현재 문제는 단순히 `조금 느리다`가 아니라,
  연속 촬영에서 second/third shot latency가 제품 허용 범위를 완전히 벗어나는 상태다.
- 다음 세션에서는 이 기록을 기준으로,
  preview lane과 helper completion boundary 중 어느 쪽을 먼저 바로잡을지 다시 판단해야 한다.

## 2026-04-04 후속 로그 분석: 첫 촬영 뒤 두 번째 샷 정지는 preview worker deadlock보다 `capture-download-timeout -> phone-required 고착`이 직접 원인이었다

사용자 최신 제보는 이랬다.

- 첫 번째 촬영은 성공적으로 진행됐다.
- 그런데 두 번째 샷에서 제품이 멈췄다.

이번 회차에서 실제로 다시 확인한 최신 세션은
`C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a3018959f17008`였다.

이번 로그에서 먼저 확인된 사실:

- 첫 번째 샷 `capture_20260404005935044_075740bc3a`는 실제로 닫혔다.
- 글로벌 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에는
  `2026-04-04 00:59:48 KST` 기준
  `capture_preview_ready elapsed_ms=14926`가 남아 있었다.
- 같은 세션 `camera-helper-events.jsonl`에도 첫 요청은
  `capture-accepted -> file-arrived -> fast-preview-ready`까지 정상으로 남아 있었다.

하지만 두 번째 샷은 다른 패턴이었다.

- 둘째 요청 `request_000000000000064e97f2d99910`은 `capture-accepted`까지만 남았다.
- 그 뒤 같은 세션 helper 이벤트는
  `recovery-status(detailCode=capture-download-timeout)`와
  `helper-error(detailCode=capture-download-timeout)`로 종료됐다.
- 중요한 점은, 그 직후 `camera-helper-status.json` 최종 상태가 다시
  `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`로 회복했다는 것이다.

즉 이번 회차의 직접 원인은 이렇게 정리됐다.

- 두 번째 샷이 멈춘 직접 원인은 resident preview worker queue deadlock이 아니라,
  helper completion boundary에서 `file-arrived`를 끝내 닫지 못한
  `capture-download-timeout`이었다.
- 그런데 제품이 실제로 멈춘 것처럼 보인 더 직접적인 이유는,
  helper는 이미 `ready/healthy`로 회복했는데 host session lifecycle이
  `phone-required`에 고정된 채 풀리지 않았기 때문이다.
- 다시 말해 이번 증상은
  `촬영 timeout 1회 -> helper 회복 -> booth는 계속 정지 상태`
  순서의 제품 회귀였다.

이번 회차에서 제거한 오진:

- 처음에는 `second shot freeze`라서 resident first-visible worker 정지,
  speculative preview lock 잔류, preview queue saturation을 먼저 의심할 수 있었다.
- 하지만 실제 최신 세션 evidence에는 둘째 샷에 대해
  `preview-render-queue-saturated`나 `resident_first_visible_render_enqueue_failed`가 아니라,
  helper 쪽 `capture-download-timeout`만 남았다.
- 따라서 이번 회차의 우선 문제는 preview lane보다
  `timeout 뒤 session recovery truth를 어떻게 제품에 반영하느냐`였다.

이번에 바로 반영한 조치:

1. host recovery 규칙을 보강했다.
   - helper live truth가 다시 `fresh / matched / ready / healthy`로 회복하면,
     기존 retryable focus failure뿐 아니라 `capture-download-timeout`도
     `phone-required` 고착에서 자동 복귀할 수 있게 했다.
2. 회귀 테스트를 추가했다.
   - `capture-download-timeout` 이후 helper가 다시 정상 상태를 쓰면
     readiness가 `ready`, lifecycle stage가 `capture-ready`로 복귀하는지 테스트를 추가했다.
3. 기존 보호 경계는 유지했다.
   - helper가 아직 회복하지 않은 진짜 timeout 상태는 계속 `phone-required`를 유지하는지도 함께 다시 확인했다.

이번 회차에서 실행해 통과한 검증:

- `cargo test --test capture_readiness readiness_releases_phone_required_after_capture_download_timeout_recovers -- --nocapture`
- `cargo test --test capture_readiness readiness_releases_phone_required_after_retryable_focus_failure_recovers -- --nocapture`
- `cargo test --test capture_readiness capture_flow_times_out_when_helper_accepts_but_no_file_arrives -- --nocapture`

이번 메모의 제품 의미:

- 이번 수정은 `두 번째 샷 timeout 자체를 없앴다`기보다,
  timeout 뒤 helper가 이미 살아났는데 제품이 계속 멈춰 보이는 회귀를 먼저 제거한 것이다.
- 따라서 다음 실장비 확인에서는
  `두 번째 샷 timeout 재발 여부`와
  `재발하더라도 booth가 영구 정지 상태에 남지 않는지`
  를 분리해서 봐야 한다.

이번 시점에 남아 있는 별도 이슈:

- 첫 번째 샷 preview latency는 여전히 매우 길다.
  이번 최신 세션에서도 첫 컷 `capture_preview_ready elapsed_ms=14926`가 남아 있었다.
- 같은 세션 `timing-events.log`에는 여전히
  `request-capture`, `file-arrived`, `fast-preview-visible`, `recent-session-visible`가
  session-scoped seam으로 충분히 닫히지 않았다.
- 또한 최신 세션 preview render detail에는 아직 `--disable-opencl`이 남아 있어,
  known-good booth invocation 복구 과제도 계속 유효하다.

이번 메모의 결론:

- `second shot freeze`의 최신 직접 원인은
  `preview worker deadlock`보다 `capture-download-timeout 이후 phone-required 고착`이었다.
- 이번 회차 조치로 product stop 상태는 완화했지만,
  helper completion timeout 자체와 preview lane latency/correctness 문제는 별도 후속 과제로 남아 있다.

## 2026-04-04 실장비 재검증: 5컷 세션에서도 여전히 제품 기준 미달, 계측 공백과 preview lane 회귀 동시 확인

`2026-04-04 00:55 ~ 00:56 KST` 실장비 재테스트 기준 최신 세션은
`session_000000000018a2e3d9373fdd38`였다.

이번 재검증에서 바로 확인된 사실:

- helper 쪽 capture round-trip 자체는 5컷 모두 닫혔다.
- 최신 세션 `diagnostics/camera-helper-status.json` 최종 상태도 `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`였다.
- 즉 이번 회차의 본문제는 `Phone Required`나 helper timeout 재발이 아니라, recent-session first-visible과 preset-applied preview lane의 속도/안정성 쪽이었다.

이번 세션에서 남은 속도는 수치로도 분명했다:

- `camera-helper-requests.jsonl` / `camera-helper-events.jsonl` 기준 `request-capture -> file-arrived`는 평균 약 `3286ms`였다.
- 같은 기준 `request-capture -> fast-preview-ready`는 평균 약 `3863ms`였다.
- `session.json` 기준 5컷의 `capture acknowledged -> RAW persisted`는 평균 약 `3028ms`였다.
- 같은 세션에서 preview가 실제로 닫힌 4컷의 `capture acknowledged -> preview visible`은 평균 약 `9238ms`였다.
- 특히 최근 3컷은 `7616ms`, `7761ms`, `8189ms`로, warm 상태라고 보기 쉬운 구간에서도 여전히 `7.6s ~ 8.2s`였다.
- 최신 5번째 컷 `capture_20260403155615575_64db44e705`는 앱 로그 기준 `capture_preview_ready elapsed_ms=8189`, `recent-session-visible uiLagMs=48`이었다.

이번 로그가 주는 제품 의미:

- 이번에도 고객 체감 병목은 UI 반영이 아니라 preview 생성 자체였다.
- RAW handoff만 늦은 것이 아니라, 그 뒤 preset-applied preview lane도 여전히 너무 무겁다.
- 현재 수치에서는 미세 조정으로 `충분히 빨라졌다`고 보기 어렵다.

이번 회차에서 새로 확인된 correctness/운영 리스크:

- 같은 세션 첫 컷 `capture_20260403155533795_795031ecc2`는 `fastPreviewVisibleAtMs`가 기록됐고 preview 파일도 존재했지만, `session.json` 최종 상태가 `renderStatus=previewWaiting`으로 남아 first-visible replacement가 끝내 닫히지 않았다.
- 같은 세션 둘째 컷은 `timing-events.log`에 `preview-render-queue-saturated`가 반복해서 남았고, 실제 `capture acknowledged -> preview visible`이 `13385ms`까지 늘어났다.
- booth 장비의 preview stderr 로그에는 `warning: unknown option '--disable-opencl'`가 반복됐다.
- 같은 stderr 근거에는 `Magick ... Access violation`과 `can't open file ... renders/previews/...jpg`도 남아, 최근 preview lane 실험이 현재 booth의 darktable `5.4.0` 환경에서는 안정적이지 않다는 신호가 확인됐다.

이번 재검증에서 다시 드러난 계측 공백:

- 이 브랜치의 목적과 달리, 최신 세션 `diagnostics/timing-events.log`에는 `request-capture`, `file-arrived`, `fast-preview-visible`, `recent-session-visible`가 남지 않았다.
- 실제 최신 세션에서는 같은 정보가 `camera-helper-requests.jsonl`, `camera-helper-events.jsonl`, 글로벌 `Boothy.log`로는 확인되는데, per-session seam log에는 충분히 닫히지 못했다.
- 즉 다음 구조 변경 판단 전에 `최신 세션 timing log 하나만 보면 병목을 다시 닫을 수 있는 상태`는 아직 아니다.

이번 시점의 제품 판단:

- helper 안정성은 이번 회차에서 우선 합격선에 가깝다.
- 하지만 recent-session 속도는 여전히 허용 불가다.
- 동시에 preview lane correctness도 일부 다시 흔들렸다.
- 따라서 다음 단계는 단순 옵션 미세 조정이 아니라, `known-good correctness 복구 + 계측 복구 + 구조 변경 준비`가 함께 가야 한다.

그래서 다음 단계는 아래 순서로 준비한다:

1. preview invocation을 booth 실장비 기준 known-good 쪽으로 다시 고정한다.
   - 최신 stderr가 보여 준 `--disable-opencl` 비지원과 speculative/fast-preview-raster 실패 흔적은 더 이상 운영 경로에 남겨 두지 않는다.
   - 목표는 first capture가 다시 `previewWaiting`에 고착되지 않게 만드는 것이다.
2. per-session seam 계측을 실제 booth 세션에서도 다시 닫는다.
   - 최소한 같은 `timing-events.log` 안에 `request-capture`, `file-arrived`, `fast-preview-visible`, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`가 함께 남아야 한다.
   - 그래야 다음 구조 변경의 before/after를 한 파일에서 비교할 수 있다.
3. 구조 변경의 1차 후보는 그대로 `상주형 first-visible renderer worker`다.
   - 이번 세션에서도 preview render 자체가 steady-state로 약 `4.2s ~ 4.6s`였기 때문에, per-capture spawn과 queue 경합을 줄이는 topology 변경이 필요하다.
4. 다만 worker만으로 끝내지 않는다.
   - 이번 실장비에서 `fast-preview-ready`도 평균 약 `3.9s`였기 때문에, first-visible source 자체를 더 이르게 만들 후보도 함께 열어 둔다.
   - 즉 다음 구조 변경 준비 범위는 `same engine, different topology`에 더해 `camera-thumbnail/intermediate-preview first` 후보까지 포함한다.

이번 메모의 결론:

- 최신 실장비 테스트는 `미세 조정 단계 종료`를 다시 확인해 줬다.
- 다음 회차의 준비 완료 기준은 `preview lane correctness 복구`, `per-session seam log 복구`, `상주형 first-visible worker 설계 착수`다.

## 2026-04-03 실장비 업데이트

최신 실장비 재검증 결과,
최근 세션 썸네일 경로는 이제 설계 의도대로 동작한다.

확인된 점:

- `사진 찍기` 직후 `Preview Waiting`이 먼저 진입한다.
- 같은 촬영의 fast preview가 `현재 세션 사진` 최신 슬롯에 먼저 나타난다.
- 나중에 preset-applied preview가 같은 슬롯을 교체한다.
- 즉 Story 1.9가 목표로 한 `first-visible same-capture preview -> later preset replacement` 흐름은 실장비에서 재현됐다.

이번 확인으로 정리된 남은 문제:

- 최근 세션 썸네일 blank waiting 자체는 완화됐지만, 콜드스타트와 preset-applied preview 준비 시간은 여전히 체감상 길다.
- 따라서 다음 성능 작업의 중심은 `fast thumbnail이 보이느냐`가 아니라 `cold start`, `preview render warm-up`, `preset apply elapsed`를 줄이는 쪽으로 이동했다.
- 이 변경으로 고객 불안은 줄었지만, 전체 체감 속도는 아직 충분히 빠르다고 보기 어렵다.

연결 증거:

- 스토리 기록: [_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md](/C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/1-9-fast-preview-handoff와-xmp-preview-교체.md)
- 하드웨어 검증 장부: [_bmad-output/implementation-artifacts/hardware-validation-ledger.md](/C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/hardware-validation-ledger.md)

## 2026-04-03 후속 회귀 메모: `Preview Waiting` 고착

최신 사용자 확인에서 더 우선인 회귀가 새로 드러났다.

- `Preview Waiting`으로는 들어가지만, 선택된 프리셋이 적용된 preview가 끝내 올라오지 않고 그 상태에 머무는 경우가 있었다.

이번 회차에서 정리한 제품 판단:

- 이 증상은 단순 `조금 느리다`가 아니라, first-visible applied result 자체가 닫히지 않는 correctness 회귀다.
- 따라서 이 구간에서는 공격적인 latency cut보다 `preset-applied preview가 반드시 완료되거나 실패로 명확히 정리되는가`가 우선이다.

이번에 가장 가능성 높게 본 원인:

- 직전 회차에서 darktable preview invocation 자체를 더 공격적으로 줄이기 위해 launch contract를 건드렸다.
- 하지만 이 변경은 실장비에서 `선택된 프리셋이 적용된 preview replacement`의 안정성을 해쳤을 가능성이 높다.
- 즉 preview render hot path는 아직 `조금 더 가볍게`보다 `known-good invocation 유지`가 더 중요하다는 쪽으로 판단을 되돌렸다.

이번 회차에서 바로 반영한 방향:

- darktable preview invocation에서 공격적으로 추가한 옵션/출력 처리 변경은 되돌리고, 검증돼 있던 render contract를 우선 복구한다.
- fast preview reuse와 requestId 계측 강화는 유지하되, preset-applied replacement를 막을 수 있는 invocation 실험은 후순위로 내린다.

이번 메모의 제품 의미:

- 속도 최적화는 `Preview Waiting -> preset-applied preview` 승격을 깨지 않는 범위에서만 유효하다.
- 다음 실장비 확인의 1차 기준은 `더 빨라졌는가`보다 `같은 촬영의 프리셋 적용 결과가 다시 정상 교체되는가`다.

## 2026-04-03 로그 재확인: 남은 주병목은 RAW handoff

최신 보관 세션 로그를 다시 확인했다.

이번 확인에서 분명했던 점:

- 보관된 `session.json` 기준 recent capture들은 `capture acknowledged -> RAW persisted`가 대체로 약 `1.6s ~ 2.1s`였다.
- 같은 세션에서 `RAW persisted -> preview visible`은 대체로 약 `123ms ~ 125ms` 수준이었다.
- 즉 현재 남은 체감 지연의 더 큰 덩어리는 preview replacement보다 `카메라 -> host RAW handoff` 쪽에 더 가깝다.

이번 로그가 주는 제품 의미:

- 지금 단계에서 렌더 파라미터만 조금 더 줄이는 일은 체감 전체를 크게 바꾸기 어렵다.
- 더 큰 개선은 `같은 촬영 preview 준비`를 RAW handoff와 더 많이 겹치게 만들 수 있느냐에서 나온다.

그래서 이번 회차에서 바로 반영한 조정:

- helper의 same-capture camera thumbnail 시도를 `RAW 다운로드 완료 뒤`가 아니라 `RAW 다운로드 시작 전`으로 앞당겼다.
- 목표는 카메라가 thumbnail을 줄 수 있는 장비/상태에서, host가 speculative preview work를 더 일찍 시작하게 만드는 것이다.
- 이 조정은 `capture completion truth`는 유지하면서도, `RAW handoff`와 `preset preview 준비`를 더 많이 겹치게 하려는 목적이다.

다음 실장비 확인 기준:

- same-capture thumbnail이 살아 있는 장비에서는 `capture accepted -> preset-applied first visible`이 지금보다 더 짧아져야 한다.
- 특히 성공 기준은 단순 `previewReady가 된다`가 아니라, `RAW handoff가 끝나자마자 교체될 준비가 이미 앞당겨져 있는가`다.

## 2026-04-03 후속 사용자 재검증 메모

최신 사용자 앱 실행 재검증 결과:

- recent-session 썸네일 노출은 이전보다 **조금 빨라진 것처럼 느껴졌다.**
- 다만 사용자 평가는 여전히 **제품 기준으로는 부족하다** 쪽이었다.

이번 메모의 제품 의미:

- 이번 변경은 무효가 아니며, first-visible recent-session 쪽 체감 개선은 일부 있었던 것으로 본다.
- 하지만 이 수준만으로는 "충분히 빨라졌다"는 판단까지는 도달하지 못했다.
- 따라서 다음 회차는 correctness 재검토보다 **추가 latency 단축 작업**으로 이어져야 한다.

현재 시점의 최신 사용자 판단:

- `조금 개선됨`
- `아직 부족함`

즉 이 문서의 후속 우선순위는 그대로 유지한다.
fast preview 경로의 즉시성은 더 밀어야 하고,
동시에 preset-applied preview 준비 시간과 cold-start 비용도 계속 줄여야 한다.

## 2026-04-03 로그 재확인 후 추가 단축 조치

보관된 실제 런타임 세션 로그를 다시 확인했다.

이번 재확인에서 중요하게 보인 점:

- `C:\Users\KimYS\AppData\Local\com.tauri.dev\booth-runtime\sessions\...` 아래 최근 실세션 흔적에서는 `fast-preview-ready`가 거의 남아 있지 않았다.
- 남아 있는 helper 이벤트는 대부분 `capture-accepted`, `file-arrived` 중심이었다.
- 즉 직전 회차에서 넣은 `겹쳐 돌리기` 최적화는 맞는 방향이었지만, 실장비에서 same-capture fast preview가 충분히 빨리 안 만들어지면 체감 개선이 크게 보이기 어렵다.

이번 판단의 제품 의미:

- 고객이 느끼는 병목은 이제 `겹쳐서 시작하느냐` 하나만의 문제가 아니다.
- 더 중요한 것은 `같은 촬영 썸네일을 얼마나 높은 확률로 즉시 보여 주느냐`다.
- 그래서 이번 회차는 fast preview hit-rate를 먼저 올리고, 그 다음 첫 노출용 preset preview 자체를 더 가볍게 만드는 쪽으로 추가 조정했다.

이번에 바로 반영한 조치:

- helper가 카메라 thumbnail을 즉시 못 주더라도, RAW 저장 직후 같은 촬영의 fallback fast preview 생성을 바로 시도하도록 앞당겼다.
- 따라서 이전처럼 나중 pending 구간까지 밀리지 않고, 실장비에서 fast preview first-visible 성공률을 더 끌어올릴 수 있게 됐다.
- fast preview raster를 소스로 쓰는 preset-applied preview는 첫 노출용 크기를 더 작게 낮춰, replacement 체감 시간을 추가로 줄이도록 조정했다.

이번 회차의 목표는
`조금 빨라짐`이 아니라
`고객이 바로 느끼는 수준까지 추가 단축`이다.

## 2026-04-03 사용자 후속 메모: 더 느려졌다는 피드백 반영

최신 사용자 피드백은 명확했다.

- 이전 회차 변경 뒤 recent-session 체감은 오히려 더 느려졌다.

이번 회차에서 정리한 가장 가능성 높은 원인:

- 직전 변경에서 helper가 `RAW 저장 직후 fallback fast preview 생성`까지 동기적으로 수행하도록 앞당겨졌다.
- 이 경로는 same-capture preview hit-rate를 올리려는 의도였지만, 반대로 `file-arrived` 자체를 늦춰 RAW handoff 완료 경계를 뒤로 밀 수 있었다.
- 즉 `썸네일을 더 빨리 만들겠다`는 최적화가 `캡처 완료를 늦추는 회귀`로 보였을 가능성이 높다.

그래서 이번에는 방향을 다시 조정했다.

- 카메라 thumbnail이 즉시 있으면 그 경우만 동기 경로에서 바로 사용한다.
- RAW 기반 fallback preview 생성은 다시 pending 후속 경로로 되돌려, `file-arrived`를 먼저 닫고 바로 다음 루프에서 이어서 처리하게 했다.
- 동시에 pending fallback이 실제로 시도됐는지도 더 분명히 남기도록 helper attempt 로그를 보강했다.

이번 판단의 제품 의미:

- first-visible same-capture preview는 중요하지만, 그보다 더 우선인 경계는 `캡처가 즉시 끝난 것처럼 느껴지는가`다.
- 따라서 앞으로의 단축 작업은 `preview를 앞당긴다`와 `capture completion을 막지 않는다`를 함께 만족해야 한다.

## 2026-04-03 로그 재확인: 3.n초 복귀 원인 정리

최신 재확인에서 사용자가 말한 `다시 3.n초대로 돌아갔다`는 감각과 가장 잘 맞는 코드 경계를 찾았다.

이번에 확인된 핵심:

- host preview 완료 경로에는 speculative preview 결과를 기다리는 상한 시간이 남아 있었고, 그 값이 `3.6초`였다.
- fast preview가 늦거나 speculative render가 제때 끝나지 않으면, host는 fallback으로 바로 넘어가지 않고 이 대기 예산을 먼저 소비할 수 있었다.
- 즉 same-capture fast path를 살리려던 보호 장치가, 실제로는 `3.x초 stall`처럼 느껴지는 회귀 원인이 되었을 가능성이 매우 높다.

그래서 이번 회차에서 바로 조정한 내용:

- helper fast preview 대기 예산을 `900ms -> 240ms`로 축소했다.
- speculative preview 채택 대기 예산을 `3.6s -> 320ms`로 축소했다.
- 두 경로 모두 예산이 소진되면 바로 fallback render로 넘어가도록 유지하고, 다음 진단을 위해 `wait budget exhausted` 로그도 남기게 했다.

이번 조정의 제품 의미:

- 이제 same-capture fast path는 `아주 빨리 오면 채택`, 아니면 `즉시 fallback` 쪽으로 훨씬 공격적으로 바뀌었다.
- 목표는 fast path miss 한 번이 전체 first-visible을 다시 3초대로 끌어올리지 못하게 만드는 것이다.

## 2026-04-03 연속 촬영 멈춤 메모

최신 사용자 피드백에서 새로 드러난 문제는 속도보다 더 우선이었다.

- 연속 촬영 시 앱이 멈춘 것처럼 보이는 현상이 있었다.

이번에 보관 로그와 구현을 다시 맞춰 보며 정리한 점:

- 최근 실세션 흔적에는 `두세 장 정상 -> 이후 capture-download-timeout -> phone-required` 패턴이 반복돼 있었다.
- 이 증상은 단순 `3초대 복귀`와 별개로, 연속 촬영 중 helper가 recovery 상태로 빠지며 고객에게 앱 멈춤처럼 보이는 문제다.
- 특히 helper에는 이전 촬영의 pending fast preview 작업을 이어서 처리하는 경로가 남아 있었고, 이 작업이 카메라 SDK 객체를 붙잡은 채 다음 촬영과 겹칠 여지가 있었다.

이번 회차에서 안정성 우선으로 조정한 내용:

- 새 촬영이 시작되면 이전 촬영의 pending fast preview 작업은 즉시 버리고, 새 셔터 경로를 우선하도록 바꿨다.
- pending fast preview 후속 경로에서는 더 이상 이전 촬영의 카메라 thumbnail SDK 다운로드를 붙잡지 않도록 정리했다.
- 즉 연속 촬영 안정성을 위해 `이전 촬영 fast preview 완성도`보다 `다음 촬영 handoff 안전성`을 더 우선하도록 방향을 바꿨다.

이번 판단의 제품 의미:

- 속도 최적화는 고객이 느끼는 연속 촬영 안정성을 깨지 않는 범위에서만 유효하다.
- 앞으로 same-capture fast path는 `빠르면 채택`, 하지만 `다음 촬영을 방해할 가능성이 있으면 즉시 포기`하는 것이 맞다.

## 이번 검토에서 확인한 현재 상태

이 문서는 `2026-04-02` 현재 워킹트리 기준으로 다시 검토했다.

이번 검토에서 확인한 점:

- fast thumbnail 이벤트 브리지와 `pendingFastPreview` 기반 recent-session 반영은 이미 구현 중이며 관련 테스트가 통과한다.
- host에서 actual preview render를 시작하기 전 넣어 두었던 `120ms` 대기는 현재 워킹트리에서 이미 제거되어 있다.
- 따라서 이 문서는 `fast thumbnail first` 방향 자체는 유지하되,
  이미 반영된 변경과 아직 비어 있는 계측 갭을 구분해서 읽어야 한다.

## 한 줄 요약

지금 느린 가장 유력한 이유는
`화면이 느려서`가 아니라
`고객에게 먼저 보여 줄 진짜 같은 촬영 썸네일이 늦게 들어와서`다.

fast thumbnail이 빨리 오면 레일은 비교적 빨리 보여 줄 수 있다.
fast thumbnail이 늦거나 실패하면 결국 더 느린 actual preview render를 기다리게 된다.

단, 이 문장은 현재 코드 구조와 과거 기록을 합친 가장 강한 가설이지,
실장비 `requestId` 단위 구간 계측으로 아직 완전히 닫힌 결론은 아니다.

## 이 문서에서 구분하는 것

이 문서는 아래 둘을 분리해서 본다.

1. 확정된 사실
2. 아직 실장비 계측이 더 필요한 가설

이 구분을 지키지 않으면
느린 원인을 UI 탓으로 잘못 보거나,
반대로 카메라 쪽 병목을 놓칠 수 있다.

## 현재까지 확정된 사실

### 1. 고객이 보는 첫 썸네일은 이미 `빠른 경로`가 따로 있다

현재 구조는 아래 순서다.

1. 버튼을 누른다.
2. helper가 같은 촬영의 fast thumbnail을 구하면 host로 `capture-fast-preview-update`를 보낸다.
3. 프런트는 이 값을 `pendingFastPreview`로 받아 `현재 세션 사진` 레일에 먼저 합친다.
4. 나중에 actual preview가 준비되면 같은 자리에서 자연스럽게 교체한다.

현재 워킹트리 기준으로는
이 fast preview update가 `file-arrived`로 capture 저장이 닫히기 전에 먼저 올라올 수 있도록
host-side bridge와 테스트가 같이 들어와 있다.

근거:

- host fast-preview emit: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L88)
- helper fast-thumbnail callback: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L697)
- provider fast-preview sync: [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx#L1539)
- rail merge: [CaptureScreen.tsx](/C:/Code/Project/Boothy/src/booth-shell/screens/CaptureScreen.tsx#L26)
- round-trip test: [capture_readiness.rs](/C:/Code/Project/Boothy/src-tauri/tests/capture_readiness.rs#L1190)

### 2. 버튼 자체가 주원인은 아니다

이전에는 background readiness refresh 때문에 버튼이 깜빡이거나 클릭이 무시되는 회귀가 있었다.
이 문제는 따로 수정되었다.

즉 지금 사용자가 느끼는 `오래 기다림`의 중심은
버튼 비활성화보다는 `사진이 실제로 레일에 늦게 뜨는 문제`다.

근거:

- 관련 기록: [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md)

### 3. 프런트가 이미지를 받은 뒤의 로딩 우선순위는 이미 올려 둔 상태다

현재 이미지 컴포넌트는 최신 카드와 pending 카드에 대해
`eager`, `sync`, `high` 우선순위를 준다.

즉 `이미지가 앱까지 들어온 뒤`의 지연은
현재 구조상 최우선 의심 구간이 아니다.

근거:

- image priority: [SessionPreviewImage.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/SessionPreviewImage.tsx#L108)

### 4. host 이벤트 polling 자체는 1차 병목으로 보기 어렵다

host는 helper event file을 `10ms` 간격으로 확인한다.
이 비용은 수초 단위 지연에 비해 매우 작다.

즉 지금 병목이 있다면
이 polling보다는 그 앞 단계인 `카메라가 fast thumbnail을 줄 때까지 기다리는 시간`일 가능성이 더 크다.

근거:

- event polling loop: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L307)
- poll sleep: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L437)

### 5. actual preview render는 여전히 무겁다

기존 측정 기록상 darktable preview render는 이미 수초 단위였다.

기록:

- full-size preview render: 약 `8652ms`
- `1280x1280` low-res preview render: 약 `5973ms`
- `640x640` low-res preview render: 약 `6894ms`

의미:

- fast thumbnail이 늦거나 실패하면
  사용자는 사실상 이 무거운 경로를 기다리게 된다.
- 따라서 actual preview 최적화도 중요하지만,
  첫 체감 개선에는 `fast thumbnail first`가 더 효과가 크다.

근거:

- 기록: [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md)

### 6. 현재 워킹트리에는 이미 반영된 속도 보정이 있다

현재 host는 capture 저장 뒤 actual preview render를 바로 시작한다.
이전에 있던 host-side `120ms` 고정 대기는 현재 워킹트리에서 제거되어 있다.

의미:

- Track C에서 말하는 `render 시작 전 대기 제거`는 새 작업이 아니라
  이미 적용된 변경의 유지/회귀 방지 항목으로 보는 편이 정확하다.
- 이제 남은 fallback 비용은 `대기 삽입`보다 actual render 자체 비용과
  fast path miss 빈도 쪽에서 봐야 한다.

근거:

- render kickoff: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L138)

## 현재 구조를 아주 쉽게 설명하면

지금 booth는 이렇게 움직인다.

1. 버튼을 누른다.
2. 카메라가 사진을 넘기기 시작한다.
3. helper가 그중에서 `작은 진짜 썸네일`을 먼저 꺼내 보려고 시도한다.
4. 성공하면 그 썸네일을 레일에 먼저 보여 준다.
5. 실패하거나 늦으면 더 무거운 preview render를 기다린다.

즉 현재 문제는
`레일이 느리다`보다
`3번이 늦거나 자주 실패하면 5번으로 떨어진다`에 더 가깝다.

## 왜 다른 테더링 프로그램들은 빨라 보이는가

이 부분은 외부 문헌 인용이 아니라
현재 제품 조사 기준의 working research다.

빠른 테더링 프로그램들은 보통 아래 원칙을 쓴다고 보는 것이 합리적이다.

1. 먼저 카메라 안의 작은 미리보기 사진을 즉시 보여 준다.
2. 큰 RAW는 그 뒤에 받는다.
3. 무거운 색 보정이나 최종 preview는 더 나중에 바꿔 낀다.

중요한 점:

- 빠른 프로그램은 `완성본`이 빨라서 빠른 것이 아니라
  `먼저 보여 줄 가벼운 진짜 사진`을 빨리 꺼내기 때문에 빠르게 느껴진다.
- 우리도 이미 같은 방향으로 가고 있지만,
  실제 장비에서 fast thumbnail이 충분히 빠르고 안정적으로 오는지 아직 더 조사해야 한다.

## 현재 가장 의심되는 원인과 남겨야 할 단서

### 1순위 가설: 카메라가 같은 촬영의 fast thumbnail을 늦게 준다

helper는 `DownloadCapture` 안에서 `TryDownloadPreviewThumbnail`을 호출한다.
즉 셔터 직후 아무 때나 바로 생기는 것이 아니라,
카메라 SDK가 transfer 이벤트를 열어 준 다음에야 다운로드를 시도한다.

이 구간이 길면 앱은 할 수 있는 일이 거의 없다.

근거:

- transfer path: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L664)
- thumbnail extraction: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L843)

### 2순위 가설: fast thumbnail 성공률이 충분히 높지 않을 수 있다

현재 thumbnail 추출은 best-effort다.
실패해도 capture 성공은 계속 유지된다.

이 말은 곧,
사용자는 에러를 보지 않고도 `왜 이렇게 늦지?`를 체감할 수 있다는 뜻이다.

즉 다음 구현에서는
`fast thumbnail이 얼마나 자주 성공하는지`
`실패하면 왜 실패하는지`
를 반드시 남겨야 한다.

근거:

- helper best-effort comment: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L714)

### 3순위 가설: fast path를 놓치면 결국 actual preview render가 기다림을 먹는다

host는 capture 저장 뒤 actual preview render를 백그라운드에서 돌린다.
이 경로는 맞는 구조지만,
fast path가 늦거나 실패했을 때는 체감상 너무 느려진다.

근거:

- render kickoff: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L139)

### 남겨야 할 단서: 일부 장비/설정에서는 camera thumbnail 자체가 약할 수 있다

이 브리프는 `camera thumbnail first`를 우선 가설로 두지만,
과거 실장비 조사에는 아래 단서도 남아 있다.

- 특정 장비/RAW 조합에서는 Canon SDK thumbnail/save path가 실제로 약하거나 불안정할 수 있다.
- Windows shell thumbnail fallback도 이 머신에서는 신뢰 가능한 주경로로 증명되지 않았다.

즉 다음 구현은 `camera thumbnail을 조금 더 다듬으면 끝난다`고 가정하면 안 된다.
실장비 측정 결과 fast thumbnail 성공률이 낮거나 0에 가까우면
곧바로 `alternative same-capture source` 또는 제품 차원의 `booth-safe proxy` 분기를 검토해야 한다.

근거:

- 장비 조사 기록: [current-session-photo-troubleshooting-history.md](/C:/Code/Project/Boothy/history/current-session-photo-troubleshooting-history.md#L503)
- 대안 방향 기록: [photo-button-latency-history.md](/C:/Code/Project/Boothy/history/photo-button-latency-history.md#L268)
- 외부 제품/도구 해석 근거: [_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md#L172)

## 지금 당장은 주원인으로 보기 어려운 것

다음 항목은 현시점에서 1차 원인 후보로 보기 어렵다.

- 버튼 disabled/busy 회귀
- 최근 세션 레일 자체의 렌더링
- 이미지 loading priority 미설정
- helper event file polling 주기

이 항목들은 미세 조정 대상은 될 수 있어도
현재 사용자가 말하는 `엄청난 딜레이`를 설명하는 핵심 후보는 아니다.

## 근거가 되는 코드 경로 지도

### host

- 촬영 엔트리: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L88)
- fast preview emit: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L102)
- actual preview render 시작: [capture_commands.rs](/C:/Code/Project/Boothy/src-tauri/src/commands/capture_commands.rs#L139)

### helper

- RAW/thumbnail 다운로드 경계: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L664)
- fast thumbnail emit: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L706)
- thumbnail extraction 함수: [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs#L843)

### host-side event bridge

- helper event wait loop: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L307)
- fast preview update handling: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L344)
- file-arrived completion handling: [sidecar_client.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/sidecar_client.rs#L377)

### frontend

- fast preview state merge: [session-provider.tsx](/C:/Code/Project/Boothy/src/session-domain/state/session-provider.tsx#L1539)
- rail merge into latest list: [CaptureScreen.tsx](/C:/Code/Project/Boothy/src/booth-shell/screens/CaptureScreen.tsx#L26)
- visible event logging: [SessionPreviewImage.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/SessionPreviewImage.tsx#L121)
- current recent-session view model: [current-session-previews.ts](/C:/Code/Project/Boothy/src/session-domain/selectors/current-session-previews.ts#L4)

## 현재 의견

### 의견 1

지금 단계에서 가장 효과가 큰 작업은
`actual preview를 더 싸게 만드는 것`보다
`camera thumbnail이 더 빨리, 더 자주 뜨게 만드는 것`이다.

### 의견 2

우리는 이미 `가짜 placeholder`가 아니라
`같은 촬영의 진짜 썸네일`만 보여 주는 방향을 택했다.
이 방향은 유지해야 한다.

즉 빠르게 만들더라도 아래는 금지다.

- 다른 촬영 사진 재사용
- representative tile을 같은 촬영처럼 보여 주기
- preview가 준비되지 않았는데 `Preview Ready`처럼 보이기

### 의견 3

다음 구현 에이전트는
속도 개선 전에 먼저 `실장비 계측`을 붙여야 한다.

이유:

- 지금 구조상 의심은 strong하지만
  실제로 가장 긴 구간이 `camera -> fast thumbnail`인지,
  `fast thumbnail -> UI visible`인지,
  `fast thumbnail miss -> actual render`인지
  장비 기준 수치가 아직 모자라다.

## 필요한 추가 계측

다음 8개 시점을 같은 requestId 기준으로 한 줄로 이어야 한다.

1. `button-pressed`
2. `capture-accepted`
3. `fast-thumbnail-attempted`
4. `fast-thumbnail-ready`
5. `capture-saved`
6. `recent-session-pending-visible`
7. `preview-render-ready`
8. `recent-session-visible`

현재 상태:

- 일부는 이미 있다.
- `fast-preview-ready`는 현재 provider까지 들어오고 있다.
- `recent-session-pending-visible`도 현재 찍히지만, 아직 `requestId`를 함께 싣지 않는다.
- `button-pressed`, helper의 `fast-thumbnail-attempted`, `fast-thumbnail-failed`는 아직 없다.

중요한 구현 메모:

- 지금 `CurrentSessionPreview`에는 `requestId`가 없다.
- 따라서 `recent-session-pending-visible`에 requestId join 정보를 넣으려면
  단순 로그 문자열 수정이 아니라 selector/state/component까지 requestId를 전달해야 한다.

## 구현 방향

### Track A. 먼저 계측을 닫는다

목표:

- 감이 아니라 수치로 병목을 잡는다.

해야 할 일:

- `button-pressed` 클라이언트 로그 추가
- helper에서 `fast-thumbnail-attempted`와 `fast-thumbnail-failed` 원인 로그 추가
- `recent-session-pending-visible`에 requestId join 정보 추가
- `CurrentSessionPreview` 또는 equivalent view-model에 requestId 전달 경로 추가
- 20회 이상 실장비 샘플 수집

### Track B. fast thumbnail first 경로를 더 강하게 만든다

목표:

- 같은 촬영의 첫 썸네일이 actual render보다 먼저 보이는 비율을 최대화한다.

해야 할 일:

- `TryDownloadPreviewThumbnail` 성공률 확인
- 실패 원인 분류
- fast thumbnail이 이미 있는 경우 host/UI가 절대 늦게 반영하지 않도록 유지
- 같은 canonical path 재사용 정책 유지

### Track C. fast path miss 시 fallback 비용을 줄인다

목표:

- fast thumbnail이 빠졌을 때도 체감 악화를 줄인다.

해야 할 일:

- 이미 제거된 host-side render kickoff 지연이 회귀하지 않도록 유지
- render input/output 경계 계측 보강
- 필요 시 booth rail에서는 camera thumbnail을 더 오래 유지하고,
  actual preview는 나중에 교체

## 구현 우선순위

### 1순위

`fast thumbnail이 실제 장비에서 언제 뜨는지`와
`왜 실패하는지`를 측정한다.

### 2순위

fast thumbnail이 뜬 경우
`host -> provider -> rail -> visible` 구간이 200ms 안팎으로 정리되는지 확인한다.

### 3순위

fast thumbnail miss가 확인되면
camera/SDK 설정 또는 alternative same-capture source를 검토한다.

### 4순위

그 뒤에 actual preview render 최적화를 다시 본다.

### 분기 조건

실장비 측정 결과가 아래처럼 나오면 바로 우선순위를 바꾼다.

- fast thumbnail 성공률이 충분히 높고 `4 -> 6`만 느리다: host/UI join 문제를 먼저 수정
- fast thumbnail 성공률이 낮거나 0에 가깝다: camera/SDK 조정만 고집하지 말고 same-capture 대체 source 검토
- fast thumbnail은 빠르지만 `6 -> 8`이 길다: actual render 최적화 또는 fast thumbnail 유지 시간을 더 길게 설계

## future agent를 위한 첫 구현 순서

1. `button-pressed` 로그 추가
2. helper에 `fast-thumbnail-attempted`, `fast-thumbnail-failed` 로그 추가
3. `CurrentSessionPreview`까지 requestId를 전달해 `recent-session-pending-visible`에 join 정보 추가
4. 장비 20회 측정
5. 결과를 아래 셋 중 하나로 분류

분류 기준:

- `1 -> 4`가 길다: camera/helper 병목
- `4 -> 6`이 길다: host/UI 병목
- `6 -> 8`이 길다: actual preview render 병목

## acceptance 기준

다음 구현은 아래 조건을 만족해야 한다.

- 같은 촬영의 첫 썸네일이 actual render를 기다리지 않고 먼저 보일 것
- `Preview Waiting` truth를 깨지 않을 것
- wrong-session, wrong-capture asset leak이 없을 것
- 버튼 안정성 회귀가 없을 것
- 연속 촬영에서 capture correlation 회귀가 없을 것

## 현재 검증 근거

`2026-04-02` 현재 워킹트리에서 다시 확인한 관련 검증:

- `pnpm vitest run src/booth-shell/components/SessionPreviewImage.test.tsx src/booth-shell/screens/CaptureScreen.test.tsx src/capture-adapter/services/capture-runtime.test.ts src/session-domain/state/session-provider.test.tsx`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness fast_preview`

이 테스트들이 말해 주는 것:

- helper fast preview update가 capture 저장 완료 전에 host로 전달될 수 있다
- fast thumbnail event를 프런트가 받을 수 있다
- pending fast preview가 레일에 먼저 들어간다
- invalid fast preview는 걸러진다
- canonical same-capture preview path 재사용이 유지된다
- 현재 워킹트리 기준 fast path 변경은 실행 가능한 상태다

## 마지막 결론

현재 booth의 속도 문제는
`썸네일을 그리는 화면이 느려서`라기보다
`같은 촬영의 첫 썸네일을 카메라/helper가 충분히 빠르고 안정적으로 주지 못할 때,
느린 actual preview 경로를 기다리게 되는 구조`에 가깝다.

따라서 다음 구현은
`actual preview 최적화`부터 시작하지 말고,
반드시 `fast thumbnail 도착 시간과 성공률`부터 계측하고 줄여야 한다.

다만 실장비에서 camera thumbnail 성공률이 낮게 나오면
그 다음 단계는 UI 미세조정이 아니라
`same-capture 대체 thumbnail source` 또는 `proxy/product split` 결정을 여는 쪽이어야 한다.

## 2026-04-03 기술 리서치 기록

이번 회차에서 아래 리서치 문서를 새로 작성했다.

- [_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md)

이번 리서치의 핵심 전환점:

- 문제를 `썸네일이 빨리 보이느냐`보다 `선택된 프리셋이 적용된 같은 촬영 결과가 첫 신뢰 화면으로 얼마나 빨리 보이느냐`로 다시 정의했다.
- 즉 우선순위가 `fast thumbnail first` 단독 최적화에서 `preset-applied first-visible result` 전략으로 이동했다.
- 현재 앱 셸은 유지하되, 선택된 프리셋이 적용된 첫 결과만 전용 저지연 경로로 점진 치환하는 방향이 가장 현실적이라는 결론을 정리했다.
- 현재 기술로 부족하면 다음 단계 후보는 `local dedicated renderer`, `watch-folder 기반 외부 엔진 브리지`, `edge appliance` 순으로 검토하는 것이 맞다고 정리했다.

이번 리서치가 이 브리프에 주는 제품 의미:

- 기존 브리프의 `fast thumbnail 계측` 우선순위는 여전히 유효하지만, 그것만으로 제품 목표를 달성하는지는 별개다.
- 앞으로의 판단 기준은 `같은 촬영의 첫 이미지가 보이느냐`를 넘어서 `선택된 프리셋이 적용된 결과가 first-visible인가`여야 한다.
- 따라서 다음 구현은 단순 UI join 최적화만이 아니라, `preset apply latency`, `renderer warm-up`, `cold start 제거`, `same-capture preset preview 전용 경로`를 직접 다루는 방향으로 확장되어야 한다.

다음 실행 우선순위 갱신:

1. `button-pressed -> preset-preview-visible`를 공식 KPI로 정의한다.
2. requestId 기준으로 preset-applied preview 경로 전 구간 계측을 추가한다.
3. 현재 앱 셸을 유지한 채 `preset-applied first-visible sidecar/worker` 설계에 착수한다.
4. 제한된 환경에서 새 경로를 검증하고, 목표 미달이면 더 원초적인 렌더/플랫폼 전환 옵션을 연다.

## 2026-04-03 구현 후속 메모

이번 후속 구현에서 먼저 반영한 것:

- `button-pressed`, host `preview-render-ready`, client visible 로그가 같은 `requestId`로 더 쉽게 이어지도록 계측을 보강했다.
- client visible 로그에는 이제 `previewKind=pending-fast-preview | preset-applied-preview`가 함께 남는다.
- 이 변경으로 다음 실장비 검증에서는 `선택된 프리셋이 적용된 결과가 실제로 언제 first-visible이 되었는지`를 기존보다 훨씬 직접적으로 확인할 수 있다.

이번 구현의 제품 의미:

- 다음 회차 판단은 감각이 아니라 `preset-applied first-visible latency` 수치로 더 직접적으로 할 수 있다.
- 즉 이제는 "뭔가 빨라진 것 같다"보다 `어떤 requestId가 언제 눌렸고, 언제 preset-applied preview가 실제로 보였는가`를 기준으로 보게 된다.
## 2026-04-03 Follow-up: Fourth-Capture Stall Investigation

- Field verification showed the first three captures completed normally, but the fourth felt frozen for a long time.
- Log analysis isolated the stall to `capture accepted -> RAW file arrived`, not to preset render or thumbnail promotion.
- In the affected session, the fourth capture took about `24.7s` between host acknowledgement and RAW persistence, while preset preview render stayed normal at about `3.5s` once the file arrived.
- This means the current product issue was primarily a `camera handoff wait that looked like an app freeze`, not a filtered-preview render slowdown.
- Product-side mitigation was added so the capture surface now clearly switches into an in-flight state that tells the customer the app is still working and waiting for the camera's source file transfer.

## 2026-04-03 Additional Preview Latency Cut

- Latest re-test logs still showed preset preview render around `3.4s ~ 3.6s`.
- More importantly, the render log still showed `sourceAsset=raw-original`, which meant the faster same-capture JPEG path was not actually being used yet in the live render step.
- To close that gap, the preview render path was changed to:
  `1)` briefly wait for the helper-produced same-capture fast preview,
  `2)` reuse the canonical preview JPEG as the preset render input when available,
  `3)` lower preview render size again for first-visible speed.
- The next validation target is simple:
  latest `timing-events.log` should show `sourceAsset=fast-preview-raster` for preview renders, and elapsedMs should move materially closer to the sub-2-second goal.

## 2026-04-03 History Update: Fast Preview Reuse Optimization

- User feedback after the previous cut was: `definitely improved, but still needs to be reduced more`.
- The latest live session log confirmed the product still felt slow for the same reason:
  preview render elapsed stayed around `3.4s ~ 3.6s`, and the render path was still using `sourceAsset=raw-original`.
- Based on that log, the product direction for this round was not broad tuning but one very specific change:
  make the preset-applied preview reuse the same-capture fast JPEG path for the real preview render whenever possible.

What changed in product terms:

- The host now waits a short bounded window for the helper-produced same-capture fast preview before falling back to RAW-based preview render.
- If that same-capture JPEG is available in the canonical preview slot, the preset-applied preview render uses that raster as its input instead of the RAW original.
- The first-visible preview size cap was reduced again, from `960` to `800`, to push first applied-preview latency down further.

What this means operationally:

- The next hardware validation should no longer be judged only by feel.
- The decisive proof is in the latest session's `diagnostics/timing-events.log`.
- Success for this round means preview render lines begin showing `sourceAsset=fast-preview-raster`.
- If that value still remains `raw-original`, then the fast JPEG handoff is still not being consumed by the live render path and further work should stay focused on that bridge instead of unrelated UI tuning.

Current target remains unchanged:

- `preset-applied first visible` should reach **under 2 seconds**.
- Any result still materially above that should be treated as not yet acceptable.

## 2026-04-03 연속 컷 후속 수정: 프리셋 누락 방지와 2초대 재단축

- 최신 사용자 피드백은 두 가지였다.
- 여러 장 연속 촬영에서 최초 컷과 중간 컷 일부가 `프리셋이 적용되지 않은 것처럼` 보였다.
- 동시에 체감 속도도 여전히 `3초대처럼 느껴진다`는 평가였다.

이번에 다시 정리한 원인 판단:

- recent 보관 세션 `session_000000000018a138ef5c96c18c`와 `session_000000000018a13817723d39f4`의 `session.json` 기준 단일 컷은 `capture acknowledged -> preview visible`이 대략 `2080ms ~ 2180ms` 수준이었다.
- 그런데 실제 제품에서는 same-capture fast preview가 빨리 뜬 뒤 그 결과를 너무 일찍 `완료`로 취급해, RAW 기준의 더 정확한 프리셋 결과로 다시 닫히지 않는 경우가 섞일 수 있었다.
- 즉 이번 문제는 단순 `더 빠르게` 하나가 아니라, `빠른 첫 노출`과 `RAW 기준 최종 적용 보장`이 서로 분리되지 않았던 구조 문제에 더 가까웠다.

이번 회차에서 반영한 제품 방향:

- fast preview나 speculative preview는 계속 첫 노출용으로 사용한다.
- 하지만 그 결과를 최종 종료로 보지 않고, 뒤에서 RAW 기준 preview refinement를 반드시 한 번 더 진행하도록 바꿨다.
- 그래서 고객은 먼저 같은 촬영 결과를 빠르게 보고, 그 뒤 같은 자리에서 더 정확한 프리셋 적용 결과로 자연스럽게 교체받게 된다.
- 동시에 long-running preview render가 capture 파이프라인 전체를 오래 붙잡지 않도록 줄여, 연속 촬영에서 다음 컷이 덜 밀리게 했다.

이번 변경의 성공 기준:

- 더 이상 일부 컷이 `프리셋이 안 먹은 채로 끝난 것처럼` 남지 않아야 한다.
- 최신 실장비 로그에서는 같은 request에 대해 `fast-preview-raster` first-visible 뒤에 `raw-original` refinement가 이어지는 흐름이 보여야 한다.
- 그래도 체감이 다시 `3초대`로 남으면, 다음 판단은 미세 튜닝보다 `상주형 renderer worker` 쪽으로 넘어가는 것이 맞다.

## 2026-04-03 사용자 재검증 반영: 조금 줄었지만 아직 미달

- 최신 사용자 피드백은 `조금 줄어든 것은 맞지만, 아직 허용 가능한 속도는 아니다`였다.

이번에 로컬에서 다시 확인한 로그 사실:

- 현재 남아 있는 최신 보관 capture 세션은 여전히 `2026-03-29` 런타임 세션들이었다.
- 그 보관본 기준 latest single capture는 `capture acknowledged -> preview visible = 2079ms`였다.
- 최근 5컷 기준 평균도 약 `2031.8ms`로, 이미 `2초 바로 위`까지는 내려와 있었다.
- 반면 `2026-04-03`의 [Boothy.log]에는 실제 capture 완료 라인보다 readiness polling 라인이 주로 남아 있었고, 이번 사용자 재검증을 그대로 재구성할 새 capture trace는 충분하지 않았다.

이번 로그가 주는 제품 판단:

- 남은 차이는 이제 수초 단위의 거대한 병목보다는, `fast path miss 시 추가 대기`와 `refinement 완료 사실이 UI에 전달되는 시점` 쪽에 더 가깝다.
- 즉 실제 렌더 시간이 조금 줄어도, 화면이 그 사실을 polling 주기 뒤에 알면 고객은 여전히 느리게 느낄 수 있다.

그래서 이번 회차에서 추가로 반영한 조정:

- helper fast preview 대기 예산을 더 줄여 `240ms -> 120ms`로 낮췄다.
- speculative preview 채택 대기 예산도 `320ms -> 160ms`로 더 공격적으로 줄였다.
- RAW refinement가 뒤에서 끝나면 다음 polling까지 기다리지 않고, host가 readiness update를 한 번 더 즉시 보내도록 바꿨다.
- client readiness polling도 `300ms -> 180ms`로 낮춰, 이벤트 누락 시에도 화면 반영 지연 상한을 줄였다.

이번 회차의 제품 의미:

- 이번 조정은 화질 방향을 더 희생하지 않고, 남아 있던 `수십~수백 ms` 급 대기를 먼저 걷어내는 성격이다.
- 따라서 기대 효과는 `조금 줄었다`를 `2초 아래 체감` 쪽으로 더 당기는 것이다.
- 그래도 다음 실장비에서 여전히 `3초대 체감`이 반복되면, 그때는 더 이상 wait-budget/polling 단계가 아니라 renderer topology 자체를 바꾸는 쪽이 맞다.

## 2026-04-03 로그 재검토: readiness polling 과다

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log) 재검토에서 새 세션 `session_000000000018a2c2fc6ddc04f0` 흔적은 확인됐다.
- 하지만 이 로그에는 실제 `capture_request_saved`, `capture_preview_ready`, `capture_preview_refined`보다 `capture_readiness`와 `gateway-get-readiness`가 매우 촘촘하게 반복되고 있었다.
- 즉 이번 현장 체감 저하를 설명할 직접 capture trace가 부족했고, 동시에 idle 상태에서도 readiness polling이 과도하게 host를 두드리고 있었다.

이번 판단의 제품 의미:

- 이 상태는 진단 품질을 떨어뜨린다.
- 동시에 capture가 없는 대기 구간에도 host와 client가 계속 readiness refresh를 반복해, 실제 capture 직전/직후 경로에 불필요한 잡음을 만든다.

그래서 이번 회차에서 바로 조정한 것:

- readiness polling을 항상 빠르게 유지하지 않고, 세션이 `idle + ready` 상태일 때는 더 느리게 backoff 하도록 바꿨다.
- 대신 capture 대기, preview waiting, refinement waiting 같은 active 구간에서는 기존의 빠른 폴링을 유지한다.
- 즉 고객 체감에 필요한 시점만 민감하게 보고, 평상시에는 host를 덜 흔들도록 바꾼 것이다.

이번 조정의 기대 효과:

- idle 구간의 불필요한 host 호출과 로그 소음을 줄인다.
- 실제 capture/preview 로그가 더 잘 남아 다음 실장비 진단 품질이 좋아진다.
- capture 직전의 background churn을 줄여, 작은 체감 지연도 덜어낼 수 있다.

## 2026-04-03 사용자 판단 유지: 아직 만족 못함

- 최신 사용자 판단은 여전히 동일하다.
- `조금 나아진 요소는 있어도, 아직 고객이 만족할 속도는 아니다.`

이번 판단에서 추가로 정리한 리스크:

- 우리가 correctness를 위해 붙여 둔 RAW refinement가, 연속 촬영에서는 다음 컷의 first-visible 경로와 같은 자원을 경쟁할 수 있다.
- 즉 이전 컷의 `더 정확한 결과 만들기`가 다음 컷의 `더 빨리 보여 주기`를 갉아먹을 수 있다.

그래서 이번에 더 바꾼 방향:

- RAW refinement는 이제 capture round-trip이 완전히 idle일 때까지 더 낮은 우선순위로 미룬다.
- refinement가 render queue 때문에 즉시 못 돌면 바로 포기하지 않고, 짧게 재시도하되 현재 first-visible 경로를 먼저 비켜 준다.

이번 회차의 제품 의미:

- 연속 촬영에서 중요한 건 `이전 컷의 더 예쁜 정교화`보다 `지금 방금 찍은 컷의 first-visible`이다.
- 따라서 다음 실장비 기준도 `정교화가 언젠가 된다`보다 `다음 컷 최신 사진이 즉시 보이느냐`를 더 우선으로 봐야 한다.

## 2026-04-03 로그 재확인: fast preview가 있어도 first-visible이 RAW 저장 뒤로 밀렸다

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log) 재확인에서 세션 `session_000000000018a2c41bff76fdb4`의 최근 두 컷은 여전히 매우 느렸다.
- 실제 로그에는 `helper_fast_preview_wait_budget_exhausted wait_ms=120` 뒤에 곧바로 `render_job_started ... sourceAsset=raw-original`가 찍혔다.
- 같은 컷의 `capture_preview_ready`는 각각 `elapsed_ms=6440`, `elapsed_ms=6353`으로 남아 있었다.
- 즉 고객 체감상 `Preview Waiting`에 오래 머무는 이유는 아직도 `raw-original` 경로가 first-visible을 대표해 버리는 데 있었다.

이번에 새로 정리된 구조적 원인:

- helper가 fast preview를 먼저 알려 줘도, host가 그 사실을 `persist_capture_in_dir` 이후에만 recent-session update로 올리면 고객은 그동안 계속 기다리게 된다.
- 다시 말해 `fast preview 준비`와 `화면 first-visible`이 같은 이벤트로 이어지지 못하고 있었다.

그래서 이번 회차에서 추가로 바꾼 것:

- helper fast preview를 받는 즉시 canonical preview 경로로 승격할 수 있으면, RAW 저장 완료를 기다리지 않고 recent-session에 바로 노출되도록 host 경로를 앞당겼다.
- 이후 `persist_capture_in_dir`가 같은 canonical asset을 다시 돌려줘도 중복 emit은 하지 않도록 막았다.
- 제품 기준으로는 `정답 렌더를 더 빨리 만드는 것`보다 먼저 `같은 컷의 첫 표시를 가능한 한 즉시 띄우는 것`을 우선시한 조정이다.

이번 조정의 기대 효과:

- helper thumbnail이 살아 있는 컷에서는 `Preview Waiting` 체류 시간이 크게 줄어야 한다.
- 특히 로그상 `sourceAsset=raw-original` preview render가 first-visible을 독점하던 경우를 줄일 수 있다.
- 다음 실장비에서도 helper fast preview가 자주 비거나 여전히 `2초 이하`를 못 맞추면, 그때는 구조 조정 후보를 `상주형 renderer worker`뿐 아니라 `camera thumbnail first renderer` 쪽까지 넓혀야 한다.

## 2026-04-03 사용자 판단 유지: 여전히 허용 불가, tech 문서 재검토

- 최신 사용자 판단은 더 분명해졌다.
- `조금 줄었다` 수준으로는 부족하고, 현재 속도는 여전히 제품 기준에서 허용할 수 없다.

이번 회차에서 히스토리 문서가 가리키는 tech 문서를 다시 따라가며 정리한 사실:

- [technical-capture-preview-latency-research-2026-04-01.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-capture-preview-latency-research-2026-04-01.md#L246)는 엔진 전면 교체 전에 남아 있는 darktable 경로 최적화로 `OpenCL/GPU 검증`, `--apply-custom-presets false` A/B, `render warm-up`을 명시했다.
- [technical-filtered-thumbnail-latency-research-2026-04-03.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md#L373)는 지금 제품의 1차 정답을 `앱 셸 유지 + first-visible 전용 저지연 sidecar/worker`로 정리했고, 그 다음 단계로 `local dedicated renderer`, `watch-folder bridge`, `edge appliance`를 제시했다.
- [darktable-reference-README.md](/C:/Code/Project/Boothy/darktable-reference-README.md#L95)는 runtime truth는 유지하되, `darktable-cli`를 camera helper와 별개인 render worker로 취급하고 preview/final profile을 분리하라고 못 박고 있다.

이번 재검토에서 현재 워킹트리와 실제 장비 상태를 대조해 본 결과:

- 현재 preview render는 여전히 컷마다 새 `darktable-cli` 프로세스를 직접 띄우는 구조다.
- 현재 invocation에는 tech 문서가 언급한 `--apply-custom-presets false`가 아직 들어가 있지 않다.
- `render warm-up`, `preset preload`, `cache priming`을 상시 유지하는 상주형 경로도 아직 없다.
- `OpenCL/GPU`는 booth 실장비에서 아직 검증이 닫히지 않았다.
- 실제 장비의 `darktable-cli --version`은 `5.4.0`이었고, 참조 문서 pin은 `5.4.1`이다.
- `darktable-cltest`는 이번 점검에서도 제한 시간 안에 끝나지 않아, GPU/OpenCL 가용성 자체가 아직 운영 기준으로 확정되지 않았다.

이번 tech 문서 재검토가 주는 제품 판단:

- `그냥 렌더 엔진을 지금 당장 바꿔야만 한다`까지는 아직 아니다.
- 하지만 `현 구조에서 wait budget`, `polling`, `preview 크기` 같은 미세 조정으로 더 버티는 단계는 거의 끝났다.
- 이제 남아 있는 의미 있는 시도는 `같은 엔진을 더 잘 호출하는 구조 변경` 또는 `first-visible 전용 경로 추가`다.

엔진 교체 전 아직 남아 있는 시도 방법:

1. `darktable 유지 + 상주형 first-visible renderer worker`
   - 컷마다 새 프로세스를 띄우지 말고, 세션 시작 또는 preset 선택 시 warm 상태를 유지하는 쪽이다.
   - 이건 엔진 교체가 아니라 호출 topology 교체에 가깝다.
2. `OpenCL/GPU 실장비 검증 + on/off 비교`
   - 현재는 문서상 후보일 뿐, booth에서 실제로 이득이 나는지 확인이 안 끝났다.
3. `--apply-custom-presets false` A/B
   - custom preset 로딩 비용이 preview first-visible에 실제로 영향을 주는지 darktable 경로에서 아직 닫히지 않았다.
4. `preset preload / cache priming`
   - 세션 시작 또는 preset 변경 시 preview lane을 미리 덥혀서 cold start를 고객 밖으로 밀어내는 방식이다.
5. `same-capture intermediate preview 강화`
   - [technical-filtered-thumbnail-latency-research-2026-04-03.md](/C:/Code/Project/Boothy/_bmad-output/planning-artifacts/research/technical-filtered-thumbnail-latency-research-2026-04-03.md#L158)가 말한 것처럼, LibRaw 계열 intermediate preview를 first-visible source 후보로 검토할 여지가 있다.

이번 시점의 추천 판단:

- `엔진 교체`보다 먼저 `상주형 first-visible renderer worker`를 여는 것이 맞다.
- 이건 현재 darktable truth와 preset fidelity를 유지하면서도, cold start와 per-capture spawn 비용을 직접 겨냥하는 가장 현실적인 다음 단계다.
- 그 worker 경로에서도 목표를 못 맞추면, 그때는 `local dedicated renderer` 또는 `watch-folder 기반 외부 엔진 브리지`로 넘어가는 판단이 맞다.

즉 이번 브리프의 최신 결론은 이렇다:

- `미세 튜닝`은 거의 소진됐다.
- `엔진 교체`가 유일한 남은 선택지는 아니다.
- 하지만 다음 단계는 분명히 `구조 변경`이어야 하며, 그 첫 후보는 `same engine, different topology`다.

## 2026-04-03 남은 시도 즉시 반영: preview custom preset 로딩 축소 + preset 선택 직후 warm-up

- 사용자 요청에 따라, tech 문서에서 아직 미시도 상태로 남아 있던 항목 중 지금 코드에 바로 넣을 수 있는 것부터 반영했다.

이번에 실제로 반영한 것:

- preview용 `darktable-cli` invocation에 `--apply-custom-presets false`를 추가했다.
- 즉, first-visible preview lane에서는 `data.db` custom preset 로딩 비용을 더 줄이도록 했다.
- 동시에 preset 선택 직후 `preview renderer warm-up`을 백그라운드로 예약하도록 바꿨다.
- 이 warm-up은 현재 세션/프리셋 기준으로 한 번만 잡히며, render queue가 비어 있을 때만 조용히 실행되도록 했다.
- 목적은 첫 캡처 시점의 cold start 일부를 고객 앞이 아니라 preset 선택 직후로 미리 밀어내는 것이다.

이번 조정의 제품 의미:

- 이 단계는 `엔진 교체`가 아니라 `같은 darktable 엔진을 더 낮은 지연 구조로 쓰기`에 가깝다.
- 즉 지금은 `same engine, different topology`의 첫 실장 적용이다.
- 이 조정까지 들어간 뒤에도 실장비 체감이 여전히 허용 불가라면, 다음 판단은 더 이상 invocation 미세 조정보다 `상주형 first-visible renderer worker` 또는 `local dedicated renderer` 쪽이 된다.

이번 회차 검증:

- preview invocation 단위 테스트에서 새 플래그가 들어간 것을 확인했다.
- warm-up source 준비 테스트를 추가해 background warm-up 준비 경로가 깨지지 않음을 확인했다.
- 기존 fast preview canonical path 조기 노출 테스트도 계속 통과했다.

## 2026-04-03 사용자 판단 유지: warm-up 이후에도 여전히 허용 불가

- 최신 사용자 판단은 바뀌지 않았다.
- warm-up과 preview custom preset 로딩 축소를 넣은 뒤에도, 체감은 여전히 만족할 수준이 아니다.

이번 로그 재확인에서 확인된 사실:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에는 세션 `session_000000000018a2c66b4848bd74`의 `capture_preview_ready elapsed_ms=7241`가 남아 있었다.
- 같은 컷의 client visible은 그 직후였고, 즉 이번 병목은 UI 반영이 아니라 first-visible preview 생성 자체에 남아 있었다.
- 다시 말해 이번 시점의 주병목은 `warm-up이 없어서`라기보다, `첫 표시용 preview 프로파일이 아직 too expensive`인 쪽에 더 가깝다.

그래서 엔진 변경 전 마지막 비엔진 시도 방향:

- recent-session rail의 first-visible preview는 더 공격적으로 작은 전용 프로파일로 낮춘다.
- 이 단계는 최종 화질을 위한 것이 아니라, `현재 세션 사진` 레일에 같은 촬영 결과를 가능한 한 빨리 보이게 하기 위한 전용 절충이다.
- 사용자가 이 수준도 만족하지 못하면, 다음 판단은 더 이상 preview 크기/옵션 미세 조정이 아니라 구조 전환 쪽으로 가야 한다.

## 2026-04-03 사용자 판단 유지: 체감상 여전히 동일, 실행 방식 정리 시도

- 최신 사용자 피드백은 `체감상 여전히 똑같다`였다.

이번 로그 재확인에서 분명했던 점:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c6d30012ff84`의 `capture_preview_ready elapsed_ms=7042`가 남아 있었다.
- 같은 컷의 `recent-session-visible`은 바로 뒤였고 `uiLagMs=19`였다.
- 즉 이번에도 병목은 UI 반영이 아니라 preview render 실행 자체였다.

그래서 이번 회차에서 추가로 손댄 것:

- `darktable-cli` 실행 시 stdout/stderr를 파이프로 오래 붙잡지 않고, stderr는 파일로 흘리도록 바꿨다.
- 목적은 CLI 로그 출력이 파이프 버퍼를 채워 render 자체를 막는 가능성을 없애는 것이다.
- 이건 렌더 엔진 변경이 아니라 실행 방식 안정화에 가깝고, preview/final 품질 진실값도 바꾸지 않는다.

이번 조정의 제품 의미:

- 이번 시도는 `실제로 렌더가 느린 것`과 `실행 방식 때문에 더 느려지는 것`을 분리하기 위한 성격도 있다.
- 이 조정 뒤에도 실장비 체감이 여전히 그대로면, 다음 판단은 더 이상 invocation/출력 처리 미세 조정이 아니라 구조 전환 쪽임이 더 분명해진다.

## 2026-04-03 사용자 판단 유지: 여전히 느림, preview lane에서 OpenCL 초기화 제거 시도

- 최신 사용자 피드백은 그대로였다.
- `조금 나아진 듯해도 허용 가능한 속도는 아니다`, 그리고 체감상으로는 여전히 느리다는 판단이 유지됐다.

이번 로그 재확인에서 새로 확인한 사실:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c7a958978450`의 두 컷이 각각 `capture_preview_ready elapsed_ms=6941`, `6844`로 남아 있었다.
- 두 컷 모두 `recent-session-visible`은 바로 뒤였고 `uiLagMs`는 `21`, `20` 수준이었다.
- 즉 이번에도 제품 체감 병목은 UI 반영이 아니라 `preset-applied-preview 생성` 그 자체였다.
- 같은 로그에서 두 번째 컷은 `helper_fast_preview_wait_budget_exhausted wait_ms=120` 뒤에 `pending-fast-preview`로 들어갔고, 결국 first-visible이 `6.8s`대까지 밀렸다.
- 이건 `fast preview availability`보다도, 실제 preview render lane이 여전히 너무 무겁다는 쪽을 다시 확인해 준다.

그래서 이번 회차에서 추가로 시도한 것:

- preview 전용 `darktable-cli` invocation에 `--disable-opencl`를 추가했다.
- final render truth는 그대로 두고, first-visible preview lane에서만 OpenCL 초기화/디바이스 준비 비용을 제거하는 실험이다.
- 목적은 booth 장비에서 작은 preview 한 장을 뽑기 위해 GPU/OpenCL 런타임을 깨우는 비용이, 실제 이득보다 더 큰 상황을 먼저 배제하는 것이다.
- 즉 이 단계는 `엔진 변경`이 아니라 `preview lane을 더 단순한 CPU 경로로 고정`하는 비엔진 최적화다.

이번 판단의 의미:

- 이제 남은 비엔진 시도는 정말 얇아졌다.
- `preview 크기 축소`, `custom preset 로딩 축소`, `warm-up`, `실행 방식 정리`, `OpenCL 초기화 제거`까지 넣고도 체감이 크게 안 줄면, 그다음은 옵션 미세 조정보다 구조 변경이 맞다.
- 다만 사용자와 합의한 대로, 실제 `엔진/renderer topology` 변경 전에는 먼저 브리핑하고 결정한다.

## 2026-04-03 사용자 판단 유지: 여전히 느림, helper fast preview 적중률 보강

- 최신 사용자 판단은 동일했다.
- `여전히 느리다`, 그리고 이번엔 아예 다음 단계로 계속 진행해 달라는 요청이 있었다.

이번 로그 재확인에서 확인된 사실:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c88ee9325658`의 두 컷이 각각 `capture_preview_ready elapsed_ms=7635`, `7158`로 남아 있었다.
- 첫 컷의 `recent-session-visible`은 그 직후였고 `uiLagMs=187`이었다.
- 둘째 컷은 `helper_fast_preview_wait_budget_exhausted wait_ms=120` 이후 `pending-fast-preview`로 들어갔지만, 결국 `preset-applied-preview`가 `7.1s`대에 닫혔다.
- 즉 이번에도 UI보다 `first-visible source selection`과 `preview render lane`이 더 큰 문제였고, 특히 helper fast preview를 너무 빨리 포기하고 있다는 신호가 분명했다.

그래서 이번 회차에서 추가로 시도한 것:

- host의 helper fast preview 대기 예산을 `120ms -> 360ms`로 늘렸다.
- 목적은 booth 장비에서 helper same-capture preview가 `조금 늦게` 도착해도, 곧바로 `7초대 darktable 경로`로 내려가지 않도록 하는 것이다.
- 동시에 helper 내부에서 pending fast preview를 단일 슬롯으로 들고 있다가 다음 촬영이 시작되면 사실상 버리던 구조를 큐 형태로 바꿨다.
- 즉 immediate camera thumbnail이 실패해도, 이전 컷의 RAW 기반 fallback preview 후보를 다음 컷 때문에 폐기하지 않고 순서대로 계속 살려 두도록 했다.

이번 조정의 제품 의미:

- 이 단계도 여전히 `엔진 변경`은 아니다.
- 다만 `same-capture fast preview가 실제 first-visible에 당첨될 기회`를 늘리는 쪽의 마지막 비엔진 조정에 가깝다.
- 이 단계 후에도 체감이 그대로면, 남은 선택지는 invocation 미세 조정보다 구조 변경 쪽이 더 설득력 있어진다.

## 2026-04-03 사용자 피드백 반영: 조금 더 느려짐, 대기 예산 회귀 정리 + pending preview 즉시 재촬영 허용

- 최신 사용자 피드백은 `조금 더 느려졌다`였다.
- 실제 로그도 그 판단을 뒷받침했다.

이번 로그 재확인에서 중요했던 점:

- 최신 [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log)에서는 세션 `session_000000000018a2c90eeaced59c`의 컷이 `capture_preview_ready elapsed_ms=7156`로 남아 있었다.
- 그런데 같은 로그에서 그보다 앞서 `recent-session-pending-visible`이 먼저 찍혔다.
- 다시 말해 이번 시점엔 `same-capture pending fast preview`가 이미 고객 화면에 보였는데도, 앱 전체 상태는 끝까지 `Preview Waiting / can_capture=false`에 머물렀다.
- 즉 방금 늘린 helper 대기 예산은 체감 개선보다 오히려 blocking 시간을 늘린 회귀 성격이 더 강했다.

그래서 이번 회차에서 바로 수정한 것:

- host의 helper fast preview 대기 예산은 다시 `120ms`로 되돌렸다.
- 대신 `pending fast preview`가 실제로 보이는 순간에는, live camera gate가 건강한 조건에서 다음 촬영을 다시 허용하도록 readiness 규칙을 바꿨다.
- 핵심은 `최종 preset-applied preview가 아직 안 끝났더라도, same-capture first-visible이 이미 확보됐으면 booth 흐름은 다시 앞으로 가게 한다`는 제품 정책 전환이다.
- 프런트 쪽 surface state도 이에 맞춰 `previewWaiting`에 고정하지 않고 `captureReady`로 승격될 수 있게 맞췄다.

이번 조정의 제품 의미:

- 이 단계도 렌더 엔진 변경은 아니다.
- 하지만 지금까지의 최적화가 `render ms 절감` 위주였다면, 이번 단계는 `고객이 언제 다음 행동을 할 수 있느냐`를 직접 줄이는 제품 흐름 조정이다.
- 이 조정 후에도 여전히 체감이 만족스럽지 않다면, 남은 다음 단계는 더 분명하게 `상주형 first-visible worker` 같은 구조 변경 쪽이다.

## 2026-04-03 최신 로그 재확인: 여전히 제품 기준 미달, 이제 속도와 안정성을 함께 봐야 한다

- 최신 사용자 판단대로, 현재 recent-session 썸네일 체감은 아직 허용 가능한 속도가 아니다.

이번 회차에서 다시 확인한 최신 근거는 두 갈래였다.

1. 오늘 최신 앱 로그

- [Boothy.log](/C:/Users/KimYS/AppData/Local/com.tauri.dev/logs/Boothy.log) 최신 기록에는 `2026-04-03 17:00:33 KST` 기준 세션 `session_000000000018a2c9e8a9cef54c`의 `capture_preview_ready elapsed_ms=6949`가 남아 있었다.
- 같은 컷의 `recent-session-visible`은 바로 뒤였고 `uiLagMs=23`이었다.
- 즉 지금도 고객 체감 병목은 UI 반영이 아니라 `first-visible preview 생성` 자체에 있다.

2. 보관된 실세션 런타임 흔적

- `2026-03-29` 기준 최근 8개 보관 세션을 다시 확인한 결과, `capture-ready`로 끝난 세션은 3개뿐이었고 `phone-required`로 끝난 세션이 5개였다.
- 성공으로 닫힌 13개 컷은 `request -> RAW handoff`가 평균 약 `2306ms`였고 범위는 `1966ms ~ 2769ms`였다.
- 같은 성공 컷에서 `RAW persisted -> preview ready`는 거의 고정적으로 `123ms ~ 125ms`였다.
- 반면 실패 5건은 `capture-download-timeout`까지 평균 약 `8884ms`였고 범위는 `6268ms ~ 16723ms`였다.

이번 로그가 주는 제품 판단:

- 성공 경로만 봐도 현재 속도는 여전히 빠르지 않다.
- 더 큰 문제는 실패 경로가 아직 자주 열리고, 이때는 체감이 단순 `느림`이 아니라 `앱이 멈추거나 복구 상태로 간다` 쪽으로 악화된다는 점이다.
- 따라서 지금 단계의 핵심 문제는 하나가 아니다.
- `good path`에서는 여전히 first-visible까지가 느리고,
- `bad path`에서는 RAW handoff timeout이 recent-session 경험 전체를 무너뜨린다.

이번 회차에서 함께 드러난 계측 공백:

- 최신 `Boothy.log`에는 `6.9s`대 preview ready가 남아 있는데, 현재 보관 세션 폴더 쪽에는 같은 최신 세션의 request-level artifact가 충분히 남아 있지 않았다.
- 최근 보관 세션들에서도 `timing-events.log` 증거는 사실상 비어 있었고, 실측은 `session.json`과 `camera-helper-requests/events` 조합에 의존했다.
- 즉 지금은 `정확히 어디서 6.9초가 소비됐는가`를 오늘 경로 기준으로 한 번 더 닫아 줄 최신 seam 계측이 필요하다.

그래서 다음 단계는 이렇게 정리한다.

- 첫째, `requestId` 기준 최신 실장비 계측을 복구한다.
- 최소한 `request-capture`, `file-arrived`, `fast-preview-visible`, `preview-render-start`, `capture_preview_ready`, `recent-session-visible`가 같은 세션 폴더에 함께 남아야 한다.
- 둘째, 그 한 번의 실장비 재실행으로 이번 병목이 `RAW handoff` 중심인지, `preview lane` 중심인지 다시 단일 컷 단위로 닫는다.
- 셋째, 만약 최신 seam에서도 `capture_preview_ready`가 여전히 `5s+`면, 다음 판단은 더 이상 invocation 미세 조정이 아니라 `상주형 first-visible renderer` 또는 별도 local fast renderer 같은 구조 변경으로 넘어간다.
- 반대로 `RAW handoff` timeout이 먼저 재현되면, 다음 우선순위는 preview 미세 최적화보다 helper/SDK completion boundary 안정화다.

이번 메모의 결론:

- 현재 문제는 `조금 더 줄이면 될 수준`로 보기 어렵다.
- 제품 관점의 다음 단계는 `최신 seam 재계측 1회 -> 구조 변경 여부 결정`이다.
