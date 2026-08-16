# Story 7.6: RAW 정밀본 무중단 교체와 deadline scheduler

Status: in-progress

Type: Product path (customer-visible) + Scheduler

Epic 7 dependency: `7.1 Go` → `7.2 Go` → `7.3 route decision` → `7.4 Go` → `7.5 adoption decision` → **`7.6 Go`** → `7.7` → … → `7.10 final decision`

> ## 착수 전 반드시 읽을 것 — 상류가 남긴 사실
>
> - **HV-16은 `Technology No-Go`다 (2026-08-16).** 상주 renderer 후보는 기본 `off`이고, 고객 화면의
>   운영 경로는 그대로 **`raw-original + pinned darktable 5.4.1`**이다. 이 Story는 상주 후보를 켜지 않는다.
> - **HV-16이 남은 지연의 소유를 명시적으로 지목했다.** 실장비 종단 median 8284 ms 중 display 렌더가
>   3716 ms다. 렌더를 **0으로 만들어도 4568 ms**가 남는다. 그 4568 ms의 소유는
>   **Story 7.6(렌더 큐 대기, 게시)**와 **Story 7.8(카메라 RAW 전송, present)**이다.
> - **따라서 이 Story는 NFR-003의 warm p50 3초를 달성하지 못한다.** 그것은 이 Story의 실패가 아니다.
>   합격 조건은 (1) 무중단 교체의 정확성, (2) scheduler·취소·process tree의 정확성,
>   (3) **큐 대기 구간이 실제로 얼마였는지 정직하게 측정해 남은 지연의 소유를 7.8로 확정하는 것**이다.
>   임계값을 올리거나 느린 표본을 빼서 통과처럼 보이게 만들면 이 Story는 닫히지 않는다.
> - **세 preset 연산의 총비용은 렌더의 1.26%(3716 ms 중 47 ms)다.** "렌더러를 바꾸면 빨라진다"는
>   제안은 이 숫자 앞에서 측정 없이 걸러진다.

---

## 승인된 결정 (2026-08-16, Noah Lee)

착수 전 열려 있던 세 항목은 모두 결정됐다. **dev agent는 이 항목들을 다시 묻지 않는다.**

1. **tier 정당성 판정 기준 — 확정.**
   판정은 **detail 축(MTF50)**과 **look 축(ΔE00)**을 분리한다. 두 축은 서로 다른 질문에 답하며,
   한쪽 결과가 다른 쪽을 대신할 수 없다.
   - **detail 축이 tier의 존재 조건이다.** `--hq true`가 실제로 사는 곳이 여기다.
   - **look 축은 품질 기준이 아니라 AC 4의 전환 결함 판정이다.** 두 tier는 **같은 룩**이어야 한다.
     교체 순간 색이 바뀌면 그것은 "덜 좋은 tier"가 아니라 **보이는 전환 결함**이다.
   - Story 7.4의 시각 승인 임계값(`SSIM ≥ 0.95`, `median ΔE00 ≤ 3`)은 "정확 경로와 얼마나 같은가"를
     재는 값이므로 detail 축에 그대로 쓰지 않는다. 상세 규칙은 T6에 있다.
   - **전체 해상도 final을 기준으로 삼아 두 tier를 비교하지 않는다.** 5184×3456을 1429×953으로
     내리는 리샘플러 선택이 결과를 좌우해, 측정하려는 것이 아니라 리샘플러를 재게 된다.

2. **`Partial` 종료 — 조건부 승인.**
   detail 축이 기준에 못 미쳐 정밀본 tier가 기본 비활성으로 남더라도, 아래 **네 가지가 모두 완전할 때**
   Story 7.6을 닫을 수 있다. 하나라도 없으면 `No-Go`다.
   1. HV-17A가 온전한 `Go` (우선순위 실제 적용, 용량 제한, 취소, process tree orphan 0, P0 탈락 0)
   2. `rawRefinedDisplay` tier가 **구현·테스트·게시 가능한 상태로 완성**되어 있을 것.
      만들지 않은 것은 `Partial`을 주장할 수 없다. **측정 결과로 꺼진 것과 만들지 않은 것은 다르다**
   3. tier 정당성 원자료가 완전할 것. 추정치나 미실행 항목을 통과로 적지 않는다
   4. **FR-010의 2단계 승급 약속이 현재 승인 경로에서 성립하지 않는다는 사실을
      Story 7.10 / HV-18D의 열린 제품 항목으로 올릴 것.**
      release 판정이 존재하지 않는 progressive display를 있다고 주장하면 안 된다.
      HV-16이 남은 지연의 소유를 7.6/7.8로 지목한 것과 같은 처리다

3. **HV-17B 사람 검토 — 필요하다. 단 형식이 다르다.**
   Story 7.4/7.5의 blind review는 "이 룩이 맞는가"를 묻는 **선호 검사**다.
   HV-17B가 물어야 하는 것은 정반대다 — **"교체가 안 보였는가"**. 그래서 **탐지 검사**로 한다.
   - 관찰자 **3명 × 20 시행**. 그중 **10회는 실제 교체, 10회는 교체 없는 대조군**이며 순서는 무작위다
   - 관찰자는 교체가 일어나는지, 언제인지 모른 채 **변화를 느낀 시점을 보고**한다
   - **통과 기준: 교체 시행의 탐지율이 대조군 오탐율 + 10%p를 넘지 않을 것**
   - 대조군 없이 얻은 "아무도 못 봤다"는 결과는 통과로 적지 않는다. 무엇과 비교했는지가 없으면 숫자가 아니다
   - 5명 × 30 시행(150회) 형식은 쓰지 않는다. 탐지 검사에 그 규모는 불필요하고,
     Story 7.5가 패널 미확정으로 AC를 닫지 못한 이유가 규모였다
   - **최종 승인자는 Noah Lee다.** 관찰자 3명의 실명은 HV-17B 실행 시점에 evidence 패키지에 기록한다

---

## Story

booth 고객으로서,
**처음 뜬 프리셋 사진이 더 정밀한 사진으로 조용히 바뀌기를** 원한다.
그래야 빨리 보여 주려다 화면이 깜빡이거나 사진이 튀는 일 없이, 마지막에 보는 사진이 가장 좋다.

---

## Acceptance Criteria

1. **Given** 한 촬영에 proxy·RAW 정밀본·final·유지보수 작업이 함께 있을 때, **when** 작업이 스케줄되면, **then** 우선순위가 `P0 현재 displayFitPresetProxy → P1 현재 rawRefinedDisplay → P2 final/warm-up/history` 순서로 적용되어야 하고, 현재 촬영용 renderer 동시 실행 수가 기본값으로 제한되어야 하며, stale 작업은 병합되거나 취소되어야 하고, **취소된 작업의 자식 process tree가 살아남지 않아야** 한다.
2. **Given** 자격을 갖춘 preset proxy가 현재 표시 중일 때, **when** 같은 촬영의 RAW 정밀본 generation이 준비되면, **then** 완전히 write·decode·크기 검증되고 capture/preset이 일치하며 **더 높은 승인 tier임이 확인된 뒤에야** 교체되어야 하고, 불투명 double-buffer 교체가 준비될 때까지 **현재 이미지가 계속 보여야** 한다.
3. **Given** 촬영 겹침·preset 변경·삭제·reload·listener 유실·렌더 실패·늦게 도착한 오래된 결과가 있을 때, **when** display 상태를 화해시키면, **then** capture-bound preset에 대한 **최신 유효 generation만** display pointer를 전진시킬 수 있어야 하고, 저장된 RAW·현재 viewer 상태·정확 fallback이 그대로 진실하고 복구 가능해야 한다.
4. **Given** proxy 우선 표시부터 RAW 안정화 후 500 ms까지의 구간을 계측했을 때, **when** 전환 증거를 검토하면, **then** blank, spinner, 이전/다른 촬영, 다른 preset, crop/scale 점프, 품질 tier 하락, stale overwrite가 **전부 0**이어야 한다.
5. **Given** 구현과 자동 race/fault 테스트가 끝났을 때, **when** Story 7.6을 종료 심사하면, **then** HV-17이 burst·failure·recovery·seamless swap 증거로 `Go`를 기록할 때까지 `review`를 유지해야 하고, **scheduler/용량/취소/process-tree 증거(HV-17A)와 seamless tier 전환/frame 무결성 증거(HV-17B)를 서로 독립적으로 심사**해야 하며, **한쪽 gate가 다른 쪽 결과를 물려받을 수 없다.**
6. **Given** 현재 운영 경로에서 proxy와 RAW 정밀본이 같은 RAW·같은 XMP·같은 pinned darktable을 쓸 때, **when** 두 tier의 차이를 평가하면, **then** detail 축(MTF50)이 승인 기준을 넘고 look 축(ΔE00·clipping)이 동일 룩 기준 안에 있을 때만 `rawRefinedDisplay`를 고객 화면에 게시해야 하고, detail 축을 못 넘으면 **tier를 만들어 내지 않고** `tier-not-justified`로 기록해 기본 비활성 상태를 유지해야 하며, look 축을 벗어나면 그것은 tier 문제가 아니라 **전환 결함(`refined-look-drift`)**으로 AC 4 실패로 기록해야 한다.

---

## Scope Boundary

### In Scope

- display 계약에 **`rawRefinedDisplay` tier** 추가 (`displayFitPresetProxy` 위, order 2)
- **display-fit RAW 정밀본 렌더 진입점**: 같은 RAW original, 같은 capture-bound XMP,
  `--hq true`, proxy와 **동일한 목표 크기**, 승인된 sRGB/JPEG 출력 프로필
- **우선순위 스케줄러**: 오늘의 "가득 차면 즉시 실패" 승인 게이트를 실제 우선순위 큐로 교체
- **취소와 병합**: display lane 작업의 취소, 중복 작업 병합, Windows process tree 종료
- **deadline 기록과 순서 결정**: 촬영 시점 기준 예산과 `deadline-missed` 관측
- 무중단 교체: 기존 double-buffer·decode gate·admission guard 재사용
- tier 정당성 측정 (AC 6)과 lane 스위치 `BOOTHY_RAW_REFINED_MODE`
- 계약 문서, 자동 검증, HV-17A/HV-17B 실장비 evidence

### Explicitly Out of Scope

