import { z } from 'zod'

const branchIdPattern = /^[a-z0-9][a-z0-9-]{1,47}$/i
const safeCopySchema = z.string().trim().min(1).max(240)
const actorIdSchema = z
  .string()
  .trim()
  .regex(/^[a-z0-9][a-z0-9-]*$/i, '유효한 승인자 ID 형식이 아니에요.')
const actorLabelSchema = z.string().trim().min(1).max(120)
const buildVersionPattern = /^boothy-\d{4}\.\d{2}\.\d{2}\.\d+$/
const presetStackVersionPattern = /^catalog-\d{4}\.\d{2}\.\d{2}$/

export const branchIdSchema = z
  .string()
  .trim()
  .regex(branchIdPattern, '유효한 지점 식별자가 아니에요.')

export const branchDisplayNameSchema = z.string().trim().min(1).max(80)

export const branchLocalSettingsFieldSchema = z.enum([
  'contact-phone',
  'contact-email',
  'contact-kakao',
  'support-hours',
  'bounded-operational-toggle',
])

export const branchReleaseBaselineSchema = z.object({
  buildVersion: z
    .string()
    .trim()
    .regex(buildVersionPattern, '승인된 build version 형식이 아니에요.'),
  presetStackVersion: z
    .string()
    .trim()
    .regex(
      presetStackVersionPattern,
      '승인된 preset stack version 형식이 아니에요.',
    ),
  approvedAt: z.string().datetime(),
  actorId: actorIdSchema,
  actorLabel: actorLabelSchema,
})

export const branchRolloutApprovalSchema = z.object({
  approvedAt: z.string().datetime(),
  actorId: actorIdSchema,
  actorLabel: actorLabelSchema,
})

export const branchLocalSettingsPreservationSchema = z.object({
  preservedFields: z
    .array(branchLocalSettingsFieldSchema)
    .min(1)
    .max(8)
    .transform((fields) => Array.from(new Set(fields))),
  summary: safeCopySchema,
})

export const branchSafeTransitionSchema = z.enum(['after-session-end'])

export const branchActiveSessionSchema = z.object({
  sessionId: z.string().trim().regex(/^session_[a-z0-9]{26}$/i),
  lockedBaseline: branchReleaseBaselineSchema,
  startedAt: z.string().datetime(),
  safeTransition: branchSafeTransitionSchema,
})

export const branchCompatibilityStatusSchema = z.enum([
  'compatible',
  'deferred-until-safe-transition',
  'incompatible',
])

export const branchCompatibilityVerdictSchema = z.object({
  status: branchCompatibilityStatusSchema,
  summary: safeCopySchema,
  sessionBaseline: branchReleaseBaselineSchema.nullable(),
  safeTransitionRequired: z.boolean(),
})

export const branchRolloutRejectionCodeSchema = z.enum([
  'active-session-deferred',
  'branch-not-found',
  'unapproved-target-baseline',
  'missing-rollback-baseline',
  'compatibility-check-failed',
  'audit-write-failed',
])

export const branchRolloutRejectionSchema = z.object({
  code: branchRolloutRejectionCodeSchema,
  message: safeCopySchema,
  guidance: safeCopySchema,
})

export const branchRolloutBranchStateSchema = z.object({
  branchId: branchIdSchema,
  displayName: branchDisplayNameSchema,
  deploymentBaseline: branchReleaseBaselineSchema,
  rollbackBaseline: branchReleaseBaselineSchema.nullable(),
  pendingBaseline: branchReleaseBaselineSchema.nullable(),
  localSettings: branchLocalSettingsPreservationSchema,
  activeSession: branchActiveSessionSchema.nullable(),
})

export const branchRolloutBranchResultSchema = z.object({
  branchId: branchIdSchema,
  displayName: branchDisplayNameSchema,
  result: z.enum(['applied', 'deferred', 'rejected']),
  effectiveBaseline: branchReleaseBaselineSchema,
  pendingBaseline: branchReleaseBaselineSchema.nullable(),
  localSettings: branchLocalSettingsPreservationSchema,
  compatibility: branchCompatibilityVerdictSchema,
  rejection: branchRolloutRejectionSchema.nullable(),
})

export const branchRolloutAuditEntrySchema = z.object({
  schemaVersion: z.literal('branch-rollout-audit-entry/v1'),
  auditId: z.string().trim().min(1).max(80),
  action: z.enum(['rollout', 'rollback']),
  requestedBranchIds: z.array(branchIdSchema).min(1).max(20),
  targetBaseline: branchReleaseBaselineSchema.nullable(),
  approval: branchRolloutApprovalSchema,
  outcomes: z.array(branchRolloutBranchResultSchema).min(1).max(20),
  notedAt: z.string().datetime(),
})

export const branchRolloutOverviewResultSchema = z.object({
  schemaVersion: z.literal('branch-rollout-overview/v1'),
  approvedBaselines: z.array(branchReleaseBaselineSchema).max(20),
  branches: z.array(branchRolloutBranchStateSchema).max(50),
  recentHistory: z.array(branchRolloutAuditEntrySchema).max(20),
})

export const branchRolloutInputSchema = z
  .object({
    branchIds: z.array(branchIdSchema).min(1).max(20),
    targetBaseline: branchReleaseBaselineSchema,
    actorId: actorIdSchema,
    actorLabel: actorLabelSchema,
  })
  .superRefine((value, context) => {
    const uniqueCount = new Set(value.branchIds).size

    if (uniqueCount !== value.branchIds.length) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '같은 지점을 중복해서 선택할 수 없어요.',
        path: ['branchIds'],
      })
    }

    if (
      value.targetBaseline.actorId !== value.actorId ||
      value.targetBaseline.actorLabel !== value.actorLabel
    ) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'target baseline 승인자와 실행 승인자가 일치해야 해요.',
        path: ['targetBaseline'],
      })
    }
  })

export const branchRollbackInputSchema = z
  .object({
    branchIds: z.array(branchIdSchema).min(1).max(20),
    actorId: actorIdSchema,
    actorLabel: actorLabelSchema,
  })
  .superRefine((value, context) => {
    if (new Set(value.branchIds).size !== value.branchIds.length) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '같은 지점을 중복해서 선택할 수 없어요.',
        path: ['branchIds'],
      })
    }
  })

export const branchRolloutActionResultSchema = z
  .object({
    schemaVersion: z.literal('branch-rollout-action-result/v1'),
    action: z.enum(['rollout', 'rollback']),
    requestedBranchIds: z.array(branchIdSchema).min(1).max(20),
    targetBaseline: branchReleaseBaselineSchema.nullable(),
    approval: branchRolloutApprovalSchema,
    outcomes: z.array(branchRolloutBranchResultSchema).min(1).max(20),
    auditEntry: branchRolloutAuditEntrySchema,
    message: safeCopySchema,
  })
  .superRefine((value, context) => {
    if (new Set(value.requestedBranchIds).size !== value.requestedBranchIds.length) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '같은 지점을 중복해서 선택할 수 없어요.',
        path: ['requestedBranchIds'],
      })
    }
  })
