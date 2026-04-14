import { z } from 'zod'

import {
  catalogRevisionSchema,
  presetDisplayNameSchema,
  presetIdSchema,
  publishedPresetNoisePolicySchema,
  publishedPresetRenderProfileSchema,
  publishedPresetSummarySchema,
  publishedVersionSchema,
} from './preset-core'

const presetLifecycleStates = [
  'draft',
  'validated',
  'approved',
  'published',
] as const

function normalizeOptionalText(value: string | null | undefined) {
  if (value === undefined || value === null) {
    return null
  }

  const normalized = value.trim()

  return normalized.length > 0 ? normalized : null
}

function isSafeWorkspaceReference(reference: string) {
  if (reference.startsWith('/') || reference.startsWith('\\')) {
    return false
  }

  if (/^[a-z]:/i.test(reference)) {
    return false
  }

  const normalized = reference.replaceAll('\\', '/')

  return !normalized
    .split('/')
    .some((segment) => segment === '' || segment === '.' || segment === '..')
}

const workspaceReferenceSchema = z
  .string()
  .trim()
  .min(1, '작업공간 참조 경로를 입력해 주세요.')
  .refine(isSafeWorkspaceReference, '작업공간 바깥 경로는 저장할 수 없어요.')

const draftFolderNameSchema = z
  .string()
  .trim()
  .min(1, '복구할 draft 폴더 이름을 입력해 주세요.')
  .refine(
    (value) => {
      const normalized = value.replaceAll('\\', '/')

      return isSafeWorkspaceReference(value) && !normalized.includes('/')
    },
    '복구할 draft 폴더 이름을 다시 확인해 주세요.',
  )

const optionalTextSchema = z
  .string()
  .max(2000, '설명은 2000자 이하여야 해요.')
  .nullable()
  .optional()
  .transform(normalizeOptionalText)

export const presetLifecycleStateSchema = z.enum(presetLifecycleStates)
export const draftOnlyLifecycleStateSchema = z.literal('draft')
export const draftRuntimeLifecycleStateSchema = z.enum([
  'draft',
  'validated',
  'approved',
  'published',
])
export const draftVersionSchema = z.number().int().positive()
export const draftValidationStatusSchema = z.enum(['not-run', 'passed', 'failed'])
export const draftValidationSeveritySchema = z.enum(['error', 'warning'])
export const publicationScopeSchema = z.enum([
  'future-sessions-only',
  'active-session',
])
export const publicationAuditActionSchema = z.enum([
  'approved',
  'published',
  'rejected',
])
export const publicationRejectionReasonCodeSchema = z.enum([
  'draft-not-validated',
  'stale-validation',
  'metadata-mismatch',
  'duplicate-version',
  'path-escape',
  'future-session-only-violation',
  'stage-unavailable',
])
export const catalogVersionHistoryActionSchema = z.enum(['published', 'rollback'])
export const rollbackPresetCatalogReasonCodeSchema = z.enum([
  'target-missing',
  'target-incompatible',
  'already-live',
  'stale-catalog-revision',
  'stage-unavailable',
])

const actorIdSchema = z
  .string()
  .trim()
  .regex(/^[a-z0-9][a-z0-9-]*$/, 'actor ID 형식이 올바르지 않아요.')

const actorLabelSchema = z
  .string()
  .trim()
  .min(1, '승인자 이름을 입력해 주세요.')
  .max(120, '승인자 이름은 120자 이하여야 해요.')

export const draftPresetPreviewReferenceSchema = z.object({
  assetPath: workspaceReferenceSchema,
  altText: z.string().trim().min(1, '대표 preview 설명을 입력해 주세요.'),
})

export const draftRenderProfileSchema = publishedPresetRenderProfileSchema
export const draftNoisePolicySchema = publishedPresetNoisePolicySchema

