import { describe, expect, it, vi } from 'vitest'

import { presetCatalog } from '../../shared-contracts/presets/presetCatalog.js'
import { createPresetCatalogService } from './presetCatalogService.js'

describe('presetCatalogService', () => {
  it('returns the approved booth catalog in deterministic order when the source loads', async () => {
    const service = createPresetCatalogService()

    const result = await service.loadApprovedPresetCatalog()

    expect(result.status).toBe('ready')

    if (result.status !== 'ready') {
      throw new Error('Expected ready preset catalog result')
    }

    expect(result.presets).toHaveLength(presetCatalog.length)
    expect(result.presets.map((preset) => preset.id)).toEqual(presetCatalog.map((preset) => preset.id))
  })

  it('falls back to the approved baseline when the candidate catalog is empty', async () => {
    const service = createPresetCatalogService({
      loadCatalogSource: vi.fn(async () => []),
    })

    await expect(service.loadApprovedPresetCatalog()).resolves.toMatchObject({
      status: 'ready',
      source: 'approved-fallback',
      auditReason: 'missing_catalog_input',
      presets: expect.arrayContaining([
        expect.objectContaining({ id: presetCatalog[0].id }),
      ]),
    })
  })

  it('falls back to the approved baseline when an explicit candidate source is missing', async () => {
    const service = createPresetCatalogService({
      loadCatalogSource: vi.fn(async () => undefined),
    })

    await expect(service.loadApprovedPresetCatalog()).resolves.toMatchObject({
      status: 'ready',
      source: 'approved-fallback',
      auditReason: 'missing_catalog_input',
    })
  })

  it('falls back to the approved baseline when an explicit candidate source cannot be loaded', async () => {
    const service = createPresetCatalogService({
      loadCatalogSource: vi.fn(async () => {
        throw new Error('disk path leaked')
      }),
    })

    await expect(service.loadApprovedPresetCatalog()).resolves.toMatchObject({
      status: 'ready',
      source: 'approved-fallback',
      auditReason: 'missing_catalog_input',
    })
  })

  it('falls back to the approved baseline when a candidate catalog is only a subset', async () => {
    const service = createPresetCatalogService({
      loadCatalogSource: vi.fn(async () => presetCatalog.slice(0, 2)),
    })

    await expect(service.loadApprovedPresetCatalog()).resolves.toMatchObject({
      status: 'ready',
      source: 'approved-fallback',
      auditReason: 'invalid_catalog_shape',
    })
  })

  it('falls back to the approved baseline when a branch candidate reorders the catalog', async () => {
    const service = createPresetCatalogService({
      loadCatalogSource: vi.fn(async () => [
        presetCatalog[1],
        presetCatalog[0],
        ...presetCatalog.slice(2),
      ]),
    })

    await expect(service.loadApprovedPresetCatalog()).resolves.toMatchObject({
      status: 'ready',
      source: 'approved-fallback',
      auditReason: 'reordered_catalog',
    })
  })
})
