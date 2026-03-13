import { useEffect, useRef, useState } from 'react'
import { render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

vi.mock('../services/activePresetService.js', () => ({
  activePresetService: {
    applyPresetChange: vi.fn(),
  },
}))

import { BranchConfigContext } from '../../branch-config/BranchConfigContext.js'
import { activePresetService } from '../services/activePresetService.js'
import { SessionFlowProvider, useSessionFlow } from './SessionFlowProvider.js'

function createDeferredResult() {
  let resolve:
    | ((value: { sessionId: string; activePresetId: 'background-pink' | 'background-ivory' | 'warm-tone'; appliedAt: string }) => void)
    | undefined
  const promise = new Promise<{
    sessionId: string
    activePresetId: 'background-pink' | 'background-ivory' | 'warm-tone'
    appliedAt: string
  }>((nextResolve) => {
    resolve = nextResolve
  })

  return {
    promise,
    resolve(value: {
      sessionId: string
      activePresetId: 'background-pink' | 'background-ivory' | 'warm-tone'
      appliedAt: string
    }) {
      resolve?.(value)
    },
  }
}

function createCameraAdapter() {
  return {
    getReadinessSnapshot: vi.fn(async () => ({
      sessionId: 'session-24',
      connectionState: 'ready' as const,
      captureEnabled: true,
      lastStableCustomerState: 'ready' as const,
      error: null,
      emittedAt: '2026-03-08T10:00:05.000Z',
    })),
    watchReadiness: vi.fn(async () => () => undefined),
    getCaptureConfidenceSnapshot: vi.fn(async () => ({
      sessionId: 'session-24',
      revision: 1,
      updatedAt: '2026-03-08T10:00:06.000Z',
      shootEndsAt: '2026-03-08T10:50:00.000Z',
      activePreset: {
        presetId: 'warm-tone',
        label: '웜톤',
      },
      latestPhoto: {
        kind: 'ready' as const,
        photo: {
          sessionId: 'session-24',
          captureId: 'capture-002',
          sequence: 2,
          assetUrl: 'asset://session-24/capture-002',
          capturedAt: '2026-03-08T10:06:00.000Z',
        },
      },
    })),
    watchCaptureConfidence: vi.fn(async () => () => undefined),
  }
}

function createMultiSessionCameraAdapter() {
  return {
    getReadinessSnapshot: vi.fn(async ({ sessionId }: { sessionId: string }) => ({
      sessionId,
      connectionState: 'ready' as const,
      captureEnabled: true,
      lastStableCustomerState: 'ready' as const,
      error: null,
      emittedAt: '2026-03-08T10:00:05.000Z',
    })),
    watchReadiness: vi.fn(async () => () => undefined),
    getCaptureConfidenceSnapshot: vi.fn(async ({ sessionId }: { sessionId: string }) => ({
      sessionId,
      revision: 1,
      updatedAt: '2026-03-08T10:00:06.000Z',
      shootEndsAt: '2026-03-08T10:50:00.000Z',
      activePreset: {
        presetId: 'warm-tone',
        label: '웜톤',
      },
      latestPhoto: {
        kind: 'ready' as const,
        photo: {
          sessionId,
          captureId: `capture-${sessionId}`,
          sequence: 2,
          assetUrl: `asset://${sessionId}/capture-002`,
          capturedAt: '2026-03-08T10:06:00.000Z',
        },
      },
    })),
    watchCaptureConfidence: vi.fn(async () => () => undefined),
  }
}

function createLifecycleService() {
  return {
    startSession: vi.fn(async () => ({
      ok: true as const,
      value: {
        sessionId: 'session-24',
        sessionName: '김보라1234',
        sessionFolder: 'C:/sessions/김보라1234',
        manifestPath: 'C:/sessions/김보라1234/session.json',
        createdAt: '2026-03-08T10:00:00.000Z',
        preparationState: 'preparing' as const,
      },
    })),
  }
}

function createSequencedLifecycleService() {
  let invocationCount = 0

  return {
    startSession: vi.fn(async () => {
      invocationCount += 1

      if (invocationCount === 1) {
        return {
          ok: true as const,
          value: {
            sessionId: 'session-24',
            sessionName: '김보라1234',
            sessionFolder: 'C:/sessions/김보라1234',
            manifestPath: 'C:/sessions/김보라1234/session.json',
            createdAt: '2026-03-08T10:00:00.000Z',
            preparationState: 'preparing' as const,
          },
        }
      }

      return {
        ok: true as const,
        value: {
          sessionId: 'session-25',
          sessionName: '이다은5678',
          sessionFolder: 'C:/sessions/이다은5678',
          manifestPath: 'C:/sessions/이다은5678/session.json',
          createdAt: '2026-03-08T10:05:00.000Z',
          preparationState: 'preparing' as const,
        },
      }
    }),
  }
}

function createPresetSelectionService() {
  return {
    selectPreset: vi.fn(async () => ({
      ok: true as const,
      value: {
        manifestPath: 'C:/sessions/김보라1234/session.json',
        updatedAt: '2026-03-08T10:00:07.000Z',
        activePreset: {
          presetId: 'warm-tone',
          displayName: '웜톤',
        },
      },
    })),
  }
}

function ProviderStoryHarness({ requestedPresetId }: { requestedPresetId: 'background-pink' | 'warm-tone' }) {
  const { applyActivePresetChange, confirmPresetSelection, selectPreset, state, submitCheckIn, updateField } =
    useSessionFlow()
  const seededRef = useRef(false)
  const submittedRef = useRef(false)
  const selectedRef = useRef(false)
  const confirmedRef = useRef(false)
  const changedRef = useRef(false)
  const [applyResult, setApplyResult] = useState<'pending' | 'true' | 'false'>('pending')

  useEffect(() => {
    if (seededRef.current) {
      return
    }

    seededRef.current = true
    updateField('sessionName', '김보라1234')
  }, [updateField])

  useEffect(() => {
    if (submittedRef.current || state.phase !== 'idle' || state.fields.sessionName !== '김보라1234') {
      return
    }

    submittedRef.current = true
    void submitCheckIn()
  }, [state.fields.sessionName, state.phase, submitCheckIn])

  useEffect(() => {
    if (selectedRef.current || state.phase !== 'preset-selection') {
      return
    }

    selectedRef.current = true
    void selectPreset('warm-tone')
  }, [selectPreset, state.phase])

  useEffect(() => {
    if (
      confirmedRef.current ||
      state.phase !== 'preset-selection' ||
      state.selectedPresetId !== 'warm-tone' ||
      state.presetSelectionStatus !== 'idle'
    ) {
      return
    }

    confirmedRef.current = true
    void confirmPresetSelection()
  }, [confirmPresetSelection, state.phase, state.presetSelectionStatus, state.selectedPresetId])

  useEffect(() => {
    if (changedRef.current || state.phase !== 'capture-ready') {
      return
    }

    changedRef.current = true
    void applyActivePresetChange(requestedPresetId).then((result) => {
      setApplyResult(result ? 'true' : 'false')
    })
  }, [applyActivePresetChange, requestedPresetId, state.phase])

  return (
    <>
      <output data-testid="active-preset-id">{state.activePreset?.presetId ?? 'none'}</output>
      <output data-testid="pending-active-preset-id">{state.pendingActivePresetId ?? 'none'}</output>
      <output data-testid="latest-photo-id">
        {state.captureConfidence?.latestPhoto.kind === 'ready' ? state.captureConfidence.latestPhoto.photo.captureId : 'none'}
      </output>
      <output data-testid="apply-result">{applyResult}</output>
      <output data-testid="phase">{state.phase}</output>
    </>
  )
}

function StalePresetChangeHarness() {
  const {
    applyActivePresetChange,
    clearActiveSession,
    confirmPresetSelection,
    selectPreset,
    startJourney,
    state,
    submitCheckIn,
    updateField,
  } = useSessionFlow()
  const firstJourneyStartedRef = useRef(false)
  const firstSeededRef = useRef(false)
  const firstSubmittedRef = useRef(false)
  const firstSelectedRef = useRef(false)
  const firstConfirmedRef = useRef(false)
  const changeRequestedRef = useRef(false)
  const clearedRef = useRef(false)
  const secondJourneyStartedRef = useRef(false)
  const secondSeededRef = useRef(false)
  const secondSubmittedRef = useRef(false)

  useEffect(() => {
    if (firstJourneyStartedRef.current || state.phase !== 'start') {
      return
    }

    firstJourneyStartedRef.current = true
    startJourney()
  }, [startJourney, state.phase])

  useEffect(() => {
    if (firstSeededRef.current || state.phase !== 'idle' || secondJourneyStartedRef.current) {
      return
    }

    firstSeededRef.current = true
    updateField('sessionName', '김보라1234')
  }, [state.phase, updateField])

  useEffect(() => {
    if (
      firstSubmittedRef.current ||
      state.phase !== 'idle' ||
      state.fields.sessionName !== '김보라1234' ||
      secondJourneyStartedRef.current
    ) {
      return
    }

    firstSubmittedRef.current = true
    void submitCheckIn()
  }, [state.fields.sessionName, state.phase, submitCheckIn])

  useEffect(() => {
    if (
      firstSelectedRef.current ||
      state.phase !== 'preset-selection' ||
      secondJourneyStartedRef.current
    ) {
      return
    }

    firstSelectedRef.current = true
    void selectPreset('warm-tone')
  }, [selectPreset, state.phase])

  useEffect(() => {
    if (
      firstConfirmedRef.current ||
      state.phase !== 'preset-selection' ||
      state.selectedPresetId !== 'warm-tone' ||
      state.presetSelectionStatus !== 'idle' ||
      secondJourneyStartedRef.current
    ) {
      return
    }

    firstConfirmedRef.current = true
    void confirmPresetSelection()
  }, [confirmPresetSelection, state.phase, state.presetSelectionStatus, state.selectedPresetId])

  useEffect(() => {
    if (changeRequestedRef.current || state.phase !== 'capture-ready') {
      return
    }

    changeRequestedRef.current = true
    void applyActivePresetChange('background-pink')
  }, [applyActivePresetChange, state.phase])

  useEffect(() => {
    if (clearedRef.current || !changeRequestedRef.current || state.phase !== 'capture-ready') {
      return
    }

    clearedRef.current = true
    clearActiveSession()
  }, [clearActiveSession, state.phase])

  useEffect(() => {
    if (secondJourneyStartedRef.current || !clearedRef.current || state.activeSession !== null) {
      return
    }

    secondJourneyStartedRef.current = true
    startJourney()
  }, [startJourney, state.activeSession])

  useEffect(() => {
    if (secondSeededRef.current || state.phase !== 'idle' || state.fields.sessionName === '이다은5678' || !secondJourneyStartedRef.current) {
      return
    }

    secondSeededRef.current = true
    updateField('sessionName', '이다은5678')
  }, [state.fields.sessionName, state.phase, updateField])

  useEffect(() => {
    if (
      secondSubmittedRef.current ||
      state.phase !== 'idle' ||
      state.fields.sessionName !== '이다은5678' ||
      !secondJourneyStartedRef.current
    ) {
      return
    }

    secondSubmittedRef.current = true
    void submitCheckIn()
  }, [state.fields.sessionName, state.phase, submitCheckIn])

  return (
    <>
      <output data-testid="active-preset-id">{state.activePreset?.presetId ?? 'none'}</output>
      <output data-testid="pending-active-preset-id">{state.pendingActivePresetId ?? 'none'}</output>
      <output data-testid="phase">{state.phase}</output>
      <output data-testid="session-id">{state.activeSession?.sessionId ?? 'none'}</output>
    </>
  )
}

function renderHarness(requestedPresetId: 'background-pink' | 'warm-tone') {
  const cameraAdapter = createCameraAdapter()
  const lifecycleService = createLifecycleService()
  const presetSelectionService = createPresetSelectionService()
  const presetSelectionSettingsService = {
    loadLastUsedPresetId: vi.fn(async () => 'warm-tone'),
    saveLastUsedPresetId: vi.fn(async () => undefined),
  }
  const lifecycleLogger = {
    recordReadinessReached: vi.fn(async () => undefined),
  }
  const sessionTimingService = {
    initializeSessionTiming: vi.fn(),
    getSessionTiming: vi.fn(async ({ manifestPath, sessionId }: { manifestPath: string; sessionId: string }) => ({
      ok: true as const,
      value: {
        sessionId,
        manifestPath,
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt: '2099-03-08T09:50:00.000Z',
          sessionType: 'standard' as const,
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:00:00.000Z',
        },
      },
    })),
    extendSessionTiming: vi.fn(),
  }

  render(
    <BranchConfigContext.Provider
      value={{
        status: 'ready',
        config: {
          branchId: 'gangnam-main',
          branchPhoneNumber: '010-1234-5678',
          operationalToggles: {
            enablePhoneEscalation: true,
          },
        },
      }}
    >
      <SessionFlowProvider
        cameraAdapter={cameraAdapter}
        lifecycleLogger={lifecycleLogger}
        lifecycleService={lifecycleService}
        presetSelectionService={presetSelectionService}
        presetSelectionSettingsService={presetSelectionSettingsService}
        sessionTimingService={sessionTimingService}
      >
        <ProviderStoryHarness requestedPresetId={requestedPresetId} />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  )
}

