import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import type { BranchRolloutService } from '../../branch-config/services/branch-rollout-service'
import { SettingsScreen } from './SettingsScreen'

function createOverview() {
  return {
    schemaVersion: 'branch-rollout-overview/v1',
    approvedBaselines: [
      {
        buildVersion: 'boothy-2026.03.27.1',
        presetStackVersion: 'catalog-2026.03.27',
        approvedAt: '2026-03-27T00:10:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
      {
        buildVersion: 'boothy-2026.03.20.4',
        presetStackVersion: 'catalog-2026.03.20',
        approvedAt: '2026-03-20T00:10:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
    ],
    branches: [
      {
        branchId: 'gangnam-01',
        displayName: '강남 1호점',
        deploymentBaseline: {
          buildVersion: 'boothy-2026.03.20.4',
          presetStackVersion: 'catalog-2026.03.20',
          approvedAt: '2026-03-20T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        rollbackBaseline: {
          buildVersion: 'boothy-2026.03.13.2',
          presetStackVersion: 'catalog-2026.03.13',
          approvedAt: '2026-03-13T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        pendingBaseline: null,
        localSettings: {
          preservedFields: ['contact-phone', 'bounded-operational-toggle'],
          summary: '지점 연락처와 승인된 운영 토글은 그대로 유지돼요.',
        },
        activeSession: null,
      },
      {
        branchId: 'hongdae-02',
        displayName: '홍대 2호점',
        deploymentBaseline: {
          buildVersion: 'boothy-2026.03.20.4',
          presetStackVersion: 'catalog-2026.03.20',
          approvedAt: '2026-03-20T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        rollbackBaseline: {
          buildVersion: 'boothy-2026.03.13.2',
          presetStackVersion: 'catalog-2026.03.13',
          approvedAt: '2026-03-13T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        pendingBaseline: {
          buildVersion: 'boothy-2026.03.27.1',
          presetStackVersion: 'catalog-2026.03.27',
          approvedAt: '2026-03-27T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        localSettings: {
          preservedFields: ['contact-phone', 'contact-email'],
          summary: '지점별 연락처는 그대로 유지돼요.',
        },
        activeSession: {
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          lockedBaseline: {
            buildVersion: 'boothy-2026.03.20.4',
            presetStackVersion: 'catalog-2026.03.20',
            approvedAt: '2026-03-20T00:10:00.000Z',
            actorId: 'release-kim',
            actorLabel: 'Kim Release',
          },
          startedAt: '2026-03-27T00:00:00.000Z',
          safeTransition: 'after-session-end',
        },
      },
    ],
    recentHistory: [],
  }
}

function createActionResult() {
  return {
    schemaVersion: 'branch-rollout-action-result/v1',
    action: 'rollout',
    requestedBranchIds: ['gangnam-01', 'hongdae-02'],
    targetBaseline: {
      buildVersion: 'boothy-2026.03.27.1',
      presetStackVersion: 'catalog-2026.03.27',
      approvedAt: '2026-03-27T00:10:00.000Z',
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    },
    approval: {
      approvedAt: '2026-03-27T00:10:00.000Z',
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    },
    outcomes: [
      {
        branchId: 'gangnam-01',
        displayName: '강남 1호점',
        result: 'applied',
        effectiveBaseline: {
          buildVersion: 'boothy-2026.03.27.1',
          presetStackVersion: 'catalog-2026.03.27',
          approvedAt: '2026-03-27T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        pendingBaseline: null,
        localSettings: {
          preservedFields: ['contact-phone', 'bounded-operational-toggle'],
          summary: '지점 연락처와 승인된 운영 토글은 그대로 유지돼요.',
        },
        compatibility: {
          status: 'compatible',
          summary: '지금 바로 전환할 수 있어요.',
          sessionBaseline: null,
          safeTransitionRequired: false,
        },
        rejection: null,
      },
      {
        branchId: 'hongdae-02',
        displayName: '홍대 2호점',
        result: 'deferred',
        effectiveBaseline: {
          buildVersion: 'boothy-2026.03.20.4',
          presetStackVersion: 'catalog-2026.03.20',
          approvedAt: '2026-03-20T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        pendingBaseline: {
          buildVersion: 'boothy-2026.03.27.1',
          presetStackVersion: 'catalog-2026.03.27',
          approvedAt: '2026-03-27T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        localSettings: {
          preservedFields: ['contact-phone', 'contact-email'],
          summary: '지점별 연락처는 그대로 유지돼요.',
        },
        compatibility: {
          status: 'deferred-until-safe-transition',
          summary: '세션 종료 후에만 새 baseline이 적용돼요.',
          sessionBaseline: {
            buildVersion: 'boothy-2026.03.20.4',
            presetStackVersion: 'catalog-2026.03.20',
            approvedAt: '2026-03-20T00:10:00.000Z',
            actorId: 'release-kim',
            actorLabel: 'Kim Release',
          },
          safeTransitionRequired: true,
        },
        rejection: {
          code: 'active-session-deferred',
          message: '진행 중인 세션이 있어 지금은 바로 전환하지 않았어요.',
          guidance: '세션 종료 후 staged rollout이 적용돼요.',
        },
      },
    ],
    auditEntry: {
      schemaVersion: 'branch-rollout-audit-entry/v1',
      auditId: 'branch-rollout-20260327-0001',
      action: 'rollout',
      requestedBranchIds: ['gangnam-01', 'hongdae-02'],
      targetBaseline: {
        buildVersion: 'boothy-2026.03.27.1',
        presetStackVersion: 'catalog-2026.03.27',
        approvedAt: '2026-03-27T00:10:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
      approval: {
        approvedAt: '2026-03-27T00:10:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
      outcomes: [],
      notedAt: '2026-03-27T00:10:01.000Z',
    },
    message: '진행 중인 세션이 있는 지점은 staged rollout으로 보류했어요.',
  }
}

function createPreviewRouteResult(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'preview-renderer-route-mutation-result/v1',
    action: 'promote',
    presetId: 'preset_new-draft-2',
    publishedVersion: '2026.04.10',
    routeStage: 'canary',
    approval: {
      approvedAt: '2026-04-14T01:45:00.000Z',
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    },
    auditEntry: {
      schemaVersion: 'preview-renderer-route-policy-audit-entry/v1',
      auditId: 'preview-route-promote-20260414-0001',
      action: 'promote',
      presetId: 'preset_new-draft-2',
      publishedVersion: '2026.04.10',
      targetRouteStage: 'canary',
      approval: {
        approvedAt: '2026-04-14T01:45:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
      result: 'applied',
      canarySuccessCount: 1,
      notedAt: '2026-04-14T01:45:02.000Z',
    },
    message: 'preview route canary 승격을 적용했어요.',
    ...overrides,
  }
}

function createPreviewRouteStatusResult(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'preview-renderer-route-status-result/v1',
    presetId: 'preset_new-draft-2',
    publishedVersion: '2026.04.10',
    routeStage: 'canary',
    resolvedRoute: 'local-renderer-sidecar',
    reason: 'operator-canary',
    message: '이 프리셋 버전은 canary 상태예요.',
    ...overrides,
  }
}

function renderScreen(branchRolloutService: BranchRolloutService) {
  render(<SettingsScreen branchRolloutService={branchRolloutService} />)
}

describe('SettingsScreen', () => {
  it('renders rollout controls only on the settings surface and shows local-settings preservation copy', async () => {
    renderScreen({
      loadOverview: vi.fn().mockResolvedValue(createOverview()),
      applyRollout: vi.fn(),
      applyRollback: vi.fn(),
      loadPreviewRendererRouteStatus: vi.fn().mockResolvedValue(
        createPreviewRouteStatusResult(),
      ),
      promotePreviewRendererRoute: vi.fn(),
      rollbackPreviewRendererRoute: vi.fn(),
    })

    expect(
      await screen.findByRole('heading', { name: /Branch Rollout Governance/i }),
    ).toBeInTheDocument()
    expect(screen.getByText(/지점 연락처와 승인된 운영 토글은 그대로 유지돼요\./i)).toBeInTheDocument()
    expect(screen.getByLabelText(/^승인자 ID$/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/대상 지점 선택/i)).toBeInTheDocument()
  })

  it('submits a typed rollout request and shows defer guidance without leaking raw enum names', async () => {
    const loadOverview = vi.fn().mockResolvedValue(createOverview())
    const applyRollout = vi.fn().mockResolvedValue(createActionResult())

    renderScreen({
      loadOverview,
      applyRollout,
      applyRollback: vi.fn(),
      loadPreviewRendererRouteStatus: vi.fn().mockResolvedValue(
        createPreviewRouteStatusResult(),
      ),
      promotePreviewRendererRoute: vi.fn(),
      rollbackPreviewRendererRoute: vi.fn(),
    })

    const user = userEvent.setup()

    await screen.findByRole('heading', { name: /Branch Rollout Governance/i })
    await user.click(screen.getByRole('checkbox', { name: /강남 1호점/i }))
    await user.click(screen.getByRole('checkbox', { name: /홍대 2호점/i }))
    await user.selectOptions(
      screen.getByLabelText(/배포 target baseline/i),
      'boothy-2026.03.27.1 :: catalog-2026.03.27',
    )
    await user.type(screen.getByLabelText(/^승인자 ID$/i), 'release-kim')
    await user.type(screen.getByLabelText(/^승인자 이름$/i), 'Kim Release')
    await user.click(screen.getByRole('button', { name: /선택한 지점에 rollout/i }))

    await waitFor(() => {
      expect(applyRollout).toHaveBeenCalledWith({
        branchIds: ['gangnam-01', 'hongdae-02'],
        targetBaseline: {
          buildVersion: 'boothy-2026.03.27.1',
          presetStackVersion: 'catalog-2026.03.27',
          approvedAt: '2026-03-27T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      })
    })

    expect(
      await screen.findByText(/진행 중인 세션이 있는 지점은 staged rollout으로 보류했어요\./i),
    ).toBeInTheDocument()
    expect(screen.getByText(/세션 종료 후 staged rollout이 적용돼요\./i)).toBeInTheDocument()
    expect(screen.queryByText(/active-session-deferred/i)).not.toBeInTheDocument()
  })

  it('submits preview route promotion from the settings surface', async () => {
    const promotePreviewRendererRoute = vi
      .fn()
      .mockResolvedValue(createPreviewRouteResult())

    renderScreen({
      loadOverview: vi.fn().mockResolvedValue(createOverview()),
      applyRollout: vi.fn(),
      applyRollback: vi.fn(),
      loadPreviewRendererRouteStatus: vi.fn().mockResolvedValue(
        createPreviewRouteStatusResult(),
      ),
      promotePreviewRendererRoute,
      rollbackPreviewRendererRoute: vi.fn(),
    })

    const user = userEvent.setup()

    await screen.findByRole('heading', { name: /Preview Route Governance/i })
    await user.type(screen.getByLabelText(/프리셋 ID/i), 'preset_new-draft-2')
    await user.type(screen.getByLabelText(/게시 버전/i), '2026.04.10')
    await user.selectOptions(screen.getByLabelText(/승격 단계/i), 'canary')
    await user.type(screen.getByLabelText(/Preview route 승인자 ID/i), 'release-kim')
    await user.type(
      screen.getByLabelText(/Preview route 승인자 이름/i),
      'Kim Release',
    )
    await user.click(screen.getByRole('button', { name: /canary 승격 적용/i }))

    await waitFor(() => {
      expect(promotePreviewRendererRoute).toHaveBeenCalledWith({
        presetId: 'preset_new-draft-2',
        publishedVersion: '2026.04.10',
        targetRouteStage: 'canary',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      })
    })

    expect(
      await screen.findByText(/preview route canary 승격을 적용했어요\./i),
    ).toBeInTheDocument()
  })

  it('shows the current preview route stage for the entered preset version', async () => {
    const loadPreviewRendererRouteStatus = vi
      .fn()
      .mockResolvedValue(createPreviewRouteStatusResult())

    renderScreen({
      loadOverview: vi.fn().mockResolvedValue(createOverview()),
      applyRollout: vi.fn(),
      applyRollback: vi.fn(),
      loadPreviewRendererRouteStatus,
      promotePreviewRendererRoute: vi.fn(),
      rollbackPreviewRendererRoute: vi.fn(),
    })

    const user = userEvent.setup()

    await screen.findByRole('heading', { name: /Preview Route Governance/i })
    await user.type(screen.getByLabelText(/프리셋 ID/i), 'preset_new-draft-2')
    await user.type(screen.getByLabelText(/게시 버전/i), '2026.04.10')

    await waitFor(() => {
      expect(loadPreviewRendererRouteStatus).toHaveBeenCalledWith({
        presetId: 'preset_new-draft-2',
        publishedVersion: '2026.04.10',
      })
    })

    expect(await screen.findByText(/현재 상태: canary/i)).toBeInTheDocument()
    expect(screen.getByText(/적용 경로: local-renderer-sidecar/i)).toBeInTheDocument()
    expect(screen.getByText(/이 프리셋 버전은 canary 상태예요\./i)).toBeInTheDocument()
  })
})
