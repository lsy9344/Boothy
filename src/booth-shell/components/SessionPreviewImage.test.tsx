import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionPreviewImage } from './SessionPreviewImage'
import { logCaptureClientState } from '../../shared/runtime/log-capture-client-state'

vi.mock('../../shared/runtime/log-capture-client-state', () => ({
  logCaptureClientState: vi.fn().mockResolvedValue(undefined),
}))

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
        captureId="capture_01"
        readyAtMs={100}
        isLatest
      />,
    )

    await waitFor(() => {
      expect(screen.getByAltText('현재 세션 최신 사진')).toHaveAttribute(
        'src',
        expect.stringContaining('data:image/svg+xml'),
      )
    })
  })

  it('shows a booth-safe fallback when a raster preview cannot be loaded', async () => {
    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.jpg"
        alt="현재 세션 최신 사진"
        captureId="capture_01"
        readyAtMs={100}
        isLatest
      />,
    )

    screen.getByAltText('현재 세션 최신 사진').dispatchEvent(new Event('error'))

    await waitFor(() => {
      expect(
        screen.getByRole('img', { name: '현재 세션 최신 사진' }),
      ).toHaveTextContent('확인용 사진 준비 중')
    })
  })

  it('logs when the current session preview image becomes visible', async () => {
    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.jpg"
        alt="현재 세션 최신 사진"
        captureId="capture_01"
        readyAtMs={Date.now() - 25}
        isLatest
      />,
    )

    screen.getByAltText('현재 세션 최신 사진').dispatchEvent(new Event('load'))

    await waitFor(() => {
      expect(logCaptureClientState).toHaveBeenCalledWith(
        expect.objectContaining({
          label: 'current-session-preview-visible',
          sessionId: 'session_01',
          message: expect.stringContaining('previewKind=preset-applied-preview'),
        }),
      )
    })
  })

  it('logs a pending visibility event when a fast thumbnail appears before the final preview is ready', async () => {
    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.jpg"
        alt="현재 세션 최신 사진"
        captureId="capture_01"
        readyAtMs={null}
        isLatest
      />,
    )

    screen.getByAltText('현재 세션 최신 사진').dispatchEvent(new Event('load'))

    await waitFor(() => {
      expect(logCaptureClientState).toHaveBeenCalledWith(
        expect.objectContaining({
          label: 'current-session-preview-pending-visible',
          sessionId: 'session_01',
          message: expect.stringContaining('previewKind=pending-fast-preview'),
        }),
      )
    })
  })

  it('uses the provided visibility label base for recent-session telemetry', async () => {
    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.jpg"
        alt="현재 세션 최신 사진"
        captureId="capture_01"
        requestId="request_recent_01"
        readyAtMs={null}
        isLatest
        visibilityLabelBase="recent-session"
      />,
    )

    screen.getByAltText('현재 세션 최신 사진').dispatchEvent(new Event('load'))

    await waitFor(() => {
        expect(logCaptureClientState).toHaveBeenCalledWith(
          expect.objectContaining({
            label: 'recent-session-pending-visible',
            sessionId: 'session_01',
            message: expect.stringContaining('requestId=request_recent_01'),
          }),
        )
      })
  })

  it('uses the recent-session visibility label base when the rail tracks thumbnail visibility', async () => {
    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.jpg"
        alt="현재 세션 최신 사진"
        captureId="capture_01"
        readyAtMs={null}
        isLatest
        visibilityLabelBase="recent-session"
      />,
    )

    screen.getByAltText('현재 세션 최신 사진').dispatchEvent(new Event('load'))

    await waitFor(() => {
      expect(logCaptureClientState).toHaveBeenCalledWith(
        expect.objectContaining({
          label: 'recent-session-pending-visible',
          sessionId: 'session_01',
        }),
      )
    })
  })

  it('prioritizes loading when the rail marks a thumbnail as urgent', () => {
    render(
      <SessionPreviewImage
        assetPath="C:/boothy/sessions/session_01/renders/previews/capture.jpg"
        alt="현재 세션 최신 사진"
        captureId="capture_01"
        readyAtMs={null}
        isLatest
        prioritizeLoading
      />,
    )

    expect(screen.getByAltText('현재 세션 최신 사진')).toHaveAttribute(
      'loading',
      'eager',
    )
    expect(screen.getByAltText('현재 세션 최신 사진')).toHaveAttribute(
      'decoding',
      'sync',
    )
    expect(screen.getByAltText('현재 세션 최신 사진')).toHaveAttribute(
      'fetchpriority',
      'high',
    )
  })
})
