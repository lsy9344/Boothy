# Render Worker 계약

## 목적

이 문서는 booth runtime이 published preset bundle을 실제 preview/final 산출물로 연결할 때 지켜야 하는 기준선을 고정한다. 현재 승인된 다음 구조는 host-owned `local dedicated renderer`이며, 이 문서는 그 구조의 booth-safe contract를 설명한다.

## Runtime 입력 기준

- local dedicated renderer는 live catalog pointer가 아니라 capture record에 저장된
  `activePresetId + activePresetVersion`을 사용한다.
- host는 dedicated renderer를 임의 executable 경로로 직접 띄우지 않는다.
  bundle에 포함된 sidecar 이름 `../sidecar/dedicated-renderer/boothy-dedicated-renderer`와
  승인된 `--protocol`, `--request`, `--result` 인수만 사용한다.
- runtime bundle loader는 최소 아래 필드를 읽어야 한다.
  - `presetId`
  - `publishedVersion`
  - `darktableVersion`
  - `xmpTemplatePath`
  - `previewProfile`
  - `finalProfile`
- `darktableVersion`은 pinned `5.4.1`과 일치해야 한다.
- `xmpTemplatePath`는 bundle root 내부의 실제 파일이어야 한다.
- preview lane의 기본 invocation은 booth hardware에서 검증된 known-good contract를 사용해야 한다.
- speculative 또는 실험적 invocation flag는 별도 승인 없이는 기본 booth path에 포함되면 안 된다.

## Preview 규칙

- first-visible preview 경로는 booth UX 보호를 위해 별도 lane으로 존재할 수 있지만, preset-applied truthful close의 owner는 host-owned local dedicated renderer다.
- preset 선택 또는 세션 시작 시 dedicated renderer warm-up, preset preload, cache priming을 허용할 수 있지만 capture truth를 막으면 안 된다.
- dedicated renderer가 queue saturation, warm-state loss, restart, invalid output에 부딪히면 booth는 false-ready 없이 기존 truthful `Preview Waiting`과 approved fallback path로 내려가야 한다.
- preview result의 `schemaVersion`, typed `status`, `sessionId`, `requestId`, `captureId`,
  `diagnosticsDetailPath`, canonical preview output path 검증이 실패하면 host는 이를
  `protocol-mismatch` 또는 `invalid-output`으로 기록하고 inline truthful fallback을 유지한다.
- preview render는 `renders/previews/{captureId}.jpg`를 실제로 만든 뒤에만 `previewReady`를 기록한다.
- 같은 capture의 pending first-visible image가 이미 canonical preview path에 있어도, dedicated renderer는 그 경로를 same-path preset-applied output으로 직접 교체할 수 있어야 한다.
- first-visible image가 먼저 보였더라도 dedicated renderer만이 truthful `previewReady`와 `preview.readyAtMs`를 올릴 수 있다.
- dedicated renderer가 같은 capture의 preset-applied preview file을 성공적으로 만들었다면, 그 시점이 곧 truthful `previewReady` close다.
- same-path 교체가 실패하더라도 runtime은 기존 canonical preview를 먼저 잃어버리는 방식으로 downgrade하면 안 된다.
- RAW copy, placeholder SVG, bundle 대표 preview tile은 `previewReady` 성공 산출물로 승격하면 안 된다.
- booth는 dedicated renderer가 실제 preset-applied preview를 만들기 전까지 `Preview Waiting`을 유지해야 한다.

## Final 규칙

- final render는 `renders/finals/{captureId}.jpg`를 실제로 만든 뒤에만 `finalReady`를 기록한다.
- post-end `Completed`는 `finalReady`가 없는 상태에서 올라가면 안 된다.
- post-end에서 preview만 준비된 상태는 `export-waiting`으로 유지한다.

## Drift 보호

- capture 이후 publish/rollback 또는 active preset 변경이 있어도,
  이미 저장된 capture render는 capture record에 저장된 version으로만 다시 계산한다.
- runtime은 capture-bound bundle을 찾지 못하면 조용히 최신 live version으로 대체하면 안 된다.

## 진단과 실패

- render failure는 customer surface에 darktable/XMP/filesystem 경로를 노출하지 않는다.
- diagnostics에는 safe event만 남긴다.
  - `preview-render-start`
  - `preview-render-ready`
  - `preview-render-failed`
  - `preview-render-queue-saturated`
  - `fast-preview-visible` 또는 동등 first-visible event
  - `capture_preview_ready`
  - `recent-session-visible`
  - `final-render-start`
  - `final-render-ready`
  - `final-render-failed`
  - `final-render-queue-saturated`
- warm-up 결과는 최소 `fallback-suggested`, `warmed-up`, `restarted`, `protocol-mismatch`
  typed 상태로 남겨 host integration과 seam review에서 구분 가능해야 한다.
- preview/final failure는 저장된 RAW와 기존 session asset을 보존한 채 bounded failure truth로 기록한다.
