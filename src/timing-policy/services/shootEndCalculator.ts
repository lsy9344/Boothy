import {
  sessionTimingInitializationSchema,
  sessionTimingStateSchema,
  type SessionTimingInitialization,
  type SessionTimingState,
  type SessionType,
} from '../../shared-contracts/dto/sessionTiming.js'

const sessionDurationMinutes: Record<SessionType, number> = {
  standard: 50,
  couponExtended: 100,
}

export function shiftIsoUtcMinutes(value: string, minutes: number): string {
  const timestamp = new Date(value)

  if (Number.isNaN(timestamp.getTime())) {
    throw new TypeError(`Invalid ISO timestamp: ${value}`)
  }

  return new Date(timestamp.getTime() + minutes * 60_000).toISOString()
}

export function calculateAuthoritativeShootEndAt(reservationStartAt: string, sessionType: SessionType): string {
  return shiftIsoUtcMinutes(reservationStartAt, sessionDurationMinutes[sessionType])
}

export function createSessionTimingState(input: SessionTimingInitialization): SessionTimingState {
  const parsedInput = sessionTimingInitializationSchema.parse(input)

  return sessionTimingStateSchema.parse({
    reservationStartAt: parsedInput.reservationStartAt,
    actualShootEndAt: calculateAuthoritativeShootEndAt(parsedInput.reservationStartAt, parsedInput.sessionType),
    sessionType: parsedInput.sessionType,
    operatorExtensionCount: 0,
    lastTimingUpdateAt: parsedInput.updatedAt,
  })
}
