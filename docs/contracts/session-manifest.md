# `session.json` v1 계약

## 목적

이 문서는 booth, operator, authoring, host가 함께 참조하는 `session-manifest/v1`의 authoritative baseline을 고정한다.
Story 1.14는 이 baseline을 잠그는 범위만 소유하고, Canon helper 세부 profile은 Story 1.15가, build/release baseline은 Story 1.16이 닫는다.

## Authoritative Source / 소비 경계

- 문서 기준: 이 문서
- TypeScript 기준: `src/shared-contracts/schemas/session-manifest.ts`, `src/shared-contracts/schemas/session-capture.ts`
- Rust 기준: `src-tauri/src/session/session_manifest.rs`
- 대표 fixture: `tests/fixtures/contracts/session-manifest-v1.json`
- 소비 surface:
  - booth/operator/authoring React는 host가 반환한 DTO와 shared schema만 소비한다.
  - durable `session.json` 자체는 Rust host만 생성/수정한다.

## 저장 위치

- 세션 루트: `Pictures/dabi_shoot/sessions/{sessionId}/`
- 매니페스트: `Pictures/dabi_shoot/sessions/{sessionId}/session.json`

## 디렉터리 구조

```text
{sessionId}/
  session.json
  captures/
    originals/
  renders/
    previews/
    finals/
  handoff/
  diagnostics/
```

## JSON shape

```json
{
  "schemaVersion": "session-manifest/v1",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "boothAlias": "Kim 4821",
  "customer": {
    "name": "Kim",
    "phoneLastFour": "4821"
  },
  "createdAt": "2026-04-10T01:00:00Z",
  "updatedAt": "2026-04-10T01:00:12Z",
  "lifecycle": {
    "status": "active",
    "stage": "capture-ready"
  },
  "catalogRevision": 7,
  "catalogSnapshot": [
    {
      "presetId": "preset_soft-glow",
      "publishedVersion": "2026.04.10"
    }
  ],
  "activePreset": {
    "presetId": "preset_soft-glow",
    "publishedVersion": "2026.04.10"
  },
  "activePresetId": "preset_soft-glow",
  "activePresetDisplayName": "Soft Glow",
  "activePreviewRendererRoute": {
    "route": "local-renderer-sidecar",
    "routeStage": "canary",
    "fallbackReasonCode": null
  },
  "activePreviewRendererWarmState": {
    "presetId": "preset_soft-glow",
    "publishedVersion": "2026.04.10",
    "state": "warm-ready",
    "observedAt": "2026-04-10T01:00:12Z",
    "diagnosticsDetailPath": "C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/diagnostics/dedicated-renderer/warm-state-preset_soft-glow-2026.04.10.json"
  },
  "timing": {
    "schemaVersion": "session-timing/v1",
    "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
    "adjustedEndAt": "2026-04-10T01:01:00Z",
    "warningAt": "2026-04-10T01:00:30Z",
    "phase": "active",
    "captureAllowed": true,
    "approvedExtensionMinutes": 0,
    "approvedExtensionAuditRef": null,
    "warningTriggeredAt": null,
    "endedTriggeredAt": null
  },
  "captures": [],
  "postEnd": null
}
```

## 필드 규칙

- `schemaVersion`: 현재 baseline은 반드시 `session-manifest/v1`
- `sessionId`: 고객에게 직접 노출하지 않는 opaque durable 식별자
- `boothAlias`: 고객 화면과 handoff copy에서 재사용 가능한 표시용 별칭
- `customer.name`: trim 및 공백 정규화 이후의 이름
- `customer.phoneLastFour`: 숫자 4자리 suffix
- `createdAt`, `updatedAt`: UTC RFC3339 문자열
- `lifecycle.status`: `active`
- `lifecycle.stage`: session progress를 표현하는 host-owned 단계 문자열
- `catalogRevision`, `catalogSnapshot`: 함께 기록되거나 함께 `null`이어야 한다.
- `activePreset`: canonical preset binding
- `activePresetId`: `session-manifest/v1` 호환성을 위한 legacy mirror
- `activePresetDisplayName`: booth/operator copy 정렬용 표시 이름 mirror
- `activePreviewRendererRoute`: active session이 선택한 preview route snapshot. 이후 policy rollback이 생겨도 이미 선택된 세션 의미를 재해석하면 안 된다.
- `activePreviewRendererWarmState`: active session 기준 warm-state evidence snapshot. route snapshot과 별개 additive evidence이며, `presetId +
  publishedVersion + state + observedAt`를 최소 단위로 유지한다.
