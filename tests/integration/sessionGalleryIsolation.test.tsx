import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useEffect, useRef } from 'react'

function createFutureIso(minutesFromNow: number) {
  return new Date(Date.now() + minutesFromNow * 60_000).toISOString()
}

const mocks = vi.hoisted(() => {
  const shootEndsAt = createFutureIso(30)
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
        latestPhoto:
          | {
              kind: 'empty'
            }
          | {
              kind: 'ready'
              photo: {
                sessionId: string
                captureId: string
                sequence: number
                assetUrl: string
                capturedAt: string
              }
            }
      }) => void)
    | null = null

  const readinessSnapshot = {
    sessionId: 'session-24',
    connectionState: 'ready' as const,
    captureEnabled: true,
    lastStableCustomerState: 'ready' as const,
    error: null,
    emittedAt: '2026-03-08T10:00:05.000Z',
  }

  const createInitialCaptureConfidence = () => ({
    sessionId: 'session-24',
    revision: 1,
    updatedAt: '2026-03-08T10:00:06.000Z',
    shootEndsAt,
    activePreset: {
      presetId: 'background-pink',
      label: '배경지 - 핑크',
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
  })

  let currentCaptureConfidence = createInitialCaptureConfidence()

  const createInitialGallery = () => ({
    schemaVersion: 'boothy.camera.contract.v1' as const,
    sessionId: 'session-24',
    sessionName: '김보라1234',
    shootEndsAt,
    activePresetName: '배경지 - 핑크',
    latestCaptureId: 'capture-002',
    selectedCaptureId: 'capture-002',
    items: [
      {
        captureId: 'capture-001',
        sessionId: 'session-24',
        capturedAt: '2026-03-08T10:05:00.000Z',
        displayOrder: 0,
        isLatest: false,
        previewPath: 'asset://session-24/capture-001',
        thumbnailPath: 'asset://session-24/thumb-capture-001',
        label: '첫 번째 사진',
      },
      {
        captureId: 'capture-002',
        sessionId: 'session-24',
        capturedAt: '2026-03-08T10:06:00.000Z',
        displayOrder: 1,
        isLatest: true,
        previewPath: 'asset://session-24/capture-002',
        thumbnailPath: 'asset://session-24/thumb-capture-002',
        label: '두 번째 사진',
      },
    ],
  })

  let currentGallery = createInitialGallery()

  const defaultRequestCapture = async (
    input: {
      requestId: string
      correlationId: string
      sessionId: string
      activePreset: {
        presetId: string
        label: string
      }
    },
    onProgress?: (event: {
      schemaVersion: string
      requestId: string
      correlationId: string
      event: 'capture.progress'
      sessionId: string
      payload: {
        stage: 'captureStarted' | 'captureCompleted'
        captureId: string
        percentComplete: number
        lastUpdatedAt: string
      }
    }) => void,
  ) => {
    onProgress?.({
      schemaVersion: 'boothy.camera.protocol.v1',
      requestId: input.requestId,
      correlationId: input.correlationId,
      event: 'capture.progress',
      sessionId: input.sessionId,
      payload: {
        stage: 'captureStarted',
        captureId: 'capture-003',
        percentComplete: 0,
        lastUpdatedAt: '2026-03-08T10:07:00.000Z',
      },
    })

    currentCaptureConfidence = {
      ...currentCaptureConfidence,
      revision: 2,
      updatedAt: '2026-03-08T10:07:02.000Z',
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-24',
          captureId: 'capture-003',
          sequence: 3,
          assetUrl: 'asset://session-24/capture-003',
          capturedAt: '2026-03-08T10:07:02.000Z',
        },
      },
    }
    currentGallery = {
      ...currentGallery,
      latestCaptureId: 'capture-003',
      selectedCaptureId: 'capture-003',
      items: [
        ...currentGallery.items.map((item) => ({
          ...item,
          isLatest: false,
        })),
        {
          captureId: 'capture-003',
          sessionId: 'session-24',
          capturedAt: '2026-03-08T10:07:02.000Z',
          displayOrder: currentGallery.items.length,
          isLatest: true,
          previewPath: 'asset://session-24/capture-003',
          thumbnailPath: 'asset://session-24/thumb-capture-003',
          label: '세 번째 사진',
        },
      ],
    }
    captureConfidenceHandler?.(currentCaptureConfidence)

    onProgress?.({
      schemaVersion: 'boothy.camera.protocol.v1',
      requestId: input.requestId,
      correlationId: input.correlationId,
      event: 'capture.progress',
      sessionId: input.sessionId,
      payload: {
        stage: 'captureCompleted',
        captureId: 'capture-003',
        percentComplete: 100,
        lastUpdatedAt: '2026-03-08T10:07:02.000Z',
      },
    })

    return {
      schemaVersion: 'boothy.camera.contract.v1' as const,
      requestId: input.requestId,
      correlationId: input.correlationId,
      ok: true,
      sessionId: input.sessionId,
      captureId: 'capture-003',
      capturedAt: '2026-03-08T10:07:02.000Z',
      manifestPath: 'C:/sessions/김보라1234/session.json',
    }
  }

  const defaultWatchCaptureConfidence = async ({
    onSnapshot,
  }: {
    onSnapshot: (snapshot: typeof currentCaptureConfidence) => void
  }) => {
    captureConfidenceHandler = onSnapshot

    return () => {
      captureConfidenceHandler = null
    }
  }

  const cameraAdapter = {
    getReadinessSnapshot: vi.fn(async () => readinessSnapshot),
    watchReadiness: vi.fn(async () => () => undefined),
    getCaptureConfidenceSnapshot: vi.fn(async () => currentCaptureConfidence),
    requestCapture: vi.fn(defaultRequestCapture),
    watchCaptureConfidence: vi.fn(defaultWatchCaptureConfidence),
  }

  const captureAdapter = {
    loadSessionGallery: vi.fn(async () => currentGallery),
    deleteSessionPhoto: vi.fn(async () => {
      throw new Error('not used in story 3.2 capture surface')
    }),
  }

  const lifecycleService = {
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

  const presetSelectionService = {
    selectPreset: vi.fn(async () => ({
      ok: true as const,
      value: {
        manifestPath: 'C:/sessions/김보라1234/session.json',
        updatedAt: '2026-03-08T10:00:07.000Z',
        activePreset: {
          presetId: 'background-pink',
          displayName: '배경지 - 핑크',
        },
      },
    })),
  }

  const presetSelectionSettingsService = {
    loadLastUsedPresetId: vi.fn(async () => 'background-pink'),
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
          actualShootEndAt: shootEndsAt,
          sessionType: 'standard' as const,
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:00:00.000Z',
        },
      },
    })),
    extendSessionTiming: vi.fn(),
  }

  return {
    cameraAdapter,
    captureAdapter,
    lifecycleService,
    presetSelectionService,
    presetSelectionSettingsService,
    lifecycleLogger,
    sessionTimingService,
    readinessSnapshot,
    reset() {
      captureConfidenceHandler = null
      currentCaptureConfidence = createInitialCaptureConfidence()
      currentGallery = createInitialGallery()
      cameraAdapter.getCaptureConfidenceSnapshot.mockReset().mockImplementation(async () => currentCaptureConfidence)
      cameraAdapter.requestCapture.mockReset().mockImplementation(defaultRequestCapture)
      cameraAdapter.watchCaptureConfidence.mockReset().mockImplementation(defaultWatchCaptureConfidence)
      captureAdapter.loadSessionGallery.mockReset().mockImplementation(async () => currentGallery)
    },
  }
})

