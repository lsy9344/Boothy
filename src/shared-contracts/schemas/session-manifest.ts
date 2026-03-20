import { z } from 'zod'

import { sessionIdSchema } from './ids'
import { activePresetBindingSchema } from './preset-core'
import { sessionCaptureRecordSchema } from './session-capture'

export const sessionManifestSchemaVersion = 'session-manifest/v1' as const

export const boothAliasSchema = z
  .string()
  .trim()
  .min(1, '고객 별칭이 비어 있을 수 없어요.')

export const customerNameSchema = z
  .string()
  .trim()
  .min(1, '이름을 입력해 주세요.')

export const phoneLastFourSchema = z
  .string()
  .regex(/^\d{4}$/, '휴대전화 뒤 4자리는 숫자 4자리여야 해요.')

export const sessionManifestSchema = z.object({
  schemaVersion: z.literal(sessionManifestSchemaVersion),
  sessionId: sessionIdSchema,
  boothAlias: boothAliasSchema,
  customer: z.object({
    name: customerNameSchema,
    phoneLastFour: phoneLastFourSchema,
  }),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
  lifecycle: z.object({
    status: z.literal('active'),
    stage: z.string().trim().min(1),
  }),
  activePreset: activePresetBindingSchema.nullable(),
  activePresetId: z.string().trim().min(1).nullable().optional(),
  captures: z.array(sessionCaptureRecordSchema),
  postEnd: z.null(),
})

export const sessionStartResultSchema = z.object({
  sessionId: sessionIdSchema,
  boothAlias: boothAliasSchema,
  manifest: sessionManifestSchema,
})
