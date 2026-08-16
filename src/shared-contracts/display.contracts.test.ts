import { describe, expect, it } from 'vitest'

// Rust DTO를 원문 그대로 읽어 TS 계약과 대조한다.
// 두 정의가 어긋나면 경계에서 조용히 undefined가 되므로 문자열 수준으로 고정한다.
import rustDto from '../../src-tauri/src/contracts/dto.rs?raw'

import {
  DISPLAY_TIER_ORDER,
  clockProbeResultSchema,
  clockProbeSchema,
  compareDisplayTier,
  displayGenerationCompatSchema,
  displayGenerationSchema,
  displayHostRejectReasonSchema,
  displayPointerSnapshotSchema,
  displayPresentReportSchema,
  displayRejectReasonSchema,
  displayPointerSnapshotCompatSchema,
  displayTierSchema,
  displayUpdateSchema,
  parseResidentRendererMode,
  residentInputProvenanceSchema,
  residentProducerProvenanceSchema,
  trustedInputReportSchema,
  RESIDENT_APPROVED_DIRECT_DECODERS,
  viewerDisplaySchemaVersion,
  viewerDisplayUpdateEvent,
  viewerDisplayUpdateSchemaVersion,
} from '.'

const SESSION_ID = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

function buildGeneration() {
  return {
    generationId: 'req-1-000001',
    generationSeq: 1,
    sessionId: SESSION_ID,
    requestId: 'req-1',
    captureId: 'capture-1',
    viewerEpoch: 3,
    tier: 'sample' as const,
    assetPath: 'C:/dabi_shoot/sessions/a/renders/display/req-1/000001-a.jpg',
    sourceWidthPx: 3840,
    sourceHeightPx: 2560,
    byteSize: 701_308,
    sourceHash: 'fnv1a64:0123456789abcdef',
    sampleVariant: 'a' as const,
    proxyProvenance: null,
    committedAtHostMicros: 1_500_000,
  }
}

/** Story 7.4. host가 만드는 display-fit preset proxy generation의 실제 모양. */
function buildProxyGeneration() {
  return {
    ...buildGeneration(),
    generationId: 'req-1-000002',
    generationSeq: 2,
    tier: 'displayFitPresetProxy' as const,
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
      // 참조 렌더러가 직접 만든 프레임이다. producer가 따로 없다.
      residentProvenance: null,
      // Story 7.6. proxy lane은 `--hq false`다.
      renderQuality: 'fast' as const,
    },
  }
}

/**
 * Story 7.6. 같은 촬영·같은 XMP·같은 크기에서 `--hq true`로 다시 렌더한 정밀본.
 *
 * **`renderQuality` 말고는 proxy와 구분되는 필드가 없다.** 그래서 이 필드가 계약에 있다.
 */
function buildRefinedGeneration() {
  const proxy = buildProxyGeneration()

  return {
    ...proxy,
    generationId: 'req-1-000003',
    generationSeq: 3,
    tier: 'rawRefinedDisplay' as const,
    assetPath:
      'C:/dabi_shoot/sessions/a/renders/display/req-1/000003-refined.jpg',
    proxyProvenance: {
      ...proxy.proxyProvenance,
      renderQuality: 'high' as const,
    },
  }
}

/** Story 7.5. 상주 후보가 만든 generation의 실제 모양. */
function buildResidentProvenance() {
  return {
    producerRenderer: 'webgl2-resident',
    producerRendererVersion: '0.1.0-spike',
    producerBuildId: 'spike-build-1',
    executionMode: 'evidence' as const,
    recipeSchemaVersion: 'resident-recipe/v1',
    compiledRecipeHash: 'fnv1a64:0123456789abcdef',
    programHash: 'fnv1a64:fedcba9876543210',
    contextInitializedAtMicros: 900_000,
    hotPathProgramCompileCount: 0,
    hotPathProcessStartCount: 0,
    inputProvenance: 'predecoded-fixture' as const,
    inputProducer: 'darktable-cli',
    inputProducerVersion: '5.4.1',
    sourceReadyAtMicros: 1_200_000,
    inputStartupCostMicros: 0,
    productionEligible: false,
    adoptionBlockReason: 'resident-source-route-unapproved' as string | null,
    gpuVendor: 'unknown',
    gpuRenderer: 'unknown',
    fallbackReason: null as string | null,
  }
}

