# Story 7.3: LibRaw embedded JPEG와 RAW+JPEG fast source 비교

Status: done

> **2026-08-16 공식 경로 결정.** EOS 700D 35회 촬영의 105개 비교 행에서 세 fast-source 후보가
> 모두 0건 승인되어 `Technology No-Go`로 닫혔다. Noah Lee가 운영 경로를
> **`raw-original + pinned darktable 5.4.1`**로 공식 승인했다. 후보는 계속 비활성 상태를 유지하며,
> Story 7.4는 이 정확 경로를 사용한다. 결정 기록:
> `_bmad-output/planning-artifacts/sprint-change-proposal-20260816-010151.md`.

Type: Enabler / Experiment

Epic 7 dependency: `7.1 Go` → `7.2 Go` → **`7.3 approved route decision`** → `7.4 Go` → ... → `7.10 final decision`.
HV-13A는 2026-08-11 `Go`, HV-13B는 2026-08-12 재정의된 범위에서 `Go`이므로 착수 조건은 충족됐다.

## Story

booth product team으로서,
승인 카메라에서 **같은 촬영의 가장 빠르고 신뢰할 수 있는 JPEG source**를 비교하길 원한다.
그래야 display proxy(7.4)가 "파일이 두 개니까 빠를 것"이라는 가정이 아니라 **측정된 카메라 증거**에서 출발한다.

## Scope Boundary

### In Scope

- **Route A — LibRaw embedded JPEG**: CR2에서 embedded JPEG를 in-process로 추출하고 존재율·크기·orientation·decode 유효성·손상·추출 비용을 계측
- **Route B — capability-gated RAW+JPEG**: EDSDK `ImageQuality` capability descriptor를 **런타임 truth**로 읽고, 실제 지원되는 조합에서만 RAW+JPEG를 설정. `groupID`로 **모든 transfer object를 하나의 request에 pair**
- **Route C — 현재 incumbent (baseline)**: `WindowsShellThumbnail` 경로. 이미 제품에 있으므로 **비교의 기준선으로 반드시 함께 측정**한다
- 세 route의 randomized AB/BA 비교와 p50/p95/max/성공률/source 도착 순서/품질/실패 모드 보고
- **approved primary route / fallback route / No-Go 결정**과 근거
- source 거부 규칙: wrong-session / wrong-request / wrong-capture / partial / corrupt / stale을 **persisted RAW truth를 무효화하지 않고** 거부
- 계약 문서, 자동 검증, HV-14 실장비 evidence

### Explicitly Out of Scope

- **display proxy 생성과 관람 화면 게시 → Story 7.4.** 이 Story는 source를 **화면에 올리지 않는다**
- resident display renderer spike → Story 7.5
- RAW 정밀본 tier와 deadline scheduler → Story 7.6
- installer / 100-shot / rollout / 최종 판정 → Story 7.7~7.10
- `src-tauri/src/render/mod.rs`의 384px 상수와 darktable 경로 변경
- Story 7.2의 display generation/pointer/swap/telemetry 계약 변경
- `session.json` 스키마 확장

> **이 Story는 Enabler / Experiment다.** 두 후보가 모두 No-Go여도, 비교 구현·evidence·대체 경로 결정이
> 완전하면 조사 목적을 닫을 수 있다 (Epic 7 execution rule). **No-Go 후보를 production 경로로 승격하지 않는다.**

## Acceptance Criteria

1. **Given** 승인 하드웨어의 EOS 700D 촬영에서, **when** LibRaw embedded-JPEG route를 실행하면, **then** warm-up 5회 + 측정 30회 이상이 source 존재 여부, 크기, orientation, decode 유효성, 손상, request correlation, ready 시각, 추출 비용을 각각 기록해야 하고, 모든 측정 촬영과 대표 화질 corpus를 검토하기 전까지 이 route는 `experimental`로 남아야 한다.
2. **Given** 카메라가 자기 image-quality capability descriptor를 보고할 때, **when** 지원되는 RAW+JPEG 조합을 실행하면, **then** warm-up 5회 + 측정 30회 이상이 group/object correlation으로 **모든 transfer object를 하나의 request에 pair**해야 하고, 지원되지 않는 RAW+small 조합을 가정하지 않은 채 모든 object가 안전하게 download되거나 취소되어야 한다.
3. **Given** 두 route를 모두 시험할 수 있을 때, **when** randomized AB/BA 비교를 수행하면, **then** 보고서가 route별로 p50, p95, max, 성공률, source 도착 순서, 품질, 실패 모드를 식별해야 하고, **primary route / fallback route / No-Go 중 하나를 명시적 증거와 함께 선택**해야 한다.
4. **Given** source 후보가 사용 가능해졌을 때, **when** 제품 성공을 평가하면, **then** **보정되지 않은 source는 절대 qualifying preset-applied viewer frame으로 집계되지 않아야 하고**, wrong-session·wrong-request·wrong-capture·partial·corrupt·stale source는 **persisted RAW truth를 무효화하지 않고** 거부되어야 한다.
5. **Given** 구현과 자동 테스트가 완료되었을 때, **when** Story 7.3 closure를 검토하면, **then** HV-14가 완전한 source 비교와 승인된 route 결정을 기록하기 전까지 `review`에 머물러야 한다.

## Tasks / Subtasks

### ✅ T1. Source capability와 measurement 계약을 정의한다 (AC: 1, 2, 3, 4)

- [x] `src/shared-contracts/schemas/capture-source.ts` 신규. zod v4로 고정한다.
  - `sourceRouteSchema`: `z.enum(['libraw-embedded-jpeg', 'camera-paired-jpeg', 'windows-shell-thumbnail'])` — **incumbent를 1급 값으로 등록한다.** 기준선이 계약에 없으면 비교가 성립하지 않는다
  - `sourceCandidateSchema`: `captureId`, `requestId`, `sessionId`, `route`, `assetPath`(nullable), `widthPx`, `heightPx`, `byteSize`, `exifOrientation`(nullable), `decodeValid`, `sourceHash`, `objectIndex`, `groupId`(nullable), `readyAtHostMicros`, `extractionCostMicros`
  - `sourceRejectReasonSchema`: `z.enum(['absent','partial','corrupt','undecodable','orientation-unsupported','wrong-session','wrong-request','wrong-capture','stale','unsupported-combination','extraction-failed','cancelled'])`
  - `imageQualityCapabilitySchema` (helper → host): `descriptorAvailable`, `currentValue`, `supportedValues`(배열), `rawPlusJpegSupported`, `probedAtHostMicros`
  - `sourceComparisonSampleSchema`: 위를 묶은 표본 1행 + `blockOrder`(`'AB' | 'BA'`), `blockIndex`, `isWarmUp`
- [x] `src/shared-contracts/dto/capture-source.ts`에 `z.infer` 별칭 추가. `dto/display.ts` 형식을 그대로 따른다.
- [x] `src/shared-contracts/schemas/index.ts`, `src/shared-contracts/index.ts`에 export 추가.
- [x] `src-tauri/src/contracts/dto.rs`에 대응 DTO를 `#[serde(rename_all = "camelCase")]`로 추가한다. 필드명·nullability·enum 문자열이 TS와 **정확히** 일치해야 한다.
- [x] `docs/contracts/capture-source.md` 신규 작성. `docs/contracts/camera-helper-sidecar-protocol.md`에 상호 참조와 **아래 T2의 `fastPreviewKind` 정정**을 반영한다.

### ✅ T2. 현재 incumbent의 정체를 바로잡고 기준선으로 고정한다 (AC: 3) — **최우선, 저비용**

> 이 작업을 먼저 하지 않으면 비교의 기준선이 거짓 라벨 위에 세워진다.

- [x] **`docs/contracts/camera-helper-sidecar-protocol.md:183`의 `"fastPreviewKind": "embedded-jpeg"` 예시를 정정한다.** 현재 helper가 실제로 보내는 값은 `"windows-shell-thumbnail"`이다 (`CanonSdkCamera.cs:1115`). 계약 문서의 예시가 구현에 없는 값을 보여주고 있다.
  - 실제 사용 값 목록을 문서에 명시한다: `windows-shell-thumbnail`, `raw-fallback-preview`
  - `embedded-jpeg`는 **이 Story가 Route A로 새로 도입하는 값**임을 밝힌다
- [x] incumbent의 실제 특성을 계측 가능하게 만든다. `WindowsShellThumbnail.TrySavePreviewJpeg`는 `1600×1600 ResizeToFit | BiggerSizeOk | ThumbnailOnly`를 요청한다. **실제로 나오는 픽셀 크기를 표본마다 기록한다** — display-fit 판정(7.4)의 입력이므로 "1600"이라는 요청값이 아니라 실측값이 필요하다.
- [x] host의 `HELPER_FAST_PREVIEW_WAIT_MS = 120` 예산은 **바꾸지 않는다.** 2026-08-12 evidence에서 이 예산이 5/5 소진됐다(`helper_fast_preview_wait_budget_exhausted`)는 사실 자체가 incumbent의 측정 결과다. 예산을 늘려 통과시키면 비교가 오염된다.

### ✅ T3. Route A — 내장 JPEG 추출을 구현한다 (AC: 1, 4)

> **의존성 결정 (2026-08-12, 승인됨): LibRaw를 쓰지 않는다.** CR2는 TIFF 컨테이너이므로
> IFD#0의 `StripOffsets`/`StripByteCounts`를 직접 읽어 내장 full-size JPEG을 꺼낸다.
> 이 저장소가 이미 `image_probe.rs`에서 crate 없이 하는 것과 같은 종류의 파싱이다.
> LGPL-2.1/CDDL 게이트와 Story 7.7 offline 인벤토리 영향이 **둘 다 사라졌고**, 새 crate는
> 추가되지 않았다. route 이름은 `embedded-jpeg`다 — 계약이 쓰지도 않는 라이브러리를
> 이름에 담지 않는다.

- [x] `src-tauri/src/capture/embedded_jpeg.rs` 신규. TIFF 헤더(`II`/`MM` 모두) → IFD#0 순회 →
  `StripOffsets`(0x0111) / `StripByteCounts`(0x0117) / `Orientation`(0x0112) 추출.
- [x] 범위 검증. strip이 파일 끝을 넘어가면 `corrupt`. 조용히 잘라 쓰지 않는다.
- [x] 구조 검증은 **Story 7.2의 `probe_jpeg`를 직접 호출한다.** 규칙을 두 벌 만들지 않는다.
- [x] orientation은 JPEG 자체 EXIF 우선, 없으면 TIFF IFD#0 값. CR2는 컨테이너 쪽에 두는
  경우가 많아 이 fallback이 없으면 회전된 촬영이 orientation 1로 잘못 통과한다.
