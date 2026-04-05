# Local Renderer Adapter 계약

## 목적

이 문서는 booth host가 `local dedicated renderer sidecar`를 truthful preview close의 후보 생산자로 호출할 때 지켜야 하는 request/response/fallback 기준을 고정한다.

## 경계

- local renderer는 `sidecar/local-renderer/` 경계에 둔다.
- local renderer는 candidate result producer일 뿐이며, `previewReady` truth owner가 아니다.
- host는 direct close뿐 아니라 resident/speculative close 후보 경로에서도 같은 policy lock 아래 local renderer candidate를 호출할 수 있다.
- host만 candidate를 검증하고 canonical preview path 승격과 `preview.readyAtMs` 갱신을 수행한다.
- repo는 `sidecar/local-renderer/local-renderer-sidecar.cmd` bootstrap entrypoint를 포함하고, 전용 executable이 있으면 `BOOTHY_LOCAL_RENDERER_BIN`으로 교체한다.
- host는 repo 경계, 실행 파일 인접 경계, 번들 resource 경계에서 local renderer bootstrap을 탐색할 수 있어야 한다.
- packaged runtime은 bundled resource 경계를 우선 신뢰해야 하며, stale writable data copy가 공식 sidecar보다 먼저 선택되면 안 된다.

## Request Envelope

- schema version: `local-renderer-request/v1`
- 필수 필드:
  - `sessionId`
  - `captureId`
  - `requestId`
- `boothAlias`
- `darktableVersion`은 approved baseline pin으로 간주하며, sidecar는 실제 darktable binary version을 확인해 mismatch를 success로 위장하면 안 된다.
  - `presetId`
  - `presetVersion`
  - `rawAssetPath`
  - `sourceAssetPath`
  - `candidateOutputPath`
  - `xmpTemplatePath`
  - `darktableVersion`
  - `capturePersistedAtMs`
  - `previewWidthCap`
  - `previewHeightCap`
- `candidateOutputPath`는 host가 준비한 세션 범위 staging path여야 한다.
- sidecar는 request에 없는 세션 파일을 직접 수정하면 안 된다.

## Success Response Envelope

- schema version: `local-renderer-response/v1`
- 필수 필드:
  - `route`
  - `sessionId`
  - `captureId`
  - `requestId`
  - `presetId`
  - `presetVersion`
  - `candidatePath`
  - `candidateWrittenAtMs`
  - `elapsedMs`
  - `fidelity.verdict`
  - `attempt.retryOrdinal`
  - `attempt.completionOrdinal`
- host acceptance 조건:
  - same session
  - same capture
  - same request
  - same preset identity/version
  - session-scoped allowed path
  - valid raster
  - non-stale output
  - `completionOrdinal == 1`

## Error / Timeout 규칙

- 기본 timeout은 host 기준 10초다.
- malformed payload, missing response, launch failure, timeout, invalid raster, stale output, wrong session/capture/preset은 모두 candidate rejection으로 취급한다.
- sidecar가 error envelope를 남기면 host는 generic exit failure로 덮어쓰지 말고 operator-safe fallback reason으로 보존해야 한다.
- rejection은 customer-facing false-ready로 이어지면 안 되고, 즉시 approved darktable path fallback으로 이어져야 한다.
- `renderer-route-fallback` diagnostics에는 operator-safe reason detail도 함께 남겨서 session package만으로 fallback 원인을 재구성할 수 있어야 한다.

## Retry / Idempotency

- sidecar는 동일 request에 대한 재시도를 허용할 수 있다.
- host는 `retryOrdinal > 0`인 valid candidate는 받아들일 수 있다.
- duplicate completion(`completionOrdinal != 1`)은 reject한다.

## Fidelity Metadata

- `fidelity.verdict`는 최소 `matched` 또는 vendor-safe 동등 verdict를 제공해야 한다. 실제 비교를 수행하지 않은 bridge route는 success를 과장하는 verdict를 쓰면 안 된다.
- `fidelity.detail`에는 operator-safe comparison detail을 남길 수 있다.
- fidelity verdict는 diagnostics package에서 route 비교 증거로 재사용 가능해야 한다.
- host diagnostics는 `fidelity.detail`도 session package에 남겨서 canary evidence를 비교 가능하게 해야 한다.

## Diagnostics

- 같은 세션 package에서 아래 event를 함께 비교할 수 있어야 한다.
  - `renderer-route-selected`
  - `renderer-route-fallback`
  - `renderer-close-owner`
  - `preview-render-ready`
- resident/speculative candidate가 최종 승격된 경우에도 같은 diagnostics 세트가 유지돼야 한다.
- 고객 화면에는 local renderer, fallback reason, queue saturation 같은 내부 용어를 노출하지 않는다.
