import { load } from '@tauri-apps/plugin-store'

export type PresetCatalogCandidateStoreLike = {
  get<T>(key: string): Promise<T | undefined>
}

export type PresetCatalogCandidateStoreLoader = (
  path: string,
  options: {
    defaults: Record<string, never>
    autoSave: number
  },
) => Promise<PresetCatalogCandidateStoreLike>

const PRESET_CATALOG_CANDIDATE_STORE_PATH = 'preset-catalog-candidate.json'
const APPROVED_PRESET_CATALOG_CANDIDATE_KEY = 'approvedPresetCatalogCandidate'

type LoadApprovedPresetCatalogCandidateOptions = {
  loadStoreClient?: PresetCatalogCandidateStoreLoader
  store?: PresetCatalogCandidateStoreLike
}

async function openPresetCatalogCandidateStore(
  loadStoreClient: PresetCatalogCandidateStoreLoader = load,
): Promise<PresetCatalogCandidateStoreLike> {
  return loadStoreClient(PRESET_CATALOG_CANDIDATE_STORE_PATH, {
    defaults: {},
    autoSave: 0,
  })
}

export async function loadApprovedPresetCatalogCandidate(
  options: LoadApprovedPresetCatalogCandidateOptions = {},
): Promise<unknown | undefined> {
  try {
    const store = options.store ?? (await openPresetCatalogCandidateStore(options.loadStoreClient))
    return store.get<unknown>(APPROVED_PRESET_CATALOG_CANDIDATE_KEY)
  } catch {
    return undefined
  }
}
