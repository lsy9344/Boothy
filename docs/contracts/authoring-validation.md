# Authoring Validation 계약

## 목적

이 문서는 Story 4.2에서 사용하는 internal authoring payload와 validation result 기준을 정리한다.

## Draft Preset Artifact v1

- schemaVersion: `draft-preset-artifact/v1`
- presetId
- displayName
- draftVersion
- lifecycleState: `draft` 또는 `validated`
- darktableVersion
- darktableProjectPath: optional, legacy authoring reference metadata
- xmpTemplatePath
- previewProfile
  - profileId
  - displayName
  - outputColorSpace
- finalProfile
  - profileId
  - displayName
  - outputColorSpace
- noisePolicy
  - policyId
  - displayName
  - reductionMode
- preview
  - assetPath
  - altText
- sampleCut
  - assetPath
  - altText
- validation
  - status: `not-run` | `passed` | `failed`
  - latestReport
  - history
- updatedAt

## Validation Report v1

- schemaVersion: `draft-preset-validation/v1`
- presetId
- draftVersion
- lifecycleState: `draft` 또는 `validated`
- status: `passed` | `failed`
- checkedAt
- findings[]
  - ruleCode
  - severity: `error` | `warning`
  - fieldPath: nullable
  - message
  - guidance

## Host Validation 책임

- React는 host가 반환한 validation result를 표시만 한다.
- Rust host는 최소 아래 규칙을 평가한다.
  - 필수 artifact 존재 여부
  - draft root 내부의 안전한 경로 여부
  - pinned darktable version 일치 여부
  - preview/final profile completeness
  - preview/sample-cut asset completeness
  - XMP template의 render compatibility 힌트 존재 여부
- `.dtpreset`는 validation 통과 조건이 아니다. 있으면 안전한 workspace reference인지까지만 본다.

## 상태 전이 규칙

- save/create는 항상 `draft` 상태를 쓴다.
- validation 통과 시에만 `draft -> validated`
- validation 실패 시 lifecycle은 `draft` 유지
- 검증 결과는 validation history에 누적한다.
- `validated`는 내부 approval 준비 상태이며, published catalog나 active session을 직접 바꾸지 않는다.
