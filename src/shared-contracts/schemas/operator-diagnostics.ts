import { z } from 'zod'

import { presetDisplayNameSchema, presetIdSchema, publishedVersionSchema } from './preset-core'
import { sessionIdSchema } from './ids'
import { sessionPostEndStateSchema } from './session-manifest'

export const operatorBlockedStateCategorySchema = z.enum([
  'capture-blocked',
  'preview-render-blocked',
  'timing-post-end-blocked',
  'not-blocked',
])

export const operatorSummaryStateSchema = z.enum([
  'no-session',
  'session-loaded',
])

export const operatorBoundaryStatusSchema = z.enum(['clear', 'blocked'])

const operatorSafeCopySchema = z.string().trim().min(1).max(160)

export const operatorBoundarySummarySchema = z.object({
  status: operatorBoundaryStatusSchema,
  title: operatorSafeCopySchema,
  detail: z.string().trim().min(1).max(240),
})

export const operatorRecentFailureSummarySchema = z.object({
  title: operatorSafeCopySchema,
  detail: z.string().trim().min(1).max(240),
  observedAt: z.string().datetime().nullable().optional(),
})

export const operatorSessionSummarySchema = z.object({
  schemaVersion: z.literal('operator-session-summary/v1'),
  state: operatorSummaryStateSchema,
  blockedStateCategory: operatorBlockedStateCategorySchema,
  sessionId: sessionIdSchema.nullable(),
  boothAlias: z.string().trim().min(1).nullable(),
  activePresetId: presetIdSchema.nullable().optional(),
  activePresetDisplayName: presetDisplayNameSchema.nullable().optional(),
  activePresetVersion: publishedVersionSchema.nullable().optional(),
  lifecycleStage: z.string().trim().min(1).nullable(),
  timingPhase: z.enum(['active', 'warning', 'ended']).nullable(),
  updatedAt: z.string().datetime().nullable(),
  postEndState: sessionPostEndStateSchema.nullable().optional(),
  recentFailure: operatorRecentFailureSummarySchema.nullable(),
  captureBoundary: operatorBoundarySummarySchema,
  previewRenderBoundary: operatorBoundarySummarySchema,
  completionBoundary: operatorBoundarySummarySchema,
})