- `timing`: `session-timing/v1` 스냅샷. current runtime baseline에서는 session start 시점부터 host가 함께 기록한다.
- `captures[*]`는 `session-capture/v1`을 따른다.
- `captures[*].sessionId`, `requestId`, `captureId`: capture correlation baseline
- `captures[*].activePresetId + activePresetVersion`: capture-bound preset identity baseline
- `captures[*].renderStatus`:
  - `previewWaiting`: RAW persistence는 끝났지만 truthful preview close가 아직 안 됨
  - `previewReady`: published bundle의 `xmpTemplatePath + previewProfile`로 만든 preview truth가 닫힘
  - `finalReady`: published bundle의 `xmpTemplatePath + finalProfile`로 만든 final truth가 닫힘
  - `renderFailed`: preview/final truth를 닫지 못한 bounded failure
- `captures[*].timing.fastPreviewVisibleAtMs`: same-capture first-visible lane 시각
- `captures[*].timing.xmpPreviewReadyAtMs`: preset-applied truthful preview close 시각
- dedicated renderer warm-up/submission diagnostics는 같은 session root 아래
  `diagnostics/dedicated-renderer/`에 남고, manifest truth를 직접 덮어쓰지 않는다.
- dedicated renderer result가 `schemaVersion`, typed status, capture correlation, canonical
  preview path 검증을 통과하기 전에는 `previewReady`나 `xmpPreviewReadyAtMs`를 올리면 안 된다.
- dedicated renderer result가 `accepted + canonical output`까지 검증되면 host는 같은 canonical
  preview path를 truthful close owner로 채택할 수 있고, `capture_preview_transition_summary`
  진단 이벤트로 `firstVisibleMs`, `replacementMs`, lane owner, fallback reason, warm state를 함께 남긴다.
- Story 1.19 evidence bundle은 manifest capture record만 단독으로 읽지 않는다. 같은 capture correlation의
  `preview-promotion-evidence.jsonl`, `timing-events.log`, route policy snapshot, published bundle, catalog state를 함께 읽어야 한다.
- `captures[*].preview.assetPath`, `captures[*].final.assetPath`: runtime manifest에서는 현재 세션 root 아래 절대경로만 허용
- `postEnd`는 아래 셋 중 하나 또는 `null`
  - `export-waiting`: `{ state, evaluatedAt }`
  - `completed`: `{ state, evaluatedAt, completionVariant, primaryActionLabel, showBoothAlias, approvedRecipientLabel?, nextLocationLabel?, supportActionLabel? }`
  - `phone-required`: `{ state, evaluatedAt, primaryActionLabel, unsafeActionWarning, showBoothAlias, supportActionLabel? }`

## Canonical / Legacy Guardrail

- 새 구현은 `activePreset`을 canonical field로 사용한다.
- `activePresetId`와 capture-level legacy field는 읽기 호환용으로 유지하되, canonical preset binding과 drift하면 안 된다.
- `session-manifest/v1`은 가볍게 `v2`로 올리지 않는다. Story 1.14의 목적은 schema bump가 아니라 현재 의미를 고정하는 것이다.

## 범위 경계

- Story 1.14가 닫는 범위:
  - session manifest field names, nullability, timing semantics
  - capture correlation / preset binding / post-end variant baseline
  - 문서 + shared schema + Rust serialization alignment
- Story 1.15가 닫는 범위:
  - Canon helper deeper runtime profile
  - publication contract deeper runtime behavior
- Story 1.16이 닫는 범위:
  - build, packaging, release, CI proof baseline

## 변경 규칙

- Rust host만 이 파일을 생성/수정한다.
- React는 host DTO만 소비하고 durable truth를 직접 만들지 않는다.
- fixture 및 테스트는 `tests/fixtures/contracts/session-manifest-v1.json`을 기준 예시로 사용한다.
- post-end `completed`는 `finalReady` 근거 없이 먼저 올라가면 안 된다.
- parity gate에서 허용되는 비교는 same-capture / same-session / same-preset-version 조합뿐이다.
