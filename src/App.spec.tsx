import { render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import App from './App.js'

const { recordLifecycleEvent } = vi.hoisted(() => ({
  recordLifecycleEvent: vi.fn(async () => undefined),
}))

vi.mock('./diagnostics-log/services/operationalLogClient.js', () => ({
  recordLifecycleEvent,
}))

describe('App shell', () => {
  it('renders the session-name start surface on the default entry route', async () => {
    window.history.pushState({}, '', '/')

    render(<App />)

    expect(await screen.findByRole('heading', { name: '세션 이름을 입력해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('이름을 입력하면 다음 화면 준비를 바로 이어갈게요.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '시작하기' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: '세션 이름' })).toBeInTheDocument()
    expect(screen.queryByLabelText('예약자명')).not.toBeInTheDocument()
    expect(screen.queryByLabelText('휴대전화 뒤4자리')).not.toBeInTheDocument()

    await waitFor(() =>
      expect(recordLifecycleEvent).toHaveBeenCalledWith({
        payloadVersion: 1,
        eventType: 'first_screen_displayed',
        occurredAt: expect.any(String),
        branchId: 'branch-unconfigured',
        currentStage: 'customer-start',
      }),
    )
  })
})
