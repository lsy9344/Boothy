import type { PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import { useApprovedPresetCatalog } from '../../preset-catalog/hooks/useApprovedPresetCatalog.js'
import {
  presetCatalogService as defaultPresetCatalogService,
  type PresetCatalogLoadState,
  type PresetCatalogService,
} from '../../preset-catalog/services/presetCatalogService.js'
import type { SessionTimeDisplay } from '../../timing-policy/selectors/sessionTimeDisplay.js'
import { PresetScreen } from './PresetScreen.js'

type PresetSelectionSurfaceProps = {
  catalogState?: PresetCatalogLoadState
  isApplyingPreset: boolean
  onConfirmPreset(): void
  onSelectPreset(presetId: PresetId): void
  presetCatalogService?: PresetCatalogService
  selectionFeedback?: string | null
  selectedPresetId: PresetId | null
  sessionName: string
  sessionTimeDisplay?: SessionTimeDisplay | null
}

type LoadedPresetSelectionSurfaceProps = Omit<PresetSelectionSurfaceProps, 'catalogState'>

function LoadedPresetSelectionSurface({
  isApplyingPreset,
  onConfirmPreset,
  onSelectPreset,
  presetCatalogService,
  selectionFeedback,
  selectedPresetId,
  sessionName,
  sessionTimeDisplay,
}: LoadedPresetSelectionSurfaceProps & { presetCatalogService: PresetCatalogService }) {
  const loadedCatalogState = useApprovedPresetCatalog(presetCatalogService)

  return (
    <PresetScreen
      catalogState={loadedCatalogState}
      isApplyingPreset={isApplyingPreset}
      onConfirmPreset={onConfirmPreset}
      onSelectPreset={onSelectPreset}
      selectionFeedback={selectionFeedback}
      selectedPresetId={selectedPresetId}
      sessionName={sessionName}
      sessionTimeDisplay={sessionTimeDisplay}
    />
  )
}

export function PresetSelectionSurface({
  catalogState,
  isApplyingPreset,
  onConfirmPreset,
  onSelectPreset,
  presetCatalogService = defaultPresetCatalogService,
  selectionFeedback,
  selectedPresetId,
  sessionName,
  sessionTimeDisplay,
}: PresetSelectionSurfaceProps) {
  if (catalogState) {
    return (
      <PresetScreen
        catalogState={catalogState}
        isApplyingPreset={isApplyingPreset}
        onConfirmPreset={onConfirmPreset}
        onSelectPreset={onSelectPreset}
        selectionFeedback={selectionFeedback}
        selectedPresetId={selectedPresetId}
        sessionName={sessionName}
        sessionTimeDisplay={sessionTimeDisplay}
      />
    )
  }

  return (
    <LoadedPresetSelectionSurface
      isApplyingPreset={isApplyingPreset}
      onConfirmPreset={onConfirmPreset}
      onSelectPreset={onSelectPreset}
      presetCatalogService={presetCatalogService}
      selectionFeedback={selectionFeedback}
      selectedPresetId={selectedPresetId}
      sessionName={sessionName}
      sessionTimeDisplay={sessionTimeDisplay}
    />
  )
}
