# Render Worker 계약

## 목적

이 문서는 booth runtime이 published preset bundle을 실제 preview/final 산출물로 연결할 때 지켜야 하는 `render worker` 기준선을 고정한다.

## Runtime 입력 기준

- render worker는 live catalog pointer가 아니라 capture record에 저장된
  `activePresetId + activePresetVersion`을 사용한다.
- runtime bundle loader는 최소 아래 필드를 읽어야 한다.
  - `presetId`
  - `publishedVersion`
  - `darktableVersion`
  - `xmpTemplatePath`
  - `previewProfile`
  - `finalProfile`
- `darktableVersion`은 pinned `5.4.1`과 일치해야 한다.
- `xmpTemplatePath`는 bundle root 내부의 실제 파일이어야 한다.

## Preview 규칙

- preview render는 `renders/previews/{captureId}.jpg`를 실제로 만든 뒤에만 `previewReady`를 기록한다.
- 같은 capture의 pending fast preview가 이미 canonical preview path에 있어도, render worker는 그 경로를 그대로 재사용해 later preset-applied output으로 교체해야 한다.
- fast preview가 먼저 보였더라도 render worker만이 truthful `previewReady`와 `preview.readyAtMs`를 올릴 수 있다.
- same-path 교체가 실패하더라도 runtime은 기존 canonical preview를 먼저 잃어버리는 방식으로 downgrade하면 안 된다.
- RAW copy, placeholder SVG, bundle 대표 preview tile은 `previewReady` 성공 산출물로 승격하면 안 된다.
- booth는 preview render가 닫히기 전까지 `Preview Waiting`을 유지해야 한다.

## Display-fit Preset Proxy 규칙 (Story 7.4)

관람 화면의 첫 성공 이미지를 만드는 별도 진입점이다. **`renders/previews`의 384px 경로와 무관하며
그 상수를 바꾸지 않는다.**

**RAW 원본에서 렌더하면 이것은 근사가 아니라 정확 경로다.** 엔진(pinned darktable),
recipe(capture-bound XMP), source(RAW 원본)가 preview/final과 모두 같고 출력 크기만 다르다.
그래서 게시된 preset이면 별도의 시각 승인 없이 쓸 수 있다.
JPEG fast source로 바뀌면 입력이 달라지므로 그때만 게시 시점의 시각 승인을 요구한다.
정확 경로의 JPEG 품질 기본값은 **95**이며, 이는 `darktable-cli`가 품질을 넘기지 않았을 때
만드는 값과 바이트까지 동일하다(실측 확인).

- 목표 크기는 **호출자가 준다.** 이 경로에 크기 상수를 만들지 않는다 —
  크기는 Story 7.1의 viewer photoRect × DPR에서만 나온다.
- `--upscale false`를 반드시 넣는다. upscaled frame은 PRD NFR-003의 zero 항목이다.
  결과는 `contain`으로 표시하며 한 축이 목표 경계에 닿으면 letterbox로 승인한다.
  두 축이 모두 목표보다 작아 확대가 필요한 결과만 display fit gate에서 거부한다.
- 출력 프로필은 승인된 recipe의 값으로 고정한다: `--icc-type`(예: `SRGB`), `--icc-intent`(예: `PERCEPTUAL`),
  JPEG 품질은 core 설정 `plugins/imageio/format/jpeg/quality`.
  **승인 목록에 없는 값은 오류이며 조용한 기본값으로 떨어지지 않는다** —
  잘못된 토큰을 넘기면 `darktable-cli`는 렌더 대신 도움말을 출력하고 종료 코드 0으로 끝나,
  "성공했는데 파일이 없는" 상태가 된다.
- `--hq false`, `--apply-custom-presets false`는 preview 경로와 동일하게 유지한다.
- `--core` 뒤는 전부 darktable core로 넘어가므로 **반드시 마지막**이다.
- worker root를 분리한다: `.boothy-darktable/display-proxy/`.
  preview/final과 `configdir`·`library.db`를 공유하면 동시 실행이 서로를 막는다.
