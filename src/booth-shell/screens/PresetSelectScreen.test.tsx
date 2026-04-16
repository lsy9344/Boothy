import { StrictMode } from 'react'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RouterProvider, createMemoryRouter } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'

import { createAppRoutes } from '../../app/routes'
import { createCapabilityService } from '../../app/services/capability-service'
import {
  createCaptureRuntimeService,
  type CaptureRuntimeGateway,
  type CaptureRuntimeService,
} from '../../capture-adapter/services/capture-runtime'
import {
  createPresetCatalogService,
  type PresetCatalogGateway,
  type PresetCatalogService,
} from '../../preset-catalog/services/preset-catalog-service'
import type {
  ActivePresetBinding,
  PresetCatalogResult,
  PresetSelectionResult,
  SessionStartResult,
} from '../../shared-contracts'
import {
  createActivePresetService,
  type ActivePresetGateway,
  type ActivePresetService,
} from '../../session-domain/services/active-preset'
import {
  createStartSessionService,
  type StartSessionGateway,
} from '../../session-domain/services/start-session'

function createSessionStartResult(): SessionStartResult {
  return {
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
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
        stage: 'session-started',
      },
      activePreset: null,
      captures: [],
      postEnd: null,
    },
  }
}

function createCatalogResult(): PresetCatalogResult {
  return {
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    state: 'ready',
    presets: [
      {
        presetId: 'preset_soft-glow',
        displayName: 'Soft Glow',
        publishedVersion: '2026.03.20',
        boothStatus: 'booth-safe',
        preview: {
          kind: 'preview-tile',
          assetPath: 'published/preset_soft-glow/2026.03.20/preview.jpg',
          altText: 'Soft Glow sample portrait',
        },
      },
      {
        presetId: 'preset_mono-pop',
        displayName: 'Mono Pop',
        publishedVersion: '2026.03.20',
        boothStatus: 'booth-safe',
        preview: {
          kind: 'sample-cut',
          assetPath: 'published/preset_mono-pop/2026.03.20/sample.jpg',
          altText: 'Mono Pop sample portrait',
        },
      },
    ],
  }
}

function createSelectionResult(
  preset: ActivePresetBinding,
): PresetSelectionResult {
  const session = createSessionStartResult()

  return {
    sessionId: session.sessionId,
    activePreset: preset,
    manifest: {
      ...session.manifest,
      activePreset: preset,
      activePresetDisplayName:
        preset.presetId === 'preset_soft-glow' ? 'Soft Glow' : 'Mono Pop',
      updatedAt: '2026-03-20T00:05:00.000Z',
    },
  }
}

function createCaptureRecord() {
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
      previewBudgetMs: 2500,
      previewBudgetState: 'pending',
    },
  }
}

function renderPresetFlow({
  startSession = vi
    .fn<StartSessionGateway['startSession']>()
    .mockResolvedValue(createSessionStartResult()),
  loadPresetCatalog = vi
    .fn<PresetCatalogGateway['loadPresetCatalog']>()
    .mockResolvedValue(createCatalogResult()),
  selectActivePreset = vi
    .fn<ActivePresetGateway['selectActivePreset']>()
    .mockImplementation(async ({ preset }) => createSelectionResult(preset)),
  getCaptureReadiness = vi
    .fn<CaptureRuntimeGateway['getCaptureReadiness']>()
    .mockResolvedValue({
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
    }),
  subscribeToCaptureReadiness = vi
    .fn<CaptureRuntimeGateway['subscribeToCaptureReadiness']>()
    .mockResolvedValue(() => undefined),
  captureRuntimeService: captureRuntimeServiceProp,
  strictMode = false,
}: {
  startSession?: ReturnType<typeof vi.fn<StartSessionGateway['startSession']>>
  loadPresetCatalog?: ReturnType<typeof vi.fn<PresetCatalogGateway['loadPresetCatalog']>>
  selectActivePreset?: ReturnType<typeof vi.fn<ActivePresetGateway['selectActivePreset']>>
  getCaptureReadiness?: ReturnType<typeof vi.fn<CaptureRuntimeGateway['getCaptureReadiness']>>
  subscribeToCaptureReadiness?: ReturnType<typeof vi.fn<CaptureRuntimeGateway['subscribeToCaptureReadiness']>>
  captureRuntimeService?: CaptureRuntimeService
  strictMode?: boolean
} = {}) {
  const sessionService = createStartSessionService({
    gateway: {
      startSession,
    },
  })
  const presetCatalogService: PresetCatalogService = createPresetCatalogService({
    gateway: {
      loadPresetCatalog,
    },
  })
  const activePresetService: ActivePresetService = createActivePresetService({
    gateway: {
      selectActivePreset,
    },
  })
  const captureRuntimeService: CaptureRuntimeService =
    captureRuntimeServiceProp ??
    createCaptureRuntimeService({
      gateway: {
        getCaptureReadiness,
        requestCapture: vi.fn().mockResolvedValue({
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
        }),
        subscribeToCaptureReadiness,
      },
    })

  const router = createMemoryRouter(
    createAppRoutes({
      capabilityService: createCapabilityService(),
      sessionService,
      presetCatalogService,
      activePresetService,
      captureRuntimeService,
    }),
    {
      initialEntries: ['/booth'],
    },
  )

  const app = <RouterProvider router={router} />

  render(strictMode ? <StrictMode>{app}</StrictMode> : app)

  return {
    startSession,
    loadPresetCatalog,
    selectActivePreset,
    getCaptureReadiness,
  }
}

