# Authoring Validation 계약

## 목적

이 문서는 Story 4.2 authoring draft와 validation result 기준을 정리한다.
Story 1.17 이후 draft validation은 canonical recipe를 만들기 위한 입력 completeness와
darktable adapter compatibility를 함께 점검한다.

## Draft Preset Artifact v1

- `schemaVersion`: `draft-preset-artifact/v1`
- `presetId`
- `displayName`
- `draftVersion`
- `lifecycleState`
- adapter metadata
  - `darktableVersion`
  - optional `darktableProjectPath`
  - `xmpTemplatePath`
- canonical recipe 입력 metadata
  - `previewProfile`
  - `finalProfile`
  - `noisePolicy`
- booth-safe assets
  - `preview`
  - `sampleCut`
- `validation`
- `updatedAt`

## Host Validation 책임

- React는 host가 반환한 validation result를 표시만 한다.
- Rust host는 최소 아래 규칙을 평가한다.
  - 필수 artifact 존재 여부
  - draft root 내부의 안전한 경로 여부
  - pinned darktable version 일치 여부
  - preview/final profile completeness
  - noise policy completeness
  - preview/sample-cut asset completeness
  - XMP template의 render compatibility 힌트 존재 여부

## 해석 규칙

- draft의 `previewProfile`, `finalProfile`, `noisePolicy`는 publish 시 `canonicalRecipe`로 승격된다.
- `xmpTemplatePath`, `darktableVersion`, optional `darktableProjectPath`는 publish 시 `darktableAdapter`로 승격된다.
- `.dtpreset`는 validation 통과 조건이 아니다.
