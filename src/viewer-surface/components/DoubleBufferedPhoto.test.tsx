import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { DisplayGeneration } from '../../shared-contracts'
import { DoubleBufferedPhoto } from './DoubleBufferedPhoto'

const SESSION_A = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

/** src → 실제 픽셀 크기. jsdom은 이미지를 디코드하지 않으므로 직접 채운다. */
const naturalSizeBySrc = new Map<string, { width: number; height: number }>()
/** src → decode() 결과를 테스트가 제어한다. */
const decodeControls = new Map<
  string,
  { resolve(): void; reject(): void; promise: Promise<void> }
>()

function buildGeneration(
  overrides: Partial<DisplayGeneration> = {},
): DisplayGeneration {
  return {
    generationId: 'req-1-000001',
    generationSeq: 1,
    sessionId: SESSION_A,
    requestId: 'req-1',
    captureId: 'capture-1',
    viewerEpoch: 3,
    tier: 'sample',
    assetPath: 'C:/dabi_shoot/req-1/000001-a.jpg',
    sourceWidthPx: 3840,
    sourceHeightPx: 2560,
    byteSize: 700_000,
    sourceHash: 'fnv1a64:0000000000000001',
    sampleVariant: 'a',
    committedAtHostMicros: 1_000_000,
    ...overrides,
  }
}

function controlFor(src: string) {
  const existing = decodeControls.get(src)

  if (existing) {
    return existing
  }

  let resolveFn = () => undefined as void
  let rejectFn = () => undefined as void
  const promise = new Promise<void>((resolve, reject) => {
    resolveFn = () => resolve()
    rejectFn = () => reject(new Error('decode-failed'))
  })
  // 테스트에서 명시적으로 처리하기 전에 unhandled rejection으로 잡히지 않게 한다.
  promise.catch(() => undefined)
  const control = { resolve: resolveFn, reject: rejectFn, promise }
  decodeControls.set(src, control)

  return control
}

function layers() {
  return Array.from(
    document.querySelectorAll<HTMLImageElement>('.viewer-photo__layer'),
  )
}

function visibleLayer() {
  return layers().find(
    (layer) => layer.getAttribute('data-front') === 'true',
  )
}

beforeEach(() => {
  naturalSizeBySrc.clear()
  decodeControls.clear()

  Object.defineProperty(HTMLImageElement.prototype, 'naturalWidth', {
    configurable: true,
    get(this: HTMLImageElement) {
      return naturalSizeBySrc.get(this.getAttribute('src') ?? '')?.width ?? 0
    },
  })
  Object.defineProperty(HTMLImageElement.prototype, 'naturalHeight', {
    configurable: true,
    get(this: HTMLImageElement) {
      return naturalSizeBySrc.get(this.getAttribute('src') ?? '')?.height ?? 0
    },
  })
  Object.defineProperty(HTMLImageElement.prototype, 'decode', {
    configurable: true,
    writable: true,
    value(this: HTMLImageElement) {
      return controlFor(this.getAttribute('src') ?? '').promise
    },
  })
})

afterEach(() => {
  vi.restoreAllMocks()
})

function renderPhoto(props: Partial<Parameters<typeof DoubleBufferedPhoto>[0]>) {
  return render(
    <DoubleBufferedPhoto
      generation={null}
      requiredSourceWidthPx={1620}
      requiredSourceHeightPx={1080}
      resolveSrc={(assetPath) => assetPath}
      // rAF/MessageChannel 대신 즉시 호출해 테스트를 결정적으로 만든다.
      stampAfterPaintFn={(callback) => {
        callback(1234)
        return () => undefined
      }}
      {...props}
    />,
  )
}

