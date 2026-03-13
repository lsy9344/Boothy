import { z } from 'zod'

import { manifestSchemaVersionSchema } from './schemaVersion.js'
import { sessionTimingStateSchema } from './sessionTiming.js'

export const manifestCameraStateSchema = z
  .object({
    connectionState: z.string().min(1),
  })
  .strict()

export const exportStateSchema = z
  .object({
    status: z.enum(['notStarted', 'queued', 'processing', 'completed', 'failed']),
  })
  .strict()

export const sessionIdentitySchema = z
  .object({
    sessionId: z.string().min(1),
    sessionName: z.string().min(1),
  })
  .strict()

export const sessionCaptureRecordSchema = z
  .object({
    captureId: z.string().min(1),
    originalFileName: z.string().min(1),
    processedFileName: z.string().min(1),
    capturedAt: z.iso.datetime(),
  })
  .strict()

export const sessionActivePresetSchema = z
  .object({
    presetId: z.string().min(1),
    displayName: z.string().min(1),
  })
  .strict()

export const sessionManifestSchema = sessionIdentitySchema
  .extend({
    schemaVersion: manifestSchemaVersionSchema,
    operationalDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
    createdAt: z.iso.datetime(),
    sessionDir: z.string().min(1),
    manifestPath: z.string().min(1),
    eventsPath: z.string().min(1),
    exportStatusPath: z.string().min(1),
    processedDir: z.string().min(1),
    captureRevision: z.number().int().nonnegative().default(0),
    latestCaptureId: z.string().min(1).nullable(),
    activePresetName: z.string().min(1).nullable(),
    activePreset: sessionActivePresetSchema.nullable(),
    captures: z.array(sessionCaptureRecordSchema),
    shootEndsAt: z.iso.datetime().nullable().optional(),
    cameraState: manifestCameraStateSchema,
    timing: sessionTimingStateSchema,
    exportState: exportStateSchema,
  })
  .strict()

export type SessionIdentity = z.infer<typeof sessionIdentitySchema>
export type SessionManifest = z.infer<typeof sessionManifestSchema>
