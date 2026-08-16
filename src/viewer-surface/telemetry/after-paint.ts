/**
 * 프레임이 실제로 만들어진 직후를 잡는다.
 *
 * `requestAnimationFrame` 콜백은 paint **직전**에 실행되므로 그 시각은 present가 아니다.
 * 그 안에서 `MessageChannel`로 태스크를 하나 던지면, 해당 프레임이 compositor로 넘어간
 * 직후에 실행된다. 브라우저 API로 얻을 수 있는 present에 가장 가까운 시각이다.
 *
 * 한계 (문서에 남기고 증거에도 기록한다):
 * - 숨김/가려진 창에서는 `requestAnimationFrame`이 돌지 않는다 (hidden prewarm A/B의 핵심 변수).
 * - input 이벤트가 우선순위를 가로채면 태스크가 즉시 실행되지 않을 수 있다.
 * - **compositor → photon 구간은 JS로 측정할 수 없다.** 소프트웨어 present는 추정치이며,
 *   물리 프레임과의 오프셋은 HV-13B의 고속 촬영으로 따로 측정한다.
 */
export type AfterPaintDeps = {
  requestAnimationFrame?: (callback: FrameRequestCallback) => number
  cancelAnimationFrame?: (handle: number) => void
  createMessageChannel?: () => MessageChannel
  now?: () => number
}

/**
 * 비활성 보조 WebView에서 rAF가 정지하더라도 표시 보고가 유실되지 않게 하는 보수적 상한.
 * fallback 시각은 실제 교체보다 늦게 찍히므로 latency를 빠르게 보이게 만들지 않는다.
 */
export const AFTER_PAINT_FALLBACK_MS = 250

/**
 * 다음 프레임이 표시된 직후에 `onAfterPaint`를 호출한다.
 * 반환값은 취소 함수다. 언마운트/교체 시 반드시 호출한다.
 */
export function stampAfterPaint(
  onAfterPaint: (nowMs: number) => void,
  deps: AfterPaintDeps = {},
) {
  const raf =
    deps.requestAnimationFrame ??
    (typeof globalThis.requestAnimationFrame === 'function'
      ? globalThis.requestAnimationFrame.bind(globalThis)
      : null)
  const cancelRaf =
    deps.cancelAnimationFrame ??
    (typeof globalThis.cancelAnimationFrame === 'function'
      ? globalThis.cancelAnimationFrame.bind(globalThis)
      : null)
  const createChannel =
    deps.createMessageChannel ??
    (typeof globalThis.MessageChannel === 'function'
      ? () => new globalThis.MessageChannel()
      : null)
  const now = deps.now ?? (() => performance.now())

  let isCancelled = false
  let isReported = false
  let channel: MessageChannel | null = null
  let frameHandle: number | null = null

  const finish = () => {
    if (isCancelled || isReported) {
      return
    }

    isReported = true
    globalThis.clearTimeout(fallbackId)
    onAfterPaint(now())
  }

  const fallbackId = globalThis.setTimeout(finish, AFTER_PAINT_FALLBACK_MS)

  if (raf !== null && createChannel !== null) {
    frameHandle = raf(() => {
      if (isCancelled || isReported) {
        return
      }

      channel = createChannel()
      channel.port1.onmessage = () => {
        channel?.port1.close()
        channel?.port2.close()
        finish()
      }
      channel.port2.postMessage(null)
    })
  }

  return () => {
    isCancelled = true
    globalThis.clearTimeout(fallbackId)

    if (frameHandle !== null) {
      cancelRaf?.(frameHandle)
    }

    channel?.port1.close()
    channel?.port2.close()
  }
}

/**
 * Element Timing 관측. `<img elementtiming="...">`의 `renderTime`을 보조 진단으로 수집한다.
 *
 * **함정:** Windows에서 asset은 `http://asset.localhost`, 앱 문서는 `http://tauri.localhost`라
 * cross-origin이다. `Timing-Allow-Origin`이 없으면 `renderTime === 0`이 되고 `startTime`이
 * `loadTime`으로 대체된다. 그 값은 paint 시각이 아니므로 actual-present로 승격할 수 없다.
 */
export const QUALIFYING_FRAME_ELEMENT_TIMING = 'viewer-qualifying-frame'

export type ElementTimingObservation = {
  renderTimeMs: number
  isRenderTime: boolean
}

export function observeQualifyingFrameTiming(
  onObservation: (observation: ElementTimingObservation) => void,
  identifier: string = QUALIFYING_FRAME_ELEMENT_TIMING,
) {
  if (typeof PerformanceObserver !== 'function') {
    return () => undefined
  }

  let observer: PerformanceObserver | null = null

  try {
    observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        const elementEntry = entry as PerformanceEntry & {
          identifier?: string
          renderTime?: number
          loadTime?: number
        }

        if (elementEntry.identifier !== identifier) {
          continue
        }

        const renderTime = elementEntry.renderTime ?? 0

        onObservation({
          // renderTime이 0이면 startTime은 loadTime으로 대체된 값이다.
          renderTimeMs: renderTime > 0 ? renderTime : elementEntry.startTime,
          isRenderTime: renderTime > 0,
        })
      }
    })
    observer.observe({ type: 'element', buffered: true })
  } catch {
    return () => undefined
  }

  return () => {
    observer?.disconnect()
  }
}
