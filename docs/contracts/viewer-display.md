# Viewer Display 계약 (Story 7.2)

## 목적

이 문서는 Story 7.2가 도입한 **immutable display generation**, **atomic display pointer**,
**opaque double-buffer swap**, **one-clock actual-present 계측**을 고정한다.

Story 7.1의 `docs/contracts/viewer-readiness.md`는 *사진이 놓일 자리와 필요한 픽셀 수*를 소유한다.
이 문서는 *그 자리에 무엇을 언제 올릴 수 있는가*를 소유한다.

**소유하지 않는 것:** camera source 선택(7.3), display-fit preset proxy 생성(7.4),
resident renderer(7.5), RAW 정밀본 tier와 deadline scheduler(7.6).

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
- `tier`: Story 7.2는 `sample` 하나만 등록한다 (7.4/7.6이 상위 tier 추가)
- `assetPath`, `sourceWidthPx`, `sourceHeightPx`, `byteSize`
- `sourceHash`: `fnv1a64:<hex>`. 보안 해시가 아니라 provenance 확인용이라 알고리즘을 접두사로 드러낸다
- `sampleVariant`: `a` \| `b`
- `committedAtHostMicros`: host monotonic clock

### 저장 위치

```
<session_root>/renders/display/
├── .staging/<requestId>-<seq>.jpg     # 임시. 같은 볼륨이어야 rename이 원자적이다
├── <requestId>/<seq>-<variant>.jpg    # 확정. 절대 덮어쓰지 않는다
├── pointer.json                       # 활성 display truth
└── generations.jsonl                  # append-only 감사 기록 (승격과 거부 모두)
```

`assetProtocol.scope`의 `$PICTURE/dabi_shoot/**`에 이미 포함되므로 `tauri.conf.json` 수정이 필요 없다.

### session.json을 확장하지 않은 이유

아키텍처는 "session manifest는 기능마다 drift하지 않는다"를 규칙으로 둔다. `session.json`은
회귀 위험이 큰 공유 파일이므로 별도의 versioned display manifest(`pointer.json`, `viewer-display/v1`)를 쓴다.
Story 7.4와 7.6은 이 파일을 확장해 재사용한다.

## Commit 순서 (강제)

한 단계라도 앞당기면 커밋되지 않은 generation이 밖으로 나간다.

1. staging 파일에 write → `flush()` → `sync_all()` → 핸들 drop
2. **구조 probe**: SOI(`FFD8`) + SOF에서 width/height + **EOI(`FFD9`) trailer** + byteSize
3. EXIF orientation이 있으면 `1`만 허용
4. 크기 검증: `width >= requiredSourceWidthPx && height >= requiredSourceHeightPx`
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
| `insufficient-dimensions` | 현재 photoRect를 upscale 없이 채울 수 없음 |
| `viewer-not-ready` | viewer가 아직 photo rect를 보고하지 않음 |
| `session-mismatch` | host가 고정한 세션과 다름 |
| `stale-epoch` | request 세대가 현재 viewer 세대와 다름 |
| `lower-generation` | seq가 현재 활성과 같거나 낮음 |
| `older-request` | 더 오래된 request가 더 높은 seq로 늦게 도착 |
| `tier-downgrade` | 낮은 tier로 되돌리려 함 |
| `unknown-generation` | 등록되지 않은 tier |
| `decode-failed` | viewer의 픽셀 decode 실패. 다시 활성화되지 않는다 |
| `present-unreported` | **host 전용.** commit·notify까지 끝났지만 유예 시간 안에 viewer의 terminal 보고가 오지 않음 |

조용한 무시는 없다. 모든 거부가 `generations.jsonl`에 남는다.

### `older-request`가 왜 따로 필요한가

sample A(req-1) → 새 촬영 req-2 → req-1의 sample B 순으로 도착할 수 있다. B는 seq가 더 높지만
더 오래된 request의 자산이다. seq만 보면 화면이 이전 촬영으로 되돌아간다.
host는 request 관측 순서를, viewer는 `committedAtHostMicros`를 기준으로 이를 막는다.

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

기록 위치: `<session_root>/diagnostics/viewer-present.jsonl` (세션 범위 → NFR-004 안전)

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
  그리고 마지막 generation을 위해 sample lane 스레드가 유예 시간 뒤에 직접 sweep한다.

**viewer 쪽 대응 규칙:** 교체가 커밋된 뒤의 after-paint 스탬프는 effect 정리에서 취소하지 않는다.
프레임은 이미 표시되므로 그 표본의 terminal 보고는 반드시 나가야 한다. 취소는 언마운트에서만 한다.

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