- 렌더 실패는 저장된 RAW·preview·final truth를 건드리지 않는다.
- **큐 대기 시간을 기록한다.** Story 7.6이 우선순위 스케줄러를 도입한 뒤부터
  이 값은 **실제 대기 시간**이다 (아래 「우선순위 스케줄러」 참조).
- 진단 이벤트: `display-proxy-render-start`, `display-proxy-render-ready`, `display-proxy-render-failed`.

## RAW 정밀본 규칙 (Story 7.6)

관람 화면에 이미 올라간 proxy를 **더 정밀한 같은 사진으로 조용히 승급**시키는 두 번째 display lane이다.

**이 tier의 차이는 `--hq` 하나뿐이다.** HV-14 이후 proxy의 source가 이미 RAW original이므로
"RAW에서 온다"는 것은 더 이상 tier의 차이가 아니다. 차이는 darktable pixelpipe의
downsampling 품질 하나다.

| | `displayFitPresetProxy` | `rawRefinedDisplay` |
| --- | --- | --- |
| source | RAW original | **같은** RAW original |
| recipe | capture-bound XMP | **같은** XMP |
| renderer | pinned darktable 5.4.1 | **같은** pinned darktable 5.4.1 |
| 목표 크기 | viewer photoRect × DPR | **proxy generation에서 상속한 동일 값** |
| `--hq` | `false` | **`true`** |
| worker root | `.boothy-darktable/display-proxy/` | `.boothy-darktable/raw-refined/` |
| JPEG 품질 | 승인된 proxy recipe 값 | **같은 값** |

- **목표 크기를 렌더 시점 viewer에서 새로 읽지 않는다.** commit된 proxy generation의
  provenance에서 상속하고, 게시 직전에 실측 크기가 활성 proxy와 정확히 같은지 다시 검증한다.
  다르면 `refined-dimension-mismatch`로 거부한다 — 이것이 AC 4의 crop/scale 점프 0을 만드는
  기계적 장치다. 그 사이 viewer 문맥이 바뀌었다면 정밀본을 버리고 화면은 proxy 그대로 남긴다.
- **네 번째 worker root를 쓴다.** display-proxy와 `configdir`·`library.db`를 공유하면
  P0와 P1이 서로를 막아, 정밀본을 만드는 동안 고객의 첫 화면이 기다리게 된다.
- **`--hq` 외의 인자는 proxy lane과 한 글자도 다르지 않다.** 다른 인자가 갈라지면
  tier 정당성 측정이 `--hq`를 재는 것이 아니게 된다. `render/mod.rs`의
  `the_refined_lane_differs_from_the_proxy_lane_by_exactly_one_argument`가 이를 고정한다.
- lane 스위치 `BOOTHY_RAW_REFINED_MODE = off | on`. **기본 `off`이고 알 수 없는 값도 전부 `off`다.**
  HV-17A와 HV-17B가 각각 독립적으로 `Go`를 기록하기 전에는 켜지 않는다.
- **측정되지 않은 tier 차이는 게시하지 않는다.** AC 6의 detail 축(MTF50) gate가
  `justified`가 아니면 렌더까지 끝났더라도 `refined-tier-not-justified`로 거부된다.
  판정은 `RAW_REFINED_TIER_JUSTIFICATION` 상수가 들고 있으며, 현재 값은 `NotMeasured`다.
- 정밀본 실패는 관람 화면에 노출되지 않는다. 현재 proxy가 그대로 남는다.

## 우선순위 스케줄러 (Story 7.6)

**Story 7.6 이전의 "렌더 큐"는 큐가 아니었다.** 슬롯 2개가 다 차 있으면 기다리지 않고
`render-queue-saturated`로 즉시 실패했다. 그래서 두 가지 결과가 따라왔다.

