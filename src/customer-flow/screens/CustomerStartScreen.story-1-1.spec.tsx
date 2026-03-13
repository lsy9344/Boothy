import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { BranchConfigContext, branchConfigDefaultState } from '../../branch-config/BranchConfigContext.js'
import { CustomerStartScreen } from './CustomerStartScreen.js'

const { recordLifecycleEvent } = vi.hoisted(() => ({
  recordLifecycleEvent: vi.fn(async () => undefined),
}))

vi.mock('../../diagnostics-log/services/operationalLogClient.js', () => ({
  recordLifecycleEvent,
}))

describe('customer start surface', () => {
  beforeEach(() => {
    recordLifecycleEvent.mockClear()
  })

  it('records the first-screen lifecycle event through the typed log adapter using the configured branch id', async () => {
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
        <CustomerStartScreen />
      </BranchConfigContext.Provider>,
    )

    await waitFor(() =>
      expect(recordLifecycleEvent).toHaveBeenCalledWith({
        payloadVersion: 1,
        eventType: 'first_screen_displayed',
        occurredAt: expect.any(String),
        branchId: 'gangnam-main',
        currentStage: 'customer-start',
      }),
    )
  })

  it('renders the starter shell with approved copy and primary action', async () => {
    render(
      <BranchConfigContext.Provider
        value={{
          ...branchConfigDefaultState,
          status: 'ready',
        }}
      >
        <CustomerStartScreen />
      </BranchConfigContext.Provider>,
    )

    expect(await screen.findByRole('heading', { name: '세션 이름을 입력해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('이름을 입력하면 다음 화면 준비를 바로 이어갈게요.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '시작하기' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: '세션 이름' })).toBeInTheDocument()
    expect(recordLifecycleEvent).toHaveBeenCalledTimes(1)
  })

  it('shows inline validation when the customer tries to start without a session name', async () => {
    const user = userEvent.setup()
    const onStart = vi.fn()

    render(
      <BranchConfigContext.Provider
        value={{
          ...branchConfigDefaultState,
          status: 'ready',
        }}
      >
        <CustomerStartScreen onStart={onStart} />
      </BranchConfigContext.Provider>,
    )

    await user.click(screen.getByRole('button', { name: '시작하기' }))

    expect(onStart).not.toHaveBeenCalled()
    expect(screen.getByRole('alert')).toHaveTextContent('세션 이름을 입력해 주세요.')
  })

  it('keeps the validation message visible until the customer enters a non-empty session name', async () => {
    const user = userEvent.setup()

    render(
      <BranchConfigContext.Provider
        value={{
          ...branchConfigDefaultState,
          status: 'ready',
        }}
      >
        <CustomerStartScreen />
      </BranchConfigContext.Provider>,
    )

    const sessionNameInput = screen.getByRole('textbox', { name: '세션 이름' })
    await user.click(screen.getByRole('button', { name: '시작하기' }))
    expect(screen.getByRole('alert')).toHaveTextContent('세션 이름을 입력해 주세요.')

    await user.type(sessionNameInput, '   ')
    expect(screen.getByRole('alert')).toHaveTextContent('세션 이름을 입력해 주세요.')

    await user.type(sessionNameInput, '봄날 촬영')
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('trims the session name and passes it to the start action', async () => {
    const user = userEvent.setup()
    const onStart = vi.fn()

    render(
      <BranchConfigContext.Provider
        value={{
          ...branchConfigDefaultState,
          status: 'ready',
        }}
      >
        <CustomerStartScreen onStart={onStart} />
      </BranchConfigContext.Provider>,
    )

    await user.type(screen.getByRole('textbox', { name: '세션 이름' }), '  봄날 촬영  ')
    await user.click(screen.getByRole('button', { name: '시작하기' }))

    expect(onStart).toHaveBeenCalledTimes(1)
    expect(onStart).toHaveBeenCalledWith('봄날 촬영')
  })

  it('keeps the primary action at the approved touch-target height', async () => {
    const stylesheet = readFileSync(join(process.cwd(), 'src/index.css'), 'utf8')

    expect(stylesheet).toMatch(/\.customer-shell__actions,\s*\.primary-action-button\s*\{[\s\S]*width:\s*100%;/m)
    expect(stylesheet).toMatch(/\.primary-action-button\s*\{[\s\S]*min-height:\s*88px;/m)
  })
})
