import { describe, expect, it } from 'vitest'

import {
  clippedFraction,
  decodePpm,
  deltaE2000,
  deltaE2000Report,
  measureParity,
  mtf50FromSlantedEdge,
  srgbToLab,
  ssim,
  type RgbImage,
} from './parity-metrics'

/**
 * Sharma·Wu·Dalal(2005) CIEDE2000 검증 표에서 뽑은 쌍.
 *
 * 색차 공식은 부호 하나만 틀려도 계속 그럴듯한 값을 낸다. 검증 표 없이 통과시키면
 * 이 Story의 parity 숫자가 전부 조용히 틀린 값이 된다.
 */
const SHARMA_PAIRS: [number[], number[], number][] = [
  [[50, 2.6772, -79.7751], [50, 0, -82.7485], 2.0425],
  [[50, 3.1571, -77.2803], [50, 0, -82.7485], 2.8615],
  [[50, 2.8361, -74.02], [50, 0, -82.7485], 3.4412],
  [[50, -1.3802, -84.2814], [50, 0, -82.7485], 1.0],
  [[50, -1.1848, -84.8006], [50, 0, -82.7485], 1.0],
  [[50, -0.9009, -85.5211], [50, 0, -82.7485], 1.0],
  [[50, 0, 0], [50, -1, 2], 2.3669],
  [[50, -1, 2], [50, 0, 0], 2.3669],
  [[50, 2.49, -0.001], [50, -2.49, 0.0009], 7.1792],
  [[50, 2.49, -0.001], [50, -2.49, 0.001], 7.1792],
  [[50, 2.5, 0], [50, 0, -2.5], 4.3065],
  [[60.2574, -34.0099, 36.2677], [60.4626, -34.1751, 39.4387], 1.2644],
  [[63.0109, -31.0961, -5.8663], [62.8187, -29.7946, -4.0864], 1.263],
  [[35.0831, -44.1164, 3.7933], [35.0232, -40.0716, 1.5901], 1.8645],
  [[61.2901, 3.7196, -5.3901], [61.4292, 2.248, -4.962], 1.8731],
  [[22.7233, 20.0904, -46.694], [23.0331, 14.973, -42.5619], 2.0373],
  [[2.0776, 0.0795, -1.135], [0.9033, -0.0636, -0.5514], 0.9082],
]

function solid(
  widthPx: number,
  heightPx: number,
  rgb: [number, number, number],
): RgbImage {
  const data = new Float32Array(widthPx * heightPx * 3)

  for (let index = 0; index < data.length; index += 3) {
    data[index] = rgb[0]
    data[index + 1] = rgb[1]
    data[index + 2] = rgb[2]
  }

  return { widthPx, heightPx, data }
}

function ppmBytes(
  widthPx: number,
  heightPx: number,
  maxValue: 255 | 65535,
  fill: number,
): Uint8Array {
  const header = new TextEncoder().encode(
    `P6\n${widthPx} ${heightPx}\n${maxValue}\n`,
  )
  const sampleCount = widthPx * heightPx * 3
  const bytesPerSample = maxValue === 255 ? 1 : 2
  const out = new Uint8Array(header.length + sampleCount * bytesPerSample)
  out.set(header, 0)

  for (let index = 0; index < sampleCount; index += 1) {
    const offset = header.length + index * bytesPerSample

    if (maxValue === 255) {
      out[offset] = fill
    } else {
      out[offset] = (fill >> 8) & 0xff
      out[offset + 1] = fill & 0xff
    }
  }

  return out
}

describe('CIEDE2000', () => {
  it('matches the published Sharma verification pairs', () => {
    for (const [left, right, expected] of SHARMA_PAIRS) {
      const measured = deltaE2000(
        { l: left[0], a: left[1], b: left[2] },
        { l: right[0], a: right[1], b: right[2] },
      )

      expect(measured).toBeCloseTo(expected, 3)
    }
  })

  it('is symmetric', () => {
    const first = { l: 50, a: 2.6772, b: -79.7751 }
    const second = { l: 50, a: 0, b: -82.7485 }

    expect(deltaE2000(first, second)).toBeCloseTo(deltaE2000(second, first), 10)
  })

  it('reports zero for identical colors', () => {
    expect(deltaE2000({ l: 40, a: 12, b: -7 }, { l: 40, a: 12, b: -7 })).toBe(0)
  })
})