- **`final` tier를 관람 화면 tier로 추가하는 것** — 아래 「핵심 설계 결정 2」의 근거로 제외한다
- 상주 renderer 활성화, 새 RAW decoder, `Microsoft.RawImageExtension` 도입 → Story 7.7 dependency 판정
- HV-14의 fast source 후보 재승인 → 여전히 `Technology No-Go`
- **120fps+ 물리 monitor frame과 compositor→photon 오프셋** → HV-18B가 단독 소유 (2026-08-12 correct-course)
- 100-shot soak, cold/idle/reconnect 회차 → Story 7.8
- installer / offline inventory → Story 7.7. **새 Rust crate를 추가하지 않는다**
- rollout/rollback, 최종 release 판정 → Story 7.9~7.10
- **`RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*` 384 상수와 booth 사진 레일 경로 변경**
- Story 7.4가 `on`으로 확정한 proxy lane의 렌더 인자·크기·품질 변경
- `session.json` 스키마 확장
- Story 7.3의 `source-comparison.jsonl` 계약 변경

---

## Dev Notes

### 코드베이스 현실 — 착수 전 반드시 인지할 것

| 항목 | 현재 상태 |
| --- | --- |
| display tier | **`sample`(0), `displayFitPresetProxy`(1) 두 개.** `src/shared-contracts/schemas/viewer-display.ts:32,37`, `src-tauri/src/contracts/dto.rs`. 계약 주석이 이미 "`rawRefinedDisplay`/`final`은 7.6이 추가한다"고 적혀 있다 |
| pointer schema | **`viewer-display/v3`** (7.5가 v2→v3 승격). 읽기는 v1/v2도 받는다 |
| 게시 seam | `display::generation_publisher::publish_generation_in_dir`. **stage → probe → admission → commit → journal → notify** 순서를 이 함수가 단독으로 소유한다 |
| admission guard | `display::display_artifact::evaluate_admission`. session / epoch / tier order / 크기 / preset / capture order / request order / seq를 본다. **거부 사유가 전부 고유 코드다** |
| double-buffer | `src/viewer-surface/components/DoubleBufferedPhoto.tsx`. `img.decode()` 성공 전에는 교체하지 않는다 |
| viewer guard | `src/display-generation/services/display-guard.ts`의 `shouldAdvanceDisplay`. host와 **같은 거부 매트릭스**를 갖는다 |
| proxy lane | `display::proxy_publisher`. `BOOTHY_DISPLAY_PROXY_MODE` 기본 `on`. trigger는 `commands/display_commands.rs:629 spawn_display_proxy_publication` |
| proxy 렌더 | `render::render_display_proxy_to_path_in_dir` (`render/mod.rs:320`). `--upscale false`, `--hq false`, `--icc-type SRGB`, `--icc-intent PERCEPTUAL`, JPEG 품질은 core 설정 `plugins/imageio/format/jpeg/quality` |
| proxy worker root | `.boothy-darktable/display-proxy/` (preview/final과 분리) |
| **렌더 큐** | **`MAX_IN_FLIGHT_RENDER_JOBS = 2` (`render/mod.rs:21`). `acquire_render_queue_slot()`은 가득 차면 기다리지 않고 `render-queue-saturated`로 즉시 실패한다** — 아래 「핵심 설계 결정 3」 참조 |
| 렌더 timeout | 45초. `run_darktable_invocation`이 `child.kill()` + `child.wait()`만 호출한다. **자식의 자식은 정리하지 않는다** (`render/mod.rs:1387-1405`) |
| P2 후보 작업 | `ingest_pipeline::spawn_preview_raw_refinement_in_dir`(384px 레일 정밀화), final 렌더, `run_preview_renderer_warmup_in_dir`(`try_acquire_background_render_queue_slot` 사용) |
| 계측 | `viewer-present.jsonl`, `commands/display_commands.rs`의 `register_committed_generation` / `close_out_open_generations`. **commit된 generation당 terminal 행 정확히 하나** |
| proxy 진단 span | `proxySourceReadyAtMicros`, `proxyQueueWaitMicros`, `proxyRenderStartAtMicros`, `proxyProcessExitedAtMicros` |
| 정리 경로 | `bind_display_session`, `forget_display_request`, `close_out_open_generations(CloseOutScope::{Request,OtherSessions,Due})` |
| Rust 직접 의존성 | **4개뿐** (`serde_json`, `serde`, `log`, `tauri`, `tauri-plugin-log`). Story 7.7 offline inventory가 여기 걸려 있다 |
| 상주 renderer | `display::resident_renderer`, `BOOTHY_RESIDENT_RENDERER_MODE` 기본 `off`. **이 Story는 이 값을 바꾸지 않는다** |

### 핵심 설계 결정 1 — `rawRefinedDisplay`는 "같은 RAW를 `--hq true`로 다시 렌더한 display-fit 결과"다

HV-14 이후 proxy의 source가 이미 RAW original이다. 그래서 **"RAW에서 온다"는 것은 더 이상 tier의 차이가 아니다.**
차이는 **darktable pixelpipe의 downsampling 품질** 하나다.

| | `displayFitPresetProxy` (Story 7.4, 변경 금지) | `rawRefinedDisplay` (이 Story) |
| --- | --- | --- |
| source | RAW original (CR2) | **같은** RAW original |
| preset | capture-bound `presetId@presetVersion` XMP | **같은** XMP |
| renderer | pinned darktable 5.4.1 | **같은** pinned darktable 5.4.1 |
| 크기 | viewer photo rect × DPR, `--upscale false` | **완전히 같은 목표 크기** (아래 참조) |
| `--hq` | `false` (먼저 축소하고 처리 — 빠르고 덜 선명) | **`true`** (전체 해상도로 처리한 뒤 축소) |
| 출력 프로필 | 승인된 sRGB / perceptual / bundle JPEG 품질 | 같은 색 프로필, **final 품질 JPEG** |

**이 정의가 지켜야 할 두 가지 규칙:**

1. **크기는 proxy generation의 provenance에서 상속한다.** 렌더 시점에 viewer를 다시 읽지 않는다.
   `targetWidthPx` / `targetHeightPx` / `displayProfileId` / `devicePixelRatio`를 **commit된 proxy generation에서
   그대로 가져오고**, 게시 직전에 실측 크기가 활성 proxy와 **정확히 같은지** 검증한다.
   다르면 `refined-dimension-mismatch`로 거부한다. **이것이 AC 4의 crop/scale 점프 0을 만드는 기계적 장치다.**
   viewer 문맥이 그 사이에 바뀌었다면 정밀본을 버린다 (`viewer-context-changed`) — 화면은 proxy 그대로 남는다.
2. **AC 6이 이 tier의 존재 조건이다.** 두 결과가 측정 한계 안에서 같으면 교체는 연출일 뿐이고,
   고객에게 아무 가치가 없으면서 darktable 부하만 두 배가 된다. T6에서 측정하고, 못 넘으면
   `tier-not-justified`로 기록하고 lane을 기본 `off`로 유지한다. **없는 차이를 만들어 내지 않는다.**

### 핵심 설계 결정 2 — `final`은 관람 화면 tier가 아니다

계약 주석은 "`rawRefinedDisplay`/`final`은 7.6이 추가한다"고 적혀 있지만, **`final`은 추가하지 않는다.**

- PRD FR-010이 명시하는 승급은 **display-fit preset 이미지 → RAW 정밀본** 둘뿐이다.
- `final`은 5184×3456 전체 해상도 handoff 산출물이다. 이것을 1429×953 사진 영역에 올리면
  축소를 **브라우저가** 하게 되고, darktable의 축소보다 품질이 낮으면서 decode 비용은 훨씬 크다.
  즉 "더 높은 tier"라는 주장이 화면에서 성립하지 않는다.
- `final`을 tier로 등록하는 순간 `--upscale false` 기반 display-fit 계약과 충돌한다.

**조치:** `src/shared-contracts/schemas/viewer-display.ts`와 `docs/contracts/viewer-display.md`의
"7.6이 `final`을 추가한다"는 문구를 **위 근거와 함께 수정한다.** 조용히 빼지 않는다.

### 핵심 설계 결정 3 — 오늘의 "렌더 큐"는 큐가 아니다. 이것이 이 Story의 진짜 발견이다

`acquire_render_queue_slot()`은 슬롯이 2개 다 차 있으면 **기다리지 않고 `render-queue-saturated`로 즉시 실패한다.**
그래서:

- `proxyQueueWaitMicros`는 구조적으로 **거의 0**이다. HV-15 evidence의 낮은 큐 대기값은
  "대기가 없었다"가 아니라 **"대기라는 개념이 없다"**는 뜻이다. 이 값을 근거로
  "큐는 문제가 아니다"라고 결론 내리면 안 된다.
- 실제 위험은 대기가 아니라 **탈락**이다. 384px 레일 정밀화와 final이 두 슬롯을 잡고 있으면
  고객의 첫 화면이 렌더되지 않고 **아예 만들어지지 않는다.**

**이 Story가 만들 것:** 우선순위 큐. 작업은 `enqueue(priority, jobKey, deadline)`으로 들어가고,
worker는 항상 **가장 높은 우선순위 + 가장 이른 deadline**을 먼저 꺼낸다.

| 우선순위 | 작업 | 취소 정책 |
| --- | --- | --- |
| **P0** | 현재 촬영의 `displayFitPresetProxy` | 삭제 / 세션 교체 / viewer epoch 변경에서만 취소. **더 새 촬영이 왔다고 취소하지 않는다** — 이 사진도 고객이 실제로 찍은 사진이고 `older-capture` guard가 순서를 지킨다 |
| **P1** | 현재 촬영의 `rawRefinedDisplay` | **더 새 촬영의 generation이 pointer에 commit된 순간 취소한다.** 그 뒤에는 `older-capture`로 어차피 거부되므로 계속 돌리는 것은 순수 낭비다 |
| **P2** | final 렌더, 384px 레일 정밀화, preview warm-up, history 작업 | **display 이벤트로 취소하지 않는다.** 아래 경고 참조 |

> **경고 — final 렌더를 취소하지 말 것.** Story 3.2의 `Completed`/`Export Waiting` 진실이 final 산출물에
> 달려 있다. display lane이 급하다는 이유로 final을 취소하면 고객이 결과물을 못 받는다.
> P2는 **뒤로 밀릴 수 있을 뿐 취소되지 않는다.** 삭제와 세션 종료만 P2를 취소한다.

