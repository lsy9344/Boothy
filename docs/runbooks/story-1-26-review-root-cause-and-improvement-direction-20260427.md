---
documentType: review-root-cause
status: active
date: 2026-04-27
scope: story-1-26-preview-route
---

# Story 1.26 Review Root Cause And Improvement Direction

## 이 문서의 목적

이 문서는 Story `1.26` code review findings를 기준으로, 왜 latest fast run을 official `Go`로 볼 수 없는지와 다음 개선 방향을 정리한다.

핵심 결론은 단순하다.

- latest run은 `preset-applied visible <= 3000ms`에 가까운 속도 증거를 만들었다.
- 하지만 그 증거는 아직 제품 성공이 아니다.
- 이유는 현재 경로가 `truthful preset look`과 `host-owned reserve path`라는 Story `1.26`의 제품 경계를 동시에 만족하지 못하기 때문이다.

## 쉬운 용어 정리

- `darktable`: RAW 사진을 JPG로 현상하는 외부 프로그램이다.
- `preset`: darktable이 어떤 색감과 보정으로 현상할지 알려주는 레시피다.
- `XMP`: preset 레시피가 들어 있는 파일이다.
- `truthful preview`: 손님에게 "이게 최종 확인용 프리셋 결과"라고 말해도 되는 preview다.
- `fast preview`: 빠르게 먼저 보여줄 수 있는 preview다. 빠르다고 항상 truthful은 아니다.
- `host-owned reserve path`: 매 촬영마다 외부 darktable hot path에 기대는 대신, 앱 host가 소유하는 빠른 preview 경로다.

중요한 구분:

- darktable은 preset이 아니다.
- darktable은 preset을 실행하는 도구다.
- preset은 darktable에게 주는 보정 레시피다.

## 왜 통과하지 못했는가

Story `1.26`의 합격 조건은 "3초 안에 이미지가 나온다" 하나가 아니다.

공식 합격은 아래를 모두 만족해야 한다.

1. 손님에게 보이는 close asset이 진짜 preset-applied 결과여야 한다.
2. 그 결과가 `originalVisibleToPresetAppliedVisibleMs <= 3000ms` 안에 보여야 한다.
3. close owner가 `host-owned local native/GPU resident full-screen lane`이어야 한다.
4. first-visible나 중간 preview가 먼저 보이더라도 `Preview Waiting` truth가 깨지면 안 된다.
5. same-session, same-capture, wrong-capture discard, cross-session leakage 0을 유지해야 한다.
6. 실패하면 story는 `review` 또는 route-hold 상태에 남아야 한다.

latest fast run은 2번에 가까운 증거를 만들었다.

하지만 1번과 3번이 닫히지 않았다.

그래서 결론은:

> 속도 가능성은 보였지만, 제품이 약속한 truthful close와 새 reserve path는 아직 증명되지 않았다.

## 정확한 원인

### 1. 빠른 preview용 XMP가 preset 일부를 제거한다

위치:

- `src-tauri/src/render/mod.rs:44`

현재 빠른 preview 경로는 XMP에서 일부 darktable operation을 제거한다.

리뷰에서 문제로 본 operation:

- `highlights`
- `cacorrectrgb`

이것들은 단순히 "RAW 파일을 읽는 데만 필요한 단계"라고 확정하기 어렵다. 사진의 밝은 영역 복구나 색 번짐 보정처럼 실제 look에 영향을 줄 수 있다.

따라서 이 operation을 제거한 결과를 `preset-applied-preview`라고 부르면 위험하다.

정확한 원인:

- 속도를 위해 preset 레시피 일부를 줄였다.
- 그런데 줄인 레시피가 원본 preset look과 같은지 증명하지 않았다.
- 그 상태에서 결과를 truthful close로 승격했다.

제품 영향:

- 손님에게 보이는 확인용 사진이 실제 preset 결과와 다를 수 있다.
- 그런데 시스템은 그 사진을 성공으로 기록할 수 있다.
- 즉, "빠른데 틀린 사진"이 official pass처럼 보일 수 있다.

개선 방향:

- look-affecting 가능성이 있는 operation 제거를 truthful path에서 금지한다.
- trimmed XMP 결과는 `comparison evidence` 또는 `fast non-truth preview`로만 기록한다.
- truthful close로 쓰려면 full preset 결과와 visual parity 검증을 통과해야 한다.

### 2. main preview path가 아직 per-capture darktable에 의존한다

위치:

- `src-tauri/src/render/mod.rs:2170`
- `src-tauri/src/render/mod.rs:2268`

현재 빠른 결과는 여전히 매 촬영마다 darktable process를 실행하는 흐름에 가깝다. 여기에 Windows high priority까지 적용해 preview darktable process를 더 빨리 끝내려 한다.

이 방식은 속도 증거로는 의미가 있다.

하지만 Story `1.26`이 열었던 새 방향과는 다르다.

Story `1.26`의 방향:

- host가 소유하는 local native/GPU resident path
- display-sized preset-applied truthful artifact
- darktable은 parity reference, fallback, final/export truth로 남김

현재 문제 경로:

- per-capture darktable hot path를 계속 사용
- darktable 실행 옵션과 process priority로 tail latency를 줄임
- 결과를 official close처럼 읽을 수 있음

정확한 원인:

- 새 host-owned reserve path를 닫기 전에, 기존 darktable path 최적화가 main success evidence로 남았다.
- 문서상 이 run은 latency comparison evidence인데, 코드상 main preview path와 섞여 있다.

제품 영향:

- "새 아키텍처가 성공했다"가 아니라 "기존 외부 renderer를 더 세게 돌렸더니 빨랐다"가 된다.
- 하드웨어 상태, Windows scheduler, darktable process jitter에 계속 묶인다.
- 같은 환경이 아니면 성공이 흔들릴 수 있다.

개선 방향:

- preview darktable high priority run을 official `Go` 근거에서 제외한다.
- high priority darktable result는 comparison evidence로만 남긴다.
- official close owner는 host-owned path가 만든 artifact만 허용한다.
- host-owned path가 준비되지 않았으면 Story `1.26`은 `No-Go / in-progress`로 유지한다.

### 3. warm-up 실패가 성공처럼 cache된다

위치:

- `src/session-domain/state/session-provider.tsx:779`

preview runtime warm-up은 첫 촬영 전에 renderer를 미리 깨워 두는 역할이다.

현재 문제는 warm-up이 실패해도 같은 session/preset key가 이미 처리된 것처럼 남을 수 있다는 점이다.

정확한 원인:

- warm-up promise의 error를 삼킨다.
- promise는 비워지지만, prime key는 그대로 남는다.
- 다음 호출은 같은 key라고 보고 즉시 성공처럼 지나갈 수 있다.

제품 영향:

- 한 번의 일시 실패 때문에 이후 촬영들이 cold path로 들어갈 수 있다.
- 첫 컷 또는 다음 컷의 preview latency가 튈 수 있다.
- 검증 결과가 run마다 흔들릴 수 있다.

개선 방향:

- warm-up 실패 시 prime key도 clear한다.
- 실패를 기록하고 다음 capture 전에 retry할 수 있게 한다.
- warm-up이 실패했으면 hardware validation report에 별도 원인으로 남긴다.

### 4. 오래된 helper executable이 current source보다 먼저 선택될 수 있다

위치:

- `src-tauri/src/capture/helper_supervisor.rs:221`

debug 환경에서는 현재 C# source를 실행해야 한다. 그래야 지금 고친 helper 코드가 실제 검증에 반영된다.

현재 문제는 기존에 빌드되어 있던 `canon-helper.exe`가 있으면 그 파일을 먼저 실행할 수 있다는 점이다.

정확한 원인:

- launch target 선택 순서가 existing executable을 dotnet project보다 먼저 본다.
- local debug output이 남아 있으면 current source가 아니라 stale binary가 실행될 수 있다.

제품 영향:

- 코드상 고친 내용이 검증에 반영되지 않을 수 있다.
- 실기기 run이 "current code" evidence가 아닐 수 있다.
- helper 관련 문제를 고쳤다고 생각했는데 실제로는 예전 binary로 테스트할 수 있다.

개선 방향:

- debug build에서는 dotnet project를 우선한다.
- packaged/release build에서만 bundled executable을 우선한다.
- validation report에 helper executable path와 source type을 남긴다.

### 5. connect timeout 뒤 늦게 끝난 이전 시도가 새 상태를 오염시킬 수 있다

위치:

- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs:702`

카메라 연결 시도가 timeout되면 시스템은 다음 연결 시도를 시작할 수 있다. 그런데 이전 연결 시도가 실제 SDK thread 안에서 늦게 끝나면, 새 시도 중인 상태를 바꿀 수 있다.

정확한 원인:

- timeout 때 `_connectTask`를 비우지만, 이전 attempt를 generation으로 fence하지 않는다.
- 늦게 끝난 attempt가 `_camera`, `_sessionOpen`, `_snapshot`에 영향을 줄 수 있다.

제품 영향:

- 준비 상태가 잘못 보일 수 있다.
- helper recovery가 불안정해질 수 있다.
- hardware validation에서 readiness timeout이나 잘못된 ready 상태가 섞일 수 있다.

개선 방향:

- connect attempt마다 generation id를 둔다.
- timeout되거나 superseded된 attempt는 late completion을 무시한다.
- SDK state mutation은 current generation일 때만 허용한다.

### 6. duplicate fast-preview event에서 임시 source 파일이 남을 수 있다

위치:

- `src-tauri/src/capture/ingest_pipeline.rs:288`

speculative preview render는 fast preview asset을 별도 source로 staging한다. 그런데 duplicate event에서 이미 output이나 lock이 있으면 early return한다.

현재 문제는 staging 파일을 만든 뒤 early return할 수 있다는 점이다.

정확한 원인:

- output/lock guard보다 source staging이 먼저 실행된다.
- guard에 걸리면 cleanup code까지 가지 못한다.

제품 영향:

- 세션 폴더에 request-scoped 임시 파일이 계속 쌓일 수 있다.
- 장시간 booth 운영에서 디스크와 diagnostics가 지저분해진다.
- 같은 capture의 evidence를 읽을 때 혼란이 생긴다.

개선 방향:

- output/lock guard를 source staging보다 앞으로 옮긴다.
- 또는 early return마다 staged source를 삭제한다.

### 7. hardware validation readiness timeout이 runtime recovery budget보다 짧다

위치:

- `src-tauri/src/automation/hardware_validation.rs:1244`

runtime은 실제 카메라 복구와 reconnect에 시간이 걸릴 수 있다. 그런데 validation runner는 readiness를 8초만 기다린다.

정확한 원인:

- runtime recovery window와 validation wait budget이 맞지 않는다.
- validation이 booth가 정상 복구할 수 있는 시간을 충분히 주지 않는다.

제품 영향:

- 실제 booth에서는 회복 가능한 상황이 validation에서는 실패로 기록될 수 있다.
- 실패 원인이 preview latency인지 readiness budget인지 헷갈릴 수 있다.

개선 방향:

- validation readiness timeout을 runtime reconnect + retry guard + warm-up 시간을 포함하도록 늘린다.
- readiness timeout failure에는 마지막 helper state와 elapsed budget을 명확히 남긴다.

### 8. release diff에 local artifact가 섞여 있다

위치:

- `.codex/config.toml`
- `.agents/skills/...`
- `.tmp-review-1.10.diff`
- `UsersKimYS...jpg`
- `sidecar/canon-helper/tests/CanonHelper.Tests/bin/...`
- `sidecar/canon-helper/tests/CanonHelper.Tests/obj/...`

정확한 원인:

- local tool config, temporary review file, generated image, test build output이 branch diff에 들어왔다.

제품 영향:

- 리뷰 대상이 흐려진다.
- stale binary나 local-only file이 evidence를 오염시킬 수 있다.
- release branch 재현성이 낮아진다.

개선 방향:

- release diff에서 local artifacts를 제거한다.
- `.gitignore`가 빠뜨린 test `bin/` and `obj/` 경로를 보강한다.
- validation evidence로 필요한 이미지는 `history/` 또는 명시된 evidence folder에만 넣는다.

## 개선 원칙

앞으로는 아래 순서로 고친다.

### 원칙 1: 빠른 결과와 truthful 결과를 분리한다

빠른 preview가 먼저 보일 수는 있다.

하지만 빠른 preview가 full preset look과 같다는 증거가 없으면 `previewReady`를 소유하면 안 된다.

상태 이름을 분리한다.

- `fast-visible`: 먼저 볼 수 있는 이미지
- `truthful-close`: 진짜 preset-applied close
- `previewReady`: truthful-close가 닫힌 뒤에만 허용

### 원칙 2: trimmed XMP는 기본적으로 official truth가 아니다

XMP를 줄이면 속도는 빨라질 수 있다.

하지만 줄인 결과는 반드시 full preset result와 비교해야 한다.

비교를 통과하기 전까지는:

- official gate에 쓰지 않는다.
- hardware ledger `Go` 근거로 쓰지 않는다.
- `preset-applied-preview` truth owner로 기록하지 않는다.

### 원칙 3: darktable priority run은 comparison evidence로만 읽는다

preview darktable process priority를 올린 run은 latency tail을 이해하는 데 도움은 된다.

하지만 그것만으로 Story `1.26`을 닫으면 안 된다.

그 run은 아래 의미로만 남긴다.

- system tail jitter가 줄어들 수 있다는 evidence
- current darktable hot path의 latency ceiling 참고값
- host-owned path가 달성해야 할 target band 참고값

### 원칙 4: official close owner를 하나로 고정한다

Story `1.26` official close owner는 아래 조건을 만족해야 한다.

- host-owned path가 만든 display-sized preset-applied artifact
- same-session and same-capture 검증 통과
- wrong-capture discard 통과
- cross-session leakage 0
- full preset look parity 검증 통과
- `originalVisibleToPresetAppliedVisibleMs <= 3000ms`

## 추천 실행 순서

### Phase 0: release diff 청소

먼저 local artifacts를 제거한다.

목표:

- 리뷰와 validation이 제품 변경만 보게 한다.
- stale binary가 실행될 여지를 줄인다.

해야 할 일:

- `.codex/`, `.agents/skills/superpowers/` 등 local tool config가 release diff에 들어간 이유를 확인한다.
- temporary diff와 local JPG를 제거한다.
- test `bin/`, `obj/` output을 제거한다.
- 필요한 ignore rule을 보강한다.

### Phase 1: false Go 차단

목표:

- 빠르지만 틀린 결과가 official success로 기록되지 않게 한다.

해야 할 일:

- `highlights`, `cacorrectrgb` 등 look-affecting 가능성이 있는 operation 제거를 truthful path에서 중단한다.
- trimmed XMP result를 official truth로 승격하지 않는다.
- previewReady는 truthful artifact만 소유하게 한다.
- hardware validation은 `preview.kind == preset-applied-preview`뿐 아니라 truth source도 확인한다.

### Phase 2: 안정성 finding 처리

목표:

- 검증 run이 흔들리지 않게 한다.

해야 할 일:

- warm-up 실패 시 retry 가능하게 한다.
- debug helper launch가 stale executable보다 current source를 우선하게 한다.
- Canon SDK connect attempt에 generation fence를 추가한다.
- speculative source cleanup을 보장한다.
- validation readiness timeout을 runtime recovery budget과 맞춘다.

### Phase 3: host-owned truthful path 정의

목표:

- Story `1.26`의 새 아키텍처를 실제로 닫는다.

해야 할 일:

- host-owned renderer가 어떤 preset operation을 지원할지 좁게 정한다.
- 지원하지 못하는 operation이 있으면 해당 preset은 fast truthful path 대상에서 제외한다.
- full darktable output과 host-owned output을 비교하는 parity check를 만든다.
- parity를 통과한 preset만 host-owned truthful close로 허용한다.

### Phase 4: official validation 재수집

목표:

- 제품 기준의 새 Go/No-Go를 다시 얻는다.

필수 evidence:

- one approved hardware session
- 5/5 same-session, same-capture correctness
- wrong-capture 0
- cross-session leakage 0
- `Preview Waiting -> Preview Ready` truth 유지
- full preset look parity pass
- `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
- helper executable/source path 기록
- warm-up status 기록

## 어떤 선택지를 버려야 하는가

