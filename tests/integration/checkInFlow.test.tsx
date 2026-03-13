import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { BranchConfigContext, branchConfigDefaultState } from '../../src/branch-config/BranchConfigContext.js'
import { CustomerFlowContent } from '../../src/customer-flow/screens/CustomerFlowScreen.js'
import { SessionFlowProvider } from '../../src/session-domain/state/SessionFlowProvider.js'

const { recordLifecycleEvent } = vi.hoisted(() => ({
  recordLifecycleEvent: vi.fn(async () => undefined),
}))

vi.mock('../../src/diagnostics-log/services/operationalLogClient.js', () => ({
  recordLifecycleEvent,
}))

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

function renderCustomerEntry(
  startSession = vi.fn(async ({ sessionName }: { sessionName: string }) => ({
    ok: true as const,
    value: {
      sessionId: 'session-21',
      sessionName,
      sessionFolder: 'C:/sessions/session-21',
      manifestPath: 'C:/sessions/session-21/session.json',
      createdAt: '2026-03-12T09:00:00.000Z',
      preparationState: 'preparing' as const,
    },
  })),
) {
  render(
    <BranchConfigContext.Provider
      value={{
        ...branchConfigDefaultState,
        status: 'ready',
      }}
      >
        <SessionFlowProvider
          cameraAdapter={createReadinessAdapter()}
        lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
        lifecycleService={{ startSession }}
        sessionTimingService={{
          initializeSessionTiming: vi.fn(),
          getSessionTiming: vi.fn(async () => ({
            ok: true as const,
            value: {
              sessionId: 'session-21',
              manifestPath: 'C:/sessions/session-21/session.json',
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
          <CustomerFlowContent />
        </SessionFlowProvider>
      </BranchConfigContext.Provider>,
  )

  return { startSession }
}

function createDeferredStartSession() {
  let resolve:
    | ((value: {
        ok: true
        value: {
          sessionId: string
          sessionName: string
          sessionFolder: string
          manifestPath: string
          createdAt: string
          preparationState: 'preparing'
        }
      }) => void)
    | undefined

  return {
    startSession: vi.fn(
      () =>
        new Promise<{
          ok: true
          value: {
            sessionId: string
            sessionName: string
            sessionFolder: string
            manifestPath: string
            createdAt: string
            preparationState: 'preparing'
          }
        }>((nextResolve) => {
          resolve = nextResolve
        }),
    ),
    resolve(sessionName: string) {
      resolve?.({
        ok: true,
        value: {
          sessionId: 'session-21',
          sessionName,
          sessionFolder: 'C:/sessions/session-21',
          manifestPath: 'C:/sessions/session-21/session.json',
          createdAt: '2026-03-12T09:00:00.000Z',
          preparationState: 'preparing',
        },
      })
    },
  }
}

describe('session start flow', () => {
  beforeEach(() => {
    recordLifecycleEvent.mockClear()
  })

  it('shows the session-name-only start surface immediately', async () => {
    renderCustomerEntry()

    expect(await screen.findByRole('heading', { name: '세션 이름을 입력해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('이름을 입력하면 다음 화면 준비를 바로 이어갈게요.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '시작하기' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: '세션 이름' })).toBeInTheDocument()
    expect(screen.queryByLabelText('예약자명')).not.toBeInTheDocument()
    expect(screen.queryByLabelText('휴대전화 뒤4자리')).not.toBeInTheDocument()
  })

  it('blocks empty or whitespace-only submission with inline validation', async () => {
    const user = userEvent.setup()

    renderCustomerEntry()

    await user.type(screen.getByRole('textbox', { name: '세션 이름' }), '   ')
    await user.click(screen.getByRole('button', { name: '시작하기' }))

    expect(screen.getByRole('alert')).toHaveTextContent('세션 이름을 입력해 주세요.')
  })

  it('uses the handoff seam as a customer confirmation step and resumes provisioning only after continue', async () => {
    const user = userEvent.setup()
    const deferredSessionStart = createDeferredStartSession()
    const { startSession } = renderCustomerEntry(deferredSessionStart.startSession)

    await user.type(screen.getByRole('textbox', { name: '세션 이름' }), '  봄날 촬영  ')
    await user.click(screen.getByRole('button', { name: '시작하기' }))

    expect(startSession).not.toHaveBeenCalled()
    expect(await screen.findByRole('heading', { name: '세션 이름을 확인했어요.' })).toBeInTheDocument()
    expect(screen.getByLabelText('세션 이름')).toHaveTextContent('봄날 촬영')
    expect(screen.getByRole('main')).toHaveAttribute('aria-busy', 'false')
    expect(screen.getByRole('button', { name: '계속하기' })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '계속하기' }))

    expect(startSession).toHaveBeenCalledWith(
      expect.objectContaining({
        branchId: 'branch-unconfigured',
        sessionName: '봄날 촬영',
      }),
    )
    expect(screen.getByRole('main')).toHaveAttribute('aria-busy', 'true')

    deferredSessionStart.resolve('봄날 촬영')

    expect(await screen.findByText('촬영 준비 중입니다. 잠시만 기다려 주세요.')).toBeInTheDocument()
  })
})
