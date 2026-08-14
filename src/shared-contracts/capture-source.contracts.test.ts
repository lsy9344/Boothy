import { describe, expect, it } from 'vitest'

// Rust DTO를 원문 그대로 읽어 TS 계약과 대조한다.
// 두 정의가 어긋나면 경계에서 조용히 undefined가 되므로 문자열 수준으로 고정한다.
import rustDto from '../../src-tauri/src/contracts/dto.rs?raw'
// helper가 실제로 판정에 쓰는 값도 같은 방식으로 대조한다. 계약 문서가 아니라
// 구현이 truth라는 것을 테스트가 강제한다.
import correlatorSource from '../../sidecar/canon-helper/src/CanonHelper/Runtime/CaptureObjectCorrelator.cs?raw'

import {
  captureSourceSchemaVersion,
  imageQualityCapabilitySchema,
  sourceBlockOrderSchema,
  sourceCandidateSchema,
  sourceCompareModeSchema,
  sourceComparisonSampleSchema,
  sourceComparisonSchemaVersion,
  sourceObjectRoleSchema,
  sourceRejectReasonSchema,
  sourceRouteSchema,
} from '.'

const SESSION_ID = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

function buildCandidate() {
  return {
    captureId: 'capture_20260812_001',
    requestId: 'capture_req_20260812_001',
    sessionId: SESSION_ID,
    route: 'embedded-jpeg' as const,
    assetPath:
      'C:/dabi_shoot/sessions/a/renders/sources/capture_20260812_001-embedded.jpg',
    widthPx: 5184,
    heightPx: 3456,
    byteSize: 2_104_882,
    exifOrientation: 1,
    decodeValid: true,
    sourceHash: 'fnv1a64:0123456789abcdef',
    objectIndex: 0,
    groupId: 42,
    readyAtHostMicros: 1_500_000,
    extractionCostMicros: 12_000,
  }
}

function buildSample() {
  return {
    schemaVersion: sourceComparisonSchemaVersion,
    candidate: buildCandidate(),
    accepted: true,
    rejectReason: null,
    objectRole: 'jpeg' as const,
    usedFallbackCorrelation: false,
    blockOrder: 'AB' as const,
    blockIndex: 3,
    isWarmUp: false,
    randomizationSeed: 20260812,
    isPresetApplied: false as const,
    recordedAtHostMicros: 1_500_100,
  }
}

