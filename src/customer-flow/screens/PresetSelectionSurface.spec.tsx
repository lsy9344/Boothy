import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { PresetSelectionSurface } from './PresetSelectionSurface.js'

describe('PresetSelectionSurface', () => {
  it('prefers the supplied catalogState even if the catalog hook also runs', () => {
    const loadApprovedPresetCatalog = vi.fn(async () => ({
      status: 'ready' as const,
      presets: [],
      source: 'approved' as const,
    }))

    render(
      <PresetSelectionSurface
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
        isApplyingPreset={false}
        onConfirmPreset={() => undefined}
        onSelectPreset={() => undefined}
        presetCatalogService={{
          loadApprovedPresetCatalog,
        }}
        selectedPresetId="warm-tone"
        sessionName="김보라1234"
      />,
    )

    expect(screen.getByRole('button', { name: /웜톤/i })).toBeInTheDocument()
  })

  it('renders approved preset cards after the catalog loads', async () => {
    render(
      <PresetSelectionSurface
        isApplyingPreset={false}
        onConfirmPreset={() => undefined}
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
          })),
        }}
        selectedPresetId="warm-tone"
        sessionName="김보라1234"
      />,
    )

    expect(await screen.findByRole('button', { name: /웜톤/i })).toBeInTheDocument()
    expect(screen.getByLabelText('세션 이름')).toHaveTextContent('김보라1234')
  })

  it('switches to a customer-safe unavailable state when the catalog cannot load', async () => {
    render(
      <PresetSelectionSurface
        isApplyingPreset={false}
        onConfirmPreset={() => undefined}
        onSelectPreset={() => undefined}
        presetCatalogService={{
          loadApprovedPresetCatalog: vi.fn(async () => ({
            status: 'unavailable' as const,
            auditReason: 'missing_catalog_input' as const,
          })),
        }}
        selectedPresetId="warm-tone"
        sessionName="김보라1234"
      />,
    )

    expect(await screen.findByText('프리셋을 준비하고 있어요.')).toBeInTheDocument()
    expect(screen.getByText('잠시만 기다려 주세요. 계속 안 되면 직원에게 알려 주세요.')).toBeInTheDocument()
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })
})
