import { z } from 'zod'

const nonEmptyStringSchema = z.string().trim().min(1)

export const latestSessionPhotoSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    captureId: nonEmptyStringSchema,
    sequence: z.number().int().nonnegative(),
    assetUrl: nonEmptyStringSchema,
    capturedAt: z.iso.datetime(),
  })
  .strict()

export const latestPhotoStateSchema = z.discriminatedUnion('kind', [
  z
    .object({
      kind: z.literal('empty'),
    })
    .strict(),
  z
    .object({
      kind: z.literal('updating'),
      nextCaptureId: nonEmptyStringSchema,
      preview: latestSessionPhotoSchema.nullable().optional(),
    })
    .strict(),
  z
    .object({
      kind: z.literal('ready'),
      photo: latestSessionPhotoSchema,
    })
    .strict(),
])

export const activePresetSchema = z
  .object({
    presetId: nonEmptyStringSchema,
    label: nonEmptyStringSchema,
  })
  .strict()

export const captureConfidenceSnapshotSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    revision: z.number().int().nonnegative(),
    updatedAt: z.iso.datetime(),
    shootEndsAt: z.iso.datetime(),
    activePreset: activePresetSchema,
    latestPhoto: latestPhotoStateSchema,
  })
  .strict()

export type ActivePreset = z.infer<typeof activePresetSchema>
export type CaptureConfidenceSnapshot = z.infer<typeof captureConfidenceSnapshotSchema>
export type LatestPhotoState = z.infer<typeof latestPhotoStateSchema>
export type LatestSessionPhoto = z.infer<typeof latestSessionPhotoSchema>