아래는 Story `1.26` official close로 쓰면 안 된다.

- trimmed XMP가 full preset look과 같은지 검증하지 않은 결과
- Windows high priority darktable run만으로 닫은 결과
- first-visible image를 truth owner로 승격한 결과
- `sameCaptureFullScreenVisibleMs`만 빠른 결과
- 한 번 빠른 run이지만 반복 stability가 없는 결과
- local stale helper binary로 얻은 결과

## 다음 작업자가 지켜야 할 체크리스트

작업 시작 전:

- `docs/README.md`를 읽는다.
- 이 문서를 읽는다.
- `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`를 읽는다.
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에서 latest verdict를 확인한다.

코드 수정 전:

- 내가 고치는 path가 fast-visible인지 truthful-close인지 분리한다.
- official `previewReady`를 어떤 artifact가 소유하는지 적는다.
- darktable run인지 host-owned run인지 evidence에 남길 방법을 정한다.

검증 전:

- local artifact가 diff에 섞이지 않았는지 확인한다.
- helper launch source가 current source인지 확인한다.
- warm-up 실패가 retry되는지 확인한다.
- readiness timeout이 runtime recovery budget보다 짧지 않은지 확인한다.

검증 후:

- 3초 숫자만 보지 않는다.
- full preset look parity를 확인한다.
- same-capture truth를 확인한다.
- ledger에 `Go / No-Go`를 기록한다.

## 2026-04-27 추가 검증 기록

요청한 `Kim4821` hardware validation을 먼저 실행했다.

- 초기 run: `hardware-validation-run-1777260672018`
- session: `session_000000000018aa192a350d074c`
- 결과: `failed`
- 실패 코드: `preview-truth-gate-failed`
- 핵심 수치: capture 1 `originalVisibleToPresetAppliedVisibleMs=3346ms`, `preview-render-ready elapsedMs=3328ms`
- 해석: helper/camera readiness와 warm-up은 정상이었지만, current path는 여전히 per-capture `darktable-cli` close에 기대고 3초 gate도 넘었다.

이 로그를 기준으로 hardware validation runner를 보강했다. 이제 runner는 `preview.kind == preset-applied-preview`와 3초 숫자만 보지 않고, `timing-events.log`의 `preview-render-ready` detail에서 official route owner를 확인한다. 공식 close는 `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`가 함께 남아야 한다.

보강 후 같은 요청 스크립트를 다시 실행했다.

- 최종 run: `hardware-validation-run-1777261059811`
- session: `session_000000000018aa19847f462f3c`
- 결과: `failed`
- 실패 코드: `preview-route-owner-gate-failed`
- 핵심 route detail: `binary=C:\Program Files\darktable\bin\darktable-cli.exe`, `source=program-files-bin`, `elapsedMs=3077`, `inputSourceAsset=fast-preview-raster`, `sourceAsset=preset-applied-preview`
- 핵심 수치: `originalVisibleToPresetAppliedVisibleMs=3100ms`
- 해석: 결과 파일은 `preset-applied-preview`였지만 close owner가 host-owned reserve path가 아니라 per-capture `darktable-cli`였으므로 official `Go`가 아니다.

따라서 2026-04-27 기준 Story `1.26`은 계속 `in-progress / No-Go`다. 이번 개선은 속도 개선이 아니라 false `Go` 차단이다.

## 2026-04-27 최신 앱 로그 기반 추가 개선

최근 일반 앱 실행 로그도 같은 원인을 보였다.

- app session: `session_000000000018a9e0f606e69ed0`
- capture 1: helper `file-arrived fastPreviewKind=none`, later `fastPreviewKind=windows-shell-thumbnail`, darktable close `elapsedMs=4592`, official metric `4616ms`
- capture 2: helper `file-arrived fastPreviewKind=none`, later `fastPreviewKind=windows-shell-thumbnail`, darktable close `elapsedMs=2988`, official metric `3017ms`

해석:

- helper는 host-owned `preset-applied-preview` handoff를 만들지 못했다.
- 앱은 first-visible로 `windows-shell-thumbnail`을 보여준 뒤 per-capture `darktable-cli`로 truthful preview를 닫았다.
- 따라서 현재 문제는 단순 darktable tail latency가 아니라 official reserve path 입력 자체가 없다는 점이다.

이 로그를 기준으로 hardware validation runner를 한 번 더 보강했다. 이제 runner는 darktable fallback close까지 기다린 뒤 route owner를 거르는 대신, capture 저장 직후 helper event에서 host-owned `preset-applied-preview` handoff가 있는지 먼저 기록한다. 없으면 `preview-host-owned-reserve-unavailable`로 중단한다.

보강 후 요청 스크립트를 다시 실행했다.

- 최종 run: `hardware-validation-run-1777262125772`
- session: `session_000000000018aa1a7caf7e88b8`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- step evidence: `host-owned-reserve-input` status `failed`
- 핵심 helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`, `satisfiesHostOwnedBoundary=false`
- 해석: Canon helper/camera는 정상 ready였지만, official Go에 필요한 host-owned `preset-applied-preview` handoff가 없었다.

따라서 latest product 판단은 더 좁아졌다. Story `1.26`의 다음 실제 개선은 darktable latency를 더 줄이는 것이 아니라, host-owned path가 full preset look을 보존한 `preset-applied-preview` handoff를 만들 수 있게 하는 것이다.

## 2026-04-27 bounded settle 보강 후 요청 스크립트 재검증

최근 로그의 `file-arrived`와 `fast-preview-ready`가 짧은 시간차로 기록될 수 있으므로, hardware validation runner가 helper event를 단발로 읽고 너무 빨리 실패하지 않도록 보강했다.

개선 내용:

- host-owned reserve input 확인 전에 최대 `1500ms` 동안 helper event를 짧게 poll한다.
- host-owned `preset-applied-preview`가 오면 통과한다.
- `windows-shell-thumbnail` 같은 non-host preview가 먼저 와도 즉시 끝내지 않고, bounded window 끝까지 host-owned handoff를 기다린다.
- step detail에 `waitElapsedMs`, `waitTimedOut`을 남긴다.

자동 테스트:

- `cargo test --manifest-path src-tauri/Cargo.toml --test hardware_validation_runner -- --test-threads=1`
- 결과: `7 passed`
- 추가 검증: file-arrived 뒤 `250ms` 늦게 host-owned `preset-applied-preview`가 도착하는 경우 runner가 실패하지 않고 기다린다.
- 추가 검증: 먼저 `windows-shell-thumbnail`이 와도, 뒤따라오는 host-owned `preset-applied-preview`가 bounded window 안에 있으면 runner가 통과한다.

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777263286397`
- session: `session_000000000018aa1b8aea1ff3f8`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- host-owned reserve step: `failed`
- 핵심 helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`, `satisfiesHostOwnedBoundary=false`, `waitTimedOut=false`
- 핵심 session state: latest preview kind는 `windows-shell-thumbnail`, renderStatus는 `previewWaiting`

해석:

- 이번 실패는 로그 flush race가 아니다.
- runner가 기다릴 수 있는 범위를 보강했지만, helper가 곧바로 non-host-owned terminal preview인 `windows-shell-thumbnail`을 기록했다.
- 따라서 현재 blocker는 그대로 `host-owned preset-applied-preview handoff 없음`이다.

이후 최신 로그를 기준으로 한 번 더 보강했다. 이전 runner는 `windows-shell-thumbnail`을 terminal evidence처럼 보고 즉시 반환할 수 있었기 때문에, 아주 짧게 뒤따르는 host-owned handoff를 놓칠 여지가 있었다.

개선 내용:

- `windows-shell-thumbnail`을 terminal failure로 취급하지 않는다.
- bounded window 안에서는 host-owned `preset-applied-preview`가 늦게 오는지 계속 확인한다.
- host-owned handoff가 끝내 없을 때만 `waitTimedOut=true`로 실패를 남긴다.

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777264963875`
- session: `session_000000000018aa1d117bab2d24`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- host-owned reserve step: `failed`
- 핵심 helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`, `satisfiesHostOwnedBoundary=false`, `waitElapsedMs=1530`, `waitTimedOut=true`
- 핵심 session state: operator summary는 `preview-waiting`, `preview-render-blocked` 상태였다.

해석:

- 이번 실패는 early return 문제가 아니다.
- runner가 `windows-shell-thumbnail` 이후에도 `1500ms` 이상 기다렸지만 host-owned `preset-applied-preview`는 오지 않았다.
- 최신 blocker는 더 명확하게 `host-owned preset-applied-preview handoff 없음`이다.

## 2026-04-27 route evidence 보강 후 요청 스크립트 재검증

최근 개선은 검증기가 helper handoff만 보지 않게 하는 것이다.

개선 내용:

- host-owned reserve input 확인 시 `camera-helper-events.jsonl`뿐 아니라 `timing-events.log`의 `preview-render-ready` route detail도 읽는다.
- `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`가 있으면 helper metadata가 없어도 host-owned route evidence로 인정한다.
- 실패 detail에 `latestPreviewRouteDetail`을 남겨, helper handoff 부재인지 route evidence 부재인지 구분한다.

자동 테스트:

- `cargo test --manifest-path src-tauri\Cargo.toml --lib automation::hardware_validation::tests::host_owned_reserve_input_accepts_host_route_timing_evidence_without_helper_handoff -- --exact`
- 결과: `1 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner -- --test-threads=1`
- 결과: `7 passed`

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777266271721`
- session: `session_000000000018aa1e41fd6f3360`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- host-owned reserve step: `failed`
- 핵심 helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- 핵심 route evidence: `latestPreviewRouteDetail=null`, summary `latestPreviewRoute=none`
- wait evidence: `waitElapsedMs=1521`, `waitTimedOut=true`

