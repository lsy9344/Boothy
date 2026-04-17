import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RouterProvider, createMemoryRouter } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'

import { createAppRoutes } from '../../app/routes'
import { createCapabilityService } from '../../app/services/capability-service'
import type { OperatorDiagnosticsService } from '../services/operator-diagnostics-service'

type OperatorRecoverySummaryFixture = ReturnType<typeof createOperatorRecoverySummary>

function createOperatorRecoverySummary(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-recovery-summary/v1',
    state: 'session-loaded',
    blockedStateCategory: 'preview-render-blocked',
    blockedCategory: 'preview-or-render',
    diagnosticsSummary: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    allowedActions: ['retry', 'approved-boundary-restart', 'route-phone-required'],
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetId: 'preset_soft-glow',
    activePresetDisplayName: 'Soft Glow',
    activePresetVersion: '2026.03.26',
    lifecycleStage: 'preview-waiting',
    timingPhase: 'active',
    updatedAt: '2026-03-26T00:10:00.000Z',
    postEndState: null,
    recentFailure: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    cameraConnection: {
      state: 'connected',
      title: '카메라와 helper 연결이 확인됐어요.',
      detail: '카메라와 helper가 현재 세션 기준으로 연결된 상태예요.',
      observedAt: '2026-03-26T00:10:00.000Z',
    },
    captureBoundary: {
      status: 'clear',
      title: '캡처 경계 정상',
      detail: '활성 preset이 선택돼 있어 capture 경계는 열려 있어요.',
    },
    previewRenderBoundary: {
      status: 'blocked',
      title: '프리뷰/렌더 결과 준비 지연',
      detail:
        '가장 최근 촬영본은 저장되었지만 preview/render 결과가 아직 준비되지 않았어요.',
    },
    completionBoundary: {
      status: 'clear',
      title: '완료 경계 대기 전',
      detail: '아직 종료 후 완료 경계로 들어가지 않았어요.',
    },
    previewArchitecture: {
      route: 'actual-primary-lane',
      routeStage: 'canary',
      implementationTrack: 'actual-primary-lane',
      laneOwner: 'inline-truthful-fallback',
      fallbackReasonCode: 'route-policy-shadow',
      firstVisibleMs: 2903,
      sameCaptureFullScreenVisibleMs: 6927,
      replacementMs: 6927,
      originalVisibleToPresetAppliedVisibleMs: 4024,
      hardwareCapability: 'dedicated-renderer-available',
      warmState: 'warm-ready',
      warmStateObservedAt: '2026-04-12T08:00:00.000Z',
    },
    ...overrides,
  }
}

function createOperatorRecoveryActionResult(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-recovery-action-result/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    action: 'retry',
    status: 'applied',
    message: '현재 막힌 preview/render 경계를 다시 시도했어요.',
    rejectionReason: null,
    diagnosticsSummary: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    nextState: {
      customerState: 'Ready',
      reasonCode: 'ready',
      lifecycleStage: 'capture-ready',
      timingPhase: 'active',
      postEndState: null,
    },
    summary: createOperatorRecoverySummary({
      blockedStateCategory: 'not-blocked',
      blockedCategory: null,
      diagnosticsSummary: null,
      allowedActions: [],
      lifecycleStage: 'capture-ready',
      previewRenderBoundary: {
        status: 'clear',
        title: '프리뷰/렌더 경계 정상',
        detail: '가장 최근 촬영본의 결과 준비가 끝나 있어요.',
      },
    }),
    ...overrides,
  }
}

