import { describe, expect, it } from 'vitest'

import { applyOperatorSessionExtension } from './extensionPolicy.js'

const baseTimingState = {
  reservationStartAt: '2026-03-08T09:00:00.000Z',
  actualShootEndAt: '2026-03-08T09:50:00.000Z',
  sessionType: 'standard' as const,
  operatorExtensionCount: 0,
  lastTimingUpdateAt: '2026-03-08T09:07:00.000Z',
}

describe('applyOperatorSessionExtension', () => {
  it('adds one hour from the current stored actualShootEndAt', () => {
    expect(
      applyOperatorSessionExtension(baseTimingState, {
        updatedAt: '2026-03-08T09:30:00.000Z',
      }),
    ).toEqual({
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      actualShootEndAt: '2026-03-08T10:50:00.000Z',
      sessionType: 'standard',
      operatorExtensionCount: 1,
      lastTimingUpdateAt: '2026-03-08T09:30:00.000Z',
    })
  })

  it('treats repeated extensions as cumulative and deterministic', () => {
    const onceExtended = applyOperatorSessionExtension(baseTimingState, {
      updatedAt: '2026-03-08T09:30:00.000Z',
    })

    expect(
      applyOperatorSessionExtension(onceExtended, {
        updatedAt: '2026-03-08T10:10:00.000Z',
      }),
    ).toEqual({
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      actualShootEndAt: '2026-03-08T11:50:00.000Z',
      sessionType: 'standard',
      operatorExtensionCount: 2,
      lastTimingUpdateAt: '2026-03-08T10:10:00.000Z',
    })
  })
})
