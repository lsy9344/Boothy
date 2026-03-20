import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}))

import {
  createActivePresetService,
  createTauriActivePresetGateway,
} from './active-preset'

describe('active preset service', () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it('rejects mismatched preset selection responses from the host', async () => {
    const service = createActivePresetService({
      gateway: {
        async selectActivePreset() {
          return {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            activePreset: {
              presetId: 'preset_other',
              publishedVersion: '2026.03.21',
            },
            manifest: {
              schemaVersion: 'session-manifest/v1',
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              boothAlias: 'Kim 4821',
              customer: {
                name: 'Kim',
                phoneLastFour: '4821',
              },
              createdAt: '2026-03-20T00:00:00.000Z',
              updatedAt: '2026-03-20T00:05:00.000Z',
              lifecycle: {
                status: 'active',
                stage: 'preset-selected',
              },
              activePreset: {
                presetId: 'preset_other',
                publishedVersion: '2026.03.21',
              },
              captures: [],
              postEnd: null,
            },
          }
        },
      },
    })

    await expect(
      service.selectActivePreset({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        preset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '요청한 세션과 다른 프리셋 선택 응답을 받았어요. 다시 시도해 주세요.',
    })
  })

  it('provides a browser fallback that can persist a selected preset in preview mode', async () => {
    const service = createActivePresetService({
      gateway: undefined,
    })

    const result = await service.selectActivePreset({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      preset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
    })

    expect(result.sessionId).toBe('session_01hs6n1r8b8zc5v4ey2x7b9g1m')
    expect(result.activePreset).toEqual({
      presetId: 'preset_soft-glow',
      publishedVersion: '2026.03.20',
    })
    expect(result.manifest.activePreset).toEqual(result.activePreset)
  })

  it('flattens the selection payload before sending it to the tauri command bridge', async () => {
    invokeMock.mockResolvedValue({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      activePreset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
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
        updatedAt: '2026-03-20T00:05:00.000Z',
        lifecycle: {
          status: 'active',
          stage: 'preset-selected',
        },
        activePreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
        captures: [],
        postEnd: null,
      },
    })

    await createTauriActivePresetGateway().selectActivePreset({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      preset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
    })

    expect(invokeMock).toHaveBeenCalledWith('select_active_preset', {
      input: {
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
    })
  })
})