beforeEach(() => {
  mocks.reset()
})

vi.mock('../../src/capture-adapter/host/cameraAdapter.js', () => ({
  cameraAdapter: mocks.cameraAdapter,
  createCameraAdapter: () => mocks.cameraAdapter,
  createFallbackCameraReadinessStatus: (sessionId: string) => ({
    ...mocks.readinessSnapshot,
    sessionId,
  }),
}))

vi.mock('../../src/capture-adapter/host/captureAdapter.js', () => ({
  captureAdapter: mocks.captureAdapter,
  createCaptureAdapter: () => mocks.captureAdapter,
}))

vi.mock('../../src/session-domain/services/sessionLifecycle.js', () => ({
  sessionLifecycleService: mocks.lifecycleService,
}))

vi.mock('../../src/session-domain/services/presetSelection.js', () => ({
  presetSelectionService: mocks.presetSelectionService,
  resolveDefaultPresetId: (lastUsedPresetId: string | null) => lastUsedPresetId ?? 'warm-tone',
}))

vi.mock('../../src/branch-config/services/presetSelectionStore.js', () => ({
  presetSelectionSettingsService: mocks.presetSelectionSettingsService,
}))

vi.mock('../../src/preset-catalog/services/presetCatalogService.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../src/preset-catalog/services/presetCatalogService.js')>()

  return {
    ...actual,
    presetCatalogService: {
      loadApprovedPresetCatalog: vi.fn(async () => ({
        status: 'ready' as const,
        presets: actual.approvedBoothPresetCatalog,
      })),
    },
  }
})

