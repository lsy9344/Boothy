import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  createBrowserCaptureRuntimeGateway,
  createCaptureRuntimeService,
  createTauriCaptureRuntimeGateway,
} from './capture-runtime'

function createCaptureRecord() {
  return {
    schemaVersion: 'session-capture/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetVersion: '2026.03.20',
    captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
    requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
    raw: {
      assetPath: 'fixtures/capture-raw.jpg',
      persistedAtMs: 100,
    },
    preview: {
      assetPath: null,
      enqueuedAtMs: 100,
      readyAtMs: null,
    },
    final: {
      assetPath: null,
      readyAtMs: null,
    },
    renderStatus: 'previewWaiting',
    postEndState: 'activeSession',
    timing: {
      captureAcknowledgedAtMs: 100,
      previewVisibleAtMs: null,
      captureBudgetMs: 1000,
      previewBudgetMs: 5000,
      previewBudgetState: 'pending',
    },
  }
}

afterEach(() => {
  clearMocks()
  vi.useRealTimers()
  delete (
    globalThis as typeof globalThis & {
      __BOOTHY_BROWSER_CAPTURE_READINESS__?: unknown
    }
  ).__BOOTHY_BROWSER_CAPTURE_READINESS__
})

describe('capture runtime adapter', () => {
  it('parses typed readiness and capture request responses from tauri IPC', async () => {
    mockIPC((cmd, payload) => {
      if (cmd === 'get_capture_readiness') {
        expect(payload).toEqual({
          input: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          },
        })

        return {
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
        }
      }

      if (cmd === 'request_capture') {
        return {
          schemaVersion: 'capture-request-result/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          status: 'capture-saved',
          capture: createCaptureRecord(),
          readiness: {
            schemaVersion: 'capture-readiness/v1',
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            surfaceState: 'captureSaved',
            customerState: 'Preview Waiting',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진이 안전하게 저장되었어요.',
            supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
            reasonCode: 'preview-waiting',
            latestCapture: createCaptureRecord(),
          },
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).resolves.toMatchObject({
      customerState: 'Ready',
      canCapture: true,
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).resolves.toMatchObject({
      status: 'capture-saved',
      capture: {
        renderStatus: 'previewWaiting',
      },
    })
  })

  it('normalizes a blocked capture response with customer-safe next action', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_capture') {
        throw {
          code: 'capture-not-ready',
          message: '지금은 촬영할 수 없어요.',
          readiness: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            customerState: 'Phone Required',
            canCapture: false,
            primaryAction: 'call-support',
            customerMessage: '지금은 도움이 필요해요.',
            supportMessage: '가까운 직원에게 알려 주세요.',
            reasonCode: 'phone-required',
          },
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'capture-not-ready',
      readiness: {
        primaryAction: 'call-support',
      },
    })
  })

  it('rejects readiness responses whose session id does not match the request', async () => {
    mockIPC((cmd) => {
      if (cmd === 'get_capture_readiness') {
        return {
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '현재 세션 상태를 다시 확인할게요.',
    })
  })

  it('adds customer-safe blocked readiness when capture-not-ready arrives without readiness', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_capture') {
        throw {
          code: 'capture-not-ready',
          message: 'camera helper busy',
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'capture-not-ready',
      message: '지금은 도움이 필요해요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        customerState: 'Phone Required',
        primaryAction: 'call-support',
        canCapture: false,
      },
    })
  })

  it('sanitizes raw host failures into customer-safe phone-required guidance', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_capture') {
        throw {
          code: 'session-persistence-failed',
          message: 'serde_json parse failure at captures/manifest.json',
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'session-persistence-failed',
      message: '지금은 도움이 필요해요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        customerState: 'Phone Required',
        primaryAction: 'call-support',
      },
    })
  })

  it('rejects capture responses whose session id does not match the request', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_capture') {
        return {
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          status: 'capture-saved',
          capture: {
            ...createCaptureRecord(),
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          },
          readiness: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '버튼을 누르면 바로 시작돼요.',
            reasonCode: 'ready',
          },
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '현재 세션 상태를 다시 확인할게요.',
    })
  })

  it('keeps browser fallback readiness scoped to the requested session', async () => {
    const service = createCaptureRuntimeService({
      gateway: createBrowserCaptureRuntimeGateway(),
    })

    await expect(
      service.getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      }),
    ).resolves.toMatchObject({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      customerState: 'Ready',
      canCapture: true,
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
        primaryAction: 'call-support',
      },
    })
  })

  it('polls readiness snapshots and stops after cleanup', async () => {
    vi.useFakeTimers()
    let pollCount = 0

    mockIPC((cmd) => {
      if (cmd === 'get_capture_readiness') {
        pollCount += 1

        return pollCount === 1
          ? {
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
              customerState: 'Preparing',
              canCapture: false,
              primaryAction: 'wait',
              customerMessage: '촬영 준비 중이에요.',
              supportMessage: '잠시만 기다려 주세요.',
              reasonCode: 'camera-preparing',
            }
          : {
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
            }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })
    const onReadiness = vi.fn()

    const unlisten = await service.subscribeToCaptureReadiness({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      onReadiness,
    })

    await vi.advanceTimersByTimeAsync(1500)
    await vi.advanceTimersByTimeAsync(1500)

    expect(onReadiness).toHaveBeenCalledTimes(2)

    unlisten()
    await vi.advanceTimersByTimeAsync(1500)

    expect(onReadiness).toHaveBeenCalledTimes(2)
  })

  it('emits customer-safe readiness when polling hits a normalized host error', async () => {
    vi.useFakeTimers()

    mockIPC((cmd) => {
      if (cmd === 'get_capture_readiness') {
        throw {
          code: 'session-not-found',
          message: 'manifest missing',
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })
    const onReadiness = vi.fn()

    const unlisten = await service.subscribeToCaptureReadiness({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      onReadiness,
    })

    await vi.advanceTimersByTimeAsync(1500)

    expect(onReadiness).toHaveBeenCalledWith(
      expect.objectContaining({
        primaryAction: 'start-session',
        customerState: 'Preparing',
      }),
    )

    unlisten()
  })
})
