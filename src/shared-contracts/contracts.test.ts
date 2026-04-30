import { describe, expect, it } from 'vitest'

import {
  authoringWorkspaceResultSchema,
  catalogStateResultSchema,
  catalogStateSummarySchema,
  catalogVersionHistoryItemSchema,
  captureDeleteResultSchema,
  captureExportResultSchema,
  activePresetBindingSchema,
  boothSessionStubSchema,
  captureReadinessSnapshotSchema,
  captureRequestResultSchema,
  capabilitySnapshotSchema,
  draftPresetEditPayloadSchema,
  draftPresetSummarySchema,
  draftValidationReportSchema,
  hostErrorEnvelopeSchema,
  operatorBlockedStateCategorySchema,
  operatorRecoveryActionRequestSchema,
  operatorRecoveryActionResultSchema,
  operatorRecoveryActionSchema,
  operatorRecoveryBlockedCategorySchema,
  operatorRecoverySummarySchema,
  operatorSessionSummarySchema,
  publicationAuditRecordSchema,
  presetCatalogResultSchema,
  presetLifecycleStateSchema,
  presetSelectionInputSchema,
  repairInvalidDraftInputSchema,
  rollbackPresetCatalogInputSchema,
  rollbackPresetCatalogResultSchema,
  publishValidatedPresetInputSchema,
  publishValidatedPresetResultSchema,
  publishedPresetSummarySchema,
  sessionManifestSchema,
  sessionCaptureRecordSchema,
  sessionStartInputSchema,
  sessionStartResultSchema,
  sessionTimingSnapshotSchema,
  validateDraftPresetInputSchema,
  validateDraftPresetResultSchema,
} from './index'

function createDraftValidationReport(overrides: Record<string, unknown> = {}) {
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
        guidance: 'sampleCut.assetPath에 draft 작업공간 안의 대표 샘플 이미지를 추가해 주세요.',
      },
    ],
    ...overrides,
  }
}

function createDraftPresetSummary(overrides: Record<string, unknown> = {}) {
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
    description: '부드러운 피부톤 baseline',
    notes: '하이라이트 재검토',
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
        toPublishedVersion: '2026.03.25',
        happenedAt: '2026-03-25T00:20:01.000Z',
      }),
      createCatalogVersionHistoryItem({
        actionType: 'rollback',
        fromPublishedVersion: '2026.03.26',
        toPublishedVersion: '2026.03.25',
        happenedAt: '2026-03-26T00:25:01.000Z',
      }),
    ],
    ...overrides,
  }
}

function createOperatorSessionSummary(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-session-summary/v1',
    state: 'session-loaded',
    blockedStateCategory: 'preview-render-blocked',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetId: 'preset_soft-glow',
    activePresetDisplayName: 'Soft Glow',
    activePresetVersion: '2026.03.26',
    lifecycleStage: 'preview-waiting',
    timingPhase: 'active',
    updatedAt: '2026-03-26T00:10:00.000Z',
    postEndState: null,
    recentFailure: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    cameraConnection: {
      state: 'connected',
      title: '카메라와 helper 연결이 확인됐어요.',
      detail: '카메라와 helper가 현재 세션 기준으로 연결된 상태예요.',
      observedAt: '2026-03-26T00:10:00.000Z',
    },
    captureBoundary: {
      status: 'clear',
      title: '캡처 경계 정상',
      detail: '활성 preset이 선택돼 있어 capture 경계는 열려 있어요.',
    },
    previewRenderBoundary: {
      status: 'blocked',
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 preview/render 결과가 아직 준비되지 않았어요.',
    },
    completionBoundary: {
      status: 'clear',
      title: '완료 경계 대기 전',
      detail: '아직 종료 후 완료 경계로 들어가지 않았어요.',
    },
    liveCaptureTruth: {
      source: 'canon-helper-sidecar',
      freshness: 'fresh',
      sessionMatch: 'matched',
      cameraState: 'ready',
      helperState: 'healthy',
      observedAt: '2026-03-26T00:10:00.000Z',
      sequence: 42,
      detailCode: 'camera-ready',
    },
    ...overrides,
  }
}

function createOperatorRecoverySummary(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-recovery-summary/v1',
    state: 'session-loaded',
    blockedStateCategory: 'preview-render-blocked',
    blockedCategory: 'preview-or-render',
    diagnosticsSummary: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    allowedActions: ['retry', 'approved-boundary-restart', 'route-phone-required'],
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetId: 'preset_soft-glow',
    activePresetDisplayName: 'Soft Glow',
    activePresetVersion: '2026.03.26',
    lifecycleStage: 'preview-waiting',
    timingPhase: 'active',
    updatedAt: '2026-03-26T00:10:00.000Z',
    postEndState: null,
    recentFailure: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    cameraConnection: {
      state: 'connected',
      title: '카메라와 helper 연결이 확인됐어요.',
      detail: '카메라와 helper가 현재 세션 기준으로 연결된 상태예요.',
      observedAt: '2026-03-26T00:10:00.000Z',
    },
    captureBoundary: {
      status: 'clear',
      title: '캡처 경계 정상',
      detail: '활성 preset이 선택돼 있어 capture 경계는 열려 있어요.',
    },
    previewRenderBoundary: {
      status: 'blocked',
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 preview/render 결과가 아직 준비되지 않았어요.',
    },
    completionBoundary: {
      status: 'clear',
      title: '완료 경계 대기 전',
      detail: '아직 종료 후 완료 경계로 들어가지 않았어요.',
    },
    ...overrides,
  }
}