- [x] CR3(ISO BMFF)는 이 파서 대상이 아니며 **조용히 실패하지 않고** `undecodable`로 거부한다.
- [x] **추출은 읽기 전용이다.** RAW 파일을 열어 훑을 뿐 어디에도 쓰지 않으므로 실패가
  RAW truth에 닿을 경로 자체가 없다.
- [x] 12개 경계 조건 단위 테스트 (추출 성공 / orientation 2종 / absent / 범위 초과 / EOI 없음 /
  CR3 / non-TIFF / 헤더 범위 밖 / byte order / 입력 불변).
- [x] **호스트 Rust에 둔다.** 당초 helper(C#) 배치의 근거는 "host에 native RAW 의존성을
  들이지 않는다"였는데 의존성이 사라지며 그 근거가 소멸했다. host에 두면 `image_probe`를
  직접 호출할 수 있고 sidecar 경계를 넓히지 않으며 카메라 없이 완전히 검증된다.

<details>
<summary>원안 (LibRaw 기반) — 승인 결과 채택되지 않음</summary>

### ⛔ T3-원안. Route A — LibRaw embedded JPEG 추출 (미채택)

- [ ] **의존성 승인이 선행 조건이다.** LibRaw는 새 native 의존성이며 **LGPL-2.1 / CDDL dual license**다. Story 7.7의 offline clean-machine 재현 인벤토리와 배포 조건에 직접 영향을 준다.
  - 착수 전에 `docs/contracts/capture-source.md`에 다음을 기록하고 승인을 받는다: 선택한 license(LGPL-2.1 동적 링크 권장), 배포 형태, 버전 pin, clean-VM 인벤토리 항목
  - **승인 없이 crate/native 라이브러리를 추가하지 않는다.** 승인이 나기 전이면 T3를 blocked로 두고 T4를 먼저 진행한다
- [ ] 추출 경계를 **helper 쪽(C#)에 둔다.** 근거: RAW 파일을 이미 helper가 소유하고 있고, host Rust에 native RAW 의존성을 들이면 Story 7.7 인벤토리와 Rust 빌드가 동시에 무거워진다. 아키텍처의 "sidecar는 얇은 Canon adapter" 경계와 충돌하지 않도록 **추출 전용 모듈**로 격리한다.
  - `sidecar/canon-helper/src/CanonHelper/Runtime/EmbeddedJpegExtractor.cs` 신규
  - 실패는 예외가 아니라 `sourceRejectReason`으로 돌려준다. **RAW 저장 경로에 절대 영향을 주지 않는다**
- [ ] 추출 결과를 검증한다. 검증 없이 통과시키면 7.4가 깨진 자산을 화면에 올린다.
  - SOI(`FFD8FF`) + SOF에서 width/height 파싱 + **EOI(`FFD9`) trailer** + `byteSize > 0`
  - EXIF orientation이 존재하면 `1`만 허용. 그 외는 `orientation-unsupported`로 거부
  - **Story 7.2의 `src-tauri/src/display/image_probe.rs`와 같은 판정 규칙을 쓴다.** 두 곳에 다른 기준이 생기면 7.4에서 통과 기준이 갈라진다. 규칙을 계약 문서에 한 번만 적고 양쪽이 그것을 참조한다
- [ ] 추출 비용(`extractionCostMicros`)을 cold/warm 구분해 기록한다. 연구 실측은 cold 약 50ms / warm 약 12ms지만 **표본이 2개뿐**이므로 30회 이상으로 다시 세운다.
- [ ] 추출 산출물은 **session-scoped 경로**에만 쓴다. `<session_root>/renders/sources/<captureId>-embedded.jpg`. NFR-004는 0 tolerance다.

</details>

### ✅ T4. Route B — capability-gated RAW+JPEG를 구현한다 (AC: 2, 4)

> **여기가 이 Story에서 가장 위험한 변경이다.** 아래 첫 항목을 모르고 시작하면 두 번째 파일이
> 조용히 사라지는 것을 "카메라가 지원하지 않는다"로 오진한다.

- [x] **`CanonSdkCamera.cs:741`의 single-object 가드를 먼저 이해하고 바꾼다.**
  ```csharp
  if (captureContext is null || Interlocked.Exchange(ref captureContext.DownloadStarted, 1) == 1)
  {
      if (inRef != IntPtr.Zero) { EDSDK.EdsRelease(inRef); }   // ← 두 번째 object를 버린다
      return EDSDK.EDS_ERR_OK;
  }
  ```
  현재 helper는 **capture당 첫 transfer object 하나만 download하고 나머지를 release한다.** RAW+JPEG를 켜면 카메라가 object event를 두 번 올리는데, 두 번째가 여기서 버려진다. AC 2의 "모든 transfer object를 하나의 request에 pair"는 이 가드를 **object 단위가 아니라 request 단위 완료 판정으로 바꾸는 것**을 의미한다.
  - `DownloadStarted` 플래그를 `expectedObjectCount` / `downloadedObjects` 모델로 바꾼다
  - `EdsGetDirectoryItemInfo`의 `groupID`로 같은 촬영의 object를 묶는다. `groupID`가 0이거나 없으면 **파일명 stem + 도착 시각 창**을 보조 correlation으로 쓰고, 그 사실을 표본에 기록한다
  - **in-flight capture는 계속 1개만 허용한다** (`camera-helper-edsdk-profile.md` 제품 고정 결정). object가 여러 개일 뿐이다
- [x] **object role을 확장자가 아니라 `EdsDirectoryItemInfo`로 판정하고, JPEG object를 RAW로 저장하지 않는다.**
  현재 `DownloadCapture`는 `info.szFileName`의 확장자를 쓰고 **비어 있으면 `.cr3`로 기본값을 넣는다.**
  (700D는 실제로 `.CR2`를 만든다.) object가 둘이 되면 이 로직이 JPEG를 RAW original로 저장할 수 있다.
  - `capturesOriginalsDir`에는 **RAW object만** 저장한다. JPEG object는 source 경로로 분리한다
  - RAW 확장자 기본값 fallback을 이 Story에서 고치되, **기존 `.cr3` 산출물의 하위 호환은 유지한다**
- [x] `ImageQuality` capability descriptor를 **런타임 truth로 읽는다.** 지원 여부를 가정하지 않는다.
  - `EDSDK.PropID_ImageQuality`의 현재값과 `EdsGetPropertyDesc`의 지원 목록을 읽어 `imageQualityCapability` 메시지로 host에 보고한다
  - **EOS 700D가 RAW+small 조합을 지원한다고 가정하지 않는다.** descriptor에 없는 조합은 시도 자체를 하지 않고 `unsupported-combination`으로 기록한다
  - 측정이 끝나면 **원래 값으로 되돌린다.** 카메라 설정을 측정 상태로 남기면 다음 세션의 제품 동작이 바뀐다
- [x] 두 object의 도착 순서를 그대로 기록한다. **JPEG가 RAW보다 먼저 안정적으로 도착할 때만** 우선 source 후보로 승격한다. 순서가 뒤집히거나 흔들리면 그 관측이 곧 결과다.
- [x] 부분 실패 처리: object 하나가 실패해도 **이미 저장된 RAW truth를 무효화하지 않는다.** 실패한 object는 `partial` / `cancelled`로 기록하고 request는 RAW 기준으로 계속 성공 처리한다.
- [x] helper → host 메시지를 확장한다. `docs/contracts/camera-helper-sidecar-protocol.md`의 `메시지 종류 v1`에 추가하고 protocol version을 올린다.
  - `image-quality-capability` (helper → host)
  - `file-arrived`에 `objectIndex`, `groupId`, `objectRole`(`'raw' | 'jpeg'`) 추가. **기존 필드의 의미를 바꾸지 않는다** — host의 기존 정규화가 깨진다

### ✅ T5. 측정 lane과 correlation guard를 만든다 (AC: 1, 2, 3, 4)

- [x] 측정 lane은 **기본 off**다. `BOOTHY_SOURCE_COMPARE_MODE = off | libraw | paired | shell | ab` (기본 `off`). off일 때 제품 경로는 지금과 완전히 동일하게 동작한다.
  - Story 7.2의 `BOOTHY_DISPLAY_SAMPLE_MODE`와 **독립적인 스위치**다. 두 lane을 동시에 켜면 표본이 서로 오염되므로, 동시 활성화 시 경고를 남기고 source lane을 우선한다
- [x] `src-tauri/src/capture/source_probe.rs` 신규 — route별 후보를 받아 **승격 판정과 거부 사유**를 소유하는 순수 로직. `src-tauri/src/display/display_artifact.rs`의 admission 판정과 같은 형태로 만든다.
  - `evaluate_source_candidate(candidate, context) -> Result<(), SourceRejectReason>`
  - session / request / capture correlation, partial, corrupt, decode, orientation, stale을 전부 **고유 사유**로 판정한다. 조용한 무시 금지
- [x] **AC 4의 핵심 방어선:** 승격된 source는 `sourceRoute`와 `isPresetApplied: false`를 반드시 함께 싣는다. 어떤 집계도 `isPresetApplied === false`인 표본을 qualifying preset-applied frame으로 세지 않는다.
  - Story 7.2의 `viewer-present.jsonl` 집계와 **분리된 파일**에 기록한다: `<session_root>/diagnostics/source-comparison.jsonl`
  - 두 파일을 합쳐 집계하는 도구를 만들지 않는다. 합치는 순간 보정되지 않은 source가 KPI에 섞인다
- [x] randomized AB/BA 순서를 **host가 결정하고 기록한다.** 사람이 순서를 고르면 편향이 들어간다. seed와 block 배치를 표본에 남긴다.

### ✅ T6. 자동 검증을 추가한다 (AC: 1~4)

- [x] `src-tauri/src/capture/source_probe.rs`의 단위 테스트 — 거부 매트릭스를 **전부** 명시적으로 단언한다: absent, partial, corrupt, undecodable, orientation != 1, wrong-session, wrong-request, wrong-capture, stale, unsupported-combination.
- [x] `src-tauri/tests/capture_source.rs` 신규 — `src-tauri/tests/viewer_display.rs`의 tempdir 패턴을 재사용한다.
  - 두 object가 하나의 request에 pair된다
  - object 하나가 실패해도 **RAW truth가 유지된다** (AC 4 회귀 방지선)
  - 보정되지 않은 source가 preset-applied 집계에 들어가지 않는다
  - 세션 교체 시 source 후보와 산출물이 정리된다 (NFR-004)
  - `BOOTHY_SOURCE_COMPARE_MODE=off`에서 어떤 source 산출물도 만들어지지 않는다
- [x] `sidecar/canon-helper/tests/CanonHelper.Tests/`에 테스트 추가 — `JsonFileProtocolTests.cs` 형식을 따른다.
  - multi-object download가 `groupID`로 pair된다
  - `groupID`가 없을 때 보조 correlation이 동작하고 그 사실이 표본에 남는다
  - `ImageQuality` descriptor에 없는 조합은 시도하지 않는다
  - embedded JPEG 추출 실패가 RAW 저장에 영향을 주지 않는다
- [x] `src/shared-contracts/capture-source.contracts.test.ts` 신규 — TS↔Rust 라운드트립. route/reject enum 문자열 고정.
- [x] Scope guard: display proxy 생성, viewer 게시, resident renderer, deadline scheduler를 나타내는 심볼이 이번 변경 범위에 없다는 것을 review checklist 항목으로 남긴다.
- [x] 검증 명령과 **있는 그대로의 결과**를 completion note에 기록한다: `pnpm lint`, `pnpm test:run`, `cargo fmt`, `cargo test`, `dotnet test`, `pnpm build`.
  - 기존 사실: `pnpm build`(`tsc -b`)는 Story 7.1 이전부터 ReadinessScreen / capture-runtime / governance / operator diagnostics / active-preset 경로의 기존 오류로 실패한다. `capture_readiness` Rust 테스트는 render 경로 공유 자원 경합으로 flaky다(HEAD baseline 11~13건). **이번 Story가 이 상태를 악화시키지 않았음**을 확인하고 정직하게 적는다

### 🟡 T7. HV-14 하드웨어 증거를 생산한다 (AC: 5)

- [x] `tests/hardware/capture-source/hv-14/README.md`와 `run-<timestamp>/`를 준비한다. `tests/hardware/viewer-present/`와 섞지 않는다.
- [ ] 환경 fingerprint: 승인 PC, EOS 700D 펌웨어, 렌즈, 카드, USB 포트/케이블, EDSDK 버전, helper 버전, LibRaw 버전, 앱 버전·커밋 해시, `BOOTHY_SOURCE_COMPARE_MODE`, 카메라 image quality 설정 전/후.
- [ ] **Route별 raw 표본**: route당 warm-up 5회 + 측정 30회 이상. 표본마다 존재/크기/orientation/decode/손상/correlation/ready 시각/추출 비용.
- [ ] **capability descriptor 증거**: `ImageQuality` 현재값과 지원 목록 원문, 시도한 조합과 거부한 조합.
- [ ] **correlation 증거**: 모든 transfer object가 하나의 request에 pair됐음을 보이는 로그 구간. object가 버려진 케이스가 0건임을 보인다.
- [x] **품질 corpus 판정**: 승인된 fast-source 산출물이 0건이므로 사람 품질 승격은 수행하지 않고 세 후보를 모두 기술 `No-Go`로 판정했다. 품질 승인을 만들어내지 않는다.
- [x] **AB/BA 집계**: 35회 촬영, 105개 route 행을 보존했으며 실패를 제외하지 않았다.
- [x] **route 결정**: 세 fast-source 후보 `Technology No-Go`; 승인 대체 운영 경로는 `raw-original + pinned darktable 5.4.1` (Noah Lee, 2026-08-16).
- [ ] Story 7.2의 계측 완결성 게이트를 이 회차에도 태운다: `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`. 2026-08-12에 수정한 terminal 행 누락이 실장비에서 재발하지 않음을 함께 확인한다.
- [x] `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의 Story 7.3 행을 갱신한다.
- [x] HV-14 후보 기술 `No-Go`와 승인 대체 운영 경로를 기록한 뒤 status를 `done`으로 전환했다. **자동 테스트 통과가 아니라 2026-08-16 제품 결정으로 닫았다.**

### ✅ T8. 영향 문서를 갱신한다 (all AC)

- [x] `docs/contracts/capture-source.md` (신규)
- [x] `docs/contracts/camera-helper-sidecar-protocol.md` (메시지 v2, `fastPreviewKind` 정정)
- [x] `docs/contracts/camera-helper-edsdk-profile.md` (multi-object download와 capability probe)
- [x] `_bmad-output/planning-artifacts/architecture.md` implementation note (route 결정과 근거)
- [x] Story completion note, `sprint-status.yaml`, HV-14 ledger row

### Review Findings

- [x] [Review][Decision] DEVICE_BUSY shutter retry가 compare mode off에서도 항상 동작 — **결정(2026-08-14): 제품 hardening으로 유지** (옵션 1).
- [x] [Review][Decision] `recent_captures` readiness 재조화가 측정 lane 밖에서 항상 활성 — **결정(2026-08-14): 제품 수정으로 유지** (옵션 1).
- [x] [Review][Decision] Paired JPEG 대기 예산 5초 — **결정(2026-08-14): RAW 여유(+15s)에 맞춰 상향** → Patch로 전환.
- [x] [Review][Decision] AB/BA가 3 route 중 앞 두 개만 swap — **결정(2026-08-14): 기준선(shell) 고정으로 의도 인정 + 문서화** → Patch로 전환.

- [x] [Review][Patch] Paired JPEG 대기 예산을 RAW allowance와 맞추어 15초로 상향 — fixed 2026-08-14
- [x] [Review][Patch] BA에서 shell이 항상 마지막임을 계약/HV-14 문서에 명시 — fixed 2026-08-14
- [x] [Review][Patch] Helper `SourceObjectRejected` 사유를 표본에 매핑하지 않고 `absent`로 기록 — fixed 2026-08-14
- [x] [Review][Patch] capability가 RAW+JPEG인데 후보 선택이 실패하면 expectedObjectCount=1로 촬영 — fixed 2026-08-14
- [x] [Review][Patch] RAW 경로에서 JPEG claim 거부 시 correlator rollback 없음 — fixed 2026-08-14
- [x] [Review][Patch] paired JPEG timeout 후 in-flight download를 닫지 않아 이중 텔레메트리 가능 — fixed 2026-08-14
- [x] [Review][Patch] `candidate_from_path`가 읽기 실패를 빈 바이트로 삼킴 — fixed 2026-08-14
- [x] [Review][Patch] `probedAtHostMicros`가 wall-clock epoch인데 host monotonic과 혼용 — docs clarified 2026-08-14
- [x] [Review][Patch] `RestoreHeldImageQuality`가 lock 밖에서 set 전 held 값을 null로 지움 — fixed 2026-08-14

- [x] [Review][Defer] helper reject→sample·timeout 경계의 자동 검증이 약함 — deferred, pre-existing

## Dev Notes

### 코드베이스 현실 — 착수 전 반드시 인지할 것

| 항목 | 현재 상태 |
| --- | --- |
| camera helper | **존재.** C# .NET 8, `sidecar/canon-helper/`, Canon EDSDK. 파일 기반 JSONL 프로토콜 |
| helper ↔ host 경계 | `src-tauri/src/capture/sidecar_client.rs` 하나로 격리됨 |
| transfer object 처리 | **capture당 1개만 download한다.** `CanonSdkCamera.cs:741`의 `Interlocked.Exchange(DownloadStarted, 1)`가 두 번째 object를 `EdsRelease`로 버린다 |
| `SaveTo` | `ConfigureSaveToHost`가 `PropID_SaveTo = Host`로 고정 (`CanonSdkCamera.cs:659`) |
| `ImageQuality` capability | **없음.** 코드 어디에서도 읽지 않는다 |
| 현재 "fast preview" | **Windows Shell 썸네일이다.** `WindowsShellThumbnail.TrySavePreviewJpeg`가 `IShellItemImageFactory.GetImage(1600×1600, ResizeToFit\|BiggerSizeOk\|ThumbnailOnly)` |
| `fastPreviewKind` 실제 값 | `windows-shell-thumbnail`, `raw-fallback-preview` |
| `fastPreviewKind` 문서 예시 | **`embedded-jpeg` — 구현에 없는 값이다.** `camera-helper-sidecar-protocol.md:183`. T2에서 정정한다 |
| host fast preview 대기 | `HELPER_FAST_PREVIEW_WAIT_MS = 120`, poll 40ms (`ingest_pipeline.rs:37`) |
| LibRaw | **없음.** Rust deps는 `serde`, `serde_json`, `log`, `tauri`, `tauri-plugin-log`뿐 |
| RAW 확장자 기본 fallback | `.cr3` (`CanonSdkCamera.cs`). 700D는 실제로 `.CR2`를 만든다 |
| display generation/pointer | **Story 7.2 완성.** `src-tauri/src/display/`. 이 Story는 여기에 게시하지 않는다 |
| `render/mod.rs` 384px 상수 | 그대로 둔다. Story 7.4/7.6 범위 |
| **Canon EDSDK payload** | **저장소에 없다.** `sidecar/canon-helper/vendor/`에는 README만 있고 실제 `canon-edsdk/`는 로컬 준비물이다 (EDSDK 13.19.0, 2025-02-28). **helper를 빌드하려면 이 payload가 먼저 있어야 한다** |
| fast preview의 UI 노출 | **노출된다.** `SessionPreviewImage.tsx:120`이 `previewKind=pending-fast-preview`로 booth 사진 레일에 띄운다 |
| 또 다른 preview 생산자 | `ingest_pipeline.rs:164`의 `start_speculative_preview_render_in_dir`. helper fast preview와 **별개 경로**다. 혼동 금지 |

### 핵심 설계 결정 1 — incumbent를 세 번째 route로 반드시 함께 측정한다

AC는 두 route를 말하지만, 제품에는 **이미 세 번째 source가 살아 있다.** `WindowsShellThumbnail`이다.
이것을 기준선으로 함께 재지 않으면 "새 route가 더 빠르다"는 주장에 비교 대상이 없다.

측정된 현실:

- 연구 실측: RAW 도착 약 1.38~1.49초 **뒤에** Shell fallback이 추가로 0.83~1.69초
- 2026-08-12 HV evidence: capture→RAW 2806~19524ms, capture→XMP preview 6804~24028ms
- 같은 회차에서 host의 120ms fast preview 예산이 **5/5 소진**됐다. 즉 incumbent는 제품 hot path에 합류조차 못 하고 있다

**Story 7.2가 증명한 표시 종단점은 이미 충분히 빠르다.** 남은 4초는 여기, source 획득 구간에 있다.

### 핵심 설계 결정 2 — capability는 가정하지 않고 런타임에서 읽는다

EOS 700D가 RAW+small JPEG를 지원한다는 **보장이 없다.** 연구 문서도 "runtime capability를 truth로
사용하고 RAW+small 지원을 가정하지 않는다"고 명시한다.

- `EdsGetPropertyDesc(PropID_ImageQuality)`의 지원 목록이 유일한 truth다
- descriptor에 없는 조합은 **시도하지 않고** `unsupported-combination`으로 기록한다. 시도해서 실패하는 것과
  지원되지 않아 시도하지 않는 것은 다른 결과다
- 측정 후 원래 값 복원은 선택이 아니라 필수다. 카메라를 측정 상태로 남기면 다음 고객 세션의 제품 동작이 바뀐다

### 핵심 설계 결정 3 — RAW truth는 어떤 실험에도 종속되지 않는다

Story 1.5~1.7이 만든 계약이다. RAW 저장이 곧 촬영 성공이고, fast source는 **best-effort 부가물**이다.

- embedded 추출 실패, JPEG object 실패, 손상, 취소 — 전부 RAW 성공 판정을 바꾸지 않는다
- helper의 기존 주석이 이미 이 계약을 적어 두었다: *"Fast-preview notifications are best-effort.
  The RAW handoff remains the only correctness boundary for capture success."*
- 이 Story가 그 경계를 넓히지 않는다. object가 두 개가 되어도 **RAW object 하나의 성공이 여전히 성공 기준**이다

### 핵심 설계 결정 4 — source lane은 고객·운영자 화면을 바꾸지 않는다

**함정:** 현재 fast preview는 이미 booth 사진 레일에 표시된다. `SessionPreviewImage.tsx:120`이
`previewKind=pending-fast-preview`로 라벨링해 띄운다. 즉 route A/B의 산출물을 기존 fast preview 경로에
그대로 흘려보내면 **측정 lane이 제품 UI를 조용히 바꾼다.**

- 측정 lane의 산출물은 **기존 fast preview 경로에 승격하지 않는다.** `<session_root>/renders/sources/`에만 쓰고,
  `renders/previews/`의 canonical 경로를 건드리지 않는다
- 레일 표시 규칙, `previewKind` 라벨, 대기 문구를 바꾸지 않는다. NFR-001의 copy budget은 이미 차 있다
- **route 결정 후 실제 승격은 Story 7.4가 한다.** 이 Story는 재기만 한다
- 운영자 진단에도 새 표시 항목을 만들지 않는다. 측정값은 JSONL과 로그로만 남긴다

### 핵심 설계 결정 5 — 보정되지 않은 source는 KPI에 섞이지 않는다

AC 4의 핵심이고, 이 Story에서 가장 쉽게 사고가 나는 지점이다.

- source lane의 기록은 `source-comparison.jsonl`에, 표시 종단점 기록은 Story 7.2의 `viewer-present.jsonl`에 둔다
- **두 파일을 합쳐 집계하는 도구를 만들지 않는다.** 합치는 순간 무보정 JPEG의 빠른 도착 시각이
  preset-applied KPI를 실제보다 좋아 보이게 만든다
- 모든 source 표본에 `isPresetApplied: false`를 명시적으로 싣는다. 필드가 없어서 기본값으로 통과하는 경로를 만들지 않는다
- FR-010의 qualifying frame 정의는 "같은 capture의 **preset이 적용된** 이미지"다. 이 Story는 그 정의를 건드리지 않는다

### Story 7.2 계약과의 접점 (깨지 말아야 할 것)

1. **display generation/pointer/swap/telemetry 계약을 바꾸지 않는다.** 이 Story는 source를 화면에 올리지 않는다.
   7.4가 `sample_publisher`만 실제 proxy로 교체하면 되도록 그 seam을 그대로 둔다.
2. **JPEG 구조 검증 규칙을 두 벌 만들지 않는다.** `image_probe.rs`가 SOI/SOF/EOI/orientation 판정을 이미 소유한다.
   route A의 추출 검증이 다른 기준을 쓰면 7.4에서 통과 기준이 갈라진다.
3. **`BOOTHY_DISPLAY_SAMPLE_MODE`와 이 Story의 lane을 동시에 켜지 않는다.** 두 lane이 같은 촬영에 붙으면
   표본이 서로 오염된다.
4. **계측 완결성 규칙을 상속한다.** commit/생성된 산출물 하나당 terminal 행 하나. 행이 없는 산출물이 생기면
   분모가 조용히 줄어 성공률이 실제보다 좋아 보인다. Story 7.2가 이 결함으로 한 회차를 잃었다.

### 절대 하지 말 것 (disaster prevention)

1. **`HELPER_FAST_PREVIEW_WAIT_MS`를 늘려 incumbent를 통과시키지 않는다.** 예산 소진이 곧 측정 결과다.
2. **RAW 저장 경로에 실험 코드를 넣지 않는다.** 추출과 pairing은 RAW 저장이 끝난 뒤의 부가 경로다.
3. **두 번째 transfer object를 조용히 버리지 않는다.** 현재 코드가 그렇게 되어 있으므로, 고치지 않고 측정하면
   "카메라가 RAW+JPEG를 지원하지 않는다"는 **잘못된 결론**이 나온다.
4. **카메라 설정을 측정 상태로 남기지 않는다.** 반드시 복원한다.
5. **LibRaw를 승인 없이 추가하지 않는다.** LGPL-2.1/CDDL이며 Story 7.7의 offline 인벤토리에 직접 영향을 준다.
6. **보정되지 않은 source를 qualifying preset-applied frame으로 집계하지 않는다.**
7. **route를 하나만 재고 비교했다고 하지 않는다.** incumbent 포함 세 route를 같은 조건에서 잰다.
8. **표본을 골라내지 않는다.** 실패와 timeout을 제외하면 성공률이 거짓이 된다.
9. **`session.json` 스키마를 확장하지 않는다.** Story 7.2와 동일하게 별도 versioned 기록을 쓴다.
10. **display proxy를 만들지 않는다.** 7.4 범위이며 끌어오면 evidence가 오염된다.
11. **384px 상수와 darktable 경로를 건드리지 않는다.**
12. **측정 산출물을 기존 fast preview 경로(`renders/previews/`)로 승격하지 않는다.** 그 경로는 이미 booth 레일에 표시된다.
13. **JPEG object를 RAW original로 저장하지 않는다.** 확장자 기본값 `.cr3` fallback이 정확히 이 사고를 일으킬 수 있다.
14. **`speculative_preview`와 helper fast preview를 혼동하지 않는다.** 서로 다른 생산자다.
15. **자동 테스트 통과만으로 `done` 처리하지 않는다.** HV-14 route 결정 없이는 `review` 유지다.

### 재사용할 기존 패턴 (바퀴 재발명 금지)

| 필요한 것 | 참고할 기존 구현 |
| --- | --- |
| JPEG 구조 검증 (SOI/SOF/EOI/orientation) | `src-tauri/src/display/image_probe.rs` |
| 승격/거부 판정을 순수 함수로 분리 | `src-tauri/src/display/display_artifact.rs` `evaluate_admission` |
| 고유 reason code로 거부 기록 | `src-tauri/src/contracts/dto.rs`의 `DISPLAY_REJECT_*` |
| 계측 완결성 (산출물당 terminal 행 하나) | `src-tauri/src/commands/display_commands.rs` `close_out_open_generations` |
| host monotonic micros 계측 | `src-tauri/src/viewer/present_telemetry.rs` |
| 세션 범위 JSONL 기록 | `append_present_sample` |
| helper ↔ host 메시지 정의 | `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs` + `src-tauri/src/capture/sidecar_client.rs` |
| helper 파일 프로토콜 테스트 | `sidecar/canon-helper/tests/CanonHelper.Tests/JsonFileProtocolTests.cs` |
| 세션 경로 계산 | `src-tauri/src/session/session_paths.rs`, helper의 `SessionPaths.cs` |
| Rust 통합 테스트 (tempdir) | `src-tauri/tests/viewer_display.rs` |
| zod 스키마 + DTO 분리 | `src/shared-contracts/schemas/viewer-display.ts` + `dto/display.ts` |
| 하드웨어 evidence 패키지 구조 | `tests/hardware/viewer-present/hv-13b/README.md` |
| 완결성 검사 스크립트 | `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1` |
| 파일 경로 → asset URL | `src/booth-shell/components/preset-preview-src.ts` |
| 비차단 후속 작업 스레드 | `src-tauri/src/commands/capture_commands.rs:168` |

### 최신 기술 정보 (구현 전 확인 완료)

- **LibRaw** — embedded thumbnail/preview API(`unpack_thumb`, `dcraw_thumb_writer`)와 EOS 700D 지원을 공식 문서화한다. **production RAW rendering engine이 아니므로 proxy source 역할로 한정**한다. 라이선스는 **LGPL-2.1 또는 CDDL dual**이다. ([LibRaw API](https://www.libraw.org/docs/API-CXX.html), [supported cameras](https://www.libraw.org/supported-cameras), [repository/license](https://github.com/LibRaw/LibRaw))
- **EOS 700D CR2 실측(표본 2개)** — 5184×3456 embedded JPEG가 존재했고 외부 프로세스 비용을 포함해 cold 약 50ms, warm 약 12ms에 추출됐다. **표본이 2개뿐이므로 기본 경로 확정 근거가 아니다.** 30회 이상에서 존재율·orientation·손상·decode·화질을 승인해야 한다.
- **Canon EDSDK `ImageQuality` + `groupID`** — RAW+JPEG 조합에서 각 transfer object가 `EdsDirectoryItemInfo.groupID`를 공유한다. 지원 조합은 `EdsGetPropertyDesc`로만 확인한다. ([Canon CAP](https://asia.canon/en/campaign/developerresources/camera/cap/cap), [EOS 700D specification](https://asia.canon/en/support/6200160400))
- **Canon SDK 배포 조건** — 지역·object-code 배포 조건과 연간 갱신 owner가 release gate다. ([Canon SDK terms](https://asia.canon/en/campaign/developerresources/terms-conditions-for-digital-camera-software-development-kit-sdk))
- **Windows Shell 썸네일** — `IShellItemImageFactory.GetImage`의 `ThumbnailOnly`는 파일에 내장된 썸네일만 쓰고 렌더링을 하지 않는다. `BiggerSizeOk`는 요청보다 큰 캐시본을 허용하므로 **실제 산출 크기가 요청값과 다를 수 있다.** 표본마다 실측 크기를 기록해야 하는 이유다.

### 기술 스택 고정값

- Tauri `2.10.3` (Rust), `@tauri-apps/cli`/`api` `^2.10.1`, Rust edition 2021, rust-version `1.77.2`
- Rust deps: `serde`, `serde_json`, `log`, `tauri`(`protocol-asset`), `tauri-plugin-log`. **새 crate 추가 전 승인 필요**
- helper: **.NET 8**, C#, `sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj`. 테스트는 `dotnet test`
- React `^19.2.4`, zod `^4.3.6` (v4 API), TypeScript `~5.9.3`, Vite `^8.0.1`
- Vitest `^4.1.0`, jsdom `^29`, Testing Library React `^16.3.2`, setup: `src/test/setup.ts`
- 패키지 매니저: **pnpm 10.31.0**. npm/yarn 금지

### Verification Guardrails

- 자동 테스트는 필요하지만 HV-14를 대신하지 않는다.
- Story 7.3 evidence는 **source 획득 비교와 route 결정**에 한정한다. 화면 표시 판정은 7.4가 소유한다.
- 이 Story의 수치는 source 구간의 기준선이지 제품 SLA 통과 근거가 아니다.
- Epic 7 dependency상 HV-14 route 결정이 막히면 7.4 이후 전체가 막힌다.
- **두 후보가 모두 No-Go여도 조사 목적은 닫을 수 있다.** 그 경우 대체 경로 결정(incumbent 유지 또는
  7.5 resident renderer 우선)을 명시적 증거와 함께 기록한다.

### Previous story intelligence — Story 7.2 (`done`, HV-13B 재정의 `Go`)

이 Story가 반드시 상속해야 할 학습:

- **계측에 행이 없는 산출물을 만들지 마라.** 7.2는 commit된 generation 10개 중 1개에 terminal 행이 없어
  한 회차를 잃었다. "표시되지 않음"과 "보고 유실"을 구분할 수 없었고 분모가 조용히 줄었다.
  이 Story도 route별 시도마다 terminal 행 하나를 보장한다.
- **자동 검증 전부 통과 + 실장비 실패가 세 번 일어났다.** Story 1.9, 7.1 HV-13A, 7.2 HV-13B가 모두 그랬다.
  **HV-14 evidence 절차를 구현과 동시에 준비한다.**
- **취소/정리 경로가 정당한 데이터를 지운다.** 7.2에서 effect cleanup이 커밋된 표본의 보고를 취소했다.
  이 Story의 object 취소·release 경로도 같은 눈으로 본다. `EdsRelease`가 아직 쓰이지 않은 object를
  지우는 순간 그 표본은 영원히 사라진다.
- **문서와 구현이 갈라져 있었다.** `fastPreviewKind: "embedded-jpeg"` 예시가 구현에 없는 값이었다.
  계약을 확장하기 전에 **현재 계약이 사실인지 먼저 확인한다.**
- **환경 변수 lane은 기본 off, 제품 경로 무영향이 원칙이다.** 7.2의 `BOOTHY_DISPLAY_SAMPLE_MODE` 패턴을 따른다.
- **물리 프레임 게이트는 7.8/HV-18B로 이관됐다.** 이 Story는 물리 촬영 rig를 필요로 하지 않는다.

### Git intelligence

최근 커밋: `c390f53`(Story 7.1), `89ae52c`, `b24cfc4`, `12309fa`, `81b1271`.
Story 7.2 작업은 아직 커밋되지 않은 작업 트리에 있다 (branch `story/7-1-viewer-window-readiness`).

앞의 하나를 제외한 최근 커밋이 전부 **capture timing / preview latency / thumbnail fallback** 영역이다.
즉 이 Story가 건드릴 코드는 최근 가장 활발히 수정된 영역과 정면으로 겹친다.

- `src-tauri/src/capture/ingest_pipeline.rs` (1345줄) — 편집 전 현재 내용을 다시 읽는다
- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs` (1593줄) — 편집 전 현재 내용을 다시 읽는다
- `src-tauri/src/contracts/dto.rs`, `src-tauri/src/render/mod.rs`도 큰 파일이다

`12309fa`("Record thumbnail validation and ship fast preview fallback")가 현재 incumbent를 만든 커밋이다.
그 커밋의 결정과 한계를 먼저 읽고 시작한다.

### Project Structure Notes

- 신규 Rust: `src-tauri/src/capture/source_probe.rs` — 아키텍처의 `src-tauri/src/capture/` 경계 안이다
- 신규 C#: `sidecar/canon-helper/src/CanonHelper/Runtime/EmbeddedJpegExtractor.cs` — sidecar 경계 안이다
- 신규 계약: `src/shared-contracts/schemas/capture-source.ts` + `dto/capture-source.ts`
- 신규 문서: `docs/contracts/capture-source.md`
- 신규 evidence: `tests/hardware/capture-source/hv-14/`
- **편차와 근거:** 아키텍처는 sidecar를 "얇은 Canon EDSDK adapter"로 규정한다. embedded JPEG 추출을
  helper에 두는 것은 그 경계를 약간 넓히는 결정이다. 근거는 (a) RAW 파일을 helper가 이미 소유하고,
  (b) host Rust에 native RAW 의존성을 들이면 Story 7.7의 offline 인벤토리와 Rust 빌드가 동시에 무거워지며,
  (c) 추출 전용 모듈로 격리하면 session/preset/timing/UI truth는 여전히 helper 밖에 남는다는 것이다.
  **이 편차를 `docs/contracts/capture-source.md`에 기록한다.**

## Dependencies

- Story 7.1 / HV-13A `Go`, Story 7.2 / HV-13B 재정의 `Go`
- 기존 helper capture 경로 (`request-capture`, `file-arrived`, `SessionPaths`)
- 승인 EOS 700D와 승인 부스 PC (HV-14의 필수 조건)
- **Canon EDSDK payload (`sidecar/canon-helper/vendor/canon-edsdk/`)** — 저장소에 없다. helper 빌드와 `dotnet test`의 선행 조건이며, 없으면 T2/T4를 시작조차 할 수 없다. 기준 릴리스는 EDSDK 13.19.0 (2025-02-28)
- **셔터 작동 예산** — route 3개 × (warm-up 5 + 측정 30) + 재시도로 **실촬영 105회 이상**이다. 회차 전에 카드 용량, 배터리/전원, 발열, 셔터 수명을 확인한다
- **LibRaw 의존성 승인** (license, 배포 형태, 버전 pin) — T3의 선행 조건
- 물리 모니터 고속 촬영 장비에는 **의존하지 않는다** (7.8/HV-18B 소유)

## Handoff

- Primary: Windows/.NET + Rust host 소유 개발팀
- Architecture review: sidecar 경계 편차, multi-object correlation 모델, 의존성 승인
- QA: HV-14 route별 raw 표본, capability descriptor, correlation, 품질 corpus, AB/BA 집계
- Product: route 결정과 그것이 7.4 display proxy에 주는 제약
- Legal/Release: LibRaw LGPL-2.1/CDDL 및 Canon SDK 배포 조건 (Story 7.7 인벤토리 연결)

## References

- `_bmad-output/planning-artifacts/epics.md#Story-73-LibRaw-embedded-JPEG와-RAW+JPEG-fast-source-비교` (AC 원문)
- `_bmad-output/planning-artifacts/epics.md#Epic-7` (execution rule, evidence dependency)
- `_bmad-output/planning-artifacts/prd.md#FR-010-Pre-Opened-Full-View-Progressive-Preset-Display`
- `_bmad-output/planning-artifacts/prd.md#NFR-004-Session-Isolation-and-Privacy`
- `_bmad-output/planning-artifacts/architecture.md#Approved-2026-08-11-Correct-Course-Baseline`
- `_bmad-output/planning-artifacts/research/technical-preset-image-fast-display-research-2026-08-10.md` (LibRaw 실측, RAW+JPEG capability, license gate, AB/BA 설계)
- `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md` (물리 프레임 게이트 이관)
- `_bmad-output/implementation-artifacts/7-2-immutable-sample과-actual-present-계측.md` (계측 완결성, lane 패턴, HV 학습)
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md#Story-73` (HV-14 required evidence)
- `docs/contracts/camera-helper-sidecar-protocol.md` (메시지 v1 — 이 Story가 v2로 확장)
- `docs/contracts/camera-helper-edsdk-profile.md` (제품 고정 결정, capture/download 시퀀스)
- `docs/contracts/viewer-display.md` (Story 7.2 계약 — 이 Story가 깨지 말아야 할 경계)

### Review Findings

- [x] [Review][Patch] 후보가 생성되지 않은 시도의 크기·바이트 수·object index를 nullable로 표현하고 교차 필드 일관성을 검증한다 [src/shared-contracts/schemas/capture-source.ts:56]
- [x] [Review][Patch] 신규 `image-quality-capability` 이벤트를 host 이벤트 파서에 연결한다 [src-tauri/src/capture/sidecar_client.rs:269]
- [x] [Review][Patch] `accepted`와 `rejectReason`이 서로 모순되는 표본을 스키마에서 거부한다 [src/shared-contracts/schemas/capture-source.ts:101]
- [x] [Review][Patch] descriptor가 없는데 RAW+JPEG 지원으로 기록되는 capability를 스키마에서 거부한다 [src/shared-contracts/schemas/capture-source.ts:84]
- [x] [Review][Patch] `sourceHash`를 문서화된 `fnv1a64:<hex>` 형식으로 검증한다 [src/shared-contracts/schemas/capture-source.ts:68]
- [x] [Review][Patch] EXIF orientation을 유효 범위 1~8로 제한한다 [src/shared-contracts/schemas/capture-source.ts:65]
- [x] [Review][Patch] Route A의 폐기된 `libraw-embedded-jpeg` 명칭을 프로토콜 문서에서 제거한다 [docs/contracts/camera-helper-sidecar-protocol.md:242]
- [x] [Review][Patch] source 비교 lane을 실제 촬영 경로에 연결해 추출·판정·표본 기록이 실행되게 하고, lane off·RAW truth·세션 정리 테스트가 그 제품 경로를 통과하게 한다 [src-tauri/src/capture/source_telemetry.rs:84]
- [x] [Review][Patch] Route A 파서가 TIFF 일반 파일을 CR2로 오인하지 않도록 CR2 signature를 검증한다 [src-tauri/src/capture/embedded_jpeg.rs:115]
- [x] [Review][Patch] strip 태그의 잘못된 type/count와 multi-strip 입력을 `absent`가 아닌 `corrupt`로 거부한다 [src-tauri/src/capture/embedded_jpeg.rs:169]
- [x] [Review][Patch] 실제 big-endian CR2 fixture로 byte-order 지원을 검증한다 [src-tauri/src/capture/embedded_jpeg.rs:497]
- [x] [Review][Patch] expected capture가 있는데 후보 captureId가 없으면 wrong-capture로 거부한다 [src-tauri/src/capture/source_probe.rs:74]
- [x] [Review][Patch] ready 시각이 없는 후보가 freshness 검사를 우회하지 못하게 거부한다 [src-tauri/src/capture/source_probe.rs:104]
- [x] [Review][Patch] paired JPEG 승격 시 groupId 또는 명시적 fallback correlation과 objectIndex를 강제한다 [src-tauri/src/capture/source_probe.rs:82]
- [x] [Review][Patch] Rust 표본 생성·저장에서도 accepted/rejectReason 및 isPresetApplied 불변식을 release 빌드에 강제한다 [src-tauri/src/capture/source_telemetry.rs:137]
- [x] [Review][Patch] 손상되거나 잘린 JSONL 행과 읽기 실패를 빈 표본으로 숨기지 않고 명시적 오류로 보고한다 [src-tauri/src/capture/source_telemetry.rs:173]
- [x] [Review][Patch] 기본 off 테스트가 외부 환경변수에 따라 건너뛰지 않도록 결정적으로 만든다 [src-tauri/tests/capture_source.rs:132]
- [x] [Review][Patch] Route B를 실제 helper 호출에 연결하고 RAW가 저장된 뒤에도 두 transfer object의 request 단위 완료·거부 기록이 끝날 때까지 context를 유지한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs:112]
- [x] [Review][Patch] SDK format과 확장자로 역할을 판정할 수 없는 object가 RAW original 경로로 내려가지 않도록 명시적으로 거부한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:802]
- [x] [Review][Patch] ImageQuality descriptor probe를 실제 호출하고 지원 조합만 설정한 뒤 원래 값으로 복원하며 `image-quality-capability` 결과를 방출한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:1208]
- [x] [Review][Patch] `file-arrived`와 paired/rejected event에 `objectIndex`, `groupId`, `objectRole`, fallback correlation 정보를 실제로 기록한다 [sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs:92]
- [x] [Review][Patch] JPEG가 RAW보다 먼저 도착해도 두 산출물이 처음부터 하나의 captureId를 공유하도록 capture id 할당 시점을 앞당긴다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:1059]
- [x] [Review][Patch] 첫 object의 directory info 조회가 일시 실패한 뒤 재조회에 성공해도 correlator 상태가 비어 두 번째 object를 잘못 수락하지 않게 한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:755]
- [x] [Review][Patch] AB 모드는 촬영 1회마다 세 route를 모두 기록하므로 셔터 예산을 105회가 아닌 warm-up 5회 + 측정 30회로 정정한다 [tests/hardware/capture-source/hv-14/README.md:44]
- [x] [Review][Patch] 완결성 게이트가 AB 회차의 세 canonical route를 명시적으로 요구하고 누락·오탈자 route를 실패 처리하게 한다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:82]
- [x] [Review][Patch] 각 requestId가 기대 route마다 정확히 한 행을 갖는지 검사해 누락 행이 다른 request의 추가 행으로 상쇄되지 않게 한다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:98]
- [x] [Review][Patch] route별 warm-up 5회와 measured 30회 분리를 표시만 하지 말고 실패 조건으로 강제한다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:84]
- [x] [Review][Patch] malformed JSONL·필수 필드 누락·여러 session 혼합을 사람이 읽을 수 있는 명시적 FAIL 보고서로 남긴다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:48]
- [x] [Review][Patch] 비어 있지 않은 단일 seed, AB와 BA 양쪽 존재, blockIndex와 route 간 order 정합성을 검증한다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:156]
- [x] [Review][Patch] 스크립트 PASS가 telemetry completeness에만 한정됨을 명시하고 capability·correlation·quality·aggregate·decision 증거 없이는 HV-14 Go가 되지 않게 한다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:172]
- [x] [Review][Patch] ledger의 갱신 시각·T4 구현 상태·자동 테스트 수치를 현재 결과와 맞춰 증거 provenance를 정정한다 [_bmad-output/implementation-artifacts/hardware-validation-ledger.md:3]
- [x] [Review][Patch] EOS 700D `file-arrived` 예시의 RAW 확장자를 실제 승인 경로인 `.cr2`로 정정한다 [docs/contracts/camera-helper-sidecar-protocol.md:226]
- [x] [Review][Patch] HV-14 runbook에 completeness script의 합성 PASS/FAIL fixture 재현 명령을 포함해 ledger의 smoke-test 주장을 반복 검증 가능하게 한다 [tests/hardware/capture-source/hv-14/README.md:144]
- [x] [Review][Patch] accepted source 표본은 byte-level provenance를 잃지 않도록 `sourceHash`를 필수로 검증한다 [src-tauri/src/capture/source_probe.rs:136]
- [x] [Review][Patch] recorded EXIF orientation이 실제 JPEG probe 결과와 다르면 corrupt로 거부한다 [src-tauri/src/capture/source_probe.rs:127]
- [x] [Review][Patch] 후보 파일이 검증 직전에 사라지거나 읽히지 않은 경우를 source 부재가 아닌 extraction failure로 기록한다 [src-tauri/src/capture/source_telemetry.rs:382]
- [x] [Review][Patch] accepted 비교 표본은 source object 역할을 반드시 기록하고 현재 세 route에서는 JPEG 역할만 허용한다 [src/shared-contracts/schemas/capture-source.ts:142]
- [x] [Review][Patch] fallback correlation 표시는 camera-paired-jpeg route에서만 허용해 correlation 통계를 오염시키지 않게 한다 [src/shared-contracts/schemas/capture-source.ts:142]
- [x] [Review][Patch] CR2 IFD의 중복 strip offset/count 태그를 ambiguous metadata로 거부한다 [src-tauri/src/capture/embedded_jpeg.rs:175]
- [x] [Review][Patch] CR2 orientation 태그가 있는데 type/count/value가 잘못된 경우 orientation 없음으로 통과시키지 않고 corrupt로 거부한다 [src-tauri/src/capture/embedded_jpeg.rs:194]
- [x] [Review][Patch] strip byte count가 0인 CR2를 embedded source 부재가 아닌 corrupt metadata로 분류한다 [src-tauri/src/capture/embedded_jpeg.rs:211]
- [x] [Review][Patch] capability wire event에 계약상 마이크로초 probe 시각인 `probedAtHostMicros`를 그대로 포함하고 host 파서와 대조한다 [sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs:111]
- [x] [Review][Patch] image-quality 복원 실패를 transfer object 거부 이벤트와 분리해 카메라 설정 실패로 기록한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:1434]
- [x] [Review][Patch] 상관관계 거부·정보 조회 실패·paired timeout 뒤 늦게 도착한 transfer object를 `EdsRelease`만 하지 말고 명시적으로 취소한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:849]
- [x] [Review][Patch] paired JPEG의 `EdsDownloadComplete` 실패 경로에서도 SDK download를 취소해 다음 촬영 상태를 보호한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:1261]
- [x] [Review][Patch] groupID가 없는 fallback correlation에서 빈 파일명 stem끼리 일치하는 것으로 간주하지 않게 거부한다 [sidecar/canon-helper/src/CanonHelper/Runtime/CaptureObjectCorrelator.cs:198]
- [x] [Review][Patch] protocol v2의 확장된 `file-arrived`, `source-object-rejected`, capability probe 시각 wire 필드를 직렬화 테스트로 고정한다 [sidecar/canon-helper/tests/CanonHelper.Tests/JsonFileProtocolTests.cs:102]
- [x] [Review][Patch] source 비교의 request 시작·candidate 준비 시각을 동일한 host monotonic clock으로 기록해 wall-clock 보정에도 freshness 판정이 흔들리지 않게 한다 [src-tauri/src/capture/source_telemetry.rs:371] — decision: true host monotonic clock
- [x] [Review][Patch] 중간 실패로 불완전한 AB block이 남으면 다음 실행이 같은 blockIndex를 재사용하지 않도록 명시적으로 실패 처리한다 [src-tauri/src/capture/source_telemetry.rs:360]
- [x] [Review][Patch] paired JPEG 파일만 있고 해당 request/capture의 실제 correlation record가 없거나 읽히지 않으면 fabricated fallback 값으로 승인하지 않고 wrong-capture로 거부한다 [src-tauri/src/capture/source_telemetry.rs:469]
- [x] [Review][Patch] evidence 복사와 completeness gate를 staging에서 수행하고 성공 시에만 확정해, gate 실패 후 동일 세션 수집을 안전하게 재시도할 수 있게 한다 [tests/hardware/capture-source/run-20260813-115426-hv14/collect-evidence.ps1:32]
- [x] [Review][Patch] measurement launcher가 runbook에 명시된 vendored Canon SDK fallback을 지원해 환경변수가 없는 표준 개발 환경에서도 preflight가 동작하게 한다 [tests/hardware/capture-source/run-20260813-115426-hv14/start-measurement.ps1:11]
- [x] [Review][Patch] 최종 HV-14 gate가 단순한 비어 있지 않은 파일이 아니라 환경 확정값, capability/correlation 구조, 품질 이미지 corpus, 집계 지표, 명시적 판정을 검증하도록 강화하고 합성 실패 테스트로 고정한다 [tests/hardware/capture-source/hv-14/check-source-completeness.ps1:172]
- [ ] [Hardware][Patch] EOS 700D 실측에서 Route A/B 17건이 모두 `orientation-unsupported`이므로 실제 orientation을 지원하거나 정규화한 뒤 재측정한다 [tests/hardware/capture-source/run-20260813-115426-hv14/attempts-20260814.md]
- [ ] [Hardware][Patch] Route C가 저장 capture 17건 모두 `absent`이므로 Windows Shell source 준비 이후에 비교하도록 측정 시점을 보정한다 [tests/hardware/capture-source/run-20260813-115426-hv14/attempts-20260814.md]
- [ ] [Hardware][Patch] helper가 `camera-ready`로 복귀한 뒤에도 반복된 `camera-busy`와 RAW handoff timeout을 해결해 35회 회차를 완결한다 [tests/hardware/capture-source/run-20260813-115426-hv14/attempts-20260814.md]
- [ ] [Hardware][Patch] helper 단계에서 실패한 촬영 요청도 source 비교 실패 행으로 남겨 성공률 분모에서 사라지지 않게 한다 [tests/hardware/capture-source/run-20260813-115426-hv14/attempts-20260814.md]
- [x] [Review][Defer] Story 7.2의 re-scoped Go와 요약 gateboard No-Go 표시가 ledger 안에서 상충한다 [_bmad-output/implementation-artifacts/hardware-validation-ledger.md:30] — deferred, pre-existing

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (1M context) — `claude-opus-5[1m]`

