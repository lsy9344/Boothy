# Canonical Preset Recipe 계약

## 목적

이 문서는 booth runtime, authoring publication, fallback/oracle, future GPU lane이
공유할 최소 preset truth shape를 고정한다.

## Schema

- `schemaVersion`: `canonical-preset-recipe/v1`
- `presetId`: `preset_*` stable identifier
- `publishedVersion`: `YYYY.MM.DD`
- `displayName`: booth-safe 표시 이름
- `boothStatus`: `booth-safe`
- `previewIntent`
  - `profileId`
  - `displayName`
  - `outputColorSpace`
- `finalIntent`
  - `profileId`
  - `displayName`
  - `outputColorSpace`
- `noisePolicy`
  - `policyId`
  - `displayName`
  - `reductionMode`

## 범위 경계

- 이 스키마는 최소 공통 truth만 담는다.
- darktable module slider, full shader graph, live GPU execution detail은 포함하지 않는다.
- XMP, `.dtpreset`, darktable project path는 canonical recipe에 들어오지 않는다.
- customer-facing vocabulary는 그대로 booth-safe wording만 사용한다.

## Adapter 분리 규칙

- canonical recipe는 editor-agnostic truth다.
- darktable XMP는 recipe를 실행하거나 parity를 검증하기 위한 adapter artifact다.
- published bundle은 recipe와 adapter reference를 함께 묶지만, 둘의 책임은 혼합하지 않는다.
