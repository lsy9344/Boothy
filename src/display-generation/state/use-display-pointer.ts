import { useCallback, useEffect, useEffectEvent, useRef, useState } from 'react'

import type {
  DisplayGeneration,
  DisplayPointerSnapshot,
  DisplayPresentReport,
  ViewerReadinessSnapshot,
} from '../../shared-contracts'
import {
  buildAbsentDisplayPointer,
  createDisplayHostService,
  type DisplayHostService,
} from '../../viewer-surface/services/viewer-host-adapter'
import { shouldAdvanceDisplay } from '../services/display-guard'

export const DISPLAY_RETRY_INTERVAL_MS = 1_000

/**
 * 늦게 도착한 pointer가 최신 상태를 덮지 않게 한다.
 *
 * revision은 host에서만 단조 증가한다. 같은 revision이라도 활성 generation이 실제로 바뀐
 * 경우는 적용한다 (host가 같은 revision으로 두 번 알릴 수 있는 재수렴 경로 대비).
 */
export function shouldApplyPointer(
  previous: DisplayPointerSnapshot,
  next: DisplayPointerSnapshot,
) {
  if (next.revision < previous.revision) {
    return false
  }

  if (next.revision > previous.revision) {
    return true
  }

  return (
    next.activeGeneration?.generationId !==
      previous.activeGeneration?.generationId ||
    next.sessionId !== previous.sessionId ||
    next.requiredSourceWidthPx !== previous.requiredSourceWidthPx ||
    next.requiredSourceHeightPx !== previous.requiredSourceHeightPx ||
    next.measurementLaneEnabled !== previous.measurementLaneEnabled ||
    next.presentTelemetryEnabled !== previous.presentTelemetryEnabled
  )
}

type UseDisplayPointerOptions = {
  readiness: ViewerReadinessSnapshot
  hostService?: DisplayHostService
}

/**
 * display pointer의 durable 수렴 경계.
 *
 * snapshot이 truth이고 live event는 알림일 뿐이다. 마운트, 재구독, viewer epoch 변경마다
 * snapshot을 다시 읽는다. 표시할 generation은 `shouldAdvanceDisplay` guard를 통과한 것만이다.
 */
