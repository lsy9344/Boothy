import { z } from 'zod'

import { contractSchemaVersionSchema } from './schemaVersion.js'

const nonEmptyStringSchema = z.string().min(1)

export const sessionGalleryItemSchema = z
  .object({
    captureId: nonEmptyStringSchema,
    sessionId: nonEmptyStringSchema,
    capturedAt: z.iso.datetime(),
    displayOrder: z.number().int().nonnegative(),
    isLatest: z.boolean(),
    previewPath: nonEmptyStringSchema,
    thumbnailPath: nonEmptyStringSchema,
    label: nonEmptyStringSchema,
  })
  .strict()

export const sessionGallerySnapshotSchema = z
  .object({
    schemaVersion: contractSchemaVersionSchema,
    sessionId: nonEmptyStringSchema,
    sessionName: nonEmptyStringSchema,
    shootEndsAt: z.iso.datetime().nullable(),
    activePresetName: nonEmptyStringSchema.nullable(),
    latestCaptureId: nonEmptyStringSchema.nullable(),
    selectedCaptureId: nonEmptyStringSchema.nullable(),
    items: z.array(sessionGalleryItemSchema),
  })
  .strict()

export const sessionGalleryRequestSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    manifestPath: nonEmptyStringSchema,
  })
  .strict()

export const deleteSessionPhotoRequestSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    captureId: nonEmptyStringSchema,
    manifestPath: nonEmptyStringSchema,
  })
  .strict()

export const deleteSessionPhotoResponseSchema = z
  .object({
    schemaVersion: contractSchemaVersionSchema,
    deletedCaptureId: nonEmptyStringSchema,
    confirmationMessage: z.literal('사진이 삭제되었습니다.'),
    gallery: sessionGallerySnapshotSchema,
  })
  .strict()

export type SessionGalleryItem = z.infer<typeof sessionGalleryItemSchema>
export type SessionGallerySnapshot = z.infer<typeof sessionGallerySnapshotSchema>
export type SessionGalleryRequest = z.infer<typeof sessionGalleryRequestSchema>
export type DeleteSessionPhotoRequest = z.infer<typeof deleteSessionPhotoRequestSchema>
export type DeleteSessionPhotoResponse = z.infer<typeof deleteSessionPhotoResponseSchema>
