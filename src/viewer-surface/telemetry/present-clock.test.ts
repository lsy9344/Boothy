import { describe, expect, it, vi } from 'vitest'

import {
  PRESENT_UNCERTAINTY_BUDGET_MICROS,
  calibrateClock,
  ceilMicros,
  classifyConfidence,
  conservativeLatencyMicros,
  estimateCalibration,
  floorMicros,
  toIntegerClockFields,
} from './present-clock'

describe('toIntegerClockFields', () => {
  it('rounds offset and uncertainty into the integer IPC contract', () => {
    expect(
      toIntegerClockFields({
        offsetMicros: 799_500.5,
        uncertaintyMicros: 12.2,
        roundTripMicros: 24.4,
      }),
    ).toEqual({
      clockOffsetMicros: 799_501,
      clockUncertaintyMicros: 13,
    })
  })
})

describe('estimateCalibration', () => {
  it('returns null without samples', () => {
    expect(estimateCalibration([])).toBeNull()
  })

  it('picks the sample with the smallest round trip', () => {
    const calibration = estimateCalibration([
      // rtt 10_000us, midpoint 105_000 → offset 895_000
      {
        clientSentMicros: 100_000,
        clientReceivedMicros: 110_000,
        hostMonotonicMicros: 1_000_000,
      },
      // rtt 1_000us, midpoint 200_500 → offset 799_500
      {
        clientSentMicros: 200_000,
        clientReceivedMicros: 201_000,
        hostMonotonicMicros: 1_000_000,
      },
      {
        clientSentMicros: 300_000,
        clientReceivedMicros: 340_000,
        hostMonotonicMicros: 1_000_000,
      },
    ])

    expect(calibration).toEqual({
      offsetMicros: 799_500,
      uncertaintyMicros: 500,
      roundTripMicros: 1_000,
    })
  })

  it('ignores samples whose round trip is negative', () => {
    const calibration = estimateCalibration([
      {
        clientSentMicros: 500,
        clientReceivedMicros: 400,
        hostMonotonicMicros: 1_000,
      },
      {
        clientSentMicros: 1_000,
        clientReceivedMicros: 1_200,
        hostMonotonicMicros: 5_000,
      },
    ])

    expect(calibration?.roundTripMicros).toBe(200)
  })
})

describe('conservativeLatencyMicros', () => {
  it('always widens the interval so measurement error cannot look faster', () => {
    expect(
      conservativeLatencyMicros({
        inputHostMicros: 1_000_000,
        presentHostMicros: 2_000_000,
        inputUncertaintyMicros: 300,
        presentUncertaintyMicros: 700,
      }),
    ).toBe(1_001_000)
  })

  it('never reports a shorter interval than the raw span', () => {
    const raw = 2_400_000

    for (const inputUncertaintyMicros of [0, 1, 500, 5_000]) {
      for (const presentUncertaintyMicros of [0, 1, 500, 5_000]) {
        expect(
          conservativeLatencyMicros({
            inputHostMicros: 1_000_000,
            presentHostMicros: 3_400_000,
            inputUncertaintyMicros,
            presentUncertaintyMicros,
          }),
        ).toBeGreaterThanOrEqual(raw)
      }
    }
  })

  it('returns null when either endpoint is missing', () => {
    expect(
      conservativeLatencyMicros({
        inputHostMicros: null,
        presentHostMicros: 2_000_000,
        inputUncertaintyMicros: 0,
        presentUncertaintyMicros: 0,
      }),
    ).toBeNull()
    expect(
      conservativeLatencyMicros({
        inputHostMicros: 1_000_000,
        presentHostMicros: null,
        inputUncertaintyMicros: 0,
        presentUncertaintyMicros: 0,
      }),
    ).toBeNull()
  })
})

describe('classifyConfidence', () => {
  it('flags samples beyond the uncertainty budget instead of dropping them', () => {
    expect(classifyConfidence(0)).toBe('measured')
    expect(classifyConfidence(PRESENT_UNCERTAINTY_BUDGET_MICROS)).toBe(
      'measured',
    )
    expect(classifyConfidence(PRESENT_UNCERTAINTY_BUDGET_MICROS + 1)).toBe(
      'low-confidence',
    )
  })
})

describe('millisecond conversion', () => {
  it('rounds the start point down and the end point up', () => {
    // 어떤 반올림도 구간을 짧게 만들면 안 된다.
    expect(floorMicros(1.2345)).toBe(1_234)
    expect(ceilMicros(1.2345)).toBe(1_235)
    expect(ceilMicros(floorMicros(2) / 1000)).toBe(2_000)
  })
})

describe('calibrateClock', () => {
  it('runs the requested rounds and keeps the fastest round trip', async () => {
    let clientClock = 1_000
    const now = vi.fn(() => {
      clientClock += 1
      return clientClock
    })
    const probe = vi.fn(async () => ({
      hostMonotonicMicros: 9_000_000,
      hostEpochMicros: 1_700_000_000_000_000,
    }))

    const calibration = await calibrateClock({ probe, now, rounds: 3 })

    expect(probe).toHaveBeenCalledTimes(3)
    expect(calibration).not.toBeNull()
    expect(calibration!.uncertaintyMicros).toBeGreaterThanOrEqual(0)
  })

  it('returns null when every probe fails rather than inventing an offset', async () => {
    const calibration = await calibrateClock({
      probe: async () => {
        throw new Error('host unavailable')
      },
      now: () => 1,
      rounds: 2,
    })

    expect(calibration).toBeNull()
  })
})