**용량:** 현재 촬영 lane(P0+P1)의 동시 실행 기본값은 **1**로 시작한다.
P0가 도는 동안 같은 촬영의 P1이 CPU를 나눠 가지면 첫 화면이 늦어지고, 그것이 이 Story가 줄이려는 바로 그 구간이다.
전체 동시 실행 상한은 오늘의 2를 유지한다. 두 값 모두 상수로 두고 테스트로 고정한다.

### 핵심 설계 결정 4 — "deadline"은 순서를 정할 뿐, 고객의 유일한 사진을 버리지 않는다

- 각 display lane 작업은 **trusted capture input 시각 + NFR-003 hard max(5초)**를 deadline으로 갖는다.
- deadline은 **같은 우선순위 안의 순서 결정**과 **`deadline-missed` 관측**에만 쓴다.
- **deadline이 지났다는 이유로 P0를 시작하지 않거나 중단하지 않는다.** 오늘 실측은 8초대이므로
  그렇게 하면 화면에 아무것도 뜨지 않는다. 늦어도 진실하게 띄우고 늦었다고 기록한다.
- deadline 초과 시 **같은 촬영의 P1 신규 투입만 멈춘다** (더 급한 다음 촬영을 위해).

### 핵심 설계 결정 5 — 취소는 process tree까지 간다. 새 crate 없이 한다

현재 `child.kill()`은 직계 자식만 종료한다. AC 1이 요구하는 것은 tree다.

**결정: Windows 내장 `taskkill`을 절대 경로로 호출한다.**
`%SystemRoot%\System32\taskkill.exe /T /F /PID <pid>` → 그 다음 `child.wait()`.

- **새 Rust crate를 추가하지 않는다.** Story 7.7의 offline 설치 인벤토리가 그대로 유지된다
  (Story 7.3이 LibRaw를 뺀 것과 같은 이유다).
- 절대 경로로 호출한다. `PATH`에 의존하면 다른 `taskkill.exe`가 잡힐 수 있다.
- `taskkill` 실패는 무시하지 않는다. 반환 코드를 기록하고 `child.kill()`로 폴백한 뒤
  **잔존 여부를 확인해 `cancelOrphanCount`로 남긴다.**
- 취소 뒤 **staging 산출물을 반드시 지운다.** 중간에 죽은 파일이 다음 시도의 "성공"으로 오인되면 안 된다
  (`render_display_proxy_to_path_in_dir`가 이미 시작 시 잔여물을 지우지만, 취소 경로에서도 지운다).
- Job Object(`windows-sys`) 방식은 `Cargo.toml` 직접 의존성을 늘린다. **이 Story에서는 하지 않는다.**
  `taskkill`이 증거상 불충분하면 dependency 범위를 적어 별도 승인을 받는다.

### 핵심 설계 결정 6 — HV-17은 120fps 장비를 기다리지 않는다

2026-08-12 correct-course가 **120fps+ 물리 frame과 compositor→photon 오프셋을 HV-18B로 이관**했다.
Story 7.2/HV-13B가 이 장비 부재로 회차를 잃었다. **같은 실수를 반복하지 않는다.**

- **HV-17이 증명하는 것:** scheduler span, 우선순위 실제 적용 순서, 용량 상한, 취소·병합·process tree,
  proxy→refined 전환의 **결정/상태 수준** 무결성(pointer 이력, generation journal, viewer swap 이벤트,
  decode gate, 두 레이어의 geometry 동일성), burst/race/fault 결과, 일반 속도 화면 녹화.
- **HV-17이 증명하지 않는 것:** 120fps+ 물리 frame, photon 오프셋, frame 단위 zero-defect 판정. → **HV-18B 소유.**
- **재개 조건:** HV-18B에서 표시 종단점 귀책의 전환 결함(blank, spinner, stale, wrong capture,
  crop jump, scale jump, tier downgrade)이 발견되면 **Story 7.6은 `review`로 되돌아가고 HV-17은 `No-Go`가 된다.**
  Story 7.2가 가진 것과 같은 재개 조건이다.

### 이 Story가 함께 닫아야 할 기존 미결 항목

`deferred-work.md`에 남은 proxy 경로 결함 중 **네 건이 이 Story가 손대는 바로 그 코드에 있다.**
같은 파일을 열어 놓고 다시 미루면 다음 회차에서 같은 자리를 세 번째로 보게 된다.

| 항목 | 위치 | 이 Story와의 관계 |
| --- | --- | --- |
| proxy staging 경로가 중복 작업 사이에서 충돌할 수 있다 | `display/proxy_publisher.rs:209` | **반드시 닫는다.** P0/P1이 동시에 돌면 충돌 확률이 올라간다. staging 경로에 generation 좌표를 넣는다 |
| render/read 실패 경로가 staging 파일을 정리하지 않는다 | `display/proxy_publisher.rs:322` | **반드시 닫는다.** T3의 취소 정리와 같은 규칙을 쓴다 |
| actual-present 수신이 report viewer epoch와 generation epoch를 대조하지 않는다 | `commands/display_commands.rs:1407` | **반드시 닫는다.** 한 촬영에 terminal 행이 둘이 되면 잘못된 짝짓기 위험이 커진다 |
| terminal present evidence 쓰기 실패를 성공으로 반환한다 | `commands/display_commands.rs:1470` | **반드시 닫는다.** HV-17B의 완결성 판정이 이 반환값 위에 선다 |
| 렌더 시작 시점 epoch를 게시 시점에 재사용한다 | `display/proxy_publisher.rs:188` | T4가 게시 직전 재검증을 다시 만들므로 함께 정리한다 |
| display telemetry map이 terminal/session 전환 뒤에도 metadata를 보유한다 | `commands/display_commands.rs:127` | 촬영당 generation이 2개가 되어 누수량이 두 배가 된다. 함께 정리한다 |

닫은 항목은 `deferred-work.md`에서 지운다. 못 닫으면 **이유를 적고 남긴다** — 조용히 두지 않는다.

### 아키텍처 가드레일 (위반 시 리뷰 반려)

- **두 번째 게시 경로를 만들지 않는다.** `publish_generation_in_dir` 하나만 쓴다.
  Story 7.2가 증명한 표시 종단점이 두 벌이 되는 순간 무효가 된다.
- notify는 **pointer commit 뒤에만** 한다. 확정 파일을 in-place overwrite하지 않는다.
- 계약 정의는 TS/Rust 한 쌍만 존재한다. 세 번째 정의를 만들지 않는다.
- React 컴포넌트에서 `invoke()` / `listen()`을 직접 호출하지 않는다. viewer는 `viewer-host-adapter.ts`만 거친다.
- Rust command는 얇은 진입점이고 도메인 로직은 `src-tauri/src/display/`, `src-tauri/src/render/`에 둔다.
- 파일 I/O·fsync·렌더를 하는 command는 반드시 `#[tauri::command(async)]`다. **HV-13A `No-Go`의 실제 원인이다.**
- 렌더는 `DisplayState` mutex를 잡은 채로 돌지 않는다. 잡으면 그 시간 동안 pointer 조회가 통째로 막힌다.
- snapshot이 durable recovery 경계다. live event만으로 display 상태를 소유하지 않는다.
- 이미지 bytes를 IPC로 옮기지 않는다. immutable asset path만 보낸다.
- render worker는 live catalog pointer가 아니라 **capture record의 `activePresetId + activePresetVersion`**을 쓴다.
- 중간 산출물은 session root 안에만 쓴다. NFR-004는 0 tolerance다.

### UX 가드레일

- **고객 문구를 새로 만들지 않는다.** NFR-001의 copy budget은 이미 차 있다.
  교체는 **말없이** 일어난다. "더 좋은 사진으로 바꾸는 중" 같은 안내를 추가하지 않는다.
- 관람 화면의 조작 요소·진단 표시는 **0**을 유지한다.
- 정밀본 렌더 실패는 관람 화면에 노출하지 않는다. 현재 proxy가 그대로 남고,
  운영자 화면의 wait/call guidance로만 투영한다.
- UX-DR19: 자격을 갖춘 첫 frame은 같은 session/request/capture/preset@version에 연결된
  display-fit preset 이미지다. 정밀본은 **그 뒤의 승급**이지 첫 성공의 대체가 아니다.

### 절대 하지 말 것 (disaster prevention)

1. **proxy 렌더 인자를 바꾸지 않는다.** `--hq false`, 목표 크기, JPEG 품질 전부 그대로. HV-15 `Go`가 그 위에 있다.
2. **384px 상수와 booth 사진 레일 경로를 바꾸지 않는다.**
3. **final 렌더를 display lane 때문에 취소하지 않는다.** Story 3.2의 완료 진실이 걸려 있다.
4. **측정되지 않은 tier 차이를 게시하지 않는다** (AC 6).
5. **새 Rust crate를 추가하지 않는다.** Story 7.7 offline inventory에 직접 영향한다.
6. **정밀본 크기를 렌더 시점 viewer에서 새로 읽지 않는다.** proxy provenance에서 상속한다.
7. **120fps 장비를 기다리며 HV-17을 붙잡지 않는다.** 그 증거는 HV-18B 소유다.
8. **상주 renderer(`BOOTHY_RESIDENT_RENDERER_MODE`)를 켜지 않는다.** HV-16이 `No-Go`다.

---

## Tasks / Subtasks

### T1. display 계약에 `rawRefinedDisplay` tier를 추가한다 (AC: 2, 3)

- [x] `src/shared-contracts/schemas/viewer-display.ts`
  - `displayTierSchema`에 `'rawRefinedDisplay'` 추가, `DISPLAY_TIER_ORDER`에 `rawRefinedDisplay: 2`
  - 확정 파일 경로 variant 상수 `DISPLAY_RAW_REFINED_PATH_VARIANT = 'refined'` 추가
    (`<seq>-refined.jpg`. **빈 문자열을 넘기지 않는다** — Story 7.4가 같은 함정을 이미 막았다)
  - `displayProxyProvenanceSchema`를 tier 공용으로 쓰되 `renderQuality: z.enum(['fast','high'])`를 추가한다.
    proxy는 `fast`, refined는 `high`. **generation만 보고 두 tier를 구분할 수 있어야 한다**
  - `superRefine` 불변식 확장: `tier === 'rawRefinedDisplay'` → `sampleVariant`는 `null`,
    `proxyProvenance` 필수, `renderQuality === 'high'`
  - 신규 거부 사유: `refined-dimension-mismatch`, `refined-tier-not-justified`.
    **`refined-look-drift`는 런타임 거부 사유가 아니다** — ΔE00은 게시 경로에서 계산할 수 없다.
    T6의 evidence 판정 값이므로 계약 enum에 넣지 않는다
  - **`final`을 tier로 추가하지 않는다.** 주석의 "7.6이 `final`을 추가한다"를 결정 2의 근거로 교체한다
