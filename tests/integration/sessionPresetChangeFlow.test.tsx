import { useEffect, useRef } from 'react'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { within } from '@testing-library/react'

vi.mock('../../src/session-domain/services/activePresetService.js', () => ({
  activePresetService: {
    applyPresetChange: vi.fn(),
  },
}))

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { CustomerFlowContent } from '../../src/customer-flow/screens/CustomerFlowScreen.js'
import { activePresetService } from '../../src/session-domain/services/activePresetService.js'
import { schemaVersions } from '../../src/shared-contracts/dto/schemaVersion.js'
import type { PresetId } from '../../src/shared-contracts/presets/presetCatalog.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'

function createFutureIso(minutesFromNow: number) {
  return new Date(Date.now() + minutesFromNow * 60_000).toISOString()
}

function createDeferredResult() {
  let resolve: ((value: { sessionId: string; activePresetId: PresetId; appliedAt: string }) => void) | undefined
  const promise = new Promise<{ sessionId: string; activePresetId: PresetId; appliedAt: string }>((nextResolve) => {
    resolve = nextResolve
  })

  return {
    promise,
    resolve(value: { sessionId: string; activePresetId: PresetId; appliedAt: string }) {
      resolve?.(value)
    },
  }
}

function createCameraAdapter() {
  const shootEndsAt = createFutureIso(30)

  return {
    getReadinessSnapshot: vi.fn(async () => ({
      sessionId: 'session-32',
      connectionState: 'ready' as const,
      captureEnabled: true,
      lastStableCustomerState: 'ready' as const,
      error: null,
      emittedAt: '2026-03-08T10:00:05.000Z',
    })),
    watchReadiness: vi.fn(async () => () => undefined),
    getCaptureConfidenceSnapshot: vi.fn(async () => ({
      sessionId: 'session-32',
      revision: 1,
      updatedAt: '2026-03-08T10:00:06.000Z',
      shootEndsAt,
      activePreset: {
        presetId: 'warm-tone',
        label: '웜톤',
      },
      latestPhoto: {
        kind: 'ready' as const,
        photo: {
          sessionId: 'session-32',
          captureId: 'capture-002',
          sequence: 2,
          assetUrl: 'asset://session-32/capture-002',
          capturedAt: '2026-03-08T10:06:00.000Z',
        },
      },
    })),
    watchCaptureConfidence: vi.fn(async () => () => undefined),
    requestCapture: vi.fn(async () => undefined),
  }
}

function createCaptureAdapter() {
  const shootEndsAt = createFutureIso(30)

  return {
    loadSessionGallery: vi.fn(async () => ({
      schemaVersion: schemaVersions.contract,
      sessionId: 'session-32',
      sessionName: '김보라1234',
      shootEndsAt,
      activePresetName: '웜톤',
      latestCaptureId: 'capture-002',
      selectedCaptureId: 'capture-001',
      items: [
        {
          captureId: 'capture-001',
          sessionId: 'session-32',
          capturedAt: '2026-03-08T10:05:00.000Z',
          displayOrder: 0,
          isLatest: false,
          previewPath: 'asset://session-32/capture-001-preview',
          thumbnailPath: 'asset://session-32/capture-001-thumb',
          label: '첫 번째 촬영',
        },
        {
          captureId: 'capture-002',
          sessionId: 'session-32',
          capturedAt: '2026-03-08T10:06:00.000Z',
          displayOrder: 1,
          isLatest: true,
          previewPath: 'asset://session-32/capture-002-preview',
          thumbnailPath: 'asset://session-32/capture-002-thumb',
          label: '두 번째 촬영',
        },
      ],
    })),
    deleteSessionPhoto: vi.fn(async () => ({
      schemaVersion: schemaVersions.contract,
      deletedCaptureId: 'capture-001',
      confirmationMessage: '사진이 삭제되었습니다.' as const,
      gallery: {
        schemaVersion: schemaVersions.contract,
        sessionId: 'session-32',
        sessionName: '김보라1234',
        shootEndsAt,
        activePresetName: '웜톤',
        latestCaptureId: 'capture-002',
        selectedCaptureId: 'capture-002',
        items: [
          {
            captureId: 'capture-002',
            sessionId: 'session-32',
            capturedAt: '2026-03-08T10:06:00.000Z',
            displayOrder: 0,
            isLatest: true,
            previewPath: 'asset://session-32/capture-002-preview',
            thumbnailPath: 'asset://session-32/capture-002-thumb',
            label: '두 번째 촬영',
          },
        ],
      },
    })),
  }
}

