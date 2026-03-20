import { describe, expect, it } from 'vitest'

import {
  boothSessionStubSchema,
  capabilitySnapshotSchema,
  hostErrorEnvelopeSchema,
  sessionManifestSchema,
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
      activePresetId: null,
      captures: [],
      postEnd: null,
    })

    expect(parsed.boothAlias).toBe('Kim 4821')
    expect(parsed.captures).toEqual([])
    expect(parsed.lifecycle.stage).toBe('session-started')
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
        activePresetId: null,
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
})
