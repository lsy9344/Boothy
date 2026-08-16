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
    requestOrder: 0,
    captureOrder: 0,
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
    proxyProvenance: null,
    committedAtHostMicros: 1_000_000,
    ...overrides,
  }
}

/** Story 7.4. display-fit preset proxy generation 한 건. */
function buildProxyGeneration(
  overrides: Partial<DisplayGeneration> = {},
  provenanceOverrides: Partial<
    NonNullable<DisplayGeneration['proxyProvenance']>
  > = {},
): DisplayGeneration {
  return buildGeneration({
    generationId: 'req-1-000002',
    generationSeq: 2,
    tier: 'displayFitPresetProxy',
    assetPath:
      'C:/dabi_shoot/sessions/a/renders/display/req-1/000002-proxy.jpg',
    sampleVariant: null,
    proxyProvenance: {
      presetId: 'preset_soft-glow',
      presetVersion: '2026.08.01',
      approvalBasis: 'exact-reference-renderer',
      proxyRecipeVersion: '1',
      referenceRenderer: 'darktable',
      referenceRendererVersion: '5.4.1',
      renderProfileId: 'preset_soft_glow-preview',
      outputColorSpace: 'sRGB',
      jpegQuality: 92,
      sourceRoute: 'raw-original',
      sourceAssetHash: 'fnv1a64:00000000000000aa',
      targetWidthPx: 1620,
      targetHeightPx: 1080,
      displayProfileId: 'approved-1080p',
      devicePixelRatio: 1,
      // Story 7.5. 참조 렌더러가 직접 만든 프레임에는 producer가 따로 없다.
      residentProvenance: null,
      // Story 7.6. proxy lane은 `--hq false`다.
      renderQuality: 'fast',
      ...provenanceOverrides,
    },
    ...overrides,
  })
}

/**
 * Story 7.6. 같은 촬영·같은 XMP·같은 크기에서 `--hq true`로 다시 렌더한 정밀본.
 *
 * 기본값은 **활성 proxy와 픽셀 크기가 정확히 같다.** 이 동일성이 AC 4의 crop/scale 점프 0을
 * 만드는 기계적 장치이므로, 테스트가 어긋난 크기를 쓰려면 명시적으로 덮어써야 한다.
 */