describe('PresetSelectScreen', () => {
  it('keeps booth users on the session start step until an active session exists', async () => {
    renderPresetFlow()

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { name: /원하는 룩을 골라 주세요/i }),
    ).not.toBeInTheDocument()
  })

  it('loads and renders the booth-safe preset catalog after session start', async () => {
    const user = userEvent.setup()
    const { loadPresetCatalog } = renderPresetFlow()

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(
      await screen.findByRole('heading', { name: /원하는 룩을 골라 주세요/i }),
    ).toBeInTheDocument()
    expect(loadPresetCatalog).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    })
    expect(screen.getByRole('button', { name: /soft glow/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /mono pop/i })).toBeInTheDocument()
  })

  it('resumes directly into capture and backfills the selected preset name when an active preset already exists', async () => {
    const user = userEvent.setup()
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(),
        manifest: {
          ...createSessionStartResult().manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetDisplayName: 'Soft Glow',
        },
      })

    renderPresetFlow({
      startSession,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(
      await screen.findByRole('heading', { name: /지금 촬영할 수 있어요/i }),
    ).toBeInTheDocument()
    expect(
      await screen.findByText(/soft glow/i),
    ).toBeInTheDocument()
    expect(screen.queryByText(/선택 대기 중/i)).not.toBeInTheDocument()
  })

  it('lets the customer switch looks mid-session without rebinding earlier photos', async () => {
    const user = userEvent.setup()
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(),
        manifest: {
          ...createSessionStartResult().manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetDisplayName: 'Soft Glow',
          captures: [
            {
              ...createCaptureRecord(),
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/current-session-latest.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            },
          ],
        },
      })

    renderPresetFlow({
      startSession,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    await user.click(
      await screen.findByRole('button', { name: /다음 촬영 룩 바꾸기/i }),
    )

    expect(
      await screen.findByRole('heading', { name: /다음 촬영 룩을 다시 골라 주세요/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByText(/이미 찍은 사진은 그대로 두고, 다음 촬영부터만 새 룩으로 이어져요\./i),
    ).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /mono pop/i }))

    expect(
      await screen.findByRole('heading', { name: /지금 촬영할 수 있어요/i }),
    ).toBeInTheDocument()
    expect(screen.getByText(/mono pop/i)).toBeInTheDocument()
    expect(screen.getByText(/촬영 당시 soft glow 룩/i)).toBeInTheDocument()
    expect(
      screen.getByText(/이 사진은 이전 룩으로 찍혔고 그대로 유지돼요\./i),
    ).toBeInTheDocument()
  })

  it('retries active preset hydration after a transient catalog failure and backfills the preset name', async () => {
    const user = userEvent.setup()
    const startSession = vi
      .fn<StartSessionGateway['startSession']>()
      .mockResolvedValue({
        ...createSessionStartResult(),
        manifest: {
          ...createSessionStartResult().manifest,
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetDisplayName: 'Soft Glow',
        },
      })
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockRejectedValueOnce({
        code: 'preset-catalog-unavailable',
        message: '지금은 프리셋을 불러올 수 없어요.',
      })
      .mockResolvedValueOnce(createCatalogResult())
    const captureRuntimeService: CaptureRuntimeService = {
      getCaptureReadiness: vi.fn().mockResolvedValue({
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
      }),
      requestCapture: vi.fn().mockResolvedValue({
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
      }),
      subscribeToCaptureReadiness: vi.fn().mockResolvedValue(() => undefined),
    }

    renderPresetFlow({
      startSession,
      loadPresetCatalog,
      captureRuntimeService,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(
      await screen.findByRole('heading', { name: /지금 촬영할 수 있어요/i }),
    ).toBeInTheDocument()
    expect(await screen.findByText(/soft glow/i)).toBeInTheDocument()
    expect(loadPresetCatalog).toHaveBeenCalledTimes(1)

    await expect
      .poll(() => loadPresetCatalog.mock.calls.length, {
        timeout: 4000,
      })
      .toBe(2)
    expect(screen.queryByText(/^2026\.03\.20$/i)).not.toBeInTheDocument()
  })

  it('renders six preset cards when the catalog is at the display limit', async () => {
    const user = userEvent.setup()
    renderPresetFlow({
      loadPresetCatalog: vi.fn().mockResolvedValue({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        state: 'ready',
        presets: Array.from({ length: 6 }, (_, index) => ({
          presetId: `preset_limit-${index + 1}`,
          displayName: `Limit ${index + 1}`,
          publishedVersion: '2026.03.20',
          boothStatus: 'booth-safe',
          preview: {
            kind: 'preview-tile' as const,
            assetPath: `fixtures/limit-${index + 1}.jpg`,
            altText: `Limit ${index + 1} sample portrait`,
          },
        })),
      }),
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(await screen.findAllByRole('button')).toHaveLength(6)
    expect(screen.getByRole('button', { name: /limit 1/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /limit 6/i })).toBeInTheDocument()
  })

  it('moves to the capture screen and requests readiness after persisting the stable preset identity', async () => {
    const user = userEvent.setup()
    const { selectActivePreset, getCaptureReadiness } = renderPresetFlow()

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))
    const softGlowCard = await screen.findByRole('button', { name: /soft glow/i })

    await user.click(softGlowCard)

    expect(selectActivePreset).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      preset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
    })
    expect(
      await screen.findByRole('heading', { name: /지금 촬영할 수 있어요/i }),
    ).toBeInTheDocument()
    expect(getCaptureReadiness).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    })
    expect(
      screen.getByText(/soft glow/i),
    ).toBeInTheDocument()
  })

  it('does not persist again while the same preset selection is already saving', async () => {
    const user = userEvent.setup()
    let resolveSelection!: () => void
    const selectActivePreset = vi
      .fn<ActivePresetGateway['selectActivePreset']>()
      .mockImplementation(
        ({ preset }) =>
          new Promise((resolve) => {
            resolveSelection = () => resolve(createSelectionResult(preset))
          }),
      )

    renderPresetFlow({
      selectActivePreset,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    const softGlowCard = await screen.findByRole('button', { name: /soft glow/i })
    await user.click(softGlowCard)
    await user.click(softGlowCard)

    expect(selectActivePreset).toHaveBeenCalledTimes(1)

    resolveSelection()
  })

  it('never exposes authoring-only terminology on the customer selection screen', async () => {
    const user = userEvent.setup()
    renderPresetFlow()

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))
    await screen.findByRole('heading', { name: /원하는 룩을 골라 주세요/i })

    expect(screen.queryByText(/darktable/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/authoring/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/xmp/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/style/i)).not.toBeInTheDocument()
  })

  it('offers a retry path after a transient preset catalog load failure', async () => {
    const user = userEvent.setup()
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockRejectedValueOnce({
        code: 'preset-catalog-unavailable',
        message: '지금은 프리셋을 불러올 수 없어요.',
      })
      .mockResolvedValue(createCatalogResult())

    renderPresetFlow({
      loadPresetCatalog,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(
      await screen.findByRole('button', { name: /다시 불러올게요/i }),
    ).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /다시 불러올게요/i }))

    expect(loadPresetCatalog).toHaveBeenCalledTimes(2)
    expect(
      await screen.findByRole('button', { name: /soft glow/i }),
    ).toBeInTheDocument()
  })

  it('does not surface a false load error during StrictMode double effects', async () => {
    const user = userEvent.setup()
    let resolveCatalog: ((result: PresetCatalogResult) => void) | undefined
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveCatalog = resolve
          }),
      )

    renderPresetFlow({
      loadPresetCatalog,
      strictMode: true,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(loadPresetCatalog).toHaveBeenCalledTimes(1)

    resolveCatalog?.(createCatalogResult())

    expect(await screen.findByRole('button', { name: /soft glow/i })).toBeInTheDocument()
    expect(
      screen.queryByText(/지금은 프리셋을 불러올 수 없어요/i),
    ).not.toBeInTheDocument()
  })

  it('returns to session start when the catalog load says the session is gone', async () => {
    const user = userEvent.setup()

    renderPresetFlow({
      loadPresetCatalog: vi.fn().mockRejectedValue({
        code: 'session-not-found',
        message: '세션을 다시 시작해 주세요.',
      }),
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
  })

  it('reloads the catalog when the selected preset is no longer available to save', async () => {
    const user = userEvent.setup()
    const loadPresetCatalog = vi
      .fn<PresetCatalogGateway['loadPresetCatalog']>()
      .mockResolvedValue(createCatalogResult())
    const selectActivePreset = vi
      .fn<ActivePresetGateway['selectActivePreset']>()
      .mockRejectedValue({
        code: 'preset-not-available',
        message: '다른 프리셋을 골라 주세요.',
      })

    renderPresetFlow({
      loadPresetCatalog,
      selectActivePreset,
    })

    await user.type(await screen.findByLabelText(/이름/i), 'Kim')
    await user.type(screen.getByLabelText(/휴대전화 뒤 4자리/i), '4821')
    await user.click(screen.getByRole('button', { name: /시작하기/i }))
    await user.click(await screen.findByRole('button', { name: /soft glow/i }))

    expect(loadPresetCatalog).toHaveBeenCalledTimes(2)
    expect(
      await screen.findByRole('heading', { name: /원하는 룩을 골라 주세요/i }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /soft glow/i })).toBeInTheDocument()
  })
})
