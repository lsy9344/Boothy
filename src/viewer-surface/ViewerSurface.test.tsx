import { act, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type {
  DisplayGeneration,
  DisplayPointerSnapshot,
  ViewerReadinessSnapshot,
} from '../shared-contracts'
import { viewerDisplaySchemaVersion } from '../shared-contracts'
import { ViewerSurface } from './ViewerSurface'
import type {
  DisplayHostService,
  ViewerHostService,
} from './services/viewer-host-adapter'

const READY_SNAPSHOT: ViewerReadinessSnapshot = {
  schemaVersion: 'viewer-readiness/v1',
  sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
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
}

function createStubHost(
  snapshot: ViewerReadinessSnapshot = READY_SNAPSHOT,
): ViewerHostService {
  return {
    async getViewerReadiness() {
      return snapshot
    },
    async reportListenerReady() {
      return snapshot
    },
    async reportLayout() {
      return snapshot
    },
    async subscribeToViewerReadiness() {
      return () => undefined
    },
  }
}

function buildGeneration(): DisplayGeneration {
  return {
    generationId: 'req-1-000001',
    generationSeq: 1,
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    requestId: 'req-1',
    captureId: 'capture-1',
    viewerEpoch: 1,
    tier: 'sample',
    assetPath: 'C:/dabi_shoot/req-1/000001-a.jpg',
    sourceWidthPx: 3840,
    sourceHeightPx: 2560,
    byteSize: 700_000,
    sourceHash: 'fnv1a64:0000000000000001',
    sampleVariant: 'a',
    proxyProvenance: null,
    committedAtHostMicros: 1_000_000,
  }
}

function createStubDisplayHost(
  activeGeneration: DisplayGeneration | null,
  presentTelemetryEnabled = true,
): DisplayHostService {
  const pointer: DisplayPointerSnapshot = {
    schemaVersion: viewerDisplaySchemaVersion,
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    revision: activeGeneration === null ? 0 : 1,
    activeGeneration,
    requiredSourceWidthPx: 1620,
    requiredSourceHeightPx: 1080,
    // Story 7.4: 계측 IPC의 근거는 fixture 여부가 아니라 이 값이다.
    measurementLaneEnabled: presentTelemetryEnabled,
    presentTelemetryEnabled,
    observedAtHostMicros: 1_000_000,
  }

  return {
    async getViewerDisplayState() {
      return pointer
    },
    async reportDisplayPresent() {
      return undefined
    },
    async stampClockProbe() {
      return { hostMonotonicMicros: 1_000, hostEpochMicros: 2_000 }
    },
    async subscribeToDisplayUpdates() {
      return () => undefined
    },
  }
}

describe('ViewerSurface read-only 계약', () => {
  it('조작 요소를 전혀 렌더링하지 않는다', async () => {
    render(<ViewerSurface hostService={createStubHost()} />)

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    for (const role of [
      'button',
      'link',
      'textbox',
      'checkbox',
      'radio',
      'menuitem',
      'combobox',
      'slider',
      'tab',
      'switch',
    ] as const) {
      expect(screen.queryAllByRole(role)).toHaveLength(0)
    }
  })

  it('포커스 가능한 요소를 두지 않는다', async () => {
    const { container } = render(<ViewerSurface hostService={createStubHost()} />)

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    expect(
      container.querySelectorAll(
        'a[href], button, input, select, textarea, [tabindex], [contenteditable]',
      ),
    ).toHaveLength(0)
  })

  it('고객에게 진단 용어나 내부 상태를 노출하지 않는다', async () => {
    const { container } = render(
      <ViewerSurface
        hostService={createStubHost({
          ...READY_SNAPSHOT,
          viewerReady: false,
          reasonCode: 'monitor-not-approved',
          monitorTargeting: 'monitor-unavailable',
        })}
      />,
    )

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    const visibleText = container.textContent ?? ''

    for (const forbidden of [
      'epoch',
      'revision',
      'monitor',
      'listener',
      'layout',
      'session',
      'DPR',
      'snapshot',
      'reasonCode',
      'viewer-ready',
      'stale',
    ]) {
      expect(visibleText.toLowerCase()).not.toContain(forbidden.toLowerCase())
    }
  })

  it('준비 실패를 관람 화면에서 에러로 표시하지 않고 차분한 대기 문구만 유지한다', async () => {
    render(
      <ViewerSurface
        hostService={createStubHost({
          ...READY_SNAPSHOT,
          viewerReady: false,
          reasonCode: 'viewer-window-closed',
        })}
      />,
    )

    await waitFor(() => {
      expect(
        screen.getByText('사진이 준비되면 여기에 보여드릴게요.'),
      ).toBeInTheDocument()
    })

    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('사진이 놓일 자리를 항상 유지한다', async () => {
    const { container } = render(<ViewerSurface hostService={createStubHost()} />)

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    expect(container.querySelector('.viewer-surface__photo')).not.toBeNull()
  })
})

describe('ViewerSurface 레이아웃 불변식 (Story 7.2)', () => {
  // jsdom에는 이미지 디코더가 없다. 실제 픽셀 크기와 decode 성공을 명시적으로 흉내 낸다.
  beforeEach(() => {
    Object.defineProperty(HTMLImageElement.prototype, 'decode', {
      configurable: true,
      writable: true,
      value: () => Promise.resolve(),
    })
    Object.defineProperty(HTMLImageElement.prototype, 'naturalWidth', {
      configurable: true,
      get(this: HTMLImageElement) {
        return this.getAttribute('src') === null ? 0 : 3840
      },
    })
    Object.defineProperty(HTMLImageElement.prototype, 'naturalHeight', {
      configurable: true,
      get(this: HTMLImageElement) {
        return this.getAttribute('src') === null ? 0 : 2560
      },
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    Reflect.deleteProperty(HTMLImageElement.prototype, 'decode')
    Reflect.deleteProperty(HTMLImageElement.prototype, 'naturalWidth')
    Reflect.deleteProperty(HTMLImageElement.prototype, 'naturalHeight')
  })

  async function renderSurface(generation: DisplayGeneration | null) {
    const rendered = render(
      <ViewerSurface
        hostService={createStubHost()}
        displayHostService={createStubDisplayHost(generation)}
      />,
    )

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    return rendered
  }

  it('사진 자리의 기하가 standby 상태와 사진 상태에서 동일하다', async () => {
    const standby = await renderSurface(null)
    const standbyPhoto = standby.container.querySelector<HTMLElement>(
      '.viewer-surface__photo',
    )
    const standbySignature = {
      className: standbyPhoto?.className,
      inlineStyle: standbyPhoto?.getAttribute('style'),
    }
    standby.unmount()

    const withPhoto = await renderSurface(buildGeneration())
    const photoBox = withPhoto.container.querySelector<HTMLElement>(
      '.viewer-surface__photo',
    )

    // 기하를 결정하는 것은 클래스와 인라인 스타일뿐이다. 둘이 같으면 rect가 흔들리지 않는다.
    expect({
      className: photoBox?.className,
      inlineStyle: photoBox?.getAttribute('style'),
    }).toEqual(standbySignature)
  })

  it('사진이 보여도 standby 행을 DOM에서 제거하지 않는다', async () => {
    const { container } = await renderSurface(buildGeneration())

    await waitFor(() => {
      expect(
        container.querySelector<HTMLElement>('.viewer-surface__standby')?.style
          .visibility,
      ).toBe('hidden')
    })

    const standby = container.querySelector<HTMLElement>(
      '.viewer-surface__standby',
    )

    // display:none으로 없애면 stage 높이가 바뀌어 촬영이 막히고 scale jump가 난다.
    expect(standby).not.toBeNull()
    expect(standby?.style.display).not.toBe('none')
    expect(standby?.style.visibility).toBe('hidden')
  })

  it('사진 레이어는 photo rectangle 안에만 존재한다', async () => {
    const { container } = await renderSurface(buildGeneration())

    await waitFor(() => {
      expect(container.querySelectorAll('.viewer-photo__layer').length).toBe(2)
    })

    for (const layer of container.querySelectorAll('.viewer-photo__layer')) {
      expect(layer.parentElement?.className).toBe('viewer-surface__photo')
    }
  })

  it('사진이 표시되어도 조작 요소가 0을 유지한다', async () => {
    await renderSurface(buildGeneration())

    for (const role of [
      'button',
      'link',
      'textbox',
      'checkbox',
      'radio',
      'menuitem',
      'combobox',
      'slider',
      'tab',
      'switch',
    ] as const) {
      expect(screen.queryAllByRole(role)).toHaveLength(0)
    }
  })

  it('clock 보정이 끝날 때까지 present 보고를 보류한다', async () => {
    let performanceTickMicros = 1
    vi.spyOn(performance, 'now').mockImplementation(
      () => performanceTickMicros++ / 1_000,
    )
    let releaseProbe!: (value: {
      hostMonotonicMicros: number
      hostEpochMicros: number
    }) => void
    const probe = new Promise<{
      hostMonotonicMicros: number
      hostEpochMicros: number
    }>((resolve) => {
      releaseProbe = resolve
    })
    const displayService = createStubDisplayHost(buildGeneration())
    displayService.stampClockProbe = vi.fn(() => probe)
    const reportSpy = vi.spyOn(displayService, 'reportDisplayPresent')
    const decodeSpy = vi.spyOn(HTMLImageElement.prototype, 'decode')

    render(
      <ViewerSurface
        hostService={createStubHost()}
        displayHostService={displayService}
        stampAfterPaintFn={(callback) => {
          callback(1234)
          return () => undefined
        }}
      />,
    )
    await waitFor(() => expect(decodeSpy).toHaveBeenCalled())
    await act(async () => {
      await new Promise((resolve) => globalThis.setTimeout(resolve, 20))
    })
    expect(reportSpy).not.toHaveBeenCalled()

    releaseProbe({ hostMonotonicMicros: 1_000_000, hostEpochMicros: 2_000_000 })
    await waitFor(() => expect(reportSpy).toHaveBeenCalled())

    const calibratedReport = reportSpy.mock.calls.at(-1)?.[0]
    expect(Number.isInteger(calibratedReport?.clockOffsetMicros)).toBe(true)
    expect(Number.isInteger(calibratedReport?.clockUncertaintyMicros)).toBe(true)
  })

  it('clock 보정 전 decode 실패도 즉시 host에 보고한다', async () => {
    const displayService = createStubDisplayHost(buildGeneration())
    displayService.stampClockProbe = vi.fn(
      () =>
        new Promise<{
          hostMonotonicMicros: number
          hostEpochMicros: number
        }>(() => undefined),
    )
    const reportSpy = vi.spyOn(displayService, 'reportDisplayPresent')
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockRejectedValueOnce(
      new Error('decode failed'),
    )

    render(
      <ViewerSurface
        hostService={createStubHost()}
        displayHostService={displayService}
      />,
    )

    await waitFor(() => {
      expect(reportSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          outcome: 'decode-failed',
          rejectReason: 'decode-failed',
          clockOffsetMicros: 0,
          clockUncertaintyMicros: Number.MAX_SAFE_INTEGER,
          spans: {
            viewerReceiptAtMicros: null,
            decodeStartAtMicros: null,
            decodeEndAtMicros: null,
            swapCommittedAtMicros: null,
            imgOnLoadAtMicros: null,
            actualPresentAtMicros: null,
            elementTimingRenderAtMicros: null,
            isElementRenderTime: false,
          },
        }),
      )
    })
  })

  it('measurement lane이 꺼져 있으면 present IPC를 보내지 않는다', async () => {
    const displayService = createStubDisplayHost(buildGeneration(), false)
    const reportSpy = vi.spyOn(displayService, 'reportDisplayPresent')
    const decodeSpy = vi.spyOn(HTMLImageElement.prototype, 'decode')

    render(
      <ViewerSurface
        hostService={createStubHost()}
        displayHostService={displayService}
        stampAfterPaintFn={(callback) => {
          callback(1234)
          return () => undefined
        }}
      />,
    )
    await waitFor(() => expect(decodeSpy).toHaveBeenCalled())
    await act(async () => {
      await new Promise((resolve) => globalThis.setTimeout(resolve, 30))
    })

    expect(reportSpy).not.toHaveBeenCalled()
  })
})
