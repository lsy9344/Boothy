import { createContext } from 'react'

import type { SessionStartInput, SessionStartResult } from '../../shared-contracts'
import type { SessionDraft } from './session-draft'

export type SessionStateContextValue = {
  isStarting: boolean
  sessionDraft: SessionDraft
  startSession(input: SessionStartInput): Promise<SessionStartResult>
}

export const SessionStateContext =
  createContext<SessionStateContextValue | null>(null)
