# Viewer Display 계약 (Story 7.2 · 7.4 · 7.6)

## 목적

이 문서는 Story 7.2가 도입한 **immutable display generation**, **atomic display pointer**,
**opaque double-buffer swap**, **one-clock actual-present 계측**을 고정한다.

Story 7.1의 `docs/contracts/viewer-readiness.md`는 *사진이 놓일 자리와 필요한 픽셀 수*를 소유한다.
이 문서는 *그 자리에 무엇을 언제 올릴 수 있는가*를 소유한다.

**소유하지 않는 것:** camera source 선택(7.3), display-fit preset proxy 생성(7.4),
resident renderer(7.5), RAW 정밀본 **렌더**와 우선순위 scheduler(7.6 — `docs/contracts/render-worker.md`).

Story 7.6은 이 문서에 **`rawRefinedDisplay` tier와 그 승급 규칙**만 더한다.
정밀본을 *어떻게 만드는가*는 render worker 계약이 소유한다.

## 계측 lane은 제품 경로가 아니다

Story 7.2는 renderer를 교체하지 않고 표시 종단점을 증명한다. 그래서 표시되는 이미지는
preset 적용 결과가 아니라 **계측용 fixture**다.

| 환경 변수 | 기본값 | 의미 |
| --- | --- | --- |
| `BOOTHY_DISPLAY_SAMPLE_MODE` | `off` | `off` \| `visible-standby` \| `hidden-prewarm` |
| `BOOTHY_DISPLAY_SAMPLE_GAP_MS` | `800` | sample A → B 사이 간격 |
| `BOOTHY_DISPLAY_HIDDEN_PREWARM_STRATEGY` | `invisible` | `invisible` \| `offscreen` |

- 알 수 없는 값은 전부 `off`로 떨어진다. 오타가 lane을 켜지 못한다.
- `off`일 때 관람 화면 동작은 Story 7.1과 **완전히 동일**하고, booth는 계측 IPC를 하나도 하지 않는다.
- `measurementLaneEnabled`가 pointer snapshot에 실려 두 surface가 같은 truth를 본다.
- **켠 채로 출시하면 실제 고객이 fixture 사진을 본다.** FR-010과 NFR-004 위반이다.
  부팅 시 `display_sample_lane_enabled` 경고 로그가 남는다.

## Display Generation

immutable generation은 **커밋된 뒤에만** 존재한다. 커밋 전에는 어떤 경로로도 밖으로 나가지 않는다.

- `generationId`: `<requestId>-<seq:06>`
- `generationSeq`: 세션 안에서 단조 증가. 되돌아가지 않는다
- `sessionId` / `requestId` / `captureId`
- `viewerEpoch`: **request가 시작될 때** 관측한 viewer 세대
- `tier`: `sample`(7.2) \| `displayFitPresetProxy`(7.4) \| `rawRefinedDisplay`(7.6).
  **`final`은 tier가 아니다** (아래 「`final`이 display tier가 아닌 이유」)
- `assetPath`, `sourceWidthPx`, `sourceHeightPx`, `byteSize`
- `sourceHash`: `fnv1a64:<hex>`. 보안 해시가 아니라 provenance 확인용이라 알고리즘을 접두사로 드러낸다
- `sampleVariant`: `a` \| `b` \| `null`. **계측 fixture에서만 값을 갖는다**
- `proxyProvenance`: proxy generation에서만 존재한다 (아래)
- `committedAtHostMicros`: host monotonic clock

### tier별 필드 불변식 (강제)

tier마다 필요한 필드가 다르다. 비어 있어 기본값으로 통과하는 경로를 만들지 않는다.
TS는 `superRefine`, host는 게시 직전 `validate_shape`로 **같은 규칙**을 강제한다.

| tier | `sampleVariant` | `proxyProvenance` | `proxyProvenance.renderQuality` |
| --- | --- | --- | --- |
| `sample` | 필수 (`a`/`b`) | `null` | — |
| `displayFitPresetProxy` | `null` | 필수 (`residentProvenance`는 그 안에서 nullable) | `fast` |
| `rawRefinedDisplay` | `null` | 필수 | `high` |

fixture에 preset 출처가 실리면 거짓 출처가 evidence에 남고, provenance 없는 proxy는
"이 사진이 어느 촬영·프리셋의 결과인가"를 증명할 수 없다. 둘 다 게시 전에 거절된다.

**`renderQuality`가 tier와 어긋난 generation도 파일이 만들어지기 전에 거절된다.**
`--hq false` 프레임이 정밀본으로 게시되면 evidence는 "정밀본이 떴다"고 적고,
Story 7.6 AC 6의 tier 정당성 측정이 통째로 무의미해진다.

### `rawRefinedDisplay` tier (Story 7.6)

**같은 RAW를 `--hq true`로 다시 렌더한 display-fit 결과다.** HV-14 이후 proxy의 source가 이미
RAW original이므로 "RAW에서 온다"는 것은 더 이상 tier의 차이가 아니다.
차이는 **darktable pixelpipe의 downsampling 품질** 하나뿐이다.

