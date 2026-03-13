import { z } from 'zod'

import { cameraStatusSnapshotSchema } from './cameraStatus.js'
import { normalizedErrorEnvelopeSchema } from './errorEnvelope.js'
import { contractSchemaVersionSchema, protocolSchemaVersionSchema, schemaVersions } from './schemaVersion.js'

const nonEmptyStringSchema = z.string().min(1)

export const mockScenarioSchema = z.enum(['readinessSuccess', 'readinessDegraded', 'normalizedError'])

export const readinessCommandInputSchema = z
  .object({
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    sessionId: nonEmptyStringSchema.optional(),
    desiredCameraId: nonEmptyStringSchema.optional(),
    mockScenario: mockScenarioSchema.optional(),
  })
  .strict()

export const cameraCommandPayloadSchema = z
  .object({
    desiredCameraId: nonEmptyStringSchema.optional(),
    mockScenario: mockScenarioSchema.optional(),
  })
  .strict()

export const captureActivePresetSchema = z
  .object({
    presetId: nonEmptyStringSchema,
    label: nonEmptyStringSchema,
  })
  .strict()

export const captureCommandInputSchema = z
  .object({
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    sessionId: nonEmptyStringSchema,
    activePreset: captureActivePresetSchema,
  })
  .strict()

export const captureCommandPayloadSchema = z
  .object({
    activePreset: captureActivePresetSchema,
  })
  .strict()

export const sidecarCaptureCommandPayloadSchema = z
  .object({
    activePreset: captureActivePresetSchema,
    captureId: nonEmptyStringSchema,
    originalFileName: nonEmptyStringSchema,
    processedFileName: nonEmptyStringSchema,
    originalOutputPath: nonEmptyStringSchema,
    processedOutputPath: nonEmptyStringSchema,
  })
  .strict()

export const cameraCommandRequestSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    method: z.literal('camera.checkReadiness'),
    sessionId: nonEmptyStringSchema.optional(),
    payload: cameraCommandPayloadSchema,
  })
  .strict()

export const captureCommandRequestSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    method: z.literal('camera.capture'),
    sessionId: nonEmptyStringSchema,
    payload: captureCommandPayloadSchema,
  })
  .strict()

export const sidecarCaptureRequestSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    method: z.literal('camera.capture'),
    sessionId: nonEmptyStringSchema,
    payload: sidecarCaptureCommandPayloadSchema,
  })
  .strict()

export const cameraStatusChangedEventSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    event: z.literal('camera.statusChanged'),
    sessionId: nonEmptyStringSchema.optional(),
    payload: cameraStatusSnapshotSchema,
  })
  .strict()

export const captureProgressEventSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    event: z.literal('capture.progress'),
    sessionId: nonEmptyStringSchema.optional(),
    payload: z
      .object({
        stage: z.enum(['captureStarted', 'captureCompleted']),
        captureId: nonEmptyStringSchema,
        percentComplete: z.number().int().min(0).max(100),
        lastUpdatedAt: z.iso.datetime(),
      })
      .strict(),
  })
  .strict()

export const exportProgressEventSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    event: z.literal('export.progress'),
    sessionId: nonEmptyStringSchema.optional(),
    payload: z
      .object({
        stage: z.literal('exportQueued'),
        percentComplete: z.number().int().min(0).max(100),
        lastUpdatedAt: z.iso.datetime(),
      })
      .strict(),
  })
  .strict()

export const sidecarSuccessResponseSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    ok: z.literal(true),
    status: cameraStatusSnapshotSchema,
  })
  .strict()

export const sidecarErrorResponseSchema = z
  .object({
    schemaVersion: protocolSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    ok: z.literal(false),
    error: normalizedErrorEnvelopeSchema,
  })
  .strict()

export const sidecarCaptureSuccessResponseSchema = z
  .object({
    schemaVersion: contractSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    ok: z.literal(true),
    sessionId: nonEmptyStringSchema,
    captureId: nonEmptyStringSchema,
    originalFileName: nonEmptyStringSchema,
    processedFileName: nonEmptyStringSchema,
    capturedAt: z.iso.datetime(),
    manifestPath: nonEmptyStringSchema,
  })
  .strict()

export const sidecarMessageSchema = z.union([
  sidecarCaptureRequestSchema,
  cameraCommandRequestSchema,
  cameraStatusChangedEventSchema,
  captureProgressEventSchema,
  exportProgressEventSchema,
  sidecarSuccessResponseSchema,
  sidecarCaptureSuccessResponseSchema,
  sidecarErrorResponseSchema,
])

export const cameraCommandResultSchema = z
  .object({
    schemaVersion: contractSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    ok: z.boolean(),
    status: cameraStatusSnapshotSchema,
    manifestPath: nonEmptyStringSchema.optional(),
    error: normalizedErrorEnvelopeSchema.optional(),
  })
  .strict()

export const captureCommandResultSchema = z
  .object({
    schemaVersion: contractSchemaVersionSchema,
    requestId: nonEmptyStringSchema,
    correlationId: nonEmptyStringSchema,
    ok: z.boolean(),
    sessionId: nonEmptyStringSchema,
    captureId: nonEmptyStringSchema,
    originalFileName: nonEmptyStringSchema,
    processedFileName: nonEmptyStringSchema,
    capturedAt: z.iso.datetime(),
    manifestPath: nonEmptyStringSchema,
  })
  .strict()

export function buildReadinessCommandRequest(input: z.input<typeof readinessCommandInputSchema>) {
  const parsed = readinessCommandInputSchema.parse(input)

  return cameraCommandRequestSchema.parse({
    schemaVersion: schemaVersions.protocol,
    requestId: parsed.requestId,
    correlationId: parsed.correlationId,
    method: 'camera.checkReadiness',
    sessionId: parsed.sessionId,
    payload: {
      desiredCameraId: parsed.desiredCameraId,
      mockScenario: parsed.mockScenario,
    },
  })
}

export function buildCaptureCommandRequest(input: z.input<typeof captureCommandInputSchema>) {
  const parsed = captureCommandInputSchema.parse(input)

  return captureCommandRequestSchema.parse({
    schemaVersion: schemaVersions.protocol,
    requestId: parsed.requestId,
    correlationId: parsed.correlationId,
    method: 'camera.capture',
    sessionId: parsed.sessionId,
    payload: {
      activePreset: parsed.activePreset,
    },
  })
}

export function generateSidecarProtocolJsonSchema() {
  return z.toJSONSchema(sidecarMessageSchema)
}

export type CameraCommandResult = z.infer<typeof cameraCommandResultSchema>
export type CameraCommandRequest = z.infer<typeof cameraCommandRequestSchema>
export type CaptureCommandResult = z.infer<typeof captureCommandResultSchema>