export const draftValidationFindingSchema = z.object({
  ruleCode: z
    .string()
    .trim()
    .regex(/^[a-z0-9-]+$/, 'rule code 형식이 올바르지 않아요.'),
  severity: draftValidationSeveritySchema,
  fieldPath: z
    .string()
    .trim()
    .max(200, 'field path는 200자 이하여야 해요.')
    .nullable()
    .optional()
    .transform(normalizeOptionalText),
  message: z.string().trim().min(1, '수정 메시지를 입력해 주세요.'),
  guidance: z.string().trim().min(1, '수정 가이드를 입력해 주세요.'),
})

export const draftValidationReportSchema = z
  .object({
    schemaVersion: z.literal('draft-preset-validation/v1'),
    presetId: presetIdSchema,
    draftVersion: draftVersionSchema,
    lifecycleState: draftRuntimeLifecycleStateSchema,
    status: z.enum(['passed', 'failed']),
    checkedAt: z.string().trim().min(1),
    findings: z.array(draftValidationFindingSchema),
  })
  .superRefine((report, context) => {
    const hasError = report.findings.some((finding) => finding.severity === 'error')

    if (report.status === 'passed' && hasError) {
      context.addIssue({
        code: 'custom',
        message: '검증 통과 결과에는 error finding을 담을 수 없어요.',
        path: ['findings'],
      })
    }

    if (report.status === 'failed' && !hasError) {
      context.addIssue({
        code: 'custom',
        message: '검증 실패 결과에는 최소 한 개 이상의 error finding이 필요해요.',
        path: ['findings'],
      })
    }
  })

export const draftValidationSnapshotSchema = z
  .object({
    status: draftValidationStatusSchema,
    latestReport: draftValidationReportSchema.nullable(),
    history: z.array(draftValidationReportSchema),
  })
  .superRefine((validation, context) => {
    if (validation.status === 'not-run' && validation.latestReport !== null) {
      context.addIssue({
        code: 'custom',
        message: '검증을 아직 실행하지 않았다면 latestReport는 비어 있어야 해요.',
        path: ['latestReport'],
      })
    }

    if (validation.status !== 'not-run' && validation.latestReport === null) {
      context.addIssue({
        code: 'custom',
        message: '검증이 끝난 draft는 latestReport를 포함해야 해요.',
        path: ['latestReport'],
      })
    }

    if (
      validation.latestReport !== null &&
      !validation.history.some(
        (report) =>
          report.checkedAt === validation.latestReport?.checkedAt &&
          report.draftVersion === validation.latestReport?.draftVersion,
      )
    ) {
      context.addIssue({
        code: 'custom',
        message: 'latestReport는 history에도 기록되어야 해요.',
        path: ['history'],
      })
    }
  })

export const publicationAuditRecordSchema = z
  .object({
    schemaVersion: z.literal('preset-publication-audit/v1'),
    presetId: presetIdSchema,
    draftVersion: draftVersionSchema,
    publishedVersion: publishedVersionSchema,
    actorId: actorIdSchema,
    actorLabel: actorLabelSchema,
    reviewNote: optionalTextSchema,
    action: publicationAuditActionSchema,
    reasonCode: publicationRejectionReasonCodeSchema.nullable(),
    guidance: z.string().trim().min(1, '감사 이력 가이드를 남겨 주세요.'),
    notedAt: z.string().trim().min(1),
  })
  .superRefine((record, context) => {
    if (record.action === 'rejected' && record.reasonCode === null) {
      addCustomIssue(
        context,
        ['reasonCode'],
        '거절된 게시 이력에는 reasonCode가 필요해요.',
      )
    }

    if (record.action !== 'rejected' && record.reasonCode !== null) {
      addCustomIssue(
        context,
        ['reasonCode'],
        '승인 또는 게시 완료 이력에는 reasonCode를 남기지 않아요.',
      )
    }
  })

