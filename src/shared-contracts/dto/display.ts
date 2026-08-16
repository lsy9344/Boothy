import type { z } from 'zod'

import {
  clockProbeResultSchema,
  clockProbeSchema,
  displayGenerationSchema,
  displayPointerSnapshotSchema,
  displayPresentOutcomeSchema,
  displayPresentReportSchema,
  displayPresentSpansSchema,
  displayProxyProvenanceSchema,
  displayHostRejectReasonSchema,
  displayRejectReasonSchema,
  displayUpdateSchema,
  trustedInputReportSchema,
} from '../schemas'

export type DisplayGeneration = z.infer<typeof displayGenerationSchema>
export type DisplayProxyProvenance = z.infer<
  typeof displayProxyProvenanceSchema
>
export type DisplayPointerSnapshot = z.infer<
  typeof displayPointerSnapshotSchema
>
export type DisplayUpdate = z.infer<typeof displayUpdateSchema>
export type DisplayRejectReason = z.infer<typeof displayRejectReasonSchema>
export type DisplayHostRejectReason = z.infer<
  typeof displayHostRejectReasonSchema
>
export type DisplayPresentOutcome = z.infer<typeof displayPresentOutcomeSchema>
export type DisplayPresentSpans = z.infer<typeof displayPresentSpansSchema>
export type DisplayPresentReport = z.infer<typeof displayPresentReportSchema>
export type ClockProbe = z.infer<typeof clockProbeSchema>
export type ClockProbeResult = z.infer<typeof clockProbeResultSchema>
export type TrustedInputReport = z.infer<typeof trustedInputReportSchema>
