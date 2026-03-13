import { describe, expect, it, vi } from 'vitest'

import { createCameraAdapter } from '../../src/capture-adapter/host/cameraAdapter.js'
import { schemaVersions } from '../../src/shared-contracts/dto/schemaVersion.js'

class MockChannel<T> {
  constructor(private readonly handler: (message: T) => void) {}

  emit(message: T) {
    this.handler(message)
  }

  toJSON() {
    return '__CHANNEL__:101'
  }
}

describe('camera adapter host boundary', () => {
  it('routes readiness through typed invoke and channel wrappers instead of raw UI invoke', async () => {
    let channelRef: MockChannel<unknown> | undefined

    const invokeCommand = vi.fn(async (_command: string, args: Record<string, unknown>) => {
      channelRef = args.channel as MockChannel<unknown>
      channelRef.emit({
        schemaVersion: schemaVersions.protocol,
        requestId: 'req-ready-001',
        correlationId: 'corr-session-001',
        event: 'camera.statusChanged',
        sessionId: 'session-001',
        payload: {
          connectionState: 'connected',
          readiness: 'ready',
          lastUpdatedAt: '2026-03-08T09:00:00.000Z',
        },
      })

      return {
        schemaVersion: schemaVersions.contract,
        requestId: 'req-ready-001',
        correlationId: 'corr-session-001',
        ok: true,
        status: {
          connectionState: 'connected',
          readiness: 'ready',
          lastUpdatedAt: '2026-03-08T09:00:00.000Z',
        },
        manifestPath: 'C:/Boothy/Sessions/session-001/session-manifest.json',
      }
    })

    const adapter = createCameraAdapter({
      invokeCommand,
      createChannel: <T,>(handler: (message: T) => void) => new MockChannel(handler),
    })

    const progressEvents: unknown[] = []
    const result = await adapter.runReadinessCheck(
      {
        requestId: 'req-ready-001',
        correlationId: 'corr-session-001',
        sessionId: 'session-001',
      },
      (event) => {
        progressEvents.push(event)
      },
    )

    expect(invokeCommand).toHaveBeenCalledWith(
      'camera_run_readiness_flow',
      expect.objectContaining({
        payload: expect.objectContaining({
          method: 'camera.checkReadiness',
          requestId: 'req-ready-001',
        }),
        channel: expect.any(MockChannel),
      }),
    )
    expect(progressEvents).toEqual([
      {
        schemaVersion: schemaVersions.protocol,
        requestId: 'req-ready-001',
        correlationId: 'corr-session-001',
        event: 'camera.statusChanged',
        sessionId: 'session-001',
        payload: {
          connectionState: 'connected',
          readiness: 'ready',
          lastUpdatedAt: '2026-03-08T09:00:00.000Z',
        },
      },
    ])
    expect(result.status.readiness).toBe('ready')
  })
})