| | `displayFitPresetProxy` | `rawRefinedDisplay` |
| --- | --- | --- |
| source | RAW original | **같은** RAW original |
| preset | capture-bound `presetId@presetVersion` XMP | **같은** XMP |
| renderer | pinned darktable 5.4.1 | **같은** pinned darktable 5.4.1 |
| 목표 크기 | viewer photoRect × DPR, `--upscale false` | **proxy generation에서 상속한 동일 값** |
| `--hq` | `false` | **`true`** |
| `renderQuality` | `fast` | `high` |

두 tier가 다른 필드로는 구분되지 않기 때문에 `renderQuality`가 계약에 있다.
**generation만 보고 두 tier를 구분할 수 있어야 한다.**

정밀본은 크기를 **렌더 시점 viewer에서 새로 읽지 않고 commit된 proxy generation에서 상속한다.**
게시 직전에 실측 크기가 활성 proxy와 정확히 같은지 다시 검증하며, 다르면
`refined-dimension-mismatch`로 거부한다.

### `final`이 display tier가 아닌 이유 (Story 7.6 「핵심 설계 결정 2」)

이 문서는 한때 "`rawRefinedDisplay`/`final`은 7.6이 추가한다"고 적고 있었다. **그 판단은 뒤집혔고,
조용히 빼지 않고 근거를 남긴다.**

- PRD FR-010이 명시하는 승급은 **display-fit preset 이미지 → RAW 정밀본** 둘뿐이다
- `final`은 5184×3456 전체 해상도 handoff 산출물이다. 이것을 1429×953 사진 영역에 올리면
  축소를 **브라우저가** 하게 되어 darktable 축소보다 품질이 낮으면서 decode 비용은 훨씬 크다.
  즉 "더 높은 tier"라는 주장이 화면에서 성립하지 않는다
- `--upscale false` 기반 display-fit 계약과 정면으로 충돌한다

`display.contracts.test.ts`가 `final`이 tier 목록에 없음을 테스트로 고정한다.

### `proxyProvenance` (Story 7.4)

AC 1이 요구하는 결속을 하나도 빠짐없이 자산에 싣는다.

| 필드 | 의미 |
| --- | --- |
| `presetId` / `presetVersion` | **capture record에 고정된** preset. live catalog pointer가 아니다 |
| `approvalBasis` | 신규 proxy admission은 명시적 `visual-approval`을 요구한다. `exact-reference-renderer`는 이전 evidence 호환을 위해 읽기만 유지한다 |
| `proxyRecipeVersion` | 승인된 versioned recipe |
| `referenceRenderer` / `referenceRendererVersion` | 화질 승인이 이뤄진 렌더러. 런타임 pin과 일치해야 한다 |
| `renderProfileId`, `outputColorSpace`, `jpegQuality` | 승인된 출력 프로필 |
| `sourceRoute` | 어느 source에서 렌더했는지. Story 7.3의 route 결정과 연결된다 |
| `sourceAssetHash` | **입력** source의 해시. generation의 `sourceHash`(결과 파일)와 다른 값이며 둘 다 필요하다 |
| `targetWidthPx` / `targetHeightPx` | 렌더 요청 시점의 목표 크기. viewer photoRect × DPR |
| `displayProfileId`, `devicePixelRatio` | 어느 화면 기준으로 만들어졌는지 |
| `residentProvenance` | **이 픽셀을 실제로 만든 renderer**. 참조 렌더러가 직접 만들었으면 `null` (Story 7.5, 아래) |
| `renderQuality` | `fast`(`--hq false`) \| `high`(`--hq true`). **두 tier를 구분하는 유일한 필드** (Story 7.6) |

### `residentProvenance` (Story 7.5)

`referenceRenderer`와 **다른 질문에 답한다.**

- `referenceRenderer`: 무엇과 비교해서 정확한가
- `residentProvenance.producerRenderer`: 누가 이 픽셀을 만들었는가

두 값을 하나로 합치면 evidence에서 darktable이 만든 프레임과 다른 엔진이 만든 프레임을
구분할 수 없다. 상주 후보가 만든 프레임에 `referenceRenderer: darktable`만 남기면 그 기록은 거짓말이다.

| 필드 | 의미 |
| --- | --- |
| `producerRenderer` / `producerRendererVersion` / `producerBuildId` | 실제 생산 엔진의 신원 |
| `executionMode` | `shadow`(비게시 비교) \| `evidence`(HV 회차). **`off`는 게시된 프레임에 나타날 수 없다** |
| `recipeSchemaVersion` / `compiledRecipeHash` / `programHash` | 계획과 shader 원문의 drift 검출 |
| `contextInitializedAtMicros` | 상주 context 생성 시각. **촬영보다 앞서야 한다** |
| `hotPathProgramCompileCount` / `hotPathProcessStartCount` | 측정 대상 hot path의 관측값. **0이 아니면 상주가 아니다** |
| `inputProvenance` | `predecoded-fixture` \| `real-capture-direct` \| `real-capture-via-one-shot` |
| `inputProducer` / `inputProducerVersion` | 입력 raster를 만든 주체 |
| `sourceReadyAtMicros` | 입력이 읽을 수 있는 상태로 도착한 시각. 지연 구간의 시작점 |
| `inputStartupCostMicros` | 입력을 만든 one-shot process의 startup 비용. **숨기지 않는다** |
| `productionEligible` / `adoptionBlockReason` | 채택 자격과 그 이유 (아래) |
| `gpuVendor` / `gpuRenderer` / `fallbackReason` | 실행 환경과 강등 사유 |

