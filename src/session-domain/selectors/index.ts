import type { SessionDraft } from '../state/session-draft'

export function isPresetSelected(sessionDraft: SessionDraft) {
  return sessionDraft.selectedPreset !== null
}

export function hasActiveSession(sessionDraft: SessionDraft) {
  return sessionDraft.sessionId !== null && sessionDraft.manifest !== null
}

export * from './current-session-previews'