1. `proxyQueueWaitMicros`가 구조적으로 거의 0이었다. HV-15 evidence의 낮은 큐 대기값은
   "대기가 없었다"가 아니라 **"대기라는 개념이 없었다"**는 뜻이다.
   **v3 이전 evidence의 이 값을 대기 시간으로 인용하면 안 된다.**
2. 실제 위험은 대기가 아니라 **탈락**이었다. 384px 레일 정밀화와 final이 두 슬롯을 잡고 있으면
   고객의 첫 화면이 늦게 만들어지는 게 아니라 **아예 만들어지지 않았다.**

| 우선순위 | 작업 | 취소 정책 |
| --- | --- | --- |
| **P0** | 현재 촬영의 `displayFitPresetProxy` | 삭제 / 세션 교체 / viewer epoch 변경에서만 취소. **더 새 촬영이 왔다고 취소하지 않는다** — 이 사진도 고객이 실제로 찍은 사진이고 `older-capture` guard가 순서를 지킨다 |
| **P1** | 현재 촬영의 `rawRefinedDisplay` | 위 세 가지 + **더 새 촬영의 generation이 pointer에 commit된 순간.** 그 뒤에는 `older-capture`로 어차피 거부되므로 계속 돌리는 것은 순수 낭비다 |
| **P2** | final 렌더, 384px 레일 정밀화, preview warm-up, history 작업 | **display 이벤트로 취소하지 않는다.** 삭제와 세션 종료만 취소한다 |

> **경고 — final 렌더를 취소하지 말 것.** Story 3.2의 `Completed` / `Export Waiting` 진실이
> final 산출물에 달려 있다. display lane이 급하다는 이유로 final을 취소하면 고객이 결과물을
> 못 받는다. **P2는 뒤로 밀릴 수 있을 뿐 취소되지 않는다.**

- worker pull 규칙: **우선순위 우선 → 같은 우선순위면 이른 deadline → 그다음 FIFO.**
  자기 lane 용량이 막힌 작업은 건너뛴다. 그러지 않으면 P0가 lane 용량을 기다리는 동안
  두 번째 슬롯이 그냥 논다.
- **용량:** 현재 촬영 lane(P0+P1) 동시 실행 **1**, 전체 동시 실행 상한 **2**(Story 7.4 값 유지).
  두 값 모두 상수이며 테스트로 고정한다. P0가 도는 동안 같은 촬영의 P1이 CPU를 나눠 가지면
  첫 화면이 늦어지고, 그것이 이 Story가 줄이려는 바로 그 구간이다.
- **병합:** 같은 `job_key`(`session/request/capture/tier/presetId@version`)가 이미 대기·실행 중이면
  새 요청을 병합하고 두 번 렌더하지 않는다. **P2는 병합하지 않는다** — 호출자가 결과 파일을
  직접 읽으므로, "이미 누가 만들고 있다"로 처리하면 파일이 없는데도 성공으로 돌아간다.
- **deadline은 순서를 정할 뿐 고객의 유일한 사진을 버리지 않는다.** 각 display lane 작업은
  trusted capture input 시각 + NFR-003 hard max(5초)를 deadline으로 갖고, 그 값은
  같은 우선순위 안의 순서 결정과 `deadline-missed` 관측에만 쓴다.
  **deadline이 지났다는 이유로 P0를 시작하지 않거나 중단하지 않는다** — 실측이 8초대인 지금
  그렇게 하면 화면에 아무것도 뜨지 않는다. 늦어도 진실하게 띄우고 늦었다고 기록한다.
- warm-up만은 줄을 서지 않는다. 렌더러가 완전히 놀고 있을 때만 슬롯을 얻고, 아니면 즉시 포기한다.
  실패해도 제품 진실에 영향이 없는 작업이 고객의 첫 화면 뒤에 darktable 실행을 하나 더 붙이면
  순손해다.
- span: `enqueuedAtMicros`, `dequeuedAtMicros`, `queueWaitMicros`, `priority`, `deadlineMicros`,
  `deadlineMissed`, `coalescedCount`, `preemptedBy`.

