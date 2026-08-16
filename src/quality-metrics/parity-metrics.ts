/**
 * Story 7.5 T6. 두 렌더 결과의 시각 동등성을 재는 지표.
 *
 * **이 파일은 판정하지 않는다. 측정만 한다.** 합격선은 story가 정하고, 판정은
 * evidence 도구가 한다. 지표 계산과 판정을 한 곳에 두면 "통과하도록" 지표를
 * 손보고 싶은 유혹이 생긴다.
 *
 * 외부 의존성을 쓰지 않는다. 입력은 darktable이 직접 내보낼 수 있고 Windows에서도
 * 만들기 쉬운 PPM(P6)이며, 8비트와 16비트를 모두 읽는다.
 */

export type RgbImage = {
  readonly widthPx: number
  readonly heightPx: number
  /** 0..1 정규화된 sRGB 값. 채널 3개 interleaved. */
  readonly data: Float32Array
}

const MAX_PARITY_SAMPLE_COUNT = 64_000_000

/** PPM(P6) 바이트를 읽는다. 헤더가 이상하면 예외 대신 `null`이다. */
export function decodePpm(bytes: Uint8Array): RgbImage | null {
  let cursor = 0

  const readToken = (): string | null => {
    while (cursor < bytes.length) {
      const byte = bytes[cursor]

      if (byte === 0x23) {
        // 주석은 줄 끝까지 건너뛴다.
        while (cursor < bytes.length && bytes[cursor] !== 0x0a) {
          cursor += 1
        }
        continue
      }

      if (byte === 0x20 || byte === 0x09 || byte === 0x0a || byte === 0x0d) {
        cursor += 1
        continue
      }

      break
    }

    if (cursor >= bytes.length) {
      return null
    }

    const start = cursor

    while (cursor < bytes.length) {
      const byte = bytes[cursor]

      if (byte === 0x20 || byte === 0x09 || byte === 0x0a || byte === 0x0d) {
        break
      }

      cursor += 1
    }

    return String.fromCharCode(...bytes.subarray(start, cursor))
  }

  if (readToken() !== 'P6') {
    return null
  }

  const widthPx = Number(readToken())
  const heightPx = Number(readToken())
  const maxValue = Number(readToken())

  if (
    !Number.isInteger(widthPx) ||
    !Number.isInteger(heightPx) ||
    widthPx <= 0 ||
    heightPx <= 0 ||
    !(maxValue === 255 || maxValue === 65535)
  ) {
    return null
  }

  // 헤더 뒤 separator를 소비한다. Windows가 만든 CRLF는 한 separator로 취급하되,
  // 그 뒤 픽셀 값이 공백 byte일 수 있으므로 임의 길이 whitespace는 건너뛰지 않는다.
  if (bytes[cursor] === 0x0d && bytes[cursor + 1] === 0x0a) {
    cursor += 2
  } else {
    cursor += 1
  }

  const sampleCount = widthPx * heightPx * 3

  if (
    !Number.isSafeInteger(sampleCount) ||
    sampleCount <= 0 ||
    sampleCount > MAX_PARITY_SAMPLE_COUNT
  ) {
    return null
  }

  const data = new Float32Array(sampleCount)

  if (maxValue === 255) {
    if (bytes.length - cursor < sampleCount) {
      return null
    }

    for (let index = 0; index < sampleCount; index += 1) {
      data[index] = bytes[cursor + index] / 255
    }
  } else {
    if (bytes.length - cursor < sampleCount * 2) {
      return null
    }

    for (let index = 0; index < sampleCount; index += 1) {
      const offset = cursor + index * 2
      // PPM의 16비트 표본은 big-endian이다.
      data[index] = ((bytes[offset] << 8) | bytes[offset + 1]) / 65535
    }
  }

  return { widthPx, heightPx, data }
}

export function srgbToLinear(value: number): number {
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
}

export type Lab = { readonly l: number; readonly a: number; readonly b: number }

