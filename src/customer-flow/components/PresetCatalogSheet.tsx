import type { PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import { useApprovedPresetCatalog } from '../../preset-catalog/hooks/useApprovedPresetCatalog.js'
import {
  presetCatalogService as defaultPresetCatalogService,
  type PresetCatalogLoadState,
  type PresetCatalogService,
} from '../../preset-catalog/services/presetCatalogService.js'
import { useModalFocusTrap } from '../../shared-ui/hooks/useModalFocusTrap.js'
import { captureScreenCopy } from '../copy/captureScreenCopy.js'
import { presetSelectionCopy } from '../copy/presetSelectionCopy.js'
import { PresetOptionCard } from './PresetOptionCard.js'

type PresetCatalogSheetProps = {
  activePresetId: PresetId
  catalogState?: PresetCatalogLoadState
  onClose(): void
  onSelectPreset(presetId: PresetId): void
  presetCatalogService?: PresetCatalogService
  selectionDisabled?: boolean
}

type LoadedPresetCatalogSheetProps = Omit<PresetCatalogSheetProps, 'catalogState'>

function LoadedPresetCatalogSheet({
  activePresetId,
  onClose,
  onSelectPreset,
  presetCatalogService,
  selectionDisabled,
}: LoadedPresetCatalogSheetProps & { presetCatalogService: PresetCatalogService }) {
  const loadedCatalogState = useApprovedPresetCatalog(presetCatalogService)

  return (
    <PresetCatalogSheetContent
      activePresetId={activePresetId}
      catalogState={loadedCatalogState}
      onClose={onClose}
      onSelectPreset={onSelectPreset}
      selectionDisabled={selectionDisabled}
    />
  )
}

type PresetCatalogSheetContentProps = {
  activePresetId: PresetId
  catalogState: PresetCatalogLoadState
  onClose(): void
  onSelectPreset(presetId: PresetId): void
  selectionDisabled?: boolean
}

function PresetCatalogSheetContent({
  activePresetId,
  catalogState,
  onClose,
  onSelectPreset,
  selectionDisabled = false,
}: PresetCatalogSheetContentProps) {
  const dialogRef = useModalFocusTrap(selectionDisabled ? () => undefined : onClose)
  const stateCopy =
    presetSelectionCopy.states[
      catalogState.status === 'unavailable'
        ? 'unavailable'
        : catalogState.status === 'empty'
          ? 'empty'
          : 'loading'
    ]

  return (
    <div
      aria-label={captureScreenCopy.presetDialogTitle}
      aria-modal="true"
      className="preset-sheet"
      ref={dialogRef}
      role="dialog"
      tabIndex={-1}
    >
      <section className="surface-frame preset-sheet__panel">
        <div className="preset-sheet__header">
          <div>
            <p className="preset-sheet__eyebrow">{captureScreenCopy.activePresetLabel}</p>
            <h2 className="preset-sheet__title">{captureScreenCopy.presetDialogTitle}</h2>
          </div>
          <button className="preset-sheet__close" disabled={selectionDisabled} onClick={onClose} type="button">
            닫기
          </button>
        </div>

        <div className="preset-sheet__grid">
          {catalogState.status === 'ready' ? (
            catalogState.presets.map((preset) => (
              <PresetOptionCard
                disabled={selectionDisabled}
                key={preset.id}
                onSelectPreset={onSelectPreset}
                preset={preset}
                selected={preset.id === activePresetId}
              />
            ))
          ) : (
            <section aria-live="polite" className="preset-screen__state" role="status">
              <h3 className="preset-screen__state-title">{stateCopy.title}</h3>
              <p className="preset-screen__state-supporting">{stateCopy.supporting}</p>
            </section>
          )}
        </div>
      </section>
    </div>
  )
}

export function PresetCatalogSheet({
  activePresetId,
  catalogState,
  onClose,
  onSelectPreset,
  presetCatalogService = defaultPresetCatalogService,
  selectionDisabled,
}: PresetCatalogSheetProps) {
  if (catalogState) {
    return (
      <PresetCatalogSheetContent
        activePresetId={activePresetId}
        catalogState={catalogState}
        onClose={onClose}
        onSelectPreset={onSelectPreset}
        selectionDisabled={selectionDisabled}
      />
    )
  }

  return (
    <LoadedPresetCatalogSheet
      activePresetId={activePresetId}
      onClose={onClose}
      onSelectPreset={onSelectPreset}
      presetCatalogService={presetCatalogService}
      selectionDisabled={selectionDisabled}
    />
  )
}
