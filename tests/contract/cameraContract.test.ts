import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  cameraCommandRequestSchema,
  captureCommandRequestSchema,
  generateSidecarProtocolJsonSchema,
  sidecarCaptureRequestSchema,
  sidecarMessageSchema,
} from '../../src/shared-contracts/dto/cameraContract.js'
import { schemaVersions } from '../../src/shared-contracts/dto/schemaVersion.js'

function readJson<T>(relativePath: string): T {
  return JSON.parse(readFileSync(resolve(process.cwd(), relativePath), 'utf8')) as T
}

describe('camera contract baseline', () => {
  it('parses the approved readiness request and keeps the checked-in JSON schema in sync', () => {
    const request = {
      schemaVersion: schemaVersions.protocol,
      requestId: 'req-ready-001',
      correlationId: 'corr-session-001',
      method: 'camera.checkReadiness',
      sessionId: 'session-001',
      payload: {
        desiredCameraId: 'canon-eos-r100',
        mockScenario: 'readinessSuccess',
      },
    }

    expect(cameraCommandRequestSchema.parse(request)).toEqual(request)
    expect(generateSidecarProtocolJsonSchema()).toEqual(readJson('sidecar/protocol/messages.schema.json'))
  })

  it('parses a session-scoped capture request with active preset context', () => {
    const request = {
      schemaVersion: schemaVersions.protocol,
      requestId: 'req-capture-001',
      correlationId: 'corr-session-001',
      method: 'camera.capture',
      sessionId: 'session-001',
      payload: {
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
      },
    }

    expect(captureCommandRequestSchema.parse(request)).toEqual(request)
  })

  it('validates the helper-facing capture request shape used by the sidecar boundary', () => {
    const request = {
      schemaVersion: schemaVersions.protocol,
      requestId: 'req-capture-001',
      correlationId: 'corr-session-001',
      method: 'camera.capture',
      sessionId: 'session-001',
      payload: {
        activePreset: {
          presetId: 'background-pink',
          label: '배경지 - 핑크',
        },
        captureId: 'capture-001',
        originalFileName: 'originals/capture-001.nef',
        processedFileName: 'capture-001.png',
        originalOutputPath: 'C:/Boothy/Sessions/2026-03-08/홍길동1234/originals/capture-001.nef',
        processedOutputPath: 'C:/Boothy/Sessions/2026-03-08/홍길동1234/processed/capture-001.png',
      },
    }

    expect(sidecarCaptureRequestSchema.parse(request)).toEqual(request)
  })

  it('validates every golden protocol fixture against the shared sidecar message union', () => {
    const fixtureDir = resolve(process.cwd(), 'sidecar/protocol/examples')
    const fixtures = readdirSync(fixtureDir).filter((entry) => entry.endsWith('.json')).sort()

    expect(fixtures).toEqual([
      'capture-completed.json',
      'capture-request.json',
      'capture-started.json',
      'capture-success.json',
      'export-progress-placeholder.json',
      'normalized-error.json',
      'readiness-degraded.json',
      'readiness-success.json',
    ])

    for (const fixture of fixtures) {
      const payload = readJson<unknown>(`sidecar/protocol/examples/${fixture}`)
      expect(sidecarMessageSchema.parse(payload)).toEqual(payload)
    }
  })
})