#### `productionEligible`은 주장할 수 없고 유도된다

fixture로 얻은 빠른 결과에 production 자격을 붙이는 것이 가장 쉬운 자기기만이다.
그래서 게시 경계 **앞에서** 막는다 — host의 `validate_resident_producer_provenance`와
TS의 `residentProducerProvenanceSchema`가 **같은 규칙**을 강제한다.

`productionEligible: true`가 통과하려면 전부 참이어야 한다.

1. `inputProvenance === 'real-capture-direct'`
2. `inputProducer`가 승인된 direct decoder 목록에 있다 — **이 목록은 현재 비어 있다**
3. `hotPathProgramCompileCount === 0` 이고 `hotPathProcessStartCount === 0`
4. `adoptionBlockReason === null`

`false`인 프레임은 반대로 `adoptionBlockReason`을 **반드시** 들고 있어야 한다.
이유 없는 강등은 evidence에서 조용한 무시와 구분되지 않는다.

### 저장 위치

```
<session_root>/renders/display/
├── .staging/<requestId>-<seq>.jpg     # 임시. 같은 볼륨이어야 rename이 원자적이다
├── <requestId>/<seq>-<variant>.jpg    # 확정. 절대 덮어쓰지 않는다
│                                     #   variant = sample이면 a/b, proxy면 `proxy`, 정밀본이면 `refined`
│                                     #   빈 문자열을 넘기면 `000001-.jpg`가 되어 아무도 못 읽는다
│                                     #   한 촬영에 proxy와 정밀본이 함께 존재하므로 두 이름이 달라야 한다
├── pointer.json                       # 활성 display truth
└── generations.jsonl                  # append-only 감사 기록 (승격과 거부 모두)
```

`assetProtocol.scope`의 `$PICTURE/dabi_shoot/**`에 이미 포함되므로 `tauri.conf.json` 수정이 필요 없다.

### session.json을 확장하지 않은 이유

아키텍처는 "session manifest는 기능마다 drift하지 않는다"를 규칙으로 둔다. `session.json`은
회귀 위험이 큰 공유 파일이므로 별도의 versioned display manifest(`pointer.json`, `viewer-display/v4`)를 쓴다.

### schemaVersion: v1 → v2 (Story 7.4) → v3 (Story 7.5) → v4 (Story 7.6)

- **v2**: generation이 tier별로 다른 모양을 갖게 됐다 (`sampleVariant` nullable, `proxyProvenance` 추가)
- **v3**: 프레임을 **만든** renderer와 **비교 기준** renderer를 분리해 기록한다 (`residentProvenance` 추가)
- **v4**: `rawRefinedDisplay` tier가 생겼고, 두 tier를 generation만 보고 구분할 수 있어야 한다
  (`renderQuality` 추가)

쓰기는 `viewer-display/v4`다. **읽기는 v1·v2·v3 전부 받는다.**

- 한 HV 회차가 빌드 경계를 걸칠 수 있고, evidence 도구가 여러 버전이 섞인 디렉터리를 읽는다
- 파싱을 실패시키면 **오래된 표본이 분모에서 조용히 사라진다**
- Story 7.9의 pre-upgrade session 호환과 old generation pointer 복구가 이 규칙 위에 선다
- v1 행은 `tier: 'sample'`, `proxyProvenance: null`로 정규화한다
- v2 proxy 행은 `residentProvenance: null`로 정규화한다 —
  `null`은 "참조 렌더러가 직접 만들었다"는 뜻이므로 HV-15 회차를 잘못 읽지 않는다
- v1~v3 proxy 행은 `renderQuality: 'fast'`로 정규화한다. 그 시절 게시 경로는 proxy lane 하나였고
  항상 `--hq false`였으므로 **추측이 아니라 그 빌드가 실제로 한 일을 적는 것이다**
- TS는 `displayPointerSnapshotCompatSchema`, host는 새 필드의 `#[serde(default)]`가 그 역할을 한다
- 이벤트 봉투(`viewer-display-update/v1`)는 자기 모양이 바뀌지 않아 버전을 올리지 않는다.
  안의 pointer가 자기 버전을 들고 다닌다

### `measurementLaneEnabled` ↔ `presentTelemetryEnabled` (Story 7.4)

**두 값은 서로 다른 질문에 답한다. 하나로 합치면 안 된다.**