해석:

- 이번 실패는 helper event만 보는 검증기 한계가 아니다.
- helper가 준 것은 여전히 `windows-shell-thumbnail`뿐이고, bounded window 안에 host-owned route ready evidence도 없었다.
- 따라서 Story `1.26`의 다음 개선은 검증기 조정이 아니라 host-owned path가 실제 `preset-applied-preview` route evidence를 만들도록 구현하는 것이다.

## 2026-04-27 speculative evidence 보강 후 요청 스크립트 재검증

최근 일반 앱 실행 `session_000000000018a9e0f606e69ed0`는 helper가 `windows-shell-thumbnail`만 제공한 뒤, preset-applied close가 per-capture `darktable-cli` speculative render로 닫히는 패턴을 보였다.

이번 개선 내용:

- hardware validation runner가 host-owned reserve input을 official 3초 window 동안 기다린다.
- helper handoff와 timing route뿐 아니라 speculative preview output/detail/lock 상태도 기록한다.
- 실패 detail에 `latestSpeculativePreviewDetail`, `speculativePreviewOutputReady`, `speculativePreviewLockPresent`를 남긴다.

자동 테스트:

- `cargo test --manifest-path src-tauri\Cargo.toml --lib automation::hardware_validation::tests::host_owned_reserve_input_records_speculative_preview_route_evidence -- --exact`
- 결과: `1 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner -- --test-threads=1`
- 결과: `7 passed`

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777268284508`
- session: `session_000000000018aa2016a0ccd26c`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- host-owned reserve step: `failed`
- 핵심 helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- 핵심 route evidence: `latestPreviewRouteDetail=null`, `latestSpeculativePreviewDetail=null`
- speculative state: `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`
- wait evidence: `waitElapsedMs=3028`, `waitTimedOut=true`

해석:

- 이번 실패는 helper event만 보는 검증기 한계도 아니고, 1.5초 early timeout도 아니다.
- official 3초 window 안에서 host-owned `preset-applied-preview` handoff가 없었고, speculative preview도 output/detail을 만들지 못했다.
- 따라서 다음 개선은 darktable tail 미세 조정이 아니라, host-owned path가 same-capture `preset-applied-preview`를 official window 안에 실제 산출하도록 구현하는 것이다.

## 최종 판단

## 2026-04-27 capture stuck symptom 수정 후 요청 스크립트 재검증

촬영 실패처럼 보였던 최신 증상은 카메라가 파일을 저장하지 못한 문제가 아니었다. `capture-saved` 이후 runner가 host-owned reserve input 부재로 No-Go를 반환했고, 이때 같은 runner 프로세스 안의 preview render가 끝나기 전에 프로세스가 종료되어 session이 `previewWaiting`에 남았다.

이번 개선 내용:

- No-Go 반환 전에 저장된 capture의 preview render를 마무리한다.
- 정리 결과를 `capture-preview-settled-after-no-go` step으로 남긴다.

자동 테스트:

- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner hardware_validation_runner_fails_when_host_owned_reserve_input_is_missing -- --exact --test-threads=1`
- 결과: RED 확인 후 수정, 최종 `1 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner -- --test-threads=1`
- 결과: `7 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --lib automation::hardware_validation::tests::host_owned_reserve_input_records_speculative_preview_route_evidence -- --exact`
- 결과: `1 passed`

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777269000875`
- session: `session_000000000018aa20bd6b9bb82c`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- settled step: `capture-preview-settled-after-no-go` status `passed`
- final session state: `capture-ready`
- latest capture: `previewReady / preset-applied-preview`

해석:

- 촬영 저장 자체는 실패하지 않았다.
- 사용자에게 실패처럼 보인 핵심 증상, 즉 No-Go 이후 capture가 `previewWaiting`에 남는 문제는 수정됐다.
- Story `1.26` 공식 판정은 host-owned reserve input 부재 때문에 계속 `No-Go / in-progress`다.

## 2026-04-27 failure summary route evidence 보강 후 요청 스크립트 재검증

최근 run은 No-Go 판단은 맞았지만, 최종 failure summary가 settle 뒤 생성된 fallback route를 충분히 보여 주지 못했다. 그래서 No-Go settle 이후 host-owned reserve evidence를 다시 읽어 final summary에 반영하도록 보강했다.

자동 테스트:

- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner hardware_validation_runner_fails_when_host_owned_reserve_input_is_missing -- --exact --test-threads=1`
- 결과: RED 확인 후 수정, 최종 `1 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner -- --test-threads=1`
- 결과: `7 passed`

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777270283523`
- session: `session_000000000018aa21e80f662534`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- host-owned reserve step: `failed`, `waitElapsedMs=3023`, `waitTimedOut=true`
- helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- speculative evidence: `speculativePreviewOutputReady=true`, `latestSpeculativePreviewDetail`은 per-capture `darktable-cli`
- final failure summary route: `latestPreviewRoute=binary=C:\Program Files\darktable\bin\darktable-cli.exe;source=program-files-bin;elapsedMs=3011;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied`
- settled step: `capture-preview-settled-after-no-go` status `passed`
- final session state: `capture-ready`

해석:

- 검증 데이터가 이제 fallback route까지 보여 주므로, 원인 판단이 더 명확해졌다.
- 촬영 저장, warm-up, readiness, preview cleanup은 정상이다.
- 하지만 official close owner는 여전히 host-owned reserve path가 아니라 per-capture `darktable-cli` fallback이다.
- Story `1.26`은 계속 `No-Go / in-progress`이며, 다음 개선은 host-owned path가 full preset look을 보존한 same-capture `preset-applied-preview` artifact를 3초 안에 직접 산출하는 것이다.

## 2026-04-27 pre-settle evidence 보존 후 요청 스크립트 재검증

최근 로그에서는 settle 전 evidence와 settle 후 evidence가 서로 다른 시점에 읽히면서, final failure summary가 원인 추적에 필요한 일부 상태를 잃을 수 있었다. 그래서 No-Go settle 전 speculative 상태가 settle 과정에서 사라져도 최종 summary가 그 상태를 보존하도록 보강했다.

자동 테스트:

- `cargo test --manifest-path src-tauri\Cargo.toml no_go_failure_evidence_preserves_pre_settle_speculative_detail_when_settle_cleans_it_up -- --nocapture`
- 결과: RED 확인 후 수정, 최종 `1 passed`

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777271169308`
- session: `session_000000000018aa22b64c44f7a4`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`
- host-owned reserve step: `failed`, `waitElapsedMs=3025`, `waitTimedOut=true`
- helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- speculative evidence: `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`
- final failure summary route: `latestPreviewRoute=binary=C:\Program Files\darktable\bin\darktable-cli.exe;source=program-files-bin;elapsedMs=3136;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied`
- official timing: `originalVisibleToPresetAppliedVisibleMs=3150`
- settled step: `capture-preview-settled-after-no-go` status `passed`
- final session state: `capture-ready`

해석:

- 최신 실행에서도 촬영 저장, warm-up, readiness, No-Go cleanup은 정상이다.
- 공식 host-owned reserve input은 여전히 없다.
- darktable fallback은 비교 증거로는 유용하지만, `3136ms / 3150ms`로 3초 기준도 넘었고 official close owner도 아니다.
- 다음 개선 방향은 runner 조정이 아니라 host-owned path가 same-capture `preset-applied-preview` artifact를 공식 window 안에 직접 만들게 하는 것이다.

## 2026-04-27 15:44 요청 스크립트 재검증

요청한 hardware validation script를 다시 실행했다.

- run: `hardware-validation-run-1777272273229`
- session: `session_000000000018aa23b7531de818`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`, `cameraModel=Canon EOS 700D`
- host-owned reserve step: `failed`, `waitElapsedMs=3030`, `waitTimedOut=true`
- helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- speculative evidence: `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`
- final failure summary route: `latestPreviewRoute=binary=C:\Program Files\darktable\bin\darktable-cli.exe;source=program-files-bin;elapsedMs=3163;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied`
- official timing: `originalVisibleToPresetAppliedVisibleMs=3179`
- settled step: `capture-preview-settled-after-no-go` status `passed`
- final session state: `capture-ready`

