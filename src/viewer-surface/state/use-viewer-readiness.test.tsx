import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type {
  ViewerLayoutReport,
  ViewerReadinessSnapshot,
} from '../../shared-contracts'
import {
  normalizeViewerHostError,
  type ViewerHostService,
} from '../services/viewer-host-adapter'
import {
  shouldApplySnapshot,
  useViewerReadiness,
  VIEWER_RETRY_INTERVAL_MS,
} from './use-viewer-readiness'

const SESSION_A = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
const SESSION_B = 'session_01hs6n1r8b8zc5v4ey2x7b9g2n'

function buildSnapshot(
  overrides: Partial<ViewerReadinessSnapshot> = {},
): ViewerReadinessSnapshot {
  return {
    schemaVersion: 'viewer-readiness/v1',
    sessionId: SESSION_A,
    viewerEpoch: 1,
    revision: 10,
    windowState: 'open',
    listenerReady: true,
    layoutReady: true,
    monitorTargeting: 'approved-customer-monitor',
    displayProfile: {
      profileId: '1080p',
      monitorName: 'BOOTH-CUSTOMER',
      monitorWidthPx: 1920,
      monitorHeightPx: 1080,
      monitorScaleFactor: 1,
    },
    photoRect: {
      cssWidth: 1620,
      cssHeight: 1080,
      devicePixelRatio: 1,
      requiredSourceWidthPx: 1620,
      requiredSourceHeightPx: 1080,
    },
    viewerReady: true,
    reasonCode: 'viewer-ready',
    observedAtMs: 1_000_000,
    lastReportAtMs: 1_000_000,
    ...overrides,
  }
}

function stubRect(element: HTMLElement, width: number, height: number) {
  element.getBoundingClientRect = () =>
    ({
      width,
      height,
      top: 0,
      left: 0,
      right: width,
      bottom: height,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }) as DOMRect
}

/** 1920x1080 stage 안의 3:2 contain rect는 1620x1080이다. */
function createRefs({ photoWidth = 1620, photoHeight = 1080 } = {}) {
  const stage = document.createElement('div')
  const photo = document.createElement('div')

  stage.append(photo)
  document.body.append(stage)
  stubRect(stage, 1920, 1080)
  stubRect(photo, photoWidth, photoHeight)

  return { stageRef: { current: stage }, photoRef: { current: photo } }
}

function createFakeHost(
  initial: ViewerReadinessSnapshot,
  {
    listenerFailures = 0,
    subscriptionFailures = 0,
  }: { listenerFailures?: number; subscriptionFailures?: number } = {},
) {
  const layoutReports: ViewerLayoutReport[] = []
  const listenerReports: number[] = []
  let emit: ((next: ViewerReadinessSnapshot) => void) | null = null
  let unlistenCount = 0
  let subscribeAttempts = 0
  let response = initial

  const service: ViewerHostService = {
    async getViewerReadiness() {
      return response
    },
    async reportListenerReady({ viewerEpoch }) {
      listenerReports.push(viewerEpoch)

      if (listenerFailures > 0) {
        listenerFailures -= 1
        throw new Error('listener report failed')
      }

      return response
    },
    async reportLayout(report) {
      layoutReports.push(report)
      return response
    },
    async subscribeToViewerReadiness({ onReadiness }) {
      subscribeAttempts += 1

      if (subscriptionFailures > 0) {
        subscriptionFailures -= 1
        throw new Error('subscription failed')
      }

      emit = onReadiness
      return () => {
        unlistenCount += 1
      }
    },
  }

  return {
    service,
    layoutReports,
    listenerReports,
    setResponse(next: ViewerReadinessSnapshot) {
      response = next
    },
    emit(next: ViewerReadinessSnapshot) {
      emit?.(next)
    },
    get unlistenCount() {
      return unlistenCount
    },
    get subscribeAttempts() {
      return subscribeAttempts
    },
  }
}

