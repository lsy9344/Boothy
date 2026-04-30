import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { CurrentSessionPreview } from '../../session-domain/selectors'
import { logCaptureClientState } from '../../shared/runtime/log-capture-client-state'
import { LatestPhotoRail } from './LatestPhotoRail'

vi.mock('../../shared/runtime/log-capture-client-state', () => ({
  logCaptureClientState: vi.fn().mockResolvedValue(undefined),
}))

function buildPreview(
  overrides: Partial<CurrentSessionPreview> = {},
): CurrentSessionPreview {
  return {
    captureId: 'capture_01',
    requestId: 'request_01',
    assetPath: 'C:/boothy/sessions/session_01/renders/previews/capture.jpg',
    previewKind: null,
    activePresetId: 'preset_soft-glow',
    activePresetVersion: '2026.03.31',
    presetDisplayName: '현재 룩',
    isCurrentActivePreset: true,
    postEndState: 'activeSession',
    readyAtMs: null,
    isLatest: true,
    ...overrides,
  }
}

describe('LatestPhotoRail', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('logs recent-session-visible again when the latest slot is replaced with a truthful preview', async () => {
    const { rerender } = render(
      <LatestPhotoRail
        previews={[buildPreview()]}
        isPreviewWaiting
        isExplicitPostEnd={false}
        isPhotoActionDisabled={false}
        deletingCaptureId={null}
        pendingDeleteCaptureId={null}
        onDeleteCancel={() => {}}
        onDeleteConfirm={() => {}}
        onDeleteIntent={() => {}}
      />,
    )

    screen
      .getByAltText(/현재 세션 최신 사진/)
      .dispatchEvent(new Event('load'))

    await waitFor(() => {
      expect(logCaptureClientState).toHaveBeenCalledWith(
        expect.objectContaining({
          label: 'recent-session-pending-visible',
          sessionId: 'session_01',
        }),
      )
    })

    rerender(
      <LatestPhotoRail
        previews={[
          buildPreview({
            readyAtMs: Date.now() - 25,
            previewKind: 'preset-applied-preview',
          }),
        ]}
        isPreviewWaiting
        isExplicitPostEnd={false}
        isPhotoActionDisabled={false}
        deletingCaptureId={null}
        pendingDeleteCaptureId={null}
        onDeleteCancel={() => {}}
        onDeleteConfirm={() => {}}
        onDeleteIntent={() => {}}
      />,
    )

    screen
      .getByAltText(/현재 세션 최신 사진/)
      .dispatchEvent(new Event('load'))

    await waitFor(() => {
      expect(logCaptureClientState).toHaveBeenCalledWith(
        expect.objectContaining({
          label: 'recent-session-visible',
          sessionId: 'session_01',
          message: expect.stringContaining('previewKind=preset-applied-preview'),
        }),
      )
    })
  })
})
