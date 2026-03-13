import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { PresetCatalogSheet } from './PresetCatalogSheet.js'

describe('PresetCatalogSheet', () => {
  it('prefers the supplied catalogState even if the catalog hook also runs', () => {
    const loadApprovedPresetCatalog = vi.fn(async () => ({
      status: 'ready' as const,
      presets: [],
      source: 'approved' as const,
    }))

    render(
      <PresetCatalogSheet
        activePresetId="warm-tone"
        catalogState={{
          status: 'ready',
          presets: [
            {
              id: 'warm-tone',
              name: '웜톤',
              group: 'tone',
              previewAssetPath: '/src/customer-flow/assets/preset-previews/warm-tone.svg',
            },
          ],
          source: 'approved',
        }}
        onClose={() => undefined}
        onSelectPreset={() => undefined}
        presetCatalogService={{
          loadApprovedPresetCatalog,
        }}
      />,
    )

    expect(screen.getByRole('button', { name: /웜톤/i })).toBeInTheDocument()
  })

  it('behaves like a modal dialog and traps focus while open', async () => {
    const user = userEvent.setup()
    const handleClose = vi.fn()

    render(
      <div>
        <button type="button">배경 요소</button>
        <PresetCatalogSheet
          activePresetId="warm-tone"
          onClose={handleClose}
          onSelectPreset={() => undefined}
        />
      </div>,
    )

    const dialog = screen.getByRole('dialog', { name: '프리셋 변경' })
    expect(dialog).toHaveAttribute('aria-modal', 'true')

    await user.tab()
    expect(dialog).toContainElement(document.activeElement)

    await user.keyboard('{Escape}')
    expect(handleClose).toHaveBeenCalledTimes(1)
  })

  it('loads the in-session preset list from the catalog service seam', async () => {
    render(
      <PresetCatalogSheet
        activePresetId="warm-tone"
        onClose={() => undefined}
        onSelectPreset={() => undefined}
        presetCatalogService={{
          loadApprovedPresetCatalog: vi.fn(async () => ({
            status: 'ready' as const,
            presets: [
              {
                id: 'warm-tone',
                name: '웜톤',
                group: 'tone' as const,
                previewAssetPath: '/src/customer-flow/assets/preset-previews/warm-tone.svg',
              },
            ],
            source: 'approved' as const,
          })),
        }}
      />,
    )

    expect(await screen.findByRole('button', { name: /웜톤/i })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /배경지 - 핑크/i })).not.toBeInTheDocument()
  })

  it('disables preset selection when the capture surface is waiting on an in-flight change', () => {
    render(
      <PresetCatalogSheet
        activePresetId="warm-tone"
        catalogState={{
          status: 'ready',
          presets: [
            {
              id: 'warm-tone',
              name: '웜톤',
              group: 'tone',
              previewAssetPath: '/src/customer-flow/assets/preset-previews/warm-tone.svg',
            },
          ],
          source: 'approved',
        }}
        onClose={() => undefined}
        onSelectPreset={() => undefined}
        selectionDisabled
      />,
    )

    expect(screen.getByRole('button', { name: /웜톤/i })).toBeDisabled()
    expect(screen.getByRole('button', { name: '닫기' })).toBeDisabled()
  })
})
