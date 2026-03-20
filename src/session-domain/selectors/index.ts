import type { SessionDraft } from '../state/session-draft'

export function isPresetSelected(sessionDraft: SessionDraft) {
  return sessionDraft.selectedPresetId !== null
}
