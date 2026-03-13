import { describe, expect, it } from 'vitest'

import { createSessionTimingState } from './shootEndCalculator.js'

describe('createSessionTimingState', () => {
  it('calculates a standard session as reservationStartAt plus 50 minutes', () => {
    expect(
      createSessionTimingState({
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        sessionType: 'standard',
        updatedAt: '2026-03-08T09:07:00.000Z',
      }),
    ).toEqual({
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      actualShootEndAt: '2026-03-08T09:50:00.000Z',
      sessionType: 'standard',
      operatorExtensionCount: 0,
      lastTimingUpdateAt: '2026-03-08T09:07:00.000Z',
    })
  })

  it('anchors late booth check-in to reservationStartAt instead of the current clock', () => {
    expect(
      createSessionTimingState({
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        sessionType: 'standard',
        updatedAt: '2026-03-08T09:23:00.000Z',
      }).actualShootEndAt,
    ).toBe('2026-03-08T09:50:00.000Z')
  })

  it('calculates coupon-extended sessions as reservationStartAt plus 100 minutes, not generic :50 logic', () => {
    expect(
      createSessionTimingState({
        reservationStartAt: '2026-03-08T09:15:00.000Z',
        sessionType: 'couponExtended',
        updatedAt: '2026-03-08T09:20:00.000Z',
      }).actualShootEndAt,
    ).toBe('2026-03-08T10:55:00.000Z')
  })
})
