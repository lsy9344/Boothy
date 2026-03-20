import { act, render, waitFor } from '@testing-library/react'
import { useEffect } from 'react'
import { describe, expect, it, vi } from 'vitest'

import {
  createCaptureRuntimeService,
  type CaptureRuntimeGateway,
  type CaptureRuntimeService,
} from '../../capture-adapter/services/capture-runtime'
import {
  createPresetCatalogService,
  type PresetCatalogGateway,
} from '../../preset-catalog/services/preset-catalog-service'
import type {
  CaptureReadinessSnapshot,
  CaptureRequestResult,
  PresetCatalogResult,
  SessionCaptureRecord,
  SessionStartResult,
} from '../../shared-contracts'
import {
  createStartSessionService,
  type StartSessionGateway,
} from '../services/start-session'
import type { SessionStateContextValue } from './session-context'
import { SessionProvider } from './session-provider'
import { useSessionState } from './use-session-state'

function createSessionStartResult(
  sessionId: string,
  boothAlias: string,
): SessionStartResult {
  return {
    sessionId,
    boothAlias,
    manifest: {
      schemaVersion: 'session-manifest/v1',
      sessionId,
      boothAlias,
      customer: {
        name: boothAlias.split(' ')[0] ?? boothAlias,
        phoneLastFour: boothAlias.split(' ')[1] ?? '0000',
      },
      createdAt: '2026-03-20T00:00:00.000Z',
      updatedAt: '2026-03-20T00:00:00.000Z',
      lifecycle: {
        status: 'active',
        stage: 'session-started',
      },
      activePreset: null,
      captures: [],
      postEnd: null,
    },
  }
}

function createPresetCatalogResult(sessionId: string): PresetCatalogResult {
  return {
    sessionId,
    state: 'ready',
    presets: [
      {
        presetId: 'preset_soft-glow',
        displayName: 'Soft Glow',
        publishedVersion: '2026.03.20',
        boothStatus: 'booth-safe',
        preview: {
          kind: 'preview-tile',
          assetPath: 'fixtures/soft-glow.jpg',
          altText: 'Soft Glow sample portrait',
        },
      },
    ],
  }
}

function createCaptureRecord(
  overrides: Partial<SessionCaptureRecord> = {},
): SessionCaptureRecord {
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
    ...overrides,
  }
}

function createReadinessSnapshot(
  overrides: Partial<CaptureReadinessSnapshot> = {},
): CaptureReadinessSnapshot {
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
    ...overrides,
  }
}

function createCaptureRequestResult(
  overrides: Partial<CaptureRequestResult> = {},
): CaptureRequestResult {
  return {
    schemaVersion: 'capture-request-result/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    status: 'capture-saved',
    capture: createCaptureRecord(),
    readiness: createReadinessSnapshot({
      surfaceState: 'captureSaved',
      customerState: 'Preview Waiting',
      canCapture: false,
      primaryAction: 'wait',
      customerMessage: '사진이 안전하게 저장되었어요.',
      supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
      reasonCode: 'preview-waiting',
      latestCapture: createCaptureRecord(),
    }),
    ...overrides,
  }
}

function SessionStateProbe({
  onChange,
}: {
  onChange(state: SessionStateContextValue): void
}) {
  const state = useSessionState()

  useEffect(() => {
    onChange(state)
  }, [onChange, state])

  return null
}

