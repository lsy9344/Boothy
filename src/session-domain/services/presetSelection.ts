import { invoke } from '@tauri-apps/api/core'

import { defaultPresetId, getPresetCatalogEntryById } from '../../shared-contracts/presets/presetCatalog.js'
import {
  sessionPresetSelectionPayloadSchema,
  sessionPresetSelectionResultSchema,
  type SessionPresetSelectionResult,
} from '../../shared-contracts/schemas/presetSchemas.js'

type PresetSelectionInvokeClient = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export type PresetSelectionService = {
  selectPreset(payload: Parameters<typeof sessionPresetSelectionPayloadSchema.parse>[0]): Promise<SessionPresetSelectionResult>
}

export function resolveDefaultPresetId(lastUsedPresetId: string | null | undefined): string {
  return getPresetCatalogEntryById(lastUsedPresetId ?? '')?.id ?? defaultPresetId
}

export function createPresetSelectionService(
  invokeClient: PresetSelectionInvokeClient = invoke,
): PresetSelectionService {
  return {
    async selectPreset(payload) {
      const request = sessionPresetSelectionPayloadSchema.parse(payload)
      const response = await invokeClient<unknown>('select_session_preset', { payload: request })

      return sessionPresetSelectionResultSchema.parse(response)
    },
  }
}

export const presetSelectionService = createPresetSelectionService()
