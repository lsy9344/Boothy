import { z } from 'zod'

import { previewRendererWarmStateSchema, runtimePathSchema } from './dedicated-renderer'
import { sessionIdSchema } from './ids'
import { presetIdSchema, publishedVersionSchema } from './preset-core'
import { captureIdSchema, captureRequestIdSchema } from './session-capture'

export const previewPromotionEvidenceRecordSchemaVersion =
  'preview-promotion-evidence-record/v1' as const
export const previewPromotionEvidenceBundleSchemaVersion =
  'preview-promotion-evidence-bundle/v1' as const

const optionalMetricSchema = z.number().int().nonnegative().nullable().optional()
const optionalRuntimePathSchema = runtimePathSchema.nullable().optional()
const parityMeasurementSchema = z
  .object({
    status: z.enum(['not-run', 'invalid-input', 'measured']),
    result: z.enum(['not-run', 'pass', 'fail']),
    referencePath: optionalRuntimePathSchema,
    referenceMetadataPath: optionalRuntimePathSchema,
    threshold: z.number().nonnegative(),
    numericScore: z.number().nonnegative().nullable().optional(),
    maxChannelDelta: z.number().int().nonnegative().nullable().optional(),
    reason: z.string().trim().min(1),
  })
  .strict()
const bundleArtifactSchema = z
  .object({
    source: optionalRuntimePathSchema,
    destination: runtimePathSchema,
  })
  .catchall(z.unknown())

export const previewPromotionEvidenceRecordSchema = z
  .object({
    schemaVersion: z.literal(previewPromotionEvidenceRecordSchemaVersion),
    observedAt: z.string().datetime({ offset: true }),
    sessionId: sessionIdSchema,
    requestId: captureRequestIdSchema,
    captureId: captureIdSchema,
    presetId: presetIdSchema.nullable().optional(),
    publishedVersion: publishedVersionSchema,
    laneOwner: z.string().trim().min(1),
    fallbackReasonCode: z.string().trim().min(1).nullable().optional(),
    routeStage: z.string().trim().min(1),
    warmState: previewRendererWarmStateSchema.nullable().optional(),
    firstVisibleMs: optionalMetricSchema,
    replacementMs: optionalMetricSchema,
    originalVisibleToPresetAppliedVisibleMs: optionalMetricSchema,
    sessionManifestPath: runtimePathSchema,
    timingEventsPath: runtimePathSchema,
    routePolicySnapshotPath: runtimePathSchema,
    publishedBundlePath: runtimePathSchema.nullable().optional(),
    catalogStatePath: runtimePathSchema,
    previewAssetPath: runtimePathSchema.nullable().optional(),
    warmStateDetailPath: runtimePathSchema.nullable().optional(),
  })
  .strict()

export const previewPromotionEvidenceBundleSchema = z
  .object({
    schemaVersion: z.literal(previewPromotionEvidenceBundleSchemaVersion),
    generatedAt: z.string().datetime({ offset: true }),
    sessionId: sessionIdSchema,
    captureId: captureIdSchema,
    requestId: captureRequestIdSchema,
    presetId: presetIdSchema,
    publishedVersion: publishedVersionSchema,
    laneOwner: z.string().trim().min(1),
    fallbackReasonCode: z.string().trim().min(1).nullable().optional(),
    routeStage: z.string().trim().min(1),
    warmState: previewRendererWarmStateSchema.nullable().optional(),
    firstVisibleMs: optionalMetricSchema,
    replacementMs: optionalMetricSchema,
    originalVisibleToPresetAppliedVisibleMs: optionalMetricSchema,
    fallbackRatio: z.number().min(0).max(1),
    outputRoot: runtimePathSchema,
    bundleManifestPath: runtimePathSchema,
    artifacts: z.record(z.string(), bundleArtifactSchema),
    missingArtifacts: z.array(z.string()),
    visualEvidence: z
      .object({
        booth: z.array(runtimePathSchema).min(1),
        operator: z.array(runtimePathSchema).min(1),
      })
      .strict(),
    rollbackEvidence: z.array(runtimePathSchema).min(1),
    parity: z
      .object({
        result: z.enum(['pass', 'conditional', 'not-run', 'fail']),
        reason: z.string().trim().min(1),
        threshold: z.number().nonnegative(),
        baseline: parityMeasurementSchema,
        fallback: parityMeasurementSchema,
      })
      .strict(),
  })
  .strict()
