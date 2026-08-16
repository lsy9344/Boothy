import { act, render, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type {
  DisplayGeneration,
  DisplayPointerSnapshot,
  ViewerReadinessSnapshot,
} from '../../shared-contracts'
import { viewerDisplaySchemaVersion } from '../../shared-contracts'
import type { DisplayHostService } from '../../viewer-surface/services/viewer-host-adapter'
import { shouldApplyPointer, useDisplayPointer } from './use-display-pointer'

const SESSION_A = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
const SESSION_B = 'session_01hs6n1r8b8zc5v4ey2x7b9g2n'

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
    proxyProvenance: null,
    committedAtHostMicros: 1_000_000,
    ...overrides,
  }
}

function buildProxyGeneration(
  overrides: Partial<DisplayGeneration> = {},
): DisplayGeneration {
  return buildGeneration({
    tier: 'displayFitPresetProxy',
    sampleVariant: null,
    proxyProvenance: {
      presetId: 'preset_soft-glow',
      presetVersion: '2026.08.01',
      approvalBasis: 'exact-reference-renderer',
      proxyRecipeVersion: '1',
      referenceRenderer: 'darktable',
      referenceRendererVersion: '5.4.1',
      renderProfileId: 'preset_soft_glow-preview',
      outputColorSpace: 'sRGB',
      jpegQuality: 92,
      sourceRoute: 'raw-original',
      sourceAssetHash: 'fnv1a64:00000000000000aa',
      targetWidthPx: 1620,
      targetHeightPx: 1080,
      displayProfileId: 'approved-1080p',
      devicePixelRatio: 1,
      residentProvenance: null,
      renderQuality: 'fast',
    },
    ...overrides,
  })
}

afterEach(() => {
  vi.useRealTimers()
})

function buildPointer(
  overrides: Partial<DisplayPointerSnapshot> = {},
): DisplayPointerSnapshot {
  return {
    schemaVersion: viewerDisplaySchemaVersion,
    sessionId: SESSION_A,
    revision: 1,
    activeGeneration: buildGeneration(),
    requiredSourceWidthPx: 1620,
    requiredSourceHeightPx: 1080,
    measurementLaneEnabled: true,
    presentTelemetryEnabled: true,
    observedAtHostMicros: 1_000_000,
    ...overrides,
  }
}

function buildReadiness(
  overrides: Partial<ViewerReadinessSnapshot> = {},
): ViewerReadinessSnapshot {
  return {
    schemaVersion: 'viewer-readiness/v1',
    sessionId: SESSION_A,
    viewerEpoch: 3,
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
    observedAtMs: 1_000,
    lastReportAtMs: 900,
    ...overrides,
  }
}

type Harness = {
  service: DisplayHostService
  emit(pointer: DisplayPointerSnapshot): void
}

function buildHostService(initial: DisplayPointerSnapshot): Harness {
  let listener: ((pointer: DisplayPointerSnapshot) => void) | null = null

  return {
    emit(pointer) {
      listener?.(pointer)
    },
    service: {
      async getViewerDisplayState() {
        return initial
      },
      async reportDisplayPresent() {
        return undefined
      },
      async stampClockProbe() {
        return { hostMonotonicMicros: 0, hostEpochMicros: 0 }
      },
      async subscribeToDisplayUpdates(input) {
        listener = input.onPointer
        return () => {
          listener = null
        }
      },
    },
  }
}

/**
 * 훅 반환값을 테스트에서 읽기 위한 상자.
 * 지역 `let`은 콜백 안에서만 대입되어 TypeScript가 `never`로 좁혀 버린다.
 */
function createStateBox() {
  return { latest: null as ReturnType<typeof useDisplayPointer> | null }
}

type ProbeProps = {
  readiness: ViewerReadinessSnapshot
  hostService: DisplayHostService
  onState(state: ReturnType<typeof useDisplayPointer>): void
}

function Probe({ readiness, hostService, onState }: ProbeProps) {
  onState(useDisplayPointer({ readiness, hostService }))

  return null
}

function renderHook(
  harness: Harness,
  readiness: ViewerReadinessSnapshot,
  onState: (state: ReturnType<typeof useDisplayPointer>) => void,
) {
  return render(
    <Probe
      readiness={readiness}
      hostService={harness.service}
      onState={onState}
    />,
  )
}

