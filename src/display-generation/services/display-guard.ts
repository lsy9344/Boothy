import {
  DISPLAY_TIER_ORDER,
  type DisplayGeneration,
  type DisplayRejectReason,
} from '../../shared-contracts'

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

  // upscale은 0이어야 한다. 고정 384px thumbnail은 승인된 어떤 profile도 만족하지 못한다.
  if (
    next.sourceWidthPx < context.requiredSourceWidthPx ||
    next.sourceHeightPx < context.requiredSourceHeightPx
  ) {
    return { advance: false, reason: 'insufficient-dimensions' }
  }

  if (current === null) {
    return { advance: true }
  }

  const currentTierOrder = tierOrder(current.tier) ?? 0

  if (nextTierOrder < currentTierOrder) {
    return { advance: false, reason: 'tier-downgrade' }
  }

  // 오래된 request가 더 높은 seq를 들고 늦게 도착할 수 있다. seq만으로는 못 막는다.
  // host가 commit 시각을 단조 증가하는 host clock으로 찍으므로 그 값으로 request 순서를 판정한다.
  if (
    current.requestId !== next.requestId &&
    next.committedAtHostMicros < current.committedAtHostMicros
  ) {
    return { advance: false, reason: 'older-request' }
  }

  if (next.generationSeq <= current.generationSeq) {
    return { advance: false, reason: 'lower-generation' }
  }

  return { advance: true }
}