export const catalogVersionHistoryItemSchema = z.object({
  schemaVersion: z.literal('preset-catalog-history/v1'),
  presetId: presetIdSchema,
  actionType: catalogVersionHistoryActionSchema,
  fromPublishedVersion: publishedVersionSchema.nullable(),
  toPublishedVersion: publishedVersionSchema,
  actorId: actorIdSchema,
  actorLabel: actorLabelSchema,
  happenedAt: z.string().trim().min(1),
})

function addCustomIssue(
  context: z.RefinementCtx,
  path: (string | number)[],
  message: string,
) {
  context.addIssue({
    code: 'custom',
    path,
    message,
  })
}

function hasPublicationHistoryAction(
  history: z.infer<typeof publicationAuditRecordSchema>[],
  action: z.infer<typeof publicationAuditActionSchema>,
  publishedVersion: string,
) {
  return history.some(
    (record) =>
      record.action === action && record.publishedVersion === publishedVersion,
  )
}

export const catalogStateSummarySchema = z
  .object({
    presetId: presetIdSchema,
    livePublishedVersion: publishedVersionSchema,
    publishedPresets: z.array(publishedPresetSummarySchema).min(1),
    versionHistory: z.array(catalogVersionHistoryItemSchema),
  })
  .superRefine((summary, context) => {
    if (
      !summary.publishedPresets.some(
        (preset) =>
          preset.presetId === summary.presetId &&
          preset.publishedVersion === summary.livePublishedVersion,
      )
    ) {
      addCustomIssue(
        context,
        ['livePublishedVersion'],
        'livePublishedVersion은 publishedPresets 안의 버전이어야 해요.',
      )
    }

    for (const [index, preset] of summary.publishedPresets.entries()) {
      if (preset.presetId !== summary.presetId) {
        addCustomIssue(
          context,
          ['publishedPresets', index, 'presetId'],
          'catalog summary는 같은 presetId의 게시 버전만 포함해야 해요.',
        )
      }
    }

    for (const [index, history] of summary.versionHistory.entries()) {
      if (history.presetId !== summary.presetId) {
        addCustomIssue(
          context,
          ['versionHistory', index, 'presetId'],
          'versionHistory는 같은 presetId에만 연결되어야 해요.',
        )
      }
    }
  })

export const catalogStateResultSchema = z.object({
  schemaVersion: z.literal('preset-catalog-state-result/v1'),
  catalogRevision: catalogRevisionSchema,
  presets: z.array(catalogStateSummarySchema),
})

export const rollbackPresetCatalogInputSchema = z.object({
  presetId: presetIdSchema,
  targetPublishedVersion: publishedVersionSchema,
  expectedCatalogRevision: catalogRevisionSchema,
  actorId: actorIdSchema,
  actorLabel: actorLabelSchema,
})

export const rollbackPresetCatalogSuccessSchema = z.object({
  schemaVersion: z.literal('preset-catalog-rollback-result/v1'),
  status: z.literal('rolled-back'),
  catalogRevision: catalogRevisionSchema,
  summary: catalogStateSummarySchema,
  auditEntry: catalogVersionHistoryItemSchema,
  message: z.string().trim().min(1),
})

export const rollbackPresetCatalogRejectionSchema = z.object({
  schemaVersion: z.literal('preset-catalog-rollback-result/v1'),
  status: z.literal('rejected'),
  reasonCode: rollbackPresetCatalogReasonCodeSchema,
  message: z.string().trim().min(1),
  guidance: z.string().trim().min(1),
  catalogRevision: catalogRevisionSchema,
  summary: catalogStateSummarySchema.nullable(),
})

export const rollbackPresetCatalogResultSchema = z.discriminatedUnion('status', [
  rollbackPresetCatalogSuccessSchema,
  rollbackPresetCatalogRejectionSchema,
])

