import { describe, expect, it } from 'vitest'

import {
  operatorAuditEntrySchema,
  operatorAuditEventCategorySchema,
  operatorAuditQueryFilterSchema,
  operatorAuditQueryResultSchema,
} from './index'

function createAuditEntry(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-audit-entry/v1',
    eventId: 'audit_20260327_0001',
    occurredAt: '2026-03-27T01:10:00.000Z',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    eventCategory: 'operator-intervention',
    eventType: 'approved-boundary-restart',
    summary: '승인된 범위 안에서 preview/render 경계를 다시 시작했어요.',
    detail: '최근 세션 결과를 다시 준비할 수 있도록 허용된 경계만 재시작했어요.',
    actorId: 'operator-kim',
    source: 'operator-console',
    captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
    presetId: 'preset_soft-glow',
    publishedVersion: '2026.03.26',
    reasonCode: null,
    ...overrides,
  }
}

describe('operator audit contracts', () => {
  it('parses a typed audit entry and session-filtered query result', () => {
    const parsedEntry = operatorAuditEntrySchema.parse(createAuditEntry())
    const parsedFilter = operatorAuditQueryFilterSchema.parse({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      eventCategories: ['operator-intervention', 'critical-failure'],
      limit: 10,
    })
    const parsedResult = operatorAuditQueryResultSchema.parse({
      schemaVersion: 'operator-audit-query-result/v1',
      filter: parsedFilter,
      events: [
        createAuditEntry(),
        createAuditEntry({
          eventId: 'audit_20260327_0002',
          occurredAt: '2026-03-27T01:05:00.000Z',
          eventCategory: 'critical-failure',
          eventType: 'post-end-phone-required',
          summary: '종료 후 자동 완료를 멈추고 직원 확인 상태로 전환했어요.',
          detail: '후처리 결과가 안전 기준을 벗어나 Phone Required 보호 상태로 잠겼어요.',
          actorId: null,
          source: 'post-end-evaluator',
          captureId: null,
          reasonCode: 'render-failed',
        }),
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
          occurredAt: '2026-03-27T01:05:00.000Z',
          eventCategory: 'critical-failure',
          eventType: 'post-end-phone-required',
          summary: '종료 후 자동 완료를 멈추고 직원 확인 상태로 전환했어요.',
        },
      },
    })

    expect(parsedEntry.eventType).toBe('approved-boundary-restart')
    expect(parsedResult.events).toHaveLength(2)
    expect(parsedResult.summary.criticalFailureEvents).toBe(1)
    expect(parsedResult.filter.sessionId).toBe('session_01hs6n1r8b8zc5v4ey2x7b9g1m')
  })

  it('rejects unsafe audit categories or inconsistent summary counts', () => {
    expect(() => operatorAuditEventCategorySchema.parse('raw-helper-dump')).toThrow()

    expect(() =>
      operatorAuditEntrySchema.parse(
        createAuditEntry({
          detail:
            'C:/boothy/sessions/session_01/render-worker/stderr.log '.repeat(8),
        }),
      ),
    ).toThrow()

    expect(() =>
      operatorAuditQueryResultSchema.parse({
        schemaVersion: 'operator-audit-query-result/v1',
        filter: {
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          limit: 10,
        },
        events: [createAuditEntry()],
        summary: {
          totalEvents: 1,
          sessionLifecycleEvents: 0,
          timingTransitionEvents: 0,
          postEndOutcomeEvents: 0,
          operatorInterventionEvents: 0,
          publicationRecoveryEvents: 0,
          criticalFailureEvents: 0,
          latestOutcome: {
            occurredAt: '2026-03-27T01:10:00.000Z',
            eventCategory: 'operator-intervention',
            eventType: 'approved-boundary-restart',
            summary: '승인된 범위 안에서 preview/render 경계를 다시 시작했어요.',
          },
        },
      }),
    ).toThrow(/totalEvents/i)
  })

  it('keeps host-generated audit event ids within the frontend contract budget', () => {
    expect(() =>
      operatorAuditEntrySchema.parse(
        createAuditEntry({
          eventId: 'audit-20260327T001000-0000000a-approvedboundaryrest-session',
        }),
      ),
    ).not.toThrow()

    expect(() =>
      operatorAuditEntrySchema.parse(
        createAuditEntry({
          eventId:
            'audit-20260327T001000-0000000a-approvedboundaryrestart-sessiontoolong-extra',
        }),
      ),
    ).toThrow()
  })
})
