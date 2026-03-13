import { act, render, screen } from '@testing-library/react'
import { useEffect, useRef } from 'react'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { CustomerFlowContent } from '../../src/customer-flow/screens/CustomerFlowScreen.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'

function AutoCheckIn() {
  const { confirmPresetSelection, selectPreset, startJourney, submitCheckIn, updateField, state } = useSessionFlow()
  const startedRef = useRef(false)
  const sessionNameSeededRef = useRef(false)
  const submittedRef = useRef(false)
  const presetSelectedRef = useRef(false)
  const presetConfirmedRef = useRef(false)
  const sessionName = '김보라1234'

  useEffect(() => {
    if (startedRef.current) {
      return
    }

    startedRef.current = true
    startJourney()
  }, [startJourney])

  useEffect(() => {
    if (state.phase !== 'idle' || sessionNameSeededRef.current) {
      return
    }

    sessionNameSeededRef.current = true
    updateField('sessionName', sessionName)
  }, [sessionName, state.phase, updateField])

  useEffect(() => {
    if (
      state.phase !== 'idle' ||
      submittedRef.current ||
      !sessionNameSeededRef.current ||
      state.fields.sessionName !== sessionName
    ) {
      return
    }

    submittedRef.current = true
    void submitCheckIn()
  }, [sessionName, state.fields.sessionName, state.phase, submitCheckIn])

  useEffect(() => {
    if (state.phase !== 'preset-selection' || presetSelectedRef.current) {
      return
    }

    presetSelectedRef.current = true
    void selectPreset('background-pink')
  }, [selectPreset, state.phase])

  useEffect(() => {
    if (state.phase !== 'preset-selection' || !state.selectedPresetId || presetConfirmedRef.current) {
      return
    }

    presetConfirmedRef.current = true
    void confirmPresetSelection()
  }, [confirmPresetSelection, state.phase, state.selectedPresetId])

  return <CustomerFlowContent />
}

async function settleIntoCaptureReady() {
  await act(async () => {
    vi.advanceTimersByTime(1_500)
    await Promise.resolve()
    await Promise.resolve()
  })

  await act(async () => {
    vi.advanceTimersByTime(100)
    await Promise.resolve()
  })
}

