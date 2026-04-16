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

  it('logs current-capture-visible again when the spotlight replaces a pending preview with a truthful preview', async () => {
    const { rerender } = render(
      <LatestPhotoRail
        previews={[buildPreview()]}
        isPreviewWaiting
        isExplicitPostEnd={false}
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
          label: 'current-capture-pending-visible',
          sessionId: 'session_01',
        }),
      )
    })

    rerender(
      <LatestPhotoRail
        previews={[buildPreview({ readyAtMs: Date.now() - 25 })]}
        isPreviewWaiting
        isExplicitPostEnd={false}
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
          label: 'current-capture-visible',
          sessionId: 'session_01',
          message: expect.stringContaining('surface=current-capture'),
        }),
      )
    })
  })

  it('marks the newest pending photo as a temporary unfiltered preview', () => {
    render(
      <LatestPhotoRail
        previews={[buildPreview()]}
        isPreviewWaiting
        isExplicitPostEnd={false}
        deletingCaptureId={null}
        pendingDeleteCaptureId={null}
        onDeleteCancel={() => {}}
        onDeleteConfirm={() => {}}
        onDeleteIntent={() => {}}
      />,
    )

    expect(screen.getByText('룩 적용 중')).toBeInTheDocument()
    expect(
      screen.getByText(/지금 보이는 첫 사진은 임시 미리보기예요\./i),
    ).toBeInTheDocument()
  })

  it('promotes the latest capture into a spotlight and keeps older captures in a separate history rail', () => {
    render(
      <LatestPhotoRail
        previews={[
          buildPreview(),
          buildPreview({
            captureId: 'capture_02',
            requestId: 'request_02',
            assetPath: 'C:/boothy/sessions/session_01/renders/previews/capture_02.jpg',
            readyAtMs: 540,
            isLatest: false,
          }),
        ]}
        isPreviewWaiting={false}
        isExplicitPostEnd={false}
        deletingCaptureId={null}
        pendingDeleteCaptureId={null}
        onDeleteCancel={() => {}}
        onDeleteConfirm={() => {}}
        onDeleteIntent={() => {}}
      />,
    )

    expect(
      screen.getByRole('heading', { level: 2, name: /방금 찍은 사진/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('img', { name: /현재 세션 최신 사진,\s*1번째,\s*현재 룩 룩/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('list', { name: /현재 세션 사진 히스토리/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('img', { name: /현재 세션 사진,\s*2번째,\s*현재 룩 룩/i }),
    ).toHaveAttribute(
      'src',
      expect.stringContaining(
        'C:/boothy/sessions/session_01/renders/previews/capture_02.jpg',
      ),
    )
  })
})
