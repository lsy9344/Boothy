# Story 7.5: 상주 display renderer 검증 spike

Status: done

Type: Architecture Spike

Epic 7 dependency: `7.1 Go` → `7.2 Go` → `7.3 route decision` → `7.4 Go` → **`7.5 adoption decision`** → `7.6 Go` → … → `7.10 final decision`

> ## 시작 조건과 현재 판단
>
> - Story 7.4의 HV-15는 2026-08-16 `Go`로 종료됐다. 현재 고객 화면 경로는
>   **`raw-original + pinned darktable 5.4.1`**이며, immutable 게시와 actual-present 계측을 유지한다.
> - Story 7.3에서 조사한 embedded JPEG, paired JPEG, Windows shell thumbnail은 모두
>   `Technology No-Go`다. 따라서 **승인된 빠른 raster 입력은 아직 없다.**
> - WebGL2 또는 Direct2D 렌더러만 빨라져도 CR2 원본을 바로 읽을 수 없다. 사전 변환한 fixture만으로
>   빠른 결과를 얻어서는 production `Go`라고 기록할 수 없다.
> - 이 Story의 성공은 무조건 채택하는 것이 아니다. 실제 촬영 입력까지 연결해 기준을 통과하면 `Go`,
>   그렇지 않으면 증거가 완전한 `Technology No-Go`로 닫고 기존 darktable 정확 경로를 유지한다.
> - Story 7.4에 한해 승인된 저노출·환경 정보·시각 검토 예외는 이 Story로 넘어오지 않는다.

## 승인된 결정 (2026-08-16, Noah Lee)

착수 전 열려 있던 세 항목은 모두 승인됐다. **dev agent는 이 항목들을 다시 묻지 않는다.**

1. **predecoded/embedded raster를 spike 실험 입력으로 쓰는 것 — 승인.**
   단, **engine feasibility 자료로만 라벨링한다.** HV-14의 세 fast-source 후보는 계속
   `Technology No-Go`이자 비활성이며, 이 사용은 route 재승인이 아니다.
   실제 CR2 촬영 입력을 처리하는 승인 경로가 끝까지 서지 않으면 **AC 6에 따라 production adoption은
   `No-Go`**이고, 구현을 억지로 확장하지 않고 `Technology No-Go + darktable fallback`으로 닫는다.
   새 RAW decoder 도입은 여기 포함되지 않는다 — dependency·installer·라이선스·성능 범위를 적어
   **별도 승인**을 받는다.
2. **T0 산수 결과와 무관하게 spike를 진행 — 승인.**
   실측상 darktable 렌더 약 3.53초를 **0으로 만들어도** 실장비 종단 median 약 8.284초에서
   약 4.8초가 남아 NFR-003의 warm p50 3초에 닿지 않는다.
   이것은 중단 사유가 아니라 **T0의 산출물**이다. 다만 T0 보고서는 **남은 지연이 어느 구간
   (RAW 전송 / 렌더 큐 대기 / 게시 / present)에 있는지와 그 구간의 소유 Story(7.6 scheduler,
   7.8 성능·복구)를 반드시 지목한다.** 지목 없이 "렌더러가 빨라졌다"만 보고하면 이 Story는 닫히지 않는다.
   화질·가용성·안정성 실패는 그대로 유효한 중단 사유다 (Product Decision Rules의 `Technology No-Go` / `Blocked`).
3. **시각 parity 최종 승인자 = Noah Lee — 승인.**
   blind review 5명 패널은 **HV-16 실행 시점에 실명으로 evidence 패키지에 기록한다.**
   패널이 확정되고 서명된 결과가 남기 전까지 **AC 3은 닫히지 않으며**, 그 사실을 ledger의 열린 항목으로 유지한다.
4. **입력 축 `Technology No-Go` 종료 예외 — 승인.**
   실제 CR2를 처리할 승인된 상주 입력 경로가 없고 exact darktable fallback이 검증된 경우, 그 입력 축만으로
   Story 7.5의 adoption 결정을 `Technology No-Go`로 닫을 수 있다. blind review·MTF50·actual-present·자원 측정은
   통과로 기록하지 않으며, 거절된 후보를 재개하거나 새 direct decoder를 승인할 때 반드시 다시 수행한다.

## Story

booth 제품팀으로서,
앱이 켜져 있는 동안 계속 준비된 display renderer를 실제로 만들어 검증하고 싶다.
그래야 촬영마다 프로그램을 새로 띄우는 시간을 없앨 수 있는지, 그리고 더 빨라져도 고객이 보는 색과
디테일이 정확한지 실제 수치로 결정할 수 있다.

## Acceptance Criteria