export function useDisplayPointer({
  readiness,
  hostService,
}: UseDisplayPointerOptions) {
  const serviceRef = useRef<DisplayHostService | null>(null)

  if (serviceRef.current === null) {
    serviceRef.current = hostService ?? createDisplayHostService()
  }

  const [pointer, setPointer] = useState<DisplayPointerSnapshot>(
    buildAbsentDisplayPointer,
  )
  const [generation, setGeneration] = useState<DisplayGeneration | null>(null)
  const pointerRef = useRef(pointer)
  const generationRef = useRef<DisplayGeneration | null>(null)
  const refinedFallbackRef = useRef<DisplayGeneration | null>(null)
  const poisonedRef = useRef<Set<string>>(new Set())
  const poisonScopeRef = useRef<string>('')

  // useEffectEvent는 항상 최신 props를 본다. readiness를 ref로 따로 들고 다닐 필요가 없다.
  const evaluateGeneration = useEffectEvent((next: DisplayPointerSnapshot) => {
    const poisonScope = `${readiness.sessionId ?? 'none'}:${readiness.viewerEpoch}`

    if (poisonScopeRef.current !== poisonScope) {
      poisonScopeRef.current = poisonScope
      poisonedRef.current.clear()
    }

    // 관람 창이 재생성되면 이전 세대의 자산은 더 이상 이 화면의 진실이 아니다. 즉시 내린다.
    if (
      generationRef.current !== null &&
      generationRef.current.viewerEpoch !== readiness.viewerEpoch
    ) {
      generationRef.current = null
      refinedFallbackRef.current = null
      setGeneration(null)
    }

    const candidate = next.activeGeneration

    // host가 pointer를 비웠다면 화면도 즉시 비운다 (세션 교체, 크기 무효화, decode 실패).
    if (candidate === null) {
      if (generationRef.current !== null) {
        generationRef.current = null
        refinedFallbackRef.current = null
        setGeneration(null)
      }

      return
    }

    const verdict = shouldAdvanceDisplay(
      generationRef.current,
      candidate,
      {
        sessionId: next.sessionId,
        viewerEpoch: readiness.viewerEpoch,
        requiredSourceWidthPx: next.requiredSourceWidthPx,
        requiredSourceHeightPx: next.requiredSourceHeightPx,
      },
      { poisonedGenerationIds: poisonedRef.current },
    )

    if (!verdict.advance) {
      return
    }

    const current = generationRef.current
    refinedFallbackRef.current =
      candidate.tier === 'rawRefinedDisplay' &&
      current?.tier === 'displayFitPresetProxy'
        ? current
        : null
    generationRef.current = candidate
    setGeneration(candidate)
  })

  const applyPointer = useEffectEvent((next: DisplayPointerSnapshot) => {
    if (shouldApplyPointer(pointerRef.current, next)) {
      pointerRef.current = next
      setPointer(next)
    }

    // revision이 같아도 판정 문맥(viewer epoch, photo rect)이 바뀌었을 수 있다.
    // 적용 여부와 무관하게 항상 현재 pointer를 다시 판정한다.
    evaluateGeneration(pointerRef.current)
  })

  const readSnapshot = useEffectEvent(() => {
    void serviceRef
      .current!.getViewerDisplayState()
      .then(applyPointer)
      .catch(() => undefined)
  })

  // snapshot이 durable recovery 경계다. live event만으로 상태를 소유하지 않는다.
  useEffect(() => {
    let isDisposed = false
    let stopListening: (() => void) | undefined
    let retryId: ReturnType<typeof globalThis.setTimeout> | null = null

    const startListening = () => {
      void serviceRef
        .current!.subscribeToDisplayUpdates({
          onPointer(next) {
            if (!isDisposed) {
              applyPointer(next)
            }
          },
        })
        .then((unlisten) => {
          if (isDisposed) {
            unlisten()
            return
          }

          stopListening = unlisten
          // 구독 성립 직후 한 번 더 읽어 구독 전 구간의 gap을 메운다.
          readSnapshot()
        })
        .catch(() => {
          if (!isDisposed) {
            retryId = globalThis.setTimeout(
              startListening,
              DISPLAY_RETRY_INTERVAL_MS,
            )
          }
        })
    }

    readSnapshot()
    startListening()

    return () => {
      isDisposed = true

      if (retryId !== null) {
        globalThis.clearTimeout(retryId)
      }

      stopListening?.()
    }
  }, [])

  // 관람 창이 재생성되면 새 epoch 기준으로 다시 판정하고 host snapshot도 다시 읽는다.
  // 이전 세대 자산의 제거는 `evaluateGeneration`이 한 곳에서 담당한다.
  useEffect(() => {
    evaluateGeneration(pointerRef.current)
    readSnapshot()
  }, [readiness.viewerEpoch])

  const reportPresent = useCallback(async (report: DisplayPresentReport) => {
    // decode 실패만 동일 자산의 재시도를 영구 차단한다. 크기 미달 같은 `rejected`는
    // 뒤 레이어만 거부된 것이므로 현재 앞 레이어와 다음 generation 수렴을 유지한다.
    if (report.outcome === 'decode-failed') {
      poisonedRef.current.add(report.generationId)

      if (generationRef.current?.generationId === report.generationId) {
        // 정밀본은 이미 표시된 proxy 위로 올라가는 승급이다. decode가 실패하면
        // 빈 화면으로 내리지 않고 직전 proxy로 되돌린다. 다른 tier의 첫 표시 실패는
        // 유지할 안전한 화면이 없으므로 기존처럼 비운다.
        const fallback = refinedFallbackRef.current
        refinedFallbackRef.current = null
        generationRef.current = fallback
        setGeneration(fallback)
      }
    } else if (
      report.outcome === 'presented' &&
      generationRef.current?.generationId === report.generationId
    ) {
      refinedFallbackRef.current = null
    }

    try {
      await serviceRef.current!.reportDisplayPresent(report)
    } catch {
      // 계측 보고 실패가 고객 화면에 영향을 주면 안 된다.
    }
  }, [])

  return { pointer, generation, reportPresent }
}
