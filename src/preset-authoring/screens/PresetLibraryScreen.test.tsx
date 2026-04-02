import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RouterProvider, createMemoryRouter } from 'react-router-dom'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { createAppRoutes } from '../../app/routes'
import { createCapabilityService } from '../../app/services/capability-service'
import {
  createPresetAuthoringService,
  type PresetAuthoringGateway,
} from '../services/preset-authoring-service'

function createValidationReport(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'draft-preset-validation/v1',
    presetId: 'preset_soft-glow-draft',
    draftVersion: 2,
    lifecycleState: 'draft',
    status: 'failed',
    checkedAt: '2026-03-26T00:10:00.000Z',
    findings: [
      {
        ruleCode: 'missing-sample-cut',
        severity: 'error',
        fieldPath: 'sampleCut.assetPath',
        message: 'sample-cut 대표 자산이 없어요.',
        guidance: 'sampleCut.assetPath에 대표 샘플 이미지를 추가해 주세요.',
      },
    ],
    ...overrides,
  }
}

function createAuthoringDraft(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'draft-preset-artifact/v1',
    presetId: 'preset_soft-glow-draft',
    displayName: 'Soft Glow Draft',
    draftVersion: 2,
    lifecycleState: 'draft',
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
    description: '부드러운 피부톤을 위한 baseline',
    notes: '하이라이트를 조금 더 눌러 보기',
    validation: {
      status: 'not-run',
      latestReport: null,
      history: [],
    },
    publicationHistory: [],
    updatedAt: '2026-03-26T00:00:00.000Z',
    ...overrides,
  }
}

function createPublicationAuditRecord(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'preset-publication-audit/v1',
    presetId: 'preset_soft-glow-draft',
    draftVersion: 2,
    publishedVersion: '2026.03.26',
    actorId: 'manager-kim',
    actorLabel: 'Kim Manager',
    reviewNote: null,
    action: 'published',
    reasonCode: null,
    guidance: '게시가 완료되었고 이 버전은 미래 세션 catalog에서만 선택할 수 있어요.',
    notedAt: '2026-03-26T00:20:00.000Z',
    ...overrides,
  }
}

function createCatalogVersionHistoryItem(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'preset-catalog-history/v1',
    presetId: 'preset_soft-glow-draft',
    actionType: 'published',
    fromPublishedVersion: null,
    toPublishedVersion: '2026.03.26',
    actorId: 'manager-kim',
    actorLabel: 'Kim Manager',
    happenedAt: '2026-03-26T00:20:01.000Z',
    ...overrides,
  }
}

function createCatalogStateSummary(overrides: Record<string, unknown> = {}) {
  return {
    presetId: 'preset_soft-glow-draft',
    livePublishedVersion: '2026.03.26',
    publishedPresets: [
      {
        presetId: 'preset_soft-glow-draft',
        displayName: 'Soft Glow Draft',
        publishedVersion: '2026.03.25',
        boothStatus: 'booth-safe',
        preview: {
          kind: 'preview-tile',
          assetPath: 'published/preset_soft-glow-draft/2026.03.25/preview.jpg',
          altText: 'Soft Glow draft portrait 2026.03.25',
        },
      },
      {
        presetId: 'preset_soft-glow-draft',
        displayName: 'Soft Glow Draft',
        publishedVersion: '2026.03.26',
        boothStatus: 'booth-safe',
        preview: {
          kind: 'preview-tile',
          assetPath: 'published/preset_soft-glow-draft/2026.03.26/preview.jpg',
          altText: 'Soft Glow draft portrait 2026.03.26',
        },
      },
    ],
    versionHistory: [
      createCatalogVersionHistoryItem({
        actionType: 'published',
        fromPublishedVersion: null,
        toPublishedVersion: '2026.03.26',
      }),
    ],
    ...overrides,
  }
}