### Debug Log References

착수 전 스토리의 사실 주장을 코드로 대조했고, **세 가지가 틀렸다.**

| 스토리 서술 | 실제 | 조치 |
| --- | --- | --- |
| `fastPreviewKind` 실제 값은 `windows-shell-thumbnail`, `raw-fallback-preview` 2개 | **4개다.** `camera-thumbnail`, `windows-shell-thumbnail`, `raw-sdk-preview`, `raw-fallback-preview` | 계약 문서에 4개 전부와 각 생산 경로를 표로 기록. helper 소스를 직접 읽는 계약 테스트로 고정 |
| Canon EDSDK payload가 저장소에 없어 **helper 빌드와 `dotnet test`를 시작조차 할 수 없다** | 이 머신에는 `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\canon-edsdk`가 설정돼 있어 빌드·테스트 모두 정상 동작한다 | T4를 blocked로 두지 않고 진행. ledger의 rerun prerequisite에 환경 변수 경로를 추가 |
| `capture_readiness` Rust 테스트가 flaky (HEAD baseline 11~13건) | flaky가 맞지만 **원인이 병렬 실행 경합임을 확증**했다. `--test-threads=1`에서 **60/60 전부 통과** | 회귀 판정을 이 방법으로 수행. 아래 검증 결과 참조 |

