import { describe, expect, it } from 'vitest'

import type { CaptureReadinessSnapshot } from '../../shared-contracts'
import { selectCustomerStatusCopy } from './customerStatusCopy'

function createReadiness(
  overrides: Partial<CaptureReadinessSnapshot>,
): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    surfaceState: 'blocked',
    customerState: 'Preparing',
    canCapture: false,
    primaryAction: 'wait',
    customerMessage: '촬영 준비 중이에요.',
    supportMessage: '잠시만 기다려 주세요.',
    reasonCode: 'camera-preparing',
    latestCapture: null,
    postEnd: null,
    timing: undefined,
    ...overrides,
  }
}

describe('customerStatusCopy', () => {
  it('maps a ready snapshot to one customer action label', () => {
    const copy = selectCustomerStatusCopy(
      createReadiness({
        customerState: 'Ready',
        canCapture: true,
        primaryAction: 'capture',
        customerMessage: '지금 촬영할 수 있어요.',
        supportMessage: '버튼을 누르면 바로 시작돼요.',
        reasonCode: 'ready',
      }),
    )

    expect(copy.stateLabel).toBe('Ready')
    expect(copy.headline).toBe('지금 촬영할 수 있어요.')
    expect(copy.detail).toBe('버튼을 누르면 바로 시작돼요.')
    expect(copy.actionLabel).toBe('사진 찍기')
    expect(copy.canCapture).toBe(true)
  })

  it('keeps phone-required guidance customer-safe and non-technical', () => {
    const copy = selectCustomerStatusCopy(
      createReadiness({
        customerState: 'Phone Required',
        primaryAction: 'call-support',
        customerMessage: '지금은 도움이 필요해요.',
        supportMessage: '가까운 직원에게 알려 주세요.',
        reasonCode: 'phone-required',
      }),
    )

    expect(copy.actionLabel).toBe('도움 요청')
    expect(copy.headline).not.toMatch(/darktable|sdk|helper/i)
    expect(copy.detail).not.toMatch(/darktable|sdk|helper/i)
  })

  it('keeps handoff-ready completion on a generic confirmation action in story 3.2', () => {
    const copy = selectCustomerStatusCopy(
      createReadiness({
        customerState: 'Completed',
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
    )

    expect(copy.actionLabel).toBe('안내 확인')
    expect(copy.isPostEndFinalized).toBe(true)
  })

  it('falls back to manifest post-end guidance when readiness only carries the explicit reason', () => {
    const copy = selectCustomerStatusCopy(
      createReadiness({
        customerState: 'Completed',
        primaryAction: 'wait',
        customerMessage: '부스 준비가 끝났어요.',
        supportMessage: '마지막 안내를 확인해 주세요.',
        reasonCode: 'completed',
        postEnd: null,
      }),
      {
        state: 'completed',
        evaluatedAt: '2026-03-20T00:00:10.000Z',
        completionVariant: 'handoff-ready',
        approvedRecipientLabel: 'Front Desk',
        primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
        supportActionLabel: null,
        showBoothAlias: true,
      },
    )

    expect(copy.actionLabel).toBe('안내 확인')
    expect(copy.postEnd?.state).toBe('completed')
  })
})
