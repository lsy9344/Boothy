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

## 참고 문서

- `history/recent-session-thumbnail-speed-brief.md`
- `docs/contracts/render-worker.md`
- `docs/contracts/session-manifest.md`
- `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
