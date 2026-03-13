import { z } from 'zod'

import presetCatalogAsset from './presetCatalog.json'

export const presetIdSchema = z
  .string()
  .trim()
  .min(1)
  .regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/)

export const presetCatalogEntrySchema = z
  .object({
    id: presetIdSchema,
    name: z.string().min(1),
    description: z.string().min(1),
    previewRef: z.string().regex(/^preset-preview\//),
  })
  .strict()

export const presetCatalogSchema = z.array(presetCatalogEntrySchema).superRefine((catalog, context) => {
  if (catalog.length < 1) {
    context.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'Preset catalog must contain at least one approved entry.',
    })
  }

  const ids = catalog.map((preset) => preset.id)

  if (new Set(ids).size !== ids.length) {
    context.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'Preset catalog ids must be unique.',
    })
  }

  if (catalog.length > 6) {
    context.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'Preset catalog must stay bounded to six entries or fewer.',
    })
  }
})

export const presetCatalog = presetCatalogSchema.parse(presetCatalogAsset)

export const defaultPresetId = presetCatalog[0].id

export function getPresetCatalogEntryById(id: string) {
  const parsedId = presetIdSchema.safeParse(id)

  if (!parsedId.success) {
    return undefined
  }

  return presetCatalog.find((preset) => preset.id === parsedId.data)
}

export type PresetCatalogEntry = z.infer<typeof presetCatalogEntrySchema>
export type PresetId = z.infer<typeof presetIdSchema>
