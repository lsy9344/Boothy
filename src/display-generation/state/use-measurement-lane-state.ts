import { useEffect, useState } from 'react'

import {
  createDisplayHostService,
  type DisplayHostService,
} from '../../viewer-surface/services/viewer-host-adapter'

/**
 * booth surface가 계측 lane 활성 여부만 읽는다.
 *
 * 이 값이 `false`면 booth는 present 계측 IPC를 **하나도** 수행하지 않는다.
 * 기본값이 `false`이므로 제품 경로는 Story 7.1과 동일하게 남는다.
 *
 * booth는 display pointer를 소비하지 않는다. 관람 화면의 자산은 고객 모니터에만 표시된다.
 */
export function useMeasurementLaneState(hostService?: DisplayHostService) {
  const [isMeasurementLaneEnabled, setIsMeasurementLaneEnabled] =
    useState(false)

  useEffect(() => {
    const service = hostService ?? createDisplayHostService()
    let isDisposed = false

    void service
      .getViewerDisplayState()
      .then((pointer) => {
        if (!isDisposed) {
          setIsMeasurementLaneEnabled(pointer.measurementLaneEnabled)
        }
      })
      .catch(() => undefined)

    return () => {
      isDisposed = true
    }
  }, [hostService])

  return isMeasurementLaneEnabled
}