- [x] `schemaVersion`을 `viewer-display/v4`로 올린다. **읽기는 v1/v2/v3 전부 받는다.**
      Story 7.9가 pre-upgrade session 호환과 old generation pointer 복구를 요구한다.
      정규화 규칙(v1/v2 → `renderQuality` 부재 시 `fast`)을 **테스트로 고정**한다
- [x] `src-tauri/src/contracts/dto.rs`
  - `DISPLAY_TIER_RAW_REFINED_DISPLAY` 상수, `display_tier_order`에 `Some(2)` 등록
  - `render_quality` 필드 추가, 신규 reject 상수 2개
  - **필드명·nullability·enum 문자열이 TS와 정확히 일치해야 한다.** 라운드트립 테스트로 고정
- [x] `src/display-generation/services/display-guard.ts`의 `shouldAdvanceDisplay`에 같은 tier order를 넣는다.
      **한쪽에만 넣으면 pointer와 화면이 갈라진다**
- [x] `docs/contracts/viewer-display.md` 갱신: 새 tier, `renderQuality`, v3→v4 읽기 규칙, 신규 거부 사유,
      `final`이 tier가 아닌 이유

### T2. 우선순위 스케줄러를 만든다 (AC: 1)

> **오늘의 승인 게이트는 큐가 아니다** (핵심 설계 결정 3). 이 Task가 이 Story의 절반이다.

- [x] `src-tauri/src/render/scheduler.rs` 신규 (또는 `src-tauri/src/display/deadline_scheduler.rs` —
      architecture 소스 트리는 후자를 예고했으나 대상이 render worker이므로 `render/` 아래를 권장한다.
      **어느 쪽이든 architecture 문서와 실제 경로를 일치시킨다**)
  - `JobPriority { P0CurrentProxy, P1CurrentRawRefined, P2Background }`
  - `enqueue(priority, job_key, deadline_micros)` → `SchedulerTicket`
  - worker pull 규칙: **우선순위 우선, 같은 우선순위면 이른 deadline 우선, 그다음 FIFO**
  - **병합(coalesce):** 같은 `job_key`(`session/request/capture/tier/presetId@version`)가 이미
    대기 중이면 새 요청을 병합하고 두 번 렌더하지 않는다
  - **용량:** 현재 촬영 lane(P0+P1) 동시 실행 기본 **1**, 전체 동시 실행 상한 **2**(오늘 값 유지).
    두 값을 상수로 두고 테스트로 고정한다
  - **span 기록:** `enqueuedAtMicros`, `dequeuedAtMicros`, `queueWaitMicros`, `priority`,
    `deadlineMicros`, `deadlineMissed`, `coalescedCount`, `preemptedBy`
- [x] 기존 `acquire_render_queue_slot()` 호출자를 전부 스케줄러로 옮긴다
  - `render_display_proxy_to_path_in_dir` → P0
  - 신규 정밀본 렌더 → P1
  - `render_capture_asset_*`(preview/final), `run_preview_renderer_warmup_in_dir` → P2
  - **`render-queue-saturated` 즉시 실패 경로를 P2에서는 없앤다** (기다리게 한다).
    P0는 즉시 실패가 아니라 **대기**해야 한다 — 오늘 고객 첫 화면이 통째로 사라질 수 있는 경로다
  - 기존 `try_acquire_background_render_queue_slot`의 "바쁘면 warm-up 건너뛰기" 동작은 유지한다
    (warm-up은 실패해도 제품 진실에 영향이 없다)
- [x] `proxyQueueWaitMicros`의 의미를 갱신한다. **이제 실제 대기 시간이다.**
      계약 문서에 "v3 이전 값은 승인 게이트의 mutex 시간이며 대기 시간이 아니다"를 남긴다

### T3. 취소·병합과 process tree 종료를 구현한다 (AC: 1, 3)

- [x] `CancellationToken`을 job에 싣고 렌더 loop(`run_darktable_invocation`의 100 ms polling)에서 확인한다
- [x] **취소 트리거**
  - 삭제(`delete_capture` → `forget_display_request`): 그 request의 P0/P1/P2 전부 취소
  - 세션 교체(`bind_display_session`): 이전 세션의 모든 작업 취소
  - viewer epoch 변경: 그 epoch에 묶인 display lane 작업 취소
  - **더 새 촬영의 generation commit: 이전 촬영의 P1만 취소** (P0는 유지 — 핵심 설계 결정 3)
- [x] **process tree 종료** (핵심 설계 결정 5)
  - `%SystemRoot%\System32\taskkill.exe /T /F /PID <pid>` 절대 경로 호출 → `child.wait()`
  - 실패 시 `child.kill()` 폴백, 결과를 `cancelKillOutcome`으로 기록
  - 종료 뒤 **staging 산출물 삭제**
  - `cancelRequestedAtMicros` → `cancelCompletedAtMicros` 지연과 `cancelOrphanCount`를 남긴다
- [x] 45초 timeout 경로도 같은 tree 종료를 쓴다. **두 벌의 종료 코드를 만들지 않는다**
- [x] 취소된 작업은 **RAW·preview·final·현재 화면 truth를 건드리지 않는다.**
      취소는 로그와 span에만 남고 고객 화면에는 나타나지 않는다

### T4. display-fit RAW 정밀본 렌더와 publisher를 만든다 (AC: 2, 3, 6)

- [x] `src-tauri/src/render/mod.rs`에 `render_raw_refined_display_to_path_in_dir(...)` 추가
  - `render_display_proxy_to_path_in_dir`과 **같은 진입 구조**를 쓰되 `--hq true`,
    목표 크기는 **호출자가 준 proxy와 동일한 값**, `--upscale false` 유지
  - JPEG 품질은 bundle의 final 프로필 값을 쓴다. **키 이름은 `plugins/imageio/format/jpeg/quality` 그대로**
  - worker root는 `.boothy-darktable/raw-refined/`로 **네 번째 root**를 만든다.
    display-proxy와 `configdir`/`library.db`를 공유하면 P0와 P1이 서로를 막는다
- [x] `src-tauri/src/display/raw_refined_publisher.rs` 신규.
      **`proxy_publisher.rs`를 지우거나 대체하지 않는다.** 두 publisher 모두
      `publish_generation_in_dir`을 호출한다
- [x] 게시 전 검증 순서를 **이 순서 그대로** 구현한다
  1. darktable 프로세스가 **종료**했음을 확인
  2. 산출 파일 완전 write·close·`sync_all`
  3. `display::image_probe::probe_jpeg` 구조 decode
  4. **크기 검증: 활성 proxy generation과 `sourceWidthPx`/`sourceHeightPx`가 정확히 같은가.**
     다르면 `refined-dimension-mismatch`
  5. provenance 검증: session / request / capture / presetId / presetVersion / sourceAssetHash 일치
  6. **tier 정당성 확인:** AC 6 gate가 `justified`인가. 아니면 `refined-tier-not-justified`
  7. atomic immutable commit → pointer rename → journal append
  8. **그다음에야** `viewer-display-update` emit
- [x] **트리거:** proxy generation이 **commit된 직후**. proxy가 없으면 정밀본도 없다
      (tier 하락 없이 승급만 존재해야 하고, proxy 없이 정밀본만 뜨면 첫 화면이 3배 느려진다)
- [x] lane 스위치 `BOOTHY_RAW_REFINED_MODE = off | on`. **HV-17 `Go` 전까지 기본 `off`**,
      `Go` 이후 같은 Story 안에서 기본 `on`으로 전환하고 그 기본값을 테스트로 고정한다.
      알 수 없는 값은 전부 `off`
- [x] 중간 산출물은 `<session_root>/renders/display/.staging/` 안에만 쓰고 세션 정리 경로가 함께 지운다
- [x] 「이 Story가 함께 닫아야 할 기존 미결 항목」 6건을 닫고 `deferred-work.md`에서 지운다.
      못 닫은 항목은 이유와 함께 남긴다

### T5. 계측을 확장한다 (AC: 1, 4, 5)

- [x] **`viewer-present.jsonl`과 span 계약을 그대로 재사용한다. 새 파일을 만들지 않는다**
- [x] 신규 **진단** span (어느 것도 KPI 종료점이 아니다)
  - `rawRefinedEnqueuedAtMicros`, `rawRefinedQueueWaitMicros`, `rawRefinedRenderStartAtMicros`,
    `rawRefinedProcessExitedAtMicros`, `rawRefinedCommittedAtMicros`, `rawRefinedPresentedAtMicros`
  - scheduler span (T2) 전체를 generation에 연결한다
- [x] **KPI는 그대로 `trusted input → 첫 자격 frame actual present`다.**
      정밀본 present는 **두 번째 terminal 행**이지 KPI 종료점이 아니다. 5초를 넘어도 그대로 기록한다
- [x] **계측 완결성 규칙을 상속한다.** commit된 generation당 terminal 행 정확히 하나.
      정밀본 generation에도 `present-unreported` 마감 경로가 적용되는지 확인한다.
      `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`이 v4 행과
      한 촬영당 2개 generation에서도 통과하는지 **먼저** 확인한다
- [x] `close_out_open_generations`의 유예 sweep이 proxy 뒤 정밀본을 기다리는 동안
      proxy 행을 조기 마감하지 않는지 확인한다

### T6. tier 정당성을 측정한다 (AC: 6)

> **Story 7.5의 `src/quality-metrics/parity-metrics.ts`를 재사용한다. 지표 도구를 다시 만들지 않는다.**
> CIEDE2000은 Sharma 검증표 17쌍으로 이미 단위 테스트되어 있다.
>
> **미실행 상태 (2026-08-16).** 판정 도구(`src/quality-metrics/tier-justification.ts`)와 승인된
> 임계값·규칙은 구현되고 12건의 단위 테스트로 고정됐다. **측정 자체는 실행되지 않았다** —
> slanted-edge 대상이 있는 실제 EOS 700D 촬영과 pinned darktable이 필요하고, 그것은 HV-17 회차에서만
> 나온다. 그래서 `RAW_REFINED_TIER_JUSTIFICATION`은 `NotMeasured`이고 정밀본은 게시되지 않는다.
> **미실행은 통과가 아니다.**

