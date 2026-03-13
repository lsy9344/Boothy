import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useEffect, useRef } from 'react'
import { describe, expect, it, vi } from 'vitest'

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { mvpPresetCatalog } from '../../src/customer-flow/data/mvpPresetCatalog.js'
import { CheckInScreen } from '../../src/customer-flow/screens/CheckInScreen.js'
import { PreparationScreen } from '../../src/customer-flow/screens/PreparationScreen.js'
import { PresetScreen } from '../../src/customer-flow/screens/PresetScreen.js'
import type { PresetCatalogService } from '../../src/preset-catalog/services/presetCatalogService.js'
import type { PresetSelectionService } from '../../src/session-domain/services/presetSelection.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'
import { selectSessionTimeDisplay } from '../../src/timing-policy/selectors/sessionTimeDisplay.js'

function createReadinessAdapter(initialSnapshot: {
  sessionId: string
  connectionState: 'preparing' | 'waiting' | 'ready' | 'phone-required'
  captureEnabled: boolean
  lastStableCustomerState: 'preparing' | 'ready' | null
  error: null
  emittedAt: string
}, capturePreset = {
  presetId: 'background-ivory',
  label: '배경지 - 아이보리',
}) {
  let handler: ((status: typeof initialSnapshot) => void) | null = null
  let captureConfidenceHandler:
    | ((snapshot: {
        sessionId: string
        revision: number
        updatedAt: string
        shootEndsAt: string
        activePreset: {
          presetId: string
          label: string
        }
        latestPhoto: {
          kind: 'empty'
        }
      }) => void)
    | null = null
  let captureConfidenceSnapshot = {
    sessionId: initialSnapshot.sessionId,
    revision: 0,
    updatedAt: '2026-03-08T00:00:05.000Z',
    shootEndsAt: '2026-03-08T00:20:00.000Z',
    activePreset: capturePreset,
    latestPhoto: {
      kind: 'empty' as const,
    },
  }

  return {
    getReadinessSnapshot: vi.fn(async () => initialSnapshot),
    watchReadiness: vi.fn(async ({ onStatus }: { onStatus: (status: typeof initialSnapshot) => void }) => {
      handler = onStatus
      return () => {
        handler = null
      }
    }),
    getCaptureConfidenceSnapshot: vi.fn(async () => captureConfidenceSnapshot),
    watchCaptureConfidence: vi.fn(
      async ({
        onSnapshot,
      }: {
        onSnapshot: (snapshot: typeof captureConfidenceSnapshot) => void
      }) => {
        captureConfidenceHandler = onSnapshot
        return () => {
          captureConfidenceHandler = null
        }
      },
    ),
    emit(nextSnapshot: typeof initialSnapshot) {
      if (!handler) {
        throw new Error('readiness handler is not registered')
      }

      handler(nextSnapshot)
    },
    emitCaptureConfidence(nextSnapshot: typeof captureConfidenceSnapshot) {
      captureConfidenceSnapshot = nextSnapshot

      captureConfidenceHandler?.(nextSnapshot)
    },
  }
}

function TestCustomerFlow() {
  const { confirmPresetSelection, presetCatalogState, selectPreset, state } = useSessionFlow()
  const sessionTimeDisplay = state.sessionTiming ? selectSessionTimeDisplay(state.sessionTiming.actualShootEndAt) : null

  if (state.phase === 'preset-selection' && state.activeSession) {
    return (
      <PresetScreen
        catalogState={presetCatalogState}
        isApplyingPreset={state.presetSelectionStatus === 'applying'}
        onConfirmPreset={confirmPresetSelection}
        onSelectPreset={selectPreset}
        selectionFeedback={state.presetSelectionFeedback}
        selectedPresetId={state.selectedPresetId}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={sessionTimeDisplay}
      />
    )
  }

  if (
    (state.phase === 'preparing' || state.phase === 'capture-loading' || state.phase === 'capture-ready') &&
    state.activeSession &&
    state.readiness
  ) {
    return (
      <PreparationScreen
        onStartCapture={() => undefined}
        readiness={state.readiness}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={sessionTimeDisplay}
      />
    )
  }

  return <CheckInScreen />
}

