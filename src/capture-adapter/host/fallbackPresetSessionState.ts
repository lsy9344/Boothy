import {
  defaultPresetId,
  getPresetCatalogEntryById,
  type PresetCatalogEntry,
  type PresetId,
} from '../../shared-contracts/presets/presetCatalog.js'

const fallbackSessionPresetIds = new Map<string, PresetId>()

function resolvePresetEntry(presetId: PresetId): PresetCatalogEntry {
  return getPresetCatalogEntryById(presetId) ?? getPresetCatalogEntryById(defaultPresetId)!
}

export function readFallbackSessionPreset(sessionId: string): PresetCatalogEntry {
  return resolvePresetEntry(fallbackSessionPresetIds.get(sessionId) ?? defaultPresetId)
}

export function writeFallbackSessionPreset(sessionId: string, presetId: PresetId): PresetCatalogEntry {
  fallbackSessionPresetIds.set(sessionId, presetId)
  return resolvePresetEntry(presetId)
}