1. **Given** 현재 승인된 세 preset(`preset-daylight`, `preset-mono-pop`, `preset-soft-glow`)과 버전이 고정된 recipe가 있을 때, **when** spike가 촬영 전에 초기화되면, **then** 렌더링 context·shader/effect/LUT·display-fit 자원을 미리 준비해야 하고, 측정 대상 hot path에서 shader compile, preset XML 순회, renderer process 시작이 발생하면 실패로 기록해야 한다.
2. **Given** 실제 촬영에서 사용할 수 있는 승인된 입력이 있을 때, **when** 상주 renderer가 display-fit 결과를 만들면, **then** `source-ready → actual-present` 지연, CPU/GPU, 메모리, driver 상태, 실패 동작을 pinned darktable one-shot 기준선과 같은 장비·같은 corpus로 비교해야 하며, 지원하지 않는 연산은 틀린 화면을 내보내지 않고 정확한 darktable 경로로 내려가야 한다.
3. **Given** 승인된 preset과 대표 EOS 700D corpus가 있을 때, **when** production `Go` 또는 후보 재활성화를 평가하면, **then** 자동 색상·디테일 지표와 사람 눈 비교가 정해진 기준을 통과해야 하며, 빠른 결과를 만들기 위해 느리거나 실패한 표본을 빼거나 시각 검토를 면제할 수 없다. 실제 입력 축에서 이미 `Technology No-Go`가 확정된 경우 미실행 항목을 통과로 기록하지 않고 재활성화 선행조건으로 이연할 수 있다.
4. **Given** WebGL2 후보가 없거나 불안정하거나 색이 맞지 않을 때, **when** 대안을 판단하면, **then** 범위를 제한한 WIC/Direct2D/D3D11 후보를 평가하거나 `Technology No-Go`를 기록해야 하며, No-Go 후보는 기본값으로 켜지지 않고 darktable 정확 fallback은 그대로 작동해야 한다.
5. **Given** 구현과 증거 수집이 끝났을 때, **when** Story 7.5를 종료 심사하면, **then** HV-16에 `Go/No-Go` 채택 결정과 그 결정을 확정한 축의 원자료, production 기본값, exact fallback을 남겨야 한다. `Go`는 raw latency·parity·stability·fallback 전체가 필요하지만, 입력 부재로 확정된 `Technology No-Go`는 입력·기준선 증거와 fallback 결정이 완전하면 미실행 채택 지표를 후속 검증으로 분리하고 spike를 종료할 수 있다.
6. **Given** 승인된 빠른 raster 입력이 현재 없을 때, **when** 후보가 predecoded fixture에서만 작동하면, **then** 그 결과는 engine feasibility 자료로만 표시하고 production `Go`로 판정하지 않는다. 실제 CR2 촬영 입력을 처리하는 승인 경로가 끝까지 없으면 HV-16 production adoption은 `No-Go`다.
7. **Given** Story 7.4의 게시 계약이 이미 검증됐을 때, **when** 상주 renderer 결과를 게시하면, **then** 기존 immutable generation, session/request/capture/preset correlation, display-fit, actual-present 종점을 그대로 사용해야 하며 별도의 pointer 또는 in-place overwrite 경로를 만들 수 없다.

## Scope Boundary

### In Scope

- 1순위 WebGL2 상주 renderer의 작동 가능한 prototype과 앱 수명주기 context
- WebGL2가 부적합할 때만 범위를 제한한 WIC/Direct2D/D3D11 대안 또는 No-Go 판단
- 세 preset recipe를 상주 renderer용으로 미리 컴파일하는 명시적 operation allowlist
- 실제 입력 가능 여부와 fixture-only 결과를 구분하는 capability gate
- Story 7.4의 immutable publisher와 actual-present telemetry를 재사용하는 prototype 연결
- 같은 장비·같은 corpus의 darktable one-shot 기준선, 성능·안정성·시각 parity 비교
- renderer/device/context 손실, 미지원 연산, timeout 때 exact fallback 검증
- HV-16 evidence와 Go/No-Go 결정

### Explicitly Out of Scope

- Story 7.3에서 No-Go된 embedded/paired/shell source route의 무근거 재승인
- predecoded fixture를 실제 촬영 production source라고 부르는 것
- Story 7.4의 generation/pointer/display-fit 계약을 새 경로로 교체하는 것
- P0/P1/P2 scheduler, RAW 정밀본 교체, stale render 취소 → Story 7.6
- installer와 offline dependency 완결 → Story 7.7
- 100-shot soak와 fault injection → Story 7.8
- rollout/rollback 및 최종 release 판정 → Story 7.9~7.10
- booth rail의 384px preview/final 경로 변경
- 품질 기준 완화 또는 Story 7.4 제품 예외 재사용

## Tasks / Subtasks

### Review Findings

