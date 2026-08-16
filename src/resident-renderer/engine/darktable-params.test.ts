import { describe, expect, it } from 'vitest'

import {
  decodeOperationParams,
  decodeParamBytes,
  readFloats,
} from './darktable-params'

/**
 * 아래 hex는 전부 `src-tauri/src/preset/default_catalog_assets/*.xmp`의 실제 값이다.
 * 합성한 값이 아니라 지금 게시되어 있는 세 preset의 파라미터다.
 */
const DAYLIGHT = {
  temperature: '000010400000803fcdccec3f0000000002000000',
  exposure: '00000000000080b9cdcc8c3f00004842000080c00000000001000000',
  sigmoid:
    '3333d33fcdcccc3d0000c8426c09793c000000000000c8420000000000000000000000000000000000000000000000000000000000000000',
  bloom: '000040410000be4200008040',
  sharpen: '000000409a99193fcdcccc3e',
} as const

const MONO_POP = {
  monochrome: '00000000000000000000004000000000',
  exposure: '00000000000080b93333b33f00004842000080c00000000001000000',
} as const

describe('darktable params 복원', () => {
  it('rejects hex that is not a whole number of bytes', () => {
    expect(decodeParamBytes('abc')).toBeNull()
    expect(decodeParamBytes('')).toBeNull()
    expect(decodeParamBytes('zzzz')).toBeNull()
    expect(decodeParamBytes('00ff')).toEqual(new Uint8Array([0x00, 0xff]))
  })

  it('reads little-endian floats the way darktable writes them', () => {
    const bytes = decodeParamBytes('0000803f')

    expect(bytes).not.toBeNull()
    expect(readFloats(bytes!)).toEqual([1])
  })

  it('restores the published Daylight parameters exactly', () => {
    expect(
      decodeOperationParams('temperature', 4, DAYLIGHT.temperature),
    ).toEqual({
      kind: 'temperature',
      red: 2.25,
      green: 1,
      blue: 1.850000023841858,
      g2: 0,
      preset: 2,
    })

    const exposure = decodeOperationParams('exposure', 7, DAYLIGHT.exposure)

    expect(exposure).toMatchObject({ kind: 'exposure', mode: 0 })
    expect((exposure as { exposureEv: number }).exposureEv).toBeCloseTo(1.1, 5)
    expect((exposure as { black: number }).black).toBeCloseTo(-0.000244, 6)

    const sigmoid = decodeOperationParams('sigmoid', 3, DAYLIGHT.sigmoid)

    expect(sigmoid).toMatchObject({ kind: 'sigmoid' })
    expect((sigmoid as { middleGreyContrast: number }).middleGreyContrast).toBeCloseTo(
      1.65,
      5,
    )
    expect((sigmoid as { contrastSkewness: number }).contrastSkewness).toBeCloseTo(
      0.1,
      5,
    )
    expect((sigmoid as { displayWhiteTarget: number }).displayWhiteTarget).toBe(100)

    expect(decodeOperationParams('bloom', 1, DAYLIGHT.bloom)).toEqual({
      kind: 'bloom',
      size: 12,
      threshold: 95,
      strength: 4,
    })

    const sharpen = decodeOperationParams('sharpen', 1, DAYLIGHT.sharpen)

    expect(sharpen).toMatchObject({ kind: 'sharpen', radius: 2 })
    expect((sharpen as { amount: number }).amount).toBeCloseTo(0.6, 5)
    expect((sharpen as { threshold: number }).threshold).toBeCloseTo(0.4, 5)
  })

  it('restores the published Mono Pop parameters exactly', () => {
    expect(decodeOperationParams('monochrome', 2, MONO_POP.monochrome)).toEqual({
      kind: 'monochrome',
      a: 0,
      b: 0,
      size: 2,
      highlights: 0,
    })

    const exposure = decodeOperationParams('exposure', 7, MONO_POP.exposure)

    // Daylight(1.1)과 다른 값이어야 한다. 같으면 preset이 서로 구분되지 않는다.
    expect((exposure as { exposureEv: number }).exposureEv).toBeCloseTo(1.4, 5)
  })

  it('refuses a params blob whose module version does not match', () => {
    // darktable이 구조체를 바꿨는데 같은 바이트로 읽으면 조용히 다른 화면이 나온다.
    expect(decodeOperationParams('temperature', 3, DAYLIGHT.temperature)).toBeNull()
    expect(decodeOperationParams('exposure', 6, DAYLIGHT.exposure)).toBeNull()
    expect(decodeOperationParams('sigmoid', 2, DAYLIGHT.sigmoid)).toBeNull()
  })

  it('refuses a params blob whose length does not match the struct', () => {
    expect(decodeOperationParams('bloom', 1, '0000404100')).toBeNull()
    expect(decodeOperationParams('sharpen', 1, '00000040')).toBeNull()
  })

  it('refuses an operation the engine never implemented', () => {
    expect(decodeOperationParams('retouch', 1, '00000000')).toBeNull()
    expect(decodeOperationParams('colorbalancergb', 5, '00000000')).toBeNull()
  })
})
