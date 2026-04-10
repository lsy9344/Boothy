import type { z } from 'zod'

import {
  authoringWorkspaceResultSchema,
  catalogStateResultSchema,
  catalogStateSummarySchema,
  catalogVersionHistoryActionSchema,
  catalogVersionHistoryItemSchema,
  activePresetBindingSchema,
  draftPresetEditPayloadSchema,
  draftPresetSummarySchema,
  draftValidationFindingSchema,
  draftValidationReportSchema,
  draftValidationSnapshotSchema,
  invalidDraftArtifactSchema,
  publicationAuditRecordSchema,
  publicationAuditActionSchema,
  publicationRejectionReasonCodeSchema,
  publicationScopeSchema,
  loadPresetCatalogInputSchema,
  publishValidatedPresetInputSchema,
  publishValidatedPresetResultSchema,
  publishValidatedPresetRejectionSchema,
  publishValidatedPresetSuccessSchema,
  repairInvalidDraftInputSchema,
  presetLifecycleStateSchema,
  presetCatalogResultSchema,
  publishedPresetBundleSchema,
  publishedPresetRenderProfileSchema,
  presetSelectionInputSchema,
  presetSelectionResultSchema,
  publishedPresetSummarySchema,
  rollbackPresetCatalogInputSchema,
  rollbackPresetCatalogReasonCodeSchema,
  rollbackPresetCatalogResultSchema,
  rollbackPresetCatalogRejectionSchema,
  rollbackPresetCatalogSuccessSchema,
  validateDraftPresetInputSchema,
  validateDraftPresetResultSchema,
} from '../schemas'

export type PresetLifecycleState = z.infer<typeof presetLifecycleStateSchema>
export type DraftPresetSummary = z.infer<typeof draftPresetSummarySchema>
export type DraftPresetEditPayload = z.infer<typeof draftPresetEditPayloadSchema>
export type DraftValidationFinding = z.infer<typeof draftValidationFindingSchema>
export type DraftValidationReport = z.infer<typeof draftValidationReportSchema>
export type DraftValidationSnapshot = z.infer<typeof draftValidationSnapshotSchema>
export type InvalidDraftArtifact = z.infer<typeof invalidDraftArtifactSchema>
export type AuthoringWorkspaceResult = z.infer<typeof authoringWorkspaceResultSchema>
export type ValidateDraftPresetInput = z.infer<typeof validateDraftPresetInputSchema>
export type RepairInvalidDraftInput = z.infer<typeof repairInvalidDraftInputSchema>
export type ValidateDraftPresetResult = z.infer<typeof validateDraftPresetResultSchema>
export type PublicationAuditAction = z.infer<typeof publicationAuditActionSchema>
export type PublicationScope = z.infer<typeof publicationScopeSchema>
export type PublicationRejectionReasonCode = z.infer<
  typeof publicationRejectionReasonCodeSchema
>
export type PublicationAuditRecord = z.infer<typeof publicationAuditRecordSchema>
export type CatalogVersionHistoryAction = z.infer<
  typeof catalogVersionHistoryActionSchema
>
export type CatalogVersionHistoryItem = z.infer<
  typeof catalogVersionHistoryItemSchema
>
export type CatalogStateSummary = z.infer<typeof catalogStateSummarySchema>
export type CatalogStateResult = z.infer<typeof catalogStateResultSchema>
export type PublishValidatedPresetInput = z.infer<
  typeof publishValidatedPresetInputSchema
>
export type PublishValidatedPresetSuccess = z.infer<
  typeof publishValidatedPresetSuccessSchema
>
export type PublishValidatedPresetRejection = z.infer<
  typeof publishValidatedPresetRejectionSchema
>
export type PublishValidatedPresetResult = z.infer<
  typeof publishValidatedPresetResultSchema
>
export type PublishedPresetRenderProfile = z.infer<
  typeof publishedPresetRenderProfileSchema
>
export type PublishedPresetBundle = z.infer<typeof publishedPresetBundleSchema>
export type PublishedPresetSummary = z.infer<typeof publishedPresetSummarySchema>
export type LoadPresetCatalogInput = z.infer<typeof loadPresetCatalogInputSchema>
export type PresetCatalogResult = z.infer<typeof presetCatalogResultSchema>
export type ActivePresetBinding = z.infer<typeof activePresetBindingSchema>
export type PresetSelectionInput = z.infer<typeof presetSelectionInputSchema>
export type PresetSelectionResult = z.infer<typeof presetSelectionResultSchema>
export type RollbackPresetCatalogReasonCode = z.infer<
  typeof rollbackPresetCatalogReasonCodeSchema
>
export type RollbackPresetCatalogInput = z.infer<
  typeof rollbackPresetCatalogInputSchema
>
export type RollbackPresetCatalogSuccess = z.infer<
  typeof rollbackPresetCatalogSuccessSchema
>
export type RollbackPresetCatalogRejection = z.infer<
  typeof rollbackPresetCatalogRejectionSchema
>
export type RollbackPresetCatalogResult = z.infer<
  typeof rollbackPresetCatalogResultSchema
>
