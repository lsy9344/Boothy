import { z } from 'zod'

import { sessionIdSchema } from './ids'
import { publishedVersionSchema } from './preset-core'

export const sessionCaptureSchemaVersion = 'session-capture/v1' as const
export const captureReadinessSchemaVersion = 'capture-readiness/v1' as const
export const captureReadinessUpdateSchemaVersion =
  'capture-readiness-update/v1' as const
export const captureRequestResultSchemaVersion =
  'capture-request-result/v1' as const

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

export const capturePostEndStateSchema = z.enum([
  'activeSession',
  'postEndPending',
  'handoffReady',
  'completed',
])

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
  activePresetVersion: publishedVersionSchema,
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
