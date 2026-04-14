# Local Dedicated Renderer 계약

## 목적

이 문서는 Boothy의 승인된 preview architecture인 `local dedicated renderer`의 최소 계약을 고정한다.

## 최소 입력

- `sessionId`
- `captureId`
- `requestId`
- `presetId`
- `publishedVersion`
- `rawPath`
- `xmpTemplatePath`
- `previewProfile`

## 해석 규칙

- `previewProfile`은 bundle top-level legacy field가 아니라 `canonicalRecipe.previewIntent`에서 유도된다.
- `xmpTemplatePath`는 `darktableAdapter.xmpTemplatePath`에서 유도된다.
- dedicated renderer는 same-capture, same-session, capture-bound preset version을 유지해야 한다.
- dedicated renderer result는 additive warm-state evidence(`warmState`, `warmStateDetailPath`)를 함께 남길 수 있지만, session truth 자체를 직접 소유하면 안 된다.

## Booth-Safe Rules

- booth는 first-visible image가 먼저 보여도 `previewReady`를 조기 승격하면 안 된다.
- same-path replacement는 기존 canonical preview를 먼저 잃지 않는 방식으로 수행돼야 한다.
- queue saturation, warm-state loss, invalid output 시 booth는 `Preview Waiting`과 approved fallback path를 유지해야 한다.
- operator/diagnostics용 warm-state vocabulary는 `warm-ready`, `warm-hit`, `cold`, `warm-state-lost` 또는 동등한 typed 값으로 제한한다.

## Promotion Evidence Contract

- dedicated renderer는 `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`에 capture-bound evidence record를 남긴다.
- 각 record는 최소 `sessionId`, `requestId`, `captureId`, `presetId`, `publishedVersion`, `laneOwner`, `fallbackReasonCode`, `routeStage`, `warmState`,
  `firstVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`를 포함해야 한다.
- 같은 record에는 `session.json`, `timing-events.log`, `branch-config/preview-renderer-policy.json`, published `bundle.json`,
  `preset-catalog/catalog-state.json`, candidate preview asset 경로가 함께 남아 evidence bundle assemble 입력으로 바로 재사용될 수 있어야 한다.
- assemble된 evidence bundle은 같은 session/preset/version family 안에서 fallback 발생 비율(`fallbackRatio`)을 계산해 release-close sign-off가
  속도뿐 아니라 fallback 안정성까지 함께 읽을 수 있어야 한다.
- parity diff는 same-capture / same-session / same-preset-version 비교에만 사용할 수 있다. 이 전제가 깨진 비교는 promotion 근거가 아니다.