describe('SessionProvider', () => {
  it('ignores stale preset catalog failures after the active session changes', async () => {
    let resolveCatalog!: (value: PresetCatalogResult) => void
    let latestState: SessionStateContextValue | null = null

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValueOnce(
        createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
      )
      .mockResolvedValueOnce(
        createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          'Lee 1234',
        ),
      )
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveCatalog = resolve
          }),
      )

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        presetCatalogService={createPresetCatalogService({
          gateway: {
            loadPresetCatalog,
          },
        })}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    const staleCatalogRequest = latestState!
      .loadPresetCatalog({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      })
      .catch((error) => error)

    await act(async () => {
      await latestState!.startSession({
        name: 'Lee',
        phoneLastFour: '1234',
      })
    })

    expect(latestState!.sessionDraft.sessionId).toBe(
      'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
    )
    expect(latestState!.sessionDraft.presetCatalogState).toBe('idle')

    await act(async () => {
      resolveCatalog(
        createPresetCatalogResult('session_01hs6n1r8b8zc5v4ey2x7b9g1m'),
      )

      await expect(staleCatalogRequest).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(
        'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      )
      expect(latestState!.sessionDraft.presetCatalogState).toBe('idle')
      expect(latestState!.sessionDraft.presetCatalog).toHaveLength(0)
    })
  })

  it('allows the next session to load its preset catalog while the previous session request is still pending', async () => {
    let resolveFirstCatalog!: (value: PresetCatalogResult) => void
    let latestState: SessionStateContextValue | null = null

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValueOnce(
        createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
      )
      .mockResolvedValueOnce(
        createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          'Lee 1234',
        ),
      )
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirstCatalog = resolve
          }),
      )
      .mockResolvedValueOnce(
        createPresetCatalogResult('session_01hs6n1r8b8zc5v4ey2x7b9g1n'),
      )

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        presetCatalogService={createPresetCatalogService({
          gateway: {
            loadPresetCatalog,
          },
        })}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    const staleCatalogRequest = latestState!
      .loadPresetCatalog({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      })
      .catch((error) => error)

    await act(async () => {
      await latestState!.startSession({
        name: 'Lee',
        phoneLastFour: '1234',
      })
    })

    await expect(
      latestState!.loadPresetCatalog({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      }),
    ).resolves.toMatchObject({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(
        'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      )
      expect(latestState!.sessionDraft.presetCatalogState).toBe('ready')
      expect(latestState!.sessionDraft.presetCatalog).toHaveLength(1)
    })

    await act(async () => {
      resolveFirstCatalog(
        createPresetCatalogResult('session_01hs6n1r8b8zc5v4ey2x7b9g1m'),
      )

      await expect(staleCatalogRequest).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })
  })

  it('ignores stale capture readiness responses after the active session changes', async () => {
    let resolveReadiness!: (value: CaptureReadinessSnapshot) => void
    let latestState: SessionStateContextValue | null = null

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValueOnce({
        ...createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
        manifest: {
          ...createSessionStartResult(
            'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            'Kim 4821',
          ).manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })
      .mockResolvedValueOnce(
        createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          'Lee 1234',
        ),
      )

    const getCaptureReadiness = vi
      .fn<CaptureRuntimeGateway['getCaptureReadiness']>()
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveReadiness = resolve
          }),
      )

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={createCaptureRuntimeService({
          gateway: {
            getCaptureReadiness,
            requestCapture: vi.fn(),
            subscribeToCaptureReadiness: vi
              .fn<CaptureRuntimeGateway['subscribeToCaptureReadiness']>()
              .mockResolvedValue(() => undefined),
          },
        })}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    const staleReadinessRequest = latestState!
      .getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      })
      .catch((error) => error)

    await act(async () => {
      await latestState!.startSession({
        name: 'Lee',
        phoneLastFour: '1234',
      })
    })

    await act(async () => {
      resolveReadiness(createReadinessSnapshot())

      await expect(staleReadinessRequest).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(
        'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      )
      expect(latestState!.sessionDraft.flowStep).toBe('preset-selection')
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'choose-preset',
      })
    })
  })

  it('clears a stale capture lock before a new session starts', async () => {
    let resolveFirstCapture!: (value: CaptureRequestResult) => void
    let latestState: SessionStateContextValue | null = null

    const firstSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const secondSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1n'
    const activePreset = {
      presetId: 'preset_soft-glow',
      publishedVersion: '2026.03.20',
    }
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValueOnce({
        ...createSessionStartResult(firstSessionId, 'Kim 4821'),
        manifest: {
          ...createSessionStartResult(firstSessionId, 'Kim 4821').manifest,
          activePreset,
        },
      })
      .mockResolvedValueOnce({
        ...createSessionStartResult(secondSessionId, 'Lee 1234'),
        manifest: {
          ...createSessionStartResult(secondSessionId, 'Lee 1234').manifest,
          activePreset,
        },
      })
    const requestCapture = vi
      .fn<CaptureRuntimeService['requestCapture']>()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirstCapture = resolve
          }),
      )
      .mockResolvedValueOnce(
        createCaptureRequestResult({
          sessionId: secondSessionId,
          capture: createCaptureRecord({
            sessionId: secondSessionId,
            boothAlias: 'Lee 1234',
            captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1n',
            requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1n',
          }),
          readiness: createReadinessSnapshot({
            sessionId: secondSessionId,
            surfaceState: 'captureSaved',
            customerState: 'Preview Waiting',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진이 안전하게 저장되었어요.',
            supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
            reasonCode: 'preview-waiting',
            latestCapture: createCaptureRecord({
              sessionId: secondSessionId,
              boothAlias: 'Lee 1234',
              captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1n',
              requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1n',
            }),
          }),
        }),
      )
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockImplementation(async ({ sessionId }) =>
          createReadinessSnapshot({ sessionId }),
        ),
      requestCapture,
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockResolvedValue(() => undefined),
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    const staleCaptureRequest = latestState!
      .requestCapture({
        sessionId: firstSessionId,
      })
      .catch((error) => error)

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(true)
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Lee',
        phoneLastFour: '1234',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(secondSessionId)
      expect(latestState!.isRequestingCapture).toBe(false)
    })

    await expect(
      latestState!.requestCapture({
        sessionId: secondSessionId,
      }),
    ).resolves.toMatchObject({
      sessionId: secondSessionId,
      status: 'capture-saved',
    })

    expect(requestCapture).toHaveBeenNthCalledWith(2, {
      sessionId: secondSessionId,
    })

    await act(async () => {
      resolveFirstCapture(
        createCaptureRequestResult({
          sessionId: firstSessionId,
          capture: createCaptureRecord({
            sessionId: firstSessionId,
          }),
          readiness: createReadinessSnapshot({
            sessionId: firstSessionId,
            surfaceState: 'captureSaved',
            customerState: 'Preview Waiting',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진이 안전하게 저장되었어요.',
            supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
            reasonCode: 'preview-waiting',
            latestCapture: createCaptureRecord({
              sessionId: firstSessionId,
            }),
          }),
        }),
      )

      await expect(staleCaptureRequest).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })
  })

  it('invalidates older readiness work after the same session resets back to preset selection', async () => {
    let resolveReadiness!: (value: CaptureReadinessSnapshot) => void
    let latestState: SessionStateContextValue | null = null

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
        manifest: {
          ...createSessionStartResult(
            'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            'Kim 4821',
          ).manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })

    const captureRuntimeService = createCaptureRuntimeService({
      gateway: {
        getCaptureReadiness: vi
          .fn<CaptureRuntimeGateway['getCaptureReadiness']>()
          .mockImplementation(
            () =>
              new Promise((resolve) => {
                resolveReadiness = resolve
              }),
          ),
        requestCapture: vi
          .fn<CaptureRuntimeGateway['requestCapture']>()
          .mockRejectedValue({
            code: 'capture-not-ready',
            message: '촬영 전에 룩을 다시 골라 주세요.',
            readiness: {
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
              customerState: 'Preparing',
              canCapture: false,
              primaryAction: 'choose-preset',
              customerMessage: '촬영 전에 룩을 다시 골라 주세요.',
              supportMessage: '선택이 끝나면 바로 찍을 수 있어요.',
              reasonCode: 'preset-missing',
            },
          }),
        subscribeToCaptureReadiness: vi
          .fn<CaptureRuntimeGateway['subscribeToCaptureReadiness']>()
          .mockResolvedValue(() => undefined),
      },
    })

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    const staleReadinessRequest = latestState!
      .getCaptureReadiness({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      })
      .catch((error) => error)

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        }),
      ).rejects.toMatchObject({
        code: 'capture-not-ready',
      })
    })

    expect(latestState!.sessionDraft.flowStep).toBe('preset-selection')
    expect(latestState!.sessionDraft.selectedPreset).toBeNull()

    await act(async () => {
      resolveReadiness(createReadinessSnapshot())

      await expect(staleReadinessRequest).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('preset-selection')
      expect(latestState!.sessionDraft.selectedPreset).toBeNull()
      expect(latestState!.sessionDraft.captureReadiness).toBeNull()
    })
  })

  it('applies customer-safe blocked readiness when capture-not-ready arrives without readiness payload', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(sessionId, 'Kim 4821'),
        manifest: {
          ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })

    const captureRuntimeService = createCaptureRuntimeService({
      gateway: {
        getCaptureReadiness: vi
          .fn<CaptureRuntimeGateway['getCaptureReadiness']>()
          .mockResolvedValue({
            sessionId,
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '버튼을 누르면 바로 시작돼요.',
            reasonCode: 'ready',
          }),
        requestCapture: vi
          .fn<CaptureRuntimeGateway['requestCapture']>()
          .mockRejectedValue({
            code: 'capture-not-ready',
            message: 'camera helper busy',
          }),
        subscribeToCaptureReadiness: vi
          .fn<CaptureRuntimeGateway['subscribeToCaptureReadiness']>()
          .mockResolvedValue(() => undefined),
      },
    })

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'capture-not-ready',
        readiness: {
          primaryAction: 'call-support',
          customerState: 'Phone Required',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'call-support',
        customerState: 'Phone Required',
        canCapture: false,
      })
    })
  })

  it('keeps the latest subscribed blocked readiness when the initial readiness request resolves late', async () => {
    let resolveReadiness!: (value: CaptureReadinessSnapshot) => void
    let emitReadiness:
      | ((readiness: CaptureReadinessSnapshot) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
        manifest: {
          ...createSessionStartResult(
            'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            'Kim 4821',
          ).manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })

    const getCaptureReadiness = vi
      .fn<CaptureRuntimeService['getCaptureReadiness']>()
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveReadiness = resolve
          }),
      )
    const subscribeToCaptureReadiness = vi
      .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
      .mockImplementation(async ({ onReadiness }) => {
        emitReadiness = onReadiness as typeof emitReadiness
        return () => {
          emitReadiness = null
        }
      })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness,
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
      subscribeToCaptureReadiness,
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(getCaptureReadiness).toHaveBeenCalledWith({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      })
      expect(subscribeToCaptureReadiness).toHaveBeenCalledTimes(1)
      expect(emitReadiness).not.toBeNull()
    })

    await act(async () => {
      emitReadiness?.(createReadinessSnapshot({
        surfaceState: 'blocked',
        customerState: 'Phone Required',
        canCapture: false,
        primaryAction: 'call-support',
        customerMessage: '지금은 도움이 필요해요.',
        supportMessage: '가까운 직원에게 알려 주세요.',
        reasonCode: 'phone-required',
      }))
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'call-support',
        canCapture: false,
        reasonCode: 'phone-required',
      })
    })

    await act(async () => {
      resolveReadiness(createReadinessSnapshot())
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'call-support',
        canCapture: false,
        reasonCode: 'phone-required',
      })
    })
  })

  it('retries capture preset hydration once after a transient failure and then stops retrying', async () => {
    try {
      let latestState: SessionStateContextValue | null = null

      const startSession = vi
        .fn<StartSessionGateway['startSession']>()
        .mockResolvedValue({
          ...createSessionStartResult(
            'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            'Kim 4821',
          ),
          manifest: {
            ...createSessionStartResult(
              'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
              'Kim 4821',
            ).manifest,
            activePreset: {
              presetId: 'preset_soft-glow',
              publishedVersion: '2026.03.20',
            },
          },
        })
      const loadPresetCatalog = vi
        .fn<PresetCatalogGateway['loadPresetCatalog']>()
        .mockRejectedValue({
          code: 'preset-catalog-unavailable',
          message: '지금은 프리셋을 불러올 수 없어요.',
        })

      render(
        <SessionProvider
          sessionService={createStartSessionService({
            gateway: {
              startSession,
            },
          })}
          presetCatalogService={createPresetCatalogService({
            gateway: {
              loadPresetCatalog,
            },
          })}
          captureRuntimeService={{
            getCaptureReadiness: vi
              .fn<CaptureRuntimeService['getCaptureReadiness']>()
              .mockResolvedValue(createReadinessSnapshot()),
            requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
            subscribeToCaptureReadiness: vi
              .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
              .mockResolvedValue(() => undefined),
          }}
        >
          <SessionStateProbe
            onChange={(state) => {
              latestState = state
            }}
          />
        </SessionProvider>,
      )

      await waitFor(() => {
        expect(latestState).not.toBeNull()
      })

      vi.useFakeTimers()

      await act(async () => {
        await latestState!.startSession({
          name: 'Kim',
          phoneLastFour: '4821',
        })
        await Promise.resolve()
      })

      expect(loadPresetCatalog).toHaveBeenCalledTimes(1)
      expect(latestState!.sessionDraft.presetCatalogState).toBe('error')

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1500)
      })

      expect(loadPresetCatalog).toHaveBeenCalledTimes(2)

      await act(async () => {
        await vi.advanceTimersByTimeAsync(6000)
      })

      expect(loadPresetCatalog).toHaveBeenCalledTimes(2)
    } finally {
      vi.useRealTimers()
    }
  })

  it('invalidates an in-flight capture request when a newer subscribed blocked readiness arrives', async () => {
    let resolveCapture!: (value: CaptureRequestResult) => void
    let emitReadiness:
      | ((readiness: CaptureReadinessSnapshot) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
        manifest: {
          ...createSessionStartResult(
            'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            'Kim 4821',
          ).manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(createReadinessSnapshot()),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockImplementation(
          () =>
            new Promise((resolve) => {
              resolveCapture = resolve
            }),
        ),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockImplementation(async ({ onReadiness }) => {
          emitReadiness = onReadiness as typeof emitReadiness
          return () => {
            emitReadiness = null
          }
        }),
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(emitReadiness).not.toBeNull()
    })

    const staleCaptureRequest = latestState!
      .requestCapture({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      })
      .catch((error) => error)

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(true)
    })

    await act(async () => {
      emitReadiness?.(createReadinessSnapshot({
        surfaceState: 'blocked',
        customerState: 'Phone Required',
        canCapture: false,
        primaryAction: 'call-support',
        customerMessage: '지금은 도움이 필요해요.',
        supportMessage: '가까운 직원에게 알려 주세요.',
        reasonCode: 'phone-required',
      }))
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'call-support',
        canCapture: false,
        reasonCode: 'phone-required',
      })
    })

    await act(async () => {
      resolveCapture(
        createCaptureRequestResult({
          readiness: createReadinessSnapshot(),
          capture: createCaptureRecord({
            renderStatus: 'previewReady',
            preview: {
              assetPath: 'fixtures/capture-preview.jpg',
              enqueuedAtMs: 100,
              readyAtMs: 180,
            },
          }),
        }),
      )

      await expect(staleCaptureRequest).resolves.toMatchObject({
        code: 'host-unavailable',
        readiness: {
          primaryAction: 'call-support',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'call-support',
        canCapture: false,
        reasonCode: 'phone-required',
      })
    })
  })

  it('keeps an in-flight capture request valid when subscribed readiness stays capture-ready', async () => {
    let resolveCapture!: (value: CaptureRequestResult) => void
    let emitReadiness:
      | ((readiness: CaptureReadinessSnapshot) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null

    const readyReadiness = createReadinessSnapshot()

    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(
          'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          'Kim 4821',
        ),
        manifest: {
          ...createSessionStartResult(
            'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            'Kim 4821',
          ).manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockImplementation(
          () =>
            new Promise((resolve) => {
              resolveCapture = resolve
            }),
        ),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockImplementation(async ({ onReadiness }) => {
          emitReadiness = onReadiness as typeof emitReadiness
          return () => {
            emitReadiness = null
          }
        }),
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(emitReadiness).not.toBeNull()
    })

    const captureRequest = latestState!.requestCapture({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(true)
    })

    await act(async () => {
      emitReadiness?.(readyReadiness)
    })

    expect(latestState!.isRequestingCapture).toBe(true)

    await act(async () => {
      resolveCapture(
        createCaptureRequestResult({
          readiness: readyReadiness,
          capture: createCaptureRecord({
            renderStatus: 'previewReady',
            preview: {
              assetPath: 'fixtures/capture-preview.jpg',
              enqueuedAtMs: 100,
              readyAtMs: 180,
            },
          }),
        }),
      )

      await expect(captureRequest).resolves.toMatchObject({
        status: 'capture-saved',
      })
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'capture',
        canCapture: true,
        reasonCode: 'ready',
      })
    })
  })

  it('ignores subscribed readiness updates that belong to a different session', async () => {
    let emitReadiness:
      | ((readiness: CaptureReadinessSnapshot) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(sessionId, 'Kim 4821'),
        manifest: {
          ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(createReadinessSnapshot({ sessionId })),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockImplementation(async ({ onReadiness }) => {
          emitReadiness = onReadiness as typeof emitReadiness
          return () => {
            emitReadiness = null
          }
        }),
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(emitReadiness).not.toBeNull()
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
      })
    })

    await act(async () => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          surfaceState: 'blocked',
          customerState: 'Phone Required',
          canCapture: false,
          primaryAction: 'call-support',
          customerMessage: '지금은 도움이 필요해요.',
          supportMessage: '가까운 직원에게 알려 주세요.',
          reasonCode: 'phone-required',
        }),
      )
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
        canCapture: true,
      })
    })
  })

  it('ignores error readiness payloads that belong to a different session', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(sessionId, 'Kim 4821'),
        manifest: {
          ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(createReadinessSnapshot({ sessionId })),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockRejectedValue({
          code: 'capture-not-ready',
          message: '지금은 도움이 필요해요.',
          readiness: createReadinessSnapshot({
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            surfaceState: 'blocked',
            customerState: 'Phone Required',
            canCapture: false,
            primaryAction: 'call-support',
            customerMessage: '지금은 도움이 필요해요.',
            supportMessage: '가까운 직원에게 알려 주세요.',
            reasonCode: 'phone-required',
          }),
        }),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockResolvedValue(() => undefined),
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
      })
    })

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'capture-not-ready',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
        canCapture: true,
      })
    })
  })

  it('starts a fresh preset catalog request after capture resets back to preset selection', async () => {
    let resolveHydrationCatalog!: (value: PresetCatalogResult) => void
    let resolveReloadCatalog!: (value: PresetCatalogResult) => void
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(sessionId, 'Kim 4821'),
        manifest: {
          ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
        },
      })
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveHydrationCatalog = resolve
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveReloadCatalog = resolve
          }),
      )
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(createReadinessSnapshot()),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockRejectedValue({
          code: 'preset-not-available',
          message: '촬영 전에 룩을 다시 골라 주세요.',
          readiness: {
            sessionId,
            customerState: 'Preparing',
            canCapture: false,
            primaryAction: 'choose-preset',
            customerMessage: '촬영 전에 룩을 다시 골라 주세요.',
            supportMessage: '선택이 끝나면 바로 찍을 수 있어요.',
            reasonCode: 'preset-missing',
          },
        }),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockResolvedValue(() => undefined),
    }

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        presetCatalogService={createPresetCatalogService({
          gateway: {
            loadPresetCatalog,
          },
        })}
        captureRuntimeService={captureRuntimeService}
      >
        <SessionStateProbe
          onChange={(state) => {
            latestState = state
          }}
        />
      </SessionProvider>,
    )

    await waitFor(() => {
      expect(latestState).not.toBeNull()
    })

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(loadPresetCatalog).toHaveBeenCalledTimes(1)
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
    })

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'preset-not-available',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('preset-selection')
      expect(latestState!.sessionDraft.presetCatalogState).toBe('idle')
    })

    const reloadCatalogRequest = latestState!.loadPresetCatalog({
      sessionId,
    })

    expect(loadPresetCatalog).toHaveBeenCalledTimes(2)

    await act(async () => {
      resolveReloadCatalog(createPresetCatalogResult(sessionId))

      await expect(reloadCatalogRequest).resolves.toMatchObject({
        sessionId,
        state: 'ready',
      })
    })

    await act(async () => {
      resolveHydrationCatalog({
        sessionId,
        state: 'ready',
        presets: [
          {
            presetId: 'preset_vintage',
            displayName: 'Vintage',
            publishedVersion: '2026.03.21',
            boothStatus: 'booth-safe',
            preview: {
              kind: 'preview-tile',
              assetPath: 'fixtures/vintage.jpg',
              altText: 'Vintage sample portrait',
            },
          },
        ],
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.presetCatalogState).toBe('ready')
      expect(latestState!.sessionDraft.presetCatalog).toMatchObject([
        {
          presetId: 'preset_soft-glow',
          displayName: 'Soft Glow',
          publishedVersion: '2026.03.20',
        },
      ])
    })
  })
})
