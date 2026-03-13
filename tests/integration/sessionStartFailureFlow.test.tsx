import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  startSession: vi
    .fn()
    .mockResolvedValueOnce({
      ok: false as const,
      errorCode: 'session.provisioning_failed' as const,
      message: 'failed to resolve app-local session root: Access is denied.',
    })
    .mockResolvedValueOnce({
      ok: true as const,
      value: {
        sessionId: 'session-42',
        sessionName: '김보라 오후 세션',
        sessionFolder: 'C:/sessions/session-42',
        manifestPath: 'C:/sessions/session-42/session.json',
        createdAt: '2026-03-13T01:00:00.000Z',
        preparationState: 'preparing' as const,
      },
    }),
  getReadinessSnapshot: vi.fn(async ({ sessionId }: { sessionId: string }) => ({
    sessionId,
    connectionState: 'preparing' as const,
    captureEnabled: false,
    lastStableCustomerState: null,
    error: null,
    emittedAt: '2026-03-13T01:00:02.000Z',
  })),
  watchReadiness: vi.fn(async () => () => undefined),
  getCaptureConfidenceSnapshot: vi.fn(async () => {
    throw new Error('capture confidence not used in this test')
  }),
  watchCaptureConfidence: vi.fn(async () => () => undefined),
  recordReadinessReached: vi.fn(async () => undefined),
  recordLifecycleEvent: vi.fn(async () => undefined),
}))

vi.mock('../../src/session-domain/services/sessionLifecycle.js', () => ({
  sessionLifecycleService: {
    startSession: mocks.startSession,
  },
}))

vi.mock('../../src/capture-adapter/host/cameraAdapter.js', () => ({
  cameraAdapter: {
    getReadinessSnapshot: mocks.getReadinessSnapshot,
    watchReadiness: mocks.watchReadiness,
    getCaptureConfidenceSnapshot: mocks.getCaptureConfidenceSnapshot,
    watchCaptureConfidence: mocks.watchCaptureConfidence,
  },
}))

vi.mock('../../src/diagnostics-log/services/lifecycleLogger.js', () => ({
  createLifecycleLogger: () => ({
    recordReadinessReached: mocks.recordReadinessReached,
  }),
}))

vi.mock('../../src/diagnostics-log/services/operationalLogClient.js', () => ({
  recordLifecycleEvent: mocks.recordLifecycleEvent,
}))

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { CheckInScreen } from '../../src/customer-flow/screens/CheckInScreen.js'
import { PreparationScreen } from '../../src/customer-flow/screens/PreparationScreen.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'

function SessionStartFailureHarness() {
  const { state } = useSessionFlow()

  if (state.activeSession && state.readiness) {
    return <PreparationScreen readiness={state.readiness} sessionName={state.activeSession.sessionName} />
  }

  return <CheckInScreen />
}

describe('session start failure recovery', () => {
  it('returns to the session-name form with safe copy and allows a touch retry after provisioning fails', async () => {
    const user = userEvent.setup()

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
        <SessionFlowProvider>
          <SessionStartFailureHarness />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    const sessionNameInput = await screen.findByRole('textbox', { name: '세션 이름' })
    await user.type(sessionNameInput, '김보라 오후 세션')
    await user.click(screen.getByRole('button', { name: '시작하기' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('세션을 시작하지 못했어요. 잠시 후 다시 시도해 주세요.')
    expect(screen.getByLabelText('세션 이름')).toHaveValue('김보라 오후 세션')
    expect(screen.queryByText('이름을 확인했어요.')).not.toBeInTheDocument()
    expect(screen.queryByText('failed to resolve app-local session root')).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '시작하기' }))

    await waitFor(() => {
      expect(screen.getByText('촬영 준비 중입니다. 잠시만 기다려 주세요.')).toBeInTheDocument()
    })
  })
})
