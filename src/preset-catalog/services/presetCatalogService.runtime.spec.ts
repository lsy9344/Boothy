import { beforeEach, describe, expect, it, vi } from 'vitest'

const loadApprovedPresetCatalogCandidate = vi.fn()

vi.mock('./presetCatalogCandidateStore.js', () => ({
  loadApprovedPresetCatalogCandidate,
}))

describe('presetCatalogService runtime wiring', () => {
  beforeEach(() => {
    loadApprovedPresetCatalogCandidate.mockReset()
    vi.resetModules()
  })

  it('uses the dedicated candidate-store loader in the default runtime service', async () => {
    loadApprovedPresetCatalogCandidate.mockResolvedValue(undefined)

    const { presetCatalogService } = await import('./presetCatalogService.js')
    const result = await presetCatalogService.loadApprovedPresetCatalog()

    expect(loadApprovedPresetCatalogCandidate).toHaveBeenCalledTimes(1)
    expect(result).toMatchObject({
      status: 'ready',
      source: 'approved-fallback',
      auditReason: 'missing_catalog_input',
    })
  })
})
