import { z } from 'zod'

import { previewRendererWarmStateSchema, runtimePathSchema } from './dedicated-renderer'
import { sessionIdSchema } from './ids'
import { presetIdSchema, publishedVersionSchema } from './preset-core'
import { captureIdSchema, captureRequestIdSchema } from './session-capture'

export const previewPromotionEvidenceRecordSchemaVersion =
  'preview-promotion-evidence-record/v1' as const
export const previewPromotionEvidenceBundleSchemaVersion =
  'preview-promotion-evidence-bundle/v1' as const
export const previewPromotionCanaryAssessmentSchemaVersion =
  'preview-promotion-canary-assessment/v1' as const

const optionalMetricSchema = z.number().int().nonnegative().nullable().optional()
const optionalRuntimePathSchema = runtimePathSchema.nullable().optional()
const implementationTrackSchema = z
  .enum(['actual-primary-lane', 'prototype-track'])
  .nullable()
const canaryCheckStatusSchema = z.enum(['pass', 'fail'])
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
const canaryHealthCheckSchema = z
  .object({
    status: canaryCheckStatusSchema,
    reason: z.string().trim().min(1),
  })
  .strict()
const canaryMetricCheckSchema = canaryHealthCheckSchema
  .extend({
    actualMs: z.number().int().nonnegative().nullable(),
    thresholdMs: z.number().int().positive(),
  })
  .strict()
const canaryFallbackCheckSchema = canaryHealthCheckSchema
  .extend({
    actualRatio: z.number().min(0).max(1),
    thresholdRatio: z.number().min(0).max(1),
  })
  .strict()
const canaryFidelityCheckSchema = canaryHealthCheckSchema
  .extend({
    parityResult: z.enum(['pass', 'conditional', 'not-run', 'fail']),
  })
  .strict()
const canaryRollbackCheckSchema = canaryHealthCheckSchema
  .extend({
    evidenceCount: z.number().int().nonnegative(),
  })
  .strict()

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
    implementationTrack: implementationTrackSchema.optional(),
    warmState: previewRendererWarmStateSchema.nullable().optional(),
    captureRequestedAtMs: z.number().int().nonnegative().optional(),
    rawPersistedAtMs: z.number().int().nonnegative().optional(),
    truthfulArtifactReadyAtMs: z.number().int().nonnegative().optional(),
    visibleOwner: z.string().trim().min(1).optional(),
    visibleOwnerTransitionAtMs: z.number().int().nonnegative().optional(),
    firstVisibleMs: optionalMetricSchema,
    sameCaptureFullScreenVisibleMs: optionalMetricSchema,
    replacementMs: optionalMetricSchema,
    originalVisibleToPresetAppliedVisibleMs: optionalMetricSchema,
    sessionManifestPath: runtimePathSchema,
    timingEventsPath: runtimePathSchema,
    routePolicySnapshotPath: runtimePathSchema,
    publishedBundlePath: runtimePathSchema.nullable().optional(),
    catalogStatePath: runtimePathSchema,
    previewAssetPath: runtimePathSchema.nullable().optional(),
    warmStateDetailPath: runtimePathSchema.nullable().optional(),
    improvementSummary: z.string().trim().min(1).optional(),
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
    implementationTrack: implementationTrackSchema.optional(),
    warmState: previewRendererWarmStateSchema.nullable().optional(),
    captureRequestedAtMs: z.number().int().nonnegative(),
    rawPersistedAtMs: z.number().int().nonnegative(),
    truthfulArtifactReadyAtMs: z.number().int().nonnegative(),
    visibleOwner: z.string().trim().min(1),
    visibleOwnerTransitionAtMs: z.number().int().nonnegative(),
    firstVisibleMs: optionalMetricSchema,
    sameCaptureFullScreenVisibleMs: optionalMetricSchema,
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

export const previewPromotionCanaryAssessmentSchema = z
  .object({
    schemaVersion: z.literal(previewPromotionCanaryAssessmentSchemaVersion),
    generatedAt: z.string().datetime({ offset: true }),
    bundleManifestPath: runtimePathSchema,
    sessionId: sessionIdSchema,
    captureId: captureIdSchema,
    requestId: captureRequestIdSchema,
    presetId: presetIdSchema,
    publishedVersion: publishedVersionSchema,
    routeStage: z.string().trim().min(1),
    implementationTrack: implementationTrackSchema.optional(),
    laneOwner: z.string().trim().min(1),
    gate: z.enum(['Go', 'No-Go']),
    nextStageAllowed: z.boolean(),
    summary: z.string().trim().min(1),
    blockers: z.array(z.string().trim().min(1)).max(16),
    checks: z
      .object({
        kpi: canaryMetricCheckSchema,
        fallbackStability: canaryFallbackCheckSchema,
        wrongCapture: canaryHealthCheckSchema,
        fidelityDrift: canaryFidelityCheckSchema,
        rollbackReadiness: canaryRollbackCheckSchema,
        activeSessionSafety: canaryHealthCheckSchema,
      })
      .strict(),
  })
  .strict()