## 취소와 process tree 종료 (Story 7.6)

- 취소 신호는 실행 중인 렌더에 `CancellationToken`으로 실리고, 렌더 loop가 100 ms polling
  사이에 확인한다.
- **자식의 자식까지 종료한다.** `child.kill()`은 직계 자식만 죽인다.
  `%SystemRoot%\System32\taskkill.exe /T /F /PID <pid>`를 **절대 경로로** 호출한 뒤 `child.wait()`한다.
  `PATH`에 의존하면 다른 `taskkill.exe`가 잡힐 수 있고, 취소 경로에서 그것은
  "종료했다고 믿었는데 살아 있는" 상태를 만든다.
- **새 Rust crate를 추가하지 않는다.** Job Object(`windows-sys`) 방식은 `Cargo.toml`
  직접 의존성을 늘려 Story 7.7의 offline 설치 인벤토리를 바꾼다.
  `taskkill`이 증거상 불충분하면 dependency 범위를 적어 별도 승인을 받는다.
- **`taskkill` 실패를 무시하지 않는다.** 반환 코드를 기록하고 `child.kill()`로 폴백한 뒤
  잔존 자손 수를 실제로 세어 `cancelOrphanCount`로 남긴다. **세지 못한 것과 0인 것은 다르다.**
- 취소 뒤 **staging 산출물을 반드시 지운다.** 중간에 죽은 파일이 다음 시도의 "성공"으로
  오인되면 안 된다.
- **45초 timeout 경로도 같은 tree 종료를 쓴다.** 두 벌의 종료 코드를 만들지 않는다.
- 취소는 **로그와 span에만 남는다.** RAW·preview·final·현재 화면 truth를 건드리지 않으며
  고객 화면에 아무것도 나타나지 않는다.

## 384px preview 레일은 이 Story의 범위 밖이다

`RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*`는 **384로 고정이고 Story 7.6이 바꾸지 않는다.**
booth 사진 레일 경로도 그대로다. 이 레일의 정밀화 작업은 P2로 줄을 설 뿐 인자와 크기가 바뀌지 않는다.
`tests/story_7_6_scope_guard.rs`가 이를 실행되는 단언으로 고정한다.

## Final 규칙

- final render는 `renders/finals/{captureId}.jpg`를 실제로 만든 뒤에만 `finalReady`를 기록한다.
- post-end `Completed`는 `finalReady`가 없는 상태에서 올라가면 안 된다.
- post-end에서 preview만 준비된 상태는 `export-waiting`으로 유지한다.

## Drift 보호

- capture 이후 publish/rollback 또는 active preset 변경이 있어도,
  이미 저장된 capture render는 capture record에 저장된 version으로만 다시 계산한다.
- runtime은 capture-bound bundle을 찾지 못하면 조용히 최신 live version으로 대체하면 안 된다.

## 진단과 실패

- render failure는 customer surface에 darktable/XMP/filesystem 경로를 노출하지 않는다.
- diagnostics에는 safe event만 남긴다.
  - `preview-render-start`
  - `preview-render-ready`
  - `preview-render-failed`
  - `preview-render-queue-saturated`
  - `final-render-start`
  - `final-render-ready`
  - `final-render-failed`
  - `final-render-queue-saturated`
- preview/final failure는 저장된 RAW와 기존 session asset을 보존한 채 bounded failure truth로 기록한다.

> **`*-render-queue-saturated`는 Story 7.6 이후 정상 경로에서 발생하지 않는다.**
> P2 작업은 즉시 실패하는 대신 줄을 서기 때문이다. 이 사유 코드는 evidence 도구가 이전 회차를
> 읽을 수 있도록 남겨 두며, 새 회차에서 이 이벤트가 보이면 그것은 스케줄러를 우회한 경로가
> 생겼다는 뜻이다. 새 실패 사유는 `render-cancelled`(삭제·세션 교체로 취소됨)다.
