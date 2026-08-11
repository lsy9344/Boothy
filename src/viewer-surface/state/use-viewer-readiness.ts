import type { RefObject } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'

import type { ViewerLayoutReport, ViewerReadinessSnapshot } from '../../shared-contracts'
import { computePhotoRect } from '../services/display-fit'
import {
  buildAbsentViewerReadiness,
  createViewerHostService,
  type ViewerHostService,
} from '../services/viewer-host-adapter'

/**
 * layout report를 liveness heartbeat로도 사용한다. host는 마지막 report 시각을
 * host clock으로 평가하므로, WebView2가 이 타이머를 throttle하면 readiness가 정직하게 내려간다.
 */
export const VIEWER_HEARTBEAT_INTERVAL_MS = 1_000
export const VIEWER_RETRY_INTERVAL_MS = 1_000

/**
 * 측정된 photo rect가 기대한 contain-fit과 이만큼 넘게 어긋나면 layout이 깨진 것으로 본다.
 * 깨진 layout에서 크기를 보고하면 잘못된 display-fit 계약이 만들어진다.
 */
export const VIEWER_LAYOUT_TOLERANCE_PX = 1

type UseViewerReadinessOptions = {
  /** 사진이 놓일 수 있는 전체 영역. 기대 rect 계산의 기준이다. */
  stageRef: RefObject<HTMLElement | null>
  /** 실제 photo rectangle. 계약에 보고하는 값은 이 요소의 실측 크기다. */
  photoRef: RefObject<HTMLElement | null>
  hostService?: ViewerHostService
}

/**
 * 늦게 도착한 snapshot이 최신 상태를 덮지 않게 한다.
 *
 * revision은 host에서만 단조 증가한다. staleness 전이는 revision을 올리지 않고
 * reasonCode만 바꾸므로 그 경우도 함께 허용한다.
 */
export function shouldApplySnapshot(
  previous: ViewerReadinessSnapshot,
  next: ViewerReadinessSnapshot,
) {
  if (next.revision < previous.revision) {
    return false
  }

  if (next.revision > previous.revision) {
    return true
  }

  return (
    next.reasonCode !== previous.reasonCode ||
    next.viewerReady !== previous.viewerReady ||
    next.viewerEpoch !== previous.viewerEpoch ||
    next.sessionId !== previous.sessionId
  )
}

