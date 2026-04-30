import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { SessionCaptureRecord } from '../../shared-contracts'
import {
  CAPTURE_READINESS_POLL_MS,
  CAPTURE_READY_IDLE_POLL_MS,
  createBrowserCaptureRuntimeGateway,
  createCaptureRuntimeService,
  createTauriCaptureRuntimeGateway,
} from './capture-runtime'
import type { CaptureRuntimeGateway } from './capture-runtime'

function createCaptureRecord(
  overrides: Partial<SessionCaptureRecord> = {},
): SessionCaptureRecord {
  return {
    schemaVersion: 'session-capture/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetId: 'preset_soft-glow',
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
    ...overrides,
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
        expect(payload).toMatchObject({
          input: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            requestId: expect.stringMatching(/^request_[0-9a-f]+$/),
          },
        })

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

      if (cmd === 'delete_capture') {
        expect(payload).toEqual({
          input: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
          },
        })

        return {
          schemaVersion: 'capture-delete-result/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
          status: 'capture-deleted',
          manifest: {
            schemaVersion: 'session-manifest/v1',
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            boothAlias: 'Kim 4821',
            customer: {
              name: 'Kim',
              phoneLastFour: '4821',
            },
            createdAt: '2026-03-20T00:00:00.000Z',
            updatedAt: '2026-03-20T00:00:00.000Z',
            lifecycle: {
              status: 'active',
              stage: 'capture-ready',
            },
            activePreset: {
              presetId: 'preset_soft-glow',
              publishedVersion: '2026.03.20',
            },
            activePresetId: 'preset_soft-glow',
            captures: [],
            postEnd: null,
          },
          readiness: {
            schemaVersion: 'capture-readiness/v1',
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            surfaceState: 'captureReady',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '버튼을 누르면 바로 시작돼요.',
            reasonCode: 'ready',
            latestCapture: null,
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

    await expect(
      service.deleteCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).resolves.toMatchObject({
      status: 'capture-deleted',
      manifest: {
        captures: [],
      },
    })
  })

  it('preserves a caller-supplied capture request id when invoking tauri IPC', async () => {
    mockIPC((cmd, payload) => {
      if (cmd === 'request_capture') {
        expect(payload).toEqual({
          input: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            requestId: 'request_client_supplied',
          },
        })

        return {
          schemaVersion: 'capture-request-result/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          status: 'capture-saved',
          capture: createCaptureRecord({
            requestId: 'request_client_supplied',
          }),
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
            latestCapture: createCaptureRecord({
              requestId: 'request_client_supplied',
            }),
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
        requestId: 'request_client_supplied',
      }),
    ).resolves.toMatchObject({
      capture: {
        requestId: 'request_client_supplied',
      },
    })
  })

  it('primes the preview runtime through tauri IPC with the active preset binding', async () => {
    mockIPC((cmd, payload) => {
      if (cmd === 'prime_preview_runtime') {
        expect(payload).toEqual({
          input: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        })

        return null
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.primePreviewRuntime?.({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      }),
    ).resolves.toBeUndefined()
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

  it('preserves retryable capture guidance from the host without promoting phone-required', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_capture') {
        throw {
          code: 'capture-not-ready',
          message: '사진을 아직 찍지 못했어요.',
          readiness: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            customerState: 'Preparing',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진을 아직 찍지 못했어요.',
            supportMessage: '대상을 다시 맞춘 뒤 잠시 후 다시 시도해 주세요.',
            reasonCode: 'capture-retry-required',
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
        canCapture: false,
        primaryAction: 'wait',
        reasonCode: 'capture-retry-required',
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

  it('downgrades capture-not-ready without readiness to a transient wait state', async () => {
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
      message: '촬영 준비 상태를 다시 확인하고 있어요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        customerState: 'Preparing',
        primaryAction: 'wait',
        canCapture: false,
        reasonCode: 'camera-preparing',
      },
    })
  })

  it('downgrades same-session request session-not-found to a transient wait state', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_capture') {
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

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'session-not-found',
      message: '촬영 준비 상태를 다시 확인하고 있어요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        customerState: 'Preparing',
        primaryAction: 'wait',
        canCapture: false,
        reasonCode: 'camera-preparing',
      },
    })
  })

  it('downgrades raw request failures to customer-safe preparing guidance', async () => {
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
      message: '촬영 준비 상태를 다시 확인하고 있어요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        customerState: 'Preparing',
        primaryAction: 'wait',
        reasonCode: 'camera-preparing',
      },
    })
  })

  it('downgrades generic readiness refresh failures to preparing instead of phone-required', async () => {
    mockIPC((cmd) => {
      if (cmd === 'get_capture_readiness') {
        throw {
          code: 'session-persistence-failed',
          message: 'unexpected runtime bridge failure',
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
      code: 'session-persistence-failed',
      message: '촬영 준비 상태를 다시 확인하고 있어요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        customerState: 'Preparing',
        primaryAction: 'wait',
        reasonCode: 'camera-preparing',
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

  it('rejects delete responses whose session id does not match the request', async () => {
    mockIPC((cmd) => {
      if (cmd === 'delete_capture') {
        return {
          schemaVersion: 'capture-delete-result/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
          status: 'capture-deleted',
          manifest: {
            schemaVersion: 'session-manifest/v1',
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            boothAlias: 'Kim 4821',
            customer: {
              name: 'Kim',
              phoneLastFour: '4821',
            },
            createdAt: '2026-03-20T00:00:00.000Z',
            updatedAt: '2026-03-20T00:00:00.000Z',
            lifecycle: {
              status: 'active',
              stage: 'capture-ready',
            },
            activePreset: {
              presetId: 'preset_soft-glow',
              publishedVersion: '2026.03.20',
            },
            activePresetId: 'preset_soft-glow',
            captures: [],
            postEnd: null,
          },
          readiness: {
            schemaVersion: 'capture-readiness/v1',
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            surfaceState: 'captureReady',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '버튼을 누르면 바로 시작돼요.',
            reasonCode: 'ready',
            latestCapture: null,
          },
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.deleteCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '현재 세션 상태를 다시 확인할게요.',
    })
  })

  it('preserves a customer-safe delete-blocked message from the host', async () => {
    mockIPC((cmd) => {
      if (cmd === 'delete_capture') {
        throw {
          code: 'capture-delete-blocked',
          message: '이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.',
          readiness: {
            schemaVersion: 'capture-readiness/v1',
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            surfaceState: 'previewReady',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '방금 찍은 사진을 아래에서 바로 확인할 수 있어요.',
            reasonCode: 'ready',
            latestCapture: createCaptureRecord({
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/current-session-preview.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          },
        }
      }

      return undefined
    })

    const service = createCaptureRuntimeService({
      gateway: createTauriCaptureRuntimeGateway(),
    })

    await expect(
      service.deleteCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'capture-delete-blocked',
      message: '이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      },
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
      customerState: 'Preparing',
      canCapture: false,
      supportMessage: '브라우저 미리보기에서는 실제 카메라 연결을 확인할 수 없어요.',
    })

    await expect(
      service.requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      readiness: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
        primaryAction: 'wait',
        customerState: 'Preparing',
        canCapture: false,
      },
    })
  })

  it('downgrades browser fixture readiness that tries to claim Ready', async () => {
    ;(
      globalThis as typeof globalThis & {
        __BOOTHY_BROWSER_CAPTURE_READINESS__?: unknown
      }
    ).__BOOTHY_BROWSER_CAPTURE_READINESS__ = {
      schemaVersion: 'capture-readiness/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      surfaceState: 'captureReady',
      customerState: 'Ready',
      canCapture: true,
      primaryAction: 'capture',
      customerMessage: '지금 촬영할 수 있어요.',
      supportMessage: '버튼을 누르면 바로 시작돼요.',
      reasonCode: 'ready',
      latestCapture: null,
      postEnd: null,
      timing: null,
    }

    const service = createCaptureRuntimeService({
      gateway: createBrowserCaptureRuntimeGateway(),
    })

    await expect(
      service.getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      }),
    ).resolves.toMatchObject({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      customerState: 'Preparing',
      canCapture: false,
      primaryAction: 'wait',
      supportMessage: '브라우저 미리보기에서는 실제 카메라 연결을 확인할 수 없어요.',
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

    await vi.advanceTimersByTimeAsync(CAPTURE_READINESS_POLL_MS)
    await vi.advanceTimersByTimeAsync(CAPTURE_READINESS_POLL_MS)

    expect(onReadiness).toHaveBeenCalledTimes(2)

    unlisten()
    await vi.advanceTimersByTimeAsync(CAPTURE_READINESS_POLL_MS)

    expect(onReadiness).toHaveBeenCalledTimes(2)
  })

  it('backs off readiness polling while the session is idle and ready', async () => {
    vi.useFakeTimers()
    let pollCount = 0

    mockIPC((cmd) => {
      if (cmd === 'get_capture_readiness') {
        pollCount += 1

        return {
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
          latestCapture: null,
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

    await vi.advanceTimersByTimeAsync(CAPTURE_READINESS_POLL_MS)
    expect(onReadiness).toHaveBeenCalledTimes(1)
    expect(pollCount).toBe(1)

    await vi.advanceTimersByTimeAsync(CAPTURE_READINESS_POLL_MS)
    expect(pollCount).toBe(1)

    await vi.advanceTimersByTimeAsync(
      CAPTURE_READY_IDLE_POLL_MS - CAPTURE_READINESS_POLL_MS,
    )
    expect(pollCount).toBe(2)

    unlisten()
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

    await vi.advanceTimersByTimeAsync(CAPTURE_READINESS_POLL_MS)

    expect(onReadiness).toHaveBeenCalledWith(
      expect.objectContaining({
        primaryAction: 'start-session',
        customerState: 'Preparing',
      }),
    )

    unlisten()
  })

  it('filters fast-preview updates by session before notifying the consumer', async () => {
    let emitFastPreview: ((payload: unknown) => void) | null = null

    const gateway: CaptureRuntimeGateway = {
      getCaptureReadiness: vi.fn(),
      requestCapture: vi.fn(),
      subscribeToCaptureReadiness: vi.fn().mockResolvedValue(() => undefined),
      subscribeToCaptureFastPreview: vi.fn().mockImplementation(async (onEvent) => {
        emitFastPreview = onEvent
        return () => {
          emitFastPreview = null
        }
      }),
    }
    const service = createCaptureRuntimeService({
      gateway,
    })
    const onFastPreview = vi.fn()

    const unlisten = await service.subscribeToCaptureFastPreview!({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      onFastPreview,
    })
    const dispatchFastPreview = emitFastPreview as unknown as (
      payload: unknown,
    ) => void

    dispatchFastPreview({
      schemaVersion: 'capture-fast-preview-update/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      requestId: 'request_foreign',
      captureId: 'capture_foreign',
      assetPath:
        'C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1n/renders/previews/capture_foreign.jpg',
      visibleAtMs: 320,
      kind: 'camera-thumbnail',
    })
    dispatchFastPreview({
      schemaVersion: 'capture-fast-preview-update/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_local',
      captureId: 'capture_local',
      assetPath:
        'C:/Users/Example/Pictures/dabi_shoot/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/capture_local.jpg',
      visibleAtMs: 360,
      kind: 'camera-thumbnail',
    })

    expect(onFastPreview).toHaveBeenCalledTimes(1)
    expect(onFastPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        captureId: 'capture_local',
        requestId: 'request_local',
      }),
    )

    unlisten()
  })

  it('forwards fast preview updates only for the active session subscription', async () => {
    let emitFastPreview: ((payload: unknown) => void) | null = null
    const onFastPreview = vi.fn()

    const service = createCaptureRuntimeService({
      gateway: {
        getCaptureReadiness: vi.fn(),
        requestCapture: vi.fn(),
        subscribeToCaptureReadiness: vi.fn(async () => () => undefined),
        subscribeToCaptureFastPreview: vi.fn(async (onEvent) => {
          emitFastPreview = onEvent
          return () => {
            emitFastPreview = null
          }
        }),
      },
    })

    const unlisten = await service.subscribeToCaptureFastPreview!({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      onFastPreview,
    })
    const dispatchFastPreview = emitFastPreview as unknown as (
      payload: unknown,
    ) => void

    dispatchFastPreview({
      schemaVersion: 'capture-fast-preview-update/v1',
      sessionId: 'session_other',
      requestId: 'request_other',
      captureId: 'capture_other',
      assetPath: 'C:/boothy/sessions/session_other/renders/previews/capture.jpg',
      visibleAtMs: 120,
      kind: 'camera-thumbnail',
    })
    dispatchFastPreview({
      schemaVersion: 'capture-fast-preview-update/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      assetPath:
        'C:/boothy/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/capture_01hs6n1r8b8zc5v4ey2x7b9g1m.jpg',
      visibleAtMs: 140,
      kind: 'camera-thumbnail',
    })

    expect(onFastPreview).toHaveBeenCalledTimes(1)
    expect(onFastPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
        kind: 'camera-thumbnail',
      }),
    )

    unlisten()
  })

  it('recovers to tauri readiness when the runtime becomes available after service creation', async () => {
    const runtimeWindow = window as typeof window & {
      __TAURI_INTERNALS__?: unknown
    }

    delete runtimeWindow.__TAURI_INTERNALS__

    const service = createCaptureRuntimeService()

    runtimeWindow.__TAURI_INTERNALS__ = {}

    mockIPC((cmd, payload) => {
      if (cmd === 'get_capture_readiness') {
        expect(payload).toEqual({
          input: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          },
        })

        return {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'captureReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
          latestCapture: null,
          postEnd: null,
          timing: null,
        }
      }

      return undefined
    })

    await expect(
      service.getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).resolves.toMatchObject({
      customerState: 'Ready',
      canCapture: true,
      reasonCode: 'ready',
    })
  })
})