function createLifecycleService() {
  return {
    startSession: vi.fn(async () => ({
      ok: true as const,
      value: {
        sessionId: 'session-32',
        sessionName: '김보라1234',
        sessionFolder: 'C:/sessions/김보라1234',
        manifestPath: 'C:/sessions/김보라1234/session.json',
        createdAt: '2026-03-08T10:00:00.000Z',
        preparationState: 'preparing' as const,
      },
    })),
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

function createPresetSelectionSettingsService() {
  return {
    loadLastUsedPresetId: vi.fn(async () => 'warm-tone'),
    saveLastUsedPresetId: vi.fn(async () => undefined),
  }
}

function createSessionTimingService() {
  const actualShootEndAt = createFutureIso(30)

  return {
    initializeSessionTiming: vi.fn(),
    getSessionTiming: vi.fn(async () => ({
      ok: true as const,
      value: {
        sessionId: 'session-32',
        manifestPath: 'C:/sessions/김보라1234/session.json',
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt,
          sessionType: 'standard' as const,
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:00:00.000Z',
        },
      },
    })),
    extendSessionTiming: vi.fn(),
  }
}

function CustomerFlowBootstrapper() {
  const { confirmPresetSelection, selectPreset, startJourney, state, submitCheckIn, updateField } = useSessionFlow()
  const journeyStartedRef = useRef(false)
  const sessionNameSeededRef = useRef(false)
  const checkInSubmittedRef = useRef(false)
  const presetSelectedRef = useRef(false)
  const presetConfirmedRef = useRef(false)

  useEffect(() => {
    if (journeyStartedRef.current || state.phase !== 'start') {
      return
    }

    journeyStartedRef.current = true
    startJourney()
  }, [startJourney, state.phase])

  useEffect(() => {
    if (sessionNameSeededRef.current || state.phase !== 'idle') {
      return
    }

    sessionNameSeededRef.current = true
    updateField('sessionName', '김보라1234')
  }, [state.phase, updateField])

  useEffect(() => {
    if (checkInSubmittedRef.current || state.phase !== 'idle' || state.fields.sessionName !== '김보라1234') {
      return
    }

    checkInSubmittedRef.current = true
    void submitCheckIn()
  }, [state.fields.sessionName, state.phase, submitCheckIn])

  useEffect(() => {
    if (presetSelectedRef.current || state.phase !== 'preset-selection') {
      return
    }

    presetSelectedRef.current = true
    void selectPreset('warm-tone')
  }, [selectPreset, state.phase])

  useEffect(() => {
    if (
      presetConfirmedRef.current ||
      state.phase !== 'preset-selection' ||
      state.selectedPresetId !== 'warm-tone' ||
      state.presetSelectionStatus !== 'idle'
    ) {
      return
    }

    presetConfirmedRef.current = true
    void confirmPresetSelection()
  }, [confirmPresetSelection, state.phase, state.presetSelectionStatus, state.selectedPresetId])

  return null
}

function renderCustomerFlow() {
  const cameraAdapter = createCameraAdapter()
  const captureAdapter = createCaptureAdapter()
  const lifecycleService = createLifecycleService()
  const presetSelectionService = createPresetSelectionService()
  const presetSelectionSettingsService = createPresetSelectionSettingsService()
  const sessionTimingService = createSessionTimingService()
  const lifecycleLogger = {
    recordReadinessReached: vi.fn(async () => undefined),
  }
  const postEndOutcomeService = {
    getPostEndOutcome: vi.fn(async () => ({
      ok: true as const,
      value: {
        kind: 'completed' as const,
      },
    })),
  }
  const timingAlertAudio = {
    play: vi.fn(async () => undefined),
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
        captureAdapter={captureAdapter}
        lifecycleLogger={lifecycleLogger}
        lifecycleService={lifecycleService}
        postEndOutcomeService={postEndOutcomeService}
        presetSelectionService={presetSelectionService}
        presetSelectionSettingsService={presetSelectionSettingsService}
        sessionTimingService={sessionTimingService}
        timingAlertAudio={timingAlertAudio}
      >
        <CustomerFlowBootstrapper />
        <CustomerFlowContent />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  )

  return {
    captureAdapter,
    user: userEvent.setup(),
  }
}

describe('session preset change flow', () => {
  it('keeps review and latest-photo context stable while reconciling to the host-confirmed preset', async () => {
    vi.mocked(activePresetService.applyPresetChange).mockResolvedValue({
      sessionId: 'session-32',
      activePresetId: 'background-ivory',
      appliedAt: '2026-03-08T10:00:09.000Z',
    })

    const { captureAdapter, user } = renderCustomerFlow()

    await screen.findByRole('button', { name: '촬영하기' }, { timeout: 7000 })
    await waitFor(() => {
      expect(captureAdapter.loadSessionGallery).toHaveBeenCalledWith({
        sessionId: 'session-32',
        manifestPath: 'C:/sessions/김보라1234/session.json',
      })
    })
    expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
      'src',
      'asset://session-32/capture-002',
    )

    await user.click(screen.getByRole('button', { name: '프리셋 변경' }))
    await user.click(await screen.findByRole('button', { name: /배경지 - 핑크/i }, { timeout: 4000 }))

    await waitFor(() => {
      expect(screen.queryByRole('dialog', { name: '프리셋 변경' })).not.toBeInTheDocument()
      expect(screen.getByText('배경지 - 아이보리')).toBeInTheDocument()
      expect(screen.getByRole('status')).toHaveTextContent('다음 촬영부터 적용됩니다.')
      expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
        'src',
        'asset://session-32/capture-002',
      )
    })

    expect(activePresetService.applyPresetChange).toHaveBeenCalledWith({
      sessionId: 'session-32',
      presetId: 'background-pink',
    })
  }, 10000)

  it('prevents a second in-session preset write while the first change is still pending', async () => {
    const deferredResult = createDeferredResult()
    vi.mocked(activePresetService.applyPresetChange).mockReturnValue(deferredResult.promise)

    const { user } = renderCustomerFlow()

    await screen.findByRole('button', { name: '촬영하기' }, { timeout: 7000 })

    await user.click(screen.getByRole('button', { name: '프리셋 변경' }))
    await user.click(await screen.findByRole('button', { name: /배경지 - 핑크/i }, { timeout: 4000 }))

    await waitFor(() => {
      expect(activePresetService.applyPresetChange).toHaveBeenCalledTimes(1)
    })

    await waitFor(() => {
      expect(screen.getByRole('button', { name: '촬영하기' })).toBeDisabled()
      expect(screen.getByRole('button', { name: '프리셋 변경' })).toBeDisabled()
      expect(screen.getByRole('button', { name: '닫기' })).toBeDisabled()
      expect(screen.getByRole('button', { name: /배경지 - 핑크/i })).toBeDisabled()
      expect(screen.getByRole('button', { name: /배경지 - 아이보리/i })).toBeDisabled()
    })

    await user.click(screen.getByRole('button', { name: /배경지 - 아이보리/i }))

    expect(activePresetService.applyPresetChange).toHaveBeenCalledTimes(1)

    deferredResult.resolve({
      sessionId: 'session-32',
      activePresetId: 'background-pink',
      appliedAt: '2026-03-08T10:00:09.000Z',
    })

    await waitFor(() => {
      expect(screen.queryByRole('dialog', { name: '프리셋 변경' })).not.toBeInTheDocument()
      expect(screen.getByText('배경지 - 핑크')).toBeInTheDocument()
    })
  }, 10000)

  it('keeps the preset sheet open and shows customer-safe retry guidance when an in-session preset change fails', async () => {
    vi.mocked(activePresetService.applyPresetChange).mockRejectedValue(new Error('native manifest path drift'))

    const { user } = renderCustomerFlow()

    await screen.findByRole('button', { name: '촬영하기' }, { timeout: 7000 })

    await user.click(screen.getByRole('button', { name: '프리셋 변경' }))
    await user.click(await screen.findByRole('button', { name: /배경지 - 핑크/i }, { timeout: 4000 }))

    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: '프리셋 변경' })).toBeInTheDocument()
      expect(screen.getByRole('status')).toHaveTextContent('프리셋을 적용하지 못했어요. 다시 선택해 주세요.')
      expect(within(screen.getByLabelText('현재 프리셋')).getByText('웜톤')).toBeInTheDocument()
    })

    expect(activePresetService.applyPresetChange).toHaveBeenCalledWith({
      sessionId: 'session-32',
      presetId: 'background-pink',
    })
  }, 10000)
})
