import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type {
  CaptureReadinessSnapshot,
  SessionCaptureRecord,
  SessionManifest,
  SessionTimingSnapshot,
} from '../../shared-contracts'
import type { SessionStateContextValue } from '../../session-domain/state/session-context'
import { SessionStateContext } from '../../session-domain/state/session-context'
import { DEFAULT_SESSION_DRAFT } from '../../session-domain/state/session-draft'
import { CaptureScreen } from './CaptureScreen'

const { playTimingCueMock } = vi.hoisted(() => ({
  playTimingCueMock: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('../../timing-policy/audio', () => ({
  playTimingCue: playTimingCueMock,
}))

beforeEach(() => {
  playTimingCueMock.mockClear()
})

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
      assetPath: 'C:/boothy/sessions/session_01/captures/originals/capture.jpg',
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

type CaptureScreenSessionDraftOverrides = Omit<
  Partial<SessionStateContextValue['sessionDraft']>,
  'captureReadiness' | 'manifest'
> & {
  captureReadiness?: Partial<CaptureReadinessSnapshot> | null
  manifest?: Partial<SessionManifest> | null
}

function renderCaptureScreen(
  overrides: Partial<SessionStateContextValue> = {},
  sessionDraftOverrides: CaptureScreenSessionDraftOverrides = {},
) {
  const {
    captureReadiness: captureReadinessOverrides,
    manifest: manifestOverrides,
    ...restSessionDraftOverrides
  } = sessionDraftOverrides
  const defaultCaptureReadiness: CaptureReadinessSnapshot = {
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
    timing: createTimingSnapshot(),
  }
  const defaultManifest: SessionManifest = {
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
      stage: 'preset-selected',
    },
    activePreset: {
      presetId: 'preset_soft-glow',
      publishedVersion: '2026.03.20',
    },
    activePresetDisplayName: 'Soft Glow',
    captures: [],
    activePresetId: 'preset_soft-glow',
    timing: createTimingSnapshot(),
    postEnd: null,
  }

  const value: SessionStateContextValue = {
    isStarting: false,
    isLoadingPresetCatalog: false,
    isSelectingPreset: false,
    isLoadingCaptureReadiness: false,
    isDeletingCapture: false,
    isRequestingCapture: false,
    sessionDraft: {
      ...DEFAULT_SESSION_DRAFT,
      flowStep: 'capture',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      selectedPreset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
      presetCatalog: [
        {
          presetId: 'preset_soft-glow',
          displayName: 'Soft Glow',
          publishedVersion: '2026.03.20',
          boothStatus: 'booth-safe',
          preview: {
            kind: 'preview-tile',
            assetPath: 'published/preset_soft-glow/2026.03.20/preview.jpg',
            altText: 'Soft Glow preview',
          },
        },
      ],
      captureReadiness:
        captureReadinessOverrides === undefined
          ? defaultCaptureReadiness
          : captureReadinessOverrides === null
            ? null
            : {
                ...defaultCaptureReadiness,
                ...captureReadinessOverrides,
              },
      manifest:
        manifestOverrides === undefined
          ? defaultManifest
          : manifestOverrides === null
            ? null
            : {
                ...defaultManifest,
                ...manifestOverrides,
              },
      ...restSessionDraftOverrides,
    },
    startSession: vi.fn(),
    beginPresetSwitch: vi.fn(),
    cancelPresetSwitch: vi.fn(),
    loadPresetCatalog: vi.fn(),
    selectActivePreset: vi.fn(),
    getCaptureReadiness: vi.fn(),
    deleteCapture: vi.fn().mockResolvedValue({
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
        activePresetDisplayName: 'Soft Glow',
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
        timing: createTimingSnapshot(),
      },
    }),
    requestCapture: vi.fn().mockResolvedValue({
      schemaVersion: 'capture-request-result/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      status: 'capture-saved',
      capture: createCaptureRecord(),
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
        timing: createTimingSnapshot(),
      },
    }),
    ...overrides,
  }

  render(
    <SessionStateContext.Provider value={value}>
      <CaptureScreen />
    </SessionStateContext.Provider>,
  )

  return value
}

