import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionPreviewImage } from './SessionPreviewImage'

describe('SessionPreviewImage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('inlines svg preview markup so local placeholder previews still render in the booth rail', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        text: async () => '<svg xmlns="http://www.w3.org/2000/svg"></svg>',
      }),
    )

    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.svg"
        alt="현재 세션 최신 사진"
      />,
    )

    await waitFor(() => {
      expect(screen.getByAltText('현재 세션 최신 사진')).toHaveAttribute(
        'src',
        expect.stringContaining('data:image/svg+xml'),
      )
    })
  })
})
