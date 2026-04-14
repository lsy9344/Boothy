# 프리셋 번들 계약

## 목적

이 문서는 booth runtime과 authoring publication flow가 함께 참조하는
`published-preset-bundle/v2`의 authoritative baseline을 고정한다.
Story 1.17부터 preset truth는 XMP 단독 표현이 아니라
`canonical recipe + darktable adapter` 조합으로 해석한다.

## Authoritative Source / 소비 경계

- 문서 기준: 이 문서
- canonical recipe 기준: `docs/contracts/canonical-preset-recipe.md`
- TypeScript 기준: `src/shared-contracts/schemas/preset-core.ts`
- Rust 기준: `src-tauri/src/preset/preset_bundle.rs`
- 대표 fixture:
  - `tests/fixtures/contracts/preset-bundle-v2/preset_soft-glow/2026.04.10/bundle.json`
- 소비 surface:
  - booth는 catalog summary와 runtime loader를 통해 읽는다.
  - authoring/publish host는 동일한 bundle shape를 canonical output으로 만든다.
  - operator는 preset identity / publishedVersion / booth-safe 여부를 같은 baseline으로 본다.

## Published Preset Bundle v2

- `schemaVersion`: `published-preset-bundle/v2`
- top-level identity:
  - `presetId`
  - `displayName`
  - `publishedVersion`
  - `lifecycleStatus = published`
  - `boothStatus = booth-safe`
- `canonicalRecipe`
  - `schemaVersion = canonical-preset-recipe/v1`
  - `presetId`
  - `publishedVersion`
  - `displayName`
  - `boothStatus`
  - `previewIntent`: `{ profileId, displayName, outputColorSpace }`
  - `finalIntent`: `{ profileId, displayName, outputColorSpace }`
  - `noisePolicy`: `{ policyId, displayName, reductionMode }`
- `darktableAdapter`
  - `schemaVersion = darktable-preset-adapter/v1`
  - `darktableVersion`: pinned runtime version, 현재 기준 `5.4.1`
  - `xmpTemplatePath`: bundle root 내부 XMP sidecar template
  - optional `darktableProjectPath`: legacy authoring reference metadata
- `preview`
  - `kind`: `preview-tile` 또는 `sample-cut`
  - `assetPath`
  - `altText`
- optional publish metadata:
  - `sampleCut`
  - `sourceDraftVersion`
  - `publishedAt`
  - `publishedBy`

## 의미 규칙

- canonical recipe가 booth/runtime/future GPU lane이 공유하는 주 truth다.
- darktable adapter는 compatibility / fallback / parity 검증용 참조다.
- bundle top-level identity와 `canonicalRecipe` identity는 같아야 한다.
- XMP path, darktable version, optional project path는 adapter metadata로만 읽는다.
- published bundle은 immutable artifact다. 같은 `presetId/publishedVersion` 디렉터리를 in-place 수정하면 안 된다.

## 런타임 로더 규칙

- booth catalog loader는 `preset-catalog/published/**/bundle.json`만 읽는다.
- `lifecycleStatus != published` 또는 `boothStatus != booth-safe`면 무시한다.
- `preview.assetPath`와 `darktableAdapter.xmpTemplatePath`가 bundle root 밖으로 벗어나면 무시한다.
- runtime render loader는 최소 아래를 읽어야 한다.
  - `canonicalRecipe.previewIntent`
  - `canonicalRecipe.finalIntent`
  - `canonicalRecipe.noisePolicy`
  - `darktableAdapter.darktableVersion`
  - `darktableAdapter.xmpTemplatePath`
- runtime은 legacy `published-preset-bundle/v1`을 읽을 수는 있지만, 새 publish output의 authoritative shape는 `v2`다.
- draft 또는 validated artifact는 이 로더 경계에 들어오면 안 된다.

## Catalog State v1 관계

- live future-session catalog truth는 immutable bundle directory와 별도의 `preset-catalog/catalog-state.json`이 소유한다.
- `catalog-state.json`은 preset identity별 현재 live published version과 `catalogRevision`을 기록한다.
- active session은 `session.json.catalogSnapshot`에 고정된 version만 사용한다.
- publish와 rollback은 immutable bundle을 지우지 않고 live pointer만 갱신한다.

## 검증 기준

- TypeScript fixture parse: `publishedPresetBundleSchema`
- Rust runtime loader parse: `load_published_preset_runtime_bundle`
- representative fixture path:
  - `tests/fixtures/contracts/preset-bundle-v2/preset_soft-glow/2026.04.10/`
