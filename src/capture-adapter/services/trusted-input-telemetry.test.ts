import { invoke } from '@tauri-apps/api/core'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { TrustedInputReport } from '../../shared-contracts'
import {
  ensureBoothClockCalibration,
  reportTrustedCaptureInput,
  resetBoothClockCalibrationForTests,
  stampTrustedCaptureInput,
} from './trusted-input-telemetry'

vi.mock('../../shared/runtime/is-tauri', () => ({
  isTauriRuntime: () => true,
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

beforeEach(() => {
  resetBoothClockCalibrationForTests()
  vi.clearAllMocks()
})

describe('stampTrustedCaptureInput', () => {
  it('stamps synchronously and rounds the start point down', () => {
    const stamp = stampTrustedCaptureInput({ isTrusted: true }, 'req-1', () => 12.3456)

    expect(stamp).toEqual({
      requestId: 'req-1',
      clientInputMicros: 12_345,
      isTrusted: true,
    })
  })

  it('marks a synthetic event so it cannot count as a qualifying sample', () => {
    expect(
      stampTrustedCaptureInput({ isTrusted: false }, 'req-1', () => 1).isTrusted,
    ).toBe(false)
  })
})

describe('reportTrustedCaptureInput', () => {
  const stamp = { requestId: 'req-1', clientInputMicros: 1_000, isTrusted: true }

  it('adds no IPC to the hot path while the measurement lane is off', async () => {
    const send = vi.fn<(report: TrustedInputReport) => Promise<void>>(async () => undefined)
    const ensureCalibration = vi.fn(async () => null)

    reportTrustedCaptureInput({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      stamp,
      isMeasurementLaneEnabled: false,
      send,
      ensureCalibration,
    })

    await Promise.resolve()

    expect(ensureCalibration).not.toHaveBeenCalled()
    expect(send).not.toHaveBeenCalled()
  })

  it('reports the client stamp with the calibrated offset', async () => {
    const send = vi.fn<(report: TrustedInputReport) => Promise<void>>(async () => undefined)

    reportTrustedCaptureInput({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      stamp,
      isMeasurementLaneEnabled: true,
      send,
      ensureCalibration: async () => ({
        offsetMicros: 500.4,
        uncertaintyMicros: 12.2,
        roundTripMicros: 24.4,
      }),
    })

    await vi.waitFor(() => {
      expect(send).toHaveBeenCalledTimes(1)
    })

    expect(send.mock.calls[0][0]).toEqual({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'req-1',
      clientInputMicros: 1_000,
      clockOffsetMicros: 500,
      // 불확실도는 올림한다. 구간이 좁아 보이면 안 된다.
      clockUncertaintyMicros: 13,
      isTrusted: true,
    })
  })

  it('skips the sample instead of inventing an offset when calibration fails', async () => {
    const send = vi.fn<(report: TrustedInputReport) => Promise<void>>(async () => undefined)

    reportTrustedCaptureInput({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      stamp,
      isMeasurementLaneEnabled: true,
      send,
      ensureCalibration: async () => null,
    })

    await Promise.resolve()
    await Promise.resolve()

    expect(send).not.toHaveBeenCalled()
  })
})

describe('ensureBoothClockCalibration', () => {
  it('invokes the payload-free host clock contract', async () => {
    vi.mocked(invoke).mockResolvedValue({
      hostMonotonicMicros: 5_000_000,
      hostEpochMicros: 1_700_000_000_000_000,
    })

    const calibration = await ensureBoothClockCalibration({ nowMs: () => 0 })

    expect(calibration).not.toBeNull()
    expect(invoke).toHaveBeenCalled()
    for (const call of vi.mocked(invoke).mock.calls) {
      expect(call).toEqual(['stamp_clock_probe'])
    }
  })

  it('calibrates once and reuses the cached result', async () => {
    const probe = vi.fn(async () => ({
      hostMonotonicMicros: 5_000_000,
      hostEpochMicros: 1_700_000_000_000_000,
    }))

    const first = await ensureBoothClockCalibration({ probe, nowMs: () => 0 })
    const callsAfterFirst = probe.mock.calls.length
    const second = await ensureBoothClockCalibration({ probe, nowMs: () => 1_000 })

    expect(first).not.toBeNull()
    expect(second).toBe(first)
    expect(probe.mock.calls.length).toBe(callsAfterFirst)
  })

  it('returns null when the host never answers', async () => {
    const calibration = await ensureBoothClockCalibration({
      probe: async () => {
        throw new Error('host unavailable')
      },
      nowMs: () => 0,
    })

    expect(calibration).toBeNull()
  })
})
