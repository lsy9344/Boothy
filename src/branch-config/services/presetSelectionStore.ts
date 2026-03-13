import { load } from '@tauri-apps/plugin-store'

export type PresetSelectionStoreLike = {
  get<T>(key: string): Promise<T | undefined>
  set(key: string, value: unknown): Promise<void>
  save(): Promise<void>
}

export type PresetSelectionStoreLoader = (
  path: string,
  options: {
    defaults: {
      lastUsedPresetId: string | null
    }
    autoSave: number
  },
) => Promise<PresetSelectionStoreLike>

export type PresetSelectionSettingsService = {
  loadLastUsedPresetId(): Promise<string | null>
  saveLastUsedPresetId(presetId: string): Promise<void>
}

const PRESET_SELECTION_STORE_PATH = 'preset-selection.json'
const LAST_USED_PRESET_ID_KEY = 'lastUsedPresetId'
const presetSelectionStoreDefaults = {
  lastUsedPresetId: null,
} as const

type PresetSelectionStoreOptions = {
  loadStoreClient?: PresetSelectionStoreLoader
  store?: PresetSelectionStoreLike
}

async function openPresetSelectionStore(
  loadStoreClient: PresetSelectionStoreLoader = load,
): Promise<PresetSelectionStoreLike> {
  return loadStoreClient(PRESET_SELECTION_STORE_PATH, {
    defaults: presetSelectionStoreDefaults,
    autoSave: 200,
  })
}

function normalizePresetId(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null
  }

  const normalizedValue = value.trim()
  return normalizedValue.length > 0 ? normalizedValue : null
}

export function createPresetSelectionSettingsService(
  options: PresetSelectionStoreOptions = {},
): PresetSelectionSettingsService {
  return {
    async loadLastUsedPresetId() {
      try {
        const store = options.store ?? (await openPresetSelectionStore(options.loadStoreClient))
        const rawPresetId = await store.get<unknown>(LAST_USED_PRESET_ID_KEY)

        return normalizePresetId(rawPresetId)
      } catch {
        return null
      }
    },
    async saveLastUsedPresetId(presetId) {
      const store = options.store ?? (await openPresetSelectionStore(options.loadStoreClient))
      const normalizedPresetId = normalizePresetId(presetId)

      if (!normalizedPresetId) {
        return
      }

      await store.set(LAST_USED_PRESET_ID_KEY, normalizedPresetId)
      await store.save()
    },
  }
}

export const presetSelectionSettingsService = createPresetSelectionSettingsService()
