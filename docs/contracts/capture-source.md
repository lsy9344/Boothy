# Capture Source 계약 (Story 7.3)

같은 촬영에서 **가장 빠르고 신뢰할 수 있는 JPEG source**를 얻는 경로를 비교하기 위한 계약이다.

이 문서는 **측정 계약**이지 제품 계약이 아니다. 여기 정의된 어떤 route도 HV-14가 결정을
기록하기 전까지 제품 경로로 승격되지 않는다.

## 이 Story가 하지 않는 것

- display proxy 생성과 관람 화면 게시 → Story 7.4. **이 Story는 source를 화면에 올리지 않는다.**
- resident display renderer → Story 7.5
- RAW 정밀본 tier와 deadline scheduler → Story 7.6
- Story 7.2의 display generation/pointer/swap/telemetry 계약 변경
- `session.json` 스키마 확장

## 비교 대상 route

| route | 설명 | 상태 |
| --- | --- | --- |
| `embedded-jpeg` | RAW 컨테이너에 내장된 full-size JPEG을 추출 (Route A) | 구현됨 (HV-14 미검증) |
| `camera-paired-jpeg` | 카메라가 RAW와 함께 만든 별도 JPEG transfer object (Route B) | 구현됨 (HV-14 미검증) |
| `windows-shell-thumbnail` | 현재 제품에 살아 있는 incumbent (Route C) | **기준선.** 이미 제품에 있음 |

> **incumbent가 1급 값으로 등록되어 있는 이유.** AC는 두 route를 말하지만 제품에는 이미
> 세 번째 source가 살아 있다. 이것을 같은 조건에서 함께 재지 않으면 "새 route가 더 빠르다"는
> 주장에 비교 대상이 없다.

### incumbent의 실제 정체

현재 제품의 fast preview는 **Windows Shell 썸네일**이다
(`WindowsShellThumbnail.TrySavePreviewJpeg` → `IShellItemImageFactory.GetImage`).

- 요청: `1600×1600`, `ResizeToFit | BiggerSizeOk | ThumbnailOnly`
- `BiggerSizeOk`는 요청보다 큰 캐시본을 허용하므로 **실제 산출 크기가 1600과 다를 수 있다.**
  display-fit 판정(Story 7.4)의 입력은 요청값이 아니라 실측값이어야 하므로,
  `WindowsShellThumbnail.Measure`가 표본마다 실제 픽셀 크기와 추출 비용을 함께 돌려준다.
- host의 fast preview 대기 예산은 `HELPER_FAST_PREVIEW_WAIT_MS = 120`이다.
  **이 값을 늘려 incumbent를 통과시키지 않는다.** 2026-08-12 evidence에서 이 예산이 5/5
  소진됐다는 사실 자체가 incumbent의 측정 결과다.

