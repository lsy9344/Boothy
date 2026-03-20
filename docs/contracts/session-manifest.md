# `session.json` v1 계약

## 목적

Story 1.2에서 고객 세션 시작 직후 생성되는 durable manifest의 최소 기준선을 고정한다.

## 저장 위치

- 세션 루트: `appLocalData/booth-runtime/sessions/{sessionId}/`
- 매니페스트: `appLocalData/booth-runtime/sessions/{sessionId}/session.json`

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
  "createdAt": "2026-03-20T00:00:00Z",
  "updatedAt": "2026-03-20T00:00:00Z",
  "lifecycle": {
    "status": "active",
    "stage": "session-started"
  },
  "activePreset": null,
  "activePresetId": null,
  "captures": [],
  "postEnd": null
}
```

## 필드 규칙

- `schemaVersion`: 현재 baseline은 반드시 `session-manifest/v1`
- `sessionId`: 고객에게 노출하지 않는 opaque durable 식별자
- `boothAlias`: 고객 화면에서 재사용 가능한 표시용 별칭
- `customer.name`: trim 및 공백 정규화 이후의 이름
- `customer.phoneLastFour`: 숫자 4자리 suffix
- `createdAt`, `updatedAt`: UTC RFC3339 문자열
- `lifecycle.status`: 초기값은 `active`
- `lifecycle.stage`: 초기값은 `session-started`
- `activePreset`: Story 1.3부터 `{ presetId, publishedVersion }` binding 또는 `null`
- `activePresetId`: `session-manifest/v1` 호환성을 위한 legacy mirror. Story 1.2에서는 `null`
- `captures`: Story 1.2에서는 빈 배열
- `postEnd`: Story 1.2에서는 `null`

## 변경 규칙

- Rust host만 이 파일을 생성/수정한다.
- React는 host가 반환한 DTO만 소비하고 durable truth를 직접 만들지 않는다.
- 후속 스토리는 기존 필드를 유지한 채 확장한다.
- 새 구현은 `activePreset`을 canonical field로 사용하고, `activePresetId`는 구버전 호환을 위해 함께 유지한다.
