# Local Dedicated Renderer 계약

## 목적

이 문서는 Boothy의 다음 preview architecture로 승인된 `local dedicated renderer`의 최소 계약을 고정한다.

## Ownership

- Tauri host가 dedicated renderer lifecycle을 직접 소유한다.
- dedicated renderer는 preset-applied truthful close owner다.
- first-visible lane은 별도 customer-visible projection일 수 있지만 `previewReady` truth owner가 아니다.

## 최소 입력

- `sessionId`
- `captureId`
- `requestId`
- `presetId`
- `publishedVersion`
- `rawPath`
- `xmpTemplatePath`
- `previewProfile`

## 최소 동작

- `warmRenderer`
  - preset preload, cache priming, warm-state 확보
- `submitPreviewJob`
  - capture-bound preset artifact로 same-capture preview close 시도
- `previewReady`
  - canonical preview path에 실제 preset-applied artifact가 생성된 뒤에만 발생
- `queueSaturated`
  - host가 truthful fallback을 결정할 수 있게 전달
- `fallbackSuggested`
  - current booth path로 안전 강등이 필요함을 전달

## Booth-Safe Rules

- booth는 first-visible image가 먼저 보여도 `previewReady`를 조기 승격하면 안 된다.
- dedicated renderer는 same-capture, same-session, capture-bound preset version을 유지해야 한다.
- same-path replacement는 기존 canonical preview를 먼저 잃지 않는 방식으로 수행돼야 한다.
- queue saturation, warm-state loss, invalid output 시 booth는 `Preview Waiting`과 approved fallback path를 유지해야 한다.

## Validation Gate

- `original visible -> preset-applied visible <= 2.5s` 목표
- wrong-capture `0`
- preset mismatch `0`
- preview/final truth drift `0`
