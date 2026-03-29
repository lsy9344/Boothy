import { invoke } from '@tauri-apps/api/core'

import {
  hostErrorEnvelopeSchema,
  presetSelectionInputSchema,
  presetSelectionResultSchema,
  type HostErrorEnvelope,
  type PresetSelectionInput,
  type PresetSelectionResult,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export interface ActivePresetGateway {
  selectActivePreset(input: PresetSelectionInput): Promise<unknown>
}

export interface ActivePresetService {
  selectActivePreset(input: PresetSelectionInput): Promise<PresetSelectionResult>
}

class DefaultActivePresetService implements ActivePresetService {
  private readonly gateway: ActivePresetGateway

  constructor(gateway: ActivePresetGateway) {
    this.gateway = gateway
  }

  async selectActivePreset(input: PresetSelectionInput) {
    const parsedInput = presetSelectionInputSchema.parse(input)

    try {
      const response = await this.gateway.selectActivePreset(parsedInput)
      const parsedResponse = presetSelectionResultSchema.parse(response)

      return ensureMatchingPresetSelectionResponse(parsedInput, parsedResponse)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }
}

function ensureMatchingPresetSelectionResponse(
  input: PresetSelectionInput,
  response: PresetSelectionResult,
) {
  const manifestActivePreset = response.manifest.activePreset

  if (
    response.sessionId !== input.sessionId ||
    response.manifest.sessionId !== input.sessionId ||
    response.activePreset.presetId !== input.preset.presetId ||
    response.activePreset.publishedVersion !== input.preset.publishedVersion ||
    manifestActivePreset?.presetId !== input.preset.presetId ||
    manifestActivePreset?.publishedVersion !== input.preset.publishedVersion
  ) {
    throw {
      code: 'host-unavailable',
      message: '요청한 세션과 다른 프리셋 선택 응답을 받았어요. 다시 시도해 주세요.',
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
    message: '지금은 선택을 저장할 수 없어요. 잠시 후 다시 시도해 주세요.',
  }
}

export function createBrowserActivePresetGateway(): ActivePresetGateway {
  return {
    async selectActivePreset(input) {
      const timestamp = new Date().toISOString()

      return {
        sessionId: input.sessionId,
        activePreset: input.preset,
        manifest: {
          schemaVersion: 'session-manifest/v1',
          sessionId: input.sessionId,
          boothAlias: 'Preview Session',
          customer: {
            name: 'Preview',
            phoneLastFour: '0000',
          },
          createdAt: timestamp,
          updatedAt: timestamp,
          lifecycle: {
            status: 'active',
            stage: 'preset-selected',
          },
          activePreset: input.preset,
          activePresetId: input.preset.presetId,
          captures: [],
          postEnd: null,
        },
      } satisfies PresetSelectionResult
    },
  }
}

function toTauriPresetSelectionInput(input: PresetSelectionInput) {
  return {
    sessionId: input.sessionId,
    presetId: input.preset.presetId,
    publishedVersion: input.preset.publishedVersion,
  }
}

export function createTauriActivePresetGateway(): ActivePresetGateway {
  return {
    async selectActivePreset(input) {
      return invoke<unknown>('select_active_preset', {
        input: toTauriPresetSelectionInput(input),
      })
    },
  }
}

export function createDefaultActivePresetGateway() {
  return {
    async selectActivePreset(input) {
      const gateway = isTauriRuntime()
        ? createTauriActivePresetGateway()
        : createBrowserActivePresetGateway()

      return gateway.selectActivePreset(input)
    },
  } satisfies ActivePresetGateway
}

type CreateActivePresetServiceOptions = {
  gateway?: ActivePresetGateway
}

export function createActivePresetService({
  gateway = createDefaultActivePresetGateway(),
}: CreateActivePresetServiceOptions = {}) {
  return new DefaultActivePresetService(gateway)
}
