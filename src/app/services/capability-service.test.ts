import { describe, expect, it } from 'vitest'

import { createCapabilityService } from './capability-service'

describe('viewer surface capability 경계', () => {
  it('viewer-window에서는 관리자 인증 없이도 관람 화면에 접근할 수 있다', () => {
    const service = createCapabilityService({
      currentWindowLabel: 'viewer-window',
      isAdminAuthenticated: false,
      allowedSurfaces: ['booth'],
    })

    expect(service.canAccess('viewer')).toBe(true)
    expect(service.canAccess('booth')).toBe(false)
  })

  it('부스 창에서는 관람 화면 라우트를 열 수 없다', () => {
    const service = createCapabilityService({
      currentWindowLabel: 'booth-window',
      isAdminAuthenticated: false,
      allowedSurfaces: ['booth'],
    })

    expect(service.canAccess('viewer')).toBe(false)
  })

  it('window label을 확인하지 못하면 관람 화면을 fail-closed로 유지한다', () => {
    const service = createCapabilityService({
      currentWindowLabel: null,
      isAdminAuthenticated: false,
      allowedSurfaces: ['booth', 'viewer'],
    })

    expect(service.canAccess('viewer')).toBe(false)
  })

  it('운영자 창에서도 관람 화면 라우트를 열 수 없다', () => {
    const service = createCapabilityService({
      currentWindowLabel: 'operator-window',
      isAdminAuthenticated: true,
      allowedSurfaces: ['booth', 'operator'],
    })

    expect(service.canAccess('viewer')).toBe(false)
  })

  it('관람 창이 privileged surface를 열도록 허용하지 않는다', () => {
    const service = createCapabilityService({
      currentWindowLabel: 'viewer-window',
      isAdminAuthenticated: true,
      allowedSurfaces: ['booth', 'operator', 'authoring', 'settings'],
    })

    expect(service.canAccess('operator')).toBe(false)
    expect(service.canAccess('authoring')).toBe(false)
  })

  it('privileged surface는 인증 요구를 유지한다', () => {
    const service = createCapabilityService({
      currentWindowLabel: null,
      isAdminAuthenticated: false,
      allowedSurfaces: ['booth', 'settings'],
    })

    expect(service.canAccess('settings')).toBe(false)
  })
})
