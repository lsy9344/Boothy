import { describe, expect, it } from 'vitest'

// Rust DTO를 원문 그대로 읽어 TS 계약과 대조한다.
// 두 정의가 어긋나면 경계에서 조용히 undefined가 되므로 문자열 수준으로 고정한다.
import rustDto from '../../src-tauri/src/contracts/dto.rs?raw'

import {
  DISPLAY_TIER_ORDER,
  clockProbeResultSchema,
  clockProbeSchema,
  compareDisplayTier,
  displayGenerationSchema,
  displayHostRejectReasonSchema,
  displayPointerSnapshotSchema,
  displayPresentReportSchema,
  displayRejectReasonSchema,
  displayTierSchema,
  displayUpdateSchema,
  trustedInputReportSchema,
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
    committedAtHostMicros: 1_500_000,
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
    expect(viewerDisplaySchemaVersion).toBe('viewer-display/v1')
    expect(viewerDisplayUpdateSchemaVersion).toBe('viewer-display-update/v1')
    expect(viewerDisplayUpdateEvent).toBe('viewer-display-update')

    expect(rustDto).toContain(
      'pub const VIEWER_DISPLAY_SCHEMA_VERSION: &str = "viewer-display/v1";',
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

  it('registers exactly one tier in Story 7.2 and keeps the order helper ready', () => {
    expect(displayTierSchema.options).toEqual(['sample'])
    expect(Object.keys(DISPLAY_TIER_ORDER)).toEqual(['sample'])
    expect(compareDisplayTier('sample', 'sample')).toBe(0)
    expect(rustDto).toContain('pub const DISPLAY_TIER_SAMPLE: &str = "sample";')
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