function renderAuthoringScreen({
  loadAuthoringWorkspace = vi.fn<PresetAuthoringGateway['loadAuthoringWorkspace']>(),
  createDraftPreset = vi.fn<PresetAuthoringGateway['createDraftPreset']>(),
  saveDraftPreset = vi.fn<PresetAuthoringGateway['saveDraftPreset']>(),
  validateDraftPreset = vi.fn<PresetAuthoringGateway['validateDraftPreset']>(),
  repairInvalidDraft = vi.fn<PresetAuthoringGateway['repairInvalidDraft']>(),
  publishValidatedPreset = vi.fn<PresetAuthoringGateway['publishValidatedPreset']>(),
  loadPresetCatalogState = vi
    .fn<PresetAuthoringGateway['loadPresetCatalogState']>()
    .mockResolvedValue({
      schemaVersion: 'preset-catalog-state-result/v1',
      catalogRevision: 0,
      presets: [],
    }),
  rollbackPresetCatalog = vi.fn<PresetAuthoringGateway['rollbackPresetCatalog']>(),
} = {}) {
  const presetAuthoringService = createPresetAuthoringService({
    gateway: {
      loadAuthoringWorkspace,
      createDraftPreset,
      saveDraftPreset,
      validateDraftPreset,
      repairInvalidDraft,
      publishValidatedPreset,
      loadPresetCatalogState,
      rollbackPresetCatalog,
    },
  })
  const router = createMemoryRouter(
    createAppRoutes({
      capabilityService: createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'authoring'],
        currentWindowLabel: 'authoring-window',
      }),
      presetAuthoringService,
    }),
    {
      initialEntries: ['/authoring'],
    },
  )

  render(<RouterProvider router={router} />)

  return {
    loadAuthoringWorkspace,
    createDraftPreset,
    saveDraftPreset,
    validateDraftPreset,
    repairInvalidDraft,
    publishValidatedPreset,
    loadPresetCatalogState,
    rollbackPresetCatalog,
  }
}