`CanonSdkCamera.cs:741`의 single-object 가드와 `camera-helper-sidecar-protocol.md:183`의
`embedded-jpeg` 예시는 **스토리 서술대로 정확히 존재했다.**

### Completion Notes

#### 무엇이 완료되었는가

- **T1 계약** — `capture-source/v1`, `source-comparison/v1`. TS zod ↔ Rust DTO ↔ helper 구현을 계약 테스트로 고정.
- **T2 incumbent 기준선** — 문서의 거짓 라벨 정정, `WindowsShellThumbnail.Measure`가 **실측 픽셀 크기와 추출 비용**을 돌려주도록 계측 가능화. `HELPER_FAST_PREVIEW_WAIT_MS = 120`은 **변경하지 않았다**(예산 소진이 곧 측정 결과다).
- **T3 Route A** — `capture::embedded_jpeg`가 CR2의 TIFF IFD#0에서 내장 full-size JPEG을 꺼낸다. **새 crate 없음** → Story 7.7 offline 인벤토리 무변화. 구조 검증은 Story 7.2의 `probe_jpeg`를 직접 호출하고, orientation은 JPEG EXIF → TIFF IFD#0 순으로 본다. CR3는 `undecodable`로 명시 거부. **읽기 전용**이라 실패가 RAW truth에 닿을 경로가 없다.
- **T4 완료** — 비교 lane에서만 descriptor가 허용한 RAW+JPEG 조합을 설정하고 모든 종료 경로에서 원래 값으로 복원한다. request 단위로 두 object를 수집하되 JPEG 누락·실패는 RAW 성공을 무효화하지 않는다. captureId는 shutter 전에 고정하며, helper protocol v2가 capability·도착·거부 correlation 증거를 host에 전달한다.
- **T5 측정 lane** — `BOOTHY_SOURCE_COMPARE_MODE` 기본 `off`, 알 수 없는 값은 전부 `off`. `source_probe.rs`가 12개 거부 사유를 전부 고유하게 판정. JPEG 구조 판정은 Story 7.2의 `image_probe`를 **직접 호출**해 규칙이 두 벌 생기지 않게 코드 구조로 보장. AB/BA 배치는 host가 seed에서 결정하고 표본에 남긴다.
- **T6 자동 검증** — 아래 수치 참조. scope guard는 체크박스가 아니라 **실제 grep으로 검증**했다.
- **T7 부분** — HV-14 증거 패키지 README와 기계식 게이트. 합성 fixture로 telemetry·최종 패키지 PASS/FAIL 경계와 결함 6종을 **실제 실행 검증**했다(FAIL 시 원인을 지목하고 exit 1).
- **T8 문서** — 계약 4종 + architecture implementation note + ledger row.

