import { describe, expect, it } from 'vitest'

import type { DeleteSessionPhotoResponse, SessionGallerySnapshot } from '../../shared-contracts/dto/sessionGallery.js'
import { schemaVersions } from '../../shared-contracts/dto/schemaVersion.js'
import { initialSessionState, sessionReducer } from './sessionReducer.js'

function createGallerySnapshot(): SessionGallerySnapshot {
  return {
    schemaVersion: schemaVersions.contract,
    sessionId: 'session-24',
    sessionName: '김보라1234',
    shootEndsAt: '2026-03-08T10:50:00.000Z',
    activePresetName: 'Soft Noir',
    latestCaptureId: 'capture-002',
    selectedCaptureId: 'capture-002',
    items: [
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
    ],
  }
}

function createDeleteResponse(): DeleteSessionPhotoResponse {
  return {
    schemaVersion: schemaVersions.contract,
    deletedCaptureId: 'capture-002',
    confirmationMessage: '사진이 삭제되었습니다.',
    gallery: {
      ...createGallerySnapshot(),
      latestCaptureId: 'capture-001',
      selectedCaptureId: 'capture-001',
      items: [
        {
          captureId: 'capture-001',
          sessionId: 'session-24',
          capturedAt: '2026-03-08T10:05:00.000Z',
          displayOrder: 0,
          isLatest: true,
          previewPath: 'asset://session-24/capture-001',
          thumbnailPath: 'asset://session-24/thumb-capture-001',
          label: '첫 번째 사진',
        },
      ],
    },
  }
}

describe('sessionReducer review rail state', () => {
  it('returns the customer to the blocked preparation phase when readiness degrades after advancing', () => {
    const degradedReadiness = {
      kind: 'waiting' as const,
      captureEnabled: false as const,
      branchPhoneNumber: '010-1234-5678',
      hostStatus: {
        sessionId: 'session-24',
        connectionState: 'waiting' as const,
        captureEnabled: false,
        lastStableCustomerState: 'ready' as const,
        error: null,
        emittedAt: '2026-03-08T10:06:00.000Z',
      },
    }

    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
        activeSession: {
          sessionId: 'session-24',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/session-24',
          manifestPath: 'C:/sessions/session-24/session.json',
          createdAt: '2026-03-08T10:00:00.000Z',
          preparationState: 'preparing',
        },
        captureConfidence: {
          sessionId: 'session-24',
          revision: 2,
          updatedAt: '2026-03-08T10:05:00.000Z',
          shootEndsAt: '2026-03-08T10:50:00.000Z',
          activePreset: {
            presetId: 'warm-tone',
            label: '웜톤',
          },
          latestPhoto: {
            kind: 'empty',
          },
        },
        sessionTiming: {
          reservationStartAt: '2026-03-08T10:00:00.000Z',
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T10:00:00.000Z',
        },
        selectedPresetId: 'background-pink',
        activePreset: {
          presetId: 'background-pink',
          displayName: '배경지 - 핑크',
        },
        pendingActivePresetId: 'background-pink',
      },
      {
        type: 'readiness_changed',
        readiness: degradedReadiness,
      },
    )

    expect(nextState.phase).toBe('preparing')
    expect(nextState.readiness).toEqual(degradedReadiness)
    expect(nextState.activeSession?.sessionId).toBe('session-24')
    expect(nextState.sessionTiming?.actualShootEndAt).toBe('2026-03-08T10:50:00.000Z')
    expect(nextState.selectedPresetId).toBe('background-pink')
    expect(nextState.activePreset).toEqual({
      presetId: 'background-pink',
      displayName: '배경지 - 핑크',
    })
    expect(nextState.pendingActivePresetId).toBe('background-pink')
  })

  it('stores ordered session gallery data and adopts the host-selected photo', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
      },
      {
        type: 'review_gallery_loaded',
        gallery: createGallerySnapshot(),
      },
    )

    expect(nextState.reviewStatus).toBe('ready')
    expect(nextState.reviewGallery?.items).toHaveLength(2)
    expect(nextState.selectedReviewCaptureId).toBe('capture-002')
    expect(nextState.reviewFeedback).toBeNull()
  })

  it('preserves the customer selection across gallery refresh when the selected photo still exists', () => {
    const seededState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
      },
      {
        type: 'review_gallery_loaded',
        gallery: createGallerySnapshot(),
      },
    )

    const manuallySelectedState = sessionReducer(seededState, {
      type: 'review_capture_selected',
      captureId: 'capture-001',
    })

    const refreshedState = sessionReducer(manuallySelectedState, {
      type: 'review_gallery_loaded',
      gallery: {
        ...createGallerySnapshot(),
        selectedCaptureId: 'capture-002',
      },
    })

    expect(refreshedState.selectedReviewCaptureId).toBe('capture-001')
  })

  it('reconciles deletion by updating selection, latest-photo identity, and success feedback', () => {
    const seededState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
      },
      {
        type: 'review_gallery_loaded',
        gallery: createGallerySnapshot(),
      },
    )

    const nextState = sessionReducer(seededState, {
      type: 'review_delete_succeeded',
      response: createDeleteResponse(),
    })

    expect(nextState.reviewStatus).toBe('ready')
    expect(nextState.selectedReviewCaptureId).toBe('capture-001')
    expect(nextState.reviewGallery?.latestCaptureId).toBe('capture-001')
    expect(nextState.reviewGallery?.items).toHaveLength(1)
    expect(nextState.reviewFeedback).toBe('사진이 삭제되었습니다.')
  })
})