/** sRGB(0..1) → CIE Lab, D65 백색점. */
export function srgbToLab(r: number, g: number, b: number): Lab {
  const lr = srgbToLinear(r)
  const lg = srgbToLinear(g)
  const lb = srgbToLinear(b)

  const x = (0.4124564 * lr + 0.3575761 * lg + 0.1804375 * lb) / 0.95047
  const y = 0.2126729 * lr + 0.7151522 * lg + 0.072175 * lb
  const z = (0.0193339 * lr + 0.119192 * lg + 0.9503041 * lb) / 1.08883

  const f = (t: number) =>
    t > 216 / 24389 ? Math.cbrt(t) : (841 / 108) * t + 4 / 29

  const fx = f(x)
  const fy = f(y)
  const fz = f(z)

  return { l: 116 * fy - 16, a: 500 * (fx - fy), b: 200 * (fy - fz) }
}

/**
 * CIEDE2000 색차.
 *
 * Sharma·Wu·Dalal(2005)의 공개 검증 표로 단위 테스트한다. 색차 공식은 손으로 옮기다
 * 부호 하나만 틀려도 그럴듯한 값을 계속 내놓기 때문에, 검증 표 없이는 신뢰할 수 없다.
 */
export function deltaE2000(first: Lab, second: Lab): number {
  const kL = 1
  const kC = 1
  const kH = 1

  const c1 = Math.hypot(first.a, first.b)
  const c2 = Math.hypot(second.a, second.b)
  const cBar = (c1 + c2) / 2
  const cBar7 = cBar ** 7
  const g = 0.5 * (1 - Math.sqrt(cBar7 / (cBar7 + 25 ** 7)))

  const a1p = (1 + g) * first.a
  const a2p = (1 + g) * second.a
  const c1p = Math.hypot(a1p, first.b)
  const c2p = Math.hypot(a2p, second.b)

  const hp = (aPrime: number, bValue: number) => {
    if (aPrime === 0 && bValue === 0) {
      return 0
    }

    const angle = (Math.atan2(bValue, aPrime) * 180) / Math.PI

    return angle >= 0 ? angle : angle + 360
  }

  const h1p = hp(a1p, first.b)
  const h2p = hp(a2p, second.b)

  const deltaLp = second.l - first.l
  const deltaCp = c2p - c1p

  let deltahp = 0

  if (c1p * c2p !== 0) {
    const diff = h2p - h1p

    if (Math.abs(diff) <= 180) {
      deltahp = diff
    } else if (diff > 180) {
      deltahp = diff - 360
    } else {
      deltahp = diff + 360
    }
  }

  const deltaHp =
    2 * Math.sqrt(c1p * c2p) * Math.sin(((deltahp / 2) * Math.PI) / 180)

  const lBarP = (first.l + second.l) / 2
  const cBarP = (c1p + c2p) / 2

  let hBarP = h1p + h2p

  if (c1p * c2p !== 0) {
    const diff = Math.abs(h1p - h2p)

    if (diff <= 180) {
      hBarP = (h1p + h2p) / 2
    } else if (h1p + h2p < 360) {
      hBarP = (h1p + h2p + 360) / 2
    } else {
      hBarP = (h1p + h2p - 360) / 2
    }
  }

  const t =
    1 -
    0.17 * Math.cos(((hBarP - 30) * Math.PI) / 180) +
    0.24 * Math.cos(((2 * hBarP) * Math.PI) / 180) +
    0.32 * Math.cos(((3 * hBarP + 6) * Math.PI) / 180) -
    0.2 * Math.cos(((4 * hBarP - 63) * Math.PI) / 180)

  const deltaTheta = 30 * Math.exp(-(((hBarP - 275) / 25) ** 2))
  const cBarP7 = cBarP ** 7
  const rC = 2 * Math.sqrt(cBarP7 / (cBarP7 + 25 ** 7))
  const rT = -rC * Math.sin(((2 * deltaTheta) * Math.PI) / 180)

  const lBarPMinus50Squared = (lBarP - 50) ** 2
  const sL =
    1 + (0.015 * lBarPMinus50Squared) / Math.sqrt(20 + lBarPMinus50Squared)
  const sC = 1 + 0.045 * cBarP
  const sH = 1 + 0.015 * cBarP * t

  const termL = deltaLp / (kL * sL)
  const termC = deltaCp / (kC * sC)
  const termH = deltaHp / (kH * sH)

  return Math.sqrt(
    termL * termL + termC * termC + termH * termH + rT * termC * termH,
  )
}

