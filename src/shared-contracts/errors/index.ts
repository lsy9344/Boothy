import { z } from 'zod'

import { captureReadinessSnapshotSchema } from '../schemas'

export const hostErrorCodeSchema = z.enum([
  'capability-denied',
  'capture-not-ready',
  'host-unavailable',
  'preset-catalog-unavailable',
  'preset-not-available',
  'session-not-found',
  'validation-error',
  'session-persistence-failed',
])

export const hostErrorEnvelopeSchema = z.object({
  code: hostErrorCodeSchema,
  message: z.string().min(1),
  readiness: captureReadinessSnapshotSchema.optional(),
  fieldErrors: z
    .object({
      name: z.string().min(1).optional(),
      phoneLastFour: z.string().min(1).optional(),
    })
    .partial()
    .optional(),
})

export type HostErrorCode = z.infer<typeof hostErrorCodeSchema>
export type HostErrorEnvelope = z.infer<typeof hostErrorEnvelopeSchema>
