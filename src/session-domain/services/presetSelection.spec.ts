import { readFile } from 'node:fs/promises'

import { describe, expect, it, vi } from 'vitest'

import { createPresetSelectionService } from './presetSelection.js'

describe('preset selection service', () => {
  it('routes the session-scoped preset update through the typed host adapter', async () => {
    const invokeClient = vi.fn(async () => ({
      ok: true,
      value: {
        manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라1234/session.json',
        updatedAt: '2026-03-08T00:00:06.000Z',
        activePreset: {
          presetId: 'background-pink',
          displayName: '배경지 - 핑크',
        },
      },
    }))

    const service = createPresetSelectionService(invokeClient)

    await expect(
      service.selectPreset({
        sessionId: '2026-03-08:김보라1234',
        presetId: 'background-pink',
      }),
    ).resolves.toEqual({
      ok: true,
      value: {
        manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라1234/session.json',
        updatedAt: '2026-03-08T00:00:06.000Z',
        activePreset: {
          presetId: 'background-pink',
          displayName: '배경지 - 핑크',
        },
      },
    })

    expect(invokeClient).toHaveBeenCalledWith('select_session_preset', {
      payload: {
        sessionId: '2026-03-08:김보라1234',
        presetId: 'background-pink',
      },
    })
  })

  it('keeps the session-domain helper independent from the customer-flow catalog service', async () => {
    const serviceSource = await readFile('src/session-domain/services/presetSelection.ts', 'utf8')

    expect(serviceSource).not.toContain("from '../../preset-catalog/services/presetCatalogService.js'")
  })

  it('accepts typed host failure codes for session-integrity preset-selection errors', async () => {
    const invokeClient = vi.fn(async () => ({
      ok: false,
      errorCode: 'session.preset_selection_session_not_found',
      message: 'Session manifest is unavailable.',
    }))

    const service = createPresetSelectionService(invokeClient)

    await expect(
      service.selectPreset({
        sessionId: '2026-03-08:김보라1234',
        presetId: 'background-pink',
      }),
    ).resolves.toEqual({
      ok: false,
      errorCode: 'session.preset_selection_session_not_found',
      message: 'Session manifest is unavailable.',
    })
  })
})
