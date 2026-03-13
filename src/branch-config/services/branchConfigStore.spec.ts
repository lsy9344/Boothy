import { describe, expect, it, vi } from 'vitest'

import { branchConfigDefaults, loadBranchConfig } from './branchConfigStore.js'

describe('loadBranchConfig', () => {
  it('falls back to safe defaults when the store is unavailable', async () => {
    const config = await loadBranchConfig({
      loadStoreClient: vi.fn(async () => {
        throw new Error('Tauri store is unavailable')
      }),
    })

    expect(config).toEqual(branchConfigDefaults)
  })

  it('normalizes branch config to approved local settings only', async () => {
    const store = {
      get: vi.fn(async () => ({
        branchPhoneNumber: '010-1234-5678',
        operationalToggles: {
          enablePhoneEscalation: true,
        },
        unsupportedSetting: 'drop-me',
      })),
      set: vi.fn(async () => undefined),
      save: vi.fn(async () => undefined),
    }

    const config = await loadBranchConfig({ store })

    expect(config).toEqual({
      branchId: 'branch-unconfigured',
      branchPhoneNumber: '010-1234-5678',
      operationalToggles: {
        enablePhoneEscalation: true,
      },
    })
    expect(store.set).toHaveBeenCalledWith('branchConfig', config)
    expect(store.save).toHaveBeenCalledTimes(1)
  })
})