export const draftPresetSummarySchema = z
  .object({
    schemaVersion: z.literal('draft-preset-artifact/v1'),
    presetId: presetIdSchema,
    displayName: presetDisplayNameSchema,
    draftVersion: draftVersionSchema,
    lifecycleState: draftRuntimeLifecycleStateSchema,
    darktableVersion: z
      .string()
      .trim()
      .regex(/^\d+\.\d+\.\d+$/, 'darktable version 형식이 올바르지 않아요.'),
    darktableProjectPath: workspaceReferenceSchema.optional(),
    xmpTemplatePath: workspaceReferenceSchema,
    previewProfile: draftRenderProfileSchema,
    finalProfile: draftRenderProfileSchema,
    noisePolicy: draftNoisePolicySchema,
    preview: draftPresetPreviewReferenceSchema,
    sampleCut: draftPresetPreviewReferenceSchema,
    description: optionalTextSchema,
    notes: optionalTextSchema,
    validation: draftValidationSnapshotSchema,
    publicationHistory: z.array(publicationAuditRecordSchema).default([]),
    updatedAt: z.string().trim().min(1),
  })
  .superRefine((draft, context) => {
    const latestReport = draft.validation.latestReport

    if (
      latestReport !== null &&
      latestReport.presetId !== draft.presetId
    ) {
      addCustomIssue(
        context,
        ['validation', 'latestReport', 'presetId'],
        'latest validation report는 같은 presetId를 가리켜야 해요.',
      )
    }

    if (
      latestReport !== null &&
      latestReport.draftVersion !== draft.draftVersion
    ) {
      addCustomIssue(
        context,
        ['validation', 'latestReport', 'draftVersion'],
        'latest validation report는 현재 draftVersion과 같아야 해요.',
      )
    }

    for (const [index, report] of draft.validation.history.entries()) {
      if (report.presetId !== draft.presetId) {
        addCustomIssue(
          context,
          ['validation', 'history', index, 'presetId'],
          'validation history는 같은 presetId에만 연결되어야 해요.',
        )
      }
    }

    if (draft.lifecycleState === 'draft' && draft.validation.status === 'passed') {
      addCustomIssue(
        context,
        ['lifecycleState'],
        'draft 상태에서는 validation passed를 주장할 수 없어요.',
      )
    }

    if (
      draft.lifecycleState === 'validated' ||
      draft.lifecycleState === 'approved' ||
      draft.lifecycleState === 'published'
    ) {
      if (draft.validation.status !== 'passed') {
        addCustomIssue(
          context,
          ['validation', 'status'],
          'approval-ready 이후 lifecycle은 validation passed 상태여야 해요.',
        )
      }

      if (latestReport === null) {
        addCustomIssue(
          context,
          ['validation', 'latestReport'],
          'approval-ready 이후 lifecycle은 latest validation report를 포함해야 해요.',
        )
      } else if (latestReport.lifecycleState !== 'validated') {
        addCustomIssue(
          context,
          ['validation', 'latestReport', 'lifecycleState'],
          'latest validation report는 validated 결과를 기준으로 남아 있어야 해요.',
        )
      }
    }
  })

export const draftPresetEditPayloadSchema = z.object({
  presetId: presetIdSchema,
  displayName: presetDisplayNameSchema,
  lifecycleState: draftOnlyLifecycleStateSchema.default('draft'),
  darktableVersion: z
    .string()
    .trim()
    .regex(/^\d+\.\d+\.\d+$/, 'darktable version 형식이 올바르지 않아요.'),
  darktableProjectPath: workspaceReferenceSchema.optional(),
  xmpTemplatePath: workspaceReferenceSchema,
  previewProfile: draftRenderProfileSchema,
  finalProfile: draftRenderProfileSchema,
  noisePolicy: draftNoisePolicySchema,
  preview: draftPresetPreviewReferenceSchema,
  sampleCut: draftPresetPreviewReferenceSchema,
  description: optionalTextSchema,
  notes: optionalTextSchema,
})

export const validateDraftPresetInputSchema = z.object({
  presetId: presetIdSchema,
})