- [ ] 같은 CR2 표본에서 proxy(`--hq false`)와 refined(`--hq true`) 출력을 쌍으로 만든다.
      **두 출력의 픽셀 크기가 같아야 한다** — 다르면 지표가 아니라 리샘플러를 재는 것이다
- [ ] **corpus 보완이 선행 조건이다.** Story 7.5에서 MTF50이 미실행으로 남은 이유는 HV-14 corpus 35장이
      전부 세로 인물/책상 장면이라 **slanted-edge 대상이 없었기** 때문이다.
      HV-17 corpus는 **해상도 차트 또는 명확한 slanted edge가 있는 실제 EOS 700D 촬영 최소 3장**을
      포함하고, 세 승인 preset 전부에 대해 측정한다 (3 × 3 = 최소 9쌍).
      자연 사진에서 slanted edge를 자동 탐지하지 않는다 — 엉뚱한 영역을 재고 그 숫자가 evidence에 남는다
- [ ] **detail 축 (tier의 존재 조건)** — `mtf50FromSlantedEdge` 사용
  - **통과: `median MTF50(refined) ≥ 1.10 × median MTF50(proxy)`**
  - **그리고 어떤 표본에서도 `MTF50(refined) < MTF50(proxy)`가 아닐 것** (역행 0건)
  - 표본별 값과 산포를 전부 남긴다. median만 적지 않는다
  - 근거: 10%는 slanted-edge MTF50의 통상 측정 노이즈보다 확실히 크고,
    **촬영마다 darktable 실행이 하나 더 늘어나는 비용을 정당화할 수 있는 최소선**이다.
    5% 개선을 위해 renderer 부하를 두 배로 만드는 것은 제품 결정으로 성립하지 않는다
  - 못 넘으면 → `tier-not-justified`. lane 기본 `off` 유지, 판정과 원자료를 evidence에 기록
- [ ] **look 축 (AC 4의 전환 결함 판정)** — `parity-metrics.ts`의 ΔE00·clipping 사용
  - **통과: `median ΔE00(refined vs proxy) ≤ 3`, `p95 ≤ 8`, clipping 증가 `≤ 2%p`**
  - 두 tier는 **같은 룩**이어야 한다. 교체 순간 색이 바뀌면 고객은 전환을 본다
  - 벗어나면 → **`refined-look-drift`. `tier-not-justified`가 아니라 AC 4 실패로 기록하고 원인을 찾는다.**
    같은 XMP·같은 renderer로 색이 달라졌다면 `--hq` 외의 무언가가 바뀐 것이다
- [x] **Story 7.4의 저노출 제품 예외를 사용하지 않는다.** 노출이 정상인 표본으로 측정한다.
      저노출 표본에서는 ΔE00이 디코더 차이가 아니라 노출 차이를 재게 된다 (Story 7.5가 같은 함정을 겪었다)
- [x] 실패 표본을 제외하지 않는다. 측정 불가 항목은 통과가 아니라 **미실행**으로 적는다

### T7. 자동 검증을 추가한다 (AC: 1~4, 6)

- [x] `render/scheduler.rs` 단위 테스트
  - P0가 대기 중인 P2보다 먼저 실행된다
  - 같은 우선순위에서 이른 deadline이 먼저 나간다
  - 같은 `job_key` 중복 요청이 병합되고 렌더가 한 번만 돈다
  - 현재 촬영 lane 동시 실행이 1을 넘지 않는다
  - deadline 초과가 P0를 취소하지 않는다 (핵심 설계 결정 4)
  - **P2 final 작업이 display 이벤트로 취소되지 않는다**
- [x] 취소 테스트 — 취소 후 staging 산출물이 남지 않고, 취소가 RAW/preview/final을 건드리지 않는다.
      process tree 종료는 **자식을 낳는 테스트용 프로세스**로 검증한다 (darktable 없이도 도는 결정적 테스트)
- [x] `display/display_artifact.rs` 거부 매트릭스 확장 — `rawRefinedDisplay` 기준으로 **전부** 단언한다:
      `refined-dimension-mismatch`, `refined-tier-not-justified`, `tier-downgrade`(refined → proxy 역행),
      `preset-mismatch`, `older-capture`, `stale-epoch`, `lower-generation`, `older-request`, `session-mismatch`
- [x] `src-tauri/tests/viewer_display.rs` 확장 (**기존 35건을 깨지 않는다**)
  - 정밀본이 proxy를 대체하지만 **역행은 거부**된다
  - 부분 write된 정밀본이 활성 truth가 되지 못한다
  - proxy와 크기가 다른 정밀본이 거부된다
  - proxy가 commit되지 않은 상태에서 정밀본만 게시되지 않는다
  - preset 변경 후 이전 preset의 늦은 정밀본이 pointer를 전진시키지 못한다
  - `BOOTHY_RAW_REFINED_MODE=off`에서 정밀본 산출물이 하나도 생기지 않는다
    (**외부 환경 변수에 따라 건너뛰지 않는 결정적 테스트**로 만든다 — 7.3의 리뷰 지적사항이다)
  - 삭제 후 standby 복귀와 다음 촬영 복구에서 두 tier 모두 정리된다
  - **v1/v2/v3 `pointer.json`을 읽어 v4로 정규화한다**
- [x] `src/display-generation/services/display-guard.test.ts` — host와 동일한 거부 매트릭스
- [x] `src/viewer-surface/components/DoubleBufferedPhoto.test.tsx` — 정밀본 교체에서도
      두 레이어의 box·`object-fit`이 동일하고 decode 성공 전에는 교체되지 않는다
- [x] `src/shared-contracts/display.contracts.test.ts` — TS↔Rust 라운드트립, tier별 교차 필드 불변식,
      v1/v2/v3 읽기
- [x] Scope guard를 **grep으로 실제 확인한다** (체크박스로 때우지 않는다):
      `RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*` 상수 diff 없음.
      proxy 렌더 인자(`--hq false`, 목표 크기, JPEG 품질) diff 없음.
      `BOOTHY_RESIDENT_RENDERER_MODE` 기본값 diff 없음.
      `Cargo.toml` `[dependencies]` diff 없음. `session-manifest.ts` diff 없음
- [x] 검증 명령과 **있는 그대로의 결과**를 completion note에 기록한다:
      `cargo fmt --check`, `cargo test --lib`, `cargo test --test viewer_display`,
      `pnpm lint`, `pnpm test:run`, `pnpm build`
  - **기존 사실 (이 Story가 만든 것이 아니다):** `pnpm build`(`tsc -b`)는 착수 시점 **20건** 실패다
    (ReadinessScreen, capture-runtime.test, governance, operator diagnostics, active-preset,
    after-paint.test, DoubleBufferedPhoto.test, use-display-pointer.test). 전체 Vitest는
    governance 문서 검사 **2건** 실패다. `capture_readiness` Rust 통합 테스트 **15건**은
    preview persistence 단계에서 실패하며 `--test-threads=1`에서 통과한다.
    **이 상태를 악화시키지 않았음을 그 방법으로 확인하고 정직하게 적는다.**
    `DoubleBufferedPhoto.test`와 `use-display-pointer.test`는 이 Story가 건드리는 파일이므로
    **그 두 건은 고칠 수 있는지 확인한다**

### T8. HV-17A / HV-17B 하드웨어 증거를 생산한다 (AC: 5)

> **두 gate는 서로 독립이다. 한쪽이 다른 쪽 결과를 물려받을 수 없다** (AC 5).
>
> **회차 미실행 (2026-08-16).** evidence root, gate 스크립트, 자체 검증, 템플릿, ledger 행은 전부
> 준비됐다. **실장비 회차는 돌지 않았다** — 승인 booth PC, EOS 700D, 고객 모니터, slanted-edge
> corpus, 관찰자 3명이 필요하다. Product Decision Rules의 `Blocked` 조건에 해당하므로
> 부족한 항목과 재실행 조건을 HV-17 ledger 행에 명시했고, **종료 판정을 보류한다.**

- [x] evidence root: `tests/hardware/raw-refined/hv-17/`.
      `viewer-present/`, `capture-source/`, `display-proxy/`, `resident-renderer/`와 섞지 않는다
- [ ] `environment.md`: 승인 PC, EOS 700D(펌웨어/렌즈/카드/케이블/전원), EDSDK·helper 버전,
      고객 모니터 모델/해상도/DPR/주사율, 승인 display profile, WebView2, GPU/driver, ICC, HDR,
      darktable pin `5.4.1`, 앱 버전·커밋 해시,
      `BOOTHY_DISPLAY_PROXY_MODE=on`, `BOOTHY_RAW_REFINED_MODE`, `BOOTHY_DISPLAY_SAMPLE_MODE=off`,
      `BOOTHY_SOURCE_COMPARE_MODE=off`, `BOOTHY_RESIDENT_RENDERER_MODE=off`.
      **읽을 수 없는 값은 만들어내지 않고 `unknown`으로 적는다**
- [ ] **HV-17A (scheduler / 용량 / 취소 / process tree)**
  - 우선순위가 실제로 적용된 실행 순서 원자료 (enqueue/dequeue span)
  - 큐 대기 실측 p50/p95/max와 **전체 종단 지연에서 차지하는 비율**
  - burst(연속 촬영 5회 이상) 중 P0 탈락 0건
  - 취소 회차: 삭제·세션 교체·epoch 변경·신규 촬영에서 각각 취소 지연과 **`cancelOrphanCount = 0`**
  - final 렌더가 취소되지 않고 완료됨을 보이는 회차
  - **`taskkill` 실행 결과 로그 원본**