| 필드 | 질문 | proxy lane만 켰을 때 |
| --- | --- | --- |
| `measurementLaneEnabled` | 표시 중인 이미지가 계측용 fixture인가? | `false` |
| `presentTelemetryEnabled` | booth/viewer가 present 계측 IPC를 해야 하는가? | `true` |

Story 7.2는 booth/viewer의 계측 IPC를 `measurementLaneEnabled`로 gate했다. proxy lane은
fixture가 아니므로 그 값이 `false`이고, 그대로 두면 **actual-present 행이 0건**이 된다 —
HV-13B 1차가 정확히 그 실패 모드였다. 계측 gate는 `presentTelemetryEnabled`를 본다.
Story 7.4와 7.6은 이 파일을 확장해 재사용한다.

## Commit 순서 (강제)

한 단계라도 앞당기면 커밋되지 않은 generation이 밖으로 나간다.

1. staging 파일에 write → `flush()` → `sync_all()` → 핸들 drop
2. **구조 probe**: SOI(`FFD8`) + SOF에서 width/height + **EOI(`FFD9`) trailer** + byteSize
3. EXIF orientation이 있으면 `1`만 허용
4. 크기 검증: `object-fit: contain`에서 확대가 없도록
   `width >= requiredSourceWidthPx || height >= requiredSourceHeightPx`여야 한다.
   세로 사진은 높이 경계에 맞추고 좌우를 letterbox로 남기며, 임의 crop·upscale은 하지 않는다.
5. correlation 검증 (아래 표)
6. `rename(staging → 확정)`. 확정 경로가 이미 있으면 **하드 에러**
7. `rename(temp → pointer.json)`, `revision += 1`
8. `generations.jsonl` append
9. **그다음에야** `viewer-display-update` emit

### "decode"는 두 단계이고 둘 다 필수다

| 단계 | 위치 | 내용 | 실패 시 |
| --- | --- | --- | --- |
| 구조 decode | host `display/image_probe.rs` | SOI + SOF 크기 + EOI trailer + orientation | pointer commit 안 함 |
| 픽셀 decode | viewer `img.decode()` | 전체 픽셀 decode | swap 안 함, 현재 이미지 유지, `decode-failed` 보고 |

EOI trailer 확인이 partial file 방어의 실질적 근거다. signature만 보면 절반만 쓰인 파일도 통과한다.
`sync_all()` 이후에 읽어야 캐시에만 있는 상태를 "완료"로 오판하지 않는다.

`decode()`를 제공하지 않는 런타임에서는 **교체하지 않는다.** 완전 decode를 증명하지 못한 자산을
올리는 것은 opaque swap 계약 위반이다.

## 거부 사유 (전부 고유 코드)

| reason | 의미 |
| --- | --- |
| `partial-file` | EOI trailer 없음. 아직 다 쓰이지 않았거나 잘림 |
| `undecodable` | SOI/SOF 구조를 해석할 수 없음 |
| `orientation-unsupported` | EXIF orientation이 1이 아님 |
| `insufficient-dimensions` | 두 축이 모두 현재 photoRect보다 작아 contain 표시에도 upscale이 필요함 |
| `viewer-not-ready` | viewer가 아직 photo rect를 보고하지 않음 |
| `session-mismatch` | host가 고정한 세션과 다름 |
| `stale-epoch` | request 세대가 현재 viewer 세대와 다름 |
| `lower-generation` | seq가 현재 활성과 같거나 낮음 |
| `older-request` | 더 오래된 request가 더 높은 seq로 늦게 도착 |
| `older-capture` | **더 오래된 촬영**의 generation이 늦게 도착 (Story 7.4) |
| `preset-mismatch` | 같은 촬영인데 preset identity/version이 현재 활성과 다름 (Story 7.4) |
| `tier-downgrade` | **같은 촬영**을 낮은 tier로 되돌리려 함 (아래) |
| `refined-dimension-mismatch` | 정밀본의 실측 크기가 활성 proxy와 다르거나, 붙일 활성 proxy가 없음 (Story 7.6) |
| `refined-tier-not-justified` | **host 전용.** AC 6의 detail 축 gate가 `justified`를 기록하지 않음 (Story 7.6) |
| `unknown-generation` | 등록되지 않은 tier |
| `decode-failed` | viewer의 픽셀 decode 실패. 다시 활성화되지 않는다 |
| `present-unreported` | **host 전용.** commit·notify까지 끝났지만 유예 시간 안에 viewer의 terminal 보고가 오지 않음 |

조용한 무시는 없다. 모든 거부가 `generations.jsonl`에 남는다.

**`refined-look-drift`는 런타임 거부 사유가 아니다.** ΔE00은 두 raster를 픽셀 단위로 비교해야
나오는 값이라 게시 경로에서 계산할 수 없다. Story 7.6 T6의 evidence 판정 값이며,
벗어나면 tier 문제가 아니라 **AC 4의 전환 결함**으로 기록한다. 계약 enum에 넣으면
절대 발생하지 않는 코드가 거부 매트릭스에 남는다.

