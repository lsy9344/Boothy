import { invoke } from '@tauri-apps/api/core'

import {
  DEFAULT_CAPABILITY_SNAPSHOT,
  capabilitySnapshotSchema,
  type CapabilitySnapshot,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export interface RuntimeCapabilityGateway {
  readSnapshot(): Promise<CapabilitySnapshot>
}

export function createBrowserRuntimeCapabilityGateway(): RuntimeCapabilityGateway {
  return {
    async readSnapshot() {
      return DEFAULT_CAPABILITY_SNAPSHOT
    },
  }
}

export function createTauriRuntimeCapabilityGateway(): RuntimeCapabilityGateway {
  return {
    async readSnapshot() {
      const snapshot = await invoke<unknown>('get_capability_snapshot')

      return capabilitySnapshotSchema.parse(snapshot)
    },
  }
}

export function createDefaultRuntimeCapabilityGateway() {
  return isTauriRuntime()
    ? createTauriRuntimeCapabilityGateway()
    : createBrowserRuntimeCapabilityGateway()
}
