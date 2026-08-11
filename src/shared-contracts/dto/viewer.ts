import type { z } from 'zod'

import {
  viewerDisplayProfileSchema,
  viewerLayoutReportSchema,
  viewerListenerReportSchema,
  viewerMonitorTargetingSchema,
  viewerPhotoRectSchema,
  viewerReadinessSnapshotSchema,
  viewerReadinessUpdateSchema,
  viewerSessionBindingUpdateSchema,
  viewerReasonCodeSchema,
  viewerWindowStateSchema,
} from '../schemas'

export type ViewerDisplayProfile = z.infer<typeof viewerDisplayProfileSchema>
export type ViewerPhotoRect = z.infer<typeof viewerPhotoRectSchema>
export type ViewerWindowState = z.infer<typeof viewerWindowStateSchema>
export type ViewerMonitorTargeting = z.infer<
  typeof viewerMonitorTargetingSchema
>
export type ViewerReasonCode = z.infer<typeof viewerReasonCodeSchema>
export type ViewerReadinessSnapshot = z.infer<
  typeof viewerReadinessSnapshotSchema
>
export type ViewerReadinessUpdate = z.infer<typeof viewerReadinessUpdateSchema>
export type ViewerSessionBindingUpdate = z.infer<
  typeof viewerSessionBindingUpdateSchema
>
export type ViewerLayoutReport = z.infer<typeof viewerLayoutReportSchema>
export type ViewerListenerReport = z.infer<typeof viewerListenerReportSchema>
