import { describe, expect, it } from 'vitest'

import { cameraReadinessStatusSchema } from '../../src/shared-contracts/dto/cameraStatus.js'
import { schemaVersions } from '../../src/shared-contracts/dto/schemaVersion.js'

describe('camera readiness status contract', () => {
  it('accepts the approved waiting readiness state while reusing the host error envelope', () => {
    const parsed = cameraReadinessStatusSchema.parse({
      sessionId: 'session-001',
      connectionState: 'waiting',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: {
        schemaVersion: schemaVersions.errorEnvelope,
        code: 'camera.reconnecting',
        severity: 'warning',
        retryable: true,
        customerState: 'cameraReconnectNeeded',
        customerCameraConnectionState: 'needsAttention',
        operatorCameraConnectionState: 'reconnecting',
        operatorAction: 'checkCableAndRetry',
        message: 'Camera connection is unstable.',
        details: 'PTP session reopened after an adapter timeout.',
      },
      emittedAt: '2026-03-08T00:00:00.000Z',
    })

    expect(parsed.error).toMatchObject({
      schemaVersion: schemaVersions.errorEnvelope,
      code: 'camera.reconnecting',
      customerState: 'cameraReconnectNeeded',
      operatorAction: 'checkCableAndRetry',
    })
  })

  it('rejects the deprecated checking-camera readiness state', () => {
    const parsed = cameraReadinessStatusSchema.safeParse({
      sessionId: 'session-legacy',
      connectionState: 'checking-camera',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:00.000Z',
    })

    expect(parsed.success).toBe(false)
  })

  it('rejects a ready connection state when captureEnabled is false', () => {
    const parsed = cameraReadinessStatusSchema.safeParse({
      sessionId: 'session-inconsistent',
      connectionState: 'ready',
      captureEnabled: false,
      lastStableCustomerState: 'ready',
      error: null,
      emittedAt: '2026-03-08T00:00:00.000Z',
    })

    expect(parsed.success).toBe(false)
  })
})