async function flushPromises() {
  await Promise.resolve()
  await Promise.resolve()
}

describe('shouldApplySnapshot', () => {
  it('더 높은 revision을 적용한다', () => {
    expect(
      shouldApplySnapshot(buildSnapshot({ revision: 10 }), buildSnapshot({ revision: 11 })),
    ).toBe(true)
  })

  it('늦게 도착한 낮은 revision을 폐기한다', () => {
    expect(
      shouldApplySnapshot(buildSnapshot({ revision: 10 }), buildSnapshot({ revision: 9 })),
    ).toBe(false)
  })

  it('같은 revision에 변화가 없으면 적용하지 않는다', () => {
    expect(shouldApplySnapshot(buildSnapshot(), buildSnapshot())).toBe(false)
  })

  it('revision이 같아도 staleness 전이는 적용한다', () => {
    expect(
      shouldApplySnapshot(
        buildSnapshot(),
        buildSnapshot({ reasonCode: 'stale-report', viewerReady: false }),
      ),
    ).toBe(true)
  })

  it('revision이 같아도 세션 교체는 적용한다', () => {
    expect(
      shouldApplySnapshot(buildSnapshot(), buildSnapshot({ sessionId: SESSION_B })),
    ).toBe(true)
  })

  it('revision이 같아도 새 epoch는 적용한다', () => {
    expect(
      shouldApplySnapshot(buildSnapshot(), buildSnapshot({ viewerEpoch: 2 })),
    ).toBe(true)
  })
})