해석:

- 같은 문제가 다시 재현됐다.
- 카메라 저장, warm-up, readiness, No-Go cleanup은 정상이다.
- 문제는 여전히 host-owned `preset-applied-preview` reserve input이 없다는 점이다.
- per-capture `darktable-cli` fallback은 비교 증거로 남길 수 있지만, `3163ms / 3179ms`로 3초 기준도 넘었고 official close owner도 아니다.
- 따라서 다음 개선은 검증기나 timeout 조정이 아니라 host-owned path가 full preset look을 보존한 same-capture truthful preview artifact를 공식 window 안에 직접 산출하게 만드는 것이다.

## 2026-04-27 15:54 최근 앱 로그 검토와 요청 스크립트 재검증

최근 일반 앱 실행 로그 `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`를 함께 확인했다.

- app session: `session_000000000018a9e0f606e69ed0`
- app capture 1: helper `file-arrived fastPreviewKind=none`, later `windows-shell-thumbnail`, final `preset-applied-preview`
- app capture 1 timing: `preview-render-ready elapsedMs=4592`, `originalVisibleToPresetAppliedVisibleMs=4616`
- app capture 2: helper `file-arrived fastPreviewKind=none`, later `windows-shell-thumbnail`, final `preset-applied-preview`
- app capture 2 timing: `preview-render-ready elapsedMs=2988`, `originalVisibleToPresetAppliedVisibleMs=3017`

해석:

- 일반 앱에서도 host-owned `preset-applied-preview` handoff는 관찰되지 않았다.
- first-visible은 `windows-shell-thumbnail`으로 빨리 보일 수 있지만, official close는 per-capture `darktable-cli` fallback에 남아 있다.
- current validation runner가 이 경로를 official `Go`로 세지 않는 것은 제품 판정상 맞다.

요청한 hardware validation script를 다시 실행했다.

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- run: `hardware-validation-run-1777272846171`
- session: `session_000000000018aa243cb94f60f0`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`, `cameraModel=Canon EOS 700D`
- host-owned reserve step: `failed`, `waitElapsedMs=3019`, `waitTimedOut=true`
- helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- speculative evidence: `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`
- final failure summary route: `latestPreviewRoute=binary=C:\Program Files\darktable\bin\darktable-cli.exe;source=program-files-bin;elapsedMs=7426;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied`
- official timing: `originalVisibleToPresetAppliedVisibleMs=7505`
- settled step: `capture-preview-settled-after-no-go` status `passed`
- final session state: `capture-ready`

해석:

- warm-up, camera readiness, RAW save, No-Go cleanup은 닫혔다.
- 반복 실패의 핵심은 helper/event race가 아니라 host-owned truthful reserve artifact 부재다.
- darktable fallback은 이번 실행에서 `7426ms / 7505ms`로 크게 늦어졌고 official owner도 아니므로 release evidence가 아니다.
- 다음 개선 방향은 `windows-shell-thumbnail -> darktable fallback`을 더 세게 조정하는 것이 아니라, host-owned path가 same-capture `preset-applied-preview`를 3초 안에 직접 제공하는 것이다.

## 2026-04-27 16:22 요청 스크립트 재검증과 증거 표기 보강

최근 failure summary에서 speculative route가 settle 전/후 표현 차이 때문에 읽기 어려운 부분이 있었다. 그래서 speculative darktable route도 `inputSourceAsset=fast-preview-raster`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied` 형태로 정규화해 기록하도록 보강했다. 단, `binary/source`가 `fast-preview-handoff`가 아니면 official host-owned route로 통과시키지 않는다.

자동 테스트:

- `cargo test --manifest-path src-tauri\Cargo.toml --lib automation::hardware_validation::tests::host_owned_reserve_input_normalizes_speculative_darktable_route_for_failure_readout -- --exact`
- 결과: `1 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --lib automation::hardware_validation::tests::no_go_failure_evidence_preserves_pre_settle_speculative_detail_when_settle_cleans_it_up -- --exact`
- 결과: `1 passed`
- `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner -- --test-threads=1`
- 결과: `7 passed`

요청한 hardware validation script를 다시 실행했다.

- command: `powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"`
- run: `hardware-validation-run-1777274530300`
- session: `session_000000000018aa25c4d6f49e20`
- 결과: `failed`
- 실패 코드: `preview-host-owned-reserve-unavailable`
- capturesPassed: `0/5`
- prompt parse: `Kim4821` -> `Kim 4821`
- warm-up: `preview-runtime-warmed` status `passed`
- camera/helper: `cameraState=ready`, `helperState=healthy`, `detailCode=camera-ready`, `cameraModel=Canon EOS 700D`
- host-owned reserve step: `failed`, `waitElapsedMs=3028`, `waitTimedOut=true`
- helper evidence: `fileArrivedFastPreviewKind=null`, `latestFastPreviewKind=windows-shell-thumbnail`
- speculative evidence at reserve window: `speculativePreviewLockPresent=true`, `speculativePreviewOutputReady=false`
- final failure summary route: `latestPreviewRoute=binary=C:\Program Files\darktable\bin\darktable-cli.exe;source=program-files-bin;elapsedMs=3325;inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview;truthOwner=display-sized-preset-applied`
- settled step: `capture-preview-settled-after-no-go` status `passed`
- final session state: `capture-ready`

해석:

- 이번 run도 camera save, warm-up, readiness, No-Go cleanup은 정상이다.
- helper가 제공한 first-visible은 `windows-shell-thumbnail`뿐이다.
- final fallback이 `preset-applied-preview`를 만들었지만 owner는 `darktable-cli / program-files-bin`이라 Story `1.26` official close가 아니다.
- 다음 제품 개선은 validation timeout 조정이 아니라, host-owned path가 full preset look을 보존한 `preset-applied-preview` artifact를 직접 산출하는 것이다.

현재 Story `1.26`은 "빠른 darktable comparison evidence"는 만들었다.

하지만 official product close는 아직 아니다.

다음 성공은 아래 문장으로 설명될 수 있어야 한다.

> host-owned path가 만든 display-sized preset-applied truthful artifact가, full preset look을 보존하면서, approved hardware에서 3초 안에 same-capture previewReady를 닫았다.

이 문장이 참이 되기 전까지 Story `1.26`은 `No-Go / in-progress`로 읽는다.

## 2026-04-28 13:29 host-owned handoff evidence

개선:

- resident preview 결과를 per-capture fallback이 아니라 host-owned `fast-preview-handoff` route evidence로 기록했다.
- speculative artifact의 준비 시각은 manifest 채택 시각이 아니라 output file ready time을 우선 사용한다.

검증:

