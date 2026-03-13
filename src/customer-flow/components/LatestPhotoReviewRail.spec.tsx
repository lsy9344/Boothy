import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import type { SessionGalleryItem } from '../../shared-contracts/dto/sessionGallery.js'
import { LatestPhotoReviewRail } from './LatestPhotoReviewRail.js'

const items: SessionGalleryItem[] = [
  {
    captureId: 'capture-001',
    sessionId: 'session-24',
    capturedAt: '2026-03-08T10:05:00.000Z',
    displayOrder: 0,
    isLatest: false,
    previewPath: 'asset://session-24/capture-001',
    thumbnailPath: 'asset://session-24/thumb-capture-001',
    label: '첫 번째 사진',
  },
  {
    captureId: 'capture-002',
    sessionId: 'session-24',
    capturedAt: '2026-03-08T10:06:00.000Z',
    displayOrder: 1,
    isLatest: true,
    previewPath: 'asset://session-24/capture-002',
    thumbnailPath: 'asset://session-24/thumb-capture-002',
    label: '두 번째 사진',
  },
]

describe('LatestPhotoReviewRail', () => {
  it('renders session thumbnails with a clear selected state and accessible labels', () => {
    render(
      <LatestPhotoReviewRail
        items={items}
        selectedCaptureId="capture-002"
        onSelectCapture={() => undefined}
      />,
    )

    expect(screen.getByRole('button', { name: '첫 번째 사진 선택' })).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByRole('button', { name: '두 번째 사진 선택' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByText('최신 사진')).toBeInTheDocument()
  })

  it('calls back with the selected capture id when another thumbnail is activated', async () => {
    const user = userEvent.setup()
    const onSelectCapture = vi.fn()

    render(
      <LatestPhotoReviewRail
        items={items}
        selectedCaptureId="capture-001"
        onSelectCapture={onSelectCapture}
      />,
    )

    await user.click(screen.getByRole('button', { name: '두 번째 사진 선택' }))

    expect(onSelectCapture).toHaveBeenCalledWith('capture-002')
  })
})
