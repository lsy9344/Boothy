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
  CaptureDeleteResult,
  CaptureReadinessSnapshot,
  CaptureRequestResult,
  PresetCatalogResult,
  SessionCaptureRecord,
  SessionStartResult,
  SessionTimingSnapshot,
} from '../../shared-contracts'
import {
  createActivePresetService,
  type ActivePresetGateway,
} from '../services/active-preset'
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
    activePresetId: 'preset_soft-glow',
    activePresetVersion: '2026.03.20',
    activePresetDisplayName: 'Soft Glow',
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

function createTimingSnapshot(
  overrides: Partial<SessionTimingSnapshot> = {},
): SessionTimingSnapshot {
  return {
    schemaVersion: 'session-timing/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    adjustedEndAt: '2026-03-20T00:15:00.000Z',
    warningAt: '2026-03-20T00:10:00.000Z',
    phase: 'active',
    captureAllowed: true,
    approvedExtensionMinutes: 0,
    approvedExtensionAuditRef: null,
    warningTriggeredAt: null,
    endedTriggeredAt: null,
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
    postEnd: null,
    timing: undefined,
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

function createCaptureDeleteResult(
  overrides: Partial<CaptureDeleteResult> = {},
): CaptureDeleteResult {
  const sessionId =
    overrides.sessionId ?? 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

  return {
    schemaVersion: 'capture-delete-result/v1',
    sessionId,
    captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
    status: 'capture-deleted',
    manifest: {
      schemaVersion: 'session-manifest/v1',
      sessionId,
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
    readiness: createReadinessSnapshot({
      sessionId,
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

    expect(requestCapture).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        sessionId: secondSessionId,
        requestId: expect.any(String),
      }),
    )

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

  it('downgrades capture-not-ready without readiness payload to a transient wait state', async () => {
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
          primaryAction: 'wait',
          customerState: 'Preparing',
          reasonCode: 'camera-preparing',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'wait',
        customerState: 'Preparing',
        canCapture: false,
        reasonCode: 'camera-preparing',
      })
    })
  })

  it('applies retryable capture guidance and clears loading when the host reports a capture-start timeout', async () => {
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
          .mockResolvedValue(createReadinessSnapshot()),
        requestCapture: vi
          .fn<CaptureRuntimeGateway['requestCapture']>()
          .mockRejectedValue({
            code: 'capture-not-ready',
            message: '초점이 맞지 않았어요.',
            readiness: {
              sessionId,
              customerState: 'Preparing',
              canCapture: false,
              primaryAction: 'wait',
              customerMessage: '초점이 맞지 않았어요.',
              supportMessage: '대상을 다시 맞추는 동안 잠시 기다려 주세요.',
              reasonCode: 'capture-retry-required',
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'capture-not-ready',
        readiness: {
          primaryAction: 'wait',
          customerState: 'Preparing',
          reasonCode: 'capture-retry-required',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'wait',
        customerState: 'Preparing',
        canCapture: false,
        reasonCode: 'capture-retry-required',
      })
    })
  })

  it('keeps the latest same-session thumbnail visible when a follow-up capture request transiently fails', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const latestCapture = createCaptureRecord({
      sessionId,
      captureId: 'capture_recent',
      requestId: 'request_recent',
      renderStatus: 'previewReady',
      preview: {
        assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/capture_recent.jpg`,
        enqueuedAtMs: 100,
        readyAtMs: 500,
      },
    })
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(sessionId, 'Kim 4821'),
        manifest: {
          ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
          lifecycle: {
            status: 'active',
            stage: 'capture-ready',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          captures: [latestCapture],
        },
      })

    const captureRuntimeService = createCaptureRuntimeService({
      gateway: {
        getCaptureReadiness: vi
          .fn<CaptureRuntimeGateway['getCaptureReadiness']>()
          .mockResolvedValue(
            createReadinessSnapshot({
              sessionId,
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '방금 찍은 사진을 아래에서 바로 확인할 수 있어요.',
              reasonCode: 'ready',
              latestCapture,
            }),
          ),
        requestCapture: vi
          .fn<CaptureRuntimeGateway['requestCapture']>()
          .mockRejectedValue({
            code: 'session-persistence-failed',
            message: 'unexpected runtime bridge failure',
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

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
        canCapture: true,
        latestCapture: {
          captureId: 'capture_recent',
        },
      })
    })

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'session-persistence-failed',
        readiness: {
          customerState: 'Preparing',
          primaryAction: 'wait',
          reasonCode: 'camera-preparing',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        customerState: 'Preparing',
        primaryAction: 'wait',
        reasonCode: 'camera-preparing',
      })
      expect(latestState!.sessionDraft.manifest?.captures).toHaveLength(1)
      expect(latestState!.sessionDraft.manifest?.captures[0]?.captureId).toBe(
        'capture_recent',
      )
    })
  })

  it('keeps the current capture session when requestCapture hits a same-session session-not-found error', async () => {
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
          .mockResolvedValue(
            createReadinessSnapshot({
              sessionId,
              surfaceState: 'captureReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
            }),
          ),
        requestCapture: vi
          .fn<CaptureRuntimeGateway['requestCapture']>()
          .mockRejectedValue({
            code: 'session-not-found',
            message: 'manifest missing',
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

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
        canCapture: true,
      })
    })

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'session-not-found',
        readiness: {
          sessionId,
          customerState: 'Preparing',
          primaryAction: 'wait',
          reasonCode: 'camera-preparing',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(sessionId)
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.selectedPreset).toMatchObject({
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      })
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        customerState: 'Preparing',
        primaryAction: 'wait',
        reasonCode: 'camera-preparing',
      })
    })
  })

  it('keeps the latest subscribed preparing readiness when the initial readiness request resolves late', async () => {
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
        customerState: 'Preparing',
        canCapture: false,
        primaryAction: 'wait',
        customerMessage: '촬영 준비 중이에요.',
        supportMessage: '잠시만 기다려 주세요.',
        reasonCode: 'helper-preparing',
      }))
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'wait',
        canCapture: false,
        reasonCode: 'helper-preparing',
      })
    })

    await act(async () => {
      resolveReadiness(createReadinessSnapshot())
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        primaryAction: 'wait',
        canCapture: false,
        reasonCode: 'helper-preparing',
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

  it('does not start an extra readiness retry loop on top of the subscription channel', async () => {
    let latestState: SessionStateContextValue | null = null

    try {
      const getCaptureReadiness = vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(createReadinessSnapshot())
      const subscribeToCaptureReadiness = vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockResolvedValue(() => undefined)
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

      render(
        <SessionProvider
          sessionService={createStartSessionService({
            gateway: {
              startSession,
            },
          })}
          captureRuntimeService={{
            getCaptureReadiness,
            requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
            subscribeToCaptureReadiness,
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

      await act(async () => {
        await latestState!.startSession({
          name: 'Kim',
          phoneLastFour: '4821',
        })
        await Promise.resolve()
      })

      await waitFor(() => {
        expect(subscribeToCaptureReadiness).toHaveBeenCalledTimes(1)
        expect(getCaptureReadiness).toHaveBeenCalledTimes(1)
      })

      vi.useFakeTimers()

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1200)
      })

      expect(getCaptureReadiness).toHaveBeenCalledTimes(1)
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

  it('keeps a pending fast preview until the matching capture owns a visible preview asset and ignores stale same-session updates', async () => {
    let resolveCapture!: (value: CaptureRequestResult) => void
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    let emitFastPreview:
      | ((update: {
          schemaVersion: 'capture-fast-preview-update/v1'
          sessionId: string
          requestId: string
          captureId: string
          assetPath: string
          visibleAtMs: number
          kind?: string | null
        }) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const captureId = 'capture_fast_pending'
    let requestId: string | null = null
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
        .mockImplementation((input) => {
          requestId = input.requestId ?? null
          return new Promise((resolve) => {
            resolveCapture = resolve
          })
        }),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockImplementation(async ({ onReadiness }) => {
          emitReadiness = onReadiness as typeof emitReadiness
          return () => {
            emitReadiness = null
          }
        }),
      subscribeToCaptureFastPreview: vi
        .fn<NonNullable<CaptureRuntimeService['subscribeToCaptureFastPreview']>>()
        .mockImplementation(async ({ onFastPreview }) => {
          emitFastPreview = onFastPreview as typeof emitFastPreview
          return () => {
            emitFastPreview = null
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

    const captureRequest = latestState!.requestCapture({
      sessionId,
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(true)
      expect(emitFastPreview).not.toBeNull()
      expect(requestId).not.toBeNull()
    })

    await act(async () => {
      emitFastPreview?.({
        schemaVersion: 'capture-fast-preview-update/v1',
        sessionId,
        requestId: 'request_old_same_session',
        captureId: 'capture_old_same_session',
        assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/capture_old_same_session.jpg`,
        visibleAtMs: 180,
        kind: 'camera-thumbnail',
      })
    })

    expect(latestState!.sessionDraft.pendingFastPreview).toBeNull()

    await act(async () => {
      emitFastPreview?.({
        schemaVersion: 'capture-fast-preview-update/v1',
        sessionId,
        requestId: requestId!,
        captureId,
        assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${captureId}.jpg`,
        visibleAtMs: 240,
        kind: 'camera-thumbnail',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.pendingFastPreview).toMatchObject({
        sessionId,
        requestId: requestId!,
        captureId,
      })
    })

    await act(async () => {
      resolveCapture(
        createCaptureRequestResult({
          sessionId,
          capture: createCaptureRecord({
            sessionId,
            requestId: requestId!,
            captureId,
            preview: {
              assetPath: null,
              enqueuedAtMs: 100,
              readyAtMs: null,
            },
            renderStatus: 'previewWaiting',
          }),
          readiness: createReadinessSnapshot({
            sessionId,
            surfaceState: 'captureSaved',
            customerState: 'Preview Waiting',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진이 안전하게 저장되었어요.',
            supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
            reasonCode: 'preview-waiting',
            latestCapture: createCaptureRecord({
              sessionId,
              requestId: requestId!,
              captureId,
              preview: {
                assetPath: null,
                enqueuedAtMs: 100,
                readyAtMs: null,
              },
              renderStatus: 'previewWaiting',
            }),
          }),
        }),
      )

      await expect(captureRequest).resolves.toMatchObject({
        status: 'capture-saved',
      })
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.pendingFastPreview).toMatchObject({
        sessionId,
        requestId: requestId!,
        captureId,
      })
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'preview-waiting',
      })
    })

    await act(async () => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'previewWaiting',
          customerState: 'Preview Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '사진이 안전하게 저장되었어요.',
          supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
          reasonCode: 'preview-waiting',
          latestCapture: createCaptureRecord({
            sessionId,
            requestId: requestId!,
            captureId,
            preview: {
              assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${captureId}.jpg`,
              enqueuedAtMs: 100,
              readyAtMs: null,
            },
            renderStatus: 'previewWaiting',
          }),
        }),
      )
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.pendingFastPreview).toBeNull()
    })
  })

  it('keeps capture-ready surface state when a visible same-capture fast preview exists before final preview-ready', async () => {
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const captureId = 'capture_fast_resume'
    const requestId = 'request_000000000000064e897e16ffe8'
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
        .mockResolvedValue(
          createReadinessSnapshot({
            sessionId,
            surfaceState: 'captureSaved',
            customerState: 'Preview Waiting',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진이 안전하게 저장되었어요.',
            supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
            reasonCode: 'preview-waiting',
            latestCapture: createCaptureRecord({
              sessionId,
              captureId,
              requestId,
              renderStatus: 'previewWaiting',
              preview: {
                assetPath: null,
                enqueuedAtMs: 100,
                readyAtMs: null,
              },
            }),
          }),
        ),
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
    })

    await act(async () => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'captureReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
          latestCapture: createCaptureRecord({
            sessionId,
            captureId,
            requestId,
            renderStatus: 'previewWaiting',
            preview: {
              assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${captureId}.jpg`,
              enqueuedAtMs: 100,
              readyAtMs: null,
            },
          }),
        }),
      )
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        surfaceState: 'captureReady',
        reasonCode: 'ready',
        canCapture: true,
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

  it('invalidates an in-flight capture request when subscribed readiness is sanitized to preview-waiting', async () => {
    let resolveCapture!: (value: CaptureRequestResult) => void
    let emitReadiness:
      | ((readiness: CaptureReadinessSnapshot) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const readyReadiness = createReadinessSnapshot({ sessionId })

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

    const staleCaptureRequest = latestState!
      .requestCapture({
        sessionId,
      })
      .catch((error) => error)

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(true)
    })

    await act(async () => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'previewReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
          latestCapture: createCaptureRecord({
            sessionId,
            captureId: 'capture_scrubbed_preview',
            renderStatus: 'previewReady',
            preview: {
              assetPath:
                'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\foreign.jpg',
              enqueuedAtMs: 120,
              readyAtMs: 450,
            },
          }),
        }),
      )
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        primaryAction: 'wait',
        canCapture: false,
        reasonCode: 'preview-waiting',
        latestCapture: {
          captureId: 'capture_scrubbed_preview',
          renderStatus: 'captureSaved',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      })
    })

    await act(async () => {
      resolveCapture(
        createCaptureRequestResult({
          sessionId,
          readiness: readyReadiness,
          capture: createCaptureRecord({
            sessionId,
            captureId: 'capture_scrubbed_preview',
            renderStatus: 'previewReady',
            preview: {
              assetPath:
                'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current.jpg',
              enqueuedAtMs: 120,
              readyAtMs: 450,
            },
          }),
        }),
      )

      await expect(staleCaptureRequest).resolves.toMatchObject({
        code: 'host-unavailable',
        readiness: {
          primaryAction: 'wait',
          reasonCode: 'preview-waiting',
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.isRequestingCapture).toBe(false)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        primaryAction: 'wait',
        canCapture: false,
        reasonCode: 'preview-waiting',
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

  it('does not merge a latest capture into the manifest when the capture belongs to a different session', async () => {
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
    })

    await act(async () => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'previewReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
          latestCapture: createCaptureRecord({
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            boothAlias: 'Lee 1234',
            captureId: 'capture_other_session',
            preview: {
              assetPath: 'fixtures/other-session.jpg',
              enqueuedAtMs: 120,
              readyAtMs: 450,
            },
            renderStatus: 'previewReady',
          }),
        }),
      )
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.manifest?.captures).toHaveLength(0)
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
        latestCapture: null,
      })
    })
  })

  it('preserves a same-session capture record while scrubbing an out-of-scope preview path', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId: 'capture_foreign_asset_path',
              renderStatus: 'previewReady',
              preview: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\foreign.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
            }),
            readiness: createReadinessSnapshot({
              sessionId,
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId,
                captureId: 'capture_foreign_asset_path',
                renderStatus: 'previewReady',
                preview: {
                  assetPath:
                    'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\foreign.jpg',
                  enqueuedAtMs: 120,
                  readyAtMs: 450,
                },
              }),
            }),
          }),
        ),
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        capture: {
          captureId: 'capture_foreign_asset_path',
          renderStatus: 'captureSaved',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
        readiness: {
          sessionId,
          surfaceState: 'captureSaved',
          reasonCode: 'preview-waiting',
          primaryAction: 'wait',
          latestCapture: {
            captureId: 'capture_foreign_asset_path',
            renderStatus: 'captureSaved',
            preview: {
              assetPath: null,
              readyAtMs: null,
            },
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.manifest?.captures).toMatchObject([
        {
          captureId: 'capture_foreign_asset_path',
          renderStatus: 'captureSaved',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      ])
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        surfaceState: 'captureSaved',
        reasonCode: 'preview-waiting',
        primaryAction: 'wait',
        latestCapture: {
          captureId: 'capture_foreign_asset_path',
          renderStatus: 'captureSaved',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      })
    })
  })

  it('scrubs a foreign final asset path and downgrades final-ready captures before they reach state or payloads', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId: 'capture_foreign_final_asset_path',
              renderStatus: 'finalReady',
              preview: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
              final: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\finals\\foreign.jpg',
                readyAtMs: 480,
              },
            }),
            readiness: createReadinessSnapshot({
              sessionId,
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId,
                captureId: 'capture_foreign_final_asset_path',
                renderStatus: 'finalReady',
                preview: {
                  assetPath:
                    'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current.jpg',
                  enqueuedAtMs: 120,
                  readyAtMs: 450,
                },
                final: {
                  assetPath:
                    'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\finals\\foreign.jpg',
                  readyAtMs: 480,
                },
              }),
            }),
          }),
        ),
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        capture: {
          captureId: 'capture_foreign_final_asset_path',
          renderStatus: 'previewReady',
          final: {
            assetPath: null,
            readyAtMs: null,
          },
        },
        readiness: {
          sessionId,
          latestCapture: {
            captureId: 'capture_foreign_final_asset_path',
            renderStatus: 'previewReady',
            final: {
              assetPath: null,
              readyAtMs: null,
            },
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.manifest?.captures).toMatchObject([
        {
          captureId: 'capture_foreign_final_asset_path',
          renderStatus: 'previewReady',
          final: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      ])
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        latestCapture: {
          captureId: 'capture_foreign_final_asset_path',
          renderStatus: 'previewReady',
          final: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      })
    })
  })

  it('preserves a same-session final-ready latestCapture when only its preview path is scrubbed', async () => {
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
        .mockResolvedValue(
          createReadinessSnapshot({
            sessionId,
            surfaceState: 'previewReady',
            latestCapture: createCaptureRecord({
              sessionId,
              captureId: 'capture_final_ready_preview_scrubbed',
              renderStatus: 'finalReady',
              preview: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\foreign.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
              final: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\finals\\current.jpg',
                readyAtMs: 520,
              },
            }),
          }),
        ),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
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

    await act(async () => {
      await expect(
        latestState!.getCaptureReadiness({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        surfaceState: 'captureReady',
        latestCapture: {
          captureId: 'capture_final_ready_preview_scrubbed',
          renderStatus: 'finalReady',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
          final: {
            assetPath:
              'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\finals\\current.jpg',
            readyAtMs: 520,
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        surfaceState: 'captureReady',
        latestCapture: {
          captureId: 'capture_final_ready_preview_scrubbed',
          renderStatus: 'finalReady',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
          final: {
            assetPath:
              'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\finals\\current.jpg',
            readyAtMs: 520,
          },
        },
      })
    })
  })

  it('preserves same-session blocked readiness when an unsafe preview path is scrubbed from latestCapture', async () => {
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
        .mockResolvedValue(
          createReadinessSnapshot({
            sessionId,
            surfaceState: 'blocked',
            customerState: 'Phone Required',
            canCapture: false,
            primaryAction: 'call-support',
            customerMessage: '지금은 도움이 필요해요.',
            supportMessage: '가까운 직원에게 알려 주세요.',
            reasonCode: 'phone-required',
            latestCapture: createCaptureRecord({
              sessionId,
              captureId: 'capture_blocked_with_foreign_preview',
              renderStatus: 'previewReady',
              preview: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\foreign.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
            }),
          }),
        ),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
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

    await act(async () => {
      await expect(
        latestState!.getCaptureReadiness({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        surfaceState: 'blocked',
        reasonCode: 'phone-required',
        primaryAction: 'call-support',
        canCapture: false,
        latestCapture: {
          captureId: 'capture_blocked_with_foreign_preview',
          renderStatus: 'captureSaved',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        surfaceState: 'blocked',
        reasonCode: 'phone-required',
        primaryAction: 'call-support',
        canCapture: false,
        latestCapture: {
          captureId: 'capture_blocked_with_foreign_preview',
          renderStatus: 'captureSaved',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
      })
    })
  })

  it('preserves a saved capture when requestCapture returns readiness for a different session', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId: 'capture_current_session',
            }),
            readiness: createReadinessSnapshot({
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
                boothAlias: 'Lee 1234',
                captureId: 'capture_foreign_session',
                preview: {
                  assetPath: 'fixtures/other-session.jpg',
                  enqueuedAtMs: 120,
                  readyAtMs: 450,
                },
                renderStatus: 'previewReady',
              }),
            }),
          }),
        ),
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
        latestCapture: null,
      })
    })

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        status: 'capture-saved',
        capture: {
          captureId: 'capture_current_session',
        },
        readiness: {
          sessionId,
          reasonCode: 'preview-waiting',
          primaryAction: 'wait',
          latestCapture: {
            captureId: 'capture_current_session',
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.manifest?.captures).toMatchObject([
        {
          captureId: 'capture_current_session',
        },
      ])
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'preview-waiting',
        primaryAction: 'wait',
        latestCapture: {
          captureId: 'capture_current_session',
        },
      })
    })
  })

  it('returns sanitized readiness from getCaptureReadiness when latestCapture belongs to a different session', async () => {
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
        .mockResolvedValue(
          createReadinessSnapshot({
            sessionId,
            surfaceState: 'previewReady',
            latestCapture: createCaptureRecord({
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              boothAlias: 'Lee 1234',
              captureId: 'capture_other_session',
              preview: {
                assetPath: 'fixtures/other-session.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
              renderStatus: 'previewReady',
            }),
          }),
        ),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
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

    await act(async () => {
      await expect(
        latestState!.getCaptureReadiness({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        latestCapture: null,
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        latestCapture: null,
      })
    })
  })

  it('returns sanitized readiness from requestCapture when latestCapture belongs to a different session', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId: 'capture_current_session',
            }),
            readiness: createReadinessSnapshot({
              sessionId,
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
                boothAlias: 'Lee 1234',
                captureId: 'capture_foreign_session',
                preview: {
                  assetPath: 'fixtures/other-session.jpg',
                  enqueuedAtMs: 120,
                  readyAtMs: 450,
                },
                renderStatus: 'previewReady',
              }),
            }),
          }),
        ),
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        readiness: {
          sessionId,
          latestCapture: null,
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        latestCapture: null,
      })
    })
  })

  it('merges a preview-ready latestCapture into the manifest when requestCapture returns a richer record for the same capture', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const captureId = 'capture_current_session'
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId,
              renderStatus: 'previewWaiting',
              preview: {
                assetPath: null,
                enqueuedAtMs: 100,
                readyAtMs: null,
              },
            }),
            readiness: createReadinessSnapshot({
              sessionId,
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId,
                captureId,
                renderStatus: 'previewReady',
                preview: {
                  assetPath: 'fixtures/capture-preview-ready.jpg',
                  enqueuedAtMs: 100,
                  readyAtMs: 180,
                },
              }),
            }),
          }),
        ),
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        readiness: {
          latestCapture: {
            captureId,
            renderStatus: 'previewReady',
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.manifest?.captures).toMatchObject([
        {
          captureId,
          renderStatus: 'previewReady',
          preview: {
            assetPath: 'fixtures/capture-preview-ready.jpg',
            readyAtMs: 180,
          },
        },
      ])
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

  it('does not reset the current session when getCaptureReadiness fails with a foreign session-not-found error', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const foreignSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1n'
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
        .mockResolvedValueOnce(createReadinessSnapshot({ sessionId }))
        .mockRejectedValueOnce({
          code: 'session-not-found',
          message: '현재 세션을 다시 불러올게요.',
          readiness: createReadinessSnapshot({
            sessionId: foreignSessionId,
            surfaceState: 'blocked',
            customerState: 'Preparing',
            canCapture: false,
            primaryAction: 'choose-preset',
            customerMessage: '촬영 전에 룩을 다시 골라 주세요.',
            supportMessage: '선택이 끝나면 바로 찍을 수 있어요.',
            reasonCode: 'preset-missing',
          }),
        }),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
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
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
      })
    })

    await act(async () => {
      await expect(
        latestState!.getCaptureReadiness({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'session-not-found',
        readiness: undefined,
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(sessionId)
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
      })
    })
  })

  it('resets to session start when getCaptureReadiness fails with a same-session session-not-found error without readiness', async () => {
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
        .mockResolvedValueOnce(createReadinessSnapshot({ sessionId }))
        .mockRejectedValueOnce({
          code: 'session-not-found',
          message: '세션을 다시 시작해야 해요.',
        }),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
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
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
    })

    await act(async () => {
      await expect(
        latestState!.getCaptureReadiness({
          sessionId,
        }),
      ).rejects.toMatchObject({
        code: 'session-not-found',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft).toMatchObject({
        flowStep: 'session-start',
        sessionId: null,
        selectedPreset: null,
      })
    })
  })

  it('does not reset the current session when requestCapture fails with a foreign preset-not-available error', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const foreignSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1n'
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockRejectedValue({
          code: 'preset-not-available',
          message: '촬영 전에 룩을 다시 골라 주세요.',
          readiness: createReadinessSnapshot({
            sessionId: foreignSessionId,
            surfaceState: 'blocked',
            customerState: 'Preparing',
            canCapture: false,
            primaryAction: 'choose-preset',
            customerMessage: '촬영 전에 룩을 다시 골라 주세요.',
            supportMessage: '선택이 끝나면 바로 찍을 수 있어요.',
            reasonCode: 'preset-missing',
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
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
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
        code: 'preset-not-available',
        readiness: undefined,
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.sessionId).toBe(sessionId)
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
      })
    })
  })

  it('resets to preset selection when requestCapture fails with a same-session preset-not-available error without readiness', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockRejectedValue({
          code: 'preset-not-available',
          message: '촬영 전에 룩을 다시 골라 주세요.',
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
      expect(latestState!.sessionDraft.selectedPreset).toBeNull()
    })
  })

  it('returns sanitized fallback readiness when requestCapture preserves a capture but its preview asset path is out of session scope', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId: 'capture_foreign_asset_fallback',
              renderStatus: 'previewReady',
              preview: {
                assetPath:
                  'sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1n/renders/previews/other-session.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
            }),
            readiness: createReadinessSnapshot({
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
                captureId: 'capture_foreign_session',
                preview: {
                  assetPath: 'fixtures/other-session.jpg',
                  enqueuedAtMs: 110,
                  readyAtMs: 440,
                },
                renderStatus: 'previewReady',
              }),
            }),
          }),
        ),
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        capture: {
          captureId: 'capture_foreign_asset_fallback',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
        },
        readiness: {
          sessionId,
          latestCapture: {
            captureId: 'capture_foreign_asset_fallback',
            preview: {
              assetPath: null,
              readyAtMs: null,
            },
          },
        },
      })
    })
  })

  it('keeps requestCapture blocked when foreign readiness arrives for a same-session final-ready capture', async () => {
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
    const readyReadiness = createReadinessSnapshot({ sessionId })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(readyReadiness),
      requestCapture: vi
        .fn<CaptureRuntimeService['requestCapture']>()
        .mockResolvedValue(
          createCaptureRequestResult({
            sessionId,
            capture: createCaptureRecord({
              sessionId,
              captureId: 'capture_final_ready_foreign_fallback',
              renderStatus: 'finalReady',
              preview: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
              final: {
                assetPath:
                  'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\finals\\current.jpg',
                readyAtMs: 520,
              },
            }),
            readiness: createReadinessSnapshot({
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              surfaceState: 'previewReady',
              customerState: 'Ready',
              canCapture: true,
              primaryAction: 'capture',
              customerMessage: '지금 촬영할 수 있어요.',
              supportMessage: '버튼을 누르면 바로 시작돼요.',
              reasonCode: 'ready',
              latestCapture: createCaptureRecord({
                sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
                captureId: 'capture_foreign_session',
                preview: {
                  assetPath: 'fixtures/other-session.jpg',
                  enqueuedAtMs: 110,
                  readyAtMs: 440,
                },
                renderStatus: 'previewReady',
              }),
            }),
          }),
        ),
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

    await act(async () => {
      await expect(
        latestState!.requestCapture({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        capture: {
          captureId: 'capture_final_ready_foreign_fallback',
          renderStatus: 'finalReady',
        },
        readiness: {
          sessionId,
          surfaceState: 'blocked',
          reasonCode: 'camera-preparing',
          primaryAction: 'wait',
          canCapture: false,
          latestCapture: {
            captureId: 'capture_final_ready_foreign_fallback',
            renderStatus: 'finalReady',
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        surfaceState: 'blocked',
        reasonCode: 'camera-preparing',
        primaryAction: 'wait',
        canCapture: false,
        latestCapture: {
          captureId: 'capture_final_ready_foreign_fallback',
          renderStatus: 'finalReady',
        },
      })
    })
  })

  it('keeps the existing same-session preview in the manifest when a matching final-ready update arrives without one', async () => {
    let latestState: SessionStateContextValue | null = null

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const captureId = 'capture_preserve_existing_preview'
    const preservedPreviewPath =
      'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current.jpg'
    const finalAssetPath =
      'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\finals\\current.jpg'
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
          captures: [
            createCaptureRecord({
              sessionId,
              captureId,
              renderStatus: 'previewReady',
              preview: {
                assetPath: preservedPreviewPath,
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
            }),
          ],
        },
      })
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(
          createReadinessSnapshot({
            sessionId,
            surfaceState: 'captureReady',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '버튼을 누르면 바로 시작돼요.',
            reasonCode: 'ready',
            latestCapture: createCaptureRecord({
              sessionId,
              captureId,
              renderStatus: 'finalReady',
              preview: {
                assetPath: null,
                enqueuedAtMs: 120,
                readyAtMs: null,
              },
              final: {
                assetPath: finalAssetPath,
                readyAtMs: 520,
              },
            }),
          }),
        ),
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
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

    await act(async () => {
      await expect(
        latestState!.getCaptureReadiness({
          sessionId,
        }),
      ).resolves.toMatchObject({
        sessionId,
        latestCapture: {
          captureId,
          renderStatus: 'finalReady',
          preview: {
            assetPath: null,
            readyAtMs: null,
          },
          final: {
            assetPath: finalAssetPath,
            readyAtMs: 520,
          },
        },
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.manifest?.captures).toMatchObject([
        {
          captureId,
          renderStatus: 'finalReady',
          preview: {
            assetPath: preservedPreviewPath,
            enqueuedAtMs: 120,
            readyAtMs: 450,
          },
          final: {
            assetPath: finalAssetPath,
            readyAtMs: 520,
          },
        },
      ])
    })
  })

  it('keeps the current preset and capture state when an in-session preset switch is rejected', async () => {
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
          activePresetId: 'preset_soft-glow',
          captures: [
            createCaptureRecord({
              sessionId,
              captureId: 'capture_existing',
              renderStatus: 'previewReady',
              preview: {
                assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/capture_existing.jpg`,
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          ],
        },
      })

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        activePresetService={createActivePresetService({
          gateway: {
            selectActivePreset: vi
              .fn<ActivePresetGateway['selectActivePreset']>()
              .mockRejectedValue({
                code: 'preset-not-available',
                message:
                  '지금 고른 프리셋은 사용할 수 없어요. 다른 프리셋을 골라 주세요.',
              }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(createReadinessSnapshot({ sessionId })),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
    })

    act(() => {
      latestState!.beginPresetSwitch()
    })

    expect(latestState!.sessionDraft.flowStep).toBe('preset-selection')
    expect(latestState!.sessionDraft.selectedPreset).toMatchObject({
      presetId: 'preset_soft-glow',
      publishedVersion: '2026.03.20',
    })

    await act(async () => {
      await expect(
        latestState!.selectActivePreset({
          sessionId,
          preset: {
            presetId: 'preset_mono-pop',
            publishedVersion: '2026.03.21',
          },
        }),
      ).rejects.toMatchObject({
        code: 'preset-not-available',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.flowStep).toBe('preset-selection')
      expect(latestState!.sessionDraft.selectedPreset).toMatchObject({
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      })
      expect(latestState!.sessionDraft.captureReadiness).toMatchObject({
        sessionId,
        reasonCode: 'ready',
      })
      expect(latestState!.sessionDraft.manifest?.captures).toHaveLength(1)
    })
  })

  it('ignores a late preset switch success after the customer cancels back to the current look', async () => {
    let latestState: SessionStateContextValue | null = null
    let resolveSelection!: (value: unknown) => void

    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const startSession = vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
      ...createSessionStartResult(sessionId, 'Kim 4821'),
      manifest: {
        ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
        activePreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
        activePresetId: 'preset_soft-glow',
        activePresetDisplayName: 'Soft Glow',
        captures: [
          createCaptureRecord({
            sessionId,
            captureId: 'capture_existing',
            renderStatus: 'previewReady',
            preview: {
              assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/capture_existing.jpg`,
              enqueuedAtMs: 100,
              readyAtMs: 500,
            },
          }),
        ],
      },
    })

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        activePresetService={createActivePresetService({
          gateway: {
            selectActivePreset: vi
              .fn<ActivePresetGateway['selectActivePreset']>()
              .mockImplementation(
                () =>
                  new Promise((resolve) => {
                    resolveSelection = resolve
                  }),
              ),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(createReadinessSnapshot({ sessionId })),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    act(() => {
      latestState!.beginPresetSwitch()
    })

    const pendingSelection = latestState!
      .selectActivePreset({
        sessionId,
        preset: {
          presetId: 'preset_mono-pop',
          publishedVersion: '2026.03.21',
        },
      })
      .catch((error) => error)

    act(() => {
      latestState!.cancelPresetSwitch()
    })

    expect(latestState!.sessionDraft.flowStep).toBe('capture')
    expect(latestState!.isSelectingPreset).toBe(true)

    await act(async () => {
      resolveSelection({
        sessionId,
        activePreset: {
          presetId: 'preset_mono-pop',
          publishedVersion: '2026.03.21',
        },
        manifest: {
          ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
          activePreset: {
            presetId: 'preset_mono-pop',
            publishedVersion: '2026.03.21',
          },
          activePresetId: 'preset_mono-pop',
          activePresetDisplayName: 'Mono Pop',
          captures: [],
        },
      })

      await expect(pendingSelection).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })

    await waitFor(() => {
      expect(latestState!.isSelectingPreset).toBe(false)
      expect(latestState!.sessionDraft.flowStep).toBe('capture')
      expect(latestState!.sessionDraft.selectedPreset).toMatchObject({
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      })
      expect(latestState!.sessionDraft.manifest?.activePreset).toMatchObject({
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      })
      expect(latestState!.sessionDraft.manifest?.activePresetDisplayName).toBe(
        'Soft Glow',
      )
      expect(latestState!.sessionDraft.manifest?.captures).toHaveLength(1)
    })
  })

  it('removes a deleted capture from the current manifest and keeps the next preview as latest', async () => {
    let latestState: SessionStateContextValue | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const latestCaptureId = 'capture_latest'
    const remainingCaptureId = 'capture_remaining'
    const startSession = vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
      sessionId,
      boothAlias: 'Kim 4821',
      manifest: {
        schemaVersion: 'session-manifest/v1',
        sessionId,
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
        captures: [
          createCaptureRecord({
            sessionId,
            captureId: latestCaptureId,
            renderStatus: 'previewReady',
            preview: {
              assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${latestCaptureId}.jpg`,
              enqueuedAtMs: 200,
              readyAtMs: 800,
            },
          }),
          createCaptureRecord({
            sessionId,
            captureId: remainingCaptureId,
            renderStatus: 'previewReady',
            preview: {
              assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${remainingCaptureId}.jpg`,
              enqueuedAtMs: 100,
              readyAtMs: 500,
            },
          }),
        ],
        postEnd: null,
      },
    })
    const deleteCapture = vi
      .fn<NonNullable<CaptureRuntimeService['deleteCapture']>>()
      .mockResolvedValue(
        createCaptureDeleteResult({
          sessionId,
          captureId: latestCaptureId,
          manifest: {
            schemaVersion: 'session-manifest/v1',
            sessionId,
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
            captures: [
              createCaptureRecord({
                sessionId,
                captureId: remainingCaptureId,
                renderStatus: 'previewReady',
                preview: {
                  assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${remainingCaptureId}.jpg`,
                  enqueuedAtMs: 100,
                  readyAtMs: 500,
                },
              }),
            ],
            postEnd: null,
          },
          readiness: createReadinessSnapshot({
            sessionId,
            surfaceState: 'previewReady',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '방금 찍은 사진을 아래에서 바로 확인할 수 있어요.',
            reasonCode: 'ready',
            latestCapture: createCaptureRecord({
              sessionId,
              captureId: remainingCaptureId,
              renderStatus: 'previewReady',
              preview: {
                assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${remainingCaptureId}.jpg`,
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          }),
        }),
      )

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi.fn<CaptureRuntimeService['getCaptureReadiness']>(),
          deleteCapture,
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await act(async () => {
      await expect(
        latestState!.deleteCapture({
          sessionId,
          captureId: latestCaptureId,
        }),
      ).resolves.toMatchObject({
        captureId: latestCaptureId,
        status: 'capture-deleted',
      })
    })

    expect(latestState!.sessionDraft.manifest?.captures).toHaveLength(1)
    expect(latestState!.sessionDraft.manifest?.captures[0]?.captureId).toBe(
      remainingCaptureId,
    )
    expect(
      latestState!.sessionDraft.captureReadiness?.latestCapture?.captureId,
    ).toBe(remainingCaptureId)
  })

  it('ignores stale delete success after the active session changes', async () => {
    let resolveDelete!: (value: CaptureDeleteResult) => void
    let latestState: SessionStateContextValue | null = null
    const firstSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const secondSessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1n'
    const captureId = 'capture_latest'
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValueOnce({
        sessionId: firstSessionId,
        boothAlias: 'Kim 4821',
        manifest: {
          ...createSessionStartResult(firstSessionId, 'Kim 4821').manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          captures: [
            createCaptureRecord({
              sessionId: firstSessionId,
              captureId,
              renderStatus: 'previewReady',
              preview: {
                assetPath: `C:/Users/Example/Pictures/dabi_shoot/sessions/${firstSessionId}/renders/previews/${captureId}.jpg`,
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          ],
        },
      })
      .mockResolvedValueOnce(createSessionStartResult(secondSessionId, 'Lee 1234'))
    const deleteCapture = vi
      .fn<NonNullable<CaptureRuntimeService['deleteCapture']>>()
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveDelete = resolve
          }),
      )

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi.fn<CaptureRuntimeService['getCaptureReadiness']>(),
          deleteCapture,
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    const staleDelete = latestState!
      .deleteCapture({
        sessionId: firstSessionId,
        captureId,
      })
      .catch((error) => error)

    await act(async () => {
      await latestState!.startSession({
        name: 'Lee',
        phoneLastFour: '1234',
      })
    })

    await act(async () => {
      resolveDelete(
        createCaptureDeleteResult({
          sessionId: firstSessionId,
          captureId,
        }),
      )

      await expect(staleDelete).resolves.toMatchObject({
        code: 'host-unavailable',
      })
    })

    expect(latestState!.sessionDraft.sessionId).toBe(secondSessionId)
    expect(latestState!.sessionDraft.boothAlias).toBe('Lee 1234')
  })

  it('does not resurrect a deleted capture when a late same-session readiness event arrives', async () => {
    let latestState: SessionStateContextValue | null = null
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const captureId = 'capture_latest'
    const previewAssetPath = `C:/Users/Example/Pictures/dabi_shoot/sessions/${sessionId}/renders/previews/${captureId}.jpg`
    const startSession = vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
      sessionId,
      boothAlias: 'Kim 4821',
      manifest: {
        schemaVersion: 'session-manifest/v1',
        sessionId,
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
        captures: [
          createCaptureRecord({
            sessionId,
            captureId,
            renderStatus: 'previewReady',
            preview: {
              assetPath: previewAssetPath,
              enqueuedAtMs: 100,
              readyAtMs: 500,
            },
          }),
        ],
        postEnd: null,
      },
    })
    const deleteCapture = vi
      .fn<NonNullable<CaptureRuntimeService['deleteCapture']>>()
      .mockResolvedValue(
        createCaptureDeleteResult({
          sessionId,
          captureId,
          manifest: {
            schemaVersion: 'session-manifest/v1',
            sessionId,
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
          readiness: createReadinessSnapshot({
            sessionId,
            latestCapture: null,
          }),
        }),
      )

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession,
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(createReadinessSnapshot({ sessionId })),
          deleteCapture,
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          subscribeToCaptureReadiness: vi
            .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
            .mockImplementation(async ({ onReadiness }) => {
              emitReadiness = onReadiness
              return () => {
                emitReadiness = null
              }
            }),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(emitReadiness).not.toBeNull()
    })

    await act(async () => {
      await latestState!.deleteCapture({
        sessionId,
        captureId,
      })
    })

    act(() => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'previewReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '방금 찍은 사진을 아래에서 바로 확인할 수 있어요.',
          reasonCode: 'ready',
          latestCapture: createCaptureRecord({
            sessionId,
            captureId,
            renderStatus: 'previewReady',
            preview: {
              assetPath: previewAssetPath,
              enqueuedAtMs: 100,
              readyAtMs: 500,
            },
          }),
        }),
      )
    })

    expect(latestState!.sessionDraft.manifest?.captures).toEqual([])
    expect(latestState!.sessionDraft.captureReadiness?.latestCapture).toBeNull()
  })

  it('ignores foreign timing updates so another session cannot flip the active warning state', async () => {
    let latestState: SessionStateContextValue | null = null
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession: vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
              ...createSessionStartResult(sessionId, 'Kim 4821'),
              manifest: {
                ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
                activePreset: {
                  presetId: 'preset_soft-glow',
                  publishedVersion: '2026.03.20',
                },
                timing: createTimingSnapshot(),
              },
            }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(
              createReadinessSnapshot({
                sessionId,
                timing: createTimingSnapshot(),
              }),
            ),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
          subscribeToCaptureReadiness: vi
            .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
            .mockImplementation(async ({ onReadiness }) => {
              emitReadiness = onReadiness
              return () => undefined
            }),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(emitReadiness).not.toBeNull()
      expect(latestState!.sessionDraft.captureReadiness?.timing?.phase).toBe('active')
    })

    act(() => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          reasonCode: 'warning',
          supportMessage: '남은 시간 안에 계속 찍을 수 있어요.',
          timing: createTimingSnapshot({
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            phase: 'warning',
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
          }),
        }),
      )
    })

    expect(latestState!.sessionDraft.captureReadiness?.sessionId).toBe(sessionId)
    expect(latestState!.sessionDraft.captureReadiness?.timing?.phase).toBe('active')
    expect(latestState!.sessionDraft.manifest?.lifecycle.stage).not.toBe('warning')
  })

  it('merges an exact-end timing update into the active session and projects the ended lifecycle stage', async () => {
    let latestState: SessionStateContextValue | null = null
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession: vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
              ...createSessionStartResult(sessionId, 'Kim 4821'),
              manifest: {
                ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
                activePreset: {
                  presetId: 'preset_soft-glow',
                  publishedVersion: '2026.03.20',
                },
                timing: createTimingSnapshot(),
              },
            }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(
              createReadinessSnapshot({
                sessionId,
                timing: createTimingSnapshot(),
              }),
            ),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
          subscribeToCaptureReadiness: vi
            .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
            .mockImplementation(async ({ onReadiness }) => {
              emitReadiness = onReadiness
              return () => undefined
            }),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(emitReadiness).not.toBeNull()
    })

    act(() => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'blocked',
          customerState: 'Export Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '촬영은 끝났고 결과를 준비하고 있어요.',
          supportMessage: '다음 안내가 나올 때까지 잠시만 기다려 주세요.',
          reasonCode: 'export-waiting',
          postEnd: {
            state: 'export-waiting',
            evaluatedAt: '2026-03-20T00:15:00.000Z',
          },
          timing: createTimingSnapshot({
            sessionId,
            phase: 'ended',
            captureAllowed: false,
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
            endedTriggeredAt: '2026-03-20T00:15:00.000Z',
          }),
        }),
      )
    })

    expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('export-waiting')
    expect(latestState!.sessionDraft.captureReadiness?.postEnd).toMatchObject({
      state: 'export-waiting',
    })
    expect(latestState!.sessionDraft.captureReadiness?.timing?.phase).toBe('ended')
    expect(latestState!.sessionDraft.manifest?.timing?.phase).toBe('ended')
    expect(latestState!.sessionDraft.manifest?.postEnd).toMatchObject({
      state: 'export-waiting',
    })
    expect(latestState!.sessionDraft.manifest?.lifecycle.stage).toBe('export-waiting')
  })

  it('preserves manifest post-end guidance when readiness confirms completion without repeating the payload', async () => {
    let latestState: SessionStateContextValue | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession: vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
              ...createSessionStartResult(sessionId, 'Kim 4821'),
              manifest: {
                ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
                activePreset: {
                  presetId: 'preset_soft-glow',
                  publishedVersion: '2026.03.20',
                },
                timing: createTimingSnapshot({
                  phase: 'ended',
                  captureAllowed: false,
                }),
                lifecycle: {
                  status: 'active',
                  stage: 'completed',
                },
                postEnd: {
                  state: 'completed',
                  evaluatedAt: '2026-03-20T00:15:05.000Z',
                  completionVariant: 'handoff-ready',
                  approvedRecipientLabel: 'Front Desk',
                  primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
                  supportActionLabel: null,
                  showBoothAlias: true,
                },
              },
            }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(
              createReadinessSnapshot({
                sessionId,
                surfaceState: 'blocked',
                customerState: 'Completed',
                canCapture: false,
                primaryAction: 'wait',
                customerMessage: '부스 준비가 끝났어요.',
                supportMessage: '마지막 안내를 확인해 주세요.',
                reasonCode: 'completed',
                postEnd: null,
                timing: createTimingSnapshot({
                  sessionId,
                  phase: 'ended',
                  captureAllowed: false,
                }),
              }),
            ),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<NonNullable<CaptureRuntimeService['deleteCapture']>>(),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('completed')
      expect(latestState!.sessionDraft.manifest?.postEnd).toMatchObject({
        state: 'completed',
        completionVariant: 'handoff-ready',
        approvedRecipientLabel: 'Front Desk',
      })
      expect(latestState!.sessionDraft.manifest?.lifecycle.stage).toBe('completed')
    })
  })

  it('does not let a late same-session ready update overwrite completed handoff guidance', async () => {
    let latestState: SessionStateContextValue | null = null
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession: vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
              ...createSessionStartResult(sessionId, 'Kim 4821'),
              manifest: {
                ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
                activePreset: {
                  presetId: 'preset_soft-glow',
                  publishedVersion: '2026.03.20',
                },
              },
            }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(
              createReadinessSnapshot({
                sessionId,
                surfaceState: 'blocked',
                customerState: 'Completed',
                canCapture: false,
                primaryAction: 'wait',
                customerMessage: '부스 준비가 끝났어요.',
                supportMessage: '마지막 안내를 확인해 주세요.',
                reasonCode: 'completed',
                postEnd: {
                  state: 'completed',
                  evaluatedAt: '2026-03-20T00:00:10.000Z',
                  completionVariant: 'handoff-ready',
                  approvedRecipientLabel: 'Front Desk',
                  primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
                  supportActionLabel: null,
                  showBoothAlias: true,
                },
              }),
            ),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
          subscribeToCaptureReadiness: vi
            .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
            .mockImplementation(async ({ onReadiness }) => {
              emitReadiness = onReadiness
              return () => undefined
            }),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('completed')
      expect(latestState!.sessionDraft.captureReadiness?.postEnd).not.toBeNull()
      expect(emitReadiness).not.toBeNull()
    })

    act(() => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'captureReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '버튼을 누르면 바로 시작돼요.',
          reasonCode: 'ready',
          postEnd: null,
        }),
      )
    })

    expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('completed')
    expect(latestState!.sessionDraft.captureReadiness?.postEnd).toMatchObject({
      state: 'completed',
      completionVariant: 'handoff-ready',
      approvedRecipientLabel: 'Front Desk',
    })
  })

  it('does not let a weaker same-session export-waiting update overwrite completed guidance', async () => {
    let latestState: SessionStateContextValue | null = null
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession: vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
              ...createSessionStartResult(sessionId, 'Kim 4821'),
              manifest: {
                ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
                activePreset: {
                  presetId: 'preset_soft-glow',
                  publishedVersion: '2026.03.20',
                },
              },
            }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(
              createReadinessSnapshot({
                sessionId,
                surfaceState: 'blocked',
                customerState: 'Completed',
                canCapture: false,
                primaryAction: 'wait',
                customerMessage: '부스 준비가 끝났어요.',
                supportMessage: '마지막 안내를 확인해 주세요.',
                reasonCode: 'completed',
                postEnd: {
                  state: 'completed',
                  evaluatedAt: '2026-03-20T00:00:10.000Z',
                  completionVariant: 'local-deliverable-ready',
                  primaryActionLabel: '안내를 확인해 주세요.',
                  supportActionLabel: null,
                  showBoothAlias: false,
                },
              }),
            ),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
          subscribeToCaptureReadiness: vi
            .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
            .mockImplementation(async ({ onReadiness }) => {
              emitReadiness = onReadiness
              return () => undefined
            }),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('completed')
      expect(emitReadiness).not.toBeNull()
    })

    act(() => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'blocked',
          customerState: 'Export Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '촬영은 끝났고 결과를 준비하고 있어요.',
          supportMessage: '다음 안내가 나올 때까지 잠시만 기다려 주세요.',
          reasonCode: 'export-waiting',
          postEnd: {
            state: 'export-waiting',
            evaluatedAt: '2026-03-20T00:00:20.000Z',
          },
        }),
      )
    })

    expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('completed')
    expect(latestState!.sessionDraft.captureReadiness?.postEnd).toMatchObject({
      state: 'completed',
      completionVariant: 'local-deliverable-ready',
    })
  })

  it('accepts a host correction from completed to phone-required for the same session', async () => {
    let latestState: SessionStateContextValue | null = null
    let emitReadiness: ((readiness: CaptureReadinessSnapshot) => void) | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    render(
      <SessionProvider
        sessionService={createStartSessionService({
          gateway: {
            startSession: vi.fn<StartSessionGateway['startSession']>().mockResolvedValue({
              ...createSessionStartResult(sessionId, 'Kim 4821'),
              manifest: {
                ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
                activePreset: {
                  presetId: 'preset_soft-glow',
                  publishedVersion: '2026.03.20',
                },
              },
            }),
          },
        })}
        captureRuntimeService={{
          getCaptureReadiness: vi
            .fn<CaptureRuntimeService['getCaptureReadiness']>()
            .mockResolvedValue(
              createReadinessSnapshot({
                sessionId,
                surfaceState: 'blocked',
                customerState: 'Completed',
                canCapture: false,
                primaryAction: 'wait',
                customerMessage: '부스 준비가 끝났어요.',
                supportMessage: '마지막 안내를 확인해 주세요.',
                reasonCode: 'completed',
                postEnd: {
                  state: 'completed',
                  evaluatedAt: '2026-03-20T00:00:10.000Z',
                  completionVariant: 'handoff-ready',
                  approvedRecipientLabel: 'Front Desk',
                  primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
                  supportActionLabel: null,
                  showBoothAlias: true,
                },
              }),
            ),
          requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
          deleteCapture: vi.fn<
            NonNullable<CaptureRuntimeService['deleteCapture']>
          >(),
          subscribeToCaptureReadiness: vi
            .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
            .mockImplementation(async ({ onReadiness }) => {
              emitReadiness = onReadiness
              return () => undefined
            }),
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

    await act(async () => {
      await latestState!.startSession({
        name: 'Kim',
        phoneLastFour: '4821',
      })
    })

    await waitFor(() => {
      expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('completed')
      expect(emitReadiness).not.toBeNull()
    })

    act(() => {
      emitReadiness?.(
        createReadinessSnapshot({
          sessionId,
          surfaceState: 'blocked',
          customerState: 'Phone Required',
          canCapture: false,
          primaryAction: 'call-support',
          customerMessage: '지금은 도움이 필요해요.',
          supportMessage: '가까운 직원에게 알려 주세요.',
          reasonCode: 'phone-required',
          postEnd: {
            state: 'phone-required',
            evaluatedAt: '2026-03-20T00:00:30.000Z',
            primaryActionLabel: '가까운 직원에게 알려 주세요.',
            supportActionLabel: '직원에게 도움을 요청해 주세요.',
            unsafeActionWarning: '다시 찍기나 기기 조작은 잠시 멈춰 주세요.',
            showBoothAlias: false,
          },
        }),
      )
    })

    expect(latestState!.sessionDraft.captureReadiness?.reasonCode).toBe('phone-required')
    expect(latestState!.sessionDraft.captureReadiness?.postEnd).toMatchObject({
      state: 'phone-required',
      primaryActionLabel: '가까운 직원에게 알려 주세요.',
    })
  })
  it('ignores a deleted capture fast preview that arrives late from the same session', async () => {
    let emitFastPreview:
      | ((update: {
          schemaVersion: string
          sessionId: string
          requestId: string
          captureId: string
          assetPath: string
          visibleAtMs: number
          kind?: string | null
        }) => void)
      | null = null
    let latestState: SessionStateContextValue | null = null
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    const deletedCaptureId = 'capture_deleted_fast_preview'
    const deletedRequestId = 'request_deleted_fast_preview'

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
          activePresetDisplayName: 'Soft Glow',
          captures: [
            createCaptureRecord({
              sessionId,
              captureId: deletedCaptureId,
              requestId: deletedRequestId,
              renderStatus: 'previewWaiting',
              preview: {
                assetPath: null,
                enqueuedAtMs: 100,
                readyAtMs: null,
              },
            }),
          ],
        },
      })
    const deleteCapture = vi
      .fn<NonNullable<CaptureRuntimeService['deleteCapture']>>()
      .mockResolvedValue(
        createCaptureDeleteResult({
          sessionId,
          captureId: deletedCaptureId,
          manifest: {
            ...createSessionStartResult(sessionId, 'Kim 4821').manifest,
            activePreset: {
              presetId: 'preset_soft-glow',
              publishedVersion: '2026.03.20',
            },
            activePresetDisplayName: 'Soft Glow',
            captures: [],
          },
          readiness: createReadinessSnapshot({
            sessionId,
            surfaceState: 'captureReady',
            customerState: 'Ready',
            canCapture: true,
            primaryAction: 'capture',
            customerMessage: '지금 촬영할 수 있어요.',
            supportMessage: '버튼을 누르면 바로 시작돼요.',
            reasonCode: 'ready',
            latestCapture: null,
          }),
        }),
      )
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi
        .fn<CaptureRuntimeService['getCaptureReadiness']>()
        .mockResolvedValue(
          createReadinessSnapshot({
            sessionId,
            latestCapture: createCaptureRecord({
              sessionId,
              captureId: deletedCaptureId,
              requestId: deletedRequestId,
              renderStatus: 'previewWaiting',
              preview: {
                assetPath: null,
                enqueuedAtMs: 100,
                readyAtMs: null,
              },
            }),
            surfaceState: 'captureSaved',
            customerState: 'Preview Waiting',
            canCapture: false,
            primaryAction: 'wait',
            customerMessage: '사진이 안전하게 저장되었어요.',
            supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
            reasonCode: 'preview-waiting',
          }),
        ),
      deleteCapture,
      requestCapture: vi.fn<CaptureRuntimeService['requestCapture']>(),
      subscribeToCaptureReadiness: vi
        .fn<CaptureRuntimeService['subscribeToCaptureReadiness']>()
        .mockResolvedValue(() => undefined),
      subscribeToCaptureFastPreview: vi
        .fn<NonNullable<CaptureRuntimeService['subscribeToCaptureFastPreview']>>()
        .mockImplementation(async ({ onFastPreview }) => {
          emitFastPreview = onFastPreview as typeof emitFastPreview
          return () => {
            emitFastPreview = null
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
      expect(emitFastPreview).not.toBeNull()
    })

    await act(async () => {
      await latestState!.deleteCapture({
        sessionId,
        captureId: deletedCaptureId,
      })
    })

    expect(latestState!.sessionDraft.pendingFastPreview).toBeNull()
    expect(latestState!.sessionDraft.manifest?.captures).toEqual([])

    act(() => {
      emitFastPreview?.({
        schemaVersion: 'capture-fast-preview-update/v1',
        sessionId,
        requestId: deletedRequestId,
        captureId: deletedCaptureId,
        assetPath:
          `C:/Pictures/Dabi_Shoot/sessions/${sessionId}/renders/previews/${deletedCaptureId}.jpg`,
        visibleAtMs: 140,
        kind: 'camera-thumbnail',
      })
    })

    expect(latestState!.sessionDraft.pendingFastPreview).toBeNull()
  })
})
