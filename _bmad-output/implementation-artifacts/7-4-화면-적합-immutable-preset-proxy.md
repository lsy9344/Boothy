# Story 7.4: 화면 적합 immutable preset proxy 생성과 관람 화면 게시

Status: done

Type: Product path (customer-visible)

Epic 7 dependency: `7.1 Go` → `7.2 Go` → `7.3 approved route decision` → **`7.4 Go`** → `7.5 adoption decision` → … → `7.10 final decision`

> ## ✅ 상류 운영 경로 결정 완료
>
> **2026-08-16 Noah Lee 공식 결정:** HV-14의 세 fast-source 후보는 `Technology No-Go`로 유지하고,
> Story 7.4 운영 경로는 **`raw-original + pinned darktable 5.4.1`**로 확정한다.
> 결정 기록: `_bmad-output/planning-artifacts/sprint-change-proposal-20260816-010151.md`.
>
> 이것이 이 Story에 주는 의미는 셋이다.
>
> 1. **HV-14 route 결정 선행 조건은 충족됐다.** fast-source 후보 자체가 `Go`가 된 것은 아니며,
>    승인된 정확 대체 경로를 사용한다.
> 2. **구현은 막히지 않는다.** 이 Story의 정확성 계약(게시 순서, immutability, correlation,
>    display fit, zero-wrong-frame)은 **source route와 무관**하다. 기본 source를
>    `raw-original`(오늘의 정확 경로)로 두고 route 결정이 승인되면 값 하나만 바꾸도록 만든다.
> 3. **속도는 이 Story의 합격 조건이 아니다.** AC 5가 명시한다 — one-shot renderer가 최종
>    latency gate를 못 넘어도 correctness와 actual-present span을 기록하면 된다.
>    실측 기준선은 display-fit 1920px darktable one-shot **CPU 약 3.53초 / OpenCL 약 4.99초**
>    (실제 pixelpipe 계산은 약 0.3초)다. 속도 해결은 7.5/7.6이 소유한다.

## Story

booth 고객으로서,
관람 화면의 **첫 성공 이미지가 승인된 사진 영역을 내가 고른 룩으로 가득 채우기를** 원한다.
그래야 늘어난 썸네일이나 보정되지 않은 임시 이미지가 아니라 **선명하고 정직한 프리셋 결과**를 본다.

## Scope Boundary

### In Scope

- display 계약에 **`displayFitPresetProxy` tier** 등록과 tier 순서 정의 (`sample` 위)
- **display proxy job**: 목표 크기를 viewer photo rectangle × DPR에서 가져오고, 승인된 sRGB/JPEG
  품질 프로필을 사용하며, 하나의 session / request / capture / preset identity+version /
  source hash / render profile / unique generation에 결속
- **proxy publication 호환성 metadata**를 published preset bundle에 추가:
  `proxyCompatible`, supported operations, versioned recipe, reference renderer/version, visual approval evidence
- 미지원·미승인 preset은 **정확한 RAW fallback**을 쓴다. 암묵적 근사 금지
- 게시 gate: 프로세스 종료 또는 job 완료 → 완전 decode → 물리 크기 충족 → provenance 일치 →
  **atomic immutable commit** → 그다음에야 notify. 부분 write와 in-place overwrite는 활성 truth가 될 수 없다
- 지연·중복·역순 갱신 화해: lower revision / stale generation / older capture / **different preset** /
  other session이 현재 화면을 대체할 수 없다
- Story 7.2의 actual-present 계측 재사용과 proxy 구간 진단 span 추가
- 계약 문서, 자동 검증, HV-15 실장비 evidence

### Explicitly Out of Scope

- **상주 display renderer** → Story 7.5. 이 Story는 기존 `darktable-cli` one-shot을 display-fit 크기로 쓴다
- **RAW 정밀본 tier(`rawRefinedDisplay`)와 deadline scheduler / stale work 취소** → Story 7.6
- **P0/P1/P2 우선순위 스케줄러** → Story 7.6. 이 Story는 **큐 대기 시간을 재기만** 한다
- installer / 100-shot / rollout / 최종 판정 → Story 7.7~7.10
- **`renders/previews/`의 384px booth 레일 preview 경로 변경** — 그대로 둔다.
  `RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*` 상수를 바꾸지 않는다
- Story 7.3의 source 비교 lane(`BOOTHY_SOURCE_COMPARE_MODE`)과 `source-comparison.jsonl` 계약 변경
- `session.json` 스키마 확장
- CSP 강화, `assetProtocol` scope 변경
- 새 Rust crate 추가 (승인 없이 금지 — Story 7.7 offline 인벤토리에 직접 영향)

## Acceptance Criteria

1. **Given** 승인된 display profile과 capture-bound published preset이 있을 때, **when** display proxy job이 생성되면, **then** 목표 크기가 viewer photo rectangle과 DPR에서 나와야 하고 승인된 sRGB/JPEG 품질 프로필을 사용해야 하며, job이 하나의 session·request·capture·preset identity/version·source hash·render profile·고유 generation에 결속되어야 한다.
2. **Given** published preset이 proxy lane 후보로 검토될 때, **when** publication 호환성을 평가하면, **then** artifact가 `proxyCompatible`, supported operations, versioned recipe, reference renderer/version, visual approval evidence를 기록해야 하고, **미지원 또는 미승인 preset은 암묵적 근사 대신 정확한 RAW fallback을 사용**해야 한다.
3. **Given** display-fit preset proxy가 완료되었을 때, **when** 관람 화면에 게시하면, **then** 프로세스가 종료했거나 상주 job이 완료되었고, 전체 이미지가 decode되고, 물리 크기가 충분하고, provenance가 일치하고, immutable generation이 **atomic하게 commit된 뒤에야** notify해야 하며, **부분 write된 파일이나 in-place overwrite는 절대 활성 display truth가 될 수 없다**.
4. **Given** 지연·중복·역순 갱신이 있을 때, **when** viewer가 channel 갱신을 최신 snapshot과 화해시키면, **then** 더 낮은 revision, stale generation, 더 오래된 capture, **다른 preset**, 다른 session은 현재 화면을 대체할 수 없어야 한다.
5. **Given** 기준 proxy 경로를 계측했을 때, **when** Story 7.4 closure를 검토하면, **then** **현재 one-shot renderer가 최종 latency gate를 넘지 못해도** correctness와 actual-present span이 기록되어야 하고, HV-15가 immutable publication·display fit·correlation·zero-wrong-frame으로 `Go`를 기록해야 한다.

## Tasks / Subtasks

### T1. Display 계약에 proxy tier와 provenance를 추가한다 (AC: 1, 3, 4) — **최우선**

> Story 7.2가 seam을 깨끗하게 남겨 뒀다. **generation / pointer / swap / telemetry 계약을 새로 만들지 않는다.**
> tier 하나와 provenance 블록만 추가한다.

- [x] `src/shared-contracts/schemas/viewer-display.ts`
  - `displayTierSchema` = `z.enum(['sample', 'displayFitPresetProxy'])`
  - `DISPLAY_TIER_ORDER` = `{ sample: 0, displayFitPresetProxy: 1 }`. 순서 비교 함수는 이미 있다
  - **`sampleVariant`를 삭제하지 않는다.** `z.enum(['a','b']).nullable()`로만 확장한다.
    기존 7.2 evidence(`generations.jsonl`, `viewer-present.jsonl`, HV-13B 패키지)가 이 필드를 읽는다
  - 신규 `displayProxyProvenanceSchema` (nullable, proxy tier에서만 존재):
    `presetId`, `presetVersion`, `proxyRecipeVersion`, `referenceRenderer`, `referenceRendererVersion`,
    `renderProfileId`, `outputColorSpace`, `jpegQuality`, `sourceRoute`, `sourceAssetHash`,
    `targetWidthPx`, `targetHeightPx`, `displayProfileId`, `devicePixelRatio`
  - **tier별 교차 필드 불변식을 `superRefine`으로 강제한다.** 값이 비어 있어 기본값으로 통과하는 경로를 만들지 않는다
    - `tier === 'sample'` → `sampleVariant` 필수, `proxyProvenance`는 `null`
    - `tier === 'displayFitPresetProxy'` → `sampleVariant`는 `null`, `proxyProvenance` 필수
- [x] 신규 거부 사유 2종을 `displayRejectReasonSchema`에 추가하고 **고유 코드**로 판정한다.
  기존 `older-request`로 뭉뚱그리지 않는다 — 원인이 다르면 회차 분석에서 구분되어야 한다
  - `preset-mismatch`: capture-bound preset identity 또는 version이 현재 활성 generation과 다르다
  - `older-capture`: 같은 request 안에서 더 오래된 capture의 generation이 늦게 도착했다
- [x] `schemaVersion`을 `viewer-display/v2`로 올린다. **읽기는 v1도 받는다.**
  - v1 `pointer.json` / journal 행은 `tier: 'sample'`, `proxyProvenance: null`로 정규화해 읽는다
  - 근거: Story 7.9가 pre-upgrade session 호환과 old generation pointer 복구를 요구한다.
    진행 중인 세션 위로 업그레이드가 떨어져도 관람 화면이 죽으면 안 된다
  - 이 정규화를 **테스트로 고정**한다. 계약 문서에도 남긴다
- [x] `src/shared-contracts/dto/display.ts`에 `z.infer` 별칭 추가
- [x] `src-tauri/src/contracts/dto.rs`
  - `DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY` 상수 추가, `display_tier_order`에 `Some(1)` 등록
    (`dto.rs:1844-1852`)
  - `DisplayGenerationDto`에 `proxy_provenance: Option<DisplayProxyProvenanceDto>` 추가,
    `sample_variant`를 `Option<String>`으로 확장
  - 신규 reject 상수 2개
  - **필드명·nullability·enum 문자열이 TS와 정확히 일치해야 한다.** 라운드트립 테스트로 고정
- [x] **확정 파일 경로의 `variant` 자리를 정한다.** `generation_path`는
  `<display_root>/<requestId>/<seq>-<variant>.jpg`를 만든다 (`generation_repository.rs:104`).
  `sampleVariant`가 `null`인 proxy는 **tier에서 파생한 고정 문자열 `proxy`**를 쓴다.
  **빈 문자열을 넘기지 않는다** — `000001-.jpg`가 만들어져 경로가 사람에게도 스크립트에게도 읽히지 않는다
- [x] **`measurementLaneEnabled`의 의미를 분리한다. 놓치면 HV-15에서 actual-present 행이 0건이 된다.**
  이 값은 지금 `current_sample_lane_mode().is_enabled()`에서 나오고,
  booth/viewer는 **이 값이 true일 때만 present 계측 IPC를 수행한다** (7.2의 리뷰 결정).
  proxy lane은 계측 lane이 아니라 제품 경로이므로 이 값이 false다. 그대로 두면 HV-13B 1차와 같은 실패 모드다
  - snapshot에 `presentTelemetryEnabled`를 추가하고 `sample lane on || proxy lane on`으로 계산한다
  - `measurementLaneEnabled`는 **"표시 중인 이미지가 fixture다"**라는 뜻으로만 남긴다.
    두 값이 서로 다른 질문에 답한다는 것을 계약 문서에 적는다
  - booth/viewer의 계측 gate를 `presentTelemetryEnabled`로 바꾸고,
    off/off · sample only · proxy only 세 조합의 동작을 테스트로 고정한다
- [x] `docs/contracts/viewer-display.md` 확장: proxy tier, provenance, v1→v2 읽기 규칙, 신규 거부 사유,
  `measurementLaneEnabled` ↔ `presentTelemetryEnabled` 구분
- [x] **`tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`이 v2 행에서도 통과하는지 확인한다.**
  스크립트가 `schemaVersion`이나 `sampleVariant`를 필수로 매칭하면 다음 회차가 통째로 FAIL한다

### T2. Proxy publication 호환성을 published preset bundle에 추가한다 (AC: 2)

> 이 AC는 렌더러 이야기가 아니라 **게시 경계 이야기**다. approximation을 숨기지 않는 것이 목적이다.

