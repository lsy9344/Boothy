import { z } from 'zod'

import { sessionIdSchema } from './ids'
import { presetIdSchema, publishedVersionSchema } from './preset-core'
import { captureIdSchema } from './session-capture'

const operatorSafeCopySchema = z.string().trim().min(1).max(160)
const operatorSafeDetailSchema = z.string().trim().min(1).max(240)
const operatorAuditReasonCodeSchema = z.string().trim().min(1).max(80)
const operatorAuditActorIdSchema = z
  .string()
  .trim()
  .regex(/^[a-z0-9][a-z0-9-]*$/i, '유효한 actorId 형식이 아니에요.')

export const operatorAuditEventCategorySchema = z.enum([
  'session-lifecycle',
  'timing-transition',
  'post-end-outcome',
  'operator-intervention',
  'publication-recovery',
  'release-governance',
  'critical-failure',
])

export const operatorAuditEventTypeSchema = z.enum([
  'session-started',
  'warning-triggered',
  'session-ended',
  'post-end-export-waiting',
  'post-end-completed',
  'post-end-phone-required',
  'retry',
  'approved-boundary-restart',
  'approved-time-extension',
  'route-phone-required',
  'publication-approved',
  'publication-published',
  'publication-rejected',
  'catalog-rollback',
  'branch-rollout-applied',
  'branch-rollout-deferred',
  'branch-rollout-rejected',
  'branch-rollback-applied',
  'branch-rollback-deferred',
  'branch-rollback-rejected',
])

export const operatorAuditEntrySchema = z.object({
  schemaVersion: z.literal('operator-audit-entry/v1'),
  eventId: z.string().trim().min(1).max(64),
  occurredAt: z.string().datetime(),
  sessionId: sessionIdSchema.nullable(),
  eventCategory: operatorAuditEventCategorySchema,
  eventType: operatorAuditEventTypeSchema,
  summary: operatorSafeCopySchema,
  detail: operatorSafeDetailSchema,
  actorId: operatorAuditActorIdSchema.nullable(),
  source: z.enum([
    'session-repository',
    'timing-policy',
    'post-end-evaluator',
    'operator-console',
    'preset-authoring',
    'preset-catalog',
    'branch-config',
  ]),
  captureId: captureIdSchema.nullable().optional(),
  presetId: presetIdSchema.nullable().optional(),
  publishedVersion: publishedVersionSchema.nullable().optional(),
  reasonCode: operatorAuditReasonCodeSchema.nullable().optional(),
})

export const operatorAuditQueryFilterSchema = z.object({
  sessionId: sessionIdSchema.nullable().optional(),
  eventCategories: z.array(operatorAuditEventCategorySchema).max(6).default([]),
  limit: z.number().int().min(1).max(50).default(20),
})

export const operatorAuditLatestOutcomeSchema = z.object({
  occurredAt: z.string().datetime(),
  eventCategory: operatorAuditEventCategorySchema,
  eventType: operatorAuditEventTypeSchema,
  summary: operatorSafeCopySchema,
})

export const operatorAuditQuerySummarySchema = z
  .object({
    totalEvents: z.number().int().nonnegative(),
    sessionLifecycleEvents: z.number().int().nonnegative(),
    timingTransitionEvents: z.number().int().nonnegative(),
    postEndOutcomeEvents: z.number().int().nonnegative(),
    operatorInterventionEvents: z.number().int().nonnegative(),
    publicationRecoveryEvents: z.number().int().nonnegative(),
    releaseGovernanceEvents: z.number().int().nonnegative().default(0),
    criticalFailureEvents: z.number().int().nonnegative(),
    latestOutcome: operatorAuditLatestOutcomeSchema.nullable(),
  })
  .superRefine((summary, context) => {
    const computedTotal =
      summary.sessionLifecycleEvents +
      summary.timingTransitionEvents +
      summary.postEndOutcomeEvents +
      summary.operatorInterventionEvents +
      summary.publicationRecoveryEvents +
      summary.releaseGovernanceEvents +
      summary.criticalFailureEvents

    if (summary.totalEvents !== computedTotal) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'totalEvents는 category별 집계 합계와 같아야 해요.',
        path: ['totalEvents'],
      })
    }
  })

export const operatorAuditQueryResultSchema = z.object({
  schemaVersion: z.literal('operator-audit-query-result/v1'),
  filter: operatorAuditQueryFilterSchema,
  events: z.array(operatorAuditEntrySchema),
  summary: operatorAuditQuerySummarySchema,
})