- run: `hardware-validation-run-1777350514619`
- session: `session_000000000018aa6ae05150b464`
- 결과: `passed`, capturesPassed `5/5`
- route elapsed band: `2867ms ~ 2946ms`
- official timing band: `2724ms ~ 2844ms`

해석:

- 이전 반복 실패 원인은 host-owned artifact가 없던 것이 아니라, resident route가 fallback처럼 기록되고 artifact ready time이 늦게 채택되던 점이었다.
- 이번 run은 Story `1.26`의 성공 문장인 host-owned same-capture truthful `preset-applied-preview` 3초 내 close를 만족한다.

## 2026-04-28 14:11 validator readout correction and latest No-Go

개선:

- hardware validator가 early reserve precheck timeout만으로 즉시 No-Go를 확정하지 않고, final host-owned route evidence와 official product gate를 함께 읽도록 보강했다.

검증:

- targeted regression: `cargo test --test hardware_validation_runner hardware_validation_runner_accepts_late_host_handoff_when_product_gate_passes`
- runner regression: `cargo test --test hardware_validation_runner -- --test-threads=1`
- requested hardware run: `hardware-validation-run-1777353095373`
- session: `session_000000000018aa6d3932443c98`
- result: `failed`, capturesPassed `1/5`
- latest blocker: capture 2 host-owned `fast-preview-handoff` route was present, but official timing was `4266ms` and route elapsed was `4423ms`.

해석:

- blocker는 missing host-owned handoff evidence가 아니라 host-owned handoff tail latency로 바뀌었다.
- 동일 원인 반복이므로 darktable fallback tuning은 재개하지 않는다.

## 2026-04-28 14:52 darktable-backed handoff readout correction

개선:

- validator가 `fast-preview-handoff` label만 보지 않고 actual engine evidence도 확인한다.
- `engineBinary=darktable-cli` 또는 `engineSource=program-files-bin`이면 official host-owned native artifact로 세지 않는다.

검증:

- targeted regression: `cargo test --manifest-path src-tauri\Cargo.toml --lib preview_truth_gate -- --test-threads=1` -> `4 passed`
- runner regression: `cargo test --manifest-path src-tauri\Cargo.toml --test hardware_validation_runner -- --test-threads=1` -> `8 passed`
- requested hardware run: `hardware-validation-run-1777355500280`
- session: `session_000000000018aa6f6921e793e0`
- result: `failed`, capturesPassed `0/5`
- latest readout: route label was `fast-preview-handoff`, route elapsed was `2962ms`, official timing was `2807ms`, but actual engine was `darktable-cli / program-files-bin`.

해석:

- 동일 원인 반복.
- 최신 blocker는 tail latency가 아니라 official host-owned native truthful artifact 부재로 다시 정정한다.
- 다음 개선은 darktable fallback tuning이 아니라 host-owned path가 full preset look을 보존한 `preset-applied-preview` artifact를 직접 산출하는 것이다.

## 2026-04-28 15:10 host-owned native artifact path false-Go

개선:

- fast-preview raster close가 darktable fallback 대신 host-owned native operation-derived artifact를 만든다.
- route evidence는 `fast-preview-handoff / host-owned-native`로 남기고, darktable engine evidence를 official close로 승격하지 않는다.

검증:

- targeted regression: `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_render_uses_host_owned_native_artifact -- --nocapture` -> `1 passed`
- route-owner regression: `cargo test --manifest-path src-tauri/Cargo.toml preview_truth_gate -- --nocapture` -> `4 passed`
- requested hardware run: `hardware-validation-run-1777356593528`
- session: `session_000000000018aa7067ac8a15dc`
- result: `passed`, capturesPassed `5/5`
- latest readout: helper first-visible was still `windows-shell-thumbnail`; official close route was `fast-preview-handoff / host-owned-native`, route elapsed `1014ms ~ 1052ms`, official timing `1014ms ~ 1054ms`.

해석:

- 사용자 검토로 false-Go를 확인했다.
- 1초대 timing은 `fast-preview-raster`에 operation-derived native 변환을 얹었기 때문에 나온 값이다.
- 원본 RAW/full preset 적용 proof가 아니므로 official `Go`로 세면 안 된다.

## 2026-04-28 15:18 false-Go correction

개선:

- validator가 `profile=operation-derived` 또는 `inputSourceAsset=fast-preview-raster`를 official original/full-preset truth로 세지 않게 막았다.

검증:

- route-owner regression: `cargo test --manifest-path src-tauri/Cargo.toml preview_truth_gate -- --nocapture` -> `5 passed`
- requested hardware run: `hardware-validation-run-1777357070026`
- session: `session_000000000018aa70d69e0d5ba8`
- result: `failed`, capturesPassed `0/5`
- latest readout: route elapsed `1185ms`, but route detail contains `inputSourceAsset=fast-preview-raster` and `profile=operation-derived`.

해석:

- 최신 blocker는 original/full-preset truthful artifact proof 부재다.
- fast raster native approximation은 comparison evidence only.

## 2026-04-28 15:39 operation-derived false-ready block

개선:

- 앱이 `profile=operation-derived` host-owned speculative output을 `previewReady` truth로 승격하지 않게 막았다.
- 해당 output은 comparison evidence로만 남고, saved capture close는 원본 기반 truthful 경로로 넘어간다.

검증:

- targeted regression: `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_does_not_promote_operation_derived_speculative_preview -- --nocapture` -> `1 passed`
- related regressions: finished speculative close / healthy speculative close tests -> `passed`
- requested hardware run: `hardware-validation-run-1777358341911`
- session: `session_000000000018aa71fec0460804`
- result: `failed`, capturesPassed `0/5`
- latest readout: speculative `fast-preview-handoff / host-owned-native` elapsed `1013ms`, but still `inputSourceAsset=fast-preview-raster` + `profile=operation-derived`; settled close was `raw-original / darktable-cli`, official timing `6300ms`.

해석:

- 동일 원인 반복.
- false-ready 승격은 막혔지만, Story `1.26`의 남은 blocker는 host-owned original/full-preset truthful artifact 부재다.

## 2026-04-28 15:52 host-owned truth contract

개선:

- Story `1.26` 문서에 다음 구현 방향을 명시했다.
- code contract도 맞췄다. host-owned `fast-preview-handoff`는 `inputSourceAsset=raw-original`과 `truthProfile=original-full-preset`을 함께 가진 경우에만 `previewReady` truth를 소유한다.

해석:

- `fast-preview-raster`, `profile=operation-derived`, incomplete host-owned handoff는 계속 comparison evidence only다.
- 다음 구현은 preset operation 지원 범위 / parity 조건을 정하고, 통과한 artifact에만 `truthProfile=original-full-preset`을 부여하는 것이다.

## 2026-04-28 16:30 helper-kind-only false truth guard

개선:

- validator가 helper `preset-applied-preview` kind만으로 official reserve input을 인정하지 않게 막았다.
- official reserve input은 계속 `inputSourceAsset=raw-original`과 `truthProfile=original-full-preset` route evidence가 필요하다.

검증:

- targeted regression: `cargo test --manifest-path src-tauri/Cargo.toml automation::hardware_validation::tests::host_owned_reserve_input` -> `5 passed`
- requested hardware run: `hardware-validation-run-1777361395745`
- session: `session_000000000018aa74c5c6fa9c54`
- result: `failed`, capturesPassed `0/5`
- latest readout: speculative route `1004ms`, settled raw-original darktable route `3224ms`, official timing `6241ms`.

해석:

- 동일 원인 반복.
- 검증기는 더 엄격해졌지만, 남은 blocker는 host-owned original/full-preset truthful artifact 부재다.

## 2026-04-28 16:47 fast-raster truth blocker evidence

개선:

- host-owned native comparison route가 `truthBlocker=fast-preview-raster-input`와 `requiredInputSourceAsset=raw-original`을 route evidence에 남기게 했다.
- validation summary도 해당 blocker를 보존한다.

검증:

- targeted regression: `cargo test --manifest-path src-tauri/Cargo.toml fast_preview_raster_render_uses_host_owned_native_artifact` -> `1 passed`
- reserve input regression: `cargo test --manifest-path src-tauri/Cargo.toml automation::hardware_validation::tests::host_owned_reserve_input` -> `5 passed`
- requested hardware run: `hardware-validation-run-1777362425289`
- session: `session_000000000018aa75b57c9c8f28`
- result: `failed`, capturesPassed `0/5`
- latest readout: speculative route `1019ms`, `truthBlocker=fast-preview-raster-input`; settled raw-original darktable route `3316ms`; official timing `6333ms`.

