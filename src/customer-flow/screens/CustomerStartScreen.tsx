import { useEffect, useEffectEvent, useRef, useState, type FormEvent } from 'react'

import { useBranchConfig } from '../../branch-config/useBranchConfig.js'
import { recordLifecycleEvent } from '../../diagnostics-log/services/operationalLogClient.js'
import { resolveOperationalBranchId } from '../../diagnostics-log/services/operationalLogContext.js'
import { HardFramePanel } from '../../shared-ui/components/HardFramePanel.js'
import { PrimaryActionButton } from '../../shared-ui/components/PrimaryActionButton.js'
import { customerStartCopy } from '../copy/customerStartCopy.js'

type CustomerStartScreenProps = {
  formErrorMessage?: string
  isStarting?: boolean
  onStart?: (sessionName: string) => void | Promise<void>
  onSessionNameChange?: (sessionName: string) => void
  sessionName?: string
  validationMessage?: string | null
}

export function CustomerStartScreen({
  formErrorMessage,
  isStarting = false,
  onStart,
  onSessionNameChange,
  sessionName: controlledSessionName,
  validationMessage: externalValidationMessage,
}: CustomerStartScreenProps) {
  const { config, status } = useBranchConfig()
  const hasLoggedFirstScreen = useRef(false)
  const [internalSessionName, setInternalSessionName] = useState('')
  const [localValidationMessage, setLocalValidationMessage] = useState<string | null>(null)
  const sessionName = controlledSessionName ?? internalSessionName
  const validationMessage = externalValidationMessage ?? localValidationMessage

  const logFirstScreenDisplayed = useEffectEvent(() => {
    void recordLifecycleEvent({
      payloadVersion: 1,
      eventType: 'first_screen_displayed',
      occurredAt: new Date().toISOString(),
      branchId: resolveOperationalBranchId(config.branchId),
      currentStage: 'customer-start',
    }).catch(() => undefined)
  })

  useEffect(() => {
    if (status !== 'ready' || hasLoggedFirstScreen.current) {
      return
    }

    hasLoggedFirstScreen.current = true
    logFirstScreenDisplayed()
  }, [status])

  const handleSubmit = async (event?: FormEvent<HTMLFormElement>) => {
    event?.preventDefault()

    const normalizedSessionName = sessionName.trim()

    if (normalizedSessionName.length === 0) {
      setLocalValidationMessage(customerStartCopy.validationError)
      return
    }

    setLocalValidationMessage(null)
    await onStart?.(normalizedSessionName)
  }

  return (
    <main aria-busy={status === 'loading' || isStarting} className="customer-shell">
      <HardFramePanel className="customer-shell__panel customer-check-in">
        <p className="customer-shell__eyebrow">{customerStartCopy.eyebrow}</p>

        <section className="customer-shell__content customer-check-in__content">
          <h1 className="customer-shell__title">{customerStartCopy.title}</h1>
          <p className="customer-shell__supporting">{customerStartCopy.supporting}</p>
        </section>

        <form className="customer-check-in__form" onSubmit={handleSubmit}>
          <label className="customer-check-in__field">
            <span className="customer-check-in__label">{customerStartCopy.sessionNameLabel}</span>
            <input
              aria-describedby={validationMessage ? 'customer-start-session-name-error' : undefined}
              aria-invalid={validationMessage ? 'true' : 'false'}
              className="customer-check-in__input"
              name="sessionName"
              autoFocus
              onChange={(event) => {
                const nextSessionName = event.currentTarget.value

                if (controlledSessionName === undefined) {
                  setInternalSessionName(nextSessionName)
                }

                onSessionNameChange?.(nextSessionName)

                if (localValidationMessage && nextSessionName.trim().length > 0) {
                  setLocalValidationMessage(null)
                }
              }}
              type="text"
              value={sessionName}
            />
            {validationMessage ? (
              <p className="customer-check-in__error" id="customer-start-session-name-error" role="alert">
                {validationMessage}
              </p>
            ) : null}
          </label>

          {formErrorMessage ? (
            <p className="customer-check-in__error customer-check-in__error--form" role="alert">
              {formErrorMessage}
            </p>
          ) : null}

          <div className="customer-shell__actions">
            <PrimaryActionButton disabled={isStarting} label={customerStartCopy.primaryAction} type="submit" />
          </div>
        </form>
      </HardFramePanel>
    </main>
  )
}
