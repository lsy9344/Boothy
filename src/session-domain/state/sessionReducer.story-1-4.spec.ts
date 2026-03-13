import { describe, expect, it } from 'vitest'

import { initialSessionState, sessionReducer } from './sessionReducer.js'

describe('Story 1.4 session reducer handoff guards', () => {
  it('clears the pending handoff session name when provisioning fails so the entry form can recover', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'provisioning',
        pendingSessionName: '  김보라 오후 세션  ',
        fields: {
          sessionName: '김보라 오후 세션',
        },
      },
      {
        type: 'provisioning_failed',
        errorCode: 'session.provisioning_failed',
      },
    )

    expect(nextState.phase).toBe('idle')
    expect(nextState.pendingSessionName).toBeNull()
    expect(nextState.fields.sessionName).toBe('김보라 오후 세션')
    expect(nextState.formErrorCode).toBe('session.provisioning_failed')
  })

  it('clears stale session-scoped state when provisioning fails after a prior active session existed', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
        activeSession: {
          sessionId: 'session-14',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/session-14',
          manifestPath: 'C:/sessions/session-14/session.json',
          createdAt: '2026-03-12T09:00:00.000Z',
          preparationState: 'preparing',
        },
        readiness: {
          kind: 'ready',
          captureEnabled: true,
          branchPhoneNumber: '010-1234-5678',
          hostStatus: {
            sessionId: 'session-14',
            connectionState: 'ready',
            captureEnabled: true,
            lastStableCustomerState: 'ready',
            error: null,
            emittedAt: '2026-03-12T09:00:05.000Z',
          },
        },
        captureConfidence: {
          sessionId: 'session-14',
          revision: 3,
          updatedAt: '2026-03-12T09:00:10.000Z',
          shootEndsAt: '2026-03-12T09:50:00.000Z',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'empty',
          },
        },
        selectedPresetId: 'background-pink',
        activePreset: {
          presetId: 'background-pink',
          displayName: '배경지 - 핑크',
        },
        pendingActivePresetId: 'background-pink',
      },
      {
        type: 'provisioning_failed',
        errorCode: 'session.provisioning_failed',
      },
    )

    expect(nextState.phase).toBe('idle')
    expect(nextState.activeSession).toBeNull()
    expect(nextState.readiness).toBeNull()
    expect(nextState.captureConfidence).toBeNull()
    expect(nextState.selectedPresetId).toBeNull()
    expect(nextState.activePreset).toBeNull()
    expect(nextState.pendingActivePresetId).toBeNull()
    expect(nextState.formErrorCode).toBe('session.provisioning_failed')
  })
})
