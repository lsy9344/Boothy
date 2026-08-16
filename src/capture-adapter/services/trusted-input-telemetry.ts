import { invoke } from '@tauri-apps/api/core'

import type { ClockProbeResult, TrustedInputReport } from '../../shared-contracts'
import {
  clockProbeResultSchema,
  trustedInputReportSchema,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'
import {
  CLOCK_RECALIBRATION_INTERVAL_MS,
  calibrateClock,
  floorMicros,
  type ClockCalibration,
} from '../../viewer-surface/telemetry/present-clock'

/**
 * booth 측 KPI 시작점 계측.
 *
 * 공식 시작은 **촬영 컨트롤의 trusted `pointerup`**이다. host가 IPC를 수신한 시각을 쓰면
 * 전송 지연이 시작점 뒤로 밀려 KPI가 실제보다 빠르게 보인다.
 *
 * booth WebView의 `performance.now()`는 viewer WebView와 time origin이 다르므로,
 * host monotonic clock에 대한 보정값을 함께 보낸다. **보정이 없으면 보고하지 않는다** —
 * 잘못된 offset으로 만든 표본은 없느니만 못하다.
 */
export type TrustedInputStamp = {
  requestId: string
  clientInputMicros: number
  isTrusted: boolean
}

type CalibrationCache = {
  calibration: ClockCalibration | null
  calibratedAtMs: number
  inFlight: Promise<ClockCalibration | null> | null
}

const cache: CalibrationCache = {
  calibration: null,
  calibratedAtMs: 0,
  inFlight: null,
}

async function stampClockProbe(): Promise<ClockProbeResult> {
  return clockProbeResultSchema.parse(await invoke('stamp_clock_probe'))
}

function isCalibrationFresh(nowMs: number) {
  return (
    cache.calibration !== null &&
    nowMs - cache.calibratedAtMs < CLOCK_RECALIBRATION_INTERVAL_MS
  )
}

/**
 * booth document의 clock 보정을 준비한다.
 *
 * 첫 촬영이 보정 비용을 물지 않도록 화면 진입 시 미리 호출한다.
 */
export async function ensureBoothClockCalibration({
  probe = stampClockProbe,
  nowMs = () => Date.now(),
}: {
  probe?: typeof stampClockProbe
  nowMs?: () => number
} = {}) {
  if (isCalibrationFresh(nowMs())) {
    return cache.calibration
  }

  if (cache.inFlight !== null) {
    return cache.inFlight
  }

  cache.inFlight = calibrateClock({ probe })
    .then((calibration) => {
      if (calibration !== null) {
        cache.calibration = calibration
        cache.calibratedAtMs = nowMs()
      }

      return cache.calibration
    })
    .catch(() => cache.calibration)
    .finally(() => {
      cache.inFlight = null
    })

  return cache.inFlight
}

export function resetBoothClockCalibrationForTests() {
  cache.calibration = null
  cache.calibratedAtMs = 0
  cache.inFlight = null
}

/**
 * `pointerup` 핸들러 **첫 줄**에서 호출한다. 동기 함수이며 어떤 IPC도 하지 않는다.
 *
 * 합성 이벤트(`isTrusted === false`)는 qualifying 표본이 아니므로 그대로 표시해 둔다.
 */
export function stampTrustedCaptureInput(
  event: { isTrusted: boolean },
  requestId: string,
  now: () => number = () => performance.now(),
): TrustedInputStamp {
  return {
    requestId,
    // 시작점은 내림으로 변환한다. 어떤 반올림도 구간을 짧게 만들면 안 된다.
    clientInputMicros: floorMicros(now()),
    isTrusted: event.isTrusted,
  }
}

/**
 * 계측 보고. **절대 `await`하지 않는다** — 이 IPC가 촬영 요청을 지연시키면 제품이 느려진다.
 *
 * 보정이 없으면 조용히 건너뛰되 콘솔에 남긴다. 표본을 잃는 편이 거짓 표본보다 낫다.
 */
export function reportTrustedCaptureInput({
  sessionId,
  stamp,
  isMeasurementLaneEnabled,
  send = async (report: TrustedInputReport) => {
    await invoke('report_trusted_capture_input', {
      input: trustedInputReportSchema.parse(report),
    })
  },
  ensureCalibration = ensureBoothClockCalibration,
}: {
  sessionId: string
  stamp: TrustedInputStamp
  isMeasurementLaneEnabled: boolean
  send?: (report: TrustedInputReport) => Promise<void>
  ensureCalibration?: typeof ensureBoothClockCalibration
}) {
  // 계측 lane이 꺼져 있으면 제품 hot path에 IPC를 하나도 추가하지 않는다.
  if (!isMeasurementLaneEnabled || !isTauriRuntime()) {
    return
  }

  void ensureCalibration()
    .then((calibration) => {
      if (calibration === null) {
        if (typeof console !== 'undefined') {
          console.info(
            '[boothy][present] trusted-input 표본을 건너뜁니다: host clock 보정 없음',
          )
        }

        return
      }

      return send({
        sessionId,
        requestId: stamp.requestId,
        clientInputMicros: stamp.clientInputMicros,
        clockOffsetMicros: Math.round(calibration.offsetMicros),
        clockUncertaintyMicros: Math.ceil(calibration.uncertaintyMicros),
        isTrusted: stamp.isTrusted,
      })
    })
    .catch(() => undefined)
}
