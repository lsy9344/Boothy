import type { PresetCatalogItem } from '../../shared-contracts/dto/presetCatalog.js'
import type { PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import type { PresetCatalogLoadState } from '../../preset-catalog/services/presetCatalogService.js'
import { HardFramePanel } from '../../shared-ui/components/HardFramePanel.js'
import { PrimaryActionButton } from '../../shared-ui/components/PrimaryActionButton.js'
import type { SessionTimeDisplay } from '../../timing-policy/selectors/sessionTimeDisplay.js'
import { PresetOptionCard } from '../components/PresetOptionCard.js'
import { SessionTimeBanner } from '../components/SessionTimeBanner.js'
import { presetSelectionCopy } from '../copy/presetSelectionCopy.js'

type PresetScreenProps = {
  catalogState?: PresetCatalogLoadState
  onConfirmPreset(): void
  isApplyingPreset: boolean
  onSelectPreset(presetId: PresetId): void
  presets?: PresetCatalogItem[]
  selectionFeedback?: string | null
  selectedPresetId: PresetId | null
  sessionName: string
  sessionTimeDisplay?: SessionTimeDisplay | null
}

export function PresetScreen({
  catalogState,
  onConfirmPreset,
  isApplyingPreset,
  onSelectPreset,
  presets,
  selectionFeedback,
  selectedPresetId,
  sessionName,
  sessionTimeDisplay,
}: PresetScreenProps) {
  const resolvedCatalogState =
    catalogState ??
    (presets
      ? {
          status: 'ready' as const,
          presets,
        }
      : {
          status: 'loading' as const,
        })
  const selectedPreset =
    resolvedCatalogState.status === 'ready' && selectedPresetId
      ? resolvedCatalogState.presets.find((preset) => preset.id === selectedPresetId) ?? null
      : null
  const selectionHintId = 'preset-selection-hint'
  const stateCopy =
    presetSelectionCopy.states[
      resolvedCatalogState.status === 'unavailable'
        ? 'unavailable'
        : resolvedCatalogState.status === 'empty'
          ? 'empty'
          : 'loading'
    ]
  const canConfirm = resolvedCatalogState.status === 'ready' && selectedPreset !== null

  return (
    <main
      aria-busy={isApplyingPreset || resolvedCatalogState.status === 'loading'}
      className="customer-shell"
    >
      <HardFramePanel className="customer-shell__panel preset-screen">
        <p className="customer-shell__eyebrow">{presetSelectionCopy.eyebrow}</p>

        <section className="customer-shell__content preset-screen__content">
          <h1 className="customer-shell__title">{presetSelectionCopy.title}</h1>
          <p className="customer-shell__supporting">{presetSelectionCopy.supporting}</p>
          {sessionTimeDisplay ? <SessionTimeBanner {...sessionTimeDisplay} /> : null}
        </section>

        {resolvedCatalogState.status === 'ready' ? (
          <div aria-label="프리셋 목록" className="preset-screen__grid" role="list">
            {resolvedCatalogState.presets.map((preset) => (
              <div className="preset-screen__item" key={preset.id} role="listitem">
                <PresetOptionCard
                  disabled={isApplyingPreset}
                  onSelectPreset={onSelectPreset}
                  preset={preset}
                  selected={preset.id === selectedPresetId}
                />
              </div>
            ))}
          </div>
        ) : (
          <section aria-live="polite" className="preset-screen__state" role="status">
            <h2 className="preset-screen__state-title">{stateCopy.title}</h2>
            <p className="preset-screen__state-supporting">{stateCopy.supporting}</p>
          </section>
        )}

        {resolvedCatalogState.status === 'ready' ? (
          <div className="customer-shell__actions preset-screen__actions">
            <PrimaryActionButton
              describedBy={selectionHintId}
              disabled={isApplyingPreset || !canConfirm}
              label={presetSelectionCopy.confirmAction}
              onClick={onConfirmPreset}
            />
            <p aria-live="polite" className="customer-shell__supporting" id={selectionHintId}>
              {selectionFeedback
                ? selectionFeedback
                : selectedPreset
                ? `${selectedPreset.name} 프리셋으로 진행할게요.`
                : presetSelectionCopy.selectionRequired}
            </p>
          </div>
        ) : null}

        <p aria-label={presetSelectionCopy.sessionLabel} className="preset-screen__session-name">
          {sessionName}
        </p>
      </HardFramePanel>
    </main>
  )
}
