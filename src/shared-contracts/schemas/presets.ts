import { z } from 'zod'

import {
  activePresetBindingSchema,
  canonicalPresetRecipeSchema,
  darktableAdapterReferenceSchema,
  loadPresetCatalogInputSchema,
  presetCatalogResultSchema,
  presetIdSchema,
  presetPreviewAssetSchema,
  publishedPresetBundleSchema,
  publishedPresetNoisePolicySchema,
  publishedPresetRenderProfileSchema,
  presetSelectionInputSchema,
  publishedPresetSummarySchema,
  publishedVersionSchema,
} from './preset-core'
import { sessionManifestSchema } from './session-manifest'

export {
  activePresetBindingSchema,
  canonicalPresetRecipeSchema,
  darktableAdapterReferenceSchema,
  loadPresetCatalogInputSchema,
  presetCatalogResultSchema,
  presetIdSchema,
  presetPreviewAssetSchema,
  publishedPresetBundleSchema,
  publishedPresetNoisePolicySchema,
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
