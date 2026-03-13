import { useState } from 'react'

import type { PresetCatalogItem } from '../../shared-contracts/dto/presetCatalog.js'
import type { PresetId } from '../../shared-contracts/presets/presetCatalog.js'

type PresetOptionCardProps = {
  disabled?: boolean
  onSelectPreset(presetId: PresetId): void
  preset: PresetCatalogItem
  selected: boolean
}

const presetGroupCopy = {
  tone: '색감/톤',
  background: '배경지',
} as const

export function PresetOptionCard({
  disabled = false,
  onSelectPreset,
  preset,
  selected,
}: PresetOptionCardProps) {
  const previewSource = preset.previewAssetUrl
  const [showPreviewTile, setShowPreviewTile] = useState(!previewSource)

  return (
    <button
      aria-pressed={selected}
      className="preset-option-card"
      disabled={disabled}
      onClick={() => {
        onSelectPreset(preset.id)
      }}
      type="button"
    >
      {showPreviewTile ? (
        <span aria-hidden="true" className="preset-option-card__preview preset-option-card__preview--tile">
          {preset.group ? presetGroupCopy[preset.group] : preset.name}
        </span>
      ) : (
        <img
          alt={`${preset.name} 미리보기`}
          className="preset-option-card__preview"
          onError={() => {
            setShowPreviewTile(true)
          }}
          src={previewSource}
        />
      )}
      <span className="preset-option-card__content">
        {preset.group ? <span className="preset-option-card__group">{presetGroupCopy[preset.group]}</span> : null}
        <span className="preset-option-card__name">{preset.name}</span>
      </span>
    </button>
  )
}