export type DeltaEReport = {
  readonly sampleCount: number
  readonly median: number
  readonly p95: number
  readonly max: number
}

function summarize(values: Float64Array): DeltaEReport {
  const sorted = Float64Array.from(values).sort()
  const n = sorted.length

  if (n === 0) {
    return { sampleCount: 0, median: 0, p95: 0, max: 0 }
  }

  const at = (fraction: number) =>
    sorted[Math.min(n - 1, Math.max(0, Math.round(fraction * (n - 1))))]

  return { sampleCount: n, median: at(0.5), p95: at(0.95), max: sorted[n - 1] }
}

export type Roi = {
  readonly x: number
  readonly y: number
  readonly widthPx: number
  readonly heightPx: number
}

/** 두 이미지의 픽셀별 ΔE00 분포. ROI를 주면 그 영역만 잰다. */
export function deltaE2000Report(
  candidate: RgbImage,
  reference: RgbImage,
  roi?: Roi,
): DeltaEReport | null {
  if (
    candidate.widthPx !== reference.widthPx ||
    candidate.heightPx !== reference.heightPx
  ) {
    return null
  }

  const region: Roi = roi ?? {
    x: 0,
    y: 0,
    widthPx: candidate.widthPx,
    heightPx: candidate.heightPx,
  }

  if (
    region.x < 0 ||
    region.y < 0 ||
    region.widthPx <= 0 ||
    region.heightPx <= 0 ||
    region.x + region.widthPx > candidate.widthPx ||
    region.y + region.heightPx > candidate.heightPx
  ) {
    return null
  }

  const values = new Float64Array(region.widthPx * region.heightPx)
  let cursor = 0

  for (let y = region.y; y < region.y + region.heightPx; y += 1) {
    for (let x = region.x; x < region.x + region.widthPx; x += 1) {
      const index = (y * candidate.widthPx + x) * 3
      values[cursor] = deltaE2000(
        srgbToLab(
          candidate.data[index],
          candidate.data[index + 1],
          candidate.data[index + 2],
        ),
        srgbToLab(
          reference.data[index],
          reference.data[index + 1],
          reference.data[index + 2],
        ),
      )
      cursor += 1
    }
  }

  return summarize(values)
}

/** Rec.709 luma. SSIM은 밝기 채널에서 잰다. */
export function toLuma(image: RgbImage): Float32Array {
  const luma = new Float32Array(image.widthPx * image.heightPx)

  for (let index = 0; index < luma.length; index += 1) {
    const offset = index * 3
    luma[index] =
      0.2126 * image.data[offset] +
      0.7152 * image.data[offset + 1] +
      0.0722 * image.data[offset + 2]
  }

  return luma
}

/**
 * 8×8 블록 SSIM의 픽셀 가중 평균. 표준 상수(K1=0.01, K2=0.03, L=1)를 쓴다.
 *
 * 가우시안 창 대신 블록을 쓰는 것은 의존성 없이 재현 가능하게 하기 위해서다.
 * 두 방식의 값이 조금 다르므로 evidence에는 이 정의를 함께 적는다.
 */
