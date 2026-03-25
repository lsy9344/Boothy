import { describe, expect, it } from 'vitest'

import {
  captureDeleteResultSchema,
  activePresetBindingSchema,
  boothSessionStubSchema,
  captureReadinessSnapshotSchema,
  captureRequestResultSchema,
  capabilitySnapshotSchema,
  hostErrorEnvelopeSchema,
  presetCatalogResultSchema,
  presetSelectionInputSchema,
  publishedPresetSummarySchema,
  sessionManifestSchema,
  sessionCaptureRecordSchema,
  sessionStartInputSchema,
  sessionStartResultSchema,
} from './index'

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
      activePreset: null,
      captures: [],
      postEnd: null,
    })

    expect(parsed.boothAlias).toBe('Kim 4821')
    expect(parsed.captures).toEqual([])
    expect(parsed.lifecycle.stage).toBe('session-started')
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
      activePreset: {
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.20',
      },
      activePresetDisplayName: 'Soft Glow',
      captures: [],
      postEnd: null,
    })

    expect(parsed.lifecycle.stage).toBe('capture-ready')
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
    expect(error.fieldErrors?.phoneLastFour).toBe('숫자 4자리여야 해요.')
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
      customerState: 'Ready',
      canCapture: false,
      primaryAction: 'wait',
      customerMessage: '사진이 안전하게 저장되었어요.',
      supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
      reasonCode: 'preview-waiting',
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
