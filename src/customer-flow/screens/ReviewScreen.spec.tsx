import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { ReviewScreen } from './ReviewScreen.js'

describe('ReviewScreen', () => {
  it('keeps the delete action available while the enlarged photo view preserves session context', async () => {
    const user = userEvent.setup()
    const onRequestDelete = vi.fn()

    render(
      <ReviewScreen
        deletePending={false}
        onClose={() => undefined}
        onRequestDelete={onRequestDelete}
        photo={{
          captureId: 'capture-002',
          label: '두 번째 사진',
          previewPath: 'asset://session-24/capture-002',
        }}
        sessionName="session-24-kim"
      />,
    )

    expect(screen.getByText('session-24-kim')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: '두 번째 사진 크게 보기' })).toHaveAttribute(
      'src',
      'asset://session-24/capture-002',
    )

    await user.click(screen.getByRole('button', { name: '사진 삭제' }))

    expect(onRequestDelete).toHaveBeenCalledWith('capture-002')
  })

  it('keeps keyboard focus inside the review dialog, closes on escape, and restores prior focus', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(
      <div>
        <button type="button">이전 포커스</button>
        <ReviewScreen
          deletePending={false}
          onClose={onClose}
          onRequestDelete={() => undefined}
          photo={{
            captureId: 'capture-002',
            label: '두 번째 사진',
            previewPath: 'asset://session-24/capture-002',
          }}
          sessionName="session-24-kim"
        />
      </div>,
    )

    const previouslyFocusedButton = screen.getByRole('button', { name: '이전 포커스' })
    const closeButton = screen.getByRole('button', { name: '닫기' })
    const deleteButton = screen.getByRole('button', { name: '사진 삭제' })

    previouslyFocusedButton.focus()
    expect(closeButton).toHaveFocus()

    await user.tab()
    expect(deleteButton).toHaveFocus()

    await user.tab({ shift: true })
    expect(closeButton).toHaveFocus()

    await user.keyboard('{Escape}')
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(previouslyFocusedButton).toHaveFocus()
  })
})
