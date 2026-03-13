import { describe, expect, it } from 'vitest'

import { sessionManifestSchema } from '../../src/shared-contracts/schemas/manifestSchemas.js'

describe('session manifest schema', () => {
  it('accepts the versioned manifest shape used on disk', () => {
    expect(
      sessionManifestSchema.parse({
        schemaVersion: 1,
        sessionId: '2026-03-08:김보라 오후 세션',
        sessionName: '김보라 오후 세션',
        operationalDate: '2026-03-08',
        createdAt: '2026-03-08T00:00:00.000Z',
        sessionDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션',
        manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/session.json',
        eventsPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/events.ndjson',
        exportStatusPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/export-status.json',
        processedDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/processed',
        captureRevision: 0,
        latestCaptureId: null,
        activePresetName: null,
        activePreset: null,
        captures: [],
        timing: {
          reservationStartAt: '2026-03-08T00:00:00.000Z',
          actualShootEndAt: '2026-03-08T00:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T00:00:00.000Z',
        },
        cameraState: {
          connectionState: 'offline',
        },
        exportState: {
          status: 'notStarted',
        },
      }),
    ).toMatchObject({
      sessionName: '김보라 오후 세션',
    })
  })

  it('accepts optional gallery metadata used for session-scoped review and deletion', () => {
    const parsed = sessionManifestSchema.parse({
      schemaVersion: 1,
      sessionId: '2026-03-08:김보라 오후 세션',
      sessionName: '김보라 오후 세션',
      operationalDate: '2026-03-08',
      createdAt: '2026-03-08T00:00:00.000Z',
      sessionDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션',
      manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/session.json',
      eventsPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/events.ndjson',
      exportStatusPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/export-status.json',
      processedDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/processed',
      captureRevision: 4,
      latestCaptureId: 'capture-002',
      activePresetName: '배경지 - 핑크',
      activePreset: {
        presetId: 'background-pink',
        displayName: '배경지 - 핑크',
      },
      timing: {
        reservationStartAt: '2026-03-08T00:00:00.000Z',
        actualShootEndAt: '2026-03-08T00:50:00.000Z',
        sessionType: 'standard',
        operatorExtensionCount: 0,
        lastTimingUpdateAt: '2026-03-08T00:00:00.000Z',
      },
      captures: [
        {
          captureId: 'capture-001',
          originalFileName: 'capture-001.nef',
          processedFileName: 'capture-001.jpg',
          capturedAt: '2026-03-08T00:00:10.000Z',
        },
        {
          captureId: 'capture-002',
          originalFileName: 'capture-002.nef',
          processedFileName: 'capture-002.jpg',
          capturedAt: '2026-03-08T00:00:20.000Z',
        },
      ],
      cameraState: {
        connectionState: 'offline',
      },
      exportState: {
        status: 'notStarted',
      },
    })

    expect(parsed).toMatchObject({
      latestCaptureId: 'capture-002',
      captureRevision: 4,
      activePresetName: '배경지 - 핑크',
      activePreset: {
        presetId: 'background-pink',
        displayName: '배경지 - 핑크',
      },
      timing: {
        actualShootEndAt: '2026-03-08T00:50:00.000Z',
        sessionType: 'standard',
      },
      captures: [
        expect.objectContaining({
          captureId: 'capture-001',
        }),
        expect.objectContaining({
          captureId: 'capture-002',
        }),
      ],
    })
  })

  it('rejects malformed manifest fields that would violate the host contract', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 1,
        sessionId: '2026-03-08:김보라 오후 세션',
        sessionName: '',
        operationalDate: '20260308',
        createdAt: '2026-03-08T00:00:00.000Z',
        sessionDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션',
        manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/session.json',
        eventsPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/events.ndjson',
        exportStatusPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/export-status.json',
        processedDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/processed',
        cameraState: {
          connectionState: 'offline',
        },
        exportState: {
          status: 'notStarted',
        },
      }),
    ).toThrow()
  })

  it('requires each capture to declare both original and processed asset references for safe deletion', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 1,
        sessionId: '2026-03-08:김보라 오후 세션',
        sessionName: '김보라 오후 세션',
        operationalDate: '2026-03-08',
        createdAt: '2026-03-08T00:00:00.000Z',
        sessionDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션',
        manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/session.json',
        eventsPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/events.ndjson',
        exportStatusPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/export-status.json',
        processedDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/processed',
        latestCaptureId: 'capture-001',
        activePresetName: null,
        activePreset: null,
        captures: [
          {
            captureId: 'capture-001',
            processedFileName: 'capture-001.jpg',
            capturedAt: '2026-03-08T00:00:10.000Z',
          },
        ],
        timing: {
          reservationStartAt: '2026-03-08T00:00:00.000Z',
          actualShootEndAt: '2026-03-08T00:50:00.000Z',
          sessionType: 'standard',
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-08T00:00:00.000Z',
        },
        cameraState: {
          connectionState: 'offline',
        },
        exportState: {
          status: 'notStarted',
        },
      }),
    ).toThrow()
  })

  it('requires authoritative timing state instead of allowing timing-less manifests', () => {
    expect(() =>
      sessionManifestSchema.parse({
        schemaVersion: 1,
        sessionId: '2026-03-08:김보라 오후 세션',
        sessionName: '김보라 오후 세션',
        operationalDate: '2026-03-08',
        createdAt: '2026-03-08T00:00:00.000Z',
        sessionDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션',
        manifestPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/session.json',
        eventsPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/events.ndjson',
        exportStatusPath: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/export-status.json',
        processedDir: 'C:/Boothy/Sessions/2026-03-08/김보라 오후 세션/processed',
        latestCaptureId: null,
        activePresetName: null,
        activePreset: null,
        captures: [],
        cameraState: {
          connectionState: 'offline',
        },
        exportState: {
          status: 'notStarted',
        },
      }),
    ).toThrow()
  })
})
