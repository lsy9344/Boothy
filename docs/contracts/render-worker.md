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
- preset 선택 또는 세션 시작 시 preview worker warm-up, preset preload, cache priming을 허용할 수 있지만 capture truth를 막으면 안 된다.
- resident first-visible worker가 queue saturation, warm-state loss, restart, invalid output에 부딪히면 booth는 false-ready 없이 기존 truthful `Preview Waiting`과 normal render follow-up으로 내려가야 한다.
- approved local dedicated renderer adapter가 추가되더라도 그 route는 host 뒤의 candidate-result producer로만 동작해야 하며, host validation 없이 직접 `previewReady`를 소유하면 안 된다.
- preview close route 선택은 host-owned `branch-config/preview-renderer-policy.json` 기준으로만 바뀌어야 하며, default route는 계속 approved darktable baseline이어야 한다.
- local renderer canary는 booth / session / preset 범위 rule로만 opt-in하고, forced fallback rule은 unhealthy sidecar를 customer-safe waiting을 깨지 않고 즉시 우회할 수 있어야 한다.
- resident/speculative worker가 실제 truthful close owner로 승격될 수 있는 구조라면, 그 worker도 같은 session-locked preview route policy를 따라야 한다.
- resident/speculative worker가 local renderer candidate를 만들더라도 host validation, canonical promotion, route diagnostics 기록 전에는 truth owner로 간주하면 안 된다.
- preview render는 `renders/previews/{captureId}.jpg`를 실제로 만든 뒤에만 `previewReady`를 기록한다.
- 같은 capture의 pending fast preview가 이미 canonical preview path에 있어도, render worker는 그 경로를 same-path preset-applied output으로 직접 교체할 수 있어야 한다.
- fast preview가 먼저 보였더라도 render worker만이 truthful `previewReady`와 `preview.readyAtMs`를 올릴 수 있다.
- resident/speculative worker나 local renderer sidecar가 같은 capture의 preset-applied preview candidate를 만들더라도, host validation과 canonical promotion이 끝나기 전에는 truthful close owner로 간주하면 안 된다.
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
  - `renderer-route-selected`
  - `renderer-route-fallback`
  - `renderer-close-owner`
  - resident/speculative close가 truth owner가 된 경우에도 위 3개 route evidence
  - `final-render-start`
  - `final-render-ready`
  - `final-render-failed`
  - `final-render-queue-saturated`
- preview/final failure는 저장된 RAW와 기존 session asset을 보존한 채 bounded failure truth로 기록한다.
- one recent approved session package만으로 route 선택, fallback reason, close-owner 결과를 비교할 수 있어야 한다.