- [ ] **HV-17B (seamless tier 전환 / frame 무결성)**
  - proxy commit → refined commit → refined present의 pointer 이력과 generation journal
  - proxy 우선 표시부터 **RAW 안정화 후 500 ms까지**의 화면 녹화(일반 속도)와 viewer swap 이벤트
  - **zero 보고:** blank / spinner / 이전·다른 촬영 / 다른 preset / crop 점프 / scale 점프 /
    tier 하락 / stale overwrite 전부 0
  - proxy와 refined의 `sourceWidthPx`/`sourceHeightPx`가 표본마다 **동일함**을 보이는 표
  - T6의 detail 축·look 축 원자료와 판정
  - **전환 탐지 검사 (승인된 결정 3):** 관찰자 3명 × 20 시행(교체 10 / 대조군 10, 무작위 순서).
    관찰자는 교체 여부와 시점을 모른 채 변화를 느낀 시점을 보고한다.
    **통과: 교체 시행 탐지율 ≤ 대조군 오탐율 + 10%p.**
    시행별 원자료, 관찰자 3명 실명, Noah Lee 서명을 evidence에 남긴다.
    **대조군 없는 결과는 통과로 적지 않는다**
  - **120fps+ 물리 frame은 이 gate가 제공하지 않는다.** HV-18B 소유임을 명시한다
- [x] `check-raw-refined-evidence.ps1` 완결성 gate를 만들고 **self-test한다**
      (정상 회차 통과 + 심어 둔 결함 전부 차단). Story 7.4/7.5의 gate 패턴을 따른다.
      **PowerShell 스크립트에 UTF-8 BOM을 붙인다** — Story 7.5가 BOM 없는 UTF-8 때문에
      한 줄이 조용히 삼켜지는 결함을 겪었다
- [x] `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`을 이 회차에도 태운다.
      **한 촬영당 generation 2개, 각각 terminal 행 1개**
- [x] `hardware-validation-ledger.md`의 Story 7.6 행을 갱신한다.
      HV-17을 **HV-17A / HV-17B로 분리 기록**하고, HV-18B 재개 조건을 명시한다
- [ ] 두 gate가 모두 `Go`일 때만 `BOOTHY_RAW_REFINED_MODE` 기본값을 `on`으로 바꾸고 status를 전환한다

### T9. 영향 문서를 갱신한다 (all AC)

- [x] `docs/contracts/viewer-display.md` — 새 tier, `renderQuality`, v3→v4, 신규 거부 사유,
      `final`이 tier가 아닌 이유, `proxyQueueWaitMicros`의 의미 변경
- [x] `docs/contracts/render-worker.md` — 우선순위 스케줄러, 네 번째 worker root, 취소/process tree,
      384 preview 경로 불변
- [x] `_bmad-output/planning-artifacts/architecture.md` — Story 7.6 implementation note.
      소스 트리의 `display/deadline_scheduler.rs` 예고를 **실제 경로와 일치시킨다**
- [ ] **조건 미도달 (2026-08-16).** 이 항목은 `Partial` 종료 시에만 수행한다. 현재는 HV-17A가
      실행되지 않아 `Partial`의 첫 조건(HV-17A 온전한 `Go`)부터 성립하지 않으므로,
      FR-010 상신을 **하지 않는다.** 미실행 상태에서 "2단계 승급이 성립하지 않는다"고 PRD에
      적으면 그것 역시 측정되지 않은 주장이 된다. HV-17 회차가 `Partial`로 닫힐 때 수행한다.
      원문: **`Partial`로 종료하는 경우에만:** `hardware-validation-ledger.md`의 Story 7.10 / HV-18D 행과
      `_bmad-output/planning-artifacts/prd.md`의 FR-010 항목에
      **"현재 승인 경로에서 2단계 승급이 성립하지 않는다"를 열린 제품 항목으로 올린다.**
      원자료 경로와 재개 조건(승인된 빠른 source 또는 승인된 상주 renderer가 생길 때)을 함께 적는다.
      **PRD 문구를 조용히 고치지 않는다** — 약속을 지우는 것이 아니라 미성립 사실을 붙이는 것이다
- [x] Story completion note, `sprint-status.yaml`, HV-17A/HV-17B ledger row

### Review Findings

- [x] [Review][Patch] 취소된 실행 작업 또는 다른 viewer 문맥의 재요청이 기존 job key에 병합되어 복구 렌더가 사라질 수 있다. [`src-tauri/src/render/scheduler.rs:398`]
- [x] [Review][Patch] process-tree 생존 확인이 직계 자식만 세고 probe 실패도 0으로 기록해 orphan evidence가 거짓 통과할 수 있다. [`src-tauri/src/render/mod.rs:1633`]
- [x] [Review][Patch] RAW 정밀본 pointer가 viewer 목표 크기 대신 proxy 실측 raster 크기를 required size로 저장한다. [`src-tauri/src/display/raw_refined_publisher.rs:449`]
- [x] [Review][Patch] RAW 정밀본이 proxy JPEG 품질로 렌더되면서 final profile ID를 기록해 실제 출력과 provenance가 어긋난다. [`src-tauri/src/display/raw_refined_publisher.rs:315`]
- [x] [Review][Patch] proxy가 deadline 뒤에 commit되어도 P1 RAW 정밀본을 새로 투입해 다음 촬영의 P0 예산을 소모한다. [`src-tauri/src/commands/display_commands.rs:981`]
- [x] [Review][Patch] RAW 정밀본이 inherited `sourceAssetHash`를 복사할 뿐 현재 RAW와 proxy provenance의 해시 일치를 검증하지 않는다. [`src-tauri/src/display/raw_refined_publisher.rs:396`]
- [x] [Review][Patch] P2 job key에 매번 nonce를 붙여 중복 final/preview 작업이 병합되지 않고 같은 staging 경로를 함께 사용할 수 있다. [`src-tauri/src/render/mod.rs:223`] — 출력 경로별 단일 owner가 동일 요청의 성공·실패를 follower와 공유하고, 다른 recipe는 commit 뒤 순차 실행하도록 수정함
- [x] [Review][Patch] 취소된 P2 final/preview 렌더가 오류로 즉시 반환되어 부분 staging 산출물을 정리하지 않는다. [`src-tauri/src/render/mod.rs:251`]
- [x] [Review][Defer] 단일 모니터 환경에서 windowed viewer fallback 없이 생성이 중단되는 누적 변경을 Story 7.1 범위에서 재검토한다. [`src-tauri/src/commands/viewer_commands.rs:329`] — deferred, pre-existing
- [x] [Review][Defer] 초기 viewer 생성도 trusted input 이후 창 이벤트로 집계될 수 있는 누적 telemetry 변경을 별도 검증한다. [`src-tauri/src/commands/viewer_commands.rs:456`] — deferred, pre-existing
- [x] [Review][Defer] reveal 성공 전에 epoch를 소비해 같은 epoch의 재시도가 막힐 수 있는 누적 viewer 변경을 별도 수정한다. [`src-tauri/src/commands/viewer_commands.rs:506`] — deferred, pre-existing

#### 2A 계약·품질 판정·display guard

- [x] [Review][Patch] look 축이 최소 9쌍을 채우지 않아도 `same-look`로 판정되어 정밀본 lane이 근거 없이 활성화될 수 있다. [`src/quality-metrics/tier-justification.ts:347`]
- [x] [Review][Patch] 미측정·크기 불일치·저노출 표본을 제외한 뒤 남은 표본만으로 통과할 수 있어 실패 표본이 승인 결과에서 사라진다. [`src/quality-metrics/tier-justification.ts:247`]
- [x] [Review][Patch] 최소 corpus가 3개 실촬영 × 3개 승인 preset 조합인지 확인하지 않아 중복 행만으로 9쌍을 채울 수 있다. [`src/quality-metrics/tier-justification.ts:214`]
- [x] [Review][Patch] 0·음수·NaN·Infinity 측정값이 측정 완료 표본으로 집계되어 품질 판정 근거를 오염시킬 수 있다. [`src/quality-metrics/tier-justification.ts:179`]
- [x] [Review][Patch] 실패 목록이 `sampleId`만 남겨 같은 사진의 어느 preset 조합이 실패했는지 증거에서 식별할 수 없다. [`src/quality-metrics/tier-justification.ts:175`]
- [x] [Review][Patch] v1/v2/v3 pointer는 정규화하지만 같은 세션의 구형 `generations.jsonl` 행은 새 필수 필드 때문에 읽지 못한다. [`src/shared-contracts/schemas/viewer-display.ts:560`]
- [x] [Review][Patch] 표시 결과와 거부 사유의 상충 조합이 계약을 통과해 디코드 실패와 순서 거부가 한 행에 동시에 기록될 수 있다. [`src/shared-contracts/schemas/viewer-display.ts:678`]
- [x] [Review][Patch] 실제 크기 0×0인 행도 `presented`로 기록할 수 있어 표시 완료 증거가 거짓 양성이 될 수 있다. [`src/shared-contracts/schemas/viewer-display.ts:700`]
- [x] [Review][Defer] 구형 generation에 안정적인 capture 순서가 없을 때 commit 시각으로 오래된 촬영을 추정하는 누적 fallback을 별도 호환성 정책으로 재검토한다. [`src/display-generation/services/display-guard.ts:145`] — deferred, pre-existing
- [x] [Review][Defer] viewer present span의 시간 선후관계를 계약에서 검증하지 않는 누적 telemetry 문제를 별도 검증한다. [`src/shared-contracts/schemas/viewer-display.ts:598`] — deferred, pre-existing

#### 2B Viewer 통합 테스트

- [x] [Review][Patch] 정밀본 decode 실패를 보고하면 pointer hook이 generation을 `null`로 내려 DoubleBufferedPhoto가 유지 중이던 정상 proxy까지 지운다. [`src/display-generation/state/use-display-pointer.ts:220`]
- [x] [Review][Patch] booth 계측 상태가 pointer revision을 비교하지 않아 늦게 도착한 구형 이벤트가 present telemetry를 다시 켜거나 끌 수 있다. [`src/display-generation/state/use-measurement-lane-state.ts:39`]
- [x] [Review][Patch] 계측 상태 테스트가 fixture lane과 present telemetry 값을 항상 같게 만들어 잘못된 필드를 읽는 회귀를 잡지 못한다. [`src/display-generation/state/use-measurement-lane-state.test.tsx:55`]
- [x] [Review][Patch] snapshot·listener 실패와 구독 공백 뒤 재수렴 경로가 자동 테스트되지 않아 AC 3 복구 계약이 검증되지 않는다. [`src/display-generation/state/use-display-pointer.test.tsx:244`]
- [x] [Review][Patch] 다음 generation이 직전 after-paint보다 먼저 도착하는 테스트가 새 generation의 최종 표시·보고까지 확인하지 않는다. [`src/viewer-surface/components/DoubleBufferedPhoto.test.tsx:325`]
- [x] [Review][Defer] clock calibration이 장시간 성립하지 않을 때 서로 다른 generation의 pending present 보고가 제한 없이 누적되는 기존 telemetry 정책을 별도 설계한다. [`src/viewer-surface/pending-present-report.ts:8`] — deferred, pre-existing

