# Authoring Publication Payload 계약

## 목적

이 문서는 publication state machine 자체가 아니라, publish 시점에 authoring UI와 host가
직접 주고받는 payload shape을 authoritative baseline으로 고정한다.

validation artifact와 validation result baseline은
`docs/contracts/authoring-validation.md`가 소유한다.

## Publish Input v1

- schemaVersion: command input envelope 없이 typed payload로 전달
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

- schemaVersion: `draft-preset-publication-result/v1`
- `status`: `published` | `rejected`
- 공통 payload:
  - latest draft snapshot
  - publication history
  - audit record

### Published result

- `publishedPreset`
- `bundlePath`
- `auditRecord.action = published`
- same `publishedVersion`에 대해 draft `publicationHistory` 안에
  `approved`, `published`가 순서대로 존재해야 한다.

### Rejected result

- `reasonCode`
  - `draft-not-validated`
  - `stale-validation`
  - `metadata-mismatch`
  - `duplicate-version`
  - `path-escape`
  - `future-session-only-violation`
  - `stage-unavailable`
- `message`
- `guidance`
- `auditRecord.action = rejected`

`stage-unavailable`는 preview 단계에서만 허용되는 rejection reason이다.
이 경우 publish side effect와 rejection audit는 생기지 않는다.

## Publication Audit Record v1

- schemaVersion: `preset-publication-audit/v1`
- `presetId`, `draftVersion`, `publishedVersion`
- `actorId`, `actorLabel`
- `action`: `approved` | `published` | `rejected`
- `reviewNote`
- `reasonCode`
- `guidance`
- `notedAt`

## Rollback Input v1

- `presetId`
- `targetPublishedVersion`
- `expectedCatalogRevision`
- `actorId`
- `actorLabel`

## Rollback Result v1

- schemaVersion: `preset-catalog-rollback-result/v1`
- `status`: `rolled-back` | `rejected`

### Rolled-back result

- `catalogRevision`
- `summary`: updated live catalog summary for the preset
- `auditEntry`: catalog version history entry with `actionType = rollback`
- `message`

### Rejected result

- `reasonCode`
  - `target-missing`
  - `target-incompatible`
  - `already-live`
  - `stale-catalog-revision`
  - `stage-unavailable`
- `message`
- `guidance`
- `catalogRevision`
- `summary`: current preset summary or `null`

## Payload Guardrails

- publish payload는 validated draft와 latest validation snapshot을 전제로 한다.
- publication truth는 `xmpTemplatePath`와 preview/sample-cut artifact다.
- `darktableProjectPath`는 있어도 되고 없어도 되는 optional authoring metadata다.
- rejection payload는 partial bundle 또는 active session mutation을 동반하면 안 된다.
- publish/rollback 모두 active session manifest와 current capture binding을 직접 수정하지 않는다.
- immutable bundle rule과 future-session-only rule 위반은 typed rejection으로만 표면화한다.
