import type { z } from 'zod'

import {
  branchActiveSessionSchema,
  branchCompatibilityStatusSchema,
  branchCompatibilityVerdictSchema,
  branchDisplayNameSchema,
  branchIdSchema,
  branchLocalSettingsFieldSchema,
  branchLocalSettingsPreservationSchema,
  branchReleaseBaselineSchema,
  branchRollbackInputSchema,
  branchRolloutActionResultSchema,
  branchRolloutApprovalSchema,
  branchRolloutAuditEntrySchema,
  branchRolloutBranchResultSchema,
  branchRolloutBranchStateSchema,
  branchRolloutInputSchema,
  branchRolloutOverviewResultSchema,
  branchRolloutRejectionCodeSchema,
  branchRolloutRejectionSchema,
  branchSafeTransitionSchema,
} from '../schemas'

export type BranchId = z.infer<typeof branchIdSchema>
export type BranchDisplayName = z.infer<typeof branchDisplayNameSchema>
export type BranchLocalSettingsField = z.infer<
  typeof branchLocalSettingsFieldSchema
>
export type BranchReleaseBaseline = z.infer<typeof branchReleaseBaselineSchema>
export type BranchRolloutApproval = z.infer<typeof branchRolloutApprovalSchema>
export type BranchLocalSettingsPreservation = z.infer<
  typeof branchLocalSettingsPreservationSchema
>
export type BranchSafeTransition = z.infer<typeof branchSafeTransitionSchema>
export type BranchActiveSession = z.infer<typeof branchActiveSessionSchema>
export type BranchCompatibilityStatus = z.infer<
  typeof branchCompatibilityStatusSchema
>
export type BranchCompatibilityVerdict = z.infer<
  typeof branchCompatibilityVerdictSchema
>
export type BranchRolloutRejectionCode = z.infer<
  typeof branchRolloutRejectionCodeSchema
>
export type BranchRolloutRejection = z.infer<
  typeof branchRolloutRejectionSchema
>
export type BranchRolloutBranchState = z.infer<
  typeof branchRolloutBranchStateSchema
>
export type BranchRolloutBranchResult = z.infer<
  typeof branchRolloutBranchResultSchema
>
export type BranchRolloutAuditEntry = z.infer<
  typeof branchRolloutAuditEntrySchema
>
export type BranchRolloutOverviewResult = z.infer<
  typeof branchRolloutOverviewResultSchema
>
export type BranchRolloutInput = z.infer<typeof branchRolloutInputSchema>
export type BranchRollbackInput = z.infer<typeof branchRollbackInputSchema>
export type BranchRolloutActionResult = z.infer<
  typeof branchRolloutActionResultSchema
>
