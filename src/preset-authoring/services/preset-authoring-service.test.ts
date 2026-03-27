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
