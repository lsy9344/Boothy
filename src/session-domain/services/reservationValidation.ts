import type { SessionErrorCode, SessionStartPayload } from '../../shared-contracts/dto/session.js'
import { sessionStartPayloadSchema } from '../../shared-contracts/schemas/sessionSchemas.js'

type FieldValidationSuccess<T> = {
  ok: true
  value: T
}

type FieldValidationFailure = {
  ok: false
  errorCode: SessionErrorCode
}

type FieldValidationResult<T> = FieldValidationSuccess<T> | FieldValidationFailure

type SessionStartValidationSuccess = {
  ok: true
  value: SessionStartPayload
}

type SessionStartValidationFailure = {
  ok: false
  fieldErrors: Partial<Record<keyof SessionStartPayload, SessionErrorCode>>
}

export type SessionStartValidationResult = SessionStartValidationSuccess | SessionStartValidationFailure

function normalizeValue(value: string): string {
  return value.trim()
}

export function validateSessionName(value: string): FieldValidationResult<string> {
  const normalizedValue = normalizeValue(value)

  if (normalizedValue.length === 0) {
    return {
      ok: false,
      errorCode: 'session_name.required',
    }
  }

  return {
    ok: true,
    value: normalizedValue,
  }
}

export function validateSessionStartInput(input: SessionStartPayload): SessionStartValidationResult {
  const sessionNameResult = validateSessionName(input.sessionName)
  const fieldErrors: Partial<Record<keyof SessionStartPayload, SessionErrorCode>> = {}

  if (!sessionNameResult.ok) {
    fieldErrors.sessionName = sessionNameResult.errorCode
  }

  if (Object.keys(fieldErrors).length > 0) {
    return {
      ok: false,
      fieldErrors,
    }
  }

  if (!sessionNameResult.ok) {
    return {
      ok: false,
      fieldErrors,
    }
  }

  const value = sessionStartPayloadSchema.parse({
    sessionName: sessionNameResult.value,
    reservationStartAt: input.reservationStartAt,
    sessionType: input.sessionType,
  })

  return {
    ok: true,
    value,
  }
}
