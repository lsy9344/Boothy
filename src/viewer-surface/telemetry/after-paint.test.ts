import { afterEach, describe, expect, it, vi } from 'vitest'

import { AFTER_PAINT_FALLBACK_MS, stampAfterPaint } from './after-paint'

afterEach(() => {
  vi.useRealTimers()
})

describe('stampAfterPaint', () => {
  it('reports conservatively when a background viewer never receives rAF', () => {
    vi.useFakeTimers()
    const onAfterPaint = vi.fn()

    stampAfterPaint(onAfterPaint, {
      requestAnimationFrame: () => 1,
      cancelAnimationFrame: () => undefined,
      now: () => 1234,
    })

    vi.advanceTimersByTime(AFTER_PAINT_FALLBACK_MS)

    expect(onAfterPaint).toHaveBeenCalledOnce()
    expect(onAfterPaint).toHaveBeenCalledWith(1234)
  })

  it('reports only once when rAF wins before the fallback', () => {
    vi.useFakeTimers()
    const onAfterPaint = vi.fn()
    const callbacks: {
      frame: FrameRequestCallback | null
      message: (() => void) | null
    } = { frame: null, message: null }

    stampAfterPaint(onAfterPaint, {
      requestAnimationFrame: (callback) => {
        callbacks.frame = callback
        return 1
      },
      cancelAnimationFrame: () => undefined,
      createMessageChannel: () =>
        ({
          port1: {
            close: () => undefined,
            set onmessage(callback: (() => void) | null) {
              callbacks.message = callback
            },
          },
          port2: {
            close: () => undefined,
            postMessage: () => callbacks.message?.(),
          },
        }) as unknown as MessageChannel,
      now: () => 5678,
    })

    callbacks.frame?.(0)
    vi.advanceTimersByTime(AFTER_PAINT_FALLBACK_MS)

    expect(onAfterPaint).toHaveBeenCalledOnce()
    expect(onAfterPaint).toHaveBeenCalledWith(5678)
  })
})
