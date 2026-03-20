import { z } from 'zod'

export const hostErrorCodeSchema = z.enum([
  'capability-denied',
  'host-unavailable',
  'validation-error',
  'session-persistence-failed',
])

export const hostErrorEnvelopeSchema = z.object({
  code: hostErrorCodeSchema,
  message: z.string().min(1),
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