- [x] `published-preset-bundle/v1`에 **optional `proxyPublication` 블록**을 추가한다.
      **`schemaVersion`을 바꾸지 않는다** — `docs/contracts/preset-bundle.md`가 이미 확장 metadata를 허용한다
  - `proxyCompatible: boolean`
  - `supportedOperations: string[]` — 승인된 operation allowlist
  - `proxyRecipeVersion: string`, `proxyRecipePath: string` (bundle root 내부)
  - `referenceRenderer: 'darktable'`, `referenceRendererVersion: string`
  - `outputProfile: { colorSpace: 'sRGB', jpegQuality: 1..100, iccIntent }`
  - `visualApproval: { approvedAt, approvedBy, corpusPath, metrics, blindReview }`
- [x] **현재 구현은 source route와 무관하게 명시적 `proxyPublication`을 요구한다**
  - 세 기본 preset에는 수동 승인 블록이 게시되어 있고, Story 7.4 실장비 증거는
    `approvalBasis: visual-approval`로 기록됐다
  - `raw-original + pinned darktable`은 운영상 승인된 정확 경로지만, 구현이 자동으로
    `exact-reference-renderer` 자격을 만들어 내는 것으로 주장하지 않는다
  - fast source 또는 Story 7.5 상주 렌더러는 별도의 자동 지표와 사람 비교 검토를 통과해야 한다
  - 승인 블록이 없거나 유효하지 않으면 proxy만 강등하고 preview/final 로딩은 유지한다
- [x] 로더 규칙 (`src-tauri/src/preset/preset_bundle.rs`의 `load_published_preset_runtime_bundle` 확장)
  - 필드 하나라도 없거나, `proxyRecipePath`가 bundle root 밖을 가리키거나,
    `referenceRendererVersion != PINNED_DARKTABLE_VERSION`이면 **`proxyCompatible=false`로 강등하고 이유를 로그에 남긴다.**
    조용한 통과 금지, 그러나 **기존 preview/final 로딩은 실패시키지 않는다** (회귀 방지)
- [x] publish flow (`src/shared-contracts/schemas/preset-authoring.ts`, `src-tauri/src/preset/authoring_pipeline.rs`)
  - publish input에 `proxyPublication`이 **있으면 검증하고, 유효하지 않으면 publish를 거절**한다
  - **없으면 오늘과 동일하게 publish 성공**하고 `proxyCompatible=false`로 게시된다.
    Story 4.2/4.3은 아직 `review`다. 그 경로의 성공 조건을 이 Story가 바꾸지 않는다
- [x] visual approval 임계값을 계약 문서에 적는다 (연구 승인값):
      `SSIM ≥ 0.95`, `median ΔE00 ≤ 3`, `p95 ΔE00 ≤ 8`, `skin ROI median ≤ 3`,
      `proxy MTF50 ≥ RAW 정밀본의 80%`, `clipping 증가 ≤ 2%p`, `upscale 0`,
      blind review 5명 × 30 transition에서 이의 제기 `< 5%`
- [x] `docs/contracts/preset-bundle.md`, `docs/contracts/authoring-publication.md` 갱신

### T3. Display-fit 렌더 진입점을 만든다 (AC: 1)

> **384px 상수와 기존 preview/final 경로를 건드리지 않는다.** 새 진입점을 추가한다.

- [x] `src-tauri/src/render/mod.rs`에 `render_display_proxy_to_path_in_dir(...)` 추가
  - **목표 크기를 호출자가 준다.** 상수를 새로 만들지 않는다.
    `build_darktable_invocation_from_source`(`render/mod.rs:861`)에 explicit width/height cap을 받는
    경로를 추가하고, 기존 호출자는 오늘의 384 값을 그대로 넘기게 한다 (`preview_render_dimensions` 유지)
  - **`--upscale false`를 반드시 넣는다.** upscale은 PRD NFR-003의 zero 항목이다.
    source가 목표보다 작으면 결과가 작게 나오고, 그건 T4의 크기 gate에서 정직하게 거부된다
  - 색 공간은 recipe의 `outputProfile.colorSpace`를 `--icc-type`으로 고정한다
  - JPEG 품질은 core 설정으로 넘긴다. **구현 전에 `darktable-cli --help`와 생성된 `darktablerc`로
    실제 키 이름을 확인한다** — 키가 틀리면 조용히 무시되어 품질 프로필이 계약과 달라진다
  - `--hq false`, `--apply-custom-presets false`는 기존 preview와 동일하게 유지 (custom preset 오염 방지)
  - **worker root를 분리한다:** `.boothy-darktable/display-proxy/`.
    preview/final과 `configdir`·`library.db`를 공유하면 동시 실행이 서로를 막는다 (`render/mod.rs:874`)
  - 렌더 실패는 `RenderWorkerError`로 돌려주고 **RAW·preview·final truth를 건드리지 않는다**
- [x] **큐 대기를 재고 기록한다.** `acquire_render_queue_slot()`은 `MAX_IN_FLIGHT_RENDER_JOBS = 2`
      (`render/mod.rs:21`)로 preview/final과 슬롯을 공유한다. proxy가 final 뒤에서 기다리면 첫 화면이 늦는다
  - `proxyQueueWaitMicros`를 진단 span으로 남긴다
  - **여기서 우선순위 스케줄러를 만들지 않는다.** 이 관측이 Story 7.6의 입력이다
- [x] 렌더 산출물은 임시 경로에 만들고, 그 bytes를 **7.2의 `stage_generation`에 넘긴다.**
      commit 순서를 두 벌 만들지 않는다

### T4. Display proxy publisher를 만든다 (AC: 1, 3, 4)

- [x] `src-tauri/src/display/proxy_publisher.rs` 신규.
      **`sample_publisher.rs`를 지우거나 대체하지 않는다** — 7.2 evidence 재현 경로가 살아 있어야 한다.
      두 publisher 모두 `publish_generation_in_dir`(`display/sample_publisher.rs:150`)을 호출하도록
      `PublishRequest`를 tier + provenance를 싣도록 일반화한다
- [x] 게시 전 검증 순서를 **이 순서 그대로** 구현한다 (AC 3)
  1. darktable 프로세스가 **종료**했음을 확인한다 (`run_darktable_invocation`의 반환이 그 경계다)
  2. 산출 파일이 완전히 write·close·`sync_all`되었는지 확인한다
  3. `display::image_probe::probe_jpeg`로 구조 decode (SOI + SOF 크기 + EOI trailer + orientation)
  4. **크기 검증:** `width >= requiredSourceWidthPx && height >= requiredSourceHeightPx`
  5. **provenance 검증:** session / request / capture / presetId / presetVersion / sourceAssetHash가
     job 생성 시점 값과 일치
  6. atomic immutable commit (`create_new`, 충돌은 하드 에러) → pointer rename → journal append
  7. **그다음에야** `viewer-display-update` emit
- [x] **호환성 gate.** `proxyCompatible = false`이거나 recipe 검증 실패면 **proxy lane을 시작조차 하지 않는다.**
      기존 정확 RAW 경로만 남기고, 그 사실을 진단에 기록한다. 근사치를 조용히 만들지 않는다 (AC 2)
- [x] **source 선택.** HV-14 route 결정 전 기본값은 `raw-original`이다
  - route 결정이 승인되면 `capture-source` 계약(`src/shared-contracts/schemas/capture-source.ts`)의
    **accepted** candidate를 쓴다. `evaluate_source_candidate`가 이미 12개 거부 사유를 소유한다
  - **source의 실측 픽셀 크기가 `requiredSource*`보다 작으면 proxy를 만들지 않는다.**
    1600px Windows Shell 썸네일은 1080p profile은 통과할 수 있어도 4K profile은 통과하지 못한다.
    이 판정을 렌더 **전에** 하고, 이유(`insufficient-source-dimensions`)를 남긴다.
    렌더를 돌린 뒤 크기 gate에서 버리면 CPU만 태우고 화면은 그대로 비어 있다
  - **보정되지 않은 source를 그대로 게시하지 않는다.** `cameraSource`는 qualifying frame이 아니다 (FR-010)
- [x] lane 스위치 `BOOTHY_DISPLAY_PROXY_MODE = off | on`, HV-15 `Go` 이후 **기본 `on`**. 명시적 `off`와 알 수 없는 값은 전부 `off`
  - `BOOTHY_DISPLAY_SAMPLE_MODE`(7.2 fixture lane)와 **동시 활성화는 금지**한다.
    켜져 있으면 경고를 남기고 proxy lane을 우선한다. fixture와 실제 결과가 같은 pointer를 다투면
    evidence가 오염된다
  - **HV-15가 `Go`를 기록한 뒤 같은 Story 안에서 기본값을 `on`으로 전환하고, 그 기본값을 테스트로 고정한다.**
    검증 전에 기본 on으로 출시하지 않는다
- [x] **트리거 시점을 정확히 잡는다.** 7.2의 sample lane은 `request_capture` **수락 직후**에 걸린다.
      fixture는 입력이 필요 없기 때문이다. **proxy는 다르다** — source 파일과 capture record의
      preset 결속이 둘 다 있어야 시작할 수 있다
  - 트리거는 `src-tauri/src/capture/ingest_pipeline.rs`의 `persist_capture_in_dir` 완료 지점이다.
    이때 `captureId` · `activePresetId` · `activePresetVersion` · source 경로가 모두 확정된다
  - `spawn_sample_lane_publication`(`commands/display_commands.rs:415`)과 **같은 비차단 스레드 패턴**을 쓰고,
    **capture 경로의 반환을 지연시키지 않는다**
  - `request_viewer_epoch`는 **request 수락 시점**에 읽어 스레드로 넘긴다. 그 사이 관람 창이 재생성되면
    `stale-epoch`로 거부되어야 한다 (7.2가 이 분리로 결함 하나를 잡았다)
  - **`captureId`는 이미 shutter 전에 고정된다** (7.3이 helper의 할당 시점을 앞당겼다).
    JPEG가 RAW보다 먼저 도착해도 두 산출물이 같은 `captureId`를 공유한다
- [x] 렌더 중간 산출물은 **session root 안**에만 쓴다.
      `<session_root>/renders/display/.staging/`을 재사용하고, 세션 정리 경로가 함께 지우는지 확인한다.
      NFR-004는 0 tolerance다. `.boothy-darktable/display-proxy/`에는 **고객 이미지를 남기지 않는다**
      (darktable config/library 전용 scratch)
- [x] 표시 실패(호환성 미달, 크기 미달, decode 실패, 렌더 실패)는 **관람 화면에 절대 노출하지 않는다.**
      booth control surface의 wait/call guidance로만 투영한다

### T5. 지연·중복·역순 갱신 화해를 강화한다 (AC: 4)

> 현재 guard는 session / epoch / seq / request / tier / 크기만 본다. **preset은 보지 않는다.**
> 촬영 중 preset을 바꾸면(Story 2.3) 이전 preset의 늦은 proxy가 현재 화면을 덮을 수 있다.

- [x] host `evaluate_admission`(`display/display_artifact.rs:41`)에 추가
  - capture-bound `presetId` + `presetVersion`이 현재 활성 generation과 다르면 `preset-mismatch`
  - 같은 request 안에서 더 오래된 `captureId`의 generation이면 `older-capture`
- [x] viewer `shouldAdvanceDisplay`(`src/display-generation/services/display-guard.ts`)에 **같은 규칙**을 넣는다.
      한쪽에만 넣으면 pointer와 화면이 갈라진다
- [x] 세션 교체 / viewer epoch 변경 / `delete_capture` / preset 변경에서 proxy generation도 함께 정리한다.
      7.2의 `bind_display_session`, `forget_display_request`, `remove_request_generations` 경로를 재사용한다
- [x] `use-display-pointer.ts`의 snapshot 재수렴은 그대로 쓴다. **live event만으로 상태를 소유하지 않는다**

### T6. 계측을 확장한다 (AC: 5)

- [x] **`viewer-present.jsonl`과 span 계약을 그대로 재사용한다. 새 파일을 만들지 않는다.**
      행에 `tier`와 `proxyProvenance` 요약(presetId, presetVersion, sourceRoute, targetWxH)을 추가한다
- [x] 신규 **진단** span (어느 것도 KPI 종료점이 아니다)
  - `proxySourceReadyAtMicros`, `proxyQueueWaitMicros`, `proxyRenderStartAtMicros`,
    `proxyRenderEndAtMicros`, `proxyProcessExitedAtMicros`
- [x] KPI는 그대로 `trusted input → actual present`다. **5초 예산을 넘어도 그대로 기록한다.**
      임계값을 올리거나 표본을 제외하지 않는다 (AC 5)