- [x] [Review][Patch] `Technology No-Go` 종료 예외를 AC·DoD·HV-16 gate·ledger에 일관되게 반영한다 — 2026-08-16 사용자 결정: 실제 CR2 입력 경로 부재로 확정된 No-Go를 Story 7.5 종료 근거로 인정하고, blind review·MTF50·actual-present·자원 측정은 채택 재개 시 필요한 후속 검증으로 분리한다. [_bmad-output/implementation-artifacts/7-5-상주-renderer-검증-spike.md:31]
- [x] [Review][Patch] `shadow` 결과가 고객 화면 게시 경계를 통과하지 못하도록 강제한다 [src-tauri/src/display/resident_renderer.rs:546]
- [x] [Review][Patch] WebGL 필수 program 누락 시 stale/부분 frame을 성공으로 반환하지 않고 즉시 fallback한다 [src/resident-renderer/engine/webgl2-resident-engine.ts:438]
- [x] [Review][Patch] 지원하지 않는 exposure deflicker mode를 compile 단계에서 거부한다 [src/resident-renderer/engine/webgl2-resident-engine.ts:523]
- [x] [Review][Patch] 비동기 prewarm 취소·0 크기 전환에서 renderer 상태와 GPU context를 확실히 정리한다 [src/resident-renderer/state/use-resident-renderer.ts:108] — 취소 분기는 await 이후 누수가 없음을 확인했고, 0 크기 표면은 즉시 IDLE로 파생한다.
- [x] [Review][Patch] shader hash 구분자의 실제 NUL 문자를 안전한 소스 표현으로 교체한다 [src/resident-renderer/engine/webgl2-resident-engine.ts:229]
- [x] [Review][Patch] frontend recipe compile 실패 이유를 누락하지 않고 preset refusal telemetry에 보존한다 [src/resident-renderer/engine/webgl2-resident-engine.ts:302]
- [x] [Review][Patch] production eligibility와 무관하게 hot-path compile/process-start 발생을 AC 1 실패로 기록한다 [src-tauri/src/contracts/dto.rs:2246]
- [x] [Review][Patch] resident provenance의 입력 producer·GPU identity·context/source 시간 순서를 검증한다 [src-tauri/src/contracts/dto.rs:2218]
- [x] [Review][Dismiss] resident 게시 시점의 최신 viewer epoch를 사용해 stale generation 게시를 차단한다 [src-tauri/src/display/resident_renderer.rs:524] — publish job이 게시 시점 epoch를 입력으로 받으며 production 비동기 caller가 없어 추가 결함이 아님을 확인했다.
- [x] [Review][Patch] present report의 outcome·rejectReason·timestamp 조합을 상호 일관되게 검증한다 [src/shared-contracts/schemas/viewer-display.ts:517]
- [x] [Review][Patch] viewer-display/v3 테스트 fixture에 필수 `presentTelemetryEnabled` 값을 반영한다 [src/display-generation/state/use-display-pointer.test.tsx]
- [x] [Review][Patch] PPM parser가 CRLF/복수 공백과 비정상 대형 크기를 안전하게 처리하도록 제한한다 [src/quality-metrics/parity-metrics.ts:61]
- [x] [Review][Patch] 크기가 다른 parity pair를 측정 완료로 집계하지 않도록 fail closed한다 [src/quality-metrics/parity-metrics.ts:572]
- [x] [Review][Patch] SSIM 계산에서 8x8 배수 밖의 가장자리 픽셀을 누락하지 않는다 [src/quality-metrics/parity-metrics.ts:354] — 부분 블록까지 픽셀 가중해 포함하고 실제 CR2 24쌍 evidence를 재생성했다.
- [x] [Review][Patch] highlight/shadow clipping 판정의 채널 기준을 일관되게 적용한다 [src/quality-metrics/parity-metrics.ts:393] — 양쪽 모두 any-channel 기준으로 고정하고 실제 CR2 24쌍 evidence를 재생성했다.
- [x] [Review][Patch] HV-16 gate의 필수 파일에 neutral parity 원자료를 포함한다 [tests/hardware/resident-renderer/hv-16/tools/check-resident-evidence.ps1:37]
- [x] [Review][Patch] WIC baseline timing 값의 공백·비수치·음수 데이터를 gate에서 거부한다 [tests/hardware/resident-renderer/hv-16/tools/check-resident-evidence.ps1:77]
- [x] [Review][Patch] decision verdict를 본문 substring이 아닌 구조화된 단일 판정으로 검증한다 [tests/hardware/resident-renderer/hv-16/tools/check-resident-evidence.ps1:116]
- [x] [Review][Patch] generation journal의 resident provenance 필수 schema와 핵심 metric null 여부를 gate에서 검증한다 [tests/hardware/resident-renderer/hv-16/tools/check-resident-evidence.ps1:136]
- [x] [Review][Patch] ledger의 "prototype/adoption decision 없음" 문구를 실제 HV-16 No-Go 결과와 일치시킨다 [_bmad-output/implementation-artifacts/hardware-validation-ledger.md:355]
- [x] [Review][Patch] 90°/270° 방향성 검증 주장과 패키지에 포함된 재현 원자료를 일치시킨다 [tests/hardware/resident-renderer/hv-16/parity/README.md:29]
- [x] [Review][Defer] 기존 proxy 경로도 렌더 시작 시점 epoch를 게시 시점에 재사용한다 [src-tauri/src/display/proxy_publisher.rs:188] — deferred, pre-existing
- [x] [Review][Defer] 기존 proxy staging 경로가 중복 작업 사이에서 충돌할 수 있다 [src-tauri/src/display/proxy_publisher.rs:209] — deferred, pre-existing
- [x] [Review][Defer] 기존 proxy render/read 실패 경로가 staging 파일을 정리하지 않는다 [src-tauri/src/display/proxy_publisher.rs:322] — deferred, pre-existing
- [x] [Review][Defer] 기존 actual-present 수신 경로가 report viewer epoch와 generation epoch를 대조하지 않는다 [src-tauri/src/commands/display_commands.rs:1407] — deferred, pre-existing
- [x] [Review][Defer] 기존 terminal present evidence 쓰기 실패를 성공으로 반환한다 [src-tauri/src/commands/display_commands.rs:1470] — deferred, pre-existing
- [x] [Review][Defer] 기존 display telemetry map이 terminal/session 전환 뒤에도 generation metadata를 보유한다 [src-tauri/src/commands/display_commands.rs:127] — deferred, pre-existing
- [x] [Review][Defer] Story 7.2/HV-13B의 `done`과 `No-Go` 기록이 ledger 안에서 충돌한다 [_bmad-output/implementation-artifacts/hardware-validation-ledger.md:28] — deferred, pre-existing
- [x] [Review][Defer] Story 7.4/HV-15의 예외 승인과 품질 미통과 표현이 일반 `Go`와 구분되지 않는다 [_bmad-output/implementation-artifacts/hardware-validation-ledger.md:30] — deferred, pre-existing
- [x] [Review][Resolved] 전체 `pnpm build`의 기존 TypeScript 오류 20건을 해소했다 — 2026-08-16 production build 통과
- [x] [Review][Defer] viewer present 계약의 일부가 아직 실제 하드웨어 재검증보다 앞서 확정 상태로 서술되어 있다 [docs/contracts/viewer-display.md:344] — deferred, pre-existing
- [x] [Review][Resolved] Story 1.4/1.5 gate 문구를 현재 상태와 맞추고 보존 worktree를 테스트 수집에서 제외했다 — 2026-08-16 전체 Vitest 통과
- [x] [Review][Resolved] render queue 상태를 runtime root별로 격리했다 — 2026-08-16 capture readiness 60건 기본 병렬 실행 통과

### T0. 실제 입력 전제와 후보 결정표를 먼저 고정한다 (AC: 2, 4, 6)

- [x] HV-14의 세 fast-source 후보가 `Technology No-Go`임을 입력 조건으로 기록한다.
- [x] 후보별로 `입력 형식`, `CR2 직접 처리 가능 여부`, `별도 process 필요 여부`, `hot-path 비용`,
  `색 관리`, `미지원 연산`, `fallback`을 한 표로 만든다.
- [x] WebGL2를 1순위로 조사하고, WIC/Direct2D/D3D11은 WebGL2가 실패한 경우에만 시작한다.
- [x] predecoded corpus는 engine 검증 전용으로 라벨링한다. 실제 촬영 채택 근거와 합치지 않는다.
- [x] 실제 촬영 입력이 없으면 구현을 억지로 확장하지 않고 `Technology No-Go + darktable fallback` 종료가
  가능한지 결정한다. 새 RAW decoder 도입은 dependency·installer·라이선스·성능 범위를 별도 기록한다.

### T1. Resident recipe와 capability 계약을 만든다 (AC: 1, 2, 4)

- [x] 세 preset에 대해 versioned resident recipe, operation allowlist, engine/version, shader/effect/LUT hash,
  output profile(sRGB, perceptual), 입력 provenance를 정의한다.
