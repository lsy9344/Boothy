import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { PresetOptionCard } from './PresetOptionCard.js'

describe('PresetOptionCard', () => {
  it('uses the standard preview tile when no runtime preview URL is available', () => {
    render(
      <PresetOptionCard
        onSelectPreset={vi.fn()}
        preset={{
          id: 'warm-tone',
          name: '웜톤',
          group: 'tone',
          previewAssetPath: '/src/customer-flow/assets/preset-previews/warm-tone.svg',
        }}
        selected={false}
      />,
    )

    expect(screen.queryByRole('img', { name: '웜톤 미리보기' })).not.toBeInTheDocument()
    expect(screen.getAllByText('색감/톤')).toHaveLength(2)
    expect(screen.getByText('웜톤')).toBeInTheDocument()
  })
})
