import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { DeletePhotoDialog } from './DeletePhotoDialog.js'

describe('DeletePhotoDialog', () => {
  it('requires explicit confirmation before deletion and keeps customer-safe copy', async () => {
    const user = userEvent.setup()
    const onCancel = vi.fn()
    const onConfirm = vi.fn()

    render(
      <DeletePhotoDialog
        deletePending={false}
        onCancel={onCancel}
        onConfirm={onConfirm}
        photoLabel="두 번째 사진"
      />,
    )

    expect(screen.getByRole('dialog', { name: '사진 삭제 확인' })).toBeInTheDocument()
    expect(screen.getByText('선택한 사진을 삭제할까요?')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '취소' }))
    await user.click(screen.getByRole('button', { name: '삭제하기' }))

    expect(onCancel).toHaveBeenCalledTimes(1)
    expect(onConfirm).toHaveBeenCalledTimes(1)
  })

  it('moves focus into the dialog, traps tab navigation, closes on escape, and restores focus on exit', async () => {
    const user = userEvent.setup()
    const onCancel = vi.fn()
    const onConfirm = vi.fn()

    render(
      <div>
        <button type="button">이전 포커스</button>
        <DeletePhotoDialog
          deletePending={false}
          onCancel={onCancel}
          onConfirm={onConfirm}
          photoLabel="두 번째 사진"
        />
      </div>,
    )

    const previouslyFocusedButton = screen.getByRole('button', { name: '이전 포커스' })
    const cancelButton = screen.getByRole('button', { name: '취소' })
    const confirmButton = screen.getByRole('button', { name: '삭제하기' })

    previouslyFocusedButton.focus()
    expect(cancelButton).toHaveFocus()

    await user.tab()
    expect(confirmButton).toHaveFocus()

    await user.tab()
    expect(cancelButton).toHaveFocus()

    await user.keyboard('{Escape}')
    expect(onCancel).toHaveBeenCalledTimes(1)
    expect(previouslyFocusedButton).toHaveFocus()
  })
})