describe('shouldApplyPointer', () => {
  it('discards a lower revision', () => {
    expect(
      shouldApplyPointer(buildPointer({ revision: 5 }), buildPointer({ revision: 4 })),
    ).toBe(false)
  })

  it('applies a higher revision', () => {
    expect(
      shouldApplyPointer(buildPointer({ revision: 5 }), buildPointer({ revision: 6 })),
    ).toBe(true)
  })

  it('applies equal-revision changes that alter display truth or guard context', () => {
    const current = buildPointer({ revision: 5 })

    expect(shouldApplyPointer(current, buildPointer({ revision: 5 }))).toBe(false)
    expect(
      shouldApplyPointer(
        current,
        buildPointer({
          revision: 5,
          activeGeneration: buildGeneration({ generationId: 'req-1-000002' }),
        }),
      ),
    ).toBe(true)
    expect(
      shouldApplyPointer(
        current,
        buildPointer({ revision: 5, requiredSourceWidthPx: 1920 }),
      ),
    ).toBe(true)
    expect(
      shouldApplyPointer(
        current,
        buildPointer({ revision: 5, measurementLaneEnabled: false }),
      ),
    ).toBe(true)
  })
})

describe('useDisplayPointer', () => {
  it('converges on the snapshot before any live event arrives', async () => {
    const harness = buildHostService(buildPointer())
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })

    await waitFor(() => {
      expect(box.latest?.generation?.generationId).toBe('req-1-000001')
    })
  })

  it('heals a snapshot failure and the subscription gap after listening starts', async () => {
    let current = buildPointer({ revision: 0, activeGeneration: null })
    let releaseSubscription!: (unlisten: () => void) => void
    const getViewerDisplayState = vi
      .fn<DisplayHostService['getViewerDisplayState']>()
      .mockRejectedValueOnce(new Error('snapshot unavailable'))
      .mockImplementation(async () => current)
    const service: DisplayHostService = {
      getViewerDisplayState,
      async reportDisplayPresent() {
        return undefined
      },
      async stampClockProbe() {
        return { hostMonotonicMicros: 0, hostEpochMicros: 0 }
      },
      subscribeToDisplayUpdates: () =>
        new Promise((resolve) => {
          releaseSubscription = resolve
        }),
    }
    const harness: Harness = { service, emit() {} }
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(getViewerDisplayState).toHaveBeenCalledTimes(2))

    // listener가 성립되기 직전에 commit된 generation은 post-subscribe snapshot이 회수한다.
    current = buildPointer({ revision: 2 })
    releaseSubscription(() => undefined)

    await waitFor(() => {
      expect(getViewerDisplayState).toHaveBeenCalledTimes(3)
      expect(box.latest?.generation?.generationId).toBe('req-1-000001')
    })
  })

  it('retries a failed display subscription', async () => {
    vi.useFakeTimers()
    let listener: ((pointer: DisplayPointerSnapshot) => void) | null = null
    const subscribeToDisplayUpdates = vi
      .fn<DisplayHostService['subscribeToDisplayUpdates']>()
      .mockRejectedValueOnce(new Error('listener unavailable'))
      .mockImplementation(async (input) => {
        listener = input.onPointer
        return () => {
          listener = null
        }
      })
    const service: DisplayHostService = {
      async getViewerDisplayState() {
        return buildPointer({ revision: 0, activeGeneration: null })
      },
      async reportDisplayPresent() {
        return undefined
      },
      async stampClockProbe() {
        return { hostMonotonicMicros: 0, hostEpochMicros: 0 }
      },
      subscribeToDisplayUpdates,
    }
    const harness: Harness = { service, emit(pointer) { listener?.(pointer) } }
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await vi.waitFor(() => expect(subscribeToDisplayUpdates).toHaveBeenCalledTimes(1))

    await vi.advanceTimersByTimeAsync(1_000)
    await vi.waitFor(() => expect(subscribeToDisplayUpdates).toHaveBeenCalledTimes(2))

    act(() => harness.emit(buildPointer({ revision: 2 })))
    expect(box.latest?.generation?.generationId).toBe('req-1-000001')
  })

  it('ignores a delayed lower-revision update', async () => {
    const harness = buildHostService(buildPointer({ revision: 5 }))
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(box.latest?.pointer.revision).toBe(5))

    act(() => {
      harness.emit(
        buildPointer({
          revision: 2,
          activeGeneration: buildGeneration({
            generationId: 'stale',
            generationSeq: 99,
          }),
        }),
      )
    })

    expect(box.latest?.generation?.generationId).toBe('req-1-000001')
  })

  it('ignores a duplicate update', async () => {
    const harness = buildHostService(buildPointer({ revision: 3 }))
    let renderCountAfterReady = 0
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
      renderCountAfterReady += 1
    })
    await waitFor(() => expect(box.latest?.pointer.revision).toBe(3))

    const before = renderCountAfterReady
    act(() => {
      harness.emit(buildPointer({ revision: 3 }))
    })

    expect(renderCountAfterReady).toBe(before)
  })

  it('rejects an out-of-order generation from an older request', async () => {
    const harness = buildHostService(
      buildPointer({
        revision: 5,
        activeGeneration: buildGeneration({
          requestId: 'req-2',
          generationSeq: 5,
          committedAtHostMicros: 5_000_000,
        }),
      }),
    )
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(box.latest?.generation?.requestId).toBe('req-2'))

    act(() => {
      harness.emit(
        buildPointer({
          revision: 6,
          activeGeneration: buildGeneration({
            requestId: 'req-1',
            generationSeq: 6,
            committedAtHostMicros: 2_000_000,
          }),
        }),
      )
    })

    // 늦게 도착한 이전 request는 화면을 되돌릴 수 없다.
    expect(box.latest?.generation?.requestId).toBe('req-2')
  })

  it('clears the display when the host clears the pointer for a new session', async () => {
    const harness = buildHostService(buildPointer())
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(box.latest?.generation).not.toBeNull())

    act(() => {
      harness.emit(
        buildPointer({
          revision: 9,
          sessionId: SESSION_B,
          activeGeneration: null,
        }),
      )
    })

    expect(box.latest?.generation).toBeNull()
  })

  it('drops a generation that belongs to a previous viewer epoch', async () => {
    const harness = buildHostService(buildPointer())
    const box = createStateBox()

    const onState = (state: ReturnType<typeof useDisplayPointer>) => {
      box.latest = state
    }
    const { rerender } = renderHook(harness, buildReadiness(), onState)
    await waitFor(() => expect(box.latest?.generation).not.toBeNull())

    // 관람 창이 재생성되어 epoch가 올라갔다.
    rerender(
      <Probe
        readiness={buildReadiness({ viewerEpoch: 4 })}
        hostService={harness.service}
        onState={onState}
      />,
    )

    await waitFor(() => {
      expect(box.latest?.generation).toBeNull()
    })
  })

  it('re-reads the snapshot when the viewer epoch changes', async () => {
    const pointer = buildPointer()
    const harness = buildHostService(pointer)
    const spy = vi.spyOn(harness.service, 'getViewerDisplayState')
    const box = createStateBox()
    const onState = (state: ReturnType<typeof useDisplayPointer>) => {
      box.latest = state
    }

    const { rerender } = renderHook(harness, buildReadiness(), onState)
    await waitFor(() => expect(box.latest?.generation).not.toBeNull())
    const initialCalls = spy.mock.calls.length

    rerender(
      <Probe
        readiness={buildReadiness({ viewerEpoch: 4 })}
        hostService={harness.service}
        onState={onState}
      />,
    )

    await waitFor(() => {
      expect(spy.mock.calls.length).toBeGreaterThan(initialCalls)
    })
  })

  it('never re-admits a generation that failed to decode', async () => {
    const harness = buildHostService(buildPointer())
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(box.latest?.generation).not.toBeNull())

    await act(async () => {
      await box.latest?.reportPresent({
        generationId: 'req-1-000001',
        viewerEpoch: 3,
        outcome: 'decode-failed',
        rejectReason: 'decode-failed',
        naturalWidthPx: 0,
        naturalHeightPx: 0,
        spans: {
          viewerReceiptAtMicros: null,
          decodeStartAtMicros: 1,
          decodeEndAtMicros: 2,
          swapCommittedAtMicros: null,
          imgOnLoadAtMicros: null,
          actualPresentAtMicros: null,
          elementTimingRenderAtMicros: null,
          isElementRenderTime: false,
        },
        clockOffsetMicros: 0,
        clockUncertaintyMicros: 0,
      })
    })

    expect(box.latest?.generation).toBeNull()

    act(() => {
      harness.emit(buildPointer({ revision: 7 }))
    })

    expect(box.latest?.generation).toBeNull()
  })

  it('does not permanently poison a generation for a transient rejection', async () => {
    const harness = buildHostService(buildPointer())
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(box.latest?.generation).not.toBeNull())

    await act(async () => {
      await box.latest?.reportPresent({
        generationId: 'req-1-000001',
        viewerEpoch: 3,
        outcome: 'rejected',
        rejectReason: 'insufficient-dimensions',
        naturalWidthPx: 100,
        naturalHeightPx: 100,
        spans: {
          viewerReceiptAtMicros: null,
          decodeStartAtMicros: 1,
          decodeEndAtMicros: 2,
          swapCommittedAtMicros: null,
          imgOnLoadAtMicros: null,
          actualPresentAtMicros: null,
          elementTimingRenderAtMicros: null,
          isElementRenderTime: false,
        },
        clockOffsetMicros: 0,
        clockUncertaintyMicros: 0,
      })
    })

    expect(box.latest?.generation?.generationId).toBe('req-1-000001')

    act(() => {
      harness.emit(
        buildPointer({
          revision: 7,
          activeGeneration: buildGeneration({
            generationId: 'req-1-000002',
            generationSeq: 2,
            sampleVariant: 'b',
          }),
        }),
      )
    })

    await waitFor(() => {
      expect(box.latest?.generation?.generationId).toBe('req-1-000002')
    })
  })

  it('keeps the proxy selected when its raw refined upgrade fails to decode', async () => {
    const proxy = buildProxyGeneration()
    const harness = buildHostService(
      buildPointer({ revision: 1, activeGeneration: proxy }),
    )
    const box = createStateBox()

    renderHook(harness, buildReadiness(), (state) => {
      box.latest = state
    })
    await waitFor(() => expect(box.latest?.generation?.tier).toBe('displayFitPresetProxy'))

    const refined = buildProxyGeneration({
      generationId: 'req-1-000002',
      generationSeq: 2,
      tier: 'rawRefinedDisplay',
      proxyProvenance: {
        ...proxy.proxyProvenance!,
        renderQuality: 'high',
      },
    })
    act(() => {
      harness.emit(buildPointer({ revision: 2, activeGeneration: refined }))
    })
    await waitFor(() => expect(box.latest?.generation?.tier).toBe('rawRefinedDisplay'))

    await act(async () => {
      await box.latest?.reportPresent({
        generationId: refined.generationId,
        viewerEpoch: refined.viewerEpoch,
        outcome: 'decode-failed',
        rejectReason: 'decode-failed',
        naturalWidthPx: 0,
        naturalHeightPx: 0,
        spans: {
          viewerReceiptAtMicros: null,
          decodeStartAtMicros: 1,
          decodeEndAtMicros: 2,
          swapCommittedAtMicros: null,
          imgOnLoadAtMicros: null,
          actualPresentAtMicros: null,
          elementTimingRenderAtMicros: null,
          isElementRenderTime: false,
        },
        clockOffsetMicros: 0,
        clockUncertaintyMicros: 0,
      })
    })

    expect(box.latest?.generation?.generationId).toBe(proxy.generationId)

    // host가 같은 실패 generation을 다시 알려도 poison guard가 proxy를 유지한다.
    act(() => {
      harness.emit(buildPointer({ revision: 3, activeGeneration: refined }))
    })
    expect(box.latest?.generation?.generationId).toBe(proxy.generationId)
  })

  it('scopes decode-failed poisoning to one session and viewer epoch', async () => {
    const harness = buildHostService(buildPointer())
    const box = createStateBox()
    const onState = (state: ReturnType<typeof useDisplayPointer>) => {
      box.latest = state
    }
    const { rerender } = renderHook(harness, buildReadiness(), onState)
    await waitFor(() => expect(box.latest?.generation).not.toBeNull())

    await act(async () => {
      await box.latest?.reportPresent({
        generationId: 'req-1-000001',
        viewerEpoch: 3,
        outcome: 'decode-failed',
        rejectReason: 'decode-failed',
        naturalWidthPx: 0,
        naturalHeightPx: 0,
        spans: {
          viewerReceiptAtMicros: null,
          decodeStartAtMicros: 1,
          decodeEndAtMicros: 2,
          swapCommittedAtMicros: null,
          imgOnLoadAtMicros: null,
          actualPresentAtMicros: null,
          elementTimingRenderAtMicros: null,
          isElementRenderTime: false,
        },
        clockOffsetMicros: 0,
        clockUncertaintyMicros: 0,
      })
    })
    expect(box.latest?.generation).toBeNull()

    rerender(
      <Probe
        readiness={buildReadiness({ sessionId: SESSION_B, viewerEpoch: 4 })}
        hostService={harness.service}
        onState={onState}
      />,
    )
    act(() => {
      harness.emit(
        buildPointer({
          revision: 8,
          sessionId: SESSION_B,
          activeGeneration: buildGeneration({
            sessionId: SESSION_B,
            viewerEpoch: 4,
          }),
        }),
      )
    })

    await waitFor(() => {
      expect(box.latest?.generation?.sessionId).toBe(SESSION_B)
    })
  })
})
