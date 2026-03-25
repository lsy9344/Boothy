import type { PublishedPresetSummary } from '../../shared-contracts'
import { resolvePresetPreviewSrc } from './preset-preview-src'

type PresetCardProps = {
  preset: PublishedPresetSummary
  isSelected: boolean
  disabled: boolean
  saveLabel: string
  selectedLabel: string
  onSelect(preset: PublishedPresetSummary): void
}

export function PresetCard({
  preset,
  isSelected,
  disabled,
  saveLabel,
  selectedLabel,
  onSelect,
}: PresetCardProps) {
  const previewSrc = resolvePresetPreviewSrc(preset.preview.assetPath)

  return (
    <button
      type="button"
      className={`preset-card${isSelected ? ' preset-card--selected' : ''}`}
      onClick={() => onSelect(preset)}
      aria-pressed={isSelected}
      disabled={disabled}
    >
      <div className="preset-card__preview">
        <img src={previewSrc} alt={preset.preview.altText} />
      </div>
      <div className="preset-card__body">
        <span className="preset-card__name">{preset.displayName}</span>
        <span className="preset-card__action">
          {isSelected ? selectedLabel : saveLabel}
        </span>
      </div>
    </button>
  )
}
