import {
  defaultPresetId,
  getPresetCatalogEntryById,
  type PresetCatalogEntry,
  type PresetId,
} from '../../shared-contracts/presets/presetCatalog.js'

export const presetChangeConfirmationMessage = '다음 촬영부터 적용됩니다.'

export type SessionCaptureRecord = {
  captureId: string
  capturedAt: string
  photoLabel: string
  presetId: PresetId
}

export type InFlightCaptureRequest = {
  captureId: string
  presetId: PresetId
  requestedAt: string
}

export type CaptureFlowState = {
  activePresetId: PresetId
  captures: SessionCaptureRecord[]
  inFlightCaptures: Record<string, InFlightCaptureRequest>
  isPresetSelectorOpen: boolean
  pendingPresetChangeMessage: string | null
  sessionEndTimeLabel: string
}

type CaptureFlowStateOverrides = Partial<CaptureFlowState>

export function createCaptureFlowState(overrides: CaptureFlowStateOverrides = {}): CaptureFlowState {
  return {
    activePresetId: overrides.activePresetId ?? defaultPresetId,
    captures: overrides.captures ?? [],
    inFlightCaptures: overrides.inFlightCaptures ?? {},
    isPresetSelectorOpen: overrides.isPresetSelectorOpen ?? false,
    pendingPresetChangeMessage: overrides.pendingPresetChangeMessage ?? null,
    sessionEndTimeLabel: overrides.sessionEndTimeLabel ?? '계산 중',
  }
}

export function openPresetSelector(state: CaptureFlowState): CaptureFlowState {
  if (state.isPresetSelectorOpen) {
    return state
  }

  return {
    ...state,
    isPresetSelectorOpen: true,
  }
}

export function closePresetSelector(state: CaptureFlowState): CaptureFlowState {
  if (!state.isPresetSelectorOpen) {
    return state
  }

  return {
    ...state,
    isPresetSelectorOpen: false,
  }
}

export function applyPresetSelection(state: CaptureFlowState, presetId: PresetId): CaptureFlowState {
  if (state.activePresetId === presetId) {
    return state
  }

  const preset = getPresetCatalogEntryById(presetId)

  if (!preset) {
    throw new Error(`Unknown preset id: ${presetId}`)
  }

  return {
    ...state,
    activePresetId: preset.id,
    pendingPresetChangeMessage: presetChangeConfirmationMessage,
  }
}

export function dismissPresetChangeFeedback(state: CaptureFlowState): CaptureFlowState {
  if (!state.pendingPresetChangeMessage) {
    return state
  }

  return {
    ...state,
    pendingPresetChangeMessage: null,
  }
}

export function startCaptureRequest(
  state: CaptureFlowState,
  request: Omit<InFlightCaptureRequest, 'presetId'>,
): CaptureFlowState {
  return {
    ...state,
    inFlightCaptures: {
      ...state.inFlightCaptures,
      [request.captureId]: {
        ...request,
        presetId: state.activePresetId,
      },
    },
  }
}

export function completeCaptureRequest(
  state: CaptureFlowState,
  request: Omit<SessionCaptureRecord, 'presetId'>,
): CaptureFlowState {
  const matchedInFlightCapture = state.inFlightCaptures[request.captureId]
  const presetId = matchedInFlightCapture?.presetId ?? state.activePresetId
  const nextInFlightCaptures = { ...state.inFlightCaptures }
  delete nextInFlightCaptures[request.captureId]

  return {
    ...state,
    captures: [
      {
        ...request,
        presetId,
      },
      ...state.captures,
    ],
    inFlightCaptures: nextInFlightCaptures,
  }
}

export function selectActivePreset(state: CaptureFlowState): PresetCatalogEntry {
  return getPresetCatalogEntryById(state.activePresetId) ?? getPresetCatalogEntryById(defaultPresetId)!
}

export function selectLatestCapture(state: CaptureFlowState): SessionCaptureRecord | null {
  return state.captures[0] ?? null
}
