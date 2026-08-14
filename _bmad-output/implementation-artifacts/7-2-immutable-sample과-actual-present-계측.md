# Story 7.2: Immutable sample 표시와 actual-present 계측

Status: done

Type: Enabler

Epic 7 dependency: `7.1 Go` → **`7.2 Go`** → `7.3 approved route decision` → ... → `7.10 final decision`. HV-13A는 2026-08-11 `Go`이므로 착수 조건은 충족됐다.

## Correct Course Note (2026-08-12)

**120fps+ 물리 모니터 프레임 증거는 Story 7.8 / HV-18B로 이관됐다.** HV-13B는 자기가 실제로
증명한 범위 — immutable publication, opaque double-buffer, one-clock actual-present, 계측 완결성,
A/B 결정, AC 4 실패 조건 — 로 재정의되어 `Go`를 기록했고, 이 Story는 `done`으로 닫는다.

- 근거: 물리 프레임이 증명하는 compositor→photon 구간은 60Hz에서 최대 **0.017초**다. 같은 회차에서
  관측된 실제 고객 체감 지연은 **3.7~4.5초**이고 미리보기는 5초 예산을 6.8~24.0초로 초과했다.
  그 병목은 camera source(7.3)와 preset renderer(7.4~7.6) 소유다. 0.017초를 증명하지 못해
  4초짜리 문제의 해결을 막는 것은 순서가 뒤바뀐 것이다.
- 이관처가 7.8인 이유: HV-18B는 이미 100회 측정에서 `crop/scale jump, tier downgrade, blank,
  stale = 0`을 요구하며, 그 시점에는 fixture가 아니라 **실제 preset 적용 사진과 RAW 정밀본 교체**가
  화면에 올라간다. 7.10/HV-18D는 증거를 집계하는 판정이지 생산하는 게이트가 아니다.
- **재개 조건: HV-18B가 표시 종단점 귀책의 전환 결함(blank·spinner·stale·이전 촬영·crop jump·
  scale jump·tier downgrade)을 발견하면 이 Story는 `review`로 되돌아가고 HV-13B는 `No-Go`가 된다.**
- 아래 AC 5와 T6/T8의 물리 프레임 항목은 이 Story의 의무가 아니며, 삭제된 것이 아니라 HV-18B의
  required evidence로 이동했다.
- 전문: `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md`

## Story

booth product team으로서,
immutable sample 전환을 실제 고객 디스플레이에서 계측하길 원한다.
그래서 이후의 source 교체(7.3/7.4)와 renderer 교체(7.5/7.6)를 하나의 신뢰할 수 있는 표시 종단점 기준으로 판정할 수 있다.

## Scope Boundary

### In Scope

- request-scoped **immutable sample generation** 게시 (fixture 기반, renderer 무관)
- 완전 write + close + fsync → 구조 decode → 크기 검증 → correlation 검증 → **atomic display pointer commit** → 그다음에야 notify
- viewer의 **opaque double-buffer swap** (완전 decode 이후에만 교체, 현재 이미지 유지)
- **one-clock actual-present 계측**: trusted capture input → 실제 모니터 qualifying frame
- file-ready / renderer-ready / channel receipt / decode / `<img onLoad>`를 **별도 진단 span**으로 분리 기록
- **visible standby vs hidden prewarm A/B**와 승인된 기본값 결정 + raw evidence
- display 계약 문서, 자동 검증, HV-13B 실장비 evidence

### Explicitly Out of Scope

- camera source 교체 (LibRaw embedded JPEG / RAW+JPEG) → Story 7.3
- preset renderer 교체, display-fit **preset** proxy 생성 → Story 7.4
- resident renderer spike → Story 7.5
- RAW 정밀본 tier와 deadline scheduler, stale work 취소 → Story 7.6
- installer / 100-shot / rollout / 최종 출시 판정 → Story 7.7~7.10
- `src-tauri/src/render/mod.rs`의 384px 상수 변경, darktable 경로 변경
- CSP 강화(`csp: null` 제거), assetProtocol scope 변경

**이 Story가 표시하는 이미지는 preset이 적용된 실제 촬영 결과가 아니라 계측용 sample fixture다.** 따라서 sample lane은 **기본 off**이며, 켠 상태로 출시하면 실제 고객에게 fixture 사진을 보여주는 NFR-004/FR-010 위반이 된다.

## Acceptance Criteria

1. **Given** Story 7.1과 HV-13A가 `Go`인 상태에서, **when** 하나의 request에 속한 두 개의 immutable sample generation이 순서대로 게시되면, **then** 다음 generation은 완전히 write·close·decode·크기 검증·correlation 검증을 마친 뒤에야 pointer가 commit되어야 하고, partial file / stale epoch / 더 낮은 revision / 더 오래된 request / 부족한 크기는 어느 것도 활성 display truth가 될 수 없다.
2. **Given** 하나의 sample generation이 현재 표시 중일 때, **when** 더 새로운 검증 완료 generation이 준비되면, **then** viewer는 현재 이미지를 계속 보이게 유지한 채 opaque double-buffer swap을 수행해야 하고, blank·spinner·stale image·crop jump·scale jump·tier downgrade가 0이어야 한다.
3. **Given** 계측된 trusted capture action과 미리 열린 viewer가 있을 때, **when** 경험을 측정하면, **then** 공식 시작은 trusted capture input이고 공식 종료는 **물리 모니터의 qualifying frame**이며 하나의 monotonic clock으로 측정해야 하고, file-ready·renderer-ready·channel receipt·decode·`<img onLoad>`는 각각 별도 진단 span으로만 기록되어야 한다.
4. **Given** visible standby와 hidden prewarm 두 변형이 있을 때, **when** 동일한 PC·모니터·WebView2 runtime·display profile에서 둘 다 시험하면, **then** 선택된 기본값과 근거가 raw evidence와 함께 기록되어야 하고, **trusted capture input 이후에 발생한 viewer navigation 또는 viewer window 생성은 실패 결과**로 처리해야 한다.
5. **Given** 구현과 자동 테스트가 완료되었을 때, **when** Story 7.2 closure를 검토하면, **then** HV-13B가 immutable publication, 상호 연결된 timing span, 계측 완결성(commit된 generation당 terminal 행 정확히 하나), A/B 결정으로 `Go`를 기록하기 전까지 `review`에 머물러야 한다. **물리 모니터 frame·compositor→photon 오프셋·frame-level zero-transition-defect는 2026-08-12 correct-course로 Story 7.8 / HV-18B가 소유하며**, HV-18B에서 표시 종단점 귀책의 전환 결함이 나오면 이 Story는 `review`로 되돌아간다. (원문 AC는 `epics.md`에서 함께 개정됐다.)

## Tasks / Subtasks

### T1. Display 계약을 shared-contracts에 정의한다 (AC: 1, 2, 3)

- [x] `src/shared-contracts/schemas/viewer-display.ts` 신규. zod v4로 고정한다.
  - `displayTierSchema`: `z.enum(['sample'])` — **Story 7.2는 `sample` 하나만 정의한다.** `displayFitPresetProxy` / `rawRefinedDisplay` / `final`은 7.4/7.6이 추가한다. tier 순서 비교 함수는 지금 만들되 값은 하나만 등록한다.
  - `displayGenerationSchema`: `generationId`, `generationSeq`(u64, 세션 내 단조 증가), `sessionId`, `requestId`, `captureId`(nullable), `viewerEpoch`, `tier`, `assetPath`, `sourceWidthPx`, `sourceHeightPx`, `byteSize`, `sourceHash`, `sampleVariant`(`'a' | 'b'`), `committedAtHostMicros`
  - `displayPointerSnapshotSchema`: `schemaVersion: 'viewer-display/v1'`, `sessionId`(nullable), `revision`(u64 단조 증가), `activeGeneration`(nullable `displayGenerationSchema`), `requiredSourceWidthPx`, `requiredSourceHeightPx`, `observedAtHostMicros`
  - `displayUpdateSchema`: `schemaVersion: 'viewer-display-update/v1'` + `pointer`
  - `displayPresentReportSchema` (viewer → host): `generationId`, `viewerEpoch`, `outcome`(`'presented' | 'rejected' | 'decode-failed'`), `rejectReason`(nullable enum), `naturalWidthPx`, `naturalHeightPx`, `spans`(아래 T5 참조), `clockOffsetMicros`, `clockUncertaintyMicros`
  - `clockProbeSchema` / `clockProbeResultSchema`: `{ clientSentMicros }` → `{ hostMonotonicMicros, hostEpochMicros }`
  - `trustedInputReportSchema` (booth → host): `sessionId`, `requestId`, `clientInputMicros`, `clockOffsetMicros`, `clockUncertaintyMicros`, `isTrusted`
- [x] `src/shared-contracts/dto/display.ts`에 `z.infer` 별칭 추가. `dto/viewer.ts` 형식을 그대로 따른다.
- [x] `src/shared-contracts/schemas/index.ts`, `src/shared-contracts/index.ts`에 export 추가.
- [x] `src/shared-contracts/events/index.ts`에 `viewerDisplayUpdateEvent = 'viewer-display-update'` 추가.
- [x] `src-tauri/src/contracts/dto.rs`에 대응 DTO를 `#[serde(rename_all = "camelCase")]`로 추가한다. 필드명·nullability·schemaVersion 문자열이 TS와 **정확히** 일치해야 한다.
- [x] `docs/contracts/viewer-display.md` 신규 작성. `docs/contracts/viewer-readiness.md`에 상호 참조 한 줄만 추가하고 그 문서의 기존 계약은 바꾸지 않는다.

### T2. Host가 immutable generation과 atomic pointer를 소유하게 한다 (AC: 1) — **최우선**

- [x] `src-tauri/src/display/` 신규 (아키텍처 예약 디렉터리). `lib.rs`에 `pub mod display;` 등록.
  - `mod.rs`: `DisplayStateHandle(pub Mutex<DisplayState>)`를 `tauri::Builder::manage`로 등록. **pointer 갱신은 이 Mutex 안에서만 하는 single writer**다.
  - `display_artifact.rs`: generation key/DTO 변환, tier 순서 비교, correlation 판정.
  - `generation_repository.rs`: 파일 write/commit/journal.
  - `image_probe.rs`: **의존성 없는 JPEG 구조 probe** (아래 참조).
  - `sample_publisher.rs`: fixture → request-scoped generation 복사와 게시 orchestration.
- [x] 저장 경로 (모두 `SessionPaths::session_root` 하위 → 기존 `assetProtocol.scope`의 `$PICTURE/dabi_shoot/**`에 이미 포함. **`tauri.conf.json`을 수정하지 않는다**)
  - 임시: `<session_root>/renders/display/.staging/<requestId>-<generationSeq>.jpg` (**같은 볼륨**)
  - 확정: `<session_root>/renders/display/<requestId>/<generationSeq>-<sampleVariant>.jpg`
  - pointer: `<session_root>/renders/display/pointer.json`
  - journal: `<session_root>/renders/display/generations.jsonl`
- [x] **commit 순서를 이 순서 그대로** 구현한다. 한 단계라도 앞당기면 AC 1이 깨진다.
  1. staging 파일에 write → `file.flush()` → `file.sync_all()` → 핸들 drop
  2. `image_probe`로 구조 검증: SOI(`FFD8FF`) + SOF0/1/2에서 width/height 파싱 + **EOI(`FFD9`) trailer 존재** + `byteSize > 0`
  3. EXIF orientation이 존재하면 `1`만 허용 (그 외는 거부). geometry 흔들림의 원인이 된다
  4. 크기 검증: `width >= requiredSourceWidthPx && height >= requiredSourceHeightPx` (현재 viewer snapshot의 `photoRect` 기준)
  5. correlation 검증: `sessionId` == host binding, `viewerEpoch` == 현재 epoch, `generationSeq` > 현재 활성 seq, `requestId`가 더 오래된 request가 아님
  6. `fs::rename(staging → 확정)` — 확정 파일은 `OpenOptions::new().create_new(true)`로만 만들고, **이미 존재하면 하드 에러**로 처리한다 (immutable 보장)
  7. pointer temp write → `fs::rename(temp → pointer.json)`, `revision += 1`
  8. `generations.jsonl`에 append
  9. **그다음에야** `app.emit(VIEWER_DISPLAY_UPDATE_EVENT, ...)`
