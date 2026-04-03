import type { z } from 'zod'

import {
  captureDeleteInputSchema,
  captureDeleteResultSchema,
  captureReadinessInputSchema,
  captureReadinessSnapshotSchema,
  liveCaptureTruthSchema,
  captureReadinessUpdateSchema,
  captureFastPreviewUpdateSchema,
  captureRequestInputSchema,
  captureRequestResultSchema,
  sessionCaptureRecordSchema,
} from '../schemas'

export type CaptureReadinessSnapshot = z.infer<
  typeof captureReadinessSnapshotSchema
>
export type LiveCaptureTruth = z.infer<typeof liveCaptureTruthSchema>
export type CaptureReadinessInput = z.infer<typeof captureReadinessInputSchema>
export type CaptureReadinessUpdate = z.infer<typeof captureReadinessUpdateSchema>
export type CaptureFastPreviewUpdate = z.infer<
  typeof captureFastPreviewUpdateSchema
>
export type CaptureDeleteInput = z.infer<typeof captureDeleteInputSchema>
export type CaptureDeleteResult = z.infer<typeof captureDeleteResultSchema>
export type CaptureRequestInput = z.infer<typeof captureRequestInputSchema>
export type CaptureRequestResult = z.infer<typeof captureRequestResultSchema>
export type SessionCaptureRecord = z.infer<typeof sessionCaptureRecordSchema>
