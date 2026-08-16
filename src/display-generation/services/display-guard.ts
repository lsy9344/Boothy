import {
  DISPLAY_TIER_ORDER,
  type DisplayGeneration,
  type DisplayRejectReason,
} from '../../shared-contracts'
import { isDisplayFitSource } from '../../viewer-surface/services/display-fit'

/**
 * 표시 전진 판정 문맥. 전부 host가 소유한 truth에서 온다.
 */
export type DisplayGuardContext = {
  /** host가 고정한 현재 세션. */
  sessionId: string | null
  /** 현재 viewer window 세대. */
  viewerEpoch: number
  /** Story 7.1의 photoRect에서 파생한 필요 소스 픽셀. 0이면 아직 측정되지 않았다. */
  requiredSourceWidthPx: number
  requiredSourceHeightPx: number
}

export type DisplayGuardOptions = {
  /** 픽셀 decode에 실패한 generation. 다시 활성화하지 않는다. */
  poisonedGenerationIds?: ReadonlySet<string>
}

export type DisplayGuardVerdict =
  | { advance: true }
  | { advance: false; reason: DisplayRejectReason }

function tierOrder(tier: string): number | null {
  return Object.hasOwn(DISPLAY_TIER_ORDER, tier)
    ? DISPLAY_TIER_ORDER[tier as keyof typeof DISPLAY_TIER_ORDER]
    : null
}

/**
 * 다음 generation이 현재 화면을 대체할 수 있는지 판정한다.
 *
 * **여기가 AC 2의 정확성 핵심이다.** 늦게 도착하거나 중복된 업데이트가 화면을 되돌리지
 * 못하게 하는 보장은 전송 순서가 아니라 이 순수 함수에서 나온다.
 *
 * 판정은 전부 AND이며, 실패는 항상 고유한 이유를 돌려준다.
 *
 * **`refined-tier-not-justified`는 여기서 판정되지 않는다.** AC 6의 detail 축 gate는 측정된
 * 제품 결정이고 host의 게시 경계에서만 평가된다 — generation에는 그 판정이 실려 있지 않다.
 * viewer는 그 gate를 통과한 generation만 보게 되므로 결과적으로 같은 truth를 따른다.
 * (`present-unreported`가 host 전용인 것과 같은, 문서화된 비대칭이다.)
 */
