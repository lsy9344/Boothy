import { describe, expect, it } from 'vitest'

import { initialSessionState, sessionReducer, type SessionState } from './sessionReducer.js'

function createCaptureReadyState(): SessionState {
  return {
    ...initialSessionState,
    phase: 'capture-ready',
    activeSession: {
      sessionId: 'session-41',
      sessionName: '김보라1234',
      sessionFolder: 'C:/sessions/김보라1234',
      manifestPath: 'C:/sessions/김보라1234/session.json',
      createdAt: '2026-03-08T10:00:00.000Z',
      preparationState: 'preparing',
    },
    sessionTiming: {
      reservationStartAt: '2026-03-08T10:00:00.000Z',
      actualShootEndAt: '2026-03-08T10:20:00.000Z',
      sessionType: 'standard',
      operatorExtensionCount: 0,
      lastTimingUpdateAt: '2026-03-08T10:00:00.000Z',
    },
    captureConfidence: {
      sessionId: 'session-41',
      revision: 1,
      updatedAt: '2026-03-08T10:05:00.000Z',
      shootEndsAt: '2026-03-08T10:20:00.000Z',
      activePreset: {
        presetId: 'background-pink',
        label: '배경지 - 핑크',
      },
      latestPhoto: {
        kind: 'empty',
      },
    },
  }
}

describe('Story 4.1 session reducer', () => {
  it('reconciles session timing from later capture-confidence snapshots', () => {
    const nextState = sessionReducer(createCaptureReadyState(), {
      type: 'capture_confidence_updated',
      snapshot: {
        sessionId: 'session-41',
        revision: 2,
        updatedAt: '2026-03-08T10:08:00.000Z',
        shootEndsAt: '2026-03-08T10:25:00.000Z',
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        latestPhoto: {
          kind: 'empty',
        },
      },
    })

    expect(nextState.sessionTiming).toMatchObject({
      actualShootEndAt: '2026-03-08T10:25:00.000Z',
      lastTimingUpdateAt: '2026-03-08T10:08:00.000Z',
    })
  })

  it('seeds session timing from the host snapshot when the initial timing read has not completed', () => {
    const nextState = sessionReducer(
      {
        ...createCaptureReadyState(),
        sessionTiming: null,
      },
      {
        type: 'capture_confidence_updated',
        snapshot: {
          sessionId: 'session-41',
          revision: 2,
          updatedAt: '2026-03-08T10:08:00.000Z',
          shootEndsAt: '2026-03-08T10:25:00.000Z',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'empty',
          },
        },
      },
    )

    expect(nextState.sessionTiming).toMatchObject({
      reservationStartAt: '2026-03-08T10:00:00.000Z',
      actualShootEndAt: '2026-03-08T10:25:00.000Z',
      sessionType: 'standard',
      operatorExtensionCount: 0,
      lastTimingUpdateAt: '2026-03-08T10:08:00.000Z',
    })
  })

  it('clears session timing when the active session is cleared', () => {
    const nextState = sessionReducer(createCaptureReadyState(), {
      type: 'active_session_cleared',
    })

    expect(nextState.sessionTiming).toBeNull()
    expect(nextState.activeSession).toBeNull()
  })

  it('does not let a slower initial timing read overwrite newer snapshot timing for the active session', () => {
    const stateWithNewerSnapshot = sessionReducer(createCaptureReadyState(), {
      type: 'capture_confidence_updated',
      snapshot: {
        sessionId: 'session-41',
        revision: 2,
        updatedAt: '2026-03-08T10:08:00.000Z',
        shootEndsAt: '2026-03-08T10:25:00.000Z',
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        latestPhoto: {
          kind: 'empty',
        },
      },
    })

    const nextState = sessionReducer(stateWithNewerSnapshot, {
      type: 'session_timing_loaded',
      sessionId: 'session-41',
      timing: {
        reservationStartAt: '2026-03-08T10:00:00.000Z',
        actualShootEndAt: '2026-03-08T10:20:00.000Z',
        sessionType: 'standard',
        operatorExtensionCount: 0,
        lastTimingUpdateAt: '2026-03-08T10:01:00.000Z',
      },
    })

    expect(nextState.sessionTiming).toMatchObject({
      actualShootEndAt: '2026-03-08T10:25:00.000Z',
      lastTimingUpdateAt: '2026-03-08T10:08:00.000Z',
    })
  })
})