### `older-request`가 왜 따로 필요한가

sample A(req-1) → 새 촬영 req-2 → req-1의 sample B 순으로 도착할 수 있다. B는 seq가 더 높지만
더 오래된 request의 자산이다. seq만 보면 화면이 이전 촬영으로 되돌아간다.
host는 request 관측 순서를, viewer는 `committedAtHostMicros`를 기준으로 이를 막는다.

### `older-capture`가 왜 또 따로 필요한가 (Story 7.4)

Story 7.2의 fixture는 즉시 게시되므로 **완료 순서 = 촬영 순서**였다. proxy 렌더는 수 초가 걸리고,
`generationSeq`와 request 관측 순서는 **게시 시점**에 부여된다. 그래서 먼저 찍고 늦게 끝난 사진이
둘 다 더 큰 값을 들고 도착해 `lower-generation`도 `older-request`도 발화하지 않는다.

host는 **촬영이 확정된 시점**에 capture 순서 좌표를 고정하고(`DisplayState::observe_capture`),
게시 요청이 그 값을 실어 온다(`record_capture_order`). 이 좌표만이 완료 순서가 뒤바뀐 경우를 막는다.

### `preset-mismatch`가 왜 필요한가 (Story 7.4)

한 촬영에는 capture-bound preset이 정확히 하나다. catalog rollback(Story 4.4) 뒤 같은 촬영이
다른 version으로 재렌더되면, 고객은 자기 사진의 룩이 화면에서 바뀌는 것을 본다.
순서 문제가 아니라 정체성 문제이므로 `older-request`와 뭉뚱그리지 않는다.

### `tier-downgrade`는 **같은 촬영 안에서만** 판정한다 (Story 7.6)

tier가 둘뿐이던 시절에는 이 구분이 필요 없었다. sample lane과 proxy lane이 서로 배타적이라
서로 다른 촬영의 tier를 비교할 일이 없었기 때문이다.

`rawRefinedDisplay`(2)가 생기면서 이 구분이 **필수**가 됐다.

> 촬영 A가 정밀본(tier 2)까지 올라간 뒤 촬영 B의 proxy(tier 1)가 도착한다.
> tier만 비교하면 `tier-downgrade`로 거부되고 — **고객의 다음 사진이 화면에 영영 뜨지 않는다.**

다른 촬영의 낮은 tier는 하락이 아니라 **새 사진**이다. 촬영 사이의 순서는 `older-capture`와
`older-request` 좌표가 판정한다. capture 좌표가 없는 표본(계측 fixture, v1 generation)은
이전 동작을 그대로 유지한다.

### `refined-dimension-mismatch`가 AC 4를 만드는 방식 (Story 7.6)

정밀본이 활성 proxy와 **픽셀 크기가 정확히 같을 때만** 승급한다. 한 픽셀이라도 다르면
`object-fit: contain` 박스가 달라져 교체 순간 사진이 튄다 — 그것이 AC 4가 0으로 규정한
crop/scale 점프다. 그래서 크기는 렌더 시점 viewer가 아니라 **commit된 proxy generation에서
상속**하고, 게시 직전에 실측으로 다시 확인한다.

붙일 활성 proxy가 아예 없는 경우도 같은 사유로 거부한다. 맞출 geometry가 없을 뿐 아니라,
**정밀본은 승급이지 첫 성공 화면의 대체가 아니다** (UX-DR19). proxy 없이 정밀본만 뜨면
고객의 첫 화면이 3배 느려진다.

### `refined-tier-not-justified`가 host 전용인 이유 (Story 7.6)

AC 6의 detail 축(MTF50) gate는 **측정된 제품 결정**이고 host의 게시 경계에서만 평가된다.
generation에는 그 판정이 실려 있지 않으므로 viewer는 스스로 이 사유를 주장할 수 없다.
viewer는 gate를 통과한 generation만 보게 되어 결과적으로 같은 truth를 따른다.
(`present-unreported`가 host 전용인 것과 같은 방향의, 문서화된 비대칭이다.)

## 세션 경계 무효화 (NFR-004, 0 tolerance)

- `start_session`으로 세션이 바뀌면 pointer를 즉시 비우고 revision을 올린다.
- viewer epoch가 바뀌면 활성 generation을 **새 `requiredSource*` 기준으로 재검증**한다.
  크기가 부족해지면 pointer를 비운다. 이전 epoch의 통과를 상속하지 않는다.
- `delete_capture`는 해당 request의 generation 디렉터리를 지우고, 그것이 활성이면 pointer를 비운다.
  `generations.jsonl`은 append-only 감사 기록이므로 지우지 않는다.

## Opaque double-buffer swap

- 두 `<img>` 레이어는 **같은 box, 같은 `object-fit: contain`**을 쓴다. 하나라도 다르면 scale jump가 생긴다.
- 다음 이미지가 `decode()`를 끝내기 전까지 현재 이미지를 내리지 않는다.
- `z-index`만 한 번의 style commit으로 바꾼다. transition / opacity fade / spinner / placeholder를 쓰지 않는다.
- 이전 레이어의 `src`를 비우지 않는다. 레이어가 둘뿐이라 다음 교체에서 어차피 덮어써지며,
  중간에 비우면 빈 레이어가 생길 위험만 늘고 얻는 것이 없다.
