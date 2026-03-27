import type { z } from 'zod'

import {
  operatorAuditEntrySchema,
  operatorAuditEventCategorySchema,
  operatorAuditEventTypeSchema,
  operatorAuditLatestOutcomeSchema,
  operatorAuditQueryFilterSchema,
  operatorAuditQueryResultSchema,
  operatorAuditQuerySummarySchema,
  operatorBlockedStateCategorySchema,
  operatorBoundaryStatusSchema,
  operatorBoundarySummarySchema,
  operatorRecentFailureSummarySchema,
  operatorRecoveryActionRejectionReasonSchema,
  operatorRecoveryActionRequestSchema,
  operatorRecoveryActionResultSchema,
  operatorRecoveryActionSchema,
  operatorRecoveryActionStatusSchema,
  operatorRecoveryBlockedCategorySchema,
  operatorRecoveryDiagnosticsSummarySchema,
  operatorRecoveryNextStateSchema,
  operatorRecoverySummarySchema,
  operatorSessionSummarySchema,
  operatorSummaryStateSchema,
} from '../schemas'

export type OperatorAuditEventCategory = z.infer<
  typeof operatorAuditEventCategorySchema
>
export type OperatorAuditEventType = z.infer<typeof operatorAuditEventTypeSchema>
export type OperatorAuditEntry = z.infer<typeof operatorAuditEntrySchema>
export type OperatorAuditQueryFilter = z.infer<typeof operatorAuditQueryFilterSchema>
export type OperatorAuditLatestOutcome = z.infer<
  typeof operatorAuditLatestOutcomeSchema
>
export type OperatorAuditQuerySummary = z.infer<
  typeof operatorAuditQuerySummarySchema
>
export type OperatorAuditQueryResult = z.infer<
  typeof operatorAuditQueryResultSchema
>
export type OperatorBlockedStateCategory = z.infer<
  typeof operatorBlockedStateCategorySchema
>
export type OperatorSummaryState = z.infer<typeof operatorSummaryStateSchema>
export type OperatorBoundaryStatus = z.infer<typeof operatorBoundaryStatusSchema>
export type OperatorBoundarySummary = z.infer<typeof operatorBoundarySummarySchema>
export type OperatorRecentFailureSummary = z.infer<
  typeof operatorRecentFailureSummarySchema
>
export type OperatorSessionSummary = z.infer<typeof operatorSessionSummarySchema>
export type OperatorRecoveryBlockedCategory = z.infer<
  typeof operatorRecoveryBlockedCategorySchema
>
export type OperatorRecoveryAction = z.infer<typeof operatorRecoveryActionSchema>
export type OperatorRecoveryActionStatus = z.infer<
  typeof operatorRecoveryActionStatusSchema
>
export type OperatorRecoveryActionRejectionReason = z.infer<
  typeof operatorRecoveryActionRejectionReasonSchema
>
export type OperatorRecoveryDiagnosticsSummary = z.infer<
  typeof operatorRecoveryDiagnosticsSummarySchema
>
export type OperatorRecoverySummary = z.infer<
  typeof operatorRecoverySummarySchema
>
export type OperatorRecoveryActionRequest = z.infer<
  typeof operatorRecoveryActionRequestSchema
>
export type OperatorRecoveryNextState = z.infer<
  typeof operatorRecoveryNextStateSchema
>
export type OperatorRecoveryActionResult = z.infer<
  typeof operatorRecoveryActionResultSchema
>