export function shouldAdvanceDisplay(
  current: DisplayGeneration | null,
  next: DisplayGeneration,
  context: DisplayGuardContext,
  options: DisplayGuardOptions = {},
): DisplayGuardVerdict {
  if (options.poisonedGenerationIds?.has(next.generationId)) {
    return { advance: false, reason: 'decode-failed' }
  }

  if (context.sessionId === null || next.sessionId !== context.sessionId) {
    return { advance: false, reason: 'session-mismatch' }
  }

  if (next.viewerEpoch !== context.viewerEpoch) {
    return { advance: false, reason: 'stale-epoch' }
  }

  const nextTierOrder = tierOrder(next.tier)

  if (nextTierOrder === null) {
    return { advance: false, reason: 'unknown-generation' }
  }

  if (
    context.requiredSourceWidthPx <= 0 ||
    context.requiredSourceHeightPx <= 0
  ) {
    return { advance: false, reason: 'viewer-not-ready' }
  }

  // contain 경계에 어느 축도 닿지 못하면 브라우저가 확대해야 한다.
  if (!isDisplayFitSource(
    { naturalWidth: next.sourceWidthPx, naturalHeight: next.sourceHeightPx },
    context,
  )) {
    return { advance: false, reason: 'insufficient-dimensions' }
  }

  if (current === null) {
    // Story 7.6 / UX-DR19. 정밀본은 **승급**이지 첫 성공 화면의 대체가 아니다.
    // proxy 없이 정밀본만 뜨면 고객의 첫 화면이 3배 느려지고, 맞춰야 할 geometry도 없다.
    if (next.tier === 'rawRefinedDisplay') {
      return { advance: false, reason: 'refined-dimension-mismatch' }
    }

    return { advance: true }
  }

  const currentTierOrder = tierOrder(current.tier) ?? 0

  // `?? null`을 쓰는 이유: 이 함수는 zod를 통과하지 않은 값도 받을 수 있고(스냅샷 캐시,
  // 이전 빌드가 남긴 v1 generation), 그때 `proxyProvenance`는 `null`이 아니라 `undefined`다.
  // `!== null`만 보면 그 경로에서 런타임 예외가 나고 **화면 갱신이 통째로 멈춘다.**
  const currentPreset = current.proxyProvenance ?? null
  const nextPreset = next.proxyProvenance ?? null
  const currentCaptureId = current.captureId ?? null
  const nextCaptureId = next.captureId ?? null
  const currentCaptureOrder = current.captureOrder ?? null
  const nextCaptureOrder = next.captureOrder ?? null

  // **tier 비교는 같은 촬영 안에서만 의미가 있다.**
  //
  // Story 7.6이 `rawRefinedDisplay`(2)를 추가하면서 이 구분이 필수가 됐다. 촬영 A가 정밀본까지
  // 올라간 뒤 촬영 B의 proxy(1)가 도착하면, tier만 비교할 경우 **고객의 다음 사진이 통째로
  // 거부된다.** 다른 촬영의 낮은 tier는 하락이 아니라 새 사진이며, 순서는 아래의
  // capture/request 좌표가 판정한다.
  const isSameCapture =
    currentCaptureId === null ||
    nextCaptureId === null ||
    currentCaptureId === nextCaptureId

  if (isSameCapture && nextTierOrder < currentTierOrder) {
    return { advance: false, reason: 'tier-downgrade' }
  }

  // 한 촬영에는 capture-bound preset이 정확히 하나다. 같은 사진의 룩이 화면에서 바뀌면
  // 고객은 자기가 고른 것과 다른 결과를 보게 된다. host도 같은 판정을 하지만,
  // 한쪽에만 두면 pointer와 화면이 갈라진다.
  if (
    currentPreset !== null &&
    nextPreset !== null &&
    currentCaptureId !== null &&
    currentCaptureId === nextCaptureId &&
    (currentPreset.presetId !== nextPreset.presetId ||
      currentPreset.presetVersion !== nextPreset.presetVersion)
  ) {
    return { advance: false, reason: 'preset-mismatch' }
  }

  // 다른 촬영의 generation이 늦게 도착했다. proxy 렌더는 수 초가 걸리므로
  // **완료 순서가 촬영 순서와 어긋난다.** request보다 capture가 더 정확한 단위이므로 먼저 본다.
  const olderCaptureByOrder =
    currentCaptureOrder !== null &&
    nextCaptureOrder !== null &&
    nextCaptureOrder < currentCaptureOrder
  const olderCaptureByLegacyClock =
    (currentCaptureOrder === null || nextCaptureOrder === null) &&
    currentCaptureId !== null &&
    nextCaptureId !== null &&
    currentCaptureId !== nextCaptureId &&
    next.committedAtHostMicros < current.committedAtHostMicros

  if (olderCaptureByOrder || olderCaptureByLegacyClock) {
    return { advance: false, reason: 'older-capture' }
  }

  // 오래된 request가 더 높은 seq를 들고 늦게 도착할 수 있다. seq만으로는 못 막는다.
  // host가 commit 시각을 단조 증가하는 host clock으로 찍으므로 그 값으로 request 순서를 판정한다.
  const currentRequestOrder = current.requestOrder ?? null
  const nextRequestOrder = next.requestOrder ?? null

  const olderRequestByOrder =
    current.requestId !== next.requestId &&
    currentRequestOrder !== null &&
    nextRequestOrder !== null &&
    nextRequestOrder < currentRequestOrder
  const olderRequestByLegacyClock =
    current.requestId !== next.requestId &&
    (currentRequestOrder === null || nextRequestOrder === null) &&
    next.committedAtHostMicros < current.committedAtHostMicros

  if (olderRequestByOrder || olderRequestByLegacyClock) {
    return { advance: false, reason: 'older-request' }
  }

  // Story 7.6. **정밀본은 활성 proxy와 픽셀 크기가 정확히 같을 때만 승급한다.**
  //
  // 이것이 AC 4의 crop/scale 점프 0을 만드는 기계적 장치다. 크기가 한 픽셀이라도 다르면
  // `object-fit: contain` 박스가 달라져 교체 순간 사진이 튄다. 같은 촬영이 아니면
  // 맞출 proxy 자체가 화면에 없다는 뜻이므로 같은 사유로 거부한다 — 순서 문제는 위에서
  // 이미 걸러졌으므로 여기 남는 것은 "붙일 곳이 없는 정밀본"뿐이다.
  if (next.tier === 'rawRefinedDisplay') {
    const geometryMatches =
      currentCaptureId !== null &&
      currentCaptureId === nextCaptureId &&
      current.sourceWidthPx === next.sourceWidthPx &&
      current.sourceHeightPx === next.sourceHeightPx

    if (!geometryMatches) {
      return { advance: false, reason: 'refined-dimension-mismatch' }
    }
  }

  if (next.generationSeq <= current.generationSeq) {
    return { advance: false, reason: 'lower-generation' }
  }

  return { advance: true }
}
