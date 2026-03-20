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
})
