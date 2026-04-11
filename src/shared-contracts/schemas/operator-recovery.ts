import { z } from 'zod'

import { captureReasonCodeSchema, customerReadinessStateSchema } from './capture-readiness'
import { sessionIdSchema } from './ids'
import {
  operatorSessionSummarySchema,
  operatorSummaryStateSchema,
} from './operator-diagnostics'
import { sessionPostEndStateSchema } from './session-manifest'
import { sessionTimingPhaseSchema } from './session-timing'

const operatorSafeCopySchema = z.string().trim().min(1).max(160)

export const operatorRecoveryBlockedCategorySchema = z.enum([
  'capture',
  'preview-or-render',
  'timing-or-post-end',
])

export const operatorRecoveryActionSchema = z.enum([
  'retry',
  'approved-boundary-restart',
  'approved-time-extension',
  'route-phone-required',
])

export const operatorRecoveryActionStatusSchema = z.enum(['applied', 'rejected'])

export const operatorRecoveryActionRejectionReasonSchema = z.enum([
  'not-blocked',
  'action-not-allowed',
  'session-mismatch',
  'recovery-unavailable',
  'extension-limit-reached',
])

export const operatorRecoveryDiagnosticsSummarySchema = z.object({
  title: operatorSafeCopySchema,
  detail: z.string().trim().min(1).max(240),
  observedAt: z.string().datetime({ offset: true }).nullable().optional(),
})

export const operatorRecoverySummarySchema = operatorSessionSummarySchema
  .omit({
    schemaVersion: true,
  })
  .extend({
    schemaVersion: z.literal('operator-recovery-summary/v1'),
    blockedCategory: operatorRecoveryBlockedCategorySchema.nullable(),
    diagnosticsSummary: operatorRecoveryDiagnosticsSummarySchema.nullable(),
    allowedActions: z.array(operatorRecoveryActionSchema).max(4),
  })
  .superRefine((summary, context) => {
    if (summary.state === 'no-session') {
      if (summary.blockedCategory !== null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'no-session summary는 blockedCategory를 가질 수 없어요.',
          path: ['blockedCategory'],
        })
      }

      if (summary.allowedActions.length > 0) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'no-session summary는 recovery action을 노출할 수 없어요.',
          path: ['allowedActions'],
        })
      }
    }

    if (summary.blockedCategory === null && summary.allowedActions.length > 0) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'blockedCategory가 없으면 recovery action도 비어 있어야 해요.',
        path: ['allowedActions'],
      })
    }
  })

export const operatorRecoveryActionRequestSchema = z.object({
  sessionId: sessionIdSchema,
  action: operatorRecoveryActionSchema,
})

export const operatorRecoveryNextStateSchema = z.object({
  customerState: customerReadinessStateSchema,
  reasonCode: captureReasonCodeSchema,
  lifecycleStage: z.string().trim().min(1).nullable(),
  timingPhase: sessionTimingPhaseSchema.nullable(),
  postEndState: sessionPostEndStateSchema.nullable(),
})

export const operatorRecoveryActionResultSchema = z.object({
  schemaVersion: z.literal('operator-recovery-action-result/v1'),
  sessionId: sessionIdSchema,
  action: operatorRecoveryActionSchema,
  status: operatorRecoveryActionStatusSchema,
  message: z.string().trim().min(1).max(240),
  rejectionReason: operatorRecoveryActionRejectionReasonSchema.nullable().optional(),
  diagnosticsSummary: operatorRecoveryDiagnosticsSummarySchema.nullable(),
  nextState: operatorRecoveryNextStateSchema,
  summary: operatorRecoverySummarySchema,
})

export type OperatorRecoverySummaryState = z.infer<typeof operatorSummaryStateSchema>
