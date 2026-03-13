import { z } from 'zod'

import {
  presetCatalog as approvedPresetCatalog,
  presetIdSchema,
} from '../presets/presetCatalog.js'

export const presetCatalogGroupSchema = z.enum(['tone', 'background'])

const approvedPresetNameById = new Map(
  approvedPresetCatalog.map((preset) => [preset.id, preset.name]),
)
const approvedPresetOrderById = new Map(
  approvedPresetCatalog.map((preset, index) => [preset.id, index]),
)
const approvedPresetCount = approvedPresetCatalog.length
const approvedPresetIdSchema = presetIdSchema.refine((presetId) => approvedPresetNameById.has(presetId), {
  message: 'Preset ID must exist in the approved catalog.',
})

export const presetCatalogItemSchema = z
  .object({
    id: presetIdSchema,
    name: z.string().trim().min(1),
    group: presetCatalogGroupSchema.optional(),
    previewAssetPath: z
      .string()
      .trim()
      .min(1)
      .regex(/^\/src\/customer-flow\/assets\/preset-previews\/.+\.(svg|png|jpg|jpeg)$/i)
      .optional(),
    previewAssetUrl: z.string().trim().min(1).optional(),
  })
  .strict()

export const presetCatalogSchema = z
  .array(presetCatalogItemSchema)
  .min(1)
  .max(6)
  .superRefine((catalog, context) => {
    if (catalog.length !== approvedPresetCount) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Preset catalog must match the full approved baseline.',
      })
    }

    const seenPresetIds = new Set<string>()
    let lastApprovedIndex = -1

    catalog.forEach((preset, index) => {
      if (seenPresetIds.has(preset.id)) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Preset IDs must be unique.',
          path: [index, 'id'],
        })
        return
      }

      seenPresetIds.add(preset.id)

      const approvedPresetIndex = approvedPresetOrderById.get(preset.id)

      if (approvedPresetIndex === undefined) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Preset ID must exist in the approved catalog.',
          path: [index, 'id'],
        })
        return
      }

      if (approvedPresetIndex <= lastApprovedIndex) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Preset catalog order must remain deterministic and approved.',
          path: [index, 'id'],
        })
      }

      lastApprovedIndex = approvedPresetIndex

      const approvedPreset = approvedPresetCatalog[approvedPresetIndex]

      if (preset.name !== approvedPreset.name) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Preset catalog names must match the approved catalog.',
          path: [index, 'name'],
        })
      }
    })
  })

export const activeSessionPresetSchema = z
  .object({
    presetId: approvedPresetIdSchema,
    displayName: z.string().trim().min(1),
  })
  .superRefine((preset, context) => {
    const approvedDisplayName = approvedPresetNameById.get(preset.presetId)

    if (!approvedDisplayName || preset.displayName !== approvedDisplayName) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Active preset display names must match the approved catalog.',
        path: ['displayName'],
      })
    }
  })
  .strict()

export const sessionPresetSelectionPayloadSchema = z
  .object({
    sessionId: z.string().trim().min(1),
    presetId: approvedPresetIdSchema,
  })
  .strict()

export const sessionPresetSelectionResultSchema = z.discriminatedUnion('ok', [
  z.object({
    ok: z.literal(true),
    value: z
      .object({
        manifestPath: z.string().trim().min(1),
        updatedAt: z.iso.datetime(),
        activePreset: activeSessionPresetSchema,
      })
      .strict(),
  }),
  z.object({
    ok: z.literal(false),
    errorCode: z.enum([
      'session.preset_selection_failed',
      'session.preset_selection_invalid_preset',
      'session.preset_selection_invalid_session',
      'session.preset_selection_session_not_found',
    ]),
    message: z.string().trim().min(1),
  }),
])

export type PresetCatalogGroup = z.infer<typeof presetCatalogGroupSchema>
export type PresetCatalogItem = z.infer<typeof presetCatalogItemSchema>
export type PresetCatalog = z.infer<typeof presetCatalogSchema>
export type ActiveSessionPreset = z.infer<typeof activeSessionPresetSchema>
export type SessionPresetSelectionPayload = z.infer<typeof sessionPresetSelectionPayloadSchema>
export type SessionPresetSelectionResult = z.infer<typeof sessionPresetSelectionResultSchema>
