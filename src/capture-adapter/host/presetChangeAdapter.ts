import { invoke, isTauri } from '@tauri-apps/api/core'

import { writeFallbackSessionPreset } from './fallbackPresetSessionState.js'
import {
  activePresetChangeRequestSchema,
  activePresetChangeResultSchema,
  type ActivePresetChangeRequest,
  type ActivePresetChangeResult,
} from '../../shared-contracts/presets/presetChangeSchemas.js'
import { sessionPresetSelectionResultSchema } from '../../shared-contracts/schemas/presetSchemas.js'

export type PresetChangeExecutor = (
  request: ActivePresetChangeRequest,
) => Promise<ActivePresetChangeResult>

export type PresetChangeAdapter = {
  applyActivePreset(request: ActivePresetChangeRequest): Promise<ActivePresetChangeResult>
}

export function createPresetChangeAdapter(
  executor: PresetChangeExecutor = async (request) => {
    if (!isTauri()) {
      writeFallbackSessionPreset(request.sessionId, request.presetId)

      return {
        sessionId: request.sessionId,
        activePresetId: request.presetId,
        appliedAt: new Date().toISOString(),
      }
    }

    const response = await invoke('select_session_preset', {
      payload: request,
    })
    const parsedResponse = sessionPresetSelectionResultSchema.parse(response)

    if (!parsedResponse.ok) {
      throw new Error(parsedResponse.message)
    }

    return {
      sessionId: request.sessionId,
      activePresetId: parsedResponse.value.activePreset.presetId,
      appliedAt: parsedResponse.value.updatedAt,
    }
  },
): PresetChangeAdapter {
  return {
    async applyActivePreset(request) {
      const parsedRequest = activePresetChangeRequestSchema.parse(request)
      const result = await executor(parsedRequest)

      return activePresetChangeResultSchema.parse(result)
    },
  }
}

export const presetChangeAdapter = createPresetChangeAdapter()