- [x] 실제 producer renderer ID/version/build, 참조 renderer, compiled recipe hash, execution mode,
  context 초기화 시각, hot-path compile/process-start 횟수, fallback reason을 거짓 없이 구분한다.
- [x] pointer/generation 계약을 확장해야 하면 새 schema version으로 올리고 기존 v1/v2 reader 호환을 유지한다.
  CPU/GPU/memory/driver 표본은 generation과 연결된 별도 HV-16 diagnostics에 둔다.
- [x] capture hot path에서 XMP/XML을 다시 순회하지 않도록 publication 시점에 실행 계획을 만든다.
- [x] 지원하지 않는 operation, version 불일치, hash 불일치, output profile 불일치를 명확한 reason code로
  `resident-ineligible` 처리하고 exact darktable fallback으로 보낸다.
- [x] 현재 `proxyPublication`을 자동 승인으로 바꾸지 않는다. resident 후보는 자체 metrics와 blind review를
  완료한 별도 승인 근거가 있어야 한다.

### T2. 앱 수명주기 상주 prototype을 구현한다 (AC: 1, 4)

- [x] 촬영 전에 renderer context를 만들고 세 preset 자원을 precompile/prewarm한다.
- [x] viewer photo rectangle × DPR을 기준으로 출력 surface/buffer를 미리 준비한다.
- [x] measured hot path에 renderer process startup, shader compile, XML traversal이 없음을 telemetry로 증명한다.
- [x] context/device loss, WebView2 재생성, monitor/DPR 변경 때 안전하게 재초기화하며 준비되지 않은 결과는
  고객 화면에 게시하지 않는다.
- [x] 새 라이브러리나 native binary가 필요하면 정확한 version/hash/license/offline footprint를 남긴다.
- [x] 실험 mode를 최소 `off`(기본), `shadow`(비게시 비교), `evidence`(HV-16 명시 실행)로 분리하고
  알 수 없는 값은 `off`로 처리한다. HV-16 Go여도 Story 7.6 전에는 production 기본값을 바꾸지 않는다.

### T3. 기존 immutable 게시 경계에 연결한다 (AC: 2, 7)

- [x] 결과는 Story 7.4의 공통 `publish_generation_in_dir` 경계를 거쳐 완전히 기록·decode·검증된 뒤
  pointer commit과 notify가 일어나게 한다.
- [x] session/request/capture/preset identity+version/source hash/renderer profile/context epoch를 generation에
  결속한다.
- [x] 이전 capture, 다른 preset, 죽은 viewer epoch, 중복/역순 완료가 현재 화면을 바꾸지 못하게 한다.
- [x] resident 실패 시 현재 표시 중인 정상 frame을 지우거나 덮어쓰지 않고 exact fallback 결과만 새
  immutable generation으로 게시한다.
- [x] Story 7.4에서 `on`으로 확정된 proxy lane과 preview/final 경로를 회귀시키지 않는다.

### T4. 실제 촬영 입력 truth를 증명한다 (AC: 2, 6)

- [x] 입력마다 `sourceRoute`, `sourceAssetHash`, decode 단계, `sourceReadyAt`, 승인 근거를 기록한다.
- [x] EOS 700D CR2 실제 촬영에서 후보가 무엇을 직접 읽고 어떤 변환을 거치는지 재현 가능하게 남긴다.
- [x] darktable이나 다른 one-shot process가 먼저 raster를 만드는 경우 그 startup 비용을 숨기지 않고 전체
  hot path에 포함한다.
- [x] fixture-only 실험과 real-capture 실험을 evidence 디렉터리와 결과표에서 분리한다.

### T5. 성능·안정성 telemetry와 기준선을 수집한다 (AC: 1, 2, 4)

- [x] 입력 축 No-Go를 확정한 동일 corpus의 darktable/WIC 구성요소 기준선을 보존했다. 실제 카메라 종단 비교는 후보 재활성화 시 후속 검증한다.
- [x] `source-ready → actual-present` 전체 span은 production 후보가 없어 미실행으로 기록하고 Story 7.8 및 후보 재활성화 선행조건으로 분리했다.
- [x] cold/warm, preset별, 성공/실패별 표본 수와 median/p95/max를 기록한다. 느린 표본과 실패를 제외하지 않는다.
- [x] 환경과 복구 동작을 기록했다. 실제 GPU/working-set/VRAM 측정은 production 후보가 없어 미실행으로 기록하고 후보 재활성화 선행조건으로 분리했다.
- [x] WebGL2 비활성·GPU unavailable·renderer crash·unsupported operation을 주입하고 incorrect-frame 0건과
  exact fallback을 확인한다.

### T6. 시각 parity를 독립적으로 통과시킨다 (AC: 3, 5, 6)

- [x] WIC direct decoder와 pinned darktable의 neutral/preset 쌍을 보존했고, WebGL2 fixture 결과는 production parity 근거가 아님을 분리했다.
- [x] 수집 가능한 SSIM·ΔE00·clipping·upscale 결과는 기준 미달로 기록했다. MTF50·skin ROI는 미실행이며 후보 재활성화 선행조건이다.
- [x] 5명 × 30 transition blind review는 production 후보가 없어 미실행으로 기록했다. 후보 재활성화 시 objection rate ≤ 5% 검증을 수행한다.
  **최종 시각 승인자는 Noah Lee다 (2026-08-16 승인).** 패널 5명의 실명은 HV-16 실행 시점에 확정해
  evidence 패키지에 기록한다. 패널 미확정 상태에서는 AC 3을 통과로 적지 않는다.
- [x] Story 7.4의 저노출 제품 예외를 사용하지 않는다. 식별 가능한 정상 노출 corpus가 없으면 parity gate를
  통과로 기록하지 않는다.
- [x] 지표 또는 사람 검토가 실패하면 해당 후보는 기본 비활성 상태를 유지한다.

### T7. 자동 검증과 회귀 방지를 추가한다 (AC: 1, 2, 4, 7)