- **cache buster(`?v=`)를 붙이지 않는다.** generation 경로가 이미 유일하다.

### 레이아웃 불변식 (Story 7.1 계약과의 접점)

- `.viewer-surface__photo`의 기하를 바꾸지 않는다. `use-viewer-readiness`가 이 요소를 실측해
  host 계약과 1px 허용오차로 비교하며, 어긋나면 `layout-ready`가 내려가 **촬영이 막힌다**.
- 이미지 레이어는 이 요소 **안쪽**에 absolute로만 들어간다.
- standby 문구를 `display: none`으로 없애지 않는다. flex 행이 사라지면 stage 높이 → photo rect가
  커져서 촬영 차단과 scale jump가 동시에 발생한다. `visibility: hidden`으로 자리를 유지한다.

## Transport: broadcast event + snapshot 재수렴

아키텍처는 ordered stream에 Tauri Channel을 권장한다. Story 7.2는 그럼에도 Story 7.1과 동일하게
**broadcast event + snapshot 재수렴 + monotonic guard**를 쓴다.

- 근거: "delayed / duplicated / out-of-order 업데이트가 화면을 되돌릴 수 없다"는 보장은 전송 순서가
  아니라 `shouldAdvanceDisplay` guard와 snapshot 재수렴에서 나온다. 전송 순서에 의존하는 설계가 더 약하다.
- viewer에 두 개의 서로 다른 수신 모델을 만들지 않는다.
- Story 7.6에서 tier가 늘어 순서 요구가 강해지면 Channel 전환을 재평가한다.

| command | 방향 | 설명 |
| --- | --- | --- |
| `get_viewer_display_state` | 양방향 | 현재 pointer snapshot. 재수렴 경계 |
| `publish_display_sample` | booth/도구 → host | 계측 lane이 켜진 경우에만. **`async` 필수** (파일 I/O) |
| `report_display_present` | viewer → host | 표시 결과와 viewer 측 span. `viewer-window` label만 허용 |
| `report_trusted_capture_input` | booth → host | 공식 KPI 시작점 |
| `stamp_clock_probe` | 양방향 | clock 보정 왕복 |

event: `viewer-display-update` (`viewer-display-update/v1`). 아키텍처의 `dot.case` 규칙 대신
코드베이스 관행인 kebab-case를 따랐다 (`viewer-readiness-update`와 동일한 결정).

## One-clock actual-present 계측

공식 KPI는 **trusted capture input → 물리 모니터의 qualifying frame**이며, 기준 clock은
host의 monotonic `Instant`다 (`viewer::current_monotonic_micros()`,
`current_monotonic_ms()`와 동일한 `OnceLock` 공유).

### 보정

booth WebView와 viewer WebView는 time origin이 다르다. 각 문서가 `stamp_clock_probe` 왕복 9회로
Cristian 방식 보정을 하고 **RTT가 가장 작은 표본만 채택**한다.

```
offset      = hostMonotonicMicros - (clientSent + clientReceived) / 2
uncertainty = (clientReceived - clientSent) / 2
```

재보정: viewer epoch 변경, 마운트(reload), 5분 주기.
**보정에 실패하면 표본을 보고하지 않는다.** 임의의 offset을 지어내면 계측 전체가 거짓이 된다.

### 보고 규칙 (강제)

```
qualifyingLatency = (present + presentUncertainty) - (input - inputUncertainty)
```

항상 가장 넓은 구간을 보고한다. 시작점 변환은 내림, 종료점 변환은 올림이다.
**어떤 반올림도 구간을 짧게 만들 수 없다.**

총 불확실도가 5ms를 넘는 표본은 `low-confidence`로 표시한다. **조용히 버리지 않는다** —
제외하면 결과가 실제보다 좋아 보인다.

### 종료점 측정

- 주 측정: swap을 커밋한 `requestAnimationFrame` 안에서 `MessageChannel` 태스크를 던져,
  해당 프레임이 compositor로 넘어간 직후에 stamp한다.
- 보조 진단: `<img elementtiming="viewer-qualifying-frame">`의 `renderTime`.
  **Windows에서 asset은 `http://asset.localhost`, 문서는 `http://tauri.localhost`라 cross-origin이다.**
  `Timing-Allow-Origin`이 없으면 `renderTime === 0`이 되고 `startTime`이 `loadTime`으로 대체된다.
  `isElementRenderTime`을 함께 기록하며, fallback 값은 **actual-present로 승격하지 않는다**.
- **compositor → photon 구간은 JS로 측정할 수 없다.** 소프트웨어 present는 추정치다.
  HV-13B가 고속 촬영으로 물리 오프셋을 측정하고 그것을 최종 KPI에 반영한다.

### 진단 span (어느 것도 KPI 종료점이 아니다)