function renderStaleHarness() {
  const cameraAdapter = createMultiSessionCameraAdapter()
  const lifecycleService = createSequencedLifecycleService()
  const presetSelectionService = createPresetSelectionService()
  const presetSelectionSettingsService = {
    loadLastUsedPresetId: vi.fn(async () => 'warm-tone'),
    saveLastUsedPresetId: vi.fn(async () => undefined),
  }
  const lifecycleLogger = {
    recordReadinessReached: vi.fn(async () => undefined),
  }
  const sessionTimingService = {
    initializeSessionTiming: vi.fn(),
    getSessionTiming: vi.fn(async () => ({
      ok: true as const,
      value: {
        sessionId: 'session-24',
        manifestPath: 'C:/sessions/김보라1234/session.json',
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt: '2099-03-08T09:50:00.000Z',
          sessionType: 'standard' as const,
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:00:00.000Z',
        },
      },
    })),
    extendSessionTiming: vi.fn(),
  }

  render(
    <BranchConfigContext.Provider
      value={{
        status: 'ready',
        config: {
          branchId: 'gangnam-main',
          branchPhoneNumber: '010-1234-5678',
          operationalToggles: {
            enablePhoneEscalation: true,
          },
        },
      }}
    >
      <SessionFlowProvider
        cameraAdapter={cameraAdapter}
        lifecycleLogger={lifecycleLogger}
        lifecycleService={lifecycleService}
        presetSelectionService={presetSelectionService}
        presetSelectionSettingsService={presetSelectionSettingsService}
        sessionTimingService={sessionTimingService}
      >
        <StalePresetChangeHarness />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  )
}