describe('useViewerReadiness', () => {
  it('durable snapshot을 먼저 읽어 초기 상태를 세운다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()

    const { result } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => {
      expect(result.current.viewerReady).toBe(true)
    })
    expect(result.current.revision).toBe(10)
  })

  it('구독이 성립한 뒤에만 listener readiness를 알린다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()

    renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))

    await waitFor(() => {
      expect(host.listenerReports).toContain(1)
    })
  })

  it('event 구독이 일시적으로 실패하면 자동으로 재시도한다', async () => {
    vi.useFakeTimers()

    try {
      const host = createFakeHost(buildSnapshot(), { subscriptionFailures: 1 })
      const refs = createRefs()

      renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))
      await act(flushPromises)
      expect(host.subscribeAttempts).toBe(1)

      await act(async () => {
        await vi.advanceTimersByTimeAsync(VIEWER_RETRY_INTERVAL_MS)
      })
      expect(host.subscribeAttempts).toBe(2)
      expect(host.listenerReports).toContain(1)
    } finally {
      vi.useRealTimers()
    }
  })

  it('listener-ready 보고가 일시적으로 실패하면 같은 epoch에서 재시도한다', async () => {
    vi.useFakeTimers()

    try {
      const host = createFakeHost(buildSnapshot(), { listenerFailures: 1 })
      const refs = createRefs()

      renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))
      await act(flushPromises)
      expect(host.listenerReports).toEqual([1])

      await act(async () => {
        await vi.advanceTimersByTimeAsync(VIEWER_RETRY_INTERVAL_MS)
      })
      expect(host.listenerReports).toEqual([1, 1])
    } finally {
      vi.useRealTimers()
    }
  })

  it('viewer window가 재생성되면 새 epoch로 listener readiness를 다시 알린다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()

    renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))

    await waitFor(() => {
      expect(host.listenerReports).toEqual([1])
    })

    host.emit(buildSnapshot({ viewerEpoch: 2, revision: 20 }))

    await waitFor(() => {
      expect(host.listenerReports).toEqual([1, 2])
    })
  })

  it('실측 photo rect와 현재 epoch/세션을 host에 보고한다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()

    renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))

    await waitFor(() => {
      expect(host.layoutReports.length).toBeGreaterThan(0)
    })

    expect(host.layoutReports[0]).toEqual({
      viewerEpoch: 1,
      sessionId: SESSION_A,
      cssWidth: 1620,
      cssHeight: 1080,
      devicePixelRatio: 1,
      layoutReady: true,
    })
  })

  it('DPR이 바뀌면 heartbeat를 기다리지 않고 layout을 다시 보고한다', async () => {
    const originalMatchMedia = window.matchMedia
    const originalDpr = window.devicePixelRatio
    let onDprChange: ((event: MediaQueryListEvent) => void) | null = null

    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: vi.fn(() => ({
        matches: true,
        media: '',
        onchange: null,
        addEventListener: (
          _type: string,
          listener: (event: MediaQueryListEvent) => void,
        ) => {
          onDprChange = listener
        },
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    })

    try {
      const host = createFakeHost(buildSnapshot())
      const refs = createRefs()

      renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))
      await waitFor(() => expect(host.layoutReports.length).toBeGreaterThan(0))
      const reportCount = host.layoutReports.length

      Object.defineProperty(window, 'devicePixelRatio', {
        configurable: true,
        value: 2,
      })
      act(() => onDprChange?.({} as MediaQueryListEvent))

      await waitFor(() => {
        expect(host.layoutReports.length).toBeGreaterThan(reportCount)
        expect(host.layoutReports.at(-1)?.devicePixelRatio).toBe(2)
      })
    } finally {
      Object.defineProperty(window, 'matchMedia', {
        configurable: true,
        value: originalMatchMedia,
      })
      Object.defineProperty(window, 'devicePixelRatio', {
        configurable: true,
        value: originalDpr,
      })
    }
  })

  it('layout이 기대한 contain-fit과 어긋나면 layout-ready를 주장하지 않는다', async () => {
    const host = createFakeHost(buildSnapshot())
    // stage는 1920x1080인데 photo가 800x800이면 3:2 contain(1620x1080)이 깨진 것이다.
    const refs = createRefs({ photoWidth: 800, photoHeight: 800 })

    renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))

    await waitFor(() => {
      expect(host.layoutReports.length).toBeGreaterThan(0)
    })

    expect(host.layoutReports[0]?.layoutReady).toBe(false)
  })

  it('photo rect가 0px로 붕괴하면 즉시 layout 미준비를 보고한다', async () => {
    const originalResizeObserver = Object.getOwnPropertyDescriptor(
      globalThis,
      'ResizeObserver',
    )
    let resizeCallback: ResizeObserverCallback | null = null

    class TestResizeObserver {
      constructor(callback: ResizeObserverCallback) {
        resizeCallback = callback
      }

      observe() {}
      unobserve() {}
      disconnect() {}
    }

    Object.defineProperty(globalThis, 'ResizeObserver', {
      configurable: true,
      value: TestResizeObserver,
    })

    try {
      const host = createFakeHost(buildSnapshot())
      const refs = createRefs()

      renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))
      await waitFor(() => expect(host.layoutReports.length).toBeGreaterThan(0))
      const reportCount = host.layoutReports.length

      stubRect(refs.photoRef.current, 0, 0)
      act(() => {
        resizeCallback?.([], {} as ResizeObserver)
      })

      await waitFor(() => {
        expect(host.layoutReports.length).toBeGreaterThan(reportCount)
        expect(host.layoutReports.at(-1)).toMatchObject({
          cssWidth: 0,
          cssHeight: 0,
          layoutReady: false,
        })
      })
    } finally {
      if (originalResizeObserver) {
        Object.defineProperty(
          globalThis,
          'ResizeObserver',
          originalResizeObserver,
        )
      } else {
        Reflect.deleteProperty(globalThis, 'ResizeObserver')
      }
    }
  })

  it('늦게 도착한 이전 revision 이벤트가 최신 상태를 덮지 않는다', async () => {
    const host = createFakeHost(buildSnapshot({ revision: 30 }))
    const refs = createRefs()

    const { result } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => {
      expect(result.current.revision).toBe(30)
    })

    host.emit(buildSnapshot({ revision: 12, viewerReady: false, reasonCode: 'stale-report' }))

    await waitFor(() => {
      expect(result.current.revision).toBe(30)
    })
    expect(result.current.viewerReady).toBe(true)
  })

  it('같은 snapshot이 중복 도착해도 상태를 다시 적용하지 않는다', async () => {
    const host = createFakeHost(buildSnapshot({ revision: 30 }))
    const refs = createRefs()
    const { result } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => expect(result.current.revision).toBe(30))
    const appliedSnapshot = result.current

    act(() => host.emit(buildSnapshot({ revision: 30 })))
    await act(flushPromises)

    expect(result.current).toBe(appliedSnapshot)
  })

  it('host가 알린 wrong-session snapshot으로 수렴한다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()
    const { result } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => expect(result.current.viewerReady).toBe(true))
    act(() =>
      host.emit(
        buildSnapshot({
          revision: 31,
          sessionId: SESSION_B,
          viewerReady: false,
          reasonCode: 'session-mismatch',
        }),
      ),
    )

    await waitFor(() => {
      expect(result.current.sessionId).toBe(SESSION_B)
      expect(result.current.reasonCode).toBe('session-mismatch')
    })
  })

  it('stale epoch 진단을 미준비 상태로 적용한다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()
    const { result } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => expect(result.current.viewerReady).toBe(true))
    act(() =>
      host.emit(
        buildSnapshot({
          revision: 31,
          listenerReady: false,
          layoutReady: false,
          viewerReady: false,
          reasonCode: 'stale-epoch',
        }),
      ),
    )

    await waitFor(() => {
      expect(result.current.viewerReady).toBe(false)
      expect(result.current.reasonCode).toBe('stale-epoch')
    })
  })

  it('listener를 잃으면 host가 알린 미준비 상태를 그대로 반영한다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()

    const { result } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => {
      expect(result.current.viewerReady).toBe(true)
    })

    host.emit(
      buildSnapshot({
        revision: 31,
        windowState: 'closed',
        listenerReady: false,
        layoutReady: false,
        viewerReady: false,
        reasonCode: 'viewer-window-closed',
      }),
    )

    await waitFor(() => {
      expect(result.current.viewerReady).toBe(false)
    })
    expect(result.current.reasonCode).toBe('viewer-window-closed')
  })

  it('언마운트되면 구독을 해제한다', async () => {
    const host = createFakeHost(buildSnapshot())
    const refs = createRefs()

    const { unmount } = renderHook(() =>
      useViewerReadiness({ ...refs, hostService: host.service }),
    )

    await waitFor(() => {
      expect(host.listenerReports.length).toBeGreaterThan(0)
    })

    unmount()

    expect(host.unlistenCount).toBeGreaterThan(0)
  })

  it('epoch를 모르는 동안에는 layout을 보고하지 않는다', async () => {
    const host = createFakeHost(buildSnapshot({ viewerEpoch: 0, revision: 0, viewerReady: false, reasonCode: 'viewer-absent' }))
    const refs = createRefs()

    renderHook(() => useViewerReadiness({ ...refs, hostService: host.service }))

    await waitFor(() => {
      expect(host.unlistenCount).toBe(0)
    })

    expect(host.layoutReports).toEqual([])
  })
})

describe('normalizeViewerHostError', () => {
  it('계약된 host error는 그대로 보존한다', () => {
    expect(
      normalizeViewerHostError({
        code: 'validation-error',
        message: '잘못된 viewer report예요.',
      }),
    ).toEqual({
      code: 'validation-error',
      message: '잘못된 viewer report예요.',
    })
  })

  it('알 수 없는 오류는 host-unavailable로 정규화한다', () => {
    expect(normalizeViewerHostError(new Error('ipc failed'))).toEqual({
      code: 'host-unavailable',
      message: '관람 화면 준비 상태를 다시 확인하고 있어요.',
    })
  })
})