#### 검증 결과 (있는 그대로)

| 명령 | 기준선 (HEAD 작업트리) | 이번 변경 후 | 판정 |
| --- | --- | --- | --- |
| `cargo test --no-fail-fast` | 260 passed / 13 failed | **313 passed / 13 failed** | 신규 53건(단위 41 + 통합 12) 전부 통과. **실패 수가 기준선과 같다** |
| `cargo test --test capture_readiness -- --test-threads=1` | (미측정) | **60 passed / 0 failed** | **경합이 원인임을 확증.** 회귀 아님 |
| `cargo fmt --check` | — | diff 없음 | 통과 |
| `pnpm test:run` | 418 passed / 3 failed (2 files) | **434 passed / 1 failed (1 file)** | 신규 14건 통과. 남은 1건은 아래 참조 |
| `pnpm lint` | 통과 | **통과** | 신규 경고 없음 |
| `dotnet test` | 3 passed / 0 failed | **29 passed / 0 failed** | 신규 26건 통과 |
| 신규 테스트 합계 | — | **93건** | Rust 단위 41 + Rust 통합 12 + C# 26 + TS 14 |
| `pnpm build` (`tsc -b`) | 실패 (기존) | **실패 (18건, 8개 파일)** | **capture-source 관련 오류 0건.** 악화 없음 |