- [x] recipe compile/allowlist/version/hash/output-profile contract 단위 테스트
- [x] context prewarm과 hot-path no-compile/no-process-start 검증
- [x] unsupported operation, unavailable GPU, device/context loss, timeout, corrupt output fallback 테스트
- [x] immutable publication, stale/older/preset mismatch, delete-to-standby/recovery, actual-present 회귀 테스트
- [x] fixture-only candidate가 production eligible로 승격되지 않는 정책 테스트
- [x] 기존 Rust lib/viewer display, frontend lint/test/build 검증을 실행하고 기존 실패와 신규 실패를 구분한다.

### T8. HV-16 evidence와 채택 결정을 닫는다 (AC: 5)

- [x] evidence root: `tests/hardware/resident-renderer/hv-16/`
- [x] `environment.md`: PC/OS/GPU/driver/WebView2/monitor/DPR/ICC/HDR/camera/firmware/lens/USB/전원 및 unknown 표시
- [x] candidate build, source, shader/effect/LUT, binary와 dependency의 version/hash
- [x] 입력 축 No-Go를 확정한 raw 기준선과 WIC latency rows, failure/fallback 결과를 보존했다. GPU/memory/actual-present는 재활성화 선행조건으로 명시했다.
- [x] neutral/preset parity 원자료를 보존했다. blind-review 원본·Noah Lee 서명·패널 실명은 미실행으로 기록하고 재활성화 선행조건으로 명시했다.
- [x] `decision.md`에 `Go` 또는 `Technology No-Go`, 근거, production 기본값, exact fallback, 잔여 위험 기록
- [x] `Go`는 실제 촬영 입력·성능·parity·안정성·fallback을 모두 만족할 때만 가능하다.
- [x] `Technology No-Go`는 결정을 확정한 축의 원자료와 현재 darktable 정확 fallback이 완전할 때 Story를 닫을 수 있다. 미실행 채택 지표는 통과가 아니라 재활성화 선행조건으로 남긴다.
- [x] hardware ledger와 sprint status를 실제 결과로 갱신한다.

## Product Decision Rules

| 결과 | 조건 | 다음 단계 |
|---|---|---|
| `Go` | 실제 CR2 촬영 입력 연결, hot-path startup 제거, parity/안정성/fallback 전부 통과 | 후보를 기본 활성화하지 않은 채 Story 7.6에서 scheduler·RAW 교체와 통합 |
| `Technology No-Go` | 입력 부재, 성능 미달, parity 실패, 불안정 중 하나 이상이 독립적으로 채택을 차단하고 해당 축의 원자료와 exact fallback이 완전 | resident 후보 비활성 유지, 미실행 채택 지표를 재활성화 선행조건으로 남기고 darktable 경로로 Story 7.6 범위 재검토 |
| `Blocked` | 장비·corpus·측정 도구 등 필수 evidence 자체를 수집할 수 없음 | 부족한 항목과 재실행 조건을 HV-16에 명시하고 종료 판정 보류 |

## Dev Notes

### Reuse, Do Not Rebuild

- `src-tauri/src/display/generation_publisher.rs`: immutable stage → probe → admission → commit → journal 경계
- `src-tauri/src/display/proxy_publisher.rs`: proxy job, source route, preset publication 자격, generation provenance
- `src-tauri/src/commands/display_commands.rs`: viewer snapshot/update/present telemetry 경계
- `src/viewer-surface/services/viewer-host-adapter.ts`: viewer가 사용하는 유일한 host display adapter
- `src-tauri/src/viewer/present_telemetry.rs`: actual-present 종점과 correlation

### Architecture Guardrails

- darktable 5.4.1은 RAW 정밀본·final·parity oracle·exact fallback으로 유지한다.
- 새 renderer 결과도 파일 기반 immutable generation을 통해 게시한다. pointer 경로를 둘로 만들지 않는다.
- 기존 active frame보다 새 후보의 성공이 우선하지 않는다. 검증 실패나 fallback 중에는 정상 화면을 유지한다.
- “렌더러가 빠르다”와 “실제 촬영부터 화면까지 빠르다”를 분리해 기록한다.
- GPU 사용을 임의로 끈 결과를 정상 WebGL2 기준으로 취급하지 않는다.

### Evidence Baseline

