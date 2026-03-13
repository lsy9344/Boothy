import { load } from '@tauri-apps/plugin-store'

import { branchConfigDefaults, branchConfigSchema, type BranchConfig } from './branchConfigSchema.js'

const BRANCH_CONFIG_STORE_PATH = 'branch-config.json'
const BRANCH_CONFIG_STORE_KEY = 'branchConfig'

type BranchConfigStoreDefaults = {
  branchConfig: BranchConfig
}

export type BranchConfigStoreLike = {
  get<T>(key: string): Promise<T | undefined>
  set(key: string, value: unknown): Promise<void>
  save(): Promise<void>
}

export type BranchConfigStoreLoader = (
  path: string,
  options: {
    defaults: BranchConfigStoreDefaults
    autoSave: number
  },
) => Promise<BranchConfigStoreLike>

type LoadBranchConfigOptions = {
  loadStoreClient?: BranchConfigStoreLoader
  store?: BranchConfigStoreLike
}

const branchConfigStoreDefaults: BranchConfigStoreDefaults = {
  branchConfig: branchConfigDefaults,
}

export async function openBranchConfigStore(
  loadStoreClient: BranchConfigStoreLoader = load,
): Promise<BranchConfigStoreLike> {
  return loadStoreClient(BRANCH_CONFIG_STORE_PATH, {
    defaults: branchConfigStoreDefaults,
    autoSave: 200,
  })
}

export async function loadBranchConfig(options: LoadBranchConfigOptions = {}): Promise<BranchConfig> {
  try {
    const store = options.store ?? (await openBranchConfigStore(options.loadStoreClient))
    const rawConfig = await store.get<unknown>(BRANCH_CONFIG_STORE_KEY)
    const config = branchConfigSchema.parse(rawConfig ?? {})

    await store.set(BRANCH_CONFIG_STORE_KEY, config)
    await store.save()

    return config
  } catch {
    return branchConfigDefaults
  }
}

export { branchConfigDefaults }
export type { BranchConfig }
