import { describe, expect, it } from 'vitest'

import {
  branchRolloutActionResultSchema,
  branchRolloutOverviewResultSchema,
} from './index'

function createReleaseBaseline(overrides: Record<string, unknown> = {}) {
  return {
    buildVersion: 'boothy-2026.03.27.1',
    presetStackVersion: 'catalog-2026.03.27',
    approvedAt: '2026-03-27T00:10:00.000Z',
    actorId: 'release-kim',
    actorLabel: 'Kim Release',
    ...overrides,
  }
}

function createBranchState(overrides: Record<string, unknown> = {}) {
  return {
    branchId: 'gangnam-01',
    displayName: '강남 1호점',
    deploymentBaseline: createReleaseBaseline(),
    rollbackBaseline: createReleaseBaseline({
      buildVersion: 'boothy-2026.03.20.4',
      presetStackVersion: 'catalog-2026.03.20',
      approvedAt: '2026-03-20T00:10:00.000Z',
    }),
    pendingBaseline: null,
    localSettings: {
      preservedFields: ['contact-phone', 'contact-email', 'bounded-operational-toggle'],
      summary: '지점 연락처와 운영 토글은 그대로 유지돼요.',
    },
    activeSession: null,
    ...overrides,
  }
}

function createBranchOutcome(overrides: Record<string, unknown> = {}) {
  return {
    branchId: 'gangnam-01',
    displayName: '강남 1호점',
    result: 'applied',
    effectiveBaseline: createReleaseBaseline(),
    pendingBaseline: null,
    localSettings: {
      preservedFields: ['contact-phone', 'contact-email'],
      summary: '승인된 연락처 설정은 그대로 유지돼요.',
    },
    compatibility: {
      status: 'compatible',
      summary: '현재 세션을 끊지 않고 바로 적용할 수 있어요.',
      sessionBaseline: null,
      safeTransitionRequired: false,
    },
    rejection: null,
    ...overrides,
  }
}

function createAuditEntry(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'branch-rollout-audit-entry/v1',
    auditId: 'branch-rollout-20260327-0001',
    action: 'rollout',
    requestedBranchIds: ['gangnam-01'],
    targetBaseline: createReleaseBaseline(),
    approval: {
      approvedAt: '2026-03-27T00:10:00.000Z',
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    },
    outcomes: [createBranchOutcome()],
    notedAt: '2026-03-27T00:10:01.000Z',
    ...overrides,
  }
}

describe('branch rollout contracts', () => {
  it('parses rollout overview with deployment baseline, pending baseline, and recent history', () => {
    const parsed = branchRolloutOverviewResultSchema.parse({
      schemaVersion: 'branch-rollout-overview/v1',
      approvedBaselines: [
        createReleaseBaseline(),
        createReleaseBaseline({
          buildVersion: 'boothy-2026.03.20.4',
          presetStackVersion: 'catalog-2026.03.20',
          approvedAt: '2026-03-20T00:10:00.000Z',
        }),
      ],
      branches: [
        createBranchState({
          activeSession: {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
            lockedBaseline: createReleaseBaseline({
              buildVersion: 'boothy-2026.03.20.4',
              presetStackVersion: 'catalog-2026.03.20',
              approvedAt: '2026-03-20T00:10:00.000Z',
            }),
            startedAt: '2026-03-27T00:00:00.000Z',
            safeTransition: 'after-session-end',
          },
          pendingBaseline: createReleaseBaseline(),
        }),
      ],
      recentHistory: [createAuditEntry()],
    })

    expect(parsed.branches[0]?.branchId).toBe('gangnam-01')
    expect(parsed.branches[0]?.pendingBaseline?.buildVersion).toBe('boothy-2026.03.27.1')
    expect(parsed.recentHistory[0]?.action).toBe('rollout')
  })

  it('parses deferred rollout results with safe-transition compatibility guidance', () => {
    const parsed = branchRolloutActionResultSchema.parse({
      schemaVersion: 'branch-rollout-action-result/v1',
      action: 'rollout',
      requestedBranchIds: ['gangnam-01'],
      targetBaseline: createReleaseBaseline(),
      approval: {
        approvedAt: '2026-03-27T00:10:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
      outcomes: [
        createBranchOutcome({
          result: 'deferred',
          pendingBaseline: createReleaseBaseline(),
          compatibility: {
            status: 'deferred-until-safe-transition',
            summary: '진행 중인 세션은 기존 baseline을 유지하고 종료 후에만 전환돼요.',
            sessionBaseline: createReleaseBaseline({
              buildVersion: 'boothy-2026.03.20.4',
              presetStackVersion: 'catalog-2026.03.20',
              approvedAt: '2026-03-20T00:10:00.000Z',
            }),
            safeTransitionRequired: true,
          },
          rejection: {
            code: 'active-session-deferred',
            message: '진행 중인 세션이 있어 지금은 바로 전환하지 않았어요.',
            guidance: '세션 종료 후 다시 확인하면 staged rollout이 자동 적용돼요.',
          },
        }),
      ],
      auditEntry: createAuditEntry({
        outcomes: [
          createBranchOutcome({
            result: 'deferred',
            pendingBaseline: createReleaseBaseline(),
            compatibility: {
              status: 'deferred-until-safe-transition',
              summary: '진행 중인 세션은 기존 baseline을 유지하고 종료 후에만 전환돼요.',
              sessionBaseline: createReleaseBaseline({
                buildVersion: 'boothy-2026.03.20.4',
                presetStackVersion: 'catalog-2026.03.20',
                approvedAt: '2026-03-20T00:10:00.000Z',
              }),
              safeTransitionRequired: true,
            },
            rejection: {
              code: 'active-session-deferred',
              message: '진행 중인 세션이 있어 지금은 바로 전환하지 않았어요.',
              guidance: '세션 종료 후 다시 확인하면 staged rollout이 자동 적용돼요.',
            },
          }),
        ],
      }),
      message: '선택한 지점 중 진행 중인 세션이 있는 곳은 staged rollout으로 보류했어요.',
    })

    expect(parsed.outcomes[0]?.result).toBe('deferred')
    expect(parsed.outcomes[0]?.compatibility.status).toBe(
      'deferred-until-safe-transition',
    )
    expect(parsed.auditEntry.outcomes).toHaveLength(1)
  })

  it('rejects duplicate branch selection and malformed preservation summaries', () => {
    expect(() =>
      branchRolloutActionResultSchema.parse({
        schemaVersion: 'branch-rollout-action-result/v1',
        action: 'rollback',
        requestedBranchIds: ['gangnam-01', 'gangnam-01'],
        targetBaseline: null,
        approval: {
          approvedAt: '2026-03-27T00:10:00.000Z',
          actorId: 'release-kim',
          actorLabel: 'Kim Release',
        },
        outcomes: [
          createBranchOutcome({
            result: 'rejected',
            localSettings: {
              preservedFields: [],
              summary: '',
            },
            compatibility: {
              status: 'incompatible',
              summary: 'rollback baseline을 확인하지 못했어요.',
              sessionBaseline: null,
              safeTransitionRequired: false,
            },
            rejection: {
              code: 'missing-rollback-baseline',
              message: '되돌릴 승인 baseline이 아직 없어요.',
              guidance: '먼저 승인된 rollout 이력이 있는지 확인해 주세요.',
            },
          }),
        ],
        auditEntry: createAuditEntry({
          action: 'rollback',
          targetBaseline: null,
        }),
        message: 'rollback을 진행하지 않았어요.',
      }),
    ).toThrow()
  })
})