**`capture_readiness` 실패에 대한 정직한 설명.** 병렬 실행 시 13~15건이 실패하는데, 실패 *집합*이
회차마다 양방향으로 달라진다(이번 회차: 신규 6건, 해소 5건). 단일 스레드에서 60/60이 통과하므로
render 경로 공유 자원 경합이며 이번 변경과 무관하다. Rust 제품 코드 변경은
`sidecar_client.rs`에 **optional 필드 3개 추가**와 신규 모듈 2개뿐이고, 실패 테스트는 전부
render/preview 경로다.

**`pnpm test:run`의 남은 1건은 이번 변경과 무관하다.**
`src/governance/hardware-validation-governance.test.ts`가 Story 1.4 문서에서
`Current hardware gate: \`Go\``를 찾지 못한다. 그 문서는 **HEAD에서는 해당 문구를 가지고 있었고**
세션 시작 시점에 이미 작업 트리에서 `Current story status: \`done\``으로 바뀌어 있었다
(초기 `git status`에 Modified로 표시됨). 이 Story는 그 파일을 건드리지 않았다.
기준선의 `PresetLibraryScreen` 실패 2건은 이번 회차에 통과했으므로 그쪽도 flaky다.

#### Scope guard 검증 (grep으로 실제 확인)

- 신규/변경 파일에 `display_fit`, `displayFitPresetProxy`, `rawRefinedDisplay`,
  `resident_renderer`, `deadline_scheduler`, `commit_generation`, `publish_generation`,
  `sample_publisher` **전부 없음**