- [x] **계측 완결성 규칙을 상속한다.** commit된 generation당 terminal 행 정확히 하나.
      Story 7.2가 이 결함으로 한 회차를 잃었다. `present-unreported` 마감 경로가 proxy generation에도 적용되는지 확인한다
- [x] **Story 7.3의 `source-comparison.jsonl`과 합쳐 집계하는 도구를 만들지 않는다.**
      합치는 순간 무보정 source의 빠른 도착이 preset-applied KPI를 실제보다 좋아 보이게 만든다

### T7. 자동 검증을 추가한다 (AC: 1~4)

- [x] `src-tauri/src/display/display_artifact.rs` 단위 테스트 — 거부 매트릭스를 **전부** 단언한다:
      `preset-mismatch`, `older-capture`, `tier-downgrade`(proxy → sample 역행), `insufficient-dimensions`,
      `stale-epoch`, `lower-generation`, `older-request`, `session-mismatch`
- [x] `src-tauri/src/display/proxy_publisher.rs` 단위 테스트 —
      호환성 미달 시 lane 미시작, source 크기 미달 시 **렌더 전 중단**, provenance 불일치 거부
- [x] `src-tauri/tests/viewer_display.rs` 확장 (기존 22건을 깨지 않는다)
  - proxy generation이 sample generation을 대체하지만 **역행은 거부**된다
  - 부분 write된 proxy 파일이 활성 truth가 되지 못한다
  - 같은 확정 경로에 두 번 쓰면 하드 에러
  - preset 변경 후 이전 preset의 늦은 proxy가 pointer를 전진시키지 못한다
  - `BOOTHY_DISPLAY_PROXY_MODE=off`에서 proxy 산출물이 하나도 생기지 않는다
      (**외부 환경 변수에 따라 건너뛰지 않는 결정적 테스트**로 만든다 — 7.3의 리뷰 지적사항이다)
  - proxy 확정 경로가 `<seq>-proxy.jpg`이고 빈 variant 경로가 생기지 않는다
  - `presentTelemetryEnabled`가 sample lane off + proxy lane on에서 **true**다
  - **v1 `pointer.json`을 읽어 v2로 정규화한다**
- [x] `src-tauri/src/render/mod.rs` 단위 테스트 —
      display-proxy invocation이 **호출자가 준 width/height**를 쓰고, `--upscale false`를 포함하고,
      worker root가 preview/final과 분리되며, **기존 preview invocation의 384 인자가 변하지 않았음**을 단언한다
- [x] `src/display-generation/services/display-guard.test.ts` 확장 — host와 동일한 거부 매트릭스
- [x] `src/viewer-surface/components/DoubleBufferedPhoto.test.tsx` — proxy asset 교체에서도
      두 레이어의 box·`object-fit`이 동일하고, decode 성공 전에는 교체되지 않는다
- [x] `src/shared-contracts/display.contracts.test.ts` 확장 — TS↔Rust 라운드트립,
      tier별 교차 필드 불변식(`sample`+provenance, `proxy`+sampleVariant 조합이 **파싱 실패**), v1 읽기
- [x] `src/shared-contracts/preset-*.contracts.test.ts` — `proxyPublication` 부재 시 `proxyCompatible=false`
- [x] Scope guard를 **grep으로 실제 확인**한다 (체크박스로 때우지 않는다):
      `rawRefinedDisplay`, `resident_renderer`, `deadline_scheduler`, `P0`/`P1` 우선순위 심볼이 변경 범위에 없음.
      `RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*` 상수 diff 없음.
      `src/shared-contracts/schemas/capture-source.ts`, `src-tauri/src/capture/source_*.rs` diff 없음.
      `session-manifest.ts` diff 없음
- [x] 검증 명령과 **있는 그대로의 결과**를 completion note에 기록한다:
      `pnpm lint`, `pnpm test:run`, `cargo fmt`, `cargo test`, `dotnet test`, `pnpm build`
  - 기존 사실: `pnpm build`(`tsc -b`)는 Story 7.1 이전부터 ReadinessScreen / capture-runtime /
    governance / operator diagnostics / active-preset 경로의 기존 오류로 실패한다 (직전 회차 18건 / 8파일).
    `capture_readiness` Rust 테스트는 **병렬 실행 경합**으로 flaky하며 `--test-threads=1`에서 60/60 통과한다.
    **이번 Story가 이 상태를 악화시키지 않았음**을 그 방법으로 확인하고 정직하게 적는다

### T8. HV-15 하드웨어 증거를 생산한다 (AC: 5)

> **HV-14 route 결정은 2026-08-16 충족됐다.** 남은 환경·화질 조건 없이는 이 게이트를 `Go`로 기록할 수 없다.

- [x] `tests/hardware/display-proxy/hv-15/README.md`와 `run-<timestamp>/`를 준비한다.
      `tests/hardware/viewer-present/`, `tests/hardware/capture-source/`와 섞지 않는다
- [x] 환경 fingerprint: 승인 PC, EOS 700D 펌웨어/렌즈/카드/케이블, EDSDK·helper 버전,
      **고객 모니터 모델/해상도/DPR/주사율과 승인 display profile**, WebView2 runtime, GPU/driver, ICC, HDR on/off,
      darktable 버전(pinned `5.4.1`), 앱 버전·커밋 해시, `BOOTHY_DISPLAY_PROXY_MODE`,
      `BOOTHY_DISPLAY_SAMPLE_MODE`(off여야 한다), `BOOTHY_SOURCE_COMPARE_MODE`(off여야 한다)
  - 2026-08-16 보완: Canon EF-S 18–55mm 번들렌즈, 카드 미사용/SaveTo Host, USB Port 6 직결·허브 없음, LG GSM5B55 1920×1080/60Hz/DPR1, 별도 사용자 ICC 연결 없음·sRGB/perceptual 운영을 기록했다. 읽을 수 없는 펌웨어 정확 버전·렌즈 세부 리비전·전원 어댑터 모델·HDR 활성 상태는 만들어내지 않고 Noah Lee 승인 제한 예외로 명시했다.
- [x] **display profile 증거**: `photoRect`와 `requiredSource*` 실측값. 표본마다 함께 기록한다.
      Story 7.1 evidence의 1080p 실측은 `1428.859 × 952.562`였고 전체화면 `1620×1080`이 아니었다
- [x] **proxy bundle metadata 증거**: 사용된 preset의 `proxyPublication` 원문, recipe version, visual approval 결과
- [x] **generation manifest 증거**: `pointer.json` 전/후, `generations.jsonl`, 확정 파일 경로·해시·크기
- [x] **publish/decode 증거**: 프로세스 종료 → 구조 probe → 크기 → provenance → commit → notify 순서를
      보이는 로그 구간. 거부 케이스별 고유 reason
- [x] **actual-present span 증거**: 표본별 raw JSONL + 집계(p50/p95/max/성공률).
      **실패와 timeout을 제외하지 않는다.** 5초를 넘으면 넘은 대로 적는다
- [x] **zero-wrong-frame 보고**: wrong-session / wrong-request / wrong-capture / **wrong-preset** /
      unfiltered qualifying / blank / stale / **upscaled** / crop jump / scale jump / tier downgrade가 전부 0
- [x] **현재 밝기 제품 예외(2026-08-16, Noah Lee):** Daylight 평균 12.27/255, 최종 Mono Pop 평균 7.39/255의 심한 저조도를 일반 품질 기준 통과로 바꾸지 않고 현재 운영 환경의 제한된 예외로 허용했다. 원 수치와 위험은 보존한다. 이 결정은 환경 fingerprint 또는 측정하지 않은 metrics/blind review를 면제하지 않는다.
- [x] **계측 완결성 게이트**를 이 회차에도 태운다:
      `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`.
      proxy generation 하나당 terminal 행 하나