function buildPointer() {
  return {
    schemaVersion: viewerDisplaySchemaVersion,
    sessionId: SESSION_ID,
    revision: 4,
    activeGeneration: buildGeneration(),
    requiredSourceWidthPx: 1620,
    requiredSourceHeightPx: 1080,
    measurementLaneEnabled: true,
    presentTelemetryEnabled: true,
    observedAtHostMicros: 1_500_000,
  }
}

function buildPresentReport() {
  return {
    generationId: 'req-1-000001',
    viewerEpoch: 3,
    outcome: 'presented' as const,
    rejectReason: null as string | null,
    naturalWidthPx: 3840,
    naturalHeightPx: 2560,
    spans: {
      viewerReceiptAtMicros: 1_000,
      decodeStartAtMicros: 1_100,
      decodeEndAtMicros: 1_200,
      swapCommittedAtMicros: 1_300,
      imgOnLoadAtMicros: 1_150,
      actualPresentAtMicros: 1_400,
      elementTimingRenderAtMicros: null,
      isElementRenderTime: false,
    },
    clockOffsetMicros: -12_345,
    clockUncertaintyMicros: 250,
  }
}

describe('viewer-display 계약 라운드트립', () => {
  it('pins the schema version strings shared with the Rust host', () => {
    expect(viewerDisplaySchemaVersion).toBe('viewer-display/v4')
    expect(viewerDisplayUpdateSchemaVersion).toBe('viewer-display-update/v1')
    expect(viewerDisplayUpdateEvent).toBe('viewer-display-update')

    expect(rustDto).toContain(
      'pub const VIEWER_DISPLAY_SCHEMA_VERSION: &str = "viewer-display/v4";',
    )
    expect(rustDto).toContain(
      'pub const VIEWER_DISPLAY_UPDATE_SCHEMA_VERSION: &str = "viewer-display-update/v1";',
    )
  })

  it('accepts a host-shaped generation payload unchanged', () => {
    expect(displayGenerationSchema.parse(buildGeneration())).toEqual(
      buildGeneration(),
    )
  })

  it('accepts a host-shaped pointer and update envelope unchanged', () => {
    expect(displayPointerSnapshotSchema.parse(buildPointer())).toEqual(
      buildPointer(),
    )
    expect(
      displayUpdateSchema.parse({
        schemaVersion: viewerDisplayUpdateSchemaVersion,
        pointer: buildPointer(),
      }).pointer.activeGeneration?.generationId,
    ).toBe('req-1-000001')
  })

  it('accepts a portrait proxy that reaches the contain boundary without upscaling', () => {
    const pointer = {
      ...buildPointer(),
      activeGeneration: {
        ...buildProxyGeneration(),
        sourceWidthPx: 634,
        sourceHeightPx: 1080,
      },
    }

    expect(displayPointerSnapshotSchema.parse(pointer)).toEqual(pointer)
    expect(
      displayUpdateSchema.parse({
        schemaVersion: viewerDisplayUpdateSchemaVersion,
        pointer,
      }).pointer.activeGeneration?.generationId,
    ).toBe('req-1-000002')
  })

  it('rejects a pointer whose active generation cannot fill the photo rectangle', () => {
    const pointer = buildPointer()

    expect(() =>
      displayPointerSnapshotSchema.parse({
        ...pointer,
        requiredSourceWidthPx: 5000,
        requiredSourceHeightPx: 3400,
      }),
    ).toThrow()
  })

  it('rejects a pointer whose active generation belongs to another session', () => {
    const pointer = buildPointer()

    expect(() =>
      displayPointerSnapshotSchema.parse({
        ...pointer,
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g2n',
      }),
    ).toThrow()
  })

  it('rejects a pointer that claims an active generation without a measured photo rectangle', () => {
    const pointer = buildPointer()

    expect(() =>
      displayPointerSnapshotSchema.parse({
        ...pointer,
        requiredSourceWidthPx: 0,
        requiredSourceHeightPx: 0,
      }),
    ).toThrow()
  })

  it('registers the sample, display-fit proxy and raw refined tiers in that order', () => {
    expect(displayTierSchema.options).toEqual([
      'sample',
      'displayFitPresetProxy',
      'rawRefinedDisplay',
    ])
    expect(Object.keys(DISPLAY_TIER_ORDER)).toEqual([
      'sample',
      'displayFitPresetProxy',
      'rawRefinedDisplay',
    ])
    expect(compareDisplayTier('sample', 'sample')).toBe(0)
    // proxy가 fixture보다 높은 tier여야 sample이 실제 결과를 덮지 못한다.
    expect(
      compareDisplayTier('displayFitPresetProxy', 'sample'),
    ).toBeGreaterThan(0)
    // 정밀본이 proxy보다 높은 tier여야 proxy가 정밀본을 되돌리지 못한다.
    expect(
      compareDisplayTier('rawRefinedDisplay', 'displayFitPresetProxy'),
    ).toBeGreaterThan(0)
    expect(rustDto).toContain('pub const DISPLAY_TIER_SAMPLE: &str = "sample";')
    expect(rustDto).toContain(
      'pub const DISPLAY_TIER_DISPLAY_FIT_PRESET_PROXY: &str = "displayFitPresetProxy";',
    )
    expect(rustDto).toContain(
      'pub const DISPLAY_TIER_RAW_REFINED_DISPLAY: &str = "rawRefinedDisplay";',
    )
  })

  /**
   * Story 7.6 「핵심 설계 결정 2」. 계약 주석은 한때 "7.6이 `final`을 추가한다"고 적혀 있었지만
   * 그 판단은 뒤집혔다. 전체 해상도 산출물을 사진 영역에 올리면 축소를 브라우저가 하게 되어
   * darktable 축소보다 품질이 낮으면서 decode 비용은 훨씬 크고, `--upscale false` 기반
   * display-fit 계약과도 충돌한다. **조용히 빼지 않고 테스트로 막는다.**
   */
  it('never registers the full-resolution final artifact as a display tier', () => {
    expect(displayTierSchema.options).not.toContain('final')
    expect(Object.keys(DISPLAY_TIER_ORDER)).not.toContain('final')
    expect(() =>
      displayGenerationSchema.parse({
        ...buildRefinedGeneration(),
        tier: 'final',
      }),
    ).toThrow()
    expect(rustDto).not.toContain('DISPLAY_TIER_FINAL')
  })

  it('accepts a host-shaped display-fit proxy generation unchanged', () => {
    expect(displayGenerationSchema.parse(buildProxyGeneration())).toEqual(
      buildProxyGeneration(),
    )
  })

  it('accepts a host-shaped raw refined generation unchanged', () => {
    expect(displayGenerationSchema.parse(buildRefinedGeneration())).toEqual(
      buildRefinedGeneration(),
    )
  })

  /**
   * 두 tier는 같은 RAW·같은 XMP·같은 renderer·같은 목표 크기를 쓴다. `--hq`만 다르다.
   * `renderQuality`가 tier와 어긋나면 evidence는 "정밀본이 떴다"고 적고,
   * AC 6의 tier 정당성 측정이 통째로 무의미해진다.
   */
  it('refuses a generation whose render quality contradicts its tier', () => {
    expect(() =>
      displayGenerationSchema.parse({
        ...buildRefinedGeneration(),
        proxyProvenance: buildProxyGeneration().proxyProvenance,
      }),
    ).toThrow()
    expect(() =>
      displayGenerationSchema.parse({
        ...buildProxyGeneration(),
        proxyProvenance: buildRefinedGeneration().proxyProvenance,
      }),
    ).toThrow()
  })

  it('refuses a proxy generation that cannot prove where it came from', () => {
    // provenance 없는 proxy는 "이 사진이 어느 촬영·프리셋의 결과인가"를 증명할 수 없다.
    expect(() =>
      displayGenerationSchema.parse({
        ...buildProxyGeneration(),
        proxyProvenance: null,
      }),
    ).toThrow()
  })

  it('refuses a proxy generation that also claims to be a measurement fixture', () => {
    expect(() =>
      displayGenerationSchema.parse({
        ...buildProxyGeneration(),
        sampleVariant: 'a',
      }),
    ).toThrow()
  })

  it('refuses a measurement sample that carries preset provenance', () => {
    // fixture에 preset 출처가 실리면 거짓 출처가 evidence에 남는다.
    expect(() =>
      displayGenerationSchema.parse({
        ...buildGeneration(),
        proxyProvenance: buildProxyGeneration().proxyProvenance,
      }),
    ).toThrow()
  })

  it('refuses a measurement sample without its fixture variant', () => {
    expect(() =>
      displayGenerationSchema.parse({
        ...buildGeneration(),
        sampleVariant: null,
      }),
    ).toThrow()
  })

  it('reads a durable v1 pointer written by an earlier build', () => {
    // 한 HV 회차가 빌드 경계를 걸칠 수 있다. 파싱을 실패시키면 오래된 표본이
    // 분모에서 조용히 사라진다.
    const v1Pointer = {
      schemaVersion: 'viewer-display/v1',
      sessionId: SESSION_ID,
      revision: 4,
      activeGeneration: {
        generationId: 'req-1-000001',
        generationSeq: 1,
        sessionId: SESSION_ID,
        requestId: 'req-1',
        captureId: 'capture-1',
        viewerEpoch: 3,
        tier: 'sample',
        assetPath:
          'C:/dabi_shoot/sessions/a/renders/display/req-1/000001-a.jpg',
        sourceWidthPx: 3840,
        sourceHeightPx: 2560,
        byteSize: 701_308,
        sourceHash: 'fnv1a64:0123456789abcdef',
        sampleVariant: 'a',
        committedAtHostMicros: 1_500_000,
      },
      requiredSourceWidthPx: 1620,
      requiredSourceHeightPx: 1080,
      measurementLaneEnabled: true,
      observedAtHostMicros: 1_500_000,
    }

    const parsed = displayPointerSnapshotCompatSchema.parse(v1Pointer)

    expect(parsed.schemaVersion).toBe(viewerDisplaySchemaVersion)
    expect(parsed.activeGeneration?.tier).toBe('sample')
    expect(parsed.activeGeneration?.proxyProvenance).toBeNull()
    // v1에는 이 필드가 없었다. fixture lane이 켜져 있었다면 계측도 켜져 있었다.
    expect(parsed.presentTelemetryEnabled).toBe(true)
  })

  it('reads a durable v2 proxy pointer written before the resident spike', () => {
    // Story 7.4가 만든 HV-15 evidence 디렉터리는 전부 v2다.
    // `residentProvenance`를 필수로 만들면 그 회차가 통째로 분모에서 사라진다.
    const v2Proxy = buildProxyGeneration() as Record<string, unknown>
    const legacyProvenance = {
      ...(v2Proxy.proxyProvenance as Record<string, unknown>),
    }
    delete legacyProvenance.residentProvenance
    delete legacyProvenance.renderQuality

    const parsed = displayPointerSnapshotCompatSchema.parse({
      ...buildPointer(),
      schemaVersion: 'viewer-display/v2',
      activeGeneration: { ...v2Proxy, proxyProvenance: legacyProvenance },
    })

    expect(parsed.schemaVersion).toBe(viewerDisplaySchemaVersion)
    expect(parsed.activeGeneration?.tier).toBe('displayFitPresetProxy')
    // `null`은 "참조 렌더러가 직접 만들었다"는 뜻이다. 옛 행을 잘못 읽지 않는다.
    expect(
      parsed.activeGeneration?.proxyProvenance?.residentProvenance,
    ).toBeNull()
    // v2 시절 게시 경로는 proxy lane 하나였고 항상 `--hq false`였다.
    // `fast`는 추측이 아니라 그 빌드가 실제로 한 일이다.
    expect(parsed.activeGeneration?.proxyProvenance?.renderQuality).toBe('fast')
  })

  it('reads a durable v3 proxy pointer written before the raw refined tier existed', () => {
    // Story 7.5가 만든 HV-16 evidence 디렉터리는 전부 v3다.
    // `renderQuality`를 필수로 만들면 그 회차가 통째로 분모에서 사라진다.
    const v3Proxy = buildProxyGeneration() as Record<string, unknown>
    const legacyProvenance = {
      ...(v3Proxy.proxyProvenance as Record<string, unknown>),
    }
    delete legacyProvenance.renderQuality

    const parsed = displayPointerSnapshotCompatSchema.parse({
      ...buildPointer(),
      schemaVersion: 'viewer-display/v3',
      activeGeneration: { ...v3Proxy, proxyProvenance: legacyProvenance },
    })

    expect(parsed.schemaVersion).toBe(viewerDisplaySchemaVersion)
    expect(parsed.activeGeneration?.tier).toBe('displayFitPresetProxy')
    expect(parsed.activeGeneration?.proxyProvenance?.renderQuality).toBe('fast')
  })

  it('reads a durable v2/v3 generation journal row with the compat schema', () => {
    const legacyGeneration = buildProxyGeneration() as Record<string, unknown>
    const legacyProvenance = {
      ...(legacyGeneration.proxyProvenance as Record<string, unknown>),
    }
    delete legacyProvenance.residentProvenance
    delete legacyProvenance.renderQuality

    expect(() =>
      displayGenerationSchema.parse({
        ...legacyGeneration,
        proxyProvenance: legacyProvenance,
      }),
    ).toThrow()

    const parsed = displayGenerationCompatSchema.parse({
      ...legacyGeneration,
      proxyProvenance: legacyProvenance,
    })
    expect(parsed.proxyProvenance?.residentProvenance).toBeNull()
    expect(parsed.proxyProvenance?.renderQuality).toBe('fast')
  })

  it('separates the renderer that made the frame from the renderer it is compared against', () => {
    // 상주 후보가 만든 프레임에 `referenceRenderer: darktable`만 남기면 evidence가 거짓말을 한다.
    const generation = {
      ...buildProxyGeneration(),
      proxyProvenance: {
        ...buildProxyGeneration().proxyProvenance,
        approvalBasis: 'visual-approval' as const,
        sourceRoute: 'predecoded-fixture',
        residentProvenance: buildResidentProvenance(),
      },
    }

    const parsed = displayGenerationSchema.parse(generation)

    expect(parsed.proxyProvenance?.referenceRenderer).toBe('darktable')
    expect(parsed.proxyProvenance?.residentProvenance?.producerRenderer).toBe(
      'webgl2-resident',
    )
  })

  it('refuses a fixture-only resident frame that claims production eligibility', () => {
    // 이 Story에서 가장 쉬운 자기기만이다. 계약 경계에서 막는다 (AC 6).
    expect(() =>
      residentProducerProvenanceSchema.parse({
        ...buildResidentProvenance(),
        productionEligible: true,
        adoptionBlockReason: null,
      }),
    ).toThrow()
  })

  it('refuses a production claim from an unapproved direct decoder', () => {
    expect(RESIDENT_APPROVED_DIRECT_DECODERS).toHaveLength(0)
    expect(() =>
      residentProducerProvenanceSchema.parse({
        ...buildResidentProvenance(),
        inputProvenance: 'real-capture-direct',
        inputProducer: 'windows-wic-raw',
        productionEligible: true,
        adoptionBlockReason: null,
      }),
    ).toThrow()
  })

  it('refuses a production claim whose hot path still compiled or spawned', () => {
    expect(() =>
      residentProducerProvenanceSchema.parse({
        ...buildResidentProvenance(),
        inputProvenance: 'real-capture-direct',
        inputProducer: 'windows-wic-raw',
        hotPathProgramCompileCount: 1,
        productionEligible: true,
        adoptionBlockReason: null,
      }),
    ).toThrow()
  })

  it('refuses a demoted resident frame that hides why it was demoted', () => {
    expect(() =>
      residentProducerProvenanceSchema.parse({
        ...buildResidentProvenance(),
        productionEligible: false,
        adoptionBlockReason: null,
      }),
    ).toThrow()
  })

  it('closes an unknown resident mode to off instead of guessing', () => {
    expect(parseResidentRendererMode(undefined)).toBe('off')
    expect(parseResidentRendererMode('')).toBe('off')
    expect(parseResidentRendererMode('on')).toBe('off')
    expect(parseResidentRendererMode('EVIDENCE')).toBe('off')
    expect(parseResidentRendererMode(' shadow ')).toBe('shadow')
    expect(parseResidentRendererMode('evidence')).toBe('evidence')
  })

  it('keeps the resident contract identical on the Rust side', () => {
    expect(rustDto).toContain('pub struct ResidentProducerProvenanceDto')
    expect(rustDto).toContain(
      'pub const RESIDENT_APPROVED_DIRECT_DECODERS: &[&str] = &[];',
    )

    for (const value of ['off', 'shadow', 'evidence']) {
      expect(rustDto).toContain(`"${value}"`)
    }

    for (const value of residentInputProvenanceSchema.options) {
      expect(rustDto).toContain(`"${value}"`)
    }
  })

  it('keeps every reject reason defined on both sides of the boundary', () => {
    for (const reason of displayHostRejectReasonSchema.options) {
      expect(rustDto).toContain(`"${reason}"`)
    }
  })

  it('lets only the host conclude that a committed generation was never reported', () => {
    // commit·notify까지 끝난 generation에 terminal 보고가 오지 않았다는 것은 host의 관측이다.
    // viewer가 스스로 주장할 수 있는 결론이 아니므로 viewer → host 계약에서는 거부되어야 한다.
    expect(displayHostRejectReasonSchema.options).toContain('present-unreported')
    expect(displayRejectReasonSchema.options).not.toContain('present-unreported')
    expect(() =>
      displayPresentReportSchema.parse({
        ...buildPresentReport(),
        outcome: 'rejected',
        rejectReason: 'present-unreported',
      }),
    ).toThrow()
    expect(rustDto).toContain(
      'pub const DISPLAY_REJECT_PRESENT_UNREPORTED: &str = "present-unreported";',
    )
    // host의 거부 목록에도 넣으면 viewer 보고가 통과해 버린다.
    expect(rustDto).not.toContain('| DISPLAY_REJECT_PRESENT_UNREPORTED')
  })

  it('accepts the present report and clock probe payloads', () => {
    const report = {
      generationId: 'req-1-000001',
      viewerEpoch: 3,
      outcome: 'presented' as const,
      rejectReason: null,
      naturalWidthPx: 3840,
      naturalHeightPx: 2560,
      spans: {
        viewerReceiptAtMicros: 1_000,
        decodeStartAtMicros: 1_100,
        decodeEndAtMicros: 1_200,
        swapCommittedAtMicros: 1_300,
        imgOnLoadAtMicros: 1_150,
        actualPresentAtMicros: 1_400,
        elementTimingRenderAtMicros: null,
        isElementRenderTime: false,
      },
      clockOffsetMicros: -12_345,
      clockUncertaintyMicros: 250,
    }

    expect(displayPresentReportSchema.parse(report)).toEqual(report)
    expect(clockProbeSchema.parse({ clientSentMicros: 5 })).toEqual({
      clientSentMicros: 5,
    })
    expect(
      clockProbeResultSchema.parse({
        hostMonotonicMicros: 10,
        hostEpochMicros: 20,
      }),
    ).toEqual({ hostMonotonicMicros: 10, hostEpochMicros: 20 })
  })

  it('rejects contradictory present outcomes and reasons', () => {
    const presented = buildPresentReport()

    expect(() =>
      displayPresentReportSchema.parse({
        ...presented,
        rejectReason: 'undecodable',
      }),
    ).toThrow()
    expect(() =>
      displayPresentReportSchema.parse({
        ...presented,
        outcome: 'decode-failed',
        rejectReason: null,
        spans: { ...presented.spans, actualPresentAtMicros: null },
      }),
    ).toThrow()

    expect(() =>
      displayPresentReportSchema.parse({
        ...presented,
        outcome: 'decode-failed',
        rejectReason: 'older-request',
        naturalWidthPx: 0,
        naturalHeightPx: 0,
        spans: { ...presented.spans, actualPresentAtMicros: null },
      }),
    ).toThrow()
    expect(() =>
      displayPresentReportSchema.parse({
        ...presented,
        outcome: 'rejected',
        rejectReason: 'decode-failed',
        naturalWidthPx: 0,
        naturalHeightPx: 0,
        spans: { ...presented.spans, actualPresentAtMicros: null },
      }),
    ).toThrow()

    expect(
      displayPresentReportSchema.parse({
        ...presented,
        outcome: 'decode-failed',
        rejectReason: 'decode-failed',
        naturalWidthPx: 0,
        naturalHeightPx: 0,
        spans: { ...presented.spans, actualPresentAtMicros: null },
      }).outcome,
    ).toBe('decode-failed')
  })

  it('rejects a presented report with zero natural dimensions', () => {
    expect(() =>
      displayPresentReportSchema.parse({
        ...buildPresentReport(),
        naturalWidthPx: 0,
      }),
    ).toThrow()
    expect(() =>
      displayPresentReportSchema.parse({
        ...buildPresentReport(),
        naturalHeightPx: 0,
      }),
    ).toThrow()
  })

  it('accepts the trusted input report and allows a negative clock offset', () => {
    const report = {
      sessionId: SESSION_ID,
      requestId: 'req-1',
      clientInputMicros: 42,
      clockOffsetMicros: -900_000,
      clockUncertaintyMicros: 130,
      isTrusted: true,
    }

    expect(trustedInputReportSchema.parse(report)).toEqual(report)
  })

  it('keeps the Rust DTO field names in camelCase across the boundary', () => {
    for (const marker of [
      'pub struct DisplayGenerationDto',
      'pub struct DisplayPointerSnapshotDto',
      'pub struct DisplayUpdateDto',
      'pub struct DisplayPresentReportDto',
      'pub struct DisplayPresentSpansDto',
      'pub struct ClockProbeResultDto',
      'pub struct TrustedInputReportDto',
    ]) {
      expect(rustDto).toContain(marker)
    }

    // camelCase 직렬화가 빠지면 TS 경계에서 전부 undefined가 된다.
    const displaySection = rustDto.slice(
      rustDto.indexOf('pub const VIEWER_DISPLAY_SCHEMA_VERSION'),
    )
    const structCount = (displaySection.match(/pub struct /g) ?? []).length
    const renameCount = (
      displaySection.match(/#\[serde\(rename_all = "camelCase"\)\]/g) ?? []
    ).length

    expect(renameCount).toBe(structCount)
  })
})
