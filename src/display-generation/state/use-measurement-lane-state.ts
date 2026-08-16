import { useEffect, useState } from 'react'

import {
  createDisplayHostService,
  type DisplayHostService,
} from '../../viewer-surface/services/viewer-host-adapter'
import type { DisplayPointerSnapshot } from '../../shared-contracts'

export const MEASUREMENT_LANE_RETRY_INTERVAL_MS = 1_000

/**
 * booth surface가 **present 계측을 해야 하는지**만 읽는다.
 *
 * 이 값이 `false`면 booth는 present 계측 IPC를 **하나도** 수행하지 않는다.
 * 기본값이 `false`이므로 제품 경로는 Story 7.1과 동일하게 남는다.
 *
 * Story 7.4 주의: 근거는 `measurementLaneEnabled`가 아니라 `presentTelemetryEnabled`다.
 * 앞의 값은 "표시 중인 이미지가 fixture인가"만 답하므로, proxy lane만 켠 회차에서는
 * `false`가 되어 **trusted input이 한 건도 기록되지 않는다.**
 *
 * booth는 display pointer를 소비하지 않는다. 관람 화면의 자산은 고객 모니터에만 표시된다.
 */
export function useMeasurementLaneState(hostService?: DisplayHostService) {
  const [isPresentTelemetryEnabled, setIsPresentTelemetryEnabled] =
    useState(false)

  useEffect(() => {
    const service = hostService ?? createDisplayHostService()
    let isDisposed = false
    let stopListening: (() => void) | undefined
    let snapshotRetryId: ReturnType<typeof setTimeout> | undefined
    let subscriptionRetryId: ReturnType<typeof setTimeout> | undefined
    let observationVersion = 0
    let latestPointer: DisplayPointerSnapshot | null = null

    const applyPointer = (
      pointer: Awaited<ReturnType<DisplayHostService['getViewerDisplayState']>>,
    ) => {
      if (latestPointer !== null && pointer.revision < latestPointer.revision) {
        return
      }

      latestPointer = pointer
      observationVersion += 1
      if (!isDisposed) {
        setIsPresentTelemetryEnabled(pointer.presentTelemetryEnabled)
      }
    }

    const readSnapshot = () => {
      const readVersion = ++observationVersion
      void service
        .getViewerDisplayState()
        .then((pointer) => {
          if (!isDisposed && readVersion === observationVersion) {
            applyPointer(pointer)
          }
        })
        .catch(() => {
          if (!isDisposed && readVersion === observationVersion) {
            snapshotRetryId = setTimeout(
              readSnapshot,
              MEASUREMENT_LANE_RETRY_INTERVAL_MS,
            )
          }
        })
    }

    const subscribe = () => {
      void service
        .subscribeToDisplayUpdates({ onPointer: applyPointer })
        .then((unlisten) => {
          if (isDisposed) {
            unlisten()
            return
          }

          stopListening = unlisten
          readSnapshot()
        })
        .catch(() => {
          if (!isDisposed) {
            subscriptionRetryId = setTimeout(
              subscribe,
              MEASUREMENT_LANE_RETRY_INTERVAL_MS,
            )
          }
        })
    }

    readSnapshot()
    subscribe()

    return () => {
      isDisposed = true
      stopListening?.()
      if (snapshotRetryId !== undefined) {
        clearTimeout(snapshotRetryId)
      }
      if (subscriptionRetryId !== undefined) {
        clearTimeout(subscriptionRetryId)
      }
    }
  }, [hostService])

  return isPresentTelemetryEnabled
}
