import { describe, expect, it, vi } from 'vitest'

import { resolvePresetPreviewSrc } from './preset-preview-src'

describe('resolvePresetPreviewSrc', () => {
  it('keeps relative fixture paths unchanged outside the Tauri runtime', () => {
    expect(
      resolvePresetPreviewSrc('fixtures/soft-glow.jpg', {
        isTauriRuntime: false,
      }),
    ).toBe('fixtures/soft-glow.jpg')
  })

  it('converts absolute filesystem paths into Tauri asset URLs', () => {
    const convertFileSrcFn = vi.fn((assetPath: string) => `asset://${assetPath}`)

    const result = resolvePresetPreviewSrc('C:/boothy/published/soft-glow.jpg', {
      convertFileSrcFn,
      isTauriRuntime: true,
    })

    expect(convertFileSrcFn).toHaveBeenCalledWith('C:/boothy/published/soft-glow.jpg')
    expect(result).toBe('asset://C:/boothy/published/soft-glow.jpg')
  })
})