- [x] `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의 Story 7.4 행을 갱신한다
- [x] **HV-15 `Go`를 2026-08-16 기록한 뒤 status를 `done`으로 전환했다.** 자동 테스트만이 아니라 실장비 결과, 운영 경로 결정, 밝기·환경·화질 제한 예외 승인을 함께 근거로 삼았다.

### T9. 영향 문서를 갱신한다 (all AC)

- [x] `docs/contracts/viewer-display.md` (proxy tier, provenance, v1→v2, 신규 거부 사유)
- [x] `docs/contracts/preset-bundle.md` (`proxyPublication` 블록과 부재 시 기본값)
- [x] `docs/contracts/authoring-publication.md` (publish input 확장과 거절 조건)
- [x] `docs/contracts/render-worker.md` (display-proxy 진입점, 별도 worker root, 384 preview 경로 불변)
- [x] `docs/contracts/capture-source.md` (accepted source → proxy 승격 조건, 크기 gate)
- [x] `_bmad-output/planning-artifacts/architecture.md` implementation note (Story 7.4)
- [x] Story completion note, `sprint-status.yaml`, HV-15 ledger row

### Review Findings

- [x] [Review][Decision→Patch] `proxyPublication`이 없는 preset의 정확 RAW 렌더를 `displayFitPresetProxy`로 허용할지 결정 — **해결:** 승인 없는 bundle은 source route와 무관하게 차단하고, 내장 preset 3종에는 Noah Lee의 수동 제품 승인·실제 XMP operation·승인 recipe를 명시했다. 기존 내장 bundle도 카탈로그 준비 시 같은 승인 정보로 보강한다.
- [x] [Review][Patch] sample/proxy 동시 활성화 시 sample이 아니라 proxy를 우선하도록 충돌 처리를 바로잡는다 — **해결:** 충돌 시 proxy lane만 게시·계측하도록 단일 우선순위를 적용했다. [src-tauri/src/commands/capture_commands.rs:196]
- [x] [Review][Patch] 렌더 완료 직전 현재 viewer 세션·epoch·화면 크기를 재검증해 이전 세션 proxy와 stale 크기 게시를 차단한다 — **해결:** 렌더 전후 viewer 문맥과 게시 시점 세션을 다시 확인하고 불일치 산출물을 폐기한다. [src-tauri/src/commands/display_commands.rs:675]
- [x] [Review][Patch] 게시 입력의 colorSpace·iccIntent·선택 문자열을 실제 렌더러 allowlist와 동일하게 검증한다 — **해결:** host와 shared contract를 sRGB 및 지원 intent로 제한하고 빈 선택 문자열을 거절한다. [src-tauri/src/contracts/dto.rs:719]
- [x] [Review][Decision] visualApproval metrics/blindReview — **해결(2026-08-16):** 측정값을 만들어내지 않고 Story 7.4의 현재 `raw-original + pinned darktable` 운영 경로에 한해 Noah Lee 제한 예외로 닫는다. 정확 경로와 달라질 수 있는 Story 7.5 상주 렌더러에는 metrics와 사람 비교 검토를 다시 필수로 인계한다. [src-tauri/src/contracts/dto.rs:709]
- [x] [Review][Patch] viewer의 older-request/older-capture 판정을 host와 같은 순서 좌표로 수행한다 — **해결:** generation 계약에 request/capture 순서를 전달하고 legacy 데이터만 시간값으로 판정한다. [src/display-generation/services/display-guard.ts:115]
- [x] [Review][Patch] booth present 계측 gate를 1회 snapshot이 아니라 pointer 갱신에 재수렴시키고 초기 조회 실패를 복구한다 — **해결:** pointer 구독과 재시도를 연결하고 늦은 snapshot이 최신 이벤트를 덮지 못하게 했다. [src/display-generation/state/use-measurement-lane-state.ts:24]
- [x] [Review][Patch] `presented` 보고에는 `actualPresentAtMicros`가 반드시 존재하도록 host 경계에서 검증한다 — **해결:** frontend schema와 host validation 모두 실제 표시 시각을 필수로 검증한다. [src-tauri/src/contracts/dto.rs:2126]
- [x] [Review][Patch] clock calibration 전 pending present 보고가 16건을 넘을 때 실제 present 증거가 조용히 유실되지 않게 한다 — **해결:** 중복 결과만 교체하고 보정 완료 전의 서로 다른 표시 증거는 모두 유지한다. [src/viewer-surface/ViewerSurface.tsx:115]

## Dev Notes

### 코드베이스 현실 — 착수 전 반드시 인지할 것

| 항목 | 현재 상태 |
| --- | --- |
| viewer window / readiness | **완성 (7.1).** `src-tauri/src/viewer/`. `photoRect.requiredSource*`는 host가 `ceil(css × DPR)`로 재계산해 이미 갖고 있다 |
| immutable generation / atomic pointer | **완성 (7.2).** `src-tauri/src/display/`. commit 순서가 코드 구조로 강제되어 있다 |
| opaque double-buffer swap | **완성 (7.2).** `src/viewer-surface/components/DoubleBufferedPhoto.tsx` |
| one-clock actual-present 계측 | **완성 (7.2).** `src-tauri/src/viewer/present_telemetry.rs`, `viewer-present.jsonl` |
| display tier | **`sample` 하나뿐.** `viewer-display.ts:17`, `dto.rs:1844-1852`. **이 Story가 두 번째 값을 추가한다** |
| `sampleVariant` | **필수 `z.enum(['a','b'])`** (`viewer-display.ts:90`). proxy에는 의미가 없다 → nullable 확장 필요 |
| 게시 seam | `publish_generation_in_dir`(`sample_publisher.rs:150`). **fixture 읽기만 실제 렌더로 바꾸면 된다** |
| `measurementLaneEnabled` | pointer snapshot의 필드. **7.2의 sample lane 상태에서만 나온다.** booth/viewer가 이 값으로 present 계측 IPC를 gate한다 → proxy lane만 켜면 계측이 0건이 된다 |
| 확정 경로 | `<session_root>/renders/display/<requestId>/<seq>-<variant>.jpg` (`generation_repository.rs:104`) |
| preview 렌더 출력 크기 | **384px 고정.** `render/mod.rs:24-27`. booth 사진 레일용이며 **바꾸지 않는다** |
| darktable 호출 | `darktable-cli` one-shot. `--hq false`, `--apply-custom-presets false`, `--width/--height` (`render/mod.rs:890-898`) |
| darktable worker root | `.boothy-darktable/{preview,final}/` (`render/mod.rs:874`). proxy는 **세 번째 root**가 필요하다 |
| 렌더 큐 | `MAX_IN_FLIGHT_RENDER_JOBS = 2`, timeout 45초 (`render/mod.rs:21-22`). preview/final과 공유한다 |
| pinned darktable | `5.4.1` (`render/mod.rs:20`) |
| published bundle | `published-preset-bundle/v1`. `darktableVersion`, `xmpTemplatePath`, `previewProfile`, `finalProfile`. **proxy metadata 없음** |
| capture ↔ preset 결속 | `SessionCaptureRecord.active_preset_id` + `active_preset_version` (`session_manifest.rs:238-239`). **capture record가 truth이고 live catalog pointer가 아니다** |
| accepted fast source | **아직 없다.** 7.3의 3 route가 실장비에서 전부 거부됐다 (`orientation-unsupported` / `absent`) |
| source 승격 판정 | `src-tauri/src/capture/source_probe.rs`. 12개 고유 거부 사유를 이미 소유한다 |
| RAW 추출 | `src-tauri/src/capture/embedded_jpeg.rs`. CR2 IFD#0에서 full-size JPEG (5184×3456 관측) |
| 진행 중 preset 변경 | **가능하다 (Story 2.3).** 이후 촬영에만 적용되지만, **늦은 proxy가 화면을 덮을 경로가 생긴다** |

### 핵심 설계 결정 1 — 이 Story는 계약을 새로 만들지 않는다. tier 하나를 추가한다

Story 7.2가 남긴 문장이 이 Story의 설계다:

> Story 7.4는 여기의 fixture 읽기만 실제 display-fit preset proxy 렌더로 교체하면 된다.
> generation/pointer/journal 계약은 그대로 재사용된다. — `sample_publisher.rs:6`

- `publish_generation_in_dir`의 commit 순서·거부 매트릭스·journal·pointer는 **그대로 쓴다**
- viewer의 double-buffer swap·decode gate·snapshot 재수렴은 **그대로 쓴다**
- `viewer-present.jsonl`의 span 계약은 **그대로 쓴다**
- 새로 만드는 것: tier 값 1개, provenance 블록 1개, 거부 사유 2개, 렌더 진입점 1개, publisher 1개

**두 번째 게시 경로를 만들면 이 Story의 진짜 위험이 시작된다.** 게시 순서가 두 벌이 되면 한쪽이
조용히 어긋나고, Story 7.2가 증명한 표시 종단점은 더 이상 같은 종단점이 아니게 된다.

### 핵심 설계 결정 2 — display fit은 크기 gate가 아니라 **source 선택 조건**이다

크기 검증은 이미 `evaluate_admission`에 있다. 문제는 그 gate가 **렌더가 끝난 뒤**에 돈다는 것이다.

- `--upscale false`이므로 darktable은 source보다 큰 결과를 만들지 않는다
- source가 `requiredSource*`보다 작으면 결과도 작고, 3.5초를 태운 뒤 `insufficient-dimensions`로 버려진다
- 그동안 관람 화면은 **비어 있다.** CPU만 쓰고 고객은 아무것도 못 본다

그래서 **크기 판정을 렌더 전으로 당긴다.** source의 실측 크기를 먼저 보고, 부족하면 proxy를 만들지 않는다.

| source | 실측 크기 | 1080p (≈1429×953) | 4K (≈2880×1920) |
| --- | --- | --- | --- |
| CR2 embedded JPEG | 5184×3456 | 통과 | 통과 |
| Windows Shell 썸네일 | 요청 1600, **실측은 다를 수 있다** | 조건부 | **미달** |
| RAW original | 5184×3456 | 통과 | 통과 |

`IShellItemImageFactory.GetImage`의 `BiggerSizeOk`는 요청보다 큰 캐시본을 허용하므로 **실제 산출 크기가
요청값과 다르다.** 7.3이 표본마다 실측 크기를 기록하게 만든 이유가 바로 이 판정이다.

### 핵심 설계 결정 3 — 미승인 preset은 정확한 RAW 경로만 쓴다 (근사 금지)

AC 2의 핵심이고, 제품 신뢰가 걸린 지점이다.

- `proxyPublication` 블록이 없거나 `proxyCompatible=false` → **proxy lane을 시작조차 하지 않는다**
- 부분적으로만 지원되는 operation을 "대충 비슷하게" 만들지 않는다.
  과거 XMP trimming이 2.8초를 만들었지만 look-affecting module을 제거한 것이어서
  **비교 증거로 강등된 이력**이 이 원칙의 근거다
- fallback은 오늘 이미 동작하는 정확한 darktable RAW 경로다. 새로 만들 것이 없다
- **블록 부재의 기본값은 `false`다.** 기존 게시 번들이 조용히 proxy lane에 들어가면
  승인되지 않은 룩이 고객 화면에 올라간다

### 핵심 설계 결정 4 — preset guard가 없으면 AC 4가 성립하지 않는다

현재 `evaluate_admission`은 session / epoch / tier / seq / request order / 크기를 본다. **preset은 보지 않는다.**

Story 2.3이 세션 중 preset 변경을 허용한다. 시나리오:

1. capture A를 preset X로 촬영 → proxy 렌더 시작 (3.5초)
2. 고객이 preset Y로 변경
3. capture B를 preset Y로 촬영 → proxy 렌더 시작
4. **A의 proxy가 B보다 늦게 끝나거나, B가 먼저 끝난 뒤 A가 도착한다**

seq guard가 대부분을 막지만, capture record의 preset이 화면과 갈라지는 경로를 **명시적 고유 사유**로
막아야 한다. AC 4가 "different preset"을 별도로 적은 이유다.

### 핵심 설계 결정 5 — 속도는 이 Story의 합격 조건이 아니다. 정직한 기록이 합격 조건이다

AC 5가 명시적으로 허용한다. 실측 기준선:

| 구간 | 실측 |
| --- | --- |
| display-fit 1920px darktable one-shot (CPU) | 약 3.53초 |
| 같은 작업 (OpenCL) | 약 4.99초 |
| 실제 pixelpipe 계산 | **약 0.3초** |
| 2026-08-11 HV-13B 표시 종단점 (fixture) | p50 3710.617 / p95 4280.416 / max 4672.717 ms |
| 2026-08-12 실장비 capture→XMP preview | 6804~24028 ms |

즉 비용의 대부분은 **매 capture마다 새 프로세스·core·OpenCL 초기화**다. 이것을 없애는 것이 7.5의 spike이고,
무중단 교체가 7.6이다. **이 Story에서 임계값을 올리거나 표본을 골라내면 7.5의 판단 근거가 사라진다.**

### 핵심 설계 결정 6 — proxy는 큐를 공유한다. 지금은 재기만 한다

`acquire_render_queue_slot()`은 슬롯 2개를 preview/final과 공유한다. 그리고 현재 파이프라인은
RAW 도착 후 384px preview refinement를 자동으로 건다 (`ingest_pipeline.rs`의
`spawn_preview_raw_refinement_in_dir`). 즉 **고객의 첫 화면이 booth 레일 썸네일 렌더 뒤에서 기다릴 수 있다.**

- 이 Story는 `proxyQueueWaitMicros`를 기록한다
- **우선순위 스케줄러(P0 proxy → P1 RAW refine → P2 final/warm-up)는 Story 7.6이 소유한다.**
  여기서 만들면 7.6의 evidence가 오염되고, 이 Story의 회귀 위험이 크게 커진다
- 큐 대기가 실제로 크게 나오면 그 관측 자체가 7.6의 착수 근거다

### 핵심 설계 결정 7 — pointer 스키마 v2는 v1을 읽을 수 있어야 한다

`pointer.json`은 session root 안에 있고 세션은 짧지만, Story 7.9가 **pre-upgrade session 호환과
old generation pointer 복구**를 요구한다. 진행 중인 세션 위로 업그레이드가 떨어졌을 때 관람 화면이
죽으면 rollback 증거가 성립하지 않는다.

- 쓰기는 `viewer-display/v2`
- 읽기는 v1도 받고 `tier: 'sample'`, `proxyProvenance: null`로 정규화한다
- **이 정규화를 테스트로 고정한다.** "아마 괜찮을 것"으로 두면 7.9에서 발견된다

### UX 가드레일

- **고객 문구를 새로 만들지 않는다.** NFR-001의 copy budget(주 문장 1 + 보조 문장 1 + 액션 라벨 1)은
  이미 차 있다. `customerStatusCopy.ts`에 새 문구를 추가할 이유가 생겼다면 설계가 잘못된 것이다
- 관람 화면의 standby 문구는 Story 7.1의 `"사진이 준비되면 여기에 보여드릴게요."` 하나를 유지한다.
  이미지가 표시되는 동안에는 `visibility: hidden`으로 **자리만 남긴다** (`display: none`은 photo rect를 바꾼다)
- 표시 실패는 관람 화면에 노출하지 않는다. booth control surface의 wait/call guidance로만 투영한다
- 관람 화면의 조작 요소·진단 표시는 **0**을 유지한다
- `<img>`의 `alt`는 고객 안전 문구 하나만 쓴다
- UX-DR19: 첫 qualifying frame은 같은 session/request/capture/preset@version에 연결된
  **physical display-fit preset-applied 이미지**여야 한다. 무보정 camera image와 review rail thumbnail은 성공이 아니다

### 아키텍처 가드레일 (위반 시 리뷰 반려)

- React 컴포넌트에서 `invoke()` / `listen()`을 직접 호출하지 않는다. viewer는 `viewer-host-adapter.ts`만 거친다
- Rust command는 `src-tauri/src/commands/`의 얇은 진입점이고 도메인 로직은 `src-tauri/src/display/`, `src-tauri/src/render/`에 둔다
- 계약 정의는 TS/Rust 한 쌍만 존재한다. 세 번째 정의를 만들지 않는다
- snapshot이 durable recovery 경계다. live event만으로 display 상태를 소유하지 않는다
- 이미지 bytes를 IPC로 옮기지 않는다. **immutable asset path만 보내고 WebView가 직접 읽는다**
- 파일 I/O·fsync·렌더를 하는 command는 반드시 `#[tauri::command(async)]`다.
  **HV-13A `No-Go`의 실제 원인이 이 규칙 위반이었다**
- 세션 밖 자산·상태가 viewer로 새지 않게 한다. NFR-004는 0 tolerance다
- render worker는 live catalog pointer가 아니라 **capture record의 `activePresetId + activePresetVersion`**을 쓴다

