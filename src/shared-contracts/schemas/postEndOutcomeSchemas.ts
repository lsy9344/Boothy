import { z } from 'zod'

import { postEndOutcomeSchema } from '../dto/postEndOutcome.js'

export const getPostEndOutcomePayloadSchema = z
  .object({
    sessionId: z.string().trim().min(1),
    manifestPath: z.string().trim().min(1),
  })
  .strict()

export const postEndOutcomeErrorCodeSchema = z.enum([
  'post_end.invalid_payload',
  'post_end.not_ready',
  'post_end.persistence_failed',
])

export const postEndOutcomeEnvelopeSchema = z.discriminatedUnion('ok', [
  z.object({
    ok: z.literal(true),
    value: postEndOutcomeSchema,
  }),
  z.object({
    ok: z.literal(false),
    errorCode: postEndOutcomeErrorCodeSchema,
    message: z.string().trim().min(1),
  }),
])

export type GetPostEndOutcomePayload = z.infer<typeof getPostEndOutcomePayloadSchema>
export type PostEndOutcomeEnvelope = z.infer<typeof postEndOutcomeEnvelopeSchema>
