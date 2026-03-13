import { describe, expect, it } from 'vitest'

import { deriveTimingThresholds, selectCustomerFacingSessionEndTime, selectTimingAlertStatus } from './timingSelectors.js'

const baseTimingState = {
  reservationStartAt: '2026-03-08T09:00:00.000Z',
  actualShootEndAt: '2026-03-08T09:50:00.000Z',
  sessionType: 'standard' as const,
  operatorExtensionCount: 0,
  lastTimingUpdateAt: '2026-03-08T09:07:00.000Z',
}

describe('timingSelectors', () => {
  it('exposes the stored end time without recalculating it in the component tree', () => {
    expect(selectCustomerFacingSessionEndTime(baseTimingState)).toBe('2026-03-08T09:50:00.000Z')
  })

  it('recomputes warning and escalation thresholds from the latest stored end time after an extension update', () => {
    expect(
      deriveTimingThresholds(baseTimingState, {
        warningLeadMinutes: 5,
        phoneEscalationDelayMinutes: 2,
      }),
    ).toEqual({
      warningAt: '2026-03-08T09:45:00.000Z',
      shootStopAt: '2026-03-08T09:50:00.000Z',
      phoneEscalationAt: '2026-03-08T09:52:00.000Z',
    })

    expect(
      deriveTimingThresholds(
        {
          ...baseTimingState,
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          operatorExtensionCount: 1,
          lastTimingUpdateAt: '2026-03-08T09:30:00.000Z',
        },
        {
          warningLeadMinutes: 5,
          phoneEscalationDelayMinutes: 2,
        },
      ),
    ).toEqual({
      warningAt: '2026-03-08T10:45:00.000Z',
      shootStopAt: '2026-03-08T10:50:00.000Z',
      phoneEscalationAt: '2026-03-08T10:52:00.000Z',
    })
  })

  it('classifies the alert window from the authoritative end time without inventing a second timing model', () => {
    const options = {
      warningLeadMinutes: 5,
      phoneEscalationDelayMinutes: 2,
    }

    expect(selectTimingAlertStatus(baseTimingState, options, Date.parse('2026-03-08T09:44:59.000Z'))).toBe('none')
    expect(selectTimingAlertStatus(baseTimingState, options, Date.parse('2026-03-08T09:45:00.000Z'))).toBe('warning')
    expect(selectTimingAlertStatus(baseTimingState, options, Date.parse('2026-03-08T09:49:59.000Z'))).toBe('warning')
    expect(selectTimingAlertStatus(baseTimingState, options, Date.parse('2026-03-08T09:50:00.000Z'))).toBe('ended')
  })
})
