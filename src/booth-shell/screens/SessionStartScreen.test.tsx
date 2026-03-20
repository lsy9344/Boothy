import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RouterProvider, createMemoryRouter } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'

import { createAppRoutes } from '../../app/routes'
import { createCapabilityService } from '../../app/services/capability-service'
import type { SessionStartResult } from '../../shared-contracts'
import { createStartSessionService } from '../../session-domain/services/start-session'

function renderBoothScreen(startSession = vi.fn()) {
  const sessionService = createStartSessionService({
    gateway: {
      startSession,
    },
  })

  const router = createMemoryRouter(
    createAppRoutes({
      capabilityService: createCapabilityService(),
      sessionService,
    }),
    {
      initialEntries: ['/booth'],
    },
  )

  render(<RouterProvider router={router} />)

  return {
    startSession,
  }
}

describe('SessionStartScreen', () => {
  it('shows only the two customer inputs required to begin', async () => {
    renderBoothScreen()

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(screen.getAllByRole('textbox')).toHaveLength(2)
    expect(
      screen.queryByRole('textbox', { name: /전체 전화번호/i }),
    ).not.toBeInTheDocument()
  })

  it('blocks invalid input and shows plain-language validation messages', async () => {
    const user = userEvent.setup()
    const { startSession } = renderBoothScreen()

    await user.click(
      await screen.findByRole('button', { name: /시작하기/i }),
    )

    expect(await screen.findByText(/이름을 입력해 주세요\./i)).toBeInTheDocument()
    expect(
      screen.getByText(/휴대전화 뒤 4자리는 숫자 4자리여야 해요\./i),
    ).toBeInTheDocument()
    expect(startSession).not.toHaveBeenCalled()
  })

  it('does not coerce mixed or overlong suffix input into a valid session start', async () => {
    const user = userEvent.setup()
    const { startSession } = renderBoothScreen()

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '12a34')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(
      await screen.findByText(/휴대전화 뒤 4자리는 숫자 4자리여야 해요\./i),
    ).toBeInTheDocument()
    expect(startSession).not.toHaveBeenCalled()
  })

  it('submits valid input through the typed session service and advances the booth flow', async () => {
    const user = userEvent.setup()
    const startSession = vi.fn().mockResolvedValue({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      manifest: {
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:00.000Z',
        lifecycle: {
          status: 'active',
          stage: 'session-started',
        },
        activePreset: null,
        captures: [],
        postEnd: null,
      },
    })

    renderBoothScreen(startSession)

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(startSession).toHaveBeenCalledWith({
      name: 'Kim',
      phoneLastFour: '4821',
    })
    expect(await screen.findByText(/Kim 4821/i)).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /원하는 룩을 골라 주세요/i }),
    ).toBeInTheDocument()
  })

  it('blocks a second session start attempt while the first one is still pending', async () => {
    const user = userEvent.setup()
    let resolveStart!: (value: SessionStartResult) => void

    const startSession = vi.fn().mockImplementation(
      () =>
        new Promise<SessionStartResult>((resolve) => {
          resolveStart = resolve
        }),
    )

    renderBoothScreen(startSession)

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')

    const submitButton = screen.getByRole('button', { name: /시작하기/i })
    await user.click(submitButton)
    await user.click(submitButton)

    expect(startSession).toHaveBeenCalledTimes(1)

    resolveStart({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      manifest: {
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:00.000Z',
        lifecycle: {
          status: 'active',
          stage: 'session-started',
        },
        activePreset: null,
        captures: [],
        postEnd: null,
      },
    })

    expect(await screen.findByText(/Kim 4821/i)).toBeInTheDocument()
  })
})
