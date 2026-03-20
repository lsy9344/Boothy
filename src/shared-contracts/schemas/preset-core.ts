import { z } from 'zod'

import { sessionIdSchema } from './ids'

export const presetIdSchema = z
  .string()
  .regex(/^preset_[a-z0-9-]+$/i, '유효한 프리셋 식별자가 아니에요.')

export const publishedVersionSchema = z
  .string()
  .regex(/^\d{4}\.\d{2}\.\d{2}$/, '게시 버전 형식이 올바르지 않아요.')

export const presetPreviewAssetSchema = z.object({
  kind: z.enum(['preview-tile', 'sample-cut']),
  assetPath: z.string().min(1),
  altText: z.string().min(1),
})

export const publishedPresetSummarySchema = z.object({
  presetId: presetIdSchema,
  displayName: z.string().trim().min(1),
  publishedVersion: publishedVersionSchema,
  boothStatus: z.literal('booth-safe'),
  preview: presetPreviewAssetSchema,
})

export const loadPresetCatalogInputSchema = z.object({
  sessionId: sessionIdSchema,
})

export const presetCatalogResultSchema = z.object({
  sessionId: sessionIdSchema,
  state: z.enum(['ready', 'empty']),
  presets: z.array(publishedPresetSummarySchema).max(6),
}).superRefine((catalog, context) => {
  if (catalog.state === 'ready' && catalog.presets.length === 0) {
    context.addIssue({
      code: 'custom',
      message: '표시할 프리셋이 하나 이상 필요해요.',
      path: ['presets'],
    })
  }

  if (catalog.state === 'empty' && catalog.presets.length > 0) {
    context.addIssue({
      code: 'custom',
      message: '빈 카탈로그 상태에는 프리셋을 담을 수 없어요.',
      path: ['presets'],
    })
  }
})

export const activePresetBindingSchema = z.object({
  presetId: presetIdSchema,
  publishedVersion: publishedVersionSchema,
})

export const presetSelectionInputSchema = z.object({
  sessionId: sessionIdSchema,
  preset: activePresetBindingSchema,
})
