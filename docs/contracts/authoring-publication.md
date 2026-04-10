# Authoring Publication 계약

## 목적

이 문서는 validated draft를 immutable published bundle로 게시할 때 authoring surface와 host가
공유하는 입력, 결과, 거절, 감사 이력 계약을 고정한다.

## 문서 역할

- `docs/contracts/authoring-publication.md`: publication state machine, guardrail, approval/published 의미
- `docs/contracts/authoring-publication-payload.md`: publish input/result/audit payload shape baseline
- `docs/contracts/authoring-validation.md`: draft validation artifact와 validation result baseline

## Publish Input

- `presetId`: 게시할 draft의 stable identifier
- `draftVersion`: approver가 검토한 draft version
- `validationCheckedAt`: approver가 신뢰한 latest validation timestamp
- `expectedDisplayName`: 승인 검토에 사용한 표시 이름
- `publishedVersion`: `YYYY.MM.DD`
- `actorId`: host audit용 승인자 identifier
- `actorLabel`: authoring UI에 표시할 승인자 이름
- `scope`: `future-sessions-only` 또는 테스트용 거절 입력 `active-session`
- `reviewNote`: 선택 메모

## Publish Result

- `schemaVersion`: `draft-preset-publication-result/v1`
- `status`:
  - `published`: immutable bundle 생성 완료
  - `rejected`: publish side effect 없이 멈춘 typed rejection
- 공통 draft payload는 최신 draft snapshot과 publication history를 포함한다.

### Published

- `publishedPreset`: booth runtime이 바로 읽을 수 있는 published preset summary
- `bundlePath`: app-local-data 아래 immutable bundle directory
- `auditRecord.action`: `published`
- success draft의 `publicationHistory`에는 같은 `publishedVersion`에 대한 `approved`, `published`
  두 기록이 순서대로 남아야 한다.

### Rejected

- `reasonCode`:
  - `draft-not-validated`
  - `stale-validation`
  - `metadata-mismatch`
  - `duplicate-version`
  - `path-escape`
  - `future-session-only-violation`
  - `stage-unavailable`
- `message`: authoring 상단 상태 문구
- `guidance`: 사용자가 바로 조치할 수 있는 수정 가이드
- `auditRecord.action`: `rejected`

`stage-unavailable`는 preview/inspection 단계에서만 허용되는 typed rejection이다.
이 경우 bundle 생성, live pointer 변경, rejection audit 저장을 수행하지 않는다.

## Audit Record

- `schemaVersion`: `preset-publication-audit/v1`
- `presetId`, `draftVersion`, `publishedVersion`
- `actorId`, `actorLabel`
- `action`: `approved` | `published` | `rejected`
- `reviewNote`: 승인자 검토 메모. 없으면 `null`
- `reasonCode`: `approved`/`published`는 `null`, `rejected`는 위 reason code 중 하나
- `guidance`: 당시 사용자에게 보여 준 조치 문구
- `notedAt`: host timestamp

## Guardrails

- publish는 `future-sessions-only` scope만 성공할 수 있다.
- publish 성공은 `approved -> published` 전이를 publication history에 남겨야 한다.
- duplicate version은 기존 bundle directory를 절대 수정하지 않고 거절해야 한다.
- stale validation이나 metadata mismatch는 partial bundle 없이 거절해야 한다.
- rejection audit는 bundle truth와 분리된 host-owned store에 남아야 한다.
- typed rejection은 bundle 생성, live pointer 변경, active session mutation 없이 끝나야 한다.
- publish 성공도 active session manifest나 current capture binding을 직접 갱신하면 안 되고,
  audit/draft 저장이 실패하면 live bundle도 함께 롤백되어야 한다.
- rollback은 immutable bundle 삭제가 아니라 live catalog pointer 전환으로만 성공할 수 있다.
- rollback 성공/거절 payload는 `preset-catalog-rollback-result/v1`를 따르고,
  active session manifest나 current capture binding을 직접 수정하면 안 된다.
