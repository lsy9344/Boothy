import { invoke } from '@tauri-apps/api/core'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { createDisplayHostService } from './viewer-host-adapter'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

describe('viewer display host adapter', () => {
  afterEach(() => {
    Reflect.deleteProperty(window, '__TAURI_INTERNALS__')
    vi.clearAllMocks()
  })

  it('invokes the payload-free clock probe contract', async () => {
    Reflect.set(window, '__TAURI_INTERNALS__', {})
    vi.mocked(invoke).mockResolvedValueOnce({
      hostMonotonicMicros: 2_000,
      hostEpochMicros: 3_000,
    })

    const service = createDisplayHostService()
    await service.stampClockProbe({ clientSentMicros: 1_000 })

    expect(invoke).toHaveBeenCalledWith('stamp_clock_probe')
  })
})
