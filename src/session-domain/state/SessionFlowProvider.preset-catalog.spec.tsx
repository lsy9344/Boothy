import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { BranchConfigContext } from '../../branch-config/BranchConfigContext.js'
import { approvedBoothPresetCatalog } from '../../preset-catalog/services/presetCatalogService.js'
import { SessionFlowProvider, useSessionFlow } from './SessionFlowProvider.js'

function JourneyControls() {
  const { startJourney, state, submitCheckIn, updateField } = useSessionFlow()

  return (
    <div>
      <button
        onClick={() => {
          startJourney('김보라1234')
        }}
        type="button"
      >
        세션 시작
      </button>
      <button
        onClick={() => {
          updateField('sessionName', '김보라1234')
        }}
        type="button"
      >
        세션명 입력
      </button>
      <button
        onClick={() => {
          void submitCheckIn()
        }}
        type="button"
      >
        체크인 제출
      </button>
      <output data-testid="phase">{state.phase}</output>
      <output data-testid="session-name">{state.fields.sessionName}</output>
      <output data-testid="active-session">{state.activeSession?.sessionId ?? 'none'}</output>
    </div>
  )
}

function renderProvider(lifecycleLogger: {
  recordReadinessReached: ReturnType<typeof vi.fn>
  recordPresetCatalogFallback: ReturnType<typeof vi.fn>
}, presetSelectionSettingsService?: {
  loadLastUsedPresetId: ReturnType<typeof vi.fn>
  saveLastUsedPresetId: ReturnType<typeof vi.fn>
}, presetCatalogState = {
  status: 'ready' as const,
  source: 'approved-fallback' as const,
  auditReason: 'reordered_catalog' as const,
  presets: approvedBoothPresetCatalog,
}) {
  const lifecycleService = {
    startSession: vi.fn(async () => ({
      ok: true as const,
      value: {
        sessionId: 'session-42',
        sessionName: '김보라1234',
        sessionFolder: 'C:/sessions/김보라1234',
        manifestPath: 'C:/sessions/김보라1234/session.json',
        createdAt: '2026-03-13T00:00:00.000Z',
        preparationState: 'preparing' as const,
      },
    })),
  }

  const rendered = render(
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
        cameraAdapter={{
          getReadinessSnapshot: vi.fn(async () => ({
            sessionId: 'session-42',
            connectionState: 'ready' as const,
            captureEnabled: true,
            lastStableCustomerState: 'ready' as const,
            error: null,
            emittedAt: '2026-03-13T00:00:01.000Z',
          })),
          watchReadiness: vi.fn(async () => () => undefined),
          getCaptureConfidenceSnapshot: vi.fn(async () => ({
            sessionId: 'session-42',
            revision: 0,
            updatedAt: '2026-03-13T00:00:01.000Z',
            shootEndsAt: '2099-03-13T09:50:00.000Z',
            activePreset: {
              presetId: 'warm-tone',
              label: '웜톤',
            },
            latestPhoto: {
              kind: 'empty' as const,
            },
          })),
          watchCaptureConfidence: vi.fn(async () => () => undefined),
        }}
        lifecycleLogger={lifecycleLogger}
        lifecycleService={lifecycleService}
        presetCatalogService={{
          loadApprovedPresetCatalog: vi.fn(async () => presetCatalogState),
        }}
        presetSelectionSettingsService={
          presetSelectionSettingsService ?? {
            loadLastUsedPresetId: vi.fn(async () => null),
            saveLastUsedPresetId: vi.fn(async () => undefined),
          }
        }
        sessionTimingService={{
          initializeSessionTiming: vi.fn(),
          getSessionTiming: vi.fn(async () => ({
            ok: true as const,
            value: {
              sessionId: 'session-42',
              manifestPath: 'C:/sessions/김보라1234/session.json',
              timing: {
                reservationStartAt: '2026-03-13T09:00:00.000Z',
                actualShootEndAt: '2099-03-13T09:50:00.000Z',
                sessionType: 'standard' as const,
                operatorExtensionCount: 0,
                lastTimingUpdateAt: '2026-03-13T09:00:00.000Z',
              },
            },
          })),
          extendSessionTiming: vi.fn(),
        }}
      >
        <JourneyControls />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  )

  return {
    lifecycleService,
    rendered,
    user: userEvent.setup(),
  }
}