function createOperatorRecoveryActionResult(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-recovery-action-result/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    action: 'retry',
    status: 'applied',
    message: '현재 막힌 preview/render 경계를 다시 시도했어요.',
    rejectionReason: null,
    diagnosticsSummary: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    nextState: {
      customerState: 'Ready',
      reasonCode: 'ready',
      lifecycleStage: 'capture-ready',
      timingPhase: 'active',
      postEndState: null,
    },
    summary: createOperatorRecoverySummary({
      blockedStateCategory: 'not-blocked',
      blockedCategory: null,
      diagnosticsSummary: null,
      allowedActions: [],
      lifecycleStage: 'capture-ready',
      previewRenderBoundary: {
        status: 'clear',
        title: '프리뷰/렌더 경계 정상',
        detail: '가장 최근 촬영본의 결과 준비가 끝나 있어요.',
      },
    }),
    ...overrides,
  }
}

describe('shared contracts baseline', () => {
  it('normalizes booth capability access to always include the booth surface', () => {
    const parsed = capabilitySnapshotSchema.parse({
      isAdminAuthenticated: false,
      allowedSurfaces: [],
    })

    expect(parsed.allowedSurfaces).toContain('booth')
    expect(parsed.allowedSurfaces).toHaveLength(1)
  })

  it('parses a placeholder booth session DTO', () => {
    const parsed = boothSessionStubSchema.parse({
      sessionId: 'session_001',
      boothAlias: 'KIM-4821',
      presetId: 'preset_neutral',
    })

    expect(parsed.sessionId).toBe('session_001')
    expect(parsed.boothAlias).toBe('KIM-4821')
    expect(parsed.presetId).toBe('preset_neutral')
  })

  it('accepts a valid session start input payload', () => {
    const parsed = sessionStartInputSchema.parse({
      name: 'Kim Noah',
      phoneLastFour: '4821',
    })

    expect(parsed.name).toBe('Kim Noah')
    expect(parsed.phoneLastFour).toBe('4821')
  })

  it('rejects invalid session start input payloads', () => {
    expect(() =>
      sessionStartInputSchema.parse({
        name: '   ',
        phoneLastFour: '12a4',
      }),
    ).toThrow()

    expect(() =>
      sessionStartInputSchema.parse({
        name: 'Kim Noah',
        phoneLastFour: '821',
      }),
    ).toThrow()
  })

  it('parses the session manifest v1 baseline', () => {
    const parsed = sessionManifestSchema.parse({
      schemaVersion: 'session-manifest/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      customer: {
        name: 'Kim',
        phoneLastFour: '4821',
      },
      createdAt: '2026-03-20T00:00:00.000Z',
      updatedAt: '2026-03-20T00:00:00.000Z',
      lifecycle: {
        status: 'active',
        stage: 'session-started',
      },
      catalogRevision: null,
      catalogSnapshot: null,
      activePreset: null,
      captures: [],
      postEnd: null,
    })

    expect(parsed.boothAlias).toBe('Kim 4821')
    expect(parsed.captures).toEqual([])
    expect(parsed.lifecycle.stage).toBe('session-started')
    expect(parsed.catalogRevision).toBeNull()
    expect(parsed.catalogSnapshot).toBeNull()
  })

  it('accepts later lifecycle stages so follow-up stories can preserve session progress', () => {
    const parsed = sessionManifestSchema.parse({
      schemaVersion: 'session-manifest/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      customer: {
        name: 'Kim',
        phoneLastFour: '4821',
      },
      createdAt: '2026-03-20T00:00:00.000Z',
      updatedAt: '2026-03-20T00:00:00.000Z',
      lifecycle: {
        status: 'active',
        stage: 'capture-ready',
      },
      catalogRevision: 3,
      catalogSnapshot: [
        {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
      ],
      activePreset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
      activePresetDisplayName: 'Soft Glow',
      captures: [],
      postEnd: null,
    })

    expect(parsed.lifecycle.stage).toBe('capture-ready')
    expect(parsed.catalogRevision).toBe(3)
    expect(parsed.catalogSnapshot).toHaveLength(1)
  })

  it('requires a completion variant before post-end completed truth can be claimed', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:00.000Z',
        lifecycle: {
          status: 'active',
          stage: 'completed',
        },
        activePreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
        captures: [],
        postEnd: {
          state: 'completed',
          evaluatedAt: '2026-03-20T00:15:00.000Z',
        },
      }),
    ).toThrow()
  })

  it('parses a typed session start result and serializable host error envelope', () => {
    const result = sessionStartResultSchema.parse({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      manifest: {
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:00.000Z',
        lifecycle: {
          status: 'active',
          stage: 'session-started',
        },
        catalogRevision: null,
        catalogSnapshot: null,
        activePreset: null,
        captures: [],
        postEnd: null,
      },
    })

    const error = hostErrorEnvelopeSchema.parse({
      code: 'validation-error',
      message: '휴대전화 뒤 4자리를 확인해 주세요.',
      fieldErrors: {
        phoneLastFour: '숫자 4자리여야 해요.',
      },
    })

    expect(result.manifest.sessionId).toBe(result.sessionId)
    expect(result.manifest.catalogRevision).toBeNull()
    expect(error.fieldErrors?.phoneLastFour).toBe('숫자 4자리여야 해요.')
  })

  it('rejects partially pinned catalog metadata in the session manifest', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:00.000Z',
        lifecycle: {
          status: 'active',
          stage: 'session-started',
        },
        catalogRevision: 3,
        catalogSnapshot: null,
        activePreset: null,
        captures: [],
        postEnd: null,
      }),
    ).toThrow(/catalogRevision과 catalogSnapshot은 함께 기록/)
  })

  it('parses host-owned session timing truth with warning and end markers', () => {
    const parsed = sessionTimingSnapshotSchema.parse({
      schemaVersion: 'session-timing/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      adjustedEndAt: '2026-03-20T00:15:00.000Z',
      warningAt: '2026-03-20T00:10:00.000Z',
      phase: 'warning',
      captureAllowed: true,
      approvedExtensionMinutes: 0,
      approvedExtensionAuditRef: null,
      warningTriggeredAt: '2026-03-20T00:10:01.000Z',
      endedTriggeredAt: null,
    })

    expect(parsed.phase).toBe('warning')
    expect(parsed.captureAllowed).toBe(true)
  })

  it('parses an operator session summary with normalized blocked-state diagnostics', () => {
    const parsed = operatorSessionSummarySchema.parse(
      createOperatorSessionSummary(),
    )

    expect(parsed.blockedStateCategory).toBe('preview-render-blocked')
    expect(parsed.cameraConnection.state).toBe('connected')
    expect(parsed.previewRenderBoundary.status).toBe('blocked')
    expect(parsed.recentFailure?.title).toBe('프리뷰/렌더 결과 준비 지연')
    expect(parsed.liveCaptureTruth?.cameraState).toBe('ready')
    expect(parsed.liveCaptureTruth?.helperState).toBe('healthy')
  })

  it('rejects operator summaries whose blocked-state category or safe detail shape drift', () => {
    expect(() => operatorBlockedStateCategorySchema.parse('render-stuck')).toThrow()

    expect(() =>
      operatorSessionSummarySchema.parse(
        createOperatorSessionSummary({
          cameraConnection: {
            state: 'camera-ready',
            title: '카메라 준비 완료',
            detail: '잘못된 상태예요.',
          },
          recentFailure: {
            title: 'render stderr',
            detail: '',
            observedAt: '2026-03-26T00:10:01.000Z',
          },
        }),
      ),
    ).toThrow()
  })

  it('parses a recovery summary with category-specific allowed actions', () => {
    const parsedSummary = operatorRecoverySummarySchema.parse(
      createOperatorRecoverySummary(),
    )
    const parsedAction = operatorRecoveryActionSchema.parse('approved-boundary-restart')
    const parsedCategory =
      operatorRecoveryBlockedCategorySchema.parse('preview-or-render')

    expect(parsedSummary.blockedCategory).toBe('preview-or-render')
    expect(parsedSummary.allowedActions).toContain('route-phone-required')
    expect(parsedAction).toBe('approved-boundary-restart')
    expect(parsedCategory).toBe('preview-or-render')
  })

  it('rejects recovery summaries that expose actions without a blocked category', () => {
    expect(() =>
      operatorRecoverySummarySchema.parse(
        createOperatorRecoverySummary({
          blockedCategory: null,
          allowedActions: ['retry'],
        }),
      ),
    ).toThrow(/blockedCategory/i)
  })

  it('parses typed operator recovery action request and result payloads', () => {
    const parsedRequest = operatorRecoveryActionRequestSchema.parse({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      action: 'retry',
    })
    const parsedResult = operatorRecoveryActionResultSchema.parse(
      createOperatorRecoveryActionResult(),
    )

    expect(parsedRequest.action).toBe('retry')
    expect(parsedResult.status).toBe('applied')
    expect(parsedResult.nextState.customerState).toBe('Ready')
    expect(parsedResult.summary.allowedActions).toEqual([])
  })

  it('accepts a booth-safe published preset summary with a customer preview asset', () => {
    const parsed = publishedPresetSummarySchema.parse({
      presetId: 'preset_soft-glow',
      displayName: 'Soft Glow',
      publishedVersion: '2026.03.20',
      boothStatus: 'booth-safe',
      preview: {
        kind: 'preview-tile',
        assetPath: 'published/preset_soft-glow/2026.03.20/preview.jpg',
        altText: 'Soft Glow sample portrait',
      },
    })

    expect(parsed.preview.kind).toBe('preview-tile')
    expect(parsed.displayName).toBe('Soft Glow')
  })

  it('rejects preset summaries that do not expose a booth-safe preview asset', () => {
    expect(() =>
      publishedPresetSummarySchema.parse({
        presetId: 'preset_soft-glow',
        displayName: 'Soft Glow',
        publishedVersion: '2026.03.20',
        boothStatus: 'booth-safe',
      }),
    ).toThrow()
  })

  it('parses a preset catalog result with at most six published entries', () => {
    const preset = {
      presetId: 'preset_soft-glow',
      displayName: 'Soft Glow',
      publishedVersion: '2026.03.20',
      boothStatus: 'booth-safe',
      preview: {
        kind: 'preview-tile',
        assetPath: 'published/preset_soft-glow/2026.03.20/preview.jpg',
        altText: 'Soft Glow sample portrait',
      },
    }

    const parsed = presetCatalogResultSchema.parse({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      state: 'ready',
      presets: Array.from({ length: 6 }, () => preset),
    })

    expect(parsed.presets).toHaveLength(6)
  })

  it('parses a typed preset selection payload with stable preset identity and version', () => {
    const binding = activePresetBindingSchema.parse({
      presetId: 'preset_soft-glow',
      publishedVersion: '2026.03.20',
    })

    const parsed = presetSelectionInputSchema.parse({
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      preset: binding,
    })

    expect(parsed.preset.publishedVersion).toBe('2026.03.20')
  })

  it('parses a draft artifact candidate with validation metadata and booth-safe render profiles', () => {
    const parsed = draftPresetSummarySchema.parse(createDraftPresetSummary())

    expect(parsed.lifecycleState).toBe('draft')
    expect(parsed.xmpTemplatePath).toBe('xmp/soft-glow.xmp')
    expect(parsed.validation.status).toBe('not-run')
  })

  it('rejects lifecycle and validation combinations that cannot both be true', () => {
    expect(() =>
      draftPresetSummarySchema.parse(
        createDraftPresetSummary({
          lifecycleState: 'draft',
          validation: {
            status: 'passed',
            latestReport: createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
            history: [
              createDraftValidationReport({
                lifecycleState: 'validated',
                status: 'passed',
                findings: [],
              }),
            ],
          },
        }),
      ),
    ).toThrow(/draft 상태에서는 validation passed/i)

    expect(() =>
      draftPresetSummarySchema.parse(
        createDraftPresetSummary({
          lifecycleState: 'validated',
          validation: {
            status: 'failed',
            latestReport: createDraftValidationReport(),
            history: [createDraftValidationReport()],
          },
        }),
      ),
    ).toThrow(/approval-ready 이후 lifecycle은 validation passed/i)
  })

  it('keeps the lifecycle enum ready for validated, approved, and published follow-ups while limiting edits to draft payloads', () => {
    expect(presetLifecycleStateSchema.parse('published')).toBe('published')

    const parsedWorkspace = authoringWorkspaceResultSchema.parse({
      schemaVersion: 'preset-authoring-workspace/v1',
      supportedLifecycleStates: ['draft', 'validated', 'approved', 'published'],
      drafts: [],
      invalidDrafts: [
        {
          draftFolder: 'preset_broken-draft',
          message: '저장된 draft JSON 형식이 손상되어 작업공간에서 열 수 없어요.',
          guidance:
            '목록에서 손상 draft 정리를 실행한 뒤 새 draft를 만들고 메타데이터와 자산 참조를 다시 저장해 주세요.',
          canRepair: true,
        },
      ],
    })

    expect(parsedWorkspace.supportedLifecycleStates).toContain('approved')
    expect(parsedWorkspace.invalidDrafts).toHaveLength(1)
    expect(parsedWorkspace.invalidDrafts[0]?.canRepair).toBe(true)
    expect(
      repairInvalidDraftInputSchema.parse({
        draftFolder: 'preset_broken-draft',
      }),
    ).toEqual({
      draftFolder: 'preset_broken-draft',
    })

    expect(() =>
      draftPresetEditPayloadSchema.parse({
        presetId: 'preset_soft-glow-draft',
        displayName: 'Soft Glow Draft',
        lifecycleState: 'approved',
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
      }),
    ).toThrow()
  })

  it('parses validation transition input and output with actionable findings', () => {
    const parsedInput = validateDraftPresetInputSchema.parse({
      presetId: 'preset_soft-glow-draft',
    })
    const parsedResult = validateDraftPresetResultSchema.parse({
      schemaVersion: 'draft-preset-validation-result/v1',
      draft: createDraftPresetSummary({
        lifecycleState: 'validated',
        validation: {
          status: 'passed',
          latestReport: createDraftValidationReport({
            lifecycleState: 'validated',
            status: 'passed',
            findings: [],
          }),
          history: [
            createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
          ],
        },
      }),
      report: createDraftValidationReport({
        lifecycleState: 'validated',
        status: 'passed',
        findings: [],
      }),
    })

    expect(parsedInput.presetId).toBe('preset_soft-glow-draft')
    expect(parsedResult.draft.lifecycleState).toBe('validated')
    expect(parsedResult.report.status).toBe('passed')
  })

  it('rejects validation transition payloads whose draft and report truths diverge', () => {
    expect(() =>
      validateDraftPresetResultSchema.parse({
        schemaVersion: 'draft-preset-validation-result/v1',
        draft: createDraftPresetSummary({
          lifecycleState: 'published',
          validation: {
            status: 'passed',
            latestReport: createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
            history: [
              createDraftValidationReport({
                lifecycleState: 'validated',
                status: 'passed',
                findings: [],
              }),
            ],
          },
        }),
        report: createDraftValidationReport({
          lifecycleState: 'validated',
          status: 'passed',
          findings: [],
        }),
      }),
    ).toThrow(/draft 또는 validated lifecycle만 반환/i)

    expect(() =>
      validateDraftPresetResultSchema.parse({
        schemaVersion: 'draft-preset-validation-result/v1',
        draft: createDraftPresetSummary({
          lifecycleState: 'validated',
          validation: {
            status: 'passed',
            latestReport: createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
            history: [
              createDraftValidationReport({
                lifecycleState: 'validated',
                status: 'passed',
                findings: [],
              }),
            ],
          },
        }),
        report: createDraftValidationReport({
          presetId: 'preset_other-draft',
          lifecycleState: 'validated',
          status: 'passed',
          findings: [],
        }),
      }),
    ).toThrow(/report presetId는 draft presetId와 같아야/i)
  })

  it('parses publication input, success result, and audit history for future-session-only publication', () => {
    const parsedInput = publishValidatedPresetInputSchema.parse({
      presetId: 'preset_soft-glow-draft',
      draftVersion: 2,
      validationCheckedAt: '2026-03-26T00:10:00.000Z',
      expectedDisplayName: 'Soft Glow Draft',
      publishedVersion: '2026.03.26',
      actorId: 'manager-kim',
      actorLabel: 'Kim Manager',
      scope: 'future-sessions-only',
      reviewNote: '현재 진행 중인 부스 세션은 유지',
    })
    const parsedAudit = publicationAuditRecordSchema.parse(
      createPublicationAuditRecord(),
    )
    const parsedResult = publishValidatedPresetResultSchema.parse({
      schemaVersion: 'draft-preset-publication-result/v1',
      status: 'published',
      draft: createDraftPresetSummary({
        lifecycleState: 'published',
        validation: {
          status: 'passed',
          latestReport: createDraftValidationReport({
            lifecycleState: 'validated',
            status: 'passed',
            findings: [],
          }),
          history: [
            createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
          ],
        },
        publicationHistory: [
          createPublicationAuditRecord({
            action: 'approved',
            reviewNote: '현재 진행 중인 부스 세션은 유지',
            guidance: '승인 검토가 완료되었고 immutable 게시 아티팩트를 확정하고 있어요.',
          }),
          createPublicationAuditRecord(),
        ],
      }),
      publishedPreset: {
        presetId: 'preset_soft-glow-draft',
        displayName: 'Soft Glow Draft',
        publishedVersion: '2026.03.26',
        boothStatus: 'booth-safe',
        preview: {
          kind: 'preview-tile',
          assetPath: 'published/preset_soft-glow-draft/2026.03.26/preview.jpg',
          altText: 'Soft Glow draft portrait',
        },
      },
      bundlePath: 'C:/boothy/preset-catalog/published/preset_soft-glow-draft/2026.03.26',
      auditRecord: createPublicationAuditRecord(),
    })

    expect(parsedInput.scope).toBe('future-sessions-only')
    expect(parsedInput.reviewNote).toBe('현재 진행 중인 부스 세션은 유지')
    expect(parsedAudit.action).toBe('published')
    expect(parsedAudit.reviewNote).toBeNull()
    expect(parsedResult.status).toBe('published')
    expect(parsedResult.draft.publicationHistory).toHaveLength(2)
    expect(parsedResult.draft.publicationHistory[0].action).toBe('approved')
    expect(parsedResult.draft.publicationHistory[0].reviewNote).toBe(
      '현재 진행 중인 부스 세션은 유지',
    )
  })

  it('parses catalog state summaries and version history for authoring rollback management', () => {
    const parsedHistory = catalogVersionHistoryItemSchema.parse(
      createCatalogVersionHistoryItem(),
    )
    const parsedSummary = catalogStateSummarySchema.parse(
      createCatalogStateSummary(),
    )
    const parsedState = catalogStateResultSchema.parse({
      schemaVersion: 'preset-catalog-state-result/v1',
      catalogRevision: 4,
      presets: [createCatalogStateSummary()],
    })

    expect(parsedHistory.actionType).toBe('published')
    expect(parsedSummary.livePublishedVersion).toBe('2026.03.26')
    expect(parsedSummary.versionHistory[1]?.actionType).toBe('rollback')
    expect(parsedState.catalogRevision).toBe(4)
  })

  it('parses rollback input and completion results for future-session-safe catalog changes', () => {
    const parsedInput = rollbackPresetCatalogInputSchema.parse({
      presetId: 'preset_soft-glow-draft',
      targetPublishedVersion: '2026.03.25',
      expectedCatalogRevision: 4,
      actorId: 'manager-kim',
      actorLabel: 'Kim Manager',
    })
    const parsedSuccess = rollbackPresetCatalogResultSchema.parse({
      schemaVersion: 'preset-catalog-rollback-result/v1',
      status: 'rolled-back',
      catalogRevision: 5,
      summary: createCatalogStateSummary({
        livePublishedVersion: '2026.03.25',
      }),
      auditEntry: createCatalogVersionHistoryItem({
        actionType: 'rollback',
        fromPublishedVersion: '2026.03.26',
        toPublishedVersion: '2026.03.25',
      }),
      message:
        '선택한 승인 버전으로 되돌렸어요. 이미 진행 중인 세션은 기존 바인딩을 계속 유지해요.',
    })
    const parsedRejected = rollbackPresetCatalogResultSchema.parse({
      schemaVersion: 'preset-catalog-rollback-result/v1',
      status: 'rejected',
      reasonCode: 'already-live',
      message: '이미 현재 미래 세션 catalog에 노출 중인 버전이에요.',
      guidance: '다른 승인 버전을 선택하거나 현재 상태를 유지해 주세요.',
      catalogRevision: 4,
      summary: createCatalogStateSummary(),
    })

    expect(parsedInput.expectedCatalogRevision).toBe(4)
    expect(parsedSuccess.status).toBe('rolled-back')
    if (parsedSuccess.status !== 'rolled-back') {
      throw new Error('expected a rolled-back catalog result')
    }
    expect(parsedSuccess.summary.livePublishedVersion).toBe('2026.03.25')
    expect(parsedRejected.status).toBe('rejected')
  })

  it('rejects catalog summaries whose live version is missing from the published version list', () => {
    expect(() =>
      catalogStateSummarySchema.parse(
        createCatalogStateSummary({
          livePublishedVersion: '2026.03.24',
        }),
      ),
    ).toThrow(/livePublishedVersion/i)
  })

  it('requires publication history to record approved before published success', () => {
    expect(() =>
      publishValidatedPresetResultSchema.parse({
        schemaVersion: 'draft-preset-publication-result/v1',
        status: 'published',
        draft: createDraftPresetSummary({
          lifecycleState: 'published',
          validation: {
            status: 'passed',
            latestReport: createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
            history: [
              createDraftValidationReport({
                lifecycleState: 'validated',
                status: 'passed',
                findings: [],
              }),
            ],
          },
          publicationHistory: [createPublicationAuditRecord()],
        }),
        publishedPreset: {
          presetId: 'preset_soft-glow-draft',
          displayName: 'Soft Glow Draft',
          publishedVersion: '2026.03.26',
          boothStatus: 'booth-safe',
          preview: {
            kind: 'preview-tile',
            assetPath: 'published/preset_soft-glow-draft/2026.03.26/preview.jpg',
            altText: 'Soft Glow draft portrait',
          },
        },
        bundlePath: 'C:/boothy/preset-catalog/published/preset_soft-glow-draft/2026.03.26',
        auditRecord: createPublicationAuditRecord(),
      }),
    ).toThrow(/approved 이력을 먼저 남겨야 해요/i)
  })

  it('parses actionable publication rejection guidance without claiming a published artifact', () => {
    const parsed = publishValidatedPresetResultSchema.parse({
      schemaVersion: 'draft-preset-publication-result/v1',
      status: 'rejected',
      draft: createDraftPresetSummary({
        lifecycleState: 'validated',
        validation: {
          status: 'passed',
          latestReport: createDraftValidationReport({
            lifecycleState: 'validated',
            status: 'passed',
            findings: [],
          }),
          history: [
            createDraftValidationReport({
              lifecycleState: 'validated',
              status: 'passed',
              findings: [],
            }),
          ],
        },
        publicationHistory: [
          createPublicationAuditRecord({
            action: 'rejected',
            reasonCode: 'duplicate-version',
            guidance: '새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.',
          }),
        ],
      }),
      reasonCode: 'duplicate-version',
      message: '같은 published version이 이미 존재해서 immutable 게시 규칙을 지킬 수 없어요.',
      guidance: '새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.',
      auditRecord: createPublicationAuditRecord({
        action: 'rejected',
        reasonCode: 'duplicate-version',
        guidance: '새 publishedVersion을 사용하거나 기존 게시 버전을 유지해 주세요.',
      }),
    })

    expect(parsed.status).toBe('rejected')
    if (parsed.status !== 'rejected') {
      throw new Error('expected a rejected publication result')
    }
    expect(parsed.reasonCode).toBe('duplicate-version')
    expect(parsed.auditRecord.action).toBe('rejected')
  })

  it('rejects invalid validation finding payloads and passed reports with error findings', () => {
    expect(() =>
      draftValidationReportSchema.parse({
        ...createDraftValidationReport(),
        findings: [
          {
            ruleCode: 'Invalid Rule',
            severity: 'error',
            fieldPath: 'sampleCut.assetPath',
            message: 'sample-cut 대표 자산이 없어요.',
            guidance: 'sampleCut.assetPath를 다시 확인해 주세요.',
          },
        ],
      }),
    ).toThrow()

    expect(() =>
      draftValidationReportSchema.parse({
        ...createDraftValidationReport({
          status: 'passed',
        }),
      }),
    ).toThrow()
  })

  it('rejects preview or source references that try to escape the authoring workspace', () => {
    expect(() =>
      draftPresetEditPayloadSchema.parse({
        ...createDraftPresetSummary({
          validation: undefined,
          updatedAt: undefined,
        }),
        preview: {
          assetPath: '../outside/preview.jpg',
          altText: 'Soft Glow draft portrait',
        },
      }),
    ).toThrow()
  })

  it('parses customer-safe readiness snapshots and capture-saved request responses', () => {
    const capture = sessionCaptureRecordSchema.parse({
      schemaVersion: 'session-capture/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      activePresetId: 'preset_soft-glow',
      activePresetVersion: '2026.03.20',
      activePresetDisplayName: 'Soft Glow',
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
      raw: {
        assetPath: 'C:/boothy/sessions/session_01/captures/originals/capture.jpg',
        persistedAtMs: 100,
      },
      preview: {
        assetPath: null,
        enqueuedAtMs: 100,
        readyAtMs: null,
      },
      final: {
        assetPath: null,
        readyAtMs: null,
      },
      renderStatus: 'previewWaiting',
      postEndState: 'activeSession',
      timing: {
        captureAcknowledgedAtMs: 100,
        previewVisibleAtMs: null,
        captureBudgetMs: 1000,
        previewBudgetMs: 5000,
        previewBudgetState: 'pending',
      },
    })

    const readiness = captureReadinessSnapshotSchema.parse({
      schemaVersion: 'capture-readiness/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      surfaceState: 'previewWaiting',
      latestCapture: capture,
      liveCaptureTruth: {
        source: 'canon-helper-sidecar',
        freshness: 'fresh',
        sessionMatch: 'matched',
        cameraState: 'ready',
        helperState: 'healthy',
        observedAt: '2026-03-20T00:10:00.000Z',
        sequence: 7,
        detailCode: 'camera-ready',
      },
      customerState: 'Ready',
      canCapture: false,
      primaryAction: 'wait',
      customerMessage: '사진이 안전하게 저장되었어요.',
      supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
      reasonCode: 'preview-waiting',
      timing: {
        schemaVersion: 'session-timing/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        adjustedEndAt: '2026-03-20T00:15:00.000Z',
        warningAt: '2026-03-20T00:10:00.000Z',
        phase: 'active',
        captureAllowed: true,
        approvedExtensionMinutes: 0,
        approvedExtensionAuditRef: null,
        warningTriggeredAt: null,
        endedTriggeredAt: null,
      },
    })
    const captureResult = captureRequestResultSchema.parse({
      schemaVersion: 'capture-request-result/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      status: 'capture-saved',
      capture,
      readiness,
    })

    expect(captureResult.readiness.latestCapture?.captureId).toBe(capture.captureId)
    expect(captureResult.capture.activePresetDisplayName).toBe('Soft Glow')
    expect(captureResult.capture.raw.assetPath).toContain('captures/originals')
    expect(captureResult.status).toBe('capture-saved')
    expect(captureResult.readiness.timing?.adjustedEndAt).toBe(
      '2026-03-20T00:15:00.000Z',
    )
  })

  it('parses capture export result without marking the session completed', () => {
    const capture = sessionCaptureRecordSchema.parse({
      schemaVersion: 'session-capture/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      activePresetId: 'preset_soft-glow',
      activePresetVersion: '2026.03.20',
      activePresetDisplayName: 'Soft Glow',
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
      raw: {
        assetPath:
          'C:/boothy/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/captures/originals/capture.jpg',
        persistedAtMs: 100,
      },
      preview: {
        assetPath:
          'C:/boothy/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/capture.jpg',
        enqueuedAtMs: 100,
        readyAtMs: 180,
      },
      final: {
        assetPath:
          'C:/boothy/sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/finals/capture.jpg',
        readyAtMs: 320,
      },
      renderStatus: 'finalReady',
      postEndState: 'activeSession',
      timing: {
        captureAcknowledgedAtMs: 100,
        previewVisibleAtMs: 180,
        captureBudgetMs: 1000,
        previewBudgetMs: 5000,
        previewBudgetState: 'withinBudget',
      },
    })
    const readiness = captureReadinessSnapshotSchema.parse({
      schemaVersion: 'capture-readiness/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      surfaceState: 'captureReady',
      customerState: 'Ready',
      canCapture: true,
      primaryAction: 'capture',
      customerMessage: '지금 촬영할 수 있어요.',
      supportMessage: '방금 찍은 사진을 아래에서 바로 확인할 수 있어요.',
      reasonCode: 'ready',
      latestCapture: capture,
      postEnd: null,
    })
    const result = captureExportResultSchema.parse({
      schemaVersion: 'capture-export-result/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      exportedCount: 1,
      skippedCount: 0,
      manifest: {
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:10.000Z',
        lifecycle: {
          status: 'active',
          stage: 'capture-ready',
        },
        activePreset: {
          presetId: 'preset_soft-glow',
          publishedVersion: '2026.03.20',
        },
        activePresetId: 'preset_soft-glow',
        activePresetDisplayName: 'Soft Glow',
        captures: [capture],
        postEnd: null,
      },
      readiness,
    })

    expect(result.exportedCount).toBe(1)
    expect(result.manifest.lifecycle.stage).toBe('capture-ready')
    expect(result.manifest.postEnd).toBeNull()
    expect(result.manifest.captures[0]?.postEndState).toBe('activeSession')
    expect(result.readiness.reasonCode).toBe('ready')
  })

  it('parses pending same-capture fast preview timing without treating it as preview ready', () => {
    const capture = sessionCaptureRecordSchema.parse({
      schemaVersion: 'session-capture/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      activePresetId: 'preset_soft-glow',
      activePresetVersion: '2026.03.20',
      activePresetDisplayName: 'Soft Glow',
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
      raw: {
        assetPath: 'C:/boothy/sessions/session_01/captures/originals/capture.cr3',
        persistedAtMs: 100,
      },
      preview: {
        assetPath: 'C:/boothy/sessions/session_01/renders/previews/capture.jpg',
        enqueuedAtMs: 100,
        readyAtMs: null,
      },
      final: {
        assetPath: null,
        readyAtMs: null,
      },
      renderStatus: 'previewWaiting',
      postEndState: 'activeSession',
      timing: {
        captureAcknowledgedAtMs: 100,
        previewVisibleAtMs: null,
        fastPreviewVisibleAtMs: 180,
        xmpPreviewReadyAtMs: null,
        captureBudgetMs: 1000,
        previewBudgetMs: 5000,
        previewBudgetState: 'pending',
      },
    })

    expect(capture.renderStatus).toBe('previewWaiting')
    expect(capture.preview.readyAtMs).toBeNull()
    expect(capture.timing.fastPreviewVisibleAtMs).toBe(180)
    expect(capture.timing.xmpPreviewReadyAtMs).toBeNull()
  })

  it('accepts live capture truth timestamps that use an explicit UTC offset', () => {
    const readiness = captureReadinessSnapshotSchema.parse({
      schemaVersion: 'capture-readiness/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      surfaceState: 'captureReady',
      customerState: 'Ready',
      canCapture: true,
      primaryAction: 'capture',
      customerMessage: '지금 촬영할 수 있어요.',
      supportMessage: '버튼을 누르면 바로 시작돼요.',
      reasonCode: 'ready',
      latestCapture: null,
      liveCaptureTruth: {
        source: 'canon-helper-sidecar',
        freshness: 'fresh',
        sessionMatch: 'matched',
        cameraState: 'ready',
        helperState: 'healthy',
        observedAt: '2026-03-28T03:10:57.1234567+00:00',
        sequence: 12,
        detailCode: 'camera-ready',
      },
    })

    expect(readiness.liveCaptureTruth?.observedAt).toBe(
      '2026-03-28T03:10:57.1234567+00:00',
    )
    expect(readiness.liveCaptureTruth?.cameraState).toBe('ready')
  })

  it('accepts a customer-safe explicit post-end readiness without leaking internal policy language', () => {
    const readiness = captureReadinessSnapshotSchema.parse({
      schemaVersion: 'capture-readiness/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      surfaceState: 'blocked',
      customerState: 'Export Waiting',
      canCapture: false,
      primaryAction: 'wait',
      customerMessage: '안내를 준비하고 있어요.',
      supportMessage: '잠시만 기다리면 다음 안내를 보여드릴게요.',
      reasonCode: 'export-waiting',
      latestCapture: null,
      postEnd: {
        state: 'export-waiting',
        evaluatedAt: '2026-03-20T00:15:00.000Z',
      },
      timing: {
        schemaVersion: 'session-timing/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        adjustedEndAt: '2026-03-20T00:15:00.000Z',
        warningAt: '2026-03-20T00:10:00.000Z',
        phase: 'ended',
        captureAllowed: false,
        approvedExtensionMinutes: 0,
        approvedExtensionAuditRef: null,
        warningTriggeredAt: '2026-03-20T00:10:01.000Z',
        endedTriggeredAt: '2026-03-20T00:15:00.000Z',
      },
    })

    expect(readiness.reasonCode).toBe('export-waiting')
    expect(readiness.postEnd?.state).toBe('export-waiting')
    expect(readiness.supportMessage).not.toMatch(/policy|scheduler/i)
  })

  it('still parses legacy capture records that do not include activePresetId yet', () => {
    const capture = sessionCaptureRecordSchema.parse({
      schemaVersion: 'session-capture/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      activePresetVersion: '2026.03.20',
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
      raw: {
        assetPath: 'C:/boothy/sessions/session_01/captures/originals/capture.jpg',
        persistedAtMs: 100,
      },
      preview: {
        assetPath: null,
        enqueuedAtMs: 100,
        readyAtMs: null,
      },
      final: {
        assetPath: null,
        readyAtMs: null,
      },
      renderStatus: 'previewWaiting',
      postEndState: 'activeSession',
      timing: {
        captureAcknowledgedAtMs: 100,
        previewVisibleAtMs: null,
        captureBudgetMs: 1000,
        previewBudgetMs: 5000,
        previewBudgetState: 'pending',
      },
    })

    expect(capture.activePresetId).toBeUndefined()
    expect(capture.activePresetVersion).toBe('2026.03.20')
  })

  it('normalizes host kebab-case capture post-end states into the frontend contract', () => {
    const capture = sessionCaptureRecordSchema.parse({
      schemaVersion: 'session-capture/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      activePresetId: 'preset_soft-glow',
      activePresetVersion: '2026.03.20',
      activePresetDisplayName: 'Soft Glow',
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
      raw: {
        assetPath: 'C:/boothy/sessions/session_01/captures/originals/capture.jpg',
        persistedAtMs: 100,
      },
      preview: {
        assetPath: 'C:/boothy/sessions/session_01/renders/previews/capture.jpg',
        enqueuedAtMs: 100,
        readyAtMs: 500,
      },
      final: {
        assetPath: null,
        readyAtMs: null,
      },
      renderStatus: 'previewReady',
      postEndState: 'local-deliverable-ready',
      timing: {
        captureAcknowledgedAtMs: 100,
        previewVisibleAtMs: 500,
        captureBudgetMs: 1000,
        previewBudgetMs: 5000,
        previewBudgetState: 'withinBudget',
      },
    })

    expect(capture.postEndState).toBe('localDeliverableReady')
  })

  it('parses capture deletion results with the updated manifest and readiness', () => {
    const manifest = sessionManifestSchema.parse({
      schemaVersion: 'session-manifest/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      customer: {
        name: 'Kim',
        phoneLastFour: '4821',
      },
      createdAt: '2026-03-20T00:00:00.000Z',
      updatedAt: '2026-03-20T00:00:00.000Z',
      lifecycle: {
        status: 'active',
        stage: 'capture-ready',
      },
      activePreset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
      activePresetId: 'preset_soft-glow',
      captures: [],
      postEnd: null,
    })

    const result = captureDeleteResultSchema.parse({
      schemaVersion: 'capture-delete-result/v1',
      sessionId: manifest.sessionId,
      captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
      status: 'capture-deleted',
      manifest,
      readiness: {
        schemaVersion: 'capture-readiness/v1',
        sessionId: manifest.sessionId,
        surfaceState: 'captureReady',
        customerState: 'Ready',
        canCapture: true,
        primaryAction: 'capture',
        customerMessage: '지금 촬영할 수 있어요.',
        supportMessage: '버튼을 누르면 바로 시작돼요.',
        reasonCode: 'ready',
        latestCapture: null,
      },
    })

    expect(result.status).toBe('capture-deleted')
    expect(result.manifest.captures).toEqual([])
  })

  it('parses blocked capture errors with embedded customer-safe readiness guidance', () => {
    const error = hostErrorEnvelopeSchema.parse({
      code: 'capture-not-ready',
      message: '지금은 촬영할 수 없어요.',
      readiness: {
        customerState: 'Phone Required',
        canCapture: false,
        primaryAction: 'call-support',
        customerMessage: '지금은 도움이 필요해요.',
        supportMessage: '가까운 직원에게 알려 주세요.',
        reasonCode: 'phone-required',
      },
    })

    expect(error.readiness?.primaryAction).toBe('call-support')
  })

  it('parses handoff-ready post-end guidance with handoff metadata', () => {
    const manifest = sessionManifestSchema.parse({
      schemaVersion: 'session-manifest/v1',
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      boothAlias: 'Kim 4821',
      customer: {
        name: 'Kim',
        phoneLastFour: '4821',
      },
      createdAt: '2026-03-20T00:00:00.000Z',
      updatedAt: '2026-03-20T00:00:10.000Z',
      lifecycle: {
        status: 'active',
        stage: 'completed',
      },
      activePreset: null,
      captures: [],
      postEnd: {
        state: 'completed',
        evaluatedAt: '2026-03-20T00:00:10.000Z',
        completionVariant: 'handoff-ready',
        approvedRecipientLabel: 'Front Desk',
        primaryActionLabel: '안내된 직원에게 이름을 말씀해 주세요.',
        supportActionLabel: null,
        showBoothAlias: true,
      },
    })

    expect(manifest.postEnd?.state).toBe('completed')
    expect(
      manifest.postEnd?.state === 'completed'
        ? manifest.postEnd.completionVariant
        : null,
    ).toBe('handoff-ready')
    expect(
      manifest.postEnd?.state === 'completed'
        ? manifest.postEnd.approvedRecipientLabel
        : null,
    ).toBe('Front Desk')
  })

  it('rejects completed post-end guidance without a completion variant', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:10.000Z',
        lifecycle: {
          status: 'active',
          stage: 'completed',
        },
        activePreset: null,
        captures: [],
        postEnd: {
          state: 'completed',
          evaluatedAt: '2026-03-20T00:00:10.000Z',
        },
      }),
    ).toThrow(/completionVariant/i)
  })

  it('rejects handoff-ready post-end guidance without destination details', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 'session-manifest/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-03-20T00:00:00.000Z',
        updatedAt: '2026-03-20T00:00:10.000Z',
        lifecycle: {
          status: 'active',
          stage: 'completed',
        },
        activePreset: null,
        captures: [],
        postEnd: {
          state: 'completed',
          evaluatedAt: '2026-03-20T00:00:10.000Z',
          completionVariant: 'handoff-ready',
          approvedRecipientLabel: null,
          nextLocationLabel: null,
          primaryActionLabel: '마지막 안내를 확인해 주세요.',
          supportActionLabel: null,
          showBoothAlias: false,
        },
      }),
    ).toThrow(/승인된 수령 대상 또는 다음 이동 위치/i)
  })

  it('rejects capture request responses that omit the persisted capture record', () => {
    expect(() =>
      captureRequestResultSchema.parse({
        schemaVersion: 'capture-request-result/v1',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        status: 'capture-saved',
        readiness: {
          schemaVersion: 'capture-readiness/v1',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
          surfaceState: 'captureSaved',
          customerState: 'Preview Waiting',
          canCapture: false,
          primaryAction: 'wait',
          customerMessage: '사진이 안전하게 저장되었어요.',
          supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
          reasonCode: 'preview-waiting',
          latestCapture: null,
        },
      }),
    ).toThrow()
  })
})
