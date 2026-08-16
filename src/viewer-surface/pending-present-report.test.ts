import { describe, expect, it } from 'vitest'

import type { DisplayPresentReport } from '../shared-contracts'
import { enqueuePendingPresentReport } from './pending-present-report'

function buildReport(generationId: string): DisplayPresentReport {
  return {
    generationId,
    viewerEpoch: 3,
    outcome: 'presented',
    rejectReason: null,
    naturalWidthPx: 3840,
    naturalHeightPx: 2560,
    spans: {
      viewerReceiptAtMicros: 1_000,
      decodeStartAtMicros: 1_100,
      decodeEndAtMicros: 1_200,
      swapCommittedAtMicros: 1_300,
      imgOnLoadAtMicros: 1_150,
      actualPresentAtMicros: 1_400,
      elementTimingRenderAtMicros: null,
      isElementRenderTime: false,
    },
    clockOffsetMicros: 0,
    clockUncertaintyMicros: 100,
  }
}

describe('enqueuePendingPresentReport', () => {
  it('retains every distinct report until calibration becomes available', () => {
    const pending = Array.from({ length: 20 }, (_, index) => ({
      report: buildReport(`generation-${index}`),
      sessionId: 'session-a',
    }))

    const result = enqueuePendingPresentReport(pending, {
      report: buildReport('generation-20'),
      sessionId: 'session-a',
    })

    expect(result).toHaveLength(21)
    expect(result[0]?.report.generationId).toBe('generation-0')
  })

  it('replaces only a duplicate generation outcome', () => {
    const first = buildReport('generation-1')
    const replacement = {
      ...first,
      spans: { ...first.spans, actualPresentAtMicros: 9_999 },
    }

    const result = enqueuePendingPresentReport(
      [{ report: first, sessionId: 'old-session' }],
      { report: replacement, sessionId: 'new-session' },
    )

    expect(result).toEqual([
      { report: replacement, sessionId: 'new-session' },
    ])
  })
})
