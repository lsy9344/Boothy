import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { DisplayPointerSnapshot } from '../../shared-contracts'
import { viewerDisplaySchemaVersion } from '../../shared-contracts'
import type { DisplayHostService } from '../../viewer-surface/services/viewer-host-adapter'
import {
  MEASUREMENT_LANE_RETRY_INTERVAL_MS,
  useMeasurementLaneState,
} from './use-measurement-lane-state'

function buildPointer(
  presentTelemetryEnabled: boolean,
  overrides: Partial<DisplayPointerSnapshot> = {},
): DisplayPointerSnapshot {
  return {
    schemaVersion: viewerDisplaySchemaVersion,
    sessionId: null,
    revision: 0,
    activeGeneration: null,
    requiredSourceWidthPx: 0,
    requiredSourceHeightPx: 0,
    measurementLaneEnabled: false,
    presentTelemetryEnabled,
    observedAtHostMicros: 0,
    ...overrides,
  }
}

function buildService(input: {
  getViewerDisplayState: DisplayHostService['getViewerDisplayState']
  subscribeToDisplayUpdates?: DisplayHostService['subscribeToDisplayUpdates']
}): DisplayHostService {
  return {
    getViewerDisplayState: input.getViewerDisplayState,
    async reportDisplayPresent() {
      return undefined
    },
    async stampClockProbe() {
      return { hostMonotonicMicros: 0, hostEpochMicros: 0 }
    },
    subscribeToDisplayUpdates:
      input.subscribeToDisplayUpdates ??
      (async () => {
        return () => undefined
      }),
  }
}

afterEach(() => {
  vi.useRealTimers()
})

describe('useMeasurementLaneState', () => {
  it('uses present telemetry even when the fixture lane is disabled', async () => {
    const service = buildService({
      async getViewerDisplayState() {
        return buildPointer(true, { measurementLaneEnabled: false })
      },
    })
    const { result } = renderHook(() => useMeasurementLaneState(service))

    await waitFor(() => expect(result.current).toBe(true))
  })

  it('tracks pointer updates after the initial snapshot', async () => {
    let emit: ((pointer: DisplayPointerSnapshot) => void) | undefined
    const service = buildService({
      async getViewerDisplayState() {
        return buildPointer(false)
      },
      async subscribeToDisplayUpdates(input) {
        emit = input.onPointer
        return () => {
          emit = undefined
        }
      },
    })
    const { result } = renderHook(() => useMeasurementLaneState(service))

    await waitFor(() => expect(emit).toBeDefined())
    act(() => emit?.(buildPointer(true)))

    expect(result.current).toBe(true)
  })

  it('does not let a late initial snapshot overwrite a newer pointer event', async () => {
    let emit: ((pointer: DisplayPointerSnapshot) => void) | undefined
    let resolveInitial:
      | ((pointer: DisplayPointerSnapshot) => void)
      | undefined
    const initialSnapshot = new Promise<DisplayPointerSnapshot>((resolve) => {
      resolveInitial = resolve
    })
    const service = buildService({
      getViewerDisplayState: vi
        .fn<DisplayHostService['getViewerDisplayState']>()
        .mockReturnValue(initialSnapshot),
      async subscribeToDisplayUpdates(input) {
        emit = input.onPointer
        return () => undefined
      },
    })
    const { result } = renderHook(() => useMeasurementLaneState(service))

    await waitFor(() => expect(emit).toBeDefined())
    act(() => emit?.(buildPointer(true)))
    await act(async () => resolveInitial?.(buildPointer(false)))

    expect(result.current).toBe(true)
  })

  it('ignores a delayed lower-revision pointer event', async () => {
    let emit: ((pointer: DisplayPointerSnapshot) => void) | undefined
    const service = buildService({
      async getViewerDisplayState() {
        return buildPointer(false, { revision: 5 })
      },
      async subscribeToDisplayUpdates(input) {
        emit = input.onPointer
        return () => undefined
      },
    })
    const { result } = renderHook(() => useMeasurementLaneState(service))

    await waitFor(() => expect(emit).toBeDefined())
    act(() => emit?.(buildPointer(true, { revision: 6 })))
    expect(result.current).toBe(true)

    act(() => emit?.(buildPointer(false, { revision: 4 })))
    expect(result.current).toBe(true)
  })

  it('retries a failed snapshot instead of remaining disabled forever', async () => {
    vi.useFakeTimers()
    const getViewerDisplayState = vi
      .fn<DisplayHostService['getViewerDisplayState']>()
      .mockRejectedValueOnce(new Error('host unavailable'))
      .mockResolvedValue(buildPointer(true))
    const service = buildService({
      getViewerDisplayState,
      subscribeToDisplayUpdates: () => new Promise(() => undefined),
    })
    const { result } = renderHook(() => useMeasurementLaneState(service))

    await act(async () => {
      await Promise.resolve()
      await vi.advanceTimersByTimeAsync(MEASUREMENT_LANE_RETRY_INTERVAL_MS)
    })

    expect(getViewerDisplayState).toHaveBeenCalledTimes(2)
    expect(result.current).toBe(true)
  })
})
