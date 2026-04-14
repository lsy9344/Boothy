import { z } from 'zod'

import { previewRendererWarmStateSchema } from './dedicated-renderer'
import { sessionIdSchema } from './ids'
import {
  activePresetBindingSchema,
  catalogRevisionSchema,
  presetDisplayNameSchema,
} from './preset-core'
import { sessionCaptureRecordSchema } from './session-capture'
import { sessionTimingSnapshotSchema } from './session-timing'

export const sessionManifestSchemaVersion = 'session-manifest/v1' as const
const postEndTimestampSchema = z.string().datetime()
const postEndLabelSchema = z.string().trim().min(1).max(80)
const postEndCopySchema = z.string().trim().min(1).max(120)

export const sessionPostEndStateSchema = z.enum([
  'export-waiting',
  'completed',
  'phone-required',
])
export const sessionPostEndCompletionVariantSchema = z.enum([
  'local-deliverable-ready',
  'handoff-ready',
])

export const exportWaitingPostEndSchema = z.object({
  state: z.literal('export-waiting'),
  evaluatedAt: postEndTimestampSchema,
})

export const completedPostEndSchema = z
  .object({
    state: z.literal('completed'),
    evaluatedAt: postEndTimestampSchema,
    completionVariant: sessionPostEndCompletionVariantSchema,
    approvedRecipientLabel: postEndLabelSchema.nullable().optional(),
    nextLocationLabel: postEndLabelSchema.nullable().optional(),
    primaryActionLabel: postEndLabelSchema,
    supportActionLabel: postEndLabelSchema.nullable().optional(),
    showBoothAlias: z.boolean(),
    handoff: z.unknown().nullable().optional(),
  })
  .superRefine((record, context) => {
    if (
      record.completionVariant === 'handoff-ready' &&
      record.approvedRecipientLabel == null &&
      record.nextLocationLabel == null
    ) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message:
          'handoff-ready completion에는 승인된 수령 대상 또는 다음 이동 위치가 필요해요.',
        path: ['completionVariant'],
      })
    }
  })

export const phoneRequiredPostEndSchema = z.object({
  state: z.literal('phone-required'),
  evaluatedAt: postEndTimestampSchema,
  primaryActionLabel: postEndLabelSchema,
  supportActionLabel: postEndLabelSchema.nullable().optional(),
  unsafeActionWarning: postEndCopySchema,
  showBoothAlias: z.boolean().default(false),
})

export const sessionPostEndSchema = z.union([
  exportWaitingPostEndSchema,
  completedPostEndSchema,
  phoneRequiredPostEndSchema,
])

export const boothAliasSchema = z
  .string()
  .trim()
  .min(1, '고객 별칭이 비어 있을 수 없어요.')

export const previewRendererRouteSnapshotSchema = z.object({
  route: z.string().trim().min(1),
  routeStage: z.string().trim().min(1),
  fallbackReasonCode: z.string().trim().min(1).nullable().optional(),
})

export const previewRendererWarmStateSnapshotSchema = z.object({
  presetId: z.string().trim().min(1),
  publishedVersion: z.string().trim().min(1),
  state: previewRendererWarmStateSchema,
  observedAt: z.string().datetime({ offset: true }),
  diagnosticsDetailPath: z.string().trim().min(1).nullable().optional(),
})

export const customerNameSchema = z
  .string()
  .trim()
  .min(1, '이름을 입력해 주세요.')

export const phoneLastFourSchema = z
  .string()
  .regex(/^\d{4}$/, '휴대전화 뒤 4자리는 숫자 4자리여야 해요.')

export const sessionManifestSchema = z
  .object({
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
    catalogRevision: catalogRevisionSchema.nullable().optional(),
    catalogSnapshot: z
      .array(activePresetBindingSchema)
      .max(6)
      .nullable()
      .optional(),
    activePreset: activePresetBindingSchema.nullable(),
    activePresetId: z.string().trim().min(1).nullable().optional(),
    activePresetDisplayName: presetDisplayNameSchema.nullable().optional(),
    activePreviewRendererRoute: previewRendererRouteSnapshotSchema
      .nullable()
      .optional(),
    activePreviewRendererWarmState: previewRendererWarmStateSnapshotSchema
      .nullable()
      .optional(),
    timing: sessionTimingSnapshotSchema.nullable().optional(),
    captures: z.array(sessionCaptureRecordSchema),
    postEnd: sessionPostEndSchema.nullable(),
  })
  .superRefine((manifest, context) => {
    const hasCatalogRevision = manifest.catalogRevision != null
    const hasCatalogSnapshot = manifest.catalogSnapshot != null

    if (hasCatalogRevision !== hasCatalogSnapshot) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message:
          'catalogRevision과 catalogSnapshot은 함께 기록되어야 해요.',
        path: hasCatalogRevision ? ['catalogSnapshot'] : ['catalogRevision'],
      })
    }

    if (
      manifest.activePreset != null &&
      manifest.activePresetId != null &&
      manifest.activePreset.presetId !== manifest.activePresetId
    ) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message:
          'activePresetId는 canonical activePreset.presetId와 같은 값을 유지해야 해요.',
        path: ['activePresetId'],
      })
    }
  })

export const sessionStartResultSchema = z.object({
  sessionId: sessionIdSchema,
  boothAlias: boothAliasSchema,
  manifest: sessionManifestSchema,
})