describe('SessionFlowProvider preset catalog audit logging', () => {
  it('records one preset catalog fallback audit event as soon as the runtime catalog resolves to an approved fallback', async () => {
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
      recordPresetCatalogFallback: vi.fn(async () => undefined),
    }

    renderProvider(lifecycleLogger)

    await waitFor(() => {
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledTimes(1)
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledWith(
        expect.objectContaining({
          branchId: 'gangnam-main',
          reason: 'reordered_catalog',
        }),
      )
    })
  })

  it('records one preset catalog fallback audit event when the verified catalog blocks customer selection', async () => {
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
      recordPresetCatalogFallback: vi.fn(async () => undefined),
    }
    const { lifecycleService, user } = renderProvider(lifecycleLogger)

    await user.click(screen.getByRole('button', { name: '세션 시작' }))
    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('idle')
    })
    await user.click(screen.getByRole('button', { name: '세션명 입력' }))
    await waitFor(() => {
      expect(screen.getByTestId('session-name')).toHaveTextContent('김보라1234')
    })
    await user.click(screen.getByRole('button', { name: '체크인 제출' }))

    await waitFor(() => {
      expect(lifecycleService.startSession).toHaveBeenCalledTimes(1)
      expect(screen.getByTestId('active-session')).toHaveTextContent('session-42')
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledTimes(1)
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledWith(
        expect.objectContaining({
          branchId: 'gangnam-main',
          reason: 'reordered_catalog',
        }),
      )
    })
  })

  it('records the same fallback reason again after the catalog recovers and drifts later', async () => {
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
      recordPresetCatalogFallback: vi.fn(async () => undefined),
    }
    const healthyCatalogState = {
      status: 'ready' as const,
      presets: approvedBoothPresetCatalog,
      source: 'approved' as const,
    }
    const fallbackCatalogState = {
      status: 'ready' as const,
      presets: approvedBoothPresetCatalog,
      source: 'approved-fallback' as const,
      auditReason: 'reordered_catalog' as const,
    }

    const { rendered } = renderProvider(lifecycleLogger, undefined, fallbackCatalogState)

    await waitFor(() => {
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledTimes(1)
    })

    rendered.rerender(
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
          cameraAdapter={{
            getReadinessSnapshot: vi.fn(async () => ({
              sessionId: 'session-42',
              connectionState: 'ready' as const,
              captureEnabled: true,
              lastStableCustomerState: 'ready' as const,
              error: null,
              emittedAt: '2026-03-13T00:00:01.000Z',
            })),
            watchReadiness: vi.fn(async () => () => undefined),
            getCaptureConfidenceSnapshot: vi.fn(async () => ({
              sessionId: 'session-42',
              revision: 0,
              updatedAt: '2026-03-13T00:00:01.000Z',
              shootEndsAt: '2099-03-13T09:50:00.000Z',
              activePreset: {
                presetId: 'warm-tone',
                label: '웜톤',
              },
              latestPhoto: {
                kind: 'empty' as const,
              },
            })),
            watchCaptureConfidence: vi.fn(async () => () => undefined),
          }}
          lifecycleLogger={lifecycleLogger}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-42',
                sessionName: '김보라1234',
                sessionFolder: 'C:/sessions/김보라1234',
                manifestPath: 'C:/sessions/김보라1234/session.json',
                createdAt: '2026-03-13T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          presetCatalogService={{
            loadApprovedPresetCatalog: vi.fn(async () => healthyCatalogState),
          }}
          presetSelectionSettingsService={{
            loadLastUsedPresetId: vi.fn(async () => null),
            saveLastUsedPresetId: vi.fn(async () => undefined),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-42',
                manifestPath: 'C:/sessions/김보라1234/session.json',
                timing: {
                  reservationStartAt: '2026-03-13T09:00:00.000Z',
                  actualShootEndAt: '2099-03-13T09:50:00.000Z',
                  sessionType: 'standard' as const,
                  operatorExtensionCount: 0,
                  lastTimingUpdateAt: '2026-03-13T09:00:00.000Z',
                },
              },
            })),
            extendSessionTiming: vi.fn(),
          }}
        >
          <JourneyControls />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await waitFor(() => {
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledTimes(1)
    })

    rendered.rerender(
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
          cameraAdapter={{
            getReadinessSnapshot: vi.fn(async () => ({
              sessionId: 'session-42',
              connectionState: 'ready' as const,
              captureEnabled: true,
              lastStableCustomerState: 'ready' as const,
              error: null,
              emittedAt: '2026-03-13T00:00:01.000Z',
            })),
            watchReadiness: vi.fn(async () => () => undefined),
            getCaptureConfidenceSnapshot: vi.fn(async () => ({
              sessionId: 'session-42',
              revision: 0,
              updatedAt: '2026-03-13T00:00:01.000Z',
              shootEndsAt: '2099-03-13T09:50:00.000Z',
              activePreset: {
                presetId: 'warm-tone',
                label: '웜톤',
              },
              latestPhoto: {
                kind: 'empty' as const,
              },
            })),
            watchCaptureConfidence: vi.fn(async () => () => undefined),
          }}
          lifecycleLogger={lifecycleLogger}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-42',
                sessionName: '김보라1234',
                sessionFolder: 'C:/sessions/김보라1234',
                manifestPath: 'C:/sessions/김보라1234/session.json',
                createdAt: '2026-03-13T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          presetCatalogService={{
            loadApprovedPresetCatalog: vi.fn(async () => fallbackCatalogState),
          }}
          presetSelectionSettingsService={{
            loadLastUsedPresetId: vi.fn(async () => null),
            saveLastUsedPresetId: vi.fn(async () => undefined),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-42',
                manifestPath: 'C:/sessions/김보라1234/session.json',
                timing: {
                  reservationStartAt: '2026-03-13T09:00:00.000Z',
                  actualShootEndAt: '2099-03-13T09:50:00.000Z',
                  sessionType: 'standard' as const,
                  operatorExtensionCount: 0,
                  lastTimingUpdateAt: '2026-03-13T09:00:00.000Z',
                },
              },
            })),
            extendSessionTiming: vi.fn(),
          }}
        >
          <JourneyControls />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await waitFor(() => {
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledTimes(2)
    })
  })

  it('does not double-log the same fallback before and after the active session is created', async () => {
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
      recordPresetCatalogFallback: vi.fn(async () => undefined),
    }
    const { user } = renderProvider(lifecycleLogger)

    await user.click(screen.getByRole('button', { name: '세션 시작' }))
    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('idle')
    })
    await user.click(screen.getByRole('button', { name: '세션명 입력' }))
    await waitFor(() => {
      expect(screen.getByTestId('session-name')).toHaveTextContent('김보라1234')
    })
    await user.click(screen.getByRole('button', { name: '체크인 제출' }))

    await waitFor(() => {
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledTimes(1)
      expect(lifecycleLogger.recordPresetCatalogFallback).toHaveBeenCalledWith(
        expect.objectContaining({
          branchId: 'gangnam-main',
          reason: 'reordered_catalog',
        }),
      )
    })
  })

  it('normalizes a stale last-used preset id to the approved default without auto-selecting it', async () => {
    const lifecycleLogger = {
      recordReadinessReached: vi.fn(async () => undefined),
      recordPresetCatalogFallback: vi.fn(async () => undefined),
    }
    const presetSelectionSettingsService = {
      loadLastUsedPresetId: vi.fn(async () => 'not-approved'),
      saveLastUsedPresetId: vi.fn(async () => undefined),
    }

    const { user } = renderProvider(
      lifecycleLogger,
      presetSelectionSettingsService,
      {
        status: 'ready',
        presets: approvedBoothPresetCatalog,
        source: 'approved',
      },
    )

    await user.click(screen.getByRole('button', { name: '세션 시작' }))
    await waitFor(() => {
      expect(screen.getByTestId('phase')).toHaveTextContent('idle')
    })
    await user.click(screen.getByRole('button', { name: '세션명 입력' }))
    await waitFor(() => {
      expect(screen.getByTestId('session-name')).toHaveTextContent('김보라1234')
    })
    await user.click(screen.getByRole('button', { name: '체크인 제출' }))

    await waitFor(() => {
      expect(presetSelectionSettingsService.loadLastUsedPresetId).toHaveBeenCalledTimes(1)
      expect(presetSelectionSettingsService.saveLastUsedPresetId).toHaveBeenCalledWith('warm-tone')
    })
  })
})
