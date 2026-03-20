import type { z } from 'zod'

import {
  captureReadinessInputSchema,
  captureReadinessSnapshotSchema,
  captureReadinessUpdateSchema,
  captureRequestInputSchema,
  captureRequestResultSchema,
  sessionCaptureRecordSchema,
} from '../schemas'

export type CaptureReadinessSnapshot = z.infer<
  typeof captureReadinessSnapshotSchema
>
export type CaptureReadinessInput = z.infer<typeof captureReadinessInputSchema>
export type CaptureReadinessUpdate = z.infer<typeof captureReadinessUpdateSchema>
export type CaptureRequestInput = z.infer<typeof captureRequestInputSchema>
export type CaptureRequestResult = z.infer<typeof captureRequestResultSchema>
export type SessionCaptureRecord = z.infer<typeof sessionCaptureRecordSchema>