export function ssim(candidate: RgbImage, reference: RgbImage): number | null {
  if (
    candidate.widthPx !== reference.widthPx ||
    candidate.heightPx !== reference.heightPx
  ) {
    return null
  }

  const block = 8
  const c1 = 0.01 ** 2
  const c2 = 0.03 ** 2
  const left = toLuma(candidate)
  const right = toLuma(reference)

  let total = 0
  let sampleCount = 0

  for (let blockY = 0; blockY < candidate.heightPx; blockY += block) {
    for (let blockX = 0; blockX < candidate.widthPx; blockX += block) {
      const blockWidth = Math.min(block, candidate.widthPx - blockX)
      const blockHeight = Math.min(block, candidate.heightPx - blockY)
      let sumL = 0
      let sumR = 0
      let sumLL = 0
      let sumRR = 0
      let sumLR = 0

      for (let y = 0; y < blockHeight; y += 1) {
        for (let x = 0; x < blockWidth; x += 1) {
          const index = (blockY + y) * candidate.widthPx + (blockX + x)
          const l = left[index]
          const r = right[index]

          sumL += l
          sumR += r
          sumLL += l * l
          sumRR += r * r
          sumLR += l * r
        }
      }

      const count = blockWidth * blockHeight
      const meanL = sumL / count
      const meanR = sumR / count
      const varL = sumLL / count - meanL * meanL
      const varR = sumRR / count - meanR * meanR
      const covariance = sumLR / count - meanL * meanR

      const blockSsim =
        ((2 * meanL * meanR + c1) * (2 * covariance + c2)) /
        ((meanL * meanL + meanR * meanR + c1) * (varL + varR + c2))
      total += blockSsim * count
      sampleCount += count
    }
  }

  return sampleCount === 0 ? null : total / sampleCount
}

/** highlight 또는 shadow에서 채널 하나라도 경계에 닿은 픽셀의 비율(0..1). */
export function clippedFraction(image: RgbImage, epsilon = 1 / 255): number {
  let clipped = 0

  for (let index = 0; index < image.data.length; index += 3) {
    const r = image.data[index]
    const g = image.data[index + 1]
    const b = image.data[index + 2]
    const high = r >= 1 - epsilon || g >= 1 - epsilon || b >= 1 - epsilon
    const low = r <= epsilon || g <= epsilon || b <= epsilon

    if (high || low) {
      clipped += 1
    }
  }

  return clipped / (image.widthPx * image.heightPx)
}

/**
 * 기울어진 경계(slanted edge)에서 MTF50을 잰다. 단위는 cycles/pixel.
 *
 * ROI는 **호출자가 지정한다.** 자연 사진에서 적합한 경계를 자동으로 찾으려 하면
 * 조용히 엉뚱한 영역을 재고 그 숫자가 evidence에 남는다. 적합한 대상이 없으면
 * 이 함수를 부르지 않고 "측정하지 않음"으로 기록하는 것이 맞다.
 *
 * ROI 안에 뚜렷한 수직 경계가 없으면 `null`이다.
 */