describe('post-end flow integration', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-03-08T10:49:55.000Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('leaves capture mode at the authoritative end threshold and shows a handoff surface with the active session name', async () => {
    const postEndOutcomeService = {
      getPostEndOutcome: vi.fn(async () => ({
        ok: true as const,
        value: {
          sessionId: 'session-43',
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          outcomeKind: 'handoff' as const,
          guidanceMode: 'standard' as const,
          sessionName: '김보라1234',
          showSessionName: true,
          handoffTargetLabel: '프런트 데스크',
        },
      })),
    }

    render(
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
              sessionId: 'session-43',
              connectionState: 'ready' as const,
              captureEnabled: true,
              lastStableCustomerState: 'ready' as const,
              error: null,
              emittedAt: '2026-03-08T10:49:56.000Z',
            })),
            watchReadiness: vi.fn(async () => () => undefined),
            getCaptureConfidenceSnapshot: vi.fn(async () => ({
              sessionId: 'session-43',
              revision: 2,
              updatedAt: '2026-03-08T10:49:57.000Z',
              shootEndsAt: '2026-03-08T10:50:00.000Z',
              activePreset: {
                presetId: 'background-pink',
                label: '배경지 - 핑크',
              },
              latestPhoto: {
                kind: 'ready' as const,
                photo: {
                  sessionId: 'session-43',
                  captureId: 'capture-001',
                  sequence: 1,
                  assetUrl: 'asset://session-43/capture-001',
                  capturedAt: '2026-03-08T10:49:57.000Z',
                },
              },
            })),
            watchCaptureConfidence: vi.fn(async () => () => undefined),
          }}
          lifecycleLogger={{
            recordActualShootEnd: vi.fn(async () => undefined),
            recordExportStateChanged: vi.fn(async () => undefined),
            recordPhoneRequired: vi.fn(async () => undefined),
            recordPresetCatalogFallback: vi.fn(async () => undefined),
            recordReadinessReached: vi.fn(async () => undefined),
            recordSessionCompleted: vi.fn(async () => undefined),
          }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-43',
                sessionName: '김보라1234',
                sessionFolder: 'C:/sessions/김보라1234',
                manifestPath: 'C:/sessions/김보라1234/session.json',
                createdAt: '2026-03-08T10:49:55.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          postEndOutcomeService={postEndOutcomeService}
          presetCatalogService={{
            loadApprovedPresetCatalog: vi.fn(async () => ({
              status: 'ready' as const,
              presets: [],
              source: 'approved' as const,
            })),
          }}
          presetSelectionService={{
            selectPreset: vi.fn(async () => ({
              ok: true as const,
              value: {
                manifestPath: 'C:/sessions/김보라1234/session.json',
                updatedAt: '2026-03-08T10:49:57.000Z',
                activePreset: {
                  presetId: 'background-pink',
                  displayName: '배경지 - 핑크',
                },
              },
            })),
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
                sessionId: 'session-43',
                manifestPath: 'C:/sessions/김보라1234/session.json',
                timing: {
                  reservationStartAt: '2026-03-08T10:00:00.000Z',
                  actualShootEndAt: '2026-03-08T10:50:00.000Z',
                  sessionType: 'standard' as const,
                  operatorExtensionCount: 0,
                  lastTimingUpdateAt: '2026-03-08T10:00:00.000Z',
                },
              },
            })),
            extendSessionTiming: vi.fn(),
          }}
        >
          <AutoCheckIn />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await settleIntoCaptureReady()

    expect(screen.getByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()

    await act(async () => {
      vi.advanceTimersByTime(5_500)
      await Promise.resolve()
    })

    expect(postEndOutcomeService.getPostEndOutcome).toHaveBeenCalledWith({
      manifestPath: 'C:/sessions/김보라1234/session.json',
      sessionId: 'session-43',
    })

    expect(screen.getByRole('heading', { name: '세션 안내를 확인해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('김보라1234')).toBeInTheDocument()
    expect(screen.getByText('프런트 데스크')).toBeInTheDocument()
    expect(screen.queryByText('배경지 - 핑크')).not.toBeInTheDocument()
  })

  it('routes unresolved post-end resolution to bounded wait-or-call guidance without exposing raw export states', async () => {
    render(
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
              sessionId: 'session-44',
              connectionState: 'ready' as const,
              captureEnabled: true,
              lastStableCustomerState: 'ready' as const,
              error: null,
              emittedAt: '2026-03-08T10:49:56.000Z',
            })),
            watchReadiness: vi.fn(async () => () => undefined),
            getCaptureConfidenceSnapshot: vi.fn(async () => ({
              sessionId: 'session-44',
              revision: 2,
              updatedAt: '2026-03-08T10:49:57.000Z',
              shootEndsAt: '2026-03-08T10:50:00.000Z',
              activePreset: {
                presetId: 'background-pink',
                label: '배경지 - 핑크',
              },
              latestPhoto: {
                kind: 'empty' as const,
              },
            })),
            watchCaptureConfidence: vi.fn(async () => () => undefined),
          }}
          lifecycleLogger={{
            recordActualShootEnd: vi.fn(async () => undefined),
            recordExportStateChanged: vi.fn(async () => undefined),
            recordPhoneRequired: vi.fn(async () => undefined),
            recordPresetCatalogFallback: vi.fn(async () => undefined),
            recordReadinessReached: vi.fn(async () => undefined),
            recordSessionCompleted: vi.fn(async () => undefined),
          }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-44',
                sessionName: '김보라5678',
                sessionFolder: 'C:/sessions/김보라5678',
                manifestPath: 'C:/sessions/김보라5678/session.json',
                createdAt: '2026-03-08T10:49:55.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          postEndOutcomeService={{
            getPostEndOutcome: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-44',
                actualShootEndAt: '2026-03-08T10:50:00.000Z',
                outcomeKind: 'export-waiting' as const,
                guidanceMode: 'wait-or-call' as const,
                sessionName: null,
                showSessionName: false,
                handoffTargetLabel: null,
              },
            })),
          }}
          presetCatalogService={{
            loadApprovedPresetCatalog: vi.fn(async () => ({
              status: 'ready' as const,
              presets: [],
              source: 'approved' as const,
            })),
          }}
          presetSelectionService={{
            selectPreset: vi.fn(async () => ({
              ok: true as const,
              value: {
                manifestPath: 'C:/sessions/김보라5678/session.json',
                updatedAt: '2026-03-08T10:49:57.000Z',
                activePreset: {
                  presetId: 'background-pink',
                  displayName: '배경지 - 핑크',
                },
              },
            })),
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
                sessionId: 'session-44',
                manifestPath: 'C:/sessions/김보라5678/session.json',
                timing: {
                  reservationStartAt: '2026-03-08T10:00:00.000Z',
                  actualShootEndAt: '2026-03-08T10:50:00.000Z',
                  sessionType: 'standard' as const,
                  operatorExtensionCount: 0,
                  lastTimingUpdateAt: '2026-03-08T10:00:00.000Z',
                },
              },
            })),
            extendSessionTiming: vi.fn(),
          }}
        >
          <AutoCheckIn />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    await settleIntoCaptureReady()

    expect(screen.getByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()

    await act(async () => {
      vi.advanceTimersByTime(5_500)
      await Promise.resolve()
    })

    expect(screen.getByRole('heading', { name: '잠시만 기다려 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()
    expect(screen.queryByText('processing')).not.toBeInTheDocument()
    expect(screen.queryByText('failed')).not.toBeInTheDocument()
  })
})