function FlowStateProbe() {
  const {
    clearActiveSession,
    confirmPresetSelection,
    selectPreset,
    startJourney,
    submitCheckIn,
    updateField,
    state,
  } = useSessionFlow()
  const startRequestedRef = useRef(false)
  const sessionNameSeededRef = useRef(false)
  const checkInSubmittedRef = useRef(false)
  const sessionName = '김보라7777'

  useEffect(() => {
    if (startRequestedRef.current) {
      return
    }

    startRequestedRef.current = true
    startJourney()
  }, [startJourney])

  useEffect(() => {
    if (!startRequestedRef.current || sessionNameSeededRef.current || state.phase !== 'idle') {
      return
    }

    sessionNameSeededRef.current = true
    updateField('sessionName', sessionName)
  }, [sessionName, state.phase, updateField])

  useEffect(() => {
    if (
      !startRequestedRef.current ||
      checkInSubmittedRef.current ||
      !sessionNameSeededRef.current ||
      state.phase !== 'idle' ||
      state.fields.sessionName !== sessionName
    ) {
      return
    }

    checkInSubmittedRef.current = true
    void submitCheckIn()
  }, [sessionName, state.fields.sessionName, state.phase, submitCheckIn])

  return (
    <div>
      <button
        onClick={() => {
          startRequestedRef.current = true
          startJourney()
        }}
        type="button"
      >
        세션 시작
      </button>
      <button
        onClick={() => {
          void selectPreset('background-pink')
        }}
        type="button"
      >
        핑크 선택
      </button>
      <button
        onClick={() => {
          void confirmPresetSelection()
        }}
        type="button"
      >
        프리셋 확정
      </button>
      <button
        onClick={() => {
          clearActiveSession()
        }}
        type="button"
      >
        세션 초기화
      </button>
      <output data-testid="phase">{state.phase}</output>
      <output data-testid="session-id">{state.activeSession?.sessionId ?? 'none'}</output>
      <output data-testid="active-preset">{state.activePreset?.displayName ?? 'none'}</output>
      <output data-testid="selected-preset">{state.selectedPresetId ?? 'none'}</output>
      <output data-testid="preset-selection-feedback">{state.presetSelectionFeedback ?? 'none'}</output>
      <output data-testid="preset-selection-error-code">{state.presetSelectionFailure?.errorCode ?? 'none'}</output>
    </div>
  )
}

