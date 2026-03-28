import { invoke } from '@tauri-apps/api/core'

import {
  authoringWorkspaceResultSchema,
  catalogStateResultSchema,
  draftPresetEditPayloadSchema,
  draftPresetSummarySchema,
  rollbackPresetCatalogInputSchema,
  rollbackPresetCatalogResultSchema,
  publishValidatedPresetInputSchema,
  publishValidatedPresetResultSchema,
  validateDraftPresetInputSchema,
  validateDraftPresetResultSchema,
  hostErrorEnvelopeSchema,
  type AuthoringWorkspaceResult,
  type CatalogStateResult,
  type DraftPresetEditPayload,
  type DraftPresetSummary,
  type HostErrorEnvelope,
  type PublishValidatedPresetInput,
  type PublishValidatedPresetResult,
  type RollbackPresetCatalogInput,
  type RollbackPresetCatalogResult,
  type ValidateDraftPresetInput,
  type ValidateDraftPresetResult,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export interface PresetAuthoringGateway {
  loadAuthoringWorkspace(): Promise<unknown>
  createDraftPreset(input: DraftPresetEditPayload): Promise<unknown>
  saveDraftPreset(input: DraftPresetEditPayload): Promise<unknown>
  validateDraftPreset(input: ValidateDraftPresetInput): Promise<unknown>
  publishValidatedPreset(input: PublishValidatedPresetInput): Promise<unknown>
  loadPresetCatalogState(): Promise<unknown>
  rollbackPresetCatalog(input: RollbackPresetCatalogInput): Promise<unknown>
}

export interface PresetAuthoringService {
  loadAuthoringWorkspace(): Promise<AuthoringWorkspaceResult>
  createDraftPreset(input: DraftPresetEditPayload): Promise<DraftPresetSummary>
  saveDraftPreset(input: DraftPresetEditPayload): Promise<DraftPresetSummary>
  validateDraftPreset(input: ValidateDraftPresetInput): Promise<ValidateDraftPresetResult>
  publishValidatedPreset(
    input: PublishValidatedPresetInput,
  ): Promise<PublishValidatedPresetResult>
  loadPresetCatalogState(): Promise<CatalogStateResult>
  rollbackPresetCatalog(
    input: RollbackPresetCatalogInput,
  ): Promise<RollbackPresetCatalogResult>
}

class DefaultPresetAuthoringService implements PresetAuthoringService {
  private readonly gateway: PresetAuthoringGateway

  constructor(gateway: PresetAuthoringGateway) {
    this.gateway = gateway
  }

  async loadAuthoringWorkspace() {
    try {
      const response = await this.gateway.loadAuthoringWorkspace()

      return authoringWorkspaceResultSchema.parse(response)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async createDraftPreset(input: DraftPresetEditPayload) {
    const parsedInput = draftPresetEditPayloadSchema.parse(input)

    try {
      const response = await this.gateway.createDraftPreset(parsedInput)

      return ensureMatchingDraftPreset(parsedInput, draftPresetSummarySchema.parse(response))
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async saveDraftPreset(input: DraftPresetEditPayload) {
    const parsedInput = draftPresetEditPayloadSchema.parse(input)

    try {
      const response = await this.gateway.saveDraftPreset(parsedInput)

      return ensureMatchingDraftPreset(parsedInput, draftPresetSummarySchema.parse(response))
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async validateDraftPreset(input: ValidateDraftPresetInput) {
    const parsedInput = validateDraftPresetInputSchema.parse(input)

    try {
      const response = await this.gateway.validateDraftPreset(parsedInput)

      return ensureMatchingValidationResult(
        parsedInput,
        validateDraftPresetResultSchema.parse(response),
      )
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async publishValidatedPreset(input: PublishValidatedPresetInput) {
    const parsedInput = publishValidatedPresetInputSchema.parse(input)

    try {
      const response = await this.gateway.publishValidatedPreset(parsedInput)

      return ensureMatchingPublicationResult(
        parsedInput,
        publishValidatedPresetResultSchema.parse(response),
      )
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async loadPresetCatalogState() {
    try {
      const response = await this.gateway.loadPresetCatalogState()

      return catalogStateResultSchema.parse(response)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async rollbackPresetCatalog(input: RollbackPresetCatalogInput) {
    const parsedInput = rollbackPresetCatalogInputSchema.parse(input)

    try {
      const response = await this.gateway.rollbackPresetCatalog(parsedInput)

      return ensureMatchingRollbackResult(
        parsedInput,
        rollbackPresetCatalogResultSchema.parse(response),
      )
    } catch (error) {
      throw normalizeHostError(error)
    }
  }
}

type BrowserDraftStore = {
  drafts: DraftPresetSummary[]
}

function normalizeHostError(error: unknown): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    return parsed.data
  }

  if (error instanceof Error) {
    return {
      code: 'host-unavailable',
      message: error.message,
    }
  }

  return {
    code: 'host-unavailable',
    message: '지금은 draft 작업공간을 열 수 없어요. 잠시 후 다시 시도해 주세요.',
  }
}

function ensureMatchingDraftPreset(
  input: DraftPresetEditPayload,
  response: DraftPresetSummary,
) {
  if (
    response.presetId !== input.presetId ||
    response.displayName !== input.displayName ||
    response.lifecycleState !== 'draft'
  ) {
    throw {
      code: 'host-unavailable',
      message: '요청한 draft와 다른 authoring 응답을 받았어요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  return response
}

function ensureMatchingValidationResult(
  input: ValidateDraftPresetInput,
  response: ValidateDraftPresetResult,
) {
  const latestReport = response.draft.validation.latestReport
  const isPassedResult = response.report.status === 'passed'

  if (
    response.draft.presetId !== input.presetId ||
    response.report.presetId !== input.presetId ||
    response.report.draftVersion !== response.draft.draftVersion ||
    latestReport?.checkedAt !== response.report.checkedAt ||
    latestReport?.status !== response.report.status ||
    response.draft.validation.status !== response.report.status ||
    (isPassedResult
      ? response.draft.lifecycleState !== 'validated' ||
        response.report.lifecycleState !== 'validated'
      : response.draft.lifecycleState !== 'draft' ||
        response.report.lifecycleState !== 'draft')
  ) {
    throw {
      code: 'host-unavailable',
      message: '요청한 draft 검증 결과와 다른 authoring 응답을 받았어요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  return response
}

function ensureMatchingPublicationResult(
  input: PublishValidatedPresetInput,
  response: PublishValidatedPresetResult,
) {
  if (
    response.draft.presetId !== input.presetId ||
    response.auditRecord.presetId !== input.presetId ||
    response.auditRecord.publishedVersion !== input.publishedVersion
  ) {
    throw {
      code: 'host-unavailable',
      message: '요청한 draft 게시 결과와 다른 authoring 응답을 받았어요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  if (response.status === 'published') {
    if (
      response.publishedPreset.publishedVersion !== input.publishedVersion ||
      response.draft.lifecycleState !== 'published' ||
      response.auditRecord.action !== 'published' ||
      !response.draft.publicationHistory.some(
        (record) =>
          record.action === 'approved' &&
          record.publishedVersion === input.publishedVersion,
      )
    ) {
      throw {
        code: 'host-unavailable',
        message: '게시 결과가 승인 후 published 전이를 끝냈는지 확인할 수 없어요.',
      } satisfies HostErrorEnvelope
    }
  } else if (
    response.auditRecord.action !== 'rejected' ||
    response.auditRecord.reasonCode !== response.reasonCode
  ) {
    throw {
      code: 'host-unavailable',
      message: '게시 거절 결과의 audit 이력이 응답과 맞지 않아요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  } else if (response.reasonCode === 'future-session-only-violation') {
    if (input.scope === 'future-sessions-only') {
      throw {
        code: 'host-unavailable',
        message: 'future-session-only 요청이 잘못 거절되었어요. host publication 응답을 다시 확인해 주세요.',
      } satisfies HostErrorEnvelope
    }
  }

  return response
}

function ensureMatchingRollbackResult(
  input: RollbackPresetCatalogInput,
  response: RollbackPresetCatalogResult,
) {
  if (response.status === 'rolled-back') {
    if (
      response.summary.presetId !== input.presetId ||
      response.summary.livePublishedVersion !== input.targetPublishedVersion ||
      response.auditEntry.presetId !== input.presetId ||
      response.auditEntry.actionType !== 'rollback' ||
      response.auditEntry.toPublishedVersion !== input.targetPublishedVersion ||
      response.catalogRevision <= input.expectedCatalogRevision
    ) {
      throw {
        code: 'host-unavailable',
        message: '롤백 결과가 요청한 catalog 변경과 맞지 않아요. 다시 시도해 주세요.',
      } satisfies HostErrorEnvelope
    }

    return response
  }

  if (response.summary !== null && response.summary.presetId !== input.presetId) {
    throw {
      code: 'host-unavailable',
      message: '롤백 거절 결과가 요청한 preset과 맞지 않아요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  return response
}

function getBrowserDraftStore() {
  const scopedGlobal = globalThis as typeof globalThis & {
    __BOOTHY_AUTHORING_DRAFT_STORE__?: BrowserDraftStore
  }

  if (!scopedGlobal.__BOOTHY_AUTHORING_DRAFT_STORE__) {
    scopedGlobal.__BOOTHY_AUTHORING_DRAFT_STORE__ = {
      drafts: [],
    }
  }

  return scopedGlobal.__BOOTHY_AUTHORING_DRAFT_STORE__
}

function buildBrowserWorkspace(): AuthoringWorkspaceResult {
  const store = getBrowserDraftStore()

  return {
    schemaVersion: 'preset-authoring-workspace/v1',
    supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
    drafts: [...store.drafts].sort((left, right) =>
      right.updatedAt.localeCompare(left.updatedAt),
    ),
  }
}

function buildBrowserDraftSummary(
  input: DraftPresetEditPayload,
  draftVersion: number,
  existingDraft?: DraftPresetSummary,
): DraftPresetSummary {
  return {
    schemaVersion: 'draft-preset-artifact/v1',
    presetId: input.presetId,
    displayName: input.displayName,
    draftVersion,
    lifecycleState: 'draft',
    darktableVersion: input.darktableVersion,
    darktableProjectPath: input.darktableProjectPath,
    xmpTemplatePath: input.xmpTemplatePath,
    previewProfile: input.previewProfile,
    finalProfile: input.finalProfile,
    noisePolicy: input.noisePolicy,
    preview: input.preview,
    sampleCut: input.sampleCut,
    description: input.description ?? null,
    notes: input.notes ?? null,
    validation: {
      status: 'not-run',
      latestReport: null,
      history: existingDraft?.validation.history ?? [],
    },
    publicationHistory: existingDraft?.publicationHistory ?? [],
    updatedAt: new Date().toISOString(),
  }
}

function buildHostValidationUnavailableError(): HostErrorEnvelope {
  return {
    code: 'host-unavailable',
    message:
      'booth compatibility 검증은 authoring host에서만 실행할 수 있어요. 브라우저 미리보기에서는 approval 준비 상태를 계산하지 않아요.',
  }
}

function buildHostPublicationUnavailableError(): HostErrorEnvelope {
  return {
    code: 'host-unavailable',
    message:
      'preset publish는 authoring host에서만 실행할 수 있어요. 브라우저 미리보기에서는 future-session catalog를 바꾸지 않아요.',
  }
}

function buildHostCatalogStateUnavailableError(): HostErrorEnvelope {
  return {
    code: 'host-unavailable',
    message:
      'catalog version management는 authoring host에서만 실행할 수 있어요. 브라우저 미리보기에서는 live catalog 상태를 바꾸지 않아요.',
  }
}

export function createBrowserPresetAuthoringGateway(): PresetAuthoringGateway {
  return {
    async loadAuthoringWorkspace() {
      return buildBrowserWorkspace()
    },
    async createDraftPreset(input) {
      const store = getBrowserDraftStore()

      if (store.drafts.some((draft) => draft.presetId === input.presetId)) {
        throw {
          code: 'validation-error',
          message: '같은 presetId의 draft가 이미 있어요.',
        } satisfies HostErrorEnvelope
      }

      const summary = buildBrowserDraftSummary(input, 1)
      store.drafts = [summary, ...store.drafts]

      return summary
    },
    async saveDraftPreset(input) {
      const store = getBrowserDraftStore()
      const existingDraft = store.drafts.find((draft) => draft.presetId === input.presetId)

      if (!existingDraft) {
        throw {
          code: 'validation-error',
          message: '먼저 새 draft를 만들어 주세요.',
        } satisfies HostErrorEnvelope
      }

      const summary = buildBrowserDraftSummary(
        input,
        existingDraft.draftVersion + 1,
        existingDraft,
      )
      store.drafts = store.drafts.map((draft) =>
        draft.presetId === input.presetId ? summary : draft,
      )

      return summary
    },
    async validateDraftPreset(input) {
      const store = getBrowserDraftStore()
      const existingDraft = store.drafts.find((draft) => draft.presetId === input.presetId)

      if (!existingDraft) {
        throw {
          code: 'validation-error',
          message: '검증할 draft를 찾지 못했어요.',
        } satisfies HostErrorEnvelope
      }

      throw buildHostValidationUnavailableError()
    },
    async publishValidatedPreset(input) {
      const store = getBrowserDraftStore()
      const existingDraft = store.drafts.find((draft) => draft.presetId === input.presetId)

      if (!existingDraft) {
        throw {
          code: 'validation-error',
          message: '게시할 draft를 찾지 못했어요.',
        } satisfies HostErrorEnvelope
      }

      throw buildHostPublicationUnavailableError()
    },
    async loadPresetCatalogState() {
      throw buildHostCatalogStateUnavailableError()
    },
    async rollbackPresetCatalog(input) {
      void input
      throw buildHostCatalogStateUnavailableError()
    },
  }
}

export function createTauriPresetAuthoringGateway(): PresetAuthoringGateway {
  return {
    async loadAuthoringWorkspace() {
      return invoke<unknown>('load_authoring_workspace')
    },
    async createDraftPreset(input) {
      return invoke<unknown>('create_draft_preset', { input })
    },
    async saveDraftPreset(input) {
      return invoke<unknown>('save_draft_preset', { input })
    },
    async validateDraftPreset(input) {
      return invoke<unknown>('validate_draft_preset', { input })
    },
    async publishValidatedPreset(input) {
      return invoke<unknown>('publish_validated_preset', { input })
    },
    async loadPresetCatalogState() {
      return invoke<unknown>('load_preset_catalog_state')
    },
    async rollbackPresetCatalog(input) {
      return invoke<unknown>('rollback_preset_catalog', { input })
    },
  }
}

export function createDefaultPresetAuthoringGateway() {
  return isTauriRuntime()
    ? createTauriPresetAuthoringGateway()
    : createBrowserPresetAuthoringGateway()
}

type CreatePresetAuthoringServiceOptions = {
  gateway?: PresetAuthoringGateway
}

export function createPresetAuthoringService({
  gateway = createDefaultPresetAuthoringGateway(),
}: CreatePresetAuthoringServiceOptions = {}) {
  return new DefaultPresetAuthoringService(gateway)
}
