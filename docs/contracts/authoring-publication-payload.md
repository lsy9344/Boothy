# Authoring Publication Payload 계약

## 목적

이 문서는 publish 시점에 authoring UI와 host가 직접 주고받는 payload baseline을 고정한다.

## Publish Input v1

- `presetId`
- `draftVersion`
- `validationCheckedAt`
- `expectedDisplayName`
- `publishedVersion`
- `actorId`
- `actorLabel`
- `scope`
- `reviewNote`

## Publish Result v1

- `schemaVersion`: `draft-preset-publication-result/v1`
- `status`: `published` | `rejected`

### Published result

- `publishedPreset`
- `bundlePath`
- `auditRecord.action = published`
- `bundlePath/bundle.json`은 `published-preset-bundle/v2`여야 한다.
- 해당 bundle은 `canonicalRecipe`와 `darktableAdapter`를 함께 포함해야 한다.

### Rejected result

- `reasonCode`
- `message`
- `guidance`
- `auditRecord.action = rejected`

## Payload Guardrails

- publish payload는 validated draft와 latest validation snapshot을 전제로 한다.
- publication truth는 더 이상 XMP 단독이 아니다.
- canonical recipe가 primary truth이고, darktable adapter는 compatibility/fallback reference다.
- `darktableProjectPath`는 optional adapter metadata다.
- rejection payload는 partial bundle 또는 active session mutation을 동반하면 안 된다.
