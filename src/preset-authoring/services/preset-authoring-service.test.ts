import { afterEach, describe, expect, it } from 'vitest'

import {
  createBrowserPresetAuthoringGateway,
  createPresetAuthoringService,
} from './preset-authoring-service'

const DRAFT_PAYLOAD = {
  presetId: 'preset_soft-glow-draft',
  displayName: 'Soft Glow Draft',
  lifecycleState: 'draft' as const,
  darktableVersion: '5.4.1',
  darktableProjectPath: 'darktable/soft-glow.dtpreset',
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
})

describe('preset authoring service publication guardrails', () => {
  it('rejects a published host response for active-session scope', async () => {
    const service = createPresetAuthoringService({
      gateway: {
        loadAuthoringWorkspace: async () => ({
          schemaVersion: 'preset-authoring-workspace/v1',
          supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
          drafts: [],
          invalidDrafts: [],
        }),
        createDraftPreset: async () => DRAFT_PAYLOAD,
        saveDraftPreset: async () => DRAFT_PAYLOAD,
        validateDraftPreset: async () => {
          throw new Error('not used')
        },
        repairInvalidDraft: async () => undefined,
        loadPresetCatalogState: async () => {
          throw new Error('not used')
        },
        rollbackPresetCatalog: async () => {
          throw new Error('not used')
        },
        publishValidatedPreset: async () => ({
          schemaVersion: 'draft-preset-publication-result/v1',
          status: 'published',
          draft: {
            ...DRAFT_PAYLOAD,
            schemaVersion: 'draft-preset-artifact/v1',
            draftVersion: 1,
            lifecycleState: 'published',
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
            publicationHistory: [
              {
                schemaVersion: 'preset-publication-audit/v1',
                presetId: DRAFT_PAYLOAD.presetId,
                draftVersion: 1,
                publishedVersion: '2026.03.26',
                actorId: 'manager-kim',
                actorLabel: 'Kim Manager',
                reviewNote: null,
                action: 'approved',
                reasonCode: null,
                guidance: '승인 검토가 완료되었어요.',
                notedAt: '2026-03-26T00:11:00.000Z',
              },
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
                guidance: '게시가 완료되었어요.',
                notedAt: '2026-03-26T00:11:00.000Z',
              },
            ],
            updatedAt: '2026-03-26T00:11:00.000Z',
          },
          publishedPreset: {
            presetId: DRAFT_PAYLOAD.presetId,
            displayName: DRAFT_PAYLOAD.displayName,
            publishedVersion: '2026.03.26',
            boothStatus: 'booth-safe',
            preview: {
              kind: 'preview-tile',
              assetPath:
                'C:/boothy/preset-catalog/published/preset_soft-glow-draft/2026.03.26/preview/soft-glow.jpg',
              altText: 'Soft Glow draft portrait',
            },
          },
          bundlePath:
            'C:/boothy/preset-catalog/published/preset_soft-glow-draft/2026.03.26',
          auditRecord: {
            schemaVersion: 'preset-publication-audit/v1',
            presetId: DRAFT_PAYLOAD.presetId,
            draftVersion: 1,
            publishedVersion: '2026.03.26',
            actorId: 'manager-kim',
            actorLabel: 'Kim Manager',
            reviewNote: null,
            action: 'published',
            reasonCode: null,
            guidance: '게시가 완료되었어요.',
            notedAt: '2026-03-26T00:11:00.000Z',
          },
        }),
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
        scope: 'active-session',
        reviewNote: null,
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
    })
  })
})