`trustedInput`, `hostAccepted`, `sampleWriteStart`, `fileReady`, `probeOk`, `pointerCommitted`,
`eventEmitted`, `viewerReceipt`, `decodeStart`, `decodeEnd`, `swapCommitted`, `imgOnLoad`,
`actualPresent`, `elementTimingRender`

Story 7.4의 proxy 구간: `proxySourceReady`, `proxyQueueWait`, `proxyRenderStart`, `proxyProcessExited`

Story 7.6의 정밀본 구간과 스케줄러 구간: `rawRefinedEnqueued`, `rawRefinedQueueWait`,
`rawRefinedRenderStart`, `rawRefinedProcessExited`, `rawRefinedCommitted`, `rawRefinedPresented`,
`schedulerPriority`, `schedulerDeadline`, `schedulerDeadlineMissed`, `schedulerCoalescedCount`,
`schedulerPreemptedBy`, `renderQuality`

기록 위치: `<session_root>/diagnostics/viewer-present.jsonl` (세션 범위 → NFR-004 안전).
**새 파일을 만들지 않는다.** Story 7.6도 같은 파일과 같은 span 계약을 재사용한다.

#### `proxyQueueWaitMicros`의 의미가 Story 7.6에서 바뀌었다

**v3 이전 값은 승인 게이트의 mutex 시간이며 대기 시간이 아니다.** 그 시절 렌더 큐는
슬롯이 차 있으면 기다리지 않고 즉시 실패했으므로, 이 값은 구조적으로 거의 0이었다.
HV-15 evidence의 낮은 큐 대기값을 근거로 "큐는 문제가 아니다"라고 결론 내리면 안 된다 —
그것은 "대기가 없었다"가 아니라 **"대기라는 개념이 없었다"**는 뜻이다.

Story 7.6의 우선순위 스케줄러 이후 이 값은 **실제 대기 시간**이다.

#### 정밀본 present는 KPI 종료점이 아니다

**공식 KPI는 그대로 `trusted input → 첫 자격 frame actual present` 하나다.**
정밀본 present는 **두 번째 terminal 행**이며 5초를 넘어도 그대로 기록한다.

그래서 정밀본 행은 `qualifyingLatencyMicros`를 **갖지 않는다.** 채우면 evidence 도구가
첫 화면 지연과 승급 지연을 같은 분포에 넣게 되고, NFR-003 판정이 조용히 거짓이 된다.
정밀본의 종료점은 `rawRefinedPresentedAtMicros`에 그대로 남는다.
`check-telemetry-completeness.ps1`이 이 규칙을 기계적으로 강제한다.

### 계측 완결성 (강제)

**commit된 generation 하나당 terminal 행이 정확히 하나 남는다.** 이것이 evidence의 분모다.

행이 아예 없는 generation이 생기면 두 가지를 구분할 수 없게 된다: 화면에 올라가지 않은 것인지,
올라갔는데 보고가 유실된 것인지. 그리고 분모가 조용히 줄어 성공률이 실제보다 좋아 보인다.
2026-08-12 HV-13B offscreen 회차에서 10개 중 1개가 정확히 이렇게 사라졌다.

| 도착 순서 | terminal 행 |
| --- | --- |
| viewer 보고가 먼저 | 그 보고가 terminal 행이다 |
| 유예 시간(`PRESENT_REPORT_GRACE_MICROS`, 2초)이 먼저 | host가 `present-unreported` 행으로 닫는다 |
| trusted input을 기다리며 보류된 보고가 있음 | 그 보고를 기록한다. 종료점은 실제로 관측했고 시작점만 없다 |
| terminal 행 이후에 보고가 도착 | 행을 더 만들지 않고 `display_present_after_terminal_record`로 로그에 남긴다 |

- `present-unreported`는 **host만 쓸 수 있는 사유**다. viewer가 스스로 주장할 수 있는 결론이 아니므로
  viewer → host 계약(`displayRejectReasonSchema`)에서는 거부된다.
- 이 행의 `actualPresentAtMicros`는 `null`이고 `confidence`는 `unreported`다. 추정값을 넣으면
  KPI가 거짓이 된다. **KPI 계산에서는 제외하되 분모에는 포함한다.**
- 보고가 오지 않은 이유가 창 이벤트일 수 있으므로 `viewerWindowEventsAfterInput`을 함께 센다.
- 닫는 시점: 다음 generation commit, 세션 교체(이전 세션 것 전부), `delete_capture`(그 request 전부),
  그리고 마지막 generation을 위해 lane 스레드가 유예 시간 뒤에 직접 sweep한다.

**viewer 쪽 대응 규칙:** 교체가 커밋된 뒤의 after-paint 스탬프는 effect 정리에서 취소하지 않는다.
프레임은 이미 표시되므로 그 표본의 terminal 보고는 반드시 나가야 한다. 취소는 언마운트에서만 한다.

#### Story 7.6: 한 촬영이 generation을 둘 만든다