export function mtf50FromSlantedEdge(
  image: RgbImage,
  roi: Roi,
): number | null {
  if (
    roi.x < 0 ||
    roi.y < 0 ||
    roi.widthPx < 8 ||
    roi.heightPx < 8 ||
    roi.x + roi.widthPx > image.widthPx ||
    roi.y + roi.heightPx > image.heightPx
  ) {
    return null
  }

  const luma = toLuma(image)

  // 행마다 경계 위치를 무게중심으로 찾고, 그 위치를 기준으로 표본을 모은다.
  const samples: { position: number; value: number }[] = []
  let edgeContrast = 0

  for (let y = 0; y < roi.heightPx; y += 1) {
    const rowStart = (roi.y + y) * image.widthPx + roi.x
    let weightedSum = 0
    let weightSum = 0
    let minimum = Number.POSITIVE_INFINITY
    let maximum = Number.NEGATIVE_INFINITY

    for (let x = 1; x < roi.widthPx; x += 1) {
      const derivative = Math.abs(luma[rowStart + x] - luma[rowStart + x - 1])
      weightedSum += derivative * (x - 0.5)
      weightSum += derivative
    }

    for (let x = 0; x < roi.widthPx; x += 1) {
      minimum = Math.min(minimum, luma[rowStart + x])
      maximum = Math.max(maximum, luma[rowStart + x])
    }

    edgeContrast = Math.max(edgeContrast, maximum - minimum)

    if (weightSum <= 1e-6) {
      continue
    }

    const center = weightedSum / weightSum

    for (let x = 0; x < roi.widthPx; x += 1) {
      samples.push({ position: x - center, value: luma[rowStart + x] })
    }
  }

  // 경계라고 부를 만한 밝기 차이가 없으면 측정하지 않는다.
  if (samples.length === 0 || edgeContrast < 0.1) {
    return null
  }

  // 4배 과표본 ESF → 미분해서 LSF → 이산 푸리에로 MTF.
  const oversample = 4
  const half = Math.floor(roi.widthPx / 2)
  const binCount = half * 2 * oversample
  const sums = new Float64Array(binCount)
  const counts = new Float64Array(binCount)

  for (const sample of samples) {
    const bin = Math.round((sample.position + half) * oversample)

    if (bin >= 0 && bin < binCount) {
      sums[bin] += sample.value
      counts[bin] += 1
    }
  }

  const esf: number[] = []

  for (let bin = 0; bin < binCount; bin += 1) {
    if (counts[bin] > 0) {
      esf.push(sums[bin] / counts[bin])
    } else if (esf.length > 0) {
      esf.push(esf[esf.length - 1])
    }
  }

  if (esf.length < 16) {
    return null
  }

  const lsf: number[] = []

  for (let index = 1; index < esf.length; index += 1) {
    lsf.push(esf[index] - esf[index - 1])
  }

  const dc = lsf.reduce((total, value) => total + Math.abs(value), 0)

  if (dc <= 1e-9) {
    return null
  }

  // 주파수는 cycles/pixel. 과표본 배수만큼 촘촘하게 계산한다.
  const steps = 256
  let previousFrequency = 0
  let previousMtf = 1

  for (let step = 1; step <= steps; step += 1) {
    const frequency = (step / steps) * 0.5
    let real = 0
    let imaginary = 0

    for (let index = 0; index < lsf.length; index += 1) {
      const phase = (2 * Math.PI * frequency * index) / oversample
      real += lsf[index] * Math.cos(phase)
      imaginary += lsf[index] * Math.sin(phase)
    }

    const mtf = Math.hypot(real, imaginary) / dc

    if (mtf <= 0.5) {
      // 0.5를 지나는 지점을 선형 보간한다.
      const span = previousMtf - mtf

      return span <= 1e-9
        ? frequency
        : previousFrequency +
            ((previousMtf - 0.5) / span) * (frequency - previousFrequency)
    }

    previousFrequency = frequency
    previousMtf = mtf
  }

  return null
}

export type ParityMeasurement = {
  readonly widthPx: number
  readonly heightPx: number
  readonly ssim: number | null
  readonly deltaE: DeltaEReport | null
  readonly skinRoiDeltaE: DeltaEReport | null
  readonly candidateClippedFraction: number
  readonly referenceClippedFraction: number
  readonly clippingIncreasePercentagePoints: number
  /** 후보가 참조보다 크면 upscale이다. NFR-003의 zero 항목이다. */
  readonly upscaled: boolean
}

/**
 * 한 쌍을 측정한다. **판정하지 않는다.**
 *
 * `skinRoi`는 호출자가 지정한다. 피부 영역을 자동으로 찾으려 하면 조용히 엉뚱한
 * 영역을 재고, 그 숫자가 통과 근거로 evidence에 남는다.
 */
export function measureParity(
  candidate: RgbImage,
  reference: RgbImage,
  skinRoi?: Roi,
): ParityMeasurement | null {
  if (
    candidate.widthPx !== reference.widthPx ||
    candidate.heightPx !== reference.heightPx
  ) {
    return null
  }

  const candidateClipped = clippedFraction(candidate)
  const referenceClipped = clippedFraction(reference)

  return {
    widthPx: candidate.widthPx,
    heightPx: candidate.heightPx,
    ssim: ssim(candidate, reference),
    deltaE: deltaE2000Report(candidate, reference),
    skinRoiDeltaE:
      skinRoi === undefined
        ? null
        : deltaE2000Report(candidate, reference, skinRoi),
    candidateClippedFraction: candidateClipped,
    referenceClippedFraction: referenceClipped,
    clippingIncreasePercentagePoints:
      (candidateClipped - referenceClipped) * 100,
    upscaled:
      candidate.widthPx > reference.widthPx ||
      candidate.heightPx > reference.heightPx,
  }
}
