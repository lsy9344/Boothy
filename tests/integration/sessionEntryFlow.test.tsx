import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { BranchConfigContext } from '../../src/branch-config/BranchConfigContext.js'
import { CheckInScreen } from '../../src/customer-flow/screens/CheckInScreen.js'
import { PreparationScreen } from '../../src/customer-flow/screens/PreparationScreen.js'
import { SessionFlowProvider, useSessionFlow } from '../../src/session-domain/state/SessionFlowProvider.js'

function createReadinessAdapter() {
  return {
    getReadinessSnapshot: vi.fn(async ({ sessionId }: { sessionId: string }) => ({
      sessionId,
      connectionState: 'preparing' as const,
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-12T09:00:02.000Z',
    })),
    watchReadiness: vi.fn(async () => () => undefined),
    getCaptureConfidenceSnapshot: vi.fn(async () => {
      throw new Error('capture confidence not used in this test')
    }),
    watchCaptureConfidence: vi.fn(async () => () => undefined),
  }
}

function SessionEntryHarness() {
  const { clearActiveSession, state } = useSessionFlow()

  if (state.activeSession && state.readiness) {
    return (
      <>
        <PreparationScreen readiness={state.readiness} sessionName={state.activeSession.sessionName} />
        <button onClick={() => clearActiveSession()} type="button">
          세션 초기화
        </button>
      </>
    )
  }

  return <CheckInScreen />
}

describe('session entry flow', () => {
  it('returns the customer entry surface to the session-name start screen when active session context is cleared', async () => {
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
        <SessionFlowProvider
          cameraAdapter={createReadinessAdapter()}
          lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
          lifecycleService={{
            startSession: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-14',
                sessionName: '김보라1234',
                sessionFolder: 'C:/sessions/session-14',
                manifestPath: 'C:/sessions/session-14/session.json',
                createdAt: '2026-03-12T09:00:00.000Z',
                preparationState: 'preparing' as const,
              },
            })),
          }}
          sessionTimingService={{
            initializeSessionTiming: vi.fn(),
            getSessionTiming: vi.fn(async () => ({
              ok: true as const,
              value: {
                sessionId: 'session-14',
                manifestPath: 'C:/sessions/session-14/session.json',
                timing: {
                  reservationStartAt: '2026-03-12T09:00:00.000Z',
                  actualShootEndAt: '2099-03-12T09:50:00.000Z',
                  sessionType: 'standard' as const,
                  operatorExtensionCount: 0,
                  lastTimingUpdateAt: '2026-03-12T09:00:00.000Z',
                },
              },
            })),
            extendSessionTiming: vi.fn(),
          }}
        >
          <SessionEntryHarness />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
    )

    expect(await screen.findByRole('heading', { name: '세션 이름을 입력해 주세요.' })).toBeInTheDocument()
    await user.type(screen.getByRole('textbox', { name: '세션 이름' }), '김보라1234')
    await user.click(screen.getByRole('button', { name: '시작하기' }))

    expect(await screen.findByText('촬영 준비 중입니다. 잠시만 기다려 주세요.')).toBeInTheDocument()
    expect(await screen.findByText('김보라1234')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '세션 초기화' })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '세션 초기화' }))

    expect(await screen.findByRole('heading', { name: '세션 이름을 입력해 주세요.' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: '세션 이름' })).toHaveValue('')
    expect(screen.queryByText('김보라1234')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '촬영 시작' })).not.toBeInTheDocument()
  })
})
