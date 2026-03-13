import { describe, expect, it, vi } from 'vitest'

import { createLifecycleLogger } from './lifecycleLogger.js'

describe('createLifecycleLogger', () => {
  it('uses the same fallback branch identifier as the rest of the customer lifecycle logging', async () => {
    const invokeClient = vi.fn(async () => undefined)
    const logger = createLifecycleLogger(invokeClient)

    await logger.recordReadinessReached({
      sessionId: 'session-1',
      sessionName: '홍길동1234',
    })

    expect(invokeClient).toHaveBeenCalledWith('record_lifecycle_event', {
      event: expect.objectContaining({
        branchId: 'branch-unconfigured',
        currentStage: 'cameraReady',
        eventType: 'readiness_reached',
        sessionId: 'session-1',
        sessionName: '홍길동1234',
      }),
    })
  })

  it('records bounded preset catalog fallback audits through the lifecycle event path', async () => {
    const invokeClient = vi.fn(async () => undefined)
    const logger = createLifecycleLogger(invokeClient)

    await logger.recordPresetCatalogFallback?.({
      branchId: 'gangnam-main',
      reason: 'reordered_catalog',
      sessionId: 'session-7',
      sessionName: '홍길동5678',
    })

    expect(invokeClient).toHaveBeenCalledWith('record_lifecycle_event', {
      event: expect.objectContaining({
        branchId: 'gangnam-main',
        catalogFallbackReason: 'reordered_catalog',
        currentStage: 'presetSelection',
        eventType: 'preset_catalog_fallback',
        sessionId: 'session-7',
        sessionName: '홍길동5678',
      }),
    })
  })

  it('records warning and exact-end milestones with the authoritative shoot end time', async () => {
    const invokeClient = vi.fn(async () => undefined)
    const logger = createLifecycleLogger(invokeClient)

    await logger.recordWarningShown?.({
      branchId: 'gangnam-main',
      sessionId: 'session-7',
      sessionName: '홍길동5678',
      actualShootEndAt: '2026-03-08T09:50:00.000Z',
    })

    await logger.recordActualShootEnd?.({
      branchId: 'gangnam-main',
      sessionId: 'session-7',
      sessionName: '홍길동5678',
      actualShootEndAt: '2026-03-08T09:50:00.000Z',
    })

    expect(invokeClient).toHaveBeenNthCalledWith(1, 'record_lifecycle_event', {
      event: expect.objectContaining({
        actualShootEndAt: '2026-03-08T09:50:00.000Z',
        branchId: 'gangnam-main',
        currentStage: 'captureActive',
        eventType: 'warning_shown',
        sessionId: 'session-7',
        sessionName: '홍길동5678',
      }),
    })
    expect(invokeClient).toHaveBeenNthCalledWith(2, 'record_lifecycle_event', {
      event: expect.objectContaining({
        actualShootEndAt: '2026-03-08T09:50:00.000Z',
        branchId: 'gangnam-main',
        currentStage: 'captureActive',
        eventType: 'actual_shoot_end',
        sessionId: 'session-7',
        sessionName: '홍길동5678',
      }),
    })
  })
})