- Story 7.4 darktable display-fit one-shot: CPU 약 3.53초, OpenCL 약 4.99초. 실제 pixelpipe 구간은 약 0.3초로 관측됐다.
- Story 7.4 current-lighting 실장비: 5/5 committed/presented, median 약 8.284초, p95/max 약 9.735초.
- 이 수치는 비교 기준선이지 7.5 합격선 완화 근거가 아니다.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 7.5 AC와 Epic 7 dependency
- `_bmad-output/planning-artifacts/architecture.md` — display generation, proxy, fallback 결정
- `_bmad-output/planning-artifacts/research/technical-preset-image-fast-display-research-2026-08-10.md` — 후보·지표·측정 연구
- `_bmad-output/implementation-artifacts/7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교.md` — fast source Technology No-Go
- `_bmad-output/implementation-artifacts/7-4-화면-적합-immutable-preset-proxy.md` — HV-15 Go와 현재 production route
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` — HV-16 gateboard
- `docs/contracts/viewer-display.md`, `docs/contracts/preset-bundle.md`, `docs/contracts/render-worker.md`
- Microsoft Direct2D effects overview: https://learn.microsoft.com/en-us/windows/win32/direct2d/effects-overview
- Microsoft Direct2D custom effects: https://learn.microsoft.com/en-us/windows/win32/direct2d/custom-effects
- Microsoft Direct2D overview: https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-overview
- Microsoft WebView2 performance guidance: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance

## Definition of Done

- [x] 작동 가능한 WebGL2 prototype 또는 근거가 완전한 WebGL2 No-Go
- [x] 필요할 때만 bounded WIC/Direct2D/D3D11 평가 또는 No-Go
- [x] 실제 촬영 입력 가능 여부를 fixture 실험과 분리한 판정
- [x] 입력 축 No-Go를 확정한 기준선·parity 실패·fallback raw evidence와 미실행 재활성화 조건
- [x] Story 7.4 immutable 게시와 actual-present 회귀 없음
- [x] HV-16 `Go/Technology No-Go` 결정과 production 기본값 기록
- [x] Go여도 7.6 전에는 production 기본 활성화하지 않음
- [x] No-Go여도 darktable exact fallback과 후속 계획이 유지됨

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (1M context) — `claude-opus-5[1m]`

### Debug Log References

착수 전 스토리의 전제를 실측으로 대조했고, **두 가지가 스토리 예상과 달랐다.**

| 스토리/사전 예상 | 실측 | 조치 |
| --- | --- | --- |
| WebGL2가 preset 연산을 대신하면 렌더가 빨라진다 | **preset 연산은 렌더의 1.26%다.** 세 preset median 3716 ms, preset 연산이 전혀 없는 중립 렌더 median 3669 ms — 차이 **47 ms**. 나머지 98.7%는 RAW 디코드와 process startup이다 | 1순위 후보의 상한을 47 ms로 확정하고 T0 결정표의 근거로 삼았다 |
| 실제 CR2를 직접 읽을 수 있는 승인 후보가 없다 | **Windows가 CR2를 in-process로 디코드한다.** `Microsoft Raw Image Decoder`(`Microsoft.RawImageExtension` 2.5.24.0), median **1284 ms**, 5208×3476 Rgb24 | HV-14가 닫은 세 후보와 다른 성질의 경로이므로 AC 4의 bounded WIC 평가로 정식 측정하고, dependency 승인 범위를 Story 7.7로 넘겼다 |

구현 중 확인한 것들:

- **축소 디코드가 빨라지지 않는다.** WIC에 1620px 목표를 줘도 1311 ms로 full 디코드(1284 ms)와 같다.
  RAW decoder가 전체 demosaic을 먼저 끝내기 때문이며, "작게 뽑으면 빠를 것"이라는 가정이 틀렸다.
- **`default-render-template.xmp`으로는 상주 계획을 만들 수 없다.** 이 템플릿은
  `<darktable:module>` 형식이라 적용할 파라미터가 없다. "아마 아무것도 안 하는 것"으로 가정하면
  중립 렌더가 preset 렌더로 둔갑하므로 `resident-recipe-unresolvable`로 거절하고 테스트로 고정했다.
- **`useResidentRenderer`가 매 렌더마다 GPU context를 다시 세우는 결함을 테스트가 잡았다.**
  호출자가 넘기는 `{ getResidentRendererPlan }` 객체 리터럴이 effect 의존성에 있어
  15,014회 재준비가 일어났다. 서비스를 ref로 붙잡고 의존성에서 뺐다.
  이후 lint가 "렌더 중 ref 쓰기"를 지적해 동기화를 effect로 옮겼다.
- **CIEDE2000 검증 표를 잘못 옮겨 적었다.** `(35.0831, -44.1164, 3.7933)` 쌍의 기대값을
  1.8731로 적었으나 실제 Sharma 표 값은 **1.8645**이고 1.8731은 `(61.2901, 3.7196, -5.3901)` 쌍의 값이다.
  구현이 아니라 내 테스트 데이터가 틀렸다. 두 쌍을 모두 넣어 17쌍으로 고정했다.
- **PowerShell 게이트가 조용히 한 줄을 삼켰다.** Windows PowerShell 5.1이 BOM 없는 UTF-8을 ANSI로
  읽어 한글 주석의 바이트가 line-continuation으로 오독되고, 바로 다음 statement가 주석에 흡수됐다.
  `$trimmed`가 `null`이 되어도 오류가 나지 않아 원인이 보이지 않았다. 두 스크립트에 UTF-8 BOM을 붙였다.
  (같은 패턴의 HV-15 스크립트도 BOM이 없다 — 지금은 동작하지만 같은 위험이 있다.)
- **parity 참조를 처음에 잘못 골랐다.** 중립 렌더를 기준으로 삼았는데 그 산출물의 평균 luma가
  0.0064(정규화)로 사실상 검은 화면이었다. 그 상태의 ΔE00은 디코더 차이가 아니라 노출 차이를 잰다.
  세 preset 렌더를 참조로 추가하고, 두 결과를 모두 보존하면서 단서를 함께 적었다.
- **회전 방향이 결과를 좌우하지 않음을 확인했다.** corpus 35장이 전부 세로 촬영이고 WIC는 EXIF 회전을
  적용하지 않는다. 90°와 270°를 모두 측정했고 SSIM이 0.0212 / 0.0219로 거의 같았다 —
  차이의 원인은 회전이 아니라 톤·색 파이프라인이다.

### Completion Notes List

#### 판정

**HV-16 = `Technology No-Go`.** production 채택하지 않고 `raw-original + pinned darktable 5.4.1`을 유지한다.
근거는 두 개의 실측이다.

1. **입력 축**: WebGL2는 CR2를 읽지 못한다. 누군가 raster를 먼저 만들어야 하는데
   그 비용이 preset까지 적용한 완성본을 만드는 비용과 같다 (47 ms 차이).
2. **거리 축**: 렌더를 **0으로 만들어도** 8284 − 3716 = **4568 ms**가 남아 NFR-003의 warm p50 3000 ms에 닿지 않는다.
   상주 direct decoder를 써도 5855 ms다.

남은 지연의 소유는 **Story 7.6**(렌더 큐 대기, 게시)과 **Story 7.8**(카메라 RAW 전송, present)이다.

#### 무엇이 완료되었는가

- **T0** — 420회 darktable 실측 + 72회 WIC 실측으로 후보 결정표를 세웠다. 추정이 아니라 측정이다.
- **T1** — `resident-recipe/v1` 계약. 실제 게시된 세 preset이 컴파일되는지를 테스트한다(합성 fixture가 아니다).
  mask, 알 수 없는 blendop, modversion 불일치, allowlist 밖 연산을 각각 고유 코드로 거절한다.
  `viewer-display/v2` → `v3`로 올려 **만든 renderer와 비교 기준 renderer를 분리**했고 v1/v2 읽기를 유지했다.
- **T2** — WebGL2 상주 엔진. prewarm에서 프로그램 5종을 전부 컴파일하고, hot path의 컴파일 횟수를
  **관측값으로** 남긴다. 프로그램 집합이 preset 수와 무관해 새 preset이 hot path에 컴파일을 끌고 들어오지 않는다.
  context loss / DPR 변경 / GPU 부재 / 예산 초과가 전부 fallback으로 닫힌다. **새 의존성은 추가하지 않았다.**
- **T3** — `publish_generation_in_dir`을 그대로 통과한다. tier도 pointer 경로도 새로 만들지 않았고,
  older-capture / preset-mismatch / delete-to-standby guard에 똑같이 걸리는 것을 통합 테스트로 고정했다.
- **T4** — `sourceRoute` / `sourceAssetHash` / `inputProvenance` / `sourceReadyAtMicros` /
  `inputStartupCostMicros`를 generation에 싣는다. one-shot이 raster를 먼저 만든 경우
  startup 비용을 숨길 수 없고 그 자체가 채택 불가 사유(`resident-one-shot-startup-not-eliminated`)다.
- **T6 (부분)** — 지표 도구를 만들고 **검증**했다. CIEDE2000은 공개 Sharma 검증 표 17쌍으로 단위 테스트한다.
  bounded WIC parity 24쌍을 측정했고 전부 기준 미달이다 (SSIM 0.013–0.159 / 기준 ≥ 0.95).
- **T7** — 아래 수치 참조. 게이트 자체를 self-test로 검증했다 (정상 2건 통과 + 심어 둔 결함 11종 전부 차단).
- **T8** — evidence 패키지와 `decision.md`, ledger, sprint status.

#### 완료하지 못한 것 (통과로 적지 않았다)

| 항목 | 이유 | 판정에 미치는 영향 |
| --- | --- | --- |
| 5명 × 30 transition blind review | 패널 미확정. T6이 "패널 미확정 상태에서는 AC 3을 통과로 적지 않는다"고 명시 | **AC 3은 통과가 아니다.** ledger의 열린 항목으로 남겼다 |
| MTF50 | 도구(`mtf50FromSlantedEdge`)는 구현·검증했으나 HV-14 corpus에 적합한 slanted-edge 대상이 없다. 자연 사진에서 자동 탐지하면 엉뚱한 영역을 재고 그 숫자가 evidence에 남는다 | 없음. 측정해도 47 ms 상한은 그대로다 |
| skin ROI ΔE00 | ROI는 호출자가 지정해야 한다. 이번 매니페스트는 `skinRoi: null` | 없음 |
| 실장비 촬영 회차 (T5의 카메라·화면·span, CPU/GPU 사용량·peak working set/VRAM, actual-present 완결성) | 이 회차는 **구성요소 수준 측정**이다. 카메라를 연결하지 않았고 상주 후보가 실제 촬영을 처리할 승인 경로가 없어 측정 대상 자체가 서지 않는다 | 없음. 판정은 입력 축에서 이미 결정된다 |
| 상주 엔진의 실제 GPU 실행 시간 | headless 환경에 WebGL2가 없다 | 없음 |

승인 결정 #1이 "실제 촬영 입력을 처리하는 승인 경로가 끝까지 서지 않으면 구현을 억지로 확장하지 않고
`Technology No-Go + darktable fallback`으로 닫는다"이므로, 위 항목들을 채우려고 범위를 넓히지 않았다.

#### 계약상 절충 (숨기지 않는다)

- **context epoch**는 별도 카운터가 아니라 `contextInitializedAtMicros`로 기록된다.
  context 인스턴스를 유일하게 식별하므로 목적은 같지만, 스토리 문구와 필드 이름이 다르다.
- **CPU/메모리/driver 표본**은 generation이 아니라 evidence 패키지(`environment.md`, `baseline/`)에 있다.
  GPU vendor/renderer만 **신원**으로 provenance에 실었다. 실촬영 회차가 없어 generation과 1:1로 묶을
  표본 자체가 생기지 않았다.
- **`RESIDENT_KNOWN_BLENDOP_PARAMS`는 값을 해석하지 않는다.** darktable의 `blendop_params`는 압축 blob이라
  알려진 두 값과 정확히 같을 때만 통과시킨다. 모르는 blend를 "아마 기본값"으로 가정하지 않기 위해서다.

#### 검증 결과 (있는 그대로)

| 명령 | 결과 | 판정 |
| --- | --- | --- |
| `cargo fmt --check` | diff 없음 | 통과 |
| `cargo test --lib` | **175 / 175** | 통과 |
| `cargo test --test viewer_display` | **35 / 35** | 통과 (Story 7.4 회귀 없음) |
| `cargo test --test resident_renderer` | **7 / 7** | 신규 |
| `pnpm lint` (`eslint .`) | 통과 | 신규 경고 없음 |
| `pnpm test:run` (전체 Vitest) | **533 passed / 1 skipped** | 통과. 보존 worktree는 제품 테스트 수집 범위에서 제외했다 |
| Story 7.5 관련 Vitest | **512 / 512** | 36 files |
| HV-16 evidence gate | **PASS** (baseline 420행 / WIC 72행 / 승인 decoder 0) | |
| HV-16 gate self-test | **정상 2건 통과 + 심어 둔 결함 11종 전부 차단** | |
| `pnpm build` (`tsc -b` + Vite production build) | **통과** | 기존 TypeScript 오류 20건 해소 |
| `cargo test` (전체 Rust) | **통과** | capture readiness 60건을 포함해 기본 병렬 실행 통과 |

#### 이 Story가 남긴 가장 유용한 사실

세 preset 연산의 총비용이 렌더의 **1.26%**라는 실측이다.
이 숫자가 있으면 앞으로 "렌더러를 바꾸면 빨라진다"는 제안을 측정 없이 걸러낼 수 있고,
Story 7.6이 렌더러가 아니라 **큐와 전송**에서 시작해야 하는 이유가 된다.

### File List

**신규 — host (Rust)**

- `src-tauri/src/display/resident_renderer.rs`
- `src-tauri/src/commands/resident_renderer_commands.rs`
- `src-tauri/tests/resident_renderer.rs`

**수정 — host (Rust)**

- `src-tauri/src/contracts/dto.rs`
- `src-tauri/src/display/mod.rs`
- `src-tauri/src/display/generation_publisher.rs`
- `src-tauri/src/display/proxy_publisher.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/display_commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tests/viewer_display.rs`

**신규 — frontend**

- `src/resident-renderer/engine/darktable-params.ts`
- `src/resident-renderer/engine/darktable-params.test.ts`
- `src/resident-renderer/engine/resident-recipe.ts`
- `src/resident-renderer/engine/shaders.ts`
- `src/resident-renderer/engine/webgl2-resident-engine.ts`
- `src/resident-renderer/engine/webgl2-resident-engine.test.ts`
- `src/resident-renderer/services/browser-resident-context.ts`
- `src/resident-renderer/services/resident-renderer-host-adapter.ts`
- `src/resident-renderer/state/use-resident-renderer.ts`
- `src/resident-renderer/state/use-resident-renderer.test.tsx`
- `src/quality-metrics/parity-metrics.ts`
- `src/quality-metrics/parity-metrics.test.ts`

**수정 — frontend**

- `src/shared-contracts/schemas/viewer-display.ts`
- `src/shared-contracts/display.contracts.test.ts`
- `src/viewer-surface/ViewerSurface.tsx`
- `src/viewer-surface/ViewerSurface.test.tsx`
- `src/display-generation/services/display-guard.test.ts`
- `src/display-generation/state/use-display-pointer.test.tsx`
- `src/display-generation/state/use-measurement-lane-state.test.tsx`

**신규 — HV-16 evidence**

- `tests/hardware/resident-renderer/hv-16/README.md`
- `tests/hardware/resident-renderer/hv-16/decision.md`
- `tests/hardware/resident-renderer/hv-16/environment.md`
- `tests/hardware/resident-renderer/hv-16/t0-input-premise.md`
- `tests/hardware/resident-renderer/hv-16/baseline/darktable-oneshot-latency.csv`
- `tests/hardware/resident-renderer/hv-16/baseline/wic-cr2-decode.csv`
- `tests/hardware/resident-renderer/hv-16/baseline/summary.json`
- `tests/hardware/resident-renderer/hv-16/capability/engine-contract.json`
- `tests/hardware/resident-renderer/hv-16/parity/README.md`
- `tests/hardware/resident-renderer/hv-16/parity/wic-vs-darktable-neutral.json`
- `tests/hardware/resident-renderer/hv-16/parity/wic-vs-darktable-preset.json`
- `tests/hardware/resident-renderer/hv-16/tools/hv16-parity.test.ts`
- `tests/hardware/resident-renderer/hv-16/tools/check-resident-evidence.ps1`
- `tests/hardware/resident-renderer/hv-16/tools/test-check-resident-evidence.ps1`

**수정 — 문서와 sprint 아티팩트**

- `docs/contracts/viewer-display.md`
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/7-5-상주-renderer-검증-spike.md`