완결성 계약 자체는 **바뀌지 않는다** — commit된 generation 하나당 terminal 행 하나다.
proxy와 정밀본은 각자의 유예 시간을 갖고 각자 닫히므로, 정밀본을 기다리는 동안
proxy 행이 조기 마감되지 않는다.

달라지는 것은 **KPI 분모**다. 정밀본 행은 첫 자격 frame이 아니므로 qualifying 표본에 넣지 않는다.
넣으면 같은 촬영이 두 번 세어지고, 정밀본을 게시하지 않은 회차와 비교가 불가능해진다.

Story 7.6이 함께 닫은 두 결함:

- **보고한 viewer 세대와 generation의 세대를 대조한다.** 재생성된 창의 늦은 보고가 이전 세대의
  generation과 짝지어져 "표시됐다"로 집계되던 경로를 막는다. 한 촬영에 generation이 둘이 되면서
  잘못된 짝짓기 위험이 실제로 커졌다. **행을 없애지는 않는다** — `stale-epoch` 거부로 확정한다.
- **terminal evidence 쓰기 실패를 성공으로 반환하지 않는다.** 그 행은 이미 `terminal_recorded`로
  표시됐으므로 쓰기가 실패하면 영영 남지 않는다. HV-17B의 완결성 판정이 이 반환값 위에 선다.

## Visible standby vs hidden prewarm A/B

| 변형 | 구현 |
| --- | --- |
| `visible-standby` | Story 7.1 그대로. 세션 시작부터 승인 모니터에 보인다 |
| `hidden-prewarm` + `invisible` | `visible(false)`로 생성, 첫 qualifying generation에서 노출 |
| `hidden-prewarm` + `offscreen` | 승인 모니터 밖 좌표에 배치, 첫 generation에서 복귀 |

**알려진 충돌:** WebView2는 숨김/가려진 창의 타이머와 `requestAnimationFrame`을 throttle한다.
Story 7.1의 liveness는 1초 heartbeat + 5초 staleness 임계이므로, 숨긴 창은 `stale-report`가 되어
**촬영이 막힐 수 있다.** 이는 버그가 아니라 Story 7.1이 의도한 정직한 차단이다.

- `VIEWER_REPORT_STALE_AFTER_MS`를 늘려 회피하지 않는다. 자동 테스트가 이 값을 고정한다.
- rAF가 멈추면 present 계측 자체가 성립하지 않는다. 그 관측이 A/B의 결과다.
- 창을 드러내는 비용은 present 계측에 그대로 포함된다.

**노출은 viewer 세대당 정확히 한 번이다.** generation마다 `set_position` · `show` · `set_fullscreen`을
다시 걸면 측정 구간 한가운데에서 창 상태가 흔들리고, 그때 발생하는 layout 재보고가 viewer의
present 계측을 무너뜨린다. 창이 재생성되면 새 세대는 다시 숨은 상태로 시작하므로 그때만 다시 드러낸다.

### 승인된 기본값: `visible-standby` (2026-08-11 HV-13B)

승인 PC·Canon EOS 700D·승인 DISPLAY3에서 변형당 warm-up 5회 + 측정 30회를 randomized 순서로 실행했다.

| 변형 | combined p50 | p95 | max |
| --- | --- | --- | --- |
| `visible-standby` | 3710.617ms | 4280.416ms | 4672.717ms |
| `hidden-prewarm` (`invisible`) | 3774.393ms | 4540.441ms | **15024.476ms** |

- **`visible-standby`를 기본값으로 유지한다.** hidden 변형은 p95가 더 느렸고 15.024초 outlier가
  있었으며, 이 WebView2·모니터 구성에서 신뢰성 이점이 없다.
- `hidden-prewarm`의 `offscreen` 방식은 2026-08-12에 별도로 실장비 검증했다. 표시 경로는 동작했지만
  기본값을 바꿀 근거는 나오지 않았다.
- 이 값들은 **표시 종단점의 기준선**이지 제품 SLA 통과 근거가 아니다. 실제 preset 결과의 latency
  판정은 Story 7.4/7.8이 소유한다.
- 원본: `tests/hardware/viewer-present/run-20260811-203500-hv13b-rerun/ab/summary.json`

## AC 4 실패 조건 계측

**trusted capture input 이후의 viewer window 생성이나 navigation은 실패 결과다.**

`get_capture_readiness`가 `ensure_viewer_window_state`를 호출하고 booth가 이 command를 주기적으로
조회하므로, 촬영 중 poll이 창을 재생성할 수 있는 경로가 실제로 존재한다.

host는 창 생성마다 카운터를 올리고, present 보고 시 trusted input 이후의 증가분을
`viewerWindowEventsAfterInput`으로 기록한다. 0이 아니면 그 표본은 실패다.

## 참고

- Story 7.1 계약: `docs/contracts/viewer-readiness.md`
- Story: `_bmad-output/implementation-artifacts/7-2-immutable-sample과-actual-present-계측.md`
- HV-13B 증거: `tests/hardware/viewer-present/hv-13b/README.md`