describe('capture-source contracts', () => {
  it('accepts a well formed candidate and comparison sample', () => {
    expect(sourceCandidateSchema.parse(buildCandidate())).toMatchObject({
      route: 'embedded-jpeg',
      objectIndex: 0,
    })
    expect(sourceComparisonSampleSchema.parse(buildSample())).toMatchObject({
      accepted: true,
      isPresetApplied: false,
    })
  })

  it('represents a missing candidate without fabricated measurements', () => {
    const missing = {
      ...buildCandidate(),
      assetPath: null,
      widthPx: null,
      heightPx: null,
      byteSize: null,
      objectIndex: null,
      decodeValid: false,
      sourceHash: null,
    }

    expect(sourceCandidateSchema.parse(missing)).toMatchObject({
      assetPath: null,
      widthPx: null,
      objectIndex: null,
    })
    expect(() =>
      sourceCandidateSchema.parse({ ...missing, widthPx: 0 }),
    ).toThrow()
    expect(
      sourceCandidateSchema.parse({
        ...buildCandidate(),
        widthPx: null,
        heightPx: null,
        decodeValid: false,
      }),
    ).toMatchObject({ assetPath: buildCandidate().assetPath, widthPx: null })
  })

  it('validates source hashes and EXIF orientation at the boundary', () => {
    expect(() =>
      sourceCandidateSchema.parse({
        ...buildCandidate(),
        sourceHash: 'sha256:not-the-contract',
      }),
    ).toThrow()
    expect(() =>
      sourceCandidateSchema.parse({ ...buildCandidate(), exifOrientation: 9 }),
    ).toThrow()
  })

  it('requires accepted and rejectReason to describe one outcome', () => {
    expect(() =>
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        rejectReason: 'corrupt',
      }),
    ).toThrow()
    expect(() =>
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        accepted: false,
      }),
    ).toThrow()
  })

  it('requires complete provenance for accepted samples', () => {
    expect(() =>
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        candidate: { ...buildCandidate(), sourceHash: null },
      }),
    ).toThrow()
    expect(() =>
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        objectRole: null,
      }),
    ).toThrow()
    expect(() =>
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        objectRole: 'raw',
      }),
    ).toThrow()
  })

  it('allows fallback correlation only for paired JPEG samples', () => {
    expect(() =>
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        usedFallbackCorrelation: true,
      }),
    ).toThrow()

    expect(
      sourceComparisonSampleSchema.parse({
        ...buildSample(),
        candidate: {
          ...buildCandidate(),
          route: 'camera-paired-jpeg',
          groupId: null,
        },
        usedFallbackCorrelation: true,
      }),
    ).toMatchObject({ usedFallbackCorrelation: true })
  })

  it('does not infer RAW+JPEG support without a descriptor', () => {
    expect(() =>
      imageQualityCapabilitySchema.parse({
        descriptorAvailable: false,
        currentValue: null,
        supportedValues: [],
        rawPlusJpegSupported: true,
        probedAtHostMicros: 900,
      }),
    ).toThrow()
  })

  it('registers the incumbent as a first-class route', () => {
    // 기준선이 계약에 없으면 "새 route가 더 빠르다"는 주장에 비교 대상이 없다.
    expect(sourceRouteSchema.options).toContain('windows-shell-thumbnail')
    expect(sourceRouteSchema.options).toHaveLength(3)
  })

  it('keeps every reject reason distinct so nothing is silently ignored', () => {
    const reasons = sourceRejectReasonSchema.options

    expect(new Set(reasons).size).toBe(reasons.length)
    expect(reasons).toEqual([
      'absent',
      'partial',
      'corrupt',
      'undecodable',
      'orientation-unsupported',
      'wrong-session',
      'wrong-request',
      'wrong-capture',
      'stale',
      'unsupported-combination',
      'extraction-failed',
      'cancelled',
    ])
  })

  it('defaults the comparison lane to off', () => {
    expect(sourceCompareModeSchema.options[0]).toBe('off')
    expect(sourceCompareModeSchema.parse('off')).toBe('off')
    expect(() => sourceCompareModeSchema.parse('on')).toThrow()
  })

  // -------------------------------------------------------------------
  // AC 4: 보정되지 않은 source가 qualifying preset-applied frame이 되지 않는다
  // -------------------------------------------------------------------

  it('refuses any sample that claims a preset was applied', () => {
    const lying = { ...buildSample(), isPresetApplied: true }

    expect(() => sourceComparisonSampleSchema.parse(lying)).toThrow()
  })

  it('requires isPresetApplied to be present rather than defaulting silently', () => {
    const sample = buildSample() as Record<string, unknown>
    delete sample.isPresetApplied

    expect(() => sourceComparisonSampleSchema.parse(sample)).toThrow()
  })

  // -------------------------------------------------------------------
  // TS ↔ Rust 라운드트립
  // -------------------------------------------------------------------

  it('pins the schema versions on both sides', () => {
    expect(rustDto).toContain(
      `pub const CAPTURE_SOURCE_SCHEMA_VERSION: &str = "${captureSourceSchemaVersion}";`,
    )
    expect(rustDto).toContain(
      `pub const SOURCE_COMPARISON_SCHEMA_VERSION: &str = "${sourceComparisonSchemaVersion}";`,
    )
  })

  it('pins every route string in the Rust DTO', () => {
    for (const route of sourceRouteSchema.options) {
      expect(rustDto).toContain(`"${route}"`)
    }
  })

  it('pins every reject reason string in the Rust DTO', () => {
    for (const reason of sourceRejectReasonSchema.options) {
      expect(rustDto).toContain(`"${reason}"`)
    }
  })

  it('pins object roles, block orders and lane modes in the Rust DTO', () => {
    for (const role of sourceObjectRoleSchema.options) {
      expect(rustDto).toContain(`pub const SOURCE_OBJECT_ROLE_`)
      expect(rustDto).toContain(`"${role}"`)
    }

    for (const order of sourceBlockOrderSchema.options) {
      expect(rustDto).toContain(`"${order}"`)
    }

    for (const mode of sourceCompareModeSchema.options) {
      expect(rustDto).toContain(`"${mode}"`)
    }
  })

  it('keeps candidate field names identical across the boundary', () => {
    // camelCase 직렬화이므로 Rust는 snake_case로 같은 필드를 가진다.
    const rustFieldNames = [
      'capture_id',
      'request_id',
      'session_id',
      'route',
      'asset_path',
      'width_px',
      'height_px',
      'byte_size',
      'exif_orientation',
      'decode_valid',
      'source_hash',
      'object_index',
      'group_id',
      'ready_at_host_micros',
      'extraction_cost_micros',
    ]
    const candidateDto = rustDto.slice(
      rustDto.indexOf('pub struct SourceCandidateDto'),
    )

    for (const field of rustFieldNames) {
      expect(candidateDto).toContain(`pub ${field}:`)
    }

    // 필드 수가 어긋나면 한쪽에만 추가된 필드가 있다는 뜻이다.
    expect(Object.keys(sourceCandidateSchema.shape)).toHaveLength(
      rustFieldNames.length,
    )
  })

  it('keeps the image quality capability shape identical across the boundary', () => {
    const capabilityDto = rustDto.slice(
      rustDto.indexOf('pub struct ImageQualityCapabilityDto'),
    )

    for (const field of [
      'descriptor_available',
      'current_value',
      'supported_values',
      'raw_plus_jpeg_supported',
      'probed_at_host_micros',
    ]) {
      expect(capabilityDto).toContain(`pub ${field}:`)
    }

    expect(
      imageQualityCapabilitySchema.parse({
        descriptorAvailable: true,
        currentValue: 0x00640013,
        supportedValues: [0x0010ff0f, 0x00640013],
        rawPlusJpegSupported: true,
        probedAtHostMicros: 900,
      }).rawPlusJpegSupported,
    ).toBe(true)
  })

  // -------------------------------------------------------------------
  // helper 구현과의 대조
  // -------------------------------------------------------------------

  it('matches the object roles the helper actually resolves', () => {
    for (const role of sourceObjectRoleSchema.options) {
      const pascal = role === 'raw' ? 'Raw' : 'Jpeg'
      expect(correlatorSource).toContain(`CaptureObjectRole.${pascal}`)
    }
  })

  it('confirms the helper no longer defaults an unknown RAW extension to cr3', () => {
    // 구현에 없는 값을 문서가 주장하던 문제를 되풀이하지 않기 위한 고정이다.
    expect(correlatorSource).toContain('return ".cr2";')
    expect(correlatorSource).not.toContain('return ".cr3";')
  })
})
