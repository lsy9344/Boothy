# Local Dedicated Renderer 계약

## 목적

이 문서는 Boothy의 승인된 preview architecture인 `local dedicated renderer`의 최소 계약을 고정한다.
Story 1.23은 이 경계를 local full-screen lane의 `prototype owner`로만 소유한다. Story 1.24는 prototype-track `canary` proof를, Story 1.25는 prototype-track `default/rollback` authority를, Story 1.27은 corrective follow-up proof를 소유한다. Stories 1.28 through 1.31은 actual primary lane 구현과 재검증을 소유하며, Story 1.13은 그 이후의 최종 release close를 소유하므로 이 문서는 그 ownership 분리를 유지해야 한다.

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
- Story 1.23에서 local lane output은 customer-visible `display-sized preset-applied truthful artifact` 후보를 만드는 prototype path다. darktable-compatible path는 계속 `parity/fallback/final reference`로 남아야 한다.

## Booth-Safe Rules

- booth는 first-visible image가 먼저 보여도 `previewReady`를 조기 승격하면 안 된다.
- same-path replacement는 기존 canonical preview를 먼저 잃지 않는 방식으로 수행돼야 한다.
- queue saturation, warm-state loss, invalid output 시 booth는 `Preview Waiting`과 approved fallback path를 유지해야 한다.
- operator/diagnostics용 warm-state vocabulary는 `warm-ready`, `warm-hit`, `cold`, `warm-state-lost` 또는 동등한 typed 값으로 제한한다.

## Promotion Evidence Contract

- dedicated renderer는 `diagnostics/dedicated-renderer/preview-promotion-evidence.jsonl`에 capture-bound evidence record를 남긴다.
- 각 record는 최소 `sessionId`, `requestId`, `captureId`, `presetId`, `publishedVersion`, `laneOwner`, `fallbackReasonCode`, `routeStage`, `warmState`,
  `implementationTrack`, `captureRequestedAtMs`, `rawPersistedAtMs`, `truthfulArtifactReadyAtMs`, `visibleOwner`, `visibleOwnerTransitionAtMs`,
  `firstVisibleMs`, `sameCaptureFullScreenVisibleMs`, `replacementMs`, `originalVisibleToPresetAppliedVisibleMs`를 포함해야 한다.
- `sameCaptureFullScreenVisibleMs`는 primary close KPI이고, `replacementMs`는 legacy comparison metric이다. 두 값을 같은 의미로 재해석하거나
  항상 같다고 강제하면 안 된다.
- Story 1.28의 additive migration 동안 `laneOwner`는 backward compatibility를 위해 유지할 수 있지만, `implementationTrack=actual-primary-lane`이 없는 record는 comparison/audit input으로만 남고 actual lane success/promotion 근거로 읽으면 안 된다.
- 같은 record에는 `session.json`, selected-capture `timing-events.log`, `diagnostics/dedicated-renderer/captured-preview-renderer-policy-<captureId>.json`,
  published `bundle.json`, `diagnostics/dedicated-renderer/captured-catalog-state-<captureId>.json`, candidate preview asset 경로가 함께 남아 evidence bundle assemble 입력으로 바로 재사용될 수 있어야 한다.
- assemble된 evidence bundle은 같은 session/preset/version family 안에서 fallback 발생 비율(`fallbackRatio`)을 계산해 release-close sign-off가
  속도뿐 아니라 fallback 안정성까지 함께 읽을 수 있어야 한다.
- parity diff는 same-capture / same-session / same-preset-version 비교에만 사용할 수 있다. 이 전제가 깨진 비교는 promotion 근거가 아니다.
- 문서와 증거 해석은 Story 1.23 prototype owner, Stories 1.24 through 1.27 prototype/gate history, Stories 1.28 through 1.31 actual-lane implementation/gate ownership, Story 1.13 final close owner를 구분해야 한다.