- `src-tauri/src/render/mod.rs` (384px 상수, darktable 경로) — **diff 없음**
- `src/shared-contracts/schemas/session-manifest.ts` — **diff 없음**
- Story 7.2 display 계약 5개 파일 (`viewer-display.ts`, `display_artifact.rs`, `image_probe.rs`,
  `sample_publisher.rs`, `generation_repository.rs`) — **전부 diff 없음**

#### 제품 경로 무영향의 근거

- `BOOTHY_SOURCE_COMPARE_MODE` 기본 `off`, 알 수 없는 값도 `off`. off일 때 측정 대상 route가 0개다
- `BOOTHY_SOURCE_COMPARE_MODE`가 `paired`/`ab`일 때만 카메라 설정을 바꾼다. 기본 경로는 RAW 한 개만
  수락하며 JPEG/unknown object를 RAW로 오인하지 않는다
- 측정 산출물은 `renders/sources/`에만 쓰이며 booth 레일이 읽는 `renders/previews/`를 건드리지 않는다
- 새 Rust crate를 추가하지 않았다 → Story 7.7 offline 인벤토리 변화 없음

#### 해결된 결정 — Route A는 LibRaw를 쓰지 않는다 (2026-08-12 승인)

세 선택지를 놓고 **의존성 없는 CR2 IFD 파싱**이 선택됐다.

| 선택지 | 라이선스 게이트 | Story 7.7 인벤토리 | 결과 |
| --- | --- | --- | --- |
| LibRaw 도입 | LGPL-2.1/CDDL 검토 필요 | 항목 추가 | 미채택 |
| **의존성 없는 CR2 IFD 파싱** | **없음** | **변화 없음** | **채택** |
| Route A 포기 | 없음 | 없음 | 미채택 |

그 결과 route 이름을 `libraw-embedded-jpeg` → `embedded-jpeg`로, lane 모드를
`libraw` → `embedded`로 개명했다. 계약이 쓰지도 않는 라이브러리를 이름에 담으면
`fastPreviewKind: "embedded-jpeg"` 사건을 되풀이하게 된다. 옛 값 `libraw`는 이제 유효하지
않으며, 남아 있는 설정이 lane을 조용히 켜지 않는다는 것을 테스트로 고정했다.

**HV-14가 답해야 할 것이 하나 늘었다:** 700D의 CR2가 실제로 full-size 내장 JPEG을 담고 있는지,
그리고 orientation이 항상 1인지. 합성 fixture로는 확인할 수 없다.

#### 남은 작업

- **T7 실행분** — HV-14 실촬영 AB 회차 (qualifying 세션당 셔터 정확히 35회; 중단 시 별도 세션에서 전체 재실행)

### File List

**신규**

