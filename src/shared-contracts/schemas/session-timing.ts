import { z } from 'zod'

import { sessionIdSchema } from './ids'

export const sessionTimingSchemaVersion = 'session-timing/v1' as const

export const sessionTimingPhaseSchema = z.enum(['active', 'warning', 'ended'])

export const sessionTimingSnapshotSchema = z.object({
  schemaVersion: z.literal(sessionTimingSchemaVersion),
  sessionId: sessionIdSchema,
  adjustedEndAt: z.string().datetime(),
  warningAt: z.string().datetime(),
  phase: sessionTimingPhaseSchema,
  captureAllowed: z.boolean(),
  approvedExtensionMinutes: z.number().int().nonnegative(),
  approvedExtensionAuditRef: z.string().trim().min(1).nullable(),
  warningTriggeredAt: z.string().datetime().nullable(),
  endedTriggeredAt: z.string().datetime().nullable(),
})
