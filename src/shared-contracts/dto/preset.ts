import type { z } from 'zod'

import {
  activePresetBindingSchema,
  loadPresetCatalogInputSchema,
  presetCatalogResultSchema,
  presetSelectionInputSchema,
  presetSelectionResultSchema,
  publishedPresetSummarySchema,
} from '../schemas'

export type PublishedPresetSummary = z.infer<typeof publishedPresetSummarySchema>
export type LoadPresetCatalogInput = z.infer<typeof loadPresetCatalogInputSchema>
export type PresetCatalogResult = z.infer<typeof presetCatalogResultSchema>
export type ActivePresetBinding = z.infer<typeof activePresetBindingSchema>
export type PresetSelectionInput = z.infer<typeof presetSelectionInputSchema>
export type PresetSelectionResult = z.infer<typeof presetSelectionResultSchema>
