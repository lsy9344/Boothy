# Render Worker 계약

## 목적

이 문서는 booth runtime이 published preset bundle을 실제 preview/final 산출물로 연결할 때 지켜야 하는 `render worker` 기준선을 고정한다.

## Runtime 입력 기준

- render worker는 live catalog pointer가 아니라 capture record에 저장된
  `activePresetId + activePresetVersion`을 사용한다.
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

- first-visible preview 경로는 가능하면 warm 상태를 유지하는 resident worker를 우선 사용하고, per-capture one-shot spawn은 fallback 또는 비교 기준으로만 남긴다.
- Story `1.26` reserve path에서 truthful close owner는 host가 소유하는 local native/GPU resident full-screen lane이 만든 `display-sized preset-applied truthful artifact`다.
- 이 reserve artifact가 same-capture `preset-applied-preview`로 canonical preview path에 안전하게 닫히면, host는 per-capture `darktable-cli` preview close를 다시 열지 않고 곧바로 `previewReady`를 기록할 수 있어야 한다.
- preset 선택 또는 세션 시작 시 preview worker warm-up, preset preload, cache priming을 허용할 수 있지만 capture truth를 막으면 안 된다.
- resident first-visible worker가 queue saturation, warm-state loss, restart, invalid output에 부딪히면 booth는 false-ready 없이 기존 truthful `Preview Waiting`과 normal render follow-up으로 내려가야 한다.
- preview render는 `renders/previews/{captureId}.jpg`를 실제로 만든 뒤에만 `previewReady`를 기록한다.
- 같은 capture의 pending fast preview가 이미 canonical preview path에 있어도, render worker는 그 경로를 same-path preset-applied output으로 직접 교체할 수 있어야 한다.
- fast preview가 먼저 보였더라도 truth owner가 아닌 자산은 truthful `previewReady`와 `preview.readyAtMs`를 올릴 수 없다.
- resident/speculative worker가 같은 capture의 preset-applied preview file을 성공적으로 만들었다면, 그 시점이 곧 truthful `previewReady` close다. 이후 RAW 기반 재렌더는 필수 close owner가 아니다.
- `darktable-cli` preview path는 reserve lane의 parity reference, bounded fallback, final/export truth로 남길 수 있지만, Story `1.26` 이후 기본 booth-visible hot path owner로 조용히 복귀하면 안 된다.
- same-path 교체가 실패하더라도 runtime은 기존 canonical preview를 먼저 잃어버리는 방식으로 downgrade하면 안 된다.
- RAW copy, placeholder SVG, bundle 대표 preview tile은 `previewReady` 성공 산출물로 승격하면 안 된다.
- booth는 render worker가 실제 preset-applied preview를 만들기 전까지 `Preview Waiting`을 유지해야 한다.

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
- preview/final failure는 저장된 RAW와 기존 session asset을 보존한 채 bounded failure truth로 기록한다.
