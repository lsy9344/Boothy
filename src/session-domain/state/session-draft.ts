import type { SessionManifest } from '../../shared-contracts'

export type SessionFlowStep = 'session-start' | 'preset-selection'

export type SessionDraft = {
  flowStep: SessionFlowStep
  sessionId: string | null
  boothAlias: string | null
  selectedPresetId: string | null
  manifest: SessionManifest | null
}

export const DEFAULT_SESSION_DRAFT: SessionDraft = {
  flowStep: 'session-start',
  sessionId: null,
  boothAlias: null,
  selectedPresetId: null,
  manifest: null,
}