export const repairInvalidDraftInputSchema = z.object({
  draftFolder: draftFolderNameSchema,
})

export const validateDraftPresetResultSchema = z
  .object({
    schemaVersion: z.literal('draft-preset-validation-result/v1'),
    draft: draftPresetSummarySchema,
    report: draftValidationReportSchema,
  })
  .superRefine((result, context) => {
    const latestReport = result.draft.validation.latestReport

    if (
      result.draft.lifecycleState !== 'draft' &&
      result.draft.lifecycleState !== 'validated'
    ) {
      addCustomIssue(
        context,
        ['draft', 'lifecycleState'],
        'validateDraftPreset 결과는 draft 또는 validated lifecycle만 반환할 수 있어요.',
      )
    }

    if (result.report.presetId !== result.draft.presetId) {
      addCustomIssue(
        context,
        ['report', 'presetId'],
        'validation result의 report presetId는 draft presetId와 같아야 해요.',
      )
    }

    if (result.report.draftVersion !== result.draft.draftVersion) {
      addCustomIssue(
        context,
        ['report', 'draftVersion'],
        'validation result의 report draftVersion은 draft와 같아야 해요.',
      )
    }

    if (latestReport === null) {
      addCustomIssue(
        context,
        ['draft', 'validation', 'latestReport'],
        'validation result는 draft.latestReport를 함께 포함해야 해요.',
      )
    } else {
      if (latestReport.checkedAt !== result.report.checkedAt) {
        addCustomIssue(
          context,
          ['draft', 'validation', 'latestReport', 'checkedAt'],
          'draft.latestReport와 report.checkedAt은 같은 validation 결과여야 해요.',
        )
      }

      if (latestReport.status !== result.report.status) {
        addCustomIssue(
          context,
          ['draft', 'validation', 'latestReport', 'status'],
          'draft.latestReport와 report.status는 같아야 해요.',
        )
      }
    }

    if (result.draft.validation.status !== result.report.status) {
      addCustomIssue(
        context,
        ['draft', 'validation', 'status'],
        'draft.validation.status와 report.status는 같아야 해요.',
      )
    }

    if (result.report.status === 'passed') {
      if (result.report.lifecycleState !== 'validated') {
        addCustomIssue(
          context,
          ['report', 'lifecycleState'],
          '통과한 validation result는 validated lifecycle을 반환해야 해요.',
        )
      }

      if (result.draft.lifecycleState !== 'validated') {
        addCustomIssue(
          context,
          ['draft', 'lifecycleState'],
          '통과한 validation result는 draft를 validated 상태로 반환해야 해요.',
        )
      }
    } else {
      if (result.report.lifecycleState !== 'draft') {
        addCustomIssue(
          context,
          ['report', 'lifecycleState'],
          '실패한 validation result는 draft lifecycle을 유지해야 해요.',
        )
      }

      if (result.draft.lifecycleState !== 'draft') {
        addCustomIssue(
          context,
          ['draft', 'lifecycleState'],
          '실패한 validation result는 draft 상태를 유지해야 해요.',
        )
      }
    }
  })

export const publishValidatedPresetInputSchema = z.object({
  presetId: presetIdSchema,
  draftVersion: draftVersionSchema,
  validationCheckedAt: z.string().trim().min(1),
  expectedDisplayName: presetDisplayNameSchema,
  publishedVersion: publishedVersionSchema,
  actorId: actorIdSchema,
  actorLabel: actorLabelSchema,
  scope: publicationScopeSchema,
  reviewNote: optionalTextSchema,
})