describe('SessionFlowProvider story 3.4', () => {
  it('reconciles the visible active preset from the typed host result while preserving the current latest-photo snapshot', async () => {
    vi.mocked(activePresetService.applyPresetChange).mockResolvedValue({
      sessionId: 'session-24',
      activePresetId: 'background-ivory',
      appliedAt: '2026-03-08T10:00:09.000Z',
    })

    renderHarness('background-pink')

    await waitFor(
      () => {
        expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
        expect(screen.getByTestId('active-preset-id')).toHaveTextContent('background-ivory')
        expect(screen.getByTestId('pending-active-preset-id')).toHaveTextContent('background-ivory')
        expect(screen.getByTestId('latest-photo-id')).toHaveTextContent('capture-002')
        expect(screen.getByTestId('apply-result')).toHaveTextContent('true')
      },
      { timeout: 4000 },
    )

    expect(activePresetService.applyPresetChange).toHaveBeenCalledWith({
      sessionId: 'session-24',
      presetId: 'background-pink',
    })
  })

  it('keeps same-preset selection as a provider-level no-op without host writes', async () => {
    vi.mocked(activePresetService.applyPresetChange).mockResolvedValue({
      sessionId: 'session-24',
      activePresetId: 'background-ivory',
      appliedAt: '2026-03-08T10:00:09.000Z',
    })

    renderHarness('warm-tone')

    await waitFor(
      () => {
        expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
        expect(screen.getByTestId('active-preset-id')).toHaveTextContent('warm-tone')
        expect(screen.getByTestId('pending-active-preset-id')).toHaveTextContent('none')
        expect(screen.getByTestId('apply-result')).toHaveTextContent('false')
      },
      { timeout: 4000 },
    )

    expect(activePresetService.applyPresetChange).not.toHaveBeenCalled()
  })

  it('ignores a stale in-flight preset change result after a new active session replaces the old one', async () => {
    const deferredResult = createDeferredResult()
    vi.mocked(activePresetService.applyPresetChange).mockReturnValue(deferredResult.promise)

    renderStaleHarness()

    await waitFor(
      () => {
        expect(activePresetService.applyPresetChange).toHaveBeenCalledWith({
          sessionId: 'session-24',
          presetId: 'background-pink',
        })
        expect(screen.getByTestId('session-id')).toHaveTextContent('session-25')
        expect(screen.getByTestId('active-preset-id')).toHaveTextContent('none')
      },
      { timeout: 6000 },
    )

    deferredResult.resolve({
      sessionId: 'session-24',
      activePresetId: 'background-pink',
      appliedAt: '2026-03-08T10:00:09.000Z',
    })

    await waitFor(
      () => {
        expect(screen.getByTestId('session-id')).toHaveTextContent('session-25')
        expect(screen.getByTestId('active-preset-id')).toHaveTextContent('none')
        expect(screen.getByTestId('pending-active-preset-id')).toHaveTextContent('none')
      },
      { timeout: 6000 },
    )
  }, 10000)

  it('returns a failed result when the host rejects an in-session preset change', async () => {
    vi.mocked(activePresetService.applyPresetChange).mockRejectedValue(new Error('host failed'))

    renderHarness('background-pink')

    await waitFor(
      () => {
        expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
        expect(screen.getByTestId('active-preset-id')).toHaveTextContent('warm-tone')
        expect(screen.getByTestId('pending-active-preset-id')).toHaveTextContent('none')
        expect(screen.getByTestId('apply-result')).toHaveTextContent('false')
      },
      { timeout: 4000 },
    )

    expect(activePresetService.applyPresetChange).toHaveBeenCalledWith({
      sessionId: 'session-24',
      presetId: 'background-pink',
    })
  })
})
