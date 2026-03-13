import { describe, expect, it } from 'vitest'

import type { PostEndOutcome } from '../../shared-contracts/dto/postEndOutcome.js'
import { initialSessionState, sessionReducer, type SessionState } from './sessionReducer.js'

function createCaptureReadyState(): SessionState {
  return {
    ...initialSessionState,
    phase: 'capture-ready',
    activeSession: {
      sessionId: 'session-43',
      sessionName: '김보라1234',
      sessionFolder: 'C:/sessions/김보라1234',
      manifestPath: 'C:/sessions/김보라1234/session.json',
      createdAt: '2026-03-08T10:49:55.000Z',
      preparationState: 'preparing',
    },
    captureConfidence: {
      sessionId: 'session-43',
      revision: 3,
      updatedAt: '2026-03-08T10:49:57.000Z',
      shootEndsAt: '2026-03-08T10:50:00.000Z',
      activePreset: {
        presetId: 'background-pink',
        label: '배경지 - 핑크',
      },
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-43',
          captureId: 'capture-001',
          sequence: 1,
          assetUrl: 'asset://session-43/capture-001',
          capturedAt: '2026-03-08T10:49:57.000Z',
        },
      },
    },
    activePreset: {
      presetId: 'background-pink',
      displayName: '배경지 - 핑크',
    },
    isReviewExpanded: true,
    pendingDeleteCaptureId: 'capture-001',
    reviewStatus: 'ready',
  }
}

describe('Story 4.3 session reducer', () => {
  it('moves to post-end while preserving the current session context and closing capture-only review state', () => {
    const outcome: PostEndOutcome = {
      sessionId: 'session-43',
      actualShootEndAt: '2026-03-08T10:50:00.000Z',
      outcomeKind: 'handoff',
      guidanceMode: 'standard',
      sessionName: '김보라1234',
      showSessionName: true,
      handoffTargetLabel: '프런트 데스크',
    }

    const nextState = sessionReducer(createCaptureReadyState(), {
      type: 'post_end_started',
      outcome,
    })

    expect(nextState.phase).toBe('post-end')
    expect(nextState.activeSession?.sessionId).toBe('session-43')
    expect(nextState.captureConfidence?.latestPhoto.kind).toBe('ready')
    expect(nextState.postEndOutcome).toEqual(outcome)
    expect(nextState.isReviewExpanded).toBe(false)
    expect(nextState.pendingDeleteCaptureId).toBeNull()
  })
})
