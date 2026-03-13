import { describe, expect, it, vi } from 'vitest'

import {
  normalizeOperationalLogError,
  recordLifecycleEvent,
  recordOperatorIntervention,
} from './operationalLogClient.js'

describe('operationalLogClient', () => {
  it('sends validated lifecycle payloads through the host adapter command', async () => {
    const invoke = vi.fn(async () => undefined)

    await recordLifecycleEvent(
      {
        payloadVersion: 1,
        eventType: 'readiness_reached',
        occurredAt: '2026-03-08T12:10:00.000Z',
        branchId: 'branch-unconfigured',
        currentStage: 'camera-ready',
        sessionId: 'session-001',
      },
      { invoke },
    )

    expect(invoke).toHaveBeenCalledWith('record_lifecycle_event', {
      event: {
        payloadVersion: 1,
        eventType: 'readiness_reached',
        occurredAt: '2026-03-08T12:10:00.000Z',
        branchId: 'branch-unconfigured',
        currentStage: 'camera-ready',
        sessionId: 'session-001',
      },
    })
  })

  it('normalizes typed host errors for lifecycle and operator writes', async () => {
    const hostError = {
      code: 'diagnostics.invalidPayload',
      message: 'branchId is required',
      severity: 'error',
      retryable: false,
      surface: 'silent',
    }
    const invoke = vi.fn(async () => {
      throw hostError
    })

    await expect(
      recordOperatorIntervention(
        {
          payloadVersion: 1,
          occurredAt: '2026-03-08T12:11:00.000Z',
          branchId: 'branch-unconfigured',
          currentStage: 'operator-recovery',
          interventionOutcome: 'recovered',
        },
        { invoke },
      ),
    ).rejects.toEqual(normalizeOperationalLogError(hostError))
  })
})
