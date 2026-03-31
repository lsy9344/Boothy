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

## Preview 규칙

- preview render는 `renders/previews/{captureId}.jpg`를 실제로 만든 뒤에만 `previewReady`를 기록한다.
- RAW copy, placeholder SVG, bundle 대표 preview tile은 `previewReady` 성공 산출물로 승격하면 안 된다.
- booth는 preview render가 닫히기 전까지 `Preview Waiting`을 유지해야 한다.

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
  - `render-ready`
  - `render-failed`
- preview/final failure는 저장된 RAW와 기존 session asset을 보존한 채 bounded failure truth로 기록한다.