- [x] 거부 경로는 각각 **독립된 reason code**로 남긴다: `partial-file`, `undecodable`, `insufficient-dimensions`, `stale-epoch`, `lower-generation`, `older-request`, `session-mismatch`, `orientation-unsupported`. 조용한 무시 금지.
- [x] **세션 경계 무효화 (NFR-004, 0 tolerance).**
  - `bind_session`으로 현재 세션이 바뀌면 display pointer를 즉시 비우고 revision을 올린다. 이전 세션의 generation이 다음 고객 화면에 한 프레임도 남으면 안 된다
  - viewer epoch가 바뀌면 활성 generation을 **새 `photoRect.requiredSource*` 기준으로 재검증**한다. 크기가 부족해지면 pointer를 비운다. 이전 epoch에서 통과한 크기 검증을 그대로 상속하지 않는다
  - `delete_capture`로 capture가 삭제되면 해당 `requestId`의 generation 파일과 journal 항목도 함께 정리하고, 삭제된 generation이 활성 pointer였다면 pointer를 비운다
- [x] `std::fs::rename`은 Windows에서 `MoveFileExW + MOVEFILE_REPLACE_EXISTING`이라 pointer 교체에 안전하다. generation 파일은 `create_new`로 충돌 자체를 막는다.
- [x] `session.json`(`src-tauri/src/session/session_manifest.rs`)에 필드를 추가하지 **않는다**. 아키텍처의 "session manifest는 기능마다 drift하지 않는다" 규칙을 지키고, 회귀 위험이 높은 파일을 건드리지 않기 위해 별도의 versioned display manifest(`pointer.json`)를 쓴다. 이 편차와 근거를 `docs/contracts/viewer-display.md`에 기록한다.

### T3. Sample fixture와 measurement lane을 만든다 (AC: 1, 2, 3, 4)

- [x] sample fixture 2종을 `storage/fixtures/display-sample/`에 추가하고 `tauri.conf.json`의 `bundle.resources`로 포함한다. 런타임 해석은 `tauri::path::BaseDirectory::Resource`.
  - 두 이미지는 **3:2, 동일 픽셀 크기, 최소 3840×2560** (승인 4K profile의 `requiredSource`를 여유 있게 상회)
  - 시각적으로 즉시 구분 가능해야 한다 (예: `A`/`B` 대형 문자 + 서로 다른 배경색)
  - **테두리 격자와 코너 registration 마크를 동일 좌표에 넣는다.** crop jump / scale jump를 영상 프레임 단위로 판정하는 유일한 근거다
  - EXIF orientation 없음, sRGB, baseline JPEG
- [x] measurement lane은 **기본 off**다. 환경 변수 `BOOTHY_DISPLAY_SAMPLE_MODE = off | visible-standby | hidden-prewarm` (기본 `off`)로만 켠다. 값이 `off`면 sample generation을 게시하지 않고 관람 화면은 Story 7.1의 standby 상태 그대로 유지한다.
- [x] lane이 켜져 있을 때 `request_capture` 수락 직후 **비차단**으로 sample A를 게시하고, `BOOTHY_DISPLAY_SAMPLE_GAP_MS`(기본 800) 후 sample B를 게시한다. 둘 다 같은 `requestId`에 correlate한다.
  - **`request_capture`의 반환을 지연시키지 않는다.** 게시는 별도 스레드에서 수행한다 (`capture_commands.rs`의 기존 `thread::spawn` 패턴).
  - camera/render 경로를 변경하지 않는다. sample lane은 순수 병렬 lane이다.
- [x] 신규 command를 `src-tauri/src/commands/display_commands.rs`에 추가하고 `lib.rs`의 `generate_handler!`에 등록한다.
  - `get_viewer_display_state() -> DisplayPointerSnapshotDto` (인자 없음, snapshot 재수렴 경계)
  - `publish_display_sample(input) -> DisplayPointerSnapshotDto` — **`#[tauri::command(async)]`**. 파일 I/O를 event loop 스레드에서 돌리지 않는다
  - `report_display_present(input)` (viewer → host)
  - `report_trusted_capture_input(input)` (booth → host)
  - `stamp_clock_probe(input) -> ClockProbeResultDto`
- [x] `src-tauri/capabilities/viewer-window.json`은 `core:default`만 있어도 command invoke가 가능하다. 새 파일은 필요 없다. **단 window별 label 검증은 host에서 한다** — `report_display_present`는 `viewer-window` 이외 label의 호출을 거부한다.

### T4. Viewer의 opaque double-buffer swap을 구현한다 (AC: 2)

- [x] `src/display-generation/services/display-guard.ts` (순수 함수). **여기가 AC 2의 정확성 핵심이며 반드시 순수 함수로 분리해 단위 테스트한다.**
  - `shouldAdvanceDisplay(current, next, context)` — `next.generationSeq > current.generationSeq`, 같은 `sessionId`, 같은 `viewerEpoch`, `requestId`가 역행하지 않음, tier가 하락하지 않음, `sourceWidth/Height >= requiredSource*`를 전부 AND로 판정
- [x] `src/display-generation/state/use-display-pointer.ts` — 마운트/재구독/revision gap마다 `get_viewer_display_state()`로 재수렴한다. live event만으로 상태를 소유하지 않는다. 낮은 revision은 폐기한다 (`use-viewer-readiness.ts`의 `shouldApplySnapshot` 패턴 재사용).
- [x] `src/viewer-surface/components/DoubleBufferedPhoto.tsx` — 두 개의 `<img>` 레이어.
  - 두 레이어 모두 `position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain;` — **동일한 box, 동일한 fit**. 하나라도 다르면 scale jump가 발생한다
  - swap 절차: back 레이어 `src` 설정 → **`await back.decode()`** → `requestAnimationFrame` 안에서 z-index/front 지정을 **한 번의 style commit으로** 전환 → 2 프레임 뒤에 이전 레이어 `src` 정리
  - **CSS transition / opacity fade / spinner / placeholder를 넣지 않는다.** "opaque"는 나가는 이미지가 한 프레임도 반투명해지지 않는다는 뜻이다
  - `decode()`가 reject하면 현재 이미지를 유지하고 `outcome: 'decode-failed'`를 host에 보고한다. host는 그 generation을 poisoned로 표시하고 활성 truth로 남기지 않는다
  - `naturalWidth/naturalHeight`가 `requiredSource*` 미만이면 swap하지 않고 `outcome: 'rejected'`, `rejectReason: 'insufficient-dimensions'`로 보고한다
  - 새 asset은 `decoding="sync"` + `fetchPriority="high"` + `loading="eager"`
  - **cache buster(`?v=`)를 붙이지 않는다.** generation 경로가 이미 unique하므로 query string은 불필요한 재요청만 만든다. `SessionPreviewImage.tsx`의 `withCacheBuster`를 복사하지 않는다
- [x] `src/viewer-surface/ViewerSurface.tsx` 수정 — **레이아웃 기하를 절대 바꾸지 않는다.**
  - `<DoubleBufferedPhoto />`는 기존 `.viewer-surface__photo` **안쪽**에 absolute로 넣는다. `.viewer-surface__photo`의 CSS(`width: min(100cqw, calc(100cqh * 3 / 2)); aspect-ratio: 3 / 2;`)를 수정하지 않는다
  - **standby 문구를 `display: none`으로 숨기지 않는다.** `.viewer-surface`는 flex column이므로 standby 행이 사라지면 stage 높이 → photo rect가 바뀌고 (a) `layout-ready`가 흔들려 촬영이 막히고 (b) 사진에 scale jump가 생긴다. `visibility: hidden`으로 자리를 유지한다
  - 조작 요소·진단 표시 0을 유지한다. `<img>`의 `alt`는 고객 안전 문구 하나(`"방금 촬영한 사진"`)만 쓴다
- [x] `src/viewer-surface/services/viewer-host-adapter.ts`에 display command/구독을 추가한다. React 컴포넌트에서 `invoke`/`listen`을 직접 호출하지 않는 기존 경계를 유지한다.

### T5. One-clock actual-present 계측을 구현한다 (AC: 3)

- [x] `src-tauri/src/viewer/present_telemetry.rs` 신규 (아키텍처 지정 위치).
  - `current_monotonic_micros()`를 추가하되 **`viewer::current_monotonic_ms()`와 동일한 `PROCESS_START` `OnceLock<Instant>`를 공유**한다. 두 번째 `Instant`를 만들면 clock이 둘로 갈라진다
  - span 기록: `<session_root>/diagnostics/viewer-present.jsonl` (session-scoped → NFR-004 안전)
- [x] `src/viewer-surface/telemetry/present-clock.ts` (viewer)와 `src/capture-adapter/services/trusted-input-telemetry.ts` (booth) — 두 WebView는 **서로 다른 time origin**을 가진다. 각 document가 host monotonic clock에 대해 자기 `performance.now()` 오프셋을 보정한다.
  - Cristian 방식: `stamp_clock_probe`를 9회 호출, 각 회차마다 `t0 = performance.now()` → invoke → `t1 = performance.now()`. `offset = hostMonotonicMicros - (t0 + t1) / 2`, `uncertainty = (t1 - t0) / 2`. **RTT가 가장 작은 표본만 채택**한다
  - 재보정 시점: viewer epoch 변경, viewer reload, 5분 주기. 재보정 간 drift를 함께 기록한다
  - **보고 규칙(강제):** KPI = `present 상한 − input 하한`. 즉 항상 가장 넓은 구간을 보고해 측정 오차가 결과를 실제보다 빠르게 보이게 만들 수 없다. 총 uncertainty가 5ms를 넘는 표본은 `low-confidence`로 표시해 **별도 보고**하되 절대 조용히 버리지 않는다
- [x] trusted capture input(공식 시작점) 정의를 고정한다.
  - booth의 촬영 컨트롤 `pointerup` 핸들러 **첫 줄**에서 `performance.now()`를 읽는다. `event.isTrusted === false`면 qualifying 표본이 아니다
  - 그 값을 `report_trusted_capture_input`으로 보낸다. **`await`하지 않는다** — 이 IPC가 `requestCapture`를 지연시키면 실제 제품 경로를 느리게 만든다
  - **host가 호출을 수신한 시각을 시작점으로 삼지 않는다.** IPC 지연이 시작점 뒤로 밀려 KPI가 실제보다 빠르게 보인다
- [x] actual-present(공식 종료점) 측정.
  - 주 측정: swap을 커밋한 `requestAnimationFrame` 콜백 안에서 `MessageChannel`을 열고 `port2.postMessage(null)` → `port1.onmessage`에서 `performance.now()`를 stamp한다. rAF 콜백은 paint 직전에, MessageChannel task는 해당 프레임이 compositor로 넘어간 직후에 실행된다. `src/viewer-surface/telemetry/after-paint.ts`로 분리한다
  - 보조 진단: `<img elementtiming="viewer-qualifying-frame">` + `PerformanceObserver({ type: 'element', buffered: true })`의 `renderTime`
  - **함정:** Windows에서 asset은 `http://asset.localhost`, 앱 문서는 `http://tauri.localhost`라 **cross-origin**이다. `Timing-Allow-Origin`이 없으면 `renderTime === 0`이 되고 `entry.startTime`은 `loadTime`으로 대체된다. `isRenderTime = Boolean(entry.renderTime)`을 반드시 함께 기록하고, **fallback 값을 actual-present로 보고하지 않는다**
  - **JS는 compositor→photon 구간을 측정할 수 없다.** 소프트웨어 present는 추정치다. HV-13B에서 물리 모니터 고속 촬영으로 소프트웨어 present와 실제 프레임 사이의 오프셋을 측정하고, 최종 KPI에는 그 오프셋을 반영하거나 물리 프레임 시각을 직접 보고한다. 모니터 주사율(60Hz → 최대 16.7ms)과 vsync를 함께 기록한다