describe('DoubleBufferedPhoto', () => {
  it('renders two layers with identical geometry so a swap cannot move the photo', () => {
    renderPhoto({})

    const rendered = layers()
    expect(rendered).toHaveLength(2)

    for (const layer of rendered) {
      expect(layer.style.position).toBe('absolute')
      expect(layer.style.width).toBe('100%')
      expect(layer.style.height).toBe('100%')
      expect(layer.style.objectFit).toBe('contain')
      // fade/transition은 opaque swap 계약 위반이다.
      expect(layer.style.transition).toBe('')
      expect(layer.style.opacity).toBe('')
    }
  })

  it('keeps the current image visible while the next generation decodes', async () => {
    const first = buildGeneration()
    naturalSizeBySrc.set(first.assetPath, { width: 3840, height: 2560 })

    const { rerender } = renderPhoto({ generation: first })
    controlFor(first.assetPath).resolve()

    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(first.assetPath)
    })

    const second = buildGeneration({
      generationId: 'req-1-000002',
      generationSeq: 2,
      assetPath: 'C:/dabi_shoot/req-1/000002-b.jpg',
      sampleVariant: 'b',
    })
    naturalSizeBySrc.set(second.assetPath, { width: 3840, height: 2560 })

    rerender(
      <DoubleBufferedPhoto
        generation={second}
        requiredSourceWidthPx={1620}
        requiredSourceHeightPx={1080}
        resolveSrc={(assetPath) => assetPath}
        stampAfterPaintFn={(callback) => {
          callback(1234)
          return () => undefined
        }}
      />,
    )

    // decode가 아직 끝나지 않았다: 이전 이미지가 그대로 보여야 한다.
    await waitFor(() => {
      expect(
        layers().some((layer) => layer.getAttribute('src') === second.assetPath),
      ).toBe(true)
    })
    expect(visibleLayer()?.getAttribute('src')).toBe(first.assetPath)

    controlFor(second.assetPath).resolve()

    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(second.assetPath)
    })
  })

  it('clears both layers immediately when the host clears the generation', async () => {
    const generation = buildGeneration()
    naturalSizeBySrc.set(generation.assetPath, { width: 3840, height: 2560 })

    const { rerender } = renderPhoto({ generation })
    controlFor(generation.assetPath).resolve()
    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(generation.assetPath)
    })

    rerender(
      <DoubleBufferedPhoto
        generation={null}
        requiredSourceWidthPx={1620}
        requiredSourceHeightPx={1080}
        resolveSrc={(assetPath) => assetPath}
      />,
    )

    expect(layers().every((layer) => layer.style.visibility === 'hidden')).toBe(
      true,
    )
    await waitFor(() => {
      expect(layers().every((layer) => layer.getAttribute('src') === null)).toBe(
        true,
      )
    })
  })

  it('reports the swap only after decode resolves', async () => {
    const onPresented = vi.fn()
    const generation = buildGeneration()
    naturalSizeBySrc.set(generation.assetPath, { width: 3840, height: 2560 })

    renderPhoto({ generation, onPresented })

    expect(onPresented).not.toHaveBeenCalled()

    controlFor(generation.assetPath).resolve()

    await waitFor(() => {
      expect(onPresented).toHaveBeenCalledTimes(1)
    })

    const report = onPresented.mock.calls[0][0]
    expect(report.generationId).toBe(generation.generationId)
    expect(report.outcome).toBe('presented')
    expect(report.spans.decodeEndAtMicros).not.toBeNull()
    expect(report.spans.actualPresentAtMicros).not.toBeNull()
  })

  it('keeps the after-paint report alive across the swap rerender', async () => {
    const onPresented = vi.fn()
    const generation = buildGeneration()
    let deliverAfterPaint: (() => void) | null = null
    let isCancelled = false
    naturalSizeBySrc.set(generation.assetPath, { width: 3840, height: 2560 })

    renderPhoto({
      generation,
      onPresented,
      stampAfterPaintFn: (callback) => {
        deliverAfterPaint = () => {
          if (!isCancelled) {
            callback(1234)
          }
        }

        return () => {
          isCancelled = true
        }
      },
    })
    controlFor(generation.assetPath).resolve()

    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(generation.assetPath)
      expect(deliverAfterPaint).not.toBeNull()
    })

    deliverAfterPaint?.()

    expect(onPresented).toHaveBeenCalledTimes(1)
  })

  it('still reports a committed swap when the effect is torn down before the frame is stamped', async () => {
    // HV-13B 2026-08-12 회차에서 10건 중 1건의 terminal 행이 이렇게 사라졌다.
    // 교체가 커밋된 뒤 photo rect 재보고 같은 상위 변화가 effect를 정리하면
    // after-paint 스탬프가 취소되어 표본이 통째로 증발했다.
    const onPresented = vi.fn()
    const generation = buildGeneration()
    let deliverAfterPaint: (() => void) | null = null
    let isCancelled = false
    naturalSizeBySrc.set(generation.assetPath, { width: 3840, height: 2560 })

    const stampAfterPaintFn = (callback: (nowMs: number) => void) => {
      deliverAfterPaint = () => {
        if (!isCancelled) {
          callback(1234)
        }
      }

      return () => {
        isCancelled = true
      }
    }

    const { rerender } = renderPhoto({
      generation,
      onPresented,
      stampAfterPaintFn,
    })
    controlFor(generation.assetPath).resolve()

    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(generation.assetPath)
      expect(deliverAfterPaint).not.toBeNull()
    })

    // 스탬프가 도착하기 전에 photo rect가 갱신되어 effect가 다시 돈다.
    rerender(
      <DoubleBufferedPhoto
        generation={generation}
        requiredSourceWidthPx={1621}
        requiredSourceHeightPx={1081}
        resolveSrc={(assetPath) => assetPath}
        onPresented={onPresented}
        stampAfterPaintFn={stampAfterPaintFn}
      />,
    )

    deliverAfterPaint?.()

    expect(isCancelled).toBe(false)
    expect(onPresented).toHaveBeenCalledTimes(1)
    expect(onPresented.mock.calls[0][0].generationId).toBe(
      generation.generationId,
    )
    expect(onPresented.mock.calls[0][0].spans.actualPresentAtMicros).not.toBeNull()
  })

  it('still reports a committed swap when the next generation arrives before the frame is stamped', async () => {
    const onPresented = vi.fn()
    const first = buildGeneration()
    let deliverFirstAfterPaint: (() => void) | null = null
    let cancelCount = 0
    naturalSizeBySrc.set(first.assetPath, { width: 3840, height: 2560 })

    const stampAfterPaintFn = (callback: (nowMs: number) => void) => {
      if (deliverFirstAfterPaint === null) {
        deliverFirstAfterPaint = () => callback(1234)
      }

      return () => {
        cancelCount += 1
      }
    }

    const { rerender } = renderPhoto({
      generation: first,
      onPresented,
      stampAfterPaintFn,
    })
    controlFor(first.assetPath).resolve()

    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(first.assetPath)
      expect(deliverFirstAfterPaint).not.toBeNull()
    })

    const second = buildGeneration({
      generationId: 'req-1-000002',
      generationSeq: 2,
      assetPath: 'C:/dabi_shoot/req-1/000002-b.jpg',
      sampleVariant: 'b',
    })
    naturalSizeBySrc.set(second.assetPath, { width: 3840, height: 2560 })

    rerender(
      <DoubleBufferedPhoto
        generation={second}
        requiredSourceWidthPx={1620}
        requiredSourceHeightPx={1080}
        resolveSrc={(assetPath) => assetPath}
        onPresented={onPresented}
        stampAfterPaintFn={stampAfterPaintFn}
      />,
    )

    // 다음 generation이 도착해도 직전 표본의 terminal 보고는 살아 있어야 한다.
    expect(cancelCount).toBe(0)
    deliverFirstAfterPaint?.()

    expect(onPresented).toHaveBeenCalledTimes(1)
    expect(onPresented.mock.calls[0][0].generationId).toBe(first.generationId)
  })

  it('keeps the current image when the next generation fails to decode', async () => {
    const onRejected = vi.fn()
    const first = buildGeneration()
    naturalSizeBySrc.set(first.assetPath, { width: 3840, height: 2560 })

    const { rerender } = renderPhoto({ generation: first, onRejected })
    controlFor(first.assetPath).resolve()
    await waitFor(() => {
      expect(visibleLayer()?.getAttribute('src')).toBe(first.assetPath)
    })

    const broken = buildGeneration({
      generationId: 'req-1-000002',
      generationSeq: 2,
      assetPath: 'C:/dabi_shoot/req-1/000002-b.jpg',
    })
    naturalSizeBySrc.set(broken.assetPath, { width: 3840, height: 2560 })

    rerender(
      <DoubleBufferedPhoto
        generation={broken}
        requiredSourceWidthPx={1620}
        requiredSourceHeightPx={1080}
        resolveSrc={(assetPath) => assetPath}
        onRejected={onRejected}
        stampAfterPaintFn={(callback) => {
          callback(1234)
          return () => undefined
        }}
      />,
    )

    await waitFor(() => {
      expect(
        layers().some((layer) => layer.getAttribute('src') === broken.assetPath),
      ).toBe(true)
    })
    controlFor(broken.assetPath).reject()

    await waitFor(() => {
      expect(onRejected).toHaveBeenCalledTimes(1)
    })
    expect(onRejected.mock.calls[0][0].outcome).toBe('decode-failed')
    // 현재 이미지는 유지된다. 빈 화면으로 떨어지지 않는다.
    expect(visibleLayer()?.getAttribute('src')).toBe(first.assetPath)
  })

  it('refuses to swap to a source that cannot fill the photo rectangle', async () => {
    const onRejected = vi.fn()
    const upscaled = buildGeneration()
    // host가 통과시켰더라도 실제 픽셀이 부족하면 viewer가 다시 막는다.
    naturalSizeBySrc.set(upscaled.assetPath, { width: 384, height: 256 })

    renderPhoto({ generation: upscaled, onRejected })
    controlFor(upscaled.assetPath).resolve()

    await waitFor(() => {
      expect(onRejected).toHaveBeenCalledTimes(1)
    })
    expect(onRejected.mock.calls[0][0].rejectReason).toBe(
      'insufficient-dimensions',
    )
    expect(visibleLayer()?.getAttribute('src') ?? '').toBe('')
  })

  it('exposes no interactive controls on the customer surface', () => {
    const generation = buildGeneration()
    naturalSizeBySrc.set(generation.assetPath, { width: 3840, height: 2560 })

    renderPhoto({ generation })

    expect(screen.queryAllByRole('button')).toHaveLength(0)
    expect(screen.queryAllByRole('link')).toHaveLength(0)
    expect(screen.queryAllByRole('textbox')).toHaveLength(0)
    expect(screen.queryAllByRole('menuitem')).toHaveLength(0)
  })
})
