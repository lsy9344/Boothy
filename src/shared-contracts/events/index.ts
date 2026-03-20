export const captureReadinessUpdateEvent = 'capture-readiness-update' as const

export type HostLifecycleEvent =
  | 'booth-runtime-started'
  | 'operator-surface-requested'
  | 'authoring-surface-requested'
