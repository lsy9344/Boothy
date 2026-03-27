import { z } from 'zod'

import { sessionIdSchema } from './ids'
import {
  captureDeleteResultSchemaVersion,
  captureReadinessSchemaVersion,
  captureReadinessUpdateSchemaVersion,
  captureRequestResultSchemaVersion,
  captureSurfaceStateSchema,
  sessionCaptureRecordSchema,
} from './session-capture'
import { sessionManifestSchema, sessionPostEndSchema } from './session-manifest'
import { sessionTimingSnapshotSchema } from './session-timing'

const fallbackSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m' as const

export const customerReadinessStateSchema = z.enum([
  'Preparing',
  'Ready',
  'Preview Waiting',
  'Export Waiting',
  'Completed',
  'Phone Required',
  'Session Ended',
])

export const capturePrimaryActionSchema = z.enum([
  'wait',
  'finish',
  'capture',
  'choose-preset',
  'start-session',
  'call-support',
])

export const captureReasonCodeSchema = z.enum([
  'session-missing',
  'preset-missing',
  'camera-preparing',
  'helper-preparing',
  'preview-waiting',
  'export-waiting',
  'completed',
  'phone-required',
  'warning',
  'ended',
  'ready',
  'blocked',
])

const customerGuidanceSchema = z.string().trim().min(1).max(120)

function inferSurfaceState(input: {
  canCapture: boolean
  reasonCode: z.infer<typeof captureReasonCodeSchema>
  latestCapture: z.infer<typeof sessionCaptureRecordSchema> | null
}) {
  if (input.latestCapture?.renderStatus === 'previewReady') {
    return 'previewReady' as const
  }

  if (input.latestCapture?.renderStatus === 'previewWaiting') {
    return 'previewWaiting' as const
  }

  if (input.reasonCode === 'preview-waiting') {
    return input.latestCapture === null ? 'captureSaved' : 'previewWaiting'
  }

  return input.canCapture ? 'captureReady' : 'blocked'
}

const captureReadinessSnapshotInputSchema = z.object({
  schemaVersion: z.literal(captureReadinessSchemaVersion).optional(),
  sessionId: sessionIdSchema.optional(),
  surfaceState: captureSurfaceStateSchema.optional(),
  customerState: customerReadinessStateSchema,
  canCapture: z.boolean(),
  primaryAction: capturePrimaryActionSchema,
  customerMessage: customerGuidanceSchema,
  supportMessage: customerGuidanceSchema,
  reasonCode: captureReasonCodeSchema,
  latestCapture: sessionCaptureRecordSchema.nullable().optional(),
  postEnd: sessionPostEndSchema.nullable().optional(),
  timing: sessionTimingSnapshotSchema.nullable().optional(),
})

export const captureReadinessSnapshotSchema = captureReadinessSnapshotInputSchema.transform(
  (snapshot) => {
    const sessionId = snapshot.sessionId ?? fallbackSessionId
    const latestCapture = snapshot.latestCapture ?? null

    const normalized: {
      schemaVersion: typeof captureReadinessSchemaVersion
      sessionId: string
      surfaceState: z.infer<typeof captureSurfaceStateSchema>
      customerState: z.infer<typeof customerReadinessStateSchema>
      canCapture: boolean
      primaryAction: z.infer<typeof capturePrimaryActionSchema>
      customerMessage: string
      supportMessage: string
      reasonCode: z.infer<typeof captureReasonCodeSchema>
      latestCapture: z.infer<typeof sessionCaptureRecordSchema> | null
      postEnd?: z.infer<typeof sessionPostEndSchema> | null
      timing?: z.infer<typeof sessionTimingSnapshotSchema> | null
    } = {
      schemaVersion: snapshot.schemaVersion ?? captureReadinessSchemaVersion,
      sessionId,
      surfaceState:
        snapshot.surfaceState ??
        inferSurfaceState({
          canCapture: snapshot.canCapture,
          reasonCode: snapshot.reasonCode,
          latestCapture,
        }),
      customerState: snapshot.customerState,
      canCapture: snapshot.canCapture,
      primaryAction: snapshot.primaryAction,
      customerMessage: snapshot.customerMessage,
      supportMessage: snapshot.supportMessage,
      reasonCode: snapshot.reasonCode,
      latestCapture,
    }

    if (snapshot.postEnd !== undefined) {
      normalized.postEnd = snapshot.postEnd
    }

    if (snapshot.timing !== undefined) {
      normalized.timing = snapshot.timing
    }

    return normalized
  },
)

export const captureReadinessInputSchema = z.object({
  sessionId: sessionIdSchema,
})

export const captureReadinessUpdateSchema = z.object({
  schemaVersion: z.literal(captureReadinessUpdateSchemaVersion).default(
    captureReadinessUpdateSchemaVersion,
  ),
  sessionId: sessionIdSchema,
  readiness: captureReadinessSnapshotSchema,
})

export const captureRequestInputSchema = z.object({
  sessionId: sessionIdSchema,
})

export const captureDeleteInputSchema = z.object({
  sessionId: sessionIdSchema,
  captureId: z.string().trim().min(1),
})

const captureRequestResultInputSchema = z.object({
  schemaVersion: z.literal(captureRequestResultSchemaVersion).optional(),
  sessionId: sessionIdSchema,
  status: z.literal('capture-saved'),
  capture: sessionCaptureRecordSchema,
  readiness: captureReadinessSnapshotSchema,
})

export const captureRequestResultSchema = captureRequestResultInputSchema.transform(
  (result) => ({
    schemaVersion: result.schemaVersion ?? captureRequestResultSchemaVersion,
    sessionId: result.sessionId,
    status: 'capture-saved' as const,
    capture: result.capture,
    readiness: result.readiness,
  }),
)

const captureDeleteResultInputSchema = z.object({
  schemaVersion: z.literal(captureDeleteResultSchemaVersion).optional(),
  sessionId: sessionIdSchema,
  captureId: z.string().trim().min(1),
  status: z.literal('capture-deleted'),
  manifest: sessionManifestSchema,
  readiness: captureReadinessSnapshotSchema,
})

export const captureDeleteResultSchema = captureDeleteResultInputSchema.transform(
  (result) => ({
    schemaVersion: result.schemaVersion ?? captureDeleteResultSchemaVersion,
    sessionId: result.sessionId,
    captureId: result.captureId,
    status: 'capture-deleted' as const,
    manifest: result.manifest,
    readiness: result.readiness,
  }),
)