## Change Log

- 2026-08-16: 별도 소유 스토리가 없던 리뷰 후속 3건을 즉시 해소했다. 전체 production build, Vitest, Rust, .NET helper, lint 검증이 모두 통과한다.
- 2026-08-16: 남은 SSIM 가장자리와 clipping 정의를 확정하고 실제 HV-14 CR2 6장으로 neutral 6쌍·preset 18쌍 evidence를 전부 재생성했다. 품질 실패와 `Technology No-Go` 판정은 유지됐고, 리뷰 patch 21건을 모두 닫아 상태를 `done`으로 전환했다.
- 2026-08-16: 코드 리뷰의 비논쟁 수정 19건을 반영하고 검증했다. 별도 제품 판단과 evidence 재생성이 필요한 SSIM 가장자리 및 clipping 정의 2건은 열린 항목으로 유지해 상태를 `in-progress`로 조정했다.
- 2026-08-16: HV-16을 `Technology No-Go`로 종료했다. 실측 근거는 두 가지다. (1) pinned darktable one-shot 420회에서 세 preset 연산의 총비용이 렌더의 **1.26%**(3716 ms 중 47 ms)로, WebGL2가 preset 연산을 대신해도 상한이 47 ms다. (2) display 렌더를 **0으로 만들어도** 실장비 종단 median 8284 ms에서 4568 ms가 남아 NFR-003 warm p50 3000 ms에 닿지 않는다. 측정 중 Windows가 `Microsoft.RawImageExtension` 2.5.24.0으로 CR2를 in-process 디코드할 수 있다는 새 사실(median 1284 ms)을 확인했으나, 미승인 Store dependency이고 parity가 미달(SSIM 0.013–0.159 / 기준 ≥ 0.95)이라 production 채택 대상이 아니며 dependency 판정은 Story 7.7로 넘겼다. 상주 후보는 기본 비활성(`off`)이고 `raw-original + pinned darktable 5.4.1`이 그대로 production route다. blind review 패널과 MTF50은 **열린 항목**으로 남겨 AC 3을 통과로 기록하지 않았다. 상태를 `review`로 올렸다.
- 2026-08-16: Story 7.5 구현. `resident-recipe/v1` 계약과 WebGL2 상주 엔진 prototype, `viewer-display/v2` → `v3` 승격(만든 renderer와 비교 기준 renderer 분리, v1/v2 읽기 유지), fixture 결과의 production 승격을 게시 경계 앞에서 막는 기계적 gate, 검증된 parity 지표 도구(CIEDE2000 Sharma 17쌍), HV-16 evidence 패키지와 self-test된 완결성 gate를 추가했다.
- 2026-08-16: Noah Lee가 착수 전 열려 있던 세 결정을 승인했다. (1) predecoded/embedded raster를 spike 실험 입력으로 쓰되 engine feasibility 자료로만 라벨링하고 HV-14 후보는 계속 비활성으로 둔다, (2) T0의 산수가 "렌더러를 0으로 만들어도 warm p50 3초에 못 닿는다"로 나와도 중단하지 않고 진행하되 남은 지연 구간과 그 소유 Story(7.6/7.8)를 반드시 지목한다, (3) 시각 parity 최종 승인자는 Noah Lee이며 blind 패널 5명 실명은 HV-16 실행 시 evidence에 기록한다. 승인 내용을 「승인된 결정」 절과 T6·T8에 반영했다.
- 2026-08-16: Story created from Epic 7.5, HV-14/HV-15 decisions, current renderer seams, and official WebGL2/Direct2D guidance. Status set to `ready-for-dev`.
