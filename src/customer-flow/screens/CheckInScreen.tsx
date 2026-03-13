import type { SessionStartPayload } from '../../shared-contracts/dto/session.js'
import { useSessionFlow } from '../../session-domain/state/SessionFlowProvider.js'
import { sessionStartErrorCopy } from '../copy/sessionStartErrorCopy.js'
import { CustomerStartScreen } from './CustomerStartScreen.js'

export function CheckInScreen() {
  const { state, submitCheckIn, updateField } = useSessionFlow()

  const getFieldErrorMessage = (field: keyof SessionStartPayload) => {
    const errorCode = state.fieldErrors[field]
    return errorCode ? sessionStartErrorCopy[errorCode] : undefined
  }

  const sessionNameError = getFieldErrorMessage('sessionName')
  const formError = state.formErrorCode ? sessionStartErrorCopy[state.formErrorCode] : undefined

  return (
    <CustomerStartScreen
      formErrorMessage={formError}
      isStarting={state.phase === 'provisioning'}
      onSessionNameChange={(sessionName) => {
        updateField('sessionName', sessionName)
      }}
      onStart={async () => {
        await submitCheckIn()
      }}
      sessionName={state.fields.sessionName}
      validationMessage={sessionNameError ?? null}
    />
  )
}
