import { useState } from 'react'
import { ZodError } from 'zod'

import { type HostErrorEnvelope, sessionStartInputSchema } from '../../shared-contracts'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { sessionStartCopy } from '../copy/sessionStartCopy'

type FieldErrors = Partial<Record<'name' | 'phoneLastFour', string>>

function mapValidationErrors(error: ZodError): FieldErrors {
  return error.issues.reduce<FieldErrors>((errors, issue) => {
    const field = issue.path[0]

    if (field === 'name' || field === 'phoneLastFour') {
      errors[field] = issue.message
    }

    return errors
  }, {})
}

function isHostErrorEnvelope(error: unknown): error is HostErrorEnvelope {
  return typeof error === 'object' && error !== null && 'code' in error && 'message' in error
}

export function SessionStartForm() {
  const { isStarting, startSession } = useSessionState()
  const [name, setName] = useState('')
  const [phoneLastFour, setPhoneLastFour] = useState('')
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [submissionError, setSubmissionError] = useState<string | null>(null)

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setSubmissionError(null)

    const parsed = sessionStartInputSchema.safeParse({
      name,
      phoneLastFour,
    })

    if (!parsed.success) {
      setFieldErrors(mapValidationErrors(parsed.error))
      return
    }

    setFieldErrors({})

    try {
      await startSession(parsed.data)
    } catch (error) {
      if (isHostErrorEnvelope(error)) {
        setFieldErrors(error.fieldErrors ?? {})
        setSubmissionError(error.message)
        return
      }

      setSubmissionError('지금은 시작할 수 없어요. 잠시 후 다시 시도해 주세요.')
    }
  }

  return (
    <form className="session-start-form" onSubmit={handleSubmit} noValidate>
      <div className="session-start-form__field">
        <label className="session-start-form__label" htmlFor="customer-name">
          {sessionStartCopy.nameLabel}
        </label>
        <input
          id="customer-name"
          className="session-start-form__input"
          name="name"
          autoComplete="name"
          value={name}
          onChange={(event) => {
            setName(event.target.value)
            setFieldErrors((current) => ({ ...current, name: undefined }))
          }}
        />
        {fieldErrors.name ? (
          <p className="session-start-form__error">{fieldErrors.name}</p>
        ) : null}
      </div>

      <div className="session-start-form__field">
        <label
          className="session-start-form__label"
          htmlFor="customer-phone-last-four"
        >
          {sessionStartCopy.phoneLastFourLabel}
        </label>
        <input
          id="customer-phone-last-four"
          className="session-start-form__input"
          name="phoneLastFour"
          inputMode="numeric"
          autoComplete="off"
          maxLength={4}
          value={phoneLastFour}
          onChange={(event) => {
            setPhoneLastFour(event.target.value.replace(/\D/g, '').slice(0, 4))
            setFieldErrors((current) => ({
              ...current,
              phoneLastFour: undefined,
            }))
          }}
        />
        {fieldErrors.phoneLastFour ? (
          <p className="session-start-form__error">{fieldErrors.phoneLastFour}</p>
        ) : null}
      </div>

      {submissionError ? (
        <p className="session-start-form__error" role="alert">
          {submissionError}
        </p>
      ) : null}

      <button className="session-start-form__submit" type="submit" disabled={isStarting}>
        {isStarting ? sessionStartCopy.pendingLabel : sessionStartCopy.submitLabel}
      </button>
    </form>
  )
}