describe('sRGB → Lab', () => {
  it('maps pure white and black to the Lab endpoints', () => {
    const white = srgbToLab(1, 1, 1)
    const black = srgbToLab(0, 0, 0)

    expect(white.l).toBeCloseTo(100, 3)
    expect(white.a).toBeCloseTo(0, 3)
    expect(white.b).toBeCloseTo(0, 3)
    expect(black.l).toBeCloseTo(0, 6)
  })

  it('maps mid grey to the known lightness', () => {
    // sRGB 128/255는 L* ≈ 53.6이다. 감마를 잊으면 50 근처가 나온다.
    expect(srgbToLab(128 / 255, 128 / 255, 128 / 255).l).toBeCloseTo(53.585, 2)
  })
})

describe('PPM 읽기', () => {
  it('reads an 8-bit P6 image', () => {
    const image = decodePpm(ppmBytes(4, 2, 255, 255))

    expect(image).not.toBeNull()
    expect(image!.widthPx).toBe(4)
    expect(image!.heightPx).toBe(2)
    expect(image!.data[0]).toBeCloseTo(1, 6)
  })

  it('reads a 16-bit P6 image the way darktable writes it', () => {
    const image = decodePpm(ppmBytes(3, 3, 65535, 65535))

    expect(image).not.toBeNull()
    expect(image!.data[0]).toBeCloseTo(1, 6)
  })

  it('treats CRLF after max value as one binary separator', () => {
    const header = new TextEncoder().encode('P6\r\n1 1\r\n255\r\n')
    const bytes = new Uint8Array([...header, 255, 0, 0])
    const image = decodePpm(bytes)

    expect(image?.data[0]).toBe(1)
    expect(image?.data[1]).toBe(0)
  })

  it('refuses dimensions that would allocate an unbounded sample buffer', () => {
    expect(decodePpm(new TextEncoder().encode('P6\n999999 999999\n255\n'))).toBeNull()
  })

  it('refuses a header it does not understand instead of guessing', () => {
    expect(decodePpm(new TextEncoder().encode('P3\n2 2\n255\n'))).toBeNull()
    expect(decodePpm(new TextEncoder().encode('P6\n2 2\n1023\n'))).toBeNull()
    // 선언한 크기보다 데이터가 짧으면 나머지를 0으로 채우지 않는다.
    expect(decodePpm(new TextEncoder().encode('P6\n8 8\n255\nabc'))).toBeNull()
  })
})

describe('SSIM', () => {
  it('is 1 for identical images', () => {
    const image = solid(32, 32, [0.4, 0.5, 0.6])

    expect(ssim(image, image)).toBeCloseTo(1, 10)
  })

  it('falls well below the 0.95 gate for a visibly different image', () => {
    const measured = ssim(solid(32, 32, [0.2, 0.2, 0.2]), solid(32, 32, [0.8, 0.8, 0.8]))

    expect(measured).not.toBeNull()
    expect(measured!).toBeLessThan(0.95)
  })

  it('refuses mismatched dimensions rather than comparing the overlap', () => {
    expect(ssim(solid(32, 32, [0.5, 0.5, 0.5]), solid(16, 16, [0.5, 0.5, 0.5]))).toBeNull()
  })

  it('includes pixels in partial blocks at the right and bottom edges', () => {
    const candidate = solid(9, 9, [0.5, 0.5, 0.5])
    const reference = solid(9, 9, [0.5, 0.5, 0.5])

    for (let y = 0; y < 9; y += 1) {
      const offset = (y * 9 + 8) * 3
      candidate.data[offset] = 0
      candidate.data[offset + 1] = 0
      candidate.data[offset + 2] = 0
      reference.data[offset] = 1
      reference.data[offset + 1] = 1
      reference.data[offset + 2] = 1
    }

    expect(ssim(candidate, reference)).toBeLessThan(0.95)
  })
})

describe('ΔE00 리포트', () => {
  it('summarizes a uniform difference', () => {
    const candidate = solid(16, 16, [0.5, 0.5, 0.5])
    const reference = solid(16, 16, [0.55, 0.5, 0.5])
    const report = deltaE2000Report(candidate, reference)

    expect(report).not.toBeNull()
    expect(report!.sampleCount).toBe(256)
    expect(report!.median).toBeGreaterThan(0)
    expect(report!.median).toBeCloseTo(report!.max, 6)
  })

  it('measures only the requested ROI', () => {
    const candidate = solid(16, 16, [0.5, 0.5, 0.5])
    const reference = solid(16, 16, [0.5, 0.5, 0.5])
    const report = deltaE2000Report(candidate, reference, {
      x: 4,
      y: 4,
      widthPx: 4,
      heightPx: 4,
    })

    expect(report!.sampleCount).toBe(16)
  })

  it('refuses an ROI that leaves the image', () => {
    const image = solid(16, 16, [0.5, 0.5, 0.5])

    expect(
      deltaE2000Report(image, image, { x: 12, y: 12, widthPx: 8, heightPx: 8 }),
    ).toBeNull()
  })
})

