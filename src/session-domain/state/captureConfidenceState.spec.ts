import { describe, expect, it } from 'vitest'

const currentSnapshot = {
  sessionId: 'session-24',
  revision: 3,
  updatedAt: '2026-03-08T10:05:00.000Z',
  shootEndsAt: '2026-03-08T10:50:00.000Z',
  activePreset: {
    presetId: 'preset-noir',
    label: 'Soft Noir',
  },
  latestPhoto: {
    kind: 'ready',
    photo: {
      sessionId: 'session-24',
      captureId: 'capture-2',
      sequence: 2,
      assetUrl: 'asset://session-24/capture-2',
      capturedAt: '2026-03-08T10:04:00.000Z',
    },
  },
} as const

describe('mergeCaptureConfidenceState', () => {
  it('drops a leaked ready photo when the first snapshot points at a different session', async () => {
    const module = await import('./captureConfidenceState.js').catch(() => null)

    expect(module).not.toBeNull()

    const leakedInitialSnapshot = {
      ...currentSnapshot,
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-99',
          captureId: 'capture-9',
          sequence: 9,
          assetUrl: 'asset://session-99/capture-9',
          capturedAt: '2026-03-08T10:06:00.000Z',
        },
      },
    }

    expect(module?.mergeCaptureConfidenceState(null, leakedInitialSnapshot)).toEqual({
      ...currentSnapshot,
      latestPhoto: {
        kind: 'empty',
      },
    })
  })

  it('keeps the newest ready photo when an older update arrives out of order', async () => {
    const module = await import('./captureConfidenceState.js').catch(() => null)

    expect(module).not.toBeNull()

    const staleUpdate = {
      ...currentSnapshot,
      revision: 2,
      updatedAt: '2026-03-08T10:03:00.000Z',
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-24',
          captureId: 'capture-1',
          sequence: 1,
          assetUrl: 'asset://session-24/capture-1',
          capturedAt: '2026-03-08T10:02:00.000Z',
        },
      },
    }

    expect(module?.mergeCaptureConfidenceState(currentSnapshot, staleUpdate)).toEqual(currentSnapshot)
  })

  it('preserves the current session photo while applying newer metadata from a leaked later payload', async () => {
    const module = await import('./captureConfidenceState.js').catch(() => null)

    expect(module).not.toBeNull()

    const leakedPhotoUpdate = {
      ...currentSnapshot,
      revision: 4,
      updatedAt: '2026-03-08T10:06:00.000Z',
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-99',
          captureId: 'capture-9',
          sequence: 9,
          assetUrl: 'asset://session-99/capture-9',
          capturedAt: '2026-03-08T10:06:00.000Z',
        },
      },
    }

    expect(module?.mergeCaptureConfidenceState(currentSnapshot, leakedPhotoUpdate)).toEqual({
      ...leakedPhotoUpdate,
      latestPhoto: currentSnapshot.latestPhoto,
    })
  })

  it('keeps showing the current session photo while a newer same-session update is still processing without a preview', async () => {
    const module = await import('./captureConfidenceState.js').catch(() => null)

    expect(module).not.toBeNull()

    const updatingWithoutPreview = {
      ...currentSnapshot,
      revision: 4,
      updatedAt: '2026-03-08T10:06:00.000Z',
      latestPhoto: {
        kind: 'updating' as const,
        nextCaptureId: 'capture-3',
      },
    }

    expect(module?.mergeCaptureConfidenceState(currentSnapshot, updatingWithoutPreview)).toEqual({
      ...updatingWithoutPreview,
      latestPhoto: currentSnapshot.latestPhoto,
    })
  })
})