---

## Product Decision Rules

| 결과 | 조건 | 다음 단계 |
| --- | --- | --- |
| `Go` | HV-17A와 HV-17B가 **각각 독립적으로** 통과. detail 축 통과, look 축 통과, 전환 탐지율 기준 통과, 전환 결함 0, 취소 orphan 0, P0 탈락 0 | `BOOTHY_RAW_REFINED_MODE` 기본 `on`. status `done`. Story 7.7 착수 |
| `Partial` | HV-17A 온전한 `Go` **그리고** detail 축 미달로 tier가 `not-justified` **그리고** 승인된 결정 2의 네 조건이 전부 충족 | scheduler·취소·process tree를 채택하고 정밀본 lane은 기본 `off` 유지. **FR-010 2단계 승급 미성립을 Story 7.10 / HV-18D 열린 항목으로 올린 뒤** status `done` |
| `No-Go` | 전환 결함(`refined-look-drift` 포함) 발생, 취소 orphan 발생, P0 탈락 발생, 또는 `Partial` 네 조건 중 하나라도 미충족 | 원인 수정 후 재실행. 기본값 전환 금지. status `review` 유지 |
| `Blocked` | slanted-edge corpus·관찰자·실장비 등 필수 evidence 자체를 수집할 수 없음 | 부족한 항목과 재실행 조건을 HV-17에 명시하고 종료 판정 보류 |

---

## Definition of Done

- [x] `rawRefinedDisplay` tier와 `viewer-display/v4`가 v1/v2/v3 읽기 호환과 함께 추가됨
- [x] 우선순위 스케줄러가 P0/P1/P2를 실제 실행 순서로 강제하고, 현재 촬영 lane 용량이 제한됨
- [x] 취소·병합이 동작하고 **process tree orphan이 0**임을 재현 가능한 테스트로 증명함
- [x] 정밀본이 proxy와 동일 크기·동일 preset·동일 capture에서만 승급하고, 역행이 거부됨
- [ ] AC 6의 detail 축(MTF50 ≥ 1.10×, 역행 0건)과 look 축(median ΔE00 ≤ 3, p95 ≤ 8, clipping ≤ 2%p)이
      slanted-edge corpus 최소 9쌍에서 측정되었고, detail 축을 못 넘으면 게시하지 않았음
- [ ] 전환 탐지 검사(3명 × 20 시행, 대조군 포함)가 실행되고 관찰자 실명과 Noah Lee 서명이 남았음
- [x] proxy 렌더 인자·384 상수·final 렌더 완료·상주 renderer 기본값에 회귀 없음
- [ ] 계측 완결성: 실제 회차는 proxy generation 1개와 terminal 행 1개만 기록됨. refined generation 0개
- [x] HV-17A와 HV-17B가 독립 심사로 각각 기록됨
- [x] 남은 지연 중 **큐 대기 구간의 실측 비율**이 기록되고, 잔여 지연의 소유가 Story 7.8로 명시됨
- [ ] `deferred-work.md`의 proxy 경로 미결 6건이 닫혔거나, 못 닫은 이유가 기록됨

---

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 7.6 AC와 Epic 7 dependency 규칙 (line 1085)
- `_bmad-output/planning-artifacts/prd.md` — FR-010, NFR-003, NFR-004
- `_bmad-output/planning-artifacts/architecture.md` — progressive display model, 배달 순서 6번, Story 7.4 note
- `_bmad-output/implementation-artifacts/7-4-화면-적합-immutable-preset-proxy.md` — HV-15 `Go`, proxy 계약
- `_bmad-output/implementation-artifacts/7-5-상주-renderer-검증-spike.md` — HV-16 `Technology No-Go`,
  1.26% 실측, 남은 지연의 소유 지목, parity 도구
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` — HV-17 gateboard, HV-18B 상속 조항
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md` — 120fps 물리 frame 이관 근거
- `_bmad-output/implementation-artifacts/deferred-work.md` — proxy 경로의 기존 미결 항목 6건
- `docs/contracts/viewer-display.md`, `docs/contracts/render-worker.md`, `docs/contracts/preset-bundle.md`
- Microsoft `taskkill` 참조: https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/taskkill

---

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (1M context) — `claude-opus-5[1m]`

### Debug Log References

- 스케줄러 단위 테스트: `cargo test --lib render::scheduler` (14건)
- 정밀본 tier 통합 테스트: `cargo test --test viewer_display` (47건, 기존 35건 포함)
- 범위 가드: `cargo test --test story_7_6_scope_guard` (7건)
- HV-17 gate 자체 검증: `tests/hardware/raw-refined/hv-17/test-check-raw-refined-evidence.ps1` (18 케이스)
- 계측 완결성 gate v2 자체 검증: 정상 2-generation 회차 통과 + 심어 둔 결함 3종 차단

### Completion Notes List

#### 착수 시점 기준선 — 스토리 문서의 baseline이 낡아 있었다

T7이 인용한 baseline(`pnpm build` 20건 실패, Vitest governance 2건 실패,
`capture_readiness` 15건 실패)은 **착수 시점에 이미 해소된 상태였다.**
`deferred-work.md`의 「Resolved follow-up (2026-08-16)」이 그 사실을 기록하고 있다.
착수 직후 실측한 기준선은 `cargo test --lib` 175 passed / 0 failed였다.
**낡은 baseline을 인용해 "악화시키지 않았다"고 적으면 그것은 확인이 아니라 복사다.**

#### 검증 명령과 있는 그대로의 결과 (2026-08-16)

