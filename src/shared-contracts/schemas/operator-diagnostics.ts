import { z } from 'zod'

const previewArchitectureImplementationTrackSchema = z
  .enum(['actual-primary-lane', 'prototype-track'])
  .nullable()

import { liveCaptureTruthSchema } from './capture-readiness'
import { previewRendererWarmStateSchema } from './dedicated-renderer'
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

export const operatorCameraConnectionStateSchema = z.enum([
  'disconnected',
  'connecting',
  'connected',
  'recovery-required',
])

export const operatorBoundarySummarySchema = z.object({
  status: operatorBoundaryStatusSchema,
  title: operatorSafeCopySchema,
  detail: z.string().trim().min(1).max(240),
})

export const operatorCameraConnectionSummarySchema = z.object({
  state: operatorCameraConnectionStateSchema,
  title: operatorSafeCopySchema,
  detail: z.string().trim().min(1).max(240),
  observedAt: z.string().datetime({ offset: true }).nullable().optional(),
})

export const operatorRecentFailureSummarySchema = z.object({
  title: operatorSafeCopySchema,
  detail: z.string().trim().min(1).max(240),
  observedAt: z.string().datetime({ offset: true }).nullable().optional(),
})

export const operatorPreviewArchitectureSummarySchema = z.object({
    route: z.string().trim().min(1).nullable().optional(),
    routeStage: z.string().trim().min(1).nullable().optional(),
    implementationTrack: previewArchitectureImplementationTrackSchema.optional(),
    laneOwner: z.string().trim().min(1).nullable().optional(),
    fallbackReasonCode: z.string().trim().min(1).nullable().optional(),
    captureId: z.string().trim().min(1).nullable().optional(),
    requestId: z.string().trim().min(1).nullable().optional(),
    visibleOwner: z.string().trim().min(1).nullable().optional(),
    visibleOwnerTransitionAtMs: z.number().int().nonnegative().nullable().optional(),
    warmState: previewRendererWarmStateSchema.nullable().optional(),
    warmStateObservedAt: z.string().datetime({ offset: true }).nullable().optional(),
    firstVisibleMs: z.number().int().nonnegative().nullable().optional(),
    sameCaptureFullScreenVisibleMs: z
      .number()
      .int()
      .nonnegative()
      .nullable()
      .optional(),
    replacementMs: z.number().int().nonnegative().nullable().optional(),
    originalVisibleToPresetAppliedVisibleMs: z
      .number()
      .int()
      .nonnegative()
      .nullable()
      .optional(),
    hardwareCapability: z.string().trim().min(1),
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
  cameraConnection: operatorCameraConnectionSummarySchema,
  captureBoundary: operatorBoundarySummarySchema,
  previewRenderBoundary: operatorBoundarySummarySchema,
  completionBoundary: operatorBoundarySummarySchema,
  previewArchitecture: operatorPreviewArchitectureSummarySchema,
  liveCaptureTruth: liveCaptureTruthSchema.optional(),
})