function createSessionTimingService(sessionId: string, manifestPath: string) {
  return {
    initializeSessionTiming: vi.fn(),
    getSessionTiming: vi.fn(async () => ({
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
}

function renderSelectionFlow(
  {
    branchPhoneNumber = '010-1234-5678',
    enablePhoneEscalation = false,
    lifecycleLogger = { recordReadinessReached: vi.fn(async () => undefined) },
    lifecycleService,
    presetCatalogService,
    presetSelectionService,
    presetSelectionSettingsService,
    readinessAdapter,
    sessionTimingService,
  }: {
    branchPhoneNumber?: string
    enablePhoneEscalation?: boolean
    lifecycleLogger?: { recordReadinessReached: ReturnType<typeof vi.fn> }
    lifecycleService: {
      startSession: ReturnType<typeof vi.fn>
    }
    presetCatalogService?: PresetCatalogService
    presetSelectionService?: {
      selectPreset: ReturnType<typeof vi.fn>
    }
    presetSelectionSettingsService?: {
      loadLastUsedPresetId: ReturnType<typeof vi.fn>
      saveLastUsedPresetId: ReturnType<typeof vi.fn>
    }
    readinessAdapter: ReturnType<typeof createReadinessAdapter>
    sessionTimingService: ReturnType<typeof createSessionTimingService>
  },
) {
  render(
    <BranchConfigContext.Provider
      value={{
        status: 'ready',
        config: {
          branchPhoneNumber,
          operationalToggles: {
            enablePhoneEscalation,
          },
        },
      }}
    >
      <SessionFlowProvider
        cameraAdapter={readinessAdapter}
        lifecycleLogger={lifecycleLogger}
        lifecycleService={lifecycleService}
        presetCatalogService={presetCatalogService}
        presetSelectionService={presetSelectionService}
        presetSelectionSettingsService={presetSelectionSettingsService}
        sessionTimingService={sessionTimingService}
      >
        <TestCustomerFlow />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  )
}

describe('preset selection flow', () => {
  it('requires explicit selection and confirmation before capture progression', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-1',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-1',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/김보라1234',
          manifestPath: 'C:/sessions/김보라1234/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }
    const presetSelectionSettingsService = {
      loadLastUsedPresetId: vi.fn(async () => 'background-ivory'),
      saveLastUsedPresetId: vi.fn(async () => undefined),
    }
    const presetSelectionService = {
      selectPreset: vi.fn(async () => ({
        ok: true as const,
        value: {
          manifestPath: 'C:/sessions/김보라1234/session.json',
          updatedAt: '2026-03-08T00:00:06.000Z',
          activePreset: {
            presetId: 'background-pink',
            displayName: '배경지 - 핑크',
          },
        },
      })),
    }
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
    }

    renderSelectionFlow({
      enablePhoneEscalation: true,
      lifecycleLogger,
      lifecycleService,
      presetSelectionService,
      presetSelectionSettingsService,
      readinessAdapter,
      sessionTimingService: createSessionTimingService('session-1', 'C:/sessions/김보라1234/session.json'),
    })

    await user.type(screen.getByLabelText('세션 이름'), '김보라1234')
    await user.keyboard('{Enter}')

    expect(
      await screen.findByRole('heading', { name: '원하는 프리셋을 눌러 주세요.' }, { timeout: 3000 }),
    ).toBeInTheDocument()
    expect(screen.getByText('촬영 종료 시간')).toBeInTheDocument()
    expect(screen.getByText('오후 6:50')).toBeInTheDocument()
    expect(screen.getByText('이 시간까지 촬영할 수 있어요.')).toBeInTheDocument()

    const confirmButton = screen.getByRole('button', { name: '이 프리셋으로 계속' })
    expect(confirmButton).toBeDisabled()
    expect(screen.getByText('프리셋을 먼저 고르면 다음으로 넘어갈 수 있어요.')).toBeInTheDocument()
    expect(presetSelectionSettingsService.loadLastUsedPresetId).toHaveBeenCalledTimes(1)

    const presetList = screen.getByRole('list', { name: '프리셋 목록' })
    const presetButtons = within(presetList).getAllByRole('button')
    expect(presetButtons).toHaveLength(mvpPresetCatalog.length)

    const pinkPresetButton = screen.getByRole('button', { name: /배경지 - 핑크/i })
    expect(pinkPresetButton).toHaveAttribute('aria-pressed', 'false')

    await user.click(pinkPresetButton)

    expect(presetSelectionService.selectPreset).not.toHaveBeenCalled()
    expect(presetSelectionSettingsService.saveLastUsedPresetId).not.toHaveBeenCalled()
    expect(pinkPresetButton).toHaveAttribute('aria-pressed', 'true')
    expect(confirmButton).toBeEnabled()

    await user.click(confirmButton)

    await waitFor(() => {
      expect(presetSelectionService.selectPreset).toHaveBeenCalledWith({
        presetId: 'background-pink',
        sessionId: 'session-1',
      })
    })
    await waitFor(() => {
      expect(presetSelectionSettingsService.saveLastUsedPresetId).toHaveBeenCalledWith('background-pink')
    })
    await waitFor(() => {
      expect(readinessAdapter.watchCaptureConfidence).toHaveBeenCalled()
    })
    await act(async () => {
      readinessAdapter.emitCaptureConfidence({
        sessionId: 'session-1',
        revision: 1,
        updatedAt: '2026-03-08T00:00:07.000Z',
        shootEndsAt: '2099-03-08T00:20:00.000Z',
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        latestPhoto: {
          kind: 'empty',
        },
      })
    })
    expect(await screen.findByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()
  })

  it('starts each new session with no preselected preset even if a last-used preference exists', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-2',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-2',
          sessionName: '김보라4321',
          sessionFolder: 'C:/sessions/김보라4321',
          manifestPath: 'C:/sessions/김보라4321/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }
    const presetSelectionSettingsService = {
      loadLastUsedPresetId: vi.fn(async () => 'background-ivory'),
      saveLastUsedPresetId: vi.fn(async () => undefined),
    }
    const presetSelectionService = {
      selectPreset: vi.fn(async () => ({
        ok: true as const,
        value: {
          manifestPath: 'C:/sessions/김보라4321/session.json',
          updatedAt: '2026-03-08T00:00:06.000Z',
          activePreset: {
            presetId: 'background-ivory',
            displayName: '배경지 - 아이보리',
          },
        },
      })),
    }

    renderSelectionFlow({
      branchPhoneNumber: '010-9999-0000',
      lifecycleService,
      presetSelectionService,
      presetSelectionSettingsService,
      readinessAdapter,
      sessionTimingService: createSessionTimingService('session-2', 'C:/sessions/김보라4321/session.json'),
    })

    await user.type(screen.getByLabelText('세션 이름'), '김보라4321')
    await user.keyboard('{Enter}')

    expect(
      await screen.findByRole('heading', { name: '원하는 프리셋을 눌러 주세요.' }, { timeout: 3000 }),
    ).toBeInTheDocument()

    expect(presetSelectionSettingsService.loadLastUsedPresetId).toHaveBeenCalledTimes(1)
    expect(screen.getByRole('button', { name: '이 프리셋으로 계속' })).toBeDisabled()
    expect(screen.getByText('촬영 종료 시간')).toBeInTheDocument()
    expect(screen.getByText('오후 6:50')).toBeInTheDocument()

    const ivoryPresetButton = screen.getByRole('button', { name: /배경지 - 아이보리/i })
    expect(ivoryPresetButton).toHaveAttribute('aria-pressed', 'false')

    const previewImages = screen.getAllByRole('img')
    expect(previewImages).toHaveLength(mvpPresetCatalog.length)

    for (const image of previewImages) {
      expect(image.getAttribute('src') ?? '').not.toMatch(/sessions/i)
    }
  })

  it('refreshes the visible adjusted end time while preset selection stays on screen', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-2-timing-refresh',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-2-timing-refresh',
          sessionName: '김보라2468',
          sessionFolder: 'C:/sessions/김보라2468',
          manifestPath: 'C:/sessions/김보라2468/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }

    renderSelectionFlow({
      lifecycleService,
      readinessAdapter,
      sessionTimingService: createSessionTimingService(
        'session-2-timing-refresh',
        'C:/sessions/김보라2468/session.json',
      ),
    })

    await user.type(screen.getByLabelText('세션 이름'), '김보라2468')
    await user.keyboard('{Enter}')

    expect(
      await screen.findByRole('heading', { name: '원하는 프리셋을 눌러 주세요.' }, { timeout: 3000 }),
    ).toBeInTheDocument()
    expect(screen.getByText('오후 6:50')).toBeInTheDocument()
    await waitFor(() => {
      expect(readinessAdapter.watchCaptureConfidence).toHaveBeenCalled()
    })

    await act(async () => {
      readinessAdapter.emitCaptureConfidence({
        sessionId: 'session-2-timing-refresh',
        revision: 1,
        updatedAt: '2026-03-08T09:02:10.000Z',
        shootEndsAt: '2099-03-08T09:56:00.000Z',
        activePreset: {
          presetId: 'background-ivory',
          label: '배경지 - 아이보리',
        },
        latestPhoto: {
          kind: 'empty',
        },
      })
    })

    await waitFor(() => {
      expect(screen.getByText('오후 6:56')).toBeInTheDocument()
    })
  })

  it('falls back to the approved preset list and keeps customer copy safe when the verified candidate catalog drifts', async () => {
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-2c',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-2c',
          sessionName: '김보라1111',
          sessionFolder: 'C:/sessions/김보라1111',
          manifestPath: 'C:/sessions/김보라1111/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }

    renderSelectionFlow({
      lifecycleService,
      presetCatalogService: {
        loadApprovedPresetCatalog: vi.fn(async () => ({
          status: 'ready' as const,
          source: 'approved-fallback' as const,
          auditReason: 'reordered_catalog' as const,
          presets: mvpPresetCatalog,
        })),
      },
      presetSelectionSettingsService: {
        loadLastUsedPresetId: vi.fn(async () => null),
        saveLastUsedPresetId: vi.fn(async () => undefined),
      },
      readinessAdapter,
      sessionTimingService: createSessionTimingService('session-2c', 'C:/sessions/김보라1111/session.json'),
    })

    const user = userEvent.setup()
    await user.type(screen.getByLabelText('세션 이름'), '김보라1111')
    await user.keyboard('{Enter}')

    expect(
      await screen.findByRole('heading', { name: '원하는 프리셋을 눌러 주세요.' }, { timeout: 3000 }),
    ).toBeInTheDocument()
    expect(screen.getByRole('list', { name: '프리셋 목록' })).toBeInTheDocument()
    expect(screen.getAllByRole('button').some((button) => button.textContent?.includes('직원에게'))).toBe(false)
    expect(screen.getByRole('button', { name: '이 프리셋으로 계속' })).toBeDisabled()
  })

  it('keeps the active preset through a ready-waiting-ready regression without returning to preset selection', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter(
      {
        sessionId: 'session-3',
        connectionState: 'ready',
        captureEnabled: true,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:05.000Z',
      },
      {
        presetId: 'background-pink',
        label: '배경지 - 핑크',
      },
    )
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-3',
          sessionName: '김보라7777',
          sessionFolder: 'C:/sessions/김보라7777',
          manifestPath: 'C:/sessions/김보라7777/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }
    const presetSelectionService = {
      selectPreset: vi.fn(async () => ({
        ok: true as const,
        value: {
          manifestPath: 'C:/sessions/김보라7777/session.json',
          updatedAt: '2026-03-08T00:00:06.000Z',
          activePreset: {
            presetId: 'background-pink',
            displayName: '배경지 - 핑크',
          },
        },
      })),
    }

    render(
      <BranchConfigContext.Provider
        value={{
          status: 'ready',
          config: {
            branchPhoneNumber: '010-1234-5678',
            operationalToggles: {
              enablePhoneEscalation: false,
            },
          },
        }}
      >
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={lifecycleService}
          presetSelectionService={presetSelectionService}
          sessionTimingService={createSessionTimingService('session-3', 'C:/sessions/김보라7777/session.json')}
        >
          <FlowStateProbe />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('preset-selection')
    }, { timeout: 4000 })

    await user.click(screen.getByRole('button', { name: '핑크 선택' }))
    await waitFor(() => {
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')
    })
    await user.click(screen.getByRole('button', { name: '프리셋 확정' }))

    await waitFor(() => {
      expect(readinessAdapter.watchCaptureConfidence).toHaveBeenCalled()
    })
    await act(async () => {
      readinessAdapter.emitCaptureConfidence({
        sessionId: 'session-3',
        revision: 1,
        updatedAt: '2026-03-08T00:00:07.000Z',
        shootEndsAt: '2099-03-08T00:20:00.000Z',
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        latestPhoto: {
          kind: 'empty',
        },
      })
    })
    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
    })
    expect(screen.getByTestId('active-preset')).toHaveTextContent('배경지 - 핑크')

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-3',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:09.000Z',
      })
    })

    expect(screen.getByTestId('phase')).not.toHaveTextContent('preset-selection')
    expect(screen.getByTestId('active-preset')).toHaveTextContent('배경지 - 핑크')
    expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-3',
        connectionState: 'ready',
        captureEnabled: true,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:12.000Z',
      })
    })

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
    })
    expect(screen.getByTestId('active-preset')).toHaveTextContent('배경지 - 핑크')
    expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')

    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 1300))
    })

    expect(screen.getByTestId('phase')).toHaveTextContent('capture-ready')
  })

  it('keeps the typed host failure envelope and shows restart guidance for session-integrity confirm failures', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-4',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-4',
          sessionName: '김보라8888',
          sessionFolder: 'C:/sessions/김보라8888',
          manifestPath: 'C:/sessions/김보라8888/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }
    const presetSelectionService: PresetSelectionService = {
      selectPreset: vi.fn(async () => ({
        ok: false as const,
        errorCode: 'session.preset_selection_invalid_session' as const,
        message: 'The preset selection could not be completed.',
      })),
    }

    render(
      <BranchConfigContext.Provider
        value={{
          status: 'ready',
          config: {
            branchPhoneNumber: '010-1234-5678',
            operationalToggles: {
              enablePhoneEscalation: false,
            },
          },
        }}
      >
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={lifecycleService}
          presetSelectionService={presetSelectionService}
          sessionTimingService={createSessionTimingService('session-4', 'C:/sessions/김보라8888/session.json')}
        >
          <FlowStateProbe />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('preset-selection')
    }, { timeout: 4000 })

    await user.click(screen.getByRole('button', { name: '핑크 선택' }))
    await waitFor(() => {
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')
    })
    await user.click(screen.getByRole('button', { name: '프리셋 확정' }))

    await waitFor(() => {
      expect(presetSelectionService.selectPreset).toHaveBeenCalledWith({
        presetId: 'background-pink',
        sessionId: 'session-4',
      })
    }, { timeout: 4000 })

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('preset-selection')
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')
      expect(screen.getByTestId('preset-selection-error-code')).toHaveTextContent(
        'session.preset_selection_invalid_session',
      )
      expect(screen.getByTestId('preset-selection-feedback')).toHaveTextContent(
        '세션을 다시 확인해 주세요. 문제가 계속되면 처음부터 다시 시작해 주세요.',
      )
    })
  })

  it('uses the typed preset-selection error code instead of parsing message text', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-4b',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-4b',
          sessionName: '김보라8889',
          sessionFolder: 'C:/sessions/김보라8889',
          manifestPath: 'C:/sessions/김보라8889/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }
    const presetSelectionService: PresetSelectionService = {
      selectPreset: vi.fn(async () => ({
        ok: false,
        errorCode: 'session.preset_selection_session_not_found',
        message: 'The preset selection could not be completed.',
      })),
    }

    render(
      <BranchConfigContext.Provider
        value={{
          status: 'ready',
          config: {
            branchPhoneNumber: '010-1234-5678',
            operationalToggles: {
              enablePhoneEscalation: false,
            },
          },
        }}
      >
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={lifecycleService}
          presetSelectionService={presetSelectionService}
          sessionTimingService={createSessionTimingService('session-4b', 'C:/sessions/김보라8889/session.json')}
        >
          <FlowStateProbe />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('preset-selection')
    }, { timeout: 4000 })

    await user.click(screen.getByRole('button', { name: '핑크 선택' }))
    await waitFor(() => {
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')
    })
    await user.click(screen.getByRole('button', { name: '프리셋 확정' }))

    await waitFor(() => {
      expect(screen.getByTestId('preset-selection-error-code')).toHaveTextContent(
        'session.preset_selection_session_not_found',
      )
      expect(screen.getByTestId('preset-selection-feedback')).toHaveTextContent(
        '세션을 다시 확인해 주세요. 문제가 계속되면 처음부터 다시 시작해 주세요.',
      )
    })
  })

  it('ignores an in-flight preset confirm result after the active session is cleared', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-5',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:05.000Z',
    })
    const lifecycleService = {
      startSession: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-5',
          sessionName: '김보라9999',
          sessionFolder: 'C:/sessions/김보라9999',
          manifestPath: 'C:/sessions/김보라9999/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing' as const,
        },
      })),
    }
    let resolveSelectPreset:
      | ((result: {
          ok: true
          value: {
            manifestPath: string
            updatedAt: string
            activePreset: {
              presetId: 'background-pink'
              displayName: '배경지 - 핑크'
            }
          }
        }) => void)
      | undefined
    const presetSelectionService = {
      selectPreset: vi.fn(
        () =>
          new Promise<{
            ok: true
            value: {
              manifestPath: string
              updatedAt: string
              activePreset: {
                presetId: 'background-pink'
                displayName: '배경지 - 핑크'
              }
            }
          }>((resolve) => {
            resolveSelectPreset = resolve
          }),
      ),
    }
    const presetSelectionSettingsService = {
      loadLastUsedPresetId: vi.fn(async () => 'background-ivory'),
      saveLastUsedPresetId: vi.fn(async () => undefined),
    }

    render(
      <BranchConfigContext.Provider
        value={{
          status: 'ready',
          config: {
            branchPhoneNumber: '010-1234-5678',
            operationalToggles: {
              enablePhoneEscalation: false,
            },
          },
        }}
      >
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={lifecycleService}
          presetSelectionService={presetSelectionService}
          presetSelectionSettingsService={presetSelectionSettingsService}
          sessionTimingService={createSessionTimingService('session-5', 'C:/sessions/김보라9999/session.json')}
        >
          <FlowStateProbe />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('preset-selection')
      expect(screen.getByTestId('session-id')).toHaveTextContent('session-5')
    }, { timeout: 4000 })

    await user.click(screen.getByRole('button', { name: '핑크 선택' }))
    await waitFor(() => {
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('background-pink')
    })
    await user.click(screen.getByRole('button', { name: '프리셋 확정' }))

    await waitFor(() => {
      expect(presetSelectionService.selectPreset).toHaveBeenCalledWith({
        presetId: 'background-pink',
        sessionId: 'session-5',
      })
    })

    await user.click(screen.getByRole('button', { name: '세션 초기화' }))

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('start')
      expect(screen.getByTestId('session-id')).toHaveTextContent('none')
      expect(screen.getByTestId('active-preset')).toHaveTextContent('none')
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('none')
    })

    await act(async () => {
      resolveSelectPreset?.({
        ok: true,
        value: {
          manifestPath: 'C:/sessions/김보라9999/session.json',
          updatedAt: '2026-03-08T00:00:06.000Z',
          activePreset: {
            presetId: 'background-pink',
            displayName: '배경지 - 핑크',
          },
        },
      })
      await Promise.resolve()
    })

    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('start')
      expect(screen.getByTestId('session-id')).toHaveTextContent('none')
      expect(screen.getByTestId('active-preset')).toHaveTextContent('none')
      expect(screen.getByTestId('selected-preset')).toHaveTextContent('none')
    })
    expect(presetSelectionSettingsService.saveLastUsedPresetId).not.toHaveBeenCalled()
  })
})