describe('PresetLibraryScreen', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('shows saved drafts to authorized internal users and keeps booth copy separate', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [],
      }),
      loadPresetCatalogState: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-catalog-state-result/v1',
        catalogRevision: 4,
        presets: [createCatalogStateSummary()],
      }),
    })

    expect(
      await screen.findByRole('heading', { name: /Draft Preset Workspace/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByText(/이 단계에서도 booth catalog와 현재 세션 binding은 즉시 바뀌지 않아요/i),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Soft Glow Draft/i })).toBeInTheDocument()
    expect(screen.getByDisplayValue('Soft Glow Draft')).toBeInTheDocument()
    expect(screen.getByText(/현재 future session live version/i)).toBeInTheDocument()
  })

  it(
    'creates a new draft and reflects internal save status without exposing booth affordances',
    async () => {
    const loadAuthoringWorkspace = vi
      .fn<PresetAuthoringGateway['loadAuthoringWorkspace']>()
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [],
        invalidDrafts: [],
      })
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [
          createAuthoringDraft({
            presetId: 'preset_porcelain-draft',
            displayName: 'Porcelain Draft',
            draftVersion: 1,
            preview: {
              assetPath: 'previews/porcelain.jpg',
              altText: 'Porcelain draft portrait',
            },
            sampleCut: {
              assetPath: 'samples/porcelain-cut.jpg',
              altText: 'Porcelain sample cut',
            },
            darktableProjectPath: 'darktable/porcelain.dtpreset',
            xmpTemplatePath: 'xmp/porcelain.xmp',
          }),
        ],
        invalidDrafts: [],
      })
    const createDraftPreset = vi
      .fn<PresetAuthoringGateway['createDraftPreset']>()
      .mockResolvedValue(
        createAuthoringDraft({
          presetId: 'preset_porcelain-draft',
          displayName: 'Porcelain Draft',
          draftVersion: 1,
          preview: {
            assetPath: 'previews/porcelain.jpg',
            altText: 'Porcelain draft portrait',
          },
          sampleCut: {
            assetPath: 'samples/porcelain-cut.jpg',
            altText: 'Porcelain sample cut',
          },
          darktableProjectPath: 'darktable/porcelain.dtpreset',
          xmpTemplatePath: 'xmp/porcelain.xmp',
        }),
      )

    renderAuthoringScreen({
      loadAuthoringWorkspace,
      createDraftPreset,
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /새 draft 만들기/i })

    await user.clear(screen.getByLabelText(/Preset ID/i))
    await user.type(screen.getByLabelText(/Preset ID/i), 'preset_porcelain-draft')
    await user.type(screen.getByLabelText(/Draft name/i), 'Porcelain Draft')
    await user.clear(screen.getByLabelText(/대표 preview 경로/i))
    await user.type(screen.getByLabelText(/대표 preview 경로/i), 'previews/porcelain.jpg')
    await user.type(screen.getByLabelText(/preview 설명/i), 'Porcelain draft portrait')
    await user.clear(screen.getByLabelText(/대표 sample-cut 경로/i))
    await user.type(screen.getByLabelText(/대표 sample-cut 경로/i), 'samples/porcelain-cut.jpg')
    await user.type(screen.getByLabelText(/sample-cut 설명/i), 'Porcelain sample cut')
    await user.clear(screen.getByLabelText(/darktable project 참조/i))
    await user.type(
      screen.getByLabelText(/darktable project 참조/i),
      'darktable/porcelain.dtpreset',
    )
    await user.clear(screen.getByLabelText(/XMP template 경로/i))
    await user.type(screen.getByLabelText(/XMP template 경로/i), 'xmp/porcelain.xmp')
    await user.type(screen.getByLabelText(/기본 설명/i), '매끈한 피부톤 기반 draft')
    await user.type(screen.getByLabelText(/내부 메모/i), '승인 전까지 booth catalog 비공개 유지')
    await user.click(screen.getByRole('button', { name: /draft 저장/i }))

    await waitFor(() => {
      expect(createDraftPreset).toHaveBeenCalledTimes(1)
    })
    expect(createDraftPreset.mock.calls[0]?.[0]).toMatchObject({
      presetId: expect.stringContaining('preset_porcelain-draft'),
      displayName: expect.stringContaining('Porcelain Draft'),
      lifecycleState: 'draft',
    })

    expect(
      await screen.findByText(/Porcelain Draft draft가 저장되었어요/i),
    ).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /authoring/i })).not.toBeInTheDocument()
    },
    10_000,
  )

  it('renders actionable validation findings when host validation fails and keeps the draft in internal state', async () => {
    const validationReport = createValidationReport()
    const loadAuthoringWorkspace = vi
      .fn<PresetAuthoringGateway['loadAuthoringWorkspace']>()
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [],
      })
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [
          createAuthoringDraft({
            validation: {
              status: 'failed',
              latestReport: validationReport,
              history: [validationReport],
            },
            updatedAt: validationReport.checkedAt,
          }),
        ],
        invalidDrafts: [],
      })
    const validateDraftPreset = vi
      .fn<PresetAuthoringGateway['validateDraftPreset']>()
      .mockResolvedValue({
        schemaVersion: 'draft-preset-validation-result/v1',
        draft: createAuthoringDraft({
          validation: {
            status: 'failed',
            latestReport: validationReport,
            history: [validationReport],
          },
          updatedAt: validationReport.checkedAt,
        }),
        report: validationReport,
      })

    renderAuthoringScreen({
      loadAuthoringWorkspace,
      validateDraftPreset,
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /호환성 검증 실행/i })
    await user.click(screen.getByRole('button', { name: /호환성 검증 실행/i }))

    await waitFor(() => {
      expect(validateDraftPreset).toHaveBeenCalledWith({
        presetId: 'preset_soft-glow-draft',
      })
    })

    expect(await screen.findByText(/아직 draft 상태예요/i)).toBeInTheDocument()
    expect(screen.getByText(/sample-cut 대표 자산이 없어요/i)).toBeInTheDocument()
    expect(
      screen.getByText(/sampleCut\.assetPath에 대표 샘플 이미지를 추가해 주세요/i),
    ).toBeInTheDocument()
  })

  it('requires the latest draft save before host validation can run', async () => {
    const validateDraftPreset = vi.fn<PresetAuthoringGateway['validateDraftPreset']>()

    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [],
      }),
      validateDraftPreset,
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /호환성 검증 실행/i })
    await user.clear(screen.getByLabelText(/Draft name/i))
    await user.type(screen.getByLabelText(/Draft name/i), 'Soft Glow Draft Updated')

    expect(
      screen.getByText(/저장되지 않은 변경이 있어요\. 현재 보이는 값으로 검증하려면 먼저 draft를 저장해 주세요/i),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /호환성 검증 실행/i })).toBeDisabled()
    expect(validateDraftPreset).not.toHaveBeenCalled()
  })

  it('prevents leaving a dirty existing draft before saving', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [
          createAuthoringDraft(),
          createAuthoringDraft({
            presetId: 'preset_fresh-contrast-draft',
            displayName: 'Fresh Contrast Draft',
            draftVersion: 1,
          }),
        ],
        invalidDrafts: [],
      }),
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /호환성 검증 실행/i })
    await user.clear(screen.getByLabelText(/Draft name/i))
    await user.type(screen.getByLabelText(/Draft name/i), 'Soft Glow Draft Updated')
    await user.click(screen.getByRole('button', { name: /Fresh Contrast Draft/i }))

    expect(
      screen.getByText(/먼저 draft를 저장하거나 변경을 되돌린 뒤 다른 draft를 열어 주세요/i),
    ).toBeInTheDocument()
    expect(screen.getByLabelText(/Preset ID/i)).not.toHaveValue('preset_fresh-contrast-draft')
    expect(screen.getByLabelText(/Draft name/i)).not.toHaveValue('Fresh Contrast Draft')

    await user.click(screen.getByRole('button', { name: /새 draft 만들기/i }))

    expect(
      screen.getByText(/먼저 draft를 저장하거나 변경을 되돌린 뒤 화면을 전환해 주세요/i),
    ).toBeInTheDocument()
    expect(screen.getByLabelText(/Preset ID/i)).not.toHaveValue('preset_new-draft')
  })

  it('lets the operator revert dirty changes before switching drafts', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [
          createAuthoringDraft(),
          createAuthoringDraft({
            presetId: 'preset_fresh-contrast-draft',
            displayName: 'Fresh Contrast Draft',
            draftVersion: 1,
          }),
        ],
        invalidDrafts: [],
      }),
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /호환성 검증 실행/i })
    await user.clear(screen.getByLabelText(/Draft name/i))
    await user.type(screen.getByLabelText(/Draft name/i), 'Soft Glow Draft Updated')
    await user.click(screen.getByRole('button', { name: /변경 되돌리기/i }))
    expect(screen.getByDisplayValue('Soft Glow Draft')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /Fresh Contrast Draft/i }))

    expect(screen.getByDisplayValue('Fresh Contrast Draft')).toBeInTheDocument()
  })

  it('prevents leaving a dirty new draft before saving', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [],
      }),
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /새 draft 만들기/i })
    await user.click(screen.getByRole('button', { name: /새 draft 만들기/i }))
    await user.clear(screen.getByLabelText(/Preset ID/i))
    await user.type(screen.getByLabelText(/Preset ID/i), 'preset_porcelain-draft')
    await user.type(screen.getByLabelText(/Draft name/i), 'Porcelain Draft')
    await user.click(screen.getByRole('button', { name: /Soft Glow Draft/i }))

    expect(
      screen.getByText(/먼저 draft를 저장하거나 변경을 되돌린 뒤 다른 draft를 열어 주세요/i),
    ).toBeInTheDocument()
    expect(screen.getByLabelText(/Preset ID/i)).not.toHaveValue('preset_soft-glow-draft')
    expect(screen.getByLabelText(/Draft name/i)).not.toHaveValue('Soft Glow Draft')
  })

  it('lets the operator revert a new draft baseline before opening an existing draft', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [],
      }),
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /새 draft 만들기/i })
    await user.click(screen.getByRole('button', { name: /새 draft 만들기/i }))
    await user.clear(screen.getByLabelText(/Preset ID/i))
    await user.type(screen.getByLabelText(/Preset ID/i), 'preset_porcelain-draft')
    await user.type(screen.getByLabelText(/Draft name/i), 'Porcelain Draft')
    await user.click(screen.getByRole('button', { name: /변경 되돌리기/i }))
    await user.click(screen.getByRole('button', { name: /Soft Glow Draft/i }))

    expect(screen.getByDisplayValue('preset_soft-glow-draft')).toBeInTheDocument()
    expect(screen.getByDisplayValue('Soft Glow Draft')).toBeInTheDocument()
  })

  it('marks the draft as approval-ready when validation passes without implying publication', async () => {
    const passedReport = createValidationReport({
      lifecycleState: 'validated',
      status: 'passed',
      findings: [],
    })
    const validatedDraft = createAuthoringDraft({
      lifecycleState: 'validated',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      updatedAt: passedReport.checkedAt,
    })
    const loadAuthoringWorkspace = vi
      .fn<PresetAuthoringGateway['loadAuthoringWorkspace']>()
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [],
      })
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [validatedDraft],
        invalidDrafts: [],
      })
    const validateDraftPreset = vi
      .fn<PresetAuthoringGateway['validateDraftPreset']>()
      .mockResolvedValue({
        schemaVersion: 'draft-preset-validation-result/v1',
        draft: validatedDraft,
        report: passedReport,
      })

    renderAuthoringScreen({
      loadAuthoringWorkspace,
      validateDraftPreset,
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /호환성 검증 실행/i })
    await user.click(screen.getByRole('button', { name: /호환성 검증 실행/i }))

    expect(
      await screen.findByText(/approval 준비 완료 상태로 전환되었어요/i),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /게시 승인 실행/i })).toBeInTheDocument()
    expect(screen.getByLabelText(/Published version/i)).toBeInTheDocument()
  })

  it('shows publish review controls for validated drafts and keeps booth copy separate', async () => {
    const passedReport = createValidationReport({
      lifecycleState: 'validated',
      status: 'passed',
      findings: [],
    })
    const validatedDraft = createAuthoringDraft({
      lifecycleState: 'validated',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      updatedAt: passedReport.checkedAt,
    })

    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [validatedDraft],
        invalidDrafts: [],
      }),
    })

    expect(
      await screen.findByText(
        /승인 게시는 future session catalog에만 반영되고, 현재 세션과 이미 저장된 capture binding은 계속 그대로 유지돼요\./i,
      ),
    ).toBeInTheDocument()
    expect(screen.getByText(/handoff 준비/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /게시 승인 실행/i })).toBeInTheDocument()
    expect(
      screen.getByText(/approval-ready draft를 게시하면 여기에서 승인과 게시 경계를 추적할 수 있어요/i),
    ).toBeInTheDocument()
  })

  it('surfaces corrupted draft entries as repair-needed items in the library', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [
          {
            draftFolder: 'preset_broken-draft',
            message: '저장된 draft JSON 형식이 손상되어 작업공간에서 열 수 없어요.',
            guidance:
              '목록에서 손상 draft 정리를 실행한 뒤 새 draft를 만들고 메타데이터와 자산 참조를 다시 저장해 주세요.',
            canRepair: true,
          },
        ],
      }),
    })

    expect(await screen.findByText(/복구 필요 · preset_broken-draft/i)).toBeInTheDocument()
    expect(
      screen.getByText(/저장된 draft JSON 형식이 손상되어 작업공간에서 열 수 없어요/i),
    ).toBeInTheDocument()
    expect(
      screen.getByText(
        /목록에서 손상 draft 정리를 실행한 뒤 새 draft를 만들고 메타데이터와 자산 참조를 다시 저장해 주세요/i,
      ),
    ).toBeInTheDocument()
  })

  it('repairs a corrupted draft entry from the library without disturbing the current draft form', async () => {
    const repairInvalidDraft = vi.fn<PresetAuthoringGateway['repairInvalidDraft']>().mockResolvedValue(
      undefined,
    )

    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [
          {
            draftFolder: 'preset_broken-draft',
            message: '저장된 draft JSON 형식이 손상되어 작업공간에서 열 수 없어요.',
            guidance:
              '목록에서 손상 draft 정리를 실행한 뒤 새 draft를 만들고 메타데이터와 자산 참조를 다시 저장해 주세요.',
            canRepair: true,
          },
        ],
      }),
      repairInvalidDraft,
    })

    const user = userEvent.setup()

    await screen.findByText(/복구 필요 · preset_broken-draft/i)
    await user.click(screen.getByRole('button', { name: /손상 draft 정리/i }))

    expect(repairInvalidDraft).toHaveBeenCalledWith({
      draftFolder: 'preset_broken-draft',
    })
    expect(screen.queryByText(/복구 필요 · preset_broken-draft/i)).not.toBeInTheDocument()
    expect(screen.getByDisplayValue('Soft Glow Draft')).toBeInTheDocument()
    expect(
      screen.getByText(
        /preset_broken-draft 손상 draft 기록을 정리했어요\. 같은 presetId로 새 draft를 다시 만들 수 있어요\./i,
      ),
    ).toBeInTheDocument()
  })

  it('keeps manual-inspection drafts visible without exposing the destructive repair action', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [createAuthoringDraft()],
        invalidDrafts: [
          {
            draftFolder: 'preset_folder-mismatch',
            message: 'draft 폴더 이름과 저장된 presetId가 서로 달라 자동 정리를 막았어요.',
            guidance:
              '자동 삭제 대신 작업공간을 수동 점검해 주세요. 폴더 이름과 presetId를 맞추면 기존 draft와 자산을 보존할 수 있어요.',
            canRepair: false,
          },
        ],
      }),
    })

    expect(await screen.findByText(/복구 필요 · preset_folder-mismatch/i)).toBeInTheDocument()
    expect(
      screen.getByText(/자동 삭제 대신 작업공간을 수동 점검해 주세요/i),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /손상 draft 정리/i })).not.toBeInTheDocument()
  })

  it('publishes a validated draft and surfaces approval and publication history', async () => {
    const passedReport = createValidationReport({
      lifecycleState: 'validated',
      status: 'passed',
      findings: [],
    })
    const validatedDraft = createAuthoringDraft({
      lifecycleState: 'validated',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      updatedAt: passedReport.checkedAt,
    })
    const publishedDraft = createAuthoringDraft({
      draftVersion: 2,
      lifecycleState: 'published',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      publicationHistory: [
        createPublicationAuditRecord({
          action: 'approved',
          reviewNote: '현재 세션 유지',
          guidance: '승인 검토가 완료되었고 immutable 게시 아티팩트를 확정하고 있어요.',
          notedAt: '2026-03-26T00:20:00.000Z',
        }),
        createPublicationAuditRecord({
          action: 'published',
          notedAt: '2026-03-26T00:20:01.000Z',
        }),
      ],
      updatedAt: '2026-03-26T00:20:01.000Z',
    })
    const loadAuthoringWorkspace = vi
      .fn<PresetAuthoringGateway['loadAuthoringWorkspace']>()
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [validatedDraft],
      })
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [publishedDraft],
      })
    const publishValidatedPreset = vi
      .fn<PresetAuthoringGateway['publishValidatedPreset']>()
      .mockResolvedValue({
        schemaVersion: 'draft-preset-publication-result/v1',
        status: 'published',
        draft: publishedDraft,
        publishedPreset: {
          presetId: 'preset_soft-glow-draft',
          displayName: 'Soft Glow Draft',
          publishedVersion: '2026.03.26',
          boothStatus: 'booth-safe',
          preview: {
            kind: 'preview-tile',
            assetPath: 'C:/boothy/preset-catalog/published/preset_soft-glow-draft/2026.03.26/preview/soft-glow.jpg',
            altText: 'Soft Glow draft portrait',
          },
        },
        bundlePath:
          'C:/boothy/preset-catalog/published/preset_soft-glow-draft/2026.03.26',
        auditRecord: createPublicationAuditRecord({
          action: 'published',
          notedAt: '2026-03-26T00:20:01.000Z',
        }),
      })

    renderAuthoringScreen({
      loadAuthoringWorkspace,
      publishValidatedPreset,
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /게시 승인 실행/i })
    await user.clear(screen.getByLabelText(/Published version/i))
    await user.type(screen.getByLabelText(/Published version/i), '2026.03.26')
    await user.type(screen.getByLabelText(/승인자 ID/i), 'manager-kim')
    await user.type(screen.getByLabelText(/승인자 이름/i), 'Kim Manager')
    await user.type(screen.getByLabelText(/검토 메모/i), '현재 세션 유지')
    await user.click(screen.getByRole('button', { name: /게시 승인 실행/i }))

    await waitFor(() => {
      expect(publishValidatedPreset).toHaveBeenCalledWith(
        expect.objectContaining({
          presetId: 'preset_soft-glow-draft',
          publishedVersion: '2026.03.26',
          actorId: 'manager-kim',
          actorLabel: 'Kim Manager',
          scope: 'future-sessions-only',
          reviewNote: '현재 세션 유지',
        }),
      )
    })

    expect(
      await screen.findByText(/승인 게시가 완료되었어요\. 새 버전은 미래 세션 catalog에만 반영되고 현재 세션은 그대로 유지돼요/i),
    ).toBeInTheDocument()
    expect(screen.getByText(/승인 완료 · 2026\.03\.26 · Kim Manager/i)).toBeInTheDocument()
    expect(screen.getByText(/게시 완료 · 2026\.03\.26 · Kim Manager/i)).toBeInTheDocument()
    expect(screen.getByText(/검토 메모: 현재 세션 유지/i)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /게시 승인 실행/i })).not.toBeInTheDocument()
  })

  it('shows actionable rejection guidance when publication is rejected', async () => {
    const passedReport = createValidationReport({
      lifecycleState: 'validated',
      status: 'passed',
      findings: [],
    })
    const validatedDraft = createAuthoringDraft({
      lifecycleState: 'validated',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      updatedAt: passedReport.checkedAt,
    })
    const rejectedDraft = createAuthoringDraft({
      lifecycleState: 'validated',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      publicationHistory: [
        createPublicationAuditRecord({
          action: 'rejected',
          reasonCode: 'duplicate-version',
          guidance: '새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.',
        }),
      ],
      updatedAt: '2026-03-26T00:20:01.000Z',
    })
    const loadAuthoringWorkspace = vi
      .fn<PresetAuthoringGateway['loadAuthoringWorkspace']>()
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [validatedDraft],
      })
      .mockResolvedValueOnce({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [rejectedDraft],
      })
    const publishValidatedPreset = vi
      .fn<PresetAuthoringGateway['publishValidatedPreset']>()
      .mockResolvedValue({
        schemaVersion: 'draft-preset-publication-result/v1',
        status: 'rejected',
        draft: rejectedDraft,
        reasonCode: 'duplicate-version',
        message: '같은 published version이 이미 존재해서 immutable 게시 규칙을 지킬 수 없어요.',
        guidance: '새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.',
        auditRecord: createPublicationAuditRecord({
          action: 'rejected',
          reasonCode: 'duplicate-version',
          guidance: '새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.',
        }),
      })

    renderAuthoringScreen({
      loadAuthoringWorkspace,
      publishValidatedPreset,
    })

    const user = userEvent.setup()

    await screen.findByRole('button', { name: /게시 승인 실행/i })
    await user.clear(screen.getByLabelText(/Published version/i))
    await user.type(screen.getByLabelText(/Published version/i), '2026.03.26')
    await user.type(screen.getByLabelText(/승인자 ID/i), 'manager-kim')
    await user.type(screen.getByLabelText(/승인자 이름/i), 'Kim Manager')
    await user.click(screen.getByRole('button', { name: /게시 승인 실행/i }))

    expect(
      await screen.findAllByText(
        /같은 published version이 이미 존재해서 immutable 게시 규칙을 지킬 수 없어요/i,
      ),
    ).toHaveLength(2)
    expect(
      screen.getAllByText(
        /새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요/i,
      ),
    ).toHaveLength(2)
    expect(screen.getByText(/사유: 중복 게시 버전/i)).toBeInTheDocument()
  })

  it('treats approved or published records as read-only follow-up states in this screen', async () => {
    const passedReport = createValidationReport({
      lifecycleState: 'validated',
      status: 'passed',
      findings: [],
    })
    const publishedDraft = createAuthoringDraft({
      lifecycleState: 'published',
      validation: {
        status: 'passed',
        latestReport: passedReport,
        history: [passedReport],
      },
      publicationHistory: [
        createPublicationAuditRecord({
          action: 'approved',
          reviewNote: '현재 세션 유지',
          guidance: '승인 검토가 완료되었고 immutable 게시 아티팩트를 확정하고 있어요.',
        }),
        createPublicationAuditRecord(),
      ],
      updatedAt: passedReport.checkedAt,
    })

    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [publishedDraft],
      }),
    })

    expect(await screen.findAllByText(/게시 완료/i)).not.toHaveLength(0)
    expect(
      screen.getByText(/이 화면에서는 읽기 전용으로만 확인하고, 후속 수정이 필요하면 새 draft를 만들어/i),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /호환성 검증 실행/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /draft 업데이트/i })).not.toBeInTheDocument()
    expect(screen.getByLabelText(/Draft name/i)).toBeDisabled()
  })

  it('hides authoring controls when the host rejects access for the current window', async () => {
    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockRejectedValue({
        code: 'capability-denied',
        message: '승인된 내부 authoring 세션에서만 draft 작업공간을 열 수 있어요.',
      }),
      loadPresetCatalogState: vi.fn().mockRejectedValue({
        code: 'capability-denied',
        message: '승인된 내부 authoring 세션에서만 catalog 상태를 볼 수 있어요.',
      }),
    })

    expect(
      await screen.findByText(/authoring 제어를 표시하지 않았어요/i),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /새 draft 만들기/i })).not.toBeInTheDocument()
    expect(screen.queryByLabelText(/Preset ID/i)).not.toBeInTheDocument()
  })

  it('shows rollback candidates and updates the live catalog version after confirmation', async () => {
    const rolledBackSummary = createCatalogStateSummary({
      livePublishedVersion: '2026.03.25',
      versionHistory: [
        createCatalogVersionHistoryItem({
          actionType: 'published',
          fromPublishedVersion: null,
          toPublishedVersion: '2026.03.26',
          happenedAt: '2026-03-26T00:20:01.000Z',
        }),
        createCatalogVersionHistoryItem({
          actionType: 'rollback',
          fromPublishedVersion: '2026.03.26',
          toPublishedVersion: '2026.03.25',
          happenedAt: '2026-03-26T00:30:01.000Z',
        }),
      ],
    })
    const loadPresetCatalogState = vi
      .fn<PresetAuthoringGateway['loadPresetCatalogState']>()
      .mockResolvedValueOnce({
        schemaVersion: 'preset-catalog-state-result/v1',
        catalogRevision: 4,
        presets: [createCatalogStateSummary()],
      })
      .mockResolvedValueOnce({
        schemaVersion: 'preset-catalog-state-result/v1',
        catalogRevision: 5,
        presets: [rolledBackSummary],
      })
    const rollbackPresetCatalog = vi
      .fn<PresetAuthoringGateway['rollbackPresetCatalog']>()
      .mockResolvedValue({
        schemaVersion: 'preset-catalog-rollback-result/v1',
        status: 'rolled-back',
        catalogRevision: 5,
        summary: rolledBackSummary,
        auditEntry: createCatalogVersionHistoryItem({
          actionType: 'rollback',
          fromPublishedVersion: '2026.03.26',
          toPublishedVersion: '2026.03.25',
          happenedAt: '2026-03-26T00:30:01.000Z',
        }),
        message:
          '선택한 승인 버전으로 되돌렸어요. 이미 진행 중인 세션은 기존 바인딩을 계속 유지해요.',
      })

    renderAuthoringScreen({
      loadAuthoringWorkspace: vi.fn().mockResolvedValue({
        schemaVersion: 'preset-authoring-workspace/v1',
        supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
        drafts: [],
      }),
      loadPresetCatalogState,
      rollbackPresetCatalog,
    })

    const user = userEvent.setup()

    await screen.findByText(/현재 future session live version/i)
    await user.selectOptions(
      screen.getByLabelText(/Rollback target version/i),
      '2026.03.25',
    )
    await user.type(screen.getByLabelText(/롤백 승인자 ID/i), 'manager-kim')
    await user.type(screen.getByLabelText(/롤백 승인자 이름/i), 'Kim Manager')
    await user.click(screen.getByRole('button', { name: /선택한 버전으로 롤백/i }))

    await waitFor(() => {
      expect(rollbackPresetCatalog).toHaveBeenCalledWith({
        presetId: 'preset_soft-glow-draft',
        targetPublishedVersion: '2026.03.25',
        expectedCatalogRevision: 4,
        actorId: 'manager-kim',
        actorLabel: 'Kim Manager',
      })
    })

    expect(
      await screen.findByText(
        /선택한 승인 버전으로 되돌렸어요\. 이미 진행 중인 세션은 기존 바인딩을 계속 유지해요\./i,
      ),
    ).toBeInTheDocument()
    expect(screen.getByText(/현재 future session live version: 2026\.03\.25/i)).toBeInTheDocument()
    expect(screen.getByText(/rollback · 2026\.03\.25 · Kim Manager/i)).toBeInTheDocument()
  })
})
