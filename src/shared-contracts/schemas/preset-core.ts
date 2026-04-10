import { z } from 'zod'

import { sessionIdSchema } from './ids'

export const presetIdSchema = z
  .string()
  .regex(/^preset_[a-z0-9-]+$/i, '유효한 프리셋 식별자가 아니에요.')

export const publishedVersionSchema = z
  .string()
  .regex(/^\d{4}\.\d{2}\.\d{2}$/, '게시 버전 형식이 올바르지 않아요.')

export const catalogRevisionSchema = z
  .number()
  .int()
  .nonnegative('catalog revision은 0 이상의 정수여야 해요.')

const bundleRelativePathSchema = z
  .string()
  .trim()
  .min(1)
  .refine((value) => {
    if (/^[a-zA-Z]:[\\/]/.test(value) || value.startsWith('/') || value.startsWith('\\')) {
      return false
    }

    return value
      .split(/[\\/]+/)
      .every((segment) => segment.length > 0 && segment !== '.' && segment !== '..')
  }, 'bundle root 내부의 안전한 상대 경로여야 해요.')

const runtimeAssetPathSchema = z
  .string()
  .trim()
  .min(1)
  .refine(
    (value) =>
      bundleRelativePathSchema.safeParse(value).success ||
      /^[a-zA-Z]:[\\/]/.test(value) ||
      value.startsWith('/') ||
      value.startsWith('\\'),
    '런타임 응답에서는 안전한 상대 경로나 절대 파일 경로만 허용돼요.',
  )

const runtimePresetPreviewAssetSchema = z
  .object({
    kind: z.enum(['preview-tile', 'sample-cut']),
    assetPath: runtimeAssetPathSchema,
    altText: z.string().min(1),
  })
  .strict()

export const presetPreviewAssetSchema = z
  .object({
    kind: z.enum(['preview-tile', 'sample-cut']),
    assetPath: bundleRelativePathSchema,
    altText: z.string().min(1),
  })
  .strict()

export const publishedPresetRenderProfileSchema = z
  .object({
    profileId: z.string().trim().min(1),
    displayName: z.string().trim().min(1),
    outputColorSpace: z.string().trim().min(1),
  })
  .strict()

export const publishedPresetSummarySchema = z
  .object({
    presetId: presetIdSchema,
    displayName: z.string().trim().min(1),
    publishedVersion: publishedVersionSchema,
    boothStatus: z.literal('booth-safe'),
    preview: runtimePresetPreviewAssetSchema,
  })
  .strict()

export const publishedPresetBundleSchema = z
  .object({
    schemaVersion: z.literal('published-preset-bundle/v1'),
    presetId: presetIdSchema,
    displayName: z.string().trim().min(1),
    publishedVersion: publishedVersionSchema,
    lifecycleStatus: z.literal('published'),
    boothStatus: z.literal('booth-safe'),
    darktableVersion: z
      .string()
      .trim()
      .regex(/^\d+\.\d+\.\d+$/, 'darktable version 형식이 올바르지 않아요.'),
    darktableProjectPath: bundleRelativePathSchema.optional(),
    xmpTemplatePath: bundleRelativePathSchema,
    previewProfile: publishedPresetRenderProfileSchema,
    finalProfile: publishedPresetRenderProfileSchema,
    preview: presetPreviewAssetSchema,
    sampleCut: presetPreviewAssetSchema.optional(),
    sourceDraftVersion: z.number().int().positive().optional(),
    publishedAt: z.string().trim().min(1).optional(),
    publishedBy: z.string().trim().min(1).optional(),
  })
  .strict()

export const presetDisplayNameSchema = z.string().trim().min(1)

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
