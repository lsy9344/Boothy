import { useContext } from 'react'

import { SessionStateContext } from './session-context'

export function useSessionState() {
  const context = useContext(SessionStateContext)

  if (context === null) {
    throw new Error('SessionProvider is required for booth session state.')
  }

  return context
}
