import { describe, expect, it } from 'vitest'

import { initialSessionState, sessionReducer } from './sessionReducer.js'

describe('Story 3.2 capture request state', () => {
  it('marks capture requests as in flight and preserves the latest same-session preview while updating', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
        captureRequestStatus: 'idle',
        captureConfidence: {
          sessionId: 'session-32',
          revision: 4,
          updatedAt: '2026-03-08T10:00:06.000Z',
          shootEndsAt: '2026-03-08T10:50:00.000Z',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'ready',
            photo: {
              sessionId: 'session-32',
              captureId: 'capture-002',
              sequence: 2,
              assetUrl: 'asset://session-32/capture-002',
              capturedAt: '2026-03-08T10:00:05.000Z',
            },
          },
        },
      },
      {
        type: 'capture_request_started',
        captureId: 'capture-003',
        requestedAt: '2026-03-08T10:00:07.000Z',
      },
    )

    expect(nextState.captureRequestStatus).toBe('requesting')
    expect(nextState.captureConfidence).toMatchObject({
      revision: 5,
      updatedAt: '2026-03-08T10:00:07.000Z',
      latestPhoto: {
        kind: 'updating',
        nextCaptureId: 'capture-003',
        preview: {
          sessionId: 'session-32',
          captureId: 'capture-002',
        },
      },
    })
  })

  it('returns to idle once a fresh capture-confidence snapshot arrives', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
        readiness: {
          kind: 'ready',
          captureEnabled: true,
          branchPhoneNumber: '010-1234-5678',
          hostStatus: {
            sessionId: 'session-32',
            connectionState: 'ready',
            captureEnabled: true,
            lastStableCustomerState: 'ready',
            error: null,
            emittedAt: '2026-03-08T10:00:07.000Z',
          },
        },
        captureRequestStatus: 'requesting',
        captureConfidence: {
          sessionId: 'session-32',
          revision: 5,
          updatedAt: '2026-03-08T10:00:07.000Z',
          shootEndsAt: '2026-03-08T10:50:00.000Z',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'updating',
            nextCaptureId: 'capture-003',
          },
        },
      },
      {
        type: 'capture_confidence_updated',
        snapshot: {
          sessionId: 'session-32',
          revision: 6,
          updatedAt: '2026-03-08T10:00:10.000Z',
          shootEndsAt: '2026-03-08T10:50:00.000Z',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'ready',
            photo: {
              sessionId: 'session-32',
              captureId: 'capture-003',
              sequence: 3,
              assetUrl: 'asset://session-32/capture-003',
              capturedAt: '2026-03-08T10:00:10.000Z',
            },
          },
        },
      },
    )

    expect(nextState.captureRequestStatus).toBe('idle')
    expect(nextState.phase).toBe('capture-ready')
    expect(nextState.captureConfidence?.latestPhoto).toMatchObject({
      kind: 'ready',
      photo: {
        captureId: 'capture-003',
      },
    })
  })
})
