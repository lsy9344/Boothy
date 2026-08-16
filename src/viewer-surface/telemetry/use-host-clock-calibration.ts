import { useEffect, useState } from 'react'

import {
  CLOCK_RECALIBRATION_INTERVAL_MS,
  calibrateClock,
  type ClockCalibration,
} from './present-clock'
import {
  createDisplayHostService,
  type DisplayHostService,
} from '../services/viewer-host-adapter'

/**
 * viewer document의 `performance.now()`를 host monotonic clock에 맞춘다.
 *
 * 재보정 시점: viewer epoch 변경(창 재생성), 마운트(reload), 그리고 주기적 갱신.
 * 보정에 실패하면 `null`을 유지한다 — 임의의 offset을 지어내면 계측 전체가 거짓이 된다.
 *
 * **부팅을 막지 않는다.** 관람 창은 host IPC를 기다리지 않고 렌더링해야 한다
 * (Story 7.1 HV-13A 흰 화면 보정). 그래서 이 훅은 렌더 이후 effect에서만 동작한다.
 */
export function useHostClockCalibration({
  viewerEpoch,
  hostService,
}: {
  viewerEpoch: number
  hostService?: DisplayHostService
}) {
  const [calibrationState, setCalibrationState] = useState<{
    viewerEpoch: number
    calibration: ClockCalibration | null
  } | null>(null)

  useEffect(() => {
    const service = hostService ?? createDisplayHostService()
    let isDisposed = false

    const run = () => {
      void calibrateClock({
        probe: (input) => service.stampClockProbe(input),
      })
        .then((next) => {
          if (!isDisposed) {
            setCalibrationState({ viewerEpoch, calibration: next })
          }
        })
        .catch(() => undefined)
    }

    run()
    const intervalId = globalThis.setInterval(
      run,
      CLOCK_RECALIBRATION_INTERVAL_MS,
    )

    return () => {
      isDisposed = true
      globalThis.clearInterval(intervalId)
    }
  }, [viewerEpoch, hostService])

  return calibrationState?.viewerEpoch === viewerEpoch
    ? calibrationState.calibration
    : null
}