`fastPreviewKind`의 허용 값과 각 값의 생산 경로는
[`camera-helper-sidecar-protocol.md`](./camera-helper-sidecar-protocol.md#fastpreviewkind-허용-값)에 있다.

## 거부 사유

모든 거부 경로가 고유 코드를 가진다. **조용한 무시는 허용하지 않는다** — 성공률 분모가
소리 없이 줄면 결과가 실제보다 좋아 보인다.

| 사유 | 뜻 |
| --- | --- |
| `absent` | 후보 자체가 없다 |
| `partial` | EOI trailer가 없다. 아직 다 쓰이지 않았거나 잘렸다 |
| `corrupt` | 기록된 크기·해시가 파일과 어긋난다 |
| `undecodable` | SOI/SOF 구조를 해석할 수 없다 |
| `orientation-unsupported` | EXIF orientation이 `1`이 아니다 |
| `wrong-session` | 다른 세션의 산출물 (NFR-004) |
| `wrong-request` | 다른 request의 산출물 |
| `wrong-capture` | 다른 capture의 산출물 |
| `stale` | 이 request가 시작되기 전에 만들어진 잔재 |
| `unsupported-combination` | 카메라 descriptor에 없는 조합이라 **시도하지 않았다** |
| `extraction-failed` | 추출을 시도했으나 실패했다 |
| `cancelled` | 진행 중이던 전송이 취소됐다 |

> `unsupported-combination`과 `extraction-failed`는 다른 결과다. 시도해서 실패한 것과
> 지원되지 않아 시도하지 않은 것을 섞으면 "카메라가 지원하지 않는다"와 "우리 구현이
> 실패했다"를 evidence에서 구분할 수 없다.

### JPEG 구조 판정 규칙은 한 곳에만 있다

`src-tauri/src/display/image_probe.rs`(Story 7.2)가 SOI / SOF / EOI trailer / EXIF orientation
판정을 소유한다. Story 7.3의 `capture::source_probe`는 그 함수를 **직접 호출**하고 결과를
자기 거부 사유로 옮기기만 한다.

규칙을 두 벌 만들면 Story 7.4에서 통과 기준이 갈라진다. 코드 구조로 그것을 막는다.

## 측정 lane

`BOOTHY_SOURCE_COMPARE_MODE` = `off` | `embedded` | `paired` | `shell` | `ab` (**기본 `off`**)

RAW persistence와 manifest 반영이 끝난 직후 host의 실제 촬영 완료 경로가 lane을 한 번 실행한다.
`off`이면 파일 읽기·디렉터리 생성·표본 기록을 전혀 하지 않는다. lane 오류는 진단 로그에
남지만 이미 저장된 RAW 성공을 되돌리거나 촬영 실패로 바꾸지 않는다.

- **`off`일 때 제품 경로는 지금과 완전히 동일하게 동작한다.** 알 수 없는 값은 전부 `off`로
  떨어진다 — 오타가 lane을 켜는 일은 없어야 한다.
- Story 7.2의 `BOOTHY_DISPLAY_SAMPLE_MODE`와 **독립적인 스위치**다. 두 lane을 동시에 켜면
  표본이 서로 오염되므로, 동시 활성화 시 경고를 남기고 source lane을 우선한다.
- `ab` 모드는 **세 route를 모두** 잰다. incumbent를 빼지 않는다.

### 산출물 경로

측정 산출물은 `<session_root>/renders/sources/`에만 쓴다.

**`renders/previews/`(canonical preview 경로)를 절대 건드리지 않는다.** 그 경로는 이미
`SessionPreviewImage`가 booth 사진 레일에 표시하므로, 측정 산출물을 그곳에 쓰면 lane이
제품 UI를 조용히 바꾼다.

### 표본 기록

`<session_root>/diagnostics/source-comparison.jsonl` (`source-comparison/v1`)

- Story 7.2의 `viewer-present.jsonl`과 **의도적으로 분리된 파일이다.**
- **두 파일을 합쳐 집계하는 도구를 만들지 않는다.** 합치는 순간 보정되지 않은 JPEG의 빠른
  도착 시각이 preset-applied KPI를 실제보다 좋아 보이게 만든다.
- 모든 표본에 `isPresetApplied: false`가 명시적으로 실린다. 스키마가 `z.literal(false)`이므로
  `true`인 표본은 파싱 자체가 실패한다. 필드가 없어서 기본값으로 통과하는 경로도 없다.
- **모든 시도에 행이 하나씩 남는다.** 거부된 시도도, 후보가 아예 없던 시도도 남는다.
  Story 7.2는 terminal 행이 없는 산출물 때문에 한 회차를 잃었다.
- 후보가 생성되지 않아 `assetPath`가 `null`이면 `widthPx`, `heightPx`, `byteSize`,
  `objectIndex`도 `null`이다. 결측을 숫자 `0`으로 꾸며 기록하지 않는다.

### AB/BA 배치

`block_order_for(seed, blockIndex)`가 host에서 결정하고 표본에 `randomizationSeed`와
`blockIndex`를 남긴다. **사람이 순서를 고르면 편향이 들어간다.** seed가 있으면 배치를
그대로 재현할 수 있다.

## Route B — capability-gated RAW+JPEG

### capability는 가정하지 않고 런타임에서 읽는다

`EdsGetPropertyDesc(PropID_ImageQuality)`의 지원 목록이 **유일한 truth**다.

- descriptor에 없는 조합은 **시도하지 않고** `unsupported-combination`으로 기록한다.
- **EOS 700D가 RAW+small JPEG를 지원한다고 가정하지 않는다.**
  `ImageQualityValue.SelectRawPlusJpegCandidate`는 descriptor가 준 목록 안에서만 고르고,
  목록에 RAW+JPEG가 없으면 `null`을 돌려준다. 대체 조합을 지어내지 않는다.
- 측정이 끝나면 **원래 값으로 되돌린다.** 카메라를 측정 상태로 남기면 다음 고객 세션의
  제품 동작이 바뀐다.

`EdsImageQuality` 32비트 값의 해석은 `ImageQualityValue`에 있다.

| 비트 | 뜻 |
| --- | --- |
| 24–31 | 첫 이미지 크기 |
| 16–23 | 첫 이미지 형식 (RAW `0x64`, CRAW `0x63`) |
| 8–15 | 둘째 이미지 크기 (`0xFF` = 없음) |
| 0–7 | 둘째 이미지 형식 (JPEG `0x10`/`0x13`/`0x12`, `0x0F` = 없음) |

RAW+HEIF(`0x00640080` 등)는 JPEG가 아니므로 이 실험의 대상이 아니다.

### multi-object correlation

**이전 구현의 결함이 이 Story의 출발점이다.** `CanonSdkCamera.HandleObjectEvent`는
`Interlocked.Exchange(ref captureContext.DownloadStarted, 1)`로 capture당 **첫 transfer object
하나만** download하고 나머지를 아무 기록 없이 `EdsRelease`했다. RAW+JPEG를 켜면 카메라가
object event를 두 번 올리는데 두 번째가 거기서 사라진다. 그 상태로 측정하면
**"카메라가 RAW+JPEG를 지원하지 않는다"는 잘못된 결론**이 나온다.

`CaptureObjectCorrelator`가 이것을 고친다.

- 완료 판정이 **object 단위가 아니라 request 단위**다 (`expectedObjectCount` / `AcceptedObjects`).
- `EdsDirectoryItemInfo.GroupID`로 같은 촬영의 object를 묶는다.
- `GroupID`가 `0`이거나 없으면 **파일명 stem + 도착 시각 창**(기본 5초)을 보조 correlation으로
  쓰고, `usedFallbackCorrelation: true`로 그 사실을 표본에 남긴다. correlation 근거가 약하다는
  것이 결론에 그대로 드러나야 한다.
- helper의 `source-object-arrived` event가 있으면 host가 실제 `objectIndex`와 `groupId`를 표본에
  옮긴다. event가 없는 구버전 helper에서는 captureId 기반 보조 correlation을 명시적으로 남긴다.
- 거부된 object도 사유와 함께 알린다 (`group-mismatch`, `correlation-failed`,
  `duplicate-raw-object`, `request-already-complete` 등). **조용히 버리지 않는다.**
- **in-flight capture는 계속 1개만 허용한다.** object가 여러 개일 뿐이다.
- **기본값은 RAW 한 개다.** `expectedObjectCount`가 1이면 JPEG/unknown object는 RAW로 오인하지
  않고 거부한 뒤 RAW object를 기다리므로, 측정 lane이 꺼져도 추가 source 파일을 만들지 않는다.

### object role은 확장자가 아니라 SDK 정보로 판정한다

`CaptureObjectCorrelator.ResolveRole`이 `EdsDirectoryItemInfo.format`을 먼저 본다.
EDSDK `ImageFormat_*` 코드와 PTP object format 코드를 모두 인식하고, 해석할 수 없으면
파일 확장자로 내려간다. 둘 다 실패하면 `Unknown`을 돌려주고 **RAW 경로로 보내지 않는다.**

- `capturesOriginalsDir`에는 **RAW object만** 저장한다.
- JPEG object는 `renders/sources/`로 분리된다 (`DownloadPairedJpeg`).
- **RAW 확장자 기본값이 `.cr3` → `.cr2`로 바뀌었다.** 승인 하드웨어 EOS 700D가 실제로 만드는
  확장자다. 이전 기본값은 이 카메라가 만들지 않는 이름이었고, object가 둘이 되면 그 로직이
  JPEG를 RAW original로 저장할 수 있었다.
  **기존 `.cr3` 산출물은 그대로 읽힌다** — 바뀐 것은 새로 쓸 이름뿐이며, host에는 확장자를
  가정하는 코드가 없다.

### 부분 실패는 RAW truth를 무효화하지 않는다

Story 1.5~1.7이 만든 계약이다. RAW 저장이 곧 촬영 성공이고 fast source는 best-effort 부가물이다.

`DownloadPairedJpeg`는 `context.Completion`을 건드리지 않는다. 여기서 무엇이 실패하거나
취소되어도 이미 저장된 RAW의 성공 판정은 바뀌지 않는다. object가 두 개가 되어도
**RAW object 하나의 성공이 여전히 성공 기준**이다.

## Route A — 내장 JPEG 추출

> **의존성 결정 (2026-08-12, 승인됨):** LibRaw를 도입하지 않고 **CR2의 TIFF IFD를 직접 읽는다.**
>
> 이 저장소는 이미 새 crate 없이 이미지 구조를 파싱하고 있고(`display::image_probe`),
> CR2는 TIFF 기반 컨테이너이므로 내장 JPEG의 위치는 IFD#0이 그대로 가리킨다.
> 그 결과 **LGPL-2.1/CDDL license gate와 Story 7.7 offline clean-machine 인벤토리 영향이
> 둘 다 발생하지 않는다.** 새 Rust crate는 추가되지 않았다.
>
> route 이름이 `libraw-embedded-jpeg`가 아니라 `embedded-jpeg`인 이유가 이것이다.
> 구현에 없는 것을 계약이 주장하는 상황(`fastPreviewKind: "embedded-jpeg"` 사건)을
> 되풀이하지 않는다.

구현: `src-tauri/src/capture/embedded_jpeg.rs`

### 추출 절차

1. **TIFF 헤더** — byte order(`II`/`MM`)와 IFD#0 offset을 읽는다. byte order를 가정하지 않는다.
2. **IFD#0 순회** — 다음 태그를 찾는다.
   - `0x0111 StripOffsets` — 내장 full-size JPEG의 시작 위치
   - `0x0117 StripByteCounts` — 그 길이
   - `0x0112 Orientation` — 컨테이너가 선언한 방향
3. **범위 검증** — strip이 파일 끝을 넘어가면 `corrupt`. 조용히 잘라 쓰지 않는다.
4. **구조 검증** — 꺼낸 바이트를 **Story 7.2의 `probe_jpeg`에 그대로 넣는다.**
   SOI / SOF 크기 / EOI trailer / EXIF orientation 판정이 전부 거기서 온다.
5. **orientation 확정** — JPEG 자체 EXIF가 있으면 그것이 우선, 없으면 TIFF IFD#0의 값을 쓴다.
   `1`이 아니면 `orientation-unsupported`.

### 거부 사유 매핑

| 상황 | 사유 |
| --- | --- |
| TIFF가 아님 (CR3의 ISO BMFF 포함) | `undecodable` |
| strip 태그가 없음 / 길이 0 | `absent` |
| IFD offset·strip 범위가 파일 밖 | `corrupt` |
| JPEG에 EOI trailer 없음 | `partial` |
| JPEG 구조 해석 불가 | `undecodable` |
| orientation ≠ 1 (JPEG EXIF 또는 컨테이너) | `orientation-unsupported` |

> **CR3는 이 파서의 대상이 아니다.** ISO BMFF 기반이라 구조가 완전히 다르다.
> 승인 하드웨어 EOS 700D는 CR2를 만들며, CR3가 들어오면 조용히 실패하지 않고
> `undecodable`로 거부한다. CR3 지원이 필요해지면 그때 별도 결정 사항이다.

### 왜 host Rust인가

당초 스토리는 추출을 helper(C#)에 두려 했고, 근거는 "host Rust에 native RAW 의존성을 들이면
Story 7.7 인벤토리와 Rust 빌드가 동시에 무거워진다"였다. **의존성이 없어지면서 그 근거가
사라졌다.**

host에 두는 것이 더 낫다.

- `image_probe`를 **직접 호출**할 수 있어 판정 규칙이 한 곳에 남는다
- sidecar 경계를 넓히지 않는다 (아키텍처의 "얇은 Canon adapter" 규정 유지)
- `cargo test`로 완전히 검증된다 — 카메라 없이 12개 경계 조건을 전부 단언한다
- 추출은 **읽기 전용**이다. RAW 파일을 열어 바이트를 훑을 뿐 어디에도 쓰지 않으므로
  실패가 RAW truth에 닿을 경로 자체가 없다

## 아키텍처 편차와 근거

아키텍처는 sidecar를 "얇은 Canon EDSDK adapter"로 규정한다. multi-object correlation과
paired JPEG 저장을 helper에 두는 것은 그 경계를 약간 넓히는 결정이다.

근거:

1. transfer object는 EDSDK 콜백 경로에서만 접근 가능하다. correlation을 host로 올리면
   object 수명 관리(`EdsRelease`)가 프로세스 경계를 넘어야 한다.
2. 순수 판정 로직(`CaptureObjectCorrelator`, `ImageQualityValue`)을 SDK 호출에서 분리해
   `dotnet test`로 검증 가능하게 만들었다. helper에 남은 것은 SDK 호출과 파일 I/O뿐이다.
3. session / preset / timing / UI truth는 여전히 helper 밖에 있다. helper는 여전히
   "무엇이 지금 진실인가"를 결정하지 않는다.

## 검증

| 대상 | 위치 |
| --- | --- |
| 거부 매트릭스 (12개 사유 전수) | `src-tauri/src/capture/source_probe.rs` 단위 테스트 |
| 내장 JPEG 추출 경계 (CR2/CR3/손상/orientation) | `src-tauri/src/capture/embedded_jpeg.rs` 단위 테스트 |
| lane 스위치·경로·기록 | `src-tauri/src/capture/source_telemetry.rs` 단위 테스트 |
| 경계 회귀 (lane off, RAW truth, 세션 격리, 행 누락) | `src-tauri/tests/capture_source.rs` |
| multi-object correlation | `sidecar/canon-helper/tests/CanonHelper.Tests/CaptureObjectCorrelatorTests.cs` |
| capability descriptor 해석 | `sidecar/canon-helper/tests/CanonHelper.Tests/ImageQualityCapabilityTests.cs` |
| TS ↔ Rust ↔ helper 문자열 고정 | `src/shared-contracts/capture-source.contracts.test.ts` |

**자동 테스트는 필요하지만 HV-14를 대신하지 않는다.** Story 1.9, 7.1(HV-13A), 7.2(HV-13B)가
모두 자동 검증 전부 통과 뒤 실장비에서 실패했다. HV-14가 route 결정을 기록하기 전까지
이 Story는 `review`에 머문다.

## 관련 문서

- [`camera-helper-sidecar-protocol.md`](./camera-helper-sidecar-protocol.md) — 메시지 계약, `fastPreviewKind` 허용 값
- [`camera-helper-edsdk-profile.md`](./camera-helper-edsdk-profile.md) — capture/download 시퀀스, 제품 고정 결정
- [`viewer-display.md`](./viewer-display.md) — Story 7.2 계약. 이 Story가 깨지 말아야 할 경계
