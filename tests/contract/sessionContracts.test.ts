import { describe, expect, it } from 'vitest'

import {
  sessionStartCommandPayloadSchema,
  sessionResultEnvelopeSchema,
  sessionStartPayloadSchema,
} from '../../src/shared-contracts/schemas/sessionSchemas.js'

describe('session contracts', () => {
  it('accepts the normalized session-start payload shape', () => {
    expect(
      sessionStartPayloadSchema.parse({
        sessionName: '김보라 오후 세션',
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        sessionType: 'couponExtended',
      }),
    ).toEqual({
      sessionName: '김보라 오후 세션',
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      sessionType: 'couponExtended',
    })
  })

  it('requires branch-scoped command payloads for the host start-session boundary', () => {
    expect(
      sessionStartCommandPayloadSchema.parse({
        branchId: 'gangnam-main',
        sessionName: '김보라 오후 세션',
        reservationStartAt: '2026-03-08T09:00:00.000Z',
        sessionType: 'couponExtended',
      }),
    ).toEqual({
      branchId: 'gangnam-main',
      sessionName: '김보라 오후 세션',
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      sessionType: 'couponExtended',
    })
  })

  it('accepts success and failure result envelopes for the host boundary', () => {
    expect(
      sessionResultEnvelopeSchema.parse({
        ok: true,
        value: {
          sessionId: '2026-03-08:김보라 오후 세션',
          sessionName: '김보라 오후 세션',
          sessionFolder: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션',
          manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/session.json',
          createdAt: '2026-03-08T00:00:00.000Z',
          preparationState: 'preparing',
        },
      }),
    ).toMatchObject({
      ok: true,
    })

    expect(
      sessionResultEnvelopeSchema.parse({
        ok: false,
        errorCode: 'session.validation_failed',
        message: 'Session details are invalid.',
      }),
    ).toEqual({
      ok: false,
      errorCode: 'session.validation_failed',
      message: 'Session details are invalid.',
    })

    expect(
      sessionResultEnvelopeSchema.parse({
        ok: false,
        errorCode: 'session.provisioning_failed',
        message: 'Unable to start the session right now.',
      }),
    ).toEqual({
      ok: false,
      errorCode: 'session.provisioning_failed',
      message: 'Unable to start the session right now.',
    })
  })
})
