export type SessionTimingAlertKind = 'none' | 'warning' | 'ended'

export type SessionTimingAlertState = {
  kind: SessionTimingAlertKind
  effectiveTimingRevision: string | null
  actualShootEndAt: string | null
  warningAt: string | null
}

export const initialSessionTimingAlertState: SessionTimingAlertState = {
  kind: 'none',
  effectiveTimingRevision: null,
  actualShootEndAt: null,
  warningAt: null,
}
