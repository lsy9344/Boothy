/**
 * Story 7.5. darktable XMP의 `darktable:params` hex 문자열을 읽는다.
 *
 * darktable은 모듈 파라미터 구조체를 **little-endian 바이트 그대로** XMP에 적는다.
 * 그래서 이 파일이 하는 일은 해석이 아니라 **복원**이다 — 값을 추측하지 않는다.
 *
 * 길이가 맞지 않거나 모르는 모듈이면 `null`을 돌려준다. 호출자는 그 recipe 전체를
 * 상주 경로에서 제외하고 정확 darktable 경로로 내려보낸다. **부분 적용은 하지 않는다** —
 * 일부만 적용한 화면은 빠른 화면이 아니라 틀린 화면이다.
 */

/** hex 문자열을 바이트로 되돌린다. 홀수 길이나 hex가 아닌 문자는 `null`이다. */
export function decodeParamBytes(paramsHex: string): Uint8Array | null {
  const trimmed = paramsHex.trim()

  if (trimmed.length === 0 || trimmed.length % 2 !== 0) {
    return null
  }

  if (!/^[0-9a-fA-F]+$/.test(trimmed)) {
    return null
  }

  const bytes = new Uint8Array(trimmed.length / 2)

  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(trimmed.slice(index * 2, index * 2 + 2), 16)
  }

  return bytes
}

/** 바이트를 little-endian f32 배열로 읽는다. */
export function readFloats(bytes: Uint8Array): number[] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const floats: number[] = []

  for (let offset = 0; offset + 4 <= bytes.byteLength; offset += 4) {
    floats.push(view.getFloat32(offset, true))
  }

  return floats
}

/** 바이트를 little-endian i32 하나로 읽는다. */
export function readInt(bytes: Uint8Array, wordIndex: number): number | null {
  const offset = wordIndex * 4

  if (offset + 4 > bytes.byteLength) {
    return null
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)

  return view.getInt32(offset, true)
}

/** `temperature` (modversion 4). 화이트밸런스 채널 계수. */
export type TemperatureParams = {
  readonly kind: 'temperature'
  readonly red: number
  readonly green: number
  readonly blue: number
  readonly g2: number
  readonly preset: number
}

/** `exposure` (modversion 7). */
export type ExposureParams = {
  readonly kind: 'exposure'
  /** 0 = manual, 1 = deflicker. deflicker는 상주 엔진이 구현하지 않는다. */
  readonly mode: number
  readonly black: number
  readonly exposureEv: number
  readonly compensateExposureBias: boolean
}

/** `sigmoid` (modversion 3). darktable의 표시용 톤 매퍼. */
export type SigmoidParams = {
  readonly kind: 'sigmoid'
  readonly middleGreyContrast: number
  readonly contrastSkewness: number
  readonly displayWhiteTarget: number
  readonly displayBlackTarget: number
}

/** `bloom` (modversion 1). */
export type BloomParams = {
  readonly kind: 'bloom'
  readonly size: number
  readonly threshold: number
  readonly strength: number
}

/** `sharpen` (modversion 1). unsharp mask. */
export type SharpenParams = {
  readonly kind: 'sharpen'
  readonly radius: number
  readonly amount: number
  readonly threshold: number
}

/** `monochrome` (modversion 2). Lab a/b 평면의 가우시안 필터. */
export type MonochromeParams = {
  readonly kind: 'monochrome'
  readonly a: number
  readonly b: number
  readonly size: number
  readonly highlights: number
}

export type DarktableOperationParams =
  | TemperatureParams
  | ExposureParams
  | SigmoidParams
  | BloomParams
  | SharpenParams
  | MonochromeParams

/**
 * 모듈 이름 + modversion + params hex를 타입이 있는 파라미터로 되돌린다.
 *
 * **modversion이 다르면 `null`이다.** darktable이 구조체를 바꿨는데 같은 바이트로 읽으면
 * 조용히 다른 화면이 나온다.
 */
export function decodeOperationParams(
  operation: string,
  modVersion: number,
  paramsHex: string,
): DarktableOperationParams | null {
  const bytes = decodeParamBytes(paramsHex)

  if (bytes === null) {
    return null
  }

  const floats = readFloats(bytes)

  switch (operation) {
    case 'temperature': {
      if (modVersion !== 4 || bytes.byteLength !== 20) {
        return null
      }

      const preset = readInt(bytes, 4)

      if (preset === null) {
        return null
      }

      return {
        kind: 'temperature',
        red: floats[0],
        green: floats[1],
        blue: floats[2],
        g2: floats[3],
        preset,
      }
    }

    case 'exposure': {
      if (modVersion !== 7 || bytes.byteLength !== 28) {
        return null
      }

      const mode = readInt(bytes, 0)
      const compensate = readInt(bytes, 6)

      if (mode === null || compensate === null) {
        return null
      }

      return {
        kind: 'exposure',
        mode,
        black: floats[1],
        exposureEv: floats[2],
        compensateExposureBias: compensate !== 0,
      }
    }

    case 'sigmoid': {
      if (modVersion !== 3 || bytes.byteLength !== 56) {
        return null
      }

      return {
        kind: 'sigmoid',
        middleGreyContrast: floats[0],
        contrastSkewness: floats[1],
        displayWhiteTarget: floats[2],
        displayBlackTarget: floats[3],
      }
    }

    case 'bloom': {
      if (modVersion !== 1 || bytes.byteLength !== 12) {
        return null
      }

      return {
        kind: 'bloom',
        size: floats[0],
        threshold: floats[1],
        strength: floats[2],
      }
    }

    case 'sharpen': {
      if (modVersion !== 1 || bytes.byteLength !== 12) {
        return null
      }

      return {
        kind: 'sharpen',
        radius: floats[0],
        amount: floats[1],
        threshold: floats[2],
      }
    }

    case 'monochrome': {
      if (modVersion !== 2 || bytes.byteLength !== 16) {
        return null
      }

      return {
        kind: 'monochrome',
        a: floats[0],
        b: floats[1],
        size: floats[2],
        highlights: floats[3],
      }
    }

    default:
      return null
  }
}