vi.mock('../../src/diagnostics-log/services/lifecycleLogger.js', () => ({
  createLifecycleLogger: () => mocks.lifecycleLogger,
}))

vi.mock('../../src/timing-policy/services/sessionTimingService.js', () => ({
  sessionTimingService: mocks.sessionTimingService,
}))

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { CustomerFlowContent } from '../../src/customer-flow/screens/CustomerFlowScreen.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'

function AutoStartJourney() {
  const { startJourney, state, submitCheckIn, updateField } = useSessionFlow()
  const startedRef = useRef(false)
  const sessionNameSeededRef = useRef(false)
  const submittedRef = useRef(false)
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

  return null
}

describe('session gallery isolation flow', () => {
  it('keeps the capture surface focused on latest-photo confidence even when session review data exists', async () => {
    const user = userEvent.setup()

    render(
      <BranchConfigContext.Provider
        value={{
          status: 'ready',
          config: {
            branchPhoneNumber: '010-1234-5678',
            operationalToggles: {
              enablePhoneEscalation: true,
            },
          },
        }}
      >
        <SessionFlowProvider>
          <AutoStartJourney />
          <CustomerFlowContent />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await screen.findByRole('button', { name: '이 프리셋으로 계속' }, { timeout: 5000 })
    await user.click(await screen.findByRole('button', { name: /배경지 - 핑크/i }, { timeout: 5000 }))
    await user.click(screen.getByRole('button', { name: '이 프리셋으로 계속' }))

    expect(await screen.findByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
      'src',
      'asset://session-24/capture-002',
    )

    await waitFor(() => {
      expect(mocks.captureAdapter.loadSessionGallery).toHaveBeenCalledWith({
        sessionId: 'session-24',
        manifestPath: 'C:/sessions/김보라1234/session.json',
      })
    })

    expect(screen.getByText('현재 프리셋')).toBeInTheDocument()
    expect(screen.queryByLabelText('세션 사진 썸네일')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 크게 보기' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 삭제' })).not.toBeInTheDocument()
  }, 10000)

  it('refreshes the latest-photo preview after capture while keeping review controls hidden', async () => {
    const user = userEvent.setup()

    render(
      <BranchConfigContext.Provider
        value={{
          status: 'ready',
          config: {
            branchPhoneNumber: '010-1234-5678',
            operationalToggles: {
              enablePhoneEscalation: true,
            },
          },
        }}
      >
        <SessionFlowProvider>
          <AutoStartJourney />
          <CustomerFlowContent />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await screen.findByRole('button', { name: '이 프리셋으로 계속' }, { timeout: 5000 })
    await user.click(await screen.findByRole('button', { name: /배경지 - 핑크/i }, { timeout: 5000 }))
    await user.click(screen.getByRole('button', { name: '이 프리셋으로 계속' }))

    await user.click(await screen.findByRole('button', { name: '촬영하기' }))

    await waitFor(() => {
      expect(mocks.cameraAdapter.requestCapture).toHaveBeenCalledWith(
        expect.objectContaining({
          sessionId: 'session-24',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
        }),
        expect.any(Function),
      )
    })

    await waitFor(() => {
      expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
        'src',
        'asset://session-24/capture-003',
      )
    })

    expect(screen.getByText('현재 프리셋')).toBeInTheDocument()
    expect(screen.queryByLabelText('세션 사진 썸네일')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 크게 보기' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 삭제' })).not.toBeInTheDocument()
  }, 10000)
})
