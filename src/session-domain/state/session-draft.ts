import type {
  ActivePresetBinding,
  CaptureReadinessSnapshot,
  PublishedPresetSummary,
  SessionManifest,
} from '../../shared-contracts'

export type SessionFlowStep = 'session-start' | 'preset-selection' | 'capture'
export type PresetCatalogState = 'idle' | 'loading' | 'ready' | 'empty' | 'error'

export type SessionDraft = {
  flowStep: SessionFlowStep
  sessionId: string | null
  boothAlias: string | null
  selectedPreset: ActivePresetBinding | null
  presetCatalog: PublishedPresetSummary[]
  presetCatalogState: PresetCatalogState
  captureReadiness: CaptureReadinessSnapshot | null
  manifest: SessionManifest | null
}

export const DEFAULT_SESSION_DRAFT: SessionDraft = {
  flowStep: 'session-start',
  sessionId: null,
  boothAlias: null,
  selectedPreset: null,
  presetCatalog: [],
  presetCatalogState: 'idle',
  captureReadiness: null,
  manifest: null,
}