해석:

- 동일 원인 반복.
- latest evidence now names the generation-path gap directly: the fast native artifact is still raster-derived, and official success needs raw-original/full-preset host-owned output.

## 2026-04-28 17:39 RAW-source preference and eligibility gate

개선:

- helper fast-preview generation path now prefers Canon SDK RAW preview before Windows shell thumbnail.
- host-owned native route evidence now separates `raw-sdk-preview` source handling from `fast-preview-raster` comparison output.
- `truthProfile=original-full-preset` is only allowed when the preset operation set is supported; unsupported presets stay comparison-only with an explicit truth blocker.

검증:

- targeted regression: `cargo test raw_sdk_preview_render_requires_supported_preset_before_claiming_full_preset_truth --manifest-path src-tauri/Cargo.toml` -> `1 passed`
- capture readiness regression: `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness -- --test-threads=1` -> `107 passed`
- C# helper targeted test was added, but local execution is blocked before test discovery because Canon SDK source/vendor files are missing.
- requested hardware run: `hardware-validation-run-1777365515848`
- session: `session_000000000018aa788510413540`
- result: `failed`, capturesPassed `0/5`
- latest readout: running app mode was `appLaunchMode=skip`; helper still supplied `windows-shell-thumbnail`; speculative route stayed `fast-preview-raster / operation-derived-comparison` with `truthBlocker=fast-preview-raster-input`; settled raw-original darktable route elapsed `3385ms`; official timing `6432ms`.

해석:

- 동일 원인 반복.
- 이번 hardware run did not exercise a RAW SDK helper handoff because the already-running app/helper still emitted `windows-shell-thumbnail`.
- next smallest product path is to deploy/restart the updated helper path, verify `raw-sdk-preview` appears, then close or reject the remaining preset eligibility/parity gap for `look2`.

## 2026-04-29 10:21 RAW SDK helper path retry

개선:

- helper RAW SDK preview path now tries Canon SDK RGB/JPEG extraction before Windows shell fallback.
- RAW SDK preview generation is forced onto the Canon SDK STA thread before Shell fallback is allowed.
- helper timeout/reconnect stability regressions were restored after the full helper test suite caught them.

검증:

- targeted C# helper test with `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745203316536_Kykl2PJDH9` -> `1 passed`
- helper test suite with `BOOTHY_CANON_SDK_ROOT=C:\Code\cannon_sdk\1745203316536_Kykl2PJDH9` -> `22 passed`
- requested hardware run: `hardware-validation-run-1777425683850`
- session: `session_000000000018aaaf3e04b8d3a0`
- result: `failed`, capturesPassed `0/5`
- latest readout: helper still supplied `windows-shell-thumbnail`; speculative route stayed `fast-preview-raster / operation-derived-comparison` with `truthBlocker=fast-preview-raster-input`; settled raw-original darktable route elapsed `3683ms`; official timing `6725ms`.

해석:

- 동일 원인 반복.
- review follow-up improved the intended helper generation path, but approved hardware still did not produce `raw-sdk-preview`.
- next smallest product path is to either prove Canon SDK RAW extraction cannot produce this artifact on EOS 700D, or bypass it with a native RAW decode + preset application path that creates `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, and `truthProfile=original-full-preset`.

## 2026-04-29 10:46 raw-original handoff route attempt

개선:

- app preview completion now attempts a host-owned `raw-original -> preset-applied-preview` handoff before darktable fallback when the original input is directly decodable by the native renderer.
- this is a product-path change, not a validator-only change: successful handoff evidence must include `binary=fast-preview-handoff`, `engineSource=host-owned-native`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, and `truthProfile=original-full-preset`.

검증:

- new RED/GREEN regression: `cargo test --test capture_readiness complete_preview_render_builds_host_owned_original_full_preset_handoff_from_raw_before_darktable -- --nocapture` -> `1 passed`
- related grouped regression: `cargo test --test capture_readiness late_ -- --test-threads=1 --nocapture` -> `5 passed`
- requested hardware run: `hardware-validation-run-1777427158882`
- session: `session_000000000018aab09573b633bc`
- result: `failed`, capturesPassed `0/5`
- latest readout: helper still supplied `windows-shell-thumbnail`; speculative route stayed `fast-preview-raster / operation-derived-comparison` with `truthBlocker=fast-preview-raster-input`; settled raw-original darktable route elapsed `3459ms`; official timing `6503ms`.

해석:

- 동일 원인 반복.
- code 기준으로 current native renderer can close the official handoff only when the original source is directly decodable by the app renderer; approved hardware `.CR2` still reaches darktable fallback.
- next smallest product path is native `.CR2` decode/preset application, or a Canon SDK RAW extraction result that reliably supplies the app with a raw-original-derived source eligible for `truthProfile=original-full-preset`.

## 2026-04-29 11:19 native CR2 handoff and parity blocker

개선:

- RAW persist now starts the host-owned `raw-original -> preset-applied-preview` route before the fast-raster comparison route.
- native preview rendering now accepts approved hardware `.CR2` sources and emits raw-original handoff evidence.

검증:

- targeted capture regressions passed:
  - `cargo test --test capture_readiness capture_persist_starts_host_owned_raw_original_handoff_before_preview_completion -- --nocapture`
  - `cargo test --test capture_readiness complete_preview_render_builds_host_owned_original_full_preset_handoff_from_raw_before_darktable -- --nocapture`
- native decode fixture passed with latest approved-hardware CR2.
- requested hardware run: `hardware-validation-run-1777429142787`
- session: `session_000000000018aab2635d598144`
- result: `failed`, capturesPassed `0/5`
- latest readout: host-owned handoff now has `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `engineSource=host-owned-native`, elapsed `1071ms`; it is blocked by `truthProfile=unsupported-preset-comparison` for `look2`.

해석:

- blocker moved.
- The previous host-owned original artifact absence is improved.
- The remaining product blocker is full-preset parity for `look2`; unsupported operations include raw pipeline/correction modules such as `lens`, `denoiseprofile`, `demosaic`, `rawprepare`, and color pipeline modules.
- Do not promote this artifact to official truth until it can honestly emit `truthProfile=original-full-preset`.

## 2026-04-29 11:38 look2 eligibility fix and approved-hardware Go

개선:

- `look2`의 native RAW pipeline/correction operations를 unsupported preset-look operation으로 잘못 분류하지 않도록 eligibility를 정리했다.
- unsupported guard는 유지해서 실제 미지원 look operation은 계속 `unsupported-preset-comparison`으로 남는다.

검증:

- RED/GREEN regression: `raw_original_truth_profile_accepts_native_raw_pipeline_operations_for_look2`
- unsupported guard regression: `raw_sdk_preview_render_requires_supported_preset_before_claiming_full_preset_truth`
- capture path regressions passed:
  - `capture_persist_starts_host_owned_raw_original_handoff_before_preview_completion`
  - `complete_preview_render_builds_host_owned_original_full_preset_handoff_from_raw_before_darktable`
