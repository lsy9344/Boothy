import { describe, expect, it } from 'vitest'

import { mvpPresetCatalog } from '../../src/customer-flow/data/mvpPresetCatalog.js'
import {
  activeSessionPresetSchema,
  presetCatalogSchema,
  sessionPresetSelectionPayloadSchema,
} from '../../src/shared-contracts/schemas/presetSchemas.js'

describe('preset catalog contract', () => {
  it('accepts the bounded source-controlled MVP catalog in registration order', () => {
    expect(presetCatalogSchema.parse(mvpPresetCatalog)).toEqual(mvpPresetCatalog)
    expect(mvpPresetCatalog.map((preset) => preset.id)).toEqual([
      'warm-tone',
      'cool-tone',
      'background-ivory',
      'background-pink',
    ])
  })

  it('rejects duplicate preset identifiers and catalogs larger than six items', () => {
    expect(() =>
      presetCatalogSchema.parse([
        ...mvpPresetCatalog,
        {
          ...mvpPresetCatalog[0],
          name: '웜톤 복제',
        },
      ]),
    ).toThrow(/unique/i)

    expect(() =>
      presetCatalogSchema.parse([
        ...mvpPresetCatalog,
        {
          id: 'extra-1',
          name: '배경지 - 민트',
          group: 'background',
          previewAssetPath: '/src/customer-flow/assets/preset-previews/background-mint.svg',
        },
        {
          id: 'extra-2',
          name: '배경지 - 라벤더',
          group: 'background',
          previewAssetPath: '/src/customer-flow/assets/preset-previews/background-lavender.svg',
        },
        {
          id: 'extra-3',
          name: '배경지 - 블루',
          group: 'background',
          previewAssetPath: '/src/customer-flow/assets/preset-previews/background-blue.svg',
        },
      ]),
    ).toThrow(/6/)
  })

  it('rejects reordered catalogs and stale preset identifiers outside the approved catalog', () => {
    expect(() =>
      presetCatalogSchema.parse([
        mvpPresetCatalog[1],
        mvpPresetCatalog[0],
        ...mvpPresetCatalog.slice(2),
      ]),
    ).toThrow(/approved|order/i)

    expect(() =>
      sessionPresetSelectionPayloadSchema.parse({
        sessionId: '2026-03-08:김보라1234',
        presetId: 'not-approved',
      }),
    ).toThrow()

    expect(() =>
      activeSessionPresetSchema.parse({
        presetId: 'missing-preset',
        displayName: '없는 프리셋',
      }),
    ).toThrow()
  })

  it('rejects partial subsets so the customer flow cannot drift from the single approved baseline', () => {
    expect(() => presetCatalogSchema.parse(mvpPresetCatalog.slice(0, 2))).toThrow(/baseline|approved|match/i)
  })
})
