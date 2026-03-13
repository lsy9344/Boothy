import {
  presetChangeAdapter,
  type PresetChangeAdapter,
} from '../../capture-adapter/host/presetChangeAdapter.js'
import {
  activePresetChangeRequestSchema,
  type ActivePresetChangeRequest,
  type ActivePresetChangeResult,
} from '../../shared-contracts/presets/presetChangeSchemas.js'

export type ActivePresetService = {
  applyPresetChange(request: ActivePresetChangeRequest): Promise<ActivePresetChangeResult>
}

export function createActivePresetService(
  adapter: PresetChangeAdapter = presetChangeAdapter,
): ActivePresetService {
  return {
    async applyPresetChange(request) {
      const parsedRequest = activePresetChangeRequestSchema.parse(request)

      return adapter.applyActivePreset(parsedRequest)
    },
  }
}

export const activePresetService = createActivePresetService()