describe('CaptureScreen', () => {
  it('shows the adjusted end time as soon as the capture flow opens', async () => {
    renderCaptureScreen()

    expect(await screen.findByText(/^종료 시각$/)).toBeInTheDocument()
    expect(screen.getByText(/^남은 시간$/)).toBeInTheDocument()
  })

  it('enables capture only when the normalized readiness says capture is allowed', async () => {
    const user = userEvent.setup()
    const value = renderCaptureScreen()

    const button = await screen.findByRole('button', { name: /사진 찍기/i })

    expect(button).toBeEnabled()

    await user.click(button)

    expect(value.requestCapture).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    })
  })

  it('shows a camera status card that explains browser preview cannot verify a real camera connection', async () => {
    renderCaptureScreen()

    expect(await screen.findByText(/^카메라 상태$/)).toBeInTheDocument()
    expect(screen.getByText(/^브라우저 미리보기$/)).toBeInTheDocument()
    expect(
      screen.getByText(/^실제 카메라 연결 상태는 앱에서만 확인할 수 있어요\.$/),
    ).toBeInTheDocument()
  })

  it('blocks capture during preparing states and shows wait guidance', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Preparing',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '촬영 준비 중이에요.',
          supportMessage: '잠시만 기다려 주세요.',
          reasonCode: 'helper-preparing',
          latestCapture: null,
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /촬영 준비 중이에요/i }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /잠시 기다리기/i })).toBeDisabled()
  })

  it('blocks capture and guides the customer to ask for help in phone-required state', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Phone Required',
          canCapture: false,
          primaryAction: 'call-support',
          customerMessage: '지금은 도움이 필요해요.',
          supportMessage: '가까운 직원에게 알려 주세요.',
          reasonCode: 'phone-required',
          latestCapture: null,
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /지금은 도움이 필요해요/i }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /도움 요청/i })).toBeDisabled()
    expect(screen.queryByText(/darktable|sdk|helper/i)).not.toBeInTheDocument()
  })

  it('keeps handoff-ready completion on generic completed guidance in story 3.2', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Completed',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '부스 준비가 끝났어요.',
          supportMessage: '마지막 안내를 확인해 주세요.',
          reasonCode: 'completed',
          latestCapture: null,
          postEnd: {
            state: 'completed',
            evaluatedAt: '2026-03-20T00:00:10.000Z',
            completionVariant: 'handoff-ready',
            approvedRecipientLabel: 'Front Desk',
            primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
            supportActionLabel: null,
            showBoothAlias: true,
          },
        },
        manifest: {
          schemaVersion: 'session-manifest/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          boothAlias: 'Kim 4821',
          customer: {
            name: 'Kim',
            phoneLastFour: '4821',
          },
          createdAt: '2026-03-20T00:00:00.000Z',
          updatedAt: '2026-03-20T00:00:10.000Z',
          lifecycle: {
            status: 'active',
            stage: 'completed',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          activePresetDisplayName: 'Soft Glow',
          captures: [],
          postEnd: {
            state: 'completed',
            evaluatedAt: '2026-03-20T00:00:10.000Z',
            completionVariant: 'handoff-ready',
            approvedRecipientLabel: 'Front Desk',
            primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
            supportActionLabel: null,
            showBoothAlias: true,
          },
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /부스 준비가 끝났어요/i }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /안내 확인/i })).not.toBeInTheDocument()
    expect(screen.getByText(/인계 안내/i)).toBeInTheDocument()
    expect(screen.getByText(/front desk/i)).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /승인된 수령 대상/i }),
    ).toBeInTheDocument()
    expect(screen.getAllByText(/kim 4821/i).length).toBeGreaterThan(0)
    expect(
      screen.queryByRole('heading', { level: 1, name: /촬영 시간이 끝났어요/i }),
    ).not.toBeInTheDocument()
  })

  it('keeps phone-required guidance generic while booth-loop actions stay blocked', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Phone Required',
          canCapture: false,
          primaryAction: 'call-support',
          customerMessage: '지금은 도움이 필요해요.',
          supportMessage: '가까운 직원에게 알려 주세요.',
          reasonCode: 'phone-required',
          latestCapture: createCaptureRecord({
            captureId: 'capture_locked',
            renderStatus: 'finalReady',
            postEndState: 'handoffReady',
            preview: {
              assetPath: 'fixtures/current-session-latest.jpg',
              enqueuedAtMs: 100,
              readyAtMs: 500,
            },
            final: {
              assetPath: 'fixtures/current-session-latest-final.jpg',
              readyAtMs: 540,
            },
          }),
          postEnd: {
            state: 'phone-required',
            evaluatedAt: '2026-03-20T00:00:10.000Z',
            primaryActionLabel: '가까운 직원에게 알려 주세요.',
            supportActionLabel: '직원에게 도움을 요청해 주세요.',
            unsafeActionWarning: '다시 찍기나 기기 조작은 잠시 멈춰 주세요.',
            showBoothAlias: false,
          },
        },
        manifest: {
          schemaVersion: 'session-manifest/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          boothAlias: 'Kim 4821',
          customer: {
            name: 'Kim',
            phoneLastFour: '4821',
          },
          createdAt: '2026-03-20T00:00:00.000Z',
          updatedAt: '2026-03-20T00:00:10.000Z',
          lifecycle: {
            status: 'active',
            stage: 'phone-required',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          activePresetDisplayName: 'Soft Glow',
          captures: [
            createCaptureRecord({
              captureId: 'capture_locked',
              renderStatus: 'finalReady',
              postEndState: 'handoffReady',
              preview: {
                assetPath: 'fixtures/current-session-latest.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
              final: {
                assetPath: 'fixtures/current-session-latest-final.jpg',
                readyAtMs: 540,
              },
            }),
          ],
          postEnd: {
            state: 'phone-required',
            evaluatedAt: '2026-03-20T00:00:10.000Z',
            primaryActionLabel: '가까운 직원에게 알려 주세요.',
            supportActionLabel: '직원에게 도움을 요청해 주세요.',
            unsafeActionWarning: '다시 찍기나 기기 조작은 잠시 멈춰 주세요.',
            showBoothAlias: false,
          },
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /지금은 도움이 필요해요/i }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /도움 요청/i })).not.toBeInTheDocument()
    expect(screen.getByText(/보호 안내/i)).toBeInTheDocument()
    expect(screen.getAllByText(/가까운 직원에게 알려 주세요\./i).length).toBeGreaterThan(0)
    expect(
      screen.getByText(/다시 찍기나 기기 조작은 잠시 멈춰 주세요\./i),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^사진 정리$/i })).not.toBeInTheDocument()
    expect(
      screen.getByText(/지금은 결과 안내가 우선이라 여기서 사진을 정리할 수 없어요\./i),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { level: 1, name: /촬영 시간이 끝났어요/i }),
    ).not.toBeInTheDocument()
  })

  it('blocks capture safely when the normalized state says the session is missing', async () => {
    renderCaptureScreen(
      {},
      {
        sessionId: null,
        boothAlias: null,
        selectedPreset: null,
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Preparing',
          canCapture: false,
          primaryAction: 'start-session',
          customerMessage: '세션을 다시 시작해 주세요.',
          supportMessage: '이름과 휴대전화 뒤 4자리를 다시 확인할게요.',
          reasonCode: 'session-missing',
          latestCapture: null,
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /세션을 다시 시작해 주세요/i }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /처음으로/i })).toBeDisabled()
  })

  it('blocks capture safely when no preset is active yet', async () => {
    renderCaptureScreen(
      {},
      {
        selectedPreset: null,
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Preparing',
          canCapture: false,
          primaryAction: 'choose-preset',
          customerMessage: '촬영 전에 룩을 먼저 골라 주세요.',
          supportMessage: '선택이 끝나면 바로 찍을 수 있어요.',
          reasonCode: 'preset-missing',
          latestCapture: null,
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /촬영 전에 룩을 먼저 골라 주세요/i }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /룩 고르기/i })).toBeDisabled()
  })

  it('matches the selected preset name by preset id and published version together', async () => {
    renderCaptureScreen(
      {},
      {
        selectedPreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.19',
        },
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
            publishedVersion: '2026.03.19',
          },
          activePresetDisplayName: null,
          activePresetId: 'preset_soft-glow',
          captures: [],
          postEnd: null,
        },
        presetCatalog: [
          {
            presetId: 'preset_soft-glow',
            displayName: 'Soft Glow March',
            publishedVersion: '2026.03.19',
            boothStatus: 'booth-safe',
            preview: {
              kind: 'preview-tile',
              assetPath: 'published/preset_soft-glow/2026.03.19/preview.jpg',
              altText: 'Soft Glow March preview',
            },
          },
          {
            presetId: 'preset_soft-glow',
            displayName: 'Soft Glow April',
            publishedVersion: '2026.03.20',
            boothStatus: 'booth-safe',
            preview: {
              kind: 'preview-tile',
              assetPath: 'published/preset_soft-glow/2026.03.20/preview.jpg',
              altText: 'Soft Glow April preview',
            },
          },
        ],
      },
    )

    expect(await screen.findByText(/soft glow march/i)).toBeInTheDocument()
    expect(screen.queryByText(/soft glow april/i)).not.toBeInTheDocument()
  })

  it('keeps the current look label customer-safe when the catalog name is unavailable', async () => {
    renderCaptureScreen(
      {},
      {
        presetCatalog: [],
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
          activePresetDisplayName: null,
          activePresetId: 'preset_soft-glow',
          captures: [],
          postEnd: null,
        },
      },
    )

    expect(await screen.findByText(/현재 룩 확인 중/i)).toBeInTheDocument()
    expect(screen.queryByText(/^2026\.03\.20$/i)).not.toBeInTheDocument()
  })

  it('opens the in-session preset switch flow without dropping the current capture surface', async () => {
    const user = userEvent.setup()
    const value = renderCaptureScreen()

    await user.click(
      await screen.findByRole('button', { name: /다음 촬영 룩 바꾸기/i }),
    )

    expect(value.beginPresetSwitch).toHaveBeenCalledTimes(1)
  })

  it('does not show a fallback error banner when requestCapture rejects with customer-safe readiness', async () => {
    const user = userEvent.setup()

    renderCaptureScreen({
      requestCapture: vi.fn().mockRejectedValue({
        code: 'capture-not-ready',
        message: 'camera helper busy',
        readiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Phone Required',
          canCapture: false,
          primaryAction: 'call-support',
          customerMessage: '지금은 도움이 필요해요.',
          supportMessage: '가까운 직원에게 알려 주세요.',
          reasonCode: 'phone-required',
          latestCapture: null,
        },
      }),
    })

    await user.click(await screen.findByRole('button', { name: /사진 찍기/i }))

    expect(screen.queryByText(/camera helper busy/i)).not.toBeInTheDocument()
  })

  it('shows a customer-safe fallback error banner when requestCapture rejects with readiness for a different session', async () => {
    const user = userEvent.setup()

    renderCaptureScreen({
      requestCapture: vi.fn().mockRejectedValue({
        code: 'capture-not-ready',
        message: 'camera helper busy',
        readiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          surfaceState: 'blocked',
          customerState: 'Phone Required',
          canCapture: false,
          primaryAction: 'call-support',
          customerMessage: '다른 세션 안내예요.',
          supportMessage: '직원을 불러 주세요.',
          reasonCode: 'phone-required',
          latestCapture: null,
        },
      }),
    })

    await user.click(await screen.findByRole('button', { name: /사진 찍기/i }))

    expect(
      screen.getByText(/현재 세션 상태를 다시 확인하고 있어요\. 잠시 후 다시 시도해 주세요\./i),
    ).toBeInTheDocument()
    expect(screen.queryByText(/camera helper busy/i)).not.toBeInTheDocument()
  })

  it('shows truthful preview waiting guidance, keeps the active preset visible, and explains an empty rail safely', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'previewWaiting',
          customerState: 'Preview Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '사진이 안전하게 저장되었어요.',
          supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
          reasonCode: 'preview-waiting',
          latestCapture: createCaptureRecord(),
          timing: createTimingSnapshot({
            phase: 'warning',
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
          }),
        },
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
            stage: 'preview-waiting',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          timing: createTimingSnapshot({
            phase: 'warning',
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
          }),
          captures: [createCaptureRecord()],
          postEnd: null,
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /사진이 안전하게 저장되었어요/i }),
    ).toBeInTheDocument()
    expect(screen.getByText(/저장 완료/i)).toBeInTheDocument()
    expect(screen.getByText(/사진 레일이 아직 비어 있어도 현재 세션 기준으로는 정상/i)).toBeInTheDocument()
    expect(screen.getByText(/soft glow/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /잠시 기다리기/i })).toBeDisabled()
    expect(
      screen.getByRole('button', { name: /다음 촬영 룩 바꾸기/i }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(screen.queryByText(/darktable|helper|filesystem|sdk/i)).not.toBeInTheDocument()
  })

  it('shows export waiting guidance without offering another preset change', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Export Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '촬영은 끝났고 결과를 준비하고 있어요.',
          supportMessage: '다음 안내가 나올 때까지 잠시만 기다려 주세요.',
          reasonCode: 'export-waiting',
          latestCapture: createCaptureRecord(),
          postEnd: {
            state: 'export-waiting',
            evaluatedAt: '2026-03-20T00:15:00.000Z',
          },
          timing: createTimingSnapshot({
            phase: 'ended',
            captureAllowed: false,
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
            endedTriggeredAt: '2026-03-20T00:15:00.000Z',
          }),
        },
      },
    )

    expect(
      await screen.findByRole('heading', {
        name: /촬영은 끝났고 결과를 준비하고 있어요/i,
      }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /다음 촬영 룩 바꾸기/i })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /잠시 기다리기/i })).toBeDisabled()
    expect(
      screen.queryByRole('heading', { level: 1, name: /촬영 시간이 끝났어요/i }),
    ).not.toBeInTheDocument()
  })

  it('shows completed guidance only after host post-end truth confirms completion', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Completed',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '부스 준비가 끝났어요.',
          supportMessage: '다음 안내를 확인해 주세요.',
          reasonCode: 'completed',
          latestCapture: createCaptureRecord({
            renderStatus: 'previewReady',
            postEndState: 'completed',
            preview: {
              assetPath: 'fixtures/current-session-latest.jpg',
              enqueuedAtMs: 100,
              readyAtMs: 500,
            },
          }),
          postEnd: {
            state: 'completed',
            evaluatedAt: '2026-03-20T00:15:05.000Z',
            completionVariant: 'local-deliverable-ready',
            primaryActionLabel: '안내를 확인해 주세요.',
            supportActionLabel: null,
            showBoothAlias: false,
          },
          timing: createTimingSnapshot({
            phase: 'ended',
            captureAllowed: false,
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
            endedTriggeredAt: '2026-03-20T00:15:00.000Z',
          }),
        },
        manifest: {
          schemaVersion: 'session-manifest/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          boothAlias: 'Kim 4821',
          customer: {
            name: 'Kim',
            phoneLastFour: '4821',
          },
          createdAt: '2026-03-20T00:00:00.000Z',
          updatedAt: '2026-03-20T00:15:05.000Z',
          lifecycle: {
            status: 'active',
            stage: 'completed',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetDisplayName: 'Soft Glow',
          activePresetId: 'preset_soft-glow',
          timing: createTimingSnapshot({
            phase: 'ended',
            captureAllowed: false,
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
            endedTriggeredAt: '2026-03-20T00:15:00.000Z',
          }),
          captures: [
            createCaptureRecord({
              renderStatus: 'previewReady',
              postEndState: 'completed',
              preview: {
                assetPath: 'fixtures/current-session-latest.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          ],
          postEnd: {
            state: 'completed',
            evaluatedAt: '2026-03-20T00:15:05.000Z',
            completionVariant: 'local-deliverable-ready',
            primaryActionLabel: '안내를 확인해 주세요.',
            supportActionLabel: null,
            showBoothAlias: false,
          },
        },
      },
    )

    expect(
      await screen.findByRole('heading', { name: /부스 준비가 끝났어요/i }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /다음 촬영 룩 바꾸기/i })).not.toBeInTheDocument()
    expect(
      screen.getByText(/지금은 결과 안내가 우선이라 여기서 사진을 정리할 수 없어요\./i),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { level: 1, name: /촬영 시간이 끝났어요/i }),
    ).not.toBeInTheDocument()
  })

  it('shows a warning state and does not replay the warning cue on a matching rerender', async () => {
    playTimingCueMock.mockClear()

    const warningValue = {
      isStarting: false,
      isLoadingPresetCatalog: false,
      isSelectingPreset: false,
      isLoadingCaptureReadiness: false,
      isDeletingCapture: false,
      isRequestingCapture: false,
      sessionDraft: {
        ...DEFAULT_SESSION_DRAFT,
        flowStep: 'capture' as const,
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        selectedPreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
        presetCatalog: [],
        presetCatalogState: 'idle' as const,
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'captureReady',
          customerState: 'Ready',
          canCapture: true,
          primaryAction: 'capture',
          customerMessage: '지금 촬영할 수 있어요.',
          supportMessage: '남은 시간 안에 계속 찍을 수 있어요.',
          reasonCode: 'warning',
          latestCapture: null,
          postEnd: null,
          timing: createTimingSnapshot({
            phase: 'warning',
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
          }),
        },
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
            stage: 'warning',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          activePresetDisplayName: 'Soft Glow',
          timing: createTimingSnapshot({
            phase: 'warning',
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
          }),
          captures: [],
          postEnd: null,
        },
      },
      startSession: vi.fn(),
      beginPresetSwitch: vi.fn(),
      cancelPresetSwitch: vi.fn(),
      loadPresetCatalog: vi.fn(),
      selectActivePreset: vi.fn(),
      getCaptureReadiness: vi.fn(),
      deleteCapture: vi.fn(),
      requestCapture: vi.fn(),
    } satisfies SessionStateContextValue

    const { rerender } = render(
      <SessionStateContext.Provider value={warningValue}>
        <CaptureScreen />
      </SessionStateContext.Provider>,
    )

    expect(await screen.findByText(/곧 종료돼요/i)).toBeInTheDocument()
    expect(screen.getByText(/종료 5분 전이에요\./i)).toBeInTheDocument()
    expect(playTimingCueMock).toHaveBeenCalledTimes(1)
    expect(playTimingCueMock).toHaveBeenCalledWith('warning')

    rerender(
      <SessionStateContext.Provider value={warningValue}>
        <CaptureScreen />
      </SessionStateContext.Provider>,
    )

    expect(playTimingCueMock).toHaveBeenCalledTimes(1)
  })

  it('shows export waiting guidance and blocks fresh capture after the adjusted end time', async () => {
    playTimingCueMock.mockClear()

    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'blocked',
          customerState: 'Export Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '촬영은 끝났고 결과를 준비하고 있어요.',
          supportMessage: '다음 안내가 나올 때까지 잠시만 기다려 주세요.',
          reasonCode: 'export-waiting',
          latestCapture: null,
          postEnd: {
            state: 'export-waiting',
            evaluatedAt: '2026-03-20T00:15:00.000Z',
          },
          timing: createTimingSnapshot({
            phase: 'ended',
            captureAllowed: false,
            warningTriggeredAt: '2026-03-20T00:10:01.000Z',
            endedTriggeredAt: '2026-03-20T00:15:00.000Z',
          }),
        },
      },
    )

    expect(
      await screen.findByRole('heading', {
        level: 1,
        name: /촬영은 끝났고 결과를 준비하고 있어요/i,
      }),
    ).toBeInTheDocument()
    expect(screen.getByText(/다음 안내가 나올 때까지 잠시만 기다려 주세요\./i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /잠시 기다리기/i })).toBeDisabled()
    expect(playTimingCueMock).toHaveBeenCalledWith('ended')
  })

  it('shows only preview-ready captures from the active session in the latest photo rail', async () => {
    renderCaptureScreen(
      {},
      {
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
          captures: [
            createCaptureRecord({
              captureId: 'capture_latest',
              renderStatus: 'finalReady',
              preview: {
                assetPath: 'fixtures/current-session-latest.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
              final: {
                assetPath: 'fixtures/current-session-latest-final.jpg',
                readyAtMs: 540,
              },
            }),
            createCaptureRecord({
              captureId: 'capture_older',
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/current-session-older.jpg',
                enqueuedAtMs: 90,
                readyAtMs: 400,
              },
            }),
            createCaptureRecord({
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              boothAlias: 'Lee 1234',
              captureId: 'capture_other-session',
              preview: {
                assetPath: 'fixtures/other-session.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 450,
              },
              renderStatus: 'previewReady',
            }),
          ],
          postEnd: null,
        },
      },
    )

    const images = await screen.findAllByRole('img')
    const rail = screen.getByRole('list', { name: /현재 세션 사진 레일/i })
    const scrollBy = vi.fn()

    Object.defineProperty(rail, 'scrollBy', {
      value: scrollBy,
      configurable: true,
    })

    expect(rail).toBeInTheDocument()
    expect(rail).toHaveAttribute('tabindex', '0')
    expect(images).toHaveLength(2)
    expect(
      screen.getByRole('img', {
        name: /현재 세션 최신 사진,\s*1번째,\s*soft glow 룩/i,
      }),
    ).toHaveAttribute('src', 'fixtures/current-session-latest.jpg')
    expect(
      screen.getByRole('img', {
        name: /현재 세션 사진,\s*2번째,\s*soft glow 룩/i,
      }),
    ).toHaveAttribute(
      'src',
      'fixtures/current-session-older.jpg',
    )
    expect(screen.getByText('최신 사진')).toBeInTheDocument()
    expect(screen.getAllByText(/촬영 당시 soft glow 룩/i)).toHaveLength(2)
    expect(
      screen.getAllByText(/현재 룩과 같은 바인딩으로 유지돼요\./i),
    ).toHaveLength(2)
    expect(screen.queryByText(/filesystem|render|diagnostic/i)).not.toBeInTheDocument()

    fireEvent.keyDown(rail, { key: 'ArrowRight' })

    expect(scrollBy).toHaveBeenCalledWith({
      left: 240,
      behavior: 'smooth',
    })
  })

  it('does not label an older thumbnail as the latest photo while a newer capture preview is still pending', async () => {
    renderCaptureScreen(
      {},
      {
        captureReadiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'previewWaiting',
          customerState: 'Preview Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '사진이 안전하게 저장되었어요.',
          supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
          reasonCode: 'preview-waiting',
          latestCapture: createCaptureRecord({
            captureId: 'capture_waiting_newer',
            raw: {
              assetPath: 'fixtures/waiting-newer-raw.jpg',
              persistedAtMs: 220,
            },
            preview: {
              assetPath: null,
              enqueuedAtMs: 220,
              readyAtMs: null,
            },
            timing: {
              captureAcknowledgedAtMs: 220,
              previewVisibleAtMs: null,
              captureBudgetMs: 1000,
              previewBudgetMs: 5000,
              previewBudgetState: 'pending',
            },
            renderStatus: 'previewWaiting',
          }),
        },
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
            stage: 'preview-waiting',
          },
          activePreset: {
            presetId: 'preset_soft-glow',
            publishedVersion: '2026.03.20',
          },
          activePresetId: 'preset_soft-glow',
          captures: [
            createCaptureRecord({
              captureId: 'capture_waiting_newer',
              raw: {
                assetPath: 'fixtures/waiting-newer-raw.jpg',
                persistedAtMs: 220,
              },
              preview: {
                assetPath: null,
                enqueuedAtMs: 220,
                readyAtMs: null,
              },
              timing: {
                captureAcknowledgedAtMs: 220,
                previewVisibleAtMs: null,
                captureBudgetMs: 1000,
                previewBudgetMs: 5000,
                previewBudgetState: 'pending',
              },
              renderStatus: 'previewWaiting',
            }),
            createCaptureRecord({
              captureId: 'capture_ready_older',
              raw: {
                assetPath: 'fixtures/ready-older-raw.jpg',
                persistedAtMs: 120,
              },
              preview: {
                assetPath: 'fixtures/ready-older.jpg',
                enqueuedAtMs: 120,
                readyAtMs: 180,
              },
              timing: {
                captureAcknowledgedAtMs: 120,
                previewVisibleAtMs: 180,
                captureBudgetMs: 1000,
                previewBudgetMs: 5000,
                previewBudgetState: 'withinBudget',
              },
              renderStatus: 'previewReady',
            }),
          ],
          postEnd: null,
        },
      },
    )

    expect(
      await screen.findByRole('img', {
        name: /현재 세션 사진,\s*1번째,\s*soft glow 룩/i,
      }),
    ).toHaveAttribute('src', 'fixtures/ready-older.jpg')
    expect(
      screen.queryByRole('img', {
        name: /현재 세션 최신 사진,\s*1번째,\s*soft glow 룩/i,
      }),
    ).not.toBeInTheDocument()
    expect(screen.queryByText('최신 사진')).not.toBeInTheDocument()
  })

  it('keeps earlier captures labeled with their original look after the active preset changes', async () => {
    renderCaptureScreen(
      {},
      {
        presetCatalog: [
          {
            presetId: 'preset_soft-glow',
            displayName: 'Soft Glow',
            publishedVersion: '2026.03.20',
            boothStatus: 'booth-safe',
            preview: {
              kind: 'preview-tile',
              assetPath: 'published/preset_soft-glow/2026.03.20/preview.jpg',
              altText: 'Soft Glow preview',
            },
          },
          {
            presetId: 'preset_mono-pop',
            displayName: 'Mono Pop',
            publishedVersion: '2026.03.19',
            boothStatus: 'booth-safe',
            preview: {
              kind: 'preview-tile',
              assetPath: 'published/preset_mono-pop/2026.03.19/preview.jpg',
              altText: 'Mono Pop preview',
            },
          },
        ],
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
          captures: [
            createCaptureRecord({
              captureId: 'capture_previous_look',
              activePresetId: 'preset_mono-pop',
              activePresetVersion: '2026.03.19',
              activePresetDisplayName: 'Mono Pop',
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/previous-look.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          ],
          postEnd: null,
        },
      },
    )

    expect(await screen.findByText(/촬영 당시 mono pop 룩/i)).toBeInTheDocument()
    expect(
      screen.getByText(/이 사진은 이전 룩으로 찍혔고 그대로 유지돼요\./i),
    ).toBeInTheDocument()
  })

  it('asks for confirmation before deleting a current-session photo and calls deleteCapture on confirm', async () => {
    const user = userEvent.setup()
    const value = renderCaptureScreen(
      {},
      {
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
          captures: [
            createCaptureRecord({
              captureId: 'capture_latest',
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/current-session-latest.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          ],
          postEnd: null,
        },
      },
    )

    await user.click(await screen.findByRole('button', { name: '사진 정리' }))

    expect(screen.getByText(/이 사진을 정리할까요\?/i)).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^사진 정리$/i }))

    expect(value.deleteCapture).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      captureId: 'capture_latest',
    })
  })

  it('shows plain-language blocked guidance when deleteCapture is rejected for the current session', async () => {
    const user = userEvent.setup()

    renderCaptureScreen(
      {
        deleteCapture: vi.fn().mockRejectedValue({
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
            latestCapture: null,
          },
        }),
      },
      {
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
          captures: [
            createCaptureRecord({
              captureId: 'capture_latest',
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/current-session-latest.jpg',
                enqueuedAtMs: 100,
                readyAtMs: 500,
              },
            }),
          ],
          postEnd: null,
        },
      },
    )

    await user.click(await screen.findByRole('button', { name: '사진 정리' }))
    await user.click(screen.getByRole('button', { name: /^사진 정리$/i }))

    expect(
      await screen.findByText(/이 사진은 지금 정리할 수 없어요\. 잠시 후 다시 확인해 주세요\./i),
    ).toBeInTheDocument()
    expect(screen.queryByText(/policy|filesystem|internal/i)).not.toBeInTheDocument()
  })
})
