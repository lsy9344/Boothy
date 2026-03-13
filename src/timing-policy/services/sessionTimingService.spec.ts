import { describe, expect, it, vi } from 'vitest'

import { createSessionTimingService } from './sessionTimingService.js'

describe('createSessionTimingService', () => {
  it('parses initialization results from the typed host command envelope', async () => {
    const invokeClient = vi.fn(async () => ({
      ok: true,
      value: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt: '2026-03-08T09:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:07:00.000Z',
        },
      },
    }))
    const service = createSessionTimingService(invokeClient)

    await expect(
      service.initializeSessionTiming({
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        sessionType: 'standard',
        updatedAt: '2026-03-08T09:07:00.000Z',
      }),
    ).resolves.toEqual({
      ok: true,
      value: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt: '2026-03-08T09:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:07:00.000Z',
        },
      },
    })
    expect(invokeClient).toHaveBeenCalledWith('initialize_session_timing', {
      payload: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        sessionType: 'standard',
        updatedAt: '2026-03-08T09:07:00.000Z',
      },
    })
  })

  it('routes operator extensions through the typed host command path', async () => {
    const invokeClient = vi.fn(async () => ({
      ok: true,
      value: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 1,
          lastTimingUpdateAt: '2026-03-08T09:30:00.000Z',
        },
      },
    }))
    const service = createSessionTimingService(invokeClient)

    await expect(
      service.extendSessionTiming({
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        updatedAt: '2026-03-08T09:30:00.000Z',
      }),
    ).resolves.toMatchObject({
      ok: true,
      value: {
        timing: {
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          operatorExtensionCount: 1,
        },
      },
    })
    expect(invokeClient).toHaveBeenCalledWith('extend_session_timing', {
      payload: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        updatedAt: '2026-03-08T09:30:00.000Z',
      },
    })
  })

  it('reads stored timing through a typed host command for customer-facing consumption', async () => {
    const invokeClient = vi.fn(async () => ({
      ok: true,
      value: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
        timing: {
          reservationStartAt: '2026-03-08T09:00:00.000Z',
          actualShootEndAt: '2026-03-08T09:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T09:07:00.000Z',
        },
      },
    }))
    const service = createSessionTimingService(invokeClient)

    await expect(
      service.getSessionTiming({
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
      }),
    ).resolves.toMatchObject({
      ok: true,
      value: {
        timing: {
          actualShootEndAt: '2026-03-08T09:50:00.000Z',
        },
      },
    })
    expect(invokeClient).toHaveBeenCalledWith('get_session_timing', {
      payload: {
        sessionId: 'session-1',
        manifestPath: 'C:/Boothy/Sessions/session-1/session-manifest.json',
      },
    })
  })
})
