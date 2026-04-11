import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  createBrowserPresetAuthoringGateway,
  createPresetAuthoringService,
} from './preset-authoring-service'

const DRAFT_PAYLOAD = {
  presetId: 'preset_soft-glow-draft',
  displayName: 'Soft Glow Draft',
  lifecycleState: 'draft' as const,
  darktableVersion: '5.4.1',
  xmpTemplatePath: 'xmp/soft-glow.xmp',
  previewProfile: {
    profileId: 'preview-standard',
    displayName: 'Preview Standard',
    outputColorSpace: 'sRGB',
  },
  finalProfile: {
    profileId: 'final-standard',
    displayName: 'Final Standard',
    outputColorSpace: 'sRGB',
  },
  noisePolicy: {
    policyId: 'balanced-noise',
    displayName: 'Balanced Noise',
    reductionMode: 'balanced',
  },
  preview: {
    assetPath: 'previews/soft-glow.jpg',
    altText: 'Soft Glow draft portrait',
  },
  sampleCut: {
    assetPath: 'samples/soft-glow-cut.jpg',
    altText: 'Soft Glow sample cut',
  },
  description: '부드러운 피부톤 baseline',
  notes: '승인 전 내부 검토용',
}

function createValidatedDraft(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'draft-preset-artifact/v1',
    presetId: DRAFT_PAYLOAD.presetId,
    displayName: DRAFT_PAYLOAD.displayName,
    draftVersion: 1,
    lifecycleState: 'validated',
    darktableVersion: DRAFT_PAYLOAD.darktableVersion,
    xmpTemplatePath: DRAFT_PAYLOAD.xmpTemplatePath,
    previewProfile: DRAFT_PAYLOAD.previewProfile,
    finalProfile: DRAFT_PAYLOAD.finalProfile,
    noisePolicy: DRAFT_PAYLOAD.noisePolicy,
    preview: DRAFT_PAYLOAD.preview,
    sampleCut: DRAFT_PAYLOAD.sampleCut,
    description: DRAFT_PAYLOAD.description,
    notes: DRAFT_PAYLOAD.notes,
    validation: {
      status: 'passed',
      latestReport: {
        schemaVersion: 'draft-preset-validation/v1',
        presetId: DRAFT_PAYLOAD.presetId,
        draftVersion: 1,
        lifecycleState: 'validated',
        status: 'passed',
        checkedAt: '2026-03-26T00:10:00.000Z',
        findings: [],
      },
      history: [
        {
          schemaVersion: 'draft-preset-validation/v1',
          presetId: DRAFT_PAYLOAD.presetId,
          draftVersion: 1,
          lifecycleState: 'validated',
          status: 'passed',
          checkedAt: '2026-03-26T00:10:00.000Z',
          findings: [],
        },
      ],
    },
    publicationHistory: [],
    updatedAt: '2026-03-26T00:10:00.000Z',
    ...overrides,
  }
}

