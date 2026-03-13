import type { SessionType } from './sessionTiming.js'

export type SessionStartPayload = {
  sessionName: string
  reservationStartAt?: string
  sessionType?: SessionType
}

export type SessionStartCommandPayload = SessionStartPayload & {
  branchId: string
}

export type SessionPreparationState = 'preparing'

export type SessionStartResult = {
  sessionId: string
  sessionName: string
  sessionFolder: string
  manifestPath: string
  createdAt: string
  preparationState: SessionPreparationState
}

export type SessionErrorCode =
  | 'session_name.required'
  | 'session.validation_failed'
  | 'session.provisioning_failed'

export type SessionResultEnvelope =
  | {
      ok: true
      value: SessionStartResult
    }
  | {
      ok: false
      errorCode: SessionErrorCode
      message: string
    }
