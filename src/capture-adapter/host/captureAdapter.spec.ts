import { describe, expect, it, vi } from 'vitest'

import { createCaptureAdapter } from './captureAdapter.js'

describe('createCaptureAdapter', () => {
  it('converts native gallery file paths into previewable asset URLs before returning them to the UI', async () => {
    const convertFileSrcFn = vi.fn((filePath: string) => `asset://${filePath.replaceAll(':', '').replaceAll('/', '-')}`)
    const invokeFn = vi.fn(async () => ({
      schemaVersion: 'boothy.camera.contract.v1',
      sessionId: 'session-1',
      sessionName: 'Kim Family',
      shootEndsAt: '2026-03-08T09:15:00.000Z',
      activePresetName: 'Classic Mono',
      latestCaptureId: 'capture-002',
      selectedCaptureId: 'capture-002',
      items: [
        {
          captureId: 'capture-002',
          sessionId: 'session-1',
          capturedAt: '2026-03-08T09:02:00.000Z',
          displayOrder: 1,
          isLatest: true,
          previewPath: 'C:/Boothy/Sessions/session-1/processed/capture-002.jpg',
          thumbnailPath: 'C:/Boothy/Sessions/session-1/processed/thumb-capture-002.jpg',
          label: '두 번째 사진',
        },
      ],
    }))

    const adapter = createCaptureAdapter({
      convertFileSrcFn,
      invokeFn,
      isTauriFn: () => true,
    })

    await expect(
      adapter.loadSessionGallery({
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session.json',
      }),
    ).resolves.toMatchObject({
      items: [
        {
          previewPath: 'asset://C-Boothy-Sessions-session-1-processed-capture-002.jpg',
          thumbnailPath: 'asset://C-Boothy-Sessions-session-1-processed-thumb-capture-002.jpg',
        },
      ],
    })
    expect(convertFileSrcFn).toHaveBeenCalledTimes(2)
  })

  it('does not re-convert already previewable gallery URLs', async () => {
    const convertFileSrcFn = vi.fn((filePath: string) => `asset://${filePath}`)
    const invokeFn = vi.fn(async () => ({
      schemaVersion: 'boothy.camera.contract.v1',
      deletedCaptureId: 'capture-002',
      confirmationMessage: '사진이 삭제되었습니다.',
      gallery: {
        schemaVersion: 'boothy.camera.contract.v1',
        sessionId: 'session-1',
        sessionName: 'Kim Family',
        shootEndsAt: '2026-03-08T09:15:00.000Z',
        activePresetName: 'Classic Mono',
        latestCaptureId: 'capture-001',
        selectedCaptureId: 'capture-001',
        items: [
          {
            captureId: 'capture-001',
            sessionId: 'session-1',
            capturedAt: '2026-03-08T09:00:00.000Z',
            displayOrder: 0,
            isLatest: true,
            previewPath: 'asset://session-1/capture-001',
            thumbnailPath: 'asset://session-1/thumb-capture-001',
            label: '첫 번째 사진',
          },
        ],
      },
    }))

    const adapter = createCaptureAdapter({
      convertFileSrcFn,
      invokeFn,
      isTauriFn: () => true,
    })

    await expect(
      adapter.deleteSessionPhoto({
        sessionId: 'session-1',
        captureId: 'capture-002',
        manifestPath: 'C:/Boothy/Sessions/session-1/session.json',
      }),
    ).resolves.toMatchObject({
      gallery: {
        items: [
          {
            previewPath: 'asset://session-1/capture-001',
            thumbnailPath: 'asset://session-1/thumb-capture-001',
          },
        ],
      },
    })
    expect(convertFileSrcFn).not.toHaveBeenCalled()
  })

  it('rejects gallery payloads whose native asset paths escape the active session root even when the session id matches', async () => {
    const invokeFn = vi.fn(async () => ({
      schemaVersion: 'boothy.camera.contract.v1',
      sessionId: 'session-1',
      sessionName: 'Kim Family',
      shootEndsAt: '2026-03-08T09:15:00.000Z',
      activePresetName: 'Classic Mono',
      latestCaptureId: 'capture-002',
      selectedCaptureId: 'capture-002',
      items: [
        {
          captureId: 'capture-002',
          sessionId: 'session-1',
          capturedAt: '2026-03-08T09:02:00.000Z',
          displayOrder: 1,
          isLatest: true,
          previewPath: 'C:/Boothy/Sessions/session-2/processed/capture-002.jpg',
          thumbnailPath: 'C:/Boothy/Sessions/session-2/processed/thumb-capture-002.jpg',
          label: '두 번째 사진',
        },
      ],
    }))

    const adapter = createCaptureAdapter({
      convertFileSrcFn: (filePath) => filePath,
      invokeFn,
      isTauriFn: () => true,
    })

    await expect(
      adapter.loadSessionGallery({
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session.json',
      }),
    ).rejects.toThrow(/outside the active session/i)
  })

  it('selects the nearest remaining capture after deleting the current selection in the browser fallback store', async () => {
    const adapter = createCaptureAdapter({
      isTauriFn: () => false,
    })

    await adapter.loadSessionGallery({
      sessionId: 'session-nearest',
      manifestPath: 'C:/Boothy/Sessions/session-nearest/session.json',
    })

    const response = await adapter.deleteSessionPhoto({
      sessionId: 'session-nearest',
      captureId: 'capture-001',
      manifestPath: 'C:/Boothy/Sessions/session-nearest/session.json',
    })

    expect(response.gallery.selectedCaptureId).toBe('capture-002')
    expect(response.gallery.latestCaptureId).toBe('capture-002')
    expect(response.gallery.items).toHaveLength(1)
  })

  it('rejects stale capture ids in the browser fallback store instead of fabricating a delete success', async () => {
    const adapter = createCaptureAdapter({
      isTauriFn: () => false,
    })

    await adapter.loadSessionGallery({
      sessionId: 'session-stale-delete',
      manifestPath: 'C:/Boothy/Sessions/session-stale-delete/session.json',
    })

    await expect(
      adapter.deleteSessionPhoto({
        sessionId: 'session-stale-delete',
        captureId: 'capture-999',
        manifestPath: 'C:/Boothy/Sessions/session-stale-delete/session.json',
      }),
    ).rejects.toThrow('capture not found for session: capture-999')
  })
})
