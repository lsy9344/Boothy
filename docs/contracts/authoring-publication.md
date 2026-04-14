# Authoring Publication 계약

## 목적

이 문서는 validated draft를 immutable published bundle로 게시할 때 authoring surface와 host가
공유하는 state machine과 guardrail을 고정한다.
Story 1.17부터 publish 성공 artifact는 `published-preset-bundle/v2`이며,
그 안에 `canonical recipe + darktable adapter`를 함께 담아야 한다.

## Publish Input

- `presetId`
- `draftVersion`
- `validationCheckedAt`
- `expectedDisplayName`
- `publishedVersion`
- `actorId`
- `actorLabel`
- `scope`
- `reviewNote`

## Publish Result

- `schemaVersion`: `draft-preset-publication-result/v1`
- `status`: `published` | `rejected`
- 공통 payload는 최신 draft snapshot과 publication history를 포함한다.

### Published

- `publishedPreset`: booth runtime이 읽는 published preset summary
- `bundlePath`: immutable published bundle directory
- `auditRecord.action = published`
- 성공 bundle은 반드시 아래를 함께 포함해야 한다.
  - `canonicalRecipe`
  - `darktableAdapter`
  - booth-safe preview/sample-cut artifact

### Rejected

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

## Guardrails

- publish는 `future-sessions-only` scope만 성공할 수 있다.
- duplicate version은 기존 bundle directory를 절대 수정하지 않고 거절해야 한다.
- stale validation이나 metadata mismatch는 partial bundle 없이 거절해야 한다.
- canonical recipe와 darktable adapter는 같은 publish transaction 안에서 함께 직렬화돼야 한다.
- publish 성공도 active session manifest나 current capture binding을 직접 갱신하면 안 된다.
- rollback은 immutable bundle 삭제가 아니라 live catalog pointer 전환으로만 성공할 수 있다.
