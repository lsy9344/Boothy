import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'

import type { PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import { approvedBoothPresetCatalog } from '../../preset-catalog/services/presetCatalogService.js'
import { presetSelectionCopy } from '../copy/presetSelectionCopy.js'
import { PresetScreen } from './PresetScreen.js'

function PresetScreenHarness({
  isApplyingPreset = false,
  onConfirmPreset = vi.fn(),
}: {
  isApplyingPreset?: boolean
  onConfirmPreset?: () => void
}) {
  const [selectedPresetId, setSelectedPresetId] = useState<PresetId | null>(null)

  return (
    <PresetScreen
      catalogState={{
        status: 'ready',
        presets: approvedBoothPresetCatalog,
        source: 'approved',
      }}
      isApplyingPreset={isApplyingPreset}
      onConfirmPreset={onConfirmPreset}
      onSelectPreset={setSelectedPresetId}
      selectedPresetId={selectedPresetId}
      sessionName="김보라1234"
    />
  )
}

describe('PresetScreen', () => {
  it('keeps the confirm action disabled until a customer chooses a preset', () => {
    render(
      <PresetScreen
        catalogState={{
          status: 'ready',
          presets: approvedBoothPresetCatalog,
          source: 'approved',
        }}
        isApplyingPreset={false}
        onConfirmPreset={() => undefined}
        onSelectPreset={() => undefined}
        selectedPresetId={null}
        sessionName="김보라1234"
      />,
    )

    expect(screen.getByRole('button', { name: '이 프리셋으로 계속' })).toBeDisabled()
    expect(screen.getByText('프리셋을 먼저 고르면 다음으로 넘어갈 수 있어요.')).toBeInTheDocument()
  })

  it('treats card clicks as candidate selection until the confirm action fires', async () => {
    const user = userEvent.setup()
    const handleConfirmPreset = vi.fn()

    render(<PresetScreenHarness onConfirmPreset={handleConfirmPreset} />)

    const pinkPresetButton = screen.getByRole('button', { name: /배경지 - 핑크/i })
    const confirmButton = screen.getByRole('button', { name: '이 프리셋으로 계속' })

    await user.click(pinkPresetButton)

    expect(pinkPresetButton).toHaveAttribute('aria-pressed', 'true')
    expect(confirmButton).toBeEnabled()
    expect(handleConfirmPreset).not.toHaveBeenCalled()

    await user.click(confirmButton)

    expect(handleConfirmPreset).toHaveBeenCalledTimes(1)
  })

  it('shows customer-safe retry guidance after a failed confirmation attempt', () => {
    render(
      <PresetScreen
        catalogState={{
          status: 'ready',
          presets: approvedBoothPresetCatalog,
          source: 'approved',
        }}
        isApplyingPreset={false}
        onConfirmPreset={() => undefined}
        onSelectPreset={() => undefined}
        selectionFeedback={presetSelectionCopy.selectionRetryRequired}
        selectedPresetId="background-pink"
        sessionName="김보라1234"
      />,
    )

    expect(screen.getByText(presetSelectionCopy.selectionRetryRequired)).toBeInTheDocument()
    expect(screen.queryByText('배경지 - 핑크 프리셋으로 진행할게요.')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: '이 프리셋으로 계속' })).toBeEnabled()
  })
})