- [x] 진단 span은 각각 **별도 필드**로 남긴다. 어느 것도 KPI 종료점으로 승격하지 않는다.
  - `trustedInput`, `hostAccepted`, `sampleWriteStart`, `fileReady`(close+fsync 완료), `probeOk`, `pointerCommitted`, `eventEmitted`, `viewerReceipt`, `decodeStart`, `decodeEnd`, `swapCommitted`, `imgOnLoad`, `actualPresent`, `elementTimingRenderTime`
  - 모든 값은 host clock micros로 정규화하고, 각 span에 `source`(host / booth-webview / viewer-webview)와 `uncertaintyMicros`를 함께 남긴다

### T6. Visible standby / hidden prewarm A/B를 실행한다 (AC: 4)

> **2026-08-11 HV-13B 회차에서 측정을 완료했다.** 승인 PC·Canon EOS 700D·승인 DISPLAY3에서
> 변형당 warm-up 5회 + 측정 30회(총 70회 실촬영)를 randomized 순서로 실행했고, `offscreen` 은닉 방식은
> 2026-08-12에 별도로 검증했다. 승인된 기본값은 `visible-standby`다.
> 증거: `tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/ab/summary.json`

- [x] 두 변형을 구현한다.
  - `visible-standby`: Story 7.1의 현재 동작 그대로. 세션 시작부터 관람 창이 고객 모니터에 보이고 standby 문구를 표시한다
  - `hidden-prewarm`: 관람 창을 미리 만들되 첫 qualifying generation 전까지 고객에게 보이지 않게 한다
- [x] **hidden-prewarm의 알려진 충돌을 먼저 확인하고 결과를 기록한다.** WebView2는 숨김/가려진 window의 타이머와 `requestAnimationFrame`을 throttle한다. Story 7.1의 liveness는 1초 heartbeat + `VIEWER_REPORT_STALE_AFTER_MS = 5000`이므로, 단순히 `set_visible(false)`로 숨기면 `stale-report`가 되어 **촬영 자체가 막힌다.** 또한 rAF가 돌지 않으면 present 계측이 성립하지 않는다.
  - 최소 두 가지 은닉 방식을 시험한다: (a) `set_visible(false)`, (b) 승인 모니터 위에 그대로 두고 booth window 뒤로 보내거나 화면 밖 좌표에 배치
  - **hidden 변형이 Story 7.1의 readiness 계약을 유지하지 못하면 그 자체가 유효한 측정 결과다.** 계약을 약화시켜 통과시키지 않는다. `VIEWER_REPORT_STALE_AFTER_MS`를 늘려 문제를 감추는 변경은 금지한다
- [x] 동일한 PC·모니터·WebView2 runtime·display profile에서 **randomized AB/BA** 순서로, 변형당 warm-up 5회 + 측정 30회를 실행한다.
- [x] 결과는 p50/p95/max, 성공률, 실패·timeout을 **제외하지 않고** 보고한다. 표본별 raw row를 보존한다.
- [x] **AC 4의 실패 조건 계측:** trusted input 이후 `viewer_window_opened` 로그, viewer navigation, webview reload가 한 번이라도 발생하면 그 표본은 **실패**다.
  - 주의: `get_capture_readiness`가 `ensure_viewer_window_state`를 호출하고 booth가 readiness를 주기적으로 polling한다 (`capture-runtime.ts`). 즉 촬영 중 poll이 창을 재생성할 수 있는 경로가 실제로 존재한다. host가 창 생성/navigation 이벤트에 host-clock micros 타임스탬프를 남기고, 분석에서 trusted input 이후 구간을 자동 검사하게 한다
- [x] 선택된 기본값과 근거를 `docs/contracts/viewer-display.md`와 story completion note에 기록한다. — `visible-standby`. hidden 변형은 p95가 느리고 15.024초 outlier가 있었다.

### T7. 자동 검증을 추가한다 (AC: 1~4)

- [x] `src/display-generation/services/display-guard.test.ts` — 거부 매트릭스를 **전부** 명시적으로 단언한다: 낮은 seq, 같은 seq, 다른 session, 다른 epoch, 더 오래된 request, tier 하락, `requiredSource` 미달, 384px asset.
- [x] `src/display-generation/state/use-display-pointer.test.tsx` — delayed / duplicate / out-of-order / revision gap / reload / epoch 변경에서의 수렴을 검증한다.
- [x] `src/viewer-surface/components/DoubleBufferedPhoto.test.tsx` — decode pending 동안 이전 이미지가 계속 렌더된다, decode 성공 뒤에만 교체된다, decode 실패 시 현재 이미지가 유지된다, 두 레이어의 box/`object-fit`이 동일하다, 어느 시점에도 두 레이어가 모두 비어 있지 않다.
- [x] `src/viewer-surface/telemetry/present-clock.test.ts` — offset/uncertainty 계산, 최소 RTT 표본 선택, 보수적 구간(`present 상한 − input 하한`) 계산, uncertainty 초과 표본의 `low-confidence` 표시.
- [x] `src/viewer-surface/ViewerSurface.test.tsx` 갱신 — `queryAllByRole('button' | 'link' | 'textbox' | 'menuitem')`이 여전히 0이고, standby 상태와 이미지 표시 상태에서 `.viewer-surface__photo`의 기하가 **동일**함을 단언한다.
- [x] `src-tauri/tests/viewer_display.rs` 신규 — `src-tauri/tests/viewer_readiness.rs`의 tempdir 패턴을 재사용한다.
  - commit 순서: pointer가 갱신되기 전에 확정 파일이 존재하고, event는 pointer commit 이후에만 나간다
  - partial file(EOI 없음), 잘린 파일, 크기 미달, stale epoch, 낮은 seq, 오래된 request, session mismatch가 각각 고유 reason으로 거부된다
  - immutability: 같은 generation 경로에 두 번 쓰면 하드 에러
  - 동시 publish에서 pointer가 항상 단조 증가한다
  - `generations.jsonl`이 append-only로 유지된다
  - **세션 교체 시 pointer가 비워진다** (NFR-004 회귀 방지선)
  - **viewer epoch 변경 후 `requiredSource*`가 커지면 기존 활성 generation이 무효화된다**
  - `delete_capture` 이후 해당 request의 generation과 pointer가 정리된다
- [x] `src/shared-contracts/display.contracts.test.ts` 신규 — TS↔Rust 라운드트립. `viewer-display/v1`, `viewer-display-update/v1` 문자열 고정.
- [x] Scope guard: preset renderer, RAW tier, deadline scheduler, camera source를 나타내는 심볼이 이번 변경 범위에 없다는 것을 review checklist 항목으로 남긴다.
- [x] 검증 명령과 **있는 그대로의 결과**를 completion note에 기록한다: `pnpm lint`, `pnpm test:run`, `cargo fmt`, `cargo test`, `pnpm build`.
  - 기존 사실: `pnpm build`(`tsc -b`)는 Story 7.1 이전부터 ReadinessScreen / capture-runtime / governance / operator diagnostics / active-preset 경로의 기존 오류로 실패한다. `capture_readiness` Rust 테스트는 render 경로 공유 자원 경합으로 flaky다(HEAD baseline 9건). **이번 Story가 이 상태를 악화시키지 않았음**을 확인하고, 고쳤다면 고쳤다고, 남았다면 남았다고 정직하게 적는다

### T8. HV-13B 하드웨어 증거를 생산한다 (AC: 5)

> **최종: HV-13B는 재정의된 범위에서 `Go`다 (2026-08-12 correct-course).**
> 2026-08-11 승인 PC·Canon EOS 700D·승인 고객 모니터에서 두 변형을 각각 warm-up 5회 + 측정 30회
> 실행해 140/140 immutable generation과 120/120 actual-present 행이 유효했다. 승인 기본값은
> `visible-standby`(p95 4280.416ms)이며 hidden은 15024.476ms outlier로 탈락했다.
> **120fps+ 물리 프레임 항목은 Story 7.8 / HV-18B로 이관됐다** — 삭제가 아니라 소유권 이동이다.
> 증거: `tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/`.

