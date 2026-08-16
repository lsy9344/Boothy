import { describe, expect, it } from 'vitest'

import {
  buildProductionRenderArguments,
  measurePairFromPpm,
  validateTierManifest,
  type Hv17TierManifest,
} from './run-tier-evidence.ts'

function ppm(widthPx: number, heightPx: number, blurPx: number): Uint8Array {
  const header = new TextEncoder().encode(`P6\n${widthPx} ${heightPx}\n255\n`)
  const data = new Uint8Array(widthPx * heightPx * 3)
  for (let y = 0; y < heightPx; y += 1) {
    const edge = widthPx / 2 + y / 16
    for (let x = 0; x < widthPx; x += 1) {
      const value = Math.round((0.5 + 0.5 * Math.tanh((x - edge) / blurPx)) * 255)
      const offset = (y * widthPx + x) * 3
      data.fill(value, offset, offset + 3)
    }
  }
  return new Uint8Array([...header, ...data])
}

describe('HV-17 tier evidence runner', () => {
  it('keeps the production render arguments equal except for hq and isolated worker roots', () => {
    const common = { targetWidthPx: 1620, targetHeightPx: 1080, jpegQuality: 92 }
    const proxy = buildProductionRenderArguments(common, 'C:\\a.cr2', 'C:\\p.xmp', 'C:\\out.jpg', 'proxy', 'C:\\evidence\\proxy')
    const refined = buildProductionRenderArguments(common, 'C:\\a.cr2', 'C:\\p.xmp', 'C:\\out.jpg', 'refined', 'C:\\evidence\\refined')
    const differences = proxy.map((value, index) => [value, refined[index]]).filter(([left, right]) => left !== right)

    expect(differences).toEqual([
      ['false', 'true'],
      ['C:/evidence/proxy/config', 'C:/evidence/refined/config'],
      ['C:/evidence/proxy/library.db', 'C:/evidence/refined/library.db'],
    ])
  })

  it('measures the actual PPM pair with the shared quality metrics', () => {
    const measured = measurePairFromPpm(
      ppm(64, 64, 2.5),
      ppm(64, 64, 0.8),
      { sampleId: 'raw-1', presetId: 'daylight', edgeRoi: { x: 0, y: 0, widthPx: 64, heightPx: 64 }, exposureConfirmed: true },
    )

    expect(measured.sample.proxyMtf50).not.toBeNull()
    expect(measured.sample.refinedMtf50).toBeGreaterThan(measured.sample.proxyMtf50 as number)
    expect(measured.sample.look).not.toBeNull()
    expect(measured.sample.underexposed).toBe(false)
  })

  it('rejects a corpus that could be mistaken for a complete 3 by 3 hardware run', () => {
    const manifest = {
      schemaVersion: 'hv17-tier-run/v1', evidenceRoot: 'C:\\run-hv17', darktableCli: 'C:\\darktable-cli.exe',
      targetWidthPx: 1620, targetHeightPx: 1080, jpegQuality: 92,
      outputColorSpace: 'sRGB', iccIntent: 'PERCEPTUAL', samples: [], presets: [],
    } as unknown as Hv17TierManifest

    expect(() => validateTierManifest(manifest)).toThrow(/three uniquely named RAW samples/)
  })

  it('rejects an absolute output directory that is not explicitly an HV-17 run', () => {
    const manifest = {
      schemaVersion: 'hv17-tier-run/v1', evidenceRoot: 'C:\\evidence', darktableCli: 'C:\\darktable-cli.exe',
      targetWidthPx: 1620, targetHeightPx: 1080, jpegQuality: 92,
      outputColorSpace: 'sRGB', iccIntent: 'PERCEPTUAL', samples: [], presets: [],
    } as unknown as Hv17TierManifest

    expect(() => validateTierManifest(manifest)).toThrow(/identify an HV-17 run/)
  })

  it('records unconfirmed exposure as unmeasured instead of producing an approval metric', () => {
    const measured = measurePairFromPpm(
      ppm(64, 64, 2.5),
      ppm(64, 64, 0.8),
      { sampleId: 'dark-1', presetId: 'daylight', edgeRoi: { x: 0, y: 0, widthPx: 64, heightPx: 64 }, exposureConfirmed: false },
    )

    expect(measured.sample.proxyMtf50).toBeNull()
    expect(measured.sample.refinedMtf50).toBeNull()
    expect(measured.sample.look).toBeNull()
    expect(measured.sample.underexposed).toBe(true)
  })
})
