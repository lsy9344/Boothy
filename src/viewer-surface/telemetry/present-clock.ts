import type { ClockProbeResult } from '../../shared-contracts'

/**
 * one-clock 계측의 클라이언트 절반.
 *
 * booth WebView와 viewer WebView는 **서로 다른 time origin**을 가지므로 두 문서의
 * `performance.now()`를 그대로 비교할 수 없다. `performance.timeOrigin + now()`는 wall-clock
 * 기반이라 시스템 시간 보정에 취약하므로 쓰지 않는다.
 *
 * 각 문서는 host monotonic clock에 자기를 보정하고, **보정 오차를 함께 보고한다.**
 * KPI는 항상 `present 상한 − input 하한`인 보수적 구간으로 계산한다.
 */

/** 이 값을 넘는 총 불확실도는 `low-confidence`로 표시한다. 버리지 않는다. */
export const PRESENT_UNCERTAINTY_BUDGET_MICROS = 5_000

/** 기본 왕복 횟수. 최소 RTT 표본만 채택하므로 몇 번은 필요하다. */
export const DEFAULT_CALIBRATION_ROUNDS = 9

/** 재보정 주기. viewer epoch 변경과 reload에서도 별도로 재보정한다. */
export const CLOCK_RECALIBRATION_INTERVAL_MS = 5 * 60 * 1000

export type ClockSample = {
  clientSentMicros: number
  clientReceivedMicros: number
  hostMonotonicMicros: number
}

export type ClockCalibration = {
  /** `host = client + offset` */
  offsetMicros: number
  /** 왕복의 절반. 보정값이 틀릴 수 있는 최대 폭이다. */
  uncertaintyMicros: number
  roundTripMicros: number
}

/** IPC DTO는 정수 microseconds만 허용한다. 종료점 불확도는 절대 낮추지 않는다. */
export function toIntegerClockFields(calibration: ClockCalibration) {
  return {
    clockOffsetMicros: Math.round(calibration.offsetMicros),
    clockUncertaintyMicros: Math.ceil(calibration.uncertaintyMicros),
  }
}

/** 시작점 변환. 내림이라 시작이 앞으로만 밀린다 → 구간이 짧아지지 않는다. */
export function floorMicros(milliseconds: number) {
  return Math.floor(milliseconds * 1000)
}

/** 종료점 변환. 올림이라 끝이 뒤로만 밀린다 → 구간이 짧아지지 않는다. */
export function ceilMicros(milliseconds: number) {
  return Math.ceil(milliseconds * 1000)
}

/**
 * Cristian 방식. 왕복이 대칭이라고 가정하고 중간값으로 offset을 추정하되,
 * **가장 빠른 왕복 표본만 채택**한다. 느린 왕복은 비대칭 오차가 크다.
 */
export function estimateCalibration(
  samples: readonly ClockSample[],
): ClockCalibration | null {
  let best: ClockCalibration | null = null

  for (const sample of samples) {
    const roundTripMicros = sample.clientReceivedMicros - sample.clientSentMicros

    if (!Number.isFinite(roundTripMicros) || roundTripMicros < 0) {
      continue
    }

    const midpoint =
      (sample.clientSentMicros + sample.clientReceivedMicros) / 2
    const candidate: ClockCalibration = {
      offsetMicros: sample.hostMonotonicMicros - midpoint,
      uncertaintyMicros: roundTripMicros / 2,
      roundTripMicros,
    }

    if (best === null || candidate.roundTripMicros < best.roundTripMicros) {
      best = candidate
    }
  }

  return best
}

export function toHostMicros(clientMicros: number, offsetMicros: number) {
  return Math.max(0, Math.round(clientMicros + offsetMicros))
}

/**
 * 공식 KPI. 항상 가장 넓은 구간을 돌려준다.
 *
 * 측정 오차가 결과를 실제보다 빠르게 보이게 만들 수 없어야 한다.
 */
export function conservativeLatencyMicros({
  inputHostMicros,
  presentHostMicros,
  inputUncertaintyMicros,
  presentUncertaintyMicros,
}: {
  inputHostMicros: number | null
  presentHostMicros: number | null
  inputUncertaintyMicros: number
  presentUncertaintyMicros: number
}) {
  if (inputHostMicros === null || presentHostMicros === null) {
    return null
  }

  const lowerInput = inputHostMicros - Math.max(0, inputUncertaintyMicros)
  const upperPresent = presentHostMicros + Math.max(0, presentUncertaintyMicros)

  return Math.max(0, Math.ceil(upperPresent - lowerInput))
}

export function classifyConfidence(totalUncertaintyMicros: number) {
  return totalUncertaintyMicros > PRESENT_UNCERTAINTY_BUDGET_MICROS
    ? 'low-confidence'
    : 'measured'
}

/**
 * host clock에 대한 보정을 수행한다.
 *
 * 모든 왕복이 실패하면 **null을 돌려준다.** 임의의 offset을 지어내면 계측 전체가 거짓이 된다.
 */
export async function calibrateClock({
  probe,
  now = () => floorMicros(performance.now()),
  rounds = DEFAULT_CALIBRATION_ROUNDS,
}: {
  probe(input: { clientSentMicros: number }): Promise<ClockProbeResult>
  now?: () => number
  rounds?: number
}): Promise<ClockCalibration | null> {
  const samples: ClockSample[] = []

  for (let round = 0; round < rounds; round += 1) {
    const clientSentMicros = now()

    try {
      const result = await probe({ clientSentMicros })

      samples.push({
        clientSentMicros,
        clientReceivedMicros: now(),
        hostMonotonicMicros: result.hostMonotonicMicros,
      })
    } catch {
      // 개별 왕복 실패는 무시하고 나머지 표본으로 추정한다.
    }
  }

  return estimateCalibration(samples)
}
