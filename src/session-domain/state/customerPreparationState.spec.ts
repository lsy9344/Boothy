import { describe, expect, it } from 'vitest'

import { deriveCustomerPreparationState } from './customerPreparationState.js'

describe('deriveCustomerPreparationState', () => {
  it('keeps the host waiting state even when the error envelope contains cameraUnavailable', () => {
    const preparationState = deriveCustomerPreparationState(
      {
        sessionId: 'session-waiting',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: 'ready',
        error: {
          schemaVersion: 'boothy.camera.error-envelope.v1',
          code: 'camera.poll.retryable',
          severity: 'warning',
          retryable: true,
          customerState: 'cameraUnavailable',
          customerCameraConnectionState: 'needsAttention',
          operatorCameraConnectionState: 'reconnecting',
          operatorAction: 'checkCableAndRetry',
          message: 'Camera connection is unstable.',
        },
        emittedAt: '2026-03-13T09:00:00.000Z',
      },
      '010-1234-5678',
    )

    expect(preparationState).toMatchObject({
      kind: 'waiting',
      captureEnabled: false,
    })
  })

  it('rejects contradictory ready payloads when captureEnabled is false', () => {
    expect(() =>
      deriveCustomerPreparationState(
        {
          sessionId: 'session-inconsistent-ready',
          connectionState: 'ready',
          captureEnabled: false,
          lastStableCustomerState: 'ready',
          error: null,
          emittedAt: '2026-03-13T09:00:00.000Z',
        },
        '010-1234-5678',
      ),
    ).toThrowError(/normalized readiness payload/i)
  })
})
