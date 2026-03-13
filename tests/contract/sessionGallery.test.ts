import { describe, expect, it } from 'vitest'

import {
  deleteSessionPhotoRequestSchema,
  deleteSessionPhotoResponseSchema,
  sessionGalleryItemSchema,
  sessionGallerySnapshotSchema,
} from '../../src/shared-contracts/dto/sessionGallery.js'
import { schemaVersions } from '../../src/shared-contracts/dto/schemaVersion.js'

describe('session gallery contract', () => {
  it('accepts a session-scoped gallery snapshot with stable ordering and preview metadata', () => {
    expect(
      sessionGallerySnapshotSchema.parse({
        schemaVersion: schemaVersions.contract,
        sessionId: 'session-001',
        sessionName: 'Kim Family',
        shootEndsAt: '2026-03-08T09:15:00.000Z',
        activePresetName: 'Classic Mono',
        latestCaptureId: 'capture-002',
        selectedCaptureId: 'capture-002',
        items: [
          {
            captureId: 'capture-001',
            sessionId: 'session-001',
            capturedAt: '2026-03-08T09:00:00.000Z',
            displayOrder: 0,
            isLatest: false,
            previewPath: 'C:/Boothy/Sessions/session-001/processed/capture-001.jpg',
            thumbnailPath: 'C:/Boothy/Sessions/session-001/processed/thumb-capture-001.jpg',
            label: '첫 번째 사진',
          },
          {
            captureId: 'capture-002',
            sessionId: 'session-001',
            capturedAt: '2026-03-08T09:02:00.000Z',
            displayOrder: 1,
            isLatest: true,
            previewPath: 'C:/Boothy/Sessions/session-001/processed/capture-002.jpg',
            thumbnailPath: 'C:/Boothy/Sessions/session-001/processed/thumb-capture-002.jpg',
            label: '두 번째 사진',
          },
        ],
      }),
    ).toMatchObject({
      latestCaptureId: 'capture-002',
      selectedCaptureId: 'capture-002',
      items: [
        expect.objectContaining({
          captureId: 'capture-001',
          displayOrder: 0,
        }),
        expect.objectContaining({
          captureId: 'capture-002',
          isLatest: true,
        }),
      ],
    })
  })

  it('requires delete requests and responses to stay bound to the active session', () => {
    expect(
      deleteSessionPhotoRequestSchema.parse({
        sessionId: 'session-001',
        captureId: 'capture-002',
        manifestPath: 'C:/Boothy/Sessions/session-001/session.json',
      }),
    ).toEqual({
      sessionId: 'session-001',
      captureId: 'capture-002',
      manifestPath: 'C:/Boothy/Sessions/session-001/session.json',
    })

    expect(
      deleteSessionPhotoResponseSchema.parse({
        schemaVersion: schemaVersions.contract,
        deletedCaptureId: 'capture-002',
        confirmationMessage: '사진이 삭제되었습니다.',
        gallery: {
          schemaVersion: schemaVersions.contract,
          sessionId: 'session-001',
          sessionName: 'Kim Family',
          shootEndsAt: '2026-03-08T09:15:00.000Z',
          activePresetName: 'Classic Mono',
          latestCaptureId: 'capture-001',
          selectedCaptureId: 'capture-001',
          items: [
            {
              captureId: 'capture-001',
              sessionId: 'session-001',
              capturedAt: '2026-03-08T09:00:00.000Z',
              displayOrder: 0,
              isLatest: true,
              previewPath: 'C:/Boothy/Sessions/session-001/processed/capture-001.jpg',
              thumbnailPath: 'C:/Boothy/Sessions/session-001/processed/thumb-capture-001.jpg',
              label: '첫 번째 사진',
            },
          ],
        },
      }),
    ).toMatchObject({
      deletedCaptureId: 'capture-002',
      confirmationMessage: '사진이 삭제되었습니다.',
      gallery: {
        latestCaptureId: 'capture-001',
        selectedCaptureId: 'capture-001',
      },
    })
  })

  it('rejects gallery items that omit session ownership or preview references', () => {
    expect(() =>
      sessionGalleryItemSchema.parse({
        captureId: 'capture-003',
        sessionId: '',
        capturedAt: '2026-03-08T09:03:00.000Z',
        displayOrder: 2,
        isLatest: false,
        previewPath: '',
        thumbnailPath: 'C:/Boothy/Sessions/session-001/processed/thumb-capture-003.jpg',
        label: '세 번째 사진',
      }),
    ).toThrow()
  })
})
