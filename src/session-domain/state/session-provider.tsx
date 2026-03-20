import type { ReactNode } from 'react'
import { useState } from 'react'

import type { SessionStartInput } from '../../shared-contracts'
import {
  createStartSessionService,
  type StartSessionService,
} from '../services/start-session'
import { SessionStateContext } from './session-context'
import { DEFAULT_SESSION_DRAFT, type SessionDraft } from './session-draft'

type SessionProviderProps = {
  children: ReactNode
  sessionService?: StartSessionService
}

export function SessionProvider({
  children,
  sessionService = createStartSessionService(),
}: SessionProviderProps) {
  const [sessionDraft, setSessionDraft] = useState<SessionDraft>(DEFAULT_SESSION_DRAFT)
  const [isStarting, setIsStarting] = useState(false)

  async function startSession(input: SessionStartInput) {
    setIsStarting(true)

    try {
      const result = await sessionService.startSession(input)

      setSessionDraft({
        flowStep: 'preset-selection',
        sessionId: result.sessionId,
        boothAlias: result.boothAlias,
        selectedPresetId: result.manifest.activePresetId,
        manifest: result.manifest,
      })

      return result
    } finally {
      setIsStarting(false)
    }
  }

  return (
    <SessionStateContext.Provider
      value={{
        isStarting,
        sessionDraft,
        startSession,
      }}
    >
      {children}
    </SessionStateContext.Provider>
  )
}
