import { describe, expect, it } from 'vitest'

import { FIXED_THUMBNAIL_MAX_EDGE_PX } from '../../viewer-surface/services/display-fit'
import type { DisplayGeneration } from '../../shared-contracts'
import { shouldAdvanceDisplay } from './display-guard'

const SESSION_A = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
const SESSION_B = 'session_01hs6n1r8b8zc5v4ey2x7b9g2n'

function buildGeneration(
  overrides: Partial<DisplayGeneration> = {},
): DisplayGeneration {
  return {
    generationId: 'req-1-000001',
    generationSeq: 1,
    sessionId: SESSION_A,
    requestId: 'req-1',
    captureId: 'capture-1',
    viewerEpoch: 3,
    tier: 'sample',
    assetPath: 'C:/dabi_shoot/sessions/a/renders/display/req-1/000001-a.jpg',
    sourceWidthPx: 3840,
    sourceHeightPx: 2560,
    byteSize: 700_000,
    sourceHash: 'fnv1a64:0000000000000001',
    sampleVariant: 'a',
    committedAtHostMicros: 1_000_000,
    ...overrides,
  }
}

const context = {
  sessionId: SESSION_A,
  viewerEpoch: 3,
  requiredSourceWidthPx: 1620,
  requiredSourceHeightPx: 1080,
}

describe('shouldAdvanceDisplay', () => {
  it('accepts the first qualifying generation', () => {
    expect(shouldAdvanceDisplay(null, buildGeneration(), context)).toEqual({
      advance: true,
    })
  })

  it('accepts a strictly newer generation of the same request', () => {
    const current = buildGeneration()
    const next = buildGeneration({
      generationId: 'req-1-000002',
      generationSeq: 2,
      sampleVariant: 'b',
    })

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: true,
    })
  })

  it('rejects a lower generation sequence', () => {
    const current = buildGeneration({ generationSeq: 5 })
    const next = buildGeneration({ generationSeq: 4 })

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: false,
      reason: 'lower-generation',
    })
  })

  it('rejects a duplicate of the current generation', () => {
    const current = buildGeneration({ generationSeq: 5 })
    const next = buildGeneration({ generationSeq: 5 })

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: false,
      reason: 'lower-generation',
    })
  })

  it('rejects another session', () => {
    const next = buildGeneration({ sessionId: SESSION_B })

    expect(shouldAdvanceDisplay(null, next, context)).toEqual({
      advance: false,
      reason: 'session-mismatch',
    })
  })

  it('rejects a stale viewer epoch', () => {
    const next = buildGeneration({ viewerEpoch: 2 })

    expect(shouldAdvanceDisplay(null, next, context)).toEqual({
      advance: false,
      reason: 'stale-epoch',
    })
  })

  it('rejects an older request arriving late with a higher sequence', () => {
    const current = buildGeneration({
      requestId: 'req-2',
      generationSeq: 5,
      committedAtHostMicros: 5_000_000,
    })
    const late = buildGeneration({
      requestId: 'req-1',
      generationSeq: 6,
      committedAtHostMicros: 2_000_000,
    })

    expect(shouldAdvanceDisplay(current, late, context)).toEqual({
      advance: false,
      reason: 'older-request',
    })
  })

  it('rejects a source that cannot fill the photo rectangle without upscaling', () => {
    const next = buildGeneration({
      sourceWidthPx: 1619,
      sourceHeightPx: 1080,
    })

    expect(shouldAdvanceDisplay(null, next, context)).toEqual({
      advance: false,
      reason: 'insufficient-dimensions',
    })
  })

  it('rejects the fixed 384px thumbnail on every approved profile', () => {
    const thumbnail = buildGeneration({
      sourceWidthPx: FIXED_THUMBNAIL_MAX_EDGE_PX,
      sourceHeightPx: FIXED_THUMBNAIL_MAX_EDGE_PX,
    })

    for (const required of [
      { requiredSourceWidthPx: 1620, requiredSourceHeightPx: 1080 },
      { requiredSourceWidthPx: 2160, requiredSourceHeightPx: 1440 },
      { requiredSourceWidthPx: 3240, requiredSourceHeightPx: 2160 },
    ]) {
      expect(
        shouldAdvanceDisplay(null, thumbnail, { ...context, ...required }),
      ).toEqual({ advance: false, reason: 'insufficient-dimensions' })
    }
  })

  it('rejects any generation while the viewer has not measured its photo rectangle', () => {
    expect(
      shouldAdvanceDisplay(null, buildGeneration(), {
        ...context,
        requiredSourceWidthPx: 0,
        requiredSourceHeightPx: 0,
      }),
    ).toEqual({ advance: false, reason: 'viewer-not-ready' })
  })

  it('rejects a poisoned generation that already failed to decode', () => {
    expect(
      shouldAdvanceDisplay(null, buildGeneration(), context, {
        poisonedGenerationIds: new Set(['req-1-000001']),
      }),
    ).toEqual({ advance: false, reason: 'decode-failed' })
  })

  it('rejects an unknown tier rather than guessing its order', () => {
    const next = buildGeneration({
      tier: 'rawRefinedDisplay' as DisplayGeneration['tier'],
    })

    expect(shouldAdvanceDisplay(null, next, context)).toEqual({
      advance: false,
      reason: 'unknown-generation',
    })
  })
})