describe('browser preset authoring gateway', () => {
  afterEach(() => {
    delete (
      globalThis as typeof globalThis & {
        __BOOTHY_AUTHORING_DRAFT_STORE__?: unknown
      }
    ).__BOOTHY_AUTHORING_DRAFT_STORE__
  })

  it('keeps validation host-owned instead of computing approval-ready state in the browser', async () => {
    const service = createPresetAuthoringService({
      gateway: createBrowserPresetAuthoringGateway(),
    })

    await service.createDraftPreset(DRAFT_PAYLOAD)

    await expect(
      service.validateDraftPreset({
        presetId: DRAFT_PAYLOAD.presetId,
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })

    const workspace = await service.loadAuthoringWorkspace()

    expect(workspace.drafts).toHaveLength(1)
    expect(workspace.drafts[0]).toMatchObject({
      presetId: DRAFT_PAYLOAD.presetId,
      lifecycleState: 'draft',
      validation: {
        status: 'not-run',
        latestReport: null,
      },
    })
    expect(workspace.drafts[0]).not.toHaveProperty('darktableProjectPath')
  })

  it('keeps invalid draft repair host-owned instead of mutating browser preview storage', async () => {
    const service = createPresetAuthoringService({
      gateway: createBrowserPresetAuthoringGateway(),
    })

    await expect(
      service.repairInvalidDraft({
        draftFolder: 'preset_broken-draft',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })

  it('keeps publication host-owned instead of mutating future-session catalog in the browser', async () => {
    const service = createPresetAuthoringService({
      gateway: createBrowserPresetAuthoringGateway(),
    })

    await service.createDraftPreset(DRAFT_PAYLOAD)

    await expect(
      service.publishValidatedPreset({
        presetId: DRAFT_PAYLOAD.presetId,
        draftVersion: 1,
        validationCheckedAt: '2026-03-26T00:10:00.000Z',
        expectedDisplayName: DRAFT_PAYLOAD.displayName,
        publishedVersion: '2026.03.26',
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
        scope: 'future-sessions-only',
        reviewNote: '브라우저 미리보기에서는 비활성',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })

  it('returns a typed publication rejection when the stage is intentionally unavailable', async () => {
    const service = createPresetAuthoringService({
      gateway: {
        loadAuthoringWorkspace: vi.fn(),
        createDraftPreset: vi.fn(),
        saveDraftPreset: vi.fn(),
        validateDraftPreset: vi.fn(),
        repairInvalidDraft: vi.fn(),
        publishValidatedPreset: vi.fn().mockResolvedValue({
          schemaVersion: 'draft-preset-publication-result/v1',
          status: 'rejected',
          draft: createValidatedDraft(),
          reasonCode: 'stage-unavailable',
          message: '이 단계에서는 게시를 실행하지 않아요.',
          guidance: 'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
          auditRecord: {
            schemaVersion: 'preset-publication-audit/v1',
            presetId: DRAFT_PAYLOAD.presetId,
            draftVersion: 1,
            publishedVersion: '2026.03.26',
            actorId: 'manager-kim',
            actorLabel: 'Kim Manager',
            reviewNote: null,
            action: 'rejected',
            reasonCode: 'stage-unavailable',
            guidance:
              'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
            notedAt: '2026-03-26T00:20:00.000Z',
          },
        }),
        loadPresetCatalogState: vi.fn(),
        rollbackPresetCatalog: vi.fn(),
      },
    })

    await expect(
      service.publishValidatedPreset({
        presetId: DRAFT_PAYLOAD.presetId,
        draftVersion: 1,
        validationCheckedAt: '2026-03-26T00:10:00.000Z',
        expectedDisplayName: DRAFT_PAYLOAD.displayName,
        publishedVersion: '2026.03.26',
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
        scope: 'future-sessions-only',
        reviewNote: null,
      }),
    ).resolves.toMatchObject({
      status: 'rejected',
      reasonCode: 'stage-unavailable',
    })
  })

  it('accepts a stage-unavailable publication rejection for an already published draft', async () => {
    const service = createPresetAuthoringService({
      gateway: {
        loadAuthoringWorkspace: vi.fn(),
        createDraftPreset: vi.fn(),
        saveDraftPreset: vi.fn(),
        validateDraftPreset: vi.fn(),
        repairInvalidDraft: vi.fn(),
        publishValidatedPreset: vi.fn().mockResolvedValue({
          schemaVersion: 'draft-preset-publication-result/v1',
          status: 'rejected',
          draft: createValidatedDraft({
            lifecycleState: 'published',
            publicationHistory: [
              {
                schemaVersion: 'preset-publication-audit/v1',
                presetId: DRAFT_PAYLOAD.presetId,
                draftVersion: 1,
                publishedVersion: '2026.03.26',
                actorId: 'manager-kim',
                actorLabel: 'Kim Manager',
                reviewNote: null,
                action: 'published',
                reasonCode: null,
                guidance:
                  '게시가 완료되었고 이 버전은 미래 세션 catalog에서만 선택할 수 있어요.',
                notedAt: '2026-03-26T00:20:00.000Z',
              },
              {
                schemaVersion: 'preset-publication-audit/v1',
                presetId: DRAFT_PAYLOAD.presetId,
                draftVersion: 1,
                publishedVersion: '2026.03.27',
                actorId: 'manager-kim',
                actorLabel: 'Kim Manager',
                reviewNote: null,
                action: 'rejected',
                reasonCode: 'stage-unavailable',
                guidance:
                  'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
                notedAt: '2026-03-27T00:20:00.000Z',
              },
            ],
          }),
          reasonCode: 'stage-unavailable',
          message: '이 단계에서는 게시를 실행하지 않아요.',
          guidance: 'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
          auditRecord: {
            schemaVersion: 'preset-publication-audit/v1',
            presetId: DRAFT_PAYLOAD.presetId,
            draftVersion: 1,
            publishedVersion: '2026.03.27',
            actorId: 'manager-kim',
            actorLabel: 'Kim Manager',
            reviewNote: null,
            action: 'rejected',
            reasonCode: 'stage-unavailable',
            guidance:
              'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
            notedAt: '2026-03-27T00:20:00.000Z',
          },
        }),
        loadPresetCatalogState: vi.fn(),
        rollbackPresetCatalog: vi.fn(),
      },
    })

    await expect(
      service.publishValidatedPreset({
        presetId: DRAFT_PAYLOAD.presetId,
        draftVersion: 1,
        validationCheckedAt: '2026-03-26T00:10:00.000Z',
        expectedDisplayName: DRAFT_PAYLOAD.displayName,
        publishedVersion: '2026.03.27',
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
        scope: 'future-sessions-only',
        reviewNote: null,
      }),
    ).resolves.toMatchObject({
      status: 'rejected',
      reasonCode: 'stage-unavailable',
      draft: {
        lifecycleState: 'published',
      },
    })
  })

  it('rejects a stage-unavailable publication response that claims published state without prior publish history', async () => {
    const service = createPresetAuthoringService({
      gateway: {
        loadAuthoringWorkspace: vi.fn(),
        createDraftPreset: vi.fn(),
        saveDraftPreset: vi.fn(),
        validateDraftPreset: vi.fn(),
        repairInvalidDraft: vi.fn(),
        publishValidatedPreset: vi.fn().mockResolvedValue({
          schemaVersion: 'draft-preset-publication-result/v1',
          status: 'rejected',
          draft: createValidatedDraft({
            lifecycleState: 'published',
            publicationHistory: [
              {
                schemaVersion: 'preset-publication-audit/v1',
                presetId: DRAFT_PAYLOAD.presetId,
                draftVersion: 1,
                publishedVersion: '2026.03.27',
                actorId: 'manager-kim',
                actorLabel: 'Kim Manager',
                reviewNote: null,
                action: 'rejected',
                reasonCode: 'stage-unavailable',
                guidance:
                  'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
                notedAt: '2026-03-27T00:20:00.000Z',
              },
            ],
          }),
          reasonCode: 'stage-unavailable',
          message: '이 단계에서는 게시를 실행하지 않아요.',
          guidance: 'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
          auditRecord: {
            schemaVersion: 'preset-publication-audit/v1',
            presetId: DRAFT_PAYLOAD.presetId,
            draftVersion: 1,
            publishedVersion: '2026.03.27',
            actorId: 'manager-kim',
            actorLabel: 'Kim Manager',
            reviewNote: null,
            action: 'rejected',
            reasonCode: 'stage-unavailable',
            guidance:
              'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
            notedAt: '2026-03-27T00:20:00.000Z',
          },
        }),
        loadPresetCatalogState: vi.fn(),
        rollbackPresetCatalog: vi.fn(),
      },
    })

    await expect(
      service.publishValidatedPreset({
        presetId: DRAFT_PAYLOAD.presetId,
        draftVersion: 1,
        validationCheckedAt: '2026-03-26T00:10:00.000Z',
        expectedDisplayName: DRAFT_PAYLOAD.displayName,
        publishedVersion: '2026.03.27',
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
        scope: 'future-sessions-only',
        reviewNote: null,
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })

  it('rejects a stage-unavailable publication response that mixes a stale audit draft version', async () => {
    const service = createPresetAuthoringService({
      gateway: {
        loadAuthoringWorkspace: vi.fn(),
        createDraftPreset: vi.fn(),
        saveDraftPreset: vi.fn(),
        validateDraftPreset: vi.fn(),
        repairInvalidDraft: vi.fn(),
        publishValidatedPreset: vi.fn().mockResolvedValue({
          schemaVersion: 'draft-preset-publication-result/v1',
          status: 'rejected',
          draft: createValidatedDraft({
            draftVersion: 2,
            validation: {
              status: 'passed',
              latestReport: {
                schemaVersion: 'draft-preset-validation/v1',
                presetId: DRAFT_PAYLOAD.presetId,
                draftVersion: 2,
                lifecycleState: 'validated',
                status: 'passed',
                checkedAt: '2026-03-26T00:10:00.000Z',
                findings: [],
              },
              history: [
                {
                  schemaVersion: 'draft-preset-validation/v1',
                  presetId: DRAFT_PAYLOAD.presetId,
                  draftVersion: 2,
                  lifecycleState: 'validated',
                  status: 'passed',
                  checkedAt: '2026-03-26T00:10:00.000Z',
                  findings: [],
                },
              ],
            },
          }),
          reasonCode: 'stage-unavailable',
          message: '이 단계에서는 게시를 실행하지 않아요.',
          guidance: 'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
          auditRecord: {
            schemaVersion: 'preset-publication-audit/v1',
            presetId: DRAFT_PAYLOAD.presetId,
            draftVersion: 1,
            publishedVersion: '2026.03.26',
            actorId: 'manager-kim',
            actorLabel: 'Kim Manager',
            reviewNote: null,
            action: 'rejected',
            reasonCode: 'stage-unavailable',
            guidance:
              'approval 준비 상태까지만 확인하고, 실제 게시는 다음 단계에서 진행해 주세요.',
            notedAt: '2026-03-26T00:20:00.000Z',
          },
        }),
        loadPresetCatalogState: vi.fn(),
        rollbackPresetCatalog: vi.fn(),
      },
    })

    await expect(
      service.publishValidatedPreset({
        presetId: DRAFT_PAYLOAD.presetId,
        draftVersion: 1,
        validationCheckedAt: '2026-03-26T00:10:00.000Z',
        expectedDisplayName: DRAFT_PAYLOAD.displayName,
        publishedVersion: '2026.03.26',
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
        scope: 'future-sessions-only',
        reviewNote: null,
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })

  it('keeps rollback host-owned instead of rewriting catalog state in the browser', async () => {
    const service = createPresetAuthoringService({
      gateway: createBrowserPresetAuthoringGateway(),
    })

    await expect(service.loadPresetCatalogState()).rejects.toMatchObject({
      code: 'host-unavailable',
    })

    await expect(
      service.rollbackPresetCatalog({
        presetId: 'preset_soft-glow-draft',
        targetPublishedVersion: '2026.03.25',
        expectedCatalogRevision: 4,
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })

  it('returns a typed rollback rejection when the stage is intentionally unavailable', async () => {
    const service = createPresetAuthoringService({
      gateway: {
        loadAuthoringWorkspace: vi.fn(),
        createDraftPreset: vi.fn(),
        saveDraftPreset: vi.fn(),
        validateDraftPreset: vi.fn(),
        repairInvalidDraft: vi.fn(),
        publishValidatedPreset: vi.fn(),
        loadPresetCatalogState: vi.fn(),
        rollbackPresetCatalog: vi.fn().mockResolvedValue({
          schemaVersion: 'preset-catalog-rollback-result/v1',
          status: 'rejected',
          reasonCode: 'stage-unavailable',
          message: '이 단계에서는 롤백을 실행하지 않아요.',
          guidance: 'approval 준비 상태까지만 확인하고, 실제 롤백은 다음 단계에서 진행해 주세요.',
          catalogRevision: 4,
          summary: null,
        }),
      },
    })

    await expect(
      service.rollbackPresetCatalog({
        presetId: DRAFT_PAYLOAD.presetId,
        targetPublishedVersion: '2026.03.25',
        expectedCatalogRevision: 4,
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
      }),
    ).resolves.toMatchObject({
      status: 'rejected',
      reasonCode: 'stage-unavailable',
    })
  })
})
