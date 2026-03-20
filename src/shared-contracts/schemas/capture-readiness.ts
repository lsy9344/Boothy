import { z } from 'zod'

import { sessionIdSchema } from './ids'
import {
  captureReadinessSchemaVersion,
  captureReadinessUpdateSchemaVersion,
  captureRequestResultSchemaVersion,
  captureSurfaceStateSchema,
  sessionCaptureRecordSchema,
} from './session-capture'

const fallbackSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m' as const

export const customerReadinessStateSchema = z.enum([
  'Preparing',
  'Ready',
  'Preview Waiting',
  'Phone Required',
])

export const capturePrimaryActionSchema = z.enum([
  'wait',
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
})

export const captureReadinessSnapshotSchema = captureReadinessSnapshotInputSchema.transform(
  (snapshot) => {
    const sessionId = snapshot.sessionId ?? fallbackSessionId
    const latestCapture = snapshot.latestCapture ?? null

    return {
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