function createOperatorAuditHistory(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-audit-query-result/v1',
    filter: {
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      limit: 10,
    },
    events: [
      {
        schemaVersion: 'operator-audit-entry/v1',
        eventId: 'audit_20260327_0003',
        occurredAt: '2026-03-27T00:12:00.000Z',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        eventCategory: 'critical-failure',
        eventType: 'post-end-phone-required',
        summary: '종료 후 자동 완료를 멈추고 직원 확인 상태로 전환했어요.',
        detail: '후처리 결과가 안전 기준을 벗어나 Phone Required 보호 상태로 잠겼어요.',
        actorId: null,
        source: 'post-end-evaluator',
        captureId: null,
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.26',
        reasonCode: 'render-failed',
      },
      {
        schemaVersion: 'operator-audit-entry/v1',
        eventId: 'audit_20260327_0002',
        occurredAt: '2026-03-27T00:10:00.000Z',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        eventCategory: 'operator-intervention',
        eventType: 'retry',
        summary: '막힌 preview/render 경계를 다시 시도했어요.',
        detail: '최근 촬영 결과를 다시 준비할 수 있도록 재시도를 적용했어요.',
        actorId: 'operator-kim',
        source: 'operator-console',
        captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.26',
        reasonCode: null,
      },
    ],
    summary: {
      totalEvents: 2,
      sessionLifecycleEvents: 0,
      timingTransitionEvents: 0,
      postEndOutcomeEvents: 0,
      operatorInterventionEvents: 1,
      publicationRecoveryEvents: 0,
      criticalFailureEvents: 1,
      latestOutcome: {
        occurredAt: '2026-03-27T00:12:00.000Z',
        eventCategory: 'critical-failure',
        eventType: 'post-end-phone-required',
        summary: '종료 후 자동 완료를 멈추고 직원 확인 상태로 전환했어요.',
      },
    },
    ...overrides,
  }
}

function createDeferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve
    reject = nextReject
  })

  return {
    promise,
    resolve,
    reject,
  }
}

function renderOperatorScreen(
  operatorDiagnosticsService: OperatorDiagnosticsService,
  {
    allowedSurfaces = ['booth', 'operator'] as string[],
  }: {
    allowedSurfaces?: string[]
  } = {},
) {
  const router = createMemoryRouter(
    createAppRoutes({
      capabilityService: createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces,
        currentWindowLabel: 'operator-window',
      }),
      operatorDiagnosticsService,
    }),
    {
      initialEntries: ['/operator'],
    },
  )

  render(<RouterProvider router={router} />)

  return router
}

