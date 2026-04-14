import { z } from 'zod'

import { sessionIdSchema } from './ids'
import {
  presetIdSchema,
  publishedPresetRenderProfileSchema,
  publishedVersionSchema,
} from './preset-core'
import { captureIdSchema, captureRequestIdSchema } from './session-capture'

export const dedicatedRendererPreviewJobRequestSchemaVersion =
  'dedicated-renderer-preview-job-request/v1' as const
export const dedicatedRendererPreviewJobResultSchemaVersion =
  'dedicated-renderer-preview-job-result/v1' as const

export const runtimePathSchema = z.string().trim().min(1)

export const dedicatedRendererPreviewJobRequestSchema = z
  .object({
    schemaVersion: z.literal(dedicatedRendererPreviewJobRequestSchemaVersion),
    sessionId: sessionIdSchema,
    requestId: captureRequestIdSchema,
    captureId: captureIdSchema,
    presetId: presetIdSchema,
    publishedVersion: publishedVersionSchema,
    darktableVersion: z
      .string()
      .trim()
      .regex(/^\d+\.\d+\.\d+$/, 'darktable version 형식이 올바르지 않아요.'),
    xmpTemplatePath: runtimePathSchema,
    previewProfile: publishedPresetRenderProfileSchema,
    sourceAssetPath: runtimePathSchema,
    canonicalPreviewOutputPath: runtimePathSchema,
    diagnosticsDetailPath: runtimePathSchema,
  })
  .strict()

export const dedicatedRendererPreviewJobStatusSchema = z.enum([
  'accepted',
  'fallback-suggested',
  'queue-saturated',
  'protocol-mismatch',
  'invalid-output',
  'restarted',
])

export const previewRendererWarmStateSchema = z.enum([
  'warm-ready',
  'warm-hit',
  'cold',
  'warm-state-lost',
])

export const dedicatedRendererPreviewJobResultSchema = z
  .object({
    schemaVersion: z.literal(dedicatedRendererPreviewJobResultSchemaVersion),
    sessionId: sessionIdSchema,
    requestId: captureRequestIdSchema,
    captureId: captureIdSchema,
    status: dedicatedRendererPreviewJobStatusSchema,
    diagnosticsDetailPath: runtimePathSchema,
    outputPath: runtimePathSchema.nullable().optional(),
    detailCode: z.string().trim().min(1).nullable().optional(),
    detailMessage: z.string().trim().min(1).nullable().optional(),
    warmState: previewRendererWarmStateSchema.nullable().optional(),
    warmStateDetailPath: runtimePathSchema.nullable().optional(),
  })
  .strict()