export const publishValidatedPresetSuccessSchema = z
  .object({
    schemaVersion: z.literal('draft-preset-publication-result/v1'),
    status: z.literal('published'),
    draft: draftPresetSummarySchema,
    publishedPreset: publishedPresetSummarySchema,
    bundlePath: z.string().trim().min(1),
    auditRecord: publicationAuditRecordSchema,
  })
  .superRefine((result, context) => {
    if (result.draft.lifecycleState !== 'published') {
      addCustomIssue(
        context,
        ['draft', 'lifecycleState'],
        '게시 성공 결과는 draft를 published 상태로 반환해야 해요.',
      )
    }

    if (result.auditRecord.action !== 'published') {
      addCustomIssue(
        context,
        ['auditRecord', 'action'],
        '게시 성공 결과의 auditRecord.action은 published여야 해요.',
      )
    }

    if (
      !hasPublicationHistoryAction(
        result.draft.publicationHistory,
        'approved',
        result.publishedPreset.publishedVersion,
      )
    ) {
      addCustomIssue(
        context,
        ['draft', 'publicationHistory'],
        '게시 성공 결과는 approved 이력을 먼저 남겨야 해요.',
      )
    }

    if (
      !hasPublicationHistoryAction(
        result.draft.publicationHistory,
        'published',
        result.publishedPreset.publishedVersion,
      )
    ) {
      addCustomIssue(
        context,
        ['draft', 'publicationHistory'],
        '게시 성공 결과는 published 이력을 함께 남겨야 해요.',
      )
    }
  })

export const publishValidatedPresetRejectionSchema = z
  .object({
    schemaVersion: z.literal('draft-preset-publication-result/v1'),
    status: z.literal('rejected'),
    draft: draftPresetSummarySchema,
    reasonCode: publicationRejectionReasonCodeSchema,
    message: z.string().trim().min(1),
    guidance: z.string().trim().min(1),
    auditRecord: publicationAuditRecordSchema,
  })
  .superRefine((result, context) => {
    const hasPriorPublishedHistory = result.draft.publicationHistory.some(
      (record) =>
        record.action === 'published' &&
        record.draftVersion === result.draft.draftVersion &&
        record.notedAt <= result.auditRecord.notedAt,
    )

    if (result.auditRecord.action !== 'rejected') {
      addCustomIssue(
        context,
        ['auditRecord', 'action'],
        '게시 거절 결과의 auditRecord.action은 rejected여야 해요.',
      )
    }

    if (result.auditRecord.reasonCode !== result.reasonCode) {
      addCustomIssue(
        context,
        ['auditRecord', 'reasonCode'],
        '게시 거절 결과는 reasonCode를 auditRecord와 동일하게 남겨야 해요.',
      )
    }

    if (result.draft.lifecycleState === 'published') {
      if (result.reasonCode !== 'stage-unavailable') {
        addCustomIssue(
          context,
          ['draft', 'lifecycleState'],
          '게시 거절 결과는 draft를 published 상태로 바꾸면 안 돼요.',
        )
      } else if (!hasPriorPublishedHistory) {
        addCustomIssue(
          context,
          ['draft', 'publicationHistory'],
          'stage-unavailable 거절이 published 상태를 유지하려면 이전 published 이력이 필요해요.',
        )
      }
    }
  })

export const publishValidatedPresetResultSchema = z.discriminatedUnion('status', [
  publishValidatedPresetSuccessSchema,
  publishValidatedPresetRejectionSchema,
])

export const invalidDraftArtifactSchema = z.object({
  draftFolder: z.string().trim().min(1, '복구 대상 draft 폴더 이름이 필요해요.'),
  message: z.string().trim().min(1, '복구 안내 메시지가 필요해요.'),
  guidance: z.string().trim().min(1, '복구 가이드가 필요해요.'),
  canRepair: z.boolean().default(false),
})

export const authoringWorkspaceResultSchema = z.object({
  schemaVersion: z.literal('preset-authoring-workspace/v1'),
  supportedLifecycleStates: z
    .tuple([
      z.literal('draft'),
      z.literal('validated'),
      z.literal('approved'),
      z.literal('published'),
    ])
    .readonly(),
  drafts: z.array(draftPresetSummarySchema),
  invalidDrafts: z.array(invalidDraftArtifactSchema).default([]),
})
