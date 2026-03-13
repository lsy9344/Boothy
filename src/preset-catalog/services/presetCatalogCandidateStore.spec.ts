import { describe, expect, it, vi } from 'vitest'

import { loadApprovedPresetCatalogCandidate } from './presetCatalogCandidateStore.js'

describe('presetCatalogCandidateStore', () => {
  it('loads the raw branch-local candidate catalog from the dedicated store seam', async () => {
    const store = {
      get: vi.fn(async () => [{ id: 'warm-tone', name: '웜톤' }]),
    }

    await expect(loadApprovedPresetCatalogCandidate({ store })).resolves.toEqual([
      { id: 'warm-tone', name: '웜톤' },
    ])
    expect(store.get).toHaveBeenCalledWith('approvedPresetCatalogCandidate')
  })

  it('returns undefined when the dedicated store seam cannot be opened', async () => {
    await expect(
      loadApprovedPresetCatalogCandidate({
        loadStoreClient: vi.fn(async () => {
          throw new Error('store unavailable')
        }),
      }),
    ).resolves.toBeUndefined()
  })
})
