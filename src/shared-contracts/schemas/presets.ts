import { z } from 'zod'

import {
  activePresetBindingSchema,
  loadPresetCatalogInputSchema,
  presetCatalogResultSchema,
  presetIdSchema,
  presetPreviewAssetSchema,
  publishedPresetBundleSchema,
  publishedPresetRenderProfileSchema,
  presetSelectionInputSchema,
  publishedPresetSummarySchema,
  publishedVersionSchema,
} from './preset-core'
import { sessionManifestSchema } from './session-manifest'

export {
  activePresetBindingSchema,
  loadPresetCatalogInputSchema,
  presetCatalogResultSchema,
  presetIdSchema,
  presetPreviewAssetSchema,
  publishedPresetBundleSchema,
  publishedPresetRenderProfileSchema,
  presetSelectionInputSchema,
  publishedPresetSummarySchema,
  publishedVersionSchema,
}

export const presetSelectionResultSchema = z.object({
  sessionId: sessionManifestSchema.shape.sessionId,
  activePreset: activePresetBindingSchema,
  manifest: sessionManifestSchema,
})
