import { z } from 'zod'

import { sessionTypeSchema } from '../dto/sessionTiming.js'

export const sessionErrorCodeSchema = z.enum([
  'session_name.required',
  'session.validation_failed',
  'session.provisioning_failed',
])

export const sessionStartPayloadSchema = z.object({
  sessionName: z.string().trim().min(1),
  reservationStartAt: z.iso.datetime().optional(),
  sessionType: sessionTypeSchema.optional(),
})

export const sessionStartCommandPayloadSchema = sessionStartPayloadSchema.extend({
  branchId: z.string().trim().min(1),
})

export const sessionStartResultSchema = z.object({
  sessionId: z.string().min(1),
  sessionName: z.string().min(1),
  sessionFolder: z.string().min(1),
  manifestPath: z.string().min(1),
  createdAt: z.string().min(1),
  preparationState: z.literal('preparing'),
})

export const sessionResultEnvelopeSchema = z.discriminatedUnion('ok', [
  z.object({
    ok: z.literal(true),
    value: sessionStartResultSchema,
  }),
  z.object({
    ok: z.literal(false),
    errorCode: sessionErrorCodeSchema,
    message: z.string().min(1),
  }),
])

export type SessionStartPayloadSchema = z.infer<typeof sessionStartPayloadSchema>
export type SessionStartCommandPayloadSchema = z.infer<typeof sessionStartCommandPayloadSchema>
export type SessionStartResultSchema = z.infer<typeof sessionStartResultSchema>
export type SessionResultEnvelopeSchema = z.infer<typeof sessionResultEnvelopeSchema>
