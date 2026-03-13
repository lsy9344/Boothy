import { z } from 'zod'

import { presetIdSchema } from './presetCatalog.js'

const nonEmptyStringSchema = z.string().min(1)

export const activePresetChangeRequestSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    presetId: presetIdSchema,
  })
  .strict()

export const activePresetChangeResultSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    activePresetId: presetIdSchema,
    appliedAt: z.iso.datetime(),
  })
  .strict()

export type ActivePresetChangeRequest = z.infer<typeof activePresetChangeRequestSchema>
export type ActivePresetChangeResult = z.infer<typeof activePresetChangeResultSchema>
