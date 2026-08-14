import type { z } from 'zod'

import {
  imageQualityCapabilitySchema,
  sourceBlockOrderSchema,
  sourceCandidateSchema,
  sourceCompareModeSchema,
  sourceComparisonSampleSchema,
  sourceObjectRoleSchema,
  sourceRejectReasonSchema,
  sourceRouteSchema,
} from '../schemas'

export type SourceRoute = z.infer<typeof sourceRouteSchema>
export type SourceRejectReason = z.infer<typeof sourceRejectReasonSchema>
export type SourceObjectRole = z.infer<typeof sourceObjectRoleSchema>
export type SourceCandidate = z.infer<typeof sourceCandidateSchema>
export type ImageQualityCapability = z.infer<typeof imageQualityCapabilitySchema>
export type SourceBlockOrder = z.infer<typeof sourceBlockOrderSchema>
export type SourceComparisonSample = z.infer<
  typeof sourceComparisonSampleSchema
>
export type SourceCompareMode = z.infer<typeof sourceCompareModeSchema>