| 명령 | 결과 |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo test --lib` | **202 passed / 0 failed** (착수 시 175) |
| `cargo test --test viewer_display` | **47 passed / 0 failed** (착수 시 35) |
| `cargo test --test story_7_6_scope_guard` | **7 passed / 0 failed** (신규) |
| `cargo test --test capture_readiness` | **60 passed / 0 failed** (병렬), 60 passed (`--test-threads=1`) |
| `pnpm lint` | exit 0 |
| `pnpm test:run` | **559 passed / 1 skipped / 0 failed** |
| `pnpm build` (`tsc -b && vite build`) | **exit 0** |

`pnpm build`는 처음 실행에서 3건 실패했다. 세 테스트 파일이 `'viewer-display/v3'` 문자열을
하드코딩하고 있었기 때문이다(`use-display-pointer.test.tsx`, `use-measurement-lane-state.test.tsx`,
`ViewerSurface.test.tsx`). 문자열을 다시 하드코딩하지 않고 `viewerDisplaySchemaVersion` 상수를
쓰도록 고쳤다 — 다음 버전 승격에서 같은 자리를 세 번째로 보지 않기 위해서다.

#### 스토리 문서가 예고하지 않았지만 반드시 고쳐야 했던 결함

**`tier-downgrade`가 촬영을 넘어 적용되면 고객의 다음 사진이 화면에 영영 뜨지 않는다.**

tier가 `sample`(0)과 `displayFitPresetProxy`(1) 둘뿐이던 동안에는 두 lane이 서로 배타적이라
이 경로가 발화하지 않았다. `rawRefinedDisplay`(2)가 생기는 순간 실제 결함이 된다 —
촬영 A가 정밀본까지 올라간 뒤 촬영 B의 proxy(1)가 도착하면 `tier-downgrade`로 거부된다.

그래서 tier 비교를 **같은 촬영 안으로 한정**했다. 다른 촬영의 낮은 tier는 하락이 아니라
새 사진이며, 촬영 사이의 순서는 `older-capture` / `older-request`가 이미 판정한다.
host와 viewer guard 양쪽에 같은 규칙을 넣었고, 두 곳 모두 회귀 테스트로 고정했다
(`the_next_photo_still_starts_at_the_proxy_tier_after_the_previous_photo_was_refined`,
`lets the next photo start at the proxy tier after the previous photo was refined`).

**정밀본 present가 KPI 종료점으로 집계되면 NFR-003 수치가 조용히 거짓이 된다.**

정밀본 행도 `presented` + trusted input + 창 이벤트 0이므로, 손대지 않으면
`qualifyingLatencyMicros`가 채워져 첫 화면 지연과 승급 지연이 같은 분포에 섞인다.
같은 촬영이 두 번 세어지고, 정밀본을 게시하지 않은 회차와 비교도 불가능해진다.
정밀본 행에서 KPI latency를 제거하고, 종료점은 `rawRefinedPresentedAtMicros`에 그대로 남겼다.
`check-telemetry-completeness.ps1`을 v2로 올려 이 규칙을 기계적으로 강제한다 —
정밀본 행이 KPI를 주장하면 gate가 실패한다.

#### `deferred-work.md`의 proxy 경로 미결 6건 — 전부 닫았다

같은 파일을 열어 놓고 다시 미루면 다음 회차에서 같은 자리를 세 번째로 보게 된다.

1. **렌더 시작 시점 epoch 재사용** → `publish_rendered_*_in_dir`이 `current_viewer_epoch`를
   인자로 받는다. job에 실린 값을 쓸 수 없게 **구조로** 막았다.
2. **staging 경로 충돌** → 경로에 `<requestId>-<captureId>-<tier>` 좌표를 넣었다.
   P0와 P1이 함께 도는 지금 충돌 확률이 올라갔다.
3. **실패 경로가 staging을 정리하지 않음** → 재읽기 실패·decode 실패·크기 불일치·취소가
   전부 staging 파일을 지운다.
4. **actual-present가 epoch를 대조하지 않음** → `GenerationContext`가 `viewer_epoch`를 들고,
   불일치 보고는 `stale-epoch` 거부로 확정된다. **행을 없애지는 않는다.**
5. **terminal evidence 쓰기 실패를 성공으로 반환** → 실패를 그대로 반환한다.
   HV-17B의 완결성 판정이 이 반환값 위에 선다.
6. **telemetry map 누수** → 세션 교체 시 이전 세션 tombstone을 지우고 512개 상한을 뒀다.
   **미결 generation은 절대 버리지 않는다.**

#### `render-queue-saturated`가 사라졌다

P2가 이제 줄을 서므로 이 사유는 정상 경로에서 발생하지 않는다.
`ingest_pipeline`의 재시도 loop(1200 ms 예산)도 제거했다 — 두 곳에서 같은 대기를 구현하면
실제 대기 시간이 evidence에서 두 배로 보인다. 사유 코드 자체는 evidence 도구가 이전 회차를
읽을 수 있도록 남겼고, 새 회차에서 이 이벤트가 보이면 스케줄러를 우회한 경로가 생긴 것이다.

#### 범위 가드를 grep이 아니라 테스트로 만든 이유

스토리는 scope guard를 "grep으로 실제 확인"하라고 했다. **이 저장소에서는 그것이 성립하지 않는다** —
`HEAD`가 Story 7.1이고 7.2~7.5 작업이 아직 커밋되지 않아, `git diff HEAD`는 이 Story가
무엇을 바꿨는지 말해 주지 않는다(실제로 `render/mod.rs`의 HEAD 버전에는
`build_display_proxy_invocation`조차 없다). 그래서 **범위 밖 값 자체를 단언하는 테스트**로 바꿨다
(`src-tauri/tests/story_7_6_scope_guard.rs`). 커밋 이력과 무관하게 앞으로도 유효하다.

고정한 것: `Cargo.toml [dependencies]` 5개 그대로(새 crate 없음),
`BOOTHY_RESIDENT_RENDERER_MODE` 기본 `off`, `BOOTHY_DISPLAY_PROXY_MODE` 기본 `on`,
`BOOTHY_RAW_REFINED_MODE` 기본 `off` + 판정 `NotMeasured`, 384px 상수 4개,
동시 실행 상한 2/1, `final`이 tier가 아님.

#### 실장비 직접 실행 결과 (T6 측정, T8 회차)

2026-08-17 Canon EOS 700D와 고객 모니터에서 직접 회차를 실행했다. **부분 실행은 통과가 아니다.**
실제 증거가 없는 항목은 만들지 않았고 HV-17 ledger 행에 재실행 조건을 적었다.

| 항목 | 상태 | 필요한 것 |
| --- | --- | --- |
| detail 축(MTF50) 측정 | **Blocked / not-measured** | 실제 5장 모두 저노출이고 slanted edge가 없음. 정상 조명·차트 촬영 3장 필요 |
| look 축(ΔE00·clipping) 측정 | **Blocked / not-measured** | refined 산출물 0건. 같은 corpus의 proxy/refined 9쌍 필요 |
| HV-17A 회차 | **No-Go** | 5-shot burst·P0 무탈락·큐 span은 확보. 네 취소/process-tree 회차 필요 |
| HV-17B 회차 | **No-Go / Blocked** | proxy 5건 표시, refined 0건. 실제 swap·화면 녹화·관찰자 3명 필요 |
| 전환 탐지 검사 | **미실행** | 관찰자 3명 + Noah Lee 서명 |
| lane 기본값 `on` 전환 | **미도달** | HV-17A·HV-17B 각각 `Go` |

실행 증거: `tests/hardware/raw-refined/run-20260817-003242-hv17/`. RAW+JPEG에서 RAW handoff가
timeout 난 뒤 descriptor 승인값 RAW-only로 설정·readback 검증했고, CR2 5/5·proxy present 5/5·
P0 탈락 0건을 확보했다. 판정 도구와 승인 임계값(12건 테스트), evidence root와 두 gate 스크립트,
gate 자체 검증 18케이스(**AC 5의 독립성 — 한쪽 gate 결함이 다른 쪽 verdict를 건드리지 않음 —
을 케이스로 고정**), environment/corpus/탐지검사 템플릿, ledger의 HV-17A/HV-17B 분리 기록.

**FR-010 상신은 하지 않았다.** 그것은 `Partial` 종료 시의 조치이고, `Partial`의 첫 조건인
HV-17A 온전한 `Go`부터 성립하지 않는다. 미실행 상태에서 "2단계 승급이 성립하지 않는다"고
PRD에 적으면 그것 역시 측정되지 않은 주장이 된다.

#### 남은 지연의 소유

실장비 4개 trusted-input 표본의 종단 median은 7,141,922 μs이고 큐 대기 p50/p95/max는
9/22/22 μs였다. 큐 share는 **0.0001260165%**라서 이번 회차의 병목은 큐가 아니다.
queue 외 7,141.913 ms는 이 회차에서 RAW 전송·render·publish·present로 더 분리하지 않았으며,
Story 7.8의 capture/present 검증으로 넘긴다. 추정으로 하위 구간을 채우지 않는다.

### File List

**신규**

- `src-tauri/src/render/scheduler.rs`
- `src-tauri/src/display/raw_refined_publisher.rs`
- `src-tauri/tests/story_7_6_scope_guard.rs`
- `src/quality-metrics/tier-justification.ts`
- `src/quality-metrics/tier-justification.test.ts`
- `tests/hardware/raw-refined/hv-17/README.md`
- `tests/hardware/raw-refined/hv-17/environment.md`
- `tests/hardware/raw-refined/hv-17/check-raw-refined-evidence.ps1`
- `tests/hardware/raw-refined/hv-17/test-check-raw-refined-evidence.ps1`
- `tests/hardware/raw-refined/hv-17/tier-justification/README.md`
- `tests/hardware/raw-refined/hv-17/detection-trial/README.md`
- `tests/hardware/raw-refined/hv-17/detection-trial/trials-template.csv`

**수정**

- `src/shared-contracts/schemas/viewer-display.ts`
- `src/shared-contracts/display.contracts.test.ts`
- `src/display-generation/services/display-guard.ts`
- `src/display-generation/services/display-guard.test.ts`
- `src/display-generation/state/use-display-pointer.test.tsx`
- `src/display-generation/state/use-measurement-lane-state.test.tsx`
- `src/viewer-surface/ViewerSurface.test.tsx`
- `src/viewer-surface/components/DoubleBufferedPhoto.test.tsx`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/display/mod.rs`
- `src-tauri/src/display/display_artifact.rs`
- `src-tauri/src/display/generation_publisher.rs`
- `src-tauri/src/display/proxy_publisher.rs`
- `src-tauri/src/display/resident_renderer.rs`
- `src-tauri/src/render/mod.rs`
- `src-tauri/src/capture/ingest_pipeline.rs`
- `src-tauri/src/commands/display_commands.rs`
- `src-tauri/src/commands/viewer_commands.rs`
- `src-tauri/src/viewer/present_telemetry.rs`
- `src-tauri/tests/viewer_display.rs`
- `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`
- `docs/contracts/viewer-display.md`
- `docs/contracts/render-worker.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/7-6-raw-정밀본-무중단-교체.md`

## Change Log

- 2026-08-17: Canon EOS 700D 실장비 HV-17을 직접 실행했다. RAW+JPEG handoff timeout을
  RAW-only 설정과 readback으로 회복한 뒤 5/5 CR2·proxy present, P0 탈락 0건,
  큐 대기 9/22/22 μs(p50/p95/max)를 확인했다. 그러나 refined generation은 0건이었고
  저노출·slanted-edge 부재로 tier 측정이 불가능했다. 네 취소 회차와 3명 판별 시험도 없어
  HV-17A/HV-17B는 각각 No-Go, 제품 판정은 Blocked다. story와 sprint status는 `in-progress`,
  정밀본 lane 기본값은 `off`를 유지한다.
- 2026-08-16: P2 final/preview 중복 실행 보류 항목을 닫았다. 동일 산출물 요청은 한 번만 실행해
  성공·실패 결과를 공유하고, 서로 다른 recipe가 같은 출력 경로를 노리면 앞 commit 이후 순차 실행한다.
  자동 테스트로 중복 성공, 중복 실패, 출력 경로 충돌을 검증했다. 실장비 HV-17 증거는 여전히 미수집이므로
  story와 sprint status는 `in-progress`를 유지한다.
- 2026-08-16: 구현 완료, status `in-progress` → `review`. T1~T5·T7·T9는 전부 닫혔고 T6의 판정 도구와
  T8의 evidence 뼈대·gate·자체 검증도 완료됐다. **실장비가 필요한 측정과 회차(T6 측정, HV-17A/HV-17B)는
  실행되지 않았고 `Blocked`로 남는다** — Product Decision Rules의 `Blocked` 조건에 해당하므로
  종료 판정을 보류하고 HV-17 ledger 행에 부족한 항목과 재실행 조건을 적었다.
  정밀본 lane은 기본 `off`이고 AC 6 detail 축 판정은 `NotMeasured`이므로,
  **측정되지 않은 tier가 고객 화면에 도달하는 경로가 없다.**
  스토리 문서가 예고하지 않았던 두 결함을 함께 고쳤다: (1) `tier-downgrade`가 촬영을 넘어 적용되면
  고객의 다음 사진이 화면에 뜨지 않는 문제, (2) 정밀본 present가 KPI 종료점으로 집계되어
  NFR-003 수치를 오염시키는 문제. `deferred-work.md`의 proxy 경로 미결 6건도 전부 닫았다.
- 2026-08-16: Noah Lee가 착수 전 열려 있던 세 결정을 승인했다. (1) tier 정당성을 **detail 축(MTF50 ≥ 1.10×,
  역행 0건)과 look 축(median ΔE00 ≤ 3 / p95 ≤ 8 / clipping ≤ 2%p)으로 분리**하고, look 축 이탈은
  tier 문제가 아니라 AC 4 전환 결함(`refined-look-drift`)으로 판정한다. 전체 해상도 final을 기준으로 한
  비교는 리샘플러가 결과를 좌우하므로 쓰지 않는다. (2) **`Partial` 종료를 조건부 승인**한다 —
  HV-17A 온전한 `Go`, tier의 완성된 구현, 완전한 원자료, 그리고 **FR-010 2단계 승급 미성립을
  Story 7.10 / HV-18D 열린 항목으로 상신**하는 네 조건이 모두 충족될 때만이다. (3) HV-17B 사람 검토는
  선호 검사가 아니라 **탐지 검사**로 한다 — 3명 × 20 시행(교체 10 / 대조군 10), 통과 기준은
  교체 탐지율 ≤ 대조군 오탐율 + 10%p이며 최종 승인자는 Noah Lee다. 승인 내용을 「승인된 결정」 절과
  AC 6, T6, T8, Product Decision Rules에 반영했다.
- 2026-08-16: Story created from Epic 7.6, HV-16 `Technology No-Go` 결과, Story 7.4의 proxy 계약,
  현재 render queue 실측 구조, 2026-08-12 correct-course의 HV-18B 이관 조항. Status set to `ready-for-dev`.