describe('clipping', () => {
  it('counts fully blown and fully crushed pixels', () => {
    expect(clippedFraction(solid(8, 8, [1, 1, 1]))).toBe(1)
    expect(clippedFraction(solid(8, 8, [0, 0, 0]))).toBe(1)
    expect(clippedFraction(solid(8, 8, [0.5, 0.5, 0.5]))).toBe(0)
  })

  it('uses the same any-channel rule for highlights and shadows', () => {
    expect(clippedFraction(solid(8, 8, [1, 0.5, 0.5]))).toBe(1)
    expect(clippedFraction(solid(8, 8, [0, 0.5, 0.5]))).toBe(1)
  })
})

describe('MTF50', () => {
  /** 알려진 흐림을 가진 기울어진 경계를 합성한다. */
  function slantedEdge(widthPx: number, heightPx: number, blurPx: number): RgbImage {
    const data = new Float32Array(widthPx * heightPx * 3)

    for (let y = 0; y < heightPx; y += 1) {
      // 기울기 약 1/16. 완전히 수직이면 표본이 정수 위치에만 쌓여 과표본이 무의미해진다.
      const edgeAt = widthPx / 2 + y / 16

      for (let x = 0; x < widthPx; x += 1) {
        const distance = (x - edgeAt) / Math.max(blurPx, 1e-3)
        const value = 0.5 + 0.5 * Math.tanh(distance)
        const index = (y * widthPx + x) * 3
        data[index] = value
        data[index + 1] = value
        data[index + 2] = value
      }
    }

    return { widthPx, heightPx, data }
  }

  it('measures a sharper edge as a higher MTF50', () => {
    const roi = { x: 0, y: 0, widthPx: 64, heightPx: 64 }
    const sharp = mtf50FromSlantedEdge(slantedEdge(64, 64, 0.6), roi)
    const soft = mtf50FromSlantedEdge(slantedEdge(64, 64, 3), roi)

    expect(sharp).not.toBeNull()
    expect(soft).not.toBeNull()
    expect(sharp!).toBeGreaterThan(soft!)
  })

  it('stays inside the Nyquist range', () => {
    const measured = mtf50FromSlantedEdge(slantedEdge(64, 64, 1), {
      x: 0,
      y: 0,
      widthPx: 64,
      heightPx: 64,
    })

    expect(measured).not.toBeNull()
    expect(measured!).toBeGreaterThan(0)
    expect(measured!).toBeLessThanOrEqual(0.5)
  })

  it('refuses a flat region instead of inventing a number', () => {
    // 자연 사진에서 경계를 자동으로 찾으려 하면 조용히 엉뚱한 값이 evidence에 남는다.
    expect(
      mtf50FromSlantedEdge(solid(64, 64, [0.5, 0.5, 0.5]), {
        x: 0,
        y: 0,
        widthPx: 64,
        heightPx: 64,
      }),
    ).toBeNull()
  })
})

describe('parity 측정', () => {
  it('reports a perfect match without claiming a verdict', () => {
    const image = solid(32, 32, [0.45, 0.5, 0.55])
    const measurement = measureParity(image, image)

    expect(measurement).not.toBeNull()
    expect(measurement!.ssim).toBeCloseTo(1, 10)
    expect(measurement!.deltaE!.median).toBeCloseTo(0, 10)
    expect(measurement!.clippingIncreasePercentagePoints).toBeCloseTo(0, 10)
    expect(measurement!.upscaled).toBe(false)
    // 측정 결과에 pass/fail 필드가 없어야 한다. 판정은 evidence 도구의 몫이다.
    expect(Object.keys(measurement!)).not.toContain('passed')
  })

  it('refuses a mismatched pair instead of counting null core metrics', () => {
    const measurement = measureParity(
      solid(64, 64, [0.5, 0.5, 0.5]),
      solid(32, 32, [0.5, 0.5, 0.5]),
    )

    expect(measurement).toBeNull()
  })
})
