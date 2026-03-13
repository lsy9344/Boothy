import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { CustomerFlowContent } from '../../src/customer-flow/screens/CustomerFlowScreen.js'
import { createCameraAdapter } from '../../src/capture-adapter/host/cameraAdapter.js'
import { CheckInScreen } from '../../src/customer-flow/screens/CheckInScreen.js'
import { PreparationScreen } from '../../src/customer-flow/screens/PreparationScreen.js'
import { PresetSelectionSurface } from '../../src/customer-flow/screens/PresetSelectionSurface.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'
import { selectSessionTimeDisplay } from '../../src/timing-policy/selectors/sessionTimeDisplay.js'

function createReadinessAdapter(initialSnapshot: {
  sessionId: string
  connectionState: 'preparing' | 'waiting' | 'ready' | 'phone-required'
  captureEnabled: boolean
  lastStableCustomerState: 'preparing' | 'ready' | null
  error: {
    schemaVersion: 'boothy.camera.error-envelope.v1'
    code: string
    severity: 'info' | 'warning' | 'error' | 'critical'
    customerState: 'cameraReconnectNeeded' | 'cameraUnavailable'
    customerCameraConnectionState: 'connected' | 'needsAttention' | 'offline'
    operatorCameraConnectionState: 'connected' | 'reconnecting' | 'disconnected' | 'offline'
    operatorAction: 'checkCableAndRetry' | 'restartHelper' | 'contactSupport'
    retryable: boolean
    message: string
    details?: string
  } | null
  emittedAt: string
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
    activePreset: {
      presetId: 'background-ivory',
      label: '배경지 - 아이보리',
    },
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

function ReadinessFlowSurface() {
  const { confirmPresetSelection, continueFromPreparation, presetCatalogState, selectPreset, state } = useSessionFlow()

  if (state.phase === 'preset-selection' && state.activeSession) {
    return (
      <PresetSelectionSurface
        catalogState={presetCatalogState}
        isApplyingPreset={state.presetSelectionStatus === 'applying'}
        onConfirmPreset={confirmPresetSelection}
        onSelectPreset={selectPreset}
        selectionFeedback={state.presetSelectionFeedback}
        selectedPresetId={state.selectedPresetId}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={state.sessionTiming ? selectSessionTimeDisplay(state.sessionTiming.actualShootEndAt) : null}
      />
    )
  }

  if (state.activeSession && state.readiness) {
    return (
      <PreparationScreen
        onStartCapture={continueFromPreparation}
        readiness={state.readiness}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={state.sessionTiming ? selectSessionTimeDisplay(state.sessionTiming.actualShootEndAt) : null}
      />
    )
  }

  return <CheckInScreen />
}

describe('customer readiness flow', () => {
  it('loads the authoritative session timing once the active session is provisioned', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-timing-visible',
      connectionState: 'preparing',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })
    const sessionTimingService = {
      initializeSessionTiming: vi.fn(),
      getSessionTiming: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-timing-visible',
          manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-timing-visible',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={sessionTimingService}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '촬영 준비 중입니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()
    await waitFor(() => {
      expect(sessionTimingService.getSessionTiming).toHaveBeenCalledWith({
        manifestPath: 'C:/sessions/홍길동1234/session.json',
        sessionId: 'session-timing-visible',
      })
    })
    expect(screen.getByLabelText('세션 이름')).toHaveTextContent('홍길동1234')
  })

  it('returns to a recoverable inline error when session provisioning rejects unexpectedly', async () => {
    const user = userEvent.setup()

    const lifecycleService = {
      startSession: vi.fn(async () => {
        throw new Error('IPC bridge unavailable')
      }),
    }

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
        <SessionFlowProvider
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={lifecycleService}
        >
          <CheckInScreen />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByText('세션을 시작하지 못했어요. 잠시 후 다시 시도해 주세요.')).toBeInTheDocument()
    expect(screen.getByLabelText('세션 이름')).toHaveValue('홍길동1234')
  })

  it('transitions from preparing to waiting to ready with customer-safe copy and capture gating', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-waiting-ready',
      connectionState: 'preparing',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })

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
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-waiting-ready',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-waiting-ready',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '촬영 준비 중입니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '촬영 시작' })).toBeDisabled()
    await waitFor(() => {
      expect(readinessAdapter.watchReadiness).toHaveBeenCalled()
    })

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-waiting-ready',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:04.000Z',
      })
    })

    expect(await screen.findByRole('heading', { name: '아직 촬영할 수 없습니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '촬영 시작' })).toBeDisabled()
    expect(screen.queryByText('010-1234-5678')).not.toBeInTheDocument()

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-waiting-ready',
        connectionState: 'ready',
        captureEnabled: true,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:06.000Z',
      })
    })

    expect(await screen.findByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '촬영 시작' })).toBeEnabled()
  })

  it('lets the customer leave the ready preparation screen immediately when they press the enabled CTA', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-ready-cta',
      connectionState: 'ready',
      captureEnabled: true,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })

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
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-ready-cta',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-ready-cta',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          }}
        >
          <CustomerFlowContent />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')
    await user.click(await screen.findByRole('button', { name: '계속하기' }))

    expect(await screen.findByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '촬영 시작' }))

    expect(await screen.findByRole('heading', { name: '원하는 프리셋을 눌러 주세요.' })).toBeInTheDocument()
  })

  it('transitions from preparation to phone-required with the configured branch number', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-2',
      connectionState: 'preparing',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })

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
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-2',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-2',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '촬영 준비 중입니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-2',
        connectionState: 'phone-required',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: {
          schemaVersion: 'boothy.camera.error-envelope.v1',
          code: 'camera.unavailable',
          severity: 'error',
          customerState: 'cameraUnavailable',
          customerCameraConnectionState: 'offline',
          operatorCameraConnectionState: 'disconnected',
          operatorAction: 'contactSupport',
          retryable: false,
          message: 'Camera helper stopped responding.',
          details: 'SDK path C:/camera-helper/canon.dll missing',
        },
        emittedAt: '2026-03-08T00:02:00.000Z',
      })
    })

    expect(await screen.findByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '촬영 시작' })).not.toBeInTheDocument()
  })

  it('keeps watching readiness so the booth can recover from phone-required back to ready', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-phone-recovery',
      connectionState: 'preparing',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })

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
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-phone-recovery',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-phone-recovery',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '촬영 준비 중입니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()
    await waitFor(() => {
      expect(readinessAdapter.watchReadiness).toHaveBeenCalled()
    })

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-phone-recovery',
        connectionState: 'phone-required',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: {
          schemaVersion: 'boothy.camera.error-envelope.v1',
          code: 'camera.unavailable',
          severity: 'error',
          customerState: 'cameraUnavailable',
          customerCameraConnectionState: 'offline',
          operatorCameraConnectionState: 'disconnected',
          operatorAction: 'contactSupport',
          retryable: false,
          message: 'Camera helper stopped responding.',
        },
        emittedAt: '2026-03-08T00:02:00.000Z',
      })
    })

    expect(await screen.findByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-phone-recovery',
        connectionState: 'ready',
        captureEnabled: true,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:02:06.000Z',
      })
    })

    expect(await screen.findByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '촬영 시작' })).toBeEnabled()
    expect(readinessAdapter.watchReadiness).toHaveBeenCalled()
  })

  it('keeps the visible adjusted end time fresh while phone-required preparation stays visible', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-phone-required-timing',
      connectionState: 'preparing',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })

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
        <SessionFlowProvider
          cameraAdapter={readinessAdapter}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-phone-required-timing',
                sessionName: '홍길동5678',
                sessionFolder: 'C:/sessions/홍길동5678',
                manifestPath: 'C:/sessions/홍길동5678/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-phone-required-timing',
                manifestPath: 'C:/sessions/홍길동5678/session.json',
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
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동5678')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '촬영 준비 중입니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('오후 6:50')).toBeInTheDocument()

    await act(async () => {
      readinessAdapter.emit({
        sessionId: 'session-phone-required-timing',
        connectionState: 'phone-required',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: {
          schemaVersion: 'boothy.camera.error-envelope.v1',
          code: 'camera.unavailable',
          severity: 'error',
          customerState: 'cameraUnavailable',
          customerCameraConnectionState: 'offline',
          operatorCameraConnectionState: 'disconnected',
          operatorAction: 'contactSupport',
          retryable: false,
          message: 'Camera helper stopped responding.',
        },
        emittedAt: '2026-03-08T00:02:00.000Z',
      })
    })

    expect(await screen.findByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()

    await act(async () => {
      readinessAdapter.emitCaptureConfidence({
        sessionId: 'session-phone-required-timing',
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

  it('routes directly to phone-required when the readiness contract cannot be loaded', async () => {
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
        <SessionFlowProvider
          cameraAdapter={{
            getReadinessSnapshot: vi.fn(async () => {
              throw new Error('command get_camera_readiness_snapshot not found')
            }),
            watchReadiness: vi.fn(async () => () => undefined),
            getCaptureConfidenceSnapshot: vi.fn(async () => {
              throw new Error('not used')
            }),
            watchCaptureConfidence: vi.fn(async () => () => undefined),
          }}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-contract-failure',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-contract-failure',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()
  })

  it('falls back to phone-required when a watched readiness payload violates the shared contract', async () => {
    const user = userEvent.setup()
    const invokeFn = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      if (command === 'get_camera_readiness_snapshot') {
        return {
          sessionId: 'session-watch-contract-failure',
          connectionState: 'preparing',
          captureEnabled: false,
          lastStableCustomerState: null,
          error: null,
          emittedAt: '2026-03-08T00:00:02.000Z',
        }
      }

      if (command === 'watch_camera_readiness') {
        const statusChannel = args?.statusChannel as {
          onmessage: (payload: unknown) => void
        }

        queueMicrotask(() => {
          statusChannel.onmessage({
            sessionId: 'session-watch-contract-failure',
            connectionState: 'ready',
            captureEnabled: false,
            lastStableCustomerState: 'ready',
            error: null,
            emittedAt: '2026-03-08T00:00:04.000Z',
          })
        })

        return undefined
      }

      throw new Error(`Unexpected command: ${command}`)
    })

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
        <SessionFlowProvider
          cameraAdapter={createCameraAdapter({
            invokeFn,
            isTauriFn: () => true,
          })}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-watch-contract-failure',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-watch-contract-failure',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
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
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()
  })

  it('escalates to phone-required when the approved preparation threshold has already elapsed', async () => {
    const user = userEvent.setup()
    const readinessAdapter = createReadinessAdapter({
      sessionId: 'session-timeout',
      connectionState: 'preparing',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:02.000Z',
    })

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
        <SessionFlowProvider
          cameraAdapter={{
            ...readinessAdapter,
            getCaptureConfidenceSnapshot: vi.fn(async () => {
              throw new Error('not used')
            }),
            watchCaptureConfidence: vi.fn(async () => () => undefined),
          }}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-timeout',
                sessionName: '홍길동1234',
                sessionFolder: 'C:/sessions/홍길동1234',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                createdAt: '2026-03-08T00:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-timeout',
                manifestPath: 'C:/sessions/홍길동1234/session.json',
                timing: {
                  reservationStartAt: '2026-03-08T08:00:00.000Z',
                  actualShootEndAt: '2026-03-08T08:10:00.000Z',
                  sessionType: 'standard' as const,
                  operatorExtensionCount: 0,
                  lastTimingUpdateAt: '2026-03-08T08:00:00.000Z',
                },
              },
            })),
            extendSessionTiming: vi.fn(),
          }}
        >
          <ReadinessFlowSurface />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByLabelText('세션 이름'), '홍길동1234')
    await user.keyboard('{Enter}')

    expect(await screen.findByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()
  })
})
