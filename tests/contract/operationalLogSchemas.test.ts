import { describe, expect, it } from 'vitest'

import {
  lifecycleEventWriteSchema,
  operatorInterventionWriteSchema,
} from '../../src/shared-contracts/logging/operationalEvents.js'

describe('operational log schemas', () => {
  it('accepts the stable session_created lifecycle event code used by the host logger', () => {
    const parsed = lifecycleEventWriteSchema.parse({
      payloadVersion: 1,
      eventType: 'session_created',
      occurredAt: '2026-03-08T12:00:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'check-in',
      sessionId: '2026-03-08:홍길동1234',
      sessionName: '홍길동1234',
    })

    expect(parsed.eventType).toBe('session_created')
  })

  it('accepts the approved baseline lifecycle payload', () => {
    const parsed = lifecycleEventWriteSchema.parse({
      payloadVersion: 1,
      eventType: 'first_screen_displayed',
      occurredAt: '2026-03-08T12:00:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'customer-start',
      sessionName: 'Session 001',
    })

    expect(parsed).toEqual({
      payloadVersion: 1,
      eventType: 'first_screen_displayed',
      occurredAt: '2026-03-08T12:00:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'customer-start',
      sessionName: 'Session 001',
    })
  })

  it('accepts preset catalog fallback events only when a bounded reason code is provided', () => {
    const parsed = lifecycleEventWriteSchema.parse({
      payloadVersion: 1,
      eventType: 'preset_catalog_fallback',
      occurredAt: '2026-03-13T12:00:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'presetSelection',
      sessionId: 'session-001',
      catalogFallbackReason: 'reordered_catalog',
    })

    expect(parsed.catalogFallbackReason).toBe('reordered_catalog')

    const missingReason = lifecycleEventWriteSchema.safeParse({
      payloadVersion: 1,
      eventType: 'preset_catalog_fallback',
      occurredAt: '2026-03-13T12:00:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'presetSelection',
      sessionId: 'session-001',
    })

    expect(missingReason.success).toBe(false)
  })

  it('rejects disallowed sensitive lifecycle fields', () => {
    const result = lifecycleEventWriteSchema.safeParse({
      payloadVersion: 1,
      eventType: 'session_created',
      occurredAt: '2026-03-08T12:00:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'check-in',
      sessionId: 'session-001',
      fullPhoneNumber: '010-1234-5678',
      paymentData: { cardLast4: '1234' },
      rawReservationPayload: { guestName: 'Kim' },
    })

    expect(result.success).toBe(false)
    if (result.success) {
      return
    }

    expect(result.error.issues.map((issue) => issue.path.join('.'))).toEqual([
      'fullPhoneNumber',
      'paymentData',
      'rawReservationPayload',
    ])
  })

  it('rejects non-RFC3339 timestamps that JavaScript Date.parse would otherwise coerce', () => {
    const result = lifecycleEventWriteSchema.safeParse({
      payloadVersion: 1,
      eventType: 'session_created',
      occurredAt: '2026-03-08 12:00:00Z',
      branchId: 'branch-unconfigured',
      currentStage: 'check-in',
      sessionId: 'session-001',
    })

    expect(result.success).toBe(false)
    if (result.success) {
      return
    }

    expect(result.error.issues.map((issue) => issue.path.join('.'))).toContain('occurredAt')
  })

  it('requires intervention outcome for operator intervention writes', () => {
    const result = operatorInterventionWriteSchema.safeParse({
      payloadVersion: 1,
      occurredAt: '2026-03-08T12:05:00.000Z',
      branchId: 'branch-unconfigured',
      currentStage: 'operator-recovery',
      sessionId: 'session-001',
    })

    expect(result.success).toBe(false)
    if (result.success) {
      return
    }

    expect(result.error.issues.map((issue) => issue.path.join('.'))).toContain('interventionOutcome')
  })
})