export function useViewerReadiness({
  stageRef,
  photoRef,
  hostService,
}: UseViewerReadinessOptions) {
  const serviceRef = useRef<ViewerHostService | null>(null)

  if (serviceRef.current === null) {
    serviceRef.current = hostService ?? createViewerHostService()
  }

  const [readiness, setReadiness] = useState<ViewerReadinessSnapshot>(
    buildAbsentViewerReadiness,
  )
  const [isSubscribed, setIsSubscribed] = useState(false)
  const [listenerRetryVersion, setListenerRetryVersion] = useState(0)
  const readinessRef = useRef(readiness)
  const listenerEpochRef = useRef<number | null>(null)
  const listenerReportInFlightEpochRef = useRef<number | null>(null)
  const listenerRetryIdRef = useRef<ReturnType<typeof globalThis.setTimeout> | null>(null)
  const isSubscribedRef = useRef(false)

  const applyReadiness = useEffectEvent((next: ViewerReadinessSnapshot) => {
    if (!shouldApplySnapshot(readinessRef.current, next)) {
      return
    }

    readinessRef.current = next
    setReadiness(next)
  })

  const reportLayout = useEffectEvent(() => {
    const stage = stageRef.current
    const photo = photoRef.current
    const snapshot = readinessRef.current

    if (stage === null || photo === null || snapshot.viewerEpoch === 0) {
      return
    }

    const stageBounds = stage.getBoundingClientRect()
    const photoBounds = photo.getBoundingClientRect()

    const hasMeasurablePhoto =
      Number.isFinite(photoBounds.width) &&
      Number.isFinite(photoBounds.height) &&
      photoBounds.width > 0 &&
      photoBounds.height > 0

    // 실제 화면에 있는 크기를 보고하고, 기대 contain-fit과 어긋나면 layout-ready를 주장하지 않는다.
    const expectedRect = computePhotoRect({
      containerWidth: stageBounds.width,
      containerHeight: stageBounds.height,
    })
    const isLayoutIntact =
      hasMeasurablePhoto &&
      expectedRect !== null &&
      Math.abs(expectedRect.cssWidth - photoBounds.width) <=
        VIEWER_LAYOUT_TOLERANCE_PX &&
      Math.abs(expectedRect.cssHeight - photoBounds.height) <=
        VIEWER_LAYOUT_TOLERANCE_PX

    const report: ViewerLayoutReport = {
      viewerEpoch: snapshot.viewerEpoch,
      sessionId: snapshot.sessionId,
      cssWidth: hasMeasurablePhoto ? photoBounds.width : 0,
      cssHeight: hasMeasurablePhoto ? photoBounds.height : 0,
      devicePixelRatio:
        typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1,
      layoutReady: isLayoutIntact,
    }

    void serviceRef
      .current!.reportLayout(report)
      .then(applyReadiness)
      .catch(() => undefined)
  })

  const announceListenerReady = useEffectEvent((viewerEpoch: number) => {
    if (
      listenerEpochRef.current === viewerEpoch ||
      listenerReportInFlightEpochRef.current === viewerEpoch
    ) {
      return
    }

    listenerReportInFlightEpochRef.current = viewerEpoch
    void serviceRef
      .current!.reportListenerReady({ viewerEpoch })
      .then((next) => {
        listenerReportInFlightEpochRef.current = null

        if (readinessRef.current.viewerEpoch !== viewerEpoch) {
          return
        }

        listenerEpochRef.current = viewerEpoch
        applyReadiness(next)
      })
      .catch(() => {
        listenerReportInFlightEpochRef.current = null

        if (
          !isSubscribedRef.current ||
          readinessRef.current.viewerEpoch !== viewerEpoch
        ) {
          return
        }

        if (listenerRetryIdRef.current !== null) {
          globalThis.clearTimeout(listenerRetryIdRef.current)
        }

        listenerRetryIdRef.current = globalThis.setTimeout(() => {
          listenerRetryIdRef.current = null
          setListenerRetryVersion((version) => version + 1)
        }, VIEWER_RETRY_INTERVAL_MS)
      })
  })

  // Snapshot이 durable recovery 경계다. live event만으로 readiness를 소유하지 않는다.
  useEffect(() => {
    let isDisposed = false
    let stopListening: (() => void) | undefined
    let subscriptionRetryId: ReturnType<typeof globalThis.setTimeout> | null = null

    const startListening = () => {
      void serviceRef
        .current!.subscribeToViewerReadiness({
          onReadiness(next) {
            if (!isDisposed) {
              applyReadiness(next)
            }
          },
        })
        .then((unlisten) => {
          if (isDisposed) {
            unlisten()
            return
          }

          stopListening = unlisten
          isSubscribedRef.current = true
          setIsSubscribed(true)
        })
        .catch(() => {
          if (!isDisposed) {
            subscriptionRetryId = globalThis.setTimeout(
              startListening,
              VIEWER_RETRY_INTERVAL_MS,
            )
          }
        })
    }

    void serviceRef
      .current!.getViewerReadiness()
      .then((snapshot) => {
        if (!isDisposed) {
          applyReadiness(snapshot)
        }
      })
      .catch(() => undefined)

    startListening()

    return () => {
      isDisposed = true
      isSubscribedRef.current = false
      setIsSubscribed(false)

      if (subscriptionRetryId !== null) {
        globalThis.clearTimeout(subscriptionRetryId)
      }

      if (listenerRetryIdRef.current !== null) {
        globalThis.clearTimeout(listenerRetryIdRef.current)
        listenerRetryIdRef.current = null
      }

      listenerReportInFlightEpochRef.current = null

      if (stopListening) {
        stopListening()
      }
    }
  }, [])

  // viewer window가 재생성되면 새 epoch로 listener readiness를 다시 알린다.
  useEffect(() => {
    if (!isSubscribed || readiness.viewerEpoch === 0) {
      return
    }

    announceListenerReady(readiness.viewerEpoch)
  }, [isSubscribed, readiness.viewerEpoch, listenerRetryVersion])

  useEffect(() => {
    const stage = stageRef.current
    const photo = photoRef.current

    if (stage === null || photo === null) {
      return
    }

    reportLayout()

    const heartbeatId = globalThis.setInterval(
      () => reportLayout(),
      VIEWER_HEARTBEAT_INTERVAL_MS,
    )
    const observer =
      typeof ResizeObserver === 'undefined'
        ? null
        : new ResizeObserver(() => reportLayout())
    let dprMediaQuery: MediaQueryList | null = null

    function handleDprChange() {
      reportLayout()
      watchCurrentDpr()
    }

    function watchCurrentDpr() {
      dprMediaQuery?.removeEventListener('change', handleDprChange)
      dprMediaQuery = null

      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
        return
      }

      dprMediaQuery = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`)
      dprMediaQuery.addEventListener('change', handleDprChange)
    }

    observer?.observe(stage)
    observer?.observe(photo)
    watchCurrentDpr()

    return () => {
      globalThis.clearInterval(heartbeatId)
      observer?.disconnect()
      dprMediaQuery?.removeEventListener('change', handleDprChange)
    }
  }, [stageRef, photoRef, readiness.viewerEpoch])

  return readiness
}
