import { z } from 'zod'

import { sessionIdSchema } from './ids'

export const viewerReadinessSchemaVersion = 'viewer-readiness/v1' as const
export const viewerReadinessUpdateSchemaVersion =
  'viewer-readiness-update/v1' as const
const maxU32 = 0xffff_ffff

/**
 * 승인된 고객 모니터 profile. Story 7.1은 1080p / 1440p / 4K만 승인 대상으로 둔다.
 */
export const viewerDisplayProfileIdSchema = z.enum(['1080p', '1440p', '4k'])

export const viewerWindowStateSchema = z.enum([
  'absent',
  'creating',
  'open',
  'closed',
])

export const viewerMonitorTargetingSchema = z.enum([
  'unresolved',
  'approved-customer-monitor',
  'single-monitor-fallback',
  'unapproved-profile',
  'monitor-unavailable',
])

/**
 * viewer가 capture-ready가 아닐 때의 host-normalized 원인. 고객 화면에는 노출하지 않고
 * booth control surface의 wait/call guidance로만 투영한다.
 */
export const viewerReasonCodeSchema = z.enum([
  'viewer-ready',
  'viewer-absent',
  'viewer-window-closed',
  'listener-not-ready',
  'layout-not-ready',
  'session-unbound',
  'session-mismatch',
  'stale-epoch',
  'stale-report',
  'monitor-not-approved',
])

export const viewerDisplayProfileSchema = z.object({
  profileId: viewerDisplayProfileIdSchema,
  monitorName: z.string().trim().min(1).nullable(),
  monitorWidthPx: z.number().int().positive(),
  monitorHeightPx: z.number().int().positive(),
  monitorScaleFactor: z.number().positive(),
})

/**
 * 실제 CSS photo rectangle과 DPR에서 파생한 physical display-fit 계약.
 * `requiredSource*`는 host가 report에서 재계산한 값이며 client 계산값을 신뢰하지 않는다.
 */
export const viewerPhotoRectSchema = z.object({
  cssWidth: z.number().positive(),
  cssHeight: z.number().positive(),
  devicePixelRatio: z.number().positive(),
  requiredSourceWidthPx: z.number().int().positive(),
  requiredSourceHeightPx: z.number().int().positive(),
})

export const viewerReadinessSnapshotSchema = z
  .object({
    schemaVersion: z
      .literal(viewerReadinessSchemaVersion)
      .default(viewerReadinessSchemaVersion),
    sessionId: sessionIdSchema.nullable(),
    viewerEpoch: z.number().int().nonnegative(),
    revision: z.number().int().nonnegative(),
    windowState: viewerWindowStateSchema,
    listenerReady: z.boolean(),
    layoutReady: z.boolean(),
    monitorTargeting: viewerMonitorTargetingSchema,
    displayProfile: viewerDisplayProfileSchema.nullable(),
    photoRect: viewerPhotoRectSchema.nullable(),
    viewerReady: z.boolean(),
    reasonCode: viewerReasonCodeSchema,
    observedAtMs: z.number().int().nonnegative(),
    lastReportAtMs: z.number().int().nonnegative().nullable(),
  })
  .superRefine((snapshot, context) => {
    const structurallyReady =
      snapshot.windowState === 'open' &&
      snapshot.monitorTargeting === 'approved-customer-monitor' &&
      snapshot.sessionId !== null &&
      snapshot.listenerReady &&
      snapshot.layoutReady &&
      snapshot.displayProfile !== null &&
      snapshot.photoRect !== null &&
      snapshot.lastReportAtMs !== null

    if (
      (snapshot.viewerReady &&
        (!structurallyReady || snapshot.reasonCode !== 'viewer-ready')) ||
      (snapshot.reasonCode === 'viewer-ready' && !snapshot.viewerReady)
    ) {
      context.addIssue({
        code: 'custom',
        message: 'viewerReady requires a complete ready snapshot.',
      })
    }
  })

/** viewer -> host. 첫 layout 확정과 rect/DPR 변화 때마다 보고한다. */
export const viewerLayoutReportSchema = z
  .object({
    viewerEpoch: z.number().int().nonnegative(),
    sessionId: sessionIdSchema.nullable(),
    cssWidth: z.number().nonnegative(),
    cssHeight: z.number().nonnegative(),
    devicePixelRatio: z.number().positive(),
    layoutReady: z.boolean(),
  })
  .superRefine((report, context) => {
    if (report.layoutReady && (report.cssWidth === 0 || report.cssHeight === 0)) {
      context.addIssue({
        code: 'custom',
        message: 'A ready viewer layout requires a measurable photo rectangle.',
      })
    }

    if (
      report.cssWidth * report.devicePixelRatio > maxU32 ||
      report.cssHeight * report.devicePixelRatio > maxU32
    ) {
      context.addIssue({
        code: 'custom',
        message: 'Required viewer source dimensions exceed the host contract.',
      })
    }
  })

/** viewer -> host. event listener 구독이 실제로 성립한 시점에 보고한다. */
export const viewerListenerReportSchema = z.object({
  viewerEpoch: z.number().int().nonnegative(),
})

export const viewerReadinessUpdateSchema = z.object({
  schemaVersion: z
    .literal(viewerReadinessUpdateSchemaVersion)
    .default(viewerReadinessUpdateSchemaVersion),
  readiness: viewerReadinessSnapshotSchema,
})

/** session binding event는 동일한 durable snapshot envelope을 전달한다. */
export const viewerSessionBindingUpdateSchema = viewerReadinessUpdateSchema
