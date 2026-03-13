import { act, render, screen } from '@testing-library/react'
import { useEffect, useRef, type ComponentProps } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { BranchConfigContext } from '../../branch-config/BranchConfigContext.js'
import { SessionFlowProvider, useSessionFlow } from './SessionFlowProvider.js'

type TimingState = {
  reservationStartAt: string
  actualShootEndAt: string
  sessionType: 'standard'
  operatorExtensionCount: number
  lastTimingUpdateAt: string
}

function createDeferredTimingResult() {
  let resolve:
    | ((value: {
        ok: true
        value: {
          sessionId: string
          manifestPath: string
          timing: TimingState
        }
      }) => void)
    | undefined

  const promise = new Promise<{
    ok: true
    value: {
      sessionId: string
      manifestPath: string
      timing: TimingState
    }
  }>((nextResolve) => {
    resolve = nextResolve
  })

  return { promise, resolve }
}

function createDeferredStopper() {
  const stopper = vi.fn()
  let resolve: ((value: () => void) => void) | undefined

  const promise = new Promise<() => void>((nextResolve) => {
    resolve = nextResolve
  })

  return {
    promise,
    resolve: () => resolve?.(stopper),
    stopper,
  }
}

function createTimingHarness(options?: {
  delayedInitialTiming?: ReturnType<typeof createDeferredTimingResult>
  initialTiming?: TimingState
  initialCaptureConfidenceShootEndsAt?: string
}) {
  const readinessHandlers = new Set<
    (snapshot: {
      sessionId: string
      connectionState: 'ready' | 'waiting'
      captureEnabled: boolean
      lastStableCustomerState: 'ready' | null
      error: null
      emittedAt: string
    }) => void
  >()
  const captureConfidenceHandlers = new Set<
    (snapshot: {
      sessionId: string
      revision: number
      updatedAt: string
      shootEndsAt: string
      activePreset: {
        presetId: string
        label: string
      }
      latestPhoto: { kind: 'empty' }
    }) => void
  >()

  const readinessSnapshot = {
    sessionId: 'session-42',
    connectionState: 'ready' as const,
    captureEnabled: true,
    lastStableCustomerState: 'ready' as const,
    error: null,
    emittedAt: '2026-03-08T09:39:00.000Z',
  }

  let currentCaptureConfidence = {
    sessionId: 'session-42',
    revision: 1,
    updatedAt: '2026-03-08T09:39:00.000Z',
    shootEndsAt: options?.initialCaptureConfidenceShootEndsAt ?? '2026-03-08T09:50:00.000Z',
    activePreset: {
      presetId: 'background-pink',
      label: '배경지 - 핑크',
    },
    latestPhoto: {
      kind: 'empty' as const,
    },
  }

  let currentTiming: TimingState = options?.initialTiming ?? {
    reservationStartAt: '2026-03-08T09:00:00.000Z',
    actualShootEndAt: '2026-03-08T09:50:00.000Z',
    sessionType: 'standard',
    operatorExtensionCount: 0,
    lastTimingUpdateAt: '2026-03-08T09:39:00.000Z',
  }

  return {
    cameraAdapter: {
      getReadinessSnapshot: vi.fn(async () => readinessSnapshot),
      watchReadiness: vi.fn(async ({ onStatus }: { onStatus: (snapshot: typeof readinessSnapshot) => void }) => {
        readinessHandlers.add(onStatus)

        return () => {
          readinessHandlers.delete(onStatus)
        }
      }),
      getCaptureConfidenceSnapshot: vi.fn(async () => currentCaptureConfidence),
      requestCapture: vi.fn(async () => ({
        schemaVersion: 'boothy.camera.contract.v1' as const,
        requestId: 'req-capture-001',
        correlationId: 'session-42',
        ok: true,
        sessionId: 'session-42',
        captureId: 'capture-001',
        capturedAt: '2026-03-08T09:39:01.000Z',
        manifestPath: 'C:/sessions/김보라1234/session.json',
      })),
      watchCaptureConfidence: vi.fn(
        async ({ onSnapshot }: { onSnapshot: (snapshot: typeof currentCaptureConfidence) => void }) => {
          captureConfidenceHandlers.add(onSnapshot)

          return () => {
            captureConfidenceHandlers.delete(onSnapshot)
          }
        },
      ),
    },
    emitCaptureConfidence(nextSnapshot: typeof currentCaptureConfidence) {
      currentCaptureConfidence = nextSnapshot

      for (const handler of captureConfidenceHandlers) {
        handler(nextSnapshot)
      }
    },
    emitReadiness(nextSnapshot: typeof readinessSnapshot) {
      for (const handler of readinessHandlers) {
        handler(nextSnapshot)
      }
    },
    lifecycleService: {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-42',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/김보라1234',
          manifestPath: 'C:/sessions/김보라1234/session.json',
          createdAt: '2026-03-08T09:39:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    },
    presetSelectionService: {
      selectPreset: vi.fn(async () => ({
        ok: true as const,
        value: {
          activePreset: {
            presetId: 'background-pink',
            displayName: '배경지 - 핑크',
          },
        },
      })),
    },
    sessionTimingService: {
      initializeSessionTiming: vi.fn(),
      getSessionTiming: vi.fn(async () => {
        if (options?.delayedInitialTiming) {
          return options.delayedInitialTiming.promise
        }

        return {
          ok: true as const,
          value: {
            sessionId: 'session-42',
            manifestPath: 'C:/sessions/김보라1234/session.json',
            timing: currentTiming,
          },
        }
      }),
      extendSessionTiming: vi.fn(),
    },
    setTiming(nextTiming: TimingState) {
      currentTiming = nextTiming
    },
  }
}

function AutoDriveCaptureFlow() {
  const { confirmPresetSelection, requestCapture, selectPreset, startJourney, submitCheckIn, updateField, state } =
    useSessionFlow()
  const startedRef = useRef(false)
  const sessionNameSeededRef = useRef(false)
  const submittedRef = useRef(false)
  const selectedRef = useRef(false)
  const confirmedRef = useRef(false)
  const sessionName = '김보라1234'

  useEffect(() => {
    if (startedRef.current) {
      return
    }

    startedRef.current = true
    startJourney()
  }, [startJourney])

  useEffect(() => {
    if (sessionNameSeededRef.current || state.phase !== 'idle') {
      return
    }

    sessionNameSeededRef.current = true
    updateField('sessionName', sessionName)
  }, [sessionName, state.phase, updateField])

  useEffect(() => {
    if (
      submittedRef.current ||
      state.phase !== 'idle' ||
      !sessionNameSeededRef.current ||
      state.fields.sessionName !== sessionName
    ) {
      return
    }

    submittedRef.current = true
    void submitCheckIn()
  }, [sessionName, state.fields.sessionName, state.phase, submitCheckIn])

  useEffect(() => {
    if (selectedRef.current || state.phase !== 'preset-selection') {
      return
    }

    selectedRef.current = true
    void selectPreset('background-pink')
  }, [selectPreset, state.phase])

  useEffect(() => {
    if (confirmedRef.current || state.phase !== 'preset-selection' || !state.selectedPresetId) {
      return
    }

    confirmedRef.current = true
    void confirmPresetSelection()
  }, [confirmPresetSelection, state.phase, state.selectedPresetId])

  return (
    <>
      <output data-testid="phase">{state.phase}</output>
      <output data-testid="timing-end">{state.sessionTiming?.actualShootEndAt ?? 'none'}</output>
      <button
        onClick={() => {
          void requestCapture()
        }}
        type="button"
      >
        request-capture
      </button>
    </>
  )
}

function renderHarness(overrides: Partial<ComponentProps<typeof SessionFlowProvider>> = {}) {
  return render(
    <BranchConfigContext.Provider
      value={{
        status: 'ready',
        config: {
          branchId: 'gangnam-main',
          branchPhoneNumber: '010-1234-5678',
          operationalToggles: {
            enablePhoneEscalation: false,
          },
        },
      }}
    >
      <SessionFlowProvider
        captureAdapter={{
          loadSessionGallery: vi.fn(async () => ({
            schemaVersion: 'boothy.contract.v1',
            sessionId: 'session-42',
            sessionName: '김보라1234',
            shootEndsAt: '2026-03-08T09:50:00.000Z',
            activePresetName: '배경지 - 핑크',
            latestCaptureId: null,
            selectedCaptureId: null,
            items: [],
          })),
          deleteSessionPhoto: vi.fn(async () => {
            throw new Error('not used')
          }),
        }}
        lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
        presetCatalogService={{
          loadApprovedPresetCatalog: vi.fn(async () => ({
            status: 'ready' as const,
            presets: [],
            source: 'approved' as const,
          })),
        }}
        {...overrides}
      >
        <AutoDriveCaptureFlow />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  )
}

describe('SessionFlowProvider timing synchronization', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  async function flushEffects() {
    await act(async () => {
      await Promise.resolve()
      await Promise.resolve()
    })
  }

  it('keeps the authoritative timing synced while the booth regresses to a waiting preparation state', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const harness = createTimingHarness()

    renderHarness({
      cameraAdapter: harness.cameraAdapter,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
    expect(screen.getByTestId('timing-end')).toHaveTextContent('2026-03-08T09:50:00.000Z')

    act(() => {
      harness.emitReadiness({
        sessionId: 'session-42',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T09:40:00.000Z',
      })
    })

    await flushEffects()
    expect(screen.getByTestId('phase')).toHaveTextContent('preparing')

    act(() => {
      harness.emitCaptureConfidence({
        sessionId: 'session-42',
        revision: 2,
        updatedAt: '2026-03-08T09:40:10.000Z',
        shootEndsAt: '2026-03-08T09:56:00.000Z',
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        latestPhoto: {
          kind: 'empty',
        },
      })
    })

    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('preparing')
    expect(screen.getByTestId('timing-end')).toHaveTextContent('2026-03-08T09:56:00.000Z')
  })

  it('does not let a slower timing read overwrite newer snapshot timing for the active session', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const delayedInitialTiming = createDeferredTimingResult()
    const harness = createTimingHarness({ delayedInitialTiming })

    renderHarness({
      cameraAdapter: harness.cameraAdapter,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
    expect(screen.getByTestId('timing-end')).toHaveTextContent('2026-03-08T09:50:00.000Z')

    act(() => {
      harness.emitCaptureConfidence({
        sessionId: 'session-42',
        revision: 2,
        updatedAt: '2026-03-08T09:40:10.000Z',
        shootEndsAt: '2026-03-08T09:56:00.000Z',
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        latestPhoto: {
          kind: 'empty',
        },
      })
    })

    await flushEffects()
    expect(screen.getByTestId('timing-end')).toHaveTextContent('2026-03-08T09:56:00.000Z')

    await act(async () => {
      delayedInitialTiming.resolve?.({
        ok: true,
        value: {
          sessionId: 'session-42',
          manifestPath: 'C:/sessions/김보라1234/session.json',
          timing: {
            reservationStartAt: '2026-03-08T09:00:00.000Z',
            actualShootEndAt: '2026-03-08T09:50:00.000Z',
            sessionType: 'standard',
            operatorExtensionCount: 0,
            lastTimingUpdateAt: '2026-03-08T09:39:00.000Z',
          },
        },
      })
      await Promise.resolve()
    })

    expect(screen.getByTestId('timing-end')).toHaveTextContent('2026-03-08T09:56:00.000Z')
  })

  it('does not make exact-end capture gating a Story 4.1 provider responsibility', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const harness = createTimingHarness()

    renderHarness({
      cameraAdapter: harness.cameraAdapter,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')

    vi.setSystemTime(new Date('2026-03-08T09:50:01.000Z'))

    await act(async () => {
      screen.getByRole('button', { name: 'request-capture' }).click()
      await Promise.resolve()
    })

    expect(harness.cameraAdapter.requestCapture).toHaveBeenCalledTimes(1)
  })

  it('does not demote a ready capture flow when capture-confidence sync fails', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const harness = createTimingHarness()
    const cameraAdapter = {
      ...harness.cameraAdapter,
      getCaptureConfidenceSnapshot: vi.fn(async () => {
        throw new Error('capture confidence unavailable')
      }),
      watchCaptureConfidence: vi.fn(async () => {
        throw new Error('capture confidence unavailable')
      }),
    }

    renderHarness({
      cameraAdapter,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
  })

  it('cancels a readiness watch that resolves after provider cleanup already ran', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const deferredStopper = createDeferredStopper()
    const harness = createTimingHarness()
    const cameraAdapter = {
      ...harness.cameraAdapter,
      watchReadiness: vi.fn(async () => deferredStopper.promise),
    }

    const view = renderHarness({
      cameraAdapter,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    expect(cameraAdapter.watchReadiness).toHaveBeenCalled()

    view.unmount()
    deferredStopper.resolve()
    await flushEffects()

    expect(deferredStopper.stopper).toHaveBeenCalledTimes(1)
  })

  it('cancels a capture-confidence watch that resolves after provider cleanup already ran', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const deferredStopper = createDeferredStopper()
    const harness = createTimingHarness()
    const watchCaptureConfidence = vi
      .fn<(
        args: { onSnapshot: (snapshot: { sessionId: string; revision: number; updatedAt: string; shootEndsAt: string; activePreset: { presetId: string; label: string }; latestPhoto: { kind: 'empty' } }) => void },
      ) => Promise<() => void>>()
      .mockResolvedValueOnce(() => undefined)
      .mockImplementationOnce(async () => deferredStopper.promise)
    const cameraAdapter = {
      ...harness.cameraAdapter,
      watchCaptureConfidence,
    }

    const view = renderHarness({
      cameraAdapter,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()
    expect(watchCaptureConfidence).toHaveBeenCalledTimes(2)

    view.unmount()
    deferredStopper.resolve()
    await flushEffects()

    expect(deferredStopper.stopper).toHaveBeenCalledTimes(1)
  })

  it('does not trigger warning or ended side effects from the Story 4.1 provider path', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const harness = createTimingHarness()
    const timingAlertAudio = {
      play: vi.fn(async () => undefined),
    }
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
      recordWarningShown: vi.fn(async () => undefined),
      recordActualShootEnd: vi.fn(async () => undefined),
    }

    renderHarness({
      cameraAdapter: harness.cameraAdapter,
      lifecycleLogger,
      lifecycleService: harness.lifecycleService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
      timingAlertAudio,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')

    await act(async () => {
      await vi.advanceTimersByTimeAsync(11 * 60 * 1000)
    })
    await flushEffects()

    expect(timingAlertAudio.play).not.toHaveBeenCalled()
    expect(lifecycleLogger.recordWarningShown).not.toHaveBeenCalled()
    expect(lifecycleLogger.recordActualShootEnd).not.toHaveBeenCalled()
  })

  it('does not resolve post-end immediately when the shoot end time is far beyond the browser timer limit', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const farFutureShootEndAt = '2099-03-08T09:50:00.000Z'
    const harness = createTimingHarness({
      initialTiming: {
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        actualShootEndAt: farFutureShootEndAt,
        sessionType: 'standard',
        operatorExtensionCount: 0,
        lastTimingUpdateAt: '2026-03-08T09:39:00.000Z',
      },
      initialCaptureConfidenceShootEndsAt: farFutureShootEndAt,
    })
    const postEndOutcomeService = {
      getPostEndOutcome: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-42',
          actualShootEndAt: farFutureShootEndAt,
          outcomeKind: 'completed' as const,
          guidanceMode: 'done' as const,
          sessionName: '김보라1234',
          showSessionName: true,
          handoffTargetLabel: null,
        },
      })),
    }

    renderHarness({
      cameraAdapter: harness.cameraAdapter,
      lifecycleService: harness.lifecycleService,
      postEndOutcomeService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10)
    })
    await flushEffects()

    expect(postEndOutcomeService.getPostEndOutcome).not.toHaveBeenCalled()
    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
  })

  it('resolves post-end outcome once the authoritative end time passes', async () => {
    vi.setSystemTime(new Date('2026-03-08T09:39:00.000Z'))
    const harness = createTimingHarness()
    const postEndOutcomeService = {
      getPostEndOutcome: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-42',
          actualShootEndAt: '2026-03-08T09:50:00.000Z',
          outcomeKind: 'completed' as const,
          guidanceMode: 'done' as const,
          sessionName: '김보라1234',
          showSessionName: true,
          handoffTargetLabel: null,
        },
      })),
    }

    renderHarness({
      cameraAdapter: harness.cameraAdapter,
      lifecycleService: harness.lifecycleService,
      postEndOutcomeService,
      presetSelectionService: harness.presetSelectionService,
      sessionTimingService: harness.sessionTimingService,
    })

    await flushEffects()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200)
    })
    await flushEffects()

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')

    await act(async () => {
      await vi.advanceTimersByTimeAsync(11 * 60 * 1000)
    })
    await flushEffects()

    expect(postEndOutcomeService.getPostEndOutcome).toHaveBeenCalledWith({
      manifestPath: 'C:/sessions/김보라1234/session.json',
      sessionId: 'session-42',
    })
    expect(screen.getByTestId('phase')).toHaveTextContent('post-end')
  })
})
