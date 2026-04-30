import { createContext } from 'react'

import type {
  CaptureDeleteInput,
  CaptureDeleteResult,
  CaptureExportInput,
  CaptureExportResult,
  CaptureReadinessInput,
  CaptureReadinessSnapshot,
  CaptureRequestInput,
  CaptureRequestResult,
  LoadPresetCatalogInput,
  PresetCatalogResult,
  PresetSelectionInput,
  PresetSelectionResult,
  SessionStartInput,
  SessionStartResult,
} from '../../shared-contracts'
import type { SessionDraft } from './session-draft'

export type SessionStateContextValue = {
  isStarting: boolean
  isLoadingPresetCatalog: boolean
  isSelectingPreset: boolean
  isLoadingCaptureReadiness: boolean
  isDeletingCapture: boolean
  isExportingCaptures: boolean
  isRequestingCapture: boolean
  sessionDraft: SessionDraft
  startSession(input: SessionStartInput): Promise<SessionStartResult>
  beginPresetSwitch(): void
  cancelPresetSwitch(): void
  loadPresetCatalog(input: LoadPresetCatalogInput): Promise<PresetCatalogResult>
  selectActivePreset(input: PresetSelectionInput): Promise<PresetSelectionResult>
  getCaptureReadiness(
    input: CaptureReadinessInput,
  ): Promise<CaptureReadinessSnapshot>
  deleteCapture(input: CaptureDeleteInput): Promise<CaptureDeleteResult>
  exportCaptures(input: CaptureExportInput): Promise<CaptureExportResult>
  requestCapture(input: CaptureRequestInput): Promise<CaptureRequestResult>
}

export const SessionStateContext =
  createContext<SessionStateContextValue | null>(null)