### 절대 하지 말 것 (disaster prevention)

1. **두 번째 게시 경로를 만들지 않는다.** `publish_generation_in_dir` 하나만 쓴다. commit 순서가 두 벌이 되는 순간 7.2의 증명이 무효가 된다.
2. **notify를 pointer commit보다 먼저 하지 않는다.**
3. **canonical 파일을 in-place overwrite하지 않는다.** Story 1.9가 그 방식이었고 superseded evidence로 강등됐다.
4. **`RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*` 384 상수를 바꾸지 않는다.** booth 사진 레일 경로다.
5. **`--upscale`을 켜거나 생략하지 않는다.** upscaled frame은 PRD의 zero 항목이다.
6. **source 크기 미달을 렌더 뒤에 발견하지 않는다.** 렌더 전에 판정한다.
7. **보정되지 않은 source를 관람 화면에 게시하지 않는다.** `cameraSource`는 qualifying frame이 아니다.
8. **`proxyCompatible`이 없는 번들을 proxy lane에 태우지 않는다.** 부재의 기본값은 `false`다.
9. **미지원 operation을 근사하지 않는다.** 정확한 RAW fallback을 쓴다.
10. **preset guard 없이 AC 4를 통과했다고 하지 않는다.** 다른 preset의 늦은 proxy가 현재 화면을 대체할 수 있다.
11. **`sampleVariant`를 삭제하지 않는다.** 7.2 evidence와 완결성 스크립트가 읽는다. nullable 확장만 한다.
12. **darktable worker root를 preview/final과 공유하지 않는다.** `library.db` 경합으로 서로를 막는다.
13. **우선순위 스케줄러를 만들지 않는다.** Story 7.6 범위다. 큐 대기는 재기만 한다.
14. **`viewer-present.jsonl`과 `source-comparison.jsonl`을 합쳐 집계하지 않는다.**
15. **5초 예산 초과를 숨기거나 임계값을 올리지 않는다.** AC 5가 초과를 허용하되 **기록을 요구**한다.
16. **`BOOTHY_DISPLAY_PROXY_MODE`를 HV-15 `Go` 전에 기본 `on`으로 두지 않는다.**
17. **7.2의 fixture lane과 동시에 켜지 않는다.** 같은 pointer를 다투면 evidence가 오염된다.
18. **`session.json` 스키마를 확장하지 않는다.** `pointer.json`의 versioned manifest를 쓴다.
19. **새 Rust crate를 승인 없이 추가하지 않는다.** Story 7.7 offline 인벤토리에 직접 영향을 준다.
20. **자동 테스트 통과만으로 `done` 처리하지 않는다.** HV-14 route 결정과 HV-15 `Go` 없이는 `review` 유지다.

### 재사용할 기존 패턴 (바퀴 재발명 금지)

| 필요한 것 | 참고할 기존 구현 |
| --- | --- |
| immutable commit 순서 전체 | `src-tauri/src/display/sample_publisher.rs` `publish_generation_in_dir` |
| 승격/거부 판정 순수 함수 | `src-tauri/src/display/display_artifact.rs` `evaluate_admission` |
| staging → probe → rename → pointer → journal | `src-tauri/src/display/generation_repository.rs` |
| JPEG 구조 probe (SOI/SOF/EOI/orientation) | `src-tauri/src/display/image_probe.rs` |
| darktable 호출 조립과 실행 | `src-tauri/src/render/mod.rs` `build_darktable_invocation_from_source`, `run_darktable_invocation` |
| 임의 source → 임의 output 렌더 | `src-tauri/src/render/mod.rs` `render_preview_asset_to_path_in_dir` |
| published bundle 로딩과 검증 | `src-tauri/src/preset/preset_bundle.rs` `load_published_preset_runtime_bundle` |
| source 승격/거부 판정 | `src-tauri/src/capture/source_probe.rs` `evaluate_source_candidate` |
| 비차단 게시 스레드 | `src-tauri/src/commands/display_commands.rs:415` `spawn_sample_lane_publication` |
| lane 환경 변수 파싱 (알 수 없는 값 → off) | `src-tauri/src/display/sample_publisher.rs` `parse_sample_lane_mode` |
| host monotonic micros 계측 | `src-tauri/src/viewer/present_telemetry.rs` |
| 계측 완결성 (generation당 terminal 행 하나) | `src-tauri/src/commands/display_commands.rs` `close_out_open_generations` |
| viewer 승격 guard | `src/display-generation/services/display-guard.ts` `shouldAdvanceDisplay` |
| snapshot 재수렴 + stale 폐기 | `src/display-generation/state/use-display-pointer.ts` |
| opaque double-buffer swap | `src/viewer-surface/components/DoubleBufferedPhoto.tsx` |
| 파일 경로 → asset URL | `src/viewer-surface/services/display-asset-src.ts` |
| zod 스키마 + DTO 분리 | `src/shared-contracts/schemas/viewer-display.ts` + `dto/display.ts` |
| Rust 통합 테스트 (tempdir) | `src-tauri/tests/viewer_display.rs` |
| 하드웨어 evidence 패키지 구조 | `tests/hardware/viewer-present/hv-13b/README.md` |
| 완결성 검사 스크립트 | `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1` |
| 최종 증거 게이트 스크립트 | `tests/hardware/capture-source/hv-14/check-source-completeness.ps1` |

### 최신 기술 정보 (구현 전 확인 완료)

- **`darktable-cli` 옵션** — `--width`, `--height`, `--bpp`, `--hq <0|1>`, `--upscale <0|1>`,
  `--apply-custom-presets <0|1>`, `--out-ext`, `--icc-type`, `--icc-file`, `--icc-intent`, `--core`.
  `--core` 뒤의 인자는 darktable core로 전달되므로 **반드시 마지막**이다 (현재 코드도 그렇게 조립한다).
  **daemon / stdin / RPC mode는 없다** — 이것이 3.5초의 구조적 원인이며 7.5의 존재 이유다.
  ([darktable-cli](https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/))
- **JPEG 품질 설정** — `darktable-cli`에 전용 플래그가 없다. core 설정 키로 전달해야 하며,
  **키 이름을 구현 전에 실제 `darktablerc`로 확인한다.** 틀린 키는 오류 없이 무시되어
  계약과 다른 품질이 조용히 나간다. `--out-ext jpg`와 함께 검증한다.
- **OpenCL** — 프로세스 시작마다 device 확인·context·kernel 준비를 수행한다.
  실측에서 one-shot OpenCL(4.99초)이 CPU(3.53초)보다 **느렸다.** proxy 기본은 CPU로 두고,
  OpenCL 강제는 7.5의 비교 대상이다.
  ([OpenCL activation](https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/))
