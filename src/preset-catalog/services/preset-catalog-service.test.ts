import { afterEach, describe, expect, it } from 'vitest'

import {
  createBrowserPresetCatalogGateway,
  createPresetCatalogService,
} from './preset-catalog-service'

describe('browser preset catalog gateway', () => {
  afterEach(() => {
    delete (
      globalThis as typeof globalThis & {
        __BOOTHY_BROWSER_PRESET_CATALOG__?: unknown
      }
    ).__BOOTHY_BROWSER_PRESET_CATALOG__
  })

  it('uses an injected browser fixture when one is available', async () => {
    ;(
      globalThis as typeof globalThis & {
        __BOOTHY_BROWSER_PRESET_CATALOG__?: unknown
      }
    ).__BOOTHY_BROWSER_PRESET_CATALOG__ = {
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      state: 'ready',
      presets: [
        {
          presetId: 'preset_soft-glow',
          displayName: 'Soft Glow',
          publishedVersion: '2026.03.20',
          boothStatus: 'booth-safe',
          preview: {
            kind: 'preview-tile',
            assetPath: 'fixtures/soft-glow.jpg',
            altText: 'Soft Glow sample portrait',
          },
        },
      ],
    }

    const result = await createBrowserPresetCatalogGateway().loadPresetCatalog({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    })

    expect(result).toMatchObject({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      state: 'ready',
    })
  })

  it('surfaces a clear host-unavailable error when no browser fixture exists', async () => {
    await expect(
      createBrowserPresetCatalogGateway().loadPresetCatalog({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })

  it('normalizes malformed browser fixtures into the host error envelope', async () => {
    ;(
      globalThis as typeof globalThis & {
        __BOOTHY_BROWSER_PRESET_CATALOG__?: unknown
      }
    ).__BOOTHY_BROWSER_PRESET_CATALOG__ = {
      sessionId: 'bad-session',
      state: 'ready',
      presets: [],
    }

    await expect(
      createBrowserPresetCatalogGateway().loadPresetCatalog({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '브라우저 프리셋 카탈로그 fixture 형식이 올바르지 않아요.',
    })
  })

  it('rejects host responses that do not match the requested session id', async () => {
    const service = createPresetCatalogService({
      gateway: {
        async loadPresetCatalog() {
          return {
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            state: 'ready',
            presets: [
              {
                presetId: 'preset_soft-glow',
                displayName: 'Soft Glow',
                publishedVersion: '2026.03.20',
                boothStatus: 'booth-safe',
                preview: {
                  kind: 'preview-tile',
                  assetPath: 'fixtures/soft-glow.jpg',
                  altText: 'Soft Glow sample portrait',
                },
              },
            ],
          }
        },
      },
    })

    await expect(
      service.loadPresetCatalog({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '요청한 세션과 다른 프리셋 카탈로그 응답을 받았어요. 다시 시도해 주세요.',
    })
  })
})
