# 현재 세션 사진 Troubleshooting History

## 목적

이 문서는 `현재 세션 사진` 섹션에 촬영 직후 이미지가 보이지 않는 문제를
다음 세션과 다른 에이전트가 빠르게 이어받을 수 있게 정리한 이력이다.

이 문서의 목표는 세 가지다.

1. 이미 배제한 가설을 다시 처음부터 반복하지 않게 한다.
2. 실제로 관찰된 세션 파일과 preview 자산 상태를 증거 중심으로 남긴다.
3. 이미 시도한 수정, 검증 결과, 아직 남아 있는 불확실성을 분리해 전달한다.

## 이 문서가 다루는 범위

포함:

- `현재 세션 사진` 레일이 비어 보이거나 깨진 이미지처럼 보이는 문제
- 촬영 직후 session preview 자산이 `.svg` placeholder로 남는 문제
- host/helper/frontend 중 어느 경계에서 preview가 실제 사진으로 바뀌지 않는지 추적한 내용

제외:

- 일반적인 카메라 연결 상태 `Preparing/Ready` 이력
- `사진찍기` 요청 자체의 requestId correlation, timeout, `Phone Required` 분석 전반

관련 일반 이력은 아래 문서를 함께 본다.

- [camera-helper-troubleshooting-history.md](/C:/Code/Project/Boothy/history/camera-helper-troubleshooting-history.md)
- [camera-capture-validation-history.md](/C:/Code/Project/Boothy/history/camera-capture-validation-history.md)

## 현재 결론

이번 이슈에서 가장 중요한 결론은 아래 두 가지다.

1. 이번 재현 세션에서는 **프리셋 누락이 원인이 아니었다.**
2. 실제 세션 preview 폴더에는 **촬영본 JPG가 아니라 `.svg` placeholder만 존재했다.**
3. frontend가 절대경로 preview 자산을 읽으려면 **Tauri `asset` protocol 자체가 활성화돼 있어야 하는데, 이번 회차 전까지 그 설정이 빠져 있었다.**
4. 두 번째 촬영 뒤 첫 화면으로 튕긴 최신 이슈의 직접 원인은 **촬영 실패가 아니라 readiness polling이 원자적 manifest 저장 중간 gap을 `session-not-found`로 오진한 레이스**였다.

즉 문제의 핵심은 "프리셋이 없어서 preview가 안 보인다"가 아니라,
"RAW 촬영 뒤 실제 preview raster가 생기지 않거나, 생겨도 화면/manifest가 그 자산을 쓰지 못한다" 쪽이었다.

## 최신 회차 추가 결론: 두 번째 촬영 뒤 첫 화면으로 튕긴 원인

사용자 최신 피드백:

- 첫 번째 `사진 찍기` 뒤에는 현재 세션 사진이 정상으로 보였다.
- 두 번째 `사진 찍기` 뒤에는 앱이 첫 화면으로 튕긴 것처럼 보였다.

이번 회차에서 확인한 실제 사실:

- 두 번째 촬영본 자체는 **같은 세션에 정상 저장되었다.**
- 문제는 촬영 직후 프런트가 **현재 세션을 잃어버렸다고 오판**한 것이다.
- 그 결과 사용자가 체감하기에는 "두 번째 촬영 후 첫 화면으로 튕김"처럼 보였다.

대표 최신 재현 세션:

- 정상으로 두 장이 저장된 세션:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a140425d38239c`
- 튕긴 뒤 새로 시작된 세션:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a14047f12470b4`

확인 결과:

- `session_000000000018a140425d38239c\session.json`에는 capture 2장이 모두 기록되어 있었다.
- second capture preview도 `.jpg`로 정상 생성돼 있었다.
- 그런데 같은 시각 operator audit에는 바로 뒤이어 `session-started`가 추가돼 새 세션 `session_000000000018a14047f12470b4`가 생겼다.

의미:

- 사용자가 본 문제는 "두 번째 촬영본이 저장되지 않음"이 아니라
  "두 번째 촬영은 저장됐지만, UI가 현재 세션을 잃어버렸다고 판단해 세션 시작 화면으로 리셋됨"이었다.

