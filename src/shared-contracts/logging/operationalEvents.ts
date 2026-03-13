import { z } from 'zod'

const isoTimestampSchema = z.string().trim().pipe(z.iso.datetime())

const requiredText = (maxLength: number) => z.string().trim().min(1).max(maxLength)
const optionalText = (maxLength: number) => requiredText(maxLength).optional()

export const catalogFallbackReasonSchema = z.enum([
  'invalid_id',
  'invalid_catalog_shape',
  'missing_catalog_input',
  'name_mismatch',
  'oversized_catalog',
  'reordered_catalog',
])

const sensitiveFieldGuards = {
  fullPhoneNumber: z.never().optional(),
  paymentData: z.never().optional(),
  rawReservationPayload: z.never().optional(),
} as const

const operationalContextSchema = z
  .object({
    payloadVersion: z.literal(1),
    occurredAt: isoTimestampSchema,
    branchId: requiredText(120),
    sessionId: optionalText(120),
    sessionName: optionalText(160),
    currentStage: requiredText(80),
    actualShootEndAt: isoTimestampSchema.optional(),
    catalogFallbackReason: catalogFallbackReasonSchema.optional(),
    extensionStatus: optionalText(80),
    recentFaultCategory: optionalText(120),
    ...sensitiveFieldGuards,
  })
  .strict()

export const lifecycleEventKindSchema = z.enum([
  'first_screen_displayed',
  'session_created',
  'readiness_reached',
  'warning_shown',
  'actual_shoot_end',
  'export_state_changed',
  'preset_catalog_fallback',
  'session_completed',
  'phone_required',
])

export const lifecycleEventWriteSchema = operationalContextSchema
  .extend({
    eventType: lifecycleEventKindSchema,
  })
  .strict()
  .superRefine((event, context) => {
    if (event.eventType === 'preset_catalog_fallback' && !event.catalogFallbackReason) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'catalogFallbackReason is required for preset_catalog_fallback events.',
        path: ['catalogFallbackReason'],
      })
    }
  })

export const operatorInterventionWriteSchema = operationalContextSchema
  .extend({
    interventionOutcome: requiredText(120),
  })
  .strict()

export type LifecycleEventWrite = z.infer<typeof lifecycleEventWriteSchema>
export type OperatorInterventionWrite = z.infer<typeof operatorInterventionWriteSchema>
export type CatalogFallbackReason = z.infer<typeof catalogFallbackReasonSchema>