- requested hardware run: `hardware-validation-run-1777430298314`
- session: `session_000000000018aab370683ef338`
- result: `passed`, capturesPassed `5/5`
- route evidence: all captures emitted `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, `truthProfile=original-full-preset`, `engineSource=host-owned-native`.
- official timings: `357ms`, `177ms`, `174ms`, `225ms`, `170ms`.

해석:

- Retracted by the 11:51 investigation. The pass was a false Go because the native RAW approximation was over-white and not a verified full-preset renderer.

## 2026-04-29 11:51 false Go correction

원인:

- The native RAW renderer is a fast approximation, not a verified full-preset renderer.
- The 11:38 change looked only at operation eligibility and mislabeled that approximation as `truthProfile=original-full-preset`.
- Hardware output confirmed the product issue: false-Go native output average luma `246.62`, white pixels `87.99%`, while the latest darktable fallback output average luma was `30.9`, white pixels `0%`.
- The unrealistically low timings were caused by that same false promotion.

수정:

- Native RAW approximation now emits `truthProfile=host-owned-native-preview-comparison` and `truthBlocker=full-preset-parity-unverified`.
- It no longer owns `previewReady` or Story `1.26` official Go.

검증:

- RED/GREEN regression: `raw_original_native_preview_does_not_claim_full_preset_without_parity_engine`
- capture regressions:
  - `complete_preview_render_does_not_promote_unverified_native_raw_handoff`
  - `capture_persist_starts_comparison_only_host_owned_raw_original_handoff_before_preview_completion`
- requested hardware run: `hardware-validation-run-1777431052504`
- session: `session_000000000018aab420015bb524`
- result: `failed`, capturesPassed `0/5`
- latest readout: `truthProfile=host-owned-native-preview-comparison`, `truthBlocker=full-preset-parity-unverified`; darktable fallback route elapsed `3336ms`.

해석:

- false Go is fixed.
- Story `1.26` is back to `No-Go` until there is a real host-owned full-preset renderer or a visual/parity qualification path that cannot pass over-white output.

## 2026-04-29 11:58 native RAW white-balance clipping fix

원인:

- Native RAW conversion multiplied camera white-balance coefficients directly into display values.
- Midtone samples could clip to display white before preset/profile application.

수정:

- White-balance scaling now normalizes against the strongest white-balance coefficient to reduce midtone clipping.
- The output remains comparison-only; this does not restore official truth ownership.

검증:

- RED/GREEN regression: `rawloader_white_balance_does_not_clip_midtones_to_white`
- RAW conversion regression: `rawloader_bayer_image_converts_to_rgb_preview_pixels`
- capture regressions:
  - `complete_preview_render_does_not_promote_unverified_native_raw_handoff`
  - `capture_persist_starts_comparison_only_host_owned_raw_original_handoff_before_preview_completion`
- requested hardware run: `hardware-validation-run-1777431500206`
- session: `session_000000000018aab4883e7811d8`
- result: `failed`, capturesPassed `0/5`
- latest readout: `truthProfile=host-owned-native-preview-comparison`, `truthBlocker=full-preset-parity-unverified`; final canonical preview average luma `30.88`, white pixels `0%`.

해석:

- The white-photo symptom is contained for the final displayed preview.
- Story `1.26` remains No-Go because the native RAW route is still not a verified full-preset renderer.

## 2026-04-29 direction decision: choose option 2

확인:

- `docs/preview-architecture-history-and-agent-guide.md` already had the old `resident first-visible` history and treated `darktable-compatible path` as parity/fallback/final reference.
- `docs/runbooks/preview-latency-next-steps-checklist-20260422.md` already recorded repeated darktable tail and host-owned reserve artifact failures.
- Neither document explicitly recorded the latest decision to stop pursuing partial native approximation and choose a resident/long-lived actual preset engine path.

결정:

- Choose option 2.
- The next product direction is a resident/long-lived darktable-compatible full-preset engine path.
- This is not more per-capture darktable fallback tuning.
- The goal is to keep the real preset result owner hot enough for the official window while preserving full preset fidelity.
- Native RAW approximation remains comparison-only unless it can be backed by real full-preset parity proof.

다음 실제 개선 방향:

- Build or restore a long-lived preset renderer owner that can output `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, and `truthProfile=original-full-preset` honestly.
- Reject fast-preview-raster input, operation-derived profile, darktable-backed fallback, and parity-less native output as official truth.
- Re-run hardware validation only after the generation/promotion path can prove actual full-preset ownership.

## 2026-04-29 research briefing conclusion

판단:

- Option 2 is the correct product direction, but it is not yet a proven implementation.
- The next implementation should start as a bounded spike for a resident/long-lived darktable-compatible full-preset engine.
- The spike must prove three things together: full preset fidelity, same-capture close ownership, and approved hardware timing inside the official gate.

가드레일:

- Do not treat a fast native approximation as product truth without parity proof.
- Do not count per-capture darktable fallback as the new reserve path, even if one run is fast.
- Do not loosen the customer promise: `previewReady` still belongs only to the actual preset-applied close asset.

권장 구현 순서:

1. Try a long-lived preset renderer worker that owns full-preset output.
2. If needed, evaluate a darktable-compatible in-process/library-style owner as a spike, not as an immediate release path.
3. Keep current `darktable-cli` output as reference/fallback/final correctness evidence until the resident engine proves parity.

## 2026-04-29 12:45 option 2 implementation result

확인:

- 반복 No-Go 원인은 full-preset truthful artifact 부재였다.
- RAW original speculative handoff가 native approximation이 아니라 resident darktable-compatible full-preset route를 사용하도록 바꿨다.
- 첫 patched hardware run `hardware-validation-run-1777434033121`은 route ownership은 통과했지만 Windows `//?/` path prefix 때문에 첫 resident 시도가 실패해 official timing `6133ms`로 실패했다.
- path prefix를 darktable CLI 인자에서 제거한 뒤 `hardware-validation-run-1777434275752`가 `5/5` 통과했다.

최신 검증:

- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777434275752\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aab70e79e5baa8\`
- route: `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `engineMode=resident-full-preset`, `engineAdapter=darktable-compatible`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthProfile=original-full-preset`
- official timing: `2316ms ~ 2338ms`

해석:

- This evidence was later retracted by code review because the route could still be per-capture `darktable-cli` output self-labeled as resident full-preset.
- Partial native RAW approximation remains comparison-only and is not part of the product truth path.

## 2026-04-29 14:38 false Go retraction

판단:

- `hardware-validation-run-1777434275752` is not official Story `1.26` Go evidence.
- Runtime truth now requires more than route labels: metadata-only `preset-applied-preview`, filename-derived kind, and per-capture `darktable-cli` output cannot close `previewReady`.
- Hardware validation now rejects self-labeled resident routes that still expose `darktable-cli` / `program-files-bin` evidence.

현재 제품 상태:

- Story `1.26` is back to `in-progress / No-Go`.
- Option 2 remains the target, but still needs a real resident/long-lived full-preset owner and fresh approved-hardware validation.

## 2026-04-29 14:59 product decision and validation result

판단:

- The product boundary now accepts an explicit per-capture full-preset route when the route evidence is honest.
- The accepted route must say `engineMode=per-capture-cli`; it must not self-label as resident.
- Metadata-only `preset-applied-preview` and filename-derived truth remain blocked.

최신 검증:

- run: `C:\Users\KimYS\Pictures\dabi_shoot\hardware-validation-runs\hardware-validation-run-1777442288984\`
- session: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018aabe5833c11d8c\`
- result: `passed / 5/5`
- official timing: `2387ms ~ 2480ms`
- route: `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `engineMode=per-capture-cli`, `engineAdapter=darktable-compatible`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthProfile=original-full-preset`

현재 제품 상태:

- Story `1.26` is `Go` in the hardware ledger.

## 2026-04-29 traceability note for future changes

현재 정답:

- Story `1.26`의 현재 제품 목표는 option 2다.
- 즉, 원본 RAW를 resident/long-lived darktable-compatible full-preset owner가 처리해 `preset-applied-preview`를 만드는 길이다.
- 이 결과만 official truth로 본다.

성공으로 볼 수 있는 조건:

- same-capture artifact
- `inputSourceAsset=raw-original`
- `sourceAsset=preset-applied-preview`
- `truthOwner=display-sized-preset-applied`
- `truthProfile=original-full-preset`
- `engineMode=resident-full-preset`
- `engineAdapter=darktable-compatible`
- approved hardware run에서 official `originalVisibleToPresetAppliedVisibleMs <= 3000ms`

성공으로 보면 안 되는 조건:

- partial native RAW approximation
- fast preview raster에서 만든 결과
- operation-derived profile
- per-capture darktable fallback
- full-preset parity proof가 없는 host-owned output

변경 추적 기준:

- 이 경로를 바꾸는 작업은 위 route fields가 계속 남는지 먼저 확인한다.
- timing만 나빠진 경우에는 fallback 튜닝보다 resident owner reuse, path handling, warm state를 먼저 본다.
- route fields가 사라진 경우에는 official truth가 깨진 것으로 보고 resident full-preset generation/promotion path를 복구한다.
- native approximation을 개선하더라도 full-preset parity proof가 없으면 comparison-only로 둔다.