## 이번 회차 로그에서 잡은 직접 증거

로그 파일:

- `C:\Users\KimYS\AppData\Local\com.tauri.dev\logs\Boothy.log`

관찰 순서:

1. `07:46:41`에 `gateway-request-capture session=session_000000000018a140425d38239c`
2. 거의 동시에 host에서 `capture_readiness_failed ... code=session-not-found`
3. 이어서 frontend debug log에
   - `apply-subscribed-readiness ... reason_code=session-missing`
   - `readiness-error ... 이전 세션의 준비 상태 응답이 늦게 도착했어요`
4. 직후 operator audit에 새 세션 `session_000000000018a14047f12470b4`가 생김

핵심 해석:

- `request_capture`와 거의 동시에 돌아가던 readiness polling이
  manifest atomic write 중간 순간을 읽었다.
- 그 순간 host가 `session-not-found`를 돌려주면서
  subscription 경로가 `primaryAction = start-session` 성격의 readiness로 번역했다.
- 프런트는 이것을 실제 세션 소실로 받아들여 `resetToSessionStart()` 쪽으로 흘렀다.

## 이번 회차에서 확인한 코드 레벨 원인

문제 파일:

- [normalized_state.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/normalized_state.rs)
- [session_repository.rs](/C:/Code/Project/Boothy/src-tauri/src/session/session_repository.rs)

원인:

- `session_repository::write_session_manifest(...)`는 `session.json -> session.json.bak -> session.json.tmp -> session.json`
  순서의 atomic swap을 사용한다.
- 그런데 `normalized_state.rs`에는 별도의 로컬 `read_session_manifest(...)`가 있었고,
  이 구현은 `.json.bak` fallback을 보지 않았다.
- 따라서 readiness polling이 swap gap에 걸리면 실제 세션이 살아 있어도
  `session-not-found`를 반환할 수 있었다.

즉 같은 프로젝트 안에서도:

- session/preset 경로는 backup recovery를 지원했고
- capture readiness 경로는 지원하지 않아
- 두 번째 촬영 직후에만 레이스성 reset이 발생할 수 있었다.

## 이번 회차 수정 내용

적용한 수정:

- `normalized_state.rs`의 로컬 manifest reader를 제거했다.
- capture readiness 경로도 `session_repository::read_session_manifest(...)`를 사용하도록 바꿨다.
- 그래서 atomic swap gap에서 `session.json.bak`가 남아 있으면 backup을 복구해 계속 읽는다.

추가한 회귀 테스트:

- `readiness_recovers_from_a_manifest_backup_left_during_an_atomic_swap_gap`