function buildRefinedGeneration(
  overrides: Partial<DisplayGeneration> = {},
  provenanceOverrides: Partial<
    NonNullable<DisplayGeneration['proxyProvenance']>
  > = {},
): DisplayGeneration {
  return buildProxyGeneration(
    {
      generationId: 'req-1-000003',
      generationSeq: 3,
      tier: 'rawRefinedDisplay',
      assetPath:
        'C:/dabi_shoot/sessions/a/renders/display/req-1/000003-refined.jpg',
      ...overrides,
    },
    { renderQuality: 'high', ...provenanceOverrides },
  )
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
      requestOrder: 2,
      generationSeq: 5,
      committedAtHostMicros: 5_000_000,
    })
    const late = buildGeneration({
      requestId: 'req-1',
      requestOrder: 1,
      generationSeq: 6,
      committedAtHostMicros: 6_000_000,
    })

    expect(shouldAdvanceDisplay(current, late, context)).toEqual({
      advance: false,
      reason: 'older-request',
    })
  })

  it('accepts a portrait source that reaches the contain boundary without upscaling', () => {
    const next = buildGeneration({
      sourceWidthPx: 720,
      sourceHeightPx: 1080,
    })

    expect(shouldAdvanceDisplay(null, next, context)).toEqual({
      advance: true,
    })
  })

  it('rejects a source when both axes are below the contain boundary', () => {
    const next = buildGeneration({
      sourceWidthPx: 1619,
      sourceHeightPx: 1079,
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
    // `final`은 등록된 tier가 아니고 앞으로도 아니다 (Story 7.6 「핵심 설계 결정 2」).
    const next = buildGeneration({
      tier: 'final' as DisplayGeneration['tier'],
    })

    expect(shouldAdvanceDisplay(null, next, context)).toEqual({
      advance: false,
      reason: 'unknown-generation',
    })
  })

  // ---------------------------------------------------------------------------
  // Story 7.6: RAW 정밀본 승급.
  // host의 `evaluate_admission`과 **같은 거부 매트릭스**를 갖는다.
  // 한쪽에만 두면 pointer와 화면이 갈라진다.
  // ---------------------------------------------------------------------------

  it('lets a raw refined generation replace the proxy of the same capture', () => {
    expect(
      shouldAdvanceDisplay(
        buildProxyGeneration(),
        buildRefinedGeneration(),
        context,
      ),
    ).toEqual({ advance: true })
  })

  it('refuses a proxy that would walk the same capture back from its refined frame', () => {
    expect(
      shouldAdvanceDisplay(
        buildRefinedGeneration(),
        buildProxyGeneration({ generationId: 'req-1-000004', generationSeq: 4 }),
        context,
      ),
    ).toEqual({ advance: false, reason: 'tier-downgrade' })
  })

  it('refuses a refined frame whose pixel size differs from the active proxy', () => {
    // 한 픽셀만 달라도 `object-fit: contain` 박스가 달라져 교체 순간 사진이 튄다.
    expect(
      shouldAdvanceDisplay(
        buildProxyGeneration(),
        buildRefinedGeneration({ sourceWidthPx: 3841 }),
        context,
      ),
    ).toEqual({ advance: false, reason: 'refined-dimension-mismatch' })
    expect(
      shouldAdvanceDisplay(
        buildProxyGeneration(),
        buildRefinedGeneration({ sourceHeightPx: 2559 }),
        context,
      ),
    ).toEqual({ advance: false, reason: 'refined-dimension-mismatch' })
  })

  it('refuses a refined frame when no proxy is on screen yet', () => {
    // UX-DR19. 정밀본은 승급이지 첫 성공 화면의 대체가 아니다.
    expect(shouldAdvanceDisplay(null, buildRefinedGeneration(), context)).toEqual(
      { advance: false, reason: 'refined-dimension-mismatch' },
    )
  })

  it('refuses a refined frame that has no proxy of its own capture on screen', () => {
    const otherCaptureProxy = buildProxyGeneration({
      generationId: 'req-2-000001',
      generationSeq: 4,
      requestId: 'req-2',
      requestOrder: 1,
      captureId: 'capture-2',
      captureOrder: 1,
    })
    const refined = buildRefinedGeneration({
      generationId: 'req-3-000001',
      generationSeq: 5,
      requestId: 'req-3',
      requestOrder: 2,
      captureId: 'capture-3',
      captureOrder: 2,
    })

    expect(shouldAdvanceDisplay(otherCaptureProxy, refined, context)).toEqual({
      advance: false,
      reason: 'refined-dimension-mismatch',
    })
  })

  /**
   * **이 판정이 없으면 고객의 다음 사진이 통째로 사라진다.**
   *
   * 촬영 A가 정밀본(tier 2)까지 올라간 뒤 촬영 B의 proxy(tier 1)가 도착한다.
   * tier만 비교하면 `tier-downgrade`로 거부되고, 화면에는 이전 사진이 그대로 남는다.
   */
  it('lets the next photo start at the proxy tier after the previous photo was refined', () => {
    const previousRefined = buildRefinedGeneration()
    const nextProxy = buildProxyGeneration({
      generationId: 'req-2-000001',
      generationSeq: 4,
      requestId: 'req-2',
      requestOrder: 1,
      captureId: 'capture-2',
      captureOrder: 1,
    })

    expect(shouldAdvanceDisplay(previousRefined, nextProxy, context)).toEqual({
      advance: true,
    })
  })

  it('still refuses an older capture arriving after a newer refined frame', () => {
    const currentRefined = buildRefinedGeneration({
      requestId: 'req-2',
      requestOrder: 1,
      captureId: 'capture-2',
      captureOrder: 1,
    })
    const olderProxy = buildProxyGeneration({
      generationId: 'req-1-000009',
      generationSeq: 9,
      captureOrder: 0,
    })

    expect(shouldAdvanceDisplay(currentRefined, olderProxy, context)).toEqual({
      advance: false,
      reason: 'older-capture',
    })
  })

  it('refuses a late refined frame rendered from a superseded preset', () => {
    const current = buildProxyGeneration({}, { presetVersion: '2026.08.10' })
    const stale = buildRefinedGeneration({}, { presetVersion: '2026.08.01' })

    expect(shouldAdvanceDisplay(current, stale, context)).toEqual({
      advance: false,
      reason: 'preset-mismatch',
    })
  })

  it('lets a display-fit preset proxy replace a measurement sample', () => {
    const current = buildGeneration()
    const next = buildProxyGeneration()

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: true,
    })
  })

  it('never lets a measurement sample replace a preset proxy', () => {
    const current = buildProxyGeneration()
    const next = buildGeneration({
      generationId: 'req-1-000003',
      generationSeq: 3,
    })

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: false,
      reason: 'tier-downgrade',
    })
  })

  it('refuses to swap the look of one photo for another preset version', () => {
    // 세션 중 preset을 바꾼 뒤(Story 2.3) catalog rollback 재렌더가 도달할 수 있는 경로다.
    const current = buildProxyGeneration()
    const next = buildProxyGeneration(
      { generationId: 'req-1-000003', generationSeq: 3 },
      { presetVersion: '2026.07.01' },
    )

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: false,
      reason: 'preset-mismatch',
    })
  })

  it('refuses a different preset identity for the same capture', () => {
    const current = buildProxyGeneration()
    const next = buildProxyGeneration(
      { generationId: 'req-1-000003', generationSeq: 3 },
      { presetId: 'preset_mono-pop' },
    )

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: false,
      reason: 'preset-mismatch',
    })
  })

  it('rejects an earlier capture whose long render finished last', () => {
    // proxy 렌더는 수 초가 걸린다. 먼저 찍은 사진이 나중에 끝나면 seq도 request order도
    // 더 크게 부여되므로, 촬영 시각 좌표 없이는 이 경우를 막을 수 없다.
    const current = buildProxyGeneration({
      requestId: 'req-2',
      captureId: 'capture-2',
      captureOrder: 2,
      generationId: 'req-2-000003',
      generationSeq: 3,
      committedAtHostMicros: 3_000_000,
    })
    const next = buildProxyGeneration({
      generationId: 'req-1-000004',
      captureOrder: 1,
      generationSeq: 4,
      committedAtHostMicros: 4_000_000,
    })

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: false,
      reason: 'older-capture',
    })
  })

  it('accepts the next capture even when it shares the preset', () => {
    const current = buildProxyGeneration()
    const next = buildProxyGeneration({
      requestId: 'req-2',
      captureId: 'capture-2',
      captureOrder: 1,
      generationId: 'req-2-000003',
      generationSeq: 3,
      committedAtHostMicros: 3_000_000,
    })

    expect(shouldAdvanceDisplay(current, next, context)).toEqual({
      advance: true,
    })
  })
})
