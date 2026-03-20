import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import type { SessionCaptureRecord } from '../../shared-contracts'
import type { SessionStateContextValue } from '../../session-domain/state/session-context'
import { SessionStateContext } from '../../session-domain/state/session-context'
import { DEFAULT_SESSION_DRAFT } from '../../session-domain/state/session-draft'
import { CaptureScreen } from './CaptureScreen'

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

function renderCaptureScreen(
  overrides: Partial<SessionStateContextValue> = {},
  sessionDraftOverrides: Partial<SessionStateContextValue['sessionDraft']> = {},
) {
  const value: SessionStateContextValue = {
    isStarting: false,
    isLoadingPresetCatalog: false,
    isSelectingPreset: false,
    isLoadingCaptureReadiness: false,
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
      captureReadiness: {
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
          stage: 'preset-selected',
        },
        activePreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
        captures: [],
        activePresetId: 'preset_soft-glow',
        postEnd: null,
      },
      ...sessionDraftOverrides,
    },
    startSession: vi.fn(),
    loadPresetCatalog: vi.fn(),
    selectActivePreset: vi.fn(),
    getCaptureReadiness: vi.fn(),
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
    expect(screen.getAllByRole('button')).toHaveLength(1)
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(screen.queryByText(/darktable|helper|filesystem|sdk/i)).not.toBeInTheDocument()
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
              renderStatus: 'previewReady',
              preview: {
                assetPath: 'fixtures/current-session.jpg',
                enqueuedAtMs: 100,
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

    expect(images).toHaveLength(1)
    expect(images[0]).toHaveAttribute('src', 'fixtures/current-session.jpg')
  })
})
