# Render Worker 계약

## 목적

이 문서는 booth runtime이 published preset bundle을 실제 preview/final 산출물로 연결할 때
지켜야 하는 기준선을 고정한다.

## Runtime 입력 기준

- runtime bundle loader는 최소 아래 필드를 읽어야 한다.
  - `presetId`
  - `publishedVersion`
  - `canonicalRecipe.previewIntent`
  - `canonicalRecipe.finalIntent`
  - `canonicalRecipe.noisePolicy`
  - `darktableAdapter.darktableVersion`
  - `darktableAdapter.xmpTemplatePath`
- `darktableAdapter.darktableVersion`은 pinned `5.4.1`과 일치해야 한다.
- `darktableAdapter.xmpTemplatePath`는 bundle root 내부의 실제 파일이어야 한다.
- runtime truth는 editor 내부 표현 하나가 아니라 capture-bound bundle에 고정된다.

## Preview 규칙

- first-visible preview 경로는 별도 lane으로 존재할 수 있지만, truthful close owner는 host-owned renderer다.
- `previewReady`는 canonical recipe intent와 darktable adapter를 사용해 같은 capture의 preset-applied preview file을 실제로 만든 뒤에만 기록한다.
- first-visible image가 먼저 보여도 booth는 `Preview Waiting`을 유지해야 한다.
- RAW copy, placeholder SVG, bundle 대표 preview tile은 `previewReady` 성공 산출물로 승격하면 안 된다.
- resident prototype evidence는 `capture_preview_transition_summary` 계열 진단에서 `laneOwner`, `fallbackReason`, `routeStage`, `warmState`,
  `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`를 함께 남겨 Story 1.19 seam으로 이어져야 한다.

## Final 규칙

- `finalReady`도 canonical recipe final intent와 darktable adapter XMP를 사용한 성공 산출물 뒤에만 기록한다.
- post-end `Completed`는 `finalReady`가 없는 상태에서 올라가면 안 된다.

## Drift 보호

- capture 이후 publish/rollback 또는 active preset 변경이 있어도, 이미 저장된 capture render는 capture record에 저장된 version으로만 다시 계산한다.
- runtime은 capture-bound bundle을 찾지 못하면 조용히 최신 live version으로 대체하면 안 된다.
- promotion evidence bundle은 `preview-promotion-evidence.jsonl` record, `session.json`, `timing-events.log`, route policy snapshot, published `bundle.json`,
  `catalog-state.json`을 같이 읽을 수 있어야 한다.
- parity diff gate는 darktable baseline/fallback oracle against 비교만 허용하며, same-capture / same-session / same-preset-version 전제가 깨진 비교는
  `Go` 근거가 될 수 없다.
