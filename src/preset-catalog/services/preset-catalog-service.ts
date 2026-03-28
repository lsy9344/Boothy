import { invoke } from '@tauri-apps/api/core'

import {
  hostErrorEnvelopeSchema,
  loadPresetCatalogInputSchema,
  presetCatalogResultSchema,
  type HostErrorEnvelope,
  type LoadPresetCatalogInput,
  type PresetCatalogResult,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export interface PresetCatalogGateway {
  loadPresetCatalog(input: LoadPresetCatalogInput): Promise<unknown>
}

export interface PresetCatalogService {
  loadPresetCatalog(input: LoadPresetCatalogInput): Promise<PresetCatalogResult>
}

class DefaultPresetCatalogService implements PresetCatalogService {
  private readonly gateway: PresetCatalogGateway

  constructor(gateway: PresetCatalogGateway) {
    this.gateway = gateway
  }

  async loadPresetCatalog(input: LoadPresetCatalogInput) {
    const parsedInput = loadPresetCatalogInputSchema.parse(input)

    try {
      const response = await this.gateway.loadPresetCatalog(parsedInput)
      const parsedResponse = presetCatalogResultSchema.parse(response)

      return ensureMatchingCatalogResponse(parsedInput, parsedResponse)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }
}

function ensureMatchingCatalogResponse(
  input: LoadPresetCatalogInput,
  response: PresetCatalogResult,
) {
  if (response.sessionId !== input.sessionId) {
    throw {
      code: 'host-unavailable',
      message: '요청한 세션과 다른 프리셋 카탈로그 응답을 받았어요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  return response
}

function normalizeHostError(error: unknown): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    return parsed.data
  }

  if (error instanceof Error) {
    return {
      code: 'host-unavailable',
      message: error.message,
    }
  }

  return {
    code: 'host-unavailable',
    message: '지금은 프리셋을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
  }
}

function readBrowserCatalogFixture() {
  const fixture = (
    globalThis as typeof globalThis & {
      __BOOTHY_BROWSER_PRESET_CATALOG__?: unknown
    }
  ).__BOOTHY_BROWSER_PRESET_CATALOG__

  if (fixture === undefined) {
    return null
  }

  const parsed = presetCatalogResultSchema.safeParse(fixture)

  if (parsed.success) {
    return parsed.data
  }

  throw {
    code: 'host-unavailable',
    message: '브라우저 프리셋 카탈로그 fixture 형식이 올바르지 않아요.',
  } satisfies HostErrorEnvelope
}

export function createBrowserPresetCatalogGateway(): PresetCatalogGateway {
  return {
    async loadPresetCatalog(input) {
      const fixture = readBrowserCatalogFixture()

      if (fixture !== null) {
        return {
          ...fixture,
          sessionId: input.sessionId,
        } satisfies PresetCatalogResult
      }

      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 프리셋 카탈로그 fixture를 먼저 연결해 주세요.',
      } satisfies HostErrorEnvelope
    },
  }
}

export function createTauriPresetCatalogGateway(): PresetCatalogGateway {
  return {
    async loadPresetCatalog(input) {
      return invoke<unknown>('load_preset_catalog', { input })
    },
  }
}

export function createDefaultPresetCatalogGateway() {
  return isTauriRuntime()
    ? createTauriPresetCatalogGateway()
    : createBrowserPresetCatalogGateway()
}

type CreatePresetCatalogServiceOptions = {
  gateway?: PresetCatalogGateway
}

export function createPresetCatalogService({
  gateway = createDefaultPresetCatalogGateway(),
}: CreatePresetCatalogServiceOptions = {}) {
  return new DefaultPresetCatalogService(gateway)
}