describe('OperatorSummaryScreen', () => {
  it('opens settings governance from the operator window when settings access is allowed', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValue(createOperatorRecoverySummary())
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    const router = renderOperatorScreen(
      {
        loadOperatorRecoverySummary,
        loadOperatorAuditHistory,
        runOperatorRecoveryAction,
      },
      {
        allowedSurfaces: ['booth', 'operator', 'settings'],
      },
    )

    const user = userEvent.setup()

    expect(
      await screen.findByRole('heading', { name: /Operator Console/i }),
    ).toBeInTheDocument()

    await user.click(screen.getByRole('link', { name: /운영 설정/i }))

    expect(
      await screen.findByRole('heading', { name: /Settings Governance/i }),
    ).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/settings')
  })

  it('shows current session context and only the allowed policy actions', async () => {
    const loadOperatorRecoverySummary = vi.fn().mockResolvedValue(
      createOperatorRecoverySummary({
        rawHelperOutput: 'C:\\render-worker\\stderr.log',
      }),
    )
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    expect(
      await screen.findByRole('heading', { name: /Operator Console/i }),
    ).toBeInTheDocument()
    expect(
      await screen.findByRole('heading', { name: /^현재 세션 문맥$/i }),
    ).toBeInTheDocument()
    expect(screen.getAllByText(/Preview \/ Render 확인 필요/i)).not.toHaveLength(0)
    expect(screen.getByText('session_01hs6n1r8b8zc5v4ey2x7b9g1m')).toBeInTheDocument()
    expect(screen.getByText('Kim 4821')).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /^카메라 연결 상태$/i }),
    ).toBeInTheDocument()
    expect(screen.getByText(/^연결됨$/i)).toBeInTheDocument()
    expect(
      screen.getByText(/카메라와 helper가 현재 세션 기준으로 연결된 상태예요\./i),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^Retry$/i })).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: /^Approved Boundary Restart$/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: /^Route To Phone Required$/i }),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: /^Approved Time Extension$/i }),
    ).not.toBeInTheDocument()
    expect(
      await screen.findByRole('heading', { name: /세션 감사 기록/i }),
    ).toBeInTheDocument()
    expect(screen.getByText(/Critical Failure 1건/i)).toBeInTheDocument()
    expect(screen.getByText(/Operator Intervention 1건/i)).toBeInTheDocument()
    expect(
      screen.getByText(/종료 후 자동 완료를 멈추고 직원 확인 상태로 전환했어요\./i),
    ).toBeInTheDocument()
    expect(screen.queryByText(/preview-render-blocked/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/C:\\render-worker\\stderr\.log/i)).not.toBeInTheDocument()
  })

  it('shows preview architecture diagnostics for the active session', async () => {
    const loadOperatorRecoverySummary = vi.fn().mockResolvedValue(
      createOperatorRecoverySummary({
        previewArchitecture: {
          route: 'actual-primary-lane',
          routeStage: 'canary',
          implementationTrack: 'actual-primary-lane',
          laneOwner: 'local-fullscreen-lane',
          fallbackReasonCode: 'none',
          firstVisibleMs: 1280,
          sameCaptureFullScreenVisibleMs: 1840,
          replacementMs: 9999,
          originalVisibleToPresetAppliedVisibleMs: 560,
          hardwareCapability: 'dedicated-renderer-available',
          warmState: 'warm-ready',
          warmStateObservedAt: '2026-04-12T08:00:00.000Z',
        },
      }),
    )
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    expect(await screen.findByText(/Preview Architecture/i)).toBeInTheDocument()
    expect(screen.getAllByText(/^actual-primary-lane$/i)).toHaveLength(2)
    expect(screen.getByText(/^local-fullscreen-lane$/i)).toBeInTheDocument()
    expect(screen.getByText(/canary/i)).toBeInTheDocument()
    expect(screen.getByText(/dedicated-renderer-available/i)).toBeInTheDocument()
    expect(screen.getByText(/warm-ready/i)).toBeInTheDocument()
    expect(screen.getByText(/^Same-Capture Full Screen$/i)).toBeInTheDocument()
    expect(screen.getByText(/1\.3초 \(1280ms\)/i)).toBeInTheDocument()
    expect(screen.getByText(/1\.8초 \(1840ms\)/i)).toBeInTheDocument()
    expect(screen.getByText(/0\.6초 \(560ms\)/i)).toBeInTheDocument()
    expect(screen.getByText(/목표 2\.5초 이하 · 현재 1\.8초/i)).toBeInTheDocument()
    expect(screen.queryByText(/9\.9초 \(9999ms\)/i)).not.toBeInTheDocument()
  })

  it('does not treat slot replacement timing as the same-capture full-screen KPI', async () => {
    const loadOperatorRecoverySummary = vi.fn().mockResolvedValue(
      createOperatorRecoverySummary({
        previewArchitecture: {
          route: 'local-renderer-sidecar',
          routeStage: 'canary',
          implementationTrack: 'prototype-track',
          laneOwner: 'local-fullscreen-lane',
          fallbackReasonCode: 'none',
          firstVisibleMs: 1280,
          sameCaptureFullScreenVisibleMs: null,
          replacementMs: 9999,
          originalVisibleToPresetAppliedVisibleMs: 560,
          hardwareCapability: 'dedicated-renderer-available',
          warmState: 'warm-ready',
          warmStateObservedAt: '2026-04-12T08:00:00.000Z',
        },
      }),
    )
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    expect(await screen.findByText(/Preview Architecture/i)).toBeInTheDocument()
    expect(screen.getByText(/^prototype-track$/i)).toBeInTheDocument()
    expect(screen.getByText(/^Same-Capture Full Screen$/i)).toBeInTheDocument()
    expect(screen.getByText(/^Slot Replacement$/i)).toBeInTheDocument()
    expect(screen.getByText(/목표 2\.5초 이하 · 아직 계측 없음/i)).toBeInTheDocument()
    expect(screen.queryByText(/9\.9초 \(9999ms\)/i)).not.toBeInTheDocument()
  })

  it('renders a safe empty state when no active session exists', async () => {
    const loadOperatorRecoverySummary = vi.fn().mockResolvedValue(
      createOperatorRecoverySummary({
        state: 'no-session',
        blockedStateCategory: 'not-blocked',
        blockedCategory: null,
        diagnosticsSummary: null,
        allowedActions: [],
        sessionId: null,
        boothAlias: null,
        activePresetId: null,
        activePresetDisplayName: null,
        activePresetVersion: null,
        lifecycleStage: null,
        timingPhase: null,
        updatedAt: null,
        postEndState: null,
        recentFailure: null,
        cameraConnection: {
          state: 'disconnected',
          title: '세션 없음',
          detail: '진행 중인 세션이 없어 카메라 연결 상태를 아직 판단하지 않았어요.',
          observedAt: null,
        },
        captureBoundary: {
          status: 'clear',
          title: '현재 세션 없음',
          detail: '진행 중인 세션이 없어 capture 경계를 아직 판단하지 않았어요.',
        },
        previewRenderBoundary: {
          status: 'clear',
          title: '최근 결과 없음',
          detail: '진행 중인 세션이 없어 preview/render 경계도 아직 비어 있어요.',
        },
        completionBoundary: {
          status: 'clear',
          title: '후처리 경계 비어 있음',
          detail: '현재 세션이 시작되면 completion 경계 진단을 함께 보여 드릴게요.',
        },
      }),
    )
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    expect(await screen.findByText(/진행 중인 세션이 아직 없어요/i)).toBeInTheDocument()
    expect(screen.getByText(/현재 세션 없음/i)).toBeInTheDocument()
    expect(screen.getByText(/카메라 연결 상태를 아직 판단하지 않았어요\./i)).toBeInTheDocument()
    expect(screen.getByText(/후처리 경계 비어 있음/i)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Retry$/i })).not.toBeInTheDocument()
  })

  it('keeps the newest refresh result when an older response resolves late', async () => {
    const first = createDeferred<OperatorRecoverySummaryFixture>()
    const second = createDeferred<OperatorRecoverySummaryFixture>()
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise)
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    const user = userEvent.setup()

    await screen.findByRole('heading', { name: /Operator Console/i })
    await user.click(screen.getByRole('button', { name: /새로고침/i }))

    second.resolve(
      createOperatorRecoverySummary({
        boothAlias: 'Kim 2222',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
      }),
    )

    expect(await screen.findByText('Kim 2222')).toBeInTheDocument()

    first.resolve(
      createOperatorRecoverySummary({
        boothAlias: 'Kim 1111',
      }),
    )

    await waitFor(() => {
      expect(screen.getByText('Kim 2222')).toBeInTheDocument()
    })
    expect(screen.queryByText('Kim 1111')).not.toBeInTheDocument()
  })

  it('runs an allowed action and updates the operator-safe next state result', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValue(createOperatorRecoverySummary())
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    const user = userEvent.setup()

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^Retry$/i }))

    expect(runOperatorRecoveryAction).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      action: 'retry',
    })
    expect(
      await screen.findByText(/현재 막힌 preview\/render 경계를 다시 시도했어요\./i),
    ).toBeInTheDocument()
    expect(screen.getByText(/다음 booth 상태: Ready/i)).toBeInTheDocument()
    expect(screen.getByText(/상태 이유: 바로 다음 촬영이나 안내로 이어질 수 있어요\./i)).toBeInTheDocument()
    expect(screen.queryByText(/거절 사유:/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/^ready$/i)).not.toBeInTheDocument()
  })

  it('clears the previous recovery result after refreshing into a newer session summary', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValueOnce(createOperatorRecoverySummary())
      .mockResolvedValueOnce(
        createOperatorRecoverySummary({
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          boothAlias: 'Kim 2222',
          blockedStateCategory: 'not-blocked',
          blockedCategory: null,
          diagnosticsSummary: null,
          allowedActions: [],
        }),
      )
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    const user = userEvent.setup()

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^Retry$/i }))

    expect(
      await screen.findByText(/현재 막힌 preview\/render 경계를 다시 시도했어요\./i),
    ).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /진단 새로고침/i }))

    expect(await screen.findByText('Kim 2222')).toBeInTheDocument()
    expect(
      screen.queryByText(/현재 막힌 preview\/render 경계를 다시 시도했어요\./i),
    ).not.toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { name: /최근 recovery 결과/i }),
    ).not.toBeInTheDocument()
  })

  it('renders operator-safe reason labels for rejected actions instead of raw enums', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValue(createOperatorRecoverySummary())
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi.fn().mockResolvedValue(
      createOperatorRecoveryActionResult({
        status: 'rejected',
        message: '현재 범주에서는 선택한 액션을 실행하지 않았어요.',
        rejectionReason: 'action-not-allowed',
        nextState: {
          customerState: 'Preview Waiting',
          reasonCode: 'preview-waiting',
          lifecycleStage: 'preview-waiting',
          timingPhase: 'active',
          postEndState: null,
        },
      }),
    )

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    const user = userEvent.setup()

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^Retry$/i }))

    expect(await screen.findByText(/거절 사유: 현재 범주에서는 허용되지 않은 액션이에요\./i)).toBeInTheDocument()
    expect(screen.getByText(/상태 이유: 최근 촬영본의 결과를 준비하는 중이에요\./i)).toBeInTheDocument()
    expect(screen.queryByText(/action-not-allowed/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/preview-waiting/i)).not.toBeInTheDocument()
  })

  it('keeps camera connection connected while preview/render remains blocked', async () => {
    const loadOperatorRecoverySummary = vi.fn().mockResolvedValue(
      createOperatorRecoverySummary({
        blockedStateCategory: 'preview-render-blocked',
        cameraConnection: {
          state: 'connected',
          title: '카메라와 helper 연결이 확인됐어요.',
          detail: '카메라와 helper는 정상 연결 상태이고 다른 경계 문제는 별도로 확인해 주세요.',
          observedAt: '2026-03-26T00:10:00.000Z',
        },
      }),
    )
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    expect(await screen.findByText(/카메라 연결 상태/i)).toBeInTheDocument()
    expect(screen.getByText(/^연결됨$/i)).toBeInTheDocument()
    expect(screen.getAllByText(/Preview \/ Render 확인 필요/i)).not.toHaveLength(0)
    expect(screen.getByText(/다른 경계 문제는 별도로 확인해 주세요\./i)).toBeInTheDocument()
  })

  it('keeps current session diagnostics visible when audit history refresh fails', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValue(createOperatorRecoverySummary())
    const loadOperatorAuditHistory = vi.fn().mockRejectedValue({
      code: 'host-unavailable',
      message: '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
    })
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /^현재 세션 문맥$/i }),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { name: /^현재 세션 진단을 다시 불러와 주세요$/i }),
    ).not.toBeInTheDocument()
    expect(screen.getByText(/감사 기록을 불러오지 못했어요/i)).toBeInTheDocument()
    expect(
      screen.getByText(/세션 진단과 허용 액션은 계속 확인할 수 있어요\./i),
    ).toBeInTheDocument()
  })

  it('keeps the applied recovery result even when the audit history reload fails', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValue(createOperatorRecoverySummary())
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValueOnce(createOperatorAuditHistory())
      .mockRejectedValueOnce({
        code: 'host-unavailable',
        message: '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
      })
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    const user = userEvent.setup()

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^Retry$/i }))

    expect(
      await screen.findByText(/현재 막힌 preview\/render 경계를 다시 시도했어요\./i),
    ).toBeInTheDocument()
    expect(screen.getByText(/다음 booth 상태: Ready/i)).toBeInTheDocument()
    expect(screen.getByText(/감사 기록을 불러오지 못했어요/i)).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: /^Retry$/i }),
    ).not.toBeInTheDocument()
  })

  it('keeps the last known session visible when refresh fails after a successful load', async () => {
    const loadOperatorRecoverySummary = vi
      .fn()
      .mockResolvedValueOnce(createOperatorRecoverySummary())
      .mockRejectedValueOnce({
        code: 'host-unavailable',
        message: '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
      })
    const loadOperatorAuditHistory = vi
      .fn()
      .mockResolvedValue(createOperatorAuditHistory())
    const runOperatorRecoveryAction = vi
      .fn()
      .mockResolvedValue(createOperatorRecoveryActionResult())

    renderOperatorScreen({
      loadOperatorRecoverySummary,
      loadOperatorAuditHistory,
      runOperatorRecoveryAction,
    })

    const user = userEvent.setup()

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /진단 새로고침/i }))

    expect(await screen.findByText('Kim 4821')).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /^현재 세션 문맥$/i }),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { name: /^현재 세션 진단을 다시 불러와 주세요$/i }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByText(/지금은 현재 세션 진단을 불러올 수 없어요\. 잠시 후 다시 시도해 주세요\./i),
    ).toBeInTheDocument()
    const retryButton = screen.getByRole('button', { name: /^Retry$/i })
    expect(retryButton).toBeEnabled()

    await user.click(retryButton)

    expect(runOperatorRecoveryAction).toHaveBeenCalledWith({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      action: 'retry',
    })
  })
})