검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_recovers_from_a_manifest_backup_left_during_an_atomic_swap_gap -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test session_manifest selecting_a_preset_recovers_from_a_manifest_backup_left_by_an_interrupted_write -- --exact`

의미:

- 앞으로는 촬영 직후 readiness polling이 manifest atomic swap 중간 상태를 읽더라도
  같은 세션을 `session-not-found`로 오진해 첫 화면으로 튕길 가능성이 크게 줄어든다.

## 이번 회차에서 확정한 추가 원인

이전까지는 `.svg` placeholder와 preview 생성 경계에 초점을 맞췄지만,
이번 회차에서는 **현재 세션 사진이 실제로 보이지 않는 직접 원인 중 하나가
Tauri `asset` protocol 비활성화**였음을 확인했다.

확인 내용:

- frontend는 `resolvePresetPreviewSrc(...)`에서 절대경로 자산을 `convertFileSrc(...)`로 바꿔 사용한다.
- 하지만 `src-tauri/Cargo.toml`의 `tauri` crate feature에 `protocol-asset`이 없었다.
- `src-tauri/tauri.conf.json`의 `app.security.assetProtocol.enable`도 꺼져 있었고 scope도 비어 있었다.

의미:

- 이 상태에서는 현재 세션 preview처럼 `C:\Users\...\Pictures\dabi_shoot\...` 절대경로 자산을 WebView가 안정적으로 읽을 수 없다.
- 따라서 `.svg` 인라인 보강이 있어도, 원본 asset protocol이 비활성화돼 있으면 `fetch`와 `<img>` 둘 다 실패할 수 있다.

이번에 적용한 보강:

- `tauri = { version = "2.10.3", features = ["protocol-asset"] }`
- `tauri.conf.json`에 아래 scope 추가
  - `$PICTURE/dabi_shoot/**`
  - `$APPLOCALDATA/dabi_shoot/**`

이 보강은 "실제 JPG preview 생성 실패" 문제를 대신 해결하는 것은 아니지만,
적어도 현재 세션 rail이 session preview 자산을 읽는 기본 경계는 정상화한다.

## 대표 재현 세션

사용자가 직접 지목한 세션:

- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc`

핵심 확인 경로:

- session 파일: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc\session.json`
- preview 폴더: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc\renders\previews`

실제 관찰:

- 이 세션은 `session_manifest.json`이 아니라 `session.json`을 사용한다.
- preview 폴더에는 아래 두 파일만 있었다.
  - `capture_20260329070018096_83e398b0eb.svg`
  - `capture_20260329070025071_19704dba0d.svg`
- 같은 이름의 `.jpg`, `.jpeg`, `.png`, `.webp`, `.bmp`, `.gif` 파일은 없었다.

## 먼저 배제된 가설

### 1. 프리셋이 없어서 안 보이는 것 아닌가

배제됨.

실제 `session.json` 확인 결과:

- `activePreset.presetId = "preset_daylight"`
- `activePreset.publishedVersion = "2026.03.27"`
- `activePresetDisplayName = "Daylight"`

각 capture record에도 아래 값이 있었다.

- `activePresetId = "preset_daylight"`
- `activePresetVersion = "2026.03.27"`
- `activePresetDisplayName = "Daylight"`

즉 이 재현 세션은 프리셋 누락 상태가 아니다.

### 2. manifest가 preview를 아직 준비 중이라서 안 보이는 것 아닌가

배제됨.

같은 세션 `session.json` 기준:

- 각 capture의 `renderStatus`는 `previewReady`
- `preview.readyAtMs`도 채워져 있었다.
- 단지 `preview.assetPath`가 실제 JPG가 아니라 `.svg`를 가리키고 있었다.

즉 "preview가 아직 준비 중이라 안 보인다"보다는
"준비 완료로 표기된 preview가 placeholder 자산이다"가 더 정확하다.

## 실제 증거

### 1. preview asset path는 `.svg` placeholder였다

대표 capture:

- `capture_20260329070018096_83e398b0eb`

`session.json` 안 preview 경로:

- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc\renders\previews\capture_20260329070018096_83e398b0eb.svg`

### 2. `.svg` 파일 내용은 실제 촬영본이 아니라 fallback 안내였다

실제 파일에는 아래 의미의 SVG 마크업이 들어 있었다.

- 큰 카드 배경
- `Preview unavailable`
- `capture: capture_20260329070018096_83e398b0eb`

즉 preview 자산 자체가 "실제 사진이 없는 상태에서 만든 fallback 이미지"였다.

### 3. 사용자가 본 화면 증상과 파일 증거가 일치했다

사용자 스크린샷에서는:

- 카드 텍스트는 보이는데 이미지 자리는 비어 있거나 깨져 보였다.
- alt text가 겹쳐 보이는 깨진 이미지 느낌이 있었다.

이 현상은 아래 두 사실과 맞아떨어진다.

1. manifest는 `previewReady`로 간주한다.
2. 실제 자산은 `.svg` placeholder인데 WebView가 그 파일을 정상 표시하지 못한다.

## 코드 조사에서 확인한 사실

### 1. host는 RAW 계열에서 raster preview가 없으면 `.svg` fallback을 만든다

파일:

- [ingest_pipeline.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/ingest_pipeline.rs)

핵심 흐름:

- `complete_preview_render_in_dir(...)`
- `materialize_preview_asset(...)`

확인 결과:

- RAW 확장자(`jpg/png/...`가 아닌 경우)는 원본 복사가 아니라 sidecar preview 탐색으로 들어간다.
- sidecar preview가 없으면 host는 `build_preview_fallback_svg_bytes(...)`로 `.svg`를 만든다.
- 따라서 helper가 JPG preview를 제때 만들어 주지 못하면 host는 매우 쉽게 `.svg` placeholder로 확정된다.

### 2. helper는 preview 생성 시도를 하지만 모두 best-effort였다

파일:

- [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs)
- [WindowsShellThumbnail.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/WindowsShellThumbnail.cs)

추가한 경로:

1. `EdsDownloadThumbnail(...)`
2. `EdsSaveImage(...)`로 RAW에서 JPG 렌더
3. Windows shell thumbnail extraction
4. helper loop에서 기존 RAW에 대한 preview backfill

하지만 실제 재현 세션에서는:

- helper가 살아 있고 `ready/healthy`일 때도
- 해당 session의 `renders/previews` 아래에 JPG가 생기지 않았다.

즉 "코드상 fallback 시도 추가"와 "실제 장비에서 raster preview 생성 성공"은 아직 분리해서 봐야 한다.

### 3. frontend는 `.svg` local asset을 직접 표시하지 못하는 경로가 있었다

파일:

- [LatestPhotoRail.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/LatestPhotoRail.tsx)
- [SessionPreviewImage.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/SessionPreviewImage.tsx)

확인 결과:

- 원래 레일은 preview 경로를 바로 `<img src=...>`로 사용했다.
- local `.svg` placeholder는 Tauri/WebView에서 그대로 깨져 보일 수 있었다.
- 그래서 `.svg` 파일은 직접 fetch해서 `data:image/svg+xml` URL로 인라인하는 wrapper를 추가했다.

의미:

- 이 수정은 "실제 사진이 없음"을 해결하는 게 아니라,
  적어도 `.svg` placeholder가 있을 때 화면이 완전히 비어 보이지 않게 하는 보강이다.

### 4. host는 나중에 raster preview가 생기면 manifest를 JPG로 복구하도록 보강했다

파일:

- [normalized_state.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/normalized_state.rs)

추가한 내용:

- `sync_better_preview_assets_in_manifest(...)`
- `find_better_session_preview_asset(...)`

동작:

- manifest에 `.svg`가 기록돼 있어도
- 같은 capture id의 `.jpg/.jpeg/.png/.webp/.gif/.bmp`가 나중에 생기면
- readiness 계산 시 manifest preview path를 그 raster 자산으로 바꾼다.

의미:

- 이미 placeholder로 굳은 세션도, 나중에 실제 preview sidecar가 생기면 host가 회복할 수 있다.

### 5. host는 sidecar preview를 조금 기다렸다가 쓰도록 보강했다

파일:

- [ingest_pipeline.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/ingest_pipeline.rs)

추가한 내용:

- `SIDECAR_PREVIEW_DISCOVERY_TIMEOUT_MS = 900`
- `wait_for_existing_sidecar_preview_asset(...)`

의미:

- helper가 preview JPG를 아주 조금 늦게 쓰는 경우,
  host가 곧바로 `.svg` fallback으로 굳지 않게 잠깐 기다린다.

이건 "새 촬영부터 실제 JPG를 더 잘 잡게 하려는 보강"이지,
기존 세션에 이미 남은 `.svg`만으로 실제 사진을 복원하는 기능은 아니다.

## 이번 이슈에서 실제로 시도한 방법

### 시도 1. 세션 구조와 프리셋 상태 확인

한 일:

- `session_manifest.json`을 찾았으나 실제 파일은 `session.json`임을 확인
- `activePreset`, capture별 preset binding, `renderStatus`, `preview.assetPath` 확인

결론:

- 프리셋 누락 아님
- preview 준비 중 아님
- `previewReady` + `.svg` placeholder 상태

### 시도 2. preview 폴더 실물 확인

한 일:

- 사용자가 준 `renders\previews` 폴더를 직접 조회
- 파일 개수, 이름, 확장자 확인
- `.svg` 파일 내용 직접 확인

결론:

- 실제 JPG/PNG는 없음
- placeholder SVG만 존재

### 시도 3. frontend 표시 경로 보강

한 일:

- `.svg` local asset도 화면에서 렌더되도록 `SessionPreviewImage` 추가
- `LatestPhotoRail`에서 raw `<img>` 대신 wrapper 사용

결론:

- placeholder가 있을 때 빈 카드보다는 나은 표시 경로를 마련함
- 하지만 이건 "실제 촬영본이 보이게 하는 것"과는 다른 층위

### 시도 4. host manifest 자동 복구 경로 추가

한 일:

- manifest가 `.svg`를 가리켜도 나중에 `.jpg`가 생기면 그쪽으로 교체

결론:

- 나중에 raster preview가 생기는 경우 회복 가능
- 하지만 현재 재현 세션에는 실제 JPG가 없어서 이 로직만으로는 즉시 해결되지 않음

### 시도 5. helper에서 RAW preview JPG 만들기 시도

한 일:

- `EdsDownloadThumbnail(...)`
- `EdsSaveImage(...)`
- Windows shell thumbnail fallback
- helper loop backfill

결론:

- 코드와 빌드는 성립
- 실제 재현 세션의 CR2 파일들에 대해 JPG 생성이 확인되지는 않음
- 따라서 이 층위는 아직 "가설/후보 해결책" 단계

### 시도 6. host가 placeholder로 너무 빨리 굳지 않게 대기 추가

한 일:

- `complete_preview_render_in_dir(...)`가 sidecar JPG를 최대 약 900ms 기다리도록 보강

결론:

- helper가 약간 늦게 쓰는 JPG를 놓칠 가능성은 줄였음
- 이미 placeholder만 남은 과거 세션을 자동 복원하진 않음

## 이번 회차에서 확인한 오진 포인트

### 오진 1. 프리셋이 없어서 현재 세션 사진이 안 보인다

틀림.

- 재현 세션은 `Daylight` 프리셋이 정상으로 붙어 있었다.

### 오진 2. `현재 세션 사진`이 비니 selector가 빈 배열을 내는 문제다

절반만 맞다.

- selector 자체는 previewReady capture를 잘 뽑을 수 있었다.
- 실제로는 asset path가 `.svg` placeholder라 화면 렌더와 preview 품질 문제가 더 컸다.

### 오진 3. previewWaiting이 길어서 안 보인다

틀림.

- 이 세션은 이미 `previewReady`였다.
- 문제는 "준비 완료로 표기된 결과물이 실제 사진이 아님"이었다.

### 오진 4. `.svg`가 있으니 어쨌든 화면에는 보여야 한다

틀릴 수 있다.

- Tauri/WebView local file 경로와 `<img src>` 조합에서는 `.svg`가 깨질 수 있었다.
- 그래서 인라인 처리 보강이 필요했다.

## 자동 검증에서 확인한 것

통과한 프런트 검증:

- `pnpm vitest run src/booth-shell/components/SessionPreviewImage.test.tsx src/booth-shell/screens/CaptureScreen.test.tsx src/session-domain/selectors/current-session-previews.test.ts`

통과한 host 검증:

- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness readiness_repairs_a_placeholder_svg_preview_when_a_raster_sidecar_exists -- --exact`
- `cargo test --manifest-path src-tauri/Cargo.toml --test capture_readiness complete_preview_render_waits_briefly_for_a_delayed_sidecar_preview_before_falling_back -- --exact`

중요한 운영 메모:

- 기본 `src-tauri\target`은 실행 중인 `boothy.exe`가 파일을 잡고 있어 테스트가 막힐 수 있었다.
- 이 경우 `CARGO_TARGET_DIR`를 별도 디렉터리로 분리해 검증했다.

## 아직 풀리지 않은 부분

사용자 최신 피드백 기준으로는 **여전히 실제 앱에서 동작하지 않는다.**

따라서 아래 둘 중 하나 이상이 남아 있을 가능성이 높다.

### 가설 1. 실제 앱이 아직 새 frontend/runtime 번들을 쓰지 않는다

근거:

- `.svg` 인라인 표시 보강이 들어갔으면 최소한 완전한 빈 카드보다는 placeholder가 보여야 한다.
- 그런데 사용자 피드백은 여전히 "동작하지 않는다"였다.

남은 가능성:

- 열려 있는 앱/윈도우가 예전 번들을 계속 사용 중
- dev/runtime 재시작이 되지 않음
- 다른 빌드 산출물을 보고 있음

### 가설 2. live preview 생성 자체가 계속 실패하고 있고, placeholder조차 UI에 안 올라오는 별도 경계가 있다

근거:

- 실제 세션 폴더에는 JPG가 끝내 생기지 않았다.
- helper의 preview 생성 fallback은 build는 되지만 장비에서 성공이 아직 증명되지 않았다.

남은 가능성:

- Canon SDK의 thumbnail/save path가 EOS 700D + 현재 RAW 설정에서 실제로 동작하지 않음
- Windows shell thumbnail provider가 이 머신의 CR2에 대해 비활성
- frontend가 `convertFileSrc` + fetch 조합에서 특정 자산을 못 읽는 다른 경계가 있음

## 다음 에이전트가 바로 보면 좋은 파일

세션 증거:

- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc\session.json`
- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc\renders\previews\capture_20260329070018096_83e398b0eb.svg`
- `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a13dbaf2adabfc\renders\previews\capture_20260329070025071_19704dba0d.svg`

host:

- [ingest_pipeline.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/ingest_pipeline.rs)
- [normalized_state.rs](/C:/Code/Project/Boothy/src-tauri/src/capture/normalized_state.rs)
- [capture_readiness.rs](/C:/Code/Project/Boothy/src-tauri/tests/capture_readiness.rs)

frontend:

- [LatestPhotoRail.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/LatestPhotoRail.tsx)
- [SessionPreviewImage.tsx](/C:/Code/Project/Boothy/src/booth-shell/components/SessionPreviewImage.tsx)
- [current-session-previews.ts](/C:/Code/Project/Boothy/src/session-domain/selectors/current-session-previews.ts)

helper:

- [CanonSdkCamera.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs)
- [WindowsShellThumbnail.cs](/C:/Code/Project/Boothy/sidecar/canon-helper/src/CanonHelper/Runtime/WindowsShellThumbnail.cs)

## 다음 에이전트용 빠른 점검 순서

1. 사용자가 보는 실제 앱이 새 번들인지 먼저 확인한다.
2. 같은 세션 preview 폴더에 raster preview 파일이 생겼는지 먼저 본다.
3. `session.json`의 `preview.assetPath`가 무엇을 가리키는지 확인한다.
4. `.svg`만 있다면 UI가 그 placeholder를 실제로 렌더하는지 확인한다.
5. 실제 JPG가 생기지 않는다면 helper preview 생성 경계를 다시 집중 점검한다.
6. 기본 target 잠금이 있으면 `CARGO_TARGET_DIR`를 별도로 잡고 검증한다.

## 이 문서의 한 줄 요약

이번 이슈의 핵심은 `현재 세션 사진`이 프리셋 문제로 비는 것이 아니라,
RAW 촬영 뒤 실제 preview raster가 생성되지 않아 `.svg` placeholder가 previewReady로 굳고,
그 placeholder를 UI가 제대로 표시하지 못하거나 최신 번들이 반영되지 않는 경계에 있다.

## 2026-04-01 00:15 +09:00 후속 메모: 카메라 회귀는 해결됐고, 남은 건 `최근 세션` 이미지 표현 경계다

- 사용자 실기기 재검증 결과:
  - 초점 실패 뒤에도 다시 `Ready`로 복귀한다.
  - 같은 세션에서 즉시 재촬영도 가능하다.
- 즉 이번 시점의 남은 문제는 더 이상 helper readiness나 capture blocking이 아니다.

Story 1.8과의 관계 정리:

- Story 1.8 문서 자체는 아직 `review` 상태지만, 코드 범위는 대부분 이미 반영되어 있다.
- 특히 문서 기준으로도
  - published bundle runtime loader
  - preview/final render worker 연결
  - `previewReady` / `finalReady` gating
  - drift protection
  는 구현 완료로 체크돼 있다.
- 아직 비어 있는 항목은 "실제 하드웨어에서 서로 다른 preset 두 개로 preview/final 산출물 차이를 canonical evidence로 닫는 검증"이다.

현재 제품 관점 해석:

- 따라서 지금 보이는 `최근 세션 이미지가 제대로 표현되지 않음`을
  단순히 "Story 1.8이 미구현이라서 생긴 문제"로 단정하면 안 된다.
- 1.8이 닫는 것은 주로
  - 선택한 preset이 실제 preview/final 결과물에 반영되는지
  - false `previewReady` / false `Completed`를 막는지
  쪽이다.
- 반면 최근 세션 rail에서 이미지가 보이지 않는 문제는 여전히 아래 경계가 따로 남아 있을 수 있다.
  - session-scoped asset path sanitizer가 preview path를 제거한 경우
  - 실제 raster preview 대신 `.svg` placeholder 또는 읽기 불가능한 자산만 남은 경우
  - Tauri/WebView가 local preview asset을 `<img>`로 못 읽는 경우

현재 판단:

- Story 1.8을 "제대로 구현했다"는 사실만으로 최근 세션 이미지 표시가 자동으로 보장되지는 않는다.
- 다만 1.8의 하드웨어 close evidence가 확보되면, 적어도 "preview 자체가 selected preset render 결과물인가"에 대한 불확실성은 크게 줄어든다.
- 이번 증상은 1.8 미구현 여부보다, recent/current session preview rail이 실제 preview asset을 어떻게 읽고 sanitizing하는지 추가 확인이 필요하다.

## 2026-04-01 00:44 +09:00 구현 완료: 최근 세션 이미지 미표시의 직접 원인은 fake `.jpg` render output이었다

이번 회차에서 확인한 직접 원인:

- 현재 Rust render worker는 `darktable-cli` invocation metadata를 텍스트로 조립해
  `.jpg` 경로에 그대로 기록하고 있었다.
- 즉 `previewReady`로 보이던 일부 preview asset은
  - `.svg` placeholder
  - 또는 확장자만 `.jpg`이고 실제 내용은 이미지가 아닌 텍스트
  인 상태가 될 수 있었다.
- 이런 파일은 rail 입장에서는 "path는 있는데 실제 표시할 raster는 없는 것"이어서,
  최근 세션에서 이미지가 비거나 깨져 보이는 증상과 정확히 맞아떨어졌다.

이번 회차 수정:

- `render/mod.rs`는 이제 실제 `darktable-cli` 프로세스를 실행하고,
  종료 코드 + 출력 파일 존재 + JPEG 시그니처를 확인한 뒤에만 `previewReady` / `finalReady`를 기록한다.
- legacy 잘못된 preview truth는 readiness 계산 시 자동 복구한다.
  - `.svg` placeholder
  - 파일 없음
  - 텍스트 `.jpg`
  같은 preview는 더 이상 ready 근거로 유지하지 않는다.
- 현재 세션 rail 이미지 컴포넌트는 raster load 실패 시 broken image 대신 booth-safe fallback을 보여준다.

정리된 제품 판단:

- 이번 문제는 단순히 "Story 1.8이 아직 review 상태라서"가 아니라,
  **render seam이 실제 raster 산출물 기준으로 닫히지 않았던 구현 결손**이 직접 원인이었다.
- 따라서 이번 수정 이후에는
  - 새 촬영은 실제 JPEG preview가 생성될 때만 최근 세션에 노출되고
  - 과거의 잘못된 previewReady도 자동 복구 또는 재렌더 대상이 된다.

검증:

- `cargo test --test capture_readiness --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `cargo test --test session_manifest --target-dir C:\Code\Project\Boothy\src-tauri\target-supervisor-check`
- `pnpm vitest run src/booth-shell/components/SessionPreviewImage.test.tsx`
- 모두 통과했다.

## 2026-04-01 01:19 +09:00 후속 현장 검증: 최근 세션 반영 속도가 여전히 비정상적으로 느리다

사용자 실기기 피드백:

- 현재는 preview와 `최근 세션` 사진이 결국 올라오기는 한다.
- 다만 `Preview Waiting`에서 실제 사진이 보이기까지, 그리고 같은 사진이 `최근 세션`에 나타나기까지
  체감상 `5초 이상` 걸린다.
- 사용자는 이 속도를 "프로덕트로 사용하지 못할 수준"으로 평가했다.

정리:

- 이 시점의 주 이슈는 "표시가 안 된다"가 아니라 **표시는 되지만 latency가 너무 크다** 쪽으로 이동했다.
- 따라서 current/recent session preview 경계는 correctness보다는 responsiveness 문제로 다시 봐야 한다.

후속 분석 방향:

- `darktable-cli` 실행 시간이 실제로 대부분을 차지하는지,
  아니면 render 완료 뒤 host readiness/manifest sync 또는 프런트 구독 갱신이 늦는지 분리 측정이 필요하다.
- 다음 회차에는 최소 아래 구간을 각각 계측해야 한다.
  - capture persisted
  - preview render start
  - render ready
  - current session preview visible
  - recent session visible

제품 판단:

- Story 1.8의 correctness close 여부와 별개로,
  이 latency가 줄지 않으면 booth product acceptance는 여전히 어렵다.

## 2026-04-01 01:37 +09:00 구현 후 측정: 저해상도 actual render만으로는 2초 목표를 닫기 어렵다

이번 회차 구현:

- current/recent session이 보는 preview는 full-size 대신
  `darktable-cli --hq false --width 1280 --height 1280` 기반 display-sized actual render로 바꿨다.
- rail 이미지가 실제로 로드된 시점도 `current-session-preview-visible` 로그로 남기게 했다.

자동 검증:

- Rust render/capture 테스트와 targeted vitest는 모두 통과했다.

그러나 수동 측정 결과:

- 같은 RAW/XMP를 실제 darktable CLI로 직접 측정했을 때
  - full-size preview: 약 `8652ms`
  - `1280x1280` preview: 약 `5973ms`
  - `640x640` preview: 약 `6894ms`
  수준이었다.

의미:

- 이 장비에서는 "actual preset-applied preview를 darktable로 한 번 렌더한다"는 사실 자체가 이미 수초 단위 비용이다.
- 즉 이번 문제는 단순한 프런트 stale update나 polling 지연만으로 설명되지 않는다.
- 저해상도 actual render 최적화는 유지하되,
  booth product 기준 `2초 이내`를 요구한다면 다음 단계는 구조적 대안 검토가 필요할 수 있다.

현재 판단:

- representative tile/sample-cut은 기본 fallback으로 채택하지 않았다.
- 따라서 다음 후보는 "darktable를 쓰지 않는 더 빠른 same-capture thumbnail source" 또는
  "capture 직후 빠른 booth-safe proxy와 이후 actual render를 명확히 구분하는 별도 제품 설계" 쪽이다.

## 2026-04-01 01:43 +09:00 사용자 최종 확인: 현재 preview/current session 상태는 최신 정상 기준선이다

- 사용자 실기기 재확인 결과, 현재 상태는 문제없이 정상 동작한다.
- `Preview Waiting` 이후 실제 사진 노출과 현재 세션 사진 반영이 현 시점 기준으로 사용 가능 상태로 확인됐다.

운영 메모:

- 이 시점을 preview/current session 경계의 최신 정상 baseline으로 취급한다.
- 다음 회차에서 1.8 hardware close 또는 추가 최적화를 진행하더라도,
  회귀 여부는 반드시 이 상태와 비교해 판단한다.