- `src/shared-contracts/schemas/capture-source.ts`
- `src/shared-contracts/dto/capture-source.ts`
- `src/shared-contracts/capture-source.contracts.test.ts`
- `src-tauri/src/capture/embedded_jpeg.rs`
- `src-tauri/src/capture/source_probe.rs`
- `src-tauri/src/capture/source_telemetry.rs`
- `src-tauri/tests/capture_source.rs`
- `sidecar/canon-helper/src/CanonHelper/Runtime/CaptureObjectCorrelator.cs`
- `sidecar/canon-helper/src/CanonHelper/Runtime/ImageQualityCapability.cs`
- `sidecar/canon-helper/tests/CanonHelper.Tests/CaptureObjectCorrelatorTests.cs`
- `sidecar/canon-helper/tests/CanonHelper.Tests/ImageQualityCapabilityTests.cs`
- `docs/contracts/capture-source.md`
- `tests/hardware/capture-source/hv-14/README.md`
- `tests/hardware/capture-source/hv-14/check-source-completeness.ps1`
- `tests/hardware/capture-source/hv-14/test-check-source-completeness.ps1`
- `tests/hardware/capture-source/run-20260813-115426-hv14/preflight.md`
- `tests/hardware/capture-source/run-20260813-115426-hv14/environment.template.md`
- `tests/hardware/capture-source/run-20260813-115426-hv14/operator-checklist.md`
- `tests/hardware/capture-source/run-20260813-115426-hv14/start-measurement.ps1`
- `tests/hardware/capture-source/run-20260813-115426-hv14/collect-evidence.ps1`
- `tests/hardware/capture-source/run-20260813-115426-hv14/capability/summary.template.json`
- `tests/hardware/capture-source/run-20260813-115426-hv14/correlation/summary.template.json`
- `tests/hardware/capture-source/run-20260813-115426-hv14/quality/manifest.template.json`
- `tests/hardware/capture-source/run-20260813-115426-hv14/aggregate/summary.template.json`
- `tests/hardware/capture-source/run-20260813-115426-hv14/decision.template.md`

**수정**

- `src/shared-contracts/schemas/index.ts` — `capture-source` export
- `src/shared-contracts/index.ts` — `dto/capture-source` export
- `src-tauri/src/contracts/dto.rs` — capture-source DTO와 상수
- `src-tauri/src/capture/mod.rs` — 신규 모듈 2개 등록
- `src-tauri/src/capture/sidecar_client.rs` — capability·source object 이벤트 수신과 correlation 조회
- `sidecar/canon-helper/src/CanonHelper/HelperVersion.cs` — additive protocol v2
- `sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs` — capability·source object 이벤트 계약
- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs` — 비교 lane 연결과 helper event 발신
- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs` — request 단위 완료, 설정·복원, paired JPEG, 고정 captureId
- `sidecar/canon-helper/src/CanonHelper/Runtime/WindowsShellThumbnail.cs` — 실측 크기·비용 계측
- `docs/contracts/camera-helper-sidecar-protocol.md` — `fastPreviewKind` 정정, 메시지 v2 추가분
- `docs/contracts/camera-helper-edsdk-profile.md` — multi-object download, capability probe
- `sidecar/canon-helper/tests/CanonHelper.Tests/JsonFileProtocolTests.cs` — correlation event 직렬화 검증
- `_bmad-output/planning-artifacts/architecture.md` — Story 7.3 implementation note
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` — Story 7.3 행
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 상태 전환
- `_bmad-output/implementation-artifacts/7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교.md` — 이 파일

## Change Log

- 2026-08-16: Noah Lee가 HV-14 fast-source 후보의 `Technology No-Go`와 `raw-original + pinned darktable 5.4.1` 운영 경로를 공식 승인했다. 후보는 계속 비활성으로 유지하고, 조사 Story 7.3을 `done`으로 닫았다. Story 7.4의 경로 선행 조건만 해제하며 HV-15 화질·환경 증거는 별도로 유지한다.
- 2026-08-14: code review batch patch 9건 반영. paired JPEG 대기 15초·timeout abandon, helper reject→표본 사유 매핑, descriptor 없이도 현재 RAW+JPEG면 paired 활성, JPEG claim rollback, candidate 읽기 실패를 extraction-failed로, ImageQuality restore held 유실 방지, BA에서 shell 기준선 고정 문서화. Status는 HV-14 실장비 재검증 전이므로 `review` 유지.
- 2026-08-14: 연결된 EOS 700D로 HV-14를 직접 실행했다. preflight와 종료 후 self-check는 `camera-ready`였지만 세 회차가 각각 `capture-download-timeout`, `camera-busy`, `capture-download-timeout`으로 중단됐다. 저장된 17 captures의 51 route 행은 전부 거부됐다(Route A/B `orientation-unsupported`, Route C `absent`). qualifying 35회는 미완결이므로 hardware ledger를 No-Go로 갱신하고 Status는 `review` 유지.
- 2026-08-13: 최종 HV-14 코드 리뷰 조치 6건을 일괄 반영했다. source freshness를 host monotonic clock으로 통일하고, 불완전 AB block 재사용과 correlation 없는 paired JPEG 승인을 차단했다. 증거 수집은 staging 성공 후 확정하도록 바꿨고, launcher의 vendored SDK fallback과 구조화된 최종 증거 게이트·운영 템플릿을 추가했다. capture-source 20건, capture readiness 단일 스레드 60건, HV-14 합성 게이트 15개 시나리오와 PowerShell 구문 검증이 통과했다. 실장비 35회 회차는 아직 남아 Status는 `review` 유지.
- 2026-08-13: HV-14 실장비 preflight를 실행했다. NOAH_WIN에서 EOS 700D 1대, EDSDK 13.19.0 초기화, helper `camera-ready`, 저장 공간과 측정 lane 초기 상태를 확인했다. 회차 전 운영자 확인 템플릿, 안전한 AB 실행 스크립트, 세션 증거 수집·완결성 검사 스크립트를 준비했다. 펌웨어·렌즈·카드·전원·케이블·측정 전 Image Quality가 기록되기 전에는 실행이 차단된다. Qualifying 35회 촬영은 아직 `Not run`이므로 Status는 `review` 유지.
- 2026-08-13: 4차 HV-14 증거 패키지 코드 리뷰 조치 10건을 일괄 반영했다. 셔터 예산을 AB 35회로 바로잡고, completeness gate를 35 request × 3 route 매트릭스·warm-up/측정·단일 session/seed·AB/BA 순서까지 강화했다. telemetry-only와 최종 증거 패키지의 PASS/FAIL 경계 및 결함 6종을 반복 검증하는 합성 테스트를 추가했다. ledger와 700D `.cr2` 예시를 현재 상태로 정정했다. 구현 리뷰는 완료되어 Status를 `review`로 전환했으며, HV-14는 `Not run` 상태다.
- 2026-08-12: 3차 Canon Helper 코드 리뷰 조치. Route B를 실제 helper 호출에 연결하고, descriptor 기반 ImageQuality 설정·복원, RAW 기준 부분 성공, object 순서와 무관한 captureId, unknown role 거부, capability/source-object protocol v2 이벤트를 구현했다. helper 41개, Rust 단위 105개, capture-source 통합 17개, TS 계약 18개와 핵심 capture 회귀 테스트가 통과했다. Status는 HV-14와 후속 문서 리뷰가 남아 `in-progress` 유지.
- 2026-08-12: **Route A 의존성 결정 승인 — LibRaw 미채택, 의존성 없는 CR2 TIFF IFD 파싱 채택.** `src-tauri/src/capture/embedded_jpeg.rs`가 IFD#0의 `StripOffsets`/`StripByteCounts`로 내장 full-size JPEG을 꺼내고 Story 7.2의 `probe_jpeg`로 검증한다. 새 crate가 없으므로 LGPL-2.1/CDDL 게이트와 Story 7.7 offline 인벤토리 영향이 둘 다 사라졌다. route 이름을 `libraw-embedded-jpeg` → `embedded-jpeg`로, lane 모드를 `libraw` → `embedded`로 개명했다 — 계약이 쓰지도 않는 라이브러리를 이름에 담지 않는다. 추출 위치를 helper(C#)에서 host(Rust)로 옮겼다: 원안의 근거("host에 native RAW 의존성을 들이지 않는다")가 의존성과 함께 소멸했고, host에 두면 `image_probe`를 직접 호출해 판정 규칙이 한 곳에 남으며 sidecar 경계도 넓히지 않는다. 경계 조건 12건(CR3·손상·EOI 누락·orientation 2종·byte order 포함) 단위 테스트 추가. 신규 테스트 누계 93건 전부 통과, cargo 313 passed / 13 failed로 **실패 수가 기준선과 동일**.
- 2026-08-12: 구현 1차. T1·T2·T5·T6·T8 완료, T4·T7 부분 완료, T3은 의존성 결정 대기. 스토리 서술 3건을 코드로 대조해 정정했다: `fastPreviewKind` 실제 값은 2개가 아니라 4개, EDSDK payload는 `BOOTHY_CANON_SDK_ROOT`로 이미 사용 가능(빌드·테스트 정상), `capture_readiness` flakiness는 병렬 경합이며 단일 스레드에서 60/60 통과. `CanonSdkCamera.cs:741`의 single-object 가드를 request 단위 완료 판정(`CaptureObjectCorrelator`)으로 교체하되 `expectedObjectCount=1`에서 오늘과 동일 동작을 유지했고, RAW 확장자 기본값을 `.cr3`에서 700D 실제 산출물인 `.cr2`로 고쳤다. 신규 자동 검증 81건(Rust 단위 29 + Rust 통합 12, C# 26, TS 14) 전부 통과, scope guard와 Story 7.2 계약 무변경을 grep으로 확인. Status `in-progress`.
- 2026-08-12: Story 7.3 컨텍스트 생성. Epic 7 AC, fast-display 연구의 LibRaw/RAW+JPEG 실측과 license gate, Canon helper 구현 실물, Story 7.2 학습을 반영해 세 route 비교(LibRaw embedded / capability-gated RAW+JPEG / Windows Shell incumbent) 범위를 확정했다. 착수 전 반드시 알아야 할 두 가지를 Dev Notes에 고정했다: 현재 helper가 capture당 transfer object를 하나만 download하고 나머지를 버린다는 것(`CanonSdkCamera.cs:741`), 그리고 계약 문서의 `fastPreviewKind: "embedded-jpeg"` 예시가 구현에 없는 값이라는 것. Status `ready-for-dev`.
