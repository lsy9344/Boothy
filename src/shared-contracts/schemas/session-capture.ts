import { z } from 'zod'

import { sessionIdSchema } from './ids'
import {
  presetDisplayNameSchema,
  presetIdSchema,
  publishedVersionSchema,
} from './preset-core'

export const sessionCaptureSchemaVersion = 'session-capture/v1' as const
export const captureReadinessSchemaVersion = 'capture-readiness/v1' as const
export const captureReadinessUpdateSchemaVersion =
  'capture-readiness-update/v1' as const
export const captureRequestResultSchemaVersion =
  'capture-request-result/v1' as const
export const captureDeleteResultSchemaVersion =
  'capture-delete-result/v1' as const

export const captureIdSchema = z.string().trim().min(1)
export const captureRequestIdSchema = z.string().trim().min(1)
export const captureEventTimeMsSchema = z.number().int().nonnegative()

export const captureRenderStatusSchema = z.enum([
  'captureSaved',
  'previewWaiting',
  'previewReady',
  'finalReady',
  'renderFailed',
])

const capturePostEndStateInputSchema = z.enum([
  'activeSession',
  'postEndPending',
  'completed',
  'localDeliverableReady',
  'handoffReady',
  'local-deliverable-ready',
  'handoff-ready',
])

export const capturePostEndStateSchema = capturePostEndStateInputSchema.transform(
  (value) => {
    switch (value) {
      case 'local-deliverable-ready':
        return 'localDeliverableReady' as const
      case 'handoff-ready':
        return 'handoffReady' as const
      default:
        return value
    }
  },
)

export const previewBudgetStateSchema = z.enum([
  'pending',
  'withinBudget',
  'exceededBudget',
])

export const rawCaptureAssetSchema = z.object({
  assetPath: z.string().trim().min(1),
  persistedAtMs: captureEventTimeMsSchema,
})

export const previewCaptureAssetSchema = z.object({
  assetPath: z.string().trim().min(1).nullable(),
  enqueuedAtMs: captureEventTimeMsSchema.nullable(),
  readyAtMs: captureEventTimeMsSchema.nullable(),
})

export const finalCaptureAssetSchema = z.object({
  assetPath: z.string().trim().min(1).nullable(),
  readyAtMs: captureEventTimeMsSchema.nullable(),
})

export const captureTimingMetricsSchema = z.object({
  captureAcknowledgedAtMs: captureEventTimeMsSchema,
  previewVisibleAtMs: captureEventTimeMsSchema.nullable(),
  captureBudgetMs: z.literal(1000),
  previewBudgetMs: z.literal(5000),
  previewBudgetState: previewBudgetStateSchema,
})

export const sessionCaptureRecordSchema = z.object({
  schemaVersion: z.literal(sessionCaptureSchemaVersion),
  sessionId: sessionIdSchema,
  boothAlias: z.string().trim().min(1),
  activePresetId: presetIdSchema.nullable().optional(),
  activePresetVersion: publishedVersionSchema,
  activePresetDisplayName: presetDisplayNameSchema.nullable().optional(),
  captureId: captureIdSchema,
  requestId: captureRequestIdSchema,
  raw: rawCaptureAssetSchema,
  preview: previewCaptureAssetSchema,
  final: finalCaptureAssetSchema,
  renderStatus: captureRenderStatusSchema,
  postEndState: capturePostEndStateSchema,
  timing: captureTimingMetricsSchema,
})

export const captureSurfaceStateSchema = z.enum([
  'captureReady',
  'captureSaved',
  'previewWaiting',
  'previewReady',
  'blocked',
])