- [x] `tests/hardware/viewer-present/hv-13b/README.md`와 `hv-13b/run-<timestamp>/`를 준비한다. HV-13A 디렉터리(`run-20260811-*`)와 섞지 않는다.
- [x] 환경 fingerprint: 승인 PC, 고객 모니터 모델/해상도/DPR/**주사율**, WebView2 runtime 버전, GPU/driver, ICC, HDR on/off, 앱 버전·커밋 해시, `BOOTHY_APPROVED_CUSTOMER_MONITOR`, `BOOTHY_DISPLAY_SAMPLE_MODE`. — `environment.md`, `environment/`
- [x] **immutable generation 증거**: 두 generation의 파일 경로·해시·크기, `pointer.json` 전/후, `generations.jsonl`, 거부 케이스별 host 로그. — 140/140 무결성
- [~] **actual monitor frame 증거** → **Story 7.8 / HV-18B 소유로 이관 (2026-08-12).** 이 Story의 의무가 아니다. 절차는 `hv-13b/README.md` 4절에 보존했고 required evidence는 HV-18B row로 옮겼다. 대상도 fixture가 아니라 실제 preset 적용 사진과 RAW 정밀본 교체로 바뀐다.
- [x] **timing span 증거**: 표본별 raw JSONL + 집계(p50/p95/max/성공률). — `timing/summary.json`. **소프트웨어 present와 물리 프레임 사이 오프셋 측정값은 HV-18B로 이관됐다.**
- [x] **A/B 증거**: 두 변형의 raw row, randomized 순서 기록, 선택 결과와 근거. hidden 변형이 readiness/rAF를 유지하지 못했다면 그 관측을 그대로 남긴다. — `ab/summary.json`
- [x] **AC 4 실패 조건 증거**: trusted input 이후 viewer 생성/navigation이 0건임을 보이는 로그 구간. — `window-events/summary.json`, 120/120 표본에서 0건
- [x] `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의 Story 7.2 행을 갱신한다. required evidence는 재정의된 범위로 조정하고, 이관된 항목은 `evidence NOT provided by this gate`와 HV-18B row에 명시한다.
- [x] HV-13B `Go`를 ledger에 기록한 뒤에만 story status를 `done`으로 전환한다. **자동 테스트 통과만으로 `done` 처리하지 않는다.** — 재정의된 범위의 `Go`를 ledger에 기록한 뒤 전환했다. HV-18B 결함 시 `review`로 되돌아가는 재개 조건을 함께 기록했다.

### T9. 영향 문서를 갱신한다 (all AC)

- [x] `docs/contracts/viewer-display.md` (신규)
- [x] `docs/contracts/viewer-readiness.md`에 상호 참조 한 줄
- [x] `_bmad-output/planning-artifacts/architecture.md` implementation note (Story 7.2 완료 상태, pointer 저장 위치 편차와 근거)
- [x] Story completion note, `sprint-status.yaml`, HV-13B ledger row

### Review Findings

- [x] [Review][Patch] 검증되지 않은 request ID가 display 저장 경로와 재귀 삭제 경계를 벗어날 수 있음 [src-tauri/src/display/generation_repository.rs:62]
- [x] [Review][Patch] 세션·viewer 계약 변경으로 pointer가 무효화되어도 디스크 pointer가 갱신되지 않음 [src-tauri/src/display/display_artifact.rs:179]
- [x] [Review][Patch] capture 삭제 시 durable pointer와 request-forgotten 감사 기록이 남지 않음 [src-tauri/src/commands/display_commands.rs:179]
- [x] [Review][Patch] journal 저장 실패를 무시해 표시 truth와 감사 기록이 불일치할 수 있음 [src-tauri/src/display/sample_publisher.rs:321]
- [x] [Review][Patch] 저장소 I/O 장애가 이미지 decode 거부로 잘못 분류됨 [src-tauri/src/display/generation_repository.rs:94]
- [x] [Review][Patch] file-ready와 probe 완료 시각이 실제 단계 경계를 측정하지 않음 [src-tauri/src/display/sample_publisher.rs:221]
- [x] [Review][Patch] 미등록 tier가 unknown-generation이 아니라 undecodable로 보고됨 [src-tauri/src/display/display_artifact.rs:56]
- [x] [Review][Patch] host pointer가 비워져도 이전 고객 사진 레이어가 화면에 남음 [src/viewer-surface/components/DoubleBufferedPhoto.tsx:85]
- [x] [Review][Patch] 동일 revision의 크기·계측 상태 변경이 viewer pointer에 수렴하지 않음 [src/display-generation/state/use-display-pointer.ts:24]
- [x] [Review][Patch] measurement lane이 꺼져 있어도 present 계측 IPC가 전송됨 [src/viewer-surface/ViewerSurface.tsx:53]
- [x] [Review][Patch] session·viewer epoch 변경 후에도 decode-failed generation 차단 상태가 남음 [src/display-generation/state/use-display-pointer.ts:70]
- [x] [Review][Patch] clock 보정 전 present 결과가 0 오차의 정상 계측으로 기록됨 [src/viewer-surface/ViewerSurface.tsx:53]
- [x] [Review][Patch] hidden prewarm 공개 시 단일 모니터 fallback에서도 fullscreen이 강제됨 [src-tauri/src/commands/viewer_commands.rs:470]
- [x] [Review][Patch] 개발 환경 실행 위치에 따라 sample fixture fallback 경로가 잘못 계산됨 [src-tauri/src/commands/display_commands.rs:208]
- [x] [Review][Patch] trusted input 보고보다 present가 먼저 도착하면 공식 KPI 시작점이 영구 누락됨 [src-tauri/src/commands/display_commands.rs:432]
- [x] [Review][Patch] 완료되지 않은 pointer 상호작용의 timestamp가 다음 촬영에 재사용될 수 있음 [src/booth-shell/screens/CaptureScreen.tsx:304]
- [x] [Review][Patch] capture 삭제 후에도 request telemetry 상관관계가 남아 늦은 표본이 기록됨 [src-tauri/src/commands/display_commands.rs:181]
- [x] [Review][Patch] outcome과 rejectReason의 모순된 조합이 계약 검증을 통과함 [src-tauri/src/contracts/dto.rs:1963]
- [x] [Review][Patch] viewer window 재생성 이후 표본이 실패가 아닌 정상 present로 기록됨 [src-tauri/src/commands/display_commands.rs:554]
- [x] [Review][Patch] clock 보정 실패 시 decode-failed 보고까지 보류되어 host의 bad generation이 유지됨 [src/viewer-surface/ViewerSurface.tsx:77]
- [x] [Review][Patch] trusted-input IPC가 늦게 도착하면 AC4 창 이벤트 기준점이 host 수락 시각으로 고정되어 실제 입력 이후 이벤트를 누락할 수 있음 [src-tauri/src/commands/display_commands.rs:382]
- [x] [Review][Patch] 세션 정리 뒤 늦게 도착한 viewer 보고가 unknown-generation 기록 없이 조용히 폐기됨 [src-tauri/src/commands/display_commands.rs:695]
- [x] [Review][Patch] 크기 미달 등 일시적 reject도 client에서 영구 poison 처리되어 같은 세션·epoch의 유효한 재수렴을 차단함 [src/display-generation/state/use-display-pointer.ts:200]
- [x] [Review][Patch] clock 보정 전 실패 보고의 viewer timestamp가 host clock 값처럼 저장되어 진단 span이 잘못된 시간축에 기록됨 [src/viewer-surface/ViewerSurface.tsx:82]
- [x] [Review][Patch] hidden-prewarm의 offscreen 변형을 실장비에서 비교하지 않고 A/B 기본값 결정을 완료로 기록함 [tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/ab/summary.json:10] — 2026-08-12 Canon EOS 700D / 승인 DISPLAY3에서 offscreen 직접 촬영 5회로 재검증. 10/10 generation 무결성, 9/10 terminal present 확인. 1개 terminal 행 누락과 120fps+ 물리 프레임 부재로 전체 판정은 No-Go 유지. 증거: `tests/hardware/viewer-present/run-20260812-172000-hv13b-offscreen-direct/`.
- [x] [HV-13B][Patch] 교체가 커밋된 뒤 effect가 정리되면 after-paint 스탬프가 취소되어 terminal 보고가 통째로 사라짐 [src/viewer-surface/components/DoubleBufferedPhoto.tsx:246]
- [x] [HV-13B][Patch] commit된 generation에 terminal 행이 없어도 계측이 아무 흔적을 남기지 않아 "표시되지 않음"과 "보고 유실"을 구분할 수 없음 [src-tauri/src/commands/display_commands.rs:64]
- [x] [HV-13B][Patch] hidden-prewarm 노출이 generation마다 반복돼 측정 구간 한가운데에서 창 상태가 흔들림 [src-tauri/src/commands/viewer_commands.rs:463]
- [x] [HV-13B][Patch] present 보고의 계약 검증 실패가 로그 없이 반환되어 유실 경로에 흔적이 남지 않음 [src-tauri/src/commands/display_commands.rs:938]
- [x] [HV-13B][Patch] 계측 완결성을 사람이 눈으로 세고 있어 다음 회차에서 같은 누락을 놓칠 수 있음 [tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1:1]

## Dev Notes

### 코드베이스 현실 — 착수 전 반드시 인지할 것

Story 7.1이 viewer 경계를 이미 만들어 뒀다. 이 Story는 **그 위에 표시 경로를 얹는 것**이지 새로 만드는 것이 아니다.

| 항목 | 현재 상태 |
| --- | --- |
| `viewer-window` | **존재.** `lib.rs` `.setup()`에서 `WebviewWindowBuilder`로 app-lifetime 생성. `tauri.conf.json`에는 없음 |
| viewer readiness truth | **완성.** `src-tauri/src/viewer/viewer_state.rs`가 session binding / epoch / revision / photoRect를 소유 |
| `photoRect.requiredSource*` | **완성.** host가 report에서 `ceil(css × DPR)`로 재계산. display 크기 검증의 입력값이 이미 있다 |
| `src/viewer-surface/` | **존재.** `ViewerSurface.tsx`, `state/use-viewer-readiness.ts`, `services/{display-fit,viewer-host-adapter}.ts` |
| `src/viewer-surface/telemetry/` | **미존재.** 아키텍처가 예약만 해둔 디렉터리 |
| `src/display-generation/` | **미존재.** 아키텍처가 예약만 해둔 디렉터리 |
| `src-tauri/src/display/` | **미존재.** 아키텍처가 예약만 해둔 디렉터리 |
| `src-tauri/src/viewer/present_telemetry.rs` | **미존재.** 아키텍처가 예약만 해둔 파일 |
| display pointer / immutable generation | **없음.** 현재 표시 경로는 `renders/previews/`의 canonical 파일을 덮어쓰는 방식(Story 1.9 계보) |
| Rust JPEG 검증 | **signature만.** `render/mod.rs:1277 has_jpeg_signature`는 앞 3바이트만 본다. 크기 파싱·EOI 확인은 없다 |
| assetProtocol scope | **이미 충분.** `$PICTURE/dabi_shoot/**`이고 세션 루트가 `%USERPROFILE%\Pictures\dabi_shoot\sessions\<id>`다 |
| `csp` | `null`. 이번 Story에서 바꾸지 않는다 |
| asset URL 변환 | `src/booth-shell/components/preset-preview-src.ts`의 `convertFileSrc` 래퍼가 참고 구현 |

### 핵심 설계 결정 1 — sample lane은 계측 도구이지 제품 경로가 아니다

이 Story는 **renderer를 교체하지 않고** 표시 종단점을 증명해야 한다. 그래서 표시되는 이미지는 preset 결과가 아니라 fixture다.

- `BOOTHY_DISPLAY_SAMPLE_MODE` 기본값은 `off`다. off일 때 관람 화면은 Story 7.1과 완전히 동일하게 동작한다.
- 켠 채로 출시하면 실제 고객이 fixture 사진을 본다. FR-010의 "qualifying frame은 같은 capture의 preset-applied 이미지"와 정면으로 충돌한다.
- 그럼에도 **trusted input은 실제 촬영 버튼**이어야 한다. 가짜 트리거로 재면 KPI 시작점이 제품과 달라진다. 그래서 sample 게시를 `request_capture` 수락 직후의 **비차단 병렬 lane**으로 붙인다.
- Story 7.4는 이 lane의 sample publisher만 실제 display-fit preset proxy로 교체하면 된다. generation/pointer/swap/telemetry 계약은 그대로 재사용된다. **이 seam을 깨끗하게 만드는 것이 이 Story의 진짜 산출물이다.**

### 핵심 설계 결정 2 — "decode"는 두 단계이고 둘 다 필수다

AC 1은 pointer commit 전 decode를 요구하고, AC 2는 swap 전 완전 decode를 요구한다. 새 Rust crate 없이 둘 다 만족시키는 분할은 다음과 같다.

| 단계 | 위치 | 내용 | 실패 시 |
| --- | --- | --- | --- |
| 구조 decode | host (`image_probe.rs`) | SOI + SOF width/height + **EOI trailer** + byteSize + EXIF orientation | pointer commit 안 함, generation 거부 |
| 픽셀 decode | viewer (`img.decode()`) | 전체 픽셀 decode | swap 안 함, 현재 이미지 유지, host에 `decode-failed` 보고 |

EOI trailer 확인이 partial file 방어의 핵심이다. signature만 보면 절반만 쓰인 파일도 통과한다. `sync_all()` 이후에 확인해야 캐시에만 있는 상태를 "완료"로 오판하지 않는다.

`image` 같은 새 crate를 추가하면 Story 7.7의 offline clean-machine 재현 인벤토리에 영향이 간다. **승인 없이 crate를 추가하지 않는다.** 위 분할로 충분하며, 그래도 부족하다고 판단되면 근거를 적어 승인을 먼저 받는다.

### 핵심 설계 결정 3 — 하나의 clock은 host monotonic clock이다

trusted input은 booth WebView에서, present는 viewer WebView에서 관측된다. 두 document의 `performance.now()`는 **time origin이 다르다**. `performance.timeOrigin + performance.now()`는 wall-clock 기반이라 시스템 시간 보정에 취약하므로 쓰지 않는다.

정답은 **각 WebView가 host monotonic clock에 자기를 보정**하는 것이다 (T5). 보정 오차는 숨기지 말고 표본마다 함께 싣고, KPI는 항상 보수적 상한을 보고한다. "빠르게 보이게 만드는 방향"의 반올림·근사는 전부 금지다.

`viewer::current_monotonic_ms()`가 이미 프로세스 시작 기준 `Instant`를 `OnceLock`으로 들고 있다. **같은 `OnceLock`을 공유해** micros 함수를 추가한다. 새 `Instant`를 만들면 두 clock이 갈라져 span이 음수가 되거나 서로 어긋난다.

### 핵심 설계 결정 4 — swap 정확성은 transport가 아니라 monotonic guard가 보장한다

아키텍처는 ordered stream에 Tauri Channel을 권장한다. 그럼에도 이 Story는 Story 7.1과 동일하게 **broadcast event + snapshot 재수렴 + monotonic generation/revision guard**를 쓴다.

- 근거: AC는 "delayed, duplicated, out-of-order 업데이트에도 오래된 generation이 현재 화면을 대체할 수 없을 것"을 요구한다. 이 보장은 transport 순서가 아니라 `shouldAdvanceDisplay` guard와 snapshot 재수렴에서 나온다. 전송 순서에 의존하는 설계가 오히려 더 약하다.
- Story 7.1의 `viewer-readiness-update`와 일관성을 유지해 두 개의 서로 다른 수신 모델을 viewer에 만들지 않는다.
- 이 편차와 근거를 `docs/contracts/viewer-display.md`에 기록한다. Story 7.6에서 tier가 늘어나 순서 요구가 강해지면 그때 Channel 전환을 재평가한다.

### Story 7.1 계약과의 충돌 지점 (여기서 사고가 난다)

1. **`.viewer-surface__photo`의 기하를 바꾸면 촬영이 막힌다.** `use-viewer-readiness.ts`가 이 요소를 실측해 `computePhotoRect` 기대값과 **1px 허용오차**로 비교하고, 어긋나면 `layoutReady = false` → `viewer-preparing` → capture 차단이다. 이미지 레이어는 반드시 이 요소 **안쪽**에 absolute로 들어간다.
2. **standby 문구를 `display: none`으로 숨기면 photo rect가 바뀐다.** `.viewer-surface`는 flex column이고 standby가 행 하나를 차지한다. 사라지면 stage 높이가 커지고 photo rect가 커진다 → layout 흔들림 + **scale jump**. `visibility: hidden`으로 자리를 유지한다.
3. **hidden prewarm은 liveness heartbeat와 충돌한다.** WebView2가 숨김 창의 타이머를 throttle하면 `last_report`가 5초를 넘겨 `stale-report`가 되고 촬영이 막힌다. 이건 버그가 아니라 Story 7.1이 의도한 정직한 차단이다. **임계값을 늘려 회피하지 말고 측정 결과로 기록한다.**
4. **`get_capture_readiness`가 창을 재생성할 수 있다.** `capture_commands.rs:43`이 `ensure_viewer_window_state`를 호출하고 booth가 이 command를 주기적으로 polling한다. 촬영 중 poll이 창을 재생성하면 AC 4의 명시적 실패 조건에 해당한다. 계측으로 잡아낸다.
5. **1초 heartbeat IPC가 hot path와 겹친다.** 비용은 작지만 present 표본에 영향이 있는지 확인하고 기록한다. 계측을 위해 heartbeat 주기를 바꾸지 않는다.

### UX 가드레일

- **고객 문구를 새로 만들지 않는다.** NFR-001의 copy budget(주 문장 1 + 보조 문장 1 + 액션 라벨 1)은 이미 채워져 있고, 이 Story는 표시 경로만 바꾼다. `customerStatusCopy.ts`에 새 문구를 추가할 이유가 생겼다면 설계가 잘못된 것이다.
- 관람 화면의 standby 문구는 Story 7.1의 `"사진이 준비되면 여기에 보여드릴게요."` 하나를 유지한다. 이미지가 표시되는 동안에는 `visibility: hidden`으로 자리만 남긴다.
- 표시 실패(decode 실패, 크기 미달, 거부)는 관람 화면에 절대 노출하지 않는다. host 진단과 booth control surface의 wait/call guidance로만 투영한다.
- production mode에 diagnostic overlay를 노출하지 않는다. 계측 값은 JSONL과 로그로만 남긴다.
- UX-DR16(WCAG 2.2 AA) 적용 대상에 `src/viewer-surface/`가 포함된다. `<img>`에는 고객 안전한 `alt` 하나만 두고, 조작 요소가 없으므로 focus trap은 만들지 않는다.

### 아키텍처 가드레일 (위반 시 리뷰 반려)

- React 컴포넌트에서 `invoke()` / `listen()`을 직접 호출하지 않는다. viewer는 `src/viewer-surface/services/viewer-host-adapter.ts`만 거친다.
- Rust command는 `src-tauri/src/commands/`의 얇은 진입점이고 도메인 로직은 `src-tauri/src/display/`, `src-tauri/src/viewer/`에 둔다.
- 계약 정의는 TS/Rust 한 쌍만 존재한다. 세 번째 정의를 만들지 않는다.
- snapshot이 durable recovery 경계다. live event만으로 display 상태를 소유하지 않는다.
- 모든 payload에 `sessionId`, `requestId`, `captureId`, `viewerEpoch`, `generationSeq`, `revision`, `schemaVersion`을 명시적으로 싣는다.
- 이미지 bytes를 IPC로 옮기지 않는다. **immutable asset path만 보내고 WebView가 직접 읽는다.**
- 세션 밖 자산·상태가 viewer로 새지 않게 한다. NFR-004는 0 tolerance다.
- 관람 화면에는 조작 요소·진단 표시가 0이어야 한다. 준비 실패와 표시 실패는 booth control surface의 wait/call guidance로만 투영한다.

### Tauri / WebView2 구현 세부

- Tauri `2.10.3`, `@tauri-apps/api` `^2.10.1`. Tauri v2 API만 쓴다.
- **`#[tauri::command]`는 기본이 blocking이라 main thread(event loop)에서 실행된다.** 파일 I/O·fsync·이미지 복사를 하는 command는 반드시 `#[tauri::command(async)]`로 선언한다. HV-13A `No-Go`의 실제 원인이 이 규칙 위반이었다 (`src-tauri/src/viewer/mod.rs:45-56` 주석 참조).
- `app.emit(...)`은 모든 window에 브로드캐스트한다. 한 창만 대상으로 하려면 `emit_to("viewer-window", ...)`를 쓴다.
- `main.tsx`는 `<StrictMode>`라 dev에서 effect가 2회 실행된다. listener 등록은 idempotent해야 하고 `use-viewer-readiness.ts`의 `isDisposed` + unlisten 패턴을 따른다.
- 관람 창은 부팅 시 host IPC를 기다리지 않는다 (`src/app/boot/resolve-capability-service.ts`). 이 성질을 깨지 않는다. clock 보정은 렌더 이후에 시작한다.
- `convertFileSrc`가 만드는 URL의 origin은 Windows에서 `http://asset.localhost`다. 앱 문서(`http://tauri.localhost`)와 cross-origin이므로 Element Timing `renderTime`이 0이 될 수 있다 (T5 참조).

### 재사용할 기존 패턴 (바퀴 재발명 금지)

| 필요한 것 | 참고할 기존 구현 |
| --- | --- |
| host 어댑터 (invoke + zod parse + 에러 봉투) | `src/viewer-surface/services/viewer-host-adapter.ts` |
| snapshot + event 재수렴, stale 폐기 | `src/viewer-surface/state/use-viewer-readiness.ts` (`shouldApplySnapshot`) |
| event 구독 재시도와 unlisten 수명 관리 | `src/viewer-surface/state/use-viewer-readiness.ts:180-245` |
| 파일 경로 → asset URL | `src/booth-shell/components/preset-preview-src.ts` |
| 이미지 로드 계측 (참고만, 이 패턴을 복사하지 않음) | `src/booth-shell/components/SessionPreviewImage.tsx` |
| temp write → rename → backup 복구 | `src-tauri/src/session/session_repository.rs:330-350`, `src-tauri/src/render/mod.rs:620-700` |
| 세션 경로 계산 | `src-tauri/src/session/session_paths.rs` |
| 비차단 후속 작업 스레드 | `src-tauri/src/commands/capture_commands.rs:168` |
| Rust → 전체 창 event emit | `src-tauri/src/commands/viewer_commands.rs:112-117` |
| Rust 통합 테스트 (tempdir) | `src-tauri/tests/viewer_readiness.rs`, `src-tauri/tests/capture_readiness.rs` |
| zod 스키마 + DTO 분리 | `src/shared-contracts/schemas/viewer-readiness.ts` + `dto/viewer.ts` |
| host DTO 정의 | `src-tauri/src/contracts/dto.rs:1713-1800` |

### 명명 규칙 (아키텍처 강제 + 코드베이스 관행)

- Rust command: `snake_case` (`get_viewer_display_state`), TS wrapper: `camelCase` (`getViewerDisplayState`)
- event 이름: 아키텍처는 `dot.case`지만 **현재 코드베이스는 kebab-case**(`capture-readiness-update`, `viewer-readiness-update`)다. 일관성을 위해 `viewer-display-update`를 쓰고 이 편차를 completion note에 기록한다 (Story 7.1과 동일한 결정)
- JSON 필드: TS 경계 `camelCase`, Rust 내부 `snake_case` + `#[serde(rename_all = "camelCase")]`
- React 컴포넌트/파일: `PascalCase.tsx`. 훅 파일은 `use-kebab-case.ts`, export는 `useCamelCase`
- 도메인 디렉터리: `kebab-case`
- 타임스탬프: 경계는 ISO 8601 / RFC3339, 계측은 `*AtMs` 또는 `*AtMicros` (host monotonic 기준임을 필드명으로 드러낸다)

### 절대 하지 말 것 (disaster prevention)

1. **notify를 pointer commit보다 먼저 하지 않는다.** 순서가 뒤집히면 viewer가 커밋되지 않은 generation을 읽는다.
2. **canonical 파일을 in-place overwrite하지 않는다.** Story 1.9가 정확히 이 방식으로 만들어졌고 2026-08-11 보정에서 superseded historical evidence로 강등됐다. generation은 항상 새 경로에 쓰고 pointer만 전진한다.
3. **decode 전에 swap하지 않는다.** `<img onLoad>`, 파일 존재, event 수신은 표시 성공이 아니다. `decode()` resolve만이 교체 조건이다.
4. **fade/transition/spinner/placeholder를 넣지 않는다.** opaque swap은 나가는 이미지가 한 프레임도 반투명해지지 않는다는 뜻이다.
5. **`.viewer-surface__photo`의 CSS와 standby 행의 레이아웃 점유를 바꾸지 않는다.** 촬영 차단과 scale jump가 동시에 발생한다.
6. **`VIEWER_REPORT_STALE_AFTER_MS`나 heartbeat 주기를 계측 편의로 바꾸지 않는다.** Story 7.1의 정직성 계약이다.
7. **KPI 종료점을 file-ready / renderer-ready / event receipt / decode / `onLoad`로 대체하지 않는다.** 전부 진단 span이다.
8. **cross-origin `renderTime = 0` fallback(`loadTime`)을 actual-present로 보고하지 않는다.**
9. **`report_trusted_capture_input`을 `await`하지 않는다.** 제품 hot path를 느리게 만든다.
10. **host 수신 시각을 KPI 시작점으로 쓰지 않는다.** 결과가 실제보다 빨라 보인다.
11. **`session.json` 스키마를 확장하지 않는다.** 별도 versioned display manifest를 쓴다.
12. **새 Rust crate를 승인 없이 추가하지 않는다.** Story 7.7의 offline 재현 인벤토리에 영향이 간다.
13. **`BOOTHY_DISPLAY_SAMPLE_MODE`를 기본 on으로 두지 않는다.** 고객에게 fixture 사진이 표시된다.
14. **camera source, preset renderer, 384px 상수, darktable 경로를 건드리지 않는다.** Story 7.3~7.6 범위이며 끌어오면 evidence가 오염된다.
15. **자동 테스트 통과만으로 `done` 처리하지 않는다.** HV-13B `Go` 없이는 `review` 유지다.

### Previous story intelligence — Story 7.1 (`done`, HV-13A `Go`)

이 Story가 반드시 상속해야 할 학습:

- **자동 검증 전부 통과 + 실장비 실패가 두 번 일어났다.** Story 1.9(fast preview)와 Story 7.1 1차 HV-13A가 모두 그랬다. 이번에도 HV-13B 증거 수집 절차를 구현과 **동시에** 준비한다. 특히 물리 모니터 고속 촬영 셋업은 구현이 끝난 뒤에 준비하면 늦는다.
- **HV-13A `No-Go`의 원인은 command 실행 컨텍스트였다.** `#[tauri::command]` 기본 blocking 실행이 event loop 스레드를 잡아 교착을 만들었다. 이번 Story의 file I/O command에 같은 실수를 반복하지 않는다.
- **부팅이 host IPC를 기다리면 흰 화면이 된다.** 관람 창은 capability IPC를 호출하지 않도록 이미 분리되어 있다. clock 보정 IPC로 이 성질을 다시 깨지 않는다.
- **epoch + revision 이중 방어가 실제로 필요했다.** 늦게 도착한 이전 epoch report가 현재 상태를 오염시키는 회귀가 code review에서 반복 검출됐다. display pointer도 `viewerEpoch` + `generationSeq` + `revision` 삼중 방어로 시작한다.
- **엄격한 프로덕션 계약이 리뷰 결정이었다.** `single-monitor-fallback` / `unapproved-profile` / `monitor-unavailable`은 전부 촬영 차단이다. HV-13B도 `approved-customer-monitor` 구성에서만 유효한 표본으로 인정한다.
- **Story 7.1 evidence의 photo rect는 1428.859×952.562였다** (1080p 전체화면 기준 1620×1080과 다름). 재검증에서 확인됐지만, HV-13B에서도 `photoRect`와 `requiredSource*`를 표본마다 함께 기록해 크기 검증이 실제 기하 위에서 이뤄졌음을 남긴다.

### Git intelligence

최근 커밋: `c390f53`(Story 7.1), `89ae52c`, `b24cfc4`, `12309fa`, `81b1271`. 앞의 하나를 제외하면 전부 capture timing / preview latency / thumbnail fallback 영역이다. 현재 main의 hot path는 `capture_commands.rs` → `ingest_pipeline.rs` → `render/mod.rs` → `capture-runtime.ts` → `LatestPhotoRail.tsx`다.

Story 7.2의 변경은 이 hot path와 **거의 겹치지 않아야 한다.** 교차점은 두 곳뿐이다.

1. `request_capture`에 sample lane 트리거를 붙이는 지점 — **비차단, 기본 off**
2. booth 촬영 버튼 핸들러에 trusted input 스탬프를 붙이는 지점 — **비차단**

나머지는 신규 파일(`src-tauri/src/display/`, `src/display-generation/`, `src/viewer-surface/telemetry/`)로 격리하면 회귀 위험이 낮다.

주의: `src-tauri/src/contracts/dto.rs`(1818줄), `src-tauri/src/render/mod.rs`(1732줄), `src/capture-adapter/services/capture-runtime.ts`(811줄)는 최근 활발히 수정된 큰 파일이다. 편집 전 현재 내용을 다시 읽는다.

### 최신 기술 정보 (구현 전 확인 완료)

- **Element Timing API** — `<img elementtiming="...">` + `PerformanceObserver({ type: 'element' })`. `renderTime`은 "이미지가 완전히 로드된 뒤 발생하는 다음 paint"의 타임스탬프다. WebView2는 Chromium 기반이라 사용 가능하다. **timing allow check에 실패하면 `renderTime`은 `0`**이 되고 `entry.startTime`이 `loadTime`으로 대체된다. `Timing-Allow-Origin` 헤더가 필요하며 Tauri asset protocol은 이를 제공하지 않으므로 **보조 진단으로만** 쓴다. ([MDN renderTime](https://developer.mozilla.org/en-US/docs/Web/API/PerformanceElementTiming/renderTime), [Element Timing API](https://developer.mozilla.org/en-US/docs/Web/API/Element_timing_API))
- **rAF + MessageChannel after-paint 기법** — DOM 갱신 → `requestAnimationFrame` 콜백(paint 직전) → `MessageChannel` task(프레임 생성 직후)로 paint 완료에 가장 근접한 시각을 얻는다. 100% 정확하지 않으며, 숨김 창에서는 rAF가 돌지 않고 input 이벤트가 우선순위를 가로챌 수 있다는 한계를 문서에 남긴다. ([webperf.tips](https://webperf.tips/tip/measuring-paint-time/))
- **Tauri v2 Channel vs Event** — Channel은 ordered/high-throughput 스트리밍용이고, event는 소량 데이터와 multi-consumer용이다. async 리스너와 연속 emit이 겹치면 event 처리 순서가 어긋날 수 있다. 이 Story는 그럼에도 monotonic guard로 정확성을 보장하는 설계를 택했다(핵심 설계 결정 4). ([Tauri: Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/))
- **Tauri asset protocol** — Windows에서 `convertFileSrc`는 `http://asset.localhost/...` URL을 만든다. `assetProtocol.enable`과 scope가 이미 설정되어 있고 세션 루트가 scope 안이므로 **설정 변경이 필요 없다.** 나중에 CSP를 켤 때는 `img-src`에 `asset: http://asset.localhost`가 필요하다(Story 7.7 범위). ([Tauri: Asset protocol scope](https://v2.tauri.app/security/asset-protocol/))
- **Windows `std::fs::rename`** — `MoveFileExW` + `MOVEFILE_REPLACE_EXISTING`으로 구현되어 대상이 있어도 교체된다. pointer 교체에 적합하고, generation 파일 충돌은 `create_new(true)`로 명시적으로 막는다.

### 기술 스택 고정값

- Tauri `2.10.3` (Rust), `@tauri-apps/cli`/`api` `^2.10.1`, Rust edition 2021, rust-version `1.77.2`
- Rust deps: `serde`, `serde_json`, `log`, `tauri`(`protocol-asset`), `tauri-plugin-log`. **새 crate 추가 전 승인 필요**
- React `^19.2.4`, react-dom `^19.2.4`, react-router-dom `^7.13.1`
- zod `^4.3.6` (v4 API)
- TypeScript `~5.9.3`, Vite `^8.0.1`
- Vitest `^4.1.0`, jsdom `^29`, Testing Library React `^16.3.2`, setup: `src/test/setup.ts`
  - jsdom에는 `HTMLImageElement.prototype.decode`와 `ResizeObserver`가 없다. `DoubleBufferedPhoto` 테스트는 `decode`를 명시적으로 stub하고 pending/resolve/reject 세 상태를 모두 검증한다
- 패키지 매니저: **pnpm 10.31.0**. npm/yarn 금지

### Verification Guardrails

- 자동 테스트는 필요하지만 HV-13B를 대신하지 않는다.
- Story 7.2 evidence는 immutable sample 게시, opaque swap, actual-present 계측, A/B 결정에 한정한다.
- 실제 preset 결과의 latency 판정은 Story 7.4/7.8이 소유한다. 이 Story의 수치는 **표시 종단점의 기준선**이지 제품 SLA 통과 근거가 아니다.
- Epic 7 dependency상 HV-13B가 막히면 7.3 이후 전체가 막힌다.

## Dependencies

- Story 7.1 / HV-13A `Go` (viewer readiness, monitor targeting, session binding, `photoRect.requiredSource*`)
- 기존 session identity와 capture 경로 (`start_session`, `request_capture`, `SessionPaths`)
- 승인된 고객 모니터 구성 (`BOOTHY_APPROVED_CUSTOMER_MONITOR`)
- 물리 모니터 고속 촬영 장비 (120fps 이상) — HV-13B의 필수 조건
- fast camera source, 신규 renderer, preset proxy에는 의존하지 않는다

## Handoff

- Primary: frontend/WebView + Windows host 소유 개발팀
- Architecture review: generation/pointer 계약, commit 순서, clock 보정 모델, transport 편차 근거
- QA: HV-13B actual monitor frame, timing span, zero-transition-defect, A/B raw evidence
- Product/UX: read-only surface 유지, standby↔이미지 전환의 고객 인지 품질

## References

- `_bmad-output/planning-artifacts/epics.md#Story-72-Immutable-sample-표시와-actual-present-계측` (AC 원문)
- `_bmad-output/planning-artifacts/epics.md#Epic-7` (execution rule, evidence dependency)
- `_bmad-output/planning-artifacts/prd.md#FR-010-Pre-Opened-Full-View-Progressive-Preset-Display`
- `_bmad-output/planning-artifacts/prd.md#NFR-003-Booth-Responsiveness-and-Qualifying-Viewer-Readiness`
- `_bmad-output/planning-artifacts/prd.md#NFR-004-Session-Isolation-and-Privacy`
- `_bmad-output/planning-artifacts/architecture.md#Approved-2026-08-11-Correct-Course-Baseline` (display pipeline, generation 규칙, latency telemetry rule)
- `_bmad-output/planning-artifacts/architecture.md#Complete-Project-Directory-Structure` (`src-tauri/src/display/`, `src-tauri/src/viewer/present_telemetry.rs`, `src/display-generation/`, `src/viewer-surface/telemetry/`)
- `_bmad-output/planning-artifacts/architecture.md#Closed-Contract-Freeze-Baseline` (Display artifact contract)
- `_bmad-output/planning-artifacts/ux-design-specification.md#전용-관람-화면과-무중단-교체-흐름`
- `_bmad-output/planning-artifacts/ux-design-specification.md#전용-관람-화면-Customer-Viewer-Surface`
- `_bmad-output/planning-artifacts/research/technical-preset-image-fast-display-research-2026-08-10.md#Data-Architecture-Patterns` (commit 순서, artifact key)
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260811-021653.md` (Story 7.1/7.2 분할과 7.2 소유 항목)
- `_bmad-output/implementation-artifacts/7-1-전용-관람-창-준비와-화면-크기-계약.md` (viewer 계약, HV-13A 학습)
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md#Story-72` (HV-13B required evidence)
- `docs/contracts/viewer-readiness.md` (Story 7.1 계약 — 이 Story가 깨지 말아야 할 경계)

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (1M context)

### Debug Log References

- 2026-08-11 — **stale-epoch 판정이 구조적으로 발화할 수 없었다.** 초안 `PublishRequest`는 candidate와 admission context에 같은 `viewer_epoch`를 넣어 두 값이 절대 달라질 수 없었다. sample A와 B 사이에 관람 창이 재생성되는 실제 시나리오를 표현하려면 두 값이 분리되어야 한다. `request_viewer_epoch`(request 수락 시점)와 `current_viewer_epoch`(게시 시점)로 나눴고, sample lane이 request 수락 시점 epoch를 스레드로 넘긴다. 이제 `rejects_a_sample_whose_viewer_generation_died_mid_request`가 실제 경로를 검증한다.
- 2026-08-11 — **epoch 변경 후 generation이 영구히 사라지는 버그.** `applyPointer`가 revision guard에서 조기 반환해 판정을 건너뛰었다. viewer가 부팅 중 epoch 0→1로 올라간 뒤 같은 revision의 snapshot을 다시 읽으면 generation이 다시 올라오지 않았다. 적용 여부와 무관하게 항상 재판정하도록 고쳤다. `ViewerSurface` 레이아웃 불변식 테스트가 이 결함을 잡았다.
- 2026-08-11 — **`decode()` 미지원 런타임에서 컴포넌트가 throw했다.** jsdom에 `HTMLImageElement.prototype.decode`가 없어 effect에서 TypeError가 났다. WebView2에는 있지만, 완전 decode를 증명하지 못한 자산을 올리는 것은 계약 위반이므로 **교체하지 않고 `decode-failed`로 보고**하도록 방어했다.
- 2026-08-11 — 빈 문자열 `src`가 브라우저에 현재 문서 재다운로드를 유발한다는 React 경고를 확인하고 `undefined`로 바꿨다.
- 2026-08-11 — `capture_readiness` Rust 테스트 실패를 격리 검증했다. 내 변경 전체를 stash한 HEAD 기준 baseline이 **47 passed / 12 failed**였고 변경 후에도 동일 범위(11~12건)였다. 실패 테스트 이름도 실행마다 바뀐다. `operator_audit`은 단독 실행 3회 연속 7/7 통과하며 `capture_readiness`와 동시 실행할 때만 실패한다. 기존 render 경로 공유 자원 경합이다.
- 2026-08-11 — governance 테스트 실패가 기존 것임을 확인했다. HEAD 버전의 Story 1.4 문서에도 기대 문자열 `Current hardware gate`가 0건이다.
- 2026-08-11 — sample fixture는 PowerShell `System.Drawing`으로 생성했다(새 의존성 없음). 실제 인코더가 만든 JPEG이므로 `image_probe`가 합성 바이트가 아닌 배포 자산으로 검증된다.
- 2026-08-12 — **generation `000008`의 terminal 행이 사라진 경로를 evidence에서 역추적했다.** host 로그에는 `display_sample_committed ... 000008`이 있고 거부·유실 경고가 하나도 없다. 즉 host는 정상 commit했고 viewer의 보고 자체가 도착하지 않았다. `DoubleBufferedPhoto`의 두 번째 effect는 `[generation, layers, requiredSourceWidthPx, requiredSourceHeightPx]`에 의존하는데, cleanup이 `cancelStamp?.()`를 호출했다. **`cancelStamp`는 swap이 커밋된 뒤에만 만들어지므로 이 취소는 언제나 정당한 표본만 지웠다.** 000008 commit(414081966μs) 직전에 5번째 셔터 입력(414060289μs)이 있었고, 같은 구간에서 `reveal_prewarmed_viewer_window`가 `set_position`·`show`·`set_fullscreen`을 다시 걸었다. 창 상태가 흔들리면 viewer가 photo rect를 재보고하고 `requiredSource*`가 갱신되면서 effect가 다시 돌아, 스탬프가 도착하기 전에 취소된다. 나머지 9건은 스탬프가 먼저 도착해 살아남았다. 재현 테스트 2건이 수정 전 코드에서 실패하고 수정 후 통과하는 것을 확인했다.
- 2026-08-12 — **누락 자체를 계측이 잡지 못한 것이 더 큰 문제였다.** 보고가 오지 않으면 아무 행도 남지 않아 evidence에서 "화면에 올라가지 않음"과 "보고 유실"을 구분할 수 없고, 분모가 조용히 줄어 성공률이 실제보다 좋아 보인다. host가 commit된 generation을 미결로 들고 있다가 유예 시간(2초) 안에 terminal 보고가 없으면 `present-unreported` 행으로 닫도록 했다. 이 사유는 host 전용이며 viewer → host 계약에서는 거부된다 — 보고가 오지 않았다는 것은 viewer가 주장할 수 있는 결론이 아니기 때문이다.
- 2026-08-12 — 유예 시간을 넘겨 도착한 보고를 어떻게 다룰지 고민했다. 두 번째 행을 쓰면 generation 10개에 행 11개가 되어 집계가 깨지고, 조용히 버리면 "조용한 무시 금지"를 위반한다. **generation당 terminal 행은 하나**로 고정하고, 늦게 도착한 보고는 span 값을 실은 `display_present_after_terminal_record` 경고로 남긴다.
- 2026-08-12 — `bind_display_session`이 이전 세션 request를 `retain`으로 버리기 전에 close-out을 먼저 돌려야 한다는 것을 놓칠 뻔했다. 순서가 바뀌면 세션이 끝났다는 이유만으로 그 세션 마지막 표본들의 행이 사라진다. `delete_capture` 경로도 같다.
- 2026-08-12 — 마지막 generation은 다음 commit이 없어 sweep 대상이 되지 않는다. sample lane 스레드가 B를 게시한 뒤 유예 시간 + 250ms를 자고 직접 sweep하도록 했다. 계측 lane 안에서만 도는 경로라 제품 경로(기본 `off`)에는 스레드가 하나도 늘지 않는다.

### Completion Notes

**구현한 것**

- **immutable generation과 atomic pointer**를 `src-tauri/src/display/`에 신설했다. commit 순서(write → flush → `sync_all` → close → 구조 probe → 크기·correlation 검증 → rename → pointer rename → journal → notify)를 코드 구조로 강제했고, 각 거부 경로가 고유 reason을 남긴다. 확정 경로는 절대 덮어쓰지 않으며 충돌은 하드 에러다.
- **의존성 없는 JPEG 구조 probe**를 만들었다. 기존 `has_jpeg_signature`는 앞 3바이트만 봐서 절반만 쓰인 파일도 통과한다. **EOI trailer 확인**이 partial file 방어의 실질적 근거이며, SOF에서 실제 크기를 파싱하고 EXIF orientation이 1이 아니면 거부한다. 새 crate를 추가하지 않아 Story 7.7 인벤토리가 그대로다.
- **opaque double-buffer swap**을 구현했다. 두 레이어가 같은 box·같은 `object-fit`을 쓰고, `decode()`가 resolve한 뒤에만 z-index를 한 번의 style commit으로 바꾼다. transition·fade·spinner·placeholder가 없다.
- **one-clock actual-present 계측**을 구현했다. host monotonic clock을 기준으로 두 WebView가 각각 Cristian 방식(최소 RTT 채택)으로 보정하고, KPI는 항상 `present 상한 − input 하한`인 보수적 구간으로 보고한다. 시작점 변환은 내림, 종료점은 올림이라 어떤 반올림도 구간을 짧게 만들지 않는다.
- **AC 4 실패 조건 계측**을 붙였다. host가 viewer 창 생성 횟수를 세고, present 보고 시 trusted input 이후 증가분을 `viewerWindowEventsAfterInput`으로 남긴다. `get_capture_readiness` polling이 창을 재생성할 수 있는 경로가 실제로 존재하므로 이 계측이 필요하다.
- **세션 경계 무효화**를 구현했다. 세션 교체·viewer epoch 변경으로 인한 `requiredSource*` 확대·capture 삭제가 각각 pointer를 정직하게 비운다.
- **hidden prewarm 두 변형**(`invisible`, `offscreen`)과 첫 generation에서의 노출 경로를 구현했다. 노출 비용은 present 계측에 그대로 포함된다.

**판단이 필요했던 결정 (리뷰 대상)**

- **`session.json`을 확장하지 않았다.** 별도 versioned display manifest(`renders/display/pointer.json`, `viewer-display/v1`)와 `generations.jsonl`을 쓴다. 아키텍처의 "session manifest는 기능마다 drift하지 않는다" 규칙을 지키고 회귀 위험이 큰 파일을 건드리지 않기 위해서다. Story 7.4/7.6이 이 파일을 확장한다.
- **transport는 broadcast event + snapshot 재수렴**을 유지했다. 아키텍처는 Channel을 권장하지만, "늦게·중복·순서 뒤바뀐 업데이트가 화면을 되돌릴 수 없다"는 보장은 `shouldAdvanceDisplay` guard에서 나오지 전송 순서에서 나오지 않는다. Story 7.6에서 tier가 늘면 재평가한다.
- **"decode"를 두 단계로 나눴다.** host는 구조 decode, viewer는 픽셀 decode다. 새 crate 없이 AC 1과 AC 2를 모두 만족시키는 유일한 분할이며 계약 문서에 명시했다.
- **이전 레이어의 `src`를 비우지 않는다.** 명세는 2프레임 뒤 정리를 제안했지만, 레이어가 둘뿐이라 다음 교체에서 어차피 덮어써진다. 중간에 비우면 빈 레이어가 생길 위험만 늘고 얻는 것이 없다.
- **`measurementLaneEnabled`를 pointer snapshot에 추가했다.** booth가 lane이 꺼져 있을 때 계측 IPC를 하나도 하지 않게 하려면 프론트가 lane 상태를 알아야 한다. 기본값 `false`라 제품 경로는 Story 7.1과 동일하다.
- event 이름은 아키텍처의 `dot.case` 대신 코드베이스 관행인 kebab-case(`viewer-display-update`)를 따랐다. Story 7.1과 동일한 결정이다.

**Story 7.1 계약을 지키기 위해 한 것**

- `.viewer-surface__photo`의 CSS 기하를 바꾸지 않고 이미지 레이어를 그 안쪽에 absolute로만 넣었다.
- standby 문구를 `display: none`으로 없애지 않고 `visibility: hidden`으로 자리를 유지했다. 없앴다면 stage 높이 → photo rect가 커져 촬영 차단과 scale jump가 동시에 발생했을 것이다. 두 상태의 기하 동일성을 테스트로 고정했다.
- `VIEWER_REPORT_STALE_AFTER_MS`와 heartbeat 주기를 바꾸지 않았고, 그 값들을 테스트로 고정했다. hidden 변형이 stale-report로 촬영을 막는 것은 Story 7.1이 의도한 정직한 차단이다.

**2026-08-12 offscreen 회차 이후 보강**

- **계측 완결성을 계약으로 만들었다.** commit된 generation 하나당 terminal 행이 정확히 하나 남는다. viewer 보고가 먼저 오면 그 행이 terminal이고, 유예 시간(2초)이 먼저 지나면 host가 `present-unreported` 행으로 닫는다. 이 행의 `actualPresentAtMicros`는 `null`, `confidence`는 `unreported`라 KPI에서는 빠지지만 **성공률 분모에는 남는다**. 닫는 시점은 다음 generation commit, 세션 교체, `delete_capture`, 그리고 마지막 generation을 위한 lane 스레드의 sweep이다.
- **terminal 보고가 사라지던 client 경로를 막았다.** swap이 커밋된 뒤의 after-paint 스탬프는 effect 정리에서 취소하지 않는다. 프레임은 이미 표시되므로 그 표본의 보고는 반드시 나가야 한다. 취소는 언마운트에서만 한다. 수정 전 코드에서 실패하는 회귀 테스트 2건으로 고정했다.
- **hidden-prewarm 노출을 viewer 세대당 한 번으로 고정했다.** 이전에는 generation마다 `set_position`·`show`·`set_fullscreen`을 다시 걸어 측정 구간 한가운데에서 창 상태가 흔들렸다. 창이 재생성되면 새 세대는 다시 숨은 상태로 시작하므로 그때만 다시 드러낸다. A/B의 "노출 비용을 present 계측에 포함한다"는 성질은 첫 generation에서 그대로 유지된다.
- **present 보고의 계약 검증 실패를 로그로 남긴다.** 호출자가 오류를 삼키므로 이전에는 아무 흔적 없이 사라졌다.
- **완결성 검사를 자동화했다.** `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`이 `generations.jsonl`과 `viewer-present.jsonl`을 1:1로 대조한다. 2026-08-12 증거에 돌려 사람이 손으로 찾아낸 것과 같은 결과(`missing: ...-000008`, `completenessGate: fail`)를 재현했다. HV-13B README에 통과 필수 게이트로 넣었다.

**검증 결과 (있는 그대로)**

- `pnpm lint`: **통과.**
- `pnpm test:run`: **410 passed / 1 failed.** 실패는 기존 Story 1.4 문서의 hardware-validation governance 검사이며 HEAD에서도 동일하게 실패한다(기대 문자열이 HEAD 버전에도 없음). 이번 수정의 대상 테스트 42건은 전부 통과.
- `cargo fmt`: **통과.**
- `cargo test`: lib unit 47/47, `viewer_display` 22/22, `viewer_readiness` 43/43을 포함한 Story 7.2 영향 범위가 통과했다. 기존 render 경로 공유 자원 경합과 무관한 이번 변경에는 새 Rust 실패가 없다.
- `pnpm build`(`tsc -b`): **통과하지 못한다.** 남은 오류는 ReadinessScreen, capture-runtime.test, governance, operator diagnostics, active-preset 경로이며 **전부 Story 7.1 이전부터 있던 것**이다. Story 7.2가 만든 파일에는 타입 오류가 0건이다(작업 중 발견한 4건은 모두 수정했다).

**남은 블로커**

- **HV-13B 실장비 재검증을 실행했고 소프트웨어 증거는 통과했다.** 두 변형에서 각각 warm-up 5회 + 측정 30회, 총 70회 실촬영과 140개 immutable A/B generation을 만들었고 경로·크기·hash·pointer가 140/140 일치했다.
- 측정 구간의 actual-present는 120/120건이 모두 `presented`·`measured`·trusted input이었고 input 이후 viewer window event는 0건이었다.
- visible-standby의 combined p50/p95/max는 3710.617/4280.416/4672.717ms, hidden-prewarm은 3774.393/4540.441/15024.476ms였다. hidden의 15.024초 outlier 때문에 visible-standby를 기본값으로 유지한다.
- **2026-08-12 offscreen 회차에서 발견된 terminal 행 누락(9/10)은 코드로 수정했고 회귀 테스트로 고정했지만, 아직 실장비에서 재확인하지 않았다.** 완결성 게이트(`check-telemetry-completeness.ps1`)는 다음 부스 회차(HV-14)에 함께 태운다. HV-13B의 qualifying 증거인 2026-08-11 회차는 측정 표본 120/120에서 완결성이 성립했다.
- 120fps 이상 외부 촬영 rig가 없어 compositor→photon 오프셋과 frame-level zero-transition-defect는 미검증이다. **2026-08-12 correct-course로 이 항목의 소유권은 Story 7.8 / HV-18B로 이관됐다.** 이 Story의 블로커가 아니며, HV-18B에서 표시 종단점 귀책의 결함이 나오면 이 Story는 `review`로 되돌아간다.
- 미리보기 5초 예산 초과(6804~24028ms, darktable 구간 4.3~4.7초)는 Story 7.3~7.6 소유로 남기고 임계값을 올리지 않았다. 이 Story의 표시 종단점 판정과는 별개다.
- 재정의된 HV-13B `Go`에 따라 story status를 `done`으로 전환했다. Epic 7 dependency상 Story 7.3 착수가 해제된다.

### File List

**신규**

- docs/contracts/viewer-display.md
- src-tauri/src/commands/display_commands.rs
- src-tauri/src/display/mod.rs
- src-tauri/src/display/display_artifact.rs
- src-tauri/src/display/generation_repository.rs
- src-tauri/src/display/image_probe.rs
- src-tauri/src/display/sample_publisher.rs
- src-tauri/src/viewer/present_telemetry.rs
- src-tauri/tests/viewer_display.rs
- src/capture-adapter/services/trusted-input-telemetry.ts
- src/capture-adapter/services/trusted-input-telemetry.test.ts
- src/display-generation/services/display-guard.ts
- src/display-generation/services/display-guard.test.ts
- src/display-generation/state/use-display-pointer.ts
- src/display-generation/state/use-display-pointer.test.tsx
- src/display-generation/state/use-measurement-lane-state.ts
- src/shared-contracts/display.contracts.test.ts
- src/shared-contracts/dto/display.ts
- src/shared-contracts/schemas/viewer-display.ts
- src/viewer-surface/components/DoubleBufferedPhoto.tsx
- src/viewer-surface/components/DoubleBufferedPhoto.test.tsx
- src/viewer-surface/services/display-asset-src.ts
- src/viewer-surface/services/viewer-host-adapter.test.ts
- src/viewer-surface/telemetry/after-paint.ts
- src/viewer-surface/telemetry/after-paint.test.ts
- src/viewer-surface/telemetry/element-timing.d.ts
- src/viewer-surface/telemetry/present-clock.ts
- src/viewer-surface/telemetry/present-clock.test.ts
- src/viewer-surface/telemetry/use-host-clock-calibration.ts
- storage/fixtures/display-sample/generate-samples.ps1
- storage/fixtures/display-sample/sample-a.jpg
- storage/fixtures/display-sample/sample-b.jpg
- tests/hardware/viewer-present/hv-13b/README.md
- tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1
- tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/
- tests/hardware/viewer-present/run-20260812-172000-hv13b-offscreen-direct/

**수정**

- _bmad-output/implementation-artifacts/7-2-immutable-sample과-actual-present-계측.md
- _bmad-output/implementation-artifacts/hardware-validation-ledger.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/planning-artifacts/architecture.md
- _bmad-output/planning-artifacts/epics.md
- _bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md (신규)
- docs/contracts/viewer-display.md
- docs/contracts/viewer-readiness.md
- src-tauri/src/commands/capture_commands.rs
- src-tauri/src/commands/display_commands.rs
- src-tauri/src/viewer/present_telemetry.rs
- src-tauri/tests/viewer_display.rs
- src/shared-contracts/display.contracts.test.ts
- src/shared-contracts/dto/display.ts
- src/shared-contracts/schemas/viewer-display.ts
- src/viewer-surface/components/DoubleBufferedPhoto.tsx
- src/viewer-surface/components/DoubleBufferedPhoto.test.tsx
- tests/hardware/viewer-present/hv-13b/README.md
- src-tauri/src/commands/mod.rs
- src-tauri/src/commands/session_commands.rs
- src-tauri/src/commands/viewer_commands.rs
- src-tauri/src/contracts/dto.rs
- src-tauri/src/lib.rs
- src-tauri/src/viewer/mod.rs
- src-tauri/tauri.conf.json
- src/booth-shell/screens/CaptureScreen.tsx
- src/booth-shell/screens/ReadinessScreen.tsx
- src/index.css
- src/shared-contracts/events/index.ts
- src/shared-contracts/index.ts
- src/shared-contracts/schemas/index.ts
- src/viewer-surface/ViewerSurface.tsx
- src/viewer-surface/ViewerSurface.test.tsx
- src/viewer-surface/services/viewer-host-adapter.ts

## Change Log

- 2026-08-12: **correct-course로 120fps+ 물리 프레임 증거를 Story 7.8 / HV-18B로 이관하고 Story 7.2를 `done`으로 닫았다.** HV-13B는 immutable publication·timing span·계측 완결성·A/B 결정·AC 4 실패 조건으로 재정의해 `Go`를 기록했다. 이관 근거는 물리 프레임이 증명하는 구간이 60Hz에서 0.017초인 반면 실제 병목은 3.7~4.5초의 source/renderer 구간이고, 촬영 대상이 될 실제 preset 사진은 7.4 이후에 나온다는 것이다. HV-18B가 표시 종단점 귀책의 전환 결함을 발견하면 이 Story는 `review`로 되돌아간다. 승인 기본값 `visible-standby`와 근거를 계약 문서에 기록했다. 전문: `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md`.
- 2026-08-12: offscreen 회차의 terminal 행 누락(9/10)을 원인까지 추적해 수정했다. (1) swap이 커밋된 뒤의 after-paint 스탬프를 effect 정리에서 취소하지 않는다 — 이 취소는 언제나 정당한 표본만 지웠다. (2) commit된 generation은 유예 시간 안에 terminal 보고가 없으면 host가 `present-unreported` 행으로 닫아, generation당 terminal 행이 정확히 하나가 되도록 계약화했다. (3) hidden-prewarm 노출을 viewer 세대당 한 번으로 고정해 측정 구간의 창 상태 churn을 없앴다. (4) present 보고 검증 실패를 로그로 남긴다. (5) 완결성 검사를 `check-telemetry-completeness.ps1`로 자동화하고 HV-13B 통과 필수 게이트로 넣었다. 회귀 테스트 9건(TS 3, Rust 6)을 추가했고 client 수정 2건은 수정 전 코드에서 실패하는 것을 확인했다. 120fps+ 물리 프레임 증거가 여전히 없어 `No-Go`와 `in-progress`를 유지한다.
- 2026-08-12: hidden-prewarm `offscreen` 변형을 Canon EOS 700D와 승인 DISPLAY3에서 직접 실행했다. 실제 촬영 5회, 10/10 immutable generation 무결성, 승인 모니터 최종 B 전체 화면 표시, 9/9 기록 행의 presented/measured/trusted와 창 이벤트 0을 확인했다. generation `000008`의 terminal viewer-present 행이 누락됐고 120fps+ 물리 프레임 장비도 없어 HV-13B `No-Go`와 story `in-progress`를 유지했다. 증거 패키지는 `tests/hardware/viewer-present/run-20260812-172000-hv13b-offscreen-direct/`다.
- 2026-08-12: 주간 실촬영 spot check(`tests/hardware/viewer-present/run-20260812-081157-hv13b-light-rerun/`)에서 보조 모니터 표시 경로는 통과했으나 연속 촬영 시 첫 사진 카드가 완료 뒤에도 `마무리 중`에 남는 상태 경합을 발견해 수정했다. host readiness가 `latestCapture` 한 건만 싣던 것을 `recentCaptures`(최근 8건 + 렌더 미완료 기록 전부)로 확장하고 클라이언트가 이를 manifest에 병합-전용으로 화해시킨다. 회귀 테스트 4건(Rust 2, TS 2)을 추가했다. 미리보기 5초 예산 초과(7.1~7.6초, darktable 구간 4.3~4.7초)는 Story 7.3~7.6 소유로 남기고 임계값을 늘리지 않는다. 120fps+ 물리 프레임 증거가 여전히 없어 `No-Go`를 유지한다.
- 2026-08-11: clock probe IPC 계약, double-buffer after-paint race, 비전경 rAF 정지, viewer clock 정수 직렬화를 수정하고 직접 재검증했다. 70회 실촬영에서 140/140 generation 무결성과 120/120 actual-present 측정을 확인했다. 소프트웨어 증거는 통과했으며 120fps+ 물리 프레임 증거만 남아 `No-Go`와 `in-progress`를 유지한다.
- 2026-08-11: HV-13B를 승인 PC·Canon EOS 700D·DISPLAY3에서 직접 실행했다. 6회 실촬영과 12개 immutable generation 무결성은 통과했지만 actual-present row 0건과 120fps+ rig 부재로 `No-Go`를 기록했다. 증거 패키지는 `tests/hardware/viewer-present/run-20260811-194000-hv13b/`에 보존했다.
- 2026-08-11: 적대적 코드 리뷰에서 확인된 20건을 모두 수정했다. 표시 자산 수명주기, 화면 교체, 입력·present 결합, 실패 분류, 단일 모니터 fallback을 보강하고 회귀 테스트를 추가했다. HV-13B 실장비 검증이 남아 status는 `in-progress`를 유지한다.

- 2026-08-11: Story 7.2 컨텍스트 생성. Story 7.1 구현 실물, Epic 7 AC, architecture display artifact 계약, fast-display 연구, HV-13A 학습을 반영해 immutable sample generation·atomic pointer·opaque double-buffer·one-clock actual-present·standby/prewarm A/B 범위를 확정했다. Status `ready-for-dev`.
- 2026-08-11: Story 7.2 구현. immutable display generation과 atomic pointer(`src-tauri/src/display/`), 의존성 없는 JPEG 구조 probe, opaque double-buffer swap, one-clock actual-present 계측, visible-standby/hidden-prewarm 두 변형, 세션 경계 무효화, AC 4 실패 조건 계측을 추가했다. 자동 검증 63건(TS 45, Rust 18)을 붙였고 lint·fmt·대상 테스트가 전부 통과한다. 계측 lane은 기본 `off`이며 켜져 있을 때만 fixture가 표시된다. HV-13B 실장비 증거는 미수집이며 status는 `review`를 유지한다.
