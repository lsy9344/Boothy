# `session.json` v1 계약

## 목적

Story 1.2에서 고객 세션 시작 직후 생성되는 durable manifest의 최소 기준선을 고정한다.

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
  "createdAt": "2026-03-20T00:00:00Z",
  "updatedAt": "2026-03-20T00:00:00Z",
  "lifecycle": {
    "status": "active",
    "stage": "session-started"
  },
  "catalogRevision": null,
  "catalogSnapshot": null,
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
- `catalogRevision`, `catalogSnapshot`: 현재 세션이 처음 booth catalog를 읽거나 preset을 고를 때 고정되는
  future-session-safe catalog baseline. 새 live catalog change가 있더라도 이미 진행 중인 세션은 이
  snapshot 안의 버전만 본다.
- `activePreset`: Story 1.3부터 `{ presetId, publishedVersion }` binding 또는 `null`
- `activePresetId`: `session-manifest/v1` 호환성을 위한 legacy mirror. Story 1.2에서는 `null`
- `captures`: Story 1.2에서는 빈 배열
- `postEnd`: post-end truth가 아직 확정되지 않았으면 `null`, 확정되면 아래 셋 중 하나
  - `export-waiting`: `{ state, evaluatedAt }`
  - `completed`: `{ state, evaluatedAt, completionVariant, primaryActionLabel, showBoothAlias, approvedRecipientLabel? , nextLocationLabel?, supportActionLabel? }`
  - `phone-required`: `{ state, evaluatedAt, primaryActionLabel, unsafeActionWarning, showBoothAlias, supportActionLabel? }`
- `captures[*].preview.assetPath`, `captures[*].final.assetPath`: runtime manifest에서는 반드시 OS가 제공한 사용자 Pictures 경로 아래의 현재 세션 루트(`.../Pictures/dabi_shoot/sessions/{sessionId}/`)를 가리켜야 한다. 프런트엔드 session guard는 절대경로 안의 `pictures/dabi_shoot/sessions/{sessionId}/` anchor를 기준으로 현재 세션 자산 여부를 판정한다. `fixtures/...` 같은 상대경로는 Vitest/unit test fixture에서만 허용한다.
- `captures[*].renderStatus`:
  - `previewWaiting`: RAW persistence는 끝났지만 preset-applied preview render가 아직 닫히지 않음
  - `previewReady`: published bundle의 `xmpTemplatePath + previewProfile`로 만든 실제 preview file이 존재함
  - `finalReady`: published bundle의 `xmpTemplatePath + finalProfile`로 만든 실제 final file이 존재함
  - `renderFailed`: preview 또는 final render truth를 닫지 못해 bounded failure 상태로 잠김
- RAW copy, placeholder SVG, bundle 대표 preview tile은 `previewReady`나 `finalReady`의 근거가 될 수 없다.

## 변경 규칙

- Rust host만 이 파일을 생성/수정한다.
- React는 host가 반환한 DTO만 소비하고 durable truth를 직접 만들지 않는다.
- 후속 스토리는 기존 필드를 유지한 채 확장한다.
- 새 구현은 `activePreset`을 canonical field로 사용하고, `activePresetId`는 구버전 호환을 위해 함께 유지한다.
- `catalogSnapshot`은 customer-visible top 6 preset만 고정하고, rollback이나 publish 이후에도 기존
  active session manifest를 다시 쓰지 않는다.
- post-end `completed`는 `finalReady`가 없는 capture에서 먼저 올라가면 안 된다.