- **Tauri asset protocol** — Windows에서 `convertFileSrc`는 `http://asset.localhost/...`를 만든다.
  세션 루트가 이미 `$PICTURE/dabi_shoot/**` scope 안이므로 **`tauri.conf.json` 변경이 필요 없다.**
  앱 문서(`http://tauri.localhost`)와 cross-origin이라 Element Timing `renderTime`이 0이 될 수 있고,
  그 fallback 값을 actual-present로 승격하면 안 된다 (7.2 계약).
  ([Tauri asset protocol](https://v2.tauri.app/security/asset-protocol/))
- **Windows `std::fs::rename`** — `MoveFileExW + MOVEFILE_REPLACE_EXISTING`이라 pointer 교체에 안전하다.
  generation 파일 충돌은 `create_new(true)`로 막는다.
- **화질 gate 기준값** — SSIM ≥ 0.95, median ΔE00 ≤ 3, p95 ≤ 8, skin ROI median ≤ 3,
  proxy MTF50 ≥ RAW 정밀본의 80%, clipping 증가 ≤ 2%p, upscale 0.
  blind review 5명 × 30 transition에서 "프리셋이 달라 보이거나 교체가 거슬린다"가 5%를 넘으면 승인하지 않는다.
  ([OpenCV SSIM](https://docs.opencv.org/4.8.0/d5/dc4/tutorial_video_input_psnr_ssim.html),
  [CIEDE2000](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/))

### 기술 스택 고정값

- Tauri `2.10.3` (Rust), `@tauri-apps/cli`/`api` `^2.10.1`, Rust edition 2021, rust-version `1.77.2`
- Rust deps: `serde`, `serde_json`, `log`, `tauri`(`protocol-asset`), `tauri-plugin-log`. **새 crate 추가 전 승인 필요**
- darktable pinned `5.4.1` (`render/mod.rs:20`). 번들 metadata와 불일치하면 `darktable-version-mismatch`로 거절된다
- helper: .NET 8, C#, `sidecar/canon-helper/`. 테스트는 `dotnet test`
- React `^19.2.4`, zod `^4.3.6` (v4 API), TypeScript `~5.9.3`, Vite `^8.0.1`
- Vitest `^4.1.0`, jsdom `^29`, Testing Library React `^16.3.2`, setup: `src/test/setup.ts`
  - jsdom에는 `HTMLImageElement.prototype.decode`가 없다. `DoubleBufferedPhoto` 테스트는 `decode`를 stub한다
- 패키지 매니저: **pnpm 10.31.0**. npm/yarn 금지

### Previous story intelligence — Story 7.3 (`review`, HV-14 `No-Go`)

이 Story가 반드시 상속해야 할 학습:

- **자동 검증 전부 통과 + 실장비 실패가 네 번 일어났다.** Story 1.9, 7.1 HV-13A, 7.2 HV-13B, 7.3 HV-14가
  모두 그랬다. **HV-15 증거 절차를 구현과 동시에 준비한다.** 나중에 준비하면 늦는다.
- **7.3은 착수 전 스토리의 사실 주장 3건이 코드와 달랐다.** 편집 전에 대상 파일의 현재 내용을 다시 읽고,
  틀린 서술을 발견하면 **구현이 아니라 스토리를 먼저 고친다.**
- **계약 문서가 구현에 없는 값을 예시로 갖고 있었다** (`fastPreviewKind: "embedded-jpeg"`).
  계약을 확장하기 전에 **현재 계약이 사실인지 먼저 확인한다.**
- **취소·정리 경로가 정당한 데이터를 지운다.** 7.2의 effect cleanup이 커밋된 표본의 보고를 취소했고,
  7.3의 `EdsRelease`도 같은 위험이었다. proxy 취소·정리 경로를 같은 눈으로 본다.
- **`capture_readiness` flakiness는 병렬 실행 경합이다.** `--test-threads=1`에서 60/60 통과한다.
  회귀 판정을 이 방법으로 한다.
- **환경 변수 lane은 기본 off, 알 수 없는 값도 off.** 오타가 lane을 켜지 않아야 한다.

### Previous story intelligence — Story 7.2 (`done`, HV-13B 재정의 `Go`)

- **계측에 행이 없는 산출물을 만들지 마라.** commit된 generation 10개 중 1개에 terminal 행이 없어
  한 회차를 잃었다. 분모가 조용히 줄어 성공률이 실제보다 좋아 보인다.
- **`.viewer-surface__photo`의 기하를 바꾸면 촬영이 막힌다.** `use-viewer-readiness.ts`가 이 요소를
  실측해 1px 허용오차로 비교하고, 어긋나면 `layoutReady = false` → 촬영 차단이다.
- **`#[tauri::command]` 기본 blocking 실행이 HV-13A `No-Go`의 실제 원인이었다.**
- **epoch + revision + seq 삼중 방어가 실제로 필요했다.** 늦게 도착한 이전 epoch report가 현재 상태를
  오염시키는 회귀가 code review에서 반복 검출됐다.
- **승인 기본값은 `visible-standby`다.** hidden 변형은 p95가 느리고 15.024초 outlier가 있었다.
  proxy lane도 이 기본값 위에서 측정한다.
- **물리 프레임 게이트는 7.8 / HV-18B로 이관됐다.** 이 Story는 120fps 촬영 rig를 필요로 하지 않는다.
  다만 **HV-18B가 표시 종단점 귀책 결함을 발견하면 7.2가 `review`로 되돌아간다** — 이 Story가
  게시 계약을 흔들면 그 위험이 커진다.

### Git intelligence

최근 커밋: `c390f53`(Story 7.1), `89ae52c`, `b24cfc4`, `12309fa`, `81b1271`.
**Story 7.2와 7.3 작업은 아직 커밋되지 않은 작업 트리에 있다** (branch `story/7-1-viewer-window-readiness`,
변경 143건). 즉 이 Story가 의존하는 `src-tauri/src/display/`와 `src-tauri/src/capture/source_*.rs`는
**HEAD에 없다.** 착수 전 작업 트리 상태를 먼저 확인한다.

앞의 하나를 제외한 최근 커밋이 전부 **capture timing / preview latency / thumbnail fallback** 영역이다.
`12309fa`("Record thumbnail validation and ship fast preview fallback")가 현재 384px incumbent를 만든 커밋이다.

편집 전 현재 내용을 다시 읽어야 할 큰 파일:

- `src-tauri/src/render/mod.rs` (1732줄)
- `src-tauri/src/contracts/dto.rs` (1800줄+)
- `src-tauri/src/commands/display_commands.rs` (1281줄)
- `src-tauri/src/capture/ingest_pipeline.rs` (1345줄)
- `src/shared-contracts/schemas/preset-authoring.ts` (720줄)

### Project Structure Notes

- 신규 Rust: `src-tauri/src/display/proxy_publisher.rs` — 아키텍처가 예약한 `src-tauri/src/display/` 경계 안이다
- 확장 Rust: `src-tauri/src/render/mod.rs`(진입점 추가), `src-tauri/src/display/{display_artifact,mod}.rs`,
  `src-tauri/src/preset/preset_bundle.rs`, `src-tauri/src/contracts/dto.rs`,
  `src-tauri/src/commands/display_commands.rs`
- 확장 TS: `src/shared-contracts/schemas/{viewer-display,preset-authoring}.ts`,
  `src/shared-contracts/dto/display.ts`, `src/display-generation/services/display-guard.ts`
- 신규 evidence: `tests/hardware/display-proxy/hv-15/`
- 신규 저장 경로: 없음. proxy generation은 7.2의 `<session_root>/renders/display/<requestId>/`를 그대로 쓴다
- 신규 worker root: `<base_dir>/.boothy-darktable/display-proxy/` — 기존 `{preview,final}`과 형제
- **편차와 근거:** 아키텍처는 display generation transport로 Tauri Channel을 권장하지만, Story 7.2가
  broadcast event + snapshot 재수렴 + monotonic guard를 선택했고 이 Story도 그것을 유지한다.
  정확성은 전송 순서가 아니라 guard에서 나오며, tier가 3개가 되는 **Story 7.6에서 재평가**한다.
  이 편차는 `docs/contracts/viewer-display.md`에 이미 기록되어 있다

### Verification Guardrails

- 자동 테스트는 필요하지만 HV-15를 대신하지 않는다
- Story 7.4 evidence는 **immutable publication · display fit · correlation · zero-wrong-frame**에 한정한다.
  warm 100-shot 성능 판정은 Story 7.8이, 물리 프레임은 HV-18B가 소유한다
- 이 Story의 latency 수치는 **one-shot 기준선**이지 제품 SLA 통과 근거가 아니다
- Epic 7 dependency상 HV-14 route 결정이 막히면 HV-15 `Go`도 막힌다. 구현과 자동 검증은 그와 무관하게 완결할 수 있다
- HV-18B가 표시 종단점 귀책의 전환 결함을 발견하면 Story 7.2가 `review`로 되돌아간다.
  이 Story가 게시·swap 계약을 흔들지 않았음을 scope guard로 증명한다

## Dependencies

- **Story 7.3 / HV-14 승인 route 결정** — HV-15 `Go`의 선행 조건. **2026-08-14 현재 `No-Go`다**
- Story 7.1 / HV-13A `Go` (viewer readiness, `photoRect.requiredSource*`, 승인 모니터 지정)
- Story 7.2 / HV-13B 재정의 `Go` (immutable generation, atomic pointer, opaque swap, actual-present 계측)
- Story 4.3 published preset bundle (`published-preset-bundle/v1`)과 catalog snapshot 결속.
  **Story 4.2/4.3은 아직 `review`다** — 이 Story가 그 경로의 성공 조건을 바꾸지 않는다
- **최소 1개 이상의 preset에 대한 `proxyPublication` 승인** (recipe version + visual approval).
  없으면 proxy lane이 한 번도 실행되지 않아 HV-15를 실행할 수 없다
- 승인 EOS 700D, 승인 부스 PC, **승인 고객 모니터와 display profile** (HV-15 필수 조건)
- pinned darktable `5.4.1` 실행 가능 환경 (`BOOTHY_DARKTABLE_CLI_BIN` 또는 알려진 설치 경로)
- Canon EDSDK payload (`BOOTHY_CANON_SDK_ROOT`) — helper 빌드/테스트 선행 조건
- 물리 모니터 고속 촬영 장비에는 **의존하지 않는다** (7.8 / HV-18B 소유)

## Handoff

- Primary: Windows host(Rust) + frontend/WebView 소유 개발팀
- Architecture review: tier 확장, provenance 계약, pointer v1→v2 호환, worker root 분리, 큐 공유 관측
- Preset authoring / Product: `proxyPublication` 승인 절차, 화질 gate 임계값, 미승인 preset의 fallback 정책
- QA: HV-15 display profile, bundle metadata, generation manifest, publish/decode 순서, actual-present span, zero-wrong-frame
- UX: 첫 성공 화면의 고객 인지 품질, standby → proxy 전환, copy budget 불변

## References

- `_bmad-output/planning-artifacts/epics.md#Story-74-화면-적합-immutable-preset-proxy-생성과-관람-화면-게시` (AC 원문)
- `_bmad-output/planning-artifacts/epics.md#Epic-7` (execution rule, evidence dependency, UX-DR19)
- `_bmad-output/planning-artifacts/prd.md#FR-010-Pre-Opened-Full-View-Progressive-Preset-Display`
- `_bmad-output/planning-artifacts/prd.md#NFR-003-Booth-Responsiveness-and-Qualifying-Viewer-Readiness`
- `_bmad-output/planning-artifacts/prd.md#NFR-004-Session-Isolation-and-Privacy`
- `_bmad-output/planning-artifacts/prd.md#Published-Preset-Artifact-Model` (proxy 적격성 기록 요구)
- `_bmad-output/planning-artifacts/architecture.md#Approved-2026-08-11-Correct-Course-Baseline` (display pipeline, tier 규칙)
- `_bmad-output/planning-artifacts/architecture.md#Story-72-implementation-note` (7.4가 fixture만 교체한다는 seam 선언)
- `_bmad-output/planning-artifacts/architecture.md#Closed-Contract-Freeze-Baseline` (Display artifact / Proxy publication contract)
- `_bmad-output/planning-artifacts/ux-design-specification.md#전용-관람-화면-Customer-Viewer-Surface`
- `_bmad-output/planning-artifacts/research/technical-preset-image-fast-display-research-2026-08-10.md` (display-fit one-shot 실측, 화질 gate 임계값, proxyCompatible 원칙)
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260811-012637.md` (proxy publication 결정 원문)
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md` (물리 프레임 게이트 이관)
- `_bmad-output/implementation-artifacts/7-2-immutable-sample과-actual-present-계측.md` (게시 계약, 계측 완결성, HV 학습)
- `_bmad-output/implementation-artifacts/7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교.md` (source route, 크기 실측 규칙, HV-14 현황)
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md#Story-74` (HV-15 required evidence)
- `docs/contracts/viewer-display.md` (Story 7.2 계약 — 이 Story가 확장하되 깨지 말아야 할 경계)
- `docs/contracts/capture-source.md` (Story 7.3 계약 — source 승격 조건)
- `docs/contracts/preset-bundle.md`, `docs/contracts/authoring-publication.md`, `docs/contracts/render-worker.md`

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (1M context) — `claude-opus-5[1m]`; Codex GPT-5 (2026-08-15 실장비 재검증)

### Debug Log References

착수 전 스토리의 사실 주장을 코드로 대조했고, **두 가지가 달랐다.**

| 스토리 서술 | 실제 | 조치 |
| --- | --- | --- |
| `darktable-cli`의 JPEG 품질 키와 ICC 토큰을 "구현 전에 확인한다" | darktable 5.4.1이 설치되어 있어 **실측으로 확정했다.** `--icc-intent PERCEPTUAL`은 받아들여지고 `INTENT_PERCEPTUAL`은 거부된다. 품질은 `plugins/imageio/format/jpeg/quality`로만 전달된다 (q40 103KB vs q95 457KB) | 두 토큰 매핑을 화이트리스트로 고정하고, 알 수 없는 값은 오류로 만들었다 |
| preset id 예시 `preset_soft_glow` | `is_valid_preset_id`는 접두사 뒤에 `_`를 허용하지 않는다. 실제 catalog는 `preset_soft-glow` | 모든 fixture를 실제 형식으로 맞췄다 |

구현 중 확인한 것들:

- **`--upscale false`의 실제 동작을 확인했다.** 3840×2560 source + 목표 1620×1080 → 정확히
  1620×1080. 800×533 source + 같은 목표 → **800×533 그대로** (upscale 없음).
  스토리가 예측한 "작은 source는 렌더해도 화면을 못 채운다"가 실측으로 확인됐고,
  이것이 크기 판정을 렌더 **전으로** 당긴 근거다.
- **잘못된 darktable 플래그는 조용히 실패한다.** 인자 하나가 틀리면 렌더 대신 도움말을
  출력하고 **종료 코드 0**으로 끝난다. "성공했는데 파일이 없는" 상태가 되므로,
  색공간·intent 값을 추측하지 않고 화이트리스트로 막았다.
- **`older-capture` guard가 처음엔 발화하지 않았다.** 후보는 촬영 좌표를 들고 왔지만 host가
  commit된 generation의 좌표를 **기억하지 않아** 비교 대상이 없었다. `record_capture_order`를
  게시 경로에 넣어 고쳤다. 회귀 테스트가 수정 전 코드에서 실패하는 것을 확인했다.
- **viewer guard가 `undefined` provenance에서 throw했다.** `!== null`만 보면 zod를 거치지 않은
  값(스냅샷 캐시, 이전 빌드가 남긴 v1 generation)에서 런타임 예외가 나고 **화면 갱신이 통째로
  멈춘다.** `?? null`로 고쳤다. 이 결함은 기존 `use-display-pointer` 테스트가 잡았다.
- **`pnpm build`의 TS2719를 내 변경 탓으로 오인할 뻔했다.** `superRefine`을 걷어내고 다시 재
  봤지만 오류 수가 20으로 동일했고, 내가 건드리지 않은 `DoubleBufferedPhoto.test.tsx`에
  같은 오류가 있다. 기존 문제이며 이번 변경이 만든 것이 아니다.

#### 2026-08-14 정정 — proxy 자격 게이트가 과도했다

리뷰 지적("이미 만들어 둔 프리셋을 왜 못 쓰나")을 받아 재검토한 결과 **내 설계가 틀렸다.**

- `proxyCompatible` 승인 장치는 연구 문서와 아키텍처에서 **다른 렌더링 엔진**(Story 7.5의 상주
  렌더러)이 darktable을 근사하는 것을 막으려고 도입됐다. 아키텍처 문장도 "A **separate**
  display-fit proxy renderer may serve only presets with an approved versioned proxy recipe"다.
- 그런데 Story 7.4의 렌더러는 **darktable 자신**이고, recipe는 **같은 XMP**, source는 **같은 RAW
  원본**이다. 오늘의 preview/final과 다른 것은 출력 크기뿐이다. **근사가 존재하지 않는다.**
- 참조 렌더러 자신의 정확한 결과를 "승인되지 않았다"는 이유로 막으면, AC 2가 요구하는
  "암묵적 근사 대신 정확한 RAW 경로를 쓴다"와 **정반대**가 된다. 그리고 그 상태로는 게시된
  preset이 하나도 통과하지 못해 Story가 화면에 아무것도 올릴 수 없고 HV-15를 실행할 수도 없다.

**실측으로 확인한 것:**

- 이미 게시된 세 preset(`preset_soft-glow` / `preset_mono-pop` / `preset_daylight`, 2026.03.27)은
  전부 실제 darktable XMP를 갖고 있고 모듈은 `temperature`·`exposure`·`sigmoid`·`sharpen`과
  `bloom`/`monochrome`이다. **mask가 비어 있고** heal·AI·복잡한 geometry가 없다.
- 세 preset 모두 display-fit invocation으로 **정확히 1620×1080**을 만들고 서로 다른 룩을 낸다
  (평균 RGB — daylight 31,97,183 / mono-pop 98,98,98 / soft-glow 106,133,169).
- `darktable-cli`의 기본 JPEG 품질은 **95**다. 품질을 넘기지 않은 산출물과
  `plugins/imageio/format/jpeg/quality=95` 산출물이 **바이트까지 동일**했다.
  그래서 정확 경로의 기본 품질을 95로 두면 오늘 제품이 이미 만드는 품질과 같다.

**후속 정정(2026-08-16):** 위 조사에서 `raw-original`이 정확 경로임은 유지하지만, 현재 구현은
source route와 무관하게 유효한 `proxyPublication`을 요구한다. 세 기본 preset의 수동 승인과 HV-15의
`visual-approval` evidence로 Story 7.4를 닫았으며, 자동 `exact-reference-renderer` 자격이 구현된 것으로
주장하지 않는다. Story 7.5의 다른 renderer는 별도 metrics/blind review를 통과해야 한다.

#### 2026-08-15 실장비 재검증

- 세로 촬영본이 1429x953 영역에서 crop 또는 upscale 없이 높이 953을 채우는 contain 정책을 host, viewer, 계약 parser에 동일하게 적용했다.
- EOS 700D 실촬영 6건이 모두 immutable generation으로 commit되고 실제 관람 화면에 presented 됐다. median 8.1848655초, p95/max 10.464558초이며 느린 표본을 제외하지 않았다.
- S4에서 사진 삭제가 실패하는 실제 결함을 발견했다. 수정 후 새 실장비 세션에서 RAW 1→0, active generation null, `request-forgotten`, viewer standby를 확인했다.
- S5에서 renderer version 불일치를 일시 주입해 RAW/preview는 보존되고 proxy 게시만 `proxy-reference-renderer-mismatch`로 차단되는 것을 확인했다. 원본 bundle은 SHA256 `C0D1F57F3AD26B17CB0ECACD628F2F1BCB8D7AD282D2C9A6CCBCD203ED3CA48A`로 복원했다.
- S5 준비 중 RAW 다운로드 timeout 2회가 발생했고 helper 재시작 후 통과했다. display 성공 표본에서 숨기지 않고 camera 계층 장애로 별도 기록했다.
- 최종 Daylight/Mono Pop 사진은 평균 luma 13.89/255와 9.79/255로 심하게 어두워 품질 승인하지 않았다.

#### 2026-08-15 현재 조명 그대로 재검증

- 사용자 요청에 따라 조명과 카메라 노출을 조정하지 않고 새 세션 `session_000000000018cbdf04a181798c`을 실행했다.
- Daylight 연속 3회, Mono Pop 전환 1회, 삭제 후 Mono Pop 복구 1회가 모두 immutable commit 및 actual-present까지 성공했다. camera busy/RAW download timeout/proxy failure는 0건이다.
- 5개 표시 표본의 median은 8.284043초, p95/max는 9.735243초다. 삭제는 viewer를 standby로 되돌렸고 다음 촬영이 정상 표시를 복구했다.
- Daylight 평균 luma 12.27/255, 최종 Mono Pop 7.39/255로 현재 환경의 심한 저조도를 다시 확인했다. 기능 통과와 품질 승인을 분리하며, HV-14와 환경/visual approval 누락 때문에 No-Go를 유지한다.

### Completion Notes

- 2026-08-16 HV-15 종료: 사용자 제공 18–55mm 번들렌즈와 기존 HV-14/Windows 근거로 환경 정보를 보완하고, 읽을 수 없는 세부값과 별도 metrics/blind review는 Story 7.4 한정 제한 예외로 기록했다. 실장비 5/5 actual-present, 프리셋 전환, 삭제·복구, 기계식 gate, 계측 완결성을 근거로 HV-15 `Go`를 기록했다.
- HV-15 `Go` 후 `BOOTHY_DISPLAY_PROXY_MODE`의 무설정 기본값을 `on`으로 전환했다. 명시적 `off`와 알 수 없는 값은 계속 `off`로 닫히며, 기본값 단위/통합 테스트를 red-green으로 갱신했다.
- 종료 검증: `cargo fmt --check`, Rust lib 151/151, `viewer_display` 35/35, HV-15 기계식 gate, 계측 완결성, `pnpm lint` 통과. 전체 Vitest는 기존 Story 1.4 거버넌스 문구 실패 2건만 남아 903/905 통과했고, `pnpm build`는 기존 TypeScript 기준선 오류로 실패했다. 이번 기본값 변경에서 새 실패는 없다.

#### 무엇이 완료되었는가

- **T1 계약** — `displayFitPresetProxy` tier와 `proxyProvenance`를 추가하고, tier별 필드
  불변식을 TS `superRefine`과 host `validate_shape` **양쪽에서** 강제했다. pointer manifest는
  `viewer-display/v2`로 올리되 **v1을 읽을 수 있게** 남겼다. 신규 거부 사유 2종
  (`preset-mismatch`, `older-capture`)을 고유 코드로 등록했다.
- **`measurementLaneEnabled` / `presentTelemetryEnabled` 분리** — 스토리가 지목한 함정이 실재했다.
  proxy lane만 켜면 앞의 값이 `false`라 booth/viewer가 계측 IPC를 하나도 하지 않는다.
  두 값을 나누고 세 조합(off/off · sample only · proxy only)을 테스트로 고정했다.
- **T2 게시 자격** — `published-preset-bundle/v1`에 optional `proxyPublication`을 추가했다.
  **schemaVersion은 바꾸지 않았다.** 현재 구현은 모든 source route에 명시적 승인 블록을 요구하며,
  세 기본 preset에는 수동 승인 데이터가 포함돼 있다. Story 7.4 evidence는 `visual-approval`을 사용했고,
  fast source나 다른 엔진에는 별도의 metrics/blind review가 필요하다. 강등은 6가지 고유 사유로 남고,
  **preview/final 로딩은 절대 실패시키지 않는다.**
  게시 입력에도 optional 블록을 붙였고, 있으면 전부 유효해야 게시된다.
  `proxyRecipePath`는 **입력에 없다** — host가 번들의 XMP 경로를 직접 기록해 경로 탈출 여지를 없앴다.
- **T3 렌더 진입점** — `render_display_proxy_to_path_in_dir`. 목표 크기를 호출자가 주고,
  `--upscale false`를 넣고, worker root를 `.boothy-darktable/display-proxy/`로 분리했다.
  **384px 상수와 기존 preview/final 인자는 그대로다** (전용 테스트로 고정).
- **T4 게시** — `proxy_publisher`는 **Story 7.2의 `publish_generation_in_dir`을 그대로 호출한다.**
  게시 순서를 두 벌 만들지 않았다. 자격 판정 7종, source 크기 판정을 **렌더 전에** 수행한다.
  lane 기본값은 `off`이며 알 수 없는 값도 `off`다.
- **T5 화해** — host `evaluate_admission`과 viewer `shouldAdvanceDisplay`에 **같은** preset/capture
  규칙을 넣었다. 한쪽에만 넣으면 pointer와 화면이 갈라진다.
- **T6 계측** — `viewer-present.jsonl`을 그대로 재사용하고 tier·provenance 요약과 proxy 구간
  span 4종을 추가했다. **새 파일을 만들지 않았고**, `source-comparison.jsonl`과 합치는 도구도 만들지 않았다.
- **T7 자동 검증** — 아래 수치 참조. scope guard는 체크박스가 아니라 **실제 grep으로 확인**했다.
- **T8 부분** — HV-15 절차와 기계식 게이트. **합성 fixture로 게이트 자체를 검증했다**
  (정상 회차 2건 통과 + 심어 둔 결함 11종 전부 차단).
- **T9 문서** — 계약 5종 + architecture implementation note + ledger row.

#### 검증 결과 (있는 그대로)

| 명령 | 결과 | 판정 |
| --- | --- | --- |
| `cargo fmt --check` | diff 없음 | 통과 |
| `cargo test` (lib + 통합) | `viewer_display` **35/35**, lib unit **151/151** | 통과 |
| `cargo test --test capture_readiness -- --test-threads=1` | **60 passed / 0 failed** | 병렬 실행 시의 11~14건 실패는 render 경로 공유 자원 경합이며 이번 변경과 무관하다 |
| `pnpm lint` | 통과 | 신규 경고 없음 |
| `pnpm test:run` | **903 passed / 2 failed** | 실패 2건은 기존 governance 검사(Story 1.4 문서의 기대 문구 부재)이며 본 트리와 중첩 worktree 사본에서 각각 1건씩이다 |
| Story 7.4 관련 Vitest | **270/270 통과** | 15 files |
| HV-15 게이트 self-test | **정상 2건 통과 + 결함 11건 차단** | 세로 contain 정상 회차 포함 |
| 실장비 telemetry gate | **committed 1 / terminal 1 / qualifying 1** | 완결성 통과 |
| `pnpm build` (`tsc -b`) | **실패 (20건 / 9파일)** | 아래 참조 |

**`pnpm build`에 대한 정직한 설명.** 이번 Story가 만든 파일에는 타입 오류가 **0건**이다.
남은 오류는 두 종류다.

1. 기존 오류: ReadinessScreen, capture-runtime.test, governance, operator diagnostics,
   active-preset, after-paint.test, DoubleBufferedPhoto.test.
2. TS2719 "Two different types with this name exist" — `use-display-pointer.test.tsx`와
   `DoubleBufferedPhoto.test.tsx`. **후자는 이번 Story가 한 줄도 건드리지 않았고**,
   `displayGenerationSchema`의 `superRefine`을 걷어내고 다시 측정해도 오류 수가 20으로 같았다.
   내 변경이 만든 것이 아니다.

**정확한 사전 baseline은 재구성할 수 없었다.** 착수 시점의 작업 트리가 이미 미커밋 상태
(7.2·7.3 작업 포함)여서 git으로 그 시점을 복원할 수 없다. 위 두 근거로 판정을 대신했다.

#### Scope guard 검증 (grep으로 실제 확인)

- `rawRefinedDisplay` / `resident_renderer` / `deadline_scheduler` — 구현 없음.
  주석 1곳과 **unknown tier 거부 테스트 fixture** 1곳뿐이다
- `RAW_PREVIEW_MAX_*` / `FAST_PREVIEW_RENDER_MAX_*` = 384 그대로.
  기존 preview invocation 인자 불변을 전용 테스트로 고정했다
- Story 7.3 계약 파일(`capture-source.ts`, `source_probe.rs`, `source_telemetry.rs`,
  `embedded_jpeg.rs`) — **이번 Story가 수정하지 않았다**
- `session-manifest.ts` / `session_manifest.rs` — 변경 없음
- 우선순위 스케줄러 없음. 큐 대기는 `proxyQueueWaitMicros`로 **재기만** 한다

#### 제품 경로 무영향의 근거

- `BOOTHY_DISPLAY_PROXY_MODE` 기본 `on`, 명시적 `off`와 알 수 없는 값은 `off`. off일 때
  `spawn_display_proxy_publication`은 스레드조차 띄우지 않는다
- lane을 켜면 이미 게시된 세 preset이 **정확 경로로 동작한다.** 근사가 아니라 오늘의
  preview/final과 같은 엔진·recipe·source이며 출력 크기만 화면에 맞춘 결과다
- `renders/previews/`의 384px booth 레일 경로는 그대로다
- darktable worker root가 분리되어 기존 preview/final 실행과 config·library를 다투지 않는다
- 새 Rust crate를 추가하지 않았다 → Story 7.7 offline 인벤토리 변화 없음

#### 판단이 필요했던 결정 (리뷰 대상)

- **`generation_publisher`를 새 모듈로 분리했다.** 게시 순서가 `sample_publisher`에 있으면
  proxy가 "sample" 모듈에 의존하게 된다. 두 lane이 같은 함수를 부르는 것이 이 Story의 핵심이라
  이름이 사실을 말하게 했다. `sample_publisher`는 lane 고유 설정만 남기고 삭제하지 않았다.
- **`RenderIntent`에 세 번째 변형을 넣지 않았다.** `RenderIntent`는 "어느 canonical 세션 산출물을
  만드는가"를 뜻하는데 proxy는 `renders/previews`도 `renders/finals`도 만들지 않는다.
  대신 실행에 필요한 것만 `RenderStage`(label + 고객 안전 문구)로 뽑아 공유했다.
- **`PublishRequest`에 lane 플래그를 명시적으로 실었다.** 이전에는 게시 함수 내부에서 환경 변수를
  읽어 snapshot 플래그를 만들었고, 그래서 테스트 결과가 프로세스 환경에 좌우됐다.
- **RAW source의 크기 판정을 `Unknown`으로 기록한다.** 센서 크기를 값싸게 알 수 없으므로
  "충분하다"고 가정하지 않고, 판정을 하지 않았다는 사실 자체를 남긴다.
- **`viewer-display-update` 봉투 버전은 올리지 않았다.** 봉투 자체의 모양이 바뀌지 않았고,
  안의 pointer가 자기 버전을 들고 다닌다.

#### 종료 및 다음 Story 인계

- HV-14 후보는 `Technology No-Go`, 운영 경로는 `raw-original + pinned darktable 5.4.1`로 확정됐다.
- 환경 정보는 확인 가능한 값과 제한 예외를 함께 기록했다.
- 현재 저조도는 2026-08-16 제품 예외로 허용했으며 고객 품질 위험과 원 수치를 유지한다.
- HV-15 `Go` 뒤 `BOOTHY_DISPLAY_PROXY_MODE` 기본값을 `on`으로 전환하고 테스트로 고정했다.
- Story 7.5 상주 렌더러는 자체 metrics/blind review와 exact darktable fallback 증거 없이는 채택하지 않는다.

### File List

**신규**

- `_bmad-output/planning-artifacts/sprint-change-proposal-20260816-011543-hv15-evidence-exception.md`
- `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/environment-completion-20260816.md`

- `src-tauri/src/display/generation_publisher.rs`
- `src-tauri/src/display/proxy_publisher.rs`
- `tests/hardware/display-proxy/hv-15/README.md`
- `tests/hardware/display-proxy/hv-15/check-proxy-evidence.ps1`
- `tests/hardware/display-proxy/hv-15/test-check-proxy-evidence.ps1`

**수정**

- `src-tauri/src/display/proxy_publisher.rs`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/tests/viewer_display.rs`
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/environment.md`
- `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/product-decision-20260816.md`
- `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/result.md`
- `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/regression/summary.md`

- `src/shared-contracts/schemas/viewer-display.ts` — proxy tier, provenance, v1 호환 reader, `presentTelemetryEnabled`
- `src/shared-contracts/schemas/preset-authoring.ts` — `proxyPublicationPayloadSchema`, publish input 확장
- `src/shared-contracts/dto/display.ts` — provenance 타입 별칭
- `src/shared-contracts/display.contracts.test.ts` — proxy 라운드트립, tier 불변식, v1 읽기
- `src/viewer-surface/services/display-fit.ts`, `display-fit.test.ts` — contain-no-upscale 세로 표시 정책
- `src/viewer-surface/components/DoubleBufferedPhoto.tsx`, `DoubleBufferedPhoto.test.tsx` — crop 없는 contain 표시
- `src/display-generation/services/display-guard.ts` — preset/capture guard
- `src/display-generation/services/display-guard.test.ts` — proxy 거부 매트릭스
- `src/display-generation/state/use-display-pointer.ts` — 새 플래그 수렴
- `src/display-generation/state/use-display-pointer.test.tsx` — v2 fixture
- `src/display-generation/state/use-measurement-lane-state.ts` — 계측 gate 근거 교체
- `src/display-generation/state/use-measurement-lane-state.test.tsx` — 계측 gate 구독·복구 회귀 검증
- `src/viewer-surface/ViewerSurface.tsx` — 계측 gate 근거 교체
- `src/viewer-surface/pending-present-report.ts`, `pending-present-report.test.ts` — 보정 대기 표시 증거 보존
- `src/viewer-surface/ViewerSurface.test.tsx` — 두 lane 플래그 fixture
- `src/viewer-surface/services/viewer-host-adapter.ts` — absent pointer 기본값
- `src-tauri/src/contracts/dto.rs` — proxy tier/provenance/거부 사유, publish payload와 검증
- `src-tauri/src/display/mod.rs` — `DisplayLaneFlags`, 신규 모듈 등록
- `src-tauri/src/display/display_artifact.rs` — `ActiveDisplay`, capture 좌표, preset/capture guard
- `src-tauri/src/display/proxy_publisher.rs` — 세로 raster contain 자격 판정
- `src-tauri/src/display/sample_publisher.rs` — lane 설정만 남기고 게시 순서 이관
- `src-tauri/src/render/mod.rs` — `RenderStage`, display-proxy 진입점과 invocation
- `src-tauri/src/preset/preset_bundle.rs` — `proxyPublication` 로딩, 강등 사유, 정확 경로 자격
- `src-tauri/src/preset/default_catalog.rs` — 내장 preset 3종의 명시적 수동 승인 정보와 legacy 보강
- `src-tauri/src/preset/authoring_pipeline.rs` — publish 시 proxy 블록 검증·기록
- `src-tauri/src/commands/display_commands.rs` — proxy lane 진입점, 계측 등록 공통화
- `src-tauri/src/commands/capture_commands.rs` — 촬영 확정 시점 트리거
- `src-tauri/src/viewer/present_telemetry.rs` — tier·provenance·proxy span
- `src-tauri/tests/viewer_display.rs` — Story 7.4 회귀 8건
- `src/session-domain/state/session-provider.tsx`, `session-provider.test.tsx` — 실장비에서 발견한 삭제 실패 수정과 회귀 검증
- `src-tauri/tests/operator_audit.rs`, `src-tauri/tests/preset_authoring.rs` — publish input 필드
- `docs/contracts/viewer-display.md`, `preset-bundle.md`, `authoring-publication.md`,
  `render-worker.md`, `capture-source.md`
- `_bmad-output/planning-artifacts/architecture.md` — Story 7.4 implementation note
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` — Story 7.4 행
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 상태 전환
- `_bmad-output/implementation-artifacts/7-4-화면-적합-immutable-preset-proxy.md` — 이 파일
- `tests/hardware/display-proxy/run-20260815-120737-hv15-portrait-contain/` — 실장비 증거 패키지
- `tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/` — 조명·노출 무조정 재검증 증거 패키지

## Change Log

- 2026-08-16: 사용자 제공 18–55mm 번들렌즈와 기존 HV-14/Windows 근거로 환경 정보를 보완했다. 카드 미사용·PC 직접 저장, USB Port 6 직결·허브 없음, 1920×1080/60Hz/DPR1, sRGB 운영을 기록하고 읽을 수 없는 펌웨어·세부 리비전·HDR 및 별도 metrics/blind review는 사실을 만들지 않은 제한 예외로 승인받았다. HV-15를 `Go`, Story 7.4를 `done`으로 닫고 display proxy 기본값을 `on`으로 전환했다.
- 2026-08-16: Noah Lee가 선택지 1을 승인해 현재의 심한 저조도를 Story 7.4 범위의 제품 예외로 허용했다. 밝기 기준을 통과한 것으로 쓰지 않고 실측값을 보존했으며, 환경 fingerprint와 측정하지 않은 metrics/blind review는 여전히 열린 항목으로 유지한다.
- 2026-08-15: 사용자 요청에 따라 조명·노출을 바꾸지 않고 실장비를 재검증했다. 5/5 actual-present, 프리셋 전환, 삭제 후 standby, 재촬영 복구가 모두 통과했고 카메라 timeout은 없었다. 저조도는 재현됐으나 기능 판정과 분리해 기록했으며 전체 gate는 No-Go / `in-progress`로 유지한다.
- 2026-08-15: EOS 700D 실장비 재검증에서 세로 촬영본 contain-no-upscale 표시를 적용해 6/6 게시와 S1-S6를 확인했다. 현장에서 발견한 삭제 실패를 수정·재검증했고 renderer mismatch 및 새 세션 격리도 통과했다. 사진이 심하게 어둡고 HV-14/환경 fingerprint가 미완료라 HV-15와 Story 상태는 No-Go / `in-progress`로 유지한다.
- 2026-08-14: 코드 리뷰 안전 수정 7건을 적용했다. 내장 preset 3종에 사용자 지시 기반 수동 승인 정보를 명시하고, proxy 우선순위·viewer 문맥 재검증·게시 입력 검증·순서 좌표·계측 재수렴·actual-present 경계·pending 증거 보존을 보강했다. 측정하지 않은 visual approval metrics/blind review는 생성하지 않아 후속 작업으로 남기고 Status를 `in-progress`로 전환했다.
- 2026-08-14: Story 7.4 구현. `displayFitPresetProxy` tier 하나를 추가하되 **게시 순서는 Story 7.2의 것을 그대로 호출**하도록 `generation_publisher`로 분리했다. 착수 전 예상하지 못한 결함 두 가지를 구현 중 발견해 고쳤다: (1) 후보가 촬영 좌표를 들고 와도 host가 commit된 generation의 좌표를 기억하지 않아 `older-capture` guard가 발화하지 못했고, (2) viewer guard가 `undefined` provenance에서 throw해 화면 갱신이 멈출 수 있었다. darktable 5.4.1로 플래그를 실측 확정했다 — `--icc-intent PERCEPTUAL` 허용/`INTENT_PERCEPTUAL` 거부, 품질은 `plugins/imageio/format/jpeg/quality`, `--upscale false`에서 800×533 source는 800×533으로 남는다(크기 판정을 렌더 전으로 당긴 근거). `measurementLaneEnabled`와 `presentTelemetryEnabled`를 분리해 proxy lane 회차의 actual-present 0건 실패 모드를 막았다. 신규 자동 검증 43건(Rust 통합 11 + Rust 단위 19 + TS 13)과 HV-15 게이트 self-test 12 시나리오가 통과한다. lane 기본값은 `off`라 제품 경로는 변하지 않는다. HV-14 route 결정과 HV-15 실장비 회차가 남아 Status는 `review`다.
- 2026-08-14: **proxy 자격 게이트를 정정했다.** 리뷰 지적을 받아 재검토한 결과, `proxyCompatible` 승인은 원래 *다른 렌더링 엔진*(7.5)이 darktable을 근사하는 것을 막으려는 장치인데 이를 darktable 자신에게 적용하고 있었다. RAW 원본에서 같은 XMP로 렌더하는 것은 오늘의 정확 경로를 화면 크기로 만든 것일 뿐 근사가 아니며, 이를 "미승인"으로 막으면 AC 2가 요구하는 바와 정반대가 되고 게시된 preset이 하나도 통과하지 못해 HV-15 자체가 불가능해진다. 자격 판정을 source route로 나눠 정확 경로는 승인 없이 통과시키고, fast source나 다른 엔진일 때만 시각 승인을 요구한다. 두 근거를 `approvalBasis`로 자산·계측·HV 게이트에 기록해 evidence에서 구분되게 했다. 실측으로 확인: 이미 게시된 세 preset이 display-fit invocation에서 정확히 1620×1080과 서로 다른 룩을 만들고(mono-pop 평균 RGB 98,98,98), `darktable-cli`의 기본 JPEG 품질은 95다(품질 미지정 산출물과 바이트 동일).
- 2026-08-14: Story 7.4 컨텍스트 생성. Story 7.2가 남긴 게시 seam(`publish_generation_in_dir`)을 재사용하는 것을 설계의 축으로 고정하고, 추가되는 것을 tier 1개·provenance 1개·거부 사유 2개·렌더 진입점 1개·publisher 1개로 한정했다. 착수 전 반드시 알아야 할 세 가지를 Dev Notes에 고정했다: (1) HV-14가 `No-Go`라 승인된 fast source route가 아직 없으므로 기본 source를 `raw-original`로 두고 구현을 진행한다, (2) `--upscale false` 아래에서는 source 크기 미달이 렌더 후 폐기로 이어지므로 크기 판정을 **렌더 전으로** 당겨야 한다, (3) 현재 승격 guard가 preset을 보지 않아 세션 중 preset 변경 시 이전 preset의 늦은 proxy가 화면을 덮을 수 있다. 속도는 합격 조건이 아니며(AC 5), one-shot 3.53초 기준선을 정직하게 기록하는 것이 이 Story의 산출물임을 명시했다. Status `ready-for-dev`.
