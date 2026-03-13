import { z } from 'zod'

import { sessionTimingStateSchema, sessionTypeSchema } from '../dto/sessionTiming.js'

const sessionTimingIdentitySchema = z
  .object({
    sessionId: z.string().min(1),
    manifestPath: z.string().min(1),
  })
  .strict()

export const initializeSessionTimingPayloadSchema = sessionTimingIdentitySchema
  .extend({
    reservationStartAt: z.iso.datetime(),
    sessionType: sessionTypeSchema,
    updatedAt: z.iso.datetime(),
  })
  .strict()

export const getSessionTimingPayloadSchema = sessionTimingIdentitySchema

export const extendSessionTimingPayloadSchema = sessionTimingIdentitySchema
  .extend({
    updatedAt: z.iso.datetime(),
  })
  .strict()

export const sessionTimingResultSchema = z
  .object({
    sessionId: z.string().min(1),
    manifestPath: z.string().min(1),
    timing: sessionTimingStateSchema,
  })
  .strict()

export const sessionTimingErrorCodeSchema = z.enum([
  'session_timing.invalid_payload',
  'session_timing.not_found',
  'session_timing.persistence_failed',
])

export const sessionTimingResultEnvelopeSchema = z.discriminatedUnion('ok', [
  z.object({
    ok: z.literal(true),
    value: sessionTimingResultSchema,
  }),
  z.object({
    ok: z.literal(false),
    errorCode: sessionTimingErrorCodeSchema,
    message: z.string().min(1),
  }),
])

export type InitializeSessionTimingPayload = z.infer<typeof initializeSessionTimingPayloadSchema>
export type GetSessionTimingPayload = z.infer<typeof getSessionTimingPayloadSchema>
export type ExtendSessionTimingPayload = z.infer<typeof extendSessionTimingPayloadSchema>
export type SessionTimingResult = z.infer<typeof sessionTimingResultSchema>
export type SessionTimingResultEnvelope = z.infer<typeof sessionTimingResultEnvelopeSchema>
