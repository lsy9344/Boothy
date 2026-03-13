import { sessionTimingStateSchema, type SessionTimingState } from '../../shared-contracts/dto/sessionTiming.js'
import { shiftIsoUtcMinutes } from '../services/shootEndCalculator.js'

export type TimingThresholdOptions = {
  warningLeadMinutes: number
  phoneEscalationDelayMinutes: number
}

export function selectCustomerFacingSessionEndTime(state: SessionTimingState): string {
  return sessionTimingStateSchema.parse(state).actualShootEndAt
}

export function deriveTimingThresholds(state: SessionTimingState, options: TimingThresholdOptions) {
  const parsedState = sessionTimingStateSchema.parse(state)

  return {
    warningAt: shiftIsoUtcMinutes(parsedState.actualShootEndAt, options.warningLeadMinutes * -1),
    shootStopAt: parsedState.actualShootEndAt,
    phoneEscalationAt: shiftIsoUtcMinutes(parsedState.actualShootEndAt, options.phoneEscalationDelayMinutes),
  }
}

export function selectTimingAlertStatus(
  state: SessionTimingState,
  options: TimingThresholdOptions,
  now = Date.now(),
) {
  const { shootStopAt, warningAt } = deriveTimingThresholds(state, options)

  if (now >= Date.parse(shootStopAt)) {
    return 'ended'
  }

  if (now >= Date.parse(warningAt)) {
    return 'warning'
  }

  return 'none'
}
