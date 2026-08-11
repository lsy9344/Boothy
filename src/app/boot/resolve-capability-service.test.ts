import { describe, expect, it, vi } from 'vitest'

import type { CapabilitySnapshot } from '../../shared-contracts'
import { resolveBootCapabilityService } from './resolve-capability-service'

const ADMIN_SNAPSHOT: CapabilitySnapshot = {
  isAdminAuthenticated: true,
  allowedSurfaces: ['booth', 'operator', 'authoring', 'settings'],
}

/**
 * HV-13A 2026-08-11 회귀 방지.
 * 재생성된 관람 창이 흰 화면에 머문 원인 중 하나는 부팅이 host IPC 응답을 무기한 기다린 것이었다.
 * host가 멈춰도 UI는 반드시 그려져야 한다.
 */
describe('resolveBootCapabilityService', () => {
  it('관람 창은 host IPC를 아예 기다리지 않는다', async () => {
    const readSnapshot = vi.fn(() => new Promise<CapabilitySnapshot>(() => {}))

    const service = await resolveBootCapabilityService({
      currentWindowLabel: 'viewer-window',
      readSnapshot,
    })

    expect(readSnapshot).not.toHaveBeenCalled()
    expect(service.canAccess('viewer')).toBe(true)
  })

  it('host가 영원히 응답하지 않아도 제한 시간 뒤 기본 권한으로 진행한다', async () => {
    vi.useFakeTimers()

    try {
      const service = resolveBootCapabilityService({
        currentWindowLabel: 'booth-window',
        readSnapshot: () => new Promise<CapabilitySnapshot>(() => {}),
        timeoutMs: 3_000,
      })

      await vi.advanceTimersByTimeAsync(3_000)

      const resolved = await service

      expect(resolved.canAccess('booth')).toBe(true)
      expect(resolved.canAccess('operator')).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })

  it('host가 실패해도 기본 권한으로 진행한다', async () => {
    const service = await resolveBootCapabilityService({
      currentWindowLabel: 'booth-window',
      readSnapshot: () => Promise.reject(new Error('host-unavailable')),
    })

    expect(service.canAccess('booth')).toBe(true)
  })

  it('정상 응답이면 host snapshot 권한을 반영한다', async () => {
    const service = await resolveBootCapabilityService({
      currentWindowLabel: 'operator-window',
      readSnapshot: () => Promise.resolve(ADMIN_SNAPSHOT),
    })

    expect(service.canAccess('operator')).toBe(true)
  })
})
