# 프리셋 번들 계약

## 목적

이 문서는 booth runtime과 authoring publication flow가 함께 참조하는 `published-preset-bundle/v1`의 authoritative baseline을 고정한다.
Story 1.14는 닫힌 published bundle baseline만 고정하고, publication workflow의 deeper state machine은 Story 1.15 / 4.x가 계속 소유한다.

## Authoritative Source / 소비 경계

- 문서 기준: 이 문서
- TypeScript 기준: `src/shared-contracts/schemas/preset-core.ts`
- Rust 기준: `src-tauri/src/preset/preset_bundle.rs`
- 대표 fixture:
  - `tests/fixtures/contracts/preset-bundle-v1/preset_soft-glow/2026.04.10/bundle.json`
- 소비 surface:
  - booth는 catalog summary와 runtime loader를 통해 읽는다.
  - authoring/publish host는 동일한 bundle shape를 canonical output으로 만든다.
  - operator는 preset identity / publishedVersion / booth-safe 여부를 같은 baseline으로 본다.

## Published Preset Bundle v1

- `schemaVersion`: `published-preset-bundle/v1`
- `presetId`: `preset_*` 형식의 stable identifier
- `displayName`: booth 고객에게 보이는 이름
- `publishedVersion`: `YYYY.MM.DD`
- `lifecycleStatus`: `published`
- `boothStatus`: `booth-safe`
- runtime render metadata:
  - `darktableVersion`: pinned runtime version, 현재 기준 `5.4.1`
  - `xmpTemplatePath`: bundle root 내부 XMP sidecar template
  - `previewProfile`: `{ profileId, displayName, outputColorSpace }`
  - `finalProfile`: `{ profileId, displayName, outputColorSpace }`
- `preview`:
  - `kind`: `preview-tile` 또는 `sample-cut`
  - `assetPath`: bundle root 내부 파일
  - `altText`: 고객에게 보여 줄 설명
- optional publish metadata:
  - `darktableProjectPath`
    - optional legacy authoring reference only
    - runtime loader의 필수 입력이 아니다
  - `sourceDraftVersion`
  - `publishedAt`
  - `publishedBy`
  - `sampleCut`

## 런타임 로더 규칙

- booth catalog loader는 `preset-catalog/published/**/bundle.json`만 읽는다.
- `lifecycleStatus != published` 또는 `boothStatus != booth-safe`면 무시한다.
- `preview.assetPath` 또는 `xmpTemplatePath`가 bundle root 밖으로 벗어나면 무시한다.
- runtime render loader는 `darktableVersion`, `xmpTemplatePath`, `previewProfile`, `finalProfile`이 모두 채워져 있어야 한다.
- `darktableProjectPath`가 없어도 published bundle은 유효하다.
- preview asset은 booth catalog summary와 customer-visible top-6 selection에 사용되고, render metadata는 runtime render loader가 그대로 재사용한다.
- draft 또는 validated artifact는 이 로더 경계에 들어오면 안 된다.
- publish host는 기존 `presetId/publishedVersion` 디렉터리를 in-place 수정하면 안 된다.

## Catalog State v1 관계

- live future-session catalog truth는 immutable bundle directory와 별도의 `preset-catalog/catalog-state.json`이 소유한다.
- `catalog-state.json`은 preset identity별 현재 live published version과 `catalogRevision`을 기록한다.
- active session은 `session.json.catalogSnapshot`에 고정된 version만 사용한다.
- publish와 rollback은 immutable bundle을 지우지 않고 live pointer만 갱신한다.

## 범위 경계

- Story 1.14가 닫는 범위:
  - booth runtime이 읽는 published bundle 필수 필드
  - preview/final render profile baseline
  - booth/operator/authoring이 같은 bundle identity와 publishedVersion을 참조하는 기준
- Story 1.15 / 4.x가 닫는 범위:
  - approval, publication, rollback state machine의 deeper behavior
  - future-session-only publication governance 세부
- Story 1.16이 닫는 범위:
  - build/release packaging baseline

## 검증 기준

- TypeScript fixture parse: `publishedPresetBundleSchema`
- Rust runtime loader parse: `load_published_preset_runtime_bundle`
- representative fixture path:
  - `tests/fixtures/contracts/preset-bundle-v1/preset_soft-glow/2026.04.10/`
